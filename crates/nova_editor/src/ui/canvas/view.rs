//! What the canvas draws, as data: [`BlockView`], built from [`ScriptNodes`].
//!
//! A view is a value with no entities of its own, so a handler repaints by
//! comparing two of them: a different shape is a rebuild, the same shape with a
//! different value is a text write. See [`BlockView::same_shape`].

use std::collections::HashMap;

use bevy::prelude::*;
use nova_scenario::prelude::*;

use crate::{
    event::{
        action_choice, filter_choice, is_literal, ActionChoice, ActionKind, BlockItemType,
        BlockRoleType, BlockShapeType, BlockSlotType, ExprChoice, ExprKind, FilterKind,
        ScriptNodes, ValueOperand,
    },
    inspect::{
        action_rows, event_rows, filter_rows, gate_rows, operand_row, query_rows, step_rows,
        variant_hint, DragRule, InspectorRow, Operand, RowValue,
    },
    palette::{block_colour, block_lifted},
    ui::inspector::{InspectorField, Offers},
};

/// The most fields a block shows inline. The rest are behind its `...` chip.
const MAX_INLINE: usize = 3;

/// The longest a chip's text runs before it is cut.
const MAX_CHIP_CHARS: usize = 26;

/// The silhouette a block is drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutlineType {
    /// Rounded top, holds a stack below it.
    Hat,
    /// A plain stack block.
    Stack,
    /// A C-block: head, left bar, indented stack, foot.
    Chain,
    /// A row inside a step's head.
    Gate,
    /// Pointed ends: a condition.
    Boolean,
    /// Round ends: a value.
    Reporter,
}

/// How an inline field edits.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ChipType {
    /// Click to type.
    Text,
    /// Click to type, drag to scrub.
    Number(DragRule),
    /// Click for the list.
    Pick {
        /// Every option.
        options: Vec<String>,
        /// What each option means.
        hints: Vec<String>,
    },
    /// Click to flip.
    Flag,
    /// Click for what the document names.
    Ref(Offers),
}

/// One inline field.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ChipView {
    /// The small word before the value. Empty for none.
    pub(crate) label: String,
    /// The value, as it reads.
    pub(crate) text: String,
    /// The whole value, for the editor a click opens.
    pub(crate) whole: String,
    /// The unit after the value.
    pub(crate) unit: &'static str,
    /// How it edits.
    pub(crate) kind: ChipType,
    /// What it writes to. `None` on a palette preview.
    pub(crate) field: Option<InspectorField>,
}

impl ChipView {
    /// What the chip reads. An empty one still has to be a target.
    pub(crate) fn shown(&self) -> String {
        if self.text.is_empty() {
            "...".to_string()
        } else {
            self.text.clone()
        }
    }
}

/// One thing on a block's line.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PieceType {
    /// Fixed text.
    Word(String),
    /// An inline field.
    Chip(ChipView),
    /// A value socket holding its own typed literal.
    Literal {
        /// The socket.
        slot: BlockSlotType,
        /// The literal's field.
        chip: ChipView,
    },
    /// A socket with nothing in it, saying what it wants.
    Empty {
        /// The socket.
        slot: BlockSlotType,
        /// The word it shows.
        wants: &'static str,
    },
    /// A block sitting in a socket.
    Block(BlockView),
    /// This block has more fields than it shows: the node they belong to.
    More(Entity),
}

/// A stack of blocks, with a slot before each and one after the last.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StackView {
    /// The node the stack hangs from.
    pub(crate) parent: Entity,
    /// Whether this stack holds beats rather than actions.
    pub(crate) steps: bool,
    /// The blocks, top first.
    pub(crate) blocks: Vec<BlockView>,
}

impl StackView {
    /// The slot before block `index`.
    pub(crate) fn slot(&self, index: usize) -> BlockSlotType {
        if self.steps {
            BlockSlotType::Step {
                parent: self.parent,
                index,
            }
        } else {
            BlockSlotType::Action {
                parent: self.parent,
                index,
            }
        }
    }
}

