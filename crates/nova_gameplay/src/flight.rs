//! Nova's flight layer: manual Newtonian piloting plus a diegetic autopilot
//! that flies the ship through its real actuators.
//!
//! Design: docs/spikes/20260709-103324-diegetic-autopilot.md (which supersedes
//! the velocity-servo model of the earlier flight-assist spike). There are no
//! invisible forces anywhere in this module: the autopilot swings the nose by
//! writing the same [`ControllerSectionRotationInput`] the player's mouse
//! command uses (the controller section's PD torque does the turning), and it
//! burns by writing the live thrusters' [`ThrusterSectionInput`] - so the
//! plume, the engine hum, and the actual impulse are the maneuver.
//!
//! - **Manual** (default): the mouse points the hull, W/Space/right-trigger is
//!   an analog main-drive burn, momentum persists. Pure Newtonian.
//! - **Autopilot** (engaged per action, [`Autopilot`] component present):
//!   - `X` - **STOP**: face retrograde, burn until the ship is at rest.
//!   - `G` - **GOTO** the current aim-assist lock: burn toward the target,
//!     flip at the arrival curve (`v_allowed = sqrt(2 * a * margin * d)`),
//!     decelerate, and come to rest at a standoff outside blast radius.
//!
//!   Both are one rule: compute the desired velocity for the goal, face the
//!   velocity *error*, and burn when aligned - the flip emerges naturally the
//!   moment the error points backward. While engaged, the ship stops
//!   listening to the mouse (the manual rotation copy is gated off), which
//!   makes the mouse camera-only free-look for free; any flight input
//!   disengages, and disengaging re-seeds the mouse rig from the ship's
//!   current facing so nothing lurches (see `camera_controller.rs`).
//!
//! Capability comes from the live sections: the main drive is the summed
//! magnitude of forward-aligned live thrusters, and the flight computer *is*
//! the controller section - no live controller, no autopilot (it disengages),
//! exactly as rotation authority already dies with it. Thruster inputs are
//! spooled (exponential ramp) so engines light up and cut instead of
//! snapping. Tunables live on the reflected [`FlightSettings`]; the math is
//! pure helpers, unit-tested, shared-shaped so the AI brain (input/ai.rs,
//! today a cruder version of the same idea) can adopt it later.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        Autopilot, AutopilotAction, AutopilotPhase, FlightIntent, FlightSettings, NovaFlightPlugin,
        NovaFlightSystems,
    };
}

/// A thruster counts as part of the main drive when its thrust direction
/// aligns with the ship's forward axis at least this much (cosine). Everything
/// less aligned is left alone - the flight layer only commands the engines
/// that actually push the ship forward.
const FORWARD_ALIGNMENT_COS: f32 = 0.9;

/// Projected-gradient iterations for the thrust balancer ([`balance_throttles`]).
/// The problem is a tiny convex QP (one equality, box bounds, a firing set of a
/// few engines) that converges in a handful of steps; this is a generous cap.
const BALANCE_ITERS: usize = 40;

/// Bisection iterations for the balancer's capacity projection
/// ([`project_onto_demand`]) - enough to pin the multiplier to f32 precision.
const BALANCE_PROJECT_ITERS: usize = 40;

/// The pilot's manual input, on the ship root. Written by the player input
/// layer; consumed by [`manual_burn_system`] when no autopilot is engaged.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
pub struct FlightIntent {
    /// Analog main-drive burn, `0..1` (W / Space / right trigger).
    pub burn: f32,
}

/// An engaged autopilot maneuver, on the ship root. Present = engaged; the
/// input layer inserts it (X = STOP, G = GOTO the lock) and removes it on any
/// flight input, so manual authority is simply "this component is absent".
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
pub struct Autopilot {
    /// What the computer is trying to do.
    pub action: AutopilotAction,
    /// Where the maneuver currently is (for the HUD); updated every tick by
    /// [`autopilot_system`].
    pub phase: AutopilotPhase,
}

impl Autopilot {
    /// A freshly engaged maneuver, starting in the align phase.
    pub fn engage(action: AutopilotAction) -> Self {
        Self {
            action,
            phase: AutopilotPhase::Align,
        }
    }
}

/// The autopilot's goal.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub enum AutopilotAction {
    /// Kill all velocity: flip retrograde and burn to rest.
    Stop,
    /// Fly to `target` and come to rest at [`FlightSettings::arrival_standoff`]
    /// from it. Replans toward the target's current position every tick, so a
    /// drifting target is tracked; there is no collision avoidance (spike).
    Goto {
        /// The destination entity (the aim-assist lock at engage time).
        target: Entity,
    },
}

/// Which part of the maneuver the ship is in, for the HUD readout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Reflect)]
pub enum AutopilotPhase {
    /// Swinging the nose toward the burn direction; engines cold.
    #[default]
    Align,
    /// Aligned and burning.
    Burn,
}

/// All flight tunables in one reflected resource, for the inspector and a
/// future settings menu. Authority (how hard the ship can burn or turn) is
/// *not* here - that comes from the ship's live sections.
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct FlightSettings {
    /// Spool rate toward a higher thruster input, 1/s (exponential). Engines
    /// light up at this rate.
    pub spool_up_rate: f32,
    /// Spool rate toward a lower thruster input, 1/s. Engines cut faster than
    /// they light.
    pub spool_down_rate: f32,
    /// Fraction of the ship's braking acceleration the autopilot plans with.
    /// Below 1.0 it brakes early, absorbing spool lag and PD settling instead
    /// of overshooting the goal.
    pub decel_margin: f32,
    /// GOTO arrives at rest this far from the target, world units. Kept
    /// outside the torpedo's 30u blast radius on purpose.
    pub arrival_standoff: f32,
    /// The autopilot only burns when the nose is at least this aligned with
    /// the burn direction (cosine) - same discipline as the AI.
    pub align_cos: f32,
    /// Below this speed relative to the goal the maneuver counts as done and
    /// the autopilot disengages.
    pub stop_speed_epsilon: f32,
    /// Minimum closing speed a GOTO keeps while still outside the standoff.
    /// The pure arrival curve goes to zero *at* the boundary, so without a
    /// floor the ship approaches it asymptotically and never crosses; with
    /// it, the ship enters at this gentle speed and the terminal retro burn
    /// kills the remainder.
    pub min_approach_speed: f32,
    /// How expensive rotating feels to the planner relative to burning:
    /// group score = `rotation_time * rotation_bias + burn_time`. Above 1.0
    /// the computer prefers the engine already pointing the right way (small
    /// trims use your retro/lateral thrusters); big burns still flip to the
    /// strongest drive.
    pub rotation_bias: f32,
    /// Trim on the derived hull turn rate, dimensionless. The rate itself
    /// comes from the ship's torque budget and live inertia
    /// ([`hull_turn_rate`]): the average rate of a torque-limited 180 is
    /// `sqrt(pi * max_torque / inertia) / 2`; 1.0 commands exactly that
    /// optimum, lower is more stately. This is what makes mass legible -
    /// a stripped hull turns visibly faster than a full build (torque-budget
    /// decision, task 20260709-095043).
    pub turn_rate_scale: f32,
    /// Floor on the derived turn rate, degrees/second, so a crippled or
    /// torque-starved hull still answers the helm.
    pub turn_rate_min_deg: f32,
    /// Ceiling on the derived turn rate, degrees/second, so a near-empty
    /// hull snaps hard but does not teleport.
    pub turn_rate_max_deg: f32,
    /// Extra seconds of un-braked travel the arrival plan budgets for engine
    /// spool-up on top of the brake group's rotation time.
    pub arrival_spool_pad: f32,
    /// Velocity errors at or below this are "crumbs": the computer stops
    /// re-aiming the hull for them (it finishes axially if the nose is
    /// already on the error, and otherwise accepts the residual). Without
    /// this deadband the ship pirouettes after ever-smaller leftovers,
    /// twitching toward a perfection nobody can see.
    pub attitude_deadband: f32,
    /// Once the engines are lit, keep burning until alignment falls this far
    /// below [`FlightSettings::align_cos`], so the plume does not flicker
    /// on/off right at the gate boundary.
    pub align_hysteresis: f32,
}

impl Default for FlightSettings {
    fn default() -> Self {
        Self {
            spool_up_rate: 6.0,
            spool_down_rate: 10.0,
            decel_margin: 0.85,
            arrival_standoff: 50.0,
            align_cos: 0.95,
            stop_speed_epsilon: 0.2,
            min_approach_speed: 1.5,
            rotation_bias: 1.5,
            // 0.9 of the bang-bang optimum: the PD tracks a slightly
            // conservative command instead of riding saturation the whole
            // flip. Ship-class feel then comes from max_torque vs inertia -
            // at torque 40 the asteroid_field flagship (I ~10.8) commands
            // ~88 deg/s while a bare remnant pins the 240 deg/s ceiling.
            // These are command rates; the PD tracks the ramp with ~0.5*w
            // rad of lag, so delivered flips run ~25-30% past the optimum.
            turn_rate_scale: 0.9,
            turn_rate_min_deg: 10.0,
            turn_rate_max_deg: 240.0,
            arrival_spool_pad: 0.5,
            // A 0.4 u/s drift is a slow creep nobody notices; chasing it
            // with attitude swings is what everybody notices.
            attitude_deadband: 0.4,
            align_hysteresis: 0.03,
        }
    }
}

