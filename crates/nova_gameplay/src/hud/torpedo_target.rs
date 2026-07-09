//! The torpedo-lock reticle and the locked-target info readout: a
//! screen-projected indicator on the entity the player's aim-assist currently
//! locks (`SpaceshipPlayerTargetLock`), with distance, closing speed
//! and a health bar riding its edge (tasks 20260708-165700 / 165702).
//!
//! A thin consumer of the [`screen_indicator`](super::screen_indicator)
//! widget: the widget owns projection, sizing and visibility; this module
//! spawns the reticle, drives its anchor from the lock resource, and fills
//! the readout content. The readout is a child of the reticle node at
//! `left: 100%`, so UI layout keeps it on the reticle's scaled edge and
//! visibility inheritance hides it with the reticle - no projection or
//! visibility code of its own.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::prelude::*;

/// Minimum on-screen size (px) of the target reticle. This is its historical
/// fixed size: the reticle grows to match larger targets but never shrinks
/// below this, so small or distant targets still show a clearly visible,
/// clickable marker.
const MIN_RETICLE_PX: f32 = 32.0;

/// Font size (px) of the readout lines, matching the flight-status readout.
const READOUT_FONT_PX: f32 = 14.0;

/// Gap (px) between the reticle edge and the readout column.
const READOUT_GAP_PX: f32 = 8.0;

/// Health bar size (px): a small underline below the text lines.
const HEALTH_BAR_SIZE: Vec2 = Vec2::new(64.0, 6.0);

/// Health bar backdrop (the "missing health" part).
const HEALTH_BAR_BACKDROP: Color = Color::srgba(0.15, 0.15, 0.15, 0.8);

/// Focus meter size (px): a thin underline below the reticle that fills
/// while the focus dwell accumulates (component-lock arc, task
/// 20260709-192523).
const FOCUS_METER_SIZE: Vec2 = Vec2::new(48.0, 4.0);

/// Focus meter backdrop.
const FOCUS_METER_BACKDROP: Color = Color::srgba(0.15, 0.15, 0.15, 0.8);

/// Focus meter fill: hot-metal red, matching the component markers it
/// unlocks.
const FOCUS_METER_COLOR: Color = Color::srgba(1.0, 0.4, 0.25, 0.9);

// Reticle tint by relation to the locked target (task 20260708-203708):
// hostile reads as a threat, your own torpedo as friendly, everything else
// (asteroids, neutral ships) stays plain white.
const RETICLE_HOSTILE_COLOR: Color = Color::srgba(1.0, 0.35, 0.3, 1.0);
const RETICLE_OWN_COLOR: Color = Color::srgba(0.35, 0.9, 0.55, 1.0);
const RETICLE_NEUTRAL_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);

pub mod prelude {
    pub use super::{
        torpedo_target_hud, TorpedoTargetFocusFillMarker, TorpedoTargetFocusMeterMarker,
        TorpedoTargetHealthBarMarker, TorpedoTargetHealthFillMarker, TorpedoTargetHudConfig,
        TorpedoTargetHudMarker, TorpedoTargetHudPlugin, TorpedoTargetReadoutLine,
        TorpedoTargetReadoutMarker, TorpedoTargetReticleMarker,
    };
}

/// Marker for the full-screen reticle layer (the root the HUD setup spawns).
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetHudMarker;

/// Marker for the reticle indicator node itself. Public so other HUD pieces
/// (e.g. the locked-target readout) can attach content to it.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetReticleMarker;

/// Marker for the readout column riding the reticle's right edge.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetReadoutMarker;

/// Which readout line a `Text` node shows. One enum component instead of one
/// marker type per line, so a single query updates all lines without filter
/// gymnastics.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum TorpedoTargetReadoutLine {
    /// `DST {:5.0}m` - range to the locked target.
    Distance,
    /// `CLS {:+5.1} u/s` - closing speed, positive when approaching.
    ClosingSpeed,
}

