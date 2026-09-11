//! system_lock_line_of_sight: a lock is a radio link, and rock stops radio.
//!
//! Task 20260905-114723. The radar picker used to see through anything: a
//! hostile parked behind an asteroid was as lockable as one in open space, and
//! a held lock rode through a rock that drifted across the line. The rule is
//! now one ray, cast in `collect_lockable`, so the radar pick, lock validity
//! and the threat set cannot disagree about what the ship can see.
//!
//! One player ship at the origin facing -Z, one uncontrolled target ship
//! parked dead ahead at 1.5 km, and one rock that starts well off the line and
//! is flown onto it and off it again.
//!
//! SIX named invariants:
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a clear line takes the lock` | the range locks at all |
//! | 2 | `outcome: cover breaks a held lock` | and names the branch |
//! | 3 | `outcome: cover keeps a lock from being taken` | the picker agrees |
//! | 4 | `outcome: a cleared line gives the lock back` | cover, not a ban |
//! | 5 | `outcome: cover drops the travel designation too` | both slots go |
//! | 6 | `outcome: an engaged trip flies through the drop` | GOTO owns it |
//!
//! Invariant 1 is the control: without it the five that follow are satisfied
//! by a range that could never lock anything.
//!
//! Invariants 5 and 6 are the pair that makes the rule liveable. The slots do
//! not disagree about sight - a nav designation is the same radio link a
//! weapons lock is, so cover takes both - but an ENGAGED trip is not a
//! designation: the autopilot owns its target from the moment it engages, so
//! the ship keeps flying the leg it was given. Only the player's own tap-clear
//! ends a trip.
//!
//! Controls: none needed; fly and look around freely in interactive runs.
//!
//! `NOVA_SIGHT_LOOP=1` records the news loop instead of running the
//! assertions: the lock taken on the clear line, the rock DRIFTING across it
//! at [`COVER_DRIFT_SPEED`] rather than teleporting, the bracket dropping as it
//! covers the target. The teleport stays the assertion's move - a drift
//! crosses the line at a time the ray decides, and an assertion wants the
//! moment pinned.
//! ```text
//! NOVA_SIGHT_LOOP=1 NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=target/loop-shots \
//!   cargo run --example system_lock_line_of_sight --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_lock_line_of_sight --features debug
//! # look for: `line of sight: the clear line locked the target`,
//! #           `line of sight: the lock let go, reason Occluded`,
//! #           `line of sight: the designation let go and the trip flew on`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use std::collections::BTreeMap;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_lock_line_of_sight")]
#[command(version = "1.0.0")]
#[command(about = "A test range for radar line of sight. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario object id of the rock that does the hiding.
const ROCK: &str = "cover";

/// One seed, so every run hides behind the same silhouette.
const ROCK_SEED: u32 = 20260905;

/// How big the cover is. Small on purpose: the test is a RAY, so what matters
/// is that the rock straddles the line, not that it looks like a wall. Three
/// world units is the size the carve ranges already mesh cheaply.
const ROCK_RADIUS: Meters = Meters(30.0);

/// How far ahead the cover sits when it is hiding the target - a quarter of
/// the way to it, so nothing about the geometry is marginal.
#[cfg(feature = "debug")]
const COVER_ON_THE_LINE: Meters3 = Meters3::new(0.0, 0.0, -400.0);

/// Where the cover waits while the sky is clear. Abeam and far off the axis:
/// outside the aim cone, so it is not a candidate, and nowhere near the
/// segment the scanner's ray walks.
const COVER_OFF_THE_LINE: Meters3 = Meters3::new(1200.0, 0.0, -400.0);

/// Where the target ship parks: dead ahead, inside the ship-class lock ceiling
/// and inside the aim cone, so a clear sky acquires it without steering.
const TARGET_AT: Meters3 = Meters3::new(0.0, 0.0, -1_500.0);

/// How long the run gives a held lock to notice the cover that just arrived.
/// The drop is one upkeep pass, so this is slack, not a budget.
#[cfg(feature = "debug")]
const DROP_SECS: f32 = 10.0;

