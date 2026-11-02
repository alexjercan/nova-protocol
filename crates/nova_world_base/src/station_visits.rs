//! Station visits: generated ships with a trade profile fly to a nearby
//! living station, dock there, trade, post their profile's jobs and leave.
//!
//! [`drive_station_visits`] is the one writer of
//! [`StationVisits`](nova_gameplay::prelude::StationVisits). Every frame it
//! walks the pending visits of live ships, then binds new ones:
//!
//! - an intact AI generated ship with a profile, no helm order, no dock and
//!   no pending visit, whose wait has passed, binds the nearest live living
//!   station within [`VISIT_RANGE`] that offers Trade or Jobs, is not the
//!   player's, admits it, has a free port and has no other visitor, and
//!   takes a Dock order to it. One visitor per station bounds the traffic by
//!   the stations the window holds;
//! - on the frame the ship stands docked at its station, the same system
//!   runs the profile's trades against the station's stock, balance and
//!   price, posts its jobs beside the station's slots up to
//!   [`VISIT_EXTRAS_MAX`] unaccepted ones, and stores the receipt. The
//!   receipt marks the visit docked, so nothing runs twice, whatever
//!   freezes, thaws or Loads come after;
//! - [`VISIT_DWELL`] of play after the capture the ship cancels its order,
//!   asks to be let go and the visit ends. Its next visit waits a seeded
//!   [`VISIT_WAIT`].
//!
//! Nothing is saved but the ledger. A thawed or loaded ship carries no helm
//! order, so the writer installs the Dock order again from the pending
//! visit: an approaching ship resumes its approach, a docked one holds its
//! restored dock until its dwell passes. Nothing happens off the window: a
//! frozen ship is not walked, and its visit waits for the thaw.
//!
//! A visit ends with a warning and no transfer when its Dock order fails
//! (no safe path, a hostile station, an overrun), when a script gives its
//! ship another order before the capture, when it has not docked
//! [`VISIT_APPROACH_MAX`] of play after the bind (frozen play time counts),
//! or when its ship or its station is neutralized, derelict or taken by the
//! player. A ship docked at a station that stops living is let go. The
//! `ondestroyed` of either body ends the visit in [`end_visits_on_loss`].

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use nova_events::prelude::{
    EventKind, GameEvent, GeneratedBodyId, Meters, Meters3, OnDestroyedEvent,
};
use nova_gameplay::prelude::{
    apply_visit_trades, station_offers, DerelictShipMarker, Fnv32, GameItems, GamePriceTable,
    GameTradeProfiles, Job, JobId, JobPostings, JobSourceType, JobType, Jobs, NeutralizedMarker,
    OwnedShipMarker, OwnedStationMarker, PlayerFleetMarker, PlayerSpaceshipMarker, SeedStream,
    ShipCredits, ShipInventory, SpaceshipRootMarker, StationMarker, StationServiceType,
    StationServices, StationVisit, StationVisitId, StationVisitPhaseType, StationVisitReceipt,
    StationVisits, TraderMarket, VisitJobType, WorldClock, VISIT_EXTRAS_MAX,
};
use nova_ship::prelude::{
    cancel_ship_order, AISpaceshipMarker, DockAdmission, DockedShip, DockingConnection,
    DockingReleaseRequest, ShipHelmOrder, ShipOrderDirective, ShipOrderHelmAuthority,
    ShipOrderReported, ShipOrderReports,
};
use nova_world::prelude::WorldConfig;

use crate::{jobs::event_subject, NovaLayeredWorld};

/// How far a ship looks for a station to visit, centre to centre.
const VISIT_RANGE: Meters = Meters(20_000.0);

/// The play seconds a visitor stays docked after its capture.
const VISIT_DWELL: f64 = 60.0;

/// The shortest and longest seeded wait, in play seconds, before a ship's
/// first visit, counted from the start of play.
const FIRST_VISIT_WAIT: (f64, f64) = (60.0, 300.0);

/// The shortest and longest seeded wait, in play seconds, between the end of
/// one visit and the next.
const VISIT_WAIT: (f64, f64) = (300.0, 600.0);

/// The helm order key a visit's Dock order is reported under.
fn order_key(visit: &StationVisitId) -> String {
    visit.to_string()
}

/// A seeded wait in `band` for `ship`'s visit number `ordinal`, in the
/// world of `seed`.
fn seeded_wait(seed: u32, ship: &GeneratedBodyId, ordinal: u32, band: (f64, f64)) -> f64 {
    let draw = SeedStream::new(
        Fnv32::new()
            .write(&seed.to_le_bytes())
            .write(b"station_visit_wait")
            .write(ship.to_string().as_bytes())
            .write(&ordinal.to_le_bytes())
            .finish(),
    )
    .unit();
    band.0 + (band.1 - band.0) * f64::from(draw)
}

/// The play seconds a visit may stay Approaching before it gives up: a Dock
/// order that waits on a busy port or a station out of the window never
/// reports, and the station stays reserved meanwhile.
const VISIT_APPROACH_MAX: f64 = 900.0;

