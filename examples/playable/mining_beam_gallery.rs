//! mining_beam_gallery: the two mining-beam art candidates on one turntable
//! bench, for the owner to judge the stow cycle before any mining section
//! exists.
//!
//! Everything here is VISUAL ONLY. Nothing carries a socket, mines or spawns
//! a ship. The candidates are the review-only glbs
//! `art/part-candidates/sections/mining_beam_{compact,forward}.glb` (recipes
//! in `scripts/section-part-recipes/`, built by `scripts/gen-section-parts.py`),
//! decoded off disk by `screenshots/shared/glb.rs`, which panics on a missing
//! file. `art/` ships in no build.
//!
//! Each candidate rests DEPLOYED, as the whole catalog does. Two tracks stow
//! it, both composed the way `SectionAnimationMotion::Translate` composes a
//! node at runtime (`rest.translation + rest.rotation * offset * progress`):
//! the `stow_lid_*` doors slide inward along their local -X, and `beam_tip`
//! slides along +Z into the collar. The doors open before the tip extends,
//! and the tip retracts before the doors close, off one reversible progress
//! value. The bench panics at load if a named node is missing or unknown, or
//! if any sampled pose leaves the cell box, collides the doors with each
//! other or with the tip, leaves the stowed tip in front of the shut doors,
//! or leaves the deployed tip behind the open doors.
//!
//! Hand-run (a bare `App`, no game plugins):
//! ```text
//! cargo run --example mining_beam_gallery
//! ```
//! - 1 / 2 / Tab: select the compact or forward candidate
//! - Space: open or stow; a press mid-travel reverses
//! - V: front or side view; arrows: orbit
//!
//! Two harnessed modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the full walk, recording
//!   nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: shoot
//!   `mining-beam-<candidate>-<front|side>-<stowed|half|open|closing>.png`
//!   per candidate, then record `mining-beam-gallery.webm` into
//!   `NOVA_CAPTURE_DIR`: each candidate opens, closes, and reverses mid-open.
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/mining-beam NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example mining_beam_gallery --features debug
//! ```

use std::path::Path;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::GameStates;

#[path = "../screenshots/shared/glb.rs"]
mod glb;

#[derive(Parser)]
#[command(name = "mining_beam_gallery")]
#[command(version = "1.0.0")]
#[command(about = "Review the mining-beam art candidates' stow cycle", long_about = None)]
struct Cli;

/// Seconds from stowed to open, and back. Example-only: no gameplay default.
const TRAVEL_SECS: f32 = 1.2;

/// The share of the travel the doors take; the tip takes the rest. The two
/// never move at once, so the tip is always behind the doors or clear of them.
const DOOR_SHARE: f32 = 0.5;

/// The node-name prefix of both doors.
const LID_PREFIX: &str = "stow_lid_";

/// The one tip node.
const TIP_NODE: &str = "beam_tip";

/// Progress values the load check grades, stowed and open included.
const POSE_CHECKS: usize = 24;

/// Bounds slack for the load check, in engine units: the generator rounds
/// vertices to 1e-4.
const BOUNDS_SLACK: f32 = 1e-4;

/// One candidate on the bench. The offsets are the stowed end of each future
/// `Translate` track, node-local, in engine units.
struct Candidate {
    id: &'static str,
    /// The glb, relative to the crate root. Its muzzle face is -Z.
    model: &'static str,
    /// Cell footprint: width, height, depth.
    cells: Vec3,
    /// Half-size of the visible front opening, copied from the recipe frame.
    aperture_half: Vec2,
    door_offset: Vec3,
    tip_offset: Vec3,
}

const CANDIDATES: [Candidate; 2] = [
    Candidate {
        id: "mining_beam_compact",
        model: "art/part-candidates/sections/mining_beam_compact.glb",
        cells: Vec3::new(1.0, 1.0, 1.0),
        aperture_half: Vec2::splat(0.20),
        door_offset: Vec3::new(-0.22, 0.0, 0.0),
        tip_offset: Vec3::new(0.0, 0.0, 0.16),
    },
    Candidate {
        id: "mining_beam_forward",
        model: "art/part-candidates/sections/mining_beam_forward.glb",
        cells: Vec3::new(1.0, 1.0, 2.0),
        aperture_half: Vec2::splat(0.20),
        door_offset: Vec3::new(-0.22, 0.0, 0.0),
        tip_offset: Vec3::new(0.0, 0.0, 0.30),
    },
];

