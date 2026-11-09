//! The reflection walk and the write-back, tested against real configs rather
//! than a fixture struct: the whole claim of this module is that it works on
//! whatever content declares, so a test that walks its own toy type would not
//! be testing it.

use bevy::{ecs::system::RunSystemOnce, reflect::Typed};
use nova_gameplay::prelude::Allegiance;
use nova_scenario::prelude::{
    AIControllerConfig, AnchorConfig, AsteroidConfig, BeaconConfig, EntityFilterConfig,
    EventActionConfig, EventConfig, LightConfig, Names, NarrativeCueActionConfig,
    OrbitShipActionConfig, ScenarioAreaConfig, ScenarioObjectKind, SectionSource, ShipDesignSource,
    SpaceshipConfig, SpaceshipController, TimerFilterConfig, ASTEROID_KINDS, KIND_ICE, KIND_ROCK,
};
use nova_ship::prelude::{
    AmmoCapacity, BaseSectionConfig, GameSections, MuzzleConfig, MuzzleConfigPatch,
    RailgunSectionConfig, ReloadConfig, SectionConfig, SectionKind, SectionKindPatch,
    SectionReloadConfig, ThrusterExhaust, ThrusterExhaustConfig, ThrusterExhaustShape,
    ThrusterSectionConfig, ThrusterSectionConfigPatch, TorpedoSectionConfig, TurretJoint,
    TurretSectionConfig, TurretSectionConfigPatch, WellTargetType,
};

use super::*;
use crate::{
    event::{
        action_config, action_config_mut, expr_config_mut, filter_config_mut, ActionChoice,
        ActionChoiceExt, ActionKind, ActionNode, EventNode, ExprChoice, ExpressionNode,
        FilterChoice, FilterKind, FilterNode, ScriptNode, SequenceHead, StepNode,
    },
    node::{
        EditorNode, NextChildOrdinal, NodeId, ObjectChoice, ObjectNode, ScenarioNode, SectionNode,
        ShipDriver, ShipNode,
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
            id: "main".to_string(),
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
        initial_velocity: MetersPerSecond3::ZERO,
        texture: default(),
        kind: KIND_ROCK.into(),
        destroy_sound: None,
        seed: None,
        lock_signature: None,
    }
}

/// The row of this name holding a NUMBER. A payload enum draws two rows under
/// one name - the variant it is, then the number inside it - and a test that
/// asks about a unit or a floor is asking about the number.
fn number_row<'a>(rows: &'a [InspectorRow], label: &str) -> &'a InspectorRow {
    rows.iter()
        .find(|row| row.label == label && matches!(row.value, RowValue::Number(_)))
        .unwrap_or_else(|| {
            panic!(
                "no number row {label:?}; found {:?}",
                rows.iter().map(|row| &row.label).collect::<Vec<_>>()
            )
        })
}

/// Write into a SECTION the way the Inspector's edit routing does: through
/// [`edit_section`], so an inline part is written in place and a prototype
/// reference keeps its id and carries a patch.
fn write_section(
    node: &mut SectionNode,
    catalog: Option<&GameSections>,
    row: &InspectorRow,
    text: &str,
) -> Result<(), String> {
    edit_section(node, catalog, |config| {
        write_field(
            section_config_mut(&mut config.kind),
            &row.path,
            row.optional,
            text,
        )
    })
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

    write_section(&mut node, None, &magnitude, "250.5").expect("a number the field takes");

    let config = node.patched(None).expect("an inline section");
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

    let refused = write_section(&mut node, None, &magnitude, "fast");

    assert!(refused.is_err(), "'fast' is not a number");
    let config = node.patched(None).expect("an inline section");
    let SectionKind::Thruster(untouched) = &config.kind else {
        panic!("still a thruster");
    };
    assert!((untouched.magnitude - 120.0).abs() < f32::EPSILON);
}

/// What a prototype instance writes: a PATCH on the reference, not a copy of
/// the part. The document keeps naming the catalog entry, so a later change to
/// that entry still reaches every field this placement did not touch.
#[test]
fn editing_a_catalog_section_patches_the_reference() {
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
        source: SectionSource::prototype("thruster"),
        binds: vec![],
    };
    // The prototype's fields are readable before any of this.
    assert_eq!(
        text_of(&section_rows(&node, Some(&catalog)), "Magnitude"),
        "40"
    );

    let magnitude = row(&section_rows(&node, Some(&catalog)), "Magnitude").clone();
    write_section(&mut node, Some(&catalog), &magnitude, "95").expect("a number the field takes");

    let SectionSource::Prototype { id, patch } = &node.source else {
        panic!("the reference is kept: {:?}", node.source);
    };
    assert_eq!(id, "thruster", "still the catalog part it names");
    assert_eq!(
        patch.kind,
        Some(SectionKindPatch::Thruster(ThrusterSectionConfigPatch {
            magnitude: Some(95.0),
        })),
        "and the edit is the delta on it"
    );
    // The row a builder reads is the patched one.
    assert_eq!(
        text_of(&section_rows(&node, Some(&catalog)), "Magnitude"),
        "95"
    );
    // The catalog entry is untouched: an edit to the id would be an edit to
    // every ship that names it.
    assert!(matches!(
        &catalog.get_section("thruster").expect("still listed").kind,
        SectionKind::Thruster(entry) if (entry.magnitude - 40.0).abs() < f32::EPSILON
    ));
}

