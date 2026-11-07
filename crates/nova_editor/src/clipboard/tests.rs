use super::*;
use crate::{
    event::{place_new, ActionNode, BlockItemType, BlockSlotType, EventNode, FilterChoice},
    history::tests::{baseline, depth, first_with, history_app, lowered, undo},
    node::{EditorNode, ScenarioNode, SectionNode, ShipDriver},
};

fn clipboard_app() -> App {
    let mut app = history_app();
    app.init_resource::<EditorClipboard>();
    app
}

fn select(app: &mut App, node: Entity) {
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));
}

fn ids(app: &mut App) -> Vec<String> {
    let mut ids: Vec<String> = app
        .world_mut()
        .query_filtered::<&NodeId, With<EditorNode>>()
        .iter(app.world())
        .map(|id| id.0.clone())
        .collect();
    ids.sort();
    ids
}

fn run(app: &mut App, verb: fn(&mut World)) {
    verb(app.world_mut());
    app.update();
}

#[test]
fn duplicate_gets_a_fresh_id_and_is_one_step() {
    let mut app = clipboard_app();
    let rock = first_with::<ObjectNode>(&mut app);
    let source = app.world().get::<NodeId>(rock).expect("an id").clone();
    select(&mut app, rock);
    let before = ids(&mut app);

    run(&mut app, duplicate_selection);

    let copy = app
        .world()
        .resource::<SelectedNode>()
        .primary()
        .expect("the copy is selected");
    assert_ne!(copy, rock);
    let fresh = app.world().get::<NodeId>(copy).expect("an id").clone();
    assert_ne!(fresh, source);
    assert!(!before.contains(&fresh.0), "{} was taken", fresh.0);
    assert_eq!(depth(&app), 1);
    let kind = |node| format!("{:?}", app.world().get::<ObjectNode>(node).map(|o| &o.kind));
    assert_eq!(kind(copy), kind(rock));
}

#[test]
fn a_duplicate_stands_clear_of_what_it_copied() {
    let mut app = clipboard_app();
    let rock = first_with::<ObjectNode>(&mut app);
    select(&mut app, rock);

    run(&mut app, duplicate_selection);

    let copy = app
        .world()
        .resource::<SelectedNode>()
        .primary()
        .expect("the copy is selected");
    let at = |node| {
        app.world()
            .get::<Transform>(node)
            .expect("a pose")
            .translation
    };
    assert!(at(copy).x > at(rock).x);
}

#[test]
fn ctrl_z_after_paste_removes_it() {
    let mut app = clipboard_app();
    let rock = first_with::<ObjectNode>(&mut app);
    select(&mut app, rock);
    let before = ids(&mut app);

    run(&mut app, copy_selection);
    assert_eq!(depth(&app), 0, "a copy is no edit");
    run(&mut app, paste_clipboard);
    assert_eq!(ids(&mut app).len(), before.len() + 1);
    assert_eq!(depth(&app), 1);

    undo(&mut app);
    app.update();

    assert_eq!(ids(&mut app), before);
}

#[test]
fn a_second_paste_lands_clear_of_the_first() {
    let mut app = clipboard_app();
    let rock = first_with::<ObjectNode>(&mut app);
    select(&mut app, rock);
    run(&mut app, copy_selection);

    run(&mut app, paste_clipboard);
    let first = app.world().resource::<SelectedNode>().primary();
    run(&mut app, paste_clipboard);
    let second = app.world().resource::<SelectedNode>().primary();

    let at = |node: Option<Entity>| {
        app.world()
            .get::<Transform>(node.expect("a paste"))
            .expect("a pose")
            .translation
    };
    assert_ne!(at(first), at(second));
    assert_eq!(depth(&app), 2);
}

#[test]
fn a_group_paste_is_one_step() {
    let mut app = clipboard_app();
    let rock = first_with::<ObjectNode>(&mut app);
    let ship = first_with::<ShipNode>(&mut app);
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set_all([rock, ship]);
    let before = ids(&mut app).len();

    run(&mut app, duplicate_selection);

    assert_eq!(depth(&app), 1);
    assert_eq!(app.world().resource::<SelectedNode>().len(), 2);
    assert!(ids(&mut app).len() >= before + 2);
}

#[test]
fn a_copied_ship_keeps_its_sections_and_gives_up_the_player() {
    let mut app = clipboard_app();
    let ship = first_with::<ShipNode>(&mut app);
    app.world_mut()
        .get_mut::<ShipNode>(ship)
        .expect("a ship")
        .driver = ShipDriver::Player;
    select(&mut app, ship);

    run(&mut app, duplicate_selection);

    let copy = app
        .world()
        .resource::<SelectedNode>()
        .primary()
        .expect("the copy is selected");
    assert_eq!(
        app.world().get::<ShipNode>(copy).map(|ship| ship.driver),
        Some(ShipDriver::Ai)
    );
    let parts = |app: &App, node: Entity| {
        app.world()
            .get::<Children>(node)
            .into_iter()
            .flatten()
            .filter(|child| app.world().get::<SectionNode>(**child).is_some())
            .count()
    };
    assert_eq!(parts(&app, copy), parts(&app, ship));
}

#[test]
fn duplicate_is_refused_inside_a_ship_and_says_so() {
    let mut app = clipboard_app();
    let ship = first_with::<ShipNode>(&mut app);
    let scenario = first_with::<ScenarioNode>(&mut app);
    app.world_mut().resource_mut::<EditContext>().path = vec![scenario, ship];
    select(&mut app, ship);
    let before = ids(&mut app);

    run(&mut app, duplicate_selection);

    assert_eq!(ids(&mut app), before);
    assert_eq!(depth(&app), 0);
    let status = app.world().resource::<crate::config::EditorStatus>();
    assert_eq!(status.line().map(|(line, _)| line), Some(PART_REFUSAL));
}

#[test]
fn an_id_stem_loses_its_ordinal_and_nothing_else() {
    assert_eq!(id_stem(&NodeId("rock_3".to_string())), "rock");
    assert_eq!(
        id_stem(&NodeId("picket_warden".to_string())),
        "picket_warden"
    );
    assert_eq!(id_stem(&NodeId("hulk".to_string())), "hulk");
}

/// A set the marked block cannot take whole is refused whole: a paste that
/// lands half of it leaves a step nobody asked for.
#[test]
fn a_refused_block_paste_lands_nothing() {
    let mut app = clipboard_app();
    let hat = first_with::<EventNode>(&mut app);
    let action = first_with::<ActionNode>(&mut app);
    let filter = place_new(
        app.world_mut(),
        BlockItemType::Filter(FilterChoice::Timer),
        BlockSlotType::Filter {
            parent: hat,
            index: 0,
        },
    )
    .expect("a filter joins a handler");
    app.update();
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set_all(vec![action, filter]);
    run(&mut app, copy_selection);
    select(&mut app, action);
    baseline(&mut app);
    let before = lowered(&mut app);

    run(&mut app, paste_clipboard);

    assert_eq!(lowered(&mut app), before);
    assert_eq!(depth(&app), 0);
}
