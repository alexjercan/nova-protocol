//! railgun_wake_bench: the lance slug's ionized wake, three speeds side by
//! side, every knob live.
//!
//! The visual bench for `tasks/20260902-143732` (the railgun slug's ionized
//! wake). The slug is the production slug - the real marker, the real sweep,
//! and the weapon's own observer dressing it with the dart, the tracer, the
//! wake and the riding light - and what the bench adds is the range and the
//! knobs: it writes its sliders into every live [`RailgunWakeEmitter`] and
//! [`RailgunSlugLight`] each frame, so a hand-run turns the same values the
//! weapon ships as constants.
//!
//! Three lanes fire the same slug at 250, 750 and the shipped 1500 u/s, so the
//! one question a constant cannot answer is on screen at once: a wake that
//! lives half a second is 125 units long on the top lane and 750 on the bottom
//! one. The three wake policies (`1`-`3`) turn that trade into a picture. Dark
//! hull plates stand behind and beneath each lane so the moving light is
//! judged by what it lights, not by its own core.
//!
//! `T` switches off the spread of each frame's particles along the ground the
//! slug covered, to show the row of puffs a point spawner draws at 1500 u/s.
//!
//! Hand-run (the bench fires by itself every couple of seconds):
//! ```text
//! cargo run --example railgun_wake_bench --features debug
//! # Enter: volley    F: auto-fire    P: pause    [ ]: slow motion
//! # 1-3: wake policy    Up/Down + Left/Right: tune (Shift = coarse)
//! # H haze  J filaments  L light  T spread  G quality  C camera  R reset
//! ```
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load, frame, fire a volley under each
//!   policy and pose, exit clean. This is the path `probe run` takes.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot one fixed-camera frame per
//!   step, in slow motion so all three wakes are in flight at once (staged
//!   under `NOVA_CAPTURE_DIR`).

#[cfg(feature = "debug")]
use std::sync::Arc;

use avian3d::prelude::{Physics, PhysicsTime};
use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use nova_debug::harness::Predicate;
use nova_debug::prelude::capturing;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "railgun_wake_bench")]
#[command(version = "1.0.0")]
#[command(about = "The railgun slug's ionized wake at three speeds, every knob live", long_about = None)]
struct Cli;

/// One world unit is ten meters. Every figure on screen is printed through
/// this; the code stays in world units.
const METERS_PER_UNIT: f32 = 10.0;

/// The lanes, slowest first, in world units per second. The last is the
/// shipped lance's `slug_speed`.
const LANE_SPEEDS: [f32; 3] = [250.0, 750.0, 1500.0];

/// Length of every lane, in world units.
///
/// The shipped half-second wake is longer than this behind the two fast
/// lanes, so on those the whole lane is wake while the slug is in flight and
/// the tail fades after it is gone. A slug is born at `+LANE_LENGTH / 2` on
/// its lane and flies down -Z, exactly as a fired slug does.
const LANE_LENGTH: f32 = 240.0;

/// Half of [`LANE_LENGTH`], the lane's start and end.
const LANE_HALF: f32 = LANE_LENGTH / 2.0;

/// Where each lane sits relative to the middle one, per index step.
///
/// Diagonal rather than stacked: from the side pose the lanes read as three
/// lines one above the other, and from the chase pose as three lines side by
/// side, so neither fixed camera has them overlap.
const LANE_STEP: Vec3 = Vec3::new(8.0, -8.0, 0.0);

/// How far behind its lane (away from the side camera) a hull plate stands.
/// Inside the shipped light range, so a passing slug lights it.
const PLATE_STANDOFF: f32 = 5.0;

/// Where along each lane the plates stand, in world units of Z.
const PLATE_STATIONS: [f32; 6] = [-100.0, -60.0, -20.0, 20.0, 60.0, 100.0];

/// A plate's size: long along the lane, tall, thin.
const PLATE_SIZE: Vec3 = Vec3::new(0.5, 6.0, 8.0);

/// The deck strip under each lane, so the light's highlight has a surface to
/// slide along for the whole flight and not only at the plates.
const DECK_SIZE: Vec3 = Vec3::new(6.0, 0.4, LANE_LENGTH);

/// How far under its lane the deck lies.
const DECK_DROP: f32 = 4.0;

/// Key light, in lux. Deliberately dim - a tenth of the standard rig's fill -
/// so an unlit plate is dark and a lit one is the slug's doing.
const KEY_LUX: f32 = 250.0;

/// The slow-motion ladder `[` and `]` walk.
const TIME_SCALES: [f32; 6] = [1.0, 0.5, 0.25, 0.1, 0.05, 0.02];

