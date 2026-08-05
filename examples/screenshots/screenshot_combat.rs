//! screenshot_combat: "Rock hollow" - the combat and HUD set.
//!
//! A clearing inside a dense rock shell. The player's racer sits in the middle
//! with a derelict raider parked ahead of it (the lock subject and the ship
//! that loses a section on camera), two friendly racers hold the flanks, and a
//! hostile pair works the far side of the hollow. It replaces the old
//! `screenshot_combat` range (three primitive blocks on an empty backdrop) and
//! absorbs `screenshot_juice`, whose scripted section blow is one beat here.
//!
//! WHAT THE PROOF RUN SHOWED (2026-08-05), and why the set is shaped like this:
//! two AI flights DO fight each other with no player in the scene - a flight
//! spawned `allegiance: Some(Player)` acquires the default-Enemy flight within
//! a second and both sides run their guns continuously. Nothing in acquisition
//! is player-specific (`crates/nova_gameplay/src/input/ai/acquisition.rs`), and
//! now it has been run. What they will NOT do is brawl: the engage maneuver
//! flies to `AI_STANDOFF_RANGE` (250 units) and EXTENDS AWAY once inside the
//! band, so a fight is a wide slow ring, and turret bullets (100 u/s, 5 s life)
//! never connect at that range - 45 seconds of continuous fire took zero
//! sections off. So the AI pairs are the set's live background - tracer streams
//! and moving hulls - while every close beat is authored: the lock subject is
//! PARKED, and the destruction is scripted through the real damage path.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - reach Playing, drive the whole
//!   script, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_REEL=1`: also capture the shots (staged under
//!   `NOVA_SHOT_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/reel NOVA_AUTOPILOT=1 NOVA_REEL=1 \
//!   cargo run --example screenshot_combat --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_combat --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[path = "shared/kit.rs"]
mod kit;

