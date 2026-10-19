//! The gallery's 3D scenes: one reusable chart scene (the Chart page, the
//! Load preview and the New Game seed preview), the spinning hull viewport
//! of the role step, and the menu backdrop of drifting rocks behind the New
//! Game modal.
//!
//! Each scene renders through its own `Camera3d` into an image that a UI
//! `ImageNode` shows, sized 1:1 to the node each frame, exactly as the game's
//! Map and Ship panes do. Tiles are textured planes: one procedural texture
//! per cell content, drawn once at startup and tinted by the theme at build
//! time; a cell with several contents stacks one layer per content. Marks are
//! the game's meshes (the 9 m station cube, the wreck tetrahedron, the 4 x 14
//! job post, the 3.5 m player sphere) and their codes are UI labels projected
//! through the camera every frame.
//!
//! The orbit is the game's viewer: right-drag turns at 0.0024 rad/px, the
//! wheel zooms proportionally, WASD pans in the view frame, Q/E and R/F turn
//! and tilt, T reframes, and the elevation stays off the poles. Orbits live
//! in [`ChartOrbits`] per scene kind, so a page rebuild keeps the camera where
//! the player left it.

use bevy::{
    asset::RenderAssetUsages,
    camera::{visibility::RenderLayers, ImageRenderTarget, RenderTarget},
    ecs::system::SystemParam,
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
    picking::hover::Hovered,
    platform::collections::HashMap,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use nova_protocol::nova_interface::prelude::stack_chart_labels;
use nova_ui::{
    prelude::{TextFieldFocused, UiText},
    theme::{ActiveUiTheme, UiColor},
    widget::{ThemedBorder, ThemedFill, ThemedText},
};

use crate::{
    data::{hash, Content, Hull, StationKind},
    ChartState, Click, TILE_PREFIX,
};

/// Width of one cell in chart space, in map units, as the game's chart.
pub const CELL: f32 = 60.0;
/// Spacing of two marks found in one cell.
const MARK_SPACING: f32 = 14.0;
/// Height of a mark over its cell's floor.
const MARK_LIFT: f32 = 8.0;
const THETA_DEFAULT: f32 = 0.8;
const PHI_DEFAULT: f32 = 0.62;
/// The elevation rails: short of straight overhead, where the look-at up
/// vector degenerates, and above the focus plane.
const PHI_MAX: f32 = 1.45;
const PHI_MIN: f32 = 0.12;
const TURN_RATE: f32 = 1.6;
const DRAG_RADIANS_PER_PX: f32 = 0.0024;
const ZOOM_PER_NOTCH: f32 = 0.12;
/// The default 45 degree fov holds the whole grid at this multiple of its
/// half span.
const FRAMING_MARGIN: f32 = 2.42;
const PREVIEW_FRAMING_MARGIN: f32 = 1.7;
const RADIUS_MIN: f32 = 40.0;
/// The camera keeps the default 1000 unit far plane; the far corner must stay
/// inside it.
const RADIUS_MAX: f32 = 680.0;

/// Which scene a viewport is: the orbit is remembered per kind, and each
/// kind draws on its own render layer so the cameras never see each other.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SceneKind {
    Chart,
    LoadPreview,
    NewGamePreview,
    Backdrop,
    Hull(usize),
}

impl SceneKind {
    fn index(self) -> usize {
        match self {
            SceneKind::Chart => 0,
            SceneKind::LoadPreview => 1,
            SceneKind::NewGamePreview => 2,
            SceneKind::Backdrop => 3,
            SceneKind::Hull(i) => 4 + i,
        }
    }

    /// Layers 21 up, clear of the world's layer 0.
    fn layer(self) -> usize {
        21 + self.index()
    }

    /// Before the window camera, each distinct.
    fn order(self) -> isize {
        -30 - self.index() as isize
    }
}

/// An orbit camera's spherical state around its focus.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbit {
    pub theta: f32,
    pub phi: f32,
    pub radius: f32,
    pub center: Vec3,
}

impl Orbit {
    /// The default view of a `reach` grid. A preview sits in a wide, short
    /// node, so it frames tighter than the chart.
    fn framed(kind: SceneKind, reach: i32) -> Self {
        let margin = if kind == SceneKind::Chart {
            FRAMING_MARGIN
        } else {
            PREVIEW_FRAMING_MARGIN
        };
        Self {
            theta: THETA_DEFAULT,
            phi: PHI_DEFAULT,
            radius: CELL * (reach as f32 + 0.5) * margin,
            center: Vec3::ZERO,
        }
    }

    fn eye(&self) -> Vec3 {
        let horizontal = self.radius * self.phi.cos();
        self.center
            + Vec3::new(
                horizontal * self.theta.sin(),
                self.radius * self.phi.sin(),
                horizontal * self.theta.cos(),
            )
    }

    /// A right-drag of `delta` px, the game's feel.
    pub fn drag(&mut self, delta: Vec2) {
        self.theta -= delta.x * DRAG_RADIANS_PER_PX;
        self.phi = (self.phi + delta.y * DRAG_RADIANS_PER_PX).clamp(PHI_MIN, PHI_MAX);
    }
}

/// The remembered orbit of each scene kind. A page rebuild spawns a fresh
/// camera that reads its orbit from here, so a tile click never resets the
/// view.
#[derive(Resource, Default)]
pub struct ChartOrbits(HashMap<SceneKind, Orbit>);

impl ChartOrbits {
    /// Forget a kind's orbit: the next frame frames it afresh.
    pub fn reframe(&mut self, kind: SceneKind) {
        self.0.remove(&kind);
    }

