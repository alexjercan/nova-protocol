//! wiki_map_names: stills of a generated planet and asteroid by name - the
//! target inset locked on the planet, then the Map's contact panel on the
//! nearest asteroid (its kind, as the inset names it) and on the planet (its
//! seeded name).
//!
//! The world starts the way the Create World modal starts it, on
//! [`WORLD_SEED`] as [`ROLE`], with no save folder. Once the start docks and
//! the window is in, the walk logs every planet the window holds by name,
//! lets go of the station, and jumps the player [`STANDOFF_RADII`] planet
//! radii off the nearest planet, bow on it. The clock pauses there, so the
//! well pulls nothing while the stills are taken.
//!
//! | # | claim |
//! | - | - |
//! | 1 | every streamed planet's [`Name`] is not its scenario id |
//! | 2 | the inset caption locked on the planet reads its name |
//! | 3 | the Map's contact panel reads the nearest asteroid's [`Name`] |
//! | 4 | the Map's contact panel reads the planet's [`Name`] |
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the walk and its asserts.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write [`INSET_SHOT`],
//!   [`MAP_ASTEROID_SHOT`] and [`MAP_PLANET_SHOT`] under `NOVA_CAPTURE_DIR`.
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/map-names NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   NOVA_AUTOPILOT_DEADLINE=3000 \
//!   cargo run --example wiki_map_names --features debug
//! ```

#[path = "../playable/shared/open_world_stage.rs"]
mod open_world_stage;

use std::sync::Arc;

use avian3d::prelude::Position;
use bevy::{prelude::*, ui_widgets::Activate};
use clap::Parser;
use nova_protocol::{
    nova_debug::harness::{AutopilotPlugin, Predicate},
    nova_interface::{map::MapContactCode, pane::InterfacePaneType},
    prelude::*,
};
use open_world_stage::{jump_facing, the_player, window_settled, OpenWorldStartPlugin};

#[derive(Parser)]
#[command(name = "wiki_map_names")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture a generated planet and asteroid by name: the locked inset and the Map's contact panel",
    long_about = None
)]
struct Cli;

/// The seed the walk plays.
const WORLD_SEED: u32 = 10;

/// The start role the walk flies.
const ROLE: &str = "transport";

/// How far off the planet's centre the player stands, in planet radii.
const STANDOFF_RADII: f32 = 3.0;

/// Real seconds the world gets to start, stream and dock on a software
/// rasterizer.
const SETUP_SECS: f32 = 600.0;

/// Real seconds a beat gets.
const BEAT_SECS: f32 = 120.0;

/// Frames a view settles before its still.
const SETTLE_FRAMES: u32 = 10;

const INSET_SHOT: &str = "map-names-inset.png";
const MAP_ASTEROID_SHOT: &str = "map-names-map-asteroid.png";
const MAP_PLANET_SHOT: &str = "map-names-map-planet.png";

const MAP_TAB: &str = "InterfaceTabMap";
const MAP_PANEL: &str = "MapPanel";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = editor_app(true, None);
    app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
        bevy::ecs::error::panic,
    ));
    app.add_plugins(OpenWorldStartPlugin {
        seed: WORLD_SEED,
        role: ROLE.to_string(),
    });
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    app.add_systems(
        Update,
        open_world_stage::hold_the_walk_clock.run_if(in_state(GameStates::Playing)),
    );
    app.add_plugins(script());
    app.run()
}

/// The window is in and the start docked the player.
#[derive(Resource)]
struct StartReady;

/// The bodies the stills name, chosen once the window is in.
#[derive(Resource, Clone, Debug)]
struct Picks {
    planet: GeneratedBodyId,
    planet_name: String,
    /// Set once the player stands off the planet.
    asteroid: Option<(Entity, String)>,
}

fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
    Arc::new(check)
}

