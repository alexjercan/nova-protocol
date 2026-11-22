//! A ship root with no live flight computer lets its tumble settle.
//!
//! Only a live flight computer turns a hull: its PD holds the commanded
//! attitude, and with none nothing applies attitude torque, so a wreck kept
//! the spin it died with for good and no port on it could be docked. This
//! rule halves that spin every [`FlightSettings::dead_spin_half_life`]
//! seconds. Spin only: the hull keeps its drift, which MATCH already handles.
//!
//! The set is every ship root with no live flight computer that is not
//! docked: a neutralized ship that lost its computer, a spawned derelict, a
//! dead station and the player's own hull after its last computer dies.
//! Rocks, wreck fragments and explode pieces are no ship roots and keep their
//! tumble. A docked pair is turned by its driver's computer, so neither root
//! of an assembly is settled.
//!
//! A frozen hull does not settle while it stands frozen, and the thaw pays it
//! nothing for that time. The thaw catches up stores only (the RCS magazine,
//! a station's PDC reserve, a trader's hold); motion resumes as it froze, the
//! drift and the neutralized brake included, and this rule is motion.

use avian3d::prelude::*;
use bevy::{ecs::entity::EntityHashSet, prelude::*};
use nova_gameplay::prelude::*;

use super::{capability::LiveFlightComputers, FlightSettings};
use crate::prelude::*;

/// `spin` after `seconds` of settling at `half_life`. Exact for any step:
/// two steps of `dt` give the same spin as one step of `2 * dt`.
pub(super) fn settled_spin(spin: Vec3, seconds: f32, half_life: f32) -> Vec3 {
    spin * 0.5_f32.powf(seconds / half_life)
}

/// Whether `half_life` is a usable settle time: finite and above zero.
fn is_valid_half_life(half_life: f32) -> bool {
    half_life.is_finite() && half_life > 0.0
}

