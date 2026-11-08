//! The canvas's own rules, as far as they hold without a window.

use bevy::{
    camera::{ImageRenderTarget, NormalizedRenderTarget},
    ecs::system::RunSystemOnce,
    picking::{
        backend::HitData,
        pointer::{Location, PointerId},
    },
    prelude::*,
    ui_widgets::{observe, Activate},
};
use nova_scenario::prelude::EventConfig;
use nova_ui::prelude::*;

use super::{
    canvas_hint_text, delete_reporter_key,
    drag::{
        clear_of, drive_canvas_drag, land, on_canvas_press, Candidate, CanvasBlock, CanvasGhost,
        DragSourceType, Held, Landing,
    },
    menu::{clean_up, column},
    popup::{on_more_chip, settle_block_popup, MoreChip},
    sync_canvas,
    view::{BlockView, ChipType, ChipView, OutlineType, PieceType, ViewBuilder},
    BlockPopup, CanvasBoard, CanvasDrag, CanvasView, CanvasWorld, ChipInput, EMPTY_CANVAS_HINT,
    LOOSE_BLOCK_HINT,
};
use crate::{
    camera::ViewportPointer,
    clipboard::{clipboard_keys, EditorClipboard},
    config::{EditorViewType, SelectedNode},
    event::{
        place_new, BlockItemType, BlockRoleType, BlockShapeType, BlockSlotType, EventNode,
        ExprChoice, ExpressionNode, ScriptNodes,
    },
    history::{
        history_keys,
        tests::{assert_one_step, baseline, depth, first_with, history_app, lowered, redo, undo},
        EditHistory, HISTORY_DEPTH,
    },
    placement::delete_key,
    problems::{Problem, ProblemSeverityType, Problems},
    ui::inspector::InspectorField,
};

fn chip(text: &str) -> PieceType {
    PieceType::Chip(ChipView {
        label: String::new(),
        text: text.to_string(),
        whole: text.to_string(),
        unit: "",
        kind: ChipType::Text,
        field: None,
    })
}

#[test]
fn a_dropped_hat_is_nudged_clear_of_the_hats_it_lands_on() {
    let upper = Rect::new(0.0, 0.0, 100.0, 50.0);
    let lower = Rect::new(0.0, 60.0, 100.0, 120.0);
    let size = Vec2::new(40.0, 30.0);

    let at = clear_of(Vec2::new(20.0, 10.0), size, &[upper, lower]);

    let own = Rect::from_corners(at, at + size);
    assert!(own.intersect(upper).is_empty() && own.intersect(lower).is_empty());
    assert_eq!(at.y, 10.0, "right of the column is nearer than under it");
    let wide = Rect::new(0.0, 0.0, 900.0, 50.0);
    assert_eq!(
        clear_of(Vec2::new(20.0, 10.0), size, &[wide]).x,
        20.0,
        "under a wide hat is nearer than past it"
    );
    assert_eq!(
        clear_of(Vec2::new(200.0, 10.0), size, &[upper, lower]),
        Vec2::new(200.0, 10.0),
        "a clear drop lands where it was let go"
    );
}

#[test]
fn an_edited_value_keeps_the_shape_so_a_scrub_survives_it() {
    let mut before = BlockView::preview(BlockItemType::Hat(EventConfig::ALL[0]));
    before.head.push(chip("10"));
    let mut after = before.clone();
    after.head[1] = chip("11");
    assert!(before.same_shape(&after));
    assert_ne!(before, after);
}

#[test]
fn a_new_piece_changes_the_shape_so_the_handler_rebuilds() {
    let before = BlockView::preview(BlockItemType::Hat(EventConfig::ALL[0]));
    let mut after = before.clone();
    after.head.push(PieceType::More(Entity::PLACEHOLDER));
    assert!(!before.same_shape(&after));
    assert_eq!(before.outline, OutlineType::Hat);
}

/// A handler dragged over the canvas is one step, and undoing it puts the
/// handler back where it stood.
#[test]
fn a_dropped_handler_is_one_undo_step() {
    let mut app = history_app();
    let hat = first_with::<EventNode>(&mut app);

    assert_one_step(&mut app, "Move Block", |app| {
        land(
            app.world_mut(),
            DragSourceType::Block(hat),
            Landing::Slot(BlockSlotType::Canvas(Vec2::new(120.0, 80.0))),
            &[],
        );
    });
}

