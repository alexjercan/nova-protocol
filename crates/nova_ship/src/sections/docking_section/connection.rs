//! The connection: what `DOCK` builds, what holds it, who flies it, and what
//! takes it away.
//!
//! One accepted pair becomes ONE entity carrying a
//! [`DockingConnection`] and the avian [`FixedJoint`] that does the actual
//! work. Releasing despawns that one entity - no query sweeps a joint it did
//! not create. The joint holds the pair rigid, so the two hulls do not
//! collide while docked ([`JointCollisionDisabled`]): a skin plate or a
//! greeble that reaches past a port face would otherwise press on the other
//! hull every step and wear both down while they sit still.
//!
//! The same plate still reaches into the other hull the step after the
//! release, so a contact there would shove both hulls apart. The release
//! therefore leaves an [`UndockSeparation`] that keeps the pair free of
//! collisions until no collider of one hull overlaps the other, or until
//! either hull's body leaves.
//!
//! # Ships hold one dock, a station is a hub
//!
//! A ship holds at most one connection ([`DockedShip`]). A station holds one
//! per port: each connection to it carries [`StationDock`], the station
//! lists them in [`StationDocks`] and carries [`DockedStation`] while it
//! holds any. The station and every hull docked at it are one assembly
//! ([`DockedAssembly`](super::DockedAssembly)), measured and flown as one
//! body. A dock between two stations is refused: a station is never the
//! ship that issues `DOCK`, and two hubs would join two assemblies.
//!
//! # One helm per pair
//!
//! The connection owns the pair's helm ([`DockedHelmType`]). A dock between
//! two ships starts NEUTRAL: the player's hull is held and the partner flies
//! the pair - its autopilot, its AI and its scripted order keep working.
//! `HELM` takes the pair ([`DockedHelmType::Held`]): the player flies it and
//! the partner is held instead, its maneuver and its order frozen rather than
//! retired. `HELM` again hands it back. Neither touches the joint; `DOCK`
//! pressed again is still what ends a dock, from either side.
//!
//! The partner decides the helm at capture ([`helm_on_capture`]) and the
//! joint is the same one. A LIVING station owns every connection to it
//! ([`DockedHelmType::Station`]): `HELM` is refused, every docked hull is
//! held, and the station's own flight computer drives the whole assembly. A
//! station has no thruster, so the assembly never translates; the computer's
//! attitude torque, spread over every root as the helm wrench, is all that
//! acts, and it holds the assembly level on the station's parked attitude. A
//! DEAD hull, ship or station ([`DerelictShipMarker`] or
//! [`NeutralizedMarker`]), hands the docked player the helm of the player's
//! own connection the instant the dock is made, as `HELM` would on a living
//! ship, because nobody is left at its helm; the player then flies the whole
//! assembly. Neutralization never inserts [`DerelictShipMarker`] on a ship,
//! so the helm rule reads both markers.
//!
//! Exactly one root drives an assembly ([`DockedShip::drives`],
//! [`DockedStation::drives`]) and every actuator skips the others. The
//! driver flies the ASSEMBLY rather than its own hull: see
//! [`DockedAssembly`](super::DockedAssembly).
//!
//! # A docked pair and the active window
//!
//! The player is always inside the active window, so the player's own dock
//! never meets a retiring cell. A pair of NPC hulls at retirement is a fault
//! (`nova_world::freeze_body` under `DockPolicy::Refuse`), and so is a station
//! hub with a hull docked.
//!
//! A world save keeps the player's dock and refuses every other one but a
//! station's parked hulls; a Load rebuilds the saved connection with
//! [`restore_docking_connection`].

use avian3d::{
    collision::contact_types::ContactId,
    data_structures::pair_key::PairKey,
    dynamics::solver::{
        constraint_graph::ConstraintGraph,
        islands::{BodyIslandNode, PhysicsIslands, WakeIslands},
        joint_graph::JointGraph,
    },
    prelude::*,
};
use bevy::{ecs::entity_disabling::Disabled, prelude::*};
use nova_gameplay::prelude::*;

use super::{DockAdmission, DockingPorts, DockingSectionMarker, DockingSectionState};
use crate::prelude::*;

/// Ordering handle for docking's own fixed-clock work.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DockingSystems {
    /// Takes a connection away when an endpoint is destroyed. Ahead of
    /// [`Assembly`](Self::Assembly), so a pair that just lost a hull is not
    /// measured or flown as one.
    Release,
    /// Measures every pair ([`DockedAssembly`](super::DockedAssembly)),
    /// decides its one driver ([`DockedShip::drives`]) and parks the helms of
    /// the held roots. Ahead of the controller stack, which tunes against the
    /// assembly, and so ahead of the flight layer and the PD.
    Assembly,
}