    pub fn get(&self, kind: SceneKind) -> Option<Orbit> {
        self.0.get(&kind).copied()
    }

    /// Apply a drag to a kind's orbit, framing it first if it has none.
    pub fn drag(&mut self, kind: SceneKind, reach: i32, delta: Vec2) {
        self.0
            .entry(kind)
            .or_insert_with(|| Orbit::framed(kind, reach))
            .drag(delta);
    }
}

/// Which scene the right button went down over, so a drag that leaves the
/// node keeps turning it.
#[derive(Resource, Default)]
pub struct ChartPointer {
    drag: Option<Entity>,
}

// ---------------------------------------------------------------------------
// Tile textures
// ---------------------------------------------------------------------------

/// The procedural tile textures, one per content plus the faint grid of an
/// empty cell and the outline of an unvisited one.
#[derive(Resource)]
pub struct TileTextures {
    grid: Handle<Image>,
    outline: Handle<Image>,
    asteroids: Handle<Image>,
    rocks: Handle<Image>,
    planet: Handle<Image>,
    derelicts: Handle<Image>,
}

impl TileTextures {
    fn content(&self, content: Content) -> Handle<Image> {
        match content {
            Content::Asteroids => self.asteroids.clone(),
            Content::Rocks => self.rocks.clone(),
            Content::Planet => self.planet.clone(),
            Content::Derelicts => self.derelicts.clone(),
        }
    }
}

/// Texture edge, in texels.
const TILE_TEXELS: u32 = 128;

fn soft(d: f32, px: f32) -> f32 {
    (0.5 - d / px).clamp(0.0, 1.0)
}

fn disc(p: Vec2, c: Vec2, r: f32) -> f32 {
    (p - c).length() - r
}

/// A seeded point in `[-0.9, 0.9]` squared.
fn scatter(seed: u64, i: u64) -> Vec2 {
    let h = hash(seed, i);
    let x = (h & 0xFFFF) as f32 / 65_535.0;
    let y = ((h >> 16) & 0xFFFF) as f32 / 65_535.0;
    Vec2::new(x * 1.8 - 0.9, y * 1.8 - 0.9)
}

fn scatter_size(seed: u64, i: u64, min: f32, max: f32) -> f32 {
    let h = hash(seed, i ^ 0xABCD);
    min + ((h >> 32) & 0xFFFF) as f32 / 65_535.0 * (max - min)
}