/// A block fresh off the palette is one step too, however many nodes it makes.
#[test]
fn a_block_off_the_palette_is_one_undo_step() {
    let mut app = history_app();
    let hat = first_with::<EventNode>(&mut app);
    let item = BlockItemType::all()
        .into_iter()
        .find(|item| item.label() == "Sequence")
        .expect("the palette holds a Sequence");

    assert_one_step(&mut app, "Add Sequence", |app| {
        land(
            app.world_mut(),
            DragSourceType::Palette(item),
            Landing::Slot(BlockSlotType::Action {
                parent: hat,
                index: 0,
            }),
            &[],
        );
    });
}

/// A refused drop writes nothing and leaves no step to undo.
#[test]
fn a_refused_drop_leaves_no_step() {
    let mut app = history_app();
    let hat = first_with::<EventNode>(&mut app);
    baseline(&mut app);
    let before = lowered(&mut app);

    land(
        app.world_mut(),
        DragSourceType::Block(hat),
        Landing::Refused("nowhere to go".to_string()),
        &[],
    );
    app.update();

    assert_eq!(lowered(&mut app), before);
    assert_eq!(depth(&app), 0);
}

/// The rig with what the canvas keys read, and Del or a chord held down.
fn keyed_app(pressed: &[KeyCode]) -> App {
    let mut app = history_app();
    let mut keys = ButtonInput::<KeyCode>::default();
    for key in pressed {
        keys.press(*key);
    }
    app.insert_resource(keys);
    app.init_resource::<CanvasDrag>();
    app.init_resource::<EditorClipboard>();
    app
}

/// A handler on the canvas, a held press on it, and it marked.
fn press_on_a_hat(app: &mut App) -> Entity {
    let hat = first_with::<EventNode>(app);
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(hat));
    app.world_mut().resource_mut::<CanvasDrag>().candidate = Some(Candidate {
        source: DragSourceType::Block(hat),
        at: Vec2::ZERO,
        draggable: true,
    });
    hat
}

/// Del under a held press would take the block out from under the release.
#[test]
fn del_is_refused_while_a_block_is_pressed() {
    let mut app = keyed_app(&[KeyCode::Delete]);
    let hat = press_on_a_hat(&mut app);

    app.world_mut()
        .run_system_once(delete_key)
        .expect("the key runs");
    app.update();

    assert!(app.world().get_entity(hat).is_ok(), "the block stands");
}

/// A chord under a held press would stand a copy under the block in hand.
#[test]
fn the_clipboard_keys_are_refused_while_a_block_is_pressed() {
    let mut app = keyed_app(&[KeyCode::ControlLeft, KeyCode::KeyD]);
    press_on_a_hat(&mut app);
    baseline(&mut app);

    app.world_mut()
        .run_system_once(clipboard_keys)
        .expect("the key runs");
    app.update();

    assert_eq!(depth(&app), 0, "nothing landed");
}

/// However the block went, a drop that follows it says so and changes nothing.
#[test]
fn a_block_gone_from_under_the_hand_refuses_its_drop() {
    for landing in [
        Landing::Bin,
        Landing::Slot(BlockSlotType::Canvas(Vec2::new(10.0, 10.0))),
    ] {
        let mut app = history_app();
        let hat = first_with::<EventNode>(&mut app);
        app.world_mut().entity_mut(hat).despawn();
        baseline(&mut app);

        land(app.world_mut(), DragSourceType::Block(hat), landing, &[]);
        app.update();

        assert_eq!(depth(&app), 0);
    }
}

/// The slot's owner went while a palette block was in hand.
#[test]
fn a_slot_gone_from_under_the_hand_refuses_its_drop() {
    let mut app = history_app();
    let hat = first_with::<EventNode>(&mut app);
    let item = BlockItemType::all()
        .into_iter()
        .find(|item| item.label() == "Sequence")
        .expect("the palette holds a Sequence");
    app.world_mut().entity_mut(hat).despawn();
    baseline(&mut app);

    land(
        app.world_mut(),
        DragSourceType::Palette(item),
        Landing::Slot(BlockSlotType::Action {
            parent: hat,
            index: 0,
        }),
        &[],
    );
    app.update();

    assert_eq!(depth(&app), 0);
}