/// One block and everything inside it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BlockView {
    /// The node it draws. `None` on a palette preview.
    pub(crate) node: Option<Entity>,
    /// Ids from the handler down: what the block's `Name` is built from.
    pub(crate) path: String,
    /// Its silhouette.
    pub(crate) outline: OutlineType,
    /// Its fill.
    pub(crate) colour: Color,
    /// The head line.
    pub(crate) head: Vec<PieceType>,
    /// The step this block is, for the slot a gate lands in.
    pub(crate) gate_slot: Option<Entity>,
    /// The gate, where this beat has one.
    pub(crate) gate: Option<Box<BlockView>>,
    /// The filter list's parent and its filters, for a block that holds one.
    pub(crate) filters: Option<(Entity, Vec<BlockView>)>,
    /// The stack inside or below it.
    pub(crate) stack: Option<StackView>,
    /// What the lint or the gap check says about it.
    pub(crate) fault: Option<String>,
}

impl BlockView {
    fn new(node: Option<Entity>, path: String, outline: OutlineType, colour: Color) -> Self {
        Self {
            node,
            path,
            outline,
            colour,
            head: Vec::new(),
            gate_slot: None,
            gate: None,
            filters: None,
            stack: None,
            fault: None,
        }
    }

    /// The block a palette entry shows and a drag out of it carries.
    pub(crate) fn preview(item: BlockItemType) -> Self {
        // A C-block holds beats: it shows its open mouth, as on the canvas.
        let holds_beats = matches!(
            item,
            BlockItemType::Step
                | BlockItemType::Action(ActionChoice::Sequence | ActionChoice::Cinematic)
        );
        let outline = match item.shape() {
            BlockShapeType::Hat => OutlineType::Hat,
            _ if holds_beats => OutlineType::Chain,
            BlockShapeType::Stack | BlockShapeType::Step | BlockShapeType::Gate => {
                OutlineType::Stack
            }
            BlockShapeType::Boolean | BlockShapeType::Comparison => OutlineType::Boolean,
            BlockShapeType::Reporter => OutlineType::Reporter,
        };
        let mut view = Self::new(None, item.label(), outline, block_colour(item.role()));
        view.head.push(PieceType::Word(item.label()));
        if matches!(outline, OutlineType::Hat | OutlineType::Chain) {
            view.stack = Some(StackView {
                parent: Entity::PLACEHOLDER,
                steps: holds_beats && item != BlockItemType::Step,
                blocks: Vec::new(),
            });
        }
        view
    }

    /// Every chip in reading order, the order [`paint`](super::paint) spawns
    /// them in.
    pub(crate) fn chips(&self) -> Vec<&ChipView> {
        let mut chips = Vec::new();
        self.gather_chips(&mut chips);
        chips
    }

    fn gather_chips<'a>(&'a self, chips: &mut Vec<&'a ChipView>) {
        for piece in &self.head {
            match piece {
                PieceType::Chip(chip) | PieceType::Literal { chip, .. } => chips.push(chip),
                PieceType::Block(block) => block.gather_chips(chips),
                PieceType::Word(_) | PieceType::Empty { .. } | PieceType::More(_) => {}
            }
        }
        if let Some(gate) = &self.gate {
            gate.gather_chips(chips);
        }
        if let Some((_, filters)) = &self.filters {
            for filter in filters {
                filter.gather_chips(chips);
            }
        }
        if let Some(stack) = &self.stack {
            for block in &stack.blocks {
                block.gather_chips(chips);
            }
        }
    }

    fn strip_values(&mut self) {
        for piece in &mut self.head {
            match piece {
                PieceType::Chip(chip) | PieceType::Literal { chip, .. } => {
                    chip.text.clear();
                    chip.whole.clear();
                }
                PieceType::Block(block) => block.strip_values(),
                PieceType::Word(_) | PieceType::Empty { .. } | PieceType::More(_) => {}
            }
        }
        if let Some(gate) = &mut self.gate {
            gate.strip_values();
        }
        if let Some((_, filters)) = &mut self.filters {
            filters.iter_mut().for_each(Self::strip_values);
        }
        if let Some(stack) = &mut self.stack {
            stack.blocks.iter_mut().for_each(Self::strip_values);
        }
    }

    /// Whether `other` draws the same nodes, values aside. An edit keeps the
    /// shape, so a scrub in progress is not despawned under the pointer.
    pub(crate) fn same_shape(&self, other: &Self) -> bool {
        let (mut left, mut right) = (self.clone(), other.clone());
        left.strip_values();
        right.strip_values();
        left == right
    }

    /// How many blocks this view draws, itself included.
    pub(crate) fn count(&self) -> usize {
        let mut count = 1;
        for piece in &self.head {
            if let PieceType::Block(block) = piece {
                count += block.count();
            }
        }
        count += self.gate.as_ref().map_or(0, |gate| gate.count());
        if let Some((_, filters)) = &self.filters {
            count += filters.iter().map(Self::count).sum::<usize>();
        }
        if let Some(stack) = &self.stack {
            count += stack.blocks.iter().map(Self::count).sum::<usize>();
        }
        count
    }

    /// The height of a pointed or round block, which its nesting sets.
    pub(crate) fn pill_height(&self) -> f32 {
        let inner = self
            .head
            .iter()
            .filter_map(|piece| match piece {
                PieceType::Block(block) => Some(block.pill_height()),
                _ => None,
            })
            .fold(0.0, f32::max);
        (inner + 8.0).max(28.0)
    }
}

