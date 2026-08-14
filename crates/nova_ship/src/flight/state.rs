//! Flight state: the components an engaged maneuver rides on, the
//! telemetry it publishes, and the reflected tunables every flight system
//! reads.

use bevy::prelude::*;

/// The geometric radius of a scenario object, world units: the surface the GOTO
/// arrival standoff measures from and the orbit band's clearance floor clears
/// (the "stops too close" playtest). Derived from the actual generated collider
/// where one exists (asteroids: the noise-displaced mesh's outermost vertex,
/// which can reach well past the nominal designation radius) rather than
/// authored by hand. Unsized targets fall back to zero (center-relative, the
/// pre-existing behavior, fine for ships and debris). Well bodies are also
/// covered by [`GravityWell::body_radius`](nova_gameplay::gravity::GravityWell) (the
/// nominal physics radius); the arrival and the band take the larger of the two
/// when both exist.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct BodyRadius(pub f32);

/// The pilot's manual input, on the ship root. Written by the player input
/// layer; consumed by `manual_burn_system` when no autopilot is engaged.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
pub struct FlightIntent {
    /// Analog main-drive burn, `0..1` (W / Space / right trigger).
    pub burn: f32,
}

/// Soft cap (u/s) on the MANUAL main-drive burn, on the ship root:
/// scenario-authored for ships whose pilot should not be able to sail off into
/// the void (the shakedown starter ship). `manual_burn_system` tapers the
/// commanded burn to zero as the velocity component along the burn direction
/// approaches the cap - a held W levels off instead of accelerating forever.
/// Deliberately narrow: only the manual burn reads it (the autopilot plans its
/// own decel), only the along-burn component counts (turning and retro-braking
/// are never blocked), and ships without the component keep unbounded Newtonian
/// burn.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct FlightSpeedCap(pub f32);

/// Per-ship override (world units) of [`FlightSettings::arrival_standoff`]
/// for translation legs, on the ship root: how far from a GOTO/GotoPos goal
/// this ship's computer comes to rest. Scenario-authored for ships that must
/// visibly REACH their marks (a nav drill parking on its beacons) instead of
/// stopping the default 50 u short. Narrow like the speed cap: only the
/// GOTO/GotoPos arrival rule reads it - the ORBIT park and every global
/// tuning stay on [`FlightSettings`] - and ships without the component keep
/// the default.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct FlightArrivalStandoff(pub f32);

/// The pilot's (or autopilot's) RCS fine-adjustment command, on the ship root:
/// a desired translation direction in the ship's LOCAL frame, each component
/// roughly `-1..1` (the magnitude is how hard the nudge). Written by the player
/// input layer while RCS is held or by the autopilot; consumed by
/// `rcs_burn_system`. Zero (or absent) = no RCS. This is the shared primitive
/// both drivers write, so RCS never grew its own force path.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct RcsIntent(pub Vec3);

/// Present on the player ship while the pilot is HOLDING the RCS fine-adjust
/// modifier (SHIFT), inserted/removed by the input layer. It is the modal gate
/// the rest of the flight/camera/input stack reads, exactly as [`Autopilot`]
/// presence gates manual rotation: while it is present the mouse is repurposed
/// from aiming to translation (`RcsIntent` accumulation), and both the helm and
/// the camera rig stop consuming the mouse so the heading and view hold steady.
/// Not written by the autopilot - the autopilot drives `RcsIntent` directly, no
/// modal state.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct RcsActive;

/// Per-ship override of the RCS fine-adjust speed cap (u/s), on the ship root.
/// Unlike [`FlightSpeedCap`], RCS is ALWAYS capped - that is the whole point of
/// a fine-adjust mode - so a ship without this component still gets the default
/// [`FlightSettings::rcs_speed_cap`]; the component only lets a scenario tune
/// the ceiling per hull. `rcs_burn_system` gates each ship-local axis on the
/// along-axis velocity component just like the main-burn taper.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct RcsSpeedCap(pub f32);

/// World-frame REFERENCE velocity the RCS cap is measured against, on the ship
/// root. `rcs_burn_system` caps the along-axis component of `velocity -
/// reference`, not of the absolute velocity - so RCS can trim a fast-moving
/// craft by a sub-cap delta relative to this reference. ABSENT or ZERO restores
/// the plain absolute cap (`reference = 0`), which is exactly the player
/// fine-adjust mode and the STOP/GOTO terminal settle - both leave this unset.
/// The autopilot writes it to the desired ORBITAL velocity while
/// station-keeping, so a small prograde/retrograde correction trims the
/// orbit instead of gating to zero (the absolute cap would fight the ~2.5-6 u/s
/// orbital speed). Cleared to zero on autopilot disengage so a stale reference
/// never leaks into the player's absolute-cap mode.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct RcsReference(pub Vec3);

