//! loop_intake_compare: the accordion cargo intake and canister content
//! tags on one bench - `loop-intake-compare`, for the owner to judge the look
//! of the intake art beside the canister it takes.
//!
//! Everything here is VISUAL ONLY. Nothing carries a socket, holds cargo or
//! spawns a ship. Three kinds of subject:
//!
//! - INTAKE: the promoted 3x2x1 accordion intake
//!   (`assets/base/gltf/intake_accordion_3x2x1.glb`, recipe in
//!   `scripts/section-part-recipes/intake_accordion_3x2x1.json`, built by
//!   `scripts/gen-section-parts.py`). Each leaf is six named slat nodes that
//!   fold into a zig-zag against the post and stay inside the section. This
//!   example drives its own copy of the fold, and it panics if any fold pose
//!   leaves the cell box. Loaded with canisters so an open door shows what
//!   the face is for.
//! - SCALE: the shipped cargo hull cell (`assets/base/gltf/hull_cargo.glb`)
//!   bolted to the left flank of the intake. One cell is one engine unit is
//!   10 m.
//! - CANISTERS: the promoted cuboid canister
//!   (`assets/base/gltf/cargo_canister_cuboid.glb`, an item model built by
//!   the same generator), with content tags that show only inside
//!   [`NEAR_RANGE`] of the lens, which stands in for the ship here. The walk
//!   dollies in on them so the tags appear on camera.
//!
//! Every model is decoded off disk by `shared/glb.rs`, which panics on a
//! missing file. Grey plates name the variants for review. They are not a
//! candidate; the amber-barred tags over the canisters are.
//!
//! Hand-run (free-fly with WASD; every door cycles on its own):
//! ```text
//! cargo run --example loop_intake_compare --features debug
//! ```
//!
//! Two harnessed modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the full walk, recording
//!   nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: record and encode the loop into
//!   `NOVA_CAPTURE_DIR/loop-intake-compare.webm`: the overview with the door
//!   shut, then open, then a door cycle close on the intake, then the dolly
//!   onto the canisters.
//!
//! Capture, then pull one still per second for review:
//! ```text
//! NOVA_CAPTURE_DIR=target/loop-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example loop_intake_compare --features debug
//! ffmpeg -i target/loop-shots/loop-intake-compare.webm -vf fps=1 \
//!   target/loop-shots/intake-%02d.png
//! ```

use std::{f32::consts::PI, path::Path};

use bevy::{prelude::*, transform::helper::TransformHelper, ui::UiSystems};
use clap::Parser;
use nova_protocol::prelude::*;

#[path = "shared/glb.rs"]
mod glb;

#[derive(Parser)]
#[command(name = "loop_intake_compare")]
#[command(version = "1.0.0")]
#[command(about = "The accordion cargo intake and near-only canister content tags, beside shipped hull art", long_about = None)]
struct Cli;

/// The loop this example records - the webm's file stem.
#[cfg(feature = "debug")]
const LOOP_NAME: &str = "loop-intake-compare";

/// The shipped cell the intake is bolted to, relative to the crate root.
/// The cargo hull, because an intake would sit on a cargo spine.
const SHIPPED_CELL: &str = "assets/base/gltf/hull_cargo.glb";

/// The promoted canister, relative to the crate root. Long axis X.
const CANISTER: &str = "assets/base/gltf/cargo_canister_cuboid.glb";

/// The floor every subject stands on: the bottom face of a cell centred at
/// the origin. Engine units, like every figure in this layout.
const FLOOR: f32 = -0.5;

/// Where the canister row stands: in front of the intakes, and behind the
/// lens of either intake close-up.
const CANISTER_ROW_Z: f32 = 6.0;

/// Centre-to-centre spacing along the canister row: a 1-cell canister and
/// enough air that neighbouring tags clear each other in the overview.
const CANISTER_SPACING: f32 = 1.8;

/// How close the lens must be for a near-only tag to show: 100 m, an
/// example-only default. Far enough that a tag appears while its canister
/// still reads as a distinct box at 720p, near enough that the overview and
/// the dolly's far end hold every tag hidden.
const NEAR_RANGE: f32 = 10.0;

