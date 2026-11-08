//! What a step is, what undoing one restores, and what survives it.

use bevy::{
    camera::{ImageRenderTarget, NormalizedRenderTarget},
    ecs::system::RunSystemOnce,
    picking::pointer::{Location, PointerId},
    ui::InteractionDisabled,
    ui_widgets::observe,
};
use nova_gameplay::prelude::{GuardedLeave, UnsavedWork};
use nova_scenario::prelude::SectionSource;

use super::*;
use crate::{
    config::{EditorStatus, SceneRow, StyleChoice},
    frame::CameraFraming,
    node::{
        ensure_document, reflow_auto_ships, sync_stage_frame, ObjectBodyStale, ObjectChoice,
        StageFrame,
    },
    placement::{
        create_blank_ship, create_scenario_object, delete_selected_node, on_stage_drag_end,
        StageDrag,
    },
    project::{ship::open_ship_node, verbs::go_back},
    ui::{
        menu::{
            menu_item_row, sync_history_menu, sync_unsaved_mark, MenuHistoryItem, MenuItemLabel,
            MenuLead, MenuTail, UnsavedMark,
        },
        on_row_trash, on_skin_toggle, on_style_choice,
    },
};

/// The document as a save would write it.
pub(crate) fn lowered(app: &mut App) -> String {
    format!("{:?}", crate::bundle::tests::lower(app.world_mut()))
}

/// Take one step, as the keys and the menu rows do.
pub(crate) fn undo(app: &mut App) {
    with_history(app.world_mut(), |history, world| {
        history.step(world, Direction::Undo);
    });
    app.update();
}

/// Take one step forward.
pub(crate) fn redo(app: &mut App) {
    with_history(app.world_mut(), |history, world| {
        history.step(world, Direction::Redo);
    });
    app.update();
}

/// The steps the document can go back.
pub(crate) fn depth(app: &App) -> usize {
    app.world().resource::<EditHistory>().depth()
}

/// Start history at the document as it stands, whatever founded it.
pub(crate) fn baseline(app: &mut App) {
    with_history(app.world_mut(), EditHistory::forget);
}

/// `verb` is exactly one step named `label`, undo returns the lowered document
/// to what it was, and redo returns it to what the verb made.
///
/// Shared with the module of each verb, whose fixture knows how to drive it.
pub(crate) fn assert_one_step(app: &mut App, label: &str, verb: impl FnOnce(&mut App)) {
    assert_one_step_of(app, label, lowered, verb);
}

/// The same, read through `read`, for a fixture whose node a save would not
/// write: a filter under no handler lowers to nothing.
pub(crate) fn assert_one_step_of(
    app: &mut App,
    label: &str,
    read: impl Fn(&mut App) -> String,
    verb: impl FnOnce(&mut App),
) {
    baseline(app);
    let before = read(app);

    verb(app);
    app.update();
    let after = read(app);

    assert_ne!(before, after, "'{label}' changes the document");
    assert_eq!(depth(app), 1, "'{label}' is one step");
    assert_eq!(
        app.world().resource::<EditHistory>().undo_label(),
        Some(label)
    );

    undo(app);
    assert_eq!(read(app), before, "undoing '{label}'");
    redo(app);
    assert_eq!(read(app), after, "redoing '{label}'");
}

/// The stock range, founded the way entering the editor founds it.
pub(crate) fn history_app() -> App {
    let mut app = App::new();
    app.init_resource::<EditContext>();
    app.init_resource::<SelectedNode>();
    app.init_resource::<EditorStatus>();
    app.init_resource::<CameraFraming>();
    app.init_resource::<Time>();
    app.init_resource::<EditHistory>();
    app.init_resource::<crate::ui::canvas::BlockPopup>();
    app.add_message::<ObjectBodyStale>();
    app.add_systems(Update, reflow_auto_ships);
    app.world_mut()
        .run_system_once(ensure_document)
        .expect("the document is founded");
    app.update();
    app
}

/// Let go of a drag on `target`, the way the picking layer reports it.
pub(crate) fn end_drag(app: &mut App, target: Entity) {
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::Image(ImageRenderTarget {
                handle: Handle::default(),
                scale_factor: 1.0,
            }),
            position: Vec2::ZERO,
        },
        DragEnd {
            button: PointerButton::Primary,
            distance: Vec2::ZERO,
        },
        target,
    ));
    app.update();
}

