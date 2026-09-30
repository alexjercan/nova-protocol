//! The `:` command modal: the retained CRT monitor (casing, bezel, phosphor
//! screen and overlays) titled `NOVA COMMANDS`, over any surface except
//! Loading and an armed rebind capture.
//!
//! This module owns the modal's prompt, scrollback, input handling and chrome.
//! The command language lives in `nova_command`; `nova_console` dispatches.
//!
//! # Interaction model
//!
//! `nova_menu::open_command_shell` opens the modal (`PauseStates::Commands`)
//! and records the state it covered in [`NovaOsCloseTransition::return_to`].
//! Escape, Start or `close` request the animated close; the slide returns to
//! that state when it reaches zero, so gameplay stays frozen throughout. The
//! freeze and cursor hooks live in `nova_menu` on
//! `OnEnter/OnExit(PauseStates::Commands)`.
//!
//! # Animation clock
//!
//! The slide is driven by [`Time<Real>`], never the default `Res<Time>` (=
//! `Time<Virtual>`). Opening the modal PAUSES virtual time, so a
//! virtual-clocked animation would freeze mid-slide.
//!
//! # Module layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `style` | Layout metrics, palette and z-layer constants; font helpers. |
//! | `components` | Markers, [`NovaOsMonitorSettings`] and the modal's resources. |
//! | `crt` | The CRT material and the render-to-texture pointer/hover pipeline. |
//! | `content` | Pure text builders for the modal chrome. |
//! | `input` | Keyboard, gamepad and wheel handling, and the back-out owner. |
//! | `sound` | The power cues and the ambient bed. |
//! | `shell` | Header/footer/prompt reconcilers and the slide. |
//! | `flight_log` | The flight-log model the `log` command prints. |
//! | `spawn` | Modal setup, and the header/main/footer regions. |
//! | `casing` | The physical monitor: casing, bezel, glass and chin controls. |

mod casing;
mod components;
mod content;
mod crt;
mod flight_log;
mod input;
mod shell;
mod sound;
mod spawn;
mod style;

#[cfg(test)]
mod tests;

use bevy::{prelude::*, ui_render::prelude::UiMaterialPlugin};
use nova_command::prelude::CommandTerminal;

/// Glob-import surface: `use nova_interface::terminal::prelude::*`.
pub mod prelude {
    pub use super::{
        components::{
            NovaOsCloseTransition, NovaOsFlightLog, NovaOsFlightLogEntry, NovaOsFlightLogEntryKind,
            NovaOsMonitorSettings,
        },
        crt::{nova_os_openness, nova_os_pointer_id, nova_os_window_px_showing},
        sound::play_nova_os_cue,
        CommandsSystems,
    };
}

use nova_gameplay::{objectives::prelude::GameObjectives, PauseStates};
use nova_hud::prelude::StoryFeed;

pub use self::{
    components::{
        NovaOsCloseTransition, NovaOsFlightLog, NovaOsFlightLogEntry, NovaOsFlightLogEntryKind,
        NovaOsMonitorSettings,
    },
    crt::{nova_os_openness, nova_os_pointer_id, nova_os_window_px_showing},
    sound::play_nova_os_cue,
};
pub(crate) use self::{content::section_kind_from_markers, input::NovaOsAppInput};
use self::{
    crt::{
        animate_nova_os_crt, forward_nova_os_pointer, mirror_nova_os_hover,
        reconcile_nova_os_target, NovaOsCrtMaterial, NovaOsRtt,
    },
    flight_log::{log_combat_lock_drops, sync_nova_os_logs},
    input::{close_surface_from_menu_keys, handle_terminal_keyboard, scroll_nova_os_panels},
    shell::{
        blink_nova_os_caret, drain_nova_os_boot, drive_nova_os_power_led, drive_nova_os_slide,
        drive_nova_os_topbar_fps, lift_exempt_chrome_over_nova_os, normalize_nova_os_scroll,
        nova_os_footer_just_spawned, nova_os_header_just_spawned, position_nova_os_block_caret,
        rebuild_nova_os_footer_hints, rebuild_terminal_ui, reconcile_nova_os_header,
        sync_nova_os_monitor_controls, terminal_ui_just_spawned,
    },
    sound::{
        apply_nova_os_bed_volume, play_nova_os_power_down, start_nova_os_sound, stop_nova_os_bed,
    },
    spawn::{ensure_nova_os_spawned, reset_nova_os_for_new_ship},
};

/// The command modal's frame, in order.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CommandsSystems {
    /// The back-out owner for the modal and the TAB interface. Deliberately NOT
    /// inside [`NovaHudSystems`](nova_hud::prelude::NovaHudSystems): it answers
    /// a key press wherever in the frame it lands.
    Toggle,
    /// Player intent into the terminal model: pointer forwarding, keyboard and
    /// wheel. This is where the pending command invocation is produced.
    Input,
    /// The modal's own state between input and paint - the slide, the boot
    /// drain, the flight-log model, sound and the CRT animation.
    Simulate,
    /// Rebuilding the terminal UI from the model, so a command's rows land on
    /// the frame it was typed.
    Paint,
}