use bevy::{platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_combat")]
#[command(version = "1.0.0")]
#[command(about = "The Rock hollow combat and HUD set", long_about = None)]
struct Cli;

/// Scenario id of the parked raider the player locks, shoots and finally blows
/// a section off.
const RAIDER_ID: &str = "hollow_raider";
/// Where it sits: dead ahead, inside the lock cone and close enough that the
/// hull reads at the chase framing.
const RAIDER_POSITION: Vec3 = Vec3::new(0.0, 0.4, -22.0);
/// The raider section the scripted blow takes off - a forward hull cube on the
/// camera's side of the ship, so the fragments and the hole are both in frame.
const RAIDER_BLOWN_SECTION: &str = "racer_cube_i0_j0_km2";
/// The nav beacon the weapons-lowered radar sweep latches. BEYOND the raider
/// and off the axis: a beacon between camera and subject puts its glow over the
/// player's own hull, and the orb is bright enough to blow out half the frame.
const BEACON_POSITION: Vec3 = Vec3::new(2.6, 1.0, -34.0);

/// Seconds each AI flight holds fire after the scenario starts, so the shots
/// are taken of a fight that has settled rather than of four ships still
/// sorting out where they are.
const ENGAGE_DELAY: f32 = 3.0;
/// How far an AI ship may stray from its post before it breaks off and comes
/// back. Wider than the standoff range the engage maneuver flies to (250), so
/// the fight is not permanently interrupted, tight enough that the hollow keeps
/// its ships instead of watching them leave.
const AI_LEASH: f32 = 320.0;

/// Frames a capture step holds after requesting its PNG. `capture_window`
/// spawns a bare `Screenshot` and is NOT a completion collector, so the last
/// step's hold is the only thing giving `save_to_disk` room to land before the
/// driver reports done and the app exits. A smoke run captures nothing and only
/// needs the step to be observable.
#[cfg(feature = "debug")]
fn capture_settle_frames(capturing: bool) -> u32 {
    if capturing {
        20
    } else {
        2
    }
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // NOT debug-gated: the rig is the set's look, so a plain run shows what a
    // capture would shoot.
    app.add_plugins(kit::photo_rig());

    #[cfg(feature = "debug")]
    {
        let capturing = std::env::var_os(nova_protocol::nova_debug::harness::REEL_ENV).is_some();
        // One step per beat, and every capture gets its OWN step: Bevy services
        // one primary-window capture per frame, so the rule is structural here
        // rather than a guard inside a shared step.
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                .step("load the hollow")
                .enter(GameStates::Loading)
                .until(player_ship_present())
                .deadline(30.0)
                .add()
                // Let the flights take up their posts and open fire before any
                // shot is taken - ENGAGE_DELAY plus a few seconds of closing.
                // The player holds station through all of it - see
                // [`pin_player`] for why the set cannot tolerate its drift.
                .step("let the flights engage")
                .until(elapsed(ENGAGE_DELAY + 3.0))
                .add()
                // Quiet stance first: weapons lowered, no lock, the contextual
                // rules keeping idle chrome off the frame. That IS the shot.
                .step("capture the contextual HUD")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "news-090-contextual-hud.png");
                })
                .until(elapsed(0.3))
                .add()
                // Weapons still lowered, hold CTRL: the nav-slot radar opens and
                // the white NAV crosshair sweeps onto the beacon.
                .step("sweep the nav radar")
                .on_enter(hold_radar)
                .until(elapsed(1.2))
                .add()
                .step("capture the radar lock")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "tutorial-radar-lock.png");
                })
                .until(elapsed(0.2))
                .add()
                // The radar instrument as its own subject: the same latched nav
                // sweep from a tighter, lower camera, so the sweep and the
                // hollow behind it fill the frame.
                .step("frame the radar instrument")
                .on_enter(|world| {
                    pose(world, Vec3::new(-3.6, 1.0, 6.0), Vec3::new(0.0, 0.3, -10.0))
                })
                .until(elapsed(0.4))
                .add()
                .step("capture the radar")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "wiki-radar.png");
                })
                .until(elapsed(0.2))
                .add()
                // A LATER step than the captures that want it: despawning in a
                // capture frame removes the beacon before that frame's render,
                // and its glow must not sit on the reticle in the combat shots.
                .step("clear the nav beacon")
                .on_enter(|world| {
                    release_radar(world);
                    despawn_by_id(world, "hollow_beacon");
                    unpose(world);
                })
                .until(elapsed(0.3))
                .add()
                // Raise weapons (RMB), then hold radar (CTRL) a beat later - the
                // natural order. At the hold threshold the radar latches the
                // combat slot on the raider and the reticle + inset come up.
                .step("raise the weapons")
                .on_enter(raise_stance)
                .until(elapsed(0.3))
                .add()
                .step("latch the combat lock")
                .on_enter(hold_radar)
                .until(elapsed(1.8))
                .add()
                // Guns live: the player's own turret streams tracers at the
                // locked raider, so the combat frames have the player's fire in
                // them and not just the AI's.
                .step("open fire")
                .on_enter(open_fire)
                .until(elapsed(0.8))
                .add()
                .step("capture the combat frame")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "feature-combat.png");
                })
                .until(elapsed(0.2))
                .add()
                .step("capture the combat lock")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "tutorial-combat-lock.png");
                })
                .until(elapsed(0.2))
                .add()
                // The same fight from the ship's shoulder: the HUD showcase,
                // every situational readout up with the hull in frame.
                .step("frame the HUD showcase")
                .on_enter(|world| pose(world, Vec3::new(5.0, 1.6, 7.0), Vec3::new(0.0, 0.4, -14.0)))
                .until(elapsed(0.4))
                .add()
                .step("capture the HUD in combat")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "feature-hud.png");
                })
                .until(elapsed(0.2))
                .add()
                .step("frame the HUD reference")
                .on_enter(|world| {
                    pose(world, Vec3::new(-6.0, 2.8, 5.0), Vec3::new(0.0, 0.2, -16.0))
                })
                .until(elapsed(0.4))
                .add()
                .step("capture the HUD reference")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "wiki-hud.png");
                })
                .until(elapsed(0.2))
                .add()
                // The wide beats: the HUD goes cinematic and the camera leaves
                // the player, so the chrome would be reading a ship the frame
                // is no longer with. They are framed on the PLAYER's tracer
                // stream, not on the AI pairs: the AI holds a 250-unit standoff
                // where its ships are specks, while the player's fire crosses
                // 36 units of open hollow into a hull.
                .step("frame the exchange")
                .on_enter(|world| {
                    hud_cinematic(world);
                    pose(world, Vec3::new(9.0, 2.5, 6.0), Vec3::new(0.0, 0.3, -14.0));
                })
                .until(elapsed(0.5))
                .add()
                .step("capture the exchange")
                .on_enter(move |world| shoot(world, capturing, "wiki-combat.png"))
                .until(elapsed(0.2))
                .add()
                // The receiving end: past the raider's shoulder back down the
                // stream, so the frame is bullets, impact flashes and a hull
                // taking them.
                .step("frame the readability shot")
                .on_enter(|world| {
                    pose(
                        world,
                        RAIDER_POSITION + Vec3::new(-7.0, 2.0, -8.0),
                        Vec3::ZERO,
                    )
                })
                .until(elapsed(0.5))
                .add()
                .step("capture the readability shot")
                .on_enter(move |world| shoot(world, capturing, "news-090-combat-readability.png"))
                .until(elapsed(0.2))
                .add()
                // The juice: one section blown off the raider through the
                // production damage path, shot while the fragments and hit
                // rings are still live.
                // Three-quarter on the raider from inside the fire line, so
                // the frame carries the player's incoming tracers, the impact
                // flashes and the burnt-out section together. The destruction
                // is a BURNT HULL, not a fireball: a dead section is graded to
                // `DEAD_COLOR` in place (`sections/damage_tint.rs`) and the
                // burst is over in a frame or two, so the shot is the damage,
                // shown while the rounds are still arriving.
                .step("frame the raider")
                .on_enter(|world| {
                    pose(
                        world,
                        RAIDER_POSITION + Vec3::new(6.5, 2.2, 9.0),
                        RAIDER_POSITION,
                    )
                })
                .until(elapsed(0.5))
                .add()
                .step("blow a section off the raider")
                .on_enter(blow_raider_section)
                .until(frames(12))
                .add()
                .step("capture the juice")
                .on_enter(move |world| shoot(world, capturing, "feature-juice.png"))
                .until(frames(capture_settle_frames(capturing)))
                .add(),
        );
        app.add_systems(Startup, (force_resolution, hide_dev_overlays));
        // Only under the script (`NOVA_AUTOPILOT`, named literally - the
        // harness re-exports `REEL_ENV` but not the autopilot's own): a plain
        // run is the owner flying this set, and a pinned ship cannot be flown.
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.add_systems(Update, pin_player);
        }
    }

    app.run()
}