/// Seconds one door takes from shut to open.
const DOOR_TRAVEL_SECS: f32 = 1.2;

/// Seconds a hand-run holds the doors open or shut before they move back.
const DOOR_DWELL_SECS: f32 = 1.5;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(bench_plugin).build();

    #[cfg(feature = "debug")]
    {
        // No frame-time capture: a posed bench holds no steady-state load
        // worth measuring.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(intake_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_systems(Update, drive_dolly);
    }

    app.run()
}

fn bench_plugin(app: &mut App) {
    app.init_resource::<Doors>();
    app.insert_resource(BenchPlates(true));
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_bench);
    app.add_systems(Update, (frame_new_camera, move_doors));
    // The HUD indicator slot: after the last camera writer, so a scripted cut
    // projects through the camera that renders the frame, and before UI
    // layout reads the node positions.
    app.add_systems(
        PostUpdate,
        place_screen_labels
            .after(CameraAuthoritySystems::Override)
            .before(UiSystems::Layout),
    );
}

/// One intake on the bench.
struct Intake {
    /// The intake glb, relative to the crate root. Its door face is -Z.
    model: &'static str,
    id: &'static str,
    /// Cell footprint: width, height, depth.
    size: Vec3,
    /// Centre of the intake along the bench.
    x: f32,
}

/// The promoted accordion intake.
fn intakes() -> [Intake; 1] {
    [Intake {
        model: "assets/base/gltf/intake_accordion_3x2x1.glb",
        id: "intake_accordion_3x2x1",
        size: Vec3::new(3.0, 2.0, 1.0),
        x: 0.0,
    }]
}

/// What a content tag reads: the cargo and its mass.
struct Contents {
    cargo: &'static str,
    mass: &'static str,
}

/// The cargo the canister row carries.
const CARGO: [Contents; 3] = [
    Contents {
        cargo: "WATER ICE",
        mass: "40 t",
    },
    Contents {
        cargo: "IRON ORE",
        mass: "25 t",
    },
    Contents {
        cargo: "MED SUPPLIES",
        mass: "6 t",
    },
];

/// The stage: the game's own sky and the repo's standard three-point rig,
/// with NO ships - every subject is a display entity this example owns.
fn bench_stage(game_assets: &GameAssets) -> ScenarioConfig {
    ScenarioConfig {
        description: "Cargo intake and canister tag bench".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: ThreePointRig::around("photo", Meters3::ZERO, 3.0).actions(),
        }],
        ..ScenarioConfig::new(
            "intake_compare".to_string(),
            "Intake Compare".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

fn load_bench(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.trigger(LoadScenario(bench_stage(&game_assets)));

    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let root = Path::new(&root);
    let cell = glb::read_glb(&root.join(SHIPPED_CELL));
    let (_, cell_size) = glb::bounds(&cell);
    info!(
        "intake_compare: shipped cell {SHIPPED_CELL}, native {:.2} x {:.2} x {:.2}",
        cell_size.x, cell_size.y, cell_size.z
    );
    let canister = glb::read_glb(&root.join(CANISTER));
    let (_, canister_size) = glb::bounds(&canister);
    info!(
        "intake_compare: canister {CANISTER}, native {:.2} x {:.2} x {:.2}",
        canister_size.x, canister_size.y, canister_size.z
    );
    let canister: Vec<_> = canister
        .iter()
        .map(|primitive| {
            (
                meshes.add(primitive.mesh()),
                materials.add(primitive.material()),
            )
        })
        .collect();
    let canister_half = canister_size * 0.5;
    let cell_meshes: Vec<_> = cell
        .iter()
        .map(|primitive| {
            (
                meshes.add(primitive.mesh()),
                materials.add(primitive.material()),
            )
        })
        .collect();

    for (index, intake) in intakes().iter().enumerate() {
        spawn_intake(
            &mut commands,
            &mut meshes,
            &mut materials,
            &canister,
            root,
            intake,
        );

        let cell_at = Vec3::new(intake.x - intake.size.x * 0.5 - 0.5, 0.0, 0.0);
        commands
            .spawn((
                Name::new(format!("{} shipped cell", intake.id)),
                Transform::from_translation(cell_at),
                Visibility::default(),
            ))
            .with_children(|parent| {
                for (mesh, material) in &cell_meshes {
                    parent.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())));
                }
            });

        // Over the intake, so it never meets the scale plate under the cell
        // beside it.
        let plate_at = Vec3::new(intake.x, FLOOR + intake.size.y + 0.2, 0.0);
        let plate = spawn_plate(
            &mut commands,
            plate_at,
            HangType::Above,
            intake.id,
            "promoted glb, 6-slat leaves",
        );
        commands.entity(plate).insert(BenchPlate);
        if index == 0 {
            let plate = spawn_plate(
                &mut commands,
                cell_at + Vec3::new(-0.4, FLOOR - 0.25, 0.5),
                HangType::Below,
                "hull_cargo (shipped)",
                "1 cell = 10 m",
            );
            commands.entity(plate).insert(BenchPlate);
        }
    }

    for (index, contents) in CARGO.iter().enumerate() {
        let at = Vec3::new(
            (index as f32 - 1.0) * CANISTER_SPACING,
            FLOOR + canister_half.y,
            CANISTER_ROW_Z,
        );
        commands
            .spawn((
                Name::new(contents.cargo),
                Transform::from_translation(at),
                Visibility::default(),
            ))
            .with_children(|parent| spawn_canister(parent, &canister));
        spawn_tag(
            &mut commands,
            at + Vec3::Y * (canister_half.y + 0.25),
            contents,
        );
    }
    spawn_plate(
        &mut commands,
        Vec3::new(0.0, FLOOR - 0.25, CANISTER_ROW_Z + canister_half.z),
        HangType::Below,
        "tags near",
        "only inside 100 m of the lens",
    );
}

