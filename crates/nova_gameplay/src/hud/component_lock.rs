//! Component-lock markers: one small screen-projected marker per attached
//! section of the locked ship, visible only while the focus dwell is
//! complete, with the fine-locked section highlighted
//! (task 20260709-192523; mechanic in input/targeting.rs, design in
//! docs/spikes/20260709-192358-component-lock-vats-lite.md).
//!
//! A thin consumer of the [`screen_indicator`](mod@super::screen_indicator)
//! widget with `Entity` anchors on section entities: a reconcile system
//! keeps marker membership in sync with the locked ship's attached sections
//! (they die mid-fight), and a highlight system restyles the selected one.
//! The layer spawns/despawns with the player ship via the hud/mod.rs
//! observers, like every other overlay.

use bevy::prelude::*;

use crate::prelude::*;

/// On-screen size (px) of an unselected section marker. Small and dim: the
/// markers are an overlay on the silhouette, not reticles.
const MARKER_PX: f32 = 10.0;

/// On-screen size (px) of the fine-locked section's marker.
const MARKER_SELECTED_PX: f32 = 16.0;

/// Unselected marker tint: dim hot-metal red, distinct from the amber lead
/// pip, the nav-cyan destination marker and the untinted lock reticle.
const MARKER_COLOR: Color = Color::srgba(1.0, 0.3, 0.2, 0.55);

/// Selected marker tint: the same hue at full presence.
const MARKER_SELECTED_COLOR: Color = Color::srgba(1.0, 0.45, 0.3, 0.95);

pub mod prelude {
    pub use super::{
        component_lock_hud, ComponentLockHudMarker, ComponentLockHudPlugin,
        ComponentLockSectionMarker, ComponentLockSectionTarget,
    };
}

/// Marker for the full-screen component-marker layer (the root the HUD setup
/// spawns).
#[derive(Component, Debug, Clone, Reflect)]
pub struct ComponentLockHudMarker;

/// Marker for one section marker node.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ComponentLockSectionMarker;

/// The section entity this marker overlays.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct ComponentLockSectionTarget(pub Entity);

/// UI bundle for the marker layer. Markers are spawned under it by
/// `sync_component_markers`, one per attached section of the locked ship,
/// while focused.
pub fn component_lock_hud() -> impl Bundle {
    (
        Name::new("ComponentLockHUD"),
        ComponentLockHudMarker,
        screen_indicator_layer(),
    )
}

/// Bundle for a single section marker: a small tinted square indicator
/// entity-anchored to the section, so the widget tracks and hides it for
/// free.
fn component_marker(section: Entity) -> impl Bundle {
    (
        Name::new("ComponentLockMarker"),
        ComponentLockSectionMarker,
        ComponentLockSectionTarget(section),
        screen_indicator(ScreenIndicatorConfig {
            anchor: Some(ScreenIndicatorAnchorKind::Entity(section)),
            size: ScreenIndicatorSize::Fixed(Vec2::splat(MARKER_PX)),
            offset: Vec2::ZERO,
            offscreen: ScreenIndicatorOffscreen::Hide,
        }),
        BackgroundColor(MARKER_COLOR),
    )
}

/// Drives the per-section component-lock markers over the locked ship's
/// silhouette (the VATS-lite overlay).
/// Adds `sync_component_markers` (reconcile marker membership) then
/// `highlight_selected_marker` (restyle the fine-locked section), chained in
/// Update within [`super::NovaHudSystems`].
#[derive(Default)]
pub struct ComponentLockHudPlugin;

impl Plugin for ComponentLockHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("ComponentLockHudPlugin: build");

        app.add_systems(
            Update,
            (sync_component_markers, highlight_selected_marker)
                .chain()
                .in_set(super::NovaHudSystems),
        );
    }
}

/// Keep exactly one marker per attached section of the locked ship while the
/// focus dwell is complete, and none otherwise. A reconcile system, like the
/// turret pips: sections die mid-fight and the lock/focus state changes
/// freely, and one idempotent pass covers every ordering.
#[allow(clippy::type_complexity)]
fn sync_component_markers(
    mut commands: Commands,
    q_ship: Query<(&CombatLock, &LockFocus), With<PlayerSpaceshipMarker>>,
    q_layer: Query<Entity, With<ComponentLockHudMarker>>,
    q_sections: Query<(Entity, &ChildOf), With<SectionMarker>>,
    q_markers: Query<(Entity, &ComponentLockSectionTarget), With<ComponentLockSectionMarker>>,
) {
    let Ok(layer) = q_layer.single() else {
        // No layer means no player HUD; its despawn removed the markers too.
        return;
    };

    // The marker set exists only while focused on the current COMBAT lock.
    let target = q_ship.iter().next().and_then(|(lock, focus)| match lock.0 {
        Some(target) if focus.focused_on(target) => Some(target),
        _ => None,
    });

    // Despawn markers whose section died, left the ship, or whose focus
    // window closed.
    for (marker, section) in &q_markers {
        let keep = target.is_some_and(|target| {
            q_sections
                .get(**section)
                .is_ok_and(|(_, ChildOf(parent))| *parent == target)
        });
        if !keep {
            commands.entity(marker).despawn();
        }
    }

    // Spawn markers for sections that have none yet.
    let Some(target) = target else {
        return;
    };
    for (section, ChildOf(parent)) in &q_sections {
        if *parent != target {
            continue;
        }
        let has_marker = q_markers.iter().any(|(_, marked)| **marked == section);
        if !has_marker {
            commands.entity(layer).with_child(component_marker(section));
        }
    }
}

