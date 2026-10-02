//! Minimal faction/relation model: who is hostile to whom, and no more.
//!
//! The game needs exactly three answers about any pair of entities - own,
//! hostile, or neutral - to drive AI target selection and HUD coloring This
//! module provides an [`Allegiance`] component and a pure [`relation`] resolver
//! over optional allegiances, so callers can pass `Option<&Allegiance>`
//! straight from a query and unmarked entities (asteroids, debris) resolve as
//! neutral. A fuller faction system (alliances, reputation) is deliberately out
//! of scope.
//!
//! One ship-local exception rides beside the allegiance: a
//! [`RetaliationTarget`] makes a ship and the one ship it answers hostile to
//! each other without changing either side. [`ship_relation`] is the resolver
//! for any pair of ships; [`relation`] stays the side-only rule it falls back
//! to.

use bevy::prelude::*;

use crate::projectile_hooks::ProjectileOwner;

/// `Relation`, `Allegiance`, the `relation` helper and `NovaRelationsPlugin`.
pub mod prelude {
    pub use super::{
        projectile_party, relation, ship_relation, Allegiance, NovaRelationsPlugin, Relation,
        RelationParty, RetaliationTarget,
    };
}

/// Which side an entity fights for. Lives on ship roots (the Player/AI
/// spaceship markers require it) and is copied onto projectiles at spawn so
/// "your own torpedo" keeps reading as yours even if the shooter dies.
/// Serde: authorable in scenario RON via `SpaceshipConfig.allegiance`
/// (neutral haulers, scripted bystanders).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component)]
pub enum Allegiance {
    /// The player's side.
    Player,
    /// The hostile AI side.
    Enemy,
    /// Unaligned: bystanders and scripted haulers, hostile to no one.
    Neutral,
}

/// How two entities stand to each other, resolved by [`relation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum Relation {
    /// Same combatant side: an entity and its own ship/projectiles/allies.
    Own,
    /// Opposing combatant sides: valid targets for each other.
    Hostile,
    /// Everything else: bystanders, unmarked bodies, anything neutral.
    Neutral,
}

/// Resolve the relation between two entities' allegiances, as taken from a
/// query (`None` = the entity carries no [`Allegiance`] and is a bystander).
///
/// Only combatant sides relate strongly: Player and Enemy are [`Relation::Hostile`]
/// to each other and [`Relation::Own`] to themselves. A [`Allegiance::Neutral`]
/// or missing allegiance on either side resolves [`Relation::Neutral`] - two
/// neutral asteroids are not each other's "own" in any meaningful sense.
pub fn relation(a: Option<&Allegiance>, b: Option<&Allegiance>) -> Relation {
    match (a, b) {
        (Some(Allegiance::Player), Some(Allegiance::Player))
        | (Some(Allegiance::Enemy), Some(Allegiance::Enemy)) => Relation::Own,
        (Some(Allegiance::Player), Some(Allegiance::Enemy))
        | (Some(Allegiance::Enemy), Some(Allegiance::Player)) => Relation::Hostile,
        _ => Relation::Neutral,
    }
}

/// The one ship this ship answers with fire although their sides are not
/// hostile: the most recent ship that damaged an armed Neutral AI ship. Its
/// allegiance and its civilization's side stay as they were; [`ship_relation`]
/// reads the pair as hostile while this names the other ship. The AI sets it
/// on a hit and clears it when the target is lost or leaves the ship's
/// territory, or the ship stops being Neutral. Required by the AI ship marker; `None` on every calm ship.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct RetaliationTarget(pub Option<Entity>);

/// One side of a [`ship_relation`] question, as taken from a query.
#[derive(Debug, Clone, Copy)]
pub struct RelationParty<'a> {
    /// The entity asked about.
    pub entity: Entity,
    /// Its side; `None` is a bystander.
    pub allegiance: Option<&'a Allegiance>,
    /// The ship it answers, if it is a ship that can retaliate.
    pub retaliation: Option<&'a RetaliationTarget>,
}