/// System set for the flight layer; ordered before the section systems in
/// `FixedUpdate` so the thruster impulse system consumes the inputs written
/// this tick.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NovaFlightSystems;

/// Plugin wiring the flight layer.
#[derive(Default)]
pub struct NovaFlightPlugin;

impl Plugin for NovaFlightPlugin {
    fn build(&self, app: &mut App) {
        debug!("NovaFlightPlugin: build");

        app.init_resource::<FlightSettings>()
            // Register the whole reflected tree, not just the resource root.
            .register_type::<FlightSettings>()
            .register_type::<FlightIntent>()
            .register_type::<Autopilot>()
            .register_type::<AutopilotAction>()
            .register_type::<AutopilotPhase>();

        app.add_observer(insert_flight_control);
        app.add_observer(on_autopilot_removed_cool_engines);

        app.configure_sets(
            FixedUpdate,
            NovaFlightSystems.before(SpaceshipSectionSystems),
        );
        app.add_systems(
            FixedUpdate,
            (autopilot_system, manual_burn_system)
                .chain()
                .in_set(NovaFlightSystems),
        );
    }
}

/// Give the player's ship its manual flight input. Only intent-carrying ships
/// are driven by this layer; AI ships keep writing the raw seams directly.
fn insert_flight_control(add: On<Add, PlayerSpaceshipMarker>, mut commands: Commands) {
    let entity = add.entity;
    trace!("insert_flight_control: entity {:?}", entity);

    commands.entity(entity).insert(FlightIntent::default());
}

/// The fastest speed the ship may still carry `distance` short of its goal
/// and still be able to stop there, budgeting `lead_time` seconds of
/// un-braked travel for the flip: the `v` solving
/// `v * lead_time + v^2 / (2 * a * margin) = distance`. With zero lead this
/// is the classic `sqrt(2 * a * margin * d)` arrival rule. Zero at (or past)
/// the goal. Pure for unit testing.
fn arrival_speed_limit(distance: f32, accel: f32, margin: f32, lead_time: f32) -> f32 {
    let braking = accel.max(0.0) * margin.clamp(0.0, 1.0);
    let distance = distance.max(0.0);
    if braking <= 0.0 || distance <= 0.0 {
        return 0.0;
    }
    let lead = lead_time.max(0.0);
    braking * ((lead * lead + 2.0 * distance / braking).sqrt() - lead)
}

/// The velocity a GOTO wants right now: toward the target, at the arrival
/// rule's speed for the remaining distance (minus the standoff), floored at
/// `min_approach` so the ship actually crosses the boundary instead of
/// approaching it asymptotically. Zero once inside the standoff. Pure for
/// unit testing.
fn goto_desired_velocity(
    to_target: Vec3,
    standoff: f32,
    accel: f32,
    margin: f32,
    lead_time: f32,
    min_approach: f32,
) -> Vec3 {
    let distance = to_target.length();
    let remaining = distance - standoff;
    if remaining <= 0.0 || distance <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let speed = arrival_speed_limit(remaining, accel, margin, lead_time).max(min_approach.max(0.0));
    (to_target / distance) * speed
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
/// A torque-limited bang-bang 180 at angular acceleration
/// `alpha = max_torque / inertia` takes `2 * sqrt(pi / alpha)` seconds, an
/// average rate of `sqrt(pi * alpha) / 2`. The command slew and the
/// autopilot's rotation-time planning both run at this rate (trimmed by
/// [`FlightSettings::turn_rate_scale`], clamped by the min/max), so the same
/// stick input swings a stripped hull visibly faster than a fully built one.
/// `inertia` is the largest principal component - the conservative axis.
/// Pure for unit testing.
pub(crate) fn hull_turn_rate(max_torque: f32, inertia: f32, settings: &FlightSettings) -> f32 {
    let alpha = (max_torque / inertia.max(1e-6)).max(0.0);
    let optimum = (core::f32::consts::PI * alpha).sqrt() * 0.5;
    // Ordered defensively: both bounds are inspector-editable on the
    // reflected FlightSettings, and f32::clamp panics when min > max.
    let lo = settings.turn_rate_min_deg.to_radians();
    let hi = settings.turn_rate_max_deg.to_radians().max(lo);
    (optimum * settings.turn_rate_scale).clamp(lo, hi)
}

/// A cluster of live engines that push the ship in (roughly) the same world
/// direction, with their summed per-tick authority. The planner's unit of
/// choice: rotate whichever group is cheapest onto the needed burn.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ThrusterGroup {
    world_dir: Vec3,
    authority: f32,
}

/// Greedily cluster engines (`(world thrust dir, magnitude)`) into direction
/// groups: an engine joins the first group within `cone_cos` of its
/// direction, else seeds a new one. Group direction is the magnitude-weighted
/// mean. Pure for unit testing.
fn cluster_thrusters(engines: &[(Vec3, f32)], cone_cos: f32) -> Vec<ThrusterGroup> {
    let mut sums: Vec<(Vec3, f32)> = Vec::new();
    for &(dir, magnitude) in engines {
        if magnitude <= 0.0 {
            continue;
        }
        match sums
            .iter_mut()
            .find(|(sum, _)| sum.normalize_or_zero().dot(dir) >= cone_cos)
        {
            Some((sum, authority)) => {
                *sum += dir * magnitude;
                *authority += magnitude;
            }
            None => sums.push((dir * magnitude, magnitude)),
        }
    }
    sums.into_iter()
        .map(|(sum, authority)| ThrusterGroup {
            world_dir: sum.normalize_or_zero(),
            authority,
        })
        .collect()
}

/// Estimated seconds for `group` to deliver a `delta_v` burn along
/// `burn_dir`: time to rotate the group onto the burn (weighted by the
/// rotation bias - turning feels expensive) plus time to burn the impulse at
/// the group's authority. Pure for unit testing.
fn group_time_score(
    group: &ThrusterGroup,
    burn_dir: Vec3,
    delta_v: f32,
    mass: f32,
    dt: f32,
    turn_rate: f32,
    bias: f32,
) -> f32 {
    let rotation_time = group.world_dir.angle_between(burn_dir) / turn_rate.max(1e-3);
    let burn_time = if group.authority > 0.0 {
        delta_v * mass * dt / group.authority
    } else {
        f32::INFINITY
    };
    rotation_time * bias + burn_time
}

/// The fastest group for a burn, per [`group_time_score`].
fn choose_group<'a>(
    groups: &'a [ThrusterGroup],
    burn_dir: Vec3,
    delta_v: f32,
    mass: f32,
    dt: f32,
    turn_rate: f32,
    bias: f32,
) -> Option<&'a ThrusterGroup> {
    groups.iter().min_by(|a, b| {
        group_time_score(a, burn_dir, delta_v, mass, dt, turn_rate, bias).total_cmp(
            &group_time_score(b, burn_dir, delta_v, mass, dt, turn_rate, bias),
        )
    })
}

/// Main-drive input (`0..1`) to deliver `impulse` this tick given the drive's
/// per-tick authority. Pure for unit testing.
fn burn_input(impulse: f32, authority: f32) -> f32 {
    if authority <= 0.0 {
        return 0.0;
    }
    (impulse / authority).clamp(0.0, 1.0)
}

/// One engine's linear contribution to the thrust-balance problem, both per
/// unit of input (`0..1`): `forward` is the thrust it adds along the burn
/// direction (>= 0 for an engine inside the firing cone), `torque` is the
/// torque it exerts about the ship's center of mass
/// (`(engine_pos - com) x thrust`). Given in any single consistent frame -
/// the balance constraint (net torque = 0) is frame-invariant.
#[derive(Clone, Copy, Debug, PartialEq)]
struct BalanceEngine {
    forward: f32,
    torque: Vec3,
}

