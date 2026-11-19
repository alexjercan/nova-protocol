//! The encounter director: what counts the player's eligible travel, fires
//! the drawn [`EncounterDesign`] ahead of the ship and ends it.
//!
//! The record is [`Encounters`]: `chart::clear_chart` resets it with the
//! ledger and a save carries it. Travel counts on virtual time only while
//! every rule of [`travel_counts`] holds. When the count reaches the gap
//! [`plan_encounter_draw`] drew, the director looks for a place
//! ([`plan_encounter_place`]) and puts the design's bodies there, each a
//! generated hull under the live cell root of where it stands, with a
//! [`GeneratedBodyId`] whose stem carries the draw number, so the streaming
//! layer adopts, freezes and saves it like any cell body and no id is minted
//! twice. A draw with no design for the hull, or one the place cannot staff,
//! is spent; a due draw with no place waits and tries again every
//! [`ENCOUNTER_RETRY_SECONDS`].
//!
//! A hail puts a stranded civilian hull there and offers its job
//! ([`plan_hail_offer`]); the player answers with a [`HailAnswerCommand`]
//! or by docking with the caller, and the job pays on the dock that does it
//! ([`plan_hail_pay`]). An ambush puts raiders in wait; a derelict find puts
//! one wreck there. [`end_encounters`] ends each by its own rule, and
//! [`mark_encounter_bodies`] keeps the `HAIL` and `SIGNAL` chips on the
//! bodies, after a thaw and a Load too.
//!
//! The live record names only bodies a cell holds, live or frozen: a body
//! that is destroyed comes off it ([`lose_bodies`]) and the record ends
//! with its last one. A world save runs [`drop_lost_bodies`] before it reads
//! the record, so its check that every named body is a saved ship holds in
//! the middle of a fight. For as long as the record names a body no board
//! draws a job on it (`jobs::FieldBodies`).

use std::collections::BTreeMap;

use avian3d::prelude::LinearVelocity;
use bevy::{
    ecs::{entity::EntityHashSet, system::SystemParam},
    prelude::*,
};
use nova_events::prelude::{
    CreditChangeCauseType, EntityId, GameEvent, GeneratedBodyId, JobDropCauseType, Meters, Meters3,
    MetersPerSecond, SectorCoord,
};
use nova_gameplay::prelude::{
    default_comms_accent, escort_raider_counts, escort_raider_size, plan_encounter_draw,
    plan_hail_accept, plan_hail_offer, plan_hail_pay, unit_sphere_point, EncounterDesign,
    EncounterFireCommand, EncounterLive, EncounterOffer, EncounterType, Encounters, Fnv32,
    GameEncounters, GameItems, HailAnswerCommand, HailAnswerType, HailAnswered, HailRaised, Job,
    JobAccepted, JobDone, JobDropped, JobHandedIn, JobType, Jobs, NeutralizedMarker,
    ObjectiveMarkerTarget, PlayerSpaceshipMarker, SeedStream, ShipCredits, ShipInventory,
    ShipInventoryReports, ShipRoleType, ShipSizeType, SpaceshipRootMarker, StationMarker,
    HAIL_OFFER_SECONDS,
};
use nova_scenario::prelude::{NarrativeCueActionConfig, NovaEventWorld};
use nova_ship::prelude::{AIBehaviorState, DockedShip, ShipRescueCall, ThreatContacts, WeaponsHot};
use nova_world::prelude::{
    desired_sectors, live_sectors, CurrentSector, FrozenSectors, NovaWorldSystems,
    PendingSectorStation, SectorRoot, SectorShipConditionType, ThawedAfter, WorldClock,
    WorldConfig,
};

use crate::{
    civilizations::{Civilization, CivilizationField},
    jobs::{
        answer_distress, caller_bundle, caller_name, caller_stock, deliver_supply, job_hull,
        player_docked_with, raider_bundle, raider_name, PlayerHull, FIELD_LEG_STEP,
        JOB_SPAWN_CLEARANCE, RAIDER_SPACING,
    },
    sector_ships::{ship_credits, ship_stock, uniform_rotation},
    ship_layout::{generate_wreck, ShipLayoutRequest, ShipSpareDriveType},
    NovaLayeredWorld,
};

/// The speed the player must hold for travel to count.
const TRAVEL_SPEED_MIN: MetersPerSecond = MetersPerSecond(50.0);
/// How far from every station the player must be for travel to count, and
/// how far from every station an encounter is placed.
const STATION_STANDOFF: Meters = Meters(10_000.0);
/// How many seconds of flight ahead of the player an encounter is placed.
const PLACE_LEAD_SECONDS: f32 = 30.0;
/// The nearest and the farthest an encounter is placed ahead of the player.
const PLACE_LEAD: (Meters, Meters) = (Meters(6_000.0), Meters(40_000.0));
/// How far off the player's line of flight an encounter is placed.
const PLACE_ASIDE: Meters = Meters(2_000.0);
/// How many turns around the line of flight a placement tries.
const PLACE_TRIES: u32 = 8;
/// The virtual seconds a due encounter with no place waits before the
/// director looks for a place again.
const ENCOUNTER_RETRY_SECONDS: f32 = 5.0;
/// How far from every raider of an ambush the player must be for the ambush
/// to end with raiders still standing.
const AMBUSH_BREAK_OFF: Meters = Meters(30_000.0);
/// How close the player must come to a derelict find for its signal to end.
const DERELICT_FOUND: Meters = Meters(1_000.0);
/// The virtual seconds between two looks in the frozen ledger for a body of
/// the live encounter that does not stand: the scan reads every frozen
/// body.
const LEDGER_LOOK_SECONDS: f32 = 1.0;
/// The chip a hailing hull wears while its offer stands.
pub const HAIL_CHIP: &str = "HAIL";
/// The chip a derelict find wears until the player comes within
/// [`DERELICT_FOUND`].
pub const SIGNAL_CHIP: &str = "SIGNAL";

/// The encounter director's systems, in `Update` after
/// [`NovaWorldSystems::Retire`]: after the station boards post and before
/// the held jobs mark their bodies, so the `HAIL` chip is off a caller
/// before its accepted job's chip goes on.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EncounterSystems;

/// The open world's encounter director.
pub struct EncounterPlugin;

impl Plugin for EncounterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Encounters>();
        app.add_message::<HailAnswerCommand>();
        app.add_message::<HailAnswered>();
        app.add_message::<EncounterFireCommand>();
        app.add_observer(hail_on_dock);
        app.add_observer(pay_hail_on_done);
        app.add_observer(end_on_hail_drop);
        app.add_systems(
            Update,
            (
                answer_hails,
                end_encounters,
                drive_encounters,
                mark_encounter_bodies,
            )
                .chain()
                .in_set(EncounterSystems)
                .after(NovaWorldSystems::Retire)
                .run_if(
                    resource_exists::<WorldConfig<NovaLayeredWorld>>
                        .and_then(resource_exists::<GameEncounters>),
                ),
        );
    }
}

/// What the travel clock reads off the player and the world on one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TravelFacts {
    /// The player's hull is docked.
    pub docked: bool,
    /// The player's speed.
    pub speed: MetersPerSecond,
    /// The distance to the nearest standing station, if one stands.
    pub nearest_station: Option<Meters>,
    /// The player's weapons are hot or a hostile contact is tracked.
    pub in_combat: bool,
    /// The player's hull is neutralized.
    pub neutralized: bool,
    /// The player's rescue call stands.
    pub calling_rescue: bool,
    /// An encounter is live or a hail offer stands.
    pub encounter_live: bool,
    /// The player holds a hail job.
    pub hail_job_held: bool,
}

/// Whether this frame's travel counts toward the next encounter: the player
/// is undocked, faster than [`TRAVEL_SPEED_MIN`], more than
/// [`STATION_STANDOFF`] from every standing station, not in combat, not
/// neutralized, with no rescue call, no live encounter or hail offer and no
/// held hail job.
pub(crate) fn travel_counts(facts: &TravelFacts) -> bool {
    !facts.docked
        && facts.speed.0 > TRAVEL_SPEED_MIN.0
        && facts
            .nearest_station
            .is_none_or(|distance| distance.get() > STATION_STANDOFF.get())
        && !facts.in_combat
        && !facts.neutralized
        && !facts.calling_rescue
        && !facts.encounter_live
        && !facts.hail_job_held
}

/// What one frame of eligible travel leads to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TravelStepType<'a> {
    /// The gap has not passed.
    Wait,
    /// The gap passed and no design may fire for the hull: the draw was
    /// spent.
    Spent,
    /// The gap passed: this design is due a place.
    Due(&'a EncounterDesign),
}

/// Count `seconds` of eligible travel on `record` and read the draw of a
/// world of `seed` for a hull that is `armed` or not. The count stops at the
/// drawn gap, so a due encounter that waits for a place stays due. A due
/// draw with no design is spent here ([`Encounters::consume_draw`]).
pub(crate) fn step_travel<'a>(
    record: &mut Encounters,
    seconds: f64,
    seed: u32,
    designs: &'a GameEncounters,
    armed: bool,
) -> TravelStepType<'a> {
    let draw = plan_encounter_draw(seed, record.fired, designs, armed);
    let gap = f64::from(draw.gap_s);
    if record.travel_seconds < gap {
        record.count_travel(seconds);
    }
    if record.travel_seconds < gap {
        return TravelStepType::Wait;
    }
    match draw.design {
        Some(design) => TravelStepType::Due(design),
        None => {
            record.consume_draw();
            TravelStepType::Spent
        }
    }
}

