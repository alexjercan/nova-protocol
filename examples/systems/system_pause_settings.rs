//! system_pause_settings: the Settings panel stays on screen at every window
//! the game will open in.
//!
//! The panel is the widest layout the game draws and the only one a player can
//! be stuck behind: it is opened from the pause menu in the middle of a run,
//! and the way out of it is a Back button at its own bottom edge.
//!
//! WHAT A FIXED WIDTH ACTUALLY DID, measured here before it was changed: at
//! 500x600 the 620 px panel came out 500 wide, not 620. It is a flex item, so
//! the default `flex_shrink` squeezed it to the window instead of overflowing
//! it - edge to edge, no gutter, and the label column of every keybind row
//! giving up whatever the two fixed chip columns did not. Nothing left the
//! screen. A width nobody chose is still not a layout, and a shrink that runs
//! out of label to take is an overflow one narrower window away, so the panel
//! now asks for a SHARE of the window up to its own maximum and this range
//! pins that number rather than only the containment it never lost.
//!
//! Two halves, because the window is only half the story:
//!
//! 1. THE PANEL. Read at 500x600 - narrower than any native window the game
//!    will now open, and the shape a canvas embedded in a page column arrives
//!    at, where no resize constraint of ours applies. The panel, its tab bar,
//!    every keybind row and the Back button stay inside the window, and the
//!    panel is the width the policy names. Read again at the native floor, and
//!    once more at the stock shape, where the cap - not the percentage - is
//!    what the panel takes.
//! 2. THE WINDOW. The floor itself, handed to the window manager so a drag
//!    stops rather than the UI coming apart, and the mode a fresh install
//!    opens in.
//!
//! Containment is checked as RECTS against the viewport, never as a computed
//! root width: a root that reports 460 while its children draw 620 is the
//! shape of the defect this guards against.
//!
//! The window mode is read as the SETTING rather than off the window: a
//! scripted run's settings store is inert ([`SettingsStoreAccess`]), so
//! `apply_window_mode` is never registered and the harness keeps the window it
//! was given. What a fresh install opens in is the value that would be applied,
//! and what survives an update is a store round trip - both asserted here.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_pause_settings --features debug
//! # look for: `pause_settings: at 500x600 the panel is inside the window`,
//! #           `pause_settings: the window will not go below 640x600`,
//! #           `pause_settings: a fresh install opens borderless`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::{prelude::*, window::PrimaryWindow};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_pause_settings")]
#[command(version = "1.0.0")]
#[command(
    about = "The Settings panel read at every window the game opens in, plus the window floor and the fresh-install mode. Autopilot-only correctness range - play the game to use the menu",
    long_about = None
)]
struct Cli;

/// The scenario the run flies, so ESC is unambiguously the pause menu.
#[cfg(feature = "debug")]
const SCENARIO: &str = "tutorial";

/// The pause menu's way into the panel under test.
#[cfg(feature = "debug")]
const PAUSE_SETTINGS_BUTTON: &str = "Pause Settings Button";
/// A node only the OPEN pause Settings panel has.
#[cfg(feature = "debug")]
const PAUSE_SETTINGS_BACK: &str = "Pause Settings Back Button";
/// The pause panel itself - the node carrying the width policy.
#[cfg(feature = "debug")]
const PAUSE_SETTINGS_PANEL: &str = "Pause Settings Panel";
/// The pause menu's Resume button, so the overlay can be waited on.
#[cfg(feature = "debug")]
const RESUME_BUTTON: &str = "Resume Button";
/// The pause menu's way out to the main menu.
#[cfg(feature = "debug")]
const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";

/// The main menu's way into the SAME panel, built from the same policy.
#[cfg(feature = "debug")]
const MENU_SETTINGS_BUTTON: &str = "Settings Button";
/// A node only the open main-menu Settings panel has.
#[cfg(feature = "debug")]
const MENU_SETTINGS_BACK: &str = "Settings Back Button";
/// The main-menu panel itself.
#[cfg(feature = "debug")]
const MENU_SETTINGS_PANEL: &str = "Settings Panel";

/// The tab bar both panels build, and the tab that fills the body with the
/// widest rows the panel has to hold.
#[cfg(feature = "debug")]
const TAB_BAR: &str = "Settings Tab Bar";
#[cfg(feature = "debug")]
const CONTROLS_TAB: &str = "Settings Tab: Controls";