/// Force the window to 1920x1080 (the 16:9 the web figures use) at startup.
#[cfg(feature = "debug")]
fn force_resolution(mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>) {
    if let Ok(mut window) = windows.single_mut() {
        window.resolution.set(1920.0, 1080.0);
        window.resizable = false;
    }
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(rock_hollow(&game_assets, &sections)));
}

/// The set: a rock shell around a clearing, the player, a parked raider, and
/// two AI flights fighting across it.
fn rock_hollow(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let player = ship(
        "hollow_player",
        "Player Ship",
        Vec3::ZERO,
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::new(),
            speed_cap: None,
            // The player holds fire through several beats; running dry
            // mid-capture would leave a reload where the tracers should be.
            infinite_ammo: true,
            lock_refire_secs: None,
        }),
        None,
        kit::kenney_hull(sections, "racer"),
    );

    // The lock subject: PARKED, because an AI hostile flies to a 250-unit
    // standoff and no close framing survives that (see the module docs).
    let raider = ship(
        RAIDER_ID,
        "Raider",
        RAIDER_POSITION,
        // Nose toward the player, turned off square: a hostile bearing down
        // reads better than a hull presenting its flank, and it puts the
        // section the juice beat blows on the camera's side of the ship.
        Quat::from_rotation_y(std::f32::consts::PI - 0.4),
        SpaceshipController::None,
        Some(Allegiance::Enemy),
        kit::kenney_hull(sections, "racer"),
    );

    // The live background: two friendlies on the flanks, two hostiles across
    // the hollow. Leashed to their posts so the ring they fly stays in the set.
    let wingman_a = ship(
        "hollow_wing_a",
        "Wingman",
        Vec3::new(-46.0, 8.0, -30.0),
        Quat::from_rotation_y(0.2),
        fighter(),
        Some(Allegiance::Player),
        kit::kenney_hull(sections, "racer"),
    );
    let wingman_b = ship(
        "hollow_wing_b",
        "Wingman",
        Vec3::new(44.0, -10.0, -40.0),
        Quat::from_rotation_y(-0.2),
        fighter(),
        Some(Allegiance::Player),
        kit::kenney_hull(sections, "racer"),
    );
    let hostile_a = ship(
        "hollow_hostile_a",
        "Raider",
        Vec3::new(-120.0, 26.0, -170.0),
        Quat::from_rotation_y(3.0),
        fighter(),
        None,
        kit::kenney_hull(sections, "racer"),
    );
    let hostile_b = ship(
        "hollow_hostile_b",
        "Raider",
        Vec3::new(140.0, -30.0, -200.0),
        Quat::from_rotation_y(3.3),
        fighter(),
        None,
        kit::kenney_hull(sections, "cargob"),
    );

    // The shell: denser and bigger-bodied than the drift set's belt, and it
    // starts outside the raider so the lock framings keep a clear sightline.
    let shell = kit::NearField {
        id_prefix: "hollow_rock_",
        count: 44,
        seed: 40507,
        distance: (28.0, 120.0),
        radius: (1.0, 2.8),
        y_spread: 42.0,
    };

    ScenarioConfig {
        id: "rock_hollow".to_string(),
        name: "Rock Hollow".to_string(),
        description: "A firefight in a clearing inside a rock field.".to_string(),
        cubemap: game_assets.cubemap.clone().into(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: vec![
                shell.action(game_assets),
                player,
                raider,
                wingman_a,
                wingman_b,
                hostile_a,
                hostile_b,
                beacon(),
            ],
        }],
        ..Default::default()
    }
}

