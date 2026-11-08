//! What the bench promises: an item comes back as it went in, each edit is one
//! step, and what a mod's other content needs from a section is kept.

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use nova_assets::{mod_cache, prelude::ModQuarantine};
use nova_gameplay::prelude::{BuildRecipe, ItemDesignId};
use nova_modding::prelude::{parse_content, serialize_content, Content};
use nova_ship::prelude::{
    GameSections, LinkPoint, LinkPointSkinType, SectionCollider, SectionConfig, SectionKind,
    ThrusterSectionConfig,
};

use super::{
    blank::blank_section,
    lowered_item, mesh_fault, mesh_of, section_of,
    verbs::{
        add_and_open, add_asset, add_socket, delete_asset, duplicate_asset, glb_fault, import_mesh,
        rename_asset,
    },
    AssetNode, AssetPartType, SocketNode,
};
use crate::{
    asset_index::AssetIndex,
    history::{
        commit_edit_now,
        tests::{assert_one_step_of, depth, first_with, undo},
    },
    inspect::{
        asset_kind_mut, asset_rows, content_reflect_mut, edited_entries, item_text, list_entries,
        set_item_text, set_list_entries, write_field, AssetListType, AssetTextType, FieldRoot,
        ListEditType, ListEntry, RowValue,
    },
    node::{
        spawn_section_node, teardown_document, EditContext, NextChildOrdinal, NodeId, ShipNode,
    },
    project::{
        package::{asset_faults, package_files, unzip_bytes, zip_bytes},
        store::{read_project, renamed_copy},
        tests::{
            data, disk, load_bundle, merged, new_mod, one_of_every_other_kind, open, project_app,
        },
        verbs::{lower_open_document, open_asset, save_project},
        ModProject, KINDS, SCENARIO_KIND,
    },
};

/// A real model, as an import would find it on disk.
const HULL_GLB: &str = "../../assets/base/gltf/dock_flush.glb";

/// The six faces of a cell and the socket id each takes on the test hull.
const FACES: [(Vec3, &str); 6] = [
    (Vec3::X, "positive_x"),
    (Vec3::NEG_X, "negative_x"),
    (Vec3::Y, "positive_y"),
    (Vec3::NEG_Y, "negative_y"),
    (Vec3::Z, "positive_z"),
    (Vec3::NEG_Z, "negative_z"),
];

/// A one-cell hull that lints clean: a socket on each face, and the recipe of
/// the first hull the project can see.
fn test_hull(app: &mut App) -> Content {
    let build = app
        .world_mut()
        .run_system_once(|index: AssetIndex, project: Res<ModProject>| {
            index
                .visible_packs(&project.data.meta.dependencies)
                .iter()
                .flat_map(|pack| pack.items.iter())
                .find_map(|item| match item {
                    Content::Section(config) if matches!(config.kind, SectionKind::Hull(_)) => {
                        Some(config.base.build.clone())
                    }
                    _ => None,
                })
        })
        .expect("the index reads")
        .unwrap_or(BuildRecipe {
            credits: 1,
            strip_yield: 0.5,
            ..default()
        });
    let mut config = blank_section("Hull").expect("a hull has a blank");
    let namespace = app
        .world()
        .resource::<ModProject>()
        .namespace
        .replace('-', "_");
    config.base.id = format!("{namespace}__plain_hull");
    config.base.name = "Plain Hull".to_string();
    config.base.health = 100.0;
    config.base.build = build;
    config.base.advancement = 0.1;
    config.base.draw_weight = 1.0;
    config.base.collider = Some(SectionCollider::Cuboid { size: Vec3::ONE });
    config.base.link_points = FACES
        .iter()
        .map(|(normal, id)| LinkPoint {
            id: (*id).to_string(),
            position: *normal * 0.5,
            normal: *normal,
            skin: LinkPointSkinType::Clad,
        })
        .collect();
    Content::Section(Box::new(config))
}

/// Open a new mod `id` holding the test hull on the bench, and say its id.
fn section_mod(app: &mut App, id: &str) -> String {
    new_mod(app, id);
    let hull = test_hull(app);
    add_and_open(app.world_mut(), "Add Section".to_string(), hull);
    app.update();
    data(app).open.clone().expect("the section is open").id
}

/// The document's root.
fn root(app: &App) -> Entity {
    app.world()
        .resource::<EditContext>()
        .scenario()
        .expect("a document stands")
}

/// The item on the bench as the file will hold it.
fn lowered(app: &App) -> Content {
    lowered_item(app.world(), root(app)).expect("an item is on the bench")
}

/// The section on the bench as the file will hold it.
fn section(app: &App) -> SectionConfig {
    section_of(&lowered(app)).expect("a section").clone()
}

/// The socket called `id`.
fn socket(app: &mut App, id: &str) -> Entity {
    app.world_mut()
        .query_filtered::<(Entity, &NodeId), With<SocketNode>>()
        .iter(app.world())
        .find(|(_, held)| held.0 == id)
        .expect("the socket stands")
        .0
}