/// The name prefix of every slat node in the intake:
/// `intake_slat_<l|r><index>`, `l` for the leaf on the model's -X post.
const SLAT_PREFIX: &str = "intake_slat_";

/// How far a slat turns at full open. The slats of a leaf stay edge to edge,
/// so the folded pack is the run times the fold's cosine.
const FOLD_DEGREES: f32 = 80.0;

/// Fold poses the cell-box check grades, shut and open included.
const FOLD_CHECKS: usize = 8;

/// An accordion slat, read off its node. Its centre sits `index + 0.5` slat
/// widths from the pocket edge along `toward`, shortened by the fold, and it
/// turns about its long axis (the node's local X) by the fold, with the sign
/// alternating slat to slat, so neighbours share an edge. The whole pack also
/// steps off the pocket by half a slat's turned thickness, or the first slat's
/// corner would sink into the wall.
#[derive(Component, Clone, Copy)]
struct Slat {
    rest: Transform,
    pocket: f32,
    toward: f32,
    index: usize,
    width: f32,
    thickness: f32,
}

impl Slat {
    /// The slat `node` carries, or `None` for the unnamed static body. Panics
    /// on any other name: the intake art and this example disagree.
    fn of(node: &glb::GlbNode, model: &str) -> Option<Self> {
        let name = node.name.as_deref()?;
        let (toward, index) = name
            .strip_prefix(SLAT_PREFIX)
            .and_then(|slat| slat.split_at_checked(1))
            .and_then(|(side, index)| {
                let toward = match side {
                    "l" => 1.0,
                    "r" => -1.0,
                    _ => return None,
                };
                Some((toward, index.parse::<usize>().ok()?))
            })
            .unwrap_or_else(|| {
                panic!("{model}: node {name:?} is not an `{SLAT_PREFIX}<l|r><index>` slat")
            });
        // A slat's width runs along its local Y, the model's X at rest, and
        // its thickness along Z.
        let (_, size) = glb::bounds(&node.primitives);
        let rest = node.rest;
        Some(Self {
            rest,
            pocket: rest.translation.x - toward * (index as f32 + 0.5) * size.y,
            toward,
            index,
            width: size.y,
            thickness: size.z,
        })
    }

    /// The slat's pose at `fold` radians.
    fn pose(&self, fold: f32) -> Transform {
        let sign = if self.index.is_multiple_of(2) {
            1.0
        } else {
            -1.0
        };
        let mut pose = self.rest;
        pose.translation.x = self.pocket
            + self.toward
                * ((self.index as f32 + 0.5) * self.width * fold.cos()
                    + self.thickness * 0.5 * fold.sin());
        pose.rotation = self.rest.rotation * Quat::from_rotation_x(sign * fold);
        pose
    }
}

