//! news_0160_release_lead: the v0.16.0 news release lead - a fresh New Game
//! as the military start, the patrol picket undocking from its allied ring
//! starting station with a held Escort marked on the waiting hauler.
//!
//! The world starts the way the Create World modal starts it, on
//! [`WORLD_SEED`] as `military`, with no save folder. Once the start has
//! docked the picket and the starting station has posted its starter board,
//! the walk asserts the hull passes the Jobs pane's hull-fit gate
//! ([`plan_job_hull_fit`]), so the Escort is eligible on the fresh board.
//! It then accepts the board's first Escort offer through
//! [`JobAcceptCommand`], the message the Jobs pane's Accept button writes,
//! and waits for the accept's hauler to stand off the station wearing the
//! `JOB-1 Escort` chip.
//! The loop opens docked with the HELM chip on the dock, presses the real
//! `dock` key to let go, then turns the look onto the hauler with the mouse
//! motion a player's hand writes, the hull following it off the station.
//! It travel-locks the hauler as a radar dwell does and presses the real
//! GOTO key, and holds the burn until the leg closes on the chip.
//!
//! The seed is pinned for its start: the starting station must be a ring
//! (it wears [`StationArchetypeType::Ring`]'s style) and its board must post
//! an Escort the hull can fly. A seed or role that gives any of these up
//! stops the run, naming it. The board draws by the role's job mix; the
//! escort loops fly this same seed-2 Military offer.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the whole walk with its
//!   asserts, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write
//!   `news-0160-release-lead.webm` (staged under `NOVA_CAPTURE_DIR`).
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0160-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0160_release_lead --features debug
//! ```

#[path = "../playable/shared/open_world_stage.rs"]
mod open_world_stage;

use bevy::{input::mouse::MouseMotion, prelude::*};
use clap::Parser;
use nova_protocol::{nova_interface::jobs::prelude::JobAcceptCommand, prelude::*};
use nova_world::prelude::SectorRoot;
use open_world_stage::{window_settled, OpenWorldStartPlugin};

#[derive(Parser)]
#[command(name = "news_0160_release_lead")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the v0.16.0 release lead: the military start undocking from its ring station with an Escort held",
    long_about = None
)]
struct Cli {
    /// The world seed. Another seed must still start at a ring station whose
    /// board posts an Escort the military hull can fly.
    #[arg(long, default_value_t = WORLD_SEED)]
    seed: u32,
}

/// The seed the lead plays: its military start docks at a ring station
/// whose starter board posts an Escort.
const WORLD_SEED: u32 = 2;

/// The start role the lead flies: an armed hull, so the starter board lists
/// the Escort.
const ROLE: &str = "military";

/// The loop this producer writes.
const LOOP_NAME: &str = "news-0160-release-lead";

/// Real seconds the world gets to stream in, dock the start and post its
/// board on a software rasterizer.
const SETUP_SECS: f32 = 600.0;

/// Real seconds the accept gets to put its hauler in the world, marked.
const ACCEPT_SECS: f32 = 120.0;

/// Frames the loop holds docked before the undock, the HELM chip on the
/// dock: one second under the 30 fps capture clock.
const DOCKED_FRAMES: u32 = 30;

/// Mouse pixels a look push writes per radian off the hauler, the most it
/// writes in one frame, and how close the look must come before the lock.
const LOOK_GAIN_PX: f32 = 40.0;
const LOOK_PUSH_PX: f32 = 10.0;
const LOOK_DONE_RAD: f32 = 0.05;

/// Real seconds the look gets to come round onto the hauler.
const LOOK_SECS: f32 = 600.0;

/// Frames the lock settles before GOTO is pressed.
const LOCK_FRAMES: u32 = 15;

/// How far GOTO must close on the hauler before the loop starts its close:
/// the turn off the station and the burn that first cancels the undock
/// drift both come before it.
const CLOSING_M: f32 = 50.0;

/// Real seconds the GOTO leg gets to close [`CLOSING_M`] on a software
/// rasterizer, which draws about one capture frame a second.
const CLOSING_SECS: f32 = 900.0;

