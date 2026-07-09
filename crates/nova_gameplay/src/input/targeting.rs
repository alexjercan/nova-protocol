//! The player's target lock: angular aim-assist acquisition and the shared
//! lock resource every targeting consumer reads (torpedo launches, the HUD
//! reticle/readout, and - as of the component-lock arc - auto-mode turrets).
//!
//! Extracted from input/player.rs (task 20260709-192503); the acquisition
//! rule lives in pure helpers so it stays unit-testable.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        ComponentLockMode, SpaceshipPlayerComponentLock, SpaceshipPlayerLockFocus,
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
        app.insert_resource(SpaceshipPlayerLockFocus::default());
        app.insert_resource(SpaceshipPlayerComponentLock::default());
        app.add_systems(
            Update,
            (
                update_spaceship_target_input,
                tick_lock_focus,
                update_component_lock,
            )
                .chain()
                .in_set(SpaceshipTargetingSystems)
                .in_set(super::SpaceshipInputSystems),
        );
        app.add_observer(on_component_cycle_next);
        app.add_observer(on_component_cycle_prev);
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

/// Seconds of continuous lock on the same target before the component layer
/// unlocks (the WoT-style aim-in dwell from the component-lock spike,
/// docs/spikes/20260709-192358-component-lock-vats-lite.md).
const FOCUS_TIME: f32 = 1.5;

/// Seconds a cycle press pins the component selection before aim-snap
/// resumes. A feel knob; tune in playtest.
const COMPONENT_PIN_WINDOW: f32 = 2.0;

/// Snap hysteresis: a challenger section only steals the fine lock when its
/// ray distance is below this fraction of the incumbent's, so the selection
/// does not flicker between adjacent sections. A feel knob; tune in playtest.
const SNAP_HYSTERESIS: f32 = 0.75;

/// Focus: how long the current lock has been held on the same target.
/// Component fine-locking unlocks at [`FOCUS_TIME`]; the HUD renders the
/// fill fraction while it accumulates.
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct SpaceshipPlayerLockFocus {
    /// The target the timer is accumulating on (mirrors the lock).
    pub target: Option<Entity>,
    /// Continuous seconds the lock has stayed on `target`.
    pub seconds: f32,
}

impl SpaceshipPlayerLockFocus {
    /// Focus completion in [0, 1], for the HUD meter.
    pub fn fraction(&self) -> f32 {
        (self.seconds / FOCUS_TIME).clamp(0.0, 1.0)
    }

    /// Whether the component layer is unlocked for `target`.
    pub fn focused_on(&self, target: Entity) -> bool {
        self.target == Some(target) && self.seconds >= FOCUS_TIME
    }
}

/// How the component fine-lock is currently selected.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ComponentLockMode {
    /// Follow the live section nearest the crosshair ray (with hysteresis).
    #[default]
    Snap,
    /// A cycle press chose deliberately; snap is suppressed until `until`
    /// (Time::elapsed_secs) or until the pinned section dies.
    Pinned {
        /// Elapsed-time deadline after which snap resumes.
        until: f32,
    },
}

/// The fine-locked section of the locked ship, only ever `Some` while the
/// focus dwell is complete. Sections stay lockable while ATTACHED - a
/// disabled-in-place section (`SectionInactiveMarker`) can still be targeted
/// to blow it off the hull; despawn/detach clears the selection (decision
/// from the component-lock spike, lockable-while-attached).
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct SpaceshipPlayerComponentLock {
    /// The fine-locked section entity (a `SectionMarker` child of the lock).
    pub section: Option<Entity>,
    /// Snap or pinned-by-cycle selection.
    pub mode: ComponentLockMode,
}

/// Cycle the component fine-lock to the next section (stable order).
#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct ComponentCycleNextInput;

/// Cycle the component fine-lock to the previous section (stable order).
#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct ComponentCyclePrevInput;

/// Range (m) of the close-in "signature" auto-acquisition: with nothing in
/// the aim cone, the nearest hostile inside this range locks by itself, as if
/// the ship's sensors picked up its heat signature. Deliberately well inside
/// [`TARGETING_MAX_RANGE`], so long-range designation stays aim-driven
/// (decided in docs/spikes/20260709-192358-component-lock-vats-lite.md).
const TARGETING_SIGNATURE_RANGE: f32 = 550.0;