/// Coverage of one tile texture at `p` in `[-1, 1]` squared.
fn tile_coverage(kind: usize, p: Vec2, px: f32) -> f32 {
    match kind {
        // A faint grid, four lines each way, and a border.
        0 => {
            let cellf = |v: f32| ((v + 1.0) * 2.0).fract();
            let line = |f: f32| (f.min(1.0 - f) * 0.5 - 0.01).min(0.0).abs() / 0.01;
            let grid = line(cellf(p.x)).max(line(cellf(p.y))).clamp(0.0, 1.0);
            let edge = (p.x.abs().max(p.y.abs()) - 0.97).max(0.0) / 0.03;
            (grid * 0.45).max(edge.min(1.0) * 0.7)
        }
        // An outline only.
        1 => {
            let edge = p.x.abs().max(p.y.abs());
            soft((edge - 0.97).abs() - 0.025, px)
        }
        // A speckle of lumps.
        2 => (0..70u64)
            .map(|i| {
                let c = scatter(11, i);
                let r = scatter_size(11, i, 0.02, 0.065);
                let crater = disc(p, c + Vec2::splat(r * 0.3), r * 0.4);
                soft(disc(p, c, r), px) * (0.55 + 0.45 * soft(crater, px).max(0.0)).min(1.0)
            })
            .fold(0.0, f32::max),
        // A sparse scatter of rocks.
        3 => (0..16u64)
            .map(|i| {
                let c = scatter(23, i);
                let r = scatter_size(23, i, 0.015, 0.035);
                soft(disc(p, c, r), px)
            })
            .fold(0.0, f32::max),
        // A disc with a terminator and a ring.
        4 => {
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
        // A few hull slivers, each split by a crack.
        _ => (0..5u64)
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

fn tile_image(images: &mut Assets<Image>, kind: usize) -> Handle<Image> {
    let texel = 2.0 / TILE_TEXELS as f32;
    let mut data = Vec::with_capacity((TILE_TEXELS * TILE_TEXELS * 4) as usize);
    for y in 0..TILE_TEXELS {
        for x in 0..TILE_TEXELS {
            let at = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * texel - Vec2::ONE;
            let alpha = tile_coverage(kind, at, texel);
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

/// Draw every tile texture once.
pub fn init_tile_textures(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(TileTextures {
        grid: tile_image(&mut images, 0),
        outline: tile_image(&mut images, 1),
        asteroids: tile_image(&mut images, 2),
        rocks: tile_image(&mut images, 3),
        planet: tile_image(&mut images, 4),
        derelicts: tile_image(&mut images, 5),
    });
}

// ---------------------------------------------------------------------------
// The chart scene
// ---------------------------------------------------------------------------

/// A mark a cell holds, as the scene draws it.
#[derive(Clone, Debug)]
pub enum MarkView {
    Station { kind: StationKind, code: String },
    Wreck(String),
    Job(String),
}

/// One cell of a chart view.
#[derive(Clone, Debug)]
pub struct CellView {
    pub at: (i32, i32),
    pub visited: bool,
    pub contents: Vec<Content>,
    pub marks: Vec<MarkView>,
}

/// What a chart scene draws.
#[derive(Clone, Debug)]
pub struct ChartView {
    pub player: (i32, i32),
    pub reach: i32,
    pub cells: Vec<CellView>,
    pub selected: Option<(i32, i32)>,
    /// Visited cells carry their coordinate label, which selects them.
    pub label_cells: bool,
    /// The player sphere's tag.
    pub you: &'static str,
}

impl ChartView {
    /// A signature of the drawn cells, for a smoke check that a seed change
    /// changed the preview.
    #[cfg(feature = "debug")]
    pub fn signature(&self) -> u64 {
        self.cells.iter().fold(0, |acc, cell| {
            let contents = cell
                .contents
                .iter()
                .fold(0u64, |a, c| a * 7 + *c as u64 + 1);
            hash(
                acc,
                (cell.at.0 as u64) << 32 ^ cell.at.1 as u64 ^ contents << 8,
            )
        })
    }
}

/// The assets a scene build needs.
#[derive(SystemParam)]
pub struct SceneAssets<'w> {
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
    pub images: ResMut<'w, Assets<Image>>,
    pub tiles: Res<'w, TileTextures>,
    pub theme: Res<'w, ActiveUiTheme>,
    pub orbits: Res<'w, ChartOrbits>,
}

/// The camera and scene root a viewport node owns, despawned with the node.
#[derive(Component)]
pub struct SceneTarget {
    camera: Entity,
    root: Entity,
    image: Handle<Image>,
}

impl SceneTarget {
    /// The scene's camera, for the smoke check that an orbit moves it.
    #[cfg(feature = "debug")]
    pub fn camera(&self) -> Entity {
        self.camera
    }
}

/// A chart viewport node.
#[derive(Component)]
pub struct ChartScene {
    pub kind: SceneKind,
    player: (i32, i32),
    reach: i32,
    /// Keys reach this scene while no scene is hovered.
    primary: bool,
    /// A click on the floor selects the cell under it.
    clicks: bool,
}

/// A hull viewport node: which role's hull it shows.
#[derive(Component)]
pub struct HullScene {
    #[cfg_attr(
        not(feature = "debug"),
        expect(dead_code, reason = "the smoke reads which role the viewport shows")
    )]
    pub role: usize,
}

/// A scene root that turns about Y at a rate in radians per second.
#[derive(Component)]
pub struct HullSpin(pub f32);

/// One projected label over a viewport.
#[derive(Component)]
pub struct ChartLabel {
    scene: Entity,
    anchor: Vec3,
    selects: Option<(i32, i32)>,
}

fn unlit(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    }
}

fn textured(texture: Handle<Image>, color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        base_color_texture: Some(texture),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    }
}

fn render_target(images: &mut Assets<Image>) -> Handle<Image> {
    images.add(Image::new_target_texture(
        64,
        64,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ))
}

fn cell_position(at: (i32, i32), player: (i32, i32)) -> Vec3 {
    Vec3::new(
        (at.0 - player.0) as f32 * CELL,
        0.0,
        (at.1 - player.1) as f32 * CELL,
    )
}

/// The tint a content's tile layer reads in, the game's cluster colours.
pub fn content_color(content: Content) -> UiColor {
    match content {
        Content::Asteroids => UiColor::Accent,
        Content::Rocks => UiColor::Secondary,
        Content::Planet => UiColor::Info,
        Content::Derelicts => UiColor::Body,
    }
}

/// The tint of a station mark by kind.
pub fn station_color(kind: StationKind) -> UiColor {
    match kind {
        StationKind::Living | StationKind::Posting(_) | StationKind::Refusing => UiColor::Nominal,
        StationKind::Dead => UiColor::Danger,
        StationKind::Owned => UiColor::Info,
    }
}

/// Spawn a chart viewport under `parent`: the node, its camera, its scene
/// and its labels. Returns the node entity.
pub fn spawn_chart_scene(
    parent: &mut ChildSpawnerCommands,
    assets: &mut SceneAssets,
    kind: SceneKind,
    view: &ChartView,
    node: Node,
    name: &str,
) -> Entity {
    let image = render_target(&mut assets.images);
    let node_entity = parent
        .spawn((
            Name::new(name.to_string()),
            Node {
                position_type: PositionType::Relative,
                overflow: Overflow::clip(),
                ..node
            },
            ImageNode::new(image.clone()),
            Hovered::default(),
        ))
        .id();
    let commands = parent.commands_mut();
    let theme = &*assets.theme;
    let layer = RenderLayers::layer(kind.layer());
    let orbit = assets
        .orbits
        .get(kind)
        .unwrap_or_else(|| Orbit::framed(kind, view.reach));

    let root = commands
        .spawn((
            Name::new(format!("{name} Scene")),
            Transform::default(),
            Visibility::Visible,
        ))
        .id();
    let camera = commands
        .spawn((
            Name::new(format!("{name} Camera")),
            Camera3d::default(),
            Camera {
                order: kind.order(),
                clear_color: ClearColorConfig::Custom(theme.color(UiColor::Surface)),
                ..default()
            },
            RenderTarget::Image(ImageRenderTarget {
                handle: image.clone(),
                scale_factor: 1.0,
            }),
            Transform::from_translation(orbit.eye()).looking_at(orbit.center, Vec3::Y),
            layer.clone(),
        ))
        .id();

    let spawn_mesh = |commands: &mut Commands,
                      mesh: Handle<Mesh>,
                      material: Handle<StandardMaterial>,
                      at: Transform| {
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            at,
            layer.clone(),
            ChildOf(root),
        ));
    };

    // The grid: cell borders.
    let span = CELL * (2 * view.reach + 1) as f32;
    let line_material = assets
        .materials
        .add(unlit(theme.color_alpha(UiColor::Secondary, 0.5)));
    let along_x = assets.meshes.add(Cuboid::new(span, 0.6, 0.6));
    let along_z = assets.meshes.add(Cuboid::new(0.6, 0.6, span));
    for line in 0..=(2 * view.reach + 1) {
        let offset = (line as f32 - view.reach as f32 - 0.5) * CELL;
        spawn_mesh(
            commands,
            along_x.clone(),
            line_material.clone(),
            Transform::from_xyz(0.0, 0.0, offset),
        );
        spawn_mesh(
            commands,
            along_z.clone(),
            line_material.clone(),
            Transform::from_xyz(offset, 0.0, 0.0),
        );
    }

    let tile = assets
        .meshes
        .add(Plane3d::default().mesh().size(CELL * 0.92, CELL * 0.92));
    let mut labels: Vec<(Vec3, Option<(i32, i32)>, String, String, UiColor, f32)> = Vec::new();
    for cell in &view.cells {
        let at = cell_position(cell.at, view.player);
        let player = cell.at == view.player;
        if cell.visited {
            let (color, alpha) = if player {
                (UiColor::Primary, 0.16)
            } else {
                (UiColor::Secondary, 0.14)
            };
            spawn_mesh(
                commands,
                tile.clone(),
                assets.materials.add(unlit(theme.color_alpha(color, alpha))),
                Transform::from_translation(at - Vec3::Y * 0.3),
            );
            if cell.contents.is_empty() {
                spawn_mesh(
                    commands,
                    tile.clone(),
                    assets.materials.add(textured(
                        assets.tiles.grid.clone(),
                        theme.color_alpha(UiColor::Label, 0.5),
                    )),
                    Transform::from_translation(at),
                );
            }
            for (index, content) in cell.contents.iter().enumerate() {
                spawn_mesh(
                    commands,
                    tile.clone(),
                    assets.materials.add(textured(
                        assets.tiles.content(*content),
                        theme.color_alpha(content_color(*content), 0.9),
                    )),
                    Transform::from_translation(at + Vec3::Y * (0.2 + index as f32 * 0.2)),
                );
            }
            if view.label_cells {
                labels.push((
                    at + Vec3::new(-0.38, 0.0, -0.38) * CELL,
                    Some(cell.at),
                    format!("{},{}", cell.at.0, cell.at.1),
                    format!("{TILE_PREFIX} {} {}", cell.at.0, cell.at.1),
                    UiColor::Label,
                    10.0,
                ));
            }
        } else {
            spawn_mesh(
                commands,
                tile.clone(),
                assets.materials.add(textured(
                    assets.tiles.outline.clone(),
                    theme.color_alpha(UiColor::Secondary, 0.6),
                )),
                Transform::from_translation(at),
            );
        }
        if view.selected == Some(cell.at) {
            spawn_mesh(
                commands,
                tile.clone(),
                assets.materials.add(textured(
                    assets.tiles.outline.clone(),
                    theme.color(UiColor::Accent),
                )),
                Transform::from_translation(at + Vec3::Y * 1.2),
            );
        }
    }

    // Marks: the game's meshes, side by side in their cell.
    let station_mesh = assets.meshes.add(Cuboid::new(9.0, 9.0, 9.0));
    let refusal_ring = assets.meshes.add(Torus::new(8.0, 9.5));
    let wreck_mesh = assets.meshes.add(Tetrahedron::default());
    let job_mesh = assets.meshes.add(Cuboid::new(4.0, 14.0, 4.0));
    for cell in &view.cells {
        let count = cell.marks.len();
        for (slot, mark) in cell.marks.iter().enumerate() {
            let shift = (slot as f32 - (count - 1) as f32 * 0.5) * MARK_SPACING;
            let at = cell_position(cell.at, view.player) + Vec3::new(shift, MARK_LIFT, 0.0);
            match mark {
                MarkView::Station { kind, code } => {
                    let color = station_color(*kind);
                    let alpha = if *kind == StationKind::Dead { 0.3 } else { 1.0 };
                    spawn_mesh(
                        commands,
                        station_mesh.clone(),
                        assets.materials.add(unlit(theme.color_alpha(color, alpha))),
                        Transform::from_translation(at),
                    );
                    if *kind == StationKind::Refusing {
                        spawn_mesh(
                            commands,
                            refusal_ring.clone(),
                            assets.materials.add(unlit(theme.color(UiColor::Danger))),
                            Transform::from_translation(at - Vec3::Y * 4.0),
                        );
                    }
                    // A station with open jobs counts them on its own label.
                    let text = match kind {
                        StationKind::Posting(n) => format!("{code} ({n})"),
                        _ => code.clone(),
                    };
                    labels.push((at, Some(cell.at), text, code.clone(), color, 12.0));
                }
                MarkView::Wreck(code) => {
                    spawn_mesh(
                        commands,
                        wreck_mesh.clone(),
                        assets
                            .materials
                            .add(unlit(theme.color(UiColor::AccentHigh))),
                        Transform::from_translation(at).with_scale(Vec3::splat(10.0)),
                    );
                    labels.push((
                        at,
                        Some(cell.at),
                        code.clone(),
                        code.clone(),
                        UiColor::AccentHigh,
                        12.0,
                    ));
                }
                MarkView::Job(code) => {
                    spawn_mesh(
                        commands,
                        job_mesh.clone(),
                        assets.materials.add(unlit(theme.color(UiColor::Accent))),
                        Transform::from_translation(at + Vec3::Y * 3.0),
                    );
                    labels.push((
                        at,
                        Some(cell.at),
                        code.clone(),
                        code.clone(),
                        UiColor::Accent,
                        12.0,
                    ));
                }
            }
        }
    }

    // The own ship, at its cell's centre.
    let you = Vec3::Y * MARK_LIFT * 0.5;
    spawn_mesh(
        commands,
        assets.meshes.add(Sphere::new(3.5)),
        assets.materials.add(unlit(theme.color(UiColor::Primary))),
        Transform::from_translation(you),
    );
    labels.push((
        you,
        None,
        view.you.to_string(),
        view.you.to_string(),
        UiColor::Primary,
        11.0,
    ));

    for (anchor, selects, text, name, color, size) in labels {
        spawn_chart_label(
            commands,
            node_entity,
            anchor,
            selects,
            &text,
            &name,
            color,
            size,
        );
    }

    commands.entity(node_entity).insert((
        SceneTarget {
            camera,
            root,
            image,
        },
        ChartScene {
            kind,
            player: view.player,
            reach: view.reach,
            primary: kind == SceneKind::Chart,
            clicks: kind == SceneKind::Chart,
        },
    ));
    commands.entity(node_entity).observe(on_chart_click);
    node_entity
}