/// The model-space bounds, as `(low, high)`, of `parts` with every slat at
/// `fold` radians.
fn posed_bounds<'a>(
    parts: impl IntoIterator<Item = &'a (glb::GlbNode, Option<Slat>)>,
    fold: f32,
) -> (Vec3, Vec3) {
    parts
        .into_iter()
        .flat_map(|(node, slat)| {
            let pose = slat.map_or(node.rest, |slat| slat.pose(fold));
            node.primitives.iter().flat_map(move |primitive| {
                primitive
                    .positions
                    .iter()
                    .map(move |position| pose.transform_point(Vec3::from_array(*position)))
            })
        })
        .fold((Vec3::MAX, Vec3::MIN), |(low, high), position| {
            (low.min(position), high.max(position))
        })
}

/// The intake at its stand: its glb node by node, yawed so the door
/// face (-Z in the model) looks down the bench's +Z, and a canister per cell
/// row inside. Panics if any fold pose leaves the cell box: a door that only
/// fits shut does not ship.
fn spawn_intake(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    canister: &[(Handle<Mesh>, Handle<StandardMaterial>)],
    root: &Path,
    intake: &Intake,
) {
    let mut parts: Vec<_> = glb::read_glb_nodes(&root.join(intake.model))
        .into_iter()
        .map(|node| {
            let slat = Slat::of(&node, intake.model);
            (node, slat)
        })
        .collect();
    let slats = parts.iter().filter(|(_, slat)| slat.is_some()).count();
    assert!(slats > 0, "{}: no `{SLAT_PREFIX}*` nodes", intake.model);
    // The lead slat's amber edge deepens its bounds: the whole pack steps off
    // by the bare slat's thickness, or the lead would break its shared edge.
    let thickness = parts
        .iter()
        .filter_map(|(_, slat)| slat.map(|slat| slat.thickness))
        .fold(f32::INFINITY, f32::min);
    for slat in parts.iter_mut().filter_map(|(_, slat)| slat.as_mut()) {
        slat.thickness = thickness;
    }

    let half = intake.size * 0.5 + Vec3::splat(1e-4);
    for step in 0..=FOLD_CHECKS {
        let fold = FOLD_DEGREES.to_radians() * step as f32 / FOLD_CHECKS as f32;
        let (low, high) = posed_bounds(&parts, fold);
        assert!(
            low.cmpge(-half).all() && high.cmple(half).all(),
            "{}: at a {:.0} deg fold the model spans {low}..{high}, outside its {} cell box",
            intake.model,
            fold.to_degrees(),
            intake.size
        );
    }
    // The slats alone: the whole model always fills its box, so only the
    // leaves show the fold.
    for (pose, fold) in [("shut", 0.0), ("open", FOLD_DEGREES.to_radians())] {
        let (low, high) = posed_bounds(parts.iter().filter(|(_, slat)| slat.is_some()), fold);
        info!(
            "intake_compare: {} {pose}: {slats} slats span {low}..{high} in a {} cell box",
            intake.id, intake.size
        );
    }

    let centre = Vec3::new(intake.x, FLOOR + intake.size.y * 0.5, 0.0);
    commands
        .spawn((
            Name::new(intake.id),
            Transform::from_translation(centre).with_rotation(Quat::from_rotation_y(PI)),
            Visibility::default(),
        ))
        .with_children(|parent| {
            for (node, slat) in &parts {
                let mut entity = parent.spawn((node.rest, Visibility::default()));
                if let Some(slat) = slat {
                    entity.insert(*slat);
                }
                entity.with_children(|node_parent| {
                    for primitive in &node.primitives {
                        node_parent.spawn((
                            Mesh3d(meshes.add(primitive.mesh())),
                            MeshMaterial3d(materials.add(primitive.material())),
                        ));
                    }
                });
            }
            // Cargo: one canister per cell row, set back from the folded
            // slats (+Z is the back in the model).
            for row in 0..intake.size.y.round() as usize {
                let y = -intake.size.y * 0.5 + 0.5 + row as f32;
                parent
                    .spawn((Transform::from_xyz(0.0, y, 0.1), Visibility::default()))
                    .with_children(|stack| spawn_canister(stack, canister));
            }
        });
}

