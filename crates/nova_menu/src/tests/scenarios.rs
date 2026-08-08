//! The scenario and campaign pickers: which entries are listed and in what
//! order, selection and details, and that the list scrolls on the wheel and
//! clamps at both ends.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ui::widget::Selected;

use super::support::{
    app, entity_by_name, label_of, observe_load_scenario, LoadedScenario, TEST_BACKDROP_ID,
    TEST_START_ID,
};
use crate::{
    scenarios::{
        CampaignHeader, NewGameScenario, ScenarioRow, ScenariosList, SelectedScenarioId,
        CAMPAIGN_MEMBER_INDENT_PX,
    },
    widgets::{scroll_menu_lists, ScrollableList},
};

/// DoD 1: a `ScrollableList` (mods AND scenarios now share the marker) moves its
/// `ScrollPosition` on the wheel and CLAMPS the stored offset at both ends against
/// content height - not just the top (lesson
/// bevy-ui-scroll-input-clamps-stored-offset). Before the generalized wiring,
/// `ScenariosList` had no scroll driver, so this fails.
#[test]
fn scenarios_list_scrolls_on_wheel_and_clamps() {
    use bevy::{
        ecs::system::RunSystemOnce,
        input::{
            mouse::{MouseScrollUnit, MouseWheel},
            touch::TouchPhase,
        },
    };

    // content 180 in a 100-tall box -> max scroll offset 80.
    let scroll_after = |start_y: f32, wheel_y: f32| -> f32 {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut().init_resource::<Messages<MouseWheel>>();
        app.world_mut().spawn((
            ScenariosList,
            ScrollableList,
            ScrollPosition(Vec2::new(0.0, start_y)),
            ComputedNode {
                size: Vec2::new(200.0, 100.0),
                content_size: Vec2::new(200.0, 180.0),
                ..default()
            },
        ));
        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: wheel_y,
            window: Entity::PLACEHOLDER,
            phase: TouchPhase::Moved,
        });
        app.world_mut()
            .run_system_once(scroll_menu_lists)
            .expect("scroll system runs");
        app.world_mut()
            .query::<&ScrollPosition>()
            .single(app.world())
            .expect("one scroll position")
            .0
            .y
    };

    // A downward wheel (negative dy) scrolls down; a big one clamps at max.
    assert!(
        scroll_after(0.0, -1.0) > 0.0,
        "wheel moves the scenarios list"
    );
    assert_eq!(
        scroll_after(0.0, -100.0),
        80.0,
        "clamps at the bottom (content - box)"
    );
    // An upward wheel from a mid offset clamps at the top.
    assert_eq!(scroll_after(40.0, 100.0), 0.0, "clamps at the top");
}

// --- Scenarios picker ------------------------------------------------------

fn picker_scenario(id: &str, name: &str, hidden: bool) -> (String, ScenarioConfig) {
    (
        id.to_string(),
        ScenarioConfig {
            id: id.to_string(),
            name: name.to_string(),
            description: format!("{name} blurb"),
            hidden,
            ..Default::default()
        },
    )
}

/// A registry with a listed story entry, a listed mod scenario, and the
/// hidden menu backdrop (so `load_menu_ambience` on menu entry still finds
/// its scenario). The picker must show the two listed ones and drop the
/// hidden backdrop.
fn picker_scenarios() -> GameScenarios {
    GameScenarios(bevy::platform::collections::HashMap::from([
        picker_scenario(TEST_START_ID, "Shakedown Run", false),
        picker_scenario("practice_run", "Practice Run", false),
        picker_scenario(TEST_BACKDROP_ID, "Menu Ambience", true),
    ]))
}

/// Enter the menu with the picker registry; one update runs OnEnter
/// (setup_menu_ui) and the refresh chain, populating the scenario list and
/// default-selecting the first row.
fn scenarios_app() -> App {
    let mut app = app();
    app.insert_resource(picker_scenarios());
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();
    app
}

fn scenario_row(app: &mut App, id: &str) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &ScenarioRow)>();
    q.iter(app.world())
        .find(|(_, r)| r.id == id)
        .map(|(e, _)| e)
}

fn scenario_row_ids(app: &mut App) -> Vec<String> {
    let mut q = app.world_mut().query::<&ScenarioRow>();
    let mut ids: Vec<String> = q.iter(app.world()).map(|r| r.id.clone()).collect();
    ids.sort();
    ids
}