/// Which track moves a node.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TrackType {
    Doors,
    Tip,
}

impl TrackType {
    /// The track that owns `name`. Panics on a name neither track owns: the
    /// art and this bench disagree.
    fn of(name: &str, model: &str) -> Self {
        if name.starts_with(LID_PREFIX) {
            Self::Doors
        } else if name == TIP_NODE {
            Self::Tip
        } else {
            panic!("{model}: node {name:?} is neither `{LID_PREFIX}*` nor `{TIP_NODE}`")
        }
    }

    /// This track's stow progress, 0 deployed to 1 stowed, at bench `open`
    /// (0 stowed to 1 open), eased.
    fn stow(self, open: f32) -> f32 {
        let phase = match self {
            Self::Doors => open / DOOR_SHARE,
            Self::Tip => (open - DOOR_SHARE) / (1.0 - DOOR_SHARE),
        };
        1.0 - ease(phase.clamp(0.0, 1.0))
    }
}

/// A moving node: its rest pose and the stowed end of its track.
#[derive(Component, Clone, Copy)]
struct TrackNode {
    track: TrackType,
    rest: Transform,
    offset: Vec3,
}

impl TrackNode {
    /// The node's pose at bench `open`, composed as a runtime `Translate`.
    fn pose(&self, open: f32) -> Transform {
        Transform {
            translation: self.rest.translation
                + self.rest.rotation * (self.offset * self.track.stow(open)),
            ..self.rest
        }
    }
}

/// The root entity of one candidate, by its index in [`CANDIDATES`].
#[derive(Component)]
struct CandidateRoot(usize);

/// The bench's shared stow state.
#[derive(Resource)]
struct Beam {
    /// 0 stowed to 1 open, linear in time; each track eases its own share.
    open: f32,
    target: f32,
}

/// The review views.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewType {
    /// On the muzzle axis, a little high.
    Front,
    /// Well round to the +X flank, so the hood depth and the flank read.
    Side,
}

impl ViewType {
    fn label(self) -> &'static str {
        match self {
            Self::Front => "front",
            Self::Side => "side",
        }
    }

    /// Yaw off the muzzle axis toward +X, and pitch down, in radians.
    fn angles(self) -> (f32, f32) {
        match self {
            Self::Front => (0.0, 0.14),
            Self::Side => (1.1, 0.3),
        }
    }
}

/// What the bench shows and from where.
#[derive(Resource)]
struct Bench {
    selected: usize,
    view: ViewType,
    yaw: f32,
    pitch: f32,
}

impl Bench {
    fn set_view(&mut self, view: ViewType) {
        self.view = view;
        (self.yaw, self.pitch) = view.angles();
    }
}

/// The status line under the header.
#[derive(Component)]
struct StatusText;

fn main() -> AppExit {
    let _ = Cli::parse();
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resolution: (1280, 720).into(),
            title: "Nova Protocol - mining beam gallery".into(),
            ..default()
        }),
        ..default()
    }));
    app.insert_resource(ClearColor(Color::srgb(0.015, 0.022, 0.045)));
    app.insert_resource(Beam {
        open: 0.0,
        target: 0.0,
    });
    let mut bench = Bench {
        selected: 0,
        view: ViewType::Front,
        yaw: 0.0,
        pitch: 0.0,
    };
    bench.set_view(ViewType::Side);
    app.insert_resource(bench);

    // The state machine the shared harness drives; "Playing" here means "the
    // bench is up" (the parts_viewer idiom).
    app.init_state::<GameStates>();
    app.add_systems(Startup, (setup, load_candidates, reach_playing));
    app.add_systems(
        Update,
        (
            keyboard,
            move_beam,
            show_selected,
            place_camera,
            update_status,
        )
            .chain(),
    );

    #[cfg(feature = "debug")]
    {
        // No frame-time capture: a posed bench holds no steady-state load.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_systems(Startup, nova_protocol::prelude::force_capture_resolution);
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            // No DebugPlugin in a bare app, so the smoke sentinel is logged
            // here, off the same const.
            app.add_systems(OnEnter(GameStates::Playing), || {
                info!("{}", nova_protocol::nova_debug::harness::REACHED_PLAYING)
            });
        }
        app.add_plugins(gallery_script());
    }

    app.run()
}

