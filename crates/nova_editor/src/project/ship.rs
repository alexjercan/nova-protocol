//! A ship as a document of its own.
//!
//! Every ship opens alone at identity, with no scenario around it: a design of
//! the mod (`Content::Ship`) and an ad-hoc ship a scenario placed. An ad-hoc
//! ship's design is written by its scenario under `<scenario>.<node>`
//! ([`design_id`]), and the scenario spawns it by that id. Its document edits
//! the design and the spawn's own facts (name, keys) and writes both back to the
//! scenario, so one undo history holds both.
//!
//! The document is the scenario document's own machinery: one root, one ship
//! node under it, and the edit context standing inside that ship. What makes it
//! a ship document is [`ProjectData::open`], not the context.

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use nova_modding::prelude::Content;
use nova_scenario::prelude::{
    AIControllerConfig, EventActionConfig, ScenarioConfig, ScenarioObjectConfig,
    ScenarioObjectKind, ShipDesign, ShipDesignPrototype, ShipDesignSource, ShipPresentationConfig,
    SpaceshipConfig, StationConfig, StationDesignSource,
};

use super::{
    ids::{design_id, node_of_design},
    ModProject, OpenAsset, ProjectData,
};
use crate::{
    bundle::{fill_document, is_layout, lift_scenario, LiftedDocument, LiftedShip},
    node::{EditContext, NodeId, ObjectNode, SectionNodes, ShipDriver, ShipNode},
    scenario::{lower_ship, ship_design, spawn_of, HullForm, LoweredFleet, LoweredShip},
    station::StationCatalogs,
};

/// Whether `kind` opens as a ship document.
pub(crate) fn is_ship_kind(kind: &str) -> bool {
    matches!(kind, "ship" | "station")
}

/// The scenario of the mod whose namespace holds design `id`, the longest
/// match first: `a__b.c` belongs to `a__b`, not to `a`.
pub(crate) fn owner_scenario(data: &ProjectData, id: &str) -> Option<String> {
    data.scenarios()
        .filter(|scenario| {
            id.strip_prefix(scenario.id.as_str())
                .is_some_and(|rest| rest.starts_with('.'))
        })
        .max_by_key(|scenario| scenario.id.len())
        .map(|scenario| scenario.id.to_string())
}

/// The ship a document stands up for `open`, at identity, or why it cannot.
pub(crate) fn lift_ship_document(
    data: &ProjectData,
    open: &OpenAsset,
    old_save: impl Fn(&str) -> bool,
) -> Result<LiftedShip, String> {
    if let Some(scenario) = owner_scenario(data, &open.id) {
        let node = node_of_design(&scenario, &open.id).to_string();
        let lifted = lift_scenario(data.items(), &scenario, old_save(&scenario))
            .ok_or_else(|| format!("{scenario} is not in this mod"))?;
        let mut ship = lifted
            .ships
            .into_iter()
            .find(|ship| ship.id == node)
            .ok_or_else(|| {
                format!("{node} holds more than a ship node can show; edit it in the file")
            })?;
        ship.pose = Transform::IDENTITY;
        return Ok(ship);
    }
    match data.item(open.kind, &open.id) {
        Some(Content::Ship(design)) => Ok(LiftedShip {
            id: open.id.clone(),
            name: design.name.clone(),
            driver: ShipDriver::Adrift,
            allegiance: None,
            pilot: AIControllerConfig::default(),
            pose: Transform::IDENTITY,
            skin: design.design.presentation.skin,
            style: design.design.presentation.style.clone(),
            sections: design.design.sections.clone(),
            binds: Default::default(),
            station: false,
        }),
        Some(_) => Err(format!(
            "{} is a station design no scenario of this mod places",
            open.id
        )),
        None => Err(format!("{} is not in this mod", open.id)),
    }
}

/// Stand `ship` up under the empty root `scenario` and stand inside it.
pub(crate) fn fill_ship_document(world: &mut World, scenario: Entity, ship: LiftedShip) {
    let id = ship.id.clone();
    fill_document(
        world,
        scenario,
        LiftedDocument {
            ships: vec![ship],
            ..default()
        },
    );
    let node = world
        .query::<(Entity, &NodeId, &ChildOf)>()
        .iter(world)
        .find(|(_, held, parent)| held.0 == id && parent.parent() == scenario)
        .map(|(node, _, _)| node);
    if let Some(node) = node {
        enter_only_ship(world, node);
    }
}

/// Stand the edit context inside `ship`, and only there.
fn enter_only_ship(world: &mut World, ship: Entity) {
    let mut context = world.resource_mut::<EditContext>();
    context.exit_all();
    context.enter(ship);
}

