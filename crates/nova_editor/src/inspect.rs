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
use nova_scenario::prelude::{ScenarioObjectKind, SectionSource};
use nova_ship::prelude::{binding_label, GameSections, SectionConfig, SectionKind};

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
            Self::Text(text) | Self::Colour(text) | Self::Fixed(text) | Self::Key(text) => {
                text.clone()
            }
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
pub(crate) fn driver_label(driver: ShipDriver) -> &'static str {
    match driver {
        ShipDriver::Player => "Player",
        ShipDriver::Ai => "AI",
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
        value: RowValue::Fixed(text.into()),
    }
}

/// A walked row: its heading and its label both come from WHERE it sits, so a
/// caller that has the path never has to name it twice.
fn walked(root: FieldRoot, path: Vec<PathStep>, optional: bool, value: RowValue) -> InspectorRow {
    let (group, label) = heading_and_label(&path);
    // A unit belongs to a NUMBER. A checkbox or a variant name has none, and
    // one drawn beside it would be a label for the wrong thing.
    let unit = match value {
        RowValue::Text(_) => number_rule(&path).map_or("", |rule| rule.unit),
        _ => "",
    };
    InspectorRow {
        root,
        path,
        optional,
        group,
        label,
        unit,
        value,
    }
}

/// What a number is measured in, and the floor below which it is not that
/// number at all.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NumberRule {
    /// The unit shown beside the box, or the empty string where the value has
    /// no unit but still has a floor.
    pub(crate) unit: &'static str,
    /// The smallest value the field takes.
    pub(crate) floor: f32,
}

/// The rule for the field `path` ends at, if that field has one.
///
/// Keyed on the config's OWN field names, and by suffix where a name is a
/// family - every `*_radius` is a radius, whatever it hangs off. A field that
/// is not listed is unlabelled and unbounded on purpose: a floor invented for a
/// number nobody checked would refuse an edit the runtime accepts.
///
/// Lengths are `u` - the authored world unit. The HUD converts to metres for
/// the player; content does not, and this box is content.
fn number_rule(path: &[PathStep]) -> Option<NumberRule> {
    let name = path.iter().rev().find_map(|step| match step {
        PathStep::Field(name) => Some(name.as_str()),
        _ => None,
    })?;
    let (unit, floor) = match name {
        "illuminance" => ("lx", 0.0),
        "health" => ("hp", 0.0),
        "fire_rate" => ("/s", 0.0),
        "size" | "width" | "range" => ("u", 0.0),
        "steering_lag" | "delay" | "lifetime" | "cooldown" => ("s", 0.0),
        // No unit anyone would recognise, but a floor all the same: negative
        // mass and negative thrust are not values, they are typos.
        "mass" | "magnitude" | "max_torque" => ("", 0.0),
        name if name.ends_with("radius") || name.ends_with("height") => ("u", 0.0),
        _ => return None,
    };
    Some(NumberRule { unit, floor })
}

/// Refuse a number under its field's floor, in the words the box will show.
///
/// Checked HERE rather than where a negative radius used to be found out - the
/// spawn, at run time. The builder who typed it is the one who can fix it, and
/// by then they are flying the range.
///
/// The reason is the RULE, in three characters, because it is shown where the
/// unit stands: a sentence there would squeeze the box holding the number it
/// is about down to four characters.
fn check_floor(path: &[PathStep], value: &dyn PartialReflect) -> Result<(), String> {
    let Some(rule) = number_rule(path) else {
        return Ok(());
    };
    let number = value
        .try_downcast_ref::<f32>()
        .map(|number| f64::from(*number))
        .or_else(|| value.try_downcast_ref::<f64>().copied());
    let Some(number) = number else {
        return Ok(());
    };
    if number >= f64::from(rule.floor) {
        return Ok(());
    }
    Err(format!("min {}", number_text(f64::from(rule.floor))))
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
    if let Some(text) = leaf_text(value) {
        out.push(walked(root, path, false, RowValue::Text(text)));
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
        out.push(walked(root, path, true, RowValue::Text(text)));
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
/// `Spaceship` is absent because a ship in this editor is a [`ShipNode`] built
/// out of sections; the variant is only reachable through a document the
/// editor did not author, and it has no inspector until it does.
pub(crate) fn object_config(kind: &ScenarioObjectKind) -> Option<&dyn PartialReflect> {
    match kind {
        ScenarioObjectKind::Anchor(config) => Some(config),
        ScenarioObjectKind::Asteroid(config) => Some(config),
        ScenarioObjectKind::Beacon(config) => Some(config),
        ScenarioObjectKind::SalvageCrate(config) => Some(config),
        ScenarioObjectKind::Light(config) => Some(config),
        ScenarioObjectKind::Spaceship(_) => None,
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
        ScenarioObjectKind::Spaceship(_) => None,
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

/// The rows a ship shows: who flies it, and where it sits.
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
            value: RowValue::Driver(ship.driver),
        },
    ];
    rows.extend(pose_rows(pose));
    rows
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
        axes_row(FieldRoot::Pose, "Position", "u", pose.translation),
        // ROTATION, not heading: it is the node's rotation, and rotation is
        // what every other editor calls that. Both facts it used to keep in a
        // doc comment - degrees, and which turn is which - are on screen.
        axes_row(
            FieldRoot::Rotation,
            "Rotation",
            "deg, yaw/pitch/roll",
            rotation_degrees(pose),
        ),
    ]
}

/// One vector row: three numbers, each in the form every other number wears.
fn axes_row(root: FieldRoot, label: &str, unit: &'static str, value: Vec3) -> InspectorRow {
    InspectorRow {
        root,
        path: Vec::new(),
        optional: false,
        group: vec![TRANSFORM.to_string()],
        label: label.to_string(),
        unit,
        value: RowValue::Axes([value.x, value.y, value.z].map(|part| number_text(f64::from(part)))),
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
