//! The player's target lock: angular aim-assist acquisition and the shared
//! lock resource every targeting consumer reads (torpedo launches, the HUD
//! reticle/readout, and - as of the component-lock arc - auto-mode turrets).
//!
//! Extracted from input/player.rs (task 20260709-192503); the acquisition
//! rule lives in pure helpers so it stays unit-testable.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        SpaceshipPlayerTargetLock, SpaceshipTargetingPlugin, SpaceshipTargetingSystems,
    };
}

/// The player's current target lock. `None` means no lock (reticle hidden,
/// torpedoes dumb-fire). Torpedo launches, the HUD and turret auto-fire all
/// consume this one resource.
#[derive(Resource, Debug, Clone, Deref, DerefMut, Default)]
pub struct SpaceshipPlayerTargetLock(pub Option<Entity>);

/// System set for the lock update, so consumers (torpedo commit, turret
/// feed) can order after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpaceshipTargetingSystems;

/// Plugin owning the lock resource and its per-frame acquisition.
pub struct SpaceshipTargetingPlugin;

impl Plugin for SpaceshipTargetingPlugin {
    fn build(&self, app: &mut App) {
        debug!("SpaceshipTargetingPlugin: build");

        app.insert_resource(SpaceshipPlayerTargetLock::default());
        app.add_systems(
            Update,
            update_spaceship_target_input
                .in_set(SpaceshipTargetingSystems)
                .in_set(super::SpaceshipInputSystems),
        );
    }
}

/// Maximum distance at which the aim-assist will lock a target. Bodies further
/// than this from the ship are ignored, so distant clutter never steals the lock.
const TARGETING_MAX_RANGE: f32 = 2000.0;

/// Half-angle (degrees) of the lock-on cone around the aim direction. Any lockable
/// body whose bearing from the ship falls within this angle of where the player is
/// aiming is eligible, and the one closest to the aim ray wins. This is the whole
/// point of the aim-assist: a wide cone means the player only has to point roughly
/// at a target instead of landing a pixel-perfect ray on it. Pan the view and the
/// lock snaps to whichever eligible body is now nearest the center, so cycling
/// between targets is just "look at the next one".
const TARGETING_CONE_HALF_ANGLE_DEG: f32 = 18.0;

/// Range (m) of the close-in "signature" auto-acquisition: with nothing in
/// the aim cone, the nearest hostile inside this range locks by itself, as if
/// the ship's sensors picked up its heat signature. Deliberately well inside
/// [`TARGETING_MAX_RANGE`], so long-range designation stays aim-driven
/// (decided in docs/spikes/20260709-192358-component-lock-vats-lite.md).
const TARGETING_SIGNATURE_RANGE: f32 = 550.0;

/// Choose the closest hostile within `max_range` of `origin` - the signature
/// fallback used when the aim cone is empty. Candidates carry an
/// `is_hostile` flag (minimally: AI-controlled ships, until the faction
/// model 20260708-203708 generalizes hostility); non-hostiles are never
/// auto-acquired, so asteroids and stray torpedoes do not steal the lock.
///
/// Pure and camera/physics-free so the selection rule can be unit-tested
/// directly.
fn pick_signature_target(
    origin: Vec3,
    max_range: f32,
    candidates: impl Iterator<Item = (Entity, Vec3, bool)>,
) -> Option<Entity> {
    candidates
        .filter_map(|(entity, position, is_hostile)| {
            if !is_hostile {
                return None;
            }
            let distance = origin.distance(position);
            (distance <= max_range && distance > f32::EPSILON).then_some((entity, distance))
        })
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(entity, _)| entity)
}

/// Choose the best lock-on target from `candidates` (each an entity and its world
/// position): the one whose bearing from `origin` is closest to the `aim`
/// direction, as long as it is within `max_range` and inside the cone (bearing
/// dot aim `>= min_cos`, i.e. `min_cos = cos(half_angle)`). Returns `None` when
/// nothing qualifies - e.g. the player is looking at empty space - which drops the
/// lock and hides the reticle.
///
/// Pure and camera/physics-free so the selection rule can be unit-tested directly.
fn pick_target(
    origin: Vec3,
    aim: Vec3,
    max_range: f32,
    min_cos: f32,
    candidates: impl Iterator<Item = (Entity, Vec3)>,
) -> Option<Entity> {
    candidates
        .filter_map(|(entity, position)| {
            let to_target = position - origin;
            let distance = to_target.length();
            if distance > max_range || distance < f32::EPSILON {
                return None;
            }
            let cos_angle = to_target.normalize().dot(aim);
            (cos_angle >= min_cos).then_some((entity, cos_angle))
        })
        // Largest cosine == smallest angle from the aim ray == closest to center.
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(entity, _)| entity)
}

