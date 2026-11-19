//! Start-role census: what each start role is given and what its start
//! station offers it, over a fixed list of world seeds, on the real base
//! content and the real world generator.
//!
//! A printer, not a gate: it asserts only that every start role was
//! measured. The table is the same for the same code and content, so a run
//! before a retune and a run after it compare line by line.
//!
//! An integration test here, not an example: the real-content checks of the
//! generated world live in this folder, the crate already depends on the
//! world, gameplay and scenario crates, and a test needs no display.
//! Ignored because one run generates the 125 cells around the start of
//! every seed, about two minutes on six workers.
//!
//! Run (fish), from the repository root:
//!
//! ```text
//! env NOVA_CENSUS_OUT=(pwd)/target/start-role-census.md nix develop --command cargo test -p nova_authoring --test start_role_census -- --ignored --nocapture
//! ```
//!
//! The table goes to stdout, and to the file `NOVA_CENSUS_OUT` names when it
//! is set. `NOVA_CENSUS_ROWS` names a file for every judged board row, one
//! tab-separated line each. The report's own header states what is real and
//! what is modelled.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

use bevy::{
    ecs::{schedule::SingleThreadedExecutor, system::RunSystemOnce},
    prelude::*,
};
use nova_assets::prelude::{ContentCatalogDigest, LoadedSectionPack, LoadedSectionPacks};
use nova_authoring::{
    generation::{
        build_consumables, build_items, build_price_table, build_section_catalog, build_ships,
        build_start_roles, build_styles, build_trade_profiles,
    },
    lint_walk::repo_ship_part_packs,
};
use nova_events::prelude::Meters;
use nova_gameplay::prelude::{
    market_deals_in, plan_item_trade, plan_job_accept, plan_job_hand_in, plan_job_hull_fit, price,
    station_offers, ClusterType, DerelictShipMarker, Encounters, GameConsumables, GameItems,
    GamePriceTable, GameTradeProfiles, ItemDesignId, ItemTradeType, Job, JobField,
    JobHandInPartner, JobHullFit, JobId, JobKindType, JobPostings, JobType, Jobs,
    NeutralizedMarker, PlayerSpaceshipMarker, PriceTable, RailgunSectionMarker, SectionMarker,
    ShipInventory, SlotDraw, SpaceshipRootMarker, StanceType, StationMarker, StationServiceType,
    StationServices, TorpedoSectionMarker, TradePriceType, TraderMarket, TurretSectionMarker,
    WorldClock, JOBS_PER_BOARD, OFFENCE_STEP,
};
use nova_scenario::prelude::{
    ore_for_asteroid_kind, resolve_ship_design, GameShipDesigns, GameStartRoles, OpenWorldSession,
    RoleGoalType, ShipDesignSource, StartRole, StartingStation,
};
use nova_ship::prelude::{
    hull_price_cr, AmmoCapacity, DockingSectionMarker, GameSections, GameStyles, SectionKind,
    ShipMeasure, TurretJoint, TurretSectionConfig,
};
use nova_world::prelude::{
    desired_sectors, GeneratedBodyId, SectorClusterTypes, SectorCoord, SectorGenerationInput,
    SectorGenerator, SectorManifest, SectorRoot, SectorShipConditionType, WorldConfig,
    WorldGeometry,
};
use nova_world_base::prelude::{
    goal_progress, post_station_boards, seeded_stance, starting_station, CivilizationField,
    FieldBodies, GoalFacts, NovaLayeredWorld, PlayerHull, HAULER_CRUISE, OPEN_WORLD_ACTIVE_RADIUS,
    OPEN_WORLD_SECTOR_EDGE,
};

/// How many world seeds one run measures.
const SEED_COUNT: u32 = 240;

/// The first seed; the list is this seed and the `SEED_COUNT - 1` after it.
const FIRST_SEED: u32 = 1;

/// How many postings of each slot one run reads: the first and the next four.
const POSTINGS: u32 = 5;

/// The paper seconds one docking costs, approach to clamp. An assumption,
/// not a measurement: no run timed a dock.
const DOCK_ALLOWANCE_S: f64 = 60.0;

/// Where the greedy model stops: fifteen minutes of paper time.
const HORIZON_S: f64 = 900.0;

/// The paper seconds one hostile hull costs: closing from the raider's
/// leash, tracking it and breaking it. An assumption, not a measurement.
/// Content gives no figure: a start-band raider's health over the raw
/// damage rate of two point-defence mounts is a few seconds, and the
/// measured ammunition ledger (582 and 1,971 rounds for one raider) is 3 to
/// 10 s of trigger time, so the trigger is not what bounds a fight.
const FIGHT_S_PER_HOSTILE: f64 = 60.0;

/// The paper seconds to stop at a rock, open the emitter, line the beam up
/// and wait out the first pulse, which seeds the rock's field and takes
/// nothing. An assumption, not a measurement.
const MINING_SETUP_S: f64 = 30.0;

/// The most jobs one greedy run hands in. A docked procurement costs no
/// paper time, so only this bounds a run that never leaves the station.
const GREEDY_JOB_CAP: usize = 100;

/// The most worker threads. Each seed is measured alone, so the worker
/// count changes the run time and nothing in the table.
const WORKERS_MAX: usize = 6;

/// The seeds whose first boards the report prints row by row, to compare by
/// eye with a board line a live run logs for the same seed and role.
const SHOWN_SEEDS: [u32; 2] = [10, 17];

/// The six job kinds in the order every table lists them.
const KINDS: [JobKindType; 6] = [
    JobKindType::Procure,
    JobKindType::Recover,
    JobKindType::Escort,
    JobKindType::Distress,
    JobKindType::Neutralize,
    JobKindType::Supply,
];

/// The base content one run reads, built once and shared by the workers.
struct Catalog {
    items: GameItems,
    table: PriceTable,
    roles: Vec<StartRole>,
    hulls: Vec<RoleHull>,
    designs: GameShipDesigns,
    sections: GameSections,
    generator: NovaLayeredWorld,
}

/// A start role's hull and kit, as the base content builds them.
struct RoleHull {
    sections: usize,
    mass: f32,
    thrust: f32,
    accel: f64,
    health: f32,
    hold_g: u32,
    kit_g: u64,
    clamps: usize,
    turrets: usize,
    torpedo_bays: usize,
    railguns: usize,
    miners: usize,
    /// The first mining beam on the hull.
    beam: Option<Beam>,
    hull_cr: u64,
    stock_cr: u64,
    credits: u32,
    pdc_kit_rounds: u32,
    pdc_magazine_rounds: u32,
    pdc_unlimited: bool,
    pdc_rounds_per_second: f32,
    /// Each section's markers as [`PlayerHull`] reads them: clamp, turret,
    /// torpedo bay, railgun.
    marks: Vec<[bool; 4]>,
}

/// A mining beam as the section's config states it.
#[derive(Clone, Copy)]
struct Beam {
    reach_m: f64,
    pulse_s: f64,
    /// The ore one pulse into fresh rock is modelled to yield: one item a
    /// field corner, and half the carve sphere's volume in cells, since the
    /// beam meets a surface.
    ore_per_pulse: u32,
}

impl Beam {
    /// The paper seconds to mine `count` items once the hull is at the rock.
    fn mine_s(&self, count: u32) -> f64 {
        MINING_SETUP_S + f64::from(count.div_ceil(self.ore_per_pulse)) * self.pulse_s
    }
}

impl RoleHull {
    fn weapons(&self) -> usize {
        self.turrets + self.torpedo_bays + self.railguns
    }

    /// The seconds the kit's point-defence rounds and the loaded magazines
    /// last with every turret firing. `None` for a hull with no turret or
    /// with a magazine that never empties.
    fn fire_seconds(&self) -> Option<f32> {
        (self.pdc_rounds_per_second > 0.0 && !self.pdc_unlimited).then(|| {
            (self.pdc_kit_rounds + self.pdc_magazine_rounds) as f32 / self.pdc_rounds_per_second
        })
    }

    /// The flip-and-burn seconds over `leg_m` from rest to rest.
    fn burn_s(&self, leg_m: u32) -> f64 {
        self.burn_over_s(f64::from(leg_m))
    }

    /// [`RoleHull::burn_s`] over a distance that is no posted leg.
    fn burn_over_s(&self, metres: f64) -> f64 {
        2.0 * (metres / self.accel).sqrt()
    }
}

/// One generated ship as the mirror spawns it.
struct ShipFacts {
    hold_g: u32,
    turrets: usize,
    torpedo_bays: usize,
    railguns: usize,
}

/// One seed's world: the start the game picks and the cells of the active
/// window around the player's spawn, nearest first.
struct SeedWorld {
    seed: u32,
    start: StartingStation,
    manifests: Vec<SectorManifest>,
    /// Per manifest, per ship of it.
    ships: Vec<Vec<ShipFacts>>,
    /// The start station's hold capacity, off its hull design.
    station_hold_g: u32,
    /// Per ore, the metres from the start station to the nearest rock of
    /// its cell that yields it, less the rock's authored radius.
    rocks: BTreeMap<ItemDesignId, f64>,
    /// Every intact ship of the window that trades: the hulls the Outlaw
    /// goal counts when the player neutralizes them.
    prey: Vec<Prey>,
    /// Where the start station stands, in metres.
    station_at: Vec3,
    /// The hulls of the start station's civilization the player may take
    /// before the next one steps it to Enemy and its stations refuse the
    /// dock: [`OFFENCE_STEP`] a step from the seeded stance, less one.
    home_spare: u32,
}

/// One trading hull of the window.
#[derive(Clone, Copy)]
struct Prey {
    /// Where the manifest puts it, in metres. Traffic moves after that.
    at: Vec3,
    /// It belongs to the start station's civilization.
    home: bool,
}

/// A hunt-only paper run of the hull whose goal counts hulls taken: from
/// the start station to the nearest trading hull, then the nearest left,
/// one flip-and-burn and one fight each, no dock.
#[derive(Clone, Copy)]
struct Hunt {
    prey: usize,
    prey_home: usize,
    nearest_m: Option<f64>,
    /// Hulls taken inside the horizon.
    taken: u32,
    /// Of those, hulls of the start station's civilization.
    taken_home: u32,
    /// The paper seconds at the third hull, however late.
    three_s: Option<f64>,
    /// Hulls taken inside the horizon by a run that keeps its dock: it
    /// passes over a hull of the start station's civilization once
    /// `home_spare` of them are taken.
    kept_taken: u32,
    /// The paper seconds at that run's third hull, however late.
    kept_three_s: Option<f64>,
    /// The paper seconds when that run is back off the start station with
    /// two hulls taken.
    two_back_s: Option<f64>,
    /// The credits gained and the jobs handed in by the split run: two
    /// hulls first, then the greedy run on what is left of the horizon.
    split: Option<(f64, f64)>,
}

/// The start station as every role of one seed finds it.
#[derive(Clone)]
struct StartFacts {
    station: GeneratedBodyId,
    spawns_in_station_cell: bool,
    cluster: ClusterType,
    advancement: f32,
    design_advancement: f32,
    credits: u32,
    trades: bool,
    lawful: bool,
    stock_stacks: usize,
    stock_cr: u64,
    window_stations: usize,
    window_board_stations: usize,
    window_ships: usize,
}

/// What the player carries in a model run.
#[derive(Clone)]
struct Purse {
    hold: ShipInventory,
    credits: u32,
}

/// The start station's market side in a model run.
#[derive(Clone)]
struct Counter {
    id: GeneratedBodyId,
    market: TraderMarket,
    lawful: bool,
    trades: bool,
    stock: ShipInventory,
    credits: u32,
}

/// Where a job's goods come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FillType {
    /// The kit holds the stack.
    Kit,
    /// The station sells the stack and the player can pay and stow it.
    Station,
    /// The hull's beam cuts the stack from a rock of the station's cell.
    Mined,
    /// None of these: the goods are not to be had.
    Short,
}

/// The goods a Procure or a Supply row asks for.
#[derive(Clone)]
struct Goods {
    kit_holds: bool,
    station_stocks: bool,
    ask_cr: u64,
    fill: FillType,
}

/// One board row as the census judges it.
#[derive(Clone)]
struct Row {
    posting: u32,
    slot: u32,
    drawn: JobKindType,
    posted: JobKindType,
    /// A recovery or a neutralize that names its own target: the field had
    /// no body left to name, so the accept puts one in the world.
    spawned: bool,
    /// The starter board's redraw posted another kind than the slot drew.
    redrawn: bool,
    /// The real hull gate passes: the Jobs pane offers Accept on the row.
    fits: bool,
    /// Handing the job in alone counts toward the role's goal.
    goal: bool,
    goods: Option<Goods>,
    /// What the goods bought cost.
    cost_cr: u32,
    /// What a recovered stack sells for at the station's bid.
    kept_cr: u64,
    /// The paper seconds from accept to payout. `None` when the goods are
    /// not to be had or a recovered stack has no room.
    paper_s: Option<f64>,
    job: Job,
}

