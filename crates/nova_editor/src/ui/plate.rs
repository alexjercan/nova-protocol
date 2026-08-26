//! Nameplates: the document's names, said on the stage instead of only in the
//! rail.
//!
//! The rail names every node and the stage names none, so a scenario holding
//! five derelicts is five identical grey shapes and the only way to find out
//! which one is Derelict Hulk 3 is to click each in turn and read the rail. A
//! plate is that name said where the thing IS.
//!
//! Screen-space, positioned from the node's own bounds every frame - the same
//! rig [`crate::ui::callout`] hangs the placement verdict on. What earns a
//! plate and what does not is argued in the spike page for this editor pass,
//! `tasks/20260825-221015/diegetic-surfaces.html`; the short of it is that a
//! surface earns an anchor when the reader has to be LOOKING at the thing
//! while they read it.

use bevy::{color::Mix, picking::Pickable, prelude::*, ui::widget::TextShadow};
use nova_scenario::prelude::ScenarioObjectKind;
use nova_ui::{
    prelude::{clear_of, hang_at, Hang, UiSkin, UiText},
    theme,
    widget::list_row_colors,
};

use crate::{
    config::{HoveredNode, SelectedNode},
    frame::node_bounds,
    gallery::EditorCamera,
    node::{id_order, EditContext, NodeId, ObjectNode, ShipNode},
    ui::{layer, tree_text},
};

/// The plate stands centred over the node's own top, 14 logical pixels clear.
const PLATE_HANG: Hang = Hang {
    align: Vec2::new(0.5, 1.0),
    gap: Vec2::new(0.0, -14.0),
};

/// How much room a plate keeps to itself when it is pushed clear of another,
/// on top of its own size.
///
/// Two hulls a hand's width apart project to the same few pixels, and the pile
/// that makes reads as one unreadable name.
const CROWD_GAP: f32 = 2.0;

/// How solid the chip of screen behind a plate is.
///
/// Not opaque: a plate is a label on the scene, and a stage read through it is
/// worth more than the last of the contrast.
const FILL: f32 = 0.72;

/// The layer the plates hang in.
#[derive(Component)]
pub(crate) struct PlateLayer;

/// One plate, carrying the node it names.
#[derive(Component)]
pub(crate) struct NamePlate(pub(crate) Entity);

/// The layer, spawned once with the rest of the editor's chrome.
pub(crate) fn plate_layer() -> impl Bundle {
    (
        Name::new("Name Plates"),
        PlateLayer,
        GlobalZIndex(layer::STAGE_LABEL_Z),
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            ..default()
        },
    )
}

/// One plate. Marked ones wear the list's selected paint, so a plate and the
/// tree row it belongs to say "this one" the same way.
fn plate(label: &str, marked: bool, skin: UiSkin) -> impl Bundle {
    let (fill, border) = list_row_colors(marked, false, skin);
    // The row's fill assumes a PANEL behind it. A plate has the stage behind
    // it, so the row paint is mixed over a chip of screen rather than trusted
    // to carry a label over whatever the camera is pointed at.
    let background = theme::SCREEN_0.with_alpha(FILL).mix(&fill, fill.alpha());
    (
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            padding: UiRect::axes(px(5), px(1)),
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(background),
        children![(
            UiText,
            Text::new(label.to_string()),
            TextFont {
                font_size: FontSize::Px(11.0),
                ..default()
            },
            TextColor(if marked {
                theme::PHOSPHOR
            } else {
                theme::PHOSPHOR_MUTED
            }),
            // The stage behind a plate is whatever the camera is pointed at,
            // so the type carries its own contrast rather than trusting it.
            TextShadow::default(),
        )],
    )
}

/// What the stage wants a plate on, in the order the plates are built.
///
/// HULLS always - the ship being built and the seeded spacecraft alike: they
/// are the document's citizens, there is a handful of them, and which grey
/// shape is Derelict Hulk 3 is a question the stage could not answer.
/// Everything else only while it is marked or under the pointer - a range of
/// fifteen rocks with fifteen labels on it is a rail, not a stage.
///
/// Scenario-node only, like the grid: inside a ship the stage holds one hull,
/// and the crumb at the top of the screen already names it.
fn wanted_plates(
    context: &EditContext,
    selected: &SelectedNode,
    hovered: &HoveredNode,
    q_ships: &Query<(Entity, &ShipNode, &NodeId, &ChildOf)>,
    q_objects: &Query<(Entity, &ObjectNode, &NodeId, &ChildOf)>,
) -> Vec<(Entity, String, bool)> {
    let Some(scenario) = context.scenario() else {
        return Vec::new();
    };
    if context.ship().is_some() {
        return Vec::new();
    }
    let owned = |owner: &ChildOf| owner.parent() == scenario;
    let mut hulls: Vec<(Entity, String, String)> = q_ships
        .iter()
        .filter(|(_, _, _, owner)| owned(owner))
        .map(|(node, ship, id, _)| (node, id.0.clone(), ship.name.clone()))
        .chain(
            q_objects
                .iter()
                .filter(|(_, object, _, owner)| {
                    owned(owner) && matches!(object.kind, ScenarioObjectKind::Spaceship(_))
                })
                .map(|(node, object, id, _)| (node, id.0.clone(), object.name.clone())),
        )
        .collect();
    // In the rail's order, not the archetype walk's: two runs of the same
    // document put the same plates in the same place.
    hulls.sort_by(|(_, left, _), (_, right, _)| id_order(left).cmp(&id_order(right)));
    let mut wanted: Vec<(Entity, String, bool)> = hulls
        .into_iter()
        .map(|(node, id, name)| (node, label_of(&name, &id), selected.0 == Some(node)))
        .collect();
    for node in [selected.0, hovered.0].into_iter().flatten() {
        if wanted.iter().any(|(listed, _, _)| *listed == node) {
            continue;
        }
        if let Ok((_, object, id, _)) = q_objects.get(node) {
            wanted.push((
                node,
                label_of(&object.name, &id.0),
                selected.0 == Some(node),
            ));
        }
    }
    wanted
}

