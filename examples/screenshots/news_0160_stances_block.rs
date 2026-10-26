//! news_0160_stances_block: the v0.16.0 news still of the Map chart view's
//! STANCES block with nothing selected (`news-0160-stances-block.png`).
//!
//! The world starts the way the Create World modal starts it, on
//! [`WORLD_SEED`] as `pirate`, with no save folder, and the start docks the
//! raider at its starting station. Once the window is in and the sensors
//! have charted what they see, the walk picks three civilizations, ties
//! broken by id:
//!
//! - S, the starting station's civilization: seeded Ally, charted, and
//!   left alone;
//! - X, not S and not seeded Enemy, with two living intact generated ships
//!   in the window: both are neutralized, two offences of the three that
//!   step it;
//! - Y, another civilization not S and not seeded Enemy, with one living
//!   Neutral or Ally ship: the player's damage crosses its retaliation
//!   threshold, the AI fires `ShipProvoked`, and Y reads HOSTILE for its
//!   window.
//!
//! The overview lists every civilization with a record or a charted living
//! station. On seeds 1 to 40 the start charts its own station alone, so no
//! seeded Enemy or fine stands in the overview. Seed 2 gives the three
//! required rows. X may also read HOSTILE after the two damaging hits.
//! Another seed must pass the same assertions or the walk stops loudly.
//!
//! DISCLOSED FIXTURE STATE: the kills and the provocation are damage sourced
//! from the player's ship through the production entry point,
//! `HealthApplyDamage`, as `system_civilization_stance` drives its kills.
//! The raider does not fly to them or fire a gun. Everything that follows,
//! the offences, the provocation and the stance block, is the game's own.
//!
//! The walk opens the interface with the real key, selects the Map tab,
//! presses the real chart key and asserts the block draws the three names,
//! `OFFENCES 2/3` and a `HOSTILE` countdown, with no `SEEDED`, no `FINE` and
//! no Pay fine button, before the shot.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the whole walk with its
//!   asserts, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNG, staged under
//!   `NOVA_CAPTURE_DIR`.
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0160-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0160_stances_block --features debug
//! ```

#[path = "../playable/shared/open_world_stage.rs"]
mod open_world_stage;

use std::{collections::BTreeSet, sync::Arc};

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{
    nova_debug::harness::{AutopilotPlugin, Predicate},
    nova_interface::pane::InterfacePaneType,
    prelude::*,
};
use nova_world::prelude::WorldClock;
use open_world_stage::{body_by_id, the_player, window_settled, OpenWorldStartPlugin};

#[derive(Parser)]
#[command(name = "news_0160_stances_block")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the v0.16.0 STANCES overview with offences and a provocation",
    long_about = None
)]
struct Cli {
    /// The world seed. Another seed must still chart exactly the three
    /// civilizations the walk needs.
    #[arg(long, default_value_t = WORLD_SEED)]
    seed: u32,
}

/// The seed the figure plays.
const WORLD_SEED: u32 = 2;

/// The start role the walk flies.
const ROLE: &str = "pirate";

/// The offences X takes: one short of the step.
const OFFENCES: usize = 2;

/// The damage the provocation deals: twice the threshold, so the drain
/// between the hit and the tally cannot hold it under.
const PROVOKE_HP: f32 = 2.0 * NEUTRAL_RETALIATION_THRESHOLD;

/// The still this producer writes.
const SHOT_FILE: &str = "news-0160-stances-block.png";

/// Real seconds the world gets to start, stream and dock on a software
/// rasterizer.
const SETUP_SECS: f32 = 600.0;

/// Frames the sensors get to chart the window once it is in.
const CHART_FRAMES: u32 = 30;

/// Real seconds a beat gets.
const BEAT_SECS: f32 = 60.0;

/// Frames the pane gets to draw before the shot.
const PANE_FRAMES: u32 = 10;

/// Real seconds a frame capture gets on a software rasterizer.
const SHOT_SECS: f32 = 120.0;

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = editor_app(true, None);
    app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
        bevy::ecs::error::panic,
    ));
    app.add_plugins(OpenWorldStartPlugin {
        seed: cli.seed,
        role: ROLE.to_string(),
    });
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    app.add_systems(
        Update,
        open_world_stage::hold_the_walk_clock.run_if(in_state(GameStates::Playing)),
    );
    app.add_plugins(script(cli.seed));
    app.run()
}

/// The three civilizations the block shows and the hulls the walk hits.
#[derive(Resource, Clone, Debug)]
struct Cast {
    /// Takes [`OFFENCES`] offences.
    offended: CivilizationId,
    /// Its ships the walk neutralizes.
    victims: Vec<Entity>,
    /// Is provoked.
    provoked: CivilizationId,
    /// Its ship the walk provokes.
    provoked_ship: Entity,
    /// The starting station's: seeded Ally and charted.
    start: CivilizationId,
}

