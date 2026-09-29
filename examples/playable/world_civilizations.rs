//! world_civilizations: look at the seeded civilizations a world's ships will
//! be drawn from.
//!
//! One flat plane through the world origin, 2,560 km across, sampled on the
//! CPU at 8 km a sample through the same `CivilizationField` the generator
//! will read, and painted as six panels. Nothing streams and no ship is drawn:
//! these are the maps a tuning pass reads before any civilization fields a
//! ship.
//!
//! # Reading the picture
//!
//! - Primary civilization: the highest selection weight, one colour per
//!   identity, brighter where its share is higher. Black lines mark where the
//!   primary changes; white dots mark centroids within 120 km of the plane.
//! - Civilizations in reach: how many centroids are within the 320 km reach,
//!   0 to 16.
//! - Advancement, linear and smoothstep: the selection-weighted mean
//!   advancement, 0 to 1, under each curve.
//! - Living share: the selection-weight share of living civilizations, 0 to 1.
//! - Dominant role: the role with the largest selection-weighted share BEFORE
//!   content eligibility, brighter where it dominates more. Scavenger and
//!   armored are also hatched.
//!
//! Every panel carries rings at 500 and 1,000 km from the origin. Bright red is
//! a place with no civilization in reach, which the lattice bounds forbid.
//!
//! # Hand-run
//!
//! ```text
//! cargo run --example world_civilizations --features debug
//! ```
//!
//! | key | what it does |
//! | - | - |
//! | 1 / 2 / 3 | world seed 20260922 / 12345 / 4000000000 |
//! | X / Y / Z | the XY, XZ or YZ plane through the origin |
//! | R | back to the opening view |
//!
//! Harnessed mode:
//! - `NOVA_AUTOPILOT=1`: walk the three seeds on the three planes and exit
//!   clean.
//! - `NOVA_CAPTURE=1`: also writes `world-civilizations-<seed>-<plane>.png`
//!   for each of the nine views and `world-civilizations-report.md`.
//!
//! Every run logs the same report at boot: coverage, reach, primary share,
//! living share, advancement and pre-eligibility role mix on shells at 0,
//! 500, 1,000 and 1,250 km, civilization counts within 1,250 km, sample names
//! and every name collision.

#[path = "../shared/world_fixture/mod.rs"]
pub mod world_fixture;

use std::collections::BTreeMap;

use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    tasks::{block_on, poll_once, AsyncComputeTaskPool, Task},
};
use clap::Parser;
use nova_protocol::prelude::*;
use nova_world::prelude::*;
use world_fixture::free_play_scenario;

#[derive(Parser)]
#[command(name = "world_civilizations")]
#[command(version = "1.0.0")]
#[command(
    about = "Paint the seeded civilization field on planes through the world origin",
    long_about = None
)]
struct Cli;

/// The empty bootstrap this session runs inside. The field is a pure function
/// of the seed, so nothing streams.
const SCENARIO_ID: &str = "world_civilizations_observer";

/// The three world seeds every view and the report compare.
const SEEDS: [u32; 3] = [20_260_922, 12_345, 4_000_000_000];

/// Samples across one panel.
const PANEL_PIXELS: u32 = 320;

/// World covered by one panel on both axes: 8 km a sample, from 1,280 km out
/// on one side of the origin to 1,280 km on the other, past the 1,000 km
/// advancement saturation.
const PANEL_EXTENT: Meters = Meters(2_560_000.0);

/// The rings every panel carries, in meters from the origin.
const RINGS: [f32; 2] = [500_000.0, 1_000_000.0];

/// The civilization lattice spacing and per-axis jitter, in meters, mirrored
/// from `crates/nova_world_base/src/civilizations.rs` to bound the centroid
/// scans below. The field's own queries never read these.
const LATTICE: f32 = 240_000.0;
const JITTER: f32 = 40_000.0;

/// How far off the plane a centroid may sit and still be dotted: half the
/// lattice, so each centroid near the plane is dotted once.
const CENTROID_DOT_DEPTH: f32 = LATTICE * 0.5;

/// The reach count the reach panel saturates at.
const REACH_COUNT_MAX: f32 = 16.0;

/// The report's shells, in meters from the origin. Radius zero is the origin
/// itself.
const REPORT_SHELLS: [f32; 4] = [0.0, 500_000.0, 1_000_000.0, 1_250_000.0];

/// Points on each report shell past the origin.
const REPORT_SHELL_POINTS: usize = 409;

/// The centroid radius the report counts civilizations inside.
const REPORT_CENSUS_RADIUS: f32 = 1_250_000.0;

/// A role preference at or above this is a specialty: an unspecialized draw is
/// `0.5 + [0, 1)` (`crates/nova_world_base/src/civilizations.rs`).
const SPECIALTY_PREFERENCE: f32 = 1.5;

