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

use super::gate::fail_scenario_load;
use crate::prelude::*;

/// The held-handle resource and the walk that fills it.
pub mod prelude {
    pub use super::{scenario_render_meshes, ScenarioPreload};
}

/// How long a load waits WITHOUT PROGRESS before giving up, seconds.
///
/// A no-progress budget, not a total one. A total budget punishes a slow
/// machine for being slow: a big catalog on a cold spinning disk is WORKING,
/// and cutting it off at ten seconds was how a load that would have succeeded
/// turned into a scene with art missing. Every asset that settles resets the
/// budget, so the total load time is unbounded while anything is still
/// arriving.
///
/// What it catches is the case a total budget was really for: an asset source
/// that never answers at all. Ten seconds of complete silence from every
/// outstanding handle is a broken source, not a slow one.
const PRELOAD_STALL_SECS: f32 = 10.0;

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
    /// `Time<Real>` elapsed when the wait began. Absolute rather than an
    /// accumulated delta, so the reported wait is the wall time the load
    /// actually cost and not the frame that preceded it.
    started: f32,
    /// `Time<Real>` elapsed when this load last made progress, for
    /// [`PRELOAD_STALL_SECS`]. Set with `started`, then moved forward every
    /// time the outstanding count drops.
    progressed: f32,
    /// How many handles were outstanding at `progressed`. The progress test:
    /// fewer outstanding than last frame is an asset that settled.
    outstanding: usize,
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
/// `AssetServer`. A design or section prototype that resolves to nothing is
/// silently skipped: the spawn reports that miss, and `content lint` reports it
/// before the spawn ever runs.
///
/// Through [`EventActionConfig::walk`], so a spawn NESTED inside a `Sequence`
/// or `Cinematic` step names its art here too. A beat three deep still lands
/// mid-mission with no time to fetch anything, which is the case the whole
/// warm-up exists for.
pub fn scenario_render_meshes(
    scenario: &ScenarioConfig,
    ships: &GameShipDesigns,
    sections: &GameSections,
) -> Vec<AssetRef<WorldAsset>> {
    let mut meshes = Vec::new();
    for action in scenario.events.iter().flat_map(|event| &event.actions) {
        action.walk(&mut |action| {
            let object = match action {
                EventActionConfig::SpawnScenarioObject(object) => object,
                // Every copy a scatter places is a clone of the one template,
                // so the template names the whole field's art.
                EventActionConfig::ScatterObjects(scatter) => &scatter.template,
                _ => return,
            };
            // Ships are the only object kind that names a glTF. The rest build
            // primitives (beacon, salvage crate) or generate their mesh on a
            // worker (asteroid), and a light and an anchor have no mesh at all.
            let ScenarioObjectKind::Spaceship(spaceship) = &object.kind else {
                return;
            };
            // The SAME resolve the spawn runs, so the warm-up fetches the art
            // a patched ship actually wears. Errors are reported by the lint
            // and again by the spawn; a warm-up that finds nothing simply
            // warms nothing.
            let (design, _) = resolve_ship_design(&spaceship.design, ships, sections);
            for section in &design.sections {
                push_section_meshes(&section.config, &mut meshes);
            }
        });
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
        // No projectile mesh: a slug is a built-in nose cone, not an asset a
        // scenario can be waiting on.
        SectionKind::Railgun(railgun) => push_mesh(railgun.render_mesh.as_ref(), meshes),
        SectionKind::Docking(docking) => push_mesh(docking.render_mesh.as_ref(), meshes),
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
    ships: Res<GameShipDesigns>,
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
    preload.progressed = preload.started;
    preload.outstanding = preload.handles.len();
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
/// FAILURE counts as settled - as in "stopped moving", not as in "fine". A
/// failed handle ends the WAIT, and [`track_scenario_preload`] then fails the
/// whole load closed on it.
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

/// Release the load once every held mesh is in memory, fail it closed when one
/// of them failed, and fail it closed again after [`PRELOAD_STALL_SECS`] with
/// nothing moving.
///
/// `Time<Real>`, not the virtual clock: the load HOLDS the virtual clock (see
/// [`gate`](super::gate)), so a deadline measured on it would never arrive.
///
/// Failing CLOSED is the contract. The alternative - carry on with placeholder
/// art, let the real mesh pop in when it shows up - hands the player a scene
/// that is not the scene the author wrote and no way to know it.
fn track_scenario_preload(
    time: Res<Time<Real>>,
    asset_server: Res<AssetServer>,
    mut preload: ResMut<ScenarioPreload>,
    mut gate: ResMut<ScenarioLoadGate>,
    mut failure: Option<ResMut<ScenarioStartFailure>>,
    current: Res<CurrentScenario>,
) {
    if !preload.pending {
        return;
    }
    let now = time.elapsed_secs();
    let waited = now - preload.started;

    let outstanding = preload
        .handles
        .iter()
        .filter(|handle| !has_settled(&asset_server, handle))
        .count();
    // Any asset settling is progress, and progress buys the whole budget
    // again: a load is only stuck when NOTHING has moved for the whole of it.
    if outstanding < preload.outstanding {
        preload.outstanding = outstanding;
        preload.progressed = now;
    }

    if outstanding == 0 {
        let failed: Vec<String> = preload
            .handles
            .iter()
            .filter(|handle| !asset_server.is_loaded_with_dependencies(*handle))
            .map(mesh_name)
            .collect();
        preload.pending = false;
        if failed.is_empty() {
            debug!(
                "track_scenario_preload: {} render mesh(es) warm after {:.3}s",
                preload.handles.len(),
                waited
            );
            return;
        }
        let messages = failed
            .iter()
            .map(|mesh| format!("'{mesh}' failed to load"))
            .collect();
        fail_scenario_load(
            &mut gate,
            failure.as_deref_mut(),
            scenario_name(&current),
            messages,
        );
        return;
    }

    if now - preload.progressed >= PRELOAD_STALL_SECS {
        let stalled: Vec<String> = preload
            .handles
            .iter()
            .filter(|handle| !has_settled(&asset_server, handle))
            .map(mesh_name)
            .collect();
        let messages = stalled
            .iter()
            .map(|mesh| format!("'{mesh}' stopped loading"))
            .collect();
        preload.pending = false;
        fail_scenario_load(
            &mut gate,
            failure.as_deref_mut(),
            scenario_name(&current),
            messages,
        );
    }
}

/// What the failure report calls the scenario that could not start.
fn scenario_name(current: &CurrentScenario) -> String {
    current.as_ref().map_or_else(
        || "the scenario".to_string(),
        |scenario| scenario.name.clone(),
    )
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
    use nova_events::prelude::*;

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
        }
    }

    /// A `SpawnScenarioObject` action for a ship flying `design`.
    fn spawn_ship(id: &str, design: ShipDesignSource) -> EventActionConfig {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: id.to_string(),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                design,
                ..default()
            }),
        })
    }

    /// A fixed joint carrying `path` as its art, over `children`.
    fn joint(path: &str, children: Vec<TurretJoint>) -> TurretJoint {
        TurretJoint {
            name: None,
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
        let ships = GameShipDesigns(vec![
            ShipDesignPrototype {
                id: "opener_ship".to_string(),
                name: "Opener".to_string(),
                design: ShipDesign {
                    sections: vec![section_at("a", SectionSource::prototype("opener"))],
                    ..default()
                },
            },
            ShipDesignPrototype {
                id: "late_ship".to_string(),
                name: "Late".to_string(),
                design: ShipDesign {
                    sections: vec![section_at("a", SectionSource::prototype("late"))],
                    ..default()
                },
            },
        ]);

        let scenario = scenario_with(
            "two_beats",
            vec![
                event_with(vec![spawn_ship(
                    "opener",
                    ShipDesignSource::prototype("opener_ship"),
                )]),
                ScenarioEventConfig {
                    label: None,
                    name: EventConfig::OnTimerEnd,
                    once: false,
                    filters: vec![],
                    actions: vec![spawn_ship("late", ShipDesignSource::prototype("late_ship"))],
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
        let ships = GameShipDesigns(vec![]);

        let inline = ShipDesignSource::Inline(ShipDesign {
            sections: vec![section_at(
                "bay",
                SectionSource::Inline(hull_prototype("bay", "art/bay.glb#Scene0")),
            )],
            ..default()
        });
        let scattered = ShipDesignSource::Inline(ShipDesign {
            sections: vec![section_at("tile", SectionSource::prototype("rock_tile"))],
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
                min: Meters3::new(-100.0, -100.0, -100.0),
                max: Meters3::new(100.0, 100.0, 100.0),
            },
            template,
            asteroid_radius: None,
            asteroid_kinds: vec![],
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

    /// A spawn buried in a `Sequence` beat is exactly the case the warm-up
    /// exists for: it lands mid-mission with no time to fetch anything. The
    /// flat read of each handler's own action list walked straight past it,
    /// so its hull would have popped in late or not at all.
    #[test]
    fn the_walk_reaches_a_hull_spawned_from_inside_a_sequence_step() {
        let sections = GameSections(vec![hull_prototype("late", "art/late.glb#Scene0")]);
        let ships = GameShipDesigns(vec![ShipDesignPrototype {
            id: "late_ship".to_string(),
            name: "Late".to_string(),
            design: ShipDesign {
                sections: vec![section_at("a", SectionSource::prototype("late"))],
                ..default()
            },
        }]);

        let scenario = scenario_with(
            "chained",
            vec![event_with(vec![EventActionConfig::Sequence(
                SequenceActionConfig {
                    key: "opening".to_string(),
                    steps: vec![SequenceStepConfig {
                        after: Some(5.0),
                        actions: vec![spawn_ship("late", ShipDesignSource::prototype("late_ship"))],
                        ..default()
                    }],
                },
            )])],
        );

        assert_eq!(
            paths(&scenario_render_meshes(&scenario, &ships, &sections)),
            vec!["art/late.glb#Scene0"]
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
        let ships = GameShipDesigns(vec![ShipDesignPrototype {
            id: "gunboat".to_string(),
            name: "Gunboat".to_string(),
            design: ShipDesign {
                sections: vec![
                    section_at("plate_a", SectionSource::prototype("plate_a")),
                    section_at("plate_b", SectionSource::prototype("plate_b")),
                    section_at("turret", SectionSource::prototype("turret")),
                ],
                ..default()
            },
        }]);

        let scenario = scenario_with(
            "gunboat",
            vec![event_with(vec![spawn_ship(
                "gunboat",
                ShipDesignSource::prototype("gunboat"),
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
                spawn_ship("ghost", ShipDesignSource::prototype("no_such_ship")),
                spawn_ship(
                    "gappy",
                    ShipDesignSource::Inline(ShipDesign {
                        sections: vec![section_at(
                            "gap",
                            SectionSource::prototype("no_such_section"),
                        )],
                        ..default()
                    }),
                ),
                spawn_object_action(),
            ])],
        );

        assert!(
            scenario_render_meshes(&scenario, &GameShipDesigns(vec![]), &GameSections(vec![]))
                .is_empty()
        );
    }

    /// A rig with the tracker and the gate it reports to.
    fn preload_app(preload: ScenarioPreload) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_resource::<ScenarioLoadGate>();
        app.init_resource::<ScenarioStartFailure>();
        app.insert_resource(CurrentScenario(Some(scenario_with("stuck", vec![]))));
        app.add_systems(Update, track_scenario_preload);
        app.insert_resource(preload);
        app
    }

    /// The worst failure this warm-up could cause is a load that never ends,
    /// because the loading panel and the simulation hold both wait on it. A
    /// handle the `AssetServer` will never report on (here a defaulted one)
    /// stands in for the asset source that never answers.
    ///
    /// It ends CLOSED: the scene is missing art the author asked for, so the
    /// run reports rather than flying with holes in it.
    #[test]
    fn a_load_that_stops_moving_fails_closed() {
        let mut app = preload_app(ScenarioPreload {
            handles: vec![Handle::default()],
            pending: true,
            // Backdated past the budget rather than slept through it: the
            // system reads `Time<Real>`, which a test cannot advance.
            started: -PRELOAD_STALL_SECS - 1.0,
            progressed: -PRELOAD_STALL_SECS - 1.0,
            outstanding: 1,
        });

        app.update();

        assert!(
            !app.world().resource::<ScenarioPreload>().is_pending(),
            "a stalled preload must stop waiting"
        );
        assert_eq!(
            *app.world().resource::<ScenarioLoadGate>(),
            ScenarioLoadGate::Failed,
            "and fail the load rather than release it"
        );
        let report = app.world().resource::<ScenarioStartFailure>().0.clone();
        let report = report.expect("a failed load reports");
        assert_eq!(report.scenario_name, "Test Scenario");
        assert_eq!(report.messages.len(), 1, "the stalled path is named");
    }

    /// A load that is still MOVING is not a load that is stuck. The budget is
    /// per stall, not per load: the total-time budget this replaced cut off a
    /// cold-disk load of the shipped catalog that was working the whole time.
    #[test]
    fn progress_buys_the_whole_budget_again() {
        let mut app = preload_app(ScenarioPreload {
            handles: vec![Handle::default(), Handle::default()],
            pending: true,
            started: -600.0,
            // Nothing has moved for longer than the budget...
            progressed: -PRELOAD_STALL_SECS - 1.0,
            // ...but this frame finds fewer outstanding than the last frame
            // recorded here, which is an asset that settled in between.
            outstanding: 9,
        });

        app.update();

        assert_eq!(
            *app.world().resource::<ScenarioLoadGate>(),
            ScenarioLoadGate::Idle,
            "a load that made progress this frame is not a stalled load"
        );
        assert!(
            app.world().resource::<ScenarioStartFailure>().0.is_none(),
            "and nothing is reported"
        );
    }
}
