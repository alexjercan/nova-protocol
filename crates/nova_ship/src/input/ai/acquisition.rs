//! Who an AI ship fights: primary target selection over the relation model
//! ([`AITarget`]), the inbound-torpedo override the guns defend against
//! ([`AIPointDefenseTarget`]), and the mirror that publishes the AI's
//! engagement onto the shared combat components.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

#[cfg(test)]
use super::behavior::update_behavior_state;
#[cfg(test)]
use super::guns::{on_projectile_input, update_turret_target_input, AI_BURST_FIRE_SECS};
#[cfg(test)]
use super::point_defense::update_turret_point_defense;
use super::threat::AI_THREAT_ATTACKER_DISCOUNT;
use crate::prelude::*;

/// The entity this AI ship currently fights - what every AI behavior system
/// aims, chases and shoots at. Written by `update_ai_target` from the
/// relation model; `None` means nothing hostile in acquisition range, which
/// `update_behavior_state` turns into `Idle`.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct AITarget(pub Option<Entity>);

/// Mirror the AI's engagement onto the shared combat components (deliberate-
/// radar AI parity): `CombatLock` = the point-defense override else the
/// primary target, refreshed every frame (the AI's own acquisition hygiene
/// replaces the player's validity/decay upkeep), and the stance is raised
/// while engaged - so the shared section-side weapons-safety gate never
/// silences a fighting AI, and the AI's guns go SAFE the moment it
/// disengages. Instant acquisition for AI is the accepted spec (the human
/// gesture is the deliberate part, not the machine's).
pub(super) fn mirror_ai_combat_state(
    mut commands: Commands,
    mut q_ships: Query<
        (
            Entity,
            &AITarget,
            &AIPointDefenseTarget,
            Option<&mut CombatLock>,
            Option<&mut WeaponsRaised>,
        ),
        With<AISpaceshipMarker>,
    >,
) {
    for (ship, target, pd_target, lock, raised) in &mut q_ships {
        let engaged = pd_target.0.or(target.0);
        let managed = lock.is_some();
        match lock {
            Some(mut lock) => {
                if lock.0 != engaged {
                    lock.0 = engaged;
                }
            }
            None => {
                commands.entity(ship).insert(CombatLock(engaged));
            }
        }
        let is_raised = engaged.is_some();
        match raised {
            Some(mut raised) => {
                raised.set_if_neq(WeaponsRaised(is_raised));
            }
            None => {
                commands.entity(ship).insert(WeaponsRaised(is_raised));
            }
        }
        // WeaponsHot itself is derived by the shared safety system; give the
        // ship the component so it becomes a MANAGED ship.
        if !managed {
            commands.entity(ship).insert(WeaponsHot::default());
        }
    }
}

/// What kind of body a target candidate is. Priority TIER, not a score tweak:
/// hostile ships always beat hostile torpedoes (the urgency flip for an
/// incoming torpedo is the point-defense).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum AITargetKind {
    Ship,
    Torpedo,
}

/// Acquisition range (m) of AI target selection. Deliberately shorter than
/// the player's TARGETING_MAX_RANGE (20 km): the player's lock doubles as a
/// long-range designator for GOTO legs and torpedo launches, while AI
/// sensors only need to find things worth fighting.
const AI_TARGET_MAX_RANGE: f32 = 2000.0;
/// Switch hysteresis: the current target's distance is discounted by this
/// factor, so a rival has to be meaningfully closer (not a frame-noise
/// sliver) to steal the pick.
const AI_TARGET_HYSTERESIS_DISCOUNT: f32 = 0.8;

/// Choose the best target from `candidates`: highest priority tier first
/// ([`AITargetKind`] order), nearest within the tier, with the current
/// target's distance discounted by [`AI_TARGET_HYSTERESIS_DISCOUNT`] so the
/// pick does not flip-flop between two comparably distant hostiles, and the
/// ship that recently damaged me discounted by
/// [`AI_THREAT_ATTACKER_DISCOUNT`] so whoever is shooting me steals the
/// pick from comparably distant bystanders (the discounts stack). Out of
/// [`AI_TARGET_MAX_RANGE`] (or with no candidates) the pick is `None`.
/// Pure for unit testing.
fn pick_ai_target(
    own_anchor: Vec3,
    current: Option<Entity>,
    attacker: Option<Entity>,
    candidates: impl Iterator<Item = (Entity, Vec3, AITargetKind)>,
) -> Option<Entity> {
    candidates
        .filter_map(|(entity, position, kind)| {
            let mut distance = own_anchor.distance(position);
            if distance > AI_TARGET_MAX_RANGE || distance <= f32::EPSILON {
                return None;
            }
            if current == Some(entity) {
                distance *= AI_TARGET_HYSTERESIS_DISCOUNT;
            }
            if attacker == Some(entity) {
                distance *= AI_THREAT_ATTACKER_DISCOUNT;
            }
            Some((entity, kind, distance))
        })
        .min_by(|(_, kind_a, dist_a), (_, kind_b, dist_b)| {
            kind_a.cmp(kind_b).then(dist_a.total_cmp(dist_b))
        })
        .map(|(entity, _, _)| entity)
}