/// A click whose block went before the release selects nothing.
#[test]
fn a_click_released_on_a_dead_block_selects_nothing() {
    let mut app = keyed_app(&[]);
    app.init_resource::<CanvasView>();
    app.init_resource::<CanvasBoard>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.insert_resource(ViewportPointer {
        at: Some(Vec2::ZERO),
        ..default()
    });
    let hat = press_on_a_hat(&mut app);
    app.world_mut().resource_mut::<SelectedNode>().set(None);
    app.world_mut().entity_mut(hat).despawn();

    app.world_mut()
        .run_system_once(drive_canvas_drag)
        .expect("the drag runs");
    app.update();

    assert_eq!(app.world().resource::<SelectedNode>().primary(), None);
}

/// A comparison with a reporter in its left socket, and the reporter.
fn comparison_with_a_reporter(app: &mut App) -> (Entity, Entity) {
    let hat = first_with::<EventNode>(app);
    let world = app.world_mut();
    let filter = place_new(
        world,
        BlockItemType::Expr(ExprChoice::LessThan),
        BlockSlotType::Filter {
            parent: hat,
            index: 0,
        },
    )
    .expect("a comparison wraps itself in a filter");
    let root = world
        .run_system_once(move |script: ScriptNodes| script.operands_of(filter)[0])
        .expect("the walk runs");
    let reporter = place_new(
        world,
        BlockItemType::Query,
        BlockSlotType::Value {
            parent: root,
            index: 0,
        },
    )
    .expect("a reporter replaces a literal");
    app.update();
    (root, reporter)
}

/// A dead selection is a mark nothing can show and every verb must step round.
#[test]
fn del_on_a_reporter_clears_the_selection() {
    let mut app = keyed_app(&[KeyCode::Delete]);
    let (_, reporter) = comparison_with_a_reporter(&mut app);
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(reporter));

    app.world_mut()
        .run_system_once(delete_reporter_key)
        .expect("the key runs");
    app.update();

    assert!(app.world().get_entity(reporter).is_err());
    assert_eq!(app.world().resource::<SelectedNode>().primary(), None);
}

/// A comparison is the condition, not a value in it: Del on one would leave a
/// filter that compares nothing.
#[test]
fn del_leaves_a_comparison_standing() {
    let mut app = keyed_app(&[KeyCode::Delete]);
    let (root, _) = comparison_with_a_reporter(&mut app);
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(root));

    app.world_mut()
        .run_system_once(delete_reporter_key)
        .expect("the key runs");
    app.update();

    assert!(
        app.world().get_entity(root).is_ok(),
        "the comparison stands"
    );
    assert!(app.world().get::<ExpressionNode>(root).is_some());
}

/// Esc takes the block out of the hand: the ghost goes, the blocks it hid come
/// back, and nothing moved.
#[test]
fn a_cancelled_drag_puts_the_block_back() {
    let mut app = keyed_app(&[]);
    app.init_resource::<CanvasView>();
    app.init_resource::<CanvasBoard>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.insert_resource(ViewportPointer::default());
    let hat = first_with::<EventNode>(&mut app);
    let hidden = app
        .world_mut()
        .spawn((
            CanvasBlock {
                node: hat,
                faulted: false,
            },
            Node {
                display: Display::None,
                ..default()
            },
            ComputedNode::default(),
            bevy::ui::UiGlobalTransform::default(),
        ))
        .id();
    let ghost = app
        .world_mut()
        .spawn((CanvasGhost, Node::default(), UiTransform::default()))
        .id();
    baseline(&mut app);
    let before = lowered(&mut app);
    {
        let mut drag = app.world_mut().resource_mut::<CanvasDrag>();
        drag.held = Some(Held {
            source: DragSourceType::Block(hat),
            shape: BlockShapeType::Hat,
            grab: Vec2::ZERO,
            run: vec![hat],
            aim: None,
        });
        assert!(drag.cancel(), "a block was in hand");
        assert!(!drag.busy(), "the hand is empty");
        assert!(!drag.cancel(), "a second press has nothing to cancel");
    }

    app.world_mut()
        .run_system_once(drive_canvas_drag)
        .expect("the drag runs");
    app.update();

    assert!(app.world().get_entity(ghost).is_err(), "the ghost went");
    let shown = app.world().get::<Node>(hidden).expect("the block").display;
    assert_eq!(shown, Display::Flex, "the block came back");
    assert_eq!(lowered(&mut app), before);
    assert_eq!(depth(&app), 0);
}

