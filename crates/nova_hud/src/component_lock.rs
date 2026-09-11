//! Component-lock markers: one small screen-projected marker per attached
//! section of the locked ship, visible only while the focus dwell is
//! complete, with the fine-locked section highlighted (mechanic in
//! input/targeting/component_lock.rs).
//!
//! A thin consumer of the [`screen_indicator`](mod@super::screen_indicator)
//! widget with `Entity` anchors on section entities: a reconcile system
//! keeps marker membership in sync with the locked ship's attached sections
//! (they die mid-fight), and a highlight system restyles the selected one.
//! The layer spawns/despawns with the player ship via the hud/mod.rs
//! observers, like every other overlay.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::theme::combat;

use crate::prelude::*;

/// Floor (px) on an unselected section marker, and its size on a section too
/// small or too distant to measure. Small and dim: the markers are an overlay
/// on the silhouette, not reticles.
const MARKER_PX: f32 = 10.0;

/// Floor (px) on the fine-locked section's marker.
const MARKER_SELECTED_PX: f32 = 16.0;

/// Fraction of a section's on-screen extent an unselected marker covers. A
/// carrier thruster block is five plates wide and a hull plate is one; a
/// marker that ignored that painted both the same dot.
const MARKER_EXTENT_SCALE: f32 = 0.5;

/// The same fraction for the fine-locked section. Held at
/// `MARKER_SELECTED_PX / MARKER_PX` of its siblings, so the selection reads
/// as the bigger marker whether the pair is tracking or floored.
const MARKER_SELECTED_EXTENT_SCALE: f32 = 0.8;

/// How many section markers the overlay paints at once. Every hand-flyable
/// hull fits under it whole (the gunship, the largest, has 53 sections); the
/// capital hulls do not, and a marker per plate buries the fine-lock
/// highlight under 2081 dots.
const MARKER_BUDGET: usize = 64;

/// Unselected marker tint: dim hot metal, distinct from the amber lead pip,
/// the nav-cyan destination marker and the untinted lock reticle.
const MARKER_COLOR: Color = combat::at(combat::HOT, 0.55);

/// Selected marker tint: the same hue at full presence, which is what the
/// comment here always claimed and the literal did not.
const MARKER_SELECTED_COLOR: Color = combat::at(combat::HOT, 0.95);

/// The `component_lock_hud` spawner, its section-target components and `ComponentLockHudPlugin`.
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

/// How a section marker is sized: it tracks the section's own on-screen
/// extent, floored so a distant or tiny section still gets a legible dot.
fn marker_size(selected: bool) -> ScreenIndicatorSize {
    let (min_px, scale) = if selected {
        (MARKER_SELECTED_PX, MARKER_SELECTED_EXTENT_SCALE)
    } else {
        (MARKER_PX, MARKER_EXTENT_SCALE)
    };
    ScreenIndicatorSize::ApparentSize { min_px, scale }
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
            size: marker_size(false),
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
        trace!("ComponentLockHudPlugin: build");

        app.add_systems(
            Update,
            (sync_component_markers, highlight_selected_marker)
                .chain()
                .in_set(super::NovaHudSystems),
        );
    }
}

/// Take at most `room` of `sections`, evenly strided. Sections spawn in
/// authored grid order, so a stride samples the whole silhouette where a
/// truncation would mark one end of the ship and leave the other bare.
fn strided(sections: &[Entity], room: usize) -> impl Iterator<Item = Entity> + '_ {
    let stride = if room == 0 {
        1
    } else {
        sections.len().div_ceil(room).max(1)
    };
    sections
        .iter()
        .copied()
        .step_by(stride)
        .take(if room == 0 { 0 } else { room })
}

/// Which of the locked ship's sections get a marker: the fine-locked one
/// always, then the sections that carry a system, then the hull, each
/// strided into whatever room [`MARKER_BUDGET`] has left.
fn marked_sections(sections: &[(Entity, SectionClass)], selected: Option<Entity>) -> Vec<Entity> {
    let selected =
        selected.filter(|selected| sections.iter().any(|(section, _)| section == selected));

    let mut systems = Vec::new();
    let mut hull = Vec::new();
    for (section, class) in sections {
        if Some(*section) == selected {
            continue;
        }
        if matches!(class, SectionClass::Hull) {
            hull.push(*section);
        } else {
            systems.push(*section);
        }
    }

    let mut kept: Vec<Entity> = selected.into_iter().collect();
    kept.extend(strided(&systems, MARKER_BUDGET.saturating_sub(kept.len())));
    kept.extend(strided(&hull, MARKER_BUDGET.saturating_sub(kept.len())));
    kept
}

