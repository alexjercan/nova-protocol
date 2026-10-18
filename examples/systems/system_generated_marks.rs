//! system_generated_marks: an objective mark on a GENERATED body outlives its
//! sector.
//!
//! Boots the shipped app (via [`editor_app`]), enables the example mod, and
//! starts its `example_survey` chapter the way the Scenarios list does. The
//! chapter spawns its own player ship at the origin by `SpawnScenarioObject`,
//! not through `SpawnStartRole`, so no `OpenWorldSession` stands and nothing
//! docks the player at a starting station: the window streams in around the
//! origin from the start. Then it moves a generated rock from the window's
//! -X face into the chapter's survey sphere, so the chapter's own `OnEnter`
//! handler marks it through the `$other_id` payload binding - the only way a
//! handler can name a body the world generator placed. The player then
//! jumps one cell along +X, which retires the rock's sector, and jumps back,
//! which thaws it. The claim is that the body comes back wearing the same
//! [`GeneratedBodyId`] and the same marker chip, with the handler having
//! fired exactly once.
//!
//! ONE SUBJECT: addressing a streamed body across retire and thaw. What a
//! sector contains is `system_world_sectors`' range, the `bounds` block is
//! `system_world_bounds`', and a mark across save and load is no range's yet:
//! the save does not carry it.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=900 \
//!   cargo run --example system_generated_marks --features debug
//! # look for: `generated_marks: contact 'sector_n2_..' marked CONTACT`,
//! #           `generated_marks: sector (-2, 0, 0) retired with the contact`,
//! #           `generated_marks: PASS the thawed contact wears the mark`,
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
use nova_world::prelude::{
    desired_sectors, FrozenSector, FrozenSectors, SectorRoot, WorldConfig, WorldObserver,
};

#[derive(Parser)]
#[command(name = "system_generated_marks")]
#[command(version = "1.0.0")]
#[command(
    about = "The example mod's survey chapter: a generated rock drifts into the survey sphere and is marked through the payload binding, its sector retires behind a one-cell jump and thaws on the way back, and the thawed body wears the same id and the same mark. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings: the example mod starts disabled on a fresh
    // profile, and no player's enabled set or saved world may be in the way.
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
        // No frame-time claim: the range holds ids, components and counts.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.init_resource::<SurveyWatch>();
        app.add_plugins(survey_script());
    }

    app.run()
}

/// The mod and the chapter under test, as the example mod ships them.
#[cfg(feature = "debug")]
const EXAMPLE_MOD: &str = "example";
#[cfg(feature = "debug")]
const SCENARIO: &str = "example_survey";
/// The chapter's survey sphere, as it authors it: its area id and centre.
#[cfg(feature = "debug")]
const SURVEY_AREA_ID: &str = "survey";
#[cfg(feature = "debug")]
const SURVEY_CENTRE: Meters3 = Meters3::new(-64_000.0, 0.0, 12_000.0);
/// The cell the sphere stands in: the -X face of the window around the
/// origin, which one cell of +X retires.
#[cfg(feature = "debug")]
const SURVEY_CELL: SectorCoord = SectorCoord::new(-2, 0, 0);
/// The chip text the chapter's `OnEnter` attaches.
#[cfg(feature = "debug")]
const LABEL: &str = "CONTACT";
/// The counter the chapter's `OnEnter` advances.
#[cfg(feature = "debug")]
const CONTACTS_VAR: &str = "contacts";
/// How far from the sphere's centre no other body may stand before the rock
/// is moved there, so the move is an entry and not a collision.
#[cfg(feature = "debug")]
const CLEAR_RADIUS: Meters = Meters(3_000.0);

/// Seconds a load or a stream gets on a software-rendered CI GPU. Under the
/// harness completion deadline, so a stall names its beat.
#[cfg(feature = "debug")]
const SESSION_SECS: f32 = 240.0;

