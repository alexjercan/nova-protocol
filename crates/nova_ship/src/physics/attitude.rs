//! How hard a hull is allowed to turn: the propulsive ceiling its computers can
//! push to, the structural ceiling its metal survives, and the lower of the two.
//!
//! The two ceilings answer different questions and bind on different ships, so
//! neither may be dropped. Change this module when the shape of the attitude
//! model changes - not to retune it, which is
//! [`LOAD_LIMIT`](nova_events::scale::LOAD_LIMIT) and the controllers' authored
//! torque.

use avian3d::{
    dynamics::rigid_body::mass_properties::bevy_heavy::{
        ComputeMassProperties3d, MassProperties3d,
    },
    prelude::*,
};
use bevy::prelude::*;
use nova_events::{prelude::LOAD_LIMIT, units::prelude::*};

/// The attitude envelope, which limit binds it, and the hull-arm helper the two
/// callers that have no rigid body to read share.
pub mod prelude {
    pub use super::{hull_mass_properties, structural_arm, AttitudeEnvelope, AttitudeLimit};
}

/// Which of the two ceilings is the one the hull actually feels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum AttitudeLimit {
    /// The computers cannot push the hull's inertia any harder. Fitting more
    /// of them raises the ceiling; this is where a capital lives.
    Torque,
    /// The hull would tear before the computers ran out. More computers change
    /// nothing; this is where everything that ships today lives.
    Structure,
}

impl AttitudeLimit {
    /// The readout word, for a build screen that has to say WHY a number is
    /// what it is.
    pub fn label(self) -> &'static str {
        match self {
            Self::Torque => "torque-limited",
            Self::Structure => "structure-limited",
        }
    }
}

/// A hull's two attitude ceilings, in rad/s2.
///
/// Both are derived, never authored: the propulsive one from the torque its
/// live computers sum to against the inertia avian measured, the structural one
/// from the global load limit against the arm from the centre of mass to the
/// furthest section. Size therefore falls out of the physics instead of being
/// legislated - a fighter is structure-bound and sharp, a capital is
/// torque-bound and a barge.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct AttitudeEnvelope {
    /// `sum(max_torque) / I`. Infinite while the hull has no measured inertia,
    /// which is the case for the first few ticks after a spawn.
    pub torque_ceiling: f32,
    /// `LOAD_LIMIT / arm`. Infinite for a hull with no arm - a point mass has
    /// nothing to tear.
    pub structural_ceiling: f32,
}

impl AttitudeEnvelope {
    /// Derive the envelope from the hull's summed computer torque, its largest
    /// principal moment of inertia, and its structural arm.
    ///
    /// The largest principal moment is the conservative axis and the one
    /// `hull_turn_rate` budgets against, so one scalar covers a hull that is
    /// much easier to roll than to yaw.
    pub fn new(total_torque: f32, angular_inertia: f32, arm: Meters) -> Self {
        let torque_ceiling = if angular_inertia > 0.0 {
            total_torque.max(0.0) / angular_inertia
        } else {
            f32::INFINITY
        };
        let arm = arm.max(Meters::ZERO);
        let structural_ceiling = if arm > Meters::ZERO {
            LOAD_LIMIT.get() / arm.get()
        } else {
            f32::INFINITY
        };
        Self {
            torque_ceiling,
            structural_ceiling,
        }
    }

    /// The lower of the two ceilings - what the hull gets at rest.
    pub fn ceiling(self) -> f32 {
        self.torque_ceiling.min(self.structural_ceiling)
    }

    /// Which ceiling binds. Ties read as [`AttitudeLimit::Structure`]: the
    /// structural one is the answer a player can do nothing about.
    pub fn binds(self) -> AttitudeLimit {
        if self.torque_ceiling < self.structural_ceiling {
            AttitudeLimit::Torque
        } else {
            AttitudeLimit::Structure
        }
    }

    /// The sustained turn rate the structure allows, in rad/s: at
    /// `sqrt(LOAD_LIMIT / arm)` the centripetal load alone spends the whole
    /// budget and the hull is committed to the turn.
    pub fn sustained_turn_rate(self) -> f32 {
        self.structural_ceiling.sqrt()
    }

    /// What is left for turning while the hull already spins at `spin` rad/s.
    ///
    /// The tangential load `alpha * r` and the centripetal load `omega^2 * r`
    /// are perpendicular components of one acceleration at the tip, so they add
    /// as a vector rather than being bounded separately: a hull already in a
    /// hard turn has spent its margin. Past the corner the centripetal load
    /// alone is over the limit, and the only correct thing left is to shed
    /// rate - so the full ceiling comes back there instead of trapping the ship
    /// in a spin it can never stop.
    pub fn available(self, spin: f32) -> f32 {
        let structural = self.structural_ceiling;
        let centripetal = spin * spin;
        let tangential = if centripetal <= structural {
            (structural * structural - centripetal * centripetal).sqrt()
        } else {
            structural
        };
        self.torque_ceiling.min(tangential)
    }
}

