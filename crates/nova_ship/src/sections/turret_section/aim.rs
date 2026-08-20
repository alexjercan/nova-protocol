//! Turret aiming: the lead-intercept solve and the per-frame hinge pass that
//! swings each articulated joint toward the aim point.

use avian3d::prelude::*;
use bevy::prelude::*;

use super::*;
use crate::physics::prelude::rigid_body_point_velocity;

/// System set for the PostUpdate aim chain (intercept solve + rotator
/// targets), so HUD consumers can order same-frame readers after it (the
/// turret lead pips do).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TurretSectionAimSystems;

/// Half the beam of a shipped corvette, in world units: the cargoa's pods span
/// x -1.6..1.6 (the hull is 4.9 u long by 3.2 u across by 1.6 u deep). What a
/// round has to land inside to hit a ship at all.
pub const HULL_HIT_RADIUS: f32 = 1.6;

/// The CLOSE edge of a gunfight, in world units. Nova's scale is 100 u = 1 km
/// and gunfights are fought at 1-2 km, so this is the range at which the cone
/// below is widest.
pub const CLOSE_ENGAGEMENT_RANGE: f32 = 100.0;

/// How far off its aim point a muzzle may point and still count as ON it:
/// 0.016 rad, 0.92 degrees.
///
/// DERIVED, not chosen. A round leaving the muzzle `e` off the aim point misses
/// that point laterally by `range * sin(e)`, so the widest error that still puts
/// the round on the thing being aimed at is `asin(hit_radius / range)`. At this
/// size an angle and its sine agree to within 1e-6 rad, so the RATIO is the
/// angle and the constant needs no transcendental to state;
/// [`the_on_target_cone_is_what_still_lands_on_a_hull`] pins that it is.
///
/// The CLOSE edge of the band is deliberate, and it is where the slack comes
/// from: it is the LOOSEST angle that still means "this round lands on the
/// thing". Graded at the far edge instead - the PDC's 180 u fire gate - the same
/// arithmetic reads 0.51 deg, and a round fired at the full 0.92 deg from there
/// still passes only 2.9 u wide, inside a 3.2 u beam.
///
/// The floor under it is what a barrel actually holds. A converged turret
/// tracking a crossing target settles well inside this cone (measured on the
/// turret range, task 20260816-144947), so a mount that is genuinely ON its
/// target fires, and one that is slewing - or that cannot depress far enough to
/// bear at all - does not.
pub const TURRET_ON_TARGET_RAD: f32 = HULL_HIT_RADIUS / CLOSE_ENGAGEMENT_RANGE;

/// The angle (radians) between where a muzzle POINTS and where it should point
/// to put a round on `aim`. Zero for a degenerate line (the aim point sitting on
/// the muzzle): there is no bearing to be wrong about.
///
/// `forward` need not be normalized; `muzzle` and `aim` are world-space points.
pub fn muzzle_aim_error(forward: Vec3, muzzle: Vec3, aim: Vec3) -> f32 {
    let (Some(forward), Some(to_aim)) = (forward.try_normalize(), (aim - muzzle).try_normalize())
    else {
        return 0.0;
    };
    forward.dot(to_aim).clamp(-1.0, 1.0).acos()
}

/// Whether a muzzle at `muzzle` pointing along `forward` is on `aim` within
/// [`TURRET_ON_TARGET_RAD`] - the single "may this barrel shoot" predicate, used
/// by the section fire path and by the AI trigger.
///
/// This is the whole reachability rule as well as the alignment one: a mount
/// whose hinges cannot swing onto its target never converges, so it is never
/// within the cone, so it never fires. [`TurretSectionArc`] stays the ACQUIRE-
/// time question (never assign a mount a target it cannot reach); this is the
/// fire-time one, and it needs no second reachability test to answer.
pub fn muzzle_on_target(forward: Vec3, muzzle: Vec3, aim: Vec3) -> bool {
    muzzle_aim_error(forward, muzzle, aim) <= TURRET_ON_TARGET_RAD
}

/// The point a turret should aim at to hit a moving target: the intercept point,
/// where a bullet leaving the muzzle at `projectile_speed` meets the target moving
/// at `target_vel`. Solves `|(target - shooter) + target_vel*t| = projectile_speed*t`
/// for the smallest positive time-to-intercept `t` and returns `target + target_vel*t`.
///
/// The solve assumes the bullet travels at `projectile_speed` from the shooter in
/// the frame `target_vel` is expressed in. Because bullets inherit the shooter's
/// muzzle velocity on launch, the caller passes the target velocity RELATIVE to
/// the shooter (see `update_turret_aim_point`); pass a world velocity only for a
/// shooter that is truly at rest.
///
/// Falls back to the target's current position when there is no valid intercept -
/// a target with no relative motion resolves to the target itself, and a target
/// too fast to catch (or receding faster than the bullet) has no positive solution,
/// so the turret simply aims where the target is now.
///
/// `pub(crate)` because it is the solution the torpedo's terminal weave exists
/// to break: the weave measurement in `torpedo_section` scores itself against
/// THIS solve rather than a copy of it that could drift away from what turrets
/// actually fire on.
pub(crate) fn lead_intercept_point(
    shooter: Vec3,
    target: Vec3,
    target_vel: Vec3,
    projectile_speed: f32,
) -> Vec3 {
    let to_target = target - shooter;
    // a*t^2 + b*t + c = 0
    let a = target_vel.length_squared() - projectile_speed * projectile_speed;
    let b = 2.0 * to_target.dot(target_vel);
    let c = to_target.length_squared();

    let time_to_intercept = if a.abs() < 1e-4 {
        // Target speed ~ bullet speed: the equation is linear in t.
        (b.abs() > 1e-6).then(|| -c / b)
    } else {
        let discriminant = b * b - 4.0 * a * c;
        (discriminant >= 0.0)
            .then(|| {
                let sqrt_d = discriminant.sqrt();
                [(-b + sqrt_d) / (2.0 * a), (-b - sqrt_d) / (2.0 * a)]
                    .into_iter()
                    .filter(|&t| t > 0.0)
                    .reduce(f32::min)
            })
            .flatten()
    };

    match time_to_intercept {
        Some(t) if t > 0.0 && t.is_finite() => target + target_vel * t,
        _ => target,
    }
}

