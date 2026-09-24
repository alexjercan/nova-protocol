//! world_field_slices: look straight at the noise the world is gated on.
//!
//! `world_features` shows what the field DID - the spheres it accepted and the
//! rocks, worlds and derelicts they placed. This example shows the field
//! itself: one flat plane through the world, sampled on the CPU at one sample
//! per kilometre, painted into an image, and put on the screen.
//!
//! What is drawn is the RAW `Fbm<Perlin>` reading of one [`FeatureLayer`], NOT
//! a body count and NOT a density. A cell's rocks come from accepted spheres,
//! and a sphere is accepted only after the reading clears the layer's
//! threshold AND survives same-layer thinning against its neighbours. This
//! picture is the first of those three steps, which is exactly why it is worth
//! looking at on its own: a threshold that never gets cleared and a threshold
//! that gets cleared everywhere look identical once the spheres are drawn.
//!
//! # Reading the picture
//!
//! The ramp pivots on the layer's threshold, so the gate is a colour boundary
//! rather than a cliff:
//!
//! | what you see | what it means |
//! | - | - |
//! | near-black to steel blue | BELOW the threshold, continuously - far below is black, just below is blue. No candidate here. |
//! | dim to bright layer colour | ABOVE the threshold, ramped by the same normalization the generator's `strength` uses |
//! | white contour | exactly the threshold: the line a candidate is gated on |
//! | faint grey grid | the 32 km sector lattice, so a feature's size is readable in cells |
//!
//! Below-threshold ground keeps its shading on purpose. Clipping it to one
//! flat colour would hide how CLOSE a region is to gating, which is the thing
//! a threshold change moves and the thing a tuning pass needs to see.
//!
//! # Hand-run
//!
//! ```text
//! cargo run --example world_field_slices --features debug
//! ```
//!
//! | key | what it does |
//! | - | - |
//! | 1 / 2 / 3 | asteroid / planet / derelict layer |
//! | X / Y / Z | the XY, XZ or YZ plane |
//! | Up / Down | step the plane one 32 km sector along its fixed axis |
//! | R | back to the opening slice |
//!
//! Harnessed mode:
//! - `NOVA_AUTOPILOT=1`: walk three representative slices and exit clean.
//! - `NOVA_CAPTURE=1`: also writes `world-field-slice-asteroid-xy.png`,
//!   `world-field-slice-planet-xz.png` and
//!   `world-field-slice-derelict-yz.png`.

#[path = "../shared/world_fixture/mod.rs"]
pub mod world_fixture;

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
use world_fixture::{featured_world_config, free_play_scenario, EXAMPLE_SECTOR_EDGE};

#[derive(Parser)]
#[command(name = "world_field_slices")]
#[command(version = "1.0.0")]
#[command(
    about = "Paint a flat plane of the raw world feature field as a heatmap, one layer at a time",
    long_about = None
)]
struct Cli;

/// The empty bootstrap this session runs inside.
///
/// The field is a pure function of the config, so this example streams
/// nothing: it loads the bootstrap for a session, a camera and a sky, and
/// everything on the screen after that is one image and one readout.
const SCENARIO_ID: &str = "world_field_slices_observer";

/// How many samples across the painted square.
///
/// 512, with [`SLICE_EXTENT`] at 512 km, is exactly one sample per kilometre -
/// 32 samples across a sector and 128 across a lattice spacing. Fine enough
/// that the threshold contour is a line rather than a staircase, coarse enough
/// that one recomputation is a fraction of a second on one worker.
const SLICE_PIXELS: u32 = 512;

/// How much world the painted square covers, on both of its axes.
///
/// 512 km: sixteen sectors and four lattice spacings across, so a slice holds
/// several candidate nodes and shows a feature as a REGION with neighbours
/// rather than as one blob filling the frame.
const SLICE_EXTENT: Meters = Meters(512_000.0);

