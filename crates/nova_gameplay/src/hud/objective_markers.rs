//! Objective marker HUD chips (task 20260712-093831, spike
//! docs/spikes/20260712-140842-objective-conveyance-visuals.md): one
//! screen-projected gold chip per [`ObjectiveMarkerTarget`] entity - the
//! marker's label plus live distance to the player ship - with the
//! indicator widget's `ClampToEdge` path, so an off-screen objective's chip
//! pins to the viewport edge and its chevron points at it. Where the nav
//! beacon chip says "a waypoint exists", this chip says "go HERE now":
//! gold, a diamond glyph instead of the beacon dot language, and a slow
//! alpha breath so it reads in peripheral vision without strobing.
//!
//! Chrome tier, like the beacon chips - the same nav-chip family.

use bevy::prelude::*;

use super::{screen_indicator::prelude::*, HudTier, OBJECTIVE_GOLD};
use crate::prelude::*;

pub mod prelude {
    pub use super::{
        ObjectiveMarkerChipHudMarker, ObjectiveMarkerChipLabelMarker,
        ObjectiveMarkerChipTargetEntity, ObjectiveMarkersHudPlugin,
    };
}

/// Chip footprint (px). Matches the beacon chip so the marker reads as the
/// same chip "promoted" to gold.
const CHIP_SIZE: Vec2 = Vec2::new(140.0, 16.0);

/// The chip floats above its target so the label never sits on the mesh.
const CHIP_OFFSET: Vec2 = Vec2::new(0.0, -28.0);

/// Inset (px) from the viewport edges while clamped; the shared HUD frame.
const EDGE_MARGIN_PX: f32 = 30.0;

/// Chevron stroke geometry, the edge-indicator arrow language at chip scale.
const ARROW_PX: f32 = 16.0;
const STROKE_LEN_PX: f32 = 11.0;
const STROKE_THICK_PX: f32 = 2.0;

/// Diamond glyph: a square border rotated 45 degrees, sitting left of the
/// label - the marker's identity mark, always visible (the chevron only
/// shows while edge-clamped).
const DIAMOND_PX: f32 = 8.0;
const DIAMOND_BORDER_PX: f32 = 1.5;

const LABEL_FONT_PX: f32 = 12.0;

/// Alpha breath of the whole chip: slow and shallow - noticeable in
/// peripheral vision, not a strobe.
const BREATH_PERIOD_SECS: f32 = 1.25;
const BREATH_ALPHA_MIN: f32 = 0.7;
const BREATH_ALPHA_MAX: f32 = 1.0;

/// Marker for one objective marker chip layer (one per marked entity).
#[derive(Component, Debug, Clone, Reflect)]
pub struct ObjectiveMarkerChipHudMarker;

/// The marked entity this chip tracks.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct ObjectiveMarkerChipTargetEntity(pub Entity);

/// Marker for the chip's text node.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ObjectiveMarkerChipLabelMarker;

/// Marker for a node whose color breathes with the chip: the diamond
/// border and the chevron strokes - NOT the label text, which stays at
/// full alpha for readability (task 20260712-152340).
#[derive(Component, Debug, Clone, Reflect)]
struct ObjectiveMarkerBreathMarker;

/// UI bundle for one marked entity's chip layer.
fn objective_marker_chip_hud(target: Entity) -> impl Bundle {
    (
        Name::new("ObjectiveMarkerChipHUD"),
        ObjectiveMarkerChipHudMarker,
        ObjectiveMarkerChipTargetEntity(target),
        HudTier::Chrome,
        screen_indicator_layer(),
        children![(
            Name::new("ObjectiveMarkerChipUI"),
            ObjectiveMarkerChipLabelMarker,
            screen_indicator(ScreenIndicatorConfig {
                anchor: Some(ScreenIndicatorAnchorKind::Entity(target)),
                size: ScreenIndicatorSize::Fixed(CHIP_SIZE),
                offset: CHIP_OFFSET,
                offscreen: ScreenIndicatorOffscreen::ClampToEdge {
                    margin_px: EDGE_MARGIN_PX,
                },
            }),
            Text::new(""),
            TextFont::from_font_size(LABEL_FONT_PX),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            // The LABEL does not breathe: 12 px gold at 0.7 alpha over a
            // bright planetoid was unreadable (playtest 2026-07-12, task
            // 20260712-152340). Constant full gold + a tight dark shadow
            // for contrast; the diamond and chevron carry the motion.
            TextColor(OBJECTIVE_GOLD),
            TextShadow {
                offset: Vec2::splat(1.0),
                color: Color::srgba(0.0, 0.0, 0.0, 0.9),
            },
            children![objective_marker_diamond(), objective_marker_arrow()],
        )],
    )
}

