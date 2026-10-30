//! system_job_ammo_economics: What one shipped escort job costs in ammunition, what it pays, and what the posting station charges to buy the rounds back.
//!
//! BALANCE RESEARCH, not a balance change. Boots the shipped app and plays the
//! same seed-2 `military` escort walk as `system_job_escort`
//! ([`located_jobs::walk_to_done`] then [`located_jobs::hand_in`]): the
//! player's own turrets fight the ambush, and the reward is paid through the
//! Jobs pane's Hand in button. The walk's markers and asserts run unchanged.
//! Beside the walk, this range keeps an ammunition ledger and then buys the
//! spent PDC rounds back at the posting station through the Inventory pane.
//!
//! What is MEASURED (actual state and transactions):
//! - at the accept, at the done and at the hand-in: every ammunition stack in
//!   the player's hold and every weapon magazine (`SectionAmmo`) on the hull,
//!   active or disabled;
//! - every turret round the player fires (`RoundFired` with the player as
//!   `ProjectileOwner`);
//! - the rounds in a magazine whose section left the hull, at the last frame
//!   it stood, and the rounds left in magazines of disabled sections;
//! - the credits the hand-in moved to the player and out of the station;
//! - the station's live PDC round ask, bid and stock after the hand-in, by the
//!   same `price` call the Inventory pane quotes with, and the text the pane's
//!   PDC round row shows;
//! - the Buy itself: the pane's Confirm button settles it, and both balances,
//!   both holds and the price paid are read before and after.
//!
//! What is DERIVED (arithmetic, not a transaction): the replacement cost of
//! every spent round at the live ask, and the net of reward minus that cost.
//!
//! What is FIXTURE (the shared walk's moves, no cheat): the walk teleports
//! the player beside the posting station and onto its berth before the accept
//! and again for the hand-in. The escort flight, the fight and every round
//! are real. No ammunition, credits or stock are added; nothing is set
//! unlimited.
//!
//! The Inventory row is selected with an `Activate` on the named row, as the
//! walk's Accept and Hand in are; the Confirm is a pointer click.
//!
//! | marker | data |
//! | - | - |
//! | `ammo economics: accept` | the job, its reward, the player's credits, ammo hold and magazines at the accept |
//! | `ammo economics: done` | the same snapshot when the world marks the job done or lost |
//! | `ammo economics: fight ledger` | per ammo item: start and end pool, rounds consumed, turret rounds fired, rounds lost with a section, rounds stranded in a disabled one, and the unexplained residual |
//! | `ammo economics: hand-in` | the reward, the credits the player gained and the station paid |
//! | `ammo economics: market` | the posting station's Trade service, PDC round stock, ask, bid, factors, the pane row's text, the derived replacement cost and the Buy plan |
//! | `ammo economics: buy` | the Buy's quantity, quoted ask, balances and holds before and after, and the price paid |
//! | `ammo economics: net` | reward credited, buy paid, rounds bought against rounds consumed, derived full replacement cost |
//!
//! It asserts only the transactions it reads: the Buy moved exactly the
//! quantity and paid exactly quantity times the quoted ask, and the station
//! took it. A posting station that sells no PDC round, or a fight that spent
//! none, stops the run with that limitation named.
//!
//! Headless run (needs a display; the leg is real flight, minutes on a
//! software GPU):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=2400 \
//!   cargo run --example system_job_ammo_economics --features debug
//! # look for: `ammo economics: net`, `autopilot: cycle complete, no panic`
//! ```
//! `NOVA_PROBE_TIMELINE=<file.jsonl>` records the markers. `NOVA_CAPTURE=1`
//! also writes the walk's `job-escort-board.png`,
//! `job-escort-inspector-done.png` and `job-escort-done.png`, and this range's
//! `ammo-economics-buy-draft.png` and `ammo-economics-bought.png`.

use bevy::prelude::*;
use clap::Parser;

#[cfg(feature = "debug")]
#[path = "shared/located_jobs.rs"]
pub mod located_jobs;

