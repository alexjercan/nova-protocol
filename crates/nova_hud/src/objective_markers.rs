//! Objective marker HUD chips: one gold chip per [`ObjectiveMarkerTarget`]
//! entity - the marker's label plus live distance to the player ship - with
//! the indicator widget's `ClampToEdge` path, so an off-screen objective's chip
//! pins to the viewport edge and its chevron points at it. Where the nav beacon
//! chip says "a waypoint exists", this chip says "go HERE now": gold, a diamond
//! glyph instead of the beacon dot language, and a slow alpha breath so it
//! reads in peripheral vision without strobing.
//!
//! The full chip leads the player to the target, then settles. While the
//! target is out of view the full chip stays at the edge. In view it shows for
//! [`CHIP_OPEN_SECS`] after the tag lands, after its label changes, or after
//! the target comes back from [`REOPEN_AWAY_SECS`] or more out of view; then
//! only the gold diamond stays, standing on the target above its allegiance
//! triangle. A lock does not reopen it: the lock readout carries the range and
//! the target inset the name.
//!
//! The chip SHAPE - layer, pill, label leaf, chevron, despawn and the
//! label-plus-distance walk - is [`anchored_chip`](super::anchored_chip)'s,
//! shared with the beacon chips. What is the objective's own lives here: the
//! gold tone and geometry, the diamond identity glyph and its settled mark, the
//! dwell, and the breath.
//!
//! Chrome tier, like the beacon chips - the same nav-chip family.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_ui::{hud::ChipTone, widget::metrics};

use super::{
    allegiance_markers::MARKER_TOP_CLEARANCE, anchored_chip::prelude::*,
    hud_notice::NOTICE_DWELL_SECS, screen_indicator::prelude::*, OBJECTIVE_GOLD,
};

/// The objective marker chip layer marker, its shown look,
/// `ObjectiveMarkersHudPlugin` and `ObjectiveChipSystems`.
pub mod prelude {
    pub use super::{
        ObjectiveChipSystems, ObjectiveChipType, ObjectiveMarkerChipHudMarker,
        ObjectiveMarkersHudPlugin,
    };
}

/// How the chip stands off its target so the label never sits on the mesh.
///
/// A marked target is whatever the scenario points at - a crate, a carrier, a
/// planetoid - so the push is measured from the target's own projected
/// silhouette. The floor is the 36 px the chip has always floated at, which is
/// the composition on a small or distant mark.
const CHIP_CLEARANCE: ScreenIndicatorClearance = ScreenIndicatorClearance {
    direction: Vec2::NEG_Y,
    gap_px: 14.0,
    min_px: 36.0,
};

/// The objective chip's chevron: the beacon chip's, in objective gold.
const CHEVRON: AnchoredChipChevron = AnchoredChipChevron {
    arrow_px: 16.0,
    stroke_len_px: 11.0,
    stroke_thick_px: 2.0,
    color: OBJECTIVE_GOLD,
};

/// The settled diamond: a square border rotated 45 degrees, standing on the
/// target in view - the marker's identity mark.
const DIAMOND_PX: f32 = 12.0;
const DIAMOND_BORDER_PX: f32 = 2.0;

/// The same diamond as a glyph inside the chip, left of the label. Turned 45
/// degrees, its diagonal is the label's font size, so the glyph stands as
/// tall as the text and never sets the chip's height.
const GLYPH_PX: f32 = metrics::TEXT_CHIP / std::f32::consts::SQRT_2;

/// Half the drawn height of the diamond: a square turned 45 degrees reaches
/// half its diagonal above and below its centre.
const DIAMOND_REACH_PX: f32 = DIAMOND_PX * std::f32::consts::SQRT_2 / 2.0;

/// How the settled diamond stands off its target: a small gap above the
/// allegiance triangle, so on a ship the two read as one stack and neither
/// covers the other. A body with no triangle (a station, a beacon) keeps the
/// same height.
const MARK_CLEARANCE: ScreenIndicatorClearance = ScreenIndicatorClearance {
    direction: MARKER_TOP_CLEARANCE.direction,
    gap_px: MARKER_TOP_CLEARANCE.gap_px + metrics::GAP_S + DIAMOND_REACH_PX - DIAMOND_PX / 2.0,
    min_px: MARKER_TOP_CLEARANCE.min_px + metrics::GAP_S + DIAMOND_REACH_PX,
};

