//! screenshot_docking_gallery: the docking-section mesh spike (task
//! `20260915-165338`, phase 1) - every candidate port shown retracted, shown
//! extended, and shown MATED with a copy of itself at three face gaps.
//!
//! Everything here is VISUAL ONLY. There is no `Docking` section kind yet:
//! nothing on this stand carries a socket, a collider or a link point, and
//! nothing spawns a ship. What it does carry is the geometry contract the
//! real section will inherit, so what the owner picks off these frames is
//! what the section will look like:
//!
//! - the candidates are recipe-generated (`scripts/gen-section-parts.py`,
//!   `"frame": "dock"`), decoded straight off disk by `shared/glb.rs` and
//!   shown at NATIVE size - they are authored in build-grid cells, so what
//!   stands here is what a grid cell gets;
//! - the extended pose is the moving `dock_tube` node slid
//!   [`DOCK_EXTENSION`] along the section's own outward axis, composed the
//!   way `SectionAnimationMotion::Translate` composes it at runtime, not a
//!   hand-drawn second model;
//! - the white wire boxes are the 1x1x1 cell each port must fit retracted -
//!   the scale reference, and the thing the extended tube is supposed to
//!   break out of by exactly 0.5.
//!
//! The mated rows are the point of the spike. Travel is FIXED, never fitted
//! to the actual gap, so two ports that each reach 0.5 close a 1.0 gap exactly
//! and overlap at anything less. Two identical mirrored profiles always cross
//! somewhere, so the question is not whether a seam exists but whether it
//! reads as a seal joint or as a mistake - which is what the 1.0, 0.5 and 0.1
//! rows are for. 0.5 is the WORST case, where the two sleeves occupy the same
//! length of tunnel end to end.
//!
//! Hand-run (free-fly with WASD; the stand idles on a slow orbit until the
//! rig is touched):
//! ```text
//! cargo run --example screenshot_docking_gallery --features debug
//! ```
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load the stand, frame it, exit clean.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot the whole stand and one
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
#[command(name = "screenshot_docking_gallery")]
#[command(version = "1.0.0")]
#[command(about = "Docking-port mesh candidates: retracted, extended, and mated at three face gaps", long_about = None)]
struct Cli;

/// Where the generated candidate glbs live, relative to the crate root.
const PARTS_DIR: &str = "art/part-candidates/sections";

/// The moving sleeve's glTF node-name prefix, and the distance it travels
/// along the port's outward axis. Both mirror `gen-section-parts.py`'s
/// `DOCK_NODE_PREFIX` and `DOCK_EXTENSION`, which is what the dock frame
/// grades every candidate against - a port that disagreed about either could
/// not mate with the rest.
const DOCK_NODE_PREFIX: &str = "dock_tube";
const DOCK_EXTENSION: f32 = 0.5;

/// Centre-to-centre spacing across a row. Sized for the widest subject on the
/// stand: a mated pair at the 1.0 gap, which spans two cells plus the gap.
const COLUMN_SPACING: f32 = 4.2;
/// Centre-to-centre spacing between rows. Every subject is one cell deep in Z
/// - the pairs run along X - so this is label clearance, not geometry.
const ROW_SPACING: f32 = 4.6;
/// How far under a subject its name hangs, in world units. Clears the row in
/// front of it in the wide shot, where five rows of plates land in one frame.
const LABEL_DROP: f32 = 2.1;
/// The screen column a row's caption sits in, in logical pixels from the left
/// edge. PINNED rather than projected: a caption anchored in world space
/// spreads with the row's distance, and the nearest row's would then hang off
/// the side of the frame. Only its HEIGHT tracks the row.
const CAPTION_MARGIN: f32 = 48.0;
/// Extra frame width the wide shot leaves beside the stand, in columns, so
/// the pinned caption column has empty sky to print on.
const GUTTER_COLUMNS: f32 = 1.6;

/// The quarter yaw a SINGLE port stands at, turned around so its working face
/// (-Z, the port face) and one flank read at once - the section gallery's
/// pose, so the two stands compare.
const SINGLE_YAW: f32 = std::f32::consts::PI - 0.55;