impl Row {
    fn net_cr(&self) -> i64 {
        i64::from(self.job.reward.total_cr()) - i64::from(self.cost_cr)
    }

    /// The hull fits it and the paper model can finish it.
    fn doable(&self) -> bool {
        self.fits && self.paper_s.is_some()
    }

    /// Doable and it pays more than its goods cost.
    fn pays(&self) -> bool {
        self.doable() && self.net_cr() > 0
    }

    /// The order the greedy model prefers rows in: a row that costs no time
    /// first, then net pay per paper second, then net pay.
    fn rate_key(&self) -> (bool, f64, i64) {
        let seconds = self.paper_s.unwrap_or(f64::INFINITY);
        let net = self.net_cr();
        if seconds <= 0.0 {
            (true, 0.0, net)
        } else {
            (false, net as f64 / seconds, net)
        }
    }
}

/// The field the start station's draw read.
#[derive(Clone, Default)]
struct FieldSeen {
    /// Less the bodies other boards named.
    seen: JobField,
    /// Every wreck with a hold and every hostile in the cube.
    whole: JobField,
}

/// Why a greedy run took no further job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum StopType {
    /// No row on the board fits the hull.
    NothingFits,
    /// A row fits, but its goods are not to be had or its stack has no room.
    NotDoable,
    /// A row can be done, but none pays more than its goods cost.
    NothingPays,
    /// A row pays, but the station has no credits for the reward.
    StationCannotPay,
    /// A row pays, but it ends past the horizon.
    Horizon,
    /// The run reached the job cap.
    JobCap,
}

impl StopType {
    fn label(self) -> &'static str {
        match self {
            Self::NothingFits => "no row fits the hull",
            Self::NotDoable => "goods not to be had or no hold room",
            Self::NothingPays => "no row pays above its goods",
            Self::StationCannotPay => "station cannot pay",
            Self::Horizon => "next row ends past the horizon",
            Self::JobCap => "job cap reached",
        }
    }
}

/// One job a greedy run handed in.
#[derive(Clone)]
struct Done {
    kind: JobKindType,
    net_cr: i64,
    docked: bool,
}

/// One greedy run.
#[derive(Clone)]
struct Greedy {
    end_cr: u32,
    done: Vec<Done>,
    sales_cr: u32,
    first_payout_s: Option<f64>,
    /// The model clock when the run took its last job's payout or stopped.
    stopped_s: f64,
    goal: (u64, u64),
    stop: StopType,
}

/// One role on one seed.
struct RoleMeasure {
    /// How many cells were live when the start board posted.
    cells_live: usize,
    field: FieldSeen,
    rows: Vec<Row>,
    /// The goal count when every fitting row of the fifteen is handed in.
    goal_all: (u64, u64),
    greedy: Greedy,
    /// The first board's titles, kept for the shown seeds only.
    titles: Vec<String>,
}

/// One seed.
struct SeedMeasure {
    seed: u32,
    start: StartFacts,
    roles: Vec<RoleMeasure>,
    /// The hunt-only run of each role whose goal counts hulls taken.
    hunts: Vec<(usize, Hunt)>,
}

/// A live mirror of one seed's window with the real board pass in `Update`.
struct Mirror {
    app: App,
    bodies: BTreeMap<GeneratedBodyId, Entity>,
    /// Every living station of the window that sorts before the start
    /// station: the board pass draws those first on a frame.
    earlier: Vec<GeneratedBodyId>,
}

fn catalog() -> Catalog {
    let items = GameItems::new(build_items());
    let loaded = LoadedSectionPacks {
        packs: repo_ship_part_packs(&["base"])
            .into_iter()
            .map(|pack| LoadedSectionPack {
                id: pack.id,
                dependencies: pack.dependencies,
                sections: pack.sections,
            })
            .collect(),
        digest: ContentCatalogDigest(1),
    };
    let generator = NovaLayeredWorld::from_loaded(
        &loaded,
        &GameStyles(build_styles()),
        &items,
        &GameConsumables::new(build_consumables()),
        &GameTradeProfiles(
            build_trade_profiles()
                .into_iter()
                .map(|profile| (profile.id.clone(), profile))
                .collect(),
        ),
    )
    .unwrap_or_else(|faults| panic!("the base part packs build a generator: {faults:?}"));
    let designs = GameShipDesigns(build_ships());
    let sections = GameSections(build_section_catalog());
    let roles = build_start_roles();
    let hulls = roles
        .iter()
        .map(|role| role_hull(role, &designs, &sections, &items))
        .collect();
    Catalog {
        items,
        table: build_price_table(),
        roles,
        hulls,
        designs,
        sections,
        generator,
    }
}

/// The summed fire rate of every muzzle under `joint`.
fn muzzle_rate(joint: &TurretJoint) -> f32 {
    joint.muzzle.as_ref().map_or(0.0, |muzzle| muzzle.fire_rate)
        + joint.children.iter().map(muzzle_rate).sum::<f32>()
}

fn role_hull(
    role: &StartRole,
    designs: &GameShipDesigns,
    sections: &GameSections,
    items: &GameItems,
) -> RoleHull {
    let (design, errors) = resolve_ship_design(
        &ShipDesignSource::prototype(role.hull.clone()),
        designs,
        sections,
    );
    assert!(
        errors.is_empty(),
        "the start hull '{}' resolves: {errors:?}",
        role.hull
    );
    let measure = ShipMeasure::of(design.sections.iter().map(|section| &section.config));
    // A drive's magnitude is an impulse per fixed tick, so the acceleration
    // is `thrust / (mass * tick)` in engine units.
    let tick = Time::<Fixed>::default().timestep().as_secs_f32();
    let accel = f64::from(Meters::from_engine(measure.thrust / (measure.mass * tick)).get());
    let pdc: ItemDesignId = TurretSectionConfig::RELOAD_ITEM.into();
    let mut hull = RoleHull {
        sections: design.sections.len(),
        mass: measure.mass,
        thrust: measure.thrust,
        accel,
        health: design
            .sections
            .iter()
            .map(|section| section.config.base.health)
            .sum(),
        hold_g: design.cargo_capacity_g(),
        kit_g: role
            .kit
            .stock
            .stacks()
            .map(|(item, count)| items.design(item).stack_mass_g(count))
            .sum(),
        clamps: 0,
        turrets: 0,
        torpedo_bays: 0,
        railguns: 0,
        miners: 0,
        beam: None,
        hull_cr: hull_price_cr(
            items,
            design
                .sections
                .iter()
                .map(|section| &section.config.base.build),
        ),
        stock_cr: role
            .kit
            .stock
            .stacks()
            .map(|(item, count)| items.design(item).reference_price().buy_cr(count))
            .sum(),
        credits: role.kit.credits,
        pdc_kit_rounds: role
            .kit
            .stock
            .stacks()
            .find(|(item, _)| **item == pdc)
            .map_or(0, |(_, count)| count),
        pdc_magazine_rounds: 0,
        pdc_unlimited: false,
        pdc_rounds_per_second: 0.0,
        marks: Vec::new(),
    };
    for section in &design.sections {
        let mut mark = [false; 4];
        match &section.config.kind {
            SectionKind::Docking(_) => {
                hull.clamps += 1;
                mark[0] = true;
            }
            SectionKind::Turret(turret) => {
                hull.turrets += 1;
                mark[1] = true;
                hull.pdc_rounds_per_second += muzzle_rate(&turret.root);
                match turret.ammunition {
                    AmmoCapacity::Limited(rounds) => hull.pdc_magazine_rounds += rounds,
                    AmmoCapacity::Unlimited => hull.pdc_unlimited = true,
                }
            }
            SectionKind::Torpedo(_) => {
                hull.torpedo_bays += 1;
                mark[2] = true;
            }
            SectionKind::Railgun(_) => {
                hull.railguns += 1;
                mark[3] = true;
            }
            SectionKind::Mining(beam) => {
                hull.miners += 1;
                let radius = f64::from(beam.carve_radius_cells);
                hull.beam.get_or_insert(Beam {
                    reach_m: f64::from(beam.reach.get()),
                    pulse_s: f64::from(beam.pulse_interval_seconds),
                    ore_per_pulse: ((2.0 / 3.0) * std::f64::consts::PI * radius.powi(3)).floor()
                        as u32,
                });
            }
            SectionKind::Hull(_)
            | SectionKind::Thruster(_)
            | SectionKind::Controller(_)
            | SectionKind::CargoIntake(_) => {}
        }
        hull.marks.push(mark);
    }
    hull
}

/// The start of `seed` and the manifests of the window around the spawn.
fn seed_world(catalog: &Catalog, seed: u32) -> Result<SeedWorld, String> {
    let start = starting_station(seed, &catalog.generator).map_err(|fault| fault.to_string())?;
    let centre = SectorCoord::containing(start.spawn, OPEN_WORLD_SECTOR_EDGE);
    let mut cells: Vec<SectorCoord> = desired_sectors(centre, OPEN_WORLD_ACTIVE_RADIUS)
        .into_iter()
        .collect();
    // The streaming loop materializes the nearest ready cell first.
    cells.sort_by_key(|cell| {
        let (x, y, z) = (
            i64::from(cell.x - centre.x),
            i64::from(cell.y - centre.y),
            i64::from(cell.z - centre.z),
        );
        (x * x + y * y + z * z, *cell)
    });
    let manifests: Vec<SectorManifest> = cells
        .into_iter()
        .map(|coord| {
            catalog
                .generator
                .generate(SectorGenerationInput {
                    seed,
                    geometry: WorldGeometry {
                        sector_edge: OPEN_WORLD_SECTOR_EDGE,
                    },
                    coord,
                })
                .unwrap_or_else(|fault| panic!("seed {seed}: the cell {coord} generates: {fault}"))
        })
        .collect();
    let resolve = |source: &ShipDesignSource, what: &dyn std::fmt::Display| {
        let (design, errors) = resolve_ship_design(source, &catalog.designs, &catalog.sections);
        assert!(
            errors.is_empty(),
            "seed {seed}: {what} resolves: {errors:?}"
        );
        design
    };
    let ships = manifests
        .iter()
        .map(|manifest| {
            manifest
                .ships
                .iter()
                .map(|ship| {
                    let design = resolve(&ship.spawn.design, &ship.id);
                    let count = |wanted: fn(&SectionKind) -> bool| {
                        design
                            .sections
                            .iter()
                            .filter(|section| wanted(&section.config.kind))
                            .count()
                    };
                    ShipFacts {
                        hold_g: design.cargo_capacity_g(),
                        turrets: count(|kind| matches!(kind, SectionKind::Turret(_))),
                        torpedo_bays: count(|kind| matches!(kind, SectionKind::Torpedo(_))),
                        railguns: count(|kind| matches!(kind, SectionKind::Railgun(_))),
                    }
                })
                .collect()
        })
        .collect();
    let station = manifests
        .iter()
        .flat_map(|manifest| &manifest.stations)
        .find(|station| station.id == start.id)
        .unwrap_or_else(|| panic!("seed {seed}: the window holds the start '{}'", start.id));
    let station_hold_g = resolve(
        &ShipDesignSource::Inline(station.design().hull.clone()),
        &station.id,
    )
    .cargo_capacity_g();
    let home = manifests
        .iter()
        .find(|manifest| manifest.stations.iter().any(|other| other.id == start.id))
        .expect("the start station's cell is in the window");
    let station_at = station.position.get();
    let mut rocks: BTreeMap<ItemDesignId, f64> = BTreeMap::new();
    for rock in &home.asteroids {
        let Some(ore) = ore_for_asteroid_kind(&rock.config.kind) else {
            continue;
        };
        let off = f64::from((rock.position.get() - station_at).length() - rock.config.radius.get())
            .max(0.0);
        let nearest = rocks.entry(ore.id().into()).or_insert(f64::INFINITY);
        *nearest = nearest.min(off);
    }
    let prey = manifests
        .iter()
        .flat_map(|manifest| &manifest.ships)
        .filter(|ship| ship.market.is_some())
        .map(|ship| Prey {
            at: ship.position.get(),
            home: ship.civilization == station.civilization,
        })
        .collect();
    let home_spare = match seeded_stance(&station.civilization) {
        StanceType::Ally => 2 * OFFENCE_STEP - 1,
        StanceType::Neutral => OFFENCE_STEP - 1,
        StanceType::Enemy => 0,
    };
    Ok(SeedWorld {
        seed,
        start,
        manifests,
        ships,
        station_hold_g,
        rocks,
        prey,
        station_at,
        home_spare,
    })
}

/// The start station's manifest entry.
fn start_station(world: &SeedWorld) -> &nova_world::prelude::SectorStation {
    world
        .manifests
        .iter()
        .flat_map(|manifest| &manifest.stations)
        .find(|station| station.id == world.start.id)
        .expect("seed_world found the start station")
}

