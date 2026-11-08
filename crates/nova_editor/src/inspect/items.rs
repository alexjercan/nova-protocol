//! Adding to and dropping from a walked list.
//!
//! A list a form or a script block owns is drawn with a remove control at the
//! head of each entry and an add control after the last one. A list of
//! distinct bare names - a section's damage effects - is drawn as one toggle
//! per name instead, because a set has nothing to order and nothing to
//! repeat.
//!
//! A new entry starts from [`fresh`](super::fresh) and is then made DISTINCT
//! from its siblings, because the content keys entries by what they hold: two
//! price rules for one cluster and category are a fault, and two fixtures
//! named alike are one fixture. Where no distinct entry is left, the add is
//! refused and says so.

use super::{fresh::fresh_variant, *};

/// The one option of a list whose entries have one shape.
pub(crate) const ADD: &str = "Add";

/// How many combinations of keys an add tries before it gives up. A set of
/// keys this large is no set a builder fills by hand.
const TRIES: usize = 4096;

/// The rows of the list `value` at `path`: each entry under its remove
/// control, then the add control. A set of names is one row of toggles.
pub(super) fn list_rows(
    value: &dyn PartialReflect,
    root: FieldRoot,
    path: Vec<PathStep>,
    out: &mut Vec<InspectorRow>,
) {
    let ReflectRef::List(items) = value.reflect_ref() else {
        return;
    };
    if let Some(set) = name_set(value) {
        out.push(walked(root, path, false, set));
        return;
    }
    for index in 0..items.len() {
        let Some(item) = items.get(index) else {
            continue;
        };
        let at = step(&path, PathStep::Item(index));
        out.push(walked(root, at.clone(), false, RowValue::Remove));
        walk_in(item, root, at, out, ListsType::Edited);
    }
    if let Some(options) = item_info(value).and_then(addable) {
        out.push(walked(root, path, false, RowValue::Append(options)));
    }
}

/// The type of an entry of the list `value`.
fn item_info(value: &dyn PartialReflect) -> Option<&'static TypeInfo> {
    let TypeInfo::List(info) = value.get_represented_type_info()? else {
        return None;
    };
    info.item_info()
}

/// The names of `info` when it is a set of bare names, and not an `Option`.
fn bare_names(info: &TypeInfo) -> Option<Vec<(String, String)>> {
    let TypeInfo::Enum(variants) = info else {
        return None;
    };
    if info.type_path().starts_with("core::option::Option<") {
        return None;
    }
    variants
        .iter()
        .map(|variant| {
            matches!(variant, VariantInfo::Unit(_))
                .then(|| (variant.name().to_string(), hint_of(variant.docs())))
        })
        .collect()
}

/// The list `value` as one toggle per name, when its entries are bare names
/// and no name is held twice. A list that repeats one is not a set, and is
/// drawn entry by entry so the repeat stays visible.
fn name_set(value: &dyn PartialReflect) -> Option<RowValue> {
    let names = bare_names(item_info(value)?)?;
    let ReflectRef::List(items) = value.reflect_ref() else {
        return None;
    };
    let held: Vec<&str> = items
        .iter()
        .filter_map(|item| match item.reflect_ref() {
            ReflectRef::Enum(chosen) => Some(chosen.variant_name()),
            _ => None,
        })
        .collect();
    let distinct = held
        .iter()
        .enumerate()
        .all(|(index, name)| !held[..index].contains(name));
    if !distinct || held.len() != items.len() {
        return None;
    }
    Some(RowValue::Set {
        on: names
            .iter()
            .map(|(name, _)| held.contains(&name.as_str()))
            .collect(),
        options: names.iter().map(|(name, _)| name.clone()).collect(),
        hints: names.into_iter().map(|(_, hint)| hint).collect(),
    })
}

