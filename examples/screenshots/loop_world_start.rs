//! loop_world_start: two short webm loops opening a fresh open world - the
//! news release lead and the sector-streaming explainer.
//!
//! `news-0150-release-lead`: New Game into a live window, a burn on the
//! player's own `main_drive` under the game's own chase camera, then the
//! nearest generated ship travel-locked so the inset names it.
//!
//! `news-0150-sector-streaming`: a STAGED, parked view - not the ship's own
//! camera. The player is berthed 1 km short of the current cell's +X face
//! and driven across it on real streaming. Before the crossing, the loop
//! parks on the largest body of the retiring cells, 64-96 km behind the
//! ship; after the crossing retires that cell, it cuts to the largest body of
//! the nearest arriving cell that holds one, predicted ahead of time with
//! [`nova_world::generation::generate_sector`] - the same pure call the
//! streaming system itself resolves a cell with - and holds there once the
//! real stream spawns it.
//!
//! One producer, one session: both loops play in the one world the entry
//! opens.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole session,
//!   assert the burn and the streaming window, exit clean, recording
//!   nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write both webm loops (staged
//!   under `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0150-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example loop_world_start --features debug
//! ```

#[cfg(feature = "debug")]
use avian3d::prelude::*;
#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;
#[cfg(feature = "debug")]
use nova_scenario::objects::asteroid::ASTEROID_GEOMETRIC_FACTOR_MAX;
#[cfg(feature = "debug")]
use nova_ui::widget::TextFieldValue;
#[cfg(feature = "debug")]
use nova_world::prelude::{
    desired_sectors, generate_sector, CurrentSector, SectorCoord, SectorRoot, WorldConfig,
};

#[derive(Parser)]
#[command(name = "loop_world_start")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the open-world release-lead burn and a real sector-streaming crossing",
    long_about = None
)]
struct Cli;

/// The seed the walk plays: the one `screenshot_menu` types into the world
/// setup window, so the post shows one world from setup to flight.
#[cfg(feature = "debug")]
const WORLD_SEED: u32 = 115;

#[cfg(feature = "debug")]
const NEW_GAME_BUTTON: &str = "New Game Button";
#[cfg(feature = "debug")]
const SEED_FIELD: &str = "World Seed Field";
#[cfg(feature = "debug")]
const CREATE_WORLD_BUTTON: &str = "Create World Button";

/// Backspaces that clear any seed the window opens with: it pre-fills a random
/// u32, at most ten digits.
#[cfg(feature = "debug")]
const SEED_DIGITS_MAX: usize = 10;

/// How many cells the open world keeps live: radius 2 is a 5x5x5 window.
#[cfg(feature = "debug")]
const LIVE_SECTORS: usize = 125;

/// Seconds a menu load gets on a software-rendered GPU.
#[cfg(feature = "debug")]
const MENU_SESSION_SECS: f32 = 90.0;

/// Seconds a real sector crossing or a generation wait gets: generous,
/// because lavapipe can be far slower than frame-rate real time and these
/// steps are not capped by a sim-second `elapsed()` deadline.
#[cfg(feature = "debug")]
const STREAM_DEADLINE_SECS: f32 = 240.0;

#[cfg(feature = "debug")]
const LOOP1_NAME: &str = "news-0150-release-lead";
#[cfg(feature = "debug")]
const LOOP2_NAME: &str = "news-0150-sector-streaming";

/// How long loop 1 holds the main drive: three seconds of game time under the
/// pinned capture clock, long enough for the burn to read clearly against
/// [`SPEED_DELTA_THRESHOLD_MPS`].
#[cfg(feature = "debug")]
const MAIN_DRIVE_BURN_FRAMES: u32 = 90;

/// The least speed the burn must add for loop 1's assert: comfortably above
/// drift or physics-step noise, well below what three seconds of thrust adds.
#[cfg(feature = "debug")]
const SPEED_DELTA_THRESHOLD_MPS: f32 = 10.0;

/// How short of the departure cell's +X face loop 2 berths the player.
#[cfg(feature = "debug")]
const APPROACH_DISTANCE: Meters = Meters(1_000.0);

/// The player's velocity while it closes on and crosses the boundary.
#[cfg(feature = "debug")]
const CROSSING_SPEED_MPS: f32 = 300.0;

/// How long loop 2 holds the arrived view before closing.
#[cfg(feature = "debug")]
const ARRIVAL_SETTLE_SECS: f32 = 1.0;

