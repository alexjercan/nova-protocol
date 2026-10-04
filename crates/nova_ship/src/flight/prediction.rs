//! The flight prediction: the path the player's autopilot will fly a GOTO,
//! GotoPos or STOP leg on, found by flying the leg ahead of the ship.
//!
//! The predictor does not model the autopilot. It runs the live chain itself,
//! one fixed tick at a time: [`autopilot_step`], then [`step_flight_body`],
//! which applies the RCS push, the PD torque, the dominant-well pull and the
//! thruster impulses the live systems apply, and integrates them the way avian
//! 0.7 does. The predicted centre of mass therefore matches the flown one for
//! as long as the world the run froze at its seed holds: a moving target or
//! well, a collision, a docking or a change to the hull is not predicted, and
//! the moving cases hide the prediction instead.
//!
//! Engine units throughout, as in the autopilot.

use std::time::Duration;

use avian3d::{dynamics::integrator::solve_gyroscopic_torque, math::MatExt, prelude::*};
use bevy::{ecs::entity::EntityHashSet, prelude::*};
use nova_gameplay::prelude::*;

use super::{
    autopilot::{
        arrival_target, autopilot_step, flight_body, flight_helm, well_samples, ArrivalTarget,
        ArrivalTargetQuery, AutopilotEndType, FlightBody, FlightEnvironment, FlightHelm,
        FlightWellQuery, HelmControllerQuery, HelmThrusterQuery, WellSample,
    },
    capability::{ship_capabilities, LiveFlightComputers, ShipCapabilityQuery},
    manual::rcs_push,
};
use crate::{physics::compute_pd_torque, prelude::*, sections::thruster_section::engine_direction};

/// How far ahead a prediction flies, seconds.
const PREDICTION_HORIZON: f32 = 30.0;

/// Predicted ticks one [`predict_flight_path`] pass advances a run. A pass runs
/// on each live fixed tick, or on each rendered frame while `Time<Virtual>` is
/// paused. A run to the 30 s horizon at 64 Hz is 1920 ticks, so it publishes
/// `1920 / PREDICTED_TICKS_PER_FIXED_TICK` passes after its seed: 16 fixed
/// updates or paused frames at 120. A leg that ends before the horizon
/// publishes sooner. A measured 240-tick pass exceeded the approved 1 ms
/// per-pass bound, so the pass is 120 ticks.
const PREDICTED_TICKS_PER_FIXED_TICK: u32 = 120;

/// Predicted ticks between two regular points of a prediction.
const SAMPLE_TICKS: u32 = 8;

/// The path the player's autopilot will fly its GOTO, GotoPos or STOP leg on.
///
/// Flown ahead through the same autopilot step, actuator chain and avian
/// integration as the live ship, from the ship's state at [`Self::seed_time`];
/// see the module docs for what the run cannot see. Present only while such a
/// leg is engaged on an undocked player ship with a live flight computer, a
/// live engine and a measured mass, and only while the target and every well
/// are at rest. Replaced by each finished run.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct FlightPrediction {
    /// The flown centre of mass, world space: the seed state first, then one
    /// point every [`Self::sample_interval`]. When the leg ends between two
    /// samples, the last point is the state it ends at, at
    /// [`Self::final_point_time`].
    pub points: Vec<Vec3>,
    /// The first point at or after the tick the leg starts braking. `None`
    /// when the leg does not brake inside the prediction.
    pub flip_index: Option<usize>,
    /// The `Time<Fixed>` elapsed time of the seed state, `points[0]`.
    pub seed_time: Duration,
    /// Seconds between two regular points.
    pub sample_interval: f32,
    /// Seconds from [`Self::seed_time`] to the last point: the predicted tick
    /// it was flown at, which is off the [`Self::sample_interval`] grid when
    /// the leg ends between two samples.
    pub final_point_time: f32,
    /// How the predicted leg ends.
    pub end: FlightPredictionEndType,
}

/// How a predicted leg ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum FlightPredictionEndType {
    /// The leg arrives and the autopilot lets go.
    Completed,
    /// The leg arrives at a well body and the autopilot parks into ORBIT.
    ParkedIntoOrbit,
    /// The autopilot disengages before the leg arrives.
    Disengaged,
    /// The leg still flies at the prediction horizon. The path stops there; it
    /// is not drawn on to the goal.
    Horizon,
}