/// One candidate port.
struct Candidate {
    id: &'static str,
    file: &'static str,
    /// The second label line: what the viewer should know at a glance.
    note: &'static str,
}

fn candidates() -> Vec<Candidate> {
    vec![
        Candidate {
            id: "dock_collar",
            file: "dock_collar.glb",
            note: "plain industrial: proud collar, bolted flange, steel sleeve",
        },
        Candidate {
            id: "dock_bellows",
            file: "dock_bellows.glb",
            note: "heavy hard-dock: armour ring, corner clamps, two-stage sleeve",
        },
        Candidate {
            id: "dock_flush",
            file: "dock_flush.glb",
            note: "sealed hatch: nothing proud of the cell, graphite sleeve",
        },
    ]
}

/// What one row asks of every candidate.
enum Pose {
    /// One port at rest: the silhouette a ship wears when nothing is docked.
    Retracted,
    /// One port with its sleeve run all the way out.
    Extended,
    /// Two extended ports facing each other across `gap`, the distance
    /// between their RETRACTED outer faces. A pair closes exactly at 1.0 and
    /// overlaps below it.
    Mated { gap: f32 },
}

impl Pose {
    /// How far each moving node has travelled in this pose.
    fn travel(&self) -> f32 {
        match self {
            Self::Retracted => 0.0,
            Self::Extended | Self::Mated { .. } => DOCK_EXTENSION,
        }
    }

    /// Where each cell of this pose stands, and the yaw it stands at, in the
    /// subject's own local space. A single port takes the gallery's quarter
    /// yaw; a mated pair lies ACROSS the frame so the tunnel is seen along
    /// its length instead of end-on.
    fn cells(&self) -> Vec<(Vec3, f32)> {
        match self {
            Self::Retracted | Self::Extended => vec![(Vec3::ZERO, SINGLE_YAW)],
            Self::Mated { gap } => {
                // Faces sit half a cell inside each centre, so a `gap` between
                // them puts the centres `gap + 1` apart.
                let half = (gap + 1.0) * 0.5;
                vec![
                    (Vec3::X * half, std::f32::consts::FRAC_PI_2),
                    (Vec3::NEG_X * half, -std::f32::consts::FRAC_PI_2),
                ]
            }
        }
    }
}

/// One row of the stand: the pose every candidate takes, and the slug that
/// names the row's shot.
struct Row {
    slug: &'static str,
    pose: Pose,
    /// The row's own caption, hung on the far left of the row.
    caption: &'static str,
}