/// Press a row carrying `verb`.
pub(crate) fn press(app: &mut App, row: impl Bundle) {
    let row = app.world_mut().spawn(row).id();
    app.world_mut().trigger(Activate { entity: row });
    app.world_mut().entity_mut(row).despawn();
    app.update();
}

fn root(app: &App) -> Entity {
    app.world()
        .resource::<EditContext>()
        .scenario()
        .expect("the document exists")
}

/// The first node under the root carrying `T`, in id order.
pub(crate) fn first_with<T: Component>(app: &mut App) -> Entity {
    let mut found: Vec<(String, Entity)> = app
        .world_mut()
        .query_filtered::<(&NodeId, Entity), With<T>>()
        .iter(app.world())
        .map(|(id, entity)| (id.0.clone(), entity))
        .collect();
    found.sort();
    found.first().expect("the stock range holds one").1
}

fn select(app: &mut App, node: Entity) {
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));
}

/// New Ship, then Back to the scenario: the ship it added, standing there.
fn add_ship(app: &mut App) -> Entity {
    press(app, observe(create_blank_ship));
    app.world_mut().commands().queue(go_back);
    app.update();
    newest_ship(app)
}

/// The ship New Ship minted last.
fn newest_ship(app: &mut App) -> Entity {
    let mut minted: Vec<(String, Entity)> = app
        .world_mut()
        .query_filtered::<(&NodeId, Entity), With<ShipNode>>()
        .iter(app.world())
        .filter(|(id, _)| id.0.starts_with("ship_"))
        .map(|(id, entity)| (id.0.clone(), entity))
        .collect();
    minted.sort_by_key(|(id, _)| (id.len(), id.clone()));
    minted.last().expect("New Ship minted a ship").1
}

/// Open `ship` on its own, as a double click on its row does.
fn open_ship(app: &mut App, ship: Entity) -> Entity {
    app.world_mut()
        .commands()
        .queue(move |world: &mut World| open_ship_node(world, ship, None));
    app.update();
    app.world()
        .resource::<EditContext>()
        .ship()
        .expect("the ship's document stands it entered")
}

/// Put one part on `ship` as a finished edit, under a minted id.
fn place_part(app: &mut App, ship: Entity) -> NodeId {
    let id = app
        .world_mut()
        .run_system_once(move |mut ordinals: Query<&mut NextChildOrdinal>| {
            crate::node::mint_id(&mut ordinals, ship, "hull")
        })
        .expect("an id is minted");
    app.world_mut().spawn((
        EditorNode,
        id.clone(),
        SectionNode {
            source: SectionSource::prototype("light_hull_section"),
            binds: Vec::new(),
        },
        Transform::default(),
        Visibility::Inherited,
        ChildOf(ship),
    ));
    commit_edit_now(app.world_mut(), "Place Section");
    app.update();
    id
}

#[test]
fn every_document_verb_here_undoes_and_redoes_to_an_equal_lowered_document() {
    let cases: [(&str, fn(&mut App)); 6] = [
        ("Add Ship", |app| {
            add_ship(app);
        }),
        ("Add Asteroid", |app| {
            press(
                app,
                (ObjectChoice::Asteroid, observe(create_scenario_object)),
            );
        }),
        ("Delete Object", |app| {
            let object = first_with::<ObjectNode>(app);
            select(app, object);
            press(app, observe(delete_selected_node));
        }),
        ("Delete Ship", |app| {
            let ship = first_with::<ShipNode>(app);
            select(app, ship);
            press(app, observe(delete_selected_node));
        }),
        ("Delete Section", |app| {
            let section = first_with::<SectionNode>(app);
            select(app, section);
            press(app, observe(delete_selected_node));
        }),
        ("Delete Object", |app| {
            let object = first_with::<ObjectNode>(app);
            let row = app.world_mut().spawn(SceneRow(object)).id();
            let slot = app.world_mut().spawn(ChildOf(row)).id();
            let trash = app
                .world_mut()
                .spawn((observe(on_row_trash), ChildOf(slot)))
                .id();
            app.world_mut().trigger(Activate { entity: trash });
            app.world_mut().entity_mut(row).despawn();
        }),
    ];
    for (label, verb) in cases {
        let mut app = history_app();
        assert_one_step(&mut app, label, verb);
    }
}

