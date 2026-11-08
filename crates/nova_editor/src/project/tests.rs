//! What a mod keeps, what a step takes back, and what a package is.

use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use bevy::{
    asset::{AssetPlugin, RecursiveDependencyLoadState, UntypedAssetId},
    ecs::system::RunSystemOnce,
    state::app::StatesPlugin,
};
use nova_assets::{
    mod_cache::{self, prelude::read_mod_file},
    mod_refs::prelude::{prototype_refs, string_leaves},
    prelude::*,
};
use nova_gameplay::prelude::{AssetRef, GameStates};
use nova_modding::prelude::{
    parse_content, serialize_content, serialize_manifest, BundleAsset, InstalledCatalog,
    NovaModdingPlugin,
};
use nova_scenario::prelude::{EventConfig, GameScenarios, GameShipDesigns, ScenarioObjectKind};

use super::{
    package::{
        adopt_package, import_over, import_package, manifest_findings, other_files, package_files,
        play_faults, unzip_bytes, zip_bytes,
    },
    store::{
        content_file, local_ids, manifest_of, read_project, renamed_copy, write_copy, write_project,
    },
    verbs::{
        add_dependency, add_scenario, delete_scenario, duplicate_scenario, lower_open_document,
        name_project, open_asset, open_project_document, remove_dependency, rename_scenario,
        save_project,
    },
    SCENARIO_KIND, *,
};
use crate::{
    bundle::FileWindowRequest,
    config::EditorViewType,
    event::{place_new, BlockItemType, BlockSlotType, EventNode, FilterChoice, FilterNode},
    history::{
        commit_edit_now,
        tests::{depth, first_with, history_app, redo, undo},
        EditHistory,
    },
    node::{teardown_document, EditContext, NodeId, ScenarioNode, ShipNode},
    problems::ProblemSeverityType,
    ui::hub::HubForm,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

thread_local! {
    /// Whether this test holds [`disk`]. A founded mod writes the store at once,
    /// and with no [`disk`] that store is the developer's real profile.
    static HOLDS_DISK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Everything a save writes, in a fresh directory. The cache root, the data dir
/// and the config dir are process-wide, so disk tests take this one lock, and
/// none of them can reach a developer's own mods or settings.
pub(crate) struct DiskGuard {
    _lock: MutexGuard<'static, ()>,
    _root: tempfile::TempDir,
}

impl Drop for DiskGuard {
    fn drop(&mut self) {
        HOLDS_DISK.set(false);
    }
}

pub(crate) fn disk() -> DiskGuard {
    let lock = ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    HOLDS_DISK.set(true);
    let root = tempfile::tempdir().expect("a temp root");
    std::env::set_var("NOVA_MODDING_CACHE_ROOT", root.path());
    std::env::set_var("XDG_DATA_HOME", root.path().join("data"));
    std::env::set_var("XDG_CONFIG_HOME", root.path().join("config"));
    DiskGuard {
        _lock: lock,
        _root: root,
    }
}

/// The history rig, with what the file verbs write.
pub(crate) fn project_app() -> App {
    let mut app = history_app();
    app.init_resource::<FileWindowRequest>();
    app
}

/// Close whatever is open and stand `project` up in the document.
pub(crate) fn open(app: &mut App, project: ModProject) {
    assert!(
        project.id.is_none() || project.seed.is_none() || HOLDS_DISK.get(),
        "founding a mod writes the store: take disk() first"
    );
    let world = app.world_mut();
    world
        .run_system_once(teardown_document)
        .expect("it tears down");
    world.insert_resource(project);
    open_project_document(world);
    app.update();
}

/// A new mod on disk, founded on the stock range.
pub(crate) fn new_mod(app: &mut App, id: &str) {
    let project = ModProject::new(
        Some(id.to_string()),
        "Red Run",
        "tester",
        ScenarioTemplate::Range,
    );
    open(app, project);
}

pub(crate) fn data(app: &App) -> Arc<ProjectData> {
    Arc::clone(&app.world().resource::<ModProject>().data)
}

fn open_id(app: &App) -> String {
    data(app).open.clone().expect("a scenario is open").id
}

/// The whole mod, as text, by file.
pub(crate) fn written(data: &ProjectData) -> BTreeMap<String, String> {
    data.files
        .iter()
        .map(|file| {
            let items: Vec<Content> = file.items.iter().map(|item| (**item).clone()).collect();
            (
                file.path.clone(),
                serialize_content(&items).expect("the items serialize"),
            )
        })
        .collect()
}

/// Rename the open scenario's root and commit it, as an inspector row does.
fn edit_name(app: &mut App, name: &str) {
    let root = app
        .world()
        .resource::<EditContext>()
        .scenario()
        .expect("a document stands");
    app.world_mut()
        .get_mut::<ScenarioNode>(root)
        .expect("the root is a scenario")
        .name = name.to_string();
    commit_edit_now(app.world_mut(), "Edit Name");
    app.update();
}

fn document_name(app: &App) -> String {
    let root = app
        .world()
        .resource::<EditContext>()
        .scenario()
        .expect("a document stands");
    app.world()
        .get::<ScenarioNode>(root)
        .expect("the root is a scenario")
        .name
        .clone()
}

/// Every `Content` kind but a scenario, read from the shipped content, one each.
pub(crate) fn one_of_every_other_kind() -> Vec<Content> {
    fn walk(dir: &Path, found: &mut BTreeMap<&'static str, Content>) {
        for entry in std::fs::read_dir(dir).expect("the base content is here") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                walk(&path, found);
            } else if path.to_string_lossy().ends_with(".content.ron") {
                let bytes = std::fs::read(&path).expect("a content file reads");
                for item in parse_content(&bytes).expect("a shipped file parses") {
                    found.entry(item.kind()).or_insert(item);
                }
            }
        }
    }
    let mut found = BTreeMap::new();
    walk(Path::new("../../assets/base"), &mut found);
    assert_eq!(
        found.len(),
        KINDS.len(),
        "the shipped content holds every kind the table names"
    );
    found.remove("scenario");
    found.into_values().collect()
}

