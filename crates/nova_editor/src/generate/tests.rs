//! What Generate has to do to the document, run as the live verb: the observer
//! the button carries, over the SHIPPED catalog and the SHIPPED grammar.
//!
//! The catalog comes out of the builders rather than off disk, so these fail on
//! a content change rather than on a missing asset server.

use bevy::ui_widgets::observe;
use nova_input::prelude::InputSource;
use nova_ship::prelude::LinkPoint;
use nova_wfc::prelude::GRID_EPSILON;

use super::*;
use crate::node::{EditorNode, ScenarioNode};

/// The resources the verb reads, a scenario, and one empty ship INSIDE which
/// the editor is standing - which is where Generate lives.
fn generate_app() -> (App, Entity) {
    let mut app = App::new();
    app.init_resource::<Time>();
    app.init_resource::<crate::config::EditorStatus>();
    app.init_resource::<SelectedNode>();
    app.init_resource::<HullSeed>();
    app.insert_resource(GameSections(
        nova_authoring::generation::build_section_catalog(),
    ));
    app.insert_resource(GameGrammars(nova_authoring::generation::build_grammars()));
    app.insert_resource(GameStyles(nova_authoring::generation::build_styles()));
    let scenario = app
        .world_mut()
        .spawn((
            EditorNode,
            ScenarioNode::default(),
            NodeId("scenario".to_string()),
            NextChildOrdinal::default(),
        ))
        .id();
    let ship = app
        .world_mut()
        .spawn((
            EditorNode,
            ShipNode::default(),
            NodeId("ship_1".to_string()),
            NextChildOrdinal::default(),
            ChildOf(scenario),
        ))
        .id();
    app.world_mut().insert_resource(EditContext {
        path: vec![scenario, ship],
    });
    tick_the_grammars_own_draw(&mut app);
    (app, ship)
}

/// The rows the rail spawns ticked: the shipped grammar's own draw. The rows
/// ARE the setting, so a test that wants the base game's roll spawns them.
fn tick_the_grammars_own_draw(app: &mut App) {
    let drawn: Vec<String> = app
        .world()
        .resource::<GameGrammars>()
        .get_grammar(STANDARD_HULL_GRAMMAR_ID)
        .map(|grammar| {
            grammar
                .parts
                .iter()
                .map(|part| part.prototype.clone())
                .collect()
        })
        .unwrap_or_default();
    for prototype in drawn {
        app.world_mut().spawn((ticked(&prototype), Selected));
    }
}

/// One ticked row of the draw list, unzoned.
fn ticked(prototype: &str) -> PartChoice {
    PartChoice {
        prototype: prototype.to_string(),
        zone: None,
    }
}

/// One entry of the list `drawn_grammar` reads, unzoned.
fn drawn(prototype: &str) -> Drawn {
    Drawn {
        prototype: prototype.to_string(),
        zone: None,
    }
}

/// The button the builder presses, with the verb on it.
fn generate_button(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((GenerateButton, observe(generate_ship)))
        .id()
}

/// Every section node standing on `ship`.
fn hull_of(app: &mut App, ship: Entity) -> Vec<Entity> {
    app.world()
        .get::<Children>(ship)
        .map(|children| {
            children
                .iter()
                .filter(|child| app.world().get::<SectionNode>(*child).is_some())
                .collect()
        })
        .unwrap_or_default()
}

/// The line the status is holding.
fn status_line(app: &App) -> String {
    app.world()
        .resource::<crate::config::EditorStatus>()
        .line()
        .map(|(text, _)| text.to_string())
        .unwrap_or_default()
}

/// One press fills the ship the editor is inside. No second ship appears: the
/// verb is what THIS hull is, not a way to add one.
#[test]
fn generate_lays_a_hull_into_the_ship_being_edited() {
    let (mut app, ship) = generate_app();
    let button = generate_button(&mut app);
    app.world_mut().trigger(Activate { entity: button });
    app.update();

    assert!(
        !hull_of(&mut app, ship).is_empty(),
        "the collapse lifted no sections onto the ship at all"
    );
    let ships = app
        .world_mut()
        .query_filtered::<Entity, With<ShipNode>>()
        .iter(app.world())
        .count();
    assert_eq!(ships, 1, "generating a hull is not adding a ship");
}