/// The diamond identity glyph: a hollow square border rotated 45 degrees
/// (the same UiTransform trick as the chevron strokes), parked left of the
/// label text.
fn objective_marker_diamond() -> impl Bundle {
    (
        Name::new("ObjectiveMarkerDiamond"),
        ObjectiveMarkerBreathMarker,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(-(DIAMOND_PX + 6.0)),
            top: Val::Px(LABEL_FONT_PX / 2.0 - DIAMOND_PX / 2.0 + 1.0),
            width: Val::Px(DIAMOND_PX),
            height: Val::Px(DIAMOND_PX),
            border: UiRect::all(Val::Px(DIAMOND_BORDER_PX)),
            ..default()
        },
        UiTransform {
            rotation: Rot2::degrees(45.0),
            ..default()
        },
        BorderColor::all(OBJECTIVE_GOLD),
        Pickable::IGNORE,
    )
}

/// An up-pointing chevron the widget rotates toward the target while the
/// chip is edge-clamped (the edge-indicator arrow language, chip-sized).
/// Hidden while the target is on-screen - the widget owns its visibility.
fn objective_marker_arrow() -> impl Bundle {
    let stroke = |left: f32, degrees: f32| {
        (
            ObjectiveMarkerBreathMarker,
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
            BackgroundColor(OBJECTIVE_GOLD),
            Pickable::IGNORE,
        )
    };

    (
        Name::new("ObjectiveMarkerArrow"),
        ScreenIndicatorArrowMarker,
        Node {
            position_type: PositionType::Absolute,
            // Park the chevron just above the label text, centered on the
            // chip's anchor point.
            left: Val::Px(-ARROW_PX / 2.0),
            top: Val::Px(-ARROW_PX - 2.0),
            width: Val::Px(ARROW_PX),
            height: Val::Px(ARROW_PX),
            ..default()
        },
        UiTransform::default(),
        Visibility::Hidden,
        Pickable::IGNORE,
        children![
            stroke(-0.5, -45.0),
            stroke(ARROW_PX - STROKE_LEN_PX + 0.5, 45.0),
        ],
    )
}

#[derive(Default)]
pub struct ObjectiveMarkersHudPlugin;

impl Plugin for ObjectiveMarkersHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("ObjectiveMarkersHudPlugin: build");

        app.register_type::<ObjectiveMarkerTarget>();

        app.add_observer(setup_objective_marker_chip);
        app.add_observer(remove_objective_marker_chip);
        app.add_systems(
            Update,
            (update_objective_marker_labels, breathe_objective_markers)
                .in_set(super::NovaHudSystems),
        );
    }
}

/// Every marked entity grows its chip the moment the tag lands.
fn setup_objective_marker_chip(add: On<Add, ObjectiveMarkerTarget>, mut commands: Commands) {
    let target = add.entity;
    debug!("setup_objective_marker_chip: target {:?}", target);
    commands.spawn(objective_marker_chip_hud(target));
}

/// The chip layer dies with its tag - explicit detach action or the marked
/// entity despawning (crate picked up, pirate destroyed, scenario unload).
fn remove_objective_marker_chip(
    remove: On<Remove, ObjectiveMarkerTarget>,
    mut commands: Commands,
    q_chips: Query<(Entity, &ObjectiveMarkerChipTargetEntity), With<ObjectiveMarkerChipHudMarker>>,
) {
    let target = remove.entity;
    for (chip, chip_target) in &q_chips {
        if **chip_target == target {
            trace!("remove_objective_marker_chip: despawning chip {:?}", chip);
            commands.entity(chip).despawn();
        }
    }
}

