//! What a save is made of, and what a load gets back out of it.

use bevy::ecs::system::RunSystemOnce;
use nova_input::prelude::InputSource;
use nova_modding::prelude::serialize_content;
use nova_scenario::prelude::{
    AnchorConfig, BaseScenarioObjectConfig, EntityFilterConfig, EventConfig, EventFilterConfig,
    ObjectiveActionConfig, OutcomeActionConfig, PlayerControllerConfig, ScenarioEventConfig,
    ScenarioOutcomeKind, SectionSource, ShipHull, SpaceshipConfig,
};

use super::*;
use crate::node::ObjectNode;

fn section(id: &str) -> SpaceshipSectionConfig {
    SpaceshipSectionConfig {
        id: id.to_string(),
        position: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        source: SectionSource::Prototype("light_hull_section".to_string()),
        modifications: vec![],
    }
}

fn design(id: &str) -> Content {
    Content::Ship(ShipConfig {
        id: id.to_string(),
        name: id.to_string(),
        hull: ShipHull {
            sections: vec![section("hull_1")],
            skin: true,
            style: Some("worn".to_string()),
            ..default()
        },
    })
}

fn spawn(object: ScenarioObjectConfig) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(object)
}

fn object(id: &str) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: id.to_string(),
            position: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Anchor(AnchorConfig {
            body_radius: 1.0,
            mass: None,
        }),
    }
}

fn instance(id: &str, hull: ShipSource, controller: SpaceshipController) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: id.to_string(),
            position: Vec3::new(4.0, 0.0, -8.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            hull,
            controller,
            ..default()
        }),
    }
}

fn scenario(actions: Vec<EventActionConfig>) -> Content {
    Content::Scenario(ScenarioConfig {
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions,
        }],
        ..ScenarioConfig::new(
            "editor_save".to_string(),
            "Saved Range".to_string(),
            "scenarios/space.cube.png".into(),
        )
    })
}

/// A spaceship pointing at a design the same file carries is the editor's own
/// work and comes back as a ship node - with the design's structure, not a
/// copy the instance was carrying.
#[test]
fn a_spaceship_that_names_this_files_design_lifts_as_a_ship() {
    let items = vec![
        design("ship_1"),
        scenario(vec![spawn(instance(
            "player_spaceship",
            ShipSource::Prototype("ship_1".to_string()),
            SpaceshipController::Player(PlayerControllerConfig::default()),
        ))]),
    ];

    let lifted = lift_content(&items).expect("the file carries a scenario");

    assert!(lifted.objects.is_empty(), "the ship is not also an object");
    assert_eq!(lifted.ships.len(), 1);
    let ship = &lifted.ships[0];
    assert_eq!(ship.id, "ship_1", "the node is named by its design");
    assert_eq!(ship.driver, ShipDriver::Player);
    assert_eq!(ship.pose.translation, Vec3::new(4.0, 0.0, -8.0));
    assert!(ship.skin);
    assert_eq!(ship.style.as_deref(), Some("worn"));
    assert_eq!(ship.sections.len(), 1);
}

/// An INLINE hull is a ship the document can open: every section is right
/// there in the file, so the node it becomes is one a double click goes inside.
/// It takes the SPAWN's id, because that is the only name it has.
///
/// A hull naming a prototype nothing here carries stays an object. The editor
/// would have to invent the sections it cannot read, and a design it cannot
/// show is not one it should offer to edit.
#[test]
fn an_inline_hull_opens_and_a_hull_this_file_lacks_stays_an_object() {
    let items = vec![
        design("ship_1"),
        scenario(vec![
            spawn(instance(
                "inline_hulk",
                ShipSource::Inline(ShipHull {
                    sections: vec![section("spine")],
                    ..default()
                }),
                SpaceshipController::None,
            )),
            spawn(instance(
                "foreign",
                ShipSource::Prototype("some_other_mods_corvette".to_string()),
                SpaceshipController::None,
            )),
        ]),
    ];

    let lifted = lift_content(&items).expect("the file carries a scenario");

    let hulk = lifted
        .ships
        .iter()
        .find(|ship| ship.id == "inline_hulk")
        .expect("the inline hull is a ship node");
    assert_eq!(hulk.driver, ShipDriver::Adrift, "nobody is at the controls");
    assert_eq!(hulk.sections.len(), 1, "and it brought its sections");

    assert_eq!(lifted.objects.len(), 1);
    assert_eq!(lifted.objects[0].base.id, "foreign");
}