/// Update the player's target lock, hybrid-style: angular aim-assist first
/// (enumerate the physical bodies in front of the ship and lock the one
/// nearest the aim direction, see [`pick_target`]), and with an empty cone
/// the signature fallback (nearest hostile inside
/// [`TARGETING_SIGNATURE_RANGE`], see [`pick_signature_target`]).
fn update_spaceship_target_input(
    point_rotation: Single<
        &PointRotationOutput,
        (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipCameraTurretInputMarker>,
        ),
    >,
    // Turret bullets are excluded outright: they are dynamic bodies that stream
    // straight down the aim ray, so without this the lock would constantly snap
    // onto the player's own gunfire instead of the enemy behind it.
    q_candidates: Query<
        (
            Entity,
            &GlobalTransform,
            &RigidBody,
            Option<&TorpedoProjectileMarker>,
            Option<&TorpedoTargetChosen>,
            Has<AISpaceshipMarker>,
        ),
        Without<TurretBulletProjectileMarker>,
    >,
    spaceship: Single<
        (&Transform, Option<&ComputedCenterOfMass>, Entity),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    mut res_target: ResMut<SpaceshipPlayerTargetLock>,
) {
    let point_rotation = point_rotation.into_inner();
    let (ship_transform, ship_com, ship_entity) = spaceship.into_inner();

    // Cone origin on the live structure, not the root origin, so the lock
    // cone agrees with the COM-anchored camera crosshair after losing
    // sections (task 20260709-150711).
    let origin = live_structure_anchor(ship_transform, ship_com);
    let aim = (**point_rotation * Vec3::NEG_Z).normalize();
    let min_cos = TARGETING_CONE_HALF_ANGLE_DEG.to_radians().cos();

    // Collected once because both pickers walk it: the cone pick first, then
    // the signature fallback.
    let candidates: Vec<(Entity, Vec3, bool)> = q_candidates
        .iter()
        .filter_map(
            |(entity, transform, rigid_body, is_torpedo, torpedo_committed, is_hostile)| {
                // Only physical, movable bodies are lockable. This skips static sensor
                // volumes such as scenario trigger areas (`RigidBody::Static`), which
                // are invisible and must never be locked.
                if !matches!(rigid_body, RigidBody::Dynamic) {
                    return None;
                }
                // Never lock the player's own ship.
                if entity == ship_entity {
                    return None;
                }
                // Skip a freshly launched torpedo that has not committed its
                // launch-time target yet: it spawns right on the aim ray and would
                // otherwise be picked as its own target. Once committed
                // (`TorpedoTargetChosen`) a torpedo is a normal lockable body - e.g.
                // you can lock and shoot down your own dumb-fired torpedo.
                if is_torpedo.is_some() && torpedo_committed.is_none() {
                    return None;
                }
                Some((entity, transform.translation(), is_hostile))
            },
        )
        .collect();

    // Aiming designates as always; with an empty cone the nearest hostile
    // inside the signature range auto-acquires (the close-in heat-signature
    // lock from the component-lock spike).
    let cone_pick = pick_target(
        origin,
        aim,
        TARGETING_MAX_RANGE,
        min_cos,
        candidates
            .iter()
            .map(|&(entity, position, _)| (entity, position)),
    );
    **res_target = cone_pick.or_else(|| {
        pick_signature_target(
            origin,
            TARGETING_SIGNATURE_RANGE,
            candidates.iter().copied(),
        )
    });
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn cone_cos(half_angle_deg: f32) -> f32 {
        half_angle_deg.to_radians().cos()
    }

    #[test]
    fn pick_target_locks_the_body_nearest_the_aim_ray() {
        // Two candidates in front: one slightly off-axis, one further off-axis.
        // The nearer-to-center one wins even though it is further away.
        let origin = Vec3::ZERO;
        let aim = Vec3::NEG_Z;
        let near_center = Entity::from_raw_u32(1).unwrap();
        let off_center = Entity::from_raw_u32(2).unwrap();
        let candidates = [
            (near_center, Vec3::new(2.0, 0.0, -100.0)), // ~1.1 deg off axis, far
            (off_center, Vec3::new(3.0, 0.0, -20.0)),   // ~8.5 deg off axis, near
        ];

        let picked = pick_target(
            origin,
            aim,
            TARGETING_MAX_RANGE,
            cone_cos(18.0),
            candidates.into_iter(),
        );
        assert_eq!(picked, Some(near_center));
    }

    #[test]
    fn pick_target_ignores_bodies_outside_the_cone() {
        // A body 90 deg off the aim direction (straight to the side) is not lockable.
        let picked = pick_target(
            Vec3::ZERO,
            Vec3::NEG_Z,
            TARGETING_MAX_RANGE,
            cone_cos(18.0),
            [(Entity::from_raw_u32(1).unwrap(), Vec3::new(50.0, 0.0, 0.0))].into_iter(),
        );
        assert_eq!(picked, None, "a body outside the cone must not be locked");
    }

    #[test]
    fn pick_target_ignores_bodies_behind_the_ship() {
        // A body directly behind (dot with aim is negative) is never locked.
        let picked = pick_target(
            Vec3::ZERO,
            Vec3::NEG_Z,
            TARGETING_MAX_RANGE,
            cone_cos(18.0),
            [(Entity::from_raw_u32(1).unwrap(), Vec3::new(0.0, 0.0, 100.0))].into_iter(),
        );
        assert_eq!(picked, None, "a body behind the ship must not be locked");
    }

    #[test]
    fn pick_target_ignores_bodies_beyond_max_range() {
        // Dead ahead but past the range limit: not lockable.
        let picked = pick_target(
            Vec3::ZERO,
            Vec3::NEG_Z,
            100.0,
            cone_cos(18.0),
            [(
                Entity::from_raw_u32(1).unwrap(),
                Vec3::new(0.0, 0.0, -500.0),
            )]
            .into_iter(),
        );
        assert_eq!(picked, None, "a body beyond max range must not be locked");
    }

    #[test]
    fn pick_target_returns_none_with_no_candidates() {
        let picked = pick_target(
            Vec3::ZERO,
            Vec3::NEG_Z,
            TARGETING_MAX_RANGE,
            cone_cos(18.0),
            std::iter::empty(),
        );
        assert_eq!(picked, None);
    }

    #[test]
    fn lock_cone_originates_at_the_live_structure_anchor() {
        // A candidate dead ahead of the ANCHOR but 33 degrees off the ROOT
        // ORIGIN bearing: it locks only if the cone originates at the anchor
        // (18 degree half-angle).
        let mut world = World::new();
        world.insert_resource(SpaceshipPlayerTargetLock(None));
        world.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraTurretInputMarker,
            PointRotationOutput(Quat::IDENTITY),
        ));
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            ComputedCenterOfMass(Vec3::new(2.0, 0.0, 0.0)),
        ));
        let candidate = world
            .spawn((
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(12.0, 0.0, -3.0)),
            ))
            .id();

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();

        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(candidate),
            "the cone must originate at the anchor, not the root origin"
        );
    }

    #[test]
    fn signature_picks_the_nearest_hostile_in_range() {
        let near = Entity::from_raw_u32(1).unwrap();
        let far = Entity::from_raw_u32(2).unwrap();
        let candidates = [
            (far, Vec3::new(0.0, 0.0, 400.0), true),
            (near, Vec3::new(0.0, 0.0, -200.0), true),
        ];

        let picked = pick_signature_target(Vec3::ZERO, 550.0, candidates.into_iter());

        assert_eq!(picked, Some(near), "nearest hostile wins, direction-blind");
    }

    #[test]
    fn signature_never_acquires_non_hostiles() {
        let rock = Entity::from_raw_u32(1).unwrap();
        let candidates = [(rock, Vec3::new(0.0, 0.0, -50.0), false)];

        let picked = pick_signature_target(Vec3::ZERO, 550.0, candidates.into_iter());

        assert_eq!(picked, None, "asteroids and neutral bodies never auto-lock");
    }

    #[test]
    fn signature_respects_the_range() {
        let hostile = Entity::from_raw_u32(1).unwrap();
        let candidates = [(hostile, Vec3::new(0.0, 0.0, -600.0), true)];

        let picked = pick_signature_target(Vec3::ZERO, 550.0, candidates.into_iter());

        assert_eq!(picked, None, "beyond signature range needs deliberate aim");
    }

    /// Spawn the camera-input rig + player the acquisition system needs.
    fn spawn_acquisition_rig(world: &mut World) {
        world.insert_resource(SpaceshipPlayerTargetLock(None));
        world.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraTurretInputMarker,
            PointRotationOutput(Quat::IDENTITY),
        ));
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::IDENTITY,
        ));
    }

    #[test]
    fn cone_pick_beats_the_signature_fallback() {
        // A hostile BEHIND the player inside signature range, and a body dead
        // ahead in the cone: aiming designates, so the cone target wins.
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        world.spawn((
            AISpaceshipMarker,
            RigidBody::Dynamic,
            GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
        ));
        let aimed = world
            .spawn((
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -300.0)),
            ))
            .id();

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();

        assert_eq!(**world.resource::<SpaceshipPlayerTargetLock>(), Some(aimed));
    }

    #[test]
    fn empty_cone_auto_acquires_the_close_hostile() {
        // Nothing ahead; a hostile behind the player inside signature range
        // locks by itself - the heat-signature acquisition.
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        let hostile = world
            .spawn((
                AISpaceshipMarker,
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
            ))
            .id();

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();

        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(hostile)
        );
    }

    #[test]
    fn empty_cone_ignores_non_hostiles_and_far_hostiles() {
        // A controller-less ship nearby and a hostile beyond signature range:
        // neither auto-acquires, the lock stays empty.
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        world.spawn((
            SpaceshipRootMarker,
            RigidBody::Dynamic,
            GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
        ));
        world.spawn((
            AISpaceshipMarker,
            RigidBody::Dynamic,
            GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 900.0)),
        ));

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();

        assert_eq!(**world.resource::<SpaceshipPlayerTargetLock>(), None);
    }
}
