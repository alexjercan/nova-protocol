//! Minimal faction/relation model: who is hostile to whom, and no more.
//!
//! The game needs exactly three answers about any pair of entities - own,
//! hostile, or neutral - to drive AI target selection and HUD coloring
//! (task 20260708-203708). This module provides an [`Allegiance`] component
//! and a pure [`relation`] resolver over optional allegiances, so callers
//! can pass `Option<&Allegiance>` straight from a query and unmarked
//! entities (asteroids, debris) resolve as neutral. A fuller faction system
//! (alliances, reputation) is deliberately out of scope; spike it if the
//! game ever needs one.

use bevy::prelude::*;

pub mod prelude {
    pub use super::{relation, Allegiance, NovaRelationsPlugin, Relation};
}

/// Which side an entity fights for. Lives on ship roots (the Player/AI
/// spaceship markers require it) and is copied onto projectiles at spawn so
/// "your own torpedo" keeps reading as yours even if the shooter dies.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub enum Allegiance {
    Player,
    Enemy,
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

pub struct NovaRelationsPlugin;

impl Plugin for NovaRelationsPlugin {
    fn build(&self, app: &mut App) {
        debug!("NovaRelationsPlugin: build");

        app.register_type::<Allegiance>();
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
}
