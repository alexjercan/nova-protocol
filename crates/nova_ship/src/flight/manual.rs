//! Manual piloting: the analog main-drive burn a pilot flies without an
//! autopilot, and the RCS fine-adjust primitive both the pilot and the
//! autopilot's terminal settle drive.
//!
//! Engine units throughout: the caps and tapers here are compared against an
//! avian `LinearVelocity`, so a speed is a world unit per second and a world
//! unit is 10 m.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::{
    state::RcsReference,
    thrusters::{balance_throttles, spool_allocated_thrusters, BalanceEngine},
};
use crate::prelude::*;

/// Integrate one RCS virtual-joystick axis by `delta` and clamp to the unit
/// range the primitive expects (`RcsIntent` components are ~`[-1, 1]`). The
/// held-direction offset PERSISTS across frames: the pilot pushes to build it
/// and pulls back to zero it. Shared by the mouse (XZ) and scroll (Y) input
/// paths in the player-input layer.
pub(crate) fn accumulate_rcs_axis(current: f32, delta: f32) -> f32 {
    (current + delta).clamp(-1.0, 1.0)
}

/// Per-tick multiplier that fades the PLAYER's `RcsIntent` toward zero when no
/// fresh mouse/scroll motion arrives ([`decay_player_rcs_intent`]), so RCS is
/// delta-driven, not a persistent joystick. ~0.4 leaves a ~3-tick (~50 ms) tail
/// that smooths the per-frame input without feeling like a held stick.
/// Feel-tune.
const RCS_PLAYER_INTENT_DECAY: f32 = 0.4;

/// Fraction of the cap over which a speed budget tapers to zero (the
/// last stretch below the cap). Wide enough to feel like drag, not a wall.
const SPEED_CAP_TAPER_FRACTION: f32 = 0.2;

/// The fraction of `push` - the delta-v one tick of a commanded burn would
/// add, world frame - that a VECTOR speed budget of `cap` allows.
///
/// `residual` is the velocity the budget is measured against: the plain
/// velocity for the manual burn, `velocity - RcsReference` for RCS. The budget
/// limits its MAGNITUDE, so straight and diagonal input spend the same
/// allowance and no combination of axes buys more speed than one axis does.
///
/// Three regimes, in order:
///
/// - **Not growing.** A step that leaves the residual no faster than it
///   already is passes untouched, so braking and retrograde trim keep full
///   authority at and above the cap and an overspeed ship can always fly back
///   inside the budget.
/// - **Soft taper.** Otherwise the headroom `cap - speed` tapers the step over
///   the last `taper_band`, in proportion to how much of the step actually
///   becomes speed: a push straight down the residual meets the whole taper
///   (the straight-line rule the manual burn always had), a near-tangential
///   one barely feels it. Both terms are continuous and monotone in the
///   residual speed, so the approach to the cap is a first-order relaxation
///   rather than an on/off gate that could chatter at the boundary.
/// - **Finite step.** A tapered step still lands a whole tick's delta-v at
///   once, so it is finally shrunk to the largest fraction that stays inside
///   the budget sphere. The sphere never shrinks below where the ship already
///   is: an overspeed ship is held, never shoved.
///
/// Pure for unit testing.
pub(super) fn speed_budget_scale(residual: Vec3, push: Vec3, cap: f32, taper_band: f32) -> f32 {
    let step = push.length();
    if step <= 0.0 {
        return 1.0;
    }
    let speed = residual.length();
    let grown = (residual + push).length();
    if grown <= speed {
        return 1.0;
    }
    let growth = ((grown - speed) / step).clamp(0.0, 1.0);
    let taper = ((cap - speed) / taper_band.max(f32::EPSILON)).clamp(0.0, 1.0);
    let gate = 1.0 - growth * (1.0 - taper);
    gate.min(step_inside_sphere(residual, push, speed.max(cap)))
}

