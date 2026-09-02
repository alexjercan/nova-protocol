//! The reflection walk and the write-back, tested against real configs rather
//! than a fixture struct: the whole claim of this module is that it works on
//! whatever content declares, so a test that walks its own toy type would not
//! be testing it.

use bevy::{ecs::system::RunSystemOnce, reflect::Typed};
use nova_gameplay::prelude::Allegiance;
use nova_scenario::prelude::{
    AIControllerConfig, AnchorConfig, AsteroidConfig, BeaconConfig, EntityFilterConfig,
    EventActionConfig, EventConfig, LightConfig, Names, ScenarioAreaConfig, ScenarioObjectKind,
    SectionSource, ShipSource, SpaceshipConfig, SpaceshipController, StoryMessageActionConfig,
    TimerFilterConfig,
};
use nova_ship::prelude::{
    BaseSectionConfig, GameSections, MuzzleConfig, RailgunSectionConfig, SectionConfig,
    SectionKind, SectionReloadConfig, ThrusterExhaust, ThrusterExhaustConfig, ThrusterExhaustShape,
    ThrusterSectionConfig, TorpedoSectionConfig, TurretJoint, TurretSectionConfig,
};

use super::*;
use crate::{
    event::{
        action_config_mut, expr_config_mut, ActionChoice, ActionKind, ActionNode, EventNode,
        ExprChoice, ExpressionNode, FilterChoice, FilterKind, FilterNode, ScriptNode, SequenceHead,
        StepNode,
    },
    node::{
        EditorNode, NextChildOrdinal, NodeId, ObjectNode, ScenarioNode, SectionNode, ShipDriver,
        ShipNode,
    },
};

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
        RowValue::Text(text) | RowValue::Number(text) | RowValue::Colour(text) => text.clone(),
        RowValue::Operand {
            text: Some(text), ..
        } => text.clone(),
        // A vector row reads as one line however many boxes it is typed in.
        RowValue::Axes(axes) => axes.join(", "),
        other => panic!("row {label:?} is {other:?}, not text"),
    }
}