/// The builder stays inside the ship with it marked, because the block that
/// rolled it is in there and a reroll is the next thing they want.
#[test]
fn generate_leaves_the_builder_where_they_can_roll_another() {
    let (mut app, ship) = generate_app();
    let button = generate_button(&mut app);
    app.world_mut().trigger(Activate { entity: button });
    app.update();

    assert_eq!(
        app.world().resource::<EditContext>().current(),
        Some(ship),
        "generating a hull walked out of the ship it was rolled into"
    );
    assert_eq!(
        app.world().resource::<SelectedNode>().0,
        Some(ship),
        "the tree marks the ship that just changed"
    );
}

/// A seed is a dial. Rolling it again REPLACES the hull, because a Generate
/// that added would stack every draft inside one ship on the way to the keeper.
#[test]
fn a_second_generate_replaces_the_hull_rather_than_stacking_one() {
    let (mut app, ship) = generate_app();
    let button = generate_button(&mut app);
    app.insert_resource(HullSeed(7));
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    let first = hull_of(&mut app, ship).len();

    app.insert_resource(HullSeed(8));
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    let second = hull_of(&mut app, ship).len();

    assert!(first > 0 && second > 0, "both rolls built something");
    assert!(
        second < first * 2,
        "the second roll stacked on the first: {first} sections became {second}"
    );
    let ids: Vec<String> = hull_of(&mut app, ship)
        .iter()
        .filter_map(|node| app.world().get::<NodeId>(*node))
        .map(|id| id.0.clone())
        .collect();
    let unique: std::collections::HashSet<&String> = ids.iter().collect();
    assert_eq!(
        unique.len(),
        ids.len(),
        "two hulls in one ship means two sections on one id"
    );
}

/// A generated ship is one the builder can set to Player and fly. An unbound
/// battery is a ship with no trigger.
#[test]
fn every_weapon_the_collapse_lays_comes_out_bound() {
    let (mut app, ship) = generate_app();
    let button = generate_button(&mut app);
    app.world_mut().trigger(Activate { entity: button });
    app.update();

    let catalog = app.world().resource::<GameSections>().clone();
    let mut bindable = 0;
    for node in hull_of(&mut app, ship) {
        let section = app
            .world()
            .get::<SectionNode>(node)
            .expect("hull_of only returns sections");
        if !section.bindable(Some(&catalog)) {
            assert!(
                section.binds.is_empty(),
                "hull and controller sections are not things a pilot presses"
            );
            continue;
        }
        bindable += 1;
        assert!(
            section
                .binds
                .iter()
                .any(|source| matches!(source, InputSource::Keyboard(_) | InputSource::Mouse(_))),
            "'{}' came out with no desk binding at all",
            section.prototype()
        );
    }
    assert!(
        bindable > 0,
        "the shipped grammar draws thrusters and guns, so a hull with none is a broken roll"
    );
}

/// The same seed builds the same ship, twice, in the editor's own path - the
/// property that makes the number worth showing.
#[test]
fn one_seed_lifts_one_hull() {
    let ids = |app: &mut App| -> Vec<String> {
        let mut ids: Vec<String> = app
            .world_mut()
            .query_filtered::<&NodeId, With<SectionNode>>()
            .iter(app.world())
            .map(|id| id.0.clone())
            .collect();
        ids.sort();
        ids
    };

    let (mut first, _) = generate_app();
    first.insert_resource(HullSeed(7));
    let button = generate_button(&mut first);
    first.world_mut().trigger(Activate { entity: button });
    first.update();

    let (mut second, _) = generate_app();
    second.insert_resource(HullSeed(7));
    let button = generate_button(&mut second);
    second.world_mut().trigger(Activate { entity: button });
    second.update();

    assert_eq!(ids(&mut first), ids(&mut second), "seed 7 names one hull");
    assert!(!ids(&mut first).is_empty(), "seed 7 built something");
}

