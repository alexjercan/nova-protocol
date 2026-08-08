//! Pure flight math: the arrival rule a translation leg is planned with,
//! the ORBIT plan and its desired velocity, and the hull's rotation
//! budget. No queries, no world - every helper here is unit-testable in
//! isolation, and the autopilot is the only caller.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

use crate::prelude::*;

/// The fastest speed the ship may still carry `distance` short of its goal and
/// still be able to stop there, budgeting `lead_time` seconds of un-braked
/// travel for the flip and a well pull of `gravity_along` toward the goal: the
/// `v` solving `v*lead + g*lead^2/2 + (v + g*lead)^2 / (2*(a*margin - g)) =
/// distance`. Gravity keeps accelerating through the lead window (`g*lead` of
/// speed, `g*lead^2/2` of distance) and fights the brake afterwards (effective
/// deceleration `a*margin - g`). With `g = 0` this is the previous rule
/// exactly; with the pull at or above the brake authority no stopping plan
/// exists and the limit is zero. Zero at (or past) the goal. Pure for unit
/// testing.
fn arrival_speed_limit(
    distance: f32,
    accel: f32,
    margin: f32,
    lead_time: f32,
    gravity_along: f32,
) -> f32 {
    let braking = accel.max(0.0) * margin.clamp(0.0, 1.0);
    let g = gravity_along.max(0.0);
    let effective = braking - g;
    let distance = distance.max(0.0);
    if effective <= 0.0 || distance <= 0.0 {
        return 0.0;
    }
    let lead = lead_time.max(0.0);
    // Substituting u = v + g*lead (the speed when the brake lights) turns
    // the rule into u^2 / (2*effective) + u*lead = distance + g*lead^2/2.
    let rhs = distance + 0.5 * g * lead * lead;
    let u =
        -effective * lead + (effective * effective * lead * lead + 2.0 * effective * rhs).sqrt();
    (u - g * lead).max(0.0)
}

/// The velocity a GOTO wants right now: toward the target, at the arrival
/// rule's speed for the remaining distance (minus the standoff), floored at
/// `min_approach` so the ship actually crosses the boundary instead of
/// approaching it asymptotically. Zero once inside the standoff, and zero -
/// without the floor - when `gravity_along` eats the whole brake authority:
/// a ship that cannot stop must not be nursed inward at approach speed.
/// (In the last few units before the standoff the arrival limit can dip
/// below the floor even under a survivable pull; the floor still applies
/// there - that boundary-crossing creep is what it exists for.) Pure for
/// unit testing.
pub(super) fn goto_desired_velocity(
    to_target: Vec3,
    standoff: f32,
    accel: f32,
    margin: f32,
    lead_time: f32,
    min_approach: f32,
    gravity_along: f32,
) -> Vec3 {
    let distance = to_target.length();
    let remaining = distance - standoff;
    if remaining <= 0.0 || distance <= f32::EPSILON {
        return Vec3::ZERO;
    }
    if accel.max(0.0) * margin.clamp(0.0, 1.0) <= gravity_along.max(0.0) {
        return Vec3::ZERO;
    }
    let speed = arrival_speed_limit(remaining, accel, margin, lead_time, gravity_along)
        .max(min_approach.max(0.0));
    (to_target / distance) * speed
}

