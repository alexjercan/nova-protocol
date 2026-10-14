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
            let reach =
                leader_arm.map_or(0.0, |arm| **arm) + arm.map_or(0.0, |arm| **arm) + AI_FOLLOW_GAP;
            let slot = leader_anchor + leader_transform.rotation * (follow.side * reach);
            let to_slot = slot - live_structure_anchor(transform, com);
            let distance = to_slot.length();
            let closing = if distance > AI_FOLLOW_SLACK {
                to_slot / distance
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
