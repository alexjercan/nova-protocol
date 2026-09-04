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
//! - [`catalog_hull`]: the shipped section list for a ship in the catalog, and
//!   [`cell_section`], the id of the section one block hull carries at a cell.
//! - [`NearField`]: near-field asteroid dressing, close enough to the subject to
//!   actually be in frame.
//! - [`ship_root`] and [`section_health`]: the two lookups a scene needs to
//!   drive production damage at a named section of a named ship.
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
/// `hull` is the catalog ship id (`block_gunship`, `block_raider`, ...).
///
/// Read from the catalog rather than copied, because a copy drifts: a turret
/// mount hand-typed a tenth of a unit off its authored seat is a hundred times
/// the mate epsilon, so its socket misses the plate's, the mount joins no
/// component, `derive_link_point_graph` rejects the WHOLE ship as
/// `Disconnected`, and section integrity falls back to empty adjacency - under
/// which any single section death severs the entire hull into loose wrecks.
/// There is exactly one set of coordinates, and it is the one the game ships.
pub fn catalog_hull(ships: &GameShips, hull: &str) -> Vec<SpaceshipSectionConfig> {
    ships
        .get_ship(hull)
        .unwrap_or_else(|| panic!("catalog_hull: unknown ship '{hull}'"))
        .hull
        .sections
        .clone()
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
pub fn cell_section(ships: &GameShips, hull: &str, cell: Vec3) -> String {
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
                center: Meters3::ZERO,
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
                    material: KIND_ROCK.to_string(),
                    destroy_sound: None,
                    // No wells in the dressing: a near-field rock strong enough
                    // to pull the posed subject would drift it out of frame
                    // over a capture run.
                    mass: None,
                    invulnerable: false,
                    seed: None,
                    lock_signature: None,
                }),
            },
            asteroid_radius: Some(self.radius),
            asteroid_kinds: vec![(KIND_ROCK.to_string(), 1)],
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
    let root = ship_root(world, ship)?;
    let owner = world
        .query_filtered::<(Entity, &EntityId, &ChildOf), With<SectionMarker>>()
        .iter(world)
        .find(|(_, id, parent)| id.0 == section && parent.parent() == root)
        .map(|(entity, _, _)| entity)?;
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
