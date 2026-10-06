//! AI torpedo launches: the per-ship/target in-flight limit, the launch
//! envelope (range band plus rough alignment), and the launch-time target write.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::Meters;
use nova_gameplay::prelude::*;

use super::{guns::ai_line_of_fire_blocked, maneuver::ai_target_anchor};
use crate::prelude::*;

/// Outer edge of the launch envelope. Beyond detection range
/// (AI_ENGAGE_RANGE) but well inside the AI sensor reach
/// (AI_SENSOR_RANGE), so a launch can open the approach on a fight the
/// ship is already committing to. Playtest knob.
///
/// Public because the balance audit's threat envelope IS this number: a copy
/// there would drift from the range the AI actually launches at.
pub const AI_TORPEDO_MAX_RANGE: Meters = Meters(10_000.0);
/// Tactical margin at the inner edge of the envelope, as a multiple of the
/// bay's configured blast radius: a point-blank launch detonates inside the
/// shooter's own blast, which damages everything it reaches. The factor keeps
/// the detonation point - the target - clear of the shooter, with margin for
/// the closure that happens while the torpedo flies. Playtest knob.
///
/// A MARGIN between two hulls, not the whole floor. It is added to both ships'
/// live arms, so the gap it buys is FACE to face; on its own it is measured
/// between two origins, and a carrier 194 m to its own bow keeps most of the
/// margin inside itself.
const AI_TORPEDO_MIN_RANGE_BLAST_FACTOR: f32 = 3.0;
/// Rough hull-alignment gate (cos) on a launch. Deliberately loose - PN
/// guidance does the turning - it exists so launches read as aimed attack
/// runs rather than popping off an orbit tangent pointed away, and so the
/// torpedo does not open its flight turning back through the shooter.
const AI_TORPEDO_ALIGNMENT_COS: f32 = 0.5;

/// The geometric half of the launch decision: the target inside the range
/// band with the hull roughly on the bearing ([`AI_TORPEDO_ALIGNMENT_COS`]).
/// The per-ship gates (behavior state, ship target, in-flight limit) live in
/// the calling system. Pure for unit testing.
///
/// The inner edge is a FACE gap. `to_target` runs anchor to anchor, so the
/// floor sums what each hull spends on itself - `own_arm` and `target_arm`,
/// both live [`HullRadius`] values - and the tactical margin
/// (`blast_radius * `[`AI_TORPEDO_MIN_RANGE_BLAST_FACTOR`]) sits between them.
/// Taking the margin alone made the floor mean something different for every
/// hull: a carrier reaches 194 m out of its own anchor, so 900 m of margin
/// bought it 700 m of clear space at one end and less than that at the other.
fn ai_torpedo_envelope(
    to_target: Vec3,
    forward: Vec3,
    blast_radius: f32,
    own_arm: f32,
    target_arm: f32,
) -> bool {
    let distance = to_target.length();
    let floor = own_arm + target_arm + blast_radius * AI_TORPEDO_MIN_RANGE_BLAST_FACTOR;
    if distance <= f32::EPSILON || distance < floor || distance > AI_TORPEDO_MAX_RANGE.to_engine() {
        return false;
    }
    forward.dot(to_target / distance) > AI_TORPEDO_ALIGNMENT_COS
}

