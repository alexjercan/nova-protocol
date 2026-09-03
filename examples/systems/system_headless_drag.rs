//! system_headless_drag: spike 5 for `nova_channel` - drag a slider by wire.
//!
//! The ledger row this proves (task 20260820-174148, nova-channel.html):
//! "Drag a slider, scrub a numeric grip - `Pointer<Drag>` observers on the
//! widget; the pointer lane fires them". The one slider in the settings UI is
//! the master volume (`"Volume Slider Track"`, Audio tab); the drive is the
//! full human shape - hover, press, two cursor legs to the right, release -
//! and the verdict is the RESOURCE: `MasterVolume` moved by the drag, and the
//! widget's own `SliderValue` agrees with it.
//!
//! Two mechanics worth knowing (both from `bevy_ui_widgets` 0.19):
//!
//!   - the track is `TrackClick::Snap`, so the arming press ALREADY sets the
//!     value to the pointer's position; the baseline is recorded after the
//!     press so the verdict measures the drag, not the snap;
//!   - `Pointer<*>` auto-propagates, so a press on the track's visual children
//!     resolves to the slider entity - aiming at the track centre is safe.
//!
//! The range then walks to CONTROLS -> MOUSE and drags a second track, the one
//! setting whose value is not the resource anybody reads: the sliders speak
//! PERCENTAGES, `MouseSensitivity` stores raw gains, and
//! `apply_mouse_sensitivity` pushes those onto the `Scale` of every tagged
//! binding. So the mouse verdict reads all three - the slider, the resource and
//! the live `Scale` on the flight rig - because a wire client that moved the
//! slider and never reached the rig would look identical at the first two.
//!
//! This range also prints the `ui` census the design record proposes for the
//! snapshot's `ui` block: with the settings modal open, one JSON object that
//! says which named widgets are on the screen, where, and which of them are
//! buttons - what a channel client would read before deciding where to click.
//!
//! The store is INERT under `NOVA_AUTOPILOT`, which is what keeps the STARTING
//! volume deterministic and stops the drag debounce-saving into the developer's
//! real `settings.ron`. `system_headless_rebind` asserts that gate.
//!
//! Run (no display needed):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_headless_drag --features debug
//! # look for: `headless drag: PASS ...` and the `ui census` JSON line.
//! ```

#[cfg(feature = "debug")]
use bevy::{prelude::*, ui::Pressed, ui_widgets::SliderValue, window::PrimaryWindow};
#[cfg(feature = "debug")]
use bevy_enhanced_input::prelude::Scale;
#[cfg(feature = "debug")]
use nova_input::prelude::{MousePath, MouseSensitivity};
#[cfg(feature = "debug")]
use nova_protocol::nova_os_ui::nova_os::prelude::NovaOsTerminal;
#[cfg(feature = "debug")]
use nova_protocol::prelude::*;

#[cfg(not(feature = "debug"))]
fn main() {
    eprintln!("system_headless_drag drives the app through the debug-only autopilot gestures;");
    eprintln!("run it with --features debug");
}

