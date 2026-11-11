//! Player kill credit: a hull the player's own fire reached remembers when,
//! and a defeat inside [`KILL_CREDIT_WINDOW`] after that is the player's
//! kill.
//!
//! Any attributed hit counts - a PDC round, a railgun slug, a torpedo blast,
//! a ram - as long as [`resolve_damage_attacker`] walks it back to the
//! player's ship root. The stinger and the camera jolt that answer
//! [`PlayerKillConfirmed`] are the pilot's confirmation, and a pilot who
//! finishes a hull with the guns earned it as much as one who finished it
//! with a warhead.

use bevy::prelude::*;
use nova_gameplay::prelude::*;

use crate::input::ai::resolve_damage_attacker;

/// `PlayerKillCredit` and `PlayerKillCreditPlugin`.
pub mod prelude {
    pub use super::{PlayerKillCredit, PlayerKillCreditPlugin};
}

/// Seconds the player's credit stays on the hull it reached.
///
/// A hit resolves inside one fixed tick, but the defeat it causes may land
/// later: neutralization is detected from section state on a later frame, and
/// a collapse peels sections over several. Half a second covers both and is
/// short enough that a hull the player only scratched and an ally killed a
/// second later is not credited to the player.
pub(crate) const KILL_CREDIT_WINDOW: f32 = 0.5;

/// The last hit of the player's that reached this ship root, on the virtual
/// clock. Stamped by [`on_damage_stamp_player_credit`] and spent by the first
/// defeat signal inside [`KILL_CREDIT_WINDOW`]. Only the player's hits stamp
/// it, so it carries no owner. Not frozen: a thawed hull starts with no
/// credit.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct PlayerKillCredit {
    /// `Time<Virtual>::elapsed_secs` when the hit reached the hull.
    pub at_secs: f32,
}

impl PlayerKillCredit {
    /// Whether the credit is still fresh at `now`, and spend it either way.
    ///
    /// Spent IN PLACE rather than only removed: a hull that dies outright gets
    /// both defeat signals (the destroy and the [`DefeatedMarker`] it causes)
    /// in one command flush, and a queued remove has not landed when the
    /// second observer reads the credit.
    fn take(&mut self, now: f32) -> bool {
        let fresh = now - self.at_secs <= KILL_CREDIT_WINDOW;
        self.at_secs = f32::NEG_INFINITY;
        fresh
    }
}

/// Stamp the ship root the player's hit reached with [`PlayerKillCredit`].
///
/// Acts on the ORIGINAL target only, the collider the hit landed on, and
/// walks up to its ship root. The root hop is too late: a hit that zeroes the
/// root's pool queues the root's death in the same hop, in no fixed order
/// against this insert, and the destroy observer would read the hull before
/// its credit landed. From the first hop the insert is queued ahead of the
/// root's death.
///
/// The attacker is whoever [`resolve_damage_attacker`] names: the owner of
/// the round, slug or blast, or the striking hull itself for a ram. A hit
/// with no source (a scripted `destroy`), a hit nobody owns (a bare
/// `nova_blast`, a rock) and a hull caught in its own fire stamp nothing. A
/// stream of rounds re-stamps the same second only once.
#[expect(
    clippy::too_many_arguments,
    reason = "one observer: the clock, the attacker walk's four lookups, the ship walk and the credit already there"
)]
pub(crate) fn on_damage_stamp_player_credit(
    damage: On<HealthApplyDamage>,
    time: Res<Time<Virtual>>,
    q_owner: Query<&ProjectileOwner>,
    q_allegiance: Query<&Allegiance>,
    q_parent: Query<&ChildOf>,
    q_ship_root: Query<(), With<SpaceshipRootMarker>>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    q_credit: Query<&PlayerKillCredit>,
    mut commands: Commands,
) {
    if damage.entity != damage.original_event_target() {
        return;
    }
    let Some(source) = damage.source else {
        return;
    };
    let (attacker, _) =
        resolve_damage_attacker(source, &q_owner, &q_allegiance, &q_parent, &q_ship_root);
    let Some(attacker) = attacker.filter(|attacker| q_player.contains(*attacker)) else {
        return;
    };
    let mut ship = damage.entity;
    while !q_ship_root.contains(ship) {
        let Ok(&ChildOf(parent)) = q_parent.get(ship) else {
            return;
        };
        ship = parent;
    }
    if ship == attacker {
        return;
    }
    let at_secs = time.elapsed_secs();
    if q_credit
        .get(ship)
        .is_ok_and(|credit| credit.at_secs == at_secs)
    {
        return;
    }
    commands
        .entity(ship)
        .try_insert(PlayerKillCredit { at_secs });
}

