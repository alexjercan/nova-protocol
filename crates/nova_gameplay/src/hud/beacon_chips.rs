//! Nav beacon HUD chips (task 20260712-093044): one screen-projected chip
//! per [`BeaconMarker`] entity - the beacon's label plus live distance to
//! the player ship - with the indicator widget's `ClampToEdge` path, so an
//! off-screen beacon's chip pins to the viewport edge and its chevron
//! points at it. The chip IS the game's direction-to-objective cue; the
//! scenario only spawns a beacon and the HUD does the rest.
//!
//! Chrome tier: beacons are guidance, not flight instruments.

use bevy::prelude::*;

use super::{screen_indicator::prelude::*, HudTier, NAV_CYAN};
use crate::prelude::*;

/// Glob-import surface: `use nova_gameplay::hud::beacon_chips::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{
        BeaconChipHudMarker, BeaconChipLabelMarker, BeaconChipTargetEntity, BeaconChipsHudPlugin,
    };
}

/// Chip footprint (px). Wide enough for "BEACON 1 1234m" on one line.
const CHIP_SIZE: Vec2 = Vec2::new(140.0, 16.0);

/// The chip floats above the beacon so the label never sits on the mesh.
const CHIP_OFFSET: Vec2 = Vec2::new(0.0, -28.0);

/// Inset (px) from the viewport edges while clamped. Matches the edge
/// indicators' frame so clamped beacon chips join the same visual ring.
const EDGE_MARGIN_PX: f32 = 30.0;

/// Chevron stroke geometry, the edge-indicator arrow language at chip scale.
const ARROW_PX: f32 = 16.0;
const STROKE_LEN_PX: f32 = 11.0;
const STROKE_THICK_PX: f32 = 2.0;

const LABEL_FONT_PX: f32 = 12.0;

/// Marker for one beacon chip layer (one per beacon).
#[derive(Component, Debug, Clone, Reflect)]
pub struct BeaconChipHudMarker;

/// The beacon entity this chip tracks.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct BeaconChipTargetEntity(pub Entity);

/// Marker for the chip's text node.
#[derive(Component, Debug, Clone, Reflect)]
pub struct BeaconChipLabelMarker;

