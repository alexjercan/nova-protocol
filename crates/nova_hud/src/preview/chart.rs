//! The chart scene the Map's chart view, the New Game seed preview and the
//! Load preview all draw: a grid of cells at cell scale round one centre
//! cell, with the marks found in them.
//!
//! A visited cell is a textured tile: one layer per cluster type, each the
//! type's texture ([`ChartTileTextures`]) in the type's colour, or a faint
//! grid for a cell with no cluster body; a cell never entered is an outline.
//! A station, a wreck or a held job stands as a mark in its cell: a living
//! station in the living colour, a dead one faint in the danger colour, the
//! player's own in the ally colour, with a ring under one that refuses a
//! dock. Chart space is its own frame: the centre cell's middle sits at the
//! origin and one cell is [`CHART_CELL`] units across.
//!
//! Each mask is white with its coverage as alpha, drawn once at startup, and
//! the scene tints it with the cluster type's colour when it builds, so a
//! theme change redraws no texture. A cell with several cluster types stacks
//! one tinted layer per type.
//!
//! The builder spawns meshes only and hands back where each mark stands, so
//! a host that labels its marks (the Map) projects its own labels and a host
//! that does not (the menu) draws nothing more. Touch this module when
//! changing what a cell's contents or a mark look like on any chart.

use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::RenderLayers,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use nova_events::prelude::SectorCoord;
use nova_gameplay::prelude::ClusterType;
use nova_ui::theme::{ActiveUiTheme, UiColor};

/// Glob-import surface: `use nova_hud::preview::chart::prelude::*`.
pub mod prelude {
    pub use super::{
        chart_cell_position, chart_cluster_color, chart_radius, chart_station_color,
        init_chart_tiles, spawn_chart_scene, ChartMarkPlacement, ChartSceneCell, ChartSceneMark,
        ChartSceneMarkType, ChartSceneView, ChartTileTextures, CHART_CELL, CHART_JOB_COLOR,
        CHART_LABEL_STACK, CHART_MARK_LIFT, CHART_PHI, CHART_RADIUS_MAX, CHART_RADIUS_MIN,
        CHART_THETA, CHART_WRECK_COLOR, CHART_YOU_COLOR,
    };
}

/// Width of one cell in chart space, in chart units.
pub const CHART_CELL: f32 = 60.0;
/// The orbit radius that frames a grid reaching `reach` cells out from its
/// centre, with `margin` of room round it in the default fov.
pub fn chart_radius(reach: i32, margin: f32) -> f32 {
    CHART_CELL * (reach as f32 + 0.5) * margin
}
/// How close the wheel may come to a chart: one cell fills the view.
pub const CHART_RADIUS_MIN: f32 = 40.0;
/// How far the wheel may pull back from a chart. A chart camera keeps the
/// default 1000 unit far plane, and the far grid corner must stay inside it.
pub const CHART_RADIUS_MAX: f32 = 680.0;
/// The bearing a chart opens at, radians: the Map's opening view.
pub const CHART_THETA: f32 = 0.8;
/// The height a chart opens at, radians above the floor.
pub const CHART_PHI: f32 = 0.62;
/// Spacing of two marks found in one cell, in chart units.
pub const CHART_MARK_SPACING: f32 = 14.0;
/// Height of a mark over its cell's floor, in chart units.
pub const CHART_MARK_LIFT: f32 = 8.0;
/// Vertical step between the labels of marks that share a cell: a 14 unit
/// sideways shift is narrower than a label, so stacked labels never overlap.
pub const CHART_LABEL_STACK: f32 = 16.0;
/// Side of a tile, as a share of its cell: a gap keeps the grid readable.
const CHART_TILE_SHARE: f32 = 0.92;
/// The alpha of a cluster type's tile layer.
const CHART_TILE_ALPHA: f32 = 0.9;
/// The fill under a visited cell's layers: the centre cell in the own-ship
/// colour, every other visited cell faint.
const CHART_CENTRE_CELL_FILL: (UiColor, f32) = (UiColor::Primary, 0.16);
const CHART_VISITED_CELL_FILL: (UiColor, f32) = (UiColor::Secondary, 0.14);
/// The grid of a visited cell with no cluster body.
const CHART_EMPTY_CELL: (UiColor, f32) = (UiColor::Label, 0.5);
/// The outline of a cell the player never entered.
const CHART_UNVISITED_CELL: (UiColor, f32) = (UiColor::Secondary, 0.6);
/// The alpha a dead station's mark fades to.
const CHART_DEAD_STATION_ALPHA: f32 = 0.3;
/// The colour of a wreck mark.
pub const CHART_WRECK_COLOR: UiColor = UiColor::AccentHigh;
/// The colour of a held job's mark.
pub const CHART_JOB_COLOR: UiColor = UiColor::Accent;
/// The colour of the own ship: the Map's own-ship colour.
pub const CHART_YOU_COLOR: UiColor = UiColor::Primary;