/// Confirm the player's kill when a credited hull is defeated: destroyed or
/// neutralized, the one edge [`DefeatedMarker`] marks.
pub(crate) fn on_defeated_confirm_player_kill(
    add: On<Add<DefeatedMarker>>,
    time: Res<Time<Virtual>>,
    mut q_credit: Query<(&mut PlayerKillCredit, &GlobalTransform)>,
    mut commands: Commands,
) {
    confirm_player_kill(
        add.entity,
        time.elapsed_secs(),
        &mut q_credit,
        &mut commands,
    );
}

/// The destroy edge, for a hull the [`DefeatedMarker`] insert never reaches.
///
/// The destroy path inserts that marker with `try_insert`, racing the reaper's
/// despawn of the same root, so a hull destroyed outright can lose it. A hull
/// already defeated (a neutralized wreck finished off) is not a second kill.
pub(crate) fn on_destroyed_confirm_player_kill(
    add: On<Add<IntegrityDestroyMarker>>,
    time: Res<Time<Virtual>>,
    q_ship: Query<(), (With<SpaceshipRootMarker>, Without<DefeatedMarker>)>,
    mut q_credit: Query<(&mut PlayerKillCredit, &GlobalTransform)>,
    mut commands: Commands,
) {
    if !q_ship.contains(add.entity) {
        return;
    }
    confirm_player_kill(
        add.entity,
        time.elapsed_secs(),
        &mut q_credit,
        &mut commands,
    );
}

/// Spend `ship`'s credit and fire [`PlayerKillConfirmed`] when it was fresh.
fn confirm_player_kill(
    ship: Entity,
    now: f32,
    q_credit: &mut Query<(&mut PlayerKillCredit, &GlobalTransform)>,
    commands: &mut Commands,
) {
    let Ok((mut credit, at)) = q_credit.get_mut(ship) else {
        return;
    };
    let fresh = credit.take(now);
    commands.entity(ship).try_remove::<PlayerKillCredit>();
    if fresh {
        debug!("confirm_player_kill: player kill on {ship:?}");
        commands.trigger(PlayerKillConfirmed {
            at: at.translation(),
        });
    }
}

/// Stamps the player's hits and confirms the defeats they earn. Every build,
/// render or headless: the stinger and the jolt answer the event it fires.
pub struct PlayerKillCreditPlugin;