#[derive(Parser)]
#[command(name = "system_job_ammo_economics")]
#[command(version = "1.0.0")]
#[command(
    about = "Balance research: play the seed-2 military escort job, ledger the ammunition the \
             fight spends and the reward the hand-in pays, then buy the spent PDC rounds back at \
             the posting station through the Inventory pane. Autopilot-only",
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
        use located_jobs::{
            hand_in, walk_to_done, EscortMarkers, LocatedJobMarkers, LocatedJobWalkPlugin,
            LocatedKindType,
        };

        // The escort walk's own claims, run unchanged under this range.
        const MARKERS: LocatedJobMarkers = LocatedJobMarkers {
            accepted: "outcome: a escort job is posted and accepted",
            refused: "outcome: the hand-in is refused until the escort job is done",
            done: "outcome: the world marks the escort job done",
            paid: "outcome: the hand-in pays the escort job",
            escort: Some(EscortMarkers {
                waits: "outcome: the escort's hauler waits for the rendezvous",
                leaves: "outcome: the hauler leaves when the escort arrives",
                springs: "outcome: the raiders spring with the escort in formation",
                survives: "outcome: the escorted hauler survives the ambush to its mark",
            }),
        };

        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // No frame-time claim: the range measures ammunition and credits.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(LocatedJobWalkPlugin(LocatedKindType::Escort, MARKERS));
        app.add_plugins(economics::AmmoLedgerPlugin);
        app.add_plugins(economics::buy_back(hand_in(
            walk_to_done(LocatedKindType::Escort),
            LocatedKindType::Escort,
        )));
        app.add_systems(
            Startup,
            (
                nova_protocol::nova_debug::harness::force_capture_resolution,
                nova_protocol::nova_debug::harness::hide_dev_overlays,
            ),
        );
    }

    app.run()
}

#[cfg(feature = "debug")]
mod economics {
    use std::{collections::BTreeMap, sync::Arc};

    use bevy::prelude::*;
    use nova_protocol::{
        nova_debug::harness::{shoot, AutopilotPlugin, Predicate},
        nova_interface::pane::InterfacePaneType,
        prelude::*,
    };
    use nova_ui::widget::TextFieldValue;
    use serde_json::{json, Value};

    const INVENTORY_TAB: &str = "InterfaceTabInventory";
    const QUANTITY_FIELD: &str = "InventoryDraftQuantity";
    const CONFIRM: &str = "InventoryDraftConfirm";
    const DRAFT_SHOT: &str = "ammo-economics-buy-draft.png";
    const BOUGHT_SHOT: &str = "ammo-economics-bought.png";

    /// Seconds a click gets.
    const BEAT_SECS: f32 = 60.0;
    /// Frames a pane gets to draw.
    const SETTLE_FRAMES: u32 = 6;

    /// One weapon magazine on the player hull.
    #[derive(Clone, Debug)]
    struct Magazine {
        section: Entity,
        name: String,
        /// The item its reload draws from the hold; `None` for a magazine
        /// that never reloads.
        item: Option<ItemDesignId>,
        rounds: u32,
        capacity: u32,
        active: bool,
    }

    /// The player's credits, ammunition hold and magazines at one moment.
    #[derive(Clone, Debug)]
    struct Snapshot {
        player_cr: u32,
        hold: BTreeMap<ItemDesignId, u32>,
        magazines: Vec<Magazine>,
    }

    /// The credits on both sides of the hand-in.
    #[derive(Clone, Copy, Debug)]
    struct Balances {
        player_cr: u32,
        station: Entity,
        station_cr: u32,
    }

    /// What the Buy is about to do, read the step before Confirm.
    #[derive(Clone, Debug)]
    struct BuyPlan {
        station: Entity,
        quantity: u32,
        ask_cr: u32,
        player_cr: u32,
        station_cr: u32,
        player_rounds: u32,
        station_rounds: u32,
    }

