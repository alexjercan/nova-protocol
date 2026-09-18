//! system_hud_scales: the HUD read at the two ends of the hull scale, in two
//! window shapes.
//!
//! `system_hud_indicators` proves the indicator chain exists and tracks, and
//! `system_hud_shell` proves the shells enclose both reference hulls. Neither
//! asks what happens when the thing the HUD is drawn OVER changes size by three
//! orders of magnitude, or when the window it is drawn IN changes shape. This
//! range asks only that, and it asks it as a cross product: the smallest
//! possible hull and the largest fixture one, each read at a tall 4:3 window
//! and at a wide one.
//!
//! One gunship flies. Two things are locked in turn: a one-section drone - the
//! minimum a ship can be - and the carrier fixture, 2 081 sections and 194 m of
//! containment radius. Four claims:
//!
//! - Every visible screen indicator lands on the LIVE window, at both hull
//!   sizes and both shapes. The widget clamps against the camera's viewport;
//!   this reads the window instead, so a camera that kept a stale viewport
//!   across the reshape leaves its indicators off the short edge.
//! - The component-marker overlay paints one marker per attached section until
//!   its budget caps it: one marker on the drone, a capped set on the capital,
//!   every one of them on a distinct live section of the locked hull.
//! - The target inset frames the capital's WHOLE live hull - every corner of
//!   the union of its section colliders lands inside the panel's texture.
//! - The turret's lead pip holds the projected intercept at capital scale, and
//!   holds it again after the reshape moves that projection.
//!
//! Controls: none needed; fly and look around freely in interactive runs.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_hud_scales --features debug
//! # look for: `hud scales: 4:3 / one-section drone: ... indicators on the window`,
//! #           `hud scales: the capital's 2081 sections read as 64 markers`,
//! #           `hud scales: the inset frames the whole hull`,
//! #           `hud scales: the lead pip holds the intercept`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[path = "../shared/dev_fixtures/mod.rs"]
mod dev_fixtures;

use std::collections::BTreeMap;

#[cfg(feature = "debug")]
use avian3d::prelude::{ColliderAabb, Sensor};
use bevy::prelude::*;
#[cfg(feature = "debug")]
use bevy::window::PrimaryWindow;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_hud_scales")]
#[command(version = "1.0.0")]
#[command(about = "A test range for the HUD at both ends of the hull scale in two window shapes. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario the range loads under.
const SCENARIO_ID: &str = "hud_scales";

/// The gunship whose HUD is being read, and its sections.
const PLAYER_ID: &str = "player_ship";
const PLAYER_HELM: &str = "player_helm";
const PLAYER_HULL: &str = "player_hull";
const PLAYER_DRIVE: &str = "player_drive";
const PLAYER_GUN: &str = "player_gun";

/// The minimum hull: one section, nothing else.
const DRONE_ID: &str = "drone";
const DRONE_SECTION: &str = "drone_core";

/// The capital.
const CARRIER_ID: &str = "carrier";

/// Where each target is parked. Both sit off the bow far enough to be framed
/// whole and far enough apart that neither hides the other: the lock upkeep
/// drops a target that loses line of sight, and a capital is a lot of cover.
const DRONE_AT: Meters3 = Meters3::new(-250.0, 0.0, -1_500.0);
const CARRIER_AT: Meters3 = Meters3::new(300.0, 0.0, -1_800.0);

/// The two window shapes, named by the ratio a player would call them.
///
/// Materially different: 4:3 is 1.33 and the wide one is 2.13, so the same
/// world point projects to a different pixel in each, and the wide one is 168
/// px SHORTER - a HUD holding a stale viewport across the reshape draws its
/// indicators off the bottom of the new window.
#[cfg(feature = "debug")]
const TALL_SHAPE: (&str, f32, f32) = ("4:3", 1024.0, 768.0);
#[cfg(feature = "debug")]
const WIDE_SHAPE: (&str, f32, f32) = ("32:15", 1280.0, 600.0);

/// How many section markers the overlay paints at once.
///
/// `MARKER_BUDGET` in `nova_hud/src/component_lock.rs`, which is private. It is
/// restated here because the claim is about the CAP: a hull with more sections
/// than this must read as exactly this many markers, and a hull with fewer must
/// read as one per section.
#[cfg(feature = "debug")]
const MARKER_BUDGET: usize = 64;