/// Where an encounter goes for a player at `from` flying along the unit
/// `heading` at `speed`: [`PLACE_LEAD_SECONDS`] of flight ahead, inside
/// [`PLACE_LEAD`], and [`PLACE_ASIDE`] off the line of flight. The first of
/// [`PLACE_TRIES`] turns around the line, the first one drawn off `seed`,
/// that stands in a cell of `live`, [`JOB_SPAWN_CLEARANCE`] from every
/// position of `standing` and [`STATION_STANDOFF`] from every one of
/// `stations`. `None` when no turn fits.
#[expect(
    clippy::too_many_arguments,
    reason = "one placement: the seed, the player's motion, the grid and the three clearances"
)]
pub(crate) fn plan_encounter_place(
    seed: u32,
    from: Meters3,
    heading: Vec3,
    speed: MetersPerSecond,
    edge: Meters,
    live: &BTreeMap<SectorCoord, Entity>,
    standing: &[Meters3],
    stations: &[Meters3],
) -> Option<Meters3> {
    let lead = (speed.0 * PLACE_LEAD_SECONDS).clamp(PLACE_LEAD.0.get(), PLACE_LEAD.1.get());
    let ahead = from.get() + heading * lead;
    let across = heading.any_orthonormal_vector();
    let first = SeedStream::new(seed).unit() * std::f32::consts::TAU;
    (0..PLACE_TRIES).find_map(|turn| {
        let angle = first + turn as f32 / PLACE_TRIES as f32 * std::f32::consts::TAU;
        let aside = Quat::from_axis_angle(heading, angle) * across;
        let place = Meters3(ahead + aside * PLACE_ASIDE.get());
        let clear = |bodies: &[Meters3], clearance: Meters| {
            bodies
                .iter()
                .all(|body| place.distance(*body).get() >= clearance.get())
        };
        (live.contains_key(&SectorCoord::containing(place, edge))
            && clear(standing, JOB_SPAWN_CLEARANCE)
            && clear(stations, STATION_STANDOFF))
        .then_some(place)
    })
}

/// The player's hull as the director reads it.
#[derive(SystemParam)]
pub(crate) struct EncounterPlayer<'w, 's> {
    ship: Query<
        'w,
        's,
        (
            &'static GlobalTransform,
            Option<&'static LinearVelocity>,
            Has<DockedShip>,
            Option<&'static WeaponsHot>,
            Option<&'static ThreatContacts>,
            Has<NeutralizedMarker>,
            Has<ShipRescueCall>,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
}

impl EncounterPlayer<'_, '_> {
    /// Where the player's hull stands, while one does.
    fn position(&self) -> Option<Meters3> {
        let (at, ..) = self.ship.single().ok()?;
        // Engine boundary: a transform counts engine units.
        Some(Meters3::from_engine(at.translation()))
    }