/// How many sample names the report lists per seed.
const REPORT_SAMPLE_NAMES: usize = 12;

/// The report's file name under the capture dir.
#[cfg(feature = "debug")]
const REPORT_FILE: &str = "world-civilizations-report.md";

/// In-step seconds a harnessed paint gets before the run aborts naming it.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 120.0;

/// The chart surface every ramp starts from.
const SURFACE: Srgba = Srgba::rgb(0.102, 0.102, 0.098);

/// The bright end of the reach ramp: blue.
const REACH_COLOUR: Srgba = Srgba::rgb(0.224, 0.529, 0.898);

/// The bright end of both advancement ramps: orange, the same on both so the
/// two curves compare directly.
const ADVANCEMENT_COLOUR: Srgba = Srgba::rgb(0.851, 0.349, 0.149);

/// The bright end of the living ramp: aqua.
const LIVING_COLOUR: Srgba = Srgba::rgb(0.098, 0.620, 0.439);

/// The role colours, in `ShipRoleType::ALL` order. Four hues cannot all stay
/// apart under colour blindness, so the two fighter roles are also hatched in
/// opposite directions.
const ROLE_COLOURS: [Srgba; 4] = [
    Srgba::rgb(0.224, 0.529, 0.898),
    Srgba::rgb(0.788, 0.522, 0.0),
    Srgba::rgb(0.835, 0.318, 0.506),
    Srgba::rgb(0.098, 0.620, 0.439),
];

/// A place with no civilization in reach.
const UNCOVERED: Srgba = Srgba::rgb(1.0, 0.0, 0.0);

/// Which pair of axes the painted plane spans. Every plane passes through the
/// origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlicePlane {
    /// Horizontal +X, vertical +Y, Z = 0.
    Xy,
    /// Horizontal +X, vertical +Z, Y = 0.
    Xz,
    /// Horizontal +Y, vertical +Z, X = 0.
    Yz,
}

impl SlicePlane {
    const ALL: [Self; 3] = [Self::Xy, Self::Xz, Self::Yz];

    const fn label(self) -> &'static str {
        match self {
            Self::Xy => "XY",
            Self::Xz => "XZ",
            Self::Yz => "YZ",
        }
    }

    #[cfg(feature = "debug")]
    const fn slug(self) -> &'static str {
        match self {
            Self::Xy => "xy",
            Self::Xz => "xz",
            Self::Yz => "yz",
        }
    }

    /// The world point `horizontal` and `vertical` meters across the plane.
    fn point(self, horizontal: f32, vertical: f32) -> Meters3 {
        match self {
            Self::Xy => Meters3::new(horizontal, vertical, 0.0),
            Self::Xz => Meters3::new(horizontal, 0.0, vertical),
            Self::Yz => Meters3::new(0.0, horizontal, vertical),
        }
    }

    /// A world point's `(horizontal, vertical, off-plane)` coordinates.
    fn project(self, point: Vec3) -> (f32, f32, f32) {
        match self {
            Self::Xy => (point.x, point.y, point.z),
            Self::Xz => (point.x, point.z, point.y),
            Self::Yz => (point.y, point.z, point.x),
        }
    }
}

/// Which view is on the screen.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
struct CivilizationView {
    /// Index into [`SEEDS`].
    seed: usize,
    /// Which plane through the origin is painted.
    plane: SlicePlane,
}

impl CivilizationView {
    const OPENING: Self = Self {
        seed: 0,
        plane: SlicePlane::Xy,
    };

    #[cfg(feature = "debug")]
    fn shot(self) -> String {
        format!(
            "world-civilizations-{}-{}.png",
            SEEDS[self.seed],
            self.plane.slug()
        )
    }
}

/// The six panels, in the order they are laid out and painted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PanelType {
    Primary,
    ReachCount,
    AdvancementLinear,
    AdvancementSmoothstep,
    LivingShare,
    DominantRole,
}

impl PanelType {
    const ALL: [Self; 6] = [
        Self::Primary,
        Self::ReachCount,
        Self::AdvancementLinear,
        Self::AdvancementSmoothstep,
        Self::LivingShare,
        Self::DominantRole,
    ];

    const fn title(self) -> &'static str {
        match self {
            Self::Primary => "primary civilization",
            Self::ReachCount => "civilizations in reach, 0-16",
            Self::AdvancementLinear => "advancement, linear, 0-1",
            Self::AdvancementSmoothstep => "advancement, smoothstep, 0-1",
            Self::LivingShare => "living share, 0-1",
            Self::DominantRole => "dominant role, pre-eligibility",
        }
    }
}