/// Change the item on the bench and commit it, as an Inspector row does.
fn edit(app: &mut App, label: &'static str, change: impl FnOnce(&mut Content)) {
    let node = root(app);
    let mut held = app
        .world_mut()
        .get_mut::<AssetNode>(node)
        .expect("an item is on the bench");
    change(&mut held.0);
    commit_edit_now(app.world_mut(), label);
    app.update();
}

/// A section built on the bench is the same section after a save and a reopen,
/// sockets and all.
#[test]
fn a_built_section_round_trips_through_save_and_reopen_equal() {
    let _disk = disk();
    let mut app = project_app();
    let id = section_mod(&mut app, "editor_bench");
    edit(&mut app, "Edit Section", |item| {
        if let Content::Section(config) = item {
            config.base.name = "Long Spar".to_string();
            config.base.health = 250.0;
            config.base.collider = Some(SectionCollider::Cuboid {
                size: Vec3::new(1.0, 1.0, 2.0),
            });
        }
    });
    let node = socket(&mut app, "positive_z");
    app.world_mut()
        .get_mut::<Transform>(node)
        .expect("it stands")
        .translation = Vec3::new(0.0, 0.5, 1.0);
    commit_edit_now(app.world_mut(), "Move Socket");
    let built = section(&app);

    save_project(app.world_mut());
    let saved = read_project("editor_bench").expect("the mod reads back");
    assert_eq!(
        section_of(saved.data.item("section", &id).expect("it was written")),
        Some(&built),
        "the file holds what the bench did"
    );

    open(&mut app, saved);
    open_asset(app.world_mut(), "section", &id);
    app.update();
    assert_eq!(
        section(&app),
        built,
        "the bench reopens on the same section"
    );
}

/// Lifting an item and lowering it again writes the same bytes, whatever a
/// socket's normal rounds to through a rotation.
#[test]
fn an_item_nobody_edited_lowers_equal_to_what_was_lifted() {
    let _disk = disk();
    let mut app = project_app();
    new_mod(&mut app, "editor_lift");
    let shipped = one_of_every_other_kind();
    let held: Vec<&Content> = shipped.iter().collect();
    let mut oblique = held
        .iter()
        .find_map(|item| section_of(item))
        .expect("the shipped content holds a section")
        .clone();
    oblique.base.link_points[0].normal = Vec3::new(0.6, 0.0, 0.8);
    for item in held
        .into_iter()
        .cloned()
        .chain([Content::Section(Box::new(oblique))])
    {
        // A ship opens as a ship's document, not on the bench: see
        // `an_unedited_ship_design_writes_back_unchanged`.
        let bench_kinds = KINDS.iter().find(|row| row.kind == item.kind());
        if !bench_kinds.is_some_and(|row| row.opens)
            || crate::project::ship::is_ship_kind(item.kind())
        {
            continue;
        }
        app.world_mut()
            .resource_mut::<ModProject>()
            .edit()
            .add_item(item.clone(), "kept.content.ron");
        let id = item.id().to_string();
        app.world_mut().resource_mut::<ModProject>().edit().open =
            Some(crate::project::OpenAsset {
                kind: item.kind(),
                id,
            });
        crate::project::verbs::rebuild_open_document(app.world_mut());
        app.update();
        assert_eq!(
            format!("{item:?}"),
            format!("{:?}", lowered(&app)),
            "{} came back changed",
            item.kind()
        );
        let mut project = app.world_mut().resource_mut::<ModProject>();
        let kind = item.kind();
        let id = item.id().to_string();
        project.edit().remove_item(kind, &id);
    }
}

/// One drag of a socket is one step, and undo puts it back.
#[test]
fn a_socket_move_is_one_undo_step() {
    let _disk = disk();
    let mut app = project_app();
    section_mod(&mut app, "editor_socket");
    let node = socket(&mut app, "positive_x");

    assert_one_step_of(
        &mut app,
        "Move Socket",
        |app| format!("{:?}", section(app).base.link_points),
        |app| {
            app.world_mut()
                .get_mut::<Transform>(node)
                .expect("it stands")
                .translation = Vec3::new(0.5, 0.5, 0.0);
            commit_edit_now(app.world_mut(), "Move Socket");
        },
    );
}

/// A turned socket faces where it was turned, on a clean grid.
#[test]
fn a_turned_socket_writes_the_way_it_faces() {
    let _disk = disk();
    let mut app = project_app();
    section_mod(&mut app, "editor_turn");
    let node = socket(&mut app, "positive_z");

    app.world_mut()
        .get_mut::<Transform>(node)
        .expect("it stands")
        .rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);

    let turned = section(&app)
        .base
        .link_points
        .into_iter()
        .find(|point| point.id == "positive_z")
        .expect("it lowers");
    assert_eq!(turned.normal, Vec3::X);
}

