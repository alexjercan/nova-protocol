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
pub(crate) const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);

/// Gap a scripted berth leaves between two retracted faces, engine units:
/// one cell, the reach two full sleeves bridge, and the least that keeps a
/// hull's plates clear of a station port recessed in a plated surface.
pub const BERTH_GAP: f32 = 1.0;

/// Slack the gap gate allows over `capture_distance`, engine units: 0.625 m
/// past the authored 10 m, for a player's `DOCK` as much as for a scripted
/// berth. The faces are absolute f32 world positions, so a berth posed at
/// exactly `BERTH_GAP` reads up to about one ulp of its distance from the
/// origin off it: 0.0024 at the start stations (within 36,300 units, the
/// 21-seed six-role census), 0.03 at the 400,000-unit advancement saturation
/// radius. Two ulp there. A berth a half cell off still fails.
pub const GAP_SLACK: f32 = 0.0625;

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
        self.gap <= self.envelope.capture_distance + GAP_SLACK
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
    /// Ships that already hold a connection: one per ship. A station holds
    /// no [`DockedShip`] and stays a candidate while it has a free port.
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

    /// The world position and rotation of a ship root, as avian holds them.
    pub(crate) fn body_pose(&self, root: Entity) -> Option<(Vec3, Quat)> {
        self.bodies
            .get(root)
            .ok()
            .map(|(position, rotation, ..)| (position.0, rotation.0))
    }

    /// The velocity of a ship root, as avian holds it.
    pub(crate) fn body_velocity(&self, root: Entity) -> Option<Vec3> {
        self.bodies
            .get(root)
            .ok()
            .map(|(_, _, velocity, _)| velocity.0)
    }

    /// The angular velocity of a ship root, as avian holds it.
    pub(crate) fn body_spin(&self, root: Entity) -> Option<Vec3> {
        self.bodies.get(root).ok().map(|(.., spin)| spin.0)
    }

    /// Whether `root` has a free, live port.
    pub(crate) fn has_free_port(&self, root: Entity) -> bool {
        self.ports.iter().any(|(_, mount, _)| mount.0 == root)
    }

    /// The pair of `first_section` on `first_ship` and `second_section` on
    /// `second_ship`, graded whether or not it passes; `None` when either
    /// port is held, dead or not mounted on its ship, or either ship already
    /// holds a connection. What a DOCK approach flies once it has picked its
    /// pair, so a nearer pair cannot take the approach over.
    pub(crate) fn pair(
        &self,
        first_ship: Entity,
        first_section: Entity,
        second_ship: Entity,
        second_section: Entity,
    ) -> Option<DockingPair> {
        if first_ship == second_ship
            || self.docked.contains(first_ship)
            || self.docked.contains(second_ship)
        {
            return None;
        }
        let (_, first_root, first_config) = self.ports.get(first_section).ok()?;
        let (_, second_root, second_config) = self.ports.get(second_section).ok()?;
        if first_root.0 != first_ship || second_root.0 != second_ship {
            return None;
        }
        let (.., first_velocity, first_spin) = self.bodies.get(first_ship).ok()?;
        let (.., second_velocity, second_spin) = self.bodies.get(second_ship).ok()?;
        Some(judge_pair(
            first_ship,
            first_section,
            self.pose(first_section, first_ship)?,
            second_ship,
            second_section,
            self.pose(second_section, second_ship)?,
            first_config.envelope().strictest(second_config.envelope()),
            (first_velocity.0 - second_velocity.0).length(),
            (first_spin.0 - second_spin.0).length(),
        ))
    }

    /// The best pair of free ports between `first_ship` and `second_ship`
    /// that passes every gate, or `None` when none does.
    ///
    /// Gate first, then rank. Ranking the whole field and gating the winner
    /// would hide an eligible pair behind a nearer one that is facing the
    /// wrong way.
    pub fn best_candidate(&self, first_ship: Entity, second_ship: Entity) -> Option<DockingPair> {
        self.graded_pairs(first_ship, second_ship)
            .filter(|(pair, ..)| pair.is_eligible())
            .min_by(rank)
            .map(|(pair, ..)| pair)
    }

    /// The nearest pair of free ports between the two ships, eligible or not.
    ///
    /// What the docking sight draws. A pilot needs the line to line up with
    /// BEFORE the pair is eligible - that is the whole of the approach - so
    /// this one gates nothing and reports the measurements instead.
    pub fn nearest_pair(&self, first_ship: Entity, second_ship: Entity) -> Option<DockingPair> {
        self.graded_pairs(first_ship, second_ship)
            .min_by(rank)
            .map(|(pair, ..)| pair)
    }

    /// Every pair of free ports between the two ships, graded, best first by
    /// [`rank`]; empty when either ship already holds a connection.
    pub(crate) fn ranked_pairs(&self, first_ship: Entity, second_ship: Entity) -> Vec<DockingPair> {
        let mut ranked: Vec<_> = self.graded_pairs(first_ship, second_ship).collect();
        ranked.sort_by(rank);
        ranked.into_iter().map(|(pair, ..)| pair).collect()
    }

    /// The shared walk: every free port on one hull against every free port on
    /// the other, graded, each with the two section ids [`rank`] breaks ties
    /// on. Empty when either ship already holds a connection.
    fn graded_pairs(
        &self,
        first_ship: Entity,
        second_ship: Entity,
    ) -> impl Iterator<Item = (DockingPair, &str, &str)> + '_ {
        let motion = (first_ship != second_ship
            && !self.docked.contains(first_ship)
            && !self.docked.contains(second_ship))
        .then(|| {
            let (.., first_velocity, first_spin) = self.bodies.get(first_ship).ok()?;
            let (.., second_velocity, second_spin) = self.bodies.get(second_ship).ok()?;
            Some((
                (first_velocity.0 - second_velocity.0).length(),
                (first_spin.0 - second_spin.0).length(),
            ))
        })
        .flatten();
        motion
            .into_iter()
            .flat_map(move |(relative_speed, relative_spin)| {
                self.ports
                    .iter()
                    .filter(move |(_, root, _)| root.0 == first_ship)
                    .filter_map(move |(section, _, config)| {
                        Some((section, self.pose(section, first_ship)?, config))
                    })
                    .flat_map(move |(first_section, first_pose, first_config)| {
                        self.ports
                            .iter()
                            .filter(move |(_, root, _)| root.0 == second_ship)
                            .filter_map(move |(second_section, _, second_config)| {
                                let second_pose = self.pose(second_section, second_ship)?;
                                Some((
                                    judge_pair(
                                        first_ship,
                                        first_section,
                                        first_pose,
                                        second_ship,
                                        second_section,
                                        second_pose,
                                        first_config.envelope().strictest(second_config.envelope()),
                                        relative_speed,
                                        relative_spin,
                                    ),
                                    self.ids.get(first_section).map_or("", |id| id.0.as_str()),
                                    self.ids.get(second_section).map_or("", |id| id.0.as_str()),
                                ))
                            })
                    })
            })
    }
}

/// The order pairs rank in: face gap, then alignment, then the two section
/// ids.
///
/// The ids are what make the answer a FUNCTION of the world rather than of
/// query iteration order - two ports mounted symmetrically on the same hull
/// are the same distance and the same alignment away, and without the last
/// key the pair chosen would depend on archetype layout. A total order, so a
/// non-finite grade cannot break the sort.
fn rank(
    (a, a_first, a_second): &(DockingPair, &str, &str),
    (b, b_first, b_second): &(DockingPair, &str, &str),
) -> core::cmp::Ordering {
    a.gap
        .total_cmp(&b.gap)
        .then(a.opposition.total_cmp(&b.opposition))
        .then_with(|| a_first.cmp(b_first))
        .then_with(|| a_second.cmp(b_second))
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
