//! AI torpedo launches: the per-bay cooldown ([`AITorpedoBay`]), the launch
//! envelope (range band plus rough alignment), and the bay's target write.

use avian3d::prelude::*;
use bevy::prelude::*;

use super::{guns::ai_line_of_fire_blocked, maneuver::ai_target_anchor};
use crate::prelude::*;

// AI torpedo launch tuning. The bay's own fire-rate cooldown
// (TorpedoSectionSpawnerFireState) still applies underneath; these knobs
// shape WHEN the AI pulls the trigger at all.
/// Per-bay launch cadence (s): the AI takes deliberate, spaced torpedo
/// shots instead of holding the trigger and dumping one every
/// 1/fire_rate seconds. Playtest knob.
const AI_TORPEDO_COOLDOWN_SECS: f32 = 10.0;
/// Outer edge (m) of the launch envelope. Beyond detection range
/// (AI_ENGAGE_RANGE) but well inside acquisition range
/// (AI_TARGET_MAX_RANGE), so a launch can open the approach on a fight the
/// ship is already committing to. Playtest knob.
const AI_TORPEDO_MAX_RANGE: f32 = 1000.0;
/// Inner edge of the envelope, as a multiple of the bay's configured blast
/// radius: a point-blank launch detonates inside the shooter's own blast
/// (blast damage deliberately affects the). The factor keeps the detonation
/// point - the target - clear of the shooter, with margin for the closure
/// that happens while the torpedo flies. Playtest knob.
const AI_TORPEDO_MIN_RANGE_BLAST_FACTOR: f32 = 3.0;
/// Rough hull-alignment gate (cos) on a launch. Deliberately loose - PN
/// guidance does the turning - it exists so launches read as aimed attack
/// runs rather than popping off an orbit tangent pointed away, and so the
/// torpedo does not open its flight turning back through the shooter.
const AI_TORPEDO_ALIGNMENT_COS: f32 = 0.5;

/// Per-bay AI launch state: the launch cadence on top of the bay's own
/// fire-rate timer. Lazily inserted by `update_torpedo_section_input` on
/// torpedo sections whose ship is AI-controlled - an Add-observer would
/// race the root's `AISpaceshipMarker`, which lands after the child
/// sections spawn. Reset by `update_torpedo_target_input` when a launch
/// ACTUALLY happens (a projectile spawned), so a trigger pull a disabled
/// bay ignored never burns the cooldown.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AITorpedoBay {
    /// Time until this bay may launch again. A fresh [`Cooldown`] is READY, so
    /// the first launch of a fight comes as soon as the envelope opens.
    cooldown: Cooldown,
}

impl Default for AITorpedoBay {
    fn default() -> Self {
        Self {
            cooldown: Cooldown::new(AI_TORPEDO_COOLDOWN_SECS),
        }
    }
}

/// The geometric half of the launch decision: the target inside the range
/// band [blast_radius * [`AI_TORPEDO_MIN_RANGE_BLAST_FACTOR`],
/// [`AI_TORPEDO_MAX_RANGE`]] with the hull roughly on the bearing
/// ([`AI_TORPEDO_ALIGNMENT_COS`]). The per-ship gates (behavior state,
/// ship-kind target, bay cooldown) live in the calling system. Pure for
/// unit testing.
fn ai_torpedo_envelope(to_target: Vec3, forward: Vec3, blast_radius: f32) -> bool {
    let distance = to_target.length();
    if distance <= f32::EPSILON
        || distance < blast_radius * AI_TORPEDO_MIN_RANGE_BLAST_FACTOR
        || distance > AI_TORPEDO_MAX_RANGE
    {
        return false;
    }
    forward.dot(to_target / distance) > AI_TORPEDO_ALIGNMENT_COS
}

