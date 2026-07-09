//! 12_hud_range: verify the screen-projected HUD indicators, live.
//!
//! Task 20260708-165700: the torpedo-lock reticle and the autopilot
//! destination marker are now consumers of the generic screen-indicator
//! widget (`hud/screen_indicator.rs`). This range checks the whole wiring in
//! the real app: the camera glue observer tags the chase camera, the
//! aim-assist lock drives the reticle anchor, the engaged GOTO drives the
//! destination marker, and both indicators hide again when their anchor dies.
//!
//! One player ship at the origin facing -Z, one uncontrolled target ship
//! parked dead ahead at z = -150 - inside the aim-assist cone, so the lock
//! acquires by itself.
//!
//! Controls: none needed; fly and look around freely in interactive runs.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example 12_hud_range --features debug
//! # scripted (relative to entering Playing): at +2s assert the aim-assist
//! # locked the target and the reticle is visible and centered on the
//! # target's projection; at +2.5s engage a GOTO on the target; at +3.5s
//! # assert the destination marker is visible and centered on it; at +4s
//! # despawn the target; at +4.5s assert both indicators hid again. Exits
//! # non-zero on any failed stage or if the script never finishes (e.g.
//! # loading ate the window).
//! ```

use bevy::{platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "12_hud_range")]
#[command(version = "1.0.0")]
#[command(about = "A test range for the screen-projected HUD indicators", long_about = None)]
struct Cli;

/// How far (px) an indicator's center may sit from the fresh projection of
/// its anchor. The HUD positions nodes in Update from last frame's propagated
/// transforms, so one frame of camera/ship motion is expected slack; measured
/// drift in this range is 0.0-0.1 px, so 10 px is still generous.
#[cfg(feature = "debug")]
const CENTER_TOLERANCE_PX: f32 = 10.0;

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Not the stock nova_autopilot(): the scripted timeline needs ~4.5s
        // of Playing, so hold a longer total window than the 6s preset.
        app.add_plugins(
            AutopilotPlugin::<GameStates>::new()
                .hold(GameStates::Loading, 8.0)
                .input(autopilot_script),
        );
        app.add_plugins(nova_screenshot());
    }

    app.run();
}

fn custom_plugin(app: &mut App) {
    #[cfg(feature = "debug")]
    app.init_resource::<HudRangeScript>();
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

/// Progress of the scripted (autopilot) run.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct HudRangeScript {
    playing_since: Option<f32>,
    asserted_lock: bool,
    engaged_goto: bool,
    asserted_goto: bool,
    killed_target: bool,
    done: bool,
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(hud_range(&game_assets, &sections)));
}

/// Build the range scenario: a player ship at the origin and an uncontrolled
/// target ship parked dead ahead.
fn hud_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
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
        config: section(kind),
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
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::new(),
        }),
        sections: sections_line("player"),
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
            spawn("player_ship", "HUD Test Ship", Vec3::ZERO, player),
            // Dead ahead (the ship and camera face -Z at spawn), well inside
            // the 2000 m lock range and the 18 degree aim cone.
            spawn(
                "target_ship",
                "HUD Target Ship",
                Vec3::new(0.0, 0.0, -150.0),
                target,
            ),
        ],
    }];

    ScenarioConfig {
        id: "hud_range".to_string(),
        name: "HUD Range".to_string(),
        description: "A test range for the screen-projected HUD indicators.".to_string(),
        cubemap: game_assets.cubemap.clone(),
        events,
    }
}

/// The player ship root. Mandatory: stages that need it must fail loudly, not
/// skip, if it is gone.
#[cfg(feature = "debug")]
fn player_root(world: &mut World) -> Entity {
    world
        .query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()
        .iter(world)
        .next()
        .expect("hud range: no player ship root")
}

/// The target ship root (the only non-player ship in the range), if it still
/// exists. `None` is only expected after the kill stage.
#[cfg(feature = "debug")]
fn target_root(world: &mut World) -> Option<Entity> {
    world
        .query_filtered::<Entity, (With<SpaceshipRootMarker>, Without<PlayerSpaceshipMarker>)>()
        .iter(world)
        .next()
}

/// Fresh projection of `world_pos` through the screen-indicator camera. The
/// camera is mandatory: a missing `ScreenIndicatorCamera` means the hud/mod.rs
/// glue observer broke, and every lookup failure must fail the run.
#[cfg(feature = "debug")]
fn project_through_indicator_camera(world: &mut World, world_pos: Vec3) -> Vec2 {
    let (camera_transform, camera) = world
        .query_filtered::<(&GlobalTransform, &Camera), With<ScreenIndicatorCamera>>()
        .iter(world)
        .next()
        .expect("hud range: no ScreenIndicatorCamera - the camera glue observer failed");
    camera
        .world_to_viewport(camera_transform, world_pos)
        .expect("hud range: the anchor does not project onto the viewport")
}

/// Center and visibility of the single indicator node matching the marker
/// component `M`. Every lookup is mandatory (expect, not if-let) so refactors
/// fail loud.
#[cfg(feature = "debug")]
fn indicator_state<M: Component>(world: &mut World, what: &str) -> (Vec2, Vec2, Visibility) {
    let (node, visibility) = world
        .query_filtered::<(&Node, &Visibility), With<M>>()
        .iter(world)
        .next()
        .unwrap_or_else(|| panic!("hud range: no {what} indicator node found"));
    let px = |val: Val, name: &str| match val {
        Val::Px(px) => px,
        other => panic!("hud range: {what} {name} is {other:?}, expected Val::Px"),
    };
    let size = Vec2::new(px(node.width, "width"), px(node.height, "height"));
    let center = Vec2::new(
        px(node.left, "left") + size.x / 2.0,
        px(node.top, "top") + size.y / 2.0,
    );
    (center, size, *visibility)
}