    /// The player's velocity in meters per second and the way the hull
    /// points.
    fn motion(&self) -> Option<(Vec3, Vec3)> {
        let (at, velocity, ..) = self.ship.single().ok()?;
        // Engine boundary: avian counts engine units per second.
        let velocity = velocity.map_or(Vec3::ZERO, |velocity| {
            nova_events::prelude::MetersPerSecond3::from_engine(velocity.0).get()
        });
        Some((velocity, at.forward().as_vec3()))
    }
}

/// The standing bodies and stations a placement clears and the live cells.
#[derive(SystemParam)]
pub(crate) struct EncounterField<'w, 's> {
    roots: Query<'w, 's, (Entity, &'static SectorRoot)>,
    bodies: Query<'w, 's, (&'static GeneratedBodyId, &'static GlobalTransform)>,
    stations: Query<'w, 's, &'static GlobalTransform, With<StationMarker>>,
    held_stations: Query<'w, 's, &'static PendingSectorStation>,
}

impl EncounterField<'_, '_> {
    /// Where every station stands: the live ones and the ones a cell holds
    /// back until the player's hull is clear of them.
    fn stations(&self) -> Vec<Meters3> {
        self.stations
            .iter()
            .map(|at| Meters3::from_engine(at.translation()))
            .chain(
                self.held_stations
                    .iter()
                    .map(|held| held.station().position),
            )
            .collect()
    }
}

/// Why a design's bodies were not put in the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StageRefusalType {
    /// No place fits now: the draw stays due.
    NoPlace,
    /// The place cannot stage the design: a due draw is spent, a command
    /// is refused.
    Unstaffed,
}

/// Advance the travel clock and fire what is due, and fire every
/// [`EncounterFireCommand`] at once.
///
/// Travel counts by [`travel_counts`] on virtual time. A due design is
/// placed ahead on the player's velocity; a command's design is placed
/// ahead of the hull's nose when the hull stands still, and is refused
/// while an encounter is live, a hail job is held or no player hull stands.
/// A place that cannot stage the design spends a due draw: no civilization
/// in reach builds the hull, the hull or its hold fails, the catalog has no
/// item a supply hail may ask for, or the player's hull cannot take the
/// hail's job ([`plan_hail_accept`]). A command such a place refuses, or
/// one with no place, puts nothing in the world, spends no draw and leaves
/// the travel count. Every refused command is logged with its design.
#[expect(
    clippy::too_many_arguments,
    reason = "one director: the clock, the catalogs, the record, the player, the field and the comms"
)]
pub(crate) fn drive_encounters(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    config: Res<WorldConfig<NovaLayeredWorld>>,
    designs: Res<GameEncounters>,
    items: Res<GameItems>,
    clock: Res<WorldClock>,
    jobs: Res<Jobs>,
    mut record: ResMut<Encounters>,
    hull: PlayerHull,
    player: EncounterPlayer,
    field: EncounterField,
    mut comms: Option<ResMut<NovaEventWorld>>,
    mut asked: MessageReader<EncounterFireCommand>,
    mut retry: Local<f32>,
) {
    let asked: Vec<EncounterFireCommand> = asked.read().cloned().collect();
    let Some(from) = player.position() else {
        for command in &asked {
            warn!(
                "nova_world_base: the encounter '{}' is not fired: no player hull stands",
                command.design
            );
        }
        return;
    };
    let (velocity, nose) = player.motion().expect("the player's hull stands");
    let speed = MetersPerSecond(velocity.length());
    let stations = field.stations();
    let hail_job_held = jobs.held().iter().any(|job| job.id.is_hail());
    let fit = hull.fit();

    let mut stage = |serial: u32, design: &EncounterDesign| {
        let heading = velocity.try_normalize().unwrap_or(nose);
        let key = |aspect: &[u8]| {
            Fnv32::new()
                .write(&config.seed.to_le_bytes())
                .write(b"encounter")
                .write(aspect)
                .write(&serial.to_le_bytes())
                .finish()
        };
        let live = live_sectors(field.roots.iter());
        let standing: Vec<Meters3> = field
            .bodies
            .iter()
            .map(|(_, at)| Meters3::from_engine(at.translation()))
            .collect();
        let Some(place) = plan_encounter_place(
            key(b"place"),
            from,
            heading,
            speed,
            config.sector_edge,
            &live,
            &standing,
            &stations,
        ) else {
            return Err(StageRefusalType::NoPlace);
        };
        let cell = SectorCoord::containing(place, config.sector_edge);
        let root = live[&cell];
        let parts = config.generator.parts();
        let role = match design.kind {
            EncounterType::Hail { .. } | EncounterType::Derelict => ShipRoleType::Civilian,
            EncounterType::Ambush => ShipRoleType::Scavenger,
        };
        // The strongest civilization at the place that builds the hull. The
        // lattice puts every place in reach of one, so a fault is a place
        // off the grid.
        let mut reach = CivilizationField::new(config.seed)
            .in_reach(place)
            .unwrap_or_else(|fault| {
                panic!("drive_encounters: no civilization at {place:?}: {fault}")
            });
        reach.sort_by(|a, b| b.influence.total_cmp(&a.influence));
        let Some(civilization) =
            reach
                .into_iter()
                .map(|reach| reach.civilization)
                .find(|civilization| {
                    parts
                        .eligible_roles(civilization.id, civilization.advancement)
                        .contains(&role)
                })
        else {
            info!(
                "nova_world_base: the encounter '{}' cannot be staged: no civilization at its \
                 place builds a {} hull",
                design.id,
                role.label()
            );
            return Err(StageRefusalType::Unstaffed);
        };
        let body = |part: &str, index: u32| {
            GeneratedBodyId::new(cell, format!("encounter_{serial}_{part}"), index)
        };
        let unstaffed = |fault: String| {
            warn!(
                "nova_world_base: the encounter '{}' cannot be staged: {fault}",
                design.id
            );
            Err(StageRefusalType::Unstaffed)
        };
        let now = clock.play_seconds;
        let turn = Quat::from_rotation_arc(Vec3::NEG_Z, unit_sphere_point(key(b"turn")));
        let (name, bodies, offer) = match &design.kind {
            EncounterType::Hail { job, line } => {
                let caller = body("caller", 0);
                let leg = from.distance(place).get();
                let leg_m = ((leg / FIELD_LEG_STEP.get()).round() * FIELD_LEG_STEP.get()) as u32;
                let Some(offered) = plan_hail_offer(
                    config.seed,
                    *job,
                    &caller,
                    serial,
                    leg_m,
                    civilization.advancement,
                    &items,
                ) else {
                    return unstaffed("the catalog holds no item a supply job asks for".into());
                };
                if let Err(refusal) = plan_hail_accept(&offered, &jobs, false, &items, fit) {
                    info!(
                        "nova_world_base: the encounter '{}' cannot be staged: the hull cannot \
                         take its job ({refusal:?})",
                        design.id
                    );
                    return Err(StageRefusalType::Unstaffed);
                }
                let (hull, stock) = match job_hull(
                    parts,
                    &items,
                    &civilization,
                    key(b"caller"),
                    role,
                    ShipSizeType::Advancement,
                    SectorShipConditionType::Derelict,
                    ShipSpareDriveType::Without,
                ) {
                    Ok(built) => built,
                    Err(fault) => return unstaffed(fault),
                };
                let name = caller_name(&civilization);
                commands.spawn(caller_bundle(
                    &caller,
                    name.clone(),
                    place,
                    turn,
                    hull,
                    caller_stock(&offered.kind, stock),
                    0,
                    root,
                ));
                if let Some(comms) = comms.as_mut() {
                    comms.push_narrative_cue(NarrativeCueActionConfig {
                        speaker: name.clone(),
                        text: line.clone(),
                        dwell: None,
                        icon: None,
                        accent: default_comms_accent(),
                    });
                }
                commands.trigger(HailRaised {
                    caller: name.clone(),
                    job: offered.clone(),
                });
                let offer = EncounterOffer {
                    job: offered,
                    expires_at: now + HAIL_OFFER_SECONDS,
                };
                (name, vec![caller], Some(offer))
            }
            EncounterType::Ambush => {
                let (least, most) = escort_raider_counts(civilization.advancement);
                let raiders =
                    least + SeedStream::new(key(b"raiders")).next_u32() % (most - least + 1);
                let across = heading.any_orthonormal_vector();
                let mut built = Vec::new();
                for index in 0..raiders {
                    match job_hull(
                        parts,
                        &items,
                        &civilization,
                        key(format!("raider_{index}").as_bytes()),
                        role,
                        // The budget an escort's raiders get at this
                        // advancement.
                        escort_raider_size(civilization.advancement),
                        SectorShipConditionType::Intact,
                        ShipSpareDriveType::Without,
                    ) {
                        Ok(hull) => built.push(hull),
                        Err(fault) => return unstaffed(fault),
                    }
                }
                let mut ids = Vec::new();
                for (index, (hull, stock)) in built.into_iter().enumerate() {
                    let id = body("raider", index as u32);
                    let offset =
                        (index as f32 - (raiders as f32 - 1.0) / 2.0) * RAIDER_SPACING.get();
                    let at = Meters3(place.get() + across * offset);
                    let facing = (from.get() - at.get()).try_normalize().unwrap_or(-heading);
                    commands.spawn(raider_bundle(
                        &id,
                        &civilization,
                        at,
                        Quat::from_rotation_arc(Vec3::NEG_Z, facing),
                        hull,
                        stock,
                        root,
                    ));
                    ids.push(id);
                }
                (raider_name(&civilization), ids, None)
            }
            EncounterType::Derelict => {
                let wreck = body("wreck", 0);
                let built = derelict_hull(parts, &items, &civilization, key(b"wreck"));
                let (hull, stock) = match built {
                    Ok(built) => built,
                    Err(fault) => return unstaffed(fault),
                };
                let name = format!("{} derelict", civilization.id.name());
                commands.spawn(caller_bundle(
                    &wreck,
                    name.clone(),
                    place,
                    uniform_rotation({
                        let mut stream = SeedStream::new(key(b"wreck_turn"));
                        [stream.unit(), stream.unit(), stream.unit()]
                    }),
                    hull,
                    stock,
                    ship_credits(
                        civilization.advancement,
                        SectorShipConditionType::Derelict,
                        key(b"wreck_credits"),
                    ),
                    root,
                ));
                (name, vec![wreck], None)
            }
        };
        info!(
            "nova_world_base: the encounter '{}' fires as number {serial} at {now:.1} s of play: \
             {} body(ies) of {} {:.0} m out, first '{}'",
            design.id,
            bodies.len(),
            civilization.id.name(),
            from.distance(place).get(),
            bodies[0]
        );
        Ok((
            EncounterLive {
                design: design.id.clone(),
                name,
                bodies,
                started_at: now,
            },
            offer,
        ))
    };

    for command in asked {
        if record.live.is_some() || hail_job_held {
            warn!(
                "nova_world_base: the encounter '{}' is not fired: one is live or a hail job is \
                 held",
                command.design
            );
            continue;
        }
        let Some(design) = designs.get(&command.design) else {
            warn!(
                "nova_world_base: the encounter '{}' is not fired: no encounter has that id",
                command.design
            );
            continue;
        };
        match stage(record.fired, design) {
            Ok((live, offer)) => record.fire(live, offer),
            // A command is one try. A try that puts nothing in the world
            // spends no draw and leaves the travel count: the next natural
            // encounter keeps its number and its gap.
            Err(StageRefusalType::NoPlace) => warn!(
                "nova_world_base: the encounter '{}' is not fired: no place ahead is clear",
                design.id
            ),
            Err(StageRefusalType::Unstaffed) => warn!(
                "nova_world_base: the encounter '{}' is not fired: its place cannot stage it",
                design.id
            ),
        }
    }

    let (docked, in_combat, neutralized, calling_rescue) = player
        .ship
        .single()
        .map(|(_, _, docked, hot, contacts, neutralized, calling)| {
            let in_combat = hot.is_some_and(|hot| hot.0)
                || contacts.is_some_and(|contacts| !contacts.entries.is_empty());
            (docked, in_combat, neutralized, calling)
        })
        .expect("the player's hull stands");
    let facts = TravelFacts {
        docked,
        speed,
        nearest_station: stations
            .iter()
            .map(|station| from.distance(*station))
            .min_by(|a, b| a.get().total_cmp(&b.get())),
        in_combat,
        neutralized,
        calling_rescue,
        encounter_live: record.live.is_some() || record.offer.is_some(),
        hail_job_held,
    };
    if !travel_counts(&facts) {
        return;
    }
    // Through the bypass: a frame of travel must not wake the readers of
    // the record; a spent draw and a fired encounter mark it below.
    let before = record.fired;
    let step = step_travel(
        record.bypass_change_detection(),
        f64::from(time.delta_secs()),
        config.seed,
        &designs,
        fit.has_weapon,
    );
    match step {
        TravelStepType::Wait => {}
        TravelStepType::Spent => {
            record.set_changed();
            info!(
                "nova_world_base: encounter number {before} is spent: no design fires for this \
                 hull"
            );
        }
        TravelStepType::Due(design) => {
            *retry -= time.delta_secs();
            if *retry > 0.0 {
                return;
            }
            match stage(record.fired, design) {
                Ok((live, offer)) => {
                    record.fire(live, offer);
                    *retry = 0.0;
                }
                Err(StageRefusalType::NoPlace) => *retry = ENCOUNTER_RETRY_SECONDS,
                Err(StageRefusalType::Unstaffed) => {
                    info!(
                        "nova_world_base: encounter number {before} is spent: its place cannot \
                         stage '{}'",
                        design.id
                    );
                    record.consume_draw();
                    *retry = 0.0;
                }
            }
        }
    }
}

/// The wreck of a civilian hull of `civilization` and its hold, as a
/// sector's derelict is built: the layout ruined off `seed`, the stock drawn
/// off the next seed.
fn derelict_hull(
    parts: &crate::ship_parts::ShipPartSnapshot,
    items: &GameItems,
    civilization: &Civilization,
    seed: u32,
) -> Result<
    (
        nova_scenario::prelude::ShipDesign,
        nova_gameplay::prelude::ShipInventoryStock,
    ),
    String,
> {
    let role = ShipRoleType::Civilian;
    let request = ShipLayoutRequest {
        seed,
        civilization: civilization.id,
        role,
        advancement: civilization.advancement,
        size: ShipSizeType::Advancement,
        spare_drive: ShipSpareDriveType::Without,
    };
    let layout = generate_wreck(parts, request).map_err(|failure| failure.to_string())?;
    let stock = ship_stock(
        parts,
        items,
        &layout.design,
        role,
        SectorShipConditionType::Derelict,
        civilization.advancement,
        seed.wrapping_add(1),
    )
    .ok_or_else(|| "the hold cannot fit one unit of its lightest item".to_string())?;
    Ok((layout.design, stock))
}