/// How long loop 2 holds the emptied retiring view before cutting away.
#[cfg(feature = "debug")]
const EMPTIED_VIEW_SECS: f32 = 1.0;

/// Seconds either loop's closing wait gets once its last frame is in.
#[cfg(feature = "debug")]
const LOOP_CLOSE_DEADLINE_SECS: f32 = 60.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings and saved worlds, set before the app reads
    // them: Create makes a world, and neither a player's worlds nor an
    // earlier run's "New World" may be in its way.
    std::env::set_var(
        nova_assets::storage::CONFIG_ROOT_ENV,
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "target/example-profiles/{}-{}-{}",
            env!("CARGO_CRATE_NAME"),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is after 1970")
                .as_nanos(),
        )),
    );

    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(world_script());
    }

    app.run()
}

/// The generated ship loop 1 locks, and the inset name it waits on.
#[cfg(feature = "debug")]
#[derive(Resource, Debug, Clone)]
struct LockedShip {
    name: String,
}

/// The player's speed, sampled before loop 1's burn, for the after-burn
/// assert.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct BurnSpeed(f32);

/// What loop 2 plans before opening: the cell it leaves and the cell it
/// enters, the retiring cells' live largest body (to frame and then confirm
/// gone), and the predicted arrival body (to frame once the real stream
/// spawns it).
#[cfg(feature = "debug")]
#[derive(Resource, Clone)]
struct StreamingPlan {
    arrival: SectorCoord,
    trailing_root: Entity,
    trailing_body: Entity,
    trailing_body_name: String,
    arriving_coord: SectorCoord,
    arriving_body_name: String,
    arriving_body_position: Meters3,
    arriving_body_radius: Meters,
}

/// The one player ship, if exactly one stands.
#[cfg(feature = "debug")]
fn the_player(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

/// The player's current speed, read off its avian velocity.
#[cfg(feature = "debug")]
fn player_speed_mps(world: &World) -> f32 {
    let player = the_player(world).expect("world start: exactly one player ship");
    let velocity = world
        .get::<LinearVelocity>(player)
        .expect("world start: the player ship has a LinearVelocity");
    MetersPerSecond3::from_engine(velocity.0).get().length()
}

/// Sample the player's speed before loop 1's burn.
#[cfg(feature = "debug")]
fn record_speed_before_burn(world: &mut World) {
    let speed = player_speed_mps(world);
    info!("world start: speed before the burn {speed:.1} m/s");
    world.insert_resource(BurnSpeed(speed));
}

/// Assert the burn added at least [`SPEED_DELTA_THRESHOLD_MPS`].
#[cfg(feature = "debug")]
fn assert_burn_increased_speed(world: &mut World) {
    let before = world.resource::<BurnSpeed>().0;
    let after = player_speed_mps(world);
    info!("world start: speed after the burn {after:.1} m/s (was {before:.1} m/s)");
    assert!(
        after - before > SPEED_DELTA_THRESHOLD_MPS,
        "world start: the burn only added {:.1} m/s, short of the {SPEED_DELTA_THRESHOLD_MPS} m/s \
         threshold",
        after - before
    );
}

/// Pick the nearest live generated ship that is not the player, under any
/// live `SectorRoot`, and travel-lock it the way a radar dwell does.
#[cfg(feature = "debug")]
fn lock_nearest_generated_ship(world: &mut World) {
    let player = the_player(world).expect("world start: exactly one player ship");
    let origin = world
        .get::<GlobalTransform>(player)
        .expect("world start: the player ship has a transform")
        .translation();
    let candidates: Vec<(Entity, String, Entity, Vec3)> = world
        .query_filtered::<(Entity, &Name, &ChildOf, &GlobalTransform), (
            With<SpaceshipRootMarker>,
            Without<PlayerSpaceshipMarker>,
        )>()
        .iter(world)
        .map(|(entity, name, parent, at)| {
            (entity, name.to_string(), parent.parent(), at.translation())
        })
        .collect();
    let (target, name) = candidates
        .into_iter()
        .filter(|(_, _, root, _)| world.get::<SectorRoot>(*root).is_some())
        .map(|(entity, name, _, at)| (at.distance(origin), entity, name))
        .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.2.cmp(&b.2)))
        .map(|(_, entity, name)| (entity, name))
        .unwrap_or_else(|| panic!("world start: seed {WORLD_SEED} streamed no generated ship"));
    world
        .get_mut::<TravelLock>(player)
        .expect("world start: the player ship carries a travel lock")
        .0 = Some(target);
    info!("world start: locked the nearest generated ship '{name}' {target:?}");
    world.insert_resource(LockedShip { name });
}

