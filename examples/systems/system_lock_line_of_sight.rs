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
//! FOUR named invariants:
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a clear line takes the lock` | the range locks at all |
//! | 2 | `outcome: cover breaks a held lock` | and names the branch |
//! | 3 | `outcome: cover keeps a lock from being taken` | the picker agrees |
//! | 4 | `outcome: a cleared line gives the lock back` | cover, not a ban |
//!
//! Invariant 1 is the control: without it the three that follow are satisfied
//! by a range that could never lock anything.
//!
//! Controls: none needed; fly and look around freely in interactive runs.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_lock_line_of_sight --features debug
//! # look for: `line of sight: the clear line locked the target`,
//! #           `line of sight: the lock let go, reason Occluded`,
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

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

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

#[cfg(feature = "debug")]
fn sight_script() -> Script {
    Script::new()
        .step("load the range")
        .enter(GameStates::Loading)
        .until(scenario_camera_present())
        .deadline(30.0)
        .add()
        .step("wait for the cast")
        .until(the_cast_is_present())
        .deadline(30.0)
        .add()
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
        // The script's last beat: the run ends on the assertion rather than
        // idling out a runway.
        .step("assert the cleared line locks again")
        .on_enter(assert_the_cleared_line_locks_again)
        .add()
}
