//! world_field_clouds: the same raw field, read in three dimensions.
//!
//! `world_field_slices` answers "what does the field read on this plane". It
//! cannot answer the question that actually decides whether a world reads as
//! PLACES: are the gated regions blobs with shape, size and overlap, or is a
//! slice through them just a lucky cut through a scatter. That needs the third
//! axis, so this example takes a bounded lattice of sample points around the
//! feature home cell, reads all three layers at each one, and marks the points
//! that clear a gate.
//!
//! What is drawn is SAMPLED RAW NOISE, NOT object density. A mark says "the
//! layer's field reads above its threshold at this point". It does not say a
//! rock is there, and it does not say how many: a sphere still has to survive
//! same-layer thinning, and what it then places is decided per cell. The
//! accepted sphere centres are drawn beside the marks precisely so the two can
//! be told apart.
//!
//! # Why crosses and not a volume
//!
//! A translucent volume of this would be a lie you cannot argue with: overlap
//! and depth both read as "brighter", so a deep single-layer region and a
//! shallow three-layer one look the same, and nothing behind the near face is
//! legible at all. Sparse axis crosses keep every mark separable - the eye
//! reads the three layer colours crossing each other where the layers overlap,
//! which is the one thing this picture exists to show.
//!
//! # Reading the picture
//!
//! | what you see | what it means |
//! | - | - |
//! | amber / cyan / fuchsia cross | a sample where the asteroid / planet / anchorage field clears its gate |
//! | cross ARM LENGTH and brightness | how far above the gate, by the same normalization the generator ranks on |
//! | white ring | an ACCEPTED feature sphere's centre - a sphere that survived thinning |
//! | grey cage | the 32 km sector lattice of the 5x5x5 window around the home cell |
//!
//! # Hand-run
//!
//! ```text
//! cargo run --example world_field_clouds --features debug
//! ```
//!
//! | key | what it does |
//! | - | - |
//! | 1 / 2 / 3 | show or hide the asteroid / planet / anchorage marks |
//! | O | only the samples where TWO OR MORE layers clear their gate |
//! | R | show all three layers again |
//!
//! WASD and right-drag fly; hold shift-ramp to cross the volume in reasonable
//! time. Harnessed mode:
//! - `NOVA_AUTOPILOT=1`: sample the volume, walk the two views, exit clean.
//! - `NOVA_CAPTURE=1`: also writes `world-field-cloud-layers.png` and
//!   `world-field-cloud-overlap.png`.

#[path = "../shared/world_fixture/mod.rs"]
pub mod world_fixture;

use bevy::{
    color::palettes::tailwind,
    prelude::*,
    tasks::{block_on, poll_once, AsyncComputeTaskPool, Task},
};
use clap::Parser;
use nova_protocol::prelude::*;
use nova_world::prelude::*;
use world_fixture::{
    featured_world_config, free_play_scenario, EXAMPLE_ACTIVE_RADIUS, FEATURE_HOME,
};

#[derive(Parser)]
#[command(name = "world_field_clouds")]
#[command(version = "1.0.0")]
#[command(
    about = "Mark a bounded 3D lattice of raw feature-field samples around the feature home cell",
    long_about = None
)]
struct Cli;

/// The empty bootstrap this session runs inside.
///
/// Nothing streams here: the field is a pure function of the config, so this
/// example reads it directly and never builds a sector. That is why
/// [`NovaWorldPlugin`] is absent - a picture of the field has no business
/// spawning the world the field would build.
const SCENARIO_ID: &str = "world_field_clouds_observer";

/// How many samples along each axis of the lattice.
///
/// 11 cubed is 1,331 readings, taken once, over a 320 km cube. Coarse ON
/// PURPOSE: the three gates together clear on about a third of the world, so a
/// fine lattice of this volume is a solid fog no matter how small the marks
/// are drawn. A sample a sector is the coarsest reading that still resolves a
/// gated region, and it is the granularity the streaming loop works in.
const CLOUD_EDGE_SAMPLES: usize = 11;