/// S17: a ghost spawned with an `Auto` corner stood at the window's top left
/// for its first frame, so every lifted block flickered there.
#[test]
fn a_lifted_block_spawns_its_ghost_under_the_pointer() {
    let mut app = keyed_app(&[]);
    app.insert_resource(CanvasView {
        pan: Vec2::ZERO,
        zoom: 1.5,
    });
    app.init_resource::<CanvasBoard>();
    let mut buttons = ButtonInput::<MouseButton>::default();
    buttons.press(MouseButton::Left);
    app.insert_resource(buttons);
    let at = Vec2::new(300.0, 200.0);
    app.insert_resource(ViewportPointer {
        at: Some(at),
        ..default()
    });
    let item = super::palette::palette_items()[0];
    app.world_mut().resource_mut::<CanvasDrag>().candidate = Some(Candidate {
        source: DragSourceType::Palette(item),
        at: at - Vec2::new(10.0, 0.0),
        draggable: true,
    });

    app.world_mut()
        .run_system_once(drive_canvas_drag)
        .expect("the drag runs");

    let held = app
        .world()
        .resource::<CanvasDrag>()
        .held
        .clone()
        .expect("the press became a drag");
    let corner = at + held.grab * 1.5;
    let mut ghosts = app
        .world_mut()
        .query_filtered::<(&Node, &UiTransform), With<CanvasGhost>>();
    let (node, transform) = ghosts.single(app.world()).expect("one ghost");
    assert_eq!(
        (node.left, node.top),
        (px(corner.x), px(corner.y)),
        "under the pointer from its first frame"
    );
    assert_eq!(transform.scale, Vec2::splat(1.5), "at the canvas zoom");
}

/// The canvas rig: one handler drawn on a world, in Events.
fn drawn_canvas() -> (App, Entity) {
    let mut app = keyed_app(&[]);
    app.init_resource::<CanvasBoard>();
    app.init_resource::<Problems>();
    app.insert_resource(EditorViewType::Events);
    app.add_systems(Update, sync_canvas);
    app.world_mut().spawn(CanvasWorld);
    app.update();
    app.update();
    let hat = first_with::<EventNode>(&mut app);
    (app, hat)
}

/// The fault that makes a block redraw arrives by lint while the builder types.
#[test]
fn a_rebuild_waits_for_an_open_chip_field() {
    let (mut app, hat) = drawn_canvas();
    let chip = app
        .world_mut()
        .query_filtered::<Entity, With<CanvasBlock>>()
        .iter(app.world())
        .next()
        .expect("the canvas drew a block");
    let input = app.world_mut().spawn((ChipInput, ChildOf(chip))).id();

    app.world_mut().resource_mut::<Problems>().list = vec![Problem {
        severity: ProblemSeverityType::Error,
        message: "a fault".to_string(),
        node: Some(hat),
    }];
    app.update();
    assert!(app.world().get_entity(input).is_ok(), "the field stands");

    app.world_mut().entity_mut(input).despawn();
    app.update();
    let faulted = app
        .world_mut()
        .query::<&CanvasBlock>()
        .iter(app.world())
        .any(|block| block.faulted);
    assert!(faulted, "the rebuild owed lands once the field is gone");
}

/// A press in an open field places the caret. It must not lift the block.
#[test]
fn a_press_inside_a_text_field_does_not_start_a_block_drag() {
    let mut app = keyed_app(&[]);
    app.add_observer(on_canvas_press);
    let hat = first_with::<EventNode>(&mut app);
    let block = app
        .world_mut()
        .spawn(CanvasBlock {
            node: hat,
            faulted: false,
        })
        .id();
    let field = app
        .world_mut()
        .spawn((ChildOf(block), text_field(TextFieldSpec::new("10"))))
        .id();

    app.world_mut().trigger(PointerPress {
        entity: field,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::Image(ImageRenderTarget {
                    handle: Handle::default(),
                    scale_factor: 1.0,
                }),
                position: Vec2::ZERO,
            },
        ),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        count: 1,
    });
    app.update();

    assert!(!app.world().resource::<CanvasDrag>().busy());
}

