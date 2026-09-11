//! Per-frame lock upkeep: validate and drop the held locks (naming the branch
//! that dropped one), rank the hostile [`ThreatContacts`] for the edge arrows,
//! and accumulate the [`LockFocus`] dwell.
//!
//! WHAT the ship can see is not decided here. The one sensor pass in
//! [`sensing`](super::sensing) publishes that, and this reads it - so the
//! locks, the radar picker and an AI picket beside the player cannot disagree
//! about the same sighting.

use avian3d::prelude::RigidBody;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

use crate::prelude::*;

/// Seconds of continuous lock on the same target before the component layer
/// unlocks (the WoT-style aim-in dwell).
pub(super) const FOCUS_TIME: f32 = 1.5;

/// Focus: how long the COMBAT lock has been held on the same target.
/// Component fine-locking unlocks at `FOCUS_TIME`; the HUD renders the
/// fill fraction while it accumulates. On the player ship root; the
/// provisional radar candidate never touches it - only a committed lock
/// accrues dwell.
#[derive(Component, Debug, Clone, PartialEq, Default, Reflect)]
#[reflect(Component)]
pub struct LockFocus {
    /// The target the timer is accumulating on (mirrors the combat lock).
    pub target: Option<Entity>,
    /// Continuous seconds the lock has stayed on `target`.
    pub seconds: f32,
}

impl LockFocus {
    /// Focus completion in [0, 1], for the HUD meter.
    pub fn fraction(&self) -> f32 {
        (self.seconds / FOCUS_TIME).clamp(0.0, 1.0)
    }

    /// Whether the component layer is unlocked for `target`.
    pub fn focused_on(&self, target: Entity) -> bool {
        self.target == Some(target) && self.seconds >= FOCUS_TIME
    }
}

/// How many hostile contacts the threat tracker keeps for the edge
/// indicators. A feel knob; more would clutter the HUD.
const TARGET_CANDIDATE_COUNT: usize = 5;