/// Resolve each turret's lead intercept point into [`TurretSectionAimPoint`] from
/// its target position, target velocity, and the bullet `muzzle_speed`. Runs
/// before the yaw/pitch systems, which steer toward this point.
///
/// The intercept is solved in the SHOOTER's frame: bullets inherit the full
/// muzzle point velocity at fire time (`shoot_spawn_projectile` adds ship
/// linear velocity plus the angular swing at the muzzle), so the solve uses
/// the target velocity RELATIVE to that same muzzle point velocity. Aiming
/// the barrel at the shooter-frame intercept and adding the inherited
/// velocity on launch lands the bullet on the true world-frame intercept:
/// dir*s*t = (target - muzzle) + (v_target - v_muzzle)*t. Solving in the
/// world frame instead makes every shot drift off by the shooter's own
/// motion.
// `pub` so the turret-lead pip regression in `nova_hud` can register the real
// aim system with its production set constraints (the full TurretSectionPlugin
// drags render-material plugins into headless tests). Crossing the HUD seam is
// what widened it past `pub(crate)`.
pub fn update_turret_aim_point(
    mut q_turret: Query<
        (
            &TurretSectionTargetInput,
            &TurretSectionTargetVelocity,
            &TurretSectionConfigHelper,
            &TurretSectionMuzzleEntity,
            &ChildOf,
            &mut TurretSectionAimPoint,
        ),
        With<TurretSectionMarker>,
    >,
    // Fresh render-clock poses: this runs BEFORE this frame's transform
    // propagation (see the registration comment), so GlobalTransform still
    // holds last frame - TransformHelper composes the eased poses the frame
    // will render with. The pip therefore marks the intercept as the player
    // SEES it; the physical bullet spawns from the raw pose (sub-tick
    // apart, see shoot_spawn_projectile).
    transform_helper: TransformHelper,
    q_spaceship: Query<
        (&LinearVelocity, &AngularVelocity, &ComputedCenterOfMass),
        With<SpaceshipRootMarker>,
    >,
) {
    for (
        target_input,
        target_velocity,
        config,
        TurretSectionMuzzleEntity(muzzle),
        ChildOf(spaceship),
        mut aim_point,
    ) in &mut q_turret
    {
        let Some(target_pos) = **target_input else {
            **aim_point = None;
            continue;
        };
        let Ok(muzzle_transform) = transform_helper.compute_global_transform(*muzzle) else {
            continue;
        };
        let muzzle_pos = muzzle_transform.translation();

        // The same muzzle point velocity the bullet will inherit on launch
        // (same COM lift frame as the muzzle pose above). A shooter without
        // physics components (test rigs) inherits nothing.
        let shooter_velocity = q_spaceship
            .get(*spaceship)
            .ok()
            .zip(transform_helper.compute_global_transform(*spaceship).ok())
            .map(|((lin_vel, ang_vel, center), ship_transform)| {
                rigid_body_point_velocity(
                    **lin_vel,
                    **ang_vel,
                    ship_transform.transform_point(**center),
                    muzzle_pos,
                )
            })
            .unwrap_or(Vec3::ZERO);

        **aim_point = Some(lead_intercept_point(
            muzzle_pos,
            target_pos,
            **target_velocity - shooter_velocity,
            config.muzzle_speed,
        ));
    }
}

/// The signed angle (radians) that rotates `from` onto `to` about `axis`,
/// measured in the plane perpendicular to `axis`. `axis` is assumed normalized.
///
/// Near-antiparallel `from`/`to` (target dead behind the barrel) is a rotation
/// singularity: either half-turn is valid and the cross product carries no
/// reliable sign, so a naive `atan2` dithers frame to frame and the hinge freezes
/// in place ("stuck, won't turn around"). When the vectors are within ~1.6 deg of
/// opposite, commit to a deterministic +pi half-turn so the joint swings around;
/// once it moves off the singularity the normal solve resumes and finishes the
/// turn the short way.
fn signed_angle_about(from: Vec3, to: Vec3, axis: Vec3) -> f32 {
    let from = from.normalize();
    let to = to.normalize();
    let c = from.cross(to).dot(axis);
    let d = from.dot(to);
    if d < -0.9996 {
        return std::f32::consts::PI;
    }
    c.atan2(d)
}

/// Per-second decay rate of each hinge's CCD aim error. The step target is
/// `output + gain * delta` with `gain = 1 - exp(-AIM_CORRECTION_RATE * dt)`,
/// NOT the full `output + delta`, for two reasons:
///
/// - DAMPING. A hinge solves its `delta` assuming every OTHER joint holds
///   still, but a turret's joints are coupled (yaw and pitch both swing the
///   offset muzzle), so applying each full correction at once overshoots the
///   joint solution and settles into a visible limit cycle - the barrel shakes
///   a few degrees around the aim. Under-correcting makes the coupled system
///   converge monotonically instead of ringing. Large errors are still
///   rate-limited by `SmoothLookRotation` (not this), so a slew stays
///   responsive; the decay only shapes the settle.
/// - FRAMERATE INVARIANCE. A flat per-frame gain decays the error per FRAME,
///   so residual tracking lag scaled with frame time (~0.43 deg at 60 fps vs
///   ~1.8 deg at 14 fps on a crossing target) and sat above the
///   [`TURRET_ON_TARGET_RAD`] fire gate exactly when the machine struggled
///   (task 20260816-184718). The exponential form decays the same error
///   fraction per unit TIME at any frame rate.
///
/// DERIVED from the shipped per-frame gain, not retuned: the damp shipped as
/// a flat 0.35 of the error per frame, tuned at 60 fps (playtest-lowered from
/// 0.5 for shake margin). `-ln(1 - 0.35) * 60 = 25.847` reproduces exactly
/// that fraction at dt = 1/60, so the 60 fps feel is unchanged. Lower the
/// rate if a deep chain rings.
const AIM_CORRECTION_RATE: f32 = 25.847;

