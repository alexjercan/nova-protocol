//! asteroid_kinds: the asteroid KIND lineup - five kinds across three
//! silhouette seeds, every rock spawned through the real scenario path
//! (`AsteroidConfig` -> `asteroid_scenario_object` -> `insert_asteroid_render`),
//! so what the frame shows is what a scenario gets.
//!
//! Columns are kinds and rows are seeds, which makes the two questions
//! readable in one frame: does a kind read as its own material, and do two
//! rocks OF a kind read as two rocks.
//!
//! The `plain` column is the control, not a fifth rock: every kind knob is off,
//! the per-body frame jitter included, so it is the surface as it was drawn
//! before kinds existed - the before picture, standing in the after picture's
//! own lighting. Its three rows differ only in silhouette, which is the point:
//! that is how much variety a field of rocks used to have.
//!
//! Hand-run (WASD + mouse free-fly):
//! ```text
//! cargo run --example asteroid_kinds --features debug
//! ```
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load, frame the grid, walk the closeups,
//!   exit clean. This is the path `probe run` takes.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot the grid, the four closeups,
//!   and the stone rock again after a cut has rebuilt its mesh (staged under
//!   `NOVA_CAPTURE_DIR`).

#[path = "shared/compare.rs"]
mod compare;

#[cfg(feature = "debug")]
use avian3d::prelude::RigidBody;
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "asteroid_kinds")]
#[command(version = "1.0.0")]
#[command(about = "Asteroid kinds across seeds, on the real rock mesh", long_about = None)]
struct Cli;

/// The kinds on show, left to right. The control leads, so the eye reads the
/// before picture first and every column after it is the difference.
const KINDS: [&str; 5] = [KIND_PLAIN, KIND_ROCK, KIND_METAL, KIND_ICE, KIND_CARBON];

/// The silhouette seeds, top to bottom. Small and hand-picked so a label is
/// readable and a re-run is the same lineup.
const SEEDS: [u32; 3] = [101, 202, 303];

/// Authored radius of every rock in the grid. The mesh reaches 3.5 to 6 times
/// past this ([`ASTEROID_GEOMETRIC_FACTOR_MIN`]..[`ASTEROID_GEOMETRIC_FACTOR_MAX`]),
/// so a body draws 84 to 144 m across.
const ROCK_RADIUS: Meters = Meters(12.0);

/// Distance between kind columns. Comfortably more than the widest a rock can
/// draw, so no two bodies touch and the physics has nothing to resolve.
const COLUMN_SPACING: f32 = 200.0;

/// Distance between seed rows.
const ROW_SPACING: f32 = 190.0;

/// How high above a rock's centre its label floats, in ENGINE world units - the
/// label rides the subject's `Transform`, which is the one place in this file
/// that is not in meters.
const LABEL_HEIGHT: f32 = 9.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(kinds_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants. No frame-time capture - a
        // posed lineup holds no steady-state load worth measuring, and this
        // example shares a box with whatever else is building.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        // Clean frames at the fleet's known 16:9; dev overlays and the
        // fps/version bar out of shot. The subject labels stay - they name the
        // kind and the seed, which is what the frame is evidence OF.
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        app.add_plugins(kinds_script());
    }

    app.run()
}

fn kinds_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
    app.add_systems(Update, (label_spawned_rocks, compare::position_labels));
}

/// Where the rock in this cell stands, in meters.
fn cell_position(column: usize, row: usize) -> Meters3 {
    let across = (column as f32 - (KINDS.len() as f32 - 1.0) / 2.0) * COLUMN_SPACING;
    let down = ((SEEDS.len() as f32 - 1.0) / 2.0 - row as f32) * ROW_SPACING;
    Meters3::new(across, down, 0.0)
}

/// The scenario id of the rock in this cell. Also its label, so a frame and a
/// log name the same body.
fn cell_id(kind: &str, seed: u32) -> String {
    format!("{kind} seed {seed}")
}