/// Kinds the editor cannot edit ride through a save untouched: a mod holding one
/// of each comes back with all of them, and the file nothing edited keeps its
/// bytes, hand-written comments included.
#[test]
fn a_project_round_trip_keeps_every_content_kind() {
    let _disk = disk();
    let mut app = project_app();
    new_mod(&mut app, "editor_kept");
    let kept = one_of_every_other_kind();
    let bytes = format!(
        "// written by hand\n{}",
        serialize_content(&kept).expect("the items serialize")
    );
    {
        let mut project = app.world_mut().resource_mut::<ModProject>();
        project.edit().files.push(ContentFile {
            path: "packs/kept.content.ron".to_string(),
            items: kept.into_iter().map(Arc::new).collect(),
        });
    }
    let before = written(&data(&app));
    write_project(app.world().resource::<ModProject>()).expect("the mod writes");
    mod_cache::install_local(
        "editor_kept",
        "0.1.0",
        "editor_kept.bundle.ron",
        &[(
            "packs/kept.content.ron".to_string(),
            bytes.clone().into_bytes(),
        )],
    )
    .expect("the hand edit lands");

    let project = read_project("editor_kept").expect("the mod reads back");
    assert_eq!(written(&project.data), before, "nothing was dropped");
    assert_eq!(
        project.data.kind_counts().len(),
        KINDS.len(),
        "all twelve kinds are held"
    );

    open(&mut app, project);
    edit_name(&mut app, "Edited");
    save_project(app.world_mut());

    let saved = read_project("editor_kept").expect("the mod reads back");
    let open = saved.data.open.clone().expect("a scenario is open");
    assert_eq!(
        saved.data.scenario(&open.id).expect("it stands").name,
        "Edited"
    );
    assert_eq!(
        read_mod_file("editor_kept", "packs/kept.content.ron"),
        Some(bytes.into_bytes()),
        "a save leaves a file nothing touched alone"
    );
}

/// `editor_` bundles are a shipped format: one scenario under the mod's own id,
/// designs bare. One opens as a one-scenario mod and resaves with namespaced
/// designs, under the same scenario id, so a campaign pointing at it still works.
#[test]
fn an_old_editor_bundle_opens_and_resaves() {
    let _disk = disk();
    let mut app = project_app();
    lower_open_document(app.world_mut());
    let sid = open_id(&app);
    let text = written(&data(&app))
        .into_values()
        .next()
        .expect("the stock range has a file")
        .replace(&format!("{sid}."), "")
        .replace(&sid, "editor_old");
    let old_ships: Vec<String> = parse_content(text.as_bytes())
        .expect("the fixture parses")
        .iter()
        .filter(|item| item.kind() == "ship")
        .map(|item| item.id().to_string())
        .collect();
    assert!(
        !old_ships.is_empty() && old_ships.iter().all(|id| !id.contains('.')),
        "the fixture is the old layout: {old_ships:?}"
    );
    let path = content_file("editor_old");
    let mut old = (*data(&app)).clone();
    old.files.truncate(1);
    old.files[0].path.clone_from(&path);
    let manifest = serialize_manifest(&manifest_of(&old)).expect("the manifest writes");
    mod_cache::install_local(
        "editor_old",
        "0.1.0",
        "editor_old.bundle.ron",
        &[
            ("editor_old.bundle.ron".to_string(), manifest.into_bytes()),
            (path, text.into_bytes()),
        ],
    )
    .expect("the old save lands");

    let project = read_project("editor_old").expect("an old save opens");
    assert_eq!(project.data.scenarios().count(), 1);
    open(&mut app, project);
    let ships: Vec<String> = app
        .world_mut()
        .query_filtered::<&NodeId, With<ShipNode>>()
        .iter(app.world())
        .map(|id| id.0.clone())
        .collect();
    assert!(
        old_ships.iter().all(|id| ships.contains(id)),
        "{old_ships:?} in {ships:?}"
    );

    edit_name(&mut app, "Resaved");
    save_project(app.world_mut());

    let saved = read_project("editor_old").expect("the resave reads back");
    assert_eq!(
        saved
            .data
            .scenario("editor_old")
            .expect("the id stands")
            .name,
        "Resaved"
    );
    let designs: Vec<&str> = saved
        .data
        .items()
        .filter(|item| item.kind() == "ship")
        .map(Content::id)
        .collect();
    assert!(!designs.is_empty());
    assert!(
        designs.iter().all(|id| id.starts_with("editor_old.")),
        "no bare design is left to collide: {designs:?}"
    );
}

/// Moving to another asset lowers the open one first, so nothing typed into it
/// is lost, and it comes back as it was left.
#[test]
fn switching_asset_keeps_edits() {
    let mut app = project_app();
    let first = open_id(&app);
    edit_name(&mut app, "Edited First");

    add_scenario(app.world_mut());
    app.update();
    let second = open_id(&app);
    assert_ne!(first, second);
    edit_name(&mut app, "Edited Second");

    open_asset(app.world_mut(), SCENARIO_KIND, &first);
    app.update();

    assert_eq!(document_name(&app), "Edited First");
    assert_eq!(
        data(&app).scenario(&second).expect("it stands").name,
        "Edited Second",
        "the one left behind was lowered"
    );
}

/// A switch is not a step, and a step taken back after one walks back into the
/// asset it was made in.
#[test]
fn undo_after_a_switch_returns_to_the_asset_of_the_step() {
    let mut app = project_app();
    let first = open_id(&app);
    add_scenario(app.world_mut());
    app.update();
    let second = open_id(&app);
    open_asset(app.world_mut(), SCENARIO_KIND, &first);
    app.update();
    edit_name(&mut app, "Edited First");
    let steps = depth(&app);

    open_asset(app.world_mut(), SCENARIO_KIND, &second);
    app.update();
    assert_eq!(depth(&app), steps, "opening is not a step");

    undo(&mut app);

    assert_eq!(open_id(&app), first, "the step's asset is open again");
    assert_ne!(document_name(&app), "Edited First");
}

/// A ship tile opens the ship alone at identity: no other node of its scenario
/// stands. Back returns to the scenario with its view and selection. Neither
/// is a step, and neither marks anything.
#[test]
fn a_ship_tile_opens_the_ship_alone_and_back_returns() {
    let mut app = project_app();
    app.init_resource::<EditorViewType>();
    let range = open_id(&app);
    lower_open_document(app.world_mut());
    let design = data(&app)
        .items()
        .find(|item| item.kind() == "ship")
        .map(|item| item.id().to_string())
        .expect("the range places a ship");
    let rock = first_with::<crate::node::ObjectNode>(&mut app);
    let rock_id = app.world().get::<NodeId>(rock).cloned();
    app.world_mut()
        .resource_mut::<crate::config::SelectedNode>()
        .set(Some(rock));
    *app.world_mut().resource_mut::<EditorViewType>() = EditorViewType::Events;
    let steps = depth(&app);
    let dirty = is_dirty(&app);

    open_asset(app.world_mut(), "ship", &design);
    app.update();

    assert_eq!(open_id(&app), design, "the ship is open");
    let ship = app
        .world()
        .resource::<EditContext>()
        .ship()
        .expect("the ship stands entered");
    let node = app.world().get::<NodeId>(ship).expect("it has an id");
    assert_eq!(design, format!("{range}.{}", node.0));
    let others = app
        .world_mut()
        .query_filtered::<Entity, Or<(With<ShipNode>, With<crate::node::ObjectNode>)>>()
        .iter(app.world())
        .filter(|other| *other != ship)
        .count();
    assert_eq!(others, 0, "no other node of the scenario stands");
    assert_eq!(
        *app.world().resource::<EditorViewType>(),
        EditorViewType::Ship
    );
    assert_eq!(depth(&app), steps, "opening is not a step");
    assert_eq!(is_dirty(&app), dirty, "and marks nothing");

    app.world_mut().commands().queue(super::verbs::go_back);
    app.update();

    assert_eq!(open_id(&app), range, "Back opens the scenario again");
    assert_eq!(app.world().resource::<EditContext>().ship(), None);
    assert_eq!(
        *app.world().resource::<EditorViewType>(),
        EditorViewType::Events,
        "with the view it was left on"
    );
    let marked = app
        .world()
        .resource::<crate::config::SelectedNode>()
        .primary()
        .and_then(|node| app.world().get::<NodeId>(node).cloned());
    assert_eq!(marked, rock_id, "and the selection");
    assert_eq!(depth(&app), steps, "Back is not a step");
    assert_eq!(is_dirty(&app), dirty, "and marks nothing");
}