/// Where the cover sits while the trip is flying: on the same line, but far
/// enough down it that the ship cannot reach the rock inside the beat. The
/// assertion is about a designation letting go, not about a crash.
#[cfg(feature = "debug")]
const COVER_ON_THE_TRIP: Meters3 = Meters3::new(0.0, 0.0, -1_200.0);

/// How long the trip is left to fly after its designation goes, before the
/// run asks whether it made way. Long enough for a burn to show against the
/// start distance, short enough that the rock stays far ahead.
#[cfg(feature = "debug")]
const TRIP_SECS: f32 = 3.0;

/// How much closer the trip has to get, in meters, for the run to call it
/// flying. A burn under thrust covers this many times over; the number is a
/// floor against a ship that merely drifts.
#[cfg(feature = "debug")]
const TRIP_CLOSED_AT_LEAST: Meters = Meters(50.0);

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

/// Set to `1` to record the news loop instead of running the assertions.
#[cfg(feature = "debug")]
const LOOP_ENV: &str = "NOVA_SIGHT_LOOP";

/// The loop the recording run writes.
#[cfg(feature = "debug")]
const LOOP_NAME: &str = "news-0130-lock-occlusion";

/// Where the drifting cover starts, in meters: abeam of the line and further
/// down it than the assertion parks the rock. At the assertion's stand-off
/// the rock fills the frame from the cockpit; two thirds of the way to the
/// target it reads as cover crossing a line, with the bracket still legible.
#[cfg(feature = "debug")]
const COVER_DRIFT_FROM: Meters3 = Meters3::new(480.0, 0.0, -1_000.0);

/// How fast the cover crosses, in meters per second. Slow enough that the
/// rock is on the line for a readable beat, fast enough that the whole
/// crossing fits one loop.
#[cfg(feature = "debug")]
const COVER_DRIFT_SPEED: f32 = 200.0;

/// How long the drift runs: the mirror-image exit, plus a beat.
#[cfg(feature = "debug")]
const COVER_DRIFT_SECS: f32 = 2.0 * 480.0 / COVER_DRIFT_SPEED + 0.4;

/// The still beats either side of the crossing.
#[cfg(feature = "debug")]
const LOOP_HOLD_SECS: f32 = 0.7;

/// Whether THIS run records the loop.
#[cfg(feature = "debug")]
fn sight_loop() -> bool {
    let Ok(raw) = std::env::var(LOOP_ENV) else {
        return false;
    };
    match raw.as_str() {
        "0" => false,
        "1" => true,
        other => panic!("{LOOP_ENV}={other:?} must be 0 or 1"),
    }
}

/// Present while the cover is sliding across the line, in meters per second.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct CoverDrift(Meters3);

/// Put the cover on its start mark and set it going toward the far side.
#[cfg(feature = "debug")]
fn start_the_drift(world: &mut World) {
    let rock = cover_root(world);
    world
        .entity_mut(rock)
        .get_mut::<Transform>()
        .expect("line of sight: the cover rock has no transform")
        .translation = COVER_DRIFT_FROM.to_engine();
    world.insert_resource(CoverDrift(Meters3::new(-COVER_DRIFT_SPEED, 0.0, 0.0)));
    info!("line of sight: cover drifting across the line");
}

/// Slide the cover by its drift each frame. The transform is written directly,
/// the way the assertion's teleport writes it: the rock is scenery with a
/// collider, and the ray reads the collider wherever the transform put it.
#[cfg(feature = "debug")]
fn drift_the_cover(
    time: Res<Time>,
    drift: Res<CoverDrift>,
    mut rocks: Query<(&EntityId, &mut Transform), With<AsteroidMarker>>,
) {
    for (id, mut transform) in &mut rocks {
        if id.as_str() == ROCK {
            transform.translation += drift.0.to_engine() * time.delta_secs();
        }
    }
}

