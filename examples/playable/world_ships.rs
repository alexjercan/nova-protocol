//! world_ships: the open world's generated ships, laid out by the same
//! `generate_ship` the world will spawn them through, posed as a fixed matrix
//! beside the current WFC hulls and flown one at a time.
//!
//! The matrix is every role at its lowest and its highest eligible
//! advancement, each with a centred single drive and with a mirrored pair: for
//! each cell, the first hull seed in a pinned range whose layout carries that
//! drive. The snapshot is the content lint's own: the repo bundles `--mods`
//! names, base and every dependency included, so the content hash in the report
//! is the one the lint checked. Nothing here spawns an open-world ship or
//! draws a civilization; the requests carry one pinned civilization id, which
//! only a failure line reads.
//!
//! Each role also has a WRECK: `generate_wreck` of the request its
//! `<role>-high-single` ship was drawn from, so a wreck and its intact twin
//! share seed, role, source and advancement. A wreck that cannot be generated
//! fails the run with its seed, role and constraint. Wrecks stand bare like the
//! matrix; no weathered or damaged section art exists in the snapshot, and the
//! report and readout say so rather than count the exposed cube lattice as
//! that art.
//!
//! # Views
//!
//! - Low and high advancement: one column per role (civilian, industrial,
//!   scavenger, armored, left to right), the centred-single drive in the front
//!   row and the mirrored pair behind it, bare as the generator emits them.
//! - WFC: four hulls of the current `wfc_ships` generator on the standard
//!   plan, also bare, at the same spacing rule.
//! - Wrecks: one DERELICT per role, same column order, bare. Every active
//!   section (drive, flight computer, weapons, intake) is disabled in place as
//!   it spawns; the ships have no controller, Neutral allegiance and every
//!   ship capability off, and keep their physics.
//! - Pilot: one matrix ship under the player's own controller. An industrial
//!   ship is offered a canister that drifts into its intake; a fighter's
//!   turrets fire on the left mouse button, its bays on `F` and its rails on
//!   `R` once weapons are raised.
//!
//! # Hand-run
//!
//! ```text
//! cargo run --example world_ships --features debug
//! cargo run --example world_ships --features debug -- --mods example
//! cargo run --example world_ships --features debug -- --pilot armored-high-paired
//! ```
//!
//! | key | what it does |
//! | - | - |
//! | 1 / 2 / 3 / 4 | low advancement / high advancement / WFC / wrecks view |
//! | P | pilot the first matrix ship |
//! | N | pilot the next matrix ship |
//! | B | back to the matrix views |
//!
//! `--mods` is written to the enabled-mods set, as a Mods screen toggle is, so
//! the runtime catalog holds exactly the snapshot's sections. The game saves
//! that set to the user config dir like any toggle, so a run changes the mods
//! the next game starts with; point `XDG_CONFIG_HOME` at a scratch dir to keep
//! yours.
//!
//! Harnessed mode:
//! - `NOVA_AUTOPILOT=1`: shoot the four views; command every wreck to burn,
//!   turn, fire and take a canister, and fail the run when one answers, when
//!   an active section of one is live, or when a hit does not damage it; then
//!   fly every matrix ship: take a canister through each industrial intake,
//!   fire each fighter, burn the main drive and turn, and fail the run when a
//!   ship does not answer.
//! - `NOVA_CAPTURE=1`: also writes `world-ships-<view>.png`,
//!   `world-ships-wrecks-commanded.png` with every wreck drive at full input, one
//!   `world-ships-pilot-<ship>.png` per matrix ship and
//!   `world-ships-report.md` under `NOVA_CAPTURE_DIR`.
//!
//! Every run logs the same report at boot: snapshot sources, content hash,
//! part advancement, exclusions, the matrix, the wrecks, the WFC reference and
//! the matched generation cost of both generators.

use std::{fmt::Write as _, time::Instant};

#[cfg(feature = "debug")]
use avian3d::prelude::{Collider, LinearVelocity, Sleeping};
use bevy::{input::mouse::MouseMotion, prelude::*};
use clap::Parser;
#[cfg(feature = "debug")]
use nova_debug::prelude::capturing;
use nova_input::prelude::InputSource;
use nova_protocol::prelude::*;
use nova_wfc::prelude::*;

#[derive(Parser)]
#[command(name = "world_ships")]
#[command(version = "1.0.0")]
#[command(
    about = "Pose and fly the open world's generated ships beside the current WFC hulls",
    long_about = None
)]
struct Cli {
    /// Repo bundles whose effective catalog the snapshot reads, comma
    /// separated. Base and every dependency join on their own.
    #[arg(long, default_value = "base", value_delimiter = ',')]
    mods: Vec<String>,
    /// Start by piloting one matrix ship, named `<role>-<low|high>-<single|paired>`.
    #[arg(long)]
    pilot: Option<String>,
}

/// The world seed of the pinned civilization every request names.
const WORLD_SEED: u32 = 20_260_922;

/// The first hull seed a matrix cell tries.
const FIRST_HULL_SEED: u32 = 1;

/// How many hull seeds a matrix cell tries for its drive layout before it is
/// reported as a gap.
const HULL_SEED_SEARCH: u32 = 32;

/// The advancement steps the lowest eligible advancement of a role is searched
/// on.
const ADVANCEMENT_STEPS: u32 = 20;

/// The standard-plan WFC seeds the reference view and the cost sets use: the
/// `wfc_ships` row's default first seed and the three after it.
const WFC_SEEDS: [u64; 4] = [20_260_815, 20_260_816, 20_260_817, 20_260_818];

/// Matched cost sets: each set lays out every matrix request and every WFC
/// seed once.
const COST_SETS: usize = 5;

/// Clear space between the clearance spheres of two neighbouring ships, in
/// engine units.
const STAND_GAP: f32 = 2.0;

/// Every posed ship stands at the same quarter yaw, as the `wfc_ships` row
/// does.
const SHIP_YAW: f32 = -0.5;

/// Mouse motion a turn beat writes per frame, in pixels.
const TURN_PIXELS: Vec2 = Vec2::new(12.0, 0.0);

/// Frames a turn beat writes mouse motion for.
#[cfg(feature = "debug")]
const TURN_FRAMES: u32 = 30;

/// Real seconds a burn, turn or intake beat may take before the run fails:
/// heavy high-advancement hulls answer slowly, and a software-rendered frame of
/// one costs seconds while the app clock advances at most `max_delta`.
#[cfg(feature = "debug")]
const PILOT_DEADLINE_SECS: f32 = 180.0;

/// Seconds a pilot beat holds the fire keys.
#[cfg(feature = "debug")]
const FIRE_SECS: f32 = 2.0;

/// Seconds a wreck's intake is offered a canister: several times what a live
/// intake takes to draw one in at the offered closing speed.
#[cfg(feature = "debug")]
const WRECK_INTAKE_SECS: f32 = 6.0;

/// Seconds weapons get to deploy after the stance is raised.
#[cfg(feature = "debug")]
const DEPLOY_SECS: f32 = 2.0;

/// The least speed a burn must reach along the bow, in meters per second.
#[cfg(feature = "debug")]
const MIN_BURN_SPEED: MetersPerSecond = MetersPerSecond(1.0);

/// The least heading change a turn must reach, in degrees.
#[cfg(feature = "debug")]
const MIN_TURN_DEGREES: f32 = 5.0;

/// How far past the capture gap an offered canister's near side starts, in
/// engine units.
#[cfg(feature = "debug")]
const CANISTER_STANDOFF: f32 = 0.3;

/// The offered canister's closing speed as a share of the intake's capture
/// speed limit. Base content's limit is 5 m/s, so 0.6 closes at 3 m/s: a
/// canister at avian's default linear sleep threshold (1.5 m/s) falls asleep
/// and never reaches the door.
#[cfg(feature = "debug")]
const CANISTER_CLOSING_SHARE: f32 = 0.6;

/// The name `nova_core` gives the scenario loading panel. On a software
/// renderer the panel holds to its hard cap after the scene is up, so a shot
/// waits for it to come down.
#[cfg(feature = "debug")]
const LOAD_PANEL: &str = "Scenario Loading Screen";