/// The contact the range chose and what it stood as before the retire.
#[cfg(feature = "debug")]
#[derive(Resource, Default, Debug)]
struct SurveyWatch {
    /// The chosen body's typed id.
    id: Option<GeneratedBodyId>,
    /// The entity that wore it before the retire; the thaw must be another.
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

#[cfg(feature = "debug")]
fn count<M: Component>(world: &World) -> usize {
    world
        .try_query_filtered::<(), With<M>>()
        .map_or(0, |mut q| q.iter(world).count())
}

/// The open world's cell edge and live radius, as the chapter armed them.
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

/// The chosen contact's id.
#[cfg(feature = "debug")]
fn contact_id(world: &World) -> GeneratedBodyId {
    world
        .resource::<SurveyWatch>()
        .id
        .clone()
        .expect("generated_marks: the contact is chosen before it is read")
}

#[cfg(feature = "debug")]
fn contacts_var(world: &World) -> f64 {
    match world
        .resource::<NovaEventWorld>()
        .get_variable(CONTACTS_VAR)
    {
        Some(VariableLiteral::Number(value)) => *value,
        other => panic!("generated_marks: '{CONTACTS_VAR}' is {other:?}, not a number"),
    }
}

/// Teleport the player to `position`, at rest: the fixture crosses a cell
/// face without flying it, the technique `system_world_resume` uses.
#[cfg(feature = "debug")]
fn park_player(position: Meters3) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let player = the_player(world).expect("generated_marks: exactly one player ship");
        world.entity_mut(player).insert((
            Position(position.to_engine()),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
        ));
        info!("generated_marks: the player jumps to {position:?}");
    }
}

/// Advance once the world is armed around one player ship.
#[cfg(feature = "debug")]
fn armed_around_a_player() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world.contains_resource::<WorldConfig<NovaLayeredWorld>>() && the_player(world).is_some()
    })
}

/// Advance once the live window is exactly the one around `centre`.
#[cfg(feature = "debug")]
fn window_is(centre: SectorCoord) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        if !world.contains_resource::<WorldConfig<NovaLayeredWorld>>() {
            return false;
        }
        let (_, radius) = world_shape(world);
        live_cells(world) == desired_sectors(centre, radius)
    })
}

/// Advance once a body wears the contact's id AND the mark.
#[cfg(feature = "debug")]
fn the_contact_is_marked() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(id) = world.resource::<SurveyWatch>().id.as_ref() else {
            return false;
        };
        body_wearing(world, id).is_some_and(|entity| {
            world
                .get::<ObjectiveMarkerTarget>(entity)
                .is_some_and(|marker| marker.label == LABEL)
        })
    })
}

/// The walk, one beat per gesture.
#[cfg(feature = "debug")]
fn survey_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("generated_marks: reach the main menu")
        .until(state_is(GameStates::MainMenu))
        .deadline(SESSION_SECS)
        .add()
        // The example mod ships disabled; the merge re-runs on the enabled set.
        .step("generated_marks: enable the example mod")
        .on_enter(|world: &mut World| {
            world
                .resource_mut::<EnabledMods>()
                .0
                .insert(EXAMPLE_MOD.to_string());
        })
        .until(frames(SETTLE_FRAMES * 2))
        .add()
        .step("generated_marks: start the survey chapter the way the picker does")
        .on_enter(|world: &mut World| {
            let listed = world
                .resource::<GameScenarios>()
                .get(SCENARIO)
                .unwrap_or_else(|| panic!("generated_marks: '{SCENARIO}' is not registered"))
                .picker_lists();
            assert!(listed, "a fixed-seed chapter is a picker row");
            world.resource_mut::<NewGameScenario>().0 = Some(SCENARIO.to_string());
            *world.resource_mut::<GameMode>() = GameMode::NewGame;
            world
                .resource_mut::<NextState<GameStates>>()
                .set(GameStates::Playing);
        })
        .until(state_is(GameStates::Playing))
        .deadline(SESSION_SECS)
        .add()
        .step("generated_marks: the world arms around the player")
        .until(armed_around_a_player())
        .deadline(SESSION_SECS)
        .add()
        .step("generated_marks: the window streams in around the origin")
        .until(window_is(SectorCoord::ORIGIN))
        .deadline(SESSION_SECS)
        .add()
        .step("generated_marks: a rock of the -X face drifts into the survey sphere")
        .on_enter(move_a_rock_into_the_sphere)
        .until(scenario_variable_is(CONTACTS_VAR, 1.0))
        .deadline(SESSION_SECS)
        .add()
        .step("generated_marks: the contact is marked through the binding")
        .until(the_contact_is_marked())
        .deadline(SESSION_SECS)
        .add()
        .step("generated_marks: report the mark")
        .on_enter(assert_the_mark)
        .add()
        .step("generated_marks: jump one cell along +X")
        .on_enter(|world: &mut World| {
            let (edge, _) = world_shape(world);
            park_player(SectorCoord::new(1, 0, 0).centre(edge))(world);
        })
        .until(window_is(SectorCoord::new(1, 0, 0)))
        .deadline(SESSION_SECS)
        .add()
        .step("generated_marks: report the retire")
        .on_enter(assert_the_retire)
        .add()
        .step("generated_marks: jump back to the origin")
        .on_enter(park_player(Meters3::ZERO))
        .until(and(window_is(SectorCoord::ORIGIN), the_contact_is_marked()))
        .deadline(SESSION_SECS)
        .add()
        .step("generated_marks: the thawed contact wears the same id and the mark")
        .on_enter(assert_the_thaw)
        .add()
}

