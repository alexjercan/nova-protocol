//! The gallery's mock data: jobs, the charted sectors, wares, roles and
//! saves. Every page reads from here; the clicks move copies held in the
//! state resources of the root file.

use crate::icons::IconType;

/// The five job kinds of the proposed board.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JobKind {
    Bring,
    Recover,
    Escort,
    Answer,
    Neutralize,
}

impl JobKind {
    pub fn label(self) -> &'static str {
        match self {
            JobKind::Bring => "Bring",
            JobKind::Recover => "Recover",
            JobKind::Escort => "Escort",
            JobKind::Answer => "Answer",
            JobKind::Neutralize => "Neutralize",
        }
    }

    pub fn icon(self) -> IconType {
        match self {
            JobKind::Bring => IconType::Bring,
            JobKind::Recover => IconType::Recover,
            JobKind::Escort => IconType::Escort,
            JobKind::Answer => IconType::Answer,
            JobKind::Neutralize => IconType::Neutralize,
        }
    }
}

/// One job of the mock board. `have`/`need` are the held progress, `done` is
/// the world's own mark, and `hand_in_here` says the docked station is the
/// target.
#[derive(Clone, Copy)]
pub struct JobCard {
    pub id: u32,
    pub kind: JobKind,
    pub title: &'static str,
    pub reward: u32,
    pub target: (i32, i32),
    pub target_name: &'static str,
    pub minutes_left: u32,
    pub window: u32,
    pub fits_role: bool,
    pub poster: &'static str,
    pub hand_in: &'static str,
    pub task: &'static str,
    pub have: u32,
    pub need: u32,
    pub done: bool,
    pub hand_in_here: bool,
}