/// One live station a ship could visit.
struct LiveStation<'w> {
    entity: Entity,
    at: Meters3,
    /// Carries its hold: a thawed station gets it when it settles.
    settled: bool,
    /// Intact, not derelict and not the player's.
    living: bool,
    trades: bool,
    posts: bool,
    market: Option<&'w TraderMarket>,
}

/// Whether a ship root, by its markers, is generated AI traffic a visit may
/// fly: AI, intact and not the player's.
fn free_traffic(
    (ai, neutralized, derelict, player, fleet, owned): (bool, bool, bool, bool, bool, bool),
) -> bool {
    ai && !(neutralized || derelict || player || fleet || owned)
}

/// End `visit`, set its ship's next stamp, and queue the cancel of the
/// visit's own Dock order and the release of its dock.
fn end_visit(
    commands: &mut Commands,
    visits: &mut StationVisits,
    seed: u32,
    now: f64,
    ship: Entity,
    visit: &StationVisit,
    cancel: bool,
    release: bool,
) {
    let next = now + seeded_wait(seed, &visit.id.ship, visit.id.ordinal + 1, VISIT_WAIT);
    visits.end(&visit.id.ship, next);
    commands.queue(move |world: &mut World| {
        if cancel {
            cancel_ship_order(world, ship);
        }
        if release {
            world.trigger(DockingReleaseRequest { entity: ship });
        }
    });
}

