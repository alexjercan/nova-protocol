//! screenshot_thruster_gallery: a named row of thruster LOOKS for the thruster-shell
//! spike (task 20260816-200255) - the current drive, the proposed shell size
//! family, and the CC0 candidate models already imported under
//! `art/part-candidates/`.
//!
//! Three kinds of subject, VISUAL ONLY - nothing here is a section prototype,
//! carries a socket or spawns a ship:
//!
//! - TODAY: the shipped `basic_thruster_section` render, spawned through the
//!   game's own observer (`insert_thruster_section_render`) so the gallery
//!   shows the real thing, not a copy of it - a grey barrel and a red bell,
//!   two primitives, which is exactly why the owner calls it simple.
//! - SHELLS: the proposed size family (1x1x1 up through 5x5x3, plus the flat
//!   and long variants) mocked from the SAME two primitives, one bell per
//!   exhaust cell, so the row reads as "the drive, grown" rather than as new
//!   art. The real parts would be recipe-generated (see THRUSTERS.md in the
//!   task folder); these mocks exist to judge silhouette and scale only.
//! - CANDIDATES: the committed part-candidate `.glb` files, decoded straight
//!   off disk: the selected recipe-generated shell family and rejected shell
//!   studies, in the cross-faction mechanical voice,
//!   `scripts/gen-thruster-shells.py`, task 20260817-013639) and the pack
//!   conversions with an engine or thruster silhouette (Fertile Soil blocks
//!   pack, Kenney cast cuts, Quaternius cuts - all CC0, provenance in
//!   `art/README.md`). Not through the asset server: every candidate glb
//!   comes out of the repo's own writer, so the shared decoder
//!   (`shared/glb.rs`) reads it off disk - see that module for why `art/`
//!   cannot be an asset source here.
//!
//! Hand-run (free-fly with WASD; the roster idles on a slow orbit until the
//! rig is touched):
//! ```text
//! cargo run --example screenshot_thruster_gallery --features debug
//! ```
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load the gallery, frame it, exit clean.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot the full gallery, a close
//!   pass on the size family and one on the proposed shell candidates
//!   (staged under `NOVA_CAPTURE_DIR`).

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use clap::Parser;
// Direct, not through `nova_protocol::nova_debug`: that path only exists under
// the `debug` feature, and `capturing()` gates the idle orbit in EVERY build.
use nova_debug::prelude::capturing;
use nova_protocol::prelude::*;

#[path = "shared/glb.rs"]
mod glb;

#[derive(Parser)]
#[command(name = "screenshot_thruster_gallery")]
#[command(version = "1.0.0")]
#[command(about = "A named row of thruster looks: today, the shell sizes, the candidates. Autopilot-only: a posed row with no controls of its own", long_about = None)]
struct Cli;

/// Centre-to-centre spacing across a row, wide enough for the 5x5 shells plus
/// a margin. The stand lays meshes out at their native cell size, so every
/// figure in this layout is in engine world units.
const COLUMN_SPACING: f32 = 7.5;
/// Centre-to-centre spacing between rows, sized against the longest subject
/// (the 1x1x5 shell) the same way.
const ROW_SPACING: f32 = 9.0;
/// Every subject stands at the same quarter yaw, so the camera reads the
/// flank and the exhaust face at once.
const SUBJECT_YAW: f32 = -0.55;
/// How far under a subject its name hangs, in world units. Clears the 5x5
/// shells' half height.
const LABEL_DROP: f32 = 3.6;
/// The presentation size candidate models are fitted to, in world units:
/// about two cells, so a pack thruster reads beside the two-cell shells.
const CANDIDATE_FIT: f32 = 2.6;

/// Where the part-candidate glbs live, relative to the crate root.
const CANDIDATES_DIR: &str = "art/part-candidates";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(gallery_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring, the shape_bench pattern: run timeline + engine-bound
        // invariants so `probe run` grades this example. No frame-time capture
        // - a posed gallery holds no steady-state load worth measuring.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        app.add_plugins(gallery_script());
    }

    app.run()
}

