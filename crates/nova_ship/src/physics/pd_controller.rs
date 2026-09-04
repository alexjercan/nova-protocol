//! PD Controller for 3D rotations in Bevy using Avian3D

use avian3d::prelude::*;
use bevy::prelude::*;

/// Component that defines a PD controller for rotational control.
///
/// `PartialEq` so a writer that re-derives this every tick can leave it alone
/// when nothing moved, instead of waking every reader through change detection.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[require(PDControllerInput, PDControllerOutput)]
pub struct PDController {
    /// The frequency of the PD controller in Hz.
    pub frequency: f32,
    /// The damping ratio of the PD controller.
    pub damping_ratio: f32,
    /// The maximum angular acceleration on each principal axis, in rad/s2.
    pub max_angular_acceleration: f32,
    /// The angular speed, in rad/s, that this loop is allowed to HOLD.
    ///
    /// Past it the loop may only slow the body down: whatever authority it has
    /// is projected onto the directions that reduce the spin, and the part
    /// pointing along the spin is dropped. `INFINITY` for a loop with no such
    /// limit, which is every controller that is not fighting a structural one.
    ///
    /// Not a clamp on the rate itself. An outside push - a collision, a blast,
    /// a hull that just lost the sections its limit was computed from - may
    /// leave the body over this speed, and the loop's whole job there is to
    /// bring it back down.
    pub sustained_angular_speed: f32,
}

/// Input rotation for the PD controller.
#[derive(Component, Debug, Clone, Default, Deref, DerefMut, Reflect)]
pub struct PDControllerInput(pub Quat);

/// Target entity for the PD controller to follow.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct PDControllerTarget(pub Entity);

/// Output torque from the PD controller.
#[derive(Component, Debug, Clone, Default, Deref, DerefMut, Reflect)]
pub struct PDControllerOutput(pub Vec3);

/// System sets for the PD controller pass.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PDControllerSystems {
    /// The FixedUpdate pass that turns each controller's input into a torque on
    /// its target root.
    Sync,
}

/// Registers the PD controller setup observer and its FixedUpdate torque pass.
pub struct PDControllerPlugin;

impl Plugin for PDControllerPlugin {
    fn build(&self, app: &mut App) {
        trace!("PDControllerPlugin: build");

        app.add_observer(setup_pd_controller_system);

        app.add_systems(
            FixedUpdate,
            update_controller_root_torque.in_set(PDControllerSystems::Sync),
        );
    }
}

fn setup_pd_controller_system(add: On<Add, PDController>, mut commands: Commands) {
    let entity = add.entity;
    trace!("setup_pd_controller_system: entity {:?}", entity);

    commands
        .entity(entity)
        .insert(PDControllerInput::default())
        .insert(PDControllerOutput::default());
}

/// Turn each controller's attitude command into a torque for the body it
/// targets.
///
/// A target that is not a body is NORMAL, not an error. A ship dies section by
/// section, so there are ticks where the root is already despawned and the
/// controller's own despawn has not applied yet, and a freshly severed
/// fragment is a rigid body a tick before avian's prepare pass gives it mass
/// properties. The output is ZEROED rather than left alone: it is the only
/// value `sync_controller_section_forces` applies, and a stale torque held
/// over from the body that no longer exists would be pushed into the next one
/// the controller is pointed at.
///
/// What keeps the target itself honest is
/// `sync_controller_section_target`, which re-points a controller
/// section when a split moves it onto a new hull. Before it existed this loop
/// logged an error at the fixed rate for the rest of the scene.
fn update_controller_root_torque(
    q_root: Query<(&ComputedAngularInertia, &Rotation, &AngularVelocity)>,
    mut q_controller: Query<(
        &PDController,
        &PDControllerInput,
        &PDControllerTarget,
        &mut PDControllerOutput,
    )>,
) {
    for (controller, controller_input, controller_target, mut controller_output) in
        &mut q_controller
    {
        let Ok((angular_inertia, rotation, angular_velocity)) = q_root.get(**controller_target)
        else {
            trace!(
                "update_controller_root_torque: target {:?} is not a body; holding no torque",
                **controller_target
            );
            if **controller_output != Vec3::ZERO {
                **controller_output = Vec3::ZERO;
            }
            continue;
        };

        let (principal, local_frame) = angular_inertia.principal_angular_inertia_with_local_frame();

        let torque = compute_pd_torque(
            controller.frequency,
            controller.damping_ratio,
            controller.max_angular_acceleration,
            controller.sustained_angular_speed,
            **rotation,
            **controller_input,
            **angular_velocity,
            principal,
            local_frame,
        );

        **controller_output = torque;
    }
}