/// Restyle the fine-locked section's marker: bigger and brighter than its
/// siblings, reverted when the selection moves on.
fn highlight_selected_marker(
    q_ship: Query<&ComponentLock, With<PlayerSpaceshipMarker>>,
    mut q_markers: Query<
        (
            &ComponentLockSectionTarget,
            &mut ScreenIndicatorSize,
            &mut BackgroundColor,
        ),
        With<ComponentLockSectionMarker>,
    >,
) {
    let selected_section = q_ship.iter().next().and_then(|component| component.section);
    for (section, mut size, mut color) in &mut q_markers {
        let selected = selected_section == Some(**section);
        let (want_px, want_color) = if selected {
            (MARKER_SELECTED_PX, MARKER_SELECTED_COLOR)
        } else {
            (MARKER_PX, MARKER_COLOR)
        };
        let want_size = ScreenIndicatorSize::Fixed(Vec2::splat(want_px));
        if *size != want_size {
            *size = want_size;
        }
        if color.0 != want_color {
            color.0 = want_color;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// Layer + a player ship combat-locked and focused on a target ship with
    /// two sections. Returns (world, player, [sections]).
    fn focused_world() -> (World, Entity, [Entity; 2]) {
        let mut world = World::new();
        world.spawn(component_lock_hud());
        let target = world.spawn(SpaceshipRootMarker).id();
        let a = world.spawn((SectionMarker, ChildOf(target))).id();
        let b = world.spawn((SectionMarker, ChildOf(target))).id();
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                CombatLock(Some(target)),
                LockFocus {
                    target: Some(target),
                    seconds: f32::MAX,
                },
                ComponentLock::default(),
            ))
            .id();
        (world, player, [a, b])
    }

    fn marker_sections(world: &mut World) -> Vec<Entity> {
        let mut sections: Vec<Entity> = world
            .query_filtered::<&ComponentLockSectionTarget, With<ComponentLockSectionMarker>>()
            .iter(world)
            .map(|section| **section)
            .collect();
        sections.sort();
        sections
    }

    #[test]
    fn markers_exist_only_while_focused() {
        let (mut world, player, [a, b]) = focused_world();

        world.run_system_once(sync_component_markers).unwrap();
        let mut expected = vec![a, b];
        expected.sort();
        assert_eq!(marker_sections(&mut world), expected);

        // Losing focus removes every marker.
        world.get_mut::<LockFocus>(player).unwrap().seconds = 0.0;
        world.run_system_once(sync_component_markers).unwrap();
        assert!(marker_sections(&mut world).is_empty());
    }

    #[test]
    fn markers_follow_section_death() {
        let (mut world, _, [a, b]) = focused_world();
        world.run_system_once(sync_component_markers).unwrap();

        world.despawn(a);
        world.run_system_once(sync_component_markers).unwrap();

        assert_eq!(marker_sections(&mut world), vec![b]);
    }

    #[test]
    fn markers_clear_on_lock_change() {
        let (mut world, player, _) = focused_world();
        world.run_system_once(sync_component_markers).unwrap();
        assert_eq!(marker_sections(&mut world).len(), 2);

        // A new lock without a completed dwell shows nothing.
        let other = world.spawn(SpaceshipRootMarker).id();
        world.get_mut::<CombatLock>(player).unwrap().0 = Some(other);
        world.run_system_once(sync_component_markers).unwrap();

        assert!(marker_sections(&mut world).is_empty());
    }

    #[test]
    fn highlight_follows_the_component_lock() {
        let (mut world, player, [a, b]) = focused_world();
        world.run_system_once(sync_component_markers).unwrap();
        world.get_mut::<ComponentLock>(player).unwrap().section = Some(a);

        world.run_system_once(highlight_selected_marker).unwrap();

        let styles: Vec<(Entity, ScreenIndicatorSize, Color)> = world
            .query_filtered::<(
                &ComponentLockSectionTarget,
                &ScreenIndicatorSize,
                &BackgroundColor,
            ), With<ComponentLockSectionMarker>>()
            .iter(&world)
            .map(|(section, size, color)| (**section, *size, color.0))
            .collect();
        for (section, size, color) in styles {
            if section == a {
                assert_eq!(
                    size,
                    ScreenIndicatorSize::Fixed(Vec2::splat(MARKER_SELECTED_PX))
                );
                assert_eq!(color, MARKER_SELECTED_COLOR);
            } else {
                assert_eq!(section, b);
                assert_eq!(size, ScreenIndicatorSize::Fixed(Vec2::splat(MARKER_PX)));
                assert_eq!(color, MARKER_COLOR);
            }
        }

        // Selection moves on: the old highlight reverts.
        world.get_mut::<ComponentLock>(player).unwrap().section = Some(b);
        world.run_system_once(highlight_selected_marker).unwrap();
        let (size, color) = world
            .query_filtered::<(
                &ComponentLockSectionTarget,
                &ScreenIndicatorSize,
                &BackgroundColor,
            ), With<ComponentLockSectionMarker>>()
            .iter(&world)
            .find(|(section, _, _)| ***section == a)
            .map(|(_, size, color)| (*size, color.0))
            .expect("marker for a exists");
        assert_eq!(size, ScreenIndicatorSize::Fixed(Vec2::splat(MARKER_PX)));
        assert_eq!(color, MARKER_COLOR);
    }
}
