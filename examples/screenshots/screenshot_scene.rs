//! screenshot_scene: "Drydock drift" - the beauty set the website's pure-3D
//! shots are taken from.
//!
//! A hero Kenney racer posed in the foreground, a near-field rock belt close
//! enough to be IN the frame, two neutral cargo hulls flying slow loops through
//! the yard, and a planetoid far enough back that its surface reads as a body
//! rather than a wall. The photo kit lights it (key + rim + fill) instead of
//! the scenario's single top-down key.
//!
//! It replaces `screenshot_reel`, whose set was a primitive-block prop ship, a
//! 4-unit "planetoid" and a rock field scattered 90-180 units out - off camera
//! in every framing it was supposed to dress.
//!
//! The scene is built in Rust, not a scenario RON: [`kit::kenney_hull`] derives
//! the 18-to-54 section entries of each Kenney hull from the section catalog, so
//! the RON it would write is generated anyway (which is exactly what the
//! shipped `menu_*` / `broadside` files are - the same list, typed out).
//!
//! THE SET DRIFTS, on purpose and in the name: the planetoid is a real well
//! (these shots illustrate gravity, so the body in them had better be one),
//! and at this range it pulls the yard at about 0.01 u/s^2 - metres of drift
//! over a long free-fly look, nothing over a capture, which freezes the scene
//! anyway (`freeze_bodies`).
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`) - the script
//! below is the whole example, and `NOVA_CAPTURE` only decides whether its
//! shot steps write a PNG:
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - walk every framing, reach
//!   Playing, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot each framing (staged under
//!   `NOVA_SHOT_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_scene --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_scene --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[path = "shared/kit.rs"]
mod kit;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_scene")]
#[command(version = "1.0.0")]
#[command(about = "The Drydock drift beauty set for the website's 3D shots", long_about = None)]
struct Cli;

/// The planetoid's scenario id.
const PLANETOID_ID: &str = "drydock_planetoid";
/// Where the planetoid sits: down and well behind the yard, so a wide shot has
/// a body in it and the hero still has open space around it.
const PLANETOID_POSITION: Vec3 = Vec3::new(170.0, -95.0, -560.0);
/// Big enough for its surface to read at that distance (the old reel's was 4
/// units across and read as a pebble). The generated rock reaches well past its
/// nominal radius, so 30 units here draws a body roughly 120 across.
const PLANETOID_RADIUS: f32 = 30.0;
/// The body's mass parameter (mu, u^3/s^2), deliberately weak: these shots
/// illustrate gravity, so the body in them is a real well - but at the default
/// 45 000 it would haul the posed set out of frame during a long look. Reach
/// follows strength now, so a weak well is also a SHORT one: the SOI is 156u,
/// and nothing posed further out than that is inside it.
const PLANETOID_MASS: f32 = 6_000.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PERF_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window,
        // so a captured fps would measure the script, not the engine.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        // Clean frames at a known 16:9: force the window size, drop the dev
        // overlays and the HUD chrome (this set carries no player HUD, so the
        // fps/version bar is just clutter).
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        // The scene is posed, so it must not drift between framings - but only
        // on a capture run, so a plain `cargo run` keeps its physics and the
        // yard really does drift.
        app.add_systems(Update, freeze_bodies.run_if(capturing));
        app.add_plugins(scene_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(drydock_drift(&game_assets, &sections)));
}

