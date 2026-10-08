//! The flight prediction: the path the player's autopilot will fly a GOTO,
//! GotoPos or STOP leg on, found by flying the leg ahead of the ship.
//!
//! The predictor does not model the autopilot. It runs the live chain itself,
//! one fixed tick at a time: [`autopilot_step`], then [`step_flight_body`],
//! which applies the RCS push, the PD torque, the dominant-well pull and the
//! thruster impulses the live systems apply, and integrates them the way avian
//! 0.7 does. A GOTO target flies with it as a [`BallisticTarget`]: as avian
//! moves and turns a body with no input. The predicted centre of mass
//! therefore matches the flown one for as long as the world the run froze at
//! its seed holds. Each live tick checks the target against the same coast and
//! drops a forecast it has left. A moving well, a collision, a docking or a
//! change to the hull is not predicted; a moving well hides the prediction
//! instead.
//!
//! Engine units throughout, as in the autopilot.

use std::time::Duration;

use avian3d::{dynamics::integrator::solve_gyroscopic_torque, math::MatExt, prelude::*};
use bevy::{ecs::entity::EntityHashSet, prelude::*};
use nova_gameplay::prelude::*;

use super::{
    autopilot::{
        arrival_target, autopilot_step, flight_body, flight_helm, well_samples, ArrivalAnchorType,
        ArrivalTarget, ArrivalTargetQuery, AutopilotEndType, FlightBody, FlightEnvironment,
        FlightHelm, FlightWellQuery, HelmControllerQuery, HelmThrusterQuery, WellSample,
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
/// per-pass bound, so the pass is 120 ticks. A tumbling GOTO target in a well
/// flown with the run raised the pass about 40%, to a 311 us max on one
/// pinned slower core of one host. That is not a frame-budget guarantee: a
/// loaded host measured passes over 1 ms with and without the target.
const PREDICTED_TICKS_PER_FIXED_TICK: u32 = 120;

/// Predicted ticks between two regular points of a prediction.
const SAMPLE_TICKS: u32 = 8;

/// How far the live GOTO target's `LinearVelocity` may leave the coast its
/// forecast flew it on, u/s (0.1 m/s): a burn, a collision or any force the
/// coast does not model past this makes the forecast a lie.
const TARGET_UNEXPLAINED_DELTA_V: f32 = 0.01;

/// How far the live GOTO goal may leave the one its forecast flew to, u: the
/// bound the flown-path proof holds a prediction to. Catches a moved target
/// and a changed centre of mass.
const TARGET_GOAL_DEVIATION: f32 = 1.0;

/// How far the live GOTO target's `AngularVelocity` may leave the tumble its
/// forecast flew it on, rad/s: a flight computer's turn or a collision past
/// this makes the forecast a lie.
const TARGET_UNEXPLAINED_ANGULAR_DELTA_V: f32 = 0.01;

/// The path the player's autopilot will fly its GOTO, GotoPos or STOP leg on.
///
/// Flown ahead through the same autopilot step, actuator chain and avian
/// integration as the live ship, from the ship's state at [`Self::seed_time`];
/// see the module docs for what the run cannot see. Present only while such a
/// leg is engaged on an undocked player ship with a live flight computer, a
/// live engine and a measured mass, and only while every well is at rest and
/// the live target keeps to the coast its run flew it on. Replaced by each
/// finished run.
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
/// Neither run reseeds for the same leg on the frozen fixed tick. A run whose
/// live target leaves its coast reseeds, and a published path whose own seed's
/// target does is removed.
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
    /// The leg's goal at the seed, then moved with the predicted target.
    target: Option<ArrivalTarget>,
    /// The GOTO target flown with the run; `None` for GotoPos, STOP and a
    /// target that avian does not move.
    target_motion: Option<TargetMotion>,
    /// The published path's target, flown from that path's seed at the live
    /// rate; `None` while no path with a flown target is published.
    shown_target: Option<ExpectedTarget>,
    /// The telemetry the last predicted tick published.
    telemetry: Option<ManeuverTelemetry>,
    /// The wells the autopilot reads, frozen at the seed.
    wells: Vec<WellSample>,
    /// The wells the gravity force system reads, frozen at the seed.
    gravity_wells: Vec<(Entity, Vec3, GravityWell)>,
    /// The gravity force system pulls the ship.
    ship_feels_wells: bool,
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

/// A GOTO target as avian moves it with no input: it coasts and tumbles. A
/// dynamic body feels avian's [`Gravity`] and, as [`GravityAffected`] without
/// a [`GravityWell`], the dominant-well pull at its origin, as in
/// `gravity_well_system`; a kinematic body keeps its velocities.
///
/// It assumes a body with no damping, `GravityScale`, `LockedAxes` or speed
/// cap, and no contact or joint. The live checks see a departure only after
/// it happens, so a shown path does not foresee a later burn, turn or
/// collision of its target.
/// `a_goto_to_a_coasting_target_is_predicted_along_the_target_flight` checks
/// the coast against flown paths.
#[derive(Clone, Copy)]
struct BallisticTarget {
    /// The avian `Position`, the body origin.
    position: Vec3,
    /// The avian `Rotation`.
    rotation: Quat,
    /// The avian `LinearVelocity`, the velocity of the centre of mass.
    velocity: Vec3,
    /// The avian `AngularVelocity`.
    angular_velocity: Vec3,
    /// The avian `ComputedAngularInertia`, for the gyroscopic motion.
    angular_inertia: ComputedAngularInertia,
    /// The avian `ComputedCenterOfMass`, which the body turns about.
    local_center_of_mass: Vec3,
    /// Forces act on the body: `RigidBody::Dynamic`.
    dynamic: bool,
    /// `gravity_well_system` pulls the body.
    feels_wells: bool,
    /// The well that owned the last pull.
    dominant_well: Option<Entity>,
}

impl BallisticTarget {
    /// One fixed tick: the pull at [`Self::position`] as
    /// `gravity_well_system` finds it, then avian's `substeps` (count, substep
    /// duration) as [`step_flight_body`] integrates them, with no torque.
    fn step(
        &mut self,
        wells: &[(Entity, Vec3, GravityWell)],
        gravity: &GravitySettings,
        substeps: (u32, Duration),
        avian_gravity: Vec3,
        candidates: &mut Vec<(Entity, f32, Vec3)>,
        pulls: &mut Vec<(Entity, f32)>,
    ) {
        let mut linear_increment = Vec3::ZERO;
        if self.feels_wells {
            let pull = dominant_well_acceleration(
                self.position,
                self.dominant_well,
                wells,
                gravity,
                candidates,
                pulls,
            );
            self.dominant_well = pull.map(|(well, _)| well);
            if let Some((_, acceleration)) = pull {
                linear_increment += acceleration;
            }
        }
        integrate_avian_substeps(
            (&mut self.position, &mut self.rotation),
            (&mut self.velocity, &mut self.angular_velocity),
            (linear_increment, Vec3::ZERO),
            (&self.angular_inertia, self.local_center_of_mass),
            self.dynamic,
            substeps,
            avian_gravity,
        );
    }

    /// The [`ArrivalTarget::goal`] `arrival_target` reads off this pose.
    fn goal(&self, anchor: ArrivalAnchorType) -> Vec3 {
        match anchor {
            ArrivalAnchorType::Origin => self.position,
            ArrivalAnchorType::CenterOfMass => {
                self.position + Rotation(self.rotation).mul_vec3(self.local_center_of_mass)
            }
        }
    }
}

/// A GOTO target flown from one seed at the live fixed rate: the coast the
/// live target is checked against.
#[derive(Clone, Copy)]
struct ExpectedTarget {
    /// Where the leg's [`ArrivalTarget::goal`] sits on the target.
    anchor: ArrivalAnchorType,
    /// The target at [`Self::time`].
    body: BallisticTarget,
    /// The `Time<Fixed>` elapsed time [`Self::body`] is flown to.
    time: Duration,
}

/// The GOTO target flown with a prediction run.
#[derive(Clone, Copy)]
struct TargetMotion {
    /// The target at the predicted tick.
    predicted: BallisticTarget,
    /// The target at the live tick, from the run's seed.
    expected: ExpectedTarget,
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

    integrate_avian_substeps(
        (&mut body.position, &mut body.rotation),
        (&mut linear_velocity, &mut angular_velocity),
        (linear_increment, angular_increment),
        (&body.angular_inertia, body.local_center_of_mass),
        true,
        (substep_count, substep),
        avian_gravity,
    );
    body.linear_velocity = linear_velocity;
    body.angular_velocity = angular_velocity;
    body.center_of_mass =
        Rotation(body.rotation).mul_vec3(body.local_center_of_mass) + body.position;
}

/// avian 0.7's `pre_process_velocity_increments` with no damping, gravity
/// scale or locked axes, `integrate_velocities` and `integrate_positions` for
/// each of `substeps` (count, substep duration), then
/// `writeback_solver_bodies`. The increments are accelerations; `avian_gravity`
/// is avian's global [`Gravity`]. A kinematic body (`dynamic` false) takes no
/// increment, gravity or gyroscopic motion and keeps its velocities.
fn integrate_avian_substeps(
    (position, rotation): (&mut Vec3, &mut Quat),
    (linear_velocity, angular_velocity): (&mut Vec3, &mut Vec3),
    (mut linear_increment, mut angular_increment): (Vec3, Vec3),
    (angular_inertia, local_center_of_mass): (&ComputedAngularInertia, Vec3),
    dynamic: bool,
    (substep_count, substep): (u32, Duration),
    avian_gravity: Vec3,
) {
    let substep_secs_f64 = substep.as_secs_f64() as f32;
    let substep_secs = substep.as_secs_f32();
    let damping = 1.0 / (1.0 + substep_secs_f64 * 0.0);
    linear_increment += avian_gravity * 1.0;
    linear_increment *= substep_secs_f64;
    angular_increment *= substep_secs_f64;
    let gyroscopic = !angular_inertia.inverse().is_isotropic(1e-6);
    let start = Rotation(*rotation);
    let mut delta_position = Vec3::ZERO;
    let mut delta_rotation = Rotation::IDENTITY;
    for _ in 0..substep_count {
        if dynamic {
            *linear_velocity *= damping;
            *angular_velocity *= damping;
            *linear_velocity += linear_increment;
            *angular_velocity += angular_increment;
            if gyroscopic {
                solve_gyroscopic_torque(
                    angular_velocity,
                    delta_rotation.0 * start.0,
                    angular_inertia,
                    substep_secs_f64,
                );
            }
        }
        delta_position += *linear_velocity * substep_secs;
        delta_rotation.0 =
            Quat::from_scaled_axis(*angular_velocity * substep_secs) * delta_rotation.0;
    }
    let old_world_com = start * local_center_of_mass;
    let end = (delta_rotation * start).fast_renormalize();
    let new_world_com = end * local_center_of_mass;
    *position += delta_position + old_world_com - new_world_com;
    *rotation = end.0;
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
/// Each pass first flies the GOTO target of the published path and of the run
/// to the live tick. A live target more than [`TARGET_UNEXPLAINED_DELTA_V`],
/// [`TARGET_UNEXPLAINED_ANGULAR_DELTA_V`] or [`TARGET_GOAL_DEVIATION`] off one
/// removes that path or reseeds that run.
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
        Query<(
            (Option<&Position>, Option<&Rotation>),
            (Option<&LinearVelocity>, Option<&AngularVelocity>),
            (
                Option<&ComputedAngularInertia>,
                Option<&ComputedCenterOfMass>,
            ),
            Option<&RigidBody>,
            Option<&DominantWell>,
            Has<GravityAffected>,
            Has<GravityWell>,
        )>,
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
        q_motion
            .get(entity)
            .is_ok_and(|(_, (linear, angular), ..)| {
                linear.is_some_and(|v| v.0 != Vec3::ZERO)
                    || angular.is_some_and(|w| w.0 != Vec3::ZERO)
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
        (mut run, published),
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
        // The GOTO target as avian moves it now.
        let target_body = match autopilot.action {
            AutopilotAction::Goto { target } => q_motion.get(target).ok().and_then(
                |(
                    (position, rotation),
                    (velocity, angular_velocity),
                    (angular_inertia, com),
                    body,
                    dominant,
                    affected,
                    is_well,
                )| {
                    let body = body.filter(|body| !body.is_static())?;
                    let dynamic = body.is_dynamic();
                    Some(BallisticTarget {
                        position: position?.0,
                        rotation: rotation?.0,
                        velocity: velocity?.0,
                        angular_velocity: angular_velocity?.0,
                        angular_inertia: *angular_inertia?,
                        local_center_of_mass: com?.0,
                        dynamic,
                        feels_wells: dynamic && affected && !is_well,
                        dominant_well: dominant.map(|well| **well),
                    })
                },
            ),
            _ => None,
        };
        let gravity_wells: Vec<(Entity, Vec3, GravityWell)> = q_gravity_wells
            .iter()
            .map(|(well, position, data)| (well, position.0, data.clone()))
            .collect();
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
        if !flies || goal_gone || wells_move {
            nonfinite.remove(&ship);
            hide(&mut commands);
            continue;
        }

        // A live target off the coast a forecast flew it on, each forecast
        // flown from its own seed: the published path's, then the run's.
        let mut run_departed = false;
        if let (Some(run), Some((live, goal))) = (run.as_mut(), target_body.zip(target.as_ref())) {
            if run.action == autopilot.action {
                let now = time.elapsed();
                // Only a fixed tick moves the targets: a paused frame leaves
                // the run, a failed seed included, unwritten.
                if run.shown_target.is_some_and(|shown| shown.time < now)
                    || run
                        .target_motion
                        .is_some_and(|motion| motion.expected.time < now)
                {
                    let run: &mut FlightPredictionRun = run;
                    let mut fly = |expected: &mut ExpectedTarget| {
                        while expected.time < now {
                            expected.body.step(
                                &run.gravity_wells,
                                &gravity_settings,
                                substeps,
                                avian_gravity.0,
                                &mut candidates,
                                &mut pulls,
                            );
                            expected.time += time.timestep();
                        }
                    };
                    if let Some(shown) = run.shown_target.as_mut() {
                        fly(shown);
                    }
                    if let Some(motion) = run.target_motion.as_mut() {
                        fly(&mut motion.expected);
                    }
                }
                let departed = |expected: &ExpectedTarget| {
                    live.velocity.distance(expected.body.velocity) > TARGET_UNEXPLAINED_DELTA_V
                        || live
                            .angular_velocity
                            .distance(expected.body.angular_velocity)
                            > TARGET_UNEXPLAINED_ANGULAR_DELTA_V
                        || goal.goal.distance(expected.body.goal(expected.anchor))
                            > TARGET_GOAL_DEVIATION
                };
                if run.shown_target.as_ref().is_some_and(departed) {
                    run.shown_target = None;
                    commands.entity(ship).remove::<FlightPrediction>();
                }
                run_departed = run
                    .target_motion
                    .as_ref()
                    .is_some_and(|motion| departed(&motion.expected));
            }
        }

        let mut seeded = None;
        let run = match run {
            Some(run)
                if run.action == autopilot.action && !run.points.is_empty() && !run_departed =>
            {
                run.into_inner()
            }
            stale => {
                let same_leg = stale
                    .as_ref()
                    .is_some_and(|run| run.action == autopilot.action);
                if same_leg
                    && !run_departed
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
                let target_motion =
                    target_body
                        .zip(target.as_ref())
                        .map(|(body, goal)| TargetMotion {
                            predicted: body,
                            expected: ExpectedTarget {
                                anchor: goal.anchor,
                                body,
                                time: time.elapsed(),
                            },
                        });
                // The published path stays for the same leg until this run
                // replaces it, so its target keeps being checked.
                let shown_target = stale
                    .as_ref()
                    .filter(|_| same_leg)
                    .and_then(|run| run.shown_target);
                let seed = FlightPredictionRun {
                    action: autopilot.action,
                    autopilot: *autopilot,
                    points: vec![body.center_of_mass],
                    body,
                    helm,
                    mass: *mass,
                    target,
                    target_motion,
                    shown_target,
                    telemetry: telemetry.copied(),
                    wells,
                    gravity_wells,
                    ship_feels_wells: gravity_affected && !is_well,
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
            if let (Some(motion), Some(goal)) = (&run.target_motion, run.target.as_mut()) {
                goal.goal = motion.predicted.goal(motion.expected.anchor);
            }
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
            let pull = if run.ship_feels_wells {
                dominant_well_acceleration(
                    run.body.position,
                    run.dominant_well,
                    &run.gravity_wells,
                    &gravity_settings,
                    &mut candidates,
                    &mut pulls,
                )
            } else {
                None
            };
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
            if let Some(motion) = run.target_motion.as_mut() {
                motion.predicted.step(
                    &run.gravity_wells,
                    &gravity_settings,
                    substeps,
                    avian_gravity.0,
                    &mut candidates,
                    &mut pulls,
                );
            }
            run.ticks += 1;
            finite = run.body.position.is_finite()
                && run.body.rotation.is_finite()
                && run.body.linear_velocity.is_finite()
                && run.body.angular_velocity.is_finite()
                && run.target_motion.as_ref().is_none_or(|motion| {
                    motion.predicted.position.is_finite()
                        && motion.predicted.rotation.is_finite()
                        && motion.predicted.velocity.is_finite()
                        && motion.predicted.angular_velocity.is_finite()
                });
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
            run.shown_target = None;
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
        run.shown_target = run.target_motion.map(|motion| motion.expected);
        commands.entity(ship).insert(prediction);
        if let Some(seed) = seeded {
            commands.entity(ship).insert(seed);
        }
    }
}
