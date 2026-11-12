//! The script as PUZZLE PIECES: which block fits which slot, and the verbs that
//! put one there.
//!
//! The canvas is a view over the nodes in [`super`]. This module holds the half
//! of it that has no pixels in it, so every rule a drag obeys is a rule a test
//! can ask without a window. A verb writes nodes and returns; the caller
//! records the undo step, so one gesture is one step however many nodes it
//! touched.
//!
//! ORDER IS THE ORDINAL. [`ScriptNodes::ordered`] sorts siblings by the number
//! in their id, so a reorder hands the siblings their own sorted ordinals back
//! in the new order. A block arriving from another parent first takes a fresh
//! ordinal from that parent's counter, which only rises: no id in a parent is
//! ever minted twice, and a counter is never rewound by a move.
//!
//! A BLOCK NEVER STANDS LOOSE. A stack with no handler has no lowering, so only
//! a handler may be dropped on bare canvas. Everything else lands in a slot or
//! is refused and goes back where it was: no second document of parked blocks.
//!
//! A VALUE SOCKET IS NEVER EMPTY. The literal in it is the socket's own field,
//! so a reporter dropped on one replaces the literal, and a reporter pulled out
//! leaves a stock `0` behind. A reporter dropped on another reporter is refused
//! rather than popped out, because a popped reporter would have nowhere to go.

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use nova_scenario::prelude::*;

use super::{
    drop_children, lift_condition, lift_expression, number, open_up, spawn_expression, spawn_node,
    split_ordinal, ActionChoice, ActionChoiceExt, ActionKind, ActionNode, EventNode, ExprChoice,
    ExprKind, ExpressionNode, FilterChoice, FilterKind, FilterNode, GateNode, ScriptNodes,
    StepNode, ValueOperand,
};
use crate::{
    history::NodeCopy,
    inspect::variant_hint,
    node::{id_order, NextChildOrdinal, NodeId},
};

/// The deadline a gate arrives with when its step has none, in seconds.
///
/// A gate with no deadline is a lint error, because a gate that never opens is a
/// soft-lock. Long enough that no ordinary beat trips it, short enough that a
/// forgotten gate fails during a test run.
pub(crate) const NEW_GATE_DEADLINE: f64 = 120.0;

/// What a block is to the slots around it: the outline of the puzzle piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockShapeType {
    /// A handler: stands on the canvas, holds a stack.
    Hat,
    /// An action: joins a stack.
    Stack,
    /// A beat of a sequence or a cinematic.
    Step,
    /// What a beat waits for.
    Gate,
    /// A filter: fits a boolean socket.
    Boolean,
    /// A comparison: a boolean that arrives wrapped in an expression filter.
    Comparison,
    /// A value: fits a value socket.
    Reporter,
}

/// What a block does in a script: its palette group and its colour, in the
/// order a script is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum BlockRoleType {
    /// A handler: the hat a script starts with.
    Event,
    /// A filter that gates a handler or a wait.
    Condition,
    /// A block that does something.
    Action,
    /// A block that holds beats, and the beats and waits inside it.
    Flow,
    /// A value, a comparison, and the logic that joins conditions.
    Value,
}

impl BlockRoleType {
    /// Every group, in build order.
    pub(crate) const ALL: [BlockRoleType; 5] = [
        BlockRoleType::Event,
        BlockRoleType::Condition,
        BlockRoleType::Action,
        BlockRoleType::Flow,
        BlockRoleType::Value,
    ];

    /// The group's heading.
    pub(crate) fn label(self) -> &'static str {
        match self {
            BlockRoleType::Event => "Events",
            BlockRoleType::Condition => "Conditions",
            BlockRoleType::Action => "Actions",
            BlockRoleType::Flow => "Flow",
            BlockRoleType::Value => "Values and logic",
        }
    }

    /// The group's name on the palette's rail.
    pub(crate) fn short(self) -> &'static str {
        match self {
            BlockRoleType::Event => "Events",
            BlockRoleType::Condition => "Conditions",
            BlockRoleType::Action => "Actions",
            BlockRoleType::Flow => "Flow",
            BlockRoleType::Value => "Values",
        }
    }

    /// The few words after the heading: what the group is for.
    pub(crate) fn note(self) -> &'static str {
        match self {
            BlockRoleType::Event => "start a script",
            BlockRoleType::Condition => "only if",
            BlockRoleType::Action => "do",
            BlockRoleType::Flow => "steps and waits",
            BlockRoleType::Value => "fill a socket",
        }
    }

    /// The group an action is in: a block that holds beats is flow.
    pub(crate) fn of_action(action: ActionChoice) -> Self {
        match action {
            ActionChoice::Sequence | ActionChoice::Cinematic => BlockRoleType::Flow,
            _ => BlockRoleType::Action,
        }
    }

    /// The group a filter is in: a comparison and the joins are logic.
    pub(crate) fn of_filter(filter: FilterChoice) -> Self {
        match filter {
            FilterChoice::Expression | FilterChoice::Not | FilterChoice::And | FilterChoice::Or => {
                BlockRoleType::Value
            }
            FilterChoice::Entity
            | FilterChoice::Timer
            | FilterChoice::ShipOrder
            | FilterChoice::Cinematic => BlockRoleType::Condition,
        }
    }
}

