//! What the inspector shows for a node, and what typing into it changes.
//!
//! The rows are READ OFF THE CONFIG ITSELF by reflection rather than listed by
//! hand: a thruster's inspector is whatever `ThrusterSectionConfig` has fields
//! for, so a config that grows a field grows a row, and no editor code names
//! `magnitude` anywhere. That is the whole reason this module exists - the
//! alternative is a match arm per kind that goes stale the first time content
//! changes underneath it.
//!
//! There is exactly one hand-made row ([`RowValue::Driver`]): who flies a ship
//! is a property of the editor's own [`ShipNode`], not of any authored config,
//! so there is nothing to reflect over.
//!
//! WHAT IT REFUSES TO EDIT IT STILL SHOWS. A field the parser has no leaf for -
//! an asset path, a socket list - becomes a [`RowValue::Fixed`] row with its
//! debug text, greyed. Hiding it would say the config does not have it.

use bevy::{
    prelude::*,
    reflect::{
        enums::{DynamicEnum, DynamicVariant, VariantInfo},
        tuple::DynamicTuple,
        tuple_struct::DynamicTupleStruct,
        NamedField, ReflectMut, ReflectRef, TypeInfo, Typed,
    },
};
use nova_events::units::prelude::*;
use nova_gameplay::prelude::AssetRef;
use nova_input::prelude::source_label;
use nova_scenario::prelude::{
    Names, ScenarioObjectKind, SectionSource, VariableConditionNode, VariableExpressionNode,
    ASTEROID_KIND_SUMMARIES,
};
use nova_ship::prelude::{GameSections, SectionConfig, SectionKind};

use crate::{
    asset_index::prelude::AssetSort,
    config::SelectedNode,
    event::{
        action_choice, action_config, event_label, expr_choice, filter_choice, filter_config,
        handler_text, ActionChoice, ActionNode, EventNode, ExprChoice, ExprKind, ExpressionNode,
        FilterChoice, FilterNode, GateNode, ScriptNodes, StepNode,
    },
    node::{
        objects_of, EditContext, EditorNode, ObjectNode, ObjectNodes, ScenarioNode, SectionNode,
        ShipDriver, ShipNode, ShipNodes,
    },
    scenario::PLAYER_ID,
};

/// The document nodes the inspector can be pointed at, so one `get` answers
/// which kind of panel to draw.
pub(crate) type NodeKinds<'w, 's> = Query<
    'w,
    's,
    (
        Has<ScenarioNode>,
        Has<ShipNode>,
        Has<SectionNode>,
        Has<ObjectNode>,
        Has<EventNode>,
        Has<FilterNode>,
        Has<ActionNode>,
        Has<StepNode>,
        Has<GateNode>,
    ),
    With<EditorNode>,
>;

/// The node the inspector is reporting on, tagged with what it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InspectTarget {
    /// The document root.
    Scenario(Entity),
    /// A ship, entered or standing beside the one that is.
    Ship(Entity),
    /// One section of the entered ship.
    Section(Entity),
    /// One non-ship thing in the world.
    Object(Entity),
    /// One handler of the script.
    Event(Entity),
    /// One filter of a handler or of a gate.
    Filter(Entity),
    /// One action of a handler or of a step.
    Action(Entity),
    /// One beat of a sequence.
    Step(Entity),
    /// The event one beat waits for.
    Gate(Entity),
}

impl InspectTarget {
    /// The node itself.
    pub(crate) fn node(self) -> Entity {
        match self {
            InspectTarget::Scenario(node)
            | InspectTarget::Ship(node)
            | InspectTarget::Section(node)
            | InspectTarget::Object(node)
            | InspectTarget::Event(node)
            | InspectTarget::Filter(node)
            | InspectTarget::Action(node)
            | InspectTarget::Step(node)
            | InspectTarget::Gate(node) => node,
        }
    }

    /// The word the panel's tag wears.
    pub(crate) fn tag(self) -> &'static str {
        match self {
            InspectTarget::Scenario(_) => "SCENARIO",
            InspectTarget::Ship(_) => "SHIP",
            InspectTarget::Section(_) => "SECTION",
            InspectTarget::Object(_) => "OBJECT",
            InspectTarget::Event(_) => "HANDLER",
            InspectTarget::Filter(_) => "FILTER",
            InspectTarget::Action(_) => "ACTION",
            InspectTarget::Step(_) => "STEP",
            InspectTarget::Gate(_) => "GATE",
        }
    }
}

/// What the inspector is pointed at: the SELECTION, or - with nothing selected
/// - the node the editor is standing in.
///
/// The fallback is what gives a ship an inspector at all: entering one clears
/// the selection (`ui::on_scene_row`), because selection and context are
/// different questions. Without the fallback the one node you are certainly
/// working on would be the one node with no panel.
pub(crate) fn inspected(
    selected: &SelectedNode,
    context: &EditContext,
    kinds: &NodeKinds,
) -> Option<InspectTarget> {
    let node = selected.0.or_else(|| context.current())?;
    let (scenario, ship, section, object, event, filter, action, step, gate) =
        kinds.get(node).ok()?;
    if scenario {
        return Some(InspectTarget::Scenario(node));
    }
    if ship {
        return Some(InspectTarget::Ship(node));
    }
    if section {
        return Some(InspectTarget::Section(node));
    }
    if object {
        return Some(InspectTarget::Object(node));
    }
    if event {
        return Some(InspectTarget::Event(node));
    }
    if filter {
        return Some(InspectTarget::Filter(node));
    }
    if action {
        return Some(InspectTarget::Action(node));
    }
    if step {
        return Some(InspectTarget::Step(node));
    }
    gate.then_some(InspectTarget::Gate(node))
}

/// Which of a node's parts a row edits. The reflection path is relative to
/// this, so a row never has to say "the second component of the third field of
/// the config" in one string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FieldRoot {
    /// The node's kind config, walked by reflection.
    Config,
    /// The node's pose - `Transform::translation`.
    Pose,
    /// The node's rotation, in DEGREES, as yaw/pitch/roll.
    ///
    /// Its own root rather than a path into the pose because the value on
    /// screen is not the value in the component: a `Quat` has four numbers and
    /// no builder thinks in them. The routing converts around the edit.
    Rotation,
    /// The node's display name, which is a field of the node and not of the
    /// kind config. Ships and objects both have one.
    Label,
    /// WHICH filter or action the node is.
    ///
    /// Its own root because the value is not a field of anything: switching it
    /// replaces the config the other rows are walked from, and drops the
    /// children the old kind owned. See
    /// [`retype_script_node`](crate::event::retype_script_node).
    Kind,
}

/// One step of a reflection path.
///
/// Hand-rolled rather than `bevy_reflect`'s parsed paths because this module
/// walks and resolves with the SAME code: an `Option` the walker steps through
/// transparently has to be stepped through the same way on the way back, and a
/// string path would have to encode that convention twice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PathStep {
    /// A named struct field.
    Field(String),
    /// A tuple, tuple-struct or enum-variant slot.
    Slot(usize),
    /// One element of a list.
    ///
    /// Distinct from [`PathStep::Slot`] because the two READ differently: a
    /// slot is plumbing a builder never sees (the payload inside an `Option`),
    /// while an item is one of several things they authored and has to be told
    /// apart from its siblings.
    Item(usize),
}

/// How a row reads, and so which widget draws it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RowValue {
    /// Editable as text. The string is the value in its canonical form.
    Text(String),
    /// ONE number. Typed like text, and dragged by its name, which is what
    /// makes `nan` a thing the control cannot express rather than a thing the
    /// parser has to refuse. The string is the value as it reads.
    Number(String),
    /// A colour, as `#rrggbb`. Editable as text like any other leaf, but the
    /// panel paints the colour beside the field - a builder picking a beacon's
    /// light should not have to read hex to know what they picked.
    Colour(String),
    /// A checkbox.
    Flag(bool),
    /// One of a fixed set of names, and which is chosen.
    ///
    /// Only ever built for an enum whose variants ALL carry no fields. A
    /// variant with fields cannot be offered as a choice: switching to it would
    /// mean inventing values nobody authored. See the enum arm of [`walk`].
    Choice {
        /// Every variant, in declaration order.
        options: Vec<String>,
        /// What each of them MEANS, one per option and in the same order.
        ///
        /// Carried beside the names because the picker is where a builder
        /// meets a vocabulary they do not know yet: a list of twenty-six bare
        /// names is a list to be looked up somewhere else.
        hints: Vec<String>,
        /// Which one the value currently holds.
        chosen: usize,
    },
    /// Three numbers of one vector, each typed on its own.
    ///
    /// The rows a builder reads most, so they are the rows that get their own
    /// shape: one box per axis instead of `x, y, z` in a single field, where a
    /// real position wrapped to two lines and broke the column's rhythm.
    Axes([String; 3]),
    /// Who flies this ship.
    Driver(ShipDriver),
    /// Shown but not editable here.
    Fixed(String),
    /// ONE node of a condition: which operator it is, and - where it is a leaf
    /// - the expression it holds.
    ///
    /// Its own shape because the two belong on one LINE. A page that stacked
    /// the operator over the value it applies to would be twice as tall as the
    /// tree it is drawing, and the shape of the tree is the whole reason the
    /// page exists.
    Operand {
        /// Every operator this node's PLACE allows, and the leaf.
        options: Vec<String>,
        /// Which one it is.
        chosen: usize,
        /// What the leaf holds, or `None` for an operator - which holds
        /// operands rather than a value.
        text: Option<String>,
    },
    /// The key a section fires on, and the button that arms a new one.
    ///
    /// Its own variant rather than a [`RowValue::Fixed`] beside the top bar's
    /// Rebind action: the row NAMES the binding, so the row is the thing a
    /// builder presses to change it.
    Key(String),
}

impl RowValue {
    /// The value as one line of text: what the panel paints into a readout and
    /// what a driven run reads off [`EditorProbe`](crate::EditorProbe). A
    /// checkbox and a driver segment have no text on screen, so they get the
    /// word they stand for.
    pub(crate) fn reading(&self) -> String {
        match self {
            Self::Text(text)
            | Self::Number(text)
            | Self::Colour(text)
            | Self::Fixed(text)
            | Self::Key(text) => text.clone(),
            // One line, the way the value reads on paper: three boxes are how
            // it is TYPED, not what it is.
            Self::Axes(axes) => axes.join(", "),
            Self::Flag(flag) => flag.to_string(),
            Self::Choice {
                options, chosen, ..
            } => options.get(*chosen).cloned().unwrap_or_default(),
            // The VALUE where it has one: a leaf reading `equal` would say what
            // the row is instead of what it holds.
            Self::Operand {
                options,
                chosen,
                text,
            } => text
                .clone()
                .unwrap_or_else(|| options.get(*chosen).cloned().unwrap_or_default()),
            Self::Driver(driver) => driver_label(*driver).to_string(),
        }
    }
}

/// What a driver option is called.
///
/// The scenario model's own words - a save writes `SpaceshipController::None`
/// for the third one, and a builder reading the file should find the word the
/// panel gave them.
pub(crate) fn driver_label(driver: ShipDriver) -> &'static str {
    match driver {
        ShipDriver::Player => "Player",
        ShipDriver::Ai => "AI",
        ShipDriver::Adrift => "None",
    }
}

/// One row of the inspector: what it edits, what it is called, how it reads.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InspectorRow {
    /// Which part of the node the path starts from.
    pub(crate) root: FieldRoot,
    /// Where the value lives inside that root. Empty when the root IS the
    /// value, as it is for a name.
    pub(crate) path: Vec<PathStep>,
    /// Set when the target is an `Option`: an empty field clears it.
    pub(crate) optional: bool,
    /// Where the row sits in the config, one segment per level, outermost
    /// first. Empty for a node's own top-level fields.
    ///
    /// A LIST rather than one joined string because the panel draws it as a
    /// tree: a row eight levels down repeats what the row above it already
    /// said, and only the panel can see which levels those are.
    pub(crate) group: Vec<String>,
    /// The row's label, WITHIN its group.
    pub(crate) label: String,
    /// What the number IS, where its name does not say it: the unit it is
    /// typed in, or the empty string for a value that has none.
    pub(crate) unit: &'static str,
    /// How far one pixel of a drag moves this row's number. Zero for a row
    /// holding something that is not a number, which has nothing to scrub.
    pub(crate) nudge: f32,
    /// What the field takes. Carried on the ROW because the grip is handed the
    /// path of one vector component, and `x` is not a name any declaration can
    /// match - resolving the rule a second time from there finds nothing.
    pub(crate) limit: Limit,
    /// The value, and the widget it implies.
    pub(crate) value: RowValue,
    /// One sentence saying what this row is FOR, or empty for a row nothing
    /// has been written about.
    ///
    /// Read out of the config author's own doc comment through
    /// `reflect_documentation` rather than kept in a list here: a hint that
    /// has to be registered a second time is one that goes stale the day the
    /// field it describes changes its mind.
    pub(crate) hint: String,
    /// What this row's string NAMES, where the config said so.
    ///
    /// Read off the [`Names`] attribute at the source rather than from a list
    /// of field names kept here: a reference row that has to be registered
    /// twice is one the editor stops drawing the day the vocabulary grows.
    pub(crate) names: Option<Names>,
    /// What FILE this row's string names, where the field's own type says so.
    ///
    /// Read off the type rather than the name: `AssetRef<Image>` wants an image
    /// whether the field is called `texture`, `icon` or `cubemap`.
    pub(crate) asset: Option<AssetSort>,
    /// Which node the row WRITES TO, where that is not the node the panel is
    /// on. `None` for every row of a config, which is the node's own.
    ///
    /// Set by a page that draws a TREE of nodes rather than one node's fields:
    /// a condition is several entities, and each of its rows edits its own.
    pub(crate) owner: Option<Entity>,
    /// How far in the row stands BEYOND its group. Zero for a walked config,
    /// whose depth is the length of its heading; a page draws a tree that has
    /// no headings to count.
    pub(crate) depth: usize,
}

