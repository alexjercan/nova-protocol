//! Manual piloting: the analog main-drive burn a pilot flies without an
//! autopilot, and the RCS fine-adjust primitive both the pilot and the
//! autopilot's terminal settle drive.

use avian3d::prelude::*;
use bevy::prelude::*;

use super::{
    state::RcsReference,
    thrusters::{balance_throttles, is_forward_aligned, spool, BalanceEngine},
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

/// Fraction of the cap over which the manual burn tapers to zero (the
/// last stretch below the cap). Wide enough to feel like drag, not a wall.
const SPEED_CAP_TAPER_FRACTION: f32 = 0.2;

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

    for (ship, intent, com, speed_cap, rotation, velocity) in &q_ship {
        let mut burn = intent.burn.clamp(0.0, 1.0);

        // The soft speed cap: taper the commanded burn to zero as the
        // velocity component ALONG the burn direction (the hull's world
        // forward - the primary set's thrust axis) approaches the cap.
        // Raw-clock pose (avian Rotation) - this is FixedUpdate.
        if let Some(cap) = speed_cap {
            let burn_direction = rotation.0.mul_vec3(Vec3::NEG_Z);
            let along = velocity.dot(burn_direction);
            let taper_band = (**cap * SPEED_CAP_TAPER_FRACTION).max(1.0);
            burn *= ((**cap - along) / taper_band).clamp(0.0, 1.0);
        }

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

        // Deliver `burn` of the main-drive set's forward thrust, balanced. The
        // uniform throttle `burn` over that set is a feasible split, so a
        // centered drive spools exactly as before; an off-center one is
        // trimmed toward straight flight, recruiting an off-axis engine when
        // the set cannot trim itself.
        let demand: f32 = burn
            * allocation
                .iter()
                .filter(|(_, e)| e.primary)
                .map(|(_, e)| e.forward)
                .sum::<f32>();
        let coeffs: Vec<BalanceEngine> = allocation.iter().map(|(_, e)| *e).collect();
        let throttles = balance_throttles(&coeffs, demand);

        for (thruster, mut input, _, _, &ChildOf(parent)) in &mut q_thruster {
            if parent != ship {
                continue;
            }
            let target = allocation
                .iter()
                .position(|(e, _)| *e == thruster)
                .map(|i| throttles[i])
                .unwrap_or(0.0);
            **input = spool(
                **input,
                target,
                settings.spool_up_rate,
                settings.spool_down_rate,
                dt,
            );
        }
    }
}

/// Reaction-control fine translation: the shared RCS primitive. For a ship
/// carrying a non-zero [`RcsIntent`] (a ship-local desired direction), apply a
/// small, per-axis speed-capped acceleration at the center of mass in that
/// direction, so the pilot - or the autopilot - can nudge the hull the last few
/// meters of a docking approach.
///
/// Two properties define it:
/// - **No torque, geometry-independent.** The push is one linear impulse at the
///   COM ([`Forces::apply_linear_impulse`]), so RCS never rotates the hull and
///   needs no physical side/vertical thrusters - the `Rcs` verb is the fiction
///   that the flight computer has cold-gas quads. The impulse is scaled by mass
///   so `rcs_accel` is a true acceleration and the feel is mass-independent.
/// - **Capped, never free propulsion.** The cap is [`manual_burn_system`]'s
///   speed-cap taper generalized to three signed ship-local axes: a push in a
///   direction the hull already travels at the cap yields nothing, while the
///   opposite direction still accelerates. So RCS can only reshuffle velocity
///   within `+/-cap` per axis, never accumulate speed by spamming it.
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

        // Accumulate the per-axis capped push, then apply once at the COM so
        // the summed impulse still produces zero torque. The cap is per
        // ship-local axis, NOT a speed-magnitude limit: a diagonal command can
        // reach up to `sqrt(2..3) * cap` combined (each axis caps
        // independently), which is fine for docking nudges.
        let mut impulse = Vec3::ZERO;
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            let cmd = intent.0.dot(axis);
            if cmd.abs() < 1e-4 {
                continue;
            }
            let world_axis = rotation.mul_vec3(axis);
            // Residual along the axis, relative to the reference: this is what
            // the cap limits, so a prograde trim of an orbit sees only the
            // small `v - v_orbit` delta, not the full orbital speed.
            let along = (velocity - reference).dot(world_axis);
            // Headroom toward the commanded sign: full push while far from the
            // cap, tapering to zero as the along-axis speed nears the cap in
            // the pushed direction; the opposite direction always has headroom.
            let gate = ((cap - cmd.signum() * along) / taper_band).clamp(0.0, 1.0);
            // Desired acceleration this tick; * mass so the 1/mass inside
            // apply_linear_impulse yields exactly `accel * dt`
            // (mass-independent feel).
            let accel = cmd.clamp(-1.0, 1.0) * settings.rcs_accel * gate;
            impulse += world_axis * (accel * dt * mass);
        }
        if impulse != Vec3::ZERO {
            force.apply_linear_impulse(impulse);
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