/// Per-frame lock upkeep, always on: hold the LOCKS only while their targets
/// stay in sight (death/despawn, range and cover clear them - stickiness
/// never needs a re-pick because NOTHING re-picks), clear the combat lock
/// when a hostile target turns non-hostile, and maintain the ranked hostile
/// [`ThreatContacts`] for the edge indicators.
///
/// A lock does NOT time out. It is held until the world takes it or the
/// player taps it away: a pilot who locks a hostile and then flies for a
/// minute still has the hostile locked.
#[expect(
    clippy::type_complexity,
    reason = "one query term per contact and lock input"
)]
pub(super) fn update_contacts_and_locks(
    look_ray: ActiveLookRay,
    q_flipped: Query<(), Changed<Allegiance>>,
    q_allegiances: Query<&Allegiance>,
    q_bodies: Query<(), With<RigidBody>>,
    mut dropped: MessageWriter<CombatLockDropped>,
    mut spaceship: Query<
        (
            &Transform,
            Option<&Allegiance>,
            &SensorContacts,
            &mut TravelLock,
            &mut CombatLock,
            &mut ThreatContacts,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    for (transform, ship_allegiance, contacts, mut travel, mut combat, mut threats) in
        &mut spaceship
    {
        let origin = contacts.origin;

        // BOTH slots need the line. A lock is a radio link either way: the
        // travel designation used to ride through cover on the reading that a
        // place you were shown stays shown, and the two slots behaving
        // differently under the same rock is exactly the inconsistency the one
        // sensor pass exists to retire. Neither re-locks on its own when the
        // line clears - a fresh dwell is what takes a lock.
        let travel_now = travel.0.filter(|target| contacts.in_sight(*target));
        if travel.0 != travel_now {
            travel.0 = travel_now;
        }
        let mut combat_now = combat.0.filter(|target| contacts.in_sight(*target));
        // Name the branch that let go, rather than leaving the owner (and any
        // future investigation) to infer it from the wreckage - and name it off
        // the pass that made the decision, so the log cannot disagree with the
        // gate. Seen but out of sight is COVER; gone from the sensor pass while
        // still a body is RANGE; not a body at all is GONE.
        if let (Some(target), None) = (combat.0, combat_now) {
            let reason = match contacts.get(target) {
                Some(_) => CombatLockDrop::Occluded,
                None if q_bodies.get(target).is_ok() => CombatLockDrop::OutOfRange,
                None => CombatLockDrop::TargetGone,
            };
            report_combat_lock_drop(&mut dropped, target, reason);
        }

        // A hostile combat target FLIPPING to non-hostile clears the lock (a
        // scripted surrender must not keep the guns hot); a deliberate lock
        // on an always-neutral body is untouched - only a CHANGE trips this.
        let before_flip = combat_now;
        combat_now = combat_now.filter(|target| {
            !(q_flipped.get(*target).is_ok()
                && relation(ship_allegiance, q_allegiances.get(*target).ok()) != Relation::Hostile)
        });
        if let (Some(target), None) = (before_flip, combat_now) {
            report_combat_lock_drop(&mut dropped, target, CombatLockDrop::AllegianceFlip);
        }

        if combat.0 != combat_now {
            combat.0 = combat_now;
        }

        // The threat set: hostile combat targets ranked toward the look ray
        // (ship-forward fallback keeps the arrows meaningful rig-less).
        let aim = look_ray
            .direction()
            .unwrap_or_else(|| (transform.rotation * Vec3::NEG_Z).normalize());
        let ranked = rank_combat_targets(
            origin,
            aim,
            contacts
                .iter()
                .filter(|contact| {
                    contact.is_hostile()
                        && contact.is_combat_target()
                        && contact.in_sight
                        && !contact.neutralized
                })
                .map(|contact| (contact.entity, contact.anchor)),
        );
        let entries = maintain_contacts(&ranked, combat.0);
        if threats.entries != entries {
            threats.entries = entries;
        }
    }
}

/// Announce a combat-lock drop: one `debug!` line naming the branch, and the
/// [`CombatLockDropped`] message for any cue that wants to react. The log is
/// the point - "why did my lock let go?" was unanswerable from a shipped run
/// before.
fn report_combat_lock_drop(
    dropped: &mut MessageWriter<CombatLockDropped>,
    target: Entity,
    reason: CombatLockDrop,
) {
    debug!("combat lock dropped: target {target} - {reason:?}");
    dropped.write(CombatLockDropped { target, reason });
}

/// Rank the lockable hostile COMBAT targets (ships + committed torpedoes) for
/// the threat set: nearest the look ray first (largest cosine), distance as
/// the tie-breaker. Not cone-gated - a hostile behind the player is still
/// tracked (the edge-indicator overlay points at it).
///
/// Pure and camera/physics-free so the ranking rule can be unit-tested.
fn rank_combat_targets(
    origin: Vec3,
    aim: Vec3,
    targets: impl Iterator<Item = (Entity, Vec3)>,
) -> Vec<Entity> {
    let mut scored: Vec<(Entity, f32, f32)> = targets
        .filter_map(|(entity, position)| {
            let to_ship = position - origin;
            let distance = to_ship.length();
            (distance > f32::EPSILON).then(|| (entity, to_ship.normalize().dot(aim), distance))
        })
        .collect();
    scored.sort_by(|(_, cos_a, dist_a), (_, cos_b, dist_b)| {
        cos_b.total_cmp(cos_a).then(dist_a.total_cmp(dist_b))
    });
    scored.into_iter().map(|(entity, ..)| entity).collect()
}

/// Compose the threat entries: the top [`TARGET_CANDIDATE_COUNT`] by rank,
/// with the combat lock kept a member while it is still ranked (the arrow to
/// your own target must never vanish).
fn maintain_contacts(ranked: &[Entity], combat_lock: Option<Entity>) -> Vec<Entity> {
    let mut entries: Vec<Entity> = ranked
        .iter()
        .copied()
        .take(TARGET_CANDIDATE_COUNT)
        .collect();
    if let Some(lock) = combat_lock {
        if ranked.contains(&lock) && !entries.contains(&lock) {
            entries.pop();
            entries.push(lock);
        }
    }
    entries
}

/// Whether `ship` has a live controller section granting `verb` - the
/// computer-capability gate (mirrors player.rs's `ship_grants_verb`; the
/// radar needs it here for the Lock capability).
pub(super) fn ship_grants_lock(
    ship: Entity,
    q_controllers: &Query<
        (&ChildOf, Option<&WithheldVerbs>),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
) -> bool {
    q_controllers.iter().any(|(ChildOf(parent), withheld)| {
        *parent == ship && withheld.is_none_or(|w| w.granted(FlightVerb::Lock))
    })
}

/// Accumulate focus while the COMBAT lock stays on one target; any change
/// (new target or lock lost) restarts the dwell from zero. Generic over any
/// ship carrying the components (AI parity).
pub(super) fn tick_lock_focus(time: Res<Time>, mut q_ships: Query<(&CombatLock, &mut LockFocus)>) {
    for (lock, mut focus) in &mut q_ships {
        if focus.target != lock.0 {
            focus.target = lock.0;
            focus.seconds = 0.0;
            continue;
        }
        if focus.target.is_some() {
            focus.seconds += time.delta_secs();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn rank_orders_by_aim_angle_then_distance() {
        let origin = Vec3::ZERO;
        let aim = Vec3::NEG_Z;
        let on_ray_far = Entity::from_raw_u32(1).unwrap();
        let off_ray_near = Entity::from_raw_u32(2).unwrap();
        let behind = Entity::from_raw_u32(3).unwrap();
        let ranked = rank_combat_targets(
            origin,
            aim,
            [
                (behind, Vec3::new(0.0, 0.0, 100.0)),
                (off_ray_near, Vec3::new(30.0, 0.0, -50.0)),
                (on_ray_far, Vec3::new(1.0, 0.0, -500.0)),
            ]
            .into_iter(),
        );
        assert_eq!(
            ranked,
            vec![on_ray_far, off_ray_near, behind],
            "closest to the aim ray first; behind the ship ranks last but is still tracked"
        );
    }

    #[test]
    fn rank_breaks_angle_ties_by_distance() {
        let near = Entity::from_raw_u32(1).unwrap();
        let far = Entity::from_raw_u32(2).unwrap();
        let ranked = rank_combat_targets(
            Vec3::ZERO,
            Vec3::NEG_Z,
            [
                (far, Vec3::new(0.0, 0.0, -800.0)),
                (near, Vec3::new(0.0, 0.0, -200.0)),
            ]
            .into_iter(),
        );
        assert_eq!(ranked, vec![near, far]);
    }

    #[test]
    fn maintain_contacts_keeps_the_top_n_and_the_locked_target() {
        let entities: Vec<Entity> = (1..=7)
            .map(|raw| Entity::from_raw_u32(raw).unwrap())
            .collect();
        // Top 5 of 7 by rank.
        let entries = maintain_contacts(&entities, None);
        assert_eq!(entries, entities[..5].to_vec());
        // The combat lock ranked 7th stays a member (replaces the 5th).
        let entries = maintain_contacts(&entities, Some(entities[6]));
        assert_eq!(entries.len(), 5);
        assert!(entries.contains(&entities[6]));
        // An unranked lock (not collectible) is not forced in.
        let stranger = Entity::from_raw_u32(99).unwrap();
        let entries = maintain_contacts(&entities, Some(stranger));
        assert!(!entries.contains(&stranger));
    }

    /// Player with the state bundle and both locks set; no camera rig (the
    /// upkeep falls back to ship-forward for the threat ranking). Returns
    /// (world, player, travel_target, combat_target).
    fn locked_world() -> (World, Entity, Entity, Entity) {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        world.init_resource::<TargetingSettings>();
        // The lock scanner's line-of-sight ray reads avian's collider trees.
        // Empty here, which is the point: this rig is about validity and
        // decay, and nothing in it stands between the ship and a target.
        world.init_resource::<avian3d::collider_tree::ColliderTrees>();
        world.init_resource::<Messages<CombatLockDropped>>();
        let travel_target = world
            .spawn((
                RigidBody::Static,
                LockSignature(20.0),
                Transform::from_translation(Vec3::new(0.0, 0.0, -300.0)),
            ))
            .id();
        // A 40u structural arm returns 248u, which gates at 7440u: this rig
        // walks a target out to 6400u, and the arm is what keeps that inside
        // the gate. (`publish_hull_radii` derives the arm from live sections;
        // here it is staged, because this rig is about the lock upkeep.)
        let combat_target = world
            .spawn((
                SpaceshipRootMarker,
                AISpaceshipMarker,
                RigidBody::Dynamic,
                HullRadius(40.0),
                Transform::from_translation(Vec3::new(0.0, 0.0, -400.0)),
            ))
            .id();
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::IDENTITY,
                targeting_state(),
            ))
            .id();
        // Register the upkeep ONCE so change detection (Changed<Allegiance>)
        // is real across runs - run_system_once builds a fresh system each
        // call, which would see EVERYTHING as changed, exactly the
        // false-positive this rig must not have. Settle the spawn-frame
        // Changed ticks before locking, as a live app would.
        let signature_id =
            world.register_system(crate::sections::signature::publish_ship_signatures);
        let sensing_id = world.register_system(super::super::sensing::update_sensor_contacts);
        let upkeep_id = world.register_system(update_contacts_and_locks);
        world.insert_resource(UpkeepSystem(signature_id, sensing_id, upkeep_id));
        upkeep(&mut world);
        world.get_mut::<TravelLock>(player).unwrap().0 = Some(travel_target);
        world.get_mut::<CombatLock>(player).unwrap().0 = Some(combat_target);
        (world, player, travel_target, combat_target)
    }

    #[derive(Resource)]
    struct UpkeepSystem(
        bevy::ecs::system::SystemId,
        bevy::ecs::system::SystemId,
        bevy::ecs::system::SystemId,
    );

    /// One frame of the chain, in production order: each hull publishes what
    /// it returns to a scanner, the sensor pass decides what the ship can see
    /// from that, then the upkeep decides what it keeps.
    fn upkeep(world: &mut World) {
        let (signature, sensing, upkeep) = {
            let systems = world.resource::<UpkeepSystem>();
            (systems.0, systems.1, systems.2)
        };
        world.run_system(signature).unwrap();
        world.run_system(sensing).unwrap();
        world.run_system(upkeep).unwrap();
    }

    /// Drain the drop messages the last upkeep wrote - the evidence rig's
    /// readout: the branch NAMES itself, so nothing here has to infer the
    /// cause from the leftover world state.
    fn drops(world: &mut World) -> Vec<CombatLockDropped> {
        world
            .resource_mut::<Messages<CombatLockDropped>>()
            .drain()
            .collect()
    }

    /// Hang a weapon section with a HELD trigger on `ship`, the way the
    /// player's input observers latch one (`on_turret_input`).
    fn hold_trigger(world: &mut World, ship: Entity) -> Entity {
        world
            .spawn((SectionMarker, TurretSectionInput(true), ChildOf(ship)))
            .id()
    }

    /// THE EVIDENCE RIG. The owner asked why the ship "sometimes loses radar
    /// focus on locked enemies"; this walks each way the upkeep can let go
    /// and records the branch BY NAME, so the answer is read off the run
    /// instead of guessed.
    #[test]
    fn the_evidence_rig_names_every_branch_that_drops_the_combat_lock() {
        // A healthy lock drops nothing, however long it is held.
        let (mut world, _player, _travel, _combat_target) = locked_world();
        upkeep(&mut world);
        assert!(drops(&mut world).is_empty(), "a healthy lock drops nothing");

        // 1. Out of range: the target exists and is still lockable, it is
        // simply too far - even past the incumbent's widened gate.
        let (mut world, player, _travel, combat_target) = locked_world();
        let past_gate = PLAYER_SENSOR_RANGE * 1.15 + 1.0;
        world
            .entity_mut(combat_target)
            .insert(Transform::from_translation(Vec3::new(0.0, 0.0, -past_gate)));
        upkeep(&mut world);
        assert_eq!(
            drops(&mut world),
            vec![CombatLockDropped {
                target: combat_target,
                reason: CombatLockDrop::OutOfRange,
            }],
            "a body still in the world but past its gate reads OutOfRange"
        );
        assert_eq!(world.get::<CombatLock>(player).unwrap().0, None);

        // 2. Target gone: despawned outright.
        let (mut world, player, _travel, combat_target) = locked_world();
        world.despawn(combat_target);
        upkeep(&mut world);
        assert_eq!(
            drops(&mut world),
            vec![CombatLockDropped {
                target: combat_target,
                reason: CombatLockDrop::TargetGone,
            }],
            "a despawned target reads TargetGone, never OutOfRange"
        );
        assert_eq!(world.get::<CombatLock>(player).unwrap().0, None);

        // 3. Allegiance flip: the scripted surrender.
        let (mut world, player, _travel, combat_target) = locked_world();
        upkeep(&mut world);
        let _ = drops(&mut world);
        world.entity_mut(combat_target).insert(Allegiance::Neutral);
        upkeep(&mut world);
        assert_eq!(
            drops(&mut world),
            vec![CombatLockDropped {
                target: combat_target,
                reason: CombatLockDrop::AllegianceFlip,
            }],
            "a surrender names itself, so it cannot be mistaken for cover"
        );
        assert_eq!(world.get::<CombatLock>(player).unwrap().0, None);
    }

    /// THE PIN of the ruled-out paths: with the target alive, in range, still
    /// hostile, in sight and no gesture, the lock does NOT drop. There is no
    /// clock left to cross, so a pilot who locks a hostile and then does
    /// nothing for ten minutes still has it locked.
    #[test]
    fn a_live_in_range_hostile_lock_never_drops() {
        let (mut world, player, _travel, combat_target) = locked_world();

        // Ten idle minutes against a target that drifts within its gate.
        for step in 0..600 {
            let distance = 400.0 + (step as f32) * 10.0;
            world
                .entity_mut(combat_target)
                .insert(Transform::from_translation(Vec3::new(0.0, 0.0, -distance)));
            world
                .resource_mut::<Time>()
                .advance_by(Duration::from_secs_f32(1.0));
            upkeep(&mut world);
            let dropped = drops(&mut world);
            assert!(
                dropped.is_empty(),
                "step {step} at {distance} m dropped the lock: {dropped:?}"
            );
        }
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            Some(combat_target),
            "6400 m and 600 s later the lock still holds"
        );
    }

    /// "Focus" is ambiguous in the owner's report, so the two candidates are
    /// pinned apart: a [`LockFocus`] dwell RESET is not a lock drop.
    /// Retargeting restarts the 1.5 s component dwell while the combat lock
    /// stays firmly latched and nothing reports a drop.
    #[test]
    fn a_focus_dwell_reset_is_not_a_lock_drop() {
        let (mut world, player, _travel, first) = locked_world();
        let second = world
            .spawn((
                SpaceshipRootMarker,
                AISpaceshipMarker,
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::new(0.0, 0.0, -500.0)),
            ))
            .id();
        hold_trigger(&mut world, player);

        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(FOCUS_TIME * 2.0));
        upkeep(&mut world);
        // The first tick ADOPTS the target (dwell zero); the next accumulates.
        world.run_system_once(tick_lock_focus).unwrap();
        world.run_system_once(tick_lock_focus).unwrap();
        assert!(
            world.get::<LockFocus>(player).unwrap().focused_on(first),
            "the dwell completes on the first target"
        );

        // A retarget: same gesture, new body.
        world.get_mut::<CombatLock>(player).unwrap().0 = Some(second);
        world.run_system_once(tick_lock_focus).unwrap();
        assert!(
            !world.get::<LockFocus>(player).unwrap().focused_on(second),
            "the dwell restarts on the new target"
        );
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            Some(second),
            "but the LOCK itself never let go"
        );
        assert!(
            drops(&mut world).is_empty(),
            "and no drop was reported - a dwell reset is a different event"
        );
    }

    #[test]
    fn locks_hold_while_collectible_and_clear_on_death_or_range() {
        let (mut world, player, travel_target, combat_target) = locked_world();

        upkeep(&mut world);
        assert_eq!(
            world.get::<TravelLock>(player).unwrap().0,
            Some(travel_target),
            "delivery guard: the travel lock holds while collectible"
        );
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            Some(combat_target)
        );

        // The travel target leaves its signature range: cleared; the combat
        // ship, whose bigger hull returns far more, survives.
        world
            .entity_mut(travel_target)
            .insert(Transform::from_translation(Vec3::new(0.0, 0.0, -900.0)));
        upkeep(&mut world);
        assert_eq!(world.get::<TravelLock>(player).unwrap().0, None);
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            Some(combat_target)
        );

        // The combat target dies: cleared.
        world.despawn(combat_target);
        upkeep(&mut world);
        assert_eq!(world.get::<CombatLock>(player).unwrap().0, None);
    }

    /// A hostile combat target FLIPPING non-hostile clears the lock; a
    /// deliberate lock on an always-neutral body is untouched.
    #[test]
    fn allegiance_flip_clears_the_combat_lock_but_deliberate_neutrals_hold() {
        let (mut world, player, _travel, combat_target) = locked_world();
        upkeep(&mut world);
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            Some(combat_target),
            "delivery guard: locked while hostile"
        );

        // The scripted surrender: Enemy -> Neutral.
        world.entity_mut(combat_target).insert(Allegiance::Neutral);
        upkeep(&mut world);
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            None,
            "a surrender must not keep the guns hot"
        );

        // A deliberate combat lock on an (unchanged) neutral holds.
        world.get_mut::<CombatLock>(player).unwrap().0 = Some(combat_target);
        upkeep(&mut world);
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            Some(combat_target),
            "combat mode is combat mode - deliberate neutral locks are legal"
        );
    }

    #[test]
    fn threat_contacts_track_hostile_combat_targets() {
        let (mut world, player, _travel, combat_target) = locked_world();
        // A neutral ship and a hostile committed torpedo alongside.
        world.spawn((
            SpaceshipRootMarker,
            Allegiance::Neutral,
            RigidBody::Dynamic,
            Transform::from_translation(Vec3::new(50.0, 0.0, -100.0)),
        ));
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetChosen,
                Allegiance::Enemy,
                // The shipped 320 m/s type: 820 m of return.
                LockSignature(82.0),
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::new(0.0, 10.0, -200.0)),
            ))
            .id();

        upkeep(&mut world);
        let entries = world.get::<ThreatContacts>(player).unwrap().entries.clone();
        assert!(entries.contains(&combat_target));
        assert!(
            entries.contains(&torpedo),
            "a committed hostile torpedo is a threat"
        );
        assert_eq!(entries.len(), 2, "neutrals and beacons stay out");

        world.entity_mut(combat_target).insert(NeutralizedMarker);
        upkeep(&mut world);
        let entries = &world.get::<ThreatContacts>(player).unwrap().entries;
        assert!(
            !entries.contains(&combat_target),
            "a neutralized wreck is no longer an active threat"
        );
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            Some(combat_target),
            "neutralization preserves the player's existing combat lock"
        );
    }

    #[test]
    fn focus_accumulates_and_resets_on_lock_change() {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let ship = world
            .spawn((CombatLock(Some(a)), LockFocus::default()))
            .id();

        world.run_system_once(tick_lock_focus).unwrap();
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1.0));
        world.run_system_once(tick_lock_focus).unwrap();
        let focus = world.get::<LockFocus>(ship).unwrap();
        assert_eq!(focus.target, Some(a));
        assert!((focus.seconds - 1.0).abs() < 1e-6);
        assert!(!focus.focused_on(a));

        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(0.6));
        world.run_system_once(tick_lock_focus).unwrap();
        assert!(world.get::<LockFocus>(ship).unwrap().focused_on(a));

        // Switching targets restarts the dwell - a radar re-commit of the
        // SAME entity is equality and never lands here.
        world.get_mut::<CombatLock>(ship).unwrap().0 = Some(b);
        world.run_system_once(tick_lock_focus).unwrap();
        let focus = world.get::<LockFocus>(ship).unwrap();
        assert_eq!(focus.target, Some(b));
        assert_eq!(focus.seconds, 0.0);
    }
}
