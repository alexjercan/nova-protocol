//! The panel as a LIVE TREE: what a rebuild does to the widgets, and what a
//! submitted field does to the document. The row model itself is tested in
//! `crate::inspect`; these tests are about the reconciler around it.

use bevy::{
    camera::NormalizedRenderTarget,
    ecs::system::RunSystemOnce,
    math::Affine2,
    picking::pointer::{Location, PointerId},
    window::WindowResolution,
};
use nova_assets::prelude::EnabledMods;
use nova_gameplay::prelude::Allegiance;
use nova_modding::prelude::{BundleAsset, CatalogEntry, InstalledCatalog, ModEntry, ModMeta};
use nova_scenario::prelude::{
    AIControllerConfig, AsteroidConfig, BeaconConfig, EntityFilterConfig, EventActionConfig,
    ScenarioObjectKind, SectionSource, SpaceshipConfig, SpaceshipController,
    StoryMessageActionConfig, KIND_ROCK,
};
use nova_ship::prelude::{
    BaseSectionConfig, MuzzleConfig, SectionConfig, SectionKind, ThrusterSectionConfig,
    TurretJoint, TurretSectionConfig,
};

use super::*;
use crate::{
    event::{
        ActionChoice, ActionKind, ActionNode, ExprChoice, ExprKind, FilterChoice, FilterKind,
        FilterNode,
    },
    node::{EditorNode, NextChildOrdinal, ScenarioNode},
};

/// A panel with the reconciler running, over a document holding one scenario
/// node. The tests below hang things off it.
fn inspector_app() -> App {
    let mut app = App::new();
    app.insert_resource(UiSkin::default());
    app.init_resource::<SelectedNode>();
    // The panel reads the View menu's own toggles: CURATED unless All Fields
    // is on, which is the state a fresh editor opens in.
    app.init_resource::<EditorOverlays>();
    app.add_message::<TextFieldSubmitted>();
    // The write-back announces a stale object body rather than leaning on
    // `Changed<ObjectNode>`, so the rig carries that message too.
    app.add_message::<ObjectBodyStale>();
    // A row that can REFUSE says why on the status line, so the rig carries
    // the line and the clock that expires it.
    app.init_resource::<crate::config::EditorStatus>();
    app.init_resource::<Time>();
    app.world_mut().spawn(inspector_panel(UiSkin::default()));
    app.add_systems(Update, sync_inspector);
    app
}

/// One scenario node, entered - the context every other node hangs under.
fn document(app: &mut App) -> Entity {
    let scenario = app
        .world_mut()
        .spawn((
            EditorNode,
            ScenarioNode::default(),
            NodeId("scenario".to_string()),
            NextChildOrdinal::default(),
        ))
        .id();
    app.world_mut().insert_resource(EditContext {
        path: vec![scenario],
    });
    scenario
}

fn asteroid(app: &mut App, scenario: Entity, id: &str, radius: Meters) -> Entity {
    app.world_mut()
        .spawn((
            EditorNode,
            ObjectNode {
                name: id.to_string(),
                kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                    radius,
                    texture: default(),
                    material: KIND_ROCK.to_string(),
                    destroy_sound: None,
                    mass: None,
                    invulnerable: false,
                    seed: None,
                    lock_signature: None,
                }),
            },
            NodeId(id.to_string()),
            Transform::default(),
            ChildOf(scenario),
        ))
        .id()
}

fn beacon(app: &mut App, scenario: Entity, id: &str) -> Entity {
    app.world_mut()
        .spawn((
            EditorNode,
            ObjectNode {
                name: id.to_string(),
                kind: ScenarioObjectKind::Beacon(BeaconConfig {
                    label: "BEACON".to_string(),
                    radius: Meters(30.0),
                    color: Color::WHITE,
                    area_radius: None,
                    lock_signature: None,
                }),
            },
            NodeId(id.to_string()),
            Transform::default(),
            ChildOf(scenario),
        ))
        .id()
}

fn select(app: &mut App, node: Entity) {
    app.world_mut().resource_mut::<SelectedNode>().0 = Some(node);
    app.update();
}

/// Every row's name, in draw order.
fn row_names(app: &mut App) -> Vec<String> {
    let list = app
        .world_mut()
        .query_filtered::<Entity, With<InspectorList>>()
        .single(app.world())
        .expect("one inspector list");
    let rows: Vec<Entity> = app
        .world()
        .get::<Children>(list)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    rows.into_iter()
        .filter_map(|row| app.world().get::<Name>(row))
        .map(|name| name.as_str().replace("Inspector Row ", ""))
        .collect()
}

/// The text field of the row called `label`.
fn field_of(app: &mut App, label: &str) -> Entity {
    let wanted = format!("Inspector Field {label}");
    app.world_mut()
        .query::<(Entity, &Name, &InspectorField)>()
        .iter(app.world())
        .find(|(_, name, _)| name.as_str() == wanted)
        .map(|(entity, ..)| entity)
        .unwrap_or_else(|| panic!("no field {label:?}"))
}

/// Commit `text` into the field of `label`, the way Enter does.
fn submit(app: &mut App, label: &str, text: &str) -> Entity {
    let entity = field_of(app, label);
    app.world_mut().write_message(TextFieldSubmitted {
        entity,
        value: text.to_string(),
    });
    app.world_mut()
        .run_system_once(apply_inspector_edits)
        .expect("the write-back runs");
    app.update();
    entity
}

/// What the readout of the row called `label` says.
fn readout_of(app: &mut App, label: &str) -> String {
    let wanted = format!("Inspector Readout {label}");
    app.world_mut()
        .query::<(&Name, &Text)>()
        .iter(app.world())
        .find(|(name, _)| name.as_str() == wanted)
        .map(|(_, text)| text.0.clone())
        .unwrap_or_else(|| panic!("no readout {label:?}"))
}

/// What the unit slot of the row called `label` says.
fn unit_of(app: &mut App, label: &str) -> String {
    let wanted = format!("Inspector Unit {label}");
    app.world_mut()
        .query::<(&Name, &Text, &InspectorUnit)>()
        .iter(app.world())
        .find(|(name, ..)| name.as_str() == wanted)
        .map(|(_, text, _)| text.0.clone())
        .unwrap_or_else(|| panic!("no unit slot {label:?}"))
}

