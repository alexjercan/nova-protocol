//! second_tier_sandbox: start docked at a yard that builds every second-tier
//! section, with the recipe parts and the credits aboard, and swap the
//! first-tier parts of a shipped hull for them.
//!
//! A generated yard stands in a void, Neutral and living, at advancement
//! 0.65: past the 0.6 the heavy railgun opens at. A civilization knows a part
//! above advancement 0 on a 50 % draw, so the sandbox takes the first station
//! seed whose build list names all six second-tier parts. `--hull` chooses
//! the shipped hull the player flies, by the first-tier parts it carries:
//!
//! | `--hull` | hull | swaps it can make |
//! | - | - | - |
//! | `picket` (default) | `patrol_picket` | torpedo bay to Dart bay, cargo intake to wide intake, docking port to wide port, controller to armoured controller |
//! | `tug` | `mining_tug` | mining beam to long mining beam, cargo intake, docking port, controller |
//! | `warship` | `line_warship` | railgun lance to heavy railgun, two torpedo bays, cargo intake, two docking ports, four controllers |
//!
//! THE HOLD AND THE BALANCE ARE DISCLOSED FIXTURE STATE: one recipe of the
//! second-tier part for every first-tier part on the hull, an alloy plate in
//! each, and 20,000 cr. Weapons and the mining beam take the start-role
//! keys. The dock is made by the docking verb itself, with the hull put on
//! the berth until the dock takes. What to do:
//!
//! 1. press Tab and open the Ship tab;
//! 2. click a first-tier part on the hull;
//! 3. open the Catalogue and click the second-tier part of the same kind;
//! 4. click Swap on its card. The recipe parts leave the hold and the fee
//!    goes to the yard;
//! 5. press Tab, then D to undock, and use the part: F launches a torpedo,
//!    R fires the railgun, V deploys the mining beam. A rock stands 700 m
//!    off the yard as a target.
//!
//! The diagnostics layer (F3) is on at start: in flight, one panel line per
//! swap with how many of each part the hull carries now; always, the count
//! of second-tier parts mounted and the credits.
//!
//! ```text
//! nix develop --command cargo run --example second_tier_sandbox --features debug
//! nix develop --command cargo run --example second_tier_sandbox --features debug -- --hull tug
//! nix develop --command cargo run --example second_tier_sandbox --features debug -- --hull warship
//! ```
//!
//! No deadline ends the run unless `NOVA_AUTOPILOT=1` is set. Then a smoke
//! walk checks the staging: docked and at rest, the yard builds all six
//! parts and refits, and the hold pays every swap the hull can make. It
//! opens the Ship pane and exits. `NOVA_CAPTURE=1` shoots the docked flight
//! view and the Ship pane.

#[path = "../shared/generated_station.rs"]
mod generated_station;
#[path = "shared/open_world_stage.rs"]
mod open_world_stage;
#[path = "shared/station_void.rs"]
mod station_void;

use std::collections::BTreeMap;

use bevy::prelude::*;
use clap::{Parser, ValueEnum};
use diagnostics::{palette, DiagnosticsPanel, DiagnosticsSystems};
use generated_station::generated_station_design;
use nova_protocol::{nova_debug::diagnostics, prelude::*};

/// The shipped hull the player flies.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SandboxHullType {
    /// The patrol picket: a torpedo bay, an intake, two ports.
    #[default]
    Picket,
    /// The mining tug: a mining beam, an intake, one port.
    Tug,
    /// The line warship: a railgun lance, two torpedo bays, an intake.
    Warship,
}

impl SandboxHullType {
    /// The catalog id of the hull.
    fn ship_id(self) -> &'static str {
        match self {
            Self::Picket => "patrol_picket",
            Self::Tug => "mining_tug",
            Self::Warship => "line_warship",
        }
    }
}

#[derive(Parser, Resource, Clone, Copy)]
#[command(name = "second_tier_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Start docked at a yard that builds every second-tier section, with the recipe \
             parts aboard, and swap the first-tier parts of a shipped hull for them",
    long_about = None
)]
struct Cli {
    /// The shipped hull the player flies.
    #[arg(long, value_enum, default_value_t = SandboxHullType::Picket)]
    hull: SandboxHullType,
}

/// Each first-tier part and the second-tier part a yard swaps it for.
const SWAPS: [(&str, &str); 6] = [
    ("torpedo_section", "dart_torpedo_section"),
    ("railgun_lance_section", "heavy_railgun_section"),
    ("mining_beam_section", "long_mining_beam_section"),
    ("cargo_intake_section", "wide_intake_section"),
    ("docking_port_section", "wide_docking_port_section"),
    ("basic_controller_section", "armoured_controller_section"),
];