/// Take the radar hold off once the lock is up, and leave the stance raised
/// so the reticle and the bracket stay on the HUD through the crossing.
#[cfg(feature = "debug")]
fn keep_the_lock(world: &mut World) {
    release_action("radar_hold")(world);
}

/// The loop: the lock taken on a clear line, the rock drifting across it,
/// the bracket dropping. No assertion runs on this path; the assertions are
/// the default script.
#[cfg(feature = "debug")]
fn sight_loop_script(script: Script) -> Script {
    script
        .step("hold the radar on the clear line")
        .on_enter(open_the_radar)
        .until(elapsed(1.0))
        .add()
        .step("open the loop on the held lock")
        .on_enter(|world| {
            keep_the_lock(world);
            hide_status_bar(world);
            loop_start(world, LOOP_NAME);
        })
        .until(elapsed(LOOP_HOLD_SECS))
        .add()
        .step("drift the cover across the line")
        .on_enter(start_the_drift)
        .until(elapsed(COVER_DRIFT_SECS))
        .deadline(120.0)
        .add()
        .step("hold the cleared line")
        .on_enter(|world| {
            world.remove_resource::<CoverDrift>();
        })
        .until(elapsed(LOOP_HOLD_SECS))
        .add()
        .step("close the loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(120.0)
        .add()
}

/// How far the ship was from its destination when the cover took the
/// designation away. Invariant 6 measures the leg against it.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct TripStart(f32);

/// Every lock this run let go of, in order, with the branch that let go.
///
/// A message is read once by each reader and the range's assertions run beats
/// after the drop, so the range keeps its own tape rather than racing the
/// flight log for the same messages.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct DropsSeen(Vec<(Entity, CombatLockDrop)>);

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(sight_script());
        if sight_loop() {
            app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
            app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
            app.add_systems(
                Update,
                drift_the_cover
                    .run_if(resource_exists::<CoverDrift>)
                    .run_if(in_state(GameStates::Playing)),
            );
        }
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    // A short, distance-flat dwell so a scripted beat can hold the radar for a
    // predictable moment and know the lock either came or was refused, rather
    // than waiting out the distance-scaled default. The interactive run keeps
    // the real feel.
    #[cfg(feature = "debug")]
    {
        app.insert_resource(TargetingSettings {
            lock_dwell_base: 0.2,
            lock_dwell_range_factor: 0.0,
            lock_dwell_min: 0.0,
            ..default()
        });
        app.init_resource::<DropsSeen>();
        app.add_systems(Update, record_drops);
    }
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(range(&game_assets, &sections)));
}

/// Keep every drop the run produces, so a later beat can read the branch.
#[cfg(feature = "debug")]
fn record_drops(mut drops: MessageReader<CombatLockDropped>, mut seen: ResMut<DropsSeen>) {
    seen.0
        .extend(drops.read().map(|drop| (drop.target, drop.reason)));
}

