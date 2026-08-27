//! What a saved document IS: a mod bundle, in the same format a hand-written
//! mod is authored in.
//!
//! The file is a `Vec<Content>`: one `Content::Ship` per design in the
//! document, then one `Content::Scenario` whose spaceships reference those
//! designs by id. Nothing is written twice - edit a design and every instance
//! of it changes, and "export my ship" is the file you already saved.
//!
//! THE LOWERING CONVENTION, both ways. The editor owns the LAYOUT: the objects
//! spawned on start, and the ships among them. It does not own the SCRIPT - the
//! rock belts, the trip spheres, the wake handlers and the outcome are the
//! range's own (see [`crate::scenario`]), so a save writes them out of the same
//! constants that authored them and a load steps over them. The document
//! round-trips exactly; a hand edit to the script does not survive a re-save,
//! which is the same thing as saying the editor does not edit the script yet.
//!
//! An AI ship's key bindings are not written, because a spawned AI ship has no
//! input mapping to carry them: the ship you FLY keeps its keys, an escort's
//! are lost on reload.

use std::collections::BTreeMap;

use bevy::{prelude::*, ui_widgets::Activate};
use nova_assets::prelude::EnabledMods;
use nova_gameplay::prelude::{Allegiance, AssetRef};
use nova_input::prelude::InputSource;
use nova_modding::prelude::Content;
#[cfg(not(target_arch = "wasm32"))]
use nova_modding::prelude::{BundleManifest, ModMeta};
use nova_scenario::prelude::{
    AIControllerConfig, EventActionConfig, EventConfig, ScenarioConfig, ScenarioObjectConfig,
    ScenarioObjectKind, SectionId, ShipConfig, ShipSource, SpaceshipController,
    SpaceshipSectionConfig,
};
use nova_ship::prelude::GameSections;
use nova_ui::theme;

use crate::{
    config::{EditorStatus, SelectedNode},
    node::{
        found_empty_document, insert_lifted_section, insert_object_node, insert_ship_node,
        resume_ordinals, EditContext, NextChildOrdinal, NodeId, ObjectNodes, ScenarioNode,
        SectionNode, SectionNodes, ShipDriver, ShipNode,
    },
    scenario::{lower_fleet, ship_hull, world_objects, HullForm, LoweredFleet, Range, DEFAULT_SKY},
};

/// The mod id a save installs under: the cache directory, the enable key and
/// the overlay namespace, all one word.
///
/// ONE slot, deliberately. A file browser and a name field are their own task;
/// what this buys is the property the save had to have first - the document
/// survives the process. It also makes the read-only rule structural rather
/// than a check: the editor can only ever write this id, so a hand-written mod
/// is not something a save can reach.
pub(crate) const SAVE_MOD_ID: &str = "editor_save";
/// The bundle manifest inside that mod, relative to its own directory.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const SAVE_BUNDLE_FILE: &str = "editor_save.bundle.ron";
/// The one content file the manifest lists.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const SAVE_CONTENT_FILE: &str = "editor_save.content.ron";

/// The range a save writes: its own id, and hulls by reference.
///
/// NOT hidden, unlike the sandbox Play hands off to: a saved range is content
/// the builder made, so it belongs in the Scenarios picker once the mod is
/// enabled.
pub(crate) const SAVED_RANGE: Range<'static> = Range {
    id: "editor_save",
    name: "Saved Range",
    hidden: false,
    form: HullForm::Prototype,
    flight: false,
};

/// The document as content items: every design, then the range that places
/// them.
///
/// Designs first because that is the order they are read in - a scenario
/// referencing a prototype is easier to follow under the prototype than over
/// it - and in id order, because the output is a file.
pub(crate) fn document_content(
    world: Vec<ScenarioObjectConfig>,
    fleet: &LoweredFleet,
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
        AssetRef::from(DEFAULT_SKY),
        SAVED_RANGE,
        world,
        fleet,
    )));
    items
}

/// The manifest that makes the saved content a loadable mod.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn save_manifest() -> BundleManifest {
    BundleManifest {
        content: vec![SAVE_CONTENT_FILE.to_string()],
        resources: vec![],
        meta: ModMeta {
            name: "Saved Range".to_string(),
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
    Some(lift_objects(spawned_on_start(scenario).cloned(), &designs))
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

/// Every object the range spawns on start.
///
/// The layout half of the convention: a spawn in an OnStart handler is where
/// something STANDS, and every other action in that handler - and every other
/// handler - is script the editor does not read.
fn spawned_on_start(scenario: &ScenarioConfig) -> impl Iterator<Item = &ScenarioObjectConfig> {
    scenario
        .events
        .iter()
        .filter(|event| matches!(event.name, EventConfig::OnStart))
        .flat_map(|event| &event.actions)
        .filter_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object) => Some(object),
            _ => None,
        })
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
        pose: Transform::from_translation(object.base.position).with_rotation(object.base.rotation),
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

