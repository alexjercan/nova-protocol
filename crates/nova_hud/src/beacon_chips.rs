//! Nav beacon HUD chips: one screen-projected chip per [`BeaconMarker`]
//! entity - the beacon's label plus live distance to the player ship - with
//! the indicator widget's `ClampToEdge` path, so an off-screen beacon's chip
//! pins to the viewport edge and its chevron points at it. The chip IS the
//! game's direction-to-objective cue; the scenario only spawns a beacon and
//! the HUD does the rest.
//!
//! The chip SHAPE - layer, pill, label leaf, chevron, despawn and the
//! label-plus-distance walk - is [`anchored_chip`](super::anchored_chip)'s.
//! What is the beacon's own lives here: its cyan-phosphor tone and geometry,
//! and the one-entity-one-chip hand-off to an objective marker.
//!
//! Chrome tier: beacons are guidance, not flight instruments.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_ui::hud::ChipTone;

use super::{anchored_chip::prelude::*, screen_indicator::prelude::*, NAV_CYAN};

/// The beacon chip layer marker and `BeaconChipsHudPlugin`.
pub mod prelude {
    pub use super::{BeaconChipHudMarker, BeaconChipsHudPlugin};
}

/// How the chip stands off the beacon so the label never sits on the orb.
///
/// A beacon publishes no visible collider, because its only collider is the
/// trigger sphere, so the clearance falls back to its authored `BodyRadius`:
/// a 50 m marker gets the chip pushed clear of the orb instead of printed
/// across it. The floor is the 28 px the chip has always floated at.
const CHIP_CLEARANCE: ScreenIndicatorClearance = ScreenIndicatorClearance {
    direction: Vec2::NEG_Y,
    gap_px: 12.0,
    min_px: 28.0,
};

/// The beacon chip's chevron: the shared arrow language at beacon scale, in
/// nav cyan.
const CHEVRON: AnchoredChipChevron = AnchoredChipChevron {
    arrow_px: 16.0,
    stroke_len_px: 11.0,
    stroke_thick_px: 2.0,
    color: NAV_CYAN,
};

const LABEL_FONT_PX: f32 = 12.0;

/// Marker for one beacon chip layer (one per beacon). The family tag: every
/// shared chip system is gated on it, so a query that walks into a beacon chip
/// can never reach an objective marker's.
#[derive(Component, Debug, Clone, Reflect)]
pub struct BeaconChipHudMarker;

impl AnchoredChipLabelSource for BeaconLabel {
    fn chip_label(&self) -> &str {
        &self.0
    }
}

/// UI bundle for one beacon's chip layer. `suppressed` spawns the chip
/// already yielded (anchor None) for a beacon that carries an objective
/// marker at chip-spawn time, so no ordering of marker-attach vs
/// chip-spawn can leave two chips on one target.
fn beacon_chip_hud(beacon: Entity, suppressed: bool) -> impl Bundle {
    (
        Name::new("BeaconChipHUD"),
        BeaconChipHudMarker,
        anchored_chip_layer(beacon),
        children![(
            Name::new("BeaconChipUI"),
            anchored_chip_node(
                (!suppressed).then_some(beacon),
                ChipTone::Phosphor,
                CHIP_CLEARANCE,
            ),
            children![
                (
                    Name::new("BeaconChipLabel"),
                    anchored_chip_label(LABEL_FONT_PX, ChipTone::Phosphor.text(), ()),
                ),
                (
                    Name::new("BeaconChipArrow"),
                    anchored_chip_chevron(CHEVRON, ()),
                ),
            ],
        )],
    )
}

/// Draws one screen-projected, edge-clamping chip per [`BeaconMarker`]
/// (label + live distance), yielding the chip to an objective marker when one
/// shares the beacon (Chrome tier).
/// Registers [`BeaconMarker`]/[`BeaconLabel`], adds the spawn/despawn and
/// suppress/restore observers, and runs the shared label updater in Update
/// within [`super::NovaHudSystems`].
#[derive(Default)]
pub struct BeaconChipsHudPlugin;