/// Square side (px) of the target inset's render texture: `INSET_PANEL_PX` in
/// `nova_hud/src/target_inset.rs`, also private. The panel is a fixed square at
/// every window shape, which is exactly why the framing claim is made against
/// it and not against the window.
#[cfg(feature = "debug")]
const INSET_TEXTURE_PX: f32 = 256.0;

/// How far (px) the lead pip's centre may sit from the fresh projection of the
/// turret's intercept point. The HUD places nodes from last frame's propagated
/// transforms, so a frame of motion is expected slack; the scene is parked, so
/// the measured drift is a fraction of a pixel.
#[cfg(feature = "debug")]
const PIP_TOLERANCE_PX: f32 = 10.0;

/// Seconds of dwell written onto the focus timer when the range takes a lock.
///
/// The range stages the dwell instead of waiting it out. What the dwell itself
/// does is `system_hud_indicators`' claim (`the focus meter fills during the
/// dwell`); here it is only the gate in front of the component layer, and a
/// capital hull costs seconds a frame on a software rasterizer - waiting out
/// two real dwells would spend a third of the run's budget proving nothing this
/// range claims.
#[cfg(feature = "debug")]
const FOCUS_HELD_SECS: f32 = 30.0;

/// In-step seconds a beat gets to reach its world condition.
///
/// Well over the fleet's usual: CI runs this range on a software rasterizer,
/// where one frame of 2 081 sections costs seconds, and the inset renders the
/// scene a second time while the capital is locked. A backstop that names a
/// hung beat, not a budget the range is held to.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 90.0;

/// Frames a reshape or a fresh lock gets to reach the HUD.
///
/// The overlay reconciles in `Update` off the projected transforms of the frame
/// before, so a reading taken in the same frame as the gesture is a reading of
/// the old shape.
#[cfg(feature = "debug")]
const SETTLE_FRAMES: u32 = 4;

/// The frames the range keeps for a human to look at.
#[cfg(feature = "debug")]
const TALL_SHOT: &str = "hud_scales_tall.png";
#[cfg(feature = "debug")]
const WIDE_SHOT: &str = "hud_scales_wide.png";

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.init_resource::<ScaleLog>();
        // No frame-time pass: a capital hull plus a second full render of the
        // scene is not a frame budget anyone should read, and this range claims
        // nothing about one.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        app.add_plugins(nova_screenshot(scales_script()));
    }

    app.run()
}

fn range_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(scales_range(&game_assets, &sections)));
}

/// What a beat measured, so the next beat can compare against a matched
/// reading of this run instead of a number somebody typed.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct ScaleLog {
    /// Where the capital's intercept point projected, per shape.
    pip: Vec<(&'static str, Vec2)>,
}

