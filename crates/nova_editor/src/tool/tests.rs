use bevy::ecs::system::RunSystemOnce;
use nova_ui::prelude::InputMode;

use super::*;
use crate::{config::EditorStatus, node::ObjectChoice};

fn tool_app() -> App {
    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<StageTool>()
        .init_resource::<EditorViewType>()
        .init_resource::<EditorStatus>()
        .init_resource::<Time>()
        .init_resource::<SelectedNode>()
        .insert_resource(HeldPart(None));
    app
}

fn tap(app: &mut App, keys: &[KeyCode]) {
    let mut input = ButtonInput::<KeyCode>::default();
    for key in keys {
        input.press(*key);
    }
    app.world_mut().insert_resource(input);
    app.world_mut()
        .run_system_once(tool_keys)
        .expect("the keys are read");
}

fn tool(app: &App) -> StageToolType {
    app.world().resource::<StageTool>().tool
}

#[test]
fn q_w_e_and_r_pick_the_tools() {
    let mut app = tool_app();
    for (key, wanted) in [
        (KeyCode::KeyQ, StageToolType::Select),
        (KeyCode::KeyE, StageToolType::Turn),
        (KeyCode::KeyR, StageToolType::Scale),
        (KeyCode::KeyW, StageToolType::Move),
    ] {
        tap(&mut app, &[key]);
        assert_eq!(tool(&app), wanted);
    }
}

/// The bar shows no tools in Events, so a letter there must not change the
/// tool or the axes the Scene tab comes back to.
#[test]
fn tool_keys_do_nothing_in_events() {
    let mut app = tool_app();
    *app.world_mut().resource_mut::<EditorViewType>() = EditorViewType::Events;
    let before = *app.world().resource::<StageTool>();

    for key in [
        KeyCode::KeyQ,
        KeyCode::KeyE,
        KeyCode::KeyR,
        KeyCode::KeyW,
        KeyCode::KeyX,
    ] {
        tap(&mut app, &[key]);
        assert_eq!(*app.world().resource::<StageTool>(), before, "{key:?}");
    }
}

#[test]
fn tool_keys_leave_r_to_a_part_in_hand() {
    let mut app = tool_app();
    app.world_mut()
        .insert_resource(HeldPart(Some("hull".into())));

    tap(&mut app, &[KeyCode::KeyR]);

    assert_eq!(tool(&app), StageToolType::Move);
}

#[test]
fn a_chord_is_not_a_tool_key() {
    let mut app = tool_app();
    tap(&mut app, &[KeyCode::ControlLeft, KeyCode::KeyE]);
    tap(&mut app, &[KeyCode::AltLeft, KeyCode::KeyR]);

    assert_eq!(tool(&app), StageToolType::Move);
}

/// The mode gate is the schedule's: this is the same run condition, read over
/// the modes that own the letters.
#[test]
fn tool_keys_are_ignored_in_a_text_field() {
    let mut app = tool_app();
    app.add_systems(
        Update,
        tool_keys.run_if(nova_ui::prelude::in_input_mode(InputMode::Normal)),
    );
    for mode in [InputMode::Insert, InputMode::Browse, InputMode::Bind] {
        app.insert_resource(mode);
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyE);
        app.world_mut().insert_resource(input);

        app.update();

        assert_eq!(tool(&app), StageToolType::Move, "{mode:?}");
    }
}

#[test]
fn x_swaps_the_handle_axes_and_says_so() {
    let mut app = tool_app();

    tap(&mut app, &[KeyCode::KeyX]);

    assert_eq!(
        app.world().resource::<StageTool>().space,
        GizmoSpaceType::Local
    );
    let line = app.world().resource::<EditorStatus>().line();
    assert_eq!(line.map(|(line, _)| line), Some("handles: local axes"));
}

fn status_after_scale(app: &mut App, node: Entity) -> Option<String> {
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));
    app.world_mut().resource_mut::<StageTool>().tool = StageToolType::Scale;
    app.world_mut()
        .run_system_once(sync_scale_refusal)
        .expect("the refusal is read");
    app.world()
        .resource::<EditorStatus>()
        .line()
        .map(|(line, _)| line.to_string())
}

#[test]
fn scale_is_refused_on_a_ship() {
    let mut app = tool_app();
    let ship = app.world_mut().spawn(ShipNode::default()).id();

    let line = status_after_scale(&mut app, ship).expect("a refusal");

    assert!(line.contains("no size to scale"), "{line}");
}

#[test]
fn scale_says_nothing_on_a_node_with_a_size() {
    let mut app = tool_app();
    let rock = app.world_mut().spawn(ObjectChoice::Asteroid.stock()).id();

    assert_eq!(status_after_scale(&mut app, rock), None);
}

#[test]
fn every_object_size_reads_back_what_was_written() {
    for choice in [
        ObjectChoice::Anchor,
        ObjectChoice::Asteroid,
        ObjectChoice::Beacon,
        ObjectChoice::Planet,
    ] {
        let mut kind = choice.stock().kind;
        set_size(&mut kind, Meters(321.0));
        assert_eq!(size_of(&kind), Some(Meters(321.0)), "{choice:?}");
    }
}
