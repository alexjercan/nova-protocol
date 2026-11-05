//! Torpedo kill credit: the hull a torpedo blast reached remembers whose
//! warhead it was, and a defeat inside [`KILL_CREDIT_WINDOW`] after that is the
//! player's kill when the warhead was the player's.
//!
//! Only blasts count. A PDC stream that finishes a hull is not a torpedo kill,
//! and the stinger this feeds belongs to the one weapon whose hit decides a
//! fight.

use super::*;

/// Seconds a torpedo blast's credit stays on the hull it reached.
///
/// A blast resolves inside one fixed tick, but the defeat it causes may land
/// later: neutralization is detected from section state on a later frame, and
/// a collapse peels sections over several. Half a second covers both and is
/// short enough that a hull a torpedo only scratched and a PDC killed a second
/// later is not credited to the torpedo.
pub(super) const KILL_CREDIT_WINDOW: f32 = 0.5;

/// The last torpedo blast that reached this ship root: whose it was and when,
/// on the virtual clock. Stamped by [`on_damage_stamp_torpedo_credit`] and
/// spent by the first defeat signal inside [`KILL_CREDIT_WINDOW`]. Not frozen:
/// a thawed hull starts with no credit.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct TorpedoBlastCredit {
    /// The ship that fired the torpedo.
    pub owner: Entity,
    /// `Time<Virtual>::elapsed_secs` when the blast reached the hull.
    pub at_secs: f32,
}

impl TorpedoBlastCredit {
    /// The owner when the credit is still fresh at `now`, and spend it either
    /// way.
    ///
    /// Spent IN PLACE rather than only removed: a hull that dies outright gets
    /// both defeat signals (the destroy and the [`DefeatedMarker`] it causes)
    /// in one command flush, and a queued remove has not landed when the
    /// second observer reads the credit.
    fn take(&mut self, now: f32) -> Option<Entity> {
        let fresh = now - self.at_secs <= KILL_CREDIT_WINDOW;
        self.at_secs = f32::NEG_INFINITY;
        fresh.then_some(self.owner)
    }
}

/// Stamp the ship root a torpedo blast reached with [`TorpedoBlastCredit`].
///
/// Acts on the ORIGINAL target only, the collider the blast hit, and walks up
/// to its ship root. The root hop is too late: a hit that zeroes the root's
/// pool queues the root's death in the same hop, in no fixed order against
/// this insert, and the destroy observer would read the hull before its
/// credit landed. From the first hop the insert is queued ahead of the root's
/// death.
///
/// A blast is a torpedo's when it carries [`TorpedoSectionPartOf`]; a bare
/// `nova_blast` (a scripted detonation, a range rig) credits nobody. A hull
/// caught in its own torpedo's blast credits nobody either.
pub(super) fn on_damage_stamp_torpedo_credit(
    damage: On<HealthApplyDamage>,
    time: Res<Time<Virtual>>,
    q_blast: Query<&ProjectileOwner, (With<NovaBlast>, With<TorpedoSectionPartOf>)>,
    q_parent: Query<&ChildOf>,
    q_ship_root: Query<(), With<SpaceshipRootMarker>>,
    mut commands: Commands,
) {
    if damage.entity != damage.original_event_target() {
        return;
    }
    let Some(source) = damage.source else {
        return;
    };
    let Ok(&ProjectileOwner(owner)) = q_blast.get(source) else {
        return;
    };
    let mut ship = damage.entity;
    while !q_ship_root.contains(ship) {
        let Ok(&ChildOf(parent)) = q_parent.get(ship) else {
            return;
        };
        ship = parent;
    }
    if ship == owner {
        return;
    }
    commands.entity(ship).try_insert(TorpedoBlastCredit {
        owner,
        at_secs: time.elapsed_secs(),
    });
}

/// Confirm the player's kill when a credited hull is defeated: destroyed or
/// neutralized, the one edge [`DefeatedMarker`] marks.
pub(super) fn on_defeated_confirm_player_kill(
    add: On<Add, DefeatedMarker>,
    time: Res<Time<Virtual>>,
    mut q_credit: Query<(&mut TorpedoBlastCredit, &GlobalTransform)>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut commands: Commands,
) {
    confirm_player_kill(
        add.entity,
        time.elapsed_secs(),
        &mut q_credit,
        &q_player,
        &mut commands,
    );
}

