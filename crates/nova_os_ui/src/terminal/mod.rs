//! The Tab ship-computer NOVA OS: one inset NOVA OS cockpit monitor that opens
//! on Tab, freezing the sim and freeing the cursor while active. The monitor
//! replaces the old left/right panels with a physical terminal screen: dark
//! casing, hard bezel, green phosphor display and CRT overlays.
//! This module owns the shell, command prompt, scrollback, input handling and
//! terminal content. The existing objectives and combined flight-log data feed
//! read-only terminal commands, but they are no longer visible as permanent
//! monitor panes.
//!
//! # Interaction model
//!
//! Tab opens the NOVA OS by driving the shared [`PauseStates`] axis
//! (`Unpaused -> NovaOs`); once NOVA OS owns the keyboard, Tab completes the
//! terminal prompt instead of closing the monitor. ESC/Start/right-stick close
//! requests are owned here so gameplay stays paused until the close animation
//! reaches zero. The freeze + cursor-free are wired in `nova_menu` on
//! `OnEnter/OnExit(PauseStates::NovaOs)`, reusing the exact hooks the pause
//! overlay uses - the NOVA OS is a THIRD variant of the one freeze axis, not a
//! separate freeze. The NOVA OS is inert while the
//! pause menu owns the freeze (`PauseStates::Paused`), which also means a live
//! outcome overlay - which forces `Paused` - implicitly blocks the NOVA OS
//! without this crate depending on `nova_scenario`'s `CurrentOutcome`.
//!
//! # Animation clock
//!
//! The slide is driven by [`Time<Real>`], never the default `Res<Time>` (=
//! `Time<Virtual>`). Opening the NOVA OS PAUSES virtual time, so a
//! virtual-clocked animation would freeze mid-slide; the slide must keep
//! moving while the sim is frozen.
//!
//! # Module layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `style` | Layout metrics, palette and z-layer constants; font helpers. |
//! | `components` | Markers, [`NovaOsMonitorSettings`] and the shell's resources. |
//! | `crt` | The CRT material and the render-to-texture pointer/hover pipeline. |
//! | `content` | Pure text/row builders for the terminal's read-only commands. |
//! | `input` | Keyboard, gamepad and wheel handling. |
//! | `sound` | The power cues and the ambient bed. |
//! | `shell` | Header/footer/prompt reconcilers, the slide and the app surface. |
//! | `lists` | The flight log model and the objective / log row lists. |
//! | `spawn` | Shell setup and teardown, and the header/main/footer regions. |
//! | `casing` | The physical monitor: casing, bezel, glass and chin controls. |

mod casing;
mod components;
mod content;
mod crt;
mod input;
mod lists;
mod shell;
mod sound;
mod spawn;
mod style;

#[cfg(test)]
mod tests;

use bevy::{prelude::*, ui_render::prelude::UiMaterialPlugin};
use nova_os::prelude::{NovaOsCommandRegistry, NovaOsTerminal};

/// Glob-import surface: `use nova_os_ui::terminal::prelude::*`.
pub mod prelude {
    pub use super::components::NovaOsMonitorSettings;
}

use nova_gameplay::{objectives::prelude::GameObjectives, GameStates, PauseStates};
use nova_hud::prelude::StoryFeed;

pub use self::components::NovaOsMonitorSettings;
use self::{
    components::{NovaOsCloseTransition, NovaOsDegauss, NovaOsFlightLog},
    crt::{animate_nova_os_crt, mirror_nova_os_hover, reconcile_nova_os_target, NovaOsCrtMaterial},
    input::{
        close_nova_os_from_menu_keys, handle_nova_os_app_keyboard, handle_terminal_keyboard,
        scroll_nova_os_panels, sync_nova_os_commands, toggle_nova_os,
    },
    lists::{
        announce_objectives_in_terminal, rebuild_nova_os_flight_log, rebuild_nova_os_objectives,
        sync_nova_os_logs,
    },
    shell::{
        begin_nova_os_boot, blink_nova_os_caret, drain_nova_os_boot, drive_nova_os_power_led,
        drive_nova_os_slide, drive_nova_os_topbar_fps, lift_exempt_chrome_over_nova_os,
        mark_nova_os_events_seen, normalize_nova_os_scroll, nova_os_footer_just_spawned,
        nova_os_header_just_spawned, nova_os_lists_just_spawned, position_nova_os_block_caret,
        rebuild_nova_os_footer_hints, rebuild_terminal_ui, reconcile_nova_os_header,
        sync_nova_os_app_ui, sync_nova_os_monitor_controls, terminal_ui_just_spawned,
    },
    sound::{
        apply_nova_os_bed_volume, play_nova_os_power_down, start_nova_os_sound, stop_nova_os_bed,
    },
    spawn::{remove_nova_os, setup_nova_os},
};
pub(crate) use self::{
    content::{section_kind_from_markers, section_kind_label},
    crt::{forward_nova_os_pointer, NovaOsRtt},
    input::NovaOsAppInput,
    style::{
        nova_os_font, nova_os_text_font, DRAWER_LINE_FONT_PX, NOVA_OS_AMBER, NOVA_OS_PHOSPHOR,
        NOVA_OS_PHOSPHOR_DIM, NOVA_OS_PHOSPHOR_MUTED, NOVA_OS_SCREEN, NOVA_OS_TEXT,
    },
};
/// Re-exported for [`crate::pointer_rig`], the test-only live-tree rig for the
/// forwarded pointer.
#[cfg(test)]
pub(crate) use self::{
    crt::{
        nova_os_image_target, nova_os_new_target_image, nova_os_pointer_id,
        NovaOsForwardedPointerMarker, NovaOsImageContentRootMarker, NovaOsSamplingSurfaceMarker,
        NOVA_OS_RTT_LAYER,
    },
    style::{NOVA_OS_CRT_OVERSCAN, NOVA_OS_CRT_WARP},
};