/// Who flies a docked pair.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum DockedHelmType {
    /// The player's hull is held; the partner flies the pair. Every dock
    /// between two ships starts here.
    #[default]
    Neutral,
    /// The player ship named here flies the pair; the partner is held.
    Held(Entity),
    /// The living station named here owns the pair: its flight computer is
    /// the driver, and `HELM` is refused. Set at capture and never toggled.
    Station(Entity),
}

/// What the hull on the other end of a dock is, for the helm rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockPartnerType {
    /// A living ship: the helm starts neutral and `HELM` toggles it.
    Ship,
    /// A station that is neither derelict nor neutralized: it owns the helm.
    LivingStation,
    /// A derelict or neutralized hull, ship or station: the docked player
    /// takes the helm at capture.
    Dead,
}

/// The helm a new connection opens with.
///
/// `player` is the player ship in the pair, if either endpoint is one.
/// `station` is read only for a living station, which holds its own pair. A
/// dead hull hands it to the player, or leaves it neutral when no player is
/// in the pair; two living ships start neutral.
pub fn helm_on_capture(
    partner: DockPartnerType,
    station: Entity,
    player: Option<Entity>,
) -> DockedHelmType {
    match (partner, player) {
        (DockPartnerType::LivingStation, _) => DockedHelmType::Station(station),
        (DockPartnerType::Dead, Some(player)) => DockedHelmType::Held(player),
        (DockPartnerType::Dead | DockPartnerType::Ship, _) => DockedHelmType::Neutral,
    }
}

/// Whether `HELM` may change `helm`: never while a living station holds it.
pub fn helm_request_admitted(helm: DockedHelmType) -> bool {
    !matches!(helm, DockedHelmType::Station(_))
}

/// A live docking connection, on its own entity beside the [`FixedJoint`].
///
/// The canonical record of one pair. `first` is the ship that issued the
/// command. Who is in charge is [`helm`](Self::helm), never the order of the
/// two ships.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct DockingConnection {
    /// The ship that issued `DOCK`.
    pub first_ship: Entity,
    /// Its reserved port.
    pub first_section: Entity,
    /// The ship it docked with.
    pub second_ship: Entity,
    /// Its reserved port.
    pub second_section: Entity,
    /// Who flies the pair. The only record of it: [`DockedShip::drives`] is
    /// derived from it every fixed tick.
    pub helm: DockedHelmType,
    /// Whether the last `measure_docked_assemblies` pass could not measure
    /// the pair, so neither root drives. False on a new connection; the next
    /// pass that measures the pair clears it. The HUD reads it as `HELM
    /// FAULT`, because `drives` alone cannot tell a fault from a held hull.
    /// While it is set the helm can be handed back but not taken.
    pub measurement_fault: bool,
}

impl DockingConnection {
    /// Whether `ship` is either root of this pair. Says nothing about who
    /// flies it: that is [`DockedShip::drives`].
    pub fn joins(&self, ship: Entity) -> bool {
        self.first_ship == ship || self.second_ship == ship
    }
}

/// On a port that is spoken for, naming the connection that holds it. Its
/// ABSENCE is what makes a port a candidate, which is the whole of the
/// one-connection-per-port rule.
#[derive(Component, Clone, Copy, Debug, Deref, Reflect)]
#[reflect(Component)]
pub struct DockedPort(pub Entity);

/// On a docked ship root, naming its one connection and whether this root is
/// the one that flies the assembly. A station holds [`DockedStation`]
/// instead.
///
/// Its presence takes the root's torque off the normal PD path (see
/// `sync_controller_section_forces`): the driver's attitude loop is spread
/// over every root by `apply_docked_helm_wrench`, and a held root's loop is
/// not applied at all. One connection per ship - a second `DOCK` onto an
/// already docked ship is refused; only a station is a hub.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct DockedShip {
    /// The connection holding this ship.
    pub connection: Entity,
    /// The attitude command a HELD root is parked on: its live attitude,
    /// every tick. It is what makes taking the helm, handing it back and
    /// undocking clean, because the command a root resumes is the direction
    /// it already points rather than the order it was flying when it was
    /// held.
    pub helm: Quat,
    /// Whether this root flies the pair this tick. True on exactly one root
    /// of a measured pair and on neither of an unmeasured one. Written only
    /// by `measure_docked_assemblies` from [`DockingConnection::helm`]; every
    /// actuator, maneuver and order driver skips a root where it is false.
    pub drives: bool,
}

/// On a connection to a station, naming the station hub that holds it.
///
/// A relationship: the station lists every connection it holds in
/// [`StationDocks`], kept by Bevy as connections spawn and despawn, so a
/// release is still the despawn of one connection entity.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
#[relationship(relationship_target = StationDocks)]
pub struct StationDock(pub Entity);

