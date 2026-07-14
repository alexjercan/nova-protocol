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

/// Square size (px) of one edge-arrow node. Sized to read at a glance
/// from the screen edge (user feedback 20260711: the first cut's 14 px
/// was too small).
const ARROW_PX: f32 = 24.0;

/// Chevron stroke length / thickness (px), and the stroke placement inside
/// the arrow node (two bars rotated +-45 degrees meeting at the top-center
/// apex, forming an up-pointing "^" - the orientation the widget's rotation
/// expects).
const STROKE_LEN_PX: f32 = 16.0;
const STROKE_THICK_PX: f32 = 3.0;

/// Inset (px) from the viewport edges the arrows clamp to. Slightly outside
/// the keybind cluster and readout margins so edge arrows read as a frame,
/// with room for the distance label under the arrow.
const EDGE_MARGIN_PX: f32 = 30.0;

/// Distance-label font size (px).
const LABEL_FONT_PX: f32 = 10.0;

/// Committed hostile torpedoes: full-presence threat red.
const TORPEDO_COLOR: Color = Color::srgba(1.0, 0.2, 0.2, 0.95);

/// Tracked candidates that are not the lock: the bracket overlay's dim red.
const CANDIDATE_COLOR: Color = Color::srgba(1.0, 0.25, 0.25, 0.45);

/// The combat-lock arrow follows the reticle's slot color (torpedo_target.rs,
/// task 20260713-124000): always combat-red - red = combat lock, white =
/// travel lock, everywhere. (The relation tint it used to mirror is retired.)
const LOCK_COLOR: Color = nova_ui::theme::semantic::THREAT;

pub mod prelude {
    pub use super::{
        edge_indicators_hud, EdgeIndicatorKind, EdgeIndicatorLabelMarker, EdgeIndicatorMarker,
        EdgeIndicatorTarget, EdgeIndicatorsHudMarker, EdgeIndicatorsHudPlugin,
    };
}

/// Marker for the full-screen edge-indicator layer.
#[derive(Component, Debug, Clone, Reflect)]
pub struct EdgeIndicatorsHudMarker;

/// Marker for one edge-indicator node.
#[derive(Component, Debug, Clone, Reflect)]
pub struct EdgeIndicatorMarker;

/// Marker for the distance label under an edge arrow.
#[derive(Component, Debug, Clone, Reflect)]
pub struct EdgeIndicatorLabelMarker;

/// The tracked entity this indicator points at.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct EdgeIndicatorTarget(pub Entity);

