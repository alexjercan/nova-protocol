//! compare_planets: the planet-texture lineup for art round 2 (task
//! 20260812-100256) - six Screaming Brain Studios equirect surface maps
//! wrapped on UV spheres (bevy's `Sphere` `.uv()` mesh), previewing the
//! recommended planet route before the planet scenario object exists.
//!
//! UV spheres, NOT the asteroid `TriangleMeshBuilder`: its planar per-triangle
//! UVs cannot wrap an equirect map - which is exactly why the planned planet
//! object needs its own mesh, and what this lineup demonstrates.
//!
//! Hand-run (the comparison itself - WASD+mouse free-fly):
//! ```text
//! cargo run --example compare_planets --features debug
//! # keys 1-6 dress the big center sphere; arrows cycle; labels name the row
//! ```
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load, frame, walk the swap steps, exit
//!   clean. This is the path `probe run` takes.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot the grid and two focus
//!   swaps (staged under `NOVA_CAPTURE_DIR`).

#[path = "shared/compare.rs"]
mod compare;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "compare_planets")]
#[command(version = "1.0.0")]
#[command(about = "Side-by-side SBS planet textures on UV spheres", long_about = None)]
struct Cli;

/// Radius of the row spheres.
const ROW_RADIUS: f32 = 3.4;
/// Row placement: high and behind the focus sphere, inside the default camera
/// framing (the loader parks the free-fly camera at (0, 10, 20) facing the
/// origin).
const ROW_Y: f32 = 7.5;
/// Row depth (see [`ROW_Y`]).
const ROW_Z: f32 = -24.0;
/// Row spacing.
const ROW_SPACING: f32 = 12.4;

/// Radius of the focus sphere the keys retexture.
const FOCUS_RADIUS: f32 = 5.0;
/// Where the focus sphere sits (see the asteroid twin).
const FOCUS_POSITION: Vec3 = Vec3::new(0.0, 0.0, -6.0);

/// Longitude sectors of the UV sphere mesh; equirect maps band along
/// latitude, so a coarse sphere shows faceting before it shows seams.
const SPHERE_SECTORS: u32 = 64;
/// Latitude stacks (see [`SPHERE_SECTORS`]).
const SPHERE_STACKS: u32 = 32;

/// Bevy's `.uv()` sphere puts its poles on the Z axis, so an unrotated planet
/// stares at the camera with its polar cap; this stands the poles up on Y
/// (rotation about X mapping +Z to +Y) so the maps band along world latitude.
fn poles_up() -> Quat {
    Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
}

/// The SBS candidates, in key order - the round-2 picks (Barren, Gaseous,
/// Martian, Snowy) plus a second gas giant and a habitable-looking Tundra.
const CANDIDATES: [(&str, &str); 6] = [
    (
        "1 Barren_01 - cratered ochre",
        "art/texture-candidates/sbs-planets/Barren_01.png",
    ),
    (
        "2 Gaseous_01 - teal gas bands",
        "art/texture-candidates/sbs-planets/Gaseous_01.png",
    ),
    (
        "3 Gaseous_08 - blue-green swirls",
        "art/texture-candidates/sbs-planets/Gaseous_08.png",
    ),
    (
        "4 Martian_01 - red speckle",
        "art/texture-candidates/sbs-planets/Martian_01.png",
    ),
    (
        "5 Snowy_01 - ice blue",
        "art/texture-candidates/sbs-planets/Snowy_01.png",
    ),
    (
        "6 Tundra_01 - green-blue continents",
        "art/texture-candidates/sbs-planets/Tundra_01.png",
    ),
];

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(compare_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - a
        // posed lineup holds no steady-state load worth measuring.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
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
/// the lineup of UV spheres.
fn load_scene(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.trigger(LoadScenario(compare::compare_stage(
        &game_assets,
        "compare_planets",
        "Planet Texture Compare",
    )));

    // One unit UV sphere shared by every subject, sized per subject by its
    // Transform scale (uniform, so normals survive).
    let mesh = meshes.add(Sphere::new(1.0).mesh().uv(SPHERE_SECTORS, SPHERE_STACKS));

    let entries: Vec<compare::CompareEntry> = CANDIDATES
        .map(|(label, file)| compare::CompareEntry {
            label: label.to_string(),
            image: compare::load_candidate(&mut images, file),
        })
        .into();

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
                Transform::from_translation(position)
                    .with_rotation(poles_up())
                    .with_scale(Vec3::splat(ROW_RADIUS)),
            ))
            .id();
        compare::spawn_subject_label(
            &mut commands,
            subject,
            &entry.label,
            Vec3::Y * (ROW_RADIUS + 1.5),
        );
    }

    // The focus sphere: its own material, dressed by `apply_selection` on the
    // roster's insert frame and re-dressed on every key press.
    commands.spawn((
        compare::FocusSubject,
        Mesh3d(mesh),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
        Transform::from_translation(FOCUS_POSITION)
            .with_rotation(poles_up())
            .with_scale(Vec3::splat(FOCUS_RADIUS)),
    ));

    compare::spawn_focus_readout(&mut commands);
    commands.insert_resource(compare::CompareRoster::new(entries));
}

/// Seconds a step may sit before the run aborts naming it (llvmpipe headroom).
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 30.0;

/// The roster indices the walk dresses the focus sphere with: one gas giant
/// and the Martian pick, so both map families get a close-up frame.
#[cfg(feature = "debug")]
const FOCUS_SHOTS: [(usize, &str); 2] = [
    (1, "compare-planets-gaseous01.png"),
    (3, "compare-planets-martian01.png"),
];

/// The driven walk: wait for the scene, frame the lineup, shoot the grid,
/// then swap the focus sphere through two picks, shooting each.
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
        // One framing that holds both the focus sphere and the labeled row.
        .step("frame the lineup")
        .on_enter(|world: &mut World| {
            pose_camera(world, Vec3::new(0.0, 6.0, 26.0), Vec3::new(0.0, 4.0, -12.0));
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the grid")
        .on_enter(|world: &mut World| shoot(world, "compare-planets-grid.png"))
        .until(shot_written("compare-planets-grid.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add();

    for (index, path) in FOCUS_SHOTS {
        script = script
            .step(format!("dress the focus sphere with roster entry {index}"))
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
