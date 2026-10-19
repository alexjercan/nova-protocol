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
//! the generator used. Only the types of those clusters ride on a streamed
//! cell's root, for the player's chart; a debug view asks for the rest.
//! [`CivilizationField`] answers which seeded civilizations
//! reach a place. [`ShipPartSnapshot`] validates and scores the section
//! prototypes generated ships may use. The open world pins one to its
//! generator when it arms, built from the loaded catalog, and refuses a
//! catalog that changes under it; the content lint and the `world_ships`
//! debug example build their own. [`generate_ship`] lays out one ship from a
//! snapshot and [`generate_wreck`] ruins that same ship. [`plan_ship`] picks
//! the civilization and role of one planned hull, lays it out from the pinned
//! snapshot, and poses it: every ship a sector holds comes from it, and the
//! example-owned clustered world lays its hulls out through it too.
//!
//! The one promise a world seed makes: the same build on the same platform
//! generates the same pristine sectors from it, in any exploration order.
//! What a player changes lasts: a retired sector is frozen and comes back as
//! it was left, so a derelict destroyed there stays destroyed. On native
//! builds the world is also a folder on disk: [`create_world`] makes it,
//! [`WorldSaveSession`] saves the player, the frozen ledger and every live
//! sector on each sector crossing and on leaving, and [`open_world`] with
//! [`resume_world`] brings it back. The web build has no saved worlds.
//!
//! While a world streams, this crate also feeds the player's
//! [`Chart`]: the cells the player enters, the stations and derelict hulls
//! the player's sensors find, and what the player sees at a docked station,
//! and posts a [`nova_gameplay::prelude::JobBoard`] on every living generated
//! station, seeded from the world seed, the station and what stands around
//! it, renews an offer that stood past the price table's window or was
//! handed in, puts an accepted job's bodies in the world and marks a held job
//! done. It derives the start role's goal progress from those records every
//! frame and stamps the goal met once, as a saved fact
//! (`nova_gameplay::prelude::GoalRecord`). A world save carries the chart,
//! the player's [`Jobs`], the [`JobPostings`], the [`WorldClock`] and the
//! goal record; on the web they last one session.
#![warn(missing_docs)]

use bevy::prelude::*;
use nova_assets::prelude::LoadedSectionPacks;
use nova_events::prelude::Meters;
use nova_gameplay::prelude::{Chart, GameItems, JobPostings, Jobs, PlayerSpaceshipMarker};
use nova_scenario::prelude::{CurrentScenario, OpenWorldSession, ScenarioWorldType};
use nova_ship::prelude::GameStyles;
use nova_world::prelude::*;

mod chart;
mod civilizations;
mod clusters;
mod economy;
mod environment;
mod goal;
mod jobs;
mod layered;
#[cfg(not(target_arch = "wasm32"))]
mod save;
mod sector_ships;
mod sector_stations;
mod ship_layout;
mod ship_parts;
mod stance;
mod start;
mod station_hulls;
mod station_layout;