/// UI bundle for one beacon's chip layer. `suppressed` spawns the chip
/// already yielded (anchor None) for a beacon that carries an objective
/// marker at chip-spawn time, so no ordering of marker-attach vs
/// chip-spawn can leave two chips on one target (review R1.2).
fn beacon_chip_hud(beacon: Entity, suppressed: bool) -> impl Bundle {
    (
        Name::new("BeaconChipHUD"),
        BeaconChipHudMarker,
        BeaconChipTargetEntity(beacon),
        HudTier::Chrome,
        screen_indicator_layer(),
        children![(
            Name::new("BeaconChipUI"),
            BeaconChipLabelMarker,
            screen_indicator(ScreenIndicatorConfig {
                anchor: (!suppressed).then_some(ScreenIndicatorAnchorKind::Entity(beacon)),
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
            TextColor(NAV_CYAN),
            children![beacon_chip_arrow()],
        )],
    )
}

/// An up-pointing chevron the widget rotates toward the beacon while the
/// chip is edge-clamped (the edge-indicator arrow language, chip-sized).
/// Hidden while the beacon is on-screen - the widget owns its visibility.
fn beacon_chip_arrow() -> impl Bundle {
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
            BackgroundColor(NAV_CYAN),
            Pickable::IGNORE,
        )
    };

    (
        Name::new("BeaconChipArrow"),
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

/// Draws one screen-projected, edge-clamping chip per [`BeaconMarker`]
/// (label + live distance), yielding the chip to an objective marker when one
/// shares the beacon (Chrome tier).
/// Registers [`BeaconMarker`]/[`BeaconLabel`], adds the spawn/despawn and
/// suppress/restore observers, and runs `update_beacon_chip_labels` in Update
/// within [`super::NovaHudSystems`].
#[derive(Default)]
pub struct BeaconChipsHudPlugin;

impl Plugin for BeaconChipsHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("BeaconChipsHudPlugin: build");

        app.register_type::<BeaconMarker>();
        app.register_type::<BeaconLabel>();

        app.add_observer(setup_beacon_chip);
        app.add_observer(remove_beacon_chip);
        app.add_observer(suppress_marked_beacon_chip);
        app.add_observer(restore_unmarked_beacon_chip);
        app.add_systems(
            Update,
            update_beacon_chip_labels.in_set(super::NovaHudSystems),
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
    debug!(
        "setup_beacon_chip: beacon {:?} (suppressed {})",
        beacon, suppressed
    );
    commands.spawn(beacon_chip_hud(beacon, suppressed));
}

/// The chip layer dies with its beacon (despawn action, scenario unload -
/// any removal path).
fn remove_beacon_chip(
    remove: On<Remove, BeaconMarker>,
    mut commands: Commands,
    q_chips: Query<(Entity, &BeaconChipTargetEntity), With<BeaconChipHudMarker>>,
) {
    let beacon = remove.entity;
    for (chip, target) in &q_chips {
        if **target == beacon {
            trace!("remove_beacon_chip: despawning chip {:?}", chip);
            commands.entity(chip).despawn();
        }
    }
}

/// Label text: the beacon's name plus the live distance to the player ship
/// ("BEACON 1  420m"). Without a player (menu ambience, death gap) the
/// label alone shows - the chip is still a valid waypoint tag.
fn update_beacon_chip_labels(
    q_chips: Query<&BeaconChipTargetEntity, With<BeaconChipHudMarker>>,
    mut q_labels: Query<(&mut Text, &ChildOf), With<BeaconChipLabelMarker>>,
    q_beacons: Query<(&BeaconLabel, &GlobalTransform), With<BeaconMarker>>,
    q_player: Query<&GlobalTransform, With<PlayerSpaceshipMarker>>,
) {
    let player = q_player.iter().next();
    for (mut text, ChildOf(layer)) in &mut q_labels {
        let Ok(target) = q_chips.get(*layer) else {
            continue;
        };
        let Ok((label, beacon_transform)) = q_beacons.get(**target) else {
            continue;
        };
        let next = match player {
            Some(player_transform) => {
                let distance = player_transform
                    .translation()
                    .distance(beacon_transform.translation());
                format!("{}  {:.0}m", **label, distance)
            }
            None => (**label).clone(),
        };
        if **text != next {
            **text = next;
        }
    }
}

/// One entity, one chip: while a beacon carries [`ObjectiveMarkerTarget`]
/// its gold marker chip supersedes the cyan beacon chip - two clamped chips
/// on the same target would jitter over each other at the screen edge
/// (task 20260712-093831). Suppression goes through the anchor (the
/// widget's established hide channel, same as the verb cues): None hides
/// the chip, restoring the entity anchor revives it on detach. Observers,
/// not a polled system, so the hand-off lands in the SAME command flush as
/// the marker insert/removal - a polled pass left a schedule-tie-break
/// frame with two chips (or none) at the edge (review R1.2).
fn set_beacon_chip_anchor(
    beacon: Entity,
    wanted: Option<ScreenIndicatorAnchorKind>,
    q_chips: &Query<&BeaconChipTargetEntity, With<BeaconChipHudMarker>>,
    q_anchors: &mut Query<(&mut ScreenIndicatorAnchor, &ChildOf), With<BeaconChipLabelMarker>>,
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
    q_chips: Query<&BeaconChipTargetEntity, With<BeaconChipHudMarker>>,
    mut q_anchors: Query<(&mut ScreenIndicatorAnchor, &ChildOf), With<BeaconChipLabelMarker>>,
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
    q_chips: Query<&BeaconChipTargetEntity, With<BeaconChipHudMarker>>,
    mut q_anchors: Query<(&mut ScreenIndicatorAnchor, &ChildOf), With<BeaconChipLabelMarker>>,
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
    use super::*;

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
                .query_filtered::<(&ScreenIndicatorAnchor, &ChildOf), With<BeaconChipLabelMarker>>(
                );
            let layers: Vec<(Option<ScreenIndicatorAnchorKind>, Entity)> = q
                .iter(world)
                .map(|(anchor, ChildOf(layer))| (**anchor, *layer))
                .collect();
            let mut found = None;
            for (anchor, layer) in layers {
                let target = world.get::<BeaconChipTargetEntity>(layer).unwrap();
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

        let mut q = world.query_filtered::<&ScreenIndicatorAnchor, With<BeaconChipLabelMarker>>();
        let anchors: Vec<Option<ScreenIndicatorAnchorKind>> =
            q.iter(&world).map(|anchor| **anchor).collect();
        assert_eq!(
            anchors,
            vec![None],
            "the chip spawns already yielded to the marker"
        );
    }
}