/// An engaged autopilot maneuver, on the ship root. Present = engaged; the
/// input layer inserts it (X = STOP, G = GOTO the lock) and removes it on any
/// flight input, so manual authority is simply "this component is absent".
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
pub struct Autopilot {
    /// What the computer is trying to do.
    pub action: AutopilotAction,
    /// Where the maneuver currently is (for the HUD); updated every tick by
    /// `autopilot_system`.
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
    /// drifting target is tracked; there is no collision avoidance.
    Goto {
        /// The destination entity (the aim-assist lock at engage time).
        target: Entity,
    },
    /// Fly to a fixed world position and come to rest at
    /// [`FlightSettings::arrival_standoff`] from it - the same arrival rule as
    /// `Goto`, just without an entity to track. The AI patrol loop flies its
    /// waypoints with this; the player input layer never engages it.
    GotoPos {
        /// The destination, world coordinates.
        position: Vec3,
    },
    /// Circularize and station-keep inside `well`'s gravity well.
    /// The first engaged tick is the Plan phase: `autopilot_system` picks
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
    /// the stable band (`orbit_target_radius`).
    pub radius: f32,
    /// Orbit plane unit normal; travel direction on the ring is
    /// `normal x radial`.
    pub normal: Vec3,
}

/// Live numbers for an engaged translation leg (GOTO/GotoPos toward a goal,
/// STOP toward its predicted rest point), published on the ship by
/// `autopilot_system` every tick and removed when the leg ends (verb switch
/// clears it in-system; disengage clears it via `remove_maneuver_telemetry`).
/// This is the physics-side seam the HUD instruments read - the arrival-rule
/// internals (brake authority, rotation lead) stay in the autopilot, the
/// readouts stay dumb.
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
    /// Where the leg comes to rest, world coordinates: `goal` pulled back along
    /// the closing line by the effective standoff
    /// ([`FlightSettings::arrival_standoff`] plus the resolved target radius).
    /// At or inside the park envelope it degenerates to the ship's own
    /// position - the computer will not fly back out, and the instruments must
    /// not draw a leg it will not fly. Equals `goal` for STOP (the predicted
    /// rest point IS the park point). The trajectory ribbon terminates here,
    /// not at the goal center.
    pub park_point: Vec3,
    /// Distance to the goal SURFACE, world units: the center distance minus the
    /// target's resolved radius ([`BodyRadius`] / `GravityWell::body_radius`,
    /// zero for unsized targets and GotoPos), so the readout never says "50"
    /// while hovering over a mountain. Clamped at zero - at or inside the
    /// surface reads 0, never a negative number on the chip.
    pub distance: f32,
    /// Speed along the line to the goal, u/s (negative = opening).
    pub closing_speed: f32,
    /// The EFFECTIVE deceleration the arrival plan brakes with, u/s^2: margin
    /// applied, then reduced by the well pull toward the goal. Zero inside the
    /// standoff, and zero outside it when the pull meets or exceeds the brake
    /// authority (the degraded no-stopping-plan state; `flip_point` and `eta`
    /// are `None` there). No HUD instrument reads this yet.
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
/// `try_remove`, not `remove`: this observer also fires while the ship is being
/// DESPAWNED (the scenario unload sweep, ship death), and the `get_entity`
/// guard only proves liveness at observer time - the queued remove lands after
/// the despawn completes in the same flush and warns "Entity despawned". The
/// fallible variant makes end-of-leg cleanup and teardown commute.
pub(super) fn remove_maneuver_telemetry(remove: On<Remove, Autopilot>, mut commands: Commands) {
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
    /// Trim on the derived hull turn rate, dimensionless. The rate itself comes
    /// from the ship's torque budget and live inertia (`hull_turn_rate`): the
    /// average rate of a torque-limited 180 is `sqrt(pi * max_torque / inertia)
    /// / 2`; 1.0 commands exactly that optimum, lower is more stately. This is
    /// what makes mass legible - a stripped hull turns visibly faster than a
    /// full build.
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
    /// endgame lives in sub-u/s errors - the brake tail, the boundary creep,
    /// the ~0.45-0.6 u/s doorstep residual - and with only the tight band the
    /// computer hunts them with visible attitude swings for seconds at every
    /// arrival. NOTE: this band must stay above the doorstep residual. ORBIT
    /// deliberately keeps the tight band (station-keeping's whole job is
    /// chasing small errors), which also preserves orbit_hold_enter's
    /// documented 2x relationship. Rest precision: an AXIAL residual (the
    /// shipped single-centered-drive ship) keeps the drive's aligned authority,
    /// so STOP still brakes to [`FlightSettings::stop_speed_epsilon`] exactly;
    /// a residual OFF the drive axis (a damage-shifted hull's recruit drift) is
    /// released at up to this band rather than hunted with attitude flips -
    /// that bounded creep is the contract, and the price of not wobbling.
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
    /// The planned ring never sits closer to the body than `clearance *
    /// (body_radius + surface_margin)` - engaging ORBIT while skimming the
    /// surface plans an orbit with room to breathe.
    pub orbit_clearance_factor: f32,
    /// The planned ring never sits beyond `safety * fade_start` of the SOI:
    /// orbits are only trusted in the unfaded core, and the safety margin keeps
    /// station-keeping off the fade band's edge.
    pub orbit_band_safety: f32,
    /// Default RCS fine-adjust speed cap (u/s): the terminal speed a held RCS
    /// nudge builds to on each ship-local axis before `rcs_burn_system`
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
            // unchanged (measured at 0.6 and 0.75).
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
