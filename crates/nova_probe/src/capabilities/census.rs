//! The scene census: what the world CONTAINS when a capture measures it -
//! entity totals, per-component tallies, the archetypes those entities fall
//! into, and instance-vs-distinct counts for meshes and materials.
//!
//! Change this module when a frame-cost hypothesis needs a count nothing in
//! the tree produces. Instances and DISTINCT handles are always reported side
//! by side: 12,572 mesh instances over 681 meshes tells a different story from
//! the 12,572 alone, and a census reporting only the total keeps the wrong
//! hypothesis alive.

/// Glob-import surface for the scene census capability.
pub mod prelude {
    pub use super::{nova_census, CensusPlugin, DEFAULT_CENSUS_FRAME};
}

use std::collections::BTreeMap;

use bevy::{platform::collections::HashSet, prelude::*, render::mesh::Mesh3d};
use nova_gameplay::GameStates;

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

/// The scene census, taken once per capture.
///
/// Inert unless the run is armed for capture ([`perf_armed`]), so an example
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
        if !perf_armed() {
            return;
        }
        app.insert_resource(CensusSchedule {
            at_frame: perf_param("census_frame")
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
    mesh_assets: usize,
    standard_material_assets: usize,
    image_assets: usize,
    components: Vec<ComponentRow>,
    archetypes: Vec<ArchetypeRow>,
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
    archetypes.sort_by(|a, b| b.entities.cmp(&a.entities));
    archetypes.truncate(ARCHETYPE_ROWS);

    let mut components: Vec<ComponentRow> = per_component
        .into_iter()
        .map(|(name, entities)| ComponentRow { name, entities })
        .collect();
    components.sort_by(|a, b| b.entities.cmp(&a.entities).then(a.name.cmp(&b.name)));
    components.truncate(COMPONENT_ROWS);

    let mut distinct = HashSet::new();
    let mut mesh_instances = 0_u32;
    let mut meshes = world.query::<&Mesh3d>();
    for mesh in meshes.iter(world) {
        mesh_instances += 1;
        distinct.insert(mesh.0.id());
    }

    Census {
        entities,
        archetype_count,
        mesh_instances,
        distinct_meshes: distinct.len(),
        mesh_assets: world
            .get_resource::<Assets<Mesh>>()
            .map_or(0, |assets| assets.len()),
        standard_material_assets: world
            .get_resource::<Assets<StandardMaterial>>()
            .map_or(0, |assets| assets.len()),
        image_assets: world
            .get_resource::<Assets<Image>>()
            .map_or(0, |assets| assets.len()),
        components,
        archetypes,
    }
}

fn log_census(census: &Census) {
    info!(
        "nova census: entities={} archetypes={} mesh_instances={} distinct_meshes={} mesh_assets={} standard_materials={} images={}",
        census.entities,
        census.archetype_count,
        census.mesh_instances,
        census.distinct_meshes,
        census.mesh_assets,
        census.standard_material_assets,
        census.image_assets,
    );
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
    let Some(dir) = perf_param("out") else {
        return;
    };
    let json = serde_json::json!({
        "entities": census.entities,
        "archetypes": census.archetype_count,
        "mesh_instances": census.mesh_instances,
        "distinct_meshes": census.distinct_meshes,
        "mesh_assets": census.mesh_assets,
        "standard_material_assets": census.standard_material_assets,
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