/// Keep one marker per marked section of the locked ship while the focus
/// dwell is complete, and none otherwise. A reconcile system, like the turret
/// pips: sections die mid-fight and the lock/focus state changes freely, and
/// one idempotent pass covers every ordering.
fn sync_component_markers(
    mut commands: Commands,
    q_ship: Query<(&CombatLock, &LockFocus, &ComponentLock), With<PlayerSpaceshipMarker>>,
    q_layer: Query<Entity, With<ComponentLockHudMarker>>,
    q_sections: Query<(Entity, &ChildOf, &SectionClass), With<SectionMarker>>,
    q_markers: Query<(Entity, &ComponentLockSectionTarget), With<ComponentLockSectionMarker>>,
) {
    let Ok(layer) = q_layer.single() else {
        // No layer means no player HUD; its despawn removed the markers too.
        return;
    };

    // The marker set exists only while focused on the current COMBAT lock.
    let lock = q_ship.iter().next();
    let target = lock.and_then(|(lock, focus, _)| match lock.0 {
        Some(target) if focus.focused_on(target) => Some(target),
        _ => None,
    });

    let marked = target.map_or_else(Vec::new, |target| {
        let sections: Vec<(Entity, SectionClass)> = q_sections
            .iter()
            .filter(|(_, ChildOf(parent), _)| *parent == target)
            .map(|(section, _, class)| (section, *class))
            .collect();
        marked_sections(
            &sections,
            lock.and_then(|(_, _, component)| component.section),
        )
    });

    // Despawn markers whose section died, left the ship, lost its place in
    // the decluttered set, or whose focus window closed.
    for (marker, section) in &q_markers {
        if !marked.contains(&**section) {
            commands.entity(marker).despawn();
        }
    }

    // Spawn markers for marked sections that have none yet.
    for section in marked {
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
        let want_color = if selected {
            MARKER_SELECTED_COLOR
        } else {
            MARKER_COLOR
        };
        let want_size = marker_size(selected);
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
    /// two hull sections. Returns (world, player, [sections]).
    fn focused_world() -> (World, Entity, [Entity; 2]) {
        let mut world = World::new();
        world.spawn(component_lock_hud());
        let target = world.spawn(SpaceshipRootMarker).id();
        let a = world
            .spawn((SectionMarker, SectionClass::Hull, ChildOf(target)))
            .id();
        let b = world
            .spawn((SectionMarker, SectionClass::Hull, ChildOf(target)))
            .id();
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
                assert_eq!(size, marker_size(true));
                assert_eq!(color, MARKER_SELECTED_COLOR);
            } else {
                assert_eq!(section, b);
                assert_eq!(size, marker_size(false));
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
        assert_eq!(size, marker_size(false));
        assert_eq!(color, MARKER_COLOR);
    }

    #[test]
    fn a_marker_tracks_its_sections_on_screen_extent() {
        let unselected = marker_size(false);
        let selected = marker_size(true);

        let ScreenIndicatorSize::ApparentSize { min_px, scale } = unselected else {
            panic!("markers track their section, they are not fixed boxes");
        };
        assert_eq!(min_px, MARKER_PX);

        // The selection keeps the same lead over its siblings whether the
        // pair is tracking the hull or sitting on the floor.
        let ScreenIndicatorSize::ApparentSize {
            min_px: selected_min,
            scale: selected_scale,
        } = selected
        else {
            panic!("the fine-locked marker tracks its section too");
        };
        assert_eq!(selected_min, MARKER_SELECTED_PX);
        assert!((selected_scale / scale - selected_min / min_px).abs() < 1e-5);
    }

    #[test]
    fn a_capital_hull_is_decluttered_down_to_the_marker_budget() {
        let mut world = World::new();
        world.spawn(component_lock_hud());
        let target = world.spawn(SpaceshipRootMarker).id();
        let mut plates = Vec::new();
        for _ in 0..500 {
            plates.push(
                world
                    .spawn((SectionMarker, SectionClass::Hull, ChildOf(target)))
                    .id(),
            );
        }
        let thruster = world
            .spawn((SectionMarker, SectionClass::Thruster, ChildOf(target)))
            .id();
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            CombatLock(Some(target)),
            LockFocus {
                target: Some(target),
                seconds: f32::MAX,
            },
            ComponentLock::default(),
        ));

        world.run_system_once(sync_component_markers).unwrap();

        let marked = marker_sections(&mut world);
        assert_eq!(marked.len(), MARKER_BUDGET);
        assert!(marked.contains(&thruster), "systems outrank bare plates");

        // The kept plates spread over the ship instead of marking one end:
        // both the first tenth and the last tenth are represented.
        assert!(plates[..50].iter().any(|plate| marked.contains(plate)));
        assert!(plates[450..].iter().any(|plate| marked.contains(plate)));
    }

    #[test]
    fn the_fine_locked_section_always_keeps_its_marker() {
        let mut world = World::new();
        world.spawn(component_lock_hud());
        let target = world.spawn(SpaceshipRootMarker).id();
        let mut plates = Vec::new();
        for _ in 0..500 {
            plates.push(
                world
                    .spawn((SectionMarker, SectionClass::Hull, ChildOf(target)))
                    .id(),
            );
        }
        // A plate the stride would otherwise skip.
        let buried = plates[1];
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            CombatLock(Some(target)),
            LockFocus {
                target: Some(target),
                seconds: f32::MAX,
            },
            ComponentLock {
                section: Some(buried),
                ..default()
            },
        ));

        world.run_system_once(sync_component_markers).unwrap();

        let marked = marker_sections(&mut world);
        assert_eq!(marked.len(), MARKER_BUDGET);
        assert!(marked.contains(&buried));
    }
}