fn rows() -> Vec<Row> {
    vec![
        Row {
            slug: "retracted",
            pose: Pose::Retracted,
            caption: "RETRACTED - the 1x1x1 cell (1 unit = 10 m)",
        },
        Row {
            slug: "extended",
            pose: Pose::Extended,
            caption: "EXTENDED - 0.5 out of the cell, inner end anchored",
        },
        Row {
            slug: "mated-10",
            pose: Pose::Mated { gap: 1.0 },
            caption: "MATED, face gap 1.0 - mouths meet, no overlap",
        },
        Row {
            slug: "mated-05",
            pose: Pose::Mated { gap: 0.5 },
            caption: "MATED, face gap 0.5 - worst case, sleeves fully overlap",
        },
        Row {
            slug: "mated-01",
            pose: Pose::Mated { gap: 0.1 },
            caption: "MATED, face gap 0.1 - each mouth deep in the other barrel",
        },
    ]
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(gallery_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring, the section_gallery pattern: run timeline + engine-
        // bound invariants so `probe run` grades this example. No frame-time
        // capture - a posed stand holds no steady-state load worth measuring.
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
        (
            frame_new_camera,
            place_labels,
            show_row_subjects,
            stop_orbit_on_input,
        ),
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

/// Where one subject stands: rows along Z, each row centred on its own count.
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
        description: "Docking-port mesh candidates, retracted and mated".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: ThreePointRig::around("photo", Meters3::ZERO, 3.0).actions(),
        }],
        ..ScenarioConfig::new(
            "docking_gallery".to_string(),
            "Docking Gallery".to_string(),
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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.trigger(LoadScenario(gallery_stage(&game_assets)));

    // One cell-outline mesh and one material for the whole stand: every
    // reference box is the same value, and a distinct asset per box would be
    // extracted, prepared and bound every frame for nothing.
    let outline = meshes.add(cell_outline_mesh());
    let outline_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.75, 0.8, 0.85),
        emissive: LinearRgba::new(0.18, 0.2, 0.22, 1.0),
        unlit: false,
        ..default()
    });

    let rows = rows();
    let row_count = rows.len();
    let candidates = candidates();
    for (row, definition) in rows.iter().enumerate() {
        info!(
            "docking_gallery: row `{}`: {} candidate(s) at travel {:.2}",
            definition.slug,
            candidates.len(),
            definition.pose.travel()
        );
        for (column, candidate) in candidates.iter().enumerate() {
            let stand = stand_position(row, row_count, column, candidates.len());
            let travel = definition.pose.travel();
            for (offset, yaw) in definition.pose.cells() {
                let pose = Transform::from_translation(stand + offset)
                    .with_rotation(Quat::from_rotation_y(yaw));
                spawn_port(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    candidate,
                    travel,
                    pose,
                    row,
                );
                commands.spawn((
                    Name::new("cell reference"),
                    Mesh3d(outline.clone()),
                    MeshMaterial3d(outline_material.clone()),
                    pose,
                    RowMember(row),
                ));
            }
            spawn_label(
                &mut commands,
                stand + Vec3::NEG_Y * LABEL_DROP,
                candidate.id,
                candidate.note,
                row,
                LabelKind::Subject,
            );
        }
        // The row's own caption. Anchored on the row's CENTRE: the anchor
        // only sets the plate's height, since the caption column is pinned.
        let caption =
            stand_position(row, row_count, 0, candidates.len()).with_x(0.0) + Vec3::Y * 0.9;
        spawn_label(
            &mut commands,
            caption,
            definition.caption,
            "",
            row,
            LabelKind::Caption,
        );
    }
}

/// One candidate port at native size, its sleeve slid `travel` along the
/// port's own outward axis.
fn spawn_port(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    candidate: &Candidate,
    travel: f32,
    pose: Transform,
    row: usize,
) {
    let path = parts_root(PARTS_DIR).join(candidate.file);
    // The same composition the runtime track performs: the sleeve's own node
    // frame, displaced along the section's outward axis (-Z).
    let primitives = glb::read_glb_posed(&path, &|name| {
        if name.starts_with(DOCK_NODE_PREFIX) {
            Vec3::NEG_Z * travel
        } else {
            Vec3::ZERO
        }
    });
    let (_, size) = glb::bounds(&primitives);
    info!(
        "docking_gallery: `{}`: {} at travel {travel:.2}, depth {:.2}",
        candidate.id, candidate.file, size.z
    );
    commands
        .spawn((
            Name::new(candidate.id),
            pose,
            Visibility::default(),
            RowMember(row),
        ))
        .with_children(|parent| {
            for primitive in primitives {
                parent.spawn((
                    Mesh3d(meshes.add(primitive.mesh())),
                    MeshMaterial3d(materials.add(primitive.material())),
                ));
            }
        });
}

/// How thick a cell-outline bar is, in world units. Thin enough to read as a
/// drawn line at stand range, thick enough to survive the capture resolution.
const OUTLINE_BAR: f32 = 0.012;

/// The 1x1x1 cell reference: twelve bars on the edges of the cell a retracted
/// port has to fit. One mesh, shared by every box on the stand.
fn cell_outline_mesh() -> Mesh {
    let half = 0.5;
    let mut mesh: Option<Mesh> = None;
    let mut push = |size: Vec3, at: Vec3| {
        let bar = Cuboid::from_size(size).mesh().build().translated_by(at);
        mesh = Some(match mesh.take() {
            Some(mut acc) => {
                acc.merge(&bar).expect("cell outline bars share a layout");
                acc
            }
            None => bar,
        });
    };
    for (a, b) in [(-half, -half), (-half, half), (half, -half), (half, half)] {
        // Four bars along each axis, on the cell's edges.
        push(
            Vec3::new(1.0, OUTLINE_BAR, OUTLINE_BAR),
            Vec3::new(0.0, a, b),
        );
        push(
            Vec3::new(OUTLINE_BAR, 1.0, OUTLINE_BAR),
            Vec3::new(a, 0.0, b),
        );
        push(
            Vec3::new(OUTLINE_BAR, OUTLINE_BAR, 1.0),
            Vec3::new(a, b, 0.0),
        );
    }
    mesh.expect("twelve bars")
}

