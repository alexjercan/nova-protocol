//! The document's SCRIPT: a handler is a node, and so is every filter, action
//! and beat inside it.
//!
//! The layout half of the document has been nodes since it existed - a ship is
//! a subtree, a rock is a node beside it. The script was not: it was a constant
//! in [`crate::scenario`], written out on save and stepped over on load, and
//! "the editor does not edit the script" was the honest way to say so. This
//! module is what changes that.
//!
//! THE TREE IS THE CONFIG. A [`ScenarioEventConfig`] is a handler with two
//! lists inside it, and a `Sequence` action has a list of steps each holding
//! another list of actions. Held as one component per handler, the panel would
//! be a wall of rows and a nested action would have no row at all. Held as
//! NODES - the filters and actions children of their handler, the steps
//! children of their sequence - each one is a thing to select, inspect and
//! delete, exactly like a rock. [`lower`] walks the tree back into the nested
//! lists; [`lift`] takes them apart again.
//!
//! WHAT A NODE HOLDS IS WHAT NOTHING ELSE HOLDS. An [`ActionNode`] for a
//! `Sequence` keeps the key and NOT the steps, because the steps are its
//! children; a [`FilterNode`] for `And` keeps no config at all, because both
//! operands are. A node that kept a copy of what its children hold would be a
//! second answer to the same question, and a save would have to pick one.

use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    reflect::{ReflectRef, TypeInfo},
    ui_widgets::Activate,
};
use nova_events::units::prelude::*;
use nova_gameplay::prelude::{Allegiance, AssetRef};
use nova_scenario::prelude::*;
use nova_ship::prelude::FlightVerb;

use crate::{
    config::SelectedNode,
    node::{id_order, mint_id, split_ordinal, EditContext, EditorNode, NextChildOrdinal, NodeId},
    scenario::DEFAULT_SKY,
};

/// One authored handler: the event it listens for, and whether it retires.
///
/// Its filters and actions are its children.
#[derive(Component, Debug, Clone, Reflect)]
pub(crate) struct EventNode {
    /// What this handler is FOR, in the author's own words.
    ///
    /// The trigger is not a name. Six handlers on `OnEnter` read as six copies
    /// of one row, and this is the line that tells them apart.
    pub(crate) label: Option<String>,
    /// The event this handler reacts to.
    pub(crate) trigger: EventConfig,
    /// Retire the handler the first time its filters pass.
    pub(crate) once: bool,
}

impl Default for EventNode {
    fn default() -> Self {
        Self {
            label: None,
            trigger: EventConfig::OnStart,
            once: false,
        }
    }
}

/// One filter of a handler or of a gate.
#[derive(Component, Debug, Clone)]
pub(crate) struct FilterNode {
    /// Which filter it is, and that filter's own config.
    pub(crate) kind: FilterKind,
}

/// What a [`FilterNode`] is.
///
/// The two leaf arms carry their config; the three COMBINATOR arms carry
/// nothing, because what they combine are child filter nodes, and `Expression`
/// carries nothing because its comparison is a child too. That is the same
/// split `Sequence` gets one level up, and for the same reason: an operand
/// inside the node would be an operand the tree cannot show.
#[derive(Debug, Clone)]
pub(crate) enum FilterKind {
    /// Match the event's entities by id or type name.
    Entity(EntityFilterConfig),
    /// Compare scenario variables. Carries nothing: the comparison is its
    /// child expression node, the same way a combinator's operands are its
    /// child filters.
    Expression,
    /// Match a timer event by key.
    Timer(TimerFilterConfig),
    /// Invert the one filter inside it.
    Not,
    /// Pass when both filters inside it pass.
    And,
    /// Pass when either filter inside it passes.
    Or,
}

/// One node of an expression: an operator whose operands are its children, or
/// a leaf that is typed.
///
/// THE OPERATOR IS THE NODE. Read as one line, `picket_warden_awake == false`
/// hides its own shape - the `==` is somewhere in the middle and the two things
/// it compares are found by reading outwards from it. As nodes the operator is
/// the row you land on and its operands hang under it, which is the shape the
/// comparison has.
///
/// A LEAF IS STILL TEXT. `4`, `"act_two"`, `beat`, `entity("courier").speed`
/// are one row each either way, and a tree that spent a node on every literal
/// would bury the operators this exists to show. The leaf holds a whole
/// expression, so anything the grammar can hold can still be typed into one
/// row - see [`crate::event::ExprChoice::Value`].
#[derive(Component, Debug, Clone)]
pub(crate) struct ExpressionNode {
    /// Which operator it is, or the value it holds.
    pub(crate) kind: ExprKind,
}

/// What an [`ExpressionNode`] is.
///
/// The comparisons are the root of a CONDITION and the rest are the root of a
/// VALUE, which is the same split the grammar has: a filter compares, and what
/// it compares are expressions.
#[derive(Debug, Clone)]
pub(crate) enum ExprKind {
    /// `left == right`.
    Equal,
    /// `left < right`.
    LessThan,
    /// `left > right`.
    GreaterThan,
    /// `left + right`.
    Add,
    /// `left - right`.
    Subtract,
    /// `left * right`.
    Multiply,
    /// `left / right`.
    Divide,
    /// A typed operand: a literal, a variable, a query, or any expression that
    /// fits one row.
    Value(ValueOperand),
}

/// A leaf operand, as the one row it fits on.
///
/// The field is the authored expression itself, which reflects as an opaque
/// leaf and therefore edits as TEXT through `syntax.rs` - the same box, the
/// same parse and the same refusal a condition had before it was a tree.
#[derive(Debug, Clone, Reflect)]
pub(crate) struct ValueOperand {
    /// The operand.
    pub(crate) value: VariableExpressionNode,
}

/// One action of a handler or of a sequence step.
#[derive(Component, Debug, Clone)]
pub(crate) struct ActionNode {
    /// Which action it is, and that action's own config.
    pub(crate) kind: ActionKind,
}

/// What an [`ActionNode`] is: an action the config holds whole, or the head of
/// one whose innards are its children.
///
/// The two headed arms are the two actions that CONTAIN something the tree can
/// show - a chain of beats, and an expression - and each keeps the fields left
/// over once that something is a child. See the module note.
#[derive(Debug, Clone)]
#[expect(
    clippy::large_enum_variant,
    reason = "the leaf IS the whole action config. Boxing it buys a few hundred bytes on a tree of at most a few hundred nodes, and costs every nested pattern over it a manual deref."
)]
pub(crate) enum ActionKind {
    /// An action the config holds whole.
    Leaf(EventActionConfig),
    /// A `Sequence`: the key lives here, the steps are children.
    Sequence(SequenceHead),
    /// A `VariableSet`: the key lives here, the value is a child expression.
    VariableSet(VariableSetHead),
}

/// A sequence's own fields, minus the steps.
#[derive(Component, Debug, Clone, Default, Reflect)]
pub(crate) struct SequenceHead {
    /// Scenario-local key the engine files the cursor under.
    pub(crate) key: String,
}

/// A variable set's own field, minus the expression.
#[derive(Component, Debug, Clone, Default, Reflect)]
pub(crate) struct VariableSetHead {
    /// The scenario variable this action writes.
    #[reflect(@Names::Variable)]
    pub(crate) key: String,
}

/// One step of a sequence: what it waits for by the clock, and how long it may
/// wait. Its gate and its actions are children.
#[derive(Component, Debug, Clone, Default, Reflect)]
pub(crate) struct StepNode {
    /// Seconds on the scenario clock from when the step became current.
    pub(crate) after: Option<f64>,
    /// How long the step may wait before the run is declared stuck.
    pub(crate) deadline: Option<f64>,
}

/// A step's `until` gate: the event it waits for. Its filters are children.
///
/// A node rather than a field of [`StepNode`] because a gate HAS filters, and
/// a step with no gate is a step with no gate node - which is the whole of
/// `until: Option<..>` with nothing left over.
#[derive(Component, Debug, Clone, Reflect)]
pub(crate) struct GateNode {
    /// The event kind the step waits for.
    pub(crate) trigger: EventConfig,
}

impl Default for GateNode {
    fn default() -> Self {
        Self {
            trigger: EventConfig::OnEnter,
        }
    }
}

/// What a handler reads as: its trigger, then the name its author gave it.
///
/// BOTH, because they answer different questions. `On Enter` says WHEN the
/// handler runs and `picket warden wakes` says WHAT it is for, and a row
/// showing one of them leaves the other unanswerable from the tree.
pub(crate) fn handler_text(event: &EventNode) -> String {
    let trigger = event_label(event.trigger);
    match event
        .label
        .as_deref()
        .map(str::trim)
        .filter(|label| !label.is_empty())
    {
        Some(label) => format!("{trigger} - {label}"),
        None => trigger.to_string(),
    }
}

/// What a handler's trigger is called in the panel.
///
/// The RON name with spaces in it, not a rewording: the string a builder types
/// into a hand-written mod is the string the row shows, so the panel teaches
/// the format rather than a second vocabulary for it.
pub(crate) fn event_label(name: EventConfig) -> &'static str {
    match name {
        EventConfig::OnStart => "On Start",
        EventConfig::OnDefeated => "On Defeated",
        EventConfig::OnDestroyed => "On Destroyed",
        EventConfig::OnNeutralized => "On Neutralized",
        EventConfig::OnUpdate => "On Update",
        EventConfig::OnTimerEnd => "On Timer End",
        EventConfig::OnEnter => "On Enter",
        EventConfig::OnExit => "On Exit",
        EventConfig::OnOrbitStart => "On Orbit Start",
        EventConfig::OnOrbitStable => "On Orbit Stable",
        EventConfig::OnOrbitUnstable => "On Orbit Unstable",
        EventConfig::OnOrbitEnd => "On Orbit End",
        EventConfig::OnTravelLockStart => "On Travel Lock",
        EventConfig::OnTravelLockEnd => "On Travel Unlock",
        EventConfig::OnCombatLockStart => "On Combat Lock",
        EventConfig::OnCombatLockEnd => "On Combat Unlock",
    }
}

/// The filter kinds the editor can add, in the order a menu lists them.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub(crate) enum FilterChoice {
    /// Match by entity id or type name.
    Entity,
    /// Compare scenario variables.
    Expression,
    /// Match a timer by key.
    Timer,
    /// Invert one filter.
    Not,
    /// Both.
    And,
    /// Either.
    Or,
}

impl FilterChoice {
    /// Every filter a handler can be given.
    pub(crate) const ALL: [FilterChoice; 6] = [
        FilterChoice::Entity,
        FilterChoice::Expression,
        FilterChoice::Timer,
        FilterChoice::Not,
        FilterChoice::And,
        FilterChoice::Or,
    ];