/// The rendered row title texts in DISPLAY order: the `ScenariosList`
/// children, in child order, filtered to rows, each read via its "Scenario
/// Name" Text child. This reads what the picker actually spawned (order +
/// label), not the pure sort - the render-output-eyeball guard at the ECS
/// level.
fn scenario_row_labels_in_order(app: &mut App) -> Vec<String> {
    let list = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<ScenariosList>>();
        q.iter(app.world()).next().expect("scenarios list exists")
    };
    let rows: Vec<Entity> = {
        let children = app
            .world()
            .get::<Children>(list)
            .expect("list has children");
        children
            .iter()
            .filter(|&c| app.world().get::<ScenarioRow>(c).is_some())
            .collect()
    };
    rows.into_iter().map(|row| label_of(app, row)).collect()
}

fn selected_scenario(app: &App) -> Option<String> {
    app.world().resource::<SelectedScenarioId>().0.clone()
}

/// The details pane's name text, read from the stable-named entity (not just
/// `all_texts`, which would also match the row that shares the name).
fn scenario_details_name(app: &mut App) -> Option<String> {
    let ent = entity_by_name(app, "Scenario Details Name")?;
    app.world().get::<Text>(ent).map(|t| t.0.clone())
}

/// The picker lists exactly the `!hidden` scenarios: the story entry and the
/// mod scenario show, the hidden backdrop does not. Fails if the filter is
/// dropped (menu_ambience would appear).
#[test]
fn scenarios_panel_lists_only_unhidden_scenarios() {
    let mut app = scenarios_app();
    let ids = scenario_row_ids(&mut app);
    assert!(
        ids.contains(&TEST_START_ID.to_string()),
        "the story entry is listed: {ids:?}"
    );
    assert!(
        ids.contains(&"practice_run".to_string()),
        "the mod scenario is listed: {ids:?}"
    );
    assert!(
        !ids.contains(&TEST_BACKDROP_ID.to_string()),
        "the hidden backdrop scenario is NOT listed: {ids:?}"
    );
}

// --- Collapsible campaign headers ------------------------------------------

/// A registry with a two-chapter "Nova Protocol" campaign (chapter two is
/// `hidden`, reachable ONLY through the campaign header) plus one
/// uncampaigned standalone. The picker must render the campaign as a
/// collapsible header over its ordered members, hidden one included, with the
/// standalone flat below.
fn campaigns_app() -> App {
    let mut app = app();
    app.insert_resource(GameScenarios(bevy::platform::collections::HashMap::from([
        picker_scenario("chap1", "Chapter One", false),
        picker_scenario("chap2", "Chapter Two", true),
        picker_scenario("standalone", "Standalone", false),
    ])));
    app.insert_resource(GameCampaigns(bevy::platform::collections::HashMap::from([
        (
            "nova_protocol".to_string(),
            CampaignConfig {
                id: "nova_protocol".to_string(),
                name: "Nova Protocol".to_string(),
                scenarios: vec!["chap1".to_string(), "chap2".to_string()],
            },
        ),
    ])));
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();
    app
}

/// The `ScenariosList` children in display order, each tagged `header:<label>`
/// or `row:<label>` - the render-output-eyeball at the ECS level, reading what
/// the picker actually spawned (headers AND rows, in child order).
fn list_display_in_order(app: &mut App) -> Vec<String> {
    let list = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<ScenariosList>>();
        q.iter(app.world()).next().expect("scenarios list exists")
    };
    let children: Vec<Entity> = app
        .world()
        .get::<Children>(list)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    children
        .into_iter()
        .filter_map(|c| {
            if app.world().get::<CampaignHeader>(c).is_some() {
                Some(format!("header:{}", label_of(app, c)))
            } else if app.world().get::<ScenarioRow>(c).is_some() {
                Some(format!("row:{}", label_of(app, c)))
            } else {
                None
            }
        })
        .collect()
}

fn campaign_header(app: &mut App, id: &str) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &CampaignHeader)>();
    q.iter(app.world())
        .find(|(_, h)| h.id == id)
        .map(|(e, _)| e)
}

/// The picker renders a campaign as an expanded header over its members in
/// declared order (hidden chapter two included), then the uncampaigned
/// standalone flat below - the collapsible-grouping contract.
#[test]
fn picker_renders_collapsible_campaign_header_over_ordered_members() {
    let mut app = campaigns_app();
    assert_eq!(
        list_display_in_order(&mut app),
        vec![
            "header:[-] Nova Protocol".to_string(),
            "row:Chapter One".to_string(),
            "row:Chapter Two".to_string(),
            "row:Standalone".to_string(),
        ],
        "expanded header, then members in campaign order (hidden chapter two \
         listed for replay), then the uncampaigned standalone"
    );
}

