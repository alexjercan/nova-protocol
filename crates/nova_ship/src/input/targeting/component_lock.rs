//! The component fine-lock: which section of the combat-locked ship the
//! weapons aim at, snapped to the crosshair or pinned by the cycle keys.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_gameplay::prelude::*;

#[cfg(test)]
use super::contacts::FOCUS_TIME;
use crate::prelude::*;

/// Seconds a cycle press pins the component selection before aim-snap
/// resumes. A feel knob; tune in playtest.
const COMPONENT_PIN_WINDOW: f32 = 2.0;

/// Snap hysteresis: a challenger section only steals the fine lock when its
/// ray distance is below this fraction of the incumbent's, so the selection
/// does not flicker between adjacent sections. A feel knob; tune in playtest.
const SNAP_HYSTERESIS: f32 = 0.75;

/// How the component fine-lock is currently selected.
#[derive(Debug, Clone, Copy, PartialEq, Default, Reflect)]
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

/// The fine-locked section of the combat-locked ship, only ever `Some` while
/// the focus dwell is complete. Sections stay lockable while ATTACHED - a
/// disabled-in-place section (`SectionInactiveMarker`) can still be targeted
/// to blow it off the hull; despawn/detach clears the selection (decision
/// from the component-lock spike, lockable-while-attached). On the player
/// ship root.
#[derive(Component, Debug, Clone, PartialEq, Default, Reflect)]
#[reflect(Component)]
pub struct ComponentLock {
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
#[expect(
    clippy::type_complexity,
    reason = "one query term per component-lock input"
)]
pub(super) fn update_component_lock(
    time: Res<Time>,
    q_sections: Query<(Entity, &ChildOf, &GlobalTransform), With<SectionMarker>>,
    // The LIVE look ray (active rig), so the snap follows the crosshair in
    // every view instead of the turret rig's frozen output.
    look_ray: ActiveLookRay,
    mut q_ship: Query<
        (
            &Transform,
            Option<&ComputedCenterOfMass>,
            &CombatLock,
            &LockFocus,
            &mut ComponentLock,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    for (ship_transform, ship_com, lock, focus, mut component) in &mut q_ship {
        // The component layer only exists while focused on the combat lock.
        let target = match lock.0 {
            Some(target) if focus.focused_on(target) => target,
            _ => {
                component.set_if_neq(ComponentLock::default());
                continue;
            }
        };

        let sections: Vec<(Entity, Vec3)> = q_sections
            .iter()
            .filter(|(_, ChildOf(parent), _)| *parent == target)
            .map(|(entity, _, transform)| (entity, transform.translation()))
            .collect();
        if sections.is_empty() {
            component.set_if_neq(ComponentLock::default());
            continue;
        }

        // Detach/despawn invalidates the selection (inactive sections stay
        // lockable - see ComponentLock).
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
            continue;
        }
        let Some(aim_rotation) = look_ray.rotation() else {
            // No aim rig (menu states, headless tests): hold the current
            // selection rather than guessing.
            continue;
        };
        let origin = live_structure_anchor(ship_transform, ship_com);
        let dir = (aim_rotation * Vec3::NEG_Z).normalize();
        let candidates: Vec<(Entity, f32)> = sections
            .iter()
            .map(|&(entity, position)| (entity, ray_distance(origin, dir, position)))
            .collect();
        let picked = snap_pick(component.section, &candidates);
        if component.section != picked {
            component.section = picked;
        }
    }
}

