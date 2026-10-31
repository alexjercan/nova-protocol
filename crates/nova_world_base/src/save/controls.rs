//! The player's flight controls a save keeps: the autopilot order, the
//! travel, combat and component locks and the dock. The RCS magazine is the
//! player ship's own [`FrozenShip`](nova_scenario::prelude::FrozenShip)
//! record, not a control.
//!
//! Every reference names its body by a durable key, never by `Entity`: a body
//! by its [`EntityId`], a canister by its runtime id, a saved transient by its
//! index in the save's transient list, a section by its id on its ship.
//!
//! A save writes no control without its target. A live target with no
//! durable key fails the save. A handle whose body is gone, or whose body is
//! in a cell that is leaving the window, holds the save until its owner
//! clears the handle, within the save's settle bound.
//!
//! A Load restores the controls on the frame its window and transients come
//! back, after the sensor pass and before the first resumed fixed tick. A
//! saved reference with no live match holds the Load like a transient's does
//! and refuses it at the same bound. A dock whose port is taken refuses it at
//! once. Nothing is dropped. The restored dock is the saved connection with
//! its saved ports, helm and joint frames, and it fires no `OnDocked`.

use std::collections::HashMap;

use avian3d::prelude::{FixedJoint, JointAnchor, JointBasis, JointFrame, RigidBody};
use bevy::prelude::*;
use nova_events::prelude::{EntityId, GeneratedBodyId};
use nova_gameplay::prelude::{
    CargoCanisterRuntimeId, PlayerSpaceshipMarker, SavedBodyRef, SavedTargetRef,
    SectionInactiveMarker, SpaceshipRootMarker, StationMarker, UnsettledBody,
};
use nova_scenario::prelude::echo_restored_dock;
use nova_ship::prelude::{
    restore_docking_connection, Autopilot, AutopilotAction, AutopilotPhase, CombatLock,
    ComponentLock, ComponentLockMode, DockedHelmType, DockedPort, DockedShip, DockingConnection,
    DockingSectionMarker, LockFocus, OrbitPlan, SensorContacts, TravelLock,
};
use nova_world::prelude::{
    desired_sectors, CurrentSector, SectorRoot, SectorSnapshotError, WorldConfig,
};
use serde::{Deserialize, Serialize};

use super::transients::{live_by_id, Resolve, TransientRefFault};
use crate::NovaLayeredWorld;

/// The player's flight controls as saved.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedControls {
    /// The engaged autopilot order; `None` flies by hand.
    pub autopilot: Option<SavedAutopilot>,
    /// The travel lock; `None` designates nothing.
    pub travel_lock: Option<SavedLock>,
    /// The combat lock; `None` locks nothing.
    pub combat_lock: Option<SavedLock>,
    /// Seconds the focus dwell has held on the combat lock's target.
    pub focus_seconds: f32,
    /// The component fine-lock on the combat-locked ship.
    pub component_lock: SavedComponentLock,
    /// The dock the player ship is in; `None` floats free.
    pub dock: Option<SavedDock>,
}

/// A saved lock slot: the target and, while the sensor pass holds it, the
/// range the target returned when the hold began, engine units.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedLock {
    /// The locked body.
    pub target: SavedTargetRef,
    /// The held range floor; `None` before a sensor pass held it.
    pub floor: Option<f32>,
}

/// The saved component fine-lock.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedComponentLock {
    /// The fine-locked section of the combat-locked ship, by its id on that
    /// ship; `None` selects nothing.
    pub section: Option<EntityId>,
    /// Seconds a cycle press still pins the selection; `None` snaps.
    pub pinned_seconds: Option<f32>,
}

/// The saved autopilot order.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedAutopilot {
    /// What the computer does.
    pub action: SavedAutopilotAction,
    /// Where the maneuver is.
    pub phase: AutopilotPhase,
    /// Whether the arrival leg's flip is behind the ship.
    pub flip_marker_consumed: bool,
}