/// Put the edit context back where the open document stands it: inside the
/// one ship of a ship document, on the root of anything else. A history step
/// diffs the nodes and leaves the context as it was, which is the wrong
/// document's after a step that crossed from one to the other.
pub(crate) fn conform_context(world: &mut World) {
    // A rig with no project has no document to conform to.
    let Some(project) = world.get_resource::<ModProject>() else {
        return;
    };
    let ship_open = project
        .data
        .open
        .as_ref()
        .is_some_and(|open| is_ship_kind(open.kind));
    let Some(root) = world
        .get_resource::<EditContext>()
        .and_then(EditContext::scenario)
    else {
        return;
    };
    if !ship_open {
        world.resource_mut::<EditContext>().exit_all();
        return;
    }
    let ship = world
        .query::<(Entity, &ChildOf, &ShipNode)>()
        .iter(world)
        .find(|(_, parent, _)| parent.parent() == root)
        .map(|(ship, _, _)| ship);
    if let Some(ship) = ship {
        if world.resource::<EditContext>().ship() != Some(ship) {
            enter_only_ship(world, ship);
        }
    }
}

/// The one ship of the open ship document, lowered at identity.
fn lowered_document_ship(
    In(()): In<()>,
    context: Res<EditContext>,
    ships: Query<(&NodeId, &ShipNode)>,
    nodes: SectionNodes,
) -> Option<LoweredShip> {
    let ship = context.ship()?;
    let (id, node) = ships.get(ship).ok()?;
    Some(lower_ship(ship, id, node, Transform::IDENTITY, &nodes))
}

/// Write the open ship document back into the mod: the design, and for an
/// ad-hoc ship the spawn its scenario places it with.
pub(crate) fn absorb_ship_document(world: &mut World, open: &OpenAsset) {
    let lowered = match world.run_system_cached_with(lowered_document_ship, ()) {
        Ok(Some(lowered)) => lowered,
        Ok(None) => return,
        Err(fault) => {
            error!("editor: the ship document could not be lowered: {fault}");
            return;
        }
    };
    let data = &world.resource::<ModProject>().data;
    let Some(scenario) = owner_scenario(data, &open.id) else {
        let Some(Content::Ship(held)) = data.item("ship", &open.id).cloned() else {
            return;
        };
        let design = ShipDesign {
            sections: lowered.sections.clone(),
            presentation: ShipPresentationConfig {
                skin: lowered.skin,
                style: lowered.style.clone(),
                ..held.design.presentation.clone()
            },
            ..held.design.clone()
        };
        let item = Content::Ship(ShipDesignPrototype {
            id: held.id.clone(),
            name: lowered.name.clone(),
            design,
        });
        if format!("{:?}", Content::Ship(held)) != format!("{item:?}") {
            world
                .resource_mut::<ModProject>()
                .edit()
                .replace_item(&open.id, item);
        }
        return;
    };
    let Some(held) = data.scenario(&scenario).cloned() else {
        return;
    };
    let mut lowered = lowered;
    let Some(spawn) = placed_spawn(&held, &open.id) else {
        return;
    };
    lowered.position = spawn.base.position.to_engine();
    lowered.rotation = spawn.base.rotation;
    let mut written = spawn_of(
        &lowered,
        HullForm::Prototype {
            scenario: &scenario,
        },
    );
    written.base.position = spawn.base.position;
    written.base.rotation = spawn.base.rotation;
    let design = if lowered.station {
        let fleet = LoweredFleet::of_one(lowered);
        let owner = scenario.clone();
        let designs = world
            .run_system_once(move |catalogs: StationCatalogs| {
                catalogs.designs(&fleet, HullForm::Prototype { scenario: &owner })
            })
            .unwrap_or_default();
        let Some((design, _)) = designs.into_iter().next() else {
            return;
        };
        Content::Station(design)
    } else {
        Content::Ship(ShipDesignPrototype {
            id: design_id(&scenario, &lowered.id).as_str().into(),
            name: lowered.id.clone(),
            design: ship_design(&lowered),
        })
    };
    let replaced = with_spawn(&held, &open.id, written);
    let data = &world.resource::<ModProject>().data;
    let design_moved = data
        .item(open.kind, &open.id)
        .is_none_or(|item| format!("{item:?}") != format!("{design:?}"));
    let spawn_moved = format!("{held:?}") != format!("{replaced:?}");
    if !design_moved && !spawn_moved {
        return;
    }
    let mut project = world.resource_mut::<ModProject>();
    let data = project.edit();
    if design_moved {
        data.replace_item(&open.id, design);
    }
    if spawn_moved {
        data.replace_item(&scenario, Content::Scenario(replaced));
    }
}