/// The scene: a gunship at the origin, the minimum hull off one bow and the
/// largest shipped hull off the other.
fn scales_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let at = |id: &str, kind: &str, position: Vec3| SpaceshipSectionConfig {
        id: id.to_string(),
        position,
        rotation: Quat::IDENTITY,
        source: SectionSource::Inline(section(kind)),
    };

    let player = SpaceshipConfig {
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::new(),
        }),
        design: ShipDesignSource::Inline(ShipDesign {
            sections: vec![
                at(PLAYER_HELM, "basic_controller_section", Vec3::ZERO),
                at(
                    PLAYER_HULL,
                    "reinforced_hull_section",
                    Vec3::new(0.0, 0.0, 1.0),
                ),
                at(
                    PLAYER_DRIVE,
                    "basic_thruster_section",
                    Vec3::new(0.0, 0.0, 2.0),
                ),
                SpaceshipSectionConfig {
                    id: PLAYER_GUN.to_string(),
                    position: Vec3::new(0.0, 0.0, -0.75),
                    // The turret's one link point sits under its own origin, so
                    // the mount stands upright only when it is turned onto the
                    // face it clips to. Same placement as
                    // system_turret_gunnery; a gun that does not link leaves
                    // the hull's graph disconnected and the scenario refuses.
                    rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                    source: SectionSource::Inline(section("pdc_kinetic_turret_section")),
                },
            ],
            ..default()
        }),
        ..default()
    };

    // One section, which is the whole point of it: the smallest thing the HUD
    // can be asked to draw a component overlay over.
    let drone = SpaceshipConfig {
        controller: SpaceshipController::None,
        design: ShipDesignSource::Inline(ShipDesign {
            sections: vec![at(DRONE_SECTION, "basic_controller_section", Vec3::ZERO)],
            ..default()
        }),
        ..default()
    };

    let carrier = SpaceshipConfig {
        controller: SpaceshipController::None,
        design: ShipDesignSource::Inline(dev_fixtures::carrier()),
        ..default()
    };

    let spawn = |id: &str, name: &str, position: Meters3, ship: SpaceshipConfig| {
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
        label: None,
        name: EventConfig::OnStart,
        once: false,
        filters: vec![],
        actions: [
            vec![
                spawn(PLAYER_ID, "Scale Test Ship", Meters3::ZERO, player),
                spawn(DRONE_ID, "Survey Drone", DRONE_AT, drone),
                spawn(CARRIER_ID, "Fleet Carrier", CARRIER_AT, carrier),
            ],
            // The range lights itself: the engine spawns no light, and an
            // unlit scene shoots black.
            ThreePointRig::around("scales", Meters3::ZERO, 60.0).actions(),
        ]
        .concat(),
    }];

    ScenarioConfig {
        description: "A test range for the HUD at both ends of the hull scale.".to_string(),
        events,
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "HUD Scale Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

// --- The script --------------------------------------------------------------

/// The run: square the window, read both hulls, stretch the window, read both
/// hulls again.
///
/// The capital is read in the middle of each shape rather than at the end, so
/// the frame kept for a human to look at is the one with the overlay at its
/// busiest.
#[cfg(feature = "debug")]
fn scales_script() -> Script {
    let script = Script::new()
        .step("load the range")
        .enter(GameStates::Loading)
        .until(and(
            player_ship_present(),
            and(ship_present(DRONE_ID), ship_present(CARRIER_ID)),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add();
    let script = read_both_hulls(script, TALL_SHAPE, TALL_SHOT);
    read_both_hulls(script, WIDE_SHAPE, WIDE_SHOT)
}

/// One shape's round: reshape, lock the minimum hull, lock the capital.
#[cfg(feature = "debug")]
fn read_both_hulls(script: Script, shape: (&'static str, f32, f32), shot: &'static str) -> Script {
    let (name, width, height) = shape;
    script
        .step("shape the window")
        .on_enter(set_size(width, height))
        .until(the_window_reshaped(width, height))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("lock the one-section drone")
        .on_enter(lock_and_focus(DRONE_ID))
        .until(the_markers_have_settled(DRONE_ID))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("read the minimum hull")
        .on_enter(assert_the_markers_match_the_hull(name, DRONE_ID))
        .until(frames(SETTLE_FRAMES))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("lock the capital")
        .on_enter(lock_and_focus(CARRIER_ID))
        .until(the_markers_have_settled(CARRIER_ID))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let the overlay settle on the capital")
        .until(frames(SETTLE_FRAMES))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("read the capital")
        .on_enter(assert_the_capital_reads(name))
        .until(frames(SETTLE_FRAMES))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("keep the frame")
        .on_enter(move |world: &mut World| shoot(world, shot))
        .until(shot_written(shot))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}

// --- Staging -----------------------------------------------------------------

/// Reshape the window, the way dragging its corner does.
#[cfg(feature = "debug")]
fn set_size(width: f32, height: f32) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
        let mut window = windows
            .single_mut(world)
            .expect("hud scales: one primary window");
        window.resolution.set(width, height);
        info!("hud scales: the window is now {width} x {height}");
    }
}

/// Take the lock on `id` and hold the focus dwell open on it.
///
/// Both halves are written, not gestured: the radar gesture is
/// `system_hud_indicators`' subject, and with two lockable bodies in the sky
/// the range has to say WHICH one it took rather than let a sweep choose.
#[cfg(feature = "debug")]
fn lock_and_focus(id: &'static str) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let target = object_by_id(world, id)
            .unwrap_or_else(|| panic!("hud scales: '{id}' must be in the sky to be locked"));
        let player = player_root(world).expect("hud scales: no player ship to lock from");
        world
            .get_mut::<CombatLock>(player)
            .expect("hud scales: a player ship carries the targeting state")
            .0 = Some(target);
        let mut focus = world
            .get_mut::<LockFocus>(player)
            .expect("hud scales: a player ship carries the focus timer");
        focus.target = Some(target);
        focus.seconds = FOCUS_HELD_SECS;
        info!("hud scales: locked '{id}'");
    }
}

// --- Claims ------------------------------------------------------------------

/// The minimum hull's reading: one marker for its one section, and every
/// visible indicator on the window.
#[cfg(feature = "debug")]
fn assert_the_markers_match_the_hull(
    shape: &'static str,
    id: &'static str,
) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        assert_markers(world, shape, id);
        assert_indicators_are_on_the_window(world, shape, id);
    }
}