/// A saved [`AutopilotAction`], its bodies by durable key. Engine units.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SavedAutopilotAction {
    /// Kill all velocity.
    Stop,
    /// Fly to `target`.
    Goto {
        /// The destination body.
        target: SavedTargetRef,
    },
    /// Fly to a fixed world position.
    GotoPos {
        /// The destination.
        position: Vec3,
    },
    /// Circularize and hold an orbit of `well`.
    Orbit {
        /// The orbited well.
        well: SavedBodyRef,
        /// The insertion plan; `None` before the first engaged tick.
        plan: Option<OrbitPlan>,
    },
    /// Hold a world-space velocity.
    MatchVelocity {
        /// The velocity.
        velocity: Vec3,
        /// The unit direction the nose is asked to point.
        facing: Option<Vec3>,
        /// Whether the RCS may reach the velocity.
        rcs: bool,
    },
}

/// The saved dock of the player ship: the connection `DOCK` built, by its
/// partner's id and the two ports' section ids, and the fixed joint's local
/// frames.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedDock {
    /// The hull the player ship is docked with.
    pub partner: SavedBodyRef,
    /// Whether the player ship is the connection's first ship and the
    /// joint's first body.
    pub player_first: bool,
    /// The player ship's port.
    pub player_port: EntityId,
    /// The partner's port.
    pub partner_port: EntityId,
    /// Who flies the pair.
    pub helm: SavedDockHelm,
    /// The joint frame on the player ship, in its body frame.
    pub player_frame: SavedJointFrame,
    /// The joint frame on the partner, in its body frame.
    pub partner_frame: SavedJointFrame,
}

/// A saved [`DockedHelmType`]. Only the player ship can hold a helm, and
/// only a station partner can own one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SavedDockHelm {
    /// Neither ship flies the pair.
    Neutral,
    /// The player ship flies the pair.
    Player,
    /// The partner, a living station, owns the pair.
    Station,
}

/// One body's local joint frame. Engine units.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedJointFrame {
    /// The anchor.
    pub anchor: Vec3,
    /// The basis.
    pub basis: Quat,
}

impl SavedJointFrame {
    fn of(frame: &JointFrame) -> Self {
        let (JointAnchor::Local(anchor), JointBasis::Local(basis)) = (frame.anchor, frame.basis)
        else {
            panic!("a docking joint is built with local frames, but a frame is global");
        };
        Self { anchor, basis }
    }

    fn frame(self) -> JointFrame {
        JointFrame {
            anchor: JointAnchor::Local(self.anchor),
            basis: JointBasis::Local(self.basis),
        }
    }
}

/// What the save keeps that a control reference can name.
pub(crate) struct SavedReferents<'a> {
    /// Whether a body with this id is saved.
    pub body: &'a dyn Fn(&str) -> bool,
    /// Whether a canister with this id is saved.
    pub canister: &'a dyn Fn(CargoCanisterRuntimeId) -> bool,
    /// Whether the saved ship or station with this id holds this section;
    /// `None` when no ship or station has the id.
    pub section: &'a dyn Fn(&str, &str) -> Option<bool>,
    /// How many transients the save keeps.
    pub transients: usize,
}

