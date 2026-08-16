//! Per-turret point defense: which inbound torpedo each individual mount is
//! defending against.
//!
//! [`AIPointDefenseTarget`] is per-SHIP, so every turret on a hull swings onto
//! the same torpedo. The obvious complaint about that is overkill, but the real
//! defect is IDLENESS: a turret handed a target under its own hull cannot
//! depress far enough to engage it, so it contributes nothing while a torpedo it
//! COULD have hit flies in unopposed.
//!
//! So the rule is not "spread the fire out", it is **never assign a turret a
//! target it cannot engage**. Splitting falls out of that for free, and a hull
//! whose mounts can all see one torpedo still puts all of them on it.
//!
//! DWELL is mandatory. Slew time is real: a mount that re-decides every tick
//! swings between targets and hits nothing. A turret holds its torpedo until it
//! dies, leaves its arc, or something far more urgent turns up.
//!
//! CONTROLLER-AGNOSTIC. This pass sat under `input/ai/` for historical reasons
//! only - nothing in it reads a behavior state, a threat memory or an AI
//! target. It assigns PLAYER mounts too; which of the player's mounts it may
//! touch is the one thing [`ownership`](super::ownership) decides.

use std::collections::{HashMap, HashSet};

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

#[cfg(test)]
use super::ownership::MountAuthority;
use super::ownership::PointDefenseMount;
use crate::{
    input::ai::{AIPointDefenseRange, AI_POINT_DEFENSE_RANGE},
    prelude::*,
};

/// The inbound torpedo THIS turret is defending against, or `None` when there
/// is nothing it can reach.
///
/// `None` is a real answer, not an absence: a turret carrying this component
/// and holding `None` has been told there is nothing in its arc worth swinging
/// onto, and goes back to the ship's primary target rather than falling back on
/// the ship-wide point-defense pick. That fallback IS the dogpile.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct TurretDefenseTarget(pub Option<Entity>);

/// How much more imminent a rival torpedo must be before it takes a mount off
/// the one it is already tracking: half the time to impact.
///
/// This is the dwell knob, and it is deliberately blunt. Anything close to 1.0
/// re-decides on noise, which costs a slew every time and lands nothing. At 0.5
/// a turret only breaks off for a threat arriving in half the time - a torpedo
/// about to connect while the one it is tracking is still crossing the
/// envelope.
const AI_PD_URGENCY_FACTOR: f32 = 0.5;

/// Radians shaved off both ends of a turret's arc when ACQUIRING (never when
/// holding): about 3 degrees. A torpedo sitting exactly on the depression floor
/// would otherwise be picked up and dropped on alternate frames as it drifts
/// across the limit, which is the same swinging-and-hitting-nothing that dwell
/// exists to prevent.
const AI_PD_ARC_MARGIN: f32 = 0.05;

/// One torpedo, as a threat to one particular hull.
struct PointDefenseThreat {
    /// The torpedo.
    entity: Entity,
    /// Where it is now - what the turret has to bear on.
    position: Vec3,
    /// Seconds until it arrives, closing speed permitting. Lower is more
    /// urgent; a torpedo not closing on this hull is `f32::MAX`, so it sorts
    /// last without needing a separate tier.
    time_to_impact: f32,
}

/// Give every turret on a PILOTED ship - AI or player - a defense slot to be
/// assigned.
///
/// A separate system rather than a `#[require]` on the turret, because the
/// turret section is a ship-layer type that must not know who flies it, and
/// rather than an observer, because a turret's ship is only knowable once it is
/// parented.
///
/// An UNPILOTED hull (a bare example rig, a drifting prop) gets no slot: the
/// slot is what makes a mount answerable to a computer, and those hulls have
/// nobody aboard.
#[expect(
    clippy::type_complexity,
    reason = "the ship filter names both piloted markers"
)]
pub(super) fn insert_turret_defense_target(
    mut commands: Commands,
    q_turret: Query<(Entity, &ChildOf), (With<TurretSectionMarker>, Without<TurretDefenseTarget>)>,
    q_ship: Query<
        (),
        (
            With<SpaceshipRootMarker>,
            Or<(With<AISpaceshipMarker>, With<PlayerSpaceshipMarker>)>,
        ),
    >,
) {
    for (turret, ChildOf(ship)) in &q_turret {
        if q_ship.contains(*ship) {
            commands
                .entity(turret)
                .insert(TurretDefenseTarget::default());
        }
    }
}