impl Cast {
    fn civilizations(&self) -> BTreeSet<CivilizationId> {
        BTreeSet::from([self.offended, self.provoked, self.start])
    }
}

/// The window is in and the start docked the raider at its starting
/// station.
#[derive(Resource)]
struct StartReady;

/// The civilizations the overview lists: every one with a record or a
/// charted living station, as the Map's `overview_rows` collects them.
fn overview_civilizations(world: &World) -> BTreeSet<CivilizationId> {
    let mut civs: BTreeSet<CivilizationId> = world
        .resource::<CivilizationStances>()
        .entries()
        .map(|(id, _)| *id)
        .collect();
    civs.extend(
        world
            .resource::<Chart>()
            .stations()
            .iter()
            .filter(|station| !station.derelict)
            .filter_map(|station| station.civilization),
    );
    civs
}

/// One living, intact generated ship the walk may hit.
struct Hull {
    entity: Entity,
    id: GeneratedBodyId,
    civilization: CivilizationId,
    allegiance: Allegiance,
}

/// Every living, intact generated ship in the world, by id.
fn hulls(world: &mut World) -> Vec<Hull> {
    let mut q_hulls = world
        .query_filtered::<(Entity, &GeneratedBodyId, &TraderMarket, &Allegiance), (
            With<SpaceshipRootMarker>,
            With<AISpaceshipMarker>,
            Without<StationMarker>,
            Without<DerelictShipMarker>,
            Without<NeutralizedMarker>,
            Without<PlayerSpaceshipMarker>,
            Without<PlayerFleetMarker>,
        )>();
    let mut hulls: Vec<Hull> = q_hulls
        .iter(world)
        .map(|(entity, id, market, allegiance)| Hull {
            entity,
            id: id.clone(),
            civilization: market.civilization,
            allegiance: *allegiance,
        })
        .collect();
    hulls.sort_by(|a, b| a.id.cmp(&b.id));
    hulls
}

/// Pick S, X and Y from what the window and the chart hold.
///
/// # Panics
///
/// When the seed charts a civilization outside the three, or holds no X or
/// Y: the message lists what it saw.
fn cast_the_civilizations(world: &mut World, seed: u32) {
    let start_id = world.resource::<OpenWorldSession>().start.id.clone();
    let start_station =
        body_by_id(world, &start_id).expect("stances block: the starting station stands");
    let start = world
        .get::<TraderMarket>(start_station)
        .expect("stances block: the starting station prices goods")
        .civilization;
    let hulls = hulls(world);
    let charted = overview_civilizations(world);
    let seen: Vec<String> = charted
        .iter()
        .map(|civ| {
            let ships = hulls
                .iter()
                .filter(|hull| hull.civilization == *civ)
                .count();
            format!(
                "{} seeded {:?}, {ships} ship(s)",
                civ.name(),
                seeded_stance(civ)
            )
        })
        .chain(hulls.iter().map(|hull| {
            format!(
                "ship '{}' of {} ({:?})",
                hull.id,
                hull.civilization.name(),
                hull.allegiance
            )
        }))
        .collect();
    let of = |civ: &CivilizationId| -> Vec<&Hull> {
        hulls
            .iter()
            .filter(|hull| hull.civilization == *civ)
            .collect()
    };
    let castable = |civ: &CivilizationId| *civ != start && seeded_stance(civ) != StanceType::Enemy;
    let mut civs: Vec<CivilizationId> = hulls.iter().map(|hull| hull.civilization).collect();
    civs.sort();
    civs.dedup();
    let offended = civs
        .iter()
        .find(|civ| castable(civ) && of(civ).len() >= OFFENCES)
        .copied();
    let provoked = offended.and_then(|offended| {
        civs.iter()
            .find(|civ| {
                **civ != offended
                    && castable(civ)
                    && of(civ).iter().any(|hull| {
                        matches!(hull.allegiance, Allegiance::Neutral | Allegiance::Player)
                    })
            })
            .copied()
    });
    let (Some(offended), Some(provoked)) = (offended, provoked) else {
        panic!(
            "stances block: seed {seed} gives no cast: offended {offended:?}, provoked \
             {provoked:?}; saw {seen:#?}"
        );
    };
    assert_eq!(
        seeded_stance(&start),
        StanceType::Ally,
        "stances block: the starting station's '{}' is not seeded Ally",
        start.name()
    );
    let cast = Cast {
        offended,
        victims: of(&offended)
            .iter()
            .take(OFFENCES)
            .map(|hull| hull.entity)
            .collect(),
        provoked,
        provoked_ship: of(&provoked)
            .iter()
            .find(|hull| matches!(hull.allegiance, Allegiance::Neutral | Allegiance::Player))
            .map(|hull| hull.entity)
            .expect("stances block: the provoked civilization has a ship that tallies"),
        start,
    };
    let stray: Vec<String> = charted
        .difference(&cast.civilizations())
        .map(|civ| civ.name())
        .collect();
    assert!(
        stray.is_empty(),
        "stances block: seed {seed} charts {stray:?} besides the cast; saw {seen:#?}"
    );
    info!(
        "stances block: S '{}', X '{}', Y '{}'",
        start.name(),
        offended.name(),
        provoked.name()
    );
    world.insert_resource(cast);
}