#[test]
fn an_optional_number_is_one_row_that_empties_to_none() {
    let mut object = asteroid(stock_asteroid());
    let rows = object_rows(&object, &Transform::default());

    assert_eq!(
        text_of(&rows, "Lock Signature"),
        "",
        "an unset override reads empty"
    );

    write(&mut object, &rows, "Lock Signature", "8000").expect("a number the field takes");
    let ScenarioObjectKind::Asteroid(authored) = &object.kind else {
        panic!("still an asteroid");
    };
    assert_eq!(authored.lock_signature, Some(Meters(8000.0)));

    let rows = object_rows(&object, &Transform::default());
    assert_eq!(text_of(&rows, "Lock Signature"), "8000");

    write(&mut object, &rows, "Lock Signature", "  ").expect("blank clears it");
    let ScenarioObjectKind::Asteroid(cleared) = &object.kind else {
        panic!("still an asteroid");
    };
    assert_eq!(cleared.lock_signature, None);
}

#[test]
fn a_placed_rock_keeps_what_the_inspector_wrote_on_it() {
    let mut object = asteroid(stock_asteroid());
    let rows = object_rows(&object, &Transform::default());

    write(&mut object, &rows, "Radius", "125").expect("a radius");

    let ScenarioObjectKind::Asteroid(tuned) = &object.kind else {
        panic!("still an asteroid");
    };
    assert!((tuned.radius - Meters(125.0)).get().abs() < f32::EPSILON);
}

/// The kind decides the whole surface of the rock, and nobody guesses `carbon`
/// from an empty text box. The row is the vocabulary.
#[test]
fn the_kind_reads_as_a_pick_list_of_the_shipped_kinds() {
    let rows = object_rows(&asteroid(stock_asteroid()), &Transform::default());

    let RowValue::Choice {
        options,
        hints,
        chosen,
    } = &row(&rows, "Kind").value
    else {
        panic!("the kind is {:?}, not a choice", row(&rows, "Kind").value);
    };
    assert_eq!(options.as_slice(), ASTEROID_KINDS);
    assert_eq!(options[*chosen], KIND_ROCK);
    assert!(
        hints.iter().all(|hint| !hint.is_empty()),
        "a name a builder has never seen needs a line saying what it is: {hints:?}"
    );
}

/// An id the game does not ship is a document the lint refuses. The panel says
/// what the file says rather than snapping it to the first option, because a
/// picker that read `rock` there would hide the broken file behind a control
/// that looks like it works.
#[test]
fn a_kind_the_game_does_not_ship_stays_the_text_it_is() {
    let rock = asteroid(AsteroidConfig {
        kind: "obsidian".into(),
        ..stock_asteroid()
    });

    let rows = object_rows(&rock, &Transform::default());

    assert_eq!(text_of(&rows, "Kind"), "obsidian");
}