/// Layout is a handler that does nothing but place objects. A handler that
/// spawns and then talks is a beat of the script, and it keeps its spawn.
#[test]
fn only_a_pure_start_spawn_handler_is_layout() {
    let items = vec![Content::Scenario(ScenarioConfig {
        events: vec![
            ScenarioEventConfig {
                label: None,
                name: EventConfig::OnStart,
                once: false,
                filters: vec![],
                actions: vec![spawn(object("anchor_1"))],
            },
            ScenarioEventConfig {
                label: None,
                name: EventConfig::OnStart,
                once: false,
                filters: vec![],
                actions: vec![
                    spawn(object("mixed_1")),
                    EventActionConfig::DebugMessage(
                        nova_scenario::prelude::DebugMessageActionConfig {
                            message: "script".to_string(),
                        },
                    ),
                ],
            },
            ScenarioEventConfig {
                label: None,
                name: EventConfig::OnDestroyed,
                once: false,
                filters: vec![],
                actions: vec![spawn(object("wreck_1"))],
            },
        ],
        ..ScenarioConfig::new(
            "editor_save".to_string(),
            "Saved Range".to_string(),
            "scenarios/space.cube.png".into(),
        )
    })];

    let lifted = lift_content(&items).expect("the file carries a scenario");

    assert_eq!(lifted.objects.len(), 1);
    assert_eq!(lifted.objects[0].base.id, "anchor_1");
    assert_eq!(lifted.script.len(), 2, "the other two handlers are script");
    assert!(
        lifted.script.iter().all(|event| event
            .actions
            .iter()
            .any(|action| matches!(action, EventActionConfig::SpawnScenarioObject(_)))),
        "and neither of them lost the object it spawns"
    );
}

/// A content file with nothing but designs is a legal mod and not a document.
#[test]
fn a_file_with_no_scenario_is_not_a_document() {
    assert!(lift_content(&[design("ship_1")]).is_none());
}

/// The player's keys ride the instance's input mapping, which is where a
/// spawned ship reads them, and come back keyed by the same section ids.
#[test]
fn the_players_keys_come_back_on_the_sections_that_fire_them() {
    let mut mapping = PlayerControllerConfig::default();
    mapping.input_mapping.insert(
        "thruster_2".to_string(),
        vec![InputSource::from(KeyCode::KeyW)],
    );
    let items = vec![
        design("ship_1"),
        scenario(vec![spawn(instance(
            "player_spaceship",
            ShipSource::Prototype("ship_1".to_string()),
            SpaceshipController::Player(mapping),
        ))]),
    ];

    let lifted = lift_content(&items).expect("the file carries a scenario");

    let ship = &lifted.ships[0];
    assert_eq!(
        ship.binds.get("thruster_2"),
        Some(&vec![InputSource::from(KeyCode::KeyW)])
    );
}

/// The counter resumes above every id already in use, so the next mint is new.
#[test]
fn the_id_counter_resumes_above_what_is_already_there() {
    assert_eq!(
        resume_ordinal(["asteroid_2", "beacon_11", "picket_warden", "ship_1"]),
        11
    );
}

/// A document of authored ids alone - nothing minted - leaves the counter at
/// the start, which is what an untouched document has.
#[test]
fn authored_ids_alone_leave_the_counter_at_zero() {
    assert_eq!(resume_ordinal(["picket_warden", "beacon_veil"]), 0);
}

/// A document with a ship on it, ready to lower.
///
/// Built by hand rather than through `found_document`: the seeded stock range
/// is 20-odd objects of scenery, and what this exercises is the shape of the
/// file, not how much of it there is.
fn document(world: &mut World) -> Entity {
    let scenario = world
        .spawn((
            crate::node::ScenarioNode,
            NodeId("scenario".to_string()),
            NextChildOrdinal::default(),
            Transform::default(),
        ))
        .id();
    let ship = world
        .spawn((
            ShipNode {
                name: "Kestrel".to_string(),
                skin: true,
                style: Some("worn".to_string()),
                driver: ShipDriver::Player,
                ..default()
            },
            NodeId("ship_1".to_string()),
            NextChildOrdinal::default(),
            Transform::from_xyz(12.0, 0.0, -4.0),
            ChildOf(scenario),
        ))
        .id();
    world.spawn((
        SectionNode {
            source: SectionSource::Prototype("light_hull_section".to_string()),
            modifications: vec![],
            binds: vec![InputSource::from(KeyCode::KeyW)],
        },
        NodeId("hull_1".to_string()),
        Transform::from_xyz(0.0, 0.0, 1.0),
        ChildOf(ship),
    ));
    world.spawn((
        ObjectNode {
            name: "Rock".to_string(),
            kind: ScenarioObjectKind::Anchor(AnchorConfig {
                body_radius: 3.0,
                mass: None,
            }),
        },
        NodeId("anchor_4".to_string()),
        Transform::from_xyz(-30.0, 5.0, 0.0),
        ChildOf(scenario),
    ));
    scenario
}

