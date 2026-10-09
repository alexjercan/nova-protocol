//! world_sector_inspector: look at one generated sector of the base game's
//! world at a time.
//!
//! The base game's generator, `NovaLayeredWorld`, pinned to the loaded
//! catalog through `featured_world_config`, on the examples' 32 km cell. No
//! New Game, no player ship, no `WorldObserver` and no `NovaWorldPlugin`:
//! nothing streams. The inspector asks for ONE cell, prepares it on
//! `AsyncComputeTaskPool` with `prepare_sector`, which generates and validates
//! it, and spawns it with `materialize_sector` once it is ready. The cell on
//! screen stays until its successor is ready; then the old root goes and the
//! new one is spawned in the same command batch, so at most one `SectorRoot`
//! is ever live. A new request replaces a pending one, and dropping its task
//! cancels it, so a stale request is never materialized.
//!
//! The readout names the seed, the cell, the object counts and every object
//! name. A new cell always opens in orbit around its first object, whatever
//! the camera did in the cell before, because the free camera was left a cell
//! away. An empty cell is a valid cell: it has no orbit target, and the free
//! camera is put at its centre.
//!
//! # Hand-run
//!
//! ```text
//! cargo run --example world_sector_inspector --features debug
//! cargo run --example world_sector_inspector --features debug -- --seed 115
//! ```
//!
//! | key | what it does |
//! | - | - |
//! | Left / Right | step the cell along X |
//! | Down / Up | step the cell along Z |
//! | Page Down / Page Up | step the cell along Y |
//! | [ / ] | orbit the previous / next object of the cell |
//! | WASD, Space, Shift, RMB drag | leave orbit and fly the free camera |
//!
//! Without `--seed` the inspector uses the examples' world seed and logs it.
//!
//! Harnessed mode:
//! - `NOVA_AUTOPILOT=1`: inspect the home cell, replace a pending request,
//!   step away and back to the same cell, cycle the selection, leave orbit,
//!   orbit the first ship, step off its cell and back with the free camera
//!   parked on that ship, orbit the first planetoid the home window holds,
//!   and visit an empty cell. This is the path `probe run` takes.
//! - With `NOVA_CAPTURE_DIR` set: also capture the ship and the planetoid as
//!   `world-sector-inspector-<subject>-<id>.png`.
//!
//! # Panics
//!
//! When the loaded catalog does not arm the generator, when the config does
//! not validate, when a requested cell does not generate or validate, when a
//! generated ship design does not resolve against the loaded sections, when a
//! ship is held back as a `PendingSectorShip`, when more than one sector root
//! is live, when a live root is not the cell the inspector materialized, when
//! more than one live entity carries a selected object's id or it stands under
//! another root, and when a planetoid has no meshed surface child.

#[path = "../shared/world_fixture/mod.rs"]
pub mod world_fixture;

use bevy::{
    prelude::*,
    tasks::{block_on, poll_once, AsyncComputeTaskPool, Task},
};
use clap::Parser;
use nova_protocol::prelude::*;
use nova_world::{materialize_sector, prelude::*, ObserverBody};
use world_fixture::{featured_world_config, free_play_scenario, EXAMPLE_SEED, FEATURE_HOME};

#[derive(Parser)]
#[command(name = "world_sector_inspector")]
#[command(version = "1.0.0")]
#[command(
    about = "Inspect one generated sector of the base game's world at a time",
    long_about = None
)]
struct Cli {
    /// The world seed. Without it, the examples' world seed.
    #[arg(long)]
    seed: Option<u32>,
}

/// The empty bootstrap this session runs inside.
const SCENARIO_ID: &str = "world_sector_inspector";

/// How bright the inspector's key light is.
const KEY_ILLUMINANCE: f32 = 6_000.0;

/// Where the key light points from.
const KEY_DIRECTION: Vec3 = Vec3::new(-0.4, -1.0, -0.6);

/// Orbit distance in clearances of the selected object.
const ORBIT_CLEARANCES: f32 = 2.5;

/// How far above the selected object's horizon the orbit sits, in radians.
const ORBIT_PITCH: f32 = 0.35;

/// How many object names the readout lists around the selection.
const READOUT_NAMES: usize = 12;

/// In-step seconds the first beat gets before the run aborts naming it: the
/// asset load, the bootstrap and the home cell. A hang backstop.
#[cfg(feature = "debug")]
const LOAD_DEADLINE_SECS: f32 = 120.0;

/// In-step seconds a search of the home window gets: up to 125 cells, each
/// prepared and spawned. A hang backstop.
#[cfg(feature = "debug")]
const SEARCH_DEADLINE_SECS: f32 = 60.0;

/// In-step seconds every other beat gets: at most one cell prepared and
/// spawned. A hang backstop. With the two above, the script's deadlines sum
/// to 465 s, under the 480 s run deadline that `probe run --timeout 510` sets
/// for the worldgen shard, so a stall is named by its beat.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 15.0;