impl Plugin for BeaconChipsHudPlugin {
    fn build(&self, app: &mut App) {
        trace!("BeaconChipsHudPlugin: build");

        app.register_type::<BeaconMarker>();
        app.register_type::<BeaconLabel>();

        app.add_observer(setup_beacon_chip);
        app.add_observer(despawn_anchored_chips::<BeaconMarker, BeaconChipHudMarker>);
        app.add_observer(suppress_marked_beacon_chip);
        app.add_observer(restore_unmarked_beacon_chip);
        app.add_systems(
            Update,
            // The name is a component of its own beside the tag, so the label
            // source is gated on the tag explicitly.
            update_anchored_chip_labels::<BeaconChipHudMarker, BeaconLabel, With<BeaconMarker>>
                .in_set(super::NovaHudSystems),
        );
    }
}

/// Every beacon grows its chip the moment it spawns - already yielded if
/// the beacon is somehow marked first (see [`beacon_chip_hud`]).
fn setup_beacon_chip(
    add: On<Add, BeaconMarker>,
    q_marked: Query<(), With<ObjectiveMarkerTarget>>,
    mut commands: Commands,
) {
    let beacon = add.entity;
    let suppressed = q_marked.get(beacon).is_ok();
    trace!(
        "setup_beacon_chip: beacon {:?} (suppressed {})",
        beacon,
        suppressed
    );
    commands.spawn(beacon_chip_hud(beacon, suppressed));
}

/// One entity, one chip: while a beacon carries [`ObjectiveMarkerTarget`]
/// its gold marker chip supersedes the cyan beacon chip - two clamped chips
/// on the same target would jitter over each other at the screen edge.
/// Suppression goes through the anchor (the
/// widget's established hide channel, same as the verb cues): None hides
/// the chip, restoring the entity anchor revives it on detach. Observers,
/// not a polled system, so the hand-off lands in the SAME command flush as
/// the marker insert/removal - a polled pass left a schedule-tie-break
/// frame with two chips (or none) at the edge.
///
/// The pill node marker is shared with every other chip family, so the layer
/// query is what keeps this to beacon chips.
fn set_beacon_chip_anchor(
    beacon: Entity,
    wanted: Option<ScreenIndicatorAnchorKind>,
    q_chips: &Query<&AnchoredChipTarget, With<BeaconChipHudMarker>>,
    q_anchors: &mut Query<(&mut ScreenIndicatorAnchor, &ChildOf), With<AnchoredChipNodeMarker>>,
) {
    for (mut anchor, ChildOf(layer)) in q_anchors {
        let Ok(target) = q_chips.get(*layer) else {
            continue;
        };
        if **target == beacon && **anchor != wanted {
            **anchor = wanted;
        }
    }
}

/// A marker landing on a beacon hands the chip slot to the gold marker.
fn suppress_marked_beacon_chip(
    add: On<Add, ObjectiveMarkerTarget>,
    q_beacon: Query<(), With<BeaconMarker>>,
    q_chips: Query<&AnchoredChipTarget, With<BeaconChipHudMarker>>,
    mut q_anchors: Query<(&mut ScreenIndicatorAnchor, &ChildOf), With<AnchoredChipNodeMarker>>,
) {
    if q_beacon.get(add.entity).is_err() {
        return;
    }
    set_beacon_chip_anchor(add.entity, None, &q_chips, &mut q_anchors);
}

/// Detaching the marker (explicitly or by despawn - in which case the chip
/// is dying too and the write is moot) revives the beacon chip.
fn restore_unmarked_beacon_chip(
    remove: On<Remove, ObjectiveMarkerTarget>,
    q_beacon: Query<(), With<BeaconMarker>>,
    q_chips: Query<&AnchoredChipTarget, With<BeaconChipHudMarker>>,
    mut q_anchors: Query<(&mut ScreenIndicatorAnchor, &ChildOf), With<AnchoredChipNodeMarker>>,
) {
    if q_beacon.get(remove.entity).is_err() {
        return;
    }
    set_beacon_chip_anchor(
        remove.entity,
        Some(ScreenIndicatorAnchorKind::Entity(remove.entity)),
        &q_chips,
        &mut q_anchors,
    );
}