    /// The row label.
    pub(crate) fn label(self) -> &'static str {
        match self {
            FilterChoice::Entity => "Entity",
            FilterChoice::Expression => "Expression",
            FilterChoice::Timer => "Timer",
            FilterChoice::Not => "Not",
            FilterChoice::And => "And",
            FilterChoice::Or => "Or",
        }
    }

    /// The stem a minted id is named after.
    pub(crate) fn stem(self) -> &'static str {
        match self {
            FilterChoice::Entity => "entity",
            FilterChoice::Expression => "expression",
            FilterChoice::Timer => "timer",
            FilterChoice::Not => "not",
            FilterChoice::And => "and",
            FilterChoice::Or => "or",
        }
    }

    /// How many child filters this kind takes. Zero for a leaf.
    pub(crate) fn operands(self) -> usize {
        match self {
            FilterChoice::Entity | FilterChoice::Expression | FilterChoice::Timer => 0,
            FilterChoice::Not => 1,
            FilterChoice::And | FilterChoice::Or => 2,
        }
    }

    /// A fresh filter of this kind. Every field is EMPTY rather than guessed:
    /// a filter that arrived matching something would gate a handler on a
    /// choice nobody made.
    pub(crate) fn stock(self) -> FilterKind {
        match self {
            FilterChoice::Entity => FilterKind::Entity(EntityFilterConfig::default()),
            FilterChoice::Expression => FilterKind::Expression,
            FilterChoice::Timer => FilterKind::Timer(TimerFilterConfig { key: String::new() }),
            FilterChoice::Not => FilterKind::Not,
            FilterChoice::And => FilterKind::And,
            FilterChoice::Or => FilterKind::Or,
        }
    }
}

/// The operator kinds an expression node can be switched to, in the order the
/// panel lists them: the comparisons first, because a condition's root is one
/// of those and that is the row a builder lands on.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExprChoice {
    /// Equality.
    Equal,
    /// Less than.
    LessThan,
    /// Greater than.
    GreaterThan,
    /// Sum.
    Add,
    /// Difference.
    Subtract,
    /// Product.
    Multiply,
    /// Quotient.
    Divide,
    /// A typed operand.
    Value,
}

impl ExprChoice {
    /// Every operator, and the leaf.
    pub(crate) const ALL: [ExprChoice; 8] = [
        ExprChoice::Equal,
        ExprChoice::LessThan,
        ExprChoice::GreaterThan,
        ExprChoice::Add,
        ExprChoice::Subtract,
        ExprChoice::Multiply,
        ExprChoice::Divide,
        ExprChoice::Value,
    ];

    /// The row label: the SYMBOL, not the word for it.
    ///
    /// `==` is what the grammar spells and what the file holds, and a column of
    /// symbols is a column the eye can pick an operator out of - which is the
    /// whole reason these are nodes.
    pub(crate) fn label(self) -> &'static str {
        match self {
            ExprChoice::Equal => "==",
            ExprChoice::LessThan => "<",
            ExprChoice::GreaterThan => ">",
            ExprChoice::Add => "+",
            ExprChoice::Subtract => "-",
            ExprChoice::Multiply => "*",
            ExprChoice::Divide => "/",
            ExprChoice::Value => "value",
        }
    }

    /// The stem a minted id is named after. Words, not symbols: an id is typed
    /// into a filter field and read in a file.
    pub(crate) fn stem(self) -> &'static str {
        match self {
            ExprChoice::Equal => "equal",
            ExprChoice::LessThan => "less",
            ExprChoice::GreaterThan => "greater",
            ExprChoice::Add => "add",
            ExprChoice::Subtract => "subtract",
            ExprChoice::Multiply => "multiply",
            ExprChoice::Divide => "divide",
            ExprChoice::Value => "value",
        }
    }

    /// How many operands the kind takes. Every operator is binary; a value
    /// takes none.
    pub(crate) fn operands(self) -> usize {
        match self {
            ExprChoice::Value => 0,
            _ => 2,
        }
    }

    /// Whether this kind compares - and so may only ever be a condition's root.
    pub(crate) fn compares(self) -> bool {
        matches!(
            self,
            ExprChoice::Equal | ExprChoice::LessThan | ExprChoice::GreaterThan
        )
    }

    /// A fresh node of this kind. A value arrives as `0`, which is a whole
    /// expression that evaluates - an empty box would be a condition that
    /// cannot be read until it is finished.
    pub(crate) fn stock(self) -> ExprKind {
        match self {
            ExprChoice::Equal => ExprKind::Equal,
            ExprChoice::LessThan => ExprKind::LessThan,
            ExprChoice::GreaterThan => ExprKind::GreaterThan,
            ExprChoice::Add => ExprKind::Add,
            ExprChoice::Subtract => ExprKind::Subtract,
            ExprChoice::Multiply => ExprKind::Multiply,
            ExprChoice::Divide => ExprKind::Divide,
            ExprChoice::Value => ExprKind::Value(ValueOperand { value: number(0.0) }),
        }
    }
}

/// Which operator a node holds, as a choice.
pub(crate) fn expr_choice(kind: &ExprKind) -> ExprChoice {
    match kind {
        ExprKind::Equal => ExprChoice::Equal,
        ExprKind::LessThan => ExprChoice::LessThan,
        ExprKind::GreaterThan => ExprChoice::GreaterThan,
        ExprKind::Add => ExprChoice::Add,
        ExprKind::Subtract => ExprChoice::Subtract,
        ExprKind::Multiply => ExprChoice::Multiply,
        ExprKind::Divide => ExprChoice::Divide,
        ExprKind::Value(_) => ExprChoice::Value,
    }
}

/// The same config, for writing.
pub(crate) fn expr_config_mut(kind: &mut ExprKind) -> Option<&mut dyn PartialReflect> {
    match kind {
        ExprKind::Value(operand) => Some(operand),
        _ => None,
    }
}

/// An expression that is one number.
fn number(value: f64) -> VariableExpressionNode {
    VariableExpressionNode::new_term(VariableTermNode::new_factor(
        VariableFactorNode::new_literal(VariableLiteral::Number(value)),
    ))
}

/// Which filter a node holds, as a choice.
pub(crate) fn filter_choice(kind: &FilterKind) -> FilterChoice {
    match kind {
        FilterKind::Entity(_) => FilterChoice::Entity,
        FilterKind::Expression => FilterChoice::Expression,
        FilterKind::Timer(_) => FilterChoice::Timer,
        FilterKind::Not => FilterChoice::Not,
        FilterKind::And => FilterChoice::And,
        FilterKind::Or => FilterChoice::Or,
    }
}

/// The config a filter node carries, for reading. `None` for a filter whose
/// content is its children: a combinator, or the comparison.
pub(crate) fn filter_config(kind: &FilterKind) -> Option<&dyn PartialReflect> {
    match kind {
        FilterKind::Entity(config) => Some(config),
        FilterKind::Timer(config) => Some(config),
        FilterKind::Expression | FilterKind::Not | FilterKind::And | FilterKind::Or => None,
    }
}

/// The same config, for writing.
pub(crate) fn filter_config_mut(kind: &mut FilterKind) -> Option<&mut dyn PartialReflect> {
    match kind {
        FilterKind::Entity(config) => Some(config),
        FilterKind::Timer(config) => Some(config),
        FilterKind::Expression | FilterKind::Not | FilterKind::And | FilterKind::Or => None,
    }
}

/// The config an action node carries, for reading.
///
/// A `Sequence` hands back its HEAD - the key - and not its steps, which are
/// child nodes with panels of their own.
pub(crate) fn action_config(kind: &ActionKind) -> Option<&dyn PartialReflect> {
    match kind {
        ActionKind::Sequence(head) => Some(head),
        ActionKind::VariableSet(head) => Some(head),
        ActionKind::Leaf(action) => leaf_config(action),
    }
}

/// The same config, for writing.
pub(crate) fn action_config_mut(kind: &mut ActionKind) -> Option<&mut dyn PartialReflect> {
    match kind {
        ActionKind::Sequence(head) => Some(head),
        ActionKind::VariableSet(head) => Some(head),
        ActionKind::Leaf(action) => leaf_config_mut(action),
    }
}

/// The payload of every action arm but `Sequence`.
fn leaf_config(action: &EventActionConfig) -> Option<&dyn PartialReflect> {
    match action {
        EventActionConfig::DebugMessage(config) => Some(config),
        EventActionConfig::VariableSet(config) => Some(config),
        EventActionConfig::TimerStart(config) => Some(config),
        EventActionConfig::TimerCancel(config) => Some(config),
        EventActionConfig::Objective(config) => Some(config),
        EventActionConfig::ObjectiveComplete(config) => Some(config),
        EventActionConfig::ObjectiveMarkerAttach(config) => Some(config),
        EventActionConfig::ObjectiveMarkerDetach(config) => Some(config),
        EventActionConfig::HintEmphasisSet(config) => Some(config),
        EventActionConfig::HintEmphasisClear(config) => Some(config),
        EventActionConfig::SpawnScenarioObject(config) => Some(config),
        EventActionConfig::ScatterObjects(config) => Some(config),
        EventActionConfig::DespawnScenarioObject(config) => Some(config),
        EventActionConfig::SetSpeedCap(config) => Some(config),
        EventActionConfig::SetControllerVerb(config) => Some(config),
        EventActionConfig::SetAllegiance(config) => Some(config),
        EventActionConfig::ForceTorpedoLaunch(config) => Some(config),
        EventActionConfig::SetInfiniteAmmo(config) => Some(config),
        EventActionConfig::RefillAmmo(config) => Some(config),
        EventActionConfig::CreateScenarioArea(config) => Some(config),
        EventActionConfig::NextScenario(config) => Some(config),
        EventActionConfig::SetCamera(config) => Some(config),
        EventActionConfig::Screenshot(config) => Some(config),
        EventActionConfig::SetSkybox(config) => Some(config),
        EventActionConfig::Outcome(config) => Some(config),
        EventActionConfig::StoryMessage(config) => Some(config),
        EventActionConfig::HudReadout(config) => Some(config),
        EventActionConfig::Sequence(_) => None,
    }
}

