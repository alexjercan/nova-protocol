//! The TAB interface and the `:` command modal.
//!
//! TAB opens a themed full-screen interface with a Map pane, a Ship pane, an
//! Inventory pane and a Jobs pane, plus a Station pane while docked, over the
//! frozen world. `:` opens `NOVA COMMANDS`, a
//! command-only modal on the retained CRT monitor, over any surface -
//! including the interface, which it returns to on close. The flight HUD
//! (`nova_hud`) hides while either is open; the two share only the
//! `PauseStates` axis and the `HudInterfaceExempt` tag, so they sit in separate
//! crates.
//!
//! The command language the modal speaks - the catalog, the parser and the
//! terminal model - lives in `nova_command` and has no Bevy UI in it.
//!
//! # Crate layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `bindings` | The named actions the interface and its panes answer to. |
//! | `icons` | The section, map-body, item-category, chart and job-kind icon masks the panes draw. |
//! | `pane` | The TAB interface: open, close, pane switch and layout. |
//! | `terminal` | The command modal: casing, CRT, prompt, input, sound. |
//! | `map` | The Map pane: a schematic 3D view of local space. |
//! | `ship` | The Ship pane: a schematic 3D viewer of the player ship. |
//! | `inventory` | The Inventory pane: the player and docked-ship inventories. |
//! | `jobs` | The Jobs pane: the held jobs and the docked station's board. |
//! | `station` | The Station pane: what the docked station sells. |
//! | `viewer` | The orbit camera and selection cycle both panes run on. |

#![warn(missing_docs)]

pub mod bindings;
mod icons;
pub mod inventory;
pub mod jobs;
pub mod map;
pub mod pane;
pub mod ship;
pub mod station;
pub mod terminal;

/// The 3D viewer the Map and Ship panes are two framings of: the orbit
/// camera's math and feel, the wrapping selection cycle, and the unlit material
/// their proxy meshes share. Crate-internal - it is how the two panes behave,
/// not something a consumer configures - except the pure orbit helpers
/// ([`orbit_eye`](prelude::orbit_eye), [`OrbitGesture`](prelude::OrbitGesture),
/// [`zoom_radius`](prelude::zoom_radius)) a viewer outside the interface frames
/// and moves the same scenes with.
mod viewer;

/// The pure terminal model this crate renders. Re-exported because the two are
/// one API to a consumer: asking what the modal is showing means naming
/// `nova_command` types, and whoever already depends on the renderer should not
/// have to name the model crate a second time.
pub use nova_command;

/// Live-tree rig for the pane pointer and the forwarded CRT pointer, shared by
/// the CRT mapping, card, blip, filter and row tests.
#[cfg(test)]
mod pointer_rig;

/// Glob-import surface: `use nova_interface::prelude::*`.
pub mod prelude {
    pub use super::{
        bindings::interface_bindings,
        icons::{icon_node, init_interface_icons, InterfaceIcons},
        inventory::prelude::*,
        jobs::prelude::*,
        map::prelude::*,
        pane::InterfacePaneType,
        ship::prelude::*,
        station::prelude::*,
        terminal::prelude::*,
        viewer::{orbit_eye, zoom_radius, OrbitGesture},
        InterfacePlugin, InterfaceSystems,
    };
}

use bevy::prelude::*;
use nova_gameplay::{GameStates, PauseStates};
use nova_hud::NovaHudSystems;
use nova_input::prelude::RegisterInputActions;
use nova_ui::{
    input_mode::prelude::{in_input_mode, InputMode},
    theme::UiThemeSystems,
    widget::button_on_setting,
};

use crate::{
    inventory::InventoryPaneSystems,
    jobs::JobsPaneSystems,
    map::MapPaneSystems,
    pane::{
        fit_pane_footer, leave_station_pane, next_interface_pane, play_interface_toggle,
        rebuild_interface_body, refresh_pane_input_hints, show_pane_balance, show_pane_clock,
        show_pane_input_hints, show_station_tab, spawn_interface_root, sync_nova_os_contexts,
        toggle_interface, InterfacePaneType,
    },
    ship::ShipPaneSystems,
    station::StationPaneSystems,
    terminal::CommandsSystems,
};