#[cfg(all(not(target_arch = "wasm32"), any(test, feature = "test-support")))]
pub mod test_support;
#[cfg(test)]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
pub use crate::save::{
    check_world, create_world, delete_world, list_worlds, open_world, resume_world, world_slug,
    write_world, FrozenTransient, FrozenTransientType, RegisteredScenarios, ResumedWorld,
    SaveReason, SavedChart, SavedChartCell, SavedChartMark, SavedChartMarkType, SavedFleet,
    SavedObjective, SavedObjectiveLine, SavedObjectiveType, SavedPlayer, WorldFolder, WorldListing,
    WorldLock, WorldRefusal, WorldResumeProgress, WorldResumeRefused, WorldSaveHeader,
    WorldSaveSession, WorldSaveState, WorldSaveStatus, WORLD_RESUME_SECONDS_MAX, WORLD_SAVE_FORMAT,
};
pub use crate::{
    civilizations::{
        role_style_id, Civilization, CivilizationField, CivilizationReach, CivilizationStatusType,
        STANCE_ALLY_SHARE, STANCE_ENEMY_SHARE, STANCE_NEIGHBOUR_RADIUS,
    },
    clusters::{sector_clusters, ClusterSummary, SectorClusters},
    environment::{Environment, EnvironmentFieldType, EnvironmentFields},
    layered::{NovaLayeredWorld, CLEARANCE_MARGIN},
    sector_ships::{plan_ship, ship_stock, HullSlot, PlannedShip},
    ship_layout::{
        generate_ship, generate_wreck, ShipDriveLayoutType, ShipLayout, ShipLayoutConstraintType,
        ShipLayoutFailure, ShipLayoutRequest, MAX_SHIP_SECTIONS,
    },
    ship_parts::{
        ShipPart, ShipPartExclusionType, ShipPartFamilyType, ShipPartFault, ShipPartPack,
        ShipPartSnapshot,
    },
    stance::{
        apply_offence, enemy_neighbours, fine_cr, record_merit, seeded_stance, settle, standing,
        StanceStanding, FINE_CR,
    },
    start::{start_preview, start_session, starting_station, StartPreview, START_PREVIEW_REACH},
    station_layout::{
        generate_station, generated_station, station_archetype, station_ceiling,
        StationArchetypeType, StationLayout, StationLayoutConstraintType, StationLayoutFailure,
        StationLayoutRequest,
    },
};

/// Glob-import surface: `use nova_world_base::prelude::*` brings the plugin,
/// the generator, its clearance margin, the environment, cluster
/// and civilization diagnostics, the ship-part snapshot, the ship layout, the
/// ship planner, the base-world ids, the role style ids and the saved-world
/// format into scope.
pub mod prelude {
    pub use super::{
        apply_offence, enemy_neighbours, fine_cr, generate_ship, generate_station, generate_wreck,
        generated_station, plan_ship, record_merit, role_style_id, sector_clusters, seeded_stance,
        settle, ship_stock, standing, start_preview, start_session, starting_station,
        station_archetype, station_ceiling, Civilization, CivilizationField, CivilizationReach,
        CivilizationStatusType, ClusterSummary, Environment, EnvironmentFieldType,
        EnvironmentFields, HullSlot, NovaLayeredWorld, NovaWorldBasePlugin, PlannedShip,
        SectorClusters, ShipDriveLayoutType, ShipLayout, ShipLayoutConstraintType,
        ShipLayoutFailure, ShipLayoutRequest, ShipPart, ShipPartExclusionType, ShipPartFamilyType,
        ShipPartFault, ShipPartPack, ShipPartSnapshot, StanceStanding, StartPreview,
        StationArchetypeType, StationLayout, StationLayoutConstraintType, StationLayoutFailure,
        StationLayoutRequest, ARMOURED_STYLE_ID, BLOCK_WAYSTATION_STATION_ID, CIVILIAN_STYLE_ID,
        CLEARANCE_MARGIN, FINE_CR, INDUSTRIAL_STYLE_ID, MAX_SHIP_SECTIONS,
        OPEN_WORLD_BOUNDS_RADIUS_MAX, OPEN_WORLD_SCENARIO_ID, SALVAGE_STYLE_ID, STANCE_ALLY_SHARE,
        STANCE_ENEMY_SHARE, STANCE_NEIGHBOUR_RADIUS, START_PREVIEW_REACH, STATION_STYLE_ID,
    };
    #[cfg(not(target_arch = "wasm32"))]
    pub use super::{
        check_world, create_world, delete_world, list_worlds, open_world, resume_world, world_slug,
        write_world, FrozenTransient, FrozenTransientType, RegisteredScenarios, ResumedWorld,
        SaveReason, SavedChart, SavedChartCell, SavedChartMark, SavedChartMarkType, SavedFleet,
        SavedObjective, SavedObjectiveLine, SavedObjectiveType, SavedPlayer, WorldFolder,
        WorldListing, WorldLock, WorldRefusal, WorldResumeProgress, WorldResumeRefused,
        WorldSaveHeader, WorldSaveSession, WorldSaveState, WorldSaveStatus,
        WORLD_RESUME_SECONDS_MAX, WORLD_SAVE_FORMAT,
    };
}

