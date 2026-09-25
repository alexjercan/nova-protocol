//! The examples' world: the one seed, the three configs, the uniform baseline
//! and clustered generators, the empty bootstrap they run inside, and the
//! cells the featured and clustered examples open in.
//!
//! `nova_world` owns the streaming loop and the manifest check and names no
//! content. The featured world is the base game's generator,
//! `NovaLayeredWorld`, its cluster policy and its environment fields, all from
//! `nova_world_base`; the uniform baseline and the clustered world with its
//! own environment fields are example-owned and live beside this file.
//! Everything
//! else `nova_world` refuses to assume - a seed, a cell edge, an active
//! radius - is decided HERE, once, so the six example targets that share it
//! are looking at one world rather than six that happen to agree.
//!
//! Included with
//! `#[path = "../shared/world_fixture/mod.rs"] pub mod world_fixture;` - PUB,
//! and that is the whole dead-code story. One fixture serves six example
//! targets and no target uses all of it, but a `pub` module reachable from a
//! binary's root is not dead code to rustc, so neither an `allow` nor an
//! `expect` is needed: an `allow` would be a blanket the repository forbids,
//! and an `expect` would go unfulfilled in the targets that do use every item.
//! Anything genuinely private in here is still linted.

use bevy::prelude::*;
use nova_protocol::prelude::*;
use nova_world::prelude::*;

mod clustered;
mod environment;
mod uniform_asteroids;

pub use clustered::{
    group_at, group_chances, plan_cell, BodySource, CellPlan, ClusterBody, ClusterGroup,
    ClusteredWorld, GroupBody, GroupChances, GroupId, GroupKind, Outcome, PlannedBody, SkipReason,
    GROUP_LATTICE,
};
pub use environment::{Environment, EnvironmentField, EnvironmentFields};
pub use uniform_asteroids::UniformAsteroids;

/// The examples' world seed.
///
/// A constant, not a setting: a world seed belongs to a save, and the examples
/// have no save. Every run of every example is therefore the same world, which
/// is what lets a range compare a returned sector against the description it
/// had the first time and what lets two captures of one slice be compared.
pub const EXAMPLE_SEED: u32 = 20_260_922;

/// The cell edge every example streams at.
///
/// A TRAVEL-scale cell, not a see-it-all-at-once one: a hand-flown crossing is
/// about 530 s at the free-fly base speed and about 17 s held on the 32x ramp,
/// so a boundary is reached the way a pilot would reach one, under way, rather
/// than by drifting over a line 30 s out.
pub const EXAMPLE_SECTOR_EDGE: Meters = Meters(32_000.0);

/// How many cells out the examples' desired set reaches.
///
/// Radius 2 puts 125 sectors live at once across a 160 km cube, which leaves
/// at least 64 km of live world on every axis ahead of an observer standing
/// anywhere in the centre cell.
pub const EXAMPLE_ACTIVE_RADIUS: i32 = 2;

/// The streaming baseline: every cell filled the same way from its own seed.
///
/// Nothing about the WORLD can explain away a sector that failed to come up,
/// which is what makes it the right generator for judging a retirement or a
/// crossing.
///
/// The nominal radius is 30-60 m because an authored radius is not what a rock
/// DRAWS: the meshed radius reaches 3.5-6x past it, so this draws about
/// 210-720 m diameters. Four bodies in a 32 km cell read as scattered
/// landmarks rather than a belt, which is what the crossing claim needs and is
/// not a claim about how dense a real sector should be.
/// The kinds are the four natural ones; `plain` is the texture control, not a
/// rock a world would contain.
///
/// Whoever arms this must write it ahead of `NovaWorldSystems::Cleanup`, which
/// is nova_world's one ordering rule for a config writer. Every example here
/// arms from `OnEnter` or from an autopilot beat, and both run before
/// `Update`; an `Update` writer would have to say `.before(...)` and would be
/// refused by the streaming stages if it did not.
pub fn uniform_world_config() -> WorldConfig<UniformAsteroids> {
    WorldConfig {
        seed: EXAMPLE_SEED,
        sector_edge: EXAMPLE_SECTOR_EDGE,
        active_radius: EXAMPLE_ACTIVE_RADIUS,
        generator: UniformAsteroids {
            body_count: 4,
            radius_min: Meters(30.0),
            radius_max: Meters(60.0),
            asteroid_kinds: [KIND_ROCK, KIND_METAL, KIND_ICE, KIND_CARBON]
                .map(Into::into)
                .to_vec(),
        },
    }
}

