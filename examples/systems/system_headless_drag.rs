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
//! This range also prints the `ui` census the design record proposes for the
//! snapshot's `ui` block: with the settings modal open, one JSON object that
//! says which named widgets are on the screen, where, and which of them are
//! buttons - what a channel client would read before deciding where to click.
//!
//! The config store is ISOLATED before the app builds: the volume change would
//! otherwise debounce-save into the developer's real `settings.ron`, and the
//! persisted store would also make the STARTING volume nondeterministic.
//!
//! Run (no display needed):
//! ```text
//! cargo run --example system_headless_drag --features debug
//! # look for: `headless drag: PASS ...` and the `ui census` JSON line.
//! ```

#[cfg(feature = "debug")]
use bevy::{prelude::*, ui_widgets::SliderValue, window::PrimaryWindow};
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
    std::env::set_var(
        "NOVA_CONFIG_ROOT",
        std::env::temp_dir().join("nova_channel_drag_config"),
    );

    let mut app = editor_app(
        false,
        Some(StartupScenario::Id("shakedown_run".to_string())),
    );

    app.world_mut().spawn((
        Window {
            resolution: (1280, 720).into(),
            ..default()
        },
        PrimaryWindow,
    ));

    app.init_resource::<Spike>();
    app.add_systems(PreUpdate, drive.after(bevy::input::InputSystems));

    app.run()
}

#[cfg(feature = "debug")]
const TRACK: &str = "Volume Slider Track";

/// Frames a pointer beat holds for picking to consume it - the drag path runs
/// pointer -> picking -> `Pointer<Drag>` observer -> `ValueChange` observer,
/// each a frame apart.
#[cfg(feature = "debug")]
const SETTLE: u32 = 10;

/// Horizontal drag distance. The track is ~430 px wide, so this is ~+0.14 of
/// the 0..=1 range - far beyond the 0.02 the verdict demands over snap jitter.
#[cfg(feature = "debug")]
const DRAG_PX: f32 = 60.0;

#[cfg(feature = "debug")]
#[derive(Resource)]
struct Spike {
    step: usize,
    wait: u32,
    /// Frames since the current click target FIRST resolved - the gesture
    /// counter, distinct from `wait` (frames since the step began).
    phase: u32,
    /// `MasterVolume` after the arming press (the snap), before the drag.
    before: Option<f32>,
    started: std::time::Instant,
}

#[cfg(feature = "debug")]
impl Default for Spike {
    fn default() -> Self {
        Self {
            step: 0,
            wait: 0,
            phase: 0,
            before: None,
            started: std::time::Instant::now(),
        }
    }
}

/// Drive one click on a named widget the way a person does: aim, wait until
/// the pointer is ACTUALLY over it, press, release. True once the release
/// fired. The hover gate is the actionability check - the scenario loading
/// screen fades out OVER the fresh pause overlay and eats picks for about a
/// second, so a resolvable rect is not yet a clickable one; the press waits
/// until the pick map says the aim landed, re-aiming every frame.
#[cfg(feature = "debug")]
fn click_named(world: &mut World, name: &str) -> bool {
    if ui_node_rect(world, name).is_none() {
        world.resource_mut::<Spike>().phase = 0;
        return false;
    }
    let phase = world.resource::<Spike>().phase;
    match phase {
        0 | 1 => {
            hover_named(name)(world);
            if phase == 1 && hovering(world, name) {
                world.resource_mut::<Spike>().phase = 2;
            } else {
                world.resource_mut::<Spike>().phase = 1;
            }
        }
        2 => {
            press_mouse(MouseButton::Left)(world);
            world.resource_mut::<Spike>().phase = 3;
        }
        _ => {
            release_mouse(MouseButton::Left)(world);
            world.resource_mut::<Spike>().phase = 0;
            return true;
        }
    }
    false
}

/// Whether the window mouse pointer's picks include `name` - checked against
/// each hit AND its ancestors, because picking reports the deepest node.
#[cfg(feature = "debug")]
fn hovering(world: &World, name: &str) -> bool {
    use bevy::picking::{hover::HoverMap, pointer::PointerId};
    let Some(hits) = world.resource::<HoverMap>().get(&PointerId::Mouse) else {
        return false;
    };
    hits.keys().any(|hit| {
        std::iter::successors(Some(*hit), |entity| {
            world.get::<ChildOf>(*entity).map(|child| child.parent())
        })
        .any(|entity| {
            world
                .get::<Name>(entity)
                .is_some_and(|named| named.as_str() == name)
        })
    })
}

