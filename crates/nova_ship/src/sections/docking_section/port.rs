//! Where a port is, which way it faces, and which pair of ports `DOCK` picks.
//!
//! One [`SystemParam`] answers both questions the verb asks - "is there a
//! candidate at all" (the HUD chip, every frame) and "what exactly am I
//! docking" (the command, once) - so the offer and the act can never disagree
//! about the same two ships.

use avian3d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::*;

use super::{
    DockedPort, DockedShip, DockingEnvelope, DockingSectionConfigHelper, DockingSectionMarker,
};
use crate::sections::local_pose_in_root;

/// Where a retracted port's mouth sits in its own frame: half a cell along
/// the section's outward axis.
///
/// The outward axis is local -Z, the direction every section faces the world
/// with (a thruster's plume, a bay's muzzle, a lance's bore). A docking port
/// is a one-cell section by contract, so its face IS the cell face, and the
/// sleeve grows out from here without moving it.
const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);

/// One port's pose in the world: the centre of its retracted face and the
/// direction it faces.
///
/// Public because the docking sight draws exactly these two things: the face
/// a gap is measured from, and the axis that has to be lined up. An
/// instrument that recomputed either would eventually disagree with the
/// mechanic it is drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortPose {
    /// The centre of the retracted outer face.
    pub face: Vec3,
    /// Unit outward axis.
    pub axis: Vec3,
}

/// One pair of free ports, graded against the stricter of their two
/// envelopes - whether or not it passes.
///
/// Returned two ways, and that is the point. [`DockingPorts::best_candidate`]
/// yields only pairs that pass every gate, which is what the verb acts on;
/// [`DockingPorts::nearest_pair`] yields the nearest pair whatever its state,
/// which is what the docking sight DRAWS. An instrument that could only see
/// eligible pairs would appear at the moment it stopped being needed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DockingPair {
    /// The ship that issued `DOCK`, or the player the sight is drawn for.
    pub first_ship: Entity,
    /// Its port.
    pub first_section: Entity,
    /// The locked ship.
    pub second_ship: Entity,
    /// Its port.
    pub second_section: Entity,
    /// The first port's face and axis, in world space.
    pub first: PortPose,
    /// The second port's face and axis, in world space.
    pub second: PortPose,
    /// Gap between the two retracted faces, engine units.
    pub gap: f32,
    /// How opposed the two outward axes are: `-1` is exactly facing, `0` is
    /// perpendicular. Lower is better, which is why the ranking sorts it
    /// ascending like the gap.
    pub opposition: f32,
    /// How fast the two hulls are closing, engine units per second.
    pub relative_speed: f32,
    /// How fast they are turning relative to each other, radians per second.
    pub relative_spin: f32,
    /// The stricter of the two ports' envelopes: the one this pair is graded
    /// against, and the one the sight colours against.
    pub envelope: DockingEnvelope,
}

impl DockingPair {
    /// Whether the faces are close enough.
    pub fn gap_holds(&self) -> bool {
        self.gap <= self.envelope.capture_distance
    }

    /// Whether the two axes are opposed enough.
    ///
    /// Roll is absent on purpose. A cylindrical port has no keyed feature to
    /// line up, so the only orientation question is whether the two axes face
    /// each other; how the two hulls are clocked about that axis is whatever
    /// it was at capture, and the joint simply keeps it.
    pub fn facing_holds(&self) -> bool {
        self.opposition <= -self.envelope.capture_angle.cos()
    }

    /// Whether the two hulls are calm enough relative to each other.
    pub fn motion_holds(&self) -> bool {
        self.relative_speed <= self.envelope.maximum_relative_speed
            && self.relative_spin <= self.envelope.maximum_relative_angular_speed
    }

    /// Whether `DOCK` may be offered on this pair.
    pub fn is_eligible(&self) -> bool {
        self.gap_holds() && self.facing_holds() && self.motion_holds()
    }
}

/// The world reads the docking verb needs: every free port, the mount chain
/// under it, and the bodies the two hulls actually are.
///
/// A [`SystemParam`] rather than five arguments repeated at three call sites,
/// and the reason the availability check and the command share one definition
/// of "eligible". Note the `Without<DockedPort>` filter: a reserved port is
/// not a candidate, which is the whole of the one-connection-per-port rule.
#[derive(SystemParam)]
pub struct DockingPorts<'w, 's> {
    /// Free, live ports: the section entity, the hull it is mounted on, and
    /// its authored envelope.
    ports: Query<
        'w,
        's,
        (
            Entity,
            &'static ChildOf,
            &'static DockingSectionConfigHelper,
        ),
        (
            With<DockingSectionMarker>,
            Without<DockedPort>,
            Without<SectionInactiveMarker>,
        ),
    >,
    /// The mount chain from a section up to its root.
    chain: Query<'w, 's, (&'static Transform, &'static ChildOf)>,
    /// Ship roots, in avian's own pose and velocity - never `GlobalTransform`,
    /// which inside `FixedUpdate` still holds the previous frame's eased
    /// render pose.
    bodies: Query<
        'w,
        's,
        (
            &'static Position,
            &'static Rotation,
            &'static LinearVelocity,
            &'static AngularVelocity,
        ),
        With<SpaceshipRootMarker>,
    >,
    /// The design-local id of a section, where the spawn path gave it one.
    /// The last tie-break, and the reason two equally good pairs resolve to
    /// the same answer on every run.
    ids: Query<'w, 's, &'static EntityId>,
    /// Ships that already hold a connection. One per hull in this baseline:
    /// the second dock would need a docking GRAPH, and that is deferred.
    docked: Query<'w, 's, (), With<DockedShip>>,
}