pub const JOBS: [JobCard; 10] = [
    JobCard {
        id: 1,
        kind: JobKind::Bring,
        title: "Bring 12 t iron ore",
        reward: 1_800,
        target: (3, 3),
        target_name: "Dust Gate",
        minutes_left: 14,
        window: 30,
        fits_role: true,
        poster: "Dust Gate yard office",
        hand_in: "Dust Gate",
        task: "Bring 12 t of iron ore to Dust Gate. The yard pays on the dock; ore from any belt counts.",
        have: 0,
        need: 12,
        done: false,
        hand_in_here: true,
    },
    JobCard {
        id: 2,
        kind: JobKind::Recover,
        title: "Recover a drive stack",
        reward: 2_400,
        target: (5, 2),
        target_name: "Reef Yard belt",
        minutes_left: 6,
        window: 30,
        fits_role: false,
        poster: "Reef Yard salvage desk",
        hand_in: "Dust Gate",
        task: "A drive stack drifts in the Reef Yard belt. Cut it free, take it aboard and hand it in here.",
        have: 0,
        need: 1,
        done: false,
        hand_in_here: true,
    },
    JobCard {
        id: 3,
        kind: JobKind::Escort,
        title: "Escort hauler to Reef Yard",
        reward: 3_100,
        target: (1, 5),
        target_name: "Reef Yard",
        minutes_left: 22,
        window: 30,
        fits_role: true,
        poster: "Driftco freight",
        hand_in: "Reef Yard",
        task: "Fly beside the Driftco hauler to Reef Yard. Raiders wait at the halfway mark.",
        have: 0,
        need: 1,
        done: false,
        hand_in_here: false,
    },
    JobCard {
        id: 4,
        kind: JobKind::Answer,
        title: "Answer a distress call",
        reward: 900,
        target: (2, 1),
        target_name: "Ash orbit",
        minutes_left: 3,
        window: 20,
        fits_role: false,
        poster: "Unknown caller",
        hand_in: "Dust Gate",
        task: "A caller in Ash orbit asks for a tow. The board cannot say whether it is a trap.",
        have: 0,
        need: 1,
        done: false,
        hand_in_here: true,
    },
    JobCard {
        id: 5,
        kind: JobKind::Neutralize,
        title: "Neutralize a raider hull",
        reward: 4_600,
        target: (6, 6),
        target_name: "Far belt",
        minutes_left: 18,
        window: 30,
        fits_role: false,
        poster: "Dust Gate watch",
        hand_in: "Dust Gate",
        task: "A raider corvette holds the far belt. Neutralize it and report back on the dock.",
        have: 0,
        need: 1,
        done: false,
        hand_in_here: true,
    },
    JobCard {
        id: 6,
        kind: JobKind::Bring,
        title: "Bring 4 t water ice",
        reward: 700,
        target: (3, 3),
        target_name: "Dust Gate",
        minutes_left: 0,
        window: 30,
        fits_role: true,
        poster: "Dust Gate galley",
        hand_in: "Dust Gate",
        task: "Bring 4 t of water ice to the galley. This offer has expired.",
        have: 0,
        need: 4,
        done: false,
        hand_in_here: true,
    },
    JobCard {
        id: 7,
        kind: JobKind::Neutralize,
        title: "Neutralize the trap caller",
        reward: 2_200,
        target: (0, 0),
        target_name: "Corner drift",
        minutes_left: 9,
        window: 30,
        fits_role: false,
        poster: "Driftco freight",
        hand_in: "Dust Gate",
        task: "A fake distress caller baits haulers in the corner drift. Put it down.",
        have: 0,
        need: 1,
        done: false,
        hand_in_here: true,
    },
    JobCard {
        id: 8,
        kind: JobKind::Bring,
        title: "Bring 8 t copper",
        reward: 1_200,
        target: (3, 3),
        target_name: "Dust Gate",
        minutes_left: 11,
        window: 30,
        fits_role: true,
        poster: "Dust Gate yard office",
        hand_in: "Dust Gate",
        task: "Bring 8 t of copper to Dust Gate.",
        have: 8,
        need: 8,
        done: true,
        hand_in_here: true,
    },
    JobCard {
        id: 9,
        kind: JobKind::Recover,
        title: "Recover 3 crates of parts",
        reward: 2_000,
        target: (2, 4),
        target_name: "Wreck WRK-2",
        minutes_left: 16,
        window: 30,
        fits_role: false,
        poster: "Reef Yard salvage desk",
        hand_in: "Dust Gate",
        task: "Three crates of parts sit in wreck WRK-2. Recover every crate and hand them in here.",
        have: 1,
        need: 3,
        done: false,
        hand_in_here: true,
    },
    JobCard {
        id: 10,
        kind: JobKind::Escort,
        title: "Escort hauler to Dust Gate",
        reward: 2_800,
        target: (1, 5),
        target_name: "Reef Yard",
        minutes_left: 5,
        window: 30,
        fits_role: true,
        poster: "Driftco freight",
        hand_in: "Reef Yard",
        task: "The hauler reached Dust Gate. Driftco pays at Reef Yard, 2 sectors west and 2 south.",
        have: 1,
        need: 1,
        done: true,
        hand_in_here: false,
    },
];

pub fn job(id: u32) -> &'static JobCard {
    JOBS.iter()
        .find(|j| j.id == id)
        .unwrap_or_else(|| panic!("no mock job {id}"))
}

/// A charted content: what a visited cell holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Content {
    Asteroids,
    Rocks,
    Planet,
    Derelicts,
}

impl Content {
    pub fn label(self) -> &'static str {
        match self {
            Content::Asteroids => "Asteroids",
            Content::Rocks => "Rocks",
            Content::Planet => "Planet",
            Content::Derelicts => "Derelicts",
        }
    }

    pub fn icon(self) -> IconType {
        match self {
            Content::Asteroids => IconType::Asteroids,
            Content::Rocks => IconType::Rocks,
            Content::Planet => IconType::Planet,
            Content::Derelicts => IconType::Derelicts,
        }
    }
}

/// A chart mark: what a known cell holds.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mark {
    Field(Content, u8),
    Station(StationKind, &'static str, &'static str),
    Wreck(&'static str),
    Job(u32),
}

/// The station mark variants the chart tells apart by shape, not colour alone.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StationKind {
    Living,
    Dead,
    Owned,
    Refusing,
    Posting(u8),
}

impl StationKind {
    pub fn state(self) -> String {
        match self {
            StationKind::Living => "living".into(),
            StationKind::Dead => "dead".into(),
            StationKind::Owned => "owned".into(),
            StationKind::Refusing => "refusing, 6 min".into(),
            StationKind::Posting(n) => format!("posting {n} jobs"),
        }
    }
}

