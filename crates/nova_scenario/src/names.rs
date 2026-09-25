//! [`Names`]: what an authored string in a config REFERS TO.
//!
//! A handler is held together by strings. `SetAllegiance` names a ship,
//! `TimerCancel` names a timer, `NextScenario` names a scenario - and to the
//! type system all three are `String`, so a surface that shows a config to a
//! person can only offer a blank box and hope. The lint knows the difference
//! and has always known it, by reading the config type and the field name
//! together; that knowledge lived in one `match` arm per action and nowhere a
//! second reader could reach.
//!
//! This puts it on the FIELD, as a reflect custom attribute:
//!
//! ```ignore
//! #[reflect(@Names::Object)]
//! pub id: String,
//! ```
//!
//! [`walk_names`] is how anything reads them back, and it is the ONLY reader:
//! the lint resolves every [`Names::Object`] a config holds through it, and the
//! editor sorts the same visit into the id lists its panel offers. A surface
//! that answered the question from a list of its own would be a second table
//! to keep in step, and the day the two disagreed an author would get a green
//! lint on a handler the game silently drops.
//!
//! Resolving one of those [`Names::Object`] strings is
//! [`object_reference_resolves`], and for the same reason: the content lint,
//! the inspector's fault paint and the editor's lowering all judge a reference
//! by it. A copy that forgot one clause of that rule - the empty prefix - would
//! pass every reference in the document and still report success.
//!
//! Touch this module when a config grows a string that names something.

use bevy::{
    prelude::*,
    reflect::{ReflectRef, TypeInfo},
};

/// Glob-import surface: `use nova_scenario::names::prelude::*` brings the
/// [`Names`] field attribute, the walk that reads it, and the rule an object
/// reference resolves by into scope.
pub mod prelude {
    pub use super::{object_reference_resolves, walk_names, AuthoredName, Names};
}

/// What a string field names.
///
/// The split between [`Names::Object`] and [`Names::NewObject`] is the one
/// that matters to a reader: a reference must already exist and is offered as
/// a choice, while a declaration mints an id and must not collide.
///
/// [`Names::Order`] and [`Names::Section`] are both references, but neither
/// resolves against the scenario's spawns: an order key is minted by the helm
/// action that installs it, and a section id is only meaningful inside the one
/// ship the same config names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum Names {
    /// A scenario object something ELSE spawned: a ship, a rock, a beacon, an
    /// area. Every one of these has to resolve against the scenario's own
    /// spawns or the handler fires at nothing.
    Object,
    /// The id a spawn DECLARES. Unique within the scenario; every
    /// [`Names::Object`] reference resolves against the set of these.
    NewObject,
    /// A scenario variable key.
    Variable,
    /// A scenario-local timer key.
    Timer,
    /// A scenario in the campaign, by its registered id.
    Scenario,
    /// A HUD objective, by the id it was posted under.
    Objective,
    /// A scripted ship order, by the key its completion is reported under.
    /// Declared by the helm action that installs the order and referenced by
    /// the `ShipOrder` filter that waits for it.
    Order,
    /// A cinematic, by the key the `Cinematic` action filed its cursor under.
    Cinematic,
    /// One section of a ship, by its authored section id. Scoped to the ship
    /// the same config names, unlike every other variant here.
    Section,
}

/// One authored string a config holds, and what the author said it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthoredName<'a> {
    /// What this string names.
    pub names: Names,
    /// The field the author wrote it in, so a finding can point at the line
    /// rather than at the action. This is the enclosing FIELD even when the
    /// attribute sits on an enum VARIANT (`look_at`, not `Object`), because
    /// the field is what the RON spells.
    pub field: &'a str,
    /// The string exactly as authored. An EMPTY one is reported like any
    /// other: at every site that resolves a reference an empty id is a
    /// finding, and a walk that swallowed it would decide that for the caller.
    pub text: &'a str,
}