/// The structural arm in world UNITS: the distance from the hull's centre of
/// mass to the outer face of its furthest section.
///
/// `parts` yields each live section as its translation and rotation in the
/// hull's local frame plus its collider half extents - the same frame avian
/// reports the centre of mass in. The convention is the section's FACE along
/// the radial ray, not its centre and not its corner; on the shipped corvette
/// those three read 2.76 u, 2.25 u and 2.88 u, so the choice is worth 28 % of
/// the arm and has to be stated.
///
/// Loses sections and the arm shrinks, which raises the structural ceiling: a
/// damaged hull turns sharper than it did intact, and that is the model
/// working.
pub fn structural_arm(
    center_of_mass: Vec3,
    parts: impl IntoIterator<Item = (Vec3, Quat, Vec3)>,
) -> f32 {
    parts
        .into_iter()
        .map(|(translation, rotation, half_extents)| {
            let offset = translation - center_of_mass;
            let distance = offset.length();
            if distance <= f32::EPSILON {
                // A section sitting on the centre of mass has no radial ray of
                // its own; its own furthest face is the whole arm it offers.
                return half_extents.max_element();
            }
            // Support distance of the box along the radial ray: the box's own
            // half extents projected onto that ray, in the box's frame.
            let radial = (rotation.inverse() * (offset / distance)).abs();
            distance + half_extents.dot(radial)
        })
        .fold(0.0f32, f32::max)
}