/// A charted cell: a visited cell carries the play time it was entered and
/// its finds; a cell known only from a job mark carries neither.
pub struct ChartCell {
    pub at: (i32, i32),
    pub entered: Option<&'static str>,
    pub marks: &'static [Mark],
    pub finds: &'static [&'static str],
}

/// Cells the chart reaches from the player's cell on each axis: 7 by 7.
pub const CHART_REACH: i32 = 3;
pub const PLAYER_CELL: (i32, i32) = (3, 3);

/// Width of one sector, for the distance a job row shows.
pub const SECTOR_KM: f32 = 50.0;

pub const CHART: [ChartCell; 14] = [
    ChartCell {
        at: (3, 3),
        entered: Some("00:00"),
        marks: &[
            Mark::Station(StationKind::Posting(5), "STN-1", "Dust Gate"),
            Mark::Field(Content::Asteroids, 12),
            Mark::Job(1),
        ],
        finds: &["STN-1 Dust Gate"],
    },
    ChartCell {
        at: (4, 3),
        entered: Some("00:12"),
        marks: &[
            Mark::Field(Content::Asteroids, 30),
            Mark::Field(Content::Rocks, 4),
        ],
        finds: &[],
    },
    ChartCell {
        at: (2, 3),
        entered: Some("00:21"),
        marks: &[
            Mark::Field(Content::Rocks, 9),
            Mark::Field(Content::Derelicts, 1),
        ],
        finds: &["DRL-1"],
    },
    ChartCell {
        at: (3, 2),
        entered: Some("00:33"),
        marks: &[
            Mark::Station(StationKind::Owned, "STN-2", "Vane Dock"),
            Mark::Field(Content::Asteroids, 8),
        ],
        finds: &["STN-2 Vane Dock"],
    },
    ChartCell {
        at: (4, 2),
        entered: Some("00:40"),
        marks: &[
            Mark::Field(Content::Derelicts, 3),
            Mark::Field(Content::Rocks, 2),
        ],
        finds: &["DRL-3"],
    },
    ChartCell {
        at: (5, 2),
        entered: Some("00:52"),
        marks: &[
            Mark::Station(StationKind::Refusing, "STN-4", "Reef Yard"),
            Mark::Field(Content::Asteroids, 16),
            Mark::Job(2),
        ],
        finds: &["STN-4 Reef Yard"],
    },
    ChartCell {
        at: (2, 4),
        entered: Some("01:05"),
        marks: &[
            Mark::Wreck("WRK-2"),
            Mark::Field(Content::Rocks, 5),
            Mark::Job(9),
        ],
        finds: &["WRK-2"],
    },
    ChartCell {
        at: (1, 2),
        entered: Some("01:30"),
        marks: &[
            Mark::Field(Content::Planet, 1),
            Mark::Station(StationKind::Dead, "STN-3", "Ash Relay"),
        ],
        finds: &["PLN-1 Ash", "STN-3 Ash Relay"],
    },
    ChartCell {
        at: (3, 4),
        entered: Some("01:48"),
        marks: &[Mark::Field(Content::Asteroids, 22)],
        finds: &[],
    },
    ChartCell {
        at: (4, 4),
        entered: Some("01:58"),
        marks: &[],
        finds: &[],
    },
    ChartCell {
        at: (2, 2),
        entered: Some("02:05"),
        marks: &[
            Mark::Field(Content::Planet, 1),
            Mark::Field(Content::Rocks, 3),
        ],
        finds: &["PLN-2 Cinder"],
    },
    ChartCell {
        at: (1, 5),
        entered: None,
        marks: &[
            Mark::Station(StationKind::Living, "STN-5", "Reef Yard"),
            Mark::Job(3),
        ],
        finds: &[],
    },
    ChartCell {
        at: (2, 1),
        entered: None,
        marks: &[Mark::Job(4)],
        finds: &[],
    },
    ChartCell {
        at: (6, 6),
        entered: None,
        marks: &[Mark::Job(5)],
        finds: &[],
    },
];

pub fn chart_cell(at: (i32, i32)) -> Option<&'static ChartCell> {
    CHART.iter().find(|c| c.at == at)
}

