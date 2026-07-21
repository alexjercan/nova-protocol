//! Owner-aware collision filtering shared by all projectile kinds.
//!
//! Every projectile (turret bullet, torpedo) records the body that fired it in
//! [`ProjectileOwner`]. [`ProjectileHooks`] is the app's single avian
//! [`CollisionHooks`] implementation (avian allows exactly one hook type per
//! app, registered in `NovaGameplayPlugin`); it skips any contact pair between
//! a projectile and its owner, so a freshly fired projectile does not collide
//! with - and take impact damage from - the ship it just left.

use avian3d::prelude::*;
use bevy::{
    ecs::system::{lifetimeless::Read, SystemParam},
    prelude::*,
};

/// Glob-import surface: `use nova_gameplay::sections::projectile_hooks::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{ProjectileHooks, ProjectileOwner};
}

/// The body (a spaceship root) that fired this projectile.
///
/// Lives on the projectile's rigid-body root. [`ProjectileHooks`] uses it to
/// keep the projectile from ever contact-colliding with its owner: without the
/// filter a projectile spawns overlapping the firing ship and leaves at muzzle
/// speed, so the impact-damage pipeline would kill it (and ding the ship) on
/// frame one. The filter is deliberately permanent rather than launch-gated:
/// `filter_pairs` runs when a broad-phase pair is created, so a filter whose
/// answer changes mid-overlap would not be re-evaluated reliably. Blast damage
/// is a separate sensor path and still affects the owner.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, PartialEq, Eq, Reflect)]
pub struct ProjectileOwner(pub Entity);

/// The single avian collision hook: skips contact pairs between a projectile
/// and the body that fired it. Colliders opt in with
/// [`ActiveCollisionHooks::FILTER_PAIRS`] on the entity that carries the
/// collider - for the torpedo that is each child section, not the root.
#[derive(SystemParam)]
pub struct ProjectileHooks<'w, 's> {
    q_owner: Query<'w, 's, Read<ProjectileOwner>>,
    q_collider_of: Query<'w, 's, Read<ColliderOf>>,
}

impl ProjectileHooks<'_, '_> {
    /// The owner governing `collider`, if it is (part of) a projectile: the
    /// [`ProjectileOwner`] on the collider entity itself (turret bullet: root
    /// and collider are one entity) or on the collider's rigid body (torpedo:
    /// the colliders are child sections of the owning root).
    fn owner_of(&self, collider: Entity) -> Option<Entity> {
        if let Ok(&ProjectileOwner(owner)) = self.q_owner.get(collider) {
            return Some(owner);
        }
        let &ColliderOf { body } = self.q_collider_of.get(collider).ok()?;
        let &ProjectileOwner(owner) = self.q_owner.get(body).ok()?;
        Some(owner)
    }

    /// The rigid body `collider` belongs to.
    fn body_of(&self, collider: Entity) -> Option<Entity> {
        let &ColliderOf { body } = self.q_collider_of.get(collider).ok()?;
        Some(body)
    }
}

impl CollisionHooks for ProjectileHooks<'_, '_> {
    fn filter_pairs(&self, collider1: Entity, collider2: Entity, _commands: &mut Commands) -> bool {
        // A projectile never contact-collides with the body that fired it, in
        // either orientation of the pair. Everything else keeps colliding.
        let owned_pair = |projectile: Entity, other: Entity| {
            self.owner_of(projectile)
                .is_some_and(|owner| self.body_of(other) == Some(owner))
        };
        !(owned_pair(collider1, collider2) || owned_pair(collider2, collider1))
    }
}

/// Physics-level tests: a real avian world with the hook registered, mirroring
/// how `NovaGameplayPlugin` wires it. They assert the *outcome of a launch*
/// (nobody took contact damage, the projectile flew on unperturbed), not
/// intermediate collision events.
#[cfg(test)]
mod physics_tests {
    use bevy_common_systems::prelude::*;

    use super::*;
    use crate::{
        integrity::test_support::{settle, unfinished_integrity_physics_app_with},
        prelude::{SectionMarker, SpaceshipRootMarker},
    };

    fn hooks_app() -> App {
        let mut app = unfinished_integrity_physics_app_with(
            PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
        );
        app.finish();
        app
    }