/// Frames the loop holds the closing leg before it ends.
const CLOSING_HOLD_FRAMES: u32 = 45;

/// Real seconds the loop gets to encode once its last frame is in.
const LOOP_CLOSE_SECS: f32 = 60.0;

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = editor_app(true, None);
    app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
        bevy::ecs::error::panic,
    ));
    app.add_plugins(OpenWorldStartPlugin {
        seed: cli.seed,
        role: ROLE.to_string(),
    });
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
    app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    app.add_plugins(lead_script(cli.seed));
    app.run()
}

/// The Escort the walk accepts: its id and the hauler its accept spawns.
#[derive(Resource, Clone, Debug)]
struct HeldEscort {
    id: JobId,
    hauler: GeneratedBodyId,
}

/// The one player ship, if exactly one stands.
fn the_player(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

/// The entity wearing generated id `id`, if one does.
fn body(world: &World, id: &GeneratedBodyId) -> Option<Entity> {
    world
        .try_query::<(Entity, &GeneratedBodyId)>()?
        .iter(world)
        .find(|(_, worn)| *worn == id)
        .map(|(entity, _)| entity)
}

/// The session's starting station, once it stands.
fn start_station(world: &World) -> Option<Entity> {
    let id = world.get_resource::<OpenWorldSession>()?.start.id.clone();
    body(world, &id)
}

fn when(
    check: impl Fn(&World) -> bool + Send + Sync + 'static,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(check)
}

/// The window is in, the player is docked and the starting station has
/// posted its board.
fn ready_to_accept(world: &mut World) -> bool {
    if !window_settled(world) {
        return false;
    }
    let docked = the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some());
    docked && start_station(world).is_some_and(|station| world.get::<JobBoard>(station).is_some())
}

/// Whether the HELM chip is on the dock.
fn helm_chip_shown(world: &World) -> bool {
    let index = DOCK_VERBS
        .iter()
        .position(|verb| *verb == "HELM")
        .expect("release lead: HELM is a dock verb");
    world
        .try_query::<(&DockChip, &Node, &Visibility)>()
        .is_some_and(|mut chips| {
            chips.iter(world).any(|(chip, node, visibility)| {
                chip.0 == index
                    && node.display != Display::None
                    && *visibility != Visibility::Hidden
            })
        })
}

/// What the player's hull can do, read as the Jobs pane's gate reads it: the
/// hold's capacity and whether a live docking section and a live weapon
/// section stand on the hull.
fn hull_fit(world: &mut World, player: Entity) -> JobHullFit {
    let mut fit = JobHullFit {
        hold_capacity_g: world
            .get::<ShipInventory>(player)
            .map_or(0, ShipInventory::capacity_g),
        has_clamp: false,
        has_weapon: false,
    };
    for (&ChildOf(root), clamp, turret, bay, railgun) in world
        .query_filtered::<(
            &ChildOf,
            Has<DockingSectionMarker>,
            Has<TurretSectionMarker>,
            Has<TorpedoSectionMarker>,
            Has<RailgunSectionMarker>,
        ), (With<SectionMarker>, Without<SectionInactiveMarker>)>()
        .iter(world)
    {
        if root == player {
            fit.has_clamp |= clamp;
            fit.has_weapon |= turret || bay || railgun;
        }
    }
    fit
}