#[cfg(feature = "debug")]
fn main() -> bevy::app::AppExit {
    let mut app = editor_app(false, Some(StartupScenario::Id("first_shift".to_string())));

    app.world_mut().spawn((
        Window {
            resolution: (1280, 720).into(),
            ..default()
        },
        PrimaryWindow,
    ));

    app.add_plugins(
        nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            .step("headless drag: reach Playing with no renderer")
            .until(state_is(GameStates::Playing))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step("headless drag: ESC opens the pause overlay")
            .on_enter(press_key(KeyCode::Escape))
            .until(resource_where::<State<PauseStates>>(|pause| {
                *pause.get() == PauseStates::Paused
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless drag: release ESC")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            // The modal opens on the Audio tab, which is where the track is.
            .click_named(
                "headless drag: open Settings",
                "Pause Settings Button",
                ui_node_present(TRACK),
                BEAT_DEADLINE_SECS,
            )
            // The census is taken here, with the most UI on screen this range
            // ever has.
            .step("headless drag: census the screen")
            .on_enter(|world: &mut World| {
                let census = ui_census(world);
                info!("headless drag: ui census {census}");
            })
            .add()
            .step("headless drag: aim at the track")
            .on_enter(hover_named(TRACK))
            .until(pointer_over_node(TRACK))
            .diagnose(pointer_hover_diagnosis(TRACK))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            // The arming press. `TrackClick::Snap` sets the value from the
            // pointer's position in the same observer that marks the slider
            // `Pressed`, so that mark is the press LANDING on the widget - not
            // a guess at how long the pointer -> picking -> observer chain
            // takes.
            .step("headless drag: the track took the press")
            .on_enter(press_mouse(MouseButton::Left))
            .until(node_is_pressed(TRACK))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless drag: stamp the snap")
            .on_enter(stamp_the_snap)
            .add()
            // Two legs right along the track's centreline (it is 10-14 px
            // tall; vertical drift would leave the node).
            .step("headless drag: drag along the track")
            .on_enter(drag_right_along(TRACK))
            .until(pointer_at_node(TRACK, Vec2::new(DRAG_PX, 0.0)))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            // The widget's own value is what the release waits on: bevy writes
            // `SliderValue`, and Nova's `ValueChange` observer writes
            // `MasterVolume` off it. The verdict below reads the RESOURCE, so
            // it states something this beat did not already prove.
            .step("headless drag: release the track")
            .on_enter(release_mouse(MouseButton::Left))
            .until(the_widget_rose_off_the_snap())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless drag: the drag moved the volume")
            .on_enter(assert_the_drag_landed)
            .add()
            // The second track: a setting the player reads in percent and the
            // engine reads as a gain on a binding.
            .click_named(
                "headless drag: open the Controls tab",
                "Settings Tab: Controls",
                ui_node_present(MOUSE_GROUP_BUTTON),
                BEAT_DEADLINE_SECS,
            )
            .click_named(
                "headless drag: open the MOUSE group",
                MOUSE_GROUP_BUTTON,
                ui_node_present(LOOK_TRACK),
                BEAT_DEADLINE_SECS,
            )
            .step("headless drag: aim at the look track")
            .on_enter(hover_named(LOOK_TRACK))
            .until(pointer_over_node(LOOK_TRACK))
            .diagnose(pointer_hover_diagnosis(LOOK_TRACK))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless drag: the look track took the press")
            .on_enter(press_mouse(MouseButton::Left))
            .until(node_is_pressed(LOOK_TRACK))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless drag: stamp the look snap")
            .on_enter(stamp_the_look_snap)
            .add()
            .step("headless drag: drag along the look track")
            .on_enter(drag_right_along(LOOK_TRACK))
            .until(pointer_at_node(LOOK_TRACK, Vec2::new(DRAG_PX, 0.0)))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless drag: release the look track")
            .on_enter(release_mouse(MouseButton::Left))
            .until(the_look_rose_off_the_snap())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            // One frame past the release for `apply_mouse_sensitivity`, which
            // runs in `PreUpdate`, to carry the new gain onto the rig.
            .step("headless drag: the rig took the new gain")
            .until(the_rig_agrees_with_the_resource())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless drag: the drag moved the look sensitivity")
            .on_enter(assert_the_look_drag_landed)
            .add(),
    );

    app.run()
}

/// The slider this range drags: the master volume on the Audio tab. The Audio
/// tab now draws one track per bus, and the master is the one `MasterVolume`
/// answers for.
#[cfg(feature = "debug")]
const TRACK: &str = "Master Volume Slider Track";

/// The Controls group that holds the sensitivity sliders, and the one track
/// this range drags on it. `Look` is the path the flight rig wears, so it is
/// the one whose `Scale` this run can read back.
#[cfg(feature = "debug")]
const MOUSE_GROUP_BUTTON: &str = "Controls Group: MOUSE";

/// The look-sensitivity track, named by `MousePath::Look.label()`.
#[cfg(feature = "debug")]
const LOOK_TRACK: &str = "Look Sensitivity Slider Track";

/// Horizontal drag distance. The track is ~430 px wide, so this is ~+0.14 of
/// the volume's 0..=1 range and ~+28 points of the look slider's 100..=300 -
/// far beyond what either verdict demands over snap jitter.
#[cfg(feature = "debug")]
const DRAG_PX: f32 = 60.0;

/// Percentage points a look drag must clear to count, over snap jitter.
#[cfg(feature = "debug")]
const LOOK_MARGIN_PERCENT: f32 = 2.0;

/// `MasterVolume` and the widget's own value at the arming press, before the
/// drag - the baseline the verdict measures against.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct Snap {
    volume: f32,
    widget: f32,
}

/// The track's own `SliderValue`, which bevy writes and Nova's `ValueChange`
/// observer reads.
#[cfg(feature = "debug")]
fn widget_value(world: &World) -> Option<f32> {
    world
        .try_query::<(&Name, &SliderValue)>()?
        .iter(world)
        .find(|(name, _)| name.as_str() == TRACK)
        .map(|(_, value)| value.0)
}

/// Advance once the slider carries bevy's `Pressed` mark - the press reaching
/// the WIDGET, one whole pointer -> picking -> observer chain past
/// `pointer_pressed`.
#[cfg(feature = "debug")]
fn node_is_pressed(
    track: &'static str,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .try_query_filtered::<&Name, With<Pressed>>()
            .is_some_and(|mut query| query.iter(world).any(|name| name.as_str() == track))
    })
}

