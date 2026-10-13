//! system_generated_stations: the open world's generated stations stand where
//! the seed put them, and come back there.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed the way the Create World modal does, and lets the 5x5x5 window stream
//! in around the spawn. Then it finds the nearest generated station - its
//! typed id, the civilization in its name, living or dead, and how far it
//! stands - jumps to its side, jumps far enough along one axis that the
//! station's cell leaves the window, and jumps back. The claim is that the
//! station is a station body wearing a [`GeneratedBodyId`] and no address,
//! that its cell's record holds it as a station while the cell is retired,
//! and that the thaw puts a NEW entity at the same pose under the same id.
//!
//! ONE SUBJECT: a GENERATED station across stream, retire and thaw. What a
//! station IS (its sections, attitude, damage floor, orbit) is
//! `system_station_body`'s and `system_station_orbit`'s range; which cluster
//! gets a station and how many a civilization keeps is the seed sweep in
//! `nova_world_base::sector_stations`.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=900 \
//!   cargo run --example system_generated_stations --features debug
//! # look for: `generated_stations: nearest station 'sector_2_0_0_cluster_1_0_0_station_0'`,
//! #           `generated_stations: sector (2, 0, 0) retired with the station`,
//! #           `generated_stations: PASS the thawed station stands where it stood`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use avian3d::prelude::{AngularVelocity, LinearVelocity, Position};
#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use nova_protocol::prelude::*;
#[cfg(feature = "debug")]
use nova_ui::widget::TextFieldValue;
#[cfg(feature = "debug")]
use nova_world::prelude::{
    desired_sectors, FrozenBodyType, FrozenSector, FrozenSectors, SectorRoot, WorldConfig,
};

#[derive(Parser)]
#[command(name = "system_generated_stations")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed, then the nearest generated station is found, reached, retired with its cell by a jump away and thawed by the jump back wearing the same id at the same pose. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings: no player's saved world may be in the way.
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

    let mut app = nova_protocol::prelude::editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // No frame-time claim: the range holds ids, components and poses.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.init_resource::<StationWatch>();
        app.add_systems(Update, hold_the_walk_clock);
        app.add_plugins(station_script());
    }

    app.run()
}

/// The seed the range plays: its nearest station stands at rest beside a
/// rock in the window's +X face, cell (2, 0, 0), 53 km from the spawn. A
/// station in the spawn's own cell could not be retired by one jump, and an
/// orbiting one would make the pose claim about its orbit.
#[cfg(feature = "debug")]
const STATION_SEED: u32 = 6;

#[cfg(feature = "debug")]
const NEW_GAME_BUTTON: &str = "New Game Button";
#[cfg(feature = "debug")]
const WORLD_NAME_FIELD: &str = "World Name Field";
#[cfg(feature = "debug")]
const WORLD_NAME: &str = "Station World";
#[cfg(feature = "debug")]
const SEED_FIELD: &str = "World Seed Field";
#[cfg(feature = "debug")]
const CREATE_WORLD_BUTTON: &str = "Create World Button";

/// How far from the station the player parks: outside the observer's hold
/// radius with room to spare, so the station spawns rather than waits.
#[cfg(feature = "debug")]
const PARK_DISTANCE: Meters = Meters(2_000.0);

/// How far a thawed station may stand from where it stood, in meters: the
/// record keeps avian's pose, and a station at rest beside a rock drifts
/// nowhere between the two reads.
#[cfg(feature = "debug")]
const POSE_TOLERANCE_M: f32 = 1.0;

/// Seconds a load or a stream gets on a software-rendered GPU. Under the
/// harness completion deadline, so a stall names its beat.
#[cfg(feature = "debug")]
const SESSION_SECS: f32 = 240.0;

/// Seconds a menu click gets.
#[cfg(feature = "debug")]
const BEAT_DEADLINE_SECS: f32 = 60.0;

/// The virtual clock's clamp on this software rasterizer, which takes
/// seconds per frame: Bevy's quarter-second default would stretch each
/// stream into minutes of frames. Nothing here reads a frame.
#[cfg(feature = "debug")]
const WALK_CLOCK_SECS: f32 = 1.0;

