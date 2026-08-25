//! The reflection walk and the write-back, tested against real configs rather
//! than a fixture struct: the whole claim of this module is that it works on
//! whatever content declares, so a test that walks its own toy type would not
//! be testing it.

use bevy::ecs::system::RunSystemOnce;
use nova_scenario::prelude::{
    AnchorConfig, AsteroidConfig, BeaconConfig, LightConfig, ScenarioObjectKind, SectionSource,
};
use nova_ship::prelude::{
    BaseSectionConfig, GameSections, SectionConfig, SectionKind, ThrusterSectionConfig,
};

use super::*;
use crate::node::{EditorNode, NodeId, ObjectNode, ScenarioNode, SectionNode, ShipNode};

/// The row labelled `label`, or a failure naming what WAS found - a walk that
/// silently returns nothing is the failure mode these tests exist to catch.
fn row<'a>(rows: &'a [InspectorRow], label: &str) -> &'a InspectorRow {
    rows.iter()
        .find(|row| row.label == label)
        .unwrap_or_else(|| {
            panic!(
                "no row {label:?}; found {:?}",
                rows.iter().map(|row| &row.label).collect::<Vec<_>>()
            )
        })
}

fn text_of(rows: &[InspectorRow], label: &str) -> String {
    match &row(rows, label).value {
        RowValue::Text(text) => text.clone(),
        other => panic!("row {label:?} is {other:?}, not text"),
    }
}

fn thruster_node(magnitude: f32) -> SectionNode {
    SectionNode {
        source: SectionSource::Inline(SectionConfig {
            base: BaseSectionConfig {
                id: "thruster".to_string(),
                ..default()
            },
            kind: SectionKind::Thruster(ThrusterSectionConfig {
                magnitude,
                ..default()
            }),
        }),
        modifications: vec![],
        binds: vec![],
    }
}

fn asteroid(config: AsteroidConfig) -> ObjectNode {
    ObjectNode {
        name: "rock".to_string(),
        kind: ScenarioObjectKind::Asteroid(config),
    }
}

fn stock_asteroid() -> AsteroidConfig {
    AsteroidConfig {
        radius: 3.0,
        texture: default(),
        impact_sound: None,
        destroy_sound: None,
        mass: None,
        invulnerable: false,
        seed: None,
        lock_signature: None,
    }
}

/// Write into an object's kind config the way `apply_inspector_edits` does.
fn write(
    object: &mut ObjectNode,
    rows: &[InspectorRow],
    label: &str,
    text: &str,
) -> Result<(), String> {
    let row = row(rows, label);
    let config = object_config_mut(&mut object.kind).expect("an authorable kind");
    write_field(config, &row.path, row.optional, text)
}

#[test]
fn a_thruster_reports_the_field_its_config_declares() {
    let rows = section_rows(&thruster_node(120.0), None);

    assert_eq!(text_of(&rows, "Magnitude"), "120");
    // The catalog id it was built from, so a tuned section still says what it
    // started as.
    assert!(matches!(
        &row(&rows, "Part").value,
        RowValue::Fixed(id) if id == "thruster"
    ));
}

#[test]
fn typing_a_number_writes_it_into_the_config() {
    let mut node = thruster_node(120.0);
    let rows = section_rows(&node, None);
    let magnitude = row(&rows, "Magnitude").clone();

    let config = editable_config(&mut node, None).expect("an inline section");
    write_field(
        section_config_mut(&mut config.kind),
        &magnitude.path,
        magnitude.optional,
        "250.5",
    )
    .expect("a number the field takes");

    let SectionKind::Thruster(tuned) = &config.kind else {
        panic!("still a thruster");
    };
    assert!((tuned.magnitude - 250.5).abs() < f32::EPSILON);
}

#[test]
fn a_value_the_field_refuses_leaves_the_config_alone() {
    let mut node = thruster_node(120.0);
    let rows = section_rows(&node, None);
    let magnitude = row(&rows, "Magnitude").clone();

    let config = editable_config(&mut node, None).expect("an inline section");
    let refused = write_field(
        section_config_mut(&mut config.kind),
        &magnitude.path,
        magnitude.optional,
        "fast",
    );

    assert!(refused.is_err(), "'fast' is not a number");
    let SectionKind::Thruster(untouched) = &config.kind else {
        panic!("still a thruster");
    };
    assert!((untouched.magnitude - 120.0).abs() < f32::EPSILON);
}

#[test]
fn editing_a_catalog_section_copies_it_inline_first() {
    let catalog = GameSections(vec![SectionConfig {
        base: BaseSectionConfig {
            id: "thruster".to_string(),
            ..default()
        },
        kind: SectionKind::Thruster(ThrusterSectionConfig {
            magnitude: 40.0,
            ..default()
        }),
    }]);
    let mut node = SectionNode {
        source: SectionSource::Prototype("thruster".to_string()),
        modifications: vec![],
        binds: vec![],
    };
    // The prototype's fields are readable before any of this.
    assert_eq!(
        text_of(&section_rows(&node, Some(&catalog)), "Magnitude"),
        "40"
    );

    let config = editable_config(&mut node, Some(&catalog)).expect("a copy of the prototype");
    let SectionKind::Thruster(copied) = &config.kind else {
        panic!("the copy is still a thruster");
    };
    assert!((copied.magnitude - 40.0).abs() < f32::EPSILON);
    assert!(
        matches!(node.source, SectionSource::Inline(_)),
        "an edit to the id would be an edit to every ship that names it"
    );
    // The catalog entry is untouched by the copy.
    assert!(matches!(
        &catalog.get_section("thruster").expect("still listed").kind,
        SectionKind::Thruster(entry) if (entry.magnitude - 40.0).abs() < f32::EPSILON
    ));
}