/// Where the flip-and-burn starts for the current state of a GOTO leg. Mirrors
/// the arrival rule the autopilot actually flies ([`arrival_speed_limit`]):
/// with a well pull of `gravity_along` toward the goal, the lead window covers
/// `v*lead + g*lead^2/2` and the brake ramp `(v + g*lead)^2 / (2*(brake_accel -
/// g))`, so the flip sits that far outside the standoff. Returns `(distance
/// from the goal center, coast seconds until the ship gets there)`, or `None`
/// when braking has already begun, the closing speed is too small for the coast
/// estimate to mean anything, or the pull eats the whole brake authority (no
/// stopping plan exists). The coast estimate uses the current closing speed;
/// gravity's coast-phase gain is absorbed by per-tick replanning. Pure for unit
/// testing.
pub(super) fn goto_flip_point(
    distance: f32,
    closing_speed: f32,
    brake_accel: f32,
    lead_time: f32,
    standoff: f32,
    gravity_along: f32,
) -> Option<(f32, f32)> {
    let g = gravity_along.max(0.0);
    let effective = brake_accel - g;
    if closing_speed < 0.5 || effective <= 0.0 {
        return None;
    }
    let lead = lead_time.max(0.0);
    let brake_entry_speed = closing_speed + g * lead;
    let brake_distance = brake_entry_speed * brake_entry_speed / (2.0 * effective);
    let flip_from_goal = standoff + closing_speed * lead + 0.5 * g * lead * lead + brake_distance;
    let coast = distance - flip_from_goal;
    if coast <= 0.0 {
        return None;
    }
    Some((flip_from_goal, coast / closing_speed))
}

/// Rough arrival estimate for a GOTO leg: coast to the flip point plus the
/// brake ramp (`v / a`); once braking, twice the remaining distance over
/// the closing speed (the mean of a linear ramp to zero). `None` when the
/// closing speed is too small to estimate, or when `gravity_along` eats
/// the whole brake authority - an unstoppable leg has no honest arrival
/// time, and a blank chip beats a confident lie. Pure for unit testing.
pub(super) fn arrival_eta(
    distance: f32,
    closing_speed: f32,
    brake_accel: f32,
    lead_time: f32,
    standoff: f32,
    gravity_along: f32,
) -> Option<f32> {
    if closing_speed < 0.5 {
        return None;
    }
    let g = gravity_along.max(0.0);
    if brake_accel - g <= 0.0 {
        return None;
    }
    let remaining = (distance - standoff).max(0.0);
    match goto_flip_point(
        distance,
        closing_speed,
        brake_accel,
        lead_time,
        standoff,
        gravity_along,
    ) {
        Some((_, coast)) => {
            // The brake ramp runs from the lead-window exit speed down to
            // rest at the gravity-reduced deceleration.
            let lead = lead_time.max(0.0);
            let brake_entry_speed = closing_speed + g * lead;
            Some(coast + lead + brake_entry_speed / (brake_accel - g).max(1e-3))
        }
        None => Some(2.0 * remaining / closing_speed),
    }
}

/// How far an engaged STOP travels before resting: the un-braked lead
/// window plus the brake ramp - the same terms as the GOTO flip, with the
/// same `gravity_along` budget (pull along the velocity, so a STOP falling
/// into a well predicts its true rest point). `None` when the remaining
/// brake authority cannot stop the ship. Pure for unit testing.
pub(super) fn stop_rest_distance(
    speed: f32,
    brake_accel: f32,
    lead_time: f32,
    gravity_along: f32,
) -> Option<f32> {
    if speed <= f32::EPSILON {
        return Some(0.0);
    }
    let g = gravity_along.max(0.0);
    let effective = brake_accel - g;
    if effective <= 0.0 {
        return None;
    }
    let lead = lead_time.max(0.0);
    let brake_entry_speed = speed + g * lead;
    Some(
        speed * lead
            + 0.5 * g * lead * lead
            + brake_entry_speed * brake_entry_speed / (2.0 * effective),
    )
}