#[cfg(test)]
mod tests {
    use super::{
        super::{
            chip_layout_rig::{
                assert_chip_backs_its_label, chip_layout_app, measure, only_descendant_with, settle,
            },
            objective_markers::prelude::ObjectiveMarkerChipHudMarker,
        },
        *,
    };

    /// The dedupe rule end to end on real chip bundles through the real
    /// observers (same command flush as the marker insert - no
    /// tie-break frame with two chips or none): marking the beacon hides
    /// its chip (anchor None), detaching restores it, and an unmarked
    /// sibling never flickers.
    #[test]
    fn marked_beacons_hand_their_chip_to_the_objective_marker() {
        let mut world = World::new();
        world.add_observer(suppress_marked_beacon_chip);
        world.add_observer(restore_unmarked_beacon_chip);
        let marked = world.spawn(BeaconMarker).id();
        let plain = world.spawn(BeaconMarker).id();
        world.spawn(beacon_chip_hud(marked, false));
        world.spawn(beacon_chip_hud(plain, false));

        let anchor_of = |world: &mut World, beacon: Entity| -> Option<ScreenIndicatorAnchorKind> {
            let mut q = world
                .query_filtered::<(&ScreenIndicatorAnchor, &ChildOf), With<AnchoredChipNodeMarker>>(
                );
            let layers: Vec<(Option<ScreenIndicatorAnchorKind>, Entity)> = q
                .iter(world)
                .map(|(anchor, ChildOf(layer))| (**anchor, *layer))
                .collect();
            let mut found = None;
            for (anchor, layer) in layers {
                let target = world.get::<AnchoredChipTarget>(layer).unwrap();
                if **target == beacon {
                    found = Some(anchor);
                }
            }
            found.expect("a chip exists for the beacon")
        };

        // Unmarked: both chips anchor their beacons (the spawn default).
        assert_eq!(
            anchor_of(&mut world, marked),
            Some(ScreenIndicatorAnchorKind::Entity(marked))
        );

        // Marked: the beacon chip yields (anchor None) in the same flush,
        // the sibling holds.
        world
            .entity_mut(marked)
            .insert(ObjectiveMarkerTarget::new("BEACON 1"));
        world.flush();
        assert_eq!(anchor_of(&mut world, marked), None);
        assert_eq!(
            anchor_of(&mut world, plain),
            Some(ScreenIndicatorAnchorKind::Entity(plain)),
            "an unmarked sibling keeps its chip"
        );

        // Detached: the chip revives.
        world.entity_mut(marked).remove::<ObjectiveMarkerTarget>();
        world.flush();
        assert_eq!(
            anchor_of(&mut world, marked),
            Some(ScreenIndicatorAnchorKind::Entity(marked))
        );

        // A marker on a NON-beacon (crate, pirate) must not touch beacon
        // chips: the suppress observer is beacon-gated.
        let pirate = world.spawn(ObjectiveMarkerTarget::new("SCAVENGER")).id();
        world.flush();
        assert_eq!(
            anchor_of(&mut world, marked),
            Some(ScreenIndicatorAnchorKind::Entity(marked))
        );
        let _ = pirate;
    }