/// Advance once the WIDGET's value has risen off the snap baseline.
///
/// Deliberately not the resource: the verdict states that `MasterVolume` moved
/// and that the two agree, and a beat that waited on either of those would be
/// gating this range's claim on itself.
#[cfg(feature = "debug")]
fn the_widget_rose_off_the_snap() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(snap) = world.get_resource::<Snap>() else {
            return false;
        };
        widget_value(world).is_some_and(|now| now > snap.widget + 0.02)
    })
}

/// Record where the arming press left the volume.
#[cfg(feature = "debug")]
fn stamp_the_snap(world: &mut World) {
    let volume = world.resource::<MasterVolume>().0;
    let widget = widget_value(world).expect("the track entity carries bevy's SliderValue");
    info!("headless drag: press snapped the volume to {volume}");
    world.insert_resource(Snap { volume, widget });
}

/// Two cursor legs right along a track's centreline.
#[cfg(feature = "debug")]
fn drag_right_along(track: &'static str) -> impl Fn(&mut World) {
    move |world: &mut World| {
        let centre =
            ui_node_centre(world, track).unwrap_or_else(|| panic!("`{track}` vanished mid-drag"));
        move_cursor(centre + Vec2::new(DRAG_PX * 0.5, 0.0))(world);
        move_cursor(centre + Vec2::new(DRAG_PX, 0.0))(world);
    }
}

/// The verdict: the RESOURCE moved past snap jitter, and the widget agrees
/// with it.
#[cfg(feature = "debug")]
fn assert_the_drag_landed(world: &mut World) {
    let before = world.resource::<Snap>().volume;
    let after = world.resource::<MasterVolume>().0;
    assert!(
        after > before + 0.02,
        "the drag must raise MasterVolume past snap jitter ({before} -> {after})"
    );
    let widget = widget_value(world).expect("the track entity carries bevy's SliderValue");
    assert!(
        (widget - after).abs() < 1e-6,
        "the widget and the resource must agree ({widget} vs {after})"
    );
    info!("headless drag: PASS the drag moved MasterVolume {before} -> {after}");
    nova_probe::probe_marker(
        world,
        "outcome: a wire drag moves the volume",
        serde_json::json!({ "before": before, "after": after }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: the slider widget agrees with the resource",
        serde_json::json!({ "widget": widget, "resource": after }),
    );
}

/// `MouseSensitivity`'s look percentage at the arming press, before the drag.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct LookSnap {
    percent: f32,
}

/// The look slider's own value - a PERCENTAGE, unlike the volume track's 0..=1.
#[cfg(feature = "debug")]
fn look_widget_percent(world: &World) -> Option<f32> {
    world
        .try_query::<(&Name, &SliderValue)>()?
        .iter(world)
        .find(|(name, _)| name.as_str() == LOOK_TRACK)
        .map(|(_, value)| value.0)
}

/// The gain the flight rig is actually reading mouse motion through: the
/// `Scale` on the binding tagged `MousePath::Look`.
#[cfg(feature = "debug")]
fn rig_look_gain(world: &World) -> Option<f32> {
    world
        .try_query::<(&MousePath, &Scale)>()?
        .iter(world)
        .find(|(path, _)| **path == MousePath::Look)
        .map(|(_, scale)| scale.factor.x)
}