#[test]
fn toggling_a_ships_skin_is_one_step() {
    let mut app = history_app();
    let ship = first_with::<ShipNode>(&mut app);
    app.world_mut().resource_mut::<EditContext>().enter(ship);

    assert_one_step(&mut app, "Toggle Skin", |app| {
        press(app, observe(on_skin_toggle));
    });
}

#[test]
fn a_verb_that_changed_nothing_is_no_step() {
    let mut app = history_app();

    commit_edit_now(app.world_mut(), "Nothing");

    assert_eq!(depth(&app), 0);
}

#[test]
fn a_deleted_subtree_comes_back_with_its_children_and_its_fold() {
    let mut app = history_app();
    let ship = first_with::<ShipNode>(&mut app);
    let parts = |app: &mut App| {
        app.world_mut()
            .query_filtered::<(), With<SectionNode>>()
            .iter(app.world())
            .count()
    };
    let standing = parts(&mut app);
    app.world_mut().entity_mut(ship).insert(Expanded);
    baseline(&mut app);
    select(&mut app, ship);
    press(&mut app, observe(delete_selected_node));
    assert!(parts(&mut app) < standing);

    undo(&mut app);

    assert_eq!(parts(&mut app), standing);
    let back = app
        .world_mut()
        .query_filtered::<Entity, (With<ShipNode>, With<Expanded>)>()
        .iter(app.world())
        .count();
    assert_eq!(back, 1, "the ship comes back open");
}

#[test]
fn an_undo_keeps_the_entity_the_selection_and_the_context_of_a_surviving_node() {
    let mut app = history_app();
    let ship = first_with::<ShipNode>(&mut app);
    let section = app
        .world()
        .get::<Children>(ship)
        .and_then(|children| {
            children
                .iter()
                .find(|child| app.world().get::<SectionNode>(*child).is_some())
        })
        .expect("a stock hull has sections");
    let section_id = app.world().get::<NodeId>(section).cloned();
    let ship = open_ship(&mut app, ship);
    let section = app
        .world()
        .get::<Children>(ship)
        .and_then(|children| {
            children
                .iter()
                .find(|child| app.world().get::<NodeId>(*child) == section_id.as_ref())
        })
        .expect("the ship's document holds the same section");
    select(&mut app, section);
    baseline(&mut app);

    app.world_mut()
        .get_mut::<Transform>(section)
        .expect("the section stands")
        .translation
        .y += 1.0;
    commit_edit_now(app.world_mut(), "Move");
    undo(&mut app);

    assert_eq!(
        app.world().resource::<SelectedNode>().primary(),
        Some(section)
    );
    assert_eq!(app.world().resource::<EditContext>().ship(), Some(ship));
    assert!(app.world().get::<SectionNode>(section).is_some());
}

#[test]
fn a_selection_whose_node_is_undone_away_falls_to_its_nearest_surviving_parent() {
    let mut app = history_app();
    let ship = add_ship(&mut app);
    place_part(&mut app, ship);
    let part = app
        .world()
        .get::<Children>(ship)
        .and_then(|children| children.iter().next())
        .expect("the part stands");
    select(&mut app, part);

    undo(&mut app);
    assert_eq!(
        app.world().resource::<SelectedNode>().primary(),
        Some(ship),
        "the part went, so the ship holding it is marked"
    );

    undo(&mut app);
    assert_eq!(
        app.world().resource::<SelectedNode>().primary(),
        Some(root(&app))
    );
    assert_eq!(
        app.world().resource::<EditContext>().path,
        vec![root(&app)],
        "the context cannot stand inside a ship that is gone"
    );
}

#[test]
fn an_id_is_never_minted_twice_across_an_undo() {
    let mut app = history_app();
    let ship = add_ship(&mut app);
    let first = place_part(&mut app, ship);

    undo(&mut app);
    let second = place_part(&mut app, ship);
    assert_ne!(first, second, "the counter stays up when the part goes");

    // Undo the part and the ship: the counter goes with the ship. The redo
    // must bring it back at the height it reached.
    undo(&mut app);
    undo(&mut app);
    redo(&mut app);
    let back = app
        .world()
        .resource::<EditContext>()
        .scenario()
        .and_then(|root| {
            app.world().get::<Children>(root).and_then(|children| {
                children.iter().find(|child| {
                    app.world()
                        .get::<NodeId>(*child)
                        .is_some_and(|id| id.0.starts_with("ship_"))
                })
            })
        })
        .expect("the redo stands the ship up again");
    let third = place_part(&mut app, back);

    assert_ne!(third, first);
    assert_ne!(third, second);
}