    /// A marked beacon carries TWO chip layers naming the same entity, one per
    /// family. Detaching the marker must take down the objective's layer only.
    /// The shared [`AnchoredChipTarget`] is not what tells the families apart -
    /// the layer marker is, and a despawn observer that matched on the target
    /// alone would take the beacon's chip with it.
    #[test]
    fn one_familys_tag_leaving_leaves_the_others_chip_standing() {
        let mut world = World::new();
        world.add_observer(setup_beacon_chip);
        world.add_observer(despawn_anchored_chips::<BeaconMarker, BeaconChipHudMarker>);
        world.add_observer(
            despawn_anchored_chips::<ObjectiveMarkerTarget, ObjectiveMarkerChipHudMarker>,
        );

        let beacon = world
            .spawn((BeaconMarker, ObjectiveMarkerTarget::new("BEACON 1")))
            .id();
        // The objective family's layer, in the shape its own module spawns.
        world.spawn((ObjectiveMarkerChipHudMarker, anchored_chip_layer(beacon)));
        world.flush();

        let layers = |world: &mut World| -> (usize, usize) {
            let beacons = world
                .query_filtered::<Entity, With<BeaconChipHudMarker>>()
                .iter(world)
                .count();
            let objectives = world
                .query_filtered::<Entity, With<ObjectiveMarkerChipHudMarker>>()
                .iter(world)
                .count();
            (beacons, objectives)
        };
        assert_eq!(layers(&mut world), (1, 1), "one layer per family");

        world.entity_mut(beacon).remove::<ObjectiveMarkerTarget>();
        world.flush();

        assert_eq!(
            layers(&mut world),
            (1, 0),
            "the objective's layer comes down and the beacon's stays"
        );
    }

    /// Build the real chip through the real spawn observer and lay it out with
    /// the real taffy + text measurement, then hand back
    /// (layer, chip node, label node).
    fn laid_out_beacon_chip(label: &str) -> (App, Entity, Entity, Entity) {
        let mut app = chip_layout_app();
        app.add_plugins(BeaconChipsHudPlugin);
        app.world_mut().spawn((
            BeaconMarker,
            BeaconLabel::new(label),
            GlobalTransform::default(),
        ));
        settle(&mut app);

        let layer = app
            .world_mut()
            .query_filtered::<Entity, With<BeaconChipHudMarker>>()
            .iter(app.world())
            .next()
            .expect("the beacon grew a chip layer");
        let chip = only_descendant_with::<AnchoredChipNodeMarker>(&mut app, layer);
        let text = only_descendant_with::<Text>(&mut app, layer);
        (app, layer, chip, text)
    }

    /// The beacon chip is the second case of the same defect: its phosphor
    /// pill must back the whole label, asserted
    /// through the SAME shared helper as the objective chip.
    #[test]
    fn the_beacon_chip_backs_its_whole_label() {
        let (mut app, _layer, chip, text) = laid_out_beacon_chip("BEACON 1");
        assert_chip_backs_its_label(
            &mut app,
            chip,
            text,
            "BEACON 1",
            LABEL_FONT_PX,
            "beacon chip",
        );
    }

    /// The chevron parks centred over the pill, not off its left edge.
    #[test]
    fn the_beacon_chips_chevron_centres_over_the_pill() {
        let (mut app, layer, chip, _text) = laid_out_beacon_chip("BEACON 1");
        let arrow = only_descendant_with::<ScreenIndicatorArrowMarker>(&mut app, layer);
        let chip_centre = measure(&app, chip).rect.center();
        let arrow_centre = measure(&app, arrow).rect.center();
        assert!(
            (arrow_centre.x - chip_centre.x).abs() <= 1.0,
            "the chevron centre {arrow_centre:?} is not over the chip centre {chip_centre:?}"
        );
    }

    /// The adversarial ordering: a beacon that is ALREADY marked when its
    /// chip spawns gets a chip born yielded - no ordering of marker-attach
    /// vs chip-spawn can put two chips on one target.
    #[test]
    fn chip_spawned_for_an_already_marked_beacon_starts_yielded() {
        let mut world = World::new();
        world.add_observer(setup_beacon_chip);
        let beacon = world.spawn(ObjectiveMarkerTarget::new("BEACON 2")).id();
        world.entity_mut(beacon).insert(BeaconMarker);
        world.flush();

        let mut q = world.query_filtered::<&ScreenIndicatorAnchor, With<AnchoredChipNodeMarker>>();
        let anchors: Vec<Option<ScreenIndicatorAnchorKind>> =
            q.iter(&world).map(|anchor| **anchor).collect();
        assert_eq!(
            anchors,
            vec![None],
            "the chip spawns already yielded to the marker"
        );
    }
}
