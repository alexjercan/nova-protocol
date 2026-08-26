//! The scenario's glTF warm-up: every section render mesh a loaded scenario can
//! ever spawn, resolved at LOAD and held for the scenario's lifetime.
//!
//! Holding is the whole mechanism - [`AssetRef::resolve`] is idempotent, so the
//! spawn site asks for the same path and gets a handle that is already warm,
//! and dropping the handle would let bevy free the mesh before the mid-mission
//! spawn that needs it.
//!
//! Change this module when a scenario gains a new way to NAME art.

use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    prelude::*,
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use crate::prelude::*;

/// The held-handle resource and the walk that fills it.
pub mod prelude {
    pub use super::{scenario_render_meshes, ScenarioPreload};
}

/// How long a load waits for the scenario's art before giving up, seconds.
///
/// The bound is for the FAILURE case only. A handle that neither loads nor
/// reports a failure - an asset source that never answers - would otherwise
/// hold the loading panel up forever, which is a far worse defect than the
/// pop-in this warm-up exists to remove. Set well above any honest load of the
/// shipped catalog, so a machine that trips it is broken rather than slow.
const PRELOAD_TIMEOUT_SECS: f32 = 10.0;

/// Every glTF the loaded scenario can spawn, held for its lifetime, plus
/// whether the load is still waiting on them.
///
/// STRONG handles on purpose: `AssetRef::resolve` hands the spawn site the same
/// handle for the same path, but only while something still holds one. Cleared
/// on [`UnloadScenario`] and rebuilt by the next load, so a scenario never pins
/// the previous one's art.
#[derive(Resource, Default)]
pub struct ScenarioPreload {
    /// The warmed meshes, in walk order.
    handles: Vec<Handle<WorldAsset>>,
    /// Whether anything in `handles` is still in flight.
    pending: bool,
    /// `Time<Real>` elapsed when the wait began, for [`PRELOAD_TIMEOUT_SECS`].
    /// Absolute rather than an accumulated delta, so the reported wait is the
    /// wall time the load actually cost and not the frame that preceded it.
    started: f32,
}

impl ScenarioPreload {
    /// Whether the load is still waiting for the scenario's art. The loading
    /// panel and [`scenario_has_settled`] both hold while this is true.
    pub fn is_pending(&self) -> bool {
        self.pending
    }
}

/// Every section render mesh the scenario can spawn, in walk order, with
/// repeats removed.
///
/// A spawn action carries its object's FULL config inline rather than an id
/// looked up later, so the whole set is readable from authored data plus the
/// two catalogs the spawn itself resolves against - no world and no
/// `AssetServer`. A hull or section prototype that resolves to nothing is
/// silently skipped: the spawn reports that miss, and `content lint` reports it
/// before the spawn ever runs.
pub fn scenario_render_meshes(
    scenario: &ScenarioConfig,
    ships: &GameShips,
    sections: &GameSections,
) -> Vec<AssetRef<WorldAsset>> {
    let mut meshes = Vec::new();
    for event in &scenario.events {
        for action in &event.actions {
            let object = match action {
                EventActionConfig::SpawnScenarioObject(object) => object,
                // Every copy a scatter places is a clone of the one template,
                // so the template names the whole field's art.
                EventActionConfig::ScatterObjects(scatter) => &scatter.template,
                _ => continue,
            };
            // Ships are the only object kind that names a glTF. The rest build
            // primitives (beacon, salvage crate) or generate their mesh on a
            // worker (asteroid), and a light and an anchor have no mesh at all.
            let ScenarioObjectKind::Spaceship(spaceship) = &object.kind else {
                continue;
            };
            let Some(hull) = spaceship.hull.resolve(ships) else {
                continue;
            };
            for section in &hull.sections {
                let config = match &section.source {
                    SectionSource::Inline(config) => config,
                    SectionSource::Prototype(id) => match sections.get_section(id) {
                        Some(config) => config,
                        None => continue,
                    },
                };
                push_section_meshes(config, &mut meshes);
            }
        }
    }
    meshes
}

/// Add one section's meshes to `meshes`.
fn push_section_meshes(config: &SectionConfig, meshes: &mut Vec<AssetRef<WorldAsset>>) {
    match &config.kind {
        SectionKind::Hull(hull) => push_mesh(hull.render_mesh.as_ref(), meshes),
        SectionKind::Thruster(thruster) => push_mesh(thruster.render_mesh.as_ref(), meshes),
        SectionKind::Controller(controller) => push_mesh(controller.render_mesh.as_ref(), meshes),
        SectionKind::Turret(turret) => {
            push_joint_meshes(&turret.root, meshes);
            push_mesh(turret.projectile_render_mesh.as_ref(), meshes);
        }
        SectionKind::Torpedo(torpedo) => {
            push_mesh(torpedo.render_mesh.as_ref(), meshes);
            push_mesh(torpedo.projectile_render_mesh.as_ref(), meshes);
        }
    }
}