/// Pull each AI ship's torpedo triggers: write [`TorpedoSectionInput`] on
/// its bays when the launch envelope is open. The per-ship gates: an
/// Engage-like state - Evade excluded, a jinking hull is no launch
/// platform (Retreat inherits, per its stub) - and a SHIP target: hostile
/// torpedoes are the guns' job (point defense), not worth a bay's ordnance.
/// Per ship: the number of torpedoes in flight at this target stays below
/// the number of working bays on this ship, and only one bay claims a launch
/// each frame. Per bay: active with stock, the envelope
/// ([`ai_torpedo_envelope`]) open from the ship's anchor, and line of fire
/// clear ([`ai_line_of_fire_blocked`]) and its native spawner ready. The
/// bay's own reload and muzzle door still apply.
#[expect(
    clippy::type_complexity,
    reason = "one query per torpedo-bay lifecycle stage"
)]
pub(super) fn update_torpedo_section_input(
    mut q_section: Query<
        (
            &mut TorpedoSectionInput,
            &TorpedoEngineFigures,
            &ChildOf,
            &crate::sections::torpedo_section::TorpedoSectionSpawnerEntity,
            Option<&SectionAmmo>,
            Has<SectionInactiveMarker>,
        ),
        With<TorpedoSectionMarker>,
    >,
    q_working_bays: Query<&ChildOf, (With<TorpedoSectionMarker>, Without<SectionInactiveMarker>)>,
    q_spawner: Query<&TorpedoSectionSpawnerFireState, With<TorpedoSectionSpawnerMarker>>,
    q_spaceship: Query<
        (
            Entity,
            &Transform,
            Option<&ComputedCenterOfMass>,
            &AIBehaviorState,
            &AITarget,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    q_torpedo: Query<
        (
            &ProjectileOwner,
            Option<&TorpedoTargetEntity>,
            Has<TorpedoTargetChosen>,
        ),
        With<TorpedoProjectileMarker>,
    >,
    q_target: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
    q_hull_radius: Query<&HullRadius>,
    q_ship_root: Query<(), With<SpaceshipRootMarker>>,
    spatial: SpatialQuery,
    q_sensor: Query<(), With<Sensor>>,
    q_collider_of: Query<&ColliderOf>,
) {
    for (entity, transform, com, state, target) in &q_spaceship {
        let engaged = state.engages() && *state != AIBehaviorState::Evade;
        // The launch bearing runs anchor to anchor, like every AI vector.
        let own_anchor = live_structure_anchor(transform, com);
        let target_ship = (**target).filter(|&target| q_ship_root.contains(target));
        let target_anchor =
            target_ship.and_then(|target| ai_target_anchor(Some(target), &q_target));
        // Live arms, read this frame: a hull that has lost its bow asks for
        // less room than it did whole, and so does the thing it is shooting
        // at. A root with no published arm is a point.
        let own_arm = q_hull_radius.get(entity).map_or(0.0, |arm| **arm);
        let target_arm = target_ship
            .and_then(|target| q_hull_radius.get(target).ok())
            .map_or(0.0, |arm| **arm);
        // Only working bays contribute slots, even when their magazines are
        // empty. A destroyed or disabled section closes its slot immediately.
        let bay_count = q_working_bays
            .iter()
            .filter(|ChildOf(parent)| *parent == entity)
            .count();
        // Newly launched projectiles have not yet had their target committed;
        // hold a slot for them so consecutive render frames cannot overshoot.
        let in_flight = q_torpedo
            .iter()
            .filter(|(owner, committed, chosen)| {
                ***owner == entity
                    && (!*chosen
                        || committed.is_some_and(|committed| Some(**committed) == target_ship))
            })
            .count();
        let mut claimed = false;
        // Line-of-fire gate, memoized so the ship casts AT MOST one ray per
        // frame (every bay launches down the same anchor-to-anchor bearing)
        // and none at all while the cheap per-bay gates hold the trigger
        // anyway. A torpedo PN-navigates, so the straight ray is conservative
        // - it can hold a launch that would have curved around a rock's edge;
        // the cost is a delayed launch, never a torpedo spent on cover.
        let mut line_clear: Option<bool> = None;
        let mut line_clear = |own_anchor: Vec3| {
            *line_clear.get_or_insert_with(|| match (target_ship, target_anchor) {
                (Some(target_ship), Some(anchor)) => !ai_line_of_fire_blocked(
                    &spatial,
                    &q_sensor,
                    &q_collider_of,
                    entity,
                    target_ship,
                    own_anchor,
                    anchor,
                ),
                _ => false,
            })
        };

        for (mut input, figures, _, spawner, ammo, inactive) in q_section
            .iter_mut()
            .filter(|(_, _, ChildOf(parent), _, _, _)| *parent == entity)
        {
            let launch = engaged
                && target_ship.is_some()
                && !inactive
                && ammo.is_none_or(|ammo| !ammo.is_empty())
                && q_spawner.get(**spawner).is_ok_and(|state| state.ready())
                && in_flight < bay_count
                && !claimed
                && target_anchor.is_some_and(|anchor| {
                    ai_torpedo_envelope(
                        anchor - own_anchor,
                        *transform.forward(),
                        figures.blast_radius,
                        own_arm,
                        target_arm,
                    )
                })
                && line_clear(own_anchor);
            claimed |= launch;
            // Change-detection hygiene, and an explicit release (not a
            // skip) so a bay holding the trigger drops it the moment any
            // gate closes.
            if **input != launch {
                **input = launch;
            }
        }
    }
}

/// Commit each freshly launched AI torpedo to its owner's launch-time
/// [`AITarget`] - the AI-side sibling of the player's commit-on-launch
/// (input/player/intent.rs): the targeting decision is made exactly once, right
/// after launch, and an owner with no target by commit time makes it a
/// dumb-fire shot for life. Torpedoes owned by non-AI ships are left to the
/// player's commit system, and vice versa.
///
/// Only a SHIP target commits, matching the trigger side's gate: the
/// launch and the commit are one frame apart, and an [`AITarget`] that
/// flipped to a hostile torpedo in that frame (ship target died, ordnance
/// in range) must not send this torpedo chasing ordnance - it dumb-fires
/// instead.
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
    q_spaceship: Query<&AITarget, With<AISpaceshipMarker>>,
    q_ship_root: Query<(), With<SpaceshipRootMarker>>,
) {
    for (torpedo, owner) in &q_torpedo {
        let Ok(target) = q_spaceship.get(**owner) else {
            continue;
        };
        let target = (**target).filter(|&target| q_ship_root.contains(target));

        debug!(
            "update_torpedo_target_input: committing AI torpedo {:?} to target {:?}",
            torpedo, target
        );

        let mut torpedo_commands = commands.entity(torpedo);
        torpedo_commands.insert(TorpedoTargetChosen);
        if let Some(target_entity) = target {
            torpedo_commands.insert(TorpedoTargetEntity(target_entity));
        }
    }
}

