//! The right-hand Inspector: what the selected node IS, and the fields you
//! change it by.
//!
//! The rows come from [`crate::inspect`], which reads them off the node's own
//! config by reflection. This module only turns a row into a widget and a
//! widget back into an edit, which is why there is no field name anywhere in
//! it: a config that grows a field grows a row here without a line changing.
//!
//! THE SHAPE IS REBUILT, THE VALUES ARE NOT. A frame that changed a number
//! repaints the box that holds it; only a frame that changed which ROWS exist -
//! another node inspected, an optional struct appearing - respawns the list.
//! That is what lets a builder type into a field at all: a list rebuilt every
//! frame would despawn the box under the caret.

use bevy::{
    ecs::{component::Mutable, relationship::RelatedSpawnerCommands, system::SystemParam},
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{observe, Activate, Button},
    window::PrimaryWindow,
};
use nova_scenario::prelude::{Names, ScenarioObjectKind};
use nova_ship::prelude::GameSections;
use nova_ui::{
    prelude::{
        panel, panel_header, scroll_bar, scroll_column, scroll_row, scroll_viewport,
        segmented_container, segmented_container_wrapping, segmented_option, segmented_option_fit,
        text_field, ButtonLabel, ButtonVariant, Selected, TextFieldError, TextFieldFocused,
        TextFieldSpec, TextFieldSubmitted, TextFieldValue, ThemedButton, UiSkin,
    },
    theme,
    widget::{checkbox, checkbox_colors, checkbox_glyph, swatch},
};

use crate::{
    config::{EditorOverlays, EditorSays, InspectorHeader, RailTab, SelectedNode},
    event::{
        action_config_mut, expr_config_mut, filter_config_mut, retype_script_node, ActionNode,
        EventNode, ExpressionNode, FilterNode, GateNode, StepNode,
    },
    gizmo::GizmoAxis,
    inspect::{
        action_rows, axis_step, choose_field, curated_object_rows, curated_section_rows,
        driver_label, editable_config, event_rows, expression_rows, filter_rows, gate_rows,
        inspected, nudge_field, object_config_mut, object_rows, parse_colour, rotation_degrees,
        rotation_from_degrees, scenario_rows, script_name, section_config_mut, section_rows,
        ship_rows, step_rows, toggle_field, write_field, DocumentIds, DragRule, FieldRoot,
        InspectTarget, InspectorRow, NodeKinds, PathStep, RowValue, ScriptNames, GRIP_GONE,
    },
    keybind::on_rebind_action,
    node::{
        default_allegiance, EditContext, NodeId, ObjectBodyStale, ObjectNode, SectionNode,
        ShipDriver, ShipNode,
    },
    preview::body_is_drawn_from,
    ui::window::{on_open_colour_window, on_open_ref_window},
};

/// Panel width IN SCENE MODE. Wider than the rail because every row is a name
/// AND a value side by side; still narrow enough to leave the stage its centre,
/// which is where the placement raycast goes.
///
/// The events editor is not bound by that - it has no stage to keep clickable -
/// and takes whatever the rail leaves instead. See `crate::ui::sync_editor_mode`.
pub(crate) const PANEL_W: f32 = 300.0;
/// The name column. Fixed rather than proportional so the value boxes of every
/// row line up, which is what makes a column of numbers readable. Wide enough
/// for the longest name the curated rows carry (`Projectile Lifetime`), which
/// is what the panel's own widening bought.
const LABEL_W: f32 = 118.0;

/// The right-hand panel's root, hidden when there is nothing to inspect.
#[derive(Component)]
pub(crate) struct InspectorPanel;

/// The line under the header saying WHAT is being inspected.
#[derive(Component)]
pub(crate) struct InspectorTitle;

/// The node's ID, under the title.
///
/// A SPAN of the title's own text rather than a node of its own: it is the
/// second half of one sentence, and a `TextSpan` shares the title's layout
/// while keeping its own colour and size.
#[derive(Component)]
pub(crate) struct InspectorId;

/// The row container, refilled when the row shape changes.
#[derive(Component)]
pub(crate) struct InspectorList;

/// Which row a widget belongs to.
///
/// An INDEX rather than the field it edits, because two rows of one node can
/// name the same path (an `Option` and the struct inside it) and the repaint
/// has to tell them apart. Safe because any change to the row list rebuilds
/// the whole list.
#[derive(Component, Clone, Copy)]
pub(crate) struct InspectorSlot(usize);

/// What a widget edits: the node, and where in it the value lives.
///
/// Compared by VALUE, so a floating window opened from a row can find the row
/// it belongs to again after the panel has repainted.
#[derive(Component, Clone, PartialEq)]
pub(crate) struct InspectorField {
    node: Entity,
    root: FieldRoot,
    path: Vec<PathStep>,
    optional: bool,
}

/// The chip that opens the picker on a row that NAMES something, and what
/// kind of name it is.
///
/// Beside the box rather than instead of it: an id can be typed, including one
/// for an object the builder is about to add, and the picker is the shortcut
/// for the ones that already exist.
#[derive(Component, Clone)]
pub(crate) struct InspectorRef {
    /// The row's label, for the window's title.
    pub(crate) label: String,
    /// What the field names, which decides what the picker lists.
    pub(crate) names: Names,
}

/// A reference row whose text names nothing this document holds.
///
/// On the UNIT, which is the slot a row already borrows to say what is wrong
/// with it - the same place a refused edit puts its reason.
#[derive(Component)]
pub(crate) struct InspectorFault;

/// What the unit slot of an unresolved reference reads.
///
/// The lowering DROPS a handler that names an id nothing spawns, so this word
/// is the warning before the drop.
const UNRESOLVED: &str = "unknown";