impl InspectorRow {
    /// The same row, saying what it is for.
    pub(crate) fn saying(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }
}

/// One doc comment, as a panel says it.
///
/// The FIRST PARAGRAPH only: a doc comment goes on to say what the engine does
/// with the value, and a tooltip that quoted all of it would cover the rows it
/// was called to explain. Backticks go with it - the panel has one font, and a
/// builder reading `"scenario_elapsed"` in it sees the quotes as part of the
/// name.
pub(crate) fn hint_of(docs: Option<&str>) -> String {
    let Some(docs) = docs else {
        return String::new();
    };
    docs.lines()
        .map(str::trim)
        .skip_while(|line| line.is_empty())
        .take_while(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .replace('`', "")
}

/// One hint, short enough to stand under an option in a LIST.
///
/// A row's own tooltip says the whole first paragraph - it is one row, and the
/// reader asked. A picker draws twenty-six of them at once, and one option
/// whose sentence runs to nine lines pushes the rest of the vocabulary off the
/// bottom of the window.
pub(crate) fn clipped(hint: &str, chars: usize) -> String {
    if hint.chars().count() <= chars {
        return hint.to_string();
    }
    let kept: String = hint.chars().take(chars).collect();
    let cut = kept.rfind(' ').unwrap_or(kept.len());
    format!(
        "{}...",
        kept[..cut].trim_end_matches([',', ';', '-']).trim()
    )
}

/// What the OPTION a choice row stands on means, out of its own doc comment.
///
/// Keyed on the derived `Debug` of a bare variant, which IS that variant's
/// name. The row holds the choice and the sentence lives on the type, and
/// reflection is the only bridge between them - which is the whole reason the
/// editor's choice enums derive `Reflect`.
pub(crate) fn variant_hint<T: Typed + core::fmt::Debug>(chosen: &T) -> String {
    let TypeInfo::Enum(info) = T::type_info() else {
        return String::new();
    };
    hint_of(
        info.variant(&format!("{chosen:?}"))
            .and_then(VariantInfo::docs),
    )
}

/// The row a filter or an action is SWITCHED on: every kind it could be, and
/// which one it is.
///
/// A choice rather than a reading, because the vocabulary is forty-eight kinds
/// and the Add menu offers five: this row is where the other forty-three are.
fn kind_row(
    label: &str,
    options: impl Iterator<Item = &'static str>,
    hints: impl Iterator<Item = String>,
    chosen: Option<usize>,
) -> InspectorRow {
    InspectorRow {
        root: FieldRoot::Kind,
        path: Vec::new(),
        optional: false,
        group: Vec::new(),
        label: label.to_string(),
        unit: "",
        nudge: 0.0,
        limit: Limit::Free,
        value: RowValue::Choice {
            options: options.map(str::to_string).collect(),
            hints: hints.collect(),
            chosen: chosen.unwrap_or_default(),
        },
        hint: String::new(),
        names: None,
        asset: None,
        owner: None,
        depth: 0,
    }
}

/// A read-only row.
fn fixed(root: FieldRoot, label: &str, text: impl Into<String>) -> InspectorRow {
    InspectorRow {
        root,
        path: Vec::new(),
        optional: false,
        group: Vec::new(),
        label: label.to_string(),
        unit: "",
        nudge: 0.0,
        limit: Limit::Free,
        value: RowValue::Fixed(text.into()),
        hint: String::new(),
        names: None,
        asset: None,
        owner: None,
        depth: 0,
    }
}

/// A walked row: its heading and its label both come from WHERE it sits, so a
/// caller that has the path never has to name it twice.
fn walked(root: FieldRoot, path: Vec<PathStep>, optional: bool, value: RowValue) -> InspectorRow {
    let (group, label) = heading_and_label(&path);
    // A unit belongs to a NUMBER. A checkbox or a variant name has none, and
    // one drawn beside it would be a label for the wrong thing.
    let (unit, nudge, limit) = match value {
        // A vector's three numbers share one unit, and the row's own line is
        // where it goes - the same place the pose's Position row wears its.
        RowValue::Number(_) | RowValue::Axes(_) => field_spec(&path)
            .map_or(("", FREE_STEP, Limit::Free), |spec| {
                (spec.unit, spec.step, spec.limit)
            }),
        _ => ("", 0.0, Limit::Free),
    };
    InspectorRow {
        root,
        path,
        optional,
        group,
        label,
        unit,
        nudge,
        limit,
        value,
        hint: String::new(),
        names: None,
        asset: None,
        owner: None,
        depth: 0,
    }
}

/// A walked row holding a number, stepped by its own TYPE where its
/// declaration does not step it larger.
///
/// The step floor lives here rather than in the declaration table because it
/// is a property of the type: a whole field nobody has declared still cannot
/// be dragged a tenth at a time. Three declarations reached the same rule one
/// field at a time before this did it once.
fn walked_number(
    root: FieldRoot,
    path: Vec<PathStep>,
    optional: bool,
    text: String,
    whole: bool,
) -> InspectorRow {
    let mut row = walked(root, path, optional, RowValue::Number(text));
    if whole {
        row.nudge = row.nudge.max(WHOLE_STEP);
    }
    row
}

/// The values a field takes, and so what any control may put in it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Limit {
    /// Any finite number. Most fields: a floor invented for a number nobody
    /// checked would refuse an edit the runtime accepts.
    Free,
    /// No less than this. Negative mass and a negative radius are not values,
    /// they are typos.
    AtLeast(f32),
}

/// Everything the editor knows about one authored field, declared ONCE.
///
/// A field is named here to put it on a kind's first screen, to give it a unit,
/// to give it a floor, or to say how fast it drags - usually several at once.
/// The single declaration is the point: the first screen and the rule used to
/// be two lists keyed on the same names, and a name could sit in one and not
/// the other, which is how every number a turret shows got a bare box.
///
/// Lengths and speeds are declared in METERS, because that is what the file
/// under the box now holds: a `Meters` field is authored as the number the HUD
/// reads, and the row shows it as written. Nothing here converts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FieldSpec {
    /// The config's OWN field name, matched at ANY depth: a turret's fire rate
    /// lives on a muzzle inside a joint inside a list, and exactly where
    /// depends on the tree the content declares - a fixed path could not name
    /// it.
    ///
    /// A leading `*` makes it a FAMILY: any name ending in the rest is that
    /// thing, whatever it hangs off. A name declared in full always wins.
    name: &'static str,
    /// The unit shown beside the box, or the empty string for a value that has
    /// none - which includes every field that is not a number.
    unit: &'static str,
    /// What the field takes.
    limit: Limit,
    /// How far one pixel of a drag moves the number, and so the precision a
    /// drag lands on: a field stepped by `0.05` never comes out of one reading
    /// `0.30000001`.
    ///
    /// A WHOLE field takes at least [`WHOLE_STEP`] whatever is declared here.
    /// That floor is the type's, not this table's, so an integer nobody has
    /// declared still drags a whole number at a time.
    step: f32,
}

impl FieldSpec {
    /// Whether this declaration is for the field called `name`, by full name.
    fn is_named(&self, name: &str) -> bool {
        self.name == name
    }

    /// Whether this declaration covers the field called `name`, by full name or
    /// as a family.
    fn covers(&self, name: &str) -> bool {
        match self.name.strip_prefix('*') {
            Some(family) => name.ends_with(family),
            None => self.is_named(name),
        }
    }
}

/// A number that is never negative, dragged `step` per pixel.
const fn floored(name: &'static str, unit: &'static str, step: f32) -> FieldSpec {
    FieldSpec {
        name,
        unit,
        limit: Limit::AtLeast(0.0),
        step,
    }
}

/// How far one pixel of a drag moves a number nothing is declared about.
const FREE_STEP: f32 = 0.1;

/// The smallest step a WHOLE number can be dragged by.
///
/// Structural rather than declared, because [`snapped`] rounds a whole value:
/// a smaller step travels a fraction and lands back where it started, which
/// reads as a grip that does not work at all. A declaration may still ask for
/// a bigger step - it can no longer ask for one below this.
const WHOLE_STEP: f32 = 1.0;

/// A field with nothing to say about its values: a name, a flag, a colour, a
/// choice, or a number nobody has checked.
const fn plain(name: &'static str) -> FieldSpec {
    FieldSpec {
        name,
        unit: "",
        limit: Limit::Free,
        step: FREE_STEP,
    }
}

const MAGNITUDE: FieldSpec = floored("magnitude", "", 1.0);
const STEERING_LAG: FieldSpec = floored("steering_lag", "s", 0.005);
const MAX_TORQUE: FieldSpec = floored("max_torque", "", 1.0);
const FIRE_RATE: FieldSpec = floored("fire_rate", "/s", 0.05);
const MUZZLE_SPEED: FieldSpec = floored("muzzle_speed", "m/s", 5.0);
const BULLET_DAMAGE: FieldSpec = floored("bullet_damage", "hp", 0.5);
const BULLET_KIND: FieldSpec = plain("bullet_kind");
/// An asteroid's kind. The row's VOCABULARY is not declared here - see
/// [`offer_object_vocabularies`] - because a ship section has a `material` too
/// and this table matches by name at any depth.
const MATERIAL: FieldSpec = plain("material");
const AMMO_CAPACITY: FieldSpec = FieldSpec {
    name: "ammo_capacity",
    unit: "rounds",
    limit: Limit::AtLeast(1.0),
    step: 1.0,
};
const RELOAD: FieldSpec = plain("reload");
const RELOAD_DELAY: FieldSpec = FieldSpec {
    name: "delay",
    unit: "s",
    limit: Limit::AtLeast(0.02),
    step: 0.02,
};
const RELOAD_AMOUNT: FieldSpec = FieldSpec {
    name: "amount",
    unit: "rounds",
    limit: Limit::AtLeast(1.0),
    step: 1.0,
};
const PROJECTILE_LIFETIME: FieldSpec = floored("projectile_lifetime", "s", 0.05);
const SPAWNER_SPEED: FieldSpec = floored("spawner_speed", "m/s", 5.0);
const BLAST_DAMAGE: FieldSpec = floored("blast_damage", "hp", 0.5);
const BLAST_RADIUS: FieldSpec = floored("blast_radius", "m", 0.5);
const ARM_TIME: FieldSpec = floored("arm_time", "s", 0.02);
const ARM_DISTANCE: FieldSpec = floored("arm_distance", "m", 1.0);
const NAV_CONSTANT: FieldSpec = floored("nav_constant", "", 0.02);
const CHARGE_SECONDS: FieldSpec = floored("charge_seconds", "s", 0.05);
const SLUG_SPEED: FieldSpec = floored("slug_speed", "m/s", 50.0);
const SLUG_DAMAGE: FieldSpec = floored("slug_damage", "hp", 0.5);
/// Not hit points. Power is what a pierce round SPENDS crossing a layer, so
/// this is the depth control, and it is dragged in the register depth lives in.
const SLUG_POWER: FieldSpec = floored("slug_power", "", 10.0);
const SLUG_LIFETIME: FieldSpec = floored("slug_lifetime", "s", 0.05);
const RECOIL_IMPULSE: FieldSpec = floored("recoil_impulse", "", 5.0);