/// Scripted headless run: assert the lock-driven reticle, the GOTO-driven
/// destination marker, and that both hide when the target dies.
#[cfg(feature = "debug")]
fn autopilot_script(world: &mut World, elapsed: f32) {
    // Backstop first, before the state gate: if the run is about to exit and
    // the script never finished (loading ate the window, or Playing was never
    // reached), fail instead of vacuously passing.
    if elapsed > 7.5 && !world.resource::<HudRangeScript>().done {
        let script = world.resource::<HudRangeScript>();
        panic!(
            "hud range: the scripted run never finished (lock={} goto={} drop={})",
            script.asserted_lock, script.asserted_goto, script.done
        );
    }
    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }

    // Timeline is relative to entering Playing, so a slow load shifts the
    // script instead of truncating it.
    let playing_since = {
        let mut script = world.resource_mut::<HudRangeScript>();
        *script.playing_since.get_or_insert(elapsed)
    };
    let t = elapsed - playing_since;
    let script = world.resource::<HudRangeScript>();
    let (asserted_lock, engaged_goto, asserted_goto, killed_target, done) = (
        script.asserted_lock,
        script.engaged_goto,
        script.asserted_goto,
        script.killed_target,
        script.done,
    );

    if t > 2.0 && !asserted_lock {
        world.resource_mut::<HudRangeScript>().asserted_lock = true;

        let lock = (**world.resource::<SpaceshipPlayerTorpedoTargetEntity>())
            .expect("hud range: the aim-assist never locked the target ship dead ahead");
        let target = target_root(world).expect("hud range: target ship vanished before the kill");
        assert_eq!(
            lock, target,
            "hud range: the lock is not the target ship root"
        );

        let target_pos = world
            .entity(target)
            .get::<GlobalTransform>()
            .expect("hud range: target has a GlobalTransform")
            .translation();
        let expected = project_through_indicator_camera(world, target_pos);
        let (center, size, visibility) =
            indicator_state::<TorpedoTargetReticleMarker>(world, "reticle");
        assert_eq!(
            visibility,
            Visibility::Visible,
            "hud range: the reticle is not visible while a lock exists"
        );
        let drift = center.distance(expected);
        assert!(
            drift < CENTER_TOLERANCE_PX,
            "hud range: reticle center {center:?} is {drift:.1} px from the \
             target projection {expected:?}"
        );
        assert!(
            size.x >= 32.0,
            "hud range: reticle width {} shrank below the 32 px minimum",
            size.x
        );
        info!("hud range: lock + reticle OK (drift {drift:.1} px, size {size:?})");
    }

    if t > 2.5 && !engaged_goto {
        world.resource_mut::<HudRangeScript>().engaged_goto = true;
        let target = target_root(world).expect("hud range: target ship vanished before the GOTO");
        let player = player_root(world);
        world
            .entity_mut(player)
            .insert(Autopilot::engage(AutopilotAction::Goto { target }));
        info!("hud range: GOTO engaged on the target ship");
    }

    if t > 3.5 && !asserted_goto {
        world.resource_mut::<HudRangeScript>().asserted_goto = true;

        let target = target_root(world).expect("hud range: target ship vanished before the kill");
        let target_pos = world
            .entity(target)
            .get::<GlobalTransform>()
            .expect("hud range: target has a GlobalTransform")
            .translation();
        let expected = project_through_indicator_camera(world, target_pos);
        let (center, _, visibility) =
            indicator_state::<AutopilotDestinationUIMarker>(world, "destination marker");
        assert_eq!(
            visibility,
            Visibility::Visible,
            "hud range: the destination marker is not visible during an engaged GOTO"
        );
        let drift = center.distance(expected);
        assert!(
            drift < CENTER_TOLERANCE_PX,
            "hud range: destination marker center {center:?} is {drift:.1} px \
             from the GOTO target projection {expected:?}"
        );
        info!("hud range: GOTO destination marker OK (drift {drift:.1} px)");
    }

    if t > 4.0 && !killed_target {
        world.resource_mut::<HudRangeScript>().killed_target = true;
        let target = target_root(world).expect("hud range: target ship vanished before the kill");
        world.entity_mut(target).despawn();
        info!("hud range: target ship despawned");
    }

    if t > 4.5 && !done {
        world.resource_mut::<HudRangeScript>().done = true;

        let (_, _, reticle_visibility) =
            indicator_state::<TorpedoTargetReticleMarker>(world, "reticle");
        assert_eq!(
            reticle_visibility,
            Visibility::Hidden,
            "hud range: the reticle is still visible after its target died"
        );
        let (_, _, marker_visibility) =
            indicator_state::<AutopilotDestinationUIMarker>(world, "destination marker");
        assert_eq!(
            marker_visibility,
            Visibility::Hidden,
            "hud range: the destination marker is still visible after the GOTO target died"
        );
        info!("hud range: PASS - indicators track their anchors and hide when they die");
    }
}
