//! The screenshot photo kit: the pieces every screenshot scene shares.
//!
//! Lighting is authored scenario content ([`ThreePointRig`]), so each producer
//! spawns the same three-point rig from its own `ScenarioConfig` and the kit
//! keeps only geometry.
//!
//! Included by each `examples/screenshots/*.rs` producer with
//! `#[path = "shared/kit.rs"] mod kit;`. It lives one level down on purpose -
//! `catalog_matches_disk` (`crates/nova_probe/tests/catalog_drift.rs`) treats
//! every `.rs` DIRECTLY under a category dir as a cataloged example, so a sibling
//! `kit.rs` would fail the catalog check.
//!
//! What it holds, and nothing else:
//!
//! - [`catalog_ship`]: a shipped hull from the catalog, clad as it ships;
//!   [`clad`], the same cladding over a hand-built cell list; and
//!   [`cell_section`], the id of the section one block hull carries at a cell.
//! - [`NearField`]: near-field asteroid dressing, close enough to the subject to
//!   actually be in frame.
//! - [`ship_root`], [`section_entity`], [`section_health`] and
//!   [`section_fixtures`]: the lookups a scene needs to drive production damage
//!   at a named section of a named ship, and to reach the cladding over it.
//!
//! Scene layout (where the planetoid sits, where the ships are posed, how the
//! camera is framed) stays with each producer - this is the kit, not the set.

// Each producer includes the whole kit and uses the part its scene needs; the
// unused half is not dead code, it is another scene's tool.
#![allow(
    dead_code,
    reason = "one source, many example targets: what one producer leaves unused another needs, so no single build can fulfil an expectation"
)]

use bevy::prelude::*;
use nova_protocol::prelude::*;

/// The shipped assembly, read straight from the ship catalog.
///
/// `hull` is the catalog ship id (`block_gunship`, `block_picket`, ...).
///
/// Read from the catalog rather than copied, because a copy drifts: a turret
/// mount hand-typed a tenth of a unit off its authored seat is a hundred times
/// the mate epsilon, so its socket misses the plate's, the mount joins no
/// component, `derive_link_point_graph` rejects the WHOLE ship as
/// `Disconnected`, and section integrity falls back to empty adjacency - under
/// which any single section death severs the entire hull into loose wrecks.
/// There is exactly one set of coordinates, and it is the one the game ships.
///
/// Private: structure alone is not a ship a scene should spawn. Take a whole
/// catalog entry with [`catalog_ship`], or clad your own cells with [`clad`].
fn catalog_hull(ships: &GameShipDesigns, hull: &str) -> Vec<SpaceshipSectionConfig> {
    ships
        .get_design(&hull.into())
        .unwrap_or_else(|| panic!("catalog_hull: unknown ship '{hull}'"))
        .design
        .sections
        .clone()
}

/// The shipped hull WHOLE: its sections plus everything else the catalog entry
/// says about it - the derived skin, the style that skin wears, its collapse
/// threshold and its collapse sound.
///
/// This is what a scene should spawn. A `ShipDesign { sections, ..default() }`
/// around a bare section list spawns a
/// ship with `skin: false`: the cladding every block ship in the fleet wears
/// is DERIVED at spawn from `ShipSkin`/`ShipStyle` on the root, so a hull that
/// leaves those at their defaults renders as bare cells. The game ships no
/// such ship; a screenshot of one is a screenshot of something the player
/// never sees.
pub fn catalog_ship(ships: &GameShipDesigns, hull: &str) -> ShipDesign {
    ships
        .get_design(&hull.into())
        .unwrap_or_else(|| panic!("catalog_ship: unknown ship '{hull}'"))
        .design
        .clone()
}

/// A hand-built cell list, clad: the same derived skin a catalog block hull
/// wears, in the named style.
///
/// For the scenes that author their own structure rather than taking a
/// shipped one. The style is required rather than defaulted, because an
/// unnamed style is the undressed derivation - plate colours and no greebles -
/// and that is a look no shipped ship has.
pub fn clad(sections: Vec<SpaceshipSectionConfig>, style: &str) -> ShipDesign {
    ShipDesign {
        sections,
        presentation: ShipPresentationConfig {
            skin: true,
            style: Some(style.to_string()),
            ..default()
        },
        ..default()
    }
}