/// The id of the base bootstrap New Game launches: the player's start role
/// ship, the lights and the sky, with every sector streamed in around them.
pub const OPEN_WORLD_SCENARIO_ID: &str = "open_world";

/// The id the block-built waystation is spawned by: an unarmed plated block
/// with a bridge on top and a docking collar off its port flank, no drive.
/// Scenarios place it; generated worlds generate their own stations.
pub const BLOCK_WAYSTATION_STATION_ID: &str = "block_waystation";

/// The id the industrial look is named by. Industrial world ships wear it.
pub const INDUSTRIAL_STYLE_ID: &str = "industrial";

/// The id the armoured look is named by. Armored world ships wear it.
pub const ARMOURED_STYLE_ID: &str = "armoured";

/// The id the civilian look is named by. Civilian world ships wear it.
pub const CIVILIAN_STYLE_ID: &str = "civilian";

/// The id the salvage look is named by. Scavenger world ships wear it.
pub const SALVAGE_STYLE_ID: &str = "salvage";

/// The id the station look is named by. The ring station wears it.
pub const STATION_STYLE_ID: &str = "station";

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

/// The largest `bounds` radius a generated scenario may author: the active
/// window's half edge, so the whole bounded sphere stands in streamed cells
/// while the player is at its centre. The content lint refuses a larger one.
pub const OPEN_WORLD_BOUNDS_RADIUS_MAX: Meters =
    Meters(OPEN_WORLD_SECTOR_EDGE.0 * (2 * OPEN_WORLD_ACTIVE_RADIUS + 1) as f32 / 2.0);

/// The base game's open world.
///
/// Installs `NovaWorldPlugin<NovaLayeredWorld>` and the one system that arms
/// it. `AppBuilder` adds this on the default game path, after
/// `NovaScenarioPlugin`. An app with its own game plugins does not get it, so
/// an example can install a generator of its own.
///
/// The world streams only while a scenario whose world is
/// [`ScenarioWorldType::Generated`] is live and exactly one player ship stands
/// in it. Every other frame has no [`WorldConfig`], and with no config the
/// streaming stages do not run.
pub struct NovaWorldBasePlugin;