fn player(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

fn picks(world: &World) -> &Picks {
    world
        .get_resource::<Picks>()
        .expect("map names: the bodies are picked once the window is in")
}

fn body(world: &World, id: &GeneratedBodyId) -> Option<Entity> {
    world
        .try_query::<(Entity, &GeneratedBodyId)>()?
        .iter(world)
        .find(|(_, worn)| *worn == id)
        .map(|(entity, _)| entity)
}

/// Every text under the node `name` that draws.
fn texts_under(world: &World, name: &str) -> Vec<String> {
    let Some(mut named) = world.try_query::<(Entity, &Name)>() else {
        return Vec::new();
    };
    let Some(root) = named
        .iter(world)
        .find(|(_, found)| found.as_str() == name)
        .map(|(entity, _)| entity)
    else {
        return Vec::new();
    };
    let mut texts = Vec::new();
    let mut open = vec![root];
    while let Some(at) = open.pop() {
        let shown = world
            .get::<InheritedVisibility>(at)
            .is_some_and(|visible| visible.get())
            && world
                .get::<ComputedNode>(at)
                .is_some_and(|node| node.size().min_element() > 0.0)
            && world
                .get::<bevy::text::TextLayoutInfo>(at)
                .is_some_and(|layout| !layout.glyphs.is_empty())
            && world
                .get::<TextColor>(at)
                .is_some_and(|color| color.0.alpha() > 0.0);
        if let (Some(text), true) = (world.get::<Text>(at), shown) {
            texts.push(text.0.clone());
        }
        if let Some(children) = world.get::<Children>(at) {
            open.extend(children.iter());
        }
    }
    texts
}

/// Log every planet in the window by name, refuse one named by its id, and
/// pick the nearest to the player.
fn pick_the_planet(world: &mut World) {
    let player = the_player(world).expect("map names: exactly one player ship");
    let origin = world
        .get::<Position>(player)
        .expect("map names: the player ship has a position")
        .0;
    let mut planets: Vec<(f32, GeneratedBodyId, String)> = world
        .query_filtered::<(&GeneratedBodyId, &Name, &Position), With<PlanetMarker>>()
        .iter(world)
        .map(|(id, name, at)| (at.0.distance(origin), id.clone(), name.to_string()))
        .collect();
    planets.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.2.cmp(&b.2)));
    for (_, id, name) in &planets {
        info!("map names: planet '{id}' is named '{name}'");
        assert!(
            !name.contains(&id.to_string()),
            "map names: planet '{id}' is named by its id: '{name}'"
        );
    }
    let (_, planet, planet_name) = planets
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("map names: seed {WORLD_SEED} streamed no planet"));
    world.insert_resource(Picks {
        planet,
        planet_name,
        asteroid: None,
    });
    world.trigger(DockingReleaseRequest { entity: player });
}

/// Stand the player [`STANDOFF_RADII`] radii off the planet, bow on it, on
/// the side toward the origin.
fn stand_off_the_planet(world: &mut World) {
    let player = the_player(world).expect("map names: exactly one player ship");
    let planet = body(world, &picks(world).planet).expect("map names: the planet stands");
    let centre = world
        .get::<Position>(planet)
        .expect("map names: the planet has a position")
        .0;
    let radius = world
        .get::<PlanetRadius>(planet)
        .expect("map names: the planet has a radius")
        .0;
    let outward = (-centre).normalize_or(Vec3::X);
    let at = centre + outward * radius * STANDOFF_RADII;
    jump_facing(world, player, Meters3::from_engine(at), -outward);
}

/// Pause the clock, lock the planet and pick the nearest named asteroid.
fn lock_the_planet(world: &mut World) {
    stand_off_the_planet(world);
    world.resource_mut::<Time<Virtual>>().pause();
    let player = the_player(world).expect("map names: exactly one player ship");
    let planet = body(world, &picks(world).planet).expect("map names: the planet stands");
    world.entity_mut(player).insert(TravelLock(Some(planet)));
    let origin = world
        .get::<Position>(player)
        .expect("map names: the player ship has a position")
        .0;
    let asteroid = world
        .query_filtered::<(Entity, &Name, &Position), (With<AsteroidMarker>, With<GeneratedBodyId>)>()
        .iter(world)
        .map(|(entity, name, at)| (at.0.distance(origin), entity, name.to_string()))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, entity, name)| (entity, name))
        .expect("map names: the window holds an asteroid");
    info!(
        "map names: locked '{}', nearest asteroid '{}'",
        picks(world).planet_name,
        asteroid.1
    );
    world.resource_mut::<Picks>().asteroid = Some(asteroid);
}

/// The inset caption's first line reads the planet's name.
fn the_inset_names_the_planet() -> Arc<Predicate> {
    when(|world| {
        let name = &picks(world).planet_name;
        world
            .try_query_filtered::<&Text, With<TargetInsetCaptionMarker>>()
            .is_some_and(|mut captions| {
                captions
                    .iter(world)
                    .any(|caption| caption.0.lines().next() == Some(name.as_str()))
            })
    })
}

