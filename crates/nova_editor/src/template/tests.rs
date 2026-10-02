//! What each template has to seed, and what pressing its row does to the
//! document.

use bevy::ui_widgets::{observe, Activate};
use nova_gameplay::prelude::{GravitySettings, GravityWell};

use super::*;
use crate::{
    config::SelectedNode,
    node::{reset_document, EditContext, NodeId, ObjectNode},
};

/// Every template says what it is. A row with no label is a row nobody can
/// choose between.
#[test]
fn every_template_names_itself_and_says_what_it_starts() {
    for template in ScenarioTemplate::ALL {
        assert!(!template.label().is_empty());
        assert!(!template.hint().is_empty());
        assert!(!template.settings().name.is_empty());
        assert!(!template.settings().description.is_empty());
    }
}

/// The range template is the world the editor has always opened on, unchanged
/// by the picker being put in front of it.
#[test]
fn the_range_template_is_the_stock_range() {
    assert_eq!(
        ScenarioTemplate::Range.objects().len(),
        default_world_objects().len()
    );
    assert_eq!(
        ScenarioTemplate::Range.script().len(),
        default_script().len()
    );
}

/// The arena's rocks are SCATTERED, not authored: the whole reason the map is
/// worth starting from is that its dressing is two nodes rather than two dozen.
#[test]
fn the_arena_scatters_its_dressing_rather_than_authoring_it() {
    let objects = ScenarioTemplate::Arena.objects();
    let rocks = objects
        .iter()
        .filter(|object| matches!(object.kind, ScenarioObjectKind::Asteroid(_)))
        .count();
    assert_eq!(
        rocks, 0,
        "every rock is scattered; the landmark is a planet"
    );

    let scatters: usize = ScenarioTemplate::Arena
        .script()
        .iter()
        .flat_map(|event| event.actions.iter())
        .filter(|action| matches!(action, EventActionConfig::ScatterObjects(_)))
        .count();
    assert_eq!(scatters, ARENA_RINGS.len(), "one action per ring of rock");
}

/// Every zero-velocity rock in an arena ring must start outside the planet's
/// pull, including the outermost rock mesh, not just its nominal radius.
#[test]
fn the_arena_rock_rings_clear_the_planet_well() {
    let planet = arena_planetoid();
    let ScenarioObjectKind::Planet(config) = &planet.kind else {
        panic!("the arena landmark must be a planet");
    };
    let well = GravityWell::from_mass(
        config.mass,
        config.body_radius().to_engine(),
        &GravitySettings::default(),
    );
    let planet_center = planet.base.position.to_engine();
    for ring in &ARENA_RINGS {
        let EventActionConfig::ScatterObjects(scatter) = ring_scatter(ring) else {
            panic!("each arena ring must scatter rocks");
        };
        let ScatterRegion::Ring {
            center,
            inner,
            outer,
            y_min,
            y_max,
        } = scatter.region
        else {
            panic!("each arena ring must stay a ring");
        };
        let relative = planet_center - center.to_engine();
        let radial = relative.x.hypot(relative.z);
        let horizontal_gap = (inner.to_engine() - radial)
            .max(radial - outer.to_engine())
            .max(0.0);
        let vertical_gap = (y_min.to_engine() - relative.y)
            .max(relative.y - y_max.to_engine())
            .max(0.0);
        let closest = horizontal_gap.hypot(vertical_gap);
        let max_radius = scatter
            .asteroid_radius
            .expect("arena rock ring has a radius range")
            .1;
        let required = well.soi_radius + max_radius.to_engine() * ASTEROID_GEOMETRIC_FACTOR_MAX;
        assert!(
            closest > required,
            "{} ring comes within {closest:.0}u of the planet, under its {required:.0}u well plus maximum rock reach",
            ring.id_prefix
        );
    }
}

/// And it lights itself. A scenario that authors no light renders black, which
/// is a map that reads as broken rather than as empty.
#[test]
fn the_arena_lights_itself() {
    let lights = ScenarioTemplate::Arena
        .objects()
        .iter()
        .filter(|object| matches!(object.kind, ScenarioObjectKind::Light(_)))
        .count();
    assert!(lights > 0, "the arena template authors no light at all");
}

/// The empty template seeds NOTHING - that is the whole of what it offers, and
/// a stray light or belt would make it a fourth map instead.
#[test]
fn the_empty_template_seeds_nothing() {
    assert!(ScenarioTemplate::Empty.objects().is_empty());
    assert!(ScenarioTemplate::Empty.script().is_empty());
}

/// Pressing a template row throws the document away and founds the world that
/// row names - the live verb, over the row it reads its answer off.
#[test]
fn pressing_a_template_row_founds_that_world() {
    let mut app = App::new();
    app.init_resource::<EditContext>();
    app.init_resource::<SelectedNode>();
    app.init_resource::<crate::bundle::DocumentSlot>();
    let scenario = crate::node::found_document(
        &mut app.world_mut().commands(),
        None,
        &mut EditContext::default(),
        ScenarioTemplate::Range,
    );
    let _ = scenario;
    app.world_mut().flush();

    let row = app
        .world_mut()
        .spawn((ScenarioTemplate::Arena, observe(reset_document)))
        .id();
    app.world_mut().trigger(Activate { entity: row });
    app.world_mut().flush();

    let roots: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, With<ScenarioNode>>()
        .iter(app.world())
        .collect();
    assert_eq!(roots.len(), 1, "the old document went away");
    assert_eq!(
        app.world().get::<ScenarioNode>(roots[0]).unwrap().name,
        ScenarioTemplate::Arena.settings().name,
        "the new one is the world the row named"
    );
    let named: Vec<String> = app
        .world_mut()
        .query_filtered::<&NodeId, With<ObjectNode>>()
        .iter(app.world())
        .map(|id| id.0.clone())
        .collect();
    assert!(
        named.iter().any(|id| id == "arena_planetoid"),
        "the arena's landmark is in the document: {named:?}"
    );
    assert!(
        !named.iter().any(|id| id.starts_with("hulk_")),
        "and the range's hulks are not: {named:?}"
    );
}
