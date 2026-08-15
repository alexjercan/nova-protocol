//! What a turret can BEAR ON: the slice of sky its joint tree can actually
//! swing the muzzle through, derived once from the tree at spawn.

use bevy::prelude::*;

use super::*;

/// Elevation band a turret's muzzle can reach, in the turret section's own
/// frame. Built from the joint tree by [`TurretSectionArc::from_tree`] and
/// carried on the turret section entity.
///
/// Whether an articulated chain can reach a given direction is in general an
/// inverse-kinematics question. Every shipped turret has the same shape though -
/// one UNLIMITED traverse hinge carrying one LIMITED elevation hinge - and that
/// shape has a closed form: because traversing does not change a direction's
/// elevation off the traverse plane, the reachable set is exactly the
/// directions whose elevation lies inside the elevation hinge's limits.
///
/// That is not a simplification of the real problem, it IS the real problem. A
/// PDC is bolted to a hull, and the reason it cannot cover the whole sky is the
/// depression floor that stops its barrel swinging back into its own ship. A
/// turret told to engage something under the hull contributes nothing while a
/// torpedo it COULD have hit goes unengaged.
///
/// A tree that does not match the shape yields `None` and gets no component, so
/// consumers treat it as able to bear anywhere. Fail-open is deliberate: a mod
/// turret with an exotic chain keeps the pre-arc behavior rather than silently
/// refusing to defend its ship.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct TurretSectionArc {
    /// Unit axis, turret-local, that the barrel rises TOWARD at a positive
    /// elevation angle. Elevation of a direction `d` is `asin(d . up)`.
    up: Vec3,
    /// Lowest reachable elevation, radians (negative = below the mount plane).
    min: f32,
    /// Highest reachable elevation, radians.
    max: f32,
}

/// How closely the elevation hinge's derived "up" must line up with the
/// traverse axis for the closed form to hold. Below this the two hinges are not
/// the traverse/elevation pair the solve assumes, and the tree is unrecognised.
const ARC_AXIS_ALIGNMENT: f32 = 0.99;

impl TurretSectionArc {
    /// The arc of a turret built from `root`, or `None` when the tree is not
    /// the traverse-then-elevation shape the closed form covers.
    pub fn from_tree(root: &TurretJoint) -> Option<Self> {
        let mut hinges = Vec::new();
        if !hinges_to_first_muzzle(root, &mut hinges) {
            return None;
        }
        // Exactly two hinges: an unlimited traverse, then a limited elevation.
        let [(traverse, traverse_min, traverse_max), (elevation, min, max)] = hinges[..] else {
            return None;
        };
        if traverse_min.is_some() || traverse_max.is_some() {
            return None;
        }
        let traverse = traverse.try_normalize()?;
        let elevation = elevation.try_normalize()?;

        // The barrel's REST direction is the section's own -Z: every joint in a
        // shipped tree is a pure offset, so the muzzle inherits the section's
        // orientation before any hinge turns. Rotating that rest direction
        // about the elevation axis by a positive angle moves it toward
        // `elevation x rest`, which is therefore the axis elevation is measured
        // along.
        let up = elevation.cross(Vec3::NEG_Z).try_normalize()?;
        // The closed form only holds while traversing leaves elevation alone,
        // which needs the two axes parallel.
        if up.dot(traverse).abs() < ARC_AXIS_ALIGNMENT {
            return None;
        }

        // An unauthored limit is unlimited, and elevation saturates at
        // straight up or straight down whatever the hinge says.
        let limit = std::f32::consts::FRAC_PI_2;
        Some(Self {
            up,
            min: min.unwrap_or(-limit).clamp(-limit, limit),
            max: max.unwrap_or(limit).clamp(-limit, limit),
        })
    }

    /// Whether a turret whose section sits at `section_rotation` can swing its
    /// muzzle onto `direction` (world space, need not be normalized), with
    /// `margin` radians shaved off both ends of the band.
    ///
    /// Pass a margin when ACQUIRING and zero when holding: a target sitting
    /// exactly on the depression floor would otherwise be picked up and dropped
    /// on alternate frames as it drifts across the limit.
    pub fn bears_on(&self, section_rotation: Quat, direction: Vec3, margin: f32) -> bool {
        let Some(direction) = direction.try_normalize() else {
            // Target on the muzzle: nothing to slew to.
            return true;
        };
        let local = section_rotation.inverse() * direction;
        let elevation = local.dot(self.up).clamp(-1.0, 1.0).asin();
        (self.min + margin..=self.max - margin).contains(&elevation)
    }
}

