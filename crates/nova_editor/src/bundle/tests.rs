//! What a save is made of, and what a load gets back out of it.

use bevy::ecs::system::RunSystemOnce;
use nova_modding::prelude::serialize_content;
use nova_scenario::prelude::{
    AnchorConfig, BaseScenarioObjectConfig, PlayerControllerConfig, ScenarioEventConfig,
    SectionSource, ShipHull, SpaceshipConfig,
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
            name: EventConfig::OnStart,
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

/// A hull the editor did not write stays an object: an inline hull, or a
/// prototype belonging to some other mod. The editor cannot edit a design it
/// does not hold, so it must not offer to.
#[test]
fn a_hull_this_file_does_not_carry_stays_an_object() {
    let items = vec![
        design("ship_1"),
        scenario(vec![
            spawn(instance(
                "inline_hulk",
                ShipSource::Inline(ShipHull::default()),
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

    assert!(lifted.ships.is_empty());
    assert_eq!(lifted.objects.len(), 2);
}

/// The script is not layout: only the spawns come back, and a handler that is
/// not OnStart is not read at all.
#[test]
fn only_the_start_handlers_spawns_are_layout() {
    let items = vec![Content::Scenario(ScenarioConfig {
        events: vec![
            ScenarioEventConfig {
                name: EventConfig::OnStart,
                filters: vec![],
                actions: vec![
                    spawn(object("anchor_1")),
                    EventActionConfig::DebugMessage(
                        nova_scenario::prelude::DebugMessageActionConfig {
                            message: "script".to_string(),
                        },
                    ),
                ],
            },
            ScenarioEventConfig {
                name: EventConfig::OnDestroyed,
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
    mapping
        .input_mapping
        .insert("thruster_2".to_string(), vec![Binding::from(KeyCode::KeyW)]);
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
        Some(&vec![Binding::from(KeyCode::KeyW)])
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
                skin: true,
                style: Some("worn".to_string()),
                driver: ShipDriver::Player,
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
            binds: vec![Binding::from(KeyCode::KeyW)],
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
             q_ships: Query<(Entity, &NodeId, &ShipNode, &Transform)>| {
                document_content(
                    world_objects(&context, &q_objects),
                    &lower_fleet(&q_ships, &nodes),
                )
            },
        )
        .expect("the lowering runs")
}

/// A world with the fixture document in it and the context standing on it.
fn world_with_document() -> World {
    let mut world = World::new();
    let scenario = document(&mut world);
    let mut context = EditContext::default();
    context.path = vec![scenario];
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
        Some(&vec![Binding::from(KeyCode::KeyW)]),
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
    let mut context = EditContext::default();
    context.path = vec![scenario];
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
    let mut context = EditContext::default();
    context.path = vec![scenario];
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
