//! The autopilot: one rule flies STOP, GOTO, GotoPos and ORBIT - compute
//! the desired velocity for the goal, rotate the cheapest engine group onto
//! the velocity error, and burn. Plus the disengage cleanup that cools the
//! engines and parks the helm.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::{
    guidance::{
        arrival_eta, goto_desired_velocity, goto_flip_point, orbit_desired_velocity,
        orbit_plane_normal, orbit_ring_offset, orbit_target_radius, ship_turn_rate, slew_rotation,
        stop_rest_distance,
    },
    state::RcsReference,
    thrusters::{
        balance_throttles, burn_input, choose_group, cluster_thrusters, spool_allocated_thrusters,
        BalanceEngine,
    },
};
use crate::prelude::*;

/// Fraction of `rcs_accel` the local gravity accel must stay under for the
/// autopilot to hand ORBIT station-keeping to the RCS trim. At 0.5 the RCS push
/// has 2x authority over the inward pull, enough headroom to correct a
/// perturbation; above it the main drive (full authority) keeps the orbit. The
/// menu planetoid's ~2.2 u/s^2 pull far exceeds `rcs_accel * 0.5 = 0.75`, so
/// its orbits stay on the main drive.
const RCS_ORBIT_GRAVITY_AUTHORITY: f32 = 0.5;