/// Which row a subject belongs to. A closeup shows one row's geometry and
/// hides the rest: the stand is one flat plane, so the row in FRONT of the
/// target would otherwise fill the bottom of every close frame.
#[derive(Component)]
struct RowMember(usize);

/// Hide every subject the current shot did not ask for.
fn show_row_subjects(plates: Res<Nameplates>, mut subjects: Query<(&RowMember, &mut Visibility)>) {
    if !plates.is_changed() {
        return;
    }
    for (member, mut visibility) in &mut subjects {
        *visibility = if plates.shows_row(member.0) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// An item's nameplate, anchored under its stand in world space and tied to
/// the row it belongs to, which is what a closeup narrows the plates by.
#[derive(Component)]
struct SubjectLabel {
    anchor: Vec3,
    row: usize,
    kind: LabelKind,
}

/// What a plate names. The wide shot carries CAPTIONS alone: five rows of
/// subject plates land in one frame and print over the row in front of them,
/// and the columns hold the same three candidates in the same order in every
/// row, so the names belong on the closeups that have room for them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LabelKind {
    /// One candidate, under its own stand, centred on it.
    Subject,
    /// One row, off the left end of the stand, reading rightward.
    Caption,
}

/// A nameplate's note line. A marker only: it keeps the child text nodes out
/// of the query that positions the plates, which own the absolute layout.
#[derive(Component)]
struct NoteLine;

/// Which nameplates are on. Five rows of plates all land in the one frame, so
/// the driven walk narrows them per shot.
#[cfg_attr(
    not(feature = "debug"),
    expect(dead_code, reason = "only the driven walk narrows the plates")
)]
#[derive(Resource, Default)]
enum Nameplates {
    /// Every plate, name and note: the hand-run, which is free to fly in.
    #[default]
    All,
    /// Row captions only - the wide shot, which holds every row at once.
    CaptionsOnly,
    /// One row, name and note: a closeup, which owns its frame.
    Row(usize),
}

impl Nameplates {
    fn shows_row(&self, row: usize) -> bool {
        match self {
            Self::All | Self::CaptionsOnly => true,
            Self::Row(shown) => *shown == row,
        }
    }

    fn shows(&self, label: &SubjectLabel) -> bool {
        self.shows_row(label.row)
            && (label.kind == LabelKind::Caption || !matches!(self, Self::CaptionsOnly))
    }
}

/// The width the label centres its text in, in logical pixels.
const LABEL_WIDTH: f32 = 260.0;

/// One scrimmed line of a nameplate. A back row's label lands on lit hull; the
/// scrim keeps every name legible.
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

fn spawn_label(
    commands: &mut Commands,
    anchor: Vec3,
    name: &str,
    note: &str,
    row: usize,
    kind: LabelKind,
) {
    let align = match kind {
        LabelKind::Subject => AlignItems::Center,
        LabelKind::Caption => AlignItems::FlexStart,
    };
    commands
        .spawn((
            SubjectLabel { anchor, row, kind },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(-1000.0),
                top: Val::Px(-1000.0),
                width: Val::Px(LABEL_WIDTH),
                flex_direction: FlexDirection::Column,
                align_items: align,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(label_line(name, 15.0, Color::srgb(0.85, 0.9, 0.95)));
            if !note.is_empty() {
                parent.spawn((
                    label_line(note, 11.0, Color::srgb(0.55, 0.65, 0.7)),
                    NoteLine,
                ));
            }
        });
}