/// Acquire each AI ship's [`AITarget`] over the relation model: every
/// hostile ship root or committed hostile torpedo inside acquisition range
/// is a candidate; [`pick_ai_target`] scores them. Runs first in the AI
/// chain - acquisition drives engagement, so a ship in `Idle` still scans.
#[expect(
    clippy::type_complexity,
    reason = "one query term per target-selection input"
)]
pub(super) fn update_ai_target(
    q_candidates: Query<(
        Entity,
        &Transform,
        Option<&ComputedCenterOfMass>,
        Option<&Allegiance>,
        Has<SpaceshipRootMarker>,
        Option<&TorpedoProjectileMarker>,
        Option<&TorpedoTargetChosen>,
        Has<NeutralizedMarker>,
    )>,
    mut q_spaceship: Query<
        (
            Entity,
            &Transform,
            Option<&ComputedCenterOfMass>,
            &Allegiance,
            &AIThreat,
            &mut AITarget,
            Has<AINonCombatant>,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
) {
    for (ship, transform, com, own_allegiance, threat, mut target, non_combatant) in
        &mut q_spaceship
    {
        // A non-combatant never fights: keep its target clear so the behavior
        // FSM holds the passive routine. Cleared defensively in case the ship
        // was armed when it last acquired one (a future critical-damage path
        // could flip this flag mid-fight).
        if non_combatant {
            if target.is_some() {
                **target = None;
            }
            continue;
        }
        let own_anchor = live_structure_anchor(transform, com);
        let candidates = q_candidates.iter().filter_map(
            |(
                entity,
                c_transform,
                c_com,
                allegiance,
                is_ship,
                is_torpedo,
                committed,
                neutralized,
            )| {
                if entity == ship || neutralized {
                    return None;
                }
                // Hostility comes from the relation model: the player's ship
                // and projectiles are hostile to an Enemy-aligned AI, other
                // AI ships and neutral bodies (asteroids) are not.
                if relation(Some(own_allegiance), allegiance) != Relation::Hostile {
                    return None;
                }
                let kind = if is_ship {
                    AITargetKind::Ship
                } else if is_torpedo.is_some() {
                    // Only committed torpedoes are targets, matching the
                    // player targeting rule: a just-launched torpedo has not
                    // decided what it is yet.
                    committed?;
                    AITargetKind::Torpedo
                } else {
                    return None;
                };
                Some((entity, live_structure_anchor(c_transform, c_com), kind))
            },
        );

        let next = pick_ai_target(own_anchor, **target, threat.recent_attacker(), candidates);
        // Change-detection hygiene: only write on a real change. A dead or
        // out-of-range target clears here (the pick simply no longer finds
        // it), so consumers never chase a stale entity.
        if **target != next {
            **target = next;
        }
    }
}

/// The inbound torpedo this ship's guns are currently defending against. When
/// set, it OVERRIDES the primary [`AITarget`] for turret aim and fire - the
/// PDC role is the turrets' main purpose - while flight keeps chasing the
/// primary target. Written by `update_point_defense_target`.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct AIPointDefenseTarget(pub Option<Entity>);

/// Range (u) inside which an inbound hostile torpedo pulls the guns off the
/// primary target. Kept inside the default turret's fire gate
/// (muzzle_speed * lifetime * margin = 180 u) so a defending turret can
/// actually reach what it defends against, and moves with any lifetime change
/// - see AI_FIRE_RANGE_FACTOR in `guns.rs`.
///
/// It is also the ammunition knob. Point defense bypasses the burst cadence
/// and holds the trigger for one full time of flight before the first rounds
/// arrive, so rounds spent per intercept are
/// `fire_rate * pd_range / (muzzle_speed + torpedo_speed)`: ~111 at 150 u
/// against a standard torpedo, where the shipped 400 u burned ~296 for the
/// same 2-round kill.
pub(super) const AI_POINT_DEFENSE_RANGE: f32 = 150.0;

/// Per-ship override of the point-defense range: this ship's guns hold their
/// fire until an inbound hostile torpedo is inside THIS range instead of the
/// default [`AI_POINT_DEFENSE_RANGE`]. Author it SHORT to stage close-in
/// intercepts (a display scene that wants the kill to happen in frame, a
/// ship that conserves ammo); authoring it past the turret's own reach just
/// wastes the opening shots. Authored via `AIControllerConfig::pd_range`.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AIPointDefenseRange(pub f32);

/// Choose the torpedo to defend against: ones hunting THIS ship outrank
/// ones hunting someone else (a tier, like the ship/torpedo target tiers),
/// nearest wins within a tier, nothing outside `pd_range` (the authored
/// [`AIPointDefenseRange`] or the [`AI_POINT_DEFENSE_RANGE`] default). Pure
/// for unit testing.
fn pick_point_defense_target(
    own_anchor: Vec3,
    candidates: impl Iterator<Item = (Entity, Vec3, bool)>,
    pd_range: f32,
) -> Option<Entity> {
    candidates
        .filter_map(|(entity, position, targeting_me)| {
            let distance = own_anchor.distance(position);
            if distance > pd_range || distance <= f32::EPSILON {
                return None;
            }
            // false < true, so invert: hunting-me sorts first.
            Some((entity, !targeting_me, distance))
        })
        .min_by(|(_, me_a, dist_a), (_, me_b, dist_b)| {
            me_a.cmp(me_b).then(dist_a.total_cmp(dist_b))
        })
        .map(|(entity, _, _)| entity)
}

/// Acquire each AI ship's [`AIPointDefenseTarget`]: hostile committed
/// torpedoes inside point-defense range, preferring ones whose
/// [`TorpedoTargetEntity`] is this ship. Runs right after primary
/// acquisition; the turret systems consume the override the same frame.
///
/// An [`AINonCombatant`] hull defends against nothing, exactly as it acquires
/// nothing above - point defense is a crew job, and a neutralized wreck has no
/// crew. Cleared rather than skipped so a ship neutralized mid-intercept drops
/// the torpedo it was tracking instead of holding a stale pick forever.
#[expect(
    clippy::type_complexity,
    reason = "one query term per point-defense target input"
)]
pub(super) fn update_point_defense_target(
    q_torpedoes: Query<
        (
            Entity,
            &Transform,
            Option<&Allegiance>,
            Option<&TorpedoTargetEntity>,
        ),
        (With<TorpedoProjectileMarker>, With<TorpedoTargetChosen>),
    >,
    mut q_spaceship: Query<
        (
            Entity,
            &Transform,
            Option<&ComputedCenterOfMass>,
            &Allegiance,
            Option<&AIPointDefenseRange>,
            &mut AIPointDefenseTarget,
            Has<AINonCombatant>,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
) {
    for (ship, transform, com, own_allegiance, pd_range, mut pd_target, non_combatant) in
        &mut q_spaceship
    {
        if non_combatant {
            if pd_target.is_some() {
                **pd_target = None;
            }
            continue;
        }
        let own_anchor = live_structure_anchor(transform, com);
        let candidates =
            q_torpedoes
                .iter()
                .filter_map(|(entity, t_transform, allegiance, torpedo_target)| {
                    if relation(Some(own_allegiance), allegiance) != Relation::Hostile {
                        return None;
                    }
                    let targeting_me = torpedo_target.map(|t| **t) == Some(ship);
                    Some((entity, t_transform.translation, targeting_me))
                });

        let next = pick_point_defense_target(
            own_anchor,
            candidates,
            pd_range.map_or(AI_POINT_DEFENSE_RANGE, |range| range.0),
        );
        // Change-detection hygiene, and stale-entity safety as with AITarget.
        if **pd_target != next {
            **pd_target = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// AI parity mirror: engagement -> CombatLock + raised stance + a managed
    /// WeaponsHot, so the shared section-side safety gate never silences a
    /// fighting AI; the point-defense override wins; disengaging safes the
    /// guns.
    #[test]
    fn ai_engagement_mirrors_onto_the_combat_components() {
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        let enemy = world.spawn_empty().id();
        let torpedo = world.spawn_empty().id();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AITarget(None),
                AIPointDefenseTarget(None),
            ))
            .id();

        // Disengaged: managed but safe once the safety derivation runs.
        world.run_system_once(mirror_ai_combat_state).unwrap();
        world
            .run_system_once(crate::input::targeting::update_weapons_safety_for_tests)
            .unwrap();
        assert_eq!(world.get::<CombatLock>(ship).unwrap().0, None);
        assert!(
            !world.get::<WeaponsHot>(ship).unwrap().0,
            "disengaged AI is safe"
        );

        // Engaged on the primary target: locked, raised, hot.
        world.get_mut::<AITarget>(ship).unwrap().0 = Some(enemy);
        world.run_system_once(mirror_ai_combat_state).unwrap();
        world
            .run_system_once(crate::input::targeting::update_weapons_safety_for_tests)
            .unwrap();
        assert_eq!(world.get::<CombatLock>(ship).unwrap().0, Some(enemy));
        assert!(world.get::<WeaponsRaised>(ship).unwrap().0);
        assert!(world.get::<WeaponsHot>(ship).unwrap().0, "engaged AI fires");

        // Point defense overrides the primary.
        world.get_mut::<AIPointDefenseTarget>(ship).unwrap().0 = Some(torpedo);
        world.run_system_once(mirror_ai_combat_state).unwrap();
        assert_eq!(world.get::<CombatLock>(ship).unwrap().0, Some(torpedo));

        // Disengage everything: lock drops, stance lowers.
        world.get_mut::<AITarget>(ship).unwrap().0 = None;
        world.get_mut::<AIPointDefenseTarget>(ship).unwrap().0 = None;
        world.run_system_once(mirror_ai_combat_state).unwrap();
        world
            .run_system_once(crate::input::targeting::update_weapons_safety_for_tests)
            .unwrap();
        assert_eq!(world.get::<CombatLock>(ship).unwrap().0, None);
        assert!(!world.get::<WeaponsHot>(ship).unwrap().0);
    }

    #[test]
    fn ai_turrets_target_the_live_structure_anchor() {
        // AI fire must converge on the target's surviving structure, not the
        // root origin build-spot. Driven through the real acquisition system,
        // not a hand-set target.
        let mut world = World::new();
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            ComputedCenterOfMass(Vec3::new(0.0, 0.0, 3.0)),
        ));
        let ai_ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
                ChildOf(ai_ship),
            ))
            .id();

        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_turret_target_input).unwrap();

        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetInput>()
                .unwrap(),
            Some(Vec3::new(10.0, 0.0, 3.0)),
            "AI turret input = the player's live-structure anchor"
        );
    }

    #[test]
    fn ai_turrets_fall_back_to_the_origin_without_a_com() {
        let mut world = World::new();
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(1.0, 2.0, 3.0)),
        ));
        let ai_ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
                ChildOf(ai_ship),
            ))
            .id();

        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_turret_target_input).unwrap();

        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetInput>()
                .unwrap(),
            Some(Vec3::new(1.0, 2.0, 3.0))
        );
    }
}