fn reach_playing(mut next: ResMut<NextState<GameStates>>) {
    next.set(GameStates::Playing);
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3d::default(),
        Transform::default(),
        AmbientLight {
            color: Color::WHITE,
            brightness: 80.0,
            affects_lightmapped_meshes: true,
        },
    ));
    // Key from high on the muzzle side and to -X, fill from the +X flank: the
    // -Z face, the +X flank and the top each take a different grey.
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            ..default()
        },
        Transform::from_xyz(-3.0, 5.0, -6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 2_500.0,
            ..default()
        },
        Transform::from_xyz(6.0, -1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let font = asset_server.load("fonts/SGr-IosevkaTerm-Medium.ttf");
    let line = |size: f32, colour: Color| {
        (
            TextFont {
                font: font.clone().into(),
                font_size: FontSize::Px(size),
                ..default()
            },
            TextColor(colour),
        )
    };
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: px(16.0),
            top: px(10.0),
            flex_direction: FlexDirection::Column,
            row_gap: px(2.0),
            ..default()
        })
        .with_children(|header| {
            header.spawn((
                Text::new("MINING BEAM GALLERY // review-only candidates"),
                line(18.0, Color::srgb(0.55, 0.95, 0.65)),
            ));
            header.spawn((
                Text::new(""),
                line(14.0, Color::srgb(0.85, 0.9, 0.95)),
                StatusText,
            ));
            header.spawn((
                Text::new("1/2/Tab: candidate   Space: open/stow   V: front/side   arrows: orbit"),
                line(12.0, Color::srgb(0.45, 0.55, 0.6)),
            ));
        });
}

