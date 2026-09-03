//! Finding what a command names.
//!
//! Every id-bearing command shares one rule: an id either resolves to exactly
//! one thing, or the shell says so in the same words. An unknown id and an
//! ambiguous one are different failures and read differently.

use bevy::prelude::*;
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::*;
use nova_os::prelude::{CommandClass, CommandResult};
use nova_scenario::prelude::live_ship_sections;

/// A command body that resolves an id before it can answer.
///
/// Both halves are finished answers, so `?` on a lookup ends the command with
/// the failure the player reads.
pub type Resolved = Result<CommandResult, CommandResult>;

/// The one answer the dispatcher returns for a [`Resolved`] body.
pub fn answered(resolved: Resolved) -> CommandResult {
    resolved.unwrap_or_else(|failure| failure)
}

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
    resolve(ships(world), id, "ship", "live")
}

/// Every live section of one ship, as `(entity, id)`, in id order.
pub fn sections(world: &mut World, ship: Entity) -> Vec<(Entity, String)> {
    let mut found: Vec<(Entity, String)> = live_ship_sections(world, ship)
        .into_iter()
        .map(|section| {
            let id = world
                .entity(section)
                .get::<EntityId>()
                .map_or_else(|| format!("{section}"), |id| id.0.clone());
            (section, id)
        })
        .collect();
    found.sort_by(|left, right| left.1.cmp(&right.1));
    found
}

/// The one section of `ship` with this id.
///
/// A section id is unique to its hull, not to the field: two `cargoa` hulls
/// both carry `turret_port`. The ship is therefore part of the address, and
/// this never has to answer "which one".
pub fn section(world: &mut World, ship: Entity, id: &str) -> Found {
    let ship_id = world
        .entity(ship)
        .get::<EntityId>()
        .map_or_else(|| format!("{ship}"), |id| id.0.clone());
    resolve(
        sections(world, ship),
        id,
        "section",
        &format!("on {ship_id}"),
    )
}

/// One id against a named candidate list. `scope` names where the list came
/// from, so a miss says which haystack was searched.
fn resolve(candidates: Vec<(Entity, String)>, id: &str, noun: &str, scope: &str) -> Found {
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
                format!("no {noun} named '{id}' ({scope}: none are live)")
            } else {
                format!("no {noun} named '{id}' ({scope}: {known})")
            })
        }
        count => Found::Ambiguous(format!(
            "'{id}' names {count} {noun}s {scope}; the world is inconsistent"
        )),
    }
}

/// The candidate ids as one comma-separated line, deduplicated.
fn names(candidates: &[(Entity, String)]) -> String {
    let mut names: Vec<&str> = candidates.iter().map(|(_, name)| name.as_str()).collect();
    names.dedup();
    names.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two hulls of the same class carrying the same section id, which is the
    /// shape every shipped scenario has: `cargoa` and `cargoa_raider` both
    /// carry `turret_port`.
    fn two_ships() -> (World, Entity, Entity) {
        let mut world = World::new();
        let mut ship = |id: &str, sections: &[&str]| {
            let ship = world
                .spawn((SpaceshipRootMarker, EntityId(id.to_string())))
                .id();
            for section in sections {
                world.spawn((
                    SectionMarker,
                    EntityId((*section).to_string()),
                    ChildOf(ship),
                ));
            }
            ship
        };
        let player = ship("cargoa", &["hull_front", "turret_port"]);
        let raider = ship("cargoa_raider", &["turret_port"]);
        (world, player, raider)
    }

    /// A section id is unique to its hull, not to the field, so the ship is
    /// part of the address and the same id on two ships is not ambiguous.
    #[test]
    fn a_section_id_resolves_within_the_ship_that_holds_it() {
        let (mut world, player, raider) = two_ships();
        let found = |world: &mut World, ship, id| match section(world, ship, id) {
            Found::One(entity) => Some(entity),
            _ => None,
        };
        let mine = found(&mut world, player, "turret_port").expect("the player's turret");
        let theirs = found(&mut world, raider, "turret_port").expect("the raider's turret");
        assert_ne!(mine, theirs, "each ship answers with its own section");
    }

    /// A miss says which haystack was searched and what is in it, so the player
    /// can retype rather than guess.
    #[test]
    fn a_missing_id_names_the_scope_and_what_is_live() {
        let (mut world, player, _) = two_ships();
        let Found::Missing(detail) = section(&mut world, player, "turret_dorsal") else {
            panic!("`turret_dorsal` is not on the player ship");
        };
        assert!(detail.contains("on cargoa"), "{detail}");
        assert!(detail.contains("hull_front, turret_port"), "{detail}");

        let Found::Missing(detail) = ship(&mut world, "nobody") else {
            panic!("no ship is called `nobody`");
        };
        assert!(detail.contains("cargoa, cargoa_raider"), "{detail}");
    }
}