    /// A one-section ship body at `at` (as `base_section` builds sections:
    /// `SectionMarker` + cuboid collider + density + health).
    fn spawn_ship(app: &mut App, at: Vec3, health: f32) -> (Entity, Entity) {
        let root = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::from_translation(at),
                SpaceshipRootMarker,
            ))
            .id();
        let section = app
            .world_mut()
            .spawn((
                ChildOf(root),
                SectionMarker,
                Transform::default(),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
                Health::new(health),
            ))
            .id();
        (root, section)
    }

    /// A torpedo-shaped projectile as `shoot_spawn_projectile` builds it: the
    /// owner on the collider-less root, the collider + health on a child
    /// section flagged for pair filtering.
    fn spawn_torpedo(
        app: &mut App,
        owner: Entity,
        at: Vec3,
        velocity: Vec3,
        health: f32,
    ) -> (Entity, Entity) {
        let root = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::from_translation(at),
                LinearVelocity(velocity),
                ProjectileOwner(owner),
            ))
            .id();
        let warhead = app
            .world_mut()
            .spawn((
                ChildOf(root),
                SectionMarker,
                Transform::default(),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
                Health::new(health),
                ActiveCollisionHooks::FILTER_PAIRS,
            ))
            .id();
        (root, warhead)
    }

    fn run(app: &mut App, updates: usize) {
        settle(app);
        for _ in 0..updates {
            app.update();
        }
    }

    fn health_of(app: &App, entity: Entity) -> Option<&Health> {
        app.world().get::<Health>(entity)
    }

    #[test]
    fn a_torpedo_launched_inside_its_own_ship_takes_and_deals_no_contact_damage() {
        // The bug scenario: the torpedo spawns overlapping the bay of the ship
        // that fired it and leaves at muzzle speed. With the owner filter the
        // launch is clean: full health on both sides, and the torpedo's motion
        // is untouched by any contact response.
        let mut app = hooks_app();
        let (ship, ship_section) = spawn_ship(&mut app, Vec3::ZERO, 100.0);
        let (torpedo, warhead) = spawn_torpedo(&mut app, ship, Vec3::ZERO, Vec3::NEG_Z * 20.0, 1.0);

        run(&mut app, 10);

        let warhead_health = health_of(&app, warhead).expect("warhead survived the launch");
        assert_eq!(warhead_health.current, warhead_health.max);
        let ship_health = health_of(&app, ship_section).expect("ship section survived the launch");
        assert_eq!(ship_health.current, ship_health.max);

        // No contact response either: the torpedo still flies exactly its
        // launch velocity (zero gravity, no damping in this harness).
        let velocity = app.world().get::<LinearVelocity>(torpedo).unwrap();
        assert!(
            (velocity.0 - Vec3::NEG_Z * 20.0).length() < 1e-3,
            "torpedo velocity was perturbed by a filtered contact: {:?}",
            velocity.0
        );
    }

    #[test]
    fn a_torpedo_overlapping_a_body_it_does_not_own_still_collides() {
        // Control for the filter: the same overlap against a ship that did NOT
        // fire the torpedo must produce a real contact - impact damage lands on
        // at least one side of the pair.
        // High health on both sides: the invariant is "contact damage lands",
        // and staying far from zero keeps the destroy/explode pipeline (whose
        // render-facing observer cannot run headless) out of the test.
        let mut app = hooks_app();
        let (owner_ship, _) = spawn_ship(&mut app, Vec3::new(100.0, 0.0, 0.0), 1e6);
        let (_, target_section) = spawn_ship(&mut app, Vec3::ZERO, 1e6);
        let (_, warhead) = spawn_torpedo(&mut app, owner_ship, Vec3::ZERO, Vec3::NEG_Z * 20.0, 1e6);

        run(&mut app, 10);

        let warhead_health = health_of(&app, warhead).expect("warhead alive at 1e6 hp");
        let target_health = health_of(&app, target_section).expect("target alive at 1e6 hp");
        let warhead_damaged = warhead_health.current < warhead_health.max;
        let target_damaged = target_health.current < target_health.max;
        assert!(
            warhead_damaged || target_damaged,
            "an unowned overlap produced no contact damage on either side"
        );
    }

    #[test]
    fn a_turret_bullet_still_ignores_the_ship_that_fired_it() {
        // Regression for the generalized hook: the bullet carries the owner on
        // the collider entity itself (root and collider are one entity), the
        // lookup path the old TurretProjectileHooks covered.
        let mut app = hooks_app();
        let (ship, ship_section) = spawn_ship(&mut app, Vec3::ZERO, 100.0);
        let bullet = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                LinearVelocity(Vec3::NEG_Z * 100.0),
                Collider::sphere(0.05),
                Mass(0.5),
                ProjectileOwner(ship),
                ActiveCollisionHooks::FILTER_PAIRS,
            ))
            .id();

        run(&mut app, 10);

        let ship_health = health_of(&app, ship_section).expect("ship section survived");
        assert_eq!(ship_health.current, ship_health.max);
        let velocity = app.world().get::<LinearVelocity>(bullet).unwrap();
        assert!(
            (velocity.0 - Vec3::NEG_Z * 100.0).length() < 1e-3,
            "bullet velocity was perturbed by a filtered contact: {:?}",
            velocity.0
        );
    }

    #[test]
    fn filtering_is_symmetric_in_pair_orientation() {
        // Locks the promise in filter_pairs: the owner pair is skipped no
        // matter which collider avian happens to pass first, and a non-owner
        // pair collides in both orientations. Calls the hook directly on a
        // bare world (ColliderOf inserted by hand), no physics stepping.
        use bevy::ecs::system::SystemState;

        let mut world = World::new();
        let ship = world.spawn_empty().id();
        let ship_section = world.spawn(ColliderOf { body: ship }).id();
        let torpedo = world.spawn(ProjectileOwner(ship)).id();
        let warhead = world.spawn(ColliderOf { body: torpedo }).id();
        let other_body = world.spawn_empty().id();
        let other_collider = world.spawn(ColliderOf { body: other_body }).id();

        let mut state = SystemState::<(ProjectileHooks, Commands)>::new(&mut world);
        let (hooks, mut commands) = state.get_mut(&mut world).unwrap();

        assert!(!hooks.filter_pairs(warhead, ship_section, &mut commands));
        assert!(!hooks.filter_pairs(ship_section, warhead, &mut commands));
        assert!(hooks.filter_pairs(warhead, other_collider, &mut commands));
        assert!(hooks.filter_pairs(other_collider, warhead, &mut commands));
    }
}
