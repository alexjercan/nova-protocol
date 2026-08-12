//! compare_asteroids: the asteroid-texture lineup for art round 2 (task
//! 20260812-100256) - the shipped `asteroid.png` next to the ambientCG and
//! Poly Haven candidates, every subject the SAME fixed-seed rock from the
//! game's real mesh pipeline (`TriangleMeshBuilder` + `PlanetHeight`), so the
//! only thing that differs between subjects is the texture.
//!
//! The mesh's UVs are planar PER TRIANGLE, so macro features seam at every
//! triangle edge and stretched triangles push UVs past 1.0 - candidates load
//! with a repeat sampler (they tile), while the baseline keeps the shipped
//! ClampToEdge default so the smear it causes stays visible for contrast.
//!
//! Hand-run (the comparison itself - WASD+mouse free-fly):
//! ```text
//! cargo run --example compare_asteroids --features debug
//! # keys 1-6 dress the big center rock; arrows cycle; labels name the row
//! ```
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load, frame, walk the swap steps, exit
//!   clean. This is the path `probe run` takes.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot the grid and two focus
//!   swaps (staged under `NOVA_SHOT_DIR`).

#[path = "shared/compare.rs"]
mod compare;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "compare_asteroids")]
#[command(version = "1.0.0")]
#[command(about = "Side-by-side asteroid texture comparison on the real rock mesh", long_about = None)]
struct Cli;

/// One noise seed for every subject: the comparison varies the texture only.
const ROCK_SEED: u32 = 20260812;

/// Nominal radius of the row rocks. The noise reaches 3.7-5.6x past the unit
/// sphere (`ASTEROID_GEOMETRIC_FACTOR_*`), so 0.9 draws roughly 7 units wide.
const ROW_RADIUS: f32 = 0.9;
/// Row placement: high and behind the focus rock, inside the default camera
/// framing (the loader parks the free-fly camera at (0, 10, 20) facing the
/// origin).
const ROW_Y: f32 = 7.5;
/// Row depth (see [`ROW_Y`]).
const ROW_Z: f32 = -24.0;
/// Row spacing, sized to the rock's real ~7 unit footprint plus label room.
const ROW_SPACING: f32 = 12.0;

/// Nominal radius of the focus rock the keys retexture.
const FOCUS_RADIUS: f32 = 1.2;
/// Where the focus rock sits: ahead of the origin so the default camera has
/// it centered with air around it.
const FOCUS_POSITION: Vec3 = Vec3::new(0.0, 0.0, -6.0);

/// The disk-loaded candidates, in key order after the baseline; labels carry
/// the key number and the round-2 verdict so a frame reads on its own.
const CANDIDATES: [(&str, &str); 5] = [
    (
        "2 Rock030 (ambientCG) - pick",
        "art/texture-candidates/ambientcg/Rock030.png",
    ),
    (
        "3 Rock035 (ambientCG) - dark pick",
        "art/texture-candidates/ambientcg/Rock035.png",
    ),
    (
        "4 dark_rock_02 (Poly Haven) - backup",
        "art/texture-candidates/polyhaven/dark_rock_02.png",
    ),
    (
        "5 Rock062 (ambientCG) - rejected: orange veins",
        "art/texture-candidates/ambientcg/Rock062.png",
    ),
    (
        "6 Rock048 (ambientCG) - rejected: moss",
        "art/texture-candidates/ambientcg/Rock048.png",
    ),
];

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(compare_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PERF_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - a
        // posed lineup holds no steady-state load worth measuring.
        app.add_plugins(nova_probe::nova_timeline());
        app.add_plugins(nova_probe::nova_invariants());
        // Clean frames at the fleet's known 16:9; dev overlays and the
        // fps/version bar out of shot (the subject labels are NOT HUD-tier,
        // so they stay - they are the point of the frame).
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        app.add_plugins(compare_script());
    }

    app.run()
}

fn compare_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
    app.add_systems(
        Update,
        (
            (compare::select_by_keys, compare::apply_selection)
                .chain()
                .run_if(resource_exists::<compare::CompareRoster>),
            compare::position_labels,
        ),
    );
}