fn start_facts(catalog: &Catalog, world: &SeedWorld) -> StartFacts {
    let station = start_station(world);
    let design = station.design();
    let market = station
        .market
        .as_ref()
        .expect("a living start station carries a market");
    let civilization = CivilizationField::new(market.civilization.world_seed)
        .civilization(market.civilization.node);
    let stations = || {
        world
            .manifests
            .iter()
            .flat_map(|manifest| &manifest.stations)
    };
    let services = |station: &nova_world::prelude::SectorStation| {
        StationServices(station.design().services.clone())
    };
    StartFacts {
        station: station.id.clone(),
        spawns_in_station_cell: SectorCoord::containing(world.start.spawn, OPEN_WORLD_SECTOR_EDGE)
            == station.id.origin,
        cluster: market.cluster,
        advancement: civilization.advancement,
        design_advancement: design.advancement,
        credits: design.credits,
        trades: station_offers(
            Some(&services(station)),
            station.spawn.derelict,
            StationServiceType::Trade,
        ),
        lawful: civilization.lawful,
        stock_stacks: design.stock.stacks().count(),
        stock_cr: design
            .stock
            .stacks()
            .map(|(item, count)| catalog.items.design(item).reference_price().buy_cr(count))
            .sum(),
        window_stations: stations().count(),
        window_board_stations: stations()
            .filter(|station| {
                station_offers(
                    Some(&services(station)),
                    station.spawn.derelict,
                    StationServiceType::Jobs,
                )
            })
            .count(),
        window_ships: world.manifests.iter().map(|m| m.ships.len()).sum(),
    }
}

/// The player's purse and the start station's counter at the start.
fn opening(catalog: &Catalog, world: &SeedWorld, role: usize) -> (Purse, Counter) {
    let station = start_station(world);
    let design = station.design();
    let market = station
        .market
        .clone()
        .expect("a living start station carries a market");
    let civilization = CivilizationField::new(market.civilization.world_seed)
        .civilization(market.civilization.node);
    let kit = &catalog.roles[role].kit;
    let stacks = |stock: &nova_gameplay::prelude::ShipInventoryStock| {
        stock
            .stacks()
            .map(|(item, count)| (item.clone(), count))
            .collect::<Vec<_>>()
    };
    (
        Purse {
            hold: ShipInventory::new(
                &catalog.items,
                catalog.hulls[role].hold_g,
                stacks(&kit.stock),
            ),
            credits: kit.credits,
        },
        Counter {
            id: station.id.clone(),
            trades: station_offers(
                Some(&StationServices(design.services.clone())),
                station.spawn.derelict,
                StationServiceType::Trade,
            ),
            lawful: civilization.lawful,
            stock: ShipInventory::new(&catalog.items, world.station_hold_g, stacks(&design.stock)),
            credits: design.credits,
            market,
        },
    )
}

/// A bare app with the real board pass, the real resources and the player's
/// hull, and no cell yet.
fn mirror(catalog: &Catalog, world: &SeedWorld, role: usize) -> Mirror {
    let mut app = App::new();
    app.insert_resource(WorldConfig {
        seed: world.seed,
        sector_edge: OPEN_WORLD_SECTOR_EDGE,
        active_radius: OPEN_WORLD_ACTIVE_RADIUS,
        generator: catalog.generator.clone(),
    });
    app.insert_resource(catalog.items.clone());
    app.insert_resource(GamePriceTable(catalog.table.clone()));
    app.insert_resource(GameStartRoles(catalog.roles.clone()));
    app.insert_resource(OpenWorldSession {
        seed: world.seed,
        role: catalog.roles[role].id.clone(),
        start: world.start.clone(),
    });
    app.init_resource::<WorldClock>();
    app.init_resource::<Jobs>();
    app.init_resource::<JobPostings>();
    app.init_resource::<Encounters>();
    app.add_systems(Update, post_station_boards);
    // One system: a worker thread per core would only cost start-up time.
    app.edit_schedule(Update, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    let (purse, _) = opening(catalog, world, role);
    let player = app
        .world_mut()
        .spawn((
            PlayerSpaceshipMarker,
            Transform::from_translation(world.start.spawn.to_engine()),
            purse.hold,
        ))
        .id();
    for [clamp, turret, bay, railgun] in &catalog.hulls[role].marks {
        let mut section = app.world_mut().spawn((SectionMarker, ChildOf(player)));
        if *clamp {
            section.insert(DockingSectionMarker);
        }
        if *turret {
            section.insert(TurretSectionMarker);
        }
        if *bay {
            section.insert(TorpedoSectionMarker);
        }
        if *railgun {
            section.insert(RailgunSectionMarker);
        }
    }
    Mirror {
        app,
        bodies: BTreeMap::new(),
        earlier: Vec::new(),
    }
}

/// Put one cell of `world` in the mirror: its root, its stations and its
/// ships, each with the components the board pass reads.
fn spawn_cell(catalog: &Catalog, world: &SeedWorld, cell: usize, mirror: &mut Mirror) {
    let manifest = &world.manifests[cell];
    let app = &mut mirror.app;
    let root = app
        .world_mut()
        .spawn((
            SectorRoot(manifest.coord),
            SectorClusterTypes(manifest.cluster_types.clone()),
            Transform::default(),
        ))
        .id();
    for station in &manifest.stations {
        let design = station.design();
        let services = StationServices(design.services.clone());
        if station.id < world.start.id
            && station_offers(
                Some(&services),
                station.spawn.derelict,
                StationServiceType::Jobs,
            )
        {
            mirror.earlier.push(station.id.clone());
        }
        let mut entity = app.world_mut().spawn((
            StationMarker,
            station.id.clone(),
            Transform::from_translation(station.position.to_engine()),
            ChildOf(root),
            services,
        ));
        if station.spawn.derelict {
            entity.insert(DerelictShipMarker);
        }
        if let Some(market) = &station.market {
            entity.insert(market.clone());
        }
        mirror.bodies.insert(station.id.clone(), entity.id());
    }
    for (ship, facts) in manifest.ships.iter().zip(&world.ships[cell]) {
        let side = ship
            .spawn
            .allegiance
            .unwrap_or_else(|| panic!("the generated ship '{}' states its side", ship.id));
        let hold = ShipInventory::new(
            &catalog.items,
            facts.hold_g,
            ship.spawn
                .inventory
                .stacks()
                .map(|(item, count)| (item.clone(), count)),
        );
        let mut entity = app.world_mut().spawn((
            SpaceshipRootMarker,
            ship.id.clone(),
            Name::new(ship.id.to_string()),
            Transform::from_translation(ship.position.to_engine()),
            ChildOf(root),
            hold,
            side,
        ));
        let derelict = ship.condition == SectorShipConditionType::Derelict;
        if derelict {
            entity.insert(DerelictShipMarker);
        }
        // The spawn puts an intact ship's market on its root.
        if let Some(market) = &ship.market {
            entity.insert(market.clone());
        }
        let entity = entity.id();
        mirror.bodies.insert(ship.id.clone(), entity);
        // A derelict's weapons spawn inactive; the field counts live ones.
        if !derelict {
            for _ in 0..facts.turrets {
                app.world_mut()
                    .spawn((SectionMarker, TurretSectionMarker, ChildOf(entity)));
            }
            for _ in 0..facts.torpedo_bays {
                app.world_mut()
                    .spawn((SectionMarker, TorpedoSectionMarker, ChildOf(entity)));
            }
            for _ in 0..facts.railguns {
                app.world_mut()
                    .spawn((SectionMarker, RailgunSectionMarker, ChildOf(entity)));
            }
        }
    }
}

/// The offers on `station`'s slots, in slot order.
fn board(app: &App, station: &GeneratedBodyId) -> Vec<Job> {
    app.world()
        .resource::<JobPostings>()
        .board(station)
        .map(|slots| slots.iter().map(|slot| slot.offer.clone()).collect())
        .unwrap_or_default()
}

/// The field around `station` by the board pass's own rule, less `excluded`
/// and whole.
fn read_field(
    In((station, excluded)): In<(GeneratedBodyId, BTreeSet<GeneratedBodyId>)>,
    field: FieldBodies,
    stations: Query<(Entity, &GeneratedBodyId, &ChildOf), With<StationMarker>>,
    roots: Query<(Entity, &SectorRoot)>,
) -> FieldSeen {
    let (poster, _, parent) = stations
        .iter()
        .find(|(_, id, _)| **id == station)
        .expect("the start station stands");
    let (_, centre) = roots
        .get(parent.parent())
        .expect("the start station hangs off a cell root");
    let mut around = Vec::with_capacity(27);
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                let coord = centre.0.offset(x, y, z);
                let (root, _) = roots
                    .iter()
                    .find(|(_, root)| root.0 == coord)
                    .expect("the start station posted, so its cube is live");
                around.push(root);
            }
        }
    }
    FieldSeen {
        seen: field.field(poster, &around, &excluded),
        whole: field.field(poster, &around, &BTreeSet::new()),
    }
}

/// Run one frame. When the start board drew new offers on it, give the
/// field that draw read: the cube less every body a standing offer named
/// before the frame and every body a station drawn earlier on the frame
/// named.
fn pass(mirror: &mut Mirror, start: &GeneratedBodyId) -> Option<FieldSeen> {
    let world = mirror.app.world();
    let rule = world.resource::<GamePriceTable>().0.jobs;
    let now = world.resource::<WorldClock>().play_seconds;
    let mut excluded =
        world
            .resource::<JobPostings>()
            .named_bodies(rule, now, world.resource::<Jobs>());
    let offered = |app: &App| -> BTreeSet<JobId> {
        mirror
            .earlier
            .iter()
            .chain(std::iter::once(start))
            .flat_map(|station| board(app, station))
            .map(|job| job.id)
            .collect()
    };
    let before = offered(&mirror.app);
    mirror.app.update();
    let drew = board(&mirror.app, start)
        .iter()
        .any(|job| !before.contains(&job.id));
    if !drew {
        return None;
    }
    for station in &mirror.earlier {
        for job in board(&mirror.app, station) {
            if !before.contains(&job.id) {
                excluded.extend(job.located_body().cloned());
            }
        }
    }
    Some(
        mirror
            .app
            .world_mut()
            .run_system_once_with(read_field, (start.clone(), excluded))
            .expect("the field read runs"),
    )
}

/// Stream the window in, one cell a frame, nearest first. Gives the mirror,
/// how many cells were live when the start board posted, and the field its
/// draw read.
fn stream(catalog: &Catalog, world: &SeedWorld, role: usize) -> (Mirror, usize, FieldSeen) {
    let mut mirror = mirror(catalog, world, role);
    let mut first = None;
    for cell in 0..world.manifests.len() {
        spawn_cell(catalog, world, cell, &mut mirror);
        let seen = pass(&mut mirror, &world.start.id);
        if first.is_none() {
            first = seen.map(|field| (cell + 1, field));
        }
    }
    let (cells_live, field) = first.unwrap_or_else(|| {
        panic!(
            "seed {}: the start station '{}' posted no board with its window live",
            world.seed, world.start.id
        )
    });
    (mirror, cells_live, field)
}

/// The player's hull by the real read the board pass and the Jobs pane share.
fn hull_fit(app: &mut App) -> JobHullFit {
    app.world_mut()
        .run_system_once(|hull: PlayerHull| hull.fit())
        .expect("the hull read runs")
}

fn kind_of(job: &Job) -> JobKindType {
    match job.kind {
        JobType::Procure { .. } => JobKindType::Procure,
        JobType::Recover { .. } => JobKindType::Recover,
        JobType::Escort { .. } => JobKindType::Escort,
        JobType::Distress { .. } => JobKindType::Distress,
        JobType::Neutralize { .. } => JobKindType::Neutralize,
        JobType::Supply { .. } => JobKindType::Supply,
    }
}

/// The role's own goal line over `completed` and `hulls_taken`, the first
/// of its progress: what it has and what it asks for.
fn goal_count(catalog: &Catalog, role: usize, completed: &[Job], hulls_taken: u64) -> (u64, u64) {
    let progress = goal_progress(
        &catalog.roles[role].goal,
        &GoalFacts {
            completed,
            owned_stations: 0,
            items: &catalog.items,
            hulls_taken,
        },
    );
    let own = progress
        .lines
        .first()
        .expect("a goal's progress opens with the role's own line");
    (own.have, own.need)
}

/// Whether neutralizing the hull of `job` takes a trading hull: a Neutralize
/// of a hull that stood in the field. Every field hostile is an intact
/// generated ship, which carries a market; a job's own raider carries none.
fn takes_trading_hull(job: &Job) -> bool {
    matches!(job.kind, JobType::Neutralize { .. }) && !job.spawns_target()
}