/// The colour a cluster type's tile layer, icon and word read in.
pub const fn chart_cluster_color(cluster: ClusterType) -> UiColor {
    match cluster {
        ClusterType::AsteroidRich => UiColor::Accent,
        ClusterType::RockOnly => UiColor::Secondary,
        ClusterType::PlanetHeavy => UiColor::Info,
        ClusterType::DerelictField => UiColor::Body,
    }
}

/// The colour a station mark reads in: dead in the danger colour, the
/// player's own in the ally colour, any other living station in the living
/// colour.
pub const fn chart_station_color(dead: bool, owned: bool) -> UiColor {
    if dead {
        UiColor::Danger
    } else if owned {
        UiColor::Info
    } else {
        UiColor::Nominal
    }
}

/// Where a cell sits in chart space, from the centre cell.
pub fn chart_cell_position(cell: SectorCoord, centre: SectorCoord) -> Vec3 {
    Vec3::new(
        (cell.x - centre.x) as f32,
        (cell.y - centre.y) as f32,
        (cell.z - centre.z) as f32,
    ) * CHART_CELL
}

/// One visited cell a chart scene draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChartSceneCell {
    /// The cell.
    pub at: SectorCoord,
    /// The type of every cluster with a body in it, one tile layer each.
    pub clusters: Vec<ClusterType>,
}

/// What a chart mark stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChartSceneMarkType {
    /// A station: a cube, faint when dead, ringed when it refuses a dock.
    Station {
        /// Dead when found.
        dead: bool,
        /// The player's.
        owned: bool,
        /// Refusing the player a dock right now.
        refusing: bool,
    },
    /// A derelict hull: a tetrahedron.
    Wreck,
    /// A held job: a post in its target's cell.
    Job,
}

impl ChartSceneMarkType {
    /// The colour the mark and its label read in.
    pub const fn color(self) -> UiColor {
        match self {
            Self::Station { dead, owned, .. } => chart_station_color(dead, owned),
            Self::Wreck => CHART_WRECK_COLOR,
            Self::Job => CHART_JOB_COLOR,
        }
    }
}

/// One mark a chart scene draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChartSceneMark {
    /// The cell it stands in.
    pub at: SectorCoord,
    /// What it is.
    pub kind: ChartSceneMarkType,
}

/// Everything a chart scene draws.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartSceneView {
    /// The cell at the origin of chart space.
    pub centre: SectorCoord,
    /// How many cells the grid reaches out from the centre on each
    /// horizontal axis.
    pub reach: i32,
    /// Every visited cell, on the grid or off it. A grid cell not listed is
    /// drawn as an outline.
    pub cells: Vec<ChartSceneCell>,
    /// Every mark, in draw order: marks sharing a cell stand side by side in
    /// this order.
    pub marks: Vec<ChartSceneMark>,
    /// The cell outlined in the accent, if one is selected.
    pub selected: Option<SectorCoord>,
    /// Where the own ship stands in chart space, if it is drawn.
    pub you: Option<Vec3>,
}

/// Where one mark of a [`ChartSceneView`] stands, in the order of its marks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartMarkPlacement {
    /// The mark's anchor: its cell, shifted beside the cell's other marks
    /// and lifted over the floor.
    pub at: Vec3,
    /// Its place among the marks of its cell.
    pub slot: usize,
    /// How many marks share its cell.
    pub in_cell: usize,
}

/// An unlit material, so a chart's meshes read at full colour without a
/// light on the layer they live on.
fn unlit(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    }
}

/// A tile texture in `color`.
fn textured(texture: Handle<Image>, color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color_texture: Some(texture),
        ..unlit(color)
    }
}

