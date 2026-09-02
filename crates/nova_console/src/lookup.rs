//! Finding what a command names.
//!
//! Every id-bearing command shares one rule: an id either resolves to exactly
//! one thing, or the shell says so in the same words. An unknown id and an
//! ambiguous one are different failures and read differently.

use bevy::prelude::*;
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::*;
use nova_os::prelude::{CommandClass, CommandResult};

/// What an id resolved to.
pub enum Found {
    /// Exactly one match.
    One(Entity),
    /// Nothing matched; the message lists what is available.
    Missing(String),
    /// Several matched; the message lists them.
    Ambiguous(String),
}

impl Found {
    /// The entity, or the failure as a result under this command's name.
    pub fn or_error(self, command: &str, class: CommandClass) -> Result<Entity, CommandResult> {
        match self {
            Found::One(entity) => Ok(entity),
            Found::Missing(detail) | Found::Ambiguous(detail) => {
                Err(CommandResult::error(command, Some(class), detail))
            }
        }
    }
}

/// Every live ship, as `(entity, id)`, sorted by id so two listings of an
/// unchanged world read the same.
pub fn ships(world: &mut World) -> Vec<(Entity, String)> {
    let mut query = world.query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>();
    let mut found: Vec<(Entity, String)> = query
        .iter(world)
        .map(|(entity, id)| (entity, id.0.clone()))
        .collect();
    found.sort_by(|left, right| left.1.cmp(&right.1));
    found
}

/// The one live ship with this id.
pub fn ship(world: &mut World, id: &str) -> Found {
    resolve(ships(world), id, "ship")
}

/// Every live section of one ship, as `(entity, id)`, in id order.
pub fn sections(world: &mut World, ship: Entity) -> Vec<(Entity, String)> {
    let mut query =
        world.query_filtered::<(Entity, &ChildOf, Option<&EntityId>), With<SectionMarker>>();
    let mut found: Vec<(Entity, String)> = query
        .iter(world)
        .filter(|(_, child_of, _)| child_of.parent() == ship)
        .map(|(entity, _, id)| {
            let name = id.map_or_else(|| format!("{entity}"), |id| id.0.clone());
            (entity, name)
        })
        .collect();
    found.sort_by(|left, right| left.1.cmp(&right.1));
    found
}

/// Every live section on every ship, as `(entity, id)`, in id order. Section
/// ids are per-ship, so this is what `section <id>` searches and what makes an
/// id shared by two ships genuinely ambiguous rather than silently the first.
pub fn all_sections(world: &mut World) -> Vec<(Entity, String)> {
    let mut query = world.query_filtered::<(Entity, &EntityId), With<SectionMarker>>();
    let mut found: Vec<(Entity, String)> = query
        .iter(world)
        .map(|(entity, id)| (entity, id.0.clone()))
        .collect();
    found.sort_by(|left, right| left.1.cmp(&right.1));
    found
}

/// The one live section with this id, anywhere.
pub fn section(world: &mut World, id: &str) -> Found {
    resolve(all_sections(world), id, "section")
}

/// One id against a named candidate list.
fn resolve(candidates: Vec<(Entity, String)>, id: &str, noun: &str) -> Found {
    let matched: Vec<Entity> = candidates
        .iter()
        .filter(|(_, name)| name == id)
        .map(|(entity, _)| *entity)
        .collect();
    match matched.len() {
        1 => Found::One(matched[0]),
        0 => {
            let known = names(&candidates);
            Found::Missing(if known.is_empty() {
                format!("no {noun} named '{id}' (none are live)")
            } else {
                format!("no {noun} named '{id}' (live: {known})")
            })
        }
        count => Found::Ambiguous(format!(
            "'{id}' names {count} {noun}s; address one by the ship that holds it"
        )),
    }
}

/// The candidate ids as one comma-separated line, deduplicated.
fn names(candidates: &[(Entity, String)]) -> String {
    let mut names: Vec<&str> = candidates.iter().map(|(_, name)| name.as_str()).collect();
    names.dedup();
    names.join(", ")
}