#[cfg(feature = "debug")]
const DEADLINE_SECS: u64 = 180;

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

#[cfg(feature = "debug")]
fn drive(world: &mut World) {
    let spike = world.resource::<Spike>();
    let (step, wait) = (spike.step, spike.wait);
    if spike.started.elapsed().as_secs() > DEADLINE_SECS {
        panic!("headless drag: STALLED at step {step} after {DEADLINE_SECS}s");
    }

    let advance = |world: &mut World| {
        let mut spike = world.resource_mut::<Spike>();
        spike.step += 1;
        spike.wait = 0;
        spike.phase = 0;
    };
    let hold = |world: &mut World| world.resource_mut::<Spike>().wait += 1;

    match step {
        0 => {
            if *world.resource::<State<GameStates>>().get() == GameStates::Playing {
                advance(world);
            }
        }
        1 => {
            if wait < 5 {
                hold(world);
            } else {
                press_key(KeyCode::Escape)(world);
                advance(world);
            }
        }
        2 => {
            release_key(KeyCode::Escape)(world);
            advance(world);
        }
        3 => {
            if *world.resource::<State<PauseStates>>().get() == PauseStates::Paused {
                advance(world);
            } else {
                hold(world);
            }
        }
        // Open the settings modal: hover, settle, press, release.
        4 => {
            if click_named(world, "Pause Settings Button") {
                advance(world);
            } else {
                hold(world);
            }
        }
        // The modal opens on the Audio tab; the census is taken here, with the
        // most UI on screen this range ever has.
        5 => {
            if ui_node_rect(world, TRACK).is_some() {
                let census = ui_census(world);
                info!("headless drag: ui census {census}");
                advance(world);
            } else {
                hold(world);
            }
        }
        6 => {
            if wait == 0 {
                hover_named(TRACK)(world);
            }
            if wait < SETTLE {
                hold(world);
            } else {
                advance(world);
            }
        }
        // The arming press: TrackClick::Snap sets the value to the pointer's
        // position on this frame, so the baseline is read AFTER it settles.
        7 => {
            if wait == 0 {
                press_mouse(MouseButton::Left)(world);
            }
            if wait < SETTLE {
                hold(world);
            } else {
                let snapped = world.resource::<MasterVolume>().0;
                world.resource_mut::<Spike>().before = Some(snapped);
                info!("headless drag: press snapped the volume to {snapped}");
                advance(world);
            }
        }
        // Two legs right along the track's centreline (it is 10-14 px tall;
        // vertical drift would leave the node).
        8 => {
            if wait == 0 {
                let centre = ui_node_centre(world, TRACK)
                    .unwrap_or_else(|| panic!("`{TRACK}` vanished mid-drag"));
                move_cursor(centre + Vec2::new(DRAG_PX * 0.5, 0.0))(world);
                move_cursor(centre + Vec2::new(DRAG_PX, 0.0))(world);
            }
            if wait < SETTLE {
                hold(world);
            } else {
                advance(world);
            }
        }
        9 => {
            if wait == 0 {
                release_mouse(MouseButton::Left)(world);
            }
            if wait < SETTLE {
                hold(world);
            } else {
                advance(world);
            }
        }
        10 => {
            let before = world
                .resource::<Spike>()
                .before
                .expect("the press beat recorded the snap baseline");
            let after = world.resource::<MasterVolume>().0;
            assert!(
                after > before + 0.02,
                "the drag must raise MasterVolume past snap jitter ({before} -> {after})"
            );
            let mut query = world.query::<(Entity, &Name, &SliderValue)>();
            let widget = query
                .iter(world)
                .find(|(_, name, _)| name.as_str() == TRACK)
                .map(|(_, _, value)| value.0)
                .expect("the track entity carries bevy's SliderValue");
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
            world.write_message(AppExit::Success);
            advance(world);
        }
        _ => {}
    }
}