/// One label under the viewport, hidden until projected. A label that
/// selects a cell is a button named `name`.
#[expect(
    clippy::too_many_arguments,
    reason = "a label's place, selection, text, name, colour and size under its viewport"
)]
fn spawn_chart_label(
    commands: &mut Commands,
    viewport: Entity,
    anchor: Vec3,
    selects: Option<(i32, i32)>,
    text: &str,
    name: &str,
    color: UiColor,
    size: f32,
) {
    let clickable = selects.is_some();
    let mut label = commands.spawn((
        ChartLabel {
            scene: viewport,
            anchor,
            selects,
        },
        Name::new(name.to_string()),
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        Visibility::Hidden,
        ZIndex(i32::from(clickable)),
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Void, 0.8),
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(UiColor::Secondary, if clickable { 0.3 } else { 0.0 }),
        ChildOf(viewport),
        children![(
            UiText,
            Text::new(text.to_string()),
            TextFont {
                font_size: FontSize::Px(size),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(color),
            Pickable::IGNORE
        )],
    ));
    if clickable {
        label.insert((
            bevy::ui_widgets::Button,
            Hovered::default(),
            Click::Tile(selects.expect("clickable labels select a cell")),
            bevy::ui_widgets::observe(crate::on_click),
        ));
    } else {
        label.insert(Pickable::IGNORE);
    }
}

