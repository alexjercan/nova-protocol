//! Manual piloting: the analog main-drive burn a pilot flies without an
//! autopilot, and the RCS fine-adjust primitive both the pilot and the
//! autopilot's terminal settle drive. The main burn is plain Newtonian and
//! spends nothing; the delta-v magazine here belongs to RCS alone.
//!
//! Engine units throughout: the magazine counts delta-v delivered to an avian
//! `LinearVelocity`, so it is world units per second and a world unit is 10 m.

use avian3d::prelude::*;
use bevy::{ecs::entity::EntityHashSet, prelude::*};
use nova_gameplay::prelude::*;

use super::{
    capability::{ship_capabilities, ShipCapabilityQuery},
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

/// Manual main-drive burn for intent-carrying ships with no autopilot
/// engaged: allocate the analog burn over the live unbound engine set as a
/// torque-nulling throttle vector, so an off-center or damage-shifted drive
/// still pushes the resultant force through the COM. The forward set delivers
/// the demand via differential throttle when it has headroom; when it does
/// not (the single damage-shifted main drive), the allocator recruits an
/// off-axis engine for pure counter-torque, trading a bounded sideways drift
/// for a straight heading. Only when nothing can help - no headroom and no
/// off-axis engine left - does the ship still pull, held by the PD as before.
///
/// Also writes [`MainDriveCommanded`]: an allocated forward-aligned throttle or
/// a held bound forward thruster. Cleared first, so a skipped ship reads cold.
pub(super) fn manual_burn_system(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    mut q_ship: Query<
        (
            Entity,
            &FlightIntent,
            Option<&mut MainDriveCommanded>,
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
    // Bound thrusters fire straight from their own keys, outside the
    // allocation, so the command reads their input directly.
    q_bound: Query<
        (&ThrusterSectionInput, &Transform, &ChildOf),
        (
            With<ThrusterSectionMarker>,
            With<SpaceshipThrusterInputBinding>,
            Without<SectionInactiveMarker>,
            Without<SpaceshipRootMarker>,
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
    for (ship, intent, mut commanded, com, position, rotation, docked, assembly) in &mut q_ship {
        if let Some(commanded) = commanded.as_deref_mut() {
            commanded.0 = false;
        }
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

        if let Some(commanded) = commanded.as_deref_mut() {
            let allocated = allocation
                .iter()
                .zip(&throttles)
                .any(|((_, engine), &throttle)| engine.primary && throttle > 0.0);
            let bound = q_bound.iter().any(|(input, transform, &ChildOf(parent))| {
                parent == ship
                    && **input > 0.0
                    && engine_direction_local(transform)
                        .is_some_and(|dir| is_forward_aligned(dir, Vec3::NEG_Z))
            });
            commanded.0 = allocated || bound;
        }

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
/// magnitude-limited acceleration at the center of mass in that direction, so
/// the pilot - or the autopilot - can translate the hull without changing its
/// attitude.
///
/// Two properties define it:
/// - **No torque, geometry-independent.** The push is one linear impulse at the
///   COM ([`Forces::apply_linear_impulse`]), so RCS never rotates the hull and
///   needs no physical side/vertical thrusters - the `Rcs` verb is the fiction
///   that the flight computer has cold-gas quads. The impulse is scaled by mass
///   so `rcs_accel` is a true acceleration and the feel is mass-independent.
/// - **A magazine, not a speed limit.** RCS acts at any speed, in any
///   direction, and pays the delta-v it delivers from the driver's
///   [`RcsBudget`]. A push larger than what is left is cut to the remainder,
///   so the magazine never goes negative. It refills only after
///   `rcs_recovery_delay` with no command, so holding a command on an empty
///   magazine keeps it empty.
///
/// Gated on the ship's `rcs_enabled` capability. A docked pair's driver
/// translates the whole pair: each root takes the same delta-v at its own
/// centre of mass, so the pair moves as one body and still does not turn, and
/// only the driver pays. A suppressed partner's intent is not spent and does
/// not hold its magazine off recovery, and a driver with no assembly pushes
/// neither root. A refused command costs nothing.
/// Deliberately NOT gated on `Without<Autopilot>`: the autopilot follow-up
/// drives this very primitive while engaged.
pub(super) fn rcs_burn_system(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    mut q_ship: Query<
        (
            Entity,
            Option<&RcsIntent>,
            &mut RcsBudget,
            &ComputedMass,
            Option<&DockedShip>,
            Option<&DockedAssembly>,
            Forces,
        ),
        With<SpaceshipRootMarker>,
    >,
    q_connections: Query<&DockingConnection>,
    q_capabilities: ShipCapabilityQuery,
    // Commanded drivers this tick: the driver, its partner, and the push.
    mut requests: Local<Vec<(Entity, Option<Entity>, Vec3)>>,
    mut pushes: Local<Vec<(Entity, Vec3)>>,
    // Docked drivers whose missing assembly is already logged, so the error
    // is said once per loss, not at the fixed rate.
    mut missing_assembly: Local<EntityHashSet>,
) {
    // Cleared on every tick, even one that pushes nothing, so the hiss never
    // outlives the thrust.
    for (_, _, mut budget, ..) in &mut q_ship {
        if budget.applied != 0.0 {
            budget.applied = 0.0;
        }
    }
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    requests.clear();
    pushes.clear();
    missing_assembly.retain(|ship| {
        q_ship
            .get(*ship)
            .is_ok_and(|(.., docked, assembly, _)| docked.is_some() && assembly.is_none())
    });
    for (ship, intent, mut budget, mass, docked, assembly, force) in &mut q_ship {
        let command = intent.map_or(Vec3::ZERO, |intent| intent.0);
        let suppressed = docked.is_some_and(|docked| !docked.drives);
        if command == Vec3::ZERO || suppressed {
            budget.recover(dt, &settings);
            continue;
        }
        // A held command holds the magazine off recovery, delivered or not.
        budget.idle = 0.0;
        let partner = match (docked, assembly) {
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
        let mass = assembly.map_or_else(|| mass.value(), |assembly| assembly.mass);
        if !mass.is_finite() || mass <= 0.0 {
            continue;
        }

        // One acceleration budget for every direction: each axis is a unit
        // command, and the whole vector is clamped to unit length, so a
        // three-axis diagonal pushes exactly as hard as a single axis.
        let command = command
            .clamp(Vec3::splat(-1.0), Vec3::splat(1.0))
            .clamp_length_max(1.0);
        let step = force.rotation().mul_vec3(command) * settings.rcs_accel * dt;
        requests.push((ship, partner, step));
    }

    let full_step = settings.rcs_accel * dt;
    for &(ship, partner, step) in requests.iter() {
        // A partner that cannot take its share refuses the whole push, before
        // the driver pays for it.
        if partner.is_some_and(|partner| !q_ship.contains(partner)) {
            continue;
        }
        let Ok((_, _, mut budget, ..)) = q_ship.get_mut(ship) else {
            continue;
        };
        let wanted = step.length();
        if wanted <= 0.0 || full_step <= 0.0 {
            continue;
        }
        let delivered = budget.spend(wanted, &settings);
        if delivered <= 0.0 {
            continue;
        }
        budget.applied = (delivered / full_step).min(1.0);
        let delta_v = step * (delivered / wanted);
        pushes.push((ship, delta_v));
        pushes.extend(partner.map(|partner| (partner, delta_v)));
    }

    for &(root, delta_v) in pushes.iter() {
        let Ok((.., mass, _, _, mut force)) = q_ship.get_mut(root) else {
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