/// On a station, every connection docked at its ports, in the order they
/// were made. Absent while no hull is docked: Bevy removes it with the last
/// connection.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
#[relationship_target(relationship = StationDock)]
pub struct StationDocks(Vec<Entity>);

/// On a station holding at least one connection: the station side of
/// [`DockedShip`], with no connection of its own because it holds several
/// ([`StationDocks`]).
///
/// Its presence takes the station's torque off the normal PD path exactly as
/// [`DockedShip`] does for a ship.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct DockedStation {
    /// The attitude command the station is parked on while it is held: its
    /// live attitude, every tick, as [`DockedShip::helm`].
    pub helm: Quat,
    /// Whether the station flies its assembly this tick. Written only by
    /// `measure_docked_assemblies`.
    pub drives: bool,
}

/// On its own entity beside a slack [`DistanceJoint`] and
/// [`JointCollisionDisabled`]: a pair that was just released while its hulls
/// may still overlap. A skin plate or greeble that reached past a port face
/// would otherwise start a contact the step after the undock and shove both
/// hulls apart. [`end_undock_separations`] despawns it once no collider of
/// one hull overlaps the other, which gives the pair its collisions back.
/// [`on_separated_body_removed`] despawns it as either hull's body leaves.
///
/// The joint is what filters the pair: avian 0.7 filters a pair only through
/// a joint in its joint graph, and `JointDisabled` takes a joint out of the
/// graph. Its distance limits never bind, so it applies no impulse.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct UndockSeparation {
    /// One hull of the released pair.
    pub first_ship: Entity,
    /// The other hull.
    pub second_ship: Entity,
}

/// Asks for a connection between the ship that issued `DOCK` and its locked
/// target. The pair itself is chosen HERE, when the command executes, so the
/// offer the HUD made a frame ago can never be the pair that is built.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct DockingConnectionRequest {
    /// The ship issuing the command.
    pub entity: Entity,
    /// The ship it is docking with - the current lock.
    pub target: Entity,
}

/// Asks to take the helm of the pair `entity` is docked in, or to hand it back
/// when `entity` already holds it.
///
/// Only the player may hold a helm ([`DockedHelmType::Held`]): a request from
/// any other ship is refused. Taking is never negotiated with the partner,
/// and is refused while the pair has a
/// [`measurement_fault`](DockingConnection::measurement_fault), because the
/// player could not fly it, and while a living station holds the pair
/// ([`DockedHelmType::Station`]). Handing the helm back is never refused.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct DockingHelmRequest {
    /// The player ship asking.
    pub entity: Entity,
}

/// Asks for the connection holding `entity` to be let go.
///
/// Addressed to a SHIP, not to a connection, because that is what the pilot
/// who presses `DOCK` again knows about. Either end of a ship pair may ask,
/// whoever holds the helm: neither hull needs the other's permission to
/// leave. A station holds several connections and names none, so a request
/// addressed to a station does nothing; the ship docked at it asks.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct DockingReleaseRequest {
    /// The ship asking to be free.
    pub entity: Entity,
}