/// A fighting AI ship's routine: hold its post, engage after the grace, and
/// come back when the fight drags it too far out.
fn fighter() -> SpaceshipController {
    SpaceshipController::AI(AIControllerConfig {
        leash: Some(AI_LEASH),
        engage_delay: Some(ENGAGE_DELAY),
        ..default()
    })
}

/// The nav beacon the weapons-lowered radar sweep latches, so the tutorial's
/// radar-lock shot is a NAV lock and not a lock on the raider.
fn beacon() -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "hollow_beacon".to_string(),
            name: "Waypoint".to_string(),
            position: BEACON_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: "WAYPOINT".to_string(),
            radius: 0.8,
            color: Color::srgb(0.4, 0.75, 1.0),
            area_radius: None,
            lock_signature: None,
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
            sections,
        }),
    })
}

/// Put the HUD on (the contextual rules decide what is actually in shot).
#[cfg(feature = "debug")]
fn hud_instrument(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::On;
    }
}

/// Clean the screen, for the beats whose camera has left the player's ship.
#[cfg(feature = "debug")]
fn hud_cinematic(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::Cinematic;
    }
}

/// Request one shot of the primary window. Captures only when `NOVA_REEL` is
/// set, so the plain autopilot smoke run drives the same path without writing
/// files.
#[cfg(feature = "debug")]
fn shoot(world: &mut World, capturing: bool, path: &str) {
    if capturing {
        capture_window(world, path);
        info!("combat capture: {path}");
    }
}

/// Pin the camera for a framing the follow camera does not give.
#[cfg(feature = "debug")]
fn pose(world: &mut World, position: Vec3, look_at: Vec3) {
    reel_pose_camera(world, position, look_at);
}

/// Hand the camera back to the game.
#[cfg(feature = "debug")]
fn unpose(world: &mut World) {
    let camera = {
        let mut query = world.query_filtered::<Entity, With<ScenarioCameraMarker>>();
        query.iter(world).next()
    };
    if let Some(camera) = camera {
        world.entity_mut(camera).remove::<ScriptedCameraPose>();
    }
}