/// The same payload, for writing.
fn leaf_config_mut(action: &mut EventActionConfig) -> Option<&mut dyn PartialReflect> {
    match action {
        EventActionConfig::DebugMessage(config) => Some(config),
        EventActionConfig::VariableSet(config) => Some(config),
        EventActionConfig::TimerStart(config) => Some(config),
        EventActionConfig::TimerCancel(config) => Some(config),
        EventActionConfig::Objective(config) => Some(config),
        EventActionConfig::ObjectiveComplete(config) => Some(config),
        EventActionConfig::ObjectiveMarkerAttach(config) => Some(config),
        EventActionConfig::ObjectiveMarkerDetach(config) => Some(config),
        EventActionConfig::HintEmphasisSet(config) => Some(config),
        EventActionConfig::HintEmphasisClear(config) => Some(config),
        EventActionConfig::SpawnScenarioObject(config) => Some(config),
        EventActionConfig::ScatterObjects(config) => Some(config),
        EventActionConfig::DespawnScenarioObject(config) => Some(config),
        EventActionConfig::SetSpeedCap(config) => Some(config),
        EventActionConfig::SetControllerVerb(config) => Some(config),
        EventActionConfig::SetAllegiance(config) => Some(config),
        EventActionConfig::ForceTorpedoLaunch(config) => Some(config),
        EventActionConfig::SetInfiniteAmmo(config) => Some(config),
        EventActionConfig::RefillAmmo(config) => Some(config),
        EventActionConfig::CreateScenarioArea(config) => Some(config),
        EventActionConfig::NextScenario(config) => Some(config),
        EventActionConfig::SetCamera(config) => Some(config),
        EventActionConfig::Screenshot(config) => Some(config),
        EventActionConfig::SetSkybox(config) => Some(config),
        EventActionConfig::Outcome(config) => Some(config),
        EventActionConfig::StoryMessage(config) => Some(config),
        EventActionConfig::HudReadout(config) => Some(config),
        EventActionConfig::Sequence(_) => None,
    }
}

/// The action kinds the editor can add, grouped by what they touch and
/// ordered the way a menu lists them: the mission surface first (what a player
/// is told to do), then the world, then the ships in it, then the run's own
/// flow, and the authoring aids last.
///
/// EVERY arm of [`EventActionConfig`] is here. An action the menu skipped
/// would be one a hand-written mod can hold and the editor would silently drop
/// on the next save.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub(crate) enum ActionChoice {
    /// Post an objective on the HUD.
    Objective,
    /// Complete one.
    ObjectiveComplete,
    /// Put the gold marker chip on an object.
    ObjectiveMarkerAttach,
    /// Take it off.
    ObjectiveMarkerDetach,
    /// A speaker-attributed comms line.
    StoryMessage,
    /// Bind a HUD readout to a variable.
    HudReadout,
    /// Pulse a keybind chip gold.
    HintEmphasisSet,
    /// Stop pulsing it.
    HintEmphasisClear,
    /// Spawn one object.
    SpawnScenarioObject,
    /// Spawn a seeded field of them.
    ScatterObjects,
    /// Despawn one by id.
    DespawnScenarioObject,
    /// Create a sensor sphere that drives `OnEnter`/`OnExit`.
    CreateScenarioArea,
    /// Swap the sky.
    SetSkybox,
    /// Pose the scenario camera.
    SetCamera,
    /// Capture the window.
    Screenshot,
    /// Cap a ship's manual speed.
    SetSpeedCap,
    /// Enable or disable one of a ship's flight verbs.
    SetControllerVerb,
    /// Change which side a ship is on.
    SetAllegiance,
    /// Order a ship's torpedo bays to launch.
    ForceTorpedoLaunch,
    /// Take a ship's magazines away, or give them back.
    SetInfiniteAmmo,
    /// Refill a ship's magazines, or one section's.
    RefillAmmo,
    /// Declare the scenario won or lost.
    Outcome,
    /// Queue a switch to another scenario.
    NextScenario,
    /// Start an ordered beat chain.
    Sequence,
    /// Start or restart a timer.
    TimerStart,
    /// Cancel one.
    TimerCancel,
    /// Evaluate an expression into a variable.
    VariableSet,
    /// Log a line.
    DebugMessage,
}

impl ActionChoice {
    /// Every action a handler can be given.
    pub(crate) const ALL: [ActionChoice; 28] = [
        ActionChoice::Objective,
        ActionChoice::ObjectiveComplete,
        ActionChoice::ObjectiveMarkerAttach,
        ActionChoice::ObjectiveMarkerDetach,
        ActionChoice::StoryMessage,
        ActionChoice::HudReadout,
        ActionChoice::HintEmphasisSet,
        ActionChoice::HintEmphasisClear,
        ActionChoice::SpawnScenarioObject,
        ActionChoice::ScatterObjects,
        ActionChoice::DespawnScenarioObject,
        ActionChoice::CreateScenarioArea,
        ActionChoice::SetSkybox,
        ActionChoice::SetCamera,
        ActionChoice::Screenshot,
        ActionChoice::SetSpeedCap,
        ActionChoice::SetControllerVerb,
        ActionChoice::SetAllegiance,
        ActionChoice::ForceTorpedoLaunch,
        ActionChoice::SetInfiniteAmmo,
        ActionChoice::RefillAmmo,
        ActionChoice::Outcome,
        ActionChoice::NextScenario,
        ActionChoice::Sequence,
        ActionChoice::TimerStart,
        ActionChoice::TimerCancel,
        ActionChoice::VariableSet,
        ActionChoice::DebugMessage,
    ];

    /// The row label.
    pub(crate) fn label(self) -> &'static str {
        match self {
            ActionChoice::Objective => "Objective",
            ActionChoice::ObjectiveComplete => "Objective Complete",
            ActionChoice::ObjectiveMarkerAttach => "Marker Attach",
            ActionChoice::ObjectiveMarkerDetach => "Marker Detach",
            ActionChoice::StoryMessage => "Story Message",
            ActionChoice::HudReadout => "HUD Readout",
            ActionChoice::HintEmphasisSet => "Hint Emphasis",
            ActionChoice::HintEmphasisClear => "Hint Clear",
            ActionChoice::SpawnScenarioObject => "Spawn Object",
            ActionChoice::ScatterObjects => "Scatter Objects",
            ActionChoice::DespawnScenarioObject => "Despawn Object",
            ActionChoice::CreateScenarioArea => "Create Area",
            ActionChoice::SetSkybox => "Set Skybox",
            ActionChoice::SetCamera => "Set Camera",
            ActionChoice::Screenshot => "Screenshot",
            ActionChoice::SetSpeedCap => "Set Speed Cap",
            ActionChoice::SetControllerVerb => "Set Flight Verb",
            ActionChoice::SetAllegiance => "Set Allegiance",
            ActionChoice::ForceTorpedoLaunch => "Torpedo Launch",
            ActionChoice::SetInfiniteAmmo => "Set Infinite Ammo",
            ActionChoice::RefillAmmo => "Refill Ammo",
            ActionChoice::Outcome => "Outcome",
            ActionChoice::NextScenario => "Next Scenario",
            ActionChoice::Sequence => "Sequence",
            ActionChoice::TimerStart => "Timer Start",
            ActionChoice::TimerCancel => "Timer Cancel",
            ActionChoice::VariableSet => "Variable Set",
            ActionChoice::DebugMessage => "Debug Message",
        }
    }

    /// The stem a minted id is named after.
    pub(crate) fn stem(self) -> &'static str {
        match self {
            ActionChoice::Objective => "objective",
            ActionChoice::ObjectiveComplete => "complete",
            ActionChoice::ObjectiveMarkerAttach => "marker",
            ActionChoice::ObjectiveMarkerDetach => "unmarker",
            ActionChoice::StoryMessage => "story",
            ActionChoice::HudReadout => "readout",
            ActionChoice::HintEmphasisSet => "hint",
            ActionChoice::HintEmphasisClear => "unhint",
            ActionChoice::SpawnScenarioObject => "spawn",
            ActionChoice::ScatterObjects => "scatter",
            ActionChoice::DespawnScenarioObject => "despawn",
            ActionChoice::CreateScenarioArea => "area",
            ActionChoice::SetSkybox => "sky",
            ActionChoice::SetCamera => "camera",
            ActionChoice::Screenshot => "shot",
            ActionChoice::SetSpeedCap => "cap",
            ActionChoice::SetControllerVerb => "verb",
            ActionChoice::SetAllegiance => "allegiance",
            ActionChoice::ForceTorpedoLaunch => "launch",
            ActionChoice::SetInfiniteAmmo => "unlimited",
            ActionChoice::RefillAmmo => "refill",
            ActionChoice::Outcome => "outcome",
            ActionChoice::NextScenario => "next",
            ActionChoice::Sequence => "sequence",
            ActionChoice::TimerStart => "timer",
            ActionChoice::TimerCancel => "untimer",
            ActionChoice::VariableSet => "set",
            ActionChoice::DebugMessage => "debug",
        }
    }

    /// A fresh action of this kind.
    ///
    /// Every id field is EMPTY. A stock action that arrived naming
    /// `player_spaceship` would look authored, and a builder who never opened
    /// its panel would ship a handler pointed at a ship they never chose.
    pub(crate) fn stock(self) -> ActionKind {
        let action = match self {
            ActionChoice::Objective => {
                EventActionConfig::Objective(ObjectiveActionConfig::new("", "A new objective"))
            }
            ActionChoice::ObjectiveComplete => {
                EventActionConfig::ObjectiveComplete(ObjectiveCompleteActionConfig {
                    id: String::new(),
                })
            }
            ActionChoice::ObjectiveMarkerAttach => {
                EventActionConfig::ObjectiveMarkerAttach(ObjectiveMarkerAttachActionConfig {
                    target_id: String::new(),
                    label: String::new(),
                })
            }
            ActionChoice::ObjectiveMarkerDetach => {
                EventActionConfig::ObjectiveMarkerDetach(ObjectiveMarkerDetachActionConfig {
                    target_id: String::new(),
                })
            }
            ActionChoice::StoryMessage => {
                EventActionConfig::StoryMessage(StoryMessageActionConfig {
                    speaker: String::new(),
                    text: String::new(),
                    dwell: None,
                    icon: None,
                })
            }
            ActionChoice::HudReadout => EventActionConfig::HudReadout(HudReadoutActionConfig {
                slot: String::new(),
                variable: String::new(),
                format: HudReadoutFormatConfig::default(),
                label: None,
                visible: true,
            }),
            ActionChoice::HintEmphasisSet => {
                EventActionConfig::HintEmphasisSet(HintEmphasisSetActionConfig {
                    verb: String::new(),
                })
            }
            ActionChoice::HintEmphasisClear => {
                EventActionConfig::HintEmphasisClear(HintEmphasisClearActionConfig {
                    verb: String::new(),
                })
            }
            ActionChoice::SpawnScenarioObject => {
                EventActionConfig::SpawnScenarioObject(stock_object())
            }
            ActionChoice::ScatterObjects => {
                EventActionConfig::ScatterObjects(ScatterObjectsConfig {
                    id_prefix: String::new(),
                    count: 8,
                    seed: 1,
                    region: ScatterRegion::Box {
                        min: Meters3(Vec3::splat(-1000.0)),
                        max: Meters3(Vec3::splat(1000.0)),
                    },
                    template: stock_object(),
                    asteroid_radius: None,
                    min_separation: None,
                })
            }
            ActionChoice::DespawnScenarioObject => {
                EventActionConfig::DespawnScenarioObject(DespawnScenarioObjectActionConfig {
                    id: String::new(),
                })
            }
            ActionChoice::CreateScenarioArea => {
                EventActionConfig::CreateScenarioArea(ScenarioAreaConfig {
                    id: String::new(),
                    name: String::new(),
                    position: Meters3::ZERO,
                    rotation: Quat::IDENTITY,
                    radius: Meters(1000.0),
                })
            }
            ActionChoice::SetSkybox => EventActionConfig::SetSkybox(SetSkyboxActionConfig {
                cubemap: AssetRef::from(DEFAULT_SKY),
                brightness: None,
            }),
            ActionChoice::SetCamera => EventActionConfig::SetCamera(SetCameraActionConfig {
                position: Meters3::new(0.0, 400.0, 1200.0),
                look_at: Meters3::ZERO,
            }),
            ActionChoice::Screenshot => EventActionConfig::Screenshot(ScreenshotActionConfig {
                path: "shot.png".to_string(),
            }),
            ActionChoice::SetSpeedCap => EventActionConfig::SetSpeedCap(SetSpeedCapActionConfig {
                id: String::new(),
                cap: None,
            }),
            ActionChoice::SetControllerVerb => {
                EventActionConfig::SetControllerVerb(SetControllerVerbActionConfig {
                    id: String::new(),
                    verb: FlightVerb::Goto,
                    enabled: true,
                })
            }
            ActionChoice::SetAllegiance => {
                EventActionConfig::SetAllegiance(SetAllegianceActionConfig {
                    id: String::new(),
                    allegiance: Allegiance::Enemy,
                })
            }
            ActionChoice::ForceTorpedoLaunch => {
                EventActionConfig::ForceTorpedoLaunch(ForceTorpedoLaunchActionConfig {
                    id: String::new(),
                    target: String::new(),
                })
            }
            ActionChoice::SetInfiniteAmmo => {
                EventActionConfig::SetInfiniteAmmo(SetInfiniteAmmoActionConfig {
                    id: String::new(),
                    enabled: true,
                })
            }
            ActionChoice::RefillAmmo => EventActionConfig::RefillAmmo(RefillAmmoActionConfig {
                id: String::new(),
                section: None,
            }),
            ActionChoice::Outcome => EventActionConfig::Outcome(OutcomeActionConfig::new(
                ScenarioOutcomeKind::Victory,
                "",
            )),
            ActionChoice::NextScenario => {
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: String::new(),
                    linger: true,
                    delay: None,
                })
            }
            // The one arm whose steps are children, so the node holds the head
            // and the caller adds a first step beside it.
            ActionChoice::Sequence => return ActionKind::Sequence(SequenceHead::default()),
            ActionChoice::TimerStart => EventActionConfig::TimerStart(TimerStartActionConfig {
                key: String::new(),
                seconds: number(10.0),
            }),
            ActionChoice::TimerCancel => {
                EventActionConfig::TimerCancel(TimerCancelActionConfig { key: String::new() })
            }
            // The second arm whose innards are children: the value is an
            // expression node beside the key, not a field inside it.
            ActionChoice::VariableSet => return ActionKind::VariableSet(VariableSetHead::default()),
            ActionChoice::DebugMessage => {
                EventActionConfig::DebugMessage(DebugMessageActionConfig {
                    message: String::new(),
                })
            }
        };
        ActionKind::Leaf(action)
    }
}