/// The picker chip: a caret in a box the width of a swatch.
fn ref_chip() -> impl Bundle {
    (
        Node {
            width: px(22),
            height: px(22),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        BorderColor::all(theme::PHOSPHOR.with_alpha(0.4)),
        BackgroundColor(Color::NONE),
        children![(
            Text::new(crate::glyph::PICK),
            TextFont {
                font_size: FontSize::Px(11.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR_MUTED),
        )],
    )
}

/// The unit beside a row's value, and what that slot says when the value is
/// good.
///
/// A REFUSED edit borrows the slot for its reason. The panel has no line of its
/// own to put a reason on, and the only other free space is the next row's.
#[derive(Component)]
pub(crate) struct InspectorUnit(&'static str);

/// A number's name, which is also the grip that scrubs it: the rule the drag
/// moves the value by, and what a live drag has to remember between frames.
///
/// The NAME rather than a control of its own, because the panel is 240px wide
/// and a row that spends pixels on a grip spends them on the box holding the
/// number. It rides beside an [`InspectorField`], which says what it writes to.
#[derive(Component, Clone, Copy)]
pub(crate) struct InspectorDrag {
    /// How far one pixel takes the number, and where it stops.
    rule: DragRule,
    /// Pixels this drag has travelled that did not add up to a whole step yet.
    ///
    /// Kept rather than dropped, because one physical pixel is HALF a logical
    /// one at 2x scale: a grip that truncated each frame on its own would
    /// refuse to move on exactly the displays this range exists to support.
    residual: f32,
    /// How far the last wrap teleported the pointer.
    ///
    /// `bevy_picking` measures its delta from the last cursor position it SAW,
    /// so the warp reaches the next frame as a move like any other. Only the
    /// grip that asked for it knows to take it back.
    warped: f32,
}

impl InspectorDrag {
    /// A grip that has not been dragged yet.
    fn new(rule: DragRule) -> Self {
        Self {
            rule,
            residual: 0.0,
            warped: 0.0,
        }
    }
}

/// How close to a window edge a scrub gets before the pointer wraps to the
/// other side. The test is `at or past`, so a drag fast enough to leap the band
/// in one frame still lands inside it.
const WRAP_EDGE: f32 = 24.0;

/// A checkbox standing for a `bool` field.
#[derive(Component)]
pub(crate) struct InspectorFlag;

/// The Key row's chip: press it to arm the rebind.
#[derive(Component)]
pub(crate) struct InspectorKey;

/// Which component of a vector row's value a box holds: 0, 1 or 2.
///
/// The three boxes share one [`InspectorSlot`] - they are one row - so this is
/// what tells the repaint which of the three numbers belongs in which box.
#[derive(Component, Clone, Copy)]
pub(crate) struct InspectorAxis(usize);

/// A read-only row's text, so the repaint can tell it from the title and from
/// a checkbox's glyph.
#[derive(Component)]
pub(crate) struct InspectorFixed;

/// One option of the ship's driver row.
#[derive(Component, Clone, Copy)]
pub(crate) struct InspectorDriver {
    ship: Entity,
    driver: ShipDriver,
}

/// A heading over the rows that share a group. Text like a readout, so the
/// repaint has to be able to tell the two apart.
#[derive(Component)]
pub(crate) struct InspectorGroup;

/// The colour block beside a colour field, so the repaint can find it and
/// recolour it when the hex beside it is retyped.
///
/// It carries its row's LABEL because it is also the way into the colour
/// picker, and a floating window has to be able to say which field it is on.
#[derive(Component)]
pub(crate) struct InspectorSwatch {
    pub(crate) label: String,
}

/// One option of a unit-enum row: the variant this segment selects.
#[derive(Component, Clone)]
pub(crate) struct InspectorChoice {
    variant: String,
}

/// Everything the panel reads to decide what it is showing.
#[derive(SystemParam)]
pub(crate) struct Document<'w, 's> {
    catalog: Option<Res<'w, GameSections>>,
    context: Res<'w, EditContext>,
    /// Which editor the screen is. The panel reads it to decide whether it may
    /// go away when nothing is selected - see [`Document::is_events`].
    ///
    /// Optional like the catalog: the editor's plugin puts it in, and a fixture
    /// that runs the panel without the rest of the editor is the Inspector.
    tab: Option<Res<'w, RailTab>>,
    overlays: Res<'w, EditorOverlays>,
    selected: Res<'w, SelectedNode>,
    kinds: NodeKinds<'w, 's>,
    ids: Query<'w, 's, &'static NodeId>,
    children: Query<'w, 's, &'static Children>,
    ships: Query<'w, 's, (&'static ShipNode, &'static Transform)>,
    sections: Query<'w, 's, &'static SectionNode>,
    objects: Query<'w, 's, (&'static ObjectNode, &'static Transform)>,
    events: Query<'w, 's, &'static EventNode>,
    filters: Query<'w, 's, &'static FilterNode>,
    actions: Query<'w, 's, &'static ActionNode>,
    steps: Query<'w, 's, &'static StepNode>,
    gates: Query<'w, 's, &'static GateNode>,
    expressions: Query<'w, 's, &'static ExpressionNode>,
    parents: Query<'w, 's, &'static ChildOf>,
    names: ScriptNames<'w, 's>,
}

impl Document<'_, '_> {
    /// Whether the panel is the events editor rather than the stage's
    /// Inspector.
    pub(crate) fn is_events(&self) -> bool {
        self.tab.as_deref().copied().is_some_and(RailTab::is_events)
    }

    /// Whether `node` stands where a COMPARISON belongs: at the root of a
    /// condition, which is the child of the filter that holds it. Everything
    /// under one is a value.
    fn compares(&self, node: Entity) -> bool {
        self.parents
            .get(node)
            .is_ok_and(|parent| !self.expressions.contains(parent.parent()))
    }

    /// The node the panel is on and the rows it wants, or `None` with nothing
    /// to inspect.
    pub(crate) fn inspection(&self) -> Option<(InspectTarget, Vec<InspectorRow>)> {
        let target = inspected(&self.selected, &self.context, &self.kinds)?;
        let rows = match target {
            // The document root holds ships and objects rather than fields of
            // its own. It gets a panel anyway: one that vanished at the root
            // would read as the inspector breaking every time you left a ship.
            InspectTarget::Scenario(scenario) => self.scenario_rows(scenario),
            InspectTarget::Ship(ship) => {
                let (node, pose) = self.ships.get(ship).ok()?;
                ship_rows(node, pose)
            }
            // CURATED unless the View menu says otherwise: a turret's whole
            // config is a joint tree nobody authors from a scenario, and the
            // panel's job is to answer what the thing does before it answers
            // what it declares.
            InspectTarget::Section(section) => {
                let node = self.sections.get(section).ok()?;
                if self.overlays.all_fields {
                    section_rows(node, self.catalog.as_deref())
                } else {
                    curated_section_rows(node, self.catalog.as_deref())
                }
            }
            InspectTarget::Object(object) => {
                let (node, pose) = self.objects.get(object).ok()?;
                if self.overlays.all_fields {
                    object_rows(node, pose)
                } else {
                    curated_object_rows(node, pose)
                }
            }
            // The script's nodes are NEVER curated: a handler has four fields
            // between it and its children, and a curated action would hide the
            // one field the builder came to set.
            InspectTarget::Event(node) => event_rows(self.events.get(node).ok()?),
            InspectTarget::Filter(node) => filter_rows(self.filters.get(node).ok()?),
            InspectTarget::Action(node) => action_rows(self.actions.get(node).ok()?),
            InspectTarget::Step(node) => step_rows(self.steps.get(node).ok()?),
            InspectTarget::Gate(node) => gate_rows(self.gates.get(node).ok()?),
            InspectTarget::Expression(node) => {
                expression_rows(self.expressions.get(node).ok()?, self.compares(node))
            }
        };
        Some((target, rows))
    }

    /// What the document holds, counted off the root's own children.
    ///
    /// The CHILDREN rather than every ship in the world: a second document
    /// cannot exist today, and counting by component would start lying the day
    /// one can.
    fn scenario_rows(&self, scenario: Entity) -> Vec<InspectorRow> {
        let children: Vec<Entity> = self
            .children
            .get(scenario)
            .map(|children| children.iter().collect())
            .unwrap_or_default();
        let ships: Vec<Entity> = children
            .iter()
            .copied()
            .filter(|child| self.ships.contains(*child))
            .collect();
        let objects = children
            .iter()
            .filter(|child| self.objects.contains(**child))
            .count();
        let flown = ships.iter().copied().find(|ship| {
            self.ships
                .get(*ship)
                .is_ok_and(|(node, _)| node.driver == ShipDriver::Player)
        });
        scenario_rows(ships.len(), objects, flown.map(|ship| self.name_of(ship)))
    }

    /// What a node is CALLED: the authored name, or the id it was minted under.
    /// The same rule the tree row uses, so the panel and the rail agree.
    fn name_of(&self, node: Entity) -> String {
        let id = self
            .ids
            .get(node)
            .map_or_else(|_| String::new(), |id| id.0.clone());
        let authored = self
            .ships
            .get(node)
            .map(|(ship, _)| ship.name.clone())
            .or_else(|_| {
                self.objects
                    .get(node)
                    .map(|(object, _)| object.name.clone())
            })
            .unwrap_or_default();
        let (name, ordinal) = super::tree_text(&authored, &id);
        if ordinal.is_empty() {
            name
        } else {
            format!("{name} {ordinal}")
        }
    }

    /// The node's ID, said out loud rather than hidden behind a hover.
    ///
    /// The one string an event's filter, a spawn action and a save file all
    /// name a node by, so it belongs where a builder can read it off the panel
    /// they are already looking at.
    fn id_of(&self, node: Entity) -> String {
        self.ids
            .get(node)
            .map_or_else(|_| String::new(), |id| id.0.clone())
    }

    /// The title line: what kind of node, and which one.
    ///
    /// The node wears the same name its tree row does, so the panel and the row
    /// that opened it read alike - and so a minted section id fits the line
    /// instead of wrapping mid-word.
    ///
    /// A seeded HULL says SHIP rather than OBJECT: it is filed with the rocks
    /// only because of how a scenario stores it, and the panel is the one place
    /// the reader is asked what they are looking at.
    fn title(&self, target: InspectTarget) -> String {
        let tag = match target {
            InspectTarget::Object(object)
                if self.objects.get(object).is_ok_and(|(node, _)| {
                    matches!(node.kind, ScenarioObjectKind::Spaceship(_))
                }) =>
            {
                "SHIP"
            }
            target => target.tag(),
        };
        // A script node is named by what it IS - `On Start`, `Entity`,
        // `Sequence` - because its minted id is plumbing, and the tree row
        // that opened the panel shows the same word.
        let name = script_name(target, &self.names);
        let name = if name.is_empty() {
            self.name_of(target.node())
        } else {
            name
        };
        format!("{tag}  {name}")
    }
}

/// The panel, built empty. Its rows are [`sync_inspector`]'s, because which
/// rows exist is a question about the document rather than about the scene.
pub(crate) fn inspector_panel(skin: UiSkin) -> impl Bundle {
    (
        Name::new("Editor Inspector"),
        InspectorPanel,
        Node {
            width: px(PANEL_W),
            // Pushed to the right edge by its own margin rather than by a
            // spacer sibling: what lies between it and the rail is the 3D
            // stage, and a spacer there would be one more thing to pick
            // through.
            margin: UiRect::left(Val::Auto),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            // Bounded, so the list inside it has something to scroll WITHIN.
            // Without this the panel simply grew to its content and a long
            // config ran off the bottom of the screen.
            min_height: px(0),
            max_height: percent(100),
            padding: UiRect::all(px(10)),
            border: UiRect::left(px(theme::BORDER_W)),
            overflow: Overflow::clip(),
            ..default()
        },
        panel(skin),
        children![
            // The header says which panel this is, and the mode rewrites it:
            // beside the stage it is the Inspector, and filling the screen it
            // is the events editor.
            (panel_header("Inspector"), InspectorHeader),
            (
                Name::new("Inspector Title"),
                InspectorTitle,
                Text::new(""),
                // WRAPS, on any character. A node id is one word with
                // underscores in it, so a word-boundary break never fires and
                // `pdc_kinetic_turret_section_7` used to run off the panel
                // edge and be cut - which loses the digit that says WHICH
                // turret is on screen.
                TextLayout {
                    linebreak: LineBreak::AnyCharacter,
                    ..default()
                },
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(theme::AMBER_NOVA),
                Node {
                    width: percent(100),
                    margin: UiRect::vertical(px(6)),
                    ..default()
                },
                children![(
                    Name::new("Inspector Id"),
                    InspectorId,
                    TextSpan::new(""),
                    TextFont {
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR_MUTED),
                )],
            ),
            // SCROLLS. A turret's joint tree is thirty rows deep, and a panel
            // that simply ran off the bottom of the screen put its muzzle's
            // fire rate somewhere no pointer could reach. The bar beside it
            // says HOW deep, which a wheel alone never did.
            (
                Name::new("Inspector Scroll"),
                scroll_row(),
                children![
                    (
                        Name::new("Inspector List"),
                        InspectorList,
                        Node {
                            align_items: AlignItems::Stretch,
                            ..scroll_column()
                        },
                        scroll_viewport(),
                    ),
                    (Name::new("Inspector Scrollbar"), scroll_bar(skin)),
                ],
            ),
        ],
    )
}

/// One row's shell: the name on the left, the value on the right.
fn row_shell() -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(6),
        margin: UiRect::bottom(px(4)),
        ..default()
    }
}

/// How far in a row or heading at `depth` stands. Capped, because a config
/// deep enough to run out of panel is a config the tree is already carrying:
/// past four levels the indent stops earning its pixels.
fn indent(depth: usize) -> f32 {
    depth.min(4) as f32 * 7.0
}

/// The name column. Clipped rather than wrapped: a two-line name would push
/// its own value box out of the column the row above lined up with.
///
/// `taken` is what the row's own indent has already eaten, so the column ends
/// where every other row's does.
fn row_label(label: &str, taken: f32) -> impl Bundle {
    (
        Node {
            width: px(LABEL_W - taken),
            flex_shrink: 0.0,
            overflow: Overflow::clip(),
            ..default()
        },
        Text::new(label.to_string()),
        TextLayout {
            linebreak: LineBreak::NoWrap,
            ..default()
        },
        TextFont {
            font_size: FontSize::Px(11.0),
            ..default()
        },
        TextColor(theme::PHOSPHOR_MUTED),
    )
}

/// The value column, which takes whatever width the name leaves.
fn value_column() -> Node {
    Node {
        flex_basis: px(0),
        flex_grow: 1.0,
        min_width: px(0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        overflow: Overflow::clip(),
        ..default()
    }
}

/// The segmented control of a choice row: one option per variant, the current
/// one marked.
fn spawn_choice_options(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    label: &str,
    field: &InspectorField,
    slot: usize,
    options: &[String],
    chosen: usize,
    skin: UiSkin,
) {
    // A ROW SLOT around the control, because a segmented container sizes to
    // its content: as a flex item of a row it shrinks to the width it is given
    // and its options share it, and without one a three-option control simply
    // runs off the panel edge.
    //
    // WIDE choices WRAP, and their options are sized to their own labels: the
    // enum decides how many there are, and an event name has sixteen. A single
    // line put twelve of them past the panel edge where nothing could click
    // them, and full-width options in a wrapping bar put every one on a line of
    // its own. A short choice keeps the even share it reads best in.
    let wide = options.len() > WIDE_CHOICE;
    parent
        .spawn(Node {
            width: percent(100),
            min_width: px(0),
            flex_direction: FlexDirection::Row,
            ..default()
        })
        .with_children(|line| {
            let mut control = line.spawn(Name::new(format!("Inspector Choice {label}")));
            if wide {
                control.insert(segmented_container_wrapping(skin));
            } else {
                control.insert(segmented_container(skin));
            }
            control.with_children(|segments| {
                for (index, option) in options.iter().enumerate() {
                    let mut entity = segments.spawn((
                        Name::new(format!("Inspector Choice {label} {option}")),
                        InspectorSlot(slot),
                        InspectorChoice {
                            variant: option.clone(),
                        },
                        field.clone(),
                        observe(on_inspector_choice),
                    ));
                    if wide {
                        entity.insert(segmented_option_fit(option));
                    } else {
                        entity.insert(segmented_option(option));
                    }
                    if index == chosen {
                        entity.insert(Selected);
                    }
                }
            });
        });
}

/// How many options a choice may have before its control wraps. Three fit
/// across the value column at the panel's width; a fourth clips.
const WIDE_CHOICE: usize = 3;

/// A wide choice: the name on its own line and the options under it, across
/// the panel rather than squeezed into the value column.
#[expect(
    clippy::too_many_arguments,
    reason = "the row says what to draw, the field says where it writes"
)]
fn spawn_choice_row(
    list: &mut RelatedSpawnerCommands<ChildOf>,
    row: &InspectorRow,
    field: &InspectorField,
    slot: usize,
    options: &[String],
    chosen: usize,
    skin: UiSkin,
    step: f32,
) {
    list.spawn((
        Name::new(format!("Inspector Row {}", row.label)),
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            padding: UiRect::left(px(step)),
            margin: UiRect::bottom(px(6)),
            row_gap: px(3),
            ..default()
        },
    ))
    .with_children(|block| {
        block.spawn((
            Text::new(row.label.clone()),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            TextFont {
                font_size: FontSize::Px(11.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR_MUTED),
        ));
        spawn_choice_options(block, &row.label, field, slot, options, chosen, skin);
    });
}