/// One entry of the palette: a block that does not exist yet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum BlockItemType {
    /// A handler on this trigger.
    Hat(EventConfig),
    /// An action of this kind.
    Action(ActionChoice),
    /// A filter of this kind.
    Filter(FilterChoice),
    /// An operator, a comparison or a literal.
    Expr(ExprChoice),
    /// A scenario variable, read by name.
    Variable,
    /// A field of the event the handler is running for.
    EventField,
    /// One more beat.
    Step,
    /// What a beat waits for.
    Gate,
}

impl BlockItemType {
    /// Every block the palette offers, off the owning tables.
    pub(crate) fn all() -> Vec<BlockItemType> {
        let mut items = vec![BlockItemType::Step, BlockItemType::Gate];
        items.extend(EventConfig::ALL.into_iter().map(BlockItemType::Hat));
        items.extend(ActionChoice::ALL.into_iter().map(BlockItemType::Action));
        items.extend(FilterChoice::ALL.into_iter().map(BlockItemType::Filter));
        items.extend(ExprChoice::ALL.into_iter().map(BlockItemType::Expr));
        items.extend([BlockItemType::Variable, BlockItemType::EventField]);
        items
    }

    /// What the palette calls it.
    pub(crate) fn label(self) -> String {
        match self {
            BlockItemType::Hat(trigger) => trigger.label().to_string(),
            BlockItemType::Action(action) => action.label().to_string(),
            BlockItemType::Filter(filter) => filter.label().to_string(),
            BlockItemType::Expr(ExprChoice::Value) => "Number".to_string(),
            BlockItemType::Expr(ExprChoice::Query) => "Query".to_string(),
            BlockItemType::Expr(expr) => expr.label().to_string(),
            BlockItemType::Variable => "Variable".to_string(),
            BlockItemType::EventField => "Event Field".to_string(),
            BlockItemType::Step => "Step".to_string(),
            BlockItemType::Gate => "Wait For".to_string(),
        }
    }

    /// The palette group it is filed under, which is also its colour.
    pub(crate) fn role(self) -> BlockRoleType {
        match self {
            BlockItemType::Hat(_) => BlockRoleType::Event,
            BlockItemType::Action(action) => BlockRoleType::of_action(action),
            BlockItemType::Filter(filter) => BlockRoleType::of_filter(filter),
            BlockItemType::Expr(_) | BlockItemType::Variable | BlockItemType::EventField => {
                BlockRoleType::Value
            }
            BlockItemType::Step | BlockItemType::Gate => BlockRoleType::Flow,
        }
    }

    /// The subject a handler or an action is about, which orders a group and
    /// which the palette filter also matches. `None` for the rest, which a
    /// subject would not sort.
    pub(crate) fn subject(self) -> Option<BlockCategoryType> {
        match self {
            BlockItemType::Hat(trigger) => Some(trigger.category()),
            BlockItemType::Action(action) => Some(action.category()),
            _ => None,
        }
    }

    /// One line on where the block goes, for the palette's hover.
    pub(crate) fn placement(self) -> &'static str {
        match self.shape() {
            BlockShapeType::Hat => "Drop it on the canvas: it starts a script.",
            BlockShapeType::Stack => match self {
                BlockItemType::Action(ActionChoice::Sequence | ActionChoice::Cinematic) => {
                    "Snap it under a handler or a step. Steps go inside it."
                }
                _ => "Snap it under a handler, or inside a step.",
            },
            BlockShapeType::Step => "Snap it inside a Sequence or a Cinematic.",
            BlockShapeType::Gate => "Snap it on a step's head: the step waits for it.",
            BlockShapeType::Boolean | BlockShapeType::Comparison => {
                "Snap it on a handler's only if line, or in a condition socket."
            }
            BlockShapeType::Reporter => "Drop it on a value socket.",
        }
    }

    /// The outline it drags with.
    pub(crate) fn shape(self) -> BlockShapeType {
        match self {
            BlockItemType::Hat(_) => BlockShapeType::Hat,
            BlockItemType::Action(_) => BlockShapeType::Stack,
            BlockItemType::Filter(_) => BlockShapeType::Boolean,
            BlockItemType::Expr(expr) if expr.compares() => BlockShapeType::Comparison,
            BlockItemType::Expr(_) | BlockItemType::Variable | BlockItemType::EventField => {
                BlockShapeType::Reporter
            }
            BlockItemType::Step => BlockShapeType::Step,
            BlockItemType::Gate => BlockShapeType::Gate,
        }
    }

    /// What the block is for: the doc comment of the row that declares it.
    pub(crate) fn hint(self) -> String {
        match self {
            BlockItemType::Hat(trigger) => variant_hint(&trigger),
            BlockItemType::Action(action) => variant_hint(&action),
            BlockItemType::Filter(filter) => variant_hint(&filter),
            BlockItemType::Expr(expr) => variant_hint(&expr),
            BlockItemType::Variable => "Read a scenario variable by name.".to_string(),
            BlockItemType::EventField => "Read a field of the event this handler runs for, \
                                          such as event.id. Not inside a step's actions."
                .to_string(),
            BlockItemType::Step => "One beat: waits, then runs the actions in it.".to_string(),
            BlockItemType::Gate => "Hold this beat until an event happens.".to_string(),
        }
    }
}