/// A click on the chart's floor selects the cell under the pointer, or
/// clears the selection when that cell is the selected one: the pointer's
/// place in the node is the camera's viewport pixel, the ray from it meets
/// the floor plane, and the floor is cells.
fn on_chart_click(
    click: On<Pointer<bevy::picking::events::Click>>,
    scenes: Query<(&ChartScene, &SceneTarget, &ComputedNode)>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut chart: ResMut<ChartState>,
) {
    if click.button != PointerButton::Primary {
        return;
    }
    let Ok((scene, target, computed)) = scenes.get(click.entity) else {
        return;
    };
    if !scene.clicks || click.original_event_target() != click.entity {
        return;
    }
    let Ok((camera, cam_gt)) = cameras.get(target.camera) else {
        return;
    };
    let Some(hit) = click.hit.position else {
        return;
    };
    // The hit is normalised across the node, (-0.5, -0.5) top left; the
    // render target's pixels are the node's physical pixels.
    let pixel = (hit.truncate() + Vec2::splat(0.5)) * computed.size();
    let Ok(ray) = camera.viewport_to_world(cam_gt, pixel) else {
        return;
    };
    let Some(distance) = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y)) else {
        return;
    };
    let point = ray.get_point(distance);
    let dx = (point.x / CELL).round() as i32;
    let dz = (point.z / CELL).round() as i32;
    if dx.abs() > scene.reach || dz.abs() > scene.reach {
        return;
    }
    let at = (scene.player.0 + dx, scene.player.1 + dz);
    chart.selected = (chart.selected != Some(at)).then_some(at);
}

/// Despawn the camera and the scene a viewport node owned when the node
/// goes.
pub fn on_scene_target_removed(
    removed: On<Remove, SceneTarget>,
    targets: Query<&SceneTarget>,
    mut commands: Commands,
) {
    if let Ok(target) = targets.get(removed.entity) {
        commands.entity(target.camera).try_despawn();
        commands.entity(target.root).try_despawn();
    }
}

/// Keep each render target sized 1:1 to its node, and tell its camera.
pub fn reconcile_scene_targets(
    mut images: ResMut<Assets<Image>>,
    mut viewports: Query<(&ComputedNode, &mut ImageNode, &SceneTarget)>,
    mut cameras: Query<&mut Projection>,
) {
    for (computed, mut node, target) in &mut viewports {
        let desired = computed.size().round().as_uvec2();
        if desired.cmpeq(UVec2::ZERO).any() {
            continue;
        }
        if node.image != target.image {
            node.image = target.image.clone();
        }
        let needs_resize = images
            .get(&target.image)
            .map(|img| img.size() != desired)
            .unwrap_or(true);
        if !needs_resize {
            continue;
        }
        if let Some(mut img) = images.get_mut(&target.image) {
            img.resize(Extent3d {
                width: desired.x,
                height: desired.y,
                depth_or_array_layers: 1,
            });
        }
        // A resized image is not a changed target to bevy's camera system;
        // touching the projection makes it re-derive the target size.
        if let Ok(mut projection) = cameras.get_mut(target.camera) {
            projection.set_changed();
        }
    }
}

