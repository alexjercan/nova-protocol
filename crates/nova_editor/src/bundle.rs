//! What a saved document IS: a mod bundle, in the same format a hand-written
//! mod is authored in.
//!
//! The file is a `Vec<Content>`: one `Content::Ship` per design in the
//! document, then one `Content::Scenario` whose spaceships reference those
//! designs by id. Nothing is written twice - edit a design and every instance
//! of it changes, and "export my ship" is the file you already saved.
//!
//! THE LOWERING CONVENTION, both ways. The LAYOUT is derived: the unfiltered
//! `OnStart` handler that spawns the board is written out of the object nodes
//! and read back into them, so the tree and the spawn list can never disagree.
//! Every other handler is the SCRIPT, and the document holds it as nodes too
//! (see [`crate::event`]) - a save lowers them, a load lifts them back, and a
//! handler edited in the panel survives the file like a rock moved on the
//! stage.
//!
//! The one thing that does not come back as it went is an authored handler
//! that is INDISTINGUISHABLE from the layout: an unfiltered `OnStart` that
//! does nothing but spawn is read back as world objects, because that is what
//! it does. The range plays the same either way.
//!
//! An AI ship's key bindings are not written, because a spawned AI ship has no
//! input mapping to carry them: the ship you FLY keeps its keys, an escort's
//! are lost on reload.

use std::collections::BTreeMap;

use bevy::{prelude::*, ui_widgets::Activate};
use nova_assets::prelude::EnabledMods;
use nova_gameplay::prelude::Allegiance;
use nova_input::prelude::InputSource;
use nova_modding::prelude::Content;
#[cfg(not(target_arch = "wasm32"))]
use nova_modding::prelude::{BundleManifest, ModMeta};
use nova_scenario::prelude::{
    AIControllerConfig, EventActionConfig, EventConfig, ScenarioConfig, ScenarioEventConfig,
    ScenarioObjectConfig, ScenarioObjectKind, SectionId, ShipConfig, ShipSource,
    SpaceshipController, SpaceshipSectionConfig,
};
use nova_ship::prelude::GameSections;
use nova_ui::theme;

use crate::{
    config::{EditorStatus, SelectedNode},
    event::ScriptNodes,
    node::{
        found_empty_document, insert_lifted_section, insert_object_node, insert_ship_node,
        resume_ordinals, EditContext, NextChildOrdinal, NodeId, ObjectNodes, ScenarioNode,
        SectionNode, SectionNodes, ShipDriver, ShipNode,
    },
    scenario::{
        lower_fleet, ship_hull, world_objects, world_script, world_settings, HullForm,
        LoweredFleet, Range,
    },
};

/// The prefix on every mod id a save writes.
///
/// It makes the read-only rule STRUCTURAL rather than a check: the editor can
/// only ever name a bundle that starts with this, so a hand-written or
/// downloaded mod is not something a save can reach - and the list the builder
/// picks from is exactly the ids that carry it.
pub(crate) const EDITOR_BUNDLE_PREFIX: &str = "editor_";

/// Where one document is saved: the mod id it is written under, and the name
/// the builder typed to get that id.
///
/// The name is the RANGE's name as well as the mod's. A document has one name,
/// and keeping a file name and a scenario name in step by hand is the editor's
/// bookkeeping, not the builder's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SaveSlot {
    pub(crate) id: String,
    pub(crate) name: String,
}

/// The slot the open document belongs to, or `None` while nothing has named
/// it. Save writes here; Save As replaces it; Open sets it to what was opened.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct DocumentSlot(pub(crate) Option<SaveSlot>);