#[cfg(test)]
mod torpedo_tests {
    use avian3d::collider_tree::ColliderTrees;
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn the_envelope_is_a_range_band_with_rough_alignment() {
        // Engine units: the shipped 300 m warhead is 30 u, so two point hulls
        // keep the bare tactical margin of 30 * 3 = 90 u (900 m).
        let blast_radius = 30.0;
        let forward = Vec3::NEG_Z;
        let point = 0.0;

        assert!(
            ai_torpedo_envelope(Vec3::NEG_Z * 300.0, forward, blast_radius, point, point),
            "in band, dead ahead: launch"
        );
        assert!(
            ai_torpedo_envelope(
                Vec3::new(100.0, 0.0, -300.0),
                forward,
                blast_radius,
                point,
                point
            ),
            "in band, ~18 degrees off: rough alignment accepts it"
        );
        assert!(
            !ai_torpedo_envelope(Vec3::NEG_Z * 80.0, forward, blast_radius, point, point),
            "below the blast-derived minimum: a launch here self-hits"
        );
        assert!(
            !ai_torpedo_envelope(
                Vec3::NEG_Z * (AI_TORPEDO_MAX_RANGE.to_engine() + 1.0),
                forward,
                blast_radius,
                point,
                point
            ),
            "beyond the outer edge"
        );
        assert!(
            !ai_torpedo_envelope(Vec3::X * 300.0, forward, blast_radius, point, point),
            "perpendicular bearing: misaligned"
        );
        assert!(
            !ai_torpedo_envelope(Vec3::ZERO, forward, 0.0, point, point),
            "degenerate zero bearing"
        );
    }