/// Lower whatever document `world` holds into content items.
fn lower(world: &mut World) -> Vec<Content> {
    world
        .run_system_once(
            |context: Res<EditContext>,
             nodes: SectionNodes,
             q_objects: ObjectNodes,
             script: ScriptNodes,
             q_ships: Query<(Entity, &NodeId, &ShipNode, &Transform)>| {
                document_content(
                    world_objects(&context, &q_objects),
                    &lower_fleet(&q_ships, &nodes),
                    world_script(&context, &script),
                )
            },
        )
        .expect("the lowering runs")
}

/// A document the builder has not finished: one AI escort with nothing built
/// on it, and no ship they fly.
fn unflown_document(world: &mut World) -> Entity {
    let scenario = world
        .spawn((
            crate::node::ScenarioNode,
            NodeId("scenario".to_string()),
            NextChildOrdinal::default(),
            Transform::default(),
        ))
        .id();
    world.spawn((
        ShipNode {
            name: "Escort".to_string(),
            driver: ShipDriver::Ai,
            ..default()
        },
        NodeId("ship_2".to_string()),
        NextChildOrdinal::default(),
        Transform::from_xyz(0.0, 0.0, -20.0),
        ChildOf(scenario),
    ));
    scenario
}

/// A world with the unfinished document in it and the context standing on it.
fn world_with_unflown_document() -> World {
    let mut world = World::new();
    let scenario = unflown_document(&mut world);
    world.insert_resource(EditContext {
        path: vec![scenario],
    });
    world
}

/// A save is the DOCUMENT, not a flight. Play hands the runtime a hull to sit
/// in when the builder has not made one, and writing that hull to the file
/// would give them back a ship node they never added.
#[test]
fn a_save_never_invents_the_player_ship_the_document_lacks() {
    let mut world = world_with_unflown_document();

    let lifted = lift_content(&lower(&mut world)).expect("the file carries a range");

    let ids: Vec<&str> = lifted.ships.iter().map(|ship| ship.id.as_str()).collect();
    assert_eq!(
        ids,
        ["ship_2"],
        "only the ship the builder added comes back"
    );
    assert!(
        lifted
            .objects
            .iter()
            .all(|object| object.base.id != "player_spaceship"),
        "and no hull the editor invented for Play"
    );
}

/// A blank Add Ship is a decision not yet made. Play spawns nothing for it,
/// but the node is the builder's place in the work and the file has to hold it.
#[test]
fn a_ship_with_nothing_built_on_it_survives_the_file() {
    let mut world = world_with_unflown_document();

    let lifted = lift_content(&lower(&mut world)).expect("the file carries a range");

    let ship = &lifted.ships[0];
    assert_eq!(ship.driver, ShipDriver::Ai);
    assert!(ship.sections.is_empty(), "still nothing built on it");
    assert_eq!(ship.pose.translation, Vec3::new(0.0, 0.0, -20.0));
}

/// A world with the fixture document in it and the context standing on it.
fn world_with_document() -> World {
    let mut world = World::new();
    let scenario = document(&mut world);
    let context = EditContext {
        path: vec![scenario],
    };
    world.insert_resource(context);
    world
}