/// Execute `DOCK`: pick the pair, reserve both ports, and lock the two hulls
/// together with one fixed joint.
///
/// The joint is created BEFORE the sleeves move, and its frames hold the
/// pose the two ships are in right now: both anchors on the midpoint of the
/// two faces, and the second body's basis equal to the relative rotation the
/// hulls met at, roll included. A joint left on `JointFrame::IDENTITY` would
/// instead drive both origins and both bases together and yank the ships into
/// each other.
///
/// The frames are LOCAL, computed here. avian3d 0.7 converts a global basis
/// (`FixedJoint::with_basis`) as `basis * rot.inverse()` where its solver
/// needs `rot.inverse() * basis`, so a global frame holds the capture pose
/// only when the two hull rotations commute. A belly-up hull against a yawed
/// one does not, and the joint then swings the partner through the error in
/// one step.
pub(super) fn on_docking_connection_request(
    request: On<DockingConnectionRequest>,
    mut commands: Commands,
    ports: DockingPorts,
    admission: DockAdmission,
    q_roots: Query<(
        Has<StationMarker>,
        Has<DerelictShipMarker>,
        Has<NeutralizedMarker>,
        Has<PlayerSpaceshipMarker>,
    )>,
    mut q_states: Query<&mut DockingSectionState>,
    q_controllers: Query<
        (&ChildOf, &ControllerSectionRotationInput),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
) {
    let (first_ship, second_ship) = (request.entity, request.target);
    trace!("on_docking_connection_request: {first_ship:?} -> {second_ship:?}");

    if let Some(refusal) = admission.refusal(first_ship, second_ship) {
        debug!("on_docking_connection_request: {second_ship:?} refuses a dock now ({refusal:?})");
        return;
    }
    let Some(candidate) = ports.best_candidate(first_ship, second_ship) else {
        debug!("on_docking_connection_request: no eligible port pair, refused");
        return;
    };
    let (Some(first_pose), Some(second_pose)) = (
        ports.pose(candidate.first_section, first_ship),
        ports.pose(candidate.second_section, second_ship),
    ) else {
        return;
    };
    let (Some((first_position, first_rotation)), Some((second_position, second_rotation))) =
        (ports.body_pose(first_ship), ports.body_pose(second_ship))
    else {
        return;
    };
    let anchor = first_pose.face.midpoint(second_pose.face);

    let is_station = |ship: Entity| q_roots.get(ship).is_ok_and(|(station, ..)| station);
    let hub = match (is_station(first_ship), is_station(second_ship)) {
        (true, true) => {
            debug!("on_docking_connection_request: two stations never dock, refused");
            return;
        }
        (true, false) => Some(first_ship),
        (false, true) => Some(second_ship),
        (false, false) => None,
    };
    let helm = pair_helm_on_capture([first_ship, second_ship], &q_roots);
    // Queued ahead of the joint spawn so the pair holds no contact when
    // avian's own `JointCollisionDisabled` observer runs.
    commands.queue(move |world: &mut World| {
        if let Err(error) =
            world.run_system_cached_with(drop_pair_contacts, (first_ship, second_ship))
        {
            error!("on_docking_connection_request: dropping the pair's contacts failed: {error}");
        }
    });
    let record = DockingConnection {
        first_ship,
        first_section: candidate.first_section,
        second_ship,
        second_section: candidate.second_section,
        helm,
        measurement_fault: false,
    };
    let joint = FixedJoint::new(first_ship, second_ship)
        .with_local_anchor1(first_rotation.inverse() * (anchor - first_position))
        .with_local_anchor2(second_rotation.inverse() * (anchor - second_position))
        .with_local_basis2(second_rotation.inverse() * first_rotation);
    let helms = [(first_ship, first_rotation), (second_ship, second_rotation)]
        .map(|(ship, rotation)| (ship, helm_command(ship, &q_controllers).unwrap_or(rotation)));
    let connection = connect(&mut commands, record, joint, hub, helms);
    debug!("on_docking_connection_request: connection {connection:?} {record:?}");
    for section in [record.first_section, record.second_section] {
        if let Ok(mut state) = q_states.get_mut(section) {
            *state = DockingSectionState::Extending;
        }
    }
}

/// Spawn the connection `record` held by `joint`, reserve both ports and put
/// each hull on its docked record with the attitude command `helms` names for
/// it. `hub` is the station endpoint, if one is. The caller starts both
/// sleeves out.
///
/// No root drives until the assembly pass has measured the pair. A hub that
/// already holds a dock keeps the record it has.
fn connect(
    commands: &mut Commands,
    record: DockingConnection,
    joint: FixedJoint,
    hub: Option<Entity>,
    helms: [(Entity, Quat); 2],
) -> Entity {
    let mut connection = commands.spawn((
        Name::new("Docking Connection"),
        record,
        joint,
        JointCollisionDisabled,
    ));
    if let Some(station) = hub {
        connection.insert(StationDock(station));
    }
    let connection = connection.id();
    for section in [record.first_section, record.second_section] {
        commands.entity(section).try_insert(DockedPort(connection));
    }
    for (ship, helm) in helms {
        if hub == Some(ship) {
            commands.entity(ship).try_insert_if_new(DockedStation {
                helm,
                drives: false,
            });
        } else {
            commands.entity(ship).try_insert(DockedShip {
                connection,
                helm,
                drives: false,
            });
        }
    }
    connection
}