/// Give every weapon section of `design` a hard magazine of nothing: the guns
/// are on the ship and they are dry.
///
/// Read off the design's own section list rather than written out by id, so a
/// re-armed catalog ship arrives here dry as well.
pub fn dry_magazines(design: &mut ShipDesign, sections: &GameSections) {
    for section in &mut design.sections {
        let Some(kind) = section
            .source
            .resolve(Some(sections))
            .and_then(|config| empty_magazine(&config.kind))
        else {
            continue;
        };
        let patch = SectionConfigPatch {
            kind: Some(kind),
            ..default()
        };
        match &mut section.source {
            SectionSource::Prototype { patch: on_ref, .. } => *on_ref = patch,
            SectionSource::Inline(config) => patch
                .apply(config)
                .expect("the patch is built from the section's own kind"),
        }
    }
}

/// The empty-magazine patch for one weapon kind, or `None` for a section that
/// carries no gun at all.
///
/// The reload goes with the magazine. A weapon that batch-reloads out of an
/// empty rack would refill itself, and `Limited(0)` under a `Batch` is a
/// content error the lint refuses to start a scenario on.
fn empty_magazine(kind: &SectionKind) -> Option<SectionKindPatch> {
    let dry = Some(AmmoCapacity::Limited(0));
    let no_reload = Some(ReloadConfig::Disabled);
    match kind {
        SectionKind::Turret(_) => Some(SectionKindPatch::Turret(TurretSectionConfigPatch {
            ammunition: dry,
            reload: no_reload,
            ..default()
        })),
        SectionKind::Torpedo(_) => Some(SectionKindPatch::Torpedo(TorpedoSectionConfigPatch {
            ammunition: dry,
            reload: no_reload,
            ..default()
        })),
        SectionKind::Railgun(_) => Some(SectionKindPatch::Railgun(RailgunSectionConfigPatch {
            ammunition: dry,
            reload: no_reload,
            ..default()
        })),
        SectionKind::Hull(_)
        | SectionKind::Thruster(_)
        | SectionKind::Controller(_)
        | SectionKind::Docking(_) => None,
    }
}

/// The id of the section a BLOCK hull carries at one build-grid cell.
///
/// A block ship names its specials (`bridge`, `pdc_aft_port`) and numbers its
/// plating (`plate_17`), and the numbering is an artifact of the order the
/// design's boxes were unioned - it moves the moment a hull grows a cell. A
/// scene that wants "the plate on the port waist" says so by its CELL, which is
/// the coordinate the hull is actually authored in.
///
/// Cells are BUILD-GRID cells: one cell is one world unit is 10 m.
pub fn cell_section(ships: &GameShipDesigns, hull: &str, cell: Vec3) -> String {
    catalog_hull(ships, hull)
        .into_iter()
        .find(|section| section.position.abs_diff_eq(cell, 1e-3))
        .unwrap_or_else(|| panic!("cell_section: '{hull}' carries no section at {cell:?}"))
        .id
}

/// Near-field asteroid dressing: a ring of rocks close enough to the subject to
/// be IN the shot.
///
/// The old reel scattered its field 900-1800 m out, where it reads as
/// background noise or nothing at all. The defaults here start at 250 m with
/// real radius variance, so a wide shot has something with parallax in it -
/// close enough to be in frame, far enough that a hero at the origin is not
/// buried in rock at a 150 m camera. Scenes tune the fields for their own
/// framing; the subject is assumed to sit at the origin, so the field's
/// [`ScatterRegion::Ring`] is centred there.
pub struct NearField {
    /// Id prefix each rock gets (`"{id_prefix}{i}"`).
    pub id_prefix: &'static str,
    /// How many rocks.
    pub count: u32,
    /// Layout seed - fixed, so every run of a capture frames the same field.
    pub seed: u64,
    /// What the ring is drawn around, and what the vertical spread is measured
    /// from. The origin for a field dressing the subject; lifted or pushed
    /// aside by a set that has to keep the rocks off something - a look ray a
    /// radar sweeps down, say.
    pub center: Meters3,
    /// Ring radii the rocks land between.
    pub distance: (Meters, Meters),
    /// Per-rock radius range.
    pub radius: (Meters, Meters),
    /// Vertical spread above and below the subject's plane.
    pub y_spread: Meters,
}