/// The whole round trip in memory: what the editor holds, written as content,
/// read back, and holding the same things. This is the property the save was
/// built for - the document survives the file.
#[test]
fn a_document_survives_being_written_and_read_back() {
    let mut world = world_with_document();

    let lifted = lift_content(&lower(&mut world)).expect("the file carries a range");

    assert_eq!(lifted.ships.len(), 1);
    let ship = &lifted.ships[0];
    assert_eq!(ship.id, "ship_1");
    assert_eq!(ship.driver, ShipDriver::Player);
    assert_eq!(ship.pose.translation, Vec3::new(12.0, 0.0, -4.0));
    assert!(ship.skin);
    assert_eq!(ship.style.as_deref(), Some("worn"));
    assert_eq!(ship.sections.len(), 1);
    assert_eq!(ship.sections[0].id, "hull_1");
    assert_eq!(ship.sections[0].position, Vec3::new(0.0, 0.0, 1.0));
    assert_eq!(
        ship.binds.get("hull_1"),
        Some(&vec![InputSource::from(KeyCode::KeyW)]),
        "the keys a section fires on ride the instance's input mapping"
    );

    let anchor = lifted
        .objects
        .iter()
        .find(|object| object.base.id == "anchor_4")
        .expect("the world's own object comes back");
    assert_eq!(anchor.base.position, Vec3::new(-30.0, 5.0, 0.0));
    assert_eq!(anchor.base.name, "Rock");
}

/// The file holds each design ONCE. The instance that spawns it carries a
/// reference, so editing the design changes what every instance flies and the
/// RON never carries two copies of one hull.
#[test]
fn a_design_is_written_once_and_referenced() {
    let mut world = world_with_document();

    let items = lower(&mut world);

    let designs: Vec<&str> = items
        .iter()
        .filter_map(|item| match item {
            Content::Ship(ship) => Some(ship.id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(designs, vec!["ship_1"], "one design, under the node's id");

    let Some(Content::Scenario(scenario)) = items
        .iter()
        .find(|item| matches!(item, Content::Scenario(_)))
    else {
        panic!("the file carries a scenario");
    };
    let hulls: Vec<&ShipSource> = scenario
        .events
        .iter()
        .flat_map(|event| &event.actions)
        .filter_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object) => match &object.kind {
                ScenarioObjectKind::Spaceship(spawn) => Some(&spawn.hull),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert!(
        hulls
            .iter()
            .all(|hull| matches!(hull, ShipSource::Prototype(_))),
        "every spawned ship references a design rather than carrying one"
    );
}

/// Save, load, save: the same bytes. A file that reordered or re-derived itself
/// would diff against itself on every write, which is the one thing a text save
/// format must not do.
#[test]
fn a_re_save_writes_the_same_bytes() {
    let mut world = world_with_document();
    let first = serialize_content(&lower(&mut world)).expect("serialize");

    // The load, in full: a new document founded from the file's own contents.
    let lifted = lift_content(&parse_of(&first)).expect("the file carries a range");
    let mut reloaded = World::new();
    let scenario = reloaded
        .spawn((
            crate::node::ScenarioNode,
            NodeId("scenario".to_string()),
            NextChildOrdinal::default(),
            Transform::default(),
        ))
        .id();
    let context = EditContext {
        path: vec![scenario],
    };
    reloaded.insert_resource(context);
    fill_document(&mut reloaded, scenario, lifted);

    let second = serialize_content(&lower(&mut reloaded)).expect("serialize");
    assert_eq!(
        second, first,
        "a document reloaded from a file re-saves to it"
    );
}

/// Parse a body written by [`serialize_content`].
fn parse_of(body: &str) -> Vec<Content> {
    nova_modding::prelude::parse_content(body.as_bytes()).expect("the written file parses")
}

/// A load restores the id counters above every id the file already used, so the
/// next part placed cannot silently take the id of one that is already there.
#[test]
fn a_loaded_document_mints_ids_above_the_ones_it_read() {
    let mut world = world_with_document();
    let lifted = lift_content(&lower(&mut world)).expect("the file carries a range");

    let mut reloaded = World::new();
    let scenario = reloaded
        .spawn((
            crate::node::ScenarioNode,
            NodeId("scenario".to_string()),
            NextChildOrdinal::default(),
            Transform::default(),
        ))
        .id();
    let context = EditContext {
        path: vec![scenario],
    };
    reloaded.insert_resource(context);
    fill_document(&mut reloaded, scenario, lifted);

    assert_eq!(
        reloaded
            .entity(scenario)
            .get::<NextChildOrdinal>()
            .map(|n| n.0),
        Some(4),
        "the world's counter resumes above 'anchor_4'"
    );
    let ship = reloaded
        .query::<(Entity, &NodeId)>()
        .iter(&reloaded)
        .find(|(_, id)| id.0 == "ship_1")
        .map(|(entity, _)| entity)
        .expect("the ship came back");
    assert_eq!(
        reloaded.entity(ship).get::<NextChildOrdinal>().map(|n| n.0),
        Some(1),
        "and the ship's resumes above 'hull_1'"
    );
}

/// Editing a design does not touch the instances that stand on it (the Godot
/// #67884 lesson, in the shape this format takes).
///
/// The lesson there was inherited scenes drifting: an instance quietly kept a
/// copy of a field the source had moved on from. This format has no room for
/// that drift, and the test says so by diffing two lowerings of the same
/// document: the DESIGN changed, and the instance's bytes did not, because an
/// instance holds a reference, a pose and its controller - never a copy.
#[test]
fn editing_a_design_leaves_its_instances_untouched() {
    let mut world = world_with_document();
    let before = lower(&mut world);

    let mut sections = world.query::<&mut SectionNode>();
    for mut section in sections.query_mut(&mut world) {
        section.source = SectionSource::Prototype("reinforced_hull_section".to_string());
    }
    let after = lower(&mut world);

    let design = |items: &[Content]| {
        items
            .iter()
            .find_map(|item| match item {
                Content::Ship(ship) => Some(serialize_content(&[Content::Ship(ship.clone())])),
                _ => None,
            })
            .expect("the file carries the design")
            .expect("a design serialises")
    };
    let range = |items: &[Content]| {
        items
            .iter()
            .find_map(|item| match item {
                Content::Scenario(range) => {
                    Some(serialize_content(&[Content::Scenario(range.clone())]))
                }
                _ => None,
            })
            .expect("the file carries the range")
            .expect("a range serialises")
    };

    assert_ne!(
        design(&before),
        design(&after),
        "the edit landed on the design"
    );
    assert_eq!(
        range(&before),
        range(&after),
        "an instance names its design and carries no copy of it, so the range \
         is byte-identical across an edit to the design it points at"
    );
}

/// The objective set of the task's own example, as the editor holds it: a
/// handler that retires, scoped to the ship the document flies, that posts an
/// objective and declares the outcome.
///
/// The ship is named by the id the RANGE gives it - the flown hull spawns as
/// `player_spaceship` however its node is called - which is the id the
/// inspector's picker offers and the one the drop below is judged against.
fn authored_script(id: &str) -> Vec<ScenarioEventConfig> {
    vec![ScenarioEventConfig {
        label: None,
        name: EventConfig::OnDestroyed,
        once: true,
        filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
            id: Some(id.to_string()),
            ..default()
        })],
        actions: vec![
            EventActionConfig::Objective(ObjectiveActionConfig::new("kill", "Destroy the escort")),
            EventActionConfig::Outcome(OutcomeActionConfig::new(
                ScenarioOutcomeKind::Victory,
                "Escort down",
            )),
        ],
    }]
}