#[cfg(test)]
mod target_selection_tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn entity(raw: u32) -> Entity {
        Entity::from_raw_u32(raw).unwrap()
    }

    #[test]
    fn nearest_wins_within_a_tier() {
        let near = entity(1);
        let far = entity(2);
        let picked = pick_ai_target(
            Vec3::ZERO,
            None,
            None,
            [
                (far, Vec3::new(0.0, 0.0, -500.0), AITargetKind::Ship),
                (near, Vec3::new(0.0, 0.0, -100.0), AITargetKind::Ship),
            ]
            .into_iter(),
        );
        assert_eq!(picked, Some(near));
    }

    #[test]
    fn a_ship_beats_a_nearer_torpedo() {
        // Tiered priority, not a distance tweak: the urgency flip for
        // incoming torpedoes is the point-defense task.
        let ship = entity(1);
        let torpedo = entity(2);
        let picked = pick_ai_target(
            Vec3::ZERO,
            None,
            None,
            [
                (torpedo, Vec3::new(0.0, 0.0, -50.0), AITargetKind::Torpedo),
                (ship, Vec3::new(0.0, 0.0, -1500.0), AITargetKind::Ship),
            ]
            .into_iter(),
        );
        assert_eq!(picked, Some(ship));
    }

    #[test]
    fn hysteresis_holds_the_current_pick_against_slivers() {
        let current = entity(1);
        let rival = entity(2);
        // The rival is 10% closer: inside the 20% hysteresis discount, the
        // current target holds.
        let held = pick_ai_target(
            Vec3::ZERO,
            Some(current),
            None,
            [
                (current, Vec3::new(0.0, 0.0, -1000.0), AITargetKind::Ship),
                (rival, Vec3::new(0.0, 0.0, -900.0), AITargetKind::Ship),
            ]
            .into_iter(),
        );
        assert_eq!(held, Some(current), "a sliver does not steal the pick");

        // At 2x closer the rival wins even against the discount.
        let stolen = pick_ai_target(
            Vec3::ZERO,
            Some(current),
            None,
            [
                (current, Vec3::new(0.0, 0.0, -1000.0), AITargetKind::Ship),
                (rival, Vec3::new(0.0, 0.0, -500.0), AITargetKind::Ship),
            ]
            .into_iter(),
        );
        assert_eq!(stolen, Some(rival), "a real gap does steal the pick");
    }

    #[test]
    fn out_of_range_or_empty_picks_nothing() {
        assert_eq!(
            pick_ai_target(
                Vec3::ZERO,
                None,
                None,
                [(entity(1), Vec3::new(0.0, 0.0, -2500.0), AITargetKind::Ship)].into_iter(),
            ),
            None,
            "beyond acquisition range"
        );
        assert_eq!(
            pick_ai_target(Vec3::ZERO, None, None, std::iter::empty()),
            None,
            "no candidates"
        );
    }

    #[test]
    fn the_recent_attacker_steals_the_pick_from_a_comparable_bystander() {
        // Recently-damaged-me threat scoring: the
        // ship shooting me outranks a somewhat closer bystander...
        let attacker = entity(1);
        let bystander = entity(2);
        let picked = pick_ai_target(
            Vec3::ZERO,
            None,
            Some(attacker),
            [
                (attacker, Vec3::new(0.0, 0.0, -1000.0), AITargetKind::Ship),
                (bystander, Vec3::new(0.0, 0.0, -700.0), AITargetKind::Ship),
            ]
            .into_iter(),
        );
        assert_eq!(picked, Some(attacker), "the shooter draws the aggro");

        //...but the bias is a discount, not a tier: a bystander well
        // inside the discounted distance still wins.
        let picked = pick_ai_target(
            Vec3::ZERO,
            None,
            Some(attacker),
            [
                (attacker, Vec3::new(0.0, 0.0, -1000.0), AITargetKind::Ship),
                (bystander, Vec3::new(0.0, 0.0, -300.0), AITargetKind::Ship),
            ]
            .into_iter(),
        );
        assert_eq!(picked, Some(bystander), "a far closer threat still wins");
    }

    #[test]
    fn acquisition_prefers_the_hostile_ship_and_ignores_non_hostiles() {
        let mut world = World::new();
        let ai_ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        // A fellow AI ship (Own), a neutral asteroid-like body (no
        // allegiance), and an uncommitted hostile torpedo: all ignored.
        world.spawn((
            AISpaceshipMarker,
            Transform::from_translation(Vec3::new(20.0, 0.0, 0.0)),
        ));
        world.spawn(Transform::from_translation(Vec3::new(30.0, 0.0, 0.0)));
        world.spawn((
            TorpedoProjectileMarker,
            Allegiance::Player,
            Transform::from_translation(Vec3::new(40.0, 0.0, 0.0)),
        ));
        // A committed hostile torpedo nearer than the hostile ship: the
        // ship still wins the tier.
        world.spawn((
            TorpedoProjectileMarker,
            TorpedoTargetChosen,
            Allegiance::Player,
            Transform::from_translation(Vec3::new(50.0, 0.0, 0.0)),
        ));
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(500.0, 0.0, 0.0)),
            ))
            .id();

        world.run_system_once(update_ai_target).unwrap();

        assert_eq!(
            **world.entity(ai_ship).get::<AITarget>().unwrap(),
            Some(player),
            "the hostile SHIP wins over the nearer hostile torpedo; \
             own/neutral/uncommitted bodies are never candidates"
        );
    }

    #[test]
    fn a_committed_hostile_torpedo_is_acquired_when_no_ship_remains() {
        let mut world = World::new();
        let ai_ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetChosen,
                Allegiance::Player,
                Transform::from_translation(Vec3::new(50.0, 0.0, 0.0)),
            ))
            .id();

        world.run_system_once(update_ai_target).unwrap();

        assert_eq!(
            **world.entity(ai_ship).get::<AITarget>().unwrap(),
            Some(torpedo)
        );
    }

    #[test]
    fn a_neutralized_target_clears_on_the_next_pick() {
        let mut world = World::new();
        let ai_ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
            ))
            .id();

        world.run_system_once(update_ai_target).unwrap();
        assert_eq!(
            **world.entity(ai_ship).get::<AITarget>().unwrap(),
            Some(player)
        );

        world.entity_mut(player).insert(NeutralizedMarker);
        world.run_system_once(update_ai_target).unwrap();
        assert_eq!(
            **world.entity(ai_ship).get::<AITarget>().unwrap(),
            None,
            "AI stops treating a neutralized wreck as an active target"
        );
    }

    #[test]
    fn a_dead_target_clears_on_the_next_pick() {
        let mut world = World::new();
        let ai_ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
            ))
            .id();

        world.run_system_once(update_ai_target).unwrap();
        assert_eq!(
            **world.entity(ai_ship).get::<AITarget>().unwrap(),
            Some(player)
        );

        world.despawn(player);
        world.run_system_once(update_ai_target).unwrap();
        assert_eq!(
            **world.entity(ai_ship).get::<AITarget>().unwrap(),
            None,
            "consumers must never chase a stale entity"
        );
    }
}