/// The world the inspector generates from: the featured config with the
/// requested seed. Example-owned and not a `WorldConfig` resource, so nothing
/// can mistake it for an armed streaming world.
#[derive(Resource)]
struct InspectorWorld(WorldConfig<NovaLayeredWorld>);

/// One object of the live cell, in spawn order.
#[derive(Debug, Clone)]
struct InspectedObject {
    id: String,
    /// The radius the orbit is sized from: a rock's clearance at its widest
    /// meshed reach, a planetoid's body radius, a ship's declared clearance.
    clearance: Meters,
}

/// How the camera follows the cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InspectorCameraType {
    /// Framed on the selected object through a `ScriptedCameraAnchor`.
    Orbit,
    /// The shipped `WASDCameraController`.
    Free,
}

/// The cell being prepared. Replaced, and so cancelled, by a newer request.
struct SectorRequest {
    coord: SectorCoord,
    task: Task<Result<PreparedSector, SectorFault>>,
}

/// Everything the inspector knows about what it shows and what it waits for.
#[derive(Resource)]
struct Inspector {
    /// The cell on screen, and its root.
    live: Option<(SectorCoord, Entity)>,
    /// The live cell's canonical description, as `SectorDescription::canonical`
    /// writes it.
    canonical: String,
    objects: Vec<InspectedObject>,
    rocks: usize,
    planets: usize,
    ships: usize,
    /// The cell being prepared, if any.
    request: Option<SectorRequest>,
    /// Requests dropped because a newer one replaced them.
    cancelled: u32,
    /// Cells spawned in this run.
    materialized: u32,
    selected: Option<usize>,
    camera: InspectorCameraType,
}

/// Marks the readout text.
#[derive(Component)]
struct InspectorReadout;

fn main() -> bevy::app::AppExit {
    let seed = Cli::parse().seed;
    let mut app = AppBuilder::new()
        .with_game_plugins((move |app: &mut App| inspector_plugin(app, seed),))
        .build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(inspector_script());
    }

    app.run()
}

fn inspector_plugin(app: &mut App, seed: Option<u32>) {
    app.add_systems(
        OnEnter(GameAssetsStates::Loaded),
        move |commands: Commands,
              game_assets: Res<GameAssets>,
              loaded: Res<LoadedSectionPacks>,
              styles: Res<GameStyles>| {
            boot_inspector(commands, game_assets, loaded, styles, seed);
        },
    );
    app.add_systems(
        Update,
        (
            request_home.run_if(resource_exists::<Inspector>),
            step_sector.run_if(resource_exists::<Inspector>),
            cycle_selection.run_if(resource_exists::<Inspector>),
            leave_orbit.run_if(resource_exists::<Inspector>),
            collect_sector.run_if(resource_exists::<Inspector>),
            check_live_sector.run_if(resource_exists::<Inspector>),
            aim_camera.run_if(resource_exists::<Inspector>),
            update_readout.run_if(resource_exists::<Inspector>),
        )
            .chain(),
    );
}