/// Label text: the marker's label plus the live distance to the player ship
/// ("BEACON 1  420m"). Without a player (death gap) the label alone shows.
fn update_objective_marker_labels(
    q_chips: Query<&ObjectiveMarkerChipTargetEntity, With<ObjectiveMarkerChipHudMarker>>,
    mut q_labels: Query<(&mut Text, &ChildOf), With<ObjectiveMarkerChipLabelMarker>>,
    q_targets: Query<(&ObjectiveMarkerTarget, &GlobalTransform)>,
    q_player: Query<&GlobalTransform, With<PlayerSpaceshipMarker>>,
) {
    let player = q_player.iter().next();
    for (mut text, ChildOf(layer)) in &mut q_labels {
        let Ok(target) = q_chips.get(*layer) else {
            continue;
        };
        let Ok((marker, target_transform)) = q_targets.get(**target) else {
            continue;
        };
        let next = match player {
            Some(player_transform) => {
                let distance = player_transform
                    .translation()
                    .distance(target_transform.translation());
                format!("{}  {:.0}m", marker.label, distance)
            }
            None => marker.label.clone(),
        };
        if **text != next {
            **text = next;
        }
    }
}

/// The breath wave at `elapsed` seconds: the alpha every chip color node
/// carries this frame. One shared wave (not per-chip phase) - simultaneous
/// markers breathing in unison read as one system.
fn breath_alpha(elapsed_secs: f32) -> f32 {
    let t = elapsed_secs * std::f32::consts::TAU / BREATH_PERIOD_SECS;
    let wave = 0.5 + 0.5 * t.sin();
    BREATH_ALPHA_MIN + (BREATH_ALPHA_MAX - BREATH_ALPHA_MIN) * wave
}