/// Seconds the bench sits before the first automatic volley, so the scene is
/// framed before anything flies.
const FIRST_VOLLEY_DELAY: f32 = 1.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(bench_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env): run
        // timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // wake's cost is measured on `loop_vfx_range` against the same
        // revision every run, not here.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        if capturing() {
            app.add_systems(Startup, hide_hud);
        }
        app.add_plugins(bench_script());
    }

    app.run()
}

fn bench_plugin(app: &mut App) {
    app.init_resource::<Bench>();
    app.init_resource::<VolleyClock>();
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
    app.add_systems(
        Update,
        (
            read_keys,
            apply_clock,
            apply_camera,
            fire_volleys,
            tune_wakes,
            tune_lights,
            update_readout,
            place_lane_labels,
        )
            .chain()
            .run_if(in_state(GameStates::Playing)),
    );
}

/// How a wake's lifetime follows the slug's speed.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum WakePolicy {
    /// One lifetime at every speed: the wake's LENGTH grows with the slug.
    /// The shipped policy.
    #[default]
    FixedLifetime,
    /// One length at every speed: the wake's LIFETIME shrinks with the slug.
    FixedDistance,
    /// A lifetime, clamped so the wake never exceeds a maximum length.
    ClampedLifetime,
}

impl WakePolicy {
    const ALL: [WakePolicy; 3] = [
        WakePolicy::FixedLifetime,
        WakePolicy::FixedDistance,
        WakePolicy::ClampedLifetime,
    ];

    fn label(self) -> &'static str {
        match self {
            WakePolicy::FixedLifetime => "1 fixed lifetime",
            WakePolicy::FixedDistance => "2 fixed distance",
            WakePolicy::ClampedLifetime => "3 lifetime, length-clamped",
        }
    }

    /// The particle lifetime a slug at `speed` gets under this policy.
    fn lifetime(self, bench: &Bench, speed: f32) -> f32 {
        match self {
            WakePolicy::FixedLifetime => bench.lifetime,
            WakePolicy::FixedDistance => bench.fixed_distance / speed,
            WakePolicy::ClampedLifetime => bench.lifetime.min(bench.max_length / speed),
        }
    }
}

/// The fixed framings `C` cycles, and the loader's free-fly rig.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum CameraPose {
    /// Side on, all three lanes end to end. The comparison frame.
    #[default]
    Wide,
    /// Behind the lane starts, the shot receding: the pilot's own view of a
    /// slug leaving.
    Chase,
    /// Broadside on the middle lane at plate height: a slug crossing the
    /// frame a ship's length from the lens, its wake behind it.
    Close,
    /// The scenario's WASD free-fly camera, from wherever it was parked.
    Free,
}

impl CameraPose {
    const ALL: [CameraPose; 4] = [
        CameraPose::Wide,
        CameraPose::Chase,
        CameraPose::Close,
        CameraPose::Free,
    ];

    fn label(self) -> &'static str {
        match self {
            CameraPose::Wide => "wide",
            CameraPose::Chase => "chase",
            CameraPose::Close => "close pass",
            CameraPose::Free => "free-fly",
        }
    }

    /// Position and look-at, or `None` for the free-fly rig.
    fn pose(self) -> Option<(Vec3, Vec3)> {
        match self {
            // At bevy's 45 degree vertical fov and 16:9 the whole lane fits
            // from about 165 units; a little further keeps the wake thrown
            // past the lane end in frame.
            CameraPose::Wide => Some((Vec3::new(175.0, 14.0, 0.0), Vec3::new(0.0, -2.0, 0.0))),
            CameraPose::Chase => Some((
                Vec3::new(0.0, 6.0, LANE_HALF + 28.0),
                Vec3::new(0.0, -2.0, 0.0),
            )),
            CameraPose::Close => Some((Vec3::new(16.0, 2.0, -20.0), Vec3::new(0.0, 0.0, -20.0))),
            CameraPose::Free => None,
        }
    }
}

/// One of the sliders `Up`/`Down` walk and `Left`/`Right` move.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tune {
    Lifetime,
    FixedDistance,
    MaxLength,
    Density,
    HazeWidth,
    HazeIntensity,
    FilamentIntensity,
    LightLumens,
    LightRange,
    AutoPeriod,
}

impl Tune {
    const ALL: [Tune; 10] = [
        Tune::Lifetime,
        Tune::FixedDistance,
        Tune::MaxLength,
        Tune::Density,
        Tune::HazeWidth,
        Tune::HazeIntensity,
        Tune::FilamentIntensity,
        Tune::LightLumens,
        Tune::LightRange,
        Tune::AutoPeriod,
    ];