#[test]
fn an_optional_number_is_one_row_that_empties_to_none() {
    let mut object = asteroid(stock_asteroid());
    let rows = object_rows(&object, &Transform::default());

    assert_eq!(text_of(&rows, "Mass"), "", "an unauthored mass reads empty");

    write(&mut object, &rows, "Mass", "8000").expect("a number the field takes");
    let ScenarioObjectKind::Asteroid(authored) = &object.kind else {
        panic!("still an asteroid");
    };
    assert_eq!(authored.mass, Some(8000.0));

    let rows = object_rows(&object, &Transform::default());
    assert_eq!(text_of(&rows, "Mass"), "8000");

    write(&mut object, &rows, "Mass", "  ").expect("blank clears it");
    let ScenarioObjectKind::Asteroid(cleared) = &object.kind else {
        panic!("still an asteroid");
    };
    assert_eq!(cleared.mass, None);
}

#[test]
fn a_placed_rock_keeps_what_the_inspector_wrote_on_it() {
    let mut object = asteroid(stock_asteroid());
    let rows = object_rows(&object, &Transform::default());

    write(&mut object, &rows, "Radius", "12.5").expect("a radius");
    write(&mut object, &rows, "Invulnerable", "true").expect("a flag");

    let ScenarioObjectKind::Asteroid(tuned) = &object.kind else {
        panic!("still an asteroid");
    };
    assert!((tuned.radius - 12.5).abs() < f32::EPSILON);
    assert!(tuned.invulnerable);
}

#[test]
fn a_flag_reads_as_a_checkbox_and_not_as_text() {
    let rows = object_rows(&asteroid(stock_asteroid()), &Transform::default());

    assert_eq!(row(&rows, "Invulnerable").value, RowValue::Flag(false));
}

#[test]
fn a_colour_reads_and_writes_as_hex() {
    let mut object = ObjectNode {
        name: "beacon".to_string(),
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: "BEACON".to_string(),
            radius: 3.0,
            color: Color::srgb(1.0, 0.0, 0.0),
            area_radius: None,
            lock_signature: None,
        }),
    };
    let rows = object_rows(&object, &Transform::default());
    assert_eq!(text_of(&rows, "Color"), "#ff0000");

    write(&mut object, &rows, "Color", "#0080ff").expect("a hex colour");
    let rows = object_rows(&object, &Transform::default());
    assert_eq!(text_of(&rows, "Color"), "#0080ff");
}

#[test]
fn an_enums_variant_is_shown_and_its_own_fields_are_walked() {
    let object = ObjectNode {
        name: "sun".to_string(),
        kind: ScenarioObjectKind::Light(LightConfig::Directional {
            illuminance: 9_000.0,
            color: Color::WHITE,
            shadows: false,
            aim: None,
        }),
    };
    let rows = object_rows(&object, &Transform::default());

    // The variant itself is a readout: switching one would mean inventing
    // every field of a variant nobody has authored.
    assert_eq!(
        row(&rows, "Kind").value,
        RowValue::Fixed("Directional".to_string())
    );
    assert_eq!(text_of(&rows, "Illuminance"), "9000");
    assert_eq!(row(&rows, "Shadows").value, RowValue::Flag(false));
}

#[test]
fn an_object_reports_where_it_stands() {
    let object = asteroid(stock_asteroid());
    let rows = object_rows(&object, &Transform::from_xyz(1.0, -2.5, 30.0));

    assert_eq!(text_of(&rows, "Position"), "1, -2.5, 30");
    assert_eq!(row(&rows, "Position").root, FieldRoot::Pose);
}

#[test]
fn a_ship_reports_who_flies_it() {
    let rows = ship_rows(&ShipNode::default(), &Transform::default());

    assert_eq!(
        row(&rows, "Driver").value,
        RowValue::Driver(ShipDriver::Player)
    );
}

#[test]
fn an_anchor_with_no_mass_still_lists_the_field() {
    let object = ObjectNode {
        name: "anchor".to_string(),
        kind: ScenarioObjectKind::Anchor(AnchorConfig {
            body_radius: 5.0,
            mass: None,
        }),
    };
    let rows = object_rows(&object, &Transform::default());

    assert_eq!(text_of(&rows, "Body Radius"), "5");
    assert!(
        row(&rows, "Mass").optional,
        "an absent number is still a row"
    );
}

#[test]
fn the_inspector_falls_back_to_the_node_you_are_standing_in() {
    let mut world = World::new();
    let scenario = world
        .spawn((EditorNode, ScenarioNode, NodeId("scenario".to_string())))
        .id();
    let ship = world
        .spawn((
            EditorNode,
            ShipNode::default(),
            NodeId("ship_1".to_string()),
        ))
        .id();
    let section = world.spawn((EditorNode, thruster_node(1.0))).id();

    fn answer(
        world: &mut World,
        selected: Option<Entity>,
        path: Vec<Entity>,
    ) -> Option<InspectTarget> {
        world.insert_resource(SelectedNode(selected));
        world.insert_resource(EditContext { path });
        world
            .run_system_once(
                |selected: Res<SelectedNode>, context: Res<EditContext>, kinds: NodeKinds| {
                    inspected(&selected, &context, &kinds)
                },
            )
            .expect("the system runs")
    }

    assert_eq!(
        answer(&mut world, None, vec![scenario]),
        Some(InspectTarget::Scenario(scenario)),
        "nothing selected at the root inspects the document"
    );
    assert_eq!(
        answer(&mut world, None, vec![scenario, ship]),
        Some(InspectTarget::Ship(ship)),
        "entering a ship clears the selection, so the ship is what is being worked on"
    );
    assert_eq!(
        answer(&mut world, Some(section), vec![scenario, ship]),
        Some(InspectTarget::Section(section)),
        "a selection wins over the context"
    );
}