/// Decode every candidate, grade its stow cycle, and spawn it node by node at
/// the origin. Panics on any failed check: a candidate that only works in its
/// rest pose is not one.
fn load_candidates(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    for (index, candidate) in CANDIDATES.iter().enumerate() {
        let parts: Vec<_> = glb::read_glb_nodes(&Path::new(&root).join(candidate.model))
            .into_iter()
            .map(|node| {
                let track = node.name.as_deref().map(|name| {
                    let track = TrackType::of(name, candidate.model);
                    TrackNode {
                        track,
                        rest: node.rest,
                        offset: match track {
                            TrackType::Doors => candidate.door_offset,
                            TrackType::Tip => candidate.tip_offset,
                        },
                    }
                });
                (node, track)
            })
            .collect();
        check_candidate(candidate, &parts);

        commands
            .spawn((
                Name::new(candidate.id),
                CandidateRoot(index),
                Transform::default(),
                Visibility::Hidden,
            ))
            .with_children(|parent| {
                for (node, track) in &parts {
                    let mut entity = parent.spawn((node.rest, Visibility::default()));
                    if let Some(track) = track {
                        entity.insert((Name::new(node.name.clone().unwrap_or_default()), *track));
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
            });
    }
}

/// Model-space `(low, high)` bounds of one node at bench `open`.
fn node_bounds(node: &glb::GlbNode, track: Option<&TrackNode>, open: f32) -> (Vec3, Vec3) {
    let pose = track.map_or(node.rest, |track| track.pose(open));
    node.primitives
        .iter()
        .flat_map(|primitive| &primitive.positions)
        .map(|position| pose.transform_point(Vec3::from_array(*position)))
        .fold((Vec3::MAX, Vec3::MIN), |(low, high), position| {
            (low.min(position), high.max(position))
        })
}

fn overlaps((a_low, a_high): (Vec3, Vec3), (b_low, b_high): (Vec3, Vec3)) -> bool {
    (a_low + BOUNDS_SLACK).cmplt(b_high).all() && (b_low + BOUNDS_SLACK).cmplt(a_high).all()
}

/// Grade `candidate`'s stow cycle over sampled bench poses. Panics on the
/// first violation.
fn check_candidate(candidate: &Candidate, parts: &[(glb::GlbNode, Option<TrackNode>)]) {
    let model = candidate.model;
    let names: Vec<_> = parts
        .iter()
        .filter_map(|(node, _)| node.name.as_deref())
        .collect();
    for name in ["stow_lid_right", "stow_lid_left", TIP_NODE] {
        assert!(
            names.contains(&name),
            "{model}: no {name:?} node (has {names:?})"
        );
    }
    assert_eq!(
        parts.len(),
        names.len() + 1,
        "{model}: expected one unnamed static node beside {names:?}"
    );

    let half = candidate.cells * 0.5 + Vec3::splat(BOUNDS_SLACK);
    for step in 0..=POSE_CHECKS {
        let open = step as f32 / POSE_CHECKS as f32;
        let bounds: Vec<_> = parts
            .iter()
            .map(|(node, track)| (node, track, node_bounds(node, track.as_ref(), open)))
            .collect();
        for (node, _, (low, high)) in &bounds {
            assert!(
                low.cmpge(-half).all() && high.cmple(half).all(),
                "{model}: at open {open:.2} node {:?} spans {low}..{high}, outside its {} cell box",
                node.name,
                candidate.cells
            );
        }
        let moving: Vec<_> = bounds
            .iter()
            .filter_map(|(node, track, bounds)| Some((node.name.as_deref()?, (**track)?, *bounds)))
            .collect();
        for (i, (a, _, a_bounds)) in moving.iter().enumerate() {
            for (b, _, b_bounds) in &moving[i + 1..] {
                assert!(
                    !overlaps(*a_bounds, *b_bounds),
                    "{model}: at open {open:.2} {a:?} {a_bounds:?} collides with {b:?} {b_bounds:?}"
                );
            }
        }
        let doors = moving
            .iter()
            .filter(|(_, track, _)| track.track == TrackType::Doors);
        let (door_front, door_back) = doors.fold(
            (f32::MAX, f32::MIN),
            |(front, back), (_, _, (low, high))| (front.min(low.z), back.max(high.z)),
        );
        let (_, _, (tip_low, _)) = moving
            .iter()
            .find(|(name, _, _)| *name == TIP_NODE)
            .expect("tip checked above");
        if step == 0 {
            assert!(
                tip_low.z >= door_back - BOUNDS_SLACK,
                "{model}: stowed tip reaches z {:.4}, in front of the shut doors' back face {door_back:.4}",
                tip_low.z
            );
        }
        if step == POSE_CHECKS {
            for (name, _, (low, high)) in moving
                .iter()
                .filter(|(_, track, _)| track.track == TrackType::Doors)
            {
                let clear = match *name {
                    "stow_lid_right" => low.x >= candidate.aperture_half.x,
                    "stow_lid_left" => high.x <= -candidate.aperture_half.x,
                    _ => unreachable!("all door tracks have known names"),
                };
                assert!(
                    clear,
                    "{model}: open {name} spans x {:.4}..{:.4} inside the front aperture half-width {:.4}",
                    low.x,
                    high.x,
                    candidate.aperture_half.x
                );
            }
            assert!(
                tip_low.z < door_front,
                "{model}: deployed tip reaches only z {:.4}, behind the open doors' face {door_front:.4}",
                tip_low.z
            );
        }
    }
    info!(
        "mining_beam_gallery: {} passed {} pose checks",
        candidate.id,
        POSE_CHECKS + 1
    );
}

/// Smoothstep on `0..=1`.
fn ease(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut bench: ResMut<Bench>,
    mut beam: ResMut<Beam>,
) {
    if keys.just_pressed(KeyCode::Digit1) {
        bench.selected = 0;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        bench.selected = 1;
    }
    if keys.just_pressed(KeyCode::Tab) {
        bench.selected = (bench.selected + 1) % CANDIDATES.len();
    }
    if keys.just_pressed(KeyCode::Space) {
        beam.target = if beam.target > 0.5 { 0.0 } else { 1.0 };
    }
    if keys.just_pressed(KeyCode::KeyV) {
        let next = match bench.view {
            ViewType::Front => ViewType::Side,
            ViewType::Side => ViewType::Front,
        };
        bench.set_view(next);
    }
    let turn = time.delta_secs() * 1.2;
    let axis = |negative: KeyCode, positive: KeyCode| {
        f32::from(keys.pressed(positive)) - f32::from(keys.pressed(negative))
    };
    let yaw = bench.yaw + axis(KeyCode::ArrowLeft, KeyCode::ArrowRight) * turn;
    let pitch = bench.pitch + axis(KeyCode::ArrowDown, KeyCode::ArrowUp) * turn;
    if yaw != bench.yaw || pitch != bench.pitch {
        bench.yaw = yaw;
        bench.pitch = pitch.clamp(-1.2, 1.2);
    }
}

/// Move the bench toward its target at the travel rate and pose every track
/// node to match.
fn move_beam(
    time: Res<Time>,
    mut beam: ResMut<Beam>,
    mut nodes: Query<(&TrackNode, &mut Transform)>,
) {
    let step = time.delta_secs() / TRAVEL_SECS;
    let open = beam.open;
    let open = if open < beam.target {
        (open + step).min(beam.target)
    } else {
        (open - step).max(beam.target)
    };
    if open != beam.open {
        beam.open = open;
    }
    for (node, mut transform) in &mut nodes {
        *transform = node.pose(beam.open);
    }
}

fn show_selected(bench: Res<Bench>, mut roots: Query<(&CandidateRoot, &mut Visibility)>) {
    for (root, mut visibility) in &mut roots {
        let wanted = if root.0 == bench.selected {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        visibility.set_if_neq(wanted);
    }
}

/// Orbit the camera round the selected candidate's centre, far enough back to
/// frame its whole length.
fn place_camera(bench: Res<Bench>, mut camera: Query<&mut Transform, With<Camera3d>>) {
    let Ok(mut transform) = camera.single_mut() else {
        return;
    };
    let cells = CANDIDATES[bench.selected].cells;
    let distance = 1.5 + cells.z * 0.75;
    let direction = Vec3::new(
        bench.yaw.sin() * bench.pitch.cos(),
        bench.pitch.sin(),
        -bench.yaw.cos() * bench.pitch.cos(),
    );
    *transform = Transform::from_translation(direction * distance).looking_at(Vec3::ZERO, Vec3::Y);
}

fn update_status(bench: Res<Bench>, beam: Res<Beam>, mut text: Query<&mut Text, With<StatusText>>) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let percent = beam.open * 100.0;
    let state = if beam.open != beam.target {
        let heading = if beam.target > beam.open {
            "opening"
        } else {
            "closing"
        };
        format!("{heading} {percent:.0}%")
    } else if beam.open == 0.0 {
        "stowed".to_string()
    } else if beam.open == 1.0 {
        "open".to_string()
    } else {
        format!("held {percent:.0}%")
    };
    let line = format!(
        "{}   {state}   view: {}   travel {TRAVEL_SECS:.1} s",
        CANDIDATES[bench.selected].id,
        bench.view.label()
    );
    if text.0 != line {
        text.0 = line;
    }
}

/// Send the bench toward `target`.
#[cfg(feature = "debug")]
fn drive(target: f32) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| world.resource_mut::<Beam>().target = target
}

/// Advance once the bench rests at `target`.
#[cfg(feature = "debug")]
fn beam_at(target: f32) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .get_resource::<Beam>()
            .is_some_and(|beam| beam.open == target)
    })
}