/// Rebuild a saved connection: `record` held by a fixed joint with the local
/// frames `frames` (first body, second body), both ports reserved, both
/// sleeves started out and both hulls docked, exactly as `DOCK` left them.
/// Returns the connection.
///
/// It is not a capture. It judges no envelope and asks no admission, it
/// builds the joint from the saved frames rather than from where the hulls
/// stand, and it fires nothing. It drops no contact between the hulls, so
/// call it before the first physics step with both: a Load holds the clocks
/// from before either spawned.
///
/// # Panics
///
/// When either endpoint is not a live ship root or either port is not a
/// free, live docking port of its endpoint, or when a ship endpoint is
/// already docked. The caller checks each before it restores.
pub fn restore_docking_connection(
    world: &mut World,
    record: DockingConnection,
    frames: [JointFrame; 2],
) -> Entity {
    for (ship, port) in [
        (record.first_ship, record.first_section),
        (record.second_ship, record.second_section),
    ] {
        let root = world.entity(ship);
        assert!(
            root.contains::<SpaceshipRootMarker>(),
            "restore_docking_connection: {ship} is not a ship root"
        );
        assert!(
            root.contains::<StationMarker>() || !root.contains::<DockedShip>(),
            "restore_docking_connection: ship {ship} is already docked"
        );
        let section = world.entity(port);
        assert!(
            section.get::<ChildOf>().map(ChildOf::parent) == Some(ship)
                && section.contains::<DockingSectionMarker>()
                && !section.contains::<SectionInactiveMarker>()
                && !section.contains::<DockedPort>(),
            "restore_docking_connection: {port} is no free, live docking port of {ship}"
        );
    }
    let ships = [record.first_ship, record.second_ship];
    let hub = ships
        .into_iter()
        .find(|ship| world.entity(*ship).contains::<StationMarker>());
    let mut q_controllers = world.query_filtered::<(&ChildOf, &ControllerSectionRotationInput), (
        With<ControllerSectionMarker>,
        Without<SectionInactiveMarker>,
    )>();
    let helms = ships.map(|ship| {
        let command = q_controllers
            .iter(world)
            .find(|(mount, _)| mount.0 == ship)
            .map(|(_, command)| command.0);
        let rotation = world
            .get::<Rotation>(ship)
            .unwrap_or_else(|| panic!("restore_docking_connection: ship {ship} has no rotation"));
        (ship, command.unwrap_or(rotation.0))
    });
    let [frame1, frame2] = frames;
    let joint = FixedJoint {
        frame1,
        frame2,
        ..FixedJoint::new(record.first_ship, record.second_ship)
    };
    let mut commands = world.commands();
    let connection = connect(&mut commands, record, joint, hub, helms);
    for section in [record.first_section, record.second_section] {
        commands
            .entity(section)
            .insert(DockingSectionState::Extending);
    }
    world.flush();
    debug!("restore_docking_connection: connection {connection:?} {record:?}");
    connection
}

/// Drop every contact between two bodies that are about to be joined with
/// [`JointCollisionDisabled`], unlinking each one from its simulation island
/// first.
///
/// TODO(avian#1076): avian3d 0.7's `on_disable_joint_collision` removes the
/// pair's contacts from the contact and constraint graphs but not from their
/// island. A dock captured while the hulls touch then leaves the island
/// pointing at a dead `ContactId`; the next contact that joins the island
/// reuses that id and avian panics with "Head contact .. has no island". It
/// also removes the pair key built from the BODY indices, where the narrow
/// phase keys by COLLIDER, so the broad phase would never recreate the pair
/// after undock. Delete this function and its call when avian unlinks the
/// island itself.
fn drop_pair_contacts(
    In((body1, body2)): In<(Entity, Entity)>,
    q_colliders: Query<&RigidBodyColliders>,
    joint_graph: Res<JointGraph>,
    mut contact_graph: ResMut<ContactGraph>,
    mut constraint_graph: ResMut<ConstraintGraph>,
    mut islands: Option<ResMut<PhysicsIslands>>,
    mut body_islands: Query<&mut BodyIslandNode, Or<(With<Disabled>, Without<Disabled>)>>,
    mut colliding: Query<&mut CollidingEntities, Or<(With<Disabled>, Without<Disabled>)>>,
    mut collision_ended: MessageWriter<CollisionEnd>,
    mut commands: Commands,
) {
    let Ok(colliders) = q_colliders.get(body1) else {
        return;
    };
    let contacts: Vec<ContactId> = colliders
        .iter()
        .flat_map(|collider| contact_graph.contact_edges_with(collider))
        .filter(|edge| edge.body1 == Some(body2) || edge.body2 == Some(body2))
        .map(|edge| edge.id)
        .collect();

    let mut to_wake = Vec::new();
    for contact_id in contacts {
        let Some(edge) = contact_graph.get_edge_mut_by_id(contact_id) else {
            continue;
        };
        let (collider1, collider2) = (edge.collider1, edge.collider2);
        let (edge_body1, edge_body2) = (edge.body1, edge.body2);
        let manifolds = edge.constraint_handles.len();
        let linked = edge.island.is_some();
        let touching = edge.is_touching();
        let events = edge.events_enabled();

        if touching {
            if events {
                collision_ended.write(CollisionEnd {
                    collider1,
                    collider2,
                    body1: edge_body1,
                    body2: edge_body2,
                });
            }
            for (entity, other) in [(collider1, collider2), (collider2, collider1)] {
                if let Ok(mut entities) = colliding.get_mut(entity) {
                    entities.remove(&other);
                }
            }
        }
        for _ in 0..manifolds {
            constraint_graph.pop_manifold(&mut contact_graph.edges, contact_id, body1, body2);
        }
        if let (true, Some(islands)) = (linked, islands.as_deref_mut()) {
            let island = islands.remove_contact(
                contact_id,
                &mut body_islands,
                &mut contact_graph.edges,
                &joint_graph,
            );
            if island.is_sleeping() {
                to_wake.push(island.id());
            }
        }
        let pair_key = PairKey::new(collider1.index_u32(), collider2.index_u32());
        contact_graph.remove_edge_by_id(&pair_key, contact_id);
    }
    if !to_wake.is_empty() {
        commands.queue(WakeIslands(to_wake));
    }
}

