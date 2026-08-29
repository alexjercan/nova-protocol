//! screenshot_radar_lock: the weapons-lowered NAV radar latching a beacon
//! downrange.
//!
//! Ships `tutorial-radar-lock.png` (the latch, from the game's own follow
//! camera) and `wiki-radar.png` (the same latch from a tighter, lower camera,
//! with the instrument as the subject).
//!
//! The set is a corvette in open space with a nav beacon roughly 750 units
//! ahead. Nothing flies: the lock is the whole example.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - reach Playing, drive the whole
//!   script, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also capture the shots (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_radar_lock --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_radar_lock --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[path = "shared/kit.rs"]
mod kit;

use std::collections::BTreeMap;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use nova_debug::prelude::capturing;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_radar_lock")]
#[command(version = "1.0.0")]
#[command(about = "Capture the nav radar latching a beacon downrange. Autopilot-only: the script holds the radar gesture and poses the instrument shot", long_about = None)]
struct Cli;

#[cfg(feature = "debug")]
const LOCK_LOOP: &str = "lock-dwell";

/// Scenario id of the player's ship.
const PLAYER_ID: &str = "nav_player";
/// Where the ship sits: up, out and roughly 750 units short of the beacon, so
/// the bracket has open sky around it.
const START_POSITION: Vec3 = Vec3::new(0.0, -60.0, 754.0);

/// The nav beacon: the travel lock's subject. Sitting a long way downrange is
/// the point - a destination a few units off the nose puts its bracket over the
/// player's own hull, which is exactly what this framing must not do.
const BEACON_ID: &str = "nav_beacon";
/// Far enough ahead that the lock reads as a destination rather than as traffic.
const BEACON_POSITION: Vec3 = Vec3::new(0.0, 0.0, -46.0);
/// Radar signature, world units of lock range per unit being 30: the default 20
/// (600 units) is short of this range, so the beacon authors the range the shot
/// needs, the way the doc comment on the field asks.
const BEACON_SIGNATURE: f32 = 40.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window,
        // so a captured fps would measure the script, not the engine.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(radar_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShips>) {
    commands.trigger(LoadScenario(nav_approach(&game_assets, &ships)));
}

/// The set: a parked corvette, a beacon 750 units downrange and a corridor of
/// rocks between them.
fn nav_approach(game_assets: &GameAssets, ships: &GameShips) -> ScenarioConfig {
    let player = ship(
        PLAYER_ID,
        "Player Ship",
        START_POSITION,
        // Square with the world, NOT nosed at the beacon: the radar picks by
        // the CAMERA's look ray (`ActiveLookRay`), which opens down world -Z
        // whatever the hull is doing, and the start offset is chosen so the
        // beacon sits a few degrees off that ray - inside the 18-degree radar
        // cone, and clear of the player's own hull in frame.
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::new(),
            speed_cap: None,
            infinite_ammo: true,
        }),
        None,
        kit::kenney_hull(ships, "cargoa"),
    );

    // The corridor: big rocks spread wide around the beacon, so the shot has
    // something with parallax in it instead of an empty starfield.
    let corridor = kit::NearField {
        id_prefix: "hollow_far_",
        count: 26,
        seed: 90727,
        distance: (200.0, 640.0),
        radius: (4.0, 10.0),
        y_spread: 160.0,
    };

    ScenarioConfig {
        description: "A corvette latching a nav beacon downrange.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            // The photo rig is authored content rather than an example-side
            // observer swap: scale 1.0 around the origin reproduces the kit's
            // exact key/rim/fill numbers.
            actions: [
                vec![corridor.action(game_assets), player, beacon()],
                ThreePointRig::around("photo", Vec3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "nav_approach".to_string(),
            "Nav Approach".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The nav beacon: the travel lock's subject, and nothing else. No trigger area
/// - nothing in this set springs on arrival.
fn beacon() -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: BEACON_ID.to_string(),
            name: "Waypoint".to_string(),
            position: BEACON_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: "WAYPOINT".to_string(),
            radius: 3.0,
            color: Color::srgb(0.4, 0.75, 1.0),
            area_radius: None,
            lock_signature: Some(BEACON_SIGNATURE),
        }),
    })
}