    fn label(self) -> &'static str {
        match self {
            Tune::Lifetime => "lifetime",
            Tune::FixedDistance => "fixed distance",
            Tune::MaxLength => "max length",
            Tune::Density => "density",
            Tune::HazeWidth => "haze width",
            Tune::HazeIntensity => "haze intensity",
            Tune::FilamentIntensity => "filament intensity",
            Tune::LightLumens => "light",
            Tune::LightRange => "light range",
            Tune::AutoPeriod => "auto-fire period",
        }
    }

    /// Lowest, highest and one step, in the slider's own unit.
    ///
    /// The top of the lifetime and density sliders together overrun the
    /// wake's GPU buffers behind the fastest lane; that shows as a tail that
    /// thins, and marks a setting past anything that should ship.
    fn range(self) -> (f32, f32, f32) {
        match self {
            Tune::Lifetime => (0.02, 1.00, 0.02),
            Tune::FixedDistance => (5.0, 400.0, 5.0),
            Tune::MaxLength => (10.0, 800.0, 10.0),
            Tune::Density => (0.5, 12.0, 0.5),
            Tune::HazeWidth => (0.2, 4.0, 0.1),
            Tune::HazeIntensity => (0.0, 8.0, 0.25),
            Tune::FilamentIntensity => (0.0, 8.0, 0.25),
            Tune::LightLumens => (0.0, 2_000_000.0, 50_000.0),
            Tune::LightRange => (5.0, 100.0, 5.0),
            Tune::AutoPeriod => (0.5, 5.0, 0.25),
        }
    }

    fn get(self, bench: &Bench) -> f32 {
        match self {
            Tune::Lifetime => bench.lifetime,
            Tune::FixedDistance => bench.fixed_distance,
            Tune::MaxLength => bench.max_length,
            Tune::Density => bench.density,
            Tune::HazeWidth => bench.haze_width,
            Tune::HazeIntensity => bench.haze_intensity,
            Tune::FilamentIntensity => bench.filament_intensity,
            Tune::LightLumens => bench.light_lumens,
            Tune::LightRange => bench.light_range,
            Tune::AutoPeriod => bench.auto_period,
        }
    }

    fn set(self, bench: &mut Bench, value: f32) {
        let (low, high, _) = self.range();
        let value = value.clamp(low, high);
        match self {
            Tune::Lifetime => bench.lifetime = value,
            Tune::FixedDistance => bench.fixed_distance = value,
            Tune::MaxLength => bench.max_length = value,
            Tune::Density => bench.density = value,
            Tune::HazeWidth => bench.haze_width = value,
            Tune::HazeIntensity => bench.haze_intensity = value,
            Tune::FilamentIntensity => bench.filament_intensity = value,
            Tune::LightLumens => bench.light_lumens = value,
            Tune::LightRange => bench.light_range = value,
            Tune::AutoPeriod => bench.auto_period = value,
        }
    }

    /// The value as the readout prints it: lengths in meters, never units.
    fn show(self, value: f32) -> String {
        match self {
            Tune::Lifetime | Tune::AutoPeriod => format!("{value:.2} s"),
            Tune::FixedDistance | Tune::MaxLength | Tune::HazeWidth | Tune::LightRange => {
                meters(value)
            }
            Tune::Density => format!("{:.2} per m", value / METERS_PER_UNIT),
            Tune::HazeIntensity | Tune::FilamentIntensity => format!("{value:.2}x"),
            Tune::LightLumens => format!("{:.0}k lm", value / 1000.0),
        }
    }
}

/// A length in world units, printed in meters.
fn meters(units: f32) -> String {
    let meters = units * METERS_PER_UNIT;
    if meters >= 1000.0 {
        format!("{:.2} km", meters / 1000.0)
    } else if meters >= 10.0 {
        format!("{meters:.0} m")
    } else {
        format!("{meters:.1} m")
    }
}

/// A speed in world units per second, printed in kilometers per second.
fn speed_label(units_per_second: f32) -> String {
    format!("{:.1} km/s", units_per_second * METERS_PER_UNIT / 1000.0)
}

/// Everything the keys change. The readout is a print of this, and the
/// capture script writes it directly, so a hand-run and a scripted run drive
/// the same seams.
///
/// Starts at the weapon's own [`RailgunWakeTuning`] and light, so the first
/// volley IS the shipped look and every slider reads as a delta from it.
#[derive(Resource, Clone, Debug)]
struct Bench {
    policy: WakePolicy,
    /// Particle lifetime, seconds (policies 1 and 3).
    lifetime: f32,
    /// Wake length, world units (policy 2).
    fixed_distance: f32,
    /// Longest wake, world units (policy 3).
    max_length: f32,
    /// Haze particles per world unit of flight.
    density: f32,
    /// Haze particle size at full growth, world units.
    haze_width: f32,
    /// Multiplier on the haze's HDR colour.
    haze_intensity: f32,
    /// Multiplier on the filaments' HDR colour.
    filament_intensity: f32,
    /// The moving light's peak, in lumens.
    light_lumens: f32,
    /// The moving light's reach, world units.
    light_range: f32,
    /// Seconds between automatic volleys.
    auto_period: f32,
    haze: bool,
    filaments: bool,
    light: bool,
    /// Spawn along the ground covered since the last spawn, not at a point.
    spread: bool,
    auto_fire: bool,
    paused: bool,
    /// Index into [`TIME_SCALES`].
    speed_step: usize,
    /// Index into [`Tune::ALL`]: the slider the arrows move.
    selected: usize,
    camera: CameraPose,
}