/// Differential throttle: per-engine inputs (each `0..1`) that deliver `demand`
/// units of thrust along the burn direction while routing the resultant force
/// as close to *through* the center of mass as the engine set allows.
///
/// Solves the tiny convex QP `min ||sum torque_i u_i||^2` subject to
/// `sum forward_i u_i = demand` and `0 <= u_i <= 1` by projected gradient: the
/// firing set is a few engines, so a handful of steps converge. The equality is
/// the maneuver's demand (deliver the thrust the pilot/autopilot asked for); the
/// objective nulls the net torque within whatever throttle headroom that
/// demand leaves. When there is no headroom - a lone off-center engine, or a
/// full-throttle demand - the demand wins and the residual torque is left for
/// the PD controller, exactly the pre-balance behavior. The uniform throttle
/// `demand / sum(forward)` is always a feasible starting point, so a balanced
/// (symmetric) drive returns it unchanged. Pure for unit testing.
fn balance_throttles(engines: &[BalanceEngine], demand: f32) -> Vec<f32> {
    let n = engines.len();
    if n == 0 {
        return Vec::new();
    }
    let total_forward: f32 = engines.iter().map(|e| e.forward.max(0.0)).sum();
    if total_forward <= 1e-6 {
        return vec![0.0; n];
    }
    // Clamp the demand into what the set can actually deliver, then seed at the
    // uniform throttle: sum forward_i * (demand/total) = demand, each in [0,1].
    let demand = demand.clamp(0.0, total_forward);
    let mut u = vec![demand / total_forward; n];

    // A single engine has no redistribution freedom - throttle scales its
    // magnitude, not its line of action - so the uniform seed (which for n = 1
    // is exactly demand/forward) is already the answer; skip the solve.
    if n == 1 {
        return u;
    }

    // Gradient of ||sum T_i u_i||^2 is 2 M^T M u; its Lipschitz constant is
    // 2 * lambda_max(M^T M) <= 2 * sum||T_i||^2. A conservative (larger) bound
    // just means smaller, still-convergent steps. No torque at all -> the
    // uniform seed is already optimal.
    let lipschitz = 2.0
        * engines
            .iter()
            .map(|e| e.torque.length_squared())
            .sum::<f32>();
    if lipschitz <= 1e-12 {
        return u;
    }
    let step = 1.0 / lipschitz;

    for _ in 0..BALANCE_ITERS {
        let net_torque: Vec3 = engines.iter().zip(&u).map(|(e, &ui)| e.torque * ui).sum();
        for (e, ui) in engines.iter().zip(u.iter_mut()) {
            *ui -= step * 2.0 * e.torque.dot(net_torque);
        }
        project_onto_demand(&mut u, engines, demand);
    }
    u
}