/// Every section mounted under `root`, by its mount chain.
fn sections_of<T: Component>(world: &mut World, root: Entity) -> Vec<Entity> {
    let found: Vec<Entity> = world
        .query_filtered::<Entity, (With<T>, With<SectionMarker>)>()
        .iter(world)
        .collect();
    found
        .into_iter()
        .filter(|section| {
            let mut at = *section;
            while let Some(parent) = world.get::<ChildOf>(at).map(ChildOf::parent) {
                if parent == root {
                    return true;
                }
                at = parent;
            }
            false
        })
        .collect()
}

/// Damage `section` by `amount`, sourced from `source`.
fn damage(world: &mut World, source: Entity, section: Entity, amount: f32) {
    world.trigger(HealthApplyDamage {
        entity: section,
        source: Some(source),
        amount,
    });
}

/// Neutralize `target` with damage sourced from `player`, by the two rules
/// of `nova_gameplay::integrity::neutralize`: zero every weapon section of
/// an armed hull, else every thruster section.
fn neutralize(world: &mut World, player: Entity, target: Entity) {
    let mut critical = sections_of::<TurretSectionMarker>(world, target);
    critical.extend(sections_of::<TorpedoSectionMarker>(world, target));
    critical.extend(sections_of::<RailgunSectionMarker>(world, target));
    if critical.is_empty() {
        critical = sections_of::<ThrusterSectionMarker>(world, target);
    }
    assert!(
        !critical.is_empty(),
        "stances block: '{target:?}' carries no weapon and no thruster"
    );
    for section in critical {
        let left = world
            .get::<Health>(section)
            .map_or(0.0, |health| health.current);
        if left > 0.0 {
            damage(world, player, section, left);
        }
    }
}

/// Neutralize X's two ships, then hit Y's ship past its threshold.
fn offend_and_provoke(world: &mut World) {
    let cast = world.resource::<Cast>().clone();
    let player = the_player(world).expect("stances block: one player ship");
    for victim in &cast.victims {
        neutralize(world, player, *victim);
    }
    let section = sections_of::<SectionMarker>(world, cast.provoked_ship)
        .into_iter()
        .filter_map(|section| {
            world
                .get::<Health>(section)
                .map(|health| (section, health.current))
        })
        .filter(|(_, left)| *left > PROVOKE_HP)
        .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(section, _)| section)
        .expect("stances block: the provoked ship has a section that outlasts the hit");
    damage(world, player, section, PROVOKE_HP);
}

/// How X, Y and S stand now.
fn standings(world: &World) -> [StanceStanding; 3] {
    let cast = world.resource::<Cast>();
    let stances = world.resource::<CivilizationStances>();
    let now = world.resource::<WorldClock>().play_seconds;
    [cast.offended, cast.provoked, cast.start].map(|civ| standing(stances, &civ, now))
}

/// X holds two offences at its seeded stance, Y is provoked and S stands
/// untouched.
fn the_ledger_reads() -> Arc<Predicate> {
    when(|world| {
        world.contains_resource::<Cast>() && {
            let [x, y, _] = standings(world);
            x.offences == OFFENCES as u32 && y.provoked_left.is_some()
        }
    })
}

/// Assert the ledger the frame shows.
fn assert_the_ledger(world: &mut World) {
    let cast = world.resource::<Cast>().clone();
    let [x, y, s] = standings(world);
    assert_eq!(
        x.table,
        seeded_stance(&cast.offended),
        "stances block: two offences do not step X"
    );
    assert_eq!(x.offences, OFFENCES as u32, "stances block: X's offences");
    assert!(y.provoked_left.is_some(), "stances block: Y is provoked");
    assert_eq!(s.table, StanceType::Ally, "stances block: S stands Ally");
    assert_eq!(s.offences, 0, "stances block: S holds no offence");
    assert!(
        s.provoked_left.is_none(),
        "stances block: S is not provoked"
    );
    assert_eq!(
        overview_civilizations(world),
        cast.civilizations(),
        "stances block: the overview lists the cast alone"
    );
    info!("stances block: X {x:?}, Y {y:?}, S {s:?}");
}

fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
    Arc::new(check)
}

/// Every visible UI text laid out with a width and a height: a zero-height
/// line draws nothing.
fn drawn_texts(world: &World) -> Vec<String> {
    world
        .try_query::<(&Text, &ComputedNode, &InheritedVisibility)>()
        .map(|mut texts| {
            texts
                .iter(world)
                .filter(|(_, node, visible)| visible.get() && node.size().min_element() > 0.0)
                .map(|(text, ..)| text.0.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// The block draws the three names, X's offences and Y's countdown, and no
/// seeded note, fine or Pay fine button.
fn the_block_reads_the_ledger() -> Arc<Predicate> {
    when(|world| {
        let Some(cast) = world.get_resource::<Cast>() else {
            return false;
        };
        let texts = drawn_texts(world);
        let has = |text: &str| texts.iter().any(|drawn| drawn == text);
        let starts = |prefix: &str| texts.iter().any(|drawn| drawn.starts_with(prefix));
        cast.civilizations().iter().all(|civ| has(&civ.name()))
            && has(&format!("OFFENCES {OFFENCES}/{OFFENCE_STEP}"))
            && starts("HOSTILE ")
            && !has("SEEDED")
            && !starts("FINE ")
            && !has("Pay fine")
    })
}

/// The interface is open on `pane`.
fn the_interface_shows(pane: InterfacePaneType) -> Arc<Predicate> {
    when(move |world| {
        world
            .get_resource::<State<PauseStates>>()
            .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            && world.get_resource::<InterfacePaneType>() == Some(&pane)
    })
}

/// Press and release a key action, one frame each.
fn tap(
    script: AutopilotPlugin<GameStates>,
    label: &str,
    action: &'static str,
    landed: Arc<Predicate>,
) -> AutopilotPlugin<GameStates> {
    script
        .step(format!("stances block: press {label}"))
        .on_enter(press_action(action))
        .until(frames(1))
        .add()
        .step(format!("stances block: let {label} up"))
        .on_enter(release_action(action))
        .until(landed)
        .deadline(BEAT_SECS)
        .add()
}

/// The walk: into the world, cast, offend and provoke, open the chart view
/// and shoot the block.
fn script(seed: u32) -> AutopilotPlugin<GameStates> {
    let script = AutopilotPlugin::<GameStates>::new()
        .step("stances block: the window streams in and the start docks")
        // `window_settled` reads through a query state and needs the world
        // mutably, which a predicate never has, so each frame records it.
        .each(|world: &mut World, _, _| {
            let docked =
                the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some());
            if docked && window_settled(world) {
                world.insert_resource(StartReady);
            }
        })
        .until(when(|world| world.contains_resource::<StartReady>()))
        .deadline(SETUP_SECS)
        .add()
        .step("stances block: the sensors chart the window")
        .until(frames(CHART_FRAMES))
        .add()
        .step("stances block: cast S, X and Y")
        .on_enter(move |world: &mut World| cast_the_civilizations(world, seed))
        .add()
        .step("stances block: two offences against X and a provocation of Y")
        .on_enter(offend_and_provoke)
        .until(the_ledger_reads())
        .diagnose(|world| format!("{:?}", standings(world)))
        .deadline(BEAT_SECS)
        .add()
        .step("stances block: check the ledger")
        .on_enter(assert_the_ledger)
        .add();
    let script = tap(
        script,
        "the interface key",
        "interface_toggle",
        when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
        }),
    )
    .click_named(
        "stances block: open the Map pane",
        "InterfaceTabMap",
        the_interface_shows(InterfacePaneType::Map),
        BEAT_SECS,
    );
    tap(
        script,
        "the chart key",
        "map_chart",
        ui_node_present("MapStanceBlock"),
    )
    .step("stances block: the block reads the ledger")
    .on_enter(hide_status_bar)
    .until(the_block_reads_the_ledger())
    .diagnose(|world| format!("{:?}", drawn_texts(world)))
    .deadline(BEAT_SECS)
    .add()
    .step("stances block: settle the pane")
    .until(frames(PANE_FRAMES))
    .add()
    .step("stances block: the ledger still reads")
    .on_enter(assert_the_ledger)
    .until(the_block_reads_the_ledger())
    .deadline(BEAT_SECS)
    .add()
    .step("stances block: shoot the pane")
    .on_enter(|world: &mut World| shoot(world, SHOT_FILE))
    .until(shot_written(SHOT_FILE))
    .deadline(SHOT_SECS)
    .add()
}