/// The destroy edge, for a hull the [`DefeatedMarker`] insert never reaches.
///
/// The destroy path inserts that marker with `try_insert`, racing the reaper's
/// despawn of the same root, so a hull destroyed outright can lose it. A hull
/// already defeated (a neutralized wreck finished off) is not a second kill.
pub(super) fn on_destroyed_confirm_player_kill(
    add: On<Add, IntegrityDestroyMarker>,
    time: Res<Time<Virtual>>,
    q_ship: Query<(), (With<SpaceshipRootMarker>, Without<DefeatedMarker>)>,
    mut q_credit: Query<(&mut TorpedoBlastCredit, &GlobalTransform)>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut commands: Commands,
) {
    if !q_ship.contains(add.entity) {
        return;
    }
    confirm_player_kill(
        add.entity,
        time.elapsed_secs(),
        &mut q_credit,
        &q_player,
        &mut commands,
    );
}

/// Spend `ship`'s credit and fire [`PlayerKillConfirmed`] when it was fresh
/// and the player's.
fn confirm_player_kill(
    ship: Entity,
    now: f32,
    q_credit: &mut Query<(&mut TorpedoBlastCredit, &GlobalTransform)>,
    q_player: &Query<(), With<PlayerSpaceshipMarker>>,
    commands: &mut Commands,
) {
    let Ok((mut credit, at)) = q_credit.get_mut(ship) else {
        return;
    };
    let owner = credit.take(now);
    commands.entity(ship).try_remove::<TorpedoBlastCredit>();
    if owner.is_some_and(|owner| q_player.contains(owner)) {
        debug!("confirm_player_kill: player torpedo kill on {ship:?}");
        commands.trigger(PlayerKillConfirmed {
            at: at.translation(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How many kills the rig confirmed.
    #[derive(Resource, Default)]
    struct Confirmed(usize);

    fn kill_app() -> App {
        let mut app = App::new();
        app.add_plugins(nova_gameplay::integrity::health::NovaHealthPlugin);
        // Never advanced: every hit and defeat lands inside the window.
        app.init_resource::<Time<Virtual>>();
        app.add_observer(on_damage_stamp_torpedo_credit);
        app.add_observer(on_defeated_confirm_player_kill);
        app.add_observer(on_destroyed_confirm_player_kill);
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

    fn hit(app: &mut App, section: Entity, source: Entity) {
        app.world_mut().trigger(HealthApplyDamage {
            entity: section,
            source: Some(source),
            amount: 100.0,
        });
        app.world_mut().flush();
    }

    fn confirmed(app: &App) -> usize {
        app.world().resource::<Confirmed>().0
    }

    /// A hull the player's torpedo blast reached confirms the kill ONCE when
    /// it is defeated, neutralized or destroyed outright, however many defeat
    /// signals arrive. A PDC kill, a hostile's blast and a derelict that was
    /// defeated before the blast confirm nothing.
    #[test]
    fn a_player_torpedo_kill_is_confirmed_once_and_no_other_kill_is() {
        let mut app = kill_app();
        let player = app
            .world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        let hostile = app.world_mut().spawn(SpaceshipRootMarker).id();
        let player_blast = torpedo_blast(&mut app, player);
        let hostile_blast = torpedo_blast(&mut app, hostile);
        let player_round = app
            .world_mut()
            .spawn((
                ProjectileDamage::new(4.0, DamageType::Kinetic),
                ProjectileOwner(player),
            ))
            .id();

        // Neutralized, then the wreck finished off: one kill.
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

        // Destroyed outright: both defeat signals in one flush, one kill.
        let (destroyed, section) = hull(&mut app, false);
        hit(&mut app, section, player_blast);
        app.world_mut()
            .commands()
            .entity(destroyed)
            .insert(IntegrityDestroyMarker)
            .insert(DefeatedMarker);
        app.world_mut().flush();
        assert_eq!(confirmed(&app), 2, "an outright destroy is one kill");

        // The player's PDC alone, and a hostile's torpedo: no stinger.
        for source in [player_round, hostile_blast] {
            let (ship, section) = hull(&mut app, false);
            hit(&mut app, section, source);
            app.world_mut()
                .entity_mut(ship)
                .insert((IntegrityDestroyMarker, DefeatedMarker));
        }
        assert_eq!(confirmed(&app), 2, "only the player's torpedo confirms");

        // A derelict spawned defeated, then hit and destroyed by the player.
        let (derelict, section) = hull(&mut app, true);
        hit(&mut app, section, player_blast);
        app.world_mut()
            .entity_mut(derelict)
            .insert(IntegrityDestroyMarker);
        assert_eq!(confirmed(&app), 2, "a derelict is not a kill");
    }
}