/// A place a block can land.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum BlockSlotType {
    /// Bare canvas, at this canvas position.
    Canvas(Vec2),
    /// Between the actions of a handler or a step.
    Action {
        /// The handler or step.
        parent: Entity,
        /// The action this one would stand before.
        index: usize,
    },
    /// Between the beats of a sequence or a cinematic.
    Step {
        /// The sequence or cinematic.
        parent: Entity,
        /// The beat this one would stand before.
        index: usize,
    },
    /// A beat's gate.
    Gate {
        /// The beat.
        step: Entity,
    },
    /// Between the filters of a handler or a gate.
    Filter {
        /// The handler or gate.
        parent: Entity,
        /// The filter this one would stand before.
        index: usize,
    },
    /// One boolean socket of a combinator.
    Operand {
        /// The `Not`, `And` or `Or`.
        parent: Entity,
        /// Which socket.
        index: usize,
    },
    /// One value socket of an operator, a comparison or a variable set.
    Value {
        /// The node holding the socket.
        parent: Entity,
        /// Which socket.
        index: usize,
    },
}

impl BlockSlotType {
    /// The node this slot hangs from, or `None` for bare canvas.
    pub(crate) fn parent(self) -> Option<Entity> {
        match self {
            BlockSlotType::Canvas(_) => None,
            BlockSlotType::Gate { step } => Some(step),
            BlockSlotType::Action { parent, .. }
            | BlockSlotType::Step { parent, .. }
            | BlockSlotType::Filter { parent, .. }
            | BlockSlotType::Operand { parent, .. }
            | BlockSlotType::Value { parent, .. } => Some(parent),
        }
    }
}

/// The outline of an existing node, or `None` for a node no drag can carry.
pub(crate) fn shape_of(script: &ScriptNodes, node: Entity) -> Option<BlockShapeType> {
    if script.event(node).is_some() {
        return Some(BlockShapeType::Hat);
    }
    if script.action(node).is_some() {
        return Some(BlockShapeType::Stack);
    }
    if script.step(node).is_some() {
        return Some(BlockShapeType::Step);
    }
    if script.gate(node).is_some() {
        return Some(BlockShapeType::Gate);
    }
    if script.filter(node).is_some() {
        return Some(BlockShapeType::Boolean);
    }
    script.expression(node).map(|_| BlockShapeType::Reporter)
}

/// Whether a value socket's occupant is the socket's own field rather than a
/// block: a typed literal, which a reporter may land on.
pub(crate) fn is_literal(kind: &ExprKind) -> bool {
    matches!(
        kind,
        ExprKind::Value(ValueOperand {
            value: VariableExpressionNode::Term(VariableTermNode::Factor(
                VariableFactorNode::Literal(_)
            )),
        })
    )
}

/// Why `shape` cannot land in `slot`, or `None` where it fits.
///
/// ONE answer for every surface: the canvas lights a slot off this, the drop
/// refuses off it, and the verbs ask it again, so a slot that lights is a slot
/// that takes the block.
pub(crate) fn refusal(
    script: &ScriptNodes,
    shape: BlockShapeType,
    moving: Option<Entity>,
    slot: BlockSlotType,
) -> Option<String> {
    let wrong = match slot {
        BlockSlotType::Canvas(_) => (shape != BlockShapeType::Hat)
            .then_some("only an Events block stands alone: drop this on a lit slot"),
        BlockSlotType::Action { .. } => {
            (shape != BlockShapeType::Stack).then_some("only an action fits in a stack")
        }
        BlockSlotType::Step { .. } => {
            (shape != BlockShapeType::Step).then_some("only a step fits in a sequence")
        }
        BlockSlotType::Gate { .. } => {
            (shape != BlockShapeType::Gate).then_some("only a Wait For fits here")
        }
        BlockSlotType::Filter { .. } | BlockSlotType::Operand { .. } => {
            (!matches!(shape, BlockShapeType::Boolean | BlockShapeType::Comparison))
                .then_some("only a condition fits here")
        }
        BlockSlotType::Value { .. } => {
            (shape != BlockShapeType::Reporter).then_some("only a value fits here")
        }
    };
    if let Some(reason) = wrong {
        return Some(reason.to_string());
    }
    if let (Some(moving), Some(mut at)) = (moving, slot.parent()) {
        loop {
            if at == moving {
                return Some("a block cannot go inside itself".to_string());
            }
            match script.owner(at).filter(|owner| script.holds(*owner)) {
                Some(owner) => at = owner,
                None => break,
            }
        }
    }
    match slot {
        BlockSlotType::Gate { step } => script
            .gate_of(step)
            .filter(|gate| Some(*gate) != moving)
            .map(|_| "this step already waits: drag its Wait For out first".to_string()),
        BlockSlotType::Operand { parent, index } => {
            let held = script.filters_of(parent);
            let staying = held.iter().filter(|node| Some(**node) != moving).count();
            let room = script
                .filter(parent)
                .and_then(|filter| operand_room(&filter.kind));
            let taken = index < held.len() && Some(held[index]) != moving;
            (taken || room.is_some_and(|room| staying >= room))
                .then(|| "this socket is full: drag its block out first".to_string())
        }
        BlockSlotType::Value { parent, index } => {
            let held = *script.operands_of(parent).get(index)?;
            if Some(held) == moving {
                return Some("a block cannot go inside itself".to_string());
            }
            let literal = script
                .expression(held)
                .is_some_and(|expression| is_literal(&expression.kind));
            (!literal).then(|| "this socket is full: drag its block out first".to_string())
        }
        _ => None,
    }
}