#[test]
fn a_reflow_is_no_step() {
    let mut app = history_app();
    let first = add_ship(&mut app);
    let first = app.world().get::<NodeId>(first).cloned();
    let second = add_ship(&mut app);
    assert_ne!(first.as_ref(), app.world().get::<NodeId>(second));
    let steps = depth(&app);

    // The commit recorded the new ship where it spawned. The layout has moved
    // it since, which is the difference this test is about.
    app.update();
    app.update();
    let moved = app
        .world()
        .get::<Transform>(second)
        .expect("the ship stands")
        .translation
        .x;
    assert_ne!(moved, 0.0, "the row put the second ship beside the first");

    commit_edit_now(app.world_mut(), "Nothing");

    assert_eq!(depth(&app), steps);
    assert!(
        app.world().get::<AutoLayout>(second).is_some(),
        "and the ship is still in the row"
    );
}

#[test]
fn undoing_a_move_of_a_row_ship_puts_it_back_in_the_row() {
    let mut app = history_app();
    let ship = add_ship(&mut app);
    app.update();
    app.world_mut()
        .get_mut::<Transform>(ship)
        .expect("the ship stands")
        .translation
        .x = 500.0;
    commit_edit_now(app.world_mut(), "Move");
    app.update();
    assert!(app.world().get::<AutoLayout>(ship).is_none());

    undo(&mut app);

    assert!(app.world().get::<AutoLayout>(ship).is_some());
    let pose = app.world().get::<Transform>(ship).expect("the ship stands");
    assert_ne!(pose.translation.x, 500.0);
}

#[test]
fn a_new_edit_clears_the_redo_stack() {
    let mut app = history_app();
    add_ship(&mut app);
    undo(&mut app);
    assert!(app.world().resource::<EditHistory>().can_redo());

    add_ship(&mut app);

    assert!(!app.world().resource::<EditHistory>().can_redo());
}

/// A ship opens on its own at identity, wherever the scenario turned it, and
/// neither the open nor Back is an edit: no step, no unsaved mark, and the
/// scenario keeps the pose.
#[test]
fn opening_a_turned_ship_stands_it_at_identity_and_is_no_edit() {
    let mut app = history_app();
    app.init_resource::<StageFrame>();
    app.add_systems(Update, sync_stage_frame.after(reflow_auto_ships));
    let ship = add_ship(&mut app);
    let pose = Transform::from_xyz(40.0, -12.0, 7.0).with_rotation(Quat::from_euler(
        EulerRot::YXZ,
        1.1,
        0.4,
        -0.7,
    ));
    *app.world_mut()
        .get_mut::<Transform>(ship)
        .expect("a ship has a pose") = pose;
    commit_edit_now(app.world_mut(), "Move");
    with_history(app.world_mut(), EditHistory::saved_now);
    let steps = depth(&app);
    let id = app.world().get::<NodeId>(ship).cloned().expect("an id");

    let inside = open_ship(&mut app, ship);
    let root = root(&app);
    let shown = app
        .world()
        .get::<Transform>(root)
        .expect("a root pose")
        .compute_affine()
        * app
            .world()
            .get::<Transform>(inside)
            .expect("a ship pose")
            .compute_affine();
    assert!(
        shown.abs_diff_eq(bevy::math::Affine3A::IDENTITY, 1e-4),
        "alone, the ship stands at identity: {shown:?}"
    );
    commit_edit_now(app.world_mut(), "Nothing");
    assert_eq!(depth(&app), steps, "opening is no step");
    assert!(!app.world().resource::<EditHistory>().dirty());

    app.world_mut().commands().queue(go_back);
    app.update();
    let back = newest_ship(&mut app);
    assert_eq!(app.world().get::<NodeId>(back), Some(&id));
    assert!(
        app.world().get::<Transform>(back).is_some_and(|stood| stood
            .translation
            .abs_diff_eq(pose.translation, 1e-3)
            && stood.rotation.abs_diff_eq(pose.rotation, 1e-4)),
        "the scenario keeps the pose"
    );
    commit_edit_now(app.world_mut(), "Nothing");
    assert_eq!(depth(&app), steps, "Back is no step");
    assert!(!app.world().resource::<EditHistory>().dirty());
}

