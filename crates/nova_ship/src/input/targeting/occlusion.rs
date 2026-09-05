//! Line of sight for the lock: a radio link, and what stops it.
//!
//! A lock is a radio link between the scanner and the body it holds, so a body
//! that is opaque to radar ([`RadarOccluder`]) standing on that line breaks it.
//! The rule is one ray, asked at collection time, so the radar pick, lock
//! validity and the threat set cannot disagree about what the ship can see.
//!
//! It lives here rather than in the player's targeting because the AI's own
//! acquisition asks the same question of the same world: a hostile cannot
//! pick a ship it has no line to either.

use avian3d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};

use crate::prelude::*;

/// The spatial half of the lock scanner: what stands between the ship and a
/// body it might otherwise lock.
///
/// A [`SystemParam`] rather than a free function taking six arguments, because
/// every caller needs the same three world reads and none of them needs to
/// know that a ray cast is how the question is answered.
#[derive(SystemParam)]
pub(crate) struct RadarScan<'w, 's> {
    spatial: SpatialQuery<'w, 's>,
    /// Every collider that stops radar. The marker rides the COLLIDER (see
    /// [`RadarOccluder`]), which is what the ray meets.
    occluders: Query<'w, 's, (), With<RadarOccluder>>,
    /// Which body a collider belongs to - avian's own mapping, the same one
    /// the AI's line-of-FIRE gate reads, so a nested collider resolves to
    /// the body a lock actually names.
    collider_of: Query<'w, 's, &'static ColliderOf>,
}

impl RadarScan<'_, '_> {
    /// Whether a body that stops radar stands between `origin` and the target
    /// `body` at `at`.
    ///
    /// The target's OWN occluding collider never blocks it: a rock is lockable,
    /// and the ray to its centre goes through its own hull to get there.
    pub(crate) fn is_occluded(&self, origin: Vec3, at: Vec3, body: Entity) -> bool {
        let reach = at - origin;
        let Ok(direction) = Dir3::new(reach) else {
            // The scanner is standing on the body. Nothing can be between them.
            return false;
        };
        self.spatial
            .cast_ray_predicate(
                origin,
                direction,
                reach.length(),
                // Solid: a ray that starts inside an occluder is inside cover,
                // not looking out of a hollow shell.
                true,
                &SpatialQueryFilter::default(),
                &|collider| self.stops_radar_for(collider, body),
            )
            .is_some()
    }