/// How many conditions `kind` lowers. `None` for a kind that holds none.
fn operand_room(kind: &FilterKind) -> Option<usize> {
    match kind {
        FilterKind::Not => Some(1),
        FilterKind::And | FilterKind::Or => Some(2),
        _ => None,
    }
}

/// [`refusal`], as an error, from outside a system.
fn fits(
    world: &mut World,
    shape: BlockShapeType,
    moving: Option<Entity>,
    slot: BlockSlotType,
) -> Result<(), String> {
    let refused = world
        .run_system_once(move |script: ScriptNodes| refusal(&script, shape, moving, slot))
        .map_err(|error| error.to_string())?;
    refused.map_or(Ok(()), Err)
}

/// The children of `parent` holding `T`, in authored order.
fn siblings<T: Component>(world: &World, parent: Entity) -> Vec<Entity> {
    let Some(children) = world.get::<Children>(parent) else {
        return Vec::new();
    };
    let mut found: Vec<(u64, Entity)> = children
        .iter()
        .filter(|child| world.get::<T>(*child).is_some())
        .filter_map(|child| Some((id_order(&world.get::<NodeId>(child)?.0).1, child)))
        .collect();
    found.sort_unstable();
    found.into_iter().map(|(_, child)| child).collect()
}

/// The word a script node's `Name` starts with.
fn kind_word(world: &World, node: Entity) -> &'static str {
    if world.get::<ActionNode>(node).is_some() {
        "Action"
    } else if world.get::<FilterNode>(node).is_some() {
        "Filter"
    } else if world.get::<StepNode>(node).is_some() {
        "Step"
    } else if world.get::<GateNode>(node).is_some() {
        "Gate"
    } else if world.get::<ExpressionNode>(node).is_some() {
        "Expression"
    } else {
        "Event"
    }
}

/// Give `node` this ordinal, keeping its stem.
fn set_ordinal(world: &mut World, node: Entity, ordinal: u64) {
    let Some(id) = world.get::<NodeId>(node).map(|id| id.0.clone()) else {
        return;
    };
    let id = format!("{}_{ordinal}", split_ordinal(&id).0);
    let name = format!("{} Node {id}", kind_word(world, node));
    world.entity_mut(node).insert((NodeId(id), Name::new(name)));
}

/// The next ordinal under `parent`, which is never one it has handed out.
pub(super) fn next_ordinal(world: &mut World, parent: Entity) -> u32 {
    let mut counter = world
        .get_mut::<NextChildOrdinal>(parent)
        .expect("every script node carries a counter");
    counter.0 += 1;
    counter.0
}

/// Hand `wanted` its own sorted ordinals in this order: a permutation, so no
/// ordinal is minted and none is reused.
fn reorder(world: &mut World, wanted: &[Entity]) {
    let mut ordinals: Vec<u64> = wanted
        .iter()
        .filter_map(|node| Some(id_order(&world.get::<NodeId>(*node)?.0).1))
        .collect();
    ordinals.sort_unstable();
    for (node, ordinal) in wanted.iter().zip(ordinals) {
        let held = world.get::<NodeId>(*node).map(|id| id_order(&id.0).1);
        if held != Some(ordinal) {
            set_ordinal(world, *node, ordinal);
        }
    }
}

/// Bring `node` under `parent` on a fresh ordinal. A no-op for a node already
/// there.
fn arrive(world: &mut World, node: Entity, parent: Entity) {
    if world.get::<ChildOf>(node).map(ChildOf::parent) == Some(parent) {
        return;
    }
    let ordinal = next_ordinal(world, parent);
    set_ordinal(world, node, u64::from(ordinal));
    world.entity_mut(node).insert(ChildOf(parent));
}

/// Stand `run` in `parent`'s `T` list, before the sibling now at `index`.
fn seat<T: Component>(world: &mut World, parent: Entity, run: &[Entity], index: usize) {
    let mut list = siblings::<T>(world, parent);
    let before = list
        .iter()
        .take(index)
        .filter(|node| !run.contains(node))
        .count();
    list.retain(|node| !run.contains(node));
    for node in run {
        arrive(world, *node, parent);
    }
    list.splice(before..before, run.iter().copied());
    reorder(world, &list);
}

/// What a drag of `node` carries: it and the blocks below it, as a stack does.
pub(crate) fn carried(world: &World, node: Entity) -> Vec<Entity> {
    let Some(parent) = world.get::<ChildOf>(node).map(ChildOf::parent) else {
        return vec![node];
    };
    let list = if world.get::<ActionNode>(node).is_some() {
        siblings::<ActionNode>(world, parent)
    } else if world.get::<StepNode>(node).is_some() {
        siblings::<StepNode>(world, parent)
    } else {
        return vec![node];
    };
    let at = list
        .iter()
        .position(|sibling| *sibling == node)
        .unwrap_or(0);
    list[at..].to_vec()
}

/// Whether taking `node` away leaves its socket holding a stock `0`: a value
/// socket is never empty.
fn refill_socket(world: &mut World, node: Entity) {
    if world.get::<ExpressionNode>(node).is_none() {
        return;
    }
    let Some(parent) = world.get::<ChildOf>(node).map(ChildOf::parent) else {
        return;
    };
    let mut list = siblings::<ExpressionNode>(world, parent);
    let Some(at) = list.iter().position(|sibling| *sibling == node) else {
        return;
    };
    let ordinal = next_ordinal(world, parent);
    let mut commands = world.commands();
    let stock = lift_expression(&mut commands, parent, ordinal, number(0.0));
    world.flush();
    // The stock literal takes the leaver's place in the order; the leaver's own
    // ordinal stays out of the permutation with it.
    list[at] = stock;
    reorder(world, &list);
}