const BODY_RADIUS: FieldSpec = floored("body_radius", "m", 0.5);
const MASS: FieldSpec = floored("mass", "", 0.5);
const RADIUS: FieldSpec = floored("radius", "m", 0.5);
const AREA_RADIUS: FieldSpec = floored("area_radius", "m", 1.0);
const INVULNERABLE: FieldSpec = plain("invulnerable");
const PLANET_TYPE: FieldSpec = plain("planet_type");
const SEED: FieldSpec = FieldSpec {
    name: "seed",
    unit: "",
    // A seed is a NAME for a shape, not a quantity: one whole number per pixel
    // walks the shapes, and a tenth of one is the shape it already had.
    limit: Limit::Free,
    step: 1.0,
};
const HULL: FieldSpec = plain("hull");
const CONTROLLER: FieldSpec = plain("controller");
const ALLEGIANCE: FieldSpec = plain("allegiance");
const LABEL: FieldSpec = plain("label");
const COLOR: FieldSpec = plain("color");
const SIZE: FieldSpec = floored("size", "m", 0.5);
const ILLUMINANCE: FieldSpec = floored("illuminance", "lx", 50.0);
const INTENSITY: FieldSpec = floored("intensity", "lm", 50.0);
const RANGE: FieldSpec = floored("range", "m", 1.0);
const SHADOWS: FieldSpec = plain("shadows");
/// Lux, the same register the authored lights are in, so a builder comparing a
/// sky against a key light is comparing two numbers of one kind.
const SKYBOX_BRIGHTNESS: FieldSpec = floored("skybox_brightness", "lx", 50.0);
const HEALTH: FieldSpec = floored("health", "hp", 1.0);
/// The exhaust cone's cross-section, in build-grid CELLS.
///
/// Not meters, and not a mistake: the flame is a mesh the section builds
/// inside its own cell, sized against the nozzle it comes out of rather than
/// against the world. A cell is 10 m on a side, so a 0.8 here is an 8 m
/// nozzle, and the number a builder types is the fraction of the cell they
/// want lit.
const WIDTH: FieldSpec = floored("width", "cells", 0.05);
const DELAY: FieldSpec = floored("delay", "s", 0.02);
const LIFETIME: FieldSpec = floored("lifetime", "s", 0.05);
const COOLDOWN: FieldSpec = floored("cooldown", "s", 0.02);
/// The rake a lance's slug tears open around itself.
const RAKE_RADIUS: FieldSpec = floored("rake_radius", "m", 0.5);
/// The exhaust cones' radii - see [`WIDTH`] for why these are cells.
const ANY_RADIUS: FieldSpec = floored("*radius", "cells", 0.05);
/// The same, for their length along the nozzle's axis.
const ANY_HEIGHT: FieldSpec = floored("*height", "cells", 0.05);

/// What a SCENARIO builder authors on each kind, and so what the panel shows
/// before it is asked for the rest.
///
/// The editor is not a section editor. A turret's config is a joint tree with a
/// render mesh transform on every joint, and none of that is a question anyone
/// building a scenario asks - they ask how fast it fires and how hard it hits.
/// These lists say which fields those are, per kind, because the editor KNOWS
/// what it is looking at.
///
/// Nothing is lost. View > All Fields puts the whole walk back, which is what
/// makes this a first screen rather than a censor.
const THRUSTER_PICKS: &[FieldSpec] = &[MAGNITUDE];
/// A hull is a block. Its config is a mesh and a flag saying whether to draw
/// it, and neither is a decision made in a scenario.
const HULL_PICKS: &[FieldSpec] = &[];
const CONTROLLER_PICKS: &[FieldSpec] = &[STEERING_LAG, MAX_TORQUE];
const TURRET_PICKS: &[FieldSpec] = &[
    FIRE_RATE,
    MUZZLE_SPEED,
    BULLET_DAMAGE,
    BULLET_KIND,
    AMMO_CAPACITY,
    RELOAD,
    PROJECTILE_LIFETIME,
];
const TORPEDO_PICKS: &[FieldSpec] = &[
    FIRE_RATE,
    SPAWNER_SPEED,
    BLAST_DAMAGE,
    BLAST_RADIUS,
    ARM_TIME,
    ARM_DISTANCE,
    NAV_CONSTANT,
    AMMO_CAPACITY,
    RELOAD,
    PROJECTILE_LIFETIME,
];
/// Recoil is a pick, not a detail: it is the only weapon field that moves the
/// ship that fired, so a builder who cannot see it cannot explain the spin.
const RAILGUN_PICKS: &[FieldSpec] = &[
    CHARGE_SECONDS,
    SLUG_DAMAGE,
    SLUG_POWER,
    SLUG_SPEED,
    RECOIL_IMPULSE,
    AMMO_CAPACITY,
    RELOAD,
    SLUG_LIFETIME,
];
const ANCHOR_PICKS: &[FieldSpec] = &[BODY_RADIUS, MASS];
/// What the rock is MADE of comes second only to how big it is: the kind
/// decides the whole surface, so a curated panel that showed the radius and hid
/// the kind would be hiding the thing a builder came to pick.
const ASTEROID_PICKS: &[FieldSpec] = &[RADIUS, MATERIAL, MASS, INVULNERABLE, SEED];
/// A planet's first screen. `planet_type` leads because it is the field that
/// changes everything else about the body; `seed` is second for the same
/// reason it is on a rock - it picks WHICH world of that kind.
const PLANET_PICKS: &[FieldSpec] = &[PLANET_TYPE, SEED, RADIUS, MASS, INVULNERABLE];
/// The whole point of a spaceship object is WHICH ship and WHO flies it, and a
/// pick takes the field with everything under it - so the hull's source and the
/// controller's own fields come along.
const SPACESHIP_PICKS: &[FieldSpec] = &[HULL, CONTROLLER, ALLEGIANCE];
const BEACON_PICKS: &[FieldSpec] = &[LABEL, RADIUS, COLOR, AREA_RADIUS];
const SALVAGE_PICKS: &[FieldSpec] = &[SIZE, AREA_RADIUS];
/// No `aim`. The node's ROTATION aims the light (`node.rs`), and two controls
/// on one output is a builder turning the gizmo and watching nothing happen.
const LIGHT_PICKS: &[FieldSpec] = &[ILLUMINANCE, INTENSITY, COLOR, RANGE, RADIUS, SHADOWS];
/// The document root's own fields. Not a scenario OBJECT kind - the root is the
/// one node whose config is the node itself - but declared here for the same
/// reason every other field is: so the row carries its unit and its floor.
const SCENARIO_PICKS: &[FieldSpec] = &[SKYBOX_BRIGHTNESS];
/// The fields no kind shows first, which still carry a unit and a floor once
/// View > All Fields puts them back.
const UNPICKED: &[FieldSpec] = &[
    HEALTH,
    WIDTH,
    DELAY,
    LIFETIME,
    COOLDOWN,
    RAKE_RADIUS,
    ANY_RADIUS,
    ANY_HEIGHT,
];

/// Every declaration there is: each kind's first screen, then the rest.
///
/// The lookup walks THIS, so a field a kind shows cannot end up without the
/// unit, the floor and the step that screen shows it with.
const DECLARED: &[&[FieldSpec]] = &[
    THRUSTER_PICKS,
    HULL_PICKS,
    CONTROLLER_PICKS,
    TURRET_PICKS,
    TORPEDO_PICKS,
    RAILGUN_PICKS,
    ANCHOR_PICKS,
    ASTEROID_PICKS,
    PLANET_PICKS,
    SPACESHIP_PICKS,
    BEACON_PICKS,
    SALVAGE_PICKS,
    LIGHT_PICKS,
    SCENARIO_PICKS,
    UNPICKED,
];

/// The field name a path ends at, ignoring the list indices and tuple slots it
/// passes through.
fn leaf_name(path: &[PathStep]) -> Option<&str> {
    path.iter().rev().find_map(|step| match step {
        PathStep::Field(name) => Some(name.as_str()),
        PathStep::Item(_) | PathStep::Slot(_) => None,
    })
}

/// What the editor knows about the field `path` ends at, if anything.
///
/// A name declared in full beats a family, so `body_radius` can one day say
/// something `*radius` does not.
fn field_spec(path: &[PathStep]) -> Option<FieldSpec> {
    let name = leaf_name(path)?;
    let under_reload = path
        .iter()
        .any(|step| matches!(step, PathStep::Field(parent) if parent == RELOAD.name));
    if under_reload {
        match name {
            "delay" => return Some(RELOAD_DELAY),
            "amount" => return Some(RELOAD_AMOUNT),
            _ => {}
        }
    }
    let declared = || DECLARED.iter().copied().flatten();
    declared()
        .find(|spec| spec.is_named(name))
        .or_else(|| declared().find(|spec| spec.covers(name)))
        .copied()
}

/// Give a quantity's rows the unit and the step its TYPE carries, where the
/// declaration table names nothing.
///
/// The table still wins where it has an entry: `blast_radius` drags half a
/// meter a pixel because that is the register a blast is tuned in. This is
/// what every other authored quantity gets for free - a scatter box's corners,
/// a spawn's position - and it is what a build-grid CELL never gets, because a
/// cell is a bare `Vec3` and says nothing about meters.
///
/// A wrapper the units module does not own gets nothing: its number keeps the
/// plain control, with no unit and no meter-sized drag.
fn declare_by_type(type_path: &str, rows: &mut [InspectorRow]) {
    let Some(unit) = quantity_unit(type_path) else {
        return;
    };
    for row in rows {
        if !matches!(row.value, RowValue::Number(_) | RowValue::Axes(_))
            || field_spec(&row.path).is_some()
        {
            continue;
        }
        row.unit = unit;
        row.nudge = POSE_STEP;
    }
}

/// Whether the leaf holds ONE number, and so gets the control a number gets.
///
/// A `Quat` and a `Vec3` read as numbers and are not: one is three degrees in a
/// box and the other has a row shape of its own.
fn is_number(value: &dyn PartialReflect) -> bool {
    macro_rules! any {
        ($($kind:ty),*) => { $(if value.try_downcast_ref::<$kind>().is_some() { return true; })* };
    }
    any!(f32, f64, i32, i64, u8, u16, u32, u64, usize);
    false
}

/// Whether the leaf holds a number with no fractional part to author.
///
/// The value form of [`whole_type`], for the walk that has the value in hand.
fn is_whole(value: &dyn PartialReflect) -> bool {
    macro_rules! any {
        ($($kind:ty),*) => { $(if value.try_downcast_ref::<$kind>().is_some() { return true; })* };
    }
    any!(i32, i64, u8, u16, u32, u64, usize);
    false
}

/// The float `value` holds, whichever width it was authored at, and through a
/// quantity that wraps one.
fn as_number(value: &dyn PartialReflect) -> Option<f64> {
    let value = through_quantity(value);
    value
        .try_downcast_ref::<f32>()
        .map(|number| f64::from(*number))
        .or_else(|| value.try_downcast_ref::<f64>().copied())
}

/// Whether a type is one of the two shapes a QUANTITY wraps, and so one the
/// panel already has a control for: a scalar in a box, a vector in three.
fn quantity_leaf(type_path: &str) -> bool {
    matches!(type_path, "f32" | "glam::Vec3")
}

/// The value inside a quantity newtype - the `f32` a [`Meters`] holds, the
/// `Vec3` a [`Meters3`] holds - or `None` when `value` is not one.
///
/// Read off the SHAPE rather than off a list of types, because this decides the
/// CONTROL: a tuple struct wrapping exactly one scalar or vector is edited as
/// that number whatever it is called, so a wrapper this module has never heard
/// of is still authorable. Which UNIT the number is in is a separate question,
/// asked of the name by [`quantity_unit`], and answered only for the types
/// `nova_events` actually owns.
fn quantity_inner(value: &dyn PartialReflect) -> Option<&dyn PartialReflect> {
    let ReflectRef::TupleStruct(fields) = value.reflect_ref() else {
        return None;
    };
    if fields.field_len() != 1 {
        return None;
    }
    let inner = fields.field(0)?;
    quantity_leaf(inner.get_represented_type_info()?.type_path()).then_some(inner)
}

/// The same, for writing.
fn quantity_inner_mut(value: &mut dyn PartialReflect) -> Option<&mut dyn PartialReflect> {
    // Asked of the shared borrow FIRST, which ends here: the mutable walk below
    // may not overlap the check that decides whether to take it.
    quantity_inner(&*value)?;
    let ReflectMut::TupleStruct(fields) = value.reflect_mut() else {
        return None;
    };
    fields.field_mut(0)
}

/// `value`, stepped through a quantity newtype where it is one.
fn through_quantity(value: &dyn PartialReflect) -> &dyn PartialReflect {
    quantity_inner(value).unwrap_or(value)
}

/// The type a quantity wraps, asked of the TYPE rather than of a value - which
/// is the only way to ask it of an `Option` field currently holding `None`.
fn quantity_field(info: &TypeInfo) -> Option<&'static TypeInfo> {
    let TypeInfo::TupleStruct(info) = info else {
        return None;
    };
    if info.field_len() != 1 {
        return None;
    }
    let inner = info.field_at(0)?.type_info()?;
    quantity_leaf(inner.type_path()).then_some(inner)
}

