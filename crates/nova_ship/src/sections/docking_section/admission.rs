//! Whether a ship may open a NEW dock with a target, by what the target is
//! and the state it is in. The port search ([`super::port`]) answers whether
//! the geometry allows a pair; this answers whether the target lets one in.
//! The `DOCK` chip and the request read the same [`DockAdmission`], so the
//! offer and the act cannot disagree.
//!
//! A ship target is judged by its combat state. A station target is judged by
//! its civilization's side and whether it is dead ([`station_admission`]): a
//! living station of the Enemy side refuses, a derelict admits whoever comes
//! to loot it, and a station already holding its one connection refuses the
//! next hull. The one-connection rule itself is the `DockedShip` on the root:
//! the port search names no pair for a docked hull, and the station branch
//! here names the reason the HUD shows.
//!
//! A dock already made is never judged again here: a ship under fire stays
//! docked, and the player hitting a docked Neutral is what forces the undock
//! (`input::ai::threat`).

use bevy::{ecs::system::SystemParam, prelude::*};
use nova_gameplay::prelude::*;

use super::DockedShip;
use crate::input::ai::prelude::AIBehaviorState;

/// Why a target refuses a dock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum DockRefusalType {
    /// A ship target's combat state: hostile to the ship asking, on the Enemy
    /// side, answering fire, or its AI fighting.
    Combat,
    /// A living station of an Enemy civilization.
    EnemyStation,
    /// A station whose one connection is already held.
    StationOccupied,
}

/// The station branch of admission: what a station on `allegiance`'s side
/// answers, dead or alive, with or without a hull already docked.
///
/// # Errors
///
/// [`DockRefusalType::StationOccupied`] while a hull is docked, whatever the
/// station is; then [`DockRefusalType::EnemyStation`] for a living station on
/// the Enemy side. A derelict admits on any side: it is a hull to loot, and a
/// `None` side is hostile to no one.
pub fn station_admission(
    allegiance: Option<Allegiance>,
    derelict: bool,
    occupied: bool,
) -> Result<(), DockRefusalType> {
    if occupied {
        return Err(DockRefusalType::StationOccupied);
    }
    if !derelict && allegiance == Some(Allegiance::Enemy) {
        return Err(DockRefusalType::EnemyStation);
    }
    Ok(())
}

/// The combat state of every ship a dock could name, and the station facts
/// of every station.
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
            Has<StationMarker>,
            Has<DerelictShipMarker>,
            Has<DockedShip>,
        ),
        With<SpaceshipRootMarker>,
    >,
}

impl DockAdmission<'_, '_> {
    /// Whether `target` lets `ship` dock: [`refusal`](Self::refusal) is
    /// `None`.
    pub fn admits(&self, ship: Entity, target: Entity) -> bool {
        self.refusal(ship, target).is_none()
    }

    /// Why `target` refuses `ship`, or `None` when it lets it dock.
    ///
    /// A station answers by [`station_admission`]. A neutralized ship always
    /// admits: the dock is a boarding. Otherwise a ship refuses while it is
    /// hostile to `ship`, while it is on the Enemy side, while it answers any
    /// ship's fire, and while its AI is fighting; a calm Player-aligned or
    /// Neutral ship lets it in. A target that is not a ship root has no
    /// state to refuse with.
    pub fn refusal(&self, ship: Entity, target: Entity) -> Option<DockRefusalType> {
        let Ok((allegiance, answering, state, neutralized, station, derelict, docked)) =
            self.ships.get(target)
        else {
            return None;
        };
        if station {
            return station_admission(allegiance.copied(), derelict, docked).err();
        }
        if neutralized {
            return None;
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
        (hostile || enemy || answers || fighting).then_some(DockRefusalType::Combat)
    }
}