/// One hunt-only chain of `hull` over the trading hulls of `world`: the
/// hulls taken inside the horizon, of them the start civilization's, the
/// metres to the first and the paper seconds at the third. With
/// `home_spare` the chain passes over a hull of the start civilization once
/// that many are taken.
fn hunt_chain(
    hull: &RoleHull,
    world: &SeedWorld,
    home_spare: Option<u32>,
) -> (u32, u32, Option<f64>, Option<f64>, Option<f64>) {
    let mut left: Vec<Prey> = world.prey.clone();
    let mut at = world.station_at;
    let (mut clock, mut home, mut taken, mut taken_home) = (0.0, 0, 0, 0);
    let (mut nearest_m, mut three_s, mut two_back_s) = (None, None, None);
    for step in 0..3 {
        left.retain(|prey| !prey.home || home_spare.is_none_or(|spare| home < spare));
        let Some((next, off)) = left
            .iter()
            .enumerate()
            .map(|(next, prey)| (next, f64::from((prey.at - at).length())))
            .min_by(|a, b| a.1.total_cmp(&b.1))
        else {
            break;
        };
        if step == 0 {
            nearest_m = Some(off);
        }
        let prey = left.swap_remove(next);
        at = prey.at;
        home += u32::from(prey.home);
        clock += hull.burn_over_s(off) + FIGHT_S_PER_HOSTILE;
        if clock <= HORIZON_S {
            taken += 1;
            taken_home += u32::from(prey.home);
        }
        if step == 1 {
            let back_m = f64::from((at - world.station_at).length());
            two_back_s = Some(clock + hull.burn_over_s(back_m));
        }
        if step == 2 {
            three_s = Some(clock);
        }
    }
    (taken, taken_home, nearest_m, three_s, two_back_s)
}

/// The hunt-only paper runs of `hull` over the trading hulls of `world`.
fn hunt(hull: &RoleHull, world: &SeedWorld) -> Hunt {
    let (taken, taken_home, nearest_m, three_s, _) = hunt_chain(hull, world, None);
    let (kept_taken, _, _, kept_three_s, two_back_s) =
        hunt_chain(hull, world, Some(world.home_spare));
    Hunt {
        prey: world.prey.len(),
        prey_home: world.prey.iter().filter(|prey| prey.home).count(),
        nearest_m,
        taken,
        taken_home,
        three_s,
        kept_taken,
        kept_three_s,
        two_back_s,
        split: None,
    }
}

/// The station's price of `item` at its stock now.
fn quote(
    catalog: &Catalog,
    counter: &Counter,
    item: &ItemDesignId,
) -> nova_gameplay::prelude::ItemPrice {
    price(
        &catalog.table,
        catalog.items.design(item),
        &counter.market,
        counter.stock.count(item),
    )
    .price
}

/// Judge `job` for the hull `fit` with the purse and the counter as they
/// stand: its goods, their cost and its paper time.
fn judge(
    catalog: &Catalog,
    world: &SeedWorld,
    role: usize,
    fit: JobHullFit,
    job: &Job,
    drawn: JobKindType,
    purse: &Purse,
    counter: &Counter,
) -> Row {
    let hull = &catalog.hulls[role];
    let items = &catalog.items;
    let stocked = |item: &ItemDesignId, count: u32| -> (Goods, u32) {
        let design = items.design(item);
        let price = quote(catalog, counter, item);
        let kit_holds = purse.hold.count(item) >= count;
        let buy = plan_item_trade(
            items,
            ItemTradeType::Buy,
            counter.trades.then_some(TradePriceType::Market(price)),
            counter.lawful,
            item,
            Some(count),
            &purse.hold,
            purse.credits,
            &counter.stock,
            counter.credits,
        );
        let (fill, cost_cr) = match (kit_holds, &buy) {
            (true, _) => (FillType::Kit, 0),
            (false, Ok(trade)) => (FillType::Station, trade.price_cr),
            (false, Err(_)) => (FillType::Short, 0),
        };
        (
            Goods {
                kit_holds,
                station_stocks: counter.trades
                    && market_deals_in(design, counter.lawful)
                    && counter.stock.count(item) >= count,
                ask_cr: price.buy_cr(count),
                fill,
            },
            cost_cr,
        )
    };
    let dock = DOCK_ALLOWANCE_S;
    let burn = hull.burn_s(job.leg_m);
    let (goods, cost_cr, kept_cr, paper_s) = match &job.kind {
        JobType::Procure { item, count } => {
            let (mut goods, cost_cr) = stocked(item, *count);
            let mut paper = (goods.fill != FillType::Short).then_some(0.0);
            // The beam cuts what the kit and the station lack: out to the
            // nearest rock of the cell that yields the ore, the cut, back
            // and one dock.
            let room =
                items.design(item).stack_mass_g(*count) <= u64::from(purse.hold.free_g(items));
            if let (FillType::Short, Some(beam), Some(off), true) =
                (goods.fill, hull.beam, world.rocks.get(item), room)
            {
                let flight = hull.burn_over_s((off - beam.reach_m).max(0.0));
                goods.fill = FillType::Mined;
                paper = Some(2.0 * flight + beam.mine_s(*count) + dock);
            }
            (Some(goods), cost_cr, 0, paper)
        }
        JobType::Supply { item, count, .. } => {
            let (goods, cost_cr) = stocked(item, *count);
            let paper = (goods.fill != FillType::Short).then_some(2.0 * burn + 2.0 * dock);
            (Some(goods), cost_cr, 0, paper)
        }
        JobType::Recover { item, count, .. } => {
            // The lift is done only with the whole stack in the hold.
            let room =
                items.design(item).stack_mass_g(*count) <= u64::from(purse.hold.free_g(items));
            (
                None,
                0,
                quote(catalog, counter, item).sell_cr(*count),
                room.then_some(2.0 * burn + 2.0 * dock),
            )
        }
        JobType::Distress { .. } => (None, 0, 0, Some(2.0 * burn + 2.0 * dock)),
        JobType::Escort { raiders, .. } => (
            None,
            0,
            0,
            Some(
                f64::from(job.leg_m) / f64::from(HAULER_CRUISE.0)
                    + f64::from(*raiders) * FIGHT_S_PER_HOSTILE
                    + burn
                    + dock,
            ),
        ),
        JobType::Neutralize { .. } => (None, 0, 0, Some(2.0 * burn + FIGHT_S_PER_HOSTILE + dock)),
    };
    let (posting, slot) = job
        .id
        .slot_posting()
        .map(|(slot, posting)| (posting, slot))
        .expect("a board slot holds a slot job");
    Row {
        posting,
        slot,
        drawn,
        posted: kind_of(job),
        spawned: job.spawns_target(),
        redrawn: kind_of(job) != drawn,
        fits: plan_job_hull_fit(job, items, fit).is_ok(),
        goal: goal_count(
            catalog,
            role,
            std::slice::from_ref(job),
            u64::from(takes_trading_hull(job)),
        )
        .0 > 0,
        goods,
        cost_cr,
        kept_cr,
        paper_s,
        job: job.clone(),
    }
}

/// Check one judged row of the start board against the draw rules this
/// census reports.
///
/// # Panics
///
/// When the starter board's redraw posted a job the hull is refused for, or
/// a row names a body as the field's that the cube does not hold: the board
/// pass would then be doing something this census does not report.
fn check_row(seed: u32, row: &Row, field: &FieldSeen) {
    assert!(
        !row.redrawn || row.fits,
        "seed {seed}: '{}' drew {:?} and was redrawn to a {:?} the hull is refused for",
        row.job.id,
        row.drawn,
        row.posted
    );
    let target = match &row.job.kind {
        JobType::Recover { wreck: target, .. } | JobType::Neutralize { hull: target, .. }
            if !row.spawned =>
        {
            target
        }
        _ => return,
    };
    let stands = field.whole.wrecks.iter().any(|wreck| wreck.id == *target)
        || field.whole.hostiles.iter().any(|hull| hull.id == *target);
    assert!(
        stands,
        "seed {seed}: '{}' names '{target}', which is no body of the cube and not its own",
        row.job.id
    );
}

/// The board census of one role on one seed: the first posting and the
/// next four of every slot, each renewal by a passed offer window.
fn board_census(
    catalog: &Catalog,
    world: &SeedWorld,
    role: usize,
    advancement: f32,
) -> (usize, FieldSeen, Vec<Row>, Vec<String>) {
    let (mut mirror, cells_live, first) = stream(catalog, world, role);
    let fit = hull_fit(&mut mirror.app);
    let (purse, counter) = opening(catalog, world, role);
    let mix = catalog.roles[role].jobs;
    let rule = catalog.table.jobs;
    let window = rule.offer_seconds.max(rule.escort_offer_seconds) + 1.0;
    let mut rows = Vec::new();
    let mut titles = Vec::new();
    let mut field = first.clone();
    for posting in 0..POSTINGS {
        if posting > 0 {
            mirror
                .app
                .world_mut()
                .resource_mut::<WorldClock>()
                .play_seconds += f64::from(window);
            field = pass(&mut mirror, &world.start.id).unwrap_or_else(|| {
                panic!(
                    "seed {}: the start board did not renew at posting {posting}",
                    world.seed
                )
            });
        }
        let offers = board(&mirror.app, &world.start.id);
        assert_eq!(
            offers.len(),
            JOBS_PER_BOARD as usize,
            "seed {}: the start board's slots",
            world.seed
        );
        let drawn: Vec<JobKindType> = offers
            .iter()
            .map(|job| SlotDraw::of(world.seed, &job.id, mix, advancement).kind)
            .collect();
        for (job, drawn) in offers.iter().zip(drawn) {
            assert_eq!(
                job.id.slot_posting().map(|(_, at)| at),
                Some(posting),
                "seed {}: '{}' is on posting {posting}",
                world.seed,
                job.id
            );
            if posting == 0 && SHOWN_SEEDS.contains(&world.seed) {
                titles.push(job.title(&catalog.items));
            }
            let row = judge(catalog, world, role, fit, job, drawn, &purse, &counter);
            check_row(world.seed, &row, &field);
            rows.push(row);
        }
    }
    (cells_live, first, rows, titles)
}