/// Assert the station is a ring and the hull passes the Jobs hull-fit gate
/// for its first Escort, then send the same command as the Accept button.
///
/// # Panics
///
/// When the starting station is no ring, its board posts no Escort or the
/// Jobs pane would hide that Escort for an unfit hull: the seed or role no
/// longer gives the lead its subject.
fn accept_the_escort(world: &mut World, seed: u32) {
    let station = start_station(world).expect("release lead: the starting station stands");
    let player = the_player(world).expect("release lead: exactly one player ship");
    let fit = hull_fit(world, player);
    let style = world
        .get::<ShipStyle>(station)
        .and_then(|style| style.0.clone());
    let ring = StationArchetypeType::Ring.style_id();
    assert_eq!(
        style.as_deref(),
        Some(ring),
        "release lead: seed {seed} starts at a station in style {style:?}, not a ring ('{ring}')"
    );
    let board = world
        .get::<JobBoard>(station)
        .expect("release lead: the starting station posted its board");
    let escort = board
        .0
        .iter()
        .find(|job| matches!(job.kind, JobType::Escort { .. }))
        .unwrap_or_else(|| {
            let kinds: Vec<&str> = board.0.iter().map(|job| job.kind.label()).collect();
            panic!("release lead: seed {seed}'s starter board posts no Escort: {kinds:?}")
        });
    let items = world.resource::<GameItems>();
    if let Err(refusal) = plan_job_hull_fit(escort, items, fit) {
        panic!(
            "release lead: the {ROLE} hull ({fit:?}) fails the Escort '{}': {refusal:?}, so the \
             starter board hides it",
            escort.id
        );
    }
    let held = HeldEscort {
        id: escort.id.clone(),
        hauler: escort
            .located_body()
            .expect("an Escort names its hauler")
            .clone(),
    };
    info!(
        "release lead: accepting the Escort '{}', hauler '{}'",
        held.id, held.hauler
    );
    world.write_message(JobAcceptCommand {
        id: held.id.clone(),
    });
    world.insert_resource(held);
}

/// The accepted Escort is held and its hauler stands under a cell root,
/// wearing the `JOB-1 Escort` chip.
fn hauler_marked(world: &World) -> bool {
    let Some(held) = world.get_resource::<HeldEscort>() else {
        return false;
    };
    let holds = world
        .get_resource::<Jobs>()
        .is_some_and(|jobs| jobs.mark_code(&held.id).as_deref() == Some("JOB-1"));
    let Some(hauler) = body(world, &held.hauler) else {
        return false;
    };
    let rooted = world
        .get::<ChildOf>(hauler)
        .is_some_and(|parent| world.get::<SectorRoot>(parent.parent()).is_some());
    let marked = world
        .get::<ObjectiveMarkerTarget>(hauler)
        .is_some_and(|tag| tag.label == "JOB-1 Escort");
    holds && rooted && marked
}

/// The player's distance to the hauler, in meters.
fn hauler_distance(world: &World) -> f32 {
    let held = world.resource::<HeldEscort>();
    let hauler = body(world, &held.hauler).expect("release lead: the hauler stands");
    let player = the_player(world).expect("release lead: exactly one player ship");
    let at = |entity| {
        Meters3::from_engine(
            world
                .get::<GlobalTransform>(entity)
                .expect("release lead: a ship has a transform")
                .translation(),
        )
    };
    at(player).distance(at(hauler)).get()
}

/// How far the active camera rig looks off the hauler, in radians: `x` to
/// the right, `y` up.
fn look_error(world: &World) -> Vec2 {
    let held = world.resource::<HeldEscort>();
    let hauler = body(world, &held.hauler).expect("release lead: the hauler stands");
    let player = the_player(world).expect("release lead: exactly one player ship");
    let at = |entity| {
        world
            .get::<GlobalTransform>(entity)
            .expect("release lead: a ship has a transform")
            .translation()
    };
    let look = world
        .try_query_filtered::<&PointRotationOutput, (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipRotationInputActiveMarker>,
        )>()
        .and_then(|mut rigs| rigs.iter(world).next().map(|output| output.0))
        .expect("release lead: a camera rig holds the look");
    let local = look.inverse() * (at(hauler) - at(player)).normalize();
    Vec2::new(
        local.x.atan2(-local.z),
        local.y.atan2(Vec2::new(local.x, local.z).length()),
    )
}

/// The distance to the hauler when GOTO engaged.
#[derive(Resource)]
struct GotoFrom(f32);