fn cut(text: &str) -> String {
    let flat = text.replace('\n', " ");
    if flat.chars().count() <= MAX_CHIP_CHARS {
        return flat;
    }
    let kept: String = flat.chars().take(MAX_CHIP_CHARS - 3).collect();
    format!("{kept}...")
}

/// `row` as an inline field, or `None` for a row only the block's pop-up can
/// draw.
fn chip_of(node: Entity, row: &InspectorRow) -> Option<ChipView> {
    let field = Some(InspectorField::of_row(node, row));
    let label = row.label.to_lowercase();
    let unit = row.unit;
    let text_chip = |text: &String, kind: ChipType| ChipView {
        label: label.clone(),
        text: cut(text),
        whole: text.clone(),
        unit,
        kind,
        field: field.clone(),
    };
    match &row.value {
        RowValue::Text(text) => Some(text_chip(
            text,
            Offers::of(row).map_or(ChipType::Text, ChipType::Ref),
        )),
        RowValue::Number(text) => Some(text_chip(
            text,
            if row.nudge > 0.0 {
                ChipType::Number(DragRule {
                    step: row.nudge,
                    limit: row.limit,
                })
            } else {
                ChipType::Text
            },
        )),
        RowValue::Flag(flag) => Some(ChipView {
            label: String::new(),
            text: format!("{} {label}", if *flag { "[x]" } else { "[ ]" }),
            whole: String::new(),
            unit: "",
            kind: ChipType::Flag,
            field,
        }),
        RowValue::Choice {
            options,
            hints,
            chosen,
        } => Some(text_chip(
            &options.get(*chosen).cloned().unwrap_or_default(),
            ChipType::Pick {
                options: options.clone(),
                hints: hints.clone(),
            },
        )),
        _ => None,
    }
}

/// An optional field holding nothing has no line on a block: it would only say
/// that a field exists.
fn holds_nothing(row: &InspectorRow) -> bool {
    row.optional
        && matches!(&row.value, RowValue::Text(text) | RowValue::Number(text) if text.is_empty())
}

/// Stand a `...` chip on `view` unless it has one for `node`. An action and a
/// filter keep their Kind row off the line, so this is how that row is reached.
fn reach_kind_row(view: &mut BlockView, node: Entity) {
    let raised = view
        .head
        .iter()
        .any(|piece| matches!(piece, PieceType::More(held) if *held == node));
    if !raised {
        view.head.push(PieceType::More(node));
    }
}

/// The inline chips of `rows`, then a `...` chip if the pop-up holds more. Every
/// row left off the line has to raise it: the pop-up is that row's only way in.
fn chips_of(node: Entity, rows: &[InspectorRow], keep_empty: bool) -> Vec<PieceType> {
    let mut pieces = Vec::new();
    let mut more = false;
    for row in rows {
        if !keep_empty && holds_nothing(row) {
            more = true;
            continue;
        }
        // What names a block is worth a place on it however deep it sits: a
        // Spawn Object holds its id three levels down.
        let shown = row.group.is_empty() || matches!(row.label.as_str(), "Id" | "Name");
        let chip = shown
            .then(|| chip_of(node, row))
            .flatten()
            .filter(|_| pieces.len() < MAX_INLINE);
        match chip {
            Some(chip) => pieces.push(PieceType::Chip(chip)),
            None => more = true,
        }
    }
    if more {
        pieces.push(PieceType::More(node));
    }
    pieces
}

