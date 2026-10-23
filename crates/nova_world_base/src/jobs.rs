//! What posts a [`JobBoard`] on every living generated station the open
//! world streams, what an accepted job puts in the world, and what marks a
//! held job done.
//!
//! Each slot's offer is drawn once, when the slot posts, from the world
//! seed, the station's id, the slot's posting ([`JobPostings`]), the cluster
//! types of its cell, the start role's job mix, the board's standing offers
//! and the [`JobField`] of wrecks and hostile hulls standing in the station's
//! cell and the 26 cells around it, less every body a held job or another
//! unexpired offer names. The postings keep the offer as drawn, so a looted
//! wreck, a held job elsewhere, a thaw or a Load never changes a standing
//! offer. The board is put on the station root on the first frame every one
//! of those 27 cells is live and settled, and again whenever a slot renews:
//! an offer the player does not hold that stood longer than the price
//! table's `jobs.offer_seconds` of play, or whose job was handed in or
//! dropped, moves to its next posting, the bodies its old posting could have
//! put in the world are taken out, and the slot posts a new offer under a
//! new id ([`post_station_boards`]). A held job pins its slot. A dropped
//! job's posting is retired and never accepted again: a second hauler under
//! the first one's id would sever wrecks under the first one's wreck ids,
//! and a wreck frozen in another cell makes the save refuse the duplicate.
//! The player's record of the jobs taken off it is
//! [`nova_gameplay::prelude::Jobs`]; the postings and the jobs are saved
//! with the world and `chart::clear_chart` clears both.
//!
//! An Escort puts a hauler and its raiders in the world on the accept and a
//! Distress call puts its caller there ([`spawn_job_bodies`]); every body is a
//! generated hull under the poster's cell root with a [`GeneratedBodyId`] the
//! job names, so the streaming layer adopts, freezes and saves it like any
//! cell body. The hauler waits off the poster until the player stands
//! undocked from it, then flies its leg; the leg is kept in the jobs, so a
//! thaw or a Load, which keep no helm order, gives the hauler its order again
//! ([`fly_escort_legs`]). Each located kind is marked done by one observer
//! over one existing `nova_events` kind: a recovery on `onundocked`
//! ([`recover_on_undock`]), an escort on `onshipordercomplete`
//! ([`escort_on_arrival`]), a distress call on `ondocked` or `onneutralized`
//! ([`distress_on_answer`]), a neutralize on `onneutralized` or `ondestroyed`
//! ([`neutralize_on_kill`]). A held job is dropped, and its spawned bodies
//! taken out, on the `onneutralized` or `ondestroyed` of what retires it
//! ([`drop_on_loss`]): its poster, done or not, or its hauler, its wreck or
//! its genuine caller before the job is done, and an escort whose hauler's
//! leg order fails (`onshiporderfailed`): a hauler that cannot fly its leg
//! would hold the job for good. A done job keeps its hand-in.
//! Every standing body a held job sends the player to wears the HUD's
//! objective chip as `JOB-n` ([`mark_held_job_targets`]).

use std::collections::{BTreeMap, BTreeSet};

use bevy::{ecs::entity::EntityHashSet, prelude::*};
use nova_events::prelude::{
    EntityId, EventKind, GameEvent, GeneratedBodyId, Meters, Meters3, MetersPerSecond3,
    OnDestroyedEvent, OnDockedEvent, OnNeutralizedEvent, OnShipOrderCompleteEvent,
    OnShipOrderFailedEvent, OnUndockedEvent, SectorCoord,
};
use nova_gameplay::prelude::{
    station_job_offers, station_offers, unit_sphere_point, Allegiance, DerelictShipMarker, Fnv32,
    GameItems, GamePriceTable, Job, JobAccepted, JobBoard, JobDone, JobField, JobHull, JobId,
    JobMixType, JobOfferRule, JobPostings, JobType, JobWreck, Jobs, NeutralizedMarker,
    ObjectiveMarkerTarget, PlayerSpaceshipMarker, SeedStream, ShipInventory, ShipRoleType,
    SlotPosting, SpaceshipRootMarker, StarterBoardMarker, StationMarker, StationServiceType,
    StationServices, JOBS_PER_BOARD,
};
use nova_scenario::prelude::{
    base_scenario_object, spaceship_scenario_object, AIControllerConfig, BaseScenarioObjectConfig,
    GameStartRoles, OpenWorldSession, ShipDesignSource, SpaceshipConfig, SpaceshipController,
};
use nova_ship::prelude::{
    DockedShip, DockingConnection, ShipCapabilities, ShipHelmOrder, ShipOrderDirective,
    ShipOrderHelmAuthority, ShipOrderReports,
};
use nova_world::prelude::{
    PendingSectorShip, SectorClusterTypes, SectorRoot, SectorShipConditionType, WorldClock,
    WorldConfig,
};

use crate::{
    civilizations::CivilizationField,
    sector_ships::ship_stock,
    ship_layout::{generate_ship, ShipLayoutRequest},
    NovaLayeredWorld,
};

/// How far from the poster an escort's hauler starts and a distress caller
/// waits, and how long the hauler's leg is: the mark lies this far out on the
/// other side.
const JOB_LEG: Meters = Meters(10_000.0);
/// How far from the poster the hauler starts its leg.
const HAULER_STANDOFF: Meters = Meters(3_000.0);
/// Where along the leg the raiders lie in wait, as a share of it.
const RAIDER_AMBUSH_SHARE: f32 = 0.6;
/// How far apart the raiders stand along the leg.
const RAIDER_SPACING: Meters = Meters(600.0);
/// How far off the leg's line the raiders lie in wait: with the ambush
/// share and the spacing, every raider waits beyond its engage range of the
/// waiting hauler and within it of the leg.
const RAIDER_LATERAL: Meters = Meters(2_000.0);
/// How far a raider leaves its wait for a target.
const RAIDER_ENGAGE_RANGE: Meters = Meters(3_000.0);
/// Seconds of play a raider holds fire after it spawns, so a player who
/// undocks at once is not met by fire on the leg's first frame.
const RAIDER_ENGAGE_DELAY: f32 = 5.0;
/// How far a job body must stand from every generated body when it spawns.
const JOB_SPAWN_CLEARANCE: Meters = Meters(2_000.0);
/// How many seeded directions a spawn tries before it takes the clearest.
const JOB_SPAWN_TRIES: u32 = 8;
/// The tether a raider chases its prey within, from where it lay in wait.
const RAIDER_LEASH: Meters = Meters(6_000.0);
/// What a job's hull spawns as when no civilization reaches the poster: the
/// poster itself was generated by one, so this is an assembly fault.
const NO_CIVILIZATION: &str = "no civilization reaches the posting station";