/// Select `contact` on the Map: activate the blip that draws its code.
fn select(world: &mut World, contact: Entity) {
    let code = world
        .get::<MapContactCode>(contact)
        .unwrap_or_else(|| panic!("map names: contact {contact} has no Map code"))
        .0
        .clone();
    let label = world
        .query::<(Entity, &Text)>()
        .iter(world)
        .filter(|(_, text)| text.0 == code)
        .map(|(entity, _)| entity)
        .collect::<Vec<_>>();
    let blip = label
        .into_iter()
        .find_map(|mut at| loop {
            if world.get::<bevy::ui_widgets::Button>(at).is_some() {
                break Some(at);
            }
            match world.get::<ChildOf>(at) {
                Some(parent) => at = parent.parent(),
                None => break None,
            }
        })
        .unwrap_or_else(|| panic!("map names: no Map blip draws '{code}'"));
    world.trigger(Activate { entity: blip });
}

fn select_the_asteroid(world: &mut World) {
    let (asteroid, _) = picks(world)
        .asteroid
        .clone()
        .expect("map names: the asteroid is picked");
    select(world, asteroid);
}

fn select_the_planet(world: &mut World) {
    let planet = body(world, &picks(world).planet).expect("map names: the planet stands");
    select(world, planet);
}

/// The contact panel draws `name`.
fn the_panel_reads(name: impl Fn(&Picks) -> String + Send + Sync + 'static) -> Arc<Predicate> {
    when(move |world| {
        let name = name(picks(world));
        texts_under(world, MAP_PANEL)
            .iter()
            .any(|text| *text == name)
    })
}

fn panel_diagnosis(world: &World) -> String {
    format!(
        "picks {:?}, panel {:?}",
        world.get_resource::<Picks>(),
        texts_under(world, MAP_PANEL)
    )
}

fn script() -> AutopilotPlugin<GameStates> {
    AutopilotPlugin::<GameStates>::new()
        .step("map names: the window streams in and the start docks")
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
        .step("map names: pick the nearest planet and undock")
        .on_enter(pick_the_planet)
        .until(when(|world| {
            player(world).is_some_and(|player| world.get::<DockedShip>(player).is_none())
        }))
        .deadline(BEAT_SECS)
        .add()
        .step("map names: jump off the planet; the window around it streams in")
        .on_enter(|world: &mut World| {
            world.remove_resource::<StartReady>();
            stand_off_the_planet(world);
        })
        .each(|world: &mut World, _, _| {
            if window_settled(world) {
                world.insert_resource(StartReady);
            } else {
                world.remove_resource::<StartReady>();
            }
        })
        .until(when(|world| world.contains_resource::<StartReady>()))
        .deadline(SETUP_SECS)
        .add()
        .step("map names: lock the planet")
        .on_enter(|world: &mut World| {
            lock_the_planet(world);
            hide_dev_overlays(world);
            hide_status_bar(world);
        })
        .until(the_inset_names_the_planet())
        .diagnose(|world| format!("{:?}", world.get_resource::<Picks>()))
        .deadline(BEAT_SECS)
        .add()
        .step("map names: settle on the planet")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("map names: the inset still")
        .on_enter(|world: &mut World| shoot(world, INSET_SHOT))
        .until(shot_written(INSET_SHOT))
        .deadline(BEAT_SECS)
        .add()
        .step("map names: press the interface key")
        .on_enter(press_action("interface_toggle"))
        .until(frames(1))
        .add()
        .step("map names: let the interface key up")
        .on_enter(release_action("interface_toggle"))
        .until(when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
        }))
        .deadline(BEAT_SECS)
        .add()
        .click_named(
            "map names: open the Map tab",
            MAP_TAB,
            when(|world| {
                world.get_resource::<InterfacePaneType>() == Some(&InterfacePaneType::Map)
            }),
            BEAT_SECS,
        )
        .step("map names: settle the Map")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("map names: select the asteroid")
        .on_enter(select_the_asteroid)
        .until(the_panel_reads(|picks| {
            picks
                .asteroid
                .as_ref()
                .map(|(_, name)| name.clone())
                .unwrap_or_default()
        }))
        .diagnose(panel_diagnosis)
        .deadline(BEAT_SECS)
        .add()
        .step("map names: settle on the asteroid")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("map names: the asteroid still")
        .on_enter(|world: &mut World| shoot(world, MAP_ASTEROID_SHOT))
        .until(shot_written(MAP_ASTEROID_SHOT))
        .deadline(BEAT_SECS)
        .add()
        .step("map names: select the planet")
        .on_enter(select_the_planet)
        .until(the_panel_reads(|picks| picks.planet_name.clone()))
        .diagnose(panel_diagnosis)
        .deadline(BEAT_SECS)
        .add()
        .step("map names: settle on the planet")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("map names: the planet still")
        .on_enter(|world: &mut World| shoot(world, MAP_PLANET_SHOT))
        .until(shot_written(MAP_PLANET_SHOT))
        .deadline(BEAT_SECS)
        .add()
}