/// A seed that is not a number greys Generate and says why under the field.
#[test]
fn a_seed_that_is_not_a_number_greys_generate() {
    let (mut app, _) = generate_app();
    let button = generate_button(&mut app);
    let field = app
        .world_mut()
        .spawn((HullSeedField, TextFieldValue("twelve".to_string())))
        .id();
    app.add_systems(Update, read_seed_field);
    app.update();

    assert!(
        app.world().get::<TextFieldError>(field).is_some(),
        "the field says what it wanted"
    );
    assert!(
        app.world().get::<InteractionDisabled>(button).is_some(),
        "a button that cannot work does not look like it can"
    );

    app.world_mut().get_mut::<TextFieldValue>(field).unwrap().0 = "12".to_string();
    app.update();
    assert!(
        app.world().get::<TextFieldError>(field).is_none(),
        "a seed it can read clears the refusal"
    );
    assert!(
        app.world().get::<InteractionDisabled>(button).is_none(),
        "and gives the button back"
    );
    assert_eq!(app.world().resource::<HullSeed>().0, 12);
}

/// Reroll writes the new seed where the builder can still edit it.
#[test]
fn reroll_puts_the_new_seed_in_the_field() {
    let (mut app, _) = generate_app();
    app.insert_resource(HullSeed(1));
    let field = app
        .world_mut()
        .spawn((HullSeedField, TextFieldValue("1".to_string())))
        .id();
    let button = app.world_mut().spawn(observe(reroll_seed)).id();
    app.world_mut().trigger(Activate { entity: button });
    app.update();

    let seed = app.world().resource::<HullSeed>().0;
    assert_eq!(
        app.world().get::<TextFieldValue>(field).unwrap().0,
        seed.to_string(),
        "the field shows the seed the next Generate will use"
    );
}

/// Every way the roll can come back a refusal instead of a hull. Each leaves
/// the ship exactly as it was and puts one line where the builder is looking -
/// which is the whole contract: a generator that panics takes the game with it.
#[test]
fn a_roll_that_cannot_be_made_says_so_and_leaves_the_ship_alone() {
    // A grammar the merged content does not hold.
    let (mut app, ship) = generate_app();
    app.insert_resource(GameGrammars(vec![]));
    let button = generate_button(&mut app);
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(hull_of(&mut app, ship).is_empty(), "nothing was laid");
    assert!(
        status_line(&app).contains(STANDARD_HULL_GRAMMAR_ID),
        "the refusal names the grammar it could not find: {:?}",
        status_line(&app)
    );

    // Nothing ticked at all.
    let (mut app, ship) = generate_app();
    for row in app
        .world_mut()
        .query_filtered::<Entity, With<PartChoice>>()
        .iter(app.world())
        .collect::<Vec<_>>()
    {
        app.world_mut().entity_mut(row).remove::<Selected>();
    }
    let button = generate_button(&mut app);
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(hull_of(&mut app, ship).is_empty(), "nothing was laid");
    assert!(
        status_line(&app).contains("tick at least one"),
        "an empty draw says what to do about it: {:?}",
        status_line(&app)
    );

    // A section that cannot survive the centreline mirror. Only the starboard
    // half is ever solved, so a part whose sockets do not come in mirrored
    // pairs would put its port copy's sockets in cells nothing checked.
    let (mut app, ship) = generate_app();
    let mut catalog = app.world().resource::<GameSections>().clone();
    let lopsided = "lopsided_section";
    let mut config = catalog
        .get_section("reinforced_hull_section")
        .expect("the hull cube ships")
        .clone();
    config.base.id = lopsided.to_string();
    config
        .base
        .link_points
        .retain(|point| point.normal.x >= 0.0);
    catalog.0.push(config);
    app.insert_resource(catalog);
    for row in app
        .world_mut()
        .query_filtered::<Entity, With<PartChoice>>()
        .iter(app.world())
        .collect::<Vec<_>>()
    {
        app.world_mut().entity_mut(row).remove::<Selected>();
    }
    app.world_mut().spawn((ticked(lopsided), Selected));
    let button = generate_button(&mut app);
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(hull_of(&mut app, ship).is_empty(), "nothing was laid");
    assert!(
        status_line(&app).contains(lopsided),
        "the refusal names the part that cannot be mirrored: {:?}",
        status_line(&app)
    );
}