/// The mod id a typed name saves under, or `None` when the name holds nothing
/// an id can be made of.
///
/// DERIVED rather than typed: an id is a directory name, an enable key and a
/// merge namespace at once, and none of those is a thing to make a builder
/// spell. Every run of characters that is not an ASCII letter or digit becomes
/// one `_`, so two names that differ only in punctuation land in the same slot
/// - which the Save As window says out loud before it writes.
pub(crate) fn bundle_id(name: &str) -> Option<String> {
    let mut id = String::from(EDITOR_BUNDLE_PREFIX);
    let mut separated = false;
    for character in name.chars() {
        if !character.is_ascii_alphanumeric() {
            separated = true;
            continue;
        }
        if separated && id.len() > EDITOR_BUNDLE_PREFIX.len() {
            id.push('_');
        }
        separated = false;
        id.push(character.to_ascii_lowercase());
    }
    (id.len() > EDITOR_BUNDLE_PREFIX.len()).then_some(id)
}

/// The bundle manifest inside a slot, relative to the mod's own directory.
#[cfg(not(target_arch = "wasm32"))]
fn bundle_file(id: &str) -> String {
    format!("{id}.bundle.ron")
}

/// The one content file that manifest lists.
#[cfg(not(target_arch = "wasm32"))]
fn content_file(id: &str) -> String {
    format!("{id}.content.ron")
}

/// The range a save writes: the slot's own id, and hulls by reference.
///
/// NOT hidden, unlike the sandbox Play hands off to: a saved range is content
/// the builder made, so it belongs in the Scenarios picker once the mod is
/// enabled.
pub(crate) fn saved_range(id: &str) -> Range<'_> {
    Range {
        id,
        hidden: false,
        form: HullForm::Prototype,
        flight: false,
    }
}

/// The document as content items: every design, then the range that places
/// them.
///
/// Designs first because that is the order they are read in - a scenario
/// referencing a prototype is easier to follow under the prototype than over
/// it - and in id order, because the output is a file.
pub(crate) fn document_content(
    settings: &ScenarioNode,
    id: &str,
    world: Vec<ScenarioObjectConfig>,
    fleet: &LoweredFleet,
    script: Vec<ScenarioEventConfig>,
) -> Vec<Content> {
    let mut items: Vec<Content> = fleet
        .designs()
        .map(|ship| {
            Content::Ship(ShipConfig {
                id: ship.id.clone(),
                name: ship.id.clone(),
                hull: ship_hull(ship),
            })
        })
        .collect();
    items.push(Content::Scenario(crate::scenario::range_scenario(
        settings,
        saved_range(id),
        world,
        fleet,
        script,
    )));
    items
}

/// The manifest that makes the saved content a loadable mod.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn save_manifest(slot: &SaveSlot) -> BundleManifest {
    BundleManifest {
        content: vec![content_file(&slot.id)],
        resources: vec![],
        meta: ModMeta {
            name: slot.name.clone(),
            description: "A range built in the editor.".to_string(),
            author: String::new(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            dependencies: vec![],
            icon: None,
            screenshots: vec![],
        },
        new_game_scenario: None,
    }
}

/// One ship as the document holds it, lifted out of an authored spawn.
///
/// The seed and the save go through the same shape: a target hulk in the stock
/// range and a design read back off disk are both hulls with sections, and
/// giving them two lifts would be two ways for the document to hold one thing.
#[derive(Debug, Clone)]
pub(crate) struct LiftedShip {
    /// The node id, which is the prototype id the file wrote the design under.
    pub(crate) id: String,
    /// What the file called it. The name is display only, so a file that lost
    /// it reloads with an empty one and every surface falls back to the id.
    pub(crate) name: String,
    pub(crate) driver: ShipDriver,
    /// Which side it fights for, straight off the spawn.
    pub(crate) allegiance: Option<Allegiance>,
    /// The AI's standing orders, read off an AI spawn. A player-flown or
    /// adrift hull describes none, so one lifts with the defaults - which is
    /// also what the editor shows the moment it is flipped to AI.
    pub(crate) pilot: AIControllerConfig,
    pub(crate) pose: Transform,
    pub(crate) skin: bool,
    pub(crate) style: Option<String>,
    pub(crate) sections: Vec<SpaceshipSectionConfig>,
    /// The keys each section fires on, by section id. Sorted, because the
    /// document is rebuilt from it.
    pub(crate) binds: BTreeMap<SectionId, Vec<InputSource>>,
}