/// Build the roster, load the stage scenario (skybox + light rig), and spawn
/// the lineup: one fixed-seed rock mesh shared by every subject.
fn load_scene(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.trigger(LoadScenario(compare::compare_stage(
        &game_assets,
        "compare_asteroids",
        "Asteroid Texture Compare",
    )));

    // The exact mesh pipeline `insert_asteroid_collider` runs, at a pinned
    // seed so every subject is the same rock.
    let planet = PlanetHeight::default().with_seed(ROCK_SEED);
    let mesh = meshes.add(
        TriangleMeshBuilder::new_octahedron(3)
            .apply_noise(&planet)
            .build(),
    );

    // Baseline first: the shipped texture through its shipped (clamp) sampler.
    let mut entries = vec![compare::CompareEntry {
        label: "1 baseline asteroid.png (clamp sampler)".to_string(),
        image: game_assets.asteroid_texture.clone(),
    }];
    entries.extend(CANDIDATES.map(|(label, file)| compare::CompareEntry {
        label: label.to_string(),
        image: compare::load_candidate(&mut images, file),
    }));

    // The labeled row: one rock per roster entry, the game's material shape
    // (`StandardMaterial` with only `base_color_texture` set).
    let offset = (entries.len() as f32 - 1.0) / 2.0;
    for (index, entry) in entries.iter().enumerate() {
        let position = Vec3::new((index as f32 - offset) * ROW_SPACING, ROW_Y, ROW_Z);
        let subject = commands
            .spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color_texture: Some(entry.image.clone()),
                    ..default()
                })),
                Transform::from_translation(position).with_scale(Vec3::splat(ROW_RADIUS)),
            ))
            .id();
        compare::spawn_subject_label(
            &mut commands,
            subject,
            &entry.label,
            Vec3::Y * ROW_RADIUS * 6.5,
        );
    }

    // The focus rock: its own material, dressed by `apply_selection` on the
    // roster's insert frame and re-dressed on every key press.
    commands.spawn((
        compare::FocusSubject,
        Mesh3d(mesh),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
        Transform::from_translation(FOCUS_POSITION).with_scale(Vec3::splat(FOCUS_RADIUS)),
    ));

    compare::spawn_focus_readout(&mut commands);
    commands.insert_resource(compare::CompareRoster::new(entries));
}

/// Seconds a step may sit before the run aborts naming it (llvmpipe headroom).
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 30.0;

/// The roster indices the walk dresses the focus rock with: the two picks
/// (Rock030, Rock035), so the smoke path exercises the swap and the capture
/// path shoots each pick worn by the big rock.
#[cfg(feature = "debug")]
const FOCUS_SHOTS: [(usize, &str); 2] = [
    (1, "compare-asteroids-rock030.png"),
    (2, "compare-asteroids-rock035.png"),
];

/// The driven walk: wait for the scene, frame the lineup, shoot the grid,
/// then swap the focus rock through the picks, shooting each.
#[cfg(feature = "debug")]
fn compare_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the compare scene")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // One framing that holds both the focus rock and the labeled row.
        .step("frame the lineup")
        .on_enter(|world: &mut World| {
            pose_camera(world, Vec3::new(0.0, 6.0, 26.0), Vec3::new(0.0, 4.0, -12.0));
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the grid")
        .on_enter(|world: &mut World| shoot(world, "compare-asteroids-grid.png"))
        .until(shot_written("compare-asteroids-grid.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add();

    for (index, path) in FOCUS_SHOTS {
        script = script
            .step(format!("dress the focus rock with roster entry {index}"))
            .on_enter(move |world: &mut World| {
                world.resource_mut::<compare::CompareRoster>().selected = index;
            })
            .until(frames(SETTLE_FRAMES))
            .add()
            .step(format!("shoot {path}"))
            .on_enter(move |world: &mut World| shoot(world, path))
            .until(shot_written(path))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
    }
    script
}