/// Hold the virtual clock's clamp at [`WALK_CLOCK_SECS`], every frame: a
/// scenario load hands `Time<Virtual>` back at its default.
#[cfg(feature = "debug")]
fn hold_the_walk_clock(mut time: ResMut<Time<Virtual>>) {
    let clamp = std::time::Duration::from_secs_f32(WALK_CLOCK_SECS);
    if time.max_delta() != clamp {
        time.set_max_delta(clamp);
    }
}

/// The station the range chose and what it stood as before the retire.
#[cfg(feature = "debug")]
#[derive(Resource, Default, Debug)]
struct StationWatch {
    /// The chosen station's typed id.
    id: Option<GeneratedBodyId>,
    /// The cell it stands in.
    cell: Option<SectorCoord>,
    /// The cell the retire jump lands in.
    away: Option<SectorCoord>,
    /// Where it stood, in meters, before the retire.
    position: Option<Meters3>,
    /// Its orientation before the retire.
    rotation: Option<Quat>,
    /// Whether it is dead.
    dead: bool,
    /// The entity that wore the id before the retire; the thaw must be another.
    before: Option<Entity>,
}

/// The one player ship, if exactly one stands.
#[cfg(feature = "debug")]
fn the_player(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

/// The open world's cell edge and live radius, as New Game armed them.
#[cfg(feature = "debug")]
fn world_shape(world: &World) -> (Meters, i32) {
    let config = world.resource::<WorldConfig<NovaLayeredWorld>>();
    (config.sector_edge, config.active_radius)
}

/// The live cells, as a set.
#[cfg(feature = "debug")]
fn live_cells(world: &World) -> std::collections::BTreeSet<SectorCoord> {
    world
        .try_query::<&SectorRoot>()
        .map(|mut query| query.iter(world).map(|root| root.0).collect())
        .unwrap_or_default()
}

/// The entity wearing `id`, if one does.
#[cfg(feature = "debug")]
fn body_wearing(world: &World, id: &GeneratedBodyId) -> Option<Entity> {
    let mut query = world.try_query::<(Entity, &GeneratedBodyId)>()?;
    query
        .iter(world)
        .find(|(_, worn)| *worn == id)
        .map(|(entity, _)| entity)
}

/// The chosen station's id.
#[cfg(feature = "debug")]
fn station_id(world: &World) -> GeneratedBodyId {
    world
        .resource::<StationWatch>()
        .id
        .clone()
        .expect("generated_stations: the station is chosen before it is read")
}

/// A body's pose in meters, from avian's position when it has one.
#[cfg(feature = "debug")]
fn pose_of(world: &World, entity: Entity) -> (Meters3, Quat) {
    let transform = world
        .get::<Transform>(entity)
        .expect("generated_stations: a station has a transform");
    let position = world
        .get::<Position>(entity)
        .map_or(transform.translation, |position| position.0);
    (Meters3::from_engine(position), transform.rotation)
}

/// Teleport the player to `position`, at rest: the fixture crosses cell
/// faces without flying them, the technique `system_generated_marks` uses.
#[cfg(feature = "debug")]
fn park_player(world: &mut World, position: Meters3) {
    let player = the_player(world).expect("generated_stations: exactly one player ship");
    world.entity_mut(player).insert((
        Position(position.to_engine()),
        LinearVelocity::ZERO,
        AngularVelocity::ZERO,
    ));
    info!("generated_stations: the player jumps to {position:?}");
}

/// Advance once the world is armed around one player ship and the window
/// around `centre` is exactly the live set.
#[cfg(feature = "debug")]
fn window_is(centre: SectorCoord) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        if !world.contains_resource::<WorldConfig<NovaLayeredWorld>>()
            || the_player(world).is_none()
        {
            return false;
        }
        let (_, radius) = world_shape(world);
        live_cells(world) == desired_sectors(centre, radius)
    })
}

/// Advance once the window is the one around the chosen station's cell.
#[cfg(feature = "debug")]
fn window_is_the_stations() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(cell) = world.resource::<StationWatch>().cell else {
            return false;
        };
        window_is(cell)(world)
    })
}