    #[test]
    fn a_carrier_holds_its_ordnance_where_a_skiff_would_have_launched() {
        // The same 900 m of tactical margin, and the same anchor-to-anchor
        // range: what differs is how much of that range each pair of hulls
        // spends on itself. Both arms are the live figures measured on the
        // reference hulls (a skiff at 47.8 m, a carrier at 194.2 m).
        let blast_radius = 30.0;
        let forward = Vec3::NEG_Z;
        let skiff = 4.78;
        let carrier = 19.42;
        let range = Vec3::NEG_Z * 100.0;

        assert!(
            ai_torpedo_envelope(range, forward, blast_radius, skiff, skiff),
            "1,000 m apart, two skiffs: 904 m of clear space between the faces"
        );
        assert!(
            !ai_torpedo_envelope(range, forward, blast_radius, carrier, carrier),
            "the same 1,000 m between two carriers leaves 612 m of clear space, \
             not the 900 m the margin asks for"
        );
        assert!(
            ai_torpedo_envelope(Vec3::NEG_Z * 129.0, forward, blast_radius, carrier, carrier),
            "1,290 m puts 902 m between the two carriers' faces"
        );
    }

    /// Mirror the launch owner's real spawner relationship and ready cooldown
    /// without registering the bay's render and projectile systems in this rig.
    fn spawn_bay(world: &mut World, ship: Entity) -> Entity {
        let bay = world
            .spawn((
                torpedo_section(TorpedoSectionConfig::default()),
                ChildOf(ship),
            ))
            .id();
        let spawner = world
            .spawn((
                TorpedoSectionSpawnerMarker,
                TorpedoSectionPartOf(bay),
                TorpedoSectionSpawnerFireState(Cooldown::new(1.0)),
                ChildOf(bay),
            ))
            .id();
        world
            .entity_mut(bay)
            .insert(crate::sections::torpedo_section::TorpedoSectionSpawnerEntity(spawner));
        bay
    }