// Ch3's Lifeline convoy is the first content to put `Allegiance::Player` on
// an `AISpaceshipMarker` root - an AI-flown ship on the player's SIDE. These
// rigs prove the relation model treats it as a first-class combatant on both
// ends of acquisition, driven through the real systems (update_ai_target +
// update_behavior_state), never a hand-set target. The spawn-path half
// (authored config -> component, beating the marker's Enemy requirement
// default) is pinned in nova_scenario's
// `authored_allegiance_overrides_the_controller_default`.

#[cfg(test)]
mod ally_relation_tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn run_pipeline(world: &mut World) {
        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_behavior_state).unwrap();
    }

    #[test]
    fn enemy_and_ally_ai_ships_acquire_each_other() {
        let mut world = World::new();
        world.init_resource::<Time>();
        let enemy = world.spawn((AISpaceshipMarker, Transform::default())).id();
        // The ally: exactly what the scenario's allegiance override leaves
        // behind - the explicit component wins over the marker's required
        // Enemy default.
        let ally = world
            .spawn((
                AISpaceshipMarker,
                Allegiance::Player,
                Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
            ))
            .id();
        // Park both Idle so the assert proves the engage PULL, not the
        // Engage spawn default.
        world.entity_mut(enemy).insert(AIBehaviorState::Idle);
        world.entity_mut(ally).insert(AIBehaviorState::Idle);

        run_pipeline(&mut world);

        assert_eq!(
            world.get::<AITarget>(enemy).unwrap().0,
            Some(ally),
            "an enemy AI acquires a Player-allegiance AI ship"
        );
        assert_eq!(
            world.get::<AITarget>(ally).unwrap().0,
            Some(enemy),
            "the ally fights back: acquisition is symmetric over the relation"
        );
        for (ship, name) in [(enemy, "enemy"), (ally, "ally")] {
            assert_eq!(
                *world.get::<AIBehaviorState>(ship).unwrap(),
                AIBehaviorState::Engage,
                "{name} is pulled from Idle into the fight"
            );
        }
    }

    #[test]
    fn a_neutral_ai_ship_is_acquired_by_neither_side() {
        // Control for the rig above (same setup, Neutral instead of
        // Player): the delivery guard is the sibling test acquiring at the
        // same distance, so this None cannot pass vacuously.
        let mut world = World::new();
        world.init_resource::<Time>();
        let enemy = world.spawn((AISpaceshipMarker, Transform::default())).id();
        let bystander = world
            .spawn((
                AISpaceshipMarker,
                Allegiance::Neutral,
                Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
            ))
            .id();

        run_pipeline(&mut world);

        assert_eq!(
            world.get::<AITarget>(enemy).unwrap().0,
            None,
            "a Neutral ship is not a target"
        );
        assert_eq!(
            world.get::<AITarget>(bystander).unwrap().0,
            None,
            "and a Neutral ship acquires nothing itself"
        );
    }

    #[test]
    fn a_markerless_player_allegiance_root_is_acquired() {
        // Lifeline's shipped convoy shape: a `controller: None` hauler is a
        // bare SpaceshipRootMarker + Allegiance::Player - no AI or player
        // marker. The candidate query needs only the root marker plus a
        // hostile relation; pin that, so a future candidate-query refactor
        // cannot silently make the convoy untargetable while every marker-
        // based test stays green.
        let mut world = World::new();
        world.init_resource::<Time>();
        let raider = world.spawn((AISpaceshipMarker, Transform::default())).id();
        let hauler = world
            .spawn((
                SpaceshipRootMarker,
                Allegiance::Player,
                Transform::from_translation(Vec3::new(120.0, 0.0, 0.0)),
            ))
            .id();

        run_pipeline(&mut world);

        assert_eq!(
            world.get::<AITarget>(raider).unwrap().0,
            Some(hauler),
            "a stalled convoy hauler (no controller marker) still draws fire"
        );
    }

    #[test]
    fn the_nearest_hostile_draws_the_fire() {
        // Lifeline's screening premise: a raider near the convoy shoots
        // the convoy, not the distant player - fresh acquisition picks the
        // nearest hostile within the Ship tier, so positioning decides who
        // draws fire.
        let mut world = World::new();
        world.init_resource::<Time>();
        let raider = world.spawn((AISpaceshipMarker, Transform::default())).id();
        let hauler = world
            .spawn((
                AISpaceshipMarker,
                Allegiance::Player,
                Transform::from_translation(Vec3::new(150.0, 0.0, 0.0)),
            ))
            .id();
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(400.0, 0.0, 0.0)),
        ));

        run_pipeline(&mut world);

        assert_eq!(
            world.get::<AITarget>(raider).unwrap().0,
            Some(hauler),
            "the nearer convoy hauler draws the raider's fire over the distant player"
        );
    }
}