/// Whether `turret` can swing its muzzle onto `position`.
///
/// A turret whose joint tree the arc solver did not recognise carries no
/// [`TurretSectionArc`] and bears anywhere - fail-open, so an exotic mod mount
/// keeps defending its ship instead of silently standing down.
fn bears_on(
    turret: &GlobalTransform,
    arc: Option<&TurretSectionArc>,
    position: Vec3,
    margin: f32,
) -> bool {
    let Some(arc) = arc else {
        return true;
    };
    let (_, rotation, mount) = turret.to_scale_rotation_translation();
    arc.bears_on(rotation, position - mount, margin)
}

/// Whether the computer may put a target on this mount.
///
/// A mount with NO ownership slot is an AI ship's, and the AI holds its whole
/// battery all the time. A mount that HAS one is on the player's hull, where
/// the computer is only ever the third tier of
/// [`MountAuthority`](super::ownership::MountAuthority).
fn computer_may_assign(mount: Option<&PointDefenseMount>) -> bool {
    mount.is_none_or(PointDefenseMount::computer_owns)
}

/// Assign each PD-capable turret on a piloted ship its own inbound torpedo.
///
/// An [`AINonCombatant`] hull builds no threat list, so both passes below hand
/// its mounts `None` and a wreck neutralized mid-intercept lets go of the
/// torpedo it was tracking. Clearing beats skipping: a mount left on a stale
/// assignment would keep firing, which is the whole defect.
///
/// A mount carrying a [`PointDefenseMount`] (every turret on the PLAYER's hull)
/// is only the computer's while that slot says so. One condition, applied at
/// both passes: a mount the player holds is cleared and left alone, so taking a
/// mount back also takes back its claim on the torpedo, freeing it for a mount
/// that is still the computer's.
#[expect(
    clippy::type_complexity,
    reason = "one query term per assignment input"
)]
pub(crate) fn update_turret_point_defense(
    q_torpedoes: Query<
        (
            Entity,
            &Transform,
            Option<&LinearVelocity>,
            Option<&Allegiance>,
            Option<&TorpedoTargetEntity>,
        ),
        (With<TorpedoProjectileMarker>, With<TorpedoTargetChosen>),
    >,
    q_ship: Query<
        (
            Entity,
            &Transform,
            Option<&ComputedCenterOfMass>,
            &Allegiance,
            Option<&AIPointDefenseRange>,
        ),
        (
            With<SpaceshipRootMarker>,
            Or<(With<AISpaceshipMarker>, With<PlayerSpaceshipMarker>)>,
            Without<AINonCombatant>,
        ),
    >,
    mut q_turret: Query<
        (
            Entity,
            &GlobalTransform,
            Option<&TurretSectionArc>,
            &ChildOf,
            Option<&PointDefenseMount>,
            &mut TurretDefenseTarget,
        ),
        With<TurretSectionMarker>,
    >,
) {
    // Threats are per-HULL, not global: the same torpedo is seconds from one
    // ship and merely nearby to another, and imminence is what the assignment
    // sorts on.
    let mut threats: HashMap<Entity, Vec<PointDefenseThreat>> = HashMap::new();
    for (ship, transform, com, own_allegiance, pd_range) in &q_ship {
        let anchor = live_structure_anchor(transform, com);
        let range = pd_range.map_or(AI_POINT_DEFENSE_RANGE, |range| range.0);
        let mut ship_threats: Vec<PointDefenseThreat> = q_torpedoes
            .iter()
            .filter_map(
                |(torpedo, t_transform, velocity, allegiance, torpedo_target)| {
                    if relation(Some(own_allegiance), allegiance) != Relation::Hostile {
                        return None;
                    }
                    let position = t_transform.translation;
                    let separation = anchor - position;
                    let distance = separation.length();
                    if distance > range || distance <= f32::EPSILON {
                        return None;
                    }
                    // Time to impact off the CLOSING component, so a torpedo
                    // crossing the hull's bow is correctly less urgent than one
                    // driving straight at it even from the same distance. A
                    // torpedo hunting this ship by name and not yet closing (it
                    // is still turning onto course) still counts as arriving.
                    let closing = velocity
                        .map(|velocity| velocity.dot(separation / distance))
                        .unwrap_or(0.0);
                    let time_to_impact = if closing > f32::EPSILON {
                        distance / closing
                    } else if torpedo_target.map(|target| **target) == Some(ship) {
                        distance
                    } else {
                        f32::MAX
                    };
                    Some(PointDefenseThreat {
                        entity: torpedo,
                        position,
                        time_to_impact,
                    })
                },
            )
            .collect();
        ship_threats.sort_by(|a, b| a.time_to_impact.total_cmp(&b.time_to_impact));
        if !ship_threats.is_empty() {
            threats.insert(ship, ship_threats);
        }
    }

    // Torpedoes some mount is already on. Claims are what split the fire: a
    // free mount prefers a threat nobody has, and only doubles up once every
    // threat it can reach is already covered.
    let mut claimed: HashSet<Entity> = HashSet::new();

    // Pass one: DWELL. Every turret that can still work its current target
    // keeps it, and claims it, before any turret is allowed to choose.
    for (_, transform, arc, ChildOf(ship), mount, mut assignment) in &mut q_turret {
        if !computer_may_assign(mount) {
            assignment.set_if_neq(TurretDefenseTarget(None));
            continue;
        }
        let Some(current) = **assignment else {
            continue;
        };
        let hold = threats
            .get(ship)
            .and_then(|threats| threats.iter().find(|threat| threat.entity == current))
            // Dead, out of range, or no longer hostile: the pick simply is not
            // a threat any more.
            .filter(|threat| bears_on(transform, arc, threat.position, 0.0))
            .is_some_and(|held| {
                // ...unless something far more urgent has turned up that this
                // mount can also reach.
                !threats[ship].iter().any(|rival| {
                    rival.entity != current
                        && rival.time_to_impact < held.time_to_impact * AI_PD_URGENCY_FACTOR
                        && bears_on(transform, arc, rival.position, AI_PD_ARC_MARGIN)
                })
            });
        if hold {
            claimed.insert(current);
        } else {
            assignment.set_if_neq(TurretDefenseTarget(None));
        }
    }

    // Pass two: fill the idle mounts, most imminent reachable threat first,
    // preferring one nobody has claimed.
    for (turret, transform, arc, ChildOf(ship), mount, mut assignment) in &mut q_turret {
        if assignment.is_some() || !computer_may_assign(mount) {
            continue;
        }
        let pick = threats.get(ship).and_then(|threats| {
            threats
                .iter()
                .filter(|threat| bears_on(transform, arc, threat.position, AI_PD_ARC_MARGIN))
                // `threats` is already sorted by imminence, so the first
                // unclaimed one wins outright and `min_by_key` only has to
                // break the claimed/unclaimed tier.
                .min_by_key(|threat| claimed.contains(&threat.entity))
                .map(|threat| threat.entity)
        });
        if let Some(pick) = pick {
            claimed.insert(pick);
        }
        if **assignment != pick {
            // Only on a real change, so the log reads as the assignment
            // DECISIONS a fight made rather than a per-frame dump.
            debug!("update_turret_point_defense: mount {turret:?} -> {pick:?}");
        }
        assignment.set_if_neq(TurretDefenseTarget(pick));
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// An AI ship at the origin with no roll, plus `count` turrets mounted the
    /// same way up at the origin. Returns (world, ship, turrets).
    fn defended_ship(world: &mut World, count: usize) -> (Entity, Vec<Entity>) {
        let ship = world
            .spawn((
                AISpaceshipMarker,
                SpaceshipRootMarker,
                Allegiance::Enemy,
                Transform::default(),
                AIPointDefenseTarget::default(),
            ))
            .id();
        let arc = TurretSectionArc::from_tree(&TurretSectionConfig::default().root)
            .expect("the shipped tree has an arc");
        let turrets = (0..count)
            .map(|_| {
                world
                    .spawn((
                        TurretSectionMarker,
                        arc,
                        GlobalTransform::IDENTITY,
                        ChildOf(ship),
                        TurretDefenseTarget::default(),
                    ))
                    .id()
            })
            .collect();
        (ship, turrets)
    }

    /// A committed hostile torpedo at `position`, closing on `ship` at 30 u/s.
    fn inbound(world: &mut World, ship: Entity, position: Vec3) -> Entity {
        let closing = (Vec3::ZERO - position).normalize_or_zero() * 30.0;
        world
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetChosen,
                TorpedoTargetEntity(ship),
                Allegiance::Player,
                Transform::from_translation(position),
                LinearVelocity(closing),
            ))
            .id()
    }

    fn assignment(world: &World, turret: Entity) -> Option<Entity> {
        **world.get::<TurretDefenseTarget>(turret).unwrap()
    }

    #[test]
    fn two_turrets_take_two_torpedoes_instead_of_dogpiling_one() {
        // The Sins-of-a-Solar-Empire-II bug, and the reason the assignment is
        // per-turret at all: a ship-wide pick puts every mount on the nearest
        // threat and the second torpedo flies in unopposed.
        let mut world = World::new();
        let (ship, turrets) = defended_ship(&mut world, 2);
        let near = inbound(&mut world, ship, Vec3::new(0.0, 10.0, -60.0));
        let far = inbound(&mut world, ship, Vec3::new(0.0, 10.0, -120.0));

        world.run_system_once(update_turret_point_defense).unwrap();

        let picks: Vec<Option<Entity>> = turrets.iter().map(|&t| assignment(&world, t)).collect();
        assert!(
            picks.contains(&Some(near)) && picks.contains(&Some(far)),
            "both inbound torpedoes must be engaged, got {picks:?}"
        );
    }

    #[test]
    fn a_third_turret_doubles_up_once_every_threat_is_covered() {
        // Splitting is a consequence of reachability, not a rule of its own: a
        // spare mount is not left idle for the sake of spreading fire.
        let mut world = World::new();
        let (ship, turrets) = defended_ship(&mut world, 3);
        inbound(&mut world, ship, Vec3::new(0.0, 10.0, -60.0));
        inbound(&mut world, ship, Vec3::new(0.0, 10.0, -120.0));

        world.run_system_once(update_turret_point_defense).unwrap();

        assert!(
            turrets
                .iter()
                .all(|&turret| assignment(&world, turret).is_some()),
            "no mount stands idle while there is something it can shoot"
        );
    }

    #[test]
    fn a_turret_takes_the_threat_it_can_reach_instead_of_sitting_idle() {
        // The owner's actual complaint: "some PDCs cannot even reach the target
        // all the time, so its bad, but it can reach other targets, so it can
        // split". The nearest torpedo is UNDER the hull, past the depression
        // floor of an upright mount; the second is above it.
        let mut world = World::new();
        let (ship, turrets) = defended_ship(&mut world, 1);
        let under = inbound(&mut world, ship, Vec3::new(0.0, -60.0, -30.0));
        let above = inbound(&mut world, ship, Vec3::new(0.0, 40.0, -100.0));

        world.run_system_once(update_turret_point_defense).unwrap();

        assert_eq!(
            assignment(&world, turrets[0]),
            Some(above),
            "an upright mount must take the torpedo it can bear on, not the \
             nearer one under its own hull"
        );
        let _ = under;
    }

    #[test]
    fn a_turret_with_nothing_in_its_arc_is_assigned_nothing() {
        // And says so, rather than leaving a stale pick behind: a `None`
        // assignment is what sends the mount back to the ship's primary target.
        let mut world = World::new();
        let (ship, turrets) = defended_ship(&mut world, 1);
        inbound(&mut world, ship, Vec3::new(0.0, -60.0, -30.0));

        world.run_system_once(update_turret_point_defense).unwrap();

        assert_eq!(assignment(&world, turrets[0]), None);
    }

    #[test]
    fn a_mount_holds_its_target_against_a_comparable_rival() {
        // DWELL. Slew time is real, so a rival that is merely a bit closer must
        // not take the mount off the shot it is already lined up on.
        let mut world = World::new();
        let (ship, turrets) = defended_ship(&mut world, 1);
        let held = inbound(&mut world, ship, Vec3::new(0.0, 10.0, -100.0));
        world
            .entity_mut(turrets[0])
            .insert(TurretDefenseTarget(Some(held)));
        // 25% nearer, so 25% sooner: inside the urgency factor.
        inbound(&mut world, ship, Vec3::new(0.0, 10.0, -75.0));

        world.run_system_once(update_turret_point_defense).unwrap();

        assert_eq!(
            assignment(&world, turrets[0]),
            Some(held),
            "a comparable rival must not steal a mount mid-slew"
        );
    }

    #[test]
    fn a_far_more_urgent_threat_does_take_the_mount() {
        // The other half of dwell: holding forever is its own failure. A
        // torpedo arriving in a third of the time is worth the slew.
        let mut world = World::new();
        let (ship, turrets) = defended_ship(&mut world, 1);
        let held = inbound(&mut world, ship, Vec3::new(0.0, 10.0, -120.0));
        world
            .entity_mut(turrets[0])
            .insert(TurretDefenseTarget(Some(held)));
        let urgent = inbound(&mut world, ship, Vec3::new(0.0, 5.0, -25.0));

        world.run_system_once(update_turret_point_defense).unwrap();

        assert_eq!(assignment(&world, turrets[0]), Some(urgent));
    }

    #[test]
    fn a_target_that_leaves_the_arc_is_dropped_for_one_the_mount_can_reach() {
        // Dwell holds a target until it dies, leaves the arc, or is outranked.
        // This is the middle case: the ship rolls, and the tracked torpedo ends
        // up under the hull.
        let mut world = World::new();
        let (ship, turrets) = defended_ship(&mut world, 1);
        let tracked = inbound(&mut world, ship, Vec3::new(0.0, -60.0, -30.0));
        let reachable = inbound(&mut world, ship, Vec3::new(0.0, 40.0, -100.0));
        world
            .entity_mut(turrets[0])
            .insert(TurretDefenseTarget(Some(tracked)));

        world.run_system_once(update_turret_point_defense).unwrap();

        assert_eq!(
            assignment(&world, turrets[0]),
            Some(reachable),
            "a mount whose target left its arc must re-engage, not hold a shot \
             it cannot take"
        );
    }

    #[test]
    fn a_dead_or_out_of_range_target_clears() {
        let mut world = World::new();
        let (ship, turrets) = defended_ship(&mut world, 1);
        let torpedo = inbound(&mut world, ship, Vec3::new(0.0, 10.0, -60.0));
        world
            .entity_mut(turrets[0])
            .insert(TurretDefenseTarget(Some(torpedo)));

        // Shot down mid-flight.
        world.despawn(torpedo);
        world.run_system_once(update_turret_point_defense).unwrap();
        assert_eq!(assignment(&world, turrets[0]), None);

        // And a torpedo beyond the point-defense envelope is not a threat.
        let distant = inbound(
            &mut world,
            ship,
            Vec3::new(0.0, 10.0, -(AI_POINT_DEFENSE_RANGE * 2.0)),
        );
        world.run_system_once(update_turret_point_defense).unwrap();
        assert_eq!(assignment(&world, turrets[0]), None);
        let _ = distant;
    }

    #[test]
    fn a_turret_holding_a_target_does_not_oscillate_across_frames() {
        // The failure dwell exists to prevent, watched over TIME rather than
        // argued about: two comparable torpedoes closing together must not make
        // a mount alternate between them.
        let mut world = World::new();
        let (ship, turrets) = defended_ship(&mut world, 1);
        let a = inbound(&mut world, ship, Vec3::new(0.0, 10.0, -100.0));
        let b = inbound(&mut world, ship, Vec3::new(0.0, 10.0, -98.0));

        let mut picks = Vec::new();
        for step in 0..40 {
            // Both close on the ship; b crosses ahead of a partway through, so
            // a naive nearest-wins pick flips here.
            for (torpedo, start) in [(a, 100.0f32), (b, 98.0)] {
                let range = start - step as f32 * 2.0;
                world.get_mut::<Transform>(torpedo).unwrap().translation =
                    Vec3::new(0.0, 10.0, -range.max(20.0));
            }
            world.run_system_once(update_turret_point_defense).unwrap();
            picks.push(assignment(&world, turrets[0]));
        }

        let switches = picks.windows(2).filter(|pair| pair[0] != pair[1]).count();
        assert!(
            switches <= 1,
            "a mount must not swing between comparable targets: {switches} \
             switches across {} frames",
            picks.len()
        );
        assert!(picks.last().unwrap().is_some(), "and it must be engaged");
    }

    #[test]
    fn an_unrecognised_mount_still_defends_its_ship() {
        // Fail-open, end to end: a turret carrying no arc (a mod tree the
        // solver does not cover) is assigned the threat anyway.
        let mut world = World::new();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                SpaceshipRootMarker,
                Allegiance::Enemy,
                Transform::default(),
                AIPointDefenseTarget::default(),
            ))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                GlobalTransform::IDENTITY,
                ChildOf(ship),
                TurretDefenseTarget::default(),
            ))
            .id();
        // Under the hull, where an arc-carrying mount would refuse it.
        let under = inbound(&mut world, ship, Vec3::new(0.0, -60.0, -30.0));

        world.run_system_once(update_turret_point_defense).unwrap();

        assert_eq!(assignment(&world, turret), Some(under));
    }

    #[test]
    fn a_non_combatant_hull_drops_every_mount_it_held() {
        // A neutralized wreck is the live case (the stand-down observer
        // inserts the flag), and an unarmed hauler is the other. Clearing,
        // not skipping: a mount left on a stale assignment keeps firing.
        let mut world = World::new();
        let (ship, turrets) = defended_ship(&mut world, 2);
        let held = inbound(&mut world, ship, Vec3::new(0.0, 10.0, -60.0));
        for &turret in &turrets {
            world
                .entity_mut(turret)
                .insert(TurretDefenseTarget(Some(held)));
        }

        world.entity_mut(ship).insert(AINonCombatant);
        world.run_system_once(update_turret_point_defense).unwrap();

        assert!(
            turrets
                .iter()
                .all(|&turret| assignment(&world, turret).is_none()),
            "a hull that cannot fight defends against nothing"
        );
    }

    #[test]
    fn every_piloted_hull_gets_a_defense_slot_and_a_drifting_one_does_not() {
        // The player half is the change: the slot used to be AI-only, which is
        // what made point defence a behaviour of the AI CONTROLLER instead of
        // a capability of a flight computer.
        let mut world = World::new();
        let ai = world
            .spawn((AISpaceshipMarker, SpaceshipRootMarker, Transform::default()))
            .id();
        let player = world
            .spawn((
                PlayerSpaceshipMarker,
                SpaceshipRootMarker,
                Transform::default(),
            ))
            .id();
        let drifting = world
            .spawn((SpaceshipRootMarker, Transform::default()))
            .id();
        let ai_turret = world.spawn((TurretSectionMarker, ChildOf(ai))).id();
        let player_turret = world.spawn((TurretSectionMarker, ChildOf(player))).id();
        let drifting_turret = world.spawn((TurretSectionMarker, ChildOf(drifting))).id();

        world.run_system_once(insert_turret_defense_target).unwrap();

        assert!(world.get::<TurretDefenseTarget>(ai_turret).is_some());
        assert!(
            world.get::<TurretDefenseTarget>(player_turret).is_some(),
            "the player's mounts answer to the same allocator now"
        );
        assert!(
            world.get::<TurretDefenseTarget>(drifting_turret).is_none(),
            "an unpiloted hull has nobody aboard to work its guns"
        );
    }

    #[test]
    fn a_mount_the_player_holds_is_cleared_and_left_alone() {
        // The precedence, seen from the allocator's side: the ownership slot
        // is the ONE thing that keeps the computer off a mount, and taking a
        // mount back must also take back its claim - not leave a stale pick
        // that the mount would resume the instant the grace expired.
        let mut world = World::new();
        let (ship, turrets) = defended_ship(&mut world, 1);
        world
            .entity_mut(ship)
            .remove::<AISpaceshipMarker>()
            .insert(PlayerSpaceshipMarker);
        let inbound = inbound(&mut world, ship, Vec3::new(0.0, 10.0, -60.0));

        // Idle: the computer owns the mount and takes the shot.
        world.entity_mut(turrets[0]).insert(PointDefenseMount {
            authority: MountAuthority::FlightComputer,
            ..default()
        });
        world.run_system_once(update_turret_point_defense).unwrap();
        assert_eq!(assignment(&world, turrets[0]), Some(inbound));

        // The player locks: the claim goes the same pass.
        world.entity_mut(turrets[0]).insert(PointDefenseMount {
            authority: MountAuthority::PlayerLock,
            ..default()
        });
        world.run_system_once(update_turret_point_defense).unwrap();
        assert_eq!(assignment(&world, turrets[0]), None);

        // And a mount the verb was taken away from is no different: only the
        // computer's own tier is a claim.
        world.entity_mut(turrets[0]).insert(PointDefenseMount {
            authority: MountAuthority::Cold,
            ..default()
        });
        world.run_system_once(update_turret_point_defense).unwrap();
        assert_eq!(assignment(&world, turrets[0]), None);
    }
}
