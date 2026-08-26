//! wfc_ships: a row of random spaceships grown by wave function collapse,
//! where the adjacency rules ARE the catalog's link points.
//!
//! The generator knows nothing about ships. It knows that a cell of the build
//! grid holds one section at one rotation, that a socket may never press into a
//! face that has none - two sockets meeting is what `derive_link_point_graph`
//! calls a mate - and that nothing may stand in the space a part fires,
//! launches or exhausts into. Everything that reads as design comes out of
//! those two rules. What is collapsed is STRUCTURE only: hull, a bridge,
//! drives, bays and mounts, on a half grid that is then mirrored.
//!
//! - A PDC mount carries ONE socket, on its base plate, so the collapse can
//!   only bolt it down by the foot. Turrets end up on the skin because there is
//!   nowhere else they fit.
//! - A drive carries one too, on the flat end it mounts by. Its other five
//!   faces carry nothing to mate, which is not the same as being closed: two
//!   drives may sit flank to flank, and a bank of nozzles is a ship part.
//! - A torpedo bay's bow face is a hole, not a socket, so nothing mates across
//!   the muzzle.
//!
//! Mating alone is not enough for those three, because the rule that lets two
//! drives sit flank to flank also lets one exhaust into the other's side.
//! CLEARANCE is the second rule, read off the part's KIND rather than guessed
//! from an absent socket: the whole LANE in front of an exit is void - no
//! structure, and nothing beside it demanding cladding in it, since the skin
//! covers every face of structure that would otherwise look at vacuum.
//! `compatible` holds the one cell a binary rule can see and
//! `erode_blocked_exits` holds the rest of the lane; `refuse_blocked_exits`
//! fails the run if either misses. A part may not carry a socket on the face it
//! fires through, and the example asserts it.
//!
//! The clearance rule itself is `nova_ship::sections::clearance`, which the
//! EDITOR refuses placements with too, so a hull the collapse may not draw is a
//! hull a player may not build. The generator lives in `shared/wfc.rs`, where
//! `wfc_arena` reaches it too; this file owns the row, the photography and the
//! keys.
//!
//! The SKIN is not built here, and that is the point of the example as much as
//! a convenience. Nothing in this file places a plate or names one: a ship asks
//! for cladding with a single flag and the GAME derives the whole of it from
//! the structure at spawn (`nova_ship`'s `shell_skin`). A skin that closes over
//! a hull this generator never mentioned proves the derivation reads structure
//! and nothing else - and `--bare` is the same ships with the flag off.
//!
//! What is NOT a link-point rule, and is marked as such where it appears: the
//! draw weights, the vacuum taper that gives the silhouette a nose, the keel
//! the collapse starts from, the drive deck seeded at the far end of it, the
//! one part that may only point one way, the mirror across the centreline, and
//! the smoothing passes.
//!
//! The last two are what give a ship a BACK, and neither could be a link-point
//! rule. Mating is BINARY - one cell and its neighbour - so it cannot hold a
//! fact about the whole hull, and the face a drive is bolted to IS the
//! direction it exhausts, so mating alone bolts nozzles to the roof.
//! `Part::aim` says a drive fires AFT as a unary constraint on the opening
//! domains; `seed_stern` lays the aft-facing surface for it to stand on,
//! because a rule that says where a part may not go cannot conjure the place
//! where it may.
//!
//! Every ship is checked by the real content lint (`lint_scenario`) before it
//! is posed. A generated ship the game's own gate would reject is a broken
//! subject, so the producer panics with the findings rather than shooting it.
//!
//! Hand-run is free-fly with WASD; `R` re-rolls the row, `C` strips or restores
//! the cladding on the same seeds, `L` cycles the look:
//! ```text
//! cargo run --example wfc_ships --features debug
//! cargo run --example wfc_ships --features debug -- --seed 7 --ships 3
//! cargo run --example wfc_ships --features debug -- --ships 1 --bare
//! cargo run --example wfc_ships --features debug -- --style salvage
//! ```
//!
//! `L` - not `S`, which the free-fly camera owns - is what makes four authored
//! looks COMPARABLE: the same hulls at the same pose, redressed in place.
//! Grave/tilde cycles the game HUD, and in a hand-run the seed readout follows
//! it. Captures are exempt: the readout is what makes a frame reproducible.
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - collapse the row, frame it, strip the
//!   cladding, exit clean. This is the path `probe run` takes.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot the row clad and then bare
//!   (staged under `NOVA_CAPTURE_DIR`). Same ships in both frames, so the pair
//!   is a before and after rather than two rolls.