/// The discovery log, newest first: icon, code, sector, relative play time,
/// and whether it was found this session.
pub const DISCOVERY_LOG: [(IconType, &str, (i32, i32), &str, bool); 7] = [
    (
        IconType::Station,
        "STN-4 Reef Yard",
        (5, 2),
        "4 min ago",
        true,
    ),
    (IconType::Wreck, "WRK-2", (2, 4), "11 min ago", true),
    (IconType::Planet, "PLN-2 Cinder", (2, 2), "19 min ago", true),
    (IconType::Derelicts, "DRL-3", (4, 2), "26 min ago", true),
    (IconType::Planet, "PLN-1 Ash", (1, 2), "1 h ago", false),
    (
        IconType::Station,
        "STN-2 Vane Dock",
        (3, 2),
        "1 h ago",
        false,
    ),
    (
        IconType::Station,
        "STN-1 Dust Gate",
        (3, 3),
        "2 h ago",
        false,
    ),
];

/// The last-seen prices and stock the chart keeps for a docked station.
pub const LAST_SEEN: [(&str, u32, u32); 3] = [
    ("Iron ore", 38, 14),
    ("Copper", 126, 2),
    ("Water ice", 21, 40),
];

/// The item categories the Inventory pane filters by, in its chip order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    Food,
    Ammo,
    Repair,
    Raw,
    Parts,
}

impl Category {
    pub const ALL: [Category; 5] = [
        Category::Food,
        Category::Ammo,
        Category::Repair,
        Category::Raw,
        Category::Parts,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::Food => "Food",
            Category::Ammo => "Ammo",
            Category::Repair => "Repair",
            Category::Raw => "Raw",
            Category::Parts => "Parts",
        }
    }

    pub fn icon(self) -> IconType {
        match self {
            Category::Food => IconType::Food,
            Category::Ammo => IconType::Ammo,
            Category::Repair => IconType::Repair,
            Category::Raw => IconType::Raw,
            Category::Parts => IconType::Parts,
        }
    }
}

/// One ware of the trade view: what the ship holds, what the station holds,
/// the reference price, what the station pays and asks, and the reason for
/// the gap.
pub struct Ware {
    pub name: &'static str,
    pub category: Category,
    pub kg: u32,
    pub ship_qty: u32,
    pub station_qty: u32,
    pub reference: u32,
    pub bid: u32,
    pub ask: u32,
    pub reason: &'static str,
}

pub const WARES: [Ware; 8] = [
    Ware {
        name: "Iron ore",
        category: Category::Raw,
        kg: 1_000,
        ship_qty: 12,
        station_qty: 60,
        reference: 40,
        bid: 33,
        ask: 38,
        reason: "Cluster type: belt station, ore is plentiful (60 t against a 40 t target)",
    },
    Ware {
        name: "Copper",
        category: Category::Raw,
        kg: 1_000,
        ship_qty: 8,
        station_qty: 2,
        reference: 90,
        bid: 126,
        ask: 140,
        reason: "Scarcity: 2 t in stock against a 20 t target",
    },
    Ware {
        name: "Water ice",
        category: Category::Food,
        kg: 1_000,
        ship_qty: 0,
        station_qty: 40,
        reference: 25,
        bid: 19,
        ask: 21,
        reason: "Demand: the galley is full, water sells under reference",
    },
    Ware {
        name: "Rations",
        category: Category::Food,
        kg: 20,
        ship_qty: 6,
        station_qty: 9,
        reference: 12,
        bid: 11,
        ask: 13,
        reason: "At reference: stock is on target",
    },
    Ware {
        name: "Drive parts",
        category: Category::Parts,
        kg: 400,
        ship_qty: 3,
        station_qty: 1,
        reference: 400,
        bid: 520,
        ask: 560,
        reason: "Civilization: Driftco yards pay over reference for drive parts",
    },
    Ware {
        name: "Hull plate",
        category: Category::Repair,
        kg: 250,
        ship_qty: 0,
        station_qty: 25,
        reference: 60,
        bid: 58,
        ask: 61,
        reason: "At reference: stock is on target",
    },
    Ware {
        name: "Fuel cells",
        category: Category::Parts,
        kg: 120,
        ship_qty: 4,
        station_qty: 6,
        reference: 120,
        bid: 108,
        ask: 150,
        reason: "Scarcity: 6 against a 10 target; demand: the watch buys fuel",
    },
    Ware {
        name: "PDC rounds",
        category: Category::Ammo,
        kg: 40,
        ship_qty: 2,
        station_qty: 30,
        reference: 30,
        bid: 24,
        ask: 27,
        reason: "Cluster type: a watch station keeps rounds under reference",
    },
];

