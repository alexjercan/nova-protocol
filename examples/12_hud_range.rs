//! 12_hud_range: verify the screen-projected HUD indicators, live.
//!
//! Tasks 20260708-165700/165701/165702: the torpedo-lock reticle, the
//! locked-target readout, the autopilot destination marker and the turret
//! lead pips are consumers of the generic screen-indicator widget
//! (`hud/screen_indicator.rs`). This range checks the whole wiring in the
//! real app: the camera glue observer tags the chase camera, the aim-assist
//! lock drives the reticle anchor and fills the readout (distance, closing
//! speed, health bar), the engaged GOTO drives the destination marker, the
//! turret's computed intercept point drives its pip, and every indicator
//! hides again when its anchor dies.
//!
//! One player ship at the origin facing -Z (with one turret, so exactly one
//! lead pip exists), one uncontrolled target ship parked dead ahead at
//! z = -150 - inside the aim-assist cone, so the lock acquires by itself.
//!
//! Controls: none needed; fly and look around freely in interactive runs.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example 12_hud_range --features debug
//! # scripted (relative to entering Playing): at +0.5s assert the focus
//! # meter is filling (lock held, dwell incomplete, no component markers
//! # yet); at +2s assert the aim-assist
//! # locked the target, the reticle is visible and centered on the target's
//! # projection, the readout shows the real distance and a full health bar,
//! # the turret auto-fire feed aims at the locked ship's live structure
//! # (not the camera-ray point), and the turret lead pip is visible on the
//! # projected TurretSectionAimPoint, plus the focus dwell completed: the
//! # meter is gone and one component marker overlays each of the target's
//! # three sections; at +2.5s engage a GOTO on the target; at +3s pin a
//! # component lock on the tail section and at +3.5s assert its marker is
//! # highlighted; at +3.5s
//! # assert the destination marker is visible and centered on it and the
//! # readout's closing speed went positive under the approach burn; at +4s
//! # despawn the target and disable the turret section; at +4.5s assert all
//! # three indicators hid again and the component markers are gone. Exits non-zero on any failed stage or if
//! # the script never finishes (e.g. loading ate the window).
//! ```