/// The prefix every rebindable row's `Name` carries.
#[cfg(feature = "debug")]
const KEYBIND_ROW: &str = "Keybind: ";

/// The narrow shape. Under the native floor on purpose: it is the one a canvas
/// in a page column reaches, where the window constraints below do not apply.
#[cfg(feature = "debug")]
const NARROW: Vec2 = Vec2::new(500.0, 600.0);

/// The shape the game opens a window at, where the panel's cap is what decides
/// its width rather than the percentage.
#[cfg(feature = "debug")]
const STOCK: Vec2 = Vec2::new(1024.0, 768.0);

/// How far a measured edge may sit outside the window before it counts as
/// having left it, in logical pixels.
///
/// Not zero: a percentage of an odd window width lands on a fraction, and a
/// border rounds the rect out by a fraction more. The defect this range exists
/// for puts the panel 60 px past each edge.
#[cfg(feature = "debug")]
const DRIFT_PX: f32 = 1.5;

/// The fewest keybind rows the Controls tab must be showing for the row check
/// to mean anything.
#[cfg(feature = "debug")]
const ROWS_EXPECTED: usize = 4;

/// Seconds a boot, a scenario load or a content restart is given. Sized to
/// outlast a software-rendered host and kept under the harness completion
/// deadline, so a stall names THIS beat.
#[cfg(feature = "debug")]
const SESSION_SECS: f32 = 90.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The binary's `--scenario <id>`: the run lands in flight, where ESC is
    // the pause menu and nothing else.
    #[cfg(feature = "debug")]
    let startup = Some(StartupScenario::Id(SCENARIO.to_string()));
    #[cfg(not(feature = "debug"))]
    let startup = None;

    // The same app the game binary runs - not a bespoke copy. RENDERED,
    // because the claim is about what is on the screen at a given window size.
    let mut app = editor_app(true, startup);

    #[cfg(feature = "debug")]
    {
        // No frame-time claim: every measured beat here happens under a paused
        // world with a modal up, and the number that comes out of that says
        // nothing about the game's speed.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(pause_settings_script());
    }

    app.run()
}

/// The viewport the panel is laid out in, in the logical pixels a `Node` is
/// placed in.
#[cfg(feature = "debug")]
fn viewport(world: &mut World) -> Vec2 {
    world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .single(world)
        .expect("one primary window")
        .resolution
        .size()
}

/// Every laid-out rect whose `Name` starts with `prefix`, in logical pixels.
#[cfg(feature = "debug")]
fn rects_starting_with(world: &mut World, prefix: &str) -> Vec<(String, Rect)> {
    world
        .query::<(
            &Name,
            &bevy::ui::UiGlobalTransform,
            &ComputedNode,
            &InheritedVisibility,
        )>()
        .iter(world)
        .filter(|(name, _, _, visible)| visible.get() && name.as_str().starts_with(prefix))
        .map(|(name, transform, computed, _)| {
            let scale = computed.inverse_scale_factor();
            (
                name.as_str().to_string(),
                Rect::from_center_size(transform.translation * scale, computed.size() * scale),
            )
        })
        .collect()
}

/// A named node's rect, or a panic naming what was missing.
#[cfg(feature = "debug")]
fn rect_of(world: &World, name: &str) -> Rect {
    ui_node_rect(world, name).unwrap_or_else(|| panic!("the open panel draws `{name}`"))
}

/// How wide the panel is allowed to be in a window this wide: a share of it,
/// capped.
///
/// The range reads the policy from the same constants the panel is built from,
/// so a change to either lands here rather than in a hand-copied number.
#[cfg(feature = "debug")]
fn policy_width(window_w: f32) -> f32 {
    (SETTINGS_PANEL_WIDTH_PCT / 100.0 * window_w).min(SETTINGS_PANEL_MAX_W)
}

/// `rect` is inside `screen` on every side.
#[cfg(feature = "debug")]
fn assert_inside(what: &str, rect: Rect, screen: Vec2, shape: &str) {
    assert!(
        rect.min.x >= -DRIFT_PX
            && rect.min.y >= -DRIFT_PX
            && rect.max.x <= screen.x + DRIFT_PX
            && rect.max.y <= screen.y + DRIFT_PX,
        "at {shape} `{what}` is drawn at {rect:?}, outside the {screen:?} window: a panel wider \
         than the window it is in does not clip, it overflows, and the Back button goes off the \
         screen with the rows"
    );
}