/// Walk every pending visit of a live ship, then bind new visits. See the
/// module docs for the rules.
#[expect(
    clippy::too_many_arguments,
    reason = "one system owning the visit ledger, both holds and both balances"
)]
#[expect(
    clippy::type_complexity,
    reason = "the ship and station queries each read their markers"
)]
pub(crate) fn drive_station_visits(
    mut commands: Commands,
    config: Res<WorldConfig<NovaLayeredWorld>>,
    clock: Res<WorldClock>,
    items: Res<GameItems>,
    table: Res<GamePriceTable>,
    profiles: Res<GameTradeProfiles>,
    jobs: Res<Jobs>,
    mut visits: ResMut<StationVisits>,
    mut postings: ResMut<JobPostings>,
    admission: DockAdmission,
    ships: Query<
        (
            (
                Entity,
                &GeneratedBodyId,
                &GlobalTransform,
                Option<&TraderMarket>,
                Has<ShipInventory>,
            ),
            (
                Option<&ShipHelmOrder>,
                Has<ShipOrderReported>,
                Option<&DockedShip>,
                Has<ShipOrderReports>,
            ),
            (
                Has<AISpaceshipMarker>,
                Has<NeutralizedMarker>,
                Has<DerelictShipMarker>,
                Has<PlayerSpaceshipMarker>,
                Has<PlayerFleetMarker>,
                Has<OwnedShipMarker>,
            ),
        ),
        (With<SpaceshipRootMarker>, Without<StationMarker>),
    >,
    stations: Query<
        (
            (
                Entity,
                &GeneratedBodyId,
                &GlobalTransform,
                Option<&StationServices>,
                Option<&TraderMarket>,
            ),
            (
                Has<ShipInventory>,
                Has<DerelictShipMarker>,
                Has<NeutralizedMarker>,
                Has<OwnedStationMarker>,
            ),
        ),
        With<StationMarker>,
    >,
    connections: Query<&DockingConnection>,
    mut holds: Query<(&mut ShipInventory, &mut ShipCredits)>,
) {
    let now = clock.play_seconds;
    let seed = config.seed;
    let live_stations: BTreeMap<&GeneratedBodyId, LiveStation> = stations
        .iter()
        .map(
            |((entity, id, at, services, market), (settled, derelict, neutralized, owned))| {
                (
                    id,
                    LiveStation {
                        entity,
                        at: Meters3::from_engine(at.translation()),
                        settled,
                        living: !(derelict || neutralized || owned),
                        trades: station_offers(services, false, StationServiceType::Trade),
                        posts: station_offers(services, false, StationServiceType::Jobs),
                        market,
                    },
                )
            },
        )
        .collect();
    let docked_at = |docked: Option<&DockedShip>, station: Entity| {
        docked.is_some_and(|docked| {
            connections
                .get(docked.connection)
                .is_ok_and(|connection| connection.joins(station))
        })
    };

    for ((ship, id, _, _, settled), (order, reported, docked, has_reports), flags) in &ships {
        let Some(visit) = visits.pending(id).cloned() else {
            continue;
        };
        let key = order_key(&visit.id);
        let ours = order.is_some_and(|order| order.key == key);
        let station = live_stations.get(&visit.station);
        let at_station = station.is_some_and(|station| docked_at(docked, station.entity));
        if !free_traffic(flags) {
            // The player took the ship, or it lost its crew: it keeps its
            // dock, and only the visit's own order goes.
            warn!(
                "nova_world_base: the visit '{}' to '{}' ends: its ship is no longer AI traffic",
                visit.id, visit.station
            );
            end_visit(
                &mut commands,
                &mut visits,
                seed,
                now,
                ship,
                &visit,
                ours,
                false,
            );
            continue;
        }
        if !settled || station.is_some_and(|station| !station.settled) {
            // A thawed hull gets its hold when it settles.
            continue;
        }
        if station.is_some_and(|station| !station.living) {
            warn!(
                "nova_world_base: the visit '{}' ends: '{}' is no longer a living station",
                visit.id, visit.station
            );
            end_visit(
                &mut commands,
                &mut visits,
                seed,
                now,
                ship,
                &visit,
                ours,
                at_station,
            );
            continue;
        }
        let install = |commands: &mut Commands, station: Entity| {
            if !has_reports {
                commands.entity(ship).insert(ShipOrderReports::default());
            }
            commands.entity(ship).insert((
                ShipHelmOrder::new(key.clone(), ShipOrderDirective::Dock { station }),
                ShipOrderHelmAuthority,
            ));
        };
        match &visit.phase {
            StationVisitPhaseType::Approaching => {
                if let (true, Some(station)) = (at_station, station) {
                    let receipt = dock_visit(
                        &visit,
                        station,
                        &items,
                        &table,
                        &profiles,
                        &jobs,
                        &mut postings,
                        now,
                        ship,
                        &mut holds,
                    );
                    info!(
                        "nova_world_base: the visit '{}' docks at '{}' at {now:.1} s of play: \
                         trades {:?}, posts {:?}",
                        visit.id, visit.station, receipt.trades, receipt.jobs
                    );
                    visits.dock(id, now, receipt);
                } else if ours && reported {
                    warn!(
                        "nova_world_base: the visit '{}' to '{}' ends with no transfer: its \
                         Dock order failed",
                        visit.id, visit.station
                    );
                    end_visit(
                        &mut commands,
                        &mut visits,
                        seed,
                        now,
                        ship,
                        &visit,
                        true,
                        false,
                    );
                } else if let (false, Some(order)) = (ours, order) {
                    // A script ordered the ship elsewhere: the visit gives way.
                    warn!(
                        "nova_world_base: the visit '{}' to '{}' ends with no transfer: the \
                         order '{}' replaced its Dock order",
                        visit.id, visit.station, order.key
                    );
                    end_visit(
                        &mut commands,
                        &mut visits,
                        seed,
                        now,
                        ship,
                        &visit,
                        false,
                        false,
                    );
                } else if now - visit.bound_at_s > VISIT_APPROACH_MAX {
                    warn!(
                        "nova_world_base: the visit '{}' to '{}' ends with no transfer: no dock \
                         {VISIT_APPROACH_MAX:.0} s after the bind",
                        visit.id, visit.station
                    );
                    end_visit(
                        &mut commands,
                        &mut visits,
                        seed,
                        now,
                        ship,
                        &visit,
                        ours,
                        false,
                    );
                } else if !ours {
                    // Just bound, thawed or loaded: the order is never saved.
                    if let Some(station) = station {
                        install(&mut commands, station.entity);
                    }
                }
            }
            StationVisitPhaseType::Docked { docked_at_s, .. } => {
                if now - docked_at_s >= VISIT_DWELL {
                    info!(
                        "nova_world_base: the visit '{}' leaves '{}' at {now:.1} s of play",
                        visit.id, visit.station
                    );
                    end_visit(
                        &mut commands,
                        &mut visits,
                        seed,
                        now,
                        ship,
                        &visit,
                        ours,
                        at_station,
                    );
                } else if let (true, None, Some(station)) = (at_station, order, station) {
                    // A restored dock after a thaw or a Load: the order holds
                    // it so the ship's own AI never flies the docked hull.
                    install(&mut commands, station.entity);
                }
            }
        }
    }

    let mut taken: BTreeSet<GeneratedBodyId> = visits
        .pending_visits()
        .map(|visit| visit.station.clone())
        .collect();
    let mut ordered: Vec<_> = ships
        .iter()
        .filter(|((.., settled), _, flags)| *settled && free_traffic(*flags))
        .collect();
    ordered.sort_by(|a, b| a.0 .1.cmp(b.0 .1));
    for ((ship, id, at, market, _), (order, _, docked, has_reports), _) in ordered {
        let Some(profile) = market.and_then(|market| market.profile.as_ref()) else {
            continue;
        };
        if order.is_some() || docked.is_some() || visits.pending(id).is_some() {
            continue;
        }
        let due = visits
            .next_visit_at(id)
            .unwrap_or_else(|| seeded_wait(seed, id, 0, FIRST_VISIT_WAIT));
        if now < due {
            continue;
        }
        let from = Meters3::from_engine(at.translation());
        let Some((station_id, station)) = live_stations
            .iter()
            .filter(|(station_id, station)| {
                station.settled
                    && station.living
                    && (station.trades || station.posts)
                    && !taken.contains(**station_id)
                    && from.distance(station.at).get() <= VISIT_RANGE.get()
                    && admission.admits(ship, station.entity)
            })
            .min_by(|a, b| {
                from.distance(a.1.at)
                    .get()
                    .total_cmp(&from.distance(b.1.at).get())
                    .then_with(|| a.0.cmp(b.0))
            })
        else {
            continue;
        };
        let visit = match visits.bind(id.clone(), (*station_id).clone(), profile.clone(), now) {
            Ok(visit) => visit,
            Err(refusal) => {
                warn!("nova_world_base: '{id}' binds no visit to '{station_id}': {refusal}");
                continue;
            }
        };
        taken.insert((*station_id).clone());
        if !has_reports {
            commands.entity(ship).insert(ShipOrderReports::default());
        }
        commands.entity(ship).insert((
            ShipHelmOrder::new(
                order_key(&visit),
                ShipOrderDirective::Dock {
                    station: station.entity,
                },
            ),
            ShipOrderHelmAuthority,
        ));
        info!(
            "nova_world_base: '{id}' binds the visit '{visit}' to '{station_id}' under '{profile}' \
             at {now:.1} s of play, {:.0} m off",
            from.distance(station.at).get()
        );
    }
}