/// An undo in a ship's own document stays there: the step names the ship's
/// document, so the context stays inside the one ship.
#[test]
fn a_step_undone_in_a_ship_document_keeps_it_open() {
    let mut app = history_app();
    let ship = add_ship(&mut app);
    let ship = open_ship(&mut app, ship);

    place_part(&mut app, ship);
    undo(&mut app);

    assert_eq!(app.world().resource::<EditContext>().ship(), Some(ship));
    assert!(app
        .world()
        .resource::<ModProject>()
        .data
        .open
        .as_ref()
        .is_some_and(|open| open.kind == "ship"));
}

#[test]
fn the_dirty_mark_returns_to_clean_at_the_saved_point() {
    let mut app = history_app();
    let dirty = |app: &App| app.world().resource::<EditHistory>().dirty();
    assert!(!dirty(&app), "a new document holds nothing to lose");

    add_ship(&mut app);
    assert!(dirty(&app));

    with_history(app.world_mut(), EditHistory::saved_now);
    assert!(!dirty(&app));

    add_ship(&mut app);
    assert!(dirty(&app));

    undo(&mut app);
    assert!(!dirty(&app), "back at what is on disk");

    undo(&mut app);
    assert!(dirty(&app), "and past it is not what is on disk");

    redo(&mut app);
    assert!(!dirty(&app));
}

#[test]
fn a_saved_state_on_a_discarded_branch_never_reads_clean_again() {
    let mut app = history_app();
    let dirty = |app: &App| app.world().resource::<EditHistory>().dirty();
    add_ship(&mut app);
    with_history(app.world_mut(), EditHistory::saved_now);
    undo(&mut app);

    add_ship(&mut app);

    assert!(dirty(&app));
    undo(&mut app);
    assert!(dirty(&app), "the file holds a ship this state does not");
}

#[test]
fn a_save_during_a_held_drag_does_not_mark_the_last_commit_clean() {
    let mut app = history_app();
    add_ship(&mut app);
    let object = first_with::<ObjectNode>(&mut app);

    app.world_mut()
        .get_mut::<Transform>(object)
        .expect("the object stands")
        .translation
        .y += 1.0;
    with_history(app.world_mut(), EditHistory::saved_now);

    assert!(app.world().resource::<EditHistory>().dirty());
}

#[test]
fn the_oldest_step_falls_off_at_the_depth_limit() {
    let mut app = history_app();
    let object = first_with::<ObjectNode>(&mut app);

    for _ in 0..HISTORY_DEPTH + 3 {
        app.world_mut()
            .get_mut::<Transform>(object)
            .expect("the object stands")
            .translation
            .y += 1.0;
        commit_edit_now(app.world_mut(), "Move");
    }

    assert_eq!(depth(&app), HISTORY_DEPTH);
}

/// Run the key reader once with these keys down.
fn tap(app: &mut App, keys: &[KeyCode]) {
    let mut input = ButtonInput::<KeyCode>::default();
    for key in keys {
        input.press(*key);
    }
    app.world_mut().insert_resource(input);
    app.world_mut()
        .run_system_once(history_keys)
        .expect("the keys are read");
}

#[test]
fn ctrl_z_undoes_and_both_redo_chords_redo() {
    let mut app = history_app();
    add_ship(&mut app);
    let history = |app: &App| {
        let history = app.world().resource::<EditHistory>();
        (history.can_undo(), history.can_redo())
    };

    tap(&mut app, &[KeyCode::ControlLeft, KeyCode::KeyZ]);
    assert_eq!(history(&app), (false, true));

    tap(
        &mut app,
        &[KeyCode::ControlLeft, KeyCode::ShiftLeft, KeyCode::KeyZ],
    );
    assert_eq!(history(&app), (true, false));

    tap(&mut app, &[KeyCode::ControlRight, KeyCode::KeyZ]);
    tap(&mut app, &[KeyCode::ControlRight, KeyCode::KeyY]);
    assert_eq!(history(&app), (true, false));
}