impl SavedControls {
    /// # Errors
    ///
    /// A number no session wrote: a negative or non-finite focus or floor, a
    /// pin that is not finite, a mark or velocity that is not finite, a
    /// facing, orbit normal or joint basis that is not a unit.
    /// A reference to a body, canister, transient or section the save does
    /// not keep. A body the save does not keep is good only when its id is
    /// one the generator mints, as a regenerated planet's is. A component
    /// lock on no combat-locked ship, a dock on a partner that is not a saved
    /// ship or station, or a station helm on a partner that is no station.
    pub(crate) fn validate(&self, player: &str, saved: &SavedReferents<'_>) -> Result<(), String> {
        let reading = |value: f32, what: &str| {
            if value.is_finite() && value >= 0.0 {
                Ok(())
            } else {
                Err(format!("the player's {what} reads {value}"))
            }
        };
        reading(self.focus_seconds, "focus dwell")?;
        if let Some(pinned) = self
            .component_lock
            .pinned_seconds
            .filter(|pinned| !pinned.is_finite())
        {
            return Err(format!("the player's component pin reads {pinned}"));
        }
        for lock in [&self.travel_lock, &self.combat_lock].into_iter().flatten() {
            if let Some(floor) = lock.floor {
                reading(floor, "lock floor")?;
            }
        }
        let unit = |value: Vec3| value.is_finite() && value.is_normalized();
        let body = |what: &str, id: &EntityId| {
            if (saved.body)(&id.0) || id.0.parse::<GeneratedBodyId>().is_ok() {
                Ok(())
            } else {
                Err(format!("{what} '{}' is not saved", id.0))
            }
        };
        let target = |what: &str, target: &SavedTargetRef| match target {
            SavedTargetRef::Body(SavedBodyRef(id)) => body(what, id),
            SavedTargetRef::Canister(id) if (saved.canister)(*id) => Ok(()),
            SavedTargetRef::Canister(id) => Err(format!("{what} canister {} is not saved", id.0)),
            SavedTargetRef::Transient(index) if *index < saved.transients => Ok(()),
            SavedTargetRef::Transient(index) => Err(format!(
                "{what} transient {index} is not one of the {} saved",
                saved.transients
            )),
        };
        match self.autopilot.as_ref().map(|autopilot| &autopilot.action) {
            None | Some(SavedAutopilotAction::Stop) => {}
            Some(SavedAutopilotAction::Goto { target: goal }) => {
                target("the autopilot's GOTO target", goal)?;
            }
            Some(SavedAutopilotAction::GotoPos { position }) => {
                if !position.is_finite() {
                    return Err("the autopilot's GOTO mark is not finite".to_string());
                }
            }
            Some(SavedAutopilotAction::Orbit { well, plan }) => {
                body("the autopilot's ORBIT well", &well.0)?;
                if plan.is_some_and(|plan| {
                    !(plan.radius.is_finite() && plan.radius > 0.0 && unit(plan.normal))
                }) {
                    return Err("the autopilot's orbit plan has no ring".to_string());
                }
            }
            Some(SavedAutopilotAction::MatchVelocity {
                velocity, facing, ..
            }) => {
                if !velocity.is_finite() || facing.is_some_and(|facing| !unit(facing)) {
                    return Err("the autopilot's held velocity or facing is not finite".to_string());
                }
            }
        }
        if let Some(lock) = &self.travel_lock {
            target("the travel lock's", &lock.target)?;
        }
        if let Some(lock) = &self.combat_lock {
            target("the combat lock's", &lock.target)?;
        }
        if let Some(section) = &self.component_lock.section {
            let Some(SavedLock {
                target: SavedTargetRef::Body(SavedBodyRef(ship)),
                ..
            }) = &self.combat_lock
            else {
                return Err(format!(
                    "the component lock on '{}' has no combat-locked ship",
                    section.0
                ));
            };
            if (saved.section)(&ship.0, &section.0) != Some(true) {
                return Err(format!(
                    "the component lock's section '{}' of '{}' is not saved",
                    section.0, ship.0
                ));
            }
        }
        if let Some(dock) = &self.dock {
            let partner = &dock.partner.0 .0;
            for frame in [dock.player_frame, dock.partner_frame] {
                if !(frame.anchor.is_finite()
                    && frame.basis.is_finite()
                    && frame.basis.is_normalized())
                {
                    return Err(format!(
                        "the dock with '{partner}' has a joint frame no dock built"
                    ));
                }
            }
            if (saved.section)(player, &dock.player_port.0) != Some(true) {
                return Err(format!(
                    "the player ship's dock port '{}' is not saved",
                    dock.player_port.0
                ));
            }
            match (saved.section)(partner, &dock.partner_port.0) {
                Some(true) => {}
                Some(false) => {
                    return Err(format!(
                        "the dock port '{}' of '{partner}' is not saved",
                        dock.partner_port.0
                    ));
                }
                None => return Err(format!("the dock partner '{partner}' is not saved")),
            }
        }
        Ok(())
    }
}