/// Load the stage (skybox + the standard three-point rig) and the grid: one
/// authored asteroid per cell, differing only in its kind id and its seed.
fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>) {
    let mut stage = compare::compare_stage(&game_assets, "asteroid_kinds", "Asteroid Kinds");
    let texture: AssetRef<Image> = game_assets.asteroid_texture.clone().into();

    let rocks = KINDS
        .iter()
        .enumerate()
        .flat_map(|(column, kind)| {
            let texture = texture.clone();
            SEEDS.iter().enumerate().map(move |(row, seed)| {
                let id = cell_id(kind, *seed);
                ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: id.clone(),
                        name: id,
                        position: cell_position(column, row),
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                        radius: ROCK_RADIUS,
                        texture: texture.clone(),
                        // THE variable. Everything else in this config is the
                        // same in all fifteen cells.
                        material: (*kind).to_string(),
                        destroy_sound: None,
                        mass: None,
                        invulnerable: false,
                        lock_signature: None,
                        seed: Some(*seed),
                    }),
                }
            })
        })
        .collect::<Vec<_>>();

    for event in &mut stage.events {
        event.actions.extend(
            rocks
                .iter()
                .cloned()
                .map(EventActionConfig::SpawnScenarioObject),
        );
    }

    commands.trigger(LoadScenario(stage));
    compare::spawn_focus_readout(&mut commands);
}

/// Hang a label over every rock the scenario spawns, naming what the body is
/// and which seed drew it.
///
/// Keyed on the KIND tag rather than on the marker, because the tag is what the
/// label is about - and reading it back off the live entity is what proves the
/// authored id reached the spawned rock rather than being dropped on the way.
fn label_spawned_rocks(
    mut commands: Commands,
    q_spawned: Query<(Entity, &AsteroidKind, &AsteroidSeed), Added<AsteroidKind>>,
) {
    for (entity, kind, seed) in &q_spawned {
        compare::spawn_subject_label(
            &mut commands,
            entity,
            &cell_id(kind, **seed),
            Vec3::Y * LABEL_HEIGHT,
        );
    }
}

/// The whole grid, framed as close as all fifteen still fit.
///
/// The frame is 16:9 and the camera's vertical field of view is 45 degrees, so
/// three rows 190 m apart fix the distance and the five columns then fit
/// horizontally with room to spare. Standing further back only makes the
/// subjects smaller.
#[cfg(feature = "debug")]
const GRID_VIEW: (Meters3, Meters3) = (Meters3::new(0.0, 0.0, 760.0), Meters3::new(0.0, 0.0, 0.0));

/// The closeups, in shot order: where to stand, what to look at, and the file.
///
/// Each answers one question. The control beside ordinary stone, and then each
/// of them alone and filling the frame, is the before and after of both faults
/// at once - the repeat and the flatness. Metal beside ice is whether two kinds
/// read as two materials rather than as two tints.
#[cfg(feature = "debug")]
const CLOSEUPS: [(Meters3, Meters3, &str); 4] = [
    (
        Meters3::new(-300.0, 0.0, 430.0),
        Meters3::new(-300.0, 0.0, 0.0),
        "asteroid-kinds-control-vs-rock.png",
    ),
    (
        Meters3::new(100.0, 0.0, 430.0),
        Meters3::new(100.0, 0.0, 0.0),
        "asteroid-kinds-metal-vs-ice.png",
    ),
    // Close enough that one body fills the frame, which is the only distance
    // at which a tile repeat can be judged at all.
    (
        Meters3::new(-400.0, 0.0, 260.0),
        Meters3::new(-400.0, 0.0, 0.0),
        "asteroid-kinds-control-closeup.png",
    ),
    (
        Meters3::new(-200.0, 0.0, 260.0),
        Meters3::new(-200.0, 0.0, 0.0),
        "asteroid-kinds-rock-closeup.png",
    ),
];

/// The rock the carve step cuts: ordinary stone on the middle seed, which is the
/// body the `rock-closeup` shot already framed from the same place. Two frames
/// of ONE body, before and after its mesh was rebuilt, is what makes them
/// evidence rather than two pictures of two rocks.
#[cfg(feature = "debug")]
const CARVE_SUBJECT: (&str, u32) = (KIND_ROCK, SEEDS[1]);

/// Crater radius, as a fraction of the radius the rock is DRAWN at.
#[cfg(feature = "debug")]
const CUT_RADIUS: f32 = 1.0;