/// Mass properties of a hull assembled from posed section colliders at their
/// authored densities, for a caller with no rigid body to read them off.
///
/// The editor's preview ship is deliberately physics-free, so it cannot ask
/// avian; this runs avian's OWN mass-property arithmetic over the same
/// colliders instead of re-deriving it, which is what keeps the build screen's
/// number and the flown hull's number the same number.
pub fn hull_mass_properties(
    parts: impl IntoIterator<Item = (Vec3, Quat, Collider, f32)>,
) -> MassProperties3d {
    parts
        .into_iter()
        .map(|(translation, rotation, collider, density)| {
            collider
                .mass_properties(density)
                .transformed_by(Isometry3d::new(translation, rotation))
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference hull: three unit cubes in a line about their centre of
    /// mass. Its arm is 1.5 u by the face rule, and every number the model was
    /// calibrated against hangs off that.
    #[test]
    fn the_reference_hull_arm_reaches_the_outer_face() {
        let parts = [-1.0f32, 0.0, 1.0]
            .map(|z| (Vec3::new(0.0, 0.0, z), Quat::IDENTITY, Vec3::splat(0.5)))
            .to_vec();
        let arm = structural_arm(Vec3::ZERO, parts);
        assert!((arm - 1.5).abs() < 1e-5, "1-1-1 arm is 1.5 u, got {arm}");
    }

    /// The arm runs from the CENTRE OF MASS, not the origin: a hull whose mass
    /// sits aft has a longer reach to its nose and a lower ceiling for it.
    #[test]
    fn the_arm_is_measured_from_the_centre_of_mass() {
        let parts = [-1.0f32, 0.0, 1.0]
            .map(|z| (Vec3::new(0.0, 0.0, z), Quat::IDENTITY, Vec3::splat(0.5)))
            .to_vec();
        let arm = structural_arm(Vec3::new(0.0, 0.0, -0.5), parts);
        assert!(
            (arm - 2.0).abs() < 1e-5,
            "arm from a shifted COM, got {arm}"
        );
    }

    /// A rotated section presents a different face to the radial ray, so the
    /// arm has to project the half extents through the section's rotation.
    #[test]
    fn a_rotated_section_projects_its_own_half_extents() {
        let long = Vec3::new(0.5, 0.5, 2.0);
        let along = structural_arm(
            Vec3::ZERO,
            [(Vec3::new(0.0, 0.0, 3.0), Quat::IDENTITY, long)],
        );
        let across = structural_arm(
            Vec3::ZERO,
            [(
                Vec3::new(0.0, 0.0, 3.0),
                Quat::from_rotation_y(core::f32::consts::FRAC_PI_2),
                long,
            )],
        );
        assert!((along - 5.0).abs() < 1e-5, "long axis radial, got {along}");
        assert!(
            (across - 3.5).abs() < 1e-5,
            "long axis across, got {across}"
        );
    }

    /// The reference hull's ceiling, in the terms the calibration was set in:
    /// 8 G over a 15 m arm, and a bang-bang flip a third of what the flat
    /// 0.5 rad/s2 model gave it.
    #[test]
    fn the_reference_hull_is_structure_bound_and_sharp() {
        let envelope = AttitudeEnvelope::new(1501.0, 2.5, Meters(15.0));
        assert_eq!(envelope.binds(), AttitudeLimit::Structure);
        assert!((envelope.ceiling() - 5.232).abs() < 1e-2, "{envelope:?}");
        let flip = 2.0 * (core::f32::consts::PI / envelope.ceiling()).sqrt();
        assert!((flip - 1.55).abs() < 0.02, "bang-bang 180 in {flip} s");
    }

    /// Size falls out for free: ten times the reference hull's arm, and the
    /// inertia that comes with it, turns the same computer's hull from
    /// structure-bound into torque-bound with no rule anywhere that says so.
    /// A capital is a barge because it is a capital.
    #[test]
    fn a_capital_binds_on_torque_instead() {
        let envelope = AttitudeEnvelope::new(1501.0, 7906.0, Meters(150.0));
        assert_eq!(envelope.binds(), AttitudeLimit::Torque);
        let flip = 2.0 * (core::f32::consts::PI / envelope.ceiling()).sqrt();
        assert!((flip - 8.14).abs() < 0.05, "bang-bang 180 in {flip} s");
    }

    /// Controllers add, and the structure stops them: doubling the torque on a
    /// torque-bound hull doubles its ceiling, and on a structure-bound hull it
    /// buys nothing at all.
    #[test]
    fn torque_stacks_until_the_structure_catches_it() {
        let one = AttitudeEnvelope::new(1501.0, 7906.0, Meters(150.0));
        let two = AttitudeEnvelope::new(3002.0, 7906.0, Meters(150.0));
        assert!(
            (two.ceiling() / one.ceiling() - 2.0).abs() < 1e-3,
            "a torque-bound hull doubles: {} -> {}",
            one.ceiling(),
            two.ceiling()
        );

        let fighter_one = AttitudeEnvelope::new(1501.0, 2.5, Meters(15.0));
        let fighter_ten = AttitudeEnvelope::new(15010.0, 2.5, Meters(15.0));
        assert_eq!(fighter_one.ceiling(), fighter_ten.ceiling());
    }

    /// The two loads add as a vector: at rest the whole budget is turning
    /// authority, at the sustained rate none of it is, and past that corner the
    /// hull gets its authority back to shed the rate with.
    #[test]
    fn a_hard_turn_spends_the_structural_margin() {
        let envelope = AttitudeEnvelope::new(f32::INFINITY, 2.5, Meters(15.0));
        let ceiling = envelope.ceiling();
        assert_eq!(envelope.available(0.0), ceiling);

        let sustained = envelope.sustained_turn_rate();
        assert!(
            envelope.available(sustained) < 1e-3,
            "at {sustained} rad/s the budget is all centripetal"
        );
        assert!(
            envelope.available(sustained * 0.5) < ceiling,
            "half the sustained rate already costs authority"
        );
        assert_eq!(
            envelope.available(sustained * 2.0),
            ceiling,
            "an over-spun hull must be able to brake"
        );
    }

    /// A hull avian has not measured yet must not read as unable to turn:
    /// zero inertia is "no propulsive limit known", so the structure answers.
    #[test]
    fn an_unmeasured_hull_falls_back_to_its_structure() {
        let envelope = AttitudeEnvelope::new(1501.0, 0.0, Meters(15.0));
        assert_eq!(envelope.binds(), AttitudeLimit::Structure);
        assert!((envelope.ceiling() - 5.232).abs() < 1e-2, "{envelope:?}");
    }

    /// The editor's collider sum has to agree with what avian would compute for
    /// the flown ship, or the build screen lies. The reference hull is the
    /// closed form: three unit cubes of mass 1 in a line is
    /// `2 * 1^2 + 3 * (1/6) = 2.5`.
    #[test]
    fn assembled_mass_properties_match_the_closed_form() {
        let parts = [-1.0f32, 0.0, 1.0]
            .map(|z| {
                (
                    Vec3::new(0.0, 0.0, z),
                    Quat::IDENTITY,
                    Collider::cuboid(1.0, 1.0, 1.0),
                    1.0,
                )
            })
            .to_vec();
        let properties = hull_mass_properties(parts);
        assert!((properties.mass - 3.0).abs() < 1e-4, "{properties:?}");
        assert!(
            properties.center_of_mass.abs_diff_eq(Vec3::ZERO, 1e-4),
            "{properties:?}"
        );
        let principal = properties.principal_angular_inertia;
        assert!(
            (principal.max_element() - 2.5).abs() < 1e-3,
            "yaw inertia is 2.5, got {principal}"
        );
    }
}
