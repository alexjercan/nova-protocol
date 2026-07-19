//! hull_section: hull sections through the damage pipeline.
//!
//! One minimal ship (controller + two hulls, no player input) takes fire
//! from this example's script: first a partial hit - the hull's `Health`
//! drops by exactly the applied amount - then an overkill hit that destroys
//! the section outright while the REST of the ship survives it (the
//! integrity graph detaches the dead leaf; the root and the controller stay
//! alive). This is the per-section slice of the health -> destroy pipeline;
//! ship-wide destruction physics gets its own deep-dive in com_range.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example hull_section --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `hull probe: damage lands, the section dies, the ship survives`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "hull_section")]
#[command(version = "1.0.0")]
#[command(about = "Hull sections: damage drops health, overkill destroys the section, the ship survives", long_about = None)]
struct Cli;

/// The partial hit the probe lands first, well under the hull's health.
#[cfg(feature = "debug")]
const PARTIAL_HIT: f32 = 60.0;

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // Headless smoke-test harness: the probe asserts the pipeline's whole
    // point - damage lands on the section's Health, overkill destroys the
    // section, and the ship survives losing a leaf.
    #[cfg(feature = "debug")]
    {
        app.init_resource::<HullProbe>();
        // Probe wiring (task 20260719-210443; each plugin is inert without
        // its NOVA_PERF_* env): run timeline + engine-bound invariants +
        // frame-time capture, so `probe run` can measure this example.
        app.add_plugins(nova_probe::nova_timeline());
        app.add_plugins(nova_probe::nova_invariants());
        app.add_plugins(nova_probe::nova_frametime());
        app.add_plugins(nova_autopilot().input(autopilot_hull_probe));
        app.add_plugins(nova_screenshot());
    }

    app.run();
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_rig);
}

fn setup_rig(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(hull_rig(&game_assets, &sections)));
}

/// The rig scenario: controller + two hulls, so destroying the far hull
/// leaves a still-connected, still-alive ship.
fn hull_rig(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };

    let ship = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::None,
        sections: vec![
            SpaceshipSectionConfig {
                id: "controller".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(section("basic_controller_section")),
                modifications: vec![],
            },
            SpaceshipSectionConfig {
                id: "hull_near".to_string(),
                position: Vec3::new(0.0, 0.0, 1.0),
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(section("reinforced_hull_section")),
                modifications: vec![],
            },
            SpaceshipSectionConfig {
                id: "hull_far".to_string(),
                position: Vec3::new(0.0, 0.0, 2.0),
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(section("reinforced_hull_section")),
                modifications: vec![],
            },
        ],
    };

    ScenarioConfig {
        id: "hull_rig".to_string(),
        name: "Hull Section Rig".to_string(),
        description: "A minimal ship taking scripted section damage.".to_string(),
        cubemap: game_assets.cubemap.clone().into(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: vec![EventActionConfig::SpawnScenarioObject(
                ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: "rig_ship".to_string(),
                        name: "Rig Ship".to_string(),
                        position: Vec3::new(0.0, 0.0, -12.0),
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Spaceship(ship),
                },
            )],
        }],
        ..Default::default()
    }
}

/// Stage tracker for the hull probe.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct HullProbe {
    /// The far hull's entity and its spawn health, captured before any hit.
    target: Option<(Entity, f32, f32)>,
    partial_checked: bool,
    asserted: bool,
}

/// The FARTHEST hull from the ship root, by local mount position - the leaf
/// the rig destroys. Identified geometrically so the probe never guesses at
/// entity spawn order.
#[cfg(feature = "debug")]
fn far_hull(world: &mut World) -> Option<(Entity, f32)> {
    let mut q = world.query_filtered::<(Entity, &Transform, &Health), With<HullSectionMarker>>();
    q.iter(world)
        .map(|(entity, transform, health)| (entity, transform.translation.z, health.current))
        .reduce(|a, b| if a.1 >= b.1 { a } else { b })
        .map(|(entity, _, health)| (entity, health))
}

/// Autopilot script, three stages through the production damage path:
/// capture the far hull at full health, land a partial hit and assert the
/// exact drop, then overkill it and assert the section is GONE while the
/// ship root and the controller survive.
#[cfg(feature = "debug")]
fn autopilot_hull_probe(world: &mut World, elapsed: f32) {
    // Backstop before the state gate: if the window is about to close and
    // the probe never completed (loading ate the window, a stage stalled),
    // fail loudly instead of vacuously passing.
    if elapsed > nova_protocol::nova_debug::harness::NOVA_AUTOPILOT_SECS - 0.3
        && !world.resource::<HullProbe>().asserted
    {
        panic!("hull probe: never completed within the autopilot window");
    }
    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }
    if world.resource::<HullProbe>().asserted {
        return;
    }

    let Some((target, spawn_health, hit_at)) = world.resource::<HullProbe>().target else {
        // Stage 1: capture the target at rest and land the partial hit.
        let Some((entity, health)) = far_hull(world) else {
            return;
        };
        world.trigger(HealthApplyDamage {
            entity,
            source: None,
            amount: PARTIAL_HIT,
        });
        world.resource_mut::<HullProbe>().target = Some((entity, health, elapsed));
        return;
    };

    if !world.resource::<HullProbe>().partial_checked {
        // Stage 2 (a beat later): the hit landed on the section's Health,
        // exactly - then overkill it.
        if elapsed < hit_at + 0.5 {
            return;
        }
        let current = world
            .get::<Health>(target)
            .expect("hull probe: the hull must survive a partial hit")
            .current;
        assert!(
            (current - (spawn_health - PARTIAL_HIT)).abs() < 1e-3,
            "hull probe: expected {} - {PARTIAL_HIT} = {}, got {current}",
            spawn_health,
            spawn_health - PARTIAL_HIT
        );
        world.trigger(HealthApplyDamage {
            entity: target,
            source: None,
            amount: spawn_health * 10.0,
        });
        world.resource_mut::<HullProbe>().partial_checked = true;
        return;
    }

    // Stage 3 (another beat): the section is destroyed and gone; the ship
    // root and its controller live on.
    if elapsed < hit_at + 1.5 {
        return;
    }
    assert!(
        world.get_entity(target).is_err(),
        "hull probe: an overkilled hull section must be destroyed and despawned"
    );
    let (roots, controllers) = {
        let mut q_root = world.query_filtered::<(), With<SpaceshipRootMarker>>();
        let roots = q_root.iter(world).count();
        let mut q_controller = world.query_filtered::<(), With<ControllerSectionMarker>>();
        (roots, q_controller.iter(world).count())
    };
    assert_eq!(
        roots, 1,
        "hull probe: the ship root must survive losing a leaf hull"
    );
    assert_eq!(
        controllers, 1,
        "hull probe: the controller must survive losing a leaf hull"
    );
    info!("hull probe: damage lands, the section dies, the ship survives");
    world.resource_mut::<HullProbe>().asserted = true;
}
