//! [`InputBindings`]: the one table of named player actions, and the source
//! every rig, rebind screen and dispatcher reads.
//!
//! An action's NAME is its stable identity - `radar_hold`, `main_drive`. It is
//! a runtime string, so [`InputBindings::register`] refuses a duplicate loudly
//! and `nova_input`'s consumers pin their own name lists with tests; nothing
//! else in the workspace type-checks them.

use std::collections::BTreeMap;

use bevy::{
    ecs::spawn::{SpawnIter, SpawnableList},
    platform::collections::HashMap,
    prelude::*,
};
use bevy_enhanced_input::prelude::*;

use crate::source::{modifier_pair, InputSource};

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
    /// The axis halves: motion, wheel and stick. Not sources - they carry rig
    /// modifiers, nothing collides on them and no rebind row can capture one -
    /// but they ARE how the action is driven, so both the readout and the
    /// dispatcher need them named.
    pub axes: ActionAxes,
}

/// The axis bindings an action carries beside its buttons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActionAxes {
    /// Raw mouse motion drives this action.
    pub mouse_motion: bool,
    /// The wheel drives it, in this direction.
    pub wheel: Option<WheelDirection>,
    /// A gamepad stick drives it.
    pub stick: Option<GamepadStick>,
}

/// Which way the wheel has to turn to drive an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WheelDirection {
    /// Away from the player.
    Up,
    /// Towards the player.
    Down,
}

/// Which stick drives an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamepadStick {
    /// The left stick.
    Left,
    /// The right stick.
    Right,
}

/// What one action is bound to, with nothing else about it: the persisted form
/// of a rebind, and what a rebind screen hands back.
///
/// Nova-owned on purpose. Upstream's `Binding` derives serde only behind a
/// feature this workspace does not enable, and three of its variants (`AnyKey`,
/// `Custom`, `None`) mean nothing in a save file. This is the whole vocabulary
/// a saved keybind needs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BindingSpec {
    /// Keyboard and mouse-button sources, primary first.
    #[cfg_attr(feature = "serde", serde(default))]
    pub keyboard: Vec<InputSource>,
    /// Gamepad button sources, primary first.
    #[cfg_attr(feature = "serde", serde(default))]
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
            axes: ActionAxes::default(),
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

    /// Raw mouse motion drives this action too.
    #[must_use]
    pub fn mouse_motion(mut self) -> Self {
        self.axes.mouse_motion = true;
        self
    }

    /// The wheel drives this action, turned this way.
    #[must_use]
    pub fn wheel(mut self, direction: WheelDirection) -> Self {
        self.axes.wheel = Some(direction);
        self
    }

    /// A gamepad stick drives this action.
    #[must_use]
    pub fn stick(mut self, stick: GamepadStick) -> Self {
        self.axes.stick = Some(stick);
        self
    }

    /// The keyboard column of a settings row: every bound key, then what the
    /// motion or wheel half adds. `Unbound` when the action has neither.
    pub fn keyboard_display(&self) -> String {
        let note = if self.axes.mouse_motion {
            "Mouse"
        } else {
            match self.axes.wheel {
                Some(WheelDirection::Up) => "Scroll Up",
                Some(WheelDirection::Down) => "Scroll Down",
                None => "",
            }
        };
        display_column(&self.keyboard, note)
    }

    /// The gamepad column of a settings row.
    pub fn gamepad_display(&self) -> String {
        let note = match self.axes.stick {
            Some(GamepadStick::Left) => "Left Stick",
            Some(GamepadStick::Right) => "Right Stick",
            None => "",
        };
        display_column(&self.gamepad, note)
    }

    /// What this action is bound to right now, detached from its name and
    /// labels.
    pub fn spec(&self) -> BindingSpec {
        BindingSpec {
            keyboard: self.keyboard.clone(),
            gamepad: self.gamepad.clone(),
        }
    }

    /// Every source this action occupies, keyboard before gamepad.
    pub fn sources(&self) -> impl Iterator<Item = InputSource> + '_ {
        self.keyboard.iter().chain(self.gamepad.iter()).copied()
    }
}

/// One readout column: the sources, then the axis note. A modifier bound on
/// both sides collapses to the bare name - a player who reads `Ctrl` knows
/// both work, and `Left Ctrl / Right Ctrl` says nothing extra.
fn display_column(sources: &[InputSource], note: &'static str) -> String {
    let bound = |key: KeyCode| sources.contains(&InputSource::Keyboard(key));
    let mut parts: Vec<String> = Vec::with_capacity(sources.len() + 1);
    for source in sources {
        let label = match source {
            InputSource::Keyboard(key) => match modifier_pair(*key) {
                Some((bare, other)) if bound(other) => bare.to_string(),
                _ => source.readout_label(),
            },
            _ => source.readout_label(),
        };
        if !parts.contains(&label) {
            parts.push(label);
        }
    }
    if !note.is_empty() {
        parts.push(note.to_string());
    }
    if parts.is_empty() {
        return "Unbound".to_string();
    }
    parts.join(" / ")
}