    /// An AI ship (at the origin, facing -Z) engaged on a player ship, with
    /// one default-config torpedo bay. Returns (world, ship, target, bay).
    fn torpedo_world(target_position: Vec3) -> (World, Entity, Entity, Entity) {
        let mut world = crate::input::ai::ai_test_world();
        // Empty collider trees for the launch gate's SpatialQuery: no
        // colliders means a clear line of fire, which is this rig's intent.
        world.init_resource::<ColliderTrees>();
        let target = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                RigidBody::Dynamic,
                Transform::from_translation(target_position),
            ))
            .id();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                RigidBody::Dynamic,
                AITarget(Some(target)),
                Transform::default(),
            ))
            .id();
        let bay = spawn_bay(&mut world, ship);
        (world, ship, target, bay)
    }

    /// Run one AI trigger frame.
    fn run_trigger(world: &mut World) {
        world.run_system_once(update_torpedo_section_input).unwrap();
    }

    #[test]
    fn an_engaged_ship_pulls_the_trigger_inside_the_envelope() {
        // Default blast radius 30 u (the authored 300 m) -> min range
        // 90 u; 300 u dead ahead is in band and aligned, the default state
        // is Engage with no torpedo in flight: everything is open.
        let (mut world, _, _, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));

        run_trigger(&mut world);
        assert!(
            **world.entity(bay).get::<TorpedoSectionInput>().unwrap(),
            "envelope open: trigger pulled"
        );
    }

    #[test]
    fn out_of_envelope_releases_a_held_trigger() {
        // Inside the blast-derived minimum: the gate is closed, and a
        // trigger that was held (input true) must be explicitly released.
        let (mut world, _, _, bay) = torpedo_world(Vec3::new(0.0, 0.0, -50.0));
        run_trigger(&mut world);
        **world
            .entity_mut(bay)
            .get_mut::<TorpedoSectionInput>()
            .unwrap() = true;

        world.run_system_once(update_torpedo_section_input).unwrap();

        assert!(
            !**world.entity(bay).get::<TorpedoSectionInput>().unwrap(),
            "below minimum range: released, not skipped"
        );
    }

    #[test]
    fn evade_and_passive_states_hold_torpedoes() {
        for state in [
            AIBehaviorState::Evade,
            AIBehaviorState::Idle,
            AIBehaviorState::Patrol,
        ] {
            let (mut world, ship, _, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));
            *world.entity_mut(ship).get_mut::<AIBehaviorState>().unwrap() = state;

            run_trigger(&mut world);

            assert!(
                !**world.entity(bay).get::<TorpedoSectionInput>().unwrap(),
                "no launch from {state:?}"
            );
        }
    }

    #[test]
    fn torpedo_targets_are_not_worth_a_bay() {
        // Retarget the ship onto a hostile torpedo (a valid AITarget pick
        // when no ships are around): the guns' job, not the bay's.
        let (mut world, ship, _, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                RigidBody::Dynamic,
                TorpedoTargetChosen,
                Transform::from_translation(Vec3::new(0.0, 0.0, -300.0)),
            ))
            .id();
        **world.entity_mut(ship).get_mut::<AITarget>().unwrap() = Some(torpedo);

        run_trigger(&mut world);

        assert!(
            !**world.entity(bay).get::<TorpedoSectionInput>().unwrap(),
            "a torpedo target does not open the launch envelope"
        );
    }

    #[test]
    fn a_ready_second_bay_launches_while_the_first_is_reloading() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(TorpedoSectionPlugin::default());
        let world = app.world_mut();
        world.init_resource::<ColliderTrees>();
        let target = world
            .spawn((
                SpaceshipRootMarker,
                Transform::from_translation(Vec3::NEG_Z * 300.0),
            ))
            .id();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AITarget(Some(target)),
                Transform::default(),
                Position(Vec3::ZERO),
                Rotation::default(),
                LinearVelocity(Vec3::ZERO),
                AngularVelocity(Vec3::ZERO),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        let config = TorpedoSectionConfig {
            ammunition: AmmoCapacity::Limited(2),
            ..default()
        };
        let bay_a = world
            .spawn((
                torpedo_section(config.clone()),
                Transform::default(),
                ChildOf(ship),
            ))
            .id();
        let bay_b = world
            .spawn((torpedo_section(config), Transform::default(), ChildOf(ship)))
            .id();
        let spawner_a = **world
            .get::<crate::sections::torpedo_section::TorpedoSectionSpawnerEntity>(bay_a)
            .unwrap();
        let spawner_b = **world
            .get::<crate::sections::torpedo_section::TorpedoSectionSpawnerEntity>(bay_b)
            .unwrap();
        assert!(world
            .get::<TorpedoSectionSpawnerFireState>(spawner_a)
            .unwrap()
            .ready());
        assert!(world
            .get::<TorpedoSectionSpawnerFireState>(spawner_b)
            .unwrap()
            .ready());

        run_trigger(app.world_mut());
        let (first, first_spawner, second, second_spawner) =
            if **app.world().get::<TorpedoSectionInput>(bay_a).unwrap() {
                (bay_a, spawner_a, bay_b, spawner_b)
            } else {
                assert!(**app.world().get::<TorpedoSectionInput>(bay_b).unwrap());
                (bay_b, spawner_b, bay_a, spawner_a)
            };
        assert!(!**app.world().get::<TorpedoSectionInput>(second).unwrap());

        // Force the bay that just claimed into cooldown before any launch.
        // Its query position does not change, so a claim by it on the next
        // frame would starve the other bay even though the other is ready.
        app.world_mut()
            .get_mut::<TorpedoSectionSpawnerFireState>(first_spawner)
            .unwrap()
            .trigger();
        run_trigger(app.world_mut());
        assert!(!**app.world().get::<TorpedoSectionInput>(first).unwrap());
        assert!(**app.world().get::<TorpedoSectionInput>(second).unwrap());
        app.world_mut()
            .get_mut::<TorpedoSectionSpawnerFireState>(first_spawner)
            .unwrap()
            .tick(1.0);

        run_trigger(app.world_mut());
        assert!(**app.world().get::<TorpedoSectionInput>(first).unwrap());
        assert!(!**app.world().get::<TorpedoSectionInput>(second).unwrap());
        app.world_mut().run_schedule(FixedUpdate);
        assert_eq!(app.world().get::<SectionAmmo>(first).unwrap().rounds, 1);
        assert!(!app
            .world()
            .get::<TorpedoSectionSpawnerFireState>(first_spawner)
            .unwrap()
            .ready());
        assert!(app
            .world()
            .get::<TorpedoSectionSpawnerFireState>(second_spawner)
            .unwrap()
            .ready());

        app.world_mut()
            .get_mut::<TorpedoSectionSpawnerFireState>(second_spawner)
            .unwrap()
            .trigger();
        run_trigger(app.world_mut());
        assert!(
            !**app.world().get::<TorpedoSectionInput>(first).unwrap(),
            "busy first bay releases its trigger"
        );
        assert!(
            !**app.world().get::<TorpedoSectionInput>(second).unwrap(),
            "neither cooling bay can claim"
        );
        app.world_mut()
            .get_mut::<TorpedoSectionSpawnerFireState>(second_spawner)
            .unwrap()
            .tick(1.0);
        run_trigger(app.world_mut());
        assert!(
            !**app.world().get::<TorpedoSectionInput>(first).unwrap(),
            "busy first bay cannot claim"
        );
        assert!(
            **app.world().get::<TorpedoSectionInput>(second).unwrap(),
            "ready second bay claims the next frame"
        );
        app.world_mut().run_schedule(FixedUpdate);
        assert_eq!(
            app.world().get::<SectionAmmo>(second).unwrap().rounds,
            1,
            "the second bay fires a real round"
        );
        let launches: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<TorpedoProjectileMarker>>()
            .iter(app.world())
            .collect();
        assert_eq!(launches.len(), 2, "one real launch from each bay");
    }

    #[test]
    fn one_two_and_three_working_bays_set_the_in_flight_limit() {
        for bay_count in 1..=3 {
            let (mut world, ship, target, first) = torpedo_world(Vec3::NEG_Z * 300.0);
            let mut bays = vec![first];
            for _ in 1..bay_count {
                bays.push(spawn_bay(&mut world, ship));
            }
            let open = |world: &World| {
                bays.iter()
                    .filter(|&&bay| **world.entity(bay).get::<TorpedoSectionInput>().unwrap())
                    .count()
            };
            for _ in 0..bay_count {
                run_trigger(&mut world);
                assert_eq!(open(&world), 1, "only one bay claims this frame");
                world.spawn((
                    TorpedoProjectileMarker,
                    ProjectileOwner(ship),
                    TorpedoTargetEntity(target),
                    TorpedoTargetChosen,
                ));
            }
            run_trigger(&mut world);
            assert_eq!(
                open(&world),
                0,
                "{bay_count} live torpedoes fill {bay_count} slots"
            );
            let to_remove = world
                .query_filtered::<Entity, With<TorpedoProjectileMarker>>()
                .iter(&world)
                .find(|&projectile| {
                    world
                        .get::<ProjectileOwner>(projectile)
                        .is_some_and(|owner| **owner == ship)
                })
                .unwrap();
            world.despawn(to_remove);
            run_trigger(&mut world);
            assert_eq!(
                open(&world),
                1,
                "hit, expiry or interception reopens a slot"
            );
        }
    }

    #[test]
    fn lost_inactive_and_replaced_bays_change_slots_even_when_empty() {
        let (mut world, ship, target, first) = torpedo_world(Vec3::NEG_Z * 300.0);
        let second = spawn_bay(&mut world, ship);
        world.entity_mut(second).insert(SectionAmmo {
            rounds: 0,
            capacity: 6,
        });
        world.spawn((
            TorpedoProjectileMarker,
            ProjectileOwner(ship),
            TorpedoTargetEntity(target),
            TorpedoTargetChosen,
        ));
        run_trigger(&mut world);
        assert!(
            **world.entity(first).get::<TorpedoSectionInput>().unwrap(),
            "empty bay still supplies a slot"
        );
        assert!(
            !**world.entity(second).get::<TorpedoSectionInput>().unwrap(),
            "empty bay cannot claim launch"
        );
        world.entity_mut(second).insert(SectionInactiveMarker);
        run_trigger(&mut world);
        assert!(
            !**world.entity(first).get::<TorpedoSectionInput>().unwrap(),
            "inactive bay closes its slot"
        );
        world.despawn(second);
        let replacement = spawn_bay(&mut world, ship);
        run_trigger(&mut world);
        let open = [first, replacement]
            .into_iter()
            .filter(|&bay| **world.entity(bay).get::<TorpedoSectionInput>().unwrap())
            .count();
        assert_eq!(open, 1, "a replacement bay reopens one slot");
        world.despawn(replacement);
        run_trigger(&mut world);
        assert!(
            !**world.entity(first).get::<TorpedoSectionInput>().unwrap(),
            "losing a working bay closes its slot"
        );
        spawn_bay(&mut world, ship);
        run_trigger(&mut world);
        assert!(
            **world.entity(first).get::<TorpedoSectionInput>().unwrap(),
            "replacing the lost working bay reopens a slot"
        );
    }

    #[test]
    fn in_flight_slots_are_separate_by_ship_and_target() {
        let (mut world, ship_a, target_a, bay_a) = torpedo_world(Vec3::NEG_Z * 300.0);
        let target_b = world
            .spawn((
                SpaceshipRootMarker,
                Transform::from_translation(Vec3::NEG_Z * 400.0),
            ))
            .id();
        let ship_b = world
            .spawn((
                AISpaceshipMarker,
                AITarget(Some(target_a)),
                Transform::default(),
            ))
            .id();
        let bay_b = spawn_bay(&mut world, ship_b);
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                ProjectileOwner(ship_a),
                TorpedoTargetEntity(target_a),
                TorpedoTargetChosen,
            ))
            .id();
        run_trigger(&mut world);
        assert!(
            !**world.entity(bay_a).get::<TorpedoSectionInput>().unwrap(),
            "ship A's slot is filled"
        );
        assert!(
            **world.entity(bay_b).get::<TorpedoSectionInput>().unwrap(),
            "ship B has its own slot at the same target"
        );
        **world.entity_mut(ship_a).get_mut::<AITarget>().unwrap() = Some(target_b);
        run_trigger(&mut world);
        assert!(
            **world.entity(bay_a).get::<TorpedoSectionInput>().unwrap(),
            "ship A has a separate slot at target B"
        );
        // A new, uncommitted launch already occupies a slot until commitment.
        world.entity_mut(torpedo).remove::<TorpedoTargetEntity>();
        world.entity_mut(torpedo).remove::<TorpedoTargetChosen>();
        run_trigger(&mut world);
        assert!(
            !**world.entity(bay_a).get::<TorpedoSectionInput>().unwrap(),
            "uncommitted launch cannot overshoot target B"
        );
    }

    #[test]
    fn a_fresh_ai_torpedo_commits_to_the_owner_target() {
        let (mut world, ship, target, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));
        run_trigger(&mut world);
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                RigidBody::Dynamic,
                ProjectileOwner(ship),
                TorpedoSectionPartOf(bay),
            ))
            .id();

        world.run_system_once(update_torpedo_target_input).unwrap();

        assert!(
            world.entity(torpedo).get::<TorpedoTargetChosen>().is_some(),
            "the launch-time decision is made exactly once"
        );
        assert_eq!(
            world
                .entity(torpedo)
                .get::<TorpedoTargetEntity>()
                .map(|t| **t),
            Some(target),
            "committed to the owner's AITarget"
        );
    }

    #[test]
    fn an_ai_torpedo_without_a_target_dumb_fires() {
        // The owner lost its target between launch and commit: the torpedo
        // is committed target-less for life, like a player dumb-fire shot.
        let (mut world, ship, _, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));
        run_trigger(&mut world);
        **world.entity_mut(ship).get_mut::<AITarget>().unwrap() = None;
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                RigidBody::Dynamic,
                ProjectileOwner(ship),
                TorpedoSectionPartOf(bay),
            ))
            .id();

        world.run_system_once(update_torpedo_target_input).unwrap();

        assert!(
            world.entity(torpedo).get::<TorpedoTargetChosen>().is_some(),
            "the decision is still made"
        );
        assert!(
            world.entity(torpedo).get::<TorpedoTargetEntity>().is_none(),
            "no target to commit to: dumb-fire"
        );
    }

    #[test]
    fn player_torpedoes_are_left_to_the_player_commit_system() {
        let (mut world, _, target, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                RigidBody::Dynamic,
                ProjectileOwner(target),
                TorpedoSectionPartOf(bay),
            ))
            .id();

        world.run_system_once(update_torpedo_target_input).unwrap();

        assert!(
            world.entity(torpedo).get::<TorpedoTargetChosen>().is_none(),
            "a non-AI owner's torpedo is not this system's to commit"
        );
    }

    #[test]
    fn a_torpedo_target_at_commit_time_dumb_fires() {
        // The launch gate requires a ship, but the commit is a frame
        // later: an AITarget that flipped to a hostile torpedo in that
        // frame (ship target died, ordnance in range) must not send this
        // torpedo chasing ordnance.
        let (mut world, ship, _, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));
        run_trigger(&mut world);
        let hostile_torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                RigidBody::Dynamic,
                TorpedoTargetChosen,
                Transform::from_translation(Vec3::new(0.0, 0.0, -300.0)),
            ))
            .id();
        **world.entity_mut(ship).get_mut::<AITarget>().unwrap() = Some(hostile_torpedo);
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                RigidBody::Dynamic,
                ProjectileOwner(ship),
                TorpedoSectionPartOf(bay),
            ))
            .id();

        world.run_system_once(update_torpedo_target_input).unwrap();

        assert!(
            world.entity(torpedo).get::<TorpedoTargetChosen>().is_some(),
            "the decision is still made"
        );
        assert!(
            world.entity(torpedo).get::<TorpedoTargetEntity>().is_none(),
            "a non-ship target commits as a dumb-fire shot"
        );
    }

    #[test]
    fn the_trigger_is_per_ship() {
        // Ship A engaged in envelope, ship B (its own bay) with nothing to
        // fight: A's decision must not leak onto B's bay through the
        // shared section query.
        let (mut world, _, _, bay_a) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));
        let ship_b = world
            .spawn((
                AISpaceshipMarker,
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::new(500.0, 0.0, 0.0)),
            ))
            .id();
        let bay_b = spawn_bay(&mut world, ship_b);

        run_trigger(&mut world);

        assert!(
            **world.entity(bay_a).get::<TorpedoSectionInput>().unwrap(),
            "ship A: envelope open, trigger pulled"
        );
        assert!(
            !**world.entity(bay_b).get::<TorpedoSectionInput>().unwrap(),
            "ship B has no target: its bay stays released"
        );
    }
}