fn compute_pd_torque(
    frequency: f32,
    damping_ratio: f32,
    max_angular_acceleration: f32,
    sustained_angular_speed: f32,
    from_rotation: Quat,
    to_rotation: Quat,
    angular_velocity: Vec3,
    inertia_principal: Vec3,
    inertia_local_frame: Quat,
) -> Vec3 {
    let kp = (6.0 * frequency).powi(2) * 0.25;
    let kd = 4.5 * frequency * damping_ratio;

    let mut delta = to_rotation * from_rotation.conjugate();
    if delta.w < 0.0 {
        delta = Quat::from_xyzw(-delta.x, -delta.y, -delta.z, -delta.w);
    }

    let (axis, mut angle) = delta.to_axis_angle();
    let axis = axis.normalize_or_zero(); // normalize_or_zero, not normalize: angle 0 gives no axis
    if angle > std::f32::consts::PI {
        angle -= 2.0 * std::f32::consts::PI;
    }

    let raw = axis * (kp * angle) - angular_velocity * kd;

    // Clamp acceleration before inertia scaling. An absolute torque cap
    // makes the same computer weaker as hull inertia grows. Principal-axis
    // limits produce the authored acceleration on every hull while still
    // applying the exact torque each axis requires.
    //
    // The composition order is load-bearing: `inertia_local_frame` maps the
    // principal frame into body-local and `from_rotation` maps body-local into
    // world, so body rotation applies after the local frame.
    let rot_inertia_to_world = from_rotation * inertia_local_frame;
    let acceleration_local = rot_inertia_to_world.inverse() * raw;
    let limit = max_angular_acceleration.max(0.0);
    let acceleration = acceleration_local.clamp(Vec3::splat(-limit), Vec3::splat(limit));
    let acceleration = brake_only_past(
        acceleration,
        rot_inertia_to_world.inverse() * angular_velocity,
        sustained_angular_speed,
        limit,
    );
    let torque_local = acceleration * inertia_principal;
    rot_inertia_to_world * torque_local
}