use bevy::prelude::*;
use clap::Parser;
// Direct, not through `nova_protocol::nova_debug`: that path only exists under
// the `debug` feature, and `capturing()` gates the idle orbit and the readout
// in EVERY build.
use nova_debug::prelude::capturing;
use nova_protocol::prelude::*;

// The generator itself, shared with `wfc_arena` (which flies two of these
// hulls against each other instead of posing a row).
#[path = "shared/wfc.rs"]
mod wfc;
use wfc::{refuse_broken_ships, style_at, tile_set, wfc_hull, StyleId};

#[derive(Parser)]
#[command(name = "wfc_ships")]
#[command(version = "1.0.0")]
#[command(about = "Wave-function-collapse spaceships built from link-point rules", long_about = None)]
struct Cli {
    /// First seed of the row; ship `n` collapses from `seed + n`.
    #[arg(long, default_value_t = DEFAULT_SEED)]
    seed: u64,
    /// How many ships to stand in the row. Zero is the empty stand: the sky,
    /// the photo rig and the HUD with no subject, which is what a frame costs
    /// before any ship is in it.
    #[arg(long, default_value_t = DEFAULT_SHIPS)]
    ships: usize,
    /// Strip the cladding and show the bare structural collapse.
    #[arg(long)]
    bare: bool,
    /// Start on this style id instead of the first the content offers. `S`
    /// cycles from wherever this leaves the row.
    #[arg(long)]
    style: Option<String>,
}

/// The row's default first seed.
const DEFAULT_SEED: u64 = 20_260_815;
/// How many ships the row holds by default. Three of these hulls fill a 16:9
/// frame at a size where the skin is legible; more and they are thumbnails.
const DEFAULT_SHIPS: usize = 3;
/// Ships per row of the stand. Past three a row reads as a thin band in a 16:9
/// frame, so the stand wraps into a grid instead.
const COLUMNS: usize = 3;
/// Centre-to-centre spacing across a row, a little wider than the widest hull
/// the grid can produce (`2 * HALF_WIDTH` cells, plus a plate on each flank).
const SHIP_SPACING: f32 = 11.0;
/// Centre-to-centre spacing between rows, sized against hull LENGTH the same
/// way.
const ROW_SPACING: f32 = 13.0;
/// Every ship stands at the same quarter yaw, so the camera reads flank and
/// top at once and the fixed photo rig lights all of them the same way.
const SHIP_YAW: f32 = -0.5;

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    let roster = Roster {
        seed: cli.seed,
        ships: cli.ships,
        clad: !cli.bare,
        style: 0,
    };
    let requested = cli.style.clone();
    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| {
            wfc_plugin(app, roster, StyleRequest(requested.clone()))
        })
        .build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        // A posed row holds no steady-state load worth GRADING, so the example
        // declares no frame-time claim - but it is the subject the ship-cost
        // and floor studies are measured on, so a capture pointed at it gets
        // one. Declaring only when armed keeps `probe run` unchanged and still
        // writes the artifact the sweep reads.
        if nova_probe::probe_armed() {
            app.add_plugins(
                nova_probe::nova_frametime()
                    .window(90, 200)
                    .ready_when(row_is_standing()),
            );
        }
        // Clean frames at the fleet's known 16:9, dev overlays out of shot.
        // The HUD drops to cinematic only under capture: a hand-run keeps the
        // level On so grave/tilde round-trips the readout with the rest of
        // the HUD. The seed readout is NOT HUD-tier and stays in every
        // capture - it is what makes a frame reproducible.
        // The shot resolution stands down for a MEASURED run: the frame-time
        // capture sizes the window before winit creates it, and a second
        // Startup writer asking for 1920x1080 is both ambiguous with it and
        // refused by the window manager afterwards, which left the two
        // disagreeing about what had been measured.
        if !nova_probe::probe_armed() {
            app.add_systems(Startup, force_capture_resolution);
        }
        app.add_systems(Startup, hide_dev_overlays);
        if capturing() {
            app.add_systems(Startup, hide_hud);
        }
        // A subject is a dynamic body, and in zero-g nothing damps a spawn
        // impulse: the row drifts and TURNS while the harness settles, so two
        // runs that take different wall-clock time photograph the same ships at
        // different attitudes. That made a pair of shots useless as an A/B of
        // anything but the clock. Pinning the bodies static makes the frame a
        // function of the seeds again.
        //
        // Ungated: nothing FLIES these hulls (`SpaceshipController::None`), so
        // the turn a hand-run shows is that same spawn impulse and not input.
        // A viewer wants the subject still for the same reason a capture does.
        app.add_systems(Update, freeze_bodies);
        app.add_plugins(wfc_script());
    }

    app.run()
}

