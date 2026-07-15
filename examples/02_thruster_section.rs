//! 02_thruster_section: the thruster section - burn in, thrust and plume out.
//!
//! One minimal ship (controller + hull + main drive, no player input) holds
//! a steady full burn written straight into [`ThrusterSectionInput`] - the
//! seam the key bindings, the manual-burn allocator and the autopilot write.
//! The section under test converts that 0..1 throttle into impulse on the
//! hull and into the exhaust plume's shader (the same
//! `thruster_shader_update_system` the game runs).
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example 02_thruster_section --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `burn probe: thrust accelerates the hull and drives the plume`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use avian3d::prelude::{LinearVelocity, Rotation};
#[cfg(feature = "debug")]
use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use nova_protocol::nova_gameplay::sections::thruster_section::ThrusterExhaustMaterial;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "02_thruster_section")]
#[command(version = "1.0.0")]
#[command(about = "Thruster section: a steady burn accelerates the hull and drives the plume shader", long_about = None)]
struct Cli;

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // Headless smoke-test harness: the probe asserts the section's whole
    // point - the burn actually accelerates the hull along its nose, and
    // the exhaust shader follows the throttle.
    #[cfg(feature = "debug")]
    {
        app.init_resource::<BurnProbe>();
        app.add_plugins(nova_autopilot().input(autopilot_burn_probe));
        app.add_plugins(nova_screenshot());
    }

    app.run();
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_rig);
    app.add_systems(Update, hold_full_burn);
}

fn setup_rig(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(burn_rig(&game_assets, &sections)));
}

/// The rig scenario: one sectioned ship, no player and no AI - throttle
/// authority belongs to this example's burn writer alone.
fn burn_rig(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };

    let ship = SpaceshipConfig {
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
                id: "hull".to_string(),
                position: Vec3::new(0.0, 0.0, 1.0),
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(section("reinforced_hull_section")),
                modifications: vec![],
            },
            SpaceshipSectionConfig {
                id: "main_drive".to_string(),
                position: Vec3::new(0.0, 0.0, 2.0),
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(section("basic_thruster_section")),
                modifications: vec![],
            },
        ],
    };

    ScenarioConfig {
        id: "thruster_rig".to_string(),
        name: "Thruster Section Rig".to_string(),
        description: "A minimal ship under a steady full burn.".to_string(),
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

/// Hold the drive at full throttle - the seam the key bindings and the
/// manual-burn allocator write.
fn hold_full_burn(
    mut q_input: Query<
        &mut ThrusterSectionInput,
        (With<ThrusterSectionMarker>, Without<SectionInactiveMarker>),
    >,
) {
    for mut input in &mut q_input {
        if input.0 != 1.0 {
            input.0 = 1.0;
        }
    }
}

/// Stage tracker for the burn probe.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct BurnProbe {
    baseline: Option<(f32, f32)>,
    asserted: bool,
}

/// Autopilot script: sample the hull's speed along its nose early, then
/// again after a couple of seconds of burn - the speed must have GROWN
/// (delivery guard and assertion in one: a dead drive shows zero growth),
/// and the exhaust plume's shader uniform must sit at the held throttle.
#[cfg(feature = "debug")]
fn autopilot_burn_probe(world: &mut World, elapsed: f32) {
    // Backstop before the state gate: if the window is about to close and
    // the probe never completed (loading ate the window, a stage stalled),
    // fail loudly instead of vacuously passing.
    if elapsed > nova_protocol::nova_debug::harness::NOVA_AUTOPILOT_SECS - 0.3
        && !world.resource::<BurnProbe>().asserted
    {
        panic!("burn probe: never completed within the autopilot window");
    }
    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }
    if world.resource::<BurnProbe>().asserted {
        return;
    }

    let nose_speed = {
        let mut q =
            world.query_filtered::<(&Rotation, &LinearVelocity), With<SpaceshipRootMarker>>();
        let Ok((rotation, velocity)) = q.single(world) else {
            return;
        };
        velocity.0.dot(rotation.0 * Vec3::NEG_Z)
    };

    let Some((baseline, sampled_at)) = world.resource::<BurnProbe>().baseline else {
        world.resource_mut::<BurnProbe>().baseline = Some((nose_speed, elapsed));
        return;
    };
    if elapsed < sampled_at + 2.0 {
        return;
    }

    assert!(
        nose_speed > baseline + 0.2,
        "burn probe: nose speed must grow under a full burn, went {baseline:.3} \
         -> {nose_speed:.3} u/s"
    );

    // The plume shader follows the throttle through the production sync
    // (thruster_shader_update_system), not through anything this example
    // wired by hand.
    let handles: Vec<_> = {
        let mut q = world
            .query::<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, ThrusterExhaustMaterial>>>(
            );
        q.iter(world).map(|material| material.0.clone()).collect()
    };
    assert!(
        !handles.is_empty(),
        "burn probe: the drive must have spawned its exhaust plume material"
    );
    let materials =
        world.resource::<Assets<ExtendedMaterial<StandardMaterial, ThrusterExhaustMaterial>>>();
    for handle in &handles {
        let input = materials
            .get(handle)
            .expect("burn probe: plume material missing from assets")
            .extension
            .thruster_input;
        assert!(
            (input - 1.0).abs() < 1e-6,
            "burn probe: plume shader input {input} did not follow the held throttle (1.0)"
        );
    }

    info!(
        "burn probe: thrust accelerates the hull and drives the plume \
         ({baseline:.3} -> {nose_speed:.3} u/s)"
    );
    world.resource_mut::<BurnProbe>().asserted = true;
}
