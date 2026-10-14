//! Escort flight: an AI ship that holds a slot beside a leader while nothing
//! hostile is close enough to fight. The player's hired escorts are the one
//! user ([`AIFollow`] is inserted by nova_scenario's fleet hire and thaw).
//!
//! The order owns the passive helm: `update_passive_flight` skips a follower,
//! and `update_combat_flight` leaves a held velocity to this driver when the
//! fight ends, so the two never hand the maneuver back and forth.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::maneuver::ai_radial_speed;
use crate::prelude::*;

/// Gap (world units, 200 m) between the leader's hull and the follower's hull
/// at the slot. Wide enough that a leader's turn does not sweep its arm
/// through the escort.
pub const AI_FOLLOW_GAP: f32 = 20.0;

/// Distance (world units, 30 m) inside which the follower counts as on its
/// slot and only matches the leader's velocity. Without it the closing speed
/// hunts back and forth across the slot.
const AI_FOLLOW_SLACK: f32 = 3.0;

/// Where a follower at `from` steers to reach `slot` without flying through
/// the leader whose anchor is `leader`.
///
/// The slot itself while the straight line to it keeps `clearance` from the
/// leader. Otherwise a point turned from the follower's side toward the
/// slot's side by half the angle between them, at most 60 degrees, at twice
/// the follower's distance (at least `clearance`); when the two sides are
/// opposite it turns over the leader's `up`. At 60 degrees and twice the
/// distance the line to that point keeps the follower's own distance, so no
/// leg comes nearer the leader than the follower already is. Recomputed every
/// tick, the point walks the follower outward and round the leader. Without
/// it, a slot on the far side (an escort spawned away from a station on the
/// side the leader docked by, or a leader that turns about) sends the
/// follower straight through the leader's hull.
pub(super) fn follow_waypoint(
    from: Vec3,
    slot: Vec3,
    leader: Vec3,
    clearance: f32,
    up: Vec3,
) -> Vec3 {
    let path = slot - from;
    let along = if path.length_squared() > 0.0 {
        ((leader - from).dot(path) / path.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    if (from + path * along).distance(leader) >= clearance {
        return slot;
    }
    let own_side = (from - leader).normalize_or(up);
    let slot_side = (slot - leader).normalize_or(up);
    let axis = own_side
        .cross(slot_side)
        .try_normalize()
        .or_else(|| {
            up.reject_from(own_side)
                .try_normalize()
                .and_then(|over| own_side.cross(over).try_normalize())
        })
        .unwrap_or_else(|| own_side.any_orthonormal_vector());
    let turn = (own_side.angle_between(slot_side) * 0.5).min(std::f32::consts::FRAC_PI_3);
    let radius = 2.0 * from.distance(leader).max(clearance);
    leader + Quat::from_axis_angle(axis, turn) * own_side * radius
}

/// A standing follow order: hold a slot beside `leader` on `side`.
///
/// The slot is `side` (a unit vector in the leader's local frame; +X is
/// starboard) times both hull arms plus [`AI_FOLLOW_GAP`], measured from the
/// leader's live structure anchor. The follower matches the leader's velocity
/// and facing there, and its [`AILeash`] centre rides the leader, so a fight
/// breaks off when it strays from the leader rather than from a spawn point.
///
/// While the leader is docked the follower holds still beside it. A leader
/// that no longer exists ends the order.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct AIFollow {
    /// The ship root the follower holds station on.
    pub leader: Entity,
    /// The slot direction in the leader's local frame, unit length.
    pub side: Vec3,
}

/// Flies every [`AIFollow`] ship that is not fighting to its slot.
///
/// Runs after `update_combat_flight` in the AI chain, so a ship that broke off
/// this frame flies to its slot this frame.
pub(super) fn update_follow_flight(
    mut commands: Commands,
    settings: Res<FlightSettings>,
    mut q_follower: Query<
        (
            Entity,
            &AIFollow,
            &Transform,
            Option<&ComputedCenterOfMass>,
            &AIBehaviorState,
            Option<&HullRadius>,
            Option<&FlightAuthority>,
            Option<&mut Autopilot>,
            Option<&mut AILeash>,
            Has<DockedShip>,
        ),
        (
            With<SpaceshipRootMarker>,
            With<AISpaceshipMarker>,
            Without<ShipOrderHelmAuthority>,
        ),
    >,
    q_leader: Query<
        (
            &Transform,
            Option<&ComputedCenterOfMass>,
            Option<&LinearVelocity>,
            Option<&HullRadius>,
            Has<DockedShip>,
        ),
        Without<AIFollow>,
    >,
    // As `update_combat_flight` asks: a hull with no live thruster would see
    // the autopilot disengage and this driver re-engage every frame.
    q_engine: Query<&ChildOf, (With<ThrusterSectionMarker>, Without<SectionInactiveMarker>)>,
) {
    for (ship, follow, transform, com, state, arm, authority, autopilot, leash, docked) in
        &mut q_follower
    {
        let Ok((leader_transform, leader_com, leader_velocity, leader_arm, leader_docked)) =
            q_leader.get(follow.leader)
        else {
            commands.entity(ship).remove::<AIFollow>();
            continue;
        };
        let leader_anchor = live_structure_anchor(leader_transform, leader_com);
        if let Some(mut leash) = leash {
            leash.center = leader_anchor;
        }
        if docked || state.engages() || !q_engine.iter().any(|&ChildOf(parent)| parent == ship) {
            continue;
        }
        let leader_velocity = leader_velocity.map_or(Vec3::ZERO, |velocity| **velocity);
        let action = if leader_docked {
            AutopilotAction::MatchVelocity {
                velocity: leader_velocity,
                facing: None,
            }
        } else {
            let arms = leader_arm.map_or(0.0, |arm| **arm) + arm.map_or(0.0, |arm| **arm);
            let slot =
                leader_anchor + leader_transform.rotation * (follow.side * (arms + AI_FOLLOW_GAP));
            let from = live_structure_anchor(transform, com);
            let waypoint = follow_waypoint(
                from,
                slot,
                leader_anchor,
                arms + AI_FOLLOW_GAP * 0.5,
                leader_transform.rotation * Vec3::Y,
            );
            let to_waypoint = waypoint - from;
            // The way left, round the leader when it detours, so the closing
            // speed brakes for the slot and not for the moving waypoint.
            let distance = to_waypoint.length() + waypoint.distance(slot);
            let closing = if distance > AI_FOLLOW_SLACK {
                to_waypoint.normalize_or_zero()
                    * ai_radial_speed(
                        distance - AI_FOLLOW_SLACK,
                        authority.copied().unwrap_or_default(),
                        &settings,
                    )
            } else {
                Vec3::ZERO
            };
            AutopilotAction::MatchVelocity {
                velocity: leader_velocity + closing,
                facing: Dir3::new(leader_transform.rotation * Vec3::NEG_Z).ok(),
            }
        };
        match autopilot {
            // Retarget a held velocity in place: engaging afresh every tick
            // resets the phase the computer reports.
            Some(mut autopilot)
                if matches!(autopilot.action, AutopilotAction::MatchVelocity { .. }) =>
            {
                autopilot.action = action;
            }
            _ => {
                commands.entity(ship).insert(Autopilot::engage(action));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A follower whose slot lies across the leader goes round it, over the
    /// leader's up when the slot is straight across, and every leg keeps the
    /// clearance; a follower with a clear line flies straight to its slot.
    #[test]
    fn a_follower_goes_round_the_leader_to_a_slot_across_it_and_straight_to_a_clear_one() {
        let (leader, clearance, up) = (Vec3::ZERO, 10.0, Vec3::Y);
        let distance_to_leader = |from: Vec3, to: Vec3| {
            let path = to - from;
            let along = ((leader - from).dot(path) / path.length_squared()).clamp(0.0, 1.0);
            (from + path * along).distance(leader)
        };

        let clear = (Vec3::new(30.0, 0.0, 30.0), Vec3::new(30.0, 0.0, -30.0));
        assert_eq!(
            follow_waypoint(clear.0, clear.1, leader, clearance, up),
            clear.1
        );

        let (from, slot) = (Vec3::new(-14.0, 0.0, 0.0), Vec3::new(14.0, 0.0, 0.0));
        let waypoint = follow_waypoint(from, slot, leader, clearance, up);
        assert!(
            waypoint.y > 0.0,
            "straight across goes over the top: {waypoint}"
        );
        assert!(distance_to_leader(from, waypoint) >= clearance - 1e-3);

        // Walk the carrot: each step moves to the waypoint, and every leg of
        // the way keeps the clearance until the line to the slot is clear.
        let mut at = from;
        for _ in 0..8 {
            let next = follow_waypoint(at, slot, leader, clearance, up);
            assert!(
                distance_to_leader(at, next) >= clearance - 1e-3,
                "the leg from {at} to {next} cuts the leader"
            );
            if next == slot {
                return;
            }
            at = next;
        }
        panic!("the follower never reached a clear line to its slot, last at {at}");
    }
}
