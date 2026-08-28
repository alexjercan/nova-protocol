//! [`InputBindings`]: the one table of named player actions, and the source
//! every rig, rebind screen and dispatcher reads.
//!
//! An action's NAME is its stable identity - `radar_hold`, `main_drive`. It is
//! a runtime string, so [`InputBindings::register`] logs a duplicate loudly and
//! replaces it, and `nova_input`'s consumers pin their own name lists with
//! tests; nothing else in the workspace type-checks them.

use std::collections::BTreeMap;

use bevy::{
    ecs::spawn::{SpawnIter, SpawnableList},
    platform::collections::HashMap,
    prelude::*,
};
use bevy_enhanced_input::prelude::*;

use crate::{
    context::{ActionContext, ActiveContexts},
    source::{modifier_pair, InputSource},
};

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
    /// When this action can fire. Display grouping and firing context are
    /// close but NOT the same axis - `FLIGHT`, `TARGETING` and `CAMERA` are
    /// three headers a player reads apart and one context that is live or not
    /// as a unit - so they stay separate fields.
    pub context: ActionContext,
    /// What a player reads: `Radar Hold`.
    pub label: &'static str,
    /// The action this one shadows: it holds the same sources on purpose and
    /// moves whenever that one is rebound.
    ///
    /// `radar_clear` follows `radar_hold` - one gesture the rig reads two
    /// ways. A follower is not a conflict, gets no settings row of its own,
    /// and cannot be left behind by a rebind.
    pub follows: Option<&'static str>,
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
    /// A gamepad stick drives it. Written by
    /// [`dispatch::apply_stick`](crate::dispatch::apply_stick), not by
    /// `apply_axis`: an action that declares a stick usually declares mouse
    /// motion too, and the two are different devices.
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
            follows: None,
            context: ActionContext::default(),
            keyboard: Vec::new(),
            gamepad: Vec::new(),
            axes: ActionAxes::default(),
        }
    }

    /// Declare this action a shadow of `name`: same sources, moved together,
    /// never a conflict and never its own settings row. Register the action it
    /// follows FIRST - a rebind walks the table in order.
    #[must_use]
    pub fn follows(mut self, name: &'static str) -> Self {
        self.follows = Some(name);
        self
    }

    /// When this action can fire. Declare it beside the action, in the same
    /// list the plugin that adds the reading systems registers.
    #[must_use]
    pub fn context(mut self, context: ActionContext) -> Self {
        self.context = context;
        self
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

    /// The keyboard column of a settings row, chip by chip: every bound key,
    /// then what the motion or wheel half adds. Empty when the action has
    /// neither - the caller decides what "nothing" reads as.
    pub fn keyboard_chips(&self) -> Vec<BindingChip> {
        let note = if self.axes.mouse_motion {
            "Mouse"
        } else {
            match self.axes.wheel {
                Some(WheelDirection::Up) => "Scroll Up",
                Some(WheelDirection::Down) => "Scroll Down",
                None => "",
            }
        };
        display_chips(&self.keyboard, note)
    }

    /// The gamepad column of a settings row, chip by chip.
    pub fn gamepad_chips(&self) -> Vec<BindingChip> {
        let note = match self.axes.stick {
            Some(GamepadStick::Left) => "Left Stick",
            Some(GamepadStick::Right) => "Right Stick",
            None => "",
        };
        display_chips(&self.gamepad, note)
    }

    /// The keyboard column as one string, for a readout with no room to draw
    /// keycaps. `Unbound` when the action holds nothing there.
    pub fn keyboard_display(&self) -> String {
        joined(&self.keyboard_chips())
    }

    /// The gamepad column as one string. See [`Self::keyboard_display`].
    pub fn gamepad_display(&self) -> String {
        joined(&self.gamepad_chips())
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

/// One item of a binding readout: what it READS, and the keycap it DRAWS.
///
/// The two are different vocabularies on purpose. `text` is prose for a player
/// (`Left Ctrl`, `]`, `Right Mouse`); `glyph` is the keycap-table key
/// ([`InputSource::label`]), which a friendlier spelling would silently miss.
/// A surface with room for pictures draws the glyph and falls back to the text
/// where a key has no art; a surface without room prints the text alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingChip {
    /// What this chip reads.
    pub text: String,
    /// The keycap-table key, or `None` for an axis with no single button.
    pub glyph: Option<String>,
}

/// One readout column: the sources, then the axis note. A modifier bound on
/// both sides collapses to the bare name - a player who reads `Ctrl` knows
/// both work, and `Left Ctrl / Right Ctrl` says nothing extra, and the pair
/// shares one keycap anyway.
fn display_chips(sources: &[InputSource], note: &'static str) -> Vec<BindingChip> {
    let bound = |key: KeyCode| sources.contains(&InputSource::Keyboard(key));
    let mut chips: Vec<BindingChip> = Vec::with_capacity(sources.len() + 1);
    for source in sources {
        let text = match source {
            InputSource::Keyboard(key) => match modifier_pair(*key) {
                Some((bare, other)) if bound(other) => bare.to_string(),
                _ => source.readout_label(),
            },
            _ => source.readout_label(),
        };
        let chip = BindingChip {
            text,
            glyph: Some(source.glyph_label()),
        };
        if !chips.iter().any(|held| held.text == chip.text) {
            chips.push(chip);
        }
    }
    if !note.is_empty() {
        // The axis notes are their own keycap keys: the wheel and the mouse
        // body have art, the sticks do not, and a missing one falls back to
        // the text like any unmapped key.
        chips.push(BindingChip {
            text: note.to_string(),
            glyph: Some(note.to_string()),
        });
    }
    chips
}

/// A whole column as one string, for a readout with no room for keycaps.
fn joined(chips: &[BindingChip]) -> String {
    if chips.is_empty() {
        return "Unbound".to_string();
    }
    chips
        .iter()
        .map(|chip| chip.text.as_str())
        .collect::<Vec<_>>()
        .join(" / ")
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

    /// Bind an action to something else. Unknown names and specs the rebind
    /// screen would not have produced are refused loudly and return `false`: a
    /// store written by a build that had an action this one does not, or hand
    /// edited, must not take the whole load down with it.
    pub fn rebind(&mut self, name: &str, spec: BindingSpec) -> bool {
        let Some(&at) = self.index.get(name) else {
            warn!("InputBindings::rebind: no action named `{name}`; ignoring the binding");
            return false;
        };
        if let Some(reason) = self.refuse_spec(at, &spec) {
            warn!("InputBindings::rebind: `{name}` {reason}; ignoring the binding");
            return false;
        }
        let followers: Vec<usize> = self
            .actions
            .iter()
            .enumerate()
            .filter(|(_, action)| action.follows == Some(self.actions[at].name))
            .map(|(at, _)| at)
            .collect();
        for follower in followers {
            self.actions[follower].keyboard = spec.keyboard.clone();
            self.actions[follower].gamepad = spec.gamepad.clone();
        }
        self.actions[at].keyboard = spec.keyboard;
        self.actions[at].gamepad = spec.gamepad;
        true
    }

    /// Why `spec` cannot be what the action at `at` holds, if it cannot.
    ///
    /// Two rules the rebind SCREEN keeps and a stored file did not. A source
    /// belongs to the column of its own device: a pad button among the
    /// keyboard sources loads clean, draws a pad glyph in the desk column, and
    /// is then invisible to the desk poller and pressed by the pad one. And a
    /// column the action ships EMPTY stays empty, because that is the column
    /// the screen draws disabled - `rcs_aim` is mouse motion, and a stored key
    /// would turn its dead chip into a live row for a button no rig reads.
    fn refuse_spec(&self, at: usize, spec: &BindingSpec) -> Option<String> {
        let pad = |source: &&InputSource| matches!(source, InputSource::Gamepad(_));
        if let Some(source) = spec.keyboard.iter().find(pad) {
            return Some(format!(
                "was given the pad button {} in its keyboard column",
                source.readout_label()
            ));
        }
        if let Some(source) = spec.gamepad.iter().find(|source| !pad(source)) {
            return Some(format!(
                "was given {} in its gamepad column",
                source.readout_label()
            ));
        }
        let default = &self.defaults[at];
        if default.keyboard.is_empty() && !spec.keyboard.is_empty() {
            return Some("ships with no keyboard button, so there is none to move".to_string());
        }
        if default.gamepad.is_empty() && !spec.gamepad.is_empty() {
            return Some("ships with no pad button, so there is none to move".to_string());
        }
        None
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
    /// have are skipped with a warning, and a stored row that lands on top of
    /// another action is put back - see [`Self::drop_stored_conflicts`].
    pub fn apply_overrides(&mut self, overrides: &BTreeMap<String, BindingSpec>) {
        for (name, spec) in overrides {
            self.rebind(name, spec.clone());
        }
        self.drop_stored_conflicts(overrides);
    }

    /// Put back any STORED row that collides with the table it landed in.
    ///
    /// A store holds only the rows the player MOVED, deliberately, so a
    /// shipped default that shifts onto a stored key reaches a player who
    /// never touched either row. The rebind screen refuses that collision; a
    /// load has to refuse it too, or both actions fire on one press
    /// (`consume_input: false`) with nothing in the game saying so.
    ///
    /// The STORED row is the one that yields. Resetting the other would be a
    /// no-op - it is already on the default it just moved to - and the whole
    /// table is checked AFTER every override is in, so a legitimate swap (two
    /// rows trading keys) is left alone rather than being refused row by row
    /// against a default the next line was about to clear.
    fn drop_stored_conflicts(&mut self, overrides: &BTreeMap<String, BindingSpec>) {
        // Each pass resets one stored row, and a reset row is never restored,
        // so the number of stored rows bounds the loop.
        for _ in 0..overrides.len() {
            let collision = self
                .conflicts()
                .into_iter()
                .find_map(|(one, other, source)| {
                    if overrides.contains_key(one.name) {
                        Some((one.name, other.name, source))
                    } else if overrides.contains_key(other.name) {
                        Some((other.name, one.name, source))
                    } else {
                        None
                    }
                });
            let Some((stored, holder, source)) = collision else {
                return;
            };
            warn!(
                "InputBindings::apply_overrides: stored `{stored}` on {} collides with `{holder}`; restoring its default",
                source.readout_label()
            );
            self.reset(stored);
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

    /// Every action that can fire right now, in registration order.
    ///
    /// This is what a driver may press and what a snapshot advertises. The
    /// whole table is the wrong answer to that question: an action resolves to
    /// a key, and three actions hold `G`, so a list of all of them says the
    /// same key means three things at once.
    pub fn live<'a>(
        &'a self,
        active: &'a ActiveContexts,
    ) -> impl Iterator<Item = &'a ActionBinding> {
        self.actions
            .iter()
            .filter(|action| active.is_live(action.context))
    }

    /// The distinct contexts the table declares, in first-appearance order.
    /// A sync system reads this instead of naming the apps itself, so adding a
    /// NOVA OS app does not need a second edit somewhere else.
    pub fn contexts(&self) -> Vec<ActionContext> {
        let mut contexts: Vec<ActionContext> = Vec::new();
        for action in &self.actions {
            if !contexts.contains(&action.context) {
                contexts.push(action.context);
            }
        }
        contexts
    }

    /// Every pair of actions that share a physical source AND can be live at
    /// the same instant, with the source they collide on.
    ///
    /// Sharing a key ACROSS contexts is the normal case and not a conflict:
    /// `G` is go-to in flight and the map's GOTO in the map viewer, and one of
    /// them is always the only one listening. Sharing one WITHIN a live set is
    /// the bug - both rigs run with `consume_input: false`, so one press
    /// drives both actions.
    ///
    /// A [`follows`](ActionBinding::follows) pair is not a conflict: it shares
    /// the source on purpose and is rebound as a unit.
    pub fn conflicts(&self) -> Vec<(&ActionBinding, &ActionBinding, InputSource)> {
        let mut found = Vec::new();
        for (at, action) in self.actions.iter().enumerate() {
            for other in &self.actions[at + 1..] {
                if !action.context.overlaps(other.context) || shadows(action, other) {
                    continue;
                }
                for source in action.sources() {
                    if other.sources().any(|held| held == source) {
                        found.push((action, other, source));
                    }
                }
            }
        }
        found
    }

    /// What already holds `source` and could be listening at the same instant
    /// as `name` - the reason a rebind row refuses a capture.
    ///
    /// A key `name` already holds is not a conflict with itself, and neither
    /// is the action it shadows: rebinding `radar_hold` onto its own key is a
    /// no-op, not a collision with `radar_clear`.
    pub fn conflict_for(&self, name: &str, source: InputSource) -> Option<&ActionBinding> {
        let action = self.get(name)?;
        self.actions.iter().find(|other| {
            other.name != name
                && !shadows(action, other)
                && action.context.overlaps(other.context)
                && other.sources().any(|held| held == source)
        })
    }

    /// Which action holds `source` at a rung `context` reaches - the question a
    /// surface that binds something OUTSIDE the registry asks. A ship section's
    /// own trigger key is not an action, so it cannot be checked with
    /// [`Self::conflict_for`], but it still fires beside whatever the flight
    /// rig holds.
    ///
    /// Reads the LIVE table, so a source freed by a rebind stops being
    /// reported and a source a rebind took starts.
    pub fn holder_in(&self, context: ActionContext, source: InputSource) -> Option<&ActionBinding> {
        self.actions.iter().find(|action| {
            action.context.overlaps(context) && action.sources().any(|held| held == source)
        })
    }

    /// The actions a settings screen draws a row for: everything but the
    /// shadows, which move with the action they follow.
    pub fn rows(&self) -> impl Iterator<Item = &ActionBinding> {
        self.actions
            .iter()
            .filter(|action| action.follows.is_none())
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

/// Whether one of these two actions shadows the other. Not a method on
/// [`ActionBinding`] because it is symmetric and neither side owns it.
fn shadows(one: &ActionBinding, other: &ActionBinding) -> bool {
    one.follows == Some(other.name) || other.follows == Some(one.name)
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

    /// The whole point of the context field: three actions hold `G`, and a
    /// list that named all three would tell a driver one key means three
    /// things at once. Only the live ones come back.
    #[test]
    fn only_the_actions_whose_context_is_raised_are_live() {
        let table = InputBindings::from_actions([
            burn(),
            ActionBinding::new("autopilot_goto", "FLIGHT", "Go To")
                .context(ActionContext::Flight)
                .keyboard([InputSource::Keyboard(KeyCode::KeyG)]),
            ActionBinding::new("map_goto", "MAP", "Set GOTO")
                .context(ActionContext::ViewerApp("map"))
                .keyboard([InputSource::Keyboard(KeyCode::KeyG)]),
        ]);

        let mut active = ActiveContexts::default();
        let names = |active: &ActiveContexts| -> Vec<&'static str> {
            table.live(active).map(|action| action.name).collect()
        };

        assert_eq!(
            names(&active),
            vec!["main_drive"],
            "an undeclared action defaults to Always and is live with nothing raised"
        );

        active.set(ActionContext::Flight, true);
        assert_eq!(names(&active), vec!["main_drive", "autopilot_goto"]);

        active.set(ActionContext::Flight, false);
        active.set(ActionContext::ViewerApp("map"), true);
        assert_eq!(names(&active), vec!["main_drive", "map_goto"]);
    }

    #[test]
    fn the_declared_contexts_come_back_in_first_appearance_order() {
        let table = InputBindings::from_actions([
            ActionBinding::new("novaos_toggle", "SYSTEM", "NOVA OS"),
            ActionBinding::new("map_goto", "MAP", "Set GOTO")
                .context(ActionContext::ViewerApp("map")),
            ActionBinding::new("novaos_next", "NOVA OS", "Next").context(ActionContext::Viewer),
            ActionBinding::new("ship_mates", "SHIP", "Mates")
                .context(ActionContext::ViewerApp("ship")),
        ]);
        assert_eq!(
            table.contexts(),
            vec![
                ActionContext::Always,
                ActionContext::ViewerApp("map"),
                ActionContext::Viewer,
                ActionContext::ViewerApp("ship"),
            ]
        );
    }

    /// One key held by two actions is a conflict only when both can hear it.
    #[test]
    fn a_shared_source_is_a_conflict_only_inside_one_live_set() {
        let across = InputBindings::from_actions([
            ActionBinding::new("autopilot_goto", "FLIGHT", "Go To")
                .context(ActionContext::Flight)
                .keyboard([InputSource::Keyboard(KeyCode::KeyG)]),
            ActionBinding::new("map_goto", "MAP", "Set GOTO")
                .context(ActionContext::ViewerApp("map"))
                .keyboard([InputSource::Keyboard(KeyCode::KeyG)]),
        ]);
        assert!(
            across.conflicts().is_empty(),
            "flight and the map viewer never listen at the same instant"
        );

        let within = InputBindings::from_actions([
            ActionBinding::new("novaos_next", "NOVA OS", "Next")
                .context(ActionContext::Viewer)
                .keyboard([InputSource::Keyboard(KeyCode::BracketRight)]),
            ActionBinding::new("map_next", "MAP", "Next")
                .context(ActionContext::ViewerApp("map"))
                .keyboard([InputSource::Keyboard(KeyCode::BracketRight)]),
        ]);
        let found: Vec<_> = within
            .conflicts()
            .into_iter()
            .map(|(one, other, source)| (one.name, other.name, source))
            .collect();
        assert_eq!(
            found,
            vec![(
                "novaos_next",
                "map_next",
                InputSource::Keyboard(KeyCode::BracketRight)
            )],
            "a named app runs INSIDE the shared viewer set, so both hear the key"
        );
    }

    /// The store keeps only the rows a player MOVED, so a shipped default that
    /// shifts onto a stored key reaches a player who never touched either row.
    /// The rebind screen refuses that collision; the load has to as well.
    #[test]
    fn a_stored_row_that_a_moved_default_landed_on_is_put_back() {
        // The player moved `fire` to J. The next build moves `burn`'s DEFAULT
        // to J as well - a row they never touched, so the store says nothing
        // about it.
        let mut table = InputBindings::from_actions([
            ActionBinding::new("burn", "FLIGHT", "Burn")
                .context(ActionContext::Flight)
                .keyboard([InputSource::Keyboard(KeyCode::KeyJ)]),
            ActionBinding::new("fire", "FLIGHT", "Fire")
                .context(ActionContext::Flight)
                .keyboard([InputSource::Keyboard(KeyCode::KeyF)]),
        ]);
        let mut store = BTreeMap::new();
        store.insert(
            "fire".to_string(),
            BindingSpec {
                keyboard: vec![InputSource::Keyboard(KeyCode::KeyJ)],
                gamepad: vec![],
            },
        );

        table.apply_overrides(&store);

        assert!(
            table.conflicts().is_empty(),
            "one press must not drive both"
        );
        assert_eq!(
            table.get("fire").expect("registered").keyboard,
            vec![InputSource::Keyboard(KeyCode::KeyF)],
            "the STORED row yields - resetting the other would put it right back"
        );
        assert_eq!(
            table.get("burn").expect("registered").keyboard,
            vec![InputSource::Keyboard(KeyCode::KeyJ)],
            "and the shipped default this build chose is kept"
        );
    }

    /// The check runs on the WHOLE table once every row is in, not row by row:
    /// two rows trading keys are each stored on the other's default, and a
    /// per-row refusal would reject the first against a default the second
    /// line was about to clear.
    #[test]
    fn two_rows_that_traded_keys_both_load() {
        let mut table = InputBindings::from_actions([
            ActionBinding::new("burn", "FLIGHT", "Burn")
                .context(ActionContext::Flight)
                .keyboard([InputSource::Keyboard(KeyCode::KeyW)]),
            ActionBinding::new("fire", "FLIGHT", "Fire")
                .context(ActionContext::Flight)
                .keyboard([InputSource::Keyboard(KeyCode::KeyF)]),
        ]);
        let spec = |key: KeyCode| BindingSpec {
            keyboard: vec![InputSource::Keyboard(key)],
            gamepad: vec![],
        };
        let store = BTreeMap::from([
            ("burn".to_string(), spec(KeyCode::KeyF)),
            ("fire".to_string(), spec(KeyCode::KeyW)),
        ]);

        table.apply_overrides(&store);

        assert_eq!(
            table.get("burn").expect("registered").keyboard,
            vec![InputSource::Keyboard(KeyCode::KeyF)]
        );
        assert_eq!(
            table.get("fire").expect("registered").keyboard,
            vec![InputSource::Keyboard(KeyCode::KeyW)],
            "the swap survives the load"
        );
    }

    /// A hand-edited or stale store can hold a spec the rebind screen could
    /// never have produced. The table keeps the screen's own two rules, so
    /// what loads is what a player could have bound.
    #[test]
    fn a_spec_the_rebind_screen_could_not_produce_is_refused() {
        let mut table = InputBindings::from_actions([
            ActionBinding::new("burn", "FLIGHT", "Burn")
                .context(ActionContext::Flight)
                .keyboard([InputSource::Keyboard(KeyCode::KeyW)])
                .gamepad([InputSource::Gamepad(GamepadButton::South)]),
            ActionBinding::new("rcs_aim", "FLIGHT", "RCS Aim")
                .context(ActionContext::Flight)
                .mouse_motion(),
        ]);

        assert!(
            !table.rebind(
                "burn",
                BindingSpec {
                    keyboard: vec![InputSource::Gamepad(GamepadButton::North)],
                    gamepad: vec![],
                },
            ),
            "a pad button in the keyboard column would draw in the desk column \
             and be pressed by the pad poller"
        );
        assert!(
            !table.rebind(
                "burn",
                BindingSpec {
                    keyboard: vec![],
                    gamepad: vec![InputSource::Keyboard(KeyCode::KeyD)],
                },
            ),
            "and a key in the gamepad column is the same crossing the other way"
        );
        assert_eq!(
            table.get("burn").expect("registered").spec(),
            BindingSpec {
                keyboard: vec![InputSource::Keyboard(KeyCode::KeyW)],
                gamepad: vec![InputSource::Gamepad(GamepadButton::South)],
            },
            "a refused rebind leaves the row exactly as it was"
        );

        assert!(
            !table.rebind(
                "rcs_aim",
                BindingSpec {
                    keyboard: vec![InputSource::Keyboard(KeyCode::KeyR)],
                    gamepad: vec![],
                },
            ),
            "an axis action ships no button, its chip is drawn disabled, and a \
             store must not turn that into a live row for a key no rig reads"
        );
        assert!(table
            .get("rcs_aim")
            .expect("registered")
            .keyboard
            .is_empty());
    }

    /// One gesture the rig reads two ways is ONE thing to a player: it shares
    /// its key on purpose, it gets one settings row, and a rebind that left
    /// half of it behind would break the gesture silently.
    #[test]
    fn a_shadow_shares_a_key_gets_no_row_and_moves_with_what_it_follows() {
        let mut table = InputBindings::from_actions([
            ActionBinding::new("radar_hold", "TARGETING", "Radar")
                .context(ActionContext::Flight)
                .keyboard([InputSource::Keyboard(KeyCode::ControlLeft)]),
            ActionBinding::new("radar_clear", "TARGETING", "Radar (tap clear)")
                .context(ActionContext::Flight)
                .follows("radar_hold")
                .keyboard([InputSource::Keyboard(KeyCode::ControlLeft)]),
        ]);
        assert!(
            table.conflicts().is_empty(),
            "a shadow holding the same key is the point, not a collision"
        );
        assert_eq!(
            table.rows().map(|action| action.name).collect::<Vec<_>>(),
            vec!["radar_hold"],
            "the shadow has no settings row of its own"
        );

        table.rebind(
            "radar_hold",
            BindingSpec {
                keyboard: vec![InputSource::Keyboard(KeyCode::KeyK)],
                gamepad: vec![],
            },
        );
        assert_eq!(
            table.get("radar_clear").expect("registered").keyboard,
            vec![InputSource::Keyboard(KeyCode::KeyK)],
            "the shadow went with it"
        );
    }

    /// What a rebind row refuses on, and what it must not refuse on: its own
    /// current key, its shadow, and an action that can never be up beside it.
    #[test]
    fn a_capture_is_refused_only_by_something_that_could_answer_beside_it() {
        let table = InputBindings::from_actions([
            burn().context(ActionContext::Flight),
            ActionBinding::new("radar_clear", "TARGETING", "Radar (tap clear)")
                .context(ActionContext::Flight)
                .follows("main_drive")
                .keyboard([InputSource::Keyboard(KeyCode::KeyW)]),
            ActionBinding::new("autopilot_goto", "FLIGHT", "Go To")
                .context(ActionContext::Flight)
                .keyboard([InputSource::Keyboard(KeyCode::KeyG)]),
            ActionBinding::new("map_goto", "MAP", "Set GOTO")
                .context(ActionContext::ViewerApp("map"))
                .keyboard([InputSource::Keyboard(KeyCode::KeyM)]),
        ]);
        assert_eq!(
            table
                .conflict_for("main_drive", InputSource::Keyboard(KeyCode::KeyG))
                .map(|action| action.name),
            Some("autopilot_goto"),
            "flight is one live set; the key is taken"
        );
        assert!(
            table
                .conflict_for("main_drive", InputSource::Keyboard(KeyCode::KeyM))
                .is_none(),
            "the map viewer is never up beside flight"
        );
        assert!(
            table
                .conflict_for("main_drive", InputSource::Keyboard(KeyCode::KeyW))
                .is_none(),
            "its own key, and its own shadow, are not a conflict"
        );
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

    /// A chip says two different things at once: what a player READS, and
    /// which keycap the picture comes from. A surface that drew from `text`
    /// would look `Ctrl` up in a table keyed on `ControlLeft` and quietly find
    /// nothing.
    #[test]
    fn a_chip_reads_as_prose_and_draws_from_the_source_label() {
        let radar = ActionBinding::new("radar_hold", "TARGETING", "Radar").keyboard([
            InputSource::Keyboard(KeyCode::ControlLeft),
            InputSource::Keyboard(KeyCode::ControlRight),
        ]);
        let chips = radar.keyboard_chips();
        assert_eq!(chips.len(), 1, "both halves of one modifier are one chip");
        assert_eq!(chips[0].text, "Ctrl");
        assert_eq!(
            chips[0].glyph.as_deref(),
            Some("ControlLeft"),
            "the picture comes from the source, not from the prose"
        );

        let pad = ActionBinding::new("radar_hold", "TARGETING", "Radar")
            .gamepad([InputSource::Gamepad(GamepadButton::South)]);
        let chips = pad.gamepad_chips();
        assert_eq!(chips[0].text, "A", "the pad face button reads as the shell");
        assert_eq!(
            chips[0].glyph.as_deref(),
            Some("Pad A"),
            "and draws from a key the keyboard's own A cannot claim"
        );

        let cycle = ActionBinding::new("component_next", "TARGETING", "Next")
            .keyboard([InputSource::Keyboard(KeyCode::BracketRight)])
            .wheel(WheelDirection::Up);
        let chips = cycle.keyboard_chips();
        assert_eq!(chips[0].text, "]");
        assert_eq!(chips[0].glyph.as_deref(), Some("BracketRight"));
        assert_eq!(
            chips[1].glyph.as_deref(),
            Some("Scroll Up"),
            "an axis note is its own keycap key"
        );

        assert!(
            cycle.gamepad_chips().is_empty(),
            "an empty column is empty chips; only the joined readout says Unbound"
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