/// Frames the drawable count must hold still before the row counts as built.
#[cfg(feature = "debug")]
const ROW_STEADY_FRAMES: u32 = 90;

/// Hold a frame-time capture until the row has finished BUILDING.
///
/// The capture's warm-up is counted in frames from `Playing`, and a
/// loading-screen frame costs under 2 ms - so the declared 90 of them are
/// 0.16 s, and the window opened on a row that was still spawning and still
/// compiling pipelines. Consecutive captures of one seed read 9.9 ms and
/// 30.1 ms depending on where in that build the window happened to land.
///
/// The predicate is "the drawable count stopped moving" rather than a ship
/// count, because the SKIN is derived from the structure after it spawns: the
/// last thing to settle is how many meshes there are, and that is the thing
/// that costs. `Playing` is required inside the predicate as well as outside
/// it, so an empty stand - which is legitimately steady at zero - cannot open
/// the gate while the loading screen is still up.
#[cfg(feature = "debug")]
fn row_is_standing() -> impl Fn(&World) -> bool + Send + Sync + 'static {
    use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

    let last = AtomicUsize::new(usize::MAX);
    let steady = AtomicU32::new(0);
    move |world: &World| {
        let playing = world
            .get_resource::<State<GameStates>>()
            .is_some_and(|state| *state.get() == GameStates::Playing);
        if !playing {
            steady.store(0, Ordering::Relaxed);
            return false;
        }
        let drawables = world
            .iter_entities()
            .filter(|entity| entity.contains::<Mesh3d>())
            .count();
        if last.swap(drawables, Ordering::Relaxed) == drawables {
            steady.fetch_add(1, Ordering::Relaxed) + 1 >= ROW_STEADY_FRAMES
        } else {
            steady.store(0, Ordering::Relaxed);
            false
        }
    }
}