fn gallery_plugin(app: &mut App) {
    // Armed only for a hand-run: a capture composes its own frame.
    app.insert_resource(IdleOrbit(!capturing()));
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_gallery);
    app.add_systems(
        Update,
        (frame_new_camera, place_labels, stop_orbit_on_input),
    );
    // PostUpdate, after the rig's own write and before the transform
    // propagates: the free-fly rig syncs the camera in PostUpdate, so an
    // Update system ordered against that set is ordered against nothing and
    // loses every frame.
    app.add_systems(
        PostUpdate,
        orbit_idle_camera
            .after(WASDCameraSystems::Sync)
            .before(TransformSystems::Propagate),
    );
}

/// What one gallery item shows.
enum Look {
    /// The shipped drive, rendered by the game's own observer.
    Current,
    /// A proposed shell mock: `cells` across (x, y) and long (z), one bell
    /// per exhaust cell.
    Shell(IVec3),
    /// A committed part-candidate glb: the manifest dir under
    /// [`CANDIDATES_DIR`], the part file inside it, a presentation yaw
    /// for packs whose nozzle does not already point aft (+Z), and the
    /// presentation size the model is fitted to - [`CANDIDATE_FIT`] for the
    /// 1x1 subjects, larger for the large-format shell candidates so the
    /// size ladder stays visible.
    Candidate {
        dir: &'static str,
        file: &'static str,
        yaw: f32,
        fit: f32,
    },
}

/// One named case.
struct Item {
    id: &'static str,
    look: Look,
    /// The second label line: what the viewer should know at a glance.
    note: &'static str,
}