/// Read the pointer and the keys into the scene they aim at: the dragged
/// scene for the right button, the hovered one for the wheel, and the
/// hovered or the page's primary scene for the keys. Keys stay with a
/// focused text field.
#[expect(
    clippy::too_many_arguments,
    reason = "the pointer, the wheel, the keys and every scene they may aim at"
)]
pub fn chart_input(
    time: Res<Time>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    typing: Query<(), With<TextFieldFocused>>,
    mut pointer: ResMut<ChartPointer>,
    mut orbits: ResMut<ChartOrbits>,
    scenes: Query<(Entity, &ChartScene, &SceneTarget, &Hovered)>,
    cameras: Query<&Transform, With<Camera3d>>,
) {
    let hovered = scenes
        .iter()
        .find(|(_, _, _, hovered)| hovered.get())
        .map(|(entity, ..)| entity);
    if mouse.just_pressed(MouseButton::Right) {
        pointer.drag = hovered;
    }
    if !mouse.pressed(MouseButton::Right) {
        pointer.drag = None;
    }
    let dt = time.delta_secs().min(0.25);
    let key_target = hovered.or_else(|| {
        scenes
            .iter()
            .find(|(_, scene, ..)| scene.primary)
            .map(|(entity, ..)| entity)
    });
    for (entity, scene, target, _) in &scenes {
        let orbit = orbits
            .0
            .entry(scene.kind)
            .or_insert_with(|| Orbit::framed(scene.kind, scene.reach));
        if pointer.drag == Some(entity) {
            orbit.drag(motion.delta);
        }
        if hovered == Some(entity) && scroll.delta.y != 0.0 {
            let notches = match scroll.unit {
                MouseScrollUnit::Line => scroll.delta.y,
                MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
            };
            orbit.radius =
                (orbit.radius * (1.0 - notches * ZOOM_PER_NOTCH)).clamp(RADIUS_MIN, RADIUS_MAX);
        }
        if key_target != Some(entity) || !typing.is_empty() {
            continue;
        }
        let turn = TURN_RATE * dt;
        if keys.pressed(KeyCode::KeyQ) {
            orbit.theta += turn;
        }
        if keys.pressed(KeyCode::KeyE) {
            orbit.theta -= turn;
        }
        if keys.pressed(KeyCode::KeyR) {
            orbit.phi = (orbit.phi + turn).min(PHI_MAX);
        }
        if keys.pressed(KeyCode::KeyF) {
            orbit.phi = (orbit.phi - turn).max(PHI_MIN);
        }
        let mut pan = Vec2::ZERO;
        if keys.pressed(KeyCode::KeyW) {
            pan.y += 1.0;
        }
        if keys.pressed(KeyCode::KeyS) {
            pan.y -= 1.0;
        }
        if keys.pressed(KeyCode::KeyA) {
            pan.x -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) {
            pan.x += 1.0;
        }
        if pan != Vec2::ZERO {
            if let Ok(transform) = cameras.get(target.camera) {
                let flatten = |v: Vec3| Vec3::new(v.x, 0.0, v.z).normalize_or_zero();
                let forward = flatten(*transform.forward());
                let right = flatten(*transform.right());
                let speed = orbit.radius * 0.8 * dt;
                orbit.center += (forward * pan.y + right * pan.x) * speed;
            }
        }
        if keys.just_pressed(KeyCode::KeyT) {
            *orbit = Orbit::framed(scene.kind, scene.reach);
        }
    }
}

/// Put each chart camera on its orbit.
pub fn drive_chart_cameras(
    mut orbits: ResMut<ChartOrbits>,
    scenes: Query<(&ChartScene, &SceneTarget)>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    for (scene, target) in &scenes {
        let orbit = orbits
            .0
            .entry(scene.kind)
            .or_insert_with(|| Orbit::framed(scene.kind, scene.reach));
        if let Ok(mut transform) = cameras.get_mut(target.camera) {
            let wanted = Transform::from_translation(orbit.eye()).looking_at(orbit.center, Vec3::Y);
            if *transform != wanted {
                *transform = wanted;
            }
        }
    }
}