/// The driver row, in the same block shape a three-option choice gets.
///
/// It IS a choice of three wearing another name, and three labels in the value
/// column is three labels clipped: `Player` came out as `Play`, and the option
/// a builder was looking for was the one they could not read.
fn spawn_driver_row(
    list: &mut RelatedSpawnerCommands<ChildOf>,
    row: &InspectorRow,
    node: Entity,
    slot: usize,
    driver: ShipDriver,
    skin: UiSkin,
    step: f32,
) {
    list.spawn((
        Name::new(format!("Inspector Row {}", row.label)),
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            padding: UiRect::left(px(step)),
            margin: UiRect::bottom(px(6)),
            row_gap: px(3),
            ..default()
        },
    ))
    .with_children(|block| {
        block.spawn((
            Text::new(row.label.clone()),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            TextFont {
                font_size: FontSize::Px(11.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR_MUTED),
        ));
        block
            .spawn((Name::new("Inspector Driver"), segmented_container(skin)))
            .with_children(|options| {
                for option in [ShipDriver::Player, ShipDriver::Ai, ShipDriver::Adrift] {
                    let label = driver_label(option);
                    let mut entity = options.spawn((
                        Name::new(format!("Inspector Driver {label}")),
                        InspectorSlot(slot),
                        InspectorDriver {
                            ship: node,
                            driver: option,
                        },
                        segmented_option(label),
                        observe(on_inspector_driver),
                    ));
                    if option == driver {
                        entity.insert(Selected);
                    }
                }
            });
    });
}