/// The walk from [`ScriptNodes`] to views.
pub(crate) struct ViewBuilder<'a, 'w, 's> {
    /// The script.
    pub(crate) script: &'a ScriptNodes<'w, 's>,
    /// What is wrong with each block.
    pub(crate) faults: &'a HashMap<Entity, String>,
}

impl ViewBuilder<'_, '_, '_> {
    fn id(&self, node: Entity) -> String {
        self.script.id(node).unwrap_or("node").to_string()
    }

    fn fault(&self, node: Entity) -> Option<String> {
        self.faults.get(&node).cloned()
    }

    /// A handler.
    pub(crate) fn hat(&self, node: Entity) -> Option<BlockView> {
        let event = self.script.event(node)?;
        let path = self.id(node);
        let mut view = BlockView::new(
            Some(node),
            path.clone(),
            OutlineType::Hat,
            block_colour(BlockRoleType::Event),
        );
        view.head.push(PieceType::Word("when".to_string()));
        let rows = event_rows(event);
        // The trigger leads: it is what the block means.
        let trigger: Vec<InspectorRow> = rows
            .iter()
            .filter(|row| matches!(row.value, RowValue::Choice { .. }))
            .cloned()
            .collect();
        let rest: Vec<InspectorRow> = rows
            .iter()
            .filter(|row| !matches!(row.value, RowValue::Choice { .. }))
            .cloned()
            .collect();
        for row in &trigger {
            if let Some(mut chip) = chip_of(node, row) {
                chip.label.clear();
                view.head.push(PieceType::Chip(chip));
            }
        }
        view.head.extend(chips_of(node, &rest, false));
        view.filters = Some((node, self.filters(node, &path)));
        view.stack = Some(StackView {
            parent: node,
            steps: false,
            blocks: self.actions(node, &path),
        });
        view.fault = self.fault(node);
        Some(view)
    }

    fn filters(&self, parent: Entity, path: &str) -> Vec<BlockView> {
        self.script
            .filters_of(parent)
            .into_iter()
            .filter_map(|filter| self.filter(filter, path, 0))
            .collect()
    }

    fn actions(&self, parent: Entity, path: &str) -> Vec<BlockView> {
        self.script
            .actions_of(parent)
            .into_iter()
            .filter_map(|action| self.action(action, path))
            .collect()
    }

    /// An action.
    pub(crate) fn action(&self, node: Entity, above: &str) -> Option<BlockView> {
        let action = self.script.action(node)?;
        let path = format!("{above}/{}", self.id(node));
        let choice = action_choice(&action.kind);
        let colour = block_colour(BlockRoleType::of_action(choice));
        let rows = action_rows(action);
        // The first row is the kind, which the block's own word already says.
        let fields = rows.get(1..).unwrap_or_default();
        let chain = matches!(
            action.kind,
            ActionKind::Sequence(_) | ActionKind::Cinematic(_)
        );
        let outline = if chain {
            OutlineType::Chain
        } else {
            OutlineType::Stack
        };
        let mut view = BlockView::new(Some(node), path.clone(), outline, colour);
        match &action.kind {
            ActionKind::VariableSet(_) => {
                view.head.push(PieceType::Word("set".to_string()));
                view.head.extend(chips_of(node, fields, true));
                view.head.push(PieceType::Word("to".to_string()));
                if let Some(value) = self.script.operands_of(node).first() {
                    view.head.push(self.value(
                        *value,
                        BlockSlotType::Value {
                            parent: node,
                            index: 0,
                        },
                        &path,
                        0,
                    ));
                } else {
                    view.head.push(PieceType::Empty {
                        slot: BlockSlotType::Value {
                            parent: node,
                            index: 0,
                        },
                        wants: "value",
                    });
                }
            }
            _ => {
                view.head.push(PieceType::Word(choice.label().to_string()));
                view.head.extend(chips_of(node, fields, false));
            }
        }
        reach_kind_row(&mut view, node);
        if chain {
            view.stack = Some(StackView {
                parent: node,
                steps: true,
                blocks: self
                    .script
                    .steps_of(node)
                    .into_iter()
                    .filter_map(|step| self.step(step, &path, colour))
                    .collect(),
            });
        }
        view.fault = self.fault(node);
        Some(view)
    }

    /// A beat of a chain painted `colour`.
    pub(crate) fn step(&self, node: Entity, above: &str, colour: Color) -> Option<BlockView> {
        let step = self.script.step(node)?;
        let path = format!("{above}/{}", self.id(node));
        let colour = block_lifted(colour, 0.12);
        let mut view = BlockView::new(Some(node), path.clone(), OutlineType::Chain, colour);
        view.head.push(PieceType::Word("step".to_string()));
        view.head.extend(chips_of(node, &step_rows(step), true));
        view.gate_slot = Some(node);
        view.gate = self
            .script
            .gate_of(node)
            .and_then(|gate| self.gate(gate, &path, colour))
            .map(Box::new);
        view.stack = Some(StackView {
            parent: node,
            steps: false,
            blocks: self.actions(node, &path),
        });
        view.fault = self.fault(node);
        Some(view)
    }

    /// What a beat waits for.
    pub(crate) fn gate(&self, node: Entity, above: &str, colour: Color) -> Option<BlockView> {
        let gate = self.script.gate(node)?;
        let path = format!("{above}/{}", self.id(node));
        let mut view = BlockView::new(
            Some(node),
            path.clone(),
            OutlineType::Gate,
            block_lifted(colour, 0.1),
        );
        view.head.push(PieceType::Word("wait for".to_string()));
        let mut more = false;
        for row in &gate_rows(gate) {
            if let Some(mut chip) = chip_of(node, row) {
                chip.label.clear();
                view.head.push(PieceType::Chip(chip));
            } else {
                more = true;
            }
        }
        if more {
            view.head.push(PieceType::More(node));
        }
        view.filters = Some((node, self.filters(node, &path)));
        view.fault = self.fault(node);
        Some(view)
    }

    /// A condition, `depth` combinators in.
    pub(crate) fn filter(&self, node: Entity, above: &str, depth: usize) -> Option<BlockView> {
        let filter = self.script.filter(node)?;
        let path = format!("{above}/{}", self.id(node));
        let colour = block_lifted(
            block_colour(BlockRoleType::of_filter(filter_choice(&filter.kind))),
            0.12 * depth.min(3) as f32,
        );
        let mut view = BlockView::new(Some(node), path.clone(), OutlineType::Boolean, colour);
        match filter.kind {
            FilterKind::Expression => match self.script.operands_of(node).first() {
                Some(root) => view.head = self.comparison(*root, &path),
                None => view
                    .head
                    .push(PieceType::Word("nothing to compare".to_string())),
            },
            FilterKind::Not | FilterKind::And | FilterKind::Or => {
                let held = self.script.filters_of(node);
                let socket = |index: usize| match held.get(index) {
                    Some(operand) => self
                        .filter(*operand, &path, depth + 1)
                        .map_or(PieceType::More(*operand), PieceType::Block),
                    None => PieceType::Empty {
                        slot: BlockSlotType::Operand {
                            parent: node,
                            index,
                        },
                        wants: "condition",
                    },
                };
                if matches!(filter.kind, FilterKind::Not) {
                    view.head.push(PieceType::Word("not".to_string()));
                    view.head.push(socket(0));
                } else {
                    let word = if matches!(filter.kind, FilterKind::And) {
                        "and"
                    } else {
                        "or"
                    };
                    view.head.push(socket(0));
                    view.head.push(PieceType::Word(word.to_string()));
                    view.head.push(socket(1));
                }
            }
            _ => {
                let rows = filter_rows(filter);
                let word = filter_choice(&filter.kind).label();
                view.head.push(PieceType::Word(word.to_string()));
                view.head
                    .extend(chips_of(node, rows.get(1..).unwrap_or_default(), true));
            }
        }
        reach_kind_row(&mut view, node);
        view.fault = self.fault(node);
        Some(view)
    }

    fn comparison(&self, root: Entity, above: &str) -> Vec<PieceType> {
        let Some(expression) = self.script.expression(root) else {
            return Vec::new();
        };
        let mut pieces = Vec::new();
        let sides = self.script.operands_of(root);
        for (index, side) in sides.iter().enumerate() {
            pieces.push(self.value(
                *side,
                BlockSlotType::Value {
                    parent: root,
                    index,
                },
                above,
                0,
            ));
            if index == 0 {
                pieces.push(PieceType::Chip(operator_chip(
                    root,
                    expression,
                    Operand::Test,
                )));
            }
        }
        pieces
    }

    /// What stands in value socket `slot`.
    pub(crate) fn value(
        &self,
        node: Entity,
        slot: BlockSlotType,
        above: &str,
        depth: usize,
    ) -> PieceType {
        let Some(expression) = self.script.expression(node) else {
            return PieceType::Empty {
                slot,
                wants: "value",
            };
        };
        let path = format!("{above}/{}", self.id(node));
        if is_literal(&expression.kind) {
            let ExprKind::Value(ValueOperand { value }) = &expression.kind else {
                return PieceType::More(node);
            };
            let whole = value.to_string();
            return PieceType::Literal {
                slot,
                chip: ChipView {
                    label: String::new(),
                    text: cut(&whole),
                    whole,
                    unit: "",
                    kind: ChipType::Text,
                    field: Some(InspectorField::of_operand(node)),
                },
            };
        }
        let colour = block_lifted(
            block_colour(BlockRoleType::Value),
            0.12 * depth.min(3) as f32,
        );
        let mut view = BlockView::new(Some(node), path.clone(), OutlineType::Reporter, colour);
        if let ExprKind::Value(ValueOperand { value }) = &expression.kind {
            let whole = value.to_string();
            let named = matches!(
                value,
                VariableExpressionNode::Term(VariableTermNode::Factor(VariableFactorNode::Name(_)))
            );
            view.head.push(PieceType::Chip(ChipView {
                label: String::new(),
                text: if whole.is_empty() {
                    "name".to_string()
                } else {
                    cut(&whole)
                },
                whole,
                unit: "",
                kind: if named {
                    ChipType::Ref(Offers::Named(Names::Variable))
                } else {
                    ChipType::Text
                },
                field: Some(InspectorField::of_operand(node)),
            }));
        } else if let ExprKind::Query(operand) = &expression.kind {
            view.head
                .extend(chips_of(node, &query_rows(node, operand), true));
        } else {
            for (index, side) in self.script.operands_of(node).iter().enumerate() {
                view.head.push(self.value(
                    *side,
                    BlockSlotType::Value {
                        parent: node,
                        index,
                    },
                    &path,
                    depth + 1,
                ));
                if index == 0 {
                    view.head.push(PieceType::Chip(operator_chip(
                        node,
                        expression,
                        Operand::ValueSide,
                    )));
                }
            }
        }
        PieceType::Block(view)
    }

    /// What a drag of `node` shows: the block as it stands on the canvas.
    pub(crate) fn any(&self, node: Entity) -> Option<BlockView> {
        if self.script.event(node).is_some() {
            return self.hat(node);
        }
        if self.script.action(node).is_some() {
            return self.action(node, "");
        }
        if self.script.step(node).is_some() {
            return self.step(node, "", block_colour(BlockRoleType::Flow));
        }
        if self.script.gate(node).is_some() {
            return self.gate(node, "", block_colour(BlockRoleType::Flow));
        }
        if self.script.filter(node).is_some() {
            return self.filter(node, "", 0);
        }
        let probe = BlockSlotType::Canvas(Vec2::ZERO);
        match self.value(node, probe, "", 0) {
            PieceType::Block(view) => Some(view),
            _ => None,
        }
    }
}

fn operator_chip(
    node: Entity,
    expression: &crate::event::ExpressionNode,
    role: Operand,
) -> ChipView {
    let row = operand_row(node, expression, "Operator", role, 0);
    let (options, chosen) = match &row.value {
        RowValue::Operand {
            options, chosen, ..
        } => (options.clone(), *chosen),
        _ => (Vec::new(), 0),
    };
    let hints = ExprChoice::ALL
        .iter()
        .filter(|choice| options.iter().any(|option| option == choice.label()))
        .map(variant_hint)
        .collect();
    ChipView {
        label: String::new(),
        text: options.get(chosen).cloned().unwrap_or_default(),
        whole: String::new(),
        unit: "",
        kind: ChipType::Pick { options, hints },
        field: Some(InspectorField::of_row(node, &row)),
    }
}