/// One canister centred on `parent`, from its decoded meshes.
fn spawn_canister(
    parent: &mut ChildSpawnerCommands,
    canister: &[(Handle<Mesh>, Handle<StandardMaterial>)],
) {
    for (mesh, material) in canister {
        parent.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())));
    }
}

/// Whether the doors cycle on their own (a hand-run) or hold what the walk
/// asks for, and how far open they stand.
#[derive(Resource)]
struct Doors {
    /// 0 is shut, 1 is open, linear in time; the leaves ease it.
    open: f32,
    target: f32,
    cycle: bool,
    /// Seconds the doors have rested at `target`.
    dwell: f32,
}

impl Default for Doors {
    fn default() -> Self {
        Self {
            open: 0.0,
            target: 1.0,
            cycle: true,
            dwell: 0.0,
        }
    }
}

/// Smoothstep on `0..=1`.
fn ease(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Move every door toward its target at the travel rate, and pose every slat
/// to match. All doors share one clock, so a cut always finds them in step.
fn move_doors(
    time: Res<Time>,
    mut doors: ResMut<Doors>,
    mut slats: Query<(&Slat, &mut Transform)>,
) {
    let step = time.delta_secs() / DOOR_TRAVEL_SECS;
    let open = doors.open;
    doors.open = if open < doors.target {
        (open + step).min(doors.target)
    } else {
        (open - step).max(doors.target)
    };
    if doors.open == doors.target {
        doors.dwell += time.delta_secs();
        if doors.cycle && doors.dwell >= DOOR_DWELL_SECS {
            doors.target = 1.0 - doors.target;
            doors.dwell = 0.0;
        }
    } else {
        doors.dwell = 0.0;
    }
    let fold = FOLD_DEGREES.to_radians() * ease(doors.open);
    for (slat, mut transform) in &mut slats {
        *transform = slat.pose(fold);
    }
}

/// Which way a screen label hangs off its world anchor.
#[derive(Clone, Copy)]
enum HangType {
    /// The label's top edge sits on the anchor.
    Below,
    /// The label's bottom edge sits on the anchor.
    Above,
}

/// A UI label pinned to a world point and re-projected every frame, so it
/// keeps one screen size at any distance.
#[derive(Component)]
struct ScreenAnchor {
    world: Vec3,
    width: f32,
    hang: HangType,
    /// Hide the label while the lens is farther than this from `world`.
    /// `None` shows it at any range: the review plates.
    range: Option<f32>,
}

/// A review plate that names an intake or the shipped cell.
#[derive(Component)]
struct BenchPlate;

/// Whether the [`BenchPlate`]s show. The canister dolly drops them: the
/// intakes stand behind the canisters there, and their plates land on the
/// tags under comparison.
#[derive(Resource)]
struct BenchPlates(bool);

/// Width of a review plate, in logical pixels.
const PLATE_WIDTH: f32 = 230.0;
/// Width of a content tag, in logical pixels.
const TAG_WIDTH: f32 = 136.0;

/// A node parked off screen until the first projection places it.
fn anchored_node(width: f32, direction: FlexDirection) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(-1000.0),
        top: Val::Px(-1000.0),
        width: Val::Px(width),
        flex_direction: direction,
        align_items: AlignItems::Center,
        ..default()
    }
}

fn text_line(text: &str, size: f32, colour: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(colour),
    )
}