/// The unit a quantity is read in, off its own type name, or `None` for a
/// wrapper that is not one of `nova_events`' quantities.
///
/// The fallback for a field NOBODY has declared. It is read off the type
/// because the type is what now carries the dimension: `position` is a
/// displacement on a spawn action and a build-grid CELL on a ship's section,
/// and the two are told apart by being a [`Meters3`] and a bare `Vec3`. A
/// declaration in the table above still wins where there is one.
///
/// Enumerated rather than defaulted to meters: labelling an unknown wrapper
/// `m` states a dimension nobody declared, and a builder reading `m` off a
/// number that is not a length has been told something false.
fn quantity_unit(type_path: &str) -> Option<&'static str> {
    match type_path.rsplit("::").next().unwrap_or_default() {
        "Meters" | "Meters3" => Some("m"),
        "MetersPerSecond" => Some("m/s"),
        "MetersPerSecondSquared" => Some("m/s2"),
        _ => None,
    }
}

/// Refuse a number under its field's floor, in the words the box will show.
///
/// Checked HERE rather than at the spawn, where a negative radius is found out
/// at run time. The builder who typed it is the one who can fix it, and by then
/// they are flying the range.
///
/// The reason is the RULE, in three characters, because it is shown where the
/// unit stands: a sentence there would squeeze the box holding the number it
/// is about down to four characters.
fn check_floor(path: &[PathStep], value: &dyn PartialReflect) -> Result<(), String> {
    let Some(Limit::AtLeast(floor)) = field_spec(path).map(|spec| spec.limit) else {
        return Ok(());
    };
    let Some(number) = as_number(value) else {
        return Ok(());
    };
    if number >= f64::from(floor) {
        return Ok(());
    }
    Err(format!("min {}", number_text(f64::from(floor))))
}

/// Refuse a number that is not FINITE.
///
/// `nan` and `inf` parse as floats and every writer downstream takes them. A
/// position with a NaN in it is a node that has left the world: nothing draws
/// it, the gizmo cannot reach it, and no later edit brings it back, because
/// every arithmetic that would move it stays NaN.
///
/// Unlike a floor, this holds for EVERY float field and needs no rule - there
/// is no number a config authors for which a NaN is the intended value.
fn check_finite(value: &dyn PartialReflect) -> Result<(), String> {
    match as_number(value) {
        Some(number) if !number.is_finite() => Err("finite".to_string()),
        _ => Ok(()),
    }
}

/// A path as the levels a builder reads it in: one segment per named step,
/// prettied, with a list INDEX folded into the name it indexes.
///
/// `children[1].muzzle.fire_rate` is three segments - "Children 2", "Muzzle",
/// "Fire Rate" - because that is what it is: the second joint, its muzzle, and
/// the number on it. The index rides its own name rather than standing alone,
/// so no level of the tree is ever called "2".
fn segments(path: &[PathStep]) -> Vec<String> {
    let mut named: Vec<String> = Vec::new();
    for step in path {
        match step {
            PathStep::Field(name) => named.push(pretty(name)),
            // One-based: the second joint is "2" to everyone who is not a
            // programmer, and this label is read by a builder.
            PathStep::Item(index) => match named.last_mut() {
                Some(last) => last.push_str(&format!(" {}", index + 1)),
                None => named.push((index + 1).to_string()),
            },
            PathStep::Slot(_) => {}
        }
    }
    named
}

/// Where a row sits, and what it is called there.
///
/// The split is what keeps a deep config readable. A turret's fire rate is
/// eight steps down its joint tree, and one flat row called "Root Children 2
/// Children 2 Muzzle Fire Rate" is a path, not a label. The panel draws the
/// path once, as a tree, and the row under it is just "Fire Rate".
fn heading_and_label(path: &[PathStep]) -> (Vec<String>, String) {
    let mut named = segments(path);
    match named.pop() {
        Some(leaf) => (named, leaf),
        // The only path with no named step is the config's own root, which is
        // an enum: the row says WHICH KIND it is.
        None => (Vec::new(), "Kind".to_string()),
    }
}

/// `lock_signature` -> `Lock Signature`.
fn pretty(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether a value is an `Option`, which the walker steps through rather than
/// drawing as an enum.
fn is_option(value: &dyn PartialReflect) -> bool {
    value
        .get_represented_type_info()
        .is_some_and(|info| info.type_path().starts_with("core::option::Option<"))
}

/// The TYPE of an `Option`'s payload, or `None` when the value is not an
/// `Option` the type registry knows the shape of.
///
/// Needed because a field currently holding `None` cannot be asked what it
/// would hold: the type info is the only place that answer lives - and it is
/// the type info rather than the type PATH because a quantity is unwrapped
/// from it, which a string cannot be.
fn option_payload(value: &dyn PartialReflect) -> Option<&'static TypeInfo> {
    let TypeInfo::Enum(info) = value.get_represented_type_info()? else {
        return None;
    };
    let VariantInfo::Tuple(variant) = info.variant("Some")? else {
        return None;
    };
    variant.field_at(0)?.type_info()
}

/// Every leaf of `value`, flattened into rows under `path`.
fn walk(
    value: &dyn PartialReflect,
    root: FieldRoot,
    path: Vec<PathStep>,
    out: &mut Vec<InspectorRow>,
) {
    if let Some(flag) = value.try_downcast_ref::<bool>() {
        out.push(walked(root, path, false, RowValue::Flag(*flag)));
        return;
    }
    if let Some(colour) = any_colour(value) {
        out.push(walked(
            root,
            path,
            false,
            RowValue::Colour(colour_text(colour)),
        ));
        return;
    }
    // A VECTOR anywhere gets the pose's shape: one row, three boxes. Typed
    // into a single field it was `0, 0.064, -0.055` - three numbers a builder
    // has to count commas through to change one of them, and a line long
    // enough to wrap in a panel this narrow.
    //
    // A `Quat` is NOT given this. Its three boxes would have to be degrees,
    // and a per-axis write walks reflection into the value's own fields -
    // which on a quaternion are the raw x/y/z/w nobody authors. It stays one
    // field, parsed as the same three degrees the pose's rotation row takes.
    if let Some(vector) = value.try_downcast_ref::<Vec3>() {
        out.push(walked(root, path, false, axes_of(*vector)));
        return;
    }
    // A QUANTITY is the number inside it. Walked as the tuple struct it is, a
    // `Meters` would draw a row called "Blast Radius" holding nothing and a
    // row under it called "0" holding the number - which is the wrapper's
    // shape on screen, not the field's. The row stands at the FIELD's own
    // path, so the write-back, the drag and the floor check all still name
    // the field a builder is looking at.
    if let Some(inner) = quantity_inner(value) {
        let first = out.len();
        walk(inner, root, path, out);
        if let Some(info) = value.get_represented_type_info() {
            declare_by_type(info.type_path(), &mut out[first..]);
        }
        return;
    }
    if let Some(text) = leaf_text(value) {
        let mut row = if is_number(value) {
            walked_number(root, path, false, text, is_whole(value))
        } else {
            walked(root, path, false, RowValue::Text(text))
        };
        row.asset = value
            .get_represented_type_info()
            .and_then(|info| asset_sort(info.type_path()));
        out.push(row);
        return;
    }
    if is_option(value) {
        walk_option(value, root, path, out);
        return;
    }
    match value.reflect_ref() {
        ReflectRef::Struct(fields) => {
            // The field's own DECLARATION is where "this string is an object
            // id" is written, so the rows it produces carry it: the panel
            // offers the ids a reference could name and paints one that names
            // nothing, without a list of field names of its own.
            let info = match value.get_represented_type_info() {
                Some(TypeInfo::Struct(info)) => Some(info),
                _ => None,
            };
            for index in 0..fields.field_len() {
                let (Some(name), Some(field)) = (fields.name_at(index), fields.field_at(index))
                else {
                    continue;
                };
                let declared = info.and_then(|info| info.field_at(index));
                let names = declared
                    .and_then(NamedField::get_attribute::<Names>)
                    .copied();
                let hint = hint_of(declared.and_then(NamedField::docs));
                let first = out.len();
                walk(
                    field,
                    root,
                    step(&path, PathStep::Field(name.to_string())),
                    out,
                );
                for row in &mut out[first..] {
                    row.names = names;
                    // The INNERMOST doc wins: a struct's own field said what
                    // this one number is, where the field that holds the whole
                    // struct could only say what the struct is.
                    if row.hint.is_empty() {
                        row.hint.clone_from(&hint);
                    }
                }
            }
        }
        ReflectRef::TupleStruct(fields) => {
            for index in 0..fields.field_len() {
                let Some(field) = fields.field(index) else {
                    continue;
                };
                walk(field, root, step(&path, PathStep::Slot(index)), out);
            }
        }
        ReflectRef::List(items) => {
            // A list is walked ELEMENT BY ELEMENT rather than shown as debug
            // text. Without this a turret's fire rate - which lives on a muzzle
            // inside a joint inside `root.children` - had no row at all, and the
            // panel said the turret simply did not have one.
            for index in 0..items.len() {
                let Some(item) = items.get(index) else {
                    continue;
                };
                walk(item, root, step(&path, PathStep::Item(index)), out);
            }
        }
        ReflectRef::Enum(chosen) => {
            // The doc of the variant it IS, not of the field that holds it:
            // "the event to wait for" is the one thing a builder reading a row
            // that already says OnEnter does not need told.
            let variant = match value.get_represented_type_info() {
                Some(TypeInfo::Enum(info)) => info.variant(chosen.variant_name()),
                _ => None,
            };
            let variant = hint_of(variant.and_then(VariantInfo::docs));
            // A variant that carries FIELDS is a readout, not a choice:
            // switching to one would mean inventing every field of it that
            // nobody has authored. An enum whose variants are all bare names
            // has nothing to invent, so it is offered as a choice.
            let value = match unit_variants(value) {
                Some(offered) => {
                    let name = chosen.variant_name();
                    RowValue::Choice {
                        chosen: offered
                            .iter()
                            .position(|(option, _)| option == name)
                            .unwrap_or(0),
                        options: offered.iter().map(|(option, _)| option.clone()).collect(),
                        hints: offered.into_iter().map(|(_, hint)| hint).collect(),
                    }
                }
                None => RowValue::Fixed(chosen.variant_name().to_string()),
            };
            out.push(walked(root, path.clone(), false, value).saying(variant));
            for index in 0..chosen.field_len() {
                let Some(field) = chosen.field_at(index) else {
                    continue;
                };
                let inner = match chosen.name_at(index) {
                    Some(name) => step(&path, PathStep::Field(name.to_string())),
                    None => step(&path, PathStep::Slot(index)),
                };
                walk(field, root, inner, out);
            }
        }
        _ => out.push(walked(root, path, false, RowValue::Fixed(debug_of(value)))),
    }
}

/// An `Option` field.
///
/// A scalar one is ONE row whose empty string means `None` - the shortest
/// gesture for "this rock has no authored mass" that does not need a second
/// widget beside every number. An optional STRUCT cannot be typed into, so a
/// present one is walked through and an absent one says so.
fn walk_option(
    value: &dyn PartialReflect,
    root: FieldRoot,
    path: Vec<PathStep>,
    out: &mut Vec<InspectorRow>,
) {
    let payload = option_payload(value);
    // An optional QUANTITY is an optional number: `rake_radius` is one box a
    // builder types meters into or clears, not a struct to open.
    let leaf = payload.map(|info| quantity_field(info).unwrap_or(info));
    let leaf_path = leaf.map(TypeInfo::type_path);
    let scalar = leaf_path.is_some_and(leaf_type);
    let present = matches!(value.reflect_ref(), ReflectRef::Enum(chosen) if chosen.field_len() > 0);
    if scalar {
        let text = match value.reflect_ref() {
            ReflectRef::Enum(chosen) => chosen
                .field_at(0)
                .map(through_quantity)
                .and_then(leaf_text)
                .unwrap_or_default(),
            _ => String::new(),
        };
        // Off the PAYLOAD TYPE, not the value: a field holding `None` is still
        // a number's field, so it wears its unit, its step and its name - the
        // grip that scrubs it once it holds one.
        let mut row = if leaf_path.is_some_and(number_type) {
            let whole = leaf_path.is_some_and(whole_type);
            walked_number(root, path, true, text, whole)
        } else {
            walked(root, path, true, RowValue::Text(text))
        };
        row.asset = leaf_path.and_then(asset_sort);
        if let Some(quantity) = payload.filter(|info| quantity_field(info).is_some()) {
            declare_by_type(quantity.type_path(), core::slice::from_mut(&mut row));
        }
        out.push(row);
        return;
    }
    if !present {
        out.push(walked(
            root,
            path,
            true,
            RowValue::Fixed("none".to_string()),
        ));
        return;
    }
    let ReflectRef::Enum(chosen) = value.reflect_ref() else {
        return;
    };
    let Some(inner) = chosen.field_at(0) else {
        return;
    };
    walk(inner, root, step(&path, PathStep::Slot(0)), out);
}