/// Generate is a SHIP verb. Pressed at the scenario there is nothing to lay a
/// hull into, and the line says where to stand.
#[test]
fn generate_outside_a_ship_says_where_to_stand() {
    let (mut app, ship) = generate_app();
    let scenario = app
        .world()
        .get::<ChildOf>(ship)
        .map(ChildOf::parent)
        .expect("the ship hangs under the scenario");
    app.world_mut().insert_resource(EditContext {
        path: vec![scenario],
    });
    let button = generate_button(&mut app);
    app.world_mut().trigger(Activate { entity: button });
    app.update();

    assert!(hull_of(&mut app, ship).is_empty(), "nothing was laid");
    assert!(
        status_line(&app).contains("inside a ship"),
        "the refusal says where the verb lives: {:?}",
        status_line(&app)
    );
}

/// A part the shipped grammar does not price still joins the draw, on the
/// terms the block's own note puts on screen. The railgun is the one a builder
/// reaches for first, and it is not in the base draw.
#[test]
fn a_section_the_grammar_does_not_price_joins_the_draw_at_the_stated_weight() {
    let grammars = GameGrammars(nova_authoring::generation::build_grammars());
    let sections = GameSections(nova_authoring::generation::build_section_catalog());
    let grammar = drawn_grammar(
        &sections,
        &grammars,
        &[
            drawn("reinforced_hull_section"),
            drawn("railgun_lance_section"),
        ],
    )
    .expect("the shipped grammar is there to subset");

    let priced = grammar
        .parts
        .iter()
        .find(|part| part.prototype == "reinforced_hull_section")
        .expect("the hull cube is ticked");
    assert!(
        priced.weight > UNAUTHORED_WEIGHT,
        "a part the grammar prices keeps the weight it was tuned at"
    );

    let joined = grammar
        .parts
        .iter()
        .find(|part| part.prototype == "railgun_lance_section")
        .expect("the lance is ticked");
    assert_eq!(joined.weight, UNAUTHORED_WEIGHT);
    assert!(
        joined.aim.is_none(),
        "nothing authored which way it points, so nothing here claims one"
    );
    // The ticks decide what is ROLLED, and nothing else does: a role the
    // builder did not tick is seeded, never drawn, so it stays off the draw
    // list entirely. `nova_wfc` gives every role the grammar NAMES its tiles.
    for seeded in ["basic_controller_section", "basic_thruster_section"] {
        assert!(
            !grammar.parts.iter().any(|part| part.prototype == seeded),
            "'{seeded}' was not ticked, so the roll must never draw one"
        );
    }
    TileSet::build(&sections, &grammar)
        .expect("a grammar whose seeded roles are off the draw list still builds");
    assert_eq!(
        grammar.grid.half_width,
        grammars
            .get_grammar(STANDARD_HULL_GRAMMAR_ID)
            .expect("shipped")
            .grid
            .half_width,
        "no big drive was ticked, so the grid stays the size a standard hull is"
    );
}

/// Ticking a drive bigger than one cell says what KIND of ship this is: it
/// becomes the seeded main engine, and the grid grows to hold it.
#[test]
fn ticking_a_capital_drive_builds_the_ship_around_it() {
    let grammars = GameGrammars(nova_authoring::generation::build_grammars());
    let sections = GameSections(nova_authoring::generation::build_section_catalog());
    let shipped = grammars
        .get_grammar(STANDARD_HULL_GRAMMAR_ID)
        .expect("shipped")
        .grid;
    let grammar = drawn_grammar(
        &sections,
        &grammars,
        &[
            drawn("reinforced_hull_section"),
            drawn("basic_thruster_section"),
            drawn("capital_thruster_section"),
        ],
    )
    .expect("the shipped grammar is there to subset");

    assert_eq!(
        grammar.keel.stern_drive, "capital_thruster_section",
        "the biggest drive ticked is the one the ship is built around"
    );
    // 5 cells across, standing one column off the seam.
    assert!(
        grammar.grid.half_width >= 6 && grammar.grid.half_width > shipped.half_width,
        "the grid grew to hold the drive: {:?}",
        grammar.grid
    );
    assert!(
        grammar.grid.length >= shipped.length,
        "a grid big enough already is never shrunk: {:?}",
        grammar.grid
    );

    let hull = TileSet::build(&sections, &grammar)
        .expect("a grown grid holds its own drive")
        .hull(7, false, None)
        .expect("and collapses");
    assert_eq!(
        hull.sections
            .iter()
            .filter(|section| matches!(
                &section.source,
                nova_scenario::prelude::SectionSource::Prototype(id)
                    if id == "capital_thruster_section"
            ))
            .count(),
        2,
        "the seeded drive is mirrored, so the hull carries a PAIR"
    );
}

