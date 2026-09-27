//! The TAB interface and the `:` command modal.
//!
//! TAB opens a themed full-screen interface with a Map pane and a Ship pane
//! over the frozen world. `:` opens `NOVA COMMANDS`, a command-only modal on
//! the retained CRT monitor, over any surface - including the interface, which
//! it returns to on close. The flight HUD (`nova_hud`) hides while either is
//! open; the two share only the `PauseStates` axis and the `HudInterfaceExempt`
//! tag, so they sit in separate crates.
//!
//! The command language the modal speaks - the catalog, the parser and the
//! terminal model - lives in `nova_command` and has no Bevy UI in it.
//!
//! # Crate layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `bindings` | The named actions the interface and its panes answer to. |
//! | `icons` | The section and map-body icon masks both panes draw. |
//! | `pane` | The TAB interface: open, close, pane switch and layout. |
//! | `terminal` | The command modal: casing, CRT, prompt, input, sound. |
//! | `map` | The Map pane: a schematic 3D view of local space. |
//! | `ship` | The Ship pane: a schematic 3D viewer of the player ship. |
//! | `viewer` | The orbit camera and selection cycle both panes run on. |

#![warn(missing_docs)]

pub mod bindings;
mod icons;
pub mod map;
pub mod pane;
pub mod ship;
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

/// Live-tree rig for the forwarded CRT pointer, shared by the CRT mapping and
/// the map/ship blip click tests.
#[cfg(test)]
mod pointer_rig;

/// Glob-import surface: `use nova_interface::prelude::*`.
pub mod prelude {
    pub use super::{
        bindings::interface_bindings,
        map::prelude::*,
        pane::InterfacePaneType,
        ship::prelude::*,
        terminal::prelude::*,
        viewer::{orbit_eye, zoom_radius, OrbitGesture},
        InterfacePlugin, InterfaceSystems,
    };
}

use bevy::prelude::*;
use nova_gameplay::GameStates;
use nova_hud::NovaHudSystems;
use nova_input::prelude::RegisterInputActions;
use nova_ui::widget::button_on_setting;

use crate::{
    map::MapPaneSystems,
    pane::{
        next_interface_pane, rebuild_interface_body, spawn_interface_root, sync_nova_os_contexts,
        toggle_interface, InterfacePaneType,
    },
    ship::ShipPaneSystems,
    terminal::CommandsSystems,
};

/// Wires the TAB interface, its two panes, and the `:` command modal.
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
                .before(ShipPaneSystems),
        );
        app.add_systems(
            Update,
            (
                toggle_interface.run_if(in_state(GameStates::Playing)),
                next_interface_pane,
                spawn_interface_root,
                rebuild_interface_body,
            )
                .chain()
                .in_set(InterfaceSystems),
        );

        app.add_plugins(terminal::CommandsPlugin);
        app.add_plugins(map::MapPanePlugin);
        app.add_plugins(ship::ShipPanePlugin);
    }
}

/// The TAB interface's own frame: open and close, pane switch, root and body.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InterfaceSystems;
