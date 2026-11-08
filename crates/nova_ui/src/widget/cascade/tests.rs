use bevy::ui_widgets::Activate;

use super::*;
use crate::widget::fixtures::themed_app;

/// Ship > (New Ship, Cargoa), Objects > (Asteroid, Planet), a rule, then a
/// greyed leaf and a live one.
fn tree() -> Vec<CascadeItem> {
    vec![
        CascadeItem::submenu(
            "Ship",
            "Ship",
            vec![
                CascadeItem::leaf("New Ship", "New Ship"),
                CascadeItem::separator(),
                CascadeItem::leaf("Cargoa", "Cargoa"),
            ],
        ),
        CascadeItem::submenu(
            "Objects",
            "Objects",
            vec![
                CascadeItem::leaf("Asteroid", "Asteroid"),
                CascadeItem::leaf("Planet", "Planet"),
            ],
        ),
        CascadeItem::separator(),
        CascadeItem::leaf("Blocks", "Blocks").greyed("drag them from the palette"),
        CascadeItem::leaf("Socket", "Socket"),
    ]
}

fn labels(items: &[CascadeItem], entries: &[CascadeEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| {
            cascade_item(items, &entry.path)
                .expect("a path")
                .label
                .clone()
        })
        .collect()
}

#[test]
fn down_and_up_skip_the_rules_and_go_round() {
    let items = tree();
    let mut nav = CascadeNav::default();
    nav.down(&items);
    assert_eq!(nav.levels, vec![Some(0)]);
    nav.down(&items);
    nav.down(&items);
    assert_eq!(nav.levels, vec![Some(3)], "the rule at 2 takes no stop");
    nav.down(&items);
    nav.down(&items);
    assert_eq!(nav.levels, vec![Some(0)], "past the bottom is the top");
    nav.up(&items);
    assert_eq!(nav.levels, vec![Some(4)], "above the top is the bottom");
}

#[test]
fn right_opens_a_submenu_on_its_first_row_and_left_closes_it() {
    let items = tree();
    let mut nav = CascadeNav::default();
    nav.down(&items);
    nav.right(&items);
    assert_eq!(nav.levels, vec![Some(0), Some(0)]);
    assert_eq!(
        labels(&items, &nav.column(&items, 1)),
        ["New Ship", "", "Cargoa"]
    );
    nav.down(&items);
    assert_eq!(
        nav.levels,
        vec![Some(0), Some(2)],
        "the submenu's rule is skipped"
    );
    nav.right(&items);
    assert_eq!(nav.levels.len(), 2, "a leaf opens nothing");
    nav.left();
    assert_eq!(nav.levels, vec![Some(0)]);
    nav.left();
    assert_eq!(nav.levels, vec![Some(0)], "the root column stays");
}

#[test]
fn enter_opens_a_submenu_then_answers_the_live_leaf_only() {
    let items = tree();
    let mut nav = CascadeNav::default();
    assert_eq!(nav.enter(&items), None, "nothing highlighted, nothing runs");
    nav.down(&items);
    nav.down(&items);
    assert_eq!(nav.enter(&items), None);
    assert_eq!(nav.levels, vec![Some(1), Some(0)], "Enter opened Objects");
    nav.down(&items);
    assert_eq!(nav.enter(&items), Some(vec![1, 1]), "Planet runs");

    let mut nav = CascadeNav::default();
    nav.up(&items);
    nav.up(&items);
    assert_eq!(nav.levels, vec![Some(3)]);
    assert_eq!(nav.enter(&items), None, "a greyed row does not run");
}

#[test]
fn the_pointer_opens_a_submenu_with_no_highlight_and_closes_the_one_it_left() {
    let items = tree();
    let mut nav = CascadeNav::default();
    nav.hover(&items, 0, 0);
    assert_eq!(nav.levels, vec![Some(0), None]);
    nav.down(&items);
    assert_eq!(nav.levels, vec![Some(0), Some(0)], "a key then lands in it");
    nav.hover(&items, 0, 4);
    assert_eq!(nav.levels, vec![Some(4)], "a leaf closes the open submenu");
    nav.hover(&items, 0, 2);
    assert_eq!(nav.levels, vec![Some(4)], "a rule takes no hover");
    nav.hover(&items, 3, 0);
    assert_eq!(nav.levels, vec![Some(4)], "a column that is gone is stale");
}

#[test]
fn search_lists_every_matching_leaf_across_the_submenus() {
    let items = tree();
    let found = search_cascade(&items, "an");
    assert_eq!(labels(&items, &found), ["Planet"]);
    let found = search_cascade(&items, "SHIP");
    assert_eq!(
        labels(&items, &found),
        ["New Ship", "Cargoa"],
        "a submenu's label finds every leaf under it, whatever the case"
    );
    assert_eq!(found[1].crumb, "Ship");
    let found = search_cascade(&items, "obj ast");
    assert_eq!(
        labels(&items, &found),
        ["Asteroid"],
        "every word must match"
    );
    assert!(
        search_cascade(&items, "Objects")
            .iter()
            .all(|entry| entry.path.len() == 2),
        "submenus are not results"
    );
    assert!(search_cascade(&items, "zzz").is_empty());
}

#[test]
fn typing_a_search_lists_matches_highlights_the_first_and_opens_nothing() {
    let items = tree();
    let mut nav = CascadeNav::default();
    nav.hover(&items, 0, 1);
    nav.set_query(&items, "o");
    assert_eq!(nav.levels, vec![Some(0)], "the open submenu closed");
    assert_eq!(
        labels(&items, &nav.column(&items, 0)),
        ["Cargoa", "Asteroid", "Planet", "Blocks", "Socket"]
    );
    nav.down(&items);
    nav.right(&items);
    assert_eq!(nav.levels, vec![Some(1)], "Right does not open a result");
    assert_eq!(nav.enter(&items), Some(vec![1, 0]), "Enter runs Asteroid");
    nav.set_query(&items, "");
    assert_eq!(nav.levels, vec![None]);
    assert_eq!(nav.column(&items, 0).len(), items.len(), "the tree is back");
}

/// How many times the Planet leaf ran.
#[derive(Resource, Default)]
struct Ran(u32);

fn count(_activate: On<Activate>, mut ran: ResMut<Ran>) {
    ran.0 += 1;
}

fn press(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release(key);
    keys.clear();
}

/// Down, Right, Down and Enter run the leaf the way its click would, and
/// close the menu.
#[test]
fn the_keys_reach_a_leaf_in_a_submenu_and_run_it_once() {
    let mut app = themed_app(crate::theme::config::PHOSPHOR_THEME_ID);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<Ran>();
    app.update();
    let mut items = tree();
    items[1].children[1] = CascadeItem::leaf("Planet", "Planet").on_row(|row| {
        row.observe(count);
    });
    let world = app.world_mut();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);
    let root = spawn_cascade(&mut commands, Vec2::ZERO, 1, items);
    queue.apply(world);
    app.update();

    for key in [KeyCode::ArrowDown, KeyCode::ArrowDown, KeyCode::ArrowRight] {
        press(&mut app, key);
    }
    let named = |app: &mut App, name: &str| {
        let world = app.world_mut();
        world
            .query::<(&Name, &CascadeRow)>()
            .iter(world)
            .any(|(row, _)| row.as_str() == name)
    };
    assert!(named(&mut app, "Asteroid"), "Objects opened a column");
    press(&mut app, KeyCode::ArrowDown);
    press(&mut app, KeyCode::Enter);

    assert_eq!(app.world().resource::<Ran>().0, 1, "Planet ran once");
    assert!(app.world().get_entity(root).is_err(), "the menu closed");
}