/// The gallery, row by row. Every case is deliberate:
///
/// - the first row is the selected recipe-generated shell family: bell at
///   1x1x1, vector at 3x3x2, and capital at 5x5x3;
/// - the second row holds TODAY next to the small mocked shells, so the
///   "drive, grown" claim is judged against the thing it grows from;
/// - the third row holds the two large mocked shells (5x5x1, 5x5x3);
/// - the fourth row keeps the other proposed shell studies visible;
/// - the fifth row is the Fertile Soil blocks pack's whole propulsion
///   family - purpose-built thruster and hyperdrive blocks;
/// - the sixth row is the engines CUT from the cast ships (Kenney corvette
///   and racer, Quaternius spitfire and striker), the "assets we have
///   imported but not used yet" case from the owner's brief.
fn gallery_rows() -> Vec<(&'static str, Vec<Item>)> {
    vec![
        (
            "selected shells (recipes)",
            vec![
                Item {
                    id: "shell_bell",
                    look: Look::Candidate {
                        dir: "../../assets/base/gltf",
                        file: "shell_bell.glb",
                        yaw: 0.0,
                        fit: 1.0,
                    },
                    note: "1x1x1 basic drive",
                },
                Item {
                    id: "shell_vector",
                    look: Look::Candidate {
                        dir: "../../assets/base/gltf",
                        file: "shell_vector.glb",
                        yaw: 0.0,
                        fit: 3.0,
                    },
                    note: "3x3x2 vectoring drive, one bell",
                },
                Item {
                    id: "shell_capital",
                    look: Look::Candidate {
                        dir: "../../assets/base/gltf",
                        file: "shell_capital.glb",
                        yaw: 0.0,
                        fit: 5.0,
                    },
                    note: "5x5x3 capital drive",
                },
            ],
        ),
        (
            "mocked size studies: small",
            vec![
                Item {
                    id: "basic_thruster (today)",
                    look: Look::Current,
                    note: "shipped render: barrel + bell",
                },
                Item {
                    id: "shell 1x1x1",
                    look: Look::Shell(IVec3::new(1, 1, 1)),
                    note: "proposed shell mock",
                },
                Item {
                    id: "shell 2x2x1",
                    look: Look::Shell(IVec3::new(2, 2, 1)),
                    note: "proposed shell mock",
                },
                Item {
                    id: "shell 3x3x1",
                    look: Look::Shell(IVec3::new(3, 3, 1)),
                    note: "proposed shell mock",
                },
                Item {
                    id: "shell 1x1x5",
                    look: Look::Shell(IVec3::new(1, 1, 5)),
                    note: "long variant mock",
                },
            ],
        ),
        (
            "mocked size studies: large",
            vec![
                Item {
                    id: "shell 5x5x1",
                    look: Look::Shell(IVec3::new(5, 5, 1)),
                    note: "flat variant mock",
                },
                Item {
                    id: "shell 5x5x3",
                    look: Look::Shell(IVec3::new(5, 5, 3)),
                    note: "capital variant mock",
                },
            ],
        ),
        (
            "proposed shell studies",
            vec![
                Item {
                    id: "shell_gimbal",
                    look: Look::Candidate {
                        dir: "shells",
                        file: "shell_gimbal.glb",
                        yaw: 0.0,
                        fit: CANDIDATE_FIT,
                    },
                    note: "bell on trunnion pivots",
                },
                Item {
                    id: "shell_twin",
                    look: Look::Candidate {
                        dir: "shells",
                        file: "shell_twin.glb",
                        yaw: 0.0,
                        fit: CANDIDATE_FIT,
                    },
                    note: "twin bells, shared manifold",
                },
                Item {
                    id: "shell_paddle",
                    look: Look::Candidate {
                        dir: "shells",
                        file: "shell_paddle.glb",
                        yaw: 0.0,
                        fit: CANDIDATE_FIT,
                    },
                    note: "2D thrust paddles on hinges",
                },
                Item {
                    id: "shell_bank",
                    look: Look::Candidate {
                        dir: "shells",
                        file: "shell_bank.glb",
                        yaw: 0.0,
                        fit: 4.4,
                    },
                    note: "3x3x1 nine-bell bank",
                },
            ],
        ),
        (
            "blocks pack (CC0)",
            vec![
                Item {
                    id: "thruster_single_small",
                    look: Look::Candidate {
                        dir: "blocks/propulsion_thruster_single_small",
                        file: "spacestation_propulsion_thruster_single_small.glb",
                        yaw: 0.0,
                        fit: CANDIDATE_FIT,
                    },
                    note: "Fertile Soil blocks",
                },
                Item {
                    id: "thruster_triple_small",
                    look: Look::Candidate {
                        dir: "blocks/propulsion_thruster_triple_small",
                        file: "spacestation_propulsion_thruster_triple_small.glb",
                        yaw: 0.0,
                        fit: CANDIDATE_FIT,
                    },
                    note: "Fertile Soil blocks",
                },
                Item {
                    id: "thruster_triple_large",
                    look: Look::Candidate {
                        dir: "blocks/propulsion_thruster_triple_large",
                        file: "spacestation_propulsion_thruster_triple_large.glb",
                        yaw: 0.0,
                        fit: CANDIDATE_FIT,
                    },
                    note: "Fertile Soil blocks",
                },
                Item {
                    id: "hyperdrive_rearmount",
                    look: Look::Candidate {
                        dir: "blocks/propulsion_hyperdrive_rearmount_medium",
                        file: "spacestation_propulsion_hyperdrive_rearmount_medium.glb",
                        yaw: 0.0,
                        fit: CANDIDATE_FIT,
                    },
                    note: "Fertile Soil blocks",
                },
                Item {
                    id: "hyperdrive_sidemount",
                    look: Look::Candidate {
                        dir: "blocks/propulsion_hyperdrive_sidemount_small",
                        file: "spacestation_propulsion_hyperdrive_sidemount_small.glb",
                        yaw: 0.0,
                        fit: CANDIDATE_FIT,
                    },
                    note: "Fertile Soil blocks",
                },
            ],
        ),
        (
            "cast cuts (CC0)",
            vec![
                Item {
                    id: "cargoa engine",
                    look: Look::Candidate {
                        dir: "cargoa",
                        file: "engine_port.glb",
                        yaw: 0.0,
                        fit: CANDIDATE_FIT,
                    },
                    note: "Kenney corvette cut",
                },
                Item {
                    id: "racer engine",
                    look: Look::Candidate {
                        dir: "racer",
                        file: "engine_port.glb",
                        yaw: 0.0,
                        fit: CANDIDATE_FIT,
                    },
                    note: "Kenney racer cut",
                },
                Item {
                    id: "spitfire engine",
                    look: Look::Candidate {
                        dir: "quaternius/spitfire",
                        file: "engine_upper_port.glb",
                        yaw: 0.0,
                        fit: CANDIDATE_FIT,
                    },
                    note: "Quaternius cut",
                },
                Item {
                    id: "striker nacelle",
                    look: Look::Candidate {
                        dir: "quaternius/striker",
                        file: "nacelle_port.glb",
                        yaw: 0.0,
                        fit: CANDIDATE_FIT,
                    },
                    note: "Quaternius cut",
                },
            ],
        ),
    ]
}