#[test]
fn z_alone_is_not_an_undo() {
    let mut app = history_app();
    add_ship(&mut app);

    tap(&mut app, &[KeyCode::KeyZ]);

    assert_eq!(depth(&app), 1);
}

#[test]
fn an_undo_waits_for_a_held_pointer_to_let_go() {
    let mut app = history_app();
    add_ship(&mut app);
    let mut held = ButtonInput::<MouseButton>::default();
    held.press(MouseButton::Left);
    app.world_mut().insert_resource(held);

    tap(&mut app, &[KeyCode::ControlLeft, KeyCode::KeyZ]);

    assert_eq!(depth(&app), 1, "the drag owns the node until it lets go");
}

/// However far a drag travels, it is one step, and the step lands on release.
#[test]
fn a_stage_drag_is_one_step_on_release() {
    let mut app = history_app();
    app.init_resource::<StageDrag>();
    app.add_observer(on_stage_drag_end);
    let rock = first_with::<ObjectNode>(&mut app);

    assert_one_step(&mut app, "Move", |app| {
        app.world_mut().resource_mut::<StageDrag>().node = Some(rock);
        for step in 1..4 {
            app.world_mut()
                .get_mut::<Transform>(rock)
                .expect("the rock stands")
                .translation
                .y += step as f32;
            app.update();
            assert_eq!(depth(app), 0, "a held drag is no step yet");
        }
        end_drag(app, rock);
    });
}

#[test]
fn picking_a_style_is_one_step() {
    let mut app = history_app();
    let ship = first_with::<ShipNode>(&mut app);
    app.world_mut().resource_mut::<EditContext>().enter(ship);

    assert_one_step(&mut app, "Pick Style", |app| {
        press(
            app,
            (StyleChoice("cladding".into()), observe(on_style_choice)),
        );
    });
}

#[test]
fn the_undo_row_names_the_next_step_and_greys_with_none_left() {
    let mut app = history_app();
    app.add_systems(Update, sync_history_menu);
    let row = app
        .world_mut()
        .spawn((
            MenuHistoryItem::Undo,
            menu_item_row("Undo", MenuLead::None, MenuTail::Key("Ctrl+Z")),
            InteractionDisabled,
        ))
        .id();
    let read = |app: &mut App| {
        let label = app
            .world_mut()
            .query_filtered::<&Text, With<MenuItemLabel>>()
            .single(app.world())
            .expect("the row has a label")
            .0
            .clone();
        (label, app.world().get::<InteractionDisabled>(row).is_some())
    };
    baseline(&mut app);

    add_ship(&mut app);
    app.update();
    assert_eq!(read(&mut app), ("Undo Add Ship".to_string(), false));

    undo(&mut app);
    app.update();
    assert_eq!(read(&mut app), ("Undo".to_string(), true));
}

/// The stage hides and shows nodes as the builder moves, and that is not an edit.
#[test]
fn a_save_clears_the_mark_after_the_stage_rewrites_visibility() {
    let mut app = history_app();
    baseline(&mut app);
    add_ship(&mut app);
    app.update();
    let mut staged = app
        .world_mut()
        .query_filtered::<&mut Visibility, With<ShipNode>>();
    for mut visibility in staged.iter_mut(app.world_mut()) {
        *visibility = Visibility::Hidden;
    }

    saved_now(app.world_mut());

    assert!(!app.world().resource::<EditHistory>().dirty());
}

#[test]
fn the_unsaved_mark_and_the_leave_guard_follow_the_dirty_state() {
    let mut app = history_app();
    app.init_resource::<UnsavedWork>();
    app.add_systems(Update, sync_unsaved_mark);
    let mark = app
        .world_mut()
        .spawn((
            UnsavedMark,
            Node {
                display: Display::None,
                ..default()
            },
        ))
        .id();
    baseline(&mut app);

    add_ship(&mut app);
    app.update();
    assert_eq!(
        app.world().get::<Node>(mark).expect("the mark").display,
        Display::Flex
    );
    assert!(app
        .world_mut()
        .resource_mut::<UnsavedWork>()
        .stops(GuardedLeave::Exit));

    undo(&mut app);
    app.update();
    assert_eq!(
        app.world().get::<Node>(mark).expect("the mark").display,
        Display::None
    );
    assert!(!app
        .world_mut()
        .resource_mut::<UnsavedWork>()
        .stops(GuardedLeave::Exit));
}