/// How long the full chip shows on a target in view before it settles to the
/// diamond: the HUD notice dwell, so short-lived HUD text keeps one rhythm.
const CHIP_OPEN_SECS: f32 = NOTICE_DWELL_SECS;

/// How long a target must stay out of view before its return reopens the full
/// chip. A ship that weaves along the screen edge does not reopen it on every
/// pass.
const REOPEN_AWAY_SECS: f32 = 1.0;

/// Alpha breath of the whole chip: slow and shallow - noticeable in
/// peripheral vision, not a strobe.
const BREATH_PERIOD_SECS: f32 = 1.25;
const BREATH_ALPHA_MIN: f32 = 0.7;
const BREATH_ALPHA_MAX: f32 = 1.0;

/// Marker for one objective marker chip layer (one per marked entity). The
/// family tag: every shared chip system is gated on it, so a query that walks
/// into an objective chip can never reach a beacon's.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ObjectiveMarkerChipHudMarker;

/// What an objective chip layer draws. Lives on the layer; written each frame
/// by the settle pass in [`ObjectiveChipSystems`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum ObjectiveChipType {
    /// The full chip: diamond, label and range in the gold pill. Always the
    /// look while the target is out of view, pinned to the edge.
    Full,
    /// The diamond alone, standing on the target in view.
    Mark,
}

/// The objective chip's settle pass, in PostUpdate after
/// [`ScreenIndicatorSystems`] (it reads whether each target is in view) and
/// before UI layout. A reader of [`ObjectiveChipType`] orders after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectiveChipSystems;

/// The dwell of one chip layer: which label it last opened for, how long the
/// full chip has left on a target in view, and how long the target has been
/// out of view.
#[derive(Component, Debug, Clone, Default)]
pub(crate) struct ObjectiveChipDwell {
    /// The tag label the chip last opened for; `None` until the first pass.
    label: Option<String>,
    /// Seconds the full chip still shows on a target in view.
    open_secs: f32,
    /// Seconds the target has been out of view; `None` while in view.
    away_secs: Option<f32>,
}

impl ObjectiveChipDwell {
    /// Advance the dwell by `dt` seconds for a tag reading `label` whose
    /// target is `in_view`, and answer what the chip draws.
    fn step(&mut self, label: &str, in_view: bool, dt: f32) -> ObjectiveChipType {
        self.open_secs = (self.open_secs - dt).max(0.0);
        if self.label.as_deref() != Some(label) {
            self.label = Some(label.to_owned());
            self.open_secs = CHIP_OPEN_SECS;
        }
        if !in_view {
            self.away_secs = Some(self.away_secs.unwrap_or(0.0) + dt);
            return ObjectiveChipType::Full;
        }
        if self
            .away_secs
            .take()
            .is_some_and(|away| away >= REOPEN_AWAY_SECS)
        {
            self.open_secs = CHIP_OPEN_SECS;
        }
        if self.open_secs > 0.0 {
            ObjectiveChipType::Full
        } else {
            ObjectiveChipType::Mark
        }
    }

    /// Whether the target is out of view, so the full chip leads from the
    /// screen edge.
    pub(crate) fn leads_from_the_edge(&self) -> bool {
        self.away_secs.is_some()
    }
}

/// Marker for the diamond identity glyph, so tests and future drivers can find
/// it without matching on its display [`Name`].
#[derive(Component, Debug, Clone, Reflect)]
struct ObjectiveMarkerDiamondMarker;

/// Marker for the settled diamond: an indicator of its own beside the chip, so
/// the chip and the mark each keep one fixed placement and the settle pass
/// only moves the anchor between them.
#[derive(Component, Debug, Clone, Reflect)]
struct ObjectiveMarkMarker;

/// Marker for a node whose color breathes with the chip: the diamond
/// border and the chevron strokes - NOT the label text, which stays at
/// full alpha for readability.
#[derive(Component, Debug, Clone, Reflect)]
struct ObjectiveMarkerBreathMarker;

