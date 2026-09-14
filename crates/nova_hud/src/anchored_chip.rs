//! The world-anchored nav chip: the shape the beacon chips and the objective
//! marker chips both are.
//!
//! One screen-projected, edge-clamping pill per entity carrying a family TAG
//! component - the label plus live distance to the player ship - built from
//! [`screen_indicator`](super::screen_indicator)'s projection widget and
//! `nova_ui`'s chip paint. This module owns the parts that are the same for
//! every such family: the layer bundle, the pill, the label leaf, the edge
//! chevron, the despawn observer and the label-plus-distance updater. A family
//! module owns what is genuinely its own - its tag, its tone and geometry, and
//! any spawn-time policy (the beacon chip yields its slot to an objective
//! marker; the objective chip breathes).
//!
//! Lifecycle, as opposed to the crate's `despawn_player_hud`: that helper
//! despawns EVERY node carrying its marker when the player ship goes, which
//! its own doc rules out for a widget whose nodes point back at a specific
//! entity. These chips are exactly that widget - one layer per marked entity -
//! so [`despawn_anchored_chips`] takes down only the layers aimed at the
//! entity that left.
//!
//! A family is told apart by its own LAYER marker component, which is the one
//! per-family component the shared systems are generic over. Everything below
//! the layer (the target, the pill node, the label leaf) is shared, so a query
//! that walks into a chip is gated by the layer it hangs under, never by a
//! component spelling.

use bevy::{ecs::query::QueryFilter, prelude::*};
use nova_events::units::prelude::*;
use nova_gameplay::prelude::*;
use nova_ui::hud::{chip_node, chip_paint, ChipTone};

use super::{screen_indicator::prelude::*, HudTier};

/// The shared chip components, the chip bundles, the chevron geometry, the
/// label source trait and the two generic chip systems.
pub mod prelude {
    pub use super::{
        anchored_chip_chevron, anchored_chip_label, anchored_chip_layer, anchored_chip_node,
        despawn_anchored_chips, update_anchored_chip_labels, AnchoredChipChevron,
        AnchoredChipLabelMarker, AnchoredChipLabelSource, AnchoredChipNodeMarker,
        AnchoredChipTarget, CHIP_EDGE_MARGIN_PX,
    };
}

/// Inset (px) from the viewport edges a clamped chip sits at, so every
/// world-anchored chip joins the edge indicators' visual ring rather than each
/// picking its own frame.
pub const CHIP_EDGE_MARGIN_PX: f32 = 30.0;

/// The world entity a chip layer tracks. Lives on the LAYER, which is what the
/// family marker also sits on, so one lookup answers both "whose chip is this"
/// and "which family".
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct AnchoredChipTarget(pub Entity);

/// Marker for the chip node itself: the bordered pill carrying the
/// screen-indicator anchor. The label text lives in a CHILD (see
/// [`AnchoredChipLabelMarker`]), so this node stays the one thing the indicator
/// widget positions and `chip_paint` paints.
#[derive(Component, Debug, Clone, Reflect)]
pub struct AnchoredChipNodeMarker;

/// Marker for a chip's text node - a LEAF child of the pill.
///
/// The label cannot live on the chip entity itself: taffy only measures leaf
/// nodes, so a `Text` node that also has children loses its measure and the
/// pill collapses to its padding.
#[derive(Component, Debug, Clone, Reflect)]
pub struct AnchoredChipLabelMarker;

/// Where a chip family reads its label text from.
///
/// Implemented on the component that CARRIES the name, which is not always the
/// family's tag: a beacon's name is a separate `BeaconLabel` beside
/// `BeaconMarker`, while an objective marker carries both in one component.
pub trait AnchoredChipLabelSource: Component {
    /// The short name the chip shows before the distance.
    fn chip_label(&self) -> &str;
}

/// Bundle for one world-anchored chip LAYER: the full-screen click-through
/// container the pill hangs under, tagged with the entity it tracks. The
/// caller adds its own `Name` and its family marker.
///
/// Chrome tier for the whole family: a nav chip is guidance, not a flight
/// instrument.
pub fn anchored_chip_layer(target: Entity) -> impl Bundle {
    (
        AnchoredChipTarget(target),
        HudTier::Chrome,
        screen_indicator_layer(),
    )
}

/// Bundle for the pill itself: the projected, edge-clamping node in `tone`,
/// standing `clearance` clear of its anchor. `anchor` is `None` for a chip that
/// spawns already yielded.
///
/// Sized [`ScreenIndicatorSize::Content`] - the chip hugs its own text, because
/// with a visible fill and border a fixed footprint would either clip a long
/// name or hang an empty slab off a short one.
///
/// The pill is a pure CONTAINER: the label is an in-flow leaf child it grows
/// around, the chevron an absolute one it ignores. Putting the `Text` here
/// instead would take this node off taffy's leaf path and collapse it.
pub fn anchored_chip_node(
    anchor: Option<Entity>,
    tone: ChipTone,
    clearance: ScreenIndicatorClearance,
) -> impl Bundle {
    (
        AnchoredChipNodeMarker,
        screen_indicator_node(
            ScreenIndicatorConfig {
                anchor: anchor.map(ScreenIndicatorAnchorKind::Entity),
                size: ScreenIndicatorSize::Content,
                offset: Vec2::ZERO,
                offscreen: ScreenIndicatorOffscreen::ClampToEdge {
                    margin_px: CHIP_EDGE_MARGIN_PX,
                },
            },
            chip_node(),
        ),
        clearance,
        chip_paint(tone),
    )
}