impl DockingPorts<'_, '_> {
    /// The pose of `section`, mounted on `root`, in world space.
    pub(crate) fn pose(&self, section: Entity, root: Entity) -> Option<PortPose> {
        let (local_position, local_rotation) = local_pose_in_root(section, root, &self.chain)?;
        let (position, rotation, ..) = self.bodies.get(root).ok()?;
        let world_rotation = rotation.0 * local_rotation;
        Some(PortPose {
            face: position.0 + rotation.0 * local_position + world_rotation * PORT_FACE,
            axis: world_rotation * Vec3::NEG_Z,
        })
    }

    /// The world rotation of a ship root, as avian holds it.
    pub(crate) fn body_rotation(&self, root: Entity) -> Option<Quat> {
        self.bodies
            .get(root)
            .ok()
            .map(|(_, rotation, ..)| rotation.0)
    }

    /// The best pair of free ports between `first_ship` and `second_ship`
    /// that passes every gate, or `None` when none does.
    ///
    /// Gate first, then rank. Ranking the whole field and gating the winner
    /// would hide an eligible pair behind a nearer one that is facing the
    /// wrong way.
    pub fn best_candidate(&self, first_ship: Entity, second_ship: Entity) -> Option<DockingPair> {
        self.rank_pairs(first_ship, second_ship, true)
    }

    /// The nearest pair of free ports between the two ships, eligible or not.
    ///
    /// What the docking sight draws. A pilot needs the line to line up with
    /// BEFORE the pair is eligible - that is the whole of the approach - so
    /// this one gates nothing and reports the measurements instead.
    pub fn nearest_pair(&self, first_ship: Entity, second_ship: Entity) -> Option<DockingPair> {
        self.rank_pairs(first_ship, second_ship, false)
    }

    /// The shared walk: every free port on one hull against every free port on
    /// the other, ranked by face gap, then alignment, then the two section
    /// ids.
    ///
    /// The ids are what make the answer a FUNCTION of the world rather than of
    /// query iteration order - two ports mounted symmetrically on the same
    /// hull are the same distance and the same alignment away, and without the
    /// last key the pair chosen would depend on archetype layout.
    fn rank_pairs(
        &self,
        first_ship: Entity,
        second_ship: Entity,
        eligible_only: bool,
    ) -> Option<DockingPair> {
        if first_ship == second_ship
            || self.docked.contains(first_ship)
            || self.docked.contains(second_ship)
        {
            return None;
        }
        let (.., first_velocity, first_spin) = self.bodies.get(first_ship).ok()?;
        let (.., second_velocity, second_spin) = self.bodies.get(second_ship).ok()?;
        let relative_speed = (first_velocity.0 - second_velocity.0).length();
        let relative_spin = (first_spin.0 - second_spin.0).length();

        let mut best: Option<(DockingPair, &str, &str)> = None;
        for (first_section, first_root, first_config) in &self.ports {
            if first_root.0 != first_ship {
                continue;
            }
            let Some(first_pose) = self.pose(first_section, first_ship) else {
                continue;
            };
            for (second_section, second_root, second_config) in &self.ports {
                if second_root.0 != second_ship {
                    continue;
                }
                let Some(second_pose) = self.pose(second_section, second_ship) else {
                    continue;
                };
                let pair = judge_pair(
                    first_ship,
                    first_section,
                    first_pose,
                    second_ship,
                    second_section,
                    second_pose,
                    first_config.envelope().strictest(second_config.envelope()),
                    relative_speed,
                    relative_spin,
                );
                if eligible_only && !pair.is_eligible() {
                    continue;
                }
                let keys = (
                    self.ids.get(first_section).map_or("", |id| id.0.as_str()),
                    self.ids.get(second_section).map_or("", |id| id.0.as_str()),
                );
                let better = best.as_ref().is_none_or(|(best, first_key, second_key)| {
                    (pair.gap, pair.opposition, keys.0, keys.1)
                        < (best.gap, best.opposition, first_key, second_key)
                });
                if better {
                    best = Some((pair, keys.0, keys.1));
                }
            }
        }
        best.map(|(pair, ..)| pair)
    }
}

/// Measure one pair of poses against `envelope`. Grades nothing away: the
/// caller decides whether an ineligible pair is still interesting.
#[expect(
    clippy::too_many_arguments,
    reason = "the measurement takes both ends of the pair and the motion they share"
)]
fn judge_pair(
    first_ship: Entity,
    first_section: Entity,
    first: PortPose,
    second_ship: Entity,
    second_section: Entity,
    second: PortPose,
    envelope: DockingEnvelope,
    relative_speed: f32,
    relative_spin: f32,
) -> DockingPair {
    DockingPair {
        first_ship,
        first_section,
        second_ship,
        second_section,
        first,
        second,
        gap: first.face.distance(second.face),
        opposition: first.axis.dot(second.axis),
        relative_speed,
        relative_spin,
        envelope,
    }
}
