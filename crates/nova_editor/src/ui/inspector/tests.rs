//! The panel as a LIVE TREE: what a rebuild does to the widgets, and what a
//! submitted field does to the document. The row model itself is tested in
//! `crate::inspect`; these tests are about the reconciler around it.

use bevy::ecs::system::RunSystemOnce;
use nova_scenario::prelude::{AsteroidConfig, BeaconConfig, ScenarioObjectKind, SectionSource};
use nova_ship::prelude::{
    BaseSectionConfig, MuzzleConfig, SectionConfig, SectionKind, ThrusterSectionConfig,
    TurretJoint, TurretSectionConfig, WASDCameraController,
};

use super::*;
use crate::node::{EditorNode, NextChildOrdinal, ScenarioNode};

/// A panel with the reconciler running, over a document holding one scenario
/// node. The tests below hang things off it.
fn inspector_app() -> App {
    let mut app = App::new();
    app.insert_resource(UiSkin::default());
    app.init_resource::<SelectedNode>();
    app.add_message::<TextFieldSubmitted>();
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
            ScenarioNode,
            NodeId("scenario".to_string()),
            NextChildOrdinal::default(),
        ))
        .id();
    app.world_mut().insert_resource(EditContext {
        path: vec![scenario],
    });
    scenario
}

fn asteroid(app: &mut App, scenario: Entity, id: &str, radius: f32) -> Entity {
    app.world_mut()
        .spawn((
            EditorNode,
            ObjectNode {
                name: id.to_string(),
                kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                    radius,
                    texture: default(),
                    impact_sound: None,
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
                    radius: 3.0,
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

fn radius_of(app: &App, object: Entity) -> f32 {
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
    let rock = asteroid(&mut app, scenario, "asteroid_1", 3.0);

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

#[test]
fn inspecting_another_node_rebuilds_the_rows() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", 3.0);
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
    let first = asteroid(&mut app, scenario, "asteroid_1", 3.0);
    let second = asteroid(&mut app, scenario, "asteroid_2", 3.0);

    select(&mut app, first);
    select(&mut app, second);
    submit(&mut app, "Radius", "9");

    assert!((radius_of(&app, second) - 9.0).abs() < f32::EPSILON);
    assert!(
        (radius_of(&app, first) - 3.0).abs() < f32::EPSILON,
        "the first rock is not what the panel is on"
    );
}

/// The panel dies with the editor scene and the reconciler's `Local` does not.
/// A second visit to an unchanged document must still get its rows.
#[test]
fn a_returning_panel_gets_its_rows_back() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", 3.0);
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
    let rock = asteroid(&mut app, scenario, "asteroid_1", 3.0);
    select(&mut app, rock);

    let field = submit(&mut app, "Radius", "12.5");

    assert!((radius_of(&app, rock) - 12.5).abs() < f32::EPSILON);
    assert!(
        app.world().get::<TextFieldError>(field).is_none(),
        "a value the field took carries no error"
    );
    assert_eq!(
        app.world()
            .get::<TextFieldValue>(field)
            .expect("the field")
            .0,
        "12.5",
        "the box repaints from the document it just wrote"
    );
}

#[test]
fn a_refused_value_marks_the_field_and_leaves_the_document_alone() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", 3.0);
    select(&mut app, rock);

    let field = submit(&mut app, "Radius", "big");

    assert!((radius_of(&app, rock) - 3.0).abs() < f32::EPSILON);
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
    let rock = asteroid(&mut app, scenario, "asteroid_1", 3.0);
    select(&mut app, rock);

    let field = submit(&mut app, "Radius", "-4");

    assert!(
        (radius_of(&app, rock) - 3.0).abs() < f32::EPSILON,
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
    let rock = asteroid(&mut app, scenario, "asteroid_1", 3.0);
    select(&mut app, rock);
    app.world_mut()
        .run_system_once(paint_field_reasons)
        .expect("the reason paints");
    assert_eq!(unit_of(&mut app, "Radius"), "u", "nothing is wrong yet");

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
    let rock = asteroid(&mut app, scenario, "asteroid_1", 3.0);
    select(&mut app, rock);

    // A drag on the stage writes the pose; the panel is a readout of it.
    app.world_mut()
        .entity_mut(rock)
        .insert(Transform::from_xyz(4.0, 0.0, -6.0));
    app.update();

    for (axis, wanted) in [("X", "4"), ("Y", "0"), ("Z", "-6")] {
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
    let rock = asteroid(&mut app, scenario, "asteroid_1", 3.0);
    select(&mut app, rock);
    app.world_mut()
        .entity_mut(rock)
        .insert(Transform::from_xyz(4.0, 0.0, -6.0));
    app.update();

    submit(&mut app, "Position Y", "9");

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
    let rock = asteroid(&mut app, scenario, "asteroid_1", 3.0);
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
}

/// The panel used to go blank at the root, which reads as the panel breaking
/// every time you leave a ship. The root holds the document, so it says what
/// the document holds.
#[test]
fn the_scenario_node_counts_what_the_document_holds() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    asteroid(&mut app, scenario, "asteroid_1", 3.0);
    asteroid(&mut app, scenario, "asteroid_2", 3.0);
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
    assert_eq!(rows, ["Ships", "Objects", "Player Ship"], "{rows:?}");
    assert_eq!(readout_of(&mut app, "Ships"), "1");
    assert_eq!(readout_of(&mut app, "Objects"), "2");
    assert_eq!(
        readout_of(&mut app, "Player Ship"),
        "Kestrel",
        "and which one Play would hand over"
    );
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

#[test]
fn the_camera_gives_up_its_keys_while_a_field_is_being_typed_into() {
    let mut app = inspector_app();
    let scenario = document(&mut app);
    let rock = asteroid(&mut app, scenario, "asteroid_1", 3.0);
    select(&mut app, rock);
    let camera = app
        .world_mut()
        .spawn((EditorCamera, WASDCameraController))
        .id();

    let field = field_of(&mut app, "Radius");
    app.world_mut()
        .entity_mut(field)
        .insert(TextFieldFocused::at_end("3"));
    app.world_mut()
        .run_system_once(hold_camera_while_typing)
        .expect("the hold runs");
    assert!(
        app.world().get::<WASDCameraController>(camera).is_none(),
        "typing 'wasp' into a name must not fly the camera four ways"
    );
    assert!(app.world().get::<TypingHold>(camera).is_some());

    app.world_mut()
        .entity_mut(field)
        .remove::<TextFieldFocused>();
    app.world_mut()
        .run_system_once(hold_camera_while_typing)
        .expect("the hold runs");
    assert!(
        app.world().get::<WASDCameraController>(camera).is_some(),
        "the rig comes back with the keyboard"
    );
    assert!(app.world().get::<TypingHold>(camera).is_none());
}
