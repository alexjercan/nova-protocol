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
        ReflectMut, ReflectRef, TypeInfo,
    },
};
use nova_input::prelude::binding_label;
use nova_scenario::prelude::{ScenarioObjectKind, SectionSource};
use nova_ship::prelude::{GameSections, SectionConfig, SectionKind};

use crate::{
    config::SelectedNode,
    node::{EditContext, EditorNode, ObjectNode, ScenarioNode, SectionNode, ShipDriver, ShipNode},
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
}

impl InspectTarget {
    /// The node itself.
    pub(crate) fn node(self) -> Entity {
        match self {
            InspectTarget::Scenario(node)
            | InspectTarget::Ship(node)
            | InspectTarget::Section(node)
            | InspectTarget::Object(node) => node,
        }
    }

    /// The word the panel's tag wears.
    pub(crate) fn tag(self) -> &'static str {
        match self {
            InspectTarget::Scenario(_) => "SCENARIO",
            InspectTarget::Ship(_) => "SHIP",
            InspectTarget::Section(_) => "SECTION",
            InspectTarget::Object(_) => "OBJECT",
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
    let (scenario, ship, section, object) = kinds.get(node).ok()?;
    if scenario {
        return Some(InspectTarget::Scenario(node));
    }
    if ship {
        return Some(InspectTarget::Ship(node));
    }
    if section {
        return Some(InspectTarget::Section(node));
    }
    object.then_some(InspectTarget::Object(node))
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
            Self::Choice { options, chosen } => options.get(*chosen).cloned().unwrap_or_default(),
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
    }
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
/// Lengths are `u` - the authored world unit. The HUD converts to metres for
/// the player; content does not, and this box is content.
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
const MUZZLE_SPEED: FieldSpec = floored("muzzle_speed", "u/s", 0.5);
const BULLET_DAMAGE: FieldSpec = floored("bullet_damage", "hp", 0.5);
const BULLET_KIND: FieldSpec = plain("bullet_kind");
const PROJECTILE_LIFETIME: FieldSpec = floored("projectile_lifetime", "s", 0.05);
const SPAWNER_SPEED: FieldSpec = floored("spawner_speed", "u/s", 0.5);
const BLAST_DAMAGE: FieldSpec = floored("blast_damage", "hp", 0.5);
const BLAST_RADIUS: FieldSpec = floored("blast_radius", "u", 0.05);
const ARM_TIME: FieldSpec = floored("arm_time", "s", 0.02);
const ARM_DISTANCE: FieldSpec = floored("arm_distance", "u", 0.1);
const NAV_CONSTANT: FieldSpec = floored("nav_constant", "", 0.02);
const BODY_RADIUS: FieldSpec = floored("body_radius", "u", 0.05);
const MASS: FieldSpec = floored("mass", "", 0.5);
const RADIUS: FieldSpec = floored("radius", "u", 0.05);
const AREA_RADIUS: FieldSpec = floored("area_radius", "u", 0.1);
const INVULNERABLE: FieldSpec = plain("invulnerable");
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
const SIZE: FieldSpec = floored("size", "u", 0.05);
const ILLUMINANCE: FieldSpec = floored("illuminance", "lx", 50.0);
const INTENSITY: FieldSpec = floored("intensity", "lm", 50.0);
const RANGE: FieldSpec = floored("range", "u", 0.1);
const SHADOWS: FieldSpec = plain("shadows");
const HEALTH: FieldSpec = floored("health", "hp", 1.0);
const WIDTH: FieldSpec = floored("width", "u", 0.05);
const DELAY: FieldSpec = floored("delay", "s", 0.02);
const LIFETIME: FieldSpec = floored("lifetime", "s", 0.05);
const COOLDOWN: FieldSpec = floored("cooldown", "s", 0.02);
/// Every remaining length, whatever it hangs off.
const ANY_RADIUS: FieldSpec = floored("*radius", "u", 0.05);
/// The same, for the other half of a box.
const ANY_HEIGHT: FieldSpec = floored("*height", "u", 0.05);

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
    PROJECTILE_LIFETIME,
];
const ANCHOR_PICKS: &[FieldSpec] = &[BODY_RADIUS, MASS];
const ASTEROID_PICKS: &[FieldSpec] = &[RADIUS, MASS, INVULNERABLE, SEED];
/// The whole point of a spaceship object is WHICH ship and WHO flies it, and a
/// pick takes the field with everything under it - so the hull's source and the
/// controller's own fields come along.
const SPACESHIP_PICKS: &[FieldSpec] = &[HULL, CONTROLLER, ALLEGIANCE];
const BEACON_PICKS: &[FieldSpec] = &[LABEL, RADIUS, COLOR, AREA_RADIUS];
const SALVAGE_PICKS: &[FieldSpec] = &[SIZE, AREA_RADIUS];
/// No `aim`. The node's ROTATION aims the light (`node.rs`), and two controls
/// on one output is a builder turning the gizmo and watching nothing happen.
const LIGHT_PICKS: &[FieldSpec] = &[ILLUMINANCE, INTENSITY, COLOR, RANGE, RADIUS, SHADOWS];
/// The fields no kind shows first, which still carry a unit and a floor once
/// View > All Fields puts them back.
const UNPICKED: &[FieldSpec] = &[
    HEALTH, WIDTH, DELAY, LIFETIME, COOLDOWN, ANY_RADIUS, ANY_HEIGHT,
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
    ANCHOR_PICKS,
    ASTEROID_PICKS,
    SPACESHIP_PICKS,
    BEACON_PICKS,
    SALVAGE_PICKS,
    LIGHT_PICKS,
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
    let declared = || DECLARED.iter().copied().flatten();
    declared()
        .find(|spec| spec.is_named(name))
        .or_else(|| declared().find(|spec| spec.covers(name)))
        .copied()
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

/// The float `value` holds, whichever width it was authored at.
fn as_number(value: &dyn PartialReflect) -> Option<f64> {
    value
        .try_downcast_ref::<f32>()
        .map(|number| f64::from(*number))
        .or_else(|| value.try_downcast_ref::<f64>().copied())
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

/// The type path of an `Option`'s payload, or `None` when the value is not an
/// `Option` the type registry knows the shape of.
///
/// Needed because a field currently holding `None` cannot be asked what it
/// would hold: the type info is the only place that answer lives.
fn option_payload(value: &dyn PartialReflect) -> Option<String> {
    let TypeInfo::Enum(info) = value.get_represented_type_info()? else {
        return None;
    };
    let VariantInfo::Tuple(variant) = info.variant("Some")? else {
        return None;
    };
    Some(variant.field_at(0)?.type_path().to_string())
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
    if let Some(text) = leaf_text(value) {
        let leaf = if is_number(value) {
            RowValue::Number(text)
        } else {
            RowValue::Text(text)
        };
        out.push(walked(root, path, false, leaf));
        return;
    }
    if is_option(value) {
        walk_option(value, root, path, out);
        return;
    }
    match value.reflect_ref() {
        ReflectRef::Struct(fields) => {
            for index in 0..fields.field_len() {
                let (Some(name), Some(field)) = (fields.name_at(index), fields.field_at(index))
                else {
                    continue;
                };
                walk(
                    field,
                    root,
                    step(&path, PathStep::Field(name.to_string())),
                    out,
                );
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
            // A variant that carries FIELDS is a readout, not a choice:
            // switching to one would mean inventing every field of it that
            // nobody has authored. An enum whose variants are all bare names
            // has nothing to invent, so it is offered as a choice.
            let value = match unit_variants(value) {
                Some(options) => {
                    let name = chosen.variant_name();
                    RowValue::Choice {
                        chosen: options
                            .iter()
                            .position(|option| option == name)
                            .unwrap_or(0),
                        options,
                    }
                }
                None => RowValue::Fixed(chosen.variant_name().to_string()),
            };
            out.push(walked(root, path.clone(), false, value));
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
    let scalar = payload.as_deref().is_some_and(leaf_type);
    let present = matches!(value.reflect_ref(), ReflectRef::Enum(chosen) if chosen.field_len() > 0);
    if scalar {
        let text = match value.reflect_ref() {
            ReflectRef::Enum(chosen) => chosen.field_at(0).and_then(leaf_text).unwrap_or_default(),
            _ => String::new(),
        };
        // Off the PAYLOAD TYPE, not the value: a field holding `None` is still
        // a number's field, so it wears its unit and its name is still the grip
        // that scrubs it once it holds one.
        let leaf = if payload.as_deref().is_some_and(number_type) {
            RowValue::Number(text)
        } else {
            RowValue::Text(text)
        };
        out.push(walked(root, path, true, leaf));
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
fn unit_variants(value: &dyn PartialReflect) -> Option<Vec<String>> {
    let TypeInfo::Enum(info) = value.get_represented_type_info()? else {
        return None;
    };
    info.iter()
        .map(|variant| match variant {
            VariantInfo::Unit(unit) => Some(unit.name().to_string()),
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

/// Whether the parser has a leaf for this type, which is what decides between
/// an editable row and a readout.
fn leaf_type(type_path: &str) -> bool {
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

/// The value `path` names inside `root`, for writing.
fn resolve<'a>(
    root: &'a mut dyn PartialReflect,
    path: &[PathStep],
) -> Option<&'a mut dyn PartialReflect> {
    let mut value = root;
    for next in path {
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
            let value = parse_leaf(&payload, text)?;
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
            row.group
                .retain(|_| fields.next().is_some_and(|name| picked(name)));
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
        },
        fixed(
            FieldRoot::Config,
            "Allegiance",
            ship.allegiance.map_or_else(
                || IMPLIED_ALLEGIANCE.to_string(),
                |side| format!("{side:?}"),
            ),
        ),
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
    ships: usize,
    objects: usize,
    flown: Option<String>,
) -> Vec<InspectorRow> {
    vec![
        fixed(FieldRoot::Config, "Ships", ships.to_string()),
        fixed(FieldRoot::Config, "Objects", objects.to_string()),
        fixed(
            FieldRoot::Config,
            "Player Ship",
            flown.unwrap_or_else(|| NO_PLAYER_SHIP.to_string()),
        ),
    ]
}

/// The rows a section shows: what it was built from, what it is bound to, and
/// its kind config.
pub(crate) fn section_rows(
    node: &SectionNode,
    catalog: Option<&GameSections>,
) -> Vec<InspectorRow> {
    let mut rows = vec![fixed(FieldRoot::Config, "Part", node.prototype())];
    if node.bindable(catalog) {
        let binding = binding_label(&node.binds);
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
    }
    rows.extend(pose_rows(pose));
    rows
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
        axes_row(
            FieldRoot::Pose,
            "Position",
            "u",
            POSE_STEP,
            Limit::Free,
            pose.translation,
        ),
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
        ),
    ]
}

/// How far one pixel of a drag slides a node: fine enough to seat a beacon by
/// eye, coarse enough to cross the stage in one pull.
const POSE_STEP: f32 = 0.05;
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