impl Default for Bench {
    fn default() -> Self {
        let shipped = RailgunWakeTuning::default();
        Self {
            policy: WakePolicy::default(),
            lifetime: shipped.lifetime,
            fixed_distance: 40.0,
            max_length: 60.0,
            density: shipped.density,
            haze_width: shipped.width,
            haze_intensity: shipped.haze_intensity,
            filament_intensity: shipped.filament_intensity,
            light_lumens: RAILGUN_SLUG_LIGHT_LUMENS,
            light_range: RAILGUN_SLUG_LIGHT_RANGE,
            auto_period: 2.0,
            haze: true,
            filaments: true,
            light: true,
            spread: shipped.spread,
            auto_fire: true,
            paused: false,
            speed_step: 0,
            selected: 0,
            camera: CameraPose::default(),
        }
    }
}

impl Bench {
    /// The tuning a slug at `speed` gets from the sliders. A layer switched
    /// off is drawn at zero intensity, which is nothing.
    fn tuning(&self, speed: f32) -> RailgunWakeTuning {
        RailgunWakeTuning {
            lifetime: self.policy.lifetime(self, speed),
            density: self.density,
            width: self.haze_width,
            haze_intensity: if self.haze { self.haze_intensity } else { 0.0 },
            filament_intensity: if self.filaments {
                self.filament_intensity
            } else {
                0.0
            },
            spread: self.spread,
        }
    }
}

/// The volley timer and the one-shot request a key or a script leaves.
///
/// Apart from [`Bench`] so the timer ticking every frame does not mark the
/// tuning as changed.
#[derive(Resource, Debug)]
struct VolleyClock {
    since_last: f32,
    requested: bool,
}

impl Default for VolleyClock {
    fn default() -> Self {
        Self {
            // Counts up to the period: the first automatic volley leaves a
            // short beat after the scene is framed.
            since_last: -FIRST_VOLLEY_DELAY,
            requested: false,
        }
    }
}

/// A slug the bench fired, and the lane it is on.
#[derive(Component, Clone, Copy, Debug)]
struct LaneSlug(usize);

/// Where a lane's middle sits.
fn lane_origin(lane: usize) -> Vec3 {
    LANE_STEP * (lane as f32 - 1.0)
}

/// Build the scene: the stage scenario (sky and a dim key), the hull plates
/// and the readout.
fn load_scene(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.trigger(LoadScenario(bench_stage(&game_assets)));

    // Dark, a little metallic: what a hull is when nothing lights it. The
    // point light is judged by the highlight it puts on these.
    let plate_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.11, 0.12),
        metallic: 0.5,
        perceptual_roughness: 0.55,
        ..default()
    });
    let plate_mesh = meshes.add(Cuboid::from_size(PLATE_SIZE));
    let deck_mesh = meshes.add(Cuboid::from_size(DECK_SIZE));

    for lane in 0..LANE_SPEEDS.len() {
        let origin = lane_origin(lane);
        for station in PLATE_STATIONS {
            commands.spawn((
                Name::new(format!("Hull Plate {lane} @ {station}")),
                Mesh3d(plate_mesh.clone()),
                MeshMaterial3d(plate_material.clone()),
                Transform::from_translation(origin + Vec3::new(-PLATE_STANDOFF, 0.0, station)),
            ));
        }
        commands.spawn((
            Name::new(format!("Hull Deck {lane}")),
            Mesh3d(deck_mesh.clone()),
            MeshMaterial3d(plate_material.clone()),
            Transform::from_translation(origin + Vec3::new(-1.0, -DECK_DROP, 0.0)),
        ));
        spawn_lane_label(&mut commands, lane);
    }

    spawn_readout(&mut commands);
}

