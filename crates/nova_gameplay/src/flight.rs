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
    /// Seconds of un-braked travel the arrival plan budgets for the hull to
    /// physically flip retrograde (plus engine spool) before deceleration
    /// actually starts. Without this the plan assumes instant retro thrust
    /// and sails deep through the standoff at speed.
    pub flip_lead_time: f32,
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
            // A 180 at PD freq 4 settles in roughly a second; pad for spool.
            flip_lead_time: 1.5,
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

/// Main-drive input (`0..1`) to deliver `impulse` this tick given the drive's
/// per-tick authority. Pure for unit testing.
fn burn_input(impulse: f32, authority: f32) -> f32 {
    if authority <= 0.0 {
        return 0.0;
    }
    (impulse / authority).clamp(0.0, 1.0)
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
fn is_forward_aligned(thrust_dir: Vec3, forward: Vec3) -> bool {
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
/// for the goal, face the velocity *error*, burn (spooled) when aligned. The
/// flip-and-burn emerges the moment the error points backward. Disengages
/// (removes [`Autopilot`]) when the goal is reached, the target is gone, or
/// the flight computer (live controller section) is lost.
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
            &mut ThrusterSectionInput,
            &ThrusterSectionMagnitude,
            &Rotation,
            &ChildOf,
        ),
        (
            With<ThrusterSectionMarker>,
            Without<SectionInactiveMarker>,
            Without<SpaceshipRootMarker>,
        ),
    >,
    // A live flight computer is a controller section that still has its PD
    // (preview controllers have none) and is not disabled.
    q_computer: Query<
        &ChildOf,
        (
            With<ControllerSectionMarker>,
            With<PDController>,
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

    for (ship, mut autopilot, position, rotation, velocity, mass) in &mut q_ship {
        // No flight computer, no autopilot - the ship is adrift on manual.
        if !q_computer.iter().any(|&ChildOf(parent)| parent == ship) {
            debug!("autopilot_system: ship {ship:?} lost its flight computer, disengaging");
            commands.entity(ship).remove::<Autopilot>();
            continue;
        }

        let forward = rotation.mul_vec3(Vec3::NEG_Z).normalize();

        // Main-drive authority from the live, forward-aligned thrusters, and
        // how hot those engines currently run (for the settle check below).
        let mut main_authority = 0.0f32;
        let mut hottest_input = 0.0f32;
        for (input, magnitude, thruster_rotation, &ChildOf(parent)) in &q_thruster {
            if parent != ship {
                continue;
            }
            let dir = thruster_rotation.mul_vec3(Vec3::NEG_Z).normalize();
            if is_forward_aligned(dir, forward) {
                main_authority += **magnitude;
                hottest_input = hottest_input.max(**input);
            }
        }

        // Braking acceleration (u/s^2) the maneuver can plan with.
        let accel = if dt > 0.0 && mass.value() > 0.0 {
            (main_authority / mass.value()) / dt
        } else {
            0.0
        };

        // The goal, as a desired velocity right now.
        let desired = match autopilot.action {
            AutopilotAction::Stop => Vec3::ZERO,
            AutopilotAction::Goto { target } => {
                let Ok(target_transform) = q_target.get(target) else {
                    debug!("autopilot_system: GOTO target {target:?} is gone, disengaging");
                    commands.entity(ship).remove::<Autopilot>();
                    continue;
                };
                goto_desired_velocity(
                    target_transform.translation() - position.0,
                    settings.arrival_standoff,
                    accel,
                    settings.decel_margin,
                    settings.flip_lead_time,
                    settings.min_approach_speed,
                )
            }
        };

        let error = desired - **velocity;
        let error_speed = error.length();

        // Done: the goal wants rest here, the ship is (nearly) at rest, AND
        // the engines have wound down - releasing the ship with a still-hot,
        // spooling-down drive would let the dying burn push it off again.
        let at_rest = desired == Vec3::ZERO && error_speed <= settings.stop_speed_epsilon;
        if at_rest && hottest_input <= 0.05 {
            debug!("autopilot_system: ship {ship:?} maneuver complete, disengaging");
            commands.entity(ship).remove::<Autopilot>();
            continue;
        }

        // Face the error, burn when aligned - spool everything. While
        // settling (at rest, engines still winding down) just command zero.
        let (target_input, aligned) = if at_rest || error_speed <= 1e-3 {
            (0.0, false)
        } else {
            let error_dir = error / error_speed;
            // Minimal rotation from the current attitude, preserving roll.
            let target_rotation = Quat::from_rotation_arc(forward, error_dir) * rotation.0;
            for (mut input, &ChildOf(parent)) in &mut q_rotation_input {
                if parent == ship {
                    **input = target_rotation;
                }
            }
            let aligned = forward.dot(error_dir) >= settings.align_cos;
            let input = if aligned {
                burn_input(error_speed * mass.value(), main_authority)
            } else {
                0.0
            };
            (input, aligned)
        };

        autopilot.phase = if aligned {
            AutopilotPhase::Burn
        } else {
            AutopilotPhase::Align
        };

        for (mut input, _, thruster_rotation, &ChildOf(parent)) in &mut q_thruster {
            if parent != ship {
                continue;
            }
            let dir = thruster_rotation.mul_vec3(Vec3::NEG_Z).normalize();
            if !is_forward_aligned(dir, forward) {
                continue;
            }
            **input = spool(
                **input,
                target_input,
                settings.spool_up_rate,
                settings.spool_down_rate,
                dt,
            );
        }
    }
}

/// When the autopilot lets go - completion or any breakout - it cools the
/// engines it was driving. Nothing else writes a *bound* thruster's input
/// between key events (the manual burn system deliberately leaves bound
/// thrusters to their own keys), so a residual autopilot burn would
/// otherwise ghost on forever.
fn on_autopilot_removed_cool_engines(
    remove: On<Remove, Autopilot>,
    mut q_thruster: Query<(&mut ThrusterSectionInput, &ChildOf), With<ThrusterSectionMarker>>,
) {
    for (mut input, &ChildOf(parent)) in &mut q_thruster {
        if parent == remove.entity {
            **input = 0.0;
        }
    }
}

/// Manual main-drive burn for intent-carrying ships with no autopilot
/// engaged: spool the live forward thrusters toward the analog burn input.
fn manual_burn_system(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    q_ship: Query<
        (Entity, &FlightIntent, &Rotation),
        (With<SpaceshipRootMarker>, Without<Autopilot>),
    >,
    mut q_thruster: Query<
        (&mut ThrusterSectionInput, &Rotation, &ChildOf),
        (
            With<ThrusterSectionMarker>,
            Without<SectionInactiveMarker>,
            Without<SpaceshipRootMarker>,
            Without<SpaceshipThrusterInputBinding>,
        ),
    >,
) {
    let dt = time.delta_secs();

    for (ship, intent, rotation) in &q_ship {
        let forward = rotation.mul_vec3(Vec3::NEG_Z).normalize();
        let target = intent.burn.clamp(0.0, 1.0);

        for (mut input, thruster_rotation, &ChildOf(parent)) in &mut q_thruster {
            if parent != ship {
                continue;
            }
            let dir = thruster_rotation.mul_vec3(Vec3::NEG_Z).normalize();
            if !is_forward_aligned(dir, forward) {
                continue;
            }
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
    fn burn_input_scales_and_saturates() {
        assert!((burn_input(0.5, 1.0) - 0.5).abs() < 1e-6);
        assert_eq!(burn_input(5.0, 1.0), 1.0);
        assert_eq!(burn_input(1.0, 0.0), 0.0);
        assert_eq!(burn_input(-1.0, 1.0), 0.0);
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

        run(&mut app, 2400);

        let standoff = app.world().resource::<FlightSettings>().arrival_standoff;
        let pos = app.world().get::<Position>(ship).unwrap().0;
        let distance = (Vec3::new(0.0, 0.0, -300.0) - pos).length();
        let speed = velocity_of(&app, ship).length();
        assert!(
            distance <= standoff + 5.0 && distance >= standoff - 45.0,
            "should arrive near the {standoff}u standoff, got {distance}"
        );
        assert!(speed < 0.5, "should arrive at rest, got {speed}");
        assert!(
            app.world().get::<Autopilot>(ship).is_none(),
            "arrival disengages"
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
        };
        let controller = app
            .world_mut()
            .spawn((
                ChildOf(ship),
                base_section(base("controller")),
                controller_section(ControllerSectionConfig {
                    frequency: 4.0,
                    damping_ratio: 4.0,
                    max_torque: 100.0,
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
        assert!(
            engaged_speed < manual_speed + 0.5,
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
