//! What a saved document IS: a mod bundle, in the same format a hand-written
//! mod is authored in.
//!
//! A document lowers to a `Vec<Content>`: one `Content::Ship` per design in the
//! document, then one `Content::Scenario` whose spaceships reference those
//! designs by id. Nothing is written twice - edit a design and every instance
//! of it changes, and "export my ship" is the file you already saved. The
//! [`crate::project::ModProject`] holds these items for each of its scenarios.
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
use nova_gameplay::prelude::Allegiance;
use nova_input::prelude::InputSource;
use nova_modding::prelude::Content;
use nova_scenario::prelude::{
    AIControllerConfig, EventActionConfig, EventConfig, ScenarioConfig, ScenarioEventConfig,
    ScenarioObjectConfig, ScenarioObjectKind, SectionId, ShipDesign, ShipDesignId,
    ShipDesignPrototype, ShipDesignSource, SpaceshipConfig, SpaceshipController,
    SpaceshipSectionConfig, StationConfig, StationDesignId, StationDesignPrototype,
    StationDesignSource,
};
use nova_ship::prelude::{GameSections, TargetingSettings};

use crate::{
    node::{
        insert_lifted_section, insert_object_node, insert_ship_node, resume_ordinals,
        NextChildOrdinal, NodeId, ScenarioNode, SectionNode, ShipDriver, ShipNode,
    },
    project::{
        ids::{design_id, node_of_design},
        verbs::{accept, save_or_ask},
    },
    scenario::{ship_design, spawn_of, FileFacts, HullForm, LoweredFleet, LoweredShip, Range},
    station::is_stock_station,
    ui::files::PathPurposeType,
};

/// The range a save writes: its own id, and hulls by reference.
///
/// A saved range is content the builder made, so it belongs in the Scenarios
/// picker once the mod is enabled - unlike the sandbox Play hands off to,
/// which the picker skips by its reserved id.
pub(crate) fn saved_range<'a>(id: &'a str, file: FileFacts<'a>) -> Range<'a> {
    Range {
        id,
        form: HullForm::Prototype { scenario: id },
        flight: false,
        file,
    }
}

/// The document as content items: every design, then the range that places
/// them.
///
/// Designs first because that is the order they are read in - a scenario
/// referencing a prototype is easier to follow under the prototype than over
/// it - and in id order, because the output is a file. A ship marked as a
/// station writes its `stations` design, built by
/// [`StationCatalogs::designs`], in place of a ship prototype: the range
/// spawns it by that id.
pub(crate) fn document_content(
    settings: &ScenarioNode,
    id: &str,
    file: FileFacts,
    world: Vec<ScenarioObjectConfig>,
    fleet: &LoweredFleet,
    stations: Vec<StationDesignPrototype>,
    script: Vec<ScenarioEventConfig>,
    targeting: &TargetingSettings,
) -> Vec<Content> {
    let mut items: Vec<Content> = fleet
        .designs()
        .filter(|ship| !ship.station)
        .map(|ship| {
            Content::Ship(ShipDesignPrototype {
                id: design_id(id, &ship.id).as_str().into(),
                name: ship.id.clone(),
                design: ship_design(ship),
            })
        })
        .collect();
    items.extend(stations.into_iter().map(Content::Station));
    items.push(Content::Scenario(crate::scenario::range_scenario(
        settings,
        saved_range(id, file),
        world,
        fleet,
        script,
        targeting,
    )));
    items
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
    /// Whether the file spawned it as a station of its own design.
    pub(crate) station: bool,
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

impl LiftedShip {
    /// This ship as the lowering will see it.
    fn lowered(&self) -> LoweredShip {
        LoweredShip {
            id: self.id.clone(),
            name: self.name.clone(),
            driver: self.driver,
            allegiance: self.allegiance,
            pilot: self.pilot.clone(),
            sections: self.sections.clone(),
            inputs: self
                .sections
                .iter()
                .filter_map(|section| Some((section.id.clone(), self.binds.get(&section.id)?)))
                .filter(|(_, binds)| !binds.is_empty())
                .map(|(section, binds)| (section, binds.clone()))
                .collect(),
            skin: self.skin,
            style: self.style.clone(),
            position: self.pose.translation,
            rotation: self.pose.rotation,
            station: self.station,
        }
    }
}

/// Whose spawns are being lifted, which decides how much of one a node may hold.
#[derive(Debug, Clone, Copy)]
pub(crate) enum LiftSource<'a> {
    /// The stock world: every hull with sections is the editor's own.
    Seed,
    /// The scenario `scenario` of a mod on disk. A spawn lifts only if its node
    /// writes it back; the rest stand as authored.
    File {
        scenario: &'a str,
        /// The one-scenario layout, whose designs stand bare until this save
        /// moves them. Their spelling is the editor's, whatever the id says.
        old_save: bool,
    },
}