/// How far below a layer's threshold the ramp keeps shading.
///
/// The field's practical floor. Readings further down than this are painted
/// the same near-black, which costs nothing: what a tuning pass needs to see
/// is the ground just below the gate, not the bottom of the well.
const SLICE_FLOOR: f32 = -0.6;

/// How close to the threshold a sample has to read to be painted as the
/// contour, as a fraction of the value range the ramp spans.
///
/// A band rather than a zero-crossing search: the sample grid is 1 km and the
/// contour has to be visible, so the line is the set of samples within this
/// much of the gate.
const CONTOUR_BAND: f32 = 0.004;

/// The opening slice: the plane through the feature home cell's own centre.
const OPENING_STEP: i32 = 0;

/// In-step seconds a harnessed beat gets before the run aborts naming it.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 120.0;

/// Which pair of axes the painted plane spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlicePlane {
    /// Horizontal +X, vertical +Y, fixed Z.
    Xy,
    /// Horizontal +X, vertical +Z, fixed Y.
    Xz,
    /// Horizontal +Y, vertical +Z, fixed X.
    Yz,
}

impl SlicePlane {
    /// What the plane is called, and the two axes it spans.
    const fn label(self) -> &'static str {
        match self {
            Self::Xy => "XY",
            Self::Xz => "XZ",
            Self::Yz => "YZ",
        }
    }

    /// The axis the plane is fixed along, which [`SliceView::step`] moves.
    const fn fixed_axis(self) -> &'static str {
        match self {
            Self::Xy => "Z",
            Self::Xz => "Y",
            Self::Yz => "X",
        }
    }

    /// The world point `horizontal` and `vertical` meters across the plane,
    /// `fixed` meters along its fixed axis.
    fn point(self, horizontal: f32, vertical: f32, fixed: f32) -> Meters3 {
        match self {
            Self::Xy => Meters3::new(horizontal, vertical, fixed),
            Self::Xz => Meters3::new(horizontal, fixed, vertical),
            Self::Yz => Meters3::new(fixed, horizontal, vertical),
        }
    }
}

/// Which slice is on the screen.
///
/// A resource rather than three locals so the autopilot can set a slice the
/// same way a key press does, and so one change detector covers all three
/// dials.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
struct SliceView {
    /// Which layer's field is painted.
    layer: FeatureLayer,
    /// Which pair of axes the plane spans.
    plane: SlicePlane,
    /// How many whole sectors along the fixed axis the plane sits, measured
    /// from the feature home cell.
    ///
    /// Whole SECTORS and not free meters: a plane halfway through a cell is a
    /// picture of nothing in particular, and stepping by the cell is what
    /// makes two slices comparable.
    step: i32,
}

/// The painted image's handle and the slice it answers for.
///
/// The slice is kept beside the handle so the recompute is driven by a
/// comparison against what is ON SCREEN rather than by bevy change detection:
/// the view resource is inserted during boot, and a `Res::is_changed()` read
/// on the frame after that is already stale.
#[derive(Resource)]
struct SlicePainting {
    /// The image the panel draws.
    handle: Handle<Image>,
    /// The slice that image is of, or `None` before the first paint lands.
    painted: Option<SliceView>,
    /// The lowest and highest raw reading in the painted slice.
    range: (f32, f32),
    /// What share of the slice reads at or past the generator's rank ceiling.
    saturated: f32,
}

/// One slice being painted on a worker.
///
/// The paint is 262,144 three-octave samples. On the frame budget that is a
/// visible hitch every time a key is pressed; on `AsyncComputeTaskPool` it is
/// a frame or two of the previous slice staying up, which is what a human
/// reads as responsive.
#[derive(Resource)]
struct SliceJob {
    /// The slice being painted.
    view: SliceView,
    /// The running paint.
    task: Task<SlicePixels>,
}

/// A finished paint: the texels and what the readout says about them.
struct SlicePixels {
    /// `SLICE_PIXELS * SLICE_PIXELS` RGBA texels, top row first.
    texels: Vec<u8>,
    /// The lowest and highest raw reading in the slice.
    range: (f32, f32),
    /// What share of the slice reads at or past the generator's rank ceiling.
    saturated: f32,
}