/// The rig with a block's pop-up open on the first handler.
fn popup_app() -> (App, Entity) {
    let mut app = keyed_app(&[]);
    app.init_resource::<BlockPopup>();
    app.init_resource::<ViewportPointer>();
    app.init_resource::<InputMode>();
    app.insert_resource(EditorViewType::Events);
    app.add_systems(Update, settle_block_popup);
    let hat = first_with::<EventNode>(&mut app);
    baseline(&mut app);
    let entity = app
        .world_mut()
        .spawn((MoreChip(hat), Node::default(), observe(on_more_chip)))
        .id();
    app.world_mut().trigger(Activate { entity });
    app.world_mut().flush();
    assert!(app.world().resource::<BlockPopup>().is_open());
    (app, hat)
}

/// Land one Sequence under `hat`, as an edit made inside the pop-up.
fn edit_in_popup(app: &mut App, hat: Entity) {
    let item = BlockItemType::all()
        .into_iter()
        .find(|item| item.label() == "Sequence")
        .expect("the palette holds a Sequence");
    land(
        app.world_mut(),
        DragSourceType::Palette(item),
        Landing::Slot(BlockSlotType::Action {
            parent: hat,
            index: 0,
        }),
        &[],
    );
    app.update();
}

/// Esc takes back all a pop-up did, and leaves nothing to undo or redo.
#[test]
fn esc_in_a_block_popup_changes_nothing() {
    let (mut app, hat) = popup_app();
    let before = lowered(&mut app);
    edit_in_popup(&mut app, hat);
    edit_in_popup(&mut app, hat);
    assert_ne!(lowered(&mut app), before);

    assert!(app.world_mut().resource_mut::<BlockPopup>().cancel());
    app.update();
    app.update();

    assert!(!app.world().resource::<BlockPopup>().is_open());
    assert_eq!(lowered(&mut app), before);
    assert_eq!(depth(&app), 0);
    assert!(!app.world().resource::<EditHistory>().can_redo());
}

/// However many fields a pop-up touched, it is one step to undo.
#[test]
fn a_click_away_commits_a_block_popup_as_one_step() {
    let (mut app, hat) = popup_app();
    let before = lowered(&mut app);
    edit_in_popup(&mut app, hat);
    edit_in_popup(&mut app, hat);
    assert_eq!(depth(&app), 2);
    let after = lowered(&mut app);

    app.world_mut().resource_mut::<BlockPopup>().keep();
    app.update();
    app.update();

    assert!(!app.world().resource::<BlockPopup>().is_open());
    assert_eq!(depth(&app), 1);
    let history = app.world().resource::<EditHistory>();
    assert_eq!(history.undo_label(), Some("Edit Block"));
    undo(&mut app);
    assert_eq!(lowered(&mut app), before);
    redo(&mut app);
    assert_eq!(lowered(&mut app), after);
}

/// The block `node` shows.
fn block_view(app: &mut App, node: Entity) -> BlockView {
    app.world_mut()
        .run_system_once(move |script: ScriptNodes| {
            let faults = std::collections::HashMap::new();
            ViewBuilder {
                script: &script,
                faults: &faults,
            }
            .any(node)
        })
        .expect("the walk runs")
        .expect("the block has a view")
}

/// Each action and filter drops its Kind row off the line, so `...` is the only
/// way to retype it.
#[test]
fn every_action_and_filter_block_reaches_its_kind_row() {
    let mut app = keyed_app(&[]);
    let hat = first_with::<EventNode>(&mut app);
    let mut missing = Vec::new();
    for item in BlockItemType::all() {
        let slot = match item {
            BlockItemType::Action(_) => BlockSlotType::Action {
                parent: hat,
                index: 0,
            },
            BlockItemType::Filter(_) => BlockSlotType::Filter {
                parent: hat,
                index: 0,
            },
            _ => continue,
        };
        let node = place_new(app.world_mut(), item, slot).expect("the block fits");
        app.update();
        let view = block_view(&mut app, node);
        let reached = view
            .head
            .iter()
            .any(|piece| matches!(piece, PieceType::More(held) if *held == node));
        if !reached {
            missing.push(item.label());
        }
    }
    assert!(missing.is_empty(), "no way to the Kind row: {missing:?}");
}

