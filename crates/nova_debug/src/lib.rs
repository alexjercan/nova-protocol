//! `nova_debug` is the debug-only tooling plugin, compiled only under the
//! `debug` feature so it costs nothing in a shipped build. `DebugPlugin` adds
//! the world inspector and dev overlays (gravity, section wireframes); the
//! `harness` module provides the headless-run presets the examples and the
//! `nova_probe` run-harness drive - `nova_autopilot` (scripted play) and
//! `nova_screenshot` (settled-frame capture). Import the presets from the
//! prelude; the raw plugin types stay reachable under `nova_debug::harness::`.

#![warn(missing_docs)]

use bevy::{
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use bevy_common_systems::{
    debug::{harness::AUTOPILOT_ENV, inspector::DebugEnabled as InspectorEnabled},
    prelude::*,
};
use nova_gameplay::{prelude::PlayerSpaceshipMarker, GameStates, PauseStates};
use nova_scenario::prelude::CurrentOutcome;

pub mod gravity;
pub mod harness;
pub mod screenshot;
pub mod sections;

/// Glob-import surface: `use nova_debug::prelude::*` brings the harness presets
/// ([`nova_autopilot`](harness::nova_autopilot),
/// [`nova_screenshot`](harness::nova_screenshot), the reel plugin) and
/// [`DebugPlugin`] into scope; the raw plugin types stay under
/// `nova_debug::harness::` to avoid clashing with Bevy's own `ScreenshotPlugin`.
pub mod prelude {
    // Only the presets are the intended entry point. The raw `AutopilotPlugin` /
    // `ScreenshotPlugin` types stay reachable via `nova_debug::harness::` for the
    // rare bespoke-timeline case, so glob-importing this prelude does not clash
    // with Bevy's own `bevy::render::view::screenshot::ScreenshotPlugin`.
    pub use super::{
        debugdump,
        harness::{
            assert_scenario_loaded, capture_window, hide_dev_overlays, nova_autopilot,
            nova_screenshot, reel_pose_camera, ReelBeat, ReelCamera, ScreenshotReelPlugin,
        },
        screenshot::{ScreenshotHotkeyPlugin, SCREENSHOT_KEYCODE},
        DebugPlugin,
    };
}

/// The keycode to toggle debug mode.
pub const DEBUG_TOGGLE_KEYCODE: KeyCode = KeyCode::F11;

/// Resource with debug toggle state.
#[derive(Resource, Default, Clone, Debug, Deref, DerefMut, PartialEq, Eq, Hash)]
pub struct DebugEnabled(pub bool);

/// [`SystemSet`] gating the debug overlays; [`DebugPlugin`] configures it in
/// `Update` and `PostUpdate` to run only while [`DebugEnabled`] is `true`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DebugSystems;

/// A plugin that adds various debugging tools.
///
/// Adds the world inspector, wireframe/section/gravity overlays and the
/// screenshot hotkey as sub-plugins, inserts [`DebugEnabled`], and runs
/// `toggle_debug_mode` in `Update`; the overlay sub-plugins run under the
/// [`DebugSystems`] set gated on [`DebugEnabled`].
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugEnabled(true));

        app.add_plugins(InspectorDebugPlugin);
        app.add_plugins(WireframeDebugPlugin);
        app.add_plugins(sections::SectionsDebugPlugin);
        app.add_plugins(gravity::GravityDebugPlugin);
        app.add_plugins(screenshot::ScreenshotHotkeyPlugin);

        // The egui inspector panel needs a pointer, so flight now hides the
        // cursor unconditionally (task 20260721-211500). Default it OFF (it
        // ships ON from `InspectorDebugPlugin`) so a dev build flies cursor-free
        // out of the box. F11 raises it: `InspectorDebugPlugin` runs its OWN F11
        // `toggle_debug_mode` over this same resource (bevy_common_systems), so
        // we only override the default here, not the toggle - and while the panel
        // is up `sync_inspector_cursor` hands the cursor back. This runs in
        // lockstep with nova's own overlay `DebugEnabled` (both listen for F11
        // separately), which stays ON - gizmos need no pointer.
        app.insert_resource(InspectorEnabled(false));

        app.add_systems(Update, toggle_debug_mode);
        app.add_systems(
            Update,
            sync_inspector_cursor.run_if(in_state(GameStates::Playing)),
        );

        // Under the headless autopilot (`nova_debug::harness`), confirm the
        // asset loader actually reached gameplay before the clean exit, so a run
        // that silently stalls in `Loading` fails the smoke test instead of
        // passing on `autopilot: cycle complete, no panic` alone.
        if std::env::var(AUTOPILOT_ENV).is_ok() {
            app.add_systems(OnEnter(GameStates::Playing), || {
                info!("nova harness: reached Playing")
            });
        }

        app.configure_sets(
            Update,
            DebugSystems.run_if(resource_equals(DebugEnabled(true))),
        );
        app.configure_sets(
            PostUpdate,
            DebugSystems.run_if(resource_equals(DebugEnabled(true))),
        );
    }
}