#[cfg(test)]
mod point_defense_tests {
    use avian3d::collider_tree::ColliderTrees;
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn entity(raw: u32) -> Entity {
        Entity::from_raw_u32(raw).unwrap()
    }

    #[test]
    fn a_torpedo_hunting_me_outranks_a_nearer_one_hunting_someone_else() {
        let mine = entity(1);
        let other = entity(2);
        // Both inside the ring, as fractions of it: the ring moves with
        // turret reach, and an absolute distance silently stops testing the
        // tier once it does.
        let picked = pick_point_defense_target(
            Vec3::ZERO,
            [
                (
                    other,
                    Vec3::new(0.0, 0.0, -AI_POINT_DEFENSE_RANGE * 0.2),
                    false,
                ),
                (
                    mine,
                    Vec3::new(0.0, 0.0, -AI_POINT_DEFENSE_RANGE * 0.8),
                    true,
                ),
            ]
            .into_iter(),
            AI_POINT_DEFENSE_RANGE,
        );
        assert_eq!(picked, Some(mine));
    }

    #[test]
    fn nearest_wins_within_a_threat_tier_and_range_gates() {
        let near = entity(1);
        let far = entity(2);
        assert_eq!(
            pick_point_defense_target(
                Vec3::ZERO,
                [
                    (
                        far,
                        Vec3::new(0.0, 0.0, -AI_POINT_DEFENSE_RANGE * 0.9),
                        true
                    ),
                    (
                        near,
                        Vec3::new(0.0, 0.0, -AI_POINT_DEFENSE_RANGE * 0.3),
                        true
                    ),
                ]
                .into_iter(),
                AI_POINT_DEFENSE_RANGE,
            ),
            Some(near)
        );
        assert_eq!(
            pick_point_defense_target(
                Vec3::ZERO,
                [(
                    near,
                    Vec3::new(0.0, 0.0, -AI_POINT_DEFENSE_RANGE * 1.2),
                    true
                )]
                .into_iter(),
                AI_POINT_DEFENSE_RANGE,
            ),
            None,
            "outside point-defense range: the primary target keeps the guns"
        );
    }