/// Clicking a campaign header collapses it (members vanish, marker flips to
/// `[+]`); clicking again expands it (members return, marker `[-]`). Drives
/// the real toggle observer + refresh through the spawn path.
#[test]
fn toggling_a_campaign_header_collapses_and_expands_its_members() {
    let mut app = campaigns_app();
    let header = campaign_header(&mut app, "nova_protocol").expect("campaign header");

    app.world_mut().trigger(Activate { entity: header });
    app.update();
    assert_eq!(
        list_display_in_order(&mut app),
        vec![
            "header:[+] Nova Protocol".to_string(),
            "row:Standalone".to_string(),
        ],
        "collapsed: members hidden, marker [+], standalone still shown"
    );

    let header = campaign_header(&mut app, "nova_protocol").expect("header persists");
    app.world_mut().trigger(Activate { entity: header });
    app.update();
    assert_eq!(
        list_display_in_order(&mut app),
        vec![
            "header:[-] Nova Protocol".to_string(),
            "row:Chapter One".to_string(),
            "row:Chapter Two".to_string(),
            "row:Standalone".to_string(),
        ],
        "re-expanded: members return in order, marker [-]"
    );
}

/// DoD 2: a campaign MEMBER row is indented under its header and an uncampaigned row is
/// not, so the grouping reads at a glance (owner feedback 2026-07-29). Reads the
/// spawned `Node`, not a colour: the indent is the row's left margin.
#[test]
fn campaign_member_rows_are_indented_under_their_header() {
    let mut app = campaigns_app();

    let row_node = |app: &mut App, id: &str| -> Node {
        let row = scenario_row(app, id).expect("row exists");
        app.world().get::<Node>(row).expect("row Node").clone()
    };

    for member in ["chap1", "chap2"] {
        let node = row_node(&mut app, member);
        assert_eq!(
            node.margin.left,
            px(CAMPAIGN_MEMBER_INDENT_PX),
            "{member} is inset under its header (hidden chapters included)"
        );
        // INSET, not shifted: a `list_row`'s `percent(100)` plus an outside
        // margin would make the row wider than the pane and overhang the
        // details divider (review R1.1 - it did, visibly).
        assert_eq!(
            node.width,
            Val::Auto,
            "{member} sizes to the pane MINUS its indent, not 100% + indent"
        );
    }

    let standalone = row_node(&mut app, "standalone");
    assert_eq!(
        standalone.margin.left,
        px(0.0),
        "an uncampaigned scenario stays flush with the headers"
    );
    assert_eq!(
        standalone.width,
        percent(100),
        "an un-indented row keeps the shared list_row width"
    );
}

/// DoD 1: the two-pane screens' LIST pane cannot give up width to its sibling, so the
/// split is a property of the screen and not of the selection.
///
/// The symptom this pins was measured in the real app, where text actually measures:
/// `cargo run --example menu_scenarios --features debug` walked the shipped scenarios
/// and logged a list pane swinging 141..331 px (a 190 px spread) purely from which row
/// was selected. A headless rig measures every text node as zero-width and cannot
/// reproduce that, so THIS test pins the layout property that fixed it and the example
/// rig stays the evidence.
#[test]
fn two_pane_list_panes_cannot_shrink() {
    let mut app = campaigns_app();

    let node_named = |app: &mut App, name: &str| -> Node {
        let mut q = app.world_mut().query::<(&Name, &Node)>();
        q.iter(app.world())
            .find(|(n, _)| n.as_str() == name)
            .map(|(_, node)| node.clone())
            .unwrap_or_else(|| panic!("{name} exists"))
    };

    // The pin has TWO sides, and either one alone still overflows the panel:
    // the list pane must not shrink, AND the details pane must absorb the
    // slack it refuses (grow) and be allowed to fall below its content width
    // (`min_width: 0`, so long text wraps instead of pushing the row wider).
    for (list, details) in [
        ("Scenarios List", "Scenario Details Panel"),
        ("Mods Left Pane", "Mod Details Panel"),
    ] {
        let node = node_named(&mut app, list);
        assert_eq!(
            node.flex_shrink, 0.0,
            "{list} must not shrink for its sibling details pane"
        );
        assert_eq!(
            node.flex_grow, 0.0,
            "{list} must not grow into slack either"
        );
        assert_eq!(node.width, percent(40), "{list} keeps its fixed share");

        let node = node_named(&mut app, details);
        assert_eq!(node.flex_grow, 1.0, "{details} absorbs the slack");
        assert_eq!(
            node.min_width,
            px(0),
            "{details} may shrink below its content and wrap"
        );
    }
}