/// Take the docked record and the assembly off a station whose last
/// connection has just gone.
pub(super) fn on_station_docks_emptied(removed: On<Remove, StationDocks>, mut commands: Commands) {
    commands
        .entity(removed.entity)
        .try_remove::<(DockedStation, super::DockedAssembly)>();
}

/// Free the partner of a port that has just been destroyed.
///
/// Reaches the surviving port in the same frame the dead one leaves, rather
/// than a tick later through [`release_broken_docking_connections`]. Both
/// paths end in [`release_connection`], so a ship that dies whole (every
/// section removed at once) releases exactly once either way.
pub(super) fn on_docking_port_removed_release(
    removed: On<Remove, DockingSectionMarker>,
    mut commands: Commands,
    q_docked: Query<&DockedPort>,
    q_connections: Query<(&DockingConnection, Option<&StationDock>)>,
    mut q_states: Query<&mut DockingSectionState>,
) {
    let Ok(&DockedPort(connection)) = q_docked.get(removed.entity) else {
        return;
    };
    let Ok((record, hub)) = q_connections.get(connection) else {
        return;
    };
    debug!(
        "on_docking_port_removed_release: port {:?} died",
        removed.entity
    );
    release_connection(&mut commands, connection, record, hub, &mut q_states);
}

/// Drop any connection whose endpoints are no longer a live pair of ports on
/// a live pair of hulls.
pub(super) fn release_broken_docking_connections(
    mut commands: Commands,
    q_connections: Query<(Entity, &DockingConnection, Option<&StationDock>)>,
    q_ships: Query<(), With<SpaceshipRootMarker>>,
    q_ports: Query<(), (With<DockingSectionMarker>, Without<SectionInactiveMarker>)>,
    mut q_states: Query<&mut DockingSectionState>,
) {
    for (entity, connection, hub) in &q_connections {
        let intact = q_ships.contains(connection.first_ship)
            && q_ships.contains(connection.second_ship)
            && q_ports.contains(connection.first_section)
            && q_ports.contains(connection.second_section);
        if !intact {
            debug!("release_broken_docking_connections: {entity:?} lost an endpoint");
            release_connection(&mut commands, entity, connection, hub, &mut q_states);
        }
    }
}

/// Despawn every [`UndockSeparation`] whose hulls no longer overlap. A hull
/// with no collider overlaps nothing.
///
/// The hulls overlap while any non-sensor collider box of one meets the
/// merged collider box of the other, taken from the hull with fewer colliders
/// so the merged box stays tight. No hit against the merged box means no hit
/// against any of its colliders, so the narrow phase finds no penetration
/// once the pair collides again. A hull parked overlapping stays filtered
/// until it moves clear; there is no timeout.
pub(super) fn end_undock_separations(
    mut commands: Commands,
    q_separations: Query<(Entity, &UndockSeparation)>,
    q_roots: Query<&RigidBodyColliders>,
    q_aabbs: Query<&ColliderAabb, Without<Sensor>>,
) {
    for (entity, separation) in &q_separations {
        let (first, second) = (separation.first_ship, separation.second_ship);
        let Ok([first_colliders, second_colliders]) = q_roots.get_many([first, second]) else {
            debug!("end_undock_separations: {first:?}/{second:?} has a hull with no colliders");
            commands.entity(entity).try_despawn();
            continue;
        };
        let (merged, other) = if first_colliders.len() <= second_colliders.len() {
            (first_colliders, second_colliders)
        } else {
            (second_colliders, first_colliders)
        };
        // `INVALID` is empty: a hull with no solid collider meets nothing.
        let merged = q_aabbs
            .iter_many(merged.iter())
            .fold(ColliderAabb::INVALID, |merged, aabb| merged.merged(*aabb));
        let overlapping = q_aabbs
            .iter_many(other.iter())
            .any(|aabb| aabb.intersects(&merged));
        if !overlapping {
            debug!("end_undock_separations: {first:?}/{second:?} separated, collisions restored");
            commands.entity(entity).try_despawn();
        }
    }
}

/// Despawn every [`UndockSeparation`] that names a body as the body leaves.
///
/// avian 0.7 frees an island when its last body leaves without unlinking the
/// island's joints. A separation joint that outlives its hulls then keeps a
/// dead island id, and once a new body takes that island slot, despawning the
/// joint panics in `PhysicsIslands::remove_joint`. A world teardown or a load
/// does exactly that while [`end_undock_separations`] waits for the fixed
/// clock. The despawn queued here runs in the same command flush as the body
/// removal, before any new body can take the slot.
pub(super) fn on_separated_body_removed(
    removed: On<Remove, RigidBody>,
    mut commands: Commands,
    q_separations: Query<(Entity, &UndockSeparation)>,
) {
    let body = removed.entity;
    for (entity, separation) in &q_separations {
        if separation.first_ship == body || separation.second_ship == body {
            debug!("on_separated_body_removed: {body:?} left, its separation {entity:?} ends");
            commands.entity(entity).try_despawn();
        }
    }
}