/// Breathe every chip's gold GLYPHS - the diamond border and the chevron
/// strokes - with the shared wave. The label text deliberately does NOT
/// breathe: thinning 12 px text to 0.7 alpha broke readability over
/// bright scene content (playtest 2026-07-12, task 20260712-152340); the
/// glyphs carry all the motion.
fn breathe_objective_markers(
    time: Res<Time>,
    mut q_border: Query<&mut BorderColor, With<ObjectiveMarkerBreathMarker>>,
    mut q_background: Query<&mut BackgroundColor, With<ObjectiveMarkerBreathMarker>>,
) {
    let alpha = breath_alpha(time.elapsed_secs());
    let breathed = OBJECTIVE_GOLD.with_alpha(OBJECTIVE_GOLD.alpha() * alpha);
    for mut border in &mut q_border {
        *border = BorderColor::all(breathed);
    }
    for mut background in &mut q_background {
        background.0 = breathed;
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn world_with_observers() -> World {
        let mut world = World::new();
        world.add_observer(setup_objective_marker_chip);
        world.add_observer(remove_objective_marker_chip);
        world
    }

    fn chips(world: &mut World) -> Vec<(Entity, Entity)> {
        world
            .query_filtered::<(Entity, &ObjectiveMarkerChipTargetEntity), With<ObjectiveMarkerChipHudMarker>>()
            .iter(world)
            .map(|(chip, target)| (chip, **target))
            .collect()
    }

    /// Attach grows exactly one chip per tag; detach (component removal)
    /// takes exactly that chip down and leaves siblings alone.
    #[test]
    fn chips_follow_the_tag_lifecycle() {
        let mut world = world_with_observers();
        let a = world.spawn(ObjectiveMarkerTarget::new("BEACON 1")).id();
        let b = world.spawn(ObjectiveMarkerTarget::new("SCAVENGER")).id();
        world.flush();

        let spawned = chips(&mut world);
        assert_eq!(spawned.len(), 2, "one chip per marked entity");
        assert!(spawned.iter().any(|(_, target)| *target == a));
        assert!(spawned.iter().any(|(_, target)| *target == b));

        world.entity_mut(a).remove::<ObjectiveMarkerTarget>();
        world.flush();

        let remaining = chips(&mut world);
        assert_eq!(remaining.len(), 1, "detach removes exactly one chip");
        assert_eq!(remaining[0].1, b, "the other marker survives");
    }

    /// Despawning the marked entity (crate picked up, pirate destroyed,
    /// scenario teardown) is a detach too - the Remove observer fires on
    /// despawn.
    #[test]
    fn chips_die_with_their_target_entity() {
        let mut world = world_with_observers();
        let target = world.spawn(ObjectiveMarkerTarget::new("CRATE")).id();
        world.flush();
        assert_eq!(chips(&mut world).len(), 1);

        world.entity_mut(target).despawn();
        world.flush();

        assert!(
            chips(&mut world).is_empty(),
            "the chip dies with its target"
        );
    }

    /// The label shows "LABEL  <distance>m" with a player present, label
    /// alone without one (death gap).
    #[test]
    fn labels_show_label_and_distance() {
        let mut world = world_with_observers();
        let target = world
            .spawn((
                ObjectiveMarkerTarget::new("BEACON 3"),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -420.0)),
            ))
            .id();
        world.flush();

        // No player yet: label alone.
        world
            .run_system_once(update_objective_marker_labels)
            .unwrap();
        let label_text = |world: &mut World| -> String {
            world
                .query_filtered::<&Text, With<ObjectiveMarkerChipLabelMarker>>()
                .iter(world)
                .next()
                .unwrap()
                .0
                .clone()
        };
        assert_eq!(label_text(&mut world), "BEACON 3");

        world.spawn((
            PlayerSpaceshipMarker,
            GlobalTransform::from_translation(Vec3::ZERO),
        ));
        world
            .run_system_once(update_objective_marker_labels)
            .unwrap();
        assert_eq!(label_text(&mut world), "BEACON 3  420m");
        let _ = target;
    }

    /// The label text does NOT breathe and carries a contrast shadow;
    /// the diamond glyph carries the motion (playtest 2026-07-12, task
    /// 20260712-152340 - 12 px gold at 0.7 alpha over a bright planetoid
    /// was unreadable).
    #[test]
    fn label_stays_full_alpha_while_glyphs_breathe() {
        let mut world = world_with_observers();
        world.spawn(ObjectiveMarkerTarget::new("BEACON 1"));
        world.flush();

        // Advance the wave off its spawn value, then run the breath. An
        // eighth period - a quarter period lands exactly on the crest,
        // whose alpha factor is 1.0, indistinguishable from spawn.
        let mut time: Time = Time::default();
        time.advance_by(std::time::Duration::from_secs_f32(BREATH_PERIOD_SECS / 8.0));
        world.insert_resource(time);
        world.run_system_once(breathe_objective_markers).unwrap();

        let (label_color, has_shadow) = {
            let mut q = world.query_filtered::<(&TextColor, Option<&TextShadow>), With<ObjectiveMarkerChipLabelMarker>>();
            let (color, shadow) = q.iter(&world).next().expect("label exists");
            (color.0, shadow.is_some())
        };
        assert_eq!(
            label_color, OBJECTIVE_GOLD,
            "the label stays at constant full gold"
        );
        assert!(has_shadow, "the label carries a contrast shadow");

        // Delivery guard: the diamond DID breathe (a no-op system would
        // pass the label assert vacuously).
        let diamond_alpha = {
            let mut q = world.query::<(&BorderColor, &Name)>();
            q.iter(&world)
                .find(|(_, name)| name.as_str() == "ObjectiveMarkerDiamond")
                .expect("diamond exists")
                .0
                .top
                .alpha()
        };
        assert!(
            (diamond_alpha - OBJECTIVE_GOLD.alpha()).abs() > 1e-3,
            "the diamond's border alpha moved off the spawn value ({diamond_alpha})"
        );
    }

    /// The breath wave stays inside its authored band and actually moves -
    /// a flat wave would mean the pulse is decorative dead code.
    #[test]
    fn breath_alpha_sweeps_its_band() {
        let mut lowest = f32::MAX;
        let mut highest = f32::MIN;
        for i in 0..100 {
            let alpha = breath_alpha(i as f32 * BREATH_PERIOD_SECS / 100.0);
            assert!((BREATH_ALPHA_MIN..=BREATH_ALPHA_MAX).contains(&alpha));
            lowest = lowest.min(alpha);
            highest = highest.max(alpha);
        }
        assert!(
            highest - lowest > 0.8 * (BREATH_ALPHA_MAX - BREATH_ALPHA_MIN),
            "one period sweeps (nearly) the whole band, got [{lowest}, {highest}]"
        );
    }
}