/// The stage: the skybox and one dim key light, nothing else. The bench
/// spawns its own plates - what is compared is an effect, not a scenario
/// object.
fn bench_stage(game_assets: &GameAssets) -> ScenarioConfig {
    let key = ScenarioObjectConfig {
        base: aimed_light_base(
            "wake_key",
            "Key Light",
            Vec3::new(100.0, 80.0, 60.0),
            Vec3::ZERO,
        ),
        kind: ScenarioObjectKind::Light(LightConfig::Directional {
            illuminance: KEY_LUX,
            color: Color::srgb(0.85, 0.90, 1.0),
            shadows: false,
            aim: None,
        }),
    };
    ScenarioConfig {
        description: "The railgun slug's wake at three speeds, under every policy.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: vec![EventActionConfig::SpawnScenarioObject(key)],
        }],
        ..ScenarioConfig::new(
            "railgun_wake_bench".to_string(),
            "Railgun Wake Bench".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Fire a volley on request or on the clock: one production slug per lane.
/// The weapon's own observer dresses each with its dart, tracer, wake and
/// light; nothing here draws.
fn fire_volleys(
    mut commands: Commands,
    time: Res<Time>,
    bench: Res<Bench>,
    mut clock: ResMut<VolleyClock>,
) {
    if bench.auto_fire {
        clock.since_last += time.delta_secs();
        if clock.since_last >= bench.auto_period {
            clock.since_last = 0.0;
            clock.requested = true;
        }
    }
    if !clock.requested {
        return;
    }
    clock.requested = false;
    clock.since_last = 0.0;

    for (lane, &speed) in LANE_SPEEDS.iter().enumerate() {
        let start = lane_origin(lane) + Vec3::Z * LANE_HALF;
        // The production slug's own components, minus damage: there is
        // nothing on the range to hit.
        commands.spawn((
            Name::new(format!("Bench Slug {lane}")),
            RailgunSlugProjectileMarker,
            LaneSlug(lane),
            Transform::from_translation(start),
            RoundVelocity(Vec3::NEG_Z * speed),
            TempEntity(LANE_LENGTH / speed),
            Visibility::Visible,
        ));
    }
}

/// Write the sliders into every wake riding a bench slug, every frame, so a
/// change reaches particles already in flight.
fn tune_wakes(
    bench: Res<Bench>,
    q_slug: Query<&LaneSlug>,
    mut q_emitter: Query<&mut RailgunWakeEmitter>,
) {
    for mut emitter in &mut q_emitter {
        let Ok(slug) = q_slug.get(emitter.slug) else {
            continue;
        };
        let tuning = bench.tuning(LANE_SPEEDS[slug.0]);
        if emitter.tuning != tuning {
            emitter.tuning = tuning;
        }
    }
}

/// Write the light sliders into every light riding a bench slug. Off is
/// hidden, which a light honours.
///
/// Every frame and not on change: a light spawns after its slug, on the
/// frame after the volley, and has to pick the sliders up then.
fn tune_lights(
    bench: Res<Bench>,
    q_slug: Query<(), With<LaneSlug>>,
    mut q_light: Query<(&ChildOf, &mut PointLight, &mut Visibility), With<RailgunSlugLight>>,
) {
    for (child_of, mut light, mut visibility) in &mut q_light {
        if !q_slug.contains(child_of.parent()) {
            continue;
        }
        if light.intensity != bench.light_lumens {
            light.intensity = bench.light_lumens;
        }
        if light.range != bench.light_range {
            light.range = bench.light_range;
        }
        visibility.set_if_neq(if bench.light {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

/// The keys. Nothing here touches [`Bench`] unless a key was pressed, so the
/// readout's change detection means what it says.
fn read_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut bench: ResMut<Bench>,
    mut clock: ResMut<VolleyClock>,
    mut quality: ResMut<GraphicsQuality>,
) {
    if keys.get_just_pressed().next().is_none() {
        return;
    }
    let bench = bench.as_mut();

    if keys.just_pressed(KeyCode::Enter) {
        clock.requested = true;
    }
    if keys.just_pressed(KeyCode::KeyF) {
        bench.auto_fire = !bench.auto_fire;
    }
    if keys.just_pressed(KeyCode::KeyP) {
        bench.paused = !bench.paused;
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        bench.speed_step = (bench.speed_step + 1).min(TIME_SCALES.len() - 1);
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        bench.speed_step = bench.speed_step.saturating_sub(1);
    }
    for (index, key) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3]
        .into_iter()
        .enumerate()
    {
        if keys.just_pressed(key) {
            bench.policy = WakePolicy::ALL[index];
        }
    }
    if keys.just_pressed(KeyCode::KeyH) {
        bench.haze = !bench.haze;
    }
    if keys.just_pressed(KeyCode::KeyJ) {
        bench.filaments = !bench.filaments;
    }
    if keys.just_pressed(KeyCode::KeyL) {
        bench.light = !bench.light;
    }
    if keys.just_pressed(KeyCode::KeyT) {
        bench.spread = !bench.spread;
    }
    if keys.just_pressed(KeyCode::KeyG) {
        let index = GraphicsQuality::ALL
            .iter()
            .position(|tier| *tier == *quality)
            .unwrap_or(0);
        *quality = GraphicsQuality::ALL[(index + 1) % GraphicsQuality::ALL.len()];
    }
    if keys.just_pressed(KeyCode::KeyC) {
        let index = CameraPose::ALL
            .iter()
            .position(|pose| *pose == bench.camera)
            .unwrap_or(0);
        bench.camera = CameraPose::ALL[(index + 1) % CameraPose::ALL.len()];
    }
    if keys.just_pressed(KeyCode::KeyR) {
        *bench = Bench {
            camera: bench.camera,
            ..Bench::default()
        };
    }

    if keys.just_pressed(KeyCode::ArrowUp) {
        bench.selected = bench.selected.saturating_sub(1);
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        bench.selected = (bench.selected + 1).min(Tune::ALL.len() - 1);
    }
    let coarse = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        5.0
    } else {
        1.0
    };
    let direction = if keys.just_pressed(KeyCode::ArrowRight) {
        1.0
    } else if keys.just_pressed(KeyCode::ArrowLeft) {
        -1.0
    } else {
        0.0
    };
    if direction != 0.0 {
        let tune = Tune::ALL[bench.selected];
        let (_, _, step) = tune.range();
        tune.set(bench, tune.get(bench) + direction * step * coarse);
    }
}

/// Push pause and slow motion onto the clocks everything else reads.
///
/// Both clocks, as the pause menu does: virtual time drives the frame, the
/// fixed accumulator the slug is swept on, and hanabi's effect simulation;
/// avian's physics clock is paused beside it so nothing integrates on its own
/// schedule.
fn apply_clock(
    bench: Res<Bench>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut physics_time: ResMut<Time<Physics>>,
) {
    if !bench.is_changed() {
        return;
    }
    virtual_time.set_relative_speed(TIME_SCALES[bench.speed_step]);
    if bench.paused {
        virtual_time.pause();
        physics_time.pause();
    } else {
        virtual_time.unpause();
        physics_time.unpause();
    }
}

/// Pin the scenario camera to the chosen framing, or hand it back to the
/// free-fly rig.
///
/// On the frame the camera appears as well as on every change: the loader
/// spawns it with the WASD controller and its own parking pose, and a fixed
/// comparison camera has to win that frame too.
fn apply_camera(
    bench: Res<Bench>,
    mut commands: Commands,
    q_camera: Query<Entity, With<ScenarioCameraMarker>>,
    q_new: Query<(), Added<ScenarioCameraMarker>>,
) {
    if !bench.is_changed() && q_new.is_empty() {
        return;
    }
    for camera in &q_camera {
        match bench.camera.pose() {
            Some((position, look_at)) => {
                commands
                    .entity(camera)
                    .remove::<WASDCameraController>()
                    .insert(ScriptedCameraPose { position, look_at });
            }
            None => {
                commands
                    .entity(camera)
                    .remove::<ScriptedCameraPose>()
                    .insert(WASDCameraController);
            }
        }
    }
}

/// Marks the top-left readout.
#[derive(Component)]
struct Readout;

/// Marks a lane's projected label.
#[derive(Component)]
struct LaneLabel(usize);

/// Fixed label width in pixels; labels centre on the projected point.
const LABEL_WIDTH: f32 = 260.0;

/// How far above a lane's middle its label floats, in world units.
const LABEL_LIFT: f32 = 3.0;

fn spawn_readout(commands: &mut Commands) {
    commands.spawn((
        Readout,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(12.0),
            ..default()
        },
    ));
}

fn spawn_lane_label(commands: &mut Commands, lane: usize) {
    commands.spawn((
        LaneLabel(lane),
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(LABEL_WIDTH),
            ..default()
        },
    ));
}

