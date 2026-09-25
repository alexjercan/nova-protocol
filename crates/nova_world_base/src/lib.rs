//! `nova_world_base` is the base game's open world: the sector generator that
//! fills it from shipped content, and the session integration that streams it
//! around the player while an open-world scenario runs.
//!
//! `nova_world` owns the mechanism and names no content. This crate is the
//! complete concrete generator: its environment fields, its cluster policy,
//! its clearance margin, and the base content it fills cells from. It names
//! base content, so the stable ids the world and its bootstrap are built from
//! live here too: the authoring builders and the generator read the same
//! constants.
//!
//! The policy is diagnosable from outside: [`EnvironmentFields`] reads the
//! three fields anywhere, and [`sector_clusters`] answers which clusters a
//! sector owns bodies of and what it placed and skipped, with the same numbers
//! the generator used. Nothing puts them on a streamed entity; a debug view
//! asks for them.
//!
//! The one promise a world seed makes: the same build on the same platform
//! generates the same pristine sectors from it, in any exploration order.
//! Nothing a player changes is kept. A retired sector is generated again from
//! the seed, so a derelict that was destroyed there comes back.
#![warn(missing_docs)]

use bevy::prelude::*;
use nova_events::prelude::Meters;
use nova_gameplay::prelude::PlayerSpaceshipMarker;
use nova_scenario::prelude::{CurrentScenario, ScenarioRole};
use nova_world::prelude::*;

mod clusters;
mod environment;
mod layered;

#[cfg(test)]
mod tests;

pub use crate::{
    clusters::{sector_clusters, ClusterSummary, ClusterType, SectorClusters},
    environment::{Environment, EnvironmentFieldType, EnvironmentFields},
    layered::{NovaLayeredWorld, CLEARANCE_MARGIN},
};

/// Glob-import surface: `use nova_world_base::prelude::*` brings the plugin,
/// the session, the generator, its clearance margin, the environment and
/// cluster diagnostics and the base-world ids into scope.
pub mod prelude {
    pub use super::{
        sector_clusters, ClusterSummary, ClusterType, Environment, EnvironmentFieldType,
        EnvironmentFields, NovaLayeredWorld, NovaWorldBasePlugin, OpenWorldSession, SectorClusters,
        BLOCK_FRAME_TENDER_DAMAGED_SHIP_ID, BLOCK_LINE_WARSHIP_SHIP_ID, BLOCK_WRECK_PLATE_SHIP_ID,
        CLEARANCE_MARGIN, OPEN_WORLD_SCENARIO_ID,
    };
}

/// The id of the base bootstrap New Game launches: the player's warship, the
/// lights and the sky, with every sector streamed in around them.
pub const OPEN_WORLD_SCENARIO_ID: &str = "open_world";

/// The id the block-built line warship is spawned by: the open world's player
/// hull, with six point-defense mounts, one spinal railgun and two torpedo
/// bays.
pub const BLOCK_LINE_WARSHIP_SHIP_ID: &str = "block_line_warship";

/// The id the damaged frame tender is spawned by: the frame tender with its
/// stern and its main drive gone. One of the two hulls a cluster's derelicts
/// are drawn from.
pub const BLOCK_FRAME_TENDER_DAMAGED_SHIP_ID: &str = "block_frame_tender_damaged";

/// The id loose debris plating is spawned by: no computer, drive or gun. One
/// of the two hulls a cluster's derelicts are drawn from.
pub const BLOCK_WRECK_PLATE_SHIP_ID: &str = "block_wreck_plate";

/// The open world's cell edge.
///
/// A travel-scale cell: a boundary is reached under way, not by drifting over
/// a line 30 s out. The same edge the world examples stream at.
const OPEN_WORLD_SECTOR_EDGE: Meters = Meters(32_000.0);

/// How many cells out from the player's the open world keeps live.
///
/// Radius 2 keeps 125 sectors live across a 160 km cube, so at least 64 km of
/// world stands ahead of the player on every axis.
const OPEN_WORLD_ACTIVE_RADIUS: i32 = 2;