/// Hold the player exactly where the set was measured from, for scripted runs.
///
/// Not cosmetic, and not the STOP autopilot: STOP flips retrograde and BURNS,
/// so engaging it on a ship with a metre per second of spawn drift walks the
/// ship fifty units downrange before it settles. The set's geometry is measured
/// from a player at the origin, and the radar picks the body nearest the AIM
/// RAY (`crates/nova_gameplay/src/input/targeting/radar.rs`), so a player a few
/// tens of units off station swings the parked raider off the ray and latches a
/// hostile two kilometres out instead. A photo subject sits still.
#[cfg(feature = "debug")]
fn pin_player(
    mut player: Query<
        (
            &mut Transform,
            &mut avian3d::prelude::LinearVelocity,
            &mut avian3d::prelude::AngularVelocity,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    for (mut transform, mut linear, mut angular) in &mut player {
        transform.translation = Vec3::ZERO;
        transform.rotation = Quat::IDENTITY;
        linear.0 = Vec3::ZERO;
        angular.0 = Vec3::ZERO;
    }
}

/// Hold CTRL: the radar gesture. Which slot it latches depends on the stance.
#[cfg(feature = "debug")]
fn hold_radar(world: &mut World) {
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
}

/// Release the radar gesture.
#[cfg(feature = "debug")]
fn release_radar(world: &mut World) {
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::ControlLeft);
}

/// Raise the weapons (RMB), switching the radar from the nav slot to combat.
#[cfg(feature = "debug")]
fn raise_stance(world: &mut World) {
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Right);
}

/// Hold the trigger (LMB) so the player's turret is firing in the combat shots.
#[cfg(feature = "debug")]
fn open_fire(world: &mut World) {
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
}

/// Despawn a scenario object by its scenario id.
#[cfg(feature = "debug")]
fn despawn_by_id(world: &mut World, id: &str) {
    let found = {
        let mut query = world.query::<(Entity, &EntityId)>();
        query
            .iter(world)
            .find(|(_, live)| live.0 == id)
            .map(|(entity, _)| entity)
    };
    if let Some(entity) = found {
        world.entity_mut(entity).despawn();
    }
}

/// Blow one forward hull section off the parked raider through the production
/// damage path - the same `HealthApplyDamage` a bullet delivers, so the shot is
/// of the real destruction, not of a prop.
#[cfg(feature = "debug")]
fn blow_raider_section(world: &mut World) {
    let Some(node) = raider_section_health(world) else {
        warn!("combat: no health node under section '{RAIDER_BLOWN_SECTION}' to blow");
        return;
    };
    world.trigger(HealthApplyDamage {
        entity: node,
        source: None,
        amount: 1.0e6,
    });
    info!("combat: blew '{RAIDER_BLOWN_SECTION}' off the raider");
}

/// The raider's blown section entity. Picked BY SHIP: the two racers in the set
/// share section ids, as every shipped multi-ship scenario does.
#[cfg(feature = "debug")]
fn raider_section(world: &mut World) -> Option<Entity> {
    let raider = {
        let mut query = world.query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>();
        query
            .iter(world)
            .find(|(_, id)| id.0 == RAIDER_ID)
            .map(|(entity, _)| entity)?
    };
    let mut query = world.query_filtered::<(Entity, &EntityId), With<SectionMarker>>();
    let candidates: Vec<Entity> = query
        .iter(world)
        .filter(|(_, id)| id.0 == RAIDER_BLOWN_SECTION)
        .map(|(entity, _)| entity)
        .collect();
    candidates
        .into_iter()
        .find(|&entity| under(world, entity, raider))
}

/// The `Health` node of the raider's blown section: the health lives on the
/// section entity or on one of its children.
#[cfg(feature = "debug")]
fn raider_section_health(world: &mut World) -> Option<Entity> {
    let section = raider_section(world)?;
    if world.get::<Health>(section).is_some() {
        return Some(section);
    }
    let children: Vec<Entity> = world
        .get::<Children>(section)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    children
        .into_iter()
        .find(|&child| world.get::<Health>(child).is_some())
}

/// Whether `entity` sits under `root` in the hierarchy.
#[cfg(feature = "debug")]
fn under(world: &World, entity: Entity, root: Entity) -> bool {
    let mut current = entity;
    for _ in 0..8 {
        match world.get::<ChildOf>(current) {
            Some(parent) if parent.parent() == root => return true,
            Some(parent) => current = parent.parent(),
            None => return false,
        }
    }
    false
}