/// The capital's reading: the capped marker set, the framing, the pip, and
/// every visible indicator on the window.
#[cfg(feature = "debug")]
fn assert_the_capital_reads(shape: &'static str) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        assert_markers(world, shape, CARRIER_ID);
        assert_indicators_are_on_the_window(world, shape, CARRIER_ID);
        assert_the_inset_frames_the_whole_hull(world, shape);
        assert_the_pip_holds_the_intercept(world, shape);
    }
}

/// One marker per attached section, until the overlay's budget caps it.
///
/// The count is the claim, and so is what each marker POINTS at: a capped set
/// that marked one section twice would be the same number of dots over half the
/// silhouette.
#[cfg(feature = "debug")]
fn assert_markers(world: &mut World, shape: &str, id: &str) {
    let target = object_by_id(world, id)
        .unwrap_or_else(|| panic!("hud scales: '{id}' must still be in the sky"));
    let sections = live_sections(world, target);
    let marked = marked_sections(world);
    let wanted = sections.len().min(MARKER_BUDGET);

    assert_eq!(
        marked.len(),
        wanted,
        "hud scales: {shape} / '{id}': {} attached sections read as {} markers, not the \
         {wanted} one-per-section-up-to-{MARKER_BUDGET} the overlay owes",
        sections.len(),
        marked.len()
    );
    let distinct: std::collections::BTreeSet<Entity> = marked.iter().copied().collect();
    assert_eq!(
        distinct.len(),
        marked.len(),
        "hud scales: {shape} / '{id}': the marker set marks a section twice"
    );
    for section in &marked {
        assert!(
            sections.contains(section),
            "hud scales: {shape} / '{id}': a marker points at {section:?}, which is not an \
             attached section of the locked hull"
        );
    }
    nova_probe::probe_marker(
        world,
        "outcome: the markers count one per section until the budget caps them",
        serde_json::json!({
            "shape": shape,
            "hull": id,
            "sections": sections.len(),
            "markers": marked.len(),
            "budget": MARKER_BUDGET,
        }),
    );
    info!(
        "hud scales: {shape} / '{id}': {} sections read as {} markers",
        sections.len(),
        marked.len()
    );
}

