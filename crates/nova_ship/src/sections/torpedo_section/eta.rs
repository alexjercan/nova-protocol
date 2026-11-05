//! The player's torpedo time to impact: how long until the soonest of the
//! player's torpedoes reaches the hull the player has locked.
//!
//! [`PlayerTorpedoEta`] is the one shared source. The cockpit closing tone
//! reads it, and the lock readout's time-to-impact line will read the same
//! number, so the tone and the readout can never disagree.

use super::*;

/// Seconds until a torpedo at `p` moving at `v` reaches a target at
/// `target_p` moving at `target_v`: the range over the closing speed along the
/// line of sight. `None` when the two are not closing.
///
/// A straight-line estimate, not a guidance solution: the torpedo turns and
/// the target maneuvers, so the number is refreshed every frame rather than
/// predicted once.
pub fn torpedo_time_to_impact(p: Vec3, v: Vec3, target_p: Vec3, target_v: Vec3) -> Option<f32> {
    let line = target_p - p;
    let range = line.length();
    if range <= f32::EPSILON {
        return Some(0.0);
    }
    let closing = (v - target_v).dot(line / range);
    (closing > 0.0).then(|| range / closing)
}

/// One torpedo's time to impact.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct TorpedoEta {
    /// The torpedo.
    pub torpedo: Entity,
    /// Seconds until it reaches its target, by [`torpedo_time_to_impact`].
    pub seconds: f32,
}

/// The soonest time to impact among the player's torpedoes that are homing on
/// the hull the player has in its [`CombatLock`]. `None` when there is no
/// lock, or no torpedo of the player's is closing on it.
///
/// The SHARED source for every cue about the player's own torpedo arriving:
/// the cockpit closing tone and the lock readout both read this, and neither
/// derives its own. Written by [`update_player_torpedo_eta`] in the torpedo
/// Update chain, inside `SpaceshipSectionSystems`, so a reader ordered after
/// that set sees this frame's value.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct PlayerTorpedoEta(pub Option<TorpedoEta>);

/// The physical body `entity` belongs to: the nearest `RigidBody` at or above
/// it.
fn body_of(
    entity: Entity,
    q_body: &Query<(), With<RigidBody>>,
    q_parent: &Query<&ChildOf>,
) -> Option<Entity> {
    let mut current = entity;
    loop {
        if q_body.contains(current) {
            return Some(current);
        }
        current = q_parent.get(current).ok()?.parent();
    }
}

/// Recompute [`PlayerTorpedoEta`] from the live torpedoes.
///
/// A torpedo counts when the player's root owns it ([`ProjectileOwner`]) and
/// its [`TorpedoTargetEntity`] resolves to the same BODY as the player's
/// [`CombatLock`]. Bodies and not entities: a torpedo committed to a hull
/// still counts when the lock names one of its sections, and the reverse.
/// A torpedo still coasting cold or already shot down does not count: the
/// first has no drive lit to arrive with, the second is not arriving.
///
/// The torpedo's position is its root's [`Transform`] and the target point is
/// its [`TorpedoTargetPosition`], the live anchor guidance flies to, so the
/// number measures what guidance is closing on.
pub(super) fn update_player_torpedo_eta(
    mut eta: ResMut<PlayerTorpedoEta>,
    q_player: Query<
        (Entity, &CombatLock),
        (With<PlayerSpaceshipMarker>, With<SpaceshipRootMarker>),
    >,
    q_torpedo: Query<
        (
            Entity,
            &Transform,
            &LinearVelocity,
            &ProjectileOwner,
            &TorpedoTargetEntity,
            &TorpedoTargetPosition,
        ),
        (
            With<TorpedoProjectileMarker>,
            Without<TorpedoColdLaunch>,
            Without<TorpedoShotDownMarker>,
        ),
    >,
    q_velocity: Query<&LinearVelocity>,
    q_body: Query<(), With<RigidBody>>,
    q_parent: Query<&ChildOf>,
) {
    let soonest = q_player.iter().next().and_then(|(player, lock)| {
        let locked = body_of(lock.0?, &q_body, &q_parent)?;
        q_torpedo
            .iter()
            .filter(|(.., owner, target, _)| {
                owner.0 == player && body_of(target.0, &q_body, &q_parent) == Some(locked)
            })
            .filter_map(
                |(torpedo, transform, velocity, _, target, target_position)| {
                    let target_velocity = q_velocity.get(target.0).map_or(Vec3::ZERO, |v| v.0);
                    let seconds = torpedo_time_to_impact(
                        transform.translation,
                        velocity.0,
                        target_position.0,
                        target_velocity,
                    )?;
                    Some(TorpedoEta { torpedo, seconds })
                },
            )
            .min_by(|a, b| a.seconds.total_cmp(&b.seconds))
    });
    eta.set_if_neq(PlayerTorpedoEta(soonest));
}

/// Drop [`PlayerTorpedoEta`] the moment its torpedo is gone.
///
/// The update runs inside `SpaceshipSectionSystems`, which stops with the
/// scenario. Without this a torpedo despawned by an unload would leave its
/// last estimate standing, and the tone would keep ticking for a torpedo that
/// no longer exists.
pub(super) fn clear_player_torpedo_eta(
    remove: On<Remove, TorpedoProjectileMarker>,
    mut eta: ResMut<PlayerTorpedoEta>,
) {
    if eta.0.is_some_and(|eta| eta.torpedo == remove.entity) {
        eta.0 = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Closing head on, the estimate is range over closing speed; opening,
    /// there is none.
    #[test]
    fn time_to_impact_is_range_over_closing_speed_and_none_when_opening() {
        let tti = torpedo_time_to_impact(
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, -40.0),
            Vec3::new(0.0, 0.0, -100.0),
            Vec3::new(0.0, 0.0, -15.0),
        );
        assert_eq!(tti, Some(100.0 / 25.0));

        let opening = torpedo_time_to_impact(
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, -10.0),
            Vec3::new(0.0, 0.0, -100.0),
            Vec3::new(0.0, 0.0, -15.0),
        );
        assert_eq!(opening, None);
    }
}