/// The set: planetoid, near-field belt, hero racer, two drifting hulls.
fn drydock_drift(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    // The hero: a Kenney racer at the origin, turned three-quarters and tipped
    // off the horizontal so it reads as a ship parked in a yard rather than a
    // model on a shelf.
    let hero = ship(
        "drydock_hero",
        "Hero Racer",
        Vec3::ZERO,
        Quat::from_rotation_y(-0.55) * Quat::from_rotation_x(0.10) * Quat::from_rotation_z(0.18),
        SpaceshipController::None,
        None,
        kit::kenney_hull(sections, "racer"),
    );

    // The yard traffic: two cargo hulls flying slow loops around the hero,
    // NEUTRAL so they stay bystanders (nothing in this scene is meant to shoot
    // anything). Their routes run down opposite flanks at different heights, so
    // no framing catches them in a row.
    let hauler_a = ship(
        "drydock_hauler_a",
        "Hauler",
        Vec3::new(-52.0, 9.0, -64.0),
        Quat::from_rotation_y(0.9),
        patrolling(vec![
            Vec3::new(-52.0, 9.0, -64.0),
            Vec3::new(-96.0, 22.0, 30.0),
            Vec3::new(-30.0, 4.0, 86.0),
        ]),
        Some(Allegiance::Neutral),
        kit::kenney_hull(sections, "cargoa"),
    );
    let hauler_b = ship(
        "drydock_hauler_b",
        "Hauler",
        Vec3::new(74.0, -16.0, -98.0),
        Quat::from_rotation_y(-2.1),
        patrolling(vec![
            Vec3::new(74.0, -16.0, -98.0),
            Vec3::new(120.0, -30.0, -10.0),
            Vec3::new(58.0, -8.0, 60.0),
        ]),
        Some(Allegiance::Neutral),
        kit::kenney_hull(sections, "cargob"),
    );

    let belt = kit::NearField {
        id_prefix: "drydock_rock_",
        count: 18,
        seed: 90210,
        distance: (60.0, 165.0),
        radius: (1.5, 5.0),
        y_spread: 70.0,
    };

    ScenarioConfig {
        description: "A salvage yard adrift over a planetoid - the website's beauty set."
            .to_string(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            // The photo rig, now authored content rather than an example-side
            // observer swap: scale 1.0 around the origin reproduces the kit's
            // exact key/rim/fill numbers, so the captured frames are unchanged.
            actions: [
                vec![
                    planetoid(game_assets),
                    belt.action(game_assets),
                    hero,
                    hauler_a,
                    hauler_b,
                ],
                ThreePointRig::around("photo", Vec3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "drydock_drift".to_string(),
            "Drydock Drift".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The planetoid: the set's backdrop body and its only gravity source.
/// Invulnerable - nothing should be able to shoot the scenery out of a shot.
fn planetoid(game_assets: &GameAssets) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLANETOID_ID.to_string(),
            name: "Planetoid".to_string(),
            position: PLANETOID_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: PLANETOID_RADIUS,
            texture: game_assets.asteroid_texture.clone().into(),
            health: 5000.0,
            impact_sound: None,
            destroy_sound: None,
            mass: Some(PLANETOID_MASS),
            invulnerable: true,
            seed: None,
            lock_signature: None,
        }),
    })
}

/// A hull's routine: a slow loop around the yard.
///
/// Patrol, not `orbit`: the planetoid sits far enough back to read as a body
/// rather than a wall, which puts the yard outside the orbit band the AI flies
/// to - an orbiting hull would leave the set for the rock. A waypoint loop
/// keeps the traffic where the camera is.
fn patrolling(route: Vec<Vec3>) -> SpaceshipController {
    SpaceshipController::AI(AIControllerConfig {
        patrol: route,
        ..default()
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
            collapse_threshold: None,
            skin: false,
            style: None,
            controller,
            allegiance,
            sections,
        }),
    })
}

/// Seconds a step may sit before it is called a stall. Sized with headroom for
/// a slow software-rendered CI GPU (llvmpipe). An expiry is an error exit
/// naming the step, so a run that never loads the scene fails loudly instead of
/// producing an unframed shot.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 30.0;

/// One framing of the set: where the camera stands, what it looks at, and the
/// PNG it produces.
#[cfg(feature = "debug")]
struct SceneShot {
    eye: Vec3,
    look: Vec3,
    path: &'static str,
}

/// The three pure-3D framings this set produces. First-pass framings: the
/// capture step reviews them at the site's 16:9 page crop and tunes them here.
#[cfg(feature = "debug")]
const SCENE_SHOTS: [SceneShot; 3] = [
    // The gravity feature: hero in the near left, the planetoid behind it
    // down-right, belt rocks between the two for depth.
    SceneShot {
        eye: Vec3::new(-6.5, 2.6, 9.5),
        look: Vec3::new(0.0, 0.0, -2.0),
        path: "feature-gravity.png",
    },
    // The planetoid as the subject: closer in, the body filling the lower half
    // with the yard's rocks passing in front of it.
    SceneShot {
        eye: Vec3::new(40.0, -6.0, -110.0),
        look: PLANETOID_POSITION,
        path: "wiki-gravity.png",
    },
    // The hero, close: a three-quarter beauty pass where the hull's sections
    // read.
    SceneShot {
        eye: Vec3::new(7.0, 2.5, 9.0),
        look: Vec3::new(0.0, 0.2, 0.0),
        path: "wiki-sections.png",
    },
];

/// The whole driven walk: wait for the scene, then pose-settle-shoot each
/// framing in turn.
///
/// Written as autopilot steps rather than a beat list so the framing and the
/// shot it produces read top-to-bottom in one place, and so the smoke path and
/// the capture path are the SAME walk - the only difference is whether `shoot`
/// is armed.
#[cfg(feature = "debug")]
fn scene_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        // The scene is live once the loader has spawned its camera; posing
        // before that poses nothing.
        .step("wait for the drydock scene")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add();

    for shot in SCENE_SHOTS {
        let (eye, look, path) = (shot.eye, shot.look, shot.path);
        script = script
            .step(format!("frame {path}"))
            .on_enter(move |world: &mut World| pose_camera(world, eye, look))
            .until(frames(SETTLE_FRAMES))
            .add()
            // The shot step holds until the PNG is on disk, so the next
            // framing cannot move the camera out from under a pending write.
            .step(format!("shoot {path}"))
            .on_enter(move |world: &mut World| shoot(world, path))
            .until(shot_written(path))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
    }
    script
}