/// Post the board on every generated station that offers
/// [`StationServiceType::Jobs`], once the station's cell and the 26 cells
/// around it are live and settled: every root stands, no ship of theirs is
/// still held as a [`PendingSectorShip`], and every ship root of theirs
/// carries its hold. A station with no board yet draws its first offers
/// and posts them, stamped now. On every such frame each slot the player
/// does not hold whose job was handed in, or whose offer stood past the
/// table's `jobs.offer_seconds` of play, renews: every live body its old
/// posting could have named is taken out of the world and the slot draws its
/// next posting's offer beside the standing ones, over the field less every
/// body [`JobPostings::named_bodies`] gives. Stations draw in id order, so a
/// board posted earlier on the frame counts for the next. A station with a
/// renewed slot, or with no board on its root yet, after a thaw or a Load,
/// carries its saved offers as its [`JobBoard`]. A dead
/// station posts nothing. The mix is the session's start role's; a generated
/// scenario that runs with no session, a fixed-seed world no New Game
/// opened, posts the trade mix. The session's starting station carries the
/// [`StarterBoardMarker`] beside its board; no other station does. The
/// marker is the whole gate: the board itself is the same draw everywhere.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the config, the table, the clock, the session, the jobs, the postings, the stations, the roots and the field"
)]
#[expect(
    clippy::type_complexity,
    reason = "the station, the wreck and the hull queries each filter on three markers"
)]
pub(crate) fn post_station_boards(
    mut commands: Commands,
    config: Res<WorldConfig<NovaLayeredWorld>>,
    items: Res<GameItems>,
    table: Res<GamePriceTable>,
    clock: Res<WorldClock>,
    session: Option<Res<OpenWorldSession>>,
    roles: Res<GameStartRoles>,
    jobs: Res<Jobs>,
    mut postings: ResMut<JobPostings>,
    stations: Query<
        (
            Entity,
            &GeneratedBodyId,
            &ChildOf,
            Option<&StationServices>,
            Has<DerelictShipMarker>,
            Option<&JobBoard>,
        ),
        With<StationMarker>,
    >,
    bodies: Query<(Entity, &GeneratedBodyId)>,
    roots: Query<(Entity, &SectorRoot, &SectorClusterTypes)>,
    unsettled: Query<
        &ChildOf,
        Or<(
            With<PendingSectorShip>,
            (With<SpaceshipRootMarker>, Without<ShipInventory>),
        )>,
    >,
    wrecks: Query<
        (&GeneratedBodyId, &Name, &ShipInventory, &ChildOf),
        (
            With<SpaceshipRootMarker>,
            With<DerelictShipMarker>,
            Without<StationMarker>,
        ),
    >,
    hulls: Query<
        (&GeneratedBodyId, &Name, &Allegiance, &ChildOf),
        (
            With<SpaceshipRootMarker>,
            Without<DerelictShipMarker>,
            Without<NeutralizedMarker>,
            Without<StationMarker>,
        ),
    >,
) {
    if stations.is_empty() {
        return;
    }
    let live: BTreeMap<SectorCoord, Entity> = roots
        .iter()
        .map(|(entity, root, _)| (root.0, entity))
        .collect();
    let unsettled: BTreeSet<Entity> = unsettled.iter().map(ChildOf::parent).collect();
    let mix = session.as_deref().map_or(JobMixType::Trade, |session| {
        roles
            .get_role(&session.role)
            .unwrap_or_else(|| panic!("post_station_boards: no start role '{}'", session.role))
            .jobs
    });
    let now = clock.play_seconds;
    let rule = table.0.jobs;
    // One pass over the live bodies: a renewed slot looks its few possible
    // ids up here instead of scanning every body again.
    let worn: BTreeMap<&GeneratedBodyId, Entity> = bodies
        .iter()
        .map(|(body, body_id)| (body_id, body))
        .collect();
    // Id order: a board posted earlier this frame names its bodies before
    // the next station draws, the same in every process.
    let mut ordered: Vec<_> = stations.iter().collect();
    ordered.sort_by(|a, b| a.1.cmp(b.1));
    // Every body a renewal takes out this frame: the despawn is deferred, so
    // a later station's queries still see it.
    let mut retired: BTreeSet<GeneratedBodyId> = BTreeSet::new();
    for (entity, id, parent, services, derelict, posted) in ordered {
        if !station_offers(services, derelict, StationServiceType::Jobs) {
            continue;
        }
        let Ok((_, root, clusters)) = roots.get(parent.parent()) else {
            continue;
        };
        let around = cube_around(root.0);
        let Some(around): Option<Vec<Entity>> = around
            .iter()
            .map(|coord| live.get(coord).copied())
            .collect()
        else {
            continue;
        };
        if around.iter().any(|root| unsettled.contains(root)) {
            continue;
        }
        let mut renewing: Vec<JobId> = Vec::new();
        let mut standing: Vec<Job> = Vec::new();
        match postings.board(id) {
            None => renewing.extend((0..JOBS_PER_BOARD).map(|slot| JobId {
                station: id.clone(),
                slot,
                posting: 0,
            })),
            Some(slots) => {
                for (slot, posting) in slots.iter().enumerate() {
                    let job_id = &posting.offer.id;
                    if !slot_renews(&jobs, posting, rule, now) {
                        standing.push(posting.offer.clone());
                        continue;
                    }
                    let why = if jobs.is_completed(job_id) {
                        "handed in"
                    } else if jobs.is_dropped(job_id) {
                        "dropped"
                    } else {
                        "expired"
                    };
                    let mut taken_out = 0;
                    for body_id in job_id.possible_bodies() {
                        if let Some(body) = worn.get(&body_id) {
                            commands.entity(*body).try_despawn();
                            retired.insert(body_id);
                            taken_out += 1;
                        }
                    }
                    let next = JobId {
                        posting: posting.posting + 1,
                        ..job_id.clone()
                    };
                    info!(
                        "nova_world_base: the station '{id}' renews slot {slot} at {now:.1} s of \
                         play ({why}): '{job_id}' -> '{next}', {taken_out} body(ies) taken out"
                    );
                    renewing.push(next);
                }
            }
        }
        if renewing.is_empty() {
            if posted.is_some() {
                continue;
            }
        } else {
            let mut excluded = postings.named_bodies(rule, now, &jobs);
            excluded.extend(retired.iter().cloned());
            let field = job_field(&around, &excluded, &wrecks, &hulls);
            let offers = station_job_offers(
                config.seed,
                &renewing,
                &clusters.0,
                &items,
                mix,
                &field,
                &standing,
            );
            info!(
                "nova_world_base: the station '{id}' posts {} offer(s) over {} wreck(s) and {} \
                 hostile(s): {}",
                offers.len(),
                field.wrecks.len(),
                field.hostiles.len(),
                offers
                    .iter()
                    .map(|job| format!("{} ({})", job.title(&items), job.id))
                    .collect::<Vec<_>>()
                    .join("; ")
            );
            postings.post(offers, now);
        }
        let board: Vec<Job> = postings
            .board(id)
            .expect("the station's board was posted above or before")
            .iter()
            .map(|slot| slot.offer.clone())
            .collect();
        // The board posts every job; the Jobs pane reads the marker and
        // hides, on this one board, the jobs the current hull cannot do
        // (`plan_job_hull_fit`), so the gate follows a refit with no timer.
        let starter = session
            .as_deref()
            .is_some_and(|session| session.is_starting_station(id));
        let mut station = commands.entity(entity);
        station.insert(JobBoard(board));
        if starter {
            station.insert(StarterBoardMarker);
        } else {
            station.remove::<StarterBoardMarker>();
        }
    }
}

/// Whether the slot on `posting` renews at `now` of play under `rule`: its
/// job was handed in or dropped, or the offer the player does not hold stood
/// past its window. A held job pins its slot.
fn slot_renews(jobs: &Jobs, posting: &SlotPosting, rule: JobOfferRule, now: f64) -> bool {
    let id = &posting.offer.id;
    if jobs.holds(id) {
        return false;
    }
    jobs.is_completed(id) || jobs.is_dropped(id) || posting.is_expired(rule, now)
}

/// The cell `centre` and the 26 cells around it.
fn cube_around(centre: SectorCoord) -> Vec<SectorCoord> {
    let mut cube = Vec::with_capacity(27);
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                cube.push(centre.offset(x, y, z));
            }
        }
    }
    cube
}

/// The wrecks with something aboard and the hostile intact hulls under the
/// roots `around`, each list in id order, less the bodies in `excluded`:
/// what a renewal this frame is taking out of the world, and every body a
/// held job or an unexpired offer names ([`JobPostings::named_bodies`]), so
/// a board never offers an escort's raiders as Neutralize targets, a genuine
/// caller as a wreck to recover, or a body another offer pays for.
#[expect(
    clippy::type_complexity,
    reason = "the two queries are the poster's, passed through"
)]
fn job_field(
    around: &[Entity],
    excluded: &BTreeSet<GeneratedBodyId>,
    wrecks: &Query<
        (&GeneratedBodyId, &Name, &ShipInventory, &ChildOf),
        (
            With<SpaceshipRootMarker>,
            With<DerelictShipMarker>,
            Without<StationMarker>,
        ),
    >,
    hulls: &Query<
        (&GeneratedBodyId, &Name, &Allegiance, &ChildOf),
        (
            With<SpaceshipRootMarker>,
            Without<DerelictShipMarker>,
            Without<NeutralizedMarker>,
            Without<StationMarker>,
        ),
    >,
) -> JobField {
    let mut field = JobField::default();
    for (id, name, hold, parent) in wrecks {
        if !around.contains(&parent.parent()) || hold.is_empty() || excluded.contains(id) {
            continue;
        }
        field.wrecks.push(JobWreck {
            id: id.clone(),
            name: name.to_string(),
            stacks: hold
                .stacks()
                .map(|(item, count)| (item.clone(), count))
                .collect(),
        });
    }
    for (id, name, allegiance, parent) in hulls {
        if !around.contains(&parent.parent())
            || *allegiance != Allegiance::Enemy
            || excluded.contains(id)
        {
            continue;
        }
        field.hostiles.push(JobHull {
            id: id.clone(),
            name: name.to_string(),
        });
    }
    field.wrecks.sort_by(|a, b| a.id.cmp(&b.id));
    field.hostiles.sort_by(|a, b| a.id.cmp(&b.id));
    field
}