/// A document holding the authored script, lifted into nodes the way an opened
/// file arrives.
fn world_with_script(id: &str) -> World {
    let mut world = world_with_document();
    let scenario = world
        .query_filtered::<Entity, With<crate::node::ScenarioNode>>()
        .single(&world)
        .expect("one scenario node");
    {
        let mut commands = world.commands();
        crate::event::lift(&mut commands, scenario, authored_script(id));
    }
    world.flush();
    world
}

/// The SCRIPT survives the file, objectives and outcome included: the same
/// property the layout has, for the half of the document the editor learned to
/// author last.
///
/// Through the nodes both ways - lifted in, lowered out - because that is what
/// a builder's save actually is: the tree, written down.
#[test]
fn a_script_the_editor_wrote_survives_the_file() {
    let mut world = world_with_script("player_spaceship");

    let lifted = lift_content(&lower(&mut world)).expect("the file carries a range");

    assert_eq!(
        lifted.script.len(),
        1,
        "the derived layout handler is the world, not the script"
    );
    assert_eq!(
        format!("{:?}", lifted.script[0]),
        format!("{:?}", authored_script("player_spaceship")[0]),
        "the handler comes back as it was authored"
    );
}

/// A handler naming an id the document does not spawn does not reach the file.
///
/// The loader REFUSES a scenario over a dangling reference, so the lowering
/// drops the handler rather than writing a range that cannot be opened - which
/// is why the panel paints that row as a fault while it is still being typed.
#[test]
fn a_handler_naming_nothing_the_document_spawns_is_dropped() {
    let mut world = world_with_script("ship_9");

    let lifted = lift_content(&lower(&mut world)).expect("the file carries a range");

    assert!(
        lifted.script.is_empty(),
        "nothing in the document is called ship_9: {:?}",
        lifted.script
    );
}
