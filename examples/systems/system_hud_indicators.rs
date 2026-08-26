//! system_hud_indicators: the screen-projected HUD indicators, verified live.
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
//! NOVA_AUTOPILOT=1 cargo run --example system_hud_indicators --features debug
//! # named beats, each one waiting on the world or on its own short dwell:
//! # wait for the player ship to spawn, then perform the REAL radar gesture
//! # (raise, hold CTRL, commit) - nothing locks passively in the
//! # deliberate-radar model; assert the focus meter is filling mid-dwell;
//! # inject a half-charged dwell and assert the acquisition ring rides it;
//! # assert the reticle is centered on the locked target's projection, the
//! # readout carries the real distance and a full health bar, the turret feed
//! # aims at the locked ship's live structure (not the camera-ray point), the
//! # lead pip sits on the projected TurretSectionAimPoint and the completed
//! # dwell replaced the meter with one component marker per section; engage a
//! # GOTO, pin a component lock on the tail section, and assert the
//! # destination marker, the positive closing speed and the highlighted
//! # pinned marker; despawn the target and disable the turret, then assert
//! # every indicator hid again. The last beat reports done, so the run ends on
//! # the assertion rather than idling out a runway. A beat that never resolves
//! # inside its deadline is an error exit naming the BEAT.
//! ```

#[cfg(feature = "debug")]
use avian3d::prelude::ComputedCenterOfMass;
use bevy::{platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_hud_indicators")]
#[command(version = "1.0.0")]
#[command(about = "A test range for the screen-projected HUD indicators. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// How far (px) an indicator's center may sit from the fresh projection of
/// its anchor. The HUD positions nodes in Update from last frame's propagated
/// transforms, so one frame of camera/ship motion is expected slack; measured
/// drift in this range is 0.0-0.1 px, so 10 px is still generous.
#[cfg(feature = "debug")]
const CENTER_TOLERANCE_PX: f32 = 10.0;

/// The RTT-inset frame this range shoots. Named once: the beat that waits for
/// the write and the call that makes it must agree on the string.
#[cfg(feature = "debug")]
const INSET_SHOT: &str = "inset_shot.png";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (task 20260719-210443; each plugin is inert without
        // its NOVA_PROBE_* env): run timeline + engine-bound invariants +
        // frame-time capture, so `probe run` can measure this example.
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        // Not the stock nova_autopilot(): the script is its own beat list. The
        // first beat waits for the ship to EXIST rather than for a guessed
        // load duration, and every later beat is a gesture or an assertion
        // separated by the short dwell the HUD needs to settle. The last beat
        // reports done, so the run ends on the assertion instead of idling out
        // a runway, and a beat that never resolves inside its deadline is an
        // error exit naming it. Keep the deadlines' sum well under
        // DEFAULT_DEADLINE_SECS (120s) so a named stall wins the race against
        // the generic collector deadline.
        app.add_plugins(nova_screenshot(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                .step("load the range")
                .enter(GameStates::Loading)
                .until(player_ship_present())
                .deadline(30.0)
                .add()
                // The deliberate-radar model, live-lock revision (spike
                // 20260713-110039): NOTHING locks passively, so the next three
                // beats perform the REAL gesture through the live input
                // pipeline. Raise first, radar a beat later - the natural human
                // order; at the hold threshold the radar latches the COMBAT
                // slot and the lock goes live under the sweep.
                .step("raise the stance")
                .on_enter(raise_stance)
                .until(elapsed(0.3))
                .add()
                .step("hold the radar")
                .on_enter(hold_radar)
                .until(elapsed(0.6))
                .add()
                .step("commit the live lock")
                .on_enter(commit_lock)
                .until(elapsed(0.6))
                .add()
                .step("assert the focus meter fills")
                .on_enter(assert_focus_meter)
                .until(elapsed(0.3))
                .add()
                .step("inject a half-charged dwell")
                .on_enter(inject_dwell)
                .until(elapsed(0.2))
                .add()
                .step("capture the inset frame")
                .on_enter(request_inset_shot)
                .until(shot_written(INSET_SHOT))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                .step("assert the acquisition ring")
                .on_enter(assert_dwell_ring)
                .until(elapsed(0.5))
                .add()
                .step("assert the lock indicators")
                .on_enter(assert_lock_indicators)
                .until(elapsed(0.3))
                .add()
                .step("engage the goto")
                .on_enter(engage_goto)
                .until(elapsed(0.4))
                .add()
                .step("pin a component lock")
                .on_enter(pin_component_lock)
                .until(elapsed(0.4))
                .add()
                .step("assert the goto indicators")
                .on_enter(assert_goto_indicators)
                .until(elapsed(0.4))
                .add()
                .step("kill the target")
                .on_enter(kill_target)
                .until(elapsed(0.4))
                .add()
                // The script's last beat: `nova_screenshot` appends the
                // capture beat behind it, and the driver reports done after
                // that - so the run ends on the assertion rather than idling
                // out a runway.
                .step("assert the indicators hid with their anchors")
                .on_enter(assert_indicators_hid)
                .add(),
        ));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    // Deterministic acquisition dwell for the scripted run (20260708-165703):
    // a short, distance-flat 0.2 s so the live gesture commits the lock at a
    // predictable time (~+1.0 s) that the downstream stages are timed against,
    // rather than the distance-scaled default. The interactive run keeps the
    // real feel.
    #[cfg(feature = "debug")]
    app.insert_resource(TargetingSettings {
        lock_dwell_base: 0.2,
        lock_dwell_range_factor: 0.0,
        lock_dwell_min: 0.0,
        ..default()
    });
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(hud_indicators_scenario(
        &game_assets,
        &sections,
    )));
}