/// Every variant name of `value`, but ONLY when the enum is a plain set of
/// names.
///
/// `None` the moment one variant carries a field, which is what keeps the
/// choice honest: a dropdown that could switch to `Prototype(String)` would
/// have to invent the string.
fn unit_variants(value: &dyn PartialReflect) -> Option<Vec<(String, String)>> {
    let TypeInfo::Enum(info) = value.get_represented_type_info()? else {
        return None;
    };
    info.iter()
        .map(|variant| match variant {
            VariantInfo::Unit(unit) => Some((unit.name().to_string(), hint_of(unit.docs()))),
            _ => None,
        })
        .collect()
}

/// `path` with one more step on the end.
fn step(path: &[PathStep], next: PathStep) -> Vec<PathStep> {
    let mut extended = path.to_vec();
    extended.push(next);
    extended
}

/// A value's debug text, for the rows the inspector can only show.
fn debug_of(value: &dyn PartialReflect) -> String {
    format!("{value:?}")
}

/// Whether the type is ONE number, asked of a type path rather than a value -
/// which is the only way to ask it of a field currently holding `None`.
fn number_type(type_path: &str) -> bool {
    matches!(
        type_path,
        "f32" | "f64" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "usize"
    )
}

/// Whether the type is a number with no fractional part to author, asked the
/// same way and for the same reason as [`number_type`].
fn whole_type(type_path: &str) -> bool {
    matches!(
        type_path,
        "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "usize"
    )
}

/// Whether the parser has a leaf for this type, which is what decides between
/// an editable row and a readout.
fn leaf_type(type_path: &str) -> bool {
    if authored_type(type_path) {
        return true;
    }
    matches!(
        type_path,
        "bool"
            | "f32"
            | "f64"
            | "i32"
            | "i64"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "usize"
            | "alloc::string::String"
            | "bevy_color::color::Color"
            | "bevy_color::linear_rgba::LinearRgba"
            | "bevy_color::srgba::Srgba"
            | "glam::Vec3"
            | "glam::Quat"
    )
}

/// The leaves nova declares rather than borrows: the variables DSL, which is
/// authored as the text a RON file carries, and an asset reference, which is
/// authored as the path under `assets/`.
///
/// Both are `#[reflect(opaque)]`, so the walker cannot take them apart and
/// would otherwise show them as debug text. They are not values a builder
/// cannot author - they are values with a SYNTAX, and the syntax is the leaf.
fn authored_type(type_path: &str) -> bool {
    type_path == VariableExpressionNode::type_path()
        || type_path == VariableConditionNode::type_path()
        || asset_type(type_path)
}

/// Whether the type is one of the asset references a scenario config holds.
///
/// Named one by one rather than by prefix: a parse has to BUILD the value, and
/// only a concrete `AssetRef<A>` can be built.
fn asset_type(type_path: &str) -> bool {
    type_path == AssetRef::<Image>::type_path()
        || type_path == AssetRef::<AudioSource>::type_path()
        || type_path == AssetRef::<WorldAsset>::type_path()
}

/// What SORT of file an asset reference names, or `None` for a type that names
/// no file.
///
/// The picker and the fault mark both hang off this. Taken from the TYPE, which
/// is also how an `Option` field answers it: a row holding `None` is still an
/// image's row, and can still offer the images to fill it with.
fn asset_sort(type_path: &str) -> Option<AssetSort> {
    if type_path == AssetRef::<Image>::type_path() {
        return Some(AssetSort::Image);
    }
    if type_path == AssetRef::<AudioSource>::type_path() {
        return Some(AssetSort::Audio);
    }
    if type_path == AssetRef::<WorldAsset>::type_path() {
        return Some(AssetSort::Model);
    }
    None
}

/// The path an asset reference holds, whichever asset it points at.
///
/// A reference already RESOLVED to a handle has no path to show: it came from
/// code rather than from a file, and a builder cannot retype it.
fn asset_text(value: &dyn PartialReflect) -> Option<String> {
    macro_rules! asset {
        ($($kind:ty),*) => {
            $(if let Some(reference) = value.try_downcast_ref::<AssetRef<$kind>>() {
                return Some(reference.path().unwrap_or_default().to_string());
            })*
        };
    }
    asset!(Image, AudioSource, WorldAsset);
    None
}

/// A leaf's canonical text, or `None` when it is not a leaf.
///
/// The formats are the ones a builder would type: a plain number without a
/// trailing `.0`, a colour as `#rrggbb`, a vector as three comma-separated
/// numbers.
fn leaf_text(value: &dyn PartialReflect) -> Option<String> {
    if let Some(number) = value.try_downcast_ref::<f32>() {
        return Some(number_text(f64::from(*number)));
    }
    if let Some(number) = value.try_downcast_ref::<f64>() {
        return Some(number_text(*number));
    }
    macro_rules! integer {
        ($($kind:ty),*) => {
            $(if let Some(number) = value.try_downcast_ref::<$kind>() {
                return Some(number.to_string());
            })*
        };
    }
    integer!(i32, i64, u8, u16, u32, u64, usize);
    if let Some(text) = value.try_downcast_ref::<String>() {
        return Some(text.clone());
    }
    if let Some(expression) = value.try_downcast_ref::<VariableExpressionNode>() {
        return Some(expression.to_string());
    }
    if let Some(condition) = value.try_downcast_ref::<VariableConditionNode>() {
        return Some(condition.to_string());
    }
    if let Some(path) = asset_text(value) {
        return Some(path);
    }
    if let Some(colour) = any_colour(value) {
        return Some(colour_text(colour));
    }
    if let Some(turn) = value.try_downcast_ref::<Quat>() {
        // Degrees, for the same reason the pose's heading row is degrees: a
        // walked `Quat` is four rows called X, Y, Z and W, and nobody authors a
        // rotation that way.
        return leaf_text(&rotation_degrees(&Transform::from_rotation(*turn)));
    }
    if let Some(vector) = value.try_downcast_ref::<Vec3>() {
        return Some(format!(
            "{}, {}, {}",
            number_text(f64::from(vector.x)),
            number_text(f64::from(vector.y)),
            number_text(f64::from(vector.z))
        ));
    }
    None
}

/// A number without the noise: `3` rather than `3.0000000`, and three decimals
/// at most, because the rail is narrow and nobody authors a thruster to the
/// millionth.
fn number_text(value: f64) -> String {
    let rounded = format!("{value:.3}");
    let trimmed = rounded.trim_end_matches('0').trim_end_matches('.');
    // `-0` is the sign of a value that rounded away, not a number anyone
    // authored. A pose row reading "3.039, -0, -0" says the two zeros are
    // somehow different from each other, and they are not.
    if trimmed.is_empty() || trimmed == "-" || trimmed == "-0" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// The colour `value` holds, whichever of the three colour types a config
/// happens to have used.
///
/// `LinearRgba` and `Srgba` are colours to the builder looking at them even
/// though they are plain structs to the walker, which would otherwise take them
/// apart into four rows called Red, Green, Blue and Alpha.
fn any_colour(value: &dyn PartialReflect) -> Option<Color> {
    if let Some(colour) = value.try_downcast_ref::<Color>() {
        return Some(*colour);
    }
    if let Some(colour) = value.try_downcast_ref::<LinearRgba>() {
        return Some(Color::from(*colour));
    }
    value
        .try_downcast_ref::<Srgba>()
        .map(|colour| Color::from(*colour))
}

/// `#rrggbb`, or `#rrggbbaa` when the colour is not opaque.
pub(crate) fn colour_text(colour: Color) -> String {
    let srgb = Srgba::from(colour);
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    let (red, green, blue) = (channel(srgb.red), channel(srgb.green), channel(srgb.blue));
    if srgb.alpha >= 1.0 {
        format!("#{red:02x}{green:02x}{blue:02x}")
    } else {
        format!("#{red:02x}{green:02x}{blue:02x}{:02x}", channel(srgb.alpha))
    }
}

/// Build the value `type_path` names out of `text`, or say why it cannot be.
///
/// Keyed on the reflected TYPE rather than on the field, so the same table
/// serves a field that holds the value and an `Option` that would.
fn parse_leaf(type_path: &str, text: &str) -> Result<Box<dyn PartialReflect>, String> {
    let text = text.trim();
    macro_rules! number {
        ($kind:ty) => {
            text.parse::<$kind>()
                .map(|value| Box::new(value) as Box<dyn PartialReflect>)
                .map_err(|_| format!("not a {}", stringify!($kind)))
        };
    }
    if type_path == VariableExpressionNode::type_path() {
        return text
            .parse::<VariableExpressionNode>()
            .map(|node| Box::new(node) as Box<dyn PartialReflect>)
            .map_err(|error| error.message().to_string());
    }
    if type_path == VariableConditionNode::type_path() {
        return text
            .parse::<VariableConditionNode>()
            .map(|node| Box::new(node) as Box<dyn PartialReflect>)
            .map_err(|error| error.message().to_string());
    }
    macro_rules! asset {
        ($($kind:ty),*) => {
            $(if type_path == AssetRef::<$kind>::type_path() {
                return Ok(Box::new(AssetRef::<$kind>::from(text)));
            })*
        };
    }
    asset!(Image, AudioSource, WorldAsset);
    match type_path {
        "bool" => text
            .parse::<bool>()
            .map(|value| Box::new(value) as Box<dyn PartialReflect>)
            .map_err(|_| "not true or false".to_string()),
        "f32" => number!(f32),
        "f64" => number!(f64),
        "i32" => number!(i32),
        "i64" => number!(i64),
        "u8" => number!(u8),
        "u16" => number!(u16),
        "u32" => number!(u32),
        "u64" => number!(u64),
        "usize" => number!(usize),
        "alloc::string::String" => Ok(Box::new(text.to_string())),
        "bevy_color::color::Color" => Srgba::hex(text)
            .map(|srgba| Box::new(Color::from(srgba)) as Box<dyn PartialReflect>)
            .map_err(|_| "not a #rrggbb colour".to_string()),
        "bevy_color::linear_rgba::LinearRgba" => Srgba::hex(text)
            .map(|srgba| Box::new(LinearRgba::from(srgba)) as Box<dyn PartialReflect>)
            .map_err(|_| "not a #rrggbb colour".to_string()),
        "bevy_color::srgba::Srgba" => Srgba::hex(text)
            .map(|srgba| Box::new(srgba) as Box<dyn PartialReflect>)
            .map_err(|_| "not a #rrggbb colour".to_string()),
        // Three DEGREES, the same three the pose's heading row reads and
        // writes - so a rotation nested in a config is authored the way a
        // node's own is.
        "glam::Quat" => {
            let degrees = parse_leaf("glam::Vec3", text)?;
            let degrees = degrees
                .try_downcast_ref::<Vec3>()
                .ok_or_else(|| "wants yaw, pitch, roll".to_string())?;
            Ok(Box::new(rotation_from_degrees(*degrees)))
        }
        "glam::Vec3" => {
            let parts: Vec<&str> = text.split(',').map(str::trim).collect();
            let [x, y, z] = parts.as_slice() else {
                return Err("wants x, y, z".to_string());
            };
            let read = |part: &str| part.parse::<f32>().map_err(|_| "not a number".to_string());
            Ok(Box::new(Vec3::new(read(x)?, read(y)?, read(z)?)))
        }
        other => Err(format!("cannot author a {other}")),
    }
}

/// Build the value `info` names out of `text`, through a quantity newtype
/// where it is one: a [`Meters`] field is authored as the number inside it.
///
/// The wrapper is rebuilt reflectively rather than named, so a dimension the
/// units module grows tomorrow needs no arm here. Only the `Option` write
/// needs this - a field that HOLDS a quantity is resolved through it by
/// [`resolve`], and the box then writes the bare number it found.
fn parse_value(info: &'static TypeInfo, text: &str) -> Result<Box<dyn PartialReflect>, String> {
    let Some(inner) = quantity_field(info) else {
        return parse_leaf(info.type_path(), text);
    };
    let mut wrapped = DynamicTupleStruct::default();
    wrapped.set_represented_type(Some(info));
    wrapped.insert_boxed(parse_leaf(inner.type_path(), text)?);
    Ok(Box::new(wrapped))
}

/// The value `path` names inside `root`, for writing.
///
/// A QUANTITY is stepped through the way an `Option` is walked through: the
/// path names the field, and what a box writes is the number inside it. Both
/// on the way down - `position.x` is the `x` of the `Vec3` a `Meters3` holds -
/// and at the end, where a `Meters` field hands back its own `f32`.
fn resolve<'a>(
    root: &'a mut dyn PartialReflect,
    path: &[PathStep],
) -> Option<&'a mut dyn PartialReflect> {
    let mut value = root;
    for next in path {
        if quantity_inner(&*value).is_some() {
            value = quantity_inner_mut(value)?;
        }
        value = match (value.reflect_mut(), next) {
            (ReflectMut::Struct(fields), PathStep::Field(name)) => fields.field_mut(name)?,
            (ReflectMut::Struct(fields), PathStep::Slot(index)) => fields.field_at_mut(*index)?,
            (ReflectMut::TupleStruct(fields), PathStep::Slot(index)) => fields.field_mut(*index)?,
            (ReflectMut::Tuple(fields), PathStep::Slot(index)) => fields.field_mut(*index)?,
            (ReflectMut::Enum(chosen), PathStep::Field(name)) => chosen.field_mut(name)?,
            (ReflectMut::Enum(chosen), PathStep::Slot(index)) => chosen.field_at_mut(*index)?,
            (ReflectMut::List(items), PathStep::Item(index)) => items.get_mut(*index)?,
            _ => return None,
        };
    }
    if quantity_inner(&*value).is_some() {
        value = quantity_inner_mut(value)?;
    }
    Some(value)
}