/// Advance once the window is the one around the retire jump's cell.
#[cfg(feature = "debug")]
fn window_is_away() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(away) = world.resource::<StationWatch>().away else {
            return false;
        };
        window_is(away)(world)
    })
}

/// Advance once a body wears the station's id.
#[cfg(feature = "debug")]
fn the_station_stands() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(id) = world.resource::<StationWatch>().id.as_ref() else {
            return false;
        };
        body_wearing(world, id).is_some()
    })
}

/// The walk, one beat per gesture.
#[cfg(feature = "debug")]
fn station_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("generated_stations: reach the main menu")
        .until(ui_node_present(NEW_GAME_BUTTON))
        .deadline(SESSION_SECS)
        .add()
        .click_named(
            "generated_stations: click New Game",
            NEW_GAME_BUTTON,
            ui_node_present(CREATE_WORLD_BUTTON),
            BEAT_DEADLINE_SECS,
        )
        .step("generated_stations: name the world and enter the seed")
        .on_enter(|world: &mut World| {
            let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
            for (name, mut value) in fields.iter_mut(world) {
                if name.as_str() == WORLD_NAME_FIELD {
                    value.0 = WORLD_NAME.to_string();
                } else if name.as_str() == SEED_FIELD {
                    value.0 = STATION_SEED.to_string();
                }
            }
        })
        .until(frames(2))
        .add()
        .click_named(
            "generated_stations: click Create",
            CREATE_WORLD_BUTTON,
            state_is(GameStates::Playing),
            SESSION_SECS,
        )
        .step("generated_stations: the window streams in around the spawn")
        .until(window_is(SectorCoord::ORIGIN))
        .deadline(SESSION_SECS)
        .add()
        .step("generated_stations: find the nearest station")
        .on_enter(find_the_nearest_station)
        .add()
        .step("generated_stations: jump to the station's side")
        .on_enter(|world: &mut World| {
            let beside = park_beside(world);
            park_player(world, beside);
        })
        .until(and(window_is_the_stations(), the_station_stands()))
        .deadline(SESSION_SECS)
        .add()
        .step("generated_stations: report the station reached")
        .on_enter(assert_the_station)
        .add()
        .step("generated_stations: jump away along the station's axis")
        .on_enter(|world: &mut World| {
            let (edge, _) = world_shape(world);
            let away = world
                .resource::<StationWatch>()
                .away
                .expect("the retire cell is chosen with the station");
            park_player(world, away.centre(edge));
        })
        .until(window_is_away())
        .deadline(SESSION_SECS)
        .add()
        .step("generated_stations: report the retire")
        .on_enter(assert_the_retire)
        .add()
        .step("generated_stations: jump back to the station's side")
        .on_enter(|world: &mut World| {
            let beside = park_beside(world);
            park_player(world, beside);
        })
        .until(and(window_is_the_stations(), the_station_stands()))
        .deadline(SESSION_SECS)
        .add()
        .step("generated_stations: the thawed station wears the same id at the same pose")
        .on_enter(assert_the_thaw)
        .add()
}

/// Where the player parks: [`PARK_DISTANCE`] from the station toward the
/// spawn, in the station's own cell.
#[cfg(feature = "debug")]
fn park_beside(world: &World) -> Meters3 {
    let watch = world.resource::<StationWatch>();
    let position = watch.position.expect("the station is chosen");
    let toward_spawn = (-position.get()).normalize_or_zero();
    let beside = Meters3(position.get() + toward_spawn * PARK_DISTANCE.get());
    let (edge, _) = world_shape(world);
    assert_eq!(
        SectorCoord::containing(beside, edge),
        watch.cell.expect("the station is chosen"),
        "generated_stations: the park point stands in the station's cell"
    );
    beside
}