/// Where one item stands: rows along Z, each row centred on its own count.
fn stand_position(row: usize, rows: usize, column: usize, in_row: usize) -> Vec3 {
    Vec3::new(
        (column as f32 - (in_row as f32 - 1.0) * 0.5) * COLUMN_SPACING,
        0.0,
        (row as f32 - (rows as f32 - 1.0) * 0.5) * ROW_SPACING,
    )
}

/// The stage: the game's own sky and the repo's standard three-point rig,
/// with NO ships - every subject is a display entity this example owns.
fn gallery_stage(game_assets: &GameAssets) -> ScenarioConfig {
    ScenarioConfig {
        description: "A named row of thruster looks for the shell spike".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: ThreePointRig::around("photo", Meters3::ZERO, 3.0).actions(),
        }],
        ..ScenarioConfig::new(
            "thruster_gallery".to_string(),
            "Thruster Gallery".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

fn load_gallery(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.trigger(LoadScenario(gallery_stage(&game_assets)));

    // The shells wear the shipped drive's own two colours, so the size row
    // reads as "the drive, grown" rather than as a new look.
    let housing = materials.add(Color::srgb(0.8, 0.8, 0.8));
    let bell = materials.add(Color::srgb(0.9, 0.3, 0.2));

    let rows = gallery_rows();
    let row_count = rows.len();
    for (row, (_, items)) in rows.iter().enumerate() {
        for (column, item) in items.iter().enumerate() {
            let stand = stand_position(row, row_count, column, items.len());
            let pose = Transform::from_translation(stand)
                .with_rotation(Quat::from_rotation_y(SUBJECT_YAW));
            match &item.look {
                Look::Current => {
                    // The real bundle: the game's render observer builds the
                    // barrel, the bell and the (idle) exhaust cone exactly as
                    // it does on a ship. No parent ship, so the impulse and
                    // audio systems simply never match it.
                    commands.spawn((
                        Name::new(item.id),
                        thruster_section(ThrusterSectionConfig::default()),
                        pose,
                        Visibility::default(),
                    ));
                    info!("thruster_gallery: `{}`: shipped render", item.id);
                }
                Look::Shell(cells) => {
                    spawn_shell(&mut commands, &mut meshes, &housing, &bell, *cells, pose);
                    info!(
                        "thruster_gallery: `{}`: shell mock {}x{}x{}",
                        item.id, cells.x, cells.y, cells.z
                    );
                }
                Look::Candidate {
                    dir,
                    file,
                    yaw,
                    fit,
                } => {
                    spawn_candidate(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        item,
                        dir,
                        file,
                        *yaw,
                        *fit,
                        pose,
                    );
                }
            }
            spawn_label(&mut commands, stand + Vec3::NEG_Y * LABEL_DROP, item);
        }
    }
}

/// A shell mock: a housing slab of `cells` with one bell per exhaust cell.
/// The exhaust face is +Z, matching the drive (`exit_normal` says thrust is
/// -Z and the bell opens +Z); the real part would carry its single mating
/// socket on -Z and the proposed side sockets on the four flanks.
fn spawn_shell(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    housing: &Handle<StandardMaterial>,
    bell: &Handle<StandardMaterial>,
    cells: IVec3,
    pose: Transform,
) {
    // The bell sinks this far into the exhaust cell; the housing owns the
    // rest of the length. Mirrors the shipped drive's 0.4 barrel + 0.5 bell
    // split scaled into one cell.
    const BELL_LENGTH: f32 = 0.45;
    let size = cells.as_vec3();
    let mut root = commands.spawn((
        Name::new(format!("shell_{}x{}x{}", cells.x, cells.y, cells.z)),
        pose,
        Visibility::default(),
    ));
    root.with_children(|parent| {
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z - BELL_LENGTH))),
            MeshMaterial3d(housing.clone()),
            Transform::from_translation(Vec3::new(0.0, 0.0, -BELL_LENGTH * 0.5)),
        ));
        let bell_mesh = meshes.add(Cone::new(0.42, BELL_LENGTH));
        for x in 0..cells.x {
            for y in 0..cells.y {
                let centre = Vec3::new(
                    x as f32 - (cells.x as f32 - 1.0) * 0.5,
                    y as f32 - (cells.y as f32 - 1.0) * 0.5,
                    size.z * 0.5 - BELL_LENGTH * 0.55,
                );
                parent.spawn((
                    Mesh3d(bell_mesh.clone()),
                    MeshMaterial3d(bell.clone()),
                    // The same turn the shipped drive uses: the cone's wide
                    // end faces +Z, the bell opens aft.
                    Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                        .with_translation(centre),
                ));
            }
        }
    });
}