/// A saved file, split back into the two things the document is made of.
#[derive(Debug, Default, Clone)]
pub(crate) struct LiftedDocument {
    pub(crate) objects: Vec<ScenarioObjectConfig>,
    pub(crate) ships: Vec<LiftedShip>,
    /// Every handler that is not the layout, in file order.
    pub(crate) script: Vec<ScenarioEventConfig>,
    /// What the file says about the range AS A WHOLE - its name, its sky. Read
    /// back onto the document root so the fields a builder authored are the
    /// fields they are shown on the way in.
    pub(crate) settings: ScenarioNode,
}

/// Read a saved file back into a document.
///
/// `None` when the file carries no scenario: a content file of nothing but
/// ship prototypes is a legal mod and an empty document, and the two are worth
/// telling apart at the call site.
pub(crate) fn lift_content(items: &[Content]) -> Option<LiftedDocument> {
    let designs: BTreeMap<&str, &ShipConfig> = items
        .iter()
        .filter_map(|item| match item {
            Content::Ship(ship) => Some((ship.id.as_str(), ship)),
            _ => None,
        })
        .collect();
    let scenario = items.iter().find_map(|item| match item {
        Content::Scenario(scenario) => Some(scenario),
        _ => None,
    })?;
    let mut lifted = lift_objects(spawned_in_layout(scenario).cloned(), &designs);
    lifted.script = scenario
        .events
        .iter()
        .filter(|event| !is_layout(event))
        .cloned()
        .map(|mut event| {
            // The saved file names the range by the id it was WRITTEN under; in
            // memory a document names itself the sandbox. See
            // `crate::scenario::retarget_retries`.
            crate::scenario::retarget_retries(
                &mut event.actions,
                scenario.id.as_str(),
                crate::scenario::SANDBOX_ID,
            );
            event
        })
        .collect();
    lifted.settings = ScenarioNode {
        name: scenario.name.clone(),
        description: scenario.description.clone(),
        cubemap: scenario.cubemap.clone(),
        skybox_brightness: scenario.skybox_brightness,
    };
    Some(lifted)
}

/// Split authored spawns into the two things a document is made of.
///
/// A HULL becomes a ship node - its sections are nodes of their own and a
/// double click goes inside it. Everything else is an object, placed and
/// lowered verbatim.
///
/// The one hull that stays an object is one naming a prototype nothing here
/// carries: the editor would have to invent the sections it cannot read, and a
/// design it cannot show is not one it should offer to edit.
pub(crate) fn lift_objects(
    objects: impl IntoIterator<Item = ScenarioObjectConfig>,
    designs: &BTreeMap<&str, &ShipConfig>,
) -> LiftedDocument {
    let mut lifted = LiftedDocument::default();
    for object in objects {
        match lift_ship(&object, designs) {
            Some(ship) => lifted.ships.push(ship),
            None => lifted.objects.push(object),
        }
    }
    lifted
}

/// Every object the range's LAYOUT handler puts on the board.
fn spawned_in_layout(scenario: &ScenarioConfig) -> impl Iterator<Item = &ScenarioObjectConfig> {
    scenario
        .events
        .iter()
        .filter(|event| is_layout(event))
        .flat_map(|event| &event.actions)
        .filter_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object) => Some(object),
            _ => None,
        })
}

/// Whether this handler is the LAYOUT: an unfiltered `OnStart` that does
/// nothing but spawn.
///
/// A PROPERTY rather than a position, so a hand-written mod is read the same
/// way an editor-written one is. It is exactly the handler
/// `crate::scenario::range_events` derives, so a file the editor wrote lifts
/// its world back out of the same place it put it - and a handler that spawns
/// reinforcements beside a story beat stays script, because it has filters or
/// company.
fn is_layout(event: &ScenarioEventConfig) -> bool {
    matches!(event.name, EventConfig::OnStart)
        && !event.once
        && event.filters.is_empty()
        && event
            .actions
            .iter()
            .all(|action| matches!(action, EventActionConfig::SpawnScenarioObject(_)))
}

