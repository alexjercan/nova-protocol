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
//! else; a derelict or neutralized station admits whoever comes to loot it; a
//! living station refuses while it is on the Enemy side (a civilization's
//! stance to the player, or a provoked one, decides that side), or while it
//! is hostile to the ship asking (by side, or answering its fire); a living
//! lawful station ([`LawfulStationMarker`]) then refuses a ship whose hold
//! carries an item of category [`ItemCategoryType::Contraband`], until the
//! item leaves the hold. A ship
//! holds one connection: the port search names no pair for a docked ship. A
//! station holds one per port: a held port is no candidate, and the station
//! branch here names the reason the HUD shows when none is free.
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
    /// A living station hostile to the ship asking: their sides are hostile,
    /// or one answers the other's fire.
    HostileStation,
    /// A station whose every live port is already held.
    StationOccupied,
    /// A living lawful station, asked by a ship that carries contraband.
    Contraband,
}

/// A deliberate `DOCK` press did nothing, and why. Only the press writes it:
/// the HUD posts it as a notice, and the request stays the only judge of the
/// dock itself.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockPressRefusal(pub DockPressRefusalType);

/// Why a `DOCK` press did nothing. A target that refuses ([`DockRefusalType`])
/// is not here: a station says so at the lock, a ship by its dark chip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockPressRefusalType {
    /// No travel lock.
    NoLock,
    /// No pair of free live ports between the two hulls, which includes a
    /// target ship that already holds its one dock.
    NoFreePort,
    /// The nearest pair fails [`DockingPair::gap_holds`](super::DockingPair::gap_holds).
    TooFar,
    /// The nearest pair fails [`DockingPair::facing_holds`](super::DockingPair::facing_holds).
    NotFacing,
    /// The nearest pair fails [`DockingPair::motion_holds`](super::DockingPair::motion_holds),
    /// by relative speed or relative spin.
    TooFast,
}

/// The station branch of admission: what a station on `allegiance`'s side
/// answers, dead or alive, owned or not, with or without a free port, to a
/// ship that is `hostile` to it ([`ship_relation`]). `dead` is a derelict or
/// a neutralized station. `contraband` is a lawful station asked by a ship
/// that carries a contraband item.
///
/// # Errors
///
/// [`DockRefusalType::StationOccupied`] while every port is held (`occupied`),
/// whatever the station is. An `owned` station then admits: it refuses nobody. A `dead`
/// station admits on any side: it is a hull to loot, and a `None` side is
/// hostile to no one. A living station refuses with [`DockRefusalType::EnemyStation`]
/// on the Enemy side, then with [`DockRefusalType::HostileStation`] while it
/// and the ship are hostile, then with [`DockRefusalType::Contraband`] on
/// `contraband`.
pub fn station_admission(
    allegiance: Option<Allegiance>,
    dead: bool,
    occupied: bool,
    owned: bool,
    hostile: bool,
    contraband: bool,
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
    if hostile {
        return Err(DockRefusalType::HostileStation);
    }
    if contraband {
        return Err(DockRefusalType::Contraband);
    }
    Ok(())
}

/// The combat state of every ship a dock could name, the station facts of
/// every station, and the hold of the ship asking.
#[derive(SystemParam)]
pub struct DockAdmission<'w, 's> {
    /// The item catalog, which says which held items are contraband. `None`
    /// in an app with no catalog, where every hold is empty.
    items: Option<Res<'w, GameItems>>,
    /// The hold of every ship root.
    holds: Query<'w, 's, &'static ShipInventory, With<SpaceshipRootMarker>>,
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
            Has<LawfulStationMarker>,
        ),
        With<SpaceshipRootMarker>,
    >,
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
    /// it as an owned station does. A neutralized ship always admits: the
    /// dock is a boarding. Otherwise a ship refuses while it is hostile to
    /// `ship`, while it is on the Enemy side, while it answers any ship's
    /// fire, and while its AI is fighting; a calm Player-aligned or Neutral
    /// ship lets it in. A target that is not a ship root has no state to
    /// refuse with.
    pub fn refusal(&self, ship: Entity, target: Entity) -> Option<DockRefusalType> {
        self.judge(ship, target, true)
    }

    /// Whether station `target` turns `ship` away
    /// ([`DockRefusalType::EnemyStation`],
    /// [`DockRefusalType::HostileStation`] or
    /// [`DockRefusalType::Contraband`]), whatever its ports hold: waiting for
    /// a port changes none of these.
    /// [`refusal`](Self::refusal) answers [`DockRefusalType::StationOccupied`]
    /// first, so a full Enemy station would read as merely full there.
    pub fn turns_away(&self, ship: Entity, target: Entity) -> bool {
        match self.judge(ship, target, false) {
            Some(
                DockRefusalType::EnemyStation
                | DockRefusalType::HostileStation
                | DockRefusalType::Contraband,
            ) => true,
            Some(DockRefusalType::Combat | DockRefusalType::StationOccupied) | None => false,
        }
    }

    /// Whether the hold of `ship` carries an item of category
    /// [`ItemCategoryType::Contraband`]. A ship with no hold, or an empty
    /// one, carries none.
    ///
    /// # Panics
    ///
    /// When the hold carries an item and the app has no [`GameItems`], or
    /// the catalog lacks the item: nothing can fill a hold without the
    /// catalog, so the app is wired wrong.
    fn carries_contraband(&self, ship: Entity) -> bool {
        let Ok(hold) = self.holds.get(ship) else {
            return false;
        };
        hold.stacks().any(|(item, _)| {
            let items = self.items.as_ref().unwrap_or_else(|| {
                panic!("ship {ship:?} holds '{item}' and the app has no item catalog")
            });
            items.design(item).category == ItemCategoryType::Contraband
        })
    }

    /// [`refusal`](Self::refusal), with a station's held ports counted only
    /// when `count_occupied`.
    fn judge(&self, ship: Entity, target: Entity, count_occupied: bool) -> Option<DockRefusalType> {
        let Ok((allegiance, answering, state, neutralized, station, derelict, owned, lawful)) =
            self.ships.get(target)
        else {
            return None;
        };
        let (own_allegiance, own_answering) = self
            .ships
            .get(ship)
            .map_or((None, None), |(allegiance, answering, _, _, _, _, _, _)| {
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
        if station {
            let occupied = count_occupied && !self.free_ports.iter().any(|mount| mount.0 == target);
            let berth = self.parked.get(ship).is_ok_and(|parked| parked.0 == target);
            return station_admission(
                allegiance.copied(),
                derelict || neutralized,
                occupied,
                owned || berth,
                hostile,
                lawful && self.carries_contraband(ship),
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