/// The player ship's prediction run. Each [`predict_flight_path`] pass advances
/// it until it ends and is published as a [`FlightPrediction`]. A published
/// run stays with its points taken and its leg kept until the next pass
/// reseeds it: a reseed for the same leg keeps the published path visible until
/// the new run replaces it, and a different leg removes the path at once. A
/// failed non-finite run instead keeps empty points and no published path.
/// Neither run reseeds for the same leg on the frozen fixed tick.
#[derive(Component)]
pub(super) struct FlightPredictionRun {
    /// The leg the run was seeded for. A different engaged leg restarts it.
    action: AutopilotAction,
    /// The predicted autopilot.
    autopilot: Autopilot,
    /// The predicted mover.
    body: FlightBody,
    /// The predicted actuators.
    helm: FlightHelm,
    /// The root's avian mass, for the impulses.
    mass: ComputedMass,
    /// The leg's goal, frozen at the seed.
    target: Option<ArrivalTarget>,
    /// The telemetry the last predicted tick published.
    telemetry: Option<ManeuverTelemetry>,
    /// The wells the autopilot reads, frozen at the seed.
    wells: Vec<WellSample>,
    /// The wells the gravity force system reads, frozen at the seed; empty
    /// when the ship feels no well.
    gravity_wells: Vec<(Entity, Vec3, GravityWell)>,
    /// The well that owned the last predicted pull.
    dominant_well: Option<Entity>,
    /// The mover's resolved arrival margin.
    arrival_standoff: f32,
    /// What the mover is permitted to do.
    capabilities: ShipCapabilities,
    /// Predicted ticks flown.
    ticks: u32,
    /// The first predicted tick whose state the leg brakes from.
    braking_tick: Option<u32>,
    /// See [`FlightPrediction::seed_time`].
    seed_time: Duration,
    /// See [`FlightPrediction::points`].
    points: Vec<Vec3>,
}

/// One fixed tick of the live chain after [`autopilot_step`] for an undocked
/// root, applied to `body` and `helm` in the live order: `rcs_burn_system`'s
/// push, the PD torque of every live flight computer, `well_pull` (the
/// dominant-well acceleration at the root origin), every thruster impulse,
/// then avian's `substeps` (count, substep duration) of semi-implicit Euler
/// with gyroscopic motion, and its writeback about the centre of mass.
/// `avian_gravity` is avian's global [`Gravity`].
///
/// The mirror follows the live f32 operation order and is checked against
/// flown COM positions with an explicit error bound. It assumes a ship root
/// with no `LockedAxes`, damping or `GravityScale`, as ships spawn today. A change to any of those systems, a ship root that gains
/// one of those components, or an avian upgrade must change this mirror with
/// it;
/// `flight_prediction_follows_the_flown_center_of_mass_within_one_unit` fails
/// when the two drift apart.
pub(super) fn step_flight_body(
    body: &mut FlightBody,
    helm: &mut FlightHelm,
    mass: &ComputedMass,
    well_pull: Vec3,
    env: &FlightEnvironment,
    (substep_count, substep): (u32, Duration),
    avian_gravity: Vec3,
) {
    let settings = env.settings;
    let dt = env.dt;
    let rotation = Rotation(body.rotation);
    let inverse_inertia = body.angular_inertia.rotated(body.rotation).inverse();
    let mut linear_velocity = body.linear_velocity;
    let mut angular_velocity = body.angular_velocity;
    // `Forces::apply_linear_impulse`.
    let apply_linear_impulse = |velocity: &mut Vec3, impulse: Vec3| {
        if impulse != Vec3::ZERO {
            *velocity += Vec3::splat(mass.inverse()) * impulse;
        }
    };

    // rcs_burn_system.
    helm.rcs_budget.applied = 0.0;
    if dt > 0.0 {
        let command = helm.rcs_intent;
        if command == Vec3::ZERO {
            helm.rcs_budget.recover(dt, settings);
        } else {
            helm.rcs_budget.idle = 0.0;
            let root_mass = mass.value();
            if env.capabilities.rcs_enabled && root_mass.is_finite() && root_mass > 0.0 {
                if let Some(delta_v) =
                    rcs_push(command, rotation, &mut helm.rcs_budget, settings, dt)
                {
                    apply_linear_impulse(&mut linear_velocity, delta_v * root_mass);
                }
            }
        }
    }

    // gravity_well_system, then the PD pass and its torque, which read the
    // velocity before the thruster impulses.
    let mut linear_increment = Vec3::ZERO;
    let mut angular_increment = Vec3::ZERO;
    if well_pull != Vec3::ZERO {
        linear_increment += well_pull;
    }
    let (principal, local_frame) = body
        .angular_inertia
        .principal_angular_inertia_with_local_frame();
    for controller in &helm.controllers {
        let Some(pd) = controller.live_pd else {
            continue;
        };
        let torque = compute_pd_torque(
            pd.frequency,
            pd.damping_ratio,
            pd.max_angular_acceleration,
            pd.sustained_angular_speed,
            body.rotation,
            controller.command,
            angular_velocity,
            principal,
            local_frame,
        );
        if torque != Vec3::ZERO {
            angular_increment += inverse_inertia * torque;
        }
    }

    // thruster_impulse_system.
    let center_of_mass = body.position + rotation * body.local_center_of_mass;
    for engine in &helm.engines {
        let Some(thrust_direction) = engine_direction(&rotation, &engine.mount) else {
            continue;
        };
        let thrust_impulse = thrust_direction * engine.magnitude * engine.input.clamp(0.0, 1.0);
        let world_point = body.position + rotation.mul_vec3(engine.mount.translation);
        apply_linear_impulse(&mut linear_velocity, thrust_impulse);
        let angular_impulse = (world_point - center_of_mass).cross(thrust_impulse);
        if angular_impulse != Vec3::ZERO {
            angular_velocity += inverse_inertia * angular_impulse;
        }
    }

    // avian: pre_process_velocity_increments with no damping, gravity scale or
    // locked axes; integrate_velocities and integrate_positions per substep;
    // writeback_solver_bodies.
    let substep_secs_f64 = substep.as_secs_f64() as f32;
    let substep_secs = substep.as_secs_f32();
    let damping = 1.0 / (1.0 + substep_secs_f64 * 0.0);
    linear_increment += avian_gravity * 1.0;
    linear_increment *= substep_secs_f64;
    angular_increment *= substep_secs_f64;
    let gyroscopic = !body.angular_inertia.inverse().is_isotropic(1e-6);
    let mut delta_position = Vec3::ZERO;
    let mut delta_rotation = Rotation::IDENTITY;
    for _ in 0..substep_count {
        linear_velocity *= damping;
        angular_velocity *= damping;
        linear_velocity += linear_increment;
        angular_velocity += angular_increment;
        if gyroscopic {
            solve_gyroscopic_torque(
                &mut angular_velocity,
                delta_rotation.0 * rotation.0,
                &body.angular_inertia,
                substep_secs_f64,
            );
        }
        delta_position += linear_velocity * substep_secs;
        delta_rotation.0 =
            Quat::from_scaled_axis(angular_velocity * substep_secs) * delta_rotation.0;
    }
    let old_world_com = rotation * body.local_center_of_mass;
    let rotation = (delta_rotation * rotation).fast_renormalize();
    let new_world_com = rotation * body.local_center_of_mass;
    body.position += delta_position + old_world_com - new_world_com;
    body.rotation = rotation.0;
    body.linear_velocity = linear_velocity;
    body.angular_velocity = angular_velocity;
    body.center_of_mass = rotation.mul_vec3(body.local_center_of_mass) + body.position;
}