/// The press that leaves also refuses the field: the first click shows why, and
/// the second leaves and lets the value go.
#[test]
fn a_refused_field_holds_the_popup_for_one_click() {
    let (mut app, hat) = popup_app();
    let field = app
        .world_mut()
        .spawn((
            InspectorField::of_operand(hat),
            TextFieldError("not a number".to_string()),
        ))
        .id();

    app.world_mut().resource_mut::<BlockPopup>().keep();
    app.update();
    app.update();
    assert!(app.world().resource::<BlockPopup>().is_open());
    assert!(app.world().entity(field).contains::<TextFieldError>());

    app.world_mut().resource_mut::<BlockPopup>().keep();
    app.update();
    app.update();
    assert!(!app.world().resource::<BlockPopup>().is_open());
    assert!(
        !app.world().entity(field).contains::<TextFieldError>(),
        "no red text waits for the next open"
    );
}

/// Every verb gated on Normal goes quiet under a pop-up, through one claim.
#[test]
fn an_open_popup_takes_the_keyboard_from_the_verbs() {
    let (mut app, _) = popup_app();
    let world = app.world_mut();
    world.insert_resource(State::new(crate::ExampleStates::Editor));
    world.insert_resource(crate::keybind::EditorRebind::default());
    world.init_resource::<Messages<ClaimKeyboard>>();

    world
        .run_system_once(crate::declare_editor_keyboard_owner)
        .expect("the claim runs");

    let claimed: Vec<InputMode> = world
        .resource_mut::<Messages<ClaimKeyboard>>()
        .drain()
        .map(|claim| claim.0)
        .collect();
    assert_eq!(claimed, vec![InputMode::Browse]);
}

/// A step taken under the pop-up would leave its `since` behind, and Esc would
/// have nothing to take back.
#[test]
fn undo_under_a_popup_closes_it_first() {
    let (mut app, hat) = popup_app();
    let before = lowered(&mut app);
    edit_in_popup(&mut app, hat);
    edit_in_popup(&mut app, hat);
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::ControlLeft);
        keys.press(KeyCode::KeyZ);
    }

    app.world_mut()
        .run_system_once(history_keys)
        .expect("the key runs");
    app.update();
    app.update();

    assert!(!app.world().resource::<BlockPopup>().is_open());
    assert_eq!(lowered(&mut app), before, "both edits went as one step");
    assert_eq!(depth(&app), 0);
    assert!(app.world().resource::<EditHistory>().can_redo());
}

/// An Esc that cannot take the edits back says so rather than failing quietly.
#[test]
fn esc_says_so_when_the_history_lost_the_popups_start() {
    let (mut app, hat) = popup_app();
    let before = lowered(&mut app);
    for _ in 0..=HISTORY_DEPTH {
        edit_in_popup(&mut app, hat);
    }

    assert!(app.world_mut().resource_mut::<BlockPopup>().cancel());
    app.update();
    app.update();

    assert_ne!(lowered(&mut app), before);
    let status = app.world().resource::<crate::config::EditorStatus>();
    let (line, _) = status.said().expect("the status line speaks");
    assert!(line.contains("stayed"), "{line}");
}

#[test]
fn the_palette_lists_its_groups_in_build_order_with_only_hats_first() {
    let items = super::palette::palette_items();
    let roles: Vec<BlockRoleType> = items.iter().map(|item| item.role()).collect();
    let mut order = roles.clone();
    order.dedup();
    assert_eq!(order, BlockRoleType::ALL.to_vec(), "one run per group");
    let mut app = history_app();
    let shapes: Vec<BlockShapeType> = items.iter().map(|item| item.shape()).collect();
    let alone: Vec<bool> = app
        .world_mut()
        .run_system_once(move |script: ScriptNodes| {
            shapes
                .iter()
                .map(|shape| {
                    crate::event::refusal(&script, *shape, None, BlockSlotType::Canvas(Vec2::ZERO))
                        .is_none()
                })
                .collect::<Vec<bool>>()
        })
        .expect("the script reads");
    for (item, alone) in items.iter().zip(alone) {
        let hat = item.shape() == BlockShapeType::Hat;
        assert_eq!(
            hat,
            item.role() == BlockRoleType::Event,
            "{} is a hat exactly when it is filed under Events",
            item.label()
        );
        assert_eq!(alone, hat, "{} stands alone only as a hat", item.label());
    }
}