impl AnchoredChipLabelSource for ObjectiveMarkerTarget {
    fn chip_label(&self) -> &str {
        &self.label
    }
}

/// UI bundle for one marked entity's chip layer: the full chip, which leads at
/// spawn, and the settled diamond, which waits with no anchor.
fn objective_marker_chip_hud(target: Entity) -> impl Bundle {
    (
        Name::new("ObjectiveMarkerChipHUD"),
        ObjectiveMarkerChipHudMarker,
        ObjectiveChipType::Full,
        ObjectiveChipDwell::default(),
        anchored_chip_layer(target),
        children![
            (
                Name::new("ObjectiveMarkerChipUI"),
                // The objective chip is the amber "do this now" member of the
                // chip family (demo 2 `.obj`).
                anchored_chip_node(
                    Some(target),
                    ChipTone::Amber,
                    CHIP_CLEARANCE,
                    ScreenLabelType::Objective,
                ),
                ScreenIndicatorAnchorInView::default(),
                children![
                    objective_marker_diamond(),
                    (
                        Name::new("ObjectiveMarkerLabel"),
                        // The LABEL does not breathe: translucent gold over a
                        // bright planetoid was unreadable. Constant full gold
                        // + a tight dark shadow for contrast; the diamond and
                        // chevron carry the motion.
                        anchored_chip_label(
                            metrics::TEXT_CHIP,
                            OBJECTIVE_GOLD,
                            TextShadow {
                                offset: Vec2::splat(1.0),
                                color: Color::srgba(0.0, 0.0, 0.0, 0.9),
                            },
                        ),
                    ),
                    (
                        Name::new("ObjectiveMarkerArrow"),
                        anchored_chip_chevron(CHEVRON, ObjectiveMarkerBreathMarker),
                    ),
                ],
            ),
            objective_mark(),
        ],
    )
}

/// The diamond's own node: a `side_px` square with a gold border, turned 45
/// degrees by the [`diamond_paint`] beside it.
fn diamond_node(side_px: f32) -> Node {
    Node {
        width: Val::Px(side_px),
        height: Val::Px(side_px),
        border: UiRect::all(Val::Px(DIAMOND_BORDER_PX)),
        flex_shrink: 0.0,
        ..default()
    }
}

/// The diamond's turn and paint, shared by the chip's glyph and the settled
/// mark. It breathes with the chevron.
fn diamond_paint() -> impl Bundle {
    (
        ObjectiveMarkerBreathMarker,
        UiTransform {
            rotation: Rot2::degrees(45.0),
            ..default()
        },
        BorderColor::all(OBJECTIVE_GOLD),
    )
}

/// The diamond identity glyph: a hollow square border rotated 45 degrees
/// (the same UiTransform trick as the chevron strokes), riding INSIDE the pill
/// as the first in-flow flex item - `chip_node`'s `column_gap` spaces it from
/// the label and its `align_items: Center` centres it, so the same fill and
/// border back the mark as back the text. In flow rather than absolute at
/// `left: -14 px`, which only reads as "attached" while the pill is a
/// collapsed slab at the same origin.
fn objective_marker_diamond() -> impl Bundle {
    (
        Name::new("ObjectiveMarkerDiamond"),
        ObjectiveMarkerDiamondMarker,
        diamond_node(GLYPH_PX),
        diamond_paint(),
        Pickable::IGNORE,
    )
}

/// The settled diamond: the same glyph as an indicator of its own, with no box
/// around it. It hides off screen, where the full chip leads, and takes no
/// part in the label fit: like the allegiance triangle it marks its body and
/// never moves off it.
fn objective_mark() -> impl Bundle {
    (
        Name::new("ObjectiveMark"),
        ObjectiveMarkMarker,
        ScreenIndicatorAnchorInView::default(),
        screen_indicator_node(
            ScreenIndicatorConfig {
                anchor: None,
                size: ScreenIndicatorSize::Fixed(Vec2::splat(DIAMOND_PX)),
                offset: Vec2::ZERO,
                offscreen: ScreenIndicatorOffscreen::Hide,
            },
            diamond_node(DIAMOND_PX),
        ),
        MARK_CLEARANCE,
        diamond_paint(),
    )
}

