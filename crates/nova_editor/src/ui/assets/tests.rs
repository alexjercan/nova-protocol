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

/// Install the shipped base content as mod `base`, so a new item lints
/// against the real catalog: a new UI theme derives from `base/phosphor`.
fn load_base_content(app: &mut App) {
    use nova_modding::prelude::{
        parse_content, BundleAsset, CatalogEntry, ContentAsset, InstalledCatalog, ModEntry, ModMeta,
    };

    fn walk(dir: &std::path::Path, found: &mut Vec<Vec<Content>>) {
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .expect("the base content is here")
            .map(|entry| entry.expect("an entry").path())
            .collect();
        paths.sort();
        for path in paths {
            if path.is_dir() {
                walk(&path, found);
            } else if path.to_string_lossy().ends_with(".content.ron") {
                let bytes = std::fs::read(&path).expect("a content file reads");
                found.push(parse_content(&bytes).expect("a shipped file parses"));
            }
        }
    }
    let mut files = Vec::new();
    walk(std::path::Path::new("../../assets/base"), &mut files);
    let world = app.world_mut();
    let mut contents = Assets::<ContentAsset>::default();
    let content = files
        .into_iter()
        .map(|items| contents.add(ContentAsset(items)))
        .collect();
    let mut bundles = Assets::<BundleAsset>::default();
    let bundle = bundles.add(BundleAsset {
        content,
        meta: ModMeta::default(),
        new_game_scenario: None,
        resources: vec![],
        resource_base: "mods/base".to_string(),
    });
    let mut catalogs = Assets::<InstalledCatalog>::default();
    catalogs.add(InstalledCatalog {
        entries: vec![CatalogEntry {
            decl: ModEntry {
                id: "base".to_string(),
                bundle: "mods/base/base.bundle.ron".to_string(),
                base: true,
            },
            bundle: Some(bundle),
        }],
    });
    world.insert_resource(contents);
    world.insert_resource(bundles);
    world.insert_resource(catalogs);
    world.insert_resource(nova_assets::prelude::EnabledMods(
        ["base".to_string()].into_iter().collect(),
    ));
}

/// The leaves of `items`, depth first, separators left out.
fn leaves(items: &[CascadeItem]) -> Vec<&CascadeItem> {
    items
        .iter()
        .filter(|item| !item.separator)
        .flat_map(|item| {
            if item.is_submenu() {
                leaves(&item.children)
            } else {
                vec![item]
            }
        })
        .collect()
}

/// Run `item`'s row as a menu press would.
fn choose(app: &mut App, item: &CascadeItem) {
    let entity = app.world_mut().spawn_empty().id();
    if let Some(row) = &item.row {
        let mut commands = app.world_mut().commands();
        row(&mut commands.entity(entity));
    }
    app.world_mut().flush();
    app.world_mut().trigger(Activate { entity });
    app.update();
}

/// The New Asset tree in every view, through the Add menu, `+` and the right
/// click on the grid, offers only rows that make something: choosing each
/// one adds an item of its kind, opens it, and a save writes it. A section
/// asks its kind, and every kind it offers is made the same way. Nothing is
/// greyed, so no row refuses: a lesson and a station are not offered, and a
/// second price table is hidden once the mod states one.
#[test]
fn every_new_asset_row_makes_an_item_that_opens_and_saves() {
    use bevy::ecs::system::RunSystemOnce;
    use nova_ship::prelude::HeldPart;

    use crate::{
        asset::blank::section_kinds,
        clipboard::EditorClipboard,
        config::EditorViewType,
        project::{package::asset_faults, store::read_project, verbs::save_project, DocumentType},
        ui::{add_menu::add_items, menu_context::tests::context, section_kind::SectionKindTile},
    };

    let _disk = disk();
    let fresh = || {
        let mut app = project_app();
        app.init_resource::<DoubleClick<Entity>>();
        app.init_resource::<Time<Real>>();
        app.init_resource::<HeldPart>();
        app.init_resource::<EditorClipboard>();
        app.insert_resource(EditorViewType::Scene);
        load_base_content(&mut app);
        new_mod(&mut app, "red");
        app
    };

    let views = [
        (DocumentType::Scenario, EditorViewType::Scene),
        (DocumentType::Scenario, EditorViewType::Events),
        (DocumentType::Ship, EditorViewType::Ship),
        (DocumentType::Section, EditorViewType::Section),
        (DocumentType::Form, EditorViewType::Form),
    ];
    let tree = new_asset_items(&context(DocumentType::Scenario, EditorViewType::Scene));
    let names = |items: &[CascadeItem]| -> Vec<String> {
        leaves(items).iter().map(|item| item.name.clone()).collect()
    };
    for (document, view) in views {
        let at = context(document, view);
        let add = add_items(&at, &[], None);
        let asset = add
            .iter()
            .find(|item| item.name == "Add Menu Asset")
            .expect("every Add tree holds New Asset");
        assert_eq!(
            names(&asset.children),
            names(&tree),
            "{document:?} {view:?}"
        );
        assert_eq!(names(&new_asset_items(&at)), names(&tree));
        for item in leaves(&add) {
            assert_eq!(item.greyed, None, "{} refuses in {view:?}", item.name);
        }
    }
    let offered: Vec<&str> = leaves(&tree)
        .iter()
        .map(|item| item.label.as_str())
        .collect();
    assert!(!offered.contains(&"Lesson"), "{offered:?}");
    assert!(!offered.contains(&"Station"), "{offered:?}");

    for item in leaves(&tree) {
        let kind = KINDS
            .iter()
            .find(|row| item.name == format!("Asset Add {}", row.heading))
            .expect("a row names its kind")
            .kind;
        let made = if kind == "section" {
            section_kinds().len()
        } else {
            1
        };
        for pick in 0..made {
            let mut app = fresh();
            let before = data(&app).ids_of(kind).len();
            choose(&mut app, item);
            if kind == "section" {
                let tile = app
                    .world_mut()
                    .query::<(Entity, &SectionKindTile)>()
                    .iter(app.world())
                    .find(|(_, tile)| tile.0 == pick)
                    .map(|(entity, _)| entity)
                    .expect("the modal asks the kind");
                app.world_mut().trigger(Activate { entity: tile });
                app.world_mut().trigger(Activate { entity: tile });
                app.update();
            }
            let held = data(&app);
            assert_eq!(
                held.ids_of(kind).len(),
                before + 1,
                "{} made a {kind}",
                item.name
            );
            let open = held.open.clone().expect("the new item is open");
            assert_eq!(open.kind, kind);
            // A blank the lint refuses waits to be filled: Save keeps it out
            // of the file until then, and Problems names each field.
            let faults = asset_faults(app.world_mut());
            save_project(app.world_mut());
            let saved = read_project("red").expect("the mod reads back");
            let written = saved.data.item(kind, &open.id).is_some();
            assert_eq!(
                written,
                faults.is_empty(),
                "{} saved {}",
                item.name,
                open.id
            );
        }
    }
    let mut app = fresh();
    choose(
        &mut app,
        leaves(&tree)
            .into_iter()
            .find(|item| item.name == "Asset Add Price Tables")
            .expect("a mod with no table is offered one"),
    );
    let after = app
        .world_mut()
        .run_system_once(|facts: MenuFacts, selected: Res<SelectedNode>| {
            new_asset_items(&facts.context(&selected))
        })
        .expect("the menu facts read");
    assert!(names(&after)
        .iter()
        .all(|name| name != "Asset Add Price Tables"));
}