/// `rect` is inside the window HORIZONTALLY.
///
/// The rows live in a scroll viewport, so a row below the fold legitimately
/// measures outside the window vertically - that is the clip doing its job.
/// The width is the part no scroll can recover.
#[cfg(feature = "debug")]
fn assert_inside_across(what: &str, rect: Rect, screen: Vec2, shape: &str) {
    assert!(
        rect.min.x >= -DRIFT_PX && rect.max.x <= screen.x + DRIFT_PX,
        "at {shape} `{what}` spans {} to {} across a {} wide window: a row that leaves the panel \
         sideways cannot be scrolled back into it",
        rect.min.x,
        rect.max.x,
        screen.x
    );
}

/// The whole panel - frame, tab bar, every keybind row and the way out - is
/// inside the window.
#[cfg(feature = "debug")]
fn assert_the_panel_fits(
    shape: &'static str,
    panel: &'static str,
    back: &'static str,
    slug: &'static str,
) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let screen = viewport(world);
        let frame = rect_of(world, panel);
        assert_inside(panel, frame, screen, shape);
        let wanted = policy_width(screen.x);
        assert!(
            (frame.width() - wanted).abs() <= DRIFT_PX,
            "at {shape} the panel is {} wide in a {} wide window instead of {wanted}: a fixed \
             width in a flex row is not narrower than the window, it is SQUEEZED to it - edge to \
             edge, with no gutter, and with whatever the label columns can still give up",
            frame.width(),
            screen.x
        );
        assert_inside(TAB_BAR, rect_of(world, TAB_BAR), screen, shape);
        // The Back button is the reason the panel has to fit: it is the only
        // way out of the modal, and a player who cannot reach it is stuck in
        // the settings screen with the game paused behind it.
        assert_inside(back, rect_of(world, back), screen, shape);

        let rows = rects_starting_with(world, KEYBIND_ROW);
        assert!(
            rows.len() >= ROWS_EXPECTED,
            "the Controls tab is showing {} keybind row(s), so there is nothing here to contain",
            rows.len()
        );
        for (name, rect) in &rows {
            assert_inside_across(name, *rect, screen, shape);
        }

        nova_probe::probe_marker(
            world,
            slug,
            serde_json::json!({
                "shape": shape,
                "window_w": screen.x,
                "window_h": screen.y,
                "panel_w": frame.width(),
                "panel_h": frame.height(),
                "policy_w": wanted,
                "keybind_rows": rows.len(),
            }),
        );
        info!(
            "pause_settings: at {shape} the panel is inside the window ({} wide, {} rows)",
            frame.width(),
            rows.len()
        );
    }
}

/// At a window the panel could fill, the CAP is what it takes.
///
/// The other half of the policy: a percentage with no maximum makes the
/// settings screen most of a desk monitor wide, and the keybind columns it was
/// sized for drift apart.
#[cfg(feature = "debug")]
fn assert_the_panel_stops_at_its_maximum(world: &mut World) {
    let screen = viewport(world);
    let frame = rect_of(world, PAUSE_SETTINGS_PANEL);
    let share = SETTINGS_PANEL_WIDTH_PCT / 100.0 * screen.x;
    assert!(
        (frame.width() - SETTINGS_PANEL_MAX_W).abs() <= DRIFT_PX,
        "at {screen:?} the panel is {} wide instead of its {SETTINGS_PANEL_MAX_W} maximum: a \
         responsive width with no cap would take {share} of this window",
        frame.width()
    );
    nova_probe::probe_marker(
        world,
        "outcome: the Settings panel stops at its own maximum width",
        serde_json::json!({
            "window_w": screen.x,
            "panel_w": frame.width(),
            "uncapped_w": share,
            "max_w": SETTINGS_PANEL_MAX_W,
        }),
    );
    info!(
        "pause_settings: at {}x{} the panel took its {SETTINGS_PANEL_MAX_W} cap, not {share}",
        screen.x, screen.y
    );
}

/// The window carries the floor the layouts are designed against.
#[cfg(feature = "debug")]
fn assert_the_window_has_a_floor(world: &mut World) {
    let constraints = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .single(world)
        .expect("one primary window")
        .resize_constraints;
    assert!(
        (constraints.min_width - MIN_WINDOW_WIDTH).abs() < f32::EPSILON
            && (constraints.min_height - MIN_WINDOW_HEIGHT).abs() < f32::EPSILON,
        "the window offers no floor ({}x{}): a drag past the widest modal's own width leaves the \
         keybind rows and the Back button off the screen",
        constraints.min_width,
        constraints.min_height
    );
    nova_probe::probe_marker(
        world,
        "outcome: the window keeps a floor under its own layout",
        serde_json::json!({
            "min_width": constraints.min_width,
            "min_height": constraints.min_height,
        }),
    );
    info!(
        "pause_settings: the window will not go below {}x{}",
        constraints.min_width, constraints.min_height
    );
}