/// The object a fresh spawn or scatter starts from: a small unnamed rock, so
/// the action reads as placed rather than as configured.
fn stock_object() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: String::new(),
            name: String::new(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: Meters(30.0),
            texture: AssetRef::from(crate::node::ASTEROID_TEXTURE),
            material: None,
            destroy_sound: Some(AssetRef::from(crate::node::DESTROY_SOUND)),
            mass: None,
            invulnerable: false,
            seed: None,
            lock_signature: None,
        }),
    }
}

/// Which action a node holds, as a choice.
pub(crate) fn action_choice(kind: &ActionKind) -> ActionChoice {
    match kind {
        ActionKind::Sequence(_) => ActionChoice::Sequence,
        ActionKind::VariableSet(_) => ActionChoice::VariableSet,
        ActionKind::Leaf(action) => match action {
            EventActionConfig::DebugMessage(_) => ActionChoice::DebugMessage,
            EventActionConfig::VariableSet(_) => ActionChoice::VariableSet,
            EventActionConfig::TimerStart(_) => ActionChoice::TimerStart,
            EventActionConfig::TimerCancel(_) => ActionChoice::TimerCancel,
            EventActionConfig::Objective(_) => ActionChoice::Objective,
            EventActionConfig::ObjectiveComplete(_) => ActionChoice::ObjectiveComplete,
            EventActionConfig::ObjectiveMarkerAttach(_) => ActionChoice::ObjectiveMarkerAttach,
            EventActionConfig::ObjectiveMarkerDetach(_) => ActionChoice::ObjectiveMarkerDetach,
            EventActionConfig::HintEmphasisSet(_) => ActionChoice::HintEmphasisSet,
            EventActionConfig::HintEmphasisClear(_) => ActionChoice::HintEmphasisClear,
            EventActionConfig::SpawnScenarioObject(_) => ActionChoice::SpawnScenarioObject,
            EventActionConfig::ScatterObjects(_) => ActionChoice::ScatterObjects,
            EventActionConfig::DespawnScenarioObject(_) => ActionChoice::DespawnScenarioObject,
            EventActionConfig::SetSpeedCap(_) => ActionChoice::SetSpeedCap,
            EventActionConfig::SetControllerVerb(_) => ActionChoice::SetControllerVerb,
            EventActionConfig::SetAllegiance(_) => ActionChoice::SetAllegiance,
            EventActionConfig::ForceTorpedoLaunch(_) => ActionChoice::ForceTorpedoLaunch,
            EventActionConfig::SetInfiniteAmmo(_) => ActionChoice::SetInfiniteAmmo,
            EventActionConfig::RefillAmmo(_) => ActionChoice::RefillAmmo,
            EventActionConfig::CreateScenarioArea(_) => ActionChoice::CreateScenarioArea,
            EventActionConfig::NextScenario(_) => ActionChoice::NextScenario,
            EventActionConfig::SetCamera(_) => ActionChoice::SetCamera,
            EventActionConfig::Screenshot(_) => ActionChoice::Screenshot,
            EventActionConfig::SetSkybox(_) => ActionChoice::SetSkybox,
            EventActionConfig::Outcome(_) => ActionChoice::Outcome,
            EventActionConfig::StoryMessage(_) => ActionChoice::StoryMessage,
            EventActionConfig::HudReadout(_) => ActionChoice::HudReadout,
            // Unreachable: a lifted `Sequence` becomes `ActionKind::Sequence`.
            EventActionConfig::Sequence(_) => ActionChoice::Sequence,
        },
    }
}

/// The node the script hangs from: one per document, under the scenario.
///
/// Its own node rather than hanging handlers straight off the scenario so the
/// script has its own ID SPACE. `event_1` and `asteroid_1` are minted from
/// different counters and can never be the same node, which is what lets the
/// EVENTS tab number its rows from one while the world keeps its own.
#[derive(Component, Debug)]
pub(crate) struct ScriptNode;

/// A container whose children are drawn under it in the tree.
///
/// Absence is COLLAPSED, which is what makes a loaded script readable: a
/// shipped scenario is a hundred nodes deep and a rail that opened all of them
/// is a wall of rows with the handler you added at the bottom of it. The mark
/// is a view of the document and not part of it - nothing saves or lowers it.
#[derive(Component, Debug)]
pub(crate) struct Expanded;

/// Take a handler list apart into nodes under a fresh script node.
///
/// The inverse of [`ScriptNodes::lower`], and the only thing that creates the
/// script node. Ids are minted in AUTHORED ORDER, because the ordinal is what
/// the lowering sorts by: an action list read back in a different order is a
/// different scenario.
pub(crate) fn lift(
    commands: &mut Commands,
    scenario: Entity,
    events: Vec<ScenarioEventConfig>,
) -> Entity {
    let script = commands
        .spawn((
            EditorNode,
            ScriptNode,
            NodeId("script".to_string()),
            NextChildOrdinal(u32::try_from(events.len()).unwrap_or(u32::MAX)),
            Name::new("Script Node"),
            ChildOf(scenario),
        ))
        .id();
    for (index, event) in events.into_iter().enumerate() {
        lift_event(commands, script, ordinal_at(index), event);
    }
    script
}

/// The 1-based ordinal of the `index`th child.
fn ordinal_at(index: usize) -> u32 {
    u32::try_from(index + 1).unwrap_or(u32::MAX)
}

/// How many children a node will have, as an ordinal counter.
fn counter(count: usize) -> NextChildOrdinal {
    NextChildOrdinal(u32::try_from(count).unwrap_or(u32::MAX))
}

fn lift_event(
    commands: &mut Commands,
    script: Entity,
    ordinal: u32,
    event: ScenarioEventConfig,
) -> Entity {
    let node = commands
        .spawn((
            EditorNode,
            EventNode {
                label: event.label,
                trigger: event.name,
                once: event.once,
            },
            NodeId(format!("event_{ordinal}")),
            counter(event.filters.len() + event.actions.len()),
            Name::new(format!("Event Node event_{ordinal}")),
            ChildOf(script),
        ))
        .id();
    // FILTERS FIRST, then actions, through one ordinal space. Each list keeps
    // its own order because the lowering reads the two apart by component and
    // sorts each by ordinal.
    let mut next = 0;
    for filter in event.filters {
        next += 1;
        lift_filter(commands, node, next, filter);
    }
    for action in event.actions {
        next += 1;
        lift_action(commands, node, next, action);
    }
    node
}

fn lift_filter(
    commands: &mut Commands,
    parent: Entity,
    ordinal: u32,
    filter: EventFilterConfig,
) -> Entity {
    let (kind, operands, condition) = match filter {
        EventFilterConfig::Entity(config) => (FilterKind::Entity(config), Vec::new(), None),
        EventFilterConfig::Expression(ExpressionFilterConfig(condition)) => {
            (FilterKind::Expression, Vec::new(), Some(condition))
        }
        EventFilterConfig::Timer(config) => (FilterKind::Timer(config), Vec::new(), None),
        EventFilterConfig::Conditional(ConditionalFilterConfig::Not(inner)) => {
            (FilterKind::Not, vec![*inner], None)
        }
        EventFilterConfig::Conditional(ConditionalFilterConfig::And(left, right)) => {
            (FilterKind::And, vec![*left, *right], None)
        }
        EventFilterConfig::Conditional(ConditionalFilterConfig::Or(left, right)) => {
            (FilterKind::Or, vec![*left, *right], None)
        }
    };
    let id = format!("{}_{ordinal}", filter_choice(&kind).stem());
    let node = commands
        .spawn((
            EditorNode,
            FilterNode { kind },
            NodeId(id.clone()),
            counter(operands.len() + usize::from(condition.is_some())),
            Name::new(format!("Filter Node {id}")),
            ChildOf(parent),
        ))
        .id();
    if let Some(condition) = condition {
        lift_condition(commands, node, 1, condition);
    }
    for (index, operand) in operands.into_iter().enumerate() {
        lift_filter(commands, node, ordinal_at(index), operand);
    }
    node
}