/// Signed percent of `price` against `reference`.
pub fn delta_percent(price: u32, reference: u32) -> i32 {
    let delta = price as i64 - reference as i64;
    (delta * 100 / reference as i64) as i32
}

/// A start hull the role cards preview.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hull {
    Cargoa,
    Racer,
}

impl Hull {
    pub fn name(self) -> &'static str {
        match self {
            Hull::Cargoa => "cargoa",
            Hull::Racer => "racer",
        }
    }
}

/// A start role of the mock New Game: the shared first milestone, then its
/// own line, credits and kit.
pub struct RoleCard {
    pub id: &'static str,
    pub name: &'static str,
    pub hull: Hull,
    pub then: &'static str,
    pub credits: u32,
    pub kit: &'static [&'static str],
    pub line: &'static str,
    pub icon: IconType,
}

/// The first milestone every role shares.
pub const FIRST_MILESTONE: &str = "Own a station";

pub const ROLES: [RoleCard; 6] = [
    RoleCard {
        id: "industrial",
        name: "Industrial",
        hull: Hull::Cargoa,
        then: "then a mining fleet",
        credits: 6_000,
        kit: &["mining laser", "20 t hold"],
        line: "Starts with a mining laser and a hold.",
        icon: IconType::Raw,
    },
    RoleCard {
        id: "scavenger",
        name: "Scavenger",
        hull: Hull::Cargoa,
        then: "then a stocked yard",
        credits: 3_500,
        kit: &["cutter", "salvage drone"],
        line: "Starts with a cutter and a salvage drone.",
        icon: IconType::Recover,
    },
    RoleCard {
        id: "transport",
        name: "Transport",
        hull: Hull::Cargoa,
        then: "then a trade route",
        credits: 5_000,
        kit: &["40 t hold"],
        line: "Starts with the largest hold.",
        icon: IconType::Bring,
    },
    RoleCard {
        id: "bounty_hunter",
        name: "Bounty hunter",
        hull: Hull::Cargoa,
        then: "then a kill list",
        credits: 4_000,
        kit: &["PDC x2", "railgun"],
        line: "Starts with two PDCs and a railgun.",
        icon: IconType::Neutralize,
    },
    RoleCard {
        id: "military",
        name: "Military",
        hull: Hull::Cargoa,
        then: "then clear a ring of raiders",
        credits: 8_000,
        kit: &["PDC x2", "torpedoes"],
        line: "Starts with two PDCs and torpedoes.",
        icon: IconType::Escort,
    },
    RoleCard {
        id: "pirate",
        name: "Pirate",
        hull: Hull::Racer,
        then: "then outlive a civilization",
        credits: 2_500,
        kit: &["PDC", "boarding kit"],
        line: "Starts fast, armed and unwelcome.",
        icon: IconType::Pirate,
    },
];

/// One objective of a saved world, as its Load row and details list it.
pub struct SaveObjective {
    pub icon: IconType,
    pub title: &'static str,
    pub have: u32,
    pub need: u32,
}

/// One saved world on the mock Load page.
pub struct SaveCard {
    pub world: &'static str,
    pub role: usize,
    pub objectives: &'static [SaveObjective],
    pub jobs_held: u32,
    pub play_time: &'static str,
    pub saved_rel: &'static str,
    pub saved_at: &'static str,
    pub seed: u64,
    pub sector: &'static str,
    pub credits: u32,
    pub version: &'static str,
    /// How many cells the save has visited around its start, for the preview.
    pub explored: u32,
}

pub const RUNNING_VERSION: &str = "v0.14.0";

