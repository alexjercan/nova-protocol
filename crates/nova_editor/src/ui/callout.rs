//! The placement callout: what the solver says, said where the part is.
//!
//! Screen-space, positioned from the ghost's own pose every frame. The verdict
//! used to be a chip at the foot of the screen, 200px from the part it was
//! about, which asked a builder watching a ghost snap around a hull to read
//! the answer somewhere else.

use bevy::{picking::Pickable, prelude::*, ui::widget::TextShadow};
use nova_ship::prelude::{GameSections, SectionConfig};
use nova_ui::{
    prelude::{panel, UiSkin, UiText},
    theme,
};

use crate::{
    config::{Placement, PlacementPreview},
    gallery::EditorCamera,
    node::{EditContext, SectionNode, ShipNode},
    ui::layer,
};

/// How far under the ghost's own point the callout hangs.
///
/// UNDER, not over: the part is being mated onto something above it as often
/// as not, and a callout over the mate hides the thing the builder is aiming.
const DROP: f32 = 28.0;

/// The panel that follows the ghost.
#[derive(Component)]
pub(crate) struct PlacementCallout;

/// The fault line: why this pose is refused, and the key that resolves it.
#[derive(Component)]
pub(crate) struct CalloutRefusal;

/// The news line: which socket takes which. Its OWN row, never the fault's,
/// because a readout in the slot an error uses has to be re-read to find out
/// which one it is.
#[derive(Component)]
pub(crate) struct CalloutMate;

/// The callout, parked hidden until a part is in hand.
pub(crate) fn placement_callout(skin: UiSkin) -> impl Bundle {
    (
        Name::new("Placement Callout"),
        PlacementCallout,
        GlobalZIndex(layer::STAGE_VERDICT_Z),
        Visibility::Hidden,
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            max_width: px(240),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            row_gap: px(2),
            padding: UiRect::axes(px(8), px(4)),
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        panel(skin),
        children![
            (
                Name::new("Placement Refusal"),
                CalloutRefusal,
                // Every row of it, not just the panel: a child node is picked
                // on its own, and a label over the hull is a label the ray that
                // aims the ghost would stop at.
                Pickable::IGNORE,
                UiText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(theme::RED),
                TextShadow::default(),
                Node {
                    display: Display::None,
                    ..default()
                },
            ),
            (
                Name::new("Placement Mate"),
                CalloutMate,
                Pickable::IGNORE,
                UiText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR_MUTED),
                Node {
                    display: Display::None,
                    ..default()
                },
            ),
        ],
    )
}

/// Put the callout under the ghost and fill in whichever of its two lines the
/// solve calls for.
///
/// Hidden outright where there is no ghost: the callout is about a part in
/// hand, and a panel left standing over an empty stage is one more thing to
/// read.
pub(crate) fn sync_placement_callout(
    preview: Res<PlacementPreview>,
    sections: Res<GameSections>,
    context: Res<EditContext>,
    q_ships: Query<&GlobalTransform, With<ShipNode>>,
    q_nodes: Query<&SectionNode>,
    cameras: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    callouts: Query<(&mut Node, &mut Visibility, &ComputedNode), With<PlacementCallout>>,
    mut refusals: Query<(&mut Text, &mut Node), (With<CalloutRefusal>, Without<PlacementCallout>)>,
    mut mates: Query<
        (&mut Text, &mut Node),
        (
            With<CalloutMate>,
            Without<CalloutRefusal>,
            Without<PlacementCallout>,
        ),
    >,
) {
    let spot = preview
        .placement
        .as_ref()
        .zip(context.ship())
        .and_then(|(placement, ship)| {
            let ship_pose = q_ships.get(ship).ok()?;
            let world = ship_pose
                .mul_transform(placement.solve.transform)
                .translation();
            let (camera, camera_pose) = cameras.iter().next()?;
            camera.world_to_viewport(camera_pose, world).ok()
        });

    for (mut node, mut visibility, computed) in callouts {
        let Some((placement, spot)) = preview.placement.as_ref().zip(spot) else {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            continue;
        };
        if *visibility != Visibility::Inherited {
            *visibility = Visibility::Inherited;
        }
        // Centred on the ghost's point and hung under it. The size is last
        // frame's, which is one frame stale on the frame the text changes and
        // exact on every other - a callout that jumped a few pixels once per
        // wording change is cheaper than a layout pass to place a label.
        let size = computed.size();
        let left = px(spot.x - size.x / 2.0);
        let top = px(spot.y + DROP);
        if node.left != left {
            node.left = left;
        }
        if node.top != top {
            node.top = top;
        }

        let target = q_nodes
            .get(placement.target_section)
            .ok()
            .and_then(|section| section.resolve(Some(&sections)));
        let (refusal, mate) = callout_lines(placement, &sections, target);
        for (mut text, mut row) in &mut refusals {
            write_row(&mut text, &mut row, refusal.map(ToString::to_string));
        }
        for (mut text, mut row) in &mut mates {
            write_row(&mut text, &mut row, mate.clone());
        }
    }
}

/// The two lines a solve calls for: the fault, then the mate.
///
/// A refused pose gets NO mate line: naming the sockets under a refusal would
/// describe a mate that is not going to happen. A legal one gets no fault line,
/// which is what makes the red row mean one thing.
fn callout_lines(
    placement: &Placement,
    sections: &GameSections,
    target: Option<&SectionConfig>,
) -> (Option<&'static str>, Option<String>) {
    if let Some(refusal) = placement.solve.refusal {
        return (Some(refusal.message()), None);
    }
    // LABELLED, because a row of two ids is unreadable as either news or a
    // fault until something says which it is.
    let mate = format!(
        "mate  {} <- {}",
        socket_id(target, placement.solve.target),
        socket_id(
            sections.get_section(&placement.prototype),
            placement.solve.source,
        ),
    );
    (None, Some(mate))
}

/// Show a line with `wanted` on it, or take the line away.
fn write_row(text: &mut Text, row: &mut Node, wanted: Option<String>) {
    let display = match wanted {
        Some(wanted) => {
            if text.0 != wanted {
                text.0 = wanted;
            }
            Display::Flex
        }
        None => Display::None,
    };
    if row.display != display {
        row.display = display;
    }
}

/// The diagnostic id of one socket on a section, for the readout.
fn socket_id(config: Option<&SectionConfig>, index: usize) -> String {
    config
        .and_then(|config| config.base.link_points.get(index))
        .map_or_else(String::new, |point| point.id.clone())
}

#[cfg(test)]
mod tests;