/// Below this per-frame correction (radians, ~0.23 deg) a hinge is treated as
/// ON target and HOLDS (target = current output) instead of chasing sub-degree
/// residuals - stops the micro-jitter that the lead-point feedback (the aim
/// point moves with the muzzle it is aiming) would otherwise sustain forever.
const AIM_DEADBAND_RAD: f32 = 0.004;

/// How close the muzzle forward may come to a hinge's own axis before that
/// hinge is treated as being AT ITS POLE. The value is the sine of the angle
/// between them (~0.06 deg), which is what the in-plane muzzle component
/// measures. Below it the component carries no usable heading - it is pure
/// rounding noise - so the solver falls back to the joint's last measured
/// heading instead of reading a direction out of the noise.
const AIM_HINGE_POLE_SIN: f32 = 1e-3;

/// Generic aim: a Jacobi per-frame hinge-CCD pass over each turret's muzzle
/// chain (one step per articulated joint, from the muzzle up to the root). Each
/// articulated joint nudges its [`SmoothLookRotationTarget`] toward the angle
/// that swings the muzzle forward (-Z) at the aim point, decomposed in that
/// joint's own hinge plane and DAMPED by [`AIM_CORRECTION_RATE`] so coupled
/// joints do not overshoot into a shake; the [`SmoothLookRotation`] controller
/// rate-limits and clamps the visible motion. Basis-independent, so it reduces
/// to the old yaw/pitch behavior for the Y/X chain and solves arbitrary trees.
///
/// Same fresh-pose composition as [`update_turret_aim_point`]: runs BEFORE this
/// frame's transform propagation, so poses are composed via [`TransformHelper`].
///
/// A hinge whose muzzle forward lines up with its OWN axis is at a pole: the
/// joint can no longer see where it points, because rotating it does not move
/// the muzzle direction at all. The elevation limit of a hull-mounted turret
/// sits exactly on such a pole (barrel straight up, yaw axis vertical), and a
/// pinned pitch hinge holds the chain there, so the pole is a trap rather than a
/// point the solve passes through: yaw is the only joint that can fix a heading
/// error, and it went blind. Each articulated joint therefore remembers the last
/// heading it could measure ([`TurretJointAimHeading`]) and steers by that while
/// at the pole, which turns it onto the target's bearing until the pitch demand
/// drops back under the limit and the normal solve resumes.
pub(super) fn update_turret_target_joints_system(
    time: Res<Time>,
    q_turret: Query<
        (&TurretSectionAimPoint, &TurretSectionMuzzles),
        (With<TurretSectionMarker>, Without<SectionInactiveMarker>),
    >,
    q_child_of: Query<&ChildOf>,
    q_joint: Query<(&TurretJointMarker, &Transform)>,
    mut q_target_mut: Query<&mut SmoothLookRotationTarget>,
    mut q_heading: Query<&mut TurretJointAimHeading>,
    q_output: Query<&SmoothLookRotationOutput>,
    transform_helper: TransformHelper,
) {
    // The error fraction each hinge corrects this frame; dt 0 (startup frame)
    // holds every joint. See AIM_CORRECTION_RATE for the derivation.
    let gain = 1.0 - (-AIM_CORRECTION_RATE * time.delta_secs()).exp();
    for (aim_point, muzzles) in &q_turret {
        let Some(target) = **aim_point else {
            continue;
        };

        // Steer EVERY muzzle of the turret at the shared aim point. Shared
        // joints (a twin barrel's common yaw/pitch) are
        // written by each muzzle's pass and agree for symmetric barrels; a
        // muzzle's private joints refine only its own chain.
        for &muzzle in &muzzles.0 {
            let Ok(muzzle_transform) = transform_helper.compute_global_transform(muzzle) else {
                error!(
                    "update_turret_target_joints_system: muzzle {:?} has no computable pose",
                    muzzle
                );
                continue;
            };
            let m = muzzle_transform.translation();
            let d: Vec3 = muzzle_transform.forward().into();
            if target == m {
                continue;
            }

            // Walk from the muzzle up to the turret root via ChildOf, collecting
            // the articulated joints along the chain. A single Jacobi pass: every
            // joint steps from the CURRENT pose this frame.
            let mut chain = muzzle;
            while let Ok(&ChildOf(parent)) = q_child_of.get(chain) {
                // Only walk while the current node is a turret joint; the parent
                // of the root joint is the turret section entity (not a joint),
                // which stops the walk.
                let Ok((marker, joint_transform)) = q_joint.get(chain) else {
                    break;
                };

                if let Some(a_local) = marker.axis {
                    // Parent global pose, then the joint's pre-rotation frame F:
                    // parent orientation + this joint's origin (offset applied).
                    if let Ok(parent_global) = transform_helper.compute_global_transform(parent) {
                        let f = parent_global
                            * Transform::from_translation(joint_transform.translation);
                        let w2j = f.to_matrix().inverse();

                        let ml = w2j.transform_point3(m);
                        let dl = w2j.transform_vector3(d);
                        let tl = w2j.transform_point3(target);
                        let a = a_local.normalize();

                        let des = tl - ml;
                        let d_perp = dl - a * dl.dot(a);
                        let t_perp = des - a * des.dot(a);
                        let out = q_output.get(chain).map(|o| **o).unwrap_or(0.0);

                        // The joint's heading in its own hinge plane. Off the
                        // pole the muzzle forward gives it, and the joint keeps
                        // a copy in its post-rotation frame (so the copy turns
                        // WITH the joint and stays valid at any angle). At the
                        // pole that copy is all there is.
                        let mut stored = q_heading.get_mut(chain).ok();
                        let heading = if d_perp.length() > AIM_HINGE_POLE_SIN {
                            let heading = d_perp.normalize();
                            if let Some(stored) = stored.as_mut() {
                                stored.0 = Some(Quat::from_axis_angle(a, -out) * heading);
                            }
                            Some(heading)
                        } else {
                            stored
                                .as_ref()
                                .and_then(|stored| stored.0)
                                .map(|heading| Quat::from_axis_angle(a, out) * heading)
                        };

                        // A target ON the hinge axis has no bearing to steer to.
                        if let Some(heading) = heading.filter(|_| t_perp.length() > 1e-6) {
                            let delta = signed_angle_about(heading, t_perp, a);
                            if let Ok(mut target_angle) = q_target_mut.get_mut(chain) {
                                // Damp the correction so coupled joints converge
                                // instead of ringing; hold once aimed (deadband).
                                **target_angle = if delta.abs() <= AIM_DEADBAND_RAD {
                                    out
                                } else {
                                    out + gain * delta
                                };
                            }
                        }
                    }
                }

                chain = parent;
            }
        }
    }
}