/// Whether `object` spawns the design `id`.
fn spawns_design(object: &ScenarioObjectConfig, id: &str) -> bool {
    match &object.kind {
        ScenarioObjectKind::Spaceship(SpaceshipConfig {
            design: ShipDesignSource::Prototype { id: named, .. },
            ..
        }) => named.as_str() == id,
        ScenarioObjectKind::Station(StationConfig {
            design: StationDesignSource::Prototype(named),
            ..
        }) => named.as_str() == id,
        _ => false,
    }
}

/// The layout spawn of `scenario` that places design `id`.
fn placed_spawn<'a>(scenario: &'a ScenarioConfig, id: &str) -> Option<&'a ScenarioObjectConfig> {
    crate::bundle::spawned_in_layout(scenario).find(|object| spawns_design(object, id))
}

/// `scenario` with the layout spawn of design `id` replaced by `written`.
fn with_spawn(
    scenario: &ScenarioConfig,
    id: &str,
    written: ScenarioObjectConfig,
) -> ScenarioConfig {
    let mut scenario = scenario.clone();
    for event in scenario.events.iter_mut().filter(|event| is_layout(event)) {
        for action in &mut event.actions {
            if let EventActionConfig::SpawnScenarioObject(object) = action {
                if spawns_design(object, id) {
                    *object = written.clone();
                }
            }
        }
    }
    scenario
}

/// What a ship document is called on the bar and in a Back button: the
/// spawn's name for an ad-hoc ship, the design's for a free one, or the id when
/// it has none. Read off the files, not lifted: the bar asks every frame.
pub(crate) fn ship_document_name(data: &ProjectData, open: &OpenAsset) -> Option<String> {
    let (name, id) = match owner_scenario(data, &open.id) {
        Some(scenario) => (
            placed_spawn(data.scenario(&scenario)?, &open.id)?
                .base
                .name
                .clone(),
            node_of_design(&scenario, &open.id).to_string(),
        ),
        None => match data.item(open.kind, &open.id)? {
            Content::Ship(design) => (design.name.clone(), open.id.clone()),
            _ => (String::new(), open.id.clone()),
        },
    };
    Some(if name.is_empty() { id } else { name })
}

/// The ship design a placed object spawns by reference, if it is one.
pub(crate) fn placed_design(object: &ObjectNode) -> Option<&str> {
    match &object.kind {
        ScenarioObjectKind::Spaceship(SpaceshipConfig {
            design: ShipDesignSource::Prototype { id, .. },
            ..
        }) => Some(id.as_str()),
        _ => None,
    }
}

/// Open ship node `ship` of the open scenario as a document of its own, and
/// mark `then`, a node under it, there. In the ship's own document it only
/// marks. A placed design opens the design it names.
pub(crate) fn open_ship_node(world: &mut World, ship: Entity, then: Option<Entity>) {
    if let Some(design) = world
        .get::<ObjectNode>(ship)
        .and_then(|object| placed_design(object).map(str::to_string))
    {
        super::verbs::open_asset(world, "ship", &design);
        return;
    }
    let inside = world.resource::<EditContext>().ship() == Some(ship)
        && super::DocumentType::open_in(world) == super::DocumentType::Ship;
    if inside {
        if let Some(mut selected) = world.get_resource_mut::<crate::config::SelectedNode>() {
            selected.set(then);
        }
        return;
    }
    let Some(node) = world.get::<NodeId>(ship).map(|id| id.0.clone()) else {
        return;
    };
    let station = world.get::<ShipNode>(ship).is_some_and(|ship| ship.station);
    let mut below = Vec::new();
    let mut at = then;
    while let Some(step) = at.filter(|step| *step != ship) {
        let Some(id) = world.get::<NodeId>(step) else {
            break;
        };
        below.push(id.0.clone());
        at = world.get::<ChildOf>(step).map(ChildOf::parent);
    }
    below.reverse();
    // The design id is the scenario's, which a document never lowered has not
    // minted yet.
    super::verbs::lower_open_document(world);
    let Some(scenario) = world
        .get_resource::<ModProject>()
        .and_then(|project| project.data.open_scenario())
        .map(str::to_string)
    else {
        super::verbs::refuse(world, "open a scenario to open its ships");
        return;
    };
    let kind = if station { "station" } else { "ship" };
    super::verbs::open_asset(world, kind, &design_id(&scenario, &node));
    let Some(ship) = world.resource::<EditContext>().ship() else {
        return;
    };
    let mut marked = Some(ship);
    for step in &below {
        marked = marked.and_then(|at| {
            world
                .get::<Children>(at)?
                .iter()
                .find(|child| world.get::<NodeId>(*child).is_some_and(|id| id.0 == *step))
        });
    }
    if then.is_some() {
        if let Some(mut selected) = world.get_resource_mut::<crate::config::SelectedNode>() {
            selected.set(marked);
        }
    }
}