/// An edit in an ad-hoc ship's document is one step, and Back finds it in the
/// scenario: the ship's design is the scenario's.
#[test]
fn an_edit_in_a_ship_document_writes_back_to_its_scenario() {
    let mut app = project_app();
    app.init_resource::<EditorViewType>();
    lower_open_document(app.world_mut());
    let design = data(&app)
        .items()
        .find(|item| item.kind() == "ship")
        .map(|item| item.id().to_string())
        .expect("the range places a ship");
    open_asset(app.world_mut(), "ship", &design);
    app.update();
    let ship = app
        .world()
        .resource::<EditContext>()
        .ship()
        .expect("the ship is open");
    let steps = depth(&app);

    app.world_mut()
        .get_mut::<ShipNode>(ship)
        .expect("a ship")
        .name = "Renamed Hull".to_string();
    commit_edit_now(app.world_mut(), "Edit Name");
    app.update();
    assert_eq!(depth(&app), steps + 1, "one edit, one step");

    app.world_mut().commands().queue(super::verbs::go_back);
    app.update();
    let named = app
        .world_mut()
        .query::<&ShipNode>()
        .iter(app.world())
        .any(|ship| ship.name == "Renamed Hull");
    assert!(named, "the scenario's ship carries the edit");

    undo(&mut app);
    let named = app
        .world_mut()
        .query::<&ShipNode>()
        .iter(app.world())
        .any(|ship| ship.name == "Renamed Hull");
    assert!(!named, "and one undo takes it back");
}

/// A shipped ship design, opened on its own and lowered with no edit, is the
/// item it was: the document drops nothing it cannot show.
#[test]
fn an_unedited_ship_design_writes_back_unchanged() {
    let mut app = project_app();
    app.init_resource::<EditorViewType>();
    let design = one_of_every_other_kind()
        .into_iter()
        .find(|item| item.kind() == "ship")
        .expect("the shipped content holds a ship design");
    let id = design.id().to_string();
    app.world_mut()
        .resource_mut::<ModProject>()
        .edit()
        .add_item(design.clone(), "kept.content.ron");

    open_asset(app.world_mut(), "ship", &id);
    app.update();
    assert_eq!(open_id(&app), id, "the design is open");
    super::verbs::lower_open_document(app.world_mut());

    let held = data(&app)
        .items()
        .find(|item| item.kind() == "ship" && item.id() == id)
        .map(|item| format!("{item:?}"));
    assert_eq!(held, Some(format!("{design:?}")));
}

/// A design placed in a scenario is one step, names the design by reference,
/// and stays a reference after the scenario is lowered and lifted again. A
/// double click on it opens the design.
#[test]
fn a_placed_design_stays_a_reference_to_the_design() {
    let mut app = project_app();
    app.init_resource::<EditorViewType>();
    let range = open_id(&app);
    let design = one_of_every_other_kind()
        .into_iter()
        .find(|item| item.kind() == "ship")
        .expect("the shipped content holds a ship design");
    let id = design.id().to_string();
    app.world_mut()
        .resource_mut::<ModProject>()
        .edit()
        .add_item(design, "kept.content.ron");
    let steps = depth(&app);

    crate::placement::place_design(app.world_mut(), &id, Some(Vec3::new(40.0, 0.0, 0.0)));
    app.update();
    assert_eq!(depth(&app), steps + 1, "a placement is one step");

    let placed = |app: &mut App| {
        app.world_mut()
            .query::<(Entity, &crate::node::ObjectNode)>()
            .iter(app.world())
            .find(|(_, object)| super::ship::placed_design(object) == Some(id.as_str()))
            .map(|(node, _)| node)
    };
    let node = placed(&mut app).expect("the scenario holds the placement");

    // Open the design through the placement, then come back: the scenario is
    // lowered on the way out and lifted on the way in.
    super::ship::open_ship_node(app.world_mut(), node, None);
    app.update();
    assert_eq!(open_id(&app), id, "the placement opens its design");
    app.world_mut().commands().queue(super::verbs::go_back);
    app.update();
    assert_eq!(open_id(&app), range);
    assert!(
        placed(&mut app).is_some(),
        "the lifted scenario still names the design"
    );
    assert!(
        data(&app)
            .item("ship", &format!("{range}.{}", id))
            .is_none(),
        "and wrote no copy of it"
    );
}

fn is_dirty(app: &App) -> bool {
    app.world().resource::<EditHistory>().dirty()
}

/// A save clears the unsaved mark whichever way the document reached this state.
#[test]
fn a_save_clears_the_unsaved_mark() {
    let _disk = disk();
    let mut app = project_app();
    new_mod(&mut app, "editor_marks");
    edit_name(&mut app, "One");
    assert!(is_dirty(&app));
    save_project(app.world_mut());
    assert!(!is_dirty(&app), "after an edit");

    edit_name(&mut app, "Two");
    edit_name(&mut app, "Three");
    undo(&mut app);
    assert!(is_dirty(&app));
    save_project(app.world_mut());
    assert!(!is_dirty(&app), "after an undo");
}

/// The first save of a mod with no id clears the mark too, on the seeded range.
#[test]
fn naming_a_scratch_mod_clears_the_unsaved_mark() {
    let _disk = disk();
    let mut app = project_app();
    open(
        &mut app,
        ModProject::new(None, "Scratch Mod", "", ScenarioTemplate::default()),
    );
    edit_name(&mut app, "One");
    edit_name(&mut app, "Two");
    undo(&mut app);
    assert!(is_dirty(&app));

    name_project(app.world_mut(), "Scratch Mod");

    assert!(!is_dirty(&app));
}

/// Deleting a scenario is undoable: Ctrl+Z restores it equal, ships and all, and
/// redo deletes it again.
#[test]
fn delete_scenario_then_undo_restores_it_equal() {
    let mut app = project_app();
    let first = open_id(&app);
    add_scenario(app.world_mut());
    app.update();
    lower_open_document(app.world_mut());
    let before = written(&data(&app));

    delete_scenario(app.world_mut(), &first);
    app.update();
    assert!(data(&app).scenario(&first).is_none());

    undo(&mut app);
    assert_eq!(written(&data(&app)), before);

    redo(&mut app);
    assert!(data(&app).scenario(&first).is_none());
}