/// Let a docked ship go, on its own request.
///
/// Ungated: `DOCK` pressed again always works, exactly as `ORBIT` pressed
/// again always disengages. A capability withdrawn while docked must never be
/// able to strand a hull clamped to something.
pub(super) fn on_docking_release_request(
    request: On<DockingReleaseRequest>,
    mut commands: Commands,
    q_docked: Query<&DockedShip>,
    q_connections: Query<(&DockingConnection, Option<&StationDock>)>,
    mut q_states: Query<&mut DockingSectionState>,
) {
    let Ok(docked) = q_docked.get(request.entity) else {
        debug!(
            "on_docking_release_request: {:?} is not docked",
            request.entity
        );
        return;
    };
    let Ok((record, hub)) = q_connections.get(docked.connection) else {
        return;
    };
    debug!(
        "on_docking_release_request: {:?} lets go of {:?}",
        request.entity, docked.connection
    );
    release_connection(&mut commands, docked.connection, record, hub, &mut q_states);
}

/// Take or hand back the helm of the pair the player is docked in.
///
/// Flips [`DockingConnection::helm`] only. The roots' `drives` follow on the
/// next fixed tick, from `measure_docked_assemblies`, which is the one place
/// that decides who actuates - so a toggle and a lost player can never write
/// two different answers.
pub(super) fn on_docking_helm_request(
    request: On<DockingHelmRequest>,
    q_player: Query<&DockedShip, With<PlayerSpaceshipMarker>>,
    mut q_connections: Query<&mut DockingConnection>,
) {
    let ship = request.entity;
    let Ok(docked) = q_player.get(ship) else {
        debug!("on_docking_helm_request: {ship:?} is not a docked player ship, refused");
        return;
    };
    let Ok(mut connection) = q_connections.get_mut(docked.connection) else {
        return;
    };
    if !helm_request_admitted(connection.helm) {
        debug!(
            "on_docking_helm_request: {:?} is held by its station ({:?}), refused",
            docked.connection, connection.helm
        );
        return;
    }
    connection.helm = match connection.helm {
        DockedHelmType::Held(holder) if holder == ship => DockedHelmType::Neutral,
        _ if connection.measurement_fault => {
            debug!(
                "on_docking_helm_request: {:?} cannot be measured, refused",
                docked.connection
            );
            return;
        }
        _ => DockedHelmType::Held(ship),
    };
    debug!(
        "on_docking_helm_request: {ship:?} sets {:?} to {:?}",
        docked.connection, connection.helm
    );
}

/// Hand a docked player the helm of a station that has just died.
///
/// A living station owns every connection to it ([`DockedHelmType::Station`]);
/// its flight computer goes inactive when it turns derelict, so the assembly
/// would be held by nobody. Each connection takes the helm a dead station
/// opens with ([`helm_on_capture`]), as if its hull had docked at it dead: the
/// player's connection is the player's, every other one is neutral.
pub(super) fn on_station_derelict_release_helm(
    added: On<Add, DerelictShipMarker>,
    q_station: Query<&StationDocks, With<StationMarker>>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut q_connections: Query<&mut DockingConnection>,
) {
    let station = added.entity;
    let Ok(docks) = q_station.get(station) else {
        return;
    };
    for entity in docks.iter() {
        let Ok(mut connection) = q_connections.get_mut(entity) else {
            continue;
        };
        if connection.helm != DockedHelmType::Station(station) {
            continue;
        }
        let player = [connection.first_ship, connection.second_ship]
            .into_iter()
            .find(|ship| q_player.contains(*ship));
        connection.helm = helm_on_capture(DockPartnerType::Dead, station, player);
        debug!(
            "on_station_derelict_release_helm: station {station:?} died, {entity:?} is now {:?}",
            connection.helm
        );
    }
}

/// Hold every HELD docked root's command on the attitude it actually has.
///
/// A held root's attitude loop is not applied, so a command left where it
/// was would be a stale order its PD snaps to the instant the root drives
/// again - on taking or handing back the helm, and on undock. Re-parking it on
/// the root's live attitude every tick means the command a root resumes is
/// the direction it already points. The driver is left alone: its command is
/// its pilot's.
pub(super) fn park_suppressed_docked_helms(
    mut q_ships: Query<(
        Entity,
        &Rotation,
        Option<&mut DockedShip>,
        Option<&mut DockedStation>,
    )>,
    mut q_controllers: Query<
        (&ChildOf, &mut ControllerSectionRotationInput),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
) {
    for (root, rotation, ship, station) in &mut q_ships {
        let helm = match (ship, station) {
            (Some(ship), _) if !ship.drives => ship.map_unchanged(|ship| &mut ship.helm),
            (_, Some(station)) if !station.drives => {
                station.map_unchanged(|station| &mut station.helm)
            }
            _ => continue,
        };
        for (mount, mut command) in &mut q_controllers {
            if mount.0 == root {
                **command = rotation.0;
            }
        }
        *helm.into_inner() = rotation.0;
    }
}

