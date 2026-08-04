//! screenshot_combat: capture the combat/HUD web screenshots - a live combat
//! lock with the red reticle and the target viewfinder inset - by driving the
//! real radar-lock gesture on a small range (player ship + a target dead ahead),
//! the same setup `hud_range` verifies.
//!
//! It performs the actual player gesture through the live input pipeline: raise
//! weapons (RMB) + hold radar (CTRL); at the hold threshold the radar latches the
//! combat slot on the target ahead, the lock goes live, and the reticle + inset
//! come up. It captures with the HUD on ([`HudVisibility::On`]): the contextual
//! rules keep idle chrome out of the frame by themselves, and a live lock is
//! exactly when the reticle + inset are up.
//!
//! One beat of the reel is a verification shot rather than a web figure:
//! `hud-nav-chips.png` frames a plain nav beacon (cyan chip) and a marked
//! objective (gold chip) side by side, so the world-anchored chip family can be
//! eyeballed in one frame (task 20260730-122909).
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - reach Playing, drive the lock,
//!   exit clean, capturing nothing.
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

use bevy::{platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_combat")]
#[command(version = "1.0.0")]
#[command(about = "Capture the combat-lock web screenshots", long_about = None)]
struct Cli;

/// Distance the target sits dead ahead: inside lock range and the aim cone,
/// close enough that the reticle and target read at the chase-camera framing.
const TARGET_Z: f32 = -50.0;

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

    #[cfg(feature = "debug")]
    {
        let capturing = std::env::var_os(nova_protocol::nova_debug::harness::REEL_ENV).is_some();
        // One step per beat. Every capture gets its OWN step, so the
        // one-capture-per-frame rule (Bevy services one primary-window capture
        // per frame) is structural rather than a `shot_*` guard. Input presses
        // are held until the beat that releases them.
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                .step("load the combat range")
                .enter(GameStates::Loading)
                .until(player_ship_present())
                .deadline(30.0)
                .add()
                // Nav sweep: weapons LOWERED, hold CTRL -> the nav-slot radar
                // opens and the white NAV crosshair sweeps onto the target.
                .step("sweep the nav radar")
                .on_enter(hold_radar)
                .until(elapsed(1.2))
                .add()
                .step("capture the radar lock")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "tutorial-radar-lock.png");
                    // Release the nav sweep before the raised combat stance.
                    world
                        .resource_mut::<ButtonInput<KeyCode>>()
                        .release(KeyCode::ControlLeft);
                })
                .until(elapsed(0.2))
                .add()
                // A LATER step than the capture: despawning in the capture
                // frame removes the beacon before the screenshot's
                // end-of-frame render, and its glow must not sit on the
                // reticle in the combat shots either.
                .step("clear the nav beacon")
                .on_enter(despawn_nav_beacon)
                .until(elapsed(0.2))
                .add()
                // Raise weapons (RMB), then hold radar (CTRL) a beat later -
                // the natural order hud_range uses. At the hold threshold the
                // radar latches the combat slot and the reticle + inset come up.
                .step("raise the weapons")
                .on_enter(raise_stance)
                .until(elapsed(0.3))
                .add()
                .step("latch the combat lock")
                .on_enter(hold_radar)
                .until(elapsed(1.8))
                .add()
                .step("capture the combat frame")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "feature-combat.png");
                })
                .until(elapsed(0.2))
                .add()
                // HUD on so the target viewfinder inset is in shot (the
                // tutorial combat-lock is about the viewfinder + reticle
                // together).
                .step("capture the combat lock")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "tutorial-combat-lock.png");
                })
                .until(elapsed(0.2))
                .add()
                .step("capture the target viewfinder")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "devlog5-target-viewfinder.png");
                })
                .until(elapsed(0.2))
                .add()
                // The same locked frame with every situational readout + the
                // fps/version bar: the "HUD in combat" showcase.
                .step("capture the HUD in combat")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "feature-hud.png");
                })
                .until(elapsed(0.05))
                .add()
                // The world-anchored nav chips, both members of the family in
                // ONE frame: a plain beacon (cyan) and a marked objective
                // (gold). One entity cannot show both - a marked beacon yields
                // its chip to the gold marker - so this beat spawns two
                // subjects side by side. It is the eyeball proof that each
                // pill's fill and border back its WHOLE label (task
                // 20260730-122909); the chips are torn down right after the
                // shot so no later capture inherits them.
                .step("spawn the nav chips")
                .on_enter(spawn_nav_chips)
                .until(elapsed(0.2))
                .add()
                .step("capture the nav chips")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "hud-nav-chips.png");
                })
                .until(elapsed(0.1))
                .add()
                .step("clear the nav chips")
                .on_enter(despawn_nav_chips)
                .until(elapsed(0.15))
                .add()
                // GOTO maneuver: release the stance, stick the travel lock on
                // the target, and engage the GOTO autopilot. The hull swings
                // onto the new heading and the thruster plume lights.
                .step("engage the GOTO maneuver")
                .on_enter(engage_goto)
                .until(elapsed(1.9))
                .add()
                .step("capture the autopilot maneuver")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "feature-autopilot.png");
                })
                .until(frames(capture_settle_frames(capturing)))
                .add(),
        );
        app.add_systems(Startup, (force_resolution, hide_dev_overlays));
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
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(combat_range(&game_assets, &sections)));
}