/// Hold the standing offer's job: the offer comes off the record, the job
/// goes on the player's and [`JobAccepted`] is raised. The live encounter
/// stands until the job ends.
fn accept_offer(commands: &mut Commands, record: &mut Encounters, jobs: &mut Jobs) {
    let offer = record.offer.take().expect("the caller checked the offer");
    info!(
        "nova_world_base: the hail job '{}' is accepted",
        offer.job.id
    );
    jobs.accept(offer.job.clone());
    commands.trigger(JobAccepted { job: offer.job });
}

/// Answer the standing hail offer with each [`HailAnswerCommand`]: an accept
/// [`plan_hail_accept`] passes holds the job; a refused one changes nothing;
/// a decline ends the offer and the encounter, and the hull stays where it
/// is. With no offer standing a command changes nothing. Each command is
/// answered with one [`HailAnswered`].
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the commands, the record, the jobs, the catalog, the clock and the hull"
)]
pub(crate) fn answer_hails(
    mut commands: Commands,
    mut answers: MessageReader<HailAnswerCommand>,
    mut answered: MessageWriter<HailAnswered>,
    mut record: ResMut<Encounters>,
    mut jobs: ResMut<Jobs>,
    items: Res<GameItems>,
    clock: Res<WorldClock>,
    hull: PlayerHull,
) {
    for HailAnswerCommand { answer } in answers.read() {
        let taken = match (answer, record.offer.as_ref()) {
            (_, None) => false,
            (HailAnswerType::Accept, Some(offer)) => {
                let expired = record.offer_expired(clock.play_seconds);
                match plan_hail_accept(&offer.job, &jobs, expired, &items, hull.fit()) {
                    Ok(()) => {
                        accept_offer(&mut commands, &mut record, &mut jobs);
                        true
                    }
                    Err(refusal) => {
                        info!(
                            "nova_world_base: the hail job '{}' is not accepted ({refusal:?})",
                            offer.job.id
                        );
                        false
                    }
                }
            }
            (HailAnswerType::Decline, Some(offer)) => {
                info!(
                    "nova_world_base: the hail job '{}' is declined",
                    offer.job.id
                );
                record.end();
                true
            }
        };
        answered.write(HailAnswered {
            answer: *answer,
            taken,
        });
    }
}

/// On the player's dock with a hailing hull: hold its standing offer first
/// when [`plan_hail_accept`] passes, then answer the held hail job the dock
/// does, as a board job's dock answers it ([`answer_distress`],
/// [`deliver_supply`]). A supply dock with the hold short leaves the job
/// held and open.
pub(crate) fn hail_on_dock(
    event: On<GameEvent>,
    mut commands: Commands,
    mut record: ResMut<Encounters>,
    mut jobs: ResMut<Jobs>,
    items: Res<GameItems>,
    clock: Res<WorldClock>,
    mut player: ParamSet<(
        PlayerHull,
        Query<(&EntityId, &mut ShipInventory), With<PlayerSpaceshipMarker>>,
    )>,
) {
    let fit = player.p0().fit();
    let mut holds = player.p1();
    let Ok((own_id, mut hold)) = holds.single_mut() else {
        return;
    };
    let Some(subject) = player_docked_with(&event, own_id) else {
        return;
    };
    let offered = record
        .offer
        .as_ref()
        .filter(|offer| offer.job.id.poster.to_string() == subject)
        .map(|offer| offer.job.clone());
    if let Some(job) = offered {
        let expired = record.offer_expired(clock.play_seconds);
        match plan_hail_accept(&job, &jobs, expired, &items, fit) {
            Ok(()) => accept_offer(&mut commands, &mut record, &mut jobs),
            Err(refusal) => info!(
                "nova_world_base: the dock with '{subject}' does not accept '{}' ({refusal:?})",
                job.id
            ),
        }
    }
    let due: Vec<Job> = jobs
        .held()
        .iter()
        .filter(|job| {
            job.id.is_hail() && !jobs.is_done(&job.id) && job.id.poster.to_string() == subject
        })
        .cloned()
        .collect();
    for job in due {
        match &job.kind {
            JobType::Distress { .. } => answer_distress(&mut commands, &mut jobs, job),
            JobType::Supply { .. } => deliver_supply(&mut commands, &mut jobs, &mut hold, job),
            JobType::Procure { .. }
            | JobType::Recover { .. }
            | JobType::Escort { .. }
            | JobType::Neutralize { .. } => {
                unreachable!("a hail job is a distress call or a supply job")
            }
        }
    }
}

/// Pay the held, done hail job `job` where it stands ([`plan_hail_pay`]):
/// the player's balance takes the reward under the cause `JobReward`, the
/// job is listed completed, [`JobHandedIn`] is raised and the live encounter
/// ends. A balance the reward would overflow pays nothing and leaves the
/// job held and done; [`end_encounters`] tries again.
fn pay_hail(
    commands: &mut Commands,
    record: &mut Encounters,
    jobs: &mut Jobs,
    credits: &mut ShipCredits,
    reports: &mut ShipInventoryReports,
    job: &Job,
) {
    match plan_hail_pay(job, credits.0, jobs.is_done(&job.id)) {
        Ok(plan) => {
            reports.change_credits(credits, plan.own_cr, CreditChangeCauseType::JobReward);
            let code = jobs
                .mark_code(&job.id)
                .expect("a done hail job is held until it is paid");
            let job = jobs.complete(&job.id);
            info!(
                "nova_world_base: the hail job '{}' pays {} cr",
                job.id, plan.reward_cr
            );
            commands.trigger(JobHandedIn { job, code });
            record.end();
        }
        Err(refusal) => warn!(
            "nova_world_base: the hail job '{}' is not paid ({refusal:?})",
            job.id
        ),
    }
}

/// Pay a hail job the moment its work is done ([`pay_hail`]).
pub(crate) fn pay_hail_on_done(
    done: On<JobDone>,
    mut commands: Commands,
    mut record: ResMut<Encounters>,
    mut jobs: ResMut<Jobs>,
    mut player: Query<(&mut ShipCredits, &mut ShipInventoryReports), With<PlayerSpaceshipMarker>>,
) {
    let job = &done.event().job;
    if !job.id.is_hail() {
        return;
    }
    let Ok((mut credits, mut reports)) = player.single_mut() else {
        return;
    };
    pay_hail(
        &mut commands,
        &mut record,
        &mut jobs,
        &mut credits,
        &mut reports,
        job,
    );
}

/// End the live encounter when its hail job is dropped: abandoned by the
/// player or lost with its caller.
pub(crate) fn end_on_hail_drop(dropped: On<JobDropped>, mut record: ResMut<Encounters>) {
    let id = &dropped.event().job.id;
    let ours = record
        .live
        .as_ref()
        .is_some_and(|live| id.is_hail() && live.bodies.contains(&id.poster));
    if ours {
        record.end();
    }
}

/// Where a body of the live encounter is.
#[derive(Clone, Copy, Debug, PartialEq)]
enum BodyStateType {
    /// It stands, this far from the player, neutralized or not.
    Standing {
        /// Meters from the player's hull.
        distance: f32,
        /// It carries the neutralized marker.
        neutralized: bool,
    },
    /// Its cell is frozen, in the window the world is about to make live or
    /// outside it.
    Frozen {
        /// The cell is in the desired window: the body thaws shortly.
        thawing: bool,
    },
    /// No live cell and no frozen cell holds it: destroyed, or taken out of
    /// the world. [`lose_bodies`] takes it off the record.
    Lost,
}

/// The frozen ledger and the window, for a body that does not stand.
#[derive(SystemParam)]
pub(crate) struct EncounterLedger<'w> {
    config: Option<Res<'w, WorldConfig<NovaLayeredWorld>>>,
    current: Option<Res<'w, CurrentSector>>,
    frozen: Option<Res<'w, FrozenSectors>>,
}

impl EncounterLedger<'_> {
    /// Where the ledger holds the body `id`, if it does.
    fn state(&self, id: &GeneratedBodyId) -> BodyStateType {
        let id = id.to_string();
        let cell = self.frozen.as_ref().and_then(|frozen| {
            frozen
                .iter()
                .find(|(_, sector)| {
                    sector
                        .each_body()
                        .any(|body| body.id() == Some(id.as_str()))
                })
                .map(|(coord, _)| coord)
        });
        match cell {
            Some(cell) => BodyStateType::Frozen {
                thawing: self.config.as_ref().zip(self.current.as_ref()).is_some_and(
                    |(config, current)| {
                        desired_sectors(current.0, config.active_radius).contains(&cell)
                    },
                ),
            },
            None => BodyStateType::Lost,
        }
    }
}

/// Take the bodies `lost` off the live record: no cell holds them, live or
/// frozen, so the record would name what no save holds. The record ends
/// with its last body. A hail's one body is its caller: its standing offer
/// ends with it, and its held job is dropped as lost, done or not, since a
/// done job the dock could not pay has no caller left to pay it at.
fn lose_bodies(
    commands: &mut Commands,
    record: &mut Encounters,
    jobs: &mut Jobs,
    lost: &[GeneratedBodyId],
) {
    let Some(live) = record.live.as_mut() else {
        return;
    };
    live.bodies.retain(|body| !lost.contains(body));
    let left = live.bodies.len();
    let design = live.design.clone();
    for body in lost {
        info!(
            "nova_world_base: the encounter '{design}' loses '{body}': no cell holds it; {left} \
             body(ies) left"
        );
    }
    let orphaned: Vec<Job> = jobs
        .held()
        .iter()
        .filter(|job| job.id.is_hail() && lost.contains(&job.id.poster))
        .cloned()
        .collect();
    for job in orphaned {
        warn!(
            "nova_world_base: '{}' is lost: no cell holds its caller '{}'",
            job.id, job.id.poster
        );
        jobs.drop(&job.id);
        commands.trigger(JobDropped {
            job,
            cause: JobDropCauseType::Lost,
        });
    }
    if left == 0 {
        info!("nova_world_base: the encounter '{design}' ends: its last body is lost");
        record.end();
    }
}