/// Advance once the inset caption's first line is the locked ship's name.
#[cfg(feature = "debug")]
fn the_inset_names_the_locked_ship() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate>
{
    std::sync::Arc::new(|world: &World| {
        let name = &world.resource::<LockedShip>().name;
        world
            .try_query_filtered::<&Text, With<TargetInsetCaptionMarker>>()
            .is_some_and(|mut captions| {
                captions
                    .iter(world)
                    .any(|caption| caption.0.lines().next() == Some(name.as_str()))
            })
    })
}

/// Advance once a laid-out text field named `name` reads exactly `value`.
#[cfg(feature = "debug")]
fn text_field_reads(
    name: &'static str,
    value: String,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .try_query::<(&Name, &TextFieldValue)>()
            .is_some_and(|mut fields| {
                fields
                    .iter(world)
                    .any(|(field, text)| field.as_str() == name && text.0 == value)
            })
    })
}

/// Park the scripted camera standing off `position` enough to frame a body of
/// `radius`, looking at it square on, with the flight HUD down: it reads the
/// player's ship, tens of km away, and over a parked camera it would pass the
/// staged view off as the ship's own.
#[cfg(feature = "debug")]
fn frame_body(world: &mut World, position: Meters3, radius: Meters) {
    hide_hud(world);
    let standoff = radius.get() * 4.0 + 500.0;
    let eye = position.get() + Vec3::new(0.0, standoff * 0.4, standoff);
    pose_camera(world, Meters3::new(eye.x, eye.y, eye.z), position);
}

/// Cut the scripted camera to the planned retiring cell's largest body.
#[cfg(feature = "debug")]
fn frame_trailing_body(world: &mut World) {
    let plan = world.resource::<StreamingPlan>().clone();
    let at = world
        .get::<GlobalTransform>(plan.trailing_body)
        .expect("world start: the trailing body has a transform")
        .translation();
    let radius = *world
        .get::<BodyRadius>(plan.trailing_body)
        .expect("world start: the trailing body has a BodyRadius");
    frame_body(
        world,
        Meters3::from_engine(at),
        Meters::from_engine(radius.0),
    );
}

/// Cut the scripted camera to the predicted arrival body. Its position is
/// exact; its radius is the bound [`predict_arrival_body`] frames by.
#[cfg(feature = "debug")]
fn frame_predicted_arrival(world: &mut World) {
    let plan = world.resource::<StreamingPlan>().clone();
    frame_body(
        world,
        plan.arriving_body_position,
        plan.arriving_body_radius,
    );
}

/// Pose the player 1 km short of `departure`'s +X face, facing +X, moving at
/// [`CROSSING_SPEED_MPS`] along it - the way a berth pose is written, but
/// moving rather than at rest.
#[cfg(feature = "debug")]
fn berth_short_of_face(
    world: &mut World,
    departure: SectorCoord,
    config: &WorldConfig<NovaLayeredWorld>,
) {
    let player = the_player(world).expect("world start: exactly one player ship");
    let centre = departure.centre(config.sector_edge);
    let face_x = centre.x().get() + config.sector_edge.get() * 0.5;
    let start = Meters3::new(
        face_x - APPROACH_DISTANCE.get(),
        centre.y().get(),
        centre.z().get(),
    );
    let translation = start.to_engine();
    let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::X);
    let velocity = MetersPerSecond3::new(CROSSING_SPEED_MPS, 0.0, 0.0).to_engine();

    let mut ship = world.entity_mut(player);
    if let Some(mut transform) = ship.get_mut::<Transform>() {
        transform.translation = translation;
        transform.rotation = rotation;
    }
    if let Some(mut position) = ship.get_mut::<Position>() {
        position.0 = translation;
    }
    if let Some(mut body) = ship.get_mut::<Rotation>() {
        body.0 = rotation;
    }
    if let Some(mut linear) = ship.get_mut::<LinearVelocity>() {
        linear.0 = velocity;
    }
    if let Some(mut angular) = ship.get_mut::<AngularVelocity>() {
        angular.0 = Vec3::ZERO;
    }
    info!(
        "world start: berthed the player {:.0} m short of {departure}'s +X face, moving at \
         {CROSSING_SPEED_MPS} m/s",
        APPROACH_DISTANCE.get()
    );
}