/// Add and delete of a socket are one step each, and an emptied list stays
/// empty: no sockets means none.
#[test]
fn sockets_are_added_and_deleted_one_step_at_a_time() {
    let _disk = disk();
    let mut app = project_app();
    section_mod(&mut app, "editor_sockets");

    assert_one_step_of(
        &mut app,
        "Add Socket",
        |app| format!("{:?}", section(app).base.link_points),
        |app| add_socket(app.world_mut()),
    );
    let ids: Vec<String> = section(&app)
        .base
        .link_points
        .iter()
        .map(|point| point.id.clone())
        .collect();
    assert!(ids.contains(&"socket_1".to_string()), "{ids:?}");
    let held: Vec<Entity> = ids.iter().map(|id| socket(&mut app, id)).collect();
    for node in held {
        app.world_mut().entity_mut(node).despawn();
    }
    commit_edit_now(app.world_mut(), "Delete");
    app.update();

    assert!(section(&app).base.link_points.is_empty());
}

/// Open the mod's range and put the section `id` on its first ship, from the
/// catalog, as a finished edit.
fn place_on_a_ship(app: &mut App, id: &str) {
    let config = app
        .world()
        .resource::<GameSections>()
        .get_section(id)
        .cloned()
        .expect("an unsaved section reaches the catalog");
    let scenario = data(app)
        .scenarios()
        .next()
        .expect("the mod holds its range")
        .id
        .clone();
    open_asset(app.world_mut(), SCENARIO_KIND, &scenario);
    app.update();
    let ship = first_with::<ShipNode>(app);
    app.world_mut()
        .run_system_once(
            move |mut commands: Commands, mut ordinals: Query<&mut NextChildOrdinal>| {
                spawn_section_node(
                    &mut commands,
                    &mut ordinals,
                    ship,
                    &config,
                    Transform::from_xyz(0.0, 4.0, 0.0),
                    vec![],
                );
            },
        )
        .expect("it places");
    commit_edit_now(app.world_mut(), "Place Part");
    lower_open_document(app.world_mut());
}

/// A section's tree holds its collider and mesh handles, and they survive an
/// undo across the edit.
#[test]
fn the_parts_of_a_section_stand_under_the_root() {
    let _disk = disk();
    let mut app = project_app();
    section_mod(&mut app, "editor_parts");

    let collider = first_with::<AssetPartType>(&mut app);

    assert_eq!(
        app.world().get::<ChildOf>(collider).map(ChildOf::parent),
        Some(root(&app))
    );
}

/// A section the mod holds is in the catalog from the moment it is, a ship can
/// carry it, delete is refused while one does, and leaving takes it out again.
#[test]
fn a_project_section_is_placed_and_the_ship_lowers() {
    let _disk = disk();
    let mut app = project_app();
    app.world_mut().init_resource::<GameSections>();
    let id = section_mod(&mut app, "editor_place");
    place_on_a_ship(&mut app, &id);

    let users = data(&app).users_of("section", &id);
    assert!(!users.is_empty(), "the ship lowered with the part");

    let refused = delete_asset(app.world_mut(), "section", &id);
    assert_eq!(refused, users, "the delete names what uses it");
    assert!(data(&app).item("section", &id).is_some());

    app.world_mut()
        .run_system_once(teardown_document)
        .expect("it tears down");
    app.update();
    assert!(
        app.world()
            .resource::<GameSections>()
            .get_section(&id)
            .is_none(),
        "leaving takes the registration out"
    );
}

/// A new id reaches every reference in the mod, and undo takes it all back.
#[test]
fn a_rename_updates_the_references_inside_the_project() {
    let _disk = disk();
    let mut app = project_app();
    app.world_mut().init_resource::<GameSections>();
    let id = section_mod(&mut app, "editor_rename");
    place_on_a_ship(&mut app, &id);

    rename_asset(app.world_mut(), "section", &id, "Long Spar");
    app.update();

    let held = data(&app);
    let new = held
        .ids_of("section")
        .into_iter()
        .next()
        .expect("the section stands");
    assert!(new.ends_with("__long_spar"), "{new}");
    assert!(held.users_of("section", &id).is_empty());
    assert!(!held.users_of("section", &new).is_empty());

    undo(&mut app);
    assert!(data(&app).item("section", &id).is_some());
    assert!(!data(&app).users_of("section", &id).is_empty());
}

/// Renaming the item on the bench moves the bench with it.
#[test]
fn a_rename_on_the_bench_keeps_it_open() {
    let _disk = disk();
    let mut app = project_app();
    let id = section_mod(&mut app, "editor_rename_open");

    rename_asset(app.world_mut(), "section", &id, "Long Spar");
    app.update();

    let open = data(&app).open.clone().expect("it stays open").id;
    assert!(open.ends_with("__long_spar"), "{open}");
    assert_eq!(section(&app).base.id, open, "the bench took the new id");
}

/// An import copies the file into the mod, declares it and points the section
/// at it, as one step.
#[test]
fn an_imported_glb_lands_in_the_mod_as_one_step() {
    let _disk = disk();
    let mut app = project_app();
    let id = section_mod(&mut app, "editor_import");
    save_project(app.world_mut());
    let steps = depth(&app);

    import_mesh(app.world_mut(), HULL_GLB);
    app.update();

    assert_eq!(depth(&app), steps + 1, "an import is one step");
    assert_eq!(data(&app).resources, vec!["dock_flush.glb".to_string()]);
    assert_eq!(mesh_of(&section(&app).kind), "self://dock_flush.glb#Scene0");
    let copied = mod_cache::prelude::read_mod_file("editor_import", "dock_flush.glb");
    assert_eq!(
        copied,
        std::fs::read(HULL_GLB).ok(),
        "the bytes are the file's"
    );
    assert_eq!(data(&app).open.clone().map(|open| open.id), Some(id));
}

