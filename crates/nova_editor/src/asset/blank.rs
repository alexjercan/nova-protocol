//! A blank item of each kind: what `+ Add` makes.
//!
//! A blank copies nothing. Every field the form shows holds its least value:
//! zero, an empty text, list or map, `None`, or the first variant of an enum.
//! Where the format refuses a least value (a section's `draw_weight` of 0, a
//! recipe that costs 0 credits), the content lint says so in Problems, and
//! Save, Play and Package wait until the builder fills it. The blank never
//! hides a value the form does not show.
//!
//! The walk is over the reflected shape of the content type, so a field a
//! content type gains is blank here with no edit. A field the walk cannot make
//! is an error the Add reports, not a guess.

use std::any::TypeId;

use bevy::{
    prelude::*,
    reflect::{
        array::DynamicArray,
        enums::{DynamicEnum, DynamicVariant, VariantInfo},
        list::DynamicList,
        map::DynamicMap,
        set::DynamicSet,
        structs::DynamicStruct,
        tuple::DynamicTuple,
        tuple_struct::DynamicTupleStruct,
        TypeInfo, Typed,
    },
};
use nova_gameplay::prelude::AssetRef;
use nova_modding::prelude::{serialize_content, Content};
use nova_ship::prelude::{SectionConfig, SectionKind};

use super::blank_mesh;

/// A blank `T`: the least value of every reflected field.
pub(crate) fn blank<T: FromReflect + Typed>() -> Result<T, String> {
    let info = T::type_info();
    let value = blank_of(info)?;
    T::from_reflect(value.as_ref())
        .ok_or_else(|| format!("a blank {} did not build", info.ty().short_path()))
}

/// The names of the section kinds, in the order the format declares them: what
/// a new section asks for.
pub(crate) fn section_kinds() -> Vec<&'static str> {
    match SectionKind::type_info() {
        TypeInfo::Enum(info) => info.variant_names().to_vec(),
        _ => vec![],
    }
}

/// A variant name as a builder reads it: `CargoIntake` is `Cargo Intake`.
pub(crate) fn spaced(variant: &str) -> String {
    variant.chars().fold(String::new(), |mut spaced, letter| {
        if letter.is_uppercase() && !spaced.is_empty() {
            spaced.push(' ');
        }
        spaced.push(letter);
        spaced
    })
}

/// A blank section of `kind`, one of [`section_kinds`].
pub(crate) fn blank_section(kind: &str) -> Result<SectionConfig, String> {
    let TypeInfo::Enum(info) = SectionKind::type_info() else {
        return Err("a section kind is not an enum".to_string());
    };
    let variant = info
        .variant(kind)
        .ok_or_else(|| format!("{kind} is not a section kind"))?;
    let payload = blank_variant(variant)?;
    let mut kind = SectionKind::from_reflect(&DynamicEnum::new(variant.name(), payload))
        .ok_or_else(|| format!("a blank {} section did not build", variant.name()))?;
    // The render mesh is the mesh part's row, out of the reflection walk.
    blank_mesh(&mut kind);
    Ok(SectionConfig {
        base: blank()?,
        kind,
    })
}

/// A blank item of content `kind`. A section needs `section` (one of
/// [`section_kinds`]); the other kinds take none. A scenario is not made here:
/// it is a document of its own ([`crate::project::verbs::add_scenario`]).
///
/// A blank a file cannot hold is an error: a field the form does not show has
/// no least value a save can write, and an item nobody could finish here is
/// not one to make.
pub(crate) fn blank_item(kind: &str, section: Option<&str>) -> Result<Content, String> {
    let item = blank_content(kind, section)?;
    if let Err(fault) = serialize_content(std::slice::from_ref(&item)) {
        let what = section.map_or_else(|| kind.to_string(), spaced);
        return Err(format!("a new {what} does not save: {fault}"));
    }
    Ok(item)
}

/// [`blank_item`] before the check that a file can hold it.
fn blank_content(kind: &str, section: Option<&str>) -> Result<Content, String> {
    Ok(match kind {
        "section" => {
            let section = section.ok_or("a new section needs its kind")?;
            Content::Section(Box::new(blank_section(section)?))
        }
        "campaign" => Content::Campaign(blank()?),
        "style" => Content::Style(blank()?),
        "ui theme" => Content::UiTheme(Box::new(blank()?)),
        "item" => Content::Item(blank()?),
        "start role" => Content::StartRole(blank()?),
        "price table" => Content::PriceTable(blank()?),
        "trade profile" => Content::TradeProfile(blank()?),
        other => return Err(format!("a {other} has no blank")),
    })
}