/// The same seed, window and edge, filled by the base game's generator.
///
/// Sharing the streaming dials with [`uniform_world_config`] is the point:
/// what changes between two examples is what a cell CONTAINS, so a difference
/// in how the window behaves cannot be blamed on a different window.
pub fn featured_world_config() -> WorldConfig<NovaLayeredWorld> {
    WorldConfig {
        seed: EXAMPLE_SEED,
        sector_edge: EXAMPLE_SECTOR_EDGE,
        active_radius: EXAMPLE_ACTIVE_RADIUS,
        generator: NovaLayeredWorld,
    }
}

/// The cell the featured examples open in.
///
/// Chosen by a scan of the windows near the origin, not assumed: the window
/// around the origin holds planetoids and derelict hulls, a cluster with a
/// planetoid and one with a hull that each place bodies on both sides of a
/// face, cells with a background scatter, empty cells, and rocks in the face
/// a +X crossing retires.
pub const FEATURE_HOME: SectorCoord = SectorCoord::ORIGIN;

/// The same seed, window and edge, filled by the example-owned clustered
/// generator: asteroid-rich, rock-only, planet-heavy, derelict-only and
/// low-rock groups on a global lattice, and one background rock at most per
/// cell, all read off three environment fields.
pub fn clustered_world_config() -> WorldConfig<ClusteredWorld> {
    WorldConfig {
        seed: EXAMPLE_SEED,
        sector_edge: EXAMPLE_SECTOR_EDGE,
        active_radius: EXAMPLE_ACTIVE_RADIUS,
        generator: ClusteredWorld,
    }
}

/// The cell the clustered example opens in.
///
/// Chosen by a scan of the windows around the origin, not assumed: its window
/// holds a group with a planetoid and a group with a derelict hull that each
/// place bodies on both sides of a cell face, and members skipped at a face.
pub const CLUSTER_HOME: SectorCoord = SectorCoord::new(-1, 0, 1);

/// The free-play bootstrap: an EMPTY scenario.
///
/// No objects, no handlers, no beat list - it exists to give the session a
/// skybox, a camera and a live scenario lifetime, and everything after that is
/// Bevy-owned. Deliberately NOT paired with `assert_scenario_loaded`, whose
/// smoke contract fails a scenario that spawns zero objects: here that is the
/// claim, not the fault.
pub fn free_play_scenario(game_assets: &GameAssets, id: &str, name: &str) -> ScenarioConfig {
    ScenarioConfig {
        description: "Free play: an empty session the world plugin streams into".to_string(),
        ..ScenarioConfig::new(
            id.to_string(),
            name.to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Make the scenario camera the world's observer.
///
/// `nova_world` streams around whatever carries [`WorldObserver`] and refuses
/// any count but one, so naming the observer is the caller's job. In the
/// examples that is the scenario camera: the free-fly rig a human flies and
/// the eye a harness poses are the same entity, which is what keeps a
/// hand-flown crossing and an asserted one the same crossing.
pub fn mark_scenario_camera_observer(
    mut commands: Commands,
    cameras: Query<Entity, (With<ScenarioCameraMarker>, Without<WorldObserver>)>,
) {
    for camera in &cameras {
        commands.entity(camera).insert(WorldObserver);
        debug!("world fixture: the scenario camera is the world observer");
    }
}

/// The observer wiring every world example adds beside [`NovaWorldPlugin`].
///
/// Ordered BEFORE [`NovaWorldSystems::Observe`] rather than left to run
/// whenever: the first frame a session is live is also the frame the streaming
/// loop first asks where its observer is, and an unmarked camera there is a
/// refusal rather than a late start.
pub fn world_observer_plugin(app: &mut App) {
    app.add_systems(
        Update,
        mark_scenario_camera_observer.before(NovaWorldSystems::Observe),
    );
}