/// The spacing between two samples, on every axis.
///
/// One sector edge, so every mark sits at the centre of a cell of the drawn
/// cage and a mark's position is readable in CELLS. Note the gates are not
/// read per cell by the generator - they are read at the 128 km
/// [`FEATURE_LATTICE`] nodes - so this spacing is the picture's resolution,
/// not the field's.
const CLOUD_SPACING: Meters = Meters(32_000.0);

/// The most samples this example will ever take.
///
/// An explicit ceiling, checked at build, not a comment: the lattice dials sit
/// right next to each other and one careless edit to either turns a bounded
/// read into a several-minute stall with no output.
const CLOUD_SAMPLES_MAX: usize = 4_096;

/// The most marks ONE layer draws.
///
/// The gates are not equally tight - the asteroid layer clears on a large
/// fraction of the volume - so the cap is per layer rather than shared. A
/// shared cap would let the loosest layer spend the whole budget and leave the
/// overlap this example exists to show undrawn.
///
/// What a layer over the cap drops is every n-th of its hits in lattice order,
/// NOT its weakest ones. Keeping the strongest was tried first and it lies:
/// the survivors all come from the one or two hottest blobs, so most of a
/// gated world reads as empty space. An even thinning keeps every region in
/// the picture and lets the arm lengths say which of them is strong.
const CLOUD_MARKS_PER_LAYER: usize = 512;

/// The most accepted-sphere rings the picture draws.
///
/// How many spheres a window accepts is decided by the field and by the
/// thinning, not by a dial here - the pinned fixture accepts four - so the
/// rings enter the budget as a CAP. A config that accepted far more would
/// otherwise push the picture past its budget with nothing on screen saying
/// so; over the cap the readout names both numbers.
const CLOUD_RINGS_MAX: usize = 32;

/// The most line segments the marks, the lattice and the rings can ask for in
/// one frame.
///
/// Three arms a mark, three layers, plus the window cage and the capped
/// rings. Checked at build against what the caps can actually produce, so the
/// budget is a fact rather than an intention.
const CLOUD_GIZMOS_MAX: usize = 6_144;

/// The shortest and longest arm of a mark's cross, half-length.
///
/// Size carries strength beside colour: a bare-minimum hit is a speck and a
/// strong one is a 6 km cross, which is what makes the CORE of a gated region
/// findable from outside it.
const MARK_ARM: (Meters, Meters) = (Meters(1_500.0), Meters(6_000.0));

/// How far out the opening view stands from the home cell.
///
/// The lattice is a 320 km cube, so its far corner is 277 km off the centre
/// and the NEAR face is what overflows a frame first. 800 km clears both with
/// room around them, which is what lets the cloud read as one object.
const CLOUD_STANDOFF: Meters = Meters(800_000.0);

/// Which way the opening view looks from.
///
/// Off-axis on all three axes, so the lattice never lines up into rows and
/// columns: an axis-aligned view of an axis-aligned sample grid hides the
/// depth that is the entire reason for this example.
const CLOUD_EYE: Vec3 = Vec3::new(0.58, 0.42, 0.70);

/// How many segments ring an accepted sphere's centre marker.
const RING_SEGMENTS: u32 = 24;

/// How big an accepted sphere's centre ring is drawn.
///
/// A fixed small ring and NOT the sphere's real 32-96 km radius: the rings are
/// here to say "a sphere was accepted HERE", and drawing them at true size
/// would bury the sample marks under four overlapping horizon-scale circles.
/// `world_features` draws the true radii.
const RING_RADIUS: Meters = Meters(6_000.0);

/// How bright the key light is. Only the gizmos and the skybox are visible
/// here, so this exists to keep the session lit like its neighbours rather
/// than to light anything.
const KEY_ILLUMINANCE: f32 = 6_000.0;

/// Where the key light points from.
const KEY_DIRECTION: Vec3 = Vec3::new(-0.4, -1.0, -0.6);

/// In-step seconds a harnessed beat gets before the run aborts naming it.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 120.0;

/// One sample that cleared a gate.
struct CloudMark {
    /// Which layer's field cleared.
    layer: FeatureLayer,
    /// Where the sample was taken.
    position: Meters3,
    /// How far above the gate it read, in `[0, 1]`.
    strength: f32,
    /// Whether another layer cleared its gate at the same point.
    shared: bool,
}