fn radius_of(app: &App, object: Entity) -> Meters {
    match &app
        .world()
        .get::<ObjectNode>(object)
        .expect("an object node")
        .kind
    {
        ScenarioObjectKind::Asteroid(config) => config.radius,
        other => panic!("not an asteroid: {other:?}"),
    }
}

#[test]
fn the_panel_lists_the_fields_of_the_node_it_is_on() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));

    select(&mut app, rock);

    let names = row_names(&mut app);
    assert!(
        names.contains(&"Radius".to_string()) && names.contains(&"Position".to_string()),
        "a rock's own config drives the rows: {names:?}"
    );
    let title = app
        .world_mut()
        .query_filtered::<&Text, With<InspectorTitle>>()
        .single(app.world())
        .expect("one title")
        .0
        .clone();
    assert!(
        title.contains("OBJECT") && title.contains("asteroid_1"),
        "{title}"
    );
}

/// An event's filter names the node it fires on by ID, so the panel says that
/// id rather than leaving it to a hover over the right tree row.
#[test]
fn the_panel_says_the_id_an_event_would_name_it_by() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_7", Meters(30.0));

    select(&mut app, rock);

    let said = app
        .world_mut()
        .query_filtered::<&TextSpan, With<InspectorId>>()
        .single(app.world())
        .expect("one id line")
        .0
        .clone();
    assert_eq!(
        said, "\nasteroid_7",
        "the whole id, on its own line under the title"
    );
}

/// A seeded hull is filed with the rocks only because of how a scenario stores
/// it. The panel is the one place the reader is asked what they are looking at,
/// so it says SHIP - and it opens on which hull and who flies it, not on the
/// object machinery underneath.
#[test]
fn a_seeded_hull_is_inspected_as_a_ship() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let picket = app
        .world_mut()
        .spawn((
            EditorNode,
            ObjectNode {
                name: "Picket Warden".to_string(),
                kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                    controller: SpaceshipController::AI(AIControllerConfig::default()),
                    ..default()
                }),
            },
            NodeId("spaceship_1".to_string()),
            Transform::default(),
            ChildOf(scenario),
        ))
        .id();

    select(&mut app, picket);

    let title = app
        .world_mut()
        .query_filtered::<&Text, With<InspectorTitle>>()
        .single(app.world())
        .expect("one title")
        .0
        .clone();
    assert!(
        title.contains("SHIP") && title.contains("Picket Warden"),
        "{title}"
    );
    let names = row_names(&mut app);
    assert!(
        names.contains(&"Hull".to_string()),
        "which hull is the whole point of a seeded ship: {names:?}"
    );
}

#[test]
fn inspecting_another_node_rebuilds_the_rows() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    let nav = beacon(&mut app, scenario, "beacon_1");

    select(&mut app, rock);
    assert!(row_names(&mut app).contains(&"Invulnerable".to_string()));

    select(&mut app, nav);
    let names = row_names(&mut app);
    assert!(
        names.contains(&"Label".to_string()) && !names.contains(&"Invulnerable".to_string()),
        "the beacon's rows replaced the rock's: {names:?}"
    );
}

/// Two rocks have the SAME rows, and the widgets still have to be rebuilt:
/// each one carries the entity it writes to.
#[test]
fn inspecting_a_second_node_of_the_same_kind_rebuilds_too() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let first = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    let second = asteroid(&mut app, scenario, "asteroid_2", Meters(30.0));

    select(&mut app, first);
    select(&mut app, second);
    submit(&mut app, "Radius", "90");

    assert!((radius_of(&app, second) - Meters(90.0)).get().abs() < f32::EPSILON);
    assert!(
        (radius_of(&app, first) - Meters(30.0)).get().abs() < f32::EPSILON,
        "the first rock is not what the panel is on"
    );
}

/// The panel dies with the editor scene and the reconciler's `Local` does not.
/// A second visit to an unchanged document must still get its rows.
#[test]
fn a_returning_panel_gets_its_rows_back() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    select(&mut app, rock);
    assert!(!row_names(&mut app).is_empty());

    let panel = app
        .world_mut()
        .query_filtered::<Entity, With<InspectorPanel>>()
        .single(app.world())
        .expect("one panel");
    app.world_mut().entity_mut(panel).despawn();
    app.world_mut().spawn(inspector_panel(UiSkin::default()));
    app.update();

    assert!(
        row_names(&mut app).contains(&"Radius".to_string()),
        "the shape the Local remembers belongs to a list that no longer exists"
    );
}

#[test]
fn a_submitted_field_moves_the_number_into_the_document() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    select(&mut app, rock);

    // The box and the file are both meters: what is typed is what is kept.
    let field = submit(&mut app, "Radius", "125");

    assert!((radius_of(&app, rock) - Meters(125.0)).get().abs() < f32::EPSILON);
    assert!(
        app.world().get::<TextFieldError>(field).is_none(),
        "a value the field took carries no error"
    );
    assert_eq!(
        app.world()
            .get::<TextFieldValue>(field)
            .expect("the field")
            .0,
        "125",
        "the box repaints from the document it just wrote"
    );
}

#[test]
fn a_refused_value_marks_the_field_and_leaves_the_document_alone() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    select(&mut app, rock);

    let field = submit(&mut app, "Radius", "big");

    assert!((radius_of(&app, rock) - Meters(30.0)).get().abs() < f32::EPSILON);
    assert!(
        app.world().get::<TextFieldError>(field).is_some(),
        "the builder typed it, so the builder is told"
    );
}

/// A radius has a floor, and the floor is enforced where the number is TYPED.
/// Until now a negative one was taken here and found out at spawn time, with
/// the range already flying.
#[test]
fn a_negative_radius_is_refused_in_the_box_it_was_typed_in() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    select(&mut app, rock);

    let field = submit(&mut app, "Radius", "-4");

    assert!(
        (radius_of(&app, rock) - Meters(30.0)).get().abs() < f32::EPSILON,
        "the document keeps the radius it had"
    );
    let error = app
        .world()
        .get::<TextFieldError>(field)
        .expect("the box says why")
        .0
        .clone();
    assert_eq!(error, "min 0");
}