impl Plugin for NovaWorldBasePlugin {
    fn build(&self, app: &mut App) {
        trace!("NovaWorldBasePlugin: build");

        app.add_plugins(NovaWorldPlugin::<NovaLayeredWorld>::default());
        app.init_resource::<Chart>();
        app.init_resource::<Jobs>();
        app.init_resource::<JobPostings>();
        // A hull the player neutralizes is an offence against its
        // civilization the moment its marker lands; a provocation is stamped
        // the moment the tally crosses.
        app.add_observer(stance::record_offence);
        app.add_observer(stance::record_provocation);
        // An accepted job puts its bodies in the world; one observer per
        // located kind marks the held job done off the existing event kinds.
        app.add_observer(jobs::spawn_job_bodies);
        app.add_observer(jobs::recover_on_undock);
        app.add_observer(jobs::escort_on_arrival);
        app.add_observer(jobs::distress_on_answer);
        app.add_observer(jobs::neutralize_on_kill);
        app.add_observer(jobs::drop_on_loss);
        // In `Update`, not `PreUpdate`: a menu entry swaps the scenario in
        // `StateTransition`, and a ship destroyed in `FixedUpdate` leaves the
        // world. Both land after `PreUpdate`, and the streaming stages would
        // then read a config nobody re-checked. Exclusive, so its writes are
        // in the world before `Cleanup` decides what to clear. Skipped until
        // New Game or a Load inserts the session, a generated scenario is
        // live, or a config is armed: a campaign that never opened a world
        // pays nothing for it.
        app.add_systems(
            Update,
            sync_open_world
                .run_if(
                    resource_exists::<OpenWorldSession>
                        .or_else(resource_exists::<WorldConfig<NovaLayeredWorld>>)
                        .or_else(generated_scenario_is_live),
                )
                .before(NovaWorldSystems::Cleanup),
        );
        // The chart clears with the frozen ledger. It is fed after `Retire`,
        // so a cell materialized this frame is on hand, and before the save,
        // so the snapshot holds this frame's chart.
        app.add_systems(
            Update,
            (
                chart::clear_chart.in_set(NovaWorldSystems::Cleanup),
                (
                    chart::chart_visited_cell.run_if(resource_exists::<CurrentSector>),
                    chart::chart_sensor_finds,
                    chart::chart_docked_station,
                    // The goal reads this frame's hand-ins before the save
                    // snapshots the record.
                    goal::track_role_goal,
                    // The fresh player ship docks at the starting station
                    // the frame it stands.
                    start::berth_at_start,
                    // A thawed trader's hold catches up before a board reads
                    // it; a served window is forgotten and every body's side
                    // follows its civilization's stance before the save.
                    economy::restock_thawed_traders,
                    stance::expire_stances,
                    stance::sync_civilization_sides,
                    jobs::post_station_boards,
                    // After the boards, so a renewal's despawns land first.
                    jobs::mark_held_job_targets,
                )
                    .chain()
                    .after(NovaWorldSystems::Retire)
                    .run_if(resource_exists::<WorldConfig<NovaLayeredWorld>>),
            ),
        );
        // The saved world, native only. Restore runs on the arming frame,
        // after `Cleanup` cleared the ledger and before `Observe` asks the
        // first cells for their records. The save runs after `Retire`, so the
        // snapshot sees the window this frame left. The writer is polled
        // before the snapshot, so a waiting save starts on the frame the
        // last one finishes. The saved transients come back after `Retire`,
        // once this frame's sector spawns are applied, and before the save
        // looks at the window.
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(
            Update,
            (
                save::restore_resumed_world
                    .run_if(resource_exists::<save::ResumedWorld>)
                    .after(NovaWorldSystems::Cleanup)
                    .before(NovaWorldSystems::Observe),
                (
                    save::restore_resumed_transients
                        .run_if(resource_exists::<save::ResumedTransients>),
                    save::save_systems(),
                )
                    .chain()
                    .after(NovaWorldSystems::Retire)
                    .after(chart::chart_docked_station),
            ),
        );
    }
}

/// Whether the live scenario declares a generated world, so the arming system
/// runs for a fixed-seed scenario that never inserted a session.
fn generated_scenario_is_live(current: Option<Res<CurrentScenario>>) -> bool {
    current.is_some_and(|current| {
        current
            .0
            .as_ref()
            .is_some_and(|scenario| scenario.world.is_generated())
    })
}