/// What an add control offers for entries of `info`: each variant a new
/// entry can start as, or [`ADD`] for an entry of one shape. `None` where no
/// entry can be built.
fn addable(info: &'static TypeInfo) -> Option<Vec<String>> {
    if let TypeInfo::Enum(variants) = info {
        if !info.type_path().starts_with("core::option::Option<") {
            let offered: Vec<String> = variants
                .iter()
                .filter(|variant| fresh::buildable(info, variant))
                .map(|variant| variant.name().to_string())
                .collect();
            return (!offered.is_empty()).then_some(offered);
        }
    }
    fresh::fresh(info).ok()?;
    Some(vec![ADD.to_string()])
}

/// Add an entry `option` names to the end of the list at `path`.
///
/// `option` is [`ADD`], or the variant the entry starts as. The entry is made
/// distinct from its siblings first; where none is left, nothing is added.
pub(crate) fn append_item(
    root: &mut dyn PartialReflect,
    path: &[PathStep],
    option: &str,
) -> Result<(), String> {
    let noun = noun_of(path);
    let target = resolve(root, path).ok_or_else(|| "gone".to_string())?;
    let info = item_info(target).ok_or_else(|| "not a list".to_string())?;
    let mut item = match (option, info) {
        (ADD, _) => fresh::fresh(info)?,
        (name, TypeInfo::Enum(variants)) => {
            let variant = variants
                .variant(name)
                .ok_or_else(|| format!("{name} is not one of the options"))?;
            fresh_variant(info, variant, 0)?
        }
        (name, _) => return Err(format!("{name} is not one of the options")),
    };
    let ReflectMut::List(list) = target.reflect_mut() else {
        return Err("not a list".to_string());
    };
    distinct(item.as_mut(), list, &noun)?;
    list.push(item);
    Ok(())
}

/// Drop the entry at `path`, which ends in its [`PathStep::Item`].
pub(crate) fn remove_item(root: &mut dyn PartialReflect, path: &[PathStep]) -> Result<(), String> {
    let (PathStep::Item(index), list) = path.split_last().ok_or_else(|| "gone".to_string())? else {
        return Err("not an entry of a list".to_string());
    };
    let ReflectMut::List(items) = resolve(root, list)
        .ok_or_else(|| "gone".to_string())?
        .reflect_mut()
    else {
        return Err("not an entry of a list".to_string());
    };
    if *index >= items.len() {
        return Err("that entry is gone".to_string());
    }
    items.remove(*index);
    Ok(())
}

/// Add `name` to the set of names at `target`, or drop it where it is held.
pub(super) fn toggle_name(target: &mut dyn PartialReflect, name: &str) -> Result<(), String> {
    let info = item_info(target).ok_or_else(|| "not a list".to_string())?;
    let names = bare_names(info).ok_or_else(|| "not a set of names".to_string())?;
    if !names.iter().any(|(held, _)| held == name) {
        return Err(format!("{name} is not one of the options"));
    }
    let ReflectMut::List(items) = target.reflect_mut() else {
        return Err("not a list".to_string());
    };
    let held = (0..items.len()).find(|index| {
        items
            .get(*index)
            .is_some_and(|item| matches!(item.reflect_ref(), ReflectRef::Enum(chosen) if chosen.variant_name() == name))
    });
    match held {
        Some(index) => {
            items.remove(index);
        }
        None => {
            let TypeInfo::Enum(variants) = info else {
                return Err("not a set of names".to_string());
            };
            let variant = variants
                .variant(name)
                .ok_or_else(|| format!("{name} is not one of the options"))?;
            items.push(fresh_variant(info, variant, 0)?);
        }
    }
    Ok(())
}

/// What one entry of the list at `path` is called: `fixtures` -> `fixture`.
fn noun_of(path: &[PathStep]) -> String {
    let field = path
        .iter()
        .rev()
        .find_map(|step| match step {
            PathStep::Field(name) => Some(name.as_str()),
            _ => None,
        })
        .unwrap_or("entry");
    field.strip_suffix('s').unwrap_or(field).to_string()
}

