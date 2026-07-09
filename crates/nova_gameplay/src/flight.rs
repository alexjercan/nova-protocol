//! Nova's flight-control layer: the ship's flight computer, sitting between
//! player intent and the honest thruster simulation.
//!
//! Design: docs/spikes/20260709-094731-flight-feel-assisted-newtonian.md.
//! Two modes, toggled at runtime ([`FlightAssistMode`]):
//!
//! - **Assisted** (default): velocity-command. The player's intent nudges a
//!   *commanded velocity* ([`FlightCommand`]); no input means hold the current
//!   command (a real Newtonian hold, not drag), brake commands zero, and the
//!   commanded speed is soft-capped. Each physics tick the computer burns
//!   toward the command: the forward component goes to the main drive by
//!   writing the live thrusters' [`ThrusterSectionInput`] (so the plume,
//!   audio, and the actual impulse all ride the existing seam), and the
//!   remainder goes to RCS as a clamped impulse at the center of mass.
//! - **Newtonian** ("FA off"): no velocity servo. Intent maps directly to
//!   thrust - forward intent drives the main thrusters, everything else is a
//!   direct RCS burn - and momentum persists until you burn it off.
//!
//! Nothing here fakes physics: there is no drag anywhere, forces never exceed
//! what the surviving sections can produce, and capability dies with them -
//! thrusters shot off remove main-drive authority, and the RCS (plus the
//! whole assisted servo) lives in the controller section, so a ship that
//! loses its controller is adrift on raw engines. The layer only drives ships
//! that carry [`FlightIntent`] (the player's; see [`insert_flight_control`]) -
//! the AI keeps writing `ThrusterSectionInput` / rotation inputs directly.
//!
//! Thruster inputs are written through a spool ramp instead of snapping, so
//! the exhaust shader and the engine hum read as an engine lighting up rather
//! than a switch. All tunables live on the reflected [`FlightSettings`]
//! resource; the whole tree is registered so the inspector and a future
//! settings menu can traverse it. The math is in pure helpers, unit-tested,
//! and deliberately free of avian types where possible - it is a future
//! `bevy_common_systems` promotion candidate.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        FlightAssistMode, FlightCommand, FlightIntent, FlightRcsImpulse, FlightSettings,
        NovaFlightPlugin, NovaFlightSystems,
    };
}

/// A thruster counts as part of the main drive when its thrust direction
/// aligns with the ship's forward axis at least this much (cosine). Everything
/// less aligned is left alone - the flight computer only commands the engines
/// that actually push the ship forward.
const FORWARD_ALIGNMENT_COS: f32 = 0.9;

/// Which control law the flight computer applies. Lives on the ship root next
/// to [`FlightIntent`].
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default, Reflect)]
pub enum FlightAssistMode {
    /// Velocity-command: intent nudges a commanded velocity the computer
    /// holds; brake commands zero. The approachable default.
    #[default]
    Assisted,
    /// Direct thrust ("FA off"): intent maps straight to engine/RCS output,
    /// momentum persists, you burn to stop.
    Newtonian,
}

impl FlightAssistMode {
    /// The other mode; used by the toggle input.
    pub fn toggled(self) -> Self {
        match self {
            FlightAssistMode::Assisted => FlightAssistMode::Newtonian,
            FlightAssistMode::Newtonian => FlightAssistMode::Assisted,
        }
    }
}

/// The pilot's translation intent, in the ship's local axes (`-Z` is forward,
/// matching the thruster and camera conventions): each component `-1..1`.
/// Written by the player input layer every frame; consumed by the flight
/// computer each physics tick.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
pub struct FlightIntent {
    /// Local-frame thrust intent (`x` right, `y` up, `z` backward - a forward
    /// burn is `z = -1`, exactly what a `Spatial` input preset produces).
    pub linear: Vec3,
    /// Brake. Assisted: a kill-velocity latch - while set the command is
    /// zero, so tapping brake means "come to a stop" until a direction input
    /// re-takes the command. Newtonian: a plain full retro RCS burn while
    /// held (no servo - it burns past zero if you let it).
    pub brake: bool,
}

