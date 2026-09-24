//! world_clusters: fly a streamed world whose bodies come in groups.
//!
//! `world_features` fills a cell from feature spheres that reach it. This
//! example fills it from GROUPS: a planetoid with companion rocks, or a lead
//! derelict hull with companion hulls and debris, decided on a global lattice
//! and owned body by body by the cell each body's centre falls in. A group
//! near a face places bodies on both sides of it, and the two cells agree on
//! every one because each replays the same global decision.
//!
//! What is chosen, and how, reads three continuous environment fields -
//! material density, volatiles and human activity - together. The heatmap in
//! the corner is those fields, painted on a worker; the biome name in the
//! readout is a label for a human and nothing generates from it.
//!
//! The generator is the example-owned `ClusteredWorld` in
//! `examples/shared/world_fixture/clustered.rs`; its inline tests own the
//! seam, identity, order, accounting, cap and geometry claims. What a human
//! judges here is whether groups read as places and whether a group stays
//! whole across a face.
//!
//! Drawn every frame in the world:
//!
//! | marker | what it means |
//! | - | - |
//! | white / magenta sphere | a planetoid / derelict group's parent |
//! | white / magenta line | parent to member |
//! | white / magenta small sphere | a member placed in its parent's cell |
//! | orange sphere and square | a member placed in a cell other than its parent's, and the face its line crosses |
//! | red sphere and cross | a planned body its cell skipped, for the reason the readout counts |
//! | grey sphere | a cell's background rock |
//!
//! Hand-run (WASD + right-drag to look; HOLD a key to cross a 32 km sector):
//! ```text
//! cargo run --example world_clusters --features debug
//! ```
//!
//! Harnessed mode:
//! - `NOVA_AUTOPILOT=1`: stream the home window, check the seam groups live,
//!   frame one planetoid group and one derelict group across a face, exit
//!   clean.
//! - `NOVA_CAPTURE=1`: also writes `world-clusters-planetoid.png` and
//!   `world-clusters-derelict.png`.

#[path = "../shared/world_fixture/mod.rs"]
pub mod world_fixture;

use std::collections::BTreeMap;

use bevy::{
    asset::RenderAssetUsages,
    color::palettes::tailwind,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    tasks::{block_on, poll_once, AsyncComputeTaskPool, Task},
};
use clap::Parser;
use nova_protocol::prelude::*;
use nova_world::prelude::*;
use world_fixture::{
    clustered_world_config, free_play_scenario, group_at, parent_chances, plan_cell,
    world_observer_plugin, BodySource, CellPlan, ClusteredWorld, EnvironmentField,
    EnvironmentFields, GroupKind, Outcome, SkipReason, CLUSTER_HOME, EXAMPLE_ACTIVE_RADIUS,
    GROUP_LATTICE,
};

#[derive(Parser)]
#[command(name = "world_clusters")]
#[command(version = "1.0.0")]
#[command(
    about = "Fly a streamed world of planetoid and derelict groups that cross sector faces",
    long_about = None
)]
struct Cli;

/// The empty bootstrap this session runs inside.
const SCENARIO_ID: &str = "world_clusters_observer";

/// How bright the observer's key light is. The example lights itself because
/// the bootstrap stays empty.
const KEY_ILLUMINANCE: f32 = 6_000.0;

/// Where the key light points from.
const KEY_DIRECTION: Vec3 = Vec3::new(-0.4, -1.0, -0.6);

/// How many texels across the heatmap.
///
/// 320 over [`HEATMAP_EXTENT`] is one sample per kilometre: 32 per sector,
/// so a face is a line and not a guess. 102,400 texels of three fields is
/// too much work for one frame, which is why the paint runs on a worker.
const HEATMAP_PIXELS: u32 = 320;

/// How many pixels across the observer's dot on the heatmap.
const OBSERVER_DOT: f32 = 7.0;

/// How much world the heatmap covers on each axis: ten sectors, the five of
/// the window and two and a half either side of it.
const HEATMAP_EXTENT: Meters = Meters(320_000.0);

/// How big a parent's dot is on the heatmap, in texels.
const HEATMAP_DOT: i32 = 3;

/// How many line segments ring a drawn body.
const MARKER_SEGMENTS: u32 = 24;

/// How big the square that marks a face crossing is.
const SEAM_MARK: Meters = Meters(1_200.0);

/// How much wider than its clearance a parent's ring is drawn, so it reads
/// apart from the body inside it.
const PARENT_RING: f32 = 1.4;

/// In-step seconds a harnessed beat gets before the run aborts naming it. A
/// hang backstop: a 125-cell window meshes every planetoid in it.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 300.0;

/// The environment fields the readout samples at the observer.
///
/// Built once: `Fbm::new` seeds a table per octave, and the readout reads the
/// fields every frame.
#[derive(Resource)]
struct ObserverFields(EnvironmentFields);

/// One live root's plan: every body it owns, placed or skipped, and the
/// groups they belong to.
///
/// Example-owned: a streamed manifest carries placed bodies only, so this
/// asks the generator's plan for the same input once per root as it comes up.
#[derive(Component)]
struct RootPlan(CellPlan);