/// One slot of an entry that keys it: a bare name, or a word of a closed
/// vocabulary. Two entries holding the same keys are one entry twice.
struct Key {
    /// Where the slot sits in the entry.
    step: PathStep,
    /// Every word it may hold, in the order a new entry tries them.
    words: Vec<String>,
}

/// The keying slots of an entry shaped like `item`.
fn keys_of(item: &dyn PartialReflect) -> Vec<Key> {
    let slots: Vec<(PathStep, &dyn PartialReflect)> = match item.reflect_ref() {
        ReflectRef::Struct(fields) => (0..fields.field_len())
            .filter_map(|index| {
                Some((
                    PathStep::Field(fields.name_at(index)?.to_string()),
                    fields.field_at(index)?,
                ))
            })
            .collect(),
        ReflectRef::Tuple(fields) => (0..fields.field_len())
            .filter_map(|index| Some((PathStep::Slot(index), fields.field(index)?)))
            .collect(),
        _ => Vec::new(),
    };
    slots
        .into_iter()
        .filter_map(|(step, value)| {
            let info = value.get_represented_type_info()?;
            let words: Vec<String> = match bare_names(info) {
                Some(names) => names.into_iter().map(|(name, _)| name).collect(),
                None => vocabulary_of_type(info.type_path())?
                    .into_iter()
                    .map(|(word, _)| word.to_string())
                    .collect(),
            };
            Some(Key { step, words })
        })
        .collect()
}

/// The word the keying slot `step` of `item` holds.
fn word_at(item: &dyn PartialReflect, step: &PathStep) -> Option<String> {
    let value = peek(item, core::slice::from_ref(step))?;
    match value.reflect_ref() {
        ReflectRef::Enum(chosen) => Some(chosen.variant_name().to_string()),
        _ => leaf_text(value),
    }
}

/// Make the new `item` differ from every entry `list` already holds: the
/// first combination of keys no sibling holds, and a minted id where the
/// entry has an empty one.
fn distinct(
    item: &mut dyn PartialReflect,
    list: &dyn bevy::reflect::list::List,
    noun: &str,
) -> Result<(), String> {
    let keys = keys_of(item);
    if !keys.is_empty() {
        let held: Vec<Vec<String>> = list
            .iter()
            .map(|sibling| {
                keys.iter()
                    .map(|key| word_at(sibling, &key.step).unwrap_or_default())
                    .collect()
            })
            .collect();
        let free = combinations(&keys)
            .take(TRIES)
            .find(|words| !held.contains(words))
            .ok_or_else(|| format!("every {noun} already has an entry"))?;
        for (key, word) in keys.iter().zip(free) {
            choose_field(item, core::slice::from_ref(&key.step), &word)?;
        }
    }
    let id = [PathStep::Field("id".to_string())];
    let empty_id = peek(item, &id)
        .and_then(|held| held.try_downcast_ref::<String>())
        .is_some_and(String::is_empty);
    if empty_id {
        let taken: Vec<String> = list
            .iter()
            .filter_map(|sibling| word_at(sibling, &id[0]))
            .collect();
        let minted = (1..)
            .map(|count| format!("{noun}_{count}"))
            .find(|candidate| !taken.contains(candidate))
            .unwrap_or_default();
        write_field(item, &id, false, &minted)?;
    }
    Ok(())
}

/// Every combination of `keys`' words, the first key's words varying
/// slowest: a new price rule takes the next category for the first cluster
/// before it moves to the next cluster.
fn combinations(keys: &[Key]) -> impl Iterator<Item = Vec<String>> + '_ {
    let total: usize = keys.iter().map(|key| key.words.len().max(1)).product();
    (0..total).map(move |mut index| {
        let mut words = vec![String::new(); keys.len()];
        for (slot, key) in keys.iter().enumerate().rev() {
            let count = key.words.len().max(1);
            words[slot] = key.words.get(index % count).cloned().unwrap_or_default();
            index /= count;
        }
        words
    })
}