/// A part-candidate glb, decoded off disk, recentred on its bounds and fitted
/// to the presentation size, with the native size logged so the label's claim
/// can be checked against the report.
#[expect(
    clippy::too_many_arguments,
    reason = "one system posing the whole gallery"
)]
fn spawn_candidate(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    item: &Item,
    dir: &str,
    file: &str,
    yaw: f32,
    fit_size: f32,
    pose: Transform,
) {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let path: PathBuf = Path::new(&root).join(CANDIDATES_DIR).join(dir).join(file);
    let primitives = glb::read_glb(&path);
    let (centre, size) = glb::bounds(&primitives);
    let fit = fit_size / size.max_element();
    info!(
        "thruster_gallery: `{}`: {dir}/{file}, native {:.2} x {:.2} x {:.2}, fit x{fit:.2}",
        item.id, size.x, size.y, size.z
    );

    commands
        .spawn((
            Name::new(item.id),
            pose.with_rotation(pose.rotation * Quat::from_rotation_y(yaw))
                .with_scale(Vec3::splat(fit)),
            Visibility::default(),
        ))
        .with_children(|parent| {
            for primitive in primitives {
                parent.spawn((
                    Mesh3d(meshes.add(primitive.mesh())),
                    MeshMaterial3d(materials.add(primitive.material())),
                    Transform::from_translation(-centre),
                ));
            }
        });
}

/// An item's nameplate, anchored under its stand in world space.
#[derive(Component)]
struct SubjectLabel(Vec3);

/// The width the label centres its text in, in logical pixels.
const LABEL_WIDTH: f32 = 240.0;