/// A red border says NO; it does not say why. The unit slot is the only space
/// on the row that is not the next row's, so the reason takes it - and the
/// refused number stays in the box, because a builder corrects a number they
/// can still see.
#[test]
fn a_refusal_takes_the_unit_slot_and_the_box_keeps_what_was_typed() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    select(&mut app, rock);
    app.world_mut()
        .run_system_once(paint_field_reasons)
        .expect("the reason paints");
    assert_eq!(unit_of(&mut app, "Radius"), "m", "nothing is wrong yet");

    let field = field_of(&mut app, "Radius");
    app.world_mut()
        .entity_mut(field)
        .insert(TextFieldValue("-4".to_string()));
    submit(&mut app, "Radius", "-4");
    app.world_mut()
        .run_system_once(paint_field_reasons)
        .expect("the reason paints");

    assert_eq!(unit_of(&mut app, "Radius"), "min 0");
    assert_eq!(
        app.world()
            .get::<TextFieldValue>(field)
            .expect("the field")
            .0,
        "-4",
        "the refused number stays there to be corrected"
    );
}

#[test]
fn a_moved_node_repaints_its_own_position_row() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    select(&mut app, rock);

    // A drag on the stage writes the pose; the panel is a readout of it.
    app.world_mut()
        .entity_mut(rock)
        .insert(Transform::from_xyz(4.0, 0.0, -6.0));
    app.update();

    for (axis, wanted) in [("X", "40"), ("Y", "0"), ("Z", "-60")] {
        let box_of = field_of(&mut app, &format!("Position {axis}"));
        assert_eq!(
            app.world()
                .get::<TextFieldValue>(box_of)
                .expect("the field")
                .0,
            wanted,
            "the {axis} box repaints from the pose"
        );
    }
}

/// Each box writes ONE number. Typing into Y must not disturb X and Z, which
/// is the whole difference between three boxes and one comma-separated field.
#[test]
fn typing_into_one_axis_box_leaves_the_others_alone() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    select(&mut app, rock);
    app.world_mut()
        .entity_mut(rock)
        .insert(Transform::from_xyz(4.0, 0.0, -6.0));
    app.update();

    submit(&mut app, "Position Y", "90");

    assert_eq!(
        app.world()
            .get::<Transform>(rock)
            .expect("the rock")
            .translation,
        Vec3::new(4.0, 9.0, -6.0)
    );
}

#[test]
fn a_focused_field_is_not_overwritten_by_the_document() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    select(&mut app, rock);

    let field = field_of(&mut app, "Radius");
    app.world_mut().entity_mut(field).insert((
        TextFieldFocused::at_end("3"),
        TextFieldValue("31".to_string()),
    ));
    app.update();

    assert_eq!(
        app.world()
            .get::<TextFieldValue>(field)
            .expect("the field")
            .0,
        "31",
        "half a typed number is not a document value to repaint over"
    );
}

#[test]
fn a_ship_hands_itself_to_the_ai_from_its_driver_row() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let ship = app
        .world_mut()
        .spawn((
            EditorNode,
            ShipNode::default(),
            NodeId("ship_1".to_string()),
            Transform::default(),
            ChildOf(scenario),
        ))
        .id();
    app.world_mut().resource_mut::<EditContext>().enter(ship);
    app.update();

    let option = app
        .world_mut()
        .query::<(Entity, &InspectorDriver)>()
        .iter(app.world())
        .find(|(_, option)| option.driver == ShipDriver::Ai)
        .map(|(entity, _)| entity)
        .expect("an AI option");
    app.world_mut().trigger(Activate { entity: option });
    app.update();

    assert_eq!(
        app.world().get::<ShipNode>(ship).expect("the ship").driver,
        ShipDriver::Ai,
        "entering a ship clears the selection, so the driver row is reached through the context"
    );
    assert_eq!(
        app.world()
            .get::<ShipNode>(ship)
            .expect("the ship")
            .allegiance,
        Some(Allegiance::Neutral),
        "and the side goes with the controls: the engine's default for an \
         unstated AI allegiance is ENEMY, so a hull handed to a pilot would \
         otherwise open fire"
    );

    let option = app
        .world_mut()
        .query::<(Entity, &InspectorDriver)>()
        .iter(app.world())
        .find(|(_, option)| option.driver == ShipDriver::Adrift)
        .map(|(entity, _)| entity)
        .expect("a driverless option");
    app.world_mut().trigger(Activate { entity: option });
    app.update();

    let hull = app.world().get::<ShipNode>(ship).expect("the ship");
    assert_eq!(hull.driver, ShipDriver::Adrift);
    assert_eq!(
        hull.allegiance, None,
        "a hull nobody drives is on nobody's side"
    );
}

/// One ship flies. Lowering keeps the LAST Player ship it reads and routes the
/// rest to the standing fleet, so a second one is a ship the document loses on
/// the next save.
#[test]
fn a_second_ship_cannot_take_the_controls_while_another_flies() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    app.world_mut().spawn((
        EditorNode,
        ShipNode {
            name: "Kestrel".to_string(),
            driver: ShipDriver::Player,
            ..default()
        },
        NodeId("ship_1".to_string()),
        Transform::default(),
        ChildOf(scenario),
    ));
    let escort = app
        .world_mut()
        .spawn((
            EditorNode,
            ShipNode {
                driver: ShipDriver::Ai,
                ..default()
            },
            NodeId("ship_2".to_string()),
            Transform::default(),
            ChildOf(scenario),
        ))
        .id();
    app.world_mut().resource_mut::<EditContext>().enter(escort);
    app.update();

    let option = app
        .world_mut()
        .query::<(Entity, &InspectorDriver)>()
        .iter(app.world())
        .find(|(_, option)| option.driver == ShipDriver::Player)
        .map(|(entity, _)| entity)
        .expect("a player option");
    app.world_mut().trigger(Activate { entity: option });
    app.update();

    assert_eq!(
        app.world()
            .get::<ShipNode>(escort)
            .expect("the escort")
            .driver,
        ShipDriver::Ai,
        "the escort keeps its pilot"
    );
    let (line, _) = app
        .world()
        .resource::<crate::config::EditorStatus>()
        .line()
        .expect("the refusal is said out loud");
    assert_eq!(
        line, "Kestrel already flies - set it to AI first",
        "and the reason names the ship in the way and the way out"
    );
}