/// A grey review plate: the variant name and what to look at. The gallery's
/// nameplate.
fn spawn_plate(
    commands: &mut Commands,
    world: Vec3,
    hang: HangType,
    name: &str,
    note: &str,
) -> Entity {
    commands
        .spawn((
            ScreenAnchor {
                world,
                width: PLATE_WIDTH,
                hang,
                range: None,
            },
            anchored_node(PLATE_WIDTH, FlexDirection::Column),
        ))
        .with_children(|plate| {
            for (text, size, colour) in [
                (name, 14.0, Color::srgb(0.85, 0.9, 0.95)),
                (note, 11.0, Color::srgb(0.55, 0.65, 0.7)),
            ] {
                plate.spawn((
                    text_line(text, size, colour),
                    Node {
                        padding: UiRect::axes(Val::Px(6.0), Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                ));
            }
        })
        .id()
}

/// A content tag candidate: an amber bar, then the cargo in the glow colour
/// and its mass under it, on a dark scrim. Shown inside [`NEAR_RANGE`] only.
fn spawn_tag(commands: &mut Commands, world: Vec3, contents: &Contents) {
    let mut node = anchored_node(TAG_WIDTH, FlexDirection::Row);
    node.align_items = AlignItems::Stretch;
    commands
        .spawn((
            ScreenAnchor {
                world,
                width: TAG_WIDTH,
                hang: HangType::Above,
                range: Some(NEAR_RANGE),
            },
            node,
            BackgroundColor(Color::srgba(0.02, 0.03, 0.04, 0.8)),
        ))
        .with_children(|tag| {
            tag.spawn((
                Node {
                    width: Val::Px(4.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.95, 0.62, 0.15)),
            ));
            tag.spawn(Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(Val::Px(7.0), Val::Px(3.0)),
                ..default()
            })
            .with_children(|lines| {
                lines.spawn(text_line(
                    contents.cargo,
                    15.0,
                    Color::srgb(0.45, 0.9, 0.98),
                ));
                lines.spawn(text_line(contents.mass, 12.0, Color::srgb(0.8, 0.82, 0.84)));
            });
        });
}

/// Project every anchored label through the camera that renders this frame.
/// UI layout runs before transform propagation, so the camera's pose comes
/// from [`TransformHelper`] rather than its stale `GlobalTransform`.
fn place_screen_labels(
    bench_plates: Res<BenchPlates>,
    camera: Query<(Entity, &Camera), With<ScenarioCameraMarker>>,
    helper: TransformHelper,
    mut labels: Query<(&ScreenAnchor, &mut Node, Has<BenchPlate>)>,
) {
    let Ok((entity, camera)) = camera.single() else {
        return;
    };
    let Ok(camera_transform) = helper.compute_global_transform(entity) else {
        return;
    };
    let Some(viewport) = camera.logical_viewport_size() else {
        return;
    };
    let lens = camera_transform.translation();
    for (label, mut node, bench_plate) in &mut labels {
        let out_of_range = label
            .range
            .is_some_and(|range| lens.distance(label.world) > range);
        if (bench_plate && !bench_plates.0) || out_of_range {
            node.display = Display::None;
            continue;
        }
        let Ok(position) = camera.world_to_viewport(&camera_transform, label.world) else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = Val::Px(position.x - label.width * 0.5);
        match label.hang {
            HangType::Below => {
                node.top = Val::Px(position.y);
                node.bottom = Val::Auto;
            }
            HangType::Above => {
                node.top = Val::Auto;
                node.bottom = Val::Px(viewport.y - position.y);
            }
        }
    }
}

/// The overview: the whole bench from high and in front. Engine units.
const OVERVIEW_EYE: Vec3 = Vec3::new(0.0, 6.5, 15.0);
const OVERVIEW_LOOK: Vec3 = Vec3::new(0.0, 0.3, 2.6);

/// Frame every camera the loader spawns on the overview, so a hand-run comes
/// up composed instead of on the loader's default perch.
fn frame_new_camera(
    mut q_camera: Query<&mut Transform, (With<ScenarioCameraMarker>, Added<ScenarioCameraMarker>)>,
) {
    for mut transform in &mut q_camera {
        *transform = Transform::from_translation(OVERVIEW_EYE).looking_at(OVERVIEW_LOOK, Vec3::Y);
    }
}

/// The close-ups, as (eye, look) in engine units: the intake a little
/// off-axis so the folded packs read in depth, then both ends of the canister
/// dolly. The far end holds every tag out of range; the near end stands
/// inside [`NEAR_RANGE`] of all three.
#[cfg(feature = "debug")]
const INTAKE_SHOT: (Vec3, Vec3) = (Vec3::new(0.0, 2.3, 4.0), Vec3::new(-0.1, 0.75, 0.2));
#[cfg(feature = "debug")]
const CANISTERS_FAR: (Vec3, Vec3) = (Vec3::new(0.0, 3.2, 16.0), Vec3::new(0.0, -0.2, 6.0));
#[cfg(feature = "debug")]
const CANISTERS_NEAR: (Vec3, Vec3) = (Vec3::new(0.4, 2.0, 10.9), Vec3::new(0.2, -0.1, 6.0));

/// Seconds the canister dolly takes end to end.
#[cfg(feature = "debug")]
const DOLLY_SECS: f32 = 3.0;

#[cfg(feature = "debug")]
fn frame(world: &mut World, (eye, look): (Vec3, Vec3)) {
    pose_camera(world, Meters3::from_engine(eye), Meters3::from_engine(look));
}

/// A scripted camera move between two (eye, look) poses.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct Dolly {
    from: (Vec3, Vec3),
    to: (Vec3, Vec3),
    elapsed: f32,
}

