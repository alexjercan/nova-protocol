//! Manual piloting: the analog main-drive burn a pilot flies without an
//! autopilot, and the RCS fine-adjust primitive both the pilot and the
//! autopilot's terminal settle drive. The main burn is plain Newtonian - it
//! never reads the hull's velocity - so the cap and taper here belong to RCS
//! alone.
//!
//! Engine units throughout: the RCS cap and taper are compared against an
//! avian `LinearVelocity`, so a speed is a world unit per second and a world
//! unit is 10 m.

use avian3d::prelude::*;
use bevy::{ecs::entity::EntityHashSet, prelude::*};
use nova_gameplay::prelude::*;

use super::{
    capability::{ship_capabilities, ShipCapabilityQuery},
    state::RcsReference,
    thrusters::{balance_throttles, spool_allocated_thrusters, BalanceEngine},
};
use crate::{prelude::*, sections::thruster_section::engine_direction_local};

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

/// Fraction of the RCS cap over which its speed budget tapers to zero (the
/// last stretch below the cap). Read by `rcs_burn_system` alone - the main
/// drive has no speed budget. Wide enough to feel like drag, not a wall.
const SPEED_CAP_TAPER_FRACTION: f32 = 0.2;

/// The delta-v an RCS push actually delivers under a VECTOR speed budget of
/// `cap`.
///
/// `residual` is the velocity the budget is measured against,
/// `velocity - RcsReference`. The budget limits its MAGNITUDE, so straight and
/// diagonal input spend the same allowance and no combination of axes buys more
/// speed than one axis does.
///
/// Three regimes, in order:
///
/// - **Not growing.** A step that leaves the residual no faster than it
///   already is passes untouched, so braking and retrograde trim keep full
///   authority at and above the cap and an overspeed hull can always trim back
///   inside the budget.
/// - **Soft taper.** Otherwise the headroom `cap - speed` tapers the step over
///   the last `taper_band`, in proportion to how much of the step actually
///   becomes speed: a push straight down the residual meets the whole taper, a
///   near-tangential one barely feels it. Both terms are continuous and
///   monotone in the residual speed, so the approach to the cap is a
///   first-order relaxation rather than an on/off gate that could chatter at
///   the boundary.
/// - **Clamped result.** The tapered step is applied and its RESULT is clamped
///   back onto the budget sphere, rather than the step itself being shrunk to
///   stay inside it. Shrinking the step solves a TANGENTIAL push at the cap to
///   exactly zero, so a hull holding the cap on one axis would find every
///   perpendicular axis dead until it first braked back down the axis it came
///   in on. First Shift's RCS lesson is a box of four mutually perpendicular
///   legs, each flown at the cap, so that is the shipped case. Clamping spends
///   the growth and lets the push TURN the velocity vector - the "reshuffle
///   inside one sphere, never accumulate" the verb is documented as.
///
/// Pure for unit testing.
pub(super) fn budgeted_rcs_delta_v(residual: Vec3, push: Vec3, cap: f32, taper_band: f32) -> Vec3 {
    let step = push.length();
    if step <= 0.0 {
        return Vec3::ZERO;
    }
    let speed = residual.length();
    let grown = (residual + push).length();
    // Anything that slows the hull is free, at the cap and well past it.
    if grown <= speed {
        return push;
    }
    let growth = ((grown - speed) / step).clamp(0.0, 1.0);
    let taper = ((cap - speed) / taper_band.max(f32::EPSILON)).clamp(0.0, 1.0);
    let tapered = push * (1.0 - growth * (1.0 - taper));
    // The sphere never shrinks below where the ship already is: an overspeed
    // hull is held at its speed - it may still turn, never accelerate.
    let radius = speed.max(cap);
    let result = residual + tapered;
    if result.length() <= radius {
        return tapered;
    }
    result.clamp_length_max(radius) - residual
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
            &Position,
            &Rotation,
            Option<&DockedShip>,
            Option<&DockedAssembly>,
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
    // Docked drivers whose missing assembly is already logged, so the error
    // is said once per loss, not at the fixed rate.
    mut missing_assembly: Local<EntityHashSet>,
) {
    let dt = time.delta_secs();

    missing_assembly.retain(|ship| {
        q_ship
            .get(*ship)
            .is_ok_and(|(.., docked, assembly)| docked.is_some() && assembly.is_none())
    });
    for (ship, intent, com, position, rotation, docked, assembly) in &q_ship {
        // Only a pair's driver burns: a suppressed partner commands no
        // throttle, so no plume lights for a helm it does not hold. A driver
        // with no assembly burns nothing rather than on its root alone.
        match (docked, assembly) {
            (Some(docked), _) if !docked.drives => continue,
            (Some(_), None) => {
                if missing_assembly.insert(ship) {
                    error!(
                        "manual_burn_system: docked driver {ship:?} has no DockedAssembly; \
                         burning nothing rather than on its root alone"
                    );
                }
                continue;
            }
            _ => {}
        }
        let burn = intent.burn.clamp(0.0, 1.0);

        // The allocation set: every live unbound engine (bound thrusters keep
        // their own keys), with its balance coefficients in the ship-local
        // frame. The engines facing the hull's forward -Z are the *primary*
        // set the burn is budgeted against; the rest - laterals, retros - are
        // counter-torque candidates. The balance objective is frame-invariant,
        // and ComputedCenterOfMass is already body-local, so no world lift is
        // needed - lever arms are taken straight from the section transforms
        // about the local COM. A docked driver balances about the pair's COM,
        // the point the pair turns about.
        let com_local = match assembly {
            Some(assembly) => rotation.inverse() * (assembly.center_of_mass - position.0),
            None => com.map(|c| c.0).unwrap_or(Vec3::ZERO),
        };
        let mut allocation: Vec<(Entity, BalanceEngine)> = Vec::new();
        for (thruster, _, magnitude, transform, &ChildOf(parent)) in &q_thruster {
            if parent != ship {
                continue;
            }
            let Some(local_dir) = engine_direction_local(transform) else {
                continue;
            };
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
/// - **Capped, never free propulsion.** One [`budgeted_rcs_delta_v`] budget
///   limits the MAGNITUDE of `velocity - reference`: a push that would carry
///   the hull past the cap is spent turning the velocity instead of growing it,
///   however many axes it is spread over, while anything that slows the hull
///   acts in full. So RCS can only reshuffle velocity inside one sphere of
///   radius `cap`, never accumulate speed by spamming it diagonally.
///
/// Gated on the ship's `rcs_enabled` capability. A docked pair's driver
/// translates the whole pair: the budget is measured on the pair's mass and
/// velocity, and each root takes its own share of the push at its own centre
/// of mass, so the pair moves as one body and still does not turn. A
/// suppressed partner's intent is not spent, and a driver with no assembly
/// pushes neither root.
/// Deliberately NOT gated on `Without<Autopilot>`: the autopilot follow-up
/// drives this very primitive while engaged.
pub(super) fn rcs_burn_system(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    mut q_ship: Query<
        (
            Entity,
            Option<&RcsIntent>,
            Option<&RcsSpeedCap>,
            Option<&RcsReference>,
            &ComputedMass,
            Option<&DockedShip>,
            Option<&DockedAssembly>,
            Forces,
        ),
        With<SpaceshipRootMarker>,
    >,
    q_connections: Query<&DockingConnection>,
    q_capabilities: ShipCapabilityQuery,
    mut pushes: Local<Vec<(Entity, Vec3)>>,
    // Docked drivers whose missing assembly is already logged, so the error
    // is said once per loss, not at the fixed rate.
    mut missing_assembly: Local<EntityHashSet>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    pushes.clear();
    missing_assembly.retain(|ship| {
        q_ship
            .get(*ship)
            .is_ok_and(|(.., docked, assembly, _)| docked.is_some() && assembly.is_none())
    });
    for (ship, intent, cap, reference, mass, docked, assembly, force) in &q_ship {
        // Idle ships cost nothing.
        let Some(intent) = intent.filter(|intent| intent.0 != Vec3::ZERO) else {
            continue;
        };
        let partner = match (docked, assembly) {
            (Some(docked), _) if !docked.drives => continue,
            (Some(_), None) => {
                if missing_assembly.insert(ship) {
                    error!(
                        "rcs_burn_system: docked driver {ship:?} has no DockedAssembly; \
                         pushing neither root rather than its root alone"
                    );
                }
                continue;
            }
            (Some(docked), Some(_)) => {
                let Ok(connection) = q_connections.get(docked.connection) else {
                    continue;
                };
                Some(if connection.first_ship == ship {
                    connection.second_ship
                } else {
                    connection.first_ship
                })
            }
            (None, _) => None,
        };
        // Capability gate: only a ship configured for RCS fine-adjusts, even
        // if something wrote an intent - so the capability stays authoritative
        // no matter who drives the primitive.
        if !ship_capabilities(ship, &q_capabilities).rcs_enabled {
            continue;
        }

        let cap = cap.map(|c| c.0).unwrap_or(settings.rcs_speed_cap);
        if cap <= 0.0 {
            continue;
        }
        // The RCS cap is a few u/s by design, so the floor here guards only
        // against division blow-up on a near-zero cap.
        let taper_band = (cap * SPEED_CAP_TAPER_FRACTION).max(1e-3);
        let (mass, velocity) = match assembly {
            Some(assembly) => (assembly.mass, assembly.linear_velocity),
            None => (mass.value(), force.linear_velocity()),
        };
        if !mass.is_finite() || mass <= 0.0 {
            continue;
        }
        let rotation = *force.rotation();
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
        let delta_v = budgeted_rcs_delta_v(velocity - reference, step, cap, taper_band);
        if delta_v != Vec3::ZERO {
            pushes.push((ship, delta_v));
            pushes.extend(partner.map(|partner| (partner, delta_v)));
        }
    }

    for &(root, delta_v) in pushes.iter() {
        let Ok((_, _, _, _, mass, _, _, mut force)) = q_ship.get_mut(root) else {
            continue;
        };
        // Scale by this root's mass so the 1/mass inside
        // apply_linear_impulse yields exactly `delta_v`, independent of hull
        // mass, and a pair's two roots move together.
        let mass = mass.value();
        force.apply_linear_impulse(delta_v * mass);
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

    /// Integrate a held RCS push from rest and report the speed it settles at.
    fn rcs_terminal_speed(direction: Vec3) -> f32 {
        let push = direction.normalize() * STEP;
        let mut velocity = Vec3::ZERO;
        for _ in 0..4000 {
            velocity += budgeted_rcs_delta_v(velocity, push, CAP, BAND);
        }
        velocity.length()
    }

    /// The RCS budget is on the VECTOR, so however many axes a held push is
    /// spread over it reaches the one ceiling - the `sqrt(2)` and `sqrt(3)`
    /// diagonals a per-axis gate would hand out are gone.
    #[test]
    fn a_held_rcs_push_reaches_one_ceiling_on_one_two_or_three_axes() {
        let one = rcs_terminal_speed(Vec3::X);
        let two = rcs_terminal_speed(Vec3::new(1.0, 1.0, 0.0));
        let three = rcs_terminal_speed(Vec3::ONE);
        assert!(
            (one - CAP).abs() < 1e-2,
            "one axis settles at the cap: {one}"
        );
        assert!((two - one).abs() < 1e-3, "two axes: {two} vs {one}");
        assert!((three - one).abs() < 1e-3, "three axes: {three} vs {one}");
    }

    /// Anything that slows the hull keeps full RCS authority at the cap and
    /// well past it, so a hull carried overspeed by a well or a maneuver can
    /// always trim back inside the budget.
    #[test]
    fn rcs_braking_keeps_full_authority_at_and_above_the_cap() {
        let brake = Vec3::NEG_X * STEP;
        assert_eq!(budgeted_rcs_delta_v(Vec3::X * CAP, brake, CAP, BAND), brake);
        assert_eq!(
            budgeted_rcs_delta_v(Vec3::X * 40.0, brake, CAP, BAND),
            brake
        );
        // Partly retrograde still slows the hull, so it is still free.
        let oblique = Vec3::new(-1.0, 1.0, 0.0).normalize() * STEP;
        assert_eq!(
            budgeted_rcs_delta_v(Vec3::X * 40.0, oblique, CAP, BAND),
            oblique
        );
    }

    /// At the cap, RCS may still TURN the velocity - it just cannot grow it.
    /// Shrinking the step instead would solve a tangential push to zero, which
    /// on a hull holding the cap kills every perpendicular axis; First Shift's
    /// RCS lesson is a box of four mutually perpendicular legs flown at the
    /// cap, so RCS clamps the RESULT onto the sphere.
    #[test]
    fn rcs_at_the_cap_turns_the_velocity_without_growing_it() {
        let across = Vec3::Y * STEP;
        let at_cap = Vec3::X * CAP;
        let delta = budgeted_rcs_delta_v(at_cap, across, CAP, BAND);
        assert!(
            delta.length() > 0.5 * STEP,
            "a perpendicular axis must stay alive at the cap, got {delta:?}"
        );
        let turned = at_cap + delta;
        assert!(
            (turned.length() - CAP).abs() < 1e-3,
            "the result rides the sphere, not past it: {}",
            turned.length()
        );
        assert!(turned.y > 0.0, "the push went the way it was asked to");

        // Overspeed: held at the speed it arrived with, still free to turn.
        let fast = Vec3::X * 40.0;
        let held = fast + budgeted_rcs_delta_v(fast, across, CAP, BAND);
        assert!(
            (held.length() - 40.0).abs() < 1e-3,
            "an overspeed hull is held, never shoved: {}",
            held.length()
        );

        // Below the cap nothing changed: the push lands whole.
        let free = budgeted_rcs_delta_v(Vec3::X * 5.0, across, CAP, BAND);
        assert!((free - across).length() < 1e-4, "{free:?}");

        // A straight push at the cap still yields nothing.
        let ahead = Vec3::X * STEP;
        assert!(
            budgeted_rcs_delta_v(at_cap, ahead, CAP, BAND).length() < 1e-4,
            "the cap is still a cap"
        );

        // Braking is free at and past the cap, exactly as before.
        let brake = Vec3::NEG_X * STEP;
        assert_eq!(budgeted_rcs_delta_v(fast, brake, CAP, BAND), brake);
    }
}
