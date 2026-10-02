//! Keep the menu freight lane mobile without sending its rocks into the planet.

use bevy::prelude::Vec3;
use nova_authoring::prelude::build_scenarios;
use nova_gameplay::prelude::{circular_orbit_speed, GravitySettings, GravityWell};
use nova_scenario::prelude::*;

/// The former zero-velocity scatter placed every cargo rock inside this well.
/// Their authored first-frame velocities must fit their own orbital radii.
#[test]
fn cargo_rocks_start_in_collision_clear_tangent_orbits_about_the_menu_planet() {
    let scenario = build_scenarios()
        .into_iter()
        .find(|scenario| scenario.id == "menu_waystation")
        .expect("the menu waystation is built");
    let actions: Vec<_> = scenario
        .events
        .iter()
        .filter(|event| matches!(event.name, EventConfig::OnStart))
        .flat_map(|event| &event.actions)
        .filter_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object) => Some(object),
            _ => None,
        })
        .collect();
    let planet = actions
        .iter()
        .find_map(|object| match &object.kind {
            ScenarioObjectKind::Planet(config) if object.base.id == "menu_planetoid" => {
                Some(config)
            }
            _ => None,
        })
        .expect("the cargo lane keeps its well");
    let settings = GravitySettings::default();
    let well = GravityWell::from_mass(planet.mass, planet.body_radius().to_engine(), &settings);
    let mut positions: Vec<(Vec3, f32)> = Vec::new();
    for object in actions {
        if !object.base.id.starts_with("waystation_cargo_") {
            continue;
        }
        let ScenarioObjectKind::Asteroid(rock) = &object.kind else {
            panic!("{} must remain a mobile cargo rock", object.base.id);
        };
        let radial = object.base.position.to_engine();
        let r = radial.length();
        let clearance = rock.radius.to_engine() * ASTEROID_GEOMETRIC_FACTOR_MAX;
        let velocity = rock.initial_velocity.to_engine();
        assert!(
            rock.initial_velocity.is_finite(),
            "{} finite",
            object.base.id
        );
        // Leave an extra full rock reach above the solid planet; pairs only
        // need their two reaches to avoid an initial overlap.
        assert!(
            r > well.body_radius + clearance * 2.0
                && r + clearance < well.soi_radius * (1.0 - settings.fade_fraction),
            "{} has a body-clear orbit inside the unfaded well",
            object.base.id
        );
        assert!(
            velocity.dot(radial).abs() < 0.01,
            "{} moves tangent to its radius",
            object.base.id
        );
        assert!(
            (velocity.length() - circular_orbit_speed(well.mu, r)).abs() < 0.001,
            "{} authors its own circular speed",
            object.base.id
        );
        for (other, reach) in &positions {
            assert!(
                radial.distance(*other) > clearance + *reach,
                "{} begins clear of every other cargo rock",
                object.base.id
            );
        }
        positions.push((radial, clearance));
    }
    assert_eq!(positions.len(), 18, "the freight lane keeps all its rocks");
}