/// A condition, as the node tree that shows its shape.
fn lift_condition(
    commands: &mut Commands,
    parent: Entity,
    ordinal: u32,
    condition: VariableConditionNode,
) -> Entity {
    let (kind, left, right) = match condition {
        VariableConditionNode::Equal(left, right) => (ExprKind::Equal, left, right),
        VariableConditionNode::LessThan(left, right) => (ExprKind::LessThan, left, right),
        VariableConditionNode::GreaterThan(left, right) => (ExprKind::GreaterThan, left, right),
    };
    let node = spawn_expression(commands, parent, ordinal, kind, 2);
    lift_expression(commands, node, 1, *left);
    lift_expression(commands, node, 2, *right);
    node
}

/// A value expression, as nodes.
///
/// A `Parens` is DROPPED on the way in: the tree draws the grouping the
/// brackets were there to say, and the lowering puts them back wherever the
/// authored form needs them. What comes back is the same tree; what does not
/// survive is a bracket that was never doing anything.
fn lift_expression(
    commands: &mut Commands,
    parent: Entity,
    ordinal: u32,
    expression: VariableExpressionNode,
) -> Entity {
    match expression {
        VariableExpressionNode::Add(left, right) => {
            let node = spawn_expression(commands, parent, ordinal, ExprKind::Add, 2);
            lift_term(commands, node, 1, *left);
            lift_expression(commands, node, 2, *right);
            node
        }
        VariableExpressionNode::Subtract(left, right) => {
            let node = spawn_expression(commands, parent, ordinal, ExprKind::Subtract, 2);
            lift_term(commands, node, 1, *left);
            lift_expression(commands, node, 2, *right);
            node
        }
        VariableExpressionNode::Term(term) => lift_term(commands, parent, ordinal, term),
    }
}

fn lift_term(
    commands: &mut Commands,
    parent: Entity,
    ordinal: u32,
    term: VariableTermNode,
) -> Entity {
    match term {
        VariableTermNode::Multiply(left, right) => {
            let node = spawn_expression(commands, parent, ordinal, ExprKind::Multiply, 2);
            lift_factor(commands, node, 1, *left);
            lift_term(commands, node, 2, *right);
            node
        }
        VariableTermNode::Divide(left, right) => {
            let node = spawn_expression(commands, parent, ordinal, ExprKind::Divide, 2);
            lift_factor(commands, node, 1, *left);
            lift_term(commands, node, 2, *right);
            node
        }
        VariableTermNode::Factor(factor) => lift_factor(commands, parent, ordinal, factor),
    }
}

fn lift_factor(
    commands: &mut Commands,
    parent: Entity,
    ordinal: u32,
    factor: VariableFactorNode,
) -> Entity {
    match factor {
        VariableFactorNode::Parens(inner) => lift_expression(commands, parent, ordinal, *inner),
        leaf => {
            let value = VariableExpressionNode::new_term(VariableTermNode::new_factor(leaf));
            spawn_expression(
                commands,
                parent,
                ordinal,
                ExprKind::Value(ValueOperand { value }),
                0,
            )
        }
    }
}

/// One expression node, with room for the operands it is about to be given.
fn spawn_expression(
    commands: &mut Commands,
    parent: Entity,
    ordinal: u32,
    kind: ExprKind,
    operands: usize,
) -> Entity {
    let id = format!("{}_{ordinal}", expr_choice(&kind).stem());
    commands
        .spawn((
            EditorNode,
            ExpressionNode { kind },
            NodeId(id.clone()),
            counter(operands),
            Name::new(format!("Expression Node {id}")),
            ChildOf(parent),
        ))
        .id()
}

fn lift_action(
    commands: &mut Commands,
    parent: Entity,
    ordinal: u32,
    action: EventActionConfig,
) -> Entity {
    let (kind, steps, value) = match action {
        EventActionConfig::Sequence(config) => (
            ActionKind::Sequence(SequenceHead { key: config.key }),
            config.steps,
            None,
        ),
        EventActionConfig::VariableSet(config) => (
            ActionKind::VariableSet(VariableSetHead { key: config.key }),
            Vec::new(),
            Some(config.expression),
        ),
        leaf => (ActionKind::Leaf(leaf), Vec::new(), None),
    };
    let id = format!("{}_{ordinal}", action_choice(&kind).stem());
    let node = commands
        .spawn((
            EditorNode,
            ActionNode { kind },
            NodeId(id.clone()),
            counter(steps.len() + usize::from(value.is_some())),
            Name::new(format!("Action Node {id}")),
            ChildOf(parent),
        ))
        .id();
    // Steps and a value never meet - a sequence has no expression and a
    // variable set has no beats - so both start the ordinal space at one.
    if let Some(value) = value {
        lift_expression(commands, node, 1, value);
    }
    for (index, step) in steps.into_iter().enumerate() {
        lift_step(commands, node, ordinal_at(index), step);
    }
    node
}

fn lift_step(
    commands: &mut Commands,
    sequence: Entity,
    ordinal: u32,
    step: SequenceStepConfig,
) -> Entity {
    let id = format!("step_{ordinal}");
    let node = commands
        .spawn((
            EditorNode,
            StepNode {
                after: step.after,
                deadline: step.deadline,
            },
            NodeId(id.clone()),
            counter(usize::from(step.until.is_some()) + step.actions.len()),
            Name::new(format!("Step Node {id}")),
            ChildOf(sequence),
        ))
        .id();
    let mut next = 0;
    if let Some(gate) = step.until {
        next += 1;
        lift_gate(commands, node, next, gate);
    }
    for action in step.actions {
        next += 1;
        lift_action(commands, node, next, action);
    }
    node
}

fn lift_gate(
    commands: &mut Commands,
    step: Entity,
    ordinal: u32,
    gate: SequenceGateConfig,
) -> Entity {
    let id = format!("gate_{ordinal}");
    let node = commands
        .spawn((
            EditorNode,
            GateNode { trigger: gate.name },
            NodeId(id.clone()),
            counter(gate.filters.len()),
            Name::new(format!("Gate Node {id}")),
            ChildOf(step),
        ))
        .id();
    for (index, filter) in gate.filters.into_iter().enumerate() {
        lift_filter(commands, node, ordinal_at(index), filter);
    }
    node
}

/// Read-only access to every node of the script.
///
/// One param rather than six at every call site: the lowering, the tree and
/// the inspector all walk the same subtree, and a caller that assembled its
/// own set would be a second answer to "what is under this node".
#[derive(SystemParam)]
pub(crate) struct ScriptNodes<'w, 's> {
    roots: Query<'w, 's, (Entity, &'static ChildOf), With<ScriptNode>>,
    children: Query<'w, 's, &'static Children>,
    parents: Query<'w, 's, &'static ChildOf>,
    ids: Query<'w, 's, &'static NodeId>,
    events: Query<'w, 's, &'static EventNode>,
    filters: Query<'w, 's, &'static FilterNode>,
    actions: Query<'w, 's, &'static ActionNode>,
    steps: Query<'w, 's, &'static StepNode>,
    gates: Query<'w, 's, &'static GateNode>,
    operands: Query<'w, 's, &'static ExpressionNode>,
    open: Query<'w, 's, (), With<Expanded>>,
}

impl ScriptNodes<'_, '_> {
    /// The script node of `scenario`, if it has one.
    pub(crate) fn root(&self, scenario: Entity) -> Option<Entity> {
        self.roots
            .iter()
            .find(|(_, owner)| owner.parent() == scenario)
            .map(|(node, _)| node)
    }

    /// Whether `node`'s children are drawn under it.
    pub(crate) fn expanded(&self, node: Entity) -> bool {
        self.open.contains(node)
    }

    /// Whether `node` is part of the script at all.
    ///
    /// The operands count: an Add pressed while an operator is marked has to
    /// climb out of the condition to the handler that holds it, and a node the
    /// climb does not recognise stops it before it starts.
    pub(crate) fn holds(&self, node: Entity) -> bool {
        self.events.contains(node)
            || self.filters.contains(node)
            || self.actions.contains(node)
            || self.steps.contains(node)
            || self.gates.contains(node)
            || self.operands.contains(node)
    }

    /// The node `node` hangs from.
    pub(crate) fn owner(&self, node: Entity) -> Option<Entity> {
        self.parents.get(node).ok().map(ChildOf::parent)
    }

    /// The id a script node was minted with.
    pub(crate) fn id(&self, node: Entity) -> Option<&str> {
        self.ids.get(node).ok().map(|id| id.0.as_str())
    }

    /// The handler `node` is, if it is one.
    pub(crate) fn event(&self, node: Entity) -> Option<&EventNode> {
        self.events.get(node).ok()
    }

    /// The filter `node` is, if it is one.
    pub(crate) fn filter(&self, node: Entity) -> Option<&FilterNode> {
        self.filters.get(node).ok()
    }

    /// The action `node` is, if it is one.
    pub(crate) fn action(&self, node: Entity) -> Option<&ActionNode> {
        self.actions.get(node).ok()
    }

    /// The sequence step `node` is, if it is one.
    pub(crate) fn step(&self, node: Entity) -> Option<&StepNode> {
        self.steps.get(node).ok()
    }

    /// The gate `node` is, if it is one.
    /// The expression node `node` is, if it is one.
    pub(crate) fn expression(&self, node: Entity) -> Option<&ExpressionNode> {
        self.operands.get(node).ok()
    }

    pub(crate) fn gate(&self, node: Entity) -> Option<&GateNode> {
        self.gates.get(node).ok()
    }

    /// The condition under `node`, in the text form the grammar spells it in,
    /// or `None` where the nodes do not make one.
    ///
    /// For the tree row of the filter that HOLDS it: shut, the row still says
    /// what it compares, which is the one thing a builder scanning a handler
    /// needs from it. Open, the same condition is the rows underneath.
    pub(crate) fn condition_text(&self, node: Entity) -> Option<String> {
        let root = self.operands_of(node).into_iter().next()?;
        Some(self.lower_condition(root)?.to_string())
    }