/// Every visible screen indicator lands on the live window.
///
/// Measured against the WINDOW, not against the camera the widget clamps to:
/// the two agree only while the camera's viewport is current, and a reshape is
/// exactly where they stop agreeing.
///
/// Two readings, because the widget owes two different things. Every indicator
/// owes its CENTRE - that is what the placement pass positions and what decides
/// whether the player sees the thing at all. An indicator clamped to the edge
/// owes its whole BOX as well, because clamping is the promise that an
/// off-screen anchor still draws a widget the player can read; the crosshair
/// over a 194 m hull at close range is legitimately wider than the window, and
/// asking IT for a contained box would be asking the HUD to shrink the hull.
#[cfg(feature = "debug")]
fn assert_indicators_are_on_the_window(world: &mut World, shape: &str, id: &str) {
    let window = window_size(world);
    let readings = indicator_boxes(world);
    assert!(
        !readings.is_empty(),
        "hud scales: {shape} / '{id}': no visible indicator at all - the HUD is not up"
    );

    let mut worst_centre: Option<(f32, Vec2)> = None;
    let mut worst_clamped: Option<(f32, Rect)> = None;
    let mut clamping = 0usize;
    for reading in &readings {
        let centre = reading.rect.center();
        let outside = (-centre.x)
            .max(-centre.y)
            .max(centre.x - window.x)
            .max(centre.y - window.y);
        if worst_centre.is_none_or(|(worst, _)| outside > worst) {
            worst_centre = Some((outside, centre));
        }
        if reading.clamping {
            clamping += 1;
            let overhang = (-reading.rect.min.x)
                .max(-reading.rect.min.y)
                .max(reading.rect.max.x - window.x)
                .max(reading.rect.max.y - window.y);
            if worst_clamped.is_none_or(|(worst, _)| overhang > worst) {
                worst_clamped = Some((overhang, reading.rect));
            }
        }
    }

    let (centre_outside, centre_at) = worst_centre.expect("hud scales: the reading set is empty");
    assert!(
        centre_outside <= 0.0,
        "hud scales: {shape} / '{id}': an indicator is centred at {centre_at:?}, \
         {centre_outside:.1} px off a {window:?} window"
    );
    if let Some((overhang, rect)) = worst_clamped {
        assert!(
            overhang <= 0.0,
            "hud scales: {shape} / '{id}': an edge-clamped indicator draws {overhang:.1} px \
             off a {window:?} window ({rect:?}) - a clamped widget the player cannot read \
             is the whole reason the policy exists"
        );
    }

    nova_probe::probe_marker(
        world,
        "outcome: every visible indicator lands on the live window",
        serde_json::json!({
            "shape": shape,
            "hull": id,
            "window": [window.x, window.y],
            "visible_indicators": readings.len(),
            "edge_clamping_indicators": clamping,
            "worst_centre_margin_px": -centre_outside,
            "worst_clamped_margin_px": worst_clamped.map(|(overhang, _)| -overhang),
        }),
    );
    info!(
        "hud scales: {shape} / '{id}': {} visible indicators on a {} x {} window, {clamping} of \
         them edge-clamping; the closest centre to an edge has {:.1} px to spare",
        readings.len(),
        window.x,
        window.y,
        -centre_outside
    );
}

/// The target inset frames the capital's whole live hull.
///
/// The panel is a fixed 256 px square whatever the window is doing, so the
/// framing is the one HUD reading a reshape must NOT move. What is projected is
/// the union of the hull's live section colliders - the eight corners of the
/// box the player would call the ship - through the inset's own camera.
#[cfg(feature = "debug")]
fn assert_the_inset_frames_the_whole_hull(world: &mut World, shape: &'static str) {
    let carrier = object_by_id(world, CARRIER_ID).expect("hud scales: the capital must be up");
    let hull = hull_box(world, carrier);
    let (camera_entity, viewport) = inset_camera(world);
    assert!(
        (viewport.x - INSET_TEXTURE_PX).abs() < 0.5 && (viewport.y - INSET_TEXTURE_PX).abs() < 0.5,
        "hud scales: {shape}: the inset renders into a {viewport:?} target, not the \
         {INSET_TEXTURE_PX} px square the panel shows"
    );

    let mut worst: Option<(f32, Vec3, Vec2)> = None;
    for corner in box_corners(hull) {
        let projected = project_through(world, camera_entity, corner).unwrap_or_else(|| {
            panic!(
                "hud scales: {shape}: the hull corner {corner:?} does not project through the \
                 inset camera at all - it is behind the eye"
            )
        });
        let outside = (-projected.x)
            .max(-projected.y)
            .max(projected.x - viewport.x)
            .max(projected.y - viewport.y);
        if worst.is_none_or(|(margin, _, _)| outside > margin) {
            worst = Some((outside, corner, projected));
        }
    }
    let (outside, corner, projected) = worst.expect("hud scales: a box has eight corners");
    assert!(
        outside <= 0.0,
        "hud scales: {shape}: the capital's hull corner {corner:?} projects to {projected:?} in \
         the inset, {outside:.1} px outside its {viewport:?} panel - the scope is cropping the \
         ship it exists to show"
    );

    nova_probe::probe_marker(
        world,
        "outcome: the inset frames the capital's whole hull",
        serde_json::json!({
            "shape": shape,
            "hull_min": [hull.min.x, hull.min.y, hull.min.z],
            "hull_max": [hull.max.x, hull.max.y, hull.max.z],
            "panel_px": [viewport.x, viewport.y],
            "tightest_margin_px": -outside,
        }),
    );
    info!(
        "hud scales: {shape}: the inset frames the whole hull ({:.0} m across), tightest corner \
         {:.1} px inside the panel edge",
        Meters::from_engine((hull.max - hull.min).length()).get(),
        -outside
    );
}