/// The report file, under the capture dir.
#[cfg(feature = "debug")]
const REPORT_FILE: &str = "world-ships-report.md";

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    let ids: Vec<&str> = cli.mods.iter().map(String::as_str).collect();
    let packs = nova_authoring::lint_walk::repo_ship_part_packs(&ids);
    let snapshot = ShipPartSnapshot::build(&packs).unwrap_or_else(|faults| {
        panic!(
            "world_ships: the snapshot of {ids:?} is refused:\n  {}",
            faults
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n  ")
        )
    });
    let wfc_sections = GameSections(nova_authoring::generation::build_section_catalog());
    let wfc = TileSet::build(&wfc_sections, &WfcPlan::standard_hull())
        .unwrap_or_else(|error| panic!("world_ships: {error}"));

    let matrix = Matrix::generate(&snapshot);
    let wfc_hulls: Vec<ShipDesign> = WFC_SEEDS
        .iter()
        .map(|seed| {
            wfc.hull(*seed, false, None)
                .unwrap_or_else(|error| panic!("world_ships: WFC seed {seed}: {error}"))
        })
        .collect();
    let cost = GenerationCost::measure(&snapshot, &matrix, &wfc);
    let report = Report {
        mods: cli.mods.clone(),
        snapshot: &snapshot,
        matrix: &matrix,
        wfc_hulls: &wfc_hulls,
        cost: &cost,
    }
    .to_markdown();
    info!("world_ships report:\n{report}");

    let pilot = cli.pilot.as_deref().map(|name| {
        matrix
            .flown()
            .position(|ship| ship.name() == name)
            .unwrap_or_else(|| {
                panic!(
                    "world_ships: --pilot '{name}' is not a matrix ship; one of: {}",
                    matrix
                        .flown()
                        .map(MatrixShip::name)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
    });
    #[cfg(feature = "debug")]
    let flown: Vec<(String, ShipRoleType)> = matrix
        .flown()
        .map(|ship| (ship.name(), ship.role))
        .collect();
    let stage = Stage {
        mods: snapshot.sources().to_vec(),
        prototypes: snapshot
            .parts()
            .iter()
            .map(|part| part.id().to_string())
            .collect(),
        matrix,
        wfc_hulls,
        #[cfg(feature = "debug")]
        report,
        #[cfg(feature = "debug")]
        pilot_log: Vec::new(),
    };
    let scene = match pilot {
        Some(index) => SceneType::Pilot(index),
        None => SceneType::View(ViewType::Low),
    };

    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| stage_plugin(app, stage.clone(), scene))
        .build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        if capturing() {
            app.add_systems(Startup, hide_hud);
        }
        // Posed ships hold still, so a spawn impulse cannot turn the stand
        // between two runs' shots. A piloted ship keeps its physics, and so do
        // the wrecks, whose beats show that nothing on board moves them.
        app.add_systems(
            Update,
            freeze_bodies.run_if(|scene: Res<SceneType>| {
                matches!(*scene, SceneType::View(view) if view != ViewType::Wrecks)
            }),
        );
        app.add_systems(
            Update,
            hold_wreck_commands.run_if(resource_exists::<WreckCommand>),
        );
        app.add_plugins(world_ships_script(flown));
    }

    app.run()
}

/// Which band of a role's eligible advancement a matrix ship is drawn at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TierType {
    /// The lowest advancement at which the snapshot serves the role.
    Low,
    /// Full advancement.
    High,
}

impl TierType {
    const ALL: [Self; 2] = [Self::Low, Self::High];

    fn label(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::High => "high",
        }
    }
}

fn drive_label(drive: ShipDriveLayoutType) -> &'static str {
    match drive {
        ShipDriveLayoutType::CenteredSingle => "single",
        ShipDriveLayoutType::MirroredPair => "paired",
    }
}

const DRIVES: [ShipDriveLayoutType; 2] = [
    ShipDriveLayoutType::CenteredSingle,
    ShipDriveLayoutType::MirroredPair,
];

/// One generated matrix ship.
#[derive(Clone)]
struct MatrixShip {
    role: ShipRoleType,
    tier: TierType,
    advancement: f32,
    layout: ShipLayout,
    /// Section count per part family, in `ShipPartFamilyType::ALL` order.
    families: Vec<(ShipPartFamilyType, usize)>,
    /// Sections whose part a mod pack authored.
    mod_sections: usize,
    seed: u32,
}

impl MatrixShip {
    fn name(&self) -> String {
        format!(
            "{}-{}-{}",
            self.role.label(),
            self.tier.label(),
            drive_label(self.layout.drive)
        )
    }
}

/// One matrix cell: a ship, or why the pinned seed range holds none.
#[derive(Clone)]
struct MatrixCell {
    role: ShipRoleType,
    tier: TierType,
    drive: ShipDriveLayoutType,
    ship: Result<MatrixShip, String>,
}

/// One role's wreck: the ruin of its `<role>-high-single` matrix ship.
#[derive(Clone)]
struct WreckShip {
    role: ShipRoleType,
    seed: u32,
    advancement: f32,
    /// Sections of the intact twin.
    intact_sections: usize,
    layout: ShipLayout,
}

impl WreckShip {
    fn name(&self) -> String {
        format!("wreck-{}", self.role.label())
    }
}

/// Every role at both tiers with both drives, and one wreck per role.
#[derive(Clone)]
struct Matrix {
    civilization: CivilizationId,
    cells: Vec<MatrixCell>,
    wrecks: Vec<WreckShip>,
}

impl Matrix {
    /// Lay out every cell.
    ///
    /// # Panics
    ///
    /// When `generate_ship` fails a request: a valid snapshot that cannot build
    /// a role it serves is the generator's failure, not a gap. When a role has
    /// no `<role>-high-single` ship or `generate_wreck` fails its request: a
    /// selected wreck that cannot be built is a failure too.
    fn generate(snapshot: &ShipPartSnapshot) -> Self {
        let civilization = CivilizationId {
            world_seed: WORLD_SEED,
            node: [0, 0, 0],
        };
        let mut cells = Vec::new();
        for role in ShipRoleType::ALL {
            let lowest = (0..=ADVANCEMENT_STEPS)
                .map(|step| step as f32 / ADVANCEMENT_STEPS as f32)
                .find(|advancement| snapshot.eligible_roles(*advancement).contains(&role));
            for tier in TierType::ALL {
                let advancement = match tier {
                    TierType::Low => lowest,
                    TierType::High => lowest.map(|_| 1.0),
                };
                for drive in DRIVES {
                    let ship = match advancement {
                        None => Err(format!(
                            "the snapshot serves no {} at any advancement",
                            role.label()
                        )),
                        Some(advancement) => {
                            first_seed_with(snapshot, civilization, role, tier, advancement, drive)
                        }
                    };
                    cells.push(MatrixCell {
                        role,
                        tier,
                        drive,
                        ship,
                    });
                }
            }
        }
        let wrecks = ShipRoleType::ALL
            .into_iter()
            .map(|role| {
                let twin = cells
                    .iter()
                    .find(|cell| {
                        cell.role == role
                            && cell.tier == TierType::High
                            && cell.drive == ShipDriveLayoutType::CenteredSingle
                    })
                    .and_then(|cell| cell.ship.as_ref().ok())
                    .unwrap_or_else(|| {
                        panic!("world_ships: no {}-high-single ship to ruin", role.label())
                    });
                let request = ShipLayoutRequest {
                    seed: twin.seed,
                    civilization,
                    role,
                    advancement: twin.advancement,
                };
                let layout = generate_wreck(snapshot, request)
                    .unwrap_or_else(|failure| panic!("world_ships: wreck: {failure}"));
                WreckShip {
                    role,
                    seed: twin.seed,
                    advancement: twin.advancement,
                    intact_sections: twin.layout.design.sections.len(),
                    layout,
                }
            })
            .collect();
        Self {
            civilization,
            cells,
            wrecks,
        }
    }

    fn requests(&self) -> impl Iterator<Item = ShipLayoutRequest> + '_ {
        self.flown().map(|ship| ShipLayoutRequest {
            seed: ship.seed,
            civilization: self.civilization,
            role: ship.role,
            advancement: ship.advancement,
        })
    }

    /// Every matrix ship, in cell order.
    fn flown(&self) -> impl Iterator<Item = &MatrixShip> {
        self.cells.iter().filter_map(|cell| cell.ship.as_ref().ok())
    }
}

/// The first hull seed in the pinned range whose layout carries `drive`.
fn first_seed_with(
    snapshot: &ShipPartSnapshot,
    civilization: CivilizationId,
    role: ShipRoleType,
    tier: TierType,
    advancement: f32,
    drive: ShipDriveLayoutType,
) -> Result<MatrixShip, String> {
    for seed in FIRST_HULL_SEED..FIRST_HULL_SEED + HULL_SEED_SEARCH {
        let request = ShipLayoutRequest {
            seed,
            civilization,
            role,
            advancement,
        };
        let layout = generate_ship(snapshot, request)
            .unwrap_or_else(|failure| panic!("world_ships: {failure}"));
        if layout.drive != drive {
            continue;
        }
        let part = |section: &SpaceshipSectionConfig| -> &ShipPart {
            let SectionSource::Prototype { id, .. } = &section.source else {
                panic!("world_ships: a generated section is not a prototype");
            };
            snapshot
                .parts()
                .iter()
                .find(|part| part.id() == id)
                .unwrap_or_else(|| panic!("world_ships: '{id}' is not a snapshot part"))
        };
        let families = ShipPartFamilyType::ALL
            .iter()
            .map(|family| {
                let count = layout
                    .design
                    .sections
                    .iter()
                    .filter(|section| part(section).family == *family)
                    .count();
                (*family, count)
            })
            .collect();
        let mod_sections = layout
            .design
            .sections
            .iter()
            .filter(|section| part(section).source != "base")
            .count();
        return Ok(MatrixShip {
            role,
            tier,
            advancement,
            layout,
            families,
            mod_sections,
            seed,
        });
    }
    Err(format!(
        "no hull seed in {FIRST_HULL_SEED}..{} lays out a {} drive",
        FIRST_HULL_SEED + HULL_SEED_SEARCH,
        drive_label(drive)
    ))
}