/// One posed ship in the set.
fn ship(
    id: &str,
    name: &str,
    position: Vec3,
    rotation: Quat,
    controller: SpaceshipController,
    allegiance: Option<Allegiance>,
    sections: Vec<SpaceshipSectionConfig>,
) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller,
            allegiance,
            hull: ShipSource::Inline(ShipHull {
                sections,
                ..default()
            }),
            ..default()
        }),
    })
}

/// Latch the beacon, shoot it, then re-frame the instrument and shoot it again.
///
/// Every capture is its OWN step held until the PNG is on disk: Bevy services
/// one primary-window capture per frame, so the rule is structural here rather
/// than a guard inside a shared step.
#[cfg(feature = "debug")]
fn radar_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the approach")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle at the start")
        .on_enter(hud_instrument)
        .until(elapsed(1.0))
        .add();

    if capturing() {
        script = script
            .step("open the lock loop")
            .on_enter(|world| loop_start(world, LOCK_LOOP))
            .until(frames(1))
            .add();
    }

    // Weapons lowered, hold CTRL: the nav-slot radar opens and the white NAV
    // crosshair sweeps onto the beacon downrange. Waiting on the LOCK rather
    // than on a guessed second - and naming the beacon - so a run that latches
    // a rock instead aborts here and says so.
    script = script
        .step("sweep the nav radar")
        .on_enter(hold_radar)
        .until(travel_locked_on_beacon())
        .deadline(12.0)
        .add();

    if capturing() {
        script = script
            .step("hold the completed lock")
            .until(elapsed(0.8))
            .add()
            .step("close the lock loop")
            .on_enter(|world| loop_end(world, LOCK_LOOP))
            .until(loop_written(LOCK_LOOP))
            .deadline(60.0)
            .add();
    }

    script
        .step("capture the radar lock")
        .on_enter(move |world| shoot(world, "tutorial-radar-lock.png"))
        .until(shot_written("tutorial-radar-lock.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // The radar instrument as its own subject: the same latched nav sweep
        // from a tighter, lower camera.
        .step("frame the radar instrument")
        .on_enter(|world| {
            pose(
                world,
                START_POSITION + Vec3::new(-3.6, 1.0, 6.0),
                START_POSITION + Vec3::new(0.0, 0.3, -10.0),
            )
        })
        .until(elapsed(0.4))
        .add()
        .step("capture the radar")
        .on_enter(move |world| shoot(world, "wiki-radar.png"))
        .until(shot_written("wiki-radar.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}

/// Put the HUD on (the contextual rules decide what is actually in shot).
#[cfg(feature = "debug")]
fn hud_instrument(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::On;
    }
}

/// Pin the camera for a framing the follow camera does not give.
#[cfg(feature = "debug")]
fn pose(world: &mut World, position: Vec3, look_at: Vec3) {
    pose_camera(world, position, look_at);
}

/// Hold the radar gesture. Weapons are lowered, so it latches the nav slot.
#[cfg(feature = "debug")]
fn hold_radar(world: &mut World) {
    press_action("radar_hold")(world);
}

/// Advance once the travel lock is on the beacon (and not on some rock the aim
/// ray happened to cross).
#[cfg(feature = "debug")]
fn travel_locked_on_beacon() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(player) = player_root_ref(world) else {
            return false;
        };
        let Some(TravelLock(Some(target))) = world.get::<TravelLock>(player) else {
            return false;
        };
        world
            .get::<EntityId>(*target)
            .is_some_and(|id| id.0 == BEACON_ID)
    })
}

/// The player's ship root, from a read-only world (what a predicate gets).
#[cfg(feature = "debug")]
fn player_root_ref(world: &World) -> Option<Entity> {
    world
        .try_query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()?
        .iter(world)
        .next()
}