/// The lead pip holds the projected intercept at capital scale, in both shapes.
///
/// The second reading is the one the shape buys: the same world point projects
/// to a different pixel in a 1024 x 768 window than in a 1280 x 600 one, and
/// the range asserts the two expectations really did move before it believes
/// the pip followed them.
#[cfg(feature = "debug")]
fn assert_the_pip_holds_the_intercept(world: &mut World, shape: &'static str) {
    let turret = section_by_id(world, PLAYER_GUN).expect("hud scales: the gunship has one turret");
    let aim = (**world
        .get::<TurretSectionAimPoint>(turret)
        .expect("hud scales: the turret carries no aim point"))
    .expect("hud scales: the turret never computed an intercept point");
    let expected = project_through_indicator_camera(world, aim)
        .expect("hud scales: the intercept point does not project onto the viewport");
    let (centre, visible) = indicator_box::<TurretLeadPipMarker>(world, "lead pip");
    assert_eq!(
        visible,
        Visibility::Visible,
        "hud scales: {shape}: the lead pip is not visible while the turret tracks a capital"
    );
    let drift = centre.distance(expected);
    assert!(
        drift < PIP_TOLERANCE_PX,
        "hud scales: {shape}: the lead pip sits at {centre:?}, {drift:.1} px from the projected \
         intercept {expected:?}"
    );

    // The turret is aiming at the CAPITAL, not at the camera-ray point a
    // feedless turret falls back to. On a 420 m hull that is a claim worth
    // making in world space: a lock feed that dropped would still project to
    // roughly the same pixel, because the hull is dead ahead of the gun.
    let hull = hull_box(
        world,
        object_by_id(world, CARRIER_ID).expect("hud scales: the capital"),
    );
    assert!(
        aim.cmpge(hull.min).all() && aim.cmple(hull.max).all(),
        "hud scales: {shape}: the turret's intercept {aim:?} is outside the capital's live hull \
         box ({:?} to {:?}) - the gun is not being fed by the lock",
        hull.min,
        hull.max
    );

    let moved = world
        .resource::<ScaleLog>()
        .pip
        .iter()
        .map(|(_, at)| at.distance(expected))
        .fold(f32::NEG_INFINITY, f32::max);
    if moved.is_finite() {
        assert!(
            moved > PIP_TOLERANCE_PX,
            "hud scales: {shape}: the intercept projects {moved:.1} px from where it did at the \
             other shape - the two readings are the same measurement twice, not a cross product"
        );
    }
    world.resource_mut::<ScaleLog>().pip.push((shape, expected));

    nova_probe::probe_marker(
        world,
        "outcome: the lead pip holds the projected intercept at capital scale",
        serde_json::json!({
            "shape": shape,
            "aim_point": [aim.x, aim.y, aim.z],
            "expected_px": [expected.x, expected.y],
            "pip_px": [centre.x, centre.y],
            "drift_px": drift,
            "moved_from_other_shape_px": moved.is_finite().then_some(moved),
        }),
    );
    info!(
        "hud scales: {shape}: the lead pip holds the intercept inside the capital's hull, \
         {drift:.2} px off a projection that {}",
        if moved.is_finite() {
            format!("moved {moved:.0} px with the shape")
        } else {
            "this shape is the first to read".to_string()
        }
    );
}

// --- Predicates --------------------------------------------------------------

/// A scenario object with `id` is in the sky.
#[cfg(feature = "debug")]
fn ship_present(id: &'static str) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| object_by_id(world, id).is_some())
}

/// The overlay has reconciled onto `id`'s hull: every marker points at one of
/// its sections and there is at least one.
#[cfg(feature = "debug")]
fn the_markers_have_settled(
    id: &'static str,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        let Some(target) = object_by_id(world, id) else {
            return false;
        };
        let Some(sections) = try_live_sections(world, target) else {
            return false;
        };
        let Some(marked) = try_marked_sections(world) else {
            return false;
        };
        !marked.is_empty()
            && marked.len() == sections.len().min(MARKER_BUDGET)
            && marked.iter().all(|section| sections.contains(section))
    })
}

