//! Whether a ship may open a NEW dock with a target, by the target's combat
//! state. The port search ([`super::port`]) answers whether the geometry
//! allows a pair; this answers whether the target lets one in. The `DOCK`
//! chip and the request read the same [`DockAdmission`], so the offer and the
//! act cannot disagree.
//!
//! A dock already made is never judged again here: a ship under fire stays
//! docked, and the player hitting a docked Neutral is what forces the undock
//! (`input::ai::threat`).

use bevy::{ecs::system::SystemParam, prelude::*};
use nova_gameplay::prelude::*;

use crate::input::ai::prelude::AIBehaviorState;

/// The combat state of every ship a dock could name.
#[derive(SystemParam)]
pub struct DockAdmission<'w, 's> {
    ships: Query<
        'w,
        's,
        (
            Option<&'static Allegiance>,
            Option<&'static RetaliationTarget>,
            Option<&'static AIBehaviorState>,
            Has<NeutralizedMarker>,
        ),
        With<SpaceshipRootMarker>,
    >,
}

impl DockAdmission<'_, '_> {
    /// Whether `target` lets `ship` dock.
    ///
    /// A neutralized ship always does: the dock is a boarding. Otherwise the
    /// target refuses while it is hostile to `ship`, while it is on the Enemy
    /// side, while it answers any ship's fire, and while its AI is fighting;
    /// a calm Player-aligned or Neutral ship lets it in. A target that is not
    /// a ship root has no combat state to refuse with.
    pub fn admits(&self, ship: Entity, target: Entity) -> bool {
        let Ok((allegiance, answering, state, neutralized)) = self.ships.get(target) else {
            return true;
        };
        if neutralized {
            return true;
        }
        let (own_allegiance, own_answering) = self
            .ships
            .get(ship)
            .map_or((None, None), |(allegiance, answering, ..)| {
                (allegiance, answering)
            });
        let hostile = ship_relation(
            RelationParty {
                entity: ship,
                allegiance: own_allegiance,
                retaliation: own_answering,
            },
            RelationParty {
                entity: target,
                allegiance,
                retaliation: answering,
            },
        ) == Relation::Hostile;
        let enemy = allegiance == Some(&Allegiance::Enemy);
        let answers = answering.is_some_and(|answering| answering.is_some());
        let fighting = state.is_some_and(AIBehaviorState::engages);
        !(hostile || enemy || answers || fighting)
    }
}
