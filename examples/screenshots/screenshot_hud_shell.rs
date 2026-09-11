//! screenshot_hud_shell: the directional-HUD shells around the biggest hull in
//! the fleet.
//!
//! The visual fixture for task 20260909-212917. The velocity sphere and the
//! gravity sphere have to ENCLOSE the hull they belong to, whatever its size,
//! so the set stages the 33-cell industrial carrier - 2 081 sections, 360 m
//! stem to stern - under way and inside a real well, and films both shells
//! around it from one pinned broadside.
//!
//! Everything the frame shows is live: the player's own carrier flying the
//! production HUD, a planetoid whose SOI the ship is really inside (so the
//! yellow gravity shell is up rather than hidden in flat space), and a
//! constant velocity written every frame so the white cone has a direction to
//! ride. The hull is held on station for the capture, which is what makes the
//! before-and-after pair differ only by the fix.
//!
//! Ships `wiki-hud-shell.png` (the wiki's focused figure under Flight
//! readouts) plus `hud-shell-carrier.png`, the wide containment shot the fix
//! was graded against.
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
//!   cargo run --example screenshot_hud_shell --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_hud_shell --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[path = "shared/kit.rs"]
mod kit;

use std::collections::BTreeMap;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_hud_shell")]
#[command(version = "1.0.0")]
#[command(about = "Capture the velocity and gravity shells around the fleet's biggest hull. Autopilot-only: a posed set behind a scripted camera", long_about = None)]
struct Cli;

/// The player carrier's scenario id.
const CARRIER_ID: &str = "hud_shell_carrier";

/// The planetoid's scenario id.
const PLANETOID_ID: &str = "hud_shell_planetoid";

/// How far below the carrier the well sits. Inside the body's 4.9 km SOI, so
/// the ship really is in a well and the yellow shell is really up; far enough
/// down that the broadside framings keep the carrier against open sky instead
/// of against terrain.
const PLANETOID_POSITION: Meters3 = Meters3::new(0.0, -3_000.0, 0.0);

/// The body's mean radius. Its mass parameter is capped at
/// `max_surface_gravity * body_radius^2`, so the SOI a set can reach is a
/// property of SIZE: a small body cannot hold a 4.9 km well however heavy it
/// is authored.
const PLANETOID_RADIUS: Meters = Meters(900.0);

/// Which ice world this is. Required, and deliberately so: this figure is
/// framed against one body, and which body cannot be a number the engine picks
/// per run.
const PLANETOID_SEED: u32 = 4_711;

/// The body's mass parameter (mu), world units cubed per second squared - a
/// gravitational parameter, not a length, which is why it carries no meter
/// type. `soi = sqrt(mu / soi_cutoff_accel)` puts the boundary at 490 world
/// units (4.9 km), and it is under the cap for a 900 m body, so the authored
/// figure is the flown one.
const PLANETOID_MASS: f32 = 60_000.0;

/// What the carrier is doing: a steady cruise along its own nose, so the white
/// cone rides a real direction and the speed chip has a real number. Written
/// every frame ([`hold_carrier_velocity`]) because the hull is held on station
/// for the capture and a held hull integrates no velocity of its own.
///
/// Along the nose rather than across it because the broadside cameras stand off
/// the beam: a cone riding the shell straight at the lens would sit on top of
/// the hull instead of marking the shell's edge.
#[cfg(feature = "debug")]
const CARRIER_VELOCITY: MetersPerSecond = MetersPerSecond(140.0);

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
        // Clean frames at a known 16:9: force the window size and drop the dev
        // overlays. The HUD itself STAYS - it is the subject.
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        // The set is posed, so it must not drift between framings - but only
        // on a capture run, so a plain `cargo run` keeps its physics and the
        // carrier really does fly.
        app.add_systems(Update, freeze_bodies.run_if(capturing));
        app.add_systems(Update, hold_carrier_velocity);
        app.add_plugins(hud_shell_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShips>) {
    commands.trigger(LoadScenario(hud_shell_set(&game_assets, &ships)));
}