/// Write `text` into the value `path` names, or say why it cannot go there.
///
/// The whole edit is one `try_apply`, so a config either takes the new value or
/// keeps the old one - a half-written struct is not a state this can produce.
pub(crate) fn write_field(
    root: &mut dyn PartialReflect,
    path: &[PathStep],
    optional: bool,
    text: &str,
) -> Result<(), String> {
    let target = resolve(root, path).ok_or_else(|| "gone".to_string())?;
    if optional {
        let payload = option_payload(target).ok_or_else(|| "not optional".to_string())?;
        let wanted = if text.trim().is_empty() {
            DynamicEnum::new("None", DynamicVariant::Unit)
        } else {
            let value = parse_value(payload, text)?;
            check_finite(value.as_ref())?;
            check_floor(path, value.as_ref())?;
            let mut fields = DynamicTuple::default();
            fields.insert_boxed(value);
            DynamicEnum::new("Some", DynamicVariant::Tuple(fields))
        };
        return target
            .try_apply(&wanted)
            .map_err(|error| format!("refused: {error}"));
    }
    let type_path = target
        .get_represented_type_info()
        .ok_or_else(|| "no type".to_string())?
        .type_path()
        .to_string();
    let value = parse_leaf(&type_path, text)?;
    check_finite(value.as_ref())?;
    check_floor(path, value.as_ref())?;
    target
        .try_apply(value.as_ref())
        .map_err(|error| format!("refused: {error}"))
}

/// Switch the enum at `path` to its `variant`.
///
/// Unit variants only, which is the same rule that decides whether the row is
/// a choice at all: there is nothing to carry across, so the whole switch is
/// one `try_apply` of a bare name.
pub(crate) fn choose_field(
    root: &mut dyn PartialReflect,
    path: &[PathStep],
    variant: &str,
) -> Result<(), String> {
    let target = resolve(root, path).ok_or_else(|| "gone".to_string())?;
    // A VOCABULARY field is a String the editor knows the values of, and the
    // option text is the value: `ice` in the list is `"ice"` in the file. The
    // list is the only way to write one, so nothing here has to check that the
    // name is one the game ships - the picker never offers another.
    if target.try_downcast_ref::<String>().is_some() {
        return target
            .try_apply(&variant.to_string())
            .map_err(|error| format!("refused: {error}"));
    }
    let ReflectMut::Enum(_) = target.reflect_mut() else {
        return Err("not a choice".to_string());
    };
    let wanted = DynamicEnum::new(variant, DynamicVariant::Unit);
    target
        .try_apply(&wanted)
        .map_err(|error| format!("refused: {error}"))
}

/// The colour a `#rrggbb` or `#rrggbbaa` row is showing, for the swatch beside
/// it. Unparseable text paints nothing rather than guessing.
pub(crate) fn parse_colour(text: &str) -> Option<Color> {
    Srgba::hex(text.trim()).ok().map(Color::from)
}

/// The number a leaf holds, through an `Option` if it is one, and whether the
/// field is a WHOLE number - which decides both how far a scrub may land from
/// where it started and how the result is written back.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Held {
    value: f64,
    whole: bool,
    /// The floor the field's own TYPE carries, where it has one. An unsigned
    /// field has nowhere below zero to go, and saying so here is what stops a
    /// scrub walking off the end of it into a parse error.
    floor: Option<f64>,
}

/// What `value` holds, if it holds one number.
#[expect(
    clippy::cast_precision_loss,
    reason = "a config integer past 2^53 is not a number a builder typed"
)]
fn number_at(value: &dyn PartialReflect) -> Option<Held> {
    if is_option(value) {
        let ReflectRef::Enum(chosen) = value.reflect_ref() else {
            return None;
        };
        return chosen.field_at(0).and_then(number_at);
    }
    if let Some(number) = as_number(value) {
        return Some(Held {
            value: number,
            whole: false,
            floor: None,
        });
    }
    macro_rules! whole {
        ($($kind:ty),*) => {
            $(if let Some(number) = value.try_downcast_ref::<$kind>() {
                return Some(Held { value: *number as f64, whole: true, floor: None });
            })*
        };
    }
    macro_rules! unsigned {
        ($($kind:ty),*) => {
            $(if let Some(number) = value.try_downcast_ref::<$kind>() {
                return Some(Held { value: *number as f64, whole: true, floor: Some(0.0) });
            })*
        };
    }
    whole!(i32, i64);
    unsigned!(u8, u16, u32, u64, usize);
    None
}

/// `value` snapped to the precision `step` implies, so a scrub leaves a number
/// a builder can read: `0.35`, not `0.3500001`.
fn snapped(value: f64, step: f32, whole: bool) -> f64 {
    if whole {
        return value.round();
    }
    let step = f64::from(step);
    if step <= 0.0 {
        return value;
    }
    (value / step).round() * step
}

/// Move the number at `path` by `steps` of the row's own step.
///
/// The step is the ROW's, passed in, not looked up again here. A grip on a
/// vector is handed the path of one component, and `x` is not a name any
/// declaration matches - a second lookup finds nothing and falls back to a
/// different step from the one the drag is being scaled by. Travel and snap
/// then disagree, and a scrub either doubles or stops dead depending on where
/// the number happens to sit.
///
/// `steps` is a count, so a value already on the step grid stays on it and the
/// snap can never round a move away. Sub-pixel movement is the caller's to
/// accumulate: at scale factor 2 one physical pixel is half a logical one, and
/// a control that discards half-pixels does not move at all there.
///
/// The result goes back through [`write_field`], so a scrub answers to the same
/// floor a typed number does - except that it STOPS at it instead of being
/// refused: a drag that walks into a floor has not made a mistake, it has
/// arrived.
///
/// This is also why `nan` is not a case here. A scrubbed number is the old
/// number plus a delta, and neither can be `nan`, so the one value the typed
/// box has to refuse is a value this control cannot express.
pub(crate) fn nudge_field(
    root: &mut dyn PartialReflect,
    path: &[PathStep],
    optional: bool,
    rule: DragRule,
    steps: f64,
) -> Result<(), String> {
    let held = {
        let target = resolve(root, path).ok_or_else(|| GRIP_GONE.to_string())?;
        number_at(target).ok_or_else(|| GRIP_EMPTY.to_string())?
    };
    let floors = [
        held.floor,
        match rule.limit {
            Limit::AtLeast(floor) => Some(f64::from(floor)),
            Limit::Free => None,
        },
    ];
    let floor = floors.into_iter().flatten().reduce(f64::max);
    let mut moved = snapped(
        held.value + steps * f64::from(rule.step),
        rule.step,
        held.whole,
    );
    if let Some(floor) = floor {
        moved = moved.max(floor);
    }
    if moved == held.value {
        return Ok(());
    }
    write_field(root, path, optional, &number_text(moved))
}

/// What a scrub says when the row it was on is not there any more, which a
/// delete under a live drag can do.
pub(crate) const GRIP_GONE: &str = "that field is gone - pick the node again";

/// What a scrub says on an OPTIONAL field holding nothing.
///
/// A number's field wears its unit and its grip whether or not it holds a
/// number yet, because the alternative is a row that changes shape under the
/// pointer. There is still nothing to add a delta to, so the refusal is the way
/// out rather than the Rust word for the hole.
pub(crate) const GRIP_EMPTY: &str = "type a number here first";

/// What a grip needs to move a number: how far one pixel takes it, and where it
/// stops. Both are the ROW's, resolved once where the declaration can be found.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DragRule {
    /// How far one pixel of a drag moves the number.
    pub(crate) step: f32,
    /// What the field takes.
    pub(crate) limit: Limit,
}

/// Flip the `bool` at `path`, and say what it became.
///
/// Its own entry point rather than a `write_field` of `"true"`, because a
/// checkbox does not know what it is showing until it has read the value: the
/// widget draws the state, it does not hold it.
pub(crate) fn toggle_field(root: &mut dyn PartialReflect, path: &[PathStep]) -> Option<bool> {
    let flag = resolve(root, path)?.try_downcast_mut::<bool>()?;
    *flag = !*flag;
    Some(*flag)
}

/// The kind config an object node carries, for reading.
///
/// `Spaceship` is here too, and it is the only variant the editor cannot MINT:
/// a picket is seeded rather than built, but WHICH hull it flies and WHO flies
/// it are the two questions a reader has about it, and both are fields of the
/// config like any rock's radius.
pub(crate) fn object_config(kind: &ScenarioObjectKind) -> Option<&dyn PartialReflect> {
    match kind {
        ScenarioObjectKind::Anchor(config) => Some(config),
        ScenarioObjectKind::Asteroid(config) => Some(config),
        ScenarioObjectKind::Beacon(config) => Some(config),
        ScenarioObjectKind::SalvageCrate(config) => Some(config),
        ScenarioObjectKind::Light(config) => Some(config),
        ScenarioObjectKind::Planet(config) => Some(config),
        ScenarioObjectKind::Spaceship(config) => Some(config),
    }
}

/// The same config, for writing.
pub(crate) fn object_config_mut(kind: &mut ScenarioObjectKind) -> Option<&mut dyn PartialReflect> {
    match kind {
        ScenarioObjectKind::Anchor(config) => Some(config),
        ScenarioObjectKind::Asteroid(config) => Some(config),
        ScenarioObjectKind::Beacon(config) => Some(config),
        ScenarioObjectKind::SalvageCrate(config) => Some(config),
        ScenarioObjectKind::Light(config) => Some(config),
        ScenarioObjectKind::Planet(config) => Some(config),
        ScenarioObjectKind::Spaceship(config) => Some(config),
    }
}

/// The kind config a section carries, for reading.
pub(crate) fn section_config(kind: &SectionKind) -> &dyn PartialReflect {
    match kind {
        SectionKind::Hull(config) => config,
        SectionKind::Thruster(config) => config,
        SectionKind::Controller(config) => config,
        SectionKind::Turret(config) => config,
        SectionKind::Torpedo(config) => config,
        SectionKind::Railgun(config) => config,
    }
}

/// The same config, for writing.
pub(crate) fn section_config_mut(kind: &mut SectionKind) -> &mut dyn PartialReflect {
    match kind {
        SectionKind::Hull(config) => config,
        SectionKind::Thruster(config) => config,
        SectionKind::Controller(config) => config,
        SectionKind::Turret(config) => config,
        SectionKind::Torpedo(config) => config,
        SectionKind::Railgun(config) => config,
    }
}

/// Make `node` editable in place, and hand back the config to edit.
///
/// A section that names a catalog PROTOTYPE is copied inline first: an edit
/// applied to the id would be an edit to every ship that names it, including
/// ships in other documents. The copy is what "this ship's thruster is tuned
/// differently" has to mean.
pub(crate) fn editable_config<'a>(
    node: &'a mut SectionNode,
    catalog: Option<&GameSections>,
) -> Option<&'a mut SectionConfig> {
    if let SectionSource::Prototype(id) = &node.source {
        let config = catalog?.get_section(id)?.clone();
        node.source = SectionSource::Inline(config);
    }
    match &mut node.source {
        SectionSource::Inline(config) => Some(config),
        SectionSource::Prototype(_) => None,
    }
}

/// The fields `kind` shows before it is asked for the rest.
fn section_picks(kind: &SectionKind) -> &'static [FieldSpec] {
    match kind {
        SectionKind::Hull(_) => HULL_PICKS,
        SectionKind::Thruster(_) => THRUSTER_PICKS,
        SectionKind::Controller(_) => CONTROLLER_PICKS,
        SectionKind::Turret(_) => TURRET_PICKS,
        SectionKind::Torpedo(_) => TORPEDO_PICKS,
        SectionKind::Railgun(_) => RAILGUN_PICKS,
    }
}

