//! 15_screenshot_combat: capture the combat/HUD web screenshots - a live combat
//! lock with the red reticle and the target viewfinder inset - by driving the
//! real radar-lock gesture on a small range (player ship + a target dead ahead),
//! the same setup `11_hud_range` verifies.
//!
//! It performs the actual player gesture through the live input pipeline: raise
//! weapons (RMB) + hold radar (CTRL); at the hold threshold the radar latches the
//! combat slot on the target ahead, the lock goes live, and the reticle + inset
//! come up. It captures with the HUD at its instrument tier
//! ([`HudVisibility::Minimal`]) so the reticle/inset are in shot but the
//! fps/version chrome is not.
//!
//! Two run modes, both under the autopilot (`BCS_AUTOPILOT`):
//! - `BCS_AUTOPILOT=1` alone: the smoke path - reach Playing, drive the lock,
//!   exit clean, capturing nothing.
//! - `BCS_AUTOPILOT=1 BCS_REEL=1`: also capture the shots (staged under
//!   `NOVA_SHOT_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/reel BCS_AUTOPILOT=1 BCS_REEL=1 \
//!   cargo run --example 15_screenshot_combat --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example 15_screenshot_combat --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

use bevy::{platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "15_screenshot_combat")]
#[command(version = "1.0.0")]
#[command(about = "Capture the combat-lock web screenshots", long_about = None)]
struct Cli;

/// Distance the target sits dead ahead: inside lock range and the aim cone,
/// close enough that the reticle and target read at the chase-camera framing.
const TARGET_Z: f32 = -50.0;

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.init_resource::<CombatScript>();
        // A generous window: reach Playing, perform the gesture, let the lock +
        // inset + dwell settle, then capture.
        app.add_plugins(
            AutopilotPlugin::<GameStates>::new()
                .hold(GameStates::Loading, 14.0)
                .input(combat_capture_script),
        );
        app.add_systems(Startup, (force_resolution, hide_dev_overlays));
    }

    app.run();
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
/// parked dead ahead - the combat-lock subject. Mirrors `11_hud_range`.
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
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::new(),
            speed_cap: None,
            infinite_ammo: true,
        }),
        sections: player_sections,
    };
    let target = SpaceshipConfig {
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

/// Set the HUD to its instrument tier (reticle/inset in shot, fps chrome out).
#[cfg(feature = "debug")]
fn hud_instrument(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::Minimal;
    }
}

/// Progress of the scripted capture run. Each `shot_*` is a one-shot guard so a
/// capture fires on exactly one frame (Bevy services one primary-window capture
/// per frame, so separate frames are load-bearing).
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct CombatScript {
    playing_since: Option<f32>,
    nav_radar: bool,
    shot_radar: bool,
    beacon_gone: bool,
    raised: bool,
    radar_held: bool,
    shot_combat: bool,
    shot_combat_lock: bool,
    shot_viewfinder: bool,
    shot_hud: bool,
    goto: bool,
    shot_autopilot: bool,
    done: bool,
}

