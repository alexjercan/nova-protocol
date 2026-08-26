//! The reflection walk and the write-back, tested against real configs rather
//! than a fixture struct: the whole claim of this module is that it works on
//! whatever content declares, so a test that walks its own toy type would not
//! be testing it.

use bevy::ecs::system::RunSystemOnce;
use nova_scenario::prelude::{
    AnchorConfig, AsteroidConfig, BeaconConfig, LightConfig, ScenarioObjectKind, SectionSource,
};
use nova_ship::prelude::{
    BaseSectionConfig, GameSections, MuzzleConfig, SectionConfig, SectionKind, ThrusterExhaust,
    ThrusterExhaustConfig, ThrusterExhaustShape, ThrusterSectionConfig, TurretJoint,
    TurretSectionConfig,
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

/// The text a row is edited through. A colour row is typed into like any other
/// leaf - the swatch beside it is a readout, not a second source of truth - so
/// it answers here too.
fn text_of(rows: &[InspectorRow], label: &str) -> String {
    match &row(rows, label).value {
        RowValue::Text(text) | RowValue::Colour(text) => text.clone(),
        // A vector row reads as one line however many boxes it is typed in.
        RowValue::Axes(axes) => axes.join(", "),
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

/// A thruster whose exhaust is authored, so the walk reaches the two-name enum
/// inside it. The stock thruster has `exhaust: None` and stops short of it.
fn thruster_with_exhaust(geometry: ThrusterExhaustShape) -> SectionNode {
    SectionNode {
        source: SectionSource::Inline(SectionConfig {
            base: BaseSectionConfig {
                id: "thruster".to_string(),
                ..default()
            },
            kind: SectionKind::Thruster(ThrusterSectionConfig {
                exhaust: Some(ThrusterExhaust {
                    shape: ThrusterExhaustConfig {
                        geometry,
                        ..default()
                    },
                    ..default()
                }),
                ..default()
            }),
        }),
        modifications: vec![],
        binds: vec![],
    }
}

/// A turret whose joint tree carries a muzzle, the way every shipped turret
/// does: base -> yaw -> pitch -> barrel -> muzzle.
fn turret_with_muzzle(fire_rate: f32) -> SectionNode {
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
    let muzzle = joint(
        Some(MuzzleConfig {
            fire_rate,
            muzzle_effect: None,
        }),
        Vec::new(),
    );
    SectionNode {
        source: SectionSource::Inline(SectionConfig {
            base: BaseSectionConfig {
                id: "turret".to_string(),
                ..default()
            },
            kind: SectionKind::Turret(TurretSectionConfig {
                root: joint(None, vec![muzzle]),
                ..default()
            }),
        }),
        modifications: vec![],
        binds: vec![],
    }
}

/// A section's Key row was dead text beside a live verb in the top bar: it
/// named the binding and could not change it.
#[test]
fn a_bindable_section_offers_its_key_as_the_thing_you_press() {
    let rows = section_rows(&turret_with_muzzle(1.0), None);

    assert_eq!(
        row(&rows, "Key").value,
        RowValue::Key(UNBOUND.to_string()),
        "a turret binds, and says so even when nothing is bound yet"
    );
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

/// A colour is its own kind of row, so the panel can paint the colour beside
/// the hex. It is still typed into like any other leaf.
#[test]
fn a_colour_row_carries_the_colour_and_not_just_its_name() {
    let object = ObjectNode {
        name: "beacon".to_string(),
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: "BEACON".to_string(),
            radius: 3.0,
            color: Color::srgb(0.0, 0.5, 1.0),
            area_radius: None,
            lock_signature: None,
        }),
    };
    let rows = object_rows(&object, &Transform::default());

    assert_eq!(
        row(&rows, "Color").value,
        RowValue::Colour("#0080ff".to_string())
    );
    assert_eq!(
        parse_colour("#0080ff")
            .map(Srgba::from)
            .map(|srgba| srgba.blue),
        Some(1.0),
        "and the panel can turn that back into a colour to paint"
    );
}

/// Half-typed hex is not a colour yet. The swatch paints nothing rather than
/// guessing, which is why this returns an `Option`.
#[test]
fn text_that_is_not_a_colour_yet_paints_nothing() {
    assert_eq!(parse_colour("#00"), None);
    assert_eq!(parse_colour(""), None);
}

/// An enum whose variants are all bare NAMES has nothing to invent, so it is
/// offered as a choice rather than shown as a readout.
#[test]
fn an_enum_of_bare_names_is_a_choice() {
    let node = thruster_with_exhaust(ThrusterExhaustShape::Cone);
    let rows = section_rows(&node, None);

    let RowValue::Choice { options, chosen } = &row(&rows, "Geometry").value else {
        panic!(
            "the exhaust's geometry is a two-name enum; got {:?}",
            row(&rows, "Geometry").value
        );
    };
    assert_eq!(options, &vec!["Cone".to_string(), "Rect".to_string()]);
    assert_eq!(*chosen, 0);
}

/// And choosing one writes it, which is the half a readout could never do.
#[test]
fn choosing_a_name_switches_the_variant() {
    let mut node = thruster_with_exhaust(ThrusterExhaustShape::Cone);
    let rows = section_rows(&node, None);
    let path = row(&rows, "Geometry").path.clone();

    let mut config = node.resolve(None).expect("an inline section").clone();
    choose_field(section_config_mut(&mut config.kind), &path, "Rect").expect("a bare name");
    node.source = SectionSource::Inline(config);

    let rows = section_rows(&node, None);
    let RowValue::Choice { options, chosen } = &row(&rows, "Geometry").value else {
        panic!("still a choice");
    };
    assert_eq!(options[*chosen], "Rect");
}

/// A variant that CARRIES something stays a readout: switching to it would
/// mean inventing every field of it.
#[test]
fn a_variant_with_fields_is_not_offered_as_a_choice() {
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

    assert_eq!(
        row(&rows, "Kind").value,
        RowValue::Fixed("Directional".to_string())
    );
}

/// Which way a node faces, in the three numbers a builder means by it. A
/// `Quat`'s four are not a thing anyone types.
#[test]
fn a_node_reports_which_way_it_faces_in_degrees() {
    let object = asteroid(stock_asteroid());
    let quarter_to_port =
        Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
    let rows = object_rows(&object, &quarter_to_port);

    assert_eq!(text_of(&rows, "Rotation"), "90, 0, 0");
    assert_eq!(row(&rows, "Rotation").root, FieldRoot::Rotation);
}

/// Degrees out, degrees back. The row is the only place the conversion
/// happens, so a heading that survives the trip is the whole contract.
#[test]
fn a_heading_survives_the_round_trip() {
    // The ROTATION round-trips, which is the contract. The degrees need not
    // come back identical: a half turn to port and a half turn to starboard
    // are the same heading, and `to_euler` picks one of them.
    for degrees in [
        Vec3::ZERO,
        Vec3::new(90.0, 0.0, 0.0),
        Vec3::new(-45.0, 30.0, 0.0),
        Vec3::new(180.0, 0.0, 0.0),
    ] {
        let wanted = rotation_from_degrees(degrees);
        let back = rotation_from_degrees(rotation_degrees(&Transform::from_rotation(wanted)));
        assert!(
            (back.dot(wanted).abs() - 1.0).abs() < 1e-4,
            "{degrees:?} came back as a different heading: {back:?} vs {wanted:?}"
        );
    }
}

/// Nova sections MATE, they do not stretch - so there is no scale row, and a
/// test says so rather than leaving its absence to be read as an oversight.
#[test]
fn there_is_no_scale_row() {
    let rows = object_rows(&asteroid(stock_asteroid()), &Transform::default());

    assert!(
        !rows.iter().any(|row| row.label.contains("Scale")),
        "found {:?}",
        rows.iter().map(|row| &row.label).collect::<Vec<_>>()
    );
}

/// The row a turret was MISSING. Its fire rate lives on a muzzle, on a joint,
/// inside `root.children` - a `Vec` the walk used to stop at, showing the whole
/// joint tree as one line of debug text. A builder could not see the number,
/// let alone change it.
#[test]
fn a_turrets_fire_rate_is_reachable_through_its_joint_tree() {
    let node = turret_with_muzzle(4.0);
    let rows = section_rows(&node, None);

    let rate = rows
        .iter()
        .find(|row| row.label == "Fire Rate")
        .expect("the muzzle's fire rate has a row of its own");
    assert_eq!(rate.value, RowValue::Text("4".to_string()));
    assert!(
        rate.path.contains(&PathStep::Item(0)),
        "and it is reached by stepping INTO the joint list: {:?}",
        rate.path
    );
}

/// The editor is not a section editor. A turret's config is a joint tree with
/// a render mesh transform on every joint, and a scenario builder asks how fast
/// it fires - so that is what the panel opens on.
#[test]
fn a_turret_opens_on_what_it_does_not_on_its_joint_tree() {
    let node = turret_with_muzzle(4.0);
    let labels: Vec<String> = curated_section_rows(&node, None)
        .into_iter()
        .map(|row| row.label)
        .collect();

    assert!(
        labels.contains(&"Fire Rate".to_string()),
        "the one number a turret is authored by is there: {labels:?}"
    );
    for buried in ["Offset", "Render Mesh Transform", "Speed", "Axis"] {
        assert!(
            !labels.contains(&buried.to_string()),
            "{buried:?} is joint plumbing and is not on the first screen: {labels:?}"
        );
    }
}

/// And the tree over it goes with it: a fire rate under five levels of joint
/// heading is five lines of the thing this view puts away.
#[test]
fn a_curated_row_drops_the_headings_it_was_buried_under() {
    let node = turret_with_muzzle(4.0);
    let rows = curated_section_rows(&node, None);

    assert!(
        row(&rows, "Fire Rate").group.is_empty(),
        "the fire rate stands with the rest, not under Root > Children > Muzzle: {:?}",
        row(&rows, "Fire Rate").group
    );
}

/// And nothing is lost: the full walk still holds every field, which is what
/// View > All Fields hands back.
#[test]
fn the_full_walk_still_holds_what_the_first_screen_drops() {
    let node = turret_with_muzzle(4.0);
    let labels: Vec<String> = section_rows(&node, None)
        .into_iter()
        .map(|row| row.label)
        .collect();

    assert!(
        labels.contains(&"Offset".to_string()),
        "the joint tree is one menu item away, not gone: {labels:?}"
    );
}

/// The same rule on the things a scenario holds beside its ships.
#[test]
fn a_rock_opens_on_its_size_and_not_its_texture() {
    let rock = asteroid(stock_asteroid());
    let labels: Vec<String> = curated_object_rows(&rock, &Transform::default())
        .into_iter()
        .map(|row| row.label)
        .collect();

    assert!(
        labels.contains(&"Radius".to_string()),
        "a rock is authored by how big it is: {labels:?}"
    );
    assert!(
        !labels.contains(&"Texture".to_string()),
        "and not by which image it wears: {labels:?}"
    );
    assert!(
        labels.contains(&"Name".to_string()) && labels.contains(&"Position".to_string()),
        "the node's own rows are never a config field, so they always stay: {labels:?}"
    );
}

/// A vector inside a config is the pose's shape, not a comma-separated line:
/// the row a builder edits an offset in has one box per axis.
#[test]
fn a_vector_in_a_config_is_three_boxes() {
    let mut node = turret_with_muzzle(4.0);
    let SectionSource::Inline(config) = &mut node.source else {
        panic!("the fixture is inline");
    };
    let SectionKind::Turret(turret) = &mut config.kind else {
        panic!("the fixture is a turret");
    };
    turret.root.offset = Vec3::new(0.0, 0.25, -1.5);
    let rows = section_rows(&node, None);

    assert_eq!(
        row(&rows, "Offset").value,
        RowValue::Axes(["0".to_string(), "0.25".to_string(), "-1.5".to_string()]),
        "the offset is typed one axis at a time"
    );
}

/// The box for one axis writes THAT axis: the panel hands each box the path of
/// its own component, and the write is the same reflection walk every other
/// field takes.
#[test]
fn one_axis_of_a_config_vector_writes_alone() {
    let mut node = turret_with_muzzle(4.0);
    let rows = section_rows(&node, None);
    let mut path = row(&rows, "Offset").path.clone();
    path.push(axis_step(1));

    let mut config = node.resolve(None).expect("an inline section").clone();
    write_field(section_config_mut(&mut config.kind), &path, false, "2.5")
        .expect("a number goes into an axis");
    node.source = SectionSource::Inline(config);

    let rows = section_rows(&node, None);
    assert_eq!(
        row(&rows, "Offset").value,
        RowValue::Axes(["0".to_string(), "2.5".to_string(), "0".to_string()]),
        "Y took the number and the other two kept theirs"
    );
}

/// And writing it lands on the muzzle rather than anywhere else.
#[test]
fn a_turrets_fire_rate_can_be_retuned() {
    let mut node = turret_with_muzzle(4.0);
    let rows = section_rows(&node, None);
    let path = rows
        .iter()
        .find(|row| row.label == "Fire Rate")
        .expect("a fire rate row")
        .path
        .clone();

    let mut config = node.resolve(None).expect("an inline section").clone();
    write_field(section_config_mut(&mut config.kind), &path, false, "9")
        .expect("a number goes into a number");
    node.source = SectionSource::Inline(config);

    let rows = section_rows(&node, None);
    assert_eq!(text_of(&rows, "Fire Rate"), "9");
}

/// A row deep in a tree is labelled by where it sits, not by its whole path:
/// the group says where you are, one segment per level, and the row says what
/// it is.
#[test]
fn a_nested_row_is_a_group_path_and_a_short_label() {
    let rows = section_rows(&turret_with_muzzle(4.0), None);
    let rate = rows
        .iter()
        .find(|row| row.label == "Fire Rate")
        .expect("a fire rate row");

    let group = &rate.group;
    assert!(
        group.len() > 1,
        "a value inside a joint tree sits several levels down: {group:?}"
    );
    assert_eq!(
        group.last().map(String::as_str),
        Some("Muzzle"),
        "the level a row sits in is the one nearest it: {group:?}"
    );
    assert!(
        group.iter().any(|level| level == "Children 1"),
        "and the index rides the name it indexes, one-based: {group:?}"
    );
    assert!(
        !group.iter().any(|level| level == "1"),
        "no level of the tree is called just a number: {group:?}"
    );
}

/// A bare number in a 64-character box says nothing about what it is. The
/// rows that have a unit carry it; the rows that do not stay bare, rather than
/// wearing a unit somebody guessed.
#[test]
fn a_number_carries_the_unit_it_is_typed_in() {
    let rows = object_rows(&asteroid(stock_asteroid()), &Transform::default());

    assert_eq!(row(&rows, "Radius").unit, "u", "a length is world units");
    assert_eq!(
        row(&rows, "Mass").unit,
        "",
        "a mass has a floor, not a unit"
    );
    assert_eq!(
        row(&rows, "Invulnerable").unit,
        "",
        "a checkbox is not measured in anything"
    );
    assert_eq!(row(&rows, "Position").unit, "u");
    assert_eq!(row(&rows, "Rotation").unit, "deg, yaw/pitch/roll");
}

/// The floor is refused at the box, and it is refused for an `Option` too - a
/// mass that is authored-or-absent is still not authored NEGATIVE.
#[test]
fn a_number_under_its_floor_is_refused_with_the_reason() {
    let mut config = stock_asteroid();

    let refusal = write_field(
        &mut config,
        &[PathStep::Field("radius".to_string())],
        false,
        "-2",
    )
    .expect_err("a negative radius is not a radius");
    assert_eq!(refusal, "min 0");
    assert!(
        (config.radius - stock_asteroid().radius).abs() < f32::EPSILON,
        "and the config is left as it was"
    );

    let refusal = write_field(
        &mut config,
        &[PathStep::Field("mass".to_string())],
        true,
        "-1",
    )
    .expect_err("an optional number has the same floor");
    assert_eq!(refusal, "min 0");

    // The floor is a FLOOR, not a ban on the field: the same box takes a
    // number above it.
    write_field(
        &mut config,
        &[PathStep::Field("radius".to_string())],
        false,
        "9",
    )
    .expect("a radius above the floor is written");
    assert!((config.radius - 9.0).abs() < f32::EPSILON);
}

/// A node's OWN fields sit in nothing, so there is no group over them.
#[test]
fn a_top_level_row_has_no_group() {
    let rows = object_rows(&asteroid(stock_asteroid()), &Transform::default());

    assert!(row(&rows, "Radius").group.is_empty());
    assert!(row(&rows, "Name").group.is_empty());
}

/// Where a node stands and which way it faces are one thing with two halves,
/// so they stand under one heading - and they stand LAST, because the config's
/// own top-level rows carry no heading to take the reader back out of it.
#[test]
fn the_pose_rows_close_the_panel_under_one_heading() {
    let rows = object_rows(&asteroid(stock_asteroid()), &Transform::default());

    for label in ["Position", "Rotation"] {
        assert_eq!(
            row(&rows, label).group,
            vec![TRANSFORM.to_string()],
            "{label} stands under the transform heading"
        );
    }
    let labels: Vec<&str> = rows.iter().map(|row| row.label.as_str()).collect();
    assert_eq!(
        &labels[labels.len() - 2..],
        &["Position", "Rotation"],
        "and nothing follows them: {labels:?}"
    );
}

/// Three numbers, three boxes, each written on its own. The box's path is what
/// makes that possible: it names one component of the vector the row is on.
#[test]
fn a_vector_row_hands_each_box_its_own_component() {
    let rows = object_rows(
        &asteroid(stock_asteroid()),
        &Transform::from_xyz(1.0, 2.0, 3.0),
    );

    assert_eq!(
        row(&rows, "Position").value,
        RowValue::Axes(["1".to_string(), "2".to_string(), "3".to_string()])
    );
    assert_eq!(axis_step(0), PathStep::Field("x".to_string()));
    assert_eq!(axis_step(1), PathStep::Field("y".to_string()));
    assert_eq!(axis_step(2), PathStep::Field("z".to_string()));

    // And the write-back takes one component through that path, leaving the
    // other two where they were - which is the whole point of three boxes.
    let mut position = Vec3::new(1.0, 2.0, 3.0);
    write_field(&mut position, &[axis_step(1)], false, "8").expect("the box writes");
    assert_eq!(position, Vec3::new(1.0, 8.0, 3.0));
}

/// A rotation inside a config reads in the same degrees a node's own heading
/// does. Walked as a plain struct it was four rows called X, Y, Z and W.
#[test]
fn a_rotation_inside_a_config_reads_in_degrees() {
    let node = SectionNode {
        source: SectionSource::Inline(SectionConfig {
            base: BaseSectionConfig {
                id: "thruster".to_string(),
                ..default()
            },
            kind: SectionKind::Thruster(ThrusterSectionConfig {
                exhaust: Some(ThrusterExhaust {
                    rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                    ..default()
                }),
                ..default()
            }),
        }),
        modifications: vec![],
        binds: vec![],
    };
    let rows = section_rows(&node, None);

    assert_eq!(text_of(&rows, "Rotation"), "90, 0, 0");
    assert!(
        !rows.iter().any(|row| row.label == "W"),
        "a quat is not four rows: {:?}",
        rows.iter().map(|row| &row.label).collect::<Vec<_>>()
    );
}

/// A config may hold a bare `LinearRgba` rather than a `Color`. It is still a
/// colour to the builder looking at it, and was four rows before this.
#[test]
fn a_linear_colour_is_still_a_colour() {
    let rows = section_rows(&thruster_with_exhaust(ThrusterExhaustShape::Cone), None);

    assert!(
        rows.iter()
            .any(|row| matches!(row.value, RowValue::Colour(_))),
        "the exhaust's emissive colours are colours: {:?}",
        rows.iter().map(|row| &row.label).collect::<Vec<_>>()
    );
    assert!(
        !rows.iter().any(|row| row.label == "Red"),
        "and not a row per channel"
    );
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

/// An object could be renamed and a ship could not, which made a fleet of
/// designs a column of minted ids.
#[test]
fn a_ship_is_named_like_anything_else_in_the_document() {
    let ship = ShipNode {
        name: "Kestrel".to_string(),
        ..default()
    };
    let rows = ship_rows(&ship, &Transform::default());

    assert_eq!(text_of(&rows, "Name"), "Kestrel");
    assert_eq!(
        row(&rows, "Name").root,
        FieldRoot::Label,
        "the name is a field of the NODE, not of a kind config"
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
