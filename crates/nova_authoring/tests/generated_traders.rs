//! Generated traders from the real base catalog: a ship the world's own
//! `plan_ship` lays out, stocks and prices for a living station's
//! civilization sells the goods of its base trade profile to that
//! civilization's generated station, by the base price table.
//!
//! The sale is `apply_visit_trades`, the rule the visit writer runs on the
//! frame a visitor stands docked. The bind, the dock and the lawful refusal
//! are the writer's and are proven on fixture items in
//! `nova_world_base::station_visits`.

use std::collections::BTreeMap;

use nova_authoring::{
    generation::{build_items, build_price_table, build_section_catalog, build_trade_profiles},
    lint_walk::repo_ship_part_packs,
};
use nova_events::prelude::Meters3;
use nova_gameplay::prelude::{
    apply_visit_trades, CivilizationId, ClusterType, GameItems, GameTradeProfiles, ItemDesignId,
    ItemTradeType, ShipInventory, ShipInventoryStock, ShipRoleType, StanceType, TraderMarket,
};
use nova_scenario::prelude::{resolve_ship_design, GameShipDesigns, ShipDesign, ShipDesignSource};
use nova_ship::prelude::GameSections;
use nova_world_base::prelude::{
    generate_station, generated_station, plan_ship, Civilization, CivilizationField,
    CivilizationStatusType, Environment, HullLineageType, HullSlot, ShipPartSnapshot,
    StationArchetypeType, StationLayoutRequest,
};

/// The world seed every draw below keys on.
const SEED: u32 = 5;

/// The cluster the station and its traffic price by.
const CLUSTER: ClusterType = ClusterType::RockOnly;

/// How many traffic slots the search plans for a hull of the role that
/// carries the good. A civilian mixes medkits at 1 part in 11 over up to
/// three stacks; half the intact scavengers carry a core.
const SLOTS: usize = 64;

/// The living civilization whose station the traffic visits. Not lawful: a
/// lawful station binds no visit from a contraband carrier.
fn civilization() -> Civilization {
    Civilization {
        id: CivilizationId {
            world_seed: SEED,
            node: [0, 0, 0],
        },
        centroid: Meters3::ZERO,
        status: CivilizationStatusType::Living,
        advancement: 0.5,
        role_preference: [1.0; 4],
        stance: StanceType::Neutral,
        lawful: false,
    }
}

/// A hold of `stock` in the hull `design`, sized by its hull sections as the
/// spawn sizes it.
fn hold(
    items: &GameItems,
    sections: &GameSections,
    design: &ShipDesign,
    stock: &ShipInventoryStock,
) -> ShipInventory {
    let (resolved, errors) = resolve_ship_design(
        &ShipDesignSource::Inline(design.clone()),
        &GameShipDesigns::default(),
        sections,
    );
    assert!(errors.is_empty(), "the generated hull resolves: {errors:?}");
    ShipInventory::new(
        items,
        resolved.cargo_capacity_g(),
        stock.stacks().map(|(item, count)| (item.clone(), count)),
    )
}