/// Drop whatever part of `acceleration` would make the body spin FASTER, once
/// it is already past the speed it is allowed to hold.
///
/// `acceleration` and `spin` are both in the principal frame, already inside
/// `limit`. A body over `sustained` is over it whichever way it is pointing
/// and whichever way the loop wants to turn, so the test is on the spin's own
/// direction rather than on any single axis.
///
/// The perpendicular part survives: swinging the spin AXIS does not add to the
/// rate, and a loop that lost it could not steer an overspun body at all.
fn brake_only_past(acceleration: Vec3, spin: Vec3, sustained: f32, limit: f32) -> Vec3 {
    if spin.length() <= sustained.max(0.0) {
        return acceleration;
    }
    let along = spin.normalize_or_zero();
    let speeding_up = acceleration.dot(along);
    if speeding_up <= 0.0 {
        return acceleration;
    }
    let braking = acceleration - along * speeding_up;
    // Taking that component out can push a single principal axis past `limit`.
    // Scale the whole vector rather than clipping the axis: a clip would put
    // some of the spin direction back, which is the one direction that must
    // not survive this.
    let excess = (braking.abs() / limit).max_element();
    if excess > 1.0 {
        braking / excess
    } else {
        braking
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    /// The world-space inertia tensor the PD must scale by, built by the
    /// dependency that defines the convention (bevy_heavy via avian) rather
    /// than re-derived here, so a shared misreading of the frame composition
    /// cannot pass both the implementation and the oracle.
    fn world_inertia(principal: Vec3, local_frame: Quat, rotation: Quat) -> Mat3 {
        Mat3::from(
            ComputedAngularInertia::new_with_local_frame(principal, local_frame)
                .rotated(rotation)
                .tensor(),
        )
    }

    /// Pure-damper closed form: with zero attitude error and no clamp the
    /// output must be exactly -kd * I_world * omega.
    fn assert_pure_damper_matches_closed_form(rotation: Quat, local_frame: Quat) {
        let frequency = 1.0;
        let damping_ratio = 1.0;
        let kd = 4.5 * frequency * damping_ratio;
        let principal = Vec3::new(2.0, 3.0, 5.0);
        let omega = Vec3::new(0.3, -0.2, 0.5);

        let torque = compute_pd_torque(
            frequency,
            damping_ratio,
            1.0e6, // no clamp
            f32::INFINITY,
            rotation,
            rotation, // command parked on the attitude: P term is zero
            omega,
            principal,
            local_frame,
        );

        let expected = world_inertia(principal, local_frame, rotation) * (-kd * omega);
        assert!(
            torque.abs_diff_eq(expected, 1.0e-3),
            "pure damper torque {torque} should equal -kd * I_world * omega = {expected} \
             (rotation {rotation:?}, local_frame {local_frame:?})"
        );
    }

    #[test]
    fn pure_damper_matches_closed_form_with_local_frame() {
        assert_pure_damper_matches_closed_form(Quat::IDENTITY, Quat::from_axis_angle(Vec3::X, 0.9));
    }

    #[test]
    fn pure_damper_matches_closed_form_with_body_rotation() {
        assert_pure_damper_matches_closed_form(
            Quat::from_axis_angle(Vec3::new(1.0, 2.0, -1.0).normalize(), 1.3),
            Quat::IDENTITY,
        );
    }

    #[test]
    fn pure_damper_matches_closed_form_with_both_frames() {
        assert_pure_damper_matches_closed_form(
            Quat::from_axis_angle(Vec3::new(-1.0, 0.5, 2.0).normalize(), 0.7),
            Quat::from_axis_angle(Vec3::new(0.2, 1.0, 0.4).normalize(), 1.1),
        );
    }

    #[test]
    fn clamp_limits_each_principal_axis_in_acceleration_units() {
        let principal = Vec3::new(2.5, 4.0, 0.5);
        let torque = compute_pd_torque(
            4.0,
            4.0,
            0.5,
            f32::INFINITY,
            Quat::IDENTITY,
            Quat::from_axis_angle(Vec3::Y, 2.0),
            Vec3::new(0.0, 0.0, 1.5),
            principal,
            Quat::IDENTITY,
        );
        let acceleration = torque / principal;
        assert!(
            acceleration.abs().max_element() <= 0.5 + 1.0e-5,
            "principal acceleration must stay inside the authored limit: {acceleration}"
        );
        assert!((acceleration.y - 0.5).abs() < 1.0e-5);
        assert!((acceleration.z + 0.5).abs() < 1.0e-5);
    }

    #[test]
    fn saturated_acceleration_is_inertia_invariant() {
        let torque = |principal| {
            compute_pd_torque(
                4.0,
                4.0,
                0.5,
                f32::INFINITY,
                Quat::IDENTITY,
                Quat::from_rotation_y(1.0),
                Vec3::ZERO,
                principal,
                Quat::IDENTITY,
            )
        };
        let light = Vec3::splat(2.0);
        let heavy = light * 10.0;
        assert!((torque(light) / light).abs_diff_eq(torque(heavy) / heavy, 1.0e-5));
    }

    #[test]
    fn test_compute_pd_torque_zero_error() {
        let torque = compute_pd_torque(
            1.0,
            1.0,
            10.0,
            f32::INFINITY,
            Quat::IDENTITY,
            Quat::IDENTITY,
            Vec3::ZERO,
            Vec3::ONE,
            Quat::IDENTITY,
        );
        assert!(torque.abs_diff_eq(Vec3::ZERO, 1e-6));
    }

    #[test]
    fn test_compute_pd_torque_small_angle() {
        let torque = compute_pd_torque(
            1.0,
            1.0,
            10.0,
            f32::INFINITY,
            Quat::IDENTITY,
            Quat::from_axis_angle(Vec3::Y, 0.1),
            Vec3::ZERO,
            Vec3::ONE,
            Quat::IDENTITY,
        );
        assert!(torque.length() > 0.0);
    }

    #[test]
    fn test_compute_pd_torque_large_angle() {
        let torque = compute_pd_torque(
            1.0,
            1.0,
            10.0,
            f32::INFINITY,
            Quat::IDENTITY,
            Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI),
            Vec3::ZERO,
            Vec3::ONE,
            Quat::IDENTITY,
        );
        assert!(torque.length() > 0.0);
    }

    #[test]
    fn test_compute_pd_torque_with_angular_velocity() {
        let torque = compute_pd_torque(
            1.0,
            1.0,
            10.0,
            f32::INFINITY,
            Quat::IDENTITY,
            Quat::from_axis_angle(Vec3::Y, 0.5),
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::ONE,
            Quat::IDENTITY,
        );
        assert!(torque.length() > 0.0);
    }

    /// The acceleration the loop actually applied, read back out of the torque
    /// it produced. Identity frames, so the principal axes are the world ones
    /// and the two are one division apart.
    fn applied_acceleration(
        max_angular_acceleration: f32,
        sustained_angular_speed: f32,
        command: Quat,
        angular_velocity: Vec3,
        principal: Vec3,
    ) -> Vec3 {
        compute_pd_torque(
            1.0,
            1.0,
            max_angular_acceleration,
            sustained_angular_speed,
            Quat::IDENTITY,
            command,
            angular_velocity,
            principal,
            Quat::IDENTITY,
        ) / principal
    }

    /// The rule the structural turn limit rests on: past the speed a body may
    /// hold, the loop cannot be made to spin it FASTER, however far ahead the
    /// command is.
    ///
    /// Both signs, because the spin direction is what the rule is stated
    /// against and a test on positive yaw alone would pass on a hard-coded
    /// "subtract from +y".
    #[test]
    fn an_overspun_body_cannot_be_pushed_faster() {
        let principal = Vec3::new(2.0, 3.0, 5.0);
        for sign in [1.0f32, -1.0] {
            let spin = Vec3::new(0.0, 2.0 * sign, 0.0);
            let acceleration =
                applied_acceleration(0.5, 1.0, Quat::from_rotation_y(2.0 * sign), spin, principal);
            let along = acceleration.dot(spin.normalize());
            assert!(
                along <= 1.0e-6,
                "a command {sign:+} rad ahead of a body already over its \
                 sustained speed added {along} rad/s2 to the spin"
            );
        }
    }

    /// The other half: the authority handed back past the corner is real. A
    /// body over its speed brakes at the FULL limit, or an overspun hull could
    /// never come back down.
    #[test]
    fn an_overspun_body_still_brakes_at_the_full_limit() {
        let principal = Vec3::new(2.0, 3.0, 5.0);
        let spin = Vec3::new(0.0, 2.0, 0.0);
        // Command parked on the attitude: the loop is a pure damper, so
        // everything it asks for opposes the spin.
        let acceleration = applied_acceleration(0.5, 1.0, Quat::IDENTITY, spin, principal);
        assert!(
            (acceleration.y + 0.5).abs() < 1.0e-5,
            "braking must keep the whole authority, got {acceleration}"
        );
    }

    /// Below the corner nothing is withheld: the limit is a rate the body may
    /// HOLD, not a rate it may not approach.
    #[test]
    fn a_body_under_its_sustained_speed_keeps_all_its_authority() {
        let principal = Vec3::new(2.0, 3.0, 5.0);
        let spin = Vec3::new(0.0, 2.0, 0.0);
        let acceleration =
            applied_acceleration(0.5, 3.0, Quat::from_rotation_y(2.0), spin, principal);
        assert!(
            (acceleration.y - 0.5).abs() < 1.0e-5,
            "under the sustained speed the loop keeps the full clamp, got {acceleration}"
        );
    }

    /// An overspun body must still be STEERABLE: a command across the spin
    /// swings the axis without adding to the rate, so the rule takes the
    /// component along the spin and leaves the rest.
    #[test]
    fn an_overspun_body_still_takes_a_cross_axis_command() {
        let principal = Vec3::splat(1.0);
        let spin = Vec3::new(0.0, 0.0, 2.0);
        let acceleration =
            applied_acceleration(0.5, 1.0, Quat::from_rotation_y(2.0), spin, principal);
        assert!(
            acceleration.y > 0.4,
            "the cross-axis half of the command must survive, got {acceleration}"
        );
        assert!(
            acceleration.z <= 1.0e-6,
            "the along-spin half must not, got {acceleration}"
        );
    }

    /// An oblique spin is the case a per-axis rule gets wrong: the spin lies
    /// across the principal axes, so taking its component out moves every axis
    /// and can push one of them past the clamp. The result must stay inside
    /// the authority AND off the spin direction - both, not either.
    #[test]
    fn an_oblique_overspun_body_stays_inside_its_authority() {
        let principal = Vec3::splat(1.0);
        let spin = Vec3::new(1.0, 1.0, 0.0).normalize() * 2.0;
        let acceleration =
            applied_acceleration(0.5, 1.0, Quat::from_rotation_y(2.0), spin, principal);
        assert!(
            acceleration.dot(spin.normalize()) <= 1.0e-6,
            "an oblique spin must not be fed either, got {acceleration}"
        );
        assert!(
            acceleration.abs().max_element() <= 0.5 + 1.0e-5,
            "taking the spin component out must not spend more than the \
             authority, got {acceleration}"
        );
    }

    // The tests below are the avian integration repro - a released, fast-spinning body
    // must despin. They wire the controller the way a game does: PD input written before
    // `PDControllerSystems::Sync`, output applied via `Forces::apply_torque` after it, physics
    // stepping in FixedUpdate. The body is a symmetric top released with a roll about its long
    // axis and a command frozen at the release attitude - the configuration that corkscrewed.

    /// Marks a controller whose command should track the body attitude every
    /// tick (pure damper); without it the command stays frozen where it was.
    #[derive(Component)]
    struct TrackAttitude;

    fn track_command(
        q_root: Query<&Rotation>,
        mut q_controller: Query<(&mut PDControllerInput, &PDControllerTarget), With<TrackAttitude>>,
    ) {
        for (mut input, target) in &mut q_controller {
            if let Ok(rotation) = q_root.get(**target) {
                **input = **rotation;
            }
        }
    }

    /// Marks a controller holding a fixed turn to starboard, re-read off the
    /// body every tick. The attitude error never shrinks, so the loop never
    /// stops asking for more turn - a pilot leaning on the stick.
    #[derive(Component)]
    struct HoldTurn;

    fn hold_turn(
        q_root: Query<&Rotation>,
        mut q_controller: Query<(&mut PDControllerInput, &PDControllerTarget), With<HoldTurn>>,
    ) {
        for (mut input, target) in &mut q_controller {
            if let Ok(rotation) = q_root.get(**target) {
                **input = rotation.0 * Quat::from_rotation_y(core::f32::consts::FRAC_PI_2);
            }
        }
    }

    fn apply_pd_output(
        mut q_root: Query<Forces>,
        q_controller: Query<(&PDControllerOutput, &PDControllerTarget)>,
    ) {
        for (output, target) in &q_controller {
            if let Ok(mut forces) = q_root.get_mut(**target) {
                forces.apply_torque(**output);
            }
        }
    }

    fn physics_app() -> App {
        let mut app = App::new();
        // AssetPlugin + MeshPlugin are required even for primitive colliders - avian's
        // collider cache reads `AssetEvent<Mesh>`.
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
        ));
        app.add_plugins(PhysicsPlugins::default());
        app.add_plugins(PDControllerPlugin);
        app.insert_resource(Gravity(Vec3::ZERO));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        app.add_systems(FixedUpdate, track_command.before(PDControllerSystems::Sync));
        app.add_systems(FixedUpdate, hold_turn.before(PDControllerSystems::Sync));
        app.add_systems(
            FixedUpdate,
            apply_pd_output.after(PDControllerSystems::Sync),
        );
        // Do not drop - avian initializes its diagnostics resources in `Plugin::finish`.
        app.finish();
        app
    }

    /// A ship-like rigid body: three unit cuboids along z (the long axis), so
    /// the inertia is a symmetric top with the smallest moment about z, the
    /// shape a ship-flight controller is tuned against. Returns (body, controller).
    fn spawn_spinning_ship(app: &mut App, spin: Vec3) -> (Entity, Entity) {
        spawn_spinning_ship_with_acceleration(app, spin, 0.5)
    }

    fn spawn_spinning_ship_with_acceleration(
        app: &mut App,
        spin: Vec3,
        max_angular_acceleration: f32,
    ) -> (Entity, Entity) {
        spawn_ship(app, spin, max_angular_acceleration, 1.0, f32::INFINITY)
    }

    /// The same hull under a sustained-speed limit, which is what a hull with
    /// a structural ceiling flies under.
    fn spawn_limited_ship(
        app: &mut App,
        spin: Vec3,
        max_angular_acceleration: f32,
        sustained_angular_speed: f32,
    ) -> (Entity, Entity) {
        spawn_ship(
            app,
            spin,
            max_angular_acceleration,
            1.0,
            sustained_angular_speed,
        )
    }

    fn spawn_ship(
        app: &mut App,
        spin: Vec3,
        max_angular_acceleration: f32,
        density: f32,
        sustained_angular_speed: f32,
    ) -> (Entity, Entity) {
        let body = app
            .world_mut()
            .spawn((RigidBody::Dynamic, Transform::default()))
            .id();
        for z in [-1.0, 0.0, 1.0] {
            app.world_mut().spawn((
                ChildOf(body),
                Transform::from_xyz(0.0, 0.0, z),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(density),
            ));
        }
        let controller = app
            .world_mut()
            .spawn((
                ChildOf(body),
                PDController {
                    frequency: 4.0,
                    damping_ratio: 4.0,
                    max_angular_acceleration,
                    sustained_angular_speed,
                },
                PDControllerTarget(body),
                Transform::default(),
            ))
            .id();
        // Mass is computed over the first few steps, so the spin must not be imposed
        // until avian has linked colliders and finalized mass properties.
        for _ in 0..4 {
            app.update();
        }
        app.world_mut()
            .entity_mut(body)
            .insert(AngularVelocity(spin));
        (body, controller)
    }

    /// Step the app for `seconds` of simulated time at the 60 Hz manual timestep
    /// `physics_app` installs.
    fn simulate_seconds(app: &mut App, seconds: f32) {
        for _ in 0..(seconds * 60.0) as usize {
            app.update();
        }
    }

    fn spin_rate(app: &App, body: Entity) -> f32 {
        app.world()
            .get::<AngularVelocity>(body)
            .map(|w| w.length())
            .unwrap_or(f32::NAN)
    }

    #[test]
    fn fixed_attitude_convergence_is_inertia_invariant() {
        let mut app = physics_app();
        let (light, light_controller) = spawn_ship(&mut app, Vec3::ZERO, 0.5, 1.0, f32::INFINITY);
        let (heavy, heavy_controller) = spawn_ship(&mut app, Vec3::ZERO, 0.5, 10.0, f32::INFINITY);
        let target = Quat::from_rotation_y(core::f32::consts::FRAC_PI_2);
        app.world_mut()
            .entity_mut(light_controller)
            .insert(PDControllerInput(target));
        app.world_mut()
            .entity_mut(heavy_controller)
            .insert(PDControllerInput(target));

        let principal = |app: &App, body| {
            app.world()
                .get::<ComputedAngularInertia>(body)
                .unwrap()
                .principal_angular_inertia_with_local_frame()
                .0
                .max_element()
        };
        let ratio = principal(&app, heavy) / principal(&app, light);
        assert!(
            (ratio - 10.0).abs() < 0.1,
            "test hull inertia ratio is {ratio}"
        );

        let mut settled: [Option<u32>; 2] = [None, None];
        for tick in 1..=1800u32 {
            app.update();
            for (index, body) in [light, heavy].into_iter().enumerate() {
                let rotation = app.world().get::<Rotation>(body).unwrap().0;
                let speed = app.world().get::<AngularVelocity>(body).unwrap().length();
                if rotation.angle_between(target) < 0.05 && speed < 0.1 {
                    settled[index].get_or_insert(tick);
                }
            }
            if settled.iter().all(Option::is_some) {
                break;
            }
        }

        let [Some(light_ticks), Some(heavy_ticks)] = settled else {
            panic!("both hulls must settle, got {settled:?}");
        };
        assert!(
            light_ticks.abs_diff(heavy_ticks) as f32 <= light_ticks as f32 * 0.15,
            "10x inertia changed convergence: light {light_ticks} ticks, heavy {heavy_ticks} ticks"
        );
    }

    /// Pure damper: the command tracks the attitude every tick, so the PD is
    /// damping-only. A fast roll about the long axis must despin.
    #[test]
    fn fast_roll_despins_when_command_tracks_attitude() {
        // 1.5 rad/s is past what one tick of acceleration authority can
        // cancel, so the damper must converge over many ticks.
        let mut app = physics_app();
        let (body, controller) = spawn_spinning_ship(&mut app, Vec3::new(0.0, 0.0, 1.5));
        app.world_mut().entity_mut(controller).insert(TrackAttitude);

        simulate_seconds(&mut app, 10.0);

        let rate = spin_rate(&app, body);
        assert!(
            rate < 0.1,
            "pure damper should despin a 1.5 rad/s roll, still at {rate} rad/s"
        );
    }

    /// The release scenario: the command freezes at the release attitude while
    /// the body still rolls fast. The PD must bring it back to rest instead of
    /// corkscrewing forever.
    #[test]
    fn fast_roll_despins_with_frozen_command() {
        // Same 1.5 rad/s "fast" roll as
        // `fast_roll_despins_when_command_tracks_attitude`; the two differ only
        // in whether the command tracks, so the rate must stay in step.
        let mut app = physics_app();
        let (body, _) = spawn_spinning_ship(&mut app, Vec3::new(0.0, 0.0, 1.5));

        simulate_seconds(&mut app, 30.0);

        let rate = spin_rate(&app, body);
        assert!(
            rate < 0.1,
            "frozen command should still despin a 1.5 rad/s roll, still at {rate} rad/s"
        );
    }

    /// A skewed body: the cuboids' transverse offsets vary along z, so the
    /// products of inertia are nonzero and the principal local frame is NOT
    /// identity - this is the only integration case that runs the frame
    /// composition the fix corrected. Spinning about world z is off-principal
    /// here. Note a pure damper drains energy under either composition order
    /// (any quaternion sandwich yields an SPD tensor), so this is an
    /// end-to-end sanity check for the non-identity-frame path, not the
    /// discriminating regression test - that is
    /// `pure_damper_matches_closed_form_with_both_frames`.
    #[test]
    fn off_principal_spin_despins_on_a_skewed_body() {
        let mut app = physics_app();
        let body = app
            .world_mut()
            .spawn((RigidBody::Dynamic, Transform::default()))
            .id();
        for (x, y, z) in [(0.5, 0.4, -1.0), (0.0, 0.0, 0.0), (-0.5, -0.4, 1.0)] {
            app.world_mut().spawn((
                ChildOf(body),
                Transform::from_xyz(x, y, z),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ));
        }
        app.world_mut().spawn((
            ChildOf(body),
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_angular_acceleration: 0.5,
                sustained_angular_speed: f32::INFINITY,
            },
            PDControllerTarget(body),
            Transform::default(),
        ));
        for _ in 0..4 {
            app.update();
        }
        let (_, local_frame) = app
            .world()
            .get::<ComputedAngularInertia>(body)
            .unwrap()
            .principal_angular_inertia_with_local_frame();
        assert!(
            local_frame.angle_between(Quat::IDENTITY) > 0.1,
            "the skewed body must actually have a non-identity principal frame, got {local_frame:?}"
        );
        app.world_mut()
            .entity_mut(body)
            .insert(AngularVelocity(Vec3::new(0.0, 0.0, 1.5)));

        simulate_seconds(&mut app, 30.0);

        let rate = spin_rate(&app, body);
        assert!(
            rate < 0.1,
            "a fast off-principal spin should despin on a skewed body, still at {rate} rad/s"
        );
    }

    /// Saturation coverage for a fast released roll.
    #[test]
    fn fast_roll_despins_under_saturating_acceleration_authority() {
        let mut app = physics_app();
        let (body, _) =
            spawn_spinning_ship_with_acceleration(&mut app, Vec3::new(0.0, 0.0, 1.5), 1.25);

        simulate_seconds(&mut app, 30.0);

        let rate = spin_rate(&app, body);
        assert!(
            rate < 0.1,
            "saturating acceleration authority must not limit-cycle: still at {rate} rad/s"
        );
    }

    /// The speed a body may hold, for the held-turn pair below. Well under the
    /// rate the gains alone settle at, so the two answers cannot be confused.
    const HELD_TURN_SUSTAINED: f32 = 0.5;

    /// End to end, and the reason the limit exists: a command that never stops
    /// asking for more turn cannot drive a body past the speed it may hold.
    ///
    /// The unlimited twin is the control. Both bodies fly the same held
    /// demand under the same authority; with no sustained speed the loop
    /// coasts up to its own equilibrium (`kp * angle / kd`, which is pi rad/s
    /// on this tuning) and keeps it. That is the behaviour a scalar
    /// acceleration cap alone gives you, and it is what this rule replaces.
    #[test]
    fn a_held_command_cannot_drive_a_body_past_the_speed_it_may_hold() {
        let mut app = physics_app();
        let (limited, limited_controller) =
            spawn_limited_ship(&mut app, Vec3::ZERO, 0.5, HELD_TURN_SUSTAINED);
        let (free, free_controller) = spawn_limited_ship(&mut app, Vec3::ZERO, 0.5, f32::INFINITY);
        app.world_mut()
            .entity_mut(limited_controller)
            .insert(HoldTurn);
        app.world_mut().entity_mut(free_controller).insert(HoldTurn);

        simulate_seconds(&mut app, 20.0);

        let held = spin_rate(&app, limited);
        let unheld = spin_rate(&app, free);
        assert!(
            held > HELD_TURN_SUSTAINED * 0.9,
            "the limited body must reach the speed it IS allowed to hold, got {held} rad/s"
        );
        assert!(
            held < HELD_TURN_SUSTAINED + 0.05,
            "a held command drove the body to {held} rad/s past a {HELD_TURN_SUSTAINED} rad/s limit"
        );
        assert!(
            unheld > HELD_TURN_SUSTAINED * 2.0,
            "the unlimited control must run away as it always did, got {unheld} rad/s; \
             otherwise the limited body proves nothing"
        );
    }

    /// A ship dies section by section, so a controller outlives its hull by a
    /// tick or two. The loop must hand the next body a clean slate rather than
    /// the torque it was pushing into the one that is gone - and it must do it
    /// without narrating the state at 60 Hz for the rest of the scene.
    #[test]
    fn a_controller_whose_body_is_gone_holds_no_torque() {
        let mut app = physics_app();
        let (body, controller) = spawn_spinning_ship(&mut app, Vec3::new(0.0, 0.0, 2.0));

        // Let the loop build a real correcting torque against the spin.
        simulate_seconds(&mut app, 0.2);
        let torque = **app.world().get::<PDControllerOutput>(controller).unwrap();
        assert!(
            torque.length() > 0.0,
            "the fixture must be pushing a torque before the hull dies, got {torque:?}"
        );

        // Orphan the controller first: a despawn takes its descendants with it,
        // and the case under test is the one where it does not.
        app.world_mut().entity_mut(controller).remove::<ChildOf>();
        app.world_mut().entity_mut(body).despawn();
        app.update();

        let torque = **app.world().get::<PDControllerOutput>(controller).unwrap();
        assert_eq!(
            torque,
            Vec3::ZERO,
            "a controller with no body must hold no torque, still at {torque:?}"
        );
    }

    /// Control case documenting the known-good regime: a moderate spin damps
    /// under a frozen command today.
    #[test]
    fn moderate_spin_despins_with_frozen_command() {
        // 0.7 rad/s is the moderate-spin control case below the fast-roll
        // saturation regime.
        let mut app = physics_app();
        let (body, _) = spawn_spinning_ship(&mut app, Vec3::new(0.0, 0.0, 0.7));

        simulate_seconds(&mut app, 30.0);

        let rate = spin_rate(&app, body);
        assert!(
            rate < 0.1,
            "a 0.7 rad/s roll should despin under a frozen command, still at {rate} rad/s"
        );
    }
}
