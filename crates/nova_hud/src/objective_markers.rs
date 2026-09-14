//! Objective marker HUD chips: one screen-projected gold chip per
//! [`ObjectiveMarkerTarget`] entity - the marker's label plus live distance
//! to the player ship - with the indicator widget's `ClampToEdge` path, so an
//! off-screen objective's chip pins to the viewport edge and its chevron
//! points at it. Where the nav beacon chip says "a waypoint exists", this
//! chip says "go HERE now": gold, a diamond glyph instead of the beacon dot
//! language, and a slow alpha breath so it reads in peripheral vision without
//! strobing.
//!
//! The chip SHAPE - layer, pill, label leaf, chevron, despawn and the
//! label-plus-distance walk - is [`anchored_chip`](super::anchored_chip)'s,
//! shared with the beacon chips. What is the objective's own lives here: the
//! gold tone and geometry, the diamond identity glyph, and the breath.
//!
//! Chrome tier, like the beacon chips - the same nav-chip family.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_ui::hud::ChipTone;

use super::{anchored_chip::prelude::*, screen_indicator::prelude::*, OBJECTIVE_GOLD};

/// The objective marker chip layer marker and `ObjectiveMarkersHudPlugin`.
pub mod prelude {
    pub use super::{ObjectiveMarkerChipHudMarker, ObjectiveMarkersHudPlugin};
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

/// The objective chip's chevron: the shared arrow language at marker scale, in
/// objective gold.
const CHEVRON: AnchoredChipChevron = AnchoredChipChevron {
    arrow_px: 22.0,
    stroke_len_px: 15.0,
    stroke_thick_px: 2.5,
    color: OBJECTIVE_GOLD,
};

/// Diamond glyph: a square border rotated 45 degrees, sitting left of the
/// label - the marker's identity mark, always visible (the chevron only
/// shows while edge-clamped).
const DIAMOND_PX: f32 = 12.0;
const DIAMOND_BORDER_PX: f32 = 2.0;

const LABEL_FONT_PX: f32 = 16.0;

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

/// Marker for the diamond identity glyph, so tests and future drivers can find
/// it without matching on its display [`Name`].
#[derive(Component, Debug, Clone, Reflect)]
struct ObjectiveMarkerDiamondMarker;

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

/// UI bundle for one marked entity's chip layer.
fn objective_marker_chip_hud(target: Entity) -> impl Bundle {
    (
        Name::new("ObjectiveMarkerChipHUD"),
        ObjectiveMarkerChipHudMarker,
        anchored_chip_layer(target),
        children![(
            Name::new("ObjectiveMarkerChipUI"),
            // The objective chip is the amber "do this now" member of the chip
            // family (demo 2 `.obj`).
            anchored_chip_node(Some(target), ChipTone::Amber, CHIP_CLEARANCE),
            children![
                objective_marker_diamond(),
                (
                    Name::new("ObjectiveMarkerLabel"),
                    // The LABEL does not breathe: translucent gold over a
                    // bright planetoid was unreadable. Constant full gold + a
                    // tight dark shadow for contrast; the diamond and chevron
                    // carry the motion.
                    anchored_chip_label(
                        LABEL_FONT_PX,
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
        )],
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
        ObjectiveMarkerBreathMarker,
        Node {
            width: Val::Px(DIAMOND_PX),
            height: Val::Px(DIAMOND_PX),
            border: UiRect::all(Val::Px(DIAMOND_BORDER_PX)),
            flex_shrink: 0.0,
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

/// Draws one breathing gold chip (diamond glyph + label + edge chevron) per
/// [`ObjectiveMarkerTarget`] entity - the "do this now" objective cue (Chrome
/// tier).
/// Registers [`ObjectiveMarkerTarget`], adds the chip spawn/despawn observers,
/// and runs the shared label updater plus `breathe_objective_markers` in Update
/// within [`super::NovaHudSystems`].
#[derive(Default)]
pub struct ObjectiveMarkersHudPlugin;

impl Plugin for ObjectiveMarkersHudPlugin {
    fn build(&self, app: &mut App) {
        trace!("ObjectiveMarkersHudPlugin: build");

        app.register_type::<ObjectiveMarkerTarget>();

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
    }
}

/// Every marked entity grows its chip the moment the tag lands.
fn setup_objective_marker_chip(add: On<Add, ObjectiveMarkerTarget>, mut commands: Commands) {
    let target = add.entity;
    trace!("setup_objective_marker_chip: target {:?}", target);
    commands.spawn(objective_marker_chip_hud(target));
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
mod tests {
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
            LABEL_FONT_PX,
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