/// One lane's line of the readout: its speed, and what the policy gives it.
fn lane_line(bench: &Bench, speed: f32) -> String {
    let life = bench.policy.lifetime(bench, speed);
    format!(
        "{}  wake {:.3} s = {}",
        speed_label(speed),
        life,
        meters(life * speed)
    )
}

/// Rewrite the readout from the settings.
fn update_readout(
    bench: Res<Bench>,
    quality: Res<GraphicsQuality>,
    budget: Res<GraphicsBudget>,
    q_lit: Query<(), Or<(With<TransientLight>, With<CappedLight>)>>,
    mut q_readout: Query<&mut Text, With<Readout>>,
) {
    let Ok(mut text) = q_readout.single_mut() else {
        return;
    };
    let on = |flag: bool| if flag { "on" } else { "off" };
    let mut lines = vec![
        format!(
            "RAILGUN WAKE BENCH   policy {}   quality {}   lights {}/{}   time {}x{}",
            bench.policy.label(),
            quality.label(),
            q_lit.iter().count(),
            budget.transient_lights,
            TIME_SCALES[bench.speed_step],
            if bench.paused { " PAUSED" } else { "" },
        ),
        LANE_SPEEDS
            .iter()
            .map(|&speed| lane_line(&bench, speed))
            .collect::<Vec<_>>()
            .join("   |   "),
        String::new(),
    ];
    for (index, tune) in Tune::ALL.iter().enumerate() {
        let cursor = if index == bench.selected { ">" } else { " " };
        lines.push(format!(
            "{cursor} {:<20} {}",
            tune.label(),
            tune.show(tune.get(&bench))
        ));
    }
    lines.push(String::new());
    lines.push(format!(
        "[H] haze {}   [J] filaments {}   [L] light {}   [T] spread along travel {}",
        on(bench.haze),
        on(bench.filaments),
        on(bench.light),
        on(bench.spread),
    ));
    lines.push(format!(
        "[1-3] policy   [Enter] volley   [F] auto-fire {}   [P] pause   [ [ ] ] slow motion   [C] camera {}   [G] quality   [R] reset",
        on(bench.auto_fire),
        bench.camera.label(),
    ));
    lines.push("[Up/Down] pick a slider   [Left/Right] move it   [Shift] coarse".to_string());
    if !budget.particles {
        lines.push(
            "Low preset: no particles spawn; the tracer and the dart are the whole slug."
                .to_string(),
        );
    }
    **text = lines.join("\n");
}