/// Load the empty bootstrap, build and check the world, light the scene and
/// put the readout up. No cell is requested until the bootstrap is built: its
/// load sweeps every scenario-scoped entity, and a sector root is one.
///
/// The seed is logged here rather than in `main`, which runs before the log
/// subscriber exists.
///
/// # Panics
///
/// When the loaded catalog does not arm the generator, and when the config
/// does not validate.
fn boot_inspector(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    loaded: Res<LoadedSectionPacks>,
    styles: Res<GameStyles>,
    seed: Option<u32>,
) {
    let seed = match seed {
        Some(seed) => {
            info!("world_sector_inspector: seed {seed} (from --seed)");
            seed
        }
        None => {
            info!("world_sector_inspector: seed {EXAMPLE_SEED} (the examples' default)");
            EXAMPLE_SEED
        }
    };
    commands.trigger(LoadScenario(free_play_scenario(
        &game_assets,
        SCENARIO_ID,
        "World Sector Inspector",
    )));

    let config = WorldConfig {
        seed,
        ..featured_world_config(&loaded, &styles)
    };
    config
        .validate()
        .unwrap_or_else(|fault| panic!("world_sector_inspector: seed {seed}: {fault}"));
    commands.insert_resource(InspectorWorld(config));
    commands.insert_resource(Inspector {
        live: None,
        canonical: String::new(),
        objects: Vec::new(),
        rocks: 0,
        planets: 0,
        ships: 0,
        request: None,
        cancelled: 0,
        materialized: 0,
        selected: None,
        camera: InspectorCameraType::Orbit,
    });

    commands.spawn((
        Name::new("Inspector Key Light"),
        DirectionalLight {
            illuminance: KEY_ILLUMINANCE,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_translation(Vec3::ZERO).looking_to(KEY_DIRECTION, Vec3::Y),
    ));

    // Top left: the dev overlay's fps and version bar sits along the top right.
    commands
        .spawn((
            Name::new("Inspector Readout"),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(12.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                InspectorReadout,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Ask for the home cell once the bootstrap is built and its camera is up.
fn request_home(
    mut inspector: ResMut<Inspector>,
    world: Res<InspectorWorld>,
    gate: Res<ScenarioLoadGate>,
    current: Res<CurrentScenario>,
    cameras: Query<(), With<ScenarioCameraMarker>>,
) {
    if inspector.live.is_some()
        || inspector.request.is_some()
        || gate.is_held()
        || current.is_none()
        || cameras.is_empty()
    {
        return;
    }
    request_sector(&mut inspector, &world.0, FEATURE_HOME);
}

/// Prepare `coord` off the frame, replacing any pending request.
///
/// The cell on screen is the target already, so asking for it drops the
/// pending request and starts nothing.
fn request_sector(
    inspector: &mut Inspector,
    config: &WorldConfig<NovaLayeredWorld>,
    coord: SectorCoord,
) {
    if let Some(stale) = inspector.request.take() {
        info!(
            "inspector: dropping the pending request for {}, {coord} replaces it",
            stale.coord
        );
        inspector.cancelled += 1;
    }
    if inspector.live.is_some_and(|(live, _)| live == coord) {
        return;
    }
    info!("inspector: preparing {coord}");
    let config = config.clone();
    inspector.request = Some(SectorRequest {
        coord,
        task: AsyncComputeTaskPool::get().spawn(async move { prepare_sector(config, coord) }),
    });
}

/// The cell the next step counts from: the pending one, else the live one.
fn target_sector(inspector: &Inspector) -> Option<SectorCoord> {
    inspector
        .request
        .as_ref()
        .map(|request| request.coord)
        .or(inspector.live.map(|(coord, _)| coord))
}

/// Step the requested cell with the arrow keys and Page Up / Page Down.
fn step_sector(
    keys: Res<ButtonInput<KeyCode>>,
    world: Res<InspectorWorld>,
    mut inspector: ResMut<Inspector>,
) {
    let steps = [
        (KeyCode::ArrowRight, IVec3::X),
        (KeyCode::ArrowLeft, IVec3::NEG_X),
        (KeyCode::ArrowUp, IVec3::Z),
        (KeyCode::ArrowDown, IVec3::NEG_Z),
        (KeyCode::PageUp, IVec3::Y),
        (KeyCode::PageDown, IVec3::NEG_Y),
    ];
    let step: IVec3 = steps
        .iter()
        .filter(|(key, _)| keys.just_pressed(*key))
        .map(|(_, step)| *step)
        .sum();
    if step == IVec3::ZERO {
        return;
    }
    let Some(from) = target_sector(&inspector) else {
        return;
    };
    request_sector(
        &mut inspector,
        &world.0,
        from.offset(step.x, step.y, step.z),
    );
}

/// Cycle the selection with `[` and `]` and put the camera back in orbit.
fn cycle_selection(keys: Res<ButtonInput<KeyCode>>, mut inspector: ResMut<Inspector>) {
    let delta = match (
        keys.just_pressed(KeyCode::BracketLeft),
        keys.just_pressed(KeyCode::BracketRight),
    ) {
        (true, false) => -1,
        (false, true) => 1,
        _ => return,
    };
    let count = inspector.objects.len();
    let Some(selected) = inspector.selected else {
        info!("inspector: the cell is empty, there is nothing to orbit");
        return;
    };
    let next = (selected as isize + delta).rem_euclid(count as isize) as usize;
    inspector.selected = Some(next);
    inspector.camera = InspectorCameraType::Orbit;
}

/// Leave orbit for the free camera on any WASD key.
fn leave_orbit(keys: Res<ButtonInput<KeyCode>>, mut inspector: ResMut<Inspector>) {
    if inspector.camera == InspectorCameraType::Orbit
        && keys.any_just_pressed([KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD])
    {
        inspector.camera = InspectorCameraType::Free;
    }
}

/// Spawn the requested cell once it is ready: the old root goes and the new
/// one is spawned in the same command batch.
///
/// The observer is `outside_observer`, not the camera: the inspector has no
/// body to collide with a ship, and the free camera may stand anywhere,
/// inside a ship's clearance too, when the cell is asked for.
///
/// # Panics
///
/// On any fault the preparation returns, through `outside_observer`, and
/// through `materialize_sector` on a ship design the loaded sections do not
/// resolve.
fn collect_sector(
    mut commands: Commands,
    mut inspector: ResMut<Inspector>,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    world: Res<InspectorWorld>,
    cameras: Query<Entity, With<ScenarioCameraMarker>>,
) {
    let Some(request) = inspector.request.as_mut() else {
        return;
    };
    let Some(result) = block_on(poll_once(&mut request.task)) else {
        return;
    };
    let coord = request.coord;
    inspector.request = None;
    let prepared = result.unwrap_or_else(|fault| {
        panic!(
            "world_sector_inspector: seed {} cell {coord}: {fault}",
            world.0.seed
        )
    });
    let Ok(camera) = cameras.single() else {
        panic!("world_sector_inspector: no single scenario camera to observe {coord}");
    };

    let description = prepared.description();
    let canonical = description.canonical();
    let rocks = description.asteroids().len();
    let planets = description.planets().len();
    let ships = description.ships().len();
    let objects: Vec<InspectedObject> = description
        .asteroids()
        .iter()
        .map(|body| InspectedObject {
            id: body.id.clone(),
            clearance: Meters(body.radius.get() * ASTEROID_GEOMETRIC_FACTOR_MAX),
        })
        .chain(description.planets().iter().map(|planet| InspectedObject {
            id: planet.id.clone(),
            clearance: planet.config.body_radius(),
        }))
        .chain(description.ships().iter().map(|ship| InspectedObject {
            id: ship.id.clone(),
            clearance: ship.clearance,
        }))
        .collect();
    let observer = outside_observer(coord, world.0.sector_edge, description.ships());

    if let Some((old, root)) = inspector.live {
        debug!("inspector: retiring {old}");
        commands.entity(root).despawn();
    }
    let texture: AssetRef<Image> = game_assets.asteroid_texture.clone().into();
    let root = materialize_sector(&mut commands, prepared, None, &texture, &sections, observer);
    info!(
        "inspector: live {coord} of seed {}: {rocks} rocks, {planets} planetoids, {ships} ships",
        world.0.seed
    );

    // The old anchor's subject is gone with its root; the aim re-frames the
    // new selection once its entity exists.
    commands.entity(camera).remove::<ScriptedCameraAnchor>();
    if objects.is_empty() {
        let centre = coord.centre(world.0.sector_edge).to_engine();
        commands
            .entity(camera)
            .remove::<WASDCameraController>()
            .insert(Transform::from_translation(centre).looking_to(Vec3::NEG_Z, Vec3::Y))
            .insert(WASDCameraController);
    }

    inspector.live = Some((coord, root));
    inspector.canonical = canonical;
    inspector.selected = (!objects.is_empty()).then_some(0);
    inspector.camera = if objects.is_empty() {
        InspectorCameraType::Free
    } else {
        InspectorCameraType::Orbit
    };
    inspector.objects = objects;
    inspector.rocks = rocks;
    inspector.planets = planets;
    inspector.ships = ships;
    inspector.materialized += 1;
}

/// The bodiless observer `collect_sector` hands `materialize_sector`: two cell
/// edges past `coord`'s centre on every axis.
///
/// A ship's clearance sphere lies wholly inside its cell, at most half an
/// edge from the centre on every axis, so this point is at least one and a
/// half edges from any clearance and never holds a ship back. Each ship is
/// still checked with `bodies_clear`, the overlap test `materialize_sector`
/// runs, because so far from the origin `f32` rounding can move the point.
///
/// # Panics
///
/// When the point is not finite, or overlaps a clearance of `ships`.
fn outside_observer(coord: SectorCoord, edge: Meters, ships: &[SectorShip]) -> ObserverBody {
    let away = 2.0 * edge.get();
    let position = coord.centre(edge) + Meters3::new(away, away, away);
    assert!(
        position.get().is_finite(),
        "world_sector_inspector: no finite observer point two edges past {coord}"
    );
    for ship in ships {
        assert!(
            bodies_clear(
                position,
                Meters::ZERO,
                ship.position,
                ship.clearance,
                Meters::ZERO
            ),
            "world_sector_inspector: the observer point {:?} past {coord} overlaps ship '{}'",
            position.get(),
            ship.id
        );
    }
    ObserverBody {
        position,
        reach: Meters::ZERO,
    }
}

/// Refuse more than one live root, a root that is not the materialized cell,
/// and a held ship, which `outside_observer` rules out.
fn check_live_sector(
    inspector: Res<Inspector>,
    roots: Query<(Entity, &SectorRoot)>,
    pending: Query<&PendingSectorShip>,
) {
    let live: Vec<(Entity, SectorCoord)> =
        roots.iter().map(|(root, cell)| (root, cell.0)).collect();
    match (live.as_slice(), inspector.live) {
        ([], _) => {}
        ([(root, coord)], Some((expected, expected_root))) => assert!(
            *root == expected_root && *coord == expected,
            "world_sector_inspector: root {root} of {coord} is live, not {expected_root} of \
             {expected}"
        ),
        (many, _) => panic!(
            "world_sector_inspector: {} sector roots are live: {many:?}",
            many.len()
        ),
    }
    if let Some(held) = pending.iter().next() {
        panic!(
            "world_sector_inspector: ship '{}' is held back as a PendingSectorShip: the \
             observer stood in its {:.0} m clearance when its cell spawned",
            held.ship().id,
            held.ship().clearance.get()
        );
    }
}

/// The live entity of `id`, which must stand under `root`.
///
/// `None` until the cell's spawn commands land.
///
/// # Panics
///
/// When two live entities carry `id`, or the one that does stands under
/// another root.
fn resolve_object(
    id: &str,
    root: Entity,
    objects: &Query<(Entity, &EntityId, &ChildOf)>,
) -> Option<Entity> {
    let mut found = objects.iter().filter(|(_, entity_id, _)| entity_id.0 == id);
    let (entity, _, parent) = found.next()?;
    assert!(
        found.next().is_none(),
        "world_sector_inspector: more than one live entity carries '{id}'"
    );
    assert_eq!(
        parent.parent(),
        root,
        "world_sector_inspector: '{id}' resolved to {entity} under another root"
    );
    Some(entity)
}

/// Frame the selected object in orbit, or hand the camera to the free rig.
fn aim_camera(
    mut commands: Commands,
    inspector: Res<Inspector>,
    objects: Query<(Entity, &EntityId, &ChildOf)>,
    cameras: Query<
        (
            Entity,
            Option<&ScriptedCameraAnchor>,
            Has<WASDCameraController>,
        ),
        With<ScenarioCameraMarker>,
    >,
) {
    let Some((_, root)) = inspector.live else {
        return;
    };
    let Ok((camera, anchor, free)) = cameras.single() else {
        return;
    };
    match (inspector.camera, inspector.selected) {
        (InspectorCameraType::Orbit, Some(selected)) => {
            let object = &inspector.objects[selected];
            let Some(target) = resolve_object(&object.id, root, &objects) else {
                return;
            };
            if anchor.is_some_and(|anchor| anchor.anchor == target) && !free {
                return;
            }
            let reach = object.clearance.get() * ORBIT_CLEARANCES;
            let offset = Meters3::new(0.0, reach * ORBIT_PITCH.sin(), reach * ORBIT_PITCH.cos());
            info!("inspector: orbit '{}' entity {target}", object.id);
            commands
                .entity(camera)
                .remove::<WASDCameraController>()
                .insert(ScriptedCameraAnchor {
                    anchor: target,
                    offset,
                    frame: CameraOffsetFrame::World,
                    look_at: ScriptedCameraLookAt::Anchor,
                });
        }
        (InspectorCameraType::Orbit, None) => {}
        (InspectorCameraType::Free, _) => {
            // One command: the anchor's release drops the scripted transform
            // before the rig's first write, and the rig starts from the pose
            // the orbit left.
            if anchor.is_some() || !free {
                info!("inspector: free camera");
                commands
                    .entity(camera)
                    .remove::<ScriptedCameraAnchor>()
                    .insert(WASDCameraController);
            }
        }
    }
}

/// Name the seed, the cell, the counts and the objects around the selection.
fn update_readout(
    inspector: Res<Inspector>,
    world: Res<InspectorWorld>,
    names: Query<(&EntityId, &Name)>,
    mut readout: Query<&mut Text, With<InspectorReadout>>,
) {
    let Ok(mut text) = readout.single_mut() else {
        return;
    };
    let cell = inspector
        .live
        .map_or_else(|| "none".to_string(), |(coord, _)| coord.to_string());
    let mut lines = vec![
        format!(
            "SEED {}  SECTOR {cell}  {}",
            world.0.seed,
            match inspector.camera {
                InspectorCameraType::Orbit => "orbit",
                InspectorCameraType::Free => "free",
            }
        ),
        format!(
            "{} objects: {} rocks, {} planetoids, {} ships",
            inspector.objects.len(),
            inspector.rocks,
            inspector.planets,
            inspector.ships
        ),
    ];
    if let Some(request) = &inspector.request {
        lines.push(format!("preparing {}", request.coord));
    }
    if inspector.objects.is_empty() && inspector.live.is_some() {
        lines.push("empty cell: nothing to orbit".to_string());
    }
    let selected = inspector.selected.unwrap_or(0);
    let first = selected
        .saturating_sub(READOUT_NAMES / 2)
        .min(inspector.objects.len().saturating_sub(READOUT_NAMES));
    for (index, object) in inspector
        .objects
        .iter()
        .enumerate()
        .skip(first)
        .take(READOUT_NAMES)
    {
        let name = names
            .iter()
            .find(|(id, _)| id.0 == object.id)
            .map_or(object.id.as_str(), |(_, name)| name.as_str());
        let mark = if inspector.selected == Some(index) {
            ">"
        } else {
            " "
        };
        lines.push(format!("{mark} {:>3} {name}", index + 1));
    }
    lines.push("arrows X/Z  PgUp/PgDn Y  [ ] orbit object  WASD free camera".to_string());
    **text = lines.join("\n");
}

/// The run gate: the home cell, a replaced request, a return to the home
/// cell, the selection and free controls, a ship and a planetoid in orbit,
/// and an empty cell.
#[cfg(feature = "debug")]
fn inspector_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    use std::sync::Arc;

    use nova_autopilot::prelude::{press_key, release_key};
    use nova_protocol::nova_debug::harness::AutopilotPlugin;

    AutopilotPlugin::<GameStates>::new()
        .step("inspector: the home cell is live")
        .until(Arc::new(|world: &World| settled_on(world, FEATURE_HOME)))
        .deadline(LOAD_DEADLINE_SECS)
        .add()
        .step("inspector: Right steps to +X")
        .on_enter(|world: &mut World| {
            let inspector = world.resource::<Inspector>();
            world.insert_resource(HomeRecord {
                canonical: inspector.canonical.clone(),
            });
            press_key(KeyCode::ArrowRight)(world);
        })
        .until(Arc::new(|world: &World| {
            settled_on(world, FEATURE_HOME.offset(1, 0, 0))
        }))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("inspector: Up replaces a pending +2X request with +2X +Z")
        .on_enter(|world: &mut World| {
            release_key(KeyCode::ArrowRight)(world);
            let materialized = world.resource::<Inspector>().materialized;
            world.insert_resource(ReplaceRecord { materialized });
            // The request and the key land in one frame, so the key always
            // finds the request pending, however fast a worker prepares it.
            request_from_script(world, FEATURE_HOME.offset(2, 0, 0));
            press_key(KeyCode::ArrowUp)(world);
        })
        .until(Arc::new(|world: &World| {
            settled_on(world, FEATURE_HOME.offset(2, 0, 1))
        }))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("inspector: step back to the home cell")
        .on_enter(|world: &mut World| {
            release_key(KeyCode::ArrowUp)(world);
            let inspector = world.resource::<Inspector>();
            assert!(
                inspector.cancelled >= 1
                    && inspector.materialized == world.resource::<ReplaceRecord>().materialized + 1,
                "world_sector_inspector: the +2X request was materialized instead of replaced \
                 ({} replaced, {} materialized)",
                inspector.cancelled,
                inspector.materialized
            );
            request_from_script(world, FEATURE_HOME);
        })
        .until(Arc::new(|world: &World| settled_on(world, FEATURE_HOME)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("inspector: the home cell is the same cell")
        .on_enter(|world: &mut World| {
            let inspector = world.resource::<Inspector>();
            let home = world.resource::<HomeRecord>();
            assert_eq!(
                inspector.canonical, home.canonical,
                "world_sector_inspector: the home cell came back as a different cell"
            );
            press_key(KeyCode::BracketRight)(world);
        })
        .until(Arc::new(|world: &World| {
            let inspector = world.resource::<Inspector>();
            inspector.objects.len() < 2 || inspector.selected == Some(1) && orbiting(world)
        }))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("inspector: W leaves orbit")
        .on_enter(|world: &mut World| {
            release_key(KeyCode::BracketRight)(world);
            press_key(KeyCode::KeyW)(world);
        })
        .until(Arc::new(free_flying))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("inspector: ] orbits again")
        .on_enter(|world: &mut World| {
            release_key(KeyCode::KeyW)(world);
            press_key(KeyCode::BracketRight)(world);
        })
        .until(Arc::new(orbiting))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("inspector: search the home window for a cell with a ship")
        .on_enter(|world: &mut World| {
            release_key(KeyCode::BracketRight)(world);
            world.insert_resource(ScriptWalk::window());
        })
        .each(|world: &mut World, _, _| walk_until(world, |inspector| inspector.ships > 0))
        .until(Arc::new(|world: &World| {
            settled_here(world) && world.resource::<Inspector>().ships > 0
        }))
        .deadline(SEARCH_DEADLINE_SECS)
        .add()
        .step("inspector: orbit the first ship")
        .on_enter(|world: &mut World| {
            let mut inspector = world.resource_mut::<Inspector>();
            inspector.selected = Some(inspector.rocks + inspector.planets);
            world.insert_resource(ScriptShot::default());
        })
        .each(|world: &mut World, _, _| shoot_selection(world, "ship"))
        .until(Arc::new(shot_done))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("inspector: step off the ship's cell")
        .on_enter(|world: &mut World| {
            let (ship_cell, _) = world
                .resource::<Inspector>()
                .live
                .expect("the ship's cell is live");
            request_from_script(world, ship_cell.offset(1, 0, 0));
        })
        .until(Arc::new(settled_here))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("inspector: a free camera parked on a ship does not hold it")
        .on_enter(|world: &mut World| {
            let (beside, _) = world
                .resource::<Inspector>()
                .live
                .expect("the cell beside the ship's is live");
            let ship_cell = beside.offset(-1, 0, 0);
            let description = generate_sector(&world.resource::<InspectorWorld>().0, ship_cell)
                .unwrap_or_else(|fault| panic!("world_sector_inspector: {ship_cell}: {fault}"));
            let ship = description
                .ships()
                .first()
                .expect("the ship's cell has a ship");
            let at = ship.position.to_engine();
            info!(
                "inspector: free camera parked on ship '{}' at {at}, requesting {ship_cell}",
                ship.id
            );
            world.resource_mut::<Inspector>().camera = InspectorCameraType::Free;
            let mut cameras = world.query_filtered::<Entity, With<ScenarioCameraMarker>>();
            let camera = cameras.single(world).expect("one scenario camera is live");
            world
                .entity_mut(camera)
                .remove::<(ScriptedCameraAnchor, WASDCameraController)>()
                .insert(Transform::from_translation(at))
                .insert(WASDCameraController);
            request_from_script(world, ship_cell);
        })
        .until(Arc::new(|world: &World| {
            let inspector = world.resource::<Inspector>();
            if !settled_here(world) || inspector.ships == 0 {
                return false;
            }
            let ship = &inspector.objects[inspector.rocks + inspector.planets].id;
            let spawned = world
                .try_query::<&EntityId>()
                .is_some_and(|mut ids| ids.iter(world).any(|id| id.0 == *ship));
            let roots = world
                .try_query::<&SectorRoot>()
                .map_or(0, |mut roots| roots.iter(world).count());
            assert!(
                spawned && roots == 1,
                "world_sector_inspector: ship '{ship}' spawned {spawned} under {roots} live \
                 sector root(s)"
            );
            true
        }))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("inspector: search the home window for a cell with a planetoid")
        .on_enter(|world: &mut World| world.insert_resource(ScriptWalk::window()))
        .each(|world: &mut World, _, _| walk_until(world, |inspector| inspector.planets > 0))
        .until(Arc::new(|world: &World| {
            settled_here(world) && world.resource::<Inspector>().planets > 0
        }))
        .deadline(SEARCH_DEADLINE_SECS)
        .add()
        .step("inspector: orbit the first planetoid")
        .on_enter(|world: &mut World| {
            let mut inspector = world.resource_mut::<Inspector>();
            inspector.selected = Some(inspector.rocks);
            world.insert_resource(ScriptShot::default());
        })
        .each(|world: &mut World, _, _| shoot_selection(world, "planet"))
        .until(Arc::new(shot_done))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("inspector: search the home window for an empty cell")
        .on_enter(|world: &mut World| world.insert_resource(ScriptWalk::window()))
        .each(|world: &mut World, _, _| {
            walk_until(world, |inspector| inspector.objects.is_empty());
        })
        .until(Arc::new(|world: &World| {
            settled_here(world) && world.resource::<Inspector>().objects.is_empty()
        }))
        .deadline(SEARCH_DEADLINE_SECS)
        .add()
        .step("inspector: ] in an empty cell keeps the free camera")
        .on_enter(press_key(KeyCode::BracketRight))
        .add()
        .step("inspector: the empty cell has no orbit target")
        .on_enter(|world: &mut World| {
            release_key(KeyCode::BracketRight)(world);
            let inspector = world.resource::<Inspector>();
            assert!(
                inspector.selected.is_none() && inspector.camera == InspectorCameraType::Free,
                "world_sector_inspector: an empty cell selected {:?} in {:?}",
                inspector.selected,
                inspector.camera
            );
            info!(
                "inspector: {} cell(s) materialized, {} request(s) replaced",
                inspector.materialized, inspector.cancelled
            );
        })
        .until(Arc::new(free_flying))
        .deadline(STEP_DEADLINE_SECS)
        .add()
}

/// How far around the home cell the harness searches for a subject. The
/// fixture's home window of this radius holds planetoids, derelicts and
/// empty cells.
#[cfg(feature = "debug")]
const SEARCH_RADIUS: i32 = 2;

/// Frames a framed subject holds before it is captured, so it is drawn.
#[cfg(feature = "debug")]
const SETTLE_FRAMES: u32 = 6;

/// The home cell's description as it first came up.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct HomeRecord {
    canonical: String,
}

/// Cells materialized before the replaced request was made.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct ReplaceRecord {
    materialized: u32,
}

/// The cells the current search has still to visit, in `desired_sectors`
/// order.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct ScriptWalk(std::collections::VecDeque<SectorCoord>);

#[cfg(feature = "debug")]
impl ScriptWalk {
    /// Every cell of the home window.
    fn window() -> Self {
        Self(
            desired_sectors(FEATURE_HOME, SEARCH_RADIUS)
                .into_iter()
                .collect(),
        )
    }
}

/// The current subject's settle frames and capture.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct ScriptShot {
    settled: u32,
    shot: Option<String>,
    done: bool,
}

/// Whether the live cell is settled, whatever it is.
#[cfg(feature = "debug")]
fn settled_here(world: &World) -> bool {
    world
        .get_resource::<Inspector>()
        .and_then(|inspector| inspector.live)
        .is_some_and(|(coord, _)| settled_on(world, coord))
}

/// Request the next cell of the search while the settled cell is one `found`
/// refuses.
///
/// # Panics
///
/// When no cell of the home window is one `found` accepts.
#[cfg(feature = "debug")]
fn walk_until(world: &mut World, found: fn(&Inspector) -> bool) {
    if !settled_here(world) || found(world.resource::<Inspector>()) {
        return;
    }
    let next = world.resource_mut::<ScriptWalk>().0.pop_front();
    let Some(next) = next else {
        panic!(
            "world_sector_inspector: no cell within {SEARCH_RADIUS} of {FEATURE_HOME} holds the \
             subject"
        );
    };
    request_from_script(world, next);
}

/// Settle on the orbited selection, check that it is drawn, and capture it
/// under `NOVA_CAPTURE_DIR`.
///
/// # Panics
///
/// When a planetoid has no meshed surface child.
#[cfg(feature = "debug")]
fn shoot_selection(world: &mut World, subject: &str) {
    use nova_autopilot::prelude::{capture_window, CaptureLog, CAPTURE_DIR_ENV};

    if !orbiting(world) || world.resource::<ScriptShot>().done {
        return;
    }
    let inspector = world.resource::<Inspector>();
    let object = inspector.objects[inspector.selected.expect("orbiting has a selection")].clone();
    let shot = world.resource::<ScriptShot>();
    if shot.settled < SETTLE_FRAMES {
        world.resource_mut::<ScriptShot>().settled += 1;
        return;
    }
    if shot.shot.is_none() {
        let mut entities = world.query::<(Entity, &EntityId)>();
        let entity = entities
            .iter(world)
            .find(|(_, id)| id.0 == object.id)
            .map(|(entity, _)| entity)
            .expect("an orbited subject is live");
        let name = world
            .get::<Name>(entity)
            .map_or_else(String::new, |name| name.to_string());
        if subject == "planet" {
            let mut surfaces =
                world.query_filtered::<&ChildOf, (With<PlanetRenderBody>, With<Mesh3d>)>();
            assert!(
                surfaces.iter(world).any(|mount| mount.parent() == entity),
                "world_sector_inspector: planet '{}' has no meshed surface child",
                object.id
            );
        }
        info!(
            "inspector: settled on {subject} '{}' ({name}) entity {entity}",
            object.id
        );
        if std::env::var(CAPTURE_DIR_ENV).is_ok_and(|dir| !dir.is_empty()) {
            let file = format!("world-sector-inspector-{subject}-{}.png", object.id);
            capture_window(world, &file);
            world.resource_mut::<ScriptShot>().shot = Some(file);
        } else {
            world.resource_mut::<ScriptShot>().done = true;
        }
        return;
    }
    let written = shot.shot.as_ref().is_some_and(|file| {
        world
            .get_resource::<CaptureLog>()
            .is_some_and(|log| log.wrote(file))
    });
    world.resource_mut::<ScriptShot>().done = written;
}

/// Whether the current subject is settled and, on a capture run, shot.
#[cfg(feature = "debug")]
fn shot_done(world: &World) -> bool {
    world
        .get_resource::<ScriptShot>()
        .is_some_and(|shot| shot.done)
}

/// Request `coord` the way a key would.
#[cfg(feature = "debug")]
fn request_from_script(world: &mut World, coord: SectorCoord) {
    world.resource_scope(|world, inspector_world: Mut<InspectorWorld>| {
        let mut inspector = world.resource_mut::<Inspector>();
        request_sector(&mut inspector, &inspector_world.0, coord);
    });
}

/// Whether `coord` is live, nothing is pending, and the camera is framed.
#[cfg(feature = "debug")]
fn settled_on(world: &World, coord: SectorCoord) -> bool {
    let Some(inspector) = world.get_resource::<Inspector>() else {
        return false;
    };
    inspector.live.is_some_and(|(live, _)| live == coord)
        && inspector.request.is_none()
        && match inspector.selected {
            Some(_) => orbiting(world),
            None => free_flying(world),
        }
}

/// Whether the camera orbits the selected object's live entity.
#[cfg(feature = "debug")]
fn orbiting(world: &World) -> bool {
    let inspector = world.resource::<Inspector>();
    let Some(selected) = inspector.selected else {
        return false;
    };
    let id = &inspector.objects[selected].id;
    let Some(mut cameras) =
        world.try_query_filtered::<(&ScriptedCameraAnchor, Has<WASDCameraController>), With<ScenarioCameraMarker>>()
    else {
        return false;
    };
    let Ok((anchor, free)) = cameras.single(world) else {
        return false;
    };
    !free
        && world
            .get::<EntityId>(anchor.anchor)
            .is_some_and(|anchor_id| anchor_id.0 == *id)
}

/// Whether the camera is the free rig with no anchor.
#[cfg(feature = "debug")]
fn free_flying(world: &World) -> bool {
    let Some(mut cameras) = world.try_query_filtered::<
        (Has<ScriptedCameraAnchor>, Has<WASDCameraController>),
        With<ScenarioCameraMarker>,
    >() else {
        return false;
    };
    cameras
        .single(world)
        .is_ok_and(|(anchored, free)| !anchored && free)
}