/// The yard's scenario id and name.
const YARD_ID: &str = "yard";
const YARD_NAME: &str = "YARD";
/// The civilization's advancement: past the 0.6 the heavy railgun opens at.
const YARD_ADVANCEMENT: f32 = 0.65;
/// How many station seeds the search tries. Six 50 % draws make one seed in
/// 64 build all six parts.
const YARD_SEEDS: u32 = 2_048;
/// The yard's park, meters.
const YARD_POSITION: Meters3 = Meters3::new(0.0, 0.0, 0.0);
/// Where the player spawns, meters: clear of the largest yard, which keeps
/// 300 m. The staging puts the hull on the berth.
const PLAYER_POSITION: Meters3 = Meters3::new(0.0, 0.0, 600.0);
/// The player's ship name.
const PLAYER_NAME: &str = "Refit";
/// The fixture balance: every swap of the warship costs under 6,000 cr.
const PLAYER_CREDITS: u32 = 20_000;

/// The target rock: its id, its park and its nominal radius. The meshed
/// surface stands well past the radius.
const ROCK_ID: &str = "target_rock";
const ROCK_POSITION: Meters3 = Meters3::new(0.0, 0.0, 700.0);
const ROCK_RADIUS: Meters = Meters(15.0);
const ROCK_SEED: u32 = 7;

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    let mut app = AppBuilder::new()
        .with_game_plugins(second_tier_plugin)
        .build();
    app.insert_resource(cli);
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(
        "Docked at a yard with the parts aboard: Tab, Ship, click a part, Catalogue, Swap.",
    ));
    app.add_systems(
        PostUpdate,
        draw_the_swaps
            .in_set(DiagnosticsSystems)
            .after(TransformSystems::Propagate)
            .run_if(in_state(GameStates::Playing)),
    );

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(Startup, force_capture_resolution);
    }
    app.add_plugins(smoke::script());

    app.run()
}

fn second_tier_plugin(app: &mut App) {
    app.init_resource::<Staging>();
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_the_void);
    app.add_systems(Update, stage_the_dock.run_if(in_state(GameStates::Playing)));
}

/// The first yard whose civilization knows all six second-tier parts.
///
/// # Panics
///
/// When no seed under [`YARD_SEEDS`] builds them: the sandbox has no yard to
/// swap at.
fn the_yard(loaded: &LoadedSectionPacks, items: &GameItems) -> StationDesign {
    let (seed, yard) = (0..YARD_SEEDS)
        .map(|seed| {
            let (_, yard) = generated_station_design(
                loaded,
                items,
                StationArchetypeType::Yard,
                YARD_ADVANCEMENT,
                seed,
            );
            (seed, yard)
        })
        .find(|(_, yard)| {
            SWAPS
                .iter()
                .all(|(_, second)| yard.builds.iter().any(|id| id == second))
        })
        .unwrap_or_else(|| {
            panic!(
                "second_tier_sandbox: no yard of seeds 0..{YARD_SEEDS} at advancement \
                 {YARD_ADVANCEMENT} builds all six second-tier parts"
            )
        });
    info!(
        "second_tier_sandbox: the yard of seed {seed} builds {} parts, all six second-tier \
         parts among them",
        yard.builds.len()
    );
    yard
}

/// One recipe of the second-tier part for every first-tier part on `hull`.
///
/// # Panics
///
/// When the catalog lacks a part of [`SWAPS`].
fn recipe_stock(hull: &ResolvedShipDesign, sections: &GameSections) -> BTreeMap<ItemDesignId, u32> {
    let mut stock = BTreeMap::new();
    for mounted in &hull.sections {
        let Some((_, second)) = SWAPS
            .iter()
            .find(|(first, _)| mounted.config.base.id == *first)
        else {
            continue;
        };
        let offered = sections
            .get_section(second)
            .unwrap_or_else(|| panic!("second_tier_sandbox: the catalog has no '{second}'"));
        for (item, count) in &offered.base.build.parts {
            *stock.entry(item.clone()).or_default() += count;
        }
    }
    stock
}