/// A copy owns its designs, or the two would share a hull through one id.
#[test]
fn a_duplicated_scenario_owns_its_designs() {
    let mut app = project_app();
    lower_open_document(app.world_mut());
    let first = open_id(&app);

    duplicate_scenario(app.world_mut(), &first);
    app.update();
    lower_open_document(app.world_mut());

    let held = data(&app);
    assert_eq!(held.scenarios().count(), 2);
    let copy = held
        .scenarios()
        .map(|scenario| scenario.id.clone())
        .find(|id| *id != first)
        .expect("the copy has its own id");
    let designs: Vec<&str> = held
        .items()
        .filter(|item| item.kind() == "ship")
        .map(Content::id)
        .collect();
    assert_eq!(
        designs.len(),
        designs
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        "no two designs share an id: {designs:?}"
    );
    assert!(designs.iter().any(|id| id.starts_with(&format!("{copy}."))));
}

/// A rename is the display name: the id other content points at stays.
#[test]
fn renaming_a_scenario_keeps_its_id() {
    let mut app = project_app();
    let first = open_id(&app);

    rename_scenario(app.world_mut(), &first, "Second Wind");
    app.update();

    assert_eq!(
        data(&app).scenario(&first).expect("it stands").name,
        "Second Wind"
    );
    assert_eq!(
        document_name(&app),
        "Second Wind",
        "the open root says so too"
    );
    undo(&mut app);
    assert_ne!(document_name(&app), "Second Wind");
}

/// Set the open scenario's cubemap, as a picked asset does.
fn point_at(app: &mut App, file: &str) {
    let root = app
        .world()
        .resource::<EditContext>()
        .scenario()
        .expect("a document stands");
    app.world_mut()
        .get_mut::<ScenarioNode>(root)
        .expect("the root is a scenario")
        .cubemap = AssetRef::from(file);
    commit_edit_now(app.world_mut(), "Pick Cubemap");
    app.update();
}

/// One bundle, enabled, the way [`crate::asset_index`]'s rig loads one.
pub(crate) fn load_bundle(app: &mut App, id: &str, resources: &[&str]) {
    let world = app.world_mut();
    let mut bundles = Assets::<BundleAsset>::default();
    let bundle = bundles.add(BundleAsset {
        content: vec![],
        meta: ModMeta::default(),
        new_game_scenario: None,
        resources: resources.iter().map(|file| (*file).to_string()).collect(),
        resource_base: format!("mods/{id}"),
    });
    let mut catalogs = Assets::<InstalledCatalog>::default();
    catalogs.add(InstalledCatalog {
        entries: vec![nova_modding::prelude::CatalogEntry {
            decl: nova_modding::prelude::ModEntry {
                id: id.to_string(),
                bundle: format!("mods/{id}/{id}.bundle.ron"),
                base: true,
            },
            bundle: Some(bundle),
        }],
    });
    world.insert_resource(bundles);
    world.insert_resource(catalogs);
    world.insert_resource(EnabledMods([id.to_string()].into_iter().collect()));
}

/// A `dep://` pick from a mod the project does not declare passed Play and then
/// failed the merge gate after saving. The pick now declares it in its own step,
/// so one Ctrl+Z takes back both.
#[test]
fn an_undeclared_pick_declares_the_dependency_in_the_same_step() {
    let mut app = project_app();
    load_bundle(&mut app, "art-pack", &["textures/sky.png"]);
    let steps = depth(&app);

    point_at(&mut app, "dep://art-pack/textures/sky.png");

    assert_eq!(depth(&app), steps + 1, "one step");
    assert_eq!(data(&app).meta.dependencies, ["art-pack"]);
    assert_eq!(play_faults(app.world_mut()), Vec::<String>::new());

    undo(&mut app);
    assert!(data(&app).meta.dependencies.is_empty());
}

/// Taking a dependency away from a mod that reaches into it is refused, and the
/// refusal says who, so the merge gate never meets it.
#[test]
fn a_dependency_remove_is_refused_while_referenced() {
    let mut app = project_app();
    load_bundle(&mut app, "art-pack", &["textures/sky.png"]);
    point_at(&mut app, "dep://art-pack/textures/sky.png");
    let sid = open_id(&app);

    let refused = remove_dependency(app.world_mut(), "art-pack");

    assert_eq!(refused, [format!("scenario {sid}")]);
    assert_eq!(data(&app).meta.dependencies, ["art-pack"]);

    point_at(&mut app, "dep://base/textures/cubemap.png");
    let refused = remove_dependency(app.world_mut(), "art-pack");
    assert!(refused.is_empty());
    assert!(data(&app).meta.dependencies.is_empty());
}

/// Add and remove are steps on the one stack.
#[test]
fn a_dependency_add_is_one_undoable_step() {
    let mut app = project_app();
    load_bundle(&mut app, "art-pack", &["textures/sky.png"]);
    let steps = depth(&app);

    add_dependency(app.world_mut(), "art-pack");
    app.update();

    assert_eq!(depth(&app), steps + 1);
    assert_eq!(
        app.world().resource::<EditHistory>().undo_label(),
        Some("Add Dependency")
    );
    undo(&mut app);
    assert!(data(&app).meta.dependencies.is_empty());
}

/// A mod that is neither loaded nor installed is refused by name, and the
/// manifest keeps what it held.
#[test]
fn a_dependency_on_a_missing_mod_is_refused() {
    let mut app = project_app();
    let steps = depth(&app);

    add_dependency(app.world_mut(), "never-installed");
    app.update();

    assert_eq!(depth(&app), steps, "no step");
    assert!(data(&app).meta.dependencies.is_empty());
    let line = app
        .world()
        .resource::<crate::config::EditorStatus>()
        .line()
        .map(|(line, _)| line.to_string());
    assert_eq!(line.as_deref(), Some("never-installed is not installed"));
}

/// A cycle is refused by id, before a merge could ever meet it.
#[test]
fn a_dependency_cycle_is_refused() {
    let graph = [
        ("a".to_string(), vec!["b".to_string()]),
        ("b".to_string(), vec!["c".to_string()]),
        ("c".to_string(), vec![]),
    ]
    .into_iter()
    .collect();

    assert!(verbs::closes_cycle(&graph, "c", "a"), "c -> a -> b -> c");
    assert!(
        verbs::closes_cycle(&graph, "a", "a"),
        "a mod needs no itself"
    );
    assert!(!verbs::closes_cycle(&graph, "a", "c"));
}

/// A merge refusal while Playing is fatal, so a ref the merge would refuse stops
/// at Play.
#[test]
fn play_refuses_a_ref_the_merge_would_refuse() {
    let mut app = project_app();
    point_at(&mut app, "dep://never-installed/textures/sky.png");
    lower_open_document(app.world_mut());

    let faults = play_faults(app.world_mut());

    assert_eq!(faults.len(), 1, "{faults:?}");
    assert!(faults[0].contains("never-installed"), "{faults:?}");
}

