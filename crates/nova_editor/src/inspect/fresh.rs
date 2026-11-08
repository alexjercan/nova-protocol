//! A FRESH value of a type: what a switch to another enum variant, a set
//! optional and an added list item start from.
//!
//! Built from the type's reflection info, so a config the content grows
//! tomorrow needs no arm here. Every leaf starts at the value the editor's own
//! stock starts it at: an empty id (which the content lint names), a zero, or
//! the floor the field's declaration holds it to. A type with a
//! [`seeded`] start takes that instead, because a zero is not a thing it can
//! be: a price factor of zero gives the cargo away.
//!
//! A type this cannot build is an error, not a guess. The walk asks first and
//! offers only what can be built, so the error is never a row a builder sees.

use bevy::reflect::{list::DynamicList, map::DynamicMap};
use nova_gameplay::prelude::PriceFactor;

use super::*;
use crate::node::ObjectChoice;

/// How deep a fresh value may nest. Deeper is a type that holds itself, which
/// no authored config does through a field that is not a list.
const DEPTH: usize = 12;

/// The word an optional row offers for "holds one", beside [`NOTHING`].
pub(crate) const SET: &str = "set";

/// A fresh value of `info`, or why there cannot be one.
pub(super) fn fresh(info: &'static TypeInfo) -> Result<Box<dyn PartialReflect>, String> {
    fresh_at(info, None, 0)
}

/// [`fresh`] for the field `name` of `owner`, whose declaration may floor it.
fn fresh_at(
    info: &'static TypeInfo,
    field: Option<(&'static str, &'static str)>,
    depth: usize,
) -> Result<Box<dyn PartialReflect>, String> {
    let path = info.type_path();
    if depth > DEPTH {
        return Err(format!("{} nests too deep to start", short_type(path)));
    }
    if let Some(seed) = seeded(info) {
        return Ok(seed);
    }
    if leaf_type(path) {
        return parse_leaf(path, &seed_text(path, field));
    }
    match info {
        TypeInfo::Struct(fields) => {
            let mut built = DynamicStruct::default();
            built.set_represented_type(Some(info));
            for named in fields.iter() {
                let inner = named
                    .type_info()
                    .ok_or_else(|| format!("{} has no shape", named.type_path()))?;
                built.insert_boxed(
                    named.name(),
                    fresh_at(inner, Some((path, named.name())), depth + 1)?,
                );
            }
            Ok(Box::new(built))
        }
        TypeInfo::TupleStruct(fields) => {
            let mut built = DynamicTupleStruct::default();
            built.set_represented_type(Some(info));
            for slot in fields.iter() {
                let inner = slot
                    .type_info()
                    .ok_or_else(|| format!("{} has no shape", slot.type_path()))?;
                // A one-slot wrapper - a quantity, a typed id - is the field it
                // wraps: `Meters` under `radius` is floored as `radius`.
                built.insert_boxed(fresh_at(inner, field, depth + 1)?);
            }
            Ok(Box::new(built))
        }
        TypeInfo::Tuple(fields) => {
            let mut built = DynamicTuple::default();
            built.set_represented_type(Some(info));
            for slot in fields.iter() {
                let inner = slot
                    .type_info()
                    .ok_or_else(|| format!("{} has no shape", slot.type_path()))?;
                built.insert_boxed(fresh_at(inner, None, depth + 1)?);
            }
            Ok(Box::new(built))
        }
        TypeInfo::Enum(variants) => {
            if path.starts_with("core::option::Option<") {
                let mut built = DynamicEnum::new("None", DynamicVariant::Unit);
                built.set_represented_type(Some(info));
                return Ok(Box::new(built));
            }
            // The FIRST variant that can be built: a declaration's first
            // variant is the one its author meant as the plain case.
            let mut refused = format!("{} has no variant to start", short_type(path));
            for variant in variants.iter() {
                match fresh_variant(info, variant, depth) {
                    Ok(built) => return Ok(built),
                    Err(reason) => refused = reason,
                }
            }
            Err(refused)
        }
        TypeInfo::List(_) => {
            let mut built = DynamicList::default();
            built.set_represented_type(Some(info));
            Ok(Box::new(built))
        }
        TypeInfo::Map(_) => {
            let mut built = DynamicMap::default();
            built.set_represented_type(Some(info));
            Ok(Box::new(built))
        }
        _ => Err(format!("cannot start a {}", short_type(path))),
    }
}

