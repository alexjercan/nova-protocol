//! The connection: what `DOCK` builds, what holds it, and what takes it away.
//!
//! One accepted pair becomes ONE entity carrying a
//! [`DockingConnection`] and the avian [`FixedJoint`] that does the actual
//! work. Releasing is therefore a despawn of that one entity and nothing else
//! - no query sweeps a joint it did not create.
//!
//! # A dock is MODAL
//!
//! While a connection holds, both hulls are held: no thrust, no RCS, no
//! torque, and the helm follows the hull instead of the pilot. `DOCK` pressed
//! again is what ends it, the way `ORBIT` pressed again ends a parking - and
//! either side may press it.
//!
//! A verb, and not "fresh movement intent", because intent is not observable
//! from a control value on a ship whose controls never stop moving:
//! mouse-look writes an attitude command every frame a hand is on the mouse,
//! so an intent rule ends a dock roughly when it begins.
//!
//! An engaged [`Autopilot`] is the one thing besides the verb that releases.
//! A scenario that orders a docked ship somewhere must never be able to trap
//! it, and an autopilot holding a heading of its own would fight the joint
//! every tick.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::{DockingPorts, DockingSectionMarker, DockingSectionState};
use crate::prelude::*;

/// Ordering handle for docking's own fixed-clock work.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DockingSystems {
    /// Takes a connection away: on a destroyed endpoint, or on an engaged
    /// autopilot. Pinned ahead of the attitude copy and the section pass, so
    /// a maneuver that breaks a dock is flown by a ship that is already free.
    Release,
}

/// A live docking connection, on its own entity beside the [`FixedJoint`].
///
/// The canonical record of one pair. `first` is the ship that issued the
/// command; the order is not otherwise meaningful, and nothing may assume the
/// first ship is in charge of anything - it is not.
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
}

/// On a port that is spoken for, naming the connection that holds it. Its
/// ABSENCE is what makes a port a candidate, which is the whole of the
/// one-connection-per-port rule.
#[derive(Component, Clone, Copy, Debug, Deref, Reflect)]
#[reflect(Component)]
pub struct DockedPort(pub Entity);

/// On a docked ship's root, naming its connection and holding the helm
/// command the release pass grades fresh intent against.
///
/// Its presence also switches the ship's attitude loop off: a docked hull is
/// held by the joint, and a PD controller left running would spend the whole
/// dock pushing against it (see `sync_controller_section_forces`). One
/// connection per ship in this baseline - a second `DOCK` onto an already
/// docked hull is refused rather than growing a docking graph nothing else is
/// ready for.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct DockedShip {
    /// The connection holding this ship.
    pub connection: Entity,
    /// The attitude command, re-parked on the hull's live attitude every tick
    /// the dock holds. Nothing grades intent against it any more; it is what
    /// makes the UNDOCK clean, because the helm a released ship inherits is
    /// the direction it is already pointing rather than the order it was
    /// flying when it arrived.
    pub helm: Quat,
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

/// Asks for the connection holding `entity` to be let go.
///
/// Addressed to a SHIP, not to a connection, because that is what the pilot
/// who presses `DOCK` again knows about. Either end may ask: a dock is not an
/// authority, and neither hull needs the other's permission to leave.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct DockingReleaseRequest {
    /// The ship asking to be free.
    pub entity: Entity,
}

/// Execute `DOCK`: pick the pair, reserve both ports, and lock the two hulls
/// together with one fixed joint.
///
/// The joint is created BEFORE the sleeves move, and its frames are taken
/// from the pose the two ships are in right now:
/// [`FixedJoint::with_anchor`] and [`with_basis`](FixedJoint::with_basis)
/// place one GLOBAL frame that avian resolves into each body's local frame on
/// the next step, which is what makes the joint hold the capture pose -
/// including whatever relative roll the two hulls met at. A joint left on
/// `JointFrame::IDENTITY` would instead drive both origins and both bases
/// together and yank the ships into each other.
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
    let Some(basis) = ports.body_rotation(first_ship) else {
        return;
    };

    let record = DockingConnection {
        first_ship,
        first_section: candidate.first_section,
        second_ship,
        second_section: candidate.second_section,
    };
    let connection = commands
        .spawn((
            Name::new("Docking Connection"),
            record,
            FixedJoint::new(first_ship, second_ship)
                .with_anchor(first_pose.face.midpoint(second_pose.face))
                .with_basis(Rotation(basis)),
        ))
        .id();
    debug!("on_docking_connection_request: connection {connection:?} {record:?}");

    for section in [candidate.first_section, candidate.second_section] {
        commands.entity(section).try_insert(DockedPort(connection));
        if let Ok(mut state) = q_states.get_mut(section) {
            *state = DockingSectionState::Extending;
        }
    }
    for ship in [first_ship, second_ship] {
        let helm = helm_command(ship, &q_controllers)
            .or_else(|| ports.body_rotation(ship))
            .unwrap_or(Quat::IDENTITY);
        commands
            .entity(ship)
            .try_insert(DockedShip { connection, helm });
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

/// Hold the helm of every docked hull on the attitude it actually has, and
/// let go of any pair where a maneuver has been engaged.
///
/// The parking is what makes the release clean: a docked ship's attitude loop
/// is off, so a command left where it was at capture would be a stale order
/// the PD snaps to the instant the dock ends. Re-parking it on the hull's own
/// live attitude every tick means the helm a released ship inherits is the
/// direction it is already pointing.
pub(super) fn park_docked_helms_and_release_maneuvers(
    mut commands: Commands,
    q_connections: Query<(Entity, &DockingConnection)>,
    mut q_ships: Query<(&Rotation, &mut DockedShip, Option<&Autopilot>)>,
    mut q_controllers: Query<
        (&ChildOf, &mut ControllerSectionRotationInput),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    mut q_states: Query<&mut DockingSectionState>,
) {
    for (entity, connection) in &q_connections {
        let ships = [connection.first_ship, connection.second_ship];
        let flying = ships.iter().any(|ship| {
            q_ships
                .get(*ship)
                .is_ok_and(|(_, _, autopilot)| autopilot.is_some())
        });
        if flying {
            debug!("park_docked_helms_and_release_maneuvers: {entity:?} is flying a maneuver");
            release_connection(&mut commands, entity, connection, &mut q_states);
            continue;
        }

        for ship in ships {
            let Ok((rotation, mut docked, _)) = q_ships.get_mut(ship) else {
                continue;
            };
            for (mount, mut command) in &mut q_controllers {
                if mount.0 == ship {
                    **command = rotation.0;
                }
            }
            docked.helm = rotation.0;
        }
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

/// Despawn `connection` (and with it the joint), free both ports, and start
/// both surviving sleeves back in.
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
        commands.entity(ship).try_remove::<DockedShip>();
    }
}