/// Read the scenario `id` of a mod back into a document.
///
/// `None` when the items hold no such scenario. Every item is offered because a
/// scenario may spawn a design any file of the mod carries. `old_save` is
/// [`LiftSource::File`]'s.
pub(crate) fn lift_scenario<'a>(
    items: impl Iterator<Item = &'a Content>,
    id: &str,
    old_save: bool,
) -> Option<LiftedDocument> {
    let items: Vec<&Content> = items.collect();
    let designs = FileDesigns {
        ships: items
            .iter()
            .filter_map(|item| match item {
                Content::Ship(ship) => Some((&ship.id, ship)),
                _ => None,
            })
            .collect(),
        stations: items
            .iter()
            .filter_map(|item| match item {
                Content::Station(station) => Some((&station.id, station)),
                _ => None,
            })
            .collect(),
    };
    let scenario = items.iter().find_map(|item| match item {
        Content::Scenario(scenario) if scenario.id == id => Some(scenario),
        _ => None,
    })?;
    let source = LiftSource::File {
        scenario: id,
        old_save,
    };
    let mut lifted = lift_objects(spawned_in_layout(scenario).cloned(), &designs, source);
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

/// The designs one saved file carries, by id: what its spawns may reference
/// and the editor can still show.
#[derive(Default)]
pub(crate) struct FileDesigns<'a> {
    /// Its ship prototypes.
    pub(crate) ships: BTreeMap<&'a ShipDesignId, &'a ShipDesignPrototype>,
    /// Its station designs.
    pub(crate) stations: BTreeMap<&'a StationDesignId, &'a StationDesignPrototype>,
}

/// Split authored spawns into the two things a document is made of.
///
/// A HULL becomes a ship node - its sections are nodes of their own and a
/// double click goes inside it. A station of a design the same file carries
/// is a hull too, and lifts as a ship node marked as a station. Everything
/// else is an object, placed and lowered verbatim.
///
/// A hull stays an object when it names a design nothing here carries - a ship
/// prototype, or a station from the shipped catalog: the editor would have to
/// invent the sections it cannot read, and a design it cannot show is not one it
/// should offer to edit.
///
/// A hull from a file stays an object too when a ship node could not write it
/// back: a node holds no stock, no patches and no spawn id of its own, and the
/// design it writes is the editor's. See [`writes_back`].
pub(crate) fn lift_objects(
    objects: impl IntoIterator<Item = ScenarioObjectConfig>,
    designs: &FileDesigns,
    source: LiftSource,
) -> LiftedDocument {
    let objects: Vec<ScenarioObjectConfig> = objects.into_iter().collect();
    let mut lifted = LiftedDocument::default();
    for object in &objects {
        let ship = lift_ship(object, designs, source).filter(|ship| {
            let taken = |other: &ScenarioObjectConfig| {
                other.base.id == ship.id && other.base.id != object.base.id
            };
            !objects.iter().any(taken)
                && !lifted
                    .ships
                    .iter()
                    .any(|held: &LiftedShip| held.id == ship.id)
        });
        match ship {
            Some(ship) => lifted.ships.push(ship),
            None => lifted.objects.push(object.clone()),
        }
    }
    lifted
}