    /// What a `VariableSet` action WRITES, spelled the way the grammar spells
    /// it, or `None` where the nodes do not make an assignment yet.
    ///
    /// For the same reason the expression filter's row reads as its condition:
    /// `Variable Set` is the name of a kind, and `beat = beat + 1` is what the
    /// action does. An unnamed key has nothing to assign TO, so the row falls
    /// back to the kind.
    pub(crate) fn assignment_text(&self, node: Entity) -> Option<String> {
        let ActionKind::VariableSet(head) = &self.action(node)?.kind else {
            return None;
        };
        let key = head.key.trim();
        if key.is_empty() {
            return None;
        }
        let root = self.operands_of(node).into_iter().next()?;
        Some(format!("{key} = {}", self.lower_expression(root)?))
    }

    /// The children of `node` that hold `T`, in AUTHORED order.
    ///
    /// By ordinal and nothing else: the stem sorts `objective_1` beside
    /// `objective_3` and puts `story_2` after both, which for an action list
    /// is a different scenario than the one that was written.
    fn ordered<T: Component>(&self, node: Entity, has: &Query<&'static T>) -> Vec<Entity> {
        let Ok(children) = self.children.get(node) else {
            return Vec::new();
        };
        let mut found: Vec<(u64, Entity)> = children
            .iter()
            .filter(|child| has.contains(*child))
            .filter_map(|child| Some((id_order(&self.ids.get(child).ok()?.0).1, child)))
            .collect();
        found.sort_unstable();
        found.into_iter().map(|(_, child)| child).collect()
    }

    /// The handlers of `scenario`, in authored order.
    pub(crate) fn events_of(&self, scenario: Entity) -> Vec<Entity> {
        let Some(script) = self.root(scenario) else {
            return Vec::new();
        };
        self.ordered(script, &self.events)
    }

    /// The filters under `node` - a handler's, or a gate's.
    pub(crate) fn filters_of(&self, node: Entity) -> Vec<Entity> {
        self.ordered(node, &self.filters)
    }

    /// The actions under `node` - a handler's, or a step's.
    pub(crate) fn actions_of(&self, node: Entity) -> Vec<Entity> {
        self.ordered(node, &self.actions)
    }

    /// The steps of a sequence action.
    pub(crate) fn steps_of(&self, node: Entity) -> Vec<Entity> {
        self.ordered(node, &self.steps)
    }

    /// The operands under `node`, in left-to-right order: an operator's two,
    /// an expression filter's one condition, a value's none.
    pub(crate) fn operands_of(&self, node: Entity) -> Vec<Entity> {
        self.ordered(node, &self.operands)
    }

    /// A step's gate, if it waits for one.
    pub(crate) fn gate_of(&self, node: Entity) -> Option<Entity> {
        self.ordered(node, &self.gates).first().copied()
    }

    /// Every name the script uses, walked off the NODES.
    ///
    /// Off the nodes rather than off [`lower`](Self::lower) because the panel
    /// asks this every frame a reference row is up: the lowering would clone
    /// the whole script to answer a question about its strings.
    pub(crate) fn names(&self, scenario: Entity) -> NamedIds {
        let mut ids = NamedIds::default();
        if let Some(root) = self.root(scenario) {
            self.gather_names(root, &mut ids);
        }
        ids
    }

    /// The names on `node` and on everything under it.
    fn gather_names(&self, node: Entity, ids: &mut NamedIds) {
        if let Some(config) = self
            .filter(node)
            .and_then(|filter| filter_config(&filter.kind))
        {
            collect(config, ids);
        }
        if let Some(action) = self.action(node) {
            // A scatter names a PREFIX and its template names the same prefix
            // as an id, which is the rule the lowering judges references by.
            if let ActionKind::Leaf(EventActionConfig::ScatterObjects(scatter)) = &action.kind {
                ids.prefixes.push(scatter.id_prefix.clone());
            }
            if let Some(config) = action_config(&action.kind) {
                collect(config, ids);
            }
        }
        let Ok(children) = self.children.get(node) else {
            return;
        };
        for &child in children {
            self.gather_names(child, ids);
        }
    }

    /// The whole script of `scenario`, as a handler list.
    pub(crate) fn lower(&self, scenario: Entity) -> Vec<ScenarioEventConfig> {
        self.events_of(scenario)
            .into_iter()
            .filter_map(|node| {
                let event = self.events.get(node).ok()?;
                Some(ScenarioEventConfig {
                    label: event.label.clone(),
                    name: event.trigger,
                    once: event.once,
                    filters: self.lower_filters(node),
                    actions: self.lower_actions(node),
                })
            })
            .collect()
    }

    fn lower_filters(&self, node: Entity) -> Vec<EventFilterConfig> {
        self.filters_of(node)
            .into_iter()
            .filter_map(|child| self.lower_filter(child))
            .collect()
    }

    fn lower_filter(&self, node: Entity) -> Option<EventFilterConfig> {
        let filter = self.filters.get(node).ok()?;
        let mut operands = self.filters_of(node).into_iter();
        Some(match &filter.kind {
            FilterKind::Entity(config) => EventFilterConfig::Entity(config.clone()),
            FilterKind::Expression => EventFilterConfig::Expression(ExpressionFilterConfig(
                self.lower_condition(self.operands_of(node).into_iter().next()?)?,
            )),
            FilterKind::Timer(config) => EventFilterConfig::Timer(config.clone()),
            // A combinator with an operand missing is DROPPED rather than
            // guessed at: `Not` of nothing is not `Not` of anything, and a
            // half-built one that lowered to its own inner filter would invert
            // the handler it gates.
            FilterKind::Not => EventFilterConfig::Conditional(ConditionalFilterConfig::not(
                self.lower_filter(operands.next()?)?,
            )),
            FilterKind::And => EventFilterConfig::Conditional(ConditionalFilterConfig::and(
                self.lower_filter(operands.next()?)?,
                self.lower_filter(operands.next()?)?,
            )),
            FilterKind::Or => EventFilterConfig::Conditional(ConditionalFilterConfig::or(
                self.lower_filter(operands.next()?)?,
                self.lower_filter(operands.next()?)?,
            )),
        })
    }

    /// A condition node, back as the comparison it draws.
    ///
    /// Only a comparison can be a condition's root: an `+` where a `<` belongs
    /// is not a condition that is nearly right, so it lowers to nothing and
    /// takes its filter with it - the same refusal a combinator missing an
    /// operand makes.
    fn lower_condition(&self, node: Entity) -> Option<VariableConditionNode> {
        let expression = self.expression(node)?;
        let mut operands = self.operands_of(node).into_iter();
        let mut side = || self.lower_expression(operands.next()?);
        let (left, right) = (side()?, side()?);
        Some(match expression.kind {
            ExprKind::Equal => VariableConditionNode::new_equals(left, right),
            ExprKind::LessThan => VariableConditionNode::new_less_than(left, right),
            ExprKind::GreaterThan => VariableConditionNode::new_greater_than(left, right),
            _ => return None,
        })
    }

    /// A value node, back as an expression.
    fn lower_expression(&self, node: Entity) -> Option<VariableExpressionNode> {
        let expression = self.expression(node)?;
        let mut operands = self.operands_of(node).into_iter();
        Some(match expression.kind {
            // A COMPARISON is not a value. The panel never offers one where a
            // value belongs, so this is a state nothing can author - but the
            // three lowerings pass a kind none of them knows on to each other,
            // and a kind all three refuse would go round for ever.
            ExprKind::Equal | ExprKind::LessThan | ExprKind::GreaterThan => return None,
            ExprKind::Add => VariableExpressionNode::new_add(
                self.lower_term(operands.next()?)?,
                self.lower_expression(operands.next()?)?,
            ),
            ExprKind::Subtract => VariableExpressionNode::new_subtract(
                self.lower_term(operands.next()?)?,
                self.lower_expression(operands.next()?)?,
            ),
            ExprKind::Value(ref operand) => operand.value.clone(),
            _ => VariableExpressionNode::new_term(self.lower_term(node)?),
        })
    }

    /// The same node where a TERM belongs.
    ///
    /// A sum in a product's place is bracketed on the way out: the tree said
    /// `(a + b) * c` by hanging the sum under the product, and the grammar says
    /// it with the brackets the lifting dropped.
    fn lower_term(&self, node: Entity) -> Option<VariableTermNode> {
        let expression = self.expression(node)?;
        let mut operands = self.operands_of(node).into_iter();
        Some(match expression.kind {
            ExprKind::Multiply => VariableTermNode::new_multiply(
                self.lower_factor(operands.next()?)?,
                self.lower_term(operands.next()?)?,
            ),
            ExprKind::Divide => VariableTermNode::new_divide(
                self.lower_factor(operands.next()?)?,
                self.lower_term(operands.next()?)?,
            ),
            _ => VariableTermNode::new_factor(self.lower_factor(node)?),
        })
    }

    /// The same node where a FACTOR belongs.
    fn lower_factor(&self, node: Entity) -> Option<VariableFactorNode> {
        let expression = self.expression(node)?;
        Some(match expression.kind {
            // A leaf holds a whole expression because a builder may type one
            // into it, so it needs the brackets too - unless it is the single
            // factor it usually is, which would only gain a pair that says
            // nothing.
            ExprKind::Value(ValueOperand {
                value: VariableExpressionNode::Term(VariableTermNode::Factor(ref factor)),
            }) => factor.clone(),
            ExprKind::Value(ref operand) => VariableFactorNode::new_parens(operand.value.clone()),
            _ => VariableFactorNode::new_parens(self.lower_expression(node)?),
        })
    }

    fn lower_actions(&self, node: Entity) -> Vec<EventActionConfig> {
        self.actions_of(node)
            .into_iter()
            .filter_map(|child| self.lower_action(child))
            .collect()
    }

    fn lower_action(&self, node: Entity) -> Option<EventActionConfig> {
        let action = self.actions.get(node).ok()?;
        Some(match &action.kind {
            ActionKind::Leaf(config) => config.clone(),
            // A value that does not lower takes its action with it, the same
            // refusal a combinator missing an operand makes: an assignment
            // with no right-hand side is not an assignment.
            ActionKind::VariableSet(head) => {
                EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: head.key.clone(),
                    expression: self
                        .lower_expression(self.operands_of(node).into_iter().next()?)?,
                })
            }
            ActionKind::Sequence(head) => EventActionConfig::Sequence(SequenceActionConfig {
                key: head.key.clone(),
                steps: self
                    .steps_of(node)
                    .into_iter()
                    .filter_map(|step| self.lower_step(step))
                    .collect(),
            }),
        })
    }

    fn lower_step(&self, node: Entity) -> Option<SequenceStepConfig> {
        let step = self.steps.get(node).ok()?;
        Some(SequenceStepConfig {
            after: step.after,
            deadline: step.deadline,
            until: self.gate_of(node).and_then(|gate| self.lower_gate(gate)),
            actions: self.lower_actions(node),
        })
    }

    fn lower_gate(&self, node: Entity) -> Option<SequenceGateConfig> {
        let gate = self.gates.get(node).ok()?;
        Some(SequenceGateConfig {
            name: gate.trigger,
            filters: self.lower_filters(node),
        })
    }
}