/// The label text: a leaf `Text` child so taffy measures it and the pill grows
/// to hold it. `extra` is whatever else the family's label wears (the objective
/// chip's contrast shadow); pass `()` for none.
pub fn anchored_chip_label<B: Bundle>(font_px: f32, color: Color, extra: B) -> impl Bundle {
    (
        AnchoredChipLabelMarker,
        Text::new(""),
        TextFont::from_font_size(font_px),
        TextLayout {
            linebreak: LineBreak::NoWrap,
            ..default()
        },
        TextColor(color),
        extra,
        Pickable::IGNORE,
    )
}

/// Chevron stroke geometry and paint: the edge-indicator arrow language at chip
/// scale. Each family sizes and tints its own.
#[derive(Debug, Clone, Copy)]
pub struct AnchoredChipChevron {
    /// Side (px) of the square the chevron is drawn inside.
    pub arrow_px: f32,
    /// Length (px) of each of the two strokes.
    pub stroke_len_px: f32,
    /// Thickness (px) of each stroke.
    pub stroke_thick_px: f32,
    /// Stroke colour.
    pub color: Color,
}

/// An up-pointing chevron the indicator widget rotates toward the anchor while
/// the chip is edge-clamped. Hidden while the anchor is on-screen - the widget
/// owns its visibility. `stroke_extra` rides on each stroke entity (the
/// objective chip's breath marker); pass `()` for none.
///
/// The caller adds its own `Name`.
pub fn anchored_chip_chevron<B: Bundle + Clone>(
    chevron: AnchoredChipChevron,
    stroke_extra: B,
) -> impl Bundle {
    let AnchoredChipChevron {
        arrow_px,
        stroke_len_px,
        stroke_thick_px,
        color,
    } = chevron;

    let stroke = |left: f32, degrees: f32| {
        (
            stroke_extra.clone(),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(arrow_px / 2.0 - stroke_thick_px / 2.0),
                width: Val::Px(stroke_len_px),
                height: Val::Px(stroke_thick_px),
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
        ScreenIndicatorArrowMarker,
        Node {
            position_type: PositionType::Absolute,
            // Park the chevron just above the pill and centred on it. Half the
            // chip's width, then back off half the chevron's own - a plain
            // `-arrow_px / 2` sat near the origin, which only looked centred
            // while the pill was a collapsed slab.
            // `update_arrows` writes only `.rotation`, so this translation
            // survives every frame.
            left: Val::Percent(50.0),
            top: Val::Px(-arrow_px - 2.0),
            width: Val::Px(arrow_px),
            height: Val::Px(arrow_px),
            ..default()
        },
        UiTransform::from_translation(Val2::px(-arrow_px / 2.0, 0.0)),
        Visibility::Hidden,
        Pickable::IGNORE,
        children![
            stroke(-0.5, -45.0),
            stroke(arrow_px - stroke_len_px + 0.5, 45.0),
        ],
    )
}

/// The chip layer dies with its tag - explicit detach action, the marked entity
/// despawning, scenario unload: any removal path.
///
/// `Tag` is the family's world-side component, `Layer` its layer marker. Only
/// the layers aimed at the entity that left come down, so a sibling chip of the
/// same family survives.
pub fn despawn_anchored_chips<Tag: Component, Layer: Component>(
    remove: On<Remove, Tag>,
    mut commands: Commands,
    q_chips: Query<(Entity, &AnchoredChipTarget), With<Layer>>,
) {
    let target = remove.entity;
    for (chip, chip_target) in &q_chips {
        if **chip_target == target {
            trace!(
                "despawn_anchored_chips<{}>: despawning chip {:?}",
                core::any::type_name::<Layer>(),
                chip
            );
            commands.entity(chip).despawn();
        }
    }
}

/// Label text: the family's label plus the live distance to the player ship
/// ("BEACON 1  4.20 km"). Without a player (menu ambience, death gap) the label
/// alone shows - the chip is still a valid waypoint tag.
///
/// Two hops: the text is a leaf CHILD of the pill, which is itself a child of
/// the layer that knows the target. `Layer` gates the walk to one family, so a
/// chip of another family under the shared label marker is skipped. `Gate` is
/// the filter the family puts on its label source - `With<BeaconMarker>` where
/// the name lives on a component of its own, `()` where the tag carries it.
pub fn update_anchored_chip_labels<Layer, Label, Gate>(
    q_chips: Query<&AnchoredChipTarget, With<Layer>>,
    mut q_labels: Query<(&mut Text, &ChildOf), With<AnchoredChipLabelMarker>>,
    q_parents: Query<&ChildOf>,
    q_targets: Query<(&Label, &GlobalTransform), Gate>,
    q_player: Query<&GlobalTransform, With<PlayerSpaceshipMarker>>,
) where
    Layer: Component,
    Label: AnchoredChipLabelSource,
    Gate: QueryFilter + 'static,
{
    let player = q_player.iter().next();
    for (mut text, ChildOf(chip)) in &mut q_labels {
        let Ok(ChildOf(layer)) = q_parents.get(*chip) else {
            continue;
        };
        let Ok(target) = q_chips.get(*layer) else {
            continue;
        };
        let Ok((label, target_transform)) = q_targets.get(**target) else {
            continue;
        };
        let next = match player {
            Some(player_transform) => {
                let distance = player_transform
                    .translation()
                    .distance(target_transform.translation());
                format!(
                    "{}  {}",
                    label.chip_label(),
                    nova_ui::units::distance(Meters::from_engine(distance))
                )
            }
            None => label.chip_label().to_string(),
        };
        if **text != next {
            **text = next;
        }
    }
}