fn load_the_void(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    loaded: Res<LoadedSectionPacks>,
    items: Res<GameItems>,
    designs: Res<GameShipDesigns>,
    sections: Res<GameSections>,
    cli: Res<Cli>,
) {
    let yard = the_yard(&loaded, &items);
    let design = ShipDesignSource::prototype(cli.hull.ship_id());
    let (hull, errors) = resolve_ship_design(&design, &designs, &sections);
    assert!(
        errors.is_empty(),
        "second_tier_sandbox: the hull '{}' does not resolve: {errors:?}",
        cli.hull.ship_id()
    );
    let stock = recipe_stock(&hull, &sections);
    let stock_g: u64 = stock
        .iter()
        .map(|(item, count)| u64::from(items.design(item).mass_g) * u64::from(*count))
        .sum();
    assert!(
        stock_g <= u64::from(hull.cargo_capacity_g()),
        "second_tier_sandbox: the recipe parts weigh {stock_g} g, past the hold of '{}'",
        cli.hull.ship_id()
    );
    let player = station_void::player(
        PLAYER_NAME,
        PLAYER_POSITION,
        SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: start_role_bindings(&hull),
            }),
            design,
            inventory: ShipInventoryStock::new(stock),
            credits: PLAYER_CREDITS,
            ..default()
        },
    );
    let rock = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ROCK_ID.to_string(),
            name: "Target Rock".to_string(),
            position: ROCK_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: ROCK_RADIUS,
            texture: game_assets.asteroid_texture.clone().into(),
            kind: KIND_METAL.into(),
            destroy_sound: None,
            initial_velocity: MetersPerSecond3::ZERO,
            lock_signature: None,
            seed: Some(ROCK_SEED),
        }),
    };
    commands.trigger(LoadScenario(station_void::void(
        "second_tier_sandbox",
        "Second Tier Sandbox",
        vec![
            station_void::station(
                YARD_ID,
                YARD_NAME,
                YARD_POSITION,
                StationDesignSource::Inline(yard),
            ),
            player,
            rock,
        ],
        &[(YARD_POSITION, 8.0), (ROCK_POSITION, 4.0)],
        &game_assets,
    )));
}

/// Whether the first dock was staged. The staging stops there, so the player
/// can undock and fly.
#[derive(Resource, Default)]
struct Staging {
    docked: bool,
}

/// Put the player on the yard's berth and ask to dock, each frame until the
/// dock takes.
fn stage_the_dock(world: &mut World) {
    if world.resource::<Staging>().docked {
        return;
    }
    if station_void::berth_and_dock_at(world, YARD_ID) {
        world.resource_mut::<Staging>().docked = true;
        info!("second_tier_sandbox: docked at {YARD_NAME}; TAB opens the interface");
    }
}

/// How many sections of the catalog part `id` the design `hull` places.
fn mounted(hull: &ShipDesignSections, id: &str) -> usize {
    hull.0
        .iter()
        .filter(|section| section.config.base.id == id)
        .count()
}