/// The live largest asteroid or planet whose `SectorRoot` is in `coords`:
/// its root, its own entity, its name, and its `BodyRadius`.
#[cfg(feature = "debug")]
fn pick_largest_live_body(
    world: &mut World,
    coords: &[SectorCoord],
) -> Option<(Entity, Entity, String, BodyRadius)> {
    let roots: Vec<(Entity, SectorCoord)> = world
        .query::<(Entity, &SectorRoot)>()
        .iter(world)
        .map(|(entity, root)| (entity, root.0))
        .filter(|(_, coord)| coords.contains(coord))
        .collect();
    let bodies: Vec<(Entity, String, BodyRadius, Entity)> = world
        .query_filtered::<(Entity, &Name, &BodyRadius, &ChildOf), Or<(
            With<AsteroidMarker>,
            With<PlanetMarker>,
        )>>()
        .iter(world)
        .map(|(entity, name, radius, parent)| (entity, name.to_string(), *radius, parent.parent()))
        .collect();
    bodies
        .into_iter()
        .filter_map(|(entity, name, radius, parent)| {
            let (root, _) = roots.iter().find(|(root, _)| *root == parent)?;
            Some((*root, entity, name, radius))
        })
        .max_by(|a, b| a.3 .0.total_cmp(&b.3 .0))
}

/// The predicted largest asteroid or planet of the arriving cell nearest
/// `arrival` that holds any body, from a pure [`generate_sector`] call against
/// the live world config: the cell's coordinate, the body's id, its position,
/// and the radius to frame it by.
///
/// Nearest rather than largest across the whole face: the stream requests
/// and materializes cells nearest-first, one a frame, and this wait is inside
/// the recording, so a far cell of the 25 could push the loop past its frame
/// cap.
///
/// An asteroid's authored radius is a designation; the mesh reaches up to
/// [`ASTEROID_GEOMETRIC_FACTOR_MAX`] times past it, so the camera is sized by
/// that bound and the whole rock stays in frame. A planet's radius already
/// includes its relief.
#[cfg(feature = "debug")]
fn predict_arrival_body(
    config: &WorldConfig<NovaLayeredWorld>,
    arrival: SectorCoord,
    coords: &[SectorCoord],
) -> Option<(SectorCoord, String, Meters3, Meters)> {
    let mut coords = coords.to_vec();
    coords.sort_by_key(|coord| {
        let (x, y, z) = (
            i64::from(coord.x - arrival.x),
            i64::from(coord.y - arrival.y),
            i64::from(coord.z - arrival.z),
        );
        (x * x + y * y + z * z, *coord)
    });
    coords.into_iter().find_map(|coord| {
        let sector = generate_sector(config, coord).unwrap_or_else(|fault| {
            panic!("world start: generate_sector({coord}) faulted: {fault}")
        });
        let asteroids = sector.asteroids().iter().map(|rock| {
            (
                rock.id.clone(),
                rock.position,
                Meters(rock.radius.get() * ASTEROID_GEOMETRIC_FACTOR_MAX),
            )
        });
        let planets = sector
            .planets()
            .iter()
            .map(|planet| (planet.id.clone(), planet.position, planet.config.radius));
        asteroids
            .chain(planets)
            .max_by(|a, b| a.2.get().total_cmp(&b.2.get()))
            .map(|(id, position, radius)| (coord, id, position, radius))
    })
}

/// Berth the player short of the departure face, then plan the crossing: the
/// retiring and arriving cells, the live largest body left behind, and the
/// predicted arrival body ahead.
#[cfg(feature = "debug")]
fn berth_and_plan_crossing(world: &mut World) {
    let config = world.resource::<WorldConfig<NovaLayeredWorld>>().clone();
    let departure = world.resource::<CurrentSector>().0;
    let arrival = departure.offset(1, 0, 0);

    berth_short_of_face(world, departure, &config);

    let retiring: Vec<SectorCoord> = desired_sectors(departure, config.active_radius)
        .difference(&desired_sectors(arrival, config.active_radius))
        .copied()
        .collect();
    let arriving: Vec<SectorCoord> = desired_sectors(arrival, config.active_radius)
        .difference(&desired_sectors(departure, config.active_radius))
        .copied()
        .collect();

    let (trailing_root, trailing_body, trailing_body_name, _) =
        pick_largest_live_body(world, &retiring).unwrap_or_else(|| {
            panic!("world start: no asteroid or planet stands in the retiring cells {retiring:?}")
        });
    let (arriving_coord, arriving_body_name, arriving_body_position, arriving_body_radius) =
        predict_arrival_body(&config, arrival, &arriving).unwrap_or_else(|| {
            panic!(
                "world start: no asteroid or planet is predicted in the arriving cells {arriving:?}"
            )
        });

    info!(
        "world start: leaving {departure} for {arrival}; trailing body '{trailing_body_name}' \
         in the retiring cells {retiring:?}; predicted arrival '{arriving_body_name}' at \
         {arriving_coord}"
    );

    world.insert_resource(StreamingPlan {
        arrival,
        trailing_root,
        trailing_body,
        trailing_body_name,
        arriving_coord,
        arriving_body_name,
        arriving_body_position,
        arriving_body_radius,
    });
}