/// The greedy model of one role on one seed.
fn greedy(
    catalog: &Catalog,
    world: &SeedWorld,
    role: usize,
    advancement: f32,
    head_s: f64,
) -> Greedy {
    let (mut mirror, ..) = stream(catalog, world, role);
    let fit = hull_fit(&mut mirror.app);
    let (mut purse, mut counter) = opening(catalog, world, role);
    let items = &catalog.items;
    let mix = catalog.roles[role].jobs;
    // The player spawns off the station and docks once before any accept,
    // `head_s` after the start.
    let mut clock = head_s + DOCK_ALLOWANCE_S;
    let mut done: Vec<Done> = Vec::new();
    let mut sales_cr = 0;
    let mut first_payout_s = None;
    let mut hulls_taken = 0;
    let stop = loop {
        if done.len() >= GREEDY_JOB_CAP {
            break StopType::JobCap;
        }
        mirror
            .app
            .world_mut()
            .resource_mut::<WorldClock>()
            .play_seconds = clock;
        mirror.app.update();
        let jobs = mirror.app.world().resource::<Jobs>();
        let mut reached = StopType::NothingFits;
        let mut best: Option<Row> = None;
        for job in board(&mirror.app, &world.start.id) {
            if plan_job_accept(&job, Some(&counter.id), jobs, false, true).is_err() {
                continue;
            }
            let drawn = SlotDraw::of(world.seed, &job.id, mix, advancement).kind;
            let row = judge(catalog, world, role, fit, &job, drawn, &purse, &counter);
            let stage = if !row.fits {
                StopType::NothingFits
            } else if row.paper_s.is_none() {
                StopType::NotDoable
            } else if row.net_cr() <= 0 {
                StopType::NothingPays
            } else if counter.credits + row.cost_cr < row.job.reward.total_cr() {
                StopType::StationCannotPay
            } else if clock + row.paper_s.unwrap_or(0.0) > HORIZON_S {
                StopType::Horizon
            } else {
                if best
                    .as_ref()
                    .is_none_or(|best| row.rate_key() > best.rate_key())
                {
                    best = Some(row);
                }
                continue;
            };
            reached = reached.max(stage);
        }
        let Some(row) = best else {
            break reached;
        };
        let job = &row.job;
        let goods = match &job.kind {
            JobType::Procure { item, count } | JobType::Supply { item, count, .. } => {
                Some((item, *count))
            }
            JobType::Recover { .. }
            | JobType::Escort { .. }
            | JobType::Distress { .. }
            | JobType::Neutralize { .. } => None,
        };
        match (goods, row.goods.as_ref().map(|goods| goods.fill)) {
            (Some((item, count)), Some(FillType::Station)) => {
                purse.hold.add(items, item, count);
                counter.stock.remove(item, count);
                purse.credits -= row.cost_cr;
                counter.credits += row.cost_cr;
            }
            (Some((item, count)), Some(FillType::Mined)) => purse.hold.add(items, item, count),
            _ => {}
        }
        mirror
            .app
            .world_mut()
            .resource_mut::<Jobs>()
            .accept(job.clone());
        let mut lifted = None;
        match &job.kind {
            JobType::Procure { .. } => {}
            JobType::Supply { item, count, .. } => purse.hold.remove(item, *count),
            JobType::Recover {
                wreck, item, count, ..
            } => {
                // A job's own wreck is not mirrored: no accept runs here
                // to put it in the world, and no later draw can name it.
                if !job.spawns_target() {
                    let wreck = mirror.bodies[wreck];
                    mirror
                        .app
                        .world_mut()
                        .get_mut::<ShipInventory>(wreck)
                        .expect("a wreck carries its hold")
                        .remove(item, *count);
                }
                purse.hold.add(items, item, *count);
                lifted = Some((item, *count));
            }
            JobType::Neutralize { hull, .. } => {
                if !job.spawns_target() {
                    let hull = mirror.bodies[hull];
                    assert!(
                        mirror.app.world().get::<TraderMarket>(hull).is_some(),
                        "seed {}: the field hostile of '{}' trades",
                        world.seed,
                        job.id
                    );
                    mirror
                        .app
                        .world_mut()
                        .entity_mut(hull)
                        .insert(NeutralizedMarker);
                }
                hulls_taken += u64::from(takes_trading_hull(job));
            }
            JobType::Escort { .. } | JobType::Distress { .. } => {}
        }
        let mut jobs = mirror.app.world_mut().resource_mut::<Jobs>();
        if !matches!(job.kind, JobType::Procure { .. }) {
            jobs.mark_done(&job.id);
        }
        let plan = plan_job_hand_in(
            job,
            Some(&JobHandInPartner {
                id: counter.id.clone(),
                credits_cr: counter.credits,
            }),
            &purse.hold,
            purse.credits,
            jobs.is_done(&job.id),
        )
        .unwrap_or_else(|refusal| {
            panic!(
                "seed {}: the hand-in of '{}' is refused: {refusal:?}",
                world.seed, job.id
            )
        });
        if let Some((item, count)) = &plan.goods {
            purse.hold.remove(item, *count);
        }
        purse.credits = plan.own_cr;
        counter.credits = plan.station_cr;
        jobs.complete(&job.id);
        // A lifted stack would fill the hold against the next lift, so the
        // player sells it to the station when its market takes it.
        if let Some((item, count)) = lifted {
            let sale = plan_item_trade(
                items,
                ItemTradeType::Sell,
                counter
                    .trades
                    .then_some(TradePriceType::Market(quote(catalog, &counter, item))),
                counter.lawful,
                item,
                Some(count),
                &purse.hold,
                purse.credits,
                &counter.stock,
                counter.credits,
            );
            if let Ok(sale) = sale {
                purse.hold.remove(item, count);
                counter.stock.add(items, item, count);
                purse.credits += sale.price_cr;
                counter.credits -= sale.price_cr;
                sales_cr += sale.price_cr;
            }
        }
        let paper = row.paper_s.expect("a taken row has a paper time");
        clock += paper;
        first_payout_s.get_or_insert(clock);
        done.push(Done {
            kind: row.posted,
            net_cr: row.net_cr(),
            docked: paper <= 0.0,
        });
    };
    let goal = goal_count(
        catalog,
        role,
        mirror.app.world().resource::<Jobs>().completed(),
        hulls_taken,
    );
    Greedy {
        end_cr: purse.credits,
        done,
        sales_cr,
        first_payout_s,
        stopped_s: clock,
        goal,
        stop,
    }
}

fn measure_seed(catalog: &Catalog, seed: u32) -> Result<SeedMeasure, String> {
    let world = seed_world(catalog, seed)?;
    let start = start_facts(catalog, &world);
    let roles = (0..catalog.roles.len())
        .map(|role| {
            let (cells_live, field, rows, titles) =
                board_census(catalog, &world, role, start.advancement);
            let fitting: Vec<Job> = rows
                .iter()
                .filter(|row| row.fits)
                .map(|row| row.job.clone())
                .collect();
            RoleMeasure {
                cells_live,
                field,
                goal_all: goal_count(
                    catalog,
                    role,
                    &fitting,
                    fitting.iter().filter(|job| takes_trading_hull(job)).count() as u64,
                ),
                greedy: greedy(catalog, &world, role, start.advancement, 0.0),
                rows,
                titles,
            }
        })
        .collect();
    let hunts = catalog
        .roles
        .iter()
        .enumerate()
        .filter(|(_, role)| matches!(role.goal.kind, RoleGoalType::Outlaw { .. }))
        .map(|(role, _)| {
            let hull = &catalog.hulls[role];
            let mut hunt = hunt(hull, &world);
            hunt.split = hunt
                .two_back_s
                .filter(|head_s| head_s + DOCK_ALLOWANCE_S < HORIZON_S)
                .map(|head_s| {
                    let run = greedy(catalog, &world, role, start.advancement, head_s);
                    (
                        f64::from(run.end_cr) - f64::from(hull.credits),
                        run.done.len() as f64,
                    )
                });
            (role, hunt)
        })
        .collect();
    Ok(SeedMeasure {
        seed,
        start,
        roles,
        hunts,
    })
}

// --- The report -----------------------------------------------------------

fn sorted(mut values: Vec<f64>) -> Vec<f64> {
    values.sort_by(f64::total_cmp);
    values
}

/// The value at share `at` of `sorted`, by nearest rank.
fn quantile(sorted: &[f64], at: f64) -> f64 {
    sorted[((sorted.len() - 1) as f64 * at).round() as usize]
}

fn median(values: Vec<f64>) -> Option<f64> {
    let values = sorted(values);
    (!values.is_empty()).then(|| quantile(&values, 0.5))
}

/// `min / median / max` of shares in `[0, 1]`, to three decimals.
fn span_unit(values: Vec<f64>) -> String {
    let values = sorted(values);
    if values.is_empty() {
        return "-".to_string();
    }
    format!(
        "{:.3} / {:.3} / {:.3}",
        values[0],
        quantile(&values, 0.5),
        values[values.len() - 1]
    )
}

fn number(value: f64) -> String {
    if value.abs() >= 100.0 || value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

fn median_text(values: Vec<f64>) -> String {
    median(values).map_or_else(|| "-".to_string(), number)
}

/// `min / median / max`.
fn span(values: Vec<f64>) -> String {
    let values = sorted(values);
    if values.is_empty() {
        return "-".to_string();
    }
    format!(
        "{} / {} / {}",
        number(values[0]),
        number(quantile(&values, 0.5)),
        number(values[values.len() - 1])
    )
}

/// `lower quartile / median / upper quartile`.
fn quartiles(values: Vec<f64>) -> String {
    let values = sorted(values);
    if values.is_empty() {
        return "-".to_string();
    }
    format!(
        "{} / {} / {}",
        number(quantile(&values, 0.25)),
        number(quantile(&values, 0.5)),
        number(quantile(&values, 0.75))
    )
}

fn share(part: usize, whole: usize) -> String {
    if whole == 0 {
        "-".to_string()
    } else {
        format!("{:.1}%", 100.0 * part as f64 / whole as f64)
    }
}

fn kind_label(kind: JobKindType) -> &'static str {
    match kind {
        JobKindType::Procure => "Procure",
        JobKindType::Recover => "Recover",
        JobKindType::Escort => "Escort",
        JobKindType::Distress => "Distress",
        JobKindType::Neutralize => "Neutralize",
        JobKindType::Supply => "Supply",
    }
}