/// Resolve the relation between two entities with the ship-local exception:
/// either one retaliating against the other makes the pair
/// [`Relation::Hostile`]; otherwise their sides decide through [`relation`].
/// A Neutral ship answering a third ship stays Neutral to everyone else.
pub fn ship_relation(a: RelationParty<'_>, b: RelationParty<'_>) -> Relation {
    let answers = |party: RelationParty<'_>, other: Entity| {
        party
            .retaliation
            .is_some_and(|target| target.0 == Some(other))
    };
    if answers(a, b.entity) || answers(b, a.entity) {
        return Relation::Hostile;
    }
    relation(a.allegiance, b.allegiance)
}

/// The [`RelationParty`] a projectile stands for: its owner ship, so a
/// retaliation pair also reads each other's torpedoes and slugs as hostile.
/// The side is the projectile's own, copied from the owner at launch.
pub fn projectile_party<'a>(
    projectile: Entity,
    allegiance: Option<&'a Allegiance>,
    owner: Option<&ProjectileOwner>,
    retaliation: &'a Query<&RetaliationTarget>,
) -> RelationParty<'a> {
    let entity = owner.map_or(projectile, |owner| owner.0);
    RelationParty {
        entity,
        allegiance,
        retaliation: retaliation.get(entity).ok(),
    }
}

/// Registers the [`Allegiance`] component for reflection so the faction model
/// serializes and inspects; adds no systems (relation resolution is the pure
/// [`relation`] function, called on demand).
pub struct NovaRelationsPlugin;

impl Plugin for NovaRelationsPlugin {
    fn build(&self, app: &mut App) {
        trace!("NovaRelationsPlugin: build");

        app.register_type::<Allegiance>();
        app.register_type::<RetaliationTarget>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relation_matrix() {
        use Allegiance::*;

        // Combatant sides: own to themselves, hostile to each other (both ways).
        assert_eq!(relation(Some(&Player), Some(&Player)), Relation::Own);
        assert_eq!(relation(Some(&Enemy), Some(&Enemy)), Relation::Own);
        assert_eq!(relation(Some(&Player), Some(&Enemy)), Relation::Hostile);
        assert_eq!(relation(Some(&Enemy), Some(&Player)), Relation::Hostile);

        // Neutral allegiance never relates strongly, not even to itself.
        assert_eq!(relation(Some(&Neutral), Some(&Neutral)), Relation::Neutral);
        assert_eq!(relation(Some(&Neutral), Some(&Player)), Relation::Neutral);
        assert_eq!(relation(Some(&Enemy), Some(&Neutral)), Relation::Neutral);

        // Missing allegiance = bystander, regardless of the other side.
        assert_eq!(relation(None, Some(&Player)), Relation::Neutral);
        assert_eq!(relation(Some(&Enemy), None), Relation::Neutral);
        assert_eq!(relation(None, None), Relation::Neutral);
    }

    #[test]
    fn retaliation_is_hostile_only_between_the_pair() {
        let neutral = Entity::from_raw_u32(1).unwrap();
        let shooter = Entity::from_raw_u32(2).unwrap();
        let bystander = Entity::from_raw_u32(3).unwrap();
        let answering = RetaliationTarget(Some(shooter));
        let party = |entity, allegiance, retaliation| RelationParty {
            entity,
            allegiance,
            retaliation,
        };
        let n = party(neutral, Some(&Allegiance::Neutral), Some(&answering));

        assert_eq!(
            ship_relation(n, party(shooter, Some(&Allegiance::Player), None)),
            Relation::Hostile
        );
        assert_eq!(
            ship_relation(party(shooter, Some(&Allegiance::Player), None), n),
            Relation::Hostile,
            "the shooter sees the ship answering it as hostile"
        );
        assert_eq!(
            ship_relation(party(bystander, Some(&Allegiance::Player), None), n),
            Relation::Neutral,
            "a Neutral answering someone else stays Neutral to a third ship"
        );
    }
}
