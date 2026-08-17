//! Player intent on the ship: the mouse-driven rotation command and the
//! turret/torpedo target feeds derived from the active lock.

use avian3d::prelude::*;
use bevy::prelude::*;
#[cfg(test)]
use bevy_enhanced_input::prelude::*;
use nova_gameplay::prelude::*;

#[cfg(test)]
use super::flight_rig::{flight_input_rig, FlightInputMarker};
use crate::prelude::*;

/// System that takes the point rotation output from the chase camera and applies it to the
/// controller of the player's spaceship.
///
/// Gated on `Without<Autopilot>`: while a maneuver is engaged the autopilot
/// owns the rotation command, and the mouse - which keeps driving the camera
/// rig - becomes camera-only free-look for free.
pub(super) fn update_controller_target_rotation_torque(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    point_rotation: Single<
        &PointRotationOutput,
        (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipCameraNormalInputMarker>,
        ),
    >,
    mut q_controller: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        With<ControllerSectionMarker>,
    >,
    spaceship: Single<
        Entity,
        (
            With<SpaceshipRootMarker>,
            With<PlayerSpaceshipMarker>,
            Without<Autopilot>,
            // RCS fine-adjust repurposes the mouse to translation and freezes
            // the heading, exactly as an engaged maneuver does.
            Without<RcsActive>,
        ),
    >,
    q_computer: Query<
        (&PDController, &ChildOf),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
) {
    let point_rotation = point_rotation.into_inner();
    let spaceship = spaceship.into_inner();
    // NOTE: slew the command toward the camera instead of jumping - a mouse 180 fed
    // to the PD in one step drives it into saturation where its damping is
    // swamped and the hull limit-cycles. The camera stays instant; the hull's
    // commanded target ramps at the computer's acceleration-derived turn rate,
    // shared with autopilot planning. With no live computer the command FREEZES:
    // nothing consumes it, and slewing a dead helm would drift it so a later
    // re-activation snaps the hull.
    let Some(turn_rate) = crate::flight::ship_turn_rate(
        q_computer
            .iter()
            .filter(|(_, &ChildOf(parent))| parent == spaceship)
            .map(|(pd, _)| pd.max_angular_acceleration),
        &settings,
    ) else {
        return;
    };
    let max_step = turn_rate * time.delta_secs();

    for (mut controller, _) in q_controller
        .iter_mut()
        .filter(|(_, ChildOf(c_parent))| *c_parent == spaceship)
    {
        **controller = crate::flight::slew_rotation(**controller, **point_rotation, max_step);
    }
}