/// The first generated hull of `role` in the station's traffic that carries
/// `good`, on the base profile `profile`, sells it to the civilization's
/// generated station. Gives back how many units the station stocked before
/// the visit and how many the visit sold.
///
/// # Panics
///
/// When no hull of [`SLOTS`] carries the good, the profile is not one the
/// role draws, or the sale moves nothing or breaks the ledger.
fn sell_at_a_generated_station(role: ShipRoleType, profile: &str, good: &str) -> (u32, u32) {
    let parts = ShipPartSnapshot::build(&repo_ship_part_packs(&["base"]))
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"));
    let sections = GameSections(build_section_catalog());
    let items = GameItems::new(build_items());
    let table = build_price_table();
    let profiles = GameTradeProfiles(
        build_trade_profiles()
            .into_iter()
            .map(|profile| (profile.id.clone(), profile))
            .collect(),
    );
    let good = ItemDesignId::from(good);
    let civilization = civilization();

    let profile = profiles
        .0
        .values()
        .find(|design| design.id.as_str() == profile)
        .unwrap_or_else(|| panic!("the base catalog ships '{profile}'"));
    assert!(
        (0..8).any(|draw| {
            profiles
                .draw(role, draw as f32 / 8.0)
                .is_some_and(|drawn| drawn.id == profile.id)
        }),
        "a generated {role:?} draws '{}'",
        profile.id
    );
    let intent = profile
        .trades
        .iter()
        .find(|intent| intent.item == good && intent.side == ItemTradeType::Sell)
        .unwrap_or_else(|| panic!("'{}' sells {good}", profile.id));

    let trader = (0..SLOTS)
        .filter_map(|slot| {
            let id = format!("traffic_{slot}");
            plan_ship(
                &parts,
                &items,
                &CivilizationField::new(SEED),
                SEED,
                HullSlot {
                    id: &id,
                    node: civilization.id.node,
                    cluster: CLUSTER,
                    slot,
                    anchor: Meters3::ZERO,
                    environment: Environment {
                        material_density: 0.5,
                        volatiles: 0.5,
                        human_activity: 0.5,
                    },
                    planetoids: &[],
                    position: Meters3::ZERO,
                    yaw: 0.0,
                    lineage: HullLineageType::Station(civilization),
                },
            )
            .ok()
        })
        .find(|ship| {
            ship.role == role && ship.spawn.inventory.stacks().any(|(item, _)| *item == good)
        })
        .unwrap_or_else(|| panic!("no {role:?} of {SLOTS} traffic slots carries {good}"));
    assert!(
        trader.market.is_some(),
        "an intact generated hull prices by a market"
    );
    let ShipDesignSource::Inline(hull) = &trader.spawn.design else {
        panic!("a generated hull spawns its design inline");
    };
    let mut ship_hold = hold(&items, &sections, hull, &trader.spawn.inventory);
    let mut ship_cr = trader.spawn.credits;
    let carried = ship_hold.count(&good);

    let layout = generate_station(
        &parts,
        StationLayoutRequest {
            seed: 0,
            civilization: civilization.id,
            archetype: StationArchetypeType::Ring,
            advancement: civilization.advancement,
        },
    )
    .unwrap_or_else(|failure| panic!("the station lays out: {failure}"));
    let station = generated_station(&parts, &items, &layout, &civilization)
        .unwrap_or_else(|constraint| panic!("the station is stocked: {constraint}"));
    let mut station_hold = hold(&items, &sections, &station.hull, &station.stock);
    let mut station_cr = station.credits;
    let stocked = station_hold.count(&good);
    // What the cell gives a living station: its civilization's preference
    // and its design's stock as the target, and no profile.
    let market = TraderMarket {
        cluster: CLUSTER,
        civilization: civilization.id,
        role_preference: civilization.role_preference,
        target: station
            .stock
            .stacks()
            .map(|(item, count)| (item.clone(), count))
            .collect::<BTreeMap<_, _>>(),
        profile: None,
    };

    let credits_before = ship_cr + station_cr;
    let lines = apply_visit_trades(
        &items,
        &table,
        &market,
        &profile.trades,
        &mut ship_hold,
        &mut ship_cr,
        &mut station_hold,
        &mut station_cr,
    );
    let line = lines
        .iter()
        .find(|line| line.item == good && line.side == ItemTradeType::Sell)
        .unwrap_or_else(|| panic!("the receipt lists the {good} sale: {lines:?}"));
    assert!(
        line.count > 0,
        "the {role:?} sells no {good} of {carried} to a station that holds {stocked}: {lines:?}"
    );
    assert_eq!(line.count, carried.min(intent.quantity), "{lines:?}");
    assert!(
        line.price_cr >= intent.limit_cr * line.count,
        "the sale keeps the profile's limit: {line:?}"
    );
    assert_eq!(ship_hold.count(&good), carried - line.count);
    assert_eq!(station_hold.count(&good), stocked + line.count);
    assert_eq!(ship_cr + station_cr, credits_before, "credits conserved");
    (stocked, line.count)
}

/// A generated civilian on `base/provisioner` sells its medkits to a
/// generated station, which stocks medkits itself.
#[test]
fn a_generated_provisioner_sells_its_medkits_to_a_station_that_stocks_them() {
    let (stocked, sold) =
        sell_at_a_generated_station(ShipRoleType::Civilian, "base/provisioner", "Medkit");
    assert!(stocked > 0, "the generated station stocks medkits");
    assert!(sold > 0);
}

/// A generated scavenger on `base/salvage_broker` sells its unmarked core to
/// a generated station that is not lawful. No station stocks cores, so the
/// station pays the unkept price.
#[test]
fn a_generated_salvage_broker_sells_its_unmarked_cores_to_a_station_that_is_not_lawful() {
    let (stocked, sold) = sell_at_a_generated_station(
        ShipRoleType::Scavenger,
        "base/salvage_broker",
        "UnmarkedCores",
    );
    assert_eq!(stocked, 0, "no generated station stocks contraband");
    assert!(sold > 0);
}