impl Default for NearField {
    fn default() -> Self {
        Self {
            id_prefix: "near_rock_",
            count: 30,
            seed: 20260805,
            center: Meters3::ZERO,
            distance: (Meters(250.0), Meters(900.0)),
            radius: (Meters(12.0), Meters(50.0)),
            y_spread: Meters(180.0),
        }
    }
}

impl NearField {
    /// The scatter action to put in a scenario's `OnStart`.
    pub fn action(&self, game_assets: &GameAssets) -> EventActionConfig {
        EventActionConfig::ScatterObjects(ScatterObjectsConfig {
            id_prefix: self.id_prefix.to_string(),
            count: self.count,
            seed: self.seed,
            region: ScatterRegion::Ring {
                center: self.center,
                inner: self.distance.0,
                outer: self.distance.1,
                y_min: -self.y_spread,
                y_max: self.y_spread,
            },
            template: ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: self.id_prefix.to_string(),
                    name: "Rock".to_string(),
                    position: Meters3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                    radius: self.radius.0,
                    texture: game_assets.asteroid_texture.clone().into(),
                    kind: KIND_ROCK.into(),
                    destroy_sound: None,
                    // No wells in the dressing: a near-field rock strong enough
                    // to pull the posed subject would drift it out of frame
                    // over a capture run.
                    mass: None,
                    seed: None,
                    lock_signature: None,
                }),
            },
            asteroid_radius: Some(self.radius),
            asteroid_kinds: vec![(KIND_ROCK.into(), 1)],
            min_separation: None,
        })
    }
}

/// The ship root carrying scenario id `id`.
///
/// A scene that damages something has to find it first, and the only stable
/// handle a scenario hands out is the id it authored.
pub fn ship_root(world: &mut World, id: &str) -> Option<Entity> {
    let mut query = world.query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>();
    query
        .iter(world)
        .find(|(_, live)| live.0 == id)
        .map(|(entity, _)| entity)
}

/// The `Health` node of one named section of one named ship.
///
/// Two places to look, because health is authored on whichever entity owns the
/// hit box: the section itself for a plain hull cell, a child for a section
/// that builds a subtree. Returning the node rather than the section is what
/// lets a caller trigger [`HealthApplyDamage`] - the production damage path -
/// instead of writing a health value directly and skipping every system that
/// reacts to a hit.
pub fn section_health(world: &mut World, ship: &str, section: &str) -> Option<Entity> {
    let owner = section_entity(world, ship, section)?;
    if world.get::<Health>(owner).is_some() {
        return Some(owner);
    }
    let children: Vec<Entity> = world
        .get::<Children>(owner)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    children
        .into_iter()
        .find(|&child| world.get::<Health>(child).is_some())
}

/// Every cladding fixture bolted to `section`, at any depth, and nothing
/// belonging to a section mounted on it.
///
/// A clad hull's outer surface is FIXTURES, not the section mesh: the plates
/// and the greebles bolted to them are what a camera sees, and a round from
/// outside arrives at their colliders first. A producer staging a hit on a
/// clad section needs them by name.
///
/// The descent stops at a nested [`SectionMarker`], because a turret bolted to
/// this cell owns its own skin: stripping it here would answer a broadside on
/// the plating by undressing the gun.
pub fn section_fixtures(world: &mut World, ship: &str, section: &str) -> Vec<Entity> {
    let Some(owner) = section_entity(world, ship, section) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let mut stack = vec![owner];
    while let Some(node) = stack.pop() {
        if node != owner && world.get::<SectionMarker>(node).is_some() {
            continue;
        }
        if let Some(children) = world.get::<Children>(node) {
            stack.extend(children.iter());
        }
        if node != owner && world.get::<SectionFixture>(node).is_some() {
            found.push(node);
        }
    }
    found
}

/// The section entity itself, by authored id, under `ship`'s root.
pub fn section_entity(world: &mut World, ship: &str, section: &str) -> Option<Entity> {
    let root = ship_root(world, ship)?;
    world
        .query_filtered::<(Entity, &EntityId, &ChildOf), With<SectionMarker>>()
        .iter(world)
        .find(|(_, id, parent)| id.0 == section && parent.parent() == root)
        .map(|(entity, _, _)| entity)
}