/// Visit every id `value` names, with what the field said it names.
///
/// Reflection and the [`Names`] attribute, rather than a match arm per action:
/// a check written as a list of action kinds goes stale the day the vocabulary
/// grows one, silently, in the direction of "this reference is fine".
///
/// An `Option` field is visited only when it is `Some` - an unset filter id
/// matches any entity and names nothing.
pub fn walk_names(value: &dyn PartialReflect, visit: &mut impl FnMut(AuthoredName<'_>)) {
    walk_field(value, "", visit);
}

/// [`walk_names`] with the enclosing field name carried down, so a name found
/// on an enum variant still reports the field the author wrote it in.
///
/// `dyn` rather than a generic, because the recursion would otherwise
/// instantiate a fresh closure type at every level.
fn walk_field(value: &dyn PartialReflect, field: &str, visit: &mut dyn FnMut(AuthoredName<'_>)) {
    match value.reflect_ref() {
        ReflectRef::Struct(fields) => {
            let info = match value.get_represented_type_info() {
                Some(TypeInfo::Struct(info)) => Some(info),
                _ => None,
            };
            for index in 0..fields.field_len() {
                let Some(held) = fields.field_at(index) else {
                    continue;
                };
                let declared = info.and_then(|info| info.field_at(index));
                let names = declared
                    .and_then(|declared| declared.get_attribute::<Names>())
                    .copied();
                let field = declared.map_or(field, |declared| declared.name());
                match (names, text_of(held)) {
                    (Some(names), Some(text)) => visit(AuthoredName { names, field, text }),
                    _ => walk_field(held, field, visit),
                }
            }
        }
        ReflectRef::TupleStruct(fields) => {
            for index in 0..fields.field_len() {
                if let Some(held) = fields.field(index) {
                    walk_field(held, field, visit);
                }
            }
        }
        ReflectRef::List(items) => {
            for index in 0..items.len() {
                if let Some(item) = items.get(index) {
                    walk_field(item, field, visit);
                }
            }
        }
        ReflectRef::Enum(chosen) => {
            // A variant can carry the attribute too: `CameraLookAtConfig` names
            // an object in ONE of its shapes, and the choice is the field.
            let variant = match value.get_represented_type_info() {
                Some(TypeInfo::Enum(info)) => info.variant_at(chosen.variant_index()),
                _ => None,
            };
            let names = variant
                .and_then(|variant| variant.get_attribute::<Names>())
                .copied();
            if let (Some(names), Some(text)) = (names, text_of(value)) {
                visit(AuthoredName { names, field, text });
                return;
            }
            for index in 0..chosen.field_len() {
                if let Some(held) = chosen.field_at(index) {
                    walk_field(held, field, visit);
                }
            }
        }
        _ => {}
    }
}

/// The string a field holds - itself, or the first payload of the variant it
/// holds, however many enums deep.
///
/// Deep because an AI `orbit` is `Some(Authored("planetoid"))`: stopping at
/// the `Some` found a `WellTargetType` rather than the id, and the lint never
/// saw the reference.
fn text_of(value: &dyn PartialReflect) -> Option<&str> {
    if let Some(text) = value.try_downcast_ref::<String>() {
        return Some(text);
    }
    let ReflectRef::Enum(chosen) = value.reflect_ref() else {
        return None;
    };
    text_of(chosen.field_at(0)?)
}

/// Whether `target` names an object the scenario puts on the board.
///
/// The rule every surface that judges a [`Names::Object`] reference answers
/// by: `declared` holds the ids spawns and areas mint outright, `prefixes` the
/// ones a scatter stands for - a scatter mints `<prefix><n>` at runtime, so
/// nothing static can list those ids and a reference resolves by its opening
/// instead.
///
/// An EMPTY prefix is not a prefix. Every string starts with one, so a single
/// unfilled `id_prefix` would answer yes for every reference and retire the
/// whole reference pass while still reporting success. The editor's stock
/// Scatter action is born with an empty prefix, so this is the ordinary case
/// and not a corner of it.
///
/// An empty `target` is judged like any other string, as [`AuthoredName::text`]
/// says: a surface that tolerates a half-typed field decides that for itself
/// before it asks.
pub fn object_reference_resolves<D, P>(target: &str, declared: D, prefixes: P) -> bool
where
    D: IntoIterator,
    D::Item: AsRef<str>,
    P: IntoIterator,
    P::Item: AsRef<str>,
{
    declared.into_iter().any(|id| id.as_ref() == target)
        || prefixes.into_iter().any(|prefix| {
            let prefix = prefix.as_ref();
            !prefix.is_empty() && target.starts_with(prefix)
        })
}

#[cfg(test)]
mod tests {
    use nova_events::prelude::*;
    use nova_ship::prelude::WellTargetType;

    use crate::prelude::*;

    /// The attribute is the table: a config the lint never mentions still
    /// reports the object it names, because the field says so.
    #[test]
    fn an_action_the_lint_never_mentions_still_reports_the_id_it_names() {
        let config = SetInfiniteAmmoActionConfig {
            id: "raider_1".to_string(),
            enabled: true,
        };
        let mut found = Vec::new();
        walk_names(&config, &mut |named| {
            found.push((named.names, named.field.to_string(), named.text.to_string()));
        });
        assert_eq!(
            found,
            vec![(Names::Object, "id".to_string(), "raider_1".to_string())]
        );
    }

    /// A name on an enum VARIANT reports the field it was authored in, not the
    /// variant: `look_at` is what the RON spells and what a finding must name.
    #[test]
    fn a_name_on_a_variant_reports_the_field_it_was_authored_in() {
        let config = SetCameraAnchorActionConfig {
            anchor: "station".to_string(),
            offset: Meters3::ZERO,
            frame: CameraOffsetFrame::Local,
            look_at: CameraLookAtConfig::Object("raider_1".to_string()),
            blend: None,
        };
        let mut found = Vec::new();
        walk_names(&config, &mut |named| {
            found.push((named.field.to_string(), named.text.to_string()));
        });
        assert_eq!(
            found,
            vec![
                ("anchor".to_string(), "station".to_string()),
                ("look_at".to_string(), "raider_1".to_string()),
            ]
        );
    }

    /// An orbit's well is an object reference only when it is authored: the
    /// nearest-well target names nothing the lint could resolve.
    #[test]
    fn an_orbit_well_names_an_object_only_when_authored() {
        let names = |well| {
            let config = OrbitShipActionConfig {
                order: "ring".to_string(),
                ship: "surveyor".to_string(),
                well,
            };
            let mut found = Vec::new();
            walk_names(&config, &mut |named| {
                if named.names == Names::Object {
                    found.push((named.field.to_string(), named.text.to_string()));
                }
            });
            found
        };

        assert_eq!(
            names(WellTargetType::Authored("planetoid".to_string())),
            vec![
                ("ship".to_string(), "surveyor".to_string()),
                ("well".to_string(), "planetoid".to_string()),
            ]
        );
        assert_eq!(
            names(WellTargetType::NearestToShip),
            vec![("ship".to_string(), "surveyor".to_string())]
        );

        // The AI routine holds the same target one `Option` deeper.
        let orbit = |orbit| {
            let mut found = Vec::new();
            walk_names(
                &AIControllerConfig {
                    orbit,
                    ..Default::default()
                },
                &mut |named| found.push((named.names, named.text.to_string())),
            );
            found
        };
        assert_eq!(
            orbit(Some(WellTargetType::Authored("planetoid".to_string()))),
            vec![(Names::Object, "planetoid".to_string())]
        );
        assert!(orbit(Some(WellTargetType::NearestToShip)).is_empty());
        assert!(orbit(None).is_empty());
    }

    /// An unfilled `id_prefix` satisfies NOTHING. Every string starts with the
    /// empty one, so a prefix list that honoured it would answer yes for every
    /// reference and retire the caller's whole reference pass in silence.
    #[test]
    fn an_empty_scatter_prefix_satisfies_no_reference() {
        let declared = ["station".to_string()];

        assert!(object_reference_resolves(
            "station",
            &declared,
            &[String::new()]
        ));
        assert!(
            !object_reference_resolves("raider_1", &declared, &[String::new()]),
            "an empty prefix must not stand in for one the author never wrote"
        );
        assert!(
            object_reference_resolves("raider_1", &declared, &["raider_".to_string()]),
            "a prefix the author did write still satisfies the ids it mints"
        );
    }
}