/// Pull each AI ship's torpedo triggers: write [`TorpedoSectionInput`] on
/// its bays when the launch envelope is open. The per-ship gates: an
/// Engage-like state - Evade excluded, a jinking hull is no launch
/// platform (Retreat inherits, per its stub) - and a SHIP target: hostile
/// torpedoes are the guns' job (point defense), not worth a bay's ordnance.
/// Per ship: the line of fire clear ([`ai_line_of_fire_blocked`]) - no
/// torpedo spent on the cover between the bay and the target. Per bay: the
/// launch cadence elapsed and the envelope ([`ai_torpedo_envelope`]) open
/// from the ship's anchor.
#[allow(clippy::type_complexity)]
pub(super) fn update_torpedo_section_input(
    time: Res<Time>,
    mut commands: Commands,
    q_missing: Query<(Entity, &ChildOf), (With<TorpedoSectionMarker>, Without<AITorpedoBay>)>,
    mut q_section: Query<
        (
            &mut TorpedoSectionInput,
            &mut AITorpedoBay,
            &TorpedoSectionConfigHelper,
            &ChildOf,
        ),
        With<TorpedoSectionMarker>,
    >,
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
    q_target: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
    q_ship_root: Query<(), With<SpaceshipRootMarker>>,
    spatial: SpatialQuery,
    q_sensor: Query<(), With<Sensor>>,
    q_collider_of: Query<&ColliderOf>,
) {
    // Arm AI bays with their launch state, lazily: at section-spawn time
    // "is this an AI ship" is not answerable yet (see [`AITorpedoBay`]),
    // so a bare bay picks its state up here, one frame before first use.
    for (section, ChildOf(parent)) in &q_missing {
        if q_spaceship.contains(*parent) {
            commands.entity(section).insert(AITorpedoBay::default());
        }
    }

    for (entity, transform, com, state, target) in &q_spaceship {
        let engaged = state.engages() && *state != AIBehaviorState::Evade;
        // The launch bearing runs anchor to anchor, like every AI vector.
        let own_anchor = live_structure_anchor(transform, com);
        let target_ship = (**target).filter(|&target| q_ship_root.contains(target));
        let target_anchor =
            target_ship.and_then(|target| ai_target_anchor(Some(target), &q_target));
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

        for (mut input, mut bay, config, _) in q_section
            .iter_mut()
            .filter(|(_, _, _, ChildOf(parent))| *parent == entity)
        {
            // The cadence elapses unconditionally - maneuvering outside
            // the envelope between launches is part of the cadence, not a
            // pause of it.
            bay.cooldown.tick(time.delta_secs());
            let launch = engaged
                && bay.cooldown.ready()
                && target_anchor.is_some_and(|anchor| {
                    ai_torpedo_envelope(
                        anchor - own_anchor,
                        *transform.forward(),
                        config.blast_radius,
                    )
                })
                && line_clear(own_anchor);
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
/// dumb-fire shot for life. Also resets the sourcing bay's launch cadence
/// (attributed through the projectile's [`TorpedoSectionPartOf`]): only an
/// actual launch burns the cooldown. Torpedoes owned by non-AI ships are
/// left to the player's commit system, and vice versa.
///
/// Only a SHIP target commits, matching the trigger side's gate: the
/// launch and the commit are one frame apart, and an [`AITarget`] that
/// flipped to a hostile torpedo in that frame (ship target died, ordnance
/// in range) must not send this torpedo chasing ordnance - it dumb-fires
/// instead.
pub(super) fn update_torpedo_target_input(
    mut commands: Commands,
    q_torpedo: Query<
        (Entity, &ProjectileOwner, &TorpedoSectionPartOf),
        (
            With<TorpedoProjectileMarker>,
            Without<TorpedoTargetEntity>,
            Without<TorpedoTargetChosen>,
        ),
    >,
    q_spaceship: Query<&AITarget, With<AISpaceshipMarker>>,
    q_ship_root: Query<(), With<SpaceshipRootMarker>>,
    mut q_bay: Query<&mut AITorpedoBay>,
) {
    for (torpedo, owner, part_of) in &q_torpedo {
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
        if let Ok(mut bay) = q_bay.get_mut(**part_of) {
            bay.cooldown.trigger();
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
        let blast_radius = 30.0; // min range = 30 * 3 = 90 m
        let forward = Vec3::NEG_Z;

        assert!(
            ai_torpedo_envelope(Vec3::NEG_Z * 300.0, forward, blast_radius),
            "in band, dead ahead: launch"
        );
        assert!(
            ai_torpedo_envelope(Vec3::new(100.0, 0.0, -300.0), forward, blast_radius),
            "in band, ~18 degrees off: rough alignment accepts it"
        );
        assert!(
            !ai_torpedo_envelope(Vec3::NEG_Z * 80.0, forward, blast_radius),
            "below the blast-derived minimum: a launch here self-hits"
        );
        assert!(
            !ai_torpedo_envelope(
                Vec3::NEG_Z * (AI_TORPEDO_MAX_RANGE + 1.0),
                forward,
                blast_radius
            ),
            "beyond the outer edge"
        );
        assert!(
            !ai_torpedo_envelope(Vec3::X * 300.0, forward, blast_radius),
            "perpendicular bearing: misaligned"
        );
        assert!(
            !ai_torpedo_envelope(Vec3::ZERO, forward, 0.0),
            "degenerate zero bearing"
        );
    }

    /// An AI ship (at the origin, facing -Z) engaged on a player ship, with
    /// one default-config torpedo bay. Returns (world, ship, target, bay).
    /// The bay's `AITorpedoBay` is NOT armed yet - run
    /// `update_torpedo_section_input` once (the lazy insert) before
    /// asserting on trigger state.
    fn torpedo_world(target_position: Vec3) -> (World, Entity, Entity, Entity) {
        let mut world = World::new();
        world.init_resource::<Time>();
        // Empty collider trees for the launch gate's SpatialQuery: no
        // colliders means a clear line of fire, which is this rig's intent.
        world.init_resource::<ColliderTrees>();
        let target = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(target_position),
            ))
            .id();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AITarget(Some(target)),
                Transform::default(),
            ))
            .id();
        let bay = world
            .spawn((
                torpedo_section(TorpedoSectionConfig::default()),
                ChildOf(ship),
            ))
            .id();
        (world, ship, target, bay)
    }

    /// Arm the bay (lazy-insert pass) and run the trigger write once.
    fn run_trigger(world: &mut World) {
        world.run_system_once(update_torpedo_section_input).unwrap();
        world.run_system_once(update_torpedo_section_input).unwrap();
    }

    #[test]
    fn an_engaged_ship_pulls_the_trigger_inside_the_envelope() {
        // Default blast radius 30 -> min range 90; 300 m dead ahead is in
        // band and aligned, the default state is Engage, the cadence
        // starts elapsed: everything open.
        let (mut world, _, _, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));

        world.run_system_once(update_torpedo_section_input).unwrap();
        assert!(
            world.entity(bay).get::<AITorpedoBay>().is_some(),
            "the bare bay picks its launch state up lazily"
        );
        assert!(
            !**world.entity(bay).get::<TorpedoSectionInput>().unwrap(),
            "the arming pass itself does not fire yet"
        );

        world.run_system_once(update_torpedo_section_input).unwrap();
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
    fn the_cadence_gates_the_next_launch() {
        let (mut world, _, _, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));
        run_trigger(&mut world);
        assert!(**world.entity(bay).get::<TorpedoSectionInput>().unwrap());

        // A launch happened: the commit system resets the bay's cadence.
        world
            .entity_mut(bay)
            .get_mut::<AITorpedoBay>()
            .unwrap()
            .cooldown
            .trigger();

        world.run_system_once(update_torpedo_section_input).unwrap();
        assert!(
            !**world.entity(bay).get::<TorpedoSectionInput>().unwrap(),
            "cadence running: trigger released despite the open envelope"
        );
    }

    #[test]
    fn a_fresh_ai_torpedo_commits_to_the_owner_target_and_burns_the_cadence() {
        let (mut world, ship, target, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));
        run_trigger(&mut world);
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
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
        assert!(
            !world
                .entity(bay)
                .get::<AITorpedoBay>()
                .unwrap()
                .cooldown
                .ready(),
            "an actual launch burns the bay's cadence"
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
                TorpedoTargetChosen,
                Transform::from_translation(Vec3::new(0.0, 0.0, -300.0)),
            ))
            .id();
        **world.entity_mut(ship).get_mut::<AITarget>().unwrap() = Some(hostile_torpedo);
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
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
                Transform::from_translation(Vec3::new(500.0, 0.0, 0.0)),
            ))
            .id();
        let bay_b = world
            .spawn((
                torpedo_section(TorpedoSectionConfig::default()),
                ChildOf(ship_b),
            ))
            .id();

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