/// Carry `node`, and what stands below it, into `slot`.
///
/// Nothing changes on a refusal. The caller records the undo step.
pub(crate) fn move_block(
    world: &mut World,
    node: Entity,
    slot: BlockSlotType,
) -> Result<(), String> {
    let shape = world
        .run_system_once(move |script: ScriptNodes| shape_of(&script, node))
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "this is not a block".to_string())?;
    fits(world, shape, Some(node), slot)?;
    let run = carried(world, node);
    match slot {
        BlockSlotType::Canvas(at) => {
            let mut event = world
                .get_mut::<EventNode>(node)
                .ok_or_else(|| "only a handler stands alone".to_string())?;
            event.position = Some(at);
        }
        BlockSlotType::Action { parent, index } => seat::<ActionNode>(world, parent, &run, index),
        BlockSlotType::Step { parent, index } => seat::<StepNode>(world, parent, &run, index),
        BlockSlotType::Filter { parent, index } | BlockSlotType::Operand { parent, index } => {
            seat::<FilterNode>(world, parent, &run, index);
        }
        BlockSlotType::Gate { step } => {
            arrive(world, node, step);
            give_a_deadline(world, step);
        }
        BlockSlotType::Value { parent, index } => {
            // Named before the refill: a leaver from this same operator moves
            // the sockets the index counts.
            let target = siblings::<ExpressionNode>(world, parent)
                .get(index)
                .copied();
            refill_socket(world, node);
            fill_socket(world, parent, target, node);
        }
    }
    if let Some(parent) = slot.parent() {
        open_up(world, parent);
    }
    Ok(())
}

/// Put `node` under `parent` in the place of `target`, or last with none.
fn fill_socket(world: &mut World, parent: Entity, target: Option<Entity>, node: Entity) {
    let mut list = siblings::<ExpressionNode>(world, parent);
    list.retain(|sibling| *sibling != node);
    arrive(world, node, parent);
    match target.and_then(|old| list.iter().position(|sibling| *sibling == old)) {
        Some(at) => {
            world.entity_mut(list[at]).despawn();
            list[at] = node;
        }
        None => list.push(node),
    }
    reorder(world, &list);
}

/// A gate with no deadline is a lint error, so a step that gains one gets the
/// stock deadline unless it already has its own.
fn give_a_deadline(world: &mut World, step: Entity) {
    if let Some(mut step) = world.get_mut::<StepNode>(step) {
        if step.deadline.is_none() {
            step.deadline = Some(NEW_GATE_DEADLINE);
        }
    }
}

/// Stand a new `item` in `slot`, and answer the node made.
///
/// Nothing changes on a refusal. The caller records the undo step.
pub(crate) fn place_new(
    world: &mut World,
    item: BlockItemType,
    slot: BlockSlotType,
) -> Result<Entity, String> {
    fits(world, item.shape(), None, slot)?;
    let (parent, index) = match slot {
        BlockSlotType::Canvas(_) => (None, 0),
        BlockSlotType::Gate { step } => (Some(step), 0),
        BlockSlotType::Action { parent, index }
        | BlockSlotType::Step { parent, index }
        | BlockSlotType::Filter { parent, index }
        | BlockSlotType::Operand { parent, index }
        | BlockSlotType::Value { parent, index } => (Some(parent), index),
    };
    let made = match (item, slot) {
        (BlockItemType::Hat(trigger), BlockSlotType::Canvas(at)) => place_hat(world, trigger, at)?,
        (BlockItemType::Action(action), BlockSlotType::Action { parent, .. }) => {
            let made = new_action(world, parent, action);
            seat::<ActionNode>(world, parent, &[made], index);
            made
        }
        (BlockItemType::Step, BlockSlotType::Step { parent, .. }) => {
            let made = new_step(world, parent);
            seat::<StepNode>(world, parent, &[made], index);
            made
        }
        (BlockItemType::Gate, BlockSlotType::Gate { step }) => {
            let ordinal = next_ordinal(world, step);
            let made = spawned(
                world,
                step,
                "Gate",
                format!("gate_{ordinal}"),
                GateNode::default(),
            );
            give_a_deadline(world, step);
            made
        }
        (
            BlockItemType::Filter(_) | BlockItemType::Expr(_),
            BlockSlotType::Filter { parent, .. } | BlockSlotType::Operand { parent, .. },
        ) => {
            let made = new_filter(world, parent, item);
            seat::<FilterNode>(world, parent, &[made], index);
            made
        }
        (_, BlockSlotType::Value { parent, .. }) => {
            let ordinal = next_ordinal(world, parent);
            let made = new_reporter(world, parent, ordinal, item);
            // Spawned beside the literal it replaces, so the socket fill sees a
            // node already home and only has to take the literal away.
            let list: Vec<Entity> = siblings::<ExpressionNode>(world, parent)
                .into_iter()
                .filter(|node| *node != made)
                .collect();
            let mut wanted = list.clone();
            match list.get(index) {
                Some(old) => {
                    wanted[index] = made;
                    world.entity_mut(*old).despawn();
                }
                None => wanted.push(made),
            }
            reorder(world, &wanted);
            made
        }
        _ => return Err("this block does not fit here".to_string()),
    };
    if let Some(parent) = parent {
        open_up(world, parent);
    }
    Ok(made)
}