/// Wires the `:` command modal: the back-out, the slide and the terminal.
/// Registered by [`crate::InterfacePlugin`].
pub(crate) struct CommandsPlugin;

impl Plugin for CommandsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NovaOsFlightLog>();
        app.init_resource::<CommandTerminal>();
        app.init_resource::<NovaOsCloseTransition>();
        app.init_resource::<NovaOsMonitorSettings>();
        app.register_type::<NovaOsMonitorSettings>();
        app.add_plugins(UiMaterialPlugin::<NovaOsCrtMaterial>::default());
        // Ungated by `GameStates`: the modal opens over the main menu, and a
        // surface that opened must always be closable.
        app.add_systems(
            Update,
            close_surface_from_menu_keys.in_set(CommandsSystems::Toggle),
        );

        // Shell upkeep while the HUD is live: ease the slide and keep the
        // flight-log model in sync with the story feed and objectives.
        app.add_systems(
            Update,
            (
                drive_nova_os_slide,
                sync_nova_os_logs.run_if(
                    resource_changed::<GameObjectives>.or_else(resource_changed::<StoryFeed>),
                ),
                // Unconditional: the drops arrive while the monitor is SHUT,
                // and an undrained message queue would report them a frame late
                // or not at all.
                log_combat_lock_drops,
            )
                .in_set(CommandsSystems::Simulate),
        );
        app.add_systems(
            Update,
            scroll_nova_os_panels
                .run_if(in_state(PauseStates::Commands))
                .run_if(resource_exists::<Messages<bevy::input::mouse::MouseWheel>>)
                .in_set(CommandsSystems::Input),
        );
        // Turn a satisfied scroll-to-bottom request back into a real position
        // before anything subtracts a page or a wheel delta from it.
        app.add_systems(
            Update,
            normalize_nova_os_scroll
                .before(scroll_nova_os_panels)
                .before(handle_terminal_keyboard)
                .run_if(in_state(PauseStates::Commands))
                .in_set(CommandsSystems::Input),
        );
        // Producing the invocation. Split from the paint below so the console
        // dispatch can slot between the two. Ungated by `GameStates`: the modal
        // owns the screen over the main menu and the editor as well as flight.
        app.add_systems(
            Update,
            handle_terminal_keyboard.in_set(CommandsSystems::Input),
        );
        // Painting the model, after the console dispatch wrote this frame's
        // rows.
        app.add_systems(
            Update,
            (
                rebuild_terminal_ui
                    .run_if(resource_changed::<CommandTerminal>.or_else(terminal_ui_just_spawned)),
                rebuild_nova_os_footer_hints.run_if(
                    resource_changed::<CommandTerminal>.or_else(nova_os_footer_just_spawned),
                ),
                reconcile_nova_os_header.run_if(
                    resource_changed::<CommandTerminal>.or_else(nova_os_header_just_spawned),
                ),
            )
                .chain()
                .in_set(CommandsSystems::Paint),
        );
        // Blink the caret, shimmer the CRT grain and refresh the topbar FPS on
        // real time (virtual time is paused while the monitor is open).
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
                .run_if(in_state(PauseStates::Commands))
                .in_set(CommandsSystems::Simulate),
        );

        // Power-up sweep + ambient bed on open, bed teardown on close. Reuses the
        // exact OnEnter/OnExit(Commands) hooks the freeze axis uses.
        app.add_systems(OnEnter(PauseStates::Commands), start_nova_os_sound);
        app.add_systems(OnExit(PauseStates::Commands), stop_nova_os_bed);

        // Render-to-texture pipeline: keep the offscreen image sized to the screen
        // (always, so it is ready when the NOVA OS opens), and while the monitor is
        // open forward the pointer onto the image + mirror its hover so the
        // terminal stays interactive through the sampled surface. `mirror` runs
        // before the wheel scroll so its `Hovered` gate reads fresh state.
        app.add_systems(
            Update,
            reconcile_nova_os_target
                .run_if(resource_exists::<NovaOsRtt>)
                .in_set(CommandsSystems::Simulate),
        );
        app.add_systems(
            Update,
            (forward_nova_os_pointer, mirror_nova_os_hover)
                .chain()
                .before(scroll_nova_os_panels)
                .run_if(in_state(PauseStates::Commands))
                .run_if(resource_exists::<NovaOsRtt>)
                .in_set(CommandsSystems::Input),
        );

        // The HUD chrome that stays visible over the monitor rides this
        // monitor's z-band, so the write lives here.
        app.add_systems(
            Update,
            lift_exempt_chrome_over_nova_os.in_set(CommandsSystems::Simulate),
        );

        // The monitor is app-global: the modal is reachable from the main menu
        // and the editor, which have no ship to hang a flight surface off. The spawn is idempotent, so this is a keep-alive rather than a
        // one-shot, and the ship-scoped half is reset when the ship goes.
        // Gated on there being no root rather than early-returning inside the
        // system: the run condition is one query, where the body declares the
        // image and CRT-material asset collections it would need to spawn.
        app.add_systems(
            Update,
            ensure_nova_os_spawned
                .run_if(not(any_with_component::<components::NovaOsRootMarker>))
                .in_set(CommandsSystems::Simulate),
        );
        app.add_observer(reset_nova_os_for_new_ship);
    }
}
