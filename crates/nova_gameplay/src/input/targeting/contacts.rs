//! Per-frame lock upkeep: collect the lockable bodies, validate and drop the
//! held locks (naming the branch that dropped one), rank the hostile
//! [`ThreatContacts`] for the edge arrows, and accumulate the [`LockFocus`]
//! dwell.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::prelude::*;

/// Maximum distance at which the aim-assist will lock a target - the ceiling
/// for the intrinsic classes (well bodies, ships), which stay designatable
/// from across the play area for GOTO legs. Everything else is gated far
/// shorter by the signature model (the later report: long-range locks should
/// only see large objects), see [`LockSignature`] and [`TargetingSettings`].
const TARGETING_MAX_RANGE: f32 = 20_000.0;

/// Seconds without combat activity (the raised stance or a held weapon
/// trigger) before a held combat lock decays and the weapons safety re-
/// engages (user-tuned). A const knob; kept at 30 by the owner
/// in, which made the wind-down VISIBLE instead of shortening or removing the
/// rule.
pub const COMBAT_DECAY_SECS: f32 = 30.0;

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

/// A collected lockable body: entity, world position, hostile-to-player,
/// combat-target (ship or committed torpedo).
pub(super) type Lockable = (Entity, Vec3, bool, bool);

/// The scanner query every collection pass walks. Turret bullets are excluded
/// outright: they are dynamic bodies that stream straight down the aim ray.
pub(super) type LockableQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GlobalTransform,
        &'static RigidBody,
        Option<&'static GravityWell>,
        Option<&'static LockSignature>,
        Has<SpaceshipRootMarker>,
        Option<&'static TorpedoProjectileMarker>,
        Option<&'static TorpedoTargetChosen>,
        Option<&'static Allegiance>,
    ),
    Without<TurretBulletProjectileMarker>,
>;

/// Collect every body the scanner can currently see from `origin`, applying
/// the LockSignature range model at collection so every consumer (the radar
/// pick, lock validity, the threat set) inherits it:
///
/// - Only physical, movable bodies are lockable. This skips static sensor
///   volumes such as scenario trigger areas (`RigidBody::Static`), which are
///   invisible and must never be locked. Two exceptions sit on rails (Static)
///   yet are visible things the player navigates by: gravity-well sources and
///   bodies with an AUTHORED LockSignature (nav beacons) - trigger areas
///   never carry a signature, so the invisible-statics rule holds.
/// - A freshly launched torpedo that has not committed its target yet is
///   skipped (it spawns right on the aim ray); once committed it is a normal
///   lockable body.
/// - Range: well bodies and ships return a signature at any range; committed
///   torpedoes at combat range; signed bodies at `signature * range/unit`
///   (floored at the debris range); unsigned debris only point-blank.
/// - `incumbents` (current locks / the radar candidate) hold a little beyond
///   their gate ([`TargetingSettings::range_hysteresis`]) so a body at the
///   boundary cannot strobe its lock as the ship drifts.
pub(super) fn collect_lockable(
    q_candidates: &LockableQuery,
    settings: &TargetingSettings,
    origin: Vec3,
    ship_entity: Entity,
    ship_allegiance: Option<&Allegiance>,
    incumbents: &[Option<Entity>],
) -> Vec<Lockable> {
    q_candidates
        .iter()
        .filter_map(
            |(
                entity,
                transform,
                rigid_body,
                well,
                signature,
                is_ship,
                is_torpedo,
                torpedo_committed,
                allegiance,
            )| {
                if !matches!(rigid_body, RigidBody::Dynamic)
                    && well.is_none()
                    && signature.is_none()
                {
                    return None;
                }
                // Never lock the player's own ship.
                if entity == ship_entity {
                    return None;
                }
                if is_torpedo.is_some() && torpedo_committed.is_none() {
                    return None;
                }
                let mut max_range = if well.is_some() || is_ship {
                    TARGETING_MAX_RANGE
                } else if is_torpedo.is_some() {
                    settings.torpedo_lock_range
                } else {
                    signature.map_or(settings.unsigned_lock_range, |signature| {
                        (settings.signature_range_per_unit * **signature)
                            .max(settings.unsigned_lock_range)
                    })
                };
                if incumbents.contains(&Some(entity)) {
                    max_range *= settings.range_hysteresis.max(1.0);
                }
                let position = transform.translation();
                if position.distance_squared(origin) > max_range * max_range {
                    return None;
                }
                let is_hostile = relation(ship_allegiance, allegiance) == Relation::Hostile;
                let is_combat_target = is_ship || is_torpedo.is_some();
                Some((entity, position, is_hostile, is_combat_target))
            },
        )
        .collect()
}