/// The commanded (target) velocity the assisted mode holds, world-space.
/// `None` until the first assisted tick initializes it from the ship's actual
/// velocity, so a ship spawned in motion is held at that motion instead of
/// being yanked to zero. In Newtonian mode it tracks the actual velocity every
/// tick, which makes toggling back to assisted seamless.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
pub struct FlightCommand {
    pub velocity: Option<Vec3>,
}

/// The RCS impulse (world-space, per physics tick) the flight computer wants
/// applied at the ship's center of mass this tick. Computed by
/// [`flight_control_system`], applied by [`apply_flight_rcs`] - the same
/// compute/apply split the PD controller uses, because avian's `Forces` query
/// conflicts with reading `LinearVelocity` in the same system. Also a handy
/// seam for future RCS visual/audio cues.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, Reflect)]
pub struct FlightRcsImpulse(pub Vec3);

/// All flight-feel tunables in one reflected resource, for the inspector and
/// a future settings menu. Authority (how hard the ship can burn) is *not*
/// here - that comes from the ship's live sections.
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct FlightSettings {
    /// How fast a held intent slews the commanded velocity, world units/s per
    /// second. Higher = the command outruns the ship more eagerly.
    pub command_accel: f32,
    /// Soft cap on the commanded speed in assisted mode. The computer refuses
    /// to command faster; Newtonian mode is deliberately uncapped.
    pub max_commanded_speed: f32,
    /// Spool rate toward a higher thruster input, 1/s (exponential). Engines
    /// light up at this rate.
    pub spool_up_rate: f32,
    /// Spool rate toward a lower thruster input, 1/s. Engines cut faster than
    /// they light.
    pub spool_down_rate: f32,
}

impl Default for FlightSettings {
    fn default() -> Self {
        Self {
            // The command outpaces the ~13 u/s^2 the default single-thruster
            // ship can actually pull, so holding W keeps the engine lit
            // instead of stuttering at the command.
            command_accel: 15.0,
            // Comfortably above the AI's 20 u/s chase ceiling and below the
            // torpedo's 35 u/s, so you can outrun ships but not ordnance.
            max_commanded_speed: 30.0,
            spool_up_rate: 6.0,
            spool_down_rate: 10.0,
        }
    }
}

/// System set for the flight computer; ordered before the section systems in
/// `FixedUpdate` so the thruster impulse system consumes the inputs written
/// this tick.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NovaFlightSystems;

/// Plugin wiring the flight-control layer.
#[derive(Default)]
pub struct NovaFlightPlugin;

impl Plugin for NovaFlightPlugin {
    fn build(&self, app: &mut App) {
        debug!("NovaFlightPlugin: build");

        app.init_resource::<FlightSettings>()
            // Register the whole reflected tree, not just the resource root.
            .register_type::<FlightSettings>()
            .register_type::<FlightAssistMode>()
            .register_type::<FlightIntent>()
            .register_type::<FlightCommand>()
            .register_type::<FlightRcsImpulse>();

        app.add_observer(insert_flight_control);

        app.configure_sets(
            FixedUpdate,
            NovaFlightSystems.before(SpaceshipSectionSystems),
        );
        app.add_systems(
            FixedUpdate,
            (flight_control_system, apply_flight_rcs)
                .chain()
                .in_set(NovaFlightSystems),
        );
    }
}

/// Give the player's ship the flight-control components. Only intent-carrying
/// ships are driven by the flight computer; AI ships keep their direct seams.
fn insert_flight_control(add: On<Add, PlayerSpaceshipMarker>, mut commands: Commands) {
    let entity = add.entity;
    trace!("insert_flight_control: entity {:?}", entity);

    commands.entity(entity).insert((
        FlightIntent::default(),
        FlightAssistMode::default(),
        FlightCommand::default(),
        FlightRcsImpulse::default(),
    ));
}

