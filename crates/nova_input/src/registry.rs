//! [`InputBindings`]: the one table of named player actions, and the source
//! every rig, rebind screen and dispatcher reads.
//!
//! An action's NAME is its stable identity - `radar_hold`, `main_drive`. It is
//! a runtime string, so [`InputBindings::register`] refuses a duplicate loudly
//! and `nova_input`'s consumers pin their own name lists with tests; nothing
//! else in the workspace type-checks them.

use bevy::{
    ecs::spawn::{SpawnIter, SpawnableList},
    platform::collections::HashMap,
    prelude::*,
};
use bevy_enhanced_input::prelude::*;

use crate::source::InputSource;

/// One named action and the discrete sources bound to it.
///
/// Axis bindings - mouse motion, the wheel, gamepad sticks - are NOT here.
/// They carry modifiers that only the rig understands, they cannot be rebound
/// from a settings row, and nothing collides on them. A rig adds those itself,
/// on top of what the registry supplies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionBinding {
    /// The stable identity: `radar_hold`. The channel's key and the settings
    /// row's lookup.
    pub name: &'static str,
    /// The settings row header this action sits under: `FLIGHT`.
    pub group: &'static str,
    /// What a player reads: `Radar Hold`.
    pub label: &'static str,
    /// Keyboard and mouse-button sources, primary first. The first entry is
    /// the one a one-line readout prints.
    pub keyboard: Vec<InputSource>,
    /// Gamepad button sources, primary first.
    pub gamepad: Vec<InputSource>,
}

impl ActionBinding {
    /// A new action with no bindings yet.
    pub fn new(name: &'static str, group: &'static str, label: &'static str) -> Self {
        Self {
            name,
            group,
            label,
            keyboard: Vec::new(),
            gamepad: Vec::new(),
        }
    }

    /// Add keyboard/mouse sources, primary first.
    #[must_use]
    pub fn keyboard(mut self, sources: impl IntoIterator<Item = InputSource>) -> Self {
        self.keyboard.extend(sources);
        self
    }

    /// Add gamepad sources, primary first.
    #[must_use]
    pub fn gamepad(mut self, sources: impl IntoIterator<Item = InputSource>) -> Self {
        self.gamepad.extend(sources);
        self
    }

    /// Every source this action occupies, keyboard before gamepad.
    pub fn sources(&self) -> impl Iterator<Item = InputSource> + '_ {
        self.keyboard.iter().chain(self.gamepad.iter()).copied()
    }
}

/// The live bindings table. Registered at plugin build so it is populated in
/// the main menu, where no rig exists yet - that is the whole reason it is a
/// resource and not a property of the rig entity.
#[derive(Resource, Debug, Default)]
pub struct InputBindings {
    actions: Vec<ActionBinding>,
    index: HashMap<&'static str, usize>,
}

impl InputBindings {
    /// A table seeded with these actions.
    pub fn from_actions(actions: impl IntoIterator<Item = ActionBinding>) -> Self {
        let mut table = Self::default();
        for action in actions {
            table.register(action);
        }
        table
    }

    /// Add an action. A duplicate name REPLACES the entry and logs an error:
    /// two rigs claiming one name is an authoring bug, and silently keeping
    /// either one would leave the settings screen and the dispatcher
    /// disagreeing about what the name means.
    pub fn register(&mut self, action: ActionBinding) {
        if let Some(&existing) = self.index.get(action.name) {
            error!(
                "InputBindings::register: `{}` is already registered (was {:?}); replacing it",
                action.name, self.actions[existing].label
            );
            self.actions[existing] = action;
            return;
        }
        self.index.insert(action.name, self.actions.len());
        self.actions.push(action);
    }

    /// Look one action up by name.
    pub fn get(&self, name: &str) -> Option<&ActionBinding> {
        self.index.get(name).map(|&at| &self.actions[at])
    }

    /// Every action, in registration order.
    pub fn iter(&self) -> impl Iterator<Item = &ActionBinding> {
        self.actions.iter()
    }

    /// Every action name, in registration order.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.actions.iter().map(|action| action.name)
    }

    /// Every discrete source the table occupies, paired with the action that
    /// holds it. This is the conflict set a rebind screen and the content
    /// lint check against.
    pub fn sources(&self) -> impl Iterator<Item = (InputSource, &ActionBinding)> {
        self.actions
            .iter()
            .flat_map(|action| action.sources().map(move |source| (source, action)))
    }

    /// The `Bindings` bundle for one action, for a rig that is built from this
    /// table. An unknown name yields an empty binding list and logs an error -
    /// the action still exists, it simply never fires, which is the failure a
    /// runtime string buys us.
    pub fn bundle(&self, name: &str) -> impl Bundle {
        self.bundle_with(name, ())
    }

    /// [`bundle`](Self::bundle) plus bindings the rig owns itself: the axis
    /// sources, which carry modifiers no settings row can express. Pass the
    /// action's name even when every binding is an axis, so an unregistered
    /// name still fails loudly.
    pub fn bundle_with<L>(&self, name: &str, axes: L) -> impl Bundle
    where
        L: SpawnableList<BindingOf> + Send + Sync + 'static,
    {
        let sources: Vec<Binding> = match self.get(name) {
            Some(action) => action.sources().map(Binding::from).collect(),
            None => {
                error!("InputBindings::bundle: no action named `{name}`; the rig binds nothing");
                Vec::new()
            }
        };
        Bindings::spawn((SpawnIter(sources.into_iter()), axes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn burn() -> ActionBinding {
        ActionBinding::new("main_drive", "FLIGHT", "Main Drive")
            .keyboard([
                InputSource::Keyboard(KeyCode::KeyW),
                InputSource::Keyboard(KeyCode::Space),
            ])
            .gamepad([InputSource::Gamepad(GamepadButton::RightTrigger)])
    }

    #[test]
    fn an_action_is_reachable_by_name() {
        let table = InputBindings::from_actions([burn()]);
        assert_eq!(table.get("main_drive").map(|a| a.label), Some("Main Drive"));
        assert!(table.get("no_such_action").is_none());
    }

    #[test]
    fn sources_pair_every_bound_button_with_its_action() {
        let table = InputBindings::from_actions([burn()]);
        let sources: Vec<_> = table
            .sources()
            .map(|(source, action)| (source, action.name))
            .collect();
        assert_eq!(
            sources,
            vec![
                (InputSource::Keyboard(KeyCode::KeyW), "main_drive"),
                (InputSource::Keyboard(KeyCode::Space), "main_drive"),
                (
                    InputSource::Gamepad(GamepadButton::RightTrigger),
                    "main_drive"
                ),
            ]
        );
    }

    #[test]
    fn registering_a_name_twice_keeps_the_later_action() {
        let mut table = InputBindings::from_actions([burn()]);
        table.register(
            ActionBinding::new("main_drive", "FLIGHT", "Burn")
                .keyboard([InputSource::Keyboard(KeyCode::KeyB)]),
        );
        assert_eq!(table.iter().count(), 1);
        assert_eq!(table.get("main_drive").map(|a| a.label), Some("Burn"));
    }
}