/// Travel-lock the hauler the way a radar dwell does, so GOTO flies to it.
fn lock_the_hauler(world: &mut World) {
    let held = world.resource::<HeldEscort>().clone();
    let hauler = body(world, &held.hauler).expect("release lead: the hauler stands");
    let player = the_player(world).expect("release lead: exactly one player ship");
    world
        .get_mut::<TravelLock>(player)
        .expect("release lead: the player ship carries a travel lock")
        .0 = Some(hauler);
    let from = hauler_distance(world);
    info!("release lead: locked the hauler {from:.0} m out");
    world.insert_resource(GotoFrom(from));
}

/// Assert GOTO closed on the hauler and the hauler still wears its chip.
fn assert_closing_on_the_hauler(world: &mut World) {
    let from = world.resource::<GotoFrom>().0;
    let now = hauler_distance(world);
    info!("release lead: the hauler is {now:.0} m out (was {from:.0} m)");
    assert!(
        from - now > CLOSING_M,
        "release lead: GOTO closed {:.0} m on the hauler, short of {CLOSING_M} m",
        from - now
    );
    assert!(
        hauler_marked(world),
        "release lead: the hauler lost its JOB-1 Escort chip inside the loop"
    );
}

/// The walk: into the world, accept the Escort, then the undock loop.
fn lead_script(seed: u32) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("the window streams in, the start docks and the board posts")
        // `window_settled` reads through a query state and needs the world
        // mutably, which a predicate never has, so each frame records it.
        .each(|world: &mut World, _, _| {
            if ready_to_accept(world) {
                world.insert_resource(StartReady);
            }
        })
        .until(when(|world| world.contains_resource::<StartReady>()))
        .deadline(SETUP_SECS)
        .add()
        .step("check the ring and accept the Escort")
        .on_enter(move |world: &mut World| accept_the_escort(world, seed))
        .until(when(hauler_marked))
        .deadline(ACCEPT_SECS)
        .add()
        .step("drop the overlays and the status bar")
        .on_enter(|world: &mut World| {
            hide_dev_overlays(world);
            hide_status_bar(world);
        })
        .until(and(frames(SETTLE_FRAMES), when(helm_chip_shown)))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("open the release-lead loop")
        .on_enter(|world: &mut World| loop_start(world, LOOP_NAME))
        .add()
        .step("hold docked with the HELM chip up")
        .until(frames(DOCKED_FRAMES))
        .add()
        .step("press dock to let go")
        .on_enter(press_action("dock"))
        .until(frames(1))
        .add()
        .step("release dock and wait for the free hull")
        .on_enter(release_action("dock"))
        .until(when(|world| {
            the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_none())
        }))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("the HELM chip clears")
        .until(when(|world| !helm_chip_shown(world)))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("look round onto the hauler, the hull following")
        .each(|world: &mut World, _, _| {
            let error = look_error(world);
            let push = (Vec2::new(error.x, -error.y) * LOOK_GAIN_PX).clamp_length_max(LOOK_PUSH_PX);
            world.write_message(MouseMotion { delta: push });
        })
        .until(when(|world| look_error(world).length() < LOOK_DONE_RAD))
        .deadline(LOOK_SECS)
        .add()
        .step("lock the hauler")
        .on_enter(lock_the_hauler)
        .until(frames(LOCK_FRAMES))
        .add()
        .step("press GOTO")
        .on_enter(press_action("autopilot_goto"))
        .until(frames(1))
        .add()
        .step("release GOTO and fly until the leg closes on the hauler")
        .on_enter(release_action("autopilot_goto"))
        .until(when(|world| {
            world.resource::<GotoFrom>().0 - hauler_distance(world) > CLOSING_M
        }))
        .deadline(CLOSING_SECS)
        .add()
        .step("hold the closing leg")
        .until(frames(CLOSING_HOLD_FRAMES))
        .add()
        .step("check the closing leg and the chip")
        .on_enter(assert_closing_on_the_hauler)
        .add()
        .step("close the release-lead loop")
        .on_enter(|world: &mut World| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(LOOP_CLOSE_SECS)
        .add()
}

/// The docked start and its board are in: `ready_to_accept` held once.
#[derive(Resource)]
struct StartReady;