/// Place each label beside its anchor through its scene's camera, stacked
/// clear of its scene's other labels the way the game's chart stacks them,
/// and hide the ones whose anchor is off the viewport or whose stack runs
/// past its bottom. The selected cell's labels wear the accent.
pub fn project_chart_labels(
    chart: Res<ChartState>,
    scenes: Query<(&SceneTarget, &ComputedNode), With<ChartScene>>,
    cameras: Query<(&Camera, &Transform)>,
    mut labels: Query<(
        Entity,
        &ChartLabel,
        &ComputedNode,
        &mut Node,
        &mut Visibility,
        &mut ThemedBorder,
    )>,
) {
    let mut shown: HashMap<Entity, (Vec2, Vec<(Entity, Vec2, Vec2)>)> = HashMap::default();
    for (entity, label, label_node, _, mut visibility, _) in &mut labels {
        let Ok((target, computed)) = scenes.get(label.scene) else {
            continue;
        };
        let Ok((camera, transform)) = cameras.get(target.camera) else {
            continue;
        };
        // The camera re-derives its target size a frame after a resize; a
        // stale camera projects into the placeholder, so wait for it.
        if camera.physical_target_size() != Some(computed.size().round().as_uvec2()) {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        }
        // A chart camera stands at the root and turns this frame; its
        // `GlobalTransform` propagates after the UI lays the labels out.
        let pose = GlobalTransform::from(*transform);
        let to_logical = computed.inverse_scale_factor();
        let size = computed.size() * to_logical;
        let projected = camera
            .world_to_viewport(&pose, label.anchor)
            .ok()
            .map(|p| p * to_logical)
            .filter(|p| p.x >= 0.0 && p.y >= 0.0 && p.x <= size.x && p.y <= size.y);
        let Some(p) = projected else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        shown
            .entry(label.scene)
            .or_insert_with(|| (size, Vec::new()))
            .1
            .push((
                entity,
                p,
                label_node.size() * label_node.inverse_scale_factor(),
            ));
    }
    for (size, mut group) in shown.into_values() {
        group.sort_by(|a, b| a.1.y.total_cmp(&b.1.y).then(a.1.x.total_cmp(&b.1.x)));
        let placing: Vec<(Vec2, Vec2)> = group.iter().map(|&(_, p, extent)| (p, extent)).collect();
        for (&(entity, _, extent), corner) in group.iter().zip(stack_chart_labels(&placing)) {
            let Ok((_, label, _, mut node, mut visibility, mut border)) = labels.get_mut(entity)
            else {
                continue;
            };
            if corner.y + extent.y > size.y {
                visibility.set_if_neq(Visibility::Hidden);
                continue;
            }
            let (left, top) = (Val::Px(corner.x), Val::Px(corner.y));
            if node.left != left || node.top != top {
                node.left = left;
                node.top = top;
            }
            visibility.set_if_neq(Visibility::Inherited);
            if label.selects.is_some() {
                let selected = label.selects == chart.selected;
                let (color, alpha) = if selected {
                    (UiColor::Accent, 1.0)
                } else {
                    (UiColor::Secondary, 0.3)
                };
                if border.color != color || border.alpha != alpha {
                    border.color = color;
                    border.alpha = alpha;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Hull cards
// ---------------------------------------------------------------------------

/// Spawn a hull viewport under `parent`: a lit, slowly spinning silhouette
/// of a few cuboids standing in for the real hull preview of role `index`.
pub fn spawn_hull_scene(
    parent: &mut ChildSpawnerCommands,
    assets: &mut SceneAssets,
    index: usize,
    hull: Hull,
    node: Node,
    name: &str,
) {
    let kind = SceneKind::Hull(index);
    let image = render_target(&mut assets.images);
    let node_entity = parent
        .spawn((
            Name::new(name.to_string()),
            Node {
                overflow: Overflow::clip(),
                ..node
            },
            ImageNode::new(image.clone()),
            Pickable::IGNORE,
        ))
        .id();
    let commands = parent.commands_mut();
    let theme = &*assets.theme;
    let layer = RenderLayers::layer(kind.layer());
    let root = commands
        .spawn((
            HullSpin(0.45),
            Transform::from_rotation(Quat::from_rotation_y(index as f32 * 0.9)),
            Visibility::Visible,
        ))
        .id();
    let eye = Vec3::new(26.0, 14.0, 30.0);
    let camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: kind.order(),
                clear_color: ClearColorConfig::Custom(theme.color(UiColor::Surface)),
                ..default()
            },
            RenderTarget::Image(ImageRenderTarget {
                handle: image.clone(),
                scale_factor: 1.0,
            }),
            Transform::from_translation(eye).looking_at(Vec3::ZERO, Vec3::Y),
            AmbientLight {
                color: Color::WHITE,
                brightness: 250.0,
                affects_lightmapped_meshes: false,
            },
            layer.clone(),
        ))
        .id();
    commands.spawn((
        DirectionalLight {
            illuminance: 6_000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_translation(Vec3::new(1.0, 2.0, 0.6)).looking_at(Vec3::ZERO, Vec3::Y),
        layer.clone(),
        ChildOf(root),
    ));

    let lit = |color: UiColor| StandardMaterial {
        base_color: theme.color(color).with_alpha(1.0),
        perceptual_roughness: 0.75,
        metallic: 0.15,
        ..default()
    };
    let plate = assets.materials.add(lit(UiColor::Body));
    let part = assets.materials.add(lit(UiColor::Accent));
    let drive = assets.materials.add(lit(UiColor::Info));
    // (size, offset, material): the hull plating, the fitted parts and the
    // drives, so a cargoa reads as a boxy hauler and a racer as a dart.
    let blocks: Vec<(Vec3, Vec3, Handle<StandardMaterial>)> = match hull {
        Hull::Cargoa => vec![
            (Vec3::new(22.0, 6.0, 8.0), Vec3::ZERO, plate.clone()),
            (
                Vec3::new(10.0, 8.0, 12.0),
                Vec3::new(-2.0, 1.0, 0.0),
                plate.clone(),
            ),
            (
                Vec3::new(5.0, 3.0, 5.0),
                Vec3::new(7.5, 4.5, 0.0),
                part.clone(),
            ),
            (
                Vec3::new(2.0, 2.0, 2.0),
                Vec3::new(2.0, 6.0, 4.0),
                part.clone(),
            ),
            (
                Vec3::new(2.0, 2.0, 2.0),
                Vec3::new(2.0, 6.0, -4.0),
                part.clone(),
            ),
            (
                Vec3::new(6.0, 3.0, 3.0),
                Vec3::new(-13.0, 0.0, 3.0),
                drive.clone(),
            ),
            (
                Vec3::new(6.0, 3.0, 3.0),
                Vec3::new(-13.0, 0.0, -3.0),
                drive.clone(),
            ),
        ],
        Hull::Racer => vec![
            (Vec3::new(26.0, 3.5, 4.5), Vec3::ZERO, plate.clone()),
            (
                Vec3::new(5.0, 2.5, 3.0),
                Vec3::new(5.0, 2.5, 0.0),
                part.clone(),
            ),
            (
                Vec3::new(8.0, 1.0, 14.0),
                Vec3::new(-6.0, 0.0, 0.0),
                plate.clone(),
            ),
            (
                Vec3::new(2.0, 5.0, 1.0),
                Vec3::new(-12.0, 3.0, 0.0),
                plate.clone(),
            ),
            (
                Vec3::new(2.0, 2.0, 2.0),
                Vec3::new(9.0, -0.5, 0.0),
                part.clone(),
            ),
            (
                Vec3::new(5.0, 3.0, 3.0),
                Vec3::new(-15.0, 0.0, 0.0),
                drive.clone(),
            ),
        ],
    };
    for (size, offset, material) in blocks {
        commands.spawn((
            Mesh3d(assets.meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(material),
            Transform::from_translation(offset),
            layer.clone(),
            ChildOf(root),
        ));
    }
    commands.entity(node_entity).insert((
        HullScene { role: index },
        SceneTarget {
            camera,
            root,
            image,
        },
    ));
}

/// Turn every spinning root about its mast at its own rate.
pub fn spin_hulls(time: Res<Time>, mut roots: Query<(&mut Transform, &HullSpin)>) {
    for (mut transform, spin) in &mut roots {
        transform.rotate_y(spin.0 * time.delta_secs());
    }
}

// ---------------------------------------------------------------------------
// Menu backdrop
// ---------------------------------------------------------------------------

/// Spawn the menu backdrop under `parent`: a few grey low-poly rocks that
/// drift slowly in the dark, as the menu's ambient scenario reads behind
/// the New Game modal. The node fills its parent.
pub fn spawn_backdrop_scene(parent: &mut ChildSpawnerCommands, assets: &mut SceneAssets) {
    let kind = SceneKind::Backdrop;
    let image = render_target(&mut assets.images);
    let node_entity = parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                width: percent(100),
                height: percent(100),
                overflow: Overflow::clip(),
                ..default()
            },
            ImageNode::new(image.clone()),
            Pickable::IGNORE,
        ))
        .id();
    let commands = parent.commands_mut();
    let theme = &*assets.theme;
    let layer = RenderLayers::layer(kind.layer());
    let root = commands
        .spawn((HullSpin(0.02), Transform::IDENTITY, Visibility::Visible))
        .id();
    let camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: kind.order(),
                clear_color: ClearColorConfig::Custom(theme.color(UiColor::Void)),
                ..default()
            },
            RenderTarget::Image(ImageRenderTarget {
                handle: image.clone(),
                scale_factor: 1.0,
            }),
            Transform::from_translation(Vec3::new(0.0, 30.0, 320.0))
                .looking_at(Vec3::ZERO, Vec3::Y),
            AmbientLight {
                color: theme.color(UiColor::Secondary),
                brightness: 60.0,
                affects_lightmapped_meshes: false,
            },
            layer.clone(),
        ))
        .id();
    commands.spawn((
        DirectionalLight {
            illuminance: 1_800.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_translation(Vec3::new(-1.0, 1.5, 1.0)).looking_at(Vec3::ZERO, Vec3::Y),
        layer.clone(),
        ChildOf(root),
    ));
    let rock = assets.materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.44, 0.43),
        perceptual_roughness: 0.95,
        metallic: 0.0,
        ..default()
    });
    // (radius, position, scale): the two big rocks either side of the modal
    // and the small ones that read as the cluster's drift.
    let rocks = [
        (
            46.0,
            Vec3::new(-150.0, -10.0, -40.0),
            Vec3::new(1.2, 0.8, 1.0),
        ),
        (
            40.0,
            Vec3::new(140.0, 20.0, -60.0),
            Vec3::new(1.0, 0.7, 1.3),
        ),
        (
            12.0,
            Vec3::new(-40.0, -70.0, 60.0),
            Vec3::new(1.0, 0.9, 1.1),
        ),
        (8.0, Vec3::new(60.0, 60.0, -120.0), Vec3::new(1.3, 1.0, 0.8)),
        (
            16.0,
            Vec3::new(220.0, -60.0, -200.0),
            Vec3::new(1.0, 1.0, 1.0),
        ),
        (
            6.0,
            Vec3::new(-230.0, 70.0, -150.0),
            Vec3::new(1.0, 0.8, 1.2),
        ),
    ];
    for (i, (radius, at, scale)) in rocks.into_iter().enumerate() {
        commands.spawn((
            Mesh3d(
                assets
                    .meshes
                    .add(Sphere::new(radius).mesh().ico(1).expect("ico(1) is valid")),
            ),
            MeshMaterial3d(rock.clone()),
            Transform::from_translation(at)
                .with_scale(scale)
                .with_rotation(Quat::from_euler(
                    EulerRot::XYZ,
                    i as f32 * 0.7,
                    i as f32 * 1.3,
                    0.4,
                )),
            layer.clone(),
            ChildOf(root),
        ));
    }
    commands.entity(node_entity).insert(SceneTarget {
        camera,
        root,
        image,
    });
}