/// The panel used to go blank at the root, which reads as the panel breaking
/// every time you leave a ship. The root holds the document, so it says what
/// the document holds.
#[test]
fn the_scenario_node_counts_what_the_document_holds() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    asteroid(&mut app, scenario, "asteroid_2", Meters(30.0));
    app.world_mut().spawn((
        EditorNode,
        ShipNode {
            name: "Kestrel".to_string(),
            driver: ShipDriver::Player,
            ..default()
        },
        NodeId("ship_1".to_string()),
        Transform::default(),
        ChildOf(scenario),
    ));
    app.update();

    let rows = row_names(&mut app);
    assert_eq!(
        rows,
        [
            "Ships",
            "Objects",
            "Player Ship",
            "Name",
            "Description",
            "Cubemap",
            "Skybox Brightness",
        ],
        "the counts it holds, then the fields it authors: {rows:?}"
    );
    assert_eq!(readout_of(&mut app, "Ships"), "1");
    assert_eq!(readout_of(&mut app, "Objects"), "2");
    assert_eq!(
        readout_of(&mut app, "Player Ship"),
        "Kestrel",
        "and which one Play would hand over"
    );
}

/// The root is a node like any other: what it says about the range is typed
/// into it, not baked into the save. The rows come off `ScenarioNode` by the
/// same walk every other node gets, so a write lands on the component.
#[test]
fn the_scenario_root_is_typed_into_like_any_other_node() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    app.update();

    assert_eq!(
        unit_of(&mut app, "Skybox Brightness"),
        "lx",
        "a lux field says so"
    );
    submit(&mut app, "Name", "Ashfall Belt");
    submit(&mut app, "Skybox Brightness", "250");

    let settings = app
        .world()
        .get::<ScenarioNode>(scenario)
        .expect("the root carries its settings");
    assert_eq!(settings.name, "Ashfall Belt");
    assert_eq!(settings.skybox_brightness, 250.0);
}

/// An object could be renamed and a ship could not, so a fleet of designs read
/// as a column of minted ids.
#[test]
fn a_ship_takes_the_name_you_type_into_it() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let ship = app
        .world_mut()
        .spawn((
            EditorNode,
            ShipNode::default(),
            NodeId("ship_1".to_string()),
            Transform::default(),
            ChildOf(scenario),
        ))
        .id();
    app.world_mut().resource_mut::<EditContext>().enter(ship);
    app.update();

    submit(&mut app, "Name", "Kestrel");

    assert_eq!(
        app.world().get::<ShipNode>(ship).expect("the ship").name,
        "Kestrel"
    );
}

/// The Key row IS the rebind. The binding was named on one surface and armed
/// from another, which left the row as text beside a verb in the top bar.
#[test]
fn pressing_the_key_row_arms_the_rebind() {
    let mut app = inspector_app();
    app.init_resource::<crate::keybind::EditorRebind>();
    app.init_resource::<nova_ui::prelude::InputMode>();
    app.add_observer(on_rebind_action);
    let scenario = document(&mut app);
    let ship = app
        .world_mut()
        .spawn((
            EditorNode,
            ShipNode::default(),
            NodeId("ship_1".to_string()),
            Transform::default(),
            ChildOf(scenario),
        ))
        .id();
    app.world_mut().resource_mut::<EditContext>().enter(ship);
    let thruster = app
        .world_mut()
        .spawn((
            EditorNode,
            SectionNode {
                source: SectionSource::Inline(SectionConfig {
                    base: BaseSectionConfig {
                        id: "thruster".to_string(),
                        ..default()
                    },
                    kind: SectionKind::Thruster(ThrusterSectionConfig {
                        magnitude: 40.0,
                        ..default()
                    }),
                }),
                modifications: vec![],
                binds: vec![],
            },
            NodeId("thruster_section_1".to_string()),
            Transform::default(),
            ChildOf(ship),
        ))
        .id();
    select(&mut app, thruster);

    let chip = app
        .world_mut()
        .query_filtered::<Entity, With<InspectorKey>>()
        .single(app.world())
        .expect("one key chip");
    app.world_mut().trigger(Activate { entity: chip });
    app.update();

    assert_eq!(
        app.world()
            .resource::<crate::keybind::EditorRebind>()
            .target,
        Some(thruster),
        "the row a builder read the key off is the row that changes it"
    );
}

/// The panel draws a group PATH as a tree: one line per level, and only the
/// levels the row above did not already say. The flat version repeated the
/// whole path over every handful of rows - "Root Children 1", then "Root
/// Children 1 Muzzle" - which is the wall of words the split was meant to
/// remove.
#[test]
fn a_nested_group_is_drawn_one_level_at_a_time() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let ship = app
        .world_mut()
        .spawn((
            EditorNode,
            ShipNode::default(),
            NodeId("ship_1".to_string()),
            Transform::default(),
            ChildOf(scenario),
        ))
        .id();
    let joint = |muzzle, children| TurretJoint {
        name: None,
        offset: Vec3::ZERO,
        axis: None,
        speed: 0.0,
        min: None,
        max: None,
        render_mesh: None,
        render_mesh_transform: None,
        muzzle,
        children,
    };
    let section = app
        .world_mut()
        .spawn((
            EditorNode,
            SectionNode {
                source: SectionSource::Inline(SectionConfig {
                    base: BaseSectionConfig {
                        id: "turret".to_string(),
                        ..default()
                    },
                    kind: SectionKind::Turret(TurretSectionConfig {
                        root: joint(
                            None,
                            vec![joint(
                                Some(MuzzleConfig {
                                    fire_rate: 4.0,
                                    muzzle_effect: None,
                                }),
                                Vec::new(),
                            )],
                        ),
                        ..default()
                    }),
                }),
                modifications: vec![],
                binds: vec![],
            },
            NodeId("turret_1".to_string()),
            Transform::default(),
            ChildOf(ship),
        ))
        .id();
    app.world_mut().resource_mut::<EditContext>().enter(ship);
    // The GROUP TREE is what this is about, and the curated view is written to
    // put a turret's joint tree away - so the panel is asked for every field.
    app.world_mut().resource_mut::<EditorOverlays>().all_fields = true;
    select(&mut app, section);

    let headings: Vec<String> = app
        .world_mut()
        .query_filtered::<&Text, With<InspectorGroup>>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    assert!(
        headings.iter().any(|level| level == "MUZZLE"),
        "the muzzle is a level of its own: {headings:?}"
    );
    assert!(
        headings
            .iter()
            .all(|level| level == "ROOT" || !level.starts_with("ROOT ")),
        "no heading repeats the level above it: {headings:?}"
    );
}