/// Arm the world over a live generated-world scenario and its one player
/// ship, and disarm it everywhere else.
///
/// - No live scenario, or a scenario whose world is `Void`: remove the config.
/// - A generated scenario and no player ship (still spawning, or gone):
///   remove the config, so `nova_world` never looks for an observer that is
///   not there. Its roots retire with it.
/// - Exactly one player ship: put the [`WorldObserver`] on it, and insert the
///   config unless the same config is already armed. The seed is the
///   scenario's own fixed seed, or the [`OpenWorldSession`]'s for a scenario
///   that declares none. An armed world keeps the generator it armed with; an
///   unarmed one builds its ship-part snapshot from [`LoadedSectionPacks`],
///   once, and checks [`GameStyles`] for every role's style.
///
/// The plugin runs it only while an [`OpenWorldSession`] or an armed
/// [`WorldConfig`] exists or a generated scenario is live; an app that never
/// opened a world skips it.
///
/// # Panics
///
/// A generated scenario with no fixed seed and no [`OpenWorldSession`] has no
/// seed. Two or more player ships give the world no one place to stream
/// around. Both are assembly faults, not runtime conditions, and there is no
/// world to guess.
///
/// With exactly one player ship, it also panics when there are no
/// [`LoadedSectionPacks`] or no [`GameStyles`] (the merge logged why), when
/// the loaded ship parts do not build a snapshot or a role's style is
/// missing, and when the loaded catalog digest differs from the armed one.
/// The last check runs before any config is written: a new config would
/// retire every sector and regenerate the world from content it was not
/// armed over.
fn sync_open_world(world: &mut World) {
    let fixed_seed = match world.resource::<CurrentScenario>().0.as_ref() {
        Some(scenario) => match &scenario.world {
            ScenarioWorldType::Generated(config) => config.seed,
            ScenarioWorldType::Void => {
                disarm(world);
                return;
            }
        },
        None => {
            disarm(world);
            return;
        }
    };

    let seed = match fixed_seed {
        Some(seed) => seed,
        None => match world.get_resource::<OpenWorldSession>() {
            Some(session) => session.seed,
            None => panic!(
                "nova_world_base: a generated-world scenario with no fixed seed is live with no \
                 OpenWorldSession; the world has no seed. Start it through New Game, which \
                 inserts the session."
            ),
        },
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
            "nova_world_base: a generated-world scenario holds {} player ships {many:?}; the \
             world streams around exactly one",
            many.len()
        ),
    };

    // The player spawn takes the record; one still here was saved for an id
    // the scenario never spawned, and the world would arm around a fresh ship.
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(resumed) = world.get_resource::<nova_scenario::prelude::ResumedSpaceship>() {
        panic!(
            "nova_world_base: the open world armed with its player while the saved ship '{}' \
             was never spawned; the scenario has no player spawn of that id",
            resumed.id.0
        );
    }
    if !world.entity(player).contains::<WorldObserver>() {
        world.entity_mut(player).insert(WorldObserver);
    }
    let Some(loaded) = world.get_resource::<LoadedSectionPacks>() else {
        panic!(
            "nova_world_base: a generated-world scenario is live with no LoadedSectionPacks; \
             the content merge refused the catalog and logged why"
        );
    };
    let Some(styles) = world.get_resource::<GameStyles>() else {
        panic!(
            "nova_world_base: a generated-world scenario is live with no GameStyles; the \
             content merge refused the catalog and logged why"
        );
    };
    let Some(items) = world.get_resource::<GameItems>() else {
        panic!(
            "nova_world_base: a generated-world scenario is live with no GameItems; the \
             content merge refused the catalog and logged why"
        );
    };
    let armed = world.get_resource::<WorldConfig<NovaLayeredWorld>>();
    let generator = match armed {
        Some(armed) if armed.generator.catalog() != loaded.digest => panic!(
            "nova_world_base: the loaded content changed under an armed open world (catalog \
             {:#018x} became {:#018x}); the world is generated from the catalog it armed over",
            armed.generator.catalog().0,
            loaded.digest.0
        ),
        Some(armed) => armed.generator.clone(),
        None => NovaLayeredWorld::from_loaded(loaded, styles, items).unwrap_or_else(|faults| {
            panic!(
                "nova_world_base: the loaded content does not arm the open world:\n  {}",
                faults
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n  ")
            )
        }),
    };
    let config = WorldConfig {
        seed,
        sector_edge: OPEN_WORLD_SECTOR_EDGE,
        active_radius: OPEN_WORLD_ACTIVE_RADIUS,
        generator,
    };
    if armed != Some(&config) {
        world.insert_resource(config);
    }
}

/// Remove the config, if one is armed. `nova_world`'s `Cleanup` sees the
/// removal, retires every sector root and despawns every persistent body
/// waiting top-level for a cell.
fn disarm(world: &mut World) {
    if world.contains_resource::<WorldConfig<NovaLayeredWorld>>() {
        world.remove_resource::<WorldConfig<NovaLayeredWorld>>();
    }
}