/// The ship node this spawn stands for, or `None` if it is not a hull the
/// editor can show.
///
/// The node keeps the spawn's OWN id: a picket is `picket_warden` because the
/// sandbox's wake handler flips that id, and the design a save wrote is
/// referenced by the id it was written under.
fn lift_ship(
    object: &ScenarioObjectConfig,
    designs: &BTreeMap<&str, &ShipConfig>,
) -> Option<LiftedShip> {
    let ScenarioObjectKind::Spaceship(spawn) = &object.kind else {
        return None;
    };
    let (id, hull) = match &spawn.hull {
        // A design of this file's own, under the id it was written with.
        ShipSource::Prototype(id) => (id.clone(), &designs.get(id.as_str())?.hull),
        // A hull authored in place - every seeded ship of the stock range.
        ShipSource::Inline(hull) => (object.base.id.clone(), hull),
    };
    let (driver, pilot, binds) = match &spawn.controller {
        SpaceshipController::Player(config) => (
            ShipDriver::Player,
            AIControllerConfig::default(),
            config
                .input_mapping
                .iter()
                .map(|(section, binds)| (section.clone(), binds.clone()))
                .collect(),
        ),
        // An AI ship carries no input mapping, so its keys are the empty set;
        // its standing orders are what make it the ship it is.
        SpaceshipController::AI(pilot) => (ShipDriver::Ai, pilot.clone(), BTreeMap::new()),
        SpaceshipController::None => (
            ShipDriver::Adrift,
            AIControllerConfig::default(),
            BTreeMap::new(),
        ),
    };
    Some(LiftedShip {
        id,
        name: object.base.name.clone(),
        driver,
        allegiance: spawn.allegiance,
        pilot,
        pose: Transform::from_translation(object.base.position.to_engine())
            .with_rotation(object.base.rotation),
        skin: hull.skin,
        style: hull.style.clone(),
        sections: hull.sections.clone(),
        binds,
    })
}

/// Put one lifted ship into the document under `scenario`, sections and all.
///
/// Says the ordinal its section counter has to resume at, so a part placed on
/// a loaded ship cannot mint an id the file already used.
pub(crate) fn insert_lifted_ship(
    commands: &mut Commands,
    sections: Option<&GameSections>,
    scenario: Entity,
    ship: LiftedShip,
) -> (Entity, u32) {
    let node = insert_ship_node(
        commands,
        scenario,
        NodeId(ship.id.clone()),
        ShipNode {
            name: ship.name.clone(),
            skin: ship.skin,
            style: ship.style.clone(),
            driver: ship.driver,
            allegiance: ship.allegiance,
            pilot: ship.pilot.clone(),
        },
        ship.pose,
    );
    let ordinal = resume_ordinal(ship.sections.iter().map(|section| section.id.as_str()));
    for section in ship.sections {
        let binds = ship.binds.get(&section.id).cloned().unwrap_or_default();
        insert_lifted_section(
            commands,
            sections,
            node,
            NodeId(section.id),
            SectionNode {
                source: section.source,
                modifications: section.modifications,
                binds,
            },
            Transform::from_translation(section.position).with_rotation(section.rotation),
        );
    }
    (node, ordinal)
}

/// The ordinal a node's id counter must resume at so a fresh mint cannot
/// collide with a lifted id.
///
/// Ids are `{stem}_{n}` and the counter is monotonic within one document's
/// life. A reload starts a new life, so the guarantee it has to restore is the
/// weaker, sufficient one: every id minted from here on is new. The highest
/// suffix already in use is exactly that line.
pub(crate) fn resume_ordinal<'a>(ids: impl IntoIterator<Item = &'a str>) -> u32 {
    ids.into_iter()
        .filter_map(|id| id.rsplit_once('_'))
        .filter_map(|(_, ordinal)| ordinal.parse::<u32>().ok())
        .max()
        .unwrap_or_default()
}