/// Advance once `CurrentSector` reaches the planned arrival cell. Resolved
/// from [`StreamingPlan`] at the predicate's own poll rather than baked into
/// the script: the plan is not known until the crossing is staged, long
/// after `world_script` builds every step.
#[cfg(feature = "debug")]
fn crossed_into_arrival() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world.get_resource::<StreamingPlan>().is_some_and(|plan| {
            world
                .get_resource::<CurrentSector>()
                .is_some_and(|current| current.0 == plan.arrival)
        })
    })
}

/// Advance once the framed trailing root is retired.
#[cfg(feature = "debug")]
fn trailing_root_retired() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<StreamingPlan>()
            .is_some_and(|plan| world.get_entity(plan.trailing_root).is_err())
    })
}

/// Assert the framed trailing body went with its retired root.
#[cfg(feature = "debug")]
fn assert_trailing_body_retired(world: &mut World) {
    let plan = world.resource::<StreamingPlan>().clone();
    assert!(
        world.get_entity(plan.trailing_body).is_err(),
        "world start: the trailing body '{}' outlived its retired cell",
        plan.trailing_body_name
    );
    info!(
        "world start: the trailing body '{}' retired with its cell",
        plan.trailing_body_name
    );
}

/// Advance once the live root set is exactly the window around the arrival
/// cell: every retiring cell gone and every arriving cell materialized.
#[cfg(feature = "debug")]
fn window_slid_to_arrival() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let (Some(plan), Some(config)) = (
            world.get_resource::<StreamingPlan>(),
            world.get_resource::<WorldConfig<NovaLayeredWorld>>(),
        ) else {
            return false;
        };
        world.try_query::<&SectorRoot>().is_some_and(|mut roots| {
            let live: std::collections::BTreeSet<SectorCoord> =
                roots.iter(world).map(|root| root.0).collect();
            live == desired_sectors(plan.arrival, config.active_radius)
        })
    })
}

/// Advance once the planned arrival cell has a live `SectorRoot` and the
/// predicted body's name is live.
#[cfg(feature = "debug")]
fn arrival_cell_and_body_present() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate>
{
    std::sync::Arc::new(|world: &World| {
        let Some(plan) = world.get_resource::<StreamingPlan>() else {
            return false;
        };
        let root_present = world
            .try_query::<&SectorRoot>()
            .is_some_and(|mut roots| roots.iter(world).any(|root| root.0 == plan.arriving_coord));
        let body_present = world.try_query::<&Name>().is_some_and(|mut names| {
            names
                .iter(world)
                .any(|name| name.as_str() == plan.arriving_body_name)
        });
        root_present && body_present
    })
}

/// Move the pointer to the seed field's right edge.
#[cfg(feature = "debug")]
fn move_to_seed_field_edge(world: &mut World) {
    let rect = ui_node_rect(world, SEED_FIELD)
        .unwrap_or_else(|| panic!("world start: the seed field '{SEED_FIELD}' is not laid out"));
    move_cursor(Vec2::new(rect.max.x - 2.0, rect.center().y))(world);
}