/// Pick the generated station nearest the spawn, log what it is, and choose
/// the retire jump: along the axis the station's cell is farthest out on,
/// to the cell on the far side of the window whose window excludes it.
///
/// Refuses to run on a seed whose window holds no station, whose nearest
/// station stands in the spawn's cell, or whose nearest station is moving:
/// each would make the claim about something else.
#[cfg(feature = "debug")]
fn find_the_nearest_station(world: &mut World) {
    let (edge, radius) = world_shape(world);
    let mut stations: Vec<(f32, GeneratedBodyId, Entity)> = world
        .query_filtered::<(Entity, &GeneratedBodyId, &Position), With<StationMarker>>()
        .iter(world)
        .map(|(entity, id, position)| {
            (
                Meters3::from_engine(position.0)
                    .distance(Meters3::ZERO)
                    .get(),
                id.clone(),
                entity,
            )
        })
        .collect();
    stations.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let Some((distance, id, entity)) = stations.into_iter().next() else {
        panic!(
            "generated_stations: the window around the spawn holds no generated station on seed \
             {STATION_SEED}, so this claim cannot be made here"
        );
    };
    assert_eq!(
        world.get::<EntityId>(entity).map(|worn| worn.0.as_str()),
        Some(id.to_string().as_str()),
        "the typed id spells the body's EntityId"
    );
    let (position, rotation) = pose_of(world, entity);
    let cell = SectorCoord::containing(position, edge);
    assert_eq!(
        cell, id.origin,
        "a station is owned by the cell it stands in"
    );
    assert_ne!(
        cell,
        SectorCoord::ORIGIN,
        "generated_stations: the nearest station stands in the spawn's own cell on seed \
         {STATION_SEED}, which one jump cannot retire"
    );
    let velocity = world
        .get::<LinearVelocity>(entity)
        .map_or(Vec3::ZERO, |velocity| velocity.0);
    assert!(
        velocity.length() < 1.0e-3,
        "generated_stations: the nearest station moves at {velocity:?} on seed {STATION_SEED}; \
         the pose claim needs one at rest"
    );
    let name = world
        .get::<Name>(entity)
        .map_or_else(|| entity.to_string(), ToString::to_string);
    let dead = world.get::<DerelictShipMarker>(entity).is_some();

    // The retire cell: past the far face along the station's farthest axis.
    let axes = [cell.x, cell.y, cell.z];
    let (axis, _) = axes
        .iter()
        .enumerate()
        .max_by_key(|(_, index)| index.abs())
        .expect("three axes");
    let mut away = [0; 3];
    away[axis] = axes[axis] - axes[axis].signum() * (radius + 1);
    let away = SectorCoord::new(away[0], away[1], away[2]);
    assert!(
        !desired_sectors(away, radius).contains(&cell),
        "the window around {away} leaves {cell} out"
    );

    info!(
        "generated_stations: nearest station '{id}' ({name}, {}) in cell {cell}, {distance:.0} m \
         from the spawn",
        if dead { "dead" } else { "living" }
    );
    nova_probe::probe_marker(
        world,
        "outcome: the nearest generated station is found from the New Game spawn",
        serde_json::json!({
            "id": id.to_string(),
            "name": name,
            "dead": dead,
            "cell": cell.to_string(),
            "distance_m": distance,
        }),
    );
    let mut watch = world.resource_mut::<StationWatch>();
    watch.id = Some(id);
    watch.cell = Some(cell);
    watch.away = Some(away);
    watch.position = Some(position);
    watch.rotation = Some(rotation);
    watch.dead = dead;
    watch.before = Some(entity);
}

/// The player stands beside the station: it is a station body under its
/// cell's root wearing the typed id and no address, dead exactly when its
/// name says so, with its pose as read.
#[cfg(feature = "debug")]
fn assert_the_station(world: &mut World) {
    let id = station_id(world);
    let entity = body_wearing(world, &id).expect("the station stands");
    let watch = world.resource::<StationWatch>();
    assert_eq!(Some(entity), watch.before, "the same entity still stands");
    let root = world.entity(entity);
    assert!(root.contains::<StationMarker>(), "a station body");
    assert!(
        root.contains::<SpaceshipRootMarker>(),
        "a station root is a hull root"
    );
    assert!(
        !root.contains::<ScenarioAddressableMarker>(),
        "a generated station is not addressable by scenario scripts"
    );
    assert_eq!(
        root.contains::<DerelictShipMarker>(),
        watch.dead,
        "dead exactly when found dead"
    );
    assert_eq!(
        root.contains::<LootableShipMarker>(),
        watch.dead,
        "a dead station is lootable, a living one is not"
    );
    let player = the_player(world).expect("one player");
    let (player_position, _) = pose_of(world, player);
    let (position, _) = pose_of(world, entity);
    let gap = player_position.distance(position).get();
    assert!(
        (gap - PARK_DISTANCE.get()).abs() < 50.0,
        "the player parked {gap} m from the station"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the reached station is a station body wearing its generated id and no address",
        serde_json::json!({ "id": id.to_string(), "dead": watch.dead, "gap_m": gap }),
    );
    info!("generated_stations: station '{id}' reached, {gap:.0} m off");
}