/// Visit every id `value` names, with what the field said it names.
///
/// Reflection and the [`Names`] attribute, rather than a match arm per action:
/// a check written as a list of action kinds goes stale the day the vocabulary
/// grows one, silently, in the direction of "this reference is fine".
///
/// An `Option` field is visited only when it is `Some` - an unset filter id
/// matches any entity and names nothing.
pub(crate) fn walk_names(value: &dyn PartialReflect, visit: &mut impl FnMut(Names, &str)) {
    match value.reflect_ref() {
        ReflectRef::Struct(fields) => {
            let info = match value.get_represented_type_info() {
                Some(TypeInfo::Struct(info)) => Some(info),
                _ => None,
            };
            for index in 0..fields.field_len() {
                let Some(field) = fields.field_at(index) else {
                    continue;
                };
                let names = info
                    .and_then(|info| info.field_at(index))
                    .and_then(|field| field.get_attribute::<Names>())
                    .copied();
                match (names, text_of(field)) {
                    (Some(names), Some(text)) => visit(names, text),
                    _ => walk_names(field, visit),
                }
            }
        }
        ReflectRef::TupleStruct(fields) => {
            for index in 0..fields.field_len() {
                if let Some(field) = fields.field(index) {
                    walk_names(field, visit);
                }
            }
        }
        ReflectRef::List(items) => {
            for index in 0..items.len() {
                if let Some(item) = items.get(index) {
                    walk_names(item, visit);
                }
            }
        }
        ReflectRef::Enum(chosen) => {
            for index in 0..chosen.field_len() {
                if let Some(field) = chosen.field_at(index) {
                    walk_names(field, visit);
                }
            }
        }
        _ => {}
    }
}

/// The string a field holds - itself, or the payload of a `Some`.
fn text_of(value: &dyn PartialReflect) -> Option<&str> {
    if let Some(text) = value.try_downcast_ref::<String>() {
        return Some(text);
    }
    let ReflectRef::Enum(chosen) = value.reflect_ref() else {
        return None;
    };
    chosen
        .field_at(0)?
        .try_downcast_ref::<String>()
        .map(String::as_str)
}

/// Every name a handler uses, sorted by what the field said it names.
///
/// The object lists are the ones the lowering JUDGES a handler by - a
/// reference nothing spawns drops the handler - and the rest are what the
/// panel offers a builder who is filling one of these fields in.
#[derive(Debug, Default)]
pub(crate) struct NamedIds {
    /// Ids the handler expects something else to have spawned.
    pub(crate) referenced: Vec<String>,
    /// Ids the handler itself puts on the board.
    pub(crate) declared: Vec<String>,
    /// Id PREFIXES a scatter puts on the board, which satisfy any reference
    /// that starts with one.
    pub(crate) prefixes: Vec<String>,
    /// Scenario variable keys.
    pub(crate) variables: Vec<String>,
    /// Scenario-local timer keys.
    pub(crate) timers: Vec<String>,
    /// HUD objective ids.
    pub(crate) objectives: Vec<String>,
    /// Other scenarios, by registered id.
    pub(crate) scenarios: Vec<String>,
}

/// What one handler names.
pub(crate) fn named_ids(event: &ScenarioEventConfig) -> NamedIds {
    let mut ids = NamedIds::default();
    for filter in &event.filters {
        walk_filter_names(filter, &mut ids);
    }
    for action in &event.actions {
        action.walk(&mut |action| {
            if let EventActionConfig::ScatterObjects(scatter) = action {
                ids.prefixes.push(scatter.id_prefix.clone());
            }
            if let Some(config) = leaf_config(action) {
                collect(config, &mut ids);
            }
        });
        action.walk_filters(&mut |filter| walk_filter_names(filter, &mut ids));
    }
    ids
}

/// The ids one filter names, combinators walked through.
fn walk_filter_names(filter: &EventFilterConfig, ids: &mut NamedIds) {
    match filter {
        EventFilterConfig::Entity(config) => collect(config, ids),
        EventFilterConfig::Timer(config) => collect(config, ids),
        EventFilterConfig::Expression(_) => {}
        EventFilterConfig::Conditional(conditional) => match conditional {
            ConditionalFilterConfig::Not(inner) => walk_filter_names(inner, ids),
            ConditionalFilterConfig::And(left, right)
            | ConditionalFilterConfig::Or(left, right) => {
                walk_filter_names(left, ids);
                walk_filter_names(right, ids);
            }
        },
    }
}

/// Sort one config's named ids into the two lists.
fn collect(config: &dyn PartialReflect, ids: &mut NamedIds) {
    walk_names(config, &mut |names, text| {
        if text.is_empty() {
            return;
        }
        match names {
            Names::Object => ids.referenced.push(text.to_string()),
            Names::NewObject => ids.declared.push(text.to_string()),
            Names::Variable => ids.variables.push(text.to_string()),
            Names::Timer => ids.timers.push(text.to_string()),
            Names::Objective => ids.objectives.push(text.to_string()),
            Names::Scenario => ids.scenarios.push(text.to_string()),
        }
    });
}

/// A named filter kind, for the panel row that switches between them.
fn named_filter(label: &str) -> Option<FilterChoice> {
    FilterChoice::ALL
        .into_iter()
        .find(|choice| choice.label() == label)
}

/// A named action kind, for the panel row that switches between them.
fn named_action(label: &str) -> Option<ActionChoice> {
    ActionChoice::ALL
        .into_iter()
        .find(|choice| choice.label() == label)
}

/// A named expression kind, for the panel row that switches between them. The
/// label is the SYMBOL, which is what the row shows.
fn named_expression(label: &str) -> Option<ExprChoice> {
    ExprChoice::ALL
        .into_iter()
        .find(|choice| choice.label() == label)
}

/// Make `node` the filter or action `label` names.
///
/// Through the WORLD rather than a system's queries: the swap reads a node's
/// kind, writes it, renames the node and despawns the children the old kind
/// owned, and a system holding `&mut FilterNode` beside the `&NodeId` and
/// `&Children` it needs to do that is three conflicting queries.
pub(crate) fn retype_script_node(world: &mut World, node: Entity, label: &str) {
    if world.get::<FilterNode>(node).is_some() {
        if let Some(choice) = named_filter(label) {
            retype_filter(world, node, choice);
        }
        return;
    }
    if world.get::<ActionNode>(node).is_some() {
        if let Some(choice) = named_action(label) {
            retype_action(world, node, choice);
        }
        return;
    }
    if world.get::<ExpressionNode>(node).is_some() {
        if let Some(choice) = named_expression(label) {
            retype_expression(world, node, choice);
        }
    }
}

/// Switch a filter to another kind, keeping the operands the new kind holds.
///
/// `And` to `Or` keeps both, `And` to `Not` keeps the first, and any of them to
/// a leaf keeps none: an operand a filter cannot hold is a node with no row and
/// no way back to one. The CONFIG is not kept - an entity filter's id means
/// nothing to a timer filter - so the new kind arrives stock.
fn retype_filter(world: &mut World, node: Entity, choice: FilterChoice) {
    let Some(mut filter) = world.get_mut::<FilterNode>(node) else {
        return;
    };
    let was = filter_choice(&filter.kind);
    if was == choice {
        return;
    }
    filter.kind = choice.stock();
    rename(world, node, "Filter", choice.stem());
    // A CONDITION is not an operand: an expression filter's one child is a
    // tree of its own, and neither the kind it came from nor the one it is
    // going to can hold it. Either side of that line keeps nothing.
    let expression = was == FilterChoice::Expression || choice == FilterChoice::Expression;
    drop_children(world, node, if expression { 0 } else { choice.operands() });
    if choice == FilterChoice::Expression {
        let stock = VariableConditionNode::new_equals(number(0.0), number(0.0));
        let mut commands = world.commands();
        lift_condition(&mut commands, node, 1, stock);
        world.flush();
        open_up(world, node);
        open_down(world, node);
    }
}

/// Switch an expression node to another operator, or to a value.
///
/// The operands are KEPT where the new kind still takes them - `==` to `<` is
/// the same two sides compared differently - and a kind that takes operands it
/// has not got is given fresh ones, so an operator always has something to
/// lower. Switching to a value drops them: what a value holds is its own text.
fn retype_expression(world: &mut World, node: Entity, choice: ExprChoice) {
    let Some(mut expression) = world.get_mut::<ExpressionNode>(node) else {
        return;
    };
    if expr_choice(&expression.kind) == choice {
        return;
    }
    expression.kind = choice.stock();
    rename(world, node, "Expression", choice.stem());
    drop_children(world, node, choice.operands());
    let held = expression_children(world, node).len();
    if held == choice.operands() {
        return;
    }
    let mut commands = world.commands();
    for ordinal in held..choice.operands() {
        lift_expression(&mut commands, node, ordinal_at(ordinal), number(0.0));
    }
    world.flush();
    open_up(world, node);
}

/// Open `node` and every operand under it, so a condition just minted arrives
/// as the rows it is made of rather than as one shut caret.
fn open_down(world: &mut World, node: Entity) {
    world.entity_mut(node).insert(Expanded);
    for child in expression_children(world, node) {
        open_down(world, child);
    }
}

/// Open `node` and everything above it, so children just minted have rows.
///
/// The same rule the Add menu keeps: a node the tree cannot draw is a node the
/// selection drops, and a condition that appears folded up inside the row that
/// was just switched reads as the switch having done nothing.
fn open_up(world: &mut World, node: Entity) {
    let mut open = Some(node);
    while let Some(at) = open {
        world.entity_mut(at).insert(Expanded);
        open = world.get::<ChildOf>(at).map(ChildOf::parent);
    }
}

/// The children of `node` that are expression nodes, in authored order.
fn expression_children(world: &World, node: Entity) -> Vec<Entity> {
    let Some(children) = world.get::<Children>(node) else {
        return Vec::new();
    };
    let mut found: Vec<(u64, Entity)> = children
        .iter()
        .filter(|child| world.get::<ExpressionNode>(*child).is_some())
        .filter_map(|child| Some((id_order(&world.get::<NodeId>(child)?.0).1, child)))
        .collect();
    found.sort_unstable();
    found.into_iter().map(|(_, child)| child).collect()
}