/// Ticking a spinal gun seats it on the BOW, the way ticking a big drive seats
/// one on the stern. The builder learns no second control: the roles are
/// derived from the same ticks.
#[test]
fn ticking_a_spinal_gun_seats_it_on_the_bow() {
    let grammars = GameGrammars(nova_authoring::generation::build_grammars());
    let sections = GameSections(nova_authoring::generation::build_section_catalog());
    let grammar = drawn_grammar(
        &sections,
        &grammars,
        &[
            drawn("reinforced_hull_section"),
            drawn("basic_thruster_section"),
            drawn("railgun_lance_section"),
        ],
    )
    .expect("the shipped grammar is there to subset");

    assert_eq!(
        grammar.keel.bow_gun.as_deref(),
        Some("railgun_lance_section"),
        "the biggest spinal gun ticked is the one the nose is built around"
    );
    let tiles = TileSet::build(&sections, &grammar)
        .expect("the grid holds a lance and a drive with keel between");
    let hull = tiles.hull(7, false, None).expect("and collapses");
    // The lance is ticked, so the roll may ALSO hang one wherever its bore
    // happens to be clear - a legal placement, and the chip is how a builder
    // says they only want it forward. What the SEED owes is the pair on the
    // nose, and the lance is 1x1x3 firing -z, so its body reaches the bow face.
    let nose = hull
        .sections
        .iter()
        .filter(|section| {
            matches!(
                &section.source,
                nova_scenario::prelude::SectionSource::Prototype(id)
                    if id == "railgun_lance_section"
            ) && (section.position.z - 1.5 - tiles.bow_face()).abs() < GRID_EPSILON
        })
        .count();
    assert_eq!(
        nose, 2,
        "the gun stands ON the keel line, so the mirror owes the nose a pair"
    );

    // Untick it and the nose goes back to being a nose.
    let plain = drawn_grammar(
        &sections,
        &grammars,
        &[
            drawn("reinforced_hull_section"),
            drawn("basic_thruster_section"),
        ],
    )
    .expect("the shipped grammar is there to subset");
    assert!(
        plain.keel.bow_gun.is_none(),
        "no spinal gun ticked, no spinal gun seeded"
    );
}

/// The zone on a row reaches the grammar, and clearing it clears the grammar's
/// own - the row IS the setting, so it beats what the shipped table authored.
#[test]
fn a_zoned_row_carries_its_zone_into_the_grammar() {
    let grammars = GameGrammars(nova_authoring::generation::build_grammars());
    let sections = GameSections(nova_authoring::generation::build_section_catalog());
    let grammar = drawn_grammar(
        &sections,
        &grammars,
        &[
            drawn("reinforced_hull_section"),
            drawn("basic_thruster_section"),
            Drawn {
                prototype: "pdc_kinetic_turret_section".to_string(),
                zone: Some(GrammarZone::Dorsal),
            },
        ],
    )
    .expect("the shipped grammar is there to subset");
    let turret = grammar
        .parts
        .iter()
        .find(|part| part.prototype == "pdc_kinetic_turret_section")
        .expect("the turret is ticked");
    assert_eq!(turret.zone, Some(GrammarZone::Dorsal));
    let hull = TileSet::build(&sections, &grammar)
        .expect("a zone cannot make a grammar unbuildable")
        .hull(3, false, None)
        .expect("and collapses");
    for section in &hull.sections {
        let nova_scenario::prelude::SectionSource::Prototype(id) = &section.source else {
            continue;
        };
        assert!(
            !id.starts_with("pdc_") || section.position.y > 0.0,
            "'{id}' is zoned dorsal and stands at y {}",
            section.position.y
        );
    }
}