/// Project each nameplate under its subject, whatever the camera is doing, and
/// show only the plates the current shot asked for.
fn place_labels(
    plates: Res<Nameplates>,
    camera: Query<(&Camera, &GlobalTransform), With<ScenarioCameraMarker>>,
    mut labels: Query<(&SubjectLabel, &mut Node), Without<NoteLine>>,
) {
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    for (label, mut node) in &mut labels {
        node.display = if plates.shows(label) {
            Display::Flex
        } else {
            Display::None
        };
        match camera.world_to_viewport(camera_transform, label.anchor) {
            Ok(position) => {
                node.left = Val::Px(match label.kind {
                    LabelKind::Subject => position.x - LABEL_WIDTH * 0.5,
                    LabelKind::Caption => CAPTION_MARGIN,
                });
                node.top = Val::Px(position.y);
            }
            Err(_) => {
                node.left = Val::Px(-1000.0);
            }
        }
    }
}

/// What the camera aims at: the middle of the stand.
fn camera_target() -> Vec3 {
    Vec3::ZERO
}

/// Where the camera stands: backed off far enough to hold the fixed grid, high
/// and in front so it reads the port faces and the mated flanks at once. The
/// stand is deeper than it is wide, so the depth sets the standoff.
fn camera_position() -> Vec3 {
    let rows = rows();
    let columns = candidates().len() as f32 + GUTTER_COLUMNS;
    let span = (columns * COLUMN_SPACING).max(rows.len() as f32 * ROW_SPACING);
    // Distance rather than height carries the depth: the stand runs 5 rows
    // deep, and a camera perched over the middle of it drops the NEAREST row
    // out of the bottom of the frame.
    camera_target() + Vec3::new(0.0, span * 0.4, span * 0.85)
}

/// Frame every camera the loader spawns, so the stand comes up composed
/// instead of on the loader's default perch.
fn frame_new_camera(
    mut q_camera: Query<&mut Transform, (With<ScenarioCameraMarker>, Added<ScenarioCameraMarker>)>,
) {
    for mut transform in &mut q_camera {
        *transform =
            Transform::from_translation(camera_position()).looking_at(camera_target(), Vec3::Y);
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

/// Turn the stand on a slow turntable while nobody is flying. The CAMERA
/// orbits rather than the subjects, which hold the composition the grid exists
/// for. Runs after the free-fly rig writes its transform, because that rig
/// writes every frame and would otherwise win.
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
        .looking_at(camera_target(), Vec3::Y);
    }
}

/// Pose the harness camera on the whole stand, on the names alone.
#[cfg(feature = "debug")]
fn frame_stand(world: &mut World) {
    world.insert_resource(Nameplates::CaptionsOnly);
    pose_camera(
        world,
        Meters3::from_engine(camera_position()),
        Meters3::from_engine(camera_target()),
    );
}

/// Pose the harness camera on one row, backed off to that row's own width, and
/// hand the frame to that row's plates alone.
#[cfg(feature = "debug")]
fn frame_row(world: &mut World, row: usize) {
    world.insert_resource(Nameplates::Row(row));
    let rows = rows();
    let target = Vec3::Z * (row as f32 - (rows.len() as f32 - 1.0) * 0.5) * ROW_SPACING;
    let span = (candidates().len() as f32 + 0.4) * COLUMN_SPACING;
    pose_camera(
        world,
        Meters3::from_engine(target + Vec3::new(0.0, span * 0.3, span * 0.58)),
        Meters3::from_engine(target),
    );
}

/// The driven walk: load the stand, frame it, shoot it, then step in on each
/// row in turn.
#[cfg(feature = "debug")]
fn gallery_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the stand")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("frame the stand")
        .on_enter(|world: &mut World| frame_stand(world))
        .until(frames(SETTLE_FRAMES * 2))
        .add()
        .step("shoot the stand")
        .on_enter(|world: &mut World| shoot(world, "docking-gallery.png"))
        .until(shot_written("docking-gallery.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add();
    for (row, definition) in rows().into_iter().enumerate() {
        let slug = definition.slug;
        let shot = format!("docking-gallery-{slug}.png");
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