/// Pick a generated rock the -X face holds and move it, at rest, to the
/// centre of the survey sphere, which stands in that face. The chapter's
/// `OnEnter` does the marking; this only makes the entry happen.
///
/// Refuses to run on a seed whose face holds no rock, or whose sphere already
/// has a body near its centre: either would make the claim about something
/// other than an entry.
#[cfg(feature = "debug")]
fn move_a_rock_into_the_sphere(world: &mut World) {
    let (edge, _) = world_shape(world);
    let too_close: Vec<String> = world
        .query_filtered::<(&EntityId, &Position), Without<ScenarioAreaMarker>>()
        .iter(world)
        .filter(|(_, position)| {
            Meters3::from_engine(position.0).distance(SURVEY_CENTRE) < CLEAR_RADIUS
        })
        .map(|(id, _)| id.0.clone())
        .collect();
    assert!(
        too_close.is_empty(),
        "generated_marks: the survey sphere must stand clear on this seed, but {too_close:?} \
         are within {} m of it",
        CLEAR_RADIUS.0
    );
    assert_eq!(
        SectorCoord::containing(SURVEY_CENTRE, edge),
        SURVEY_CELL,
        "the chapter's sphere stands in the -X face"
    );

    // Rocks of the face, the sphere's own cell first, then the rest of the
    // face in id order, so the pick is the same on every run.
    let mut rocks: Vec<(GeneratedBodyId, Entity)> = world
        .query_filtered::<(Entity, &GeneratedBodyId), With<AsteroidMarker>>()
        .iter(world)
        .filter(|(_, id)| id.origin.x == SURVEY_CELL.x)
        .map(|(entity, id)| (id.clone(), entity))
        .collect();
    rocks.sort_by_key(|(id, _)| (id.origin != SURVEY_CELL, id.clone()));
    let Some((id, entity)) = rocks.into_iter().next() else {
        panic!(
            "generated_marks: the -X face holds no generated rock on seed {}, so this claim \
             cannot be made here",
            world.resource::<WorldConfig<NovaLayeredWorld>>().seed
        );
    };
    assert_eq!(
        world.get::<EntityId>(entity).map(|worn| worn.0.as_str()),
        Some(id.to_string().as_str()),
        "the typed id spells the body's EntityId"
    );
    assert!(
        world.get::<ObjectiveMarkerTarget>(entity).is_none(),
        "the rock starts unmarked"
    );
    assert_eq!(contacts_var(world), 0.0, "no contact before the entry");
    world.entity_mut(entity).insert((
        Position(SURVEY_CENTRE.to_engine()),
        LinearVelocity::ZERO,
        AngularVelocity::ZERO,
    ));
    info!("generated_marks: contact '{id}' ({entity}) moves into the survey sphere");
    let mut watch = world.resource_mut::<SurveyWatch>();
    watch.id = Some(id);
    watch.before = Some(entity);
}