/// The player ship's controls as they stand, frozen.
///
/// `transients` maps each saved transient body to its index in the save's
/// transient list.
///
/// # Errors
///
/// [`SectorSnapshotError::NoDurableId`] for a live target with no durable
/// key. [`SectorSnapshotError::Unsettled`] for a handle whose body is gone or
/// is in a cell that is leaving the window: its owner clears it on a later
/// frame.
///
/// # Panics
///
/// When the player ship's dock breaks the connection's own rules: a helm
/// another ship holds, a station helm the partner does not own, a port with
/// no id.
pub(crate) fn freeze_controls(
    world: &World,
    player: Entity,
    transients: &HashMap<Entity, usize>,
) -> Result<SavedControls, SectorSnapshotError> {
    let coord = world.resource::<CurrentSector>().0;
    let radius = world
        .resource::<WorldConfig<NovaLayeredWorld>>()
        .active_radius;
    let desired = desired_sectors(coord, radius);
    let unsettled = |what: &str, reason: &'static str| SectorSnapshotError::Unsettled {
        label: what.to_string(),
        coord,
        reason: UnsettledBody { reason },
    };
    let target = |what: &str, entity: Entity| -> Result<SavedTargetRef, SectorSnapshotError> {
        let Ok(body) = world.get_entity(entity) else {
            return Err(unsettled(what, "its target is gone"));
        };
        // A Load streams only the window back, so a target in a cell that
        // leaves it would never resolve.
        let cell = body
            .get::<ChildOf>()
            .and_then(|parent| world.get::<SectorRoot>(parent.parent()));
        if cell.is_some_and(|cell| !desired.contains(&cell.0)) {
            return Err(unsettled(what, "its target is leaving the window"));
        }
        if let Some(id) = body.get::<EntityId>() {
            Ok(SavedTargetRef::Body(SavedBodyRef(id.clone())))
        } else if let Some(&id) = body.get::<CargoCanisterRuntimeId>() {
            Ok(SavedTargetRef::Canister(id))
        } else if let Some(&index) = transients.get(&entity) {
            Ok(SavedTargetRef::Transient(index))
        } else {
            Err(SectorSnapshotError::NoDurableId {
                label: format!(
                    "{what}: '{}'",
                    body.get::<Name>()
                        .map_or_else(|| entity.to_string(), ToString::to_string)
                ),
            })
        }
    };
    let body = |what: &str, entity: Entity| match target(what, entity)? {
        SavedTargetRef::Body(body) => Ok(body),
        SavedTargetRef::Canister(_) | SavedTargetRef::Transient(_) => {
            Err(SectorSnapshotError::NoDurableId {
                label: format!("{what}: {entity} has no id"),
            })
        }
    };
    let root = world.entity(player);
    let contacts = root.get::<SensorContacts>();
    let lock = |what: &str, entity: Entity| -> Result<SavedLock, SectorSnapshotError> {
        Ok(SavedLock {
            target: target(what, entity)?,
            floor: contacts.and_then(|contacts| contacts.held_floor(entity)),
        })
    };

    let autopilot = root
        .get::<Autopilot>()
        .map(|autopilot| {
            let action = match autopilot.action {
                AutopilotAction::Stop => SavedAutopilotAction::Stop,
                AutopilotAction::Goto { target: goal } => SavedAutopilotAction::Goto {
                    target: target("the autopilot's GOTO target", goal)?,
                },
                AutopilotAction::GotoPos { position } => SavedAutopilotAction::GotoPos { position },
                AutopilotAction::Orbit { well, plan } => SavedAutopilotAction::Orbit {
                    well: body("the autopilot's ORBIT well", well)?,
                    plan,
                },
                AutopilotAction::MatchVelocity {
                    velocity,
                    facing,
                    rcs,
                } => SavedAutopilotAction::MatchVelocity {
                    velocity,
                    facing: facing.map(Vec3::from),
                    rcs,
                },
            };
            Ok(SavedAutopilot {
                action,
                phase: autopilot.phase,
                flip_marker_consumed: autopilot.flip_marker_consumed,
            })
        })
        .transpose()?;
    let combat = root.get::<CombatLock>().and_then(|lock| lock.0);
    let travel_lock = root
        .get::<TravelLock>()
        .and_then(|lock| lock.0)
        .map(|entity| lock("the travel lock", entity))
        .transpose()?;
    let combat_lock = combat
        .map(|entity| lock("the combat lock", entity))
        .transpose()?;
    // The dwell counts on the combat lock's target; a dwell on another
    // restarts from zero on the next tick anyway.
    let focus_seconds = root
        .get::<LockFocus>()
        .filter(|focus| focus.target.is_some() && focus.target == combat)
        .map_or(0.0, |focus| focus.seconds);
    let component = root.get::<ComponentLock>().cloned().unwrap_or_default();
    let component_lock = SavedComponentLock {
        section: component
            .section
            .map(|section| {
                let what = "the component lock";
                world
                    .get_entity(section)
                    .map_err(|_| unsettled(what, "its section is gone"))?
                    .get::<EntityId>()
                    .cloned()
                    .ok_or_else(|| SectorSnapshotError::NoDurableId {
                        label: format!("{what}: section {section}"),
                    })
            })
            .transpose()?,
        pinned_seconds: match component.mode {
            ComponentLockMode::Snap => None,
            ComponentLockMode::Pinned { until } => {
                Some(until - world.resource::<Time>().elapsed_secs())
            }
        },
    };
    let dock = root
        .get::<DockedShip>()
        .map(|docked| {
            freeze_dock(world, player, docked.connection, |partner| {
                body("the dock partner", partner)
            })
        })
        .transpose()?;
    Ok(SavedControls {
        autopilot,
        travel_lock,
        combat_lock,
        focus_seconds,
        component_lock,
        dock,
    })
}