/// The lead letter and colour of each box of a vector row.
///
/// The colours are the HANDLES' own, read off `crate::gizmo`: the number and
/// the arrow that drags it are the same axis, and a builder should not have to
/// learn that twice. A rotation's three numbers are yaw, pitch and roll - their
/// own initials - tinted by the axis each one turns ABOUT, so the letters and
/// the colours say the same thing.
fn axis_leads(root: FieldRoot) -> [(&'static str, Color); 3] {
    match root {
        FieldRoot::Rotation => [
            ("Y", GizmoAxis::Y.colour()),
            ("P", GizmoAxis::X.colour()),
            ("R", GizmoAxis::Z.colour()),
        ],
        _ => GizmoAxis::ALL.map(|axis| (axis.label(), axis.colour())),
    }
}

/// The unit beside a value: what the number in the box is measured in.
///
/// Muted and one step smaller than the value, because it is the same word on
/// every row of its kind - it has to be readable without being read.
fn unit_text(label: &str, unit: &'static str, slot: usize) -> impl Bundle {
    (
        Name::new(format!("Inspector Unit {label}")),
        InspectorSlot(slot),
        InspectorUnit(unit),
        Text::new(unit),
        TextLayout {
            linebreak: LineBreak::NoWrap,
            ..default()
        },
        TextFont {
            font_size: FontSize::Px(10.0),
            ..default()
        },
        TextColor(theme::PHOSPHOR_DIM),
        Node {
            flex_shrink: 0.0,
            ..default()
        },
    )
}

/// One vector row: its name and unit on one line, then a box per axis.
///
/// Stacked rather than three boxes across, because the panel is 240px wide: a
/// third of its value column is four characters, and a position of `-1234.567`
/// would be read four characters at a time.
fn spawn_axes_row(
    list: &mut RelatedSpawnerCommands<ChildOf>,
    row: &InspectorRow,
    field: &InspectorField,
    slot: usize,
    axes: &[String; 3],
    step: f32,
) {
    let leads = axis_leads(row.root);
    let unit = row.unit;
    let rule = DragRule {
        step: row.nudge,
        limit: row.limit,
    };
    let label = row.label.clone();
    list.spawn((
        Name::new(format!("Inspector Row {}", row.label)),
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            padding: UiRect::left(px(step)),
            margin: UiRect::bottom(px(6)),
            row_gap: px(3),
            ..default()
        },
    ))
    .with_children(|block| {
        block
            .spawn(Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            })
            .with_children(|line| {
                line.spawn((
                    Text::new(label.clone()),
                    TextLayout {
                        linebreak: LineBreak::NoWrap,
                        ..default()
                    },
                    TextFont {
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR_MUTED),
                ));
                line.spawn(unit_text(&label, unit, slot));
            });
        for (index, (lead, tint)) in leads.into_iter().enumerate() {
            let mut path = field.path.clone();
            path.push(axis_step(index));
            let axis_field = InspectorField {
                path,
                ..field.clone()
            };
            block
                .spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(5),
                    ..default()
                })
                .with_children(|line| {
                    line.spawn((
                        Text::new(lead),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(tint),
                        Node {
                            width: px(9),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        // The axis letter is this box's name, so it is this
                        // box's grip: dragging Y slides the node along Y.
                        Name::new(format!("Inspector Grip {label} {lead}")),
                        InspectorDrag::new(rule),
                        axis_field.clone(),
                        observe(on_inspector_drag_start),
                        observe(on_inspector_drag),
                    ));
                    line.spawn(Node {
                        flex_grow: 1.0,
                        flex_basis: px(0),
                        min_width: px(0),
                        ..default()
                    })
                    .with_children(|box_slot| {
                        box_slot.spawn((
                            Name::new(format!("Inspector Field {label} {lead}")),
                            InspectorSlot(slot),
                            InspectorAxis(index),
                            axis_field.clone(),
                            text_field(
                                TextFieldSpec::new(axes[index].clone())
                                    .max_chars(12)
                                    .dense(),
                            ),
                        ));
                    });
                });
        }
    });
}