/// The largest `s` in `0..=1` with `|residual + s * push| <= radius`: the
/// positive root of the quadratic, which exists because `radius` is never
/// below `|residual|`.
fn step_inside_sphere(residual: Vec3, push: Vec3, radius: f32) -> f32 {
    let square_step = push.length_squared();
    if square_step <= 0.0 {
        return 1.0;
    }
    let along = residual.dot(push);
    let outside = residual.length_squared() - radius * radius;
    let discriminant = (along * along - square_step * outside).max(0.0);
    ((-along + discriminant.sqrt()) / square_step).clamp(0.0, 1.0)
}

/// Manual main-drive burn for intent-carrying ships with no autopilot
/// engaged: allocate the analog burn over the live unbound engine set as a
/// torque-nulling throttle vector, so an off-center or damage-shifted drive
/// still pushes the resultant force through the COM. The forward set delivers
/// the demand via differential throttle when it has headroom; when it does
/// not (the single damage-shifted main drive), the allocator recruits an
/// off-axis engine for pure counter-torque, trading a bounded sideways drift
/// for a straight heading. Only when nothing can help - no headroom and no
/// off-axis engine left - does the ship still pull, held by the PD as before.
pub(super) fn manual_burn_system(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    q_ship: Query<
        (
            Entity,
            &FlightIntent,
            Option<&ComputedCenterOfMass>,
            Option<&FlightSpeedCap>,
            &ComputedMass,
            &Rotation,
            &LinearVelocity,
        ),
        (With<SpaceshipRootMarker>, Without<Autopilot>),
    >,
    mut q_thruster: Query<
        (
            Entity,
            &mut ThrusterSectionInput,
            &ThrusterSectionMagnitude,
            &Transform,
            &ChildOf,
        ),
        (
            With<ThrusterSectionMarker>,
            Without<SectionInactiveMarker>,
            Without<SpaceshipRootMarker>,
            Without<SpaceshipThrusterInputBinding>,
        ),
    >,
) {
    let dt = time.delta_secs();

    for (ship, intent, com, speed_cap, mass, rotation, velocity) in &q_ship {
        let burn = intent.burn.clamp(0.0, 1.0);

        // The allocation set: every live unbound engine (bound thrusters keep
        // their own keys), with its balance coefficients in the ship-local
        // frame. The engines facing the hull's forward -Z are the *primary*
        // set the burn is budgeted against; the rest - laterals, retros - are
        // counter-torque candidates. The balance objective is frame-invariant,
        // and ComputedCenterOfMass is already body-local, so no world lift is
        // needed - lever arms are taken straight from the section transforms
        // about the local COM.
        let com_local = com.map(|c| c.0).unwrap_or(Vec3::ZERO);
        let mut allocation: Vec<(Entity, BalanceEngine)> = Vec::new();
        for (thruster, _, magnitude, transform, &ChildOf(parent)) in &q_thruster {
            if parent != ship {
                continue;
            }
            let local_dir = transform.rotation.mul_vec3(Vec3::NEG_Z).normalize();
            let aligned = local_dir.dot(Vec3::NEG_Z);
            let primary = is_forward_aligned(local_dir, Vec3::NEG_Z);
            let torque = (transform.translation - com_local).cross(local_dir * **magnitude);
            // Same convention as the autopilot: recruits bill their whole
            // thrust to the off-axis penalty (see BalanceEngine).
            let (forward, lateral) = if primary {
                (
                    **magnitude * aligned,
                    (local_dir - aligned * Vec3::NEG_Z) * **magnitude,
                )
            } else {
                (0.0, local_dir * **magnitude)
            };
            allocation.push((
                thruster,
                BalanceEngine {
                    forward,
                    lateral,
                    torque,
                    primary,
                },
            ));
        }

        // The primary set's authority: a ThrusterSectionMagnitude is an
        // IMPULSE per fixed tick, so the sum over the forward set divided by
        // the hull mass is the delta-v a full-stick burn adds this tick.
        let authority: f32 = allocation
            .iter()
            .filter(|(_, e)| e.primary)
            .map(|(_, e)| e.forward)
            .sum();

        // The soft speed cap on TOTAL speed, not on the burn axis: a pilot who
        // turns and burns again spends the same one budget, instead of
        // stacking a fresh cap onto every heading they point at. Raw-clock
        // pose (avian Rotation) - this is FixedUpdate.
        let burn = match speed_cap {
            Some(cap) => {
                let step = rotation.0.mul_vec3(Vec3::NEG_Z) * authority / mass.value().max(1e-6);
                let taper_band = (**cap * SPEED_CAP_TAPER_FRACTION).max(1.0);
                burn * speed_budget_scale(velocity.0, step, **cap, taper_band)
            }
            None => burn,
        };

        // Deliver `burn` of the main-drive set's forward thrust, balanced. The
        // uniform throttle `burn` over that set is a feasible split, so a
        // centered drive spools exactly as before; an off-center one is
        // trimmed toward straight flight, recruiting an off-axis engine when
        // the set cannot trim itself.
        let demand = burn * authority;
        let coeffs: Vec<BalanceEngine> = allocation.iter().map(|(_, e)| *e).collect();
        let throttles = balance_throttles(&coeffs, demand);

        spool_allocated_thrusters(
            ship,
            &allocation,
            &throttles,
            &mut q_thruster,
            &settings,
            dt,
        );
    }
}