/// The name the tree would draw, as one string: the plate has no second column
/// to put an ordinal in.
fn label_of(name: &str, id: &str) -> String {
    let (label, ordinal) = tree_text(name, id);
    if ordinal.is_empty() {
        label
    } else {
        format!("{label} {ordinal}")
    }
}

/// Rebuild the set of plates when it changes, and put every plate over the
/// node it names.
///
/// Two jobs in one system because they share the wanted set, and because a
/// plate spawned this frame has no size to centre by until the next one - the
/// position pass runs over whatever is already there, exactly as the callout's
/// does.
#[expect(
    clippy::too_many_arguments,
    reason = "the wanted set reads the document, the placing reads the camera"
)]
pub(crate) fn sync_nameplates(
    mut commands: Commands,
    skin: Res<UiSkin>,
    context: Res<EditContext>,
    selected: Res<SelectedNode>,
    hovered: Res<HoveredNode>,
    q_ships: Query<(Entity, &ShipNode, &NodeId, &ChildOf)>,
    q_objects: Query<(Entity, &ObjectNode, &NodeId, &ChildOf)>,
    q_children: Query<&Children>,
    q_bounds: Query<&avian3d::prelude::ColliderAabb, Without<avian3d::prelude::Sensor>>,
    q_poses: Query<&GlobalTransform>,
    cameras: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    layers: Query<Entity, With<PlateLayer>>,
    fresh: Query<(), Added<PlateLayer>>,
    plates: Query<(&NamePlate, &mut Node, &mut Visibility, &ComputedNode)>,
    mut shown: Local<Vec<(Entity, String, bool)>>,
) {
    let wanted = wanted_plates(&context, &selected, &hovered, &q_ships, &q_objects);
    // A fresh layer holds no plates whatever this Local remembers: the layer
    // dies with the editor scene while a `Local` survives the state round-trip.
    if !fresh.is_empty() {
        shown.clear();
    }
    if *shown != wanted {
        if let Ok(layer) = layers.single() {
            commands.entity(layer).despawn_related::<Children>();
            commands.entity(layer).with_children(|layer| {
                for (node, label, marked) in &wanted {
                    layer.spawn((
                        Name::new(format!("Name Plate {label}")),
                        NamePlate(*node),
                        plate(label, *marked, *skin),
                    ));
                }
            });
            shown.clone_from(&wanted);
        }
    }

    let Some((camera, camera_pose)) = cameras.iter().next() else {
        return;
    };
    let Some(viewport) = camera.logical_viewport_size() else {
        return;
    };
    let mut standing = Vec::new();
    for (plate, mut node, mut visibility, computed) in plates {
        // Over the TOP of the node, not its middle: a label in the middle of a
        // hull is a label on the hull.
        let top = node_bounds(plate.0, &q_children, &q_bounds).map_or_else(
            || q_poses.get(plate.0).ok().map(|pose| pose.translation()),
            |bounds| {
                Some(Vec3::new(
                    bounds.center().x,
                    bounds.max.y,
                    bounds.center().z,
                ))
            },
        );
        let spot = top.and_then(|world| camera.world_to_viewport(camera_pose, world).ok());
        let Some(spot) = spot else {
            // Behind the eye, or the node is gone: a plate with nowhere to be
            // hides rather than pinning itself to a corner.
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            continue;
        };
        // Last frame's size, as the callout does: a plate that jumped a few
        // pixels once per rename is cheaper than a layout pass to place a label.
        let size = computed.size() * computed.inverse_scale_factor();
        let spot = clear_of(spot, size + CROWD_GAP, viewport, &mut standing);
        let Some(corner) = hang_at(spot, PLATE_HANG, computed, viewport) else {
            // Beside the frame rather than behind the eye: in front of the
            // camera and off to one side still projects, and it projects to a
            // point that is not on screen.
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            continue;
        };
        if *visibility != Visibility::Inherited {
            *visibility = Visibility::Inherited;
        }
        let left = px(corner.x);
        let top = px(corner.y);
        if node.left != left {
            node.left = left;
        }
        if node.top != top {
            node.top = top;
        }
    }
}

#[cfg(test)]
mod tests;