/// The sampled volume: everything drawn, and everything the readout counts.
#[derive(Resource)]
struct CloudField {
    /// The kept marks of all three layers.
    marks: Vec<CloudMark>,
    /// How many samples cleared each layer's gate, before the cap.
    hits: [usize; FeatureLayer::COUNT],
    /// How many of those each layer draws, after the cap.
    drawn: [usize; FeatureLayer::COUNT],
    /// How many sample points cleared two or more gates at once.
    shared: usize,
    /// The accepted spheres of the window around the home cell, up to
    /// [`CLOUD_RINGS_MAX`] of them.
    spheres: Vec<FeatureSphere>,
    /// How many the window accepted, before that cap.
    accepted: usize,
    /// How many line segments one frame of the full picture asks for.
    segments: usize,
}

/// The sampling running on a worker.
#[derive(Resource)]
struct CloudJob(Task<CloudField>);

/// Which marks are on the screen.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
struct CloudView {
    /// Whether each layer's marks are drawn.
    layers: [bool; FeatureLayer::COUNT],
    /// Whether the view is restricted to the samples two or more layers share.
    shared_only: bool,
}

impl Default for CloudView {
    fn default() -> Self {
        Self {
            layers: [true; FeatureLayer::COUNT],
            shared_only: false,
        }
    }
}

/// Marks the readout line.
#[derive(Component)]
struct CloudReadout;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(clouds_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(clouds_script());
    }

    app.run()
}

fn clouds_plugin(app: &mut App) {
    app.init_resource::<CloudView>();
    app.add_systems(OnEnter(GameAssetsStates::Loaded), boot_clouds);
    app.add_systems(
        Update,
        (
            collect_samples,
            park_at_home,
            read_keys,
            draw_marks,
            draw_window,
            update_readout,
        ),
    );
}