/// System that takes the point rotation output from the chase camera and applies it to the
/// turret target input of the player's spaceship.
///
/// Skips the mounts the Flight Computer is working. That is the ONE place the
/// ownership precedence touches the player's own aim: an idle battery with
/// nothing inbound still follows the crosshair exactly as it always has, and a
/// mount only stops following it once the computer has a torpedo to put on it.
pub(super) fn update_turret_target_input(
    point_rotation: Single<
        &PointRotationOutput,
        (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipCameraTurretInputMarker>,
        ),
    >,
    mut q_turret: Query<
        (
            &mut TurretSectionTargetInput,
            &mut TurretSectionTargetVelocity,
            &ChildOf,
            Option<&PointDefenseMount>,
            Option<&TurretDefenseTarget>,
        ),
        With<TurretSectionMarker>,
    >,
    spaceship: Single<
        (
            &Transform,
            Option<&ComputedCenterOfMass>,
            Entity,
            Option<&CombatLock>,
            Option<&ComponentLock>,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_lock_target: Query<(
        &Transform,
        Option<&ComputedCenterOfMass>,
        Option<&LinearVelocity>,
    )>,
    q_section_position: Query<&GlobalTransform, With<SectionMarker>>,
) {
    let point_rotation = point_rotation.into_inner();
    let (transform, com, spaceship, lock, component) = spaceship.into_inner();
    let lock = lock.and_then(|lock| lock.0);
    let component_section = component.and_then(|component| component.section);

    // Base the aim ray on the live structure so the turret aim point matches
    // the COM-anchored camera crosshair after losing sections.
    let position = live_structure_anchor(transform, com);

    // Three-tier auto-fire feed: the fine-locked section, else the locked
    // ship's live structure, else the camera ray as always. Lock tiers carry
    // the lock root's velocity so lead_intercept_point computes a real
    // intercept; the ray tier aims at a commanded point, not a body, so its
    // velocity is zero. A dead section or lock falls through to the next tier
    // the same frame (the targeting systems clear the stale state on their
    // next run).
    let lock_tier = lock.and_then(|target| {
        q_lock_target
            .get(target)
            .ok()
            .map(|(target_transform, target_com, target_velocity)| {
                (
                    live_structure_anchor(target_transform, target_com),
                    target_velocity
                        .map(|velocity| **velocity)
                        .unwrap_or(Vec3::ZERO),
                )
            })
    });
    let component_tier = component_section.and_then(|section| {
        let section_position = q_section_position.get(section).ok()?;
        let (_, lock_velocity) = lock_tier?;
        Some((section_position.translation(), lock_velocity))
    });
    let ray_tier = {
        let forward = **point_rotation * Vec3::NEG_Z;
        (position + forward * 100.0, Vec3::ZERO)
    };
    // LOCK-WINS routing (flipping the manual-wins knob): a
    // combat lock holds the turrets even while RAISED - moving the cursor
    // must not pull them off the target; tap CTRL (clearing the lock) is the
    // explicit road back to manual. With NO lock, the ray tier IS the raised
    // manual aim, so no stance special-case remains - the pure three-tier
    // feed.
    let (target_point, target_velocity) = component_tier.or(lock_tier).unwrap_or(ray_tier);

    for (mut turret, mut velocity, _, mount, assignment) in q_turret
        .iter_mut()
        .filter(|(_, _, ChildOf(t_parent), _, _)| *t_parent == spaceship)
    {
        if flight_computer_works(mount, assignment) {
            continue;
        }
        **turret = Some(target_point);
        **velocity = target_velocity;
    }
}

/// Commit each freshly launched torpedo to its launch-time target.
///
/// A torpedo's targeting decision is made exactly once, right after launch:
/// whatever the crosshair has locked at that moment becomes the torpedo's target
/// for life (`TorpedoTargetChosen` marks the decision as made). No lock means a
/// dumb-fire shot that never acquires anything mid-flight - so, e.g., bullets
/// fired past a loitering torpedo are not picked up as targets, and a torpedo
/// whose target died (link dropped by `update_target_position`, position frozen)
/// is not re-assigned to whatever the player locks next.
pub(super) fn update_torpedo_target_input(
    mut commands: Commands,
    q_torpedo: Query<
        (Entity, &ProjectileOwner),
        (
            With<TorpedoProjectileMarker>,
            Without<TorpedoTargetEntity>,
            Without<TorpedoTargetChosen>,
        ),
    >,
    spaceship: Single<
        (Entity, Option<&CombatLock>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    let (spaceship, lock) = spaceship.into_inner();
    let lock = lock.and_then(|lock| lock.0);

    for (torpedo, owner) in &q_torpedo {
        if **owner != spaceship {
            continue;
        }

        debug!(
            "update_torpedo_target_input: committing torpedo {:?} to target {:?}",
            torpedo, lock
        );

        let mut torpedo_commands = commands.entity(torpedo);
        torpedo_commands.insert(TorpedoTargetChosen);
        if let Some(target_entity) = lock {
            torpedo_commands.insert(TorpedoTargetEntity(target_entity));
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn no_lock_does_not_despawn_untargeted_torpedo() {
        // Regression: with no current lock, an un-targeted torpedo (e.g. one whose
        // target just died and had its link dropped) must keep flying, not vanish.
        let mut app = App::new();
        app.add_systems(Update, update_torpedo_target_input);

        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker, CombatLock(None)))
            .id();
        let torpedo = app
            .world_mut()
            .spawn((TorpedoProjectileMarker, ProjectileOwner(ship)))
            .id();

        app.update();

        assert!(
            app.world().entities().contains(torpedo),
            "un-targeted torpedo must survive when there is no lock"
        );
        assert!(
            app.world().get::<TorpedoTargetEntity>(torpedo).is_none(),
            "no target should be assigned when there is no lock"
        );
        assert!(
            app.world().get::<TorpedoTargetChosen>(torpedo).is_some(),
            "the torpedo should be committed to dumb-fire"
        );
    }

    #[test]
    fn lock_assigns_target_to_owned_torpedo() {
        // With a lock, an owned un-targeted torpedo gets the target assigned and
        // is committed to it.
        let mut app = App::new();
        let target = app.world_mut().spawn_empty().id();
        app.add_systems(Update, update_torpedo_target_input);

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                CombatLock(Some(target)),
            ))
            .id();
        let torpedo = app
            .world_mut()
            .spawn((TorpedoProjectileMarker, ProjectileOwner(ship)))
            .id();

        app.update();

        assert_eq!(
            app.world().get::<TorpedoTargetEntity>(torpedo).map(|t| **t),
            Some(target),
            "an owned torpedo should be assigned the locked target"
        );
        assert!(
            app.world().get::<TorpedoTargetChosen>(torpedo).is_some(),
            "the assignment should also commit the torpedo"
        );
    }

    #[test]
    fn dumbfire_torpedo_ignores_later_locks() {
        // THE bullet regression: a torpedo fired with no lock is committed to
        // dumb-fire; a lock appearing later (e.g. the aim cast hitting a bullet
        // fired down the crosshair ray) must not be assigned to it.
        let mut app = App::new();
        app.add_systems(Update, update_torpedo_target_input);

        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker, CombatLock(None)))
            .id();
        let torpedo = app
            .world_mut()
            .spawn((TorpedoProjectileMarker, ProjectileOwner(ship)))
            .id();

        // Frame 1: no lock -> committed dumb-fire.
        app.update();
        assert!(app.world().get::<TorpedoTargetChosen>(torpedo).is_some());

        // A "bullet" gets combat-locked (deliberately) afterwards.
        let bullet = app.world_mut().spawn_empty().id();
        app.world_mut().get_mut::<CombatLock>(ship).unwrap().0 = Some(bullet);

        // Frame 2: the committed torpedo must NOT pick it up.
        app.update();
        assert!(
            app.world().get::<TorpedoTargetEntity>(torpedo).is_none(),
            "a dumb-fired torpedo must never acquire a target mid-flight"
        );
    }

    #[test]
    fn committed_torpedo_does_not_retarget_after_target_loss() {
        // A torpedo whose target died (link removed by update_target_position,
        // position frozen) keeps its commitment: a fresh lock must not re-target it.
        let mut app = App::new();
        let new_target = app.world_mut().spawn_empty().id();
        app.add_systems(Update, update_torpedo_target_input);

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                CombatLock(Some(new_target)),
            ))
            .id();
        // Committed, un-targeted: the post-target-death state.
        let torpedo = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                ProjectileOwner(ship),
                TorpedoTargetChosen,
            ))
            .id();

        app.update();

        assert!(
            app.world().get::<TorpedoTargetEntity>(torpedo).is_none(),
            "a torpedo keeps its first target for life - no re-targeting after loss"
        );
    }

    #[test]
    fn turret_aim_ray_bases_on_the_live_structure_anchor() {
        // COM offset perpendicular to the aim: the ray base must shift with
        // it, or the turret aim point keeps a parallax against the COM-
        // anchored crosshair.
        let mut world = World::new();
        world.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraTurretInputMarker,
            PointRotationOutput(Quat::IDENTITY),
        ));
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
                ComputedCenterOfMass(Vec3::new(2.0, 0.0, 0.0)),
                CombatLock(None),
                ComponentLock::default(),
            ))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
                ChildOf(ship),
            ))
            .id();

        world.run_system_once(update_turret_target_input).unwrap();

        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetInput>()
                .unwrap(),
            Some(Vec3::new(12.0, 0.0, -100.0)),
            "aim ray base = anchor (12,0,0), not the root origin (10,0,0)"
        );
    }

    /// Player + aim rig + one turret, a locked target ship (moving, with a
    /// shifted COM) and one of its sections. Returns (turret, target,
    /// section).
    fn turret_feed_world() -> (World, Entity, Entity, Entity, Entity) {
        let mut world = World::new();
        world.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraTurretInputMarker,
            PointRotationOutput(Quat::IDENTITY),
        ));
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::IDENTITY,
                CombatLock(None),
                ComponentLock::default(),
            ))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
                ChildOf(ship),
            ))
            .id();
        let target = world
            .spawn((
                SpaceshipRootMarker,
                Transform::from_translation(Vec3::new(0.0, 0.0, -200.0)),
                ComputedCenterOfMass(Vec3::new(0.0, 0.0, 2.0)),
                LinearVelocity(Vec3::new(7.0, 0.0, 0.0)),
            ))
            .id();
        let section = world
            .spawn((
                SectionMarker,
                GlobalTransform::from_translation(Vec3::new(1.0, 0.5, -199.0)),
                ChildOf(target),
            ))
            .id();
        world.get_mut::<CombatLock>(ship).unwrap().0 = Some(target);
        (world, ship, turret, target, section)
    }

    fn turret_feed(world: &mut World, turret: Entity) -> (Option<Vec3>, Vec3) {
        world.run_system_once(update_turret_target_input).unwrap();
        let entity = world.entity(turret);
        (
            **entity.get::<TurretSectionTargetInput>().unwrap(),
            **entity.get::<TurretSectionTargetVelocity>().unwrap(),
        )
    }

    #[test]
    fn component_lock_feeds_the_section_position() {
        let (mut world, ship, turret, _, section) = turret_feed_world();
        world.get_mut::<ComponentLock>(ship).unwrap().section = Some(section);

        let (point, velocity) = turret_feed(&mut world, turret);

        assert_eq!(point, Some(Vec3::new(1.0, 0.5, -199.0)));
        assert_eq!(velocity, Vec3::new(7.0, 0.0, 0.0), "lock root velocity");
    }

    #[test]
    fn ship_lock_feeds_the_live_structure_anchor() {
        let (mut world, _ship, turret, _, _) = turret_feed_world();

        let (point, velocity) = turret_feed(&mut world, turret);

        // Anchor = target translation + COM offset (identity rotation).
        assert_eq!(point, Some(Vec3::new(0.0, 0.0, -198.0)));
        assert_eq!(velocity, Vec3::new(7.0, 0.0, 0.0));
    }

    #[test]
    fn no_lock_feeds_the_camera_ray_with_zero_velocity() {
        let (mut world, ship, turret, _, _) = turret_feed_world();
        world.get_mut::<CombatLock>(ship).unwrap().0 = None;

        let (point, velocity) = turret_feed(&mut world, turret);

        assert_eq!(point, Some(Vec3::new(0.0, 0.0, -100.0)));
        assert_eq!(velocity, Vec3::ZERO, "a commanded point has no velocity");
    }

    #[test]
    fn dead_section_falls_through_to_the_ship_lock() {
        let (mut world, ship, turret, _, section) = turret_feed_world();
        world.get_mut::<ComponentLock>(ship).unwrap().section = Some(section);
        world.despawn(section);

        let (point, velocity) = turret_feed(&mut world, turret);

        assert_eq!(point, Some(Vec3::new(0.0, 0.0, -198.0)));
        assert_eq!(velocity, Vec3::new(7.0, 0.0, 0.0));
    }

    #[test]
    fn dead_lock_falls_through_to_the_camera_ray() {
        let (mut world, _ship, turret, target, _) = turret_feed_world();
        world.despawn(target);

        let (point, velocity) = turret_feed(&mut world, turret);

        assert_eq!(point, Some(Vec3::new(0.0, 0.0, -100.0)));
        assert_eq!(velocity, Vec3::ZERO);
    }

    /// D8 capture semantics: the engaged GOTO holds the target captured at
    /// [G]; re-designating the travel lock must NOT re-route the trip.
    #[test]
    fn goto_keeps_the_captured_target_across_re_designation() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let ship = world
            .spawn((
                PlayerSpaceshipMarker,
                TravelLock(Some(a)),
                Autopilot::engage(AutopilotAction::Goto { target: a }),
            ))
            .id();

        // Radar re-designates the travel lock to B mid-flight.
        world.get_mut::<TravelLock>(ship).unwrap().0 = Some(b);

        let autopilot = world.get::<Autopilot>(ship).unwrap();
        assert!(
            matches!(autopilot.action, AutopilotAction::Goto { target } if target == a),
            "the engaged GOTO keeps the target captured at [G]"
        );
    }

    #[test]
    fn the_combat_lock_holds_the_turrets_even_while_raised() {
        // LOCK-WINS routing (flipping the manual-wins knob):
        // while RAISED with a combat lock, moving the cursor must NOT pull
        // the turrets off the target - the lock tiers win. This test fails
        // against the retired manual-wins feed by construction (it asserted
        // the look ray here).
        let (mut world, ship, turret, _, section) = turret_feed_world();
        world.get_mut::<ComponentLock>(ship).unwrap().section = Some(section);
        world.entity_mut(ship).insert(WeaponsRaised(true));

        let (point, velocity) = turret_feed(&mut world, turret);
        assert_eq!(
            point,
            Some(Vec3::new(1.0, 0.5, -199.0)),
            "raised with a lock: the turrets stay on the locked section"
        );
        assert_eq!(
            velocity,
            Vec3::new(7.0, 0.0, 0.0),
            "the lock tier carries the target's lead velocity"
        );

        // Tap-clear (the lock and its fine-lock go away): still raised, the
        // turrets hand over to the cursor - manual gunnery is the NO-LOCK
        // stance now.
        world.get_mut::<CombatLock>(ship).unwrap().0 = None;
        world.get_mut::<ComponentLock>(ship).unwrap().section = None;
        let (point, velocity) = turret_feed(&mut world, turret);
        assert_eq!(
            point,
            Some(Vec3::new(0.0, 0.0, -100.0)),
            "clearing the lock hands the turrets to the look ray"
        );
        assert_eq!(
            velocity,
            Vec3::ZERO,
            "manual aim commands a point, no lead velocity"
        );
    }
}

