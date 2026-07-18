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
        Autopilot, AutopilotAction, AutopilotPhase, BodyRadius, FlightIntent, FlightSettings,
        FlightSpeedCap, ManeuverTelemetry, NovaFlightPlugin, NovaFlightSystems, OrbitPlan,
        RcsActive, RcsIntent, RcsSpeedCap,
    };
}

/// The geometric radius of a scenario object, world units: the surface
/// the GOTO arrival standoff measures from and the orbit band's clearance
/// floor clears (tasks 20260710-202408 and the 2026-07-10 "stops too
/// close" playtest). Derived from the actual generated collider where one
/// exists (asteroids: the noise-displaced mesh's outermost vertex, which
/// can reach well past the nominal designation radius) rather than
/// authored by hand. Unsized targets fall back to zero (center-relative,
/// the pre-existing behavior, fine for ships and debris). Well bodies are
/// also covered by [`GravityWell::body_radius`](crate::gravity::GravityWell)
/// (the nominal physics radius); the arrival and the band take the larger
/// of the two when both exist.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct BodyRadius(pub f32);

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

/// Weight of the squared net off-axis force (perpendicular to the burn)
/// against the squared net torque in the balance objective
/// ([`balance_throttles`]). This is the "bounded lateral drift" trade decided
/// with the user (task 20260709-224518): recruiting an off-axis thruster for
/// counter-torque adds a sideways force the maneuver did not ask for, so the
/// allocator pays for it - softly, not with a hard zero constraint, because a
/// hard zero needs an opposing pair and gives no help to a ship whose damage
/// left it exactly one usable lateral. The weight has units of lever-arm
/// squared: an off-axis engine with lever arm `r` about the COM nulls all but
/// `w / (r^2 + w)` of the torque it is recruited against, so at 0.05 a
/// one-unit lever leaves ~5% residual (well inside the PD's hold) while an
/// engine mounted nearly through the COM - huge force for no torque - is
/// correctly not worth firing.
const LATERAL_PENALTY: f32 = 0.05;

/// The pilot's manual input, on the ship root. Written by the player input
/// layer; consumed by [`manual_burn_system`] when no autopilot is engaged.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
pub struct FlightIntent {
    /// Analog main-drive burn, `0..1` (W / Space / right trigger).
    pub burn: f32,
}

/// Soft cap (u/s) on the MANUAL main-drive burn, on the ship root:
/// scenario-authored for ships whose pilot should not be able to sail off
/// into the void (the shakedown starter ship; playtest 2026-07-12 finding
/// 1). [`manual_burn_system`] tapers the commanded burn to zero as the
/// velocity component along the burn direction approaches the cap - a
/// held W levels off instead of accelerating forever. Deliberately narrow:
/// only the manual burn reads it (the autopilot plans its own decel),
/// only the along-burn component counts (turning and retro-braking are
/// never blocked), and ships without the component keep unbounded
/// Newtonian burn.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct FlightSpeedCap(pub f32);

/// The pilot's (or autopilot's) RCS fine-adjustment command, on the ship root:
/// a desired translation direction in the ship's LOCAL frame, each component
/// roughly `-1..1` (the magnitude is how hard the nudge). Written by the player
/// input layer while RCS is held (task 20260718-122912) or by the autopilot
/// (task 20260718-122932); consumed by [`rcs_burn_system`]. Zero (or absent) =
/// no RCS. This is the shared primitive both drivers write, so RCS never grew
/// its own force path (spike 20260718-122508, fork 4).
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct RcsIntent(pub Vec3);

/// Present on the player ship while the pilot is HOLDING the RCS fine-adjust
/// modifier (SHIFT), inserted/removed by the input layer (task 20260718-122912).
/// It is the modal gate the rest of the flight/camera/input stack reads, exactly
/// as [`Autopilot`] presence gates manual rotation: while it is present the mouse
/// is repurposed from aiming to translation (`RcsIntent` accumulation), and both
/// the helm and the camera rig stop consuming the mouse so the heading and view
/// hold steady (spike 20260718-122508, Q4). Not written by the autopilot - the
/// autopilot drives `RcsIntent` directly (task 20260718-122932), no modal state.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct RcsActive;

/// Integrate one RCS virtual-joystick axis by `delta` and clamp to the unit
/// range the primitive expects (`RcsIntent` components are ~`[-1, 1]`). The
/// held-direction offset PERSISTS across frames: the pilot pushes to build it
/// and pulls back to zero it (spike 20260718-122508, Q1). Shared by the mouse
/// (XZ) and scroll (Y) input paths in the player-input layer.
pub(crate) fn accumulate_rcs_axis(current: f32, delta: f32) -> f32 {
    (current + delta).clamp(-1.0, 1.0)
}

/// Per-tick multiplier that fades the PLAYER's `RcsIntent` toward zero when no
/// fresh mouse/scroll motion arrives ([`decay_player_rcs_intent`]), so RCS is
/// delta-driven, not a persistent joystick. ~0.4 leaves a ~3-tick (~50 ms) tail
/// that smooths the per-frame input without feeling like a held stick. Feel-tune.
const RCS_PLAYER_INTENT_DECAY: f32 = 0.4;

/// Per-ship override of the RCS fine-adjust speed cap (u/s), on the ship root.
/// Unlike [`FlightSpeedCap`], RCS is ALWAYS capped - that is the whole point of
/// a fine-adjust mode - so a ship without this component still gets the default
/// [`FlightSettings::rcs_speed_cap`]; the component only lets a scenario tune
/// the ceiling per hull. [`rcs_burn_system`] gates each ship-local axis on the
/// along-axis velocity component just like the main-burn taper.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct RcsSpeedCap(pub f32);

/// Fraction of `rcs_accel` the local gravity accel must stay under for the
/// autopilot to hand ORBIT station-keeping to the RCS trim (task
/// 20260718-204640). At 0.5 the RCS push has 2x authority over the inward pull,
/// enough headroom to correct a perturbation; above it the main drive (full
/// authority) keeps the orbit. The menu planetoid's ~2.2 u/s^2 pull far exceeds
/// `rcs_accel * 0.5 = 0.75`, so its orbits stay on the main drive.
const RCS_ORBIT_GRAVITY_AUTHORITY: f32 = 0.5;

/// World-frame REFERENCE velocity the RCS cap is measured against, on the ship
/// root. [`rcs_burn_system`] caps the along-axis component of `velocity -
/// reference`, not of the absolute velocity - so RCS can trim a fast-moving
/// craft by a sub-cap delta relative to this reference. ABSENT or ZERO restores
/// the plain absolute cap (`reference = 0`), which is exactly the player
/// fine-adjust mode and the STOP/GOTO terminal settle - both leave this unset.
/// The autopilot writes it to the desired ORBITAL velocity while station-keeping
/// (task 20260718-151102), so a small prograde/retrograde correction trims the
/// orbit instead of gating to zero (the absolute cap would fight the ~2.5-6 u/s
/// orbital speed). Cleared to zero on autopilot disengage so a stale reference
/// never leaks into the player's absolute-cap mode.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct RcsReference(pub Vec3);

/// Fraction of the cap over which the manual burn tapers to zero (the
/// last stretch below the cap). Wide enough to feel like drag, not a wall.
const SPEED_CAP_TAPER_FRACTION: f32 = 0.2;

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
    /// Fly to a fixed world position and come to rest at
    /// [`FlightSettings::arrival_standoff`] from it - the same arrival rule
    /// as `Goto`, just without an entity to track. The AI patrol loop flies
    /// its waypoints with this (task 20260709-225730); the player input
    /// layer never engages it.
    GotoPos {
        /// The destination, world coordinates.
        position: Vec3,
    },
    /// Circularize and station-keep inside `well`'s gravity well (spike
    /// docs/spikes/20260709-193147-gravity-wells-orbital-mechanics.md).
    /// The first engaged tick is the Plan phase: [`autopilot_system`] picks
    /// the target ring (current radius clamped into the stable band) and
    /// the orbit plane (from r x v, ship-up fallback) and stores them here;
    /// the plan then stays sticky - a per-tick replan would chase its own
    /// drift. Never self-completes: the computer holds the orbit with
    /// micro-burns until breakout, Z, or a capability loss.
    Orbit {
        /// The well being orbited (the ship's dominant well at engage time).
        well: Entity,
        /// The sticky insertion plan; `None` until the first engaged tick.
        plan: Option<OrbitPlan>,
    },
}

/// The ORBIT verb's sticky plan, computed once when the maneuver engages.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct OrbitPlan {
    /// Target ring radius, world units - the current radius clamped into
    /// the stable band ([`orbit_target_radius`]).
    pub radius: f32,
    /// Orbit plane unit normal; travel direction on the ring is
    /// `normal x radial`.
    pub normal: Vec3,
}

/// Live numbers for an engaged translation leg (GOTO/GotoPos toward a
/// goal, STOP toward its predicted rest point), published on the ship by
/// [`autopilot_system`] every tick and removed when the leg ends (verb
/// switch clears it in-system; disengage clears it via
/// `remove_maneuver_telemetry`). This is the physics-side seam the HUD
/// instruments read (task 20260709-103454) - the arrival-rule internals
/// (brake authority, rotation lead) stay in the autopilot, the readouts
/// stay dumb.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct ManeuverTelemetry {
    /// The leg's destination, world coordinates (the tracked entity's
    /// current position for GOTO).
    pub goal: Vec3,
    /// The tracked destination entity for GOTO legs (`None` for GotoPos),
    /// so the HUD can anchor the readout to the same interpolated
    /// transform as the destination marker instead of the fixed-tick
    /// `goal` snapshot - a moving target would otherwise slide the
    /// caption off its marker.
    pub goal_entity: Option<Entity>,
    /// Where the leg comes to rest, world coordinates: `goal` pulled back
    /// along the closing line by the effective standoff
    /// ([`FlightSettings::arrival_standoff`] plus the resolved target
    /// radius). At or inside the park envelope it degenerates to the
    /// ship's own position - the computer will not fly back out, and the
    /// instruments must not draw a leg it will not fly. Equals `goal` for
    /// STOP (the predicted rest point IS the park point). The trajectory
    /// ribbon terminates here, not at the goal center (task
    /// 20260710-214316).
    pub park_point: Vec3,
    /// Distance to the goal SURFACE, world units: the center distance
    /// minus the target's resolved radius ([`BodyRadius`] /
    /// `GravityWell::body_radius`, zero for unsized targets and GotoPos),
    /// so the readout never says "50" while hovering over a mountain.
    /// Clamped at zero - at or inside the surface reads 0, never a
    /// negative number on the chip.
    pub distance: f32,
    /// Speed along the line to the goal, u/s (negative = opening).
    pub closing_speed: f32,
    /// The EFFECTIVE deceleration the arrival plan brakes with, u/s^2:
    /// margin applied, then reduced by the well pull toward the goal
    /// (spike docs/spikes/20260710-204802). Zero inside the standoff, and
    /// zero outside it when the pull meets or exceeds the brake authority
    /// (the degraded no-stopping-plan state; `flip_point` and `eta` are
    /// `None` there). No HUD instrument reads this yet.
    pub brake_accel: f32,
    /// Where on the path the flip-and-burn starts, world coordinates;
    /// `None` once braking has begun (or the estimate is meaningless at
    /// near-zero closing speed). Under heavy lateral drift the estimate
    /// runs optimistic: the flip math uses the along-track speed while
    /// the controller spends authority killing the lateral first.
    pub flip_point: Option<Vec3>,
    /// Coast time until the flip point, seconds.
    pub seconds_to_flip: Option<f32>,
    /// Rough time to arrival, seconds: coast to the flip plus the brake
    /// ramp. An estimate for the instruments, not a promise.
    pub eta: Option<f32>,
}

/// Disengaging ends the leg: drop its published numbers with it.
///
/// `try_remove`, not `remove`: this observer also fires while the ship is
/// being DESPAWNED (the scenario unload sweep, ship death), and the
/// `get_entity` guard only proves liveness at observer time - the queued
/// remove lands after the despawn completes in the same flush and warns
/// "Entity despawned" (2026-07-12 playtest, task 20260712-115902). The
/// fallible variant makes end-of-leg cleanup and teardown commute.
fn remove_maneuver_telemetry(remove: On<Remove, Autopilot>, mut commands: Commands) {
    if let Ok(mut ship) = commands.get_entity(remove.entity) {
        ship.try_remove::<ManeuverTelemetry>();
    }
}

/// Which part of the maneuver the ship is in, for the HUD readout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Reflect)]
pub enum AutopilotPhase {
    /// Swinging the nose toward the burn direction; engines cold.
    #[default]
    Align,
    /// Aligned and burning.
    Burn,
    /// ORBIT only: on the planned ring within tolerance, station-keeping
    /// with micro-burns against integrator drift and fade-band error.
    Hold,
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
    /// The crumb band for legs that END AT REST (STOP, GOTO, GotoPos; ORBIT
    /// keeps [`FlightSettings::attitude_deadband`]). A translation leg's
    /// endgame lives in sub-u/s errors - the brake tail, the boundary
    /// creep, the ~0.45-0.6 u/s doorstep residual - and with only the tight
    /// band the computer hunts them with visible attitude swings for
    /// seconds at every arrival
    /// (docs/spikes/20260711-140234-feel-filtering.md). This band must stay
    /// above the doorstep residual. ORBIT deliberately keeps the tight band
    /// (station-keeping's whole job is chasing small errors), which also
    /// preserves orbit_hold_enter's documented 2x relationship. Rest
    /// precision: an AXIAL residual (the shipped single-centered-drive
    /// ship) keeps the drive's aligned authority, so STOP still brakes to
    /// [`FlightSettings::stop_speed_epsilon`] exactly; a residual OFF the
    /// drive axis (a damage-shifted hull's recruit drift) is released at
    /// up to this band rather than hunted with attitude flips - that
    /// bounded creep is the contract, and the price of not wobbling.
    pub settle_deadband: f32,
    /// Once the engines are lit, keep burning until alignment falls this far
    /// below [`FlightSettings::align_cos`], so the plume does not flicker
    /// on/off right at the gate boundary.
    pub align_hysteresis: f32,
    /// ORBIT enters its Hold phase when the velocity error drops to this,
    /// u/s. Kept above [`FlightSettings::attitude_deadband`] so Hold still
    /// covers the micro-burn regime (drift is corrected, the label reads
    /// HOLD).
    pub orbit_hold_enter: f32,
    /// ORBIT leaves Hold (back to Align/Burn) only when the velocity error
    /// grows past this, u/s - hysteresis so the HUD phase does not flicker
    /// at the tolerance boundary.
    pub orbit_hold_exit: f32,
    /// The planned ring never sits closer to the body than
    /// `clearance * (body_radius + surface_margin)` - engaging ORBIT while
    /// skimming the surface plans an orbit with room to breathe.
    pub orbit_clearance_factor: f32,
    /// The planned ring never sits beyond `safety * fade_start` of the SOI:
    /// orbits are only trusted in the unfaded core (spike decision 3), and
    /// the safety margin keeps station-keeping off the fade band's edge.
    pub orbit_band_safety: f32,
    /// Default RCS fine-adjust speed cap (u/s): the terminal speed a held RCS
    /// nudge builds to on each ship-local axis before [`rcs_burn_system`]
    /// tapers the push to zero. Small by design - the last few meters of a
    /// docking approach. Overridable per hull with [`RcsSpeedCap`].
    pub rcs_speed_cap: f32,
    /// RCS thrust as an acceleration (u/s^2): how hard a full-deflection RCS
    /// command pushes. Sized so the cap is reached in a second or two of held
    /// input, not instantly - fine adjust, not a second main drive.
    pub rcs_accel: f32,
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
            // Above the measured doorstep residual (0.45-0.6 u/s on the
            // shipped rig): terminal spin drops from ~0.6 to under 0.1
            // rad/s, release spin from 0.44 to ~0.05, path tracking
            // unchanged (spike measurements at 0.6 and 0.75).
            settle_deadband: 0.75,
            align_hysteresis: 0.03,
            // Enter Hold at twice the attitude deadband: inside it the
            // computer is trimming crumbs, which is exactly what
            // station-keeping looks like.
            orbit_hold_enter: 0.8,
            orbit_hold_exit: 1.2,
            orbit_clearance_factor: 1.5,
            orbit_band_safety: 0.9,
            // A 2 u/s ceiling: brisk enough to close a docking gap, slow
            // enough that a held nudge never becomes free propulsion.
            rcs_speed_cap: 2.0,
            // ~1.5 u/s^2 reaches the 2 u/s cap in a bit over a second of held
            // input - a gentle station-keeping push, not a main burn.
            rcs_accel: 1.5,
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