/// The picker writes a STRING, which the walk had no path for: every choice
/// before this one switched an enum.
#[test]
fn choosing_a_kind_writes_the_id_into_the_config() {
    let mut object = asteroid(stock_asteroid());
    let rows = object_rows(&object, &Transform::default());
    let path = row(&rows, "Kind").path.clone();

    let config = object_config_mut(&mut object.kind).expect("a rock has a config");
    choose_field(config, &path, KIND_ICE).expect("the kind takes an id");

    let ScenarioObjectKind::Asteroid(tuned) = &object.kind else {
        panic!("still an asteroid");
    };
    assert_eq!(tuned.kind.as_str(), KIND_ICE);
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

/// A node's OWN kind is not switched from its rows: an object's kind config
/// is the node's kind, and the palette is what places one.
#[test]
fn a_nodes_own_kind_variant_is_a_readout() {
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
    turret.ammunition = AmmoCapacity::Limited(12);
    turret.reload = ReloadConfig::Batch(SectionReloadConfig {
        delay: 1.5,
        amount: 3,
    });

    let rows = curated_section_rows(&node, None);
    // A magazine is an `AmmoCapacity`, so it draws two rows: WHICH kind of
    // magazine it is, and - on a limited one - how many rounds are in it.
    let capacity = number_row(&rows, "Ammunition");
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
                ammunition: AmmoCapacity::Limited(6),
                reload: ReloadConfig::Batch(SectionReloadConfig {
                    delay: 10.0,
                    amount: 1,
                }),
                ..default()
            }),
        }),
        binds: vec![],
    };
    let bay_rows = curated_section_rows(&bay, None);
    for label in ["Ammunition", "Delay", "Amount"] {
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
        labels.contains(&"Kind".to_string()),
        "and by what it is MADE of, which is what decides its whole surface: {labels:?}"
    );
    assert!(
        labels.contains(&"Initial Velocity".to_string()),
        "a dynamic rock's starting motion must be visible: {labels:?}"
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
            design: ShipDesignSource::prototype("interceptor"),
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
        said.contains(&("Id".to_string(), "interceptor".to_string())),
        "the design it flies, by catalog id: {said:?}"
    );
    assert!(
        said.contains(&("Controller".to_string(), "AI".to_string())),
        "and who is at the controls: {said:?}"
    );
    assert!(
        said.iter().any(|(label, _)| label == "Initial Velocity"),
        "a ship's starting motion must be visible: {said:?}"
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
            design: ShipDesignSource::prototype("interceptor"),
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

    let anchor_rows = object_rows(
        &ObjectNode {
            name: "anchor".to_string(),
            kind: ScenarioObjectKind::Anchor(AnchorConfig {
                body_radius: Meters(50.0),
                mass: Some(1_000.0),
            }),
        },
        &Transform::default(),
    );
    assert_eq!(
        row(&anchor_rows, "Mass").unit,
        "",
        "a mass has a floor, not a unit"
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

    let mut anchor = AnchorConfig {
        body_radius: Meters(50.0),
        mass: None,
    };
    let refusal = write_field(
        &mut anchor,
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
    let rows = ship_rows(&ShipNode::default(), Some(&Transform::default()), &[]);

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

    let rows = ship_rows(&picket, Some(&Transform::default()), &[]);

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
    let rows = ship_rows(&ship, Some(&Transform::default()), &[]);

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
        world.insert_resource(SelectedNode::of(selected));
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
        initial_velocity: MetersPerSecond3::ZERO,
        texture: default(),
        kind: KIND_ROCK.into(),
        destroy_sound: None,
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
        initial_velocity: MetersPerSecond3::ZERO,
        texture: default(),
        kind: KIND_ROCK.into(),
        destroy_sound: None,
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
        initial_velocity: MetersPerSecond3::ZERO,
        texture: default(),
        kind: KIND_ROCK.into(),
        destroy_sound: None,
        seed: None,
        lock_signature: None,
    };
    let path = vec![PathStep::Field("lock_signature".to_string())];

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
        position: None,
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

/// An entity filter's type name takes only the names the game spawns, so it
/// is a choice with an empty first option. A word the game does not know stays
/// text, showing what the file says.
#[test]
fn an_entity_type_name_is_a_choice_of_what_the_game_spawns() {
    let rows = filter_rows(&FilterNode {
        kind: FilterKind::Entity(EntityFilterConfig {
            type_name: Some(nova_events::SPACESHIP_TYPE_NAME.to_string()),
            ..default()
        }),
    });
    let RowValue::Choice {
        options, chosen, ..
    } = &row(&rows, "Type Name").value
    else {
        panic!("a choice: {:?}", row(&rows, "Type Name").value);
    };
    assert_eq!(options[*chosen], nova_events::SPACESHIP_TYPE_NAME);
    assert_eq!(options.len(), 9, "nothing, then the eight spawned names");

    let rows = filter_rows(&FilterNode {
        kind: FilterKind::Entity(EntityFilterConfig {
            type_name: Some("bogus".to_string()),
            ..default()
        }),
    });
    assert_eq!(text_of(&rows, "Type Name"), "bogus");
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
        kind: ActionChoice::NarrativeCue.stock(),
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
                position: None,
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
        kind: ActionKind::Leaf(EventActionConfig::NarrativeCue(NarrativeCueActionConfig {
            accent: nova_gameplay::prelude::default_comms_accent(),
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

/// Only a quantity the units module owns names a unit.
///
/// The shape check admits any one-scalar tuple struct, because that is what
/// decides the CONTROL. The unit is a second question with a narrower answer:
/// labelling an unknown wrapper `m` would state a dimension nobody declared.
#[test]
fn a_wrapper_that_is_not_a_quantity_is_labelled_with_no_unit() {
    assert_eq!(quantity_unit("nova_events::units::Meters"), Some("m"));
    assert_eq!(quantity_unit("nova_events::units::Meters3"), Some("m"));
    assert_eq!(
        quantity_unit("nova_events::units::MetersPerSecond"),
        Some("m/s")
    );
    assert_eq!(
        quantity_unit("nova_events::units::MetersPerSecond3"),
        Some("m/s")
    );
    assert_eq!(
        quantity_unit("nova_events::units::MetersPerSecondSquared"),
        Some("m/s2")
    );
    assert_eq!(
        quantity_unit("nova_gameplay::health::Health"),
        None,
        "a one-scalar wrapper nobody declared a dimension for"
    );

    let mut rows = vec![walked_number(
        FieldRoot::Config,
        Vec::new(),
        false,
        "1".to_string(),
        false,
    )];
    declare_by_type("nova_gameplay::health::Health", &mut rows);
    assert_eq!(rows[0].unit, "", "and its row is left as the walk made it");
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
    assert!(
        position.framed,
        "and it drags at the scale the camera is framing, like a node's pose"
    );

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

/// Every action carries the sentence a builder reads beside it in the picker,
/// and the sentence comes off the action's own table row. A row added with no
/// doc comment would list a bare name in a menu forty-odd entries long - the
/// exact thing the hint column exists to stop.
#[test]
fn every_action_and_filter_offers_the_sentence_it_was_documented_with() {
    for choice in ActionChoice::ALL {
        let hint = variant_hint(&choice);
        assert!(
            !hint.trim().is_empty(),
            "{} is offered with nothing beside it; document its table row",
            choice.label()
        );
    }
    for choice in FilterChoice::ALL {
        let hint = variant_hint(&choice);
        assert!(
            !hint.trim().is_empty(),
            "the {} filter is offered with nothing beside it",
            choice.label()
        );
    }
}

/// A twin point-defense mount: two barrels off one root, told apart by the
/// ids a patch and an inspector row both address them by.
fn twin_pdc() -> SectionConfig {
    let barrel = |id: &str, fire_rate: f32| TurretJoint {
        name: None,
        offset: Vec3::ZERO,
        axis: None,
        speed: 0.0,
        min: None,
        max: None,
        render_mesh: None,
        render_mesh_transform: None,
        muzzle: Some(MuzzleConfig {
            id: id.to_string(),
            fire_rate,
            muzzle_effect: None,
        }),
        children: Vec::new(),
    };
    SectionConfig {
        base: BaseSectionConfig {
            id: "pdc".to_string(),
            ..default()
        },
        kind: SectionKind::Turret(TurretSectionConfig {
            root: TurretJoint {
                children: vec![barrel("left", 10.0), barrel("right", 10.0)],
                ..barrel("unused", 0.0)
            },
            bullet_damage: 12.0,
            ..default()
        }),
    }
}

/// The turret's own root carries no muzzle; the helper above builds one to
/// clone the joint's shape from.
fn pdc_catalog() -> GameSections {
    let mut config = twin_pdc();
    let SectionKind::Turret(turret) = &mut config.kind else {
        panic!("a turret");
    };
    turret.root.muzzle = None;
    GameSections(vec![config])
}

fn pdc_node() -> SectionNode {
    SectionNode {
        source: SectionSource::prototype("pdc"),
        binds: vec![],
    }
}

/// The three states one row can be in, on the one section that has them: a
/// prototype instance shows the resolved value, an edited field is marked, and
/// every other field still inherits.
#[test]
fn a_patched_row_is_marked_and_its_neighbours_still_inherit() {
    let catalog = pdc_catalog();
    let mut node = pdc_node();

    let inherited = section_rows(&node, Some(&catalog));
    assert_eq!(text_of(&inherited, "Bullet Damage"), "12");
    assert!(
        inherited.iter().all(|row| !row.overridden),
        "nothing is patched yet, so nothing is marked"
    );

    let damage = row(&inherited, "Bullet Damage").clone();
    write_section(&mut node, Some(&catalog), &damage, "7").expect("a number the field takes");

    let rows = section_rows(&node, Some(&catalog));
    assert_eq!(
        text_of(&rows, "Bullet Damage"),
        "7",
        "the row shows the RESOLVED value, which is the patched one"
    );
    assert!(row(&rows, "Bullet Damage").overridden, "and is marked");
    assert!(
        !row(&rows, "Muzzle Speed").overridden,
        "a field nobody touched still inherits"
    );
}

/// Reset is the other half: it takes the patch FIELD away, so a later change
/// to the catalog part reaches the row again. Copying today's prototype value
/// into the document would look the same on screen and be the opposite.
#[test]
fn reset_removes_the_patch_field_rather_than_copying_the_value() {
    let catalog = pdc_catalog();
    let mut node = pdc_node();
    let damage = row(&section_rows(&node, Some(&catalog)), "Bullet Damage").clone();
    write_section(&mut node, Some(&catalog), &damage, "7").expect("a number the field takes");

    reset_field(&mut node, Some(&catalog), &damage.path).expect("an inherited value to go back to");

    let SectionSource::Prototype { id, patch } = &node.source else {
        panic!("still a reference: {:?}", node.source);
    };
    assert_eq!(id, "pdc");
    assert!(
        patch.is_empty(),
        "the patch field is GONE, not set to the prototype's value: {patch:?}"
    );
    let rows = section_rows(&node, Some(&catalog));
    assert_eq!(text_of(&rows, "Bullet Damage"), "12");
    assert!(!row(&rows, "Bullet Damage").overridden);
}

/// A twin's two barrels carry the same fields, and the curated view drops the
/// joint tree that used to tell them apart. The ID is what the heading says -
/// and what the patch is keyed by, so re-parenting a barrel cannot move an
/// authored rate onto the other gun.
#[test]
fn a_twin_shows_and_patches_its_barrels_by_id() {
    let catalog = pdc_catalog();
    let mut node = pdc_node();

    let rows = curated_section_rows(&node, Some(&catalog));
    let rates: Vec<Vec<String>> = rows
        .iter()
        .filter(|row| row.label == "Fire Rate")
        .map(|row| row.group.clone())
        .collect();
    assert_eq!(
        rates,
        vec![
            vec!["Left Muzzle".to_string()],
            vec!["Right Muzzle".to_string()]
        ],
        "one row per barrel, each under the id it answers to"
    );

    let left = rows
        .iter()
        .find(|row| row.label == "Fire Rate" && row.group == vec!["Left Muzzle".to_string()])
        .expect("the left barrel's rate")
        .clone();
    write_section(&mut node, Some(&catalog), &left, "50").expect("a number the field takes");

    let SectionSource::Prototype { patch, .. } = &node.source else {
        panic!("still a reference: {:?}", node.source);
    };
    assert_eq!(
        patch.kind,
        Some(SectionKindPatch::Turret(TurretSectionConfigPatch {
            muzzles: [(
                "left".to_string(),
                MuzzleConfigPatch {
                    fire_rate: Some(50.0)
                }
            )]
            .into_iter()
            .collect(),
            ..default()
        })),
        "by id, and the twin is left inheriting"
    );
    let rows = curated_section_rows(&node, Some(&catalog));
    let marked: Vec<(Vec<String>, bool)> = rows
        .iter()
        .filter(|row| row.label == "Fire Rate")
        .map(|row| (row.group.clone(), row.overridden))
        .collect();
    assert_eq!(
        marked,
        vec![
            (vec!["Left Muzzle".to_string()], true),
            (vec!["Right Muzzle".to_string()], false)
        ]
    );
}

/// The boundary, at the panel. A joint OFFSET is the prototype's geometry and
/// no patch can say it, so the placement keeps the whole config instead - and
/// the edit still lands, which is what the panel offered.
#[test]
fn an_edit_a_patch_cannot_say_keeps_the_whole_config_instead() {
    let catalog = pdc_catalog();
    let mut node = pdc_node();
    let offset = row(&section_rows(&node, Some(&catalog)), "Offset").clone();

    let mut path = offset.path.clone();
    path.push(PathStep::Field("y".to_string()));
    edit_section(&mut node, Some(&catalog), |config| {
        write_field(section_config_mut(&mut config.kind), &path, false, "2")
    })
    .expect("a number the field takes");

    assert!(
        matches!(node.source, SectionSource::Inline(_)),
        "geometry is outside the patch boundary: {:?}",
        node.source
    );
    assert_eq!(
        text_of(&section_rows(&node, Some(&catalog)), "Offset"),
        "0, 2, 0"
    );
}

/// A well target is ONE row with its own states, on both fields that hold
/// one. The AI's optional `orbit` switches between an authored id, the
/// nearest well and no orbit; the order's required `well` refuses no orbit
/// and keeps what it held. The id is written through the string inside
/// `Authored`, the path the row's id box and picker write.
#[test]
fn a_well_target_row_switches_state_and_writes_its_id() {
    let mut pilot = AIControllerConfig {
        orbit: Some(WellTargetType::Authored("planetoid".to_string())),
        ..default()
    };
    let mut rows = Vec::new();
    walk(&pilot, FieldRoot::Config, Vec::new(), &mut rows);
    let orbit = row(&rows, "Orbit");
    assert_eq!(
        orbit.value,
        RowValue::WellTarget(Some(WellTargetType::Authored("planetoid".to_string())))
    );
    assert!(orbit.optional);
    assert_eq!(orbit.names, Some(Names::Object));
    let path = orbit.path.clone();

    set_well_target(&mut pilot, &path, true, Some(WellTargetType::NearestToShip)).unwrap();
    assert_eq!(pilot.orbit, Some(WellTargetType::NearestToShip));
    set_well_target(&mut pilot, &path, true, None).unwrap();
    assert_eq!(pilot.orbit, None);
    set_well_target(
        &mut pilot,
        &path,
        true,
        Some(WellTargetType::Authored(String::new())),
    )
    .unwrap();
    let id = [path, vec![PathStep::Slot(0), PathStep::Slot(0)]].concat();
    write_field(&mut pilot, &id, false, "moon").unwrap();
    assert_eq!(
        pilot.orbit,
        Some(WellTargetType::Authored("moon".to_string()))
    );

    let mut order = OrbitShipActionConfig {
        order: "ring".to_string(),
        ship: "surveyor".to_string(),
        well: WellTargetType::NearestToShip,
    };
    let mut rows = Vec::new();
    walk(&order, FieldRoot::Config, Vec::new(), &mut rows);
    let well = row(&rows, "Well");
    assert_eq!(
        well.value,
        RowValue::WellTarget(Some(WellTargetType::NearestToShip))
    );
    assert!(!well.optional);
    let path = well.path.clone();
    assert!(set_well_target(&mut order, &path, false, None).is_err());
    assert_eq!(order.well, WellTargetType::NearestToShip);
    set_well_target(
        &mut order,
        &path,
        false,
        Some(WellTargetType::Authored("planetoid".to_string())),
    )
    .unwrap();
    assert_eq!(
        order.well,
        WellTargetType::Authored("planetoid".to_string())
    );
}

/// A hull that is not a ship node has no sections to open. The panel says why.
#[test]
fn a_hull_held_as_an_object_says_it_is_preserved_as_authored() {
    let row = authored_hull_row(&ScenarioObjectKind::Spaceship(SpaceshipConfig::default()))
        .expect("a hull leads with the row");

    assert!(matches!(&row.value, RowValue::Fixed(text) if text == "preserved as authored"));
}

/// A stock action of `choice`, as the palette drops it.
fn stock_action(choice: ActionChoice) -> ActionNode {
    ActionNode {
        kind: choice.stock(),
    }
}

/// Switch the field the row `label` of `node` edits to `option`.
fn choose_on(node: &mut ActionNode, label: &str, option: &str) -> Result<(), String> {
    let path = row(&action_rows(node), label).path.clone();
    let config = action_config_mut(&mut node.kind).expect("the action has a config");
    choose_field(config, &path, option)
}

/// A scatter region switched to a ring is a ring: its own fields start fresh
/// and the box corners are gone.
#[test]
fn a_scatter_region_switches_shape_and_starts_the_new_shape_fresh() {
    let mut node = stock_action(ActionChoice::ScatterObjects);
    let rows = action_rows(&node);
    let RowValue::Choice { options, .. } = &row(&rows, "Region").value else {
        panic!("the region is a choice: {:?}", row(&rows, "Region").value);
    };
    assert_eq!(options, &["Box", "Ring"]);

    choose_on(&mut node, "Region", "Ring").expect("a ring starts fresh");

    let rows = action_rows(&node);
    assert_eq!(row(&rows, "Region").value.reading(), "Ring");
    assert_eq!(row(&rows, "Outer").value, RowValue::Number("0".to_string()));
    assert!(rows.iter().all(|row| row.label != "Min"), "the box is gone");
}

/// A spawn switched to a beacon spawns the beacon the palette places, not a
/// beacon of radius zero.
#[test]
fn a_spawned_object_switched_to_a_beacon_takes_the_palette_beacon() {
    let mut node = stock_action(ActionChoice::SpawnScenarioObject);

    choose_on(&mut node, "Kind", "Beacon").expect("a beacon has a stock");

    let config = action_config(&node.kind).expect("a config");
    let held = peek(
        config,
        &[PathStep::Field("kind".to_string()), PathStep::Slot(0)],
    )
    .expect("the beacon");
    let ScenarioObjectKind::Beacon(stock) = ObjectChoice::Beacon.stock().kind else {
        panic!("the palette beacon is a beacon");
    };
    assert_eq!(held.reflect_partial_eq(&stock), Some(true));
}

/// An optional struct is `none` or `set`. Set, its fields are rows under it;
/// back to `none`, they are gone.
#[test]
fn an_optional_blend_is_set_fresh_and_cleared_to_none() {
    let mut node = stock_action(ActionChoice::SetCamera);
    assert_eq!(row(&action_rows(&node), "Blend").value.reading(), NOTHING);

    choose_on(&mut node, "Blend", "set").expect("a blend starts fresh");
    let rows = action_rows(&node);
    assert_eq!(row(&rows, "Blend").value.reading(), "set");
    assert!(rows.iter().any(|row| row.label == "Seconds"));

    choose_on(&mut node, "Blend", NOTHING).expect("a blend clears");
    let rows = action_rows(&node);
    assert_eq!(row(&rows, "Blend").value.reading(), NOTHING);
    assert!(rows.iter().all(|row| row.label != "Seconds"));
}

/// An optional set of names is ONE choice: nothing, or each name.
#[test]
fn an_optional_order_kind_is_one_choice_of_none_and_each_name() {
    let mut node = FilterNode {
        kind: FilterChoice::ShipOrder.stock(),
    };
    let path = row(&filter_rows(&node), "Kind").path.clone();
    let config = filter_config_mut(&mut node.kind).expect("a config");

    choose_field(config, &path, "Dock").expect("a name");
    let rows = filter_rows(&node);
    assert_eq!(row(&rows, "Kind").value.reading(), "Dock");

    let config = filter_config_mut(&mut node.kind).expect("a config");
    choose_field(config, &path, NOTHING).expect("nothing");
    assert_eq!(row(&filter_rows(&node), "Kind").value.reading(), NOTHING);

    let config = filter_config_mut(&mut node.kind).expect("a config");
    assert!(choose_field(config, &path, "Warp").is_err());
}

/// A camera switched to look at an object offers the objects, because the
/// variant declares what its id names.
#[test]
fn a_camera_looking_at_an_object_offers_the_objects() {
    let mut node = stock_action(ActionChoice::SetCameraAnchor);

    choose_on(&mut node, "Look At", "Object").expect("an object id starts empty");

    let rows = action_rows(&node);
    let id = rows
        .iter()
        .find(|row| row.label == "Look At" && matches!(row.value, RowValue::Text(_)))
        .expect("the object id row");
    assert_eq!(id.names, Some(Names::Object));
}

/// A spawn's id is one it MINTS: the struct holding it names nothing, and
/// must not wipe what the id field declares.
#[test]
fn a_spawn_id_keeps_the_names_its_own_field_declares() {
    let rows = action_rows(&stock_action(ActionChoice::SpawnScenarioObject));
    let id = rows
        .iter()
        .find(|row| row.label == "Id")
        .expect("the base id row");

    assert_eq!(id.names, Some(Names::NewObject));
}

/// A scatter's mix entry reads as its two parts, each in its own control.
#[test]
fn a_scatter_mix_entry_reads_as_kind_and_weight() {
    let rows = action_rows(&stock_action(ActionChoice::ScatterObjects));

    let kind = rows
        .iter()
        .find(|row| row.label == "Kind" && row.group == ["Asteroid Kinds 1"])
        .expect("the mix entry's kind row");
    assert!(matches!(kind.value, RowValue::Choice { .. }));
    assert_eq!(
        row(&rows, "Weight").value,
        RowValue::Number("1".to_string())
    );
}

/// A rotation inside a config reads and writes as the pose's three degrees,
/// one box at a time. A box that is not a number is refused and the rotation
/// is kept.
#[test]
fn a_spawn_rotation_is_three_degree_boxes_written_one_at_a_time() {
    let mut node = stock_action(ActionChoice::SpawnScenarioObject);
    let rotation = row(&action_rows(&node), "Rotation").clone();
    assert_eq!(
        rotation.value,
        RowValue::Axes(["0", "0", "0"].map(str::to_string))
    );
    assert_eq!(rotation.unit, ROTATION_UNIT);
    let yaw = [rotation.path.clone(), vec![axis_step(0)]].concat();
    let pitch = [rotation.path.clone(), vec![axis_step(1)]].concat();

    let config = action_config_mut(&mut node.kind).expect("a config");
    write_field(config, &yaw, false, "90").expect("a yaw");
    let rule = DragRule {
        step: 1.0,
        limit: Limit::Free,
    };
    nudge_field(config, &pitch, false, rule, 5.0).expect("a pitch scrub");
    assert!(write_field(config, &yaw, false, "east").is_err());

    assert_eq!(
        row(&action_rows(&node), "Rotation").value,
        RowValue::Axes(["90", "5", "0"].map(str::to_string))
    );
}

/// The form of `kind` from the stand-in content, as the bench holds it.
fn form_of(kind: &str) -> nova_modding::prelude::Content {
    crate::project::tests::one_of_every_other_kind()
        .into_iter()
        .find(|item| item.kind() == kind)
        .unwrap_or_else(|| panic!("no {kind} in the stand-in content"))
}

/// The rows of `item`'s form.
fn form_rows_of(item: &nova_modding::prelude::Content) -> Vec<InspectorRow> {
    asset_rows(item, &Default::default())
}

/// The add row of the list labelled `label`.
fn append_row<'a>(rows: &'a [InspectorRow], label: &str) -> &'a InspectorRow {
    rows.iter()
        .find(|row| row.label == label && matches!(row.value, RowValue::Append(_)))
        .unwrap_or_else(|| panic!("no add row for {label}"))
}

/// The number of entries the list labelled `label` shows.
fn entries(rows: &[InspectorRow], label: &str) -> usize {
    rows.iter()
        .filter(|row| matches!(row.value, RowValue::Remove) && row.label.starts_with(label))
        .count()
}

/// A price rule added to the table is a rule no other rule keys alike, and a
/// removed one is gone. Once every cluster holds a rule for every category,
/// the add is refused and says why.
#[test]
fn a_price_rule_is_added_distinct_removed_and_refused_when_none_is_left() {
    let mut item = form_of("price table");
    let rows = form_rows_of(&item);
    let before = entries(&rows, "Clusters");
    let add = append_row(&rows, "Clusters").path.clone();
    let config = content_reflect_mut(&mut item).expect("a form");

    append_item(config, &add, ADD).expect("a free cluster and category");
    let rows = form_rows_of(&item);
    assert_eq!(entries(&rows, "Clusters"), before + 1);

    let last = rows
        .iter()
        .rfind(|row| matches!(row.value, RowValue::Remove) && row.label.starts_with("Clusters"))
        .expect("the new rule")
        .path
        .clone();
    let config = content_reflect_mut(&mut item).expect("a form");
    remove_item(config, &last).expect("the rule drops");
    assert_eq!(entries(&form_rows_of(&item), "Clusters"), before);

    let config = content_reflect_mut(&mut item).expect("a form");
    let refused = (0..1_000)
        .find_map(|_| append_item(config, &add, ADD).err())
        .expect("the keys run out");
    assert_eq!(refused, "every cluster already has an entry");
    assert!(append_item(config, &add, "Gold").is_err());
}

/// A fixture added to a style mints its own id and takes a model through its
/// own row. An empty model is refused.
#[test]
fn a_style_fixture_is_added_with_a_minted_id_and_a_model_row() {
    let mut item = form_of("style");
    let rows = form_rows_of(&item);
    let before = entries(&rows, "Fixtures");
    let add = append_row(&rows, "Fixtures").path.clone();
    let config = content_reflect_mut(&mut item).expect("a form");

    append_item(config, &add, ADD).expect("a fixture");

    let rows = form_rows_of(&item);
    assert_eq!(entries(&rows, "Fixtures"), before + 1);
    let group = [format!("Fixtures {}", before + 1)];
    let id = rows
        .iter()
        .find(|row| row.label == "Id" && row.group == group)
        .expect("the new fixture's id");
    assert_eq!(id.value, RowValue::Text("fixture_1".to_string()));
    let model = rows
        .iter()
        .find(|row| row.label == "Model" && row.group == group)
        .expect("the new fixture's model row");
    let FieldRoot::AssetText(kind) = model.root else {
        panic!("the model is written by its own rule");
    };
    assert!(set_item_text(&mut item, kind, " ").is_err());
    set_item_text(&mut item, kind, "self://gltf/crate.glb").expect("a model");
    assert_eq!(item_text(&item, kind), "self://gltf/crate.glb");
}

/// A section's damage effects are one toggle per effect: a press adds the
/// effect it names or drops it, and a name that is not an effect is refused.
#[test]
fn damage_effects_are_one_toggle_per_effect() {
    let mut item = form_of("section");
    let rows = form_rows_of(&item);
    let effects = row(&rows, "Damage Effects").clone();
    let RowValue::Set { options, on, .. } = &effects.value else {
        panic!("the effects are a set: {:?}", effects.value);
    };
    assert_eq!(options, &["Cracks", "Sparks", "Plume"]);
    let held = on.clone();
    let config = content_reflect_mut(&mut item).expect("a form");

    choose_field(config, &effects.path, "Plume").expect("a toggle");
    let flipped = |rows: &[InspectorRow]| match &row(rows, "Damage Effects").value {
        RowValue::Set { on, .. } => on.clone(),
        other => panic!("still a set: {other:?}"),
    };
    let after = flipped(&form_rows_of(&item));
    assert_eq!(after[2], !held[2]);
    assert_eq!(after[..2], held[..2]);

    let config = content_reflect_mut(&mut item).expect("a form");
    choose_field(config, &effects.path, "Plume").expect("a toggle back");
    assert_eq!(flipped(&form_rows_of(&item)), held);
    let config = content_reflect_mut(&mut item).expect("a form");
    assert!(choose_field(config, &effects.path, "Smoke").is_err());
}

/// A scatter's mix takes a new kind no entry holds yet, and drops one.
#[test]
fn a_scatter_mix_adds_an_unused_kind_and_drops_one() {
    let mut node = stock_action(ActionChoice::ScatterObjects);
    let add = append_row(&action_rows(&node), "Asteroid Kinds")
        .path
        .clone();
    let config = action_config_mut(&mut node.kind).expect("a config");

    append_item(config, &add, ADD).expect("a second kind");

    let rows = action_rows(&node);
    let kinds: Vec<String> = rows
        .iter()
        .filter(|row| row.label == "Kind" && row.group[0].starts_with("Asteroid Kinds"))
        .map(|row| row.value.reading())
        .collect();
    assert_eq!(kinds, ["rock", "metal"]);
    let first = [add.clone(), vec![PathStep::Item(0)]].concat();
    let config = action_config_mut(&mut node.kind).expect("a config");
    remove_item(config, &first).expect("the rock drops");
    assert!(remove_item(config, &[add, vec![PathStep::Item(5)]].concat()).is_err());
}

/// Every add and remove control of every form and every script block lands:
/// a new entry the list cannot hold would panic in reflection, so each one is
/// pressed once here.
#[test]
fn every_list_control_of_every_form_and_block_lands() {
    for mut item in crate::project::tests::one_of_every_other_kind() {
        for row in form_rows_of(&item) {
            let Some(config) = content_reflect_mut(&mut item) else {
                continue;
            };
            match &row.value {
                RowValue::Append(options) => {
                    for option in options {
                        let _ = append_item(config, &row.path, option);
                    }
                }
                RowValue::Remove => {
                    remove_item(config, &row.path).ok();
                }
                _ => {}
            }
        }
    }
    for choice in ActionChoice::ALL {
        let mut node = stock_action(choice);
        for row in action_rows(&node) {
            let config = action_config_mut(&mut node.kind).expect("a config");
            if let RowValue::Append(options) = &row.value {
                for option in options {
                    append_item(config, &row.path, option)
                        .unwrap_or_else(|reason| panic!("{}: {reason}", choice.label()));
                }
            }
        }
    }
}

/// The node of the document whose id is `id`.
fn node_named(world: &mut World, id: &str) -> Entity {
    let mut nodes = world.query::<(Entity, &NodeId)>();
    nodes
        .iter(world)
        .find(|(_, held)| held.0 == id)
        .map(|(entity, _)| entity)
        .expect("the document holds the node")
}

/// A refill names one section of the ship the same action names, so its
/// picker offers that ship's sections, nothing before a ship is named, and
/// marks a section the ship does not have.
#[test]
fn a_refill_section_offers_the_sections_of_the_ship_it_names() {
    let mut world = named_document();
    let raider = node_named(&mut world, "raider_1");
    for id in ["gun_b", "gun_a"] {
        world.spawn((
            EditorNode,
            thruster_node(1.0),
            NodeId(id.to_string()),
            Transform::default(),
            ChildOf(raider),
        ));
    }
    let names = document_names(&mut world);
    let mut refill = stock_action(ActionChoice::RefillAmmo);
    let section = row(&action_rows(&refill), "Section").clone();
    assert_eq!(section.names, Some(Names::Section));
    let config = action_config(&refill.kind).expect("a config");
    assert_eq!(
        section_scope(config, &section.path),
        None,
        "no ship named yet"
    );
    assert!(names.offers_in(Names::Section, None).is_empty());

    let id = row(&action_rows(&refill), "Id").path.clone();
    let config = action_config_mut(&mut refill.kind).expect("a config");
    write_field(config, &id, false, "raider_1").expect("the ship is named");
    let ship = section_scope(
        action_config(&refill.kind).expect("a config"),
        &section.path,
    );

    assert_eq!(ship.as_deref(), Some("raider_1"));
    assert_eq!(
        names.offers_in(Names::Section, ship.as_deref()),
        ["gun_a", "gun_b"]
    );
    assert!(names.resolves_in(Names::Section, "gun_a", ship.as_deref()));
    assert!(
        !names.resolves_in(Names::Section, "engine", ship.as_deref()),
        "a section the ship lacks is marked"
    );
}

/// A readout slot is minted by the first readout that names it, so the picker
/// offers the slots already in use and an empty slot is refused.
#[test]
fn a_hud_slot_offers_the_slots_readouts_already_name() {
    let mut world = named_document();
    let handler = node_named(&mut world, "event_1");
    let mut readout = stock_action(ActionChoice::HudReadout);
    let slot = row(&action_rows(&readout), "Slot").clone();
    assert_eq!(slot.names, Some(Names::HudSlot));
    let config = action_config_mut(&mut readout.kind).expect("a config");
    write_field(config, &slot.path, false, "timer").expect("a slot is minted");
    assert_eq!(
        write_field(config, &slot.path, false, " "),
        Err("a readout needs a slot to update or clear it".to_string())
    );
    world.spawn((
        EditorNode,
        readout,
        NodeId("readout_1".to_string()),
        ChildOf(handler),
    ));

    assert_eq!(document_names(&mut world).offers(Names::HudSlot), ["timer"]);
}

/// A spawned object's id is minted text held to the content lint's rule: not
/// empty, no '/', and not a spelling only the world generator mints.
#[test]
fn a_minted_object_id_refuses_nothing_a_slash_and_a_body_id() {
    let mut spawn = stock_action(ActionChoice::SpawnScenarioObject);
    let id = row(&action_rows(&spawn), "Id").path.clone();
    let config = action_config_mut(&mut spawn.kind).expect("a config");
    let body = nova_events::prelude::GeneratedBodyId::new(
        nova_events::prelude::SectorCoord::ORIGIN,
        "rock",
        3,
    )
    .to_string();

    for (text, refused) in [
        ("", "an object needs an id"),
        ("rock/7", "'/' is reserved for minted ids"),
        (
            body.as_str(),
            "that spells a generated body id, which only the world generator mints",
        ),
    ] {
        assert_eq!(
            write_field(config, &id, false, text),
            Err(refused.to_string()),
            "{text:?}"
        );
    }
    write_field(config, &id, false, "rock_7").expect("a plain id is minted");
}

/// A scatter's prefix and a sequence's key are minted text too: an empty one
/// is refused, because nothing could name what it mints.
#[test]
fn an_empty_scatter_prefix_or_sequence_key_is_refused() {
    let mut scatter = stock_action(ActionChoice::ScatterObjects);
    let prefix = row(&action_rows(&scatter), "Id Prefix").path.clone();
    let config = action_config_mut(&mut scatter.kind).expect("a config");
    assert_eq!(
        write_field(config, &prefix, false, ""),
        Err("a scatter needs an id prefix for its copies".to_string())
    );
    write_field(config, &prefix, false, "rock_").expect("a prefix is minted");

    let mut sequence = stock_action(ActionChoice::Sequence);
    let key = row(&action_rows(&sequence), "Key").path.clone();
    let config = action_config_mut(&mut sequence.kind).expect("a config");
    assert_eq!(
        write_field(config, &key, false, ""),
        Err("a sequence needs a key to file its cursor under".to_string())
    );
    write_field(config, &key, false, "intro").expect("a key is minted");
}