/// Fill the list with one widget per row.
///
/// The widget names are stable - the driven walks find these by name and
/// type into them.
fn build_rows(
    list: &mut RelatedSpawnerCommands<ChildOf>,
    node: Entity,
    rows: &[InspectorRow],
    skin: UiSkin,
) {
    let mut heading: Vec<String> = Vec::new();
    for (slot, row) in rows.iter().enumerate() {
        // The group as a TREE: only the levels this row does not share with
        // the one above it are drawn, each one step further in. A turret's
        // joint tree used to repeat its whole path over every handful of rows
        // - "Root Children 2", then "Root Children 2 Muzzle" - which is the
        // same eight words the split was supposed to get rid of.
        let shared = row
            .group
            .iter()
            .zip(&heading)
            .take_while(|(now, before)| now == before)
            .count();
        for (depth, level) in row.group.iter().enumerate().skip(shared) {
            list.spawn((
                Name::new(format!("Inspector Group {level}")),
                InspectorGroup,
                Text::new(level.to_uppercase()),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR_DIM),
                // A rule under each level, so the eye can see where one part
                // of the config ends and the next begins without reading the
                // headings at all.
                Node {
                    width: percent(100),
                    margin: UiRect::top(px(8)),
                    padding: UiRect::left(px(indent(depth))),
                    border: UiRect::bottom(px(theme::BORDER_W)),
                    ..default()
                },
                BorderColor::all(theme::PHOSPHOR.with_alpha(0.16)),
            ));
        }
        row.group.clone_into(&mut heading);
        let field = InspectorField {
            node,
            root: row.root,
            path: row.path.clone(),
            optional: row.optional,
        };
        // The row steps in with its group, and its NAME COLUMN gives up
        // exactly what the step took: the value boxes stay in one column down
        // the whole panel however deep the tree goes.
        let step = indent(row.group.len());
        // A vector row has its own SHAPE - a name line over one box per axis -
        // so it is spawned whole rather than as a name column and a value
        // column: three boxes in the 140px value column would each hold four
        // characters.
        if let RowValue::Axes(axes) = &row.value {
            spawn_axes_row(list, row, &field, slot, axes, step);
            continue;
        }
        // A choice of THREE or more gets the same block shape. Three damage
        // types are wider than the value column, and a control that wrapped
        // inside it stacked one option per line with the row's own name
        // floating halfway down the stack.
        if let RowValue::Choice { options, chosen } = &row.value {
            if options.len() > 2 {
                spawn_choice_row(list, row, &field, slot, options, *chosen, skin, step);
                continue;
            }
        }
        if let RowValue::Driver(driver) = &row.value {
            spawn_driver_row(list, row, node, slot, *driver, skin, step);
            continue;
        }
        list.spawn((
            Name::new(format!("Inspector Row {}", row.label)),
            Node {
                padding: UiRect::left(px(step)),
                ..row_shell()
            },
        ))
        .with_children(|shell| {
            let mut label = shell.spawn(row_label(&row.label, step));
            if row.nudge > 0.0 {
                label.insert((
                    Name::new(format!("Inspector Grip {}", row.label)),
                    InspectorDrag::new(DragRule {
                        step: row.nudge,
                        limit: row.limit,
                    }),
                    field.clone(),
                    observe(on_inspector_drag_start),
                    observe(on_inspector_drag),
                ));
            }
            shell
                .spawn(value_column())
                .with_children(|value| match &row.value {
                    // One arm: a number is TYPED the way a name is, and the
                    // scrub it also answers to rides on the row's name, not in
                    // this column.
                    RowValue::Text(text) | RowValue::Number(text) => {
                        // The placeholder is the OPTIONAL row's whole affordance:
                        // an empty box that says "none" is what tells a builder
                        // that emptying it is allowed.
                        let mut spec = TextFieldSpec::new(text.clone()).max_chars(64).dense();
                        if row.optional {
                            spec = spec.placeholder("none");
                        }
                        // The box sits in a slot of its own so the unit can
                        // stand beside it: the field is `width: 100%`, and two
                        // of those in one row is one of them off the panel.
                        value
                            .spawn(Node {
                                flex_grow: 1.0,
                                flex_basis: px(0),
                                min_width: px(0),
                                ..default()
                            })
                            .with_children(|box_slot| {
                                box_slot.spawn((
                                    Name::new(format!("Inspector Field {}", row.label)),
                                    InspectorSlot(slot),
                                    field.clone(),
                                    text_field(spec),
                                ));
                            });
                        // The picker, for a row the config said names
                        // something: what the document holds is a list only
                        // the document can write.
                        if let Some(names) = row.names {
                            value.spawn((
                                Name::new(format!("Inspector Ref {}", row.label)),
                                InspectorSlot(slot),
                                InspectorRef {
                                    label: row.label.clone(),
                                    names,
                                },
                                field.clone(),
                                Button,
                                Hovered::default(),
                                ref_chip(),
                                observe(on_open_ref_window),
                            ));
                        }
                        value.spawn(unit_text(&row.label, row.unit, slot));
                    }
                    RowValue::Colour(text) => {
                        // The swatch comes FIRST so a column of them reads as a
                        // palette down the panel: the eye finds the colour it
                        // wants without parsing six hex digits a row.
                        //
                        // And it OPENS the picker. A colour is the one value in
                        // a config that nobody can author by reading it, so the
                        // block showing it is the natural thing to press.
                        value.spawn((
                            Name::new(format!("Inspector Swatch {}", row.label)),
                            InspectorSlot(slot),
                            InspectorSwatch {
                                label: row.label.clone(),
                            },
                            field.clone(),
                            Button,
                            Hovered::default(),
                            swatch(parse_colour(text)),
                            observe(on_open_colour_window),
                        ));
                        value.spawn((
                            Name::new(format!("Inspector Field {}", row.label)),
                            InspectorSlot(slot),
                            field.clone(),
                            text_field(TextFieldSpec::new(text.clone()).max_chars(9).dense()),
                        ));
                    }
                    RowValue::Choice { options, chosen } => {
                        spawn_choice_options(
                            value, &row.label, &field, slot, options, *chosen, skin,
                        );
                    }
                    RowValue::Flag(on) => {
                        value.spawn((
                            Name::new(format!("Inspector Flag {}", row.label)),
                            InspectorSlot(slot),
                            InspectorFlag,
                            field.clone(),
                            Button,
                            Hovered::default(),
                            checkbox(*on, skin),
                            observe(on_inspector_flag),
                        ));
                    }
                    // Both spawned whole above, before the row shell exists:
                    // three axis boxes and three driver options each need the
                    // panel's width, not the value column's share of it.
                    RowValue::Axes(_) | RowValue::Driver(_) => {}
                    RowValue::Key(binding) => {
                        // The ROW is the button. A binding named on one surface
                        // and armed from another - the top bar's Rebind - left
                        // this row as text beside a verb, with no way to guess
                        // the two were about the same thing.
                        value.spawn((
                            Name::new("Inspector Key"),
                            InspectorKey,
                            ThemedButton,
                            ButtonVariant::Ghost,
                            Button,
                            Hovered::default(),
                            Node {
                                padding: UiRect::axes(px(8), px(3)),
                                border: UiRect::all(px(theme::BORDER_W)),
                                border_radius: BorderRadius::all(px(theme::RADIUS)),
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            // The ghost face, spelled out: the button reconciler
                            // repaints this on the frame it appears and on every
                            // hover after, so these are the colours it lands on.
                            BorderColor::all(theme::PHOSPHOR.with_alpha(0.25)),
                            BackgroundColor(Color::NONE),
                            observe(on_rebind_action),
                            children![(
                                Name::new("Inspector Key Text"),
                                ButtonLabel,
                                InspectorSlot(slot),
                                InspectorFixed,
                                Text::new(binding.clone()),
                                TextFont {
                                    font_size: FontSize::Px(11.0),
                                    ..default()
                                },
                                TextColor(theme::PHOSPHOR),
                            )],
                        ));
                    }
                    RowValue::Fixed(text) => {
                        value.spawn((
                            Name::new(format!("Inspector Readout {}", row.label)),
                            InspectorSlot(slot),
                            InspectorFixed,
                            Text::new(text.clone()),
                            TextLayout {
                                linebreak: LineBreak::NoWrap,
                                ..default()
                            },
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(theme::PHOSPHOR_MUTED),
                        ));
                    }
                });
        });
    }
}

/// What the list is showing, so a frame that changed a number does not respawn
/// the box that holds it.
///
/// The NODE is part of the signature: two asteroids have the same rows, and
/// switching between them still has to rebuild - the widgets carry the entity
/// they write to.
#[derive(Default)]
pub(crate) struct ShownInspector {
    shape: Option<(Entity, Vec<(String, FieldRoot, Vec<PathStep>)>)>,
}

/// The events editor with nothing picked: the panel stays, and says so.
///
/// `Entity::PLACEHOLDER` as the remembered shape is what keeps this to one
/// pass - it is a node no row can carry, so the next frame reads the state as
/// unchanged and the list is not cleared again.
fn empty_editor(
    commands: &mut Commands,
    lists: &Query<Entity, With<InspectorList>>,
    titles: &mut Query<&mut Text, With<InspectorTitle>>,
    node_ids: &mut Query<&mut TextSpan, With<InspectorId>>,
    shown: &mut ShownInspector,
) {
    let waiting = (Entity::PLACEHOLDER, Vec::new());
    if shown.shape.as_ref() == Some(&waiting) {
        return;
    }
    for mut text in titles {
        text.0 = "pick a row in the script".to_string();
    }
    for mut span in node_ids {
        span.0.clear();
    }
    if let Ok(list) = lists.single() {
        commands.entity(list).despawn_related::<Children>();
    }
    shown.shape = Some(waiting);
}