/// The painted panels' handles and the view they answer for.
///
/// The view is kept beside the handles so a repaint is driven by comparing
/// against what is ON SCREEN, not by change detection on a resource inserted
/// during boot.
#[derive(Resource)]
struct CivilizationPainting {
    handles: [Handle<Image>; 6],
    painted: Option<CivilizationView>,
    summary: String,
}

/// One view being painted on a worker.
#[derive(Resource)]
struct CivilizationJob {
    view: CivilizationView,
    task: Task<CivilizationPixels>,
}

/// A finished paint: six panels of texels and a readout of what they hold.
struct CivilizationPixels {
    panels: [Vec<u8>; 6],
    summary: String,
}

/// Marks the readout line.
#[derive(Component)]
struct CivilizationReadout;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new()
        .with_game_plugins(civilizations_plugin)
        .build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        // The fleet's 16:9, so the six panels and the readout fit one shot.
        app.add_systems(Startup, force_capture_resolution);
        app.add_plugins(civilizations_script());
    }

    app.run()
}

fn civilizations_plugin(app: &mut App) {
    app.insert_resource(CivilizationView::OPENING);
    app.add_systems(OnEnter(GameAssetsStates::Loaded), boot_civilizations);
    app.add_systems(
        Update,
        (read_keys, request_paint, collect_paint, update_readout).chain(),
    );
}