/// The same, for the things a scenario holds beside its ships.
fn object_picks(kind: &ScenarioObjectKind) -> &'static [FieldSpec] {
    match kind {
        ScenarioObjectKind::Anchor(_) => ANCHOR_PICKS,
        ScenarioObjectKind::Asteroid(_) => ASTEROID_PICKS,
        ScenarioObjectKind::Spaceship(_) => SPACESHIP_PICKS,
        ScenarioObjectKind::Beacon(_) => BEACON_PICKS,
        ScenarioObjectKind::SalvageCrate(_) => SALVAGE_PICKS,
        ScenarioObjectKind::Light(_) => LIGHT_PICKS,
        ScenarioObjectKind::Planet(_) => PLANET_PICKS,
    }
}

/// `rows` cut down to the ones `picks` names.
///
/// A pick is matched by field name at ANY depth, and takes the field with
/// everything under it. A row with an EMPTY path is the node's own - its name,
/// its pose, the part it was built from, the key it fires on - and is never a
/// config field, so it is always kept.
fn curate(rows: Vec<InspectorRow>, picks: &[FieldSpec]) -> Vec<InspectorRow> {
    let picked = |name: &str| picks.iter().any(|spec| spec.covers(name));
    rows.into_iter()
        .filter(|row| row.path.is_empty() || row.path.iter().any(|step| named(step, &picked)))
        .map(|mut row| {
            // The headings of the levels that were NOT picked go with them. A
            // fire rate under `Root > Children 1 > Children 1 > Muzzle` is five
            // lines of tree over one number, and the tree is exactly what this
            // view was written to put away. A picked level keeps its heading:
            // a spaceship's Hull is a thing a builder chose to see.
            //
            // Read POSITIONALLY rather than by comparing the prettied text.
            // `group` is `segments(path)` less its leaf and `segments` pushes
            // one entry per FIELD step, so the two run in step - and `retain`
            // visits its elements in order, once each. Building the list of
            // kept names instead meant a `Vec<String>` and a `pretty()` per
            // retained row, on a panel with no change gate under it.
            let mut fields = row.path.iter().filter_map(|step| match step {
                PathStep::Field(name) => Some(name.as_str()),
                PathStep::Item(_) | PathStep::Slot(_) => None,
            });
            row.group.retain(|_| fields.next().is_some_and(&picked));
            row
        })
        .collect()
}

/// Whether a step names a field the test accepts. A list index and a tuple slot
/// name nothing, so they never do.
fn named(step: &PathStep, test: &impl Fn(&str) -> bool) -> bool {
    match step {
        PathStep::Field(name) => test(name),
        PathStep::Item(_) | PathStep::Slot(_) => false,
    }
}

/// A section's rows, cut to what the kind is worth showing.
pub(crate) fn curated_section_rows(
    node: &SectionNode,
    catalog: Option<&GameSections>,
) -> Vec<InspectorRow> {
    let rows = section_rows(node, catalog);
    let Some(config) = node.resolve(catalog) else {
        return rows;
    };
    let picks = section_picks(&config.kind);
    curate(rows, picks)
}

/// An object's rows, cut to what the kind is worth showing.
pub(crate) fn curated_object_rows(object: &ObjectNode, pose: &Transform) -> Vec<InspectorRow> {
    curate(object_rows(object, pose), object_picks(&object.kind))
}

/// What the Allegiance row says when the ship states no side and takes the
/// one its driver implies.
pub(crate) const IMPLIED_ALLEGIANCE: &str = "default";

/// The rows a ship shows: who flies it, which side it is on, and where it
/// sits.
///
/// Allegiance is READ here, not set: it is what makes a picket dormant and a
/// hulk inert, so a builder has to be able to see it - but the only way to
/// choose one today is to choose a driver, and a control that offered more
/// than that would be offering to break the wake script.
pub(crate) fn ship_rows(ship: &ShipNode, pose: &Transform) -> Vec<InspectorRow> {
    let mut rows = vec![
        name_row(ship.name.clone()),
        InspectorRow {
            root: FieldRoot::Config,
            path: Vec::new(),
            optional: false,
            group: Vec::new(),
            label: "Driver".to_string(),
            unit: "",
            nudge: 0.0,
            limit: Limit::Free,
            value: RowValue::Driver(ship.driver),
            hint: "Who flies this ship: you, a bot, or nobody.".to_string(),
            names: None,
            asset: None,
            owner: None,
            depth: 0,
        },
        fixed(
            FieldRoot::Config,
            "Allegiance",
            ship.allegiance.map_or_else(
                || IMPLIED_ALLEGIANCE.to_string(),
                |side| format!("{side:?}"),
            ),
        )
        .saying("Which side this ship is on. It follows the driver unless a script overwrites it."),
    ];
    rows.extend(pose_rows(pose));
    rows
}

/// What the Player Ship row says when no ship of the document is flown.
pub(crate) const NO_PLAYER_SHIP: &str = "none";

/// The rows the scenario node shows: what the document HOLDS.
///
/// The root has no config of its own, so the panel used to go blank there -
/// which reads as the panel breaking every time you leave a ship. These are
/// the document's own facts, and every one of them is a thing a builder came
/// to the root to check.
pub(crate) fn scenario_rows(
    settings: Option<&ScenarioNode>,
    ships: usize,
    objects: usize,
    flown: Option<String>,
) -> Vec<InspectorRow> {
    let mut rows = vec![
        fixed(FieldRoot::Config, "Ships", ships.to_string())
            .saying("How many ships the document holds."),
        fixed(FieldRoot::Config, "Objects", objects.to_string())
            .saying("How many rocks, beacons, crates and areas stand on the board."),
        fixed(
            FieldRoot::Config,
            "Player Ship",
            flown.unwrap_or_else(|| NO_PLAYER_SHIP.to_string()),
        )
        .saying("The ship you fly when the scenario runs."),
    ];
    // The counts first because they are what the root has always answered, then
    // what the builder AUTHORS about the range as a whole. The walk is the same
    // one every other node gets, so the cubemap wears the file picker its type
    // earns it without this naming a single field.
    if let Some(settings) = settings {
        walk(
            settings.as_partial_reflect(),
            FieldRoot::Config,
            Vec::new(),
            &mut rows,
        );
    }
    rows
}

/// The rows a section shows: what it was built from, what it is bound to, and
/// its kind config.
pub(crate) fn section_rows(
    node: &SectionNode,
    catalog: Option<&GameSections>,
) -> Vec<InspectorRow> {
    let mut rows = vec![fixed(FieldRoot::Config, "Part", node.prototype())
        .saying("The catalog part this section was built from.")];
    if node.bindable(catalog) {
        let binding = source_label(&node.binds);
        rows.push(InspectorRow {
            root: FieldRoot::Config,
            path: Vec::new(),
            optional: false,
            group: Vec::new(),
            label: "Key".to_string(),
            unit: "",
            nudge: 0.0,
            limit: Limit::Free,
            value: RowValue::Key(if binding.is_empty() {
                UNBOUND.to_string()
            } else {
                binding
            }),
            hint: "The key that fires this section. Click it, then press one.".to_string(),
            names: None,
            asset: None,
            owner: None,
            depth: 0,
        });
    }
    let Some(config) = node.resolve(catalog) else {
        return rows;
    };
    walk(
        section_config(&config.kind),
        FieldRoot::Config,
        Vec::new(),
        &mut rows,
    );
    rows
}

/// The rows an object shows: its name, where it sits, and its kind config.
pub(crate) fn object_rows(object: &ObjectNode, pose: &Transform) -> Vec<InspectorRow> {
    let mut rows = vec![name_row(object.name.clone())];
    if let Some(config) = object_config(&object.kind) {
        walk(config, FieldRoot::Config, Vec::new(), &mut rows);
        offer_object_vocabularies(&object.kind, &mut rows);
    }
    rows.extend(pose_rows(pose));
    rows
}

/// Turn the text rows whose values the editor KNOWS into pick lists.
///
/// Keyed on the object's kind rather than declared in [`DECLARED`], because a
/// vocabulary belongs to the OBJECT and that table matches by field name at any
/// depth: a ship section has a `material` too, and it names a paint rather than
/// a rock.
///
/// A picker rather than a text box because the ids are the answer to "what is
/// this made of" and nobody guesses `carbon` from an empty box. It is also the
/// only control that cannot author a kind the game does not ship.
fn offer_object_vocabularies(kind: &ScenarioObjectKind, rows: &mut [InspectorRow]) {
    let ScenarioObjectKind::Asteroid(_) = kind else {
        return;
    };
    for row in rows {
        if leaf_name(&row.path) != Some(MATERIAL.name) {
            continue;
        }
        let RowValue::Text(held) = &row.value else {
            continue;
        };
        // An id the game does not ship stays a TEXT row, showing exactly what
        // the file says. Snapping it to the first option would hide a document
        // the lint refuses behind a control that looks like it works.
        let Some(chosen) = ASTEROID_KIND_SUMMARIES
            .iter()
            .position(|(id, _)| id == held)
        else {
            continue;
        };
        row.value = RowValue::Choice {
            options: ASTEROID_KIND_SUMMARIES
                .iter()
                .map(|(id, _)| (*id).to_string())
                .collect(),
            hints: ASTEROID_KIND_SUMMARIES
                .iter()
                .map(|(_, summary)| (*summary).to_string())
                .collect(),
            chosen,
        };
    }
}

/// The rows a handler shows: what it listens for, and whether it retires.
///
/// Walked off [`EventNode`] itself rather than listed here: the node IS the
/// authored handler minus its children, so the trigger becomes a choice and
/// `once` a checkbox without this module naming either.
pub(crate) fn event_rows(event: &EventNode) -> Vec<InspectorRow> {
    let mut rows = Vec::new();
    walk(event, FieldRoot::Config, Vec::new(), &mut rows);
    rows
}

/// The rows a filter shows: which filter it is, then its own config.
///
/// A combinator has no config - what it combines are its child rows in the
/// tree - so it shows the kind and nothing else, which is the whole truth
/// about it.
pub(crate) fn filter_rows(filter: &FilterNode) -> Vec<InspectorRow> {
    let choice = filter_choice(&filter.kind);
    let mut rows = vec![kind_row(
        "Filter",
        FilterChoice::ALL.into_iter().map(FilterChoice::label),
        FilterChoice::ALL.iter().map(variant_hint),
        FilterChoice::ALL.iter().position(|kind| *kind == choice),
    )
    .saying(variant_hint(&choice))];
    if let Some(config) = filter_config(&filter.kind) {
        let first = rows.len();
        walk(config, FieldRoot::Config, Vec::new(), &mut rows);
        // The expression filter IS its condition: a newtype whose one slot has
        // no name for the walker to read, so it would be called "Kind".
        if choice == FilterChoice::Expression {
            if let Some(row) = rows.get_mut(first) {
                row.label = "Condition".to_string();
            }
        }
    }
    rows
}

/// The rows an action shows: which action it is, then its own config.
///
/// A sequence shows its KEY and not its steps: the steps are rows of the tree,
/// and a panel listing them would be a second place they could be edited.
pub(crate) fn action_rows(action: &ActionNode) -> Vec<InspectorRow> {
    let choice = action_choice(&action.kind);
    let mut rows = vec![kind_row(
        "Action",
        ActionChoice::ALL.into_iter().map(ActionChoice::label),
        ActionChoice::ALL.iter().map(variant_hint),
        ActionChoice::ALL.iter().position(|kind| *kind == choice),
    )
    .saying(variant_hint(&choice))];
    if let Some(config) = action_config(&action.kind) {
        walk(config, FieldRoot::Config, Vec::new(), &mut rows);
    }
    rows
}

/// What an expression node IS to the thing above it.
///
/// Two facts in one word: WHICH page the node belongs to, and whether it is
/// that page's root. Between them they decide everything a row cannot read off
/// the node - the heading it stands under, the operators it may be switched
/// to, and the line explaining the place rather than the operator.
///
/// Only a root COMPARES, because the grammar compares at the top of a
/// condition and nowhere inside one - and a value expression, which is what an
/// action writes, never compares at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operand {
    /// The comparison a filter tests.
    Test,
    /// One side of an operator inside a condition.
    TestSide,
    /// The value an action writes.
    Value,
    /// One side of an operator inside a value.
    ValueSide,
}

impl Operand {
    /// The heading the page stands under, which every row of it shares.
    pub(crate) fn heading(self) -> &'static str {
        match self {
            Operand::Test | Operand::TestSide => "Condition",
            Operand::Value | Operand::ValueSide => "Value",
        }
    }

    /// Whether the place takes a comparison rather than arithmetic.
    fn compares(self) -> bool {
        self == Operand::Test
    }

    /// What an operand of this one is: the same page, one level in.
    pub(crate) fn inside(self) -> Self {
        match self {
            Operand::Test | Operand::TestSide => Operand::TestSide,
            Operand::Value | Operand::ValueSide => Operand::ValueSide,
        }
    }

    /// The line under the row: what BELONGS in this place.
    fn hint(self) -> &'static str {
        match self {
            Operand::Test => "The test this filter makes: it passes when the comparison holds.",
            Operand::TestSide => {
                "One side of the comparison: a variable, a number, or another operator."
            }
            Operand::Value => {
                "The value written into the variable: a number, a variable, or a sum of them."
            }
            Operand::ValueSide => {
                "One side of the operator above: a variable, a number, or another operator."
            }
        }
    }
}