/// The ORBIT plan's plane normal. The natural plane is the one the ship is
/// already moving in (`r x v`, the angular-momentum direction), so the
/// insertion keeps whatever tangential motion exists. When the velocity is
/// near-zero or near-radial that cross product is degenerate noise; the
/// fallback orbits "flat" relative to the pilot's own horizon: the ship's up
/// axis, rejected onto the radial so the normal stays perpendicular to
/// `r_vec`, with world axes as last resorts. Pure for unit testing.
pub(super) fn orbit_plane_normal(r_vec: Vec3, velocity: Vec3, ship_up: Vec3) -> Vec3 {
    let momentum = r_vec.cross(velocity);
    // Trust the momentum plane only when the tangential component is real:
    // |r x v| = |r||v| sin(angle), so this gate is sin(angle) > 0.1 with a
    // floor on the speed itself.
    if velocity.length() > 0.25 && momentum.length() > 0.1 * r_vec.length() * velocity.length() {
        if let Some(normal) = momentum.try_normalize() {
            return normal;
        }
    }
    let Some(r_hat) = r_vec.try_normalize() else {
        // On top of the well center (never happens outside tests): any
        // plane is as good as any other.
        return Vec3::Y;
    };
    for candidate in [ship_up, Vec3::Y, Vec3::X] {
        let rejected = candidate - r_hat * candidate.dot(r_hat);
        if let Some(normal) = rejected.try_normalize() {
            return normal;
        }
    }
    Vec3::Y
}

/// The ORBIT plan's target ring radius: the current radius, clamped into the
/// stable band of the well - no closer than `orbit_clearance_factor *
/// (body_radius + surface_margin)` (room to breathe over the surface clamp), no
/// farther than `orbit_band_safety * fade_start` (orbits are only trusted in
/// the unfaded core). `None` when the well has no stable band at all (clearance
/// past the trusted core) - such a well is unorbitable and the verb refuses to
/// plan. Pure for unit testing.
pub(super) fn orbit_target_radius(
    current_radius: f32,
    well: &GravityWell,
    gravity: &GravitySettings,
    settings: &FlightSettings,
) -> Option<f32> {
    let min =
        settings.orbit_clearance_factor.max(1.0) * (well.body_radius + gravity.surface_margin);
    let fade_start = well.soi_radius * (1.0 - gravity.fade_fraction.clamp(0.0, 1.0));
    let max = settings.orbit_band_safety.clamp(0.0, 1.0) * fade_start;
    // No stable band (the clearance already sits past the trusted core):
    // the well is unorbitable - a ring out there would be a permanently
    // powered fake orbit with no gravity assisting. The verb refuses to
    // plan rather than flying an incoherent circle.
    if min > max {
        return None;
    }
    Some(current_radius.clamp(min, max))
}

/// The velocity the ORBIT verb wants right now: tangential circular-orbit
/// speed on the planned ring, plus a bounded arrival-curve correction toward
/// the nearest ring point (which folds radial and out-of-plane error into
/// one term). On the ring the correction vanishes and this degenerates to
/// pure tangential v_circ - gravity does the rest. Travel direction is
/// `normal x radial`, which matches the ship's existing motion when the
/// plan's normal came from `r x v`. Pure for unit testing.
pub(super) fn orbit_desired_velocity(
    r_vec: Vec3,
    plan: &OrbitPlan,
    mu: f32,
    accel: f32,
    margin: f32,
    lead_time: f32,
) -> Vec3 {
    let tangential = plan.normal.cross(orbit_ring_radial(r_vec, plan));
    let to_ring = orbit_ring_offset(r_vec, plan);
    let correction = match to_ring.try_normalize() {
        // No gravity budget here (deliberate, unchanged from before the
        // arrival helpers learned it): during capture the correction does
        // fight residual gravity, but the ring sits in the stable band
        // above the fade, the nudge is bounded, and the hold loop
        // re-issues it every tick - v_circ balances the well once carried.
        Some(dir) => dir * arrival_speed_limit(to_ring.length(), accel, margin, lead_time, 0.0),
        None => Vec3::ZERO,
    };

    tangential * nova_gameplay::gravity::circular_orbit_speed(mu, plan.radius) + correction
}

/// Radial unit direction from the well center to the ship, projected into
/// the plan's orbit plane; when the ship sits on the plane axis itself, any
/// in-plane direction serves.
fn orbit_ring_radial(r_vec: Vec3, plan: &OrbitPlan) -> Vec3 {
    (r_vec - plan.normal * r_vec.dot(plan.normal))
        .try_normalize()
        .unwrap_or_else(|| plan.normal.any_orthonormal_vector())
}