/// Take off the live record every body no cell holds, live or frozen
/// ([`lose_bodies`]), reading the ledger now for each body that does not
/// stand.
///
/// The world save runs it before it reads the record, so a save never names
/// a body it does not hold: [`end_encounters`] reads the ledger only every
/// [`LEDGER_LOOK_SECONDS`], and a hull destroyed since its last look is
/// still on the record.
pub(crate) fn drop_lost_bodies(
    mut commands: Commands,
    mut record: ResMut<Encounters>,
    mut jobs: ResMut<Jobs>,
    ledger: EncounterLedger,
    standing: Query<&GeneratedBodyId>,
) {
    let Some(live) = record.live.as_ref() else {
        return;
    };
    let lost: Vec<GeneratedBodyId> = live
        .bodies
        .iter()
        .filter(|body| {
            !standing.iter().any(|id| id == *body) && ledger.state(body) == BodyStateType::Lost
        })
        .cloned()
        .collect();
    if !lost.is_empty() {
        lose_bodies(&mut commands, &mut record, &mut jobs, &lost);
    }
}

/// End the live encounter by its kind's rule.
///
/// - A body no cell holds, live or frozen, comes off the record first
///   ([`lose_bodies`]): the record ends with its last one, and a hail's
///   offer or held job ends with its caller.
/// - A hail whose offer stands ends when the offer's window passes
///   ([`HAIL_OFFER_SECONDS`]) or its caller is frozen with a cell that left
///   the window. The hull stays as it is.
/// - A hail whose job is held stands with the job. A held job that is done
///   and not paid is paid ([`pay_hail`]).
/// - An ambush ends when no raider stands un-neutralized within
///   [`AMBUSH_BREAK_OFF`] of the player and none is about to thaw.
/// - A derelict find ends when the player comes within [`DERELICT_FOUND`]
///   of it, or its cell left the window.
///
/// A body whose frozen cell is in the desired window is about to thaw, as
/// after a Load: the encounter waits for it. The ledger is read at most
/// every [`LEDGER_LOOK_SECONDS`]; [`drop_lost_bodies`] reads it at once for
/// a save.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the record, the jobs, the clock, the bodies, the ledger and the player's balance"
)]
pub(crate) fn end_encounters(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    designs: Res<GameEncounters>,
    clock: Res<WorldClock>,
    mut record: ResMut<Encounters>,
    mut jobs: ResMut<Jobs>,
    ledger: EncounterLedger,
    standing: Query<(&GeneratedBodyId, &GlobalTransform, Has<NeutralizedMarker>)>,
    mut player: Query<
        (
            &GlobalTransform,
            &mut ShipCredits,
            &mut ShipInventoryReports,
        ),
        With<PlayerSpaceshipMarker>,
    >,
    mut look_in: Local<f32>,
) {
    let Some(live) = record.live.clone() else {
        return;
    };
    let Ok((player_at, mut credits, mut reports)) = player.single_mut() else {
        return;
    };
    let player_at = player_at.translation();
    let Some(design) = designs.get(&live.design) else {
        warn!(
            "nova_world_base: the encounter '{}' ends: the catalog no longer holds it",
            live.design
        );
        record.end();
        return;
    };
    // A done hail job is paid before its caller is looked for: the pay
    // reads no body.
    if let EncounterType::Hail { .. } = &design.kind {
        let done = jobs
            .held()
            .iter()
            .find(|job| {
                job.id.is_hail() && live.bodies.contains(&job.id.poster) && jobs.is_done(&job.id)
            })
            .cloned();
        if let Some(job) = done {
            pay_hail(
                &mut commands,
                &mut record,
                &mut jobs,
                &mut credits,
                &mut reports,
                &job,
            );
            if record.live.is_none() {
                return;
            }
        }
    }
    *look_in -= time.delta_secs();
    let look = *look_in <= 0.0;
    let mut looked = false;
    let mut states = Vec::with_capacity(live.bodies.len());
    for body in &live.bodies {
        let state = match standing.iter().find(|(id, ..)| *id == body) {
            Some((_, at, neutralized)) => BodyStateType::Standing {
                // Engine boundary: a transform counts engine units.
                distance: Meters::from_engine(at.translation().distance(player_at)).get(),
                neutralized,
            },
            None if look => {
                looked = true;
                ledger.state(body)
            }
            // Between two looks a body that does not stand reads as about
            // to thaw: nothing ends on a guess.
            None => BodyStateType::Frozen { thawing: true },
        };
        states.push(state);
    }
    if looked {
        *look_in = LEDGER_LOOK_SECONDS;
    }
    let lost: Vec<GeneratedBodyId> = live
        .bodies
        .iter()
        .zip(&states)
        .filter(|(_, state)| **state == BodyStateType::Lost)
        .map(|(body, _)| body.clone())
        .collect();
    if !lost.is_empty() {
        lose_bodies(&mut commands, &mut record, &mut jobs, &lost);
        if record.live.is_none() {
            return;
        }
        states.retain(|state| *state != BodyStateType::Lost);
    }
    let thawing = states
        .iter()
        .any(|state| *state == BodyStateType::Frozen { thawing: true });
    let now = clock.play_seconds;
    let end = match &design.kind {
        EncounterType::Hail { .. } => {
            let caller = &live.bodies[0];
            let held = jobs
                .held()
                .iter()
                .any(|job| job.id.is_hail() && job.id.poster == *caller);
            match (record.offer.is_some(), held) {
                (true, _) if record.offer_expired(now) => Some("its offer stood unanswered"),
                (true, _) => (states[0] == BodyStateType::Frozen { thawing: false })
                    .then_some("its caller left the live window"),
                (false, true) => None,
                (false, false) => Some("its job is no longer held"),
            }
        }
        EncounterType::Ambush => {
            let fights = states.iter().any(|state| {
                matches!(
                    state,
                    BodyStateType::Standing { distance, neutralized: false }
                        if *distance <= AMBUSH_BREAK_OFF.get()
                )
            });
            (!fights && !thawing).then_some("no raider stands in reach")
        }
        EncounterType::Derelict => match states[0] {
            BodyStateType::Standing { distance, .. } => {
                (distance <= DERELICT_FOUND.get()).then_some("the player found it")
            }
            BodyStateType::Frozen { thawing: true } => None,
            BodyStateType::Frozen { thawing: false } => Some("it left the live window"),
            BodyStateType::Lost => unreachable!("a lost body is off the record above"),
        },
    };
    if let Some(why) = end {
        info!(
            "nova_world_base: the encounter '{}' ends at {now:.1} s of play: {why}",
            live.design
        );
        record.end();
    }
}

/// Keep the HUD's objective chip on the bodies of the live encounter: a
/// hailing hull wears [`HAIL_CHIP`] while its offer stands and a derelict
/// find wears [`SIGNAL_CHIP`] while it is live. A chip this system put on
/// comes off when its reason ends, before a held job marks the body. A
/// thawed or loaded body is tagged again the frame it stands, and a thawed
/// raider of a live ambush lies in wait again: a freeze keeps neither the
/// chip nor the AI state.
pub(crate) fn mark_encounter_bodies(
    mut commands: Commands,
    record: Res<Encounters>,
    designs: Res<GameEncounters>,
    mut tagged: Local<EntityHashSet>,
    added: Query<(), Added<GeneratedBodyId>>,
    bodies: Query<(
        Entity,
        &GeneratedBodyId,
        Option<&ObjectiveMarkerTarget>,
        Has<ThawedAfter>,
    )>,
) {
    if !record.is_changed() && added.is_empty() {
        return;
    }
    let kind = record
        .live
        .as_ref()
        .and_then(|live| designs.get(&live.design))
        .map(|design| &design.kind);
    let chip = |id: &GeneratedBodyId| -> Option<&'static str> {
        let live = record.live.as_ref()?;
        match kind? {
            EncounterType::Hail { .. } => record
                .offer
                .as_ref()
                .is_some_and(|offer| offer.job.id.poster == *id)
                .then_some(HAIL_CHIP),
            EncounterType::Derelict => live.bodies.contains(id).then_some(SIGNAL_CHIP),
            EncounterType::Ambush => None,
        }
    };
    for (entity, id, tag, thawed) in &bodies {
        match chip(id) {
            // A body another owner tagged keeps that tag: one chip, one
            // owner.
            Some(label) if tag.is_none() => {
                commands
                    .entity(entity)
                    .try_insert(ObjectiveMarkerTarget::new(label));
                tagged.insert(entity);
            }
            Some(_) => {}
            None if tagged.remove(&entity) => {
                let ours =
                    tag.is_some_and(|tag| tag.label == HAIL_CHIP || tag.label == SIGNAL_CHIP);
                if ours {
                    commands
                        .entity(entity)
                        .try_remove::<ObjectiveMarkerTarget>();
                }
            }
            None => {}
        }
        let waits = thawed
            && added.contains(entity)
            && matches!(kind, Some(EncounterType::Ambush))
            && record
                .live
                .as_ref()
                .is_some_and(|live| live.bodies.contains(id));
        if waits {
            commands.entity(entity).try_insert(AIBehaviorState::Idle);
        }
    }
    tagged.retain(|entity| bodies.contains(*entity));
}