/// Advances the player ship's [`FlightPredictionRun`] by up to
/// [`PREDICTED_TICKS_PER_FIXED_TICK`] ticks and publishes it as a
/// [`FlightPrediction`] when it ends; seeds a run from the ship's state when
/// none is in progress for the engaged leg or the last one is published. The
/// published path stays until the next run for the same leg replaces it; a
/// different leg removes it at the reseed. A non-finite run keeps a failed
/// empty-points seed and removes the published path. Removes both when the
/// ship leaves the scope [`FlightPrediction`] documents.
///
/// Runs in `FixedPostUpdate` after avian's writeback, so the seed is the state
/// the next live tick starts from: this tick's pose, velocities, actuator
/// writes, RCS magazine, telemetry and dominant well. Also runs in
/// `PostUpdate` while `Time<Virtual>` is paused, so a leg ordered on the paused
/// map is predicted from the frozen state, one run step per frame. A published
/// run seeded on the frozen tick is not flown again until the fixed clock
/// advances: the paused frames would only repeat the same path or failure.
pub(super) fn predict_flight_path(
    mut commands: Commands,
    (time, substep_time, substep_count, avian_gravity): (
        Res<Time<Fixed>>,
        Res<Time<Substeps>>,
        Res<SubstepCount>,
        Res<Gravity>,
    ),
    (settings, gravity_settings): (Res<FlightSettings>, Res<GravitySettings>),
    mut q_ship: Query<
        (
            Entity,
            Option<&Autopilot>,
            (&Position, &Rotation),
            (&LinearVelocity, &AngularVelocity),
            (
                &ComputedMass,
                &ComputedAngularInertia,
                Option<&ComputedCenterOfMass>,
            ),
            (
                Option<&ManeuverTelemetry>,
                Option<&FlightArrivalStandoff>,
                Option<&HullRadius>,
            ),
            (&RcsBudget, Option<&RcsIntent>),
            (
                Option<&DominantWell>,
                Has<GravityAffected>,
                Has<GravityWell>,
                Has<DockedShip>,
            ),
            (Option<&mut FlightPredictionRun>, Option<&FlightPrediction>),
        ),
        (With<PlayerSpaceshipMarker>, With<SpaceshipRootMarker>),
    >,
    (q_thruster, q_rotation_input, q_computer, q_capabilities): (
        HelmThrusterQuery,
        HelmControllerQuery,
        LiveFlightComputers,
        ShipCapabilityQuery,
    ),
    (q_target, q_wells, q_gravity_wells, q_motion): (
        ArrivalTargetQuery,
        FlightWellQuery,
        Query<(Entity, &Position, &GravityWell)>,
        Query<(Option<&LinearVelocity>, Option<&AngularVelocity>)>,
    ),
    // Ships whose non-finite prediction is already logged, so the error is
    // said once per leg, not at the fixed rate; then the scratch buffers of
    // the well pull.
    (mut nonfinite, mut candidates, mut pulls): (
        Local<EntityHashSet>,
        Local<Vec<(Entity, f32, Vec3)>>,
        Local<Vec<(Entity, f32)>>,
    ),
) {
    let dt = time.delta_secs();
    let substeps = (substep_count.0, substep_time.delta());
    let moving = |entity: Entity| {
        q_motion.get(entity).is_ok_and(|(linear, angular)| {
            linear.is_some_and(|v| v.0 != Vec3::ZERO) || angular.is_some_and(|w| w.0 != Vec3::ZERO)
        })
    };

    for (
        ship,
        autopilot,
        pose,
        velocities,
        (mass, angular_inertia, com),
        (telemetry, standoff_override, hull_radius),
        (rcs_budget, rcs_intent),
        (dominant_well, gravity_affected, is_well, docked),
        (run, published),
    ) in &mut q_ship
    {
        let hide = |commands: &mut Commands| {
            if run.is_some() {
                commands.entity(ship).remove::<FlightPredictionRun>();
            }
            if published.is_some() {
                commands.entity(ship).remove::<FlightPrediction>();
            }
        };
        let autopilot = autopilot.filter(|autopilot| {
            matches!(
                autopilot.action,
                AutopilotAction::Goto { .. }
                    | AutopilotAction::GotoPos { .. }
                    | AutopilotAction::Stop
            )
        });
        let Some(autopilot) = autopilot.filter(|_| !docked && dt > 0.0 && mass.value() > 0.0)
        else {
            nonfinite.remove(&ship);
            hide(&mut commands);
            continue;
        };
        let wells = well_samples(&q_wells, &q_target);
        let target = arrival_target(
            autopilot.action,
            &wells,
            &q_target,
            &gravity_settings,
            &settings,
        );
        let target_moves = match autopilot.action {
            AutopilotAction::Goto { target } => moving(target),
            _ => false,
        };
        let gravity_wells: Vec<(Entity, Vec3, GravityWell)> = if gravity_affected && !is_well {
            q_gravity_wells
                .iter()
                .map(|(well, position, data)| (well, position.0, data.clone()))
                .collect()
        } else {
            Vec::new()
        };
        let wells_move = gravity_wells.iter().any(|&(well, ..)| moving(well))
            || wells.iter().any(|sample| moving(sample.entity));
        let helm = flight_helm(
            ship,
            rcs_intent,
            rcs_budget,
            &q_thruster,
            &q_rotation_input,
            &q_computer,
        );
        let flies = helm.controllers.iter().any(|c| c.live_pd.is_some())
            && helm
                .engines
                .iter()
                .any(|engine| engine_direction(pose.1, &engine.mount).is_some());
        let goal_gone =
            matches!(autopilot.action, AutopilotAction::Goto { .. }) && target.is_none();
        if !flies || goal_gone || target_moves || wells_move {
            nonfinite.remove(&ship);
            hide(&mut commands);
            continue;
        }

        let mut seeded = None;
        let run = match run {
            Some(run) if run.action == autopilot.action && !run.points.is_empty() => {
                run.into_inner()
            }
            stale => {
                let same_leg = stale
                    .as_ref()
                    .is_some_and(|run| run.action == autopilot.action);
                if same_leg
                    && (stale.as_ref().is_some_and(|run| {
                        run.points.is_empty() && run.seed_time == time.elapsed()
                    }) || published.is_some_and(|path| path.seed_time == time.elapsed()))
                {
                    continue;
                }
                // A different leg makes the published path a lie at once.
                if !same_leg && published.is_some() {
                    commands.entity(ship).remove::<FlightPrediction>();
                }
                let body = flight_body(
                    pose,
                    velocities,
                    (mass, angular_inertia),
                    com,
                    None,
                    hull_radius,
                );
                let seed = FlightPredictionRun {
                    action: autopilot.action,
                    autopilot: *autopilot,
                    points: vec![body.center_of_mass],
                    body,
                    helm,
                    mass: *mass,
                    target,
                    telemetry: telemetry.copied(),
                    wells,
                    gravity_wells,
                    dominant_well: dominant_well.map(|well| **well),
                    arrival_standoff: resolved_arrival_standoff(standoff_override, &settings),
                    capabilities: ship_capabilities(ship, &q_capabilities),
                    ticks: 0,
                    braking_tick: None,
                    seed_time: time.elapsed(),
                };
                match stale {
                    Some(stale) => {
                        let stale = stale.into_inner();
                        *stale = seed;
                        stale
                    }
                    None => seeded.insert(seed),
                }
            }
        };

        let horizon_ticks = (PREDICTION_HORIZON / dt).round() as u32;
        let mut end = None;
        let mut finite = true;
        for _ in 0..PREDICTED_TICKS_PER_FIXED_TICK {
            let env = FlightEnvironment {
                dt,
                settings: &settings,
                gravity: &gravity_settings,
                wells: &run.wells,
                arrival_standoff: run.arrival_standoff,
                capabilities: run.capabilities,
            };
            let step = autopilot_step(
                &mut run.autopilot,
                &run.body,
                &mut run.helm,
                run.target.as_ref(),
                run.telemetry.as_ref(),
                &env,
            );
            if let Some(ended) = step.end {
                end = Some(match ended {
                    AutopilotEndType::Completed => FlightPredictionEndType::Completed,
                    AutopilotEndType::ParkedIntoOrbit => FlightPredictionEndType::ParkedIntoOrbit,
                    AutopilotEndType::Disengaged(_) => FlightPredictionEndType::Disengaged,
                });
                break;
            }
            run.telemetry = step.telemetry;
            if run.braking_tick.is_none() && run.telemetry.is_some_and(|numbers| numbers.braking) {
                run.braking_tick = Some(run.ticks);
            }
            let pull = dominant_well_acceleration(
                run.body.position,
                run.dominant_well,
                &run.gravity_wells,
                &gravity_settings,
                &mut candidates,
                &mut pulls,
            );
            run.dominant_well = pull.map(|(well, _)| well);
            step_flight_body(
                &mut run.body,
                &mut run.helm,
                &run.mass,
                pull.map_or(Vec3::ZERO, |(_, acceleration)| acceleration),
                &env,
                substeps,
                avian_gravity.0,
            );
            run.ticks += 1;
            finite = run.body.position.is_finite()
                && run.body.rotation.is_finite()
                && run.body.linear_velocity.is_finite()
                && run.body.angular_velocity.is_finite();
            if !finite {
                break;
            }
            if run.ticks % SAMPLE_TICKS == 0 {
                run.points.push(run.body.center_of_mass);
            }
            if run.ticks >= horizon_ticks {
                end = Some(FlightPredictionEndType::Horizon);
                break;
            }
        }

        if !finite {
            if nonfinite.insert(ship) {
                error!(
                    "predict_flight_path: the predicted flight of {ship:?} went non-finite \
                     {} ticks after its seed; hiding the prediction",
                    run.ticks
                );
            }
            run.points.clear();
            if let Some(seed) = seeded {
                commands.entity(ship).insert(seed);
            }
            commands.entity(ship).remove::<FlightPrediction>();
            continue;
        }
        let Some(end) = end else {
            if let Some(seed) = seeded {
                commands.entity(ship).insert(seed);
            }
            continue;
        };
        if run.ticks % SAMPLE_TICKS != 0 {
            run.points.push(run.body.center_of_mass);
        }
        let last = run.points.len() - 1;
        let prediction = FlightPrediction {
            points: std::mem::take(&mut run.points),
            flip_index: run
                .braking_tick
                .map(|tick| (tick.div_ceil(SAMPLE_TICKS) as usize).min(last)),
            seed_time: run.seed_time,
            sample_interval: SAMPLE_TICKS as f32 * dt,
            final_point_time: run.ticks as f32 * dt,
            end,
        };
        commands.entity(ship).insert(prediction);
        if let Some(seed) = seeded {
            commands.entity(ship).insert(seed);
        }
    }
}