/// A HIDDEN campaign member is directly selectable and launchable for replay:
/// selecting chapter two (hidden) feeds the details pane and its Play button
/// loads chapter two itself - not the earlier chapter, not the canned start.
#[test]
fn a_hidden_campaign_member_is_selectable_and_launchable() {
    let mut app = campaigns_app();
    observe_load_scenario(&mut app);

    let hidden_row = scenario_row(&mut app, "chap2").expect("hidden member row exists");
    app.world_mut().trigger(Activate { entity: hidden_row });
    app.update();

    assert_eq!(
        selected_scenario(&app).as_deref(),
        Some("chap2"),
        "the hidden member is selected"
    );
    assert_eq!(
        scenario_details_name(&mut app).as_deref(),
        Some("Chapter Two"),
        "the details pane renders the hidden member"
    );

    let play = entity_by_name(&mut app, "Scenario Play Button").expect("play button");
    app.world_mut().trigger(Activate { entity: play });
    app.update();
    assert_eq!(
        app.world().resource::<LoadedScenario>().0.as_deref(),
        Some("chap2"),
        "playing the hidden member loads it directly - a mid-campaign replay"
    );
}

/// The list default-selects the first row (sorted by name), and the details
/// pane renders that scenario's name.
#[test]
fn scenarios_panel_default_selects_first_and_renders_details() {
    let mut app = scenarios_app();
    // Sorted by name: "Practice Run" < "Shakedown Run".
    assert_eq!(selected_scenario(&app).as_deref(), Some("practice_run"));
    assert_eq!(
        scenario_details_name(&mut app).as_deref(),
        Some("Practice Run"),
        "the details pane renders the default selection"
    );
}

/// Clicking a row selects that scenario: `SelectedScenarioId` moves, the
/// highlight moves, and the details pane rebuilds with its name.
#[test]
fn clicking_a_scenario_row_selects_it_and_renders_its_details() {
    let mut app = scenarios_app();
    let story_row = scenario_row(&mut app, TEST_START_ID).expect("story row");

    app.world_mut().trigger(Activate { entity: story_row });
    app.update();

    assert_eq!(selected_scenario(&app).as_deref(), Some(TEST_START_ID));
    let story_row = scenario_row(&mut app, TEST_START_ID).unwrap();
    let practice_row = scenario_row(&mut app, "practice_run").unwrap();
    assert!(
        app.world().entity(story_row).contains::<Selected>(),
        "the clicked row is highlighted"
    );
    assert!(
        !app.world().entity(practice_row).contains::<Selected>(),
        "the previous selection is cleared"
    );
    assert_eq!(
        scenario_details_name(&mut app).as_deref(),
        Some("Shakedown Run"),
        "the details pane renders the clicked scenario"
    );
}

/// The details pane's Play button hands off exactly like New Game AND
/// (delivery guard) loads the CHOSEN scenario, not the canned start: playing
/// practice_run must fire `LoadScenario` for practice_run, not shakedown_run.
#[test]
fn play_button_hands_off_and_loads_the_chosen_scenario() {
    let mut app = scenarios_app();
    observe_load_scenario(&mut app);

    // practice_run is the default selection; its Play button carries its id.
    let play = entity_by_name(&mut app, "Scenario Play Button").expect("play button");
    app.world_mut().trigger(Activate { entity: play });
    app.update();

    assert_eq!(
        app.world().resource::<NewGameScenario>().0.as_deref(),
        Some("practice_run"),
        "Play records the scenario override"
    );
    assert_eq!(*app.world().resource::<GameMode>(), GameMode::NewGame);
    assert_eq!(
        *app.world().resource::<State<GameStates>>().get(),
        GameStates::Playing
    );
    assert_eq!(
        app.world().resource::<LoadedScenario>().0.as_deref(),
        Some("practice_run"),
        "the chosen scenario is loaded, not the canned New Game start"
    );
}

/// The flat baseline (the interim campaign grouping is superseded by the collapsible-
/// header UI): the picker lists every `!hidden` scenario sorted by display name, and a
/// hidden backdrop does not render. Reads the ACTUAL spawned row Text in child order
/// through the real spawn path, so it would catch a spawn path that ignored the sort or
/// the hidden filter.
#[test]
fn picker_rows_render_flat_name_sorted() {
    let mut app = app();
    app.insert_resource(GameScenarios(bevy::platform::collections::HashMap::from([
        picker_scenario("shakedown", "Shakedown Run", false),
        picker_scenario("broadside", "Broadside", false),
        picker_scenario("asteroid_field", "Asteroid Field", false),
        picker_scenario(TEST_BACKDROP_ID, "Menu Ambience", true),
    ])));
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();

    assert_eq!(
        scenario_row_labels_in_order(&mut app),
        vec![
            "Asteroid Field".to_string(),
            "Broadside".to_string(),
            "Shakedown Run".to_string(),
        ],
        "rows render sorted by display name; the hidden backdrop does not render"
    );
}
