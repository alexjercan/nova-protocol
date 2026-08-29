//! The scene census: what the world CONTAINS when a capture measures it -
//! entity totals, per-component tallies, the archetypes those entities fall
//! into, instance-vs-distinct counts for meshes and materials, and where each
//! distinct asset CAME FROM.
//!
//! Change this module when a frame-cost hypothesis needs a count nothing in
//! the tree produces. Instances and DISTINCT handles are always reported side
//! by side: 12,572 mesh instances over 681 meshes tells a different story from
//! the 12,572 alone, and a census reporting only the total keeps the wrong
//! hypothesis alive. The ORIGIN breakdown is the same argument one level down:
//! a distinct-mesh total names a cost without naming what to cut.

/// Glob-import surface for the scene census capability.
pub mod prelude {
    pub use super::{nova_census, CensusPlugin, DEFAULT_CENSUS_FRAME};
}

use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
};

use bevy::{
    asset::UntypedAssetId,
    platform::collections::{HashMap, HashSet},
    prelude::*,
    render::mesh::Mesh3d,
};
use nova_gameplay::GameStates;
use nova_ship::prelude::{
    SectionCracksMaterial, SectionRenderOf, ShipDecorMarker, ShipSkinMarker, SkinSurfaceMarker,
    ThrusterPlumeMaterial,
};

use crate::capabilities::frametime::prelude::*;

/// Frames after reaching `Playing` at which the census is taken.
///
/// Half the capture's default warm-up: late enough that the scenario has
/// finished spawning and the loader has torn its staging down, early enough
/// that the count describes the world the measured window runs over.
pub const DEFAULT_CENSUS_FRAME: u32 = 90;

/// How many per-component rows the report keeps. Past this the tail is
/// single-entity marker components and says nothing about a frame.
const COMPONENT_ROWS: usize = 40;

/// How many archetype rows the report keeps.
const ARCHETYPE_ROWS: usize = 20;

/// How many component names one archetype row prints before it elides. A full
/// signature runs to dozens of names and the leading ones identify it.
const ARCHETYPE_SIGNATURE_NAMES: usize = 8;

/// How many origin rows the report keeps. The tail is one-mesh scenery.
const ORIGIN_ROWS: usize = 24;

/// The origin label a drawable gets when no marker on its ancestry claims it
/// and nothing on the way up carries a [`Name`].
const UNNAMED_ORIGIN: &str = "unnamed";

/// The scene census, taken once per capture.
///
/// Inert unless the run is armed for capture ([`probe_armed`]), so an example
/// adds it permanently and an ordinary run pays nothing.
pub fn nova_census() -> CensusPlugin {
    CensusPlugin {
        at_frame: DEFAULT_CENSUS_FRAME,
    }
}

/// Counts the world once, `at_frame` frames after `Playing`, then logs the
/// report and writes `census.json` beside the frame-time artifacts.
#[derive(Clone)]
pub struct CensusPlugin {
    at_frame: u32,
}

impl CensusPlugin {
    /// Take the census this many frames after reaching `Playing` instead of
    /// [`DEFAULT_CENSUS_FRAME`].
    pub fn at_frame(mut self, frame: u32) -> Self {
        self.at_frame = frame;
        self
    }
}

impl Plugin for CensusPlugin {
    fn build(&self, app: &mut App) {
        if !probe_armed() {
            return;
        }
        app.insert_resource(CensusSchedule {
            at_frame: probe_param("census_frame")
                .and_then(|value| value.trim().parse().ok())
                .unwrap_or(self.at_frame),
            frames: 0,
            taken: false,
        });
        app.add_systems(Last, census_tick);
    }
}

/// When the census fires and whether it already has.
#[derive(Resource)]
struct CensusSchedule {
    at_frame: u32,
    frames: u32,
    taken: bool,
}

/// One component's entity count.
#[derive(Debug)]
struct ComponentRow {
    name: String,
    entities: u32,
}

/// One archetype: how many entities share this exact component set, and the
/// leading names of that set.
#[derive(Debug)]
struct ArchetypeRow {
    entities: u32,
    components: usize,
    signature: Vec<String>,
}

/// What one ORIGIN puts in the frame: how many drawables it spawns, and how
/// many DISTINCT mesh and material assets those drawables between them
/// introduce.
///
/// `distinct_meshes` sums to more than the scene total when two origins share
/// one asset. That is the point of reporting both: an origin whose share of
/// the total is smaller than its own count is one that re-uses.
#[derive(Debug)]
struct OriginRow {
    origin: String,
    instances: u32,
    distinct_meshes: usize,
    distinct_materials: usize,
}