/// Marks the readout line.
#[derive(Component)]
struct ClusterReadout;

/// Marks the observer's dot over the heatmap.
#[derive(Component)]
struct HeatmapObserver;

/// The heatmap image and the cell it is centred on.
#[derive(Resource)]
struct Heatmap {
    handle: Handle<Image>,
    /// The cell the painted image is centred on, or `None` before the first
    /// paint lands.
    painted: Option<SectorCoord>,
    /// How many parents the painted slice holds, by kind.
    parents: (usize, usize),
}

/// One heatmap being painted on a worker.
#[derive(Resource)]
struct HeatmapJob {
    centre: SectorCoord,
    task: Task<HeatmapPixels>,
}

/// A finished paint.
struct HeatmapPixels {
    /// `HEATMAP_PIXELS` squared RGBA texels, top row first.
    texels: Vec<u8>,
    /// Planetoid and derelict parents drawn.
    parents: (usize, usize),
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new()
        .with_game_plugins((
            clusters_plugin,
            world_observer_plugin,
            NovaWorldPlugin::<ClusteredWorld>::default(),
        ))
        .build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(clusters_script());
    }

    app.run()
}

fn clusters_plugin(app: &mut App) {
    app.insert_resource(ObserverFields(EnvironmentFields::new(
        clustered_world_config().seed,
    )));
    app.add_systems(OnEnter(GameAssetsStates::Loaded), boot_clusters);
    app.add_systems(
        Update,
        (
            park_at_home,
            plan_new_roots.after(NovaWorldSystems::Retire),
            (draw_groups, update_readout, report_census).after(plan_new_roots),
            (request_heatmap, collect_heatmap, place_heatmap_observer)
                .chain()
                .after(NovaWorldSystems::Observe),
        ),
    );
}

/// The colour a group kind is drawn in, on the heatmap and in the world.
/// Neither is a field hue, so a parent dot never reads as a reading.
fn kind_colour(kind: GroupKind) -> Srgba {
    match kind {
        GroupKind::Planetoid => Srgba::WHITE,
        GroupKind::Derelict => tailwind::FUCHSIA_400,
    }
}

/// The hue a field adds to the heatmap. Three primaries-ish, so two fields
/// high at once mix to a hue neither has alone.
fn field_colour(field: EnvironmentField) -> Srgba {
    match field {
        EnvironmentField::MaterialDensity => Srgba::new(1.0, 0.5, 0.1, 1.0),
        EnvironmentField::Volatiles => Srgba::new(0.2, 0.45, 1.0, 1.0),
        EnvironmentField::HumanActivity => Srgba::new(0.35, 0.95, 0.3, 1.0),
    }
}

