//! Off-screen target/threat edge indicators: an arrow clamped to the screen
//! edge pointing at each tracked entity - the active lock, the multi-target
//! candidates, and committed hostile torpedoes - so the player knows where
//! to turn (task 20260708-165704; data source decided in
//! docs/spikes/20260711-163800-multi-target-cycle.md).
//!
//! First consumer of the screen-indicator widget's `ClampToEdge` +
//! [`ScreenIndicatorArrowMarker`] path: each indicator's only visible
//! content is the arrow, and the widget shows/rotates the arrow ONLY while
//! the anchor is off-screen (or behind the camera), so on-screen entities
//! render nothing extra - no visibility coordination with the reticle or
//! the candidate brackets is needed.

use bevy::prelude::*;

use crate::prelude::*;

/// Square size (px) of one edge-arrow node.
const ARROW_PX: f32 = 14.0;

/// Chevron stroke length / thickness (px), and the stroke placement inside
/// the arrow node (two bars rotated +-45 degrees meeting at the top-center
/// apex, forming an up-pointing "^" - the orientation the widget's rotation
/// expects).
const STROKE_LEN_PX: f32 = 9.0;
const STROKE_THICK_PX: f32 = 2.0;

/// Inset (px) from the viewport edges the arrows clamp to. Slightly outside
/// the keybind cluster and readout margins so edge arrows read as a frame.
const EDGE_MARGIN_PX: f32 = 24.0;

/// Committed hostile torpedoes: full-presence threat red.
const TORPEDO_COLOR: Color = Color::srgba(1.0, 0.2, 0.2, 0.95);

/// Tracked candidates that are not the lock: the bracket overlay's dim red.
const CANDIDATE_COLOR: Color = Color::srgba(1.0, 0.25, 0.25, 0.45);

// The lock arrow follows the reticle's relation tint (torpedo_target.rs):
// hostile reads as a threat, own as friendly, neutral plain white.
const LOCK_HOSTILE_COLOR: Color = Color::srgba(1.0, 0.35, 0.3, 1.0);
const LOCK_OWN_COLOR: Color = Color::srgba(0.35, 0.9, 0.55, 1.0);
const LOCK_NEUTRAL_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);

pub mod prelude {
    pub use super::{
        edge_indicators_hud, EdgeIndicatorKind, EdgeIndicatorMarker, EdgeIndicatorTarget,
        EdgeIndicatorsHudMarker, EdgeIndicatorsHudPlugin,
    };
}

/// Marker for the full-screen edge-indicator layer.
#[derive(Component, Debug, Clone, Reflect)]
pub struct EdgeIndicatorsHudMarker;

/// Marker for one edge-indicator node.
#[derive(Component, Debug, Clone, Reflect)]
pub struct EdgeIndicatorMarker;

/// The tracked entity this indicator points at.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct EdgeIndicatorTarget(pub Entity);

/// Why the entity is tracked; decides the arrow tint. An entity that
/// qualifies more than one way gets ONE indicator with the highest-priority
/// kind (Lock > Torpedo > Candidate).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Reflect)]
pub enum EdgeIndicatorKind {
    /// The active target lock, tinted by relation like the reticle.
    Lock,
    /// A committed hostile torpedo - a threat to run from.
    Torpedo,
    /// A tracked multi-target candidate that is not the lock.
    Candidate,
}

/// UI bundle for the edge-indicator layer. Indicators are spawned under it
/// by [`sync_edge_indicators`], one per tracked entity.
pub fn edge_indicators_hud() -> impl Bundle {
    (
        Name::new("EdgeIndicatorsHUD"),
        EdgeIndicatorsHudMarker,
        screen_indicator_layer(),
    )
}