/// Write the document into `slot`: the manifest, the content, and the index
/// record that makes it an installed mod the game can enable.
///
/// Files first, index last - the order a failed write has to leave a readable
/// state in. `Err` carries a line fit to show the builder.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn write_save(slot: &SaveSlot, items: &[Content]) -> Result<(), String> {
    use nova_assets::mod_cache::prelude::install_local;
    use nova_modding::prelude::{serialize_content, serialize_manifest};

    let manifest =
        serialize_manifest(&save_manifest(slot)).map_err(|error| format!("manifest: {error}"))?;
    let content = serialize_content(items).map_err(|error| format!("content: {error}"))?;
    let (bundle, content_path) = (bundle_file(&slot.id), content_file(&slot.id));
    install_local(
        &slot.id,
        env!("CARGO_PKG_VERSION"),
        &bundle,
        &[
            (bundle.clone(), manifest.into_bytes()),
            (content_path, content.into_bytes()),
        ],
    )
    .map_err(|error| error.to_string())
}

/// Read one saved slot back, or say why there is nothing to read.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn read_save(id: &str) -> Result<Vec<Content>, String> {
    use nova_assets::mod_cache::prelude::read_mod_file;
    use nova_modding::prelude::parse_content;

    let bytes =
        read_mod_file(id, &content_file(id)).ok_or_else(|| format!("'{id}' holds no content"))?;
    parse_content(&bytes).map_err(|error| error.to_string())
}

/// Every slot the editor has written, by name.
///
/// Read off the installed-mods index and filtered to the editor's own prefix,
/// so what a builder is offered is what they saved and never someone else's
/// mod. A record whose manifest will not read is dropped rather than listed
/// under its id: a row that cannot be opened is worse than a row that is not
/// there.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn saved_bundles() -> Vec<SaveSlot> {
    use nova_assets::mod_cache::prelude::{read_index, read_mod_file};
    use nova_modding::prelude::parse_manifest;

    let mut slots: Vec<SaveSlot> = read_index()
        .unwrap_or_default()
        .into_iter()
        .filter(|record| record.id.starts_with(EDITOR_BUNDLE_PREFIX))
        .filter_map(|record| {
            let bytes = read_mod_file(&record.id, &record.bundle)?;
            let manifest = parse_manifest(&bytes).ok()?;
            Some(SaveSlot {
                id: record.id,
                name: manifest.meta.name,
            })
        })
        .collect();
    slots.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    slots
}

/// The web has no local mod cache to write into: its store is asynchronous and
/// the editor's save is not. Refused with a line rather than silently doing
/// nothing.
#[cfg(target_arch = "wasm32")]
pub(crate) fn write_save(_slot: &SaveSlot, _items: &[Content]) -> Result<(), String> {
    Err("saving is not available on the web yet".to_string())
}

/// The same, read side.
#[cfg(target_arch = "wasm32")]
pub(crate) fn read_save(_id: &str) -> Result<Vec<Content>, String> {
    Err("loading is not available on the web yet".to_string())
}

/// Nothing to list, for the same reason there is nothing to write. The file
/// window says so where the rows would be.
#[cfg(target_arch = "wasm32")]
pub(crate) fn saved_bundles() -> Vec<SaveSlot> {
    Vec::new()
}

/// What the document has been asked to do with its file.
///
/// A REQUEST rather than the work itself, for the reason the frame request is
/// one: two callers ask (the File menu and the keyboard) and one worker
/// answers, so the answer is written once and cannot drift between them.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub(crate) enum FileRequest {
    /// Nothing pending.
    #[default]
    None,
    /// Write the document to the slot it already belongs to.
    Save,
    /// Write it to the slot the builder just named, and belong there from now
    /// on.
    SaveAs(SaveSlot),
    /// Throw the document away and rebuild it from a slot.
    Open(String),
}

/// Which file window the builder asked for, if any.
///
/// A request rather than a spawn, for the reason [`FileRequest`] is one: three
/// callers ask (two menu rows and the keyboard) and one builder answers.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileWindowRequest(pub(crate) Option<FileWindowKind>);