#[test]
fn a_catalog_section_is_copied_inline_before_the_first_edit_lands() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let ship = app
        .world_mut()
        .spawn((
            EditorNode,
            ShipNode::default(),
            NodeId("ship_1".to_string()),
            Transform::default(),
            ChildOf(scenario),
        ))
        .id();
    app.world_mut()
        .insert_resource(GameSections(vec![SectionConfig {
            base: BaseSectionConfig {
                id: "thruster".to_string(),
                ..default()
            },
            kind: SectionKind::Thruster(ThrusterSectionConfig {
                magnitude: 40.0,
                ..default()
            }),
        }]));
    let section = app
        .world_mut()
        .spawn((
            EditorNode,
            SectionNode {
                source: SectionSource::Prototype("thruster".to_string()),
                modifications: vec![],
                binds: vec![],
            },
            NodeId("thruster_1".to_string()),
            Transform::default(),
            ChildOf(ship),
        ))
        .id();
    app.world_mut().resource_mut::<EditContext>().enter(ship);
    select(&mut app, section);

    submit(&mut app, "Magnitude", "77");

    let node = app
        .world()
        .get::<SectionNode>(section)
        .expect("the section");
    let SectionSource::Inline(config) = &node.source else {
        panic!("the edit copied the prototype inline");
    };
    let SectionKind::Thruster(tuned) = &config.kind else {
        panic!("still a thruster");
    };
    assert!((tuned.magnitude - 77.0).abs() < f32::EPSILON);
}

/// Drag the row's NAME and the number under it moves. The one control the
/// panel lacked: every other type already had one that could only express what
/// the type takes, and a number had a box that could express `nan`.
#[test]
fn dragging_a_rows_name_writes_the_number_into_the_document() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    select(&mut app, rock);

    scrub(&mut app, "Radius", 20.0);

    // `radius` is declared at half a meter per pixel, so twenty pixels is ten.
    assert!(
        (radius_of(&app, rock) - Meters(40.0)).get().abs() < 1e-4,
        "the radius followed the pointer (got {:?})",
        radius_of(&app, rock)
    );
}

/// A drag walking into a floor ARRIVES at it. A typed number below the floor is
/// a mistake and is refused; a drag that keeps going is a builder asking for
/// the smallest value there is.
#[test]
fn a_scrub_stops_at_the_floor_instead_of_being_refused() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(10.0));
    select(&mut app, rock);

    scrub(&mut app, "Radius", -400.0);

    assert!(
        radius_of(&app, rock).get().abs() < 1e-4,
        "the radius stopped at zero (got {:?})",
        radius_of(&app, rock)
    );
}

/// The step a field is declared with is also the precision it lands on, so a
/// scrubbed number is one a builder can read back.
#[test]
fn a_scrub_lands_on_the_step_the_field_was_declared_with() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    select(&mut app, rock);

    scrub(&mut app, "Radius", 7.3);

    let radius = radius_of(&app, rock).get();
    assert!(
        ((radius / 0.5).round() * 0.5 - radius).abs() < 1e-4,
        "the radius landed on a multiple of its step (got {radius})"
    );
}

/// A row holding something that is not a number has no grip: there is nothing
/// a pointer could slide a name into.
#[test]
fn a_row_that_is_not_a_number_has_no_grip() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    select(&mut app, rock);

    assert!(grip_of(&mut app, "Radius").is_some());
    assert!(
        grip_of(&mut app, "Invulnerable").is_none(),
        "a flag is ticked, not scrubbed"
    );
    assert!(
        grip_of(&mut app, "Name").is_none(),
        "and a name is typed, not scrubbed"
    );
}

/// A grip on ONE AXIS of a vector moves by the row's OWN step.
///
/// The step used to be resolved a second time from the axis path, where `x`
/// matches no declaration: a pose row scaled its travel by 0.05 and then
/// snapped the result onto the 0.1 fallback grid, which put every step of an
/// ordinary drag straight back where it started.
#[test]
fn a_scrub_of_one_axis_moves_by_the_rows_own_step() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    app.world_mut()
        .entity_mut(rock)
        .insert(Transform::from_xyz(3.0, 0.0, 0.0));
    select(&mut app, rock);

    scrub(&mut app, "Position X", 1.0);

    let moved = position_of(&app, rock).x;
    assert!(
        (moved - 3.05).abs() < 1e-4,
        "one pixel is one step of 0.05 (got {moved})"
    );
}

/// Pixels that do not reach a whole step are KEPT.
///
/// Half a logical pixel is what one physical pixel is worth at 2x scale. A grip
/// that dropped the half moved on no HiDPI screen at all, whatever the row.
#[test]
fn a_scrub_keeps_the_pixels_that_do_not_reach_a_whole_step() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    select(&mut app, rock);

    scrub(&mut app, "Radius", 0.5);
    assert!(
        (radius_of(&app, rock) - Meters(30.0)).get().abs() < 1e-4,
        "half a pixel is not a step yet (got {:?})",
        radius_of(&app, rock)
    );

    scrub(&mut app, "Radius", 0.5);
    assert!(
        (radius_of(&app, rock) - Meters(30.5)).get().abs() < 1e-4,
        "the two halves made the step between them (got {:?})",
        radius_of(&app, rock)
    );
}