/// Spawn the meshes of `view` under `root` on `layer`: the grid lines, a
/// tile per grid cell and per visited cell, a mark per find and the own
/// ship. Answers where each mark stands, in the order of `view.marks`, so
/// the host can label them.
#[expect(
    clippy::too_many_arguments,
    reason = "one builder taking the scene assets, the theme, the view and its place"
)]
pub fn spawn_chart_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    tiles: &ChartTileTextures,
    theme: &ActiveUiTheme,
    view: &ChartSceneView,
    layer: RenderLayers,
    root: Entity,
) -> Vec<ChartMarkPlacement> {
    let mut spawn_mesh = |mesh: Handle<Mesh>, material: Handle<StandardMaterial>, at: Transform| {
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            at,
            layer.clone(),
            ChildOf(root),
        ));
    };
    let centre = view.centre;
    let reach = view.reach;

    // The grid: cell borders on the centre's layer.
    let span = CHART_CELL * (2 * reach + 1) as f32;
    let line_material = materials.add(unlit(theme.color_alpha(UiColor::Secondary, 0.5)));
    let along_x = meshes.add(Cuboid::new(span, 0.6, 0.6));
    let along_z = meshes.add(Cuboid::new(0.6, 0.6, span));
    for line in 0..=(2 * reach + 1) {
        let offset = (line as f32 - reach as f32 - 0.5) * CHART_CELL;
        spawn_mesh(
            along_x.clone(),
            line_material.clone(),
            Transform::from_xyz(0.0, 0.0, offset),
        );
        spawn_mesh(
            along_z.clone(),
            line_material.clone(),
            Transform::from_xyz(offset, 0.0, 0.0),
        );
    }

    // The tiles: every cell of the grid, plus every visited cell off it. A
    // visited cell is a fill under one textured layer per cluster type, or
    // the empty grid; a cell never entered is an outline.
    let tile = meshes.add(
        Plane3d::default()
            .mesh()
            .size(CHART_CELL * CHART_TILE_SHARE, CHART_CELL * CHART_TILE_SHARE),
    );
    let mut coords: Vec<SectorCoord> = view.cells.iter().map(|cell| cell.at).collect();
    for dx in -reach..=reach {
        for dz in -reach..=reach {
            let coord = SectorCoord::new(centre.x + dx, centre.y, centre.z + dz);
            if !coords.contains(&coord) {
                coords.push(coord);
            }
        }
    }
    for coord in coords {
        let at = chart_cell_position(coord, centre);
        match view.cells.iter().find(|cell| cell.at == coord) {
            Some(cell) => {
                let (color, alpha) = if coord == centre {
                    CHART_CENTRE_CELL_FILL
                } else {
                    CHART_VISITED_CELL_FILL
                };
                spawn_mesh(
                    tile.clone(),
                    materials.add(unlit(theme.color_alpha(color, alpha))),
                    Transform::from_translation(at - Vec3::Y * 0.3),
                );
                if cell.clusters.is_empty() {
                    let (color, alpha) = CHART_EMPTY_CELL;
                    spawn_mesh(
                        tile.clone(),
                        materials.add(textured(
                            tiles.grid.clone(),
                            theme.color_alpha(color, alpha),
                        )),
                        Transform::from_translation(at),
                    );
                }
                for (index, cluster) in cell.clusters.iter().enumerate() {
                    spawn_mesh(
                        tile.clone(),
                        materials.add(textured(
                            tiles.cluster(*cluster),
                            theme.color_alpha(chart_cluster_color(*cluster), CHART_TILE_ALPHA),
                        )),
                        Transform::from_translation(at + Vec3::Y * (0.2 + index as f32 * 0.2)),
                    );
                }
            }
            None => {
                let (color, alpha) = CHART_UNVISITED_CELL;
                spawn_mesh(
                    tile.clone(),
                    materials.add(textured(
                        tiles.outline.clone(),
                        theme.color_alpha(color, alpha),
                    )),
                    Transform::from_translation(at),
                );
            }
        }
        if view.selected == Some(coord) {
            spawn_mesh(
                tile.clone(),
                materials.add(textured(
                    tiles.outline.clone(),
                    theme.color(UiColor::Accent),
                )),
                Transform::from_translation(at + Vec3::Y * 1.2),
            );
        }
    }

    // The marks, side by side in their cell.
    let station_mesh = meshes.add(Cuboid::new(9.0, 9.0, 9.0));
    let refusal_ring = meshes.add(Torus::new(8.0, 9.5));
    let wreck_mesh = meshes.add(Tetrahedron::default());
    let job_mesh = meshes.add(Cuboid::new(4.0, 14.0, 4.0));
    let mut placements = Vec::with_capacity(view.marks.len());
    for (index, mark) in view.marks.iter().enumerate() {
        let in_cell = view
            .marks
            .iter()
            .filter(|other| other.at == mark.at)
            .count();
        let slot = view.marks[..index]
            .iter()
            .filter(|other| other.at == mark.at)
            .count();
        let shift = (slot as f32 - (in_cell - 1) as f32 * 0.5) * CHART_MARK_SPACING;
        let at = chart_cell_position(mark.at, centre) + Vec3::new(shift, CHART_MARK_LIFT, 0.0);
        let color = mark.kind.color();
        match mark.kind {
            ChartSceneMarkType::Station { dead, refusing, .. } => {
                let alpha = if dead { CHART_DEAD_STATION_ALPHA } else { 1.0 };
                spawn_mesh(
                    station_mesh.clone(),
                    materials.add(unlit(theme.color_alpha(color, alpha))),
                    Transform::from_translation(at),
                );
                if refusing {
                    spawn_mesh(
                        refusal_ring.clone(),
                        materials.add(unlit(theme.color(UiColor::Danger))),
                        Transform::from_translation(at - Vec3::Y * 4.0),
                    );
                }
            }
            ChartSceneMarkType::Wreck => {
                spawn_mesh(
                    wreck_mesh.clone(),
                    materials.add(unlit(theme.color(color))),
                    Transform::from_translation(at).with_scale(Vec3::splat(10.0)),
                );
            }
            ChartSceneMarkType::Job => {
                spawn_mesh(
                    job_mesh.clone(),
                    materials.add(unlit(theme.color(color))),
                    Transform::from_translation(at + Vec3::Y * 3.0),
                );
            }
        }
        placements.push(ChartMarkPlacement { at, slot, in_cell });
    }

    // The own ship, where it stands in its cell.
    if let Some(you) = view.you {
        spawn_mesh(
            meshes.add(Sphere::new(3.5)),
            materials.add(unlit(theme.color(CHART_YOU_COLOR))),
            Transform::from_translation(you + Vec3::Y * CHART_MARK_LIFT * 0.5),
        );
    }
    placements
}