/// Draws one breathing gold chip (diamond glyph + label + edge chevron) per
/// [`ObjectiveMarkerTarget`] entity - the "do this now" objective cue (Chrome
/// tier) - and settles it to the diamond on a target in view.
/// Registers [`ObjectiveMarkerTarget`], adds the chip spawn/despawn observers,
/// runs the shared label updater plus `breathe_objective_markers` in Update
/// within [`super::NovaHudSystems`], and runs `settle_objective_chips` in
/// PostUpdate in [`ObjectiveChipSystems`].
#[derive(Default)]
pub struct ObjectiveMarkersHudPlugin;

impl Plugin for ObjectiveMarkersHudPlugin {
    fn build(&self, app: &mut App) {
        trace!("ObjectiveMarkersHudPlugin: build");

        app.register_type::<ObjectiveMarkerTarget>();
        app.register_type::<ObjectiveChipType>();

        app.add_observer(setup_objective_marker_chip);
        app.add_observer(
            despawn_anchored_chips::<ObjectiveMarkerTarget, ObjectiveMarkerChipHudMarker>,
        );
        app.add_systems(
            Update,
            (
                // The tag carries the name, so the label source needs no
                // further gate.
                update_anchored_chip_labels::<
                    ObjectiveMarkerChipHudMarker,
                    ObjectiveMarkerTarget,
                    (),
                >,
                breathe_objective_markers,
            )
                .in_set(super::NovaHudSystems),
        );
        // The settle reads the in-view flag the placement writes this frame.
        // Its anchor writes place the chip or the mark from the next frame.
        app.configure_sets(
            PostUpdate,
            ObjectiveChipSystems
                .after(ScreenIndicatorSystems)
                .before(bevy::ui::UiSystems::Layout),
        );
        app.add_systems(
            PostUpdate,
            settle_objective_chips.in_set(ObjectiveChipSystems),
        );
    }
}

/// Every marked entity grows its chip the moment the tag lands.
fn setup_objective_marker_chip(add: On<Add<ObjectiveMarkerTarget>>, mut commands: Commands) {
    let target = add.entity;
    trace!("setup_objective_marker_chip: target {:?}", target);
    commands.spawn(objective_marker_chip_hud(target));
}