/// Shared body of the cycle observers: step the fine lock through the locked
/// ship's attached sections in [`cycle_order`] and pin the choice for
/// [`COMPONENT_PIN_WINDOW`] seconds.
fn step_component_lock(
    direction: isize,
    time: &Time,
    lock: &CombatLock,
    focus: &LockFocus,
    component: &mut ComponentLock,
    q_sections: &Query<(Entity, &ChildOf, &Transform), With<SectionMarker>>,
) {
    let target = match lock.0 {
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

/// One scroll notch's step on the RCS vertical (Y, up/down) axis while RCS
/// fine-adjust is held. A discrete nudge that the per-tick decay then bleeds
/// off, so each notch is a transient burst rather than a persistent offset.
/// Feel-tunable (raised 0.25 -> 0.75 in so scroll bites noticeably harder
/// than the mouse).
const RCS_SCROLL_STEP: f32 = 0.75;

pub(crate) fn on_component_cycle_next(
    _: On<Start<ComponentCycleNextInput>>,
    time: Res<Time>,
    q_sections: Query<(Entity, &ChildOf, &Transform), With<SectionMarker>>,
    mut q_ship: Query<
        (
            &CombatLock,
            &LockFocus,
            &mut ComponentLock,
            Has<RcsActive>,
            Option<&mut RcsIntent>,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    pause: Res<State<nova_gameplay::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up. Releases stay ungated so held keys clear cleanly
    // during a pause.
    if pause.get().is_frozen() {
        return;
    }
    for (lock, focus, mut component, rcs_active, rcs_intent) in &mut q_ship {
        // While RCS is held the wheel drives the RCS vertical axis (up = +Y)
        // instead of stepping the component lock - the same "modifier decides"
        // rule the CTRL layer uses for the ship lock (player.rs).
        if rcs_active {
            if let Some(mut intent) = rcs_intent {
                intent.y = crate::flight::accumulate_rcs_axis(intent.y, RCS_SCROLL_STEP);
            }
            continue;
        }
        step_component_lock(1, &time, lock, focus, &mut component, &q_sections);
    }
}

pub(super) fn on_component_cycle_prev(
    _: On<Start<ComponentCyclePrevInput>>,
    time: Res<Time>,
    q_sections: Query<(Entity, &ChildOf, &Transform), With<SectionMarker>>,
    mut q_ship: Query<
        (
            &CombatLock,
            &LockFocus,
            &mut ComponentLock,
            Has<RcsActive>,
            Option<&mut RcsIntent>,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    pause: Res<State<nova_gameplay::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up. Releases stay ungated so held keys clear cleanly
    // during a pause.
    if pause.get().is_frozen() {
        return;
    }
    for (lock, focus, mut component, rcs_active, rcs_intent) in &mut q_ship {
        if rcs_active {
            if let Some(mut intent) = rcs_intent {
                intent.y = crate::flight::accumulate_rcs_axis(intent.y, -RCS_SCROLL_STEP);
            }
            continue;
        }
        step_component_lock(-1, &time, lock, focus, &mut component, &q_sections);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn ray_distance_measures_perpendicular_and_clamps_behind() {
        let origin = Vec3::ZERO;
        let dir = Vec3::NEG_Z;
        assert!((ray_distance(origin, dir, Vec3::new(3.0, 4.0, -10.0)) - 5.0).abs() < 1e-6);
        assert!((ray_distance(origin, dir, Vec3::new(0.0, 0.0, 7.0)) - 7.0).abs() < 1e-6);
    }

    #[test]
    fn snap_pick_applies_hysteresis() {
        let a = Entity::from_raw_u32(1).unwrap();
        let b = Entity::from_raw_u32(2).unwrap();
        assert_eq!(snap_pick(None, &[]), None);
        assert_eq!(snap_pick(None, &[(a, 5.0), (b, 3.0)]), Some(b));
        assert_eq!(snap_pick(Some(a), &[(a, 5.0), (b, 4.0)]), Some(a));
        assert_eq!(snap_pick(Some(a), &[(a, 5.0), (b, 1.0)]), Some(b));
    }

    /// A player combat-locked and focused on a target ship with three
    /// sections (one dead on the -Z aim ray, two off to the side), faithful
    /// split rigs. Returns (world, player, [on_ray, near_ray, far_ray]).
    fn focused_world() -> (World, Entity, [Entity; 3]) {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        world.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraNormalInputMarker,
            SpaceshipRotationInputActiveMarker,
            PointRotationOutput(Quat::IDENTITY),
        ));
        world.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraTurretInputMarker,
            PointRotationOutput(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        ));
        let target = world.spawn(SpaceshipRootMarker).id();
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
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::IDENTITY,
                CombatLock(Some(target)),
                LockFocus {
                    target: Some(target),
                    seconds: FOCUS_TIME,
                },
                ComponentLock::default(),
            ))
            .id();
        (world, player, [on_ray, near_ray, far_ray])
    }

    fn cycle(world: &mut World, direction: isize) {
        world
            .run_system_once(
                move |time: Res<Time>,
                      q_sections: Query<(Entity, &ChildOf, &Transform), With<SectionMarker>>,
                      mut q_ship: Query<(&CombatLock, &LockFocus, &mut ComponentLock)>| {
                    for (lock, focus, mut component) in &mut q_ship {
                        step_component_lock(
                            direction,
                            &time,
                            lock,
                            focus,
                            &mut component,
                            &q_sections,
                        );
                    }
                },
            )
            .unwrap();
    }

    fn selected(world: &mut World, player: Entity) -> Option<Entity> {
        world.get::<ComponentLock>(player).unwrap().section
    }

    #[test]
    fn snap_selects_the_section_nearest_the_aim_ray() {
        let (mut world, player, [on_ray, ..]) = focused_world();
        world.run_system_once(update_component_lock).unwrap();
        assert_eq!(selected(&mut world, player), Some(on_ray));
    }

    #[test]
    fn component_lock_requires_focus() {
        let (mut world, player, _) = focused_world();
        world.get_mut::<LockFocus>(player).unwrap().seconds = 0.0;
        world.run_system_once(update_component_lock).unwrap();
        assert_eq!(selected(&mut world, player), None);
    }

    #[test]
    fn lock_loss_clears_the_component_lock() {
        let (mut world, player, [on_ray, ..]) = focused_world();
        world.run_system_once(update_component_lock).unwrap();
        assert_eq!(selected(&mut world, player), Some(on_ray));

        world.get_mut::<CombatLock>(player).unwrap().0 = None;
        world.run_system_once(update_component_lock).unwrap();
        assert_eq!(selected(&mut world, player), None);
    }

    #[test]
    fn cycle_steps_the_stable_order_and_pins() {
        let (mut world, player, [on_ray, near_ray, far_ray]) = focused_world();

        // Local build order by z: near_ray (0), on_ray (1), far_ray (2).
        cycle(&mut world, 1);
        assert_eq!(selected(&mut world, player), Some(near_ray));
        cycle(&mut world, 1);
        assert_eq!(selected(&mut world, player), Some(on_ray));
        cycle(&mut world, 1);
        assert_eq!(selected(&mut world, player), Some(far_ray));
        cycle(&mut world, 1);
        assert_eq!(selected(&mut world, player), Some(near_ray), "wraps");
        assert!(matches!(
            world.get::<ComponentLock>(player).unwrap().mode,
            ComponentLockMode::Pinned { .. }
        ));

        // Pinned: the snap must NOT move the selection off near_ray even
        // though on_ray sits on the aim ray.
        world.run_system_once(update_component_lock).unwrap();
        assert_eq!(selected(&mut world, player), Some(near_ray));
    }

    #[test]
    fn cycle_is_a_no_op_before_the_dwell_completes() {
        let (mut world, player, _) = focused_world();
        world.get_mut::<LockFocus>(player).unwrap().seconds = 0.0;
        cycle(&mut world, 1);
        assert_eq!(selected(&mut world, player), None);
    }

    #[test]
    fn pin_expires_back_to_snap() {
        let (mut world, player, [on_ray, near_ray, _]) = focused_world();
        cycle(&mut world, 1);
        assert_eq!(selected(&mut world, player), Some(near_ray));

        // Past the pin window the snap resumes and picks the on-ray section.
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(COMPONENT_PIN_WINDOW + 0.1));
        world.run_system_once(update_component_lock).unwrap();
        assert_eq!(selected(&mut world, player), Some(on_ray));
    }

    #[test]
    fn pinned_section_death_reverts_to_snap() {
        let (mut world, player, [on_ray, near_ray, _]) = focused_world();
        cycle(&mut world, 1);
        assert_eq!(selected(&mut world, player), Some(near_ray));

        world.despawn(near_ray);
        world.run_system_once(update_component_lock).unwrap();
        assert_eq!(selected(&mut world, player), Some(on_ray));
        assert!(matches!(
            world.get::<ComponentLock>(player).unwrap().mode,
            ComponentLockMode::Snap
        ));
    }
}
