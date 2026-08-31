//! screenshot_section_gallery: a named grid of SECTION model candidates for
//! the section remodel (task 20260831-083625) - the torpedo bay, the PDC
//! mount, and the hull/controller cores, each row led by what ships TODAY.
//!
//! Everything here is VISUAL ONLY - nothing is a section prototype, carries a
//! socket or spawns a ship. Two kinds of subject:
//!
//! - SHIPPED: the current art, loaded through the asset server exactly as the
//!   game loads it (`assets/base/gltf/*.glb#Scene0` - Blender exports whose
//!   node transforms the manual decoder must not guess at). The torpedo bay
//!   row leads with its unit-cube placeholder on purpose: that cube is why
//!   the row exists. The controller stand is EMPTY - the shipped controller
//!   authors no render mesh at all.
//! - CANDIDATES: the recipe-generated parts under
//!   `art/part-candidates/sections/` (`scripts/gen-section-parts.py`), in the
//!   cross-faction mechanical voice the thruster shells set, decoded straight
//!   off disk by `shared/glb.rs` and shown at NATIVE size - they are authored
//!   in cell units, so what stands here is what a grid cell gets. PDC
//!   candidates are three-part assemblies (yaw, pitch, barrel) posed at each
//!   candidate's own joint offsets, at unit-turret scale like the art is
//!   drawn; the shipped mount then scales the whole tree to its 0.5 box.
//!
//! Hand-run (free-fly with WASD; the roster idles on a slow orbit until the
//! rig is touched):
//! ```text
//! cargo run --example screenshot_section_gallery --features debug
//! ```
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load the gallery, frame it, exit clean.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot the full grid and one
//!   close pass per row (staged under `NOVA_CAPTURE_DIR`).

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
#[command(name = "screenshot_section_gallery")]
#[command(version = "1.0.0")]
#[command(about = "A named grid of section model candidates: bays, PDC mounts, hull and controller cores, each row led by today's art", long_about = None)]
struct Cli;

/// Centre-to-centre spacing across a row, wide enough for the 2x1x3 twin bay
/// at a quarter yaw plus a margin.
const COLUMN_SPACING: f32 = 4.5;
/// Centre-to-centre spacing between rows, sized against the 3-cell bays the
/// same way.
const ROW_SPACING: f32 = 5.5;
/// Every subject stands at the same quarter yaw, TURNED AROUND: the working
/// face of every section here is -Z (bay muzzles, barrel lines, the panels
/// the cores wear), and the camera stands at +Z, so a half turn plus the
/// quarter angle is what reads the muzzle face and a flank at once.
const SUBJECT_YAW: f32 = std::f32::consts::PI - 0.55;
/// How far under a subject its name hangs, in world units. Clears the 2x2
/// VLS block's half height.
const LABEL_DROP: f32 = 2.0;