/// Wall-clock cost of both generators over matched sets: every set lays out
/// the matrix requests and the WFC seeds once each, after the snapshot and the
/// tile set are built.
struct GenerationCost {
    generated_ships: usize,
    wfc_ships: usize,
    /// Per set, milliseconds for all generated ships and all WFC hulls.
    sets: Vec<(f64, f64)>,
}

impl GenerationCost {
    fn measure(snapshot: &ShipPartSnapshot, matrix: &Matrix, wfc: &TileSet) -> Self {
        let requests: Vec<ShipLayoutRequest> = matrix.requests().collect();
        let sets = (0..COST_SETS)
            .map(|_| {
                let start = Instant::now();
                for request in &requests {
                    let layout = generate_ship(snapshot, *request)
                        .unwrap_or_else(|failure| panic!("world_ships: {failure}"));
                    std::hint::black_box(layout);
                }
                let generated = start.elapsed().as_secs_f64() * 1_000.0;
                let start = Instant::now();
                for seed in WFC_SEEDS {
                    let hull = wfc
                        .hull(seed, false, None)
                        .unwrap_or_else(|error| panic!("world_ships: {error}"));
                    std::hint::black_box(hull);
                }
                let wfc = start.elapsed().as_secs_f64() * 1_000.0;
                (generated, wfc)
            })
            .collect();
        Self {
            generated_ships: requests.len(),
            wfc_ships: WFC_SEEDS.len(),
            sets,
        }
    }

    fn median_per_ship(&self) -> (f64, f64) {
        let median = |mut values: Vec<f64>| {
            values.sort_by(f64::total_cmp);
            values[values.len() / 2]
        };
        (
            median(self.sets.iter().map(|set| set.0).collect()) / self.generated_ships as f64,
            median(self.sets.iter().map(|set| set.1).collect()) / self.wfc_ships as f64,
        )
    }
}

/// The boot report's inputs.
struct Report<'a> {
    mods: Vec<String>,
    snapshot: &'a ShipPartSnapshot,
    matrix: &'a Matrix,
    wfc_hulls: &'a [ShipDesign],
    cost: &'a GenerationCost,
}

impl Report<'_> {
    fn to_markdown(&self) -> String {
        let mut out = String::new();
        let snapshot = self.snapshot;
        let _ = writeln!(out, "# World ships\n");
        let _ = writeln!(
            out,
            "- requested bundles: {}\n- snapshot sources: {}\n- content hash: `{:016x}`\n- \
             civilization: `{}` (pinned; only a failure reads it)\n- hull seeds: {}..{}\n- \
             weathered or damaged section art: none in the snapshot (missing); a wreck's \
             only ruin cue is its holes\n",
            self.mods.join(", "),
            snapshot.sources().join(", "),
            snapshot.content_hash(),
            self.matrix.civilization,
            FIRST_HULL_SEED,
            FIRST_HULL_SEED + HULL_SEED_SEARCH,
        );

        let _ = writeln!(out, "## Parts\n");
        let _ = writeln!(out, "| part | source | family | advancement |");
        let _ = writeln!(out, "| - | - | - | - |");
        for part in snapshot.parts() {
            let _ = writeln!(
                out,
                "| `{}` | {} | {} | {:.3} |",
                part.id(),
                part.source,
                part.family,
                part.advancement
            );
        }
        for (id, why) in snapshot.excluded() {
            let _ = writeln!(out, "| `{id}` | excluded | {why:?} | - |");
        }

        let _ = writeln!(out, "\n## Matrix\n");
        let _ = writeln!(
            out,
            "| ship | advancement | seed | source | attempt | sections | clearance | \
             families | mod sections |"
        );
        let _ = writeln!(out, "| - | - | - | - | - | - | - | - | - |");
        for cell in &self.matrix.cells {
            match &cell.ship {
                Ok(ship) => {
                    let families = ship
                        .families
                        .iter()
                        .filter(|(_, count)| *count > 0)
                        .map(|(family, count)| format!("{family} {count}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let _ = writeln!(
                        out,
                        "| {} | {:.2} | {} | {} | {} | {} | {:.0} m | {families} | {} |",
                        ship.name(),
                        ship.advancement,
                        ship.seed,
                        ship.layout.source,
                        ship.layout.attempt,
                        ship.layout.design.sections.len(),
                        ship.layout.clearance.0,
                        ship.mod_sections,
                    );
                }
                Err(gap) => {
                    let _ = writeln!(
                        out,
                        "| {}-{}-{} | - | - | - | - | - | - | gap: {gap} | - |",
                        cell.role.label(),
                        cell.tier.label(),
                        drive_label(cell.drive),
                    );
                }
            }
        }

        let _ = writeln!(out, "\n## Wrecks\n");
        let _ = writeln!(
            out,
            "Each the ruin of its role's high single-drive ship, bare, every active section \
             disabled, no controller, Neutral.\n"
        );
        let _ = writeln!(
            out,
            "| wreck | former role | advancement | seed | intact sections | sections | \
             omitted | clearance |"
        );
        let _ = writeln!(out, "| - | - | - | - | - | - | - | - |");
        for wreck in &self.matrix.wrecks {
            let sections = wreck.layout.design.sections.len();
            let _ = writeln!(
                out,
                "| {} | {} | {:.2} | {} | {} | {sections} | {} | {:.0} m |",
                wreck.name(),
                wreck.role.label(),
                wreck.advancement,
                wreck.seed,
                wreck.intact_sections,
                wreck.intact_sections - sections,
                wreck.layout.clearance.0,
            );
        }

        let _ = writeln!(out, "\n## WFC reference\n");
        let _ = writeln!(
            out,
            "Standard plan on the base builders' catalog, bare. Sections per hull: {}.",
            WFC_SEEDS
                .iter()
                .zip(self.wfc_hulls)
                .map(|(seed, hull)| format!("{seed}: {}", hull.sections.len()))
                .collect::<Vec<_>>()
                .join(", ")
        );

        let cost = self.cost;
        let (generated, wfc) = cost.median_per_ship();
        let _ = writeln!(out, "\n## Generation cost\n");
        let _ = writeln!(
            out,
            "{} matched sets, each {} generated ships and {} WFC hulls, after the snapshot \
             and the tile set are built. Wall clock in this build's profile; not a budget.\n",
            cost.sets.len(),
            cost.generated_ships,
            cost.wfc_ships
        );
        let _ = writeln!(out, "| set | generated ms | WFC ms |");
        let _ = writeln!(out, "| - | - | - |");
        for (index, (generated, wfc)) in cost.sets.iter().enumerate() {
            let _ = writeln!(out, "| {index} | {generated:.1} | {wfc:.1} |");
        }
        let _ = writeln!(
            out,
            "\nMedian per ship: generated {generated:.2} ms, WFC {wfc:.2} ms."
        );
        out
    }
}

/// Which posed view is on the stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewType {
    Low,
    High,
    Wfc,
    Wrecks,
}

impl ViewType {
    #[cfg(feature = "debug")]
    const ALL: [Self; 4] = [Self::Low, Self::High, Self::Wfc, Self::Wrecks];

    fn label(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::High => "high",
            Self::Wfc => "wfc",
            Self::Wrecks => "wrecks",
        }
    }
}

/// What the stage shows.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
enum SceneType {
    View(ViewType),
    /// A matrix ship, by its index in `Matrix::flown`.
    Pilot(usize),
}

/// Everything the app needs from the pure half, computed before it starts.
#[derive(Resource, Clone)]
struct Stage {
    /// The snapshot's sources: the bundles the runtime must enable.
    mods: Vec<String>,
    /// Every snapshot part id: the runtime catalog must hold each.
    prototypes: Vec<String>,
    matrix: Matrix,
    wfc_hulls: Vec<ShipDesign>,
    #[cfg(feature = "debug")]
    report: String,
    /// One line per pilot beat the harness checked.
    #[cfg(feature = "debug")]
    pilot_log: Vec<String>,
}

/// The scene asked for and not yet loaded.
#[derive(Resource)]
struct PendingScene(SceneType);

fn stage_plugin(app: &mut App, stage: Stage, scene: SceneType) {
    app.insert_resource(stage);
    app.insert_resource(scene);
    app.insert_resource(PendingScene(scene));
    app.add_systems(OnEnter(GameAssetsStates::Loaded), enable_snapshot_mods);
    app.add_systems(
        Update,
        (
            load_pending_scene.run_if(in_state(GameAssetsStates::Loaded)),
            switch_scene_on_key.run_if(in_state(GameStates::Playing)),
            frame_new_camera,
            update_readout,
            push_turn.run_if(resource_exists::<TurnPush>),
        ),
    );
    app.add_systems(Startup, spawn_readout);
    app.add_observer(disable_wreck_systems);
}