/// Collect the hinge (axis, min, max) of every articulated joint on the path
/// from `joint` down to the FIRST muzzle, in root-to-muzzle order. Returns
/// whether a muzzle was found below `joint`.
fn hinges_to_first_muzzle(
    joint: &TurretJoint,
    hinges: &mut Vec<(Vec3, Option<f32>, Option<f32>)>,
) -> bool {
    let mark = hinges.len();
    if let Some(axis) = joint.axis {
        hinges.push((axis, joint.min, joint.max));
    }
    if joint.muzzle.is_some() {
        return true;
    }
    for child in &joint.children {
        if hinges_to_first_muzzle(child, hinges) {
            return true;
        }
    }
    // Not on the path after all: unwind what this branch contributed.
    hinges.truncate(mark);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Elevation of a direction relative to the mount plane, for readable
    /// test bearings: `up(0)` is level, `up(90)` straight up.
    fn bearing(degrees: f32) -> Vec3 {
        let radians = degrees.to_radians();
        Vec3::new(0.0, radians.sin(), -radians.cos())
    }

    #[test]
    fn the_shipped_tree_yields_its_depression_floor_and_elevation_ceiling() {
        let arc = TurretSectionArc::from_tree(&TurretSectionConfig::default().root)
            .expect("the shipped yaw/pitch tree is the recognised shape");

        // The default turret depresses 10 degrees and elevates to 90.
        assert!(
            (arc.min + std::f32::consts::PI / 18.0).abs() < 1e-5,
            "{arc:?}"
        );
        assert!(
            (arc.max - std::f32::consts::FRAC_PI_2).abs() < 1e-5,
            "{arc:?}"
        );
        // A turret standing on a hull rises toward the hull's up.
        assert!((arc.up - Vec3::Y).length() < 1e-5, "{arc:?}");
    }

    #[test]
    fn a_hull_mounted_turret_cannot_bear_under_its_own_ship() {
        let arc = TurretSectionArc::from_tree(&TurretSectionConfig::default().root).unwrap();
        let level = Quat::IDENTITY;

        assert!(arc.bears_on(level, bearing(0.0), 0.0), "dead level");
        assert!(arc.bears_on(level, bearing(80.0), 0.0), "high overhead");
        assert!(arc.bears_on(level, bearing(-5.0), 0.0), "just under level");
        assert!(
            !arc.bears_on(level, bearing(-30.0), 0.0),
            "well under the hull: the barrel would be aiming into its own ship"
        );
        assert!(
            !arc.bears_on(level, bearing(-90.0), 0.0),
            "straight down is the extreme of the same case"
        );
    }

    #[test]
    fn the_arc_turns_with_the_turrets_mount() {
        // The same world bearing is reachable or not depending on which way up
        // the turret is bolted - a mount on the ship's belly covers the sky
        // BELOW it. This is what makes the check per-turret rather than
        // per-ship.
        let arc = TurretSectionArc::from_tree(&TurretSectionConfig::default().root).unwrap();
        let under = Vec3::new(0.0, -1.0, -1.0); // 45 degrees below world level

        assert!(
            !arc.bears_on(Quat::IDENTITY, under, 0.0),
            "a turret standing upright cannot reach it"
        );
        assert!(
            arc.bears_on(Quat::from_rotation_z(std::f32::consts::PI), under, 0.0),
            "the same turret mounted upside down covers exactly that sky"
        );
    }

    #[test]
    fn the_acquire_margin_shrinks_the_band_at_both_ends() {
        let arc = TurretSectionArc::from_tree(&TurretSectionConfig::default().root).unwrap();
        let margin = 5.0f32.to_radians();

        assert!(arc.bears_on(Quat::IDENTITY, bearing(-8.0), 0.0), "held");
        assert!(
            !arc.bears_on(Quat::IDENTITY, bearing(-8.0), margin),
            "not acquired: a target this close to the floor drifts off it"
        );
        assert!(
            !arc.bears_on(Quat::IDENTITY, bearing(88.0), margin),
            "and the same at the ceiling"
        );
    }

    #[test]
    fn an_unrecognised_tree_has_no_arc() {
        // Fail-open: a mod turret whose chain the closed form does not cover
        // must keep the pre-arc behavior rather than silently refuse to defend
        // its ship. Three ways to be unrecognised.
        let hinge = |axis: Option<Vec3>, children: Vec<TurretJoint>| TurretJoint {
            offset: Vec3::ZERO,
            axis,
            speed: 1.0,
            min: None,
            max: None,
            render_mesh: None,
            render_mesh_transform: None,
            muzzle: None,
            children,
        };
        let muzzle = || TurretJoint {
            muzzle: Some(MuzzleConfig {
                fire_rate: 1.0,
                muzzle_effect: None,
            }),
            ..hinge(None, vec![])
        };

        assert_eq!(
            TurretSectionArc::from_tree(&hinge(Some(Vec3::Y), vec![muzzle()])),
            None,
            "one hinge is not a traverse/elevation pair"
        );
        assert_eq!(
            TurretSectionArc::from_tree(&hinge(
                Some(Vec3::Y),
                vec![hinge(
                    Some(Vec3::X),
                    vec![hinge(Some(Vec3::Z), vec![muzzle()])]
                )]
            )),
            None,
            "three hinges is a chain the closed form does not solve"
        );
        assert_eq!(
            TurretSectionArc::from_tree(&hinge(
                Some(Vec3::Y),
                vec![hinge(Some(Vec3::Y), vec![muzzle()])]
            )),
            None,
            "two PARALLEL hinges never change elevation at all"
        );
        assert_eq!(
            TurretSectionArc::from_tree(&hinge(Some(Vec3::Y), vec![hinge(Some(Vec3::X), vec![])])),
            None,
            "a tree with no muzzle has no bearing to solve"
        );
    }

    #[test]
    fn a_limited_traverse_is_not_the_recognised_shape() {
        // The closed form leans on the traverse being free; a limited one
        // needs an azimuth check this does not do, so it fails open instead of
        // reporting an arc that is wrong on one side.
        let mut root = TurretSectionConfig::default().root;
        fn limit_first_hinge(joint: &mut TurretJoint) -> bool {
            if joint.axis.is_some() {
                joint.min = Some(-1.0);
                joint.max = Some(1.0);
                return true;
            }
            joint.children.iter_mut().any(limit_first_hinge)
        }
        assert!(limit_first_hinge(&mut root));
        assert_eq!(TurretSectionArc::from_tree(&root), None);
    }
}