/// A missing dependency is a fault, and a Package error rather than a Save one.
#[test]
fn a_missing_dependency_blocks_package() {
    let mut held = (*data(&project_app())).clone();
    held.meta.dependencies = vec!["never-installed".to_string()];

    let found = manifest_findings("editor_x", &held, |_| false, |_| false, |_| true);

    assert!(found.iter().any(|problem| {
        problem.severity == ProblemSeverityType::Error
            && problem.message.contains("never-installed")
    }));
}

/// A published id may not take the place of a mod the game ships.
#[test]
fn a_package_refuses_a_shipped_id() {
    let held = ProjectData {
        meta: ModMeta {
            name: "Base".to_string(),
            version: "1.0.0".to_string(),
            ..default()
        },
        resources: vec![],
        new_game_scenario: None,
        files: vec![],
        open: None,
    };

    let found = manifest_findings("editor_base", &held, |id| id == "base", |_| true, |_| true);

    assert!(found
        .iter()
        .any(|problem| problem.severity == ProblemSeverityType::Error));
}

/// A zip of `(path, bytes)`, written without [`zip_bytes`] so a test can make an
/// archive the game would not.
fn crafted(entries: &[(&str, &[u8])]) -> Vec<u8> {
    use std::io::{Cursor, Write};

    use zip::{write::SimpleFileOptions, ZipWriter};
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in entries {
        writer
            .start_file(*path, SimpleFileOptions::default())
            .expect("an entry starts");
        writer.write_all(bytes).expect("an entry writes");
    }
    writer.finish().expect("the zip closes").into_inner()
}

/// An archive names its own paths, so a `../` entry would write outside the mod.
#[test]
fn import_refuses_a_parent_entry() {
    let archive = crafted(&[("../evil.bundle.ron", b"()")]);

    let refused = unzip_bytes(&archive).expect_err("the entry is refused");

    assert!(refused.contains("evil"), "{refused}");
}

/// The same for an absolute path.
#[test]
fn import_refuses_an_absolute_entry() {
    let archive = crafted(&[("/etc/evil.bundle.ron", b"()")]);

    assert!(unzip_bytes(&archive).is_err());
}

/// A package with no manifest is not a mod.
#[test]
fn import_refuses_a_package_with_no_manifest() {
    let files = unzip_bytes(&crafted(&[("readme.txt", b"hello")])).expect("the entry is safe");

    let refused = adopt_package(files, |_| false, |_| false).expect_err("no manifest");

    assert!(refused.contains("manifest"), "{refused}");
}

/// A package reads back as the local mod it was made from, under `editor_`.
#[test]
fn a_package_adopts_back_to_its_local_id() {
    let mut app = project_app();
    lower_open_document(app.world_mut());
    let held = data(&app);

    let files = package_files("editor_red_run", &held, vec![]).expect("it packages");
    let archive = zip_bytes(&files).expect("it zips");
    let adopted = adopt_package(
        unzip_bytes(&archive).expect("it unzips"),
        |_| false,
        |_| false,
    )
    .expect("it adopts");

    assert_eq!(files[0].0, "red-run.bundle.ron", "the portal takes this id");
    assert_eq!(adopted.id, "editor_red-run");
    assert_eq!(adopted.bundle, "editor_red-run.bundle.ron");
}

/// Two local mods packaged together stay joined: a dependency publishes as the
/// portal id and comes home when the local mod is here.
#[test]
fn a_local_dependency_survives_a_package_and_an_import() {
    let mut app = project_app();
    point_at(&mut app, "dep://editor_art/textures/sky.png");
    lower_open_document(app.world_mut());
    app.world_mut()
        .resource_mut::<ModProject>()
        .edit()
        .meta
        .dependencies = vec!["editor_art".to_string()];
    let held = data(&app);

    let files = package_files("editor_red_run", &held, vec![]).expect("it packages");
    let text: String = files
        .iter()
        .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
        .collect();
    assert!(
        text.contains("dep://art/textures/sky.png"),
        "the ref publishes"
    );
    assert!(
        !text.contains("editor_art"),
        "no local id leaves the machine"
    );

    let adopted = adopt_package(files, |id| id == "editor_art", |_| false).expect("it adopts");
    let text: String = adopted
        .files
        .iter()
        .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
        .collect();
    assert!(
        text.contains("dep://editor_art/textures/sky.png"),
        "the ref comes home"
    );
}

/// Pump until `handle` and everything under it is loaded.
fn wait_loaded(app: &mut App, server: &AssetServer, handle: UntypedAssetId, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        app.update();
        match server.get_recursive_dependency_load_state(handle) {
            Some(RecursiveDependencyLoadState::Loaded) => return,
            Some(RecursiveDependencyLoadState::Failed(error)) => {
                panic!("{what} failed to load: {error}")
            }
            _ => {}
        }
        assert!(Instant::now() < deadline, "timed out loading {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The production merge over the installed cache, with `enabled` on: the rig
/// `nova_assets`' install test proves, with no game around it.
pub(crate) fn merged(enabled: &[&str]) -> App {
    let mut app = App::new();
    mod_cache::register_mods_source(&mut app);
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: "../../assets".to_string(),
            ..default()
        },
    ));
    app.add_plugins(NovaModdingPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<GameStates>();
    app.init_resource::<EnabledMods>();
    app.init_resource::<DownloadedMods>();
    app.init_resource::<ModCatalog>();
    app.init_resource::<OptionalBundles>();
    app.init_resource::<ModQuarantine>();
    app.add_systems(Update, nova_assets::mark_installed_bundles_loaded);
    app.add_systems(
        Update,
        nova_assets::register_bundles_for_test
            .run_if(resource_exists::<GameAssets>)
            .run_if(nova_assets::installed_set_changed),
    );
    app.world_mut()
        .run_system_once(nova_assets::load_downloaded_mods)
        .expect("the cache index reads");
    let server = app.world().resource::<AssetServer>().clone();
    let catalog: Handle<InstalledCatalog> = server.load("mods.catalog.ron");
    wait_loaded(&mut app, &server, catalog.id().untyped(), "the catalog");
    let bundles: Vec<UntypedAssetId> = app
        .world()
        .resource::<DownloadedMods>()
        .0
        .iter()
        .map(|held| held.bundle.id().untyped())
        .collect();
    for bundle in bundles {
        wait_loaded(&mut app, &server, bundle, "a downloaded bundle");
    }
    app.world_mut().insert_resource(GameAssets {
        cubemap: Handle::default(),
        asteroid_texture: Handle::default(),
        hull_01: Handle::default(),
        turret_yaw_01: Handle::default(),
        turret_pitch_01: Handle::default(),
        turret_barrel_01: Handle::default(),
        torpedo_bay_01: Handle::default(),
        fps_icon: Handle::default(),
        target_sprite: Handle::default(),
        nova_crt_mark: Handle::default(),
        ui_sfx: Default::default(),
        key_glyphs: Default::default(),
        catalog,
    });
    app.world_mut().insert_resource(EnabledMods(
        std::iter::once("base")
            .chain(enabled.iter().copied())
            .map(str::to_string)
            .collect(),
    ));
    app.update();
    app
}