/// Advance once the reshape has reached the pixels the HUD is measured in: the
/// window reports the new shape AND the camera has picked it up.
#[cfg(feature = "debug")]
fn the_window_reshaped(
    width: f32,
    height: f32,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    and(
        window_size_is(width, height),
        std::sync::Arc::new(move |world: &World| {
            world
                .try_query_filtered::<&Camera, With<ScreenIndicatorCamera>>()
                .is_some_and(|mut query| {
                    query.iter(world).any(|camera| {
                        camera.logical_viewport_size().is_some_and(|size| {
                            (size.x - width).abs() < 0.5 && (size.y - height).abs() < 0.5
                        })
                    })
                })
        }),
    )
}

// --- Readings ----------------------------------------------------------------

/// One visible indicator's live box.
#[cfg(feature = "debug")]
struct IndicatorBox {
    /// The node's laid-out rect in logical window pixels.
    rect: Rect,
    /// Whether this indicator's off-screen POLICY is to clamp to the edge -
    /// not whether it is clamped right now.
    clamping: bool,
}

/// Every visible screen indicator's live box, in logical window pixels.
#[cfg(feature = "debug")]
fn indicator_boxes(world: &mut World) -> Vec<IndicatorBox> {
    let mut query = world.query_filtered::<(
        &Node,
        &bevy::ui::ComputedNode,
        &InheritedVisibility,
        &ScreenIndicatorOffscreen,
    ), With<ScreenIndicatorMarker>>();
    query
        .iter(world)
        .filter(|(_, _, inherited, _)| inherited.get())
        .map(|(node, computed, _, offscreen)| {
            let px = |val: Val, name: &str| match val {
                Val::Px(px) => px,
                other => panic!("hud scales: an indicator's {name} is {other:?}, not pixels"),
            };
            let corner = Vec2::new(px(node.left, "left"), px(node.top, "top"));
            let size = computed.size() * computed.inverse_scale_factor();
            IndicatorBox {
                rect: Rect::from_corners(corner, corner + size),
                clamping: matches!(offscreen, ScreenIndicatorOffscreen::ClampToEdge { .. }),
            }
        })
        .collect()
}

/// The centre and visibility of the one indicator node marked `M`.
#[cfg(feature = "debug")]
fn indicator_box<M: Component>(world: &mut World, what: &str) -> (Vec2, Visibility) {
    let (node, computed, visibility) = world
        .query_filtered::<(&Node, &bevy::ui::ComputedNode, &Visibility), With<M>>()
        .iter(world)
        .next()
        .unwrap_or_else(|| panic!("hud scales: no {what} node"));
    let px = |val: Val| match val {
        Val::Px(px) => px,
        other => panic!("hud scales: the {what} is placed at {other:?}, not in pixels"),
    };
    let size = computed.size() * computed.inverse_scale_factor();
    let centre = Vec2::new(px(node.left), px(node.top)) + size / 2.0;
    (centre, *visibility)
}

/// The live window's logical size.
#[cfg(feature = "debug")]
fn window_size(world: &mut World) -> Vec2 {
    let window = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .iter(world)
        .next()
        .expect("hud scales: no primary window");
    Vec2::new(window.width(), window.height())
}

/// The inset's camera and the size of the texture it renders into.
#[cfg(feature = "debug")]
fn inset_camera(world: &mut World) -> (Entity, Vec2) {
    let (entity, viewport) = world
        .query_filtered::<(Entity, &Camera), With<TargetInsetCameraMarker>>()
        .iter(world)
        .next()
        .map(|(entity, camera)| (entity, camera.logical_viewport_size()))
        .expect("hud scales: no target-inset camera while a capital is locked");
    (
        entity,
        viewport.expect("hud scales: the inset camera has no render target size"),
    )
}

/// Project `point` through `camera`, in that camera's own viewport pixels.
#[cfg(feature = "debug")]
fn project_through(world: &mut World, camera: Entity, point: Vec3) -> Option<Vec2> {
    let (transform, camera) = world
        .get::<GlobalTransform>(camera)
        .copied()
        .zip(world.get::<Camera>(camera).cloned())?;
    camera.world_to_viewport(&transform, point).ok()
}

/// Project `point` through the camera the HUD's indicators project through.
#[cfg(feature = "debug")]
fn project_through_indicator_camera(world: &mut World, point: Vec3) -> Option<Vec2> {
    let (transform, camera) = world
        .query_filtered::<(&GlobalTransform, &Camera), With<ScreenIndicatorCamera>>()
        .iter(world)
        .next()
        .map(|(transform, camera)| (*transform, camera.clone()))
        .expect("hud scales: no ScreenIndicatorCamera - the HUD camera glue broke");
    camera.world_to_viewport(&transform, point).ok()
}