/// Nudge the commanded velocity toward `dir_world` (unit-ish intent direction
/// in world space) and clamp the result to the soft cap. Pure for unit
/// testing.
fn nudge_command(command: Vec3, dir_world: Vec3, accel: f32, dt: f32, max_speed: f32) -> Vec3 {
    (command + dir_world * accel * dt).clamp_length_max(max_speed)
}

/// Split a local-frame intent (`-Z` forward) into the main-drive throttle
/// (forward component, `0..1`) and the RCS thrust direction (laterals plus
/// retro), also local-frame with length clamped to 1. Pure for unit testing.
fn split_intent(local: Vec3) -> (f32, Vec3) {
    let throttle = (-local.z).clamp(0.0, 1.0);
    let rcs = Vec3::new(
        local.x.clamp(-1.0, 1.0),
        local.y.clamp(-1.0, 1.0),
        local.z.clamp(0.0, 1.0),
    )
    .clamp_length_max(1.0);
    (throttle, rcs)
}

/// The main-drive input (`0..1`) that best serves the needed impulse: the
/// forward component of `needed`, as a fraction of the drive's authority.
/// Thrusters only push forward, so a backward need yields zero (the RCS or a
/// flip handles it). Pure for unit testing.
fn main_target_input(needed: Vec3, forward: Vec3, main_authority: f32) -> f32 {
    if main_authority <= 0.0 {
        return 0.0;
    }
    (needed.dot(forward) / main_authority).clamp(0.0, 1.0)
}

/// The RCS impulse for this tick: whatever the (already spooling) main drive
/// does not deliver, clamped to the RCS authority. Pure for unit testing.
fn rcs_impulse(needed: Vec3, delivered_main: Vec3, rcs_authority: f32) -> Vec3 {
    (needed - delivered_main).clamp_length_max(rcs_authority.max(0.0))
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

/// The flight computer. For every intent-carrying ship: update the commanded
/// velocity from intent (assisted), then turn the velocity error into a main
/// drive input (spooled onto the live thrusters) and an RCS impulse for
/// [`apply_flight_rcs`]. Newtonian mode skips the servo and maps intent to
/// thrust directly.
fn flight_control_system(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    mut q_ship: Query<
        (
            Entity,
            &FlightAssistMode,
            &FlightIntent,
            &mut FlightCommand,
            &mut FlightRcsImpulse,
            &Rotation,
            &LinearVelocity,
            &ComputedMass,
        ),
        With<SpaceshipRootMarker>,
    >,
    // A thruster with a manual per-section binding (the editor lets players
    // bind keys straight to thrusters) belongs to the pilot, not the flight
    // computer: it is excluded from both the authority sum and the drive.
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
            Without<SpaceshipThrusterInputBinding>,
        ),
    >,
    q_rcs: Query<
        (&ControllerSectionRcsMagnitude, &ChildOf),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
) {
    let dt = time.delta_secs();

    for (ship, mode, intent, mut command, mut rcs_out, rotation, velocity, mass) in &mut q_ship {
        let forward = rotation.mul_vec3(Vec3::NEG_Z).normalize();

        // Capability from the live sections: the computer (and its RCS
        // authority) is the controller section; the main drive is the sum of
        // the forward-aligned live thrusters.
        let mut has_computer = false;
        let mut rcs_authority = 0.0f32;
        for (rcs_magnitude, &ChildOf(parent)) in &q_rcs {
            if parent == ship {
                has_computer = true;
                rcs_authority = rcs_authority.max(**rcs_magnitude);
            }
        }
        let mut main_authority = 0.0f32;
        for (_, magnitude, thruster_rotation, &ChildOf(parent)) in &q_thruster {
            if parent != ship {
                continue;
            }
            let dir = thruster_rotation.mul_vec3(Vec3::NEG_Z).normalize();
            if is_forward_aligned(dir, forward) {
                main_authority += **magnitude;
            }
        }

        let assisted = *mode == FlightAssistMode::Assisted && has_computer;

        // `servo_needed` carries the assisted servo impulse (computed once,
        // reused for the RCS remainder below); `direct_rcs_local` carries the
        // Newtonian direct burn. Exactly one is set.
        let (target_input, servo_needed, direct_rcs_local) = if assisted {
            // Update the commanded velocity: brake latches it to zero, intent
            // nudges it (soft-capped), no input holds it.
            let cmd = command.velocity.get_or_insert(**velocity);
            if intent.brake {
                *cmd = Vec3::ZERO;
            } else if intent.linear != Vec3::ZERO {
                let dir_world = rotation.mul_vec3(intent.linear.clamp_length_max(1.0));
                *cmd = nudge_command(
                    *cmd,
                    dir_world,
                    settings.command_accel,
                    dt,
                    settings.max_commanded_speed,
                );
            }

            let needed = (*cmd - **velocity) * mass.value();
            (
                main_target_input(needed, forward, main_authority),
                Some(needed),
                None,
            )
        } else {
            // Newtonian (or the computer is gone): track the actual velocity
            // so toggling back to assisted is seamless, and map intent
            // straight to thrust. Brake has no servo to command here, so it
            // means a plain full retro burn instead of a dead key.
            command.velocity = Some(**velocity);
            let local = if intent.brake { Vec3::Z } else { intent.linear };
            let (throttle, rcs_local) = split_intent(local);
            (throttle, None, Some(rcs_local))
        };

        // Spool the main drive toward its target and account for what it will
        // actually deliver this tick, so the RCS only covers the remainder.
        let mut delivered = Vec3::ZERO;
        for (mut input, magnitude, thruster_rotation, &ChildOf(parent)) in &mut q_thruster {
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
            delivered += dir * **magnitude * **input;
        }

        **rcs_out = match (servo_needed, direct_rcs_local) {
            // Assisted: burn the remaining velocity error, clamped by RCS.
            (Some(needed), _) => rcs_impulse(needed, delivered, rcs_authority),
            // Direct: intent is the burn; no servo.
            (None, Some(rcs_local)) if has_computer => rotation.mul_vec3(rcs_local * rcs_authority),
            _ => Vec3::ZERO,
        };
    }
}