/// Enable exactly the snapshot's bundles, so the merged catalog is the one
/// the snapshot was drawn from.
fn enable_snapshot_mods(stage: Res<Stage>, mut enabled: ResMut<EnabledMods>) {
    let wanted = EnabledMods(stage.mods.iter().cloned().collect());
    if *enabled != wanted {
        *enabled = wanted;
    }
}

/// Load the pending scene once the merged catalog holds every snapshot part.
fn load_pending_scene(
    mut commands: Commands,
    pending: Option<Res<PendingScene>>,
    stage: Res<Stage>,
    enabled: Res<EnabledMods>,
    sections: Res<GameSections>,
    game_assets: Res<GameAssets>,
) {
    let Some(pending) = pending else {
        return;
    };
    let wanted: std::collections::HashSet<String> = stage.mods.iter().cloned().collect();
    if enabled.0 != wanted
        || !stage
            .prototypes
            .iter()
            .all(|id| sections.get_section(id).is_some())
    {
        return;
    }
    let scene = pending.0;
    commands.remove_resource::<PendingScene>();
    commands.insert_resource(scene);
    commands.trigger(LoadScenario(scenario(
        &game_assets,
        &sections,
        &stage,
        scene,
    )));
}

/// Switch views and pilots in a hand-run.
fn switch_scene_on_key(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    scene: Res<SceneType>,
    stage: Res<Stage>,
) {
    let flown = stage.matrix.flown().count();
    let next = if keyboard.just_pressed(KeyCode::Digit1) {
        SceneType::View(ViewType::Low)
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        SceneType::View(ViewType::High)
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        SceneType::View(ViewType::Wfc)
    } else if keyboard.just_pressed(KeyCode::Digit4) {
        SceneType::View(ViewType::Wrecks)
    } else if keyboard.just_pressed(KeyCode::KeyP) && flown > 0 {
        SceneType::Pilot(0)
    } else if keyboard.just_pressed(KeyCode::KeyN) && flown > 0 {
        match *scene {
            SceneType::Pilot(index) => SceneType::Pilot((index + 1) % flown),
            SceneType::View(_) => SceneType::Pilot(0),
        }
    } else if keyboard.just_pressed(KeyCode::KeyB) {
        SceneType::View(ViewType::Low)
    } else {
        return;
    };
    commands.insert_resource(PendingScene(next));
}

/// Where each ship of a posed view stands, in engine units: `columns` across,
/// rows behind, every cell as wide as the widest clearance sphere.
fn stand_positions(clearances: &[f32], columns: usize) -> Vec<Vec3> {
    let pitch = clearances.iter().copied().fold(0.0, f32::max) * 2.0 + STAND_GAP;
    let rows = clearances.len().div_ceil(columns);
    (0..clearances.len())
        .map(|index| {
            let column = index % columns;
            let row = index / columns;
            Vec3::new(
                (column as f32 - (columns as f32 - 1.0) * 0.5) * pitch,
                0.0,
                // Row 0 nearest the camera, which stands on +Z.
                ((rows as f32 - 1.0) * 0.5 - row as f32) * pitch,
            )
        })
        .collect()
}

/// The ships a posed view stands, with their names.
fn view_ships(stage: &Stage, view: ViewType) -> Vec<(String, ShipDesign, f32)> {
    match view {
        ViewType::Low | ViewType::High => {
            let tier = if view == ViewType::Low {
                TierType::Low
            } else {
                TierType::High
            };
            // Front row single drives, back row pairs, one column per role.
            DRIVES
                .iter()
                .flat_map(|drive| {
                    stage
                        .matrix
                        .cells
                        .iter()
                        .filter(move |cell| cell.tier == tier && cell.drive == *drive)
                })
                .filter_map(|cell| cell.ship.as_ref().ok())
                .map(|ship| {
                    (
                        ship.name(),
                        ship.layout.design.clone(),
                        ship.layout.clearance.to_engine(),
                    )
                })
                .collect()
        }
        ViewType::Wfc => WFC_SEEDS
            .iter()
            .zip(&stage.wfc_hulls)
            .map(|(seed, hull)| (format!("wfc-{seed}"), hull.clone(), design_clearance(hull)))
            .collect(),
        ViewType::Wrecks => stage
            .matrix
            .wrecks
            .iter()
            .map(|wreck| {
                (
                    wreck.name(),
                    wreck.layout.design.clone(),
                    wreck.layout.clearance.to_engine(),
                )
            })
            .collect(),
    }
}

/// Disable every active section of a wreck as it spawns. The section's kind
/// marker lands in the spawn's own command flush, after its `ChildOf` and the
/// root's `EntityId`, so no tick sees the section live.
fn disable_wreck_systems(
    add: On<
        Add,
        (
            ThrusterSectionMarker,
            ControllerSectionMarker,
            TurretSectionMarker,
            TorpedoSectionMarker,
            RailgunSectionMarker,
            CargoIntakeSectionMarker,
            DockingSectionMarker,
        ),
    >,
    mut commands: Commands,
    stage: Res<Stage>,
    q_section: Query<&ChildOf>,
    q_root: Query<&EntityId, With<SpaceshipRootMarker>>,
) {
    let Ok(ChildOf(root)) = q_section.get(add.entity) else {
        return;
    };
    let Ok(id) = q_root.get(*root) else {
        return;
    };
    if stage.matrix.wrecks.iter().any(|wreck| wreck.name() == id.0) {
        commands.entity(add.entity).insert(SectionInactiveMarker);
    }
}

/// Half the diagonal of a design's section positions, plus half a cell: the
/// generator's clearance rule, for a hull it did not lay out.
fn design_clearance(design: &ShipDesign) -> f32 {
    let (low, high) = design.sections.iter().fold(
        (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
        |(low, high), section| (low.min(section.position), high.max(section.position)),
    );
    (high - low + Vec3::ONE).length() * 0.5
}

/// The scenario a scene loads.
fn scenario(
    game_assets: &GameAssets,
    sections: &GameSections,
    stage: &Stage,
    scene: SceneType,
) -> ScenarioConfig {
    let (id, actions) = match scene {
        SceneType::View(view) => {
            let ships = view_ships(stage, view);
            let clearances: Vec<f32> = ships.iter().map(|ship| ship.2).collect();
            let positions = stand_positions(&clearances, ShipRoleType::ALL.len());
            let scale = clearances.iter().copied().fold(1.0, f32::max) * 0.5;
            let actions = ships
                .into_iter()
                .zip(positions)
                .map(|((name, design, _), position)| {
                    let ship = SpaceshipConfig {
                        controller: SpaceshipController::None,
                        design: ShipDesignSource::Inline(design),
                        ..default()
                    };
                    // A wreck states what it lacks rather than lean on the
                    // defaults: nobody drives it, it sides with nobody and
                    // it can do nothing.
                    let ship = match view {
                        ViewType::Wrecks => SpaceshipConfig {
                            allegiance: Some(Allegiance::Neutral),
                            capabilities: ShipCapabilities {
                                stop_enabled: false,
                                goto_enabled: false,
                                orbit_enabled: false,
                                lock_enabled: false,
                                rcs_enabled: false,
                                point_defense_enabled: false,
                                dock_enabled: false,
                            },
                            ..ship
                        },
                        ViewType::Low | ViewType::High | ViewType::Wfc => ship,
                    };
                    ship_action(&name, position, Quat::from_rotation_y(SHIP_YAW), ship)
                })
                .chain(ThreePointRig::around("stand", Meters3::ZERO, scale).actions())
                .collect();
            (format!("world_ships_{}", view.label()), actions)
        }
        SceneType::Pilot(index) => {
            let ship = stage
                .matrix
                .flown()
                .nth(index)
                .expect("a pilot scene names a matrix ship");
            let bindings = weapon_bindings(&ship.layout.design, sections);
            let scale = ship.layout.clearance.to_engine() * 0.5;
            let actions = std::iter::once(ship_action(
                &ship.name(),
                Vec3::ZERO,
                Quat::IDENTITY,
                SpaceshipConfig {
                    controller: SpaceshipController::Player(PlayerControllerConfig {
                        input_mapping: bindings,
                    }),
                    design: ShipDesignSource::Inline(ship.layout.design.clone()),
                    ..default()
                },
            ))
            .chain(ThreePointRig::around("pilot", Meters3::ZERO, scale).actions())
            .collect();
            ("world_ships_pilot".to_string(), actions)
        }
    };
    let scenario = ScenarioConfig {
        description: "Generated open-world ships beside the current WFC hulls".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions,
        }],
        ..ScenarioConfig::new(
            id,
            "World Ships".to_string(),
            game_assets.cubemap.clone().into(),
        )
    };
    let errors = lint_errors(&scenario, sections);
    assert!(
        errors.is_empty(),
        "world_ships: a posed ship is content the game would refuse:\n  {}",
        errors.join("\n  ")
    );
    scenario
}