/// Write the document out as the editor's saved mod: the manifest, the content,
/// and the index record that makes it an installed mod the game can enable.
///
/// Files first, index last - the order a failed write has to leave a readable
/// state in. `Err` carries a line fit to show the builder.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn write_save(items: &[Content]) -> Result<(), String> {
    use nova_assets::mod_cache::prelude::install_local;
    use nova_modding::prelude::{serialize_content, serialize_manifest};

    let manifest =
        serialize_manifest(&save_manifest()).map_err(|error| format!("manifest: {error}"))?;
    let content = serialize_content(items).map_err(|error| format!("content: {error}"))?;
    install_local(
        SAVE_MOD_ID,
        env!("CARGO_PKG_VERSION"),
        SAVE_BUNDLE_FILE,
        &[
            (SAVE_BUNDLE_FILE.to_string(), manifest.into_bytes()),
            (SAVE_CONTENT_FILE.to_string(), content.into_bytes()),
        ],
    )
    .map_err(|error| error.to_string())
}

/// Read the editor's saved mod back, or say why there is nothing to read.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn read_save() -> Result<Vec<Content>, String> {
    use nova_assets::mod_cache::prelude::read_mod_file;
    use nova_modding::prelude::parse_content;

    let bytes = read_mod_file(SAVE_MOD_ID, SAVE_CONTENT_FILE)
        .ok_or_else(|| "nothing saved yet".to_string())?;
    parse_content(&bytes).map_err(|error| error.to_string())
}

/// The web has no local mod cache to write into: its store is asynchronous and
/// the editor's save is not. Refused with a line rather than silently doing
/// nothing.
#[cfg(target_arch = "wasm32")]
pub(crate) fn write_save(_items: &[Content]) -> Result<(), String> {
    Err("saving is not available on the web yet".to_string())
}

/// The same, read side.
#[cfg(target_arch = "wasm32")]
pub(crate) fn read_save() -> Result<Vec<Content>, String> {
    Err("loading is not available on the web yet".to_string())
}

/// What the document has been asked to do with its file.
///
/// A REQUEST rather than the work itself, for the reason the frame request is
/// one: two callers ask (the File menu and the keyboard) and one worker
/// answers, so the answer is written once and cannot drift between them.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileRequest {
    /// Nothing pending.
    #[default]
    None,
    /// Write the document out.
    Save,
    /// Throw the document away and rebuild it from the file.
    Open,
}

/// The save shortcut. The one the File menu has always advertised.
const SAVE_KEY: KeyCode = KeyCode::KeyS;

/// File > Save.
pub(crate) fn ask_to_save(_activate: On<Activate>, mut request: ResMut<FileRequest>) {
    *request = FileRequest::Save;
}

/// File > Open.
pub(crate) fn ask_to_open(_activate: On<Activate>, mut request: ResMut<FileRequest>) {
    *request = FileRequest::Open;
}

/// Ctrl+S: the same request the menu row raises.
pub(crate) fn save_key(keys: Res<ButtonInput<KeyCode>>, mut request: ResMut<FileRequest>) {
    let held = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if held && keys.just_pressed(SAVE_KEY) {
        *request = FileRequest::Save;
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
    roots: Query<Entity, With<ScenarioNode>>,
) {
    let asked = std::mem::take(&mut *request);
    let now = time.elapsed_secs_f64();
    match asked {
        FileRequest::None => {}
        FileRequest::Save => {
            let items = document_content(
                world_objects(&context, &q_objects),
                &lower_fleet(&q_ships, &nodes),
            );
            match write_save(&items) {
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
                        enabled.0.insert(SAVE_MOD_ID.to_string());
                    }
                    info!("editor: saved the document as mod '{SAVE_MOD_ID}'");
                    status.say("saved", theme::PHOSPHOR, now);
                }
                Err(error) => {
                    error!("editor: the save failed - {error}");
                    status.say(format!("save failed: {error}"), theme::RED, now);
                }
            }
        }
        FileRequest::Open => {
            let document = match read_save().map(|items| lift_content(&items)) {
                Ok(Some(document)) => document,
                Ok(None) => {
                    status.say("the saved file holds no range", theme::RED, now);
                    return;
                }
                Err(error) => {
                    status.say(format!("nothing to open: {error}"), theme::RED, now);
                    return;
                }
            };
            let (ships, objects) = (document.ships.len(), document.objects.len());
            for root in &roots {
                commands.entity(root).despawn();
            }
            selected.0 = None;
            let scenario = found_empty_document(&mut commands, &mut context);
            commands.queue(move |world: &mut World| fill_document(world, scenario, document));
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