/// The jump away retired the station's cell: the entity is gone, nothing
/// wears its id, and its cell's record holds it as a station.
#[cfg(feature = "debug")]
fn assert_the_retire(world: &mut World) {
    let id = station_id(world);
    let watch = world.resource::<StationWatch>();
    let cell = watch.cell.expect("the station was chosen");
    let before = watch.before.expect("the station was chosen");
    assert!(
        !live_cells(world).contains(&cell),
        "the station's cell {cell} is retired"
    );
    assert!(
        world.get_entity(before).is_err(),
        "the retire despawned the station"
    );
    assert!(
        body_wearing(world, &id).is_none(),
        "nothing wears the station's id while its cell is retired"
    );
    let frozen = match world.resource::<FrozenSectors>().get(cell) {
        Some(FrozenSector::Visited(bodies)) => bodies.iter().any(|body| {
            body.id() == Some(id.to_string().as_str())
                && matches!(body.body(), FrozenBodyType::Station(_))
        }),
        other => panic!("generated_stations: {cell} froze as {other:?}, not a visit"),
    };
    assert!(
        frozen,
        "the cell's record holds the station as a station under its id"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the station's sector retires behind the player and holds it as a station",
        serde_json::json!({ "id": id.to_string(), "cell": cell.to_string() }),
    );
    info!("generated_stations: sector {cell} retired with the station");
}

/// The return thawed the cell: a NEW entity wears the station's id, is a
/// station body, dead as before, at the pose it stood at before the retire.
#[cfg(feature = "debug")]
fn assert_the_thaw(world: &mut World) {
    let id = station_id(world);
    let watch = world.resource::<StationWatch>();
    let before = watch.before.expect("the station was chosen");
    let stood = watch.position.expect("the station was chosen");
    let faced = watch.rotation.expect("the station was chosen");
    let dead = watch.dead;
    let thawed = body_wearing(world, &id).expect("the thawed station stands");
    assert_ne!(
        thawed, before,
        "the thaw is a new entity, not the old one kept"
    );
    let root = world.entity(thawed);
    assert_eq!(
        root.get::<EntityId>().map(|worn| worn.0.as_str()),
        Some(id.to_string().as_str()),
        "the thawed body's EntityId is the same string"
    );
    assert!(root.contains::<StationMarker>(), "thawed as a station");
    assert!(
        !root.contains::<ScenarioAddressableMarker>(),
        "a thawed generated station stays unaddressable"
    );
    assert_eq!(
        root.contains::<DerelictShipMarker>(),
        dead,
        "dead exactly as before"
    );
    let (position, rotation) = pose_of(world, thawed);
    let moved = position.distance(stood).get();
    assert!(
        moved <= POSE_TOLERANCE_M,
        "the thawed station stands {moved} m from where it stood: {position:?} vs {stood:?}"
    );
    assert!(
        rotation.abs_diff_eq(faced, 1.0e-3) || rotation.abs_diff_eq(-faced, 1.0e-3),
        "the thawed station faces {rotation:?}, not {faced:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the thawed station wears the same generated id at the same pose",
        serde_json::json!({
            "id": id.to_string(),
            "before": before.to_string(),
            "thawed": thawed.to_string(),
            "moved_m": moved,
        }),
    );
    info!("generated_stations: PASS the thawed station stands where it stood");
}