/// The least value of the type `info` describes.
fn blank_of(info: &'static TypeInfo) -> Result<Box<dyn PartialReflect>, String> {
    // A zero quaternion turns nothing into nothing: no rotation a form could
    // show. The identity is the least rotation, and the form shows it as 0.
    if info.type_id() == TypeId::of::<Quat>() {
        return Ok(Box::new(Quat::IDENTITY));
    }
    Ok(match info {
        TypeInfo::Struct(info) => {
            let mut value = DynamicStruct::default();
            for field in info.iter() {
                value.insert_boxed(
                    field.name(),
                    blank_of(known(field.type_info(), field.name())?)?,
                );
            }
            Box::new(value)
        }
        TypeInfo::TupleStruct(info) => {
            let mut value = DynamicTupleStruct::default();
            for field in info.iter() {
                value.insert_boxed(blank_of(known(field.type_info(), info.type_path())?)?);
            }
            Box::new(value)
        }
        TypeInfo::Tuple(info) => {
            let mut value = DynamicTuple::default();
            for field in info.iter() {
                value.insert_boxed(blank_of(known(field.type_info(), info.type_path())?)?);
            }
            Box::new(value)
        }
        TypeInfo::Array(info) => {
            let item = known(info.item_info(), info.type_path())?;
            let items = (0..info.capacity())
                .map(|_| blank_of(item))
                .collect::<Result<Vec<_>, _>>()?;
            Box::new(DynamicArray::new(items.into_boxed_slice()))
        }
        TypeInfo::List(_) => Box::new(DynamicList::default()),
        TypeInfo::Map(_) => Box::new(DynamicMap::default()),
        TypeInfo::Set(_) => Box::new(DynamicSet::default()),
        TypeInfo::Enum(info) => {
            let first = info
                .variant_at(0)
                .ok_or_else(|| format!("{} has no variant", info.type_path()))?;
            Box::new(DynamicEnum::new(first.name(), blank_variant(first)?))
        }
        TypeInfo::Opaque(info) => leaf(info.type_id())
            .ok_or_else(|| format!("{} has no blank value", info.type_path()))?,
    })
}

/// The payload of `variant`, every field blank.
fn blank_variant(variant: &'static VariantInfo) -> Result<DynamicVariant, String> {
    Ok(match variant {
        VariantInfo::Unit(_) => DynamicVariant::Unit,
        VariantInfo::Tuple(info) => {
            let mut value = DynamicTuple::default();
            for field in info.iter() {
                value.insert_boxed(blank_of(known(field.type_info(), info.name())?)?);
            }
            DynamicVariant::Tuple(value)
        }
        VariantInfo::Struct(info) => {
            let mut value = DynamicStruct::default();
            for field in info.iter() {
                value.insert_boxed(
                    field.name(),
                    blank_of(known(field.type_info(), field.name())?)?,
                );
            }
            DynamicVariant::Struct(value)
        }
    })
}

/// A field's type, or the error that it has none the walk can read.
fn known(info: Option<&'static TypeInfo>, at: &str) -> Result<&'static TypeInfo, String> {
    info.ok_or_else(|| format!("{at} has no type the editor can read"))
}

/// The least value of a type reflection holds as one opaque value.
fn leaf(id: TypeId) -> Option<Box<dyn PartialReflect>> {
    macro_rules! zero {
        ($($ty:ty),*) => {
            $(if id == TypeId::of::<$ty>() {
                return Some(Box::new(<$ty>::default()));
            })*
        };
    }
    zero!(f32, f64, u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, bool, String);
    // A file ref holds the empty path, which the content lint refuses until a
    // file is picked. Its default is a handle, which no file can hold.
    macro_rules! no_file {
        ($($asset:ty),*) => {
            $(if id == TypeId::of::<AssetRef<$asset>>() {
                return Some(Box::new(AssetRef::<$asset>::Path(String::new())));
            })*
        };
    }
    no_file!(Image, AudioSource, WorldAsset);
    // The shared text an id newtype holds.
    if id == TypeId::of::<std::sync::Arc<str>>() {
        return Some(Box::new(std::sync::Arc::<str>::from("")));
    }
    None
}

#[cfg(test)]
mod tests {
    use nova_modding::prelude::{parse_content, serialize_content};

    use super::{blank_item, section_kinds};
    use crate::project::KINDS;

    /// Every kind `+ Add` offers, each section kind with it, makes a blank a
    /// save can write and a load can read.
    #[test]
    fn every_addable_kind_has_a_blank_a_file_can_hold() {
        let mut blanks = Vec::new();
        for row in KINDS.iter().filter(|row| row.add && row.kind != "scenario") {
            if row.kind == "section" {
                for kind in section_kinds() {
                    blanks.push(
                        blank_item("section", Some(kind)).unwrap_or_else(|fault| panic!("{fault}")),
                    );
                }
            } else {
                blanks.push(blank_item(row.kind, None).unwrap_or_else(|fault| panic!("{fault}")));
            }
        }
        let text = serialize_content(&blanks).expect("a blank serializes");
        let back = parse_content(text.as_bytes()).expect("a blank parses");
        assert_eq!(format!("{back:?}"), format!("{blanks:?}"));
    }
}