/// The union of a body's live, solid collider boxes: the box a player would
/// call the ship.
#[cfg(feature = "debug")]
fn hull_box(world: &mut World, body: Entity) -> Rect3 {
    let mut children = world.query::<&Children>();
    let mut stack = vec![body];
    let mut seen = Vec::new();
    while let Some(entity) = stack.pop() {
        seen.push(entity);
        if let Ok(kids) = children.get(world, entity) {
            stack.extend(kids.iter());
        }
    }
    let mut hull: Option<Rect3> = None;
    for entity in seen {
        if world.get::<Sensor>(entity).is_some() {
            continue;
        }
        let Some(aabb) = world.get::<ColliderAabb>(entity) else {
            continue;
        };
        let (min, max) = (aabb.min, aabb.max);
        hull = Some(match hull {
            Some(box3) => Rect3 {
                min: box3.min.min(min),
                max: box3.max.max(max),
            },
            None => Rect3 { min, max },
        });
    }
    hull.expect("hud scales: the capital has no solid collider to frame")
}

/// A world-space box.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug)]
struct Rect3 {
    min: Vec3,
    max: Vec3,
}

/// The eight corners of a world-space box.
#[cfg(feature = "debug")]
fn box_corners(box3: Rect3) -> [Vec3; 8] {
    let (lo, hi) = (box3.min, box3.max);
    [
        Vec3::new(lo.x, lo.y, lo.z),
        Vec3::new(hi.x, lo.y, lo.z),
        Vec3::new(lo.x, hi.y, lo.z),
        Vec3::new(hi.x, hi.y, lo.z),
        Vec3::new(lo.x, lo.y, hi.z),
        Vec3::new(hi.x, lo.y, hi.z),
        Vec3::new(lo.x, hi.y, hi.z),
        Vec3::new(hi.x, hi.y, hi.z),
    ]
}

/// The sections still attached to `body`.
#[cfg(feature = "debug")]
fn live_sections(world: &mut World, body: Entity) -> Vec<Entity> {
    try_live_sections(world, body).expect("hud scales: the section query must build")
}

/// [`live_sections`] for a predicate, which may not query the world mutably.
#[cfg(feature = "debug")]
fn try_live_sections(world: &World, body: Entity) -> Option<Vec<Entity>> {
    let mut query = world.try_query_filtered::<(Entity, &ChildOf), With<SectionMarker>>()?;
    Some(
        query
            .iter(world)
            .filter(|(_, ChildOf(parent))| *parent == body)
            .map(|(section, _)| section)
            .collect(),
    )
}

/// The sections the overlay is currently marking.
#[cfg(feature = "debug")]
fn marked_sections(world: &mut World) -> Vec<Entity> {
    try_marked_sections(world).expect("hud scales: the marker query must build")
}

/// [`marked_sections`] for a predicate.
#[cfg(feature = "debug")]
fn try_marked_sections(world: &World) -> Option<Vec<Entity>> {
    let mut query = world
        .try_query_filtered::<&ComponentLockSectionTarget, With<ComponentLockSectionMarker>>()?;
    Some(query.iter(world).map(|target| **target).collect())
}

/// The scenario object with `id`.
#[cfg(feature = "debug")]
fn object_by_id(world: &World, id: &str) -> Option<Entity> {
    world
        .try_query::<(Entity, &EntityId)>()
        .and_then(|mut query| {
            query
                .iter(world)
                .find(|(_, entity_id)| ***entity_id == *id)
                .map(|(entity, _)| entity)
        })
}

/// The section slot `id` on whatever hull carries it.
#[cfg(feature = "debug")]
fn section_by_id(world: &mut World, id: &str) -> Option<Entity> {
    world
        .query_filtered::<(Entity, &EntityId), With<SectionMarker>>()
        .iter(world)
        .find(|(_, entity_id)| ***entity_id == *id)
        .map(|(entity, _)| entity)
}

/// The player ship root.
#[cfg(feature = "debug")]
fn player_root(world: &World) -> Option<Entity> {
    world
        .try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .and_then(|mut query| query.iter(world).next())
}