/// Marks the readout line.
#[derive(Component)]
struct SliceReadout;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(slices_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(slices_script());
    }

    app.run()
}

fn slices_plugin(app: &mut App) {
    app.insert_resource(SliceView {
        layer: FeatureLayer::Asteroid,
        plane: SlicePlane::Xy,
        step: OPENING_STEP,
    });
    app.add_systems(OnEnter(GameAssetsStates::Loaded), boot_slices);
    app.add_systems(
        Update,
        (read_keys, request_paint, collect_paint, update_readout).chain(),
    );
}

/// Load the empty bootstrap, put the (still blank) panel up, and put the
/// readout beside it.
fn boot_slices(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    game_assets: Res<GameAssets>,
) {
    commands.trigger(LoadScenario(free_play_scenario(
        &game_assets,
        SCENARIO_ID,
        "World Field Slices",
    )));

    let handle = images.add(blank_slice());
    commands.insert_resource(SlicePainting {
        handle: handle.clone(),
        painted: None,
        range: (0.0, 0.0),
        saturated: 0.0,
    });

    // The panel is square and left of centre, and the readout sits under it:
    // a full-width image would run through the dev overlay's fps and version
    // bar along the top right.
    commands
        .spawn((
            Name::new("Slice Panel"),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(12.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("Slice Image"),
                ImageNode::new(handle),
                Node {
                    width: Val::Px(SLICE_PIXELS as f32),
                    height: Val::Px(SLICE_PIXELS as f32),
                    ..default()
                },
            ));
            parent.spawn((
                SliceReadout,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// A blank image of the right size, so the panel has something to point at
/// before the first paint lands.
fn blank_slice() -> Image {
    let texels = (SLICE_PIXELS * SLICE_PIXELS) as usize;
    let mut image = Image::new(
        Extent3d {
            width: SLICE_PIXELS,
            height: SLICE_PIXELS,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0; texels * 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    // Nearest, not linear: a texel IS a sample, and a filtered heatmap invents
    // readings between two that were actually taken.
    image.sampler = ImageSampler::nearest();
    image
}

/// The hand affordance: pick a layer, pick a plane, walk the fixed axis.
fn read_keys(keys: Res<ButtonInput<KeyCode>>, mut view: ResMut<SliceView>) {
    let mut next = *view;
    for (key, layer) in [
        (KeyCode::Digit1, FeatureLayer::Asteroid),
        (KeyCode::Digit2, FeatureLayer::Planet),
        (KeyCode::Digit3, FeatureLayer::Derelict),
    ] {
        if keys.just_pressed(key) {
            next.layer = layer;
        }
    }
    for (key, plane) in [
        (KeyCode::KeyX, SlicePlane::Xy),
        (KeyCode::KeyY, SlicePlane::Xz),
        (KeyCode::KeyZ, SlicePlane::Yz),
    ] {
        if keys.just_pressed(key) {
            next.plane = plane;
        }
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        next.step = next.step.saturating_add(1);
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        next.step = next.step.saturating_sub(1);
    }
    if keys.just_pressed(KeyCode::KeyR) {
        next = SliceView {
            layer: FeatureLayer::Asteroid,
            plane: SlicePlane::Xy,
            step: OPENING_STEP,
        };
    }
    if next != *view {
        *view = next;
    }
}

/// Start a paint when the slice on screen is not the slice that was asked
/// for, and no paint for it is already running.
fn request_paint(
    mut commands: Commands,
    view: Res<SliceView>,
    painting: Option<Res<SlicePainting>>,
    job: Option<Res<SliceJob>>,
) {
    let Some(painting) = painting else {
        return;
    };
    if painting.painted == Some(*view) || job.is_some_and(|job| job.view == *view) {
        return;
    }
    let wanted = *view;
    let config = featured_world_config();
    let task = AsyncComputeTaskPool::get().spawn(async move { paint_slice(&config, wanted) });
    commands.insert_resource(SliceJob { view: wanted, task });
}

/// Take a finished paint and write it into the panel's image.
fn collect_paint(
    mut commands: Commands,
    mut job: Option<ResMut<SliceJob>>,
    mut painting: Option<ResMut<SlicePainting>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Some(job), Some(painting)) = (job.as_mut(), painting.as_mut()) else {
        return;
    };
    let Some(pixels) = block_on(poll_once(&mut job.task)) else {
        return;
    };
    let view = job.view;
    commands.remove_resource::<SliceJob>();

    let Some(mut image) = images.get_mut(&painting.handle) else {
        return;
    };
    image.data = Some(pixels.texels);
    painting.painted = Some(view);
    painting.range = pixels.range;
    painting.saturated = pixels.saturated;
    debug!(
        "world field slices: painted the {} {} plane at step {}",
        view.layer,
        view.plane.label(),
        view.step
    );
}

/// Sample one plane of one layer's raw field and colour it.
///
/// PURE, and the whole cost of this example: [`SLICE_PIXELS`] squared
/// three-octave readings, taken on a worker.
///
/// # Panics
///
/// On a [`SectorFault`] from the field. A non-finite reading is a refusal, and
/// a heatmap that painted it as some colour would be a picture of a bug.
fn paint_slice(config: &WorldConfig<NovaLayeredWorld>, view: SliceView) -> SlicePixels {
    let fields = FeatureFields::new(config.seed);
    let home = world_fixture::FEATURE_HOME.centre(config.sector_edge).get();
    let fixed = match view.plane {
        SlicePlane::Xy => home.z,
        SlicePlane::Xz => home.y,
        SlicePlane::Yz => home.x,
    } + view.step as f32 * config.sector_edge.get();
    let (horizontal_centre, vertical_centre) = match view.plane {
        SlicePlane::Xy => (home.x, home.y),
        SlicePlane::Xz => (home.x, home.z),
        SlicePlane::Yz => (home.y, home.z),
    };

    let pixels = SLICE_PIXELS as usize;
    let span = SLICE_EXTENT.get();
    let per_texel = span / SLICE_PIXELS as f32;
    let mut texels = Vec::with_capacity(pixels * pixels * 4);
    let mut range = (f32::INFINITY, f32::NEG_INFINITY);
    let mut saturated = 0_usize;

    for row in 0..pixels {
        // Row 0 is the TOP of the image, which is the HIGHEST vertical-axis
        // value: an image painted the other way round is the world upside
        // down, and nothing on screen would say so.
        let vertical = vertical_centre + span * 0.5 - (row as f32 + 0.5) * per_texel;
        for column in 0..pixels {
            let horizontal = horizontal_centre - span * 0.5 + (column as f32 + 0.5) * per_texel;
            let point = view.plane.point(horizontal, vertical, fixed);
            let value = fields
                .sample(view.layer, point)
                .unwrap_or_else(|fault| panic!("world field slices: {fault}"));
            range.0 = range.0.min(value);
            range.1 = range.1.max(value);
            if FeatureFields::strength_of(view.layer, value) >= 1.0 {
                saturated += 1;
            }
            let grid = on_sector_grid(horizontal, per_texel) || on_sector_grid(vertical, per_texel);
            texels.extend_from_slice(&slice_texel(view.layer, value, grid));
        }
    }

    SlicePixels {
        texels,
        range,
        saturated: saturated as f32 / (pixels * pixels) as f32,
    }
}

/// Whether a texel straddles a 32 km sector boundary on one axis.
fn on_sector_grid(meters: f32, per_texel: f32) -> bool {
    let edge = EXAMPLE_SECTOR_EDGE.get();
    // A cell is centred on its coordinate, so its faces sit half an edge off
    // the multiples of the edge.
    let offset = meters + edge * 0.5;
    let fraction = offset.rem_euclid(edge);
    fraction < per_texel
}

/// The colour one sample reads as.
///
/// The ramp PIVOTS on the layer's threshold. Below it the shading is
/// continuous down to [`SLICE_FLOOR`], so ground that nearly gated still looks
/// different from ground that never could. Above it the ramp is the same
/// normalization the generator's `strength` uses, so a bright pixel is a
/// strong candidate and not just a big number.
fn slice_texel(layer: FeatureLayer, value: f32, grid: bool) -> [u8; 4] {
    let threshold = layer.threshold();
    let below_span = (threshold - SLICE_FLOOR).max(f32::EPSILON);
    let colour = if (value - threshold).abs() <= CONTOUR_BAND * (1.0 - SLICE_FLOOR) {
        // The gate itself, drawn as a line so the boundary is a thing you can
        // point at rather than a colour you have to judge.
        Srgba::WHITE
    } else if value <= threshold {
        let depth = ((threshold - value) / below_span).clamp(0.0, 1.0);
        lerp_srgba(
            Srgba::new(0.16, 0.26, 0.42, 1.0),
            Srgba::new(0.02, 0.02, 0.05, 1.0),
            depth,
        )
    } else {
        let strength = FeatureFields::strength_of(layer, value);
        let (dim, bright) = layer_ramp(layer);
        let ramped = lerp_srgba(dim, bright, strength);
        if strength >= 1.0 {
            // Washed out, and a flat tone on purpose: the generator's rank
            // range ENDS here, so every candidate in this region ranks the
            // same no matter how much higher the field reads. A picture that
            // kept brightening past it would suggest a difference the thinning
            // cannot see.
            lerp_srgba(ramped, Srgba::WHITE, 0.4)
        } else {
            ramped
        }
    };
    // The lattice is drawn OVER the field and tinted rather than replacing it,
    // so a grid line never hides the reading underneath it.
    let colour = if grid {
        lerp_srgba(colour, Srgba::new(0.65, 0.65, 0.70, 1.0), 0.45)
    } else {
        colour
    };
    [
        (colour.red * 255.0) as u8,
        (colour.green * 255.0) as u8,
        (colour.blue * 255.0) as u8,
        255,
    ]
}

/// The dim and bright ends of a layer's above-threshold ramp. The same three
/// hues `world_features` rings its spheres in.
fn layer_ramp(layer: FeatureLayer) -> (Srgba, Srgba) {
    match layer {
        FeatureLayer::Asteroid => (
            Srgba::new(0.35, 0.22, 0.03, 1.0),
            Srgba::new(1.0, 0.75, 0.25, 1.0),
        ),
        FeatureLayer::Planet => (
            Srgba::new(0.04, 0.26, 0.31, 1.0),
            Srgba::new(0.25, 0.92, 1.0, 1.0),
        ),
        FeatureLayer::Derelict => (
            Srgba::new(0.30, 0.05, 0.30, 1.0),
            Srgba::new(1.0, 0.35, 1.0, 1.0),
        ),
    }
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

/// Say what is on the screen, in the terms a tuning pass needs: which layer,
/// which plane, where the plane sits, what the field actually read there, and
/// what the gate it is being judged against is.
fn update_readout(
    view: Res<SliceView>,
    painting: Option<Res<SlicePainting>>,
    job: Option<Res<SliceJob>>,
    mut readout: Query<&mut Text, With<SliceReadout>>,
) {
    let (Some(painting), Ok(mut text)) = (painting, readout.single_mut()) else {
        return;
    };
    let edge = EXAMPLE_SECTOR_EDGE.get();
    let home = world_fixture::FEATURE_HOME;
    let fixed_cell = match view.plane {
        SlicePlane::Xy => home.z,
        SlicePlane::Xz => home.y,
        SlicePlane::Yz => home.x,
    } + view.step;
    let status = match (&job, painting.painted) {
        (Some(_), _) => "painting...".to_string(),
        (None, Some(painted)) if painted == *view => format!(
            "read {:+.3} .. {:+.3}\n\
             {:.0}% of it past the rank ceiling (pale: every candidate there ties)",
            painting.range.0,
            painting.range.1,
            painting.saturated * 100.0
        ),
        _ => "waiting".to_string(),
    };

    **text = format!(
        "RAW {} FIELD - the noise the gate reads, NOT rocks per cell\n\
         {} plane, {} = {:+.0} m (cell {}), step {:+}\n\
         {:.0} km across at {:.0} m per sample, {:.0} m sector grid\n\
         gate {:.3} (white contour), {status}\n\
         [1/2/3] layer  [X/Y/Z] plane  [Up/Down] step one sector  [R] reset",
        view.layer.to_string().to_uppercase(),
        view.plane.label(),
        view.plane.fixed_axis(),
        fixed_cell as f32 * edge,
        fixed_cell,
        view.step,
        SLICE_EXTENT.get() / 1_000.0,
        SLICE_EXTENT.get() / SLICE_PIXELS as f32,
        edge,
        view.layer.threshold(),
    );
}

/// The picture of the asteroid layer, the loosest gate and the one that fills
/// most of the world.
#[cfg(feature = "debug")]
const ASTEROID_SHOT: &str = "world-field-slice-asteroid-xy.png";

/// The picture of the planet layer: the tightest gate, and the one whose
/// accepted regions are islands.
#[cfg(feature = "debug")]
const PLANET_SHOT: &str = "world-field-slice-planet-xz.png";

/// The picture of the derelict layer on the third plane, two sectors off the
/// home cell - the slice that shows a stepped plane is a DIFFERENT field and
/// not the same picture shifted.
#[cfg(feature = "debug")]
const DERELICT_SHOT: &str = "world-field-slice-derelict-yz.png";

/// The run gate: three representative slices, one per layer and one per plane,
/// with the last one stepped off the home cell.
#[cfg(feature = "debug")]
fn slices_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the opening slice")
        .enter(GameStates::Loading)
        .until(and(scenario_is_built(), slice_is_painted()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Its own beat, and after the session is up: the fps and version bar
        // is built with the rest of the dev UI, so hiding it from the boot
        // step hides nothing.
        .step("clear the dev overlays")
        .on_enter(|world: &mut World| {
            hide_dev_overlays(world);
            // The fps and version bar is a HUD widget, not dev chrome, and it
            // is the one thing that must never reach a shipped picture: its
            // version item bakes the commit of the build that took the shot.
            hide_status_bar(world);
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the asteroid XY slice")
        .on_enter(|world: &mut World| shoot(world, ASTEROID_SHOT))
        .until(shot_written(ASTEROID_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("paint the planet XZ slice")
        .on_enter(show_slice(SliceView {
            layer: FeatureLayer::Planet,
            plane: SlicePlane::Xz,
            step: 0,
        }))
        .until(and(slice_is_painted(), frames(SETTLE_FRAMES)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the planet XZ slice")
        .on_enter(|world: &mut World| shoot(world, PLANET_SHOT))
        .until(shot_written(PLANET_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("paint the derelict YZ slice two sectors out")
        .on_enter(show_slice(SliceView {
            layer: FeatureLayer::Derelict,
            plane: SlicePlane::Yz,
            step: 2,
        }))
        .until(and(slice_is_painted(), frames(SETTLE_FRAMES)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the derelict YZ slice")
        .on_enter(|world: &mut World| shoot(world, DERELICT_SHOT))
        .until(shot_written(DERELICT_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}

/// Ask for one slice, the same way a key press does.
#[cfg(feature = "debug")]
fn show_slice(view: SliceView) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        world.insert_resource(view);
    }
}

/// Advance once the image on screen is of the slice that was asked for.
#[cfg(feature = "debug")]
fn slice_is_painted() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| {
        let (Some(view), Some(painting)) = (
            world.get_resource::<SliceView>(),
            world.get_resource::<SlicePainting>(),
        ) else {
            return false;
        };
        painting.painted == Some(*view)
    })
}