/// The panel: one line per swap the hull can make or has made, and the keys.
fn draw_the_swaps(
    mut panel: ResMut<DiagnosticsPanel>,
    sections: Res<GameSections>,
    pause: Option<Res<State<PauseStates>>>,
    q_player: Query<
        (
            &ShipDesignSections,
            &ShipCredits,
            &GlobalTransform,
            Has<DockedShip>,
        ),
        With<PlayerSpaceshipMarker>,
    >,
    q_yard: Query<&StationBuilds, With<StationMarker>>,
    q_rock: Query<(&EntityId, &GlobalTransform)>,
) {
    let Ok((hull, credits, at, docked)) = q_player.single() else {
        return;
    };
    let name = |id: &str| {
        sections
            .get_section(id)
            .map_or_else(|| id.to_string(), |section| section.base.name.clone())
    };
    let interface = station_void::interface_up(pause.as_deref());
    let (mut left, mut done) = (0, 0);
    for (first, second) in SWAPS {
        let (have, swapped) = (mounted(hull, first), mounted(hull, second));
        left += have;
        done += swapped;
        // The Ship pane's legend stands where the panel drops to while the
        // interface is up, so the table shows in flight only.
        if have + swapped == 0 || interface {
            continue;
        }
        let built = q_yard
            .iter()
            .any(|builds| builds.0.iter().any(|id| id == second));
        panel.line(format!(
            "{} x{have} -> {} x{swapped}{}",
            name(first),
            name(second),
            if built { "" } else { " (no yard builds it)" }
        ));
    }
    panel.line(format!(
        "Second-tier parts mounted: {done}, first-tier parts left to swap: {left}, {} cr",
        credits.0
    ));
    if let Some((_, rock)) = q_rock.iter().find(|(id, _)| id.0 == ROCK_ID) {
        let range = Meters::from_engine(rock.translation().distance(at.translation()));
        panel.label_tinted(
            rock.translation(),
            format!("target rock\n{:.0} m", range.0),
            palette::TARGET,
        );
    }
    if !docked {
        panel.hint("F", "torpedo");
        panel.hint("R", "railgun");
        panel.hint("V", "mining beam");
        panel.hint("Ctrl", "hold on the yard to lock it, then D to dock");
    } else if interface {
        panel.hint(
            "Ship",
            "click a part, Catalogue, the second-tier part, Swap",
        );
    } else {
        panel.hint("TAB", "Ship > a part > Catalogue > Swap");
        panel.hint("D", "undock");
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::{
        nova_debug::harness::{AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS},
        nova_interface::pane::InterfacePaneType,
    };

    use super::*;

    /// Seconds the walk gives the void to load and the dock to take.
    const LOAD_SECS: f32 = 300.0;
    /// Seconds any later beat may stall before the walk fails it.
    const BEAT_SECS: f32 = 60.0;
    /// Frames a view gets to settle before a shot.
    const SETTLE_FRAMES: u32 = 10;

    /// The frames the capture path writes.
    const DOCKED_SHOT: &str = "second-tier-sandbox-docked.png";
    const SHIP_SHOT: &str = "second-tier-sandbox-ship.png";

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    /// The staging: docked and at rest at a living yard that refits and
    /// builds all six parts, on a hull with a first-tier part to swap, with
    /// a hold and a balance that pay every swap.
    fn assert_the_staging(world: &mut World) {
        let yard =
            station_void::station_root(world, YARD_ID).expect("second_tier_sandbox: the yard");
        let player = station_void::player_root(world).expect("second_tier_sandbox: a player");
        assert!(
            station_void::docked_at(world, YARD_ID),
            "second_tier_sandbox: the player starts docked at the yard"
        );
        station_void::assert_at_rest(world, player, yard);
        let services = world
            .get::<StationServices>(yard)
            .expect("second_tier_sandbox: a station root carries its services");
        assert!(
            services.0.contains(&StationServiceType::Refit),
            "second_tier_sandbox: the yard must offer Refit: {services:?}"
        );
        let builds = world
            .get::<StationBuilds>(yard)
            .expect("second_tier_sandbox: a station root carries its build list")
            .0
            .clone();
        let hull = world
            .get::<ShipDesignSections>(player)
            .expect("second_tier_sandbox: the player has design sections")
            .clone();
        let (items, sections) = (
            world.resource::<GameItems>(),
            world.resource::<GameSections>(),
        );
        let (yard_hold, yard_cr, hold, credits) = (
            world.get::<ShipInventory>(yard).expect("a yard hold"),
            world.get::<ShipCredits>(yard).expect("a yard balance").0,
            world.get::<ShipInventory>(player).expect("a player hold"),
            world.get::<ShipCredits>(player).expect("a balance").0,
        );
        let mut swaps = 0;
        for (first, second) in SWAPS {
            assert!(
                builds.iter().any(|id| id == second),
                "second_tier_sandbox: the yard builds '{second}': {builds:?}"
            );
            let Some(current) = hull
                .0
                .iter()
                .find(|section| section.config.base.id == first)
            else {
                continue;
            };
            swaps += 1;
            let offered = sections
                .get_section(second)
                .expect("second_tier_sandbox: the catalog ships every second-tier part");
            assert_eq!(
                section_swap_family(&current.config, offered),
                Ok(()),
                "second_tier_sandbox: '{second}' is of the family of '{first}'"
            );
            let payment = plan_recipe_payment(
                items,
                &offered.base.build,
                yard_hold,
                yard_cr,
                hold,
                credits,
            )
            .unwrap_or_else(|refusal| {
                panic!("second_tier_sandbox: the hold cannot pay '{second}': {refusal:?}")
            });
            assert!(
                payment.bought.is_empty(),
                "second_tier_sandbox: the hold carries every part of '{second}': {payment:?}"
            );
            info!(
                "second_tier_sandbox: PASS '{first}' x{} swaps for '{second}' at {} cr from the hold",
                mounted(&hull, first),
                payment.price_cr
            );
        }
        assert!(
            swaps >= 4,
            "second_tier_sandbox: the hull carries first-tier parts to swap, found {swaps} kinds"
        );
        info!("second_tier_sandbox: PASS staged docked with {swaps} kinds of swap, {credits} cr");
    }

    /// The smoke walk. Inert unless `NOVA_AUTOPILOT` is set.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("second tier sandbox: load the void and dock")
            .enter(GameStates::Loading)
            .until(when(|world| {
                world
                    .get_resource::<Staging>()
                    .is_some_and(|staging| staging.docked)
            }))
            .deadline(LOAD_SECS)
            .add()
            .step("second tier sandbox: settle the dock")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("second tier sandbox: check the staging")
            .on_enter(assert_the_staging)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("second tier sandbox: shoot the docked view")
            .on_enter(|world: &mut World| shoot(world, DOCKED_SHOT))
            .until(shot_written(DOCKED_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("second tier sandbox: open the Ship pane")
            .on_enter(|world: &mut World| {
                station_void::open_pane(world, InterfacePaneType::Ship);
            })
            .until(ui_node_present("ShipCatalogue"))
            .deadline(BEAT_SECS)
            .add()
            .step("second tier sandbox: settle the pane")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("second tier sandbox: shoot the Ship pane")
            .on_enter(|world: &mut World| shoot(world, SHIP_SHOT))
            .until(shot_written(SHIP_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
    }
}
