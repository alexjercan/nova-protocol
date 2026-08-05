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
//! anyway (`reel_freeze_bodies`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/reel NOVA_REEL=1 \
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
/// Deliberately weak (the default is 6.0). These shots illustrate gravity, so
/// the body in them is a real well - but at full strength it would haul the
/// posed set out of frame during a long look.
const PLANETOID_GRAVITY: f32 = 0.3;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // NOT debug-gated: the rig is the scene's look, so a plain run shows what a
    // capture would shoot.
    app.add_plugins(kit::photo_rig());

    #[cfg(feature = "debug")]
    {
        // Smoke path: reach Playing on the built scene and exit clean.
        app.add_plugins(nova_autopilot());
        // Capture path: pose the camera at each beat and shoot.
        app.add_plugins(nova_reel(scene_beats()));
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
        id: "drydock_drift".to_string(),
        name: "Drydock Drift".to_string(),
        description: "A salvage yard adrift over a planetoid - the website's beauty set."
            .to_string(),
        cubemap: game_assets.cubemap.clone().into(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: vec![
                planetoid(game_assets),
                belt.action(game_assets),
                hero,
                hauler_a,
                hauler_b,
            ],
        }],
        ..Default::default()
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
            surface_gravity: Some(PLANETOID_GRAVITY),
            invulnerable: true,
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
            controller,
            allegiance,
            sections,
        }),
    })
}

/// The three pure-3D beats this set produces. First-pass framings: the capture
/// step reviews them at the site's 16:9 page crop and tunes them here.
#[cfg(feature = "debug")]
fn scene_beats() -> Vec<ReelBeat> {
    let beat = |eye: Vec3, look: Vec3, name: &str| reel_beat(ReelCamera::new(eye, look), name);
    vec![
        // The gravity feature: hero in the near left, the planetoid behind it
        // down-right, belt rocks between the two for depth.
        beat(
            Vec3::new(-6.5, 2.6, 9.5),
            Vec3::new(0.0, 0.0, -2.0),
            "feature-gravity.png",
        ),
        // The planetoid as the subject: closer in, the body filling the lower
        // half with the yard's rocks passing in front of it.
        beat(
            Vec3::new(40.0, -6.0, -110.0),
            PLANETOID_POSITION,
            "wiki-gravity.png",
        ),
        // The hero, close: a three-quarter beauty pass where the hull's
        // sections read.
        beat(
            Vec3::new(7.0, 2.5, 9.0),
            Vec3::new(0.0, 0.2, 0.0),
            "wiki-sections.png",
        ),
    ]
}