/// Every tile texture the chart draws. [`init_chart_tiles`] draws them at
/// startup; a host without image assets has none.
#[derive(Resource)]
pub struct ChartTileTextures {
    /// The faint grid of a visited cell with no cluster body.
    pub grid: Handle<Image>,
    /// The outline of a cell the player never entered, and of the selected
    /// cell in the accent.
    pub outline: Handle<Image>,
    /// One texture per cluster type, in [`ClusterType::ALL`] order.
    clusters: [Handle<Image>; ClusterType::ALL.len()],
}

impl ChartTileTextures {
    /// The texture of a cluster type's layer.
    pub fn cluster(&self, cluster: ClusterType) -> Handle<Image> {
        let index = ClusterType::ALL
            .iter()
            .position(|each| *each == cluster)
            .expect("ClusterType::ALL lists every type");
        self.clusters[index].clone()
    }
}

/// Texture edge, in texels.
const TILE_TEXELS: u32 = 128;

/// Coverage of a shape whose signed distance at a texel is `d`.
fn soft(d: f32, px: f32) -> f32 {
    (0.5 - d / px).clamp(0.0, 1.0)
}

fn disc(p: Vec2, c: Vec2, r: f32) -> f32 {
    (p - c).length() - r
}

/// A fixed hash, so every texture draws the same scatter on every start.
fn hash(seed: u64, salt: u64) -> u64 {
    let mut x = seed ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// A seeded point in `[-0.9, 0.9]` squared.
fn scatter(seed: u64, i: u64) -> Vec2 {
    let h = hash(seed, i);
    let x = (h & 0xFFFF) as f32 / 65_535.0;
    let y = ((h >> 16) & 0xFFFF) as f32 / 65_535.0;
    Vec2::new(x * 1.8 - 0.9, y * 1.8 - 0.9)
}

/// A seeded size in `[min, max]`.
fn scatter_size(seed: u64, i: u64, min: f32, max: f32) -> f32 {
    let h = hash(seed, i ^ 0xABCD);
    min + ((h >> 32) & 0xFFFF) as f32 / 65_535.0 * (max - min)
}

/// A faint grid, four lines each way, and a border.
fn grid_coverage(p: Vec2) -> f32 {
    let cellf = |v: f32| ((v + 1.0) * 2.0).fract();
    let line = |f: f32| (f.min(1.0 - f) * 0.5 - 0.01).min(0.0).abs() / 0.01;
    let grid = line(cellf(p.x)).max(line(cellf(p.y))).clamp(0.0, 1.0);
    let edge = (p.x.abs().max(p.y.abs()) - 0.97).max(0.0) / 0.03;
    (grid * 0.45).max(edge.min(1.0) * 0.7)
}

/// An outline only.
fn outline_coverage(p: Vec2, px: f32) -> f32 {
    let edge = p.x.abs().max(p.y.abs());
    soft((edge - 0.97).abs() - 0.025, px)
}

/// A speckle of cratered lumps for an asteroid-rich cell, a sparse scatter of
/// rocks for a rock-only one, a shaded disc with a ring for a planet-heavy
/// one, and a few cracked hull slivers for a derelict field.
fn cluster_coverage(cluster: ClusterType, p: Vec2, px: f32) -> f32 {
    match cluster {
        ClusterType::AsteroidRich => (0..70u64)
            .map(|i| {
                let c = scatter(11, i);
                let r = scatter_size(11, i, 0.02, 0.065);
                let crater = disc(p, c + Vec2::splat(r * 0.3), r * 0.4);
                soft(disc(p, c, r), px) * (0.55 + 0.45 * soft(crater, px).max(0.0)).min(1.0)
            })
            .fold(0.0, f32::max),
        ClusterType::RockOnly => (0..16u64)
            .map(|i| {
                let c = scatter(23, i);
                let r = scatter_size(23, i, 0.015, 0.035);
                soft(disc(p, c, r), px)
            })
            .fold(0.0, f32::max),
        ClusterType::PlanetHeavy => {
            let c = Vec2::new(0.05, -0.05);
            let body = soft(disc(p, c, 0.34), px);
            let lit = ((p.x - c.x + p.y - c.y) * -1.6).clamp(0.0, 1.0);
            let shade = 0.3 + 0.7 * lit;
            let q = p - c;
            let ellipse = (q.x * q.x / 0.42 + q.y * q.y / 0.06).sqrt();
            let ring = soft((ellipse - 1.0).abs() - 0.07, px) * 0.75;
            // The ring passes behind the disc's upper half.
            let behind = q.y < 0.0 && q.length() < 0.34;
            if behind {
                body * shade
            } else {
                (body * shade).max(ring)
            }
        }
        ClusterType::DerelictField => (0..5u64)
            .map(|i| {
                let c = scatter(37, i) * 0.8;
                let angle = scatter_size(37, i, 0.0, std::f32::consts::PI);
                let dir = Vec2::from_angle(angle);
                let len = scatter_size(37, i + 50, 0.12, 0.22);
                let q = p - c;
                let along = q.dot(dir);
                let across = q.perp_dot(dir).abs();
                let sliver = (along.abs() - len).max(across - 0.035);
                let crack = (along - len * 0.2).abs() - 0.012;
                soft(sliver.max(-crack), px)
            })
            .fold(0.0, f32::max),
    }
}

/// Draw one texture from its coverage over `[-1, 1]` squared.
fn tile_image(images: &mut Assets<Image>, coverage: impl Fn(Vec2, f32) -> f32) -> Handle<Image> {
    let texel = 2.0 / TILE_TEXELS as f32;
    let mut data = Vec::with_capacity((TILE_TEXELS * TILE_TEXELS * 4) as usize);
    for y in 0..TILE_TEXELS {
        for x in 0..TILE_TEXELS {
            let at = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * texel - Vec2::ONE;
            let alpha = coverage(at, texel);
            data.extend_from_slice(&[255, 255, 255, (alpha * 255.0).round() as u8]);
        }
    }
    images.add(Image::new(
        Extent3d {
            width: TILE_TEXELS,
            height: TILE_TEXELS,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

/// Draw every tile texture once. A headless host with no image assets draws
/// none, and its charts stay unbuilt.
pub fn init_chart_tiles(mut commands: Commands, images: Option<ResMut<Assets<Image>>>) {
    let Some(mut images) = images else {
        return;
    };
    commands.insert_resource(ChartTileTextures {
        grid: tile_image(&mut images, |p, _| grid_coverage(p)),
        outline: tile_image(&mut images, outline_coverage),
        clusters: ClusterType::ALL
            .map(|cluster| tile_image(&mut images, |p, px| cluster_coverage(cluster, p, px))),
    });
}