/// Run `visit`'s profile at `station` on the frame its ship stands docked
/// there: its trades, when the station offers Trade, and its jobs, when it
/// offers Jobs, each in list order, and give back the receipt.
///
/// # Panics
///
/// When the profile is no longer in the catalog, or the ship or the station
/// has no hold or balance: a Load checks the profile, and a docked station
/// and a settled ship carry both.
#[expect(
    clippy::too_many_arguments,
    reason = "the visit, both holds, the catalogs and the postings, changed in one frame"
)]
fn dock_visit(
    visit: &StationVisit,
    station: &LiveStation,
    items: &GameItems,
    table: &GamePriceTable,
    profiles: &GameTradeProfiles,
    jobs: &Jobs,
    postings: &mut JobPostings,
    now: f64,
    ship: Entity,
    holds: &mut Query<(&mut ShipInventory, &mut ShipCredits)>,
) -> StationVisitReceipt {
    let profile = profiles.get(&visit.profile).unwrap_or_else(|| {
        panic!(
            "the visit '{}' runs the profile '{}', which the catalog lacks",
            visit.id, visit.profile
        )
    });
    let mut receipt = StationVisitReceipt::default();
    if station.trades && !profile.trades.is_empty() {
        let market = station.market.unwrap_or_else(|| {
            panic!(
                "the trading station '{}' prices by no market",
                visit.station
            )
        });
        let Ok([(mut ship_hold, mut ship_cr), (mut station_hold, mut station_cr)]) =
            holds.get_many_mut([ship, station.entity])
        else {
            panic!(
                "the visit '{}' finds no hold and balance on its ship or its station",
                visit.id
            );
        };
        receipt.trades = apply_visit_trades(
            items,
            &table.0,
            market,
            &profile.trades,
            &mut ship_hold,
            &mut ship_cr.0,
            &mut station_hold,
            &mut station_cr.0,
        );
    }
    if station.posts {
        for (index, template) in profile.jobs.iter().enumerate() {
            if postings.open_visit_postings(&visit.station, jobs) >= VISIT_EXTRAS_MAX {
                info!(
                    "nova_world_base: the visit '{}' skips {} job(s): '{}' holds {VISIT_EXTRAS_MAX} \
                     open visit postings",
                    visit.id,
                    profile.jobs.len() - index,
                    visit.station
                );
                break;
            }
            let VisitJobType::Procure {
                item,
                count,
                reward_cr,
            } = template;
            let job = Job {
                id: JobId {
                    station: visit.station.clone(),
                    source: JobSourceType::Visit {
                        visit: visit.id.clone(),
                        index: index as u32,
                    },
                },
                kind: JobType::Procure {
                    item: item.clone(),
                    count: *count,
                },
                target: visit.station.clone(),
                reward_cr: *reward_cr,
            };
            receipt.jobs.push(job.id.clone());
            postings.post_visit(job, now);
        }
    }
    receipt
}