/// Apply each articulated joint's controller output onto its own transform's
/// rotation (fixed joints keep identity). One generic sync, replacing the
/// per-yaw/per-pitch pair - the joint entity carries both the offset transform
/// and the controller, so the hinge rotation is written back onto the SAME
/// transform.
pub(super) fn sync_turret_joint_rotation(
    mut q_joint: Query<(
        &TurretJointMarker,
        &SmoothLookRotationOutput,
        &mut Transform,
    )>,
) {
    for (marker, output, mut transform) in &mut q_joint {
        if let Some(axis) = marker.axis {
            transform.rotation = Quat::from_axis_angle(axis.normalize(), **output);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        super::{config::default_joint_speed, test_support::*},
        *,
    };

    /// The on-target cone is arithmetic, not a taste call, so the arithmetic is
    /// pinned: the constant IS `asin(hull half-beam / close engagement range)`,
    /// and the miss it permits at the far edge of the band still lands on a
    /// hull. Re-derive both numbers in the SAME commit as any change to hull
    /// dimensions or engagement ranges.
    #[test]
    fn the_on_target_cone_is_what_still_lands_on_a_hull() {
        let derived = (HULL_HIT_RADIUS / CLOSE_ENGAGEMENT_RANGE).asin();
        assert!(
            (TURRET_ON_TARGET_RAD - derived).abs() < 1e-6,
            "the cone must be asin({HULL_HIT_RADIUS} / {CLOSE_ENGAGEMENT_RANGE}) \
             = {derived} rad, got {TURRET_ON_TARGET_RAD}"
        );

        // The far edge of a gunfight: the PDC's own fire gate, muzzle_speed *
        // projectile_lifetime * AI_FIRE_RANGE_FACTOR = 100 * 2.0 * 0.9.
        let config = TurretSectionConfig::default();
        let far = config.muzzle_speed * config.projectile_lifetime * 0.9;
        let miss = far * TURRET_ON_TARGET_RAD.sin();
        assert!(
            miss < HULL_HIT_RADIUS * 2.0,
            "a round fired at the full tolerance from the far edge ({far} u) \
             misses by {miss} u, which must still be inside a hull's \
             {} u beam",
            HULL_HIT_RADIUS * 2.0
        );
    }

    #[test]
    fn a_muzzle_pointed_across_its_own_ship_is_never_on_target() {
        // The owner's complaint, as the predicate sees it: a barrel that cannot
        // swing round to the far side reads tens of degrees off, nowhere near
        // the cone, whatever else the fire path thinks.
        let muzzle = Vec3::ZERO;
        let aim = Vec3::new(0.0, 0.0, -100.0);
        assert!(muzzle_on_target(Vec3::NEG_Z, muzzle, aim), "dead on");
        assert!(
            !muzzle_on_target(Vec3::X, muzzle, aim),
            "a mount pinned across the hull is 90 deg off"
        );
        // And the boundary: half a degree in is on, two degrees out is off.
        let off = |degrees: f32| Quat::from_rotation_y(degrees.to_radians()) * Vec3::NEG_Z;
        assert!(muzzle_on_target(off(0.5), muzzle, aim), "settled tracking");
        assert!(!muzzle_on_target(off(2.0), muzzle, aim), "mid-slew");
    }

    #[test]
    fn lead_of_a_stationary_target_is_the_target() {
        // No motion, no lead: aim straight at the target.
        let target = Vec3::new(0.0, 0.0, -100.0);
        let aim = lead_intercept_point(Vec3::ZERO, target, Vec3::ZERO, 100.0);
        assert!(
            (aim - target).length() < 1e-3,
            "expected the target itself, got {aim:?}"
        );
    }

    #[test]
    fn lead_intercepts_a_crossing_target() {
        // Shooter at origin; target 100 ahead crossing at +X. The intercept point
        // must be one a bullet at muzzle_speed and the target reach at the SAME
        // time - that is what "leading" means. It must also sit ahead of the target
        // in its direction of travel (+X).
        let shooter = Vec3::ZERO;
        let target = Vec3::new(0.0, 0.0, -100.0);
        let target_vel = Vec3::new(30.0, 0.0, 0.0);
        let speed = 100.0;

        let aim = lead_intercept_point(shooter, target, target_vel, speed);

        assert!(
            aim.x > 0.1,
            "intercept should lead a +X crosser, got {aim:?}"
        );
        // Consistency: at the bullet's flight time, the target is exactly there.
        let flight_time = (aim - shooter).length() / speed;
        let target_future = target + target_vel * flight_time;
        assert!(
            (target_future - aim).length() < 1e-2,
            "bullet and target should meet: aim {aim:?}, target at t {target_future:?}"
        );
    }

    #[test]
    fn lead_falls_back_when_the_target_cannot_be_caught() {
        // Target receding faster than the bullet: no positive intercept, so aim at
        // where it is now rather than returning a garbage/NaN point.
        let target = Vec3::new(0.0, 0.0, -50.0);
        let target_vel = Vec3::new(0.0, 0.0, -200.0); // fleeing at 200, bullet only 100
        let aim = lead_intercept_point(Vec3::ZERO, target, target_vel, 100.0);
        assert!(aim.is_finite());
        assert!(
            (aim - target).length() < 1e-3,
            "expected fallback to the target, got {aim:?}"
        );
    }

    /// World for aim-point tests: a ship with physics state, one turret with
    /// its muzzle 100 m ahead of a target-bearing setup. Returns (turret,
    /// muzzle position).
    fn aim_point_world(ship_velocity: Vec3, muzzle_pos: Vec3) -> (bevy::ecs::world::World, Entity) {
        let mut world = World::new();
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                Transform::IDENTITY,
                LinearVelocity(ship_velocity),
                AngularVelocity(Vec3::ZERO),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        let muzzle = world
            .spawn((
                TurretSectionBarrelMuzzleMarker,
                Transform::from_translation(muzzle_pos),
            ))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
                TurretSectionConfigHelper(TurretSectionConfig::default()),
                TurretSectionMuzzleEntity(muzzle),
                TurretSectionAimPoint(None),
                ChildOf(ship),
            ))
            .id();
        (world, turret)
    }

    fn aim_point(world: &mut World, turret: Entity) -> Option<Vec3> {
        use bevy::ecs::system::RunSystemOnce;
        world.run_system_once(update_turret_aim_point).unwrap();
        **world.entity(turret).get::<TurretSectionAimPoint>().unwrap()
    }

    #[test]
    fn formation_flight_needs_no_lead() {
        // Shooter and target flying in formation (equal velocities): the
        // bullet inherits the shared motion on launch, so the correct aim is
        // the target itself. Solving in the world frame instead would lead a
        // target that, relatively, is not moving - the shots that never hit
        // from a moving ship.
        let velocity = Vec3::new(40.0, 0.0, -10.0);
        let target = Vec3::new(0.0, 0.0, -100.0);
        let (mut world, turret) = aim_point_world(velocity, Vec3::ZERO);
        {
            let mut entity = world.entity_mut(turret);
            **entity.get_mut::<TurretSectionTargetInput>().unwrap() = Some(target);
            **entity.get_mut::<TurretSectionTargetVelocity>().unwrap() = velocity;
        }

        let aim = aim_point(&mut world, turret).expect("aim point computed");

        assert!(
            (aim - target).length() < 1e-2,
            "formation flight must aim at the target itself, got {aim:?}"
        );
    }

    #[test]
    fn moving_shooter_aims_retrograde_of_its_own_motion() {
        // Static target, shooter strafing +X: the bullet inherits +X on
        // launch, so the barrel must point BEHIND the shooter's motion for
        // the drift to carry the bullet onto the target.
        let target = Vec3::new(0.0, 0.0, -100.0);
        let (mut world, turret) = aim_point_world(Vec3::new(30.0, 0.0, 0.0), Vec3::ZERO);
        {
            let mut entity = world.entity_mut(turret);
            **entity.get_mut::<TurretSectionTargetInput>().unwrap() = Some(target);
        }

        let aim = aim_point(&mut world, turret).expect("aim point computed");

        assert!(
            aim.x < -0.1,
            "a +X shooter must aim -X of the target, got {aim:?}"
        );
        // Consistency: bullet (barrel direction * speed + inherited velocity)
        // and target meet at the flight time the solve implies.
        let speed = TurretSectionConfig::default().muzzle_speed;
        let flight_time = aim.length() / speed;
        let barrel_dir = aim.normalize();
        let bullet_at_t = (barrel_dir * speed + Vec3::new(30.0, 0.0, 0.0)) * flight_time;
        assert!(
            (bullet_at_t - target).length() < 0.5,
            "inherited drift must carry the bullet onto the target: bullet \
             {bullet_at_t:?}, target {target:?}"
        );
    }

    #[test]
    fn shooter_without_physics_inherits_nothing() {
        // Test rigs and previews have marker-only ships: the aim point must
        // fall back to the plain world-frame solve instead of skipping.
        let target = Vec3::new(0.0, 0.0, -100.0);
        let mut world = World::new();
        let ship = world.spawn(SpaceshipRootMarker).id();
        let muzzle = world
            .spawn((TurretSectionBarrelMuzzleMarker, Transform::IDENTITY))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(Some(target)),
                TurretSectionTargetVelocity(Vec3::ZERO),
                TurretSectionConfigHelper(TurretSectionConfig::default()),
                TurretSectionMuzzleEntity(muzzle),
                TurretSectionAimPoint(None),
                ChildOf(ship),
            ))
            .id();

        let aim = aim_point(&mut world, turret).expect("aim point computed");
        assert!((aim - target).length() < 1e-3);
    }

    /// An app that runs the full aim + controller + sync + propagation loop on a
    /// manual clock, so a stepped turret converges its muzzle onto a target.
    fn aim_convergence_app() -> App {
        use bevy::time::TimeUpdateStrategy;
        use nova_gameplay::transform::prelude::SmoothLookRotationPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, SmoothLookRotationPlugin));
        app.add_observer(insert_turret_section);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        // The aim chain runs in FixedUpdate BEFORE the controller's Sync and
        // the joint sync after it, the same edges `TurretSectionPlugin`
        // declares; this app wires the systems directly rather than through the
        // plugin, so it repeats them.
        app.add_systems(
            FixedUpdate,
            (
                (update_turret_aim_point, update_turret_target_joints_system)
                    .chain()
                    .before(SmoothLookRotationSystems::Sync),
                sync_turret_joint_rotation.after(SmoothLookRotationSystems::Sync),
            ),
        );
        app
    }

    /// The world-space muzzle forward (-Z) after propagation.
    fn muzzle_aim_error_deg(app: &mut App, muzzle: Entity, target: Vec3) -> f32 {
        let gt = *app.world().get::<GlobalTransform>(muzzle).unwrap();
        let pos = gt.translation();
        let forward: Vec3 = gt.forward().into();
        let to_target = (target - pos).normalize();
        forward.dot(to_target).clamp(-1.0, 1.0).acos().to_degrees()
    }

    #[test]
    fn a_default_turret_converges_its_muzzle_onto_a_target() {
        // AIM CONVERGENCE (behavioral CCD parity): a default turret steered at a
        // reachable target must point its muzzle forward (-Z) at the target
        // within a few degrees after stepping frames. This is the parity
        // guarantee for the CCD swap - proven behaviorally, not by theta match.
        for target in [
            Vec3::new(10.0, 5.0, -30.0),
            Vec3::new(-15.0, 8.0, -25.0),
            Vec3::new(0.0, 3.0, -40.0),
        ] {
            let mut app = aim_convergence_app();
            let ship = app
                .world_mut()
                .spawn((SpaceshipRootMarker, Transform::IDENTITY))
                .id();
            let turret = app
                .world_mut()
                .spawn(turret_section(TurretSectionConfig {
                    muzzle_speed: 1000.0, // near-straight lead so aim ~= target dir
                    ..Default::default()
                }))
                .id();
            app.world_mut()
                .entity_mut(turret)
                .insert((ChildOf(ship), Transform::IDENTITY));
            app.world_mut().flush();
            app.world_mut()
                .entity_mut(turret)
                .insert(TurretSectionTargetInput(Some(target)));
            let muzzle = muzzle_entity(&app, turret);

            for _ in 0..240 {
                app.update();
            }

            let error = muzzle_aim_error_deg(&mut app, muzzle, target);
            assert!(
                error < 5.0,
                "muzzle should converge onto {target:?}, aim error {error} deg"
            );
        }
    }

    #[test]
    fn a_turret_swings_around_to_a_target_directly_behind() {
        // STUCK-BEHIND REGRESSION: a target dead behind the barrel is a rotation
        // singularity (either way is a valid 180); the naive solve dithered and
        // the turret froze facing forward. It must commit and swing around.
        let mut app = aim_convergence_app();
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Transform::IDENTITY))
            .id();
        let turret = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig {
                muzzle_speed: 1000.0,
                ..Default::default()
            }))
            .id();
        app.world_mut()
            .entity_mut(turret)
            .insert((ChildOf(ship), Transform::IDENTITY));
        app.world_mut().flush();
        // Barrel rest points -Z; put the target dead behind at +Z (a hair off the
        // axis so it is a true 180, level so pitch can hold it).
        let target = Vec3::new(0.0, 0.0, 30.0);
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionTargetInput(Some(target)));
        let muzzle = muzzle_entity(&app, turret);

        for _ in 0..240 {
            app.update();
        }

        let error = muzzle_aim_error_deg(&mut app, muzzle, target);
        assert!(
            error < 5.0,
            "turret must swing around to a target behind, not freeze forward: \
             aim error {error} deg"
        );
    }

    /// The kinematic chain the SHIPPED turret prototypes are built from
    /// (`nova_authoring`'s `turret_joint_tree`), rebuilt here because that crate
    /// depends on this one. The dimensions matter to the pole: every offset is
    /// on the yaw axis' own plane (x = 0), and the barrel hangs BEHIND the pivot,
    /// so at full elevation the muzzle sits astern of the axis it points along -
    /// the case where the obvious lever-arm fallback steers the wrong way.
    fn shipped_turret_chain() -> TurretJoint {
        let fixed = |offset: Vec3, muzzle, children| TurretJoint {
            offset,
            axis: None,
            speed: std::f32::consts::PI,
            min: None,
            max: None,
            render_mesh: None,
            render_mesh_transform: None,
            muzzle,
            children,
        };
        fixed(
            Vec3::new(0.0, -0.5, 0.0),
            None,
            vec![TurretJoint {
                offset: Vec3::new(0.0, 0.1, 0.0),
                axis: Some(Vec3::Y),
                speed: std::f32::consts::PI,
                min: None,
                max: None,
                render_mesh: None,
                render_mesh_transform: None,
                muzzle: None,
                children: vec![TurretJoint {
                    offset: Vec3::new(0.0, 0.332706, 0.303954),
                    axis: Some(Vec3::X),
                    speed: std::f32::consts::PI,
                    min: Some(-std::f32::consts::PI / 18.0),
                    max: Some(std::f32::consts::FRAC_PI_2),
                    render_mesh: None,
                    render_mesh_transform: None,
                    muzzle: None,
                    children: vec![fixed(
                        Vec3::new(0.0, 0.128437, -0.110729),
                        None,
                        vec![fixed(
                            Vec3::new(0.0, 0.0, -1.2),
                            Some(MuzzleConfig {
                                fire_rate: 10.0,
                                muzzle_effect: None,
                            }),
                            vec![],
                        )],
                    )],
                }],
            }],
        )
    }

    #[test]
    fn a_turret_tracks_a_target_across_the_zenith() {
        // POLE REGRESSION: a target that crosses overhead drives the pitch hinge
        // into its +90 deg elevation limit, where the muzzle forward is PARALLEL
        // to the yaw hinge axis. The yaw joint's in-plane muzzle heading collapses
        // there, so it took no correction, and pitch - the only other joint - was
        // already pinned at the limit. The barrel parked pointing straight up and
        // never came back down. It must follow the target onto the new bearing.
        //
        // Run on the default chain AND on the shipped one, because the two put
        // the muzzle on opposite sides of the yaw axis at full elevation.
        for root in [TurretSectionConfig::default().root, shipped_turret_chain()] {
            let mut app = aim_convergence_app();
            let ship = app
                .world_mut()
                .spawn((SpaceshipRootMarker, Transform::IDENTITY))
                .id();
            let turret = app
                .world_mut()
                .spawn(turret_section(TurretSectionConfig {
                    root,
                    muzzle_speed: 1000.0,
                    ..Default::default()
                }))
                .id();
            app.world_mut()
                .entity_mut(turret)
                .insert((ChildOf(ship), Transform::IDENTITY));
            app.world_mut().flush();

            // Nearly overhead off the bow: the barrel sits just short of the
            // limit, so the next target change pins it before yaw can slew
            // anywhere.
            let ahead = Vec3::new(0.0, 100.0, -3.0);
            app.world_mut()
                .entity_mut(turret)
                .insert(TurretSectionTargetInput(Some(ahead)));
            let muzzle = muzzle_entity(&app, turret);
            for _ in 0..240 {
                app.update();
            }
            let error = muzzle_aim_error_deg(&mut app, muzzle, ahead);
            assert!(error < 5.0, "setup: aim error {error} deg off the bow");

            // The target has crossed the zenith: opposite bearing, well down off
            // the vertical. Pitch is driven straight into its limit while yaw
            // still has a half-turn to go, so the hinge sits pinned at the pole.
            let behind = Vec3::new(0.0, 30.0, 30.0);
            app.world_mut()
                .entity_mut(turret)
                .insert(TurretSectionTargetInput(Some(behind)));
            for _ in 0..480 {
                app.update();
            }

            let error = muzzle_aim_error_deg(&mut app, muzzle, behind);
            assert!(
                error < 5.0,
                "turret must come back down onto a target that crossed the \
                 zenith, not park at full elevation: aim error {error} deg"
            );
        }
    }

    #[test]
    fn an_aimed_turret_holds_steady_without_shaking() {
        // NO-SHAKE REGRESSION: an undamped CCD
        // step (`target = output + delta`) made the coupled yaw+pitch overshoot
        // and settle into a ~4-5 deg limit cycle - the barrel visibly shook
        // around the aim. With the damping gain + deadband the muzzle must HOLD:
        // after it converges, its aim error must barely move frame to frame.
        let target = Vec3::new(8.0, 6.0, -28.0);
        let mut app = aim_convergence_app();
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Transform::IDENTITY))
            .id();
        let turret = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig {
                muzzle_speed: 1000.0,
                ..Default::default()
            }))
            .id();
        app.world_mut()
            .entity_mut(turret)
            .insert((ChildOf(ship), Transform::IDENTITY));
        app.world_mut().flush();
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionTargetInput(Some(target)));
        let muzzle = muzzle_entity(&app, turret);

        // Converge.
        for _ in 0..240 {
            app.update();
        }

        // Then watch the aim error for 120 more frames: its peak-to-peak swing is
        // the shake amplitude. A limit cycle would read several degrees; a held
        // turret reads a fraction of one.
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for _ in 0..120 {
            app.update();
            let e = muzzle_aim_error_deg(&mut app, muzzle, target);
            min = min.min(e);
            max = max.max(e);
        }
        let peak_to_peak = max - min;
        assert!(
            peak_to_peak < 0.5,
            "an aimed turret must hold steady, not shake: aim error swings \
             {peak_to_peak} deg (min {min}, max {max})"
        );
    }

    /// Track a target crossing the bow (range 100 u, 15 u/s on +X) on a fixed
    /// `dt` clock for `sim_time` seconds, then return the residual aim error
    /// (deg) against the target's final position. One extra settle update
    /// precedes the measurement: the Update-schedule joint sync applies the
    /// PREVIOUS frame's controller output, so without it the measured pose
    /// would lag the last correction by one frame of the rate under test.
    fn crossing_residual_error_deg(dt: f32, sim_time: f32) -> f32 {
        use bevy::time::TimeUpdateStrategy;

        let target_at = |t: f32| Vec3::new(-30.0 + 15.0 * t, 0.0, -100.0);

        let mut app = aim_convergence_app();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            dt,
        )));
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Transform::IDENTITY))
            .id();
        let turret = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig::default()))
            .id();
        app.world_mut()
            .entity_mut(turret)
            .insert((ChildOf(ship), Transform::IDENTITY));
        app.world_mut().flush();
        let muzzle = muzzle_entity(&app, turret);

        // The first manual-clock update has dt 0; burn it before tracking.
        app.update();

        let steps = (sim_time / dt).round() as u32;
        for k in 1..=steps {
            let t = k as f32 * dt;
            app.world_mut()
                .entity_mut(turret)
                .insert(TurretSectionTargetInput(Some(target_at(t))));
            app.update();
        }
        app.update();

        muzzle_aim_error_deg(&mut app, muzzle, target_at(sim_time))
    }

    #[test]
    fn aim_tracking_lag_is_framerate_invariant() {
        // THE BUG (task 20260816-184718): the correction gain damped the aim
        // error per FRAME, so residual tracking lag scaled with frame time -
        // ~0.43 deg at 60 fps vs ~1.8 deg at 14 fps on a crossing target,
        // above the 0.92 deg fire gate exactly when the machine struggled.
        // With the dt-based decay, the same SIM time against the same crosser
        // must land the same residual at any frame rate.
        //
        // Tolerance: the decay itself is exactly framerate-invariant; what
        // remains is sampling - the target moves w * dt between corrections,
        // so the residual measured right after a correction reads
        // (1 - gain) * w * dt / gain (~0.24 deg at 1/60, ~0.11 deg at 1/14
        // here), both bracketing the continuous-time lag w / rate. 0.5 deg
        // covers that bias; the pre-fix per-frame gain read ~1.0 deg at 1/14
        // in this harness and fails this assert.
        let at_60 = crossing_residual_error_deg(1.0 / 60.0, 4.0);
        let at_14 = crossing_residual_error_deg(1.0 / 14.0, 4.0);
        assert!(
            at_60 < 1.0 && at_14 < 1.0,
            "both rates must actually track the crosser: {at_60} deg at 1/60, \
             {at_14} deg at 1/14"
        );
        assert!(
            (at_60 - at_14).abs() < 0.5,
            "residual tracking lag must not depend on frame rate: {at_60} deg \
             at 1/60 vs {at_14} deg at 1/14"
        );
    }

    #[test]
    fn a_three_hinge_tree_reaches_a_target_a_two_dof_turret_cannot() {
        // MULTI-HINGE: a hand-built 3-hinge arm (Y at base, then X, then Y two
        // down) converges onto a target, sanity that arbitrary chains solve.
        let mut app = aim_convergence_app();
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Transform::IDENTITY))
            .id();

        // Y -> X -> Y -> muzzle, each a link one unit long, generous limits.
        let hinge = |axis: Vec3, offset: Vec3, children: Vec<TurretJoint>| TurretJoint {
            offset,
            axis: Some(axis),
            speed: std::f32::consts::TAU,
            min: Some(-std::f32::consts::PI),
            max: Some(std::f32::consts::PI),
            render_mesh: None,
            render_mesh_transform: None,
            muzzle: None,
            children,
        };
        let root = hinge(
            Vec3::Y,
            Vec3::ZERO,
            vec![hinge(
                Vec3::X,
                Vec3::new(0.0, 1.0, 0.0),
                vec![hinge(
                    Vec3::Y,
                    Vec3::new(0.0, 1.0, 0.0),
                    vec![TurretJoint {
                        offset: Vec3::new(0.0, 0.0, -1.0),
                        axis: None,
                        speed: std::f32::consts::PI,
                        min: None,
                        max: None,
                        render_mesh: None,
                        render_mesh_transform: None,
                        muzzle: Some(MuzzleConfig {
                            fire_rate: 10.0,
                            muzzle_effect: None,
                        }),
                        children: vec![],
                    }],
                )],
            )],
        );

        let config = TurretSectionConfig {
            root,
            muzzle_speed: 1000.0,
            ..default()
        };
        let turret = app.world_mut().spawn(turret_section(config)).id();
        app.world_mut()
            .entity_mut(turret)
            .insert((ChildOf(ship), Transform::IDENTITY));
        app.world_mut().flush();

        let target = Vec3::new(6.0, -4.0, 8.0); // behind + below: needs the extra DOF
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionTargetInput(Some(target)));
        let muzzle = muzzle_entity(&app, turret);

        for _ in 0..600 {
            app.update();
        }

        let error = muzzle_aim_error_deg(&mut app, muzzle, target);
        assert!(
            error < 8.0,
            "a 3-hinge arm must converge onto {target:?}, aim error {error} deg"
        );
    }

    #[test]
    fn a_shared_chain_twin_barrel_points_both_muzzles_at_the_target() {
        // MULTI-MUZZLE AIM: a twin barrel that shares its
        // yaw+pitch chain (two muzzles hanging off one common barrel) steers BOTH
        // muzzles onto the target - the shared joints are written by each muzzle's
        // CCD pass and agree for the symmetric barrels.
        let mut app = aim_convergence_app();
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Transform::IDENTITY))
            .id();

        // base(fixed) -> yaw(Y) -> pitch(X) -> barrel(fixed) with TWO muzzles.
        let muzzle = |x: f32| TurretJoint {
            offset: Vec3::new(x, 0.0, -0.5),
            axis: None,
            speed: default_joint_speed(),
            min: None,
            max: None,
            render_mesh: None,
            render_mesh_transform: None,
            muzzle: Some(MuzzleConfig {
                fire_rate: 10.0,
                muzzle_effect: None,
            }),
            children: vec![],
        };
        let yaw = TurretJoint {
            offset: Vec3::ZERO,
            axis: Some(Vec3::Y),
            speed: std::f32::consts::TAU,
            min: Some(-std::f32::consts::PI),
            max: Some(std::f32::consts::PI),
            render_mesh: None,
            render_mesh_transform: None,
            muzzle: None,
            children: vec![TurretJoint {
                offset: Vec3::ZERO,
                axis: Some(Vec3::X),
                speed: std::f32::consts::TAU,
                min: Some(-std::f32::consts::PI),
                max: Some(std::f32::consts::PI),
                render_mesh: None,
                render_mesh_transform: None,
                muzzle: None,
                children: vec![muzzle(0.1), muzzle(-0.1)],
            }],
        };

        let config = TurretSectionConfig {
            root: yaw,
            muzzle_speed: 1000.0, // near-straight lead so aim ~= target dir
            ..default()
        };
        let turret = app.world_mut().spawn(turret_section(config)).id();
        app.world_mut()
            .entity_mut(turret)
            .insert((ChildOf(ship), Transform::IDENTITY));
        app.world_mut().flush();

        let target = Vec3::new(10.0, 5.0, -30.0);
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionTargetInput(Some(target)));
        let muzzles = app
            .world()
            .entity(turret)
            .get::<TurretSectionMuzzles>()
            .expect("the turret records its muzzles")
            .0
            .clone();
        assert_eq!(muzzles.len(), 2, "the twin barrel must record two muzzles");

        for _ in 0..240 {
            app.update();
        }

        for muzzle in muzzles {
            let error = muzzle_aim_error_deg(&mut app, muzzle, target);
            assert!(
                error < 5.0,
                "both muzzles must converge onto {target:?}, muzzle {muzzle:?} \
                 aim error {error} deg"
            );
        }
    }
}