/// Why the entity is tracked; decides the arrow tint. An entity that
/// qualifies more than one way gets ONE indicator with the highest-priority
/// kind (Lock > Torpedo > Candidate).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Reflect)]
pub enum EdgeIndicatorKind {
    /// The active combat lock, combat-red like the reticle.
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
        children![
            edge_arrow(color),
            (
                // Distance readout under the arrow; a SIBLING of the arrow
                // node (the widget rotates the arrow), driven by
                // update_edge_labels and visible only while the arrow is.
                Name::new("EdgeIndicatorLabel"),
                EdgeIndicatorLabelMarker,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(100.0),
                    left: Val::Percent(50.0),
                    // Roughly center the short distance string under the
                    // arrow; UI layout has no translate(-50%), so a fixed
                    // half-width nudge stands in.
                    margin: UiRect {
                        left: Val::Px(-16.0),
                        top: Val::Px(2.0),
                        ..default()
                    },
                    ..default()
                },
                Text::new(""),
                TextFont::from_font_size(LABEL_FONT_PX),
                TextColor(color),
                Pickable::IGNORE,
                Visibility::Hidden,
            ),
        ],
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
        // The label mirrors the arrow's visibility, which the widget writes
        // in PostUpdate (ScreenIndicatorSystems) - mirroring from Update
        // would lag it by a frame (review R1.1), so the driver runs right
        // after the widget, still before UI layout consumes the text.
        app.add_systems(
            PostUpdate,
            update_edge_labels
                .after(ScreenIndicatorSystems)
                .before(bevy::ui::UiSystems::Layout),
        );
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
fn arrow_color(kind: EdgeIndicatorKind) -> Color {
    match kind {
        EdgeIndicatorKind::Lock => LOCK_COLOR,
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
    q_layer: Query<Entity, With<EdgeIndicatorsHudMarker>>,
    q_player: Query<
        (&Allegiance, Option<&CombatLock>, Option<&ThreatContacts>),
        With<PlayerSpaceshipMarker>,
    >,
    q_torpedoes: Query<
        (Entity, Option<&Allegiance>),
        (With<TorpedoProjectileMarker>, With<TorpedoTargetChosen>),
    >,
    q_indicators: Query<
        (Entity, &EdgeIndicatorTarget, &EdgeIndicatorKind),
        With<EdgeIndicatorMarker>,
    >,
) {
    let Ok(layer) = q_layer.single() else {
        // No layer means no player HUD; its despawn removed the indicators.
        return;
    };
    let (player_allegiance, lock, threats) = match q_player.single() {
        Ok((allegiance, lock, threats)) => (Some(allegiance), lock, threats),
        Err(_) => (None, None, None),
    };
    let empty = Vec::new();
    let wanted = tracked_entities(
        lock.and_then(|lock| lock.0),
        threats.map(|threats| &threats.entries).unwrap_or(&empty),
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
        commands
            .entity(layer)
            .with_child(edge_indicator(entity, kind, arrow_color(kind)));
    }
}

/// Fill each indicator's distance label and mirror the arrow's visibility
/// onto it, so the label shows exactly while the arrow is clamped to an
/// edge. The widget owns the ARROW's visibility (Inherited while clamped,
/// Hidden on-screen); the label follows it, and its text is the straight-
/// line distance from the player ship, `{:.0}m` like the lock readout.
#[allow(clippy::type_complexity)]
fn update_edge_labels(
    q_player: Query<&GlobalTransform, With<PlayerSpaceshipMarker>>,
    q_indicators: Query<(&EdgeIndicatorTarget, &Children), With<EdgeIndicatorMarker>>,
    q_transform: Query<&GlobalTransform>,
    q_arrow: Query<
        &Visibility,
        (
            With<ScreenIndicatorArrowMarker>,
            Without<EdgeIndicatorLabelMarker>,
        ),
    >,
    mut q_label: Query<
        (&mut Text, &mut Visibility),
        (
            With<EdgeIndicatorLabelMarker>,
            Without<ScreenIndicatorArrowMarker>,
        ),
    >,
) {
    let Ok(player) = q_player.single() else {
        return;
    };

    for (target, children) in &q_indicators {
        let arrow_shown = children
            .iter()
            .filter_map(|child| q_arrow.get(child).ok())
            .any(|visibility| *visibility != Visibility::Hidden);

        for child in children.iter() {
            let Ok((mut text, mut visibility)) = q_label.get_mut(child) else {
                continue;
            };
            if !arrow_shown {
                visibility.set_if_neq(Visibility::Hidden);
                continue;
            }
            let next = q_transform
                .get(**target)
                .map(|transform| {
                    let distance = transform.translation().distance(player.translation());
                    format!("{distance:.0}m")
                })
                .unwrap_or_default();
            if **text != next {
                **text = next;
            }
            visibility.set_if_neq(Visibility::Inherited);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// Layer + player + a locked hostile ship, a second candidate, and one
    /// committed torpedo per allegiance.
    fn tracked_world() -> (World, Entity, Entity, Entity, Entity) {
        let mut world = World::new();
        world.spawn(edge_indicators_hud());
        let player = world
            .spawn((PlayerSpaceshipMarker, GlobalTransform::IDENTITY))
            .id();
        let locked = world
            .spawn((
                SpaceshipRootMarker,
                Allegiance::Enemy,
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -500.0)),
            ))
            .id();
        let other = world
            .spawn((
                SpaceshipRootMarker,
                Allegiance::Enemy,
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 1200.0)),
            ))
            .id();
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
        world.entity_mut(player).insert((
            CombatLock(Some(locked)),
            ThreatContacts {
                entries: vec![locked, other],
            },
        ));
        (world, player, locked, other, enemy_torpedo)
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
        let (mut world, _player, locked, other, enemy_torpedo) = tracked_world();

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
        let (mut world, player, locked, other, _) = tracked_world();
        world.run_system_once(sync_edge_indicators).unwrap();

        world.get_mut::<CombatLock>(player).unwrap().0 = Some(other);
        world.run_system_once(sync_edge_indicators).unwrap();

        let all = indicators(&mut world);
        assert!(all.contains(&(other, EdgeIndicatorKind::Lock)));
        assert!(all.contains(&(locked, EdgeIndicatorKind::Candidate)));
        assert!(!all.contains(&(locked, EdgeIndicatorKind::Lock)));
    }

    #[test]
    fn dropped_entities_lose_their_indicator() {
        let (mut world, player, locked, other, enemy_torpedo) = tracked_world();
        world.run_system_once(sync_edge_indicators).unwrap();
        assert_eq!(indicators(&mut world).len(), 3);

        // The torpedo dies and the other ship leaves the tracked set.
        world.despawn(enemy_torpedo);
        world.get_mut::<ThreatContacts>(player).unwrap().entries = vec![locked];
        world.run_system_once(sync_edge_indicators).unwrap();

        assert_eq!(
            indicators(&mut world),
            vec![(locked, EdgeIndicatorKind::Lock)],
            "other {other:?} and the dead torpedo keep no arrows"
        );
    }

    #[test]
    fn the_kinds_have_their_tints_and_the_lock_arrow_is_combat_red() {
        // Slot-colored lock language (task 20260713-124000): the lock arrow
        // is always combat-red, relation-independent, matching the reticle.
        assert_eq!(arrow_color(EdgeIndicatorKind::Lock), LOCK_COLOR);
        assert_eq!(arrow_color(EdgeIndicatorKind::Torpedo), TORPEDO_COLOR);
        assert_eq!(arrow_color(EdgeIndicatorKind::Candidate), CANDIDATE_COLOR);
    }

    #[test]
    fn indicator_content_renders_nothing_on_screen() {
        // The widget hides the arrow while the anchor is on-screen and the
        // label driver mirrors that, so an indicator whose content is
        // exactly arrow + label renders nothing extra on-screen. Guard the
        // structure that property depends on.
        let (mut world, _player, locked, ..) = tracked_world();
        world.run_system_once(sync_edge_indicators).unwrap();

        let indicator = world
            .query_filtered::<(Entity, &EdgeIndicatorTarget), With<EdgeIndicatorMarker>>()
            .iter(&world)
            .find(|(_, target)| ***target == locked)
            .map(|(entity, _)| entity)
            .expect("lock indicator exists");
        let children: Vec<Entity> = world.entity(indicator).get::<Children>().unwrap().to_vec();
        assert_eq!(
            children.len(),
            2,
            "arrow + distance label, nothing else that could show on-screen"
        );
        for child in children {
            let is_arrow = world.entity(child).contains::<ScreenIndicatorArrowMarker>();
            let is_label = world.entity(child).contains::<EdgeIndicatorLabelMarker>();
            assert!(
                is_arrow ^ is_label,
                "every child is exactly one of arrow/label"
            );
            assert_eq!(
                *world.entity(child).get::<Visibility>().unwrap(),
                Visibility::Hidden,
                "all content starts hidden; the widget/label driver reveal it \
                 only while clamped"
            );
        }
    }

    #[test]
    fn label_shows_distance_while_the_arrow_is_shown() {
        let (mut world, _player, locked, ..) = tracked_world();
        world.run_system_once(sync_edge_indicators).unwrap();

        let indicator = world
            .query_filtered::<(Entity, &EdgeIndicatorTarget), With<EdgeIndicatorMarker>>()
            .iter(&world)
            .find(|(_, target)| ***target == locked)
            .map(|(entity, _)| entity)
            .expect("lock indicator exists");
        let children: Vec<Entity> = world.entity(indicator).get::<Children>().unwrap().to_vec();
        let arrow = children
            .iter()
            .copied()
            .find(|&child| world.entity(child).contains::<ScreenIndicatorArrowMarker>())
            .unwrap();
        let label = children
            .iter()
            .copied()
            .find(|&child| world.entity(child).contains::<EdgeIndicatorLabelMarker>())
            .unwrap();

        // Arrow hidden (target on-screen): label stays hidden.
        world.run_system_once(update_edge_labels).unwrap();
        assert_eq!(
            *world.entity(label).get::<Visibility>().unwrap(),
            Visibility::Hidden
        );

        // The widget clamps the arrow (Inherited): the label shows the
        // player-to-target distance.
        *world.entity_mut(arrow).get_mut::<Visibility>().unwrap() = Visibility::Inherited;
        world.run_system_once(update_edge_labels).unwrap();
        assert_eq!(
            *world.entity(label).get::<Visibility>().unwrap(),
            Visibility::Inherited
        );
        assert_eq!(world.entity(label).get::<Text>().unwrap().0, "500m");

        // The target moves: the text follows (the value is live, not a
        // spawn-time snapshot).
        world
            .entity_mut(locked)
            .insert(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -2400.0,
            )));
        world.run_system_once(update_edge_labels).unwrap();
        assert_eq!(world.entity(label).get::<Text>().unwrap().0, "2400m");

        // Back on-screen: the widget hides the arrow, the label follows.
        *world.entity_mut(arrow).get_mut::<Visibility>().unwrap() = Visibility::Hidden;
        world.run_system_once(update_edge_labels).unwrap();
        assert_eq!(
            *world.entity(label).get::<Visibility>().unwrap(),
            Visibility::Hidden
        );
    }
}