/// Build the range scenario: a player ship at the origin, an uncontrolled
/// target ship parked dead ahead, and the rock waiting off the line.
fn range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let at = |id: &str, kind: &str, z: f32| SpaceshipSectionConfig {
        id: id.to_string(),
        position: Vec3::new(0.0, 0.0, z),
        rotation: Quat::IDENTITY,
        source: SectionSource::Inline(section(kind)),
        modifications: vec![],
    };
    let sections_line = |prefix: &str| {
        vec![
            at(
                &format!("{prefix}_controller"),
                "basic_controller_section",
                0.0,
            ),
            at(&format!("{prefix}_hull"), "reinforced_hull_section", 1.0),
            at(&format!("{prefix}_thruster"), "basic_thruster_section", 2.0),
        ]
    };

    let player = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::new(),
            speed_cap: None,
        }),
        hull: ShipSource::Inline(ShipHull {
            sections: sections_line("player"),
            ..default()
        }),
        ..default()
    };
    let target = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::None,
        hull: ShipSource::Inline(ShipHull {
            sections: sections_line("target"),
            ..default()
        }),
        ..default()
    };

    let spawn = |id: &str, name: &str, position: Meters3, ship: SpaceshipConfig| {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: name.to_string(),
                position,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(ship),
        })
    };
    let cover = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ROCK.to_string(),
            name: "Cover".to_string(),
            position: COVER_OFF_THE_LINE,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: ROCK_RADIUS,
            texture: game_assets.asteroid_texture.clone().into(),
            material: KIND_ROCK.to_string(),
            destroy_sound: None,
            mass: None,
            invulnerable: true,
            lock_signature: None,
            seed: Some(ROCK_SEED),
        }),
    });

    ScenarioConfig {
        description: "A test range for radar line of sight.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            // The range lights itself: the engine spawns no light, so a
            // scenario that authors none renders black.
            actions: [
                vec![
                    spawn("player_ship", "Sight Test Ship", Meters3::ZERO, player),
                    spawn("target_ship", "Sight Target Ship", TARGET_AT, target),
                    cover,
                ],
                ThreePointRig::around("sight", Meters3::ZERO, 5.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "lock_line_of_sight".to_string(),
            "Line of Sight Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The player ship root. Mandatory: a stage that needs it must fail loudly.
#[cfg(feature = "debug")]
fn player_root(world: &mut World) -> Entity {
    world
        .query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()
        .iter(world)
        .next()
        .expect("line of sight: no player ship root")
}

/// The target ship root - the only non-player ship in the range.
#[cfg(feature = "debug")]
fn target_root(world: &mut World) -> Entity {
    world
        .query_filtered::<Entity, (With<SpaceshipRootMarker>, Without<PlayerSpaceshipMarker>)>()
        .iter(world)
        .next()
        .expect("line of sight: no target ship root")
}

/// The rock's root, by the id the scenario gave it.
#[cfg(feature = "debug")]
fn cover_root(world: &mut World) -> Entity {
    world
        .query_filtered::<(Entity, &EntityId), With<AsteroidMarker>>()
        .iter(world)
        .find(|(_, id)| id.as_str() == ROCK)
        .map(|(rock, _)| rock)
        .expect("line of sight: no cover rock")
}

/// The whole cast: both ships and the rock are on the stage.
#[cfg(feature = "debug")]
fn the_cast_is_present() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let ships = world
            .try_query_filtered::<(), With<SpaceshipRootMarker>>()
            .map_or(0, |mut query| query.iter(world).count());
        let rocks = world
            .try_query_filtered::<(), With<AsteroidMarker>>()
            .map_or(0, |mut query| query.iter(world).count());
        ships == 2 && rocks == 1
    })
}

/// The player is holding nothing.
#[cfg(feature = "debug")]
fn the_lock_let_go() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&CombatLock, With<PlayerSpaceshipMarker>>()
            .and_then(|mut query| query.iter(world).next().map(|lock| lock.0.is_none()))
            .unwrap_or(false)
    })
}

/// Raise the stance and hold the radar: the live gesture, which is the only
/// thing that locks anything in the deliberate-radar model.
#[cfg(feature = "debug")]
fn open_the_radar(world: &mut World) {
    press_action("combat_stance")(world);
    press_action("radar_hold")(world);
}

/// Let the gesture go without disturbing whatever it caught.
#[cfg(feature = "debug")]
fn close_the_radar(world: &mut World) {
    release_action("radar_hold")(world);
    release_action("combat_stance")(world);
}

/// What the radar found and what the player is holding, right now.
#[cfg(feature = "debug")]
fn radar_and_lock(world: &mut World) -> (Option<Entity>, Option<Entity>) {
    let player = player_root(world);
    let candidate = world
        .entity(player)
        .get::<RadarState>()
        .copied()
        .expect("line of sight: the radar never opened on the hold")
        .candidate;
    let lock = world
        .entity(player)
        .get::<CombatLock>()
        .expect("line of sight: the player ship has no combat lock")
        .0;
    (candidate, lock)
}