    /// An authored [`AIPointDefenseRange`] moves the gate: a torpedo the
    /// default range would engage is ignored until it crosses the shorter
    /// authored ring - the knob a display scene uses to stage its
    /// intercepts in frame.
    #[test]
    fn an_authored_pd_range_moves_the_gate() {
        let torpedo = entity(1);
        let range = AI_POINT_DEFENSE_RANGE * 0.8;
        let candidates = || [(torpedo, Vec3::new(0.0, 0.0, -range), true)].into_iter();
        assert_eq!(
            pick_point_defense_target(Vec3::ZERO, candidates(), AI_POINT_DEFENSE_RANGE),
            Some(torpedo),
            "the default range engages at {range}"
        );
        assert_eq!(
            pick_point_defense_target(Vec3::ZERO, candidates(), range * 0.5),
            None,
            "a shorter authored ring holds fire until the torpedo is closer"
        );
    }

    /// An AI ship engaged on the player, with a hostile committed torpedo
    /// hunting the AI ship inside point-defense range. Returns
    /// (world, ai_ship, player, torpedo, turret).
    fn defended_world() -> (World, Entity, Entity, Entity, Entity) {
        let mut world = World::new();
        // Empty collider trees for the fire gate's SpatialQuery (no
        // occluders in this rig; PD bypasses the gate anyway).
        world.init_resource::<ColliderTrees>();
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(300.0, 0.0, 0.0)),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        let ai_ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetChosen,
                TorpedoTargetEntity(ai_ship),
                Allegiance::Player,
                Transform::from_translation(Vec3::new(0.0, 0.0, -150.0)),
                LinearVelocity(Vec3::new(0.0, 0.0, 30.0)),
            ))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
                TurretSectionAimPoint(None),
                TurretSectionConfigHelper(TurretSectionConfig::default()),
                TurretSectionInput(false),
                TurretSectionMuzzleEntity(Entity::PLACEHOLDER),
                ChildOf(ai_ship),
            ))
            .id();
        (world, ai_ship, player, torpedo, turret)
    }

    #[test]
    fn the_guns_defend_while_the_hull_keeps_chasing_the_ship() {
        let (mut world, ai_ship, player, torpedo, turret) = defended_world();

        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_point_defense_target).unwrap();
        world.run_system_once(update_turret_target_input).unwrap();

        // Flight target: still the hostile SHIP (ship-first tiers).
        assert_eq!(
            **world.entity(ai_ship).get::<AITarget>().unwrap(),
            Some(player),
            "the hull keeps chasing the ship"
        );
        // Gun target: the inbound torpedo, position and velocity.
        assert_eq!(
            **world.entity(ai_ship).get::<AIPointDefenseTarget>().unwrap(),
            Some(torpedo)
        );
        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetInput>()
                .unwrap(),
            Some(Vec3::new(0.0, 0.0, -150.0)),
            "the guns aim at the torpedo, not the ship"
        );
        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetVelocity>()
                .unwrap(),
            Vec3::new(0.0, 0.0, 30.0),
            "the lead feed follows the gun target"
        );
    }

    #[test]
    fn point_defense_bypasses_the_burst_hold() {
        let (mut world, ai_ship, _, _, turret) = defended_world();
        // Muzzle at the origin facing -Z: dead on the torpedo at -150.
        let muzzle = world
            .spawn((TurretSectionBarrelMuzzleMarker, GlobalTransform::IDENTITY))
            .id();
        world
            .entity_mut(turret)
            .insert(TurretSectionMuzzleEntity(muzzle));
        // Force the cadence into a hold phase: bursts must not delay defense.
        {
            let mut entity = world.entity_mut(ai_ship);
            let mut cadence = entity.get_mut::<AIFireCadence>().unwrap();
            cadence.tick(AI_BURST_FIRE_SECS + 0.01);
            assert!(!cadence.firing);
        }

        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_point_defense_target).unwrap();
        world.run_system_once(on_projectile_input).unwrap();

        assert!(
            **world.entity(turret).get::<TurretSectionInput>().unwrap(),
            "PDC fires through the burst hold"
        );
    }

    #[test]
    fn an_idle_ship_still_defends_itself() {
        let (mut world, ai_ship, _, torpedo, turret) = defended_world();
        world.entity_mut(ai_ship).insert(AIBehaviorState::Idle);

        world.run_system_once(update_point_defense_target).unwrap();
        world.run_system_once(update_turret_target_input).unwrap();

        let _ = torpedo;
        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetInput>()
                .unwrap(),
            Some(Vec3::new(0.0, 0.0, -150.0)),
            "point defense applies in every behavior state"
        );
    }

    /// One defense frame, in the production chain order.
    fn defend(world: &mut World) {
        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_point_defense_target).unwrap();
        world.run_system_once(update_turret_point_defense).unwrap();
        world.run_system_once(update_turret_target_input).unwrap();
        world.run_system_once(on_projectile_input).unwrap();
    }

    /// The owner's bug, end to end: point defense deliberately bypasses the
    /// behavior state, so the passive routine a neutralized ship falls into
    /// never silenced its mounts - a wreck with nobody aboard kept swatting
    /// torpedoes. The TRIGGER is the claim, not the assignment, and the
    /// torpedo is still in flight when the hull is neutralized.
    #[test]
    fn a_neutralized_hull_stops_defending_itself() {
        let (mut world, ai_ship, _, torpedo, turret) = defended_world();
        world.add_observer(super::super::on_neutralized_stand_down);
        // Muzzle at the origin facing -Z: dead on the torpedo at -150.
        let muzzle = world
            .spawn((TurretSectionBarrelMuzzleMarker, GlobalTransform::IDENTITY))
            .id();
        world.entity_mut(turret).insert((
            TurretSectionMuzzleEntity(muzzle),
            // The mount's own pose: what the per-turret assignment bears from.
            // No arc, so it is the fail-open case and can reach anything.
            GlobalTransform::IDENTITY,
            AITurretDefenseTarget::default(),
        ));

        defend(&mut world);
        assert_eq!(
            **world.entity(turret).get::<AITurretDefenseTarget>().unwrap(),
            Some(torpedo),
            "the live hull takes the inbound"
        );
        assert!(
            **world.entity(turret).get::<TurretSectionInput>().unwrap(),
            "and holds the trigger down on it"
        );

        world.entity_mut(ai_ship).insert(NeutralizedMarker);
        // The stand-down runs as a command from the observer.
        world.flush();
        defend(&mut world);

        assert!(
            world.entity(ai_ship).contains::<AINonCombatant>(),
            "the observer is what says the crew is gone"
        );
        assert_eq!(
            **world.entity(ai_ship).get::<AIPointDefenseTarget>().unwrap(),
            None,
            "the wreck defends against nothing"
        );
        assert_eq!(
            **world.entity(turret).get::<AITurretDefenseTarget>().unwrap(),
            None,
            "and every mount lets go of what it was tracking"
        );
        assert!(
            !**world.entity(turret).get::<TurretSectionInput>().unwrap(),
            "a neutralized hull does not fire at a flying torpedo"
        );
        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetInput>()
                .unwrap(),
            None,
            "the guns slew back to rest instead of holding the wreck's last aim"
        );
        assert!(
            world.entities().contains(torpedo),
            "the torpedo flies on unopposed - that is the point"
        );
    }
}