/// The player ship's dock through `connection`, frozen. `partner_ref` keys
/// the partner.
fn freeze_dock(
    world: &World,
    player: Entity,
    connection: Entity,
    partner_ref: impl FnOnce(Entity) -> Result<SavedBodyRef, SectorSnapshotError>,
) -> Result<SavedDock, SectorSnapshotError> {
    let record = world
        .get::<DockingConnection>(connection)
        .expect("a docked ship names its live connection");
    let joint = world
        .get::<FixedJoint>(connection)
        .expect("a docking connection holds its fixed joint");
    let player_first = record.first_ship == player;
    let (partner, player_port, partner_port, player_frame, partner_frame) = if player_first {
        let frames = (&joint.frame1, &joint.frame2);
        (
            record.second_ship,
            record.first_section,
            record.second_section,
            frames.0,
            frames.1,
        )
    } else {
        let frames = (&joint.frame2, &joint.frame1);
        (
            record.first_ship,
            record.second_section,
            record.first_section,
            frames.0,
            frames.1,
        )
    };
    let helm = match record.helm {
        DockedHelmType::Neutral => SavedDockHelm::Neutral,
        DockedHelmType::Held(ship) => {
            assert_eq!(ship, player, "only the player ship holds a dock's helm");
            SavedDockHelm::Player
        }
        DockedHelmType::Station(station) => {
            assert_eq!(station, partner, "a station helm is the partner's");
            SavedDockHelm::Station
        }
    };
    let port = |section: Entity| {
        world
            .get::<EntityId>(section)
            .cloned()
            .unwrap_or_else(|| panic!("docking port {section} has no section id"))
    };
    Ok(SavedDock {
        partner: partner_ref(partner)?,
        player_first,
        player_port: port(player_port),
        partner_port: port(partner_port),
        helm,
        player_frame: SavedJointFrame::of(player_frame),
        partner_frame: SavedJointFrame::of(partner_frame),
    })
}