/// What a value node HOLDS, read off the row the condition page draws it in.
fn leaf(node: &ExpressionNode) -> String {
    match operand_row(Entity::PLACEHOLDER, node, "Left", Operand::TestSide, 1).value {
        RowValue::Operand {
            text: Some(text), ..
        } => text,
        other => panic!("a value row holds text, not {other:?}"),
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
        radius: Meters(30.0),
        texture: default(),
        material: None,
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

    write(&mut object, &rows, "Radius", "125").expect("a radius");
    write(&mut object, &rows, "Invulnerable", "true").expect("a flag");

    let ScenarioObjectKind::Asteroid(tuned) = &object.kind else {
        panic!("still an asteroid");
    };
    assert!((tuned.radius - Meters(125.0)).get().abs() < f32::EPSILON);
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
            radius: Meters(30.0),
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
            radius: Meters(30.0),
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

    let RowValue::Choice {
        options, chosen, ..
    } = &row(&rows, "Geometry").value
    else {
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
    let RowValue::Choice {
        options, chosen, ..
    } = &row(&rows, "Geometry").value
    else {
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
    assert_eq!(rate.value, RowValue::Number("4".to_string()));
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

/// Magazine pacing is a first-screen weapon decision, with controls that move
/// in whole rounds and cannot author values the runtime rejects.
#[test]
fn weapons_open_on_valid_ammo_and_reload_controls() {
    let mut node = turret_with_muzzle(4.0);
    let SectionSource::Inline(config) = &mut node.source else {
        panic!("the fixture is inline");
    };
    let SectionKind::Turret(turret) = &mut config.kind else {
        panic!("the fixture is a turret");
    };
    turret.ammo_capacity = Some(12);
    turret.reload = Some(SectionReloadConfig {
        delay: 1.5,
        amount: 3,
    });

    let rows = curated_section_rows(&node, None);
    let capacity = row(&rows, "Ammo Capacity");
    assert_eq!(capacity.unit, "rounds");
    assert_eq!(capacity.nudge, 1.0);
    assert_eq!(capacity.limit, Limit::AtLeast(1.0));
    let delay = row(&rows, "Delay");
    assert_eq!(delay.unit, "s");
    assert_eq!(delay.nudge, 0.02);
    assert_eq!(delay.limit, Limit::AtLeast(0.02));
    let amount = row(&rows, "Amount");
    assert_eq!(amount.unit, "rounds");
    assert_eq!(amount.nudge, 1.0);
    assert_eq!(amount.limit, Limit::AtLeast(1.0));

    let mut config = node.resolve(None).expect("an inline section").clone();
    let refused = write_field(
        section_config_mut(&mut config.kind),
        &delay.path,
        false,
        "0",
    );
    assert_eq!(refused, Err("min 0.02".to_string()));

    let bay = SectionNode {
        source: SectionSource::Inline(SectionConfig {
            base: BaseSectionConfig {
                id: "torpedo".to_string(),
                ..default()
            },
            kind: SectionKind::Torpedo(TorpedoSectionConfig {
                ammo_capacity: Some(6),
                reload: Some(SectionReloadConfig {
                    delay: 10.0,
                    amount: 1,
                }),
                ..default()
            }),
        }),
        modifications: vec![],
        binds: vec![],
    };
    let bay_rows = curated_section_rows(&bay, None);
    for label in ["Ammo Capacity", "Delay", "Amount"] {
        assert!(
            bay_rows.iter().any(|row| row.label == label),
            "a torpedo bay exposes {label} on its first screen"
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

/// A picket is a ship a scenario stores as an object, and the two questions a
/// reader has about it are WHICH hull and WHO flies it. Before this it had no
/// config at all: the panel said Name, Position, Rotation and stopped.
#[test]
fn a_seeded_hull_opens_on_its_ship_and_its_driver() {
    let picket = ObjectNode {
        name: "Picket Warden".to_string(),
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            hull: ShipSource::Prototype("cargoa".to_string()),
            controller: SpaceshipController::AI(AIControllerConfig::default()),
            ..default()
        }),
    };

    let rows = curated_object_rows(&picket, &Transform::default());
    let said: Vec<(String, String)> = rows
        .iter()
        .map(|row| (row.label.clone(), row.value.reading()))
        .collect();

    assert!(
        said.contains(&("Hull".to_string(), "cargoa".to_string())),
        "the hull it flies, by catalog id: {said:?}"
    );
    assert!(
        said.contains(&("Controller".to_string(), "AI".to_string())),
        "and who is at the controls: {said:?}"
    );
}

/// The other half of the curation rule: a level that WAS picked keeps its
/// heading, so the fields under it say which level they belong to. The panel
/// reads the headings positionally against the path they were built from, and
/// a row that lost its own level would sit in the list as if it were the
/// node's own field.
#[test]
fn a_picked_level_keeps_the_heading_its_fields_sit_under() {
    let picket = ObjectNode {
        name: "Picket Warden".to_string(),
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            hull: ShipSource::Prototype("cargoa".to_string()),
            controller: SpaceshipController::AI(AIControllerConfig::default()),
            ..default()
        }),
    };

    let rows = curated_object_rows(&picket, &Transform::default());
    let under: Vec<(String, Vec<String>)> = rows
        .iter()
        .filter(|row| !row.group.is_empty())
        .map(|row| (row.label.clone(), row.group.clone()))
        .collect();

    assert!(
        under
            .iter()
            .any(|(_, group)| group.first().map(String::as_str) == Some("Controller")),
        "the AI's own fields stand under Controller, the level a builder picked: {under:?}"
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

    assert_eq!(row(&rows, "Radius").unit, "m", "a length reads in meters");
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
    assert_eq!(row(&rows, "Position").unit, "m");
    assert_eq!(row(&rows, "Rotation").unit, "deg, yaw/pitch/roll");
}

/// `nan` and `inf` parse as numbers and every writer downstream takes them, so
/// the box is where they have to stop. A position with a NaN in it is a node
/// that has left the world and cannot be edited back.
#[test]
fn a_number_that_is_not_finite_is_refused_wherever_it_is_typed() {
    // A pose axis, which has no floor rule at all - the case the floor check
    // cannot cover.
    let mut position = Vec3::new(1.0, 2.0, 3.0);
    let refusal = write_field(
        &mut position,
        &[PathStep::Field("x".to_string())],
        false,
        "nan",
    )
    .expect_err("a NaN is not a coordinate");
    assert_eq!(refusal, "finite");
    assert_eq!(position, Vec3::new(1.0, 2.0, 3.0), "the pose is left alone");

    write_field(
        &mut position,
        &[PathStep::Field("x".to_string())],
        false,
        "-40",
    )
    .expect("a finite coordinate still writes, including a negative one");
    assert!((position.x + 40.0).abs() < f32::EPSILON);

    // And on a field that DOES have a floor: an infinity is above every floor
    // there is, so the floor check would wave it through.
    let mut config = stock_asteroid();
    let refusal = write_field(
        &mut config,
        &[PathStep::Field("radius".to_string())],
        false,
        "inf",
    )
    .expect_err("an infinite radius is not a radius");
    assert_eq!(refusal, "finite");
    assert!((config.radius - stock_asteroid().radius).get().abs() < f32::EPSILON);
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
        (config.radius - stock_asteroid().radius).get().abs() < f32::EPSILON,
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
    assert!((config.radius - Meters(9.0)).get().abs() < f32::EPSILON);
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
        RowValue::Axes(["10".to_string(), "20".to_string(), "30".to_string()])
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

    assert_eq!(text_of(&rows, "Position"), "10, -25, 300");
    assert_eq!(row(&rows, "Position").root, FieldRoot::Pose);
}

#[test]
fn a_ship_reports_who_flies_it() {
    let rows = ship_rows(&ShipNode::default(), &Transform::default());

    assert_eq!(
        row(&rows, "Driver").value,
        RowValue::Driver(ShipDriver::Player)
    );
    assert_eq!(
        row(&rows, "Allegiance").value.reading(),
        IMPLIED_ALLEGIANCE,
        "a ship that states no side takes the one its driver implies"
    );
}

/// A seeded hull is a ship node like any other, and this is the screen the
/// complaint was about: a picket used to read out its whole flattened spawn
/// config - two Hull rows, a fixed controller and seven AI-tuning fields.
#[test]
fn a_seeded_hull_reads_as_a_ship_rather_than_as_a_spawn_config() {
    let picket = ShipNode {
        name: "Picket Warden".to_string(),
        driver: ShipDriver::Ai,
        allegiance: Some(Allegiance::Neutral),
        pilot: AIControllerConfig {
            leash: Some(Meters(4000.0)),
            ..default()
        },
        ..default()
    };

    let rows = ship_rows(&picket, &Transform::default());

    assert_eq!(text_of(&rows, "Name"), "Picket Warden");
    assert_eq!(row(&rows, "Driver").value, RowValue::Driver(ShipDriver::Ai));
    assert_eq!(
        row(&rows, "Allegiance").value.reading(),
        "Neutral",
        "what makes a picket dormant is on the screen that names it"
    );
    let names: Vec<&str> = rows.iter().map(|row| row.label.as_str()).collect();
    assert!(
        !names.contains(&"Leash") && !names.contains(&"Hull"),
        "and the pilot's tuning is carried, not shown: {names:?}"
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
            body_radius: Meters(50.0),
            mass: None,
        }),
    };
    let rows = object_rows(&object, &Transform::default());

    assert_eq!(text_of(&rows, "Body Radius"), "50", "shown in meters");
    assert!(
        row(&rows, "Mass").optional,
        "an absent number is still a row"
    );
}

#[test]
fn the_inspector_falls_back_to_the_node_you_are_standing_in() {
    let mut world = World::new();
    let scenario = world
        .spawn((
            EditorNode,
            ScenarioNode::default(),
            NodeId("scenario".to_string()),
        ))
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

/// One declaration per field. The lookup answers with the SAME declaration a
/// kind's first screen was built from, so a field cannot end up shown with one
/// rule and written with another.
#[test]
fn a_field_is_declared_exactly_once() {
    let mut seen: Vec<FieldSpec> = Vec::new();
    for spec in DECLARED.iter().copied().flatten() {
        // A name may appear in two lists - a fire rate is shown by a turret and
        // by a torpedo - but only as the SAME declaration referenced twice.
        if let Some(other) = seen.iter().find(|other| other.name == spec.name) {
            assert_eq!(
                other, spec,
                "{:?} is declared twice with different rules; a field is declared \
                 once and referenced from every list that shows it",
                spec.name
            );
            continue;
        }
        seen.push(*spec);
    }
    assert!(!seen.is_empty(), "the table is not empty");
}

/// A field a kind shows first carries its own unit and floor: a pick list and
/// the rules are one declaration, so a first screen cannot name a field and
/// leave its number in a bare box.
#[test]
fn every_field_a_kind_shows_carries_its_own_rule() {
    for spec in DECLARED.iter().copied().flatten() {
        // A family is not a field, so there is no name to look it up by. What
        // it covers is `a_named_field_beats_the_family_it_belongs_to`.
        if spec.name.starts_with('*') {
            continue;
        }
        let path = vec![PathStep::Field(spec.name.to_string())];
        assert_eq!(
            field_spec(&path),
            Some(*spec),
            "the lookup for {:?} answers with its own declaration",
            spec.name
        );
    }
}

/// A name declared in full beats a family, so a length that one day needs
/// saying something else can say it.
#[test]
fn a_named_field_beats_the_family_it_belongs_to() {
    let named = field_spec(&[PathStep::Field("blast_radius".to_string())]);
    assert_eq!(named, Some(BLAST_RADIUS));
    let family = field_spec(&[PathStep::Field("exhaust_radius".to_string())]);
    assert_eq!(family, Some(ANY_RADIUS));
}

/// A scrub of a WHOLE number stays whole: a seed is a name for a shape, and
/// there is no shape halfway between two of them.
#[test]
fn a_scrub_of_a_whole_number_stays_whole() {
    let mut config = AsteroidConfig {
        radius: Meters(30.0),
        texture: default(),
        material: None,
        destroy_sound: None,
        mass: None,
        invulnerable: false,
        seed: Some(7),
        lock_signature: None,
    };
    let path = vec![PathStep::Field("seed".to_string())];

    let rule = DragRule {
        step: 1.0,
        limit: Limit::Free,
    };
    nudge_field(&mut config, &path, true, rule, 2.4).expect("a seed scrubs");

    assert_eq!(config.seed, Some(9), "two and a bit pixels is two seeds on");
}

/// An UNSIGNED whole number stops at zero, and the floor is its TYPE's rather
/// than a declaration's: `seed` is `Limit::Free` because a seed is a name for a
/// shape and not a quantity, and there is still no `u32` below zero to walk
/// into. Without the type's own floor the scrub walked 3, 2, 1, 0 and then said
/// `not a u32` on every further pixel.
#[test]
fn a_scrub_of_an_unsigned_number_stops_at_zero() {
    let mut config = AsteroidConfig {
        radius: Meters(30.0),
        texture: default(),
        material: None,
        destroy_sound: None,
        mass: None,
        invulnerable: false,
        seed: Some(3),
        lock_signature: None,
    };
    let path = vec![PathStep::Field("seed".to_string())];
    let rule = DragRule {
        step: 1.0,
        limit: Limit::Free,
    };

    nudge_field(&mut config, &path, true, rule, -3.0).expect("a seed scrubs down");
    assert_eq!(
        config.seed,
        Some(0),
        "three pixels down is three seeds back"
    );

    nudge_field(&mut config, &path, true, rule, -1.0).expect("and the next pixel is not a refusal");
    assert_eq!(config.seed, Some(0), "it arrived at the end of its type");
}

/// An OPTIONAL field holding nothing has no number to move, and says the way
/// out rather than inventing one - or naming the Rust word for the hole.
#[test]
fn a_scrub_of_an_empty_optional_says_to_type_one() {
    let mut config = AsteroidConfig {
        radius: Meters(30.0),
        texture: default(),
        material: None,
        destroy_sound: None,
        mass: None,
        invulnerable: false,
        seed: None,
        lock_signature: None,
    };
    let path = vec![PathStep::Field("mass".to_string())];

    let rule = DragRule {
        step: 0.5,
        limit: Limit::AtLeast(0.0),
    };
    let refused = nudge_field(&mut config, &path, true, rule, 5.0);

    assert_eq!(refused, Err(GRIP_EMPTY.to_string()));
}

/// A handler is its own config: the trigger is a choice over every event name
/// and `once` is a checkbox, and this module names neither.
#[test]
fn a_handler_shows_its_trigger_as_a_choice() {
    let rows = event_rows(&EventNode {
        label: None,
        trigger: EventConfig::OnDestroyed,
        once: true,
    });

    let RowValue::Choice {
        options, chosen, ..
    } = &row(&rows, "Trigger").value
    else {
        panic!("the trigger is a choice");
    };
    // Against the ENUM, not against a list this test wrote out: the row is
    // built by reflection, and a hand-written vocabulary anywhere in the panel
    // is one that goes stale the day the engine grows an event.
    let TypeInfo::Enum(events) = EventConfig::type_info() else {
        panic!("an event name is an enum");
    };
    assert_eq!(
        options.len(),
        events.variant_len(),
        "every event is offered"
    );
    assert_eq!(options[*chosen], "OnDestroyed");
    assert_eq!(row(&rows, "Once").value, RowValue::Flag(true));
}

/// Switching the trigger is the same unit-variant write every other choice row
/// makes, so the handler needs no rule of its own.
#[test]
fn switching_the_trigger_writes_the_handler() {
    let mut event = EventNode::default();
    let path = vec![PathStep::Field("trigger".to_string())];

    choose_field(&mut event, &path, "OnTimerEnd").expect("the trigger takes");

    assert!(matches!(event.trigger, EventConfig::OnTimerEnd));
}

/// A filter says which filter it is and then shows its own fields. An unset
/// optional id is an empty box: it matches anything, and a box reading `none`
/// would say the filter had been given something.
#[test]
fn a_filter_shows_the_ids_it_matches_on() {
    let rows = filter_rows(&FilterNode {
        kind: FilterKind::Entity(EntityFilterConfig {
            id: Some("raider_1".to_string()),
            ..default()
        }),
    });

    assert_eq!(row(&rows, "Filter").value.reading(), "Entity");
    assert_eq!(text_of(&rows, "Id"), "raider_1");
    assert_eq!(text_of(&rows, "Other Id"), "");
}

/// A combinator holds nothing to author: what it combines are its child rows
/// in the tree, and the panel says so rather than showing an empty config.
#[test]
fn a_combinator_shows_only_what_it_is() {
    let rows = filter_rows(&FilterNode {
        kind: FilterKind::And,
    });

    assert_eq!(rows.len(), 1);
    assert_eq!(row(&rows, "Filter").value.reading(), "And");
}

/// The kind row is the one row that is not a FIELD: it offers every filter
/// there is, and switching it swaps the config the other rows are walked from.
#[test]
fn a_filter_is_switched_to_any_other_kind_from_its_own_row() {
    let rows = filter_rows(&FilterNode {
        kind: FilterKind::Timer(TimerFilterConfig {
            key: "patrol".to_string(),
        }),
    });

    let kind = row(&rows, "Filter");
    assert_eq!(kind.root, FieldRoot::Kind);
    assert!(kind.path.is_empty(), "a kind is not reached through a path");
    let RowValue::Choice {
        options, chosen, ..
    } = &kind.value
    else {
        panic!("the kind is a choice");
    };
    assert_eq!(
        options.len(),
        FilterChoice::ALL.len(),
        "every filter is offered"
    );
    assert_eq!(options[*chosen], "Timer");
}

/// The same row, over the vocabulary the Add menu does not list: five rows
/// there, twenty-six kinds here.
#[test]
fn an_action_is_switched_to_any_other_kind_from_its_own_row() {
    let rows = action_rows(&ActionNode {
        kind: ActionChoice::Outcome.stock(),
    });

    let kind = row(&rows, "Action");
    assert_eq!(kind.root, FieldRoot::Kind);
    let RowValue::Choice {
        options, chosen, ..
    } = &kind.value
    else {
        panic!("the kind is a choice");
    };
    assert_eq!(
        options.len(),
        ActionChoice::ALL.len(),
        "every action is offered"
    );
    assert_eq!(options[*chosen], "Outcome");
}

/// The panel says what a field is FOR out of the doc comment the config author
/// wrote beside it - the whole point of turning `reflect_documentation` on.
#[test]
fn a_row_explains_itself_from_the_config_that_declares_it() {
    let rows = action_rows(&ActionNode {
        kind: ActionChoice::StoryMessage.stock(),
    });

    assert!(
        row(&rows, "Speaker").hint.starts_with("Who says it"),
        "the speaker's own doc: {:?}",
        row(&rows, "Speaker").hint
    );
}

/// A CHOICE is explained by the variant it holds, not by the field that holds
/// it: a row already reading Victory does not need told it is an outcome.
#[test]
fn a_choice_explains_the_option_it_is_on() {
    let rows = action_rows(&ActionNode {
        kind: ActionChoice::Outcome.stock(),
    });

    assert!(
        row(&rows, "Outcome").hint.contains("The player won"),
        "the variant's own doc: {:?}",
        row(&rows, "Outcome").hint
    );
}

/// A sequence shows the KEY its gates name it by and not its steps: the steps
/// are rows of the tree, and a panel listing them would be a second place they
/// could be edited.
#[test]
fn a_sequence_shows_its_key_and_not_its_steps() {
    let rows = action_rows(&ActionNode {
        kind: ActionKind::Sequence(SequenceHead {
            key: "briefing".to_string(),
        }),
    });

    assert_eq!(row(&rows, "Action").value.reading(), "Sequence");
    assert_eq!(text_of(&rows, "Key"), "briefing");
    assert_eq!(rows.len(), 2, "and nothing else: {rows:?}");
}

/// An expression filter has no config of its own: its condition is the nodes
/// under it, and a row holding a second copy is a second place to edit it.
#[test]
fn an_expression_filter_is_switched_and_nothing_else() {
    let rows = filter_rows(&FilterNode {
        kind: FilterChoice::Expression.stock(),
    });

    assert_eq!(row(&rows, "Filter").value.reading(), "Expression");
    assert_eq!(rows.len(), 1, "the condition is the tree: {rows:?}");
}

/// The variables DSL is a LEAF, not a struct to be taken apart: a value node is
/// authored as the text a RON file carries, and typing another expression into
/// it parses.
#[test]
fn an_expression_is_authored_as_its_own_syntax() {
    let mut node = ExpressionNode {
        kind: ExprChoice::Value.stock(),
    };
    assert_eq!(leaf(&node), "0");

    let config = expr_config_mut(&mut node.kind).expect("a value has a config");
    write_field(
        config,
        &[PathStep::Field("value".to_string())],
        false,
        "scenario.elapsed",
    )
    .expect("the expression takes");

    assert_eq!(leaf(&node), "scenario.elapsed");
}

/// And an expression the grammar cannot read is refused with the reason, rather
/// than silently leaving the old one in place.
#[test]
fn an_unreadable_expression_says_why() {
    let mut node = ExpressionNode {
        kind: ExprChoice::Value.stock(),
    };
    let config = expr_config_mut(&mut node.kind).expect("a value has a config");

    let refused = write_field(
        config,
        &[PathStep::Field("value".to_string())],
        false,
        "scenario.elapsed +",
    );

    assert!(refused.is_err(), "an unfinished sum is not an expression");
    assert_eq!(leaf(&node), "0");
}

/// The operator row offers what BELONGS where the node stands: a comparison at
/// the root of a condition, arithmetic and values under one.
#[test]
fn an_operator_is_offered_the_kinds_its_place_allows() {
    let node = ExpressionNode {
        kind: ExprChoice::Equal.stock(),
    };

    let compared = operand_row(Entity::PLACEHOLDER, &node, "Compare", Operand::Test, 0);
    let RowValue::Operand {
        options, chosen, ..
    } = &compared.value
    else {
        panic!("an operand row offers its kinds");
    };
    assert_eq!(options, &["==", "<", ">"], "a condition compares");
    assert_eq!(*chosen, 0);

    let valued = operand_row(Entity::PLACEHOLDER, &node, "Left", Operand::TestSide, 1);
    let RowValue::Operand { options, .. } = &valued.value else {
        panic!("an operand row offers its kinds");
    };
    assert_eq!(
        options,
        &["+", "-", "*", "/", "value"],
        "and what it compares are values"
    );

    // What an action WRITES is a value too, all the way to its root: the
    // grammar has no way to spell a comparison into a variable.
    let written = operand_row(Entity::PLACEHOLDER, &node, "Writes", Operand::Value, 0);
    let RowValue::Operand { options, .. } = &written.value else {
        panic!("an operand row offers its kinds");
    };
    assert_eq!(options, &["+", "-", "*", "/", "value"]);
    assert_eq!(compared.group, ["Condition"], "and each page is headed");
    assert_eq!(written.group, ["Value"]);
}

/// An asset reference is a leaf too: the path under `assets/`, which is what a
/// hand-written mod carries. Without this a Set Skybox action is one greyed
/// row of debug text and nothing to type into.
#[test]
fn an_asset_reference_is_authored_as_its_path() {
    let mut node = ActionNode {
        kind: ActionChoice::SetSkybox.stock(),
    };
    let config = action_config_mut(&mut node.kind).expect("a skybox has a config");

    write_field(
        config,
        &[PathStep::Field("cubemap".to_string())],
        false,
        "scenarios/deep.cube.png",
    )
    .expect("the path takes");

    assert_eq!(
        text_of(&action_rows(&node), "Cubemap"),
        "scenarios/deep.cube.png"
    );
}

/// A beat of a sequence is two optional clocks, and an empty box is what
/// clears one - the same gesture an unauthored mass takes.
#[test]
fn a_step_clears_a_deadline_with_an_empty_box() {
    let mut step = StepNode {
        after: Some(2.0),
        deadline: Some(30.0),
    };
    assert_eq!(text_of(&step_rows(&step), "Deadline"), "30");

    let path = vec![PathStep::Field("deadline".to_string())];
    write_field(&mut step, &path, true, "").expect("the clear takes");

    assert_eq!(step.deadline, None);
    assert_eq!(text_of(&step_rows(&step), "Deadline"), "");
    assert_eq!(step.after, Some(2.0), "and the other clock is untouched");
}

/// A row that names something says WHAT it names, straight off the field's own
/// attribute. That is the whole of what the picker beside it needs, so a config
/// that grows a reference grows a picker without a line here changing.
#[test]
fn a_row_that_names_an_object_says_so() {
    let rows = filter_rows(&FilterNode {
        kind: FilterKind::Entity(EntityFilterConfig::default()),
    });

    assert_eq!(row(&rows, "Id").names, Some(Names::Object));
    assert_eq!(
        row(&rows, "Type Name").names,
        None,
        "a type name is a class of thing, not one of them"
    );
}

/// The document, read for its names: a rock on the board, a ship, and the id an
/// action of the script declares - one list, because a handler names the rock
/// the tree put down as readily as one another handler spawns.
fn named_document() -> World {
    let mut world = World::new();
    let scenario = world
        .spawn((
            EditorNode,
            ScenarioNode::default(),
            NodeId("scenario".to_string()),
            NextChildOrdinal::default(),
        ))
        .id();
    world.insert_resource(EditContext {
        path: vec![scenario],
    });
    world.spawn((
        EditorNode,
        ObjectNode {
            name: "rock".to_string(),
            kind: ScenarioObjectKind::Asteroid(stock_asteroid()),
        },
        NodeId("asteroid_1".to_string()),
        Transform::default(),
        ChildOf(scenario),
    ));
    world.spawn((
        EditorNode,
        ShipNode {
            driver: ShipDriver::Ai,
            ..default()
        },
        NodeId("raider_1".to_string()),
        ChildOf(scenario),
    ));
    let script = world
        .spawn((
            EditorNode,
            ScriptNode,
            NodeId("script".to_string()),
            ChildOf(scenario),
        ))
        .id();
    let handler = world
        .spawn((
            EditorNode,
            EventNode {
                label: None,
                trigger: EventConfig::OnStart,
                once: true,
            },
            NodeId("event_1".to_string()),
            ChildOf(script),
        ))
        .id();
    world.spawn((
        EditorNode,
        ActionNode {
            kind: ActionKind::Leaf(EventActionConfig::CreateScenarioArea(ScenarioAreaConfig {
                id: "trap".to_string(),
                name: "TRAP".to_string(),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
                radius: Meters(100.0),
            })),
        },
        NodeId("area_1".to_string()),
        ChildOf(handler),
    ));
    world
}

fn document_names(world: &mut World) -> DocumentNames {
    world
        .run_system_once(|ids: DocumentIds| ids.names())
        .expect("the system runs")
}

/// What the picker offers is what the DOCUMENT holds - the world half and the
/// script half both - because that is the set the lowering judges a reference
/// against.
#[test]
fn the_picker_offers_every_id_the_document_holds() {
    let mut world = named_document();

    let offered = document_names(&mut world).offers(Names::Object);

    assert_eq!(
        offered,
        vec![
            "asteroid_1".to_string(),
            "raider_1".to_string(),
            "trap".to_string()
        ],
        "the rock on the board, the ship, and the area the script creates"
    );
}

/// A DECLARATION is offered nothing. An id has to be unique, so a list of the
/// ones already taken is a list of mistakes.
#[test]
fn a_declared_id_is_offered_no_choices() {
    let mut world = named_document();

    assert!(document_names(&mut world)
        .offers(Names::NewObject)
        .is_empty());
}

/// The fault the panel paints: a reference naming nothing the document spawns
/// is the reference the lowering drops the handler for.
#[test]
fn an_id_nothing_spawns_does_not_resolve() {
    let mut world = named_document();
    let names = document_names(&mut world);

    assert!(names.resolves(Names::Object, "asteroid_1"));
    assert!(!names.resolves(Names::Object, "asteroid_2"));
    assert!(
        names.resolves(Names::Object, ""),
        "an unset optional matches anything, and is not a mistake"
    );
    assert!(
        names.resolves(Names::Variable, "never_written"),
        "a variable is made by the handler that first writes it"
    );
}

/// A row holding an asset ref knows what KIND of file it wants, off the type of
/// the field rather than off its name - which is what lets the panel offer the
/// images the installed bundles ship without a list of field names here.
#[test]
fn a_row_that_names_a_file_says_what_kind_of_file() {
    let action = ActionNode {
        kind: ActionKind::Leaf(EventActionConfig::StoryMessage(StoryMessageActionConfig {
            speaker: "Alpha".to_string(),
            text: "Strip it clean.".to_string(),
            dwell: None,
            icon: Some("dep://base/icons/alpha.png".into()),
        })),
    };
    let rows = action_rows(&action);

    assert_eq!(
        row(&rows, "Icon").asset,
        Some(AssetSort::Image),
        "the field is an `AssetRef<Image>`; got {:?}",
        row(&rows, "Icon").value
    );
    assert_eq!(
        row(&rows, "Speaker").asset,
        None,
        "a line of text names no file"
    );
}

/// A whole field scrubs by whole numbers whether or not anything declares it.
///
/// `count` has no declaration, and the drag lands through `snapped`, which
/// rounds a whole value: at the undeclared step of a tenth a scatter's count
/// travelled and rounded straight back, so the grip did nothing at all.
#[test]
fn a_whole_field_scrubs_by_one_with_nothing_declared_about_it() {
    let node = ActionNode {
        kind: ActionChoice::ScatterObjects.stock(),
    };
    let rows = action_rows(&node);

    let count = row(&rows, "Count");
    assert_eq!(count.nudge, 1.0, "an undeclared u32 still drags whole");
    assert_eq!(count.unit, "", "and gains no unit it was never given");

    let mut config = node.kind.clone();
    let target = action_config_mut(&mut config).expect("a scatter carries a config");
    nudge_field(
        target,
        &count.path,
        count.optional,
        DragRule {
            step: count.nudge,
            limit: count.limit,
        },
        1.0,
    )
    .expect("one step up");
    assert_eq!(
        text_of(&action_rows(&ActionNode { kind: config }), "Count"),
        "9",
        "one step of the row's own rule moves the number by one"
    );
}

/// And a fractional field keeps the step it declares: the floor is a floor,
/// not a rounding of every number in the panel.
#[test]
fn a_declared_fractional_field_keeps_its_own_step() {
    let rows = action_rows(&ActionNode {
        kind: ActionChoice::ScatterObjects.stock(),
    });

    let seed = row(&rows, "Seed");
    assert_eq!(seed.nudge, 1.0, "a seed names a shape, one whole per pixel");

    let turret = turret_with_muzzle(4.0);
    let turret_rows = curated_section_rows(&turret, None);
    assert_eq!(
        row(&turret_rows, "Fire Rate").nudge,
        0.05,
        "a rate is a fraction and stays one"
    );
}

/// A length row is the number the FILE holds, with nothing applied between
/// them. The box used to show ten times the authored figure because the file
/// counted world units; both are meters now, and a factor of ten left anywhere
/// in the panel would be a rock ten times the size the builder typed.
#[test]
fn a_length_row_reads_back_the_number_it_was_typed_with() {
    let mut object = asteroid(stock_asteroid());
    let rows = object_rows(&object, &Transform::IDENTITY);
    assert_eq!(
        text_of(&rows, "Radius"),
        "30",
        "the authored meters, as written"
    );

    write(&mut object, &rows, "Radius", "125").expect("a radius in meters");
    let ScenarioObjectKind::Asteroid(tuned) = &object.kind else {
        panic!("still an asteroid");
    };
    assert_eq!(tuned.radius, Meters(125.0), "the file holds what was typed");
    assert_eq!(
        text_of(&object_rows(&object, &Transform::IDENTITY), "Radius"),
        "125",
        "and the row reads it back unchanged"
    );
    assert_eq!(row(&rows, "Radius").unit, "m");
}

/// A quantity is a tuple struct wrapping one number, and the panel shows the
/// NUMBER: walked as the struct it is, a radius would be an unopenable row
/// with the value hidden in a child called "0".
#[test]
fn a_quantity_is_one_row_holding_the_number_inside_it() {
    let rows = object_rows(&asteroid(stock_asteroid()), &Transform::IDENTITY);

    let radius = row(&rows, "Radius");
    assert_eq!(radius.value, RowValue::Number("30".to_string()));
    assert!(
        !rows.iter().any(|row| row.label == "0"),
        "no row is the wrapper's own slot: {:?}",
        rows.iter().map(|row| &row.label).collect::<Vec<_>>()
    );
    assert_eq!(
        radius.path,
        vec![PathStep::Field("radius".to_string())],
        "and the row stands at the field's own path, not one slot inside it"
    );
}

/// An OPTIONAL quantity is an optional number: one box that takes meters and
/// clears to `None`, not a struct to open.
#[test]
fn an_optional_quantity_is_typed_and_cleared_like_any_other_number() {
    let mut config = RailgunSectionConfig::default();
    let mut rows = Vec::new();
    walk(&config, FieldRoot::Config, Vec::new(), &mut rows);

    let rake = row(&rows, "Rake Radius").clone();
    assert!(rake.optional, "an absent length is still a number's row");
    assert_eq!(rake.value, RowValue::Number(String::new()));
    assert_eq!(rake.unit, "m");
    assert!(rake.nudge > 0.0, "and it carries the grip it will need");

    write_field(&mut config, &rake.path, rake.optional, "45").expect("a rake in meters");
    assert_eq!(config.rake_radius, Some(Meters(45.0)));

    let refusal = write_field(&mut config, &rake.path, rake.optional, "-1")
        .expect_err("a negative rake is not a rake");
    assert_eq!(refusal, "min 0");

    write_field(&mut config, &rake.path, rake.optional, "  ").expect("blank clears it");
    assert_eq!(config.rake_radius, None);
}

/// A DISPLACEMENT wears its unit because its type says so, not because a list
/// here names the field: `position` is meters on a spawn action and a
/// build-grid cell on a ship's section, and only the type tells them apart.
#[test]
fn an_authored_displacement_reads_in_meters_and_writes_one_axis() {
    let mut config = ScenarioAreaConfig {
        id: "trap".to_string(),
        name: "TRAP".to_string(),
        position: Meters3::new(10.0, -25.0, 300.0),
        rotation: Quat::IDENTITY,
        radius: Meters(100.0),
    };
    let mut rows = Vec::new();
    walk(&config, FieldRoot::Config, Vec::new(), &mut rows);

    let position = row(&rows, "Position").clone();
    assert_eq!(
        position.value,
        RowValue::Axes(["10".to_string(), "-25".to_string(), "300".to_string()])
    );
    assert_eq!(position.unit, "m");
    assert_eq!(position.nudge, POSE_STEP, "and it drags like a node's pose");

    let mut axis = position.path.clone();
    axis.push(axis_step(1));
    write_field(&mut config, &axis, false, "80").expect("one axis writes");
    assert_eq!(config.position, Meters3::new(10.0, 80.0, 300.0));
}

/// A section's own GEOMETRY is not a distance a pilot reads. The exhaust cone
/// is a mesh built inside the section's build-grid cell, so its rows say
/// cells - and the panel must not offer a builder meters for a number the
/// renderer will spend in world units.
#[test]
fn a_section_mesh_reads_in_build_grid_cells_not_meters() {
    let mut rows = Vec::new();
    walk(
        &ThrusterExhaust::default(),
        FieldRoot::Config,
        Vec::new(),
        &mut rows,
    );

    for label in ["Width", "Height", "Exhaust Height", "Exhaust Radius"] {
        assert_eq!(
            row(&rows, label).unit,
            "cells",
            "{label} sizes a mesh, not a distance"
        );
    }
    assert_eq!(
        row(&rows, "Offset").unit,
        "",
        "and the cone's own offset is a cell too, so nothing labels it meters"
    );
}

/// A length drags in METERS, so the physical distance one pixel covers is what
/// it always was over a file that counted world units.
#[test]
fn a_length_drags_in_the_unit_it_is_shown_in() {
    let mut config = stock_asteroid();
    let path = vec![PathStep::Field("radius".to_string())];
    let rule = DragRule {
        step: RADIUS.step,
        limit: RADIUS.limit,
    };

    nudge_field(&mut config, &path, false, rule, 20.0).expect("a radius scrubs");

    assert_eq!(config.radius, Meters(40.0), "twenty pixels is ten meters");
}