/// Record where the arming press left the look sensitivity.
#[cfg(feature = "debug")]
fn stamp_the_look_snap(world: &mut World) {
    let percent = world
        .resource::<MouseSensitivity>()
        .percent(MousePath::Look);
    info!("headless drag: press snapped the look sensitivity to {percent}%");
    world.insert_resource(LookSnap { percent });
}

/// Advance once the RESOURCE has risen off the snap baseline.
#[cfg(feature = "debug")]
fn the_look_rose_off_the_snap() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(snap) = world.get_resource::<LookSnap>() else {
            return false;
        };
        world
            .resource::<MouseSensitivity>()
            .percent(MousePath::Look)
            > snap.percent + LOOK_MARGIN_PERCENT
    })
}

/// Advance once `apply_mouse_sensitivity` has carried the new gain onto the
/// rig. That system runs in `PreUpdate`, so it is a frame behind the release.
#[cfg(feature = "debug")]
fn the_rig_agrees_with_the_resource(
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let wanted = world.resource::<MouseSensitivity>().raw(MousePath::Look);
        rig_look_gain(world).is_some_and(|gain| (gain - wanted).abs() < 1e-9)
    })
}

/// The verdict: the resource moved past snap jitter, the slider agrees with it
/// in percent, and the rig is reading motion through the new gain.
#[cfg(feature = "debug")]
fn assert_the_look_drag_landed(world: &mut World) {
    let before = world.resource::<LookSnap>().percent;
    let sensitivity = *world.resource::<MouseSensitivity>();
    let after = sensitivity.percent(MousePath::Look);
    assert!(
        after > before + LOOK_MARGIN_PERCENT,
        "the drag must raise the look sensitivity past snap jitter \
         ({before}% -> {after}%)"
    );
    let widget = look_widget_percent(world).expect("the look track carries bevy's SliderValue");
    assert!(
        (widget - after).abs() < 1e-2,
        "the widget and the resource must agree ({widget}% vs {after}%)"
    );
    let raw = sensitivity.raw(MousePath::Look);
    let gain = rig_look_gain(world).expect("the flight rig carries a Look-tagged Scale");
    assert!(
        (gain - raw).abs() < 1e-9,
        "the rig must read motion through the new gain ({gain} vs {raw})"
    );
    info!("headless drag: PASS the drag moved the look sensitivity {before}% -> {after}%");
    nova_probe::probe_marker(
        world,
        "outcome: a wire drag moves the look sensitivity",
        serde_json::json!({ "before_percent": before, "after_percent": after }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: the flight rig reads the new mouse gain",
        serde_json::json!({ "raw": raw, "rig_scale": gain }),
    );
}

/// The `ui` block the design record proposes for the snapshot: what the screen
/// says right now - which named widgets are laid out and visible, where their
/// rects are (logical px), which are buttons, plus the mode rungs and the
/// terminal model. A channel client reads this to know where it can click.
#[cfg(feature = "debug")]
fn ui_census(world: &mut World) -> serde_json::Value {
    let mut screen = Vec::new();
    let mut query = world.query::<(
        Entity,
        &Name,
        &ComputedNode,
        &UiGlobalTransform,
        &InheritedVisibility,
    )>();
    for (entity, name, node, xf, visibility) in query.iter(world) {
        if !visibility.get() {
            continue;
        }
        let scale = node.inverse_scale_factor();
        let size = node.size() * scale;
        if size.x <= 0.0 || size.y <= 0.0 {
            continue;
        }
        let centre = xf.translation * scale;
        let min = centre - size * 0.5;
        let button = world.get::<bevy::ui_widgets::Button>(entity).is_some();
        screen.push((
            name.to_string(),
            serde_json::json!({
                "name": name.to_string(),
                "rect": [min.x.round(), min.y.round(), size.x.round(), size.y.round()],
                "button": button,
            }),
        ));
    }
    screen.sort_by(|a, b| a.0.cmp(&b.0));
    let terminal = world.get_resource::<NovaOsTerminal>().map(|terminal| {
        serde_json::json!({
            "prompt": terminal.prompt(),
            "mode": format!("{:?}", terminal.active_mode()),
        })
    });
    serde_json::json!({
        "game_state": format!("{:?}", world.resource::<State<GameStates>>().get()),
        "pause": format!("{:?}", world.resource::<State<PauseStates>>().get()),
        "screen": screen.into_iter().map(|(_, value)| value).collect::<Vec<_>>(),
        "terminal": terminal,
    })
}