/// What a file window is being opened to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileWindowKind {
    /// Name a slot and write the document into it.
    SaveAs,
    /// Pick a slot and replace the document with it.
    Open,
}

/// The save shortcut. The one the File menu has always advertised.
const SAVE_KEY: KeyCode = KeyCode::KeyS;

/// Save straight into the document's own slot, or ask for a name when it has
/// none yet.
///
/// The first save of a document is a Save As whatever asked for it, so the
/// menu row and the shortcut cannot mean different things.
fn save_or_ask(document: &DocumentSlot, request: &mut FileRequest, window: &mut FileWindowRequest) {
    if document.0.is_some() {
        *request = FileRequest::Save;
    } else {
        window.0 = Some(FileWindowKind::SaveAs);
    }
}

/// File > Save.
pub(crate) fn ask_to_save(
    _activate: On<Activate>,
    document: Res<DocumentSlot>,
    mut request: ResMut<FileRequest>,
    mut window: ResMut<FileWindowRequest>,
) {
    save_or_ask(&document, &mut request, &mut window);
}

/// File > Save As...
pub(crate) fn ask_to_save_as(_activate: On<Activate>, mut window: ResMut<FileWindowRequest>) {
    window.0 = Some(FileWindowKind::SaveAs);
}

/// File > Open.
pub(crate) fn ask_to_open(_activate: On<Activate>, mut window: ResMut<FileWindowRequest>) {
    window.0 = Some(FileWindowKind::Open);
}

/// Ctrl+S: the same request the menu row raises.
pub(crate) fn save_key(
    keys: Res<ButtonInput<KeyCode>>,
    document: Res<DocumentSlot>,
    mut request: ResMut<FileRequest>,
    mut window: ResMut<FileWindowRequest>,
) {
    let held = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if held && keys.just_pressed(SAVE_KEY) {
        save_or_ask(&document, &mut request, &mut window);
    }
}