/// Put an accepted job's bodies in the world, under the poster's cell root:
/// an escort's hauler beside the poster with no helm order, its leg laid in
/// the jobs to a mark a [`JOB_LEG`] out past it ([`Jobs::lay_leg`]), and its
/// raiders lying in wait off the leg ([`raider_wait_points`]); a
/// distress call's caller at the mark, a drifting hull when the call is
/// genuine and a raider when it is not. The leg's direction is seeded by
/// the job and chosen so every body stands [`JOB_SPAWN_CLEARANCE`] from
/// every generated body and the mark stays in the poster's cell, over
/// [`JOB_SPAWN_TRIES`] draws; when none fits, the clearest is taken and
/// logged. A procurement, a recovery and a neutralize put nothing there.
///
/// # Panics
///
/// When the poster is not a live station, when a body the job names or a
/// wreck severed off one already stands, when no civilization reaches the
/// poster, or when the layout or the stock fails: every one is an assembly
/// fault, not a world state. A posting is accepted once ([`Jobs::accept`]),
/// so a live `<body>/wreck/<section>` of a named body means a posting's ids
/// were minted twice; the sever would leave the new wreck id-less and the
/// save would refuse the duplicate.
pub(crate) fn spawn_job_bodies(
    accepted: On<JobAccepted>,
    mut commands: Commands,
    config: Res<WorldConfig<NovaLayeredWorld>>,
    items: Res<GameItems>,
    mut jobs: ResMut<Jobs>,
    stations: Query<(&GeneratedBodyId, &GlobalTransform, &ChildOf), With<StationMarker>>,
    bodies: Query<(&GeneratedBodyId, &GlobalTransform)>,
    live_ids: Query<&EntityId>,
) {
    let job = &accepted.event().job;
    let spawned = job.spawned_bodies();
    if spawned.is_empty() {
        return;
    }
    for live in &live_ids {
        let taken = spawned.iter().find(|id| {
            let id = id.to_string();
            live.0 == id || live.0.starts_with(&format!("{id}/wreck/"))
        });
        if let Some(id) = taken {
            panic!(
                "spawn_job_bodies: the body '{id}' of '{}' or a wreck off it already stands as \
                 '{}'",
                job.id, live.0
            );
        }
    }
    let (_, station_at, parent) = stations
        .iter()
        .find(|(id, _, _)| *id == job.posted_by())
        .unwrap_or_else(|| {
            panic!(
                "spawn_job_bodies: the poster '{}' of '{}' is not a live station",
                job.posted_by(),
                job.id
            )
        });
    let root = parent.parent();
    // Engine boundary: a transform counts engine units.
    let station_at = Meters3::from_engine(station_at.translation());
    let standing: Vec<Meters3> = bodies
        .iter()
        .map(|(_, at)| Meters3::from_engine(at.translation()))
        .collect();
    let key = |aspect: &[u8]| {
        Fnv32::new()
            .write(&config.seed.to_le_bytes())
            .write(aspect)
            .write(job.id.to_string().as_bytes())
            .finish()
    };
    let direction = leg_direction(
        key(b"job_leg"),
        station_at,
        config.sector_edge,
        job,
        &standing,
    );
    let parts = config.generator.parts();
    let civilization = CivilizationField::new(config.seed)
        .in_reach(station_at)
        .unwrap_or_else(|fault| panic!("spawn_job_bodies: '{}': {fault}", job.id))
        .into_iter()
        .max_by(|a, b| a.selection_weight.total_cmp(&b.selection_weight))
        .unwrap_or_else(|| panic!("spawn_job_bodies: '{}': {NO_CIVILIZATION}", job.id))
        .civilization;
    let facing = Quat::from_rotation_arc(Vec3::NEG_Z, direction);
    let hull = |id: &GeneratedBodyId, aspect: &[u8], role: ShipRoleType, condition| {
        let request = ShipLayoutRequest {
            seed: key(aspect),
            civilization: civilization.id,
            role,
            advancement: civilization.advancement,
        };
        let layout = generate_ship(parts, request)
            .unwrap_or_else(|failure| panic!("spawn_job_bodies: '{id}': {failure}"));
        let stock = ship_stock(
            parts,
            &items,
            &layout.design,
            role,
            condition,
            civilization.advancement,
            key(aspect).wrapping_add(1),
        )
        .unwrap_or_else(|| {
            panic!("spawn_job_bodies: '{id}': the hold cannot fit one unit of its lightest item")
        });
        (layout.design, stock)
    };
    let raider = |index: u32, at: Meters3| {
        let id = &job.raider_ids()[index as usize];
        let (design, stock) = hull(
            id,
            format!("job_raider_{index}").as_bytes(),
            ShipRoleType::Armored,
            SectorShipConditionType::Intact,
        );
        (
            base_scenario_object(&BaseScenarioObjectConfig {
                id: id.to_string(),
                name: format!("{} raider", civilization.id.name()),
                position: at,
                rotation: Quat::from_rotation_arc(Vec3::NEG_Z, -direction),
            }),
            id.clone(),
            spaceship_scenario_object(SpaceshipConfig {
                design: ShipDesignSource::Inline(design),
                controller: SpaceshipController::AI(AIControllerConfig {
                    leash: Some(RAIDER_LEASH),
                    engage_delay: Some(RAIDER_ENGAGE_DELAY),
                    engage_range: Some(RAIDER_ENGAGE_RANGE),
                    ..AIControllerConfig::default()
                }),
                initial_velocity: MetersPerSecond3::ZERO,
                allegiance: Some(Allegiance::Enemy),
                capabilities: ShipCapabilities::default(),
                inventory: stock,
                lootable: false,
                credits: 0,
            }),
            ChildOf(root),
        )
    };
    let along = |share: f32| Meters3(station_at.get() + direction * JOB_LEG.get() * share);
    let mark = along(1.0);
    match &job.kind {
        JobType::Escort { hauler, cargo_cr } => {
            let (design, stock) = hull(
                hauler,
                b"job_hauler",
                ShipRoleType::Civilian,
                SectorShipConditionType::Intact,
            );
            let start = Meters3(station_at.get() + direction * HAULER_STANDOFF.get());
            commands.spawn((
                base_scenario_object(&BaseScenarioObjectConfig {
                    id: hauler.to_string(),
                    name: format!("{} hauler", civilization.id.name()),
                    position: start,
                    rotation: facing,
                }),
                hauler.clone(),
                spaceship_scenario_object(SpaceshipConfig {
                    design: ShipDesignSource::Inline(design),
                    controller: SpaceshipController::AI(AIControllerConfig {
                        non_combatant: true,
                        ..AIControllerConfig::default()
                    }),
                    initial_velocity: MetersPerSecond3::ZERO,
                    allegiance: Some(Allegiance::Player),
                    capabilities: ShipCapabilities::default(),
                    inventory: stock,
                    lootable: false,
                    credits: 0,
                }),
                ChildOf(root),
            ));
            jobs.lay_leg(&job.id, mark);
            let raiders = job.raider_ids().len();
            for (index, at) in raider_wait_points(station_at, direction, raiders)
                .into_iter()
                .enumerate()
            {
                commands.spawn(raider(index as u32, at));
            }
            info!(
                "nova_world_base: '{}' puts the hauler '{hauler}' ({cargo_cr} cr) on a {} m leg \
                 with {raiders} raider(s) in wait",
                job.id,
                JOB_LEG.get()
            );
        }
        JobType::Distress { caller, genuine } => {
            if *genuine {
                let (design, stock) = hull(
                    caller,
                    b"job_caller",
                    ShipRoleType::Civilian,
                    SectorShipConditionType::Derelict,
                );
                commands.spawn((
                    base_scenario_object(&BaseScenarioObjectConfig {
                        id: caller.to_string(),
                        name: format!("{} civilian, adrift", civilization.id.name()),
                        position: mark,
                        rotation: facing,
                    }),
                    caller.clone(),
                    spaceship_scenario_object(SpaceshipConfig {
                        design: ShipDesignSource::Inline(design),
                        controller: SpaceshipController::None,
                        initial_velocity: MetersPerSecond3::ZERO,
                        allegiance: Some(Allegiance::Neutral),
                        capabilities: ShipCapabilities::default(),
                        inventory: stock,
                        lootable: true,
                        credits: 0,
                    }),
                    ChildOf(root),
                    DerelictShipMarker,
                ));
            } else {
                let id = caller;
                let (design, stock) = hull(
                    id,
                    b"job_caller",
                    ShipRoleType::Armored,
                    SectorShipConditionType::Intact,
                );
                commands.spawn((
                    base_scenario_object(&BaseScenarioObjectConfig {
                        id: id.to_string(),
                        name: format!("{} raider", civilization.id.name()),
                        position: mark,
                        rotation: Quat::from_rotation_arc(Vec3::NEG_Z, -direction),
                    }),
                    id.clone(),
                    spaceship_scenario_object(SpaceshipConfig {
                        design: ShipDesignSource::Inline(design),
                        controller: SpaceshipController::AI(AIControllerConfig {
                            leash: Some(RAIDER_LEASH),
                            ..AIControllerConfig::default()
                        }),
                        initial_velocity: MetersPerSecond3::ZERO,
                        allegiance: Some(Allegiance::Enemy),
                        capabilities: ShipCapabilities::default(),
                        inventory: stock,
                        lootable: false,
                        credits: 0,
                    }),
                    ChildOf(root),
                ));
            }
            info!(
                "nova_world_base: '{}' puts the caller '{caller}' {} m out (genuine: {genuine})",
                job.id,
                JOB_LEG.get()
            );
        }
        JobType::Procure { .. } | JobType::Recover { .. } | JobType::Neutralize { .. } => {
            unreachable!("a job with spawned bodies is an escort or a distress call")
        }
    }
}