/// Invariant 1, and the control for the three below it: an open sky locks the
/// ship dead ahead.
#[cfg(feature = "debug")]
fn assert_the_clear_line_locks(world: &mut World) {
    let target = target_root(world);
    let (candidate, lock) = radar_and_lock(world);
    assert_eq!(
        candidate,
        Some(target),
        "line of sight: the live radar did not find the ship dead ahead \
         through an empty sky"
    );
    assert_eq!(
        lock,
        Some(target),
        "line of sight: a clear line has to lock, or the beats that follow \
         prove nothing about cover"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a clear line takes the lock",
        serde_json::json!({}),
    );
    info!("line of sight: the clear line locked the target");
    close_the_radar(world);
}

/// Fly the rock onto the line between the ship and what it is holding.
#[cfg(feature = "debug")]
fn move_the_cover_onto_the_line(world: &mut World) {
    let rock = cover_root(world);
    world
        .entity_mut(rock)
        .get_mut::<Transform>()
        .expect("line of sight: the cover rock has no transform")
        .translation = COVER_ON_THE_LINE.to_engine();
    info!("line of sight: cover moved onto the line");
}

/// Invariant 2: the drop happened, and it named cover rather than range or a
/// death. An unnamed drop is what the branch model exists to stop.
#[cfg(feature = "debug")]
fn assert_the_cover_broke_the_lock(world: &mut World) {
    let target = target_root(world);
    let dropped = world.resource::<DropsSeen>().0.clone();
    assert_eq!(
        dropped,
        vec![(target, CombatLockDrop::Occluded)],
        "line of sight: the lock had to let go BECAUSE of cover; the run saw \
         {dropped:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: cover breaks a held lock",
        serde_json::json!({ "reason": "Occluded" }),
    );
    info!("line of sight: the lock let go, reason Occluded");
}

/// Invariant 3: the picker agrees with the upkeep. A ship the ray cannot reach
/// is not a candidate, so holding the radar again behind the same rock cannot
/// take the lock back.
#[cfg(feature = "debug")]
fn assert_the_cover_refuses_a_new_lock(world: &mut World) {
    let target = target_root(world);
    let (candidate, lock) = radar_and_lock(world);
    info!("line of sight: behind cover the radar offers {candidate:?}");
    assert_ne!(
        candidate,
        Some(target),
        "line of sight: the picker still offers a ship the ray cannot reach, \
         so the pick and the upkeep disagree"
    );
    assert_ne!(
        lock,
        Some(target),
        "line of sight: the ship behind the rock was locked again"
    );
    nova_probe::probe_marker(
        world,
        "outcome: cover keeps a lock from being taken",
        serde_json::json!({}),
    );
    close_the_radar(world);
}

/// Take the cover away again.
#[cfg(feature = "debug")]
fn move_the_cover_off_the_line(world: &mut World) {
    let rock = cover_root(world);
    world
        .entity_mut(rock)
        .get_mut::<Transform>()
        .expect("line of sight: the cover rock has no transform")
        .translation = COVER_OFF_THE_LINE.to_engine();
    // Wipe the tape so the last invariant reads only what happens from here.
    // The rock itself is what the player is holding - it was the pick behind
    // cover - and it lets go the moment it leaves the aim cone, which is a
    // drop about the ROCK and not about the target.
    world.resource_mut::<DropsSeen>().0.clear();
    info!("line of sight: cover moved back off the line");
}

/// Invariant 4: what the rock took away it gives back. Cover is cover, not a
/// ban on the target for the rest of the run.
#[cfg(feature = "debug")]
fn assert_the_cleared_line_locks_again(world: &mut World) {
    let target = target_root(world);
    let (candidate, lock) = radar_and_lock(world);
    assert_eq!(
        candidate,
        Some(target),
        "line of sight: the picker did not offer the ship again once the rock \
         left the line"
    );
    assert_eq!(
        lock,
        Some(target),
        "line of sight: a cleared line has to lock again"
    );
    let dropped: Vec<(Entity, CombatLockDrop)> = world
        .resource::<DropsSeen>()
        .0
        .iter()
        .copied()
        .filter(|(entity, _)| *entity == target)
        .collect();
    assert!(
        dropped.is_empty(),
        "line of sight: the re-taken lock let go again over a clear line: \
         {dropped:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a cleared line gives the lock back",
        serde_json::json!({}),
    );
    info!("line of sight: the cleared line locked the target again");
    close_the_radar(world);
}