/// Load the empty bootstrap, lay out the six blank panels and the readout,
/// and log the report.
fn boot_civilizations(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    game_assets: Res<GameAssets>,
) {
    commands.trigger(LoadScenario(free_play_scenario(
        &game_assets,
        SCENARIO_ID,
        "World Civilizations",
    )));

    info!("world civilizations report\n{}", civilization_report());

    let handles: [Handle<Image>; 6] = std::array::from_fn(|_| images.add(blank_panel()));
    commands.insert_resource(CivilizationPainting {
        handles: handles.clone(),
        painted: None,
        summary: String::new(),
    });

    // Panels in a 3x2 grid at the left, the readout in a column to their
    // right, clear of the dev overlay's fps and version bar along the top.
    commands
        .spawn((
            Name::new("Civilization Panels"),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(12.0),
                width: Val::Px(3.0 * (PANEL_PIXELS as f32 + 12.0)),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(12.0),
                row_gap: Val::Px(8.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            for (panel, handle) in PanelType::ALL.into_iter().zip(handles) {
                parent
                    .spawn(Node {
                        width: Val::Px(PANEL_PIXELS as f32),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|column| {
                        column.spawn((
                            Text::new(panel.title()),
                            TextFont {
                                font_size: FontSize::Px(15.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                        column.spawn((
                            Name::new(panel.title()),
                            ImageNode::new(handle),
                            Node {
                                width: Val::Px(PANEL_PIXELS as f32),
                                height: Val::Px(PANEL_PIXELS as f32),
                                ..default()
                            },
                        ));
                    });
            }
        });
    commands.spawn((
        CivilizationReadout,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(40.0),
            left: Val::Px(3.0 * (PANEL_PIXELS as f32 + 12.0) + 24.0),
            max_width: Val::Px(860.0),
            ..default()
        },
    ));
}

/// A blank panel image, so each panel has something to point at before the
/// first paint lands.
fn blank_panel() -> Image {
    let texels = (PANEL_PIXELS * PANEL_PIXELS) as usize;
    let mut image = Image::new(
        Extent3d {
            width: PANEL_PIXELS,
            height: PANEL_PIXELS,
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

/// The hand affordance: pick a seed, pick a plane.
fn read_keys(keys: Res<ButtonInput<KeyCode>>, mut view: ResMut<CivilizationView>) {
    let mut next = *view;
    for (seed, key) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3]
        .into_iter()
        .enumerate()
    {
        if keys.just_pressed(key) {
            next.seed = seed;
        }
    }
    for (key, plane) in [KeyCode::KeyX, KeyCode::KeyY, KeyCode::KeyZ]
        .into_iter()
        .zip(SlicePlane::ALL)
    {
        if keys.just_pressed(key) {
            next.plane = plane;
        }
    }
    if keys.just_pressed(KeyCode::KeyR) {
        next = CivilizationView::OPENING;
    }
    if next != *view {
        *view = next;
    }
}

/// Start a paint when the view on screen is not the one asked for and no
/// paint for it is running.
fn request_paint(
    mut commands: Commands,
    view: Res<CivilizationView>,
    painting: Option<Res<CivilizationPainting>>,
    job: Option<Res<CivilizationJob>>,
) {
    let Some(painting) = painting else {
        return;
    };
    if painting.painted == Some(*view) || job.is_some_and(|job| job.view == *view) {
        return;
    }
    let wanted = *view;
    let task = AsyncComputeTaskPool::get().spawn(async move { paint_view(wanted) });
    commands.insert_resource(CivilizationJob { view: wanted, task });
}

/// Take a finished paint and write it into the six panel images.
fn collect_paint(
    mut commands: Commands,
    mut job: Option<ResMut<CivilizationJob>>,
    mut painting: Option<ResMut<CivilizationPainting>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Some(job), Some(painting)) = (job.as_mut(), painting.as_mut()) else {
        return;
    };
    let Some(pixels) = block_on(poll_once(&mut job.task)) else {
        return;
    };
    let view = job.view;
    commands.remove_resource::<CivilizationJob>();

    for (handle, texels) in painting.handles.clone().iter().zip(pixels.panels) {
        let Some(mut image) = images.get_mut(handle) else {
            return;
        };
        image.data = Some(texels);
    }
    painting.painted = Some(view);
    painting.summary = pixels.summary;
    info!(
        "world civilizations: seed {} {} plane\n{}",
        SEEDS[view.seed],
        view.plane.label(),
        painting.summary
    );
}

/// What the field says at one place, reduced to what the panels paint.
struct PlaceReading {
    /// The primary civilization and its selection share.
    primary: (CivilizationId, f32),
    reach_count: usize,
    advancement_linear: f32,
    advancement_smoothstep: f32,
    living_share: f32,
    /// The selection-weighted role shares, before content eligibility.
    role_shares: [f32; 4],
    /// The distance to the nearest centroid.
    nearest: f32,
}

/// Read one place from both curves' fields and the environment.
///
/// `Ok(None)` is a place with no civilization in reach; every other refusal
/// panics, since a picture that painted it as some value would be a picture
/// of a bug.
fn read_place(
    linear: &CivilizationField,
    smooth: &CivilizationField,
    fields: &EnvironmentFields,
    position: Meters3,
) -> Option<PlaceReading> {
    let reach = match linear.in_reach(position) {
        Ok(reach) => reach,
        Err(SectorFault::Generation {
            field: "coverage", ..
        }) => return None,
        Err(fault) => panic!("world civilizations: {fault}"),
    };
    let environment = fields
        .sample(position)
        .unwrap_or_else(|fault| panic!("world civilizations: {fault}"));
    let total: f32 = reach.iter().map(|entry| entry.selection_weight).sum();

    let mut primary = (reach[0].civilization.id, 0.0);
    let mut advancement_linear = 0.0;
    let mut advancement_smoothstep = 0.0;
    let mut living_share = 0.0;
    let mut role_shares = [0.0; 4];
    let mut nearest = f32::INFINITY;
    for entry in &reach {
        let share = entry.selection_weight / total;
        let civilization = entry.civilization;
        if share > primary.1 {
            primary = (civilization.id, share);
        }
        advancement_linear += share * civilization.advancement;
        advancement_smoothstep += share * smooth.civilization(civilization.id.node).advancement;
        if civilization.status == CivilizationStatusType::Living {
            living_share += share;
        }
        let weights = civilization.role_weights(environment);
        let weight_total: f32 = weights.iter().sum();
        for (role_share, weight) in role_shares.iter_mut().zip(weights) {
            *role_share += share * weight / weight_total;
        }
        nearest = nearest.min(entry.distance.get());
    }

    Some(PlaceReading {
        primary,
        reach_count: reach.len(),
        advancement_linear,
        advancement_smoothstep,
        living_share,
        role_shares,
        nearest,
    })
}

/// Paint all six panels of one view.
///
/// PURE, and the whole cost of this example: one reach query and one
/// environment reading per sample, on a worker.
fn paint_view(view: CivilizationView) -> CivilizationPixels {
    let seed = SEEDS[view.seed];
    let linear = CivilizationField::new(seed, AdvancementCurveType::Linear);
    let smooth = CivilizationField::new(seed, AdvancementCurveType::Smoothstep);
    let fields = EnvironmentFields::new(seed);

    let pixels = PANEL_PIXELS as usize;
    let span = PANEL_EXTENT.get();
    let per_texel = span / PANEL_PIXELS as f32;
    let mut readings: Vec<Option<PlaceReading>> = Vec::with_capacity(pixels * pixels);
    for row in 0..pixels {
        // Row 0 is the TOP of the image, the highest vertical value.
        let vertical = span * 0.5 - (row as f32 + 0.5) * per_texel;
        for column in 0..pixels {
            let horizontal = -span * 0.5 + (column as f32 + 0.5) * per_texel;
            readings.push(read_place(
                &linear,
                &smooth,
                &fields,
                view.plane.point(horizontal, vertical),
            ));
        }
    }

    // Centroids near the plane, as texel positions.
    let mut dots = Vec::new();
    let nodes = ((span * 0.5 + JITTER) / LATTICE).ceil() as i32;
    for x in -nodes..=nodes {
        for y in -nodes..=nodes {
            for z in -nodes..=nodes {
                let centroid = linear.civilization([x, y, z]).centroid.get();
                let (horizontal, vertical, off) = view.plane.project(centroid);
                if off.abs() >= CENTROID_DOT_DEPTH {
                    continue;
                }
                let column = ((horizontal + span * 0.5) / per_texel).floor();
                let row = ((span * 0.5 - vertical) / per_texel).floor();
                if (0.0..pixels as f32).contains(&column) && (0.0..pixels as f32).contains(&row) {
                    dots.push((row as usize, column as usize));
                }
            }
        }
    }

    let mut panels: [Vec<u8>; 6] = std::array::from_fn(|_| Vec::with_capacity(pixels * pixels * 4));
    for row in 0..pixels {
        let vertical = span * 0.5 - (row as f32 + 0.5) * per_texel;
        for column in 0..pixels {
            let horizontal = -span * 0.5 + (column as f32 + 0.5) * per_texel;
            let ring = on_ring(horizontal, vertical, per_texel);
            let index = row * pixels + column;
            let reading = readings[index].as_ref();
            let border = reading.is_some_and(|reading| {
                let right = (column + 1 < pixels)
                    .then(|| readings[index + 1].as_ref())
                    .flatten();
                let below = (row + 1 < pixels)
                    .then(|| readings[index + pixels].as_ref())
                    .flatten();
                [right, below]
                    .into_iter()
                    .flatten()
                    .any(|other| other.primary.0 != reading.primary.0)
            });
            let dot = dots.iter().any(|&(dot_row, dot_column)| {
                dot_row.abs_diff(row) <= 1 && dot_column.abs_diff(column) <= 1
            });
            for (panel, texels) in PanelType::ALL.into_iter().zip(panels.iter_mut()) {
                let colour = match reading {
                    None => UNCOVERED,
                    Some(reading) => panel_colour(panel, reading, row, column, border, dot),
                };
                let colour = if ring {
                    lerp_srgba(colour, Srgba::rgb(0.8, 0.8, 0.8), 0.6)
                } else {
                    colour
                };
                texels.extend_from_slice(&texel(colour));
            }
        }
    }

    CivilizationPixels {
        panels,
        summary: view_summary(&readings),
    }
}

/// The colour one panel paints one reading.
fn panel_colour(
    panel: PanelType,
    reading: &PlaceReading,
    row: usize,
    column: usize,
    border: bool,
    dot: bool,
) -> Srgba {
    match panel {
        PanelType::Primary => {
            if dot {
                Srgba::WHITE
            } else if border {
                Srgba::BLACK
            } else {
                let (id, share) = reading.primary;
                lerp_srgba(SURFACE, identity_colour(id), 0.25 + 0.75 * share)
            }
        }
        PanelType::ReachCount => lerp_srgba(
            SURFACE,
            REACH_COLOUR,
            reading.reach_count as f32 / REACH_COUNT_MAX,
        ),
        PanelType::AdvancementLinear => {
            lerp_srgba(SURFACE, ADVANCEMENT_COLOUR, reading.advancement_linear)
        }
        PanelType::AdvancementSmoothstep => {
            lerp_srgba(SURFACE, ADVANCEMENT_COLOUR, reading.advancement_smoothstep)
        }
        PanelType::LivingShare => lerp_srgba(SURFACE, LIVING_COLOUR, reading.living_share),
        PanelType::DominantRole => {
            let (role, share) = dominant_role(reading.role_shares);
            // A share of one quarter is a tie of all four; one is sole.
            let strength = ((share - 0.25) / 0.75).clamp(0.0, 1.0);
            let colour = lerp_srgba(SURFACE, ROLE_COLOURS[role], 0.35 + 0.65 * strength);
            let hatch = match ShipRoleType::ALL[role] {
                ShipRoleType::Scavenger => (row + column) % 6 == 0,
                ShipRoleType::Armored => (row + PANEL_PIXELS as usize - column) % 6 == 0,
                ShipRoleType::Civilian | ShipRoleType::Industrial => false,
            };
            if hatch {
                SURFACE
            } else {
                colour
            }
        }
    }
}

/// The largest role share and its index in `ShipRoleType::ALL` order.
fn dominant_role(shares: [f32; 4]) -> (usize, f32) {
    shares
        .into_iter()
        .enumerate()
        .fold((0, f32::NEG_INFINITY), |best, (role, share)| {
            if share > best.1 {
                (role, share)
            } else {
                best
            }
        })
}

/// A stable colour per civilization identity, so neighbouring regions read
/// apart. Hashed from the full identity, never from the name.
fn identity_colour(id: CivilizationId) -> Srgba {
    let hash = Fnv32::new()
        .write(&id.world_seed.to_le_bytes())
        .write(&id.node[0].to_le_bytes())
        .write(&id.node[1].to_le_bytes())
        .write(&id.node[2].to_le_bytes())
        .finish();
    let hue = (hash >> 16) as f32 / 65_536.0 * 360.0;
    let lightness = 0.45 + ((hash >> 8) & 0xff) as f32 / 255.0 * 0.2;
    Srgba::from(Color::hsl(hue, 0.7, lightness))
}

/// Whether a texel straddles one of the [`RINGS`].
fn on_ring(horizontal: f32, vertical: f32, per_texel: f32) -> bool {
    let radius = horizontal.hypot(vertical);
    RINGS
        .iter()
        .any(|ring| (radius - ring).abs() < per_texel * 0.5)
}

fn texel(colour: Srgba) -> [u8; 4] {
    [
        (colour.red * 255.0) as u8,
        (colour.green * 255.0) as u8,
        (colour.blue * 255.0) as u8,
        255,
    ]
}

fn lerp_srgba(from: Srgba, to: Srgba, t: f32) -> Srgba {
    let t = t.clamp(0.0, 1.0);
    Srgba::new(
        from.red + (to.red - from.red) * t,
        from.green + (to.green - from.green) * t,
        from.blue + (to.blue - from.blue) * t,
        1.0,
    )
}

/// What one painted view holds, in the terms the readout shows.
fn view_summary(readings: &[Option<PlaceReading>]) -> String {
    let covered: Vec<&PlaceReading> = readings.iter().flatten().collect();
    let stats = PlaceStats::of(&covered);
    let mut dominant = [0usize; 4];
    for reading in &covered {
        dominant[dominant_role(reading.role_shares).0] += 1;
    }
    let dominant = ShipRoleType::ALL
        .iter()
        .zip(dominant)
        .map(|(role, count)| {
            format!(
                "{} {:.1}%",
                role.label(),
                100.0 * count as f32 / covered.len().max(1) as f32
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "covered {} of {} samples\n\
         in reach min {} / median {} / max {}\n\
         primary share median {:.2}, nearest centroid at most {:.0} km\n\
         living share mean {:.2}\n\
         advancement mean linear {:.2}, smoothstep {:.2}\n\
         dominant role by area: {dominant}",
        covered.len(),
        readings.len(),
        stats.reach.0,
        stats.reach.1,
        stats.reach.2,
        stats.primary_median,
        stats.farthest_nearest / 1_000.0,
        stats.living,
        stats.advancement.0,
        stats.advancement.1,
    )
}

/// Summary numbers over a set of readings.
struct PlaceStats {
    /// Reach count min, median and max.
    reach: (usize, usize, usize),
    primary_median: f32,
    farthest_nearest: f32,
    living: f32,
    /// Mean advancement, linear then smoothstep.
    advancement: (f32, f32),
    /// Mean pre-eligibility role shares.
    roles: [f32; 4],
}

impl PlaceStats {
    fn of(readings: &[&PlaceReading]) -> Self {
        let count = readings.len().max(1) as f32;
        let mut reach: Vec<usize> = readings.iter().map(|reading| reading.reach_count).collect();
        reach.sort_unstable();
        let mut primary: Vec<f32> = readings.iter().map(|reading| reading.primary.1).collect();
        primary.sort_by(f32::total_cmp);
        let mean = |value: fn(&PlaceReading) -> f32| {
            readings.iter().map(|reading| value(reading)).sum::<f32>() / count
        };
        Self {
            reach: (
                reach.first().copied().unwrap_or(0),
                reach.get(reach.len() / 2).copied().unwrap_or(0),
                reach.last().copied().unwrap_or(0),
            ),
            primary_median: primary.get(primary.len() / 2).copied().unwrap_or(0.0),
            farthest_nearest: readings
                .iter()
                .map(|reading| reading.nearest)
                .fold(0.0, f32::max),
            living: mean(|reading| reading.living_share),
            advancement: (
                mean(|reading| reading.advancement_linear),
                mean(|reading| reading.advancement_smoothstep),
            ),
            roles: std::array::from_fn(|role| {
                readings
                    .iter()
                    .map(|reading| reading.role_shares[role])
                    .sum::<f32>()
                    / count
            }),
        }
    }
}

/// Points on a sphere of `radius`, spread by a golden-angle spiral. The
/// origin alone for radius zero.
fn shell(radius: f32) -> Vec<Meters3> {
    if radius == 0.0 {
        return vec![Meters3::new(0.0, 0.0, 0.0)];
    }
    (0..REPORT_SHELL_POINTS)
        .map(|index| {
            let height = 1.0 - 2.0 * (index as f32 + 0.5) / REPORT_SHELL_POINTS as f32;
            let ring = (1.0 - height * height).sqrt();
            let turn = index as f32 * 2.399_963;
            Meters3::new(ring * turn.cos(), height, ring * turn.sin()) * radius
        })
        .collect()
}

/// The count report for all three seeds, as Markdown.
fn civilization_report() -> String {
    let mut report = String::from(
        "# World civilizations\n\n\
         Provisional A1 field. Role mixes are BEFORE content eligibility. Shares are \
         selection-weight shares at each point.\n",
    );
    for seed in SEEDS {
        let linear = CivilizationField::new(seed, AdvancementCurveType::Linear);
        let smooth = CivilizationField::new(seed, AdvancementCurveType::Smoothstep);
        let fields = EnvironmentFields::new(seed);

        report.push_str(&format!(
            "\n## Seed {seed}\n\n\
             | shell km | points | covered | reach min/med/max | primary share med | \
             nearest max km | living | adv linear | adv smooth | civilian | industrial | \
             scavenger | armored |\n\
             | - | - | - | - | - | - | - | - | - | - | - | - | - |\n"
        ));
        for radius in REPORT_SHELLS {
            let points = shell(radius);
            let readings: Vec<PlaceReading> = points
                .iter()
                .filter_map(|&point| read_place(&linear, &smooth, &fields, point))
                .collect();
            let refs: Vec<&PlaceReading> = readings.iter().collect();
            let stats = PlaceStats::of(&refs);
            report.push_str(&format!(
                "| {:.0} | {} | {} | {}/{}/{} | {:.2} | {:.0} | {:.2} | {:.2} | {:.2} | {:.2} | \
                 {:.2} | {:.2} | {:.2} |\n",
                radius / 1_000.0,
                points.len(),
                readings.len(),
                stats.reach.0,
                stats.reach.1,
                stats.reach.2,
                stats.primary_median,
                stats.farthest_nearest / 1_000.0,
                stats.living,
                stats.advancement.0,
                stats.advancement.1,
                stats.roles[0],
                stats.roles[1],
                stats.roles[2],
                stats.roles[3],
            ));
        }

        let census = census(&linear);
        let living = census
            .iter()
            .filter(|civilization| civilization.status == CivilizationStatusType::Living)
            .count();
        let specialists = ShipRoleType::ALL
            .iter()
            .enumerate()
            .map(|(role, label)| {
                let count = census
                    .iter()
                    .filter(|civilization| {
                        civilization.role_preference[role] >= SPECIALTY_PREFERENCE
                    })
                    .count();
                format!("{} {count}", label.label())
            })
            .collect::<Vec<_>>()
            .join(", ");
        let zero = |role: usize| {
            census
                .iter()
                .filter(|civilization| civilization.role_preference[role] == 0.0)
                .count()
        };
        report.push_str(&format!(
            "\nCivilizations with a centroid within {:.0} km: {} ({living} living, {} \
             extinct). At specialist preference (1.5 or more; a fighter specialty the \
             fighter-absence draw zeroed is not counted): {specialists}. Zero scavenger \
             preference: {}. Zero armored preference: {}.\n",
            REPORT_CENSUS_RADIUS / 1_000.0,
            census.len(),
            census.len() - living,
            zero(2),
            zero(3),
        ));

        report.push_str("\nSample names, nearest the origin first:\n\n");
        let mut by_radius = census.clone();
        by_radius.sort_by(|a, b| {
            a.centroid
                .length()
                .get()
                .total_cmp(&b.centroid.length().get())
                .then(a.id.cmp(&b.id))
        });
        for civilization in by_radius.iter().take(REPORT_SAMPLE_NAMES) {
            report.push_str(&format!(
                "- {} `{}` {:?}, {:.0} km, advancement {:.2}, preference {:.2?}\n",
                civilization.id.name(),
                civilization.id,
                civilization.status,
                civilization.centroid.length().get() / 1_000.0,
                civilization.advancement,
                civilization.role_preference,
            ));
        }

        let mut names: BTreeMap<String, Vec<CivilizationId>> = BTreeMap::new();
        for civilization in &census {
            names
                .entry(civilization.id.name())
                .or_default()
                .push(civilization.id);
        }
        let collisions: Vec<(&String, &Vec<CivilizationId>)> =
            names.iter().filter(|(_, ids)| ids.len() > 1).collect();
        report.push_str(&format!(
            "\nName collisions: {} names shared among {} civilizations.\n\n",
            collisions.len(),
            collisions.iter().map(|(_, ids)| ids.len()).sum::<usize>(),
        ));
        for (name, ids) in collisions {
            let ids = ids
                .iter()
                .map(|id| format!("`{id}`"))
                .collect::<Vec<_>>()
                .join(", ");
            report.push_str(&format!("- {name}: {ids}\n"));
        }
    }
    report
}

/// Every civilization whose centroid is within [`REPORT_CENSUS_RADIUS`] of the
/// origin, in node order.
fn census(field: &CivilizationField) -> Vec<Civilization> {
    let nodes = ((REPORT_CENSUS_RADIUS + JITTER) / LATTICE).ceil() as i32;
    let mut census = Vec::new();
    for x in -nodes..=nodes {
        for y in -nodes..=nodes {
            for z in -nodes..=nodes {
                let civilization = field.civilization([x, y, z]);
                if civilization.centroid.length().get() <= REPORT_CENSUS_RADIUS {
                    census.push(civilization);
                }
            }
        }
    }
    census
}

/// Say which view is on the screen and what it holds.
fn update_readout(
    view: Res<CivilizationView>,
    painting: Option<Res<CivilizationPainting>>,
    job: Option<Res<CivilizationJob>>,
    mut readout: Query<&mut Text, With<CivilizationReadout>>,
) {
    let (Some(painting), Ok(mut text)) = (painting, readout.single_mut()) else {
        return;
    };
    let status = match (&job, painting.painted) {
        (Some(_), _) => "painting...".to_string(),
        (None, Some(painted)) if painted == *view => painting.summary.clone(),
        _ => "waiting".to_string(),
    };
    let (horizontal, vertical) = match view.plane {
        SlicePlane::Xy => ("+X", "+Y"),
        SlicePlane::Xz => ("+X", "+Z"),
        SlicePlane::Yz => ("+Y", "+Z"),
    };
    **text = format!(
        "WORLD SEED {}  {} plane through the origin\n\
         right {horizontal}, up {vertical}; {:.0} km across at {:.0} km per sample\n\
         rings at 500 and 1000 km; red = no civilization in reach\n\
         primary: one colour per identity, brighter = larger share,\n\
         black = primary changes, white dot = centroid within 120 km\n\
         roles: civilian blue, industrial yellow,\n\
         scavenger magenta hatched /, armored aqua hatched \\\n\
         provisional A1 field; roles BEFORE content eligibility\n\n\
         {status}\n\n\
         [1/2/3] seed  [X/Y/Z] plane  [R] reset",
        SEEDS[view.seed],
        view.plane.label(),
        PANEL_EXTENT.get() / 1_000.0,
        PANEL_EXTENT.get() / PANEL_PIXELS as f32 / 1_000.0,
    );
}

/// The run gate: every seed on every plane, shot on the capture path, then the
/// report written beside the shots.
#[cfg(feature = "debug")]
fn civilizations_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the opening view")
        .enter(GameStates::Loading)
        .until(and(scenario_is_built(), view_is_painted()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("clear the dev overlays")
        .on_enter(|world: &mut World| {
            hide_dev_overlays(world);
            // The version item bakes the commit into a shot.
            hide_status_bar(world);
        })
        .until(frames(SETTLE_FRAMES))
        .add();
    for seed in 0..SEEDS.len() {
        for plane in SlicePlane::ALL {
            let view = CivilizationView { seed, plane };
            let shot = view.shot();
            script = script
                .step(format!("paint {}", view.shot()))
                .on_enter(move |world: &mut World| {
                    world.insert_resource(view);
                })
                .until(and(view_is_painted(), frames(SETTLE_FRAMES)))
                .deadline(STEP_DEADLINE_SECS)
                .add()
                .step(format!("shoot {shot}"))
                .on_enter({
                    let shot = shot.clone();
                    move |world: &mut World| shoot(world, &shot)
                })
                .until(shot_written(shot))
                .deadline(SHOT_DEADLINE_SECS)
                .add();
        }
    }
    script
        .step("write the report")
        .on_enter(|_: &mut World| write_report())
        .until(frames(1))
        .add()
}

/// Write the report under the capture dir, on the capture path only.
///
/// # Panics
///
/// When the file cannot be written: a capture run that silently lost its
/// report would look complete.
#[cfg(feature = "debug")]
fn write_report() {
    if !capturing() {
        return;
    }
    let path = match std::env::var(nova_autopilot::prelude::CAPTURE_DIR_ENV) {
        Ok(dir) if !dir.is_empty() => std::path::Path::new(&dir).join(REPORT_FILE),
        _ => std::path::PathBuf::from(REPORT_FILE),
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("world civilizations: {parent:?}: {error}"));
    }
    std::fs::write(&path, civilization_report())
        .unwrap_or_else(|error| panic!("world civilizations: {path:?}: {error}"));
    info!("nova capture: {}", path.display());
}

/// Advance once the panels on screen are of the view that was asked for.
#[cfg(feature = "debug")]
fn view_is_painted() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| {
        let (Some(view), Some(painting)) = (
            world.get_resource::<CivilizationView>(),
            world.get_resource::<CivilizationPainting>(),
        ) else {
            return false;
        };
        painting.painted == Some(*view)
    })
}