fn toggle_debug_mode(mut debug: ResMut<DebugEnabled>, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.just_pressed(DEBUG_TOGGLE_KEYCODE) {
        **debug = !**debug;
    }
}

/// Reconcile the flight cursor with the F11 debug inspector (task
/// 20260721-211500). Flight now hides and locks the cursor unconditionally
/// (nova_editor's `setup_grab_cursor_scenario`, nova_menu's `restore_cursor` /
/// `regrab_cursor_on_player_spawn`), including debug builds, so this is the
/// debug-only counterpart that keeps the inspector usable: while the inspector
/// (an egui panel) is up it owns the pointer, so free the cursor; when it drops,
/// grab it back for flight - unless a menu/pause/outcome surface owns it (those
/// free it through their own transitions and must not be overridden here). Runs
/// only in `GameStates::Playing`; [`Single`] makes it a no-op on headless rigs
/// with no window.
fn sync_inspector_cursor(
    inspector: Res<InspectorEnabled>,
    pause: Res<State<PauseStates>>,
    outcome: Option<Res<CurrentOutcome>>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let mut cursor = cursor.into_inner();
    if **inspector {
        // Inspector up: it owns the pointer. Idempotent guard so we do not
        // trip change detection every frame.
        if !cursor.visible {
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
        }
        return;
    }
    // Inspector down: hand the cursor back to flight, but yield to any surface
    // that legitimately owns it (the pause overlay, a live outcome frame, or the
    // pre-spawn gap with no player ship). Mirrors nova_menu's `restore_cursor` /
    // `regrab_cursor_on_player_spawn` guards - kept a separate check here rather
    // than a shared helper because those live in a crate nova_debug does not
    // depend on and each carries a slightly different guard set; the predicate is
    // simple enough that the drift risk is low.
    if *pause.get() == PauseStates::Paused
        || outcome.is_some_and(|outcome| outcome.0.is_some())
        || q_player.is_empty()
    {
        return;
    }
    if cursor.visible {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

/// Print the `Update` schedule's system graph (via `bevy_mod_debugdump`) for
/// inspecting system ordering; a dev-only diagnostic, not wired into a schedule.
pub fn debugdump(app: &mut App) {
    bevy_mod_debugdump::print_schedule_graph(app, Update);
    // bevy_mod_debugdump::print_schedule_graph(app, PostUpdate);
    // bevy_mod_debugdump::print_schedule_graph(app, FixedUpdate);
}

#[cfg(test)]
mod tests {
    use bevy::state::app::StatesPlugin;

    use super::*;

    /// Build a minimal app around `sync_inspector_cursor`: a primary window with
    /// a visible cursor, the inspector toggle, the pause state, and (optionally)
    /// a live player ship. Mirrors flight the way `DebugPlugin` runs it, minus
    /// the `GameStates::Playing` run gate (asserted by construction here).
    fn app(inspector_on: bool, cursor_visible: bool, with_player: bool) -> App {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<PauseStates>();
        app.insert_resource(InspectorEnabled(inspector_on));
        app.world_mut().spawn((
            PrimaryWindow,
            CursorOptions {
                visible: cursor_visible,
                grab_mode: if cursor_visible {
                    CursorGrabMode::None
                } else {
                    CursorGrabMode::Locked
                },
                ..default()
            },
        ));
        if with_player {
            app.world_mut().spawn(PlayerSpaceshipMarker);
        }
        app.add_systems(Update, sync_inspector_cursor);
        app
    }

    fn cursor(app: &mut App) -> CursorOptions {
        app.world_mut()
            .query_filtered::<&CursorOptions, With<PrimaryWindow>>()
            .single(app.world())
            .unwrap()
            .clone()
    }

    /// The fix: with the inspector down, flying holds the cursor hidden+locked
    /// even in a debug build (before this task the whole grab was compiled out
    /// under `feature = "debug"`).
    #[test]
    fn inspector_off_while_flying_hides_the_cursor() {
        let mut app = app(false, true, true);
        app.update();
        let c = cursor(&mut app);
        assert!(!c.visible);
        assert_eq!(c.grab_mode, CursorGrabMode::Locked);
    }

    /// The inspector (an egui panel) reclaims the cursor while it is up.
    #[test]
    fn inspector_on_frees_the_cursor() {
        let mut app = app(true, false, true);
        app.update();
        let c = cursor(&mut app);
        assert!(c.visible);
        assert_eq!(c.grab_mode, CursorGrabMode::None);
    }

    /// The regrab yields to the pause overlay: with the inspector down but the
    /// game paused, the cursor the pause menu freed must stay free.
    #[test]
    fn inspector_off_yields_to_pause() {
        let mut app = app(false, true, true);
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Paused);
        app.update();
        let c = cursor(&mut app);
        assert!(c.visible, "paused: the freed cursor must not be re-grabbed");
    }

    /// The regrab yields to the pre-spawn gap: no player ship yet means flight
    /// has not started, so leave the cursor as the menu left it.
    #[test]
    fn inspector_off_yields_when_no_player() {
        let mut app = app(false, true, false);
        app.update();
        let c = cursor(&mut app);
        assert!(c.visible, "no player ship: do not grab yet");
    }
}