/// The unit direction of `job`'s leg from `from`: the first of
/// [`JOB_SPAWN_TRIES`] draws off `seed` whose every body stands
/// [`JOB_SPAWN_CLEARANCE`] from every position in `standing` with the mark in
/// the cell `from` is in, else the draw whose nearest body is farthest.
fn leg_direction(seed: u32, from: Meters3, edge: Meters, job: &Job, standing: &[Meters3]) -> Vec3 {
    let cell = SectorCoord::containing(from, edge);
    let mut stream = SeedStream::new(seed);
    let mut best: Option<(f32, Vec3)> = None;
    for _ in 0..JOB_SPAWN_TRIES {
        let direction = unit_sphere_point(stream.next_u32());
        let along = |share: f32| from.get() + direction * JOB_LEG.get() * share;
        let mut points = vec![along(1.0)];
        if matches!(job.kind, JobType::Escort { .. }) {
            points.push(from.get() + direction * HAULER_STANDOFF.get());
            let raiders = raider_wait_points(from, direction, job.raider_ids().len());
            points.extend(raiders.iter().map(|at| at.get()));
        }
        let nearest = points
            .iter()
            .flat_map(|point| standing.iter().map(move |body| point.distance(body.get())))
            .fold(f32::INFINITY, f32::min);
        let in_cell = SectorCoord::containing(Meters3(along(1.0)), edge) == cell;
        if in_cell && nearest >= JOB_SPAWN_CLEARANCE.get() {
            return direction;
        }
        let score = if in_cell { nearest } else { nearest / 2.0 };
        if best.is_none_or(|(best, _)| score > best) {
            best = Some((score, direction));
        }
    }
    let (score, direction) = best.expect("at least one draw");
    warn!(
        "nova_world_base: no leg of '{}' clears {} m; taking the one {score:.0} m clear",
        job.id,
        JOB_SPAWN_CLEARANCE.get()
    );
    direction
}

/// Where each of `raiders` raiders waits on the leg from `from` along
/// `direction`: [`RAIDER_LATERAL`] off the leg's line, [`RAIDER_SPACING`]
/// apart along it, centred at [`RAIDER_AMBUSH_SHARE`] of the leg.
fn raider_wait_points(from: Meters3, direction: Vec3, raiders: usize) -> Vec<Meters3> {
    let across = direction.any_orthonormal_vector();
    (0..raiders)
        .map(|index| {
            let offset = (index as f32 - (raiders as f32 - 1.0) / 2.0) * RAIDER_SPACING.get();
            let along = JOB_LEG.get() * RAIDER_AMBUSH_SHARE + offset;
            Meters3(from.get() + direction * along + across * RAIDER_LATERAL.get())
        })
        .collect()
}

/// The key of the helm order that flies `hauler`'s leg.
fn leg_order_key(hauler: &GeneratedBodyId) -> String {
    format!("{hauler}_leg")
}