/// The walk: into the world, a release-lead burn and lock, then a staged
/// sector-streaming crossing.
#[cfg(feature = "debug")]
fn world_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("reach the main menu")
        .until(ui_node_present(NEW_GAME_BUTTON))
        .deadline(MENU_SESSION_SECS)
        .add()
        .click_named(
            "click New Game",
            NEW_GAME_BUTTON,
            ui_node_present(CREATE_WORLD_BUTTON),
            BEAT_DEADLINE_SECS,
        )
        .step("move to the seed field's right edge")
        .on_enter(move_to_seed_field_edge)
        .until(frames(1))
        .add()
        .step("click the seed field to place the caret")
        .on_enter(press_mouse(MouseButton::Left))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("release and let the caret land")
        .on_enter(release_mouse(MouseButton::Left))
        .until(pointer_released())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("clear the pre-filled seed")
        .on_enter(|world: &mut World| {
            press_edit_key(Key::End)(world);
            for _ in 0..SEED_DIGITS_MAX {
                press_edit_key(Key::Backspace)(world);
            }
        })
        .until(frames(1))
        .add()
        .step("type the seed")
        .on_enter(type_text(WORLD_SEED.to_string()))
        .until(text_field_reads(SEED_FIELD, WORLD_SEED.to_string()))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .click_named(
            "click Create",
            CREATE_WORLD_BUTTON,
            state_is(GameStates::Playing),
            MENU_SESSION_SECS,
        )
        .step("the window streams around the player")
        .until(std::sync::Arc::new(|world: &World| {
            the_player(world).is_some()
                && world
                    .try_query_filtered::<(), With<SectorRoot>>()
                    .is_some_and(|mut roots| roots.iter(world).count() == LIVE_SECTORS)
        }))
        .deadline(MENU_SESSION_SECS)
        .add()
        .step("assert the typed seed and hide the overlays")
        .on_enter(|world: &mut World| {
            assert_eq!(
                world.resource::<OpenWorldSession>().seed,
                WORLD_SEED,
                "world start: Create must start the typed seed"
            );
            hide_dev_overlays(world);
            hide_status_bar(world);
        })
        .add()
        // ---- Loop 1: news-0150-release-lead ----
        .step("sample speed before the burn")
        .on_enter(record_speed_before_burn)
        .until(frames(1))
        .add()
        .step("open the release-lead loop")
        .on_enter(|world: &mut World| loop_start(world, LOOP1_NAME))
        .add()
        .step("hold the main drive")
        .on_enter(press_action("main_drive"))
        .until(frames(MAIN_DRIVE_BURN_FRAMES))
        .add()
        .step("release the main drive and check the burn")
        .on_enter(release_action("main_drive"))
        .on_enter(assert_burn_increased_speed)
        .until(frames(1))
        .add()
        .step("lock the nearest generated ship")
        .on_enter(lock_nearest_generated_ship)
        .until(the_inset_names_the_locked_ship())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("settle on the locked target")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("close the release-lead loop")
        .on_enter(|world: &mut World| loop_end(world, LOOP1_NAME))
        .until(loop_written(LOOP1_NAME))
        .deadline(LOOP_CLOSE_DEADLINE_SECS)
        .add()
        // ---- Loop 2: news-0150-sector-streaming ----
        .step("berth short of the departure face and plan the crossing")
        .on_enter(berth_and_plan_crossing)
        .until(frames(2))
        .add()
        .step("park the camera on the trailing body")
        .on_enter(frame_trailing_body)
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("open the sector-streaming loop")
        .on_enter(|world: &mut World| loop_start(world, LOOP2_NAME))
        .add()
        .step("hold the main drive across the boundary")
        .on_enter(press_action("main_drive"))
        .until(crossed_into_arrival())
        .deadline(STREAM_DEADLINE_SECS)
        .add()
        .step("release the main drive after crossing")
        .on_enter(release_action("main_drive"))
        .until(frames(1))
        .add()
        .step("the trailing cell retires")
        .until(trailing_root_retired())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("the framed body went with it")
        .on_enter(assert_trailing_body_retired)
        .add()
        .step("hold the emptied view")
        .until(elapsed(EMPTIED_VIEW_SECS))
        .add()
        .step("cut to the predicted arrival")
        .on_enter(frame_predicted_arrival)
        .until(frames(2))
        .add()
        .step("wait for the arriving cell and its body")
        .until(arrival_cell_and_body_present())
        .deadline(STREAM_DEADLINE_SECS)
        .add()
        .step("settle on the arrived body")
        .until(elapsed(ARRIVAL_SETTLE_SECS))
        .add()
        .step("close the sector-streaming loop")
        .on_enter(|world: &mut World| loop_end(world, LOOP2_NAME))
        .until(loop_written(LOOP2_NAME))
        .deadline(LOOP_CLOSE_DEADLINE_SECS)
        .add()
        // After the loop: the rest of the 25 arriving cells take as long as
        // the pool needs, and the recording must not wait on them.
        .step("the window slides to the arrival cell")
        .until(window_slid_to_arrival())
        .deadline(STREAM_DEADLINE_SECS)
        .add()
}