/// The live bindings table. Registered at plugin build so it is populated in
/// the main menu, where no rig exists yet - that is the whole reason it is a
/// resource and not a property of the rig entity.
#[derive(Resource, Debug, Default)]
pub struct InputBindings {
    actions: Vec<ActionBinding>,
    /// What each action was REGISTERED with, parallel to `actions`. A rebind
    /// writes `actions` alone, so this is what tells a saved store which
    /// actions the player actually changed - and what `reset` restores.
    defaults: Vec<BindingSpec>,
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
            self.defaults[existing] = action.spec();
            self.actions[existing] = action;
            return;
        }
        self.index.insert(action.name, self.actions.len());
        self.defaults.push(action.spec());
        self.actions.push(action);
    }

    /// Bind an action to something else. Unknown names are refused loudly and
    /// return `false`: a store written by a build that had an action this one
    /// does not must not take the whole load down with it.
    pub fn rebind(&mut self, name: &str, spec: BindingSpec) -> bool {
        let Some(&at) = self.index.get(name) else {
            warn!("InputBindings::rebind: no action named `{name}`; ignoring the binding");
            return false;
        };
        self.actions[at].keyboard = spec.keyboard;
        self.actions[at].gamepad = spec.gamepad;
        true
    }

    /// Put one action back on what it was registered with.
    pub fn reset(&mut self, name: &str) -> bool {
        let Some(&at) = self.index.get(name) else {
            return false;
        };
        let default = self.defaults[at].clone();
        self.rebind(name, default)
    }

    /// Every action bound to something other than its default, keyed by name.
    ///
    /// This is what a save file holds: only the rows the player moved, so a
    /// store stays small and a change to a DEFAULT reaches a player who never
    /// touched that row.
    pub fn overrides(&self) -> BTreeMap<String, BindingSpec> {
        self.actions
            .iter()
            .zip(&self.defaults)
            .filter(|(action, default)| action.spec() != **default)
            .map(|(action, _)| (action.name.to_string(), action.spec()))
            .collect()
    }

    /// Apply saved [`overrides`](Self::overrides). Names this build does not
    /// have are skipped with a warning.
    pub fn apply_overrides(&mut self, overrides: &BTreeMap<String, BindingSpec>) {
        for (name, spec) in overrides {
            self.rebind(name, spec.clone());
        }
    }

    /// Look one action up by name.
    pub fn get(&self, name: &str) -> Option<&ActionBinding> {
        self.index.get(name).map(|&at| &self.actions[at])
    }

    /// Every action, in registration order.
    pub fn iter(&self) -> impl Iterator<Item = &ActionBinding> {
        self.actions.iter()
    }

    /// The row headers a settings screen draws, in first-appearance order.
    pub fn groups(&self) -> Vec<&'static str> {
        let mut groups: Vec<&'static str> = Vec::new();
        for action in &self.actions {
            if !groups.contains(&action.group) {
                groups.push(action.group);
            }
        }
        groups
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
    fn a_readout_column_names_the_keys_the_note_and_nothing_else() {
        assert_eq!(burn().keyboard_display(), "W / Space");
        assert_eq!(burn().gamepad_display(), "Right Trigger");

        let radar = ActionBinding::new("radar_hold", "TARGETING", "Radar")
            .keyboard([
                InputSource::Keyboard(KeyCode::ControlLeft),
                InputSource::Keyboard(KeyCode::ControlRight),
            ])
            .gamepad([InputSource::Gamepad(GamepadButton::DPadUp)]);
        assert_eq!(
            radar.keyboard_display(),
            "Ctrl",
            "both halves of one modifier read as the modifier"
        );
        assert_eq!(radar.gamepad_display(), "D-Pad Up");

        let cycle = ActionBinding::new("component_next", "TARGETING", "Next")
            .keyboard([InputSource::Keyboard(KeyCode::BracketRight)])
            .wheel(WheelDirection::Up);
        assert_eq!(cycle.keyboard_display(), "] / Scroll Up");
        assert_eq!(
            cycle.gamepad_display(),
            "Unbound",
            "a column with no source and no note says so"
        );
    }

    #[test]
    fn groups_come_back_in_first_appearance_order() {
        let table = InputBindings::from_actions([
            burn(),
            ActionBinding::new("radar_hold", "TARGETING", "Radar"),
            ActionBinding::new("autopilot_off", "FLIGHT", "Off"),
        ]);
        assert_eq!(table.groups(), vec!["FLIGHT", "TARGETING"]);
    }

    #[test]
    fn only_the_rows_the_player_moved_are_persisted() {
        let mut table = InputBindings::from_actions([
            burn(),
            ActionBinding::new("autopilot_off", "FLIGHT", "Off")
                .keyboard([InputSource::Keyboard(KeyCode::KeyZ)]),
        ]);
        assert!(
            table.overrides().is_empty(),
            "an untouched table saves nothing"
        );

        table.rebind(
            "main_drive",
            BindingSpec {
                keyboard: vec![InputSource::Keyboard(KeyCode::KeyJ)],
                gamepad: vec![],
            },
        );
        let saved = table.overrides();
        assert_eq!(saved.keys().collect::<Vec<_>>(), vec!["main_drive"]);
        assert_eq!(
            table.get("main_drive").map(ActionBinding::keyboard_display),
            Some("J".to_string())
        );

        let mut fresh = InputBindings::from_actions([
            burn(),
            ActionBinding::new("autopilot_off", "FLIGHT", "Off")
                .keyboard([InputSource::Keyboard(KeyCode::KeyZ)]),
        ]);
        fresh.apply_overrides(&saved);
        assert_eq!(fresh.overrides(), saved, "a saved override reloads as one");

        fresh.reset("main_drive");
        assert!(
            fresh.overrides().is_empty(),
            "reset puts the action back on its registered default"
        );
    }

    #[test]
    fn a_saved_binding_for_an_action_this_build_lost_is_skipped() {
        let mut table = InputBindings::from_actions([burn()]);
        let mut saved = BTreeMap::new();
        saved.insert(
            "warp_drive".to_string(),
            BindingSpec {
                keyboard: vec![InputSource::Keyboard(KeyCode::KeyQ)],
                gamepad: vec![],
            },
        );
        table.apply_overrides(&saved);
        assert!(
            table.overrides().is_empty(),
            "an unknown name is dropped, not adopted"
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