/// Load the empty bootstrap, arm the clustered stream, light the scene, and
/// put the readout, the heatmap and its legend up.
///
/// `OnEnter`, so the config lands ahead of `NovaWorldSystems::Cleanup`.
fn boot_clusters(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    game_assets: Res<GameAssets>,
) {
    commands.trigger(LoadScenario(free_play_scenario(
        &game_assets,
        SCENARIO_ID,
        "World Clusters Observer",
    )));
    commands.insert_resource(clustered_world_config());

    commands.spawn((
        Name::new("Observer Key Light"),
        DirectionalLight {
            illuminance: KEY_ILLUMINANCE,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_translation(Vec3::ZERO).looking_to(KEY_DIRECTION, Vec3::Y),
    ));

    let text = |size: f32, colour: Color| {
        (
            TextFont {
                font_size: FontSize::Px(size),
                ..default()
            },
            TextColor(colour),
        )
    };

    // Top LEFT: the dev overlay's fps and version bar sits along the top
    // right.
    commands.spawn((
        Name::new("Cluster Readout"),
        ClusterReadout,
        Text::new(""),
        text(17.0, Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));

    let handle = images.add(blank_heatmap());
    commands.insert_resource(Heatmap {
        handle: handle.clone(),
        painted: None,
        parents: (0, 0),
    });
    commands
        .spawn((
            Name::new("Heatmap Panel"),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                left: Val::Px(12.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexEnd,
                column_gap: Val::Px(10.0),
                ..default()
            },
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    Name::new("Heatmap Image"),
                    ImageNode::new(handle),
                    Node {
                        width: Val::Px(HEATMAP_PIXELS as f32),
                        height: Val::Px(HEATMAP_PIXELS as f32),
                        ..default()
                    },
                ))
                .with_children(|image| {
                    image.spawn((
                        Name::new("Heatmap Observer"),
                        HeatmapObserver,
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(OBSERVER_DOT),
                            height: Val::Px(OBSERVER_DOT),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(1.0, 1.0, 0.2)),
                    ));
                });
            panel
                .spawn((
                    Name::new("Heatmap Legend"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(3.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                ))
                .with_children(|legend| {
                    legend.spawn((Text::new("ENVIRONMENT, XZ slice"), text(15.0, Color::WHITE)));
                    for field in EnvironmentField::ALL {
                        legend.spawn((
                            Text::new(format!("## {}", field.label())),
                            text(15.0, field_colour(field).into()),
                        ));
                    }
                    for (swatch, label) in [
                        ("#", "planetoid parent"),
                        ("#", "derelict parent"),
                        ("+", "sector face / window"),
                        ("#", "you"),
                    ] {
                        let colour = match label {
                            "planetoid parent" => kind_colour(GroupKind::Planetoid).into(),
                            "derelict parent" => kind_colour(GroupKind::Derelict).into(),
                            "you" => Color::srgb(1.0, 1.0, 0.2),
                            _ => Color::srgb(0.7, 0.7, 0.75),
                        };
                        legend.spawn((Text::new(format!("{swatch}  {label}")), text(15.0, colour)));
                    }
                    legend.spawn((
                        Text::new("+X right, -Z up, 1 km a texel"),
                        text(13.0, Color::srgb(0.7, 0.7, 0.75)),
                    ));
                });
        });
}

/// A blank image of the heatmap's size, so the panel has something to point
/// at before the first paint lands.
fn blank_heatmap() -> Image {
    let texels = (HEATMAP_PIXELS * HEATMAP_PIXELS) as usize;
    let mut image = Image::new(
        Extent3d {
            width: HEATMAP_PIXELS,
            height: HEATMAP_PIXELS,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0; texels * 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    // Nearest: a texel IS a sample.
    image.sampler = ImageSampler::nearest();
    image
}

/// Put the free-fly observer down in [`CLUSTER_HOME`], once, and leave it
/// flyable.
///
/// The `WASDCamera` is re-inserted WITH the pose: the rig writes the transform
/// from its own position every frame, so a bare transform write is erased.
fn park_at_home(
    mut parked: Local<bool>,
    config: Option<Res<WorldConfig<ClusteredWorld>>>,
    observer: Query<(Entity, &WASDCamera), With<ScenarioCameraMarker>>,
    mut commands: Commands,
) {
    if *parked {
        return;
    }
    let Some(config) = config else {
        return;
    };
    let Ok((entity, rig)) = observer.single() else {
        return;
    };
    // Engine boundary: a cell centre is in meters, a transform in world units.
    let position = CLUSTER_HOME.centre(config.sector_edge).to_engine();
    let look_at = CLUSTER_HOME
        .offset(1, 0, 0)
        .centre(config.sector_edge)
        .to_engine();
    commands.entity(entity).insert((
        Transform::from_translation(position).looking_at(look_at, Vec3::Y),
        *rig,
    ));
    *parked = true;
    info!("world clusters: the observer opens in {CLUSTER_HOME}");
}

/// Plan each root that came up this frame, with the input the generator was
/// given.
///
/// After `Retire`, so a root is planned in the frame it is spawned and never
/// after it is taken back.
///
/// # Panics
///
/// On a [`SectorFault`]. The generator already accepted this input, so a
/// refusal here means the plan and the streamed cell disagree.
fn plan_new_roots(
    mut commands: Commands,
    config: Option<Res<WorldConfig<ClusteredWorld>>>,
    fields: Res<ObserverFields>,
    roots: Query<(Entity, &SectorRoot), Added<SectorRoot>>,
) {
    let Some(config) = config else {
        return;
    };
    for (entity, root) in &roots {
        let plan = plan_cell(&fields.0, config.input(root.0))
            .unwrap_or_else(|fault| panic!("world clusters: {}: {fault}", root.0));
        // A scenario swap can despawn the root on this frame.
        commands.entity(entity).try_insert(RootPlan(plan));
    }
}

/// Draw every live group: parents, parent-to-member lines, owner colours,
/// face crossings and skipped bodies.
fn draw_groups(
    mut gizmos: Gizmos,
    config: Option<Res<WorldConfig<ClusteredWorld>>>,
    roots: Query<&RootPlan>,
) {
    let Some(config) = config else {
        return;
    };
    let edge = config.sector_edge;
    let groups: BTreeMap<_, _> = roots
        .iter()
        .flat_map(|plan| &plan.0.groups)
        .map(|group| (group.id, group))
        .collect();
    for plan in &roots {
        for body in &plan.0.bodies {
            // Engine boundary: gizmos draw in world units.
            let at = body.position.to_engine();
            let clearance = body.body.clearance().to_engine();
            let skipped = matches!(body.outcome, Outcome::Skipped(_));
            let group = match body.source {
                BodySource::Parent(id) | BodySource::Member(id, _) => groups.get(&id),
                BodySource::Background => None,
            };
            let colour = match (body.source, group) {
                _ if skipped => tailwind::RED_500,
                (BodySource::Parent(_), Some(group)) => kind_colour(group.kind),
                (BodySource::Member(..), Some(group)) if group.parent.owner == plan.0.coord => {
                    kind_colour(group.kind)
                }
                (BodySource::Member(..), Some(_)) => tailwind::ORANGE_400,
                _ => tailwind::GRAY_400,
            };
            let ring = if matches!(body.source, BodySource::Parent(_)) {
                clearance * PARENT_RING
            } else {
                clearance
            };
            gizmos
                .sphere(Isometry3d::from_translation(at), ring, colour)
                .resolution(MARKER_SEGMENTS);
            if skipped {
                let arm = clearance * 1.5;
                gizmos.line(at - Vec3::splat(arm), at + Vec3::splat(arm), colour);
                gizmos.line(
                    at + Vec3::new(-arm, arm, -arm),
                    at + Vec3::new(arm, -arm, arm),
                    colour,
                );
            }
            let (BodySource::Member(..), Some(group)) = (body.source, group) else {
                continue;
            };
            let parent = group.parent.position;
            gizmos.line(
                parent.to_engine(),
                at,
                kind_colour(group.kind).with_alpha(0.7),
            );
            if plan.0.coord != group.parent.owner {
                draw_face_crossing(
                    &mut gizmos,
                    edge,
                    group.parent.owner,
                    parent,
                    plan.0.coord,
                    body.position,
                );
            }
        }
    }
}

/// Mark where the line from a parent in `from` to a member in `to` crosses
/// each face between the two cells.
fn draw_face_crossing(
    gizmos: &mut Gizmos,
    edge: Meters,
    from: SectorCoord,
    parent: Meters3,
    to: SectorCoord,
    member: Meters3,
) {
    let cells = [(from.x, to.x), (from.y, to.y), (from.z, to.z)];
    for (axis, (a, b)) in cells.into_iter().enumerate() {
        if a == b {
            continue;
        }
        // Faces sit half an edge off a cell centre.
        let face = (a.max(b) as f32 - 0.5) * edge.get();
        let (p, m) = (parent.get(), member.get());
        let t = (face - p[axis]) / (m[axis] - p[axis]);
        let point = Meters3(p + (m - p) * t.clamp(0.0, 1.0));
        let normal = Vec3::AXES[axis];
        gizmos.rect(
            Isometry3d::new(point.to_engine(), Quat::from_rotation_arc(Vec3::Z, normal)),
            Vec2::splat(SEAM_MARK.to_engine()),
            tailwind::ORANGE_400,
        );
    }
}

/// Start a heatmap paint when the observer's cell is not the one painted and
/// none for it is running. A paint for a cell the observer has left is
/// dropped, which cancels it.
fn request_heatmap(
    mut commands: Commands,
    current: Option<Res<CurrentSector>>,
    heatmap: Option<Res<Heatmap>>,
    job: Option<Res<HeatmapJob>>,
) {
    let (Some(current), Some(heatmap)) = (current, heatmap) else {
        return;
    };
    let centre = current.0;
    if heatmap.painted == Some(centre) {
        // Back in the painted cell before the paint for the cell it left
        // landed: that paint would overwrite the right picture.
        if job.is_some() {
            commands.remove_resource::<HeatmapJob>();
        }
        return;
    }
    if job.is_some_and(|job| job.centre == centre) {
        return;
    }
    let task = AsyncComputeTaskPool::get().spawn(async move { paint_heatmap(centre) });
    commands.insert_resource(HeatmapJob { centre, task });
}

/// Take a finished paint and write it into the heatmap image.
fn collect_heatmap(
    mut commands: Commands,
    mut job: Option<ResMut<HeatmapJob>>,
    mut heatmap: Option<ResMut<Heatmap>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Some(job), Some(heatmap)) = (job.as_mut(), heatmap.as_mut()) else {
        return;
    };
    let Some(pixels) = block_on(poll_once(&mut job.task)) else {
        return;
    };
    let centre = job.centre;
    commands.remove_resource::<HeatmapJob>();
    let Some(mut image) = images.get_mut(&heatmap.handle) else {
        return;
    };
    image.data = Some(pixels.texels);
    heatmap.painted = Some(centre);
    heatmap.parents = pixels.parents;
    debug!("world clusters: painted the heatmap around {centre}");
}

/// The texel a world point falls on in a heatmap centred on `centre`, if it
/// is on the image.
fn heatmap_texel(centre: Meters3, point: Meters3) -> Option<(i32, i32)> {
    let per_texel = HEATMAP_EXTENT.get() / HEATMAP_PIXELS as f32;
    let half = HEATMAP_EXTENT.get() * 0.5;
    let offset = point.get() - centre.get();
    let column = ((offset.x + half) / per_texel).floor() as i32;
    let row = ((offset.z + half) / per_texel).floor() as i32;
    let size = HEATMAP_PIXELS as i32;
    ((0..size).contains(&column) && (0..size).contains(&row)).then_some((column, row))
}

/// Paint the three fields over the XZ plane through `centre`'s own centre,
/// with the sector faces, the window, and the parents of the cell layer the
/// plane cuts.
///
/// PURE, and the whole cost of the heatmap: three field readings a texel and
/// one group decision a lattice node, never a cell plan. Taken on a worker.
///
/// # Panics
///
/// On a [`SectorFault`] from the fields or a group. A heatmap that painted a
/// refused reading as some colour would be a picture of a bug.
fn paint_heatmap(centre: SectorCoord) -> HeatmapPixels {
    let config = clustered_world_config();
    let edge = config.sector_edge;
    let fields = EnvironmentFields::new(config.seed);
    let middle = centre.centre(edge);
    let size = HEATMAP_PIXELS as usize;
    let per_texel = HEATMAP_EXTENT.get() / HEATMAP_PIXELS as f32;
    let half = HEATMAP_EXTENT.get() * 0.5;
    let window = (EXAMPLE_ACTIVE_RADIUS as f32 + 0.5) * edge.get();
    let mut texels = vec![0_u8; size * size * 4];

    for row in 0..size {
        let z = middle.z().get() - half + (row as f32 + 0.5) * per_texel;
        for column in 0..size {
            let x = middle.x().get() - half + (column as f32 + 0.5) * per_texel;
            let point = Meters3::new(x, middle.y().get(), z);
            let reading = fields
                .sample(point)
                .unwrap_or_else(|fault| panic!("world clusters: heatmap: {fault}"));
            let mut colour = Vec3::ZERO;
            for field in EnvironmentField::ALL {
                let hue = field_colour(field);
                // Squared, so a field reads only where it is high and the
                // overlaps stand out as mixed hues.
                let weight = reading.get(field).powi(2) * 0.8;
                colour += Vec3::new(hue.red, hue.green, hue.blue) * weight;
            }
            let face = |meters: f32| (meters + edge.get() * 0.5).rem_euclid(edge.get()) < per_texel;
            let inside_window =
                (x - middle.x().get()).abs() <= window && (z - middle.z().get()).abs() <= window;
            if face(x) || face(z) {
                let tint = if inside_window { 0.55 } else { 0.25 };
                colour = colour.lerp(Vec3::splat(0.75), tint);
            }
            let at = (row * size + column) * 4;
            let colour = colour.min(Vec3::ONE) * 255.0;
            texels[at..at + 4].copy_from_slice(&[
                colour.x as u8,
                colour.y as u8,
                colour.z as u8,
                255,
            ]);
        }
    }

    // One decision a node, for every node whose parent could fall in the cell
    // layer the plane cuts.
    let lattice = GROUP_LATTICE.get();
    let span =
        |from: f32, to: f32| (from / lattice).floor() as i32 - 1..=(to / lattice).ceil() as i32 + 1;
    let mut parents = (0, 0);
    for nx in span(middle.x().get() - half, middle.x().get() + half) {
        for ny in span(middle.y().get() - edge.get(), middle.y().get() + edge.get()) {
            for nz in span(middle.z().get() - half, middle.z().get() + half) {
                let group = group_at(&fields, config.seed, edge, [nx, ny, nz])
                    .unwrap_or_else(|fault| panic!("world clusters: heatmap: {fault}"));
                let Some(group) = group else {
                    continue;
                };
                if group.parent.owner.y != centre.y {
                    continue;
                }
                let Some((column, row)) = heatmap_texel(middle, group.parent.position) else {
                    continue;
                };
                match group.kind {
                    GroupKind::Planetoid => parents.0 += 1,
                    GroupKind::Derelict => parents.1 += 1,
                }
                let hue = kind_colour(group.kind);
                for dy in -HEATMAP_DOT - 1..=HEATMAP_DOT + 1 {
                    for dx in -HEATMAP_DOT - 1..=HEATMAP_DOT + 1 {
                        let (c, r) = (column + dx, row + dy);
                        if !(0..size as i32).contains(&c) || !(0..size as i32).contains(&r) {
                            continue;
                        }
                        let rim = dx.abs().max(dy.abs()) > HEATMAP_DOT;
                        let colour = if rim {
                            [0, 0, 0, 255]
                        } else {
                            [
                                (hue.red * 255.0) as u8,
                                (hue.green * 255.0) as u8,
                                (hue.blue * 255.0) as u8,
                                255,
                            ]
                        };
                        let at = (r as usize * size + c as usize) * 4;
                        texels[at..at + 4].copy_from_slice(&colour);
                    }
                }
            }
        }
    }
    HeatmapPixels { texels, parents }
}

/// Keep the observer's dot over its own position on the heatmap.
fn place_heatmap_observer(
    config: Option<Res<WorldConfig<ClusteredWorld>>>,
    heatmap: Option<Res<Heatmap>>,
    observer: Query<&GlobalTransform, With<WorldObserver>>,
    mut dot: Query<(&mut Node, &mut Visibility), With<HeatmapObserver>>,
) {
    let (Some(config), Some(heatmap), Ok(transform), Ok((mut node, mut visibility))) =
        (config, heatmap, observer.single(), dot.single_mut())
    else {
        return;
    };
    let painted = heatmap
        .painted
        .map(|centre| centre.centre(config.sector_edge));
    // Engine boundary: a transform counts world units.
    let position = Meters3::from_engine(transform.translation());
    match painted.and_then(|centre| heatmap_texel(centre, position)) {
        Some((column, row)) => {
            node.left = Val::Px(column as f32 - OBSERVER_DOT * 0.5);
            node.top = Val::Px(row as f32 - OBSERVER_DOT * 0.5);
            *visibility = Visibility::Inherited;
        }
        None => *visibility = Visibility::Hidden,
    }
}

/// Say where the observer is, what the fields and chances read there, what
/// this cell and the window planned, and what is live.
#[expect(
    clippy::too_many_arguments,
    reason = "one readout over the whole live state"
)]
fn update_readout(
    config: Option<Res<WorldConfig<ClusteredWorld>>>,
    current: Option<Res<CurrentSector>>,
    fields: Res<ObserverFields>,
    heatmap: Option<Res<Heatmap>>,
    heatmap_job: Option<Res<HeatmapJob>>,
    ready: Res<ReadySectors>,
    observer: Query<&GlobalTransform, With<WorldObserver>>,
    roots: Query<&RootPlan>,
    jobs: Query<&SectorJob>,
    mut readout: Query<&mut Text, With<ClusterReadout>>,
) {
    let (Some(config), Some(current), Ok(transform), Ok(mut text)) =
        (config, current, observer.single(), readout.single_mut())
    else {
        return;
    };
    // Engine boundary: a transform counts world units, the readout meters.
    let position = Meters3::from_engine(transform.translation());
    let offset = position - current.0.centre(config.sector_edge);
    let here = fields
        .0
        .sample(position)
        .unwrap_or_else(|fault| panic!("world clusters: readout: {fault}"));
    let chances = parent_chances(here);
    let tally = |plans: &mut dyn Iterator<Item = &CellPlan>| {
        let (mut planned, mut placed) = (0, 0);
        let mut skipped = [0; SkipReason::ALL.len()];
        for plan in plans {
            planned += plan.bodies.len();
            placed += plan.placed();
            for (slot, reason) in skipped.iter_mut().zip(SkipReason::ALL) {
                *slot += plan.skipped(reason);
            }
        }
        let reasons = SkipReason::ALL
            .iter()
            .zip(skipped)
            .map(|(reason, count)| format!("{} {count}", reason.label()))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "planned {planned} = placed {placed} + skipped {} ({reasons})",
            skipped.iter().sum::<usize>()
        )
    };
    let cell = tally(
        &mut roots
            .iter()
            .map(|plan| &plan.0)
            .filter(|plan| plan.coord == current.0),
    );
    let window = tally(&mut roots.iter().map(|plan| &plan.0));
    let groups: BTreeMap<_, _> = roots
        .iter()
        .flat_map(|plan| &plan.0.groups)
        .map(|group| (group.id, group))
        .collect();
    let count = |kind: GroupKind| groups.values().filter(|group| group.kind == kind).count();
    let seams = groups.values().filter(|group| group.spans_seam()).count();
    let heat = match (heatmap_job, heatmap.and_then(|heatmap| heatmap.painted.map(|at| (at, heatmap.parents)))) {
        (Some(_), _) => "painting...".to_string(),
        (None, Some((at, (planetoids, derelicts)))) => format!(
            "centred on {at}: {planetoids} planetoid and {derelicts} derelict parents in its cell layer"
        ),
        (None, None) => "waiting".to_string(),
    };

    **text = format!(
        "SECTOR {}  {:+.0} {:+.0} {:+.0} m\n\
         here: material {:.2}  volatiles {:.2}  human {:.2}  - '{}' (label only)\n\
         group chance here: planetoid {:.2}  derelict {:.2}\n\
         this cell: {cell}\n\
         window: {} groups ({} planetoid, {} derelict), {seams} span a face\n\
         window: {window}\n\
         live {} sectors, preparing {}, ready {}\n\
         heatmap: {heat}",
        current.0,
        offset.x().get(),
        offset.y().get(),
        offset.z().get(),
        here.material_density,
        here.volatiles,
        here.human_activity,
        here.biome(),
        chances.planetoid,
        chances.derelict,
        groups.len(),
        count(GroupKind::Planetoid),
        count(GroupKind::Derelict),
        roots.iter().count(),
        jobs.iter().count(),
        ready.0.len(),
    );
}