/// A saved reference resolved against the live window: a live entity, or
/// the index of a saved transient the restore spawns on the same frame.
#[derive(Clone, Copy, Debug)]
enum LiveRef {
    Entity(Entity),
    Transient(usize),
}

impl LiveRef {
    fn entity(self, spawned: &[Entity]) -> Entity {
        match self {
            Self::Entity(entity) => entity,
            Self::Transient(index) => spawned[index],
        }
    }
}

/// The saved controls resolved against the live window, ready to apply.
pub(crate) struct ResolvedControls {
    player: Entity,
    controls: SavedControls,
    /// The GOTO target or ORBIT well.
    goal: Option<LiveRef>,
    travel: Option<LiveRef>,
    combat: Option<LiveRef>,
    component: Option<Entity>,
    dock: Option<(DockingConnection, [JointFrame; 2])>,
}

/// Collects the references that wait and the first that refuses.
#[derive(Default)]
struct Faults {
    waiting: Vec<String>,
    refused: Option<String>,
}

impl Faults {
    fn wait(&mut self, reason: String) {
        self.waiting.push(reason);
    }

    fn refuse(&mut self, reason: String) {
        self.refused.get_or_insert(reason);
    }

    fn by_id(
        &mut self,
        what: &str,
        id: &EntityId,
        found: Result<Entity, TransientRefFault>,
    ) -> Option<Entity> {
        match found {
            Ok(entity) => Some(entity),
            Err(TransientRefFault::Missing) => {
                self.wait(format!("{what} '{}' is not in the window", id.0));
                None
            }
            Err(TransientRefFault::Ambiguous(count)) => {
                self.refuse(format!(
                    "{count} live bodies have the id '{}' of {what}",
                    id.0
                ));
                None
            }
        }
    }
}

/// The one live section with the id `id` on `ship`.
fn live_section(world: &mut World, ship: Entity, id: &EntityId) -> Result<Entity, String> {
    let matches: Vec<Entity> = world
        .query::<(Entity, &EntityId, &ChildOf)>()
        .iter(world)
        .filter(|(_, live, parent)| live.0 == id.0 && parent.parent() == ship)
        .map(|(entity, ..)| entity)
        .collect();
    match matches.as_slice() {
        [section] => Ok(*section),
        [] => Err(format!("the section '{}' is not live", id.0)),
        many => Err(format!(
            "{} live sections have the id '{}'",
            many.len(),
            id.0
        )),
    }
}