/// The world the player asked for on the New Game screen.
///
/// Inserted by the menu's Create, before the open-world bootstrap loads. It
/// stays for the whole run, so a Retry streams the same world again. A new
/// New Game replaces it.
#[derive(Resource, Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpenWorldSession {
    /// The world seed every sector is generated from.
    pub seed: u32,
}

/// The base game's open world.
///
/// Installs `NovaWorldPlugin<NovaLayeredWorld>` and the one system that arms
/// it. `AppBuilder` adds this on the default game path, after
/// `NovaScenarioPlugin`. An app with its own game plugins does not get it, so
/// an example can install a generator of its own.
///
/// The world streams only while an [`ScenarioRole::OpenWorld`] scenario is live
/// and exactly one player ship stands in it. Every other frame has no
/// [`WorldConfig`], and with no config the streaming stages do not run.
pub struct NovaWorldBasePlugin;

impl Plugin for NovaWorldBasePlugin {
    fn build(&self, app: &mut App) {
        trace!("NovaWorldBasePlugin: build");

        app.add_plugins(NovaWorldPlugin::<NovaLayeredWorld>::default());
        // In `Update`, not `PreUpdate`: a menu entry swaps the scenario in
        // `StateTransition`, and a ship destroyed in `FixedUpdate` leaves the
        // world. Both land after `PreUpdate`, and the streaming stages would
        // then read a config nobody re-checked. Exclusive, so its writes are
        // in the world before `Cleanup` decides what to clear.
        app.add_systems(Update, sync_open_world.before(NovaWorldSystems::Cleanup));
    }
}

/// Arm the world over a live open-world scenario and its one player ship, and
/// disarm it everywhere else.
///
/// - No live scenario, or a scenario of any other role: remove the config.
/// - An open-world scenario and no player ship (still spawning, or gone):
///   remove the config, so `nova_world` never looks for an observer that is
///   not there. Its roots retire with it.
/// - Exactly one player ship: put the [`WorldObserver`] on it, and insert the
///   config from [`OpenWorldSession`] unless the same config is already armed.
///
/// # Panics
///
/// An open-world scenario with no [`OpenWorldSession`] has no seed. Two or
/// more player ships give the world no one place to stream around. Both are
/// assembly faults, not runtime conditions, and there is no world to guess.
fn sync_open_world(world: &mut World) {
    let open = world
        .resource::<CurrentScenario>()
        .as_ref()
        .is_some_and(|scenario| scenario.role == ScenarioRole::OpenWorld);
    if !open {
        disarm(world);
        return;
    }

    let Some(session) = world.get_resource::<OpenWorldSession>().copied() else {
        panic!(
            "nova_world_base: an OpenWorld scenario is live with no OpenWorldSession; the world \
             has no seed. Start it through New Game, which inserts the session."
        );
    };

    let players: Vec<Entity> = world
        .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .iter(world)
        .collect();
    let player = match players.as_slice() {
        [] => {
            disarm(world);
            return;
        }
        [player] => *player,
        many => panic!(
            "nova_world_base: an OpenWorld scenario holds {} player ships {many:?}; the world \
             streams around exactly one",
            many.len()
        ),
    };

    if !world.entity(player).contains::<WorldObserver>() {
        world.entity_mut(player).insert(WorldObserver);
    }
    let armed = WorldConfig {
        seed: session.seed,
        sector_edge: OPEN_WORLD_SECTOR_EDGE,
        active_radius: OPEN_WORLD_ACTIVE_RADIUS,
        generator: NovaLayeredWorld,
    };
    if world.get_resource::<WorldConfig<NovaLayeredWorld>>() != Some(&armed) {
        world.insert_resource(armed);
    }
}

/// Remove the config, if one is armed. `nova_world`'s `Cleanup` sees the
/// removal and retires every sector root.
fn disarm(world: &mut World) {
    if world.contains_resource::<WorldConfig<NovaLayeredWorld>>() {
        world.remove_resource::<WorldConfig<NovaLayeredWorld>>();
    }
}