/// A fresh install opens borderless, and a player who chose otherwise keeps
/// their choice.
///
/// The store is driven directly rather than through the app's own: a scripted
/// run's store is INERT, which is asserted first, so the only honest way to
/// read the restart contract is to write a file and load it back.
#[cfg(feature = "debug")]
fn assert_the_fresh_install_opens_borderless(world: &mut World) {
    let access = *world.resource::<SettingsStoreAccess>();
    assert_eq!(
        access,
        SettingsStoreAccess::Inert,
        "a scripted run must not read or write the settings of whoever launched it"
    );
    let live = *world.resource::<WindowModeSetting>();
    assert_eq!(
        live,
        WindowModeSetting::Borderless,
        "with nothing saved, the game opens in the mode it defaults to"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a fresh install opens borderless",
        serde_json::json!({ "window_mode": format!("{live:?}") }),
    );
    info!("pause_settings: a fresh install opens borderless");

    // A store of this run's own, so the round trip cannot touch the player's.
    let root = SettingsStoreRoot(Some(
        std::env::temp_dir().join(format!("nova_pause_settings_probe_{}", std::process::id())),
    ));
    let _ = std::fs::remove_dir_all(root.0.as_deref().expect("the probe named its own store"));

    save_settings(
        &root,
        &PersistedSettings {
            window_mode: WindowModeSetting::Windowed,
            ..PersistedSettings::default()
        },
    );
    let reopened = load_settings(&root).expect("the probe's own store reads back");
    assert_eq!(
        reopened.window_mode,
        WindowModeSetting::Windowed,
        "a player who asked for a window keeps it: the new default is for installs that never \
         chose, not a mode change applied to everyone"
    );
    nova_probe::probe_marker(
        world,
        "outcome: an explicitly saved window mode survives a restart",
        serde_json::json!({ "saved": "Windowed", "reopened": format!("{:?}", reopened.window_mode) }),
    );
    info!("pause_settings: an explicit Windowed choice survived the restart");
    let _ = std::fs::remove_dir_all(root.0.as_deref().expect("the probe named its own store"));
}

/// Reshape the window, the way dragging its corner does.
#[cfg(feature = "debug")]
fn set_size(size: Vec2) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
        let mut window = windows.single_mut(world).expect("one primary window");
        window.resolution.set(size.x, size.y);
    }
}

/// Advance once a reshape has reached the pixels the panel is measured in: the
/// window reports the new shape AND the camera the UI is laid out against has
/// picked it up.
///
/// The camera half is the one that matters - `set_size` writes the resolution
/// inside the beat's own frame, and the layout trails it.
#[cfg(feature = "debug")]
fn the_window_reshaped(
    size: Vec2,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    and(
        window_size_is(size.x, size.y),
        std::sync::Arc::new(move |world: &World| {
            world
                .try_query_filtered::<&Camera, With<Camera3d>>()
                .is_some_and(|mut query| {
                    query.iter(world).any(|camera| {
                        camera.logical_viewport_size().is_some_and(|seen| {
                            (seen.x - size.x).abs() < 0.5 && (seen.y - size.y).abs() < 0.5
                        })
                    })
                })
        }),
    )
}

/// The Controls tab is filled: the widest rows the panel has to hold are up.
#[cfg(feature = "debug")]
fn the_keybind_rows_are_up() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query::<(&Name, &InheritedVisibility)>()
            .is_some_and(|mut query| {
                query
                    .iter(world)
                    .filter(|(name, visible)| {
                        visible.get() && name.as_str().starts_with(KEYBIND_ROW)
                    })
                    .count()
                    >= ROWS_EXPECTED
            })
    })
}