    /// The ledger the run keeps beside the walk.
    #[derive(Resource, Debug, Default)]
    struct AmmoLedger {
        job: Option<Job>,
        /// Between the accept and the hand-in.
        armed: bool,
        accept: Option<Snapshot>,
        done: Option<Snapshot>,
        paid: Option<Snapshot>,
        /// Each magazine's rounds and active flag on the last frame it stood
        /// on the hull.
        last_seen: BTreeMap<Entity, Magazine>,
        /// Turret rounds the player fired while armed.
        fired: u32,
        /// Magazines that left the hull, at the last frame they stood.
        lost: Vec<Magazine>,
        hand_in_before: Option<Balances>,
        reward_cr: Option<u32>,
        credited_cr: Option<i64>,
        consumed_pdc: u32,
        plan: Option<BuyPlan>,
        bought_cr: Option<u32>,
    }

    /// The ledger, the round counter and the per-frame watch.
    pub struct AmmoLedgerPlugin;

    impl Plugin for AmmoLedgerPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<AmmoLedger>();
            app.add_observer(count_the_round);
            app.add_systems(Last, keep_the_ledger);
        }
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    /// Record a marker on the probe timeline and the same data in the log.
    fn record(world: &mut World, name: &str, data: Value) {
        info!("{name} {data}");
        nova_probe::probe_marker(world, name, data);
    }

    fn the_player(world: &World) -> Option<Entity> {
        let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
        let mut players = players.iter(world);
        let player = players.next()?;
        players.next().is_none().then_some(player)
    }

    fn player(world: &World) -> Entity {
        the_player(world).expect("ammo economics: exactly one player ship")
    }

    /// The hull the player is docked with.
    fn docked_partner(world: &World, player: Entity) -> Option<Entity> {
        let docked = world.get::<DockedShip>(player)?;
        let connection = world.get::<DockingConnection>(docked.connection)?;
        Some(if connection.first_ship == player {
            connection.second_ship
        } else {
            connection.first_ship
        })
    }

    fn credits(world: &World, root: Entity) -> u32 {
        world
            .get::<ShipCredits>(root)
            .unwrap_or_else(|| panic!("ammo economics: {root:?} holds no credits"))
            .0
    }

    fn held(world: &World, root: Entity, item: &ItemDesignId) -> u32 {
        world
            .get::<ShipInventory>(root)
            .unwrap_or_else(|| panic!("ammo economics: {root:?} has no hold"))
            .count(item)
    }

    fn pdc_round() -> ItemDesignId {
        ItemRoleType::PdcRound.into()
    }

    /// Count a turret round the player fired while the ledger is armed.
    fn count_the_round(
        fired: On<RoundFired>,
        owners: Query<&ProjectileOwner>,
        players: Query<(), With<PlayerSpaceshipMarker>>,
        mut ledger: ResMut<AmmoLedger>,
    ) {
        if !ledger.armed {
            return;
        }
        if owners
            .get(fired.round)
            .is_ok_and(|owner| players.contains(owner.0))
        {
            ledger.fired += 1;
        }
    }

    /// Every weapon magazine standing on `root`.
    fn magazines(world: &mut World, root: Entity) -> Vec<Magazine> {
        let mut found: Vec<Magazine> = world
            .query::<(
                Entity,
                &ChildOf,
                &SectionAmmo,
                Option<&SectionReload>,
                Option<&Name>,
                Has<SectionInactiveMarker>,
            )>()
            .iter(world)
            .filter(|(_, ChildOf(parent), ..)| *parent == root)
            .map(|(section, _, ammo, reload, name, inactive)| Magazine {
                section,
                name: name.map_or_else(|| format!("{section:?}"), |name| name.to_string()),
                item: reload.map(|reload| reload.item.clone()),
                rounds: ammo.rounds,
                capacity: ammo.capacity,
                active: !inactive,
            })
            .collect();
        found.sort_by_key(|magazine| magazine.section);
        found
    }

    fn snapshot(world: &mut World, player: Entity) -> Snapshot {
        let items = world.resource::<GameItems>().clone();
        let hold = world
            .get::<ShipInventory>(player)
            .expect("ammo economics: the player has a hold")
            .stacks()
            .filter(|(item, _)| items.design(item).category == ItemCategoryType::Ammo)
            .map(|(item, count)| (item.clone(), count))
            .collect();
        Snapshot {
            player_cr: credits(world, player),
            hold,
            magazines: magazines(world, player),
        }
    }

    fn magazine_json(magazine: &Magazine) -> Value {
        json!({
            "section": format!("{:?}", magazine.section),
            "name": magazine.name,
            "item": magazine.item.as_ref().map(ToString::to_string),
            "rounds": magazine.rounds,
            "capacity": magazine.capacity,
            "active": magazine.active,
        })
    }

    fn snapshot_json(snapshot: &Snapshot) -> Value {
        json!({
            "player_cr": snapshot.player_cr,
            "hold": snapshot
                .hold
                .iter()
                .map(|(item, count)| (item.to_string(), json!(count)))
                .collect::<serde_json::Map<_, _>>(),
            "magazines": snapshot.magazines.iter().map(magazine_json).collect::<Vec<_>>(),
        })
    }

    fn job_json(job: &Job) -> Value {
        let (hauler, cargo_cr) = match &job.kind {
            JobType::Escort { hauler, cargo_cr } => (hauler.to_string(), *cargo_cr),
            other => panic!("ammo economics: the held job is {other:?}"),
        };
        json!({
            "job": job.id.to_string(),
            "poster": job.posted_by().to_string(),
            "hauler": hauler,
            "cargo_cr": cargo_cr,
            "raiders": job.raider_ids().len(),
            "reward_cr": job.reward_cr,
        })
    }

    /// Every frame: arm at the accept, follow each magazine on the hull,
    /// snapshot at the done, and read the hand-in's credits.
    fn keep_the_ledger(world: &mut World) {
        let Some(player) = the_player(world) else {
            return;
        };
        let Some(jobs) = world.get_resource::<Jobs>().cloned() else {
            return;
        };
        let job = world.resource::<AmmoLedger>().job.clone();
        let Some(job) = job else {
            let Some(job) = jobs
                .held()
                .iter()
                .find(|job| matches!(job.kind, JobType::Escort { .. }))
                .cloned()
            else {
                return;
            };
            let accept = snapshot(world, player);
            let data = json!({ "job": job_json(&job), "snapshot": snapshot_json(&accept) });
            let mut ledger = world.resource_mut::<AmmoLedger>();
            ledger.last_seen = accept
                .magazines
                .iter()
                .map(|magazine| (magazine.section, magazine.clone()))
                .collect();
            ledger.accept = Some(accept);
            ledger.job = Some(job);
            ledger.armed = true;
            record(world, "ammo economics: accept", data);
            return;
        };
        if world.resource::<AmmoLedger>().armed {
            let standing = magazines(world, player);
            let mut ledger = world.resource_mut::<AmmoLedger>();
            let gone: Vec<Entity> = ledger
                .last_seen
                .keys()
                .copied()
                .filter(|section| !standing.iter().any(|magazine| magazine.section == *section))
                .collect();
            for section in gone {
                let magazine = ledger.last_seen.remove(&section).expect("listed");
                info!(
                    "ammo economics: magazine '{}' left the hull with {} rounds",
                    magazine.name, magazine.rounds
                );
                ledger.lost.push(magazine);
            }
            for magazine in standing {
                ledger.last_seen.insert(magazine.section, magazine);
            }
        }
        let worked = jobs.is_done(&job.id) || jobs.is_dropped(&job.id);
        if worked && world.resource::<AmmoLedger>().done.is_none() {
            let done = snapshot(world, player);
            let data = json!({
                "job": job.id.to_string(),
                "dropped": jobs.is_dropped(&job.id),
                "fired_so_far": world.resource::<AmmoLedger>().fired,
                "snapshot": snapshot_json(&done),
            });
            world.resource_mut::<AmmoLedger>().done = Some(done);
            record(world, "ammo economics: done", data);
        }
        if world.resource::<AmmoLedger>().done.is_none()
            || world.resource::<AmmoLedger>().paid.is_some()
        {
            return;
        }
        if jobs.is_completed(&job.id) {
            let paid = snapshot(world, player);
            let before = world
                .resource::<AmmoLedger>()
                .hand_in_before
                .expect("ammo economics: the player docked before the hand-in");
            let station_cr = credits(world, before.station);
            let credited = i64::from(paid.player_cr) - i64::from(before.player_cr);
            let data = json!({
                "job": job.id.to_string(),
                "reward_cr": job.reward_cr,
                "player_cr_before": before.player_cr,
                "player_cr_after": paid.player_cr,
                "credited_cr": credited,
                "station_cr_before": before.station_cr,
                "station_cr_after": station_cr,
                "station_paid_cr": i64::from(before.station_cr) - i64::from(station_cr),
            });
            let mut ledger = world.resource_mut::<AmmoLedger>();
            ledger.armed = false;
            ledger.reward_cr = Some(job.reward_cr);
            ledger.credited_cr = Some(credited);
            ledger.paid = Some(paid);
            record(world, "ammo economics: hand-in", data);
            write_the_fight_ledger(world);
        } else if let Some(station) = docked_partner(world, player) {
            let balances = Balances {
                player_cr: credits(world, player),
                station,
                station_cr: credits(world, station),
            };
            world.resource_mut::<AmmoLedger>().hand_in_before = Some(balances);
        }
    }

    /// The ammunition the job spent, per item, from the accept to the
    /// hand-in: the start pool (hold and magazines), the end pool (hold and
    /// active magazines), the rounds stranded in disabled magazines, the
    /// rounds in magazines that left the hull, the turret rounds fired, and
    /// what none of those explains.
    fn write_the_fight_ledger(world: &mut World) {
        let ledger = world.resource::<AmmoLedger>();
        let accept = ledger.accept.clone().expect("armed at the accept");
        let paid = ledger.paid.clone().expect("read at the hand-in");
        let lost = ledger.lost.clone();
        let fired = ledger.fired;
        let key = |item: &Option<ItemDesignId>| {
            item.as_ref()
                .map_or_else(|| "no-reload".to_string(), ToString::to_string)
        };
        let mut items: BTreeMap<String, [u32; 5]> = BTreeMap::new();
        // [start, end, stranded, lost, magazines at the accept]
        for (item, count) in &accept.hold {
            items.entry(item.to_string()).or_default()[0] += count;
        }
        for magazine in &accept.magazines {
            let row = items.entry(key(&magazine.item)).or_default();
            row[0] += magazine.rounds;
            row[4] += 1;
        }
        for (item, count) in &paid.hold {
            items.entry(item.to_string()).or_default()[1] += count;
        }
        for magazine in &paid.magazines {
            let row = items.entry(key(&magazine.item)).or_default();
            if magazine.active {
                row[1] += magazine.rounds;
            } else {
                row[2] += magazine.rounds;
            }
        }
        for magazine in &lost {
            items.entry(key(&magazine.item)).or_default()[3] += magazine.rounds;
        }
        let pdc = pdc_round().to_string();
        let mut consumed_pdc = 0;
        let rows: Vec<Value> = items
            .iter()
            .map(|(item, [start, end, stranded, lost, mounts])| {
                let consumed = start.saturating_sub(*end);
                let turret_fired = (*item == pdc).then_some(fired);
                if *item == pdc {
                    consumed_pdc = consumed;
                }
                let residual = turret_fired.map(|fired| {
                    i64::from(consumed) - i64::from(*stranded) - i64::from(*lost) - i64::from(fired)
                });
                json!({
                    "item": item,
                    "magazines_at_accept": mounts,
                    "start_pool": start,
                    "end_usable_pool": end,
                    "consumed": consumed,
                    "stranded_in_disabled": stranded,
                    "lost_with_section": lost,
                    "turret_rounds_fired": turret_fired,
                    "residual": residual,
                })
            })
            .collect();
        world.resource_mut::<AmmoLedger>().consumed_pdc = consumed_pdc;
        let data = json!({
            "items": rows,
            "lost_magazines": lost.iter().map(magazine_json).collect::<Vec<_>>(),
            "accept": snapshot_json(&accept),
            "hand_in": snapshot_json(&paid),
        });
        record(world, "ammo economics: fight ledger", data);
    }

    fn the_interface_shows(pane: InterfacePaneType) -> Arc<Predicate> {
        when(move |world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
                && world
                    .get_resource::<InterfacePaneType>()
                    .is_some_and(|shown| *shown == pane)
        })
    }

    fn ui_node_named(name: &'static str) -> Arc<Predicate> {
        when(move |world| {
            world
                .try_query::<&Name>()
                .is_some_and(|mut query| query.iter(world).any(|found| found.as_str() == name))
        })
    }

    /// Every text under the node named `name`.
    fn texts_under(world: &mut World, name: &str) -> Vec<String> {
        let Some(root) = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, found)| found.as_str() == name)
            .map(|(entity, _)| entity)
        else {
            return Vec::new();
        };
        let mut texts = Vec::new();
        let mut stack = vec![root];
        while let Some(entity) = stack.pop() {
            let entity = world.entity(entity);
            if let Some(text) = entity.get::<Text>() {
                texts.push(text.0.clone());
            }
            if let Some(children) = entity.get::<Children>() {
                stack.extend(children.iter());
            }
        }
        texts
    }

    fn row_name() -> String {
        format!("InventoryRowPartner{}", pdc_round())
    }

    /// The docked station's live quote for one PDC round, as the Inventory
    /// pane prices it, and its stock.
    fn quote(world: &World, station: Entity) -> (PriceQuote, u32) {
        let items = world.resource::<GameItems>();
        let design = items.design(&pdc_round());
        let stock = held(world, station, &pdc_round());
        let quote = match world.get::<TraderMarket>(station) {
            Some(market) => price(&world.resource::<GamePriceTable>().0, design, market, stock),
            None => PriceQuote::reference(design),
        };
        (quote, stock)
    }

    /// Read the market after the hand-in and plan the Buy: every spent PDC
    /// round the station sells and the player can pay for and carry.
    fn plan_the_buy(world: &mut World) {
        let player = player(world);
        let station = docked_partner(world, player)
            .expect("ammo economics: the player is docked at the posting station");
        let consumed = world.resource::<AmmoLedger>().consumed_pdc;
        let trades = station_offers(
            world.get::<StationServices>(station),
            world.get::<DerelictShipMarker>(station).is_some(),
            StationServiceType::Trade,
        );
        let (quote, stock) = quote(world, station);
        let ask = quote.price.ask_cr;
        let items = world.resource::<GameItems>().clone();
        let design = items.design(&pdc_round());
        let player_cr = credits(world, player);
        let free_g = world
            .get::<ShipInventory>(player)
            .expect("the player has a hold")
            .free_g(&items);
        let affordable = player_cr.checked_div(ask).unwrap_or(u32::MAX);
        let fits =
            u32::try_from(u64::from(free_g) / u64::from(design.mass_g.max(1))).unwrap_or(u32::MAX);
        let quantity = if trades {
            consumed.min(stock).min(affordable).min(fits)
        } else {
            0
        };
        let row = row_name();
        let row_texts = texts_under(world, &row);
        let data = json!({
            "station": world.get::<GeneratedBodyId>(station).map(ToString::to_string),
            "station_name": world.get::<Name>(station).map(ToString::to_string),
            "trades": trades,
            "has_trader_market": world.get::<TraderMarket>(station).is_some(),
            "item": pdc_round().to_string(),
            "stock": stock,
            "target_stock": world
                .get::<TraderMarket>(station)
                .and_then(|market| market.target.get(&pdc_round()).copied()),
            "ask_cr": ask,
            "bid_cr": quote.price.bid_cr,
            "reference_ask_cr": design.ask_cr,
            "reference_bid_cr": design.bid_cr,
            "factor_scarcity_ask": quote.scarcity.ask,
            "factor_cluster_ask": quote.cluster.ask,
            "factor_civilization_ask": quote.civilization.ask,
            "factor_total_ask": quote.total.ask,
            "clamped": quote.clamped,
            "pane_row": row,
            "pane_row_texts": row_texts,
            "consumed": consumed,
            "derived_replacement_cr": u64::from(consumed) * u64::from(ask),
            "player_cr": player_cr,
            "player_free_g": free_g,
            "buy_quantity": quantity,
        });
        record(world, "ammo economics: market", data);
        assert!(
            consumed > 0,
            "ammo economics: LIMITATION the fight spent no PDC round, so there is nothing to \
             buy back"
        );
        assert!(
            trades && stock > 0,
            "ammo economics: LIMITATION the posting station sells no PDC round (trades: \
             {trades}, stock: {stock})"
        );
        assert!(
            quantity > 0,
            "ammo economics: LIMITATION the player cannot pay for or carry one PDC round"
        );
        let plan = BuyPlan {
            station,
            quantity,
            ask_cr: ask,
            player_cr,
            station_cr: credits(world, station),
            player_rounds: held(world, player, &pdc_round()),
            station_rounds: stock,
        };
        world.resource_mut::<AmmoLedger>().plan = Some(plan);
    }

    fn plan(world: &World) -> BuyPlan {
        world
            .resource::<AmmoLedger>()
            .plan
            .clone()
            .expect("ammo economics: the Buy is planned")
    }

    /// The live quote and balances the step before Confirm, which the
    /// settlement is checked against.
    fn note_before_the_buy(world: &mut World) {
        let player = player(world);
        let mut plan = plan(world);
        let (quote, stock) = quote(world, plan.station);
        plan.ask_cr = quote.price.ask_cr;
        plan.player_cr = credits(world, player);
        plan.station_cr = credits(world, plan.station);
        plan.player_rounds = held(world, player, &pdc_round());
        plan.station_rounds = stock;
        world.resource_mut::<AmmoLedger>().plan = Some(plan);
    }

    fn the_buy_landed() -> Arc<Predicate> {
        when(|world| {
            let plan = plan(world);
            held(world, player(world), &pdc_round()) == plan.player_rounds + plan.quantity
        })
    }

    /// The Buy moved the quantity and paid quantity times the quoted ask.
    fn assert_the_buy(world: &mut World) {
        let player = player(world);
        let plan = plan(world);
        let player_cr = credits(world, player);
        let station_cr = credits(world, plan.station);
        let player_rounds = held(world, player, &pdc_round());
        let station_rounds = held(world, plan.station, &pdc_round());
        let paid = i64::from(plan.player_cr) - i64::from(player_cr);
        let taken = i64::from(station_cr) - i64::from(plan.station_cr);
        let expected = i64::from(plan.quantity) * i64::from(plan.ask_cr);
        let data = json!({
            "quantity": plan.quantity,
            "quoted_ask_cr": plan.ask_cr,
            "expected_cr": expected,
            "player_cr_before": plan.player_cr,
            "player_cr_after": player_cr,
            "paid_cr": paid,
            "station_cr_before": plan.station_cr,
            "station_cr_after": station_cr,
            "station_took_cr": taken,
            "player_rounds_before": plan.player_rounds,
            "player_rounds_after": player_rounds,
            "station_rounds_before": plan.station_rounds,
            "station_rounds_after": station_rounds,
        });
        record(world, "ammo economics: buy", data);
        assert_eq!(
            player_rounds,
            plan.player_rounds + plan.quantity,
            "ammo economics: the Buy put the rounds in the hold"
        );
        assert_eq!(
            station_rounds,
            plan.station_rounds - plan.quantity,
            "ammo economics: the Buy took the rounds off the station"
        );
        assert_eq!(
            paid, expected,
            "ammo economics: the player paid the quoted ask"
        );
        assert_eq!(
            taken, expected,
            "ammo economics: the station took the price"
        );
        let bought = u32::try_from(paid).expect("the price fits");
        world.resource_mut::<AmmoLedger>().bought_cr = Some(bought);
        let ledger = world.resource::<AmmoLedger>();
        let credited = ledger.credited_cr.expect("read at the hand-in");
        let reward = ledger.reward_cr.expect("read at the hand-in");
        let consumed = ledger.consumed_pdc;
        let net = json!({
            "reward_cr": reward,
            "credited_cr": credited,
            "pdc_consumed": consumed,
            "pdc_bought": plan.quantity,
            "buy_paid_cr": bought,
            "derived_full_replacement_cr": u64::from(consumed) * u64::from(plan.ask_cr),
            "net_after_buy_cr": credited - i64::from(bought),
            "replaced_every_spent_round": plan.quantity == consumed,
        });
        record(world, "ammo economics: net", net);
    }

    /// Append the buy-back to the escort walk, which ends docked at the
    /// posting station with the Jobs tab open: open the Inventory tab, read
    /// the market, select the PDC round row, type the quantity, shoot the
    /// draft, Confirm, assert the settlement and shoot the result.
    pub fn buy_back(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        script
            .step("ammo economics: the hand-in is ledgered")
            .until(when(|world| world.resource::<AmmoLedger>().paid.is_some()))
            .deadline(BEAT_SECS)
            .add()
            .click_named(
                "ammo economics: open the Inventory tab",
                INVENTORY_TAB,
                the_interface_shows(InterfacePaneType::Inventory),
                BEAT_SECS,
            )
            .step("ammo economics: settle the Inventory tab")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("ammo economics: read the market and plan the Buy")
            .on_enter(plan_the_buy)
            .add()
            .step("ammo economics: select the PDC round row")
            .on_enter(|world: &mut World| {
                let row = row_name();
                let target = world
                    .query::<(Entity, &Name)>()
                    .iter(world)
                    .find(|(_, found)| found.as_str() == row)
                    .map(|(entity, _)| entity)
                    .unwrap_or_else(|| panic!("ammo economics: no node named '{row}'"));
                world.trigger(bevy::ui_widgets::Activate { entity: target });
            })
            .until(and(ui_node_named(CONFIRM), frames(SETTLE_FRAMES)))
            .deadline(BEAT_SECS)
            .add()
            .step("ammo economics: type the quantity")
            .on_enter(|world: &mut World| {
                let quantity = plan(world).quantity.to_string();
                let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
                let (_, mut value) = fields
                    .iter_mut(world)
                    .find(|(found, _)| found.as_str() == QUANTITY_FIELD)
                    .unwrap_or_else(|| panic!("ammo economics: no field '{QUANTITY_FIELD}'"));
                value.0 = quantity;
            })
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("ammo economics: shoot the Buy draft")
            .on_enter(|world: &mut World| shoot(world, DRAFT_SHOT))
            .until(shot_written(DRAFT_SHOT))
            .deadline(BEAT_SECS)
            .add()
            .step("ammo economics: note the quote and balances")
            .on_enter(note_before_the_buy)
            .add()
            .click_named(
                "ammo economics: confirm the Buy",
                CONFIRM,
                and(the_buy_landed(), frames(SETTLE_FRAMES)),
                BEAT_SECS,
            )
            .step("ammo economics: assert the Buy")
            .on_enter(assert_the_buy)
            .add()
            .step("ammo economics: shoot the bought rounds")
            .on_enter(|world: &mut World| shoot(world, BOUGHT_SHOT))
            .until(shot_written(BOUGHT_SHOT))
            .deadline(BEAT_SECS)
            .add()
    }
}