/// Log one census line per crossing: a snapshot of the live plans when the
/// observer entered a new cell, with its own live count because the window
/// is still filling.
fn report_census(current: Option<Res<CurrentSector>>, roots: Query<&RootPlan>) {
    let Some(current) = current else {
        return;
    };
    if !current.is_changed() || roots.is_empty() {
        return;
    }
    let groups: BTreeMap<_, _> = roots
        .iter()
        .flat_map(|plan| &plan.0.groups)
        .map(|group| (group.id, group))
        .collect();
    let planned: usize = roots.iter().map(|plan| plan.0.bodies.len()).sum();
    let placed: usize = roots.iter().map(|plan| plan.0.placed()).sum();
    let skipped: Vec<String> = SkipReason::ALL
        .iter()
        .map(|reason| {
            let count: usize = roots.iter().map(|plan| plan.0.skipped(*reason)).sum();
            format!("{} {count}", reason.label())
        })
        .collect();
    info!(
        "world clusters: window at {} ({} cells live): {} groups, {} span a face; \
         planned {planned} = placed {placed} + skipped ({})",
        current.0,
        roots.iter().count(),
        groups.len(),
        groups.values().filter(|group| group.spans_seam()).count(),
        skipped.join(", "),
    );
}

/// The picture of a planetoid group placed across a face.
#[cfg(feature = "debug")]
const PLANETOID_SHOT: &str = "world-clusters-planetoid.png";