/// Choose the closest hostile within `max_range` of `origin` - the signature
/// fallback used when the aim cone is empty. Candidates carry an
/// `is_hostile` flag (resolved by the relation model: [`relation`] vs the
/// player is [`Relation::Hostile`]); non-hostiles are never auto-acquired,
/// so asteroids, neutrals and stray torpedoes do not steal the lock.
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
            Option<&Allegiance>,
        ),
        Without<TurretBulletProjectileMarker>,
    >,
    spaceship: Single<
        (
            &Transform,
            Option<&ComputedCenterOfMass>,
            Entity,
            Option<&Allegiance>,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    mut res_target: ResMut<SpaceshipPlayerTargetLock>,
) {
    let point_rotation = point_rotation.into_inner();
    let (ship_transform, ship_com, ship_entity, ship_allegiance) = spaceship.into_inner();

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
            |(entity, transform, rigid_body, is_torpedo, torpedo_committed, allegiance)| {
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
                // Hostility comes from the relation model, so an enemy's
                // torpedo is auto-acquirable while the player's own is not.
                let is_hostile = relation(ship_allegiance, allegiance) == Relation::Hostile;
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

/// Accumulate focus while the lock stays on one target; any change (new
/// target or lock lost) restarts the dwell from zero.
fn tick_lock_focus(
    time: Res<Time>,
    lock: Res<SpaceshipPlayerTargetLock>,
    mut focus: ResMut<SpaceshipPlayerLockFocus>,
) {
    if focus.target != **lock {
        focus.target = **lock;
        focus.seconds = 0.0;
        return;
    }
    if focus.target.is_some() {
        focus.seconds += time.delta_secs();
    }
}

/// Distance from `point` to the ray `(origin, dir)`, with the projection
/// clamped behind the origin (a point behind the ship measures to the origin
/// rather than to the ray's backward extension).
fn ray_distance(origin: Vec3, dir: Vec3, point: Vec3) -> f32 {
    let to_point = point - origin;
    let along = to_point.dot(dir).max(0.0);
    (to_point - dir * along).length()
}

/// Snap selection with hysteresis: the nearest candidate wins, unless an
/// incumbent is still selected and the challenger is not decisively closer
/// (below [`SNAP_HYSTERESIS`] of the incumbent's distance).
fn snap_pick(current: Option<Entity>, candidates: &[(Entity, f32)]) -> Option<Entity> {
    let (best, best_distance) = candidates
        .iter()
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .copied()?;
    if let Some(current) = current {
        if let Some((_, current_distance)) =
            candidates.iter().find(|(entity, _)| *entity == current)
        {
            if best_distance >= SNAP_HYSTERESIS * current_distance {
                return Some(current);
            }
        }
    }
    Some(best)
}

/// Stable cycle order for a ship's sections: nose-to-tail by local build
/// position (z, then x, then y), so repeated presses walk the hull the same
/// way every time regardless of query iteration order.
fn cycle_order(sections: &mut [(Entity, Vec3)]) {
    sections.sort_by(|(_, a), (_, b)| {
        a.z.total_cmp(&b.z)
            .then(a.x.total_cmp(&b.x))
            .then(a.y.total_cmp(&b.y))
    });
}

/// Maintain the component fine-lock: valid only while focused on the locked
/// ship and while the section stays attached; a pin expires by deadline or
/// with its section; snap follows the crosshair ray otherwise.
#[allow(clippy::type_complexity)]
fn update_component_lock(
    time: Res<Time>,
    lock: Res<SpaceshipPlayerTargetLock>,
    focus: Res<SpaceshipPlayerLockFocus>,
    mut component: ResMut<SpaceshipPlayerComponentLock>,
    q_sections: Query<(Entity, &ChildOf, &GlobalTransform), With<SectionMarker>>,
    point_rotation: Option<
        Single<
            &PointRotationOutput,
            (
                With<SpaceshipCameraInputMarker>,
                With<SpaceshipCameraTurretInputMarker>,
            ),
        >,
    >,
    spaceship: Option<
        Single<
            (&Transform, Option<&ComputedCenterOfMass>),
            (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
        >,
    >,
) {
    // The component layer only exists while focused on the current lock.
    let target = match **lock {
        Some(target) if focus.focused_on(target) => target,
        _ => {
            component.set_if_neq(SpaceshipPlayerComponentLock::default());
            return;
        }
    };

    let sections: Vec<(Entity, Vec3)> = q_sections
        .iter()
        .filter(|(_, ChildOf(parent), _)| *parent == target)
        .map(|(entity, _, transform)| (entity, transform.translation()))
        .collect();
    if sections.is_empty() {
        component.set_if_neq(SpaceshipPlayerComponentLock::default());
        return;
    }

    // Detach/despawn invalidates the selection (inactive sections stay
    // lockable - see SpaceshipPlayerComponentLock).
    let current = component
        .section
        .filter(|section| sections.iter().any(|(entity, _)| entity == section));
    if component.section != current {
        component.section = current;
    }

    // A pin outlives neither its deadline nor its section.
    if let ComponentLockMode::Pinned { until } = component.mode {
        if component.section.is_none() || time.elapsed_secs() >= until {
            component.mode = ComponentLockMode::Snap;
        }
    }

    if component.mode != ComponentLockMode::Snap {
        return;
    }
    let (Some(point_rotation), Some(spaceship)) = (point_rotation, spaceship) else {
        // No aim rig (menu states, headless tests): hold the current
        // selection rather than guessing.
        return;
    };
    let (ship_transform, ship_com) = spaceship.into_inner();
    let origin = live_structure_anchor(ship_transform, ship_com);
    let dir = (***point_rotation * Vec3::NEG_Z).normalize();
    let candidates: Vec<(Entity, f32)> = sections
        .iter()
        .map(|&(entity, position)| (entity, ray_distance(origin, dir, position)))
        .collect();
    let picked = snap_pick(component.section, &candidates);
    if component.section != picked {
        component.section = picked;
    }
}

/// Shared body of the cycle observers: step the fine lock through the locked
/// ship's attached sections in [`cycle_order`] and pin the choice for
/// [`COMPONENT_PIN_WINDOW`] seconds.
fn step_component_lock(
    direction: isize,
    time: &Time,
    lock: &SpaceshipPlayerTargetLock,
    focus: &SpaceshipPlayerLockFocus,
    component: &mut SpaceshipPlayerComponentLock,
    q_sections: &Query<(Entity, &ChildOf, &Transform), With<SectionMarker>>,
) {
    let target = match **lock {
        Some(target) if focus.focused_on(target) => target,
        _ => return,
    };

    let mut order: Vec<(Entity, Vec3)> = q_sections
        .iter()
        .filter(|(_, ChildOf(parent), _)| *parent == target)
        .map(|(entity, _, transform)| (entity, transform.translation))
        .collect();
    if order.is_empty() {
        return;
    }
    cycle_order(&mut order);

    let len = order.len() as isize;
    let index = component
        .section
        .and_then(|section| order.iter().position(|(entity, _)| *entity == section));
    let next = match index {
        Some(index) => (index as isize + direction).rem_euclid(len) as usize,
        // First press with no selection: next starts at the nose, prev at
        // the tail.
        None if direction >= 0 => 0,
        None => (len - 1) as usize,
    };

    component.section = Some(order[next].0);
    component.mode = ComponentLockMode::Pinned {
        until: time.elapsed_secs() + COMPONENT_PIN_WINDOW,
    };
}

fn on_component_cycle_next(
    _: On<Start<ComponentCycleNextInput>>,
    time: Res<Time>,
    lock: Res<SpaceshipPlayerTargetLock>,
    focus: Res<SpaceshipPlayerLockFocus>,
    mut component: ResMut<SpaceshipPlayerComponentLock>,
    q_sections: Query<(Entity, &ChildOf, &Transform), With<SectionMarker>>,
) {
    step_component_lock(1, &time, &lock, &focus, &mut component, &q_sections);
}

fn on_component_cycle_prev(
    _: On<Start<ComponentCyclePrevInput>>,
    time: Res<Time>,
    lock: Res<SpaceshipPlayerTargetLock>,
    focus: Res<SpaceshipPlayerLockFocus>,
    mut component: ResMut<SpaceshipPlayerComponentLock>,
    q_sections: Query<(Entity, &ChildOf, &Transform), With<SectionMarker>>,
) {
    step_component_lock(-1, &time, &lock, &focus, &mut component, &q_sections);
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

    #[test]
    fn empty_cone_never_auto_acquires_a_neutral_ship() {
        // An explicitly Neutral ship close by is a bystander, not a threat:
        // the signature fallback must leave it alone (task 20260708-203708).
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        world.spawn((
            SpaceshipRootMarker,
            Allegiance::Neutral,
            RigidBody::Dynamic,
            GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
        ));

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();

        assert_eq!(**world.resource::<SpaceshipPlayerTargetLock>(), None);
    }

    // -- focus dwell + component fine-lock --

    use std::time::Duration;

    /// A focused world: aim rig on -Z, player at the origin, a locked target
    /// ship with three sections - one dead on the aim ray, two off to the
    /// side - and the focus dwell already complete.
    fn focused_world() -> (World, Entity, [Entity; 3]) {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
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
        let target = world.spawn(SpaceshipRootMarker).id();
        // Local build order (z) deliberately different from spawn order, so
        // the cycle-order sort is actually exercised.
        let on_ray = world
            .spawn((
                SectionMarker,
                Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -100.0)),
                ChildOf(target),
            ))
            .id();
        let near_ray = world
            .spawn((
                SectionMarker,
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                GlobalTransform::from_translation(Vec3::new(5.0, 0.0, -100.0)),
                ChildOf(target),
            ))
            .id();
        let far_ray = world
            .spawn((
                SectionMarker,
                Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
                GlobalTransform::from_translation(Vec3::new(10.0, 0.0, -100.0)),
                ChildOf(target),
            ))
            .id();
        world.insert_resource(SpaceshipPlayerTargetLock(Some(target)));
        world.insert_resource(SpaceshipPlayerLockFocus {
            target: Some(target),
            seconds: FOCUS_TIME,
        });
        world.insert_resource(SpaceshipPlayerComponentLock::default());
        (world, target, [on_ray, near_ray, far_ray])
    }

    fn cycle(world: &mut World, direction: isize) {
        world
            .run_system_once(
                move |time: Res<Time>,
                      lock: Res<SpaceshipPlayerTargetLock>,
                      focus: Res<SpaceshipPlayerLockFocus>,
                      mut component: ResMut<SpaceshipPlayerComponentLock>,
                      q_sections: Query<(Entity, &ChildOf, &Transform), With<SectionMarker>>| {
                    step_component_lock(
                        direction,
                        &time,
                        &lock,
                        &focus,
                        &mut component,
                        &q_sections,
                    );
                },
            )
            .unwrap();
    }

    #[test]
    fn focus_accumulates_and_resets_on_lock_change() {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        world.insert_resource(SpaceshipPlayerTargetLock(Some(a)));
        world.insert_resource(SpaceshipPlayerLockFocus::default());

        // First tick registers the new target (reset), then time accrues.
        world.run_system_once(tick_lock_focus).unwrap();
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1.0));
        world.run_system_once(tick_lock_focus).unwrap();
        let focus = world.resource::<SpaceshipPlayerLockFocus>();
        assert_eq!(focus.target, Some(a));
        assert!((focus.seconds - 1.0).abs() < 1e-6);
        assert!(!focus.focused_on(a));

        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(0.6));
        world.run_system_once(tick_lock_focus).unwrap();
        assert!(world.resource::<SpaceshipPlayerLockFocus>().focused_on(a));

        // Switching targets restarts the dwell.
        world.insert_resource(SpaceshipPlayerTargetLock(Some(b)));
        world.run_system_once(tick_lock_focus).unwrap();
        let focus = world.resource::<SpaceshipPlayerLockFocus>();
        assert_eq!(focus.target, Some(b));
        assert_eq!(focus.seconds, 0.0);
    }

    #[test]
    fn ray_distance_measures_perpendicular_and_clamps_behind() {
        let origin = Vec3::ZERO;
        let dir = Vec3::NEG_Z;
        assert!((ray_distance(origin, dir, Vec3::new(3.0, 4.0, -10.0)) - 5.0).abs() < 1e-6);
        // Behind the origin: distance to the origin itself, not the
        // backward extension.
        assert!((ray_distance(origin, dir, Vec3::new(0.0, 0.0, 7.0)) - 7.0).abs() < 1e-6);
    }

    #[test]
    fn snap_pick_applies_hysteresis() {
        let a = Entity::from_raw_u32(1).unwrap();
        let b = Entity::from_raw_u32(2).unwrap();
        assert_eq!(snap_pick(None, &[]), None);
        // No incumbent: nearest wins.
        assert_eq!(snap_pick(None, &[(a, 5.0), (b, 3.0)]), Some(b));
        // Challenger at 0.8x of the incumbent: not decisive, keep a.
        assert_eq!(snap_pick(Some(a), &[(a, 5.0), (b, 4.0)]), Some(a));
        // Challenger well under the hysteresis fraction: switch.
        assert_eq!(snap_pick(Some(a), &[(a, 5.0), (b, 2.0)]), Some(b));
        // Dead incumbent (not in candidates): nearest wins.
        assert_eq!(snap_pick(Some(a), &[(b, 9.0)]), Some(b));
    }

    #[test]
    fn component_lock_requires_focus() {
        let (mut world, _, [on_ray, _, _]) = focused_world();
        world.resource_mut::<SpaceshipPlayerComponentLock>().section = Some(on_ray);
        world.resource_mut::<SpaceshipPlayerLockFocus>().seconds = 0.5;

        world.run_system_once(update_component_lock).unwrap();

        assert_eq!(
            *world.resource::<SpaceshipPlayerComponentLock>(),
            SpaceshipPlayerComponentLock::default(),
            "an incomplete dwell clears the fine lock"
        );
    }

    #[test]
    fn snap_selects_the_section_nearest_the_aim_ray() {
        let (mut world, _, [on_ray, _, _]) = focused_world();

        world.run_system_once(update_component_lock).unwrap();

        assert_eq!(
            world.resource::<SpaceshipPlayerComponentLock>().section,
            Some(on_ray)
        );
    }

    #[test]
    fn cycle_steps_the_stable_order_and_pins() {
        let (mut world, _, [on_ray, near_ray, far_ray]) = focused_world();

        // Stable order is by local z: near_ray (0), on_ray (1), far_ray (2).
        cycle(&mut world, 1);
        let component = world.resource::<SpaceshipPlayerComponentLock>();
        assert_eq!(component.section, Some(near_ray));
        assert!(matches!(component.mode, ComponentLockMode::Pinned { .. }));

        cycle(&mut world, 1);
        assert_eq!(
            world.resource::<SpaceshipPlayerComponentLock>().section,
            Some(on_ray)
        );
        cycle(&mut world, 1);
        assert_eq!(
            world.resource::<SpaceshipPlayerComponentLock>().section,
            Some(far_ray)
        );
        // Wraps.
        cycle(&mut world, 1);
        assert_eq!(
            world.resource::<SpaceshipPlayerComponentLock>().section,
            Some(near_ray)
        );
        // Prev from a fresh (unselected) state starts at the tail.
        world.insert_resource(SpaceshipPlayerComponentLock::default());
        cycle(&mut world, -1);
        assert_eq!(
            world.resource::<SpaceshipPlayerComponentLock>().section,
            Some(far_ray)
        );
    }

    #[test]
    fn cycle_is_a_no_op_before_the_dwell_completes() {
        let (mut world, _, _) = focused_world();
        world.resource_mut::<SpaceshipPlayerLockFocus>().seconds = 0.5;

        cycle(&mut world, 1);

        assert_eq!(
            *world.resource::<SpaceshipPlayerComponentLock>(),
            SpaceshipPlayerComponentLock::default(),
            "cycling before focus completes must not select anything"
        );
    }

    #[test]
    fn pin_expires_back_to_snap() {
        let (mut world, _, [on_ray, near_ray, _]) = focused_world();
        cycle(&mut world, 1);
        assert_eq!(
            world.resource::<SpaceshipPlayerComponentLock>().section,
            Some(near_ray),
            "pinned to the first section in cycle order"
        );

        // The pin holds against snap while its window is open...
        world.run_system_once(update_component_lock).unwrap();
        assert_eq!(
            world.resource::<SpaceshipPlayerComponentLock>().section,
            Some(near_ray)
        );

        // ...and expires after COMPONENT_PIN_WINDOW, letting snap retake.
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(COMPONENT_PIN_WINDOW + 0.5));
        world.run_system_once(update_component_lock).unwrap();
        let component = world.resource::<SpaceshipPlayerComponentLock>();
        assert_eq!(component.mode, ComponentLockMode::Snap);
        assert_eq!(component.section, Some(on_ray));
    }

    #[test]
    fn pinned_section_death_reverts_to_snap() {
        let (mut world, _, [on_ray, near_ray, _]) = focused_world();
        cycle(&mut world, 1);
        assert_eq!(
            world.resource::<SpaceshipPlayerComponentLock>().section,
            Some(near_ray)
        );

        world.despawn(near_ray);
        world.run_system_once(update_component_lock).unwrap();

        let component = world.resource::<SpaceshipPlayerComponentLock>();
        assert_eq!(component.mode, ComponentLockMode::Snap);
        assert_eq!(
            component.section,
            Some(on_ray),
            "the dead pin falls back to the ray-nearest section"
        );
    }

    #[test]
    fn lock_loss_clears_the_component_lock() {
        let (mut world, _, [on_ray, _, _]) = focused_world();
        world.resource_mut::<SpaceshipPlayerComponentLock>().section = Some(on_ray);

        world.insert_resource(SpaceshipPlayerTargetLock(None));
        world.run_system_once(update_component_lock).unwrap();

        assert_eq!(
            *world.resource::<SpaceshipPlayerComponentLock>(),
            SpaceshipPlayerComponentLock::default()
        );
    }
}
