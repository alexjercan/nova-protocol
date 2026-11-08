use bevy::ui_widgets::{observe, Activate};

use super::*;
use crate::{
    history::tests::assert_one_step_of,
    project::tests::{data, disk, new_mod, project_app},
};

fn entry(kind: &'static str, label: &str) -> AssetEntry {
    AssetEntry {
        kind,
        label: label.to_string(),
        full: label.to_string(),
        name: String::new(),
        note: String::new(),
        waits: "",
        id: None,
        parts: Vec::new(),
        file: None,
        scenario: None,
    }
}

/// The filter box and the kind column narrow the same grid, and both apply.
#[test]
fn a_kind_and_a_filter_both_narrow_the_grid() {
    let entries = vec![
        entry("scenario", "Red Run"),
        entry("section", "red_plate"),
        entry("audio", "sounds/boom.ogg"),
    ];

    assert_eq!(
        filtered_entries(&entries, &AssetShelfType::All, "").len(),
        3
    );
    assert_eq!(
        filtered_entries(&entries, &AssetShelfType::All, "RED").len(),
        2
    );
    let kept = filtered_entries(&entries, &AssetShelfType::Kind("section"), "red");
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].label, "red_plate");
}

/// A ship a scenario made is on that scenario's shelf and not on the Ships
/// shelf, which holds the designs no scenario owns.
#[test]
fn a_scenario_ship_shelves_under_its_scenario() {
    let mut owned = entry("ship", "hulk_0");
    owned.full = "range.hulk_0".to_string();
    owned.scenario = Some("range".to_string());
    let mut range = entry("scenario", "Range");
    range.full = "range".to_string();
    let entries = vec![range, owned, entry("ship", "red_run__courier")];

    let scenario = filtered_entries(&entries, &AssetShelfType::Scenario("range".into()), "");
    let labels: Vec<&str> = scenario.iter().map(|entry| entry.label.as_str()).collect();
    assert_eq!(labels, ["Range", "hulk_0"]);
    let ships = filtered_entries(&entries, &AssetShelfType::Kind("ship"), "");
    let labels: Vec<&str> = ships.iter().map(|entry| entry.label.as_str()).collect();
    assert_eq!(labels, ["red_run__courier"]);
}

/// A tile is a name line over a muted kind line. A long name ends in an
/// ellipsis on its one line; the hover hint keeps it whole.
#[test]
fn a_tile_prints_its_name_over_its_kind() {
    let mut entries = vec![
        entry("section", "picket_sentinel"),
        entry("ui theme", "The Long Way Home Again"),
    ];

    name_tiles(&mut entries, "red-run");

    assert_eq!(
        (entries[0].name.as_str(), entries[0].note.as_str()),
        ("picket_sentinel", "SECTION")
    );
    assert_eq!(
        (entries[1].name.as_str(), entries[1].note.as_str()),
        ("The Long Way ...", "UI THEME")
    );
}

/// A design drops its scenario on the tile and keeps it in the hover hint.
#[test]
fn a_design_tile_prints_its_node_and_keeps_the_whole_id() {
    let mut design = entry("ship", "hulk_0");
    design.full = "saved_range.hulk_0".to_string();
    let mut entries = vec![design];

    name_tiles(&mut entries, "red-run");

    assert_eq!(entries[0].name, "hulk_0");
    assert_eq!(entries[0].note, "SHIP");
    assert_eq!(entries[0].full, "saved_range.hulk_0");
}

/// Two scenarios can hold the same node. Neither tile may read as the other,
/// even when the scenario ids are too long to print whole.
#[test]
fn tiles_with_one_short_name_print_what_tells_them_apart() {
    let tile = |full: &str| {
        let mut design = entry("ship", "hulk_0");
        design.full = full.to_string();
        design
    };
    let mut entries = vec![
        tile("red_run__arena.hulk_0"),
        tile("red_run__arena_copy.hulk_0"),
        tile("red_run__alpha_long_scenario_id.hulk_0"),
        tile("red_run__bravo_long_scenario_id.hulk_0"),
    ];

    name_tiles(&mut entries, "red-run");

    let printed: Vec<(&str, &str)> = entries
        .iter()
        .map(|entry| (entry.name.as_str(), entry.note.as_str()))
        .collect();
    assert_eq!(
        printed,
        [
            ("hulk_0", "arena"),
            ("hulk_0", "arena_copy"),
            ("hulk_0", "...g_scenario_id"),
            ("hulk_0", "...cenario_id #2"),
        ]
    );
}

/// Names that part only past the cut read alike, so they part on the id's end.
#[test]
fn names_that_read_alike_once_cut_print_the_end_of_their_id() {
    let mut entries = vec![
        entry("section", "picket_sentinel_alpha_one"),
        entry("section", "picket_sentinel_alpha_two"),
    ];

    name_tiles(&mut entries, "red-run");

    assert_eq!(entries[0].name, "picket_sentin...");
    assert_eq!(entries[0].note, "...nel_alpha_one");
    assert_eq!(entries[1].note, "...nel_alpha_two");
}

/// A short name is what a tile prints, and the filter still finds the rest.
#[test]
fn the_filter_finds_a_tile_by_the_id_it_does_not_print() {
    let mut design = entry("ship", "hulk_0");
    design.full = "saved_range.hulk_0".to_string();

    assert_eq!(
        filtered_entries(&[design], &AssetShelfType::All, "saved_range").len(),
        1
    );
}

/// Press a menu row for `asset`.
fn press_verb(app: &mut App, verb: AssetVerbType, asset: OpenAsset) {
    let entity = app
        .world_mut()
        .spawn((AssetVerb(verb, asset), observe(on_asset_verb)))
        .id();
    app.world_mut().trigger(Activate { entity });
}

/// A tile's menu acts through the project's own verbs: one step, undone whole.
#[test]
fn a_tile_right_click_verb_is_one_undo_step() {
    let _disk = disk();
    let mut app = project_app();
    app.init_resource::<AssetRenaming>();
    new_mod(&mut app, "red");
    let id = data(&app)
        .open_scenario()
        .expect("a new mod opens its scenario")
        .to_string();
    let asset = OpenAsset {
        kind: SCENARIO_KIND,
        id,
    };

    let read = |app: &mut App| data(app).items().count().to_string();
    assert_one_step_of(&mut app, "Duplicate Scenario", read, |app| {
        press_verb(app, AssetVerbType::Duplicate, asset);
    });
}

/// Rename starts on the tile, and writes nothing until the name is given.
#[test]
fn rename_opens_on_the_tile_and_writes_nothing_yet() {
    let _disk = disk();
    let mut app = project_app();
    app.init_resource::<AssetRenaming>();
    new_mod(&mut app, "red");
    let id = data(&app)
        .open_scenario()
        .expect("a new mod opens its scenario")
        .to_string();
    let asset = OpenAsset {
        kind: SCENARIO_KIND,
        id,
    };

    press_verb(&mut app, AssetVerbType::Rename, asset.clone());
    app.update();

    assert_eq!(
        app.world().resource::<AssetRenaming>().0,
        Some(RenameTarget::Item(asset))
    );
    assert_eq!(crate::history::tests::depth(&app), 0);
}