/// Add a turret joint's mesh and every mesh below it in the joint tree.
fn push_joint_meshes(joint: &TurretJoint, meshes: &mut Vec<AssetRef<WorldAsset>>) {
    push_mesh(joint.render_mesh.as_ref(), meshes);
    for child in &joint.children {
        push_joint_meshes(child, meshes);
    }
}

/// Add one authored ref, skipping one already collected.
///
/// Linear rather than hashed: `AssetRef` keys on an authored path OR a live
/// handle, neither of which is `Hash`, and a hull's whole section list is tens
/// of entries.
fn push_mesh(mesh: Option<&AssetRef<WorldAsset>>, meshes: &mut Vec<AssetRef<WorldAsset>>) {
    if let Some(mesh) = mesh {
        if !meshes.contains(mesh) {
            meshes.push(mesh.clone());
        }
    }
}

/// Resolve and hold every render mesh the freshly loaded scenario can spawn.
///
/// On [`ScenarioLoaded`] rather than [`LoadScenario`]: the loader writes
/// [`CurrentScenario`] and only then triggers that, so this reads the config
/// that actually started rather than one a content error may have refused.
fn preload_scenario_render_meshes(
    _: On<ScenarioLoaded>,
    mut preload: ResMut<ScenarioPreload>,
    current: Res<CurrentScenario>,
    ships: Res<GameShips>,
    sections: Res<GameSections>,
    asset_server: Res<AssetServer>,
    time: Res<Time<Real>>,
) {
    let Some(scenario) = &**current else {
        return;
    };
    let meshes = scenario_render_meshes(scenario, &ships, &sections);
    preload.handles = meshes
        .iter()
        .map(|mesh| mesh.resolve(&asset_server))
        .collect();
    preload.pending = !preload.handles.is_empty();
    preload.started = time.elapsed_secs();
    debug!(
        "preload_scenario_render_meshes: '{}' warms {} render mesh(es)",
        scenario.id,
        preload.handles.len()
    );
}

/// Drop the held handles with the scenario that named them.
fn drop_scenario_preload(_: On<UnloadScenario>, mut preload: ResMut<ScenarioPreload>) {
    preload.handles.clear();
    preload.pending = false;
}

/// Whether this handle has stopped moving: loaded with its whole dependency
/// tree, or failed.
///
/// FAILURE counts as settled. The spawn falls back to placeholder art either
/// way, so holding the load until the deadline over a mesh that is never
/// arriving buys a blank screen and nothing else.
fn has_settled(asset_server: &AssetServer, handle: &Handle<WorldAsset>) -> bool {
    if asset_server.is_loaded_with_dependencies(handle) {
        return true;
    }
    matches!(
        asset_server.get_load_states(handle),
        Some((LoadState::Failed(_), _, _)) | Some((_, _, RecursiveDependencyLoadState::Failed(_)))
    )
}

/// The authored path behind a handle, for a log line. Handle-backed refs carry
/// no path, so they read as the asset id instead.
fn mesh_name(handle: &Handle<WorldAsset>) -> String {
    handle
        .path()
        .map(|path| path.to_string())
        .unwrap_or_else(|| format!("{:?}", handle.id()))
}

/// Release the load once every held mesh is in memory (or has failed), and at
/// [`PRELOAD_TIMEOUT_SECS`] regardless.
///
/// `Time<Real>`, not the virtual clock: a scenario can be loaded from a paused
/// outcome frame, where the virtual clock is stopped and the deadline would
/// never arrive.
fn track_scenario_preload(
    time: Res<Time<Real>>,
    asset_server: Res<AssetServer>,
    mut preload: ResMut<ScenarioPreload>,
) {
    if !preload.pending {
        return;
    }
    let waited = time.elapsed_secs() - preload.started;

    let outstanding = preload
        .handles
        .iter()
        .filter(|handle| !has_settled(&asset_server, handle))
        .count();

    if outstanding == 0 {
        let failed: Vec<String> = preload
            .handles
            .iter()
            .filter(|handle| !asset_server.is_loaded_with_dependencies(*handle))
            .map(mesh_name)
            .collect();
        if failed.is_empty() {
            debug!(
                "track_scenario_preload: {} render mesh(es) warm after {:.3}s",
                preload.handles.len(),
                waited
            );
        } else {
            warn!(
                "track_scenario_preload: {} render mesh(es) failed to load and will spawn as \
                 placeholder art: {}",
                failed.len(),
                failed.join(", ")
            );
        }
        preload.pending = false;
        return;
    }

    if waited >= PRELOAD_TIMEOUT_SECS {
        let stalled: Vec<String> = preload
            .handles
            .iter()
            .filter(|handle| !has_settled(&asset_server, handle))
            .map(mesh_name)
            .collect();
        warn!(
            "track_scenario_preload: giving up after {:.1}s with {} render mesh(es) still \
             loading: {}",
            waited,
            stalled.len(),
            stalled.join(", ")
        );
        preload.pending = false;
    }
}