/// End the pending visit of a ship, and every pending visit to a station,
/// on the `ondestroyed` of that body: the entity is gone, so the writer
/// would wait on it forever. A visit already docked keeps what it traded
/// and posted. A live ship still flying to a lost station has its visit's
/// Dock order cancelled. A neutralized body stays, and the writer ends its
/// visits by its marker.
pub(crate) fn end_visits_on_loss(
    event: On<GameEvent>,
    mut commands: Commands,
    config: Option<Res<WorldConfig<NovaLayeredWorld>>>,
    clock: Res<WorldClock>,
    mut visits: ResMut<StationVisits>,
    ships: Query<(Entity, &GeneratedBodyId, &ShipHelmOrder)>,
) {
    // Without a world no visit was ever bound.
    let Some(config) = config else {
        return;
    };
    if event.name() != OnDestroyedEvent::name() {
        return;
    }
    let Some(subject) = event_subject(&event) else {
        return;
    };
    let lost: Vec<StationVisit> = visits
        .pending_visits()
        .filter(|visit| {
            visit.id.ship.to_string() == subject || visit.station.to_string() == subject
        })
        .cloned()
        .collect();
    for visit in lost {
        let next = clock.play_seconds
            + seeded_wait(
                config.seed,
                &visit.id.ship,
                visit.id.ordinal + 1,
                VISIT_WAIT,
            );
        visits.end(&visit.id.ship, next);
        warn!(
            "nova_world_base: the visit '{}' to '{}' ends: '{subject}' was destroyed",
            visit.id, visit.station
        );
        let key = order_key(&visit.id);
        if let Some((ship, ..)) = ships
            .iter()
            .find(|(_, body, order)| **body == visit.id.ship && order.key == key)
        {
            commands.queue(move |world: &mut World| cancel_ship_order(world, ship));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use nova_events::prelude::{OnDestroyedEventInfo, SectorCoord};
    use nova_gameplay::prelude::{
        Allegiance, CivilizationId, ClusterType, ItemDesignId, ItemRoleType, ItemTradeType,
        JobOfferRule, PriceClamp, PriceFactor, PriceTable, RestockRule, ScarcityRule, StationVisit,
        TradeIntent, TradeProfileDesign, TradeProfileDesignId,
    };
    use nova_ship::prelude::{DockedHelmType, DockingSectionMarker};

    use super::*;

    const SEED: u32 = 42;
    const START_CR: u32 = 10_000;

    fn ore() -> ItemDesignId {
        ItemDesignId::from(ItemRoleType::IronOre)
    }

    fn rations() -> ItemDesignId {
        ItemDesignId::from(ItemRoleType::Rations)
    }

    fn procure(count: u32) -> VisitJobType {
        VisitJobType::Procure {
            item: ore(),
            count,
            reward_cr: 50,
        }
    }

    /// A profile that sells 6 ore and buys 2 rations, and posts `jobs`.
    fn trading_profile(jobs: usize) -> TradeProfileDesign {
        let items = nova_gameplay::test_support::test_items();
        TradeProfileDesign {
            id: TradeProfileDesignId::from("test/visitor"),
            generated: Vec::new(),
            trades: vec![
                TradeIntent {
                    item: ore(),
                    side: ItemTradeType::Sell,
                    quantity: 6,
                    limit_cr: 1,
                },
                TradeIntent {
                    item: rations(),
                    side: ItemTradeType::Buy,
                    quantity: 2,
                    limit_cr: items.design(&rations()).ask_cr * 3,
                },
            ],
            jobs: (0..jobs).map(|index| procure(index as u32 + 1)).collect(),
        }
    }

    fn empty_profile() -> TradeProfileDesign {
        TradeProfileDesign {
            trades: Vec::new(),
            jobs: Vec::new(),
            ..trading_profile(0)
        }
    }

    fn market(profile: Option<TradeProfileDesignId>) -> TraderMarket {
        TraderMarket {
            cluster: ClusterType::RockOnly,
            civilization: CivilizationId {
                world_seed: 1,
                node: [0, 0, 0],
            },
            role_preference: [1.0, 0.0, 0.0, 0.0],
            target: BTreeMap::new(),
            profile,
        }
    }

    fn flat_price_table() -> GamePriceTable {
        let one = PriceFactor::ONE;
        GamePriceTable(PriceTable {
            clamp: PriceClamp {
                min: 0.25,
                max: 3.0,
            },
            scarcity: ScarcityRule {
                starved: one,
                glut: one,
                glut_share: 3.0,
                unkept: one,
            },
            restock: RestockRule {
                recovery_seconds: 1_000.0,
            },
            jobs: JobOfferRule {
                offer_seconds: 1_200.0,
                escort_offer_seconds: 3_600.0,
            },
            clusters: Vec::new(),
            roles: Vec::new(),
        })
    }

    /// A visitor and a station that admits it, 1 km apart, in an app that
    /// runs only the visit writer and its loss observer. The test drives
    /// `WorldClock` by hand.
    struct Rig {
        app: App,
        ship: Entity,
        station: Entity,
        ship_id: GeneratedBodyId,
        station_id: GeneratedBodyId,
        profile: TradeProfileDesign,
    }

    impl Rig {
        fn new(profile: TradeProfileDesign) -> Self {
            let items = nova_gameplay::test_support::test_items();
            let ship_id = GeneratedBodyId::new(SectorCoord::new(0, 0, 0), "visitor", 0);
            let station_id = GeneratedBodyId::new(SectorCoord::new(0, 0, 0), "station", 0);
            let mut app = App::new();
            app.insert_resource(WorldConfig {
                seed: SEED,
                sector_edge: crate::OPEN_WORLD_SECTOR_EDGE,
                active_radius: crate::OPEN_WORLD_ACTIVE_RADIUS,
                generator: crate::tests::fixture_world(),
            })
            .insert_resource(WorldClock::default())
            .insert_resource(items.clone())
            .insert_resource(flat_price_table())
            .insert_resource(GameTradeProfiles(BTreeMap::from([(
                profile.id.clone(),
                profile.clone(),
            )])))
            .init_resource::<Jobs>()
            .init_resource::<JobPostings>()
            .init_resource::<StationVisits>()
            .add_systems(Update, drive_station_visits)
            .add_observer(end_visits_on_loss);
            let station = app
                .world_mut()
                .spawn((
                    SpaceshipRootMarker,
                    StationMarker,
                    station_id.clone(),
                    GlobalTransform::from_xyz(100.0, 0.0, 0.0),
                    Allegiance::Neutral,
                    StationServices(vec![StationServiceType::Trade, StationServiceType::Jobs]),
                    market(None),
                    ShipInventory::new(&items, 1_000_000, [(rations(), 3)]),
                    ShipCredits(START_CR),
                ))
                .id();
            app.world_mut()
                .spawn((DockingSectionMarker, ChildOf(station)));
            let mut rig = Self {
                app,
                ship: Entity::PLACEHOLDER,
                station,
                ship_id,
                station_id,
                profile,
            };
            rig.ship = rig.spawn_ship(
                ShipInventory::new(&items, 1_000_000, [(ore(), 10)]),
                START_CR,
            );
            rig
        }

        fn spawn_ship(&mut self, hold: ShipInventory, credits: u32) -> Entity {
            self.app
                .world_mut()
                .spawn((
                    SpaceshipRootMarker,
                    AISpaceshipMarker,
                    self.ship_id.clone(),
                    GlobalTransform::default(),
                    Allegiance::Neutral,
                    market(Some(self.profile.id.clone())),
                    hold,
                    ShipCredits(credits),
                ))
                .id()
        }

        fn at(&mut self, play_seconds: f64) {
            self.app
                .world_mut()
                .resource_mut::<WorldClock>()
                .play_seconds = play_seconds;
            self.app.update();
        }

        /// The play second the ship's first visit may bind at, rounded up.
        fn first_due(&self) -> f64 {
            seeded_wait(SEED, &self.ship_id, 0, FIRST_VISIT_WAIT).ceil() + 1.0
        }

        fn visit(&self) -> Option<StationVisit> {
            self.app
                .world()
                .resource::<StationVisits>()
                .pending(&self.ship_id)
                .cloned()
        }

        fn order(&self) -> Option<(String, ShipOrderDirective)> {
            self.app
                .world()
                .get::<ShipHelmOrder>(self.ship)
                .map(|order| (order.key.clone(), order.directive.clone()))
        }

        /// Wait out the first-visit stamp and bind: the ship is Approaching.
        fn bind(&mut self) -> (StationVisit, f64) {
            self.at(0.0);
            assert!(self.visit().is_none(), "no visit binds before its stamp");
            let bound_at = self.first_due();
            self.at(bound_at);
            let visit = self.visit().expect("the ship binds its visit");
            assert_eq!(visit.phase, StationVisitPhaseType::Approaching);
            assert_eq!(visit.station, self.station_id);
            assert_eq!(
                self.order(),
                Some((
                    visit.id.to_string(),
                    ShipOrderDirective::Dock {
                        station: self.station
                    }
                )),
                "the Dock order is keyed by the visit id"
            );
            (visit, bound_at)
        }

        /// Join `ship` and the station in a connection and mark the ship
        /// docked, as a capture does.
        fn capture(&mut self, ship: Entity) -> Entity {
            let port = self
                .app
                .world_mut()
                .query_filtered::<Entity, With<DockingSectionMarker>>()
                .single(self.app.world())
                .expect("the station has one port");
            let connection = self
                .app
                .world_mut()
                .spawn(DockingConnection {
                    first_ship: ship,
                    first_section: port,
                    second_ship: self.station,
                    second_section: port,
                    helm: DockedHelmType::Station(self.station),
                    measurement_fault: false,
                })
                .id();
            self.app.world_mut().entity_mut(ship).insert(DockedShip {
                connection,
                helm: Quat::IDENTITY,
                drives: false,
            });
            connection
        }

        /// Both holds, both balances and the postings, for a before/after
        /// comparison.
        fn ledger(&self) -> (ShipInventory, u32, ShipInventory, u32, JobPostings) {
            let world = self.app.world();
            let ship = world.get::<ShipInventory>(self.ship).unwrap().clone();
            let station = world.get::<ShipInventory>(self.station).unwrap().clone();
            (
                ship,
                world.get::<ShipCredits>(self.ship).unwrap().0,
                station,
                world.get::<ShipCredits>(self.station).unwrap().0,
                world.resource::<JobPostings>().clone(),
            )
        }

        fn visit_job_ids(&self, visit: &StationVisitId, count: u32) -> Vec<JobId> {
            (0..count)
                .map(|index| JobId {
                    station: self.station_id.clone(),
                    source: JobSourceType::Visit {
                        visit: visit.clone(),
                        index,
                    },
                })
                .collect()
        }

        fn fire(&mut self, event: GameEvent) {
            self.app.world_mut().trigger(event);
            self.app.world_mut().flush();
        }
    }

    fn docked_receipt(visit: &StationVisit) -> (f64, StationVisitReceipt) {
        match &visit.phase {
            StationVisitPhaseType::Docked {
                docked_at_s,
                receipt,
            } => (*docked_at_s, receipt.clone()),
            other => panic!("the visit is not docked: {other:?}"),
        }
    }

    /// The visit ends: no pending visit, the next one 300-600 s after `now`,
    /// and the Dock order gone.
    fn assert_visit_ended(rig: &Rig, now: f64) {
        assert!(rig.visit().is_none(), "the visit ended");
        let next = rig
            .app
            .world()
            .resource::<StationVisits>()
            .next_visit_at(&rig.ship_id)
            .expect("an ended visit stamps the next one");
        assert!(
            (now + VISIT_WAIT.0..=now + VISIT_WAIT.1).contains(&next),
            "next visit at {next} s after ending at {now} s"
        );
        assert_eq!(rig.order(), None, "the order is cancelled");
    }

    #[test]
    fn a_docked_visit_trades_and_posts_once_and_a_thaw_or_load_never_repeats_it() {
        let mut rig = Rig::new(trading_profile(2));
        let (visit, bound_at) = rig.bind();
        let before = rig.ledger();

        let docked_at = bound_at + 10.0;
        rig.capture(rig.ship);
        rig.at(docked_at);

        let (stamp, receipt) = docked_receipt(&rig.visit().expect("the visit stands"));
        assert_eq!(stamp, docked_at);
        let [sold, bought] = receipt.trades.as_slice() else {
            panic!("two trade lines expected: {:?}", receipt.trades);
        };
        assert_eq!(
            (sold.item.clone(), sold.side, sold.count),
            (ore(), ItemTradeType::Sell, 6)
        );
        assert_eq!(
            (bought.item.clone(), bought.side, bought.count),
            (rations(), ItemTradeType::Buy, 2)
        );
        assert!(sold.price_cr > 0 && bought.price_cr > 0);
        let want_jobs = rig.visit_job_ids(&visit.id, 2);
        assert_eq!(receipt.jobs, want_jobs);

        let (ship_hold, ship_cr, station_hold, station_cr, postings) = rig.ledger();
        assert_eq!(
            (ship_hold.count(&ore()), station_hold.count(&ore())),
            (4, 6)
        );
        assert_eq!(
            (ship_hold.count(&rations()), station_hold.count(&rations())),
            (2, 1)
        );
        assert_eq!(ship_cr, START_CR + sold.price_cr - bought.price_cr);
        assert_eq!(station_cr, START_CR - sold.price_cr + bought.price_cr);
        assert_eq!(
            ship_cr + station_cr,
            before.1 + before.3,
            "credits conserved"
        );
        for item in [ore(), rations()] {
            assert_eq!(
                ship_hold.count(&item) + station_hold.count(&item),
                before.0.count(&item) + before.2.count(&item),
                "{item} conserved"
            );
        }
        let posted: Vec<JobId> = postings
            .visit_postings(&rig.station_id)
            .iter()
            .map(|posting| posting.offer.id.clone())
            .collect();
        assert_eq!(posted, want_jobs);
        let settled = rig.ledger();
        let key = visit.id.to_string();

        // A thaw: the frozen ship comes back as a fresh entity with its hold,
        // its dock and no helm order.
        let (hold, credits) = (settled.0.clone(), settled.1);
        let old_connection = rig
            .app
            .world()
            .get::<DockedShip>(rig.ship)
            .unwrap()
            .connection;
        rig.app.world_mut().despawn(old_connection);
        rig.app.world_mut().despawn(rig.ship);
        rig.ship = rig.spawn_ship(hold, credits);
        rig.capture(rig.ship);
        assert_eq!(rig.order(), None);
        rig.at(docked_at + 5.0);
        assert_eq!(rig.ledger(), settled, "a thaw trades and posts nothing");
        assert_eq!(
            rig.order(),
            Some((
                key.clone(),
                ShipOrderDirective::Dock {
                    station: rig.station
                }
            )),
            "the writer installs the Dock order again"
        );
        assert_eq!(docked_receipt(&rig.visit().unwrap()).1, receipt);

        // A Load: the ledger round-trips through its save form and the
        // restored ship carries no order.
        let visits_ron = ron::to_string(rig.app.world().resource::<StationVisits>()).unwrap();
        let postings_ron = ron::to_string(rig.app.world().resource::<JobPostings>()).unwrap();
        rig.app
            .world_mut()
            .insert_resource(ron::from_str::<StationVisits>(&visits_ron).unwrap());
        rig.app
            .world_mut()
            .insert_resource(ron::from_str::<JobPostings>(&postings_ron).unwrap());
        rig.app
            .world_mut()
            .entity_mut(rig.ship)
            .remove::<(ShipHelmOrder, ShipOrderHelmAuthority)>();
        rig.at(docked_at + 10.0);
        assert_eq!(rig.ledger(), settled, "a Load trades and posts nothing");
        assert_eq!(docked_receipt(&rig.visit().unwrap()).1, receipt);
        assert_eq!(rig.order().map(|order| order.0), Some(key));

        // The dwell passes: the visit ends and the order goes.
        let leaves_at = docked_at + VISIT_DWELL;
        rig.at(leaves_at);
        assert_visit_ended(&rig, leaves_at);
        assert_eq!(rig.ledger(), settled, "leaving moves nothing");
    }

    #[test]
    fn an_empty_profile_visit_docks_and_leaves_without_moving_anything() {
        let mut rig = Rig::new(empty_profile());
        let (_, bound_at) = rig.bind();
        let before = rig.ledger();

        let docked_at = bound_at + 10.0;
        rig.capture(rig.ship);
        rig.at(docked_at);

        let (stamp, receipt) = docked_receipt(&rig.visit().expect("the visit stands"));
        assert_eq!(stamp, docked_at);
        assert_eq!(receipt, StationVisitReceipt::default());
        assert_eq!(rig.ledger(), before, "nothing moved");
        assert!(rig.app.world().resource::<JobPostings>().is_empty());

        let leaves_at = docked_at + VISIT_DWELL;
        rig.at(leaves_at);
        assert_visit_ended(&rig, leaves_at);
        assert_eq!(rig.ledger(), before, "leaving moves nothing");
    }

    #[test]
    fn a_station_holds_at_most_three_open_visit_jobs_and_skips_the_rest() {
        let mut rig = Rig::new(trading_profile(5));
        let (visit, bound_at) = rig.bind();

        rig.capture(rig.ship);
        rig.at(bound_at + 10.0);

        let (_, receipt) = docked_receipt(&rig.visit().expect("the visit stands"));
        assert_eq!(VISIT_EXTRAS_MAX, 3);
        assert_eq!(
            receipt.jobs,
            rig.visit_job_ids(&visit.id, 3),
            "indices 0..3"
        );
        let postings = rig.app.world().resource::<JobPostings>();
        assert_eq!(postings.visit_postings(&rig.station_id).len(), 3);
        assert_eq!(
            postings.open_visit_postings(&rig.station_id, &Jobs::default()),
            VISIT_EXTRAS_MAX
        );
        for skipped in rig.visit_job_ids(&visit.id, 5).split_off(3) {
            assert!(
                postings.visit_posting(&skipped).is_none(),
                "{skipped} skipped"
            );
        }
    }

    /// The Release requests the writer queued, by ship.
    #[derive(Resource, Default)]
    struct Releases(Vec<Entity>);

    #[test]
    fn a_lost_station_or_a_stalled_approach_ends_the_visit_without_a_transfer() {
        // Destroyed: the observer ends it on the event.
        let mut rig = Rig::new(trading_profile(2));
        let (_, bound_at) = rig.bind();
        let before = rig.ledger();
        let station_id = rig.station_id.clone();
        rig.fire(GameEvent::new(
            OnDestroyedEvent::name(),
            OnDestroyedEventInfo {
                id: station_id.to_string(),
                type_name: "spaceship".to_string(),
            }
            .into(),
        ));
        assert_visit_ended(&rig, bound_at);
        assert_eq!(rig.ledger(), before, "no transfer, no posting");

        // Neutralized while approaching: the writer ends it by the marker.
        let mut rig = Rig::new(trading_profile(2));
        let (_, bound_at) = rig.bind();
        let before = rig.ledger();
        rig.app
            .world_mut()
            .entity_mut(rig.station)
            .insert(NeutralizedMarker);
        rig.at(bound_at + 1.0);
        assert_visit_ended(&rig, bound_at + 1.0);
        assert_eq!(rig.ledger(), before, "no transfer, no posting");

        // An approach that never docks gives the station back.
        let mut rig = Rig::new(trading_profile(2));
        let (_, bound_at) = rig.bind();
        rig.at(bound_at + VISIT_APPROACH_MAX);
        assert!(
            rig.visit().is_some(),
            "the approach still stands at the limit"
        );
        let ended_at = bound_at + VISIT_APPROACH_MAX + 1.0;
        rig.at(ended_at);
        assert_visit_ended(&rig, ended_at);
    }

    /// A visitor docked at a station that stops living leaves at once: the
    /// writer would otherwise never end its visit, and its dock would bar
    /// every later visit of that ship.
    #[test]
    fn a_docked_visitor_is_let_go_when_its_station_stops_living() {
        let mut rig = Rig::new(trading_profile(0));
        rig.app.init_resource::<Releases>().add_observer(
            |release: On<DockingReleaseRequest>, mut releases: ResMut<Releases>| {
                releases.0.push(release.entity);
            },
        );
        let (_, bound_at) = rig.bind();
        let ship = rig.ship;
        rig.capture(ship);
        rig.at(bound_at + 1.0);
        assert!(matches!(
            rig.visit().map(|visit| visit.phase),
            Some(StationVisitPhaseType::Docked { .. })
        ));
        rig.app
            .world_mut()
            .entity_mut(rig.station)
            .insert(OwnedStationMarker);
        rig.at(bound_at + 2.0);
        assert!(rig.visit().is_none(), "the visit ends");
        assert_eq!(
            rig.app.world().resource::<Releases>().0,
            vec![ship],
            "the docked visitor asks to be let go"
        );
        assert!(rig.order().is_none(), "the visit's order is cancelled");
    }
}