/// Bundle for one edge indicator: a small clamped screen indicator whose
/// only content is the arrow chevron, so it is invisible while its anchor
/// is on-screen (the widget hides the arrow) and an edge-clamped pointer
/// while it is not.
fn edge_indicator(target: Entity, kind: EdgeIndicatorKind, color: Color) -> impl Bundle {
    (
        Name::new("EdgeIndicator"),
        EdgeIndicatorMarker,
        EdgeIndicatorTarget(target),
        kind,
        screen_indicator(ScreenIndicatorConfig {
            anchor: Some(ScreenIndicatorAnchorKind::Entity(target)),
            size: ScreenIndicatorSize::Fixed(Vec2::splat(ARROW_PX)),
            offset: Vec2::ZERO,
            offscreen: ScreenIndicatorOffscreen::ClampToEdge {
                margin_px: EDGE_MARGIN_PX,
            },
        }),
        children![edge_arrow(color)],
    )
}

/// An up-pointing chevron built from two rotated bar nodes - UI-node art
/// like the candidate brackets, no image asset. The widget rotates the
/// WHOLE node toward the anchor via `UiTransform` and toggles its
/// visibility, so the strokes only carry their local +-45 degree tilt.
fn edge_arrow(color: Color) -> impl Bundle {
    let stroke = |left: f32, degrees: f32| {
        (
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(ARROW_PX / 2.0 - STROKE_THICK_PX / 2.0),
                width: Val::Px(STROKE_LEN_PX),
                height: Val::Px(STROKE_THICK_PX),
                ..default()
            },
            UiTransform {
                rotation: Rot2::degrees(degrees),
                ..default()
            },
            BackgroundColor(color),
            Pickable::IGNORE,
        )
    };

    (
        Name::new("EdgeIndicatorArrow"),
        ScreenIndicatorArrowMarker,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(ARROW_PX),
            height: Val::Px(ARROW_PX),
            ..default()
        },
        UiTransform::default(),
        Visibility::Hidden,
        Pickable::IGNORE,
        children![
            // Left stroke "/" and right stroke "\" meeting at the apex.
            stroke(-0.7, -45.0),
            stroke(ARROW_PX - STROKE_LEN_PX + 0.7, 45.0),
        ],
    )
}

#[derive(Default)]
pub struct EdgeIndicatorsHudPlugin;

impl Plugin for EdgeIndicatorsHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("EdgeIndicatorsHudPlugin: build");

        app.register_type::<EdgeIndicatorKind>();
        app.add_systems(Update, sync_edge_indicators.in_set(super::NovaHudSystems));
    }
}

/// The entities worth pointing at this frame, with the kind deciding the
/// tint. Pure so the tracking rule is unit-testable without UI: the lock
/// (any relation - it is also the GOTO designation), committed hostile
/// torpedoes, and the multi-target candidates, deduplicated with Lock
/// winning over the rest.
fn tracked_entities(
    lock: Option<Entity>,
    candidates: &[Entity],
    hostile_torpedoes: impl Iterator<Item = Entity>,
) -> Vec<(Entity, EdgeIndicatorKind)> {
    let mut wanted: Vec<(Entity, EdgeIndicatorKind)> = Vec::new();
    if let Some(target) = lock {
        wanted.push((target, EdgeIndicatorKind::Lock));
    }
    for torpedo in hostile_torpedoes {
        if lock != Some(torpedo) {
            wanted.push((torpedo, EdgeIndicatorKind::Torpedo));
        }
    }
    for &ship in candidates {
        if lock != Some(ship) {
            wanted.push((ship, EdgeIndicatorKind::Candidate));
        }
    }
    wanted
}

/// The arrow tint for a tracked entity.
fn arrow_color(kind: EdgeIndicatorKind, lock_relation: Relation) -> Color {
    match kind {
        EdgeIndicatorKind::Lock => match lock_relation {
            Relation::Hostile => LOCK_HOSTILE_COLOR,
            Relation::Own => LOCK_OWN_COLOR,
            Relation::Neutral => LOCK_NEUTRAL_COLOR,
        },
        EdgeIndicatorKind::Torpedo => TORPEDO_COLOR,
        EdgeIndicatorKind::Candidate => CANDIDATE_COLOR,
    }
}