pub const SAVES: [SaveCard; 4] = [
    SaveCard {
        world: "Dust Gate run",
        role: 2,
        objectives: &[
            SaveObjective {
                icon: IconType::Milestone,
                title: "Own a station",
                have: 1,
                need: 1,
            },
            SaveObjective {
                icon: IconType::Recover,
                title: "Recover 3 crates of parts",
                have: 1,
                need: 3,
            },
        ],
        jobs_held: 1,
        play_time: "02:14:07",
        saved_rel: "2 h ago",
        saved_at: "2026-10-17 09:41",
        seed: 8_841_201_337,
        sector: "(3,3) Dust Gate",
        credits: 12_450,
        version: "v0.14.0",
        explored: 11,
    },
    SaveCard {
        world: "Second yard",
        role: 1,
        objectives: &[
            SaveObjective {
                icon: IconType::Milestone,
                title: "Own a station",
                have: 0,
                need: 1,
            },
            SaveObjective {
                icon: IconType::Bring,
                title: "Bring 6 t hull plate",
                have: 2,
                need: 6,
            },
            SaveObjective {
                icon: IconType::Escort,
                title: "Escort hauler to Cinder",
                have: 0,
                need: 1,
            },
        ],
        jobs_held: 2,
        play_time: "00:48:22",
        saved_rel: "yesterday",
        saved_at: "2026-10-16 22:05",
        seed: 1_923_775,
        sector: "(2,4) Wreck field",
        credits: 2_980,
        version: "v0.14.0",
        explored: 5,
    },
    SaveCard {
        world: "Line",
        role: 4,
        objectives: &[SaveObjective {
            icon: IconType::Milestone,
            title: "Own a station",
            have: 1,
            need: 1,
        }],
        jobs_held: 0,
        play_time: "05:30:51",
        saved_rel: "3 d ago",
        saved_at: "2026-10-14 18:12",
        seed: 55_017,
        sector: "(1,2) Ash orbit",
        credits: 71_020,
        version: "v0.13.2",
        explored: 19,
    },
    SaveCard {
        world: "Empire",
        role: 0,
        objectives: &[
            SaveObjective {
                icon: IconType::Milestone,
                title: "Own a station",
                have: 1,
                need: 1,
            },
            SaveObjective {
                icon: IconType::Neutralize,
                title: "Neutralize a raider hull",
                have: 0,
                need: 1,
            },
        ],
        jobs_held: 1,
        play_time: "01:05:00",
        saved_rel: "1 w ago",
        saved_at: "2026-10-10 11:00",
        seed: 7_777,
        sector: "(3,2) Vane Dock",
        credits: 23_400,
        version: "v0.14.0",
        explored: 8,
    },
];

pub const INITIAL_SEED: u64 = 8_841_201_337;

/// A small LCG step, so Randomize always moves the seed.
pub fn next_seed(seed: u64) -> u64 {
    seed.wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
        % 10_000_000_000
}

/// A stateless hash, for the seeded previews.
pub fn hash(seed: u64, salt: u64) -> u64 {
    let mut x = seed ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

pub fn thousands(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// The sector offset from the player as words.
pub fn offset_hint(target: (i32, i32)) -> String {
    let dx = target.0 - PLAYER_CELL.0;
    let dy = target.1 - PLAYER_CELL.1;
    if dx == 0 && dy == 0 {
        return "this sector".into();
    }
    let mut parts = Vec::new();
    if dx != 0 {
        let s = if dx.abs() == 1 { "sector" } else { "sectors" };
        parts.push(format!(
            "{} {s} {}",
            dx.abs(),
            if dx > 0 { "east" } else { "west" }
        ));
    }
    if dy != 0 {
        let word = if dy > 0 { "south" } else { "north" };
        if dx == 0 {
            let s = if dy.abs() == 1 { "sector" } else { "sectors" };
            parts.push(format!("{} {s} {word}", dy.abs()));
        } else {
            parts.push(format!("{} {word}", dy.abs()));
        }
    }
    parts.join(", ")
}

/// The distance and compass direction from the player's cell to `target`,
/// as a held row shows it while undocked.
pub fn distance_hint(target: (i32, i32)) -> String {
    let dx = (target.0 - PLAYER_CELL.0) as f32;
    let dy = (target.1 - PLAYER_CELL.1) as f32;
    if dx == 0.0 && dy == 0.0 {
        return "here".into();
    }
    let km = (dx * dx + dy * dy).sqrt() * SECTOR_KM;
    let ns = if dy < 0.0 {
        "N"
    } else if dy > 0.0 {
        "S"
    } else {
        ""
    };
    let ew = if dx > 0.0 {
        "E"
    } else if dx < 0.0 {
        "W"
    } else {
        ""
    };
    format!("{km:.0} km {ns}{ew}")
}
