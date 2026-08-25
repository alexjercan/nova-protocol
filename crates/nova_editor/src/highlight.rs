//! Cross-highlight: the one node the pointer rests on, and the two surfaces
//! that answer for it.
//!
//! The rail and the stage show the same document twice, and until now nothing
//! carried a node from one to the other. A tree of minted ids beside a stage
//! of grey hulls meant clicking a row to find out which thing it names - and
//! that click also moves the camera and the selection, so finding out costs
//! the state you were working in.
//!
//! One resource, filled from whichever surface the pointer is over and read
//! back by both: [`crate::stage::draw_node_marks`] boxes it on the stage, and
//! [`paint_hovered_rows`] lights its row in the rail.

use bevy::{
    picking::hover::{HoverMap, Hovered},
    prelude::*,
};
use nova_ui::{
    prelude::UiSkin,
    widget::{list_row_colors, Selected},
};

use crate::{
    config::{HoveredNode, SceneRow},
    node::{node_of_view, EditContext, NodeView, SectionNode},
};

/// Name the node under the pointer, from the rail or from the stage.
///
/// The two sources cannot both answer at once - the rail is drawn over the
/// stage and blocks the picking ray - so the order between them is only a
/// tie-break, and the rail is asked first because its rows are what the tree
/// is FOR.
pub(crate) fn sync_hovered_node(
    hover_map: Res<HoverMap>,
    context: Res<EditContext>,
    rows: Query<(&SceneRow, &Hovered)>,
    q_views: Query<&ChildOf, With<NodeView>>,
    q_sections: Query<&ChildOf, With<SectionNode>>,
    mut hovered: ResMut<HoveredNode>,
) {
    let from_rail = rows
        .iter()
        .find(|(_, hovered)| hovered.get())
        .map(|(row, _)| row.0);
    let from_stage = nearest_view(&hover_map, &q_views).and_then(|view| {
        hovered_node_of_view(view, context.ship().is_some(), &q_views, &q_sections)
    });
    let wanted = from_rail.or(from_stage);
    if hovered.0 != wanted {
        hovered.0 = wanted;
    }
}

/// The nearest [`NodeView`] any pointer is over.
///
/// Nearest, because the hover map holds every hit that did not block the ones
/// behind it, and a trigger volume's owner standing behind a hull is not what
/// the pointer is pointing at.
fn nearest_view(hover_map: &HoverMap, q_views: &Query<&ChildOf, With<NodeView>>) -> Option<Entity> {
    hover_map
        .values()
        .flat_map(|hits| hits.iter())
        .filter(|(entity, _)| q_views.contains(**entity))
        .min_by(|(_, one), (_, other)| one.depth.total_cmp(&other.depth))
        .map(|(entity, _)| *entity)
}

/// The node a stage hit lights: the one the TREE would list.
///
/// Inside a ship the tree lists sections, so a part's own view answers for
/// itself. Out at the scenario node it lists ships and objects, so a section's
/// view answers for its ship - the same rule
/// `crate::placement::staged_node_of_view` drags by, because both are asking
/// "which thing on the stage is this".
fn hovered_node_of_view(
    view: Entity,
    inside_ship: bool,
    q_views: &Query<&ChildOf, With<NodeView>>,
    q_sections: &Query<&ChildOf, With<SectionNode>>,
) -> Option<Entity> {
    let node = node_of_view(view, q_views)?;
    if inside_ship {
        return Some(node);
    }
    Some(
        q_sections
            .get(node)
            .map_or(node, |section_owner| section_owner.parent()),
    )
}