/// One childless script node under `parent`, by world.
fn spawned<T: Component>(
    world: &mut World,
    parent: Entity,
    kind: &str,
    id: String,
    held: T,
) -> Entity {
    let mut commands = world.commands();
    let node = spawn_node(&mut commands, parent, kind, NodeId(id), held);
    world.flush();
    node
}

/// A handler at `at`.
fn place_hat(world: &mut World, trigger: EventConfig, at: Vec2) -> Result<Entity, String> {
    let root = world
        .query_filtered::<Entity, With<super::ScriptNode>>()
        .iter(world)
        .next()
        .ok_or_else(|| "this document has no script".to_string())?;
    let ordinal = next_ordinal(world, root);
    Ok(spawned(
        world,
        root,
        "Event",
        format!("event_{ordinal}"),
        EventNode {
            trigger,
            position: Some(at),
            ..default()
        },
    ))
}

/// A fresh action of `kind` under `parent`, holding what its kind must hold.
fn new_action(world: &mut World, parent: Entity, kind: ActionChoice) -> Entity {
    let ordinal = next_ordinal(world, parent);
    let made = spawned(
        world,
        parent,
        "Action",
        format!("{}_{ordinal}", kind.stem()),
        ActionNode { kind: kind.stock() },
    );
    if kind == ActionChoice::VariableSet {
        let mut commands = world.commands();
        lift_expression(&mut commands, made, 1, number(0.0));
        world.flush();
        bump_to(world, made, 1);
    }
    if matches!(kind, ActionChoice::Sequence | ActionChoice::Cinematic) {
        // A chain with no beats is a lint error, and a C-block with no inside
        // would not show where the first action goes.
        new_step(world, made);
    }
    made
}

/// Raise `node`'s counter to `ordinal`, for children spawned past it.
fn bump_to(world: &mut World, node: Entity, ordinal: u32) {
    if let Some(mut counter) = world.get_mut::<NextChildOrdinal>(node) {
        counter.0 = counter.0.max(ordinal);
    }
}

/// A fresh beat under `parent`.
fn new_step(world: &mut World, parent: Entity) -> Entity {
    let ordinal = next_ordinal(world, parent);
    spawned(
        world,
        parent,
        "Step",
        format!("step_{ordinal}"),
        StepNode::default(),
    )
}

/// A fresh filter under `parent`. A comparison arrives wrapped in the expression
/// filter that holds it, because a comparison is not a filter by itself.
fn new_filter(world: &mut World, parent: Entity, item: BlockItemType) -> Entity {
    let choice = match item {
        BlockItemType::Filter(choice) => choice,
        _ => FilterChoice::Expression,
    };
    let ordinal = next_ordinal(world, parent);
    let made = spawned(
        world,
        parent,
        "Filter",
        format!("{}_{ordinal}", choice.stem()),
        FilterNode {
            kind: choice.stock(),
        },
    );
    if choice == FilterChoice::Expression {
        let (left, right) = (number(0.0), number(0.0));
        let condition = match item {
            BlockItemType::Expr(ExprChoice::LessThan) => {
                VariableConditionNode::new_less_than(left, right)
            }
            BlockItemType::Expr(ExprChoice::GreaterThan) => {
                VariableConditionNode::new_greater_than(left, right)
            }
            _ => VariableConditionNode::new_equals(left, right),
        };
        let mut commands = world.commands();
        lift_condition(&mut commands, made, 1, condition);
        world.flush();
        bump_to(world, made, 1);
    }
    made
}

/// A fresh value under `parent`.
fn new_reporter(world: &mut World, parent: Entity, ordinal: u32, item: BlockItemType) -> Entity {
    let leaf = |factor: VariableFactorNode| {
        ExprKind::Value(ValueOperand {
            value: VariableExpressionNode::new_term(VariableTermNode::new_factor(factor)),
        })
    };
    let (kind, operands) = match item {
        BlockItemType::Expr(choice) => (choice.stock(), choice.operands()),
        BlockItemType::Variable => (leaf(VariableFactorNode::new_name(String::new())), 0),
        // `id` is the field most payloads carry, so the stock block reads
        // true under most hats; the lint names the fields of the others.
        BlockItemType::EventField => (
            leaf(VariableFactorNode::new_event_field(
                nova_events::prelude::ENTITY_ID_COMPONENT_NAME,
            )),
            0,
        ),
        BlockItemType::Hat(_)
        | BlockItemType::Action(_)
        | BlockItemType::Filter(_)
        | BlockItemType::Step
        | BlockItemType::Gate => {
            unreachable!("`place_new` refuses every shape but a reporter in a value socket")
        }
    };
    let mut commands = world.commands();
    let made = spawn_expression(&mut commands, parent, ordinal, kind, operands);
    for side in 1..=operands {
        lift_expression(
            &mut commands,
            made,
            u32::try_from(side).unwrap_or(u32::MAX),
            number(0.0),
        );
    }
    world.flush();
    made
}

/// Take `node`, and what stands below it, off the canvas.
///
/// A reporter leaves a stock `0` in its socket. The caller records the undo step.
pub(crate) fn delete_block(world: &mut World, node: Entity) {
    refill_socket(world, node);
    for block in carried(world, node) {
        drop_children(world, block, 0);
        world.entity_mut(block).despawn();
    }
}