        // The ORBIT verb reads the gravity tunables; init here too so the
        // flight layer stands alone (the AI physics tests build it without
        // NovaGravityPlugin). init_resource is idempotent, so the gravity
        // plugin owning the same resource is fine.
        app.init_resource::<GravitySettings>();

        app.init_resource::<FlightSettings>()
            // Register the whole reflected tree, not just the resource root.
            .register_type::<FlightSettings>()
            .register_type::<FlightIntent>()
            .register_type::<Autopilot>()
            .register_type::<AutopilotAction>()
            .register_type::<AutopilotPhase>()
            .register_type::<OrbitPlan>()
            .register_type::<ManeuverTelemetry>()
            .register_type::<BodyRadius>()
            .register_type::<FlightSpeedCap>()
            .register_type::<RcsIntent>()
            .register_type::<RcsSpeedCap>()
            .register_type::<RcsReference>()
            .register_type::<RcsActive>();

        app.add_observer(insert_flight_control);
        app.add_observer(on_autopilot_removed_cool_engines);
        app.add_observer(remove_maneuver_telemetry);

        app.configure_sets(
            FixedUpdate,
            NovaFlightSystems.before(SpaceshipSectionSystems),
        );
        app.add_systems(
            FixedUpdate,
            (
                autopilot_system,
                manual_burn_system,
                rcs_burn_system,
                decay_player_rcs_intent,
            )
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

    commands
        .entity(entity)
        .insert((FlightIntent::default(), RcsIntent::default()));
}

/// The fastest speed the ship may still carry `distance` short of its goal
/// and still be able to stop there, budgeting `lead_time` seconds of
/// un-braked travel for the flip and a well pull of `gravity_along` toward
/// the goal (spike docs/spikes/20260710-204802): the `v` solving
/// `v*lead + g*lead^2/2 + (v + g*lead)^2 / (2*(a*margin - g)) = distance`.
/// Gravity keeps accelerating through the lead window (`g*lead` of speed,
/// `g*lead^2/2` of distance) and fights the brake afterwards (effective
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
fn goto_desired_velocity(
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

/// Where the flip-and-burn starts for the current state of a GOTO leg.
/// Mirrors the arrival rule the autopilot actually flies
/// ([`arrival_speed_limit`]): with a well pull of `gravity_along` toward
/// the goal, the lead window covers `v*lead + g*lead^2/2` and the brake
/// ramp `(v + g*lead)^2 / (2*(brake_accel - g))`, so the flip sits that
/// far outside the standoff. Returns `(distance from the goal center,
/// coast seconds until the ship gets there)`, or `None` when braking has
/// already begun, the closing speed is too small for the coast estimate
/// to mean anything, or the pull eats the whole brake authority (no
/// stopping plan exists). The coast estimate uses the current closing
/// speed; gravity's coast-phase gain is absorbed by per-tick replanning.
/// Pure for unit testing.
pub(crate) fn goto_flip_point(
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
pub(crate) fn arrival_eta(
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
pub(crate) fn stop_rest_distance(
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
pub(crate) fn orbit_plane_normal(r_vec: Vec3, velocity: Vec3, ship_up: Vec3) -> Vec3 {
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

/// The ORBIT plan's target ring radius: the current radius, clamped into
/// the stable band of the well - no closer than
/// `orbit_clearance_factor * (body_radius + surface_margin)` (room to
/// breathe over the surface clamp), no farther than
/// `orbit_band_safety * fade_start` (orbits are only trusted in the unfaded
/// core, spike decision 3). `None` when the well has no stable band at all
/// (clearance past the trusted core) - such a well is unorbitable and the
/// verb refuses to plan. Pure for unit testing.
pub(crate) fn orbit_target_radius(
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
pub(crate) fn orbit_desired_velocity(
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

    tangential * crate::gravity::circular_orbit_speed(mu, plan.radius) + correction
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
pub(crate) fn orbit_ring_offset(r_vec: Vec3, plan: &OrbitPlan) -> Vec3 {
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
fn choose_group(
    groups: &[ThrusterGroup],
    burn_dir: Vec3,
    delta_v: f32,
    mass: f32,
    dt: f32,
    turn_rate: f32,
    bias: f32,
) -> Option<&ThrusterGroup> {
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

/// One engine's linear contribution to the thrust-balance problem, all per
/// unit of input (`0..1`). `primary` marks the firing set - the engines
/// inside the burn's alignment cone that the demand is budgeted against;
/// everything else is a counter-torque candidate the allocator may recruit.
/// `forward` is the thrust the engine adds along the burn direction: real for
/// primary engines (they carry the demand equality), and zero by convention
/// for recruits - a recruit is fired for its `torque` alone, so its ENTIRE
/// thrust vector goes into `lateral`, the force the maneuver did not ask for.
/// Billing the whole recruit force to the penalty (rather than crediting its
/// along-burn component to the equality) keeps a saturated demand feasible -
/// at full stick the equality has zero slack, and a recruit whose drift-tilted
/// thrust fights the burn by epsilon would otherwise be projected straight
/// back to zero - and makes a recruited retro honestly pay for the thrust it
/// cancels instead of the mains silently over-throttling. `torque` is
/// `(engine_pos - com) x thrust`. Given in any single consistent frame - the
/// balance objective (null torque, minimal off-axis force) is
/// frame-invariant.
#[derive(Clone, Copy, Debug, PartialEq)]
struct BalanceEngine {
    forward: f32,
    lateral: Vec3,
    torque: Vec3,
    primary: bool,
}

/// Wrench allocation: per-engine inputs (each `0..1`) that deliver `demand`
/// units of thrust along the burn direction while routing the resultant force
/// as close to *through* the center of mass as the whole live engine set
/// allows - recruiting off-axis engines (laterals, retros) purely for their
/// counter-torque when the firing set cannot balance itself.
///
/// Solves the tiny convex QP
/// `min ||sum torque_i u_i||^2 + LATERAL_PENALTY * ||sum lateral_i u_i||^2`
/// subject to `sum forward_i u_i = demand` and `0 <= u_i <= 1` by projected
/// gradient: a ship has a handful of engines, so a handful of steps converge.
/// The equality is the maneuver's demand (deliver the thrust the
/// pilot/autopilot asked for); the objective nulls the net torque while
/// keeping bounded the sideways force that nulling costs (see
/// [`LATERAL_PENALTY`]). The seed is the uniform throttle over the firing set
/// (`demand / sum(primary forward)`, off-axis engines at zero) - always
/// feasible, and a stationary point whenever the net torque is already null,
/// so a balanced (symmetric) drive returns unchanged and idle off-axis
/// engines are never lit gratuitously. When nothing can help - a lone
/// off-center engine with no off-axis thruster left - the demand wins and the
/// residual torque is left for the PD controller, exactly the pre-allocation
/// behavior. Pure for unit testing.
fn balance_throttles(engines: &[BalanceEngine], demand: f32) -> Vec<f32> {
    let n = engines.len();
    if n == 0 {
        return Vec::new();
    }
    let primary_forward: f32 = engines
        .iter()
        .filter(|e| e.primary)
        .map(|e| e.forward.max(0.0))
        .sum();
    if primary_forward <= 1e-6 {
        return vec![0.0; n];
    }
    // Clamp the demand into what the firing set can deliver, then seed at its
    // uniform throttle: sum_primary forward_i * (demand/total) = demand, each
    // in [0,1], every off-axis engine dark.
    let demand = demand.clamp(0.0, primary_forward);
    let mut u: Vec<f32> = engines
        .iter()
        .map(|e| {
            if e.primary {
                demand / primary_forward
            } else {
                0.0
            }
        })
        .collect();

    // A single engine has no redistribution freedom - throttle scales its
    // magnitude, not its line of action - so the uniform seed (which for n = 1
    // is exactly demand/forward) is already the answer; skip the solve.
    if n == 1 {
        return u;
    }

    // Gradient of ||T u||^2 + w ||L u||^2 is 2 (T^T T + w L^T L) u; its
    // Lipschitz constant is bounded by 2 * (sum||T_i||^2 + w sum||L_i||^2).
    // A conservative (larger) bound just means smaller, still-convergent
    // steps. No torque or lateral at all -> the uniform seed is already
    // optimal.
    let lipschitz = 2.0
        * engines
            .iter()
            .map(|e| e.torque.length_squared() + LATERAL_PENALTY * e.lateral.length_squared())
            .sum::<f32>();
    if lipschitz <= 1e-12 {
        return u;
    }
    let step = 1.0 / lipschitz;

    for _ in 0..BALANCE_ITERS {
        let net_torque: Vec3 = engines.iter().zip(&u).map(|(e, &ui)| e.torque * ui).sum();
        let net_lateral: Vec3 = engines.iter().zip(&u).map(|(e, &ui)| e.lateral * ui).sum();
        for (e, ui) in engines.iter().zip(u.iter_mut()) {
            *ui -= step
                * 2.0
                * (e.torque.dot(net_torque) + LATERAL_PENALTY * e.lateral.dot(net_lateral));
        }
        project_onto_demand(&mut u, engines, demand);
    }
    u
}

/// Euclidean projection of `u` onto `{ sum forward_i u_i = demand } ∩ [0,1]^n`,
/// the balancer's feasible set. The projection has the form
/// `u_i <- clamp(u_i + mu * forward_i, 0, 1)` for a single multiplier `mu`.
/// Zero-forward engines (the off-axis recruits) are untouched by `mu` - the
/// equality is the firing set's contract alone. The math also survives signed
/// coefficients: each engine's mapped contribution
/// `f_i * clamp(u_i + mu * f_i)` is nondecreasing in `mu` (slope `f_i^2`
/// where unclamped), so the mapped sum is monotone and a bisection pins it to
/// the demand. Pure.
fn project_onto_demand(u: &mut [f32], engines: &[BalanceEngine], demand: f32) {
    let mapped_sum = |mu: f32| -> f32 {
        engines
            .iter()
            .zip(u.iter())
            .map(|(e, &ui)| {
                let w = e.forward;
                w * (ui + mu * w).clamp(0.0, 1.0)
            })
            .sum::<f32>()
    };
    // Bracket the multiplier, expanding until the demand is enclosed.
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
        *ui = (*ui + mu * e.forward).clamp(0.0, 1.0);
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
fn autopilot_system(
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
            // RCS terminal settle (task 20260718-122932): the per-hull cap
            // override and the intent the autopilot writes to hand the
            // last-meters brake to the torque-free RCS primitive.
            Option<&RcsSpeedCap>,
            Option<&mut RcsIntent>,
            // RCS error-relative reference (task 20260718-151102): the autopilot
            // writes the orbital velocity here so RCS trims a fast orbit by a
            // sub-cap delta; zero (or absent) everywhere else.
            Option<&mut RcsReference>,
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
    // GOTO's goal pose: prefer the target's raw avian Position (a physics
    // body chased at closing speed must be read on the clock of the forces
    // chasing it - in FixedUpdate, GlobalTransform is the previous frame's
    // eased render pose, task 20260711-103527); the GlobalTransform fallback
    // keeps static markers without a physics body navigable.
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

        // The total well pull fighting a leg that rests at `rest_point`
        // while closing along `closing_dir`, in u/s^2: the sum of every
        // well's positive along-track component (overlapping SOIs add up;
        // a pull that helps braking is ignored, never banked). Evaluated
        // at the rest point - the worst point of a monotonic inward leg
        // (spike docs/spikes/20260710-204802). Scanning every well (they
        // are few) instead of the ship's DominantWell matters: the flip is
        // usually planned from OUTSIDE the SOI, where the ship has no
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
        // body is given its size instead of being treated as a point (task
        // 20260710-202408). Published distances are surface-relative too.
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
        // Set by the Orbit arm: gates the error-relative RCS trim (task
        // 20260718-151102), which only applies while station-keeping - the
        // desired is a fast orbital velocity, not a rest goal.
        let mut is_orbit = false;
        // The local gravitational acceleration at the orbiting ship, `mu/r^2`,
        // set by the Orbit arm. The RCS trim may only take the orbit when it has
        // clear authority over this pull (task 20260718-204640).
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
                // would have to counter if it took the orbit (task 20260718-204640).
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

        // Keep the published telemetry in step with the engaged verb: GOTO
        // and moving STOP legs update it every tick; ORBIT and a settled
        // STOP clear a stale one (disengage clears via
        // remove_maneuver_telemetry).
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

        // RCS terminal settle (task 20260718-122932): when the maneuver's GOAL
        // is rest (STOP, GOTO/GotoPos inside the standoff - `desired ~= 0`) and
        // the ship is already slow enough for the speed-capped RCS to act
        // (`|v| < cap`), hand the last-meters brake to the RCS primitive - a
        // torque-free COM push - instead of the main drive. Gated on the ship
        // granting the `Rcs` verb, so a hull without it (the mainline campaign,
        // RCS disabled pending rework) keeps the exact main-drive arrival.
        //
        // Two RCS branches share one command formula (`error / rcs_cap`,
        // proportional toward `desired`), differing only in the cap's reference
        // frame:
        //
        // - SETTLE (task 20260718-122932): the maneuver's GOAL is rest (STOP,
        //   GOTO/GotoPos inside the standoff - `desired ~= 0`) and the ship is
        //   already slow enough for the ABSOLUTE cap to act (`|v| < cap`). The
        //   reference is zero, so RCS brakes the last meters to rest.
        // - ORBIT trim (task 20260718-151102): station-keeping, where `desired`
        //   is the orbital velocity (~2.5-6 u/s, above the cap). The RESIDUAL
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
        // inward pull `mu/r^2`, or a perturbed ship spirals into the well faster
        // than RCS can correct - the menu ambience ships crashing the asteroid
        // (task 20260718-204640). In a strong well the main drive (full
        // authority) keeps the orbit, exactly as it did before the RCS trim.
        let rcs_has_orbit_authority =
            orbit_gravity_accel < settings.rcs_accel * RCS_ORBIT_GRAVITY_AUTHORITY;
        let use_rcs_orbit =
            rcs_capable && is_orbit && rcs_has_orbit_authority && error_speed < rcs_cap;
        let use_rcs = use_rcs_settle || use_rcs_orbit;
        // The reference the cap is measured against: the orbital velocity while
        // trimming an orbit, zero otherwise (absolute cap). Written EVERY tick so
        // a stale orbital reference never lingers into a settle or the player.
        let rcs_reference_v = if use_rcs_orbit { desired } else { Vec3::ZERO };
        // Proportional command toward `desired`, scaled so a cap-sized residual
        // is full deflection; fades to zero as the residual does (no overshoot).
        // Clear to zero when not using RCS so a stale nudge never lingers.
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
                // World point of the engine (direct child of the root):
                // raw root pose composed with the local mount, and
                // thruster_impulse_system pushes from this SAME composition
                // (task 20260711-103527) - the lever arm about com_world
                // matches the torque physics applies by construction, never
                // through a render-clock GlobalTransform.
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
        // for it - any engine already on the error finishes it, and a
        // residual only a rotation could remove is accepted. This is what
        // stops the ship twitching after perfection. Legs that END AT REST
        // (STOP, GOTO, GotoPos) use the wider settle band: the endgame of a
        // translation leg lives in sub-u/s errors - the brake tail, the
        // boundary creep, the doorstep residual - and chasing those with
        // attitude swings was the "wobbles on GOTO" playtest (task
        // 20260711-140234). Scoping by LEG, not by desired == 0, is
        // deliberate: the hunt's onset is in the brake tail where desired
        // is still nonzero - the desired-zero scoping was tried and left
        // the terminal spin bit-for-bit unchanged (spike
        // docs/spikes/20260711-140234-feel-filtering.md, fix record). Only
        // ORBIT keeps the tight band: station-keeping is the one regime
        // whose job is chasing small errors forever.
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
            // A GOTO that arrived at a well body parks into orbit instead
            // of handing back a ship that immediately starts falling (task
            // 20260710-195954): the one-key parking flow becomes zero-key
            // when the computer was already told where to go. engage()
            // resets the phase. The ring is planned HERE, from the leg's
            // intent - the park point, standoff above the (geometric)
            // surface - never from wherever terminal creep dragged the
            // ship: a plan-from-current-radius could ring at the band
            // bottom, and the insertion from a crept position has been
            // seen to graze the rock. max with the current radius so a
            // ship that settled slightly outside the park point is not
            // corrected inward. Breakout semantics (any flight input, Z)
            // are ORBIT's own, unchanged. Everything else - GotoPos,
            // well-less targets, STOP, a bandless well - releases as
            // before.
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
                    // cascade). The spike's deadband A/B moved this
                    // denominator together with the band - keying only the
                    // band left the terminal spin unchanged, which is how
                    // the coupling was found (task 20260711-140234).
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
            // backwards, the re-entry error re-aims the hull, and the
            // arrival bounces on the boundary in a limit cycle (task
            // 20260711-140241's trace; the cycle was previously masked by
            // the accidental dither of the cross-clock command handoff).
            // Once the wind-down tail alone covers the remaining error, the
            // correct demand is zero: cut and coast to rest.
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
    mut q_rcs: Query<(&mut RcsIntent, Option<&mut RcsReference>)>,
) {
    for (mut input, &ChildOf(parent)) in &mut q_thruster {
        if parent == remove.entity {
            **input = 0.0;
        }
    }
    // Clear the RCS command AND its error-relative reference (tasks
    // 20260718-122932, 20260718-151102): the autopilot writes both while
    // settling/trimming, and rcs_burn_system acts on ANY non-zero intent
    // regardless of autopilot state. A residual intent would push the ship past
    // rest toward the cap; a stale orbital reference would silently rebase the
    // player's next absolute-cap nudge. Zero both on disengage.
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

/// Manual main-drive burn for intent-carrying ships with no autopilot
/// engaged: allocate the analog burn over the live unbound engine set as a
/// torque-nulling throttle vector, so an off-center or damage-shifted drive
/// still pushes the resultant force through the COM. The forward set delivers
/// the demand via differential throttle when it has headroom; when it does
/// not (the single damage-shifted main drive), the allocator recruits an
/// off-axis engine for pure counter-torque, trading a bounded sideways drift
/// for a straight heading. Only when nothing can help - no headroom and no
/// off-axis engine left - does the ship still pull, held by the PD as before.
fn manual_burn_system(
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
/// direction, so the pilot - or the autopilot (task 20260718-122932) - can
/// nudge the hull the last few meters of a docking approach.
///
/// Two properties define it (spike 20260718-122508):
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
fn rcs_burn_system(
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
        // `ship_grants_verb` (input/player.rs) so the verb stays authoritative
        // no matter who drives the primitive.
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
        // The cap is measured against this REFERENCE velocity: absent/zero means
        // the plain absolute cap (player fine-adjust, STOP/GOTO settle); the
        // autopilot supplies the orbital velocity here so RCS caps the RESIDUAL
        // `v - reference` and can trim a fast-moving orbit (task 20260718-151102).
        let reference = reference.map(|r| r.0).unwrap_or(Vec3::ZERO);

        // Accumulate the per-axis capped push, then apply once at the COM so
        // the summed impulse still produces zero torque. The cap is per
        // ship-local axis, NOT a speed-magnitude limit: a diagonal command can
        // reach up to `sqrt(2..3) * cap` combined (each axis caps independently
        // - the spike's per-axis design), which is fine for docking nudges.
        let mut impulse = Vec3::ZERO;
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            let cmd = intent.0.dot(axis);
            if cmd.abs() < 1e-4 {
                continue;
            }
            let world_axis = rotation.mul_vec3(axis);
            // Residual along the axis, relative to the reference: this is what
            // the cap limits, so a prograde trim of an orbit sees only the small
            // `v - v_orbit` delta, not the full orbital speed.
            let along = (velocity - reference).dot(world_axis);
            // Headroom toward the commanded sign: full push while far from the
            // cap, tapering to zero as the along-axis speed nears the cap in
            // the pushed direction; the opposite direction always has headroom.
            let gate = ((cap - cmd.signum() * along) / taper_band).clamp(0.0, 1.0);
            // Desired acceleration this tick; * mass so the 1/mass inside
            // apply_linear_impulse yields exactly `accel * dt` (mass-independent
            // feel).
            let accel = cmd.clamp(-1.0, 1.0) * settings.rcs_accel * gate;
            impulse += world_axis * (accel * dt * mass);
        }
        if impulse != Vec3::ZERO {
            force.apply_linear_impulse(impulse);
        }
    }
}

/// Per-tick decay of the PLAYER's `RcsIntent`, so RCS fine-adjust is DELTA-driven
/// (force follows the mouse/scroll motion and stops when the input stops) instead
/// of a persistent virtual joystick that keeps pushing after you let go - which
/// playtested as "way too hard to control" (task 20260718-185826). The input
/// layer SETS the intent from each frame's motion; this fades it back to zero
/// when no fresh input arrives. Gated on [`RcsActive`] - the player's SHIFT
/// modal - so the AUTOPILOT's own `RcsIntent` (which it rewrites every tick,
/// and which never carries `RcsActive`) is untouched. Runs after
/// [`rcs_burn_system`] in the chain, so the intent this tick is spent before
/// it decays.
fn decay_player_rcs_intent(mut q_intent: Query<&mut RcsIntent, With<RcsActive>>) {
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

    // --- Pure helpers ----------------------------------------------------

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

    /// A damage-shifted hull with a single main drive: the ballast block far
    /// off the centerline stands in for the surviving half of a ship that
    /// lost a side section, so the COM sits well off the lone drive's thrust
    /// line and a burn torques the hull past what the PD can hold. With
    /// `with_lateral` a single sideways thruster survives aft of the COM -
    /// the counter-torque candidate. Returns the ship and the lateral.
    fn spawn_damage_shifted_single_drive(app: &mut App, with_lateral: bool) -> (Entity, Entity) {
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
        // The ballast: surviving structure whose mass drags the COM to +X.
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("ballast"),
            Transform::from_xyz(6.0, 0.0, 0.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("main drive"),
            ThrusterSectionMarker,
            ThrusterSectionMagnitude(2.0),
            ThrusterSectionInput(0.0),
            Transform::from_xyz(0.0, 0.0, 1.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
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
        let lateral = app
            .world_mut()
            .spawn((
                ChildOf(ship),
                Name::new("lateral"),
                ThrusterSectionMarker,
                ThrusterSectionMagnitude(2.0),
                ThrusterSectionInput(0.0),
                // Aft of the COM, thrusting +X (local -Z rotated -90 deg
                // about Y): its torque about the COM opposes the main
                // drive's, so it is the recruitable counter-torque.
                Transform::from_xyz(0.0, 0.0, 3.0)
                    .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ))
            .id();
        if !with_lateral {
            app.world_mut().entity_mut(lateral).despawn();
        }
        (ship, lateral)
    }

    /// The off-axis counter-torque task (20260709-224518): a single main
    /// drive on a damage-shifted hull cannot balance itself by differential
    /// throttle (there is nothing in the firing set to trim against), but the
    /// allocator recruits the surviving lateral purely for its counter-torque
    /// and the ship holds its heading within the centered-drive tolerance -
    /// even at full stick, because the recruit's trim budget is its own
    /// throttle, not the main drive's headroom. Without the lateral the same
    /// hull pulls, exactly the pre-allocation floor.
    #[test]
    fn single_drive_on_a_shifted_hull_recruits_a_lateral_to_hold_heading() {
        let burn_outcome = |with_lateral: bool| -> (f32, f32) {
            let mut app = flight_app();
            let (ship, lateral) = spawn_damage_shifted_single_drive(&mut app, with_lateral);
            settle(&mut app);
            app.world_mut().get_mut::<FlightIntent>(ship).unwrap().burn = 1.0;
            for _ in 0..120 {
                app.update();
            }
            let drift = app
                .world()
                .get::<Rotation>(ship)
                .unwrap()
                .0
                .angle_between(Quat::IDENTITY);
            let recruit = if with_lateral {
                **app.world().get::<ThrusterSectionInput>(lateral).unwrap()
            } else {
                0.0
            };
            (drift, recruit)
        };

        let (held, recruit) = burn_outcome(true);
        assert!(
            held < 0.15,
            "the recruited lateral must hold the heading ({held} rad drift)"
        );
        assert!(
            recruit > 0.2,
            "the lateral must actually be firing for counter-torque \
             (input {recruit})"
        );
        let (pulled, _) = burn_outcome(false);
        assert!(
            pulled > 0.4,
            "without a lateral to recruit the shifted hull must still pull \
             ({pulled} rad drift)"
        );
    }

    /// The same recruitment through the autopilot path: a STOP burn on the
    /// damage-shifted single-drive hull lights the lateral (it is outside the
    /// firing cone, recruited by the wrench allocation in the world frame),
    /// and the maneuver still converges to rest - the recruit's sideways
    /// force is the decided bounded drift, and chasing it down is exactly
    /// what the autopilot's velocity-error rule does. Heading straightness
    /// under a fixed burn is pinned by the manual-path test above; here the
    /// hull deliberately turns to kill the drift, so rest is the invariant.
    #[test]
    fn autopilot_burn_recruits_a_lateral_on_a_shifted_hull() {
        let mut app = flight_app();
        let (ship, lateral) = spawn_damage_shifted_single_drive(&mut app, true);
        settle(&mut app);
        // Moving backward (+Z): STOP's velocity error points -Z, straight
        // along the main drive - no rotation needed, the burn starts at once.
        // Enough speed that the deceleration takes long enough for the
        // spooled inputs to be observable mid-burn.
        app.world_mut().get_mut::<LinearVelocity>(ship).unwrap().0 = Vec3::new(0.0, 0.0, 20.0);
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));
        // Sample DURING the burn - the autopilot stops the ship and winds the
        // engines down, so an after-the-fact reading would see only zeros.
        let mut recruit = 0.0f32;
        let mut frames = 0;
        while app.world().get::<Autopilot>(ship).is_some() && frames < 1500 {
            app.update();
            frames += 1;
            recruit = recruit.max(**app.world().get::<ThrusterSectionInput>(lateral).unwrap());
        }
        assert!(
            recruit > 0.2,
            "the autopilot must recruit the lateral for counter-torque \
             (peak input {recruit})"
        );
        assert!(
            app.world().get::<Autopilot>(ship).is_none(),
            "STOP must converge to rest despite the recruit's bounded drift \
             (speed {} after {frames} frames)",
            velocity_of(&app, ship).length()
        );
        // The recruit's sideways push leaves a LATERAL residual, and the
        // settle band's contract (task 20260711-140234) is that sub-band
        // crumbs off the drive axis are released, not hunted with attitude
        // flips - so rest here means within the settle band, not the old
        // 0.5. The shipped single-centered-drive ship keeps its exact rest:
        // an axial residual keeps the drive's aligned authority, so release
        // still waits for stop_speed_epsilon.
        let settle_band = app.world().resource::<FlightSettings>().settle_deadband;
        assert!(
            velocity_of(&app, ship).length() < settle_band + 0.05,
            "the ship must rest within the settle band ({:?})",
            velocity_of(&app, ship)
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

    /// A firing-set engine perfectly on the burn axis: full forward per unit
    /// input, no perpendicular force, only its lever-arm torque.
    fn main_engine(forward: f32, torque: Vec3) -> BalanceEngine {
        BalanceEngine {
            forward,
            lateral: Vec3::ZERO,
            torque,
            primary: true,
        }
    }

    /// An engine perpendicular to the burn axis: zero forward, all of its
    /// thrust is off-axis force, recruitable only for its torque.
    fn lateral_engine(lateral: Vec3, torque: Vec3) -> BalanceEngine {
        BalanceEngine {
            forward: 0.0,
            lateral,
            torque,
            primary: false,
        }
    }

    #[test]
    fn balance_throttles_splits_demand_to_null_torque() {
        // Two forward engines (weight 1 each) with opposing but unequal lever
        // arms: A torques +0.5 about the COM per unit, B torques -1.5. A
        // uniform half-throttle (0.5, 0.5) would net -0.5; the balancer instead
        // runs A hotter than B so 0.5*uA - 1.5*uB = 0 while uA + uB = 1, i.e.
        // uA = 0.75, uB = 0.25 (hand-computed).
        let engines = [
            main_engine(1.0, Vec3::new(0.0, 0.5, 0.0)),
            main_engine(1.0, Vec3::new(0.0, -1.5, 0.0)),
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
            main_engine(2.0, Vec3::new(0.0, 0.0, 1.0)),
            main_engine(2.0, Vec3::new(0.0, 0.0, -1.0)),
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
        let lone = [main_engine(1.0, Vec3::new(0.0, 2.0, 0.0))];
        assert!((balance_throttles(&lone, 0.5)[0] - 0.5).abs() < 1e-3);

        // A full-throttle demand pins every engine at 1.0: no headroom to trim,
        // so the residual torque is left to the PD (the pre-balance behavior).
        let full = [
            main_engine(1.0, Vec3::new(0.0, 0.5, 0.0)),
            main_engine(1.0, Vec3::new(0.0, -1.5, 0.0)),
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
    fn balance_throttles_recruits_an_off_axis_engine_for_counter_torque() {
        // A lone main drive off the COM (torque -1 per unit about Y) and one
        // lateral whose lever arm gives +2 per unit. The demand pins the main
        // at 0.5 (only it has forward thrust); the lateral is recruited
        // purely for counter-torque. Hand-computed optimum of
        // (2 uL - 0.5)^2 + LATERAL_PENALTY * uL^2:
        // uL = 2 * 0.5 / (4 + 0.05) = 0.2469...
        let engines = [
            main_engine(1.0, Vec3::new(0.0, -1.0, 0.0)),
            lateral_engine(Vec3::X, Vec3::new(0.0, 2.0, 0.0)),
        ];
        let u = balance_throttles(&engines, 0.5);
        assert!(
            (u[0] - 0.5).abs() < 1e-3,
            "main must hold the demand: {u:?}"
        );
        assert!((u[1] - 0.2469).abs() < 1e-2, "lateral recruit: {u:?}");
        // The recruit nulls all but the lateral-penalty residual of the
        // torque - a couple of percent, well inside the PD's hold.
        let torque: Vec3 = engines.iter().zip(&u).map(|(e, &x)| e.torque * x).sum();
        assert!(torque.length() < 0.02, "net torque = {torque}");
        // The price is a bounded sideways force, honestly reported.
        let lateral: Vec3 = engines.iter().zip(&u).map(|(e, &x)| e.lateral * x).sum();
        assert!((lateral.x - u[1]).abs() < 1e-3, "lateral force = {lateral}");
    }

    #[test]
    fn balance_throttles_leaves_off_axis_engines_dark_on_a_balanced_ship() {
        // A symmetric twin drive nulls its own torque at the uniform seed, so
        // the objective's gradient is zero and the idle lateral must never be
        // recruited - no gratuitous sideways burn on a healthy ship.
        let engines = [
            main_engine(1.0, Vec3::new(0.0, 1.0, 0.0)),
            main_engine(1.0, Vec3::new(0.0, -1.0, 0.0)),
            lateral_engine(Vec3::X, Vec3::new(0.0, 3.0, 0.0)),
        ];
        let u = balance_throttles(&engines, 1.0);
        assert!(
            (u[0] - 0.5).abs() < 1e-3 && (u[1] - 0.5).abs() < 1e-3,
            "{u:?}"
        );
        assert_eq!(u[2], 0.0, "idle lateral must stay dark: {u:?}");
    }

    #[test]
    fn balance_throttles_counter_torques_even_at_full_throttle() {
        // Differential throttle needs forward headroom; a recruited lateral
        // does not - its trim budget is its own throttle box. A full-stick
        // demand pins the lone main at 1.0 and the lateral still nulls the
        // torque (uL = 2 / (4 + 0.05) = 0.4938...).
        let engines = [
            main_engine(1.0, Vec3::new(0.0, -1.0, 0.0)),
            lateral_engine(Vec3::X, Vec3::new(0.0, 2.0, 0.0)),
        ];
        let u = balance_throttles(&engines, 1.0);
        assert!((u[0] - 1.0).abs() < 1e-3, "{u:?}");
        assert!((u[1] - 0.4938).abs() < 1e-2, "{u:?}");
    }

    #[test]
    fn balance_throttles_skips_a_lateral_mounted_through_the_com() {
        // An off-axis engine with a tiny lever arm buys almost no torque for
        // its sideways force, so the penalty keeps it (nearly) dark and the
        // torque residual goes to the PD instead of the hull drifting.
        let engines = [
            main_engine(1.0, Vec3::new(0.0, -1.0, 0.0)),
            lateral_engine(Vec3::X, Vec3::new(0.0, 0.05, 0.0)),
        ];
        let u = balance_throttles(&engines, 0.5);
        // Optimum uL = 0.05 * 0.5 / (0.05^2 + 0.05) = 0.476 nulls only ~5% of
        // the torque; what matters is the sideways force stays bounded while
        // most of the residual is left to the PD.
        let torque: Vec3 = engines.iter().zip(&u).map(|(e, &x)| e.torque * x).sum();
        assert!(
            torque.length() > 0.45,
            "a useless lever must not be trusted with the torque: {u:?}"
        );
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
        let mut app = unfinished_flight_app();
        app.finish();
        app
    }

    /// The flight harness plus the real gravity layer, for the ORBIT tests:
    /// wells actually pull, ship roots opt in through the plugin's observer.
    fn orbit_app() -> App {
        let mut app = unfinished_flight_app();
        app.add_plugins(crate::gravity::NovaGravityPlugin);
        app.finish();
        app
    }

    fn unfinished_flight_app() -> App {
        let mut app = unfinished_integrity_physics_app();
        app.init_resource::<FlightSettings>();
        app.init_resource::<GravitySettings>();
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
        app.add_observer(remove_maneuver_telemetry);
        app.add_systems(
            FixedUpdate,
            (
                autopilot_system,
                manual_burn_system,
                rcs_burn_system,
                decay_player_rcs_intent,
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

    /// Withhold the RCS verb on every controller of `ship`. The legacy autopilot
    /// tests predate RCS and assert the MAIN-DRIVE arrival (flip + retro burn);
    /// with the verb granted (the production default) the autopilot would settle
    /// their terminal via RCS instead (task 20260718-122932). Disabling RCS here
    /// keeps them exercising the behavior they were written for - the same
    /// opt-out the mainline campaign uses while RCS is off pending rework.
    fn withhold_rcs(app: &mut App, ship: Entity) {
        let controllers: Vec<Entity> = app
            .world_mut()
            .query_filtered::<(Entity, &ChildOf), With<ControllerSectionMarker>>()
            .iter(app.world())
            .filter(|(_, ChildOf(parent))| *parent == ship)
            .map(|(entity, _)| entity)
            .collect();
        for controller in controllers {
            app.world_mut()
                .entity_mut(controller)
                .insert(WithheldVerbs([FlightVerb::Rcs].into_iter().collect()));
        }
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

    // --- RCS fine-adjustment (task 20260718-122906) ----------------------

    /// A ship that grants RCS, carrying a (zero) intent and a per-hull cap,
    /// with its mass finalized. Returns (ship, controller).
    fn spawn_rcs_ship(app: &mut App, cap: f32) -> (Entity, Entity) {
        let (ship, _thruster, controller) = spawn_ship(app);
        app.world_mut()
            .entity_mut(ship)
            .insert((RcsIntent::default(), RcsSpeedCap(cap)));
        settle(app);
        (ship, controller)
    }

    fn set_rcs(app: &mut App, ship: Entity, intent: Vec3) {
        app.world_mut().get_mut::<RcsIntent>(ship).unwrap().0 = intent;
    }

    fn angular_speed_of(app: &App, ship: Entity) -> f32 {
        app.world().get::<AngularVelocity>(ship).unwrap().0.length()
    }

    /// The virtual-joystick accumulator (task 20260718-122912) integrates the
    /// held offset and clamps it to the unit range the primitive expects, so a
    /// sustained push saturates at 1 and pulling back walks it toward the other
    /// rail rather than running away.
    #[test]
    fn accumulate_rcs_axis_integrates_and_clamps_to_the_unit_range() {
        // Integration accumulates across calls (held-direction persistence).
        let a = accumulate_rcs_axis(0.0, 0.3);
        let b = accumulate_rcs_axis(a, 0.3);
        assert!((b - 0.6).abs() < 1e-6, "offsets add up: {b}");
        // Saturates at the rails, never past.
        assert_eq!(accumulate_rcs_axis(0.9, 0.5), 1.0);
        assert_eq!(accumulate_rcs_axis(-0.9, -0.5), -1.0);
        // Pulling back from a rail walks toward the other one.
        assert!((accumulate_rcs_axis(1.0, -0.4) - 0.6).abs() < 1e-6);
    }

    /// The player's `RcsIntent` is delta-driven: with `RcsActive` and no fresh
    /// input, it fades to zero over ticks (task 20260718-185826), so the ship
    /// stops nudging when the mouse stops instead of coasting a held joystick.
    /// An autopilot ship (no `RcsActive`) is NOT decayed - it rewrites its own
    /// intent each tick.
    #[test]
    fn player_rcs_intent_decays_when_input_stops_but_autopilot_intent_does_not() {
        let mut app = flight_app();
        let (player, _, _) = spawn_ship(&mut app);
        let (auto, _, _) = spawn_ship(&mut app);
        settle(&mut app);
        // Player: RcsActive + a held intent from a mouse frame that then stops.
        app.world_mut()
            .entity_mut(player)
            .insert((RcsIntent(Vec3::new(0.8, 0.0, 0.0)), RcsActive));
        // Autopilot-style: an intent WITHOUT RcsActive (nothing rewrites it here).
        app.world_mut()
            .entity_mut(auto)
            .insert(RcsIntent(Vec3::new(0.8, 0.0, 0.0)));

        for _ in 0..30 {
            app.update();
        }

        assert!(
            app.world().get::<RcsIntent>(player).unwrap().0.length() < 1e-3,
            "the player's held intent decays to ~zero without fresh input (got {:?})",
            app.world().get::<RcsIntent>(player).unwrap().0
        );
        assert!(
            app.world().get::<RcsIntent>(auto).unwrap().0.length() > 0.5,
            "a non-RcsActive (autopilot) intent is NOT decayed (got {:?})",
            app.world().get::<RcsIntent>(auto).unwrap().0
        );
    }

    /// A held RCS nudge builds the along-axis speed up toward the cap and then
    /// levels off - never past it - and, applied at the COM, never spins the
    /// hull or drifts off-axis. Identity frame, so ship-local +X is world +X.
    #[test]
    fn rcs_builds_to_the_cap_then_levels_off_without_torque() {
        let mut app = flight_app();
        let cap = 2.0;
        let (ship, _controller) = spawn_rcs_ship(&mut app, cap);
        set_rcs(&mut app, ship, Vec3::X);
        for _ in 0..600 {
            app.update();
            let vx = velocity_of(&app, ship).x;
            assert!(
                vx <= cap + 1e-2,
                "RCS must never push past the cap (vx={vx})"
            );
        }
        let v = velocity_of(&app, ship);
        assert!(
            v.x > cap - 0.1,
            "a held nudge should reach the cap (vx={})",
            v.x
        );
        assert!(
            v.y.abs() < 1e-2 && v.z.abs() < 1e-2,
            "no off-axis drift ({v:?})"
        );
        assert!(
            angular_speed_of(&app, ship) < 1e-3,
            "an impulse at the COM must not spin the hull"
        );
    }

    /// The cap is directional: at `+cap` a forward command adds nothing, but the
    /// opposite command still accelerates the ship down to `-cap` - the user's
    /// "moving forward, RCS forward does nothing, backward still works" rule.
    #[test]
    fn rcs_holds_the_cap_forward_but_reverses_freely() {
        let mut app = flight_app();
        let cap = 2.0;
        let (ship, _controller) = spawn_rcs_ship(&mut app, cap);
        set_rcs(&mut app, ship, Vec3::X);
        for _ in 0..600 {
            app.update();
        }
        let at_cap = velocity_of(&app, ship).x;
        assert!(at_cap > cap - 0.1, "should be at the cap (vx={at_cap})");
        // Holding +X longer adds no further speed.
        for _ in 0..200 {
            app.update();
        }
        let still = velocity_of(&app, ship).x;
        assert!(
            (still - at_cap).abs() < 1e-2,
            "at the cap, more +X buys nothing ({at_cap} -> {still})"
        );
        // The opposite command decelerates through zero toward -cap.
        set_rcs(&mut app, ship, -Vec3::X);
        for _ in 0..900 {
            app.update();
        }
        let reversed = velocity_of(&app, ship).x;
        assert!(
            reversed < -(cap - 0.1),
            "reverse RCS still works down to -cap (vx={reversed})"
        );
    }

    /// RCS is a controller verb: a ship whose controller withholds `Rcs` does
    /// not move, even with an intent written on it.
    #[test]
    fn rcs_does_nothing_without_the_verb() {
        let mut app = flight_app();
        let (ship, controller) = spawn_rcs_ship(&mut app, 2.0);
        app.world_mut()
            .entity_mut(controller)
            .insert(WithheldVerbs([FlightVerb::Rcs].into_iter().collect()));
        set_rcs(&mut app, ship, Vec3::X);
        for _ in 0..300 {
            app.update();
        }
        assert!(
            velocity_of(&app, ship).length() < 1e-3,
            "no RCS verb, no fine-adjust"
        );
    }

    /// The push is in the ship's LOCAL frame: with the hull yawed 90 degrees, a
    /// local +X command drives the ship along the rotated world axis, not world
    /// +X, with no off-axis drift and no spin (the `degenerate-inertia-frames`
    /// lesson - exercise a non-identity frame).
    #[test]
    fn rcs_pushes_along_the_ship_local_axis_in_a_rotated_frame() {
        let mut app = flight_app();
        let cap = 2.0;
        let (ship, _thruster, controller) = spawn_ship(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert((RcsIntent::default(), RcsSpeedCap(cap)));
        let yaw = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        app.world_mut()
            .get_mut::<ControllerSectionRotationInput>(controller)
            .unwrap()
            .0 = yaw;
        // Let the PD swing the hull to the yaw and come to rest there.
        for _ in 0..400 {
            app.update();
        }
        assert!(
            angular_speed_of(&app, ship) < 1e-2,
            "hull should be settled before the RCS push"
        );
        // Command local +X; capture the ACTUAL hull frame so the test tolerates
        // any residual PD error.
        let world_axis = app.world().get::<Rotation>(ship).unwrap().mul_vec3(Vec3::X);
        assert!(
            world_axis.dot(Vec3::X).abs() < 0.05,
            "the hull really is yawed away from world +X ({world_axis:?})"
        );
        set_rcs(&mut app, ship, Vec3::X);
        for _ in 0..600 {
            app.update();
        }
        let v = velocity_of(&app, ship);
        let along = v.dot(world_axis);
        let off = (v - world_axis * along).length();
        assert!(
            along > cap - 0.15,
            "reaches the cap along the rotated local +X (along={along})"
        );
        assert!(off < 0.05, "no world off-axis drift (off={off})");
        assert!(
            angular_speed_of(&app, ship) < 5e-2,
            "still no meaningful spin from the COM push"
        );
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

    fn position_of(app: &App, ship: Entity) -> Vec3 {
        **app.world().get::<Position>(ship).unwrap()
    }

    fn run(app: &mut App, frames: usize) {
        for _ in 0..frames {
            app.update();
        }
    }

    /// The soft manual speed cap (playtest 2026-07-12 finding 1): a held
    /// full burn levels off just past the cap - the overshoot is the
    /// spool-down tail, bounded by accel / spool_down_rate - while the
    /// SAME ship uncapped blows straight past it. The uncapped leg is the
    /// delivery guard proving the burn itself works AND the measured
    /// acceleration the overshoot bound derives from (this rig is
    /// deliberately over-powered; the physics-derived bound keeps the
    /// assertion honest instead of hardcoding a slack constant).
    #[test]
    fn manual_burn_levels_off_at_the_speed_cap() {
        const CAP: f32 = 3.0;
        const FRAMES: usize = 1200;

        let run_ship = |cap: Option<f32>| -> (f32, f32) {
            let mut app = flight_app();
            let (ship, ..) = spawn_ship(&mut app);
            app.world_mut()
                .entity_mut(ship)
                .insert(FlightIntent { burn: 1.0 });
            if let Some(cap) = cap {
                app.world_mut().entity_mut(ship).insert(FlightSpeedCap(cap));
            }
            run(&mut app, FRAMES / 2);
            let mid = velocity_of(&app, ship).length();
            run(&mut app, FRAMES / 2);
            (mid, velocity_of(&app, ship).length())
        };

        let (uncapped_mid, uncapped) = run_ship(None);
        assert!(
            uncapped > CAP + 2.0,
            "delivery guard: the uncapped burn must sail past the cap, got {uncapped}"
        );
        // Measured acceleration of THIS rig, from the uncapped leg.
        let accel = uncapped_mid / (FRAMES as f32 / 2.0 / 60.0);

        let (capped_mid, capped) = run_ship(Some(CAP));
        // Overshoot bound: the spool-down tail keeps pushing for
        // ~1/spool_down_rate after the taper cuts the command, plus a
        // couple of ticks of taper-crossing.
        let settings = FlightSettings::default();
        let bound = CAP + accel * (1.0 / settings.spool_down_rate + 2.0 / 60.0) + 0.2;
        assert!(
            capped <= bound,
            "a capped ship levels off near the cap: got {capped}, bound {bound} \
             (cap {CAP}, measured accel {accel})"
        );
        assert!(
            capped >= CAP * 0.5,
            "the cap is a ceiling, not a parking brake: got {capped} vs cap {CAP}"
        );
        assert!(
            (capped - capped_mid).abs() < 0.05,
            "the capped ship has PLATEAUED, not still accelerating: {capped_mid} -> {capped}"
        );
    }

    /// A bare hull for the impulse-frame tests: no FlightIntent (the manual
    /// burn layer would zero hand-set throttles) and no controller (a PD
    /// would damp away the very spin the test measures). The only torque
    /// source left is the impulse system's application point.
    fn spawn_uncontrolled_dumbbell_with_com_lateral(app: &mut App) -> (Entity, Entity) {
        let ship = app
            .world_mut()
            // TransformInterpolation matches production ships
            // (base_scenario_object): PostUpdate then propagates the EASED
            // pose, so the stale GlobalTransform trails raw physics on every
            // tick, not just on the 64-vs-60 Hz double-tick frames.
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                TransformInterpolation,
            ))
            .id();
        for z in [-1.0, 1.0] {
            app.world_mut().spawn((
                ChildOf(ship),
                Name::new("hull"),
                Transform::from_xyz(0.0, 0.0, z),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ));
        }
        // Unit sections at z = -1, 0, +1 put the COM at the origin, which is
        // exactly the lateral engine's mount: its +X thrust line passes
        // through the COM and the TRUE torque is zero by construction.
        let thruster = app
            .world_mut()
            .spawn((
                ChildOf(ship),
                Name::new("com lateral thruster"),
                ThrusterSectionMarker,
                ThrusterSectionMagnitude(1.0),
                ThrusterSectionInput(0.0),
                Transform::from_xyz(0.0, 0.0, 0.0)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ))
            .id();
        (ship, thruster)
    }

    /// Regression for task 20260711-121701: the shipped 5-section player
    /// geometry (all sections on the z axis, unit masses, single rear drive
    /// at z = +2, PD at the shipped 4/4/40) holding the reverse direction
    /// from 300 u/s - the exact "wobbles when decelerating" playtest
    /// scenario. The diagnostic trace measured the hull DEAD STEADY here
    /// (max spin 0.0023 rad/s through flip + full 22 s burn), ruling out a
    /// physical mechanism; this pins that so any future speed-coupled
    /// torque regression (the 20260711-103527 family) fails loudly.
    #[test]
    fn hold_reverse_decel_from_300_keeps_the_hull_steady() {
        let mut app = flight_app();
        let ship = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                TransformInterpolation,
                SpaceshipRootMarker,
                FlightIntent::default(),
            ))
            .id();
        let section = |app: &mut App, name: &str, z: f32| {
            app.world_mut()
                .spawn((
                    ChildOf(ship),
                    Name::new(name.to_string()),
                    Transform::from_xyz(0.0, 0.0, z),
                    Collider::cuboid(1.0, 1.0, 1.0),
                    ColliderDensity(1.0),
                ))
                .id()
        };
        let controller = section(&mut app, "controller", 0.0);
        app.world_mut().entity_mut(controller).insert((
            ControllerSectionMarker,
            ControllerSectionRotationInput::default(),
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_torque: 40.0,
            },
            PDControllerTarget(ship),
        ));
        section(&mut app, "hull_front", 1.0);
        section(&mut app, "hull_back", -1.0);
        let thruster = section(&mut app, "thruster", 2.0);
        app.world_mut().entity_mut(thruster).insert((
            ThrusterSectionMarker,
            ThrusterSectionMagnitude(1.0),
            ThrusterSectionInput(0.0),
        ));
        section(&mut app, "turret_mass", -2.0);
        settle(&mut app);

        // Phase 1: cruising nose-first at 300 u/s, the player flips the
        // command to retrograde (mouse still afterwards: command constant).
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::NEG_Z * 300.0));
        app.world_mut()
            .get_mut::<ControllerSectionRotationInput>(controller)
            .unwrap()
            .0 = Quat::from_rotation_y(std::f32::consts::PI);

        run(&mut app, 240);
        // Delivery guard: the flip must actually have happened, or the
        // steady-burn bound below is vacuous.
        assert!(
            forward_of(&app, ship).dot(Vec3::Z) > 0.999,
            "the command flip must complete before the burn phase"
        );

        // Phase 2: hold full reverse burn until (near) rest.
        app.world_mut().get_mut::<FlightIntent>(ship).unwrap().burn = 1.0;
        let mut max_spin_burn = 0.0f32;
        for _ in 0..3600 {
            app.update();
            let spin = app.world().get::<AngularVelocity>(ship).unwrap().length();
            max_spin_burn = max_spin_burn.max(spin);
            if velocity_of(&app, ship).length() < 1.0 {
                break;
            }
        }
        // Delivery guard: the burn must have delivered the deceleration.
        let speed = velocity_of(&app, ship).length();
        assert!(
            speed < 1.0,
            "the reverse burn must bring 300 u/s to rest, got {speed}"
        );
        assert!(
            max_spin_burn < 0.05,
            "the hull must stay steady while decelerating, max spin {max_spin_burn} rad/s"
        );
    }

    /// The impulse system must push from the raw physics pose, not the render
    /// pose (task 20260711-103527). In FixedUpdate, `GlobalTransform` is the
    /// PREVIOUS frame's propagation - since the interpolation opt-in
    /// (2026-07-09) an eased pose one to two ticks behind raw physics. A
    /// lateral engine whose thrust line passes exactly through the COM adds
    /// zero true torque, but pushed from a point ~`v * dt` behind a fast hull
    /// it torques the ship every tick: the high-speed twitch/flip of the
    /// 2026-07-10 playtest. At 150 u/s the stale point trails ~2.3 u, which
    /// spun this rig past 1 rad/s within a handful of frames before the fix.
    #[test]
    fn high_speed_lateral_burn_through_the_com_adds_no_spin() {
        let mut app = flight_app();
        let (ship, thruster) = spawn_uncontrolled_dumbbell_with_com_lateral(&mut app);
        settle(&mut app);

        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::Z * 150.0));
        app.world_mut()
            .get_mut::<ThrusterSectionInput>(thruster)
            .unwrap()
            .0 = 1.0;

        run(&mut app, 60);

        let spin = app.world().get::<AngularVelocity>(ship).unwrap().length();
        assert!(
            spin < 0.05,
            "a thrust line through the COM must not spin the hull, got {spin} rad/s"
        );
    }

    /// The playtest symptom behind 20260710-231931: at high velocity the
    /// hull itself twitched - real attitude jitter, not a camera artifact.
    /// The mechanism was 20260711-103527's stale impulse point, which only
    /// bites when the thrust has a component PERPENDICULAR to the travel
    /// (a decel path with drift correction), so the faithful rig is a full
    /// production stack burning across its own velocity: PD at the shipped
    /// 40 torque budget, TransformInterpolation on the hull, centered
    /// drive, high cross velocity, zero rotation command. Against the
    /// pre-fix impulse code this rig's PD is overwhelmed by ~2.3 u of
    /// application-point error per tick and the max observed spin runs
    /// away past 1 rad/s; a steady hull must stay at zero the whole run.
    #[test]
    fn cross_velocity_burn_keeps_the_hull_steady_at_high_speed() {
        let mut app = flight_app();
        let (ship, _, controller) = spawn_ship(&mut app);
        // Production-faithful scheduling: clock-bug rigs must mirror the
        // production interpolation opt-in (see the 20260711-103527 retro).
        app.world_mut()
            .entity_mut(ship)
            .insert(TransformInterpolation);
        app.world_mut()
            .get_mut::<PDController>(controller)
            .unwrap()
            .max_torque = 40.0; // the shipped torque budget
        settle(&mut app);

        // Fast cross-travel (+X) under a full forward burn (-Z): thrust
        // perpendicular to velocity, the regime where a stale application
        // point torques the hull.
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::X * 150.0));
        app.world_mut().get_mut::<FlightIntent>(ship).unwrap().burn = 1.0;

        let mut max_spin = 0.0f32;
        for _ in 0..180 {
            app.update();
            max_spin = max_spin.max(app.world().get::<AngularVelocity>(ship).unwrap().length());
        }
        // Delivery guard (review R1.1): a steady hull only proves the fix if
        // the engine actually fired - a silent burn seam would pass the spin
        // bound vacuously. Three seconds of full burn must have accelerated
        // the ship along -Z.
        let burned = velocity_of(&app, ship).z;
        assert!(
            burned < -20.0,
            "the -Z main drive must have delivered thrust, got vz {burned}"
        );
        assert!(
            max_spin < 0.05,
            "zero rotation command + centered drive must hold the hull steady \
             at speed, max spin {max_spin} rad/s"
        );
    }

    #[test]
    fn stop_flips_the_hull_and_kills_velocity_with_no_external_force() {
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        withhold_rcs(&mut app, ship);
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

    // --- ORBIT: pure plan helpers -----------------------------------------

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
        let well = GravityWell::from_surface_gravity(3.0, 20.0, &gravity);
        // Band for the sanity rock: [1.5 * 21, 0.9 * 0.85 * 160] = [31.5, 122.4].
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
        let v_circ = crate::gravity::circular_orbit_speed(mu, 50.0);
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

    // --- ORBIT: physics-level ----------------------------------------------

    /// The spike's sanity rock, on rails at the origin, pulling for real.
    fn spawn_orbit_well(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                RigidBody::Static,
                Transform::default(),
                crate::gravity::GravityWell::from_surface_gravity(
                    3.0,
                    20.0,
                    &GravitySettings::default(),
                ),
            ))
            .id()
    }

    /// A STRONG well, like the menu planetoid: surface gravity 6 at a ~85u
    /// geometric radius gives `mu ~= 43000`, so at an r=140 orbit the local
    /// gravity accel `mu/r^2 ~= 2.2 u/s^2` EXCEEDS `rcs_accel` (1.5). The RCS
    /// fine-adjust cannot counter that inward pull.
    fn spawn_strong_well(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                RigidBody::Static,
                Transform::default(),
                crate::gravity::GravityWell::from_surface_gravity(
                    6.0,
                    85.0,
                    &GravitySettings::default(),
                ),
            ))
            .id()
    }

    /// Regression for task 20260718-204640 (the two menu ambience ships crashed
    /// the asteroid and could not hold orbit). In a STRONG well - local gravity
    /// accel above the RCS accel - the error-relative ORBIT trim (task
    /// 20260718-151102) must NOT take over station-keeping: a 1.5 u/s^2 RCS
    /// cannot hold against a >1.5 u/s^2 inward pull, so handing it the orbit and
    /// zeroing the main drive spirals the ship in. The `use_rcs_orbit` gate now
    /// requires RCS to have clear authority over local gravity; here it does not,
    /// so the ship keeps the ring on the full-authority main drive and RCS stays
    /// idle. WITHOUT the gate (the un-fixed 20260718-151102 behavior) the radius
    /// collapses and this fails.
    #[test]
    fn strong_gravity_orbit_holds_the_ring_on_the_main_drive_not_rcs() {
        let mut app = orbit_app();
        let well = spawn_strong_well(&mut app);
        let (ship, _, _) = spawn_ship(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Transform::from_xyz(140.0, 0.0, 0.0));
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Orbit {
                well,
                plan: None,
            }));

        // Let the insertion settle, then watch a long hold.
        run(&mut app, 3000);
        let plan_radius = match app.world().get::<Autopilot>(ship) {
            Some(Autopilot {
                action:
                    AutopilotAction::Orbit {
                        plan: Some(plan), ..
                    },
                ..
            }) => plan.radius,
            other => panic!("ORBIT should stay engaged with a plan, got {other:?}"),
        };

        let mut r_min = f32::MAX;
        let mut saw_rcs = false;
        for _ in 0..5000 {
            app.update();
            r_min = r_min.min(position_of(&app, ship).length());
            if app
                .world()
                .get::<RcsIntent>(ship)
                .is_some_and(|i| i.0.length() > 1e-3)
            {
                saw_rcs = true;
            }
        }

        assert!(
            r_min > 0.6 * plan_radius,
            "the ship must hold the ring, not spiral into the rock (r_min {r_min}, plan {plan_radius})"
        );
        assert!(
            !saw_rcs,
            "in a strong well the orbit stays on the main drive - RCS lacks the authority, so it must not engage"
        );
    }

    /// ORBIT trims via the error-relative RCS (task 20260718-151102), but ONLY
    /// while the residual `|v - v_orbit|` is below the cap. From near-rest the
    /// desired is the full orbital velocity (~4.9 u/s at r=50, above the 2 u/s
    /// cap), so the main drive spins the orbit up and RCS stays idle; once the
    /// ship is near orbital velocity the residual drops sub-cap and RCS takes
    /// over the trim. The invariant that pins error-relative (not absolute)
    /// behavior: whenever RCS is trimming, its `RcsReference` is the fast orbital
    /// velocity (well above the cap) and `|v - reference|` is within the cap -
    /// impossible under the old absolute cap, which would have gated to zero.
    #[test]
    fn orbit_engages_rcs_only_to_trim_a_sub_cap_residual() {
        let mut app = orbit_app();
        let well = spawn_orbit_well(&mut app);
        let (ship, _, _) = spawn_ship(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Transform::from_xyz(50.0, 0.0, 0.0));
        settle(&mut app);
        // From rest the residual is the full orbital speed, above the cap, so
        // the first ticks must NOT engage RCS - the main drive spins up.
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Orbit {
                well,
                plan: None,
            }));
        for _ in 0..5 {
            app.update();
            let intent = app
                .world()
                .get::<RcsIntent>(ship)
                .map(|i| i.0.length())
                .unwrap_or(0.0);
            assert!(
                intent < 1e-3,
                "RCS must not trim while spinning up from rest (residual > cap), got {intent}"
            );
        }

        let cap = 2.0;
        let mut saw_trim = false;
        for _ in 0..1500 {
            app.update();
            let intent = app
                .world()
                .get::<RcsIntent>(ship)
                .map(|i| i.0)
                .unwrap_or(Vec3::ZERO);
            if intent.length() > 1e-3 {
                saw_trim = true;
                let reference = app
                    .world()
                    .get::<RcsReference>(ship)
                    .map(|r| r.0)
                    .unwrap_or(Vec3::ZERO);
                let v = velocity_of(&app, ship);
                assert!(
                    reference.length() > cap,
                    "the trim reference is the fast orbital velocity, above the cap (got {})",
                    reference.length()
                );
                assert!(
                    (v - reference).length() <= cap + 0.5,
                    "RCS only trims a sub-cap residual (|v - ref| = {}, cap {cap})",
                    (v - reference).length()
                );
            }
        }
        assert!(
            saw_trim,
            "ORBIT should engage the error-relative RCS once at orbital speed"
        );
        assert!(
            app.world().get::<Autopilot>(ship).is_some(),
            "orbit never self-completes"
        );
    }

    /// The error-relative primitive: a ship already moving FASTER than the cap
    /// can still be trimmed by a sub-cap delta when an `RcsReference` rebases the
    /// cap. At 5 u/s with a matching 5 u/s reference, a prograde nudge pushes
    /// (residual is zero, full headroom) and climbs until `v - reference` hits
    /// the cap. WITHOUT the reference the same command gates to zero - the plain
    /// absolute cap (2 u/s) is already exceeded. Deleting the reference term in
    /// rcs_burn_system collapses the two cases, failing the "pushed" assertion.
    #[test]
    fn rcs_relative_cap_trims_a_fast_moving_reference() {
        // With the reference: prograde trim acts despite |v| > cap.
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        settle(&mut app);
        app.world_mut().entity_mut(ship).insert((
            LinearVelocity(Vec3::new(5.0, 0.0, 0.0)),
            RcsReference(Vec3::new(5.0, 0.0, 0.0)),
            RcsIntent(Vec3::new(0.5, 0.0, 0.0)),
        ));
        run(&mut app, 300);
        let with_ref = velocity_of(&app, ship).x;
        assert!(
            with_ref > 5.1,
            "an error-relative trim pushes prograde past the reference despite |v| > cap (v.x = {with_ref})"
        );
        assert!(
            with_ref <= 5.0 + 2.0 + 0.3,
            "but only up to cap ABOVE the reference (5 + cap = 7), got {with_ref}"
        );

        // Without the reference: the same command at |v| > cap gates to zero.
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        settle(&mut app);
        app.world_mut().entity_mut(ship).insert((
            LinearVelocity(Vec3::new(5.0, 0.0, 0.0)),
            RcsIntent(Vec3::new(0.5, 0.0, 0.0)),
        ));
        run(&mut app, 300);
        let no_ref = velocity_of(&app, ship).x;
        assert!(
            no_ref < 5.05,
            "the plain absolute cap is already exceeded, so the prograde command does nothing (v.x = {no_ref})"
        );
    }

    /// The error-relative reference is cleared on disengage
    /// (`shared-primitive-clear-on-handoff`): an orbit leaves a fast `RcsReference`
    /// behind, and if it lingered it would silently rebase the player's next
    /// absolute-cap nudge. After the orbit disengages both the intent and the
    /// reference must be zero.
    #[test]
    fn orbit_rcs_reference_clears_on_disengage() {
        let mut app = orbit_app();
        let well = spawn_orbit_well(&mut app);
        let (ship, _, _) = spawn_ship(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Transform::from_xyz(50.0, 0.0, 0.0));
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Orbit {
                well,
                plan: None,
            }));
        // Fly until the trim is live (a non-zero reference is written).
        let mut got_reference = false;
        for _ in 0..1500 {
            app.update();
            if app
                .world()
                .get::<RcsReference>(ship)
                .is_some_and(|r| r.0.length() > 1e-3)
            {
                got_reference = true;
                break;
            }
        }
        assert!(
            got_reference,
            "the orbit trim should write a live RcsReference"
        );

        app.world_mut().entity_mut(ship).remove::<Autopilot>();
        run(&mut app, 3);
        let reference = app
            .world()
            .get::<RcsReference>(ship)
            .map(|r| r.0.length())
            .unwrap_or(0.0);
        let intent = app
            .world()
            .get::<RcsIntent>(ship)
            .map(|i| i.0.length())
            .unwrap_or(0.0);
        assert!(
            reference < 1e-3,
            "the reference is cleared on disengage (got {reference})"
        );
        assert!(
            intent < 1e-3,
            "the intent is cleared on disengage (got {intent})"
        );
    }

    /// A STOP settling from below the RCS cap hands the brake to the torque-free
    /// RCS primitive: `RcsIntent` goes non-zero, the main thruster stays cold,
    /// and the ship still reaches rest. Delete the RCS branch and the main drive
    /// brakes instead (thruster fires), failing the cold-drive assertion.
    #[test]
    fn stop_terminal_brakes_via_rcs() {
        let mut app = flight_app();
        let (ship, thruster, _controller) = spawn_ship(&mut app);
        settle(&mut app);
        // Below the cap, so RCS can act. STOP's goal is rest (desired == 0).
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::new(1.5, 0.0, 0.0)));
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));

        let mut saw_rcs = false;
        let mut max_thruster = 0.0f32;
        for _ in 0..600 {
            app.update();
            if let Some(intent) = app.world().get::<RcsIntent>(ship) {
                if intent.0.length() > 1e-3 {
                    saw_rcs = true;
                }
            }
            max_thruster =
                max_thruster.max(**app.world().get::<ThrusterSectionInput>(thruster).unwrap());
            if app.world().get::<Autopilot>(ship).is_none() {
                break;
            }
        }
        assert!(saw_rcs, "STOP's terminal drove RCS (non-zero RcsIntent)");
        assert!(
            max_thruster < 0.05,
            "the main drive stayed cold - RCS did the braking (max input {max_thruster})"
        );
        // Settles to WITHIN the autopilot's settle_deadband (0.75) - the same
        // "bounded creep is the contract" release the main drive gets. RCS
        // currently releases at the deadband rather than driving to
        // stop_speed_epsilon (the disengage reads no aligned main engine while
        // in RCS mode); tightening that terminal creep is a rework item (task
        // 20260718-151102).
        assert!(
            velocity_of(&app, ship).length() < 0.8,
            "STOP settled to within the deadband via RCS (v = {})",
            velocity_of(&app, ship).length()
        );
    }

    /// After an RCS-settled STOP disengages, the ship must STAY at rest: the
    /// autopilot's residual `RcsIntent` has to be cleared on disengage, or
    /// `rcs_burn_system` (which acts on any non-zero intent, autopilot or not)
    /// keeps pushing and the ship drifts off to the RCS cap. Runs PAST the
    /// disengage; fails if the on-remove clear is missing.
    #[test]
    fn rcs_settled_autopilot_leaves_the_ship_at_rest_after_disengage() {
        let mut app = flight_app();
        let (ship, _thruster, _controller) = spawn_ship(&mut app);
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::new(1.5, 0.0, 0.0)));
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));

        // Settle until the autopilot releases.
        let mut disengaged = false;
        for _ in 0..1200 {
            app.update();
            if app.world().get::<Autopilot>(ship).is_none() {
                disengaged = true;
                break;
            }
        }
        assert!(disengaged, "the STOP should self-complete");
        let at_release = velocity_of(&app, ship).length();

        // Coast well past release: a leftover RcsIntent would accelerate the
        // ship toward the cap here.
        for _ in 0..400 {
            app.update();
        }
        let after = velocity_of(&app, ship).length();
        assert!(
            after <= at_release + 0.05,
            "the ship must stay at rest after disengage, not drift on a residual \
             RcsIntent (v {at_release} -> {after})"
        );
    }

    /// Without the `Rcs` verb the autopilot must NOT write `RcsIntent`; the same
    /// STOP settles on the main drive instead (the mainline-campaign path while
    /// RCS is disabled pending rework).
    #[test]
    fn stop_terminal_without_rcs_verb_uses_the_main_drive() {
        let mut app = flight_app();
        let (ship, _thruster, controller) = spawn_ship(&mut app);
        app.world_mut()
            .entity_mut(controller)
            .insert(WithheldVerbs([FlightVerb::Rcs].into_iter().collect()));
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::new(1.5, 0.0, 0.0)));
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));

        for _ in 0..900 {
            app.update();
            if let Some(intent) = app.world().get::<RcsIntent>(ship) {
                assert!(
                    intent.0.length() < 1e-3,
                    "no Rcs verb: the autopilot must not write RcsIntent, got {:?}",
                    intent.0
                );
            }
            if app.world().get::<Autopilot>(ship).is_none() {
                break;
            }
        }
        assert!(
            velocity_of(&app, ship).length() < 0.8,
            "still settles to within the deadband on the main drive (v = {})",
            velocity_of(&app, ship).length()
        );
    }

    #[test]
    fn orbit_engages_from_near_rest_and_holds_the_ring_for_a_lap() {
        let mut app = orbit_app();
        let well = spawn_orbit_well(&mut app);
        let (ship, _, _) = spawn_ship(&mut app);
        // Park near-rest at r = 50, inside the stable band: the whole
        // insertion - plan, align, burn to tangential v_circ, hold - is the
        // computer's job.
        app.world_mut()
            .entity_mut(ship)
            .insert(Transform::from_xyz(50.0, 0.0, 0.0));
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Orbit {
                well,
                plan: None,
            }));

        // Insertion window, then a full ~64s lap under observation.
        run(&mut app, 900);
        let plan_radius = match app.world().get::<Autopilot>(ship) {
            Some(Autopilot {
                action:
                    AutopilotAction::Orbit {
                        plan: Some(plan), ..
                    },
                ..
            }) => plan.radius,
            other => panic!("ORBIT should stay engaged with a plan, got {other:?}"),
        };
        assert!(
            (plan_radius - 50.0).abs() < 1.0,
            "r = 50 is inside the band, the plan should keep it, got {plan_radius}"
        );

        let (mut r_min, mut r_max) = (f32::MAX, f32::MIN);
        let mut held = false;
        for _ in 0..4200 {
            app.update();
            let r = position_of(&app, ship).length();
            r_min = r_min.min(r);
            r_max = r_max.max(r);
            if app.world().get::<Autopilot>(ship).map(|ap| ap.phase) == Some(AutopilotPhase::Hold) {
                held = true;
            }
        }

        assert!(
            r_min > 0.8 * plan_radius && r_max < 1.25 * plan_radius,
            "orbit drifted out of the band: min {r_min}, max {r_max}, plan {plan_radius}"
        );
        assert!(held, "station-keeping should reach the Hold phase");
        let speed = velocity_of(&app, ship).length();
        let v_circ = crate::gravity::circular_orbit_speed(1200.0, plan_radius);
        assert!(
            (speed - v_circ).abs() < 0.35 * v_circ,
            "orbital speed should sit near v_circ {v_circ}, got {speed}"
        );
        assert!(
            app.world().get::<Autopilot>(ship).is_some(),
            "an orbit is not a destination: ORBIT never self-completes"
        );
    }

    #[test]
    fn orbit_disengages_when_the_well_dies() {
        let mut app = orbit_app();
        let well = spawn_orbit_well(&mut app);
        let (ship, _, _) = spawn_ship(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Transform::from_xyz(50.0, 0.0, 0.0));
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Orbit {
                well,
                plan: None,
            }));
        run(&mut app, 60);
        assert!(app.world().get::<Autopilot>(ship).is_some());

        app.world_mut().entity_mut(well).despawn();
        run(&mut app, 2);

        assert!(
            app.world().get::<Autopilot>(ship).is_none(),
            "a dead well disengages ORBIT, like a vanished GOTO target"
        );
    }

    #[test]
    fn orbit_inherits_the_capability_coupling() {
        // Dead engines: the computer cannot circularize, ORBIT disengages -
        // same rule as STOP/GOTO.
        let mut app = orbit_app();
        let well = spawn_orbit_well(&mut app);
        let (ship, thruster, _) = spawn_ship(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Transform::from_xyz(50.0, 0.0, 0.0));
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Orbit {
                well,
                plan: None,
            }));
        run(&mut app, 30);
        app.world_mut()
            .entity_mut(thruster)
            .insert(SectionInactiveMarker);
        run(&mut app, 2);
        assert!(
            app.world().get::<Autopilot>(ship).is_none(),
            "no live engines, no ORBIT"
        );

        // Dead flight computer: same, one level earlier.
        let mut app = orbit_app();
        let well = spawn_orbit_well(&mut app);
        let (ship, _, controller) = spawn_ship(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Transform::from_xyz(50.0, 0.0, 0.0));
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Orbit {
                well,
                plan: None,
            }));
        run(&mut app, 30);
        app.world_mut()
            .entity_mut(controller)
            .insert(SectionInactiveMarker);
        run(&mut app, 2);
        assert!(
            app.world().get::<Autopilot>(ship).is_none(),
            "no live flight computer, no ORBIT"
        );
    }

    // --- Maneuver telemetry -------------------------------------------------

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

    // --- Gravity-aware arrival (spike docs/spikes/20260710-204802) ---------

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
        // braking-regime fallback (R1.1 - the instruments must go blank).
        assert_eq!(arrival_eta(300.0, 12.0, 10.0, 2.0, 50.0, 10.0), None);
        assert_eq!(arrival_eta(55.0, 12.0, 10.0, 2.0, 50.0, 10.0), None);
    }

    #[test]
    fn stop_publishes_its_rest_point_and_settling_clears_it() {
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::new(6.0, 0.0, 0.0)));
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));
        run(&mut app, 5);

        let telemetry = app
            .world()
            .get::<ManeuverTelemetry>(ship)
            .expect("a moving STOP publishes telemetry");
        // The rest point lies ahead along the velocity.
        assert!(
            (telemetry.goal - position_of(&app, ship)).dot(Vec3::X) > 0.0,
            "rest point ahead of the drift"
        );
        assert!(telemetry.eta.expect("eta while braking") > 0.0);

        // The maneuver completes: velocity nulled, autopilot gone, and the
        // telemetry with it (observer path).
        run(&mut app, 900);
        assert!(app.world().get::<Autopilot>(ship).is_none());
        assert!(app.world().get::<ManeuverTelemetry>(ship).is_none());
    }

    #[test]
    fn goto_publishes_telemetry_and_disengaging_clears_it() {
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        settle(&mut app);
        let goal = Vec3::new(0.0, 0.0, -300.0);
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::GotoPos {
                position: goal,
            }));

        // Sample early in the burn, while the flip (which now includes the
        // rotation lead, so it comes sooner) is still ahead.
        run(&mut app, 120);
        let telemetry = app
            .world()
            .get::<ManeuverTelemetry>(ship)
            .expect("an engaged GOTO publishes telemetry");
        assert_eq!(telemetry.goal, goal);
        assert_eq!(telemetry.goal_entity, None, "GotoPos tracks no entity");
        assert!(telemetry.closing_speed > 0.5, "the ship closes on the goal");
        let flip = telemetry.flip_point.expect("flip ahead while coasting");
        // The flip point sits on the segment between ship and goal.
        let ship_position = position_of(&app, ship);
        let along = (flip - ship_position).dot(goal - ship_position);
        assert!(along > 0.0, "flip is ahead of the ship");
        assert!(
            flip.distance(goal) < ship_position.distance(goal),
            "flip is short of the goal"
        );
        assert!(telemetry.eta.expect("eta while closing") > 0.0);
        // The park point sits exactly one standoff short of the goal on
        // the closing line (GotoPos has no target radius).
        let standoff = app.world().resource::<FlightSettings>().arrival_standoff;
        assert!(
            (telemetry.park_point.distance(goal) - standoff).abs() < 1e-3,
            "the park point is one standoff short of the goal, got {}",
            telemetry.park_point.distance(goal)
        );
        assert!(
            (goal - telemetry.park_point)
                .normalize()
                .dot((goal - ship_position).normalize())
                > 0.999,
            "the park point lies on the closing line"
        );

        // Switching verbs (insert-overwrite: OnRemove does NOT fire, the
        // in-system path must carry it) republishes for the new leg: a
        // moving ship on STOP reports its predicted rest point, with no
        // flip (the retrograde alignment is inside the lead window).
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));
        run(&mut app, 2);
        let stop_telemetry = app
            .world()
            .get::<ManeuverTelemetry>(ship)
            .expect("a moving STOP leg publishes its rest point");
        assert_ne!(stop_telemetry.goal, goal, "the goal is now the rest point");
        assert_eq!(stop_telemetry.flip_point, None);
        assert_eq!(
            stop_telemetry.park_point, stop_telemetry.goal,
            "a STOP has no standoff: the rest point is the park point"
        );

        // Breakout clears the numbers with the maneuver.
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::GotoPos {
                position: goal,
            }));
        run(&mut app, 60);
        assert!(app.world().get::<ManeuverTelemetry>(ship).is_some());
        app.world_mut().entity_mut(ship).remove::<Autopilot>();
        run(&mut app, 2);
        assert!(
            app.world().get::<ManeuverTelemetry>(ship).is_none(),
            "telemetry dies with the leg"
        );
    }

    use crate::test_log::CapturedLog;

    /// The 2026-07-12 playtest warn (task 20260712-115902): scenario teardown
    /// despawns a ship with an engaged autopilot, `On<Remove, Autopilot>`
    /// fires mid-flush, and the remove it queues lands after the despawn in
    /// the same queue - "Encountered an error in command ... Entity
    /// despawned". The test drives that exact path (a QUEUED despawn; a
    /// direct `World::despawn` does not reproduce it - the entity is already
    /// gone at observer-queue time, `get_entity` bails, nothing is queued)
    /// and asserts the warn does not fire, with two delivery guards: the
    /// observer's command demonstrably lands on a live disengage, and the
    /// log capture demonstrably sees this exact warn class.
    #[test]
    fn despawning_an_autopiloting_ship_queues_no_stale_telemetry_command() {
        use bevy::log::tracing_subscriber::{self, util::SubscriberInitExt};

        let log = CapturedLog::default();
        let writer = log.clone();
        let _guard = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .set_default();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_observer(remove_maneuver_telemetry);

        let telemetry = ManeuverTelemetry {
            goal: Vec3::new(0.0, 0.0, -300.0),
            goal_entity: None,
            park_point: Vec3::new(0.0, 0.0, -290.0),
            distance: 300.0,
            closing_speed: 5.0,
            brake_accel: 1.0,
            flip_point: None,
            seconds_to_flip: None,
            eta: Some(60.0),
        };

        // Delivery guard 1: the capture sees exactly this warn class - a
        // deliberately stale plain `remove` must log "Entity despawned".
        let stale = app.world_mut().spawn_empty().id();
        app.world_mut().entity_mut(stale).despawn();
        app.world_mut()
            .commands()
            .entity(stale)
            .remove::<ManeuverTelemetry>();
        app.update();
        assert!(
            log.contents().contains("Entity despawned"),
            "the log capture must see a deliberate stale-command warn; got: {}",
            log.contents()
        );
        log.clear();

        // Delivery guard 2: on a LIVE ship the observer fires and its queued
        // command really lands.
        let live = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                Autopilot::engage(AutopilotAction::Stop),
                telemetry,
            ))
            .id();
        app.update();
        app.world_mut().entity_mut(live).remove::<Autopilot>();
        app.update();
        assert!(
            app.world().get::<ManeuverTelemetry>(live).is_none(),
            "the observer clears telemetry on a live disengage"
        );

        // The race: despawn the ship WITH the autopilot engaged, through a
        // QUEUED despawn, the way the unload sweep does. Pre-fix the
        // observer's remove lands on the despawned ship and warns.
        let doomed = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                Autopilot::engage(AutopilotAction::Stop),
                telemetry,
            ))
            .id();
        app.update();
        log.clear();
        app.world_mut().commands().entity(doomed).despawn();
        app.update();
        assert!(
            !log.contents().contains("Entity despawned"),
            "teardown must not race a stale telemetry remove; got: {}",
            log.contents()
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

    #[test]
    fn goto_pos_arrives_at_standoff_and_disengages() {
        // The position-goal twin of the entity GOTO (the AI patrol leg,
        // task 20260709-225730): same arrival rule, no entity to track.
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        settle(&mut app);
        let destination = Vec3::new(0.0, 0.0, -300.0);
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::GotoPos {
                position: destination,
            }));

        let mut released = false;
        for _ in 0..4800 {
            app.update();
            if app.world().get::<Autopilot>(ship).is_none() {
                released = true;
                break;
            }
        }
        assert!(released, "GotoPos must complete and disengage in budget");

        let standoff = app.world().resource::<FlightSettings>().arrival_standoff;
        let pos = app.world().get::<Position>(ship).unwrap().0;
        let distance = (destination - pos).length();
        let speed = velocity_of(&app, ship).length();
        assert!(
            distance <= standoff + 6.0 && distance >= standoff - 45.0,
            "should arrive near the {standoff}u standoff, got {distance}"
        );
        assert!(speed < 0.5, "should arrive at rest, got {speed}");
    }

    #[test]
    fn goto_into_a_well_stops_at_the_standoff_instead_of_crashing() {
        // The playtest crash (task 20260710-193500): GOTO a well body from
        // outside the SOI at speed. A gravity-blind plan flips on the
        // vacuum curve, the well keeps feeding speed through the descent,
        // and the ship punches through the standoff into the surface. The
        // gravity-aware plan must keep the hull outside the body the whole
        // way and still park at the standoff.
        let mut app = orbit_app();
        let gravity = GravitySettings::default();
        // The strongest well the guardrail allows: surface pull 5 u/s^2 on
        // a 40u body (mu = 8000, SOI 320u).
        let well = app
            .world_mut()
            .spawn((
                RigidBody::Static,
                Transform::default(),
                crate::gravity::GravityWell::from_surface_gravity(5.0, 40.0, &gravity),
            ))
            .id();
        let (ship, _, _) = spawn_ship(&mut app);
        withhold_rcs(&mut app, ship);
        app.world_mut()
            .entity_mut(ship)
            .insert(Transform::from_xyz(0.0, 0.0, 500.0));
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::new(0.0, 0.0, -25.0)));
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Goto { target: well }));

        let body_radius = 40.0;
        let mut min_distance = f32::MAX;
        let mut parked = false;
        for _ in 0..6000 {
            app.update();
            let distance = app.world().get::<Position>(ship).unwrap().0.length();
            min_distance = min_distance.min(distance);
            match app.world().get::<Autopilot>(ship) {
                // A GOTO at a well body hands off to ORBIT (task
                // 20260710-195954) instead of releasing.
                Some(autopilot) if matches!(autopilot.action, AutopilotAction::Orbit { .. }) => {
                    parked = true;
                    break;
                }
                Some(_) => {}
                None => panic!("a GOTO at a well body must park, not release"),
            }
        }
        assert!(parked, "GOTO must arrive and hand off to ORBIT in budget");
        // The done gate structurally guarantees the handoff residual is
        // bounded by the settle band (task 20260711-140234: sub-band crumbs
        // are accepted rather than hunted); assert that contract to keep
        // the old arrival-curve check.
        let handoff_speed = velocity_of(&app, ship).length();
        let settle_band = app.world().resource::<FlightSettings>().settle_deadband;
        assert!(
            handoff_speed < settle_band + 0.05,
            "the handoff happens within the settle band ({settle_band}), got {handoff_speed}"
        );
        assert!(
            min_distance > body_radius + gravity.surface_margin,
            "the hull must never dip below the surface, got {min_distance}"
        );
        // The handoff happens at the surface-relative park point (task
        // 20260710-202408): standoff + body_radius from the center, with
        // the flat-space tests' terminal-creep lower bound.
        let standoff = app.world().resource::<FlightSettings>().arrival_standoff;
        let park = standoff + body_radius;
        let distance = app.world().get::<Position>(ship).unwrap().0.length();
        assert!(
            distance <= park + 6.0 && distance >= park - 45.0,
            "should hand off near {park}u from the center ({standoff}u above the \
             surface), got {distance}"
        );

        // ORBIT never completes: the computer station-keeps. Run on and
        // require the ship to stay engaged and above the surface while
        // the insertion pulls it onto the ring.
        for _ in 0..1200 {
            app.update();
            let distance = app.world().get::<Position>(ship).unwrap().0.length();
            assert!(
                distance > body_radius + gravity.surface_margin,
                "the parked orbit must keep the hull above the surface, got {distance}"
            );
        }
        let autopilot = app
            .world()
            .get::<Autopilot>(ship)
            .expect("ORBIT station-keeps; it never completes");
        let AutopilotAction::Orbit {
            plan: Some(plan), ..
        } = autopilot.action
        else {
            panic!("the parked autopilot flies a planned orbit");
        };
        // The insertion actually holds: after the settle window the ship
        // is on (or tight around) the planned ring, not slowly decaying
        // past it.
        let radius = app.world().get::<Position>(ship).unwrap().0.length();
        assert!(
            (radius - plan.radius).abs() < 15.0,
            "the ship should ride the {}u ring, got {radius}",
            plan.radius
        );
    }

    #[test]
    fn goto_standoff_is_surface_relative_for_sized_targets() {
        // A big rock WITHOUT a well: the authored BodyRadius alone must
        // push the park point out, and the published telemetry distance
        // must read to the surface, not the center (the chip should never
        // say "50" while hovering over a mountain).
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        let center = Vec3::new(0.0, 0.0, -300.0);
        let target = app
            .world_mut()
            .spawn((
                Transform::from_translation(center),
                GlobalTransform::from(Transform::from_translation(center)),
                BodyRadius(30.0),
            ))
            .id();
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Goto { target }));

        // Mid-leg the telemetry distance is surface-relative: center
        // distance minus the radius.
        app.update();
        let telemetry = app
            .world()
            .get::<ManeuverTelemetry>(ship)
            .expect("engaged GOTO publishes telemetry");
        let center_distance = (center - app.world().get::<Position>(ship).unwrap().0).length();
        assert!(
            (telemetry.distance - (center_distance - 30.0)).abs() < 1.0,
            "telemetry reads to the surface: got {} for center distance {center_distance}",
            telemetry.distance
        );
        // The published park point budgets the radius too: standoff plus
        // radius from the center, on the closing line (task 20260710-214316,
        // the ribbon terminates here).
        let standoff = app.world().resource::<FlightSettings>().arrival_standoff;
        assert!(
            (telemetry.park_point.distance(center) - (standoff + 30.0)).abs() < 1e-2,
            "the park point sits standoff + radius from the center, got {}",
            telemetry.park_point.distance(center)
        );

        let mut released = false;
        // Once inside the park envelope the park point degenerates to the
        // ship itself: the computer stops where it is, it never plans a
        // leg back out to the boundary - and the ribbon must not draw one.
        let mut inside_sample: Option<(Vec3, Vec3)> = None;
        for _ in 0..4800 {
            app.update();
            if let Some(numbers) = app.world().get::<ManeuverTelemetry>(ship) {
                if inside_sample.is_none() && numbers.distance <= standoff {
                    inside_sample = Some((
                        numbers.park_point,
                        app.world().get::<Position>(ship).unwrap().0,
                    ));
                }
            }
            if app.world().get::<Autopilot>(ship).is_none() {
                released = true;
                break;
            }
        }
        assert!(released, "GOTO must complete and disengage in budget");
        let (inside_park, inside_position) =
            inside_sample.expect("the leg passes through the park envelope before release");
        assert!(
            inside_park.distance(inside_position) < 2.0,
            "inside the envelope the park point pins to the ship, got {}u away",
            inside_park.distance(inside_position)
        );

        let park = standoff + 30.0;
        let distance = (center - app.world().get::<Position>(ship).unwrap().0).length();
        let speed = velocity_of(&app, ship).length();
        assert!(
            distance <= park + 6.0 && distance >= park - 45.0,
            "should park near {park}u from the center, got {distance}"
        );
        assert!(speed < 0.5, "should arrive at rest, got {speed}");
    }

    #[test]
    fn handoff_ring_clears_the_geometric_radius() {
        // A well whose real collider (BodyRadius 70) reaches far past its
        // nominal physics radius (40): the parking handoff must ring at
        // the GEOMETRIC park radius (70 + 50 = 120), not clamp the crept
        // position against a band floored on the nominal sphere - that
        // ring could sit inside the actual rock.
        let mut app = orbit_app();
        let gravity = GravitySettings::default();
        let well = app
            .world_mut()
            .spawn((
                RigidBody::Static,
                Transform::default(),
                crate::gravity::GravityWell::from_surface_gravity(5.0, 40.0, &gravity),
                BodyRadius(70.0),
            ))
            .id();
        let (ship, _, _) = spawn_ship(&mut app);
        // At rest inside the park envelope: surface distance 115 - 70 =
        // 45 <= the 50u standoff, so the leg is immediately done and the
        // handoff fires.
        app.world_mut()
            .entity_mut(ship)
            .insert(Transform::from_xyz(0.0, 0.0, 115.0));
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Goto { target: well }));

        let mut plan_radius = None;
        for _ in 0..600 {
            app.update();
            match app.world().get::<Autopilot>(ship) {
                Some(autopilot) => {
                    if let AutopilotAction::Orbit {
                        plan: Some(plan), ..
                    } = autopilot.action
                    {
                        plan_radius = Some(plan.radius);
                        break;
                    }
                }
                None => panic!("a GOTO at a well body must park, not release"),
            }
        }
        let radius = plan_radius.expect("handoff in budget");
        assert!(
            (radius - 120.0).abs() < 2.0,
            "ring at the geometric park radius, got {radius}"
        );
    }

    #[test]
    fn goto_radius_resolution_prefers_the_larger_source() {
        // A target carrying BOTH an authored BodyRadius and a well whose
        // body_radius disagrees: the arrival must budget the larger of
        // the two (conservative if they ever drift apart).
        let mut app = flight_app();
        let (ship, _, _) = spawn_ship(&mut app);
        let center = Vec3::new(0.0, 0.0, -300.0);
        let gravity = GravitySettings::default();
        let target = app
            .world_mut()
            .spawn((
                Transform::from_translation(center),
                GlobalTransform::from(Transform::from_translation(center)),
                Position(center),
                BodyRadius(20.0),
                crate::gravity::GravityWell::from_surface_gravity(3.0, 40.0, &gravity),
            ))
            .id();
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Goto { target }));

        app.update();
        let telemetry = app
            .world()
            .get::<ManeuverTelemetry>(ship)
            .expect("engaged GOTO publishes telemetry");
        let center_distance = (center - app.world().get::<Position>(ship).unwrap().0).length();
        assert!(
            (telemetry.distance - (center_distance - 40.0)).abs() < 1.0,
            "the larger well radius wins over BodyRadius(20): got {} for center distance \
             {center_distance}",
            telemetry.distance
        );
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
            ..default()
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
                    ..default()
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
                    ..default()
                }),
                Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
            ))
            .id();
        settle(&mut app);
        withhold_rcs(&mut app, ship);

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
        withhold_rcs(&mut app, ship);
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
        // wobble hit 2+ rad/s DURING the burn), and since bcs's inertia
        // frame composition fix (bcs task 20260711-091519, nova task
        // 20260709-125640) the release parks the hull too - the old
        // ~1.5 rad/s corkscrew came from the mangled tensor (avian's eigen
        // sort hands even this axis-aligned ship a cyclic-permutation
        // local frame, which the pre-fix order composed wrongly).
        run(&mut app, 300);
        let spin = app
            .world()
            .get::<AngularVelocity>(ship)
            .map(|w| w.length())
            .unwrap_or(f32::NAN);
        assert!(
            spin < 0.5,
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
        withhold_rcs(&mut app, ship);
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
        withhold_rcs(&mut app, ship);
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
        withhold_rcs(&mut app, ship);
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
        withhold_rcs(&mut app, ship);
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
        withhold_rcs(&mut app, ship);
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
        withhold_rcs(&mut app, ship);
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
        withhold_rcs(&mut app, ship);
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

    /// Shipped 5-section player geometry (same as
    /// `hold_reverse_decel_from_300_keeps_the_hull_steady`).
    fn diag_ship(app: &mut App) -> (Entity, Entity) {
        let ship = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                TransformInterpolation,
                SpaceshipRootMarker,
                FlightIntent::default(),
            ))
            .id();
        let section = |app: &mut App, name: &str, z: f32| {
            app.world_mut()
                .spawn((
                    ChildOf(ship),
                    Name::new(name.to_string()),
                    Transform::from_xyz(0.0, 0.0, z),
                    Collider::cuboid(1.0, 1.0, 1.0),
                    ColliderDensity(1.0),
                ))
                .id()
        };
        let controller = section(app, "controller", 0.0);
        app.world_mut().entity_mut(controller).insert((
            ControllerSectionMarker,
            ControllerSectionRotationInput::default(),
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_torque: 40.0,
            },
            PDControllerTarget(ship),
        ));
        section(app, "hull_front", 1.0);
        section(app, "hull_back", -1.0);
        let thruster = section(app, "thruster", 2.0);
        app.world_mut().entity_mut(thruster).insert((
            ThrusterSectionMarker,
            ThrusterSectionMagnitude(1.0),
            ThrusterSectionInput(0.0),
        ));
        section(app, "turret_mass", -2.0);
        settle(app);
        (ship, controller)
    }

    /// A GOTO leg must ARRIVE quietly (task 20260711-140234). The feel
    /// spike (docs/spikes/20260711-140234-feel-filtering.md) traced the
    /// playtest "wobbles on GOTO" to a terminal attitude hunt: the
    /// endgame of a translation leg lives in sub-u/s velocity errors, and
    /// with the tight crumb band (attitude_deadband 0.4) plus its 8x
    /// urgency denominator the computer chased them with visible attitude
    /// swings (~0.6 rad/s) for seconds at every arrival - while STOP,
    /// whose error passes the band exactly once nose-on, settled
    /// perfectly. With the settle band (and the urgency denominator it
    /// carries) scoped to rest legs, the terminal phase stays under
    /// 0.15 rad/s and the hull releases essentially still. A/B: the
    /// pre-fix config fails this at ~0.6 rad/s terminal spin.
    ///
    /// Wiring history: the arrival dynamics are wiring-SENSITIVE. Under the
    /// pre-20260711-140241 Update-schedule command copy, the doorstep
    /// brake's spool-tail overshoot happened to land under the settle band
    /// (accidental dither); the same-tick handoff phase-locked it into a
    /// boundary-bounce limit cycle until the spool-tail cutoff in
    /// autopilot_system removed the overshoot at its source. This rig runs
    /// the harness wiring (same-tick copy), which matches production since
    /// 20260711-140241 moved the shipped copy to FixedUpdate - so a hunt
    /// reappearing under EITHER the cutoff regressing or the wiring
    /// changing fails here.
    #[test]
    fn goto_arrival_settles_without_hunting() {
        let mut app = flight_app();
        let (ship, _controller) = diag_ship(&mut app);
        let goal = Vec3::new(300.0, 0.0, -600.0);
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::GotoPos {
                position: goal,
            }));
        let standoff = app.world().resource::<FlightSettings>().arrival_standoff;

        let mut max_spin_overall = 0.0f32;
        let mut max_spin_terminal = 0.0f32;
        let mut min_remaining = f32::MAX;
        let mut done = false;
        for _ in 0..4000 {
            app.update();
            let spin = app.world().get::<AngularVelocity>(ship).unwrap().length();
            let remaining = (goal - position_of(&app, ship)).length() - standoff;
            max_spin_overall = max_spin_overall.max(spin);
            min_remaining = min_remaining.min(remaining);
            if remaining < 15.0 {
                max_spin_terminal = max_spin_terminal.max(spin);
            }
            if app.world().get::<Autopilot>(ship).is_none() {
                done = true;
                break;
            }
        }

        // Delivery guards: the maneuver must actually have flown - a leg
        // that never engages, never flips, or never reaches the envelope
        // would pass the quiet-arrival bounds vacuously.
        assert!(done, "the GotoPos leg must complete and release in budget");
        assert!(
            min_remaining < 1.0,
            "the ship must actually reach the park envelope, got to {min_remaining}"
        );
        assert!(
            max_spin_overall > 0.5,
            "a real flip-and-burn must have happened (max spin {max_spin_overall})"
        );

        let release_spin = app.world().get::<AngularVelocity>(ship).unwrap().length();
        assert!(
            max_spin_terminal < 0.15,
            "the arrival must not hunt: terminal max spin {max_spin_terminal} rad/s"
        );
        assert!(
            release_spin < 0.1,
            "the hull must release still, not mid-swing: {release_spin} rad/s"
        );
    }

    /// The rotation command must reach the PD on the tick it was written
    /// (task 20260711-140241). The copy from ControllerSectionRotationInput
    /// into the bcs PDControllerInput used to run in the Update schedule
    /// while both its producer (autopilot, FixedUpdate) and consumer (PD,
    /// PDControllerSystems::Sync in FixedUpdate) tick on the fixed clock -
    /// so the PD chased a command 1-2 ticks stale, varying with the
    /// 64 Hz-vs-render beat, and fought up to 0.22 rad of phantom command
    /// error during fast slews (~20% wasted torque). This rig runs the REAL
    /// plugins (NovaFlightPlugin + ControllerSectionPlugin), so it pins the
    /// SHIPPED wiring, not a hand-wired copy of it: a probe inside
    /// FixedUpdate, after PDControllerSystems::Sync, asserts the PD
    /// consumed exactly the command the autopilot wrote this tick, on
    /// every tick of a leg with an active slew. A/B: the Update-schedule
    /// copy fails at 0.22 rad.
    #[test]
    fn autopilot_command_reaches_the_pd_on_the_same_tick() {
        #[derive(Resource, Default)]
        struct StaleTrace {
            max_angle: f32,
            max_cmd_step: f32,
            samples: usize,
        }

        fn stale_probe(
            mut trace: ResMut<StaleTrace>,
            mut prev: Local<Option<Quat>>,
            q_controller: Query<
                (&PDControllerInput, &ControllerSectionRotationInput),
                With<ControllerSectionMarker>,
            >,
        ) {
            for (pd_input, command) in &q_controller {
                trace.max_angle = trace.max_angle.max(pd_input.angle_between(**command));
                trace.samples += 1;
                if let Some(prev) = *prev {
                    trace.max_cmd_step = trace.max_cmd_step.max(command.angle_between(prev));
                }
                *prev = Some(**command);
            }
        }

        let mut app = unfinished_integrity_physics_app();
        app.add_plugins(PDControllerPlugin);
        app.add_plugins(NovaFlightPlugin);
        app.add_plugins(crate::prelude::ControllerSectionPlugin { render: false });
        app.init_resource::<StaleTrace>();
        // The thruster plugin carries render-material deps, so the impulse
        // system is registered directly, as the flight harness does.
        app.add_systems(
            FixedUpdate,
            thruster_impulse_system.in_set(SpaceshipSectionSystems),
        );
        app.add_systems(FixedUpdate, stale_probe.after(PDControllerSystems::Sync));
        app.finish();

        let (ship, _controller) = diag_ship(&mut app);
        // 30 deg off the nose: the align phase slews the command every
        // tick, which is exactly when staleness shows.
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::GotoPos {
                position: Vec3::new(300.0, 0.0, -600.0),
            }));
        for _ in 0..120 {
            app.update();
        }

        let trace = app.world().resource::<StaleTrace>();
        // Delivery guards: the probe must have sampled real ticks and the
        // command must actually have been slewing - a parked command is
        // stale-proof by construction and would prove nothing.
        assert!(trace.samples > 100, "probe sampled {} ticks", trace.samples);
        assert!(
            trace.max_cmd_step > 5e-3,
            "the command must actually slew during the align phase \
             (max step {})",
            trace.max_cmd_step
        );
        // Bound sits above f32 Quat::angle_between noise (acos of a dot
        // near 1.0 floors around 1e-3 for identical rotations) and an
        // order of magnitude below the smallest stale-wiring reading
        // (0.048 in this rig; 0.22 during a full flip).
        assert!(
            trace.max_angle < 5e-3,
            "the PD must consume the command written THIS tick; max phantom \
             error {} rad",
            trace.max_angle
        );
    }
}