/// The chapter's `OnEnter` fired once for the moved rock, the chip is on it
/// with the authored label, and the mark is remembered by typed id.
#[cfg(feature = "debug")]
fn assert_the_mark(world: &mut World) {
    let id = contact_id(world);
    let entity = body_wearing(world, &id).expect("the contact stands");
    assert_eq!(
        Some(entity),
        world.resource::<SurveyWatch>().before,
        "the marked body is the rock that was moved"
    );
    let label = world
        .get::<ObjectiveMarkerTarget>(entity)
        .map(|marker| marker.label.clone());
    assert_eq!(label.as_deref(), Some(LABEL), "the authored chip text");
    assert_eq!(contacts_var(world), 1.0, "the handler fired once");
    assert!(
        world.get::<ScenarioAddressableMarker>(entity).is_none(),
        "a generated body is reached without the addressable marker"
    );
    assert_eq!(
        count::<WorldObserver>(world),
        1,
        "the world still streams around the one player ship"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the first survey contact is a generated body marked by payload binding",
        serde_json::json!({ "id": id.to_string(), "label": LABEL, "area": SURVEY_AREA_ID }),
    );

    let remembered = world.resource::<GeneratedMarks>().get(&id).cloned();
    assert_eq!(remembered.as_deref(), Some(LABEL), "the mark is remembered");
    nova_probe::probe_marker(
        world,
        "outcome: the mark is remembered by the body's typed id",
        serde_json::json!({ "id": id.to_string(), "origin": id.origin.to_string() }),
    );
    info!("generated_marks: contact '{id}' marked {LABEL}");
}

/// The +X jump retired the face: the contact's entity is gone, nothing wears
/// its id, its cell's frozen record holds it, and the mark is still
/// remembered.
#[cfg(feature = "debug")]
fn assert_the_retire(world: &mut World) {
    let id = contact_id(world);
    let before = world
        .resource::<SurveyWatch>()
        .before
        .expect("the contact was chosen");
    assert!(
        !live_cells(world).contains(&SURVEY_CELL),
        "the sphere's cell {SURVEY_CELL} is retired"
    );
    assert!(
        world.get_entity(before).is_err(),
        "the retire despawned the contact"
    );
    assert!(
        body_wearing(world, &id).is_none(),
        "nothing wears the contact's id while its cell is retired"
    );
    let frozen = match world.resource::<FrozenSectors>().get(SURVEY_CELL) {
        Some(FrozenSector::Visited(bodies)) => bodies
            .iter()
            .any(|body| body.id() == Some(id.to_string().as_str())),
        other => panic!("generated_marks: {SURVEY_CELL} froze as {other:?}, not a visit"),
    };
    assert!(frozen, "the cell's record holds the contact under its id");
    assert_eq!(
        world
            .resource::<GeneratedMarks>()
            .get(&id)
            .map(String::as_str),
        Some(LABEL),
        "the mark outlives the body"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the marked body's sector retires behind the player and takes the body",
        serde_json::json!({ "id": id.to_string(), "cell": SURVEY_CELL.to_string() }),
    );
    info!("generated_marks: sector {SURVEY_CELL} retired with the contact");
}

/// The return thawed the face: a NEW entity wears the contact's id and the
/// remembered chip, with the handler still at one firing.
#[cfg(feature = "debug")]
fn assert_the_thaw(world: &mut World) {
    let id = contact_id(world);
    let before = world
        .resource::<SurveyWatch>()
        .before
        .expect("the contact was chosen");
    let thawed = body_wearing(world, &id).expect("the thawed contact stands");
    assert_ne!(
        thawed, before,
        "the thaw is a new entity, not the old one kept"
    );
    assert_eq!(
        world.get::<EntityId>(thawed).map(|worn| worn.0.as_str()),
        Some(id.to_string().as_str()),
        "the thawed body's EntityId is the same string"
    );
    let label = world
        .get::<ObjectiveMarkerTarget>(thawed)
        .map(|marker| marker.label.clone());
    assert_eq!(
        label.as_deref(),
        Some(LABEL),
        "the chip is back with its label"
    );
    assert!(
        world.get::<AsteroidMarker>(thawed).is_some(),
        "the contact thawed as a rock"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the thawed body wears the same generated id and the remembered mark",
        serde_json::json!({
            "id": id.to_string(),
            "label": LABEL,
            "before": before.to_string(),
            "thawed": thawed.to_string(),
        }),
    );

    assert_eq!(
        contacts_var(world),
        1.0,
        "the `once` handler did not fire again; the chip is the remembered mark"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the remembered mark needs no second event",
        serde_json::json!({ "contacts": 1 }),
    );
    info!("generated_marks: PASS the thawed contact wears the mark");
}