/// Open every container above `node`, so the tree can draw a row for it.
pub(crate) fn reveal(world: &mut World, node: Entity) {
    open_up(world, node);
}

/// The slot beside `anchor` that a block of `shape` would take: right after it
/// where it is a sibling, at the end of it where it is the container.
pub(crate) fn slot_after(
    script: &ScriptNodes,
    anchor: Entity,
    shape: BlockShapeType,
) -> Option<BlockSlotType> {
    let after = |list: Vec<Entity>| {
        list.iter()
            .position(|node| *node == anchor)
            .map(|at| at + 1)
    };
    match shape {
        BlockShapeType::Hat => {
            let held = script.event(anchor)?.position.unwrap_or_default();
            Some(BlockSlotType::Canvas(held + PASTE_STEP))
        }
        BlockShapeType::Stack => {
            if script.action(anchor).is_some() {
                let parent = script.owner(anchor)?;
                let index = after(script.actions_of(parent))?;
                return Some(BlockSlotType::Action { parent, index });
            }
            (script.event(anchor).is_some() || script.step(anchor).is_some()).then(|| {
                BlockSlotType::Action {
                    parent: anchor,
                    index: script.actions_of(anchor).len(),
                }
            })
        }
        BlockShapeType::Step => {
            if script.step(anchor).is_some() {
                let parent = script.owner(anchor)?;
                let index = after(script.steps_of(parent))?;
                return Some(BlockSlotType::Step { parent, index });
            }
            let chain = matches!(
                script.action(anchor)?.kind,
                ActionKind::Sequence(_) | ActionKind::Cinematic(_)
            );
            chain.then(|| BlockSlotType::Step {
                parent: anchor,
                index: script.steps_of(anchor).len(),
            })
        }
        BlockShapeType::Gate => script
            .step(anchor)
            .map(|_| BlockSlotType::Gate { step: anchor }),
        BlockShapeType::Boolean | BlockShapeType::Comparison => {
            if script.event(anchor).is_some() || script.gate(anchor).is_some() {
                return Some(BlockSlotType::Filter {
                    parent: anchor,
                    index: script.filters_of(anchor).len(),
                });
            }
            let parent = script.owner(anchor)?;
            let index = after(script.filters_of(parent))?;
            if script.filter(parent).is_some() {
                Some(BlockSlotType::Operand { parent, index })
            } else {
                Some(BlockSlotType::Filter { parent, index })
            }
        }
        BlockShapeType::Reporter => None,
    }
}

/// How far a pasted handler stands from the one it came from.
const PASTE_STEP: Vec2 = Vec2::new(32.0, 32.0);

/// Stand a copy of a block in `slot`, and answer the node made.
///
/// Nothing changes on a refusal. The caller records the undo step.
pub(crate) fn paste_block(
    world: &mut World,
    copy: &NodeCopy,
    shape: BlockShapeType,
    slot: BlockSlotType,
) -> Result<Entity, String> {
    fits(world, shape, None, slot)?;
    let (parent, index) = match slot {
        BlockSlotType::Canvas(_) => (
            world
                .query_filtered::<Entity, With<super::ScriptNode>>()
                .iter(world)
                .next()
                .ok_or_else(|| "this document has no script".to_string())?,
            0,
        ),
        BlockSlotType::Gate { step } => (step, 0),
        BlockSlotType::Action { parent, index }
        | BlockSlotType::Step { parent, index }
        | BlockSlotType::Filter { parent, index }
        | BlockSlotType::Operand { parent, index }
        | BlockSlotType::Value { parent, index } => (parent, index),
    };
    let made = copy.join_script(world, parent);
    match slot {
        BlockSlotType::Canvas(at) => {
            if let Some(mut event) = world.get_mut::<EventNode>(made) {
                event.position = Some(at);
            }
        }
        BlockSlotType::Action { .. } => seat::<ActionNode>(world, parent, &[made], index),
        BlockSlotType::Step { .. } => seat::<StepNode>(world, parent, &[made], index),
        BlockSlotType::Filter { .. } | BlockSlotType::Operand { .. } => {
            seat::<FilterNode>(world, parent, &[made], index);
        }
        BlockSlotType::Gate { step } => give_a_deadline(world, step),
        BlockSlotType::Value { .. } => {}
    }
    open_up(world, parent);
    Ok(made)
}

/// Everything a builder has not finished, as the node and what it still needs.
///
/// [`ScriptNodes::lower`] cannot spell any of these, so a block listed here is
/// a block the file would lose. This is what makes that loud.
impl ScriptNodes<'_, '_> {
    /// Every block of `scenario` in reading order, operands excluded.
    pub(crate) fn blocks_of(&self, scenario: Entity) -> Vec<Entity> {
        let mut blocks = Vec::new();
        for handler in self.events_of(scenario) {
            blocks.push(handler);
            self.blocks_under(handler, &mut blocks);
        }
        blocks
    }

    fn blocks_under(&self, node: Entity, blocks: &mut Vec<Entity>) {
        let held = self
            .gate_of(node)
            .into_iter()
            .chain(self.filters_of(node))
            .chain(self.actions_of(node))
            .chain(self.steps_of(node));
        for child in held {
            blocks.push(child);
            self.blocks_under(child, blocks);
        }
    }