/// `Procure 41.2%, Recover 3.0%, ...` over `kinds`.
fn kind_shares(kinds: &[JobKindType]) -> String {
    KINDS
        .iter()
        .map(|kind| {
            format!(
                "{} {}",
                kind_label(*kind),
                share(kinds.iter().filter(|k| *k == kind).count(), kinds.len())
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The row a first board's best rate names, among the rows that pay.
fn best_row(rows: &[&Row]) -> Option<Row> {
    rows.iter()
        .filter(|row| row.pays())
        .max_by(|a, b| {
            a.rate_key()
                .partial_cmp(&b.rate_key())
                .expect("a rate is a number")
                .then(b.slot.cmp(&a.slot))
        })
        .map(|row| (*row).clone())
}

/// The row a first board pays out soonest, among the rows that pay.
fn quickest_row(rows: &[&Row]) -> Option<Row> {
    rows.iter()
        .filter(|row| row.pays())
        .min_by(|a, b| {
            a.paper_s
                .partial_cmp(&b.paper_s)
                .expect("a paper time is a number")
                .then(b.net_cr().cmp(&a.net_cr()))
                .then(a.slot.cmp(&b.slot))
        })
        .map(|row| (*row).clone())
}

/// One line of the cross-role summary.
/// The four numbers the roles are held even on.
struct Band {
    gain: f64,
    first_payout_s: Option<f64>,
    goal: f64,
    need: u64,
    no_job: f64,
}

struct Summary {
    band: Band,
    role: String,
    cells: Vec<String>,
}

const SUMMARY_HEAD: [&str; 22] = [
    "role",
    "hull cr",
    "kit stock cr",
    "credits",
    "accel m/s2",
    "hold kg",
    "weapons",
    "fire s",
    "rows that fit (mean)",
    "boards with 0 rows that fit",
    "Recover rows with a spawned target",
    "Neutralize rows with a spawned target",
    "first board has a row that pays",
    "first board has a goal row",
    "first board has a goal row that pays",
    "no goal row in 5 postings",
    "first payout s (median)",
    "greedy gain cr at 15 min (quartiles)",
    "greedy jobs (median)",
    "greedy seeds with no job",
    "greedy goal at 15 min (median / need)",
    "Escort and Neutralize share of greedy pay",
];

fn report_header(out: &mut String, catalog: &Catalog, measured: usize, refused: &[(u32, String)]) {
    let rule = catalog.table.jobs;
    let _ = writeln!(
        out,
        "# Start-role census\n\n\
         Written by `crates/nova_authoring/tests/start_role_census.rs`. Do not edit the tables \
         by hand: run the tool again.\n\n\
         Seeds {FIRST_SEED} to {}: {measured} measured, {} refused by `starting_station`{}. \
         {POSTINGS} postings of each of the {JOBS_PER_BOARD} slots, so {} board rows a seed and \
         role.\n",
        FIRST_SEED + SEED_COUNT - 1,
        refused.len(),
        if refused.is_empty() {
            String::new()
        } else {
            format!(
                " ({})",
                refused
                    .iter()
                    .map(|(seed, why)| format!("seed {seed}: {why}"))
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        },
        POSTINGS * JOBS_PER_BOARD,
    );
    let _ = writeln!(
        out,
        "## What is real and what is modelled\n\n\
         Real, called and not copied:\n\n\
         - Base content from the Rust builders: items, sections, ship designs, start roles, the \
         price table, trade profiles, and the base ship part packs.\n\
         - The world generator (`NovaLayeredWorld`) and the start pick (`starting_station`).\n\
         - The board pass `post_station_boards`, run as a system: station order, the 27-cell \
         cube gate, the field rule (`FieldBodies`), the bodies other boards name, the draw \
         (`station_job_offers`, `SlotDraw`), the target a Recover or a Neutralize names for \
         itself with none left in the field (its leg, its threat and the hold of its wreck, \
         `job_wreck`), the starter board's redraw for the hull, rewards and renewals.\n\
         - The hull gate (`PlayerHull::fit`, `plan_job_hull_fit`), the accept rule \
         (`plan_job_accept`), the hand-in (`plan_job_hand_in`), trades (`plan_item_trade`, \
         `price`), and the goal count (`goal_progress`).\n\
         - Hull numbers: `resolve_ship_design`, `ShipMeasure`, `hull_price_cr`, \
         `cargo_capacity_g`, item reference prices.\n\n\
         Modelled:\n\n\
         - No game runs. Each cell of the 125-cell window around the player's spawn is \
         mirrored from its real manifest: a root, each station (services, market, dead or \
         living) and each ship (hold, side, derelict or intact, live weapon sections). Rocks, \
         planets and parked sale hulls are not mirrored; the board pass reads none of them.\n\
         - Streaming: one cell a frame, nearest to the spawn cell first. The game prepares \
         cells on worker threads, so its order can differ between cells at equal distance.\n\
         - A row counts as fitting when the hull gate passes. The Jobs pane lists every row \
         and offers Accept on those (`board_listing` in `crates/nova_interface/src/jobs.rs`), \
         stated here and not called: this crate does not depend on the interface crate.\n\
         - A spawned target is not mirrored: no accept runs, so its wreck and its raider never \
         stand. The row carries the leg, the threat and the stack the board posted.\n\
         - Postings 2 to {POSTINGS} of a slot are window renewals: the play clock steps past \
         the longer offer window ({} s; Procure and the rest stand {} s, Escort {} s), no job \
         taken, so every board of the window renews, the start board whole. Kit, credits and \
         station stock stay as at the start for every posting.\n\
         - Visit postings, scenario postings, hails and encounters do not exist here. No \
         stance changes. No restock.\n\
         - Goal: a row counts when `goal_progress` over that one job alone moves the role's \
         own line, the first. Stations owned is 0. For a goal that reads hulls taken, a \
         Neutralize of a hull that stood in the field counts as one hull taken: every field \
         hostile is an intact generated ship and carries a market. A job's own raider carries \
         none and does not count. The cross-role goal count of such a role is a hunt-only \
         paper run: from the start station to the nearest trading hull of the window \
         (an intact ship with a market, at its manifest position), then the nearest left, one \
         flip and burn and one fight each, no dock, no job. The run that counts keeps its \
         dock: {OFFENCE_STEP} hulls of one civilization inside the offence window step it one \
         stance down and an Enemy station refuses the dock, so that run passes over a hull of \
         the start station's civilization once the next one would step it to Enemy from its \
         seeded stance. The free run, which takes the nearest hull whatever its side, is \
         listed beside it. A hunt earns nothing, so it is an upper bound beside the greedy \
         run, not a part of it. The split run joins the two: the first two hulls of the run \
         that keeps its dock, one flip and burn back to the start station, then the greedy \
         run on what is left of the horizon; its goal count is 2. Traffic moves after the manifest, the steps a kill passes to \
         a neighbour civilization are not applied, and a trading hull's own weapons and \
         escorts are not read.\n",
        rule.offer_seconds.max(rule.escort_offer_seconds) + 1.0,
        rule.offer_seconds,
        rule.escort_offer_seconds,
    );
    let _ = writeln!(
        out,
        "## Paper time\n\n\
         `t(d) = 2 * sqrt(d / a)`: flip and burn over `d` metres from rest to rest, `a` the \
         hull's acceleration. `a = thrust / (mass * tick) * 10 m`, thrust and mass from \
         `ShipMeasure` over the start hull's sections, tick {} s. Assumptions: every drive \
         pushes along the flight axis, cargo adds no mass, no speed limit, no turn time.\n\n\
         One dock costs {DOCK_ALLOWANCE_S} s (`DOCK_ALLOWANCE_S`, an assumption, not a \
         measurement).\n\n\
         | kind | paper seconds from accept to payout |\n\
         |---|---|\n\
         | Procure | 0 when the kit holds the stack or the station sells it and the player can \
         pay and stow it; else, for a hull with a mining beam, a stack of an ore a rock of \
         the station's cell yields and the free hold takes: 2 t(rock) + cut + 1 dock; else \
         not doable (trade elsewhere is not modelled) |\n\
         | Supply | 2 t(leg) + 2 docks, with the stack from the kit or the station; else not \
         doable |\n\
         | Recover | 2 t(leg) + 2 docks; not doable when the stack does not fit the free hold |\n\
         | Distress | 2 t(leg) + 2 docks |\n\
         | Escort | leg / {} m/s (the hauler's cruise, `HAULER_CRUISE`) + raiders x fight + \
         t(leg) + 1 dock |\n\
         | Neutralize | 2 t(leg) + fight + 1 dock |\n\n\
         Fight: {FIGHT_S_PER_HOSTILE} s a hostile hull (`FIGHT_S_PER_HOSTILE`, an assumption, \
         not a measurement). Content does not give it: a start-band raider's health over two \
         point-defence mounts' raw damage rate is a few seconds, and the measured ammunition \
         ledger (582 and 1,971 rounds for one raider) is 3 to 10 s of trigger time, so closing \
         and tracking bound a fight, and no content field states those. A hull with no weapon \
         section is refused such rows by the hull gate. No losses, no hull damage, no deaths \
         and no ammunition spent are modelled.\n\n\
         Mining: rock is the metres from the start station to the nearest asteroid of its cell \
         whose kind yields the ore (`ore_for_asteroid_kind`), less the rock's authored radius \
         and the beam's reach. The cut is {MINING_SETUP_S} s (`MINING_SETUP_S`, an assumption: \
         stop, open the emitter, aim, and the first pulse, which takes nothing) plus one pulse \
         interval for every pulse, each pulse yielding half the carve sphere's volume in cells \
         as items (one item a field corner; the beam meets a surface). The rendered rock is \
         larger than its authored radius, so the flight is an upper bound. A hull with no \
         beam cannot fill such a row.\n\n\
         Net pay is the reward less the goods bought at the station's ask; a stack \
         from the kit costs 0. A recovered stack stays with the player and is not in net pay.\n",
        Time::<Fixed>::default().timestep().as_secs_f64(),
        HAULER_CRUISE.0,
    );
    let _ = writeln!(
        out,
        "## Greedy 15-minute model\n\n\
         The player docks once at the start ({DOCK_ALLOWANCE_S} s). Then, until {HORIZON_S} \
         paper seconds: from the current board take the row with the best net pay per paper \
         second among the rows that fit the hull, pass the accept rule, can be done, pay more \
         than their goods, end inside the horizon and that the station has credits for. A row \
         that costs 0 s goes before any timed row, the larger net pay first. The job is \
         accepted, done, and handed in through the real plans; goods are bought through the \
         real trade plan and leave the station's stock; the station's credits pay the reward. \
         The handed-in slot renews on the next board pass, the other slots stand. A Recover \
         empties its stack off the wreck and the player sells the stack to the station at its \
         bid when the market takes it; that sale is listed apart. A Neutralize marks its hull \
         neutralized. A spawned target has no body here: its stack goes to the player and its \
         raider counts as neutralized. A run stops at {GREEDY_JOB_CAP} jobs. Escort and Neutralize income is \
         listed apart because the model gives them a flat fight time and no losses.\n"
    );
}

fn report_starts(out: &mut String, seeds: &[SeedMeasure]) {
    let starts: Vec<&StartFacts> = seeds.iter().map(|seed| &seed.start).collect();
    let n = starts.len();
    let of =
        |get: fn(&StartFacts) -> f64| starts.iter().map(|start| get(start)).collect::<Vec<_>>();
    let mut clusters: BTreeMap<&'static str, usize> = BTreeMap::new();
    for start in &starts {
        *clusters.entry(start.cluster.label()).or_default() += 1;
    }
    let _ = writeln!(
        out,
        "## Start stations\n\n\
         The start is the same for every role of a seed.\n\n\
         | fact | value |\n|---|---|\n\
         | seeds measured | {n} |\n\
         | cluster type of the start station's market | {} |\n\
         | civilization advancement (the draw's), min / median / max | {} |\n\
         | station design advancement, min / median / max | {} |\n\
         | station credits, min / median / max | {} |\n\
         | station stock stacks, min / median / max | {} |\n\
         | station stock at reference ask cr, min / median / max | {} |\n\
         | stations that offer Trade | {} |\n\
         | lawful civilizations | {} |\n\
         | player spawns in the start station's own cell | {} |\n\
         | stations in the 125-cell window, min / median / max | {} |\n\
         | of them living with a Jobs board | {} |\n\
         | ships in the 125-cell window, min / median / max | {} |\n",
        clusters
            .iter()
            .map(|(label, count)| format!("{label} {}", share(*count, n)))
            .collect::<Vec<_>>()
            .join(", "),
        span_unit(of(|start| f64::from(start.advancement))),
        span_unit(of(|start| f64::from(start.design_advancement))),
        span(of(|start| f64::from(start.credits))),
        span(of(|start| start.stock_stacks as f64)),
        span(of(|start| start.stock_cr as f64)),
        share(starts.iter().filter(|start| start.trades).count(), n),
        share(starts.iter().filter(|start| start.lawful).count(), n),
        share(
            starts
                .iter()
                .filter(|start| start.spawns_in_station_cell)
                .count(),
            n
        ),
        span(of(|start| start.window_stations as f64)),
        span(of(|start| start.window_board_stations as f64)),
        span(of(|start| start.window_ships as f64)),
    );
}

fn report_role(out: &mut String, catalog: &Catalog, role: usize, seeds: &[SeedMeasure]) -> Summary {
    let spec = &catalog.roles[role];
    let hull = &catalog.hulls[role];
    let n = seeds.len();
    let measures: Vec<&RoleMeasure> = seeds.iter().map(|seed| &seed.roles[role]).collect();
    let all: Vec<&Row> = measures.iter().flat_map(|m| &m.rows).collect();
    let first: Vec<&Row> = all.iter().copied().filter(|row| row.posting == 0).collect();
    let firsts: Vec<Vec<&Row>> = measures
        .iter()
        .map(|m| m.rows.iter().filter(|row| row.posting == 0).collect())
        .collect();
    let fire = hull
        .fire_seconds()
        .map_or_else(|| "-".to_string(), |seconds| number(f64::from(seconds)));

    let _ = writeln!(
        out,
        "## {} ({})\n\n\
         Hull `{}`, job mix {:?}, goal \"{}\" ({:?}).\n\n\
         | start worth | cr |\n|---|---|\n\
         | hull (every section's recipe at reference ask) | {} |\n\
         | kit stock at reference ask | {} |\n\
         | credits | {} |\n\
         | total | {} |\n",
        spec.id,
        spec.name,
        spec.hull,
        spec.jobs,
        spec.goal.title,
        spec.goal.kind,
        hull.hull_cr,
        hull.stock_cr,
        hull.credits,
        hull.hull_cr + hull.stock_cr + u64::from(hull.credits),
    );
    let _ = writeln!(
        out,
        "| hull | value |\n|---|---|\n\
         | sections | {} |\n\
         | mass, cells | {:.1} |\n\
         | thrust, impulse per tick | {:.2} |\n\
         | acceleration, m/s2 | {:.1} |\n\
         | t(10 km), s | {:.1} |\n\
         | health sum | {:.0} |\n\
         | hold, kg | {} |\n\
         | kit mass, kg | {} |\n\
         | docking sections | {} |\n\
         | turrets / torpedo bays / railguns / mining beams | {} / {} / {} / {} |\n\
         | PDC rounds in the kit | {} |\n\
         | PDC rounds in the magazines | {} |\n\
         | PDC rounds a second, every turret | {:.1} |\n\
         | fire seconds, kit and magazines | {fire} |\n",
        hull.sections,
        hull.mass,
        hull.thrust,
        hull.accel,
        hull.burn_s(10_000),
        hull.health,
        hull.hold_g / 1000,
        hull.kit_g / 1000,
        hull.clamps,
        hull.turrets,
        hull.torpedo_bays,
        hull.railguns,
        hull.miners,
        hull.pdc_kit_rounds,
        hull.pdc_magazine_rounds,
        hull.pdc_rounds_per_second,
    );

    // The first board.
    let fitting: Vec<usize> = firsts
        .iter()
        .map(|rows| rows.iter().filter(|row| row.fits).count())
        .collect();
    let fit_mean = fitting.iter().sum::<usize>() as f64 / n as f64;
    let fit_share = |count: usize| share(fitting.iter().filter(|fit| **fit == count).count(), n);
    let with =
        |list: fn(&FieldSeen) -> usize| measures.iter().filter(|m| list(&m.field) > 0).count();
    let nearest = |list: fn(&FieldSeen) -> Option<u32>| {
        median_text(
            measures
                .iter()
                .filter_map(|m| list(&m.field))
                .map(f64::from)
                .collect(),
        )
    };
    let counts = |list: fn(&FieldSeen) -> usize| {
        median_text(
            measures
                .iter()
                .map(|m| list(&m.field))
                .filter(|count| *count > 0)
                .map(|count| count as f64)
                .collect(),
        )
    };
    let _ = writeln!(
        out,
        "| first board | value |\n|---|---|\n\
         | rows that fit, mean of 3 | {fit_mean:.2} |\n\
         | boards with 0 / 1 / 2 / 3 rows that fit | {} / {} / {} / {} |\n\
         | cells live when the board posts, min / median / max | {} |\n\
         | seeds with a wreck the draw can name | {} |\n\
         | wrecks when any, median; nearest wreck m, median | {}; {} |\n\
         | seeds with a hostile the draw can name | {} |\n\
         | hostiles when any, median; nearest hostile m, median | {}; {} |\n\
         | seeds with a wreck in the cube, named by another board or not | {} |\n\
         | seeds with a hostile in the cube, named by another board or not | {} |\n",
        fit_share(0),
        fit_share(1),
        fit_share(2),
        fit_share(3),
        span(measures.iter().map(|m| m.cells_live as f64).collect()),
        share(with(|field| field.seen.wrecks.len()), n),
        counts(|field| field.seen.wrecks.len()),
        nearest(|field| field.seen.wrecks.iter().map(|wreck| wreck.leg_m).min()),
        share(with(|field| field.seen.hostiles.len()), n),
        counts(|field| field.seen.hostiles.len()),
        nearest(|field| field.seen.hostiles.iter().map(|hull| hull.leg_m).min()),
        share(with(|field| field.whole.wrecks.len()), n),
        share(with(|field| field.whole.hostiles.len()), n),
    );

    // Kinds.
    let kinds_of = |rows: &[&Row], get: fn(&Row) -> JobKindType| -> Vec<JobKindType> {
        rows.iter().map(|row| get(row)).collect()
    };
    let _ = writeln!(
        out,
        "| kind share | value |\n|---|---|\n\
         | drawn, {POSTINGS} postings | {} |\n\
         | posted, first board | {} |\n\
         | posted, {POSTINGS} postings | {} |\n\
         | posted and fit, first board | {} |\n\
         | posted and fit, {POSTINGS} postings | {} |\n",
        kind_shares(&kinds_of(&all, |row| row.drawn)),
        kind_shares(&kinds_of(&first, |row| row.posted)),
        kind_shares(&kinds_of(&all, |row| row.posted)),
        kind_shares(&kinds_of(
            &first
                .iter()
                .copied()
                .filter(|row| row.fits)
                .collect::<Vec<_>>(),
            |row| row.posted
        )),
        kind_shares(&kinds_of(
            &all.iter()
                .copied()
                .filter(|row| row.fits)
                .collect::<Vec<_>>(),
            |row| row.posted
        )),
    );
    let spawned = |kind: JobKindType| {
        let posted = all.iter().filter(|row| row.posted == kind).count();
        let spawned = all
            .iter()
            .filter(|row| row.posted == kind && row.spawned)
            .count();
        (
            share(spawned, posted),
            format!("{spawned} of {posted} ({})", share(spawned, posted)),
        )
    };
    let (recover_spawned, recover_line) = spawned(JobKindType::Recover);
    let (neutralize_spawned, neutralize_line) = spawned(JobKindType::Neutralize);
    let redrawn = all.iter().filter(|row| row.redrawn).count();
    let _ = writeln!(
        out,
        "| draw outcome, {POSTINGS} postings | value |\n|---|---|\n\
         | Recover rows with a spawned target | {recover_line} |\n\
         | Neutralize rows with a spawned target | {neutralize_line} |\n\
         | rows the starter redraw posted as another kind | {redrawn} of {} ({}) |\n",
        all.len(),
        share(redrawn, all.len()),
    );

    // Per kind.
    let _ = writeln!(
        out,
        "Rows by posted kind, {POSTINGS} postings. Reward, cost, net pay, paper seconds, leg and \
         threat are min / median / max; cost, net pay and paper seconds over the rows that fit \
         and can be done.\n\n\
         | kind | posted | fit | doable | pay | goal | reward cr | goods cost cr | net pay cr | \
         paper s | leg m | threat |\n|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    let line = |label: &str, rows: &[&Row]| {
        let doable: Vec<&&Row> = rows.iter().filter(|row| row.doable()).collect();
        format!(
            "| {label} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            rows.len(),
            rows.iter().filter(|row| row.fits).count(),
            doable.len(),
            rows.iter().filter(|row| row.pays()).count(),
            rows.iter().filter(|row| row.goal).count(),
            span(
                rows.iter()
                    .map(|row| f64::from(row.job.reward.total_cr()))
                    .collect()
            ),
            span(doable.iter().map(|row| f64::from(row.cost_cr)).collect()),
            span(doable.iter().map(|row| row.net_cr() as f64).collect()),
            span(doable.iter().filter_map(|row| row.paper_s).collect()),
            span(rows.iter().map(|row| f64::from(row.job.leg_m)).collect()),
            span(rows.iter().map(|row| f64::from(row.job.threat)).collect()),
        )
    };
    for kind in KINDS {
        let rows: Vec<&Row> = all
            .iter()
            .copied()
            .filter(|row| row.posted == kind)
            .collect();
        let _ = writeln!(out, "{}", line(kind_label(kind), &rows));
    }
    let _ = writeln!(out, "{}\n", line("all", &all));

    // Goods.
    let _ = writeln!(
        out,
        "| goods of the rows that fit, {POSTINGS} postings | Procure | Supply |\n|---|---|---|"
    );
    let goods_rows = |kind: JobKindType| -> Vec<(&Row, &Goods)> {
        all.iter()
            .filter(|row| row.posted == kind && row.fits)
            .filter_map(|row| row.goods.as_ref().map(|goods| (*row, goods)))
            .collect()
    };
    let (procure, supply) = (
        goods_rows(JobKindType::Procure),
        goods_rows(JobKindType::Supply),
    );
    let goods_line = |label: &str, cell: &dyn Fn(&[(&Row, &Goods)]) -> String| {
        format!("| {label} | {} | {} |", cell(&procure), cell(&supply))
    };
    let of = |rows: &[(&Row, &Goods)], keep: fn(&Goods) -> bool| {
        share(
            rows.iter().filter(|(_, goods)| keep(goods)).count(),
            rows.len(),
        )
    };
    let _ = writeln!(
        out,
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n",
        goods_line("rows", &|rows| rows.len().to_string()),
        goods_line("kit holds the stack", &|rows| of(rows, |goods| goods
            .kit_holds)),
        goods_line("station stocks the stack", &|rows| of(rows, |goods| {
            goods.station_stocks
        })),
        goods_line("filled from the kit", &|rows| of(rows, |goods| {
            goods.fill == FillType::Kit
        })),
        goods_line("bought at the station", &|rows| of(rows, |goods| {
            goods.fill == FillType::Station
        })),
        goods_line("cut with the hull's beam", &|rows| of(rows, |goods| {
            goods.fill == FillType::Mined
        })),
        goods_line("not to be had", &|rows| of(rows, |goods| {
            goods.fill == FillType::Short
        })),
        goods_line(
            "stack at the station's ask cr, min / median / max",
            &|rows| span(rows.iter().map(|(_, goods)| goods.ask_cr as f64).collect())
        ),
    );
    let lifts: Vec<&&Row> = all
        .iter()
        .filter(|row| row.posted == JobKindType::Recover && row.fits)
        .collect();
    let _ = writeln!(
        out,
        "Recover rows that fit: {}; stack has room in the free hold: {}; the stack at the \
         station's bid, min / median / max: {} cr (kept by the player, not in net pay).\n",
        lifts.len(),
        share(
            lifts.iter().filter(|row| row.paper_s.is_some()).count(),
            lifts.len()
        ),
        span(lifts.iter().map(|row| row.kept_cr as f64).collect()),
    );

    // Best and quickest row of the first board.
    let best: Vec<Row> = firsts.iter().filter_map(|rows| best_row(rows)).collect();
    let quickest: Vec<Row> = firsts
        .iter()
        .filter_map(|rows| quickest_row(rows))
        .collect();
    let _ = writeln!(
        out,
        "| first board, the rows that pay | value |\n|---|---|\n\
         | seeds with a row that pays | {} |\n\
         | best row by net pay per paper minute: net pay cr, median | {} |\n\
         | best row: paper minutes, median | {} |\n\
         | best row costs 0 s (a docked Procure) | {} |\n\
         | best row kind | {} |\n\
         | quickest row: paper seconds, median | {} |\n\
         | quickest row: seconds to first payout with the start dock, median | {} |\n\
         | quickest row: net pay cr, median | {} |\n",
        share(best.len(), n),
        median_text(best.iter().map(|row| row.net_cr() as f64).collect()),
        median_text(
            best.iter()
                .filter_map(|row| row.paper_s)
                .map(|seconds| (seconds / 6.0).round() / 10.0)
                .collect()
        ),
        share(
            best.iter()
                .filter(|row| row.paper_s.is_some_and(|seconds| seconds <= 0.0))
                .count(),
            best.len()
        ),
        kind_shares(&best.iter().map(|row| row.posted).collect::<Vec<_>>()),
        median_text(quickest.iter().filter_map(|row| row.paper_s).collect()),
        median_text(
            quickest
                .iter()
                .filter_map(|row| row.paper_s)
                .map(|seconds| seconds + DOCK_ALLOWANCE_S)
                .collect()
        ),
        median_text(quickest.iter().map(|row| row.net_cr() as f64).collect()),
    );

    // Goal.
    let goal_first = firsts
        .iter()
        .filter(|rows| rows.iter().any(|row| row.fits && row.goal))
        .count();
    let goal_never = measures
        .iter()
        .filter(|m| !m.rows.iter().any(|row| row.fits && row.goal))
        .count();
    let goal_first_pays = firsts
        .iter()
        .filter(|rows| rows.iter().any(|row| row.pays() && row.goal))
        .count();
    let goal_rows = measures
        .iter()
        .flat_map(|m| &m.rows)
        .filter(|row| row.fits && row.goal);
    let goal_fit = goal_rows.clone().count();
    let goal_pay = goal_rows.filter(|row| row.pays()).count();
    let need = measures.first().map_or(0, |m| m.goal_all.1);
    let _ = writeln!(
        out,
        "| goal, rows that fit | value |\n|---|---|\n\
         | the goal asks for | {need} |\n\
         | seeds whose first board has a goal row | {} |\n\
         | seeds whose first board has a goal row that pays | {} |\n\
         | goal rows that pay, of the goal rows that fit, {POSTINGS} postings | {goal_pay} of {goal_fit} ({}) |\n\
         | seeds with no goal row in {POSTINGS} postings | {} |\n\
         | goal rows per {} board rows, lower quartile / median / upper quartile | {} |\n\
         | goal count with every fitting row of the {} handed in, quartiles | {} |\n",
        share(goal_first, n),
        share(goal_first_pays, n),
        share(goal_pay, goal_fit),
        share(goal_never, n),
        POSTINGS * JOBS_PER_BOARD,
        quartiles(
            measures
                .iter()
                .map(|m| m.rows.iter().filter(|row| row.fits && row.goal).count() as f64)
                .collect()
        ),
        POSTINGS * JOBS_PER_BOARD,
        quartiles(measures.iter().map(|m| m.goal_all.0 as f64).collect()),
    );

    // Greedy.
    let runs: Vec<&Greedy> = measures.iter().map(|m| &m.greedy).collect();
    let gain = |run: &Greedy| f64::from(run.end_cr) - f64::from(hull.credits);
    let total: i64 = runs
        .iter()
        .flat_map(|run| &run.done)
        .map(|done| done.net_cr)
        .sum();
    let net_share = |keep: &dyn Fn(&Done) -> bool| {
        let part: i64 = runs
            .iter()
            .flat_map(|run| &run.done)
            .filter(|done| keep(done))
            .map(|done| done.net_cr)
            .sum();
        if total == 0 {
            "-".to_string()
        } else {
            format!("{:.1}%", 100.0 * part as f64 / total as f64)
        }
    };
    let mut stops: BTreeMap<StopType, usize> = BTreeMap::new();
    for run in &runs {
        *stops.entry(run.stop).or_default() += 1;
    }
    let fight =
        net_share(&|done| matches!(done.kind, JobKindType::Escort | JobKindType::Neutralize));
    let _ = writeln!(
        out,
        "| greedy 15-minute model | value |\n|---|---|\n\
         | credits at the start | {} |\n\
         | credits at 15 min, quartiles | {} |\n\
         | gain cr at 15 min, quartiles | {} |\n\
         | gain cr at 15 min, min / median / max | {} |\n\
         | jobs handed in, quartiles | {} |\n\
         | seeds with no job handed in | {} |\n\
         | first payout, paper s, quartiles | {} |\n\
         | last payout or stop, paper s, quartiles | {} |\n\
         | goal count at 15 min of {need}, quartiles | {} |\n\
         | net job pay by kind | {} |\n\
         | Escort and Neutralize share of net job pay | {fight} |\n\
         | docked 0 s rows: share of jobs / of net job pay | {} / {} |\n\
         | sale of recovered stacks cr, quartiles | {} |\n\
         | why the run stopped | {} |\n",
        hull.credits,
        quartiles(runs.iter().map(|run| f64::from(run.end_cr)).collect()),
        quartiles(runs.iter().map(|run| gain(run)).collect()),
        span(runs.iter().map(|run| gain(run)).collect()),
        quartiles(runs.iter().map(|run| run.done.len() as f64).collect()),
        share(runs.iter().filter(|run| run.done.is_empty()).count(), n),
        quartiles(runs.iter().filter_map(|run| run.first_payout_s).collect()),
        quartiles(runs.iter().map(|run| run.stopped_s).collect()),
        quartiles(runs.iter().map(|run| run.goal.0 as f64).collect()),
        KINDS
            .iter()
            .map(|kind| format!(
                "{} {}",
                kind_label(*kind),
                net_share(&|done| done.kind == *kind)
            ))
            .collect::<Vec<_>>()
            .join(", "),
        share(
            runs.iter()
                .flat_map(|run| &run.done)
                .filter(|done| done.docked)
                .count(),
            runs.iter().map(|run| run.done.len()).sum()
        ),
        net_share(&|done| done.docked),
        quartiles(runs.iter().map(|run| f64::from(run.sales_cr)).collect()),
        stops
            .iter()
            .map(|(stop, count)| format!("{} {}", stop.label(), share(*count, n)))
            .collect::<Vec<_>>()
            .join(", "),
    );

    // A goal that counts hulls taken is measured by the hunt-only run: the
    // board posts no row for it unless a hostile stands in the field.
    let hunted: Vec<Hunt> = seeds
        .iter()
        .flat_map(|seed| &seed.hunts)
        .filter(|(hunter, _)| *hunter == role)
        .map(|(_, hunt)| *hunt)
        .collect();
    if !hunted.is_empty() {
        let of = |get: &dyn Fn(&Hunt) -> Option<f64>| -> Vec<f64> {
            hunted.iter().filter_map(|hunt| get(hunt)).collect()
        };
        let _ = writeln!(
            out,
            "| hunt-only paper run | value |\n|---|---|\n\
             | trading hulls in the window, quartiles | {} |\n\
             | of those, hulls of the start station's civilization, quartiles | {} |\n\
             | metres to the nearest, quartiles | {} |\n\
             | hulls taken inside {HORIZON_S} s, quartiles | {} |\n\
             | of those, hulls of the start station's civilization, quartiles | {} |\n\
             | seeds with no hull taken | {} |\n\
             | paper s at the third hull, quartiles | {} |\n\
             | seeds with fewer than three trading hulls in the window | {} |\n\
             | hulls taken inside {HORIZON_S} s by a run that keeps its dock, quartiles | {} |\n\
             | that run: seeds with 0 / 1 / 2 / 3 hulls taken | {} |\n\
             | that run: paper s at the third hull, quartiles | {} |\n\
             | that run: seeds with no third hull to take | {} |\n\
             | split run, two hulls then the board: paper s back at the station, quartiles | {} |\n\
             | split run: gain cr at 15 min, quartiles | {} |\n\
             | split run: jobs handed in, quartiles | {} |\n\
             | split run: seeds with no second hull to take | {} |\n\
             | board goal count at 15 min in the greedy run, quartiles | {} |\n",
            quartiles(of(&|hunt| Some(hunt.prey as f64))),
            quartiles(of(&|hunt| Some(hunt.prey_home as f64))),
            quartiles(of(&|hunt| hunt.nearest_m)),
            quartiles(of(&|hunt| Some(f64::from(hunt.taken)))),
            quartiles(of(&|hunt| Some(f64::from(hunt.taken_home)))),
            share(hunted.iter().filter(|hunt| hunt.taken == 0).count(), n),
            quartiles(of(&|hunt| hunt.three_s)),
            share(
                hunted.iter().filter(|hunt| hunt.three_s.is_none()).count(),
                n
            ),
            quartiles(of(&|hunt| Some(f64::from(hunt.kept_taken)))),
            (0..=3)
                .map(|count| share(
                    hunted
                        .iter()
                        .filter(|hunt| hunt.kept_taken == count)
                        .count(),
                    n
                ))
                .collect::<Vec<_>>()
                .join(" / "),
            quartiles(of(&|hunt| hunt.kept_three_s)),
            share(
                hunted
                    .iter()
                    .filter(|hunt| hunt.kept_three_s.is_none())
                    .count(),
                n
            ),
            quartiles(of(&|hunt| hunt.two_back_s)),
            quartiles(of(&|hunt| hunt.split.map(|(gain, _)| gain))),
            quartiles(of(&|hunt| hunt.split.map(|(_, jobs)| jobs))),
            share(hunted.iter().filter(|hunt| hunt.split.is_none()).count(), n),
            quartiles(runs.iter().map(|run| run.goal.0 as f64).collect()),
        );
    }
    let goal_runs: Vec<f64> = if hunted.is_empty() {
        runs.iter().map(|run| run.goal.0 as f64).collect()
    } else {
        hunted
            .iter()
            .map(|hunt| f64::from(hunt.kept_taken))
            .collect()
    };
    let first_payout: Vec<f64> = quickest
        .iter()
        .filter_map(|row| row.paper_s)
        .map(|seconds| seconds + DOCK_ALLOWANCE_S)
        .collect();

    Summary {
        band: Band {
            gain: median(runs.iter().map(|run| gain(run)).collect()).unwrap_or(0.0),
            first_payout_s: median(first_payout.clone()),
            goal: median(goal_runs.clone()).unwrap_or(0.0),
            need,
            no_job: runs.iter().filter(|run| run.done.is_empty()).count() as f64 / n as f64,
        },
        role: spec.id.to_string(),
        cells: vec![
            hull.hull_cr.to_string(),
            hull.stock_cr.to_string(),
            hull.credits.to_string(),
            format!("{:.1}", hull.accel),
            (hull.hold_g / 1000).to_string(),
            hull.weapons().to_string(),
            fire,
            format!("{fit_mean:.2}"),
            fit_share(0),
            recover_spawned,
            neutralize_spawned,
            share(best.len(), n),
            share(goal_first, n),
            share(goal_first_pays, n),
            share(goal_never, n),
            median_text(first_payout),
            quartiles(runs.iter().map(|run| gain(run)).collect()),
            median_text(runs.iter().map(|run| run.done.len() as f64).collect()),
            share(runs.iter().filter(|run| run.done.is_empty()).count(), n),
            format!("{} / {need}", median_text(goal_runs)),
            fight,
        ],
    }
}

/// The share of the six-role median a role may stand at, low and high.
const BAND: (f64, f64) = (0.75, 1.33);
/// The share of seeds with no job handed in a role may have.
const NO_JOB_MAX: f64 = 0.05;

/// The four even-ness checks over the role medians, each cell marked `ok`
/// or `OUT`.
fn report_bands(out: &mut String, summaries: &[Summary]) {
    // The true median: the mean of the two middle roles of six.
    let mid = |values: Vec<f64>| {
        let values = sorted(values);
        match values.len() {
            0 => 0.0,
            n if n % 2 == 1 => values[n / 2],
            n => (values[n / 2 - 1] + values[n / 2]) / 2.0,
        }
    };
    let gain = mid(summaries.iter().map(|s| s.band.gain).collect());
    let first = mid(summaries
        .iter()
        .filter_map(|s| s.band.first_payout_s)
        .collect());
    let mark = |ok: bool| if ok { "ok" } else { "OUT" };
    let in_band = |value: f64, centre: f64| value >= BAND.0 * centre && value <= BAND.1 * centre;
    let _ = writeln!(
        out,
        "\n## Band check\n\n\
         A role median is in band between {} and {} times the median of the six role medians: \
         gain {gain:.0} cr, first payout {first:.0} s. The goal count is in band at 1 or 2. At \
         most {:.0}% of seeds may hand in no job.\n\n\
         | role | median gain cr at 15 min | x median | | median first payout s | x median | | \
         median goal count at 15 min | | seeds with no job | |\n\
         |---|---|---|---|---|---|---|---|---|---|---|",
        BAND.0,
        BAND.1,
        100.0 * NO_JOB_MAX,
    );
    for summary in summaries {
        let band = &summary.band;
        let payout = band.first_payout_s.unwrap_or(f64::INFINITY);
        let _ = writeln!(
            out,
            "| {} | {:.0} | {:.2} | {} | {:.0} | {:.2} | {} | {} / {} | {} | {:.1}% | {} |",
            summary.role,
            band.gain,
            band.gain / gain,
            mark(in_band(band.gain, gain)),
            payout,
            payout / first,
            mark(in_band(payout, first)),
            number(band.goal),
            band.need,
            mark((1.0..=2.0).contains(&band.goal)),
            100.0 * band.no_job,
            mark(band.no_job <= NO_JOB_MAX),
        );
    }
}

fn report_shown(out: &mut String, catalog: &Catalog, seeds: &[SeedMeasure]) {
    let _ = writeln!(
        out,
        "## First boards of the shown seeds\n\n\
         To compare with the line `the station '<id>' posts 3 offer(s) over W wreck(s) and H \
         hostile(s): ...` a live run logs for the same seed and role.\n\n\
         | seed | role | station | wrecks | hostiles | slot 0 | slot 1 | slot 2 |\n\
         |---|---|---|---|---|---|---|---|"
    );
    for seed in seeds.iter().filter(|seed| SHOWN_SEEDS.contains(&seed.seed)) {
        for (role, measure) in seed.roles.iter().enumerate() {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} |",
                seed.seed,
                catalog.roles[role].id,
                seed.start.station,
                measure.field.seen.wrecks.len(),
                measure.field.seen.hostiles.len(),
                measure.titles.join(" | "),
            );
        }
    }
    let _ = writeln!(out);
}

/// Every board row of every seed and role, one tab-separated line each.
fn rows_dump(catalog: &Catalog, seeds: &[SeedMeasure]) -> String {
    let mut out = String::from(
        "seed\trole\tposting\tslot\tdrawn\tposted\tspawned_target\tredrawn\tfits\tgoal\t\
         leg_m\tthreat\tbase_cr\tgoods_cr\trange_cr\tthreat_cr\treward_cr\tkit_holds\t\
         station_stocks\tstack_at_ask_cr\tfill\tgoods_cost_cr\tnet_cr\tpaper_s\ttitle\n",
    );
    let flag = |value: Option<bool>| value.map_or("-", |value| if value { "1" } else { "0" });
    for seed in seeds {
        for (role, measure) in seed.roles.iter().enumerate() {
            for row in &measure.rows {
                let reward = &row.job.reward;
                let goods = row.goods.as_ref();
                let _ = writeln!(
                    out,
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t\
                     {}\t{}\t{}\t{}\t{}",
                    seed.seed,
                    catalog.roles[role].id,
                    row.posting,
                    row.slot,
                    kind_label(row.drawn),
                    kind_label(row.posted),
                    flag(Some(row.spawned)),
                    flag(Some(row.redrawn)),
                    flag(Some(row.fits)),
                    flag(Some(row.goal)),
                    row.job.leg_m,
                    row.job.threat,
                    reward.base_cr,
                    reward.goods_cr,
                    reward.range_cr,
                    reward.threat_cr,
                    reward.total_cr(),
                    flag(goods.map(|goods| goods.kit_holds)),
                    flag(goods.map(|goods| goods.station_stocks)),
                    goods.map_or_else(|| "-".to_string(), |goods| goods.ask_cr.to_string()),
                    goods.map_or_else(|| "-".to_string(), |goods| format!("{:?}", goods.fill)),
                    row.cost_cr,
                    row.net_cr(),
                    row.paper_s
                        .map_or_else(|| "-".to_string(), |seconds| format!("{seconds:.0}")),
                    row.job.title(&catalog.items),
                );
            }
        }
    }
    out
}

/// What each start role is given and offered over the seed list.
#[test]
#[ignore = "about two minutes: generates the 125 cells around every start; run command in the file doc"]
fn every_start_role_is_measured_over_the_seed_list() {
    let catalog = catalog();
    let seeds: Vec<u32> = (FIRST_SEED..FIRST_SEED + SEED_COUNT).collect();
    let workers = std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .min(WORKERS_MAX);
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<Result<SeedMeasure, String>>>> =
        Mutex::new(seeds.iter().map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let at = next.fetch_add(1, Ordering::Relaxed);
                let Some(seed) = seeds.get(at) else {
                    break;
                };
                let measure = measure_seed(&catalog, *seed);
                results.lock().expect("no worker panicked with the lock")[at] = Some(measure);
            });
        }
    });
    let mut measured = Vec::new();
    let mut refused = Vec::new();
    for (seed, result) in seeds
        .iter()
        .zip(results.into_inner().expect("every worker is done"))
    {
        match result.expect("every seed was taken by a worker") {
            Ok(measure) => measured.push(measure),
            Err(why) => refused.push((*seed, why)),
        }
    }
    assert!(
        !measured.is_empty(),
        "no seed has a start station: {refused:?}"
    );

    let mut out = String::new();
    report_header(&mut out, &catalog, measured.len(), &refused);
    report_starts(&mut out, &measured);
    let summaries: Vec<Summary> = (0..catalog.roles.len())
        .map(|role| report_role(&mut out, &catalog, role, &measured))
        .collect();
    report_shown(&mut out, &catalog, &measured);
    let _ = writeln!(
        out,
        "## Cross-role summary\n\n| {} |\n|{}|",
        SUMMARY_HEAD.join(" | "),
        "---|".repeat(SUMMARY_HEAD.len()).trim_end_matches('|'),
    );
    for summary in &summaries {
        let _ = writeln!(out, "| {} | {} |", summary.role, summary.cells.join(" | "));
    }
    report_bands(&mut out, &summaries);
    print!("{out}");
    if let Ok(path) = std::env::var("NOVA_CENSUS_ROWS") {
        std::fs::write(&path, rows_dump(&catalog, &measured))
            .unwrap_or_else(|fault| panic!("the census writes '{path}': {fault}"));
        eprintln!("start role census: rows written to {path}");
    }
    if let Ok(path) = std::env::var("NOVA_CENSUS_OUT") {
        std::fs::write(&path, &out)
            .unwrap_or_else(|fault| panic!("the census writes '{path}': {fault}"));
        eprintln!("start role census: written to {path}");
    }
    assert_eq!(
        summaries.len(),
        6,
        "the census measures the six start roles: {:?}",
        summaries.iter().map(|s| &s.role).collect::<Vec<_>>()
    );
}