/// Drive the full combat/flight HUD reel on one range and capture each beat: a
/// nav-slot radar sweep (weapons lowered), the live combat lock + viewfinder
/// inset (weapons raised), then a GOTO maneuver. Runs every autopilot frame;
/// input presses are held until the beat that releases them. Captures only when
/// `BCS_REEL` is set, so the plain autopilot smoke run drives the same path
/// without writing files.
#[cfg(feature = "debug")]
fn combat_capture_script(world: &mut World, elapsed: f32) {
    let capturing = std::env::var_os("BCS_REEL").is_some();

    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }
    if world.resource::<CombatScript>().done {
        return;
    }

    // Timeline relative to entering Playing, so a slow load shifts it.
    let playing_since = {
        let mut script = world.resource_mut::<CombatScript>();
        *script.playing_since.get_or_insert(elapsed)
    };
    let t = elapsed - playing_since;

    // --- Nav sweep: weapons LOWERED, hold CTRL -> the nav-slot radar opens and
    // the white NAV crosshair sweeps onto the target ahead. ---
    if t > 0.3 && !world.resource::<CombatScript>().nav_radar {
        world.resource_mut::<CombatScript>().nav_radar = true;
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ControlLeft);
    }
    if t > 1.5 && !world.resource::<CombatScript>().shot_radar {
        hud_instrument(world);
        if capturing {
            capture_window(world, "tutorial-radar-lock.png");
            info!("combat capture: tutorial-radar-lock.png");
        }
        // Release the nav sweep before switching to the raised combat stance.
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ControlLeft);
        world.resource_mut::<CombatScript>().shot_radar = true;
        return;
    }

    // The beacon has served the radar-lock shot; despawn it now (a LATER frame
    // than the capture - despawning in the capture frame removes it before the
    // screenshot's end-of-frame render) so its glow does not sit on the reticle
    // in the combat shots.
    if t > 1.7 && !world.resource::<CombatScript>().beacon_gone {
        world.resource_mut::<CombatScript>().beacon_gone = true;
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
        return;
    }

    // --- Combat lock: raise weapons (RMB), then hold radar (CTRL) a beat later
    // (the natural order 11_hud_range uses). At the hold threshold the radar
    // latches the combat slot on the target and the reticle + inset come up. ---
    if t > 1.9 && !world.resource::<CombatScript>().raised {
        world.resource_mut::<CombatScript>().raised = true;
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
    }
    if t > 2.2 && !world.resource::<CombatScript>().radar_held {
        world.resource_mut::<CombatScript>().radar_held = true;
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ControlLeft);
    }

    // Let the lock latch, the inset spin up, and the focus dwell fill; capture
    // the combat shots on SEPARATE frames.
    if t > 4.0 && !world.resource::<CombatScript>().shot_combat {
        hud_instrument(world);
        if capturing {
            capture_window(world, "feature-combat.png");
            info!("combat capture: feature-combat.png");
        }
        world.resource_mut::<CombatScript>().shot_combat = true;
        return;
    }
    if t > 4.2 && !world.resource::<CombatScript>().shot_combat_lock {
        // Full HUD tier so the target viewfinder inset is in shot (the tutorial
        // combat-lock is about the viewfinder + reticle together).
        if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
            *hud = HudVisibility::All;
        }
        if capturing {
            capture_window(world, "tutorial-combat-lock.png");
            info!("combat capture: tutorial-combat-lock.png");
        }
        world.resource_mut::<CombatScript>().shot_combat_lock = true;
        return;
    }
    if t > 4.4 && !world.resource::<CombatScript>().shot_viewfinder {
        // Keep the full HUD (viewfinder) up for the viewfinder devlog shot.
        if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
            *hud = HudVisibility::All;
        }
        if capturing {
            capture_window(world, "devlog5-target-viewfinder.png");
            info!("combat capture: devlog5-target-viewfinder.png");
        }
        world.resource_mut::<CombatScript>().shot_viewfinder = true;
        return;
    }

    // --- Full HUD: the same locked frame at full chrome (every readout + the
    // fps/version bar), the "HUD at full chrome" showcase. ---
    if t > 4.6 && !world.resource::<CombatScript>().shot_hud {
        if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
            *hud = HudVisibility::All;
        }
        if capturing {
            capture_window(world, "feature-hud.png");
            info!("combat capture: feature-hud.png");
        }
        world.resource_mut::<CombatScript>().shot_hud = true;
        return;
    }

    // --- GOTO maneuver: release the stance, stick the travel lock on the
    // target, and engage the GOTO autopilot. The hull swings onto the new
    // heading and the thruster plume lights. ---
    if t > 4.9 && !world.resource::<CombatScript>().goto {
        world.resource_mut::<CombatScript>().goto = true;
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
    if t > 6.8 && !world.resource::<CombatScript>().shot_autopilot {
        hud_instrument(world);
        if capturing {
            capture_window(world, "feature-autopilot.png");
            info!("combat capture: feature-autopilot.png");
        }
        world.resource_mut::<CombatScript>().shot_autopilot = true;
        world.resource_mut::<CombatScript>().done = true;
    }
}
