//! The connection: what `DOCK` builds, what holds it, who flies it, and what
//! takes it away.
//!
//! One accepted pair becomes ONE entity carrying a
//! [`DockingConnection`] and the avian [`FixedJoint`] that does the actual
//! work. Releasing is therefore a despawn of that one entity and nothing else
//! - no query sweeps a joint it did not create.
//!
//! # One helm per pair
//!
//! The connection owns the pair's helm ([`DockedHelmType`]). A dock starts
//! NEUTRAL: the player's hull is held and the partner flies the pair - its
//! autopilot, its AI and its scripted order keep working. `HELM` takes the
//! pair ([`DockedHelmType::Held`]): the player flies it and the partner is
//! held instead, its maneuver and its order frozen rather than retired.
//! `HELM` again hands it back. Neither touches the joint; `DOCK` pressed again
//! is still what ends a dock, from either side.
//!
//! Exactly one root drives a pair ([`DockedShip::drives`]) and every actuator
//! skips the other. The driver flies the ASSEMBLY rather than its own hull:
//! see [`DockedAssembly`](super::DockedAssembly).

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::{DockingPorts, DockingSectionMarker, DockingSectionState};
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
    /// starts here.
    #[default]
    Neutral,
    /// The player ship named here flies the pair; the partner is held.
    Held(Entity),
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

/// On both roots of a docked pair, naming the connection and whether this
/// root is the one that flies the pair.
///
/// Its presence takes the root's torque off the normal PD path (see
/// `sync_controller_section_forces`): the driver's attitude loop is spread
/// over both roots by `apply_docked_helm_wrench`, and a held root's loop is
/// not applied at all. One connection per ship - a second `DOCK` onto an
/// already docked hull is refused rather than growing a docking graph nothing
/// else is ready for.
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
/// player could not fly it. Handing the helm back is never refused.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct DockingHelmRequest {
    /// The player ship asking.
    pub entity: Entity,
}

/// Asks for the connection holding `entity` to be let go.
///
/// Addressed to a SHIP, not to a connection, because that is what the pilot
/// who presses `DOCK` again knows about. Either end may ask, whoever holds the
/// helm: neither hull needs the other's permission to leave.
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

    let record = DockingConnection {
        first_ship,
        first_section: candidate.first_section,
        second_ship,
        second_section: candidate.second_section,
        helm: DockedHelmType::Neutral,
        measurement_fault: false,
    };
    let connection = commands
        .spawn((
            Name::new("Docking Connection"),
            record,
            FixedJoint::new(first_ship, second_ship)
                .with_local_anchor1(first_rotation.inverse() * (anchor - first_position))
                .with_local_anchor2(second_rotation.inverse() * (anchor - second_position))
                .with_local_basis2(second_rotation.inverse() * first_rotation),
        ))
        .id();
    debug!("on_docking_connection_request: connection {connection:?} {record:?}");

    for section in [candidate.first_section, candidate.second_section] {
        commands.entity(section).try_insert(DockedPort(connection));
        if let Ok(mut state) = q_states.get_mut(section) {
            *state = DockingSectionState::Extending;
        }
    }
    for (ship, rotation) in [(first_ship, first_rotation), (second_ship, second_rotation)] {
        let helm = helm_command(ship, &q_controllers).unwrap_or(rotation);
        // Neither root drives until the assembly pass has measured the pair.
        commands.entity(ship).try_insert(DockedShip {
            connection,
            helm,
            drives: false,
        });
    }
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
    q_connections: Query<&DockingConnection>,
    mut q_states: Query<&mut DockingSectionState>,
) {
    let Ok(&DockedPort(connection)) = q_docked.get(removed.entity) else {
        return;
    };
    let Ok(record) = q_connections.get(connection) else {
        return;
    };
    debug!(
        "on_docking_port_removed_release: port {:?} died",
        removed.entity
    );
    release_connection(&mut commands, connection, record, &mut q_states);
}

/// Drop any connection whose endpoints are no longer a live pair of ports on
/// a live pair of hulls.
pub(super) fn release_broken_docking_connections(
    mut commands: Commands,
    q_connections: Query<(Entity, &DockingConnection)>,
    q_ships: Query<(), With<SpaceshipRootMarker>>,
    q_ports: Query<(), (With<DockingSectionMarker>, Without<SectionInactiveMarker>)>,
    mut q_states: Query<&mut DockingSectionState>,
) {
    for (entity, connection) in &q_connections {
        let intact = q_ships.contains(connection.first_ship)
            && q_ships.contains(connection.second_ship)
            && q_ports.contains(connection.first_section)
            && q_ports.contains(connection.second_section);
        if !intact {
            debug!("release_broken_docking_connections: {entity:?} lost an endpoint");
            release_connection(&mut commands, entity, connection, &mut q_states);
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
    q_connections: Query<&DockingConnection>,
    mut q_states: Query<&mut DockingSectionState>,
) {
    let Ok(docked) = q_docked.get(request.entity) else {
        debug!(
            "on_docking_release_request: {:?} is not docked",
            request.entity
        );
        return;
    };
    let Ok(record) = q_connections.get(docked.connection) else {
        return;
    };
    debug!(
        "on_docking_release_request: {:?} lets go of {:?}",
        request.entity, docked.connection
    );
    release_connection(&mut commands, docked.connection, record, &mut q_states);
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

/// Hold every HELD docked root's command on the attitude it actually has.
///
/// A held root's attitude loop is not applied, so a command left where it
/// was would be a stale order its PD snaps to the instant the root drives
/// again - on taking or handing back the helm, and on undock. Re-parking it on
/// the root's live attitude every tick means the command a root resumes is
/// the direction it already points. The driver is left alone: its command is
/// its pilot's.
pub(super) fn park_suppressed_docked_helms(
    mut q_ships: Query<(Entity, &Rotation, &mut DockedShip)>,
    mut q_controllers: Query<
        (&ChildOf, &mut ControllerSectionRotationInput),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
) {
    for (ship, rotation, mut docked) in &mut q_ships {
        if docked.drives {
            continue;
        }
        for (mount, mut command) in &mut q_controllers {
            if mount.0 == ship {
                **command = rotation.0;
            }
        }
        docked.helm = rotation.0;
    }
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
/// ports, and start both surviving sleeves back in.
///
/// Every removal is a `try_`: this runs on the destruction path too, where
/// half the entities it names are already on their way out.
fn release_connection(
    commands: &mut Commands,
    entity: Entity,
    connection: &DockingConnection,
    q_states: &mut Query<&mut DockingSectionState>,
) {
    commands.entity(entity).try_despawn();
    for section in [connection.first_section, connection.second_section] {
        commands.entity(section).try_remove::<DockedPort>();
        if let Ok(mut state) = q_states.get_mut(section) {
            *state = DockingSectionState::Retracting;
        }
    }
    for ship in [connection.first_ship, connection.second_ship] {
        commands
            .entity(ship)
            .try_remove::<(DockedShip, super::DockedAssembly)>();
    }
}