/// Build the range scenario: a player ship at the origin and an uncontrolled
/// target ship parked dead ahead.
fn hud_indicators_scenario(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
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
        position: Vec3::new(0.0, 0.0, -0.75),
        // Matches the turret placement in system_turret_gunnery so the base sits
        // upright.
        rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        source: SectionSource::Inline(section("pdc_kinetic_turret_section")),
        modifications: vec![],
    });
    let player = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::new(),
            speed_cap: None,
            // Dev/tuning harness: fire freely.
            infinite_ammo: true,
        }),
        hull: ShipSource::Inline(ShipHull {
            sections: player_sections,
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
        once: false,
        filters: vec![],
        // The range lights itself: the engine spawns no light, so a scenario
        // that authors none renders black.
        actions: [
            vec![
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
            ThreePointRig::around("range", Vec3::ZERO, 5.0).actions(),
        ]
        .concat(),
    }];

    ScenarioConfig {
        description: "A test range for the screen-projected HUD indicators.".to_string(),
        events,
        ..ScenarioConfig::new(
            "hud_indicators".to_string(),
            "HUD Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
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

/// Parse a readout line like `DST 1.50 km` or `CLS +12.3 m/s` back into BASE
/// units (metres, metres per second) so it can be compared against the world.
///
/// The readouts route through `nova_ui::units` since task 20260728-175731, so
/// they auto-scale to km / km/s: a unit-naive digit scrape reads `1.50 km` as
/// `1.5` and every distance assertion below it becomes a lie. The suffix is
/// part of the value - read it.
#[cfg(feature = "debug")]
fn readout_value(line: &str) -> f32 {
    let number: String = line
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+')
        .collect();
    let value: f32 = number
        .parse()
        .unwrap_or_else(|_| panic!("hud range: no number in readout line '{line}'"));
    // `km` scales by 1000; plain `m`/`m/s` does not. Then back out of the
    // player-facing metres into WORLD units, which is what the scene measures.
    let metres = if line.contains("km") {
        value * 1000.0
    } else {
        value
    };
    metres / nova_ui::units::METERS_PER_UNIT
}

/// Raise the stance (RMB), the first half of the live radar gesture.
///
/// Raise first, radar a beat later - the natural human order. (The old
/// press-time latch made a SAME-frame RMB+CTRL press a recorded sharp edge;
/// the threshold latch retired it, Q1a.)
#[cfg(feature = "debug")]
fn raise_stance(world: &mut World) {
    press_mouse(MouseButton::Right)(world);
    info!("hud range: raised (live gesture)");
}

/// Hold CTRL: at the hold threshold the radar latches the COMBAT slot and the
/// lock goes LIVE under the sweep.
#[cfg(feature = "debug")]
fn hold_radar(world: &mut World) {
    press_key(KeyCode::ControlLeft)(world);
    info!("hud range: radar held (live gesture)");
}

/// Assert the live gesture produced a real lock, then release it: the sweep
/// latched the combat slot by itself, the lock exists BEFORE the release, and
/// the inset viewfinder is already up. Everything downstream (dwell, meter,
/// markers, reticle, turret feed) then flows exactly as a player drive would.
#[cfg(feature = "debug")]
fn commit_lock(world: &mut World) {
    let player = player_root(world);
    let target = target_root(world).expect("hud range: no target ship");
    // The LIVE radar must have latched the COMBAT slot at its threshold
    // (raised stance) and found the target ship dead ahead on its own.
    let radar = world
        .entity(player)
        .get::<RadarState>()
        .copied()
        .expect("hud range: the radar never opened on the CTRL hold");
    assert_eq!(
        radar.engaged,
        Some(RadarSlot::Combat),
        "hud range: the threshold must latch the combat slot (raised)"
    );
    assert_eq!(
        radar.candidate,
        Some(target),
        "hud range: the live radar did not find the ship dead ahead"
    );
    // The live-lock pin: the combat lock is ALREADY written while CTRL
    // is still held - releasing only sticks it (strand A1).
    assert_eq!(
        world.entity(player).get::<CombatLock>().unwrap().0,
        Some(target),
        "hud range: the lock must be live under the sweep, before release"
    );
    // Inset-on-lock (spike 20260713-110039 B1): the viewfinder is up
    // RIGHT NOW, mid-sweep, long before the 1.5 s focus dwell - one RTT
    // camera and a visible panel.
    let inset_cameras = world
        .query_filtered::<(), With<TargetInsetCameraMarker>>()
        .iter(world)
        .count();
    assert_eq!(
        inset_cameras, 1,
        "hud range: the viewfinder must be rendering the moment the lock exists"
    );
    let panel_visibility = *world
        .query_filtered::<&Visibility, With<TargetInsetHudMarker>>()
        .iter(world)
        .next()
        .expect("hud range: no target-inset panel node");
    assert_eq!(
        panel_visibility,
        Visibility::Visible,
        "hud range: the inset panel must show at lock time, not after the dwell"
    );
    // The frame carries the safety state (Q5a): hot right now (raised +
    // lock), so the armed ticks are on.
    let tick_visibility = *world
        .query_filtered::<&Visibility, With<TargetInsetArmedTickMarker>>()
        .iter(world)
        .next()
        .expect("hud range: no armed tick nodes on the inset frame");
    assert_eq!(
        tick_visibility,
        Visibility::Inherited,
        "hud range: the armed ticks must show while the weapons are hot"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the lock is live under the sweep",
        serde_json::json!({}),
    );
    info!("hud range: inset viewfinder up at lock time, frame armed");
    // Release: stick; lower the stance; set the travel designation for
    // the GOTO stage directly (its gesture is the same, already proven).
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::ControlLeft);
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Right);
    world.entity_mut(player).get_mut::<TravelLock>().unwrap().0 = Some(target);
    info!("hud range: radar released - the live combat lock sticks");
}

/// Mid-dwell: the 1.5 s focus dwell is still filling, so the meter is visible
/// and partial, no component markers exist yet, and the acquisition ring is
/// hidden (the gesture released, so nothing is charging).
#[cfg(feature = "debug")]
fn assert_focus_meter(world: &mut World) {
    let player = player_root(world);
    world
        .entity(player)
        .get::<CombatLock>()
        .unwrap()
        .0
        .expect("hud range: no combat lock at the meter stage");
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
    // The acquisition dwell ring (20260717-004302) only shows while a
    // gesture is CHARGING a lock; the lock committed and the gesture
    // released back at ~+1.1, so the ring must be hidden now, not lingering
    // over the settled lock.
    let ring_visibility = *world
        .query_filtered::<&Visibility, With<LockDwellRingMarker>>()
        .iter(world)
        .next()
        .expect("hud range: no lock-dwell ring node");
    assert_eq!(
        ring_visibility,
        Visibility::Hidden,
        "hud range: the dwell ring lingered after the lock settled"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the focus meter fills during the dwell",
        serde_json::json!({}),
    );
    info!("hud range: focus meter OK (fill {fill_percent:.0}%), dwell ring hidden");
}

/// Acquisition ring, injected check (20260717-004302). The gesture released
/// two beats ago, so there is no live `RadarState` to catch mid-charge; drive
/// one directly - a dwell half-charged on the still-on-screen target. The hold
/// is NOT fired, so `update_radar_search` leaves the injected dwell fields
/// untouched, and the ring driver + screen-indicator widget project and fill
/// the real `UiMaterial` ring.
#[cfg(feature = "debug")]
fn inject_dwell(world: &mut World) {
    let player = player_root(world);
    let target = target_root(world).expect("hud range: target gone before the ring check");
    // Pre-condition: nothing charging, so the ring is hidden right now.
    let ring_visibility = *world
        .query_filtered::<&Visibility, With<LockDwellRingMarker>>()
        .iter(world)
        .next()
        .expect("hud range: no lock-dwell ring node");
    assert_eq!(
        ring_visibility,
        Visibility::Hidden,
        "hud range: the ring is shown with no dwell charging"
    );
    world.entity_mut(player).insert(RadarState {
        engaged: Some(RadarSlot::Combat),
        candidate: Some(target),
        dwell_target: Some(target),
        dwell_secs: 0.5,
        dwell_needed: 1.0,
        ..default()
    });
    info!("hud range: injected a half-charged dwell for the ring check");
}

/// Capture a real loaded frame (scene up, lock focused, inset rendering) to a
/// PNG, so the RTT inset can be eyeballed headlessly. Inert unless
/// `NOVA_CAPTURE` is set, like every other shot in the fleet. Injected MID-RUN
/// from the settled script: this frame is the one worth having, and a shot
/// taken before async asset loading has a scene would be black (task
/// 20260710-104421 verify note).
#[cfg(feature = "debug")]
fn request_inset_shot(world: &mut World) {
    // `shoot`, not a bare `Screenshot` + `save_to_disk`: the shared idiom is
    // what resolves the path under `NOVA_CAPTURE_DIR` and acks the write. The
    // hand-rolled pair wrote `inset_shot.png` into the process CWD - the repo
    // root, for a `cargo run`.
    shoot(world, INSET_SHOT);
}

/// The ring driver + widget have run: the acquisition ring now rides the
/// injected pending target, visible and half-filled.
#[cfg(feature = "debug")]
fn assert_dwell_ring(world: &mut World) {
    use bevy::ui_render::prelude::MaterialNode;

    // The driver + widget have run: the ring now rides the pending target,
    // visible and half-filled.
    let ring_visibility = *world
        .query_filtered::<&Visibility, With<LockDwellRingMarker>>()
        .iter(world)
        .next()
        .expect("hud range: no lock-dwell ring node");
    assert_eq!(
        ring_visibility,
        Visibility::Visible,
        "hud range: the dwell ring is not visible while a dwell charges"
    );
    let handle = world
        .query_filtered::<&MaterialNode<LockDwellRingMaterial>, With<LockDwellRingMarker>>()
        .iter(world)
        .next()
        .expect("hud range: the ring has no material node")
        .0
        .clone();
    let progress = world
        .resource::<Assets<LockDwellRingMaterial>>()
        .get(&handle)
        .expect("hud range: the ring material is missing")
        .data
        .progress;
    assert!(
        (progress - 0.5).abs() < 0.05,
        "hud range: ring fill {progress:.2} is not the injected ~0.5 dwell fraction"
    );
    // Clean up so the downstream kill stage sees no stray RadarState.
    let player = player_root(world);
    world.entity_mut(player).remove::<RadarState>();
    nova_probe::probe_marker(
        world,
        "outcome: the dwell ring rides the dwell",
        serde_json::json!({}),
    );
    info!("hud range: dwell ring visible + filled OK (progress {progress:.2})");
}

/// The lock's whole HUD surface at once: the reticle centered on the target's
/// projection, the readout's real distance and full health bar, the turret feed
/// aimed at the locked ship's live structure, the lead pip on the projected aim
/// point, and the completed focus dwell (meter gone, one component marker per
/// section) with the inset viewfinder still single and visible.
#[cfg(feature = "debug")]
fn assert_lock_indicators(world: &mut World) {
    let player = player_root(world);
    let lock = world
        .entity(player)
        .get::<CombatLock>()
        .unwrap()
        .0
        .expect("hud range: the combat lock was lost before the dwell completed");
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
    nova_probe::probe_marker(
        world,
        "outcome: the reticle sits on the locked target",
        serde_json::json!({}),
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
    nova_probe::probe_marker(
        world,
        "outcome: the readout carries distance and health",
        serde_json::json!({}),
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
    nova_probe::probe_marker(
        world,
        "outcome: the lead pip sits on the aim point",
        serde_json::json!({}),
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
    nova_probe::probe_marker(
        world,
        "outcome: one component marker per section",
        serde_json::json!({}),
    );
    info!("hud range: component markers OK ({markers} of {sections} sections)");

    // The inset has been up since the LOCK (pinned at the commit stage);
    // by now the dwell has also passed - still exactly one RTT camera
    // and a visible panel (the reconcile never duplicated it).
    let inset_cameras = world
        .query_filtered::<(), With<TargetInsetCameraMarker>>()
        .iter(world)
        .count();
    assert_eq!(
        inset_cameras, 1,
        "hud range: expected exactly one target-inset camera while focused"
    );
    let panel_visibility = *world
        .query_filtered::<&Visibility, With<TargetInsetHudMarker>>()
        .iter(world)
        .next()
        .expect("hud range: no target-inset panel node");
    assert_eq!(
        panel_visibility,
        Visibility::Visible,
        "hud range: the target-inset panel is not visible while focused"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the target inset films the lock",
        serde_json::json!({}),
    );
    info!("hud range: target inset OK (1 camera, panel visible)");

    // The weapons safety is OFF while the combat lock exists (lowered
    // stance - the lock alone keeps the guns hot, task 20260713-082337).
    assert!(
        world.entity(player).get::<WeaponsHot>().unwrap().0,
        "hud range: a combat lock must keep the weapons hot"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the safety is hot while combat-locked",
        serde_json::json!({}),
    );
    info!("hud range: weapons hot while combat-locked OK");
}

/// Engage a GOTO on the target ship: the destination marker's driver.
#[cfg(feature = "debug")]
fn engage_goto(world: &mut World) {
    let target = target_root(world).expect("hud range: target ship vanished before the GOTO");
    let player = player_root(world);
    world
        .entity_mut(player)
        .insert(Autopilot::engage(AutopilotAction::Goto { target }));
    info!("hud range: GOTO engaged on the target ship");
}

/// Pin a component lock on the target's TAIL section.
#[cfg(feature = "debug")]
fn pin_component_lock(world: &mut World) {
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
    // Well past the end of the script, so the pin outlives every beat
    // that reads it.
    let until = world.resource::<Time>().elapsed_secs() + 30.0;
    let player = player_root(world);
    let mut entity = world.entity_mut(player);
    let mut component = entity.get_mut::<ComponentLock>().unwrap();
    component.section = Some(tail);
    component.mode = ComponentLockMode::Pinned { until };
    info!("hud range: pinned component lock on the tail section");
}

/// The approach burn's HUD surface: the destination marker centered on the GOTO
/// target, a positive closing speed in the readout, the velocity sphere
/// following the ship's actual velocity, and the pinned tail marker
/// highlighted above its siblings.
#[cfg(feature = "debug")]
fn assert_goto_indicators(world: &mut World) {
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
    nova_probe::probe_marker(
        world,
        "outcome: the destination marker tracks the goto",
        serde_json::json!({}),
    );
    info!("hud range: GOTO destination marker OK (drift {drift:.1} px, '{closing_text}')");

    // Velocity sphere (folded in from the retired 05_directional, task
    // 20260712-211352): under the same approach burn, the sphere widget
    // the production HUD mounted on the player must point its orbit
    // output along the ship's actual velocity.
    let player = player_root(world);
    let ship_velocity = world
        .entity(player)
        .get::<avian3d::prelude::LinearVelocity>()
        .expect("hud range: the player ship has a velocity")
        .0;
    let orbit_direction = {
        let mut q = world.query_filtered::<(
                &DirectionalSphereOrbitOutput,
                &VelocityHudTargetEntity,
            ), With<VelocityHudMarker>>();
        q.iter(world)
            .find(|(_, target)| ***target == player)
            .map(|(output, _)| output.0)
            .expect("hud range: the player's velocity sphere widget must exist")
    };
    let alignment = orbit_direction
        .normalize_or_zero()
        .dot(ship_velocity.normalize_or_zero());
    assert!(
            alignment > 0.95,
            "hud range: the velocity sphere direction {orbit_direction:?} does not follow the ship velocity {ship_velocity:?} (dot {alignment:.3})"
        );
    nova_probe::probe_marker(
        world,
        "outcome: the velocity sphere tracks the burn",
        serde_json::json!({}),
    );
    info!("hud range: velocity sphere tracks the burn (dot {alignment:.3})");

    // The pinned tail section's marker must carry the highlight style.
    let player = player_root(world);
    let pinned = world
        .entity(player)
        .get::<ComponentLock>()
        .unwrap()
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
        .fold(
            (Vec::new(), Vec::new()),
            |(mut sel, mut rest), (px, is_sel)| {
                if is_sel {
                    sel.push(px);
                } else {
                    rest.push(px);
                }
                (sel, rest)
            },
        );
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
    nova_probe::probe_marker(
        world,
        "outcome: the pinned component marker is highlighted",
        serde_json::json!({}),
    );
    info!(
        "hud range: component highlight OK (selected {:.0} px)",
        selected[0]
    );
}

/// Despawn the target and disable the turret section, so every indicator loses
/// its anchor at once.
#[cfg(feature = "debug")]
fn kill_target(world: &mut World) {
    let target = target_root(world).expect("hud range: target ship vanished before the kill");
    // Physical destruction is the kill-cam seam; generic despawn can be cleanup.
    world.entity_mut(target).insert(IntegrityDestroyMarker);
    // The turret keeps aiming at the camera ray even with no enemy, so
    // its anchor only clears through the disabled path: mark the section
    // inactive like the health pipeline does.
    let turret = player_turret(world);
    world.entity_mut(turret).insert(SectionInactiveMarker);
    info!("hud range: target ship despawned, turret section disabled");
}

/// The script's last beat: every indicator hid when its anchor died, the
/// component markers went with it, the kill cam holds the frozen final shot,
/// and the safety re-engaged by itself.
#[cfg(feature = "debug")]
fn assert_indicators_hid(world: &mut World) {
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
    let (_, _, pip_visibility) = indicator_state::<TurretLeadPipMarker>(world, "turret lead pip");
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

    // The KILL CAM (spike 20260713-154023): the target died while
    // framed, so the panel and camera HOLD the frozen final shot for
    // KILL_CAM_SECS - this assert runs ~0.4s after the kill, inside
    // the linger (expiry-closes is unit-tested; the 6s autopilot
    // window ends before the linger does).
    let inset_cameras = world
        .query_filtered::<(), With<TargetInsetCameraMarker>>()
        .iter(world)
        .count();
    assert_eq!(
        inset_cameras, 1,
        "hud range: the kill cam must keep filming after the target's death"
    );
    let panel_visibility = *world
        .query_filtered::<&Visibility, With<TargetInsetHudMarker>>()
        .iter(world)
        .next()
        .expect("hud range: no target-inset panel node");
    assert_eq!(
        panel_visibility,
        Visibility::Visible,
        "hud range: the kill cam must hold the panel open on the final shot"
    );
    // The dead lock cleared (upkeep) and the stance is lowered: the
    // weapons safety re-engages by itself.
    let player = player_root(world);
    assert!(
        !world.entity(player).get::<WeaponsHot>().unwrap().0,
        "hud range: the safety must re-engage once the lock dies lowered"
    );
    nova_probe::probe_marker(
        world,
        "outcome: every indicator hides when its anchor dies",
        serde_json::json!({}),
    );
    info!("hud range: PASS - indicators track their anchors and hide when they die");
}