/// Load the empty bootstrap, start the one sampling pass, and put the readout
/// up.
fn boot_clouds(mut commands: Commands, game_assets: Res<GameAssets>) {
    const _: () = assert!(
        CLOUD_EDGE_SAMPLES.pow(3) <= CLOUD_SAMPLES_MAX,
        "the sample lattice must stay inside the sample cap"
    );
    const _: () = assert!(
        CLOUD_MARKS_PER_LAYER * FeatureLayer::COUNT * 3
            + window_segments()
            + CLOUD_RINGS_MAX * RING_SEGMENTS as usize
            <= CLOUD_GIZMOS_MAX,
        "the mark cap, the window cage and the ring cap must stay inside the gizmo budget"
    );

    commands.trigger(LoadScenario(free_play_scenario(
        &game_assets,
        SCENARIO_ID,
        "World Field Clouds",
    )));

    let config = featured_world_config();
    commands.insert_resource(CloudJob(
        AsyncComputeTaskPool::get().spawn(async move { sample_volume(&config) }),
    ));

    commands.spawn((
        Name::new("Cloud Key Light"),
        DirectionalLight {
            illuminance: KEY_ILLUMINANCE,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_translation(Vec3::ZERO).looking_to(KEY_DIRECTION, Vec3::Y),
    ));

    // BOTTOM left, unlike its neighbours: the cloud is centred in the frame and
    // reaches the top of it, and a readout laid over the marks is unreadable
    // against them.
    commands
        .spawn((
            Name::new("Cloud Readout"),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                left: Val::Px(12.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                CloudReadout,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// How many segments the sector cage costs: one line per axis through every
/// pair of the `2 * radius + 2` cell boundaries.
const fn window_segments() -> usize {
    let lines = 2 * EXAMPLE_ACTIVE_RADIUS as usize + 2;
    3 * lines * lines
}

/// Read all three layers at every lattice point, keep the strongest hits, and
/// collect the spheres the window actually accepted.
///
/// PURE, and run once on a worker: 15,625 points times three layers is
/// 46,875 three-octave readings, which is a visible stall on the frame budget
/// and unnoticeable off it.
///
/// # Panics
///
/// On a [`SectorFault`]. A refused config or a non-finite reading here is a
/// bug, and a cloud drawn around it would be a picture of one.
fn sample_volume(config: &WorldConfig) -> CloudField {
    let fields = FeatureFields::new(config.seed);
    let home = FEATURE_HOME.centre(config.sector_edge).get();
    let span = CLOUD_SPACING.get() * (CLOUD_EDGE_SAMPLES - 1) as f32;
    let corner = home - Vec3::splat(span * 0.5);

    let mut per_layer: [Vec<CloudMark>; FeatureLayer::COUNT] =
        [const { Vec::new() }; FeatureLayer::COUNT];
    let mut hits = [0_usize; FeatureLayer::COUNT];
    let mut shared = 0_usize;

    for x in 0..CLOUD_EDGE_SAMPLES {
        for y in 0..CLOUD_EDGE_SAMPLES {
            for z in 0..CLOUD_EDGE_SAMPLES {
                let position = Meters3::new(
                    corner.x + x as f32 * CLOUD_SPACING.get(),
                    corner.y + y as f32 * CLOUD_SPACING.get(),
                    corner.z + z as f32 * CLOUD_SPACING.get(),
                );
                let strengths = FeatureLayer::ALL.map(|layer| {
                    let value = fields
                        .sample(layer, position)
                        .unwrap_or_else(|fault| panic!("world field clouds: {fault}"));
                    FeatureFields::strength_of(layer, value)
                });
                let cleared = strengths.iter().filter(|strength| **strength > 0.0).count();
                if cleared >= 2 {
                    shared += 1;
                }
                for (layer, strength) in FeatureLayer::ALL.into_iter().zip(strengths) {
                    if strength <= 0.0 {
                        continue;
                    }
                    hits[layer.index()] += 1;
                    per_layer[layer.index()].push(CloudMark {
                        layer,
                        position,
                        strength,
                        shared: cleared >= 2,
                    });
                }
            }
        }
    }

    let mut marks = Vec::new();
    let mut drawn = [0_usize; FeatureLayer::COUNT];
    for (index, layer_marks) in per_layer.into_iter().enumerate() {
        // Every n-th hit, in lattice order: the thinning is spread over the
        // whole volume instead of over one blob, so a region the cap thins is
        // still a region on the screen.
        let stride = layer_marks.len().div_ceil(CLOUD_MARKS_PER_LAYER).max(1);
        let kept = layer_marks.into_iter().step_by(stride);
        let before = marks.len();
        marks.extend(kept);
        drawn[index] = marks.len() - before;
    }

    let mut spheres = Vec::new();
    for coord in desired_sectors(FEATURE_HOME, EXAMPLE_ACTIVE_RADIUS) {
        let found = sector_features(config, coord)
            .unwrap_or_else(|fault| panic!("world field clouds: {fault}"));
        // One sphere reaches several cells and is READ by all of them; keeping
        // only the owner's copy is what makes the ring count the accepted
        // spheres rather than the times one was seen.
        spheres.extend(found.into_iter().filter(|sphere| sphere.owner == coord));
    }

    let accepted = spheres.len();
    spheres.truncate(CLOUD_RINGS_MAX);
    let segments = marks.len() * 3 + window_segments() + spheres.len() * RING_SEGMENTS as usize;

    CloudField {
        marks,
        hits,
        drawn,
        shared,
        accepted,
        spheres,
        segments,
    }
}

/// Take the finished sampling.
fn collect_samples(mut commands: Commands, job: Option<ResMut<CloudJob>>) {
    let Some(mut job) = job else {
        return;
    };
    let Some(field) = block_on(poll_once(&mut job.0)) else {
        return;
    };
    info!(
        "world field clouds: {} gate clears over {} samples, {} of the samples shared by two \
         or more layers",
        field.hits.iter().sum::<usize>(),
        CLOUD_EDGE_SAMPLES.pow(3),
        field.shared
    );
    commands.remove_resource::<CloudJob>();
    commands.insert_resource(field);
}

/// Stand the free-fly observer off the volume, once, and leave it flyable.
///
/// The `WASDCamera` is re-inserted WITH the pose rather than the transform
/// being written on its own: the rig keeps its own position and rewrites the
/// transform from it every frame, so a bare write is erased before it can be
/// seen.
fn park_at_home(
    mut parked: Local<bool>,
    observer: Query<(Entity, &WASDCamera), With<ScenarioCameraMarker>>,
    mut commands: Commands,
) {
    if *parked {
        return;
    }
    // The rig lands a frame after the camera does, so this waits for it.
    let Ok((entity, rig)) = observer.single() else {
        return;
    };
    commands.entity(entity).insert((cloud_view_pose(), *rig));
    *parked = true;
}

/// Where the opening view stands and what it looks at.
fn cloud_view_pose() -> Transform {
    // Engine boundary: a cell centre and a standoff are meters, a transform is
    // world units.
    let home = FEATURE_HOME
        .centre(featured_world_config().sector_edge)
        .to_engine();
    let eye = home + CLOUD_EYE.normalize() * CLOUD_STANDOFF.to_engine();
    Transform::from_translation(eye).looking_at(home, Vec3::Y)
}

/// The hand affordance: isolate a layer, or isolate what the layers share.
fn read_keys(keys: Res<ButtonInput<KeyCode>>, mut view: ResMut<CloudView>) {
    let mut next = *view;
    for (key, layer) in [
        (KeyCode::Digit1, FeatureLayer::Asteroid),
        (KeyCode::Digit2, FeatureLayer::Planet),
        (KeyCode::Digit3, FeatureLayer::Anchorage),
    ] {
        if keys.just_pressed(key) {
            next.layers[layer.index()] = !next.layers[layer.index()];
        }
    }
    if keys.just_pressed(KeyCode::KeyO) {
        next.shared_only = !next.shared_only;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        next = CloudView::default();
    }
    if next != *view {
        *view = next;
    }
}

/// The colour a layer's marks carry. The same three hues `world_features`
/// rings its spheres in, so one layer is one colour across the fleet.
fn layer_colour(layer: FeatureLayer) -> Srgba {
    match layer {
        FeatureLayer::Asteroid => tailwind::AMBER_400,
        FeatureLayer::Planet => tailwind::CYAN_400,
        FeatureLayer::Anchorage => tailwind::FUCHSIA_400,
    }
}

/// Draw one axis cross per kept sample, and a ring per accepted sphere.
fn draw_marks(mut gizmos: Gizmos, field: Option<Res<CloudField>>, view: Res<CloudView>) {
    let Some(field) = field else {
        return;
    };
    let (short, long) = (MARK_ARM.0.to_engine(), MARK_ARM.1.to_engine());
    for mark in &field.marks {
        if !view.layers[mark.layer.index()] || (view.shared_only && !mark.shared) {
            continue;
        }
        // Engine boundary: a sample is in meters, a gizmo in world units.
        let centre = mark.position.to_engine();
        let arm = short + (long - short) * mark.strength;
        let colour = layer_colour(mark.layer).with_alpha(0.25 + 0.75 * mark.strength);
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            gizmos.line(centre - axis * arm, centre + axis * arm, colour);
        }
    }

    for sphere in &field.spheres {
        // Rung flat on the XZ plane and in white rather than the layer colour:
        // an accepted sphere is a DIFFERENT kind of thing from a sample, and
        // reading one as the other is the whole confusion this example is
        // trying to prevent.
        gizmos
            .circle(
                Isometry3d::new(
                    sphere.centre.to_engine(),
                    Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                ),
                RING_RADIUS.to_engine(),
                Srgba::new(1.0, 1.0, 1.0, 0.8),
            )
            .resolution(RING_SEGMENTS);
    }
}

/// Cage the 5x5x5 sector window around the home cell, so a mark's position is
/// readable in CELLS and not only in kilometres.
fn draw_window(mut gizmos: Gizmos) {
    let edge = featured_world_config().sector_edge;
    let lines = 2 * EXAMPLE_ACTIVE_RADIUS + 2;
    let span = edge.get() * (lines - 1) as f32;
    let corner = FEATURE_HOME.centre(edge).get()
        - Vec3::splat(edge.get() * (EXAMPLE_ACTIVE_RADIUS as f32 + 0.5));
    let colour = Srgba::new(0.55, 0.58, 0.62, 0.18);

    for first in 0..lines {
        let a = first as f32 * edge.get();
        for second in 0..lines {
            let b = second as f32 * edge.get();
            // Engine boundary: the cage is laid out in meters and drawn in
            // world units.
            for (from, to) in [
                (Vec3::new(0.0, a, b), Vec3::new(span, a, b)),
                (Vec3::new(a, 0.0, b), Vec3::new(a, span, b)),
                (Vec3::new(a, b, 0.0), Vec3::new(a, b, span)),
            ] {
                let (from, to) = (corner + from, corner + to);
                gizmos.line(
                    Meters3::new(from.x, from.y, from.z).to_engine(),
                    Meters3::new(to.x, to.y, to.z).to_engine(),
                    colour,
                );
            }
        }
    }
}

/// Say what was sampled, what cleared, what is drawn, and what a mark does
/// NOT mean.
fn update_readout(
    field: Option<Res<CloudField>>,
    view: Res<CloudView>,
    mut readout: Query<&mut Text, With<CloudReadout>>,
) {
    let Ok(mut text) = readout.single_mut() else {
        return;
    };
    let Some(field) = field else {
        **text = "sampling the field...".to_string();
        return;
    };

    let span = CLOUD_SPACING.get() * (CLOUD_EDGE_SAMPLES - 1) as f32 / 1_000.0;
    let layers = FeatureLayer::ALL
        .map(|layer| {
            let index = layer.index();
            let shown = if view.layers[index] { "" } else { " (hidden)" };
            format!(
                "{layer} {} of {} over {:.2}{shown}",
                field.drawn[index],
                field.hits[index],
                layer.threshold()
            )
        })
        .join("\n");

    // Both numbers when the cap engaged, one when it did not: a ring count
    // that quietly means "the first 32" would misread as the window's whole
    // accepted set.
    let rings = if field.accepted > field.spheres.len() {
        format!(
            "{} of {} accepted spheres (white rings)",
            field.spheres.len(),
            field.accepted
        )
    } else {
        format!("{} accepted spheres (white rings)", field.accepted)
    };

    **text = format!(
        "SAMPLED RAW NOISE - where a layer clears its gate, NOT object density\n\
         {} samples, one a sector, {span:.0} km cube on {}\n{layers}\n\
         {} samples shared by two or more layers{}\n\
         {rings}, full picture {} of {CLOUD_GIZMOS_MAX} segments\n\
         [1/2/3] layer  [O] shared only  [R] all",
        CLOUD_EDGE_SAMPLES.pow(3),
        FEATURE_HOME,
        field.shared,
        if view.shared_only {
            " (only these)"
        } else {
            ""
        },
        field.segments,
    );
}

/// The picture of all three layers at once: the cloud with its overlaps in it.
#[cfg(feature = "debug")]
const LAYERS_SHOT: &str = "world-field-cloud-layers.png";

/// The same volume with everything but the shared samples taken away, so the
/// overlap regions are a shape rather than a claim.
#[cfg(feature = "debug")]
const OVERLAP_SHOT: &str = "world-field-cloud-overlap.png";

/// The run gate: sample the volume, shoot it whole, then shoot only what the
/// layers share.
#[cfg(feature = "debug")]
fn clouds_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("sample the volume")
        .enter(GameStates::Loading)
        .until(and(scenario_is_built(), volume_is_sampled()))
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
        .step("frame the whole cloud")
        .on_enter(|world: &mut World| {
            let pose = cloud_view_pose();
            pose_camera(
                world,
                Meters3::from_engine(pose.translation),
                Meters3::from_engine(pose.translation + pose.forward() * 1.0),
            );
        })
        .until(frames(SETTLE_FRAMES))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the whole cloud")
        .on_enter(|world: &mut World| shoot(world, LAYERS_SHOT))
        .until(shot_written(LAYERS_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("keep only what the layers share")
        .on_enter(|world: &mut World| {
            let shared = world.resource::<CloudField>().shared;
            assert!(
                shared > 0,
                "world field clouds: the home window must hold samples two layers share, \
                 or the overlap claim has no picture"
            );
            world.resource_mut::<CloudView>().shared_only = true;
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the overlap")
        .on_enter(|world: &mut World| shoot(world, OVERLAP_SHOT))
        .until(shot_written(OVERLAP_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}

/// Advance once the one sampling pass has landed.
#[cfg(feature = "debug")]
fn volume_is_sampled() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| world.get_resource::<CloudField>().is_some())
}