impl Plugin for PlayerKillCreditPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PlayerKillCredit>();
        app.add_observer(on_damage_stamp_player_credit);
        app.add_observer(on_defeated_confirm_player_kill);
        app.add_observer(on_destroyed_confirm_player_kill);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::prelude::*;

    /// How many kills the rig confirmed.
    #[derive(Resource, Default)]
    struct Confirmed(usize);

    fn kill_app() -> App {
        let mut app = App::new();
        app.add_plugins(nova_gameplay::integrity::health::NovaHealthPlugin);
        // Advanced by hand where a test wants the window to close.
        app.init_resource::<Time<Virtual>>();
        app.add_plugins(PlayerKillCreditPlugin);
        app.init_resource::<Confirmed>();
        app.add_observer(
            |_: On<PlayerKillConfirmed>, mut confirmed: ResMut<Confirmed>| {
                confirmed.0 += 1;
            },
        );
        app
    }

    /// A hull and the section the hits land on.
    fn hull(app: &mut App, defeated: bool) -> (Entity, Entity) {
        let mut root = app.world_mut().spawn((
            SpaceshipRootMarker,
            Health::new(1000.0),
            Transform::default(),
        ));
        if defeated {
            root.insert(DefeatedMarker);
        }
        let root = root.id();
        let section = app
            .world_mut()
            .spawn((SectionMarker, Health::new(500.0), ChildOf(root)))
            .id();
        (root, section)
    }

    fn torpedo_blast(app: &mut App, owner: Entity) -> Entity {
        app.world_mut()
            .spawn((
                NovaBlast {
                    radius: 30.0,
                    max_damage: 750.0,
                    kind: DamageType::Explosive,
                },
                TorpedoSectionPartOf(Entity::PLACEHOLDER),
                ProjectileOwner(owner),
            ))
            .id()
    }

    fn pdc_round(app: &mut App, owner: Entity) -> Entity {
        app.world_mut()
            .spawn((
                TurretBulletProjectileMarker,
                ProjectileDamage::new(4.0, DamageType::Kinetic),
                ProjectileOwner(owner),
            ))
            .id()
    }

    fn railgun_slug(app: &mut App, owner: Entity) -> Entity {
        app.world_mut()
            .spawn((
                RailgunSlugProjectileMarker,
                ProjectileDamage::new(300.0, DamageType::Pierce),
                ProjectileOwner(owner),
            ))
            .id()
    }

    fn hit(app: &mut App, section: Entity, source: Entity) {
        app.world_mut().trigger(HealthApplyDamage {
            entity: section,
            source: Some(source),
            amount: 100.0,
        });
        app.world_mut().flush();
    }

    /// Hit, then defeat outright: both signals in one flush.
    fn hit_and_destroy(app: &mut App, source: Entity) {
        let (ship, section) = hull(app, false);
        hit(app, section, source);
        app.world_mut()
            .commands()
            .entity(ship)
            .insert(IntegrityDestroyMarker)
            .insert(DefeatedMarker);
        app.world_mut().flush();
    }

    fn confirmed(app: &App) -> usize {
        app.world().resource::<Confirmed>().0
    }

    /// A hull the player's round, slug or blast reached confirms the kill
    /// ONCE when it is defeated, neutralized or destroyed outright, however
    /// many defeat signals arrive. A hostile's kill, a kill outside the
    /// window and a derelict that was defeated before the hit confirm nothing.
    #[test]
    fn any_player_kill_is_confirmed_once_and_no_other_kill_is() {
        let mut app = kill_app();
        let player = app
            .world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        let hostile = app.world_mut().spawn(SpaceshipRootMarker).id();
        let player_blast = torpedo_blast(&mut app, player);
        let player_round = pdc_round(&mut app, player);
        let player_slug = railgun_slug(&mut app, player);
        let hostile_blast = torpedo_blast(&mut app, hostile);
        let hostile_round = pdc_round(&mut app, hostile);

        // Neutralized by the blast, then the wreck finished off: one kill.
        let (neutralized, section) = hull(&mut app, false);
        hit(&mut app, section, player_blast);
        app.world_mut()
            .entity_mut(neutralized)
            .insert(DefeatedMarker);
        assert_eq!(confirmed(&app), 1, "a neutralize is a kill");
        app.world_mut()
            .entity_mut(neutralized)
            .insert(IntegrityDestroyMarker);
        assert_eq!(confirmed(&app), 1, "finishing a wreck is not a second kill");

        // Destroyed outright by each of the player's weapons: one kill each.
        hit_and_destroy(&mut app, player_round);
        assert_eq!(confirmed(&app), 2, "a PDC kill is the player's");
        hit_and_destroy(&mut app, player_slug);
        assert_eq!(confirmed(&app), 3, "a railgun kill is the player's");
        hit_and_destroy(&mut app, player_blast);
        assert_eq!(confirmed(&app), 4, "an outright blast kill is one kill");

        // A ram: the player's own hull is the source.
        hit_and_destroy(&mut app, player);
        assert_eq!(confirmed(&app), 5, "a ram is the player's");

        // A hostile's weapons, and the player's own hull hit by itself.
        hit_and_destroy(&mut app, hostile_round);
        hit_and_destroy(&mut app, hostile_blast);
        assert_eq!(confirmed(&app), 5, "only the player's fire confirms");
        app.world_mut().trigger(HealthApplyDamage {
            entity: player,
            source: Some(player_round),
            amount: 100.0,
        });
        app.world_mut().flush();
        assert!(
            app.world().get::<PlayerKillCredit>(player).is_none(),
            "a self-hit stamps nothing"
        );

        // A derelict spawned defeated, then hit and destroyed by the player.
        let (derelict, section) = hull(&mut app, true);
        hit(&mut app, section, player_round);
        app.world_mut()
            .entity_mut(derelict)
            .insert(IntegrityDestroyMarker);
        assert_eq!(confirmed(&app), 5, "a derelict is not a kill");

        // The window: a hull the player hit, defeated by something else later.
        let (late, section) = hull(&mut app, false);
        hit(&mut app, section, player_round);
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .advance_by(Duration::from_secs_f32(KILL_CREDIT_WINDOW * 2.0));
        app.world_mut()
            .entity_mut(late)
            .insert((IntegrityDestroyMarker, DefeatedMarker));
        assert_eq!(confirmed(&app), 5, "a kill outside the window is nobody's");
        assert!(
            app.world().get::<PlayerKillCredit>(late).is_none(),
            "a stale credit is spent, not kept"
        );
    }
}