/// Keep exactly one indicator per tracked entity, tinted by kind. The same
/// reconcile shape as the component markers and candidate brackets: the
/// tracked set churns freely (lock moves, torpedoes commit and die,
/// candidates come and go), one idempotent pass covers every ordering. A
/// kind change (a candidate becomes the lock) respawns the indicator - it
/// is a different pointer, and lock switches are rare enough that the churn
/// is irrelevant.
#[allow(clippy::type_complexity)]
fn sync_edge_indicators(
    mut commands: Commands,
    lock: Res<SpaceshipPlayerTargetLock>,
    candidates: Res<SpaceshipPlayerTargetCandidates>,
    q_layer: Query<Entity, With<EdgeIndicatorsHudMarker>>,
    q_player: Query<&Allegiance, With<PlayerSpaceshipMarker>>,
    q_torpedoes: Query<
        (Entity, Option<&Allegiance>),
        (With<TorpedoProjectileMarker>, With<TorpedoTargetChosen>),
    >,
    q_allegiance: Query<&Allegiance>,
    q_indicators: Query<
        (Entity, &EdgeIndicatorTarget, &EdgeIndicatorKind),
        With<EdgeIndicatorMarker>,
    >,
) {
    let Ok(layer) = q_layer.single() else {
        // No layer means no player HUD; its despawn removed the indicators.
        return;
    };
    let player_allegiance = q_player.single().ok();

    let wanted = tracked_entities(
        **lock,
        &candidates.entries,
        q_torpedoes.iter().filter_map(|(torpedo, allegiance)| {
            (relation(player_allegiance, allegiance) == Relation::Hostile).then_some(torpedo)
        }),
    );

    // Despawn indicators whose entity dropped out or whose kind changed.
    for (indicator, target, kind) in &q_indicators {
        let keep = wanted
            .iter()
            .any(|&(entity, want)| entity == **target && want == *kind);
        if !keep {
            commands.entity(indicator).despawn();
        }
    }

    // Spawn indicators for tracked entities that have none yet.
    for &(entity, kind) in &wanted {
        let exists = q_indicators
            .iter()
            .any(|(_, target, have)| **target == entity && *have == kind);
        if exists {
            continue;
        }
        let lock_relation = relation(player_allegiance, q_allegiance.get(entity).ok());
        commands.entity(layer).with_child(edge_indicator(
            entity,
            kind,
            arrow_color(kind, lock_relation),
        ));
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// Layer + player + a locked hostile ship, a second candidate, and one
    /// committed torpedo per allegiance.
    fn tracked_world() -> (World, Entity, Entity, Entity) {
        let mut world = World::new();
        world.spawn(edge_indicators_hud());
        world.spawn(PlayerSpaceshipMarker);
        let locked = world.spawn((SpaceshipRootMarker, Allegiance::Enemy)).id();
        let other = world.spawn((SpaceshipRootMarker, Allegiance::Enemy)).id();
        let enemy_torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetChosen,
                Allegiance::Enemy,
            ))
            .id();
        world.spawn((
            // The player's own committed torpedo: lockable, but not a threat.
            TorpedoProjectileMarker,
            TorpedoTargetChosen,
            Allegiance::Player,
        ));
        world.spawn((
            // An uncommitted enemy torpedo is not tracked either.
            TorpedoProjectileMarker,
            Allegiance::Enemy,
        ));
        world.insert_resource(SpaceshipPlayerTargetLock(Some(locked)));
        world.insert_resource(SpaceshipPlayerTargetCandidates {
            entries: vec![locked, other],
            pinned_until: None,
        });
        (world, locked, other, enemy_torpedo)
    }

    fn indicators(world: &mut World) -> Vec<(Entity, EdgeIndicatorKind)> {
        let mut all: Vec<(Entity, EdgeIndicatorKind)> = world
            .query_filtered::<(&EdgeIndicatorTarget, &EdgeIndicatorKind), With<EdgeIndicatorMarker>>()
            .iter(world)
            .map(|(target, kind)| (**target, *kind))
            .collect();
        all.sort();
        all
    }

    #[test]
    fn tracks_lock_candidates_and_hostile_torpedoes_once_each() {
        let (mut world, locked, other, enemy_torpedo) = tracked_world();

        world.run_system_once(sync_edge_indicators).unwrap();

        let mut expected = vec![
            (locked, EdgeIndicatorKind::Lock),
            (other, EdgeIndicatorKind::Candidate),
            (enemy_torpedo, EdgeIndicatorKind::Torpedo),
        ];
        expected.sort();
        assert_eq!(
            indicators(&mut world),
            expected,
            "the locked candidate gets ONE indicator, as the lock; own and \
             uncommitted torpedoes are not threats"
        );
    }

    #[test]
    fn lock_change_reassigns_kinds() {
        let (mut world, locked, other, _) = tracked_world();
        world.run_system_once(sync_edge_indicators).unwrap();

        world.insert_resource(SpaceshipPlayerTargetLock(Some(other)));
        world.run_system_once(sync_edge_indicators).unwrap();

        let all = indicators(&mut world);
        assert!(all.contains(&(other, EdgeIndicatorKind::Lock)));
        assert!(all.contains(&(locked, EdgeIndicatorKind::Candidate)));
        assert!(!all.contains(&(locked, EdgeIndicatorKind::Lock)));
    }

    #[test]
    fn dropped_entities_lose_their_indicator() {
        let (mut world, locked, other, enemy_torpedo) = tracked_world();
        world.run_system_once(sync_edge_indicators).unwrap();
        assert_eq!(indicators(&mut world).len(), 3);

        // The torpedo dies and the other ship leaves the tracked set.
        world.despawn(enemy_torpedo);
        world
            .resource_mut::<SpaceshipPlayerTargetCandidates>()
            .entries = vec![locked];
        world.run_system_once(sync_edge_indicators).unwrap();

        assert_eq!(
            indicators(&mut world),
            vec![(locked, EdgeIndicatorKind::Lock)],
            "other {other:?} and the dead torpedo keep no arrows"
        );
    }

    #[test]
    fn lock_arrow_color_follows_relation_and_kinds_have_their_tints() {
        assert_eq!(
            arrow_color(EdgeIndicatorKind::Lock, Relation::Hostile),
            LOCK_HOSTILE_COLOR
        );
        assert_eq!(
            arrow_color(EdgeIndicatorKind::Lock, Relation::Own),
            LOCK_OWN_COLOR
        );
        assert_eq!(
            arrow_color(EdgeIndicatorKind::Lock, Relation::Neutral),
            LOCK_NEUTRAL_COLOR
        );
        assert_eq!(
            arrow_color(EdgeIndicatorKind::Torpedo, Relation::Neutral),
            TORPEDO_COLOR,
            "non-lock kinds ignore the relation"
        );
        assert_eq!(
            arrow_color(EdgeIndicatorKind::Candidate, Relation::Hostile),
            CANDIDATE_COLOR
        );
    }

    #[test]
    fn indicator_content_is_the_arrow_only() {
        // The widget hides the arrow while the anchor is on-screen, so an
        // indicator whose ONLY content is the arrow renders nothing extra
        // on-screen. Guard the structure that property depends on: exactly
        // one child, and it carries ScreenIndicatorArrowMarker.
        let (mut world, locked, ..) = tracked_world();
        world.run_system_once(sync_edge_indicators).unwrap();

        let indicator = world
            .query_filtered::<(Entity, &EdgeIndicatorTarget), With<EdgeIndicatorMarker>>()
            .iter(&world)
            .find(|(_, target)| ***target == locked)
            .map(|(entity, _)| entity)
            .expect("lock indicator exists");
        let children: Vec<Entity> = world.entity(indicator).get::<Children>().unwrap().to_vec();
        assert_eq!(children.len(), 1, "the arrow is the only content");
        assert!(
            world
                .entity(children[0])
                .contains::<ScreenIndicatorArrowMarker>(),
            "the single child is the widget-driven arrow"
        );
        assert_eq!(
            *world.entity(children[0]).get::<Visibility>().unwrap(),
            Visibility::Hidden,
            "the arrow starts hidden; the widget shows it only when clamped"
        );
    }
}