/// Where a LEAF's text is written back: the one field a value operand has.
///
/// Named here beside the row that offers it, so the page and the box beside
/// its operators cannot disagree about which field the text is.
pub(crate) fn operand_path() -> Vec<PathStep> {
    vec![PathStep::Field("value".to_string())]
}

/// ONE row of an expression page: which operator this node is, and - for a
/// leaf - the expression it holds.
///
/// `place` is what the node IS to its parent, which is the only name it has:
/// the comparison a filter tests, the value an action writes, or the left and
/// right of the operator above it. `role` narrows the operators offered to the
/// ones that BELONG there - offering `+` where a `<` has to stand would let a
/// builder author a condition that is not one, and offering `==` where a value
/// belongs would let them write a comparison into a variable the grammar has
/// no way to spell.
pub(crate) fn operand_row(
    owner: Entity,
    expression: &ExpressionNode,
    place: &str,
    role: Operand,
    depth: usize,
) -> InspectorRow {
    let choice = expr_choice(&expression.kind);
    let offered: Vec<ExprChoice> = ExprChoice::ALL
        .into_iter()
        .filter(|kind| kind.compares() == role.compares())
        .collect();
    InspectorRow {
        root: FieldRoot::Kind,
        path: Vec::new(),
        optional: false,
        group: vec![role.heading().to_string()],
        label: place.to_string(),
        unit: "",
        nudge: 0.0,
        limit: Limit::Free,
        value: RowValue::Operand {
            options: offered
                .iter()
                .copied()
                .map(ExprChoice::label)
                .map(str::to_string)
                .collect(),
            chosen: offered
                .iter()
                .position(|kind| *kind == choice)
                .unwrap_or_default(),
            text: match &expression.kind {
                ExprKind::Value(operand) => Some(operand.value.to_string()),
                _ => None,
            },
        },
        hint: role.hint().to_string(),
        names: None,
        asset: None,
        owner: Some(owner),
        depth,
    }
}

/// The rows one beat of a sequence shows: how long after the beat before it,
/// and how long it may wait before the run is declared stuck.
pub(crate) fn step_rows(step: &StepNode) -> Vec<InspectorRow> {
    let mut rows = Vec::new();
    walk(step, FieldRoot::Config, Vec::new(), &mut rows);
    rows
}

/// The rows a gate shows: the event the beat waits for. Its filters are rows
/// of the tree, like a handler's.
pub(crate) fn gate_rows(gate: &GateNode) -> Vec<InspectorRow> {
    let mut rows = Vec::new();
    walk(gate, FieldRoot::Config, Vec::new(), &mut rows);
    rows
}

/// What the document has to offer a row that names something, and what it
/// takes for that row to RESOLVE.
///
/// One answer for the panel and the picker: the chip lists these and the fault
/// paint asks the same set, so a field can never be offered an id that the row
/// beside it would then call unknown.
#[derive(Debug, Default)]
pub(crate) struct DocumentNames {
    /// Every id the document puts on the board: the world's nodes, the fleet,
    /// and the ids the script itself spawns.
    objects: Vec<String>,
    /// The prefixes a scatter stands for. A reference starting with one
    /// resolves against the field it names.
    prefixes: Vec<String>,
    variables: Vec<String>,
    timers: Vec<String>,
    objectives: Vec<String>,
    scenarios: Vec<String>,
}

impl DocumentNames {
    /// What a field of this kind could name, in id order and without repeats.
    pub(crate) fn offers(&self, names: Names) -> Vec<String> {
        let mut offered = match names {
            // A DECLARATION is not offered the ids that already exist: an id
            // is unique, and picking a taken one would be picking a collision.
            Names::NewObject => Vec::new(),
            Names::Object => self.objects.clone(),
            Names::Variable => self.variables.clone(),
            Names::Timer => self.timers.clone(),
            Names::Objective => self.objectives.clone(),
            Names::Scenario => self.scenarios.clone(),
            // Nothing to offer: an order key is minted where the order is
            // installed, and a section id lives inside the hull the config
            // names beside it, which this document-wide list cannot see.
            Names::Order | Names::Section => Vec::new(),
        };
        offered.sort_unstable();
        offered.dedup();
        offered
    }

    /// Whether `text` names something this document holds.
    ///
    /// Only an OBJECT reference can be wrong here, and that is the same rule
    /// the lowering drops a handler by (see
    /// [`following_the_objects`](crate::scenario)): a variable or a timer is
    /// made by the handler that first writes it, so any key is a key.
    pub(crate) fn resolves(&self, names: Names, text: &str) -> bool {
        if names != Names::Object || text.is_empty() {
            return true;
        }
        self.objects.iter().any(|id| id == text)
            || self
                .prefixes
                .iter()
                .any(|prefix| !prefix.is_empty() && text.starts_with(prefix))
    }
}

/// The document, read for the names it holds.
///
/// The world half and the script half both: a handler names the rock the tree
/// put on the board as readily as one another handler spawns.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct DocumentIds<'w, 's> {
    context: Res<'w, EditContext>,
    objects: ObjectNodes<'w, 's>,
    ships: ShipNodes<'w, 's>,
    script: ScriptNodes<'w, 's>,
}

impl DocumentIds<'_, '_> {
    /// Everything this document names.
    pub(crate) fn names(&self) -> DocumentNames {
        let Some(scenario) = self.context.scenario() else {
            return DocumentNames::default();
        };
        let ids = self.script.names(scenario);
        let mut names = DocumentNames {
            objects: ids.declared,
            prefixes: ids.prefixes,
            variables: ids.variables,
            timers: ids.timers,
            objectives: ids.objectives,
            scenarios: ids.scenarios,
        };
        names.objects.extend(
            objects_of(scenario, &self.objects)
                .into_iter()
                .map(|(_, id, ..)| id.0.clone()),
        );
        // A ship lowers under its NODE id, except the flown one, which lowers
        // under the id the range gives the player's hull however it was named.
        names
            .objects
            .extend(self.ships.iter().map(|(_, _, id, ship)| {
                if ship.driver == ShipDriver::Player {
                    PLAYER_ID.to_string()
                } else {
                    id.0.clone()
                }
            }));
        names
    }
}

/// What a script node is CALLED on the panel's title line: the word its tree
/// row wears, so the panel and the row that opened it read alike.
pub(crate) fn script_name(target: InspectTarget, kinds: &ScriptNames) -> String {
    match target {
        InspectTarget::Event(node) => kinds
            .events
            .get(node)
            .map_or_else(|_| String::new(), handler_text),
        InspectTarget::Filter(node) => kinds
            .filters
            .get(node)
            .map_or("", |filter| filter_choice(&filter.kind).label())
            .to_string(),
        InspectTarget::Action(node) => kinds
            .actions
            .get(node)
            .map_or("", |action| action_choice(&action.kind).label())
            .to_string(),
        InspectTarget::Step(_) => "Step".to_string(),
        InspectTarget::Gate(node) => kinds.gates.get(node).map_or_else(
            |_| String::new(),
            |gate| event_label(gate.trigger).to_string(),
        ),
        _ => String::new(),
    }
}

/// The script components a title reads. Its own param so the panel's
/// [`Document`](crate::ui::inspector::Document) does not carry five more
/// queries that only the title line uses.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ScriptNames<'w, 's> {
    events: Query<'w, 's, &'static EventNode>,
    filters: Query<'w, 's, &'static FilterNode>,
    actions: Query<'w, 's, &'static ActionNode>,
    gates: Query<'w, 's, &'static GateNode>,
}

/// What the Key row reads when the section fires on nothing.
pub(crate) const UNBOUND: &str = "unbound";

/// The row a node is NAMED in. The one row a ship and an object share, and the
/// one field of either that is not part of its kind config.
fn name_row(name: String) -> InspectorRow {
    InspectorRow {
        root: FieldRoot::Label,
        path: Vec::new(),
        optional: false,
        group: Vec::new(),
        label: "Name".to_string(),
        unit: "",
        nudge: 0.0,
        limit: Limit::Free,
        value: RowValue::Text(name),
        hint: "What this node is called on the board and in the tree.".to_string(),
        names: None,
        asset: None,
        owner: None,
        depth: 0,
    }
}

/// The heading the two pose rows stand under.
pub(crate) const TRANSFORM: &str = "Transform";

/// Where a node stands and which way it faces.
///
/// One box per axis rather than `x, y, z` in one field: a real position
/// wrapped to two lines in the panel's value column, on the two rows a builder
/// reads most. They stand together under one heading, and LAST, so the group
/// runs to the end of the panel - a row with no group of its own drawn after a
/// heading would read as belonging to it.
///
/// There is no SCALE row and there will not be one: Nova sections mate, they
/// do not stretch, and a scaled hull would put every socket somewhere the
/// solver did not leave it.
///
/// Sections are deliberately not given this: a section's pose is SOLVED by the
/// mating snap, and a typed one would put a part where no socket is.
fn pose_rows(pose: &Transform) -> Vec<InspectorRow> {
    vec![
        // The panel's one ENGINE seam. A node's pose is a Bevy `Transform`,
        // which counts world units because the stage draws it, and every
        // number a builder types is meters. The crossing is here and in the
        // pose arm of `EditTargets::edit`, and nowhere between.
        axes_row(
            FieldRoot::Pose,
            "Position",
            "m",
            POSE_STEP,
            Limit::Free,
            Meters3::from_engine(pose.translation).get(),
        )
        .saying("Where this node stands, in meters."),
        // ROTATION, not heading: it is the node's rotation, and rotation is
        // what every other editor calls that. Both facts it used to keep in a
        // doc comment - degrees, and which turn is which - are on screen.
        axes_row(
            FieldRoot::Rotation,
            "Rotation",
            "deg, yaw/pitch/roll",
            TURN_STEP,
            Limit::Free,
            rotation_degrees(pose),
        )
        .saying("Which way it faces, in degrees."),
    ]
}

/// How far one pixel of a drag slides a node, in METERS: fine enough to seat a
/// beacon by eye, coarse enough to cross the stage in one pull.
const POSE_STEP: f32 = 0.5;
/// The same for a turn. A degree per pixel: a full turn is one drag across the
/// panel.
const TURN_STEP: f32 = 1.0;

/// A vector as three typed numbers, each in the form every other number wears.
fn axes_of(value: Vec3) -> RowValue {
    RowValue::Axes([value.x, value.y, value.z].map(|part| number_text(f64::from(part))))
}

/// One vector row: three numbers, each in the form every other number wears.
fn axes_row(
    root: FieldRoot,
    label: &str,
    unit: &'static str,
    nudge: f32,
    limit: Limit,
    value: Vec3,
) -> InspectorRow {
    InspectorRow {
        root,
        path: Vec::new(),
        optional: false,
        group: vec![TRANSFORM.to_string()],
        label: label.to_string(),
        unit,
        nudge,
        limit,
        value: axes_of(value),
        hint: String::new(),
        names: None,
        asset: None,
        owner: None,
        depth: 0,
    }
}

/// Which field of a `Vec3` the box for `axis` writes.
///
/// A step rather than three rows: the panel draws one row per vector and hands
/// each box the path of its own component, so the write-back is the same
/// reflection walk every other field takes.
pub(crate) fn axis_step(axis: usize) -> PathStep {
    PathStep::Field(["x", "y", "z"][axis.min(2)].to_string())
}

/// A pose's rotation as DEGREES of yaw, pitch and roll - the three numbers a
/// builder means by "turn it a quarter to port".
///
/// `YXZ` because yaw is the turn that matters on a stage laid out on the
/// ground plane, and taking it first keeps the other two small and readable
/// for a ship that is mostly level.
pub(crate) fn rotation_degrees(pose: &Transform) -> Vec3 {
    let (yaw, pitch, roll) = pose.rotation.to_euler(EulerRot::YXZ);
    Vec3::new(yaw.to_degrees(), pitch.to_degrees(), roll.to_degrees())
}

/// The rotation `degrees` of yaw, pitch and roll name. The inverse of
/// [`rotation_degrees`].
pub(crate) fn rotation_from_degrees(degrees: Vec3) -> Quat {
    Quat::from_euler(
        EulerRot::YXZ,
        degrees.x.to_radians(),
        degrees.y.to_radians(),
        degrees.z.to_radians(),
    )
}

#[cfg(test)]
mod tests;