/// Where the craters land, in the rock's own UNIT space - the frame
/// [`AsteroidField`] carves in, where the surface stands
/// [`ASTEROID_GEOMETRIC_FACTOR_MIN`] to [`ASTEROID_GEOMETRIC_FACTOR_MAX`] out.
///
/// All five sit 3.6 units from the centre, inside the nearest the surface ever
/// comes, so every crater bites whatever silhouette the seed drew. They face
/// +Z, which is the side the closeup camera stands on.
#[cfg(feature = "debug")]
const CUT_PLACES: [Vec3; 5] = [
    Vec3::new(0.0, 0.0, 3.6),
    Vec3::new(1.6, 0.9, 3.1),
    Vec3::new(-1.5, -1.0, 3.1),
    Vec3::new(0.6, -1.9, 3.0),
    Vec3::new(-0.9, 1.8, 3.0),
];

/// How long the walk waits for the carve path to rebuild the mesh and the
/// collider. The work runs on the compute pool, and there is no public flag for
/// "done", so the wait is generous and the PICTURE is what says it landed.
#[cfg(feature = "debug")]
const CARVE_FRAMES: u32 = SETTLE_FRAMES * 4;

/// The named rock's root, the mesh node that carries its marks, and the radius
/// it is drawn at.
#[cfg(feature = "debug")]
fn carve_target(world: &World, subject: &str) -> Option<(Entity, Entity, f32)> {
    let mut q_nodes = world.try_query_filtered::<(Entity, &ChildOf), With<DamageMarks>>()?;
    let found: Vec<(Entity, Entity)> = q_nodes
        .iter(world)
        .map(|(node, ChildOf(root))| (*root, node))
        .collect();
    found.into_iter().find_map(|(root, node)| {
        let named = world
            .get::<EntityId>(root)
            .is_some_and(|id| id.as_str() == subject);
        let radius = world.get::<AsteroidRadius>(root)?.0;
        named.then_some((root, node, radius))
    })
}

/// Cut craters into the face of one rock, so the closeup can be shot again on a
/// body whose triangles the carve path made rather than the mesher.
///
/// This is the surface's real test. The material is sampled by position in the
/// body's own space and never by a UV, so a remesh must not change what the
/// rock is made of - and only a carved rock can show that.
#[cfg(feature = "debug")]
fn carve_the_rock(world: &mut World) {
    let subject = cell_id(CARVE_SUBJECT.0, CARVE_SUBJECT.1);
    let Some((root, node, radius)) = carve_target(world, &subject) else {
        warn!("asteroid_kinds: no rock named `{subject}` to carve");
        return;
    };
    // A posed lineup, not a physics subject: pin the rock so the cut cannot
    // push it out of a frame that another shot was composed against.
    world.entity_mut(root).insert(RigidBody::Static);
    let centre = world
        .get::<GlobalTransform>(root)
        .map_or(Vec3::ZERO, GlobalTransform::translation);
    let crater = CUT_RADIUS * radius;
    let damage = DAMAGE_PER_UNIT_VOLUME * (2.0 * std::f32::consts::PI / 3.0) * crater.powi(3);
    for place in CUT_PLACES {
        let mut commands = world.commands();
        apply_damage(
            &mut commands,
            node,
            None,
            damage,
            DamageType::Kinetic,
            Some(centre + place * radius),
        );
        world.flush();
    }
}

/// The driven walk: wait for the scene, frame the grid, shoot it, walk the
/// closeups shooting each, then cut the last subject and shoot it again.
#[cfg(feature = "debug")]
fn kinds_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the kinds scene")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("frame the grid")
        .on_enter(|world: &mut World| pose_camera(world, GRID_VIEW.0, GRID_VIEW.1))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the grid")
        .on_enter(|world: &mut World| shoot(world, "asteroid-kinds-grid.png"))
        .until(shot_written("asteroid-kinds-grid.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add();

    for (position, look_at, path) in CLOSEUPS {
        script = script
            .step(format!("frame {path}"))
            .on_enter(move |world: &mut World| pose_camera(world, position, look_at))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step(format!("shoot {path}"))
            .on_enter(move |world: &mut World| shoot(world, path))
            .until(shot_written(path))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
    }

    // The camera is left where the last closeup put it, so this frame and
    // `asteroid-kinds-rock-closeup.png` differ in one thing: the cut.
    script
        .step("carve the stone rock")
        .on_enter(carve_the_rock)
        .until(frames(CARVE_FRAMES))
        .add()
        .step("shoot the carved rock")
        .on_enter(|world: &mut World| shoot(world, "asteroid-kinds-rock-carved.png"))
        .until(shot_written("asteroid-kinds-rock-carved.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