fn ship_action(
    name: &str,
    position: Vec3,
    rotation: Quat,
    ship: SpaceshipConfig,
) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: name.to_string(),
            name: name.to_string(),
            position: Meters3::from_engine(position),
            rotation,
        },
        kind: ScenarioObjectKind::Spaceship(ship),
    })
}

/// The player's fire keys for every weapon section of `design`, by kind:
/// turrets on the left mouse button, bays on `F`, rails on `R`, as the
/// `wfc_arena` player slot binds them.
fn weapon_bindings(
    design: &ShipDesign,
    sections: &GameSections,
) -> std::collections::BTreeMap<String, Vec<InputSource>> {
    design
        .sections
        .iter()
        .filter_map(|section| {
            let SectionSource::Prototype { id, .. } = &section.source else {
                return None;
            };
            let config = sections
                .get_section(id)
                .unwrap_or_else(|| panic!("world_ships: '{id}' is not in the merged catalog"));
            let bindings: Vec<InputSource> = match config.kind {
                SectionKind::Turret(_) => vec![MouseButton::Left.into()],
                SectionKind::Torpedo(_) => vec![KeyCode::KeyF.into()],
                SectionKind::Railgun(_) => vec![KeyCode::KeyR.into()],
                _ => return None,
            };
            Some((section.id.clone(), bindings))
        })
        .collect()
}

/// Frame every camera the loader spawns on the posed view.
fn frame_new_camera(
    scene: Res<SceneType>,
    stage: Res<Stage>,
    mut q_camera: Query<&mut Transform, (With<ScenarioCameraMarker>, Added<ScenarioCameraMarker>)>,
) {
    let SceneType::View(view) = *scene else {
        return;
    };
    let (position, target) = view_camera(&stage, view);
    for mut transform in &mut q_camera {
        *transform = Transform::from_translation(position).looking_at(target, Vec3::Y);
    }
}

/// Where the camera stands for a posed view: high and in front, backed off to
/// hold the whole stand.
fn view_camera(stage: &Stage, view: ViewType) -> (Vec3, Vec3) {
    let ships = view_ships(stage, view);
    let clearances: Vec<f32> = ships.iter().map(|ship| ship.2).collect();
    let positions = stand_positions(&clearances, ShipRoleType::ALL.len());
    let reach = positions
        .iter()
        .zip(&clearances)
        .map(|(position, clearance)| position.length() + clearance)
        .fold(1.0, f32::max);
    (Vec3::new(0.0, reach * 0.8, reach * 1.4), Vec3::ZERO)
}

/// Marks the readout.
#[derive(Component)]
struct Readout;

fn spawn_readout(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Readout,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn update_readout(
    scene: Res<SceneType>,
    stage: Res<Stage>,
    pending: Option<Res<PendingScene>>,
    mut q_readout: Query<&mut Text, With<Readout>>,
) {
    let body = match *scene {
        SceneType::View(ViewType::Wfc) => {
            "WFC standard plan, bare, seeds 20260815..20260818".to_string()
        }
        SceneType::View(ViewType::Wrecks) => "DERELICT wrecks, bare, systems disabled\n\
             former role by column: civilian, industrial, scavenger, armored\n\
             weathered/damaged section art: none in the snapshot (missing)"
            .to_string(),
        SceneType::View(view) => format!(
            "generated ships, {} advancement, bare\n\
             columns: civilian, industrial, scavenger, armored\n\
             front row: centred single drive; back row: mirrored pair",
            view.label()
        ),
        SceneType::Pilot(index) => {
            let ship = stage.matrix.flown().nth(index);
            format!(
                "pilot {} seed {}\n[W] main drive  mouse turns  hold raise weapons: \
                 [LMB] turrets [F] bays [R] rails",
                ship.map(MatrixShip::name).unwrap_or_default(),
                ship.map(|ship| ship.seed).unwrap_or_default(),
            )
        }
    };
    let waiting = if pending.is_some() {
        "\nwaiting for the snapshot's catalog"
    } else {
        ""
    };
    let line = format!(
        "WORLD SHIPS  sources: {}\n{body}{waiting}\n\
         [1] low [2] high [3] WFC [4] wrecks  [P] pilot [N] next [B] back",
        stage.mods.join(", ")
    );
    for mut text in &mut q_readout {
        if text.as_str() != line {
            **text = line.clone();
        }
    }
}

/// While present, turn the chase camera, and so the piloted hull, one push a
/// frame.
#[derive(Resource)]
struct TurnPush {
    frames_left: u32,
}

fn push_turn(
    mut commands: Commands,
    mut push: ResMut<TurnPush>,
    mut motion: MessageWriter<MouseMotion>,
) {
    if push.frames_left == 0 {
        commands.remove_resource::<TurnPush>();
        return;
    }
    push.frames_left -= 1;
    motion.write(MouseMotion { delta: TURN_PIXELS });
}

/// The piloted ship's root.
#[cfg(feature = "debug")]
fn pilot_ship(world: &mut World) -> Entity {
    let mut query =
        world.query_filtered::<Entity, (With<PlayerSpaceshipMarker>, With<SpaceshipRootMarker>)>();
    query
        .iter(world)
        .next()
        .expect("world_ships: no piloted ship")
}

/// What the pilot beats measure against: the bow and the app clock at the
/// start of a beat, and how much the ship had fired.
#[cfg(feature = "debug")]
#[derive(Resource, Clone, Copy)]
struct PilotMark {
    forward: Vec3,
    fired: usize,
    started: f32,
}

/// The piloted ship's bow and velocity, read without a mutable world so a
/// step predicate can ask.
#[cfg(feature = "debug")]
fn pilot_motion(world: &World) -> Option<(Vec3, Vec3)> {
    let mut query = world.try_query_filtered::<(&GlobalTransform, &LinearVelocity), (
        With<PlayerSpaceshipMarker>,
        With<SpaceshipRootMarker>,
    )>()?;
    query
        .iter(world)
        .next()
        .map(|(transform, velocity)| (transform.forward().as_vec3(), velocity.0))
}

/// The speed the piloted ship has gained along the bow it had at the mark.
#[cfg(feature = "debug")]
fn burn_speed(world: &World) -> Option<MetersPerSecond> {
    let mark = world.get_resource::<PilotMark>()?;
    let (_, velocity) = pilot_motion(world)?;
    Some(MetersPerSecond::from_engine(velocity.dot(mark.forward)))
}

/// How far the piloted ship's bow has swung since the mark, in degrees.
#[cfg(feature = "debug")]
fn turn_degrees(world: &World) -> Option<f32> {
    let mark = world.get_resource::<PilotMark>()?;
    let (forward, _) = pilot_motion(world)?;
    Some(mark.forward.angle_between(forward).to_degrees())
}

/// Advance once the burn has reached [`MIN_BURN_SPEED`] along the bow.
#[cfg(feature = "debug")]
fn burn_reached() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| {
        burn_speed(world).is_some_and(|speed| speed.0 >= MIN_BURN_SPEED.0)
    })
}

/// Advance once the bow has swung [`MIN_TURN_DEGREES`].
#[cfg(feature = "debug")]
fn turn_reached() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| {
        turn_degrees(world).is_some_and(|degrees| degrees >= MIN_TURN_DEGREES)
    })
}

/// App seconds since the mark.
#[cfg(feature = "debug")]
fn since_mark(world: &World) -> f32 {
    world.resource::<Time>().elapsed_secs() - world.resource::<PilotMark>().started
}

/// Rounds the piloted ship has spent plus projectiles alive: evidence that a
/// weapon fired, whichever kind it is.
#[cfg(feature = "debug")]
fn fired(world: &mut World, ship: Entity) -> usize {
    let mut ammo = world.query::<(&SectionAmmo, &ChildOf)>();
    let spent: u32 = ammo
        .iter(world)
        .filter(|(_, ChildOf(parent))| *parent == ship)
        .map(|(ammo, _)| ammo.capacity - ammo.rounds)
        .sum();
    let mut bullets = world.query_filtered::<(), With<TurretBulletProjectileMarker>>();
    let mut torpedoes = world.query_filtered::<(), With<TorpedoProjectileMarker>>();
    let mut slugs = world.query_filtered::<(), With<RailgunSlugProjectileMarker>>();
    spent as usize
        + bullets.iter(world).count()
        + torpedoes.iter(world).count()
        + slugs.iter(world).count()
}

#[cfg(feature = "debug")]
fn mark_pilot(world: &mut World) {
    let ship = pilot_ship(world);
    let forward = world
        .get::<GlobalTransform>(ship)
        .expect("a ship root has a transform")
        .forward()
        .as_vec3();
    let fired = fired(world, ship);
    let started = world.resource::<Time>().elapsed_secs();
    world.insert_resource(PilotMark {
        forward,
        fired,
        started,
    });
}

#[cfg(feature = "debug")]
fn log_pilot(world: &mut World, line: String) {
    info!("world_ships: {line}");
    world.resource_mut::<Stage>().pilot_log.push(line);
}