fn spawn_label(commands: &mut Commands, anchor: Vec3, item: &Item) {
    commands
        .spawn((
            SubjectLabel(anchor),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(-1000.0),
                top: Val::Px(-1000.0),
                width: Val::Px(LABEL_WIDTH),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|parent| {
            for (text, size, colour) in [
                (item.id, 15.0, Color::srgb(0.85, 0.9, 0.95)),
                (item.note, 11.0, Color::srgb(0.55, 0.65, 0.7)),
            ] {
                parent.spawn((
                    Text::new(text),
                    TextFont {
                        font_size: FontSize::Px(size),
                        ..default()
                    },
                    TextColor(colour),
                    // A back row's label lands on lit hull; the scrim keeps
                    // every name legible.
                    Node {
                        padding: UiRect::axes(Val::Px(6.0), Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                ));
            }
        });
}

/// Project each nameplate under its subject, whatever the camera is doing.
fn place_labels(
    camera: Query<(&Camera, &GlobalTransform), With<ScenarioCameraMarker>>,
    mut labels: Query<(&SubjectLabel, &mut Node)>,
) {
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    for (label, mut node) in &mut labels {
        match camera.world_to_viewport(camera_transform, label.0) {
            Ok(position) => {
                node.left = Val::Px(position.x - LABEL_WIDTH * 0.5);
                node.top = Val::Px(position.y);
            }
            Err(_) => {
                node.left = Val::Px(-1000.0);
            }
        }
    }
}

/// What the camera aims at: the middle of the stand. Engine world units, the
/// space the stand itself is laid out in.
const CAMERA_TARGET: Vec3 = Vec3::ZERO;

/// Where the camera stands: backed off far enough to hold the fixed grid,
/// high and in front so it reads flank and exhaust at once.
fn camera_position() -> Vec3 {
    let rows = gallery_rows();
    let columns = rows.iter().map(|(_, items)| items.len()).max().unwrap_or(1) as f32 + 1.0;
    let span = (columns * COLUMN_SPACING).max((rows.len() as f32 + 1.0) * ROW_SPACING);
    // Higher and further back than shape_bench's stand: four rows deep, so
    // the nearest row otherwise leaves the bottom of the frame.
    Vec3::new(0.0, span * 0.44, span * 0.76)
}

/// Frame every camera the loader spawns, so the gallery comes up composed
/// instead of on the loader's default perch.
fn frame_new_camera(
    mut q_camera: Query<&mut Transform, (With<ScenarioCameraMarker>, Added<ScenarioCameraMarker>)>,
) {
    for mut transform in &mut q_camera {
        *transform =
            Transform::from_translation(camera_position()).looking_at(CAMERA_TARGET, Vec3::Y);
    }
}

/// Radians per second the idle orbit turns at.
const ORBIT_RATE: f32 = 0.25;

/// How much further out the orbit stands than the composed front-on framing,
/// so the corner subjects stay in frame on the broadside pass.
const ORBIT_STANDOFF: f32 = 1.35;

/// Whether the idle orbit still owns the camera. Cleared the first time the
/// free-fly rig is touched, and never re-armed.
#[derive(Resource, Default)]
struct IdleOrbit(bool);

/// Hand back the camera the moment the free-fly rig is asked for anything.
fn stop_orbit_on_input(mut orbit: ResMut<IdleOrbit>, q_input: Query<&WASDCameraInput>) {
    if !orbit.0 {
        return;
    }
    let touched = q_input
        .iter()
        .any(|input| input.pan != Vec2::ZERO || input.wasd != Vec2::ZERO || input.vertical != 0.0);
    if touched {
        orbit.0 = false;
    }
}

/// Turn the gallery on a slow turntable while nobody is flying. The CAMERA
/// orbits rather than the subjects, which hold the composition the grid
/// exists for. Runs after the free-fly rig writes its transform, because that
/// rig writes every frame and would otherwise win.
fn orbit_idle_camera(
    orbit: Res<IdleOrbit>,
    time: Res<Time>,
    mut q_camera: Query<&mut Transform, With<ScenarioCameraMarker>>,
) {
    if !orbit.0 {
        return;
    }
    let stand = camera_position();
    let radius = Vec2::new(stand.x, stand.z).length() * ORBIT_STANDOFF;
    let angle = time.elapsed_secs() * ORBIT_RATE;
    for mut transform in &mut q_camera {
        *transform = Transform::from_translation(Vec3::new(
            radius * angle.sin(),
            stand.y,
            radius * angle.cos(),
        ))
        .looking_at(CAMERA_TARGET, Vec3::Y);
    }
}

/// Pose the harness camera on the whole stand.
#[cfg(feature = "debug")]
fn frame_stand(world: &mut World) {
    pose_camera(
        world,
        Meters3::from_engine(camera_position()),
        Meters3::from_engine(CAMERA_TARGET),
    );
}

/// Pose the harness camera on the two mocked size rows.
#[cfg(feature = "debug")]
fn frame_sizes(world: &mut World) {
    let rows = gallery_rows().len();
    // The z of the midpoint between row 1 and row 2.
    let target = Vec3::new(0.0, 0.0, (1.5 - (rows as f32 - 1.0) * 0.5) * ROW_SPACING);
    pose_camera(
        world,
        Meters3::from_engine(target + Vec3::new(0.0, 12.0, 21.0)),
        Meters3::from_engine(target),
    );
}

/// Pose the harness camera on the selected first row.
#[cfg(feature = "debug")]
fn frame_shells(world: &mut World) {
    let rows = gallery_rows().len();
    let target = Vec3::new(0.0, 0.0, -(rows as f32 - 1.0) * 0.5 * ROW_SPACING);
    pose_camera(
        world,
        Meters3::from_engine(target + Vec3::new(0.0, 9.0, 18.0)),
        Meters3::from_engine(target),
    );
}

/// The driven walk: load the gallery, frame it, shoot it, then step in on
/// the size family and on the proposed shell candidates.
#[cfg(feature = "debug")]
fn gallery_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the gallery")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("frame the gallery")
        .on_enter(|world: &mut World| frame_stand(world))
        .until(frames(SETTLE_FRAMES * 2))
        .add()
        .step("shoot the gallery")
        .on_enter(|world: &mut World| shoot(world, "thruster-gallery.png"))
        .until(shot_written("thruster-gallery.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("frame the size family")
        .on_enter(|world: &mut World| frame_sizes(world))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the size family")
        .on_enter(|world: &mut World| shoot(world, "thruster-gallery-sizes.png"))
        .until(shot_written("thruster-gallery-sizes.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("frame the shell candidates")
        .on_enter(|world: &mut World| frame_shells(world))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the shell candidates")
        .on_enter(|world: &mut World| shoot(world, "thruster-gallery-shells.png"))
        .until(shot_written("thruster-gallery-shells.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