/// Euclidean projection of `u` onto `{ sum forward_i u_i = demand } ∩ [0,1]^n`,
/// the balancer's feasible set. The projection has the form
/// `u_i <- clamp(u_i + mu * forward_i, 0, 1)` for a single multiplier `mu`; the
/// mapped sum is monotone in `mu`, so a bisection pins it to the demand. Pure.
fn project_onto_demand(u: &mut [f32], engines: &[BalanceEngine], demand: f32) {
    let mapped_sum = |mu: f32| -> f32 {
        engines
            .iter()
            .zip(u.iter())
            .map(|(e, &ui)| {
                let w = e.forward.max(0.0);
                w * (ui + mu * w).clamp(0.0, 1.0)
            })
            .sum::<f32>()
    };
    // Bracket the multiplier: lo drives every clamp to 0 (sum 0 <= demand), hi
    // drives them to 1 (sum = total_forward >= demand). Expand until bracketed.
    let mut lo = -1.0f32;
    let mut hi = 1.0f32;
    while mapped_sum(lo) > demand && lo > -1e6 {
        lo *= 2.0;
    }
    while mapped_sum(hi) < demand && hi < 1e6 {
        hi *= 2.0;
    }
    for _ in 0..BALANCE_PROJECT_ITERS {
        let mid = 0.5 * (lo + hi);
        if mapped_sum(mid) < demand {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let mu = 0.5 * (lo + hi);
    for (e, ui) in engines.iter().zip(u.iter_mut()) {
        let w = e.forward.max(0.0);
        *ui = (*ui + mu * w).clamp(0.0, 1.0);
    }
}

/// Move a thruster input toward `target` on an exponential ramp -
/// framerate-independent, with distinct light-up and cut rates. Pure for unit
/// testing.
fn spool(current: f32, target: f32, up_rate: f32, down_rate: f32, dt: f32) -> f32 {
    let rate = if target > current { up_rate } else { down_rate };
    let alpha = 1.0 - (-rate * dt).exp();
    (current + (target - current) * alpha).clamp(0.0, 1.0)
}

/// Whether a thruster's world thrust direction counts as main drive for a
/// ship facing `forward`. Pure for unit testing.
pub(crate) fn is_forward_aligned(thrust_dir: Vec3, forward: Vec3) -> bool {
    thrust_dir.dot(forward) >= FORWARD_ALIGNMENT_COS
}

/// One line of HUD truth about the flight layer, shared with the HUD module
/// so the formatting is unit-testable. `goto_distance` is the current
/// distance to the GOTO target, when known.
pub(crate) fn flight_status_line(
    speed: f32,
    autopilot: Option<&Autopilot>,
    goto_distance: Option<f32>,
) -> String {
    let phase = |p: AutopilotPhase| match p {
        AutopilotPhase::Align => "ALIGN",
        AutopilotPhase::Burn => "BURN",
    };
    match autopilot {
        None => format!("MAN     {speed:5.1} u/s"),
        Some(ap) => match (ap.action, goto_distance) {
            (AutopilotAction::Stop, _) => {
                format!("AP STOP - {} | {speed:5.1} u/s", phase(ap.phase))
            }
            (AutopilotAction::Goto { .. }, Some(d)) => {
                format!("AP GOTO - {} | {speed:5.1} u/s | {d:5.0}m", phase(ap.phase))
            }
            (AutopilotAction::Goto { .. }, None) => {
                format!("AP GOTO - {} | {speed:5.1} u/s", phase(ap.phase))
            }
        },
    }
}

/// The autopilot. One rule flies every maneuver: compute the desired velocity
/// for the goal, rotate the *cheapest engine group* onto the velocity error
/// (rotation time * bias + burn time; the nose is nothing special), and fire
/// every engine currently inside the alignment cone. The flip-and-burn
/// emerges when the main drive is worth turning for; a retro or lateral
/// group handles what it already points at. Disengages (removes
/// [`Autopilot`]) when the goal is reached, the target is gone, the ship has
/// no engines, or the flight computer (live controller section) is lost.
/// Off-center engine torque is balanced at the source by differential throttle
/// ([`balance_throttles`], using each engine's lever arm about the live COM),
/// with the PD holding whatever residual the throttle headroom cannot null.
fn autopilot_system(
    time: Res<Time>,
    settings: Res<FlightSettings>,
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
        ),
        With<SpaceshipRootMarker>,
    >,
    // ALL live forward engines, including thrusters with manual per-section
    // bindings (the editor binds keys straight to thrusters): when the
    // computer takes the ship it commands every engine - an editor-built
    // ship would otherwise leave the autopilot with zero authority (it
    // rotated but could never burn, the 2026-07-09 playtest bug). Pressing a
    // bound thruster key is a flight input and disengages instead (see
    // input/player.rs).
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
        (&PDController, &ChildOf),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    mut q_rotation_input: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        With<ControllerSectionMarker>,
    >,
    q_target: Query<&GlobalTransform>,
) {
    let dt = time.delta_secs();

    for (ship, mut autopilot, position, rotation, velocity, mass, inertia, com) in &mut q_ship {
        // No flight computer, no autopilot - the ship is adrift on manual.
        // The strongest live computer's torque cap is the rotation authority
        // the turn-rate budget is derived from. (PD outputs stack additively
        // across computers, so max under-reports a multi-computer hull - a
        // deliberately conservative simplification.)
        let Some(computer_torque) = q_computer
            .iter()
            .filter(|(_, &ChildOf(parent))| parent == ship)
            .map(|(pd, _)| pd.max_torque)
            .reduce(f32::max)
        else {
            debug!("autopilot_system: ship {ship:?} lost its flight computer, disengaging");
            commands.entity(ship).remove::<Autopilot>();
            continue;
        };

        // Every live engine as (world thrust direction, magnitude), plus how
        // hot the hottest one runs (for the settle check). A section's local
        // Transform is its fixed attitude on the hull; engines do not gimbal.
        // Off-center engine torque is balanced below by differential throttle
        // (per-engine lever arms about the live COM); whatever the throttle
        // headroom cannot null - a lone off-center engine, or a full-throttle
        // demand with no spare thrust - the PD still holds within its cap.
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
        let (principal, _) = inertia.principal_angular_inertia_with_local_frame();
        let turn_rate = hull_turn_rate(computer_torque, principal.max_element(), &settings);

        // The goal, as a desired velocity right now. For GOTO the arrival
        // curve is planned with the group the computer would actually brake
        // with: its authority sets the deceleration, its rotation distance
        // sets the lead (a retro-equipped ship brakes late and flat; a
        // main-drive-only ship budgets its 180).
        let desired = match autopilot.action {
            AutopilotAction::Stop => Vec3::ZERO,
            AutopilotAction::Goto { target } => {
                let Ok(target_transform) = q_target.get(target) else {
                    debug!("autopilot_system: GOTO target {target:?} is gone, disengaging");
                    commands.entity(ship).remove::<Autopilot>();
                    continue;
                };
                let to_target = target_transform.translation() - position.0;
                if to_target.length() <= settings.arrival_standoff {
                    Vec3::ZERO
                } else {
                    let brake_dir = -to_target.normalize();
                    let brake_speed = velocity.length().max(settings.min_approach_speed);
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
                    goto_desired_velocity(
                        to_target,
                        settings.arrival_standoff,
                        accel,
                        settings.decel_margin,
                        lead,
                        settings.min_approach_speed,
                    )
                }
            }
        };

        let error = desired - **velocity;
        let error_speed = error.length();
        let error_dir = (error_speed > 1e-3).then(|| error / error_speed);

        // The firing set: every live engine currently inside the alignment
        // cone of the needed burn (lit engines keep a slightly looser gate -
        // hysteresis via their own spooled input - so the plume does not
        // flicker at the boundary). Each is collected with the coefficients the
        // balancer needs - forward thrust and lever-arm torque about the live
        // COM, both per unit input - so the shared burn demand can be split
        // into a torque-nulling differential throttle. The COM is body-local;
        // lift it to world with rotation + translation (never render scale).
        let com_world = com
            .map(|c| rotation.mul_vec3(c.0) + position.0)
            .unwrap_or(position.0);
        let mut firing_authority = 0.0f32;
        let mut firing: Vec<(Entity, BalanceEngine)> = Vec::new();
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
                if aligned >= gate {
                    firing_authority += **magnitude;
                    // World point of the engine (direct child of the root), the
                    // same point the impulse system pushes from, so its lever
                    // arm about com_world matches the torque physics applies.
                    let pos_world = position.0 + rotation.mul_vec3(transform.translation);
                    let torque = (pos_world - com_world).cross(dir * **magnitude);
                    firing.push((
                        thruster,
                        BalanceEngine {
                            forward: **magnitude * aligned.max(0.0),
                            torque,
                        },
                    ));
                }
            }
        }

        // Within the deadband the leftover is a crumb: never re-aim the hull
        // for it - any engine already on the error finishes it, and a
        // residual only a rotation could remove is accepted. This is what
        // stops the ship twitching after perfection.
        let fine = error_speed <= settings.attitude_deadband;

        // Done: the goal wants rest here and the ship is at rest - exactly,
        // or within the deadband with no engine on the residual. Release
        // only once the engines have wound down: a still-hot, spooling-down
        // drive would push the ship off again.
        let done = desired == Vec3::ZERO
            && (error_speed <= settings.stop_speed_epsilon || (fine && firing_authority <= 0.0));
        if done && hottest_input <= 0.05 {
            debug!("autopilot_system: ship {ship:?} maneuver complete, disengaging");
            commands.entity(ship).remove::<Autopilot>();
            continue;
        }

        // Rotate the cheapest group onto the error (only for corrections worth
        // turning for), then split the shared burn demand across the firing set
        // as a torque-nulling differential throttle. While settling (done,
        // engines still winding down) command zero to every engine.
        let mut throttles: Vec<f32> = vec![0.0; firing.len()];
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
                    // RELEASE_SPIN_EPSILON.
                    let urgency =
                        (error_speed / (settings.attitude_deadband * 8.0)).clamp(0.25, 1.0);
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
            // min(impulse, authority)). balance_throttles splits it to null the
            // net torque about the COM within that demand's headroom.
            let demand =
                firing_authority * burn_input(error_speed * mass.value(), firing_authority);
            let coeffs: Vec<BalanceEngine> = firing.iter().map(|(_, e)| *e).collect();
            throttles = balance_throttles(&coeffs, demand);
            burning = throttles.iter().any(|&u| u > 0.0);
        }

        autopilot.phase = if burning {
            AutopilotPhase::Burn
        } else {
            AutopilotPhase::Align
        };

        // Spool every engine: each firing engine toward its balanced throttle,
        // everything else (and everything while settling) toward zero.
        for (thruster, mut input, _, _, &ChildOf(parent)) in &mut q_thruster {
            if parent != ship {
                continue;
            }
            let target = firing
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

/// When the autopilot lets go - completion or any breakout - it cools the
/// engines it was driving and parks the helm on the hull's current attitude.
/// Nothing else writes a *bound* thruster's input between key events (the
/// manual burn system deliberately leaves bound thrusters to their own
/// keys), so a residual autopilot burn would otherwise ghost on forever; and
/// a rotation command abandoned mid-maneuver can sit ~180 degrees from the
/// hull, which parks the saturated PD in its degenerate zone where it
/// sustains a perpetual roll instead of damping the leftover spin.
fn on_autopilot_removed_cool_engines(
    remove: On<Remove, Autopilot>,
    q_ship: Query<&Rotation, With<SpaceshipRootMarker>>,
    mut q_thruster: Query<(&mut ThrusterSectionInput, &ChildOf), With<ThrusterSectionMarker>>,
    mut q_rotation_input: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        With<ControllerSectionMarker>,
    >,
) {
    for (mut input, &ChildOf(parent)) in &mut q_thruster {
        if parent == remove.entity {
            **input = 0.0;
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

/// Manual main-drive burn for intent-carrying ships with no autopilot
/// engaged: split the analog burn across the live forward thrusters as a
/// torque-nulling differential throttle, so an off-center or damage-shifted
/// drive still pushes the resultant force through the COM - within the throttle
/// headroom the burn leaves. A full-stick burn on an asymmetric hull has no
/// headroom and still pulls, held only by the PD as before; easing off the
/// stick frees the drive to fly straight.
fn manual_burn_system(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    q_ship: Query<
        (Entity, &FlightIntent, Option<&ComputedCenterOfMass>),
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

    for (ship, intent, com) in &q_ship {
        let burn = intent.burn.clamp(0.0, 1.0);

        // The main-drive set (engines facing the hull's forward -Z; retro and
        // laterals keep their own keys), each with its balance coefficients in
        // the ship-local frame. The balance constraint (net torque = 0) is
        // frame-invariant, and ComputedCenterOfMass is already body-local, so
        // no world lift is needed - lever arms are taken straight from the
        // section transforms about the local COM.
        let com_local = com.map(|c| c.0).unwrap_or(Vec3::ZERO);
        let mut firing: Vec<(Entity, BalanceEngine)> = Vec::new();
        for (thruster, _, magnitude, transform, &ChildOf(parent)) in &q_thruster {
            if parent != ship {
                continue;
            }
            let local_dir = transform.rotation.mul_vec3(Vec3::NEG_Z).normalize();
            if !is_forward_aligned(local_dir, Vec3::NEG_Z) {
                continue;
            }
            let torque = (transform.translation - com_local).cross(local_dir * **magnitude);
            firing.push((
                thruster,
                BalanceEngine {
                    forward: **magnitude * local_dir.dot(Vec3::NEG_Z).max(0.0),
                    torque,
                },
            ));
        }

        // Deliver `burn` of the set's forward thrust, balanced. The uniform
        // throttle `burn` is a feasible split, so a centered drive spools
        // exactly as before; an off-center one is trimmed toward straight
        // flight while the burn leaves headroom to trim with.
        let demand: f32 = burn * firing.iter().map(|(_, e)| e.forward).sum::<f32>();
        let coeffs: Vec<BalanceEngine> = firing.iter().map(|(_, e)| *e).collect();
        let throttles = balance_throttles(&coeffs, demand);

        for (thruster, mut input, _, transform, &ChildOf(parent)) in &mut q_thruster {
            if parent != ship {
                continue;
            }
            let local_dir = transform.rotation.mul_vec3(Vec3::NEG_Z).normalize();
            if !is_forward_aligned(local_dir, Vec3::NEG_Z) {
                continue;
            }
            let target = firing
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- Pure helpers ----------------------------------------------------

    #[test]
    fn arrival_speed_limit_is_zero_at_the_goal_and_grows_with_distance() {
        assert_eq!(arrival_speed_limit(0.0, 20.0, 0.85, 0.0), 0.0);
        assert_eq!(arrival_speed_limit(-5.0, 20.0, 0.85, 0.0), 0.0);
        let near = arrival_speed_limit(10.0, 20.0, 0.85, 0.0);
        let far = arrival_speed_limit(100.0, 20.0, 0.85, 0.0);
        assert!(near > 0.0 && far > near, "limit must grow with distance");
        // The margin slows the plan down.
        assert!(
            arrival_speed_limit(100.0, 20.0, 0.5, 0.0) < arrival_speed_limit(100.0, 20.0, 1.0, 0.0)
        );
        // v = sqrt(2 a d) exactly at margin 1 with no flip lead.
        assert!(
            (arrival_speed_limit(100.0, 20.0, 1.0, 0.0) - (2.0f32 * 20.0 * 100.0).sqrt()).abs()
                < 1e-4
        );
        // A flip lead budgets un-braked travel, so the allowed speed drops...
        let with_lead = arrival_speed_limit(100.0, 20.0, 1.0, 1.5);
        assert!(with_lead < arrival_speed_limit(100.0, 20.0, 1.0, 0.0));
        // ...and satisfies v * lead + v^2 / (2a) = d.
        let stopping = with_lead * 1.5 + with_lead * with_lead / (2.0 * 20.0);
        assert!((stopping - 100.0).abs() < 1e-3, "got {stopping}");
    }

    #[test]
    fn goto_desired_velocity_points_at_the_target_and_rests_inside_standoff() {
        let desired =
            goto_desired_velocity(Vec3::new(0.0, 0.0, -300.0), 50.0, 20.0, 0.85, 1.5, 1.5);
        assert!(desired.z < 0.0 && desired.x == 0.0 && desired.y == 0.0);
        assert!((desired.length() - arrival_speed_limit(250.0, 20.0, 0.85, 1.5)).abs() < 1e-4);
        // Inside the standoff the goal is rest.
        assert_eq!(
            goto_desired_velocity(Vec3::new(0.0, 0.0, -40.0), 50.0, 20.0, 0.85, 1.5, 1.5),
            Vec3::ZERO
        );
        // Just outside the boundary the floor keeps a closing speed, so the
        // ship crosses instead of stalling on the asymptote.
        let creeping =
            goto_desired_velocity(Vec3::new(0.0, 0.0, -50.01), 50.0, 20.0, 0.85, 1.5, 1.5);
        assert!((creeping.length() - 1.5).abs() < 1e-3);
        // Degenerate zero offset is safe.
        assert_eq!(
            goto_desired_velocity(Vec3::ZERO, 50.0, 20.0, 0.85, 1.5, 1.5),
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

    /// A lone off-center engine at full burn still pulls, and a centered drive
    /// stays held. This is the balancer's no-headroom floor (task
    /// 20260709-155920): differential throttle scales an engine's magnitude,
    /// not its line of action, so a single engine cannot null its own torque -
    /// and a full-throttle demand pins it at 1.0 with nothing to trim against.
    /// The PD holds within its cap (break-even lever arm ~ max_torque/64 per
    /// unit magnitude); past it the ship pulls. When there is more than one
    /// forward engine AND throttle headroom, the balancer holds the heading
    /// instead - see `balanced_partial_burn_holds_an_off_center_twin_drive`.
    #[test]
    fn off_center_burn_pulls_but_a_centered_drive_is_held() {
        let drift_after_burn = |thruster_x: f32| -> f32 {
            let mut app = flight_app();
            let (ship, thruster, controller) = spawn_ship(&mut app);
            app.world_mut()
                .get_mut::<PDController>(controller)
                .unwrap()
                .max_torque = 40.0; // the shipped torque budget
            app.world_mut()
                .get_mut::<Transform>(thruster)
                .unwrap()
                .translation
                .x = thruster_x;
            settle(&mut app);
            app.world_mut().get_mut::<FlightIntent>(ship).unwrap().burn = 1.0;
            for _ in 0..120 {
                app.update();
            }
            app.world()
                .get::<Rotation>(ship)
                .unwrap()
                .0
                .angle_between(Quat::IDENTITY)
        };

        let held = drift_after_burn(0.0);
        assert!(
            held < 0.15,
            "a centered main drive must stay held by the PD ({held} rad drift)"
        );
        let pulled = drift_after_burn(2.0);
        assert!(
            pulled > 0.4,
            "an engine 2 units off the centerline must out-torque the computer \
             ({pulled} rad drift)"
        );
    }

    /// Thrust balancing (task 20260709-155920): a drive that is off-center
    /// about the live COM pulls at full throttle (no spare thrust to trim with,
    /// held only by the PD) but a partial burn - which leaves the flight
    /// computer throttle headroom - is split into a differential throttle that
    /// nulls the net torque, so the ship tracks its heading like a centered
    /// drive. Two forward engines at unequal lever arms make the ship genuinely
    /// off-center; only the throttle headroom differs between the two cases.
    #[test]
    fn balanced_partial_burn_holds_an_off_center_twin_drive() {
        // Two forward (thrust -Z) engines at x = +4 and x = -1. The four unit
        // sections put the COM at x = (0 + 0 + 4 - 1)/4 = 0.75, so the lever
        // arms are 3.25 and 1.75 - a uniform throttle nets ~1.5 units of
        // torque, well past the PD's hold. With headroom the balancer runs the
        // near engine hotter so 3.25*near = 1.75*far and the net torque is 0.
        let drift_after_burn = |burn: f32| -> f32 {
            let mut app = flight_app();
            let ship = app
                .world_mut()
                .spawn((
                    RigidBody::Dynamic,
                    Transform::default(),
                    SpaceshipRootMarker,
                    FlightIntent::default(),
                ))
                .id();
            app.world_mut().spawn((
                ChildOf(ship),
                Name::new("hull"),
                Transform::from_xyz(0.0, 0.0, -1.0),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ));
            for x in [4.0f32, -1.0] {
                app.world_mut().spawn((
                    ChildOf(ship),
                    Name::new("thruster"),
                    ThrusterSectionMarker,
                    ThrusterSectionMagnitude(1.0),
                    ThrusterSectionInput(0.0),
                    Transform::from_xyz(x, 0.0, 1.0),
                    Collider::cuboid(1.0, 1.0, 1.0),
                    ColliderDensity(1.0),
                ));
            }
            app.world_mut().spawn((
                ChildOf(ship),
                Name::new("controller"),
                ControllerSectionMarker,
                ControllerSectionRotationInput::default(),
                PDController {
                    frequency: 4.0,
                    damping_ratio: 4.0,
                    max_torque: 40.0, // the shipped torque budget
                },
                PDControllerTarget(ship),
                Transform::from_xyz(0.0, 0.0, 0.0),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ));
            settle(&mut app);
            app.world_mut().get_mut::<FlightIntent>(ship).unwrap().burn = burn;
            for _ in 0..120 {
                app.update();
            }
            app.world()
                .get::<Rotation>(ship)
                .unwrap()
                .0
                .angle_between(Quat::IDENTITY)
        };

        // A 40% burn leaves ample headroom: the balancer nulls the torque and
        // the ship holds its heading within the centered-drive tolerance.
        let balanced = drift_after_burn(0.4);
        assert!(
            balanced < 0.15,
            "a partial burn must balance the off-center twin drive \
             ({balanced} rad drift)"
        );
        // A full-stick burn pins both engines at 1.0 - no headroom to trim -
        // so the same ship pulls, exactly the balancer's documented floor.
        let full = drift_after_burn(1.0);
        assert!(
            full > 0.4,
            "a full-throttle burn has no headroom to trim and still pulls \
             ({full} rad drift)"
        );
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
    fn cluster_thrusters_groups_by_direction() {
        let engines = [
            (Vec3::NEG_Z, 1.0),
            (Vec3::NEG_Z, 0.5), // joins the main group
            (Vec3::Z, 0.25),    // retro group
            (Vec3::X, 0.25),    // lateral group
        ];
        let groups = cluster_thrusters(&engines, 0.9);
        assert_eq!(groups.len(), 3);
        let main = groups
            .iter()
            .find(|g| g.world_dir.dot(Vec3::NEG_Z) > 0.99)
            .expect("main group");
        assert!((main.authority - 1.5).abs() < 1e-6);
        // Dead weight is ignored.
        assert_eq!(cluster_thrusters(&[(Vec3::X, 0.0)], 0.9).len(), 0);
    }

    #[test]
    fn group_choice_trades_rotation_against_burn_time() {
        let main = ThrusterGroup {
            world_dir: Vec3::NEG_Z,
            authority: 1.0,
        };
        let retro = ThrusterGroup {
            world_dir: Vec3::Z,
            authority: 0.25,
        };
        let groups = [main, retro];
        let (mass, dt, rate, bias) = (4.0, 1.0 / 64.0, 90.0f32.to_radians(), 1.5);
        // A small brake (+Z burn): the retro already points there and wins.
        let small = choose_group(&groups, Vec3::Z, 2.0, mass, dt, rate, bias).expect("group");
        assert_eq!(small.world_dir, Vec3::Z, "small trims use the retro");
        // A large brake: flipping the big main drive is faster overall.
        let large = choose_group(&groups, Vec3::Z, 60.0, mass, dt, rate, bias).expect("group");
        assert_eq!(large.world_dir, Vec3::NEG_Z, "big burns flip to the main");
        // Zero-authority groups never win over a usable one.
        let dead = ThrusterGroup {
            world_dir: Vec3::Z,
            authority: 0.0,
        };
        let handicapped = [main, dead];
        let pick = choose_group(&handicapped, Vec3::Z, 2.0, mass, dt, rate, bias).expect("group");
        assert_eq!(pick.world_dir, Vec3::NEG_Z);
    }

    #[test]
    fn burn_input_scales_and_saturates() {
        assert!((burn_input(0.5, 1.0) - 0.5).abs() < 1e-6);
        assert_eq!(burn_input(5.0, 1.0), 1.0);
        assert_eq!(burn_input(1.0, 0.0), 0.0);
        assert_eq!(burn_input(-1.0, 1.0), 0.0);
    }

    #[test]
    fn balance_throttles_splits_demand_to_null_torque() {
        // Two forward engines (weight 1 each) with opposing but unequal lever
        // arms: A torques +0.5 about the COM per unit, B torques -1.5. A
        // uniform half-throttle (0.5, 0.5) would net -0.5; the balancer instead
        // runs A hotter than B so 0.5*uA - 1.5*uB = 0 while uA + uB = 1, i.e.
        // uA = 0.75, uB = 0.25 (hand-computed).
        let engines = [
            BalanceEngine {
                forward: 1.0,
                torque: Vec3::new(0.0, 0.5, 0.0),
            },
            BalanceEngine {
                forward: 1.0,
                torque: Vec3::new(0.0, -1.5, 0.0),
            },
        ];
        let u = balance_throttles(&engines, 1.0);
        assert!((u[0] - 0.75).abs() < 1e-2, "uA = {}", u[0]);
        assert!((u[1] - 0.25).abs() < 1e-2, "uB = {}", u[1]);
        // The demand is delivered exactly and the net torque is nulled.
        let force: f32 = engines.iter().zip(&u).map(|(e, &x)| e.forward * x).sum();
        assert!((force - 1.0).abs() < 1e-2, "force = {force}");
        let torque: Vec3 = engines.iter().zip(&u).map(|(e, &x)| e.torque * x).sum();
        assert!(torque.length() < 1e-2, "net torque = {torque}");
    }

    #[test]
    fn balance_throttles_keeps_a_symmetric_drive_uniform() {
        // Mirror-image lever arms cancel at any uniform throttle, so the
        // balancer must return the shared throttle it started from unchanged
        // (demand 2.0 of total forward 4.0 -> 0.5 each).
        let engines = [
            BalanceEngine {
                forward: 2.0,
                torque: Vec3::new(0.0, 0.0, 1.0),
            },
            BalanceEngine {
                forward: 2.0,
                torque: Vec3::new(0.0, 0.0, -1.0),
            },
        ];
        let u = balance_throttles(&engines, 2.0);
        assert!(
            (u[0] - 0.5).abs() < 1e-3 && (u[1] - 0.5).abs() < 1e-3,
            "{u:?}"
        );
    }

    #[test]
    fn balance_throttles_falls_back_when_headroom_is_gone() {
        // A lone off-center engine cannot null its own torque - throttle scales
        // the magnitude, not the line of action - so it just delivers the
        // demand and leaves the torque to the PD.
        let lone = [BalanceEngine {
            forward: 1.0,
            torque: Vec3::new(0.0, 2.0, 0.0),
        }];
        assert!((balance_throttles(&lone, 0.5)[0] - 0.5).abs() < 1e-3);

        // A full-throttle demand pins every engine at 1.0: no headroom to trim,
        // so the residual torque is left to the PD (the pre-balance behavior).
        let full = [
            BalanceEngine {
                forward: 1.0,
                torque: Vec3::new(0.0, 0.5, 0.0),
            },
            BalanceEngine {
                forward: 1.0,
                torque: Vec3::new(0.0, -1.5, 0.0),
            },
        ];
        let u = balance_throttles(&full, 2.0);
        assert!(
            (u[0] - 1.0).abs() < 1e-3 && (u[1] - 1.0).abs() < 1e-3,
            "{u:?}"
        );

        // Degenerate inputs stay safe.
        assert!(balance_throttles(&[], 1.0).is_empty());
        assert_eq!(balance_throttles(&lone, 0.0)[0], 0.0);
    }

    #[test]
    fn spool_ramps_up_and_down_at_distinct_rates_and_clamps() {
        let up = spool(0.0, 1.0, 6.0, 10.0, 0.1);
        assert!(up > 0.0 && up < 1.0);
        let down = spool(1.0, 0.0, 6.0, 10.0, 0.1);
        assert!(1.0 - down > up, "cut should outpace light-up");
        let mut v = 0.0;
        for _ in 0..200 {
            v = spool(v, 1.0, 6.0, 10.0, 1.0 / 60.0);
        }
        assert!((v - 1.0).abs() < 1e-3);
        assert_eq!(spool(2.0, 1.5, 6.0, 10.0, 10.0), 1.0);
    }

    #[test]
    fn forward_alignment_selects_main_drive_thrusters() {
        let forward = Vec3::NEG_Z;
        assert!(is_forward_aligned(Vec3::NEG_Z, forward));
        assert!(!is_forward_aligned(Vec3::Z, forward));
        assert!(!is_forward_aligned(Vec3::X, forward));
    }

    #[test]
    fn flight_status_line_formats_manual_and_each_maneuver() {
        assert_eq!(flight_status_line(12.34, None, None), "MAN      12.3 u/s");
        let stop = Autopilot {
            action: AutopilotAction::Stop,
            phase: AutopilotPhase::Align,
        };
        assert_eq!(
            flight_status_line(12.34, Some(&stop), None),
            "AP STOP - ALIGN |  12.3 u/s"
        );
        let goto = Autopilot {
            action: AutopilotAction::Goto {
                target: Entity::PLACEHOLDER,
            },
            phase: AutopilotPhase::Burn,
        };
        assert_eq!(
            flight_status_line(5.0, Some(&goto), Some(320.4)),
            "AP GOTO - BURN |   5.0 u/s |   320m"
        );
    }

    #[test]
    fn player_marker_receives_flight_intent() {
        let mut app = App::new();
        app.add_observer(insert_flight_control);

        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        app.update();

        assert!(app.world().get::<FlightIntent>(ship).is_some());
        assert!(
            app.world().get::<Autopilot>(ship).is_none(),
            "no maneuver engaged at spawn"
        );
    }

    // --- Physics-level integration ---------------------------------------
    //
    // A real avian world with the real PD controller, controller-section glue,
    // and thruster impulse system, so these cover the whole diegetic pipeline:
    // autopilot -> rotation command -> PD torque -> hull swings -> aligned ->
    // spooled burn -> impulse -> velocity. No external forces anywhere.

    use crate::{
        integrity::test_support::{settle, unfinished_integrity_physics_app},
        sections::{
            controller_section::{
                sync_controller_section_forces, update_controller_section_rotation_input,
            },
            thruster_section::thruster_impulse_system,
        },
    };

    fn flight_app() -> App {
        let mut app = unfinished_integrity_physics_app();
        app.init_resource::<FlightSettings>();
        app.add_plugins(PDControllerPlugin);
        app.configure_sets(
            FixedUpdate,
            (
                NovaFlightSystems,
                PDControllerSystems::Sync,
                SpaceshipSectionSystems,
            )
                .chain(),
        );
        app.add_observer(on_autopilot_removed_cool_engines);
        app.add_systems(
            FixedUpdate,
            (
                autopilot_system,
                manual_burn_system,
                update_controller_section_rotation_input,
            )
                .chain()
                .in_set(NovaFlightSystems),
        );
        app.add_systems(
            FixedUpdate,
            (sync_controller_section_forces, thruster_impulse_system)
                .in_set(SpaceshipSectionSystems),
        );
        app.finish();
        app
    }

    /// A minimal flyable ship: hull collider, a rear main thruster (thrusting
    /// toward -Z, the ship's forward), and a live controller section with a
    /// real PD so the autopilot can actually swing the hull.
    fn spawn_ship(app: &mut App) -> (Entity, Entity, Entity) {
        let ship = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                SpaceshipRootMarker,
                FlightIntent::default(),
            ))
            .id();
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("hull"),
            Transform::from_xyz(0.0, 0.0, -1.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        let thruster = app
            .world_mut()
            .spawn((
                ChildOf(ship),
                Name::new("thruster"),
                ThrusterSectionMarker,
                ThrusterSectionMagnitude(1.0),
                ThrusterSectionInput(0.0),
                Transform::from_xyz(0.0, 0.0, 1.0),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ))
            .id();
        let controller = app
            .world_mut()
            .spawn((
                ChildOf(ship),
                Name::new("controller"),
                ControllerSectionMarker,
                ControllerSectionRotationInput::default(),
                PDController {
                    frequency: 4.0,
                    damping_ratio: 4.0,
                    // Deliberately over-torqued: the generic rig pins the
                    // derived turn rate at the clamp ceiling so maneuver
                    // tests exercise outcomes, not tuning. The shipped 40.0
                    // regime is covered by the scratch-scenario and off-axis
                    // tests.
                    max_torque: 100.0,
                },
                PDControllerTarget(ship),
                Transform::from_xyz(0.0, 0.0, 0.0),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ))
            .id();
        (ship, thruster, controller)
    }

    /// Mount an extra engine on the hull with a section-local attitude
    /// (thrust pushes along its local -Z). Kept on the ship's long axis so
    /// the tests stay torque-free unless they want torque.
    fn spawn_extra_thruster(
        app: &mut App,
        ship: Entity,
        magnitude: f32,
        local_rotation: Quat,
    ) -> Entity {
        app.world_mut()
            .spawn((
                ChildOf(ship),
                Name::new("extra thruster"),
                ThrusterSectionMarker,
                ThrusterSectionMagnitude(magnitude),
                ThrusterSectionInput(0.0),
                Transform::from_xyz(0.0, 0.0, -2.0).with_rotation(local_rotation),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ))
            .id()
    }

    fn forward_of(app: &App, ship: Entity) -> Vec3 {
        app.world()
            .get::<Rotation>(ship)
            .unwrap()
            .mul_vec3(Vec3::NEG_Z)
    }

    fn velocity_of(app: &App, ship: Entity) -> Vec3 {
        **app.world().get::<LinearVelocity>(ship).unwrap()
    }

    fn run(app: &mut App, frames: usize) {
        for _ in 0..frames {
            app.update();
        }
    }

    #[test]
    fn stop_flips_the_hull_and_kills_velocity_with_no_external_force() {
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        settle(&mut app);
        // Coasting sideways: the nose (-Z) must physically swing ~90 degrees
        // to point retrograde before the drive can brake anything.
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::new(6.0, 0.0, 0.0)));
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));

        run(&mut app, 900);

        let speed = velocity_of(&app, ship).length();
        assert!(speed < 0.5, "STOP should null the velocity, got {speed}");
        assert!(
            app.world().get::<Autopilot>(ship).is_none(),
            "a completed maneuver disengages"
        );
    }

    #[test]
    fn goto_arrives_at_standoff_and_disengages() {
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        let target = app
            .world_mut()
            .spawn((
                Transform::from_xyz(0.0, 0.0, -300.0),
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -300.0)),
            ))
            .id();
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Goto { target }));

        // Run until the autopilot releases the ship (the slewed command
        // makes the terminal creep slower), then assert AT that moment -
        // the accepted below-deadband crumb may slowly drift the parked
        // ship afterwards, which is the twitch-fix tradeoff, not a missed
        // arrival.
        let mut released_at = None;
        for tick in 0..4800 {
            app.update();
            if app.world().get::<Autopilot>(ship).is_none() {
                released_at = Some(tick);
                break;
            }
        }
        assert!(
            released_at.is_some(),
            "GOTO must complete and disengage within the budget"
        );

        let standoff = app.world().resource::<FlightSettings>().arrival_standoff;
        let pos = app.world().get::<Position>(ship).unwrap().0;
        let distance = (Vec3::new(0.0, 0.0, -300.0) - pos).length();
        let speed = velocity_of(&app, ship).length();
        assert!(
            distance <= standoff + 6.0 && distance >= standoff - 45.0,
            "should arrive near the {standoff}u standoff, got {distance}"
        );
        assert!(speed < 0.5, "should arrive at rest, got {speed}");
    }

    /// Reproduction attempt for the in-game report "autopilot rotates but
    /// never thrusts": build the ship EXACTLY like the scenario does
    /// (base_section + kind bundles, real config values from
    /// nova_assets/sections.rs) instead of the hand-rolled test sections, and
    /// diagnose the thruster-query conditions directly.
    #[test]
    fn scratch_scenario_built_ship_autopilot_thrusts() {
        let mut app = flight_app();
        let ship = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                Visibility::Visible,
                SpaceshipRootMarker,
                FlightIntent::default(),
            ))
            .id();
        let base = |id: &str| BaseSectionConfig {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            mass: 1.0,
            health: 100.0,
        };
        let controller = app
            .world_mut()
            .spawn((
                ChildOf(ship),
                base_section(base("controller")),
                controller_section(ControllerSectionConfig {
                    frequency: 4.0,
                    damping_ratio: 4.0,
                    max_torque: 40.0,
                    render_mesh: None,
                }),
                Transform::default(),
            ))
            .id();
        // The real game wires PDControllerTarget via the section observer;
        // mirror it manually like the other tests do.
        app.world_mut()
            .entity_mut(controller)
            .insert(PDControllerTarget(ship));
        let thruster = app
            .world_mut()
            .spawn((
                ChildOf(ship),
                base_section(base("thruster")),
                thruster_section(ThrusterSectionConfig {
                    magnitude: 1.0,
                    render_mesh: None,
                }),
                Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
            ))
            .id();
        settle(&mut app);

        // Diagnose the exact conditions the autopilot's thruster query needs.
        println!(
            "thruster: rotation={:?} binding={} inactive={} marker={} magnitude={:?} childof={:?} ship={ship:?}",
            app.world().get::<Rotation>(thruster),
            app.world()
                .get::<SpaceshipThrusterInputBinding>(thruster)
                .is_some(),
            app.world().get::<SectionInactiveMarker>(thruster).is_some(),
            app.world().get::<ThrusterSectionMarker>(thruster).is_some(),
            app.world()
                .get::<ThrusterSectionMagnitude>(thruster)
                .map(|m| **m),
            app.world().get::<ChildOf>(thruster),
        );

        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::new(6.0, 0.0, 0.0)));
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));
        run(&mut app, 60);
        println!(
            "after 60 ticks: thruster_input={:?} velocity={:?} ap={:?}",
            app.world()
                .get::<ThrusterSectionInput>(thruster)
                .map(|i| **i),
            app.world().get::<LinearVelocity>(ship).map(|v| **v),
            app.world().get::<Autopilot>(ship),
        );
        run(&mut app, 840);

        let speed = app
            .world()
            .get::<LinearVelocity>(ship)
            .map(|v| v.length())
            .unwrap_or(f32::NAN);
        assert!(
            speed < 0.5,
            "scenario-built ship STOP should reach rest, got {speed}"
        );
    }

    /// The high-speed flip regression: braking from a hard burn used to
    /// leave the PD limit-cycling (its torque clamp swamps the damping term
    /// on a 180 setpoint). With the slewed command the maneuver completes
    /// and the hull is parked - no residual tumble.
    #[test]
    fn high_speed_stop_settles_without_tumbling() {
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::new(0.0, 0.0, -60.0)));
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));

        run(&mut app, 3000);

        let speed = velocity_of(&app, ship).length();
        assert!(
            speed < 0.5,
            "high-speed STOP should reach rest, got {speed}"
        );
        assert!(app.world().get::<Autopilot>(ship).is_none());
        // Mid-maneuver the slewed command keeps the hull steady (the old
        // wobble hit 2+ rad/s DURING the burn); a residual endgame roll of
        // ~1.5 rad/s can survive release because the bcs PD cannot damp a
        // fast roll (known issue, filed as its own task). Guard against it
        // getting worse until that lands.
        run(&mut app, 300);
        let spin = app
            .world()
            .get::<AngularVelocity>(ship)
            .map(|w| w.length())
            .unwrap_or(f32::NAN);
        assert!(
            spin < 2.0,
            "post-release residual spin regressed: {spin} rad/s"
        );
    }

    /// Control case for the disabled-controller regression below: a LIVE
    /// controller damps an imposed spin, so the "disabled" test cannot pass
    /// vacuously. Spins about Y (a transverse axis); the ship is a symmetric
    /// top about its long z-axis, so this spin is torque-free-constant with no
    /// tumbling - any decay is the PD doing its job.
    #[test]
    fn a_live_controller_damps_an_imposed_spin() {
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        settle(&mut app);

        let spin = Vec3::new(0.0, 2.0, 0.0);
        app.world_mut()
            .entity_mut(ship)
            .insert(AngularVelocity(spin));
        run(&mut app, 120);

        let rate = app.world().get::<AngularVelocity>(ship).unwrap().length();
        assert!(
            rate < spin.length() * 0.5,
            "a live controller should damp the imposed spin: {} -> {rate} rad/s",
            spin.length()
        );
    }

    /// Regression for task 20260709-155922: a controller disabled in place
    /// (`SectionInactiveMarker`, as the integrity pipeline marks a zero-health
    /// non-leaf section) must stop torquing the hull. Before the fix,
    /// `sync_controller_section_forces` still applied the PD output toward the
    /// frozen command, so a dead computer kept stabilizing the ship. Now the
    /// imposed spin is left untouched - a spun ship keeps spinning.
    #[test]
    fn a_disabled_controller_leaves_the_spin_untouched() {
        let mut app = flight_app();
        let (ship, _, controller) = spawn_ship(&mut app);
        settle(&mut app);

        let spin = Vec3::new(0.0, 2.0, 0.0);
        app.world_mut()
            .entity_mut(ship)
            .insert(AngularVelocity(spin));
        app.world_mut()
            .entity_mut(controller)
            .insert(SectionInactiveMarker);
        run(&mut app, 120);

        // No live computer, no other torque source: the spin is conserved.
        let rate = app.world().get::<AngularVelocity>(ship).unwrap().0;
        assert!(
            (rate - spin).length() < 0.05,
            "a disabled controller must not damp the spin: {spin:?} -> {rate:?}"
        );
    }

    /// A retro-equipped ship must brake a small overspeed with the engine
    /// already pointing the right way - zero hull rotation, no flip.
    #[test]
    fn retro_group_brakes_a_small_overspeed_without_flipping() {
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        spawn_extra_thruster(
            &mut app,
            ship,
            0.25,
            Quat::from_rotation_y(std::f32::consts::PI),
        );
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::new(0.0, 0.0, -2.0)));
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));

        run(&mut app, 600);

        let speed = velocity_of(&app, ship).length();
        assert!(speed < 0.5, "retro should brake to rest, got {speed}");
        assert!(
            app.world().get::<Autopilot>(ship).is_none(),
            "completed maneuver disengages"
        );
        let forward = forward_of(&app, ship);
        assert!(
            forward.dot(Vec3::NEG_Z) > 0.95,
            "a retro brake must not flip the hull, forward now {forward}"
        );
    }

    /// For a big burn the math flips: rotating the strong main drive around
    /// beats a long slow burn on the little retro (the rotation-bias knob).
    #[test]
    fn large_burn_still_flips_to_the_main_drive() {
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        spawn_extra_thruster(
            &mut app,
            ship,
            0.25,
            Quat::from_rotation_y(std::f32::consts::PI),
        );
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::new(0.0, 0.0, -30.0)));
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));

        run(&mut app, 1800);

        let speed = velocity_of(&app, ship).length();
        assert!(speed < 0.5, "STOP should reach rest, got {speed}");
        let forward = forward_of(&app, ship);
        assert!(
            forward.dot(Vec3::NEG_Z) < 0.5,
            "a large brake should have swung the hull off the nose line, forward {forward}"
        );
    }

    /// Inside the deadband nothing rotates - but an engine that already
    /// points at the crumb kills it instead of the residual being accepted.
    #[test]
    fn side_thruster_kills_a_lateral_crumb_in_the_deadband() {
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        // Thrust toward -X (local -Z rotated +90 degrees about Y).
        spawn_extra_thruster(
            &mut app,
            ship,
            0.25,
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        );
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::new(0.3, 0.0, 0.0)));
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));

        run(&mut app, 300);

        let speed = velocity_of(&app, ship).length();
        assert!(
            speed < 0.25,
            "the side engine should kill the crumb, got {speed}"
        );
        assert!(app.world().get::<Autopilot>(ship).is_none());
        let forward = forward_of(&app, ship);
        assert!(
            forward.dot(Vec3::NEG_Z) > 0.95,
            "no rotation inside the deadband, forward now {forward}"
        );
    }

    /// Destroying the retro removes its group: the ship falls back to the
    /// flip-and-burn it would have needed anyway.
    #[test]
    fn a_dead_retro_falls_back_to_the_flip() {
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        let retro = spawn_extra_thruster(
            &mut app,
            ship,
            0.25,
            Quat::from_rotation_y(std::f32::consts::PI),
        );
        settle(&mut app);
        app.world_mut()
            .entity_mut(retro)
            .insert(SectionInactiveMarker);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::new(0.0, 0.0, -2.0)));
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));

        run(&mut app, 900);

        let speed = velocity_of(&app, ship).length();
        assert!(
            speed < 0.5,
            "main-drive fallback should still stop, got {speed}"
        );
        // The exact parking attitude wanders with the endgame crumbs; what
        // matters is that stopping required leaving the original facing (the
        // retro would have braked without turning at all).
        let forward = forward_of(&app, ship);
        assert!(
            forward.dot(Vec3::NEG_Z) < 0.5,
            "without the retro the hull must have turned away to brake, forward {forward}"
        );
    }

    /// The twitch fix: a residual drift below the attitude deadband, with the
    /// nose nowhere near the retro direction, is a crumb - the autopilot must
    /// accept it and let go instead of pirouetting the hull to chase it.
    #[test]
    fn stop_accepts_a_crumb_without_pirouetting() {
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        settle(&mut app);
        // Slow lateral creep, below the deadband; killing it would need a
        // ~90 degree pirouette.
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::new(0.3, 0.0, 0.0)));
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));

        run(&mut app, 120);

        assert!(
            app.world().get::<Autopilot>(ship).is_none(),
            "a crumb residual must be accepted, not chased"
        );
        let forward = app
            .world()
            .get::<Rotation>(ship)
            .unwrap()
            .mul_vec3(Vec3::NEG_Z);
        assert!(
            forward.dot(Vec3::NEG_Z) > 0.98,
            "the hull must not pirouette for a crumb, forward now {forward}"
        );
        let v = velocity_of(&app, ship);
        assert!(
            (v - Vec3::new(0.3, 0.0, 0.0)).length() < 0.05,
            "the crumb is accepted as-is, got {v}"
        );
    }

    /// The 2026-07-09 playtest bug: an editor-built ship binds keys straight
    /// to its thrusters (`SpaceshipThrusterInputBinding`), and the autopilot
    /// used to exclude bound thrusters from its authority - so it rotated but
    /// could never burn. The computer must command every live engine.
    #[test]
    fn autopilot_commands_editor_bound_thrusters() {
        let mut app = flight_app();
        let (ship, thruster, _) = spawn_ship(&mut app);
        app.world_mut()
            .entity_mut(thruster)
            .insert(SpaceshipThrusterInputBinding(vec![]));
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::new(6.0, 0.0, 0.0)));
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));

        run(&mut app, 900);

        let speed = velocity_of(&app, ship).length();
        assert!(
            speed < 0.5,
            "STOP must burn bound thrusters too, got {speed}"
        );
        // And the engines are cooled on release - a residual input on a
        // bound thruster would ghost-burn forever (nothing else writes it).
        let residual = app
            .world()
            .get::<ThrusterSectionInput>(thruster)
            .map(|i| i.0)
            .unwrap_or(f32::NAN);
        assert_eq!(residual, 0.0, "disengage must cool the engines");
    }

    #[test]
    fn goto_disengages_when_the_target_is_gone() {
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        let target = app
            .world_mut()
            .spawn((
                Transform::from_xyz(0.0, 0.0, -300.0),
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -300.0)),
            ))
            .id();
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Goto { target }));
        run(&mut app, 5);

        app.world_mut().entity_mut(target).despawn();
        run(&mut app, 2);

        assert!(
            app.world().get::<Autopilot>(ship).is_none(),
            "a vanished destination disengages the autopilot"
        );
    }

    #[test]
    fn a_dead_flight_computer_disengages_the_autopilot() {
        let mut app = flight_app();
        let (ship, _, controller) = spawn_ship(&mut app);
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::new(6.0, 0.0, 0.0)));
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));
        run(&mut app, 5);
        assert!(app.world().get::<Autopilot>(ship).is_some());

        // The controller section is knocked out: no computer, no autopilot.
        app.world_mut()
            .entity_mut(controller)
            .insert(SectionInactiveMarker);
        run(&mut app, 2);

        assert!(
            app.world().get::<Autopilot>(ship).is_none(),
            "losing the controller section must drop the autopilot"
        );
        // And the ship coasts from here - nothing else brakes it.
        let v0 = velocity_of(&app, ship);
        run(&mut app, 60);
        assert!((velocity_of(&app, ship) - v0).length() < 0.2);
    }

    #[test]
    fn manual_burn_accelerates_and_is_ignored_while_engaged() {
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        settle(&mut app);

        // Manual: analog burn accelerates along the nose.
        app.world_mut().get_mut::<FlightIntent>(ship).unwrap().burn = 1.0;
        run(&mut app, 120);
        let manual_speed = velocity_of(&app, ship).length();
        assert!(
            velocity_of(&app, ship).z < -1.0,
            "manual burn should accelerate"
        );

        // Engaged with the burn value still set (in the real game holding W
        // would disengage via the input observer; this pins that the manual
        // *system* never drives an engaged ship): the ship must stop
        // accelerating and start the maneuver instead of burning on.
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));
        run(&mut app, 120);
        let engaged_speed = velocity_of(&app, ship).length();
        // Allowance: the pre-engage burn spools down over ~0.4s and the hull
        // swings through partly-forward attitudes while the slewed command
        // ramps. Still burning at full manual throttle would have added
        // ~26 u/s over these ticks.
        assert!(
            engaged_speed < manual_speed + 3.0,
            "an engaged ship must not keep accelerating from stale manual burn \
             ({manual_speed} -> {engaged_speed})"
        );

        // Pilot lets go; STOP runs to completion and hands back a resting ship.
        app.world_mut().get_mut::<FlightIntent>(ship).unwrap().burn = 0.0;
        run(&mut app, 1200);
        let speed = velocity_of(&app, ship).length();
        assert!(speed < 0.5, "STOP should reach rest, got {speed}");
        assert!(app.world().get::<Autopilot>(ship).is_none());
    }
}