/// S4: a rail click shows one group, a second click or All shows them all.
#[test]
fn a_rail_click_shows_one_group_and_a_second_click_shows_all() {
    use BlockRoleType::{Action, Event};

    use super::palette::PaletteGroup;
    let all = PaletteGroup::default();
    let events = all.after_press(Some(Event));
    assert_eq!(events, PaletteGroup(Some(Event)));
    assert_eq!(events.label(), "Events");
    assert_eq!(events.after_press(Some(Event)), all, "a second click");
    assert_eq!(events.after_press(None), all, "All");
    assert_eq!(
        events.after_press(Some(Action)),
        PaletteGroup(Some(Action)),
        "another group takes over"
    );
    assert_eq!(all.label(), "All");
}

/// S4: the rail's group and the filter text both narrow the palette.
#[test]
fn the_palette_shows_what_the_group_and_the_filter_both_allow() {
    use BlockRoleType::{Event, Value};

    use super::palette::{palette_items, palette_shows, PaletteGroup};
    let items = palette_items();
    let shown = |wanted: &str, group: PaletteGroup| -> Vec<BlockItemType> {
        items
            .iter()
            .copied()
            .filter(|item| palette_shows(*item, wanted, group))
            .collect()
    };
    assert_eq!(shown("", PaletteGroup::default()).len(), items.len());
    let events = shown("", PaletteGroup(Some(Event)));
    assert!(!events.is_empty());
    assert!(events.iter().all(|item| item.role() == Event));
    assert_eq!(
        events.len(),
        items.iter().filter(|item| item.role() == Event).count(),
        "every hat, and only hats"
    );
    let start = shown("start", PaletteGroup(Some(Event)));
    assert!(!start.is_empty());
    assert!(start.len() < events.len(), "the text narrows the group");
    assert!(start
        .iter()
        .all(|item| item.role() == Event && item.label().to_lowercase().contains("start")));
    assert!(
        shown("start", PaletteGroup(Some(Value))).is_empty(),
        "a hat's name finds nothing among the values"
    );
}

#[test]
fn the_canvas_says_how_to_start_and_why_a_loose_block_will_not_stand() {
    assert_eq!(
        canvas_hint_text(None, false, false),
        Some(EMPTY_CANVAS_HINT),
        "an empty scenario says how to start"
    );
    assert_eq!(
        canvas_hint_text(None, true, true),
        None,
        "a script hides it"
    );
    assert_eq!(
        canvas_hint_text(Some((BlockShapeType::Stack, false)), true, true),
        Some(LOOSE_BLOCK_HINT),
        "an action over bare canvas says it will not stand"
    );
    assert_eq!(
        canvas_hint_text(Some((BlockShapeType::Stack, true)), true, true),
        None,
        "an action over a lit slot says nothing"
    );
    assert_eq!(
        canvas_hint_text(Some((BlockShapeType::Hat, false)), true, false),
        None,
        "a hat over bare canvas will stand"
    );
}

/// Clean Up stands every script in one column, in script order, as one undo
/// step.
#[test]
fn clean_up_stands_the_scripts_in_one_column_as_one_step() {
    assert_eq!(
        column(&[10.0, 20.0, 5.0]),
        vec![
            Vec2::new(24.0, 24.0),
            Vec2::new(24.0, 62.0),
            Vec2::new(24.0, 110.0)
        ],
        "each script stands one gap under the last"
    );
    let mut app = history_app();
    app.init_resource::<CanvasBoard>()
        .init_resource::<CanvasView>();
    assert_one_step(&mut app, "Clean Up", |app| clean_up(app.world_mut()));
    let world = app.world_mut();
    let placed: Vec<Vec2> = world
        .query::<&EventNode>()
        .iter(world)
        .filter_map(|event| event.position)
        .collect();
    assert!(!placed.is_empty(), "the fixture has a script");
    assert!(
        placed.iter().all(|at| at.x == placed[0].x),
        "one column: {placed:?}"
    );
}