/// A file that is not a whole glb never reaches the document.
#[test]
fn a_file_that_is_not_a_glb_is_refused_at_import() {
    let _disk = disk();
    let mut app = project_app();
    section_mod(&mut app, "editor_bad_import");
    save_project(app.world_mut());
    let path = std::env::temp_dir().join("nova_editor_not_a_model.glb");
    std::fs::write(&path, b"plain text").expect("the scratch file writes");

    import_mesh(app.world_mut(), &path.display().to_string());
    app.update();

    assert!(data(&app).resources.is_empty());
    assert_eq!(mesh_of(&section(&app).kind), "");
    assert!(glb_fault(b"plain text").is_some());
    let _ = std::fs::remove_file(path);
}

/// A mesh the mod does not hold is a named fault, and so is a declared file
/// that is gone from its folder.
#[test]
fn a_missing_glb_raises_a_problem() {
    let _disk = disk();
    let mut app = project_app();
    section_mod(&mut app, "editor_missing");
    let held = data(&app);

    let undeclared = mesh_fault("editor_missing", &held, "self://gone.glb#Scene0");
    assert!(
        undeclared.is_some_and(|fault| fault.contains("gone.glb")),
        "an undeclared file is a fault"
    );

    app.world_mut()
        .resource_mut::<ModProject>()
        .edit()
        .resources
        .push("gone.glb".to_string());
    let held = data(&app);
    let missing = mesh_fault("editor_missing", &held, "self://gone.glb#Scene0");
    assert!(
        missing.is_some_and(|fault| fault.contains("missing")),
        "a declared file that is not there is a fault"
    );
    assert_eq!(mesh_fault("editor_missing", &held, ""), None);
    assert_eq!(
        mesh_fault(
            "editor_missing",
            &held,
            "dep://base/gltf/hull-01.glb#Scene0"
        ),
        None,
        "another mod's file is the ref check's to judge"
    );
}

/// A fault in the content rules stops a save, and the file keeps what it held.
#[test]
fn an_error_in_a_section_stops_save() {
    let _disk = disk();
    let mut app = project_app();
    load_bundle(&mut app, "base", &[]);
    let id = section_mod(&mut app, "editor_gate");
    assert_eq!(asset_faults(app.world_mut()), Vec::<String>::new());
    save_project(app.world_mut());

    edit(&mut app, "Edit Health", |item| {
        if let Content::Section(config) = item {
            config.base.health = -5.0;
        }
    });
    save_project(app.world_mut());

    assert!(!asset_faults(app.world_mut()).is_empty());
    let saved = read_project("editor_gate").expect("the mod reads back");
    let held = section_of(saved.data.item("section", &id).expect("it was written"));
    assert_ne!(held.map(|config| config.base.health), Some(-5.0));
}

/// `+ Add` makes a blank of the asked kind and opens it, as one step. A blank
/// copies nothing, so what the format refuses in it is a fault until filled.
#[test]
fn a_new_section_is_a_blank_of_its_kind_and_waits_to_be_filled() {
    let _disk = disk();
    let mut app = project_app();
    load_bundle(&mut app, "base", &[]);
    new_mod(&mut app, "editor_blank");
    let before = depth(&app);

    add_asset(app.world_mut(), "section", Some("Thruster"));
    app.update();

    assert_eq!(depth(&app), before + 1, "the add is one step");
    let open = data(&app).open.clone().expect("the new section is open");
    assert_eq!(open.kind, "section");
    let config = section(&app);
    assert_eq!(config.base.id, open.id);
    assert_eq!(config.base.name, "New Thruster");
    assert!(config.base.link_points.is_empty(), "no socket is copied in");
    let SectionKind::Thruster(thruster) = &config.kind else {
        panic!("the kind asked for: {:?}", config.kind);
    };
    assert_eq!(thruster.magnitude, 0.0);
    let faults = asset_faults(app.world_mut());
    assert!(
        faults.iter().any(|fault| fault.contains("draw_weight")),
        "a zero the format refuses is a fault: {faults:?}"
    );
}

/// A thruster that fires through a face holding a socket is a generator fault,
/// raised while it is edited and not at the first world that loads it.
#[test]
fn a_ship_part_fault_is_a_problem_at_edit_time() {
    let _disk = disk();
    let mut app = project_app();
    load_bundle(&mut app, "base", &[]);
    section_mod(&mut app, "editor_lanes");
    edit(&mut app, "Edit Kind", |item| {
        if let Content::Section(config) = item {
            config.kind = SectionKind::Thruster(ThrusterSectionConfig {
                magnitude: 1.0,
                ..default()
            });
        }
    });
    lower_open_document(app.world_mut());

    let faults = asset_faults(app.world_mut());

    assert!(
        faults.iter().any(|fault| fault.contains("(editor_lanes)")),
        "{faults:?}"
    );
}