fn census_tick(world: &mut World) {
    {
        let playing = world
            .get_resource::<State<GameStates>>()
            .is_some_and(|state| *state.get() == GameStates::Playing);
        let Some(mut schedule) = world.get_resource_mut::<CensusSchedule>() else {
            return;
        };
        if schedule.taken {
            return;
        }
        if !playing {
            return;
        }
        schedule.frames += 1;
        if schedule.frames < schedule.at_frame {
            return;
        }
        schedule.taken = true;
    }
    let report = take_census(world);
    log_census(&report);
    write_census(&report);
}

/// Everything one census reports.
struct Census {
    entities: u32,
    archetype_count: usize,
    mesh_instances: u32,
    distinct_meshes: usize,
    distinct_materials: usize,
    mesh_assets: usize,
    standard_material_assets: usize,
    cracks_material_assets: usize,
    plume_material_assets: usize,
    image_assets: usize,
    skin_plates: u32,
    skin_shapes: Vec<String>,
    components: Vec<ComponentRow>,
    archetypes: Vec<ArchetypeRow>,
    origins: Vec<OriginRow>,
    pieces: Vec<OriginRow>,
}

fn take_census(world: &mut World) -> Census {
    let mut per_component: BTreeMap<String, u32> = BTreeMap::new();
    let mut archetypes: Vec<ArchetypeRow> = Vec::new();
    let mut entities = 0_u32;

    // One pass over the archetype graph rather than one query per component:
    // the question is "what ARE these entities", which no fixed query list can
    // answer - the interesting component is the one nobody thought to name.
    for archetype in world.archetypes().iter() {
        let count = archetype.len();
        if count == 0 {
            continue;
        }
        entities += count;
        let mut signature = Vec::new();
        for (index, id) in archetype.components().iter().enumerate() {
            let name = world
                .components()
                .get_name(*id)
                .map(|name| name.shortname().to_string())
                .unwrap_or_else(|| format!("ComponentId({})", id.index()));
            *per_component.entry(name.clone()).or_default() += count;
            if index < ARCHETYPE_SIGNATURE_NAMES {
                signature.push(name);
            }
        }
        archetypes.push(ArchetypeRow {
            entities: count,
            components: archetype.components().len(),
            signature,
        });
    }
    let archetype_count = archetypes.len();
    archetypes.sort_by_key(|a| Reverse(a.entities));
    archetypes.truncate(ARCHETYPE_ROWS);

    let mut components: Vec<ComponentRow> = per_component
        .into_iter()
        .map(|(name, entities)| ComponentRow { name, entities })
        .collect();
    components.sort_by(|a, b| b.entities.cmp(&a.entities).then(a.name.cmp(&b.name)));
    components.truncate(COMPONENT_ROWS);

    let origin_of = origin_index(world);

    let mut distinct = HashSet::new();
    let mut distinct_materials = HashSet::new();
    let mut mesh_instances = 0_u32;
    let mut per_origin: BTreeMap<String, (u32, HashSet<AssetId<Mesh>>, HashSet<UntypedAssetId>)> =
        BTreeMap::new();
    // Two material components, not one. A damaged section mesh swaps its
    // `StandardMaterial` for the shared cracks material (`damage_cracks`),
    // while pristine sections and fixture cladding keep the standard path.
    // The census must read both to report what is actually drawn.
    let mut drawables = world.query::<(
        Entity,
        &Mesh3d,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&MeshMaterial3d<SectionCracksMaterial>>,
    )>();
    let mut per_piece: BTreeMap<String, (u32, HashSet<AssetId<Mesh>>, HashSet<UntypedAssetId>)> =
        BTreeMap::new();
    for (entity, mesh, standard, cracked) in drawables.iter(world) {
        mesh_instances += 1;
        distinct.insert(mesh.0.id());
        let origin = origin_of.label(entity);
        let piece = format!("{origin} / {}", origin_of.piece(entity));
        let drawn: Vec<UntypedAssetId> = standard
            .map(|material| material.0.id().untyped())
            .into_iter()
            .chain(cracked.map(|material| material.0.id().untyped()))
            .collect();
        for row in [
            per_origin.entry(origin).or_default(),
            per_piece.entry(piece).or_default(),
        ] {
            row.0 += 1;
            row.1.insert(mesh.0.id());
            row.2.extend(drawn.iter().copied());
        }
        distinct_materials.extend(drawn);
    }
    let rank = |rows: BTreeMap<String, (u32, HashSet<AssetId<Mesh>>, HashSet<UntypedAssetId>)>| {
        let mut rows: Vec<OriginRow> = rows
            .into_iter()
            .map(|(origin, (instances, meshes, materials))| OriginRow {
                origin,
                instances,
                distinct_meshes: meshes.len(),
                distinct_materials: materials.len(),
            })
            .collect();
        rows.sort_by(|a, b| {
            b.distinct_meshes
                .cmp(&a.distinct_meshes)
                .then(b.instances.cmp(&a.instances))
                .then(a.origin.cmp(&b.origin))
        });
        rows.truncate(ORIGIN_ROWS);
        rows
    };
    let origins = rank(per_origin);
    let pieces = rank(per_piece);

    // The shape ids, not a count: two captures are compared by INTERSECTING
    // their sets, and a count cannot say whether two hulls wear the same
    // shapes or merely as many.
    let mut skin_plates = 0_u32;
    let mut shapes: BTreeSet<String> = BTreeSet::new();
    let mut plates = world.query::<&ShipSkinMarker>();
    for plate in plates.iter(world) {
        skin_plates += 1;
        shapes.insert(plate.0.id());
    }

    Census {
        entities,
        archetype_count,
        mesh_instances,
        distinct_meshes: distinct.len(),
        distinct_materials: distinct_materials.len(),
        mesh_assets: world
            .get_resource::<Assets<Mesh>>()
            .map_or(0, |assets| assets.len()),
        standard_material_assets: world
            .get_resource::<Assets<StandardMaterial>>()
            .map_or(0, |assets| assets.len()),
        cracks_material_assets: world
            .get_resource::<Assets<SectionCracksMaterial>>()
            .map_or(0, |assets| assets.len()),
        plume_material_assets: world
            .get_resource::<Assets<ThrusterPlumeMaterial>>()
            .map_or(0, |assets| assets.len()),
        image_assets: world
            .get_resource::<Assets<Image>>()
            .map_or(0, |assets| assets.len()),
        skin_plates,
        skin_shapes: shapes.into_iter().collect(),
        components,
        archetypes,
        origins,
        pieces,
    }
}