/// Marker for the health bar backdrop node.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetHealthBarMarker;

/// Marker for the health bar fill node (width = health fraction).
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetHealthFillMarker;

/// Marker for the focus meter backdrop below the reticle.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetFocusMeterMarker;

/// Marker for the focus meter fill node (width = focus fraction).
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetFocusFillMarker;

#[derive(Clone, Debug, Default)]
pub struct TorpedoTargetHudConfig {
    pub target_sprite: Handle<Image>,
}

/// UI bundle for the torpedo-lock reticle: a full-screen click-through layer
/// whose child is a screen-projected indicator sized to the locked target's
/// on-screen extent, carrying the info readout on its right edge.
pub fn torpedo_target_hud(config: TorpedoTargetHudConfig) -> impl Bundle {
    debug!("torpedo_target_hud: config {:?}", config);

    (
        Name::new("TorpedoTargetHUD"),
        TorpedoTargetHudMarker,
        screen_indicator_layer(),
        children![(
            Name::new("TorpedoTargetReticle"),
            TorpedoTargetReticleMarker,
            screen_indicator(ScreenIndicatorConfig {
                anchor: None,
                size: ScreenIndicatorSize::ApparentSize {
                    min_px: MIN_RETICLE_PX,
                },
                offset: Vec2::ZERO,
                offscreen: ScreenIndicatorOffscreen::Hide,
            }),
            ImageNode::new(config.target_sprite.clone()),
            children![
                (
                    Name::new("TorpedoTargetFocusMeter"),
                    TorpedoTargetFocusMeterMarker,
                    Node {
                        position_type: PositionType::Absolute,
                        // Centered under the reticle, tracking its scaled edge
                        // via UI layout like the readout does.
                        top: Val::Percent(100.0),
                        left: Val::Percent(50.0),
                        margin: UiRect {
                            left: Val::Px(-FOCUS_METER_SIZE.x / 2.0),
                            top: Val::Px(4.0),
                            ..default()
                        },
                        width: Val::Px(FOCUS_METER_SIZE.x),
                        height: Val::Px(FOCUS_METER_SIZE.y),
                        ..default()
                    },
                    BackgroundColor(FOCUS_METER_BACKDROP),
                    Pickable::IGNORE,
                    Visibility::Hidden,
                    children![(
                        Name::new("TorpedoTargetFocusFill"),
                        TorpedoTargetFocusFillMarker,
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(FOCUS_METER_COLOR),
                        Pickable::IGNORE,
                    )],
                ),
                (
                    Name::new("TorpedoTargetReadout"),
                    TorpedoTargetReadoutMarker,
                    Node {
                        position_type: PositionType::Absolute,
                        // Riding the reticle's right edge: `left: 100%` of the
                        // reticle node tracks its ApparentSize scaling for free.
                        left: Val::Percent(100.0),
                        top: Val::Px(0.0),
                        margin: UiRect::left(Val::Px(READOUT_GAP_PX)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(2.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                    children![
                        (
                            Name::new("TorpedoTargetReadoutDistance"),
                            TorpedoTargetReadoutLine::Distance,
                            Text::new(""),
                            TextFont::from_font_size(READOUT_FONT_PX),
                            Pickable::IGNORE,
                        ),
                        (
                            Name::new("TorpedoTargetReadoutClosing"),
                            TorpedoTargetReadoutLine::ClosingSpeed,
                            Text::new(""),
                            TextFont::from_font_size(READOUT_FONT_PX),
                            Pickable::IGNORE,
                        ),
                        (
                            Name::new("TorpedoTargetHealthBar"),
                            TorpedoTargetHealthBarMarker,
                            Node {
                                width: Val::Px(HEALTH_BAR_SIZE.x),
                                height: Val::Px(HEALTH_BAR_SIZE.y),
                                ..default()
                            },
                            BackgroundColor(HEALTH_BAR_BACKDROP),
                            Pickable::IGNORE,
                            children![(
                                Name::new("TorpedoTargetHealthFill"),
                                TorpedoTargetHealthFillMarker,
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(health_color(1.0)),
                                Pickable::IGNORE,
                            )],
                        ),
                    ],
                )
            ],
        )],
    )
}

#[derive(Default)]
pub struct TorpedoTargetHudPlugin;

impl Plugin for TorpedoTargetHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                drive_reticle_anchor.before(ScreenIndicatorSystems),
                update_reticle_relation_tint,
                update_target_readout,
                update_focus_meter,
            )
                .in_set(super::NovaHudSystems),
        );
    }
}