#[cfg(feature = "debug")]
use avian3d::prelude::ComputedCenterOfMass;
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
    asserted_meter: bool,
    asserted_lock: bool,
    engaged_goto: bool,
    pinned_component: bool,
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

    let mut player_sections = sections_line("player");
    player_sections.push(SpaceshipSectionConfig {
        id: "player_turret".to_string(),
        position: Vec3::new(0.0, 0.0, -1.0),
        // Matches the turret placement in 08_turret_range so the base sits
        // upright.
        rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        config: section("better_turret_section"),
    });
    let player = SpaceshipConfig {
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::new(),
            speed_cap: None,
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

/// The player ship's turret section. Mandatory: the range spawns exactly one.
#[cfg(feature = "debug")]
fn player_turret(world: &mut World) -> Entity {
    let player = player_root(world);
    world
        .query_filtered::<(Entity, &ChildOf), With<TurretSectionMarker>>()
        .iter(world)
        .find(|(_, ChildOf(parent))| *parent == player)
        .map(|(turret, _)| turret)
        .expect("hud range: the player ship has no turret section")
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

/// Text of the given readout line. Mandatory: the readout ships with both
/// lines, so a missing one is a broken HUD.
#[cfg(feature = "debug")]
fn readout_line(world: &mut World, which: TorpedoTargetReadoutLine) -> String {
    world
        .query::<(&Text, &TorpedoTargetReadoutLine)>()
        .iter(world)
        .find(|(_, line)| **line == which)
        .map(|(text, _)| text.0.clone())
        .unwrap_or_else(|| panic!("hud range: readout line {which:?} not found"))
}

/// Parse the trailing number out of a readout line like `DST   150m` or
/// `CLS +12.3 u/s`.
#[cfg(feature = "debug")]
fn readout_value(line: &str) -> f32 {
    let number: String = line
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+')
        .collect();
    number
        .parse()
        .unwrap_or_else(|_| panic!("hud range: no number in readout line '{line}'"))
}

/// Scripted headless run: assert the lock-driven reticle + readout, the
/// GOTO-driven destination marker, and that everything hides when the target
/// dies.
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
    let (
        asserted_meter,
        asserted_lock,
        engaged_goto,
        pinned_component,
        asserted_goto,
        killed_target,
        done,
    ) = (
        script.asserted_meter,
        script.asserted_lock,
        script.engaged_goto,
        script.pinned_component,
        script.asserted_goto,
        script.killed_target,
        script.done,
    );

    if t > 0.5 && !asserted_meter {
        world.resource_mut::<HudRangeScript>().asserted_meter = true;

        // The lock exists from the first Playing frames; half a second in,
        // the 1.5 s dwell is still filling: meter visible and partial, no
        // component markers yet.
        (**world.resource::<SpaceshipPlayerTargetLock>())
            .expect("hud range: no lock at the meter stage");
        let meter_visibility = *world
            .query_filtered::<&Visibility, With<TorpedoTargetFocusMeterMarker>>()
            .iter(world)
            .next()
            .expect("hud range: no focus meter node");
        assert_eq!(
            meter_visibility,
            Visibility::Inherited,
            "hud range: the focus meter is not filling while the dwell runs"
        );
        let fill = world
            .query_filtered::<&Node, With<TorpedoTargetFocusFillMarker>>()
            .iter(world)
            .next()
            .expect("hud range: no focus fill node");
        let Val::Percent(fill_percent) = fill.width else {
            panic!("hud range: focus fill width is not a percent");
        };
        assert!(
            (5.0..95.0).contains(&fill_percent),
            "hud range: focus fill {fill_percent:.0}% is not mid-dwell"
        );
        let markers = world
            .query_filtered::<(), With<ComponentLockSectionMarker>>()
            .iter(world)
            .count();
        assert_eq!(
            markers, 0,
            "hud range: component markers appeared before the dwell completed"
        );
        info!("hud range: focus meter OK (fill {fill_percent:.0}%)");
    }

    if t > 2.0 && !asserted_lock {
        world.resource_mut::<HudRangeScript>().asserted_lock = true;

        let lock = (**world.resource::<SpaceshipPlayerTargetLock>())
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

        // Readout: the distance line must match the actual separation, the
        // closing line must carry real velocity data (both ships have
        // LinearVelocity), and the untouched target's bar must be full.
        let player = player_root(world);
        let player_pos = world
            .entity(player)
            .get::<GlobalTransform>()
            .expect("hud range: player has a GlobalTransform")
            .translation();
        let actual_distance = player_pos.distance(target_pos);
        let distance_text = readout_line(world, TorpedoTargetReadoutLine::Distance);
        let shown_distance = readout_value(&distance_text);
        assert!(
            (shown_distance - actual_distance).abs() < 5.0,
            "hud range: readout '{distance_text}' does not match the actual \
             distance {actual_distance:.1}"
        );
        let closing_text = readout_line(world, TorpedoTargetReadoutLine::ClosingSpeed);
        assert!(
            !closing_text.contains("---"),
            "hud range: closing-speed line is the no-data placeholder \
             although both ships have LinearVelocity"
        );
        let fill = world
            .query_filtered::<&Node, With<TorpedoTargetHealthFillMarker>>()
            .iter(world)
            .next()
            .expect("hud range: no health bar fill node");
        assert_eq!(
            fill.width,
            Val::Percent(100.0),
            "hud range: the untouched target's health bar is not full"
        );
        info!("hud range: readout OK ('{distance_text}', '{closing_text}', bar full)");

        // The turret aims where the player aims (a point 100 m down the
        // camera ray), so its intercept point exists from the first Playing
        // frame and its pip must be visible on that point's projection.
        let turret = player_turret(world);
        let aim_point = (**world
            .entity(turret)
            .get::<TurretSectionAimPoint>()
            .expect("hud range: the turret has no aim point component"))
        .expect("hud range: the turret never computed an intercept point");
        // The three-tier feed must aim the turret at the LOCKED SHIP's live
        // structure, not the camera-ray point 100 m out (task 20260709-173700):
        // dead ahead both project to the screen center, so discriminate in
        // world space instead.
        let (target_transform, target_com) = world
            .entity(target)
            .get_components::<(&Transform, Option<&ComputedCenterOfMass>)>()
            .expect("hud range: target has a transform");
        let target_anchor = live_structure_anchor(target_transform, target_com);
        let feed_error = (aim_point - target_anchor).length();
        assert!(
            feed_error < 5.0,
            "hud range: turret aim point {aim_point:?} is {feed_error:.1} m from \
             the locked ship's anchor {target_anchor:?} - the lock feed is not \
             driving the turret (camera-ray point would be ~50 m short)"
        );

        let expected_pip = project_through_indicator_camera(world, aim_point);
        let (pip_center, _, pip_visibility) =
            indicator_state::<TurretLeadPipMarker>(world, "turret lead pip");
        assert_eq!(
            pip_visibility,
            Visibility::Visible,
            "hud range: the lead pip is not visible while the turret tracks"
        );
        let pip_drift = pip_center.distance(expected_pip);
        assert!(
            pip_drift < CENTER_TOLERANCE_PX,
            "hud range: lead pip center {pip_center:?} is {pip_drift:.1} px \
             from the projected aim point {expected_pip:?}"
        );
        info!("hud range: turret lead pip OK (drift {pip_drift:.1} px)");

        // Dwell complete by now (lock held since ~+0s, FOCUS_TIME 1.5 s):
        // the meter yields to one marker per attached target section.
        let meter_visibility = *world
            .query_filtered::<&Visibility, With<TorpedoTargetFocusMeterMarker>>()
            .iter(world)
            .next()
            .expect("hud range: no focus meter node");
        assert_eq!(
            meter_visibility,
            Visibility::Hidden,
            "hud range: the focus meter is still visible after the dwell"
        );
        let sections = world
            .query_filtered::<&ChildOf, With<SectionMarker>>()
            .iter(world)
            .filter(|ChildOf(parent)| *parent == target)
            .count();
        let markers = world
            .query_filtered::<(), With<ComponentLockSectionMarker>>()
            .iter(world)
            .count();
        assert_eq!(
            markers, sections,
            "hud range: expected one component marker per attached section"
        );
        assert_eq!(sections, 3, "hud range: the target ship has 3 sections");
        info!("hud range: component markers OK ({markers} of {sections} sections)");
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

    if t > 3.0 && !pinned_component {
        world.resource_mut::<HudRangeScript>().pinned_component = true;

        let target = target_root(world).expect("hud range: target ship vanished before the pin");
        // Deliberately pick the TAIL section (largest local z) - snap would
        // favor whatever sits nearest the crosshair ray, so a highlight on
        // the tail proves the pinned selection drives the HUD.
        let tail = world
            .query_filtered::<(Entity, &ChildOf, &Transform), With<SectionMarker>>()
            .iter(world)
            .filter(|(_, ChildOf(parent), _)| *parent == target)
            .max_by(|(_, _, a), (_, _, b)| a.translation.z.total_cmp(&b.translation.z))
            .map(|(entity, _, _)| entity)
            .expect("hud range: target has sections to pin");
        let until = elapsed + 30.0;
        let mut component = world.resource_mut::<SpaceshipPlayerComponentLock>();
        component.section = Some(tail);
        component.mode = ComponentLockMode::Pinned { until };
        info!("hud range: pinned component lock on the tail section");
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

        // One second into the approach burn the ship moves toward the parked
        // target, so the readout's closing speed must be positive.
        let closing_text = readout_line(world, TorpedoTargetReadoutLine::ClosingSpeed);
        let closing = readout_value(&closing_text);
        assert!(
            closing > 0.0,
            "hud range: closing speed '{closing_text}' is not positive while \
             burning toward the target"
        );
        info!("hud range: GOTO destination marker OK (drift {drift:.1} px, '{closing_text}')");

        // The pinned tail section's marker must carry the highlight style.
        let pinned = world
            .resource::<SpaceshipPlayerComponentLock>()
            .section
            .expect("hud range: the pinned component lock vanished");
        let (selected, others): (Vec<f32>, Vec<f32>) = world
            .query_filtered::<(&Node, &ComponentLockSectionTarget), With<ComponentLockSectionMarker>>()
            .iter(world)
            .map(|(node, section)| {
                let Val::Px(px) = node.width else {
                    panic!("hud range: marker width is not Val::Px");
                };
                (px, **section == pinned)
            })
            .fold((Vec::new(), Vec::new()), |(mut sel, mut rest), (px, is_sel)| {
                if is_sel {
                    sel.push(px);
                } else {
                    rest.push(px);
                }
                (sel, rest)
            });
        assert_eq!(
            selected.len(),
            1,
            "hud range: pinned section has one marker"
        );
        assert!(
            others.iter().all(|px| *px < selected[0]),
            "hud range: the pinned marker {selected:?} is not larger than its \
             siblings {others:?}"
        );
        info!(
            "hud range: component highlight OK (selected {:.0} px)",
            selected[0]
        );
    }

    if t > 4.0 && !killed_target {
        world.resource_mut::<HudRangeScript>().killed_target = true;
        let target = target_root(world).expect("hud range: target ship vanished before the kill");
        world.entity_mut(target).despawn();
        // The turret keeps aiming at the camera ray even with no enemy, so
        // its anchor only clears through the disabled path: mark the section
        // inactive like the health pipeline does.
        let turret = player_turret(world);
        world.entity_mut(turret).insert(SectionInactiveMarker);
        info!("hud range: target ship despawned, turret section disabled");
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
        let (_, _, pip_visibility) =
            indicator_state::<TurretLeadPipMarker>(world, "turret lead pip");
        assert_eq!(
            pip_visibility,
            Visibility::Hidden,
            "hud range: the lead pip is still visible after the turret was disabled"
        );
        let markers = world
            .query_filtered::<(), With<ComponentLockSectionMarker>>()
            .iter(world)
            .count();
        assert_eq!(
            markers, 0,
            "hud range: component markers survived their target's death"
        );
        info!("hud range: PASS - indicators track their anchors and hide when they die");
    }
}