#[cfg(test)]
mod command_lag_tests {
    // Kept as its own module for its distinct harness (manual time), but
    // named and placed beside `tests` deliberately.
    use core::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    /// The drift guard for [`flight_rig_reserved_sources`]: build the REAL
    /// flight rig and confirm the hand-authored reserved list names exactly the
    /// rig's discrete-button sources - no more, no less. If a future edit adds,
    /// removes or rebinds a flight action, this fails until the reserved list
    /// (and the content lint that reads it) is updated, so `input-mapping-
    /// overlays-flight-rig` cannot silently regress.
    #[test]
    fn flight_rig_reserves_exactly_these_sources() {
        use std::collections::HashSet;

        use bevy::input::InputPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_input_context::<FlightInputMarker>();
        // The context registry finalizes in App::finish; run the lifecycle
        // before spawning the rig, exactly as the production app does.
        app.finish();
        app.cleanup();
        app.update();
        app.world_mut().spawn(flight_input_rig());
        app.update();

        let mut rig_sources: HashSet<InputSource> = HashSet::new();
        let mut q = app.world_mut().query::<&Binding>();
        for binding in q.iter(app.world()) {
            if let Some(source) = binding_source(binding) {
                rig_sources.insert(source);
            }
        }

        let reserved: HashSet<InputSource> = flight_rig_reserved_sources()
            .into_iter()
            .map(|(s, _)| s)
            .collect();

        let missing: Vec<_> = rig_sources.difference(&reserved).collect();
        let extra: Vec<_> = reserved.difference(&rig_sources).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "flight_rig_reserved_sources drifted from the real rig.\n  \
             in the rig but not reserved (add them): {missing:?}\n  \
             reserved but not in the rig (remove them): {extra:?}"
        );
    }

    /// A mouse 180 must NOT reach the rotation command in one frame: the
    /// command slews at the computer's acceleration-derived turn rate, so the
    /// PD tracks a small error instead of saturating.
    #[test]
    fn a_camera_flip_reaches_the_command_over_many_frames() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        app.init_resource::<FlightSettings>();
        app.add_systems(Update, update_controller_target_rotation_torque);

        let target = Quat::from_rotation_y(core::f32::consts::PI);
        app.world_mut().spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraNormalInputMarker,
            PointRotationOutput(target),
        ));
        // The inertia is irrelevant to command slew; it remains on the rig to
        // match the production ship query surface.
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::default(),
                ComputedAngularInertia::new(Vec3::splat(2.3)),
            ))
            .id();
        let controller = app
            .world_mut()
            .spawn((
                ChildOf(ship),
                ControllerSectionMarker,
                PDController {
                    frequency: 4.0,
                    damping_ratio: 4.0,
                    max_angular_acceleration: 10.0,
                },
                ControllerSectionRotationInput::default(),
            ))
            .id();

        // First update has dt = 0; the second advances one real frame.
        app.update();
        app.update();

        let command = **app
            .world()
            .get::<ControllerSectionRotationInput>(controller)
            .unwrap();
        let moved = command.angle_between(Quat::IDENTITY);
        let remaining = command.angle_between(target);
        // One frame advances exactly one slew step of the DERIVED rate - this
        // pins hull_turn_rate's wiring, not just "some" slew.
        let expected =
            crate::flight::hull_turn_rate(10.0, &app.world().resource::<FlightSettings>().clone())
                / 60.0;
        assert!(
            (moved - expected).abs() < expected * 0.15,
            "one frame must advance one acceleration-authority slew step \
             (moved {moved}, expected {expected})"
        );
        assert!(
            remaining > 2.0,
            "a 180 flip must not reach the command in one frame ({remaining} left)"
        );
    }
}