/// A reroll is a new HULL for this ship, not a new ship. The look is the
/// builder's: they turned the skin off in Ship Settings, or picked a style
/// other than the first, and pressing Generate again must not quietly hand
/// both back at the factory setting.
#[test]
fn a_reroll_leaves_the_ships_own_look_alone() {
    let (mut app, ship) = generate_app();
    let second = app
        .world()
        .resource::<GameStyles>()
        .get(1)
        .map(|style| style.id.clone())
        .expect("the base content ships more than one style");
    {
        let mut node = app.world_mut().get_mut::<ShipNode>(ship).expect("a ship");
        node.skin = false;
        node.style = Some(second.clone());
    }

    let button = generate_button(&mut app);
    app.world_mut().trigger(Activate { entity: button });
    app.update();

    assert!(
        !hull_of(&mut app, ship).is_empty(),
        "the collapse has to have run for the assertions below to mean anything"
    );
    let node = app.world().get::<ShipNode>(ship).expect("a ship");
    assert!(!node.skin, "the reroll turned the builder's skin back on");
    assert_eq!(
        node.style.as_deref(),
        Some(second.as_str()),
        "the reroll put the ship back in the first style"
    );
}

/// The other half of the same rule, read where it is decided: the hull the
/// collapse returns wears what the SHIP wears. `None` is not "no style" - the
/// node documents it as the first style the content merge loaded, which is
/// what the build view already shows.
#[test]
fn the_collapse_dresses_the_hull_in_the_ship_it_is_built_for() {
    let (app, _) = generate_app();
    let sections = app.world().resource::<GameSections>();
    let styles = app.world().resource::<GameStyles>();
    let grammar = app
        .world()
        .resource::<GameGrammars>()
        .get_grammar(STANDARD_HULL_GRAMMAR_ID)
        .expect("the base content ships one")
        .clone();
    let second = styles.get(1).expect("more than one style").id.clone();

    let bare = collapse(sections, &grammar, Some(styles), 7, false, None).expect("a hull");
    assert!(!bare.skin, "a ship with its plating off gets a bare hull");
    assert_eq!(bare.style, None, "and no style to wear it in");

    let chosen =
        collapse(sections, &grammar, Some(styles), 7, true, Some(&second)).expect("a hull");
    assert_eq!(chosen.style.as_deref(), Some(second.as_str()));

    let unchosen = collapse(sections, &grammar, Some(styles), 7, true, None).expect("a hull");
    assert_eq!(
        unchosen.style.as_deref(),
        styles.first().map(|style| style.id.as_str()),
        "an unchosen style is the first one, which is what the node means by None"
    );

    assert_eq!(
        bare.sections.len(),
        chosen.sections.len(),
        "cladding is a look, not a hull: the same seed lays the same sections"
    );
}

/// The lint is the LAST gate and the one that decides whether a bad collapse
/// reaches the document. A refusal leaves the ship exactly as it was.
///
/// Reaching it needs a fault the TILER cannot see. Geometry is not one: a tile
/// is only built if its body stays inside its own cell, which is what makes
/// the overlap arm unreachable by construction. Sockets are - the tiler reads
/// one face per socket and does not care how many sockets name it, so a
/// prototype carrying the same socket twice tiles cleanly, and then every
/// neighbour that mates with that face has two points to mate with.
#[test]
fn a_hull_the_lint_refuses_never_enters_the_document() {
    let (mut app, ship) = generate_app();
    let button = generate_button(&mut app);

    // A good hull first, so the refusal has something to leave alone.
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    let laid = hull_of(&mut app, ship);
    assert!(!laid.is_empty(), "the first press is the one that works");

    let keel = app
        .world()
        .resource::<GameGrammars>()
        .get_grammar(STANDARD_HULL_GRAMMAR_ID)
        .expect("the base content ships one")
        .keel
        .hull
        .clone();
    {
        let mut sections = app.world_mut().resource_mut::<GameSections>();
        let block = sections
            .0
            .iter_mut()
            .find(|section| section.base.id == keel)
            .expect("the keel prototype is in the catalog");
        let doubled = block
            .base
            .link_points
            .iter()
            .map(|point| LinkPoint {
                id: format!("{}_again", point.id),
                ..point.clone()
            })
            .collect::<Vec<_>>();
        block.base.link_points.extend(doubled);
    }

    app.world_mut().trigger(Activate { entity: button });
    app.update();

    assert_eq!(
        hull_of(&mut app, ship),
        laid,
        "a refused hull replaced the one the ship was holding"
    );
    let line = status_line(&app);
    assert!(
        line.contains("the collapse built a hull the game refuses"),
        "the status has to say WHY nothing happened, got: {line}"
    );
    assert!(
        line.contains("ambiguous mates"),
        "and it has to be the LINT that refused, not the collapse: {line}"
    );
}