/// `variant` of the enum `info`, every field of it fresh.
pub(super) fn fresh_variant(
    info: &'static TypeInfo,
    variant: &VariantInfo,
    depth: usize,
) -> Result<Box<dyn PartialReflect>, String> {
    let shape = match variant {
        VariantInfo::Unit(_) => DynamicVariant::Unit,
        VariantInfo::Struct(fields) => {
            let mut built = DynamicStruct::default();
            for named in fields.iter() {
                let inner = named
                    .type_info()
                    .ok_or_else(|| format!("{} has no shape", named.type_path()))?;
                built.insert_boxed(
                    named.name(),
                    fresh_at(inner, Some((info.type_path(), named.name())), depth + 1)?,
                );
            }
            DynamicVariant::Struct(built)
        }
        VariantInfo::Tuple(fields) => {
            let mut built = DynamicTuple::default();
            for slot in fields.iter() {
                let inner = slot
                    .type_info()
                    .ok_or_else(|| format!("{} has no shape", slot.type_path()))?;
                built.insert_boxed(fresh_at(inner, None, depth + 1)?);
            }
            DynamicVariant::Tuple(built)
        }
    };
    let mut built = DynamicEnum::new(variant.name(), shape);
    built.set_represented_type(Some(info));
    Ok(Box::new(built))
}

/// Whether [`fresh`] can build every field of `variant`.
pub(super) fn buildable(info: &'static TypeInfo, variant: &VariantInfo) -> bool {
    fresh_variant(info, variant, 0).is_ok()
}

/// The text a fresh leaf parses from.
///
/// A number starts at the floor its declaration holds it to, so a fresh field
/// is never one the box would refuse. A whole number nobody floored starts at
/// one: a count of nothing is not a count anyone adds.
fn seed_text(type_path: &str, field: Option<(&'static str, &'static str)>) -> String {
    if number_type(type_path) {
        let spec = field.and_then(|(owner, name)| {
            owned_spec(owner, name).or_else(|| field_spec(&[PathStep::Field(name.to_string())]))
        });
        let floor = match spec.map(|spec| spec.limit) {
            Some(Limit::AtLeast(low) | Limit::Between(low, _)) => f64::from(low),
            Some(Limit::Above(low, high)) => (f64::from(low) + 1.0).min(f64::from(high)),
            Some(Limit::Free) | None => 0.0,
        };
        let floor = if whole_type(type_path) {
            floor.max(1.0)
        } else {
            floor
        };
        return number_text(floor);
    }
    match type_path {
        "bool" => "false".to_string(),
        "bevy_color::color::Color"
        | "bevy_color::linear_rgba::LinearRgba"
        | "bevy_color::srgba::Srgba" => "#ffffff".to_string(),
        "glam::Vec3" | "glam::Quat" => "0, 0, 0".to_string(),
        _ if type_path == VariableExpressionNode::type_path() => "0".to_string(),
        _ if type_path == VariableConditionNode::type_path() => "true".to_string(),
        _ => String::new(),
    }
}

/// A start a type takes instead of the zero its fields would build.
///
/// A placed object's config is the palette's own stock, so a spawn switched to
/// a beacon gets the beacon the palette places - not a beacon of radius zero
/// nobody could see.
fn seeded(info: &'static TypeInfo) -> Option<Box<dyn PartialReflect>> {
    seeds()
        .iter()
        .find(|seed| {
            seed.get_represented_type_info()
                .is_some_and(|held| held.type_id() == info.type_id())
        })
        .map(|seed| seed.to_dynamic())
}

/// Every [`seeded`] start, built once: a walk asks on every frame the panel
/// shows a choice.
fn seeds() -> &'static [Box<dyn PartialReflect>] {
    static SEEDS: std::sync::OnceLock<Vec<Box<dyn PartialReflect>>> = std::sync::OnceLock::new();
    SEEDS.get_or_init(|| {
        let mut seeds: Vec<Box<dyn PartialReflect>> = vec![Box::new(PriceFactor::ONE)];
        for choice in ObjectChoice::ALL {
            let stock = choice.stock().kind;
            if let ReflectRef::Enum(kind) = stock.reflect_ref() {
                if let Some(payload) = kind.field_at(0) {
                    seeds.push(payload.to_dynamic());
                }
            }
        }
        seeds
    })
}