/// The monitor's frame, in order. Every system this plugin adds names one of
/// these, and the apps that run on the monitor (`map`, `ship`) hang their own
/// sets off them - which is what makes "the app's answer appears on the frame
/// it was typed" a scheduling fact instead of a topological accident.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum NovaOsSystems {
    /// Opening and closing the monitor. Deliberately NOT inside
    /// [`NovaHudSystems`](nova_hud::prelude::NovaHudSystems): the toggle answers a key press wherever in the frame
    /// it lands, and pinning it to the HUD's slot would delay the open.
    Toggle,
    /// Player intent into the terminal model: pointer forwarding, keyboard and
    /// wheel. This is where the pending command invocation is produced.
    Input,
    /// The monitor's own state between input and paint - the slide, the boot
    /// drain, the flight-log lists, sound and the CRT animation.
    Simulate,
    /// Rebuilding the terminal and app UI from the model. Runs after the app
    /// sets, so a `ship repair` result row lands on the frame it was typed
    /// rather than the next one.
    Paint,
}

/// Wires the Tab NOVA OS shell: the toggle, the slide and the objectives section.
/// Registered by [`crate::NovaOsUiPlugin`].
pub(crate) struct NovaOsPlugin;

impl Plugin for NovaOsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NovaOsFlightLog>();
        app.init_resource::<NovaOsTerminal>();
        app.init_resource::<NovaOsCommandRegistry>();
        app.init_resource::<NovaOsCloseTransition>();
        app.init_resource::<NovaOsMonitorSettings>();
        app.init_resource::<NovaOsDegauss>();
        app.register_type::<NovaOsMonitorSettings>();
        app.add_plugins(UiMaterialPlugin::<NovaOsCrtMaterial>::default());

        // Tab opens the NOVA OS. It keeps running in all of Playing so an open
        // NOVA OS can reserve Tab for terminal completion instead of closing.
        app.add_systems(
            Update,
            (toggle_nova_os, close_nova_os_from_menu_keys)
                .chain()
                .run_if(in_state(GameStates::Playing))
                .in_set(NovaOsSystems::Toggle),
        );

        // Shell upkeep while the HUD is live: ease the slide and rebuild the
        // objectives section on change / first spawn.
        app.add_systems(
            Update,
            (
                drive_nova_os_slide,
                (
                    sync_nova_os_logs,
                    rebuild_nova_os_objectives,
                    rebuild_nova_os_flight_log,
                )
                    .chain()
                    .run_if(
                        resource_changed::<GameObjectives>
                            .or_else(resource_changed::<StoryFeed>)
                            .or_else(nova_os_lists_just_spawned),
                    ),
            )
                .in_set(NovaOsSystems::Simulate),
        );
        // Objective flips announce into the live terminal scrollback the moment
        // they happen while the computer is open (PoC `checkObjectives`). Runs
        // after `sync_nova_os_logs` so it sees the freshly appended entries.
        app.add_systems(
            Update,
            announce_objectives_in_terminal
                .after(sync_nova_os_logs)
                .run_if(resource_changed::<GameObjectives>)
                .in_set(NovaOsSystems::Simulate),
        );
        app.add_systems(
            Update,
            scroll_nova_os_panels
                .run_if(in_state(PauseStates::NovaOs))
                .run_if(resource_exists::<Messages<bevy::input::mouse::MouseWheel>>)
                .in_set(NovaOsSystems::Input),
        );
        // Turn a satisfied scroll-to-bottom request back into a real position
        // before anything subtracts a page or a wheel delta from it.
        app.add_systems(
            Update,
            normalize_nova_os_scroll
                .before(scroll_nova_os_panels)
                .before(handle_terminal_keyboard)
                .run_if(in_state(PauseStates::NovaOs))
                .in_set(NovaOsSystems::Input),
        );
        // Producing the invocation. Split from the paint below so the app sets
        // can slot between the two: this is the half that reads the keyboard.
        app.add_systems(
            Update,
            (
                sync_nova_os_commands.run_if(
                    resource_changed::<NovaOsCommandRegistry>
                        .or_else(resource_added::<NovaOsTerminal>),
                ),
                handle_terminal_keyboard.run_if(in_state(GameStates::Playing)),
                handle_nova_os_app_keyboard.run_if(in_state(GameStates::Playing)),
            )
                .chain()
                .in_set(NovaOsSystems::Input),
        );
        // Painting the model. Downstream of the app sets, so scrollback an app
        // handler wrote this frame is on screen this frame.
        app.add_systems(
            Update,
            (
                rebuild_terminal_ui
                    .run_if(resource_changed::<NovaOsTerminal>.or_else(terminal_ui_just_spawned)),
                rebuild_nova_os_footer_hints.run_if(
                    resource_changed::<NovaOsTerminal>.or_else(nova_os_footer_just_spawned),
                ),
                reconcile_nova_os_header.run_if(
                    resource_changed::<NovaOsTerminal>.or_else(nova_os_header_just_spawned),
                ),
                sync_nova_os_app_ui.run_if(in_state(PauseStates::NovaOs)),
            )
                .chain()
                .in_set(NovaOsSystems::Paint),
        );
        // Blink the caret, shimmer the CRT grain and refresh the topbar FPS on
        // real time (virtual time is paused while the computer is open).
        app.add_systems(
            Update,
            (
                blink_nova_os_caret,
                position_nova_os_block_caret,
                drain_nova_os_boot,
                drive_nova_os_topbar_fps,
                animate_nova_os_crt.run_if(resource_exists::<Assets<NovaOsCrtMaterial>>),
                sync_nova_os_monitor_controls.run_if(resource_changed::<NovaOsMonitorSettings>),
                drive_nova_os_power_led,
                // NOVA OS sound: the power-down sweep on a
                // requested close and the live bed volume / SND mute.
                play_nova_os_power_down,
                apply_nova_os_bed_volume,
            )
                .run_if(in_state(PauseStates::NovaOs))
                .in_set(NovaOsSystems::Simulate),
        );

        // Power-up sweep + ambient bed on open, bed teardown on close. Reuses the
        // exact OnEnter/OnExit(NovaOs) hooks the freeze axis uses. The first open
        // also kicks off the staggered boot banner, and each close records how
        // many flight-log events have been seen (for the unread-events count).
        app.add_systems(
            OnEnter(PauseStates::NovaOs),
            (start_nova_os_sound, begin_nova_os_boot),
        );
        app.add_systems(
            OnExit(PauseStates::NovaOs),
            (stop_nova_os_bed, mark_nova_os_events_seen),
        );

        // Render-to-texture pipeline: keep the offscreen image sized to the screen
        // (always, so it is ready when the NOVA OS opens), and while the computer is
        // open forward the pointer onto the image + mirror its hover so the
        // terminal stays interactive through the sampled surface. `mirror` runs
        // before the wheel scroll so its `Hovered` gate reads fresh state.
        app.add_systems(
            Update,
            reconcile_nova_os_target
                .run_if(resource_exists::<NovaOsRtt>)
                .in_set(NovaOsSystems::Simulate),
        );
        app.add_systems(
            Update,
            (forward_nova_os_pointer, mirror_nova_os_hover)
                .chain()
                .before(scroll_nova_os_panels)
                .run_if(in_state(PauseStates::NovaOs))
                .run_if(resource_exists::<NovaOsRtt>)
                .in_set(NovaOsSystems::Input),
        );

        // The HUD chrome that stays visible over the monitor rides this
        // monitor's z-band, so the write lives here.
        app.add_systems(
            Update,
            lift_exempt_chrome_over_nova_os.in_set(NovaOsSystems::Simulate),
        );

        // The NOVA OS is a flight surface: spawn/despawn it with the player ship,
        // like the rest of the HUD.
        app.add_observer(setup_nova_os);
        app.add_observer(remove_nova_os);
    }
}