/// The whole point of Package: the zip, extracted, is a mod the production merge
/// takes as it stands, every scenario and design with it, and nothing refused.
#[test]
fn a_package_zip_extracted_merges_through_register_bundles() {
    let _disk = disk();
    let mut app = project_app();
    new_mod(&mut app, "editor_red_run");
    add_scenario(app.world_mut());
    app.update();
    lower_open_document(app.world_mut());
    let held = data(&app);
    let scenarios: Vec<String> = held.scenarios().map(|item| item.id.clone()).collect();
    let designs: Vec<String> = held
        .items()
        .filter(|item| item.kind() == "ship")
        .map(|item| item.id().to_string())
        .collect();
    assert_eq!(scenarios.len(), 2);
    assert!(!designs.is_empty());

    let files = package_files("editor_red_run", &held, vec![]).expect("it packages");
    let archive = zip_bytes(&files).expect("it zips");
    let extracted = unzip_bytes(&archive).expect("it unzips");
    mod_cache::prelude::remove_mod("editor_red_run").expect("the local copy goes");
    mod_cache::prelude::remove_index_record("editor_red_run");
    mod_cache::install_local("red-run", "0.1.0", "red-run.bundle.ron", &extracted)
        .expect("the extracted files install");

    let game = merged(&["red-run"]);

    let quarantine = game.world().resource::<ModQuarantine>();
    assert!(
        quarantine.disabled.is_empty(),
        "the merge refused it: {:?}",
        quarantine.disabled
    );
    let held = game.world().resource::<GameScenarios>();
    for id in &scenarios {
        assert!(held.contains_key(id.as_str()), "{id} merged");
    }
    let ships = game.world().resource::<GameShipDesigns>();
    for id in &designs {
        assert!(
            ships.0.iter().any(|design| design.id.as_str() == id),
            "{id} merged"
        );
    }
}

/// The stock range as the one-scenario layout saved it, under `editor_old`.
/// `bare` leaves its designs without the scenario in front, as the old layout
/// wrote them; otherwise they stand as a resave of that layout left them.
fn old_layout(app: &mut App, bare: bool) -> ProjectData {
    lower_open_document(app.world_mut());
    let sid = open_id(app);
    let mut text = written(&data(app))
        .into_values()
        .next()
        .expect("the stock range has a file");
    if bare {
        text = text.replace(&format!("{sid}."), "");
    }
    let items = parse_content(text.replace(&sid, "editor_old").as_bytes()).expect("it parses");
    let mut old = (*data(app)).clone();
    old.open = None;
    old.files = vec![ContentFile {
        path: content_file("editor_old"),
        items: items.into_iter().map(Arc::new).collect(),
    }];
    old
}

/// A spawn that stands as authored may name a design no node writes any more:
/// the design stays while the spawn needs it.
#[test]
fn a_design_a_kept_spawn_names_outlives_its_node() {
    let mut app = project_app();
    lower_open_document(app.world_mut());
    let sid = open_id(&app);
    let mut held = (*data(&app)).clone();
    let scenario = held.scenario(&sid).expect("it stands").clone();
    let designs = prototype_refs(&Content::Scenario(scenario.clone()));
    assert!(!designs.is_empty(), "the stock range spawns its designs");

    held.absorb(&sid, vec![Content::Scenario(scenario)], false, "unused");

    for design in designs {
        assert!(held.item("ship", &design).is_some(), "{design} stands");
    }
}

/// How many times `id` stands as a whole string in `item`.
fn times_named(item: &Content, id: &str) -> usize {
    string_leaves(item)
        .iter()
        .filter(|leaf| *leaf == id)
        .count()
}

/// The old layout gave a design and its spawn one id. A copy moves the design:
/// the spawn and the handlers that name it keep theirs, or a reopened copy
/// answers to neither and the handlers go.
#[test]
fn a_copy_of_an_old_save_keeps_its_spawn_ids_and_its_handlers() {
    let _disk = disk();
    let mut app = project_app();
    let old = old_layout(&mut app, true);
    let held = Content::Scenario(old.scenario("editor_old").expect("it stands").clone());
    let ships: Vec<String> = old
        .items()
        .filter(|item| item.kind() == "ship")
        .map(|item| item.id().to_string())
        .collect();
    assert!(!ships.is_empty());

    let copy = renamed_copy(&old, "editor_old", "copied", "Copied").expect("it copies");

    let scenario = copy.scenarios().next().expect("the scenario came").clone();
    let to = scenario.id.clone();
    let events = scenario.events.len();
    let moved = Content::Scenario(scenario);
    let refs = prototype_refs(&moved);
    for ship in &ships {
        let design = format!("{to}.{ship}");
        assert!(
            copy.item("ship", &design).is_some(),
            "{design} is the design"
        );
        assert!(refs.contains(&design), "{design} is what the spawn names");
        let spawned = prototype_refs(&held)
            .iter()
            .filter(|id| *id == ship)
            .count();
        assert_eq!(
            times_named(&moved, ship),
            times_named(&held, ship) - spawned,
            "every other mention of {ship} stands"
        );
    }

    write_copy("editor_copied", &copy, "editor_copied").expect("it writes");
    open(&mut app, read_project("editor_copied").expect("it reads"));
    lower_open_document(app.world_mut());
    let lowered = data(&app);
    let reopened = lowered.scenario(&to).expect("the copy opens");
    assert_eq!(reopened.events.len(), events, "no handler was dropped");
}

/// A resaved old layout still answers to [`is_old_save`], and its designs are
/// not bare any more: they move once.
#[test]
fn a_copy_of_a_resaved_old_layout_moves_its_designs_once() {
    let mut app = project_app();
    let old = old_layout(&mut app, false);
    assert!(old.items().any(|item| item.id().starts_with("editor_old.")));

    let copy = renamed_copy(&old, "editor_old", "copied", "Copied").expect("it copies");

    let to = copy.scenarios().next().expect("it came").id.clone();
    let designs: Vec<&str> = copy
        .items()
        .filter(|item| item.kind() == "ship")
        .map(Content::id)
        .collect();
    assert!(!designs.is_empty());
    assert!(
        designs.iter().all(|id| {
            id.strip_prefix(&format!("{to}."))
                .is_some_and(|node| !node.contains('.'))
        }),
        "{designs:?}"
    );
}

/// An unfinished block lowers to nothing, so a step that lowers and rebuilds
/// would take it away. The step stops instead.
#[test]
fn a_project_step_stops_on_an_unfinished_block() {
    let mut app = project_app();
    lower_open_document(app.world_mut());
    let hat = first_with::<EventNode>(&mut app);
    let unfinished = place_new(
        app.world_mut(),
        BlockItemType::Filter(FilterChoice::And),
        BlockSlotType::Filter {
            parent: hat,
            index: 0,
        },
    )
    .expect("a filter joins a handler");
    app.update();
    let held = data(&app).scenarios().count();

    add_scenario(app.world_mut());
    app.update();

    assert_eq!(data(&app).scenarios().count(), held, "the step did not run");
    assert!(app.world().get::<FilterNode>(unfinished).is_some());
}