/// Show the inspected node: rebuild the rows when WHICH rows exist changes,
/// and repaint their values otherwise.
///
/// One system rather than two because both halves need the same rows, and a
/// reflection walk is the expensive part of either.
pub(crate) fn sync_inspector(
    mut commands: Commands,
    skin: Res<UiSkin>,
    document: Document,
    mut panels: Query<&mut Node, With<InspectorPanel>>,
    mut titles: Query<&mut Text, With<InspectorTitle>>,
    mut node_ids: Query<&mut TextSpan, With<InspectorId>>,
    lists: Query<Entity, With<InspectorList>>,
    fresh: Query<(), Added<InspectorList>>,
    mut shown: Local<ShownInspector>,
    // A field that is BEING typed into, or that refused what was typed, keeps
    // its text: repainting the document value over a refusal would answer
    // "min 0" with a number that is already fine.
    mut fields: Query<
        (&InspectorSlot, Option<&InspectorAxis>, &mut TextFieldValue),
        (Without<TextFieldFocused>, Without<TextFieldError>),
    >,
    mut readouts: Query<
        (&InspectorSlot, &mut Text),
        (
            With<InspectorFixed>,
            Without<InspectorTitle>,
            Without<InspectorGroup>,
        ),
    >,
    flags: Query<
        (
            &InspectorSlot,
            &Children,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (With<InspectorFlag>, Without<InspectorSwatch>),
    >,
    mut glyphs: Query<
        (&mut Text, &mut TextColor),
        (Without<InspectorFixed>, Without<InspectorTitle>),
    >,
    drivers: Query<(Entity, &InspectorSlot, &InspectorDriver, Has<Selected>)>,
    // `Without` the checkbox, so the two mutable `BackgroundColor` queries are
    // provably disjoint - a filter Bevy cannot prove is a panic at system init.
    mut swatches: Query<
        (&InspectorSlot, &mut BackgroundColor),
        (With<InspectorSwatch>, Without<InspectorFlag>),
    >,
    choices: Query<(Entity, &InspectorSlot, &InspectorChoice, Has<Selected>)>,
) {
    // A fresh list holds no rows whatever this `Local` remembers - it survives
    // the state round-trip that despawned the panel. The same trap
    // `sync_scene_list` documents.
    if !fresh.is_empty() {
        shown.shape = None;
    }
    let inspection = document.inspection();
    // In Scene the panel is a detail view of a selection and goes away with it;
    // in Events it is the editor the mode put on the screen, so it STAYS and
    // says what it is waiting for. A screen that empties to the stage the
    // moment a row is deselected would be the mode switching itself back.
    let events = document.is_events();
    for mut node in &mut panels {
        let display = if inspection.is_some() || events {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    let Some((target, rows)) = inspection else {
        if events {
            empty_editor(
                &mut commands,
                &lists,
                &mut titles,
                &mut node_ids,
                &mut shown,
            );
        } else {
            shown.shape = None;
        }
        return;
    };
    let title = document.title(target);
    for mut text in &mut titles {
        if text.0 != title {
            text.0.clone_from(&title);
        }
    }
    // Its own line under the title: the id is long, and a name and an id on
    // one line is a line that wraps mid-word.
    let said = format!("\n{}", document.id_of(target.node()));
    for mut span in &mut node_ids {
        if span.0 != said {
            span.0.clone_from(&said);
        }
    }
    let Ok(list) = lists.single() else {
        return;
    };
    let shape = (
        target.node(),
        // A row's heading is derived from its path, so a changed heading is a
        // changed path and this signature already catches it.
        rows.iter()
            .map(|row| (row.label.clone(), row.root, row.path.clone()))
            .collect::<Vec<_>>(),
    );
    if shown.shape.as_ref() != Some(&shape) {
        commands.entity(list).despawn_related::<Children>();
        commands.entity(list).with_children(|list| {
            build_rows(list, target.node(), &rows, *skin);
        });
        shown.shape = Some(shape);
        // The widgets just queued do not exist yet, and the ones that do have
        // been queued for despawn. Painting either is a write to an entity
        // that is about to stop existing - and they are built with the values
        // this pass would have written.
        return;
    }
    // Repaint in place. A FOCUSED field is skipped by its own query filter:
    // it holds what the builder is typing, and the document still holds what
    // it held before Enter. Overwriting it would delete the edit one character
    // in.
    for (slot, axis, mut value) in &mut fields {
        let text = match (rows.get(slot.0).map(|row| &row.value), axis) {
            (
                Some(RowValue::Text(text) | RowValue::Number(text) | RowValue::Colour(text)),
                None,
            ) => text,
            // One row, three boxes: the box says which of the three numbers it
            // is holding.
            (Some(RowValue::Axes(axes)), Some(axis)) => {
                let Some(text) = axes.get(axis.0) else {
                    continue;
                };
                text
            }
            _ => continue,
        };
        if value.0 != *text {
            value.0.clone_from(text);
        }
    }
    for (slot, mut block) in &mut swatches {
        let Some(RowValue::Colour(text)) = rows.get(slot.0).map(|row| &row.value) else {
            continue;
        };
        let wanted = parse_colour(text).unwrap_or(Color::NONE);
        if block.0 != wanted {
            *block = wanted.into();
        }
    }
    for (slot, mut text) in &mut readouts {
        let Some(RowValue::Fixed(wanted) | RowValue::Key(wanted)) =
            rows.get(slot.0).map(|row| &row.value)
        else {
            continue;
        };
        if text.0 != *wanted {
            text.0.clone_from(wanted);
        }
    }
    for (slot, children, mut background, mut border) in flags {
        let Some(RowValue::Flag(on)) = rows.get(slot.0).map(|row| &row.value) else {
            continue;
        };
        let (fill, edge, glyph_colour) = checkbox_colors(*on, *skin);
        if background.0 != fill {
            *background = fill.into();
            border.set_all(edge);
        }
        let mark = checkbox_glyph(*on);
        for &child in children {
            let Ok((mut text, mut colour)) = glyphs.get_mut(child) else {
                continue;
            };
            if text.0 != mark {
                text.0 = mark.to_string();
            }
            if colour.0 != glyph_colour {
                colour.0 = glyph_colour;
            }
        }
    }
    for (entity, slot, option, marked) in &choices {
        let Some(RowValue::Choice { options, chosen }) = rows.get(slot.0).map(|row| &row.value)
        else {
            continue;
        };
        let wanted = options
            .get(*chosen)
            .is_some_and(|name| *name == option.variant);
        match (wanted, marked) {
            (true, false) => {
                commands.entity(entity).insert(Selected);
            }
            (false, true) => {
                commands.entity(entity).remove::<Selected>();
            }
            _ => {}
        }
    }
    for (entity, slot, option, marked) in &drivers {
        let Some(RowValue::Driver(driver)) = rows.get(slot.0).map(|row| &row.value) else {
            continue;
        };
        match (*driver == option.driver, marked) {
            (true, false) => {
                commands.entity(entity).insert(Selected);
            }
            (false, true) => {
                commands.entity(entity).remove::<Selected>();
            }
            _ => {}
        }
    }
}

/// Say WHY a box is red, in the slot the unit stands in.
///
/// Its own system rather than part of the repaint, because a refusal OUTLIVES
/// the edit that caused it: the box keeps the refused text until it is
/// corrected, and the reason has to keep with it.
pub(crate) fn paint_field_reasons(
    refused: Query<(&InspectorSlot, &TextFieldError)>,
    mut units: Query<(
        &InspectorSlot,
        &InspectorUnit,
        Has<InspectorFault>,
        &mut Text,
        &mut TextColor,
    )>,
) {
    for (slot, unit, unresolved, mut text, mut colour) in &mut units {
        let reason = refused
            .iter()
            .find(|(refused, _)| refused.0 == slot.0)
            .map(|(_, error)| error.0.as_str());
        // A refusal FIRST: it is about what was just typed, and the reference
        // it names nothing is the state that was already there.
        let (wanted, tint) = match (reason, unresolved) {
            (Some(reason), _) => (reason, theme::semantic::THREAT),
            (None, true) => (UNRESOLVED, theme::semantic::THREAT),
            (None, false) => (unit.0, theme::PHOSPHOR_DIM),
        };
        if text.0 != wanted {
            text.0 = wanted.to_string();
        }
        if colour.0 != tint {
            colour.0 = tint;
        }
    }
}

/// Mark the reference rows whose text names nothing the document holds.
///
/// Its own system, and not part of the repaint, because the answer changes
/// from OUTSIDE the panel: an object deleted on the stage makes a handler on
/// another node wrong without a row of it moving.
pub(crate) fn sync_reference_faults(
    mut commands: Commands,
    ids: DocumentIds,
    refs: Query<(&InspectorSlot, &InspectorRef)>,
    boxes: Query<(&InspectorSlot, &TextFieldValue)>,
    units: Query<(Entity, &InspectorSlot, Has<InspectorFault>), With<InspectorUnit>>,
) {
    if refs.is_empty() {
        // The panel is on a node with no references at all, which is most of
        // them: nothing to name, so nothing to look up.
        for (entity, _, faulted) in &units {
            if faulted {
                commands.entity(entity).remove::<InspectorFault>();
            }
        }
        return;
    }
    let names = ids.names();
    for (entity, slot, faulted) in &units {
        let wrong = refs
            .iter()
            .find(|(row, _)| row.0 == slot.0)
            .and_then(|(_, chip)| {
                let text = boxes.iter().find(|(row, _)| row.0 == slot.0)?;
                Some(!names.resolves(chip.names, &text.1 .0))
            })
            .unwrap_or(false);
        match (wrong, faulted) {
            (true, false) => {
                commands.entity(entity).insert(InspectorFault);
            }
            (false, true) => {
                commands.entity(entity).remove::<InspectorFault>();
            }
            _ => {}
        }
    }
}

/// Light the border of the colour block the pointer is over.
///
/// The block IS the button that opens the picker, and a button that never
/// changes under the pointer reads as a readout. It cannot answer with its
/// background the way every other button does - that background is the value
/// it is showing - so the border is the whole of the affordance.
pub(crate) fn paint_swatch_hover(
    mut swatches: Query<(&Hovered, &mut BorderColor), With<InspectorSwatch>>,
) {
    for (hovered, mut border) in &mut swatches {
        let wanted = if hovered.get() {
            theme::PHOSPHOR
        } else {
            theme::PHOSPHOR.with_alpha(0.4)
        };
        if border.top != wanted {
            border.set_all(wanted);
        }
    }
}

/// Where an edit lands on the node.
#[derive(SystemParam)]
pub(crate) struct EditTargets<'w, 's> {
    catalog: Option<Res<'w, GameSections>>,
    ships: Query<'w, 's, &'static mut ShipNode>,
    sections: Query<'w, 's, &'static mut SectionNode>,
    objects: Query<'w, 's, &'static mut ObjectNode>,
    events: Query<'w, 's, &'static mut EventNode>,
    filters: Query<'w, 's, &'static mut FilterNode>,
    actions: Query<'w, 's, &'static mut ActionNode>,
    steps: Query<'w, 's, &'static mut StepNode>,
    gates: Query<'w, 's, &'static mut GateNode>,
    expressions: Query<'w, 's, &'static mut ExpressionNode>,
    poses: Query<'w, 's, &'static mut Transform>,
    stale: MessageWriter<'w, ObjectBodyStale>,
}

/// Run `edit` against a component's innards WITHOUT marking it changed, and
/// mark it only where the edit took.
///
/// A refusal that marks the component anyway is a frame of work for everything
/// watching it - a transform propagation, a body rebuilt from a config that did
/// not move - and a held scrub can refuse once a frame for as long as it is
/// held.
fn if_it_took<T: Component<Mutability = Mutable>, R>(
    held: &mut Mut<T>,
    edit: impl FnOnce(&mut T) -> Result<R, String>,
) -> Result<R, String> {
    let took = edit(held.bypass_change_detection());
    if took.is_ok() {
        held.set_changed();
    }
    took
}

impl EditTargets<'_, '_> {
    /// Hand `edit` the value `field` points at.
    ///
    /// The ROUTING is here and the operation is the caller's, so typing a
    /// number and ticking a checkbox reach a section, an object and a pose the
    /// same way - including the copy-on-write a catalog-backed section needs.
    pub(crate) fn edit(
        &mut self,
        field: &InspectorField,
        edit: impl FnOnce(&mut dyn PartialReflect, &[PathStep], bool) -> Result<(), String>,
    ) -> Result<(), String> {
        match field.root {
            FieldRoot::Label => {
                if let Ok(mut ship) = self.ships.get_mut(field.node) {
                    return if_it_took(&mut ship, |ship| {
                        edit(&mut ship.name, &field.path, field.optional)
                    });
                }
                let mut object = self
                    .objects
                    .get_mut(field.node)
                    .map_err(|_| GRIP_GONE.to_string())?;
                if_it_took(&mut object, |object| {
                    edit(&mut object.name, &field.path, field.optional)
                })
            }
            FieldRoot::Pose => {
                let mut pose = self
                    .poses
                    .get_mut(field.node)
                    .map_err(|_| GRIP_GONE.to_string())?;
                if_it_took(&mut pose, |pose| {
                    edit(&mut pose.translation, &field.path, field.optional)
                })
            }
            FieldRoot::Rotation => {
                let mut pose = self
                    .poses
                    .get_mut(field.node)
                    .map_err(|_| GRIP_GONE.to_string())?;
                // Degrees on the way out, degrees on the way back: the edit
                // never sees the quat. A refusal leaves the pose alone, which
                // is why the rotation is only rebuilt once the edit took.
                if_it_took(&mut pose, |pose| {
                    let mut degrees = rotation_degrees(pose);
                    edit(&mut degrees, &field.path, field.optional)?;
                    pose.rotation = rotation_from_degrees(degrees);
                    Ok(())
                })
            }
            // Handled before the routing: a kind is swapped on the node, not
            // written through a path. See [`on_inspector_choice`].
            FieldRoot::Kind => Err("not a field".to_string()),
            FieldRoot::Config => {
                if self.is_script(field.node) {
                    return self.edit_script(field, edit);
                }
                if let Ok(mut section) = self.sections.get_mut(field.node) {
                    return if_it_took(&mut section, |section| {
                        let config = editable_config(section, self.catalog.as_deref())
                            .ok_or_else(|| "no catalog entry".to_string())?;
                        edit(
                            section_config_mut(&mut config.kind),
                            &field.path,
                            field.optional,
                        )
                    });
                }
                let mut object = self
                    .objects
                    .get_mut(field.node)
                    .map_err(|_| GRIP_GONE.to_string())?;
                let took = if_it_took(&mut object, |object| {
                    let config = object_config_mut(&mut object.kind)
                        .ok_or_else(|| "not authored here".to_string())?;
                    edit(config, &field.path, field.optional)
                });
                // Only a config edit that TOOK, on a field the body is drawn
                // from, makes it stale. A name, a pose, a refusal and a rock's
                // seed all leave the mesh exactly as it was.
                if took.is_ok() && body_is_drawn_from(&object.kind, &field.path) {
                    self.stale.write(ObjectBodyStale(field.node));
                }
                took
            }
        }
    }

    /// Whether `node` belongs to the script rather than to the world.
    fn is_script(&self, node: Entity) -> bool {
        self.events.contains(node)
            || self.filters.contains(node)
            || self.actions.contains(node)
            || self.steps.contains(node)
            || self.gates.contains(node)
            || self.expressions.contains(node)
    }

    /// Hand `edit` a SCRIPT node's config.
    ///
    /// Its own arm because the five kinds share nothing with the world's nodes:
    /// no pose to keep, no body to rebuild, and no catalog entry to copy before
    /// writing. A handler, a step and a gate ARE their config; a filter and an
    /// action carry one inside the kind they hold.
    fn edit_script(
        &mut self,
        field: &InspectorField,
        edit: impl FnOnce(&mut dyn PartialReflect, &[PathStep], bool) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Ok(mut event) = self.events.get_mut(field.node) {
            return if_it_took(&mut event, |event| edit(event, &field.path, field.optional));
        }
        if let Ok(mut step) = self.steps.get_mut(field.node) {
            return if_it_took(&mut step, |step| edit(step, &field.path, field.optional));
        }
        if let Ok(mut gate) = self.gates.get_mut(field.node) {
            return if_it_took(&mut gate, |gate| edit(gate, &field.path, field.optional));
        }
        if let Ok(mut filter) = self.filters.get_mut(field.node) {
            return if_it_took(&mut filter, |filter| {
                let config = filter_config_mut(&mut filter.kind)
                    .ok_or_else(|| "nothing to author".to_string())?;
                edit(config, &field.path, field.optional)
            });
        }
        if let Ok(mut expression) = self.expressions.get_mut(field.node) {
            return if_it_took(&mut expression, |expression| {
                let config = expr_config_mut(&mut expression.kind)
                    .ok_or_else(|| "an operator holds its operands, not a value".to_string())?;
                edit(config, &field.path, field.optional)
            });
        }
        let mut action = self
            .actions
            .get_mut(field.node)
            .map_err(|_| GRIP_GONE.to_string())?;
        if_it_took(&mut action, |action| {
            let config = action_config_mut(&mut action.kind)
                .ok_or_else(|| "nothing to author".to_string())?;
            edit(config, &field.path, field.optional)
        })
    }
}

/// Commit a typed field to the document on Enter, or when the pointer leaves
/// it.
///
/// A refusal is written back ONTO THE FIELD as its error rather than logged:
/// the builder typed it, so the builder is who has to be told. The text stays
/// on screen so it can be corrected instead of retyped.
pub(crate) fn apply_inspector_edits(
    mut commands: Commands,
    mut submitted: MessageReader<TextFieldSubmitted>,
    fields: Query<&InspectorField>,
    mut targets: EditTargets,
) {
    for TextFieldSubmitted { entity, value } in submitted.read() {
        let Ok(field) = fields.get(*entity) else {
            continue;
        };
        let written = targets.edit(field, |root, path, optional| {
            write_field(root, path, optional, value)
        });
        match written {
            Ok(()) => {
                commands.entity(*entity).remove::<TextFieldError>();
            }
            Err(reason) => {
                commands.entity(*entity).insert(TextFieldError(reason));
            }
        }
    }
}

/// Begin a scrub from a clean slate.
///
/// The pixels a previous drag left over, and a warp whose echo its release beat
/// out, both belong to that drag. Either one carried into this drag moves the
/// number before the pointer has.
pub(crate) fn on_inspector_drag_start(
    drag: On<Pointer<DragStart>>,
    mut grips: Query<&mut InspectorDrag>,
) {
    let Ok(mut grip) = grips.get_mut(drag.entity) else {
        return;
    };
    grip.residual = 0.0;
    grip.warped = 0.0;
}

/// Scrub a number by dragging its name.
///
/// Continuous: every frame of the drag moves the value by that frame's pixels,
/// so the number under the pointer is the number the document holds - there is
/// no committed-on-release state to lose.
///
/// One pixel is one step of the ROW's rule, and the same rule then says where
/// the step lands. Reading the rule twice is what stalled this grip: the second
/// read only had the path of one vector component to go on, and `x` matches no
/// declaration, so a 0.05 field snapped to a 0.1 grid and came straight back.
///
/// A refusal is SAID rather than written onto a box: the grip is the row's
/// name, and a name has nowhere to wear an error.
pub(crate) fn on_inspector_drag(
    drag: On<Pointer<Drag>>,
    mut commands: Commands,
    mut grips: Query<(&InspectorField, &mut InspectorDrag)>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    refused: Query<(Entity, &InspectorField), With<TextFieldError>>,
    mut targets: EditTargets,
    mut says: EditorSays,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    let Ok((field, mut grip)) = grips.get_mut(drag.entity) else {
        return;
    };
    let travel = grip.residual + drag.delta.x - grip.warped;
    grip.residual = travel.fract();
    grip.warped = match windows.single_mut() {
        Ok(mut window) => wrapped(&mut window, drag.pointer_location.position, drag.delta.x),
        Err(_) => 0.0,
    };
    let steps = f64::from(travel.trunc());
    if steps == 0.0 {
        return;
    }
    let rule = grip.rule;
    let moved = targets.edit(field, |root, path, optional| {
        nudge_field(root, path, optional, rule, steps)
    });
    if let Err(reason) = moved {
        says.refuse(reason);
        return;
    }
    // A box left in the refused state is held out of the repaint, so the number
    // it is showing would stand while the scrub moved the document underneath
    // it. The scrub is the correction; the refusal it corrects goes with it.
    for (box_entity, _) in refused.iter().filter(|(_, held)| *held == field) {
        commands.entity(box_entity).remove::<TextFieldError>();
    }
}

/// Put the pointer back on the far side of the window when a scrub reaches an
/// edge, and answer how far that moved it.
///
/// A drag that runs out of screen is a drag that stops, and a step of 0.05 runs
/// out of screen long before an ordinary coordinate is where it should be.
///
/// Only a drag travelling INTO the edge wraps. Sitting on the edge and easing
/// back the other way is the gesture that corrects an overshoot, and teleporting
/// it across the window would be the last thing it wants.
fn wrapped(window: &mut Window, at: Vec2, travel: f32) -> f32 {
    let width = window.width();
    if width < WRAP_EDGE * 4.0 {
        return 0.0;
    }
    let landing = if at.x <= WRAP_EDGE && travel < 0.0 {
        width - WRAP_EDGE * 2.0
    } else if at.x >= width - WRAP_EDGE && travel > 0.0 {
        WRAP_EDGE * 2.0
    } else {
        return 0.0;
    };
    window.set_cursor_position(Some(Vec2::new(landing, at.y)));
    landing - at.x
}

/// Flip a `bool` field.
pub(crate) fn on_inspector_flag(
    activate: On<Activate>,
    boxes: Query<&InspectorField, With<InspectorFlag>>,
    mut targets: EditTargets,
    mut says: EditorSays,
) {
    let Ok(field) = boxes.get(activate.entity) else {
        return;
    };
    let flipped = targets.edit(field, |root, path, _| {
        toggle_field(root, path)
            .map(|_| ())
            .ok_or_else(|| "not a flag".to_string())
    });
    if let Err(reason) = flipped {
        says.refuse(reason);
    }
}

/// Switch a unit-enum field to the variant this segment names.
///
/// Its own entry point rather than a `write_field` of the variant name: the
/// value is not parsed out of text, it is one of a set the row already knows.
pub(crate) fn on_inspector_choice(
    activate: On<Activate>,
    mut commands: Commands,
    options: Query<(&InspectorField, &InspectorChoice)>,
    mut targets: EditTargets,
    mut says: EditorSays,
) {
    let Ok((field, option)) = options.get(activate.entity) else {
        return;
    };
    // The KIND row is not a field of anything: it replaces the config the rest
    // of the panel is walked from, so it is a swap on the node rather than a
    // write through a path.
    if field.root == FieldRoot::Kind {
        let (node, kind) = (field.node, option.variant.clone());
        commands.queue(move |world: &mut World| retype_script_node(world, node, &kind));
        return;
    }
    let chosen = targets.edit(field, |root, path, _| {
        choose_field(root, path, &option.variant)
    });
    if let Err(reason) = chosen {
        says.refuse(reason);
    }
}

/// Hand a ship to the player, to the AI, or to nobody.
///
/// The side goes with the controls. A hull taken off the player and given to a
/// pilot has to land somewhere, and the engine's answer for an unstated AI
/// allegiance is ENEMY - so a derelict flipped to AI would open fire. See
/// [`default_allegiance`].
pub(crate) fn on_inspector_driver(
    activate: On<Activate>,
    options: Query<&InspectorDriver>,
    mut ships: Query<(Entity, &mut ShipNode, &NodeId)>,
    mut says: EditorSays,
) {
    let Ok(option) = options.get(activate.entity) else {
        return;
    };
    // One ship flies. Lowering keeps the LAST Player ship it reads and routes
    // the rest to the standing fleet, so a second one is a ship the document
    // quietly loses on the next save - refuse it while it is still a click.
    if option.driver == ShipDriver::Player {
        let flown = ships
            .iter()
            .find(|(entity, ship, _)| *entity != option.ship && ship.driver == ShipDriver::Player)
            .map(|(_, ship, id)| super::tree_text(&ship.name, &id.0).0);
        if let Some(name) = flown {
            says.refuse(format!("{name} already flies - set it to AI first"));
            return;
        }
    }
    let Ok((_, mut ship, _)) = ships.get_mut(option.ship) else {
        return;
    };
    if ship.driver != option.driver {
        ship.driver = option.driver;
        ship.allegiance = default_allegiance(option.driver);
    }
}

#[cfg(test)]
mod tests;