/// Switch an action to another kind.
///
/// NOTHING survives the switch. A sequence's beats were the sequence and a
/// variable set's expression was the value it wrote, so neither means anything
/// to the kind arriving - and the kind arriving brings the children it needs.
fn retype_action(world: &mut World, node: Entity, choice: ActionChoice) {
    let Some(mut action) = world.get_mut::<ActionNode>(node) else {
        return;
    };
    if action_choice(&action.kind) == choice {
        return;
    }
    action.kind = choice.stock();
    rename(world, node, "Action", choice.stem());
    drop_children(world, node, 0);
    // A variable set arrives WRITING something, for the reason an expression
    // filter arrives comparing something: an assignment with no value is an
    // action the next save would drop.
    if choice == ActionChoice::VariableSet {
        let mut commands = world.commands();
        lift_expression(&mut commands, node, 1, number(0.0));
        world.flush();
    }
}

/// Rename a node to the stem its new kind is called by, keeping its ordinal.
///
/// The ordinal is what the lowering SORTS by, so it survives; the stem is what
/// the row reads, so it changes. A filter switched from `entity_2` to `timer_2`
/// stays the second filter of its handler.
fn rename(world: &mut World, node: Entity, kind: &str, stem: &str) {
    let Some(ordinal) = world
        .get::<NodeId>(node)
        .map(|id| split_ordinal(&id.0).1.to_string())
    else {
        return;
    };
    let id = if ordinal.is_empty() {
        stem.to_string()
    } else {
        format!("{stem}_{ordinal}")
    };
    world
        .entity_mut(node)
        .insert((NodeId(id.clone()), Name::new(format!("{kind} Node {id}"))));
}

/// Despawn every child past the first `keep`, in authored order.
fn drop_children(world: &mut World, node: Entity, keep: usize) {
    let Some(children) = world.get::<Children>(node).map(|kids| kids.to_vec()) else {
        return;
    };
    let mut ordered: Vec<(u64, Entity)> = children
        .into_iter()
        .filter_map(|child| Some((id_order(&world.get::<NodeId>(child)?.0).1, child)))
        .collect();
    ordered.sort_unstable();
    for (_, child) in ordered.into_iter().skip(keep) {
        world.entity_mut(child).despawn();
    }
}

/// What an Add row makes while the EVENTS mode is showing.
///
/// SIX rows for a vocabulary of forty-eight, because the kind is mostly not
/// chosen here: a fresh filter arrives matching entities and a fresh action
/// arrives logging a line, and the panel's own Filter and Action rows switch
/// either to any of the rest. A menu with one row per action would be a list
/// nothing on this screen is tall enough to show.
///
/// SEQUENCE is the exception, and it earns its row: it is the one action that
/// takes children, so a builder who does not already know it is an `Action`
/// switched to `Sequence` has no way to guess that steps and gates exist at
/// all.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScriptAdd {
    /// One more handler, under the script.
    Handler,
    /// One more filter, under whatever takes filters.
    Filter,
    /// One more action, under whatever takes actions.
    Action,
    /// One more action, already switched to the chain of beats.
    Sequence,
    /// One more beat of a sequence.
    Step,
    /// The event a beat waits for.
    Gate,
}

impl ScriptAdd {
    /// Every script row of the Add menu, in the order it lists them.
    pub(crate) const ALL: [ScriptAdd; 6] = [
        ScriptAdd::Handler,
        ScriptAdd::Filter,
        ScriptAdd::Action,
        ScriptAdd::Sequence,
        ScriptAdd::Step,
        ScriptAdd::Gate,
    ];

    /// The row label: the word the tree, the panel and the docs all use.
    pub(crate) fn label(self) -> &'static str {
        match self {
            ScriptAdd::Handler => "Handler",
            ScriptAdd::Filter => "Filter",
            ScriptAdd::Action => "Action",
            ScriptAdd::Sequence => "Sequence",
            ScriptAdd::Step => "Step",
            ScriptAdd::Gate => "Gate",
        }
    }

    /// What the row MAKES, in the menu's right-hand column.
    ///
    /// The label is the vocabulary and this is what it means. `Handler` and
    /// `Gate` are words a builder cannot rank until something says which one
    /// runs first and which one waits - and the menu is where they are read,
    /// before there is anything on screen to hover.
    pub(crate) fn hint(self) -> &'static str {
        match self {
            ScriptAdd::Handler => "on an event",
            ScriptAdd::Filter => "only if",
            ScriptAdd::Action => "do this",
            ScriptAdd::Sequence => "paced beats",
            ScriptAdd::Step => "one beat",
            ScriptAdd::Gate => "waits for",
        }
    }
}

/// The fresh filter every Add > Filter starts from.
const NEW_FILTER: FilterChoice = FilterChoice::Entity;
/// The fresh action every Add > Action starts from: the one that changes
/// nothing until it is authored.
const NEW_ACTION: ActionChoice = ActionChoice::DebugMessage;

/// Where a new node of this kind would go, given what is marked.
///
/// `None` is the whole of "the row is greyed": the menu asks this to draw
/// itself and the verb asks it again to do the work, so a row that can be
/// pressed is a row whose node lands somewhere.
///
/// The selection does not have to be the PARENT: the answer is the NEAREST
/// node above it that takes one. A filter marked under a handler adds the next
/// filter to that handler, so three filters is three presses rather than three
/// presses and two reselections.
pub(crate) fn add_parent(
    add: ScriptAdd,
    marked: Option<Entity>,
    script: &ScriptNodes,
    scenario: Entity,
) -> Option<Entity> {
    let root = script.root(scenario)?;
    let node = marked.filter(|node| script.holds(*node));
    match add {
        ScriptAdd::Handler => Some(root),
        ScriptAdd::Filter => climb(node?, script, filter_home),
        ScriptAdd::Action | ScriptAdd::Sequence => climb(node?, script, action_home),
        ScriptAdd::Step => climb(node?, script, sequence_home),
        ScriptAdd::Gate => climb(node?, script, gate_home),
    }
}

/// The first node from `node` up that `takes` the new one.
///
/// Up through the SCRIPT only: the climb stops at the handler, never at the
/// script node or the scenario, so a filter cannot land somewhere no row would
/// show it.
fn climb(
    node: Entity,
    script: &ScriptNodes,
    takes: fn(Entity, &ScriptNodes) -> Option<Entity>,
) -> Option<Entity> {
    let mut at = Some(node);
    while let Some(node) = at {
        if let Some(home) = takes(node, script) {
            return Some(home);
        }
        at = script.owner(node).filter(|owner| script.holds(*owner));
    }
    None
}

/// `node` itself, if a filter can hang from it.
///
/// A combinator only takes what it can hold: a third operand under an `And`
/// would be dropped by the lowering, so the row is greyed instead of offering
/// a filter the save would silently lose.
fn filter_home(node: Entity, script: &ScriptNodes) -> Option<Entity> {
    if script.event(node).is_some() || script.gate(node).is_some() {
        return Some(node);
    }
    let filter = script.filter(node)?;
    let room = filter_choice(&filter.kind).operands();
    (script.filters_of(node).len() < room).then_some(node)
}

/// `node` itself, if an action can hang from it.
fn action_home(node: Entity, script: &ScriptNodes) -> Option<Entity> {
    (script.event(node).is_some() || script.step(node).is_some()).then_some(node)
}

/// `node` itself, if it is the sequence a beat would join.
fn sequence_home(node: Entity, script: &ScriptNodes) -> Option<Entity> {
    let action = script.action(node)?;
    matches!(action.kind, ActionKind::Sequence(_)).then_some(node)
}

/// `node` itself, if it is a beat still waiting for nothing.
fn gate_home(node: Entity, script: &ScriptNodes) -> Option<Entity> {
    (script.step(node).is_some() && script.gate_of(node).is_none()).then_some(node)
}

/// Add > Handler, Filter, Action, Step or Gate: one more node, and mark it.
///
/// Marked because the panel is where the new node is authored, and the panel
/// shows what is marked - the same handover Add > Asteroid makes.
pub(crate) fn add_script_node(
    activate: On<Activate>,
    mut commands: Commands,
    rows: Query<&ScriptAdd>,
    context: Res<EditContext>,
    script: ScriptNodes,
    mut ordinals: Query<&mut NextChildOrdinal>,
    mut marked: ResMut<SelectedNode>,
) {
    let Ok(&add) = rows.get(activate.entity) else {
        return;
    };
    let Some(scenario) = context.scenario() else {
        return;
    };
    let Some(parent) = add_parent(add, marked.0, &script, scenario) else {
        return;
    };
    // Open every container between the root and the new node. A node added
    // into a collapsed parent has no row, and the tree drops a selection it
    // cannot draw - so the beat a builder just made would vanish from the
    // panel the moment it was made.
    let mut open = Some(parent);
    while let Some(node) = open {
        commands.entity(node).insert(Expanded);
        open = script.owner(node);
    }
    marked.0 = Some(spawn_script_node(&mut commands, &mut ordinals, parent, add));
}

/// Spawn one script node under `parent`, with an id minted from the parent's
/// own counter - the same counter [`lift`] seeds, so a loaded document and an
/// authored one number their children alike.
fn spawn_script_node(
    commands: &mut Commands,
    ordinals: &mut Query<&mut NextChildOrdinal>,
    parent: Entity,
    add: ScriptAdd,
) -> Entity {
    match add {
        ScriptAdd::Handler => {
            let id = mint_id(ordinals, parent, "event");
            spawn_node(commands, parent, "Event", id, EventNode::default())
        }
        ScriptAdd::Filter => {
            let id = mint_id(ordinals, parent, NEW_FILTER.stem());
            spawn_node(
                commands,
                parent,
                "Filter",
                id,
                FilterNode {
                    kind: NEW_FILTER.stock(),
                },
            )
        }
        ScriptAdd::Action | ScriptAdd::Sequence => {
            let choice = if add == ScriptAdd::Sequence {
                ActionChoice::Sequence
            } else {
                NEW_ACTION
            };
            let id = mint_id(ordinals, parent, choice.stem());
            spawn_node(
                commands,
                parent,
                "Action",
                id,
                ActionNode {
                    kind: choice.stock(),
                },
            )
        }
        ScriptAdd::Step => {
            let id = mint_id(ordinals, parent, "step");
            spawn_node(commands, parent, "Step", id, StepNode::default())
        }
        ScriptAdd::Gate => {
            let id = mint_id(ordinals, parent, "gate");
            spawn_node(commands, parent, "Gate", id, GateNode::default())
        }
    }
}

/// The six spawns' one shape: a node of the script, childless, under `parent`.
fn spawn_node<T: Component>(
    commands: &mut Commands,
    parent: Entity,
    kind: &str,
    id: NodeId,
    held: T,
) -> Entity {
    commands
        .spawn((
            EditorNode,
            held,
            Name::new(format!("{kind} Node {}", id.0)),
            id,
            NextChildOrdinal(0),
            ChildOf(parent),
        ))
        .id()
}

#[cfg(test)]
mod tests;