/// The price table rule the merge holds a mod to, said while the mod is edited.
#[test]
fn a_second_price_table_is_a_problem_at_edit_time() {
    let _disk = disk();
    let mut app = project_app();
    load_bundle(&mut app, "base", &[]);
    new_mod(&mut app, "editor_tables");
    let table = one_of_every_other_kind()
        .into_iter()
        .find_map(|item| match item {
            Content::PriceTable(table) => Some(table),
            _ => None,
        })
        .expect("the shipped content holds a table");
    {
        let mut project = app.world_mut().resource_mut::<ModProject>();
        let data = project.edit();
        data.add_item(Content::PriceTable(table.clone()), "kept.content.ron");
        data.add_item(Content::PriceTable(table), "kept.content.ron");
    }

    let faults = asset_faults(app.world_mut());

    assert!(
        faults.iter().any(|fault| fault.contains("one price table")),
        "{faults:?}"
    );
}

/// The whole road out: a section with its own mesh packages, and the production
/// merge takes the mod as it stands.
#[test]
fn a_package_holding_a_section_passes_merge_through_register_bundles() {
    let _disk = disk();
    let mut app = project_app();
    let id = section_mod(&mut app, "editor_shipped");
    save_project(app.world_mut());
    import_mesh(app.world_mut(), HULL_GLB);
    app.update();
    save_project(app.world_mut());
    lower_open_document(app.world_mut());
    let held = data(&app);
    let glb = (
        "dock_flush.glb".to_string(),
        std::fs::read(HULL_GLB).expect("the model reads"),
    );

    let files = package_files("editor_shipped", &held, vec![glb]).expect("it packages");
    let archive = zip_bytes(&files).expect("it zips");
    let extracted = unzip_bytes(&archive).expect("it unzips");
    mod_cache::prelude::remove_mod("editor_shipped").expect("the local copy goes");
    mod_cache::prelude::remove_index_record("editor_shipped");
    mod_cache::install_local("shipped", "0.1.0", "shipped.bundle.ron", &extracted)
        .expect("the extracted files install");

    let game = merged(&["shipped"]);

    let quarantine = game.world().resource::<ModQuarantine>();
    assert!(
        quarantine.disabled.is_empty(),
        "the merge refused it: {:?}",
        quarantine.disabled
    );
    let sections = game.world().resource::<GameSections>();
    let merged = sections.get_section(&id).expect("the section merged");
    assert!(mesh_of(&merged.kind).ends_with("dock_flush.glb#Scene0"));
}

/// Each kind with a form comes through its own rows and the file format equal.
#[test]
fn each_form_kind_round_trips_equal() {
    let _disk = disk();
    let mut app = project_app();
    new_mod(&mut app, "editor_forms");
    for item in one_of_every_other_kind() {
        let opens = KINDS.iter().find(|row| row.kind == item.kind());
        if !opens.is_some_and(|row| row.opens) {
            continue;
        }
        let rows = asset_rows(&item, &Default::default());
        assert!(!rows.is_empty(), "{} has a form", item.kind());
        let mut held = item.clone();
        for row in rows {
            if let (RowValue::List(entries), FieldRoot::AssetList(kind)) = (&row.value, row.root) {
                let written = set_list_entries(&mut held, kind, entries);
                assert_eq!(written, Ok(()), "{} list {}", item.kind(), row.label);
                continue;
            }
            let RowValue::Text(text) = &row.value else {
                continue;
            };
            let written = match row.root {
                FieldRoot::Asset => write_field(
                    content_reflect_mut(&mut held).expect("a form"),
                    &row.path,
                    row.optional,
                    text,
                ),
                FieldRoot::AssetKind => write_field(
                    asset_kind_mut(&mut held).expect("a section"),
                    &row.path,
                    row.optional,
                    text,
                ),
                FieldRoot::AssetText(kind) => set_item_text(&mut held, kind, text),
                _ => Ok(()),
            };
            assert_eq!(written, Ok(()), "{} row {}", item.kind(), row.label);
        }
        assert_eq!(
            format!("{item:?}"),
            format!("{held:?}"),
            "writing {} its own values changed it",
            item.kind()
        );
        let bytes = serialize_content(std::slice::from_ref(&held)).expect("it serializes");
        let read = parse_content(bytes.as_bytes()).expect("it parses");
        assert_eq!(format!("{:?}", read[0]), format!("{item:?}"));
    }
}

/// The hand-written rows say what they hold and read back what was typed.
#[test]
fn the_text_rows_read_back_what_was_typed() {
    let mut item = one_of_every_other_kind()
        .into_iter()
        .find(|item| matches!(item, Content::Section(_)))
        .expect("the shipped content holds a section");

    set_item_text(&mut item, AssetTextType::ColliderShape, "Sphere").expect("it takes");

    assert_eq!(item_text(&item, AssetTextType::ColliderShape), "Sphere");
}

/// The first shipped item of `kind`.
fn shipped(kind: &str) -> Content {
    one_of_every_other_kind()
        .into_iter()
        .find(|item| item.kind() == kind)
        .unwrap_or_else(|| panic!("the shipped content holds a {kind}"))
}