/// Select `candidate`, frame `view` and snap the bench stowed.
#[cfg(feature = "debug")]
fn stage(candidate: usize, view: ViewType) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let mut bench = world.resource_mut::<Bench>();
        bench.selected = candidate;
        bench.set_view(view);
        *world.resource_mut::<Beam>() = Beam {
            open: 0.0,
            target: 0.0,
        };
    }
}

/// The driven walk: per candidate and view, a still at stowed, half-extended
/// (doors open, tip halfway), open, and closing (tip in, doors halfway shut);
/// then one webm of every candidate opening, closing and reversing mid-open.
#[cfg(feature = "debug")]
fn gallery_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    use nova_protocol::prelude::{
        elapsed, frames, loop_end, loop_start, loop_written, shoot, shot_written, state_is,
        SETTLE_FRAMES, SHOT_DEADLINE_SECS,
    };

    const LOOP_NAME: &str = "mining-beam-gallery";
    let travel = TRAVEL_SECS * 3.0;
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("gallery: reach Playing")
        .until(state_is(GameStates::Playing))
        .deadline(30.0)
        .add()
        .step("gallery: settle")
        .until(frames(SETTLE_FRAMES))
        .add();

    for (index, candidate) in CANDIDATES.iter().enumerate() {
        let short = candidate.id.trim_start_matches("mining_beam_");
        for view in [ViewType::Front, ViewType::Side] {
            script = script
                .step(format!("gallery: stage {short} {}", view.label()))
                .on_enter(stage(index, view))
                .until(frames(SETTLE_FRAMES))
                .add();
            for (pose, target) in [
                ("stowed", 0.0),
                ("half", 0.75),
                ("open", 1.0),
                ("closing", 0.25),
            ] {
                let shot = format!("mining-beam-{short}-{}-{pose}.png", view.label());
                let ack = shot.clone();
                script = script
                    .step(format!("gallery: drive {short} {} {pose}", view.label()))
                    .on_enter(drive(target))
                    .until(beam_at(target))
                    .deadline(travel)
                    .add()
                    .step(format!("gallery: settle {short} {} {pose}", view.label()))
                    .until(frames(3))
                    .add()
                    .step(format!("gallery: shoot {shot}"))
                    .on_enter(move |world: &mut World| shoot(world, &shot))
                    .until(shot_written(ack))
                    .deadline(SHOT_DEADLINE_SECS)
                    .add();
            }
        }
    }

    script = script
        .step("gallery: stage the loop")
        .on_enter(stage(0, ViewType::Side))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("gallery: open the loop")
        .on_enter(|world| loop_start(world, LOOP_NAME))
        .add();
    for (index, candidate) in CANDIDATES.iter().enumerate() {
        let short = candidate.id.trim_start_matches("mining_beam_");
        script = script
            .step(format!("gallery: loop {short} stowed"))
            .on_enter(stage(index, ViewType::Side))
            .until(elapsed(0.5))
            .add()
            .step(format!("gallery: loop {short} open"))
            .on_enter(drive(1.0))
            .until(beam_at(1.0))
            .deadline(travel)
            .add()
            .step(format!("gallery: loop {short} hold open"))
            .until(elapsed(0.6))
            .add()
            .step(format!("gallery: loop {short} close"))
            .on_enter(drive(0.0))
            .until(beam_at(0.0))
            .deadline(travel)
            .add()
            .step(format!("gallery: loop {short} hold shut"))
            .until(elapsed(0.3))
            .add()
            // Reverse mid-extension: the tip turns back, then the doors shut.
            .step(format!("gallery: loop {short} reopen"))
            .on_enter(drive(1.0))
            .until(beam_at_least(0.7))
            .deadline(travel)
            .add()
            .step(format!("gallery: loop {short} reverse"))
            .on_enter(drive(0.0))
            .until(beam_at(0.0))
            .deadline(travel)
            .add()
            .step(format!("gallery: loop {short} hold"))
            .until(elapsed(0.5))
            .add();
    }
    script
        .step("gallery: close the loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(120.0)
        .add()
}

/// Advance once the bench is at least `open`.
#[cfg(feature = "debug")]
fn beam_at_least(open: f32) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .get_resource::<Beam>()
            .is_some_and(|beam| beam.open >= open)
    })
}