/// Wires the TAB interface, its four panes, and the `:` command modal.
///
/// Added by the assembly crate (`nova_core`), not by the HUD - the interface
/// and the flight HUD are peers, and the plugin that orders them belongs above
/// both. Added in every assembly: a headless run keeps the bindings, pane
/// lifecycle and commands, and skips only the render targets and CRT material.
pub struct InterfacePlugin;

impl Plugin for InterfacePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InterfacePaneType>();
        app.add_systems(Startup, icons::init_interface_icons);
        app.register_input_actions(bindings::interface_bindings());
        app.add_observer(button_on_setting::<InterfacePaneType>);
        app.add_systems(PreUpdate, sync_nova_os_contexts);

        // The modal's Input/Simulate/Paint sit inside the HUD's slot so it keeps
        // its position relative to the rest of gameplay; its back-out stays
        // outside it (see `CommandsSystems::Toggle`). `nova_console` slots its
        // dispatch between Input and Simulate.
        app.configure_sets(
            Update,
            (
                CommandsSystems::Input,
                CommandsSystems::Simulate,
                CommandsSystems::Paint,
            )
                .chain()
                .in_set(NovaHudSystems),
        );
        app.configure_sets(
            Update,
            CommandsSystems::Toggle.before(CommandsSystems::Input),
        );
        // The interface opens and switches after the back-out owner, and builds
        // its root and body before the panes reconcile their scenes into it.
        app.configure_sets(
            Update,
            InterfaceSystems
                .after(CommandsSystems::Toggle)
                .before(MapPaneSystems)
                .before(ShipPaneSystems)
                .before(InventoryPaneSystems)
                .before(JobsPaneSystems)
                .before(StationPaneSystems),
        );
        // A hull buy clears the Ship pane's selection before the Ship pane
        // reads it (see `StationPaneSystems`).
        app.configure_sets(Update, StationPaneSystems.before(ShipPaneSystems));
        // The widget reconcilers paint after `UiThemeSystems`. A body or pane
        // row spawned after them shows one unpainted frame on each open and
        // pane switch.
        app.configure_sets(
            Update,
            (
                InterfaceSystems,
                MapPaneSystems,
                ShipPaneSystems,
                InventoryPaneSystems,
                JobsPaneSystems,
                StationPaneSystems,
            )
                .before(UiThemeSystems),
        );
        app.add_systems(
            Update,
            (
                // Normal only: TAB typed into a focused field (a pane's
                // quantity, the Nova Debug prompt) is a character, not a close.
                toggle_interface
                    .run_if(in_state(GameStates::Playing))
                    .run_if(in_input_mode(InputMode::Normal)),
                next_interface_pane.run_if(in_input_mode(InputMode::Normal)),
                leave_station_pane,
                spawn_interface_root,
                rebuild_interface_body,
                fit_pane_footer,
                show_station_tab,
                refresh_pane_input_hints,
                show_pane_input_hints,
                show_pane_clock,
                show_pane_balance,
            )
                .chain()
                .in_set(InterfaceSystems),
        );
        app.add_systems(
            OnTransition {
                exited: PauseStates::Unpaused,
                entered: PauseStates::Interface,
            },
            play_interface_toggle,
        );
        app.add_systems(
            OnTransition {
                exited: PauseStates::Interface,
                entered: PauseStates::Unpaused,
            },
            play_interface_toggle,
        );

        app.add_plugins(terminal::CommandsPlugin);
        app.add_plugins(map::MapPanePlugin);
        app.add_plugins(ship::ShipPanePlugin);
        app.add_plugins(inventory::InventoryPanePlugin);
        app.add_plugins(jobs::JobsPanePlugin);
        app.add_plugins(station::StationPanePlugin);
    }
}

/// The TAB interface's own frame: open and close, pane switch, root and body.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InterfaceSystems;