/// The set: the player's carrier at the origin and the well it is falling
/// around, lit by the photo rig.
fn hud_shell_set(game_assets: &GameAssets, ships: &GameShips) -> ScenarioConfig {
    let carrier = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: CARRIER_ID.to_string(),
            name: "Carrier".to_string(),
            position: Meters3::ZERO,
            // Square with the world, nose down -Z: the broadside cameras stand
            // off on +X, so the hull presents its full 360 m length and the
            // shells have the longest axis to fail to contain.
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: BTreeMap::new(),
                speed_cap: None,
            }),
            allegiance: None,
            hull: ShipSource::Inline(kit::catalog_ship(ships, "block_carrier")),
            ..default()
        }),
    });

    let planetoid = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLANETOID_ID.to_string(),
            name: "Planetoid".to_string(),
            position: PLANETOID_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Planet(
            PlanetConfig::new(PlanetType::IceWorld, PLANETOID_RADIUS, PLANETOID_SEED)
                .anchored(PLANETOID_MASS),
        ),
    });

    ScenarioConfig {
        description: "The carrier under way inside a well, for the HUD shell figures.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            // The photo rig, authored content rather than an example-side
            // observer swap: scale 1.0 around the origin reproduces the kit's
            // exact key/rim/fill numbers, so the captured frames are unchanged.
            actions: [
                vec![planetoid, carrier],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "hud_shell".to_string(),
            "HUD Shell".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Write the carrier's cruise velocity every frame.
///
/// The capture run freezes every dynamic body so the posed set cannot drift,
/// and a frozen hull integrates nothing - including the velocity the white cone
/// and the speed chip are both readouts OF. Writing it keeps the instruments
/// reporting a real quantity off a hull that does not move.
#[cfg(feature = "debug")]
fn hold_carrier_velocity(
    mut q_player: Query<
        &mut avian3d::prelude::LinearVelocity,
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    for mut velocity in &mut q_player {
        velocity.0 = Vec3::NEG_Z * CARRIER_VELOCITY.to_engine();
    }
}

/// Put the HUD on and take the fps/version status bar back out.
///
/// The bar is not part of the instrument set this figure is showing: its
/// version item names the commit the debug build came from, so every re-shoot
/// bakes a different hash into the image, and its fps item puts the capture
/// rig's cadence on the page.
#[cfg(feature = "debug")]
fn hud_instrument(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::On;
    }
    hide_status_bar(world);
}

/// Pose the two framings and shoot each one.
///
/// Every capture is its OWN step held until the PNG is on disk: Bevy services
/// one primary-window capture per frame, so the rule is structural here rather
/// than a guard inside a shared step.
#[cfg(feature = "debug")]
fn hud_shell_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the carrier")
        .enter(GameStates::Loading)
        .until(and(player_ship_present(), scenario_camera_present()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // The hull is 2 081 sections of derived cladding and the well is a
        // generated ice world; both land over several frames after the ship
        // root exists, and the shells are sized off the LIVE section set.
        .step("settle the set")
        .until(elapsed(3.0))
        .add()
        // The containment shot: the whole 360 m hull on the beam, framed so a
        // shell standing 5 m clear of it still has open sky behind it. Both
        // cones are on the far edges of the shell from here - the white one
        // down the nose, the yellow one down at the well.
        .step("frame the containment shot")
        .on_enter(|world: &mut World| {
            hud_instrument(world);
            pose_camera(
                world,
                Meters3::new(700.0, 140.0, 0.0),
                Meters3::new(0.0, 0.0, 0.0),
            );
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot hud-shell-carrier.png")
        .on_enter(|world: &mut World| shoot(world, "hud-shell-carrier.png"))
        .until(shot_written("hud-shell-carrier.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // The wiki figure: the same hull three-quarters on and closer, so the
        // page shows the shells hugging a hull rather than a diagram of a
        // sphere. The chips ride the outer shell's edge, which is the other
        // half of what the figure is for.
        .step("frame the wiki figure")
        .on_enter(|world: &mut World| {
            pose_camera(
                world,
                Meters3::new(500.0, 160.0, 380.0),
                Meters3::new(0.0, 0.0, -20.0),
            );
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot wiki-hud-shell.png")
        .on_enter(|world: &mut World| shoot(world, "wiki-hud-shell.png"))
        .until(shot_written("wiki-hud-shell.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