/// The picture of a derelict group placed across a face.
#[cfg(feature = "debug")]
const DERELICT_SHOT: &str = "world-clusters-derelict.png";

/// How far from a planetoid group's centroid its shot stands. Inside the
/// camera's 10 km far plane, with the whole 8 km ring in reach of it.
#[cfg(feature = "debug")]
const PLANETOID_STANDOFF: Meters = Meters(7_000.0);

/// How far behind its lead hull a derelict group's shot stands, looking
/// through the hull at the rest of the group. A hull is about a hundred
/// meters; from the group's middle it was a speck.
#[cfg(feature = "debug")]
const DERELICT_STANDOFF: Meters = Meters(900.0);

/// How far above the line through the lead hull the derelict shot stands, so
/// the hull does not hide the member behind it.
#[cfg(feature = "debug")]
const DERELICT_RISE: Meters = Meters(250.0);

/// The run gate: stream the home window, check the seam groups against the
/// live world, frame one of each kind, and shoot them.
#[cfg(feature = "debug")]
fn clusters_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the streamed window and its heatmap")
        .enter(GameStates::Loading)
        .until(and(
            scenario_is_built(),
            and(
                sector_set_is(CLUSTER_HOME),
                and(every_root_is_planned(), heatmap_is_painted()),
            ),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("check the seam groups against the live world")
        .on_enter(check_seam_groups)
        .add()
        .step("frame the planetoid group")
        .on_enter(|world: &mut World| {
            hide_dev_overlays(world);
            // The version item bakes the commit into a shot.
            hide_status_bar(world);
            let targets = *world.resource::<SeamTargets>();
            pose_camera(
                world,
                standoff(targets.planetoid, PLANETOID_STANDOFF),
                targets.planetoid,
            );
        })
        .until(and(
            window_is_settled(),
            and(heatmap_is_painted(), frames(SETTLE_FRAMES)),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the planetoid group")
        .on_enter(|world: &mut World| shoot(world, PLANETOID_SHOT))
        .until(shot_written(PLANETOID_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("frame the derelict group")
        .on_enter(|world: &mut World| {
            let targets = *world.resource::<SeamTargets>();
            let (middle, lead) = (targets.derelict.get(), targets.derelict_lead.get());
            // A lead hull at the group's middle leaves no line to stand on;
            // any side then frames the group.
            let back = (lead - middle).normalize_or(Vec3::X);
            let eye = lead + back * DERELICT_STANDOFF.get() + Vec3::Y * DERELICT_RISE.get();
            pose_camera(world, Meters3(eye), targets.derelict);
        })
        .until(and(
            window_is_settled(),
            and(heatmap_is_painted(), frames(SETTLE_FRAMES)),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the derelict group")
        .on_enter(|world: &mut World| shoot(world, DERELICT_SHOT))
        .until(shot_written(DERELICT_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}

/// Where the two shot groups stand: the middle of each one's placed bodies,
/// and the derelict group's lead hull.
#[cfg(feature = "debug")]
#[derive(Resource, Clone, Copy)]
struct SeamTargets {
    planetoid: Meters3,
    derelict: Meters3,
    derelict_lead: Meters3,
}

/// Check the home window against the live world and pick the shot groups.
///
/// For each kind, the group nearest the home centre whose PLACED bodies are
/// owned by two or more live cells. Every placed body of it must be a live
/// entity under its planned id, and every group body whose centre is in a
/// live cell must be planned once, by that cell. Panics when any of that
/// fails: the shots would otherwise be pictures that read as a pass.
#[cfg(feature = "debug")]
fn check_seam_groups(world: &mut World) {
    use std::collections::BTreeSet;

    let edge = clustered_world_config().sector_edge;
    let home = CLUSTER_HOME.centre(edge);
    let mut plans = world.query::<&RootPlan>();
    let plans: Vec<CellPlan> = plans.iter(world).map(|plan| plan.0.clone()).collect();
    let mut ids = world.query::<&EntityId>();
    let live: BTreeSet<String> = ids.iter(world).map(|id| id.0.clone()).collect();

    let mut owners: BTreeMap<BodySource, Vec<SectorCoord>> = BTreeMap::new();
    let mut placed_cells: BTreeMap<_, BTreeSet<SectorCoord>> = BTreeMap::new();
    let mut groups = BTreeMap::new();
    for plan in &plans {
        for group in &plan.groups {
            groups.insert(group.id, group.clone());
        }
        for body in &plan.bodies {
            let (BodySource::Parent(group) | BodySource::Member(group, _)) = body.source else {
                continue;
            };
            owners.entry(body.source).or_default().push(plan.coord);
            if body.outcome == Outcome::Placed {
                assert!(
                    live.contains(&body.id),
                    "world clusters: planned body {} is not live in {}",
                    body.id,
                    plan.coord
                );
                placed_cells.entry(group).or_default().insert(plan.coord);
            }
        }
    }
    let cells: BTreeSet<SectorCoord> = plans.iter().map(|plan| plan.coord).collect();
    for group in groups.values() {
        let bodies = std::iter::once((BodySource::Parent(group.id), &group.parent)).chain(
            group
                .members
                .iter()
                .enumerate()
                .map(|(index, member)| (BodySource::Member(group.id, index), member)),
        );
        for (source, body) in bodies.filter(|(_, body)| cells.contains(&body.owner)) {
            assert_eq!(
                owners.get(&source),
                Some(&vec![body.owner]),
                "world clusters: {source:?} must be planned once, by the cell its centre is in"
            );
        }
    }
    let twice: Vec<_> = owners
        .iter()
        .filter(|(_, cells)| cells.len() != 1)
        .collect();
    assert!(
        twice.is_empty(),
        "world clusters: group bodies planned by more than one live cell: {twice:?}"
    );

    let pick = |kind: GroupKind| {
        let group = placed_cells
            .iter()
            .filter(|(id, cells)| groups[*id].kind == kind && cells.len() >= 2)
            .map(|(id, _)| &groups[id])
            .min_by(|a, b| {
                let distance = |group: &world_fixture::ClusterGroup| {
                    group.parent.position.distance(home).get()
                };
                distance(a).total_cmp(&distance(b)).then(a.id.cmp(&b.id))
            })
            .unwrap_or_else(|| {
                panic!(
                    "world clusters: the home window must place a {} group in two cells",
                    kind.label()
                )
            });
        let bodies: Vec<Meters3> = plans
            .iter()
            .flat_map(|plan| &plan.bodies)
            .filter(|body| {
                body.outcome == Outcome::Placed
                    && matches!(body.source, BodySource::Parent(id) | BodySource::Member(id, _) if id == group.id)
            })
            .map(|body| body.position)
            .collect();
        let middle = bodies.iter().fold(Vec3::ZERO, |sum, at| sum + at.get()) / bodies.len() as f32;
        info!(
            "world clusters: {} group {} placed {} bodies across {:?}, all live",
            kind.label(),
            group.id.slug(),
            bodies.len(),
            placed_cells[&group.id],
        );
        (Meters3(middle), group.parent.position)
    };
    let (planetoid, _) = pick(GroupKind::Planetoid);
    let (derelict, derelict_lead) = pick(GroupKind::Derelict);
    let targets = SeamTargets {
        planetoid,
        derelict,
        derelict_lead,
    };
    info!(
        "world clusters: {} live cells plan {} group bodies, each by exactly one cell",
        plans.len(),
        owners.len()
    );
    world.insert_resource(targets);
}

/// An eye `distance` from `target`, up and off to one side, so the shot has
/// depth.
#[cfg(feature = "debug")]
fn standoff(target: Meters3, distance: Meters) -> Meters3 {
    Meters3(target.get() + Vec3::new(0.62, 0.42, 0.66).normalize() * distance.get())
}

/// Advance once every live root carries its plan. The plan is inserted by a
/// command, so a root can be live for a frame without one.
#[cfg(feature = "debug")]
fn every_root_is_planned() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| {
        let Some(mut query) = world.try_query::<(&SectorRoot, Has<RootPlan>)>() else {
            return false;
        };
        query.iter(world).all(|(_, planned)| planned)
    })
}

/// Advance once the heatmap is painted around the observer's cell.
#[cfg(feature = "debug")]
fn heatmap_is_painted() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| {
        let (Some(current), Some(heatmap)) = (
            world.get_resource::<CurrentSector>(),
            world.get_resource::<Heatmap>(),
        ) else {
            return false;
        };
        heatmap.painted == Some(current.0)
    })
}

/// Advance once the live set is the set the observer's current cell wants.
#[cfg(feature = "debug")]
fn window_is_settled() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| {
        let Some(current) = world.get_resource::<CurrentSector>() else {
            return false;
        };
        live_set_is(world, current.0)
    })
}

/// Advance once the live set is exactly the desired set around `centre`.
#[cfg(feature = "debug")]
fn sector_set_is(centre: SectorCoord) -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(move |world: &World| live_set_is(world, centre))
}

#[cfg(feature = "debug")]
fn live_set_is(world: &World, centre: SectorCoord) -> bool {
    let Some(mut query) = world.try_query::<&SectorRoot>() else {
        return false;
    };
    let live: std::collections::BTreeSet<SectorCoord> =
        query.iter(world).map(|root| root.0).collect();
    live == desired_sectors(centre, EXAMPLE_ACTIVE_RADIUS)
}
