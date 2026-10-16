//! Whether a ship may open a NEW dock with a target, by what the target is
//! and the state it is in. The port search ([`super::port`]) answers whether
//! the geometry allows a pair; this answers whether the target lets one in.
//! The `DOCK` chip and the request read the same [`DockAdmission`], so the
//! offer and the act cannot disagree.
//!
//! A ship target is judged by its combat state. A station target is judged by
//! [`station_admission`]: a station with every port held refuses the next
//! hull; a hull parked at the station ([`ParkedHull`]) is let in whatever
//! the sides; a station the player owns ([`OwnedStationMarker`]) refuses nobody
//! else; a derelict or neutralized station admits whoever comes to loot it; a living station refuses
//! while it is on the Enemy side, while its civilization is one whose hull
//! the asking player neutralized within the refusal window
//! ([`NeutralizedCivilizations`]), or while it is hostile to the ship asking
//! (by side, or answering its fire). A ship holds one connection: the port search names no
//! pair for a docked ship. A station holds one per port: a held port is no
//! candidate, and the station branch here names the reason the HUD shows when
//! none is free.
//!
//! A dock already made is never judged again here: a ship under fire stays
//! docked, and the player hitting a docked Neutral is what forces the undock
//! (`input::ai::threat`).

use bevy::{ecs::system::SystemParam, prelude::*};
use nova_gameplay::prelude::*;

use super::{DockedPort, DockingSectionMarker};
use crate::{input::ai::prelude::AIBehaviorState, sections::hull_buy::ParkedHull};

/// Why a target refuses a dock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum DockRefusalType {
    /// A ship target's combat state: hostile to the ship asking, on the Enemy
    /// side, answering fire, or its AI fighting.
    Combat,
    /// A living station of an Enemy civilization.
    EnemyStation,
    /// A living station of a civilization whose hull the asking player
    /// neutralized within the refusal window.
    NeutralizedHull,
    /// A living station hostile to the ship asking: their sides are hostile,
    /// or one answers the other's fire.
    HostileStation,
    /// A station whose every live port is already held.
    StationOccupied,
}

/// The station branch of admission: what a station on `allegiance`'s side
/// answers, dead or alive, owned or not, with or without a free port, to a
/// ship that is `hostile` to it ([`ship_relation`]) and that, when
/// `neutralized_hull`, is the player within the refusal window of the
/// station's civilization. `dead` is a derelict or a neutralized station.
///
/// # Errors
///
/// [`DockRefusalType::StationOccupied`] while every port is held (`occupied`),
/// whatever the station is. An `owned` station then admits: it refuses nobody. A `dead`
/// station admits on any side: it is a hull to loot, and a `None` side is
/// hostile to no one. A living station refuses with [`DockRefusalType::EnemyStation`]
/// on the Enemy side, then with [`DockRefusalType::NeutralizedHull`] while
/// the player is within its civilization's refusal window, then with
/// [`DockRefusalType::HostileStation`] while it and the ship are hostile.
pub fn station_admission(
    allegiance: Option<Allegiance>,
    dead: bool,
    occupied: bool,
    owned: bool,
    hostile: bool,
    neutralized_hull: bool,
) -> Result<(), DockRefusalType> {
    if occupied {
        return Err(DockRefusalType::StationOccupied);
    }
    if owned || dead {
        return Ok(());
    }
    if allegiance == Some(Allegiance::Enemy) {
        return Err(DockRefusalType::EnemyStation);
    }
    if neutralized_hull {
        return Err(DockRefusalType::NeutralizedHull);
    }
    if hostile {
        return Err(DockRefusalType::HostileStation);
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
            Has<OwnedStationMarker>,
            Has<PlayerSpaceshipMarker>,
            Option<&'static TraderMarket>,
        ),
        With<SpaceshipRootMarker>,
    >,
    /// The player's record of the civilizations whose hull the player
    /// neutralized: a living station of one refuses the player.
    neutralized: Res<'w, NeutralizedCivilizations>,
    /// The hulls parked at a station, which berth there whatever its side.
    parked: Query<'w, 's, &'static ParkedHull>,
    /// The free live ports, by the hull they are mounted on: a station with
    /// none is occupied.
    free_ports: Query<
        'w,
        's,
        &'static ChildOf,
        (
            With<DockingSectionMarker>,
            Without<DockedPort>,
            Without<SectionInactiveMarker>,
        ),
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
    /// A station answers by [`station_admission`], and to a hull parked at
    /// it as an owned station does; it refuses the player while its
    /// civilization is in the player's [`NeutralizedCivilizations`]. A neutralized ship always
    /// admits: the dock is a boarding. Otherwise a ship refuses while it is
    /// hostile to `ship`, while it is on the Enemy side, while it answers any
    /// ship's fire, and while its AI is fighting; a calm Player-aligned or
    /// Neutral ship lets it in. A target that is not a ship root has no
    /// state to refuse with.
    pub fn refusal(&self, ship: Entity, target: Entity) -> Option<DockRefusalType> {
        let Ok((allegiance, answering, state, neutralized, station, derelict, owned, _, market)) =
            self.ships.get(target)
        else {
            return None;
        };
        let (own_allegiance, own_answering, player) = self.ships.get(ship).map_or(
            (None, None, false),
            |(allegiance, answering, _, _, _, _, _, player, _)| (allegiance, answering, player),
        );
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
        if station {
            let occupied = !self.free_ports.iter().any(|mount| mount.0 == target);
            let berth = self.parked.get(ship).is_ok_and(|parked| parked.0 == target);
            let neutralized_hull = player
                && market.is_some_and(|market| self.neutralized.refuses(&market.civilization));
            return station_admission(
                allegiance.copied(),
                derelict || neutralized,
                occupied,
                owned || berth,
                hostile,
                neutralized_hull,
            )
            .err();
        }
        if neutralized {
            return None;
        }
        let enemy = allegiance == Some(&Allegiance::Enemy);
        let answers = answering.is_some_and(|answering| answering.is_some());
        let fighting = state.is_some_and(AIBehaviorState::engages);
        (hostile || enemy || answers || fighting).then_some(DockRefusalType::Combat)
    }
}