#[cfg(test)]
mod tests {
    use nova_events::prelude::{
        DockingEventInfo, EventKind, OnDestroyedEvent, OnDestroyedEventInfo, OnDockedEvent,
    };
    use nova_gameplay::{
        prelude::{
            EncounterPlayerType, HailJobType, ItemDesignId, ItemRoleType, JobId, JobReward,
            SectionMarker,
        },
        test_support::test_items,
    };
    use nova_ship::prelude::DockingSectionMarker;

    use super::*;

    const SEED: u32 = 42;

    fn design(id: &str, player: EncounterPlayerType, kind: EncounterType) -> EncounterDesign {
        EncounterDesign {
            id: id.into(),
            weight: 1,
            player,
            kind,
        }
    }

    fn catalog() -> GameEncounters {
        GameEncounters::new([
            design(
                "test/hail",
                EncounterPlayerType::Any,
                EncounterType::Hail {
                    job: HailJobType::Distress,
                    line: "Mayday.".to_string(),
                },
            ),
            design(
                "test/supply",
                EncounterPlayerType::Any,
                EncounterType::Hail {
                    job: HailJobType::Supply,
                    line: "We are out of stores.".to_string(),
                },
            ),
            design(
                "test/ambush",
                EncounterPlayerType::Armed,
                EncounterType::Ambush,
            ),
            design(
                "test/find",
                EncounterPlayerType::Any,
                EncounterType::Derelict,
            ),
        ])
    }