/// Vector from the ship to the nearest point of the planned ring - the
/// combined radial and out-of-plane error the ORBIT correction burns down.
pub(super) fn orbit_ring_offset(r_vec: Vec3, plan: &OrbitPlan) -> Vec3 {
    orbit_ring_radial(r_vec, plan) * plan.radius - r_vec
}

/// Rotate `current` toward `target` by at most `max_step` radians. The PD's
/// torque clamp caps the SUM of its P and D terms, so feeding it a distant
/// setpoint (a 180 flip) drives it deep into saturation where the damping
/// contribution is swamped - the hull spins up unbraked, overshoots, and
/// limit-cycles ("tweaks out"). Slewing the command keeps the PD's tracking
/// error small, where its damping actually works. Reaching the target
/// exactly (instead of asymptotically) keeps fine aiming crisp. Pure for
/// unit testing.
pub(crate) fn slew_rotation(current: Quat, target: Quat, max_step: f32) -> Quat {
    let step = max_step.max(0.0);
    let angle = current.angle_between(target);
    if angle <= step || angle <= f32::EPSILON {
        target
    } else {
        current.slerp(target, step / angle)
    }
}

/// The hull's achievable turn rate under its torque budget, radians/second.
///
/// A torque-limited bang-bang 180 at angular acceleration `alpha = max_torque /
/// inertia` takes `2 * sqrt(pi / alpha)` seconds, an average rate of `sqrt(pi *
/// alpha) / 2`. The command slew and the autopilot's rotation-time planning
/// both run at this rate (trimmed by [`FlightSettings::turn_rate_scale`],
/// clamped by the min/max), so the same stick input swings a stripped hull
/// visibly faster than a fully built one. `inertia` is the largest principal
/// component - the conservative axis. Pure for unit testing.
pub(crate) fn hull_turn_rate(max_torque: f32, inertia: f32, settings: &FlightSettings) -> f32 {
    let alpha = (max_torque / inertia.max(1e-6)).max(0.0);
    let optimum = (core::f32::consts::PI * alpha).sqrt() * 0.5;
    // Ordered defensively: both bounds are inspector-editable on the
    // reflected FlightSettings, and f32::clamp panics when min > max.
    let lo = settings.turn_rate_min_deg.to_radians();
    let hi = settings.turn_rate_max_deg.to_radians().max(lo);
    (optimum * settings.turn_rate_scale).clamp(lo, hi)
}