/// The player's travel designation and its engaged maneuver, right now.
#[cfg(feature = "debug")]
fn designation_and_trip(world: &mut World) -> (Option<Entity>, Option<AutopilotAction>) {
    let player = player_root(world);
    let designation = world
        .entity(player)
        .get::<TravelLock>()
        .expect("line of sight: the player ship has no travel lock")
        .0;
    let trip = world
        .entity(player)
        .get::<Autopilot>()
        .map(|autopilot| autopilot.action);
    (designation, trip)
}

/// Centre-to-centre distance from the player to the ship it is flying at.
#[cfg(feature = "debug")]
fn distance_to_target(world: &mut World) -> f32 {
    let player = player_root(world);
    let target = target_root(world);
    let at = |entity: Entity| {
        world
            .entity(entity)
            .get::<Transform>()
            .expect("line of sight: a ship root with no transform")
            .translation
    };
    at(player).distance(at(target))
}

/// Hold the radar with the stance LOWERED: the same gesture, latching the
/// travel slot instead of the combat one.
#[cfg(feature = "debug")]
fn open_the_radar_lowered(world: &mut World) {
    press_action("radar_hold")(world);
}

/// Give the computer the designation it just took. The key goes DOWN here and
/// comes up a beat later: `[G]` engages on the action's Start, and a press
/// that is gone again before the next update never starts anything.
#[cfg(feature = "debug")]
fn engage_the_trip(world: &mut World) {
    release_action("radar_hold")(world);
    press_action("autopilot_goto")(world);
}

/// The guard for invariants 5 and 6: the ship really is holding a designation
/// and really is flying at it. Without this the two below pass on a range that
/// never designated anything.
#[cfg(feature = "debug")]
fn assert_the_trip_is_flying(world: &mut World) {
    let target = target_root(world);
    let (designation, trip) = designation_and_trip(world);
    assert_eq!(
        designation,
        Some(target),
        "line of sight: the lowered radar hold did not take a travel \
         designation, so the drop below would prove nothing"
    );
    assert_eq!(
        trip,
        Some(AutopilotAction::Goto { target }),
        "line of sight: GOTO did not engage on the designation"
    );
    let started = distance_to_target(world);
    world.insert_resource(TripStart(started));
    info!("line of sight: the trip is flying at the designated ship");
}

/// Fly the rock back onto the line, further down it: the trip is under way,
/// and the run wants the designation covered without putting a rock in the
/// ship's path.
#[cfg(feature = "debug")]
fn move_the_cover_onto_the_trip(world: &mut World) {
    let rock = cover_root(world);
    world
        .entity_mut(rock)
        .get_mut::<Transform>()
        .expect("line of sight: the cover rock has no transform")
        .translation = COVER_ON_THE_TRIP.to_engine();
    info!("line of sight: cover moved onto the trip's line");
}

/// The player is holding no travel designation.
#[cfg(feature = "debug")]
fn the_designation_let_go() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&TravelLock, With<PlayerSpaceshipMarker>>()
            .and_then(|mut query| query.iter(world).next().map(|lock| lock.0.is_none()))
            .unwrap_or(false)
    })
}

/// Invariant 5: the slots do not disagree about sight. The same rock on the
/// same line takes the nav designation as well as the weapons lock.
#[cfg(feature = "debug")]
fn assert_the_cover_drops_the_designation(world: &mut World) {
    let (designation, _) = designation_and_trip(world);
    assert_eq!(
        designation, None,
        "line of sight: cover broke the weapons lock but left the nav \
         designation standing, so the two slots read sight differently"
    );
    nova_probe::probe_marker(
        world,
        "outcome: cover drops the travel designation too",
        serde_json::json!({}),
    );
    info!("line of sight: the designation let go under cover");
}