/// The autopilot. One rule flies every maneuver: compute the desired velocity
/// for the goal, rotate the *cheapest engine group* onto the velocity error
/// (rotation time * bias + burn time; the nose is nothing special), and fire
/// every engine currently inside the alignment cone. The flip-and-burn
/// emerges when the main drive is worth turning for; a retro or lateral
/// group handles what it already points at. Disengages (removes
/// [`Autopilot`]) when the goal is reached, the target is gone, the ship has
/// no engines, or the flight computer (live controller section) is lost.
/// Off-center engine torque is balanced at the source by the wrench allocation
/// ([`balance_throttles`], using each engine's lever arm about the live COM):
/// differential throttle within the firing set when it has headroom,
/// recruiting off-axis engines (laterals, retros) for pure counter-torque when
/// it does not - at the price of a bounded sideways drift the arrival control
/// corrects. The PD holds whatever residual the allocation cannot null.
pub(super) fn autopilot_system(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    gravity_settings: Res<GravitySettings>,
    mut commands: Commands,
    mut q_ship: Query<
        (
            Entity,
            &mut Autopilot,
            &Position,
            &Rotation,
            &LinearVelocity,
            &ComputedMass,
            &ComputedAngularInertia,
            Option<&ComputedCenterOfMass>,
            Option<&ManeuverTelemetry>,
            // RCS terminal settle: the per-hull cap override and the intent the
            // autopilot writes to hand the last-meters brake to the torque-free
            // RCS primitive.
            Option<&RcsSpeedCap>,
            Option<&mut RcsIntent>,
            // RCS error-relative reference: the autopilot writes the orbital
            // velocity here so RCS trims a fast orbit by a sub-cap delta; zero
            // (or absent) everywhere else.
            Option<&mut RcsReference>,
        ),
        With<SpaceshipRootMarker>,
    >,
    // ALL live forward engines, including thrusters with manual per-section
    // bindings (the editor binds keys straight to thrusters): when the computer
    // takes the ship it commands every engine - an editor-built ship would
    // otherwise leave the autopilot with zero authority (it rotated but could
    // never burn). Pressing a bound thruster key is a flight input and
    // disengages instead (see input/player/intent.rs).
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
        ),
    >,
    // A live flight computer is a controller section that still has its PD
    // (preview controllers have none) and is not disabled. Its torque cap is
    // the hull's rotation authority, so the planner reads it too.
    q_computer: Query<
        (&PDController, &ChildOf, Option<&WithheldVerbs>),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    mut q_rotation_input: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        With<ControllerSectionMarker>,
    >,
    // GOTO's goal pose: prefer the target's raw avian Position (a physics body
    // chased at closing speed must be read on the clock of the forces chasing
    // it - in FixedUpdate, GlobalTransform is the previous frame's eased render
    // pose); the GlobalTransform fallback keeps static markers without a
    // physics body navigable.
    q_target: Query<(Option<&Position>, &GlobalTransform, Option<&BodyRadius>)>,
    // ORBIT's well lookup: avian Position (the force system's frame), not
    // GlobalTransform, so the ring the computer flies is the ring gravity
    // pulls on. Without<SpaceshipRootMarker> is a design statement, not an
    // aliasing need: a ship is never an orbit target, even if someone bolts
    // a GravityWell onto one - ORBIT would treat it as "well gone" and
    // disengage. The GOTO arm reads it too (arrival gravity budget +
    // target radius), inheriting the same statement: a ship target never
    // contributes a well radius - ships stay center-relative.
    q_wells: Query<(&Position, &GravityWell), Without<SpaceshipRootMarker>>,
) {
    let dt = time.delta_secs();

    for (
        ship,
        mut autopilot,
        position,
        rotation,
        velocity,
        mass,
        inertia,
        com,
        prev_telemetry,
        rcs_cap_override,
        rcs_intent,
        rcs_reference,
    ) in &mut q_ship
    {
        let has_telemetry = prev_telemetry.is_some();
        // No flight computer, no autopilot - the ship is adrift on manual.
        // The turn-rate budget derives from the strongest live computer (see
        // ship_turn_rate).
        let Some(turn_rate) = ship_turn_rate(
            q_computer
                .iter()
                .filter(|(_, &ChildOf(parent), _)| parent == ship)
                .map(|(pd, _, _)| pd.max_torque),
            inertia,
            &settings,
        ) else {
            debug!("autopilot_system: ship {ship:?} lost its flight computer, disengaging");
            commands.entity(ship).remove::<Autopilot>();
            continue;
        };

        // Every live engine as (world thrust direction, magnitude), plus how
        // hot the hottest one runs (for the settle check). A section's local
        // Transform is its fixed attitude on the hull; engines do not gimbal.
        // Off-center engine torque is balanced below by the wrench allocation
        // (per-engine lever arms about the live COM, off-axis engines
        // recruited for counter-torque when the firing set cannot balance
        // itself); whatever the allocation cannot null the PD still holds
        // within its cap.
        let mut engines: Vec<(Vec3, f32)> = Vec::new();
        let mut hottest_input = 0.0f32;
        for (_, input, magnitude, transform, &ChildOf(parent)) in &q_thruster {
            if parent != ship {
                continue;
            }
            let dir = rotation
                .mul_vec3(transform.rotation.mul_vec3(Vec3::NEG_Z))
                .normalize();
            engines.push((dir, **magnitude));
            hottest_input = hottest_input.max(**input);
        }
        if engines.is_empty() {
            debug!("autopilot_system: ship {ship:?} has no live engines, disengaging");
            commands.entity(ship).remove::<Autopilot>();
            continue;
        }
        let groups = cluster_thrusters(&engines, FORWARD_ALIGNMENT_COS);

        // The arrival curve is planned with the group the computer would
        // actually brake with: its authority sets the deceleration, its
        // rotation distance sets the lead (a retro-equipped ship brakes late
        // and flat; a main-drive-only ship budgets its 180). Shared by GOTO
        // and ORBIT's ring correction.
        let braking_plan = |brake_dir: Vec3, brake_speed: f32| -> (f32, f32) {
            let brake = choose_group(
                &groups,
                brake_dir,
                brake_speed,
                mass.value(),
                dt,
                turn_rate,
                settings.rotation_bias,
            );
            let (brake_authority, brake_angle) = brake
                .map(|g| (g.authority, g.world_dir.angle_between(brake_dir)))
                .unwrap_or((0.0, 0.0));
            let accel = if dt > 0.0 && mass.value() > 0.0 {
                (brake_authority / mass.value()) / dt
            } else {
                0.0
            };
            let lead = brake_angle / turn_rate.max(1e-3) + settings.arrival_spool_pad;
            (accel, lead)
        };

        // The well's GEOMETRIC radius for orbit-band math: the physics
        // body_radius is the nominal designation radius, but a generated
        // body's collider (noise-displaced mesh) can reach well past it -
        // the derived [`BodyRadius`] on the well entity carries that true
        // extent. The band's clearance floor must clear the real rock,
        // not the designation sphere.
        let band_well = |well_entity: Entity, well_data: &GravityWell| -> GravityWell {
            let mut well = well_data.clone();
            well.body_radius = well.body_radius.max(
                q_target
                    .get(well_entity)
                    .ok()
                    .and_then(|(_, _, r)| r.map(|r| **r))
                    .unwrap_or(0.0),
            );
            well
        };

        // ORBIT plans once, on its first engaged tick: target ring from the
        // current radius clamped into the stable band, plane from r x v with
        // the ship-up fallback. The plan then stays sticky - replanning
        // every tick would chase the drift the plan exists to correct.
        if let AutopilotAction::Orbit { well, plan: None } = autopilot.action {
            let Ok((well_position, well_data)) = q_wells.get(well) else {
                debug!("autopilot_system: ORBIT well {well:?} is gone, disengaging");
                commands.entity(ship).remove::<Autopilot>();
                continue;
            };
            let r_vec = position.0 - well_position.0;
            let Some(radius) = orbit_target_radius(
                r_vec.length(),
                &band_well(well, well_data),
                &gravity_settings,
                &settings,
            ) else {
                debug!("autopilot_system: well {well:?} has no stable band, disengaging ORBIT");
                commands.entity(ship).remove::<Autopilot>();
                continue;
            };
            let plan = OrbitPlan {
                radius,
                normal: orbit_plane_normal(r_vec, **velocity, rotation.mul_vec3(Vec3::Y)),
            };
            autopilot.action = AutopilotAction::Orbit {
                well,
                plan: Some(plan),
            };
        }

        // The total well pull fighting a leg that rests at `rest_point` while
        // closing along `closing_dir`, in u/s^2: the sum of every well's
        // positive along-track component (overlapping SOIs add up; a pull that
        // helps braking is ignored, never banked). Evaluated at the rest point
        // - the worst point of a monotonic inward leg. Scanning every well
        // (they are few) instead of the ship's DominantWell matters: the flip
        // is usually planned from OUTSIDE the SOI, where the ship has no
        // DominantWell yet but the goal is already deep in one.
        let gravity_along = |rest_point: Vec3, closing_dir: Vec3| -> f32 {
            q_wells
                .iter()
                .map(|(well_position, well)| {
                    let offset = well_position.0 - rest_point;
                    let pull = well_accel(
                        well.mu,
                        offset.length(),
                        well.body_radius,
                        well.soi_radius,
                        gravity_settings.fade_fraction,
                        gravity_settings.surface_margin,
                    );
                    (offset.normalize_or_zero() * pull)
                        .dot(closing_dir)
                        .max(0.0)
                })
                .sum()
        };

        // The arrival leg shared by GOTO and GotoPos: fly at the goal, come
        // to rest at the standoff - measured from the target's SURFACE
        // (`target_radius`, zero for unsized targets and GotoPos), so a big
        // body is given its size instead of being treated as a point.
        // Published distances are surface-relative too.
        let arrival_desired = |goal: Vec3, target_radius: f32| -> (Vec3, ManeuverTelemetry) {
            let standoff = settings.arrival_standoff + target_radius.max(0.0);
            let to_target = goal - position.0;
            let distance = to_target.length();
            // Zero only if the ship sits exactly on the goal center; the
            // else branch below has distance > standoff > 0, so there the
            // fallback never engages.
            let closing_dir = to_target.normalize_or_zero();
            let closing_speed = velocity.dot(closing_dir);
            // Where the leg rests: the standoff boundary on the closing
            // line. Capped at the ship's own distance so at or inside the
            // envelope it degenerates to the ship position - the computer
            // stops there, it never flies back out to the boundary.
            let park_point = goal - closing_dir * standoff.min(distance);
            if distance <= standoff {
                (
                    Vec3::ZERO,
                    ManeuverTelemetry {
                        goal,
                        goal_entity: None,
                        park_point,
                        distance: (distance - target_radius.max(0.0)).max(0.0),
                        closing_speed,
                        brake_accel: 0.0,
                        flip_point: None,
                        seconds_to_flip: None,
                        eta: None,
                    },
                )
            } else {
                let brake_dir = -closing_dir;
                let brake_speed = velocity.length().max(settings.min_approach_speed);
                let (accel, lead) = braking_plan(brake_dir, brake_speed);
                let gravity = gravity_along(goal - closing_dir * standoff, closing_dir);
                // The published deceleration is the effective one, so any
                // instrument reading it sees the plan the computer actually
                // flies (the field is currently write-only in the HUD).
                // Zero means the pull exceeds the brake authority: no
                // stopping plan (flip/eta are None and the desired velocity
                // is zero - brake flat out).
                let brake_accel = (accel * settings.decel_margin - gravity).max(0.0);
                if brake_accel <= 0.0 && prev_telemetry.is_none_or(|t| t.brake_accel > 0.0) {
                    // Once per degradation entry, not per tick: the
                    // previous published plan still had brake authority.
                    debug!(
                        "autopilot_system: well pull {gravity} exceeds brake authority \
                         on the arrival leg of {ship:?}; no stopping plan"
                    );
                }
                let flip = goto_flip_point(
                    distance,
                    closing_speed,
                    accel * settings.decel_margin,
                    lead,
                    standoff,
                    gravity,
                );
                let eta = arrival_eta(
                    distance,
                    closing_speed,
                    accel * settings.decel_margin,
                    lead,
                    standoff,
                    gravity,
                );
                (
                    goto_desired_velocity(
                        to_target,
                        standoff,
                        accel,
                        settings.decel_margin,
                        lead,
                        settings.min_approach_speed,
                        gravity,
                    ),
                    ManeuverTelemetry {
                        goal,
                        goal_entity: None,
                        park_point,
                        distance: (distance - target_radius.max(0.0)).max(0.0),
                        closing_speed,
                        brake_accel,
                        flip_point: flip.map(|(from_goal, _)| goal - closing_dir * from_goal),
                        seconds_to_flip: flip.map(|(_, seconds)| seconds),
                        eta,
                    },
                )
            }
        };

        // The goal, as a desired velocity right now. GOTO and STOP legs
        // also publish their live numbers as [`ManeuverTelemetry`] for the
        // HUD instruments; ORBIT (and a settled STOP) clears it.
        let mut telemetry: Option<ManeuverTelemetry> = None;
        // Set by the Goto arm when the ship is inside the park envelope;
        // gates the ORBIT handoff in the done branch.
        let mut goto_arrived = false;
        // Set by the Orbit arm: gates the error-relative RCS trim, which only
        // applies while station-keeping - the desired is a fast orbital
        // velocity, not a rest goal.
        let mut is_orbit = false;
        // The local gravitational acceleration at the orbiting ship, `mu/r^2`,
        // set by the Orbit arm. The RCS trim may only take the orbit when it
        // has clear authority over this pull.
        let mut orbit_gravity_accel = 0.0f32;
        let desired = match autopilot.action {
            AutopilotAction::Stop => {
                // STOP has a spatial goal too: the predicted rest point.
                // Publish it so the instruments (readout chip, trajectory
                // ribbon) cover the braking leg; near rest there is no leg
                // left and the telemetry clears. Hysteresis on the gate:
                // a ship hovering at the threshold (gravity re-accelerating
                // it, engines still winding down) must not strobe the
                // instruments, so a leg starts at twice the epsilon and
                // holds until the epsilon itself.
                let speed = velocity.length();
                let publish = if has_telemetry {
                    speed > settings.stop_speed_epsilon
                } else {
                    speed > 2.0 * settings.stop_speed_epsilon
                };
                if publish {
                    let brake_dir = -velocity.normalize();
                    // The plan's group choice floors the speed at
                    // min_approach_speed (a lead planned for a crawling
                    // ship is meaningless); the rest distance itself uses
                    // the raw speed - slight overestimate at low speed,
                    // documented asymmetry.
                    let (accel, lead) =
                        braking_plan(brake_dir, speed.max(settings.min_approach_speed));
                    // STOP's pull budget is evaluated at the ship, not the
                    // (yet-unknown) rest point - honest enough for a
                    // telemetry-only prediction, and the leg replans every
                    // tick anyway.
                    let gravity = gravity_along(position.0, velocity.normalize());
                    let effective = (accel * settings.decel_margin - gravity).max(0.0);
                    if let Some(rest) =
                        stop_rest_distance(speed, accel * settings.decel_margin, lead, gravity)
                    {
                        let goal = position.0 + velocity.normalize() * rest;
                        telemetry = Some(ManeuverTelemetry {
                            goal,
                            goal_entity: None,
                            // A STOP has no standoff: the predicted rest
                            // point IS the park point.
                            park_point: goal,
                            distance: rest,
                            closing_speed: speed,
                            brake_accel: effective,
                            flip_point: None,
                            seconds_to_flip: None,
                            eta: Some(lead + (speed + gravity * lead) / effective.max(1e-3)),
                        });
                    }
                }
                Vec3::ZERO
            }
            AutopilotAction::Goto { target } => {
                let Ok((target_position, target_transform, body_radius)) = q_target.get(target)
                else {
                    debug!("autopilot_system: GOTO target {target:?} is gone, disengaging");
                    commands.entity(ship).remove::<Autopilot>();
                    continue;
                };
                // The target's size, from whichever source it carries:
                // the authored BodyRadius and/or the well's body_radius.
                // Max is conservative if they ever disagree; unsized
                // targets stay at zero (center-relative, unchanged).
                let target_radius = body_radius.map_or(0.0, |r| **r).max(
                    q_wells
                        .get(target)
                        .map_or(0.0, |(_, well)| well.body_radius),
                );
                let goal_position = target_position
                    .map(|p| p.0)
                    .unwrap_or_else(|| target_transform.translation());
                let (desired, mut numbers) = arrival_desired(goal_position, target_radius);
                // Arrived means INSIDE the park envelope, not merely
                // "wants zero velocity": the degraded no-stopping-plan
                // state also zeroes the desired velocity arbitrarily far
                // out, and a done-at-apex there must release (as it
                // always did), never park into an orbit whose ring
                // correction assumes it starts near the ring. The
                // published distance is surface-relative, so the
                // envelope test is against the bare standoff.
                goto_arrived = numbers.distance <= settings.arrival_standoff;
                numbers.goal_entity = Some(target);
                telemetry = Some(numbers);
                desired
            }
            AutopilotAction::GotoPos { position } => {
                // A bare position has no size: center-relative, as before.
                let (desired, numbers) = arrival_desired(position, 0.0);
                telemetry = Some(numbers);
                desired
            }
            AutopilotAction::Orbit { well, plan } => {
                let Ok((well_position, well_data)) = q_wells.get(well) else {
                    debug!("autopilot_system: ORBIT well {well:?} is gone, disengaging");
                    commands.entity(ship).remove::<Autopilot>();
                    continue;
                };
                // Unreachable by construction: the plan block above either
                // filled the plan this tick or disengaged. The skip is
                // defensive only.
                let Some(plan) = plan else { continue };
                is_orbit = true;
                let r_vec = position.0 - well_position.0;
                // Local gravity accel `mu/r^2` - the inward pull the RCS trim
                // would have to counter if it took the orbit.
                orbit_gravity_accel = well_data.mu / r_vec.length_squared().max(1e-3);
                let to_ring = orbit_ring_offset(r_vec, &plan);
                let brake_dir = -to_ring
                    .try_normalize()
                    .unwrap_or_else(|| -r_vec.normalize_or(Vec3::X));
                let brake_speed = velocity.length().max(settings.min_approach_speed);
                let (accel, lead) = braking_plan(brake_dir, brake_speed);
                orbit_desired_velocity(
                    r_vec,
                    &plan,
                    well_data.mu,
                    accel,
                    settings.decel_margin,
                    lead,
                )
            }
        };

        // Keep the published telemetry in step with the engaged verb: GOTO and
        // moving STOP legs update it every tick; ORBIT and a settled STOP clear
        // a stale one (disengage clears via remove_maneuver_telemetry).
        match telemetry {
            Some(numbers) => {
                commands.entity(ship).try_insert(numbers);
            }
            None if has_telemetry => {
                commands.entity(ship).remove::<ManeuverTelemetry>();
            }
            None => {}
        }

        let error = desired - **velocity;
        let error_speed = error.length();
        let error_dir = (error_speed > 1e-3).then(|| error / error_speed);

        // RCS terminal settle: when the maneuver's GOAL is rest (STOP,
        // GOTO/GotoPos inside the standoff - `desired ~= 0`) and the ship is
        // already slow enough for the speed-capped RCS to act (`|v| < cap`),
        // hand the last-meters brake to the RCS primitive - a torque-free COM
        // push - instead of the main drive. Gated on the ship granting the
        // `Rcs` verb, so a hull without it (the mainline campaign, RCS disabled
        // pending rework) keeps the exact main-drive arrival.
        //
        // Two RCS branches share one command formula (`error / rcs_cap`,
        // proportional toward `desired`), differing only in the cap's reference
        // frame:
        //
        // - SETTLE: the maneuver's GOAL is rest (STOP, GOTO/GotoPos inside
        //   the standoff - `desired ~= 0`) and the ship is already slow enough
        //   for the ABSOLUTE cap to act (`|v| < cap`). The reference is zero,
        //   so RCS brakes the last meters to rest.
        // - ORBIT trim: station-keeping, where `desired` is the orbital
        //   velocity (~2.5-6 u/s, above the cap). The RESIDUAL
        //   `error = desired - v` is what must be sub-cap for RCS to act, and the
        //   reference is `desired`, so `rcs_burn_system` caps `v - desired` (the
        //   trim) instead of the absolute orbital speed. While the residual is
        //   above the cap (spinning up, or a big ring correction), the main drive
        //   does the work exactly as before.
        //
        // Both hand the burn to the torque-free RCS COM push and spool the main
        // drive down; both are gated on the ship granting the `Rcs` verb, so a
        // hull without it (the mainline campaign, RCS disabled pending rework)
        // keeps the exact main-drive behavior.
        let rcs_cap = rcs_cap_override
            .map(|c| c.0)
            .unwrap_or(settings.rcs_speed_cap);
        let rcs_granted = q_computer.iter().any(|(_, &ChildOf(parent), withheld)| {
            parent == ship && withheld.is_none_or(|w| w.granted(FlightVerb::Rcs))
        });
        let rcs_capable = rcs_granted && rcs_cap > 0.0 && error_speed > 1e-3;
        let use_rcs_settle = rcs_capable
            && desired.length() <= settings.stop_speed_epsilon
            && velocity.length() < rcs_cap;
        // The RCS trim takes the orbit only where it has CLEAR authority over
        // the local gravity: its `rcs_accel` push must comfortably exceed the
        // inward pull `mu/r^2`, or a perturbed ship spirals into the well
        // faster than RCS can correct - the menu ambience ships crashing the
        // asteroid. In a strong well the main drive (full authority) keeps the
        // orbit, exactly as it did before the RCS trim.
        let rcs_has_orbit_authority =
            orbit_gravity_accel < settings.rcs_accel * RCS_ORBIT_GRAVITY_AUTHORITY;
        let use_rcs_orbit =
            rcs_capable && is_orbit && rcs_has_orbit_authority && error_speed < rcs_cap;
        let use_rcs = use_rcs_settle || use_rcs_orbit;
        // The reference the cap is measured against: the orbital velocity while
        // trimming an orbit, zero otherwise (absolute cap). Written EVERY tick
        // so a stale orbital reference never lingers into a settle or the
        // player.
        let rcs_reference_v = if use_rcs_orbit { desired } else { Vec3::ZERO };
        // Proportional command toward `desired`, scaled so a cap-sized residual
        // is full deflection; fades to zero as the residual does (no
        // overshoot). Clear to zero when not using RCS so a stale nudge never
        // lingers.
        let rcs_command = if use_rcs {
            (rotation.inverse() * error / rcs_cap).clamp(Vec3::splat(-1.0), Vec3::splat(1.0))
        } else {
            Vec3::ZERO
        };
        if let Some(mut intent) = rcs_intent {
            intent.0 = rcs_command;
        } else if use_rcs {
            commands.entity(ship).insert(RcsIntent(rcs_command));
        }
        if let Some(mut reference) = rcs_reference {
            reference.0 = rcs_reference_v;
        } else if use_rcs_orbit {
            commands.entity(ship).insert(RcsReference(rcs_reference_v));
        }

        // The allocation set: EVERY live engine, with the coefficients the
        // balancer needs per unit input - signed thrust along the burn, force
        // perpendicular to it, and lever-arm torque about the live COM. The
        // engines inside the alignment cone of the needed burn are the
        // *primary* set (lit engines keep a slightly looser gate - hysteresis
        // via their own spooled input - so the plume does not flicker at the
        // boundary): they define the deliverable authority and receive the
        // demand. Everything else - laterals, retros - is a counter-torque
        // candidate the balancer may recruit when the primary set cannot
        // balance itself (the single damage-shifted main drive). The COM is
        // body-local; lift it to world with rotation + translation (never
        // render scale).
        let com_world = com
            .map(|c| rotation.mul_vec3(c.0) + position.0)
            .unwrap_or(position.0);
        let mut firing_authority = 0.0f32;
        let mut allocation: Vec<(Entity, BalanceEngine)> = Vec::new();
        if let Some(error_dir) = error_dir {
            for (thruster, input, magnitude, transform, &ChildOf(parent)) in &q_thruster {
                if parent != ship {
                    continue;
                }
                let dir = rotation
                    .mul_vec3(transform.rotation.mul_vec3(Vec3::NEG_Z))
                    .normalize();
                let gate = if **input > 0.1 {
                    settings.align_cos - settings.align_hysteresis
                } else {
                    settings.align_cos
                };
                let aligned = dir.dot(error_dir);
                let primary = aligned >= gate;
                if primary {
                    firing_authority += **magnitude;
                }
                // World point of the engine (direct child of the root): raw
                // root pose composed with the local mount, and
                // thruster_impulse_system pushes from this SAME composition -
                // the lever arm about com_world matches the torque physics
                // applies by construction, never through a render-clock
                // GlobalTransform.
                let pos_world = position.0 + rotation.mul_vec3(transform.translation);
                let torque = (pos_world - com_world).cross(dir * **magnitude);
                // A recruit's whole thrust vector is off-plan force (see
                // BalanceEngine); a primary engine contributes its aligned
                // share to the demand and only the perpendicular rest to the
                // penalty.
                let (forward, lateral) = if primary {
                    (
                        **magnitude * aligned,
                        (dir - aligned * error_dir) * **magnitude,
                    )
                } else {
                    (0.0, dir * **magnitude)
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
        }

        // Within the deadband the leftover is a crumb: never re-aim the hull
        // for it - any engine already on the error finishes it, and a residual
        // only a rotation could remove is accepted. This is what stops the ship
        // twitching after perfection. Legs that END AT REST (STOP, GOTO,
        // GotoPos) use the wider settle band: the endgame of a translation leg
        // lives in sub-u/s errors - the brake tail, the boundary creep, the
        // doorstep residual - and chasing those with attitude swings was the
        // "wobbles on GOTO" playtest. Scoping by LEG, not by desired == 0, is
        // deliberate: the hunt's onset is in the brake tail where desired is
        // still nonzero - the desired-zero scoping was tried and left the
        // terminal spin bit-for-bit unchanged. Only ORBIT keeps the tight band:
        // station-keeping is the one regime whose job is chasing small errors
        // forever.
        let crumb_band = match autopilot.action {
            AutopilotAction::Orbit { .. } => settings.attitude_deadband,
            _ => settings.settle_deadband.max(settings.attitude_deadband),
        };
        let fine = error_speed <= crumb_band;

        // Done: the goal wants rest here and the ship is at rest - exactly,
        // or within the deadband with no engine on the residual. Release
        // only once the engines have wound down: a still-hot, spooling-down
        // drive would push the ship off again. ORBIT never completes: an
        // orbit is not a destination, the computer station-keeps until
        // breakout, Z, or a capability loss.
        let done = !matches!(autopilot.action, AutopilotAction::Orbit { .. })
            && desired == Vec3::ZERO
            && (error_speed <= settings.stop_speed_epsilon || (fine && firing_authority <= 0.0));
        if done && hottest_input <= 0.05 {
            // A GOTO that arrived at a well body parks into orbit instead of
            // handing back a ship that immediately starts falling: the one-key
            // parking flow becomes zero-key when the computer was already told
            // where to go. engage() resets the phase. The ring is planned HERE,
            // from the leg's intent - the park point, standoff above the
            // (geometric) surface - never from wherever terminal creep dragged
            // the ship: a plan-from-current-radius could ring at the band
            // bottom, and the insertion from a crept position has been seen to
            // graze the rock. max with the current radius so a ship that
            // settled slightly outside the park point is not corrected inward.
            // Breakout semantics (any flight input, Z) are ORBIT's own,
            // unchanged. Everything else - GotoPos, well-less targets, STOP, a
            // bandless well - releases as before.
            if let AutopilotAction::Goto { target } = autopilot.action {
                if goto_arrived {
                    if let Ok((well_position, well_data)) = q_wells.get(target) {
                        let well = band_well(target, well_data);
                        let r_vec = position.0 - well_position.0;
                        let park = well.body_radius + settings.arrival_standoff;
                        if let Some(radius) = orbit_target_radius(
                            park.max(r_vec.length()),
                            &well,
                            &gravity_settings,
                            &settings,
                        ) {
                            debug!(
                                "autopilot_system: ship {ship:?} arrived, parking into \
                                 ORBIT at ring {radius}"
                            );
                            *autopilot = Autopilot::engage(AutopilotAction::Orbit {
                                well: target,
                                plan: Some(OrbitPlan {
                                    radius,
                                    normal: orbit_plane_normal(
                                        r_vec,
                                        **velocity,
                                        rotation.mul_vec3(Vec3::Y),
                                    ),
                                }),
                            });
                            continue;
                        }
                    }
                }
            }
            debug!("autopilot_system: ship {ship:?} maneuver complete, disengaging");
            commands.entity(ship).remove::<Autopilot>();
            continue;
        }

        // Rotate the cheapest group onto the error (only for corrections worth
        // turning for), then allocate the shared burn demand across the whole
        // live engine set as a torque-nulling throttle vector. While settling
        // (done, engines still winding down) command zero to every engine.
        let mut throttles: Vec<f32> = vec![0.0; allocation.len()];
        let mut burning = false;
        if let (Some(error_dir), false) = (error_dir, done) {
            if !fine {
                if let Some(chosen) = choose_group(
                    &groups,
                    error_dir,
                    error_speed,
                    mass.value(),
                    dt,
                    turn_rate,
                    settings.rotation_bias,
                ) {
                    // The command evolves from ITS OWN previous state, never
                    // from the hull: rotate the command so it carries the
                    // chosen group onto the burn, slewed at the estimated turn
                    // rate (see slew_rotation - a 180 step would drive the PD
                    // into undamped saturation). Anchoring to the command
                    // instead of the hull also regulates roll: a command
                    // rebuilt from the hull each tick inherits the hull's roll,
                    // the PD then sees zero roll error, and roll picked up
                    // during a flip spins the ship like a drill forever.
                    let local_dir = rotation.inverse().mul_vec3(chosen.world_dir);
                    // Turn gently when little burn remains: the ending turn is
                    // what the hull is still spinning with at release, and a
                    // slow final swing keeps that residual under
                    // RELEASE_SPIN_EPSILON. Keyed to the same regime-scoped
                    // crumb band as `fine`: on a rest leg the brake tail's
                    // few-u/s corrections must swing the hull GENTLY or each
                    // re-aim overshoots and seeds the next (the arrival hunt
                    // cascade). NOTE: the deadband A/B moved this
                    // denominator together with the band - keying only the
                    // band left the terminal spin unchanged.
                    let urgency = (error_speed / (crumb_band * 8.0)).clamp(0.25, 1.0);
                    let max_step = turn_rate * dt * urgency;
                    for (mut input, &ChildOf(parent)) in &mut q_rotation_input {
                        if parent == ship {
                            let command = **input;
                            let command_dir = command.mul_vec3(local_dir);
                            let goal = Quat::from_rotation_arc(command_dir, error_dir) * command;
                            **input = slew_rotation(command, goal, max_step);
                        }
                    }
                }
            }
            // The shared demand this tick: the impulse the maneuver wants,
            // capped by the firing set's authority (burn_input * authority =
            // min(impulse, authority)). balance_throttles delivers it through
            // the firing set and nulls the net torque about the COM,
            // recruiting off-axis engines when the firing set cannot.
            //
            // Spool-tail cutoff for legs ending at rest: a throttle commanded
            // to zero still delivers ~magnitude * input^2 / (2 *
            // spool_down_rate * dt) of impulse while it winds down, so a
            // finishing burn that keeps demanding until the error reads zero
            // integrates THROUGH zero - the ship exits its own standoff
            // backwards, the re-entry error re-aims the hull, and the arrival
            // bounces on the boundary in a limit cycle (previously masked by
            // the accidental dither of the cross-clock command handoff). Once
            // the wind-down tail alone covers the remaining error, the correct
            // demand is zero: cut and coast to rest.
            let mut tail_dv = 0.0;
            if desired == Vec3::ZERO && dt > 0.0 && mass.value() > 0.0 {
                for (_, input, magnitude, transform, &ChildOf(parent)) in &q_thruster {
                    if parent != ship {
                        continue;
                    }
                    let dir = rotation
                        .mul_vec3(transform.rotation.mul_vec3(Vec3::NEG_Z))
                        .normalize();
                    tail_dv += dir.dot(error_dir).max(0.0) * **magnitude * **input * **input
                        / (2.0 * settings.spool_down_rate * dt)
                        / mass.value();
                }
            }
            let demand = if use_rcs {
                // The RCS COM push is braking the last meters; the main drive
                // spools down so the two never double-push.
                0.0
            } else if desired == Vec3::ZERO && error_speed <= tail_dv {
                0.0
            } else {
                firing_authority * burn_input(error_speed * mass.value(), firing_authority)
            };
            let coeffs: Vec<BalanceEngine> = allocation.iter().map(|(_, e)| *e).collect();
            throttles = balance_throttles(&coeffs, demand);
            burning = throttles.iter().any(|&u| u > 0.0);
        }

        autopilot.phase = match autopilot.action {
            // ORBIT reports Hold once the velocity error is inside the hold
            // tolerance, with hysteresis so the label does not flicker at
            // the boundary. Micro-burns still fire inside Hold (the
            // attitude deadband, not the hold gate, decides burning) - that
            // IS station-keeping.
            AutopilotAction::Orbit { .. } => {
                let holding = if autopilot.phase == AutopilotPhase::Hold {
                    error_speed <= settings.orbit_hold_exit
                } else {
                    error_speed <= settings.orbit_hold_enter
                };
                if holding {
                    AutopilotPhase::Hold
                } else if burning {
                    AutopilotPhase::Burn
                } else {
                    AutopilotPhase::Align
                }
            }
            _ if burning => AutopilotPhase::Burn,
            _ => AutopilotPhase::Align,
        };

        // Spool every engine toward its allocated throttle (zero for engines
        // the allocation left dark, and for everything while settling).
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

/// When the autopilot lets go - completion or any breakout - it cools the
/// engines it was driving and parks the helm on the hull's current attitude.
/// Nothing else writes a *bound* thruster's input between key events (the
/// manual burn system deliberately leaves bound thrusters to their own
/// keys), so a residual autopilot burn would otherwise ghost on forever; and
/// a rotation command abandoned mid-maneuver can sit ~180 degrees from the
/// hull, which parks the saturated PD in its degenerate zone where it
/// sustains a perpetual roll instead of damping the leftover spin.
pub(super) fn on_autopilot_removed_cool_engines(
    remove: On<Remove, Autopilot>,
    q_ship: Query<&Rotation, With<SpaceshipRootMarker>>,
    mut q_thruster: Query<(&mut ThrusterSectionInput, &ChildOf), With<ThrusterSectionMarker>>,
    mut q_rotation_input: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        With<ControllerSectionMarker>,
    >,
    mut q_rcs: Query<(&mut RcsIntent, Option<&mut RcsReference>)>,
) {
    for (mut input, &ChildOf(parent)) in &mut q_thruster {
        if parent == remove.entity {
            **input = 0.0;
        }
    }
    // Clear the RCS command AND its error-relative reference: the autopilot
    // writes both while settling/trimming, and rcs_burn_system acts on ANY
    // non-zero intent regardless of autopilot state. A residual intent would
    // push the ship past rest toward the cap; a stale orbital reference would
    // silently rebase the player's next absolute-cap nudge. Zero both on
    // disengage.
    if let Ok((mut intent, reference)) = q_rcs.get_mut(remove.entity) {
        intent.0 = Vec3::ZERO;
        if let Some(mut reference) = reference {
            reference.0 = Vec3::ZERO;
        }
    }
    if let Ok(rotation) = q_ship.get(remove.entity) {
        for (mut input, &ChildOf(parent)) in &mut q_rotation_input {
            if parent == remove.entity {
                **input = rotation.0;
            }
        }
    }
}