/// Light the row of a node the pointer found on the STAGE.
///
/// Painted here rather than by writing the row's `Hovered`, which belongs to
/// picking and would be set false again the same frame. The colours come from
/// the shared [`list_row_colors`], so a row lit from the stage and a row under
/// the pointer are the same row - there is no second highlight to learn.
///
/// Idempotent and every frame: nothing tracks who painted last, and the
/// reconciler in `nova_ui` may repaint any row on a skin change without
/// knowing about the stage.
pub(crate) fn paint_hovered_rows(
    skin: Res<UiSkin>,
    hovered: Res<HoveredNode>,
    mut rows: Query<(
        &SceneRow,
        &Hovered,
        Has<Selected>,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (row, pointer, selected, mut background, mut border) in &mut rows {
        let lit = pointer.get() || hovered.0 == Some(row.0);
        let (wanted_background, wanted_border) = list_row_colors(selected, lit, *skin);
        if background.0 != wanted_background {
            background.0 = wanted_background;
            border.set_all(wanted_border);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use nova_scenario::prelude::SectionSource;

    use super::*;
    use crate::node::ShipNode;

    /// A ship with one section, and the section's view.
    fn ship_with_a_part(world: &mut World) -> (Entity, Entity, Entity) {
        let ship = world.spawn(ShipNode::default()).id();
        let section = world
            .spawn((
                SectionNode {
                    source: SectionSource::Prototype("hull".to_string()),
                    modifications: Vec::new(),
                    binds: Vec::new(),
                },
                ChildOf(ship),
            ))
            .id();
        let view = world.spawn((NodeView, ChildOf(section))).id();
        (ship, section, view)
    }

    /// Which node a part on the stage stands for depends on which tree is
    /// being lit: out in the world the rail lists ships, so the ship lights;
    /// inside the ship it lists parts, so the part does.
    #[test]
    fn a_part_lights_the_row_the_tree_is_showing() {
        let mut world = World::new();
        let (ship, section, view) = ship_with_a_part(&mut world);

        let named = world
            .run_system_once(
                move |q_views: Query<&ChildOf, With<NodeView>>,
                      q_sections: Query<&ChildOf, With<SectionNode>>| {
                    (
                        hovered_node_of_view(view, false, &q_views, &q_sections),
                        hovered_node_of_view(view, true, &q_views, &q_sections),
                    )
                },
            )
            .expect("the lookup runs");

        assert_eq!(named.0, Some(ship), "at the scenario node, the ship");
        assert_eq!(named.1, Some(section), "inside the ship, the part");
    }

    /// A row lit from the stage wears the hover colours, and its neighbours
    /// stay as they were: a highlight that spread to the whole tree would say
    /// nothing about which node the pointer is on.
    #[test]
    fn a_stage_hover_lights_exactly_one_row() {
        let mut world = World::new();
        world.init_resource::<UiSkin>();
        let node = world.spawn_empty().id();
        let other = world.spawn_empty().id();
        world.insert_resource(HoveredNode(Some(node)));
        let lit = world
            .spawn((
                SceneRow(node),
                Hovered(false),
                BackgroundColor::DEFAULT,
                BorderColor::all(Color::NONE),
            ))
            .id();
        let dark = world
            .spawn((
                SceneRow(other),
                Hovered(false),
                BackgroundColor::DEFAULT,
                BorderColor::all(Color::NONE),
            ))
            .id();

        world
            .run_system_once(paint_hovered_rows)
            .expect("the paint runs");

        let skin = *world.resource::<UiSkin>();
        assert_eq!(
            world.get::<BackgroundColor>(lit).expect("a colour").0,
            list_row_colors(false, true, skin).0,
            "the named row wears the hover colours"
        );
        assert_eq!(
            world.get::<BackgroundColor>(dark).expect("a colour").0,
            list_row_colors(false, false, skin).0,
            "and every other row is left alone"
        );
    }

    /// The mark follows the selection, not the other way about: a selected row
    /// keeps its own paint while the pointer is somewhere else on the stage.
    #[test]
    fn a_selected_row_is_not_dimmed_by_a_hover_elsewhere() {
        let mut world = World::new();
        world.init_resource::<UiSkin>();
        let node = world.spawn_empty().id();
        let elsewhere = world.spawn_empty().id();
        world.insert_resource(HoveredNode(Some(elsewhere)));
        let row = world
            .spawn((
                SceneRow(node),
                Selected,
                Hovered(false),
                BackgroundColor::DEFAULT,
                BorderColor::all(Color::NONE),
            ))
            .id();

        world
            .run_system_once(paint_hovered_rows)
            .expect("the paint runs");

        let skin = *world.resource::<UiSkin>();
        assert_eq!(
            world.get::<BackgroundColor>(row).expect("a colour").0,
            list_row_colors(true, false, skin).0
        );
    }
}
