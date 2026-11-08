//! The canvas's right-click menu: Paste and Clean Up, and Duplicate and Delete
//! on the block under the pointer.
//!
//! The right button also pans the canvas, so a press is a click only while it
//! stays put (see [`RightPress`]). Every row carries
//! its [`MenuVerbType`], so [`verb_state`] decides it as it decides the bar
//! menus' rows.

use bevy::{
    input::mouse::AccumulatedMouseMotion,
    picking::pointer::{PointerId, PointerInteraction},
    prelude::*,
    ui::UiGlobalTransform,
    ui_widgets::{observe, Activate},
};
use nova_ui::prelude::*;

use super::{drag::CanvasBlock, rect_of, CanvasBoard, CanvasView, CanvasViewport, FLOW_GAP};
use crate::{
    camera::ViewportPointer,
    clipboard::{duplicate_item, paste_item},
    config::{EditorViewType, SelectedNode},
    event::{EventNode, ScriptNodes},
    history::commit_edit_now,
    node::EditContext,
    placement::delete_selected_node,
    project::verbs::{pressed, refuse},
    ui::{
        layer,
        menu_context::{verb_state, MenuFacts, MenuVerbType, VerbState},
        stage_menu::RightPress,
    },
    ExampleStates,
};

/// Where Clean Up stands the column: the canvas's top left, past its margin.
const COLUMN_AT: Vec2 = Vec2::splat(super::FLOW_MARGIN);

/// The right press over the canvas, while it is held.
#[derive(Resource, Default)]
pub(crate) struct CanvasRightPress(RightPress);

/// A right click on the canvas opens its menu at the pointer. A right drag is
/// the pan's.
#[expect(
    clippy::too_many_arguments,
    reason = "a click reads the pointer, the hit, the canvas and the menu rule"
)]
pub(crate) fn canvas_context_menu(
    mut commands: Commands,
    (mouse, motion): (Res<ButtonInput<MouseButton>>, Res<AccumulatedMouseMotion>),
    pointer: Res<ViewportPointer>,
    view: Res<EditorViewType>,
    viewports: Query<(&ComputedNode, &UiGlobalTransform), With<CanvasViewport>>,
    (pointers, blocks, parents): (
        Query<(&PointerId, &PointerInteraction)>,
        Query<&CanvasBlock>,
        Query<&ChildOf>,
    ),
    facts: MenuFacts,
    mut selected: ResMut<SelectedNode>,
    mut press: ResMut<CanvasRightPress>,
    roots: Query<Entity, With<PopupRoot>>,
) {
    if !view.is_events() {
        press.0.press(None);
        return;
    }
    let over = |at: Vec2| {
        viewports
            .single()
            .is_ok_and(|(node, transform)| rect_of(node, transform).contains(at))
    };
    if mouse.just_pressed(MouseButton::Right) {
        press.0.press(pointer.at.filter(|at| over(*at)));
        return;
    }
    press.0.moved(motion.delta);
    if !mouse.just_released(MouseButton::Right) {
        return;
    }
    let Some(at) = press.0.release(pointer.at) else {
        return;
    };
    let block = pointers
        .iter()
        .filter(|(id, _)| id.is_mouse())
        .find_map(|(_, interaction)| interaction.get_nearest_hit())
        .and_then(|(hit, _)| {
            std::iter::once(*hit)
                .chain(parents.iter_ancestors(*hit))
                .find_map(|walked| blocks.get(walked).ok())
        })
        .map(|block| block.node);
    if let Some(node) = block.filter(|node| !selected.contains(*node)) {
        selected.set(Some(node));
    }
    let menu = facts.context(&selected);
    let shown = |verb| verb_state(verb, &menu) != VerbState::Hidden;
    close_popups(&mut commands, &roots);
    let popup = spawn_popup(&mut commands, at, layer::MENU_Z, |rows| {
        if shown(MenuVerbType::Paste) {
            rows.spawn((
                Name::new("Canvas Paste"),
                MenuVerbType::Paste,
                popup_row("Paste", "Ctrl+V"),
                observe(paste_item),
            ));
        }
        if shown(MenuVerbType::CleanUp) {
            rows.spawn((
                Name::new("Canvas Clean Up"),
                MenuVerbType::CleanUp,
                popup_row("Clean Up", ""),
                observe(on_clean_up),
            ));
        }
        if block.is_none() {
            return;
        }
        if shown(MenuVerbType::Duplicate) {
            rows.spawn((
                Name::new("Canvas Duplicate"),
                MenuVerbType::Duplicate,
                popup_row("Duplicate", "Ctrl+D"),
                observe(duplicate_item),
            ));
        }
        if shown(MenuVerbType::Delete) {
            rows.spawn((
                Name::new("Canvas Delete"),
                MenuVerbType::Delete,
                popup_row("Delete", "Del"),
                observe(delete_selected_node),
            ));
        }
    });
    commands
        .entity(popup)
        .insert(DespawnOnExit(ExampleStates::Editor));
}

/// Clean Up.
fn on_clean_up(_activate: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| pressed(world, clean_up));
}

/// Where each of `heights` stands in one column from [`COLUMN_AT`], top to
/// bottom, [`FLOW_GAP`] apart.
pub(crate) fn column(heights: &[f32]) -> Vec<Vec2> {
    let mut y = COLUMN_AT.y;
    heights
        .iter()
        .map(|height| {
            let at = Vec2::new(COLUMN_AT.x, y);
            y += height + FLOW_GAP;
            at
        })
        .collect()
}

/// Stand every script of the open scenario in one column, in script order, as
/// one step. The canvas then looks at the column's top.
pub(crate) fn clean_up(world: &mut World) {
    let Some(scenario) = world.resource::<EditContext>().scenario() else {
        return;
    };
    let hats = world
        .run_system_cached_with(
            |In(scenario): In<Entity>, script: ScriptNodes| script.events_of(scenario),
            scenario,
        )
        .unwrap_or_default();
    if hats.is_empty() {
        refuse(world, "there is no script to clean up");
        return;
    }
    let heights: Vec<f32> = hats
        .iter()
        .map(|hat| {
            let root = world.resource::<CanvasBoard>().roots.get(hat).copied();
            root.and_then(|root| world.get::<ComputedNode>(root))
                .map_or(0.0, |node| node.size().y * node.inverse_scale_factor)
        })
        .collect();
    let mut moved = false;
    for (hat, at) in hats.iter().zip(column(&heights)) {
        if let Some(mut event) = world.get_mut::<EventNode>(*hat) {
            if event.position != Some(at) {
                event.position = Some(at);
                moved = true;
            }
        }
    }
    world.resource_mut::<CanvasView>().pan = Vec2::ZERO;
    if moved {
        commit_edit_now(world, "Clean Up");
    }
}