/// Register the warm-up: the resource, the load/unload observers and the
/// readiness tracker.
///
/// `render == false` registers the RESOURCE only, so the settle gate can still
/// read it. A headless rig never builds a section's render mesh, so warming one
/// would load art nothing draws and then hold the scenario's clock waiting for
/// it.
pub(super) fn register_scenario_preload(app: &mut App, render: bool) {
    app.init_resource::<ScenarioPreload>();
    if !render {
        return;
    }
    app.add_observer(preload_scenario_render_meshes);
    app.add_observer(drop_scenario_preload);
    app.add_systems(Update, track_scenario_preload);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::fixtures::*;

    /// A hull section prototype whose art is `path`.
    fn hull_prototype(id: &str, path: &str) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                name: id.to_string(),
                health: 1.0,
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig {
                render_mesh: Some(AssetRef::from(path)),
                render_mesh_transform: None,
            }),
        }
    }

    /// One hull section placed at the origin from `source`.
    fn section_at(id: &str, source: SectionSource) -> SpaceshipSectionConfig {
        SpaceshipSectionConfig {
            id: id.to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            source,
            modifications: vec![],
        }
    }

    /// A `SpawnScenarioObject` action for a ship flying `hull`.
    fn spawn_ship(id: &str, hull: ShipSource) -> EventActionConfig {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: id.to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig { hull, ..default() }),
        })
    }

    /// A fixed joint carrying `path` as its art, over `children`.
    fn joint(path: &str, children: Vec<TurretJoint>) -> TurretJoint {
        TurretJoint {
            offset: Vec3::ZERO,
            axis: None,
            speed: 1.0,
            min: None,
            max: None,
            render_mesh: Some(AssetRef::from(path)),
            render_mesh_transform: None,
            muzzle: None,
            children,
        }
    }

    fn paths(meshes: &[AssetRef<WorldAsset>]) -> Vec<&str> {
        meshes.iter().filter_map(AssetRef::path).collect()
    }

    /// The whole point of the warm-up: a hull that only appears mid-mission is
    /// collected at load, alongside the one on screen from the start.
    #[test]
    fn the_walk_collects_a_hull_no_start_event_spawns() {
        let sections = GameSections(vec![
            hull_prototype("opener", "art/opener.glb#Scene0"),
            hull_prototype("late", "art/late.glb#Scene0"),
        ]);
        let ships = GameShips(vec![
            ShipConfig {
                id: "opener_ship".to_string(),
                name: "Opener".to_string(),
                hull: ShipHull {
                    sections: vec![section_at(
                        "a",
                        SectionSource::Prototype("opener".to_string()),
                    )],
                    ..default()
                },
            },
            ShipConfig {
                id: "late_ship".to_string(),
                name: "Late".to_string(),
                hull: ShipHull {
                    sections: vec![section_at(
                        "a",
                        SectionSource::Prototype("late".to_string()),
                    )],
                    ..default()
                },
            },
        ]);

        let scenario = scenario_with(
            "two_beats",
            vec![
                event_with(vec![spawn_ship(
                    "opener",
                    ShipSource::Prototype("opener_ship".to_string()),
                )]),
                ScenarioEventConfig {
                    name: EventConfig::OnTimerEnd,
                    once: false,
                    filters: vec![],
                    actions: vec![spawn_ship(
                        "late",
                        ShipSource::Prototype("late_ship".to_string()),
                    )],
                },
            ],
        );

        assert_eq!(
            paths(&scenario_render_meshes(&scenario, &ships, &sections)),
            vec!["art/opener.glb#Scene0", "art/late.glb#Scene0"]
        );
    }

    /// An inline hull is authored art like any other, and a scatter template
    /// names the whole field's art: both are walked.
    #[test]
    fn the_walk_reaches_an_inline_hull_and_a_scatter_template() {
        let sections = GameSections(vec![hull_prototype("rock_tile", "art/tile.glb#Scene0")]);
        let ships = GameShips(vec![]);

        let inline = ShipSource::Inline(ShipHull {
            sections: vec![section_at(
                "bay",
                SectionSource::Inline(hull_prototype("bay", "art/bay.glb#Scene0")),
            )],
            ..default()
        });
        let scattered = ShipSource::Inline(ShipHull {
            sections: vec![section_at(
                "tile",
                SectionSource::Prototype("rock_tile".to_string()),
            )],
            ..default()
        });

        let EventActionConfig::SpawnScenarioObject(template) = spawn_ship("drone", scattered)
        else {
            unreachable!("spawn_ship builds a SpawnScenarioObject");
        };
        let scatter = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
            id_prefix: "drone_".to_string(),
            count: 8,
            seed: 1,
            region: ScatterRegion::Box {
                min: Vec3::splat(-10.0),
                max: Vec3::splat(10.0),
            },
            template,
            asteroid_radius: None,
            min_separation: None,
        });

        let scenario = scenario_with(
            "inline_and_scatter",
            vec![event_with(vec![spawn_ship("battery", inline), scatter])],
        );

        assert_eq!(
            paths(&scenario_render_meshes(&scenario, &ships, &sections)),
            vec!["art/bay.glb#Scene0", "art/tile.glb#Scene0"]
        );
    }

    /// A turret's art hangs off its joint TREE, not off the section, so the
    /// walk has to descend; and one mesh shared by two sections is warmed once.
    #[test]
    fn the_walk_descends_a_turret_joint_tree_and_collects_a_shared_mesh_once() {
        let turret = SectionConfig {
            base: BaseSectionConfig {
                id: "turret".to_string(),
                name: "Turret".to_string(),
                health: 1.0,
                ..default()
            },
            kind: SectionKind::Turret(TurretSectionConfig {
                root: joint(
                    "art/yaw.glb#Scene0",
                    vec![joint("art/barrel.glb#Scene0", vec![])],
                ),
                projectile_render_mesh: Some(AssetRef::from("art/round.glb#Scene0")),
                ..TurretSectionConfig::default()
            }),
        };
        let sections = GameSections(vec![
            turret,
            hull_prototype("plate_a", "art/plate.glb#Scene0"),
            hull_prototype("plate_b", "art/plate.glb#Scene0"),
        ]);
        let ships = GameShips(vec![ShipConfig {
            id: "gunboat".to_string(),
            name: "Gunboat".to_string(),
            hull: ShipHull {
                sections: vec![
                    section_at("plate_a", SectionSource::Prototype("plate_a".to_string())),
                    section_at("plate_b", SectionSource::Prototype("plate_b".to_string())),
                    section_at("turret", SectionSource::Prototype("turret".to_string())),
                ],
                ..default()
            },
        }]);

        let scenario = scenario_with(
            "gunboat",
            vec![event_with(vec![spawn_ship(
                "gunboat",
                ShipSource::Prototype("gunboat".to_string()),
            )])],
        );

        assert_eq!(
            paths(&scenario_render_meshes(&scenario, &ships, &sections)),
            vec![
                "art/plate.glb#Scene0",
                "art/yaw.glb#Scene0",
                "art/barrel.glb#Scene0",
                "art/round.glb#Scene0",
            ]
        );
    }

    /// A hull naming no catalog ship, a section naming no prototype, and an
    /// object that is not a ship all contribute nothing - the walk must not
    /// panic on content the spawn itself refuses.
    #[test]
    fn the_walk_skips_what_resolves_to_nothing() {
        let scenario = scenario_with(
            "misses",
            vec![event_with(vec![
                spawn_ship("ghost", ShipSource::Prototype("no_such_ship".to_string())),
                spawn_ship(
                    "gappy",
                    ShipSource::Inline(ShipHull {
                        sections: vec![section_at(
                            "gap",
                            SectionSource::Prototype("no_such_section".to_string()),
                        )],
                        ..default()
                    }),
                ),
                spawn_object_action(),
            ])],
        );

        assert!(
            scenario_render_meshes(&scenario, &GameShips(vec![]), &GameSections(vec![])).is_empty()
        );
    }

    /// The worst failure this warm-up could cause is a load that never ends,
    /// because the loading panel and the scenario clock both wait on it. A
    /// handle the `AssetServer` will never report on (here a defaulted one)
    /// stands in for the asset source that never answers.
    #[test]
    fn a_wait_that_outlasts_the_deadline_ends_anyway() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.add_systems(Update, track_scenario_preload);
        app.insert_resource(ScenarioPreload {
            handles: vec![Handle::default()],
            pending: true,
            // Backdated past the deadline rather than slept through it: the
            // system reads `Time<Real>`, which a test cannot advance.
            started: -PRELOAD_TIMEOUT_SECS - 1.0,
        });

        app.update();

        assert!(
            !app.world().resource::<ScenarioPreload>().is_pending(),
            "a preload past its deadline must release the load"
        );
    }
}
