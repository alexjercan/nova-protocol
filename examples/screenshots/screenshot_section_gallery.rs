//! screenshot_section_gallery: a named grid of SECTION model candidates for
//! the section remodel (task 20260831-083625) - the torpedo bay, the PDC
//! mount, and the hull/controller cores, each row led by what ships TODAY.
//!
//! Everything here is VISUAL ONLY - nothing is a section prototype, carries a
//! socket or spawns a ship. Two kinds of subject:
//!
//! - SHIPPED: the current art, loaded through the asset server exactly as the
//!   game loads it (`assets/base/gltf/*.glb#Scene0` - Blender exports whose
//!   parent chains the manual decoder refuses). The torpedo bay row leads
//!   with its unit-cube placeholder on purpose: that cube is why the row
//!   exists. The controller stand is EMPTY - the shipped controller authors
//!   no render mesh at all.
//! - KEEPERS: the recipe-generated parts the owner picked, promoted into
//!   `assets/base/gltf/` (`scripts/gen-section-parts.py`), in the
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
//!
//! Nameplates are narrowed per shot: a closeup carries its own row's plates
//! alone, and the wide shot keeps the names but drops the notes, which run
//! wider than a column at that distance. A hand-run shows every plate.
//!
//! The railgun row shows the SETTLED lance design (task 20260824-125947,
//! picked at diameter 0.60 of the cell): still a mockup staged under
//! `art/part-candidates/sections/` until the railgun task promotes it into
//! the catalog. The pick rounds that led here live in this branch's history.

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

/// Where the generated keeper glbs live, relative to the crate root. The
/// picked parts promoted out of `art/part-candidates/sections/` into the
/// asset tree; the dropped candidates stay behind as the task record.
const PARTS_DIR: &str = "assets/base/gltf";