/// Reaction-control fine translation: the shared RCS primitive. For a ship
/// carrying a non-zero [`RcsIntent`] (a ship-local desired direction), apply a
/// magnitude-limited, speed-capped acceleration at the center of mass in that
/// direction, so the pilot - or the autopilot - can translate the hull without
/// changing its attitude.
///
/// Two properties define it:
/// - **No torque, geometry-independent.** The push is one linear impulse at the
///   COM ([`Forces::apply_linear_impulse`]), so RCS never rotates the hull and
///   needs no physical side/vertical thrusters - the `Rcs` verb is the fiction
///   that the flight computer has cold-gas quads. The impulse is scaled by mass
///   so `rcs_accel` is a true acceleration and the feel is mass-independent.
/// - **Capped, never free propulsion.** One [`speed_budget_scale`] budget
///   limits the MAGNITUDE of `velocity - reference`, the same rule the manual
///   burn flies: a push that would carry the hull past the cap yields nothing
///   however many axes it is spread over, while anything that slows the hull
///   still acts. So RCS can only reshuffle velocity inside one sphere of radius
///   `cap`, never accumulate speed by spamming it diagonally.
///
/// Gated on the ship granting the `Rcs` verb (same rule as `ship_grants_verb`
/// in the input layer). Deliberately NOT gated on `Without<Autopilot>`: the
/// autopilot follow-up drives this very primitive while engaged.
pub(super) fn rcs_burn_system(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    mut q_ship: Query<
        (
            Entity,
            &RcsIntent,
            Option<&RcsSpeedCap>,
            Option<&RcsReference>,
            &ComputedMass,
            Forces,
        ),
        With<SpaceshipRootMarker>,
    >,
    q_controllers: Query<
        (&ChildOf, Option<&WithheldVerbs>),
        (
            With<ControllerSectionMarker>,
            With<PDController>,
            Without<SectionInactiveMarker>,
        ),
    >,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    for (ship, intent, cap, reference, mass, mut force) in &mut q_ship {
        // Idle ships cost nothing.
        if intent.0 == Vec3::ZERO {
            continue;
        }
        // Capability gate: only a ship with a live controller section that
        // grants RCS fine-adjusts, even if something wrote an intent. Mirrors
        // `ship_grants_verb` (input/player/flight_rig.rs) so the verb stays
        // authoritative no matter who drives the primitive.
        let granted = q_controllers.iter().any(|(&ChildOf(parent), withheld)| {
            parent == ship && withheld.is_none_or(|w| w.granted(FlightVerb::Rcs))
        });
        if !granted {
            continue;
        }

        let cap = cap.map(|c| c.0).unwrap_or(settings.rcs_speed_cap);
        if cap <= 0.0 {
            continue;
        }
        // Small cap by design, so the manual-burn `.max(1.0)` floor (sized for
        // the main drive's tens-of-u/s caps) would swamp it; floor only against
        // division blow-up.
        let taper_band = (cap * SPEED_CAP_TAPER_FRACTION).max(1e-3);
        let mass = mass.value();
        if !mass.is_finite() || mass <= 0.0 {
            continue;
        }
        let rotation = *force.rotation();
        let velocity = force.linear_velocity();
        // The cap is measured against this REFERENCE velocity: absent/zero
        // means the plain absolute cap (player fine-adjust, STOP/GOTO settle);
        // the autopilot supplies the orbital velocity here so RCS caps the
        // RESIDUAL `v - reference` and can trim a fast-moving orbit.
        let reference = reference.map(|r| r.0).unwrap_or(Vec3::ZERO);

        // One acceleration budget for every direction: each axis is a unit
        // command, and the whole vector is clamped to unit length, so a
        // three-axis diagonal pushes exactly as hard as a single axis.
        let command = intent
            .0
            .clamp(Vec3::splat(-1.0), Vec3::splat(1.0))
            .clamp_length_max(1.0);
        let step = rotation.mul_vec3(command) * settings.rcs_accel * dt;
        let delta_v = step * speed_budget_scale(velocity - reference, step, cap, taper_band);
        if delta_v != Vec3::ZERO {
            // Scale by mass so the 1/mass inside apply_linear_impulse yields
            // exactly `delta_v`, independent of hull mass.
            force.apply_linear_impulse(delta_v * mass);
        }
    }
}