#[cfg(feature = "debug")]
fn pilot_name(world: &World) -> String {
    let SceneType::Pilot(index) = *world.resource::<SceneType>() else {
        panic!("world_ships: a pilot beat outside a pilot scene");
    };
    world
        .resource::<Stage>()
        .matrix
        .flown()
        .nth(index)
        .map(MatrixShip::name)
        .expect("a pilot scene names a matrix ship")
}

/// Log how long the burn took to reach its speed.
#[cfg(feature = "debug")]
fn log_burn(world: &mut World) {
    let speed = burn_speed(world).expect("the burn step held until the ship moved");
    let seconds = since_mark(world);
    let name = pilot_name(world);
    log_pilot(
        world,
        format!(
            "{name}: burn reached {:.1} m/s along the bow in {seconds:.1} s",
            speed.0
        ),
    );
}

/// Log how long the turn took to swing the bow.
#[cfg(feature = "debug")]
fn log_turn(world: &mut World) {
    let degrees = turn_degrees(world).expect("the turn step held until the bow swung");
    let seconds = since_mark(world);
    let name = pilot_name(world);
    log_pilot(
        world,
        format!("{name}: turn reached {degrees:.1} degrees in {seconds:.1} s"),
    );
}

/// A fighter must have fired.
#[cfg(feature = "debug")]
fn check_fire(world: &mut World) {
    let ship = pilot_ship(world);
    let mark = *world.resource::<PilotMark>();
    let now = fired(world, ship);
    let name = pilot_name(world);
    assert!(
        now > mark.fired,
        "world_ships: {name} fired nothing with weapons raised and every trigger held"
    );
    log_pilot(
        world,
        format!(
            "{name}: fired, {} rounds and live projectiles",
            now - mark.fired
        ),
    );
}

/// Drift a canister of one hull plate into the piloted ship's intake, square
/// to its door, closing well under the capture speed.
#[cfg(feature = "debug")]
fn offer_canister(world: &mut World) {
    let ship = pilot_ship(world);
    offer_canister_to(world, ship);
}

/// Drift a canister of one hull plate into `ship`'s first intake, as
/// [`offer_canister`] does.
#[cfg(feature = "debug")]
fn offer_canister_to(world: &mut World, ship: Entity) {
    let mut intakes = world.query_filtered::<(
        &GlobalTransform,
        &SectionCollider,
        &CargoIntakeSectionConfigHelper,
        &ChildOf,
    ), With<CargoIntakeSectionMarker>>();
    let (transform, collider, config) = intakes
        .iter(world)
        .find(|(.., ChildOf(parent))| *parent == ship)
        .map(|(transform, collider, config, _)| (*transform, *collider, (**config).clone()))
        .expect("world_ships: the ship carries an intake");
    let (_, rotation, translation) = transform.to_scale_rotation_translation();
    let (face, normal) = cargo_intake_face(translation, rotation, collider);
    let depth = CARGO_CANISTER_SIZE.z * 0.5;
    let at = face + normal * (config.capture_gap.to_engine() + depth + CANISTER_STANDOFF);
    let closing = config.maximum_capture_speed.to_engine() * CANISTER_CLOSING_SHARE;
    let ship_velocity = world
        .get::<LinearVelocity>(ship)
        .map(|velocity| velocity.0)
        .unwrap_or_default();
    world.spawn(cargo_canister(
        CargoCanister::new(ItemType::HullPlate, 1),
        Transform::from_translation(at).with_rotation(rotation),
        ship_velocity - normal * closing,
        config.canister_mesh.clone(),
    ));
}

/// Every posed wreck's name and root, in `Matrix::wrecks` order.
#[cfg(feature = "debug")]
fn wreck_roots(world: &mut World) -> Vec<(String, Entity)> {
    let names: Vec<String> = world
        .resource::<Stage>()
        .matrix
        .wrecks
        .iter()
        .map(WreckShip::name)
        .collect();
    let mut query = world.query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>();
    let roots: Vec<(Entity, String)> = query
        .iter(world)
        .map(|(root, id)| (root, id.0.clone()))
        .collect();
    names
        .into_iter()
        .map(|name| {
            let root = roots
                .iter()
                .find(|(_, id)| *id == name)
                .map(|(root, _)| *root)
                .unwrap_or_else(|| panic!("world_ships: {name} is not standing"));
            (name, root)
        })
        .collect()
}

/// Every section entity under `root`.
#[cfg(feature = "debug")]
fn root_sections(world: &mut World, root: Entity) -> Vec<Entity> {
    let mut query = world.query_filtered::<(Entity, &ChildOf), With<SectionMarker>>();
    query
        .iter(world)
        .filter(|(_, ChildOf(parent))| *parent == root)
        .map(|(section, _)| section)
        .collect()
}

/// Every wreck is off from its first frame: each section of its design
/// stands under its root, joined through the integrity graph, at full health
/// and with a collider; each hull section is live and every other section is
/// disabled in place; nothing drives it, it sides with nobody and every ship
/// capability is off.
#[cfg(feature = "debug")]
fn check_wrecks_off(world: &mut World) {
    let designs: Vec<usize> = world
        .resource::<Stage>()
        .matrix
        .wrecks
        .iter()
        .map(|wreck| wreck.layout.design.sections.len())
        .collect();
    for ((name, root), design) in wreck_roots(world).into_iter().zip(designs) {
        let ship = world.entity(root);
        assert!(
            matches!(
                ship.get::<SpaceshipController>(),
                Some(SpaceshipController::None)
            ) && !ship.contains::<PlayerSpaceshipMarker>()
                && !ship.contains::<AISpaceshipMarker>(),
            "world_ships: {name} has a controller"
        );
        assert_eq!(
            ship.get::<Allegiance>(),
            Some(&Allegiance::Neutral),
            "world_ships: {name} is not Neutral"
        );
        let capabilities = ship
            .get::<ShipCapabilities>()
            .expect("a ship root states its capabilities");
        assert!(
            !(capabilities.stop_enabled
                || capabilities.goto_enabled
                || capabilities.orbit_enabled
                || capabilities.lock_enabled
                || capabilities.rcs_enabled
                || capabilities.point_defense_enabled
                || capabilities.dock_enabled),
            "world_ships: {name} keeps a capability: {capabilities:?}"
        );
        assert!(
            !ship.contains::<RadarState>(),
            "world_ships: {name} has a radar search"
        );

        let sections = root_sections(world, root);
        assert_eq!(
            sections.len(),
            design,
            "world_ships: {name} stands {} of its {design} sections",
            sections.len()
        );
        let mut hulls = 0;
        for section in &sections {
            let entity = world.entity(*section);
            let id = entity
                .get::<EntityId>()
                .map(|id| id.0.clone())
                .unwrap_or_default();
            let hull = entity.contains::<HullSectionMarker>();
            hulls += usize::from(hull);
            assert_eq!(
                entity.contains::<SectionInactiveMarker>(),
                !hull,
                "world_ships: {name} section '{id}' is {}",
                if hull {
                    "a disabled hull"
                } else {
                    "a live active section"
                }
            );
            let health = entity.get::<Health>().expect("a live section has health");
            assert!(
                health.current == health.max && health.max > 0.0,
                "world_ships: {name} section '{id}' starts at {health:?}"
            );
            assert!(
                entity.contains::<Collider>(),
                "world_ships: {name} section '{id}' has no collider"
            );
        }
        let mut reached = std::collections::HashSet::from([sections[0]]);
        let mut queue = vec![sections[0]];
        while let Some(section) = queue.pop() {
            for next in world
                .get::<ConnectedTo>(section)
                .map(|connected| connected.0.clone())
                .unwrap_or_default()
            {
                if reached.insert(next) {
                    queue.push(next);
                }
            }
        }
        assert_eq!(
            reached.len(),
            sections.len(),
            "world_ships: {name}'s integrity graph reaches {} of {} sections",
            reached.len(),
            sections.len()
        );
        log_pilot(
            world,
            format!(
                "{name}: {} sections joined under one root at full health with colliders; {hulls} \
                 hull live, {} active disabled; no controller, Neutral, every capability off, \
                 no radar",
                sections.len(),
                sections.len() - hulls
            ),
        );
    }
}

/// Where each wreck stood and how much it had fired when its commands began.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct WreckMark(Vec<(Entity, Quat, usize)>);

/// Command every wreck as a pilot would: full drive, a quarter turn, weapons
/// hot and every trigger held. [`WreckCommand`] holds them every frame.
#[cfg(feature = "debug")]
fn command_wrecks(world: &mut World) {
    let mut marks = Vec::new();
    for (_, root) in wreck_roots(world) {
        let rotation = world
            .get::<GlobalTransform>(root)
            .expect("a ship root has a transform")
            .rotation();
        let fired = fired(world, root);
        marks.push((root, rotation, fired));
        world.entity_mut(root).insert(WeaponsHot(true));
    }
    world.insert_resource(WreckMark(marks));
    world.insert_resource(WreckCommand);
}