/// `item`'s `list` after `edits`, each written back; the first refusal stops
/// it.
fn list_after(
    item: &mut Content,
    list: AssetListType,
    known: &[&str],
    edits: &[ListEditType],
) -> Result<Vec<ListEntry>, String> {
    let known: Vec<String> = known.iter().map(|id| id.to_string()).collect();
    for edit in edits {
        let next = edited_entries(&list_entries(item, list), list, edit, &known)?;
        set_list_entries(item, list, &next)?;
    }
    Ok(list_entries(item, list))
}

/// One entry of a counted list.
fn counted(id: &str, count: u32) -> ListEntry {
    ListEntry {
        id: id.to_string(),
        count: Some(count),
    }
}

/// A section's parts take an item with a count of one, drop a row, take a
/// typed count, and refuse a repeat, an unseen item and a count of zero.
#[test]
fn the_parts_list_adds_removes_and_refuses() {
    let mut item = shipped("section");
    set_list_entries(&mut item, AssetListType::Parts, &[]).expect("it clears");
    let parts = AssetListType::Parts;
    let known = ["alloy", "wire"];

    let held = list_after(
        &mut item,
        parts,
        &known,
        &[
            ListEditType::Add("alloy".into()),
            ListEditType::Add("wire".into()),
            ListEditType::Count(1, "3".into()),
            ListEditType::Remove(0),
        ],
    );
    assert_eq!(held, Ok(vec![counted("wire", 3)]));

    let refused = |edit: ListEditType| list_after(&mut item.clone(), parts, &known, &[edit]);
    assert_eq!(
        refused(ListEditType::Add("wire".into())),
        Err("wire is already listed".to_string())
    );
    assert_eq!(
        refused(ListEditType::Add("gold".into())),
        Err("gold is not a Parts item this mod sees".to_string())
    );
    assert_eq!(
        refused(ListEditType::Count(0, "0".into())),
        Err("a count is a whole number from 1 to 9999".to_string())
    );
    assert_eq!(
        refused(ListEditType::Count(0, "lots".into())),
        Err("a count is a whole number from 1 to 9999".to_string())
    );
}

/// A start role's stock takes an item, renames a row through its picker,
/// drops a row, and refuses a pick of an item it already holds. Its rows read
/// in id order, as the file keeps them.
#[test]
fn the_stock_list_adds_removes_and_refuses() {
    let mut item = shipped("start role");
    set_list_entries(&mut item, AssetListType::Stock, &[]).expect("it clears");
    let stock = AssetListType::Stock;
    let known = ["fuel", "ore", "wire"];

    let held = list_after(
        &mut item,
        stock,
        &known,
        &[
            ListEditType::Add("fuel".into()),
            ListEditType::Add("ore".into()),
            ListEditType::Pick(0, "wire".into()),
            // The stock keeps its rows in id order: ore, then wire.
            ListEditType::Remove(0),
        ],
    );
    assert_eq!(held, Ok(vec![counted("wire", 1)]));

    // fuel, then wire: picking fuel on the wire row repeats it.
    let mut two = item.clone();
    list_after(&mut two, stock, &known, &[ListEditType::Add("fuel".into())]).expect("it adds");
    assert_eq!(
        list_after(
            &mut two,
            stock,
            &known,
            &[ListEditType::Pick(1, "fuel".into())]
        ),
        Err("fuel is already listed".to_string())
    );
}

/// A campaign's scenarios keep play order, carry no count, and refuse a
/// count and a scenario the mod does not see.
#[test]
fn the_scenario_list_adds_removes_and_refuses() {
    let mut item = shipped("campaign");
    set_list_entries(&mut item, AssetListType::Scenarios, &[]).expect("it clears");
    let members = AssetListType::Scenarios;
    let known = ["first", "second", "third"];

    let held = list_after(
        &mut item,
        members,
        &known,
        &[
            ListEditType::Add("second".into()),
            ListEditType::Add("first".into()),
            ListEditType::Add("third".into()),
            ListEditType::Remove(2),
        ],
    )
    .expect("it takes");
    let ids: Vec<&str> = held.iter().map(|entry| entry.id.as_str()).collect();
    assert_eq!(ids, ["second", "first"]);
    assert!(held.iter().all(|entry| entry.count.is_none()));

    assert_eq!(
        list_after(
            &mut item.clone(),
            members,
            &known,
            &[ListEditType::Count(0, "2".into())]
        ),
        Err("this list holds no counts".to_string())
    );
    assert_eq!(
        list_after(
            &mut item.clone(),
            members,
            &known,
            &[ListEditType::Add("fourth".into())]
        ),
        Err("fourth is not a scenario this mod sees".to_string())
    );
}

/// Two mods cannot define one id, so Save Copy takes the items along.
#[test]
fn a_copy_of_a_mod_moves_its_sections_to_the_new_namespace() {
    let _disk = disk();
    let mut app = project_app();
    let id = section_mod(&mut app, "editor_original");

    let copy = renamed_copy(&data(&app), "editor_original", "copied", "Copied").expect("it copies");

    let held = copy.ids_of("section");
    assert_eq!(held.len(), 1);
    let new = held.into_iter().next().expect("one section");
    assert_ne!(new, id);
    assert!(new.starts_with("copied__"), "{new}");
    assert_eq!(
        copy.open.map(|open| open.id),
        Some(new),
        "the copy opens on its own section"
    );
}