/// Advance the dolly, when one runs, and pose the camera along it.
#[cfg(feature = "debug")]
fn drive_dolly(world: &mut World) {
    let delta = world.resource::<Time>().delta_secs();
    let Some(mut dolly) = world.get_resource_mut::<Dolly>() else {
        return;
    };
    dolly.elapsed = (dolly.elapsed + delta).min(DOLLY_SECS);
    let t = ease(dolly.elapsed / DOLLY_SECS);
    let eye = dolly.from.0.lerp(dolly.to.0, t);
    let look = dolly.from.1.lerp(dolly.to.1, t);
    frame(world, (eye, look));
}

/// Advance once the dolly reaches its far end.
#[cfg(feature = "debug")]
fn dolly_done() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<Dolly>()
            .is_some_and(|dolly| dolly.elapsed >= DOLLY_SECS)
    })
}

/// Take the doors off their hand-run cycle and send them toward `target`.
#[cfg(feature = "debug")]
fn drive_doors(target: f32) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let mut doors = world.resource_mut::<Doors>();
        doors.cycle = false;
        doors.target = target;
    }
}

/// Advance once every door rests at `target`.
#[cfg(feature = "debug")]
fn doors_at(target: f32) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .get_resource::<Doors>()
            .is_some_and(|doors| doors.open == target)
    })
}

/// The driven walk: the overview shut, then open, a door cycle close on the
/// intake, then the dolly onto the canisters.
#[cfg(feature = "debug")]
fn intake_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let travel = DOOR_TRAVEL_SECS * 3.0;
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the bench")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("frame the overview")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            drive_doors(0.0)(world);
            world.resource_mut::<Doors>().open = 0.0;
            frame(world, (OVERVIEW_EYE, OVERVIEW_LOOK));
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("open the intake loop")
        .on_enter(|world| loop_start(world, LOOP_NAME))
        .add()
        .step("hold the shut bench")
        .until(elapsed(1.0))
        .add()
        .step("open the door")
        .on_enter(drive_doors(1.0))
        .until(doors_at(1.0))
        .deadline(travel)
        .add()
        .step("hold the open bench")
        .until(elapsed(0.8))
        .add()
        .step("close on the intake")
        .on_enter(|world: &mut World| frame(world, INTAKE_SHOT))
        .until(elapsed(0.6))
        .add()
        .step("shut the door")
        .on_enter(drive_doors(0.0))
        .until(doors_at(0.0))
        .deadline(travel)
        .add()
        .step("reopen the door")
        .on_enter(drive_doors(1.0))
        .until(doors_at(1.0))
        .deadline(travel)
        .add()
        .step("hold the open intake")
        .until(elapsed(0.8))
        .add()
        .step("dolly onto the canisters")
        .on_enter(|world: &mut World| {
            world.insert_resource(BenchPlates(false));
            frame(world, CANISTERS_FAR);
            world.insert_resource(Dolly {
                from: CANISTERS_FAR,
                to: CANISTERS_NEAR,
                elapsed: 0.0,
            });
        })
        .until(dolly_done())
        .deadline(DOLLY_SECS * 3.0)
        .add()
        .step("hold on the canisters")
        .until(elapsed(1.0))
        .add()
        .step("close the intake loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(120.0)
        .add()
}