/// Per-tick decay of the PLAYER's `RcsIntent`, so RCS fine-adjust is
/// DELTA-driven (force follows the mouse/scroll motion and stops when the input
/// stops) instead of a persistent virtual joystick that keeps pushing after you
/// let go - which playtested as "way too hard to control". The input layer SETS
/// the intent from each frame's motion; this fades it back to zero when no
/// fresh input arrives. Gated on [`RcsActive`] - the player's SHIFT modal - so
/// the AUTOPILOT's own `RcsIntent` (which it rewrites every tick, and which
/// never carries `RcsActive`) is untouched. Runs after [`rcs_burn_system`] in
/// the chain, so the intent this tick is spent before it decays.
pub(super) fn decay_player_rcs_intent(mut q_intent: Query<&mut RcsIntent, With<RcsActive>>) {
    for mut intent in &mut q_intent {
        if intent.0 == Vec3::ZERO {
            continue;
        }
        intent.0 *= RCS_PLAYER_INTENT_DECAY;
        // Snap tiny residue to zero so the ship truly coasts, not creeps.
        if intent.0.length_squared() < 1e-4 {
            intent.0 = Vec3::ZERO;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAP: f32 = 10.0;
    const BAND: f32 = CAP * SPEED_CAP_TAPER_FRACTION;
    /// One tick of the shipped 5 g RCS at 64 Hz.
    const STEP: f32 = 4.905 / 64.0;

    /// Integrate a held push from rest and report the speed it settles at.
    fn terminal_speed(direction: Vec3) -> f32 {
        let push = direction.normalize() * STEP;
        let mut velocity = Vec3::ZERO;
        for _ in 0..4000 {
            velocity += push * speed_budget_scale(velocity, push, CAP, BAND);
        }
        velocity.length()
    }

    /// The budget is on the VECTOR, so however many axes a held push is spread
    /// over it reaches the one ceiling - the `sqrt(2)` and `sqrt(3)` diagonals
    /// the per-axis gate used to hand out are gone.
    #[test]
    fn one_two_and_three_axis_pushes_reach_the_same_ceiling() {
        let one = terminal_speed(Vec3::X);
        let two = terminal_speed(Vec3::new(1.0, 1.0, 0.0));
        let three = terminal_speed(Vec3::ONE);
        assert!(
            (one - CAP).abs() < 1e-2,
            "one axis settles at the cap: {one}"
        );
        assert!((two - one).abs() < 1e-3, "two axes: {two} vs {one}");
        assert!((three - one).abs() < 1e-3, "three axes: {three} vs {one}");
    }

    /// Straight-line flight below the taper band is untouched, and inside the
    /// band the scale is exactly the old headroom taper.
    #[test]
    fn a_straight_push_keeps_full_authority_below_the_band_and_tapers_inside_it() {
        let push = Vec3::X * STEP;
        assert_eq!(speed_budget_scale(Vec3::ZERO, push, CAP, BAND), 1.0);
        assert_eq!(speed_budget_scale(Vec3::X * 5.0, push, CAP, BAND), 1.0);
        let inside = speed_budget_scale(Vec3::X * 9.0, push, CAP, BAND);
        assert!(
            (inside - 0.5).abs() < 1e-3,
            "half the headroom left: {inside}"
        );
        assert_eq!(speed_budget_scale(Vec3::X * CAP, push, CAP, BAND), 0.0);
    }

    /// Anything that slows the ship keeps full authority at the cap and well
    /// past it, so a ship carried overspeed by a well or a maneuver can always
    /// brake back inside the budget.
    #[test]
    fn braking_keeps_full_authority_at_and_above_the_cap() {
        let brake = Vec3::NEG_X * STEP;
        assert_eq!(speed_budget_scale(Vec3::X * CAP, brake, CAP, BAND), 1.0);
        assert_eq!(speed_budget_scale(Vec3::X * 40.0, brake, CAP, BAND), 1.0);
        // Partly retrograde still slows the ship, so it is still free.
        let oblique = Vec3::new(-1.0, 1.0, 0.0).normalize() * STEP;
        assert_eq!(speed_budget_scale(Vec3::X * 40.0, oblique, CAP, BAND), 1.0);
    }

    /// A push across the velocity grows the speed only to second order, which
    /// the per-tick sphere limit is what catches: at the cap a tangential push
    /// is spent, and an overspeed ship is held where it is rather than shoved
    /// further out.
    #[test]
    fn a_tangential_push_cannot_carry_the_residual_past_the_budget() {
        let across = Vec3::Y * STEP;
        assert_eq!(speed_budget_scale(Vec3::X * CAP, across, CAP, BAND), 0.0);
        assert_eq!(speed_budget_scale(Vec3::X * 40.0, across, CAP, BAND), 0.0);
        // Below the cap it costs almost nothing: the ship still maneuvers.
        let free = speed_budget_scale(Vec3::X * 5.0, across, CAP, BAND);
        assert!(
            free > 0.99,
            "a tangential push below the band is free: {free}"
        );
        // Held from the cap it never accumulates, however long it is held.
        let mut velocity = Vec3::X * CAP;
        for _ in 0..4000 {
            velocity += across * speed_budget_scale(velocity, across, CAP, BAND);
        }
        assert!(
            velocity.length() <= CAP + 1e-3,
            "a held tangential push must not creep past the cap: {}",
            velocity.length()
        );
    }

    /// The approach to the cap is a monotone first-order relaxation - the
    /// speed never overshoots and never falls back - so nothing oscillates at
    /// the boundary.
    #[test]
    fn the_approach_to_the_cap_never_overshoots_or_backs_off() {
        let push = Vec3::X * STEP;
        let mut velocity = Vec3::ZERO;
        for _ in 0..4000 {
            let next = velocity + push * speed_budget_scale(velocity, push, CAP, BAND);
            assert!(
                next.length() >= velocity.length() - 1e-6 && next.length() <= CAP + 1e-6,
                "monotone and inside the cap: {} -> {}",
                velocity.length(),
                next.length()
            );
            velocity = next;
        }
    }

    /// A step larger than the whole band still lands ON the cap rather than
    /// through it: the sphere limit, not the taper, is what bounds the
    /// finite-step overshoot.
    #[test]
    fn one_huge_step_lands_on_the_cap_instead_of_through_it() {
        let push = Vec3::X * 100.0;
        let scale = speed_budget_scale(Vec3::ZERO, push, CAP, BAND);
        assert!(
            ((push * scale).length() - CAP).abs() < 1e-3,
            "the step is trimmed to the budget: {}",
            (push * scale).length()
        );
    }
}