/// Fly every held escort's leg: start a leg not started once the player's
/// hull stands and is not docked with the leg's poster, and give a standing
/// hauler whose leg started and that carries no helm order a Move to its mark.
/// A hauler carries none when the leg just started, and after a thaw or a
/// Load, which keep no helm order; a completed or failed order stays on the
/// hauler, so a leg is ordered once per spawn. A Load before the undock puts
/// the player undocked, so the leg starts on the Load's first frame.
#[expect(
    clippy::type_complexity,
    reason = "the hauler query reads its id and its reports and filters on two markers"
)]
pub(crate) fn fly_escort_legs(
    mut commands: Commands,
    mut jobs: ResMut<Jobs>,
    player: Query<Option<&DockedShip>, With<PlayerSpaceshipMarker>>,
    connections: Query<&DockingConnection>,
    ids: Query<&GeneratedBodyId>,
    haulers: Query<
        (Entity, &GeneratedBodyId, Has<ShipOrderReports>),
        (With<SpaceshipRootMarker>, Without<ShipHelmOrder>),
    >,
) {
    let docked_at = player.single().ok().map(|docked| {
        docked
            .and_then(|docked| connections.get(docked.connection).ok())
            .map(|connection| {
                if player.contains(connection.first_ship) {
                    connection.second_ship
                } else {
                    connection.first_ship
                }
            })
            .and_then(|partner| ids.get(partner).ok())
    });
    // Read through the shared borrow: a mutable one would mark the jobs
    // changed every frame and `mark_held_job_targets` gates on that.
    let due: Vec<JobId> = jobs
        .legs()
        .filter(|(id, leg)| {
            !leg.started
                && docked_at
                    .as_ref()
                    .is_some_and(|partner| partner.is_none_or(|partner| *partner != id.station))
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in due {
        jobs.start_leg(&id);
        info!("nova_world_base: '{id}' starts its leg: the player stands undocked from its poster");
    }
    for (entity, body, reported) in &haulers {
        let Some((job, leg)) = jobs.legs().find_map(|(id, leg)| {
            let job = jobs.job(id).expect("a leg's escort is held");
            matches!(&job.kind, JobType::Escort { hauler, .. } if hauler == body)
                .then_some((job, leg))
        }) else {
            continue;
        };
        if !leg.started {
            continue;
        }
        if !reported {
            commands.entity(entity).insert(ShipOrderReports::default());
        }
        commands.entity(entity).insert((
            ShipHelmOrder::new(
                leg_order_key(body),
                ShipOrderDirective::Move {
                    // Engine boundary: a helm mark counts engine units.
                    position: leg.mark.to_engine(),
                    arrival_standoff: None,
                },
            ),
            ShipOrderHelmAuthority,
        ));
        info!(
            "nova_world_base: '{}' orders the hauler '{body}' to its mark",
            job.id
        );
    }
}

/// The `id` field of a fired event's payload, when it carries one.
fn event_subject(event: &GameEvent) -> Option<&str> {
    event.info().data.as_ref()?.get("id")?.as_str()
}

/// Whether `event` is `onneutralized` or `ondestroyed` naming `body`.
fn is_taken_out(event: &GameEvent, body: &GeneratedBodyId) -> bool {
    (event.name() == OnNeutralizedEvent::name() || event.name() == OnDestroyedEvent::name())
        && event_subject(event) == Some(body.to_string().as_str())
}

/// Drop every held job the `onneutralized` or `ondestroyed` of one body
/// retires, and take the job's spawned bodies out of the world so the board
/// offers it again and a second accept can put them back: every job of the
/// poster, done or not, since no hand-in is left; an escort whose hauler, a
/// recovery whose wreck or a genuine distress call whose caller is taken out
/// before the job is done. An undone escort is also dropped on the
/// `onshiporderfailed` of its hauler's leg order ([`leg_order_key`]): a
/// hauler that lost its engines or its flight computer never reaches its
/// mark. A done job keeps its hand-in: a hauler killed at
/// its mark or a wreck shot after the lift still pays. A trap's caller taken
/// out is [`distress_on_answer`]'s done and a neutralize's hull
/// [`neutralize_on_kill`]'s.
pub(crate) fn drop_on_loss(
    event: On<GameEvent>,
    mut commands: Commands,
    mut jobs: ResMut<Jobs>,
    bodies: Query<(Entity, &GeneratedBodyId)>,
) {
    let failed = event.name() == OnShipOrderFailedEvent::name();
    if !failed
        && event.name() != OnNeutralizedEvent::name()
        && event.name() != OnDestroyedEvent::name()
    {
        return;
    }
    let Some(subject) = event_subject(&event) else {
        return;
    };
    let order = event
        .info()
        .data
        .as_ref()
        .and_then(|data| data.get("order"))
        .and_then(|order| order.as_str());
    let lost: Vec<(Job, &'static str)> = jobs
        .held()
        .iter()
        .filter_map(|job| {
            if failed {
                return match &job.kind {
                    JobType::Escort { hauler, .. }
                        if !jobs.is_done(&job.id)
                            && hauler.to_string() == subject
                            && order == Some(leg_order_key(hauler).as_str()) =>
                    {
                        Some((job.clone(), "hauler's leg order"))
                    }
                    _ => None,
                };
            }
            if job.posted_by().to_string() == subject {
                return Some((job.clone(), "poster"));
            }
            if jobs.is_done(&job.id) {
                return None;
            }
            let target = match &job.kind {
                JobType::Escort { hauler, .. } => Some((hauler, "hauler")),
                JobType::Recover { wreck, .. } => Some((wreck, "wreck")),
                JobType::Distress {
                    caller,
                    genuine: true,
                } => Some((caller, "caller")),
                JobType::Distress { genuine: false, .. }
                | JobType::Procure { .. }
                | JobType::Neutralize { .. } => None,
            };
            target
                .filter(|(body, _)| body.to_string() == subject)
                .map(|(_, what)| (job.clone(), what))
        })
        .collect();
    for (job, what) in lost {
        jobs.drop(&job.id);
        let gone = job.spawned_bodies();
        for (entity, id) in &bodies {
            if gone.contains(id) {
                commands.entity(entity).try_despawn();
            }
        }
        warn!(
            "nova_world_base: '{}' is lost: its {what} '{subject}' was {}",
            job.id,
            event.name()
        );
    }
}

/// Mark a held recovery done on the `onundocked` from its wreck while the
/// player's hold carries at least the asked units and the wreck carries none
/// of the item. An undock that leaves units aboard marks nothing: the player
/// docks again and takes the rest.
pub(crate) fn recover_on_undock(
    event: On<GameEvent>,
    mut commands: Commands,
    mut jobs: ResMut<Jobs>,
    player: Query<&ShipInventory, With<PlayerSpaceshipMarker>>,
    holds: Query<(&GeneratedBodyId, &ShipInventory), Without<PlayerSpaceshipMarker>>,
) {
    if event.name() != OnUndockedEvent::name() {
        return;
    }
    let Some(subject) = event_subject(&event) else {
        return;
    };
    let Ok(own) = player.single() else {
        return;
    };
    let due: Vec<Job> = jobs
        .held()
        .iter()
        .filter(|job| {
            !jobs.is_done(&job.id)
                && matches!(&job.kind, JobType::Recover { wreck, .. } if wreck.to_string() == subject)
        })
        .cloned()
        .collect();
    for job in due {
        let JobType::Recover {
            wreck, item, count, ..
        } = &job.kind
        else {
            unreachable!()
        };
        let aboard = holds
            .iter()
            .find(|(id, _)| *id == wreck)
            .map_or(0, |(_, hold)| hold.count(item));
        let carried = own.count(item);
        if carried >= *count && aboard == 0 {
            jobs.mark_done(&job.id);
            commands.trigger(JobDone { id: job.id.clone() });
            info!(
                "nova_world_base: '{}' is done: the hold carries {carried} {item}, '{wreck}' none",
                job.id
            );
        } else {
            info!(
                "nova_world_base: '{}' is not done: the hold carries {carried} of {count} {item}, \
                 '{wreck}' still {aboard}",
                job.id
            );
        }
    }
}

/// Mark a held escort done on the `onshipordercomplete` of its hauler's Move
/// order. The hauler's loss is [`drop_on_loss`]'s.
pub(crate) fn escort_on_arrival(
    event: On<GameEvent>,
    mut commands: Commands,
    mut jobs: ResMut<Jobs>,
) {
    let arrived = event.name() == OnShipOrderCompleteEvent::name()
        && event
            .info()
            .data
            .as_ref()
            .and_then(|data| data.get("kind"))
            .and_then(|kind| kind.as_str())
            == Some("Move");
    if !arrived {
        return;
    }
    let Some(subject) = event_subject(&event) else {
        return;
    };
    let due: Vec<Job> = jobs
        .held()
        .iter()
        .filter(|job| {
            !jobs.is_done(&job.id)
                && matches!(&job.kind, JobType::Escort { hauler, .. } if hauler.to_string() == subject)
        })
        .cloned()
        .collect();
    for job in due {
        let JobType::Escort { hauler, .. } = &job.kind else {
            unreachable!()
        };
        jobs.mark_done(&job.id);
        commands.trigger(JobDone { id: job.id.clone() });
        info!(
            "nova_world_base: '{}' is done: the hauler '{hauler}' reached its mark",
            job.id
        );
    }
}

/// Mark a held distress call done on the `ondocked` with its caller when
/// the call is genuine, and on the caller's `onneutralized` or `ondestroyed`
/// when it is a raider's trap.
pub(crate) fn distress_on_answer(
    event: On<GameEvent>,
    mut commands: Commands,
    mut jobs: ResMut<Jobs>,
) {
    let Some(subject) = event_subject(&event) else {
        return;
    };
    let held: Vec<Job> = jobs
        .held()
        .iter()
        .filter(|job| {
            !jobs.is_done(&job.id)
                && matches!(&job.kind, JobType::Distress { caller, .. } if caller.to_string() == subject)
        })
        .cloned()
        .collect();
    for job in held {
        let JobType::Distress { caller, genuine } = &job.kind else {
            unreachable!()
        };
        let answered = if *genuine {
            event.name() == OnDockedEvent::name()
        } else {
            is_taken_out(&event, caller)
        };
        if answered {
            jobs.mark_done(&job.id);
            commands.trigger(JobDone { id: job.id.clone() });
            info!(
                "nova_world_base: '{}' is done: the caller '{caller}' was answered ({})",
                job.id,
                event.name()
            );
        }
    }
}

/// Mark a held neutralize done on the `onneutralized` or `ondestroyed` of
/// its hull.
pub(crate) fn neutralize_on_kill(
    event: On<GameEvent>,
    mut commands: Commands,
    mut jobs: ResMut<Jobs>,
) {
    let Some(subject) = event_subject(&event) else {
        return;
    };
    let held: Vec<Job> = jobs
        .held()
        .iter()
        .filter(|job| {
            !jobs.is_done(&job.id)
                && matches!(&job.kind, JobType::Neutralize { hull, .. } if hull.to_string() == subject)
        })
        .cloned()
        .collect();
    for job in held {
        let JobType::Neutralize { hull, .. } = &job.kind else {
            unreachable!()
        };
        if is_taken_out(&event, hull) {
            jobs.mark_done(&job.id);
            commands.trigger(JobDone { id: job.id.clone() });
            info!(
                "nova_world_base: '{}' is done: '{hull}' was {}",
                job.id,
                event.name()
            );
        }
    }
}

/// Keep the HUD's objective chip on every standing body a held job sends the
/// player to: an undone recovery's wreck, escort's hauler, distress call's
/// caller or neutralize's hull as `JOB-n <verb>`, and a done job's poster as
/// `JOB-n Hand in`. The codes of every job on one body share its one tag
/// (`JOB-1, JOB-2 Hand in`). A procurement marks nothing: its work is the
/// hand-in itself. A frozen or unspawned body has no entity and no chip; the
/// chart's JOB-n mark stands for it, and the body is tagged the frame it
/// thaws or spawns. A tag this system put on a body no held job names comes
/// off.
///
/// # Panics
///
/// When a body a held job names wears a tag this system did not put there:
/// a scenario and the jobs would both own its one chip.
pub(crate) fn mark_held_job_targets(
    mut commands: Commands,
    jobs: Res<Jobs>,
    mut tagged: Local<EntityHashSet>,
    added: Query<(), Added<GeneratedBodyId>>,
    bodies: Query<(Entity, &GeneratedBodyId, Option<&ObjectiveMarkerTarget>)>,
) {
    if !jobs.is_changed() && added.is_empty() {
        return;
    }
    let mut wanted: BTreeMap<&GeneratedBodyId, Vec<(String, &'static str)>> = BTreeMap::new();
    for job in jobs.held() {
        let Some(body) = jobs.marked_body(job) else {
            continue;
        };
        let verb = if jobs.is_done(&job.id) {
            "Hand in"
        } else {
            job.kind.label()
        };
        let code = jobs.mark_code(&job.id).expect("a held job has a mark code");
        wanted.entry(body).or_default().push((code, verb));
    }
    for (entity, id, tag) in &bodies {
        let ours = tagged.contains(&entity);
        match wanted.get(id) {
            Some(entries) => {
                assert!(
                    ours || tag.is_none(),
                    "mark_held_job_targets: '{id}' wears the objective tag '{}' that no held job put there",
                    tag.map_or("", |tag| tag.label.as_str())
                );
                // A run of one verb shares it, in accept order; runs join by `; `.
                let label = entries
                    .chunk_by(|a, b| a.1 == b.1)
                    .map(|run| {
                        let codes: Vec<&str> = run.iter().map(|(code, _)| code.as_str()).collect();
                        format!("{} {}", codes.join(", "), run[0].1)
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                if tag.is_none_or(|tag| tag.label != label) {
                    // A body frozen on this frame is gone by the time the
                    // command lands; the thaw tags it again.
                    commands
                        .entity(entity)
                        .try_insert(ObjectiveMarkerTarget { label });
                    tagged.insert(entity);
                }
            }
            None if ours => {
                commands
                    .entity(entity)
                    .try_remove::<ObjectiveMarkerTarget>();
                tagged.remove(&entity);
            }
            None => {}
        }
    }
    tagged.retain(|entity| bodies.contains(*entity));
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use nova_events::prelude::{
        OnDestroyedEventInfo, OnShipOrderCompleteEventInfo, OnShipOrderFailedEventInfo,
        ShipOrderKind,
    };
    use nova_gameplay::{
        prelude::{ItemDesignId, ESCORT_RAIDERS_MAX},
        test_support::test_items,
    };
    use nova_ship::prelude::DockedHelmType;

    use super::*;

    fn poster() -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::new(0, 0, 0), "cluster_0_0_0_station", 0)
    }

    fn job_id(slot: u32) -> JobId {
        JobId {
            station: poster(),
            slot,
            posting: 0,
        }
    }

    /// The id of body `index` of `part` the job `id` spawns, under the stem
    /// the gameplay crate mints; `job` checks the match with `Job::validate`.
    fn spawned(id: &JobId, part: &str, index: u32) -> GeneratedBodyId {
        GeneratedBodyId::new(
            id.station.origin,
            format!("{}_job_{}_{}_{part}", id.station.stem, id.slot, id.posting),
            index,
        )
    }

    fn wreck_id() -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::new(1, 0, 0), "cluster_1_0_0_hull", 2)
    }

    fn hull_id() -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::new(1, 0, 0), "cluster_1_0_0_hull", 0)
    }

    fn item() -> ItemDesignId {
        test_items()
            .iter()
            .next()
            .expect("the fixture catalog has an item")
            .id
            .clone()
    }

    fn job(slot: u32, kind: JobType) -> Job {
        let job = Job {
            id: job_id(slot),
            kind,
            target: poster(),
            reward_cr: 100,
        };
        job.validate().expect("the test names the job's own bodies");
        job
    }

    /// An escort whose cargo puts three raiders in wait.
    fn escort(slot: u32) -> Job {
        let id = job_id(slot);
        job(
            slot,
            JobType::Escort {
                hauler: spawned(&id, "hauler", 0),
                cargo_cr: 1_500,
            },
        )
    }

    fn recover(slot: u32) -> Job {
        job(
            slot,
            JobType::Recover {
                wreck: wreck_id(),
                wreck_name: "Vell derelict".to_string(),
                item: item(),
                count: 2,
            },
        )
    }

    fn distress(slot: u32, genuine: bool) -> Job {
        let id = job_id(slot);
        job(
            slot,
            JobType::Distress {
                caller: spawned(&id, "caller", 0),
                genuine,
            },
        )
    }

    fn neutralize(slot: u32) -> Job {
        job(
            slot,
            JobType::Neutralize {
                hull: hull_id(),
                hull_name: "Kessel armored".to_string(),
            },
        )
    }

    fn procure(slot: u32) -> Job {
        job(
            slot,
            JobType::Procure {
                item: item(),
                count: 2,
            },
        )
    }

    fn taken_out(body: &GeneratedBodyId) -> GameEvent {
        GameEvent::new(
            OnDestroyedEvent::name(),
            OnDestroyedEventInfo {
                id: body.to_string(),
                type_name: "spaceship".to_string(),
            }
            .into(),
        )
    }

    fn arrived(hauler: &GeneratedBodyId) -> GameEvent {
        GameEvent::new(
            OnShipOrderCompleteEvent::name(),
            OnShipOrderCompleteEventInfo {
                order: leg_order_key(hauler),
                id: hauler.to_string(),
                type_name: "spaceship".to_string(),
                kind: ShipOrderKind::Move,
            }
            .into(),
        )
    }

    fn failed(hauler: &GeneratedBodyId, order: String) -> GameEvent {
        GameEvent::new(
            OnShipOrderFailedEvent::name(),
            OnShipOrderFailedEventInfo {
                order,
                id: hauler.to_string(),
                type_name: "spaceship".to_string(),
                kind: ShipOrderKind::Move,
            }
            .into(),
        )
    }

    /// An app holding `held`, with every job observer over a `GameEvent`.
    fn held_app(held: impl IntoIterator<Item = Job>) -> App {
        let mut app = App::new();
        let mut jobs = Jobs::default();
        for job in held {
            jobs.accept(job);
        }
        app.insert_resource(jobs);
        app.add_observer(recover_on_undock);
        app.add_observer(escort_on_arrival);
        app.add_observer(distress_on_answer);
        app.add_observer(neutralize_on_kill);
        app.add_observer(drop_on_loss);
        app
    }

    fn fire(app: &mut App, event: GameEvent) {
        app.world_mut().trigger(event);
        app.world_mut().flush();
    }

    /// Put every body of `ids` in the world, each under its id only.
    fn stand(app: &mut App, ids: &[GeneratedBodyId]) -> Vec<Entity> {
        ids.iter()
            .map(|id| app.world_mut().spawn(id.clone()).id())
            .collect()
    }

    fn standing(app: &App, entities: &[Entity]) -> usize {
        entities
            .iter()
            .filter(|entity| app.world().entities().contains(**entity))
            .count()
    }

    fn jobs(app: &App) -> &Jobs {
        app.world().resource::<Jobs>()
    }

    /// The hauler's arrival raises one `JobDone` with the escort's id; the
    /// same arrival again, the job done, raises none.
    #[test]
    fn an_escort_arrival_raises_one_job_done() {
        #[derive(Resource, Default)]
        struct DoneRaised(Vec<JobId>);

        let escort = escort(0);
        let JobType::Escort { hauler, .. } = &escort.kind else {
            unreachable!()
        };
        let mut app = held_app([escort.clone()]);
        app.init_resource::<DoneRaised>();
        app.add_observer(|done: On<JobDone>, mut raised: ResMut<DoneRaised>| {
            raised.0.push(done.id.clone());
        });

        fire(&mut app, arrived(hauler));
        fire(&mut app, arrived(hauler));
        assert!(jobs(&app).is_done(&escort.id), "the mark marks it done");
        assert_eq!(
            app.world().resource::<DoneRaised>().0,
            [escort.id.clone()],
            "one arrival marks once and raises once"
        );
    }

    #[test]
    fn done_escort_stays_done_when_the_hauler_dies_after_its_mark() {
        let escort = escort(0);
        let JobType::Escort { hauler, .. } = &escort.kind else {
            unreachable!()
        };
        let mut app = held_app([escort.clone()]);
        stand(&mut app, &escort.spawned_bodies());

        fire(&mut app, arrived(hauler));
        assert!(jobs(&app).is_done(&escort.id), "the mark marks it done");

        fire(&mut app, taken_out(hauler));
        assert!(
            jobs(&app).holds(&escort.id),
            "a hauler killed at its mark does not lose the job"
        );
        assert!(jobs(&app).is_done(&escort.id), "the hand-in still pays");
    }

    #[test]
    fn an_escort_is_dropped_and_its_bodies_taken_out_when_the_hauler_dies_before_its_mark() {
        let escort = escort(0);
        let JobType::Escort { hauler, .. } = &escort.kind else {
            unreachable!()
        };
        let mut app = held_app([escort.clone()]);
        let bodies = stand(&mut app, &escort.spawned_bodies());
        assert_eq!(bodies.len(), 4, "the hauler and three raiders");

        fire(&mut app, taken_out(hauler));
        assert!(!jobs(&app).holds(&escort.id), "the slot is free");
        assert!(jobs(&app).done().is_empty());
        assert_eq!(
            standing(&app, &bodies),
            0,
            "the remains and the raiders are taken out so a second accept can put them back"
        );
    }

    #[test]
    fn an_escort_is_dropped_when_its_hauler_s_leg_order_fails() {
        let escort = escort(0);
        let JobType::Escort { hauler, .. } = &escort.kind else {
            unreachable!()
        };
        let mut app = held_app([escort.clone()]);
        app.world_mut()
            .resource_mut::<Jobs>()
            .lay_leg(&escort.id, Meters3::default());
        let bodies = stand(&mut app, &escort.spawned_bodies());

        fire(&mut app, failed(hauler, "another_order".to_string()));
        assert!(
            jobs(&app).holds(&escort.id),
            "only the leg's order failing loses the job"
        );

        fire(&mut app, failed(hauler, leg_order_key(hauler)));
        assert!(!jobs(&app).holds(&escort.id), "the slot is free");
        assert_eq!(jobs(&app).legs().count(), 0, "the leg is over");
        jobs(&app).validate().unwrap();
        assert_eq!(
            standing(&app, &bodies),
            0,
            "the stuck hauler and the raiders are taken out so a second accept can put them back"
        );
    }

    /// A lost hauler ends its posting: the slot renews and posts under the
    /// next number with fresh body ids, so a second hauler and its wrecks
    /// never carry the ids of the first. A held slot and a standing,
    /// unexpired slot stay.
    #[test]
    fn a_dropped_escort_renews_its_slot_while_held_and_standing_slots_stay() {
        let rule = JobOfferRule {
            offer_seconds: 1_200.0,
        };
        let slot = |job: &Job| SlotPosting {
            posting: 0,
            posted_at: 0.0,
            offer: job.clone(),
        };
        let escort = escort(0);
        let JobType::Escort { hauler, .. } = &escort.kind else {
            unreachable!()
        };
        let held = procure(1);
        let standing = neutralize(2);
        let mut app = held_app([escort.clone(), held.clone()]);
        stand(&mut app, &escort.spawned_bodies());
        assert!(!slot_renews(jobs(&app), &slot(&escort), rule, 10.0));

        fire(&mut app, taken_out(hauler));
        assert!(
            jobs(&app).is_dropped(&escort.id),
            "the lost job is recorded"
        );
        assert!(
            slot_renews(jobs(&app), &slot(&escort), rule, 10.0),
            "a dropped job's slot renews before its window passes"
        );
        assert!(
            !slot_renews(jobs(&app), &slot(&held), rule, 10.0),
            "a held job pins its slot"
        );
        assert!(
            !slot_renews(jobs(&app), &slot(&standing), rule, 10.0),
            "an unheld offer stands until its window passes"
        );
        assert!(slot_renews(jobs(&app), &slot(&standing), rule, 1_200.0));
        jobs(&app).validate().unwrap();
    }

    #[test]
    fn an_escort_leg_waits_while_the_player_is_docked_at_its_poster_and_flies_after_the_undock_a_thaw_and_a_load(
    ) {
        let escort = escort(0);
        let JobType::Escort { hauler, .. } = escort.kind.clone() else {
            unreachable!()
        };
        let mark = Meters3(Vec3::new(0.0, 0.0, -10_000.0));
        let mut saved = Jobs::default();
        saved.accept(escort.clone());
        saved.lay_leg(&escort.id, mark);
        let leg_app = |jobs: Jobs| {
            let mut app = App::new();
            app.insert_resource(jobs);
            app.add_systems(Update, fly_escort_legs);
            app.world_mut().spawn(poster());
            let player = app.world_mut().spawn(PlayerSpaceshipMarker).id();
            (app, player)
        };
        let stand_hauler = |app: &mut App| {
            app.world_mut()
                .spawn((hauler.clone(), SpaceshipRootMarker))
                .id()
        };
        let started = |app: &App| {
            app.world()
                .resource::<Jobs>()
                .legs()
                .map(|(_, leg)| leg.started)
                .collect::<Vec<_>>()
        };
        let flies_to_the_mark = |app: &App, entity: Entity| {
            let order = app.world().get::<ShipHelmOrder>(entity)?;
            let flies = order.key == leg_order_key(&hauler)
                && order.directive
                    == ShipOrderDirective::Move {
                        position: mark.to_engine(),
                        arrival_standoff: None,
                    }
                && app.world().get::<ShipOrderHelmAuthority>(entity).is_some()
                && app.world().get::<ShipOrderReports>(entity).is_some();
            Some(flies)
        };

        let (mut app, player) = leg_app(saved.clone());
        let station = app
            .world_mut()
            .query_filtered::<Entity, With<GeneratedBodyId>>()
            .single(app.world())
            .unwrap();
        let connection = app
            .world_mut()
            .spawn(DockingConnection {
                first_ship: player,
                first_section: player,
                second_ship: station,
                second_section: station,
                helm: DockedHelmType::Neutral,
                measurement_fault: false,
            })
            .id();
        app.world_mut().entity_mut(player).insert(DockedShip {
            connection,
            helm: Quat::IDENTITY,
            drives: false,
        });
        let first = stand_hauler(&mut app);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(started(&app), vec![false], "docked at the poster");
        assert_eq!(
            flies_to_the_mark(&app, first),
            None,
            "the hauler waits with no order while the player is docked"
        );

        app.world_mut().despawn(connection);
        app.world_mut().entity_mut(player).remove::<DockedShip>();
        app.update();
        assert_eq!(started(&app), vec![true], "the undock starts the leg");
        assert_eq!(flies_to_the_mark(&app, first), Some(true));

        app.world_mut().despawn(first);
        app.update();
        let thawed = stand_hauler(&mut app);
        app.update();
        assert_eq!(
            flies_to_the_mark(&app, thawed),
            Some(true),
            "a thawed hauler flies its started leg again"
        );

        let record = ron::to_string(&saved).unwrap();
        let loaded: Jobs = ron::from_str(&record).unwrap();
        assert_eq!(loaded, saved, "the save keeps the leg");
        loaded.validate().unwrap();
        let (mut app, _) = leg_app(loaded);
        let reloaded = stand_hauler(&mut app);
        app.update();
        assert_eq!(
            started(&app),
            vec![true],
            "a Load puts the player undocked, so a leg saved docked starts"
        );
        assert_eq!(flies_to_the_mark(&app, reloaded), Some(true));
    }

    #[test]
    fn every_raider_waits_beyond_its_engage_range_of_the_hauler_s_start_and_within_it_of_the_leg() {
        // One metre of slack for f32 rounding at ten kilometres.
        const SLACK: f32 = 1.0;
        let from = Meters3(Vec3::new(400.0, -250.0, 1_200.0));
        let direction = Vec3::new(1.0, 2.0, -0.5).normalize();
        let start = from.get() + direction * HAULER_STANDOFF.get();
        let leg = JOB_LEG.get() - HAULER_STANDOFF.get();
        for raiders in 1..=ESCORT_RAIDERS_MAX as usize {
            let points = raider_wait_points(from, direction, raiders);
            assert_eq!(points.len(), raiders);
            for (index, at) in points.iter().enumerate() {
                let at = at.get();
                assert!(
                    at.distance(start) > RAIDER_ENGAGE_RANGE.get(),
                    "raider {index} of {raiders} waits {} m from the waiting hauler",
                    at.distance(start)
                );
                let along = (at - start).dot(direction).clamp(0.0, leg);
                let off_leg = at.distance(start + direction * along);
                assert!(
                    off_leg >= RAIDER_LATERAL.get() - SLACK && off_leg < RAIDER_ENGAGE_RANGE.get(),
                    "raider {index} of {raiders} waits {off_leg} m off the leg"
                );
                for other in &points[..index] {
                    assert!(
                        at.distance(other.get()) >= RAIDER_SPACING.get() - SLACK,
                        "raider {index} of {raiders} stands apart from the others"
                    );
                }
            }
        }
    }

    #[test]
    fn a_held_recovery_is_dropped_when_its_wreck_is_destroyed_unless_it_is_done() {
        let recover = recover(0);
        let mut app = held_app([recover.clone()]);
        fire(&mut app, taken_out(&wreck_id()));
        assert!(
            !jobs(&app).holds(&recover.id),
            "a recovery with no wreck left to lift from is dropped"
        );

        let mut app = held_app([recover.clone()]);
        app.world_mut()
            .resource_mut::<Jobs>()
            .mark_done(&recover.id);
        fire(&mut app, taken_out(&wreck_id()));
        assert!(jobs(&app).holds(&recover.id), "the lift is done; it pays");
        assert!(jobs(&app).is_done(&recover.id));
    }

    #[test]
    fn a_held_genuine_distress_call_is_dropped_when_its_caller_is_destroyed_and_a_trap_is_done() {
        let genuine = distress(0, true);
        let JobType::Distress { caller, .. } = &genuine.kind else {
            unreachable!()
        };
        let mut app = held_app([genuine.clone()]);
        let bodies = stand(&mut app, &genuine.spawned_bodies());
        fire(&mut app, taken_out(caller));
        assert!(
            !jobs(&app).holds(&genuine.id),
            "a caller destroyed before the dock is lost"
        );
        assert_eq!(standing(&app, &bodies), 0, "the remains are taken out");

        let trap = distress(1, false);
        let JobType::Distress { caller, .. } = &trap.kind else {
            unreachable!()
        };
        let mut app = held_app([trap.clone()]);
        fire(&mut app, taken_out(caller));
        assert!(jobs(&app).holds(&trap.id), "a trap sprung is the job done");
        assert!(jobs(&app).is_done(&trap.id));
    }

    #[test]
    fn every_held_job_is_dropped_when_its_poster_dies_done_or_not() {
        let escort = escort(0);
        let JobType::Escort { hauler, .. } = &escort.kind else {
            unreachable!()
        };
        let held = [escort.clone(), procure(1), neutralize(2)];
        let mut app = held_app(held.clone());
        let bodies = stand(&mut app, &escort.spawned_bodies());
        fire(&mut app, arrived(hauler));
        assert!(jobs(&app).is_done(&escort.id));

        fire(&mut app, taken_out(&poster()));
        assert!(
            jobs(&app).held().is_empty(),
            "no hand-in is left for any of them: {:?}",
            jobs(&app).held()
        );
        assert!(jobs(&app).done().is_empty());
        assert_eq!(
            standing(&app, &bodies),
            0,
            "the dead poster never renews the slot, so the drop takes the bodies out"
        );
    }

    #[test]
    fn a_held_job_s_spawned_bodies_are_off_the_neighbouring_field() {
        let items = test_items();
        let escort = escort(0);
        let distress = distress(1, true);
        let JobType::Distress { caller, .. } = &distress.kind else {
            unreachable!()
        };
        let mut app = App::new();
        let root = app.world_mut().spawn_empty().id();
        let hostile = |app: &mut App, id: &GeneratedBodyId| {
            app.world_mut().spawn((
                id.clone(),
                Name::new(id.to_string()),
                Allegiance::Enemy,
                ChildOf(root),
                SpaceshipRootMarker,
            ));
        };
        let wreck = |app: &mut App, id: &GeneratedBodyId| {
            app.world_mut().spawn((
                id.clone(),
                Name::new(id.to_string()),
                ShipInventory::new(&items, 1_000_000, [(item(), 1)]),
                ChildOf(root),
                SpaceshipRootMarker,
                DerelictShipMarker,
            ));
        };
        for raider in escort.raider_ids() {
            hostile(&mut app, &raider);
        }
        hostile(&mut app, &hull_id());
        wreck(&mut app, caller);
        wreck(&mut app, &wreck_id());
        let field = |app: &mut App, held: Jobs| {
            let excluded = JobPostings::default().named_bodies(
                JobOfferRule {
                    offer_seconds: 100.0,
                },
                0.0,
                &held,
            );
            app.world_mut()
                .run_system_once(
                    move |wrecks: Query<
                        (&GeneratedBodyId, &Name, &ShipInventory, &ChildOf),
                        (
                            With<SpaceshipRootMarker>,
                            With<DerelictShipMarker>,
                            Without<StationMarker>,
                        ),
                    >,
                          hulls: Query<
                        (&GeneratedBodyId, &Name, &Allegiance, &ChildOf),
                        (
                            With<SpaceshipRootMarker>,
                            Without<DerelictShipMarker>,
                            Without<NeutralizedMarker>,
                            Without<StationMarker>,
                        ),
                    >| { job_field(&[root], &excluded, &wrecks, &hulls) },
                )
                .expect("the field reads the queries")
        };
        let ids = |hulls: &[JobHull]| hulls.iter().map(|hull| hull.id.clone()).collect::<Vec<_>>();
        let wreck_ids = |wrecks: &[JobWreck]| {
            wrecks
                .iter()
                .map(|wreck| wreck.id.clone())
                .collect::<Vec<_>>()
        };

        let open = field(&mut app, Jobs::default());
        assert_eq!(open.hostiles.len(), 4, "the raiders stand as hostiles");
        assert_eq!(open.wrecks.len(), 2, "the caller stands as a wreck");

        let mut jobs = Jobs::default();
        jobs.accept(escort.clone());
        jobs.accept(distress.clone());
        let posted = field(&mut app, jobs);
        assert_eq!(ids(&posted.hostiles), vec![hull_id()]);
        assert_eq!(wreck_ids(&posted.wrecks), vec![wreck_id()]);
    }

    #[test]
    fn a_held_job_marks_its_body_until_done_then_its_poster_and_remarks_a_thawed_body() {
        let answer = distress(0, true);
        let JobType::Distress { caller, .. } = answer.kind.clone() else {
            unreachable!()
        };
        let recovery = recover(1);
        let mut app = held_app([answer.clone(), recovery.clone(), procure(2)]);
        app.add_systems(Update, mark_held_job_targets);
        let tags = |app: &mut App| -> Vec<(GeneratedBodyId, String)> {
            let mut tags: Vec<_> = app
                .world_mut()
                .query::<(&GeneratedBodyId, &ObjectiveMarkerTarget)>()
                .iter(app.world())
                .map(|(id, tag)| (id.clone(), tag.label.clone()))
                .collect();
            tags.sort();
            tags
        };

        let [station, _] = stand(&mut app, &[poster(), wreck_id()])[..] else {
            unreachable!()
        };
        app.update();
        assert_eq!(
            tags(&mut app),
            vec![(wreck_id(), "JOB-2 Recover".to_string())],
            "the unspawned caller has no entity and no chip, the procurement marks nothing"
        );

        stand(&mut app, std::slice::from_ref(&caller));
        app.update();
        assert_eq!(
            tags(&mut app),
            vec![
                (caller.clone(), "JOB-1 Answer".to_string()),
                (wreck_id(), "JOB-2 Recover".to_string()),
            ],
            "the caller is marked the frame it spawns"
        );

        let mut jobs = app.world_mut().resource_mut::<Jobs>();
        jobs.mark_done(&answer.id);
        jobs.mark_done(&recovery.id);
        app.update();
        assert_eq!(
            tags(&mut app),
            vec![(poster(), "JOB-1, JOB-2 Hand in".to_string())],
            "two done jobs share their poster's one chip"
        );

        app.world_mut().despawn(station);
        app.update();
        assert!(tags(&mut app).is_empty(), "a frozen poster has no chip");
        stand(&mut app, &[poster()]);
        app.update();
        assert_eq!(
            tags(&mut app),
            vec![(poster(), "JOB-1, JOB-2 Hand in".to_string())],
            "the thawed poster is marked again"
        );

        app.world_mut().resource_mut::<Jobs>().complete(&answer.id);
        app.update();
        assert_eq!(
            tags(&mut app),
            vec![(poster(), "JOB-1 Hand in".to_string())],
            "the hand-in takes its code off and the next job moves up"
        );
    }
}