/// Pin each lane's label over the lane's middle, through the scenario camera.
fn place_lane_labels(
    bench: Res<Bench>,
    q_camera: Query<(&Camera, &GlobalTransform), With<ScenarioCameraMarker>>,
    mut q_labels: Query<(&LaneLabel, &mut Text, &mut Node, &mut Visibility)>,
) {
    let Ok((camera, camera_transform)) = q_camera.single() else {
        return;
    };
    for (label, mut text, mut node, mut visibility) in &mut q_labels {
        let speed = LANE_SPEEDS[label.0];
        **text = lane_line(&bench, speed);
        let anchor = lane_origin(label.0) + Vec3::Y * LABEL_LIFT;
        match camera.world_to_viewport(camera_transform, anchor) {
            Ok(position) => {
                node.left = Val::Px(position.x - LABEL_WIDTH / 2.0);
                node.top = Val::Px(position.y);
                visibility.set_if_neq(Visibility::Inherited);
            }
            Err(_) => {
                visibility.set_if_neq(Visibility::Hidden);
            }
        }
    }
}

/// The slow-motion step the captures are shot at: a tenth speed, so every
/// lane's slug is still in flight when the shutter fires.
#[cfg(feature = "debug")]
const CAPTURE_SPEED_STEP: usize = 3;

/// One frame of the capture walk.
#[cfg(feature = "debug")]
struct Shot {
    policy: WakePolicy,
    camera: CameraPose,
    quality: GraphicsQuality,
    /// The lane whose slug the shutter waits on.
    lane: usize,
    /// World units that slug has flown from its lane start when the shutter
    /// fires.
    ///
    /// Flight distance and not a frame count, because frames are wall-clock
    /// and a software renderer draws a tenth of them: a count tuned on a GPU
    /// shoots an empty lane on the probe box.
    travel: f32,
    haze: bool,
    filaments: bool,
    path: &'static str,
}