/// The package [`write_project`] would make of the open mod, on disk at `dir`.
fn packaged(app: &App, id: &str, dir: &Path) -> String {
    let files = package_files(id, &data(app), vec![]).expect("it packages");
    let path = dir.join("package.zip");
    std::fs::write(&path, zip_bytes(&files).expect("it zips")).expect("it writes");
    path.display().to_string()
}

/// `a` and `a/b` cannot both be files, so this archive adopts and cannot land.
fn unlandable(app: &App, id: &str, dir: &Path) -> String {
    let mut files = package_files(id, &data(app), vec![]).expect("it packages");
    files.push(("a".to_string(), b"file".to_vec()));
    files.push(("a/b".to_string(), b"under a file".to_vec()));
    let path = dir.join("broken.zip");
    std::fs::write(&path, zip_bytes(&files).expect("it zips")).expect("it writes");
    path.display().to_string()
}

/// An overwrite that fails leaves the mod it was to replace as it stood.
#[test]
fn a_failed_import_over_keeps_the_old_mod() {
    let _disk = disk();
    let dir = tempfile::tempdir().expect("a scratch dir");
    let mut app = project_app();
    app.init_resource::<HubForm>();
    new_mod(&mut app, "editor_kept");
    save_project(app.world_mut());
    let before = written(&data(&app));
    let path = unlandable(&app, "editor_kept", dir.path());
    let _ = read_project("editor_kept").expect("it stands");

    import_over(app.world_mut(), &path);

    assert!(
        app.world().resource::<HubForm>().line.contains("failed"),
        "{}",
        app.world().resource::<HubForm>().line
    );
    let after = read_project("editor_kept").expect("the old mod stands");
    assert_eq!(written(&after.data), before);
}

/// A save that fails half way leaves a manifest naming what is there, so the mod
/// still opens.
#[test]
fn a_failed_save_keeps_the_old_manifest() {
    let _disk = disk();
    let mut app = project_app();
    new_mod(&mut app, "editor_torn");
    save_project(app.world_mut());
    let mut project = read_project("editor_torn").expect("it stands");
    let held = project.data.files.len();
    let extra = ContentFile {
        path: "blocked.content.ron".to_string(),
        items: vec![],
    };
    Arc::make_mut(&mut project.data).files.push(extra);
    mod_cache::prelude::store_mod_files(
        "editor_torn",
        &[("blocked.content.ron/held".to_string(), b"x".to_vec())],
    )
    .expect("a directory stands where the file would go");

    write_project(&project).expect_err("the content file cannot land");

    let after = read_project("editor_torn").expect("the mod still opens");
    assert_eq!(after.data.files.len(), held);
}

/// What is in the folder and not in the manifest is no part of the mod: an
/// import undone leaves its file behind.
#[test]
fn a_package_and_a_copy_leave_out_files_the_manifest_does_not_name() {
    let _disk = disk();
    let mut app = project_app();
    new_mod(&mut app, "editor_tidy");
    {
        let mut project = app.world_mut().resource_mut::<ModProject>();
        let held = project.edit();
        held.resources = vec!["mesh.glb".to_string()];
        held.meta.icon = Some("icon.png".to_string());
    }
    save_project(app.world_mut());
    mod_cache::prelude::store_mod_files(
        "editor_tidy",
        &[
            ("mesh.glb".to_string(), b"mesh".to_vec()),
            ("mesh.glb.meta".to_string(), b"sidecar".to_vec()),
            ("icon.png".to_string(), b"icon".to_vec()),
            ("stray.glb".to_string(), b"orphan".to_vec()),
        ],
    )
    .expect("the files land");

    let mut packed: Vec<String> = other_files("editor_tidy", &data(&app))
        .expect("the files read")
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    packed.sort();
    assert_eq!(packed, ["icon.png", "mesh.glb", "mesh.glb.meta"]);

    let copy = renamed_copy(&data(&app), "editor_tidy", "neat", "Neat").expect("it copies");
    write_copy("editor_neat", &copy, "editor_tidy").expect("it writes");
    assert_eq!(read_mod_file("editor_neat", "stray.glb"), None);
    assert!(read_mod_file("editor_neat", "mesh.glb").is_some());
    assert!(read_mod_file("editor_neat", "icon.png").is_some());
}

/// `editor_a_b` publishes as `a-b`, which imports as `editor_a-b`: two folders,
/// one namespace. The import is the same mod coming home.
#[test]
fn an_import_lands_on_the_mod_of_the_same_published_id() {
    let _disk = disk();
    let dir = tempfile::tempdir().expect("a scratch dir");
    let mut app = project_app();
    app.init_resource::<HubForm>();
    new_mod(&mut app, "editor_a_b");
    save_project(app.world_mut());
    let path = packaged(&app, "editor_a_b", dir.path());

    import_package(app.world_mut(), &path);
    assert_eq!(local_ids(), ["editor_a_b"], "it asks before it writes");

    import_over(app.world_mut(), &path);
    assert_eq!(local_ids(), ["editor_a_b"]);
    assert!(read_project("editor_a_b").is_ok());
}

/// `base` is the game: no import may take the name.
#[test]
fn a_package_named_for_the_game_is_refused() {
    let mut app = project_app();
    lower_open_document(app.world_mut());
    let files = package_files("editor_base", &data(&app), vec![]).expect("it packages");

    let refused = adopt_package(files, |_| false, |_| false).expect_err("the name is the game's");

    assert!(refused.contains("base"), "{refused}");
}

/// A ref into the game stays one, whatever a local mod is called.
#[test]
fn a_ref_into_the_game_stays_on_the_game_through_an_import() {
    let mut app = project_app();
    point_at(&mut app, "dep://base/textures/sky.png");
    lower_open_document(app.world_mut());
    let files = package_files("editor_red_run", &data(&app), vec![]).expect("it packages");

    let adopted = adopt_package(files, |id| id == "editor_base", |_| false).expect("it adopts");

    let text: String = adopted
        .files
        .iter()
        .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
        .collect();
    assert!(text.contains("dep://base/textures/sky.png"));
    assert!(!text.contains("editor_base"));
}