fn wfc_plugin(app: &mut App, roster: Roster, requested: StyleRequest) {
    app.insert_resource(roster);
    app.insert_resource(requested);
    // Enabled only for a hand-run: a capture composes its own frame, and an
    // orbit under it would photograph a different attitude every run - the
    // exact defect `freeze_bodies` exists to stop.
    //
    // A frame-time capture is a capture in the way that matters here, and
    // `capturing()` (NOVA_CAPTURE, the screenshot arm) does not cover it. The
    // orbit made the measured attitude a function of load time: consecutive
    // captures of the same seed read 9.9 ms and 30.1 ms.
    let composed = capturing() || std::env::var_os(nova_core::PROBE_ENV).is_some();
    app.insert_resource(IdleOrbit::new(!composed));
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_row);
    app.add_systems(
        Update,
        (
            reroll_on_key.run_if(in_state(GameStates::Playing)),
            frame_new_camera,
            update_readout,
            track_orbit_idle,
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

/// Which row is on the stage: the first seed, how many ships stand in it,
/// whether they wear their skin, and WHICH look they wear.
///
/// The look is an INDEX into the merged catalog rather than an id, because this
/// example is a producer and must not know what a style is called. `L` steps it;
/// the index is taken modulo the catalog, so it survives a mod adding one.
#[derive(Resource, Clone, Copy)]
struct Roster {
    seed: u64,
    ships: usize,
    clad: bool,
    style: usize,
}

/// The style id `--style` asked for, before the content it names has loaded.
///
/// Resolved to a `Roster` index once, on the first row: the catalog does not
/// exist when the CLI is parsed, and a producer holds an index from then on.
#[derive(Resource)]
struct StyleRequest(Option<String>);

fn load_row(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    requested: Res<StyleRequest>,
    mut roster: ResMut<Roster>,
) {
    if let Some(id) = requested.0.as_deref() {
        match styles.iter().position(|style| style.id == id) {
            Some(index) => roster.style = index,
            // Loud, not silent: a typo would otherwise photograph the first
            // look and read as the one that was asked for.
            None => panic!("--style '{id}' is not in the merged content"),
        }
    }
    let style = style_at(&styles, roster.style);
    commands.trigger(LoadScenario(wfc_row(
        &game_assets,
        &sections,
        *roster,
        style,
    )));
    spawn_readout(&mut commands);
}

/// `R` re-rolls the whole row: a fresh scenario off the next seed block,
/// through the same `LoadScenario` path the first row took (which tears the
/// old one down for us). `C` strips or restores the cladding on the spot, and
/// `L` steps to the next authored look on the SAME seeds - which is the only
/// way to judge four looks against one hull rather than against four hulls.
fn reroll_on_key(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    mut roster: ResMut<Roster>,
) {
    if keyboard.just_pressed(KeyCode::KeyR) {
        roster.seed = roster.seed.wrapping_add(roster.ships as u64);
    } else if keyboard.just_pressed(KeyCode::KeyC) {
        roster.clad = !roster.clad;
    } else if keyboard.just_pressed(KeyCode::KeyL) {
        roster.style = roster.style.wrapping_add(1);
        // A style change is invisible on a bare row, so asking for one asks
        // for the skin back.
        roster.clad = true;
    } else {
        return;
    }
    let style = style_at(&styles, roster.style);
    commands.trigger(LoadScenario(wfc_row(
        &game_assets,
        &sections,
        *roster,
        style,
    )));
}

/// Where one ship stands on the grid stand: filled row by row, each row
/// centred on its own count so a short last row does not hang off one side.
fn stand_position(index: usize, ships: usize) -> Vec3 {
    let row = index / COLUMNS;
    let rows = ships.div_ceil(COLUMNS);
    let in_row = (ships - row * COLUMNS).min(COLUMNS);
    let column = index % COLUMNS;
    Vec3::new(
        (column as f32 - (in_row as f32 - 1.0) * 0.5) * SHIP_SPACING,
        0.0,
        (row as f32 - (rows as f32 - 1.0) * 0.5) * ROW_SPACING,
    )
}

/// The stage: a row of collapsed ships under the game's own sky and the
/// repo's standard three-point rig, so the subjects render the way the game
/// would draw them.
fn wfc_row(
    game_assets: &GameAssets,
    sections: &GameSections,
    roster: Roster,
    style: StyleId,
) -> ScenarioConfig {
    let tiles = tile_set(sections);
    let ships = (0..roster.ships).map(|index| {
        let seed = roster.seed.wrapping_add(index as u64);
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: format!("wfc_ship_{index}"),
                name: format!("WFC {seed}"),
                position: stand_position(index, roster.ships),
                rotation: Quat::from_rotation_y(SHIP_YAW),
            },
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                allegiance: None,
                // Scenery: these are subjects, not craft. Nothing flies them
                // and nothing shoots them - `wfc_arena` is where they fight.
                controller: SpaceshipController::None,
                hull: ShipSource::Inline(wfc_hull(&tiles, seed, roster.clad, style)),
                ..default()
            }),
        })
    });

    let scenario = ScenarioConfig {
        description: "Wave-function-collapse ships built from link-point rules".to_string(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: ships
                .chain(ThreePointRig::around("photo", Vec3::ZERO, 3.0).actions())
                .collect(),
        }],
        ..ScenarioConfig::new(
            "wfc_ships".to_string(),
            "WFC Ships".to_string(),
            game_assets.cubemap.clone().into(),
        )
    };
    refuse_broken_ships(&scenario, sections);
    scenario
}

/// What the camera aims at: the middle of the stand.
const CAMERA_TARGET: Vec3 = Vec3::ZERO;

/// Where the camera stands for a row of `ships`, backed off far enough to hold
/// whatever [`stand_position`] laid out. High and in front: the interesting
/// face of these hulls is the top skin, where the mounts and the bays surface.
///
/// Derived rather than pinned so `--ships 1` is a close-up of one hull and not
/// a speck in a frame composed for six.
fn camera_position(ships: usize) -> Vec3 {
    // One ship of margin on the count: the stand is as wide as the gaps BETWEEN
    // its ships plus the hulls on the ends, and a hull is about a gap wide.
    let columns = ships.min(COLUMNS) as f32 + 1.0;
    let rows = ships.div_ceil(COLUMNS) as f32 + 1.0;
    let span = (columns * SHIP_SPACING).max(rows * ROW_SPACING);
    Vec3::new(0.0, span * 0.34, span * 0.58)
}