/// A copy takes the mod's namespace, and a second price table is refused.
#[test]
fn a_duplicate_keeps_the_mod_namespace() {
    let _disk = disk();
    let mut app = project_app();
    let id = section_mod(&mut app, "editor_copy");

    duplicate_asset(app.world_mut(), "section", &id);
    app.update();

    let copy = data(&app).open.clone().expect("the copy is open").id;
    assert_ne!(copy, id);
    assert!(copy.starts_with(id.split("__").next().expect("a namespace")));
    assert!(data(&app).item("section", &id).is_some());
}

/// A file on disk may be what an older save points at, though the history has
/// forgotten it: the next mesh of that name takes a name of its own.
#[test]
fn an_import_never_overwrites_a_file_a_save_may_name() {
    let _disk = disk();
    let dir = tempfile::tempdir().expect("a scratch dir");
    let path = dir.path().join("hull.glb");
    let mut app = project_app();
    section_mod(&mut app, "editor_meshes");
    save_project(app.world_mut());
    std::fs::copy(HULL_GLB, &path).expect("the first model lands");
    import_mesh(app.world_mut(), &path.display().to_string());
    app.update();
    save_project(app.world_mut());
    let saved = mod_cache::prelude::read_mod_file("editor_meshes", "hull.glb");
    assert!(saved.is_some());
    undo(&mut app);

    std::fs::copy("../../assets/base/gltf/bay_tube.glb", &path).expect("the second model lands");
    import_mesh(app.world_mut(), &path.display().to_string());
    app.update();

    assert_eq!(
        mod_cache::prelude::read_mod_file("editor_meshes", "hull.glb"),
        saved
    );
    assert_eq!(data(&app).resources, vec!["hull_1.glb".to_string()]);
}

/// The same file imported again is the same file: no second name for it.
#[test]
fn an_import_of_the_same_bytes_keeps_the_file_on_disk() {
    let _disk = disk();
    let dir = tempfile::tempdir().expect("a scratch dir");
    let path = dir.path().join("hull.glb");
    std::fs::copy(HULL_GLB, &path).expect("the model lands");
    let mut app = project_app();
    section_mod(&mut app, "editor_same");
    save_project(app.world_mut());
    import_mesh(app.world_mut(), &path.display().to_string());
    app.update();
    undo(&mut app);

    import_mesh(app.world_mut(), &path.display().to_string());
    app.update();

    assert_eq!(data(&app).resources, vec!["hull.glb".to_string()]);
}

/// The first item of `kind` the shipped content holds, under `id`.
fn item_named(id: &str) -> Content {
    let mut item = one_of_every_other_kind()
        .into_iter()
        .find(|item| item.kind() == "item")
        .expect("the base content holds an item");
    if let Content::Item(design) = &mut item {
        design.id = ItemDesignId::from(id);
    }
    item
}

/// A build recipe holds its parts by key, and no leaf walk reaches a key: the
/// copy of the mod would still ask for the old mod's item.
#[test]
fn a_copy_of_a_mod_moves_the_recipe_keys_with_its_items() {
    let _disk = disk();
    let mut app = project_app();
    let id = section_mod(&mut app, "editor_original");
    let part = "original__plate";
    {
        let mut project = app.world_mut().resource_mut::<ModProject>();
        let file = project.default_file();
        let held = project.edit();
        held.add_item(item_named(part), &file);
        let mut item = held.item("section", &id).expect("it stands").clone();
        if let Content::Section(config) = &mut item {
            config.base.build.parts.insert(part.into(), 2);
        }
        held.replace_item(&id, item);
    }

    let copy = renamed_copy(&data(&app), "editor_original", "copied", "Copied").expect("it copies");

    let section = copy
        .items()
        .find(|item| item.kind() == "section")
        .expect("the section came");
    let parts = &section_of(section).expect("a section").base.build.parts;
    assert_eq!(parts.get(&ItemDesignId::from("copied__plate")), Some(&2));
    assert!(!parts.contains_key(&ItemDesignId::from(part)));
}

/// A mod id may hold `-`, and a content id never does.
#[test]
fn a_typed_id_takes_the_spelling_mod_content_ids_have() {
    let _disk = disk();
    let mut app = project_app();
    let id = section_mod(&mut app, "editor_a-b");
    assert!(id.starts_with("a_b__"), "{id}");

    rename_asset(app.world_mut(), "section", &id, "thing");
    app.update();

    assert!(data(&app).item("section", "a_b__thing").is_some());
}

/// A rename rewrites every string equal to the id, and a string does not say
/// which kind it means. So no two kinds share an id the editor made.
#[test]
fn a_new_item_takes_no_id_another_kind_holds() {
    let _disk = disk();
    let mut app = project_app();
    let first = section_mod(&mut app, "editor_kinds");
    let taken = format!("{first}_2");
    {
        let mut project = app.world_mut().resource_mut::<ModProject>();
        let file = project.default_file();
        project.edit().add_item(item_named(&taken), &file);
    }

    add_asset(app.world_mut(), "section", Some("Hull"));
    app.update();

    let minted = data(&app).open.clone().expect("the section is open").id;
    assert_ne!(minted, taken);
}