    /// The block a lint message is about, read off the names it quotes.
    ///
    /// The lint speaks about a scenario, not a node, so this is a best answer:
    /// the first block in reading order that holds what the message quotes. A
    /// message that quotes a chain and a step number lands on that step.
    pub(crate) fn located(&self, scenario: Entity, message: &str) -> Option<Entity> {
        let quoted: Vec<&str> = message.split('\'').skip(1).step_by(2).collect();
        let step: Option<usize> = message
            .split(" step ")
            .nth(1)
            .and_then(|tail| tail.split_whitespace().next())
            .and_then(|index| index.parse().ok());
        let empty_key = message.contains("empty key");
        for block in self.blocks_of(scenario) {
            if let Some(action) = self.action(block) {
                let key = match &action.kind {
                    ActionKind::Sequence(head) => Some(head.key.as_str()),
                    ActionKind::Cinematic(head) => Some(head.key.as_str()),
                    _ => None,
                };
                if let Some(key) = key {
                    if (empty_key && key.trim().is_empty()) || quoted.contains(&key) {
                        let beat = step.and_then(|index| self.steps_of(block).get(index).copied());
                        return Some(beat.unwrap_or(block));
                    }
                }
            }
            let config = self
                .action(block)
                .and_then(|action| super::action_config(&action.kind))
                .or_else(|| {
                    self.filter(block)
                        .and_then(|filter| super::filter_config(&filter.kind))
                });
            let Some(config) = config else {
                continue;
            };
            let mut named = false;
            walk_names(config, &mut |name| named |= quoted.contains(&name.text));
            if named {
                return Some(block);
            }
        }
        None
    }

    /// The unfinished blocks of `scenario`, in reading order.
    pub(crate) fn gaps(&self, scenario: Entity) -> Vec<(Entity, String)> {
        let mut gaps = Vec::new();
        for handler in self.events_of(scenario) {
            self.gaps_in_filters(handler, &mut gaps);
            self.gaps_in_actions(handler, &mut gaps);
        }
        gaps
    }

    fn gaps_in_filters(&self, parent: Entity, gaps: &mut Vec<(Entity, String)>) {
        for filter in self.filters_of(parent) {
            self.gaps_in_filter(filter, gaps);
        }
    }

    fn gaps_in_filter(&self, node: Entity, gaps: &mut Vec<(Entity, String)>) {
        let Some(filter) = self.filter(node) else {
            return;
        };
        if matches!(filter.kind, FilterKind::Expression) {
            match self.operands_of(node).first() {
                Some(root) => self.gaps_in_condition(node, *root, gaps),
                None => gaps.push((node, "this condition compares nothing".to_string())),
            }
            return;
        }
        let Some(wanted) = operand_room(&filter.kind) else {
            return;
        };
        let held = self.filters_of(node);
        if held.len() != wanted {
            let word = super::filter_choice(&filter.kind).label();
            gaps.push((
                node,
                format!("{word} has {} of {wanted} conditions", held.len()),
            ));
        }
        for operand in held {
            self.gaps_in_filter(operand, gaps);
        }
    }

    fn gaps_in_condition(&self, filter: Entity, root: Entity, gaps: &mut Vec<(Entity, String)>) {
        let compares = self
            .expression(root)
            .is_some_and(|expression| super::expr_choice(&expression.kind).compares());
        if !compares {
            gaps.push((filter, "a condition must compare two values".to_string()));
            return;
        }
        self.gaps_in_operands(filter, root, gaps);
    }

    /// `owner` is the block the canvas and the Problems row point at: an operand
    /// has no row of its own.
    fn gaps_in_operands(&self, owner: Entity, node: Entity, gaps: &mut Vec<(Entity, String)>) {
        let Some(expression) = self.expression(node) else {
            return;
        };
        match &expression.kind {
            ExprKind::Value(ValueOperand { value }) => {
                if value.to_string().trim().is_empty() {
                    gaps.push((owner, "a variable has no name".to_string()));
                }
                return;
            }
            // The scenario lint names an id or an item a query lacks.
            ExprKind::Query(_) => return,
            _ => {}
        }
        let held = self.operands_of(node);
        if held.len() < 2 {
            gaps.push((owner, "an operator is missing a value".to_string()));
        }
        for operand in held {
            if self
                .expression(operand)
                .is_some_and(|inner| super::expr_choice(&inner.kind).compares())
            {
                gaps.push((owner, "a comparison cannot be a value".to_string()));
                continue;
            }
            self.gaps_in_operands(owner, operand, gaps);
        }
    }

    fn gaps_in_actions(&self, parent: Entity, gaps: &mut Vec<(Entity, String)>) {
        for node in self.actions_of(parent) {
            let Some(action) = self.action(node) else {
                continue;
            };
            match action.kind {
                ActionKind::VariableSet(_) => match self.operands_of(node).first() {
                    Some(root) => {
                        let compares = self.expression(*root).is_some_and(|expression| {
                            super::expr_choice(&expression.kind).compares()
                        });
                        if compares {
                            gaps.push((node, "a comparison cannot be a value".to_string()));
                        } else {
                            self.gaps_in_operands(node, *root, gaps);
                        }
                    }
                    None => gaps.push((node, "this writes nothing".to_string())),
                },
                ActionKind::Sequence(_) | ActionKind::Cinematic(_) => {
                    for step in self.steps_of(node) {
                        if let Some(gate) = self.gate_of(step) {
                            self.gaps_in_filters(gate, gaps);
                        }
                        self.gaps_in_actions(step, gaps);
                    }
                }
                ActionKind::Leaf(_) => {}
            }
        }
    }
}