/// Per-frame lock upkeep, always on: hold the LOCKS only while their targets
/// stay collectible (death/despawn and out-of-range clear them - stickiness
/// never needs a re-pick because NOTHING re-picks), clear the combat lock
/// when a hostile target turns non-hostile, tick the [`CombatDecay`] idle
/// clock, and maintain the ranked hostile [`ThreatContacts`]
/// for the edge indicators.
#[expect(
    clippy::type_complexity,
    reason = "one query term per contact and lock input"
)]
pub(super) fn update_contacts_and_locks(
    time: Res<Time>,
    look_ray: ActiveLookRay,
    settings: Res<TargetingSettings>,
    q_candidates: LockableQuery,
    q_flipped: Query<(), Changed<Allegiance>>,
    q_allegiances: Query<&Allegiance>,
    // Only WEAPON sections carry a trigger, so the filter skips hull,
    // thrusters and controllers rather than walking every section.
    q_triggers: Query<
        (
            &ChildOf,
            Option<&TurretSectionInput>,
            Option<&TorpedoSectionInput>,
        ),
        Or<(With<TurretSectionInput>, With<TorpedoSectionInput>)>,
    >,
    mut dropped: MessageWriter<CombatLockDropped>,
    mut spaceship: Query<
        (
            &Transform,
            Option<&ComputedCenterOfMass>,
            Entity,
            Option<&Allegiance>,
            &mut TravelLock,
            &mut CombatLock,
            &mut CombatDecay,
            &mut ThreatContacts,
            Option<&WeaponsRaised>,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    for (
        transform,
        com,
        ship,
        ship_allegiance,
        mut travel,
        mut combat,
        mut decay,
        mut threats,
        raised,
    ) in &mut spaceship
    {
        // Cone origin on the live structure, not the root origin, so the
        // scanner agrees with the COM-anchored crosshair after losing
        // sections.
        let origin = live_structure_anchor(transform, com);
        let candidates = collect_lockable(
            &q_candidates,
            &settings,
            origin,
            ship,
            ship_allegiance,
            &[travel.0, combat.0],
        );

        // Validity: a lock holds exactly while its target is collectible.
        let still = |target: Option<Entity>| {
            target.filter(|target| candidates.iter().any(|&(entity, ..)| entity == *target))
        };
        let travel_now = still(travel.0);
        if travel.0 != travel_now {
            travel.0 = travel_now;
        }
        let mut combat_now = still(combat.0);
        // Name the branch that let go, rather than leaving the owner (and any
        // future investigation) to infer it from the wreckage. A target
        // that vanished from the candidate set is either GONE (despawned, or
        // no longer a lockable body at all) or merely OUT OF RANGE, and the
        // query tells the two apart.
        if let (Some(target), None) = (combat.0, combat_now) {
            let reason = if q_candidates.get(target).is_ok() {
                CombatLockDrop::OutOfRange
            } else {
                CombatLockDrop::TargetGone
            };
            report_combat_lock_drop(&mut dropped, target, reason, decay.0);
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
            report_combat_lock_drop(
                &mut dropped,
                target,
                CombatLockDrop::AllegianceFlip,
                decay.0,
            );
        }

        // The idle decay (D4): combat activity resets the clock; at
        // COMBAT_DECAY_SECS the lock lets go and the safety follows. Combat
        // activity is the raised stance OR a held trigger on one of this
        // ship's own weapon sections - the stance is only raised while the
        // combat button is HELD (`derive_control_mode_and_raised`), and
        // `WeaponsHot` is `raised OR locked`, so a player who locks a hostile
        // and fights with the stance lowered is unmistakably in combat. Until
        // firing did NOT count (the comments here promised it as part of,
        // which closed without the wiring) and the lock let go mid-fight at
        // 30 s - the defect behind the owner's "sometimes the ship loses
        // radar focus" report.
        if combat_now.is_some() {
            let firing = q_triggers
                .iter()
                .any(|(&ChildOf(parent), turret, torpedo)| {
                    parent == ship
                        && (turret.is_some_and(|turret| turret.0)
                            || torpedo.is_some_and(|torpedo| torpedo.0))
                });
            if raised.is_some_and(|raised| raised.0) || firing {
                decay.set_if_neq(CombatDecay(0.0));
            } else {
                decay.0 += time.delta_secs();
                if decay.0 >= COMBAT_DECAY_SECS {
                    if let Some(target) = combat_now {
                        report_combat_lock_drop(
                            &mut dropped,
                            target,
                            CombatLockDrop::IdleDecay,
                            decay.0,
                        );
                    }
                    combat_now = None;
                    decay.set_if_neq(CombatDecay(0.0));
                }
            }
        } else {
            decay.set_if_neq(CombatDecay(0.0));
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
            candidates
                .iter()
                .filter(|&&(_, _, is_hostile, is_combat)| is_hostile && is_combat)
                .map(|&(entity, position, ..)| (entity, position)),
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
/// before, which is what made an intended 30 s decay read as a random bug.
fn report_combat_lock_drop(
    dropped: &mut MessageWriter<CombatLockDropped>,
    target: Entity,
    reason: CombatLockDrop,
    idle_secs: f32,
) {
    debug!("combat lock dropped: target {target} - {reason:?} after {idle_secs:.2} s idle");
    dropped.write(CombatLockDropped {
        target,
        reason,
        idle_secs,
    });
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
        world.init_resource::<Messages<CombatLockDropped>>();
        let travel_target = world
            .spawn((
                RigidBody::Static,
                LockSignature(20.0),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -300.0)),
            ))
            .id();
        let combat_target = world
            .spawn((
                SpaceshipRootMarker,
                AISpaceshipMarker,
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -400.0)),
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
        let upkeep_id = world.register_system(update_contacts_and_locks);
        world.insert_resource(UpkeepSystem(upkeep_id));
        world.run_system(upkeep_id).unwrap();
        world.get_mut::<TravelLock>(player).unwrap().0 = Some(travel_target);
        world.get_mut::<CombatLock>(player).unwrap().0 = Some(combat_target);
        (world, player, travel_target, combat_target)
    }

    #[derive(Resource)]
    struct UpkeepSystem(bevy::ecs::system::SystemId);

    fn upkeep(world: &mut World) {
        let id = world.resource::<UpkeepSystem>().0;
        world.run_system(id).unwrap();
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
    /// and records the branch BY NAME plus the elapsed idle clock, so the
    /// answer is read off the run instead of guessed. Numbers land in the
    /// task's NOTES.md.
    #[test]
    fn the_evidence_rig_names_every_branch_that_drops_the_combat_lock() {
        // 1. Idle decay: nothing happens for 30 s with the stance lowered.
        let (mut world, player, _travel, combat_target) = locked_world();
        upkeep(&mut world);
        assert!(drops(&mut world).is_empty(), "a healthy lock drops nothing");
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(COMBAT_DECAY_SECS));
        upkeep(&mut world);
        let idle = drops(&mut world);
        assert_eq!(
            idle,
            vec![CombatLockDropped {
                target: combat_target,
                reason: CombatLockDrop::IdleDecay,
                idle_secs: COMBAT_DECAY_SECS,
            }],
            "the idle branch names itself with the clock it crossed"
        );
        assert_eq!(world.get::<CombatLock>(player).unwrap().0, None);

        // 2. Out of range: the target exists and is still lockable, it is
        // simply too far - even past the incumbent's widened gate.
        let (mut world, player, _travel, combat_target) = locked_world();
        let past_gate = TARGETING_MAX_RANGE * 1.15 + 1.0;
        world
            .entity_mut(combat_target)
            .insert(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -past_gate,
            )));
        upkeep(&mut world);
        assert_eq!(
            drops(&mut world),
            vec![CombatLockDropped {
                target: combat_target,
                reason: CombatLockDrop::OutOfRange,
                idle_secs: 0.0,
            }],
            "a body still in the world but past its gate reads OutOfRange"
        );
        assert_eq!(world.get::<CombatLock>(player).unwrap().0, None);

        // 3. Target gone: despawned outright.
        let (mut world, player, _travel, combat_target) = locked_world();
        world.despawn(combat_target);
        upkeep(&mut world);
        assert_eq!(
            drops(&mut world),
            vec![CombatLockDropped {
                target: combat_target,
                reason: CombatLockDrop::TargetGone,
                idle_secs: 0.0,
            }],
            "a despawned target reads TargetGone, never OutOfRange"
        );
        assert_eq!(world.get::<CombatLock>(player).unwrap().0, None);

        // 4. Allegiance flip: the scripted surrender.
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
                idle_secs: 0.0,
            }],
            "a surrender names itself, so it cannot be mistaken for decay"
        );
        assert_eq!(world.get::<CombatLock>(player).unwrap().0, None);
    }

    /// THE DEFECT: firing IS combat activity. The stance is only raised while
    /// the combat button is HELD, and `WeaponsHot` is `raised OR locked`, so
    /// a player can legitimately fight with it lowered - and before this fix
    /// the lock let go at 30 s mid-fight, with nothing on screen to explain
    /// it.
    #[test]
    fn a_held_trigger_resets_the_decay_so_the_lock_survives_a_long_fight() {
        let (mut world, player, _travel, combat_target) = locked_world();
        hold_trigger(&mut world, player);

        // Two full decay windows of continuous firing, stance LOWERED.
        for _ in 0..4 {
            world
                .resource_mut::<Time>()
                .advance_by(Duration::from_secs_f32(COMBAT_DECAY_SECS * 0.5));
            upkeep(&mut world);
            assert_eq!(
                world.get::<CombatDecay>(player).unwrap().0,
                0.0,
                "a held trigger resets the idle clock every frame"
            );
        }
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            Some(combat_target),
            "firing at a locked hostile must not lose the lock mid-fight"
        );
        assert!(
            drops(&mut world).is_empty(),
            "and nothing reports a drop at all"
        );
    }

    /// The other half of the same rule: releasing the trigger resumes the
    /// clock from zero, so the decay still happens - the fix widens what
    /// counts as combat, it does not disable the rule.
    #[test]
    fn releasing_the_trigger_resumes_the_decay_from_zero() {
        let (mut world, player, _travel, combat_target) = locked_world();
        let turret = hold_trigger(&mut world, player);
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(COMBAT_DECAY_SECS - 1.0));
        upkeep(&mut world);

        world.entity_mut(turret).insert(TurretSectionInput(false));
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(COMBAT_DECAY_SECS - 1.0));
        upkeep(&mut world);
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            Some(combat_target),
            "the clock restarts at the release, it does not resume mid-window"
        );

        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(2.0));
        upkeep(&mut world);
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            None,
            "a genuinely idle lock still decays at {COMBAT_DECAY_SECS} s"
        );
    }

    /// A trigger held on SOMEONE ELSE'S ship is not this player's combat
    /// activity - the sibling-scope trap the `ChildOf` filter guards.
    #[test]
    fn another_ships_trigger_never_holds_the_players_lock_open() {
        let (mut world, player, _travel, _combat_target) = locked_world();
        let other = world.spawn((SpaceshipRootMarker, Transform::IDENTITY)).id();
        hold_trigger(&mut world, other);

        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(COMBAT_DECAY_SECS));
        upkeep(&mut world);
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            None,
            "only the player's OWN sections count as their combat activity"
        );
    }

    /// THE PIN of the ruled-out paths: with the target alive, in range, still
    /// hostile and no gesture, the lock does NOT drop for any reason other
    /// than the idle clock - and while that clock is held at zero it does not
    /// drop at all. This is what stops the investigation's answer from
    /// silently rotting.
    #[test]
    fn a_live_in_range_hostile_lock_never_drops_for_any_other_reason() {
        let (mut world, player, _travel, combat_target) = locked_world();
        hold_trigger(&mut world, player);

        // Ten minutes of firing at a target that drifts within its gate.
        for step in 0..600 {
            let distance = 400.0 + (step as f32) * 10.0;
            world
                .entity_mut(combat_target)
                .insert(GlobalTransform::from_translation(Vec3::new(
                    0.0, 0.0, -distance,
                )));
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
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -500.0)),
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
        // ship (full-range class) survives.
        world
            .entity_mut(travel_target)
            .insert(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -900.0,
            )));
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

    /// D4: the combat lock decays after COMBAT_DECAY_SECS idle; the raised
    /// stance resets the clock (the delivery guard - the same span with
    /// activity does NOT decay).
    #[test]
    fn combat_lock_decays_after_idle_and_raised_resets_the_clock() {
        let (mut world, player, _travel, combat_target) = locked_world();

        // 29 idle seconds: still locked.
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(29.0));
        upkeep(&mut world);
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            Some(combat_target)
        );

        // Raised at the brink: the clock resets, another 29 s stays locked.
        world.entity_mut(player).insert(WeaponsRaised(true));
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(2.0));
        upkeep(&mut world);
        world.entity_mut(player).insert(WeaponsRaised(false));
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(29.0));
        upkeep(&mut world);
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            Some(combat_target),
            "activity resets the decay clock"
        );

        // Two more idle seconds cross the threshold: cleared.
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(2.0));
        upkeep(&mut world);
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            None,
            "an idle combat lock decays at {COMBAT_DECAY_SECS} s"
        );
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
            GlobalTransform::from_translation(Vec3::new(50.0, 0.0, -100.0)),
        ));
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetChosen,
                Allegiance::Enemy,
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(0.0, 10.0, -200.0)),
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