/// Point the reticle indicator at the current lock; `None` (no lock) hides it
/// via the widget's anchor handling.
fn drive_reticle_anchor(
    res_target: Res<SpaceshipPlayerTargetLock>,
    mut q_reticle: Query<&mut ScreenIndicatorAnchor, With<TorpedoTargetReticleMarker>>,
) {
    for mut anchor in &mut q_reticle {
        **anchor = (**res_target).map(ScreenIndicatorAnchorKind::Entity);
    }
}

/// The reticle tint for a relation to the locked target.
fn reticle_color(relation: Relation) -> Color {
    match relation {
        Relation::Hostile => RETICLE_HOSTILE_COLOR,
        Relation::Own => RETICLE_OWN_COLOR,
        Relation::Neutral => RETICLE_NEUTRAL_COLOR,
    }
}

/// Tint the reticle sprite by the locked target's relation to the player:
/// hostile vs your-own(-torpedo) vs neutral. With no lock the tint is left
/// alone - the widget hides the reticle anyway.
fn update_reticle_relation_tint(
    res_target: Res<SpaceshipPlayerTargetLock>,
    player: Single<Option<&Allegiance>, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    q_allegiance: Query<Option<&Allegiance>>,
    mut q_reticle: Query<&mut ImageNode, With<TorpedoTargetReticleMarker>>,
) {
    let Some(target) = **res_target else {
        return;
    };
    let Ok(target_allegiance) = q_allegiance.get(target) else {
        // The lock can outlive its entity by a frame (see the readout).
        return;
    };

    let color = reticle_color(relation(*player, target_allegiance));
    for mut image in &mut q_reticle {
        if image.color != color {
            image.color = color;
        }
    }
}

/// Closing speed (positive when approaching) of `target` relative to `ship`
/// along the line of sight, or `None` when the two positions coincide (no
/// line of sight to project onto).
fn closing_speed(
    ship_pos: Vec3,
    ship_vel: Vec3,
    target_pos: Vec3,
    target_vel: Vec3,
) -> Option<f32> {
    let los_dir = (target_pos - ship_pos).try_normalize()?;
    Some(-(target_vel - ship_vel).dot(los_dir))
}

/// The `DST` line, formatted like the flight-status distances (`{:5.0}m`).
fn distance_line(distance: f32) -> String {
    format!("DST {distance:5.0}m")
}

/// The `CLS` line, formatted like the flight-status speeds (`{:5.1} u/s`),
/// with an explicit sign: positive closing, negative opening. `None` (no
/// velocity data on either body) renders a placeholder.
fn closing_line(closing: Option<f32>) -> String {
    match closing {
        Some(closing) => format!("CLS {closing:+5.1} u/s"),
        None => "CLS   ---".to_string(),
    }
}

/// Health as a fraction in [0, 1]; a non-positive `max` reads as empty.
fn health_fraction(health: &Health) -> f32 {
    if health.max <= 0.0 {
        return 0.0;
    }
    (health.current / health.max).clamp(0.0, 1.0)
}

/// Fill color for a health fraction: green at full, through amber, to red
/// near death.
fn health_color(fraction: f32) -> Color {
    let fraction = fraction.clamp(0.0, 1.0);
    Color::srgba(1.0 - fraction * 0.8, 0.2 + fraction * 0.7, 0.15, 0.9)
}