/// The ship-level turn rate: the strongest live computer's torque against
/// the hull's largest principal inertia, through [`hull_turn_rate`]. `None`
/// with no live computer - every caller's "adrift" case. (PD outputs stack
/// additively across computers, so max under-reports a multi-computer hull -
/// a deliberately conservative simplification.) Shared by the player command
/// slew, the autopilot and the AI brain so the derivation cannot drift
/// apart.
pub(crate) fn ship_turn_rate(
    torques: impl Iterator<Item = f32>,
    inertia: &ComputedAngularInertia,
    settings: &FlightSettings,
) -> Option<f32> {
    let max_torque = torques.reduce(f32::max)?;
    let (principal, _) = inertia.principal_angular_inertia_with_local_frame();
    Some(hull_turn_rate(
        max_torque,
        principal.max_element(),
        settings,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrival_speed_limit_is_zero_at_the_goal_and_grows_with_distance() {
        assert_eq!(arrival_speed_limit(0.0, 20.0, 0.85, 0.0, 0.0), 0.0);
        assert_eq!(arrival_speed_limit(-5.0, 20.0, 0.85, 0.0, 0.0), 0.0);
        let near = arrival_speed_limit(10.0, 20.0, 0.85, 0.0, 0.0);
        let far = arrival_speed_limit(100.0, 20.0, 0.85, 0.0, 0.0);
        assert!(near > 0.0 && far > near, "limit must grow with distance");
        // The margin slows the plan down.
        assert!(
            arrival_speed_limit(100.0, 20.0, 0.5, 0.0, 0.0)
                < arrival_speed_limit(100.0, 20.0, 1.0, 0.0, 0.0)
        );
        // v = sqrt(2 a d) exactly at margin 1 with no flip lead.
        assert!(
            (arrival_speed_limit(100.0, 20.0, 1.0, 0.0, 0.0) - (2.0f32 * 20.0 * 100.0).sqrt())
                .abs()
                < 1e-4
        );
        // A flip lead budgets un-braked travel, so the allowed speed drops...
        let with_lead = arrival_speed_limit(100.0, 20.0, 1.0, 1.5, 0.0);
        assert!(with_lead < arrival_speed_limit(100.0, 20.0, 1.0, 0.0, 0.0));
        // ...and satisfies v * lead + v^2 / (2a) = d.
        let stopping = with_lead * 1.5 + with_lead * with_lead / (2.0 * 20.0);
        assert!((stopping - 100.0).abs() < 1e-3, "got {stopping}");
    }

    #[test]
    fn goto_desired_velocity_points_at_the_target_and_rests_inside_standoff() {
        let desired =
            goto_desired_velocity(Vec3::new(0.0, 0.0, -300.0), 50.0, 20.0, 0.85, 1.5, 1.5, 0.0);
        assert!(desired.z < 0.0 && desired.x == 0.0 && desired.y == 0.0);
        assert!((desired.length() - arrival_speed_limit(250.0, 20.0, 0.85, 1.5, 0.0)).abs() < 1e-4);
        // Inside the standoff the goal is rest.
        assert_eq!(
            goto_desired_velocity(Vec3::new(0.0, 0.0, -40.0), 50.0, 20.0, 0.85, 1.5, 1.5, 0.0),
            Vec3::ZERO
        );
        // Just outside the boundary the floor keeps a closing speed, so the
        // ship crosses instead of stalling on the asymptote.
        let creeping =
            goto_desired_velocity(Vec3::new(0.0, 0.0, -50.01), 50.0, 20.0, 0.85, 1.5, 1.5, 0.0);
        assert!((creeping.length() - 1.5).abs() < 1e-3);
        // Degenerate zero offset is safe.
        assert_eq!(
            goto_desired_velocity(Vec3::ZERO, 50.0, 20.0, 0.85, 1.5, 1.5, 0.0),
            Vec3::ZERO
        );
    }

    #[test]
    fn hull_turn_rate_makes_mass_legible() {
        let settings = FlightSettings::default();
        // Same torque budget, less hull: the stripped ship turns visibly
        // faster (stock 3-section ship ~2.3 inertia vs a ~0.9 remnant).
        let full = hull_turn_rate(10.0, 2.3, &settings);
        let stripped = hull_turn_rate(10.0, 0.9, &settings);
        assert!(
            stripped > full * 1.3,
            "stripped {stripped} should clearly out-turn full {full}"
        );
        // A torque-starved barge still answers the helm at the floor...
        let barge = hull_turn_rate(0.001, 1000.0, &settings);
        assert!((barge - settings.turn_rate_min_deg.to_radians()).abs() < 1e-5);
        // ...and an over-torqued skiff is capped at the ceiling.
        let skiff = hull_turn_rate(1000.0, 0.01, &settings);
        assert!((skiff - settings.turn_rate_max_deg.to_radians()).abs() < 1e-5);
        // Degenerate inputs stay finite.
        assert!(hull_turn_rate(10.0, 0.0, &settings).is_finite());
        assert!(hull_turn_rate(0.0, 0.0, &settings).is_finite());
    }

    #[test]
    fn slew_rotation_caps_the_step_and_reaches_the_target() {
        let target = Quat::from_rotation_y(std::f32::consts::PI);
        // One capped step covers exactly max_step of the arc.
        let stepped = slew_rotation(Quat::IDENTITY, target, 0.1);
        assert!((stepped.angle_between(Quat::IDENTITY) - 0.1).abs() < 1e-3);
        // Within one step of the target it lands exactly (crisp fine aiming).
        let near = slew_rotation(Quat::from_rotation_y(3.1), target, 0.1);
        assert_eq!(near, target);
        // Repeated steps converge.
        let mut q = Quat::IDENTITY;
        for _ in 0..40 {
            q = slew_rotation(q, target, 0.1);
        }
        assert!(q.angle_between(target) < 1e-3);
        // Degenerate inputs are safe.
        assert_eq!(slew_rotation(target, target, 0.1), target);
        assert_eq!(
            slew_rotation(Quat::IDENTITY, target, -1.0).angle_between(Quat::IDENTITY),
            0.0
        );
    }

    #[test]
    fn orbit_plane_normal_follows_momentum_and_falls_back_to_the_horizon() {
        let r = Vec3::new(50.0, 0.0, 0.0);
        // Real tangential motion: the plane is the one the ship already
        // moves in (r x v points +Y for -Z travel at +X).
        let n = orbit_plane_normal(r, Vec3::new(0.0, 0.0, -5.0), Vec3::Y);
        assert!((n - Vec3::Y).length() < 1e-5, "got {n}");
        // Near-rest and pure-radial velocities are degenerate: fall back to
        // the pilot's horizon (ship up, rejected onto the radial).
        for v in [Vec3::ZERO, Vec3::new(-5.0, 0.0, 0.0)] {
            let n = orbit_plane_normal(r, v, Vec3::Z);
            assert!((n - Vec3::Z).length() < 1e-5, "up fallback, got {n}");
        }
        // Ship up parallel to the radial: world axes take over; the result
        // must still be a unit normal perpendicular to r.
        let n = orbit_plane_normal(r, Vec3::ZERO, Vec3::X);
        assert!((n.length() - 1.0).abs() < 1e-5);
        assert!(n.dot(r).abs() < 1e-4, "normal must be perpendicular to r");
    }

    #[test]
    fn orbit_target_radius_clamps_into_the_stable_band() {
        let flight = FlightSettings::default();
        let gravity = GravitySettings::default();
        let well = GravityWell::from_mass(1200.0, 20.0, &gravity);
        // Band for the sanity rock (mu = 1200, so SOI = sqrt(1200 / 0.25) =
        // 69.28): [1.5 * 21, 0.9 * 0.85 * 69.28] = [31.5, 53.0].
        assert_eq!(
            orbit_target_radius(50.0, &well, &gravity, &flight),
            Some(50.0)
        );
        assert_eq!(
            orbit_target_radius(10.0, &well, &gravity, &flight),
            Some(31.5)
        );
        let band_max = flight.orbit_band_safety * (well.soi_radius * (1.0 - gravity.fade_fraction));
        assert_eq!(
            orbit_target_radius(150.0, &well, &gravity, &flight),
            Some(band_max)
        );
        // A well with no stable band (clearance past the trusted core) is
        // unorbitable: a ring out there would be a powered fake orbit with
        // zero gravity assisting.
        let tiny = GravityWell {
            mu: 100.0,
            body_radius: 10.0,
            soi_radius: 12.0,
        };
        assert_eq!(orbit_target_radius(5.0, &tiny, &gravity, &flight), None);
        assert_eq!(orbit_target_radius(100.0, &tiny, &gravity, &flight), None);
    }

    #[test]
    fn orbit_desired_velocity_is_pure_tangential_v_circ_on_the_ring() {
        let plan = OrbitPlan {
            radius: 50.0,
            normal: Vec3::Y,
        };
        let mu = 1200.0;
        // On the ring: no correction, pure tangential circular speed.
        let on_ring = orbit_desired_velocity(Vec3::new(50.0, 0.0, 0.0), &plan, mu, 20.0, 0.85, 0.5);
        let v_circ = nova_gameplay::gravity::circular_orbit_speed(mu, 50.0);
        assert!(
            (on_ring - Vec3::new(0.0, 0.0, -v_circ)).length() < 1e-4,
            "got {on_ring}"
        );
        // Inside the ring: the correction pushes outward (+X here).
        let inside = orbit_desired_velocity(Vec3::new(40.0, 0.0, 0.0), &plan, mu, 20.0, 0.85, 0.5);
        assert!(inside.x > 0.1, "outward correction expected, got {inside}");
        // Off-plane: the correction pulls back toward the plane (-Y here).
        let above = orbit_desired_velocity(Vec3::new(50.0, 10.0, 0.0), &plan, mu, 20.0, 0.85, 0.5);
        assert!(above.y < -0.1, "plane correction expected, got {above}");
    }

    #[test]
    fn goto_flip_point_matches_the_arrival_rule_lead_included() {
        // 12 u/s at 10 u/s^2 brakes in 7.2u; a 2s rotation lead coasts
        // another 24u un-braked; standoff 50 -> flip at 81.2 from the
        // goal, and 300 - 81.2 = 218.8u of coast at 12 u/s. This mirrors
        // arrival_speed_limit's v*lead + v^2/(2a) exactly - the marker
        // must sit where the ship actually flips.
        let flip = goto_flip_point(300.0, 12.0, 10.0, 2.0, 50.0, 0.0).expect("flip ahead");
        assert!((flip.0 - 81.2).abs() < 1e-3, "got {}", flip.0);
        assert!((flip.1 - 218.8 / 12.0).abs() < 1e-3, "got {}", flip.1);

        // Already inside the flip distance: braking has begun, no flip.
        assert_eq!(goto_flip_point(80.0, 12.0, 10.0, 2.0, 50.0, 0.0), None);
        // Near-zero closing speed or no brake authority: no estimate.
        assert_eq!(goto_flip_point(300.0, 0.1, 10.0, 2.0, 50.0, 0.0), None);
        assert_eq!(goto_flip_point(300.0, 12.0, 0.0, 2.0, 50.0, 0.0), None);
    }

    #[test]
    fn arrival_eta_covers_coast_and_brake_regimes() {
        // Coasting: coast + lead + the brake ramp (12/10 = 1.2s).
        let eta = arrival_eta(300.0, 12.0, 10.0, 2.0, 50.0, 0.0).expect("eta");
        assert!((eta - (218.8 / 12.0 + 2.0 + 1.2)).abs() < 1e-3, "got {eta}");
        // Braking: twice the remaining distance over the closing speed.
        let braking = arrival_eta(55.0, 12.0, 10.0, 2.0, 50.0, 0.0).expect("eta");
        assert!((braking - 2.0 * 5.0 / 12.0).abs() < 1e-3, "got {braking}");
        // No closing speed, no estimate.
        assert_eq!(arrival_eta(300.0, 0.1, 10.0, 2.0, 50.0, 0.0), None);
    }

    #[test]
    fn stop_rest_distance_covers_lead_and_brake_ramp() {
        // 12 u/s, 10 u/s^2, 2s lead: 24u un-braked + 7.2u ramp.
        let rest = stop_rest_distance(12.0, 10.0, 2.0, 0.0).expect("can stop");
        assert!((rest - 31.2).abs() < 1e-3, "got {rest}");
        assert_eq!(stop_rest_distance(0.0, 10.0, 2.0, 0.0), Some(0.0));
        assert_eq!(
            stop_rest_distance(12.0, 0.0, 2.0, 0.0),
            None,
            "no brake authority, no rest"
        );
    }

    #[test]
    fn arrival_speed_limit_budgets_the_well_pull() {
        // A pull toward the goal lowers the allowed speed...
        let flat = arrival_speed_limit(100.0, 20.0, 1.0, 1.5, 0.0);
        let pulled = arrival_speed_limit(100.0, 20.0, 1.0, 1.5, 4.0);
        assert!(pulled < flat, "{pulled} !< {flat}");

        // ...and the limit is exactly the v whose lead window + brake ramp
        // covers the distance under gravity: v*lead + g*lead^2/2 +
        // (v + g*lead)^2 / (2*(a*margin - g)) = d.
        let (d, a, lead, g) = (100.0f32, 20.0f32, 1.5f32, 4.0f32);
        let v = arrival_speed_limit(d, a, 1.0, lead, g);
        let u = v + g * lead;
        let covered = v * lead + 0.5 * g * lead * lead + u * u / (2.0 * (a - g));
        assert!((covered - d).abs() < 1e-3, "covered {covered} of {d}");

        // Pull at or above the brake authority: no stopping plan.
        assert_eq!(arrival_speed_limit(100.0, 20.0, 1.0, 1.5, 20.0), 0.0);
        assert_eq!(arrival_speed_limit(100.0, 20.0, 1.0, 1.5, 25.0), 0.0);
    }

    #[test]
    fn goto_desired_velocity_refuses_to_creep_into_an_unstoppable_well() {
        // With the pull eating the whole brake authority the desired
        // velocity is zero WITHOUT the min_approach floor - the floor
        // would nurse the ship inward on a leg it can never stop.
        assert_eq!(
            goto_desired_velocity(
                Vec3::new(0.0, 0.0, -300.0),
                50.0,
                20.0,
                0.85,
                1.5,
                1.5,
                17.0
            ),
            Vec3::ZERO
        );
        // A survivable pull still flies the leg, just slower.
        let flat =
            goto_desired_velocity(Vec3::new(0.0, 0.0, -300.0), 50.0, 20.0, 0.85, 1.5, 1.5, 0.0);
        let pulled =
            goto_desired_velocity(Vec3::new(0.0, 0.0, -300.0), 50.0, 20.0, 0.85, 1.5, 1.5, 4.0);
        assert!(pulled.length() < flat.length());
        assert!(pulled.length() > 0.0);
    }

    #[test]
    fn gravity_pushes_the_flip_point_out_and_the_rest_point_deeper() {
        // The flip must trigger earlier when the well fights the brake:
        // lead window gains g*lead^2/2 of drift and g*lead of speed, and
        // the ramp runs at the reduced deceleration.
        let flat = goto_flip_point(300.0, 12.0, 10.0, 2.0, 50.0, 0.0).expect("flip");
        let pulled = goto_flip_point(300.0, 12.0, 10.0, 2.0, 50.0, 3.0).expect("flip");
        // g=3: lead exit speed 18, ramp 18^2/(2*7) = 23.14, lead drift
        // 24 + 6 = 30 -> flip at 50 + 30 + 23.14 = 103.14.
        assert!((pulled.0 - 103.142_86).abs() < 1e-3, "got {}", pulled.0);
        assert!(pulled.0 > flat.0);
        // Pull >= brake authority: no plan.
        assert_eq!(goto_flip_point(300.0, 12.0, 10.0, 2.0, 50.0, 10.0), None);

        // The same terms lengthen a STOP's predicted rest.
        let rest_flat = stop_rest_distance(12.0, 10.0, 2.0, 0.0).expect("rest");
        let rest_pulled = stop_rest_distance(12.0, 10.0, 2.0, 3.0).expect("rest");
        assert!(
            (rest_pulled - (24.0 + 6.0 + 23.142_857)).abs() < 1e-3,
            "got {rest_pulled}"
        );
        assert!(rest_pulled > rest_flat);
        assert_eq!(stop_rest_distance(12.0, 10.0, 2.0, 10.0), None);

        // And the ETA brakes from the lead exit speed at the reduced rate.
        let eta = arrival_eta(300.0, 12.0, 10.0, 2.0, 50.0, 3.0).expect("eta");
        let coast = (300.0 - 103.142_86) / 12.0;
        assert!((eta - (coast + 2.0 + 18.0 / 7.0)).abs() < 1e-3, "got {eta}");
        // An unstoppable leg has no honest arrival time: None, never the
        // braking-regime fallback: the instruments must go blank.
        assert_eq!(arrival_eta(300.0, 12.0, 10.0, 2.0, 50.0, 10.0), None);
        assert_eq!(arrival_eta(55.0, 12.0, 10.0, 2.0, 50.0, 10.0), None);
    }
}