/// While present, every wreck's drives, flight computers and weapons are
/// commanded each frame.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct WreckCommand;

#[cfg(feature = "debug")]
fn hold_wreck_commands(
    mark: Res<WreckMark>,
    q_section: Query<(Entity, &ChildOf), With<SectionMarker>>,
    mut q_thruster: Query<&mut ThrusterSectionInput>,
    mut q_controller: Query<&mut ControllerSectionRotationInput>,
    mut q_turret: Query<&mut TurretSectionInput>,
    mut q_bay: Query<&mut TorpedoSectionInput>,
    mut q_rail: Query<&mut RailgunSectionInput>,
) {
    for (section, ChildOf(root)) in &q_section {
        let Some((_, rotation, _)) = mark.0.iter().find(|(wreck, ..)| wreck == root) else {
            continue;
        };
        if let Ok(mut input) = q_thruster.get_mut(section) {
            input.0 = 1.0;
        }
        if let Ok(mut input) = q_controller.get_mut(section) {
            input.0 = *rotation * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        }
        if let Ok(mut input) = q_turret.get_mut(section) {
            input.0 = true;
        }
        if let Ok(mut input) = q_bay.get_mut(section) {
            input.0 = true;
        }
        if let Ok(mut input) = q_rail.get_mut(section) {
            input.0 = true;
        }
    }
}

/// No wreck answered its commands: none gained the speed or heading a live
/// ship's burn and turn beats need, and none fired.
#[cfg(feature = "debug")]
fn check_wrecks_still(world: &mut World) {
    world.remove_resource::<WreckCommand>();
    let marks = world.resource::<WreckMark>().0.clone();
    for ((name, root), (_, rotation, fired_before)) in wreck_roots(world).into_iter().zip(marks) {
        let thrusters = root_sections(world, root)
            .into_iter()
            .filter(|section| {
                world
                    .get::<ThrusterSectionInput>(*section)
                    .is_some_and(|input| input.0 == 1.0)
            })
            .count();
        assert!(
            thrusters > 0,
            "world_ships: {name} was never commanded to burn"
        );
        let speed = MetersPerSecond::from_engine(
            world
                .get::<LinearVelocity>(root)
                .map(|velocity| velocity.0.length())
                .unwrap_or_default(),
        );
        let turned = rotation
            .angle_between(
                world
                    .get::<GlobalTransform>(root)
                    .expect("a ship root has a transform")
                    .rotation(),
            )
            .to_degrees();
        let fired = fired(world, root) - fired_before;
        let asleep = world.entity(root).contains::<Sleeping>();
        assert!(
            speed.0 < MIN_BURN_SPEED.0 && turned < MIN_TURN_DEGREES && fired == 0,
            // Projectiles carry no owner, so `fired` counts every live one:
            // a shot from any wreck fails the first wreck checked.
            "world_ships: {name} or a wreck beside it answered its commands: {:.3} m/s, \
             {turned:.3} degrees, {fired} rounds spent and projectiles live",
            speed.0
        );
        log_pilot(
            world,
            format!(
                "{name}: {thrusters} drives at full input, weapons hot and triggers held: {:.4} m/s, \
                 {turned:.4} degrees, fired nothing, asleep {asleep}",
                speed.0
            ),
        );
    }
}

/// Offer a canister to every wreck that carries an intake.
#[cfg(feature = "debug")]
fn offer_wreck_canisters(world: &mut World) {
    for (_, root) in wreck_roots(world) {
        let has_intake = root_sections(world, root)
            .into_iter()
            .any(|section| world.entity(section).contains::<CargoIntakeSectionMarker>());
        if has_intake {
            offer_canister_to(world, root);
        }
    }
}

/// No wreck took a canister: every hold is still empty and every offered
/// canister still drifts.
#[cfg(feature = "debug")]
fn check_wreck_intakes(world: &mut World) {
    let mut canisters = world.query::<&CargoCanister>();
    let drifting = canisters.iter(world).count();
    let mut offered = 0;
    for (name, root) in wreck_roots(world) {
        let intakes = root_sections(world, root)
            .into_iter()
            .filter(|section| {
                world
                    .entity(*section)
                    .contains::<CargoIntakeSectionMarker>()
            })
            .count();
        let taken = world
            .get::<ShipInventory>(root)
            .map(|inventory| inventory.count(ItemType::HullPlate))
            .unwrap_or_default();
        assert_eq!(
            taken, 0,
            "world_ships: {name}'s disabled intake took a canister"
        );
        if intakes > 0 {
            offered += 1;
            log_pilot(
                world,
                format!("{name}: disabled intakes {intakes}, offered a canister, took nothing"),
            );
        }
    }
    assert!(
        offered > 0,
        "world_ships: no wreck carries an intake to offer"
    );
    assert_eq!(
        drifting, offered,
        "world_ships: {offered} canisters offered, {drifting} still drift"
    );
}

/// Hull health each wreck's first hull section had before a hit.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct WreckHits(Vec<(Entity, f32)>);

/// Hit one hull section of every wreck for a tenth of its health.
#[cfg(feature = "debug")]
fn hit_wrecks(world: &mut World) {
    let mut hits = Vec::new();
    for (_, root) in wreck_roots(world) {
        let section = root_sections(world, root)
            .into_iter()
            .find(|section| world.entity(*section).contains::<HullSectionMarker>())
            .expect("a wreck keeps hull sections");
        let (current, max) = world
            .get::<Health>(section)
            .map(|health| (health.current, health.max))
            .expect("a hull section has health");
        hits.push((section, current));
        apply_damage(
            &mut world.commands(),
            section,
            None,
            max * 0.1,
            DamageType::Kinetic,
            None,
        );
    }
    world.flush();
    world.insert_resource(WreckHits(hits));
}

/// Every hit took health off its hull section.
#[cfg(feature = "debug")]
fn check_wreck_hits(world: &mut World) {
    let hits = world.resource::<WreckHits>().0.clone();
    for ((name, _), (section, before)) in wreck_roots(world).into_iter().zip(hits) {
        let after = world
            .get::<Health>(section)
            .map(|health| health.current)
            .unwrap_or_else(|| panic!("world_ships: {name}'s hit section is gone"));
        assert!(
            after < before,
            "world_ships: {name}'s hull section took no damage: {before} -> {after}"
        );
        log_pilot(
            world,
            format!("{name}: a hull hit took its section from {before:.0} to {after:.0} health"),
        );
    }
}

/// Why an offered canister was not taken: every canister in the piloted
/// ship's intake frame, its velocity against the ship and whether it sleeps,
/// each intake pair's ready flag and the hold's free mass.
#[cfg(feature = "debug")]
fn diagnose_intake(world: &World) -> String {
    let mut out = String::new();
    let Some(mut ships) = world.try_query_filtered::<(Entity, &ShipInventory, &LinearVelocity), (
        With<PlayerSpaceshipMarker>,
        With<SpaceshipRootMarker>,
    )>() else {
        return "no ship query".to_string();
    };
    let Some((ship, inventory, ship_velocity)) = ships.iter(world).next() else {
        return "no piloted ship".to_string();
    };
    let _ = write!(
        out,
        "hold free {}; ",
        kg_text(u64::from(inventory.free_g()))
    );
    let Some(mut intakes) = world.try_query_filtered::<(Entity, &GlobalTransform, &SectionCollider, &ChildOf), With<CargoIntakeSectionMarker>>() else {
        return out;
    };
    let Some(mut canisters) = world.try_query::<(
        Entity,
        &CargoCanister,
        &GlobalTransform,
        &LinearVelocity,
        Has<Sleeping>,
    )>() else {
        return out;
    };
    for (intake, transform, collider, ChildOf(parent)) in intakes.iter(world) {
        if *parent != ship {
            continue;
        }
        let (_, rotation, translation) = transform.to_scale_rotation_translation();
        let (face, normal) = cargo_intake_face(translation, rotation, *collider);
        let _ = write!(
            out,
            "intake {intake:?} half extents {:?}; ",
            collider.aabb_half_extents()
        );
        for (canister, _, canister_transform, velocity, asleep) in canisters.iter(world) {
            let local = rotation.inverse() * (canister_transform.translation() - face);
            let relative = velocity.0 - ship_velocity.0;
            let _ = write!(
                out,
                "canister {canister:?} face-local {local:.2?} closing {:.3} across {:.3} \
                 asleep {asleep}; ",
                -relative.dot(normal),
                relative.reject_from(normal).length()
            );
        }
    }
    if let Some(readiness) = world.get_resource::<CargoPickupReadiness>() {
        let _ = write!(out, "pairs {:?}", readiness.pairs);
    }
    out
}

/// Advance once the piloted ship's hold carries the offered plate.
#[cfg(feature = "debug")]
fn canister_taken() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&ShipInventory, With<PlayerSpaceshipMarker>>()
            .and_then(|mut query| {
                query
                    .iter(world)
                    .next()
                    .map(|inventory| inventory.count(ItemType::HullPlate) >= 1)
            })
            .unwrap_or(false)
    })
}