    /// Whether `collider` is opaque to radar looking for `body` - which its
    /// own colliders are not.
    ///
    /// An occluding collider avian cannot attribute to a body counts as
    /// cover: failing closed loses a lock, failing open locks through a
    /// world.
    fn stops_radar_for(&self, collider: Entity, body: Entity) -> bool {
        self.occluders.contains(collider)
            && self.collider_of.get(collider).map(|of| of.body) != Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use avian3d::prelude::*;
    use bevy::{ecs::system::RunSystemOnce, prelude::*};
    use nova_gameplay::{
        prelude::*,
        test_support::{settle, unfinished_integrity_physics_app},
    };

    use super::{super::contacts::update_contacts_and_locks, RadarOccluder, RadarScan};
    use crate::prelude::*;

    /// One line-of-sight question and the answer the scan gave it.
    #[derive(Resource)]
    struct Sightline {
        origin: Vec3,
        at: Vec3,
        body: Entity,
        blocked: bool,
    }

    fn answer_the_sightline(scan: RadarScan, mut line: ResMut<Sightline>) {
        line.blocked = scan.is_occluded(line.origin, line.at, line.body);
    }

    /// A rock at `at`: the body root a lock would name, with the hull that
    /// stops radar on a CHILD collider - the shape the asteroid spawner
    /// builds, and the shape the target's-own-hull rule has to survive.
    fn spawn_rock(app: &mut App, at: Vec3, radius: f32) -> Entity {
        let body = app
            .world_mut()
            .spawn((
                Name::new("rock"),
                RigidBody::Dynamic,
                Transform::from_translation(at),
            ))
            .id();
        app.world_mut().spawn((
            ChildOf(body),
            Transform::default(),
            Collider::sphere(radius),
            ColliderDensity(1.0),
            RadarOccluder,
        ));
        body
    }

    /// A ship-shaped body with no collider of its own: what the lock is for,
    /// and never what blocks one.
    fn spawn_contact(app: &mut App, at: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                Name::new("contact"),
                SpaceshipRootMarker,
                AISpaceshipMarker,
                RigidBody::Dynamic,
                Transform::from_translation(at),
            ))
            .id()
    }

    /// Whether the scanner at the origin can see `body` at `at`, asked of a
    /// real collider tree.
    fn blocked(app: &mut App, at: Vec3, body: Entity) -> bool {
        app.insert_resource(Sightline {
            origin: Vec3::ZERO,
            at,
            body,
            blocked: false,
        });
        app.world_mut()
            .run_system_once(answer_the_sightline)
            .unwrap();
        app.world().resource::<Sightline>().blocked
    }

    #[test]
    fn a_rock_between_the_ship_and_a_contact_stops_the_radar() {
        let mut app = unfinished_integrity_physics_app();
        let contact = spawn_contact(&mut app, Vec3::new(0.0, 0.0, -400.0));
        spawn_rock(&mut app, Vec3::new(0.0, 0.0, -200.0), 50.0);
        app.finish();
        settle(&mut app);
        assert!(blocked(&mut app, Vec3::new(0.0, 0.0, -400.0), contact));
    }

    #[test]
    fn a_rock_off_the_line_stops_nothing() {
        let mut app = unfinished_integrity_physics_app();
        let contact = spawn_contact(&mut app, Vec3::new(0.0, 0.0, -400.0));
        spawn_rock(&mut app, Vec3::new(300.0, 0.0, -200.0), 50.0);
        app.finish();
        settle(&mut app);
        assert!(!blocked(&mut app, Vec3::new(0.0, 0.0, -400.0), contact));
    }

    #[test]
    fn a_rock_beyond_the_contact_stops_nothing() {
        let mut app = unfinished_integrity_physics_app();
        let contact = spawn_contact(&mut app, Vec3::new(0.0, 0.0, -400.0));
        spawn_rock(&mut app, Vec3::new(0.0, 0.0, -600.0), 50.0);
        app.finish();
        settle(&mut app);
        assert!(
            !blocked(&mut app, Vec3::new(0.0, 0.0, -400.0), contact),
            "the ray stops at the contact; what is behind it was never on the line"
        );
    }

    #[test]
    fn a_rock_is_lockable_through_its_own_hull() {
        let mut app = unfinished_integrity_physics_app();
        let rock = spawn_rock(&mut app, Vec3::new(0.0, 0.0, -300.0), 50.0);
        app.finish();
        settle(&mut app);
        assert!(
            !blocked(&mut app, Vec3::new(0.0, 0.0, -300.0), rock),
            "the ray to a rock's centre goes through the rock; its own hull \
             cannot be what hides it"
        );
    }

    #[test]
    fn a_rock_that_comes_between_drops_the_combat_lock_and_names_the_branch() {
        let mut app = unfinished_integrity_physics_app();
        app.init_resource::<TargetingSettings>();
        app.init_resource::<Messages<CombatLockDropped>>();
        let contact = spawn_contact(&mut app, Vec3::new(0.0, 0.0, -400.0));
        let player = app
            .world_mut()
            .spawn((
                Name::new("player"),
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::IDENTITY,
                targeting_state(),
            ))
            .id();
        // The rock starts wide of the line, so the lock is one a clear sky
        // handed over rather than one this test asserted into place.
        let rock = spawn_rock(&mut app, Vec3::new(600.0, 0.0, -200.0), 50.0);
        app.finish();
        settle(&mut app);

        app.world_mut()
            .run_system_once(update_contacts_and_locks)
            .unwrap();
        app.world_mut().get_mut::<CombatLock>(player).unwrap().0 = Some(contact);
        app.world_mut()
            .run_system_once(update_contacts_and_locks)
            .unwrap();
        assert_eq!(
            app.world().get::<CombatLock>(player).unwrap().0,
            Some(contact),
            "a lock with a clear line to its target holds"
        );
        app.world_mut()
            .resource_mut::<Messages<CombatLockDropped>>()
            .clear();

        // Slide it onto the line and let the tree see it move.
        app.world_mut()
            .get_mut::<Transform>(rock)
            .unwrap()
            .translation = Vec3::new(0.0, 0.0, -200.0);
        settle(&mut app);
        app.world_mut()
            .run_system_once(update_contacts_and_locks)
            .unwrap();

        assert_eq!(
            app.world().get::<CombatLock>(player).unwrap().0,
            None,
            "cover breaks the link, so the lock lets go"
        );
        let dropped: Vec<(Entity, CombatLockDrop)> = app
            .world_mut()
            .resource_mut::<Messages<CombatLockDropped>>()
            .drain()
            .map(|drop| (drop.target, drop.reason))
            .collect();
        assert_eq!(
            dropped,
            vec![(contact, CombatLockDrop::Occluded)],
            "and says it was cover, not range and not a death"
        );
    }

    #[test]
    fn the_scan_reads_the_live_collider_tree() {
        let mut app = unfinished_integrity_physics_app();
        let contact = spawn_contact(&mut app, Vec3::new(0.0, 0.0, -400.0));
        let rock = spawn_rock(&mut app, Vec3::new(0.0, 0.0, -200.0), 50.0);
        app.finish();
        settle(&mut app);
        assert!(blocked(&mut app, Vec3::new(0.0, 0.0, -400.0), contact));
        app.world_mut().entity_mut(rock).despawn();
        settle(&mut app);
        assert!(
            !blocked(&mut app, Vec3::new(0.0, 0.0, -400.0), contact),
            "a rock that is gone stops nothing - the scan reads the tree, not \
             a snapshot"
        );
    }
}
