//! `nova_input` is the bindings registry: the one table that says which named
//! player actions exist, what each is called on screen, and which physical
//! sources it occupies.
//!
//! It is a LEAF on purpose - bevy and `bevy_enhanced_input`, nothing else. The
//! rigs (`nova_ship`, `nova_scenario`) are BUILT FROM it, and every rebind
//! surface (`nova_menu`, `nova_editor`, `nova_os_ui`) reads it, so it has to
//! sit below all of them. A crate that also held the process channel would
//! have to sit above `nova_scenario` too, and `nova_scenario` already depends
//! on `nova_ship`: that is a cycle, which is why the channel is its own crate.
//!
//! The table is a RESOURCE rather than a property of a rig entity because the
//! settings panel renders in the main menu, where no rig exists - the flight
//! rig spawns with the player ship.
//!
//! Defaults are pure data: each owning crate exposes its action list as a
//! plain function, and [`NovaInputPlugin`] is what turns those into the live
//! [`InputBindings`]. Offline consumers - the content lint has no world - call
//! the plain function instead.
//!
//! [`dispatch`] rides here for the same reason: driving an action by name is a
//! lookup in this table plus a press, so it belongs where the table lives.
#![warn(missing_docs)]

pub mod context;
pub mod dispatch;
pub mod poll;
pub mod registry;
pub mod source;

use bevy::prelude::*;

use crate::{context::ActiveContexts, registry::InputBindings};

/// Installs the empty [`InputBindings`] table. Every crate that owns actions
/// registers into it from its own plugin's `build`, so the table is complete
/// before the first frame and stays populated with no ship in the world.
pub struct NovaInputPlugin;

impl Plugin for NovaInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputBindings>();
        app.init_resource::<ActiveContexts>();
    }
}

/// Register actions into [`InputBindings`] from a plugin's `build`.
pub trait RegisterInputActions {
    /// Add these actions to the table, creating it if this runs before
    /// [`NovaInputPlugin`]. Plugin order is not a constraint on purpose: a
    /// rig's actions belong to the rig's own plugin, and a test app that adds
    /// one plugin should get exactly that plugin's actions.
    fn register_input_actions(
        &mut self,
        actions: impl IntoIterator<Item = registry::ActionBinding>,
    ) -> &mut Self;
}

impl RegisterInputActions for App {
    fn register_input_actions(
        &mut self,
        actions: impl IntoIterator<Item = registry::ActionBinding>,
    ) -> &mut Self {
        self.init_resource::<InputBindings>();
        self.init_resource::<ActiveContexts>();
        let mut table = self.world_mut().resource_mut::<InputBindings>();
        for action in actions {
            table.register(action);
        }
        self
    }
}

/// Glob-import surface: `use nova_input::prelude::*` brings the registry, the
/// action record, the physical-source vocabulary and the plugin into scope.
pub mod prelude {
    pub use super::{
        context::{ActionContext, ActiveContexts},
        dispatch,
        dispatch::{DispatchError, InputPhase},
        poll::InputSources,
        registry::{
            ActionAxes, ActionBinding, BindingSpec, GamepadStick, InputBindings, WheelDirection,
        },
        source::{
            binding_label, binding_source, gamepad_label, key_symbol, keyboard_label,
            modifier_pair, InputSource,
        },
        NovaInputPlugin, RegisterInputActions,
    };
}