/// Invariant 6: an engaged trip is not a designation. The computer owns its
/// target from the moment it engages, so losing the mark cannot strand the
/// ship - it keeps flying the leg, and only the player's tap-clear ends it.
#[cfg(feature = "debug")]
fn assert_the_trip_flew_on(world: &mut World) {
    let target = target_root(world);
    let (designation, trip) = designation_and_trip(world);
    assert_eq!(
        designation, None,
        "line of sight: the designation came back on its own"
    );
    assert_eq!(
        trip,
        Some(AutopilotAction::Goto { target }),
        "line of sight: the trip disengaged when its designation went, so \
         cover drifting over a nav mark strands the ship"
    );
    let started = world.resource::<TripStart>().0;
    let closed = Meters::from_engine(started - distance_to_target(world));
    assert!(
        closed >= TRIP_CLOSED_AT_LEAST,
        "line of sight: the trip held its target but made no way: {closed:?} \
         closed, {TRIP_CLOSED_AT_LEAST:?} wanted"
    );
    nova_probe::probe_marker(
        world,
        "outcome: an engaged trip flies through the drop",
        serde_json::json!({ "closed_m": closed.0 }),
    );
    info!("line of sight: the designation let go and the trip flew on");
}

#[cfg(feature = "debug")]
fn sight_script() -> Script {
    let script = Script::new()
        .step("load the range")
        .enter(GameStates::Loading)
        .until(scenario_camera_present())
        .deadline(30.0)
        .add()
        .step("wait for the cast")
        .until(the_cast_is_present())
        .deadline(30.0)
        .add();
    if sight_loop() {
        return sight_loop_script(script);
    }
    script
        .step("hold the radar on an empty sky")
        .on_enter(open_the_radar)
        .until(elapsed(1.0))
        .add()
        .step("assert the clear line locks")
        .on_enter(assert_the_clear_line_locks)
        .until(elapsed(0.2))
        .add()
        .step("fly the cover onto the line")
        .on_enter(move_the_cover_onto_the_line)
        .until(the_lock_let_go())
        .deadline(DROP_SECS)
        .add()
        .step("assert the cover broke the lock")
        .on_enter(assert_the_cover_broke_the_lock)
        .until(elapsed(0.2))
        .add()
        .step("hold the radar from behind the cover")
        .on_enter(open_the_radar)
        .until(elapsed(1.0))
        .add()
        .step("assert the cover refuses a new lock")
        .on_enter(assert_the_cover_refuses_a_new_lock)
        .until(elapsed(0.2))
        .add()
        .step("fly the cover off the line")
        .on_enter(move_the_cover_off_the_line)
        .until(elapsed(0.5))
        .add()
        .step("hold the radar on a cleared sky")
        .on_enter(open_the_radar)
        .until(elapsed(1.0))
        .add()
        .step("assert the cleared line locks again")
        .on_enter(assert_the_cleared_line_locks_again)
        .until(elapsed(0.2))
        .add()
        .step("take a travel designation on the clear line")
        .on_enter(open_the_radar_lowered)
        .until(elapsed(1.0))
        .add()
        .step("engage the trip")
        .on_enter(engage_the_trip)
        .until(elapsed(0.3))
        .add()
        .step("let the goto key up")
        .on_enter(release_action("autopilot_goto"))
        .until(elapsed(0.5))
        .add()
        .step("assert the trip is flying")
        .on_enter(assert_the_trip_is_flying)
        .until(elapsed(0.2))
        .add()
        .step("fly the cover onto the trip's line")
        .on_enter(move_the_cover_onto_the_trip)
        .until(the_designation_let_go())
        .deadline(DROP_SECS)
        .add()
        .step("assert the cover drops the designation")
        .on_enter(assert_the_cover_drops_the_designation)
        .until(elapsed(TRIP_SECS))
        .add()
        // The script's last beat: the run ends on the assertion rather than
        // idling out a runway.
        .step("assert the trip flew on")
        .on_enter(assert_the_trip_flew_on)
        .add()
}