/// Where the candidate glbs live, relative to the crate root.
const CANDIDATES_DIR: &str = "art/part-candidates/sections";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(gallery_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring, the thruster_gallery pattern: run timeline + engine-
        // bound invariants so `probe run` grades this example. No frame-time
        // capture - a posed gallery holds no steady-state load worth measuring.
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

/// One piece of a turret assembly: a candidate part glb and the cumulative
/// offset its joint origin stands at, in unit-turret space.
struct AssemblyPart {
    file: &'static str,
    offset: Vec3,
}

/// The shipped turret's cumulative joint offsets at unit scale, read off
/// `turret_joint_tree` (mount 0.5, scale 1): base plants at -0.5, yaw rides
/// 0.1 above it, then the authored pitch and barrel steps.
const SHIPPED_YAW_AT: Vec3 = Vec3::new(0.0, -0.4, 0.0);
const SHIPPED_PITCH_AT: Vec3 = Vec3::new(0.0, -0.067_294, 0.303_954);
const SHIPPED_BARREL_AT: Vec3 = Vec3::new(0.0, 0.061_143, 0.193_225);

/// What one gallery item shows.
enum Look {
    /// Shipped art, through the asset server: the game's own file, node
    /// transforms and all.
    Shipped { path: &'static str },
    /// A recipe-generated candidate glb, decoded off disk and shown at
    /// native (cell-unit) size.
    Candidate { file: &'static str },
    /// The shipped mount: three Blender glbs posed at the joint tree's own
    /// cumulative offsets.
    ShippedTurret,
    /// A candidate mount: three generated part glbs posed at the candidate's
    /// own joint offsets.
    Assembly { parts: [AssemblyPart; 3] },
    /// No art at all - the empty stand is the honest render of what ships.
    Nothing,
}

/// One named case.
struct Item {
    id: &'static str,
    look: Look,
    /// The second label line: what the viewer should know at a glance.
    note: &'static str,
}

/// The gallery, row by row. Every row leads with TODAY so each candidate is
/// judged against the thing it would replace. Third round: only the keepers -
/// dropped candidates left the stand (their recipes and glbs stay on disk as
/// the task record). PICKED promotes as-is; NEW is this round's rework - the
/// hull and controller cells now repeat the same pattern on every face so
/// section rotation never shows.
fn gallery_rows() -> Vec<(&'static str, Vec<Item>)> {
    vec![
        (
            "torpedo bays",
            vec![
                Item {
                    id: "torpedo_bay (today)",
                    look: Look::Shipped {
                        path: "base/gltf/torpedo-bay-01.glb#Scene0",
                    },
                    note: "shipped render: the unit cube",
                },
                Item {
                    id: "bay_tube",
                    look: Look::Candidate {
                        file: "bay_tube.glb",
                    },
                    note: "PICKED - the bay, 1x1x2, flush end plates",
                },
            ],
        ),
        (
            "pdc mounts (unit scale; ships at 0.5)",
            vec![
                Item {
                    id: "pdc turret (today)",
                    look: Look::ShippedTurret,
                    note: "shipped yaw + pitch + barrel",
                },
                Item {
                    id: "pdc_gatling",
                    look: Look::Assembly {
                        parts: [
                            AssemblyPart {
                                file: "pdc_gatling_yaw.glb",
                                offset: Vec3::new(0.0, -0.4, 0.0),
                            },
                            AssemblyPart {
                                file: "pdc_gatling_pitch.glb",
                                offset: Vec3::new(0.0, 0.0, 0.0),
                            },
                            AssemblyPart {
                                file: "pdc_gatling_barrel.glb",
                                offset: Vec3::new(0.0, 0.02, -0.1),
                            },
                        ],
                    },
                    note: "PICKED - the default pdc",
                },
                Item {
                    id: "pdc_twin",
                    look: Look::Assembly {
                        parts: [
                            AssemblyPart {
                                file: "pdc_twin_yaw.glb",
                                offset: Vec3::new(0.0, -0.4, 0.0),
                            },
                            AssemblyPart {
                                file: "pdc_twin_pitch.glb",
                                offset: Vec3::new(0.0, 0.05, 0.0),
                            },
                            AssemblyPart {
                                file: "pdc_twin_barrel.glb",
                                offset: Vec3::new(0.0, 0.05, -0.2),
                            },
                        ],
                    },
                    note: "PICKED - second pdc, per-muzzle fire rate",
                },
            ],
        ),
        (
            "hull cores",
            vec![
                Item {
                    id: "hull (today)",
                    look: Look::Shipped {
                        path: "base/gltf/hull-01.glb#Scene0",
                    },
                    note: "shipped render",
                },
                Item {
                    id: "hull_personnel",
                    look: Look::Candidate {
                        file: "hull_personnel.glb",
                    },
                    note: "NEW - default hull: crew hatch on every face",
                },
                Item {
                    id: "hull_cargo",
                    look: Look::Candidate {
                        file: "hull_cargo.glb",
                    },
                    note: "NEW - cargo variant: caged bags, every face alike",
                },
                Item {
                    id: "hull_tank",
                    look: Look::Candidate {
                        file: "hull_tank.glb",
                    },
                    note: "PICKED - tank variant",
                },
            ],
        ),
        (
            "controller cores",
            vec![
                Item {
                    id: "controller (today)",
                    look: Look::Nothing,
                    note: "shipped render: no mesh at all",
                },
                Item {
                    id: "core_wires",
                    look: Look::Candidate {
                        file: "core_wires.glb",
                    },
                    note: "NEW - cable-wrapped computer, every face alike",
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
        description: "A named grid of section model candidates".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: ThreePointRig::around("photo", Vec3::ZERO, 3.0).actions(),
        }],
        ..ScenarioConfig::new(
            "section_gallery".to_string(),
            "Section Gallery".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The candidates root, resolved from the crate root like the sibling
/// galleries do.
fn candidates_root() -> PathBuf {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    Path::new(&root).join(CANDIDATES_DIR)
}

fn load_gallery(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.trigger(LoadScenario(gallery_stage(&game_assets)));

    let rows = gallery_rows();
    let row_count = rows.len();
    for (row, (_, items)) in rows.iter().enumerate() {
        for (column, item) in items.iter().enumerate() {
            let stand = stand_position(row, row_count, column, items.len());
            let pose = Transform::from_translation(stand)
                .with_rotation(Quat::from_rotation_y(SUBJECT_YAW));
            match &item.look {
                Look::Shipped { path } => {
                    // The game's own file through the game's own loader; the
                    // glb's node transforms carry any authored scale.
                    commands.spawn((
                        Name::new(item.id),
                        WorldAssetRoot(asset_server.load(*path)),
                        pose,
                    ));
                    info!("section_gallery: `{}`: shipped {path}", item.id);
                }
                Look::Candidate { file } => {
                    spawn_candidate(&mut commands, &mut meshes, &mut materials, item, file, pose);
                }
                Look::ShippedTurret => {
                    let shipped = [
                        ("base/gltf/turret-yaw-01.glb#Scene0", SHIPPED_YAW_AT),
                        ("base/gltf/turret-pitch-01.glb#Scene0", SHIPPED_PITCH_AT),
                        ("base/gltf/turret-barrel-01.glb#Scene0", SHIPPED_BARREL_AT),
                    ];
                    commands
                        .spawn((Name::new(item.id), pose, Visibility::default()))
                        .with_children(|parent| {
                            for (path, offset) in shipped {
                                parent.spawn((
                                    WorldAssetRoot(asset_server.load(path)),
                                    Transform::from_translation(offset),
                                ));
                            }
                        });
                    info!("section_gallery: `{}`: shipped turret assembly", item.id);
                }
                Look::Assembly { parts } => {
                    commands
                        .spawn((Name::new(item.id), pose, Visibility::default()))
                        .with_children(|parent| {
                            for part in parts {
                                spawn_assembly_part(parent, &mut meshes, &mut materials, part);
                            }
                        });
                    info!("section_gallery: `{}`: candidate assembly", item.id);
                }
                Look::Nothing => {
                    info!("section_gallery: `{}`: nothing to render", item.id);
                }
            }
            spawn_label(&mut commands, stand + Vec3::NEG_Y * LABEL_DROP, item);
        }
    }
}

/// A candidate part glb at native size: authored centred on its cell box, so
/// no recentring and no fit - the size it stands at is the size it claims.
fn spawn_candidate(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    item: &Item,
    file: &str,
    pose: Transform,
) {
    let path = candidates_root().join(file);
    let primitives = glb::read_glb(&path);
    let (_, size) = glb::bounds(&primitives);
    info!(
        "section_gallery: `{}`: {file}, native {:.2} x {:.2} x {:.2}",
        item.id, size.x, size.y, size.z
    );
    commands
        .spawn((Name::new(item.id), pose, Visibility::default()))
        .with_children(|parent| {
            for primitive in primitives {
                parent.spawn((
                    Mesh3d(meshes.add(primitive.mesh())),
                    MeshMaterial3d(materials.add(primitive.material())),
                ));
            }
        });
}

/// One turret part at its candidate's joint offset. Turret parts are authored
/// around their own joint origins, so the offset is the pose - recentring
/// would undo the authoring.
fn spawn_assembly_part(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    part: &AssemblyPart,
) {
    let path = candidates_root().join(part.file);
    for primitive in glb::read_glb(&path) {
        parent.spawn((
            Mesh3d(meshes.add(primitive.mesh())),
            MeshMaterial3d(materials.add(primitive.material())),
            Transform::from_translation(part.offset),
        ));
    }
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

/// What the camera aims at: the middle of the stand.
const CAMERA_TARGET: Vec3 = Vec3::ZERO;

/// Where the camera stands: backed off far enough to hold the fixed grid,
/// high and in front so it reads flank and muzzle at once.
fn camera_position() -> Vec3 {
    let rows = gallery_rows();
    let columns = rows.iter().map(|(_, items)| items.len()).max().unwrap_or(1) as f32 + 1.0;
    let span = (columns * COLUMN_SPACING).max((rows.len() as f32 + 1.0) * ROW_SPACING);
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

/// Seconds a step may sit before the run aborts naming it (llvmpipe headroom).
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 30.0;

/// Pose the harness camera on the whole stand.
#[cfg(feature = "debug")]
fn frame_stand(world: &mut World) {
    pose_camera(world, camera_position(), CAMERA_TARGET);
}

/// Pose the harness camera on one row, backed off to that row's own width.
#[cfg(feature = "debug")]
fn frame_row(world: &mut World, row: usize) {
    let rows = gallery_rows();
    let target = Vec3::new(
        0.0,
        0.0,
        (row as f32 - (rows.len() as f32 - 1.0) * 0.5) * ROW_SPACING,
    );
    let span = (rows[row].1.len() as f32 + 0.5) * COLUMN_SPACING;
    pose_camera(
        world,
        target + Vec3::new(0.0, span * 0.38, span * 0.62),
        target,
    );
}

/// The driven walk: load the gallery, frame it, shoot it, then step in on
/// each row in turn.
#[cfg(feature = "debug")]
fn gallery_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
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
        .on_enter(|world: &mut World| shoot(world, "section-gallery.png"))
        .until(shot_written("section-gallery.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add();
    for (row, name) in ["bays", "pdcs", "hulls", "controllers"]
        .into_iter()
        .enumerate()
    {
        let shot = format!("section-gallery-{name}.png");
        script = script
            .step("frame a row")
            .on_enter(move |world: &mut World| frame_row(world, row))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("shoot the row")
            .until(shot_written(shot.clone()))
            .on_enter(move |world: &mut World| shoot(world, &shot))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
    }
    script
}