/// Where un-promoted mockups stage: the railgun row reads the settled lance
/// straight from the art tree, nothing enters `assets/base` until its task
/// promotes it.
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
    app.init_resource::<Nameplates>();
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
    /// A mockup still staged under `art/part-candidates/sections/`, decoded
    /// and shown exactly like a promoted candidate.
    Mockup { file: &'static str },
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

/// One row of the stand: the subjects, and the slug that names everything
/// downstream of them - the row's shot file and its two walk beats.
struct Row {
    slug: &'static str,
    items: Vec<Item>,
}

/// The gallery, row by row. Every row leads with TODAY so each candidate is
/// judged against the thing it would replace. Third round: only the keepers -
/// dropped candidates left the stand (their recipes and glbs stay on disk as
/// the task record). PICKED promotes as-is; NEW is this round's rework - the
/// hull and controller cells now repeat the same pattern on every face so
/// section rotation never shows.
fn gallery_rows() -> Vec<Row> {
    vec![
        Row {
            slug: "bays",
            items: vec![
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
        },
        Row {
            slug: "pdcs",
            items: vec![
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
        },
        Row {
            slug: "hulls",
            items: vec![
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
        },
        Row {
            slug: "controllers",
            items: vec![
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
        },
        Row {
            slug: "railgun",
            items: vec![Item {
                id: "railgun_lance",
                look: Look::Mockup {
                    file: "railgun_lance.glb",
                },
                note: "SETTLED - 1x1x3 spinal lance, diameter 0.60",
            }],
        },
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

/// A generated-parts root, resolved from the crate root like the sibling
/// galleries do.
fn parts_root(dir: &str) -> PathBuf {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    Path::new(&root).join(dir)
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
    for (row, definition) in rows.iter().enumerate() {
        let items = &definition.items;
        info!(
            "section_gallery: row `{}`: {} subject(s)",
            definition.slug,
            items.len()
        );
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
                    spawn_candidate(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        item,
                        PARTS_DIR,
                        file,
                        pose,
                    );
                }
                Look::Mockup { file } => {
                    spawn_candidate(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        item,
                        CANDIDATES_DIR,
                        file,
                        pose,
                    );
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
            spawn_label(&mut commands, stand + Vec3::NEG_Y * LABEL_DROP, item, row);
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
    dir: &str,
    file: &str,
    pose: Transform,
) {
    let path = parts_root(dir).join(file);
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
    let path = parts_root(PARTS_DIR).join(part.file);
    for primitive in glb::read_glb(&path) {
        parent.spawn((
            Mesh3d(meshes.add(primitive.mesh())),
            MeshMaterial3d(materials.add(primitive.material())),
            Transform::from_translation(part.offset),
        ));
    }
}

/// An item's nameplate, anchored under its stand in world space and tied to
/// the row it belongs to, which is what a closeup narrows the plates by.
#[derive(Component)]
struct SubjectLabel {
    anchor: Vec3,
    row: usize,
}

/// The second line of a nameplate, the one the wide shot drops.
#[derive(Component)]
struct NoteLine;

/// Which nameplates are on. Five rows of plates all land in the one frame,
/// so the driven walk narrows them per shot.
#[cfg_attr(
    not(feature = "debug"),
    expect(dead_code, reason = "only the driven walk narrows the plates")
)]
#[derive(Resource, Default)]
enum Nameplates {
    /// Every plate, name and note: the hand-run, which is free to fly in.
    #[default]
    All,
    /// Names only, every row - the wide shot, where a note runs wider than
    /// the column spacing and overprints its neighbours letter for letter.
    NamesOnly,
    /// One row, name and note: a closeup, which owns its frame.
    Row(usize),
}

impl Nameplates {
    fn shows_row(&self, row: usize) -> bool {
        match self {
            Self::All | Self::NamesOnly => true,
            Self::Row(shown) => *shown == row,
        }
    }

    fn shows_notes(&self) -> bool {
        !matches!(self, Self::NamesOnly)
    }
}

/// The width the label centres its text in, in logical pixels.
const LABEL_WIDTH: f32 = 240.0;

/// One scrimmed line of a nameplate. A back row's label lands on lit hull;
/// the scrim keeps every name legible.
fn label_line(text: &str, size: f32, colour: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(colour),
        Node {
            padding: UiRect::axes(Val::Px(6.0), Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
    )
}

fn spawn_label(commands: &mut Commands, anchor: Vec3, item: &Item, row: usize) {
    commands
        .spawn((
            SubjectLabel { anchor, row },
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
            parent.spawn(label_line(item.id, 15.0, Color::srgb(0.85, 0.9, 0.95)));
            parent.spawn((
                label_line(item.note, 11.0, Color::srgb(0.55, 0.65, 0.7)),
                NoteLine,
            ));
        });
}

/// Project each nameplate under its subject, whatever the camera is doing,
/// and show only the plates the current shot asked for.
fn place_labels(
    plates: Res<Nameplates>,
    camera: Query<(&Camera, &GlobalTransform), With<ScenarioCameraMarker>>,
    mut labels: Query<(&SubjectLabel, &mut Node), Without<NoteLine>>,
    mut notes: Query<&mut Node, With<NoteLine>>,
) {
    for mut note in &mut notes {
        note.display = if plates.shows_notes() {
            Display::Flex
        } else {
            Display::None
        };
    }
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    for (label, mut node) in &mut labels {
        node.display = if plates.shows_row(label.row) {
            Display::Flex
        } else {
            Display::None
        };
        match camera.world_to_viewport(camera_transform, label.anchor) {
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
    let columns = rows.iter().map(|row| row.items.len()).max().unwrap_or(1) as f32 + 1.0;
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

/// Pose the harness camera on the whole stand, on the names alone.
#[cfg(feature = "debug")]
fn frame_stand(world: &mut World) {
    world.insert_resource(Nameplates::NamesOnly);
    pose_camera(world, camera_position(), CAMERA_TARGET);
}

/// Pose the harness camera on one row, backed off to that row's own width,
/// and hand the frame to that row's plates alone.
#[cfg(feature = "debug")]
fn frame_row(world: &mut World, row: usize) {
    world.insert_resource(Nameplates::Row(row));
    let rows = gallery_rows();
    let target = Vec3::new(
        0.0,
        0.0,
        (row as f32 - (rows.len() as f32 - 1.0) * 0.5) * ROW_SPACING,
    );
    let span = (rows[row].items.len() as f32 + 0.5) * COLUMN_SPACING;
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
    for (row, definition) in gallery_rows().into_iter().enumerate() {
        let slug = definition.slug;
        let shot = format!("section-gallery-{slug}.png");
        script = script
            .step(format!("frame the {slug} row"))
            .on_enter(move |world: &mut World| frame_row(world, row))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step(format!("shoot the {slug} row"))
            .until(shot_written(shot.clone()))
            .on_enter(move |world: &mut World| shoot(world, &shot))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
    }
    script
}