/// A player ship at the origin with a turret, and an uncontrolled target ship
/// parked dead ahead - the combat-lock subject. Mirrors `hud_range`.
fn combat_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
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

    let mut player_sections = sections_line("player");
    player_sections.push(SpaceshipSectionConfig {
        id: "player_turret".to_string(),
        position: Vec3::new(0.0, 0.0, -1.0),
        rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        source: SectionSource::Inline(section("better_turret_section")),
        modifications: vec![],
    });
    let player = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::new(),
            speed_cap: None,
            infinite_ammo: true,
            lock_refire_secs: None,
        }),
        sections: player_sections,
    };
    let target = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::None,
        sections: sections_line("target"),
    };

    let spawn = |id: &str, name: &str, position: Vec3, ship: SpaceshipConfig| {
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

    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions: vec![
            spawn("player_ship", "Player Ship", Vec3::ZERO, player),
            spawn(
                "target_ship",
                "Hostile",
                Vec3::new(0.0, 0.0, TARGET_Z),
                target,
            ),
            // A nav waypoint dead ahead (in front of the hostile) so the
            // weapons-lowered radar sweep latches the NAV slot onto a beacon -
            // the tutorial's radar-lock, not a lock on the ship.
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "nav_beacon".to_string(),
                    name: "Waypoint".to_string(),
                    position: Vec3::new(0.0, 0.0, TARGET_Z + 14.0),
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Beacon(BeaconConfig {
                    label: "WAYPOINT".to_string(),
                    radius: 2.0,
                    color: Color::srgb(0.4, 0.75, 1.0),
                    area_radius: None,
                    lock_signature: None,
                }),
            }),
        ],
    }];

    ScenarioConfig {
        id: "combat_range".to_string(),
        name: "Combat Range".to_string(),
        description: "A range for the combat-lock screenshots.".to_string(),
        cubemap: game_assets.cubemap.clone().into(),
        events,
        ..Default::default()
    }
}

/// The player ship root, once it exists.
#[cfg(feature = "debug")]
fn player_root(world: &mut World) -> Option<Entity> {
    world
        .query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()
        .iter(world)
        .next()
}

/// The target ship root (the only non-player ship on the range), once it exists.
#[cfg(feature = "debug")]
fn target_root(world: &mut World) -> Option<Entity> {
    world
        .query_filtered::<Entity, (With<SpaceshipRootMarker>, Without<PlayerSpaceshipMarker>)>()
        .iter(world)
        .next()
}

/// Put the HUD on (the contextual rules decide what is actually in shot).
#[cfg(feature = "debug")]
fn hud_instrument(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::On;
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

/// Hold CTRL: the radar gesture. Which slot it latches depends on the stance.
#[cfg(feature = "debug")]
fn hold_radar(world: &mut World) {
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
}

/// Raise the weapons (RMB), switching the radar from the nav slot to combat.
#[cfg(feature = "debug")]
fn raise_stance(world: &mut World) {
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Right);
}

/// Despawn the scenario's nav beacon once it has served the radar-lock shot.
#[cfg(feature = "debug")]
fn despawn_nav_beacon(world: &mut World) {
    let beacon = {
        let mut query = world.query::<(Entity, &EntityId)>();
        query
            .iter(world)
            .find(|(_, id)| id.0 == "nav_beacon")
            .map(|(entity, _)| entity)
    };
    if let Some(beacon) = beacon {
        world.entity_mut(beacon).despawn();
    }
}

/// The two nav-chip subjects, named so the teardown step can find them again.
#[cfg(feature = "debug")]
const CHIP_NAMES: [&str; 2] = ["ChipShotBeacon", "ChipShotObjective"];

/// Spawn a plain beacon (cyan chip) and a marked objective (gold chip) side by
/// side, so one frame carries both members of the family.
#[cfg(feature = "debug")]
fn spawn_nav_chips(world: &mut World) {
    world.spawn((
        Name::new(CHIP_NAMES[0]),
        // The scenario's OWN beacon bundle, so the chip is driven by exactly
        // the components a scenario-spawned waypoint has (a bare
        // `BeaconMarker` makes the render observer log an error and skip the
        // orb).
        beacon_scenario_object(BeaconConfig {
            label: "WAYPOINT".to_string(),
            // Small: the chip floats 28 px above its anchor, and a range-2 orb
            // at this distance would sit behind the pill and wash the shot out.
            radius: 0.5,
            color: Color::srgb(0.4, 0.75, 1.0),
            area_radius: None,
            lock_signature: None,
        }),
        Transform::from_xyz(-11.0, 5.0, -38.0),
        // The scenario's base object bundle supplies this; the orb child needs
        // an inheritable parent visibility (B0004).
        Visibility::default(),
    ));
    world.spawn((
        Name::new(CHIP_NAMES[1]),
        ObjectiveMarkerTarget::new("BEACON 1"),
        Transform::from_xyz(11.0, 5.0, -38.0),
    ));
}

/// Tear the chip subjects down so no later capture inherits them.
#[cfg(feature = "debug")]
fn despawn_nav_chips(world: &mut World) {
    let chips: Vec<Entity> = {
        let mut query = world.query::<(Entity, &Name)>();
        query
            .iter(world)
            .filter(|(_, name)| CHIP_NAMES.contains(&name.as_str()))
            .map(|(entity, _)| entity)
            .collect()
    };
    for chip in chips {
        world.entity_mut(chip).despawn();
    }
}

/// Release the combat stance, stick the travel lock on the target, and engage
/// the GOTO autopilot.
#[cfg(feature = "debug")]
fn engage_goto(world: &mut World) {
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::ControlLeft);
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Right);
    if let (Some(player), Some(target)) = (player_root(world), target_root(world)) {
        if let Some(mut travel) = world.entity_mut(player).get_mut::<TravelLock>() {
            travel.0 = Some(target);
        }
        world
            .entity_mut(player)
            .insert(Autopilot::engage(AutopilotAction::Goto { target }));
    }
}