/// The helm a pair of `ships` opens with, through [`helm_on_capture`]: a
/// station endpoint decides, living or dead; otherwise a dead ship endpoint
/// (the one that is not the player) decides; two living ships start neutral.
fn pair_helm_on_capture(
    ships: [Entity; 2],
    q_roots: &Query<(
        Has<StationMarker>,
        Has<DerelictShipMarker>,
        Has<NeutralizedMarker>,
        Has<PlayerSpaceshipMarker>,
    )>,
) -> DockedHelmType {
    let player = ships
        .into_iter()
        .find(|ship| q_roots.get(*ship).is_ok_and(|(.., player)| player));
    let station = ships.into_iter().find_map(|ship| {
        let (station, derelict, neutralized, _) = q_roots.get(ship).ok()?;
        station.then_some((
            ship,
            if derelict || neutralized {
                DockPartnerType::Dead
            } else {
                DockPartnerType::LivingStation
            },
        ))
    });
    if let Some((station, partner)) = station {
        return helm_on_capture(partner, station, player);
    }
    let dead_partner = ships.into_iter().any(|ship| {
        Some(ship) != player
            && q_roots
                .get(ship)
                .is_ok_and(|(_, derelict, neutralized, _)| derelict || neutralized)
    });
    let partner = if dead_partner {
        DockPartnerType::Dead
    } else {
        DockPartnerType::Ship
    };
    helm_on_capture(partner, Entity::PLACEHOLDER, player)
}

/// The attitude command `ship` is holding, from its first live controller.
/// `None` for a hull with no working computer - which is a hull that cannot
/// express rotation intent, not an error.
fn helm_command(
    ship: Entity,
    q_controllers: &Query<
        (&ChildOf, &ControllerSectionRotationInput),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
) -> Option<Quat> {
    q_controllers
        .iter()
        .find(|(mount, _)| mount.0 == ship)
        .map(|(_, command)| command.0)
}

/// Despawn `connection` (and with it the joint and its helm), free both
/// ports, start both surviving sleeves back in, and keep the pair free of
/// collisions with an [`UndockSeparation`] until the hulls separate.
///
/// The ship side loses its docked record and its assembly. The `hub` side,
/// a station, keeps both while it holds other connections: its record goes
/// with the last one (`on_station_docks_emptied`), and its assembly is
/// re-measured without this hull on the next tick.
///
/// Every removal is a `try_`: this runs on the destruction path too, where
/// half the entities it names are already on their way out.
fn release_connection(
    commands: &mut Commands,
    entity: Entity,
    connection: &DockingConnection,
    hub: Option<&StationDock>,
    q_states: &mut Query<&mut DockingSectionState>,
) {
    commands.entity(entity).try_despawn();
    let (first_ship, second_ship) = (connection.first_ship, connection.second_ship);
    commands.queue(move |world: &mut World| {
        // A hull destroyed with its dock leaves no pair to keep apart.
        if world.get_entity(first_ship).is_err() || world.get_entity(second_ship).is_err() {
            return;
        }
        // A pair docked and released again before it separated still holds
        // its separation, which ends once.
        let pair = [first_ship, second_ship];
        if world
            .query::<&UndockSeparation>()
            .iter(world)
            .any(|held| pair.contains(&held.first_ship) && pair.contains(&held.second_ship))
        {
            return;
        }
        debug!(
            "release_connection: {first_ship:?}/{second_ship:?} stay collision-free until the \
             hulls separate"
        );
        world.spawn((
            Name::new("Undock Separation"),
            UndockSeparation {
                first_ship,
                second_ship,
            },
            DistanceJoint::new(first_ship, second_ship).with_limits(0.0, f32::MAX),
            JointCollisionDisabled,
        ));
    });
    for section in [connection.first_section, connection.second_section] {
        commands.entity(section).try_remove::<DockedPort>();
        if let Ok(mut state) = q_states.get_mut(section) {
            *state = DockingSectionState::Retracting;
        }
    }
    for ship in [connection.first_ship, connection.second_ship] {
        if hub.is_some_and(|hub| hub.0 == ship) {
            continue;
        }
        commands
            .entity(ship)
            .try_remove::<(DockedShip, super::DockedAssembly)>();
    }
}