/// A scrub that reaches the edge of the window comes back on the other side, so
/// the drag can keep going. At half a meter a pixel, a radius worth changing is
/// further than one screen of pointer travel.
#[test]
fn a_scrub_that_reaches_the_edge_wraps_the_pointer() {
    let mut app = inspector_app();
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: WindowResolution::new(1024, 768),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    select(&mut app, rock);

    scrub_from(&mut app, "Radius", Vec2::new(1020.0, 400.0), 4.0);

    let put = app
        .world()
        .get::<Window>(window)
        .expect("the window")
        .cursor_position();
    assert_eq!(
        put,
        Some(Vec2::new(48.0, 400.0)),
        "the pointer came back on the left"
    );

    // The warp lands in the same stream the drag reads, as a move of its own.
    // Taking it back is what stops one wrap counting as a screen of travel.
    let before = radius_of(&app, rock);
    scrub_from(&mut app, "Radius", Vec2::new(48.0, 400.0), -972.0);
    assert!(
        (radius_of(&app, rock) - before).get().abs() < 1e-4,
        "the echo of the warp moved nothing (got {:?})",
        radius_of(&app, rock)
    );
}

/// A scrub easing BACK off an edge is correcting an overshoot, and wrapping it
/// across the window would be the last thing it wants.
#[test]
fn a_scrub_easing_off_an_edge_stays_where_it_is() {
    let mut app = inspector_app();
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: WindowResolution::new(1024, 768),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    select(&mut app, rock);

    scrub_from(&mut app, "Radius", Vec2::new(1020.0, 400.0), -4.0);

    assert_eq!(
        app.world()
            .get::<Window>(window)
            .expect("the window")
            .cursor_position(),
        None,
        "nothing was warped, so nothing was ever set"
    );
}

/// A scrub past a refusal takes the refusal with it.
///
/// A refused box is HELD OUT of the repaint, so the number it shows survives
/// the document moving underneath it. The scrub is the correction, so the panel
/// would otherwise show a red `-5` over a rock that had grown on the stage.
#[test]
fn a_scrub_clears_the_refusal_the_box_was_showing() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    select(&mut app, rock);
    let field = submit(&mut app, "Radius", "-5");
    assert!(
        app.world().get::<TextFieldError>(field).is_some(),
        "the typed number was refused first"
    );

    scrub(&mut app, "Radius", 20.0);

    assert!(
        (radius_of(&app, rock) - Meters(40.0)).get().abs() < 1e-4,
        "the scrub moved the document (got {:?})",
        radius_of(&app, rock)
    );
    assert!(
        app.world().get::<TextFieldError>(field).is_none(),
        "and the box it wrote to is showing the document again"
    );
}

/// Only a CONFIG edit that took makes an object's body stale.
///
/// The body is a fresh mesh, a fresh material and a fresh collider each time it
/// is dropped, and a held scrub asks once a frame. A refusal and a name are
/// both changes to the node that the mesh is not built from.
#[test]
fn only_a_config_edit_that_took_makes_the_body_stale() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    select(&mut app, rock);

    submit(&mut app, "Radius", "big");
    assert_eq!(
        stale_bodies(&mut app),
        0,
        "a refused radius rebuilds nothing"
    );

    submit(&mut app, "Name", "boulder");
    assert_eq!(stale_bodies(&mut app), 0, "and neither does a name");

    submit(&mut app, "Seed", "9");
    assert_eq!(
        stale_bodies(&mut app),
        0,
        "the preview draws a plain ball, so a seed is not what it is built from"
    );

    submit(&mut app, "Radius", "12");
    assert_eq!(stale_bodies(&mut app), 1, "a radius that took does");
}

/// How many stale bodies the message buffer is holding.
fn stale_bodies(app: &mut App) -> usize {
    let mut cursor = app
        .world_mut()
        .resource_mut::<Messages<ObjectBodyStale>>()
        .get_cursor();
    cursor
        .read(app.world().resource::<Messages<ObjectBodyStale>>())
        .count()
}

/// Where the object sits, which is what a pose row writes to.
fn position_of(app: &App, object: Entity) -> Vec3 {
    app.world()
        .get::<Transform>(object)
        .expect("a transform")
        .translation
}

/// The grip of the row called `label`, if that row has one.
fn grip_of(app: &mut App, label: &str) -> Option<Entity> {
    let wanted = format!("Inspector Grip {label}");
    app.world_mut()
        .query::<(Entity, &Name, &InspectorDrag)>()
        .iter(app.world())
        .find(|(_, name, _)| name.as_str() == wanted)
        .map(|(entity, ..)| entity)
}

/// Slide the name of the row called `label` by `pixels`, the way a pointer
/// does.
fn scrub(app: &mut App, label: &str, pixels: f32) {
    scrub_from(app, label, Vec2::ZERO, pixels);
}

/// The same slide, from a stated place in the window - which is what decides
/// whether the pointer wraps.
fn scrub_from(app: &mut App, label: &str, at: Vec2, pixels: f32) {
    let grip = grip_of(app, label).unwrap_or_else(|| panic!("no grip {label:?}"));
    let delta = Vec2::new(pixels, 0.0);
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::Image(bevy::camera::ImageRenderTarget {
                handle: Handle::default(),
                scale_factor: 1.0,
            }),
            position: at,
        },
        Drag {
            button: PointerButton::Primary,
            distance: delta,
            delta,
        },
        grip,
    ));
    app.update();
}