/// Step each chip's dwell and hand the anchor to the full chip or to the
/// settled diamond.
///
/// The in-view flag is read from whichever node holds the anchor now, the one
/// the widget placed this frame. It is the TARGET's own projection, so it does
/// not change when the look does.
#[expect(
    clippy::type_complexity,
    reason = "one query for the layers, one for the nodes under them"
)]
fn settle_objective_chips(
    time: Res<Time>,
    mut q_layers: Query<
        (
            &AnchoredChipTarget,
            &Children,
            &mut ObjectiveChipDwell,
            &mut ObjectiveChipType,
        ),
        With<ObjectiveMarkerChipHudMarker>,
    >,
    q_tags: Query<&ObjectiveMarkerTarget>,
    mut q_nodes: Query<(
        &mut ScreenIndicatorAnchor,
        &ScreenIndicatorAnchorInView,
        Has<ObjectiveMarkMarker>,
    )>,
) {
    let dt = time.delta_secs();
    for (target, children, mut dwell, mut shown) in &mut q_layers {
        let Ok(tag) = q_tags.get(**target) else {
            continue;
        };
        let live_is_mark = *shown == ObjectiveChipType::Mark;
        let in_view = children
            .iter()
            .filter_map(|child| q_nodes.get(child).ok())
            .find(|(_, _, is_mark)| *is_mark == live_is_mark)
            .is_some_and(|(_, in_view, _)| **in_view);
        let next = dwell.step(&tag.label, in_view, dt);
        shown.set_if_neq(next);
        for child in children.iter() {
            let Ok((mut anchor, _, is_mark)) = q_nodes.get_mut(child) else {
                continue;
            };
            let live = is_mark == (next == ObjectiveChipType::Mark);
            let wanted = live.then_some(ScreenIndicatorAnchorKind::Entity(**target));
            if **anchor != wanted {
                **anchor = wanted;
            }
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
/// breathe: thinning the label to 0.7 alpha broke readability over
/// bright scene content; the
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
pub(crate) mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::{
        super::chip_layout_rig::{
            assert_child_sits_in_the_pill, assert_chip_backs_its_label, chip_layout_app, measure,
            only_descendant_with, settle,
        },
        *,
    };

    fn world_with_observers() -> World {
        let mut world = World::new();
        world.add_observer(setup_objective_marker_chip);
        world.add_observer(
            despawn_anchored_chips::<ObjectiveMarkerTarget, ObjectiveMarkerChipHudMarker>,
        );
        world
    }

    fn chips(world: &mut World) -> Vec<(Entity, Entity)> {
        world
            .query_filtered::<(Entity, &AnchoredChipTarget), With<ObjectiveMarkerChipHudMarker>>()
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

    /// The label shows "LABEL  <distance>" with a player present, label
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
            .run_system_once(
                update_anchored_chip_labels::<
                    ObjectiveMarkerChipHudMarker,
                    ObjectiveMarkerTarget,
                    (),
                >,
            )
            .unwrap();
        let label_text = |world: &mut World| -> String {
            world
                .query_filtered::<&Text, With<AnchoredChipLabelMarker>>()
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
            .run_system_once(
                update_anchored_chip_labels::<
                    ObjectiveMarkerChipHudMarker,
                    ObjectiveMarkerTarget,
                    (),
                >,
            )
            .unwrap();
        // 420 world units = 4200 m -> 4.20 km displayed.
        assert_eq!(label_text(&mut world), "BEACON 3  4.20 km");
        let _ = target;
    }

    /// The label text does NOT breathe and carries a contrast shadow;
    /// the diamond glyph carries the motion - translucent gold over a
    /// bright planetoid was unreadable.
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
            let mut q = world
                .query_filtered::<(&TextColor, Option<&TextShadow>), With<AnchoredChipLabelMarker>>(
                );
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

    /// Build the real chip through the real spawn observer and lay it out with
    /// the real taffy + text measurement, then hand back
    /// (layer, chip node, label node).
    fn laid_out_objective_chip(label: &str) -> (App, Entity, Entity, Entity) {
        let mut app = chip_layout_app();
        app.add_plugins(ObjectiveMarkersHudPlugin);
        app.world_mut().spawn((
            ObjectiveMarkerTarget::new(label),
            GlobalTransform::default(),
        ));
        settle(&mut app);

        let layer = app
            .world_mut()
            .query_filtered::<Entity, With<ObjectiveMarkerChipHudMarker>>()
            .iter(app.world())
            .next()
            .expect("the marker grew a chip layer");
        let chip = only_descendant_with::<AnchoredChipNodeMarker>(&mut app, layer);
        let text = only_descendant_with::<Text>(&mut app, layer);
        (app, layer, chip, text)
    }

    /// The reported bug: the chip's fill/border
    /// covered only a corner of "BEACON 1" instead of the whole label. Asserted
    /// against a live layout pass, comparing with an INDEPENDENT measurement of
    /// the same string - see `chip_layout_rig`.
    #[test]
    fn the_objective_chip_backs_its_whole_label() {
        let (mut app, _layer, chip, text) = laid_out_objective_chip("BEACON 1");
        assert_chip_backs_its_label(
            &mut app,
            chip,
            text,
            "BEACON 1",
            metrics::TEXT_CHIP,
            "objective marker chip",
        );
    }

    /// The diamond identity glyph rides INSIDE the pill: an in-flow flex item
    /// the chip's
    /// fill and border wrap, not an absolute glyph parked outside the box.
    #[test]
    fn the_objective_chips_diamond_sits_inside_the_pill() {
        let (mut app, layer, chip, _text) = laid_out_objective_chip("BEACON 1");
        let diamond = only_descendant_with::<ObjectiveMarkerDiamondMarker>(&mut app, layer);
        assert_child_sits_in_the_pill(&app, chip, diamond, "objective marker chip");
    }

    /// The chevron parks centred over the pill. Its `left` is a percentage of the
    /// real chip width: an offset from a collapsed 20x10 slab's origin hangs it
    /// off the left edge of a full-width pill.
    #[test]
    fn the_objective_chips_chevron_centres_over_the_pill() {
        let (mut app, layer, chip, _text) = laid_out_objective_chip("BEACON 1");
        let arrow = only_descendant_with::<ScreenIndicatorArrowMarker>(&mut app, layer);
        let chip_centre = measure(&app, chip).rect.center();
        let arrow_centre = measure(&app, arrow).rect.center();
        assert!(
            (arrow_centre.x - chip_centre.x).abs() <= 1.0,
            "the chevron centre {arrow_centre:?} is not over the chip centre {chip_centre:?}"
        );
    }

    /// A held job's chip whose target is behind the camera clamps to the
    /// bottom edge, where the key row and the notice box stand. It stands
    /// above both, at 100 % and at 150 % UI scale.
    #[test]
    fn the_job_chip_sits_above_the_key_row() {
        use std::f32::consts::FRAC_PI_2;

        use crate::{
            hud_notice::{NOTICE_BOTTOM_PX, NOTICE_MAX_HEIGHT_PX},
            keybind_dock::{keycap_sizing_tests::dock_app, prelude::KeybindDockMarker},
        };

        const VIEW: Vec2 = Vec2::new(1280.0, 720.0);
        for scale in [1.0, 1.5] {
            let (mut app, dock_chips) = dock_app();
            app.insert_resource(UiScale(scale));
            app.add_plugins((ScreenIndicatorPlugin, ObjectiveMarkersHudPlugin));
            // The rig runs no visibility propagation, and the column floor
            // counts only a dock that is on screen.
            let dock = app
                .world_mut()
                .query_filtered::<Entity, With<KeybindDockMarker>>()
                .single(app.world())
                .expect("the rig spawned one dock");
            app.world_mut()
                .entity_mut(dock)
                .insert(InheritedVisibility::VISIBLE);
            // The rig's camera projects the chips: at the origin, looking
            // down -Z, so the escort's hauler at +Z is behind it.
            let mut q_camera = app.world_mut().query::<(Entity, &mut Camera)>();
            let (camera, mut lens) = q_camera
                .single_mut(app.world_mut())
                .expect("the rig has one camera");
            lens.computed.clip_from_view =
                bevy::math::proj::perspective_infinite_reverse(FRAC_PI_2, VIEW.x / VIEW.y, 0.1);
            app.world_mut()
                .entity_mut(camera)
                .insert(ScreenIndicatorCamera);
            app.world_mut().spawn((
                ObjectiveMarkerTarget::new("JOB-1 Escort"),
                Transform::from_xyz(0.0, 0.0, 10.0),
            ));
            // The chip reads the dock's last layout, so its own place settles
            // a pass after the dock's.
            settle(&mut app);
            settle(&mut app);

            let layer = app
                .world_mut()
                .query_filtered::<Entity, With<ObjectiveMarkerChipHudMarker>>()
                .single(app.world())
                .expect("the marker grew a chip layer");
            let chip = only_descendant_with::<AnchoredChipNodeMarker>(&mut app, layer);
            let chip = measure(&app, chip).rect;
            let row_top = dock_chips
                .iter()
                .filter(|&&chip| app.world().get::<Node>(chip).unwrap().display != Display::None)
                .map(|&chip| measure(&app, chip).rect.min.y)
                .fold(f32::MAX, f32::min);
            assert!(
                row_top < VIEW.y,
                "delivery guard: the row draws chips at {scale}"
            );
            assert!(
                chip.max.y > VIEW.y / 2.0 && (chip.center().x - VIEW.x / 2.0).abs() <= 1.0,
                "delivery guard: the chip clamps to the bottom centre at {scale}: {chip:?}"
            );
            assert!(
                chip.max.y <= row_top,
                "the job chip prints through the key row at {scale}: {chip:?} vs row top {row_top}"
            );
            let notice_top = VIEW.y - (NOTICE_BOTTOM_PX + NOTICE_MAX_HEIGHT_PX) * scale;
            assert!(
                chip.max.y <= notice_top,
                "the job chip prints through the notice box at {scale}: {chip:?} vs {notice_top}"
            );
        }
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
    // -- the dwell, on a manual clock through the real placement --

    /// Virtual seconds each `app.update()` of the dwell rig advances.
    const STEP_SECS: f32 = 0.1;

    /// Straight ahead of the rig camera: in view.
    pub(crate) const AHEAD: Vec3 = Vec3::new(0.0, 0.0, -10.0);
    /// Behind the rig camera: out of view, the chip pins to the edge.
    pub(crate) const BEHIND: Vec3 = Vec3::new(2.0, -1.0, 10.0);

    /// The live layout rig with the indicator widget and the objective chips,
    /// a projecting camera at the origin looking down -Z, and a manual clock of
    /// [`STEP_SECS`] a frame. One warm-up frame: the first manual frame
    /// advances no time.
    pub(crate) fn dwell_app() -> App {
        use std::f32::consts::FRAC_PI_2;

        use bevy::time::TimeUpdateStrategy;

        let mut app = chip_layout_app();
        app.add_plugins((ScreenIndicatorPlugin, ObjectiveMarkersHudPlugin));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(STEP_SECS),
        ));
        let mut q_camera = app.world_mut().query::<(Entity, &mut Camera)>();
        let (camera, mut lens) = q_camera
            .single_mut(app.world_mut())
            .expect("the rig has one camera");
        lens.computed.clip_from_view =
            bevy::math::proj::perspective_infinite_reverse(FRAC_PI_2, 1280.0 / 720.0, 0.1);
        app.world_mut()
            .entity_mut(camera)
            .insert(ScreenIndicatorCamera);
        app.update();
        app
    }

    /// Run frames until `secs` of virtual time have passed.
    pub(crate) fn advance(app: &mut App, secs: f32) {
        for _ in 0..(secs / STEP_SECS).round() as u32 {
            app.update();
        }
    }

    /// A body tagged `label` at `at`.
    pub(crate) fn mark_body(app: &mut App, label: &str, at: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                ObjectiveMarkerTarget::new(label),
                Transform::from_translation(at),
            ))
            .id()
    }

    fn move_body(app: &mut App, body: Entity, to: Vec3) {
        app.world_mut()
            .entity_mut(body)
            .insert(Transform::from_translation(to));
    }

    fn layer_of(app: &mut App, body: Entity) -> Entity {
        app.world_mut()
            .query_filtered::<(Entity, &AnchoredChipTarget), With<ObjectiveMarkerChipHudMarker>>()
            .iter(app.world())
            .find(|(_, target)| ***target == body)
            .map(|(layer, _)| layer)
            .expect("the body grew a chip layer")
    }

    /// What the chip on `body` draws, read from the placed nodes: the full
    /// chip, the bare diamond, and whether the chip's edge chevron shows.
    #[derive(Debug, PartialEq)]
    pub(crate) struct Drawn {
        pub(crate) chip: bool,
        pub(crate) mark: bool,
        pub(crate) pinned: bool,
    }

    pub(crate) fn drawn(app: &mut App, body: Entity) -> Drawn {
        let layer = layer_of(app, body);
        let shows = |app: &mut App, node: Entity| {
            *app.world().get::<Visibility>(node).unwrap() != Visibility::Hidden
        };
        let chip = only_descendant_with::<AnchoredChipNodeMarker>(app, layer);
        let mark = only_descendant_with::<ObjectiveMarkMarker>(app, layer);
        let chevron = only_descendant_with::<ScreenIndicatorArrowMarker>(app, layer);
        let drawn = Drawn {
            chip: shows(app, chip),
            mark: shows(app, mark),
            pinned: shows(app, chevron),
        };
        // Delivery guard: the look the layer names is the node that draws.
        let shown = *app.world().get::<ObjectiveChipType>(layer).unwrap();
        assert_eq!(
            (
                shown == ObjectiveChipType::Full,
                shown == ObjectiveChipType::Mark
            ),
            (drawn.chip, drawn.mark),
            "the layer reads {shown:?} but draws {drawn:?}"
        );
        drawn
    }

    const FULL: Drawn = Drawn {
        chip: true,
        mark: false,
        pinned: false,
    };
    const MARK: Drawn = Drawn {
        chip: false,
        mark: true,
        pinned: false,
    };
    const PINNED: Drawn = Drawn {
        chip: true,
        mark: false,
        pinned: true,
    };

    /// A body tagged in view wears the full chip through the notice dwell,
    /// then only the diamond.
    #[test]
    fn a_chip_in_view_settles_to_the_diamond_after_the_notice_dwell() {
        let mut app = dwell_app();
        let body = mark_body(&mut app, "JOB-1 Neutralize", AHEAD);

        advance(&mut app, CHIP_OPEN_SECS - 0.5);
        assert_eq!(drawn(&mut app, body), FULL, "inside the dwell");

        advance(&mut app, 1.0);
        assert_eq!(drawn(&mut app, body), MARK, "past the dwell");
    }

    /// A new label reopens a settled chip for a full dwell; the same label
    /// written again does not.
    #[test]
    fn a_label_change_reopens_the_chip() {
        let mut app = dwell_app();
        let body = mark_body(&mut app, "JOB-1 Neutralize", AHEAD);
        advance(&mut app, CHIP_OPEN_SECS + 0.5);
        assert_eq!(drawn(&mut app, body), MARK, "delivery guard: settled");

        app.world_mut()
            .entity_mut(body)
            .insert(ObjectiveMarkerTarget::new("JOB-1 Neutralize"));
        advance(&mut app, 0.3);
        assert_eq!(drawn(&mut app, body), MARK, "the same label again");

        app.world_mut()
            .entity_mut(body)
            .insert(ObjectiveMarkerTarget::new("JOB-1, JOB-3 Neutralize"));
        advance(&mut app, 0.3);
        assert_eq!(drawn(&mut app, body), FULL, "a new label");

        advance(&mut app, CHIP_OPEN_SECS);
        assert_eq!(drawn(&mut app, body), MARK, "the reopened chip settles");
    }

    /// Out of view the full chip pins to the edge. Back after a second or more
    /// out of view, the chip reopens; back after a shorter trip, the diamond
    /// stays.
    #[test]
    fn a_second_out_of_view_reopens_the_chip_and_a_shorter_trip_does_not() {
        let mut app = dwell_app();
        let body = mark_body(&mut app, "JOB-1 Neutralize", AHEAD);
        advance(&mut app, CHIP_OPEN_SECS + 0.5);
        assert_eq!(drawn(&mut app, body), MARK, "delivery guard: settled");

        move_body(&mut app, body, BEHIND);
        advance(&mut app, REOPEN_AWAY_SECS / 2.0);
        assert_eq!(drawn(&mut app, body), PINNED, "out of view");
        move_body(&mut app, body, AHEAD);
        advance(&mut app, 0.3);
        assert_eq!(drawn(&mut app, body), MARK, "back after a short trip");

        move_body(&mut app, body, BEHIND);
        advance(&mut app, REOPEN_AWAY_SECS * 1.5);
        assert_eq!(drawn(&mut app, body), PINNED, "out of view");
        move_body(&mut app, body, AHEAD);
        advance(&mut app, 0.3);
        assert_eq!(drawn(&mut app, body), FULL, "back after a long trip");
    }

    /// A combat lock on a settled body leaves the diamond: the lock readout
    /// and the target inset carry the range and the name.
    #[test]
    fn a_lock_does_not_reopen_the_chip() {
        let mut app = dwell_app();
        let body = mark_body(&mut app, "JOB-1 Neutralize", AHEAD);
        advance(&mut app, CHIP_OPEN_SECS + 0.5);
        assert_eq!(drawn(&mut app, body), MARK, "delivery guard: settled");

        app.world_mut().spawn((
            PlayerSpaceshipMarker,
            Transform::default(),
            nova_ship::prelude::CombatLock(Some(body)),
        ));
        advance(&mut app, 1.0);
        assert_eq!(drawn(&mut app, body), MARK, "locked");
    }
}