/// The walk: flight -> pause -> Settings, read at three shapes, then the
/// window's own floor and mode, then the same panel from the main menu.
#[cfg(feature = "debug")]
fn pause_settings_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("pause_settings: reach the tutorial")
        .until(state_is(GameStates::Playing))
        .deadline(SESSION_SECS)
        .add()
        // NARROW FIRST, so the panel is BUILT at the shape it is read at: a
        // player who plays in a small window never sees it laid out wide.
        .step("pause_settings: narrow the window")
        .on_enter(set_size(NARROW))
        .until(the_window_reshaped(NARROW))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("pause_settings: ESC opens the pause menu")
        .on_enter(press_key(KeyCode::Escape))
        .until(ui_node_present(RESUME_BUTTON))
        .diagnose(ui_node_diagnosis(RESUME_BUTTON))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("pause_settings: let ESC go")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        .click_named(
            "pause_settings: open the pause Settings panel",
            PAUSE_SETTINGS_BUTTON,
            ui_node_present(PAUSE_SETTINGS_BACK),
            BEAT_DEADLINE_SECS,
        )
        // The Controls tab, because its rows are the widest thing the panel
        // holds: a label that takes the leftover width plus two fixed chip
        // columns is what the 620 px was chosen for.
        .click_named(
            "pause_settings: show the Controls tab",
            CONTROLS_TAB,
            the_keybind_rows_are_up(),
            BEAT_DEADLINE_SECS,
        )
        .step("pause_settings: the panel fits a narrow window")
        .on_enter(assert_the_panel_fits(
            "500x600",
            PAUSE_SETTINGS_PANEL,
            PAUSE_SETTINGS_BACK,
            "outcome: the Settings panel fits a narrow window",
        ))
        .add()
        // The native floor. A window is never dragged below this, so it is the
        // worst shape the desk build has to survive.
        .step("pause_settings: out to the native floor")
        .on_enter(set_size(Vec2::new(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT)))
        .until(the_window_reshaped(Vec2::new(
            MIN_WINDOW_WIDTH,
            MIN_WINDOW_HEIGHT,
        )))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("pause_settings: the panel fits the floor")
        .on_enter(assert_the_panel_fits(
            "640x600",
            PAUSE_SETTINGS_PANEL,
            PAUSE_SETTINGS_BACK,
            "outcome: the Settings panel fits the smallest window the game allows",
        ))
        .add()
        .step("pause_settings: back to the stock shape")
        .on_enter(set_size(STOCK))
        .until(the_window_reshaped(STOCK))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("pause_settings: the panel stops at its cap")
        .on_enter(assert_the_panel_stops_at_its_maximum)
        .add()
        .step("pause_settings: the window keeps a floor")
        .on_enter(assert_the_window_has_a_floor)
        .add()
        .step("pause_settings: the mode a fresh install opens in")
        .on_enter(assert_the_fresh_install_opens_borderless)
        .add()
        // Out of the panel before the pause menu can be used again: it is a
        // full-screen blocker, so a click aimed at a button behind it lands on
        // whichever keybind chip happens to be under the pointer.
        .click_named(
            "pause_settings: close the pause Settings panel",
            PAUSE_SETTINGS_BACK,
            nova_protocol::nova_debug::harness::not(ui_node_present(PAUSE_SETTINGS_BACK)),
            BEAT_DEADLINE_SECS,
        )
        // The SAME panel from the other entry point. One layout, two places it
        // is spawned from, and a narrow window is narrow in both.
        .click_named(
            "pause_settings: back to the main menu",
            BACK_TO_MENU_BUTTON,
            state_is(GameStates::MainMenu),
            SESSION_SECS,
        )
        .step("pause_settings: the menu laid out")
        .until(ui_node_present(MENU_SETTINGS_BUTTON))
        .diagnose(ui_node_diagnosis(MENU_SETTINGS_BUTTON))
        .deadline(SESSION_SECS)
        .add()
        .step("pause_settings: narrow the menu")
        .on_enter(set_size(NARROW))
        .until(the_window_reshaped(NARROW))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .click_named(
            "pause_settings: open the menu's Settings panel",
            MENU_SETTINGS_BUTTON,
            ui_node_present(MENU_SETTINGS_BACK),
            BEAT_DEADLINE_SECS,
        )
        .click_named(
            "pause_settings: show the menu's Controls tab",
            CONTROLS_TAB,
            the_keybind_rows_are_up(),
            BEAT_DEADLINE_SECS,
        )
        .step("pause_settings: the menu's panel fits too")
        .on_enter(assert_the_panel_fits(
            "500x600 (main menu)",
            MENU_SETTINGS_PANEL,
            MENU_SETTINGS_BACK,
            "outcome: the menu's Settings panel fits a narrow window",
        ))
        .add()
}