/// A reference naming nothing this document spawns is marked in the unit slot -
/// the same slot a refusal takes, because both are "this row is not right yet".
///
/// The warning has to be HERE and not at the file: the lowering silently drops
/// a handler that names an unspawned id, so a builder who only sees the saved
/// scenario sees a beat that never fires and no reason why.
#[test]
fn a_reference_that_names_nothing_is_marked_in_the_unit_slot() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    asteroid(&mut app, scenario, "asteroid_1", Meters(30.0));
    let filter = app
        .world_mut()
        .spawn((
            EditorNode,
            FilterNode {
                kind: FilterKind::Entity(EntityFilterConfig {
                    id: Some("asteroid_9".to_string()),
                    ..default()
                }),
            },
            NodeId("entity_1".to_string()),
            ChildOf(scenario),
        ))
        .id();
    select(&mut app, filter);
    paint_references(&mut app);

    assert_eq!(
        unit_of(&mut app, "Id"),
        "unknown",
        "nothing in the document is called asteroid_9"
    );

    submit(&mut app, "Id", "asteroid_1");
    paint_references(&mut app);

    assert_eq!(
        unit_of(&mut app, "Id"),
        "",
        "the rock is on the board, so the row is right"
    );
}

/// The lookup and the paint, in the order the schedule runs them.
fn paint_references(app: &mut App) {
    app.world_mut()
        .run_system_once(sync_reference_faults)
        .expect("the lookup runs");
    app.world_mut()
        .run_system_once(paint_field_reasons)
        .expect("the reason paints");
}

/// A condition is ONE PAGE of the filter that holds it: a row per node of the
/// tree, each writing to its own entity.
///
/// The tree in the rail says `Expression` and stops - the grammar's shape is
/// not the document's shape - so this page is the only place a condition can
/// be read or changed, and it has to reach every node of it.
#[test]
fn a_condition_is_one_page_of_the_filter_that_holds_it() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let filter = app
        .world_mut()
        .spawn((
            EditorNode,
            FilterNode {
                kind: FilterChoice::Expression.stock(),
            },
            NodeId("expression_1".to_string()),
            ChildOf(scenario),
        ))
        .id();
    let root = operand(&mut app, filter, "equal_1", ExprChoice::Equal);
    let left = operand(&mut app, root, "value_2", ExprChoice::Value);
    operand(&mut app, root, "value_3", ExprChoice::Value);
    select(&mut app, filter);

    assert_eq!(
        operand_names(&mut app),
        ["Compare", "Left", "Right"],
        "every node of the condition is a row, named by its place"
    );

    submit(&mut app, "Left", "scenario.elapsed");

    assert_eq!(
        leaf_text(&app, left),
        "scenario.elapsed",
        "the row wrote to its OWN node, not to the filter the panel is on"
    );
}

/// The value a `VariableSet` writes gets the SAME page a condition does: the
/// key, and then a row per node of the value's own tree.
///
/// The action's field is an expression, not a literal - the engine evaluates
/// it - and typed into one box the grammar was a string a builder had to
/// already know. As rows it is the shape it has, and the operators are picked
/// rather than spelled.
#[test]
fn a_variable_set_writes_a_value_its_own_page_builds() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let action = app
        .world_mut()
        .spawn((
            EditorNode,
            ActionNode {
                kind: ActionChoice::VariableSet.stock(),
            },
            NodeId("set_1".to_string()),
            ChildOf(scenario),
        ))
        .id();
    let root = operand(&mut app, action, "add_1", ExprChoice::Add);
    let left = operand(&mut app, root, "value_2", ExprChoice::Value);
    operand(&mut app, root, "value_3", ExprChoice::Value);
    select(&mut app, action);

    assert_eq!(
        chip_of(&mut app, "Key"),
        Some(Offers::Named(Names::Variable)),
        "the key is still the action's own field, and still picks a variable"
    );
    assert_eq!(
        operand_names(&mut app),
        ["Writes", "Left", "Right"],
        "and every node of the value is a row, named by its place"
    );

    submit(&mut app, "Left", "beat");

    assert_eq!(
        leaf_text(&app, left),
        "beat",
        "the row wrote to its OWN node, not to the action the panel is on"
    );
}

/// One node of a condition, hung under `owner`.
fn operand(app: &mut App, owner: Entity, id: &str, kind: ExprChoice) -> Entity {
    app.world_mut()
        .spawn((
            EditorNode,
            ExpressionNode { kind: kind.stock() },
            NodeId(id.to_string()),
            ChildOf(owner),
        ))
        .id()
}

/// The place each row of the condition page stands in, in draw order.
fn operand_names(app: &mut App) -> Vec<String> {
    let list = app
        .world_mut()
        .query_filtered::<Entity, With<InspectorList>>()
        .single(app.world())
        .expect("one inspector list");
    let rows: Vec<Entity> = app
        .world()
        .get::<Children>(list)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    rows.into_iter()
        .filter_map(|row| app.world().get::<Name>(row))
        .filter_map(|name| {
            name.as_str()
                .strip_prefix("Inspector Operand ")
                .map(str::to_string)
        })
        .collect()
}

/// What a value node holds.
fn leaf_text(app: &App, node: Entity) -> String {
    match &app
        .world()
        .get::<ExpressionNode>(node)
        .expect("an expression node")
        .kind
    {
        ExprKind::Value(operand) => operand.value.to_string(),
        other => panic!("not a value: {other:?}"),
    }
}

/// A row with a laid-out box, hovered or not, plus the one hint panel.
/// Returns the panel and its two text lines.
fn hint_tree(world: &mut World, hovered: bool) -> (Entity, Entity, Entity) {
    world.spawn((
        InspectorHint {
            title: "Turret / Fire Rate".to_string(),
            body: "Shots a second.".to_string(),
        },
        Hovered(hovered),
        ComputedNode {
            size: Vec2::new(280.0, 22.0),
            inverse_scale_factor: 1.0,
            ..default()
        },
        UiGlobalTransform::from(Affine2::from_translation(Vec2::new(1000.0, 100.0))),
    ));
    let title = world.spawn(Text::new("")).id();
    let body = world.spawn(Text::new("")).id();
    let tooltip = world
        .spawn((
            InspectorTooltip,
            Node {
                display: Display::None,
                ..default()
            },
        ))
        .add_children(&[title, body])
        .id();
    (tooltip, title, body)
}