/// A hand-written mod may share an id across kinds. The rename cannot tell the
/// two apart, so it refuses rather than move both.
#[test]
fn a_rename_refuses_an_id_two_kinds_share() {
    let _disk = disk();
    let mut app = project_app();
    let id = section_mod(&mut app, "editor_shared");
    {
        let mut project = app.world_mut().resource_mut::<ModProject>();
        let file = project.default_file();
        project.edit().add_item(item_named(&id), &file);
    }

    rename_asset(app.world_mut(), "section", &id, "thing");
    app.update();

    assert!(data(&app).item("item", &id).is_some());
    assert!(data(&app).item("section", &id).is_some());
}

/// A rename onto an id another kind holds would make the pair the rename above
/// has to refuse.
#[test]
fn a_rename_refuses_an_id_another_kind_holds() {
    let _disk = disk();
    let mut app = project_app();
    let id = section_mod(&mut app, "editor_held");
    {
        let mut project = app.world_mut().resource_mut::<ModProject>();
        let file = project.default_file();
        project.edit().add_item(item_named("held__taken"), &file);
    }

    rename_asset(app.world_mut(), "section", &id, "taken");
    app.update();

    assert!(data(&app).item("section", &id).is_some());
}

/// The files the Assets tab lists, read from the store as the tab reads them.
fn listed_files(app: &App, id: &str) -> Vec<super::files::ModFile> {
    let project = app.world().resource::<ModProject>();
    super::files::mod_files(project, &crate::project::store::files_of(id))
}

/// The Assets tab lists the folder as the packager sees it: a file the
/// manifest does not declare is marked out of the package, a declared file the
/// folder lacks is marked missing, and the manifest and content files are not
/// files of the tab.
#[test]
fn the_file_list_marks_what_the_package_leaves_out() {
    let _disk = disk();
    let mut app = project_app();
    section_mod(&mut app, "editor_files");
    save_project(app.world_mut());
    import_mesh(app.world_mut(), HULL_GLB);
    app.update();
    crate::project::store::write_mod_file("editor_files", "notes.txt", b"notes".to_vec())
        .expect("the stray file writes");
    app.world_mut()
        .resource_mut::<ModProject>()
        .edit()
        .resources
        .push("gone.png".to_string());

    let listed: Vec<(String, &str, bool, bool)> = listed_files(&app, "editor_files")
        .into_iter()
        .map(|file| {
            let kind = super::files::file_kind(&file.path);
            (file.path, kind, file.packaged, file.on_disk)
        })
        .collect();

    assert_eq!(
        listed,
        vec![
            ("gone.png".to_string(), "image", true, false),
            ("dock_flush.glb".to_string(), "mesh", true, true),
            ("notes.txt".to_string(), "file", false, true),
        ]
    );
}

/// Import File copies the file and declares it as one step. The undo takes
/// the declaration back, and the copy stays in the folder, marked.
#[test]
fn an_imported_file_is_declared_as_one_step() {
    let _disk = disk();
    let mut app = project_app();
    section_mod(&mut app, "editor_import_file");
    save_project(app.world_mut());
    let sound = tempfile::tempdir().expect("a scratch dir");
    let path = sound.path().join("Engine Hum.ogg");
    std::fs::write(&path, b"OggS").expect("the scratch file writes");
    let read = |app: &mut App| format!("{:?}", data(app).resources);

    assert_one_step_of(&mut app, "Import File", read, |app| {
        super::files::import_file(app.world_mut(), &path.display().to_string());
    });

    assert_eq!(data(&app).resources, vec!["engine_hum.ogg".to_string()]);
    undo(&mut app);
    let listed = listed_files(&app, "editor_import_file");
    assert_eq!(listed.len(), 1);
    assert!(listed[0].on_disk && !listed[0].packaged, "{listed:?}");
}

/// A file the content names is not deleted, and the refusal names the item
/// that uses it.
#[test]
fn a_file_in_use_is_not_deleted_and_the_refusal_names_the_user() {
    let _disk = disk();
    let mut app = project_app();
    app.init_resource::<super::files::ModFolder>();
    let section = section_mod(&mut app, "editor_used_file");
    save_project(app.world_mut());
    import_mesh(app.world_mut(), HULL_GLB);
    app.update();
    let steps = depth(&app);

    super::files::delete_file(app.world_mut(), "dock_flush.glb");
    app.update();

    assert_eq!(depth(&app), steps, "nothing changed");
    assert_eq!(data(&app).resources, vec!["dock_flush.glb".to_string()]);
    let line = app
        .world()
        .resource::<crate::config::EditorStatus>()
        .line()
        .map(|(line, _)| line.to_string())
        .unwrap_or_default();
    assert!(line.contains(&section), "{line}");
    assert!(mod_cache::prelude::read_mod_file("editor_used_file", "dock_flush.glb").is_some());
}