/// Fill the readout from the locked target: distance and closing speed from
/// the transforms and `LinearVelocity`s, health bar from the target root's
/// `Health`. Degrades gracefully: missing velocity on either body blanks the
/// closing line, a target without `Health` hides the bar. With no lock the
/// readout is not updated at all - it is a child of the reticle indicator,
/// which the widget already hides.
#[allow(clippy::type_complexity)]
fn update_target_readout(
    res_target: Res<SpaceshipPlayerTargetLock>,
    ship: Single<
        (&GlobalTransform, Option<&LinearVelocity>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_target: Query<(&GlobalTransform, Option<&LinearVelocity>, Option<&Health>)>,
    mut q_lines: Query<(&mut Text, &TorpedoTargetReadoutLine)>,
    mut q_bar: Query<&mut Visibility, With<TorpedoTargetHealthBarMarker>>,
    mut q_fill: Query<(&mut Node, &mut BackgroundColor), With<TorpedoTargetHealthFillMarker>>,
) {
    let Some(target) = **res_target else {
        return;
    };
    let Ok((target_transform, target_vel, target_health)) = q_target.get(target) else {
        // The lock can outlive its entity by a frame; the reticle (and the
        // readout with it) is already hidden by the widget.
        return;
    };
    let (ship_transform, ship_vel) = ship.into_inner();

    let ship_pos = ship_transform.translation();
    let target_pos = target_transform.translation();
    let distance = ship_pos.distance(target_pos);
    let closing = match (ship_vel, target_vel) {
        (Some(ship_vel), Some(target_vel)) => {
            closing_speed(ship_pos, **ship_vel, target_pos, **target_vel)
        }
        _ => None,
    };

    for (mut text, line) in &mut q_lines {
        let content = match line {
            TorpedoTargetReadoutLine::Distance => distance_line(distance),
            TorpedoTargetReadoutLine::ClosingSpeed => closing_line(closing),
        };
        if **text != content {
            **text = content;
        }
    }

    for mut visibility in &mut q_bar {
        visibility.set_if_neq(match target_health {
            Some(_) => Visibility::Inherited,
            None => Visibility::Hidden,
        });
    }
    if let Some(health) = target_health {
        let fraction = health_fraction(health);
        for (mut node, mut color) in &mut q_fill {
            let width = Val::Percent(fraction * 100.0);
            if node.width != width {
                node.width = width;
            }
            let fill_color = health_color(fraction);
            if color.0 != fill_color {
                color.0 = fill_color;
            }
        }
    }
}

/// Drive the focus meter: visible with a partial fill while a lock is held
/// and the dwell is still accumulating, gone before a lock exists and once
/// focus completes (the component markers appearing is the completion
/// signal).
fn update_focus_meter(
    lock: Res<SpaceshipPlayerTargetLock>,
    focus: Res<SpaceshipPlayerLockFocus>,
    mut q_meter: Query<&mut Visibility, With<TorpedoTargetFocusMeterMarker>>,
    mut q_fill: Query<&mut Node, With<TorpedoTargetFocusFillMarker>>,
) {
    let filling =
        matches!(**lock, Some(target) if focus.target == Some(target) && !focus.focused_on(target));

    for mut visibility in &mut q_meter {
        visibility.set_if_neq(if filling {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    if filling {
        let width = Val::Percent(focus.fraction() * 100.0);
        for mut node in &mut q_fill {
            if node.width != width {
                node.width = width;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn torpedo_target_hud_spawns_the_reticle_indicator() {
        let mut world = World::new();
        let layer = world
            .spawn(torpedo_target_hud(TorpedoTargetHudConfig::default()))
            .id();

        let children = world
            .entity(layer)
            .get::<Children>()
            .expect("layer has the reticle child");
        assert_eq!(children.len(), 1);
        let reticle = world.entity(children[0]);
        assert!(reticle.contains::<TorpedoTargetReticleMarker>());
        assert!(reticle.contains::<ScreenIndicatorMarker>());
        assert_eq!(
            **reticle.get::<ScreenIndicatorAnchor>().unwrap(),
            None,
            "the reticle starts unanchored (hidden) until a lock exists"
        );
    }

    #[test]
    fn readout_rides_the_reticle_node() {
        // The readout must be a child of the reticle indicator (not the
        // layer): that is what makes it track the scaled edge and inherit
        // the reticle's visibility.
        let mut world = World::new();
        world.spawn(torpedo_target_hud(TorpedoTargetHudConfig::default()));

        let ChildOf(parent) = world
            .query_filtered::<&ChildOf, With<TorpedoTargetReadoutMarker>>()
            .iter(&world)
            .next()
            .expect("readout spawned");
        assert!(world
            .entity(*parent)
            .contains::<TorpedoTargetReticleMarker>());

        let lines = world
            .query::<&TorpedoTargetReadoutLine>()
            .iter(&world)
            .count();
        assert_eq!(lines, 2, "distance and closing-speed lines");
    }

    #[test]
    fn reticle_anchor_follows_the_lock_resource() {
        let mut world = World::new();
        world.insert_resource(SpaceshipPlayerTargetLock(None));
        let reticle = world
            .spawn((
                TorpedoTargetReticleMarker,
                screen_indicator(ScreenIndicatorConfig::default()),
            ))
            .id();

        world.run_system_once(drive_reticle_anchor).unwrap();
        assert_eq!(
            **world
                .entity(reticle)
                .get::<ScreenIndicatorAnchor>()
                .unwrap(),
            None
        );

        let target = world.spawn_empty().id();
        world.insert_resource(SpaceshipPlayerTargetLock(Some(target)));
        world.run_system_once(drive_reticle_anchor).unwrap();
        assert_eq!(
            **world
                .entity(reticle)
                .get::<ScreenIndicatorAnchor>()
                .unwrap(),
            Some(ScreenIndicatorAnchorKind::Entity(target))
        );

        world.insert_resource(SpaceshipPlayerTargetLock(None));
        world.run_system_once(drive_reticle_anchor).unwrap();
        assert_eq!(
            **world
                .entity(reticle)
                .get::<ScreenIndicatorAnchor>()
                .unwrap(),
            None,
            "dropping the lock clears the anchor so the widget hides the reticle"
        );
    }

    // -- readout math and formatting --

    #[test]
    fn closing_speed_sign_convention() {
        let ship = Vec3::ZERO;
        let target = Vec3::new(0.0, 0.0, -100.0);
        // Target flying toward the ship (+z): closing, positive.
        assert_eq!(
            closing_speed(ship, Vec3::ZERO, target, Vec3::new(0.0, 0.0, 50.0)),
            Some(50.0)
        );
        // Target flying away (-z): opening, negative.
        assert_eq!(
            closing_speed(ship, Vec3::ZERO, target, Vec3::new(0.0, 0.0, -50.0)),
            Some(-50.0)
        );
        // Pure crossing motion: no closing component.
        assert_eq!(
            closing_speed(ship, Vec3::ZERO, target, Vec3::new(50.0, 0.0, 0.0)),
            Some(0.0)
        );
        // The ship chasing the target closes too.
        assert_eq!(
            closing_speed(ship, Vec3::new(0.0, 0.0, -30.0), target, Vec3::ZERO),
            Some(30.0)
        );
        // Coincident positions: no line of sight.
        assert_eq!(closing_speed(ship, Vec3::ZERO, ship, Vec3::ZERO), None);
    }

    #[test]
    fn readout_lines_format_like_the_flight_status() {
        assert_eq!(distance_line(150.4), "DST   150m");
        assert_eq!(distance_line(1234.6), "DST  1235m");
        assert_eq!(closing_line(Some(12.34)), "CLS +12.3 u/s");
        assert_eq!(closing_line(Some(-3.21)), "CLS  -3.2 u/s");
        assert_eq!(closing_line(None), "CLS   ---");
    }

    #[test]
    fn health_fraction_clamps() {
        let health = |current, max| Health { current, max };
        assert_eq!(health_fraction(&health(50.0, 100.0)), 0.5);
        assert_eq!(health_fraction(&health(150.0, 100.0)), 1.0);
        assert_eq!(health_fraction(&health(-5.0, 100.0)), 0.0);
        assert_eq!(health_fraction(&health(5.0, 0.0)), 0.0);
    }

    // -- readout system behavior --

    fn spawn_readout_world(world: &mut World) {
        world.spawn(torpedo_target_hud(TorpedoTargetHudConfig::default()));
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::IDENTITY,
            LinearVelocity(Vec3::ZERO),
        ));
    }

    fn line_text(world: &mut World, which: TorpedoTargetReadoutLine) -> String {
        world
            .query::<(&Text, &TorpedoTargetReadoutLine)>()
            .iter(world)
            .find(|(_, line)| **line == which)
            .map(|(text, _)| text.0.clone())
            .expect("line exists")
    }

    #[test]
    fn readout_fills_from_the_locked_target() {
        let mut world = World::new();
        spawn_readout_world(&mut world);
        // 150 m dead ahead, flying toward the ship at 20 u/s, half health.
        let target = world
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -150.0)),
                LinearVelocity(Vec3::new(0.0, 0.0, 20.0)),
                Health {
                    current: 50.0,
                    max: 100.0,
                },
            ))
            .id();
        world.insert_resource(SpaceshipPlayerTargetLock(Some(target)));

        world.run_system_once(update_target_readout).unwrap();

        assert_eq!(
            line_text(&mut world, TorpedoTargetReadoutLine::Distance),
            "DST   150m"
        );
        assert_eq!(
            line_text(&mut world, TorpedoTargetReadoutLine::ClosingSpeed),
            "CLS +20.0 u/s"
        );
        let bar_visibility = *world
            .query_filtered::<&Visibility, With<TorpedoTargetHealthBarMarker>>()
            .iter(&world)
            .next()
            .expect("bar exists");
        assert_eq!(bar_visibility, Visibility::Inherited);
        let fill = world
            .query_filtered::<&Node, With<TorpedoTargetHealthFillMarker>>()
            .iter(&world)
            .next()
            .expect("fill exists");
        assert_eq!(fill.width, Val::Percent(50.0));
    }

    #[test]
    fn readout_degrades_without_velocity_or_health() {
        let mut world = World::new();
        spawn_readout_world(&mut world);
        // A bare drifting body: transform only, no velocity, no health.
        let target = world
            .spawn(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -80.0,
            )))
            .id();
        world.insert_resource(SpaceshipPlayerTargetLock(Some(target)));

        world.run_system_once(update_target_readout).unwrap();

        assert_eq!(
            line_text(&mut world, TorpedoTargetReadoutLine::Distance),
            "DST    80m"
        );
        assert_eq!(
            line_text(&mut world, TorpedoTargetReadoutLine::ClosingSpeed),
            "CLS   ---"
        );
        let bar_visibility = *world
            .query_filtered::<&Visibility, With<TorpedoTargetHealthBarMarker>>()
            .iter(&world)
            .next()
            .expect("bar exists");
        assert_eq!(bar_visibility, Visibility::Hidden);
    }

    // -- focus meter --

    fn meter_state(world: &mut World) -> (Visibility, Val) {
        let visibility = *world
            .query_filtered::<&Visibility, With<TorpedoTargetFocusMeterMarker>>()
            .iter(world)
            .next()
            .expect("meter exists");
        let width = world
            .query_filtered::<&Node, With<TorpedoTargetFocusFillMarker>>()
            .iter(world)
            .next()
            .expect("fill exists")
            .width;
        (visibility, width)
    }

    #[test]
    fn focus_meter_fills_while_the_dwell_accumulates() {
        let mut world = World::new();
        world.spawn(torpedo_target_hud(TorpedoTargetHudConfig::default()));
        let target = world.spawn_empty().id();
        world.insert_resource(SpaceshipPlayerTargetLock(Some(target)));
        // Halfway through the dwell.
        let mut focus = SpaceshipPlayerLockFocus::default();
        focus.target = Some(target);
        world.insert_resource(focus);
        let half = {
            let mut focus = world.resource_mut::<SpaceshipPlayerLockFocus>();
            // fraction() is defined by FOCUS_TIME internally; drive through
            // the public API by finding the seconds that yield 0.5.
            focus.seconds = 0.0;
            let mut lo = 0.0_f32;
            let mut hi = 60.0_f32;
            for _ in 0..40 {
                let mid = (lo + hi) / 2.0;
                focus.seconds = mid;
                if focus.fraction() < 0.5 {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            focus.seconds
        };
        let _ = half;

        world.run_system_once(update_focus_meter).unwrap();

        let (visibility, width) = meter_state(&mut world);
        assert_eq!(visibility, Visibility::Inherited);
        match width {
            Val::Percent(percent) => {
                assert!((percent - 50.0).abs() < 1.0, "width {percent}")
            }
            other => panic!("expected Val::Percent, got {other:?}"),
        }
    }

    #[test]
    fn focus_meter_hides_without_a_lock_and_once_focused() {
        let mut world = World::new();
        world.spawn(torpedo_target_hud(TorpedoTargetHudConfig::default()));
        world.insert_resource(SpaceshipPlayerTargetLock(None));
        world.insert_resource(SpaceshipPlayerLockFocus::default());

        world.run_system_once(update_focus_meter).unwrap();
        assert_eq!(meter_state(&mut world).0, Visibility::Hidden);

        // Focused: the meter yields to the component markers.
        let target = world.spawn_empty().id();
        world.insert_resource(SpaceshipPlayerTargetLock(Some(target)));
        world.insert_resource(SpaceshipPlayerLockFocus {
            target: Some(target),
            seconds: f32::MAX,
        });
        world.run_system_once(update_focus_meter).unwrap();
        assert_eq!(meter_state(&mut world).0, Visibility::Hidden);
    }

    // -- reticle relation tint (task 20260708-203708) --

    fn reticle_tint(world: &mut World) -> Color {
        world
            .query_filtered::<&ImageNode, With<TorpedoTargetReticleMarker>>()
            .iter(world)
            .next()
            .expect("reticle exists")
            .color
    }

    #[test]
    fn reticle_tints_by_relation_to_the_locked_target() {
        let mut world = World::new();
        world.spawn(torpedo_target_hud(TorpedoTargetHudConfig::default()));
        // The player marker requires Allegiance::Player, so the rig gets the
        // relation model's player side for free.
        world.spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));

        // Hostile lock: an enemy-aligned body.
        let enemy = world.spawn(Allegiance::Enemy).id();
        world.insert_resource(SpaceshipPlayerTargetLock(Some(enemy)));
        world.run_system_once(update_reticle_relation_tint).unwrap();
        assert_eq!(reticle_tint(&mut world), RETICLE_HOSTILE_COLOR);

        // Own lock: e.g. the player's own loitering torpedo.
        let own_torpedo = world.spawn(Allegiance::Player).id();
        world.insert_resource(SpaceshipPlayerTargetLock(Some(own_torpedo)));
        world.run_system_once(update_reticle_relation_tint).unwrap();
        assert_eq!(reticle_tint(&mut world), RETICLE_OWN_COLOR);

        // Neutral lock: an unmarked body (asteroid).
        let asteroid = world.spawn_empty().id();
        world.insert_resource(SpaceshipPlayerTargetLock(Some(asteroid)));
        world.run_system_once(update_reticle_relation_tint).unwrap();
        assert_eq!(reticle_tint(&mut world), RETICLE_NEUTRAL_COLOR);
    }
}