/// Who a drawable BELONGS to, resolved by walking its ancestors once.
///
/// Markers first and [`Name`] second. A name alone is ambiguous - every glTF
/// scene root a section and a greeble load is called the same thing by the
/// loader - and a marker alone leaves everything outside `nova_ship` in one
/// bucket, which is the half of the frame a ship lane does not own.
struct OriginIndex {
    parent: HashMap<Entity, Entity>,
    name: HashMap<Entity, String>,
    cladding: HashSet<Entity>,
    greeble: HashSet<Entity>,
    section_art: HashSet<Entity>,
}

impl OriginIndex {
    fn label(&self, entity: Entity) -> String {
        let mut current = entity;
        let mut named: Option<&str> = None;
        let mut depth = 0_u32;
        loop {
            if self.cladding.contains(&current) {
                return "cladding".to_string();
            }
            if self.greeble.contains(&current) {
                return "greeble".to_string();
            }
            if self.section_art.contains(&current) {
                // A mesh ON the render child is the no-authored-art fallback
                // cuboid; a mesh UNDER it came out of the section's glTF.
                return if depth == 0 {
                    "section-fallback-cuboid".to_string()
                } else {
                    "section-art".to_string()
                };
            }
            if named.is_none() {
                named = self.name.get(&current).map(String::as_str);
            }
            match self.parent.get(&current) {
                Some(parent) => current = *parent,
                None => break,
            }
            depth += 1;
        }
        named.unwrap_or(UNNAMED_ORIGIN).to_string()
    }

    /// The nearest [`Name`] at or above a drawable, which for a glTF scene is
    /// the primitive's own node name. An origin says WHICH SYSTEM put the mesh
    /// there; this says WHICH PIECE, and the pair is what separates "the art is
    /// heavy" from "one system is minting a fresh asset per entity".
    fn piece(&self, entity: Entity) -> String {
        let mut current = entity;
        loop {
            if let Some(name) = self.name.get(&current) {
                return name.clone();
            }
            match self.parent.get(&current) {
                Some(parent) => current = *parent,
                None => return UNNAMED_ORIGIN.to_string(),
            }
        }
    }
}