    fn body(part: &str, index: u32) -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::ORIGIN, format!("encounter_0_{part}"), index)
    }

    fn live(design: &str, bodies: Vec<GeneratedBodyId>) -> EncounterLive {
        EncounterLive {
            design: design.into(),
            name: "Vell civilian, adrift".to_string(),
            bodies,
            started_at: 0.0,
        }
    }

    /// A record with the distress hail live and its offer standing until
    /// 300 s of play.
    fn hailed() -> (Encounters, Job) {
        hailed_with("test/hail", HailJobType::Distress)
    }

    /// A record with the hail `design` live and its offer of a `kind` job
    /// standing until 300 s of play.
    fn hailed_with(design: &str, kind: HailJobType) -> (Encounters, Job) {
        let caller = body("caller", 0);
        let job = plan_hail_offer(SEED, kind, &caller, 0, 6_000, 0.5, &test_items())
            .expect("the test items hold an item a supply job asks for");
        let record = Encounters {
            fired: 1,
            travel_seconds: 0.0,
            offer: Some(EncounterOffer {
                job: job.clone(),
                expires_at: HAIL_OFFER_SECONDS,
            }),
            live: Some(live(design, vec![caller])),
        };
        (record, job)
    }

    #[derive(Resource, Default)]
    struct Raised {
        accepted: u32,
        handed_in: u32,
    }

    /// An app over `record` with the director's answers, ends and observers,
    /// and a player hull with a docking section and 100 credits.
    fn app(record: Encounters) -> (App, Entity) {
        let mut app = App::new();
        app.insert_resource(record);
        app.insert_resource(catalog());
        app.insert_resource(test_items());
        app.init_resource::<Jobs>();
        app.init_resource::<WorldClock>();
        app.init_resource::<Time<Virtual>>();
        app.init_resource::<Raised>();
        app.add_message::<HailAnswerCommand>();
        app.add_message::<HailAnswered>();
        app.add_observer(hail_on_dock);
        app.add_observer(pay_hail_on_done);
        app.add_observer(end_on_hail_drop);
        app.add_observer(|_: On<JobAccepted>, mut raised: ResMut<Raised>| raised.accepted += 1);
        app.add_observer(|_: On<JobHandedIn>, mut raised: ResMut<Raised>| raised.handed_in += 1);
        app.add_systems(Update, (answer_hails, end_encounters).chain());
        let player = app
            .world_mut()
            .spawn((
                EntityId::new("player"),
                PlayerSpaceshipMarker,
                SpaceshipRootMarker,
                GlobalTransform::default(),
                ShipCredits(100),
                ShipInventoryReports::default(),
                ShipInventory::new(&test_items(), 1_000_000, []),
            ))
            .id();
        app.world_mut()
            .spawn((SectionMarker, DockingSectionMarker, ChildOf(player)));
        (app, player)
    }

    /// Stand the body `id` at `at`.
    fn stand(app: &mut App, id: &GeneratedBodyId, at: Meters3) -> Entity {
        app.world_mut()
            .spawn((
                id.clone(),
                GlobalTransform::from_translation(at.to_engine()),
            ))
            .id()
    }

    fn set_clock(app: &mut App, now: f64) {
        app.world_mut().resource_mut::<WorldClock>().play_seconds = now;
    }

    fn record(app: &App) -> &Encounters {
        app.world().resource::<Encounters>()
    }

    fn dock_with(app: &mut App, body: &GeneratedBodyId) {
        app.world_mut().trigger(GameEvent::new(
            OnDockedEvent::name(),
            DockingEventInfo {
                id: body.to_string(),
                type_name: "spaceship".to_string(),
                other_id: "player".to_string(),
                other_type_name: "spaceship".to_string(),
            }
            .into(),
        ));
        app.world_mut().flush();
    }

    #[test]
    fn travel_counts_only_while_every_rate_rule_holds() {
        let cruising = TravelFacts {
            docked: false,
            speed: MetersPerSecond(120.0),
            nearest_station: Some(Meters(25_000.0)),
            in_combat: false,
            neutralized: false,
            calling_rescue: false,
            encounter_live: false,
            hail_job_held: false,
        };
        assert!(travel_counts(&cruising));
        assert!(travel_counts(&TravelFacts {
            nearest_station: None,
            ..cruising
        }));
        let stopped: [(&str, TravelFacts); 8] = [
            (
                "docked",
                TravelFacts {
                    docked: true,
                    ..cruising
                },
            ),
            (
                "at 50 m/s",
                TravelFacts {
                    speed: TRAVEL_SPEED_MIN,
                    ..cruising
                },
            ),
            (
                "10 km from a station",
                TravelFacts {
                    nearest_station: Some(STATION_STANDOFF),
                    ..cruising
                },
            ),
            (
                "in combat",
                TravelFacts {
                    in_combat: true,
                    ..cruising
                },
            ),
            (
                "neutralized",
                TravelFacts {
                    neutralized: true,
                    ..cruising
                },
            ),
            (
                "calling for rescue",
                TravelFacts {
                    calling_rescue: true,
                    ..cruising
                },
            ),
            (
                "with an encounter live",
                TravelFacts {
                    encounter_live: true,
                    ..cruising
                },
            ),
            (
                "with a hail job held",
                TravelFacts {
                    hail_job_held: true,
                    ..cruising
                },
            ),
        ];
        for (what, facts) in stopped {
            assert!(!travel_counts(&facts), "travel counts {what}");
        }
    }

    #[test]
    fn a_fired_encounter_resets_the_clock_and_bumps_the_count() {
        let designs = catalog();
        let mut record = Encounters::default();
        let gap = f64::from(plan_encounter_draw(SEED, 0, &designs, true).gap_s);
        assert_eq!(
            step_travel(&mut record, gap - 1.0, SEED, &designs, true),
            TravelStepType::Wait
        );
        assert_eq!(record.fired, 0);
        let TravelStepType::Due(due) = step_travel(&mut record, 1.0, SEED, &designs, true) else {
            panic!("the gap passed with a design for the hull");
        };
        // A due encounter that waits for a place stays due and counts no
        // further.
        assert_eq!(
            step_travel(&mut record, 60.0, SEED, &designs, true),
            TravelStepType::Due(due)
        );
        assert_eq!(record.travel_seconds, gap);
        record.fire(live(due.id.as_str(), vec![body("wreck", 0)]), None);
        assert_eq!((record.fired, record.travel_seconds), (1, 0.0));
        assert!(record.live.is_some());
    }

    #[test]
    fn a_due_draw_with_no_design_for_the_hull_is_spent() {
        let designs = GameEncounters::new([design(
            "test/ambush",
            EncounterPlayerType::Armed,
            EncounterType::Ambush,
        )]);
        let mut record = Encounters::default();
        let step = step_travel(&mut record, 900.0, SEED, &designs, false);
        assert_eq!(step, TravelStepType::Spent);
        assert_eq!((record.fired, record.travel_seconds), (1, 0.0));
        assert!(record.live.is_none() && record.offer.is_none());
    }

    /// The fixture's player hull has no docking section, so no hail job
    /// fits it and the place cannot stage a hail. The find the same place
    /// stages next shows that the refusal was not for want of a place.
    #[test]
    fn a_command_its_place_cannot_stage_spends_no_draw_and_keeps_the_travel_count() {
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        crate::test_support::arm_save_fixture(&mut world);
        world.insert_resource(catalog());
        world.init_resource::<Messages<EncounterFireCommand>>();
        for coord in desired_sectors(SectorCoord::ORIGIN, crate::OPEN_WORLD_ACTIVE_RADIUS) {
            world.spawn(SectorRoot(coord));
        }
        let waiting = Encounters {
            fired: 3,
            travel_seconds: 123.0,
            ..Encounters::default()
        };
        *world.resource_mut::<Encounters>() = waiting.clone();
        let ask = |world: &mut World, design: &str| {
            world
                .resource_mut::<Messages<EncounterFireCommand>>()
                .clear();
            world.write_message(EncounterFireCommand {
                design: design.into(),
            });
            world
                .run_system_once(drive_encounters)
                .expect("the fixture holds what the director reads");
        };

        ask(&mut world, "test/hail");
        assert_eq!(*world.resource::<Encounters>(), waiting);
        let staged = world
            .query::<&GeneratedBodyId>()
            .iter(&world)
            .filter(|id| id.to_string().contains("encounter"))
            .count();
        assert_eq!(staged, 0, "a refused command puts nothing in the world");

        ask(&mut world, "test/find");
        let record = world.resource::<Encounters>();
        assert_eq!(
            record.fired, 4,
            "the find takes the draw number the hail left"
        );
        assert_eq!(
            record.live.as_ref().map(|live| live.design.as_str()),
            Some("test/find")
        );
    }

    #[test]
    fn a_place_stands_ahead_and_aside_in_a_live_cell_clear_of_bodies_and_stations() {
        let cells = |cells: &[SectorCoord]| -> BTreeMap<SectorCoord, Entity> {
            cells
                .iter()
                .map(|cell| (*cell, Entity::PLACEHOLDER))
                .collect()
        };
        let both = cells(&[SectorCoord::ORIGIN, SectorCoord::new(1, 0, 0)]);
        fn place(
            speed: f32,
            live: &BTreeMap<SectorCoord, Entity>,
            standing: &[Meters3],
            stations: &[Meters3],
        ) -> Option<Meters3> {
            plan_encounter_place(
                7,
                Meters3::ZERO,
                Vec3::X,
                MetersPerSecond(speed),
                Meters(32_000.0),
                live,
                standing,
                stations,
            )
        }
        for (speed, lead) in [(0.0, 6_000.0), (400.0, 12_000.0), (5_000.0, 40_000.0)] {
            let at = place(
                speed,
                &cells(&[SectorCoord::ORIGIN, SectorCoord::new(1, 0, 0)]),
                &[],
                &[],
            )
            .expect("open space has a place");
            assert_eq!(at.get().x, lead, "the lead at {speed} m/s");
            let aside = Vec3::new(0.0, at.get().y, at.get().z).length();
            assert!((aside - PLACE_ASIDE.get()).abs() < 1.0, "{aside} m aside");
        }

        let open = place(400.0, &both, &[], &[]).expect("open space has a place");
        let moved = place(400.0, &both, &[open], &[]).expect("seven turns stay clear");
        assert!(moved.distance(open).get() >= JOB_SPAWN_CLEARANCE.get());
        // A station on the line of flight is within 10 km of every turn.
        let station = Meters3::new(12_000.0, 0.0, 0.0);
        assert_eq!(place(400.0, &both, &[], &[station]), None);
        // 40 km out is the next cell: with only the first live, no place.
        assert_eq!(
            place(5_000.0, &cells(&[SectorCoord::ORIGIN]), &[], &[]),
            None
        );
    }

    #[test]
    fn an_unanswered_offer_ends_at_its_window_and_the_caller_stays() {
        let (record_, job) = hailed();
        let (mut app, _) = app(record_);
        let caller = stand(&mut app, &job.id.poster, Meters3::new(6_000.0, 0.0, 0.0));
        set_clock(&mut app, HAIL_OFFER_SECONDS - 1.0);
        app.update();
        assert!(
            record(&app).offer.is_some(),
            "the offer stands inside its window"
        );
        set_clock(&mut app, HAIL_OFFER_SECONDS);
        app.update();
        assert!(record(&app).offer.is_none() && record(&app).live.is_none());
        assert!(app.world().entities().contains(caller), "the hull stays");
        assert!(app.world().resource::<Jobs>().held().is_empty());
    }

    #[test]
    fn an_offer_ends_when_its_caller_no_longer_stands() {
        let (record_, _) = hailed();
        let (mut app, _) = app(record_);
        app.update();
        assert!(record(&app).live.is_none(), "no cell holds the caller");
    }

    #[test]
    fn a_declined_offer_ends_and_the_caller_stays() {
        let (record_, job) = hailed();
        let (mut app, _) = app(record_);
        let caller = stand(&mut app, &job.id.poster, Meters3::new(6_000.0, 0.0, 0.0));
        app.world_mut().write_message(HailAnswerCommand {
            answer: HailAnswerType::Decline,
        });
        app.update();
        assert!(record(&app).offer.is_none() && record(&app).live.is_none());
        assert!(app.world().entities().contains(caller));
        assert!(app.world().resource::<Jobs>().held().is_empty());
        assert_eq!(app.world().resource::<Raised>().accepted, 0);
    }

    #[test]
    fn a_hail_job_accepted_in_space_pays_on_the_dock_with_its_caller() {
        let (record_, job) = hailed();
        let (mut app, player) = app(record_);
        stand(&mut app, &job.id.poster, Meters3::new(6_000.0, 0.0, 0.0));
        app.world_mut().write_message(HailAnswerCommand {
            answer: HailAnswerType::Accept,
        });
        app.update();
        assert!(app.world().resource::<Jobs>().holds(&job.id));
        assert!(record(&app).offer.is_none(), "the offer is answered");
        assert!(
            record(&app).live.is_some(),
            "the encounter stands with its job"
        );
        assert_eq!(app.world().resource::<Raised>().accepted, 1);

        dock_with(&mut app, &job.id.poster);
        let jobs = app.world().resource::<Jobs>();
        assert!(jobs.held().is_empty() && jobs.is_completed(&job.id));
        assert_eq!(
            app.world().get::<ShipCredits>(player).unwrap().0,
            100 + job.reward.total_cr(),
            "the dock pays the reward at once"
        );
        assert_eq!(app.world().resource::<Raised>().handed_in, 1);
        assert!(
            record(&app).live.is_none(),
            "the paid job ends the encounter"
        );
    }

    #[test]
    fn a_dock_with_the_caller_accepts_the_standing_offer_then_pays_it() {
        let (record_, job) = hailed();
        let (mut app, player) = app(record_);
        stand(&mut app, &job.id.poster, Meters3::new(6_000.0, 0.0, 0.0));
        dock_with(&mut app, &job.id.poster);
        let raised = app.world().resource::<Raised>();
        assert_eq!((raised.accepted, raised.handed_in), (1, 1));
        assert!(app.world().resource::<Jobs>().is_completed(&job.id));
        assert_eq!(
            app.world().get::<ShipCredits>(player).unwrap().0,
            100 + job.reward.total_cr()
        );
        assert_eq!(
            *record(&app),
            Encounters {
                fired: 1,
                ..Encounters::default()
            }
        );
    }

    /// The board's dock observers stand beside the director's, as in the
    /// game: one dock must take the goods once and mint the reward once.
    #[test]
    fn a_supply_hail_job_accepted_in_space_takes_the_goods_and_pays_once_on_the_dock() {
        let (record_, job) = hailed_with("test/supply", HailJobType::Supply);
        let JobType::Supply { item, count, .. } = job.kind.clone() else {
            panic!("a supply hail offers a supply job, not {:?}", job.kind);
        };
        let items = test_items();
        let (mut app, player) = app(record_);
        app.add_observer(crate::jobs::supply_on_dock);
        app.add_observer(crate::jobs::distress_on_answer);
        stand(&mut app, &job.id.poster, Meters3::new(6_000.0, 0.0, 0.0));
        let aboard = |app: &App| {
            app.world()
                .get::<ShipInventory>(player)
                .unwrap()
                .count(&item)
        };
        let credits = |app: &App| app.world().get::<ShipCredits>(player).unwrap().0;
        let load = |app: &mut App, units: u32| {
            app.world_mut()
                .get_mut::<ShipInventory>(player)
                .unwrap()
                .add(&items, &item, units);
        };

        load(&mut app, count - 1);
        app.world_mut().write_message(HailAnswerCommand {
            answer: HailAnswerType::Accept,
        });
        app.update();
        assert!(app.world().resource::<Jobs>().holds(&job.id));

        dock_with(&mut app, &job.id.poster);
        app.update();
        let jobs = app.world().resource::<Jobs>();
        assert!(
            jobs.holds(&job.id) && !jobs.is_done(&job.id),
            "a dock with the hold short leaves the job open"
        );
        assert_eq!((aboard(&app), credits(&app)), (count - 1, 100));
        assert!(record(&app).live.is_some(), "the encounter stands");

        // The whole stack and two units over.
        load(&mut app, 3);
        dock_with(&mut app, &job.id.poster);
        let jobs = app.world().resource::<Jobs>();
        assert!(jobs.held().is_empty() && jobs.is_completed(&job.id));
        assert_eq!(
            aboard(&app),
            2,
            "the dock takes the asked stack and no more"
        );
        assert_eq!(
            credits(&app),
            100 + job.reward.total_cr(),
            "the dock pays the reward"
        );
        assert!(
            record(&app).live.is_none(),
            "the paid job ends the encounter"
        );

        // A later frame and a second dock pay nothing more.
        app.update();
        dock_with(&mut app, &job.id.poster);
        app.update();
        assert_eq!(aboard(&app), 2);
        assert_eq!(credits(&app), 100 + job.reward.total_cr());
        let raised = app.world().resource::<Raised>();
        assert_eq!((raised.accepted, raised.handed_in), (1, 1));
    }

    #[test]
    fn a_held_hail_job_is_dropped_as_lost_when_no_cell_holds_its_caller() {
        let (mut record_, job) = hailed();
        record_.offer = None;
        let (mut app, _) = app(record_);
        app.world_mut().resource_mut::<Jobs>().accept(job.clone());
        app.update();
        let jobs = app.world().resource::<Jobs>();
        assert!(!jobs.holds(&job.id) && jobs.is_dropped(&job.id));
        assert!(record(&app).live.is_none());
    }

    #[test]
    fn a_held_hail_job_is_dropped_and_its_encounter_ends_when_its_caller_is_destroyed() {
        let (mut record_, job) = hailed();
        record_.offer = None;
        let (mut app, _) = app(record_);
        app.add_observer(crate::jobs::drop_on_loss);
        app.world_mut().resource_mut::<Jobs>().accept(job.clone());
        app.world_mut().trigger(GameEvent::new(
            OnDestroyedEvent::name(),
            OnDestroyedEventInfo {
                id: job.id.poster.to_string(),
                type_name: "spaceship".to_string(),
            }
            .into(),
        ));
        app.world_mut().flush();
        let jobs = app.world().resource::<Jobs>();
        assert!(!jobs.holds(&job.id) && jobs.is_dropped(&job.id));
        assert!(record(&app).live.is_none());
    }

    #[test]
    fn an_ambush_ends_when_no_raider_stands_un_neutralized_within_30_km() {
        let raiders = vec![body("raider", 0), body("raider", 1)];
        let ambush = Encounters {
            fired: 1,
            live: Some(live("test/ambush", raiders.clone())),
            ..Encounters::default()
        };
        let (mut app, player) = app(ambush);
        let near = stand(&mut app, &raiders[0], Meters3::new(8_000.0, 0.0, 0.0));
        let far = stand(&mut app, &raiders[1], Meters3::new(0.0, 9_000.0, 0.0));
        app.update();
        assert!(record(&app).live.is_some(), "two raiders stand in reach");

        app.world_mut().entity_mut(near).insert(NeutralizedMarker);
        app.update();
        assert!(record(&app).live.is_some(), "one raider still fights");

        // The player runs 30 km and a metre from the last raider.
        *app.world_mut().get_mut::<GlobalTransform>(player).unwrap() =
            GlobalTransform::from_translation(Meters3::new(0.0, -21_001.0, 0.0).to_engine());
        app.update();
        assert!(record(&app).live.is_none(), "the player broke off");

        let again = Encounters {
            fired: 2,
            live: Some(live("test/ambush", raiders.clone())),
            ..Encounters::default()
        };
        *app.world_mut().resource_mut::<Encounters>() = again;
        *app.world_mut().get_mut::<GlobalTransform>(player).unwrap() = GlobalTransform::default();
        app.world_mut().entity_mut(far).insert(NeutralizedMarker);
        app.update();
        assert!(record(&app).live.is_none(), "every raider is neutralized");
    }

    #[test]
    fn a_destroyed_raider_comes_off_the_record_and_the_ambush_stands_with_the_rest() {
        let raiders = vec![body("raider", 0), body("raider", 1), body("raider", 2)];
        let ambush = Encounters {
            fired: 1,
            live: Some(live("test/ambush", raiders.clone())),
            ..Encounters::default()
        };
        let (mut app, _) = app(ambush);
        // Raider 2 was destroyed before this frame: no cell holds it.
        let first = stand(&mut app, &raiders[0], Meters3::new(8_000.0, 0.0, 0.0));
        stand(&mut app, &raiders[1], Meters3::new(0.0, 9_000.0, 0.0));
        app.update();
        assert_eq!(
            record(&app).live.as_ref().unwrap().bodies,
            raiders[..2],
            "the director's look takes the destroyed raider off"
        );

        // Destroyed between two looks: the director waits for its next
        // look, and the look a save takes does not.
        app.world_mut().entity_mut(first).despawn();
        app.update();
        assert_eq!(
            record(&app).live.as_ref().unwrap().bodies,
            raiders[..2],
            "the director's look is not due"
        );
        app.world_mut()
            .run_system_cached(drop_lost_bodies)
            .expect("the record and the jobs are resources");
        assert_eq!(
            record(&app).live.as_ref().unwrap().bodies,
            raiders[1..2],
            "the save's look takes it off at once"
        );

        app.world_mut()
            .run_system_cached(drop_lost_bodies)
            .expect("the record and the jobs are resources");
        assert_eq!(
            record(&app).live.as_ref().unwrap().bodies,
            raiders[1..2],
            "a standing raider stays on the record"
        );
    }

    #[test]
    fn the_save_s_look_ends_a_find_whose_wreck_is_destroyed_and_drops_a_lost_caller_s_job() {
        let wreck = body("wreck", 0);
        let find = Encounters {
            fired: 1,
            live: Some(live("test/find", vec![wreck])),
            ..Encounters::default()
        };
        let (mut found, _) = app(find);
        found
            .world_mut()
            .run_system_cached(drop_lost_bodies)
            .expect("the record and the jobs are resources");
        assert_eq!(
            *record(&found),
            Encounters {
                fired: 1,
                ..Encounters::default()
            },
            "the record ends with its last body"
        );

        let (mut record_, job) = hailed();
        record_.offer = None;
        let (mut called, _) = app(record_);
        called
            .world_mut()
            .resource_mut::<Jobs>()
            .accept(job.clone());
        called
            .world_mut()
            .run_system_cached(drop_lost_bodies)
            .expect("the record and the jobs are resources");
        let jobs = called.world().resource::<Jobs>();
        assert!(!jobs.holds(&job.id) && jobs.is_dropped(&job.id));
        assert!(record(&called).live.is_none());
    }

    /// The tightest case for the one chip a body wears: the find ends and a
    /// job that names its wreck is held on the same frame.
    #[test]
    fn a_job_on_the_hull_of_an_encounter_that_just_ended_takes_over_its_chip() {
        let wreck = body("wreck", 0);
        let find = Encounters {
            fired: 1,
            live: Some(live("test/find", vec![wreck.clone()])),
            ..Encounters::default()
        };
        let (mut app, player) = app(find);
        app.add_systems(
            Update,
            (mark_encounter_bodies, crate::jobs::mark_held_job_targets)
                .chain()
                .after(end_encounters),
        );
        let hull = stand(&mut app, &wreck, Meters3::new(6_000.0, 0.0, 0.0));
        let chip = |app: &App| {
            app.world()
                .get::<ObjectiveMarkerTarget>(hull)
                .map(|tag| tag.label.clone())
        };
        app.update();
        assert_eq!(chip(&app).as_deref(), Some(SIGNAL_CHIP));

        let station = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        app.world_mut().resource_mut::<Jobs>().accept(Job {
            id: JobId::slot(station.clone(), 0, 0),
            kind: JobType::Recover {
                wreck: wreck.clone(),
                wreck_name: "Vell derelict".to_string(),
                item: ItemDesignId::from(ItemRoleType::IronOre.id()),
                count: 2,
            },
            target: station,
            leg_m: 6_000,
            threat: 0,
            reward: JobReward::flat(420),
        });
        *app.world_mut().get_mut::<GlobalTransform>(player).unwrap() =
            GlobalTransform::from_translation(Meters3::new(5_100.0, 0.0, 0.0).to_engine());
        app.update();
        assert!(record(&app).live.is_none(), "the player found the wreck");
        assert_eq!(chip(&app).as_deref(), Some("JOB-1 Recover"));
    }

    #[test]
    fn a_derelict_find_ends_when_the_player_comes_within_1_km() {
        let wreck = body("wreck", 0);
        let find = Encounters {
            fired: 1,
            live: Some(live("test/find", vec![wreck.clone()])),
            ..Encounters::default()
        };
        let (mut app, player) = app(find);
        stand(&mut app, &wreck, Meters3::new(6_000.0, 0.0, 0.0));
        app.update();
        assert!(record(&app).live.is_some());
        *app.world_mut().get_mut::<GlobalTransform>(player).unwrap() =
            GlobalTransform::from_translation(Meters3::new(5_100.0, 0.0, 0.0).to_engine());
        app.update();
        assert!(record(&app).live.is_none());
    }
}