/// Advance once `scene` is up: nothing pending, the game playing and the
/// scene's first ship spawned. Every scene's first ship id is its own, so the
/// ship of the scene before cannot stand in for it.
#[cfg(feature = "debug")]
fn scene_standing(scene: SceneType) -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(move |world: &World| {
        if world.contains_resource::<PendingScene>()
            || world.get_resource::<State<GameStates>>().map(State::get)
                != Some(&GameStates::Playing)
        {
            return false;
        }
        let first = first_ship_id(world.resource::<Stage>(), scene);
        world
            .try_query_filtered::<&EntityId, With<SpaceshipRootMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|id| id.0 == first))
    })
}

/// The id of the first ship `scene` spawns.
#[cfg(feature = "debug")]
fn first_ship_id(stage: &Stage, scene: SceneType) -> String {
    match scene {
        SceneType::View(view) => view_ships(stage, view)
            .into_iter()
            .next()
            .map(|(name, ..)| name)
            .expect("a view stands at least one ship"),
        SceneType::Pilot(index) => stage
            .matrix
            .flown()
            .nth(index)
            .map(MatrixShip::name)
            .expect("a pilot scene names a matrix ship"),
    }
}

/// Ask for `scene` unless it is already up.
#[cfg(feature = "debug")]
fn ask_scene(world: &mut World, scene: SceneType) {
    if *world.resource::<SceneType>() != scene || world.contains_resource::<PendingScene>() {
        world.insert_resource(PendingScene(scene));
    }
}

/// Write the report under the capture dir, on the capture path only.
///
/// # Panics
///
/// When the file cannot be written: a capture run that silently lost its
/// report would look complete.
#[cfg(feature = "debug")]
fn write_report(world: &mut World) {
    let stage = world.resource::<Stage>();
    let mut report = stage.report.clone();
    let _ = writeln!(report, "\n## Pilot\n");
    for line in &stage.pilot_log {
        let _ = writeln!(report, "- {line}");
    }
    info!("world_ships pilot log:\n{}", stage.pilot_log.join("\n"));
    if !capturing() {
        return;
    }
    let path = match std::env::var(nova_autopilot::prelude::CAPTURE_DIR_ENV) {
        Ok(dir) if !dir.is_empty() => std::path::Path::new(&dir).join(REPORT_FILE),
        _ => std::path::PathBuf::from(REPORT_FILE),
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("world_ships: {parent:?}: {error}"));
    }
    std::fs::write(&path, report).unwrap_or_else(|error| panic!("world_ships: {path:?}: {error}"));
    info!("nova capture: {}", path.display());
}

/// The run gate: shoot the four views, check and command the wrecks, fly
/// every matrix ship in `flown` (name and role, in `Matrix::flown` order),
/// write the report.
#[cfg(feature = "debug")]
fn world_ships_script(
    flown: Vec<(String, ShipRoleType)>,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the first scene")
        .enter(GameStates::Loading)
        // `--pilot` starts on a pilot scene; wait for whichever scene is up.
        .until(std::sync::Arc::new(|world: &World| {
            world
                .get_resource::<SceneType>()
                .is_some_and(|scene| scene_standing(*scene)(world))
        }))
        .deadline(STEP_DEADLINE_SECS)
        .add();
    for view in ViewType::ALL {
        let shot = format!("world-ships-{}.png", view.label());
        script = script
            .step(format!("stand the {} view", view.label()))
            .on_enter(move |world: &mut World| {
                ask_scene(world, SceneType::View(view));
            })
            .until(and(
                scene_standing(SceneType::View(view)),
                scenario_camera_present(),
            ))
            .deadline(STEP_DEADLINE_SECS)
            .add();
        if view == ViewType::Wrecks {
            script = script
                .step("check the wrecks are off")
                .on_enter(check_wrecks_off)
                .until(frames(1))
                .add();
        }
        script = script
            .step(format!("frame the {} view", view.label()))
            .on_enter(move |world: &mut World| {
                let (position, target) = view_camera(world.resource::<Stage>(), view);
                pose_camera(
                    world,
                    Meters3::from_engine(position),
                    Meters3::from_engine(target),
                );
            })
            .until(frames(SETTLE_FRAMES))
            .add()
            .step(format!("clear the loading panel for {shot}"))
            .until(nova_autopilot::prelude::not(ui_node_present(LOAD_PANEL)))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("shoot {shot}"))
            .on_enter({
                let shot = shot.clone();
                move |world: &mut World| shoot(world, &shot)
            })
            .until(shot_written(shot))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
    }

    // The wrecks view is the last one shot, so its wrecks still stand.
    // The shot lands while every drive is still commanded to full thrust.
    let shot = "world-ships-wrecks-commanded.png".to_string();
    script = script
        .step("command the wrecks")
        .on_enter(command_wrecks)
        .until(elapsed(FIRE_SECS))
        .add()
        .step(format!("shoot {shot}"))
        .on_enter({
            let shot = shot.clone();
            move |world: &mut World| shoot(world, &shot)
        })
        .until(shot_written(shot))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("check the wrecks held still")
        .on_enter(check_wrecks_still)
        .until(frames(1))
        .add()
        .step("offer the wreck intakes a canister")
        .on_enter(offer_wreck_canisters)
        .until(elapsed(WRECK_INTAKE_SECS))
        .add()
        .step("check the wreck intakes")
        .on_enter(check_wreck_intakes)
        .until(frames(1))
        .add()
        .step("hit the wrecks")
        .on_enter(hit_wrecks)
        .until(frames(2))
        .add()
        .step("check the wreck hits")
        .on_enter(check_wreck_hits)
        .until(frames(1))
        .add();

    for (index, (name, role)) in flown.into_iter().enumerate() {
        let shot = format!("world-ships-pilot-{name}.png");
        script = script
            .step(format!("board {name}"))
            .on_enter(move |world: &mut World| {
                ask_scene(world, SceneType::Pilot(index));
            })
            .until(and(
                scene_standing(SceneType::Pilot(index)),
                player_ship_present(),
            ))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("settle {name}"))
            .until(elapsed(1.0))
            .add();
        if role == ShipRoleType::Industrial {
            script = script
                .step(format!("take a canister into {name}"))
                .on_enter(offer_canister)
                .until(canister_taken())
                .diagnose(diagnose_intake)
                .deadline(PILOT_DEADLINE_SECS)
                .add()
                .step(format!("log the intake of {name}"))
                .on_enter(|world: &mut World| {
                    let name = pilot_name(world);
                    log_pilot(world, format!("{name}: intake took one hull plate"));
                })
                .until(frames(1))
                .add();
        }
        if matches!(role, ShipRoleType::Scavenger | ShipRoleType::Armored) {
            script = script
                .step(format!("raise the weapons of {name}"))
                .on_enter(|world: &mut World| {
                    press_action("combat_stance")(world);
                })
                .until(elapsed(DEPLOY_SECS))
                .add()
                .step(format!("fire {name}"))
                .on_enter(|world: &mut World| {
                    mark_pilot(world);
                    press_mouse(MouseButton::Left)(world);
                    press_key(KeyCode::KeyF)(world);
                    press_key(KeyCode::KeyR)(world);
                })
                .until(elapsed(FIRE_SECS))
                .add()
                .step(format!("check the fire of {name}"))
                .on_enter(|world: &mut World| {
                    check_fire(world);
                    release_mouse(MouseButton::Left)(world);
                    release_key(KeyCode::KeyF)(world);
                    release_key(KeyCode::KeyR)(world);
                    release_action("combat_stance")(world);
                })
                .until(frames(1))
                .add();
        }
        script = script
            .step(format!("burn {name}"))
            .on_enter(|world: &mut World| {
                mark_pilot(world);
                press_action("main_drive")(world);
            })
            .until(burn_reached())
            .deadline(PILOT_DEADLINE_SECS)
            .add()
            .step(format!("log the burn of {name}"))
            .on_enter(|world: &mut World| {
                release_action("main_drive")(world);
                log_burn(world);
            })
            .until(frames(1))
            .add()
            .step(format!("turn {name}"))
            .on_enter(|world: &mut World| {
                mark_pilot(world);
                world.insert_resource(TurnPush {
                    frames_left: TURN_FRAMES,
                });
            })
            .until(turn_reached())
            .deadline(PILOT_DEADLINE_SECS)
            .add()
            .step(format!("log the turn of {name}"))
            .on_enter(log_turn)
            .until(frames(1))
            .add()
            .step(format!("clear the loading panel for {shot}"))
            .until(nova_autopilot::prelude::not(ui_node_present(LOAD_PANEL)))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("shoot {shot}"))
            .on_enter({
                let shot = shot.clone();
                move |world: &mut World| shoot(world, &shot)
            })
            .until(shot_written(shot))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
    }
    script
        .step("write the report")
        .on_enter(write_report)
        .until(frames(1))
        .add()
}