/// The shots, in order: one wide frame per policy, then the shipped policy
/// from the two closer poses, each layer of the wake alone from the close
/// pose, and the close pose again on the Medium and Low presets.
#[cfg(feature = "debug")]
const SHOTS: [Shot; 9] = [
    // 160 units at 1500 u/s is 0.107 s of flight: the slowest slug has
    // covered 27 units and every lane is in flight.
    Shot {
        policy: WakePolicy::FixedLifetime,
        camera: CameraPose::Wide,
        quality: GraphicsQuality::High,
        lane: 2,
        travel: 160.0,
        haze: true,
        filaments: true,
        path: "railgun-wake-fixed-lifetime.png",
    },
    Shot {
        policy: WakePolicy::FixedDistance,
        camera: CameraPose::Wide,
        quality: GraphicsQuality::High,
        lane: 2,
        travel: 160.0,
        haze: true,
        filaments: true,
        path: "railgun-wake-fixed-distance.png",
    },
    Shot {
        policy: WakePolicy::ClampedLifetime,
        camera: CameraPose::Wide,
        quality: GraphicsQuality::High,
        lane: 2,
        travel: 160.0,
        haze: true,
        filaments: true,
        path: "railgun-wake-clamped.png",
    },
    Shot {
        policy: WakePolicy::FixedLifetime,
        camera: CameraPose::Chase,
        quality: GraphicsQuality::High,
        lane: 2,
        travel: 120.0,
        haze: true,
        filaments: true,
        path: "railgun-wake-chase.png",
    },
    // The close pose frames the middle lane from z = -32 to -8: with its slug
    // at -28 the wake fills the frame and the slug sits at its leading edge.
    Shot {
        policy: WakePolicy::FixedLifetime,
        camera: CameraPose::Close,
        quality: GraphicsQuality::High,
        lane: 1,
        travel: 148.0,
        haze: true,
        filaments: true,
        path: "railgun-wake-close.png",
    },
    Shot {
        policy: WakePolicy::FixedLifetime,
        camera: CameraPose::Close,
        quality: GraphicsQuality::High,
        lane: 1,
        travel: 148.0,
        haze: true,
        filaments: false,
        path: "railgun-wake-close-haze.png",
    },
    Shot {
        policy: WakePolicy::FixedLifetime,
        camera: CameraPose::Close,
        quality: GraphicsQuality::High,
        lane: 1,
        travel: 148.0,
        haze: false,
        filaments: true,
        path: "railgun-wake-close-filaments.png",
    },
    Shot {
        policy: WakePolicy::FixedLifetime,
        camera: CameraPose::Close,
        quality: GraphicsQuality::Medium,
        lane: 1,
        travel: 148.0,
        haze: true,
        filaments: true,
        path: "railgun-wake-close-medium.png",
    },
    // Low: no particles and no transient lights. What is left is the dart
    // and the tracer, which is the whole of the slug on that preset.
    Shot {
        policy: WakePolicy::FixedLifetime,
        camera: CameraPose::Close,
        quality: GraphicsQuality::Low,
        lane: 1,
        travel: 148.0,
        haze: true,
        filaments: true,
        path: "railgun-wake-close-low.png",
    },
];

/// Advance once the graphics budget has followed `quality`.
///
/// The budget is what the slug's observer reads when it decides whether a
/// wake spawns, and it follows the quality a frame behind the write, so a
/// volley fired on the write's own frame would be dressed for the previous
/// preset.
#[cfg(feature = "debug")]
fn budget_follows(quality: GraphicsQuality) -> Arc<Predicate> {
    Arc::new(move |world: &World| {
        world
            .get_resource::<GraphicsBudget>()
            .is_some_and(|budget| *budget == GraphicsBudget::for_quality(quality))
    })
}

/// Advance once the slug on `lane` has flown `travel` units down it.
#[cfg(feature = "debug")]
fn slug_flew(lane: usize, travel: f32) -> Arc<Predicate> {
    let reached = LANE_HALF - travel;
    Arc::new(move |world: &World| {
        world
            .try_query::<(&LaneSlug, &Transform)>()
            .is_some_and(|mut query| {
                query
                    .iter(world)
                    .any(|(slug, transform)| slug.0 == lane && transform.translation.z <= reached)
            })
    })
}

/// Advance once no slug and no wake is left on the range.
#[cfg(feature = "debug")]
fn lanes_clear() -> Arc<Predicate> {
    Arc::new(|world: &World| {
        world
            .try_query_filtered::<(), Or<(With<LaneSlug>, With<RailgunWakeEmitter>)>>()
            .is_none_or(|mut query| query.iter(world).next().is_none())
    })
}

/// The driven walk: stand the range up, then for each shot fire one volley
/// in slow motion from its pose and shoot it once the lead slug is where the
/// shot wants it.
#[cfg(feature = "debug")]
fn bench_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the bench scene")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("take the trigger off the clock")
        .on_enter(|world: &mut World| {
            let mut bench = world.resource_mut::<Bench>();
            bench.auto_fire = false;
            bench.camera = CameraPose::Wide;
        })
        .until(frames(SETTLE_FRAMES))
        .add();

    for shot in SHOTS {
        let Shot {
            policy,
            camera,
            quality,
            lane,
            travel,
            haze,
            filaments,
            path,
        } = shot;
        script = script
            .step(format!("select {} from {}", policy.label(), camera.label()))
            .on_enter(move |world: &mut World| {
                let mut bench = world.resource_mut::<Bench>();
                bench.policy = policy;
                bench.camera = camera;
                bench.haze = haze;
                bench.filaments = filaments;
                bench.speed_step = CAPTURE_SPEED_STEP;
                world.resource_mut::<GraphicsQuality>().set_if_neq(quality);
            })
            .until(budget_follows(quality))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("volley under {}", policy.label()))
            .on_enter(|world: &mut World| world.resource_mut::<VolleyClock>().requested = true)
            .until(slug_flew(lane, travel))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("shoot {path}"))
            .on_enter(move |world: &mut World| shoot(world, path))
            .until(shot_written(path))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("let the lanes clear")
            .on_enter(|world: &mut World| world.resource_mut::<Bench>().speed_step = 0)
            .until(lanes_clear())
            .deadline(STEP_DEADLINE_SECS)
            .add();
    }
    script
}