/// Settle the spin of every undocked ship root that has no live flight
/// computer. A bad [`FlightSettings::dead_spin_half_life`] settles nothing
/// and logs one error until the value is good again.
#[expect(
    clippy::type_complexity,
    reason = "one query over undocked ship roots and their spin"
)]
pub(super) fn settle_dead_hull_spin(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    mut reported: Local<bool>,
    mut q_root: Query<
        (Entity, &mut AngularVelocity),
        (
            With<SpaceshipRootMarker>,
            Without<DockedShip>,
            Without<DockedStation>,
        ),
    >,
    q_computer: LiveFlightComputers,
) {
    let half_life = settings.dead_spin_half_life;
    if !is_valid_half_life(half_life) {
        if !*reported {
            error!(
                "settle_dead_hull_spin: FlightSettings::dead_spin_half_life is {half_life}, not \
                 a finite number of seconds above zero; no hull settles its spin"
            );
            *reported = true;
        }
        return;
    }
    *reported = false;
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let flown: EntityHashSet = q_computer.iter().map(|(_, &ChildOf(root))| root).collect();
    for (root, mut spin) in &mut q_root {
        if spin.0 == Vec3::ZERO || flown.contains(&root) {
            continue;
        }
        spin.0 = settled_spin(spin.0, dt, half_life);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };

    use bevy::log::{
        tracing::{self, Level},
        tracing_subscriber::{layer::SubscriberExt, Layer},
    };
    use nova_events::prelude::EntityId;
    use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

    use super::*;
    use crate::flight::NovaFlightSystems;

    /// The capture gate's spin limit, deg/s.
    const DOCK_GATE_DPS: f32 = 5.0;

    /// A physics app with the settle wired in the flight set, as production
    /// wires it, and with docking when `docking` asks for it.
    fn settle_app(docking: bool) -> App {
        let mut app = unfinished_integrity_physics_app();
        app.init_resource::<FlightSettings>();
        app.add_systems(FixedUpdate, settle_dead_hull_spin.in_set(NovaFlightSystems));
        if docking {
            app.add_plugins(crate::sections::docking_section::DockingSectionPlugin {
                render: false,
            });
        }
        app.finish();
        app
    }

    /// A one-cell hull at `at`.
    fn spawn_hull(app: &mut App, at: Vec3, rotation: Quat) -> Entity {
        let root = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                RigidBody::Dynamic,
                Transform::from_translation(at).with_rotation(rotation),
            ))
            .id();
        app.world_mut().spawn((
            ChildOf(root),
            Transform::default(),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        root
    }

    /// A live flight computer on `root`: the marker and the PD that make it
    /// one. Nothing wires its torque, so any spin the hull keeps is the
    /// settle leaving it alone.
    fn live_computer(app: &mut App, root: Entity) {
        app.world_mut().spawn((
            ChildOf(root),
            ControllerSectionMarker,
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_angular_acceleration: 40.0,
                sustained_angular_speed: f32::INFINITY,
            },
        ));
    }

    fn spin_dps(app: &App, root: Entity) -> f32 {
        app.world()
            .get::<AngularVelocity>(root)
            .unwrap()
            .0
            .length()
            .to_degrees()
    }

    fn run_for(app: &mut App, seconds: f32) {
        let end = app.world().resource::<Time<Fixed>>().elapsed_secs() + seconds;
        while app.world().resource::<Time<Fixed>>().elapsed_secs() < end {
            app.update();
        }
    }

    #[test]
    fn a_settle_step_is_exact_for_any_step_length() {
        let spin = Vec3::new(0.3, -1.1, 0.7);
        let whole = settled_spin(spin, 2.0, 8.0);
        let halves = settled_spin(settled_spin(spin, 1.0, 8.0), 1.0, 8.0);
        assert!((whole - halves).length() < 1e-6);
        assert!((settled_spin(spin, 8.0, 8.0) - spin * 0.5).length() < 1e-6);
    }

    #[test]
    fn a_hull_with_no_live_flight_computer_settles_its_spin_under_the_dock_gate_and_keeps_drifting()
    {
        let mut app = settle_app(false);
        let hull = spawn_hull(&mut app, Vec3::ZERO, Quat::IDENTITY);
        settle(&mut app);
        let drift = Vec3::new(1.2, 0.0, -0.4);
        app.world_mut().entity_mut(hull).insert((
            LinearVelocity(drift),
            AngularVelocity(Vec3::Y * 60.0_f32.to_radians()),
        ));

        run_for(&mut app, 10.0);
        let at_10 = spin_dps(&app, hull);
        run_for(&mut app, 20.0);
        let at_30 = spin_dps(&app, hull);

        // 60 * 0.5^(10 / 8) = 25.2 deg/s, 60 * 0.5^(30 / 8) = 4.5 deg/s.
        assert!(
            (at_10 - 25.2).abs() < 0.5,
            "the spin halves every 8 s: {at_10} deg/s at 10 s"
        );
        assert!(
            at_30 < DOCK_GATE_DPS,
            "under the dock gate by 30 s: {at_30} deg/s"
        );
        let velocity = app.world().get::<LinearVelocity>(hull).unwrap().0;
        assert!(
            (velocity - drift).length() < 1e-4,
            "spin only: the drift is untouched, {velocity} against {drift}"
        );
    }

    #[test]
    fn a_hull_with_a_live_flight_computer_is_left_to_its_attitude_hold() {
        let mut app = settle_app(false);
        let hull = spawn_hull(&mut app, Vec3::ZERO, Quat::IDENTITY);
        live_computer(&mut app, hull);
        settle(&mut app);
        app.world_mut()
            .entity_mut(hull)
            .insert(AngularVelocity(Vec3::Y * 60.0_f32.to_radians()));

        run_for(&mut app, 10.0);

        assert!(
            (spin_dps(&app, hull) - 60.0).abs() < 0.01,
            "the computer owns the attitude: {} deg/s",
            spin_dps(&app, hull)
        );
    }

    #[test]
    fn a_docked_dead_hull_is_turned_by_its_driver_not_settled() {
        let mut app = settle_app(true);
        // Two computer-less one-cell hulls nose to nose down Z, half a cell
        // between the two port faces.
        let first = spawn_hull(&mut app, Vec3::ZERO, Quat::IDENTITY);
        let second = spawn_hull(
            &mut app,
            Vec3::NEG_Z * 1.5,
            Quat::from_rotation_y(std::f32::consts::PI),
        );
        for ship in [first, second] {
            app.world_mut().spawn((
                ChildOf(ship),
                EntityId("fore".to_string()),
                Transform::default(),
                docking_section(DockingSectionConfig::default()),
            ));
        }
        settle(&mut app);
        app.world_mut().trigger(DockingConnectionRequest {
            entity: first,
            target: second,
        });
        settle(&mut app);
        assert!(app.world().get::<DockedShip>(first).is_some());
        assert!(app.world().get::<DockedShip>(second).is_some());
        // A spin about the line through both centres: one rigid motion of
        // the pair, so the joint holds it as it is.
        let spin = Vec3::Z * 60.0_f32.to_radians();
        for ship in [first, second] {
            app.world_mut()
                .entity_mut(ship)
                .insert(AngularVelocity(spin));
        }

        run_for(&mut app, 10.0);

        for ship in [first, second] {
            assert!(
                (spin_dps(&app, ship) - 60.0).abs() < 1.0,
                "a docked root is not settled: {} deg/s",
                spin_dps(&app, ship)
            );
        }
    }

    /// Counts the error events raised on this thread.
    struct ErrorCount(Arc<AtomicUsize>);

    impl<S: tracing::Subscriber> Layer<S> for ErrorCount {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _: bevy::log::tracing_subscriber::layer::Context<'_, S>,
        ) {
            if *event.metadata().level() == Level::ERROR {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    #[test]
    fn a_bad_half_life_settles_nothing_and_logs_once() {
        for half_life in [0.0, -8.0, f32::NAN, f32::INFINITY] {
            let mut world = World::new();
            let mut time = Time::<()>::default();
            time.advance_by(Duration::from_secs_f32(1.0 / 64.0));
            world.insert_resource(time);
            world.insert_resource(FlightSettings {
                dead_spin_half_life: half_life,
                ..default()
            });
            let spin = Vec3::Y * 60.0_f32.to_radians();
            let hull = world
                .spawn((SpaceshipRootMarker, AngularVelocity(spin)))
                .id();
            let system = world.register_system(settle_dead_hull_spin);
            let errors = Arc::new(AtomicUsize::new(0));
            let subscriber =
                bevy::log::tracing_subscriber::registry().with(ErrorCount(Arc::clone(&errors)));

            tracing::subscriber::with_default(subscriber, || {
                for _ in 0..3 {
                    world.run_system(system).unwrap();
                }
            });

            assert_eq!(
                world.get::<AngularVelocity>(hull).unwrap().0,
                spin,
                "half-life {half_life}: nothing settles"
            );
            assert_eq!(
                errors.load(Ordering::SeqCst),
                1,
                "half-life {half_life}: one error, not one per tick"
            );
        }
    }
}