/// Resolve the saved `controls` of a Load against the live window.
///
/// # Errors
///
/// [`Resolve::Wait`] while the player ship is not spawned, or with every
/// body that is not in the window yet.
/// [`Resolve::Refuse`] for a reference that can never resolve: an ambiguous
/// id, a component lock on a section that is not live, a dock whose port is
/// taken or is no live docking port, a player ship or ship partner that is
/// docked already.
pub(crate) fn resolve_controls(
    world: &mut World,
    controls: &SavedControls,
) -> Result<ResolvedControls, Resolve> {
    let players: Vec<Entity> = world
        .query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()
        .iter(world)
        .collect();
    let player = match players[..] {
        [player] => player,
        // The scenario's player spawn thaws the saved ship; it may not have
        // run yet.
        [] => {
            return Err(Resolve::Wait(vec![
                "the player ship is not spawned yet".to_string()
            ]));
        }
        _ => {
            return Err(Resolve::Refuse(format!(
                "{} player ships stand in the window",
                players.len()
            )));
        }
    };
    let mut faults = Faults::default();
    let resolve = |world: &mut World, faults: &mut Faults, what: &str, target: &SavedTargetRef| {
        match target {
            SavedTargetRef::Body(SavedBodyRef(id)) => faults
                .by_id(what, id, live_by_id::<With<RigidBody>>(world, id))
                .map(LiveRef::Entity),
            SavedTargetRef::Canister(id) => {
                let canisters: Vec<Entity> = world
                    .query::<(Entity, &CargoCanisterRuntimeId)>()
                    .iter(world)
                    .filter(|(_, live)| *live == id)
                    .map(|(entity, _)| entity)
                    .collect();
                match canisters.as_slice() {
                    [canister] => Some(LiveRef::Entity(*canister)),
                    [] => {
                        faults.wait(format!("{what} canister {} is not in the window", id.0));
                        None
                    }
                    many => {
                        faults.refuse(format!(
                            "{} live canisters have the runtime id {} of {what}",
                            many.len(),
                            id.0
                        ));
                        None
                    }
                }
            }
            SavedTargetRef::Transient(index) => Some(LiveRef::Transient(*index)),
        }
    };
    let goal = match controls
        .autopilot
        .as_ref()
        .map(|autopilot| &autopilot.action)
    {
        Some(SavedAutopilotAction::Goto { target }) => {
            resolve(world, &mut faults, "the autopilot's GOTO target", target)
        }
        Some(SavedAutopilotAction::Orbit { well, .. }) => {
            let found = live_by_id::<With<RigidBody>>(world, &well.0);
            faults
                .by_id("the autopilot's ORBIT well", &well.0, found)
                .map(LiveRef::Entity)
        }
        _ => None,
    };
    let travel = controls
        .travel_lock
        .as_ref()
        .and_then(|lock| resolve(world, &mut faults, "the travel lock's target", &lock.target));
    let combat = controls
        .combat_lock
        .as_ref()
        .and_then(|lock| resolve(world, &mut faults, "the combat lock's target", &lock.target));
    let component = match (&controls.component_lock.section, combat) {
        (Some(section), Some(LiveRef::Entity(ship))) => live_section(world, ship, section)
            .map_err(|reason| faults.refuse(format!("the component lock: {reason}")))
            .ok(),
        _ => None,
    };
    let dock = controls.dock.as_ref().and_then(|dock| {
        let id = &dock.partner.0;
        let found = live_by_id::<With<SpaceshipRootMarker>>(world, id);
        let partner = faults.by_id("the dock partner", id, found)?;
        restorable_dock(world, player, partner, dock)
            .map_err(|reason| faults.refuse(reason))
            .ok()
    });
    if let Some(reason) = faults.refused {
        return Err(Resolve::Refuse(reason));
    }
    if !faults.waiting.is_empty() {
        return Err(Resolve::Wait(faults.waiting));
    }
    Ok(ResolvedControls {
        player,
        controls: controls.clone(),
        goal,
        travel,
        combat,
        component,
        dock,
    })
}

/// The connection `dock` rebuilds between `player` and `partner`.
///
/// # Errors
///
/// A port that is not live, is no live docking port or is taken, a player
/// ship or ship partner that is docked already, a station helm on a partner
/// that is no station.
fn restorable_dock(
    world: &mut World,
    player: Entity,
    partner: Entity,
    dock: &SavedDock,
) -> Result<(DockingConnection, [JointFrame; 2]), String> {
    let name = &dock.partner.0 .0;
    let station = world.entity(partner).contains::<StationMarker>();
    if world.entity(player).contains::<DockedShip>() {
        return Err("the player ship is docked already".to_string());
    }
    if !station && world.entity(partner).contains::<DockedShip>() {
        return Err(format!("the dock partner '{name}' is docked already"));
    }
    let mut port = |ship: Entity, id: &EntityId, owner: &str| {
        let section = live_section(world, ship, id)
            .map_err(|reason| format!("the dock port of {owner}: {reason}"))?;
        let live = world.entity(section);
        if !live.contains::<DockingSectionMarker>() || live.contains::<SectionInactiveMarker>() {
            return Err(format!(
                "the section '{}' of {owner} is no live docking port",
                id.0
            ));
        }
        if live.contains::<DockedPort>() {
            return Err(format!("the dock port '{}' of {owner} is taken", id.0));
        }
        Ok(section)
    };
    let player_port = port(player, &dock.player_port, "the player ship")?;
    let partner_port = port(partner, &dock.partner_port, &format!("'{name}'"))?;
    let helm = match dock.helm {
        SavedDockHelm::Neutral => DockedHelmType::Neutral,
        SavedDockHelm::Player => DockedHelmType::Held(player),
        SavedDockHelm::Station if station => DockedHelmType::Station(partner),
        SavedDockHelm::Station => {
            return Err(format!(
                "the dock partner '{name}' owns the helm and is no station"
            ));
        }
    };
    let ends = [
        (player, player_port, dock.player_frame.frame()),
        (partner, partner_port, dock.partner_frame.frame()),
    ];
    let [first, second] = if dock.player_first {
        ends
    } else {
        [ends[1], ends[0]]
    };
    Ok((
        DockingConnection {
            first_ship: first.0,
            first_section: first.1,
            second_ship: second.0,
            second_section: second.1,
            helm,
            measurement_fault: false,
        },
        [first.2, second.2],
    ))
}