/// Answer whatever was asked, and say what happened.
///
/// The whole document goes out every time. There is nothing to diff against
/// and a partial save of a tree is not a document, so the file is rewritten
/// from what is on screen.
///
/// A load MERGES NOTHING. A document half of one file and half of another is a
/// thing nobody asked for, so the tree is torn down and founded again.
#[expect(
    clippy::too_many_arguments,
    reason = "one worker reads every kind of node the document holds and writes it back"
)]
pub(crate) fn apply_file_request(
    mut commands: Commands,
    mut request: ResMut<FileRequest>,
    mut document: ResMut<DocumentSlot>,
    time: Res<Time>,
    // Optional: a headless fixture drives this worker without the content
    // pipeline behind it, and a save that cannot be published is still a save.
    mut enabled: Option<ResMut<EnabledMods>>,
    mut context: ResMut<EditContext>,
    mut selected: ResMut<SelectedNode>,
    mut status: ResMut<EditorStatus>,
    nodes: SectionNodes,
    q_objects: ObjectNodes,
    q_ships: Query<(Entity, &NodeId, &ShipNode, &Transform)>,
    script: ScriptNodes,
    q_settings: Query<&ScenarioNode>,
    roots: Query<Entity, With<ScenarioNode>>,
) {
    let asked = std::mem::take(&mut *request);
    let now = time.elapsed_secs_f64();
    // Save writes where the document already lives; Save As says where it is
    // to live from now on. One write either way.
    let target = match &asked {
        FileRequest::Save => document.0.clone(),
        FileRequest::SaveAs(slot) => Some(slot.clone()),
        FileRequest::None | FileRequest::Open(_) => None,
    };
    match asked {
        FileRequest::None => {}
        FileRequest::Save | FileRequest::SaveAs(_) => {
            let Some(slot) = target else {
                status.say("nothing has named this document yet", theme::RED, now);
                return;
            };
            let items = document_content(
                &world_settings(&context, &q_settings),
                &slot.id,
                world_objects(&context, &q_objects),
                &lower_fleet(&q_ships, &nodes),
                world_script(&context, &script),
            );
            match write_save(&slot, &items) {
                Ok(()) => {
                    // ENABLED as well as installed. A save is the builder's own
                    // document, not a stranger's mod: asking them to go and
                    // switch it on in the Mods panel before their own range
                    // appears in Scenarios is the friction the save was meant to
                    // remove. Still theirs to turn off there.
                    //
                    // The range reaches the Scenarios list on the way OUT: the
                    // content restart runs when the editor is left for the main
                    // menu, which is the first moment the picker is reachable
                    // anyway.
                    if let Some(enabled) = enabled.as_mut() {
                        enabled.0.insert(slot.id.clone());
                    }
                    info!("editor: saved the document as mod '{}'", slot.id);
                    status.say(format!("saved as {}", slot.name), theme::PHOSPHOR, now);
                    document.0 = Some(slot);
                }
                Err(error) => {
                    error!("editor: the save failed - {error}");
                    status.say(format!("save failed: {error}"), theme::RED, now);
                }
            }
        }
        FileRequest::Open(id) => {
            let lifted = match read_save(&id).map(|items| lift_content(&items)) {
                Ok(Some(lifted)) => lifted,
                Ok(None) => {
                    status.say("that file holds no range", theme::RED, now);
                    return;
                }
                Err(error) => {
                    status.say(format!("nothing to open: {error}"), theme::RED, now);
                    return;
                }
            };
            let (ships, objects) = (lifted.ships.len(), lifted.objects.len());
            // The opened document belongs to the slot it came out of, so the
            // next plain Save goes back where it was read from. Its name is the
            // range's own, which is the name the slot was written under.
            document.0 = Some(SaveSlot {
                id,
                name: lifted.settings.name.clone(),
            });
            for root in &roots {
                commands.entity(root).despawn();
            }
            selected.0 = None;
            let scenario = found_empty_document(&mut commands, &mut context);
            commands.queue(move |world: &mut World| fill_document(world, scenario, lifted));
            info!("editor: opened the saved document - {ships} ship(s), {objects} object(s)");
            status.say(
                format!("opened - {ships} ship(s), {objects} object(s)"),
                theme::PHOSPHOR,
                now,
            );
        }
    }
}

/// Put a lifted document's nodes under a freshly founded scenario node.
///
/// A deferred command rather than more of the system above: the scenario node
/// it hangs everything on was spawned by that system's `Commands`, so the
/// ordinal counters it has to write are not queryable until the queue flushes.
fn fill_document(world: &mut World, scenario: Entity, document: LiftedDocument) {
    // Taken out and put back: the section configs are read through it while
    // `world.commands()` holds a mutable borrow of the world.
    let sections = world.remove_resource::<GameSections>();
    world.entity_mut(scenario).insert(document.settings);
    let mut ids: Vec<String> = Vec::new();
    let mut ship_ordinals: Vec<(Entity, u32)> = Vec::new();
    {
        let mut commands = world.commands();
        for object in document.objects {
            ids.push(object.base.id.clone());
            insert_object_node(&mut commands, scenario, object);
        }
        for ship in document.ships {
            ids.push(ship.id.clone());
            ship_ordinals.push(insert_lifted_ship(
                &mut commands,
                sections.as_ref(),
                scenario,
                ship,
            ));
        }
        crate::event::lift(&mut commands, scenario, document.script);
    }
    world.flush();

    let scenario_ordinal = resume_ordinal(ids.iter().map(String::as_str));
    let mut query = world.query::<&mut NextChildOrdinal>();
    let mut ordinals = query.query_mut(world);
    resume_ordinals(&mut ordinals, scenario, scenario_ordinal);
    for (node, ordinal) in ship_ordinals {
        resume_ordinals(&mut ordinals, node, ordinal);
    }
    if let Some(sections) = sections {
        world.insert_resource(sections);
    }
}

#[cfg(test)]
mod tests;