/// Frame every camera the loader spawns, so a re-roll comes back to the same
/// composition instead of the loader's default perch.
fn frame_new_camera(
    roster: Res<Roster>,
    mut q_camera: Query<&mut Transform, (With<ScenarioCameraMarker>, Added<ScenarioCameraMarker>)>,
) {
    for mut transform in &mut q_camera {
        *transform = Transform::from_translation(camera_position(roster.ships))
            .looking_at(CAMERA_TARGET, Vec3::Y);
    }
}

/// Radians per second the idle orbit turns at. Slow enough to read a hull's
/// far side without waiting, and to sit under a capture's own framing.
const ORBIT_RATE: f32 = 0.25;

/// How much further out the orbit stands than the composed front-on framing.
///
/// [`camera_position`] frames the row head-on, where a line of hulls is at its
/// NARROWEST. An orbit also passes the broadside, where the same row is as wide
/// as its whole span, so it needs the extra reach or the end hulls leave frame
/// there. Set by rendering both extremes.
const ORBIT_STANDOFF: f32 = 1.35;

/// Seconds the free-fly rig must sit untouched before the orbit re-arms.
///
/// Six: long enough that a viewer pausing over a detail is not yanked away
/// the moment their hands leave the keys, short enough that a parked window
/// goes back to turning before it reads as frozen.
const ORBIT_RESUME_SECS: f32 = 6.0;

/// The idle orbit's state: whether it may ever run, how long the free-fly rig
/// has sat untouched, and the bearing the orbit stands at.
///
/// The angle is a PHASE that is stepped, not read off the clock: the clock
/// keeps counting while the viewer flies, so `elapsed * ORBIT_RATE` would
/// teleport a re-armed camera onto whatever bearing it had drifted to.
/// Holding the phase, and re-deriving it from the parked camera on each
/// re-arm, is what lets the orbit pick up from where the viewer left it.
#[derive(Resource)]
struct IdleOrbit {
    /// Never set under a capture: a capture composes its own frame, and an
    /// orbit under it would photograph a different attitude every run.
    enabled: bool,
    /// Seconds since the free-fly rig last reported input.
    idle_secs: f32,
    /// The orbit's current azimuth around [`CAMERA_TARGET`], in radians.
    angle: f32,
    /// Whether the orbit owned the camera last frame, so the first re-armed
    /// frame can read the parked bearing before the orbit writes over it.
    driving: bool,
}

impl IdleOrbit {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            // Born idle-for-long-enough, so a fresh hand-run orbits at once.
            idle_secs: ORBIT_RESUME_SECS,
            angle: 0.0,
            driving: false,
        }
    }
}

/// Hand back the camera the moment the free-fly rig is asked for anything,
/// and count the quiet seconds that re-arm the orbit once the flying stops.
///
/// Reads the rig's own input component rather than the keyboard, so it cannot
/// disagree with what actually moves the camera - and so a binding change does
/// not silently leave the orbit fighting the player.
fn track_orbit_idle(
    mut orbit: ResMut<IdleOrbit>,
    time: Res<Time>,
    q_input: Query<&WASDCameraInput>,
) {
    let touched = q_input
        .iter()
        .any(|input| input.pan != Vec2::ZERO || input.wasd != Vec2::ZERO || input.vertical != 0.0);
    if touched {
        orbit.idle_secs = 0.0;
    } else {
        // Saturated at the threshold: the timer is a re-arm gate, not a
        // stopwatch, so there is nothing to count past it.
        orbit.idle_secs = (orbit.idle_secs + time.delta_secs()).min(ORBIT_RESUME_SECS);
    }
}