/// The panel explains itself: resting on a row says what it is called in full
/// - the name column clips - and what the config author said it is for.
#[test]
fn hovering_a_row_reveals_its_whole_name_and_what_it_is_for() {
    let mut world = World::new();
    let (tooltip, title, body) = hint_tree(&mut world, true);

    world
        .run_system_once(sync_inspector_tooltip)
        .expect("the sync runs");

    let node = world.get::<Node>(tooltip).expect("a node");
    assert_eq!(node.display, Display::Flex);
    assert_eq!(
        node.left,
        px(860.0 - HINT_W - HINT_GAP),
        "clear of the row, on the stage side"
    );
    assert_eq!(node.top, px(89.0), "level with the row");
    assert_eq!(
        world.get::<Text>(title).expect("the title").0,
        "Turret / Fire Rate"
    );
    assert_eq!(
        world.get::<Text>(body).expect("the body").0,
        "Shots a second."
    );
}

/// EVENTS gives the panel the whole window, so the room is to the RIGHT of the
/// rows - and a hint drawn on the left would be a hint over the tree.
#[test]
fn the_hint_takes_the_side_of_the_row_that_has_room() {
    let mut world = World::new();
    let (tooltip, _, _) = hint_tree(&mut world, true);
    world.spawn((
        Window {
            resolution: WindowResolution::new(1920, 1080),
            ..default()
        },
        PrimaryWindow,
    ));

    world
        .run_system_once(sync_inspector_tooltip)
        .expect("the sync runs");

    assert_eq!(
        world.get::<Node>(tooltip).expect("a node").left,
        px(1140.0 + HINT_GAP),
        "beside the row, clear of the tree"
    );
}

/// The pointer leaving takes the hint with it: a hint left standing over the
/// stage is a hint about a row nobody is looking at.
#[test]
fn the_inspector_hint_goes_away_with_the_pointer() {
    let mut world = World::new();
    let (tooltip, _, _) = hint_tree(&mut world, false);

    world
        .run_system_once(sync_inspector_tooltip)
        .expect("the sync runs");

    assert_eq!(
        world.get::<Node>(tooltip).expect("a node").display,
        Display::None
    );
}

/// Every span the panel spawns has to be MARKED for the editor's typeface.
///
/// `UiText` is what routes a span through Iosevka Term; a span that forgets it
/// renders in the engine's built-in face, which is close enough to pass a
/// glance and has none of the line art the panel is drawn with - an unmarked
/// picker chip is an empty box.
#[test]
fn every_span_the_panel_spawns_takes_the_editor_typeface() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let filter = app
        .world_mut()
        .spawn((
            EditorNode,
            FilterNode {
                kind: FilterKind::Entity(EntityFilterConfig {
                    id: Some("asteroid_1".to_string()),
                    ..default()
                }),
            },
            NodeId("entity_1".to_string()),
            ChildOf(scenario),
        ))
        .id();
    select(&mut app, filter);

    assert_eq!(
        unmarked_spans(&mut app),
        Vec::<String>::new(),
        "a filter draws the picker chip and the choice rows"
    );

    let beacon = beacon(&mut app, scenario, "beacon_1");
    select(&mut app, beacon);

    assert_eq!(
        unmarked_spans(&mut app),
        Vec::<String>::new(),
        "a beacon draws the swatch, the units and the group headers"
    );
}

/// What every span in the panel says, for the spans that would render in the
/// engine's own face.
fn unmarked_spans(app: &mut App) -> Vec<String> {
    app.world_mut()
        .query_filtered::<&Text, Without<UiText>>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect()
}

/// A row naming a FILE gets the same chip a row naming an id gets, and the
/// same warning: the picker is the only place the set of shippable files is
/// written down, and a path outside it is a path that loads nothing.
#[test]
fn a_file_row_offers_the_bundles_files_and_marks_one_they_do_not_ship() {
    let mut app = inspector_app();
    installed_bundle(&mut app, "base", &["icons/alpha.png"]);
    let scenario = document(&mut app);
    let action = app
        .world_mut()
        .spawn((
            EditorNode,
            ActionNode {
                kind: ActionKind::Leaf(EventActionConfig::StoryMessage(StoryMessageActionConfig {
                    speaker: "Alpha".to_string(),
                    text: "Strip it clean.".to_string(),
                    dwell: None,
                    icon: Some("dep://base/icons/gone.png".into()),
                })),
            },
            NodeId("action_1".to_string()),
            ChildOf(scenario),
        ))
        .id();
    select(&mut app, action);
    paint_references(&mut app);

    assert_eq!(
        chip_of(&mut app, "Icon"),
        Some(Offers::File(AssetSort::Image)),
        "the icon row opens the image picker"
    );
    assert_eq!(
        unit_of(&mut app, "Icon"),
        UNRESOLVED,
        "no installed bundle ships icons/gone.png"
    );

    submit(&mut app, "Icon", "dep://base/icons/alpha.png");
    paint_references(&mut app);

    assert_eq!(
        unit_of(&mut app, "Icon"),
        "",
        "the file the bundle declares is right"
    );
}

/// What the picker chip on `label` offers, or `None` when the row has none.
fn chip_of(app: &mut App, label: &str) -> Option<Offers> {
    let wanted = format!("Inspector Ref {label}");
    app.world_mut()
        .query::<(&Name, &InspectorRef)>()
        .iter(app.world())
        .find(|(name, _)| name.as_str() == wanted)
        .map(|(_, chip)| chip.offers)
}

/// One enabled bundle shipping `resources`, the way the editor's picker reads
/// the installed set.
fn installed_bundle(app: &mut App, id: &str, resources: &[&str]) {
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
        entries: vec![CatalogEntry {
            decl: ModEntry {
                id: id.to_string(),
                bundle: format!("mods/{id}/{id}.bundle.ron"),
                base: true,
                hidden: false,
            },
            bundle,
        }],
    });
    app.world_mut().insert_resource(bundles);
    app.world_mut().insert_resource(catalogs);
    app.world_mut()
        .insert_resource(EnabledMods([id.to_string()].into_iter().collect()));
}