/// Apply the RCS impulse computed by [`flight_control_system`] at the ship's
/// center of mass. Separate system because avian's `Forces` query data writes
/// `LinearVelocity`, which the control system reads.
fn apply_flight_rcs(
    q_ship: Query<(Entity, &FlightRcsImpulse), With<SpaceshipRootMarker>>,
    mut q_forces: Query<Forces>,
) {
    for (ship, impulse) in &q_ship {
        if **impulse == Vec3::ZERO {
            continue;
        }
        let Ok(mut forces) = q_forces.get_mut(ship) else {
            continue;
        };
        forces.apply_linear_impulse(**impulse);
    }
}

/// One line of HUD truth about the flight computer, shared with the HUD
/// module so the formatting is unit-testable.
pub(crate) fn flight_status_line(
    mode: FlightAssistMode,
    speed: f32,
    commanded_speed: Option<f32>,
) -> String {
    match (mode, commanded_speed) {
        (FlightAssistMode::Assisted, Some(cmd)) => {
            format!("FA ON   {speed:5.1} -> {cmd:5.1} u/s")
        }
        (FlightAssistMode::Assisted, None) => format!("FA ON   {speed:5.1} u/s"),
        (FlightAssistMode::Newtonian, _) => format!("FA OFF  {speed:5.1} u/s"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Pure helpers ----------------------------------------------------

    #[test]
    fn spool_ramps_up_and_down_at_distinct_rates_and_clamps() {
        // Rising uses the up rate...
        let up = spool(0.0, 1.0, 6.0, 10.0, 0.1);
        assert!(up > 0.0 && up < 1.0);
        // ...falling uses the (faster) down rate, so the same dt cuts deeper.
        let down = spool(1.0, 0.0, 6.0, 10.0, 0.1);
        assert!(1.0 - down > up, "cut should outpace light-up");
        // Converges to the target and clamps into 0..1.
        let mut v = 0.0;
        for _ in 0..200 {
            v = spool(v, 1.0, 6.0, 10.0, 1.0 / 60.0);
        }
        assert!((v - 1.0).abs() < 1e-3);
        assert_eq!(spool(2.0, 1.5, 6.0, 10.0, 10.0), 1.0);
    }

    #[test]
    fn split_intent_separates_throttle_from_rcs() {
        // Pure forward burn: all throttle, no RCS.
        let (throttle, rcs) = split_intent(Vec3::new(0.0, 0.0, -1.0));
        assert_eq!(throttle, 1.0);
        assert_eq!(rcs, Vec3::ZERO);
        // Retro + lateral: no throttle, RCS carries both.
        let (throttle, rcs) = split_intent(Vec3::new(1.0, 0.0, 1.0));
        assert_eq!(throttle, 0.0);
        assert!(rcs.x > 0.0 && rcs.z > 0.0);
        // Length is clamped so diagonal input cannot exceed the authority.
        assert!(rcs.length() <= 1.0 + 1e-6);
    }

    #[test]
    fn main_target_input_serves_only_the_forward_component() {
        let forward = Vec3::NEG_Z;
        // A forward need maps to a fraction of the authority.
        assert!((main_target_input(Vec3::new(0.0, 0.0, -2.0), forward, 4.0) - 0.5).abs() < 1e-6);
        // Saturates at full throttle.
        assert_eq!(
            main_target_input(Vec3::new(0.0, 0.0, -8.0), forward, 4.0),
            1.0
        );
        // A backward need cannot be served by forward engines.
        assert_eq!(
            main_target_input(Vec3::new(0.0, 0.0, 3.0), forward, 4.0),
            0.0
        );
        // No authority, no input (and no division by zero).
        assert_eq!(main_target_input(Vec3::NEG_Z, forward, 0.0), 0.0);
    }

    #[test]
    fn rcs_impulse_covers_the_remainder_and_respects_authority() {
        let needed = Vec3::new(3.0, 0.0, -4.0);
        let delivered = Vec3::new(0.0, 0.0, -4.0);
        // Exactly the lateral remainder when within authority.
        assert_eq!(
            rcs_impulse(needed, delivered, 5.0),
            Vec3::new(3.0, 0.0, 0.0)
        );
        // Clamped to the authority when the remainder is too large.
        let clamped = rcs_impulse(needed, Vec3::ZERO, 1.0);
        assert!((clamped.length() - 1.0).abs() < 1e-6);
        // Negative authority is treated as none.
        assert_eq!(rcs_impulse(needed, delivered, -1.0), Vec3::ZERO);
    }

    #[test]
    fn nudge_command_accelerates_and_soft_caps() {
        let cmd = nudge_command(Vec3::ZERO, Vec3::NEG_Z, 10.0, 0.5, 30.0);
        assert!((cmd.z + 5.0).abs() < 1e-6);
        // The cap is on the commanded speed, however long the key is held.
        let mut c = Vec3::ZERO;
        for _ in 0..1000 {
            c = nudge_command(c, Vec3::NEG_Z, 10.0, 0.1, 30.0);
        }
        assert!((c.length() - 30.0).abs() < 1e-3);
    }

    #[test]
    fn forward_alignment_selects_main_drive_thrusters() {
        let forward = Vec3::NEG_Z;
        assert!(is_forward_aligned(Vec3::NEG_Z, forward));
        // A retro or lateral thruster is not main drive.
        assert!(!is_forward_aligned(Vec3::Z, forward));
        assert!(!is_forward_aligned(Vec3::X, forward));
    }

    #[test]
    fn flight_status_line_formats_each_mode() {
        assert_eq!(
            flight_status_line(FlightAssistMode::Assisted, 12.34, Some(20.0)),
            "FA ON    12.3 ->  20.0 u/s"
        );
        assert_eq!(
            flight_status_line(FlightAssistMode::Newtonian, 5.0, Some(20.0)),
            "FA OFF    5.0 u/s"
        );
    }

    #[test]
    fn toggled_flips_the_mode() {
        assert_eq!(
            FlightAssistMode::Assisted.toggled(),
            FlightAssistMode::Newtonian
        );
        assert_eq!(
            FlightAssistMode::Newtonian.toggled(),
            FlightAssistMode::Assisted
        );
    }

    #[test]
    fn player_marker_receives_flight_components() {
        let mut app = App::new();
        app.add_observer(insert_flight_control);

        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        app.update();

        assert!(app.world().get::<FlightIntent>(ship).is_some());
        assert_eq!(
            app.world().get::<FlightAssistMode>(ship),
            Some(&FlightAssistMode::Assisted),
            "assisted is the default mode"
        );
        assert!(app.world().get::<FlightCommand>(ship).is_some());
        assert!(app.world().get::<FlightRcsImpulse>(ship).is_some());
    }

    // --- Physics-level integration ---------------------------------------
    //
    // A real avian world (the integrity test harness) with the flight systems
    // and the actual thruster impulse system, so these cover the whole
    // pipeline: intent -> command -> spooled thruster input -> impulse ->
    // velocity.

    use crate::{
        integrity::test_support::{integrity_physics_app, settle},
        sections::thruster_section::thruster_impulse_system,
    };

    fn flight_app() -> App {
        let mut app = integrity_physics_app();
        app.init_resource::<FlightSettings>();
        app.configure_sets(
            FixedUpdate,
            NovaFlightSystems.before(SpaceshipSectionSystems),
        );
        app.add_systems(
            FixedUpdate,
            (flight_control_system, apply_flight_rcs)
                .chain()
                .in_set(NovaFlightSystems),
        );
        app.add_systems(
            FixedUpdate,
            thruster_impulse_system.in_set(SpaceshipSectionSystems),
        );
        app
    }

    /// A minimal flyable ship: a hull collider at the origin, a rear main
    /// thruster (thrusting toward -Z, the ship's forward), and optionally a
    /// controller section carrying the RCS authority.
    fn spawn_ship(app: &mut App, mode: FlightAssistMode, with_rcs: bool) -> (Entity, Entity) {
        let ship = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                SpaceshipRootMarker,
                FlightIntent::default(),
                mode,
                FlightCommand::default(),
                FlightRcsImpulse::default(),
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
        if with_rcs {
            app.world_mut().spawn((
                ChildOf(ship),
                Name::new("controller"),
                ControllerSectionMarker,
                ControllerSectionRcsMagnitude(0.5),
                Transform::from_xyz(0.0, 1.0, 0.0),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ));
        }
        (ship, thruster)
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
    fn assisted_brake_kills_velocity() {
        let mut app = flight_app();
        let (ship, _) = spawn_ship(&mut app, FlightAssistMode::Assisted, true);
        settle(&mut app);

        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::new(4.0, 0.0, -8.0)));
        app.world_mut().get_mut::<FlightIntent>(ship).unwrap().brake = true;

        run(&mut app, 300);

        let speed = velocity_of(&app, ship).length();
        assert!(speed < 0.3, "brake should null the velocity, got {speed}");
    }

    #[test]
    fn assisted_holds_velocity_against_an_external_push() {
        let mut app = flight_app();
        let (ship, _) = spawn_ship(&mut app, FlightAssistMode::Assisted, true);
        settle(&mut app);
        // Let the command initialize to the resting velocity (zero), then
        // shove the ship sideways as a blast would.
        run(&mut app, 5);
        let held = velocity_of(&app, ship);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(held + Vec3::new(3.0, 0.0, 0.0)));

        run(&mut app, 300);

        let drift = (velocity_of(&app, ship) - held).length();
        assert!(
            drift < 0.3,
            "assist should station-keep after a push, drift {drift}"
        );
    }

    #[test]
    fn newtonian_coasts_without_any_damping() {
        let mut app = flight_app();
        let (ship, _) = spawn_ship(&mut app, FlightAssistMode::Newtonian, true);
        settle(&mut app);
        let v0 = Vec3::new(2.0, 1.0, -6.0);
        app.world_mut().entity_mut(ship).insert(LinearVelocity(v0));

        run(&mut app, 240);

        let drift = (velocity_of(&app, ship) - v0).length();
        assert!(drift < 1e-3, "coasting must not decay, drift {drift}");
    }

    #[test]
    fn newtonian_burn_accelerates_forward_and_dies_with_the_thruster() {
        let mut app = flight_app();
        let (ship, thruster) = spawn_ship(&mut app, FlightAssistMode::Newtonian, true);
        settle(&mut app);
        app.world_mut()
            .get_mut::<FlightIntent>(ship)
            .unwrap()
            .linear = Vec3::new(0.0, 0.0, -1.0);

        run(&mut app, 120);
        let burned = velocity_of(&app, ship);
        assert!(
            burned.z < -1.0,
            "full burn should accelerate along -Z, got {burned}"
        );

        // Kill the engine: no further forward acceleration.
        app.world_mut()
            .entity_mut(thruster)
            .insert(SectionInactiveMarker);
        // Spool state no longer matters - the impulse system skips inactive
        // sections - so velocity freezes.
        run(&mut app, 5);
        let at_cutoff = velocity_of(&app, ship);
        run(&mut app, 120);
        let after = velocity_of(&app, ship);
        assert!(
            (after - at_cutoff).length() < 1e-3,
            "a dead thruster must not thrust"
        );
    }

    #[test]
    fn newtonian_brake_is_a_direct_retro_burn() {
        let mut app = flight_app();
        let (ship, _) = spawn_ship(&mut app, FlightAssistMode::Newtonian, true);
        settle(&mut app);
        // Coasting forward (along -Z); braking with FA off must slow it via a
        // direct retro RCS burn, not a servo (it will happily burn past zero
        // if held, so only assert the decel while still moving forward).
        let v0 = Vec3::new(0.0, 0.0, -6.0);
        app.world_mut().entity_mut(ship).insert(LinearVelocity(v0));
        app.world_mut().get_mut::<FlightIntent>(ship).unwrap().brake = true;

        run(&mut app, 60);

        let v = velocity_of(&app, ship);
        assert!(
            v.z > v0.z + 0.5,
            "FA-off brake should retro-burn against the forward motion, got {v}"
        );
    }

    #[test]
    fn assisted_without_a_computer_does_not_station_keep() {
        let mut app = flight_app();
        // No controller section: the flight computer is gone.
        let (ship, _) = spawn_ship(&mut app, FlightAssistMode::Assisted, false);
        settle(&mut app);
        run(&mut app, 5);
        let kick = Vec3::new(3.0, 0.0, 0.0);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(kick));

        run(&mut app, 240);

        let v = velocity_of(&app, ship);
        assert!(
            (v - kick).length() < 1e-3,
            "without a controller there is no RCS to correct the push, got {v}"
        );
    }

    #[test]
    fn assisted_forward_intent_accelerates_and_respects_the_cap() {
        let mut app = flight_app();
        let (ship, _) = spawn_ship(&mut app, FlightAssistMode::Assisted, true);
        settle(&mut app);
        app.world_mut()
            .get_mut::<FlightIntent>(ship)
            .unwrap()
            .linear = Vec3::new(0.0, 0.0, -1.0);

        run(&mut app, 240);
        let v = velocity_of(&app, ship);
        assert!(v.z < -3.0, "assisted forward intent should burn, got {v}");

        // The command never exceeds the soft cap, so neither does the ship
        // (give it time to converge, then check).
        run(&mut app, 2000);
        let speed = velocity_of(&app, ship).length();
        let cap = app.world().resource::<FlightSettings>().max_commanded_speed;
        assert!(
            speed <= cap + 0.5,
            "speed {speed} must respect the commanded cap {cap}"
        );
    }
}