/// Turn the row on a slow turntable while nobody is flying, the way the parts
/// viewer spins a focused part.
///
/// The CAMERA orbits rather than the subject: three hulls stand in a row, so
/// spinning them in place would break the composition the row exists for. Runs
/// after the free-fly rig writes its transform, because that rig writes every
/// frame and would otherwise win.
///
/// On re-arm the azimuth is read off the parked camera's own xz offset, so
/// the orbit drifts on from wherever the viewer left it. Radius and height
/// SNAP back to the composed standoff - the one framing known to hold the
/// whole row - rather than easing out from a camera flown in close.
fn orbit_idle_camera(
    mut orbit: ResMut<IdleOrbit>,
    roster: Res<Roster>,
    time: Res<Time>,
    mut q_camera: Query<&mut Transform, With<ScenarioCameraMarker>>,
) {
    if !orbit.enabled {
        return;
    }
    if orbit.idle_secs < ORBIT_RESUME_SECS {
        orbit.driving = false;
        return;
    }
    if !orbit.driving {
        let Some(parked) = q_camera.iter().next() else {
            return;
        };
        let offset = parked.translation - CAMERA_TARGET;
        orbit.angle = offset.x.atan2(offset.z);
        orbit.driving = true;
    }
    orbit.angle += time.delta_secs() * ORBIT_RATE;
    let stand = camera_position(roster.ships);
    // Further out than the composed stand. That stand frames the row from the
    // FRONT, where the line of hulls is at its narrowest; an orbit also passes
    // the broadside, where the same row is as wide as its whole span. Framing
    // for the front and then turning crops the ships off both edges.
    let radius = Vec2::new(stand.x, stand.z).length() * ORBIT_STANDOFF;
    for mut transform in &mut q_camera {
        *transform = Transform::from_translation(Vec3::new(
            radius * orbit.angle.sin(),
            stand.y,
            radius * orbit.angle.cos(),
        ))
        .looking_at(CAMERA_TARGET, Vec3::Y);
    }
}

/// Marks the seed readout.
#[derive(Component)]
struct SeedReadout;

fn spawn_readout(commands: &mut Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                SeedReadout,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Written every frame rather than on `Roster` change: the readout is spawned
/// by a command in the same run as the roster's first change, so a
/// change-gated write lands before the text exists and never runs again.
///
/// The readout follows the grave/tilde HUD cycle in a hand-run, so "no hud"
/// clears the whole top of the frame. Captures are exempt: they run at
/// cinematic from startup, and the readout is what makes a frame
/// reproducible.
fn update_readout(
    roster: Res<Roster>,
    styles: Res<GameStyles>,
    hud: Res<HudVisibility>,
    mut q_readout: Query<(&mut Text, &mut Visibility), With<SeedReadout>>,
) {
    // The style is NAMED, because a shot of a row is only evidence about a look
    // if the frame says which look it is.
    let dress = if roster.clad {
        format!(
            "clad: {}",
            style_at(&styles, roster.style).unwrap_or("bare")
        )
    } else {
        "bare".to_string()
    };
    let line = format!(
        "WFC ships - seeds {}..{} - {dress} - [R] re-roll  [C] cladding  [L] look",
        roster.seed,
        roster.seed + roster.ships as u64 - 1,
    );
    let shown = capturing() || hud.shows();
    for (mut text, mut visibility) in &mut q_readout {
        visibility.set_if_neq(if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if text.as_str() != line {
            **text = line.clone();
        }
    }
}

/// Seconds a step may sit before the run aborts naming it (llvmpipe headroom).
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 30.0;

/// Pose the harness camera on the row that is actually standing.
#[cfg(feature = "debug")]
fn frame_row(world: &mut World) {
    let ships = world.resource::<Roster>().ships;
    pose_camera(world, camera_position(ships), CAMERA_TARGET);
}

/// The driven walk: collapse a row, shoot it, strip the skin, shoot that.
#[cfg(feature = "debug")]
fn wfc_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the first row")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("frame the row")
        .on_enter(|world: &mut World| frame_row(world))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the row")
        .on_enter(|world: &mut World| shoot(world, "wfc-ships-row.png"))
        .until(shot_written("wfc-ships-row.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // Strip the cladding off the SAME seeds. This is the load path again,
        // so it also proves a collapsed row survives being torn down and
        // rebuilt in place - and it is the frame that shows what the skin is
        // actually doing, since both shots are the same ships.
        .step("strip the cladding")
        .on_enter(|world: &mut World| {
            let assets = world.resource::<GameAssets>().clone();
            let sections = world.resource::<GameSections>().clone();
            let styles = world.resource::<GameStyles>().clone();
            let next = Roster {
                clad: false,
                ..*world.resource::<Roster>()
            };
            *world.resource_mut::<Roster>() = next;
            let style = style_at(&styles, next.style);
            world.trigger(LoadScenario(wfc_row(&assets, &sections, next, style)));
        })
        .until(and(scenario_camera_present(), frames(SETTLE_FRAMES)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the bare hulls")
        .on_enter(|world: &mut World| {
            frame_row(world);
            shoot(world, "wfc-ships-bare.png");
        })
        .until(shot_written("wfc-ships-bare.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