fn origin_index(world: &mut World) -> OriginIndex {
    let mut parent = HashMap::default();
    let mut children = world.query::<(Entity, &ChildOf)>();
    for (entity, child_of) in children.iter(world) {
        parent.insert(entity, child_of.0);
    }
    let mut name = HashMap::default();
    let mut names = world.query::<(Entity, &Name)>();
    for (entity, label) in names.iter(world) {
        name.insert(entity, label.as_str().to_string());
    }
    let mut cladding = HashSet::default();
    let mut q_cladding = world.query_filtered::<Entity, With<SkinSurfaceMarker>>();
    cladding.extend(q_cladding.iter(world));
    let mut greeble = HashSet::default();
    let mut q_greeble = world.query_filtered::<Entity, With<ShipDecorMarker>>();
    greeble.extend(q_greeble.iter(world));
    let mut section_art = HashSet::default();
    let mut q_section = world.query_filtered::<Entity, With<SectionRenderOf>>();
    section_art.extend(q_section.iter(world));
    OriginIndex {
        parent,
        name,
        cladding,
        greeble,
        section_art,
    }
}

fn log_census(census: &Census) {
    info!(
        "nova census: entities={} archetypes={} mesh_instances={} distinct_meshes={} distinct_materials={} mesh_assets={} standard_materials={} cracks_materials={} plume_materials={} images={} skin_plates={} skin_shapes={}",
        census.entities,
        census.archetype_count,
        census.mesh_instances,
        census.distinct_meshes,
        census.distinct_materials,
        census.mesh_assets,
        census.standard_material_assets,
        census.cracks_material_assets,
        census.plume_material_assets,
        census.image_assets,
        census.skin_plates,
        census.skin_shapes.len(),
    );
    for row in &census.origins {
        info!(
            "nova census origin: {:>6} instances  {:>4} meshes  {:>4} materials  {}",
            row.instances, row.distinct_meshes, row.distinct_materials, row.origin
        );
    }
    for row in &census.pieces {
        info!(
            "nova census piece: {:>6} instances  {:>4} meshes  {:>4} materials  {}",
            row.instances, row.distinct_meshes, row.distinct_materials, row.origin
        );
    }
    for row in &census.components {
        info!("nova census component: {:>6}  {}", row.entities, row.name);
    }
    for row in &census.archetypes {
        info!(
            "nova census archetype: {:>6}  ({} components) {}",
            row.entities,
            row.components,
            row.signature.join(" + ")
        );
    }
}

fn write_census(census: &Census) {
    let Some(dir) = probe_param(OUT_PARAM) else {
        return;
    };
    let json = serde_json::json!({
        "entities": census.entities,
        "archetypes": census.archetype_count,
        "mesh_instances": census.mesh_instances,
        "distinct_meshes": census.distinct_meshes,
        "distinct_materials": census.distinct_materials,
        "skin_plates": census.skin_plates,
        "skin_shapes": census.skin_shapes,
        "by_origin": census.origins.iter().map(|row| serde_json::json!({
            "origin": row.origin,
            "instances": row.instances,
            "distinct_meshes": row.distinct_meshes,
            "distinct_materials": row.distinct_materials,
        })).collect::<Vec<_>>(),
        "by_piece": census.pieces.iter().map(|row| serde_json::json!({
            "piece": row.origin,
            "instances": row.instances,
            "distinct_meshes": row.distinct_meshes,
            "distinct_materials": row.distinct_materials,
        })).collect::<Vec<_>>(),
        "mesh_assets": census.mesh_assets,
        "standard_material_assets": census.standard_material_assets,
        "cracks_material_assets": census.cracks_material_assets,
        "plume_material_assets": census.plume_material_assets,
        "image_assets": census.image_assets,
        "by_component": census.components.iter().map(|row| serde_json::json!({
            "name": row.name,
            "entities": row.entities,
        })).collect::<Vec<_>>(),
        "by_archetype": census.archetypes.iter().map(|row| serde_json::json!({
            "entities": row.entities,
            "components": row.components,
            "signature": row.signature,
        })).collect::<Vec<_>>(),
    });
    let dir = std::path::PathBuf::from(dir);
    if let Err(error) = std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(dir.join("census.json"), json.to_string()))
    {
        error!("nova census: could not write census.json: {error}");
    }
}