/// Every object the range's LAYOUT handler puts on the board.
pub(crate) fn spawned_in_layout(
    scenario: &ScenarioConfig,
) -> impl Iterator<Item = &ScenarioObjectConfig> {
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
pub(crate) fn is_layout(event: &ScenarioEventConfig) -> bool {
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
/// sandbox's wake handler flips that id. A design the mod carries names the
/// node it was written for, which [`node_of_design`] reads back out of the
/// scenario's namespace.
fn lift_ship(
    object: &ScenarioObjectConfig,
    designs: &FileDesigns,
    source: LiftSource,
) -> Option<LiftedShip> {
    let scenario = match source {
        LiftSource::Seed => "",
        LiftSource::File { scenario, .. } => scenario,
    };
    let ship = match &object.kind {
        ScenarioObjectKind::Spaceship(spawn) => lift_spaceship(object, spawn, designs, scenario),
        ScenarioObjectKind::Station(spawn) => lift_station(object, spawn, designs, scenario),
        _ => None,
    }?;
    writes_back(&ship, object, designs, source).then_some(ship)
}

/// Whether a node holding `ship` writes `object` back as it stands, and the
/// design it names with it.
///
/// Asked of the lowering itself, so a field added to a spawn stays with the file
/// until a node can hold it. A design's sounds are left out: the editor writes
/// its own, and a build saved before one changed is still its build.
fn writes_back(
    ship: &LiftedShip,
    object: &ScenarioObjectConfig,
    designs: &FileDesigns,
    source: LiftSource,
) -> bool {
    let LiftSource::File { scenario, old_save } = source else {
        return true;
    };
    let lowered = ship.lowered();
    let mut written = spawn_of(&lowered, HullForm::Prototype { scenario });
    written.base.position = object.base.position;
    let mut held = object.clone();
    let spelled = design_id(scenario, &ship.id);
    match &mut held.kind {
        ScenarioObjectKind::Spaceship(SpaceshipConfig {
            design: ShipDesignSource::Prototype { id, .. },
            ..
        }) if old_save => *id = spelled.as_str().into(),
        ScenarioObjectKind::Station(StationConfig {
            design: StationDesignSource::Prototype(id),
            ..
        }) if old_save => *id = spelled.as_str().into(),
        ScenarioObjectKind::Spaceship(SpaceshipConfig {
            design: ShipDesignSource::Inline(_),
            ..
        }) => return old_save,
        _ => {}
    }
    if format!("{held:?}") != format!("{written:?}") {
        return false;
    }
    old_save || design_writes_back(&lowered, object, designs)
}

/// Whether the design `object` names is the one [`document_content`] writes for
/// `ship`. A bench edit makes it another: a lowering would take that edit back.
fn design_writes_back(
    ship: &LoweredShip,
    object: &ScenarioObjectConfig,
    designs: &FileDesigns,
) -> bool {
    match &object.kind {
        ScenarioObjectKind::Spaceship(SpaceshipConfig {
            design: ShipDesignSource::Prototype { id, .. },
            ..
        }) => designs.ships.get(id).is_some_and(|held| {
            let written = ShipDesign {
                presentation: held.design.presentation.clone(),
                ..ship_design(ship)
            };
            held.name == ship.id && format!("{:?}", held.design) == format!("{written:?}")
        }),
        ScenarioObjectKind::Station(StationConfig {
            design: StationDesignSource::Prototype(id),
            ..
        }) => designs
            .stations
            .get(id)
            .is_some_and(|held| is_stock_station(held, ship)),
        _ => false,
    }
}

/// The ship node a spaceship spawn stands for, when it can show the hull.
fn lift_spaceship(
    object: &ScenarioObjectConfig,
    spawn: &SpaceshipConfig,
    designs: &FileDesigns,
    scenario: &str,
) -> Option<LiftedShip> {
    let (id, design) = match &spawn.design {
        // A design of this mod's own, under the id it was written with.
        ShipDesignSource::Prototype { id, .. } => (
            node_of_design(scenario, id.as_str()).to_string(),
            &designs.ships.get(id)?.design,
        ),
        // A design authored in place - every seeded ship of the stock range.
        ShipDesignSource::Inline(design) => (object.base.id.clone(), design),
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
        skin: design.presentation.skin,
        style: design.presentation.style.clone(),
        sections: design.sections.clone(),
        binds,
        station: false,
    })
}

/// The ship node a station spawn stands for, when its design is a catalog
/// design the same file carries: the hull lifts like a ship's, the node takes the design's id
/// (which a save writes as the node id), and the station's life is its
/// driver - AI for a living station, none for a dead one.
fn lift_station(
    object: &ScenarioObjectConfig,
    station: &StationConfig,
    designs: &FileDesigns,
    scenario: &str,
) -> Option<LiftedShip> {
    let StationDesignSource::Prototype(id) = &station.design else {
        return None;
    };
    let design = designs.stations.get(id)?;
    let driver = if station.derelict {
        ShipDriver::Adrift
    } else {
        ShipDriver::Ai
    };
    Some(LiftedShip {
        id: node_of_design(scenario, design.id.as_str()).to_string(),
        name: object.base.name.clone(),
        driver,
        allegiance: Some(station.allegiance),
        pilot: AIControllerConfig::default(),
        pose: Transform::from_translation(object.base.position.to_engine())
            .with_rotation(object.base.rotation),
        skin: design.design.hull.presentation.skin,
        style: design.design.hull.presentation.style.clone(),
        sections: design.design.hull.sections.clone(),
        binds: BTreeMap::new(),
        station: true,
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
            station: ship.station,
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

/// Which file window the builder asked for, if any.
///
/// A request rather than a spawn: several callers ask and one builder answers.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct FileWindowRequest(pub(crate) Option<FileWindowKind>);

/// What a file window is being opened to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileWindowKind {
    /// Name a project that has no id yet, and write it.
    NameMod,
    /// Write a copy of the mod under a new name.
    SaveCopy,
    /// Say where a package went.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            dead_code,
            reason = "only Package reads this, and the web has no Package"
        )
    )]
    Packaged(String),
    /// Pick a file by path.
    PickPath(PathPurposeType),
    /// Choose what a new item of this kind starts from.
    NewAsset(&'static str),
}

/// The save shortcut. The one the File menu has always advertised.
const SAVE_KEY: KeyCode = KeyCode::KeyS;

/// File > Save. A write cues from [`save_project`], which knows how it went;
/// the window it asks for instead is a plain press.
///
/// [`save_project`]: crate::project::verbs::save_project
pub(crate) fn ask_to_save(_activate: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| {
        save_or_ask(world);
        if world.resource::<FileWindowRequest>().0.is_some() {
            accept(world);
        }
    });
}

/// File > Save As Copy.
pub(crate) fn ask_to_save_copy(_activate: On<Activate>, mut window: ResMut<FileWindowRequest>) {
    window.0 = Some(FileWindowKind::SaveCopy);
}

/// Ctrl+S: the same request the menu row raises.
pub(crate) fn save_key(mut commands: Commands, keys: Res<ButtonInput<KeyCode>>) {
    let held = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if held && keys.just_pressed(SAVE_KEY) {
        commands.queue(save_or_ask);
    }
}

/// Put a lifted document's nodes under a freshly founded scenario node.
///
/// A deferred command rather than more of the system above: the scenario node
/// it hangs everything on was spawned by that system's `Commands`, so the
/// ordinal counters it has to write are not queryable until the queue flushes.
pub(crate) fn fill_document(world: &mut World, scenario: Entity, document: LiftedDocument) {
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
pub(crate) mod tests;
