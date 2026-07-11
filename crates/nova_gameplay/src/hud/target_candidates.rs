//! Multi-target candidate brackets: one hollow marker per tracked hostile
//! ship, so the player sees the set the target cycle walks (task
//! 20260708-165705, design in
//! docs/spikes/20260711-163800-multi-target-cycle.md).
//!
//! A thin consumer of the [`screen_indicator`](super::screen_indicator)
//! widget, reconcile-style like the component-lock markers: membership
//! mirrors [`SpaceshipPlayerTargetCandidates`], EXCEPT the active lock -
//! the reticle already marks it, and a second box would just be noise. The
//! brackets hide off-screen; pointing at off-screen candidates is the
//! edge-indicator overlay's job (task 20260708-165704).

use bevy::prelude::*;

use crate::prelude::*;

/// Minimum (and fallback) on-screen size of a candidate bracket (px).
/// Slightly under the reticle minimum so brackets read as secondary.
const BRACKET_MIN_PX: f32 = 28.0;

/// Bracket border thickness (px).
const BRACKET_BORDER_PX: f32 = 1.5;

/// Bracket tint: hostile red, dimmer than the reticle and distinct from the
/// hot-metal component markers - present, not shouting.
const BRACKET_COLOR: Color = Color::srgba(1.0, 0.25, 0.25, 0.45);

pub mod prelude {
    pub use super::{
        target_candidates_hud, TargetCandidateBracketMarker, TargetCandidateTarget,
        TargetCandidatesHudMarker, TargetCandidatesHudPlugin,
    };
}

/// Marker for the full-screen candidate-bracket layer.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TargetCandidatesHudMarker;

/// Marker for one candidate bracket node.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TargetCandidateBracketMarker;

/// The candidate ship this bracket overlays.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct TargetCandidateTarget(pub Entity);

/// UI bundle for the bracket layer. Brackets are spawned under it by
/// [`sync_candidate_brackets`], one per tracked candidate that is not the
/// active lock.
pub fn target_candidates_hud() -> impl Bundle {
    (
        Name::new("TargetCandidatesHUD"),
        TargetCandidatesHudMarker,
        screen_indicator_layer(),
    )
}

/// Bundle for a single candidate bracket: an ApparentSize-tracking indicator
/// whose only content is a hollow border child (the widget owns the node's
/// size each frame, so the border rides a full-size child instead).
fn candidate_bracket(ship: Entity) -> impl Bundle {
    (
        Name::new("TargetCandidateBracket"),
        TargetCandidateBracketMarker,
        TargetCandidateTarget(ship),
        screen_indicator(ScreenIndicatorConfig {
            anchor: Some(ScreenIndicatorAnchorKind::Entity(ship)),
            size: ScreenIndicatorSize::ApparentSize {
                min_px: BRACKET_MIN_PX,
            },
            offset: Vec2::ZERO,
            offscreen: ScreenIndicatorOffscreen::Hide,
        }),
        children![(
            Name::new("TargetCandidateBracketBorder"),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                border: UiRect::all(Val::Px(BRACKET_BORDER_PX)),
                ..default()
            },
            BorderColor::all(BRACKET_COLOR),
            Pickable::IGNORE,
        )],
    )
}

#[derive(Default)]
pub struct TargetCandidatesHudPlugin;

impl Plugin for TargetCandidatesHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("TargetCandidatesHudPlugin: build");

        app.add_systems(
            Update,
            sync_candidate_brackets.in_set(super::NovaHudSystems),
        );
    }
}

/// Keep exactly one bracket per tracked candidate that is not the active
/// lock. A reconcile pass like the component markers: the set churns freely
/// (ships die, leave range, get locked) and one idempotent pass covers every
/// ordering.
fn sync_candidate_brackets(
    mut commands: Commands,
    lock: Res<SpaceshipPlayerTargetLock>,
    candidates: Res<SpaceshipPlayerTargetCandidates>,
    q_layer: Query<Entity, With<TargetCandidatesHudMarker>>,
    q_brackets: Query<(Entity, &TargetCandidateTarget), With<TargetCandidateBracketMarker>>,
) {
    let Ok(layer) = q_layer.single() else {
        // No layer means no player HUD; its despawn removed the brackets too.
        return;
    };

    let wanted = |ship: Entity| candidates.entries.contains(&ship) && Some(ship) != **lock;

    // Despawn brackets whose candidate dropped out or became the lock.
    for (bracket, ship) in &q_brackets {
        if !wanted(**ship) {
            commands.entity(bracket).despawn();
        }
    }

    // Spawn brackets for candidates that have none yet.
    for &ship in &candidates.entries {
        if !wanted(ship) {
            continue;
        }
        let has_bracket = q_brackets.iter().any(|(_, marked)| **marked == ship);
        if !has_bracket {
            commands.entity(layer).with_child(candidate_bracket(ship));
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// Layer + three tracked candidates, the first of which is the lock.
    fn tracked_world() -> (World, [Entity; 3]) {
        let mut world = World::new();
        world.spawn(target_candidates_hud());
        let a = world.spawn(SpaceshipRootMarker).id();
        let b = world.spawn(SpaceshipRootMarker).id();
        let c = world.spawn(SpaceshipRootMarker).id();
        world.insert_resource(SpaceshipPlayerTargetLock(Some(a)));
        world.insert_resource(SpaceshipPlayerTargetCandidates {
            entries: vec![a, b, c],
            pinned_until: None,
        });
        (world, [a, b, c])
    }

    fn bracket_ships(world: &mut World) -> Vec<Entity> {
        let mut ships: Vec<Entity> = world
            .query_filtered::<&TargetCandidateTarget, With<TargetCandidateBracketMarker>>()
            .iter(world)
            .map(|ship| **ship)
            .collect();
        ships.sort();
        ships
    }

    #[test]
    fn brackets_skip_the_active_lock() {
        let (mut world, [a, b, c]) = tracked_world();

        world.run_system_once(sync_candidate_brackets).unwrap();

        let mut expected = vec![b, c];
        expected.sort();
        assert_eq!(
            bracket_ships(&mut world),
            expected,
            "the locked ship {a:?} keeps only its reticle"
        );
    }

    #[test]
    fn brackets_follow_lock_and_membership_changes() {
        let (mut world, [a, b, c]) = tracked_world();
        world.run_system_once(sync_candidate_brackets).unwrap();

        // The lock moves to b: a gets a bracket back, b loses its own.
        world.insert_resource(SpaceshipPlayerTargetLock(Some(b)));
        world.run_system_once(sync_candidate_brackets).unwrap();
        let mut expected = vec![a, c];
        expected.sort();
        assert_eq!(bracket_ships(&mut world), expected);

        // c drops out of the tracked set: its bracket goes with it.
        world
            .resource_mut::<SpaceshipPlayerTargetCandidates>()
            .entries = vec![a, b];
        world.run_system_once(sync_candidate_brackets).unwrap();
        assert_eq!(bracket_ships(&mut world), vec![a]);
    }

    #[test]
    fn empty_set_clears_every_bracket() {
        let (mut world, _) = tracked_world();
        world.run_system_once(sync_candidate_brackets).unwrap();
        assert_eq!(bracket_ships(&mut world).len(), 2);

        world.insert_resource(SpaceshipPlayerTargetCandidates::default());
        world.run_system_once(sync_candidate_brackets).unwrap();

        assert!(bracket_ships(&mut world).is_empty());
    }
}