/// Write the resolved controls onto the player ship: the dock, the
/// autopilot, the locks with their floors and focus and the component lock.
/// `spawned` are the saved transients this frame spawned, in save order.
///
/// # Panics
///
/// When the player ship lacks the sensor contacts every player ship root
/// carries.
pub(crate) fn apply_controls(world: &mut World, resolved: ResolvedControls, spawned: &[Entity]) {
    let ResolvedControls {
        player,
        controls,
        goal,
        travel,
        combat,
        component,
        dock,
    } = resolved;
    if let Some((record, frames)) = dock {
        let connection = restore_docking_connection(world, record, frames);
        echo_restored_dock(world, connection);
    }
    let goal = goal.map(|goal| goal.entity(spawned));
    let travel = travel.map(|target| target.entity(spawned));
    let combat = combat.map(|target| target.entity(spawned));
    let now = world.resource::<Time>().elapsed_secs();
    let mut root = world.entity_mut(player);
    if let Some(saved) = &controls.autopilot {
        let resolved = "resolve_controls resolves every autopilot body";
        let action = match &saved.action {
            SavedAutopilotAction::Stop => AutopilotAction::Stop,
            SavedAutopilotAction::Goto { .. } => AutopilotAction::Goto {
                target: goal.expect(resolved),
            },
            SavedAutopilotAction::GotoPos { position } => AutopilotAction::GotoPos {
                position: *position,
            },
            SavedAutopilotAction::Orbit { plan, .. } => AutopilotAction::Orbit {
                well: goal.expect(resolved),
                plan: *plan,
            },
            SavedAutopilotAction::MatchVelocity {
                velocity,
                facing,
                rcs,
            } => AutopilotAction::MatchVelocity {
                velocity: *velocity,
                facing: facing.map(|facing| Dir3::new(facing).expect("a saved facing is a unit")),
                rcs: *rcs,
            },
        };
        root.insert(Autopilot {
            action,
            phase: saved.phase,
            flip_marker_consumed: saved.flip_marker_consumed,
        });
    }
    let mut contacts = root
        .get_mut::<SensorContacts>()
        .expect("a player ship root carries its sensor contacts");
    for (target, lock) in [
        (travel, &controls.travel_lock),
        (combat, &controls.combat_lock),
    ] {
        if let (Some(target), Some(floor)) = (target, lock.as_ref().and_then(|lock| lock.floor)) {
            contacts.hold_floor(target, floor);
        }
    }
    root.insert((
        TravelLock(travel),
        CombatLock(combat),
        LockFocus {
            target: combat,
            seconds: controls.focus_seconds,
        },
        ComponentLock {
            section: component,
            mode: controls
                .component_lock
                .pinned_seconds
                .map_or(ComponentLockMode::Snap, |left| ComponentLockMode::Pinned {
                    until: now + left,
                }),
        },
    ));
    info!("nova_world_base: the player's saved controls are back");
}
