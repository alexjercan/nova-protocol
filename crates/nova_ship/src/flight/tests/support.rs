//! Shared rigs for the flight integration tests: the harness apps, the
//! ships they fly, and the accessors every module reads state through.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::{
    prelude::*,
    test_support::{settle, unfinished_integrity_physics_app},
};

use crate::{
    flight::{
        autopilot::{autopilot_system, on_autopilot_removed_cool_engines},
        manual::{decay_player_rcs_intent, manual_burn_system, rcs_burn_system},
        order::drive_scripted_align,
        state::remove_maneuver_telemetry,
        NovaFlightSystems,
    },
    prelude::*,
    sections::{
        controller_section::{
            sync_controller_section_forces, update_controller_section_rotation_input,
            update_controller_stack_tuning,
        },
        hull_radius::publish_hull_radius,
        thruster_section::thruster_impulse_system,
    },
};
pub(super) fn flight_app() -> App {
    let mut app = unfinished_flight_app();
    app.finish();
    app
}

/// The flight harness plus the real gravity layer, for the ORBIT tests:
/// wells actually pull, ship roots opt in through the plugin's observer.
pub(super) fn orbit_app() -> App {
    let mut app = unfinished_flight_app();
    app.add_plugins(nova_gameplay::gravity::NovaGravityPlugin);
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
            // The production pass that publishes each hull's own size, and the
            // one that turns it into an attitude ceiling. First in the chain:
            // everything below reads what they write, and a rig that skipped
            // them would fly on a ceiling nobody derived.
            publish_hull_radius,
            update_controller_stack_tuning,
            autopilot_system,
            // Production order: the two never coexist on one ship, but the
            // rig chains them the same way so a test cannot depend on an
            // ordering the game does not have.
            drive_scripted_align,
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
        (sync_controller_section_forces, thruster_impulse_system).in_set(SpaceshipSectionSystems),
    );
    app
}

/// A minimal flyable ship: hull collider, a rear main thruster (thrusting
/// toward -Z, the ship's forward), and a live controller section with a
/// real PD so the autopilot can actually swing the hull.
pub(super) fn spawn_ship(app: &mut App) -> (Entity, Entity, Entity) {
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
                // Deliberately high authority: the generic rig pins the
                // command rate at its ceiling so maneuver tests exercise
                // outcomes, not production handling. This controller carries no
                // `ControllerSectionTuning`, so the stack pass skips it and the
                // value stands. Model-derived coverage lives in the
                // scratch-scenario rig and in `stacking`.
                max_angular_acceleration: 100.0,
                sustained_angular_speed: f32::INFINITY,
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
/// their terminal via RCS instead. Disabling RCS here keeps them exercising the
/// behavior they were written for - the same opt-out the mainline campaign uses
/// while RCS is off pending rework.
pub(super) fn withhold_rcs(app: &mut App, ship: Entity) {
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
pub(super) fn spawn_extra_thruster(
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

/// A ship that grants RCS, carrying a (zero) intent and a per-hull cap,
/// with its mass finalized. Returns (ship, controller).
pub(super) fn spawn_rcs_ship(app: &mut App, cap: f32) -> (Entity, Entity) {
    let (ship, _thruster, controller) = spawn_ship(app);
    app.world_mut()
        .entity_mut(ship)
        .insert((RcsIntent::default(), RcsSpeedCap(cap)));
    settle(app);
    (ship, controller)
}

pub(super) fn set_rcs(app: &mut App, ship: Entity, intent: Vec3) {
    app.world_mut().get_mut::<RcsIntent>(ship).unwrap().0 = intent;
}

pub(super) fn angular_speed_of(app: &App, ship: Entity) -> f32 {
    app.world().get::<AngularVelocity>(ship).unwrap().0.length()
}

pub(super) fn forward_of(app: &App, ship: Entity) -> Vec3 {
    app.world()
        .get::<Rotation>(ship)
        .unwrap()
        .mul_vec3(Vec3::NEG_Z)
}

pub(super) fn velocity_of(app: &App, ship: Entity) -> Vec3 {
    **app.world().get::<LinearVelocity>(ship).unwrap()
}

pub(super) fn position_of(app: &App, ship: Entity) -> Vec3 {
    **app.world().get::<Position>(ship).unwrap()
}

pub(super) fn run(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

/// A damage-shifted hull with a single main drive: the ballast block far
/// off the centerline stands in for the surviving half of a ship that
/// lost a side section, so the COM sits well off the lone drive's thrust
/// line and a burn torques the hull past what the PD can hold. With
/// `with_lateral` a single sideways thruster survives aft of the COM -
/// the counter-torque candidate. Returns the ship and the lateral.
pub(super) fn spawn_damage_shifted_single_drive(
    app: &mut App,
    with_lateral: bool,
) -> (Entity, Entity) {
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
            // Shipped-scale authority - the 1-1-1's 8 G over a 15 m arm - so
            // the wrench allocation is exercised against a ceiling a real hull
            // has. Written out rather than derived: this rig's controller
            // carries no `ControllerSectionTuning`, so the stack pass leaves
            // it alone, and its ballast puts the real arm somewhere else.
            max_angular_acceleration: 5.232,
            sustained_angular_speed: f32::INFINITY,
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

/// A bare hull for the impulse-frame tests: no FlightIntent (the manual
/// burn layer would zero hand-set throttles) and no controller (a PD
/// would damp away the very spin the test measures). The only torque
/// source left is the impulse system's application point.
pub(super) fn spawn_uncontrolled_dumbbell_with_com_lateral(app: &mut App) -> (Entity, Entity) {
    let ship = app
        .world_mut()
        // TransformInterpolation matches production ships
        // (base_scenario_object): PostUpdate then propagates the EASED pose, so
        // the stale GlobalTransform trails raw physics on every tick, not just
        // on the 64-vs-60 Hz double-tick frames.
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

/// The sanity rock, on rails at the origin, pulling for real.
pub(super) fn spawn_orbit_well(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            RigidBody::Static,
            Transform::default(),
            nova_gameplay::gravity::GravityWell::from_mass(
                1200.0,
                20.0,
                &GravitySettings::default(),
            ),
        ))
        .id()
}

/// A STRONG well, like the menu planetoid: `mu = 43350` on a ~85u geometric
/// radius, so at an r=140 orbit the local gravity accel `mu/r^2 ~= 2.2 u/s^2`
/// is well over the RCS authority threshold (`rcs_accel * 0.15 ~= 0.74`). The
/// RCS fine-adjust cannot counter that inward pull.
pub(super) fn spawn_strong_well(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            RigidBody::Static,
            Transform::default(),
            nova_gameplay::gravity::GravityWell::from_mass(
                43_350.0,
                85.0,
                &GravitySettings::default(),
            ),
        ))
        .id()
}

/// Shipped 5-section player geometry (same as
/// `hold_reverse_decel_from_300_keeps_the_hull_steady`).
pub(super) fn diag_ship(app: &mut App) -> (Entity, Entity) {
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
            // 8 G over this rig's 25 m arm - five sections along z, so the
            // furthest face is 2.5 u out.
            max_angular_acceleration: 3.139,
            sustained_angular_speed: f32::INFINITY,
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