/// The mods written by hand that ship in this repository, as a project each.
fn hand_written_mods() -> Vec<ModProject> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    [
        "webmods/gauntlet",
        "webmods/the-ledger",
        "assets/mods/example",
    ]
    .map(|dir| root.join(dir))
    .iter()
    .map(|dir| {
        let bundle = std::fs::read_dir(dir)
            .expect("the mod stands")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .find(|name| name.ends_with(".bundle.ron"))
            .expect("the mod has a manifest");
        let manifest = nova_modding::prelude::parse_manifest(
            &std::fs::read(dir.join(&bundle)).expect("the manifest reads"),
        )
        .expect("the manifest parses");
        let data = store::project_of(manifest, |path| std::fs::read(dir.join(path)).ok())
            .expect("the content parses");
        let data = Arc::new(data);
        let id = bundle.trim_end_matches(".bundle.ron").to_string();
        ModProject {
            id: Some(id.clone()),
            namespace: id,
            bundle,
            on_disk: Some(Arc::clone(&data)),
            data,
            migrate: None,
            seed: None,
        }
    })
    .collect()
}

/// The lines of each file that changed, as `file: old -> new`. A file of another
/// length is one whole failure: this is for edits that replace a value.
fn lines_changed(
    before: &BTreeMap<String, String>,
    after: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut changed = Vec::new();
    for (file, old) in before {
        let new = &after[file];
        let (old, new): (Vec<&str>, Vec<&str>) = (old.lines().collect(), new.lines().collect());
        if old.len() != new.len() {
            let at = old.iter().zip(&new).position(|(a, b)| a != b);
            let first = at.map_or("the end", |at| old[at]);
            changed.push(format!(
                "{file}: {} lines became {}, first at {first:?}",
                old.len(),
                new.len()
            ));
            continue;
        }
        changed.extend(
            old.iter()
                .zip(&new)
                .filter(|(a, b)| a != b)
                .map(|(a, b)| format!("{file}: {a} -> {b}")),
        );
    }
    changed
}

/// Open scenario `id` of `project` on `app`, on a project as it stands on disk.
fn open_scenario(app: &mut App, project: &ModProject, id: &str) {
    let mut project = ModProject {
        data: Arc::clone(&project.data),
        on_disk: project.on_disk.clone(),
        id: project.id.clone(),
        namespace: project.namespace.clone(),
        bundle: project.bundle.clone(),
        migrate: None,
        seed: None,
    };
    project.edit().open = Some(OpenAsset::scenario(id.to_string()));
    open(app, project);
}

/// `project` with each scenario's spawns moved into a layout handler. The real
/// mods mix spawns with other actions in `OnStart`, so their spawns are script
/// blocks. This puts the same authored spawns where the stage owns them.
fn with_a_layout(project: &ModProject) -> ModProject {
    let mut data = (*project.data).clone();
    for item in data.files.iter_mut().flat_map(|file| &mut file.items) {
        let Content::Scenario(scenario) = &**item else {
            continue;
        };
        let mut scenario = scenario.clone();
        let mut spawns = Vec::new();
        for event in &mut scenario.events {
            if matches!(event.name, EventConfig::OnStart) && event.filters.is_empty() {
                let (moved, kept) =
                    std::mem::take(&mut event.actions)
                        .into_iter()
                        .partition(|action| {
                            matches!(action, EventActionConfig::SpawnScenarioObject(_))
                        });
                spawns.extend::<Vec<_>>(moved);
                event.actions = kept;
            }
        }
        scenario.events.retain(|event| !event.actions.is_empty());
        scenario.events.insert(
            0,
            ScenarioEventConfig {
                label: None,
                position: None,
                name: EventConfig::OnStart,
                once: false,
                filters: vec![],
                actions: spawns,
            },
        );
        *item = Arc::new(Content::Scenario(scenario));
    }
    let data = Arc::new(data);
    ModProject {
        id: project.id.clone(),
        namespace: project.namespace.clone(),
        bundle: project.bundle.clone(),
        on_disk: Some(Arc::clone(&data)),
        data,
        migrate: None,
        seed: None,
    }
}

/// A hand-written mod is the modder's file. Opening a scenario and making one
/// edit has to write that edit and nothing else: a spawn the editor has no node
/// for stays as authored, and so do its handlers and its designs.
#[test]
fn one_edit_to_a_hand_written_scenario_writes_only_that_edit() {
    let _disk = disk();
    let mut app = project_app();
    let mut scenarios = 0;
    let mods = hand_written_mods();
    let laid_out: Vec<ModProject> = mods.iter().map(with_a_layout).collect();
    for project in mods.iter().chain(&laid_out) {
        let before = written(&project.data);
        let ids: Vec<String> = project
            .data
            .scenarios()
            .map(|scenario| scenario.id.clone())
            .collect();
        for id in ids {
            scenarios += 1;

            open_scenario(&mut app, project, &id);
            lower_open_document(app.world_mut());
            assert_eq!(
                lines_changed(&before, &written(&data(&app))),
                Vec::<String>::new(),
                "{id}: no edit, no change"
            );

            open_scenario(&mut app, project, &id);
            let world = app.world_mut();
            let root = world
                .resource::<EditContext>()
                .scenario()
                .expect("it opened");
            world
                .get_mut::<ScenarioNode>(root)
                .expect("the root holds the settings")
                .description = "One edit".to_string();
            lower_open_document(world);
            let changed = lines_changed(&before, &written(&data(&app)));
            assert_eq!(changed.len(), 1, "{id}: the description only: {changed:#?}");
            assert!(changed[0].contains("One edit"), "{id}: {changed:#?}");
        }
    }
    assert!(scenarios > 5, "every mod brought its scenarios");
}

/// A spawn with no node of its own still stands on the stage and moves. The move
/// is the whole of what changes.
#[test]
fn moving_a_hand_written_spawn_writes_only_its_position() {
    let _disk = disk();
    let mut app = project_app();
    let (mut moved, mut hulls_moved) = (0, 0);
    for project in hand_written_mods().iter().map(with_a_layout) {
        let before = written(&project.data);
        let ids: Vec<String> = project
            .data
            .scenarios()
            .map(|scenario| scenario.id.clone())
            .collect();
        for id in ids {
            open_scenario(&mut app, &project, &id);
            let world = app.world_mut();
            let is_hull = |node: &crate::node::ObjectNode| {
                matches!(
                    node.kind,
                    ScenarioObjectKind::Spaceship(_) | ScenarioObjectKind::Station(_)
                )
            };
            let objects: Vec<(Entity, bool)> = world
                .query::<(Entity, &crate::node::ObjectNode)>()
                .iter(world)
                .map(|(entity, node)| (entity, is_hull(node)))
                .collect();
            let hull = objects.iter().find(|(_, hull)| *hull).or(objects.first());
            let Some(&(object, hull)) = hull else {
                continue;
            };
            moved += 1;
            hulls_moved += usize::from(hull);
            world
                .get_mut::<Transform>(object)
                .expect("an object stands somewhere")
                .translation
                .x += 1.5;
            lower_open_document(world);
            let changed = lines_changed(&before, &written(&data(&app)));
            assert_eq!(changed.len(), 1, "{id}: one coordinate: {changed:#?}");
        }
    }
    assert!(moved > 5, "every mod has objects to move, {moved} did");
    assert!(hulls_moved > 0, "a hull stands as authored and moves");
}
