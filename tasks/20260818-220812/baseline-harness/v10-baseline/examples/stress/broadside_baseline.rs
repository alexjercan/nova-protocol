//! THROWAWAY measurement harness. Never lands. Identical source in both trees.
//!
//! Boots the shipped `broadside` scenario out of the merged registry and hands
//! it to the frame-time capture, so v0.10.0 and HEAD can be measured on the
//! same subject through the same app shape.
//!
//! `NOVA_MEASURE_COMBAT=1` attaches [`combat_burst_driver`], which holds the
//! player's fire for the whole window. Without it the capture measures the
//! opening scene at rest: the corvettes are gated behind player movement that
//! nothing supplies, so an undriven window never sees a shot fired.
//!
//! The census walks ARCHETYPES rather than querying typed components, because
//! the two trees do not share a component vocabulary (`rounds` is new) and
//! neither re-exports avian through its prelude. Counting by component NAME is
//! the only instrument that reads the same on both sides.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use nova_protocol::prelude::*;

/// The shipped scenario under measurement.
const SCENARIO_ID: &str = "broadside";

/// Set to hold the player's fire for the whole capture window.
const COMBAT_ENV: &str = "NOVA_MEASURE_COMBAT";

/// Component-name substrings the census reports. Everything else is summed
/// into the entity total only.
const WATCHED: &[&str] = &[
    "RigidBody",
    "Collider",
    "LinearVelocity",
    "Mesh3d",
    "MeshMaterial3d",
    "Round",
    "Projectile",
    "Bullet",
    "Lifetime",
];

/// Frames after `Playing` at which the census logs its running peaks.
const REPORT_AT: &[u32] = &[240, 600, 1020];

fn main() -> bevy::app::AppExit {
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        let mut probe = nova_probe::NovaProbePlugin::default();
        if std::env::var_os(COMBAT_ENV).is_some() {
            probe = probe.drive_frametime(nova_probe::combat_burst_driver);
        }
        app.add_plugins(probe);
    }

    app.init_resource::<Census>();
    app.add_systems(Update, census_tick);

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_broadside);
}

fn load_broadside(mut commands: Commands, scenarios: Res<GameScenarios>) {
    let scenario = scenarios
        .get(SCENARIO_ID)
        .unwrap_or_else(|| {
            panic!(
                "scenario '{SCENARIO_ID}' is not registered; registry holds {:?}",
                scenarios.keys().collect::<Vec<_>>()
            )
        })
        .clone();
    info!("broadside_baseline: loading '{SCENARIO_ID}'");
    commands.trigger(LoadScenario(scenario));
}

/// Running peaks, so a burst that lands between reports is still counted.
#[derive(Resource, Default)]
struct Census {
    frames: u32,
    entities: u32,
    by_component: HashMap<String, u32>,
    meshes: usize,
    materials: usize,
    mesh_instances: usize,
}

fn census_tick(world: &mut World) {
    let playing = world
        .get_resource::<State<GameStates>>()
        .is_some_and(|s| *s.get() == GameStates::Playing);
    if !playing {
        return;
    }

    let frame = {
        let mut census = world.resource_mut::<Census>();
        census.frames += 1;
        census.frames
    };

    // Archetype walk: cheap enough at this cadence, and the only shape that
    // reads identically on two trees with different component vocabularies.
    let mut counts: HashMap<String, u32> = HashMap::new();
    for archetype in world.archetypes().iter() {
        let n = archetype.len();
        if n == 0 {
            continue;
        }
        for component_id in archetype.components() {
            let Some(info) = world.components().get_info(*component_id) else {
                continue;
            };
            let name = format!("{:?}", info.name());
            if WATCHED.iter().any(|w| name.contains(w)) {
                *counts.entry(name).or_default() += n;
            }
        }
    }
    let entities = world.entities().len();

    let mut meshes: HashSet<AssetId<Mesh>> = HashSet::new();
    let mut mesh_instances = 0usize;
    {
        let mut q = world.query::<&Mesh3d>();
        for mesh in q.iter(world) {
            meshes.insert(mesh.0.id());
            mesh_instances += 1;
        }
    }
    let mut materials: HashSet<AssetId<StandardMaterial>> = HashSet::new();
    {
        let mut q = world.query::<&MeshMaterial3d<StandardMaterial>>();
        for material in q.iter(world) {
            materials.insert(material.0.id());
        }
    }

    {
        let mut census = world.resource_mut::<Census>();
        census.entities = census.entities.max(entities);
        census.mesh_instances = census.mesh_instances.max(mesh_instances);
        census.meshes = census.meshes.max(meshes.len());
        census.materials = census.materials.max(materials.len());
        for (name, n) in counts {
            let slot = census.by_component.entry(name).or_default();
            *slot = (*slot).max(n);
        }
    }

    if REPORT_AT.contains(&frame) {
        let census = world.resource::<Census>();
        let mut rows: Vec<(&String, &u32)> = census.by_component.iter().collect();
        rows.sort_by(|a, b| b.1.cmp(a.1));
        info!(
            "NOVA_CENSUS frame={} entities_peak={} mesh_instances_peak={} \
             distinct_meshes_peak={} distinct_materials_peak={}",
            frame, census.entities, census.mesh_instances, census.meshes, census.materials
        );
        for (name, n) in rows {
            info!("NOVA_CENSUS frame={frame} peak={n} component={name}");
        }
    }
}
