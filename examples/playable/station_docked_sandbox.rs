//! station_docked_sandbox: start docked at a living station, rich, with
//! parts in the hold, and try the station UI by hand.
//!
//! Boots the shipped app and starts New Game on a pinned seed, as the Create
//! World modal does, but without a save folder: nothing this run does is
//! saved. When the window has streamed in, it picks the nearest generated
//! station that is living, not on the Enemy side, offers Refit and sells
//! hulls. The player jumps beside it and is put on a berth at its collar each
//! frame until the dock takes. Then the fixture lands, once:
//!
//! - the player holds [`PLAYER_CREDITS`], enough to buy the station, and
//!   the station holds [`STATION_CREDITS`], enough to pay a trade-in;
//! - the station's storage grows by the hold's mass, so a hull buy can store
//!   the hold;
//! - the hold carries up to [`PARTS_PER_ITEM`] of every part a catalogue
//!   section's recipe asks for, as far as the hold has room;
//! - the TAB interface opens on the Ship tab.
//!
//! From there it is yours: nothing closes the interface and nothing ends the
//! run. Close it with TAB and fly, dock again, or reopen it.
//!
//! What to try:
//!
//! 1. the Ship tab: rebuild and swap sections, place a catalogue part with
//!    the ghost, and strip one;
//! 2. the Station tab: buy a hull, hire an escort, buy the station;
//! 3. switch tabs after a change, and check the schematic, the plating and
//!    the greebles follow it.
//!
//! ```text
//! nix develop --command cargo run --example station_docked_sandbox --features debug
//! ```
//!
//! It runs without `--features debug` too, at the cost of a second build of
//! the engine. Under `NOVA_AUTOPILOT=1` (debug only) a smoke walk waits for
//! the docked Ship tab, takes the kinetic PDC from the station catalogue,
//! points at free sockets until the ghost stands valid, clicks the view, and
//! switches tabs after it: a Map tab click, an Inventory tab click and the
//! next-tab key, which steps on to the Station tab while docked.
//! `NOVA_CAPTURE=1` shoots `sandbox-ship.png`, `sandbox-ghost.png`,
//! `sandbox-placed.png`, `sandbox-map-after.png` and `sandbox-station.png`. Unset, no walk and no deadline run.

use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{nova_interface::pane::InterfacePaneType, prelude::*};
use nova_world::prelude::{desired_sectors, SectorRoot, WorldConfig};

#[derive(Parser)]
#[command(name = "station_docked_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Start docked at a living generated station with credits and parts, on the Ship \
             tab, and try the station UI by hand",
    long_about = None
)]
struct Cli;

/// The world seed: `system_station_buy`'s, whose window around the spawn
/// holds a living Neutral station that refits and sells hulls.
const SANDBOX_SEED: u32 = 10;
/// The start role the player spawns as: the scavenger's salvage hauler has
/// the largest start hold, so it takes the most recipe parts.
const SANDBOX_ROLE: &str = "scavenger";
/// The player's balance once docked: above the station's own price, which
/// counts the station's balance twice.
const PLAYER_CREDITS: u32 = 50_000_000;
/// The station's balance once docked, which pays a hull trade-in.
const STATION_CREDITS: u32 = 1_000_000;
/// The most of each recipe part the hold is given.
const PARTS_PER_ITEM: u32 = 4;
/// How far from the station the player parks before the berth.
const PARK_DISTANCE: Meters = Meters(2_000.0);
/// The gap the berth leaves between the two port faces, engine units: half a
/// cell, inside the 10 m capture distance.
const BERTH_GAP: f32 = 0.5;
/// Where a port's face sits in its own frame, as the docking verb reads it.
const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);

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
    app.init_resource::<Sandbox>();
    app.add_systems(OnEnter(GameStates::MainMenu), start_the_world);
    app.add_systems(
        Update,
        drive_the_sandbox.run_if(in_state(GameStates::Playing)),
    );

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
            // The software rasterizer's frames are seconds long: the clamp
            // keeps the berth from integrating a whole frame of drift.
            app.add_systems(Update, smoke::hold_the_walk_clock);
            app.add_systems(
                Startup,
                nova_protocol::nova_debug::harness::force_capture_resolution,
            );
        }
        app.add_plugins(smoke::script());
    }

    app.run()
}

/// Where the sandbox's setup stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SandboxPhaseType {
    /// The world has not started yet.
    #[default]
    Menu,
    /// New Game is up; the window around the spawn is streaming in.
    Streaming,
    /// The player jumped beside the station; its window is streaming in.
    Jumped,
    /// The player is put on the berth each frame until the dock takes.
    Docking,
    /// Docked, fixture set and the Ship tab open. The player has the run.
    Ready,
}

/// The sandbox's setup state.
#[derive(Resource, Default, Debug)]
struct Sandbox {
    phase: SandboxPhaseType,
    /// The chosen station's id, the cell it stands in and where it stood.
    station: Option<(GeneratedBodyId, SectorCoord, Meters3)>,
}

/// Start New Game on [`SANDBOX_SEED`] the first time the main menu comes up,
/// with no save folder. A later Back to Main Menu stays on the menu.
fn start_the_world(
    mut commands: Commands,
    mut sandbox: ResMut<Sandbox>,
    mut mode: ResMut<GameMode>,
    mut pick: ResMut<NewGameScenario>,
    mut state: ResMut<NextState<GameStates>>,
) {
    if sandbox.phase != SandboxPhaseType::Menu {
        return;
    }
    commands.insert_resource(OpenWorldSession {
        seed: SANDBOX_SEED,
        role: SANDBOX_ROLE.into(),
    });
    pick.0 = None;
    *mode = GameMode::NewGame;
    state.set(GameStates::Playing);
    sandbox.phase = SandboxPhaseType::Streaming;
    info!("station sandbox: New Game on seed {SANDBOX_SEED}, no save folder");
}

/// Advance the setup one phase at a time, each on a world value.
fn drive_the_sandbox(world: &mut World) {
    let phase = world.resource::<Sandbox>().phase;
    match phase {
        SandboxPhaseType::Menu | SandboxPhaseType::Ready => {}
        SandboxPhaseType::Streaming => {
            if window_is(world, Some(SectorCoord::ORIGIN)) {
                choose_the_station(world);
                park_beside(world);
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Jumped;
            }
        }
        SandboxPhaseType::Jumped => {
            let cell = world
                .resource::<Sandbox>()
                .station
                .as_ref()
                .map(|(_, cell, _)| *cell);
            let stands = station_root(world).is_some_and(|station| {
                world.get::<Position>(station).is_some()
                    && world.get::<HullOffers>(station).is_some()
            });
            if window_is(world, cell) && stands {
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Docking;
            }
        }
        SandboxPhaseType::Docking => {
            let Some(player) = the_player(world) else {
                return;
            };
            if world.get::<DockedShip>(player).is_some() {
                set_the_fixture(world, player);
                open_the_ship_tab(world);
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Ready;
            } else {
                berth_and_dock(world, player);
            }
        }
    }
}

/// The one player ship, if exactly one stands.
fn the_player(world: &mut World) -> Option<Entity> {
    let mut players = world.query_filtered::<Entity, With<PlayerSpaceshipMarker>>();
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

/// The entity wearing the chosen station's id, if one does.
fn station_root(world: &mut World) -> Option<Entity> {
    let (id, ..) = world.resource::<Sandbox>().station.clone()?;
    world
        .query::<(Entity, &GeneratedBodyId)>()
        .iter(world)
        .find(|(_, worn)| **worn == id)
        .map(|(entity, _)| entity)
}

/// Whether the world is armed around one player ship and the live set is
/// exactly the window around `centre`.
fn window_is(world: &mut World, centre: Option<SectorCoord>) -> bool {
    let Some(centre) = centre else {
        return false;
    };
    let Some(radius) = world
        .get_resource::<WorldConfig<NovaLayeredWorld>>()
        .map(|config| config.active_radius)
    else {
        return false;
    };
    if the_player(world).is_none() {
        return false;
    }
    let live: std::collections::BTreeSet<SectorCoord> = world
        .query::<&SectorRoot>()
        .iter(world)
        .map(|root| root.0)
        .collect();
    live == desired_sectors(centre, radius)
}

/// Pick the nearest station that is living, not on the Enemy side, offers
/// Refit and sells hulls, outside the spawn's cell.
fn choose_the_station(world: &mut World) {
    let edge = world
        .resource::<WorldConfig<NovaLayeredWorld>>()
        .sector_edge;
    let mut stations: Vec<(f32, GeneratedBodyId, Vec3)> = world
        .query_filtered::<(
            &GeneratedBodyId,
            &Position,
            Option<&StationServices>,
            Has<DerelictShipMarker>,
            Option<&Allegiance>,
            Option<&HullOffers>,
        ), With<StationMarker>>()
        .iter(world)
        .filter(|(_, position, services, derelict, allegiance, offers)| {
            station_offers(*services, *derelict, StationServiceType::Refit)
                && offers.is_some_and(|offers| !offers.0.is_empty())
                && *allegiance != Some(&Allegiance::Enemy)
                && SectorCoord::containing(Meters3::from_engine(position.0), edge)
                    != SectorCoord::ORIGIN
        })
        .map(|(id, position, ..)| {
            (
                Meters3::from_engine(position.0)
                    .distance(Meters3::ZERO)
                    .get(),
                id.clone(),
                position.0,
            )
        })
        .collect();
    stations.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let Some((distance, id, at)) = stations.into_iter().next() else {
        panic!(
            "station sandbox: the window on seed {SANDBOX_SEED} holds no living, non-Enemy \
             station with Refit and hulls outside the spawn's cell"
        );
    };
    let position = Meters3::from_engine(at);
    let cell = SectorCoord::containing(position, edge);
    info!("station sandbox: chose '{id}' in cell {cell}, {distance:.0} m out");
    world.resource_mut::<Sandbox>().station = Some((id, cell, position));
}

/// Jump the player [`PARK_DISTANCE`] from the station toward the spawn, at
/// rest, so the station's window streams in around it.
fn park_beside(world: &mut World) {
    let Some((_, _, position)) = world.resource::<Sandbox>().station.clone() else {
        return;
    };
    let Some(player) = the_player(world) else {
        return;
    };
    let toward_spawn = (-position.get()).normalize_or_zero();
    let beside = Meters3(position.get() + toward_spawn * PARK_DISTANCE.get());
    world.entity_mut(player).insert((
        Position(beside.to_engine()),
        LinearVelocity::ZERO,
        AngularVelocity::ZERO,
    ));
}

/// `section`'s position and rotation in `root`'s frame, by its mount chain.
fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
    let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
    while at != root {
        let transform = world
            .get::<Transform>(at)
            .expect("station sandbox: a mounted section has a transform");
        position = transform.translation + transform.rotation * position;
        rotation = transform.rotation * rotation;
        at = world
            .get::<ChildOf>(at)
            .expect("station sandbox: a section's chain reaches its root")
            .parent();
    }
    (position, rotation)
}

/// The lowest free, live docking port mounted on `root`.
fn free_port(world: &mut World, root: Entity) -> Option<Entity> {
    let ports: Vec<Entity> = world
        .query_filtered::<Entity, (
            With<DockingSectionMarker>,
            Without<DockedPort>,
            Without<SectionInactiveMarker>,
        )>()
        .iter(world)
        .collect();
    ports
        .into_iter()
        .filter(|port| {
            let mut at = *port;
            while let Some(parent) = world.get::<ChildOf>(at).map(ChildOf::parent) {
                if parent == root {
                    return true;
                }
                at = parent;
            }
            false
        })
        .min()
}

/// Put the player on the berth at the station's collar, moving and turning
/// with the station, then ask to dock.
fn berth_and_dock(world: &mut World, player: Entity) {
    let Some(station) = station_root(world) else {
        return;
    };
    let (Some(station_port), Some(player_port)) =
        (free_port(world, station), free_port(world, player))
    else {
        panic!("station sandbox: the station or the player has no free docking port");
    };
    let position = world.get::<Position>(station).expect("a station stands").0;
    let rotation = world.get::<Rotation>(station).expect("a station stands").0;
    let velocity = world
        .get::<LinearVelocity>(station)
        .map_or(Vec3::ZERO, |v| v.0);
    let spin = world
        .get::<AngularVelocity>(station)
        .map_or(Vec3::ZERO, |w| w.0);
    let (port_at, port_rot) = local_pose(world, station_port, station);
    let station_face_rot = rotation * port_rot;
    let station_face = position + rotation * port_at + station_face_rot * PORT_FACE;
    let station_axis = station_face_rot * Vec3::NEG_Z;
    let (own_at, own_rot) = local_pose(world, player_port, player);
    let turn = Quat::from_rotation_arc((own_rot * Vec3::NEG_Z).normalize(), -station_axis);
    let face_offset = turn * own_at + turn * own_rot * PORT_FACE;
    let at = station_face + station_axis * BERTH_GAP - face_offset;
    world.entity_mut(player).insert((
        Position(at),
        Rotation(turn),
        LinearVelocity(velocity),
        AngularVelocity(spin),
    ));
    world.trigger(DockingConnectionRequest {
        entity: player,
        target: station,
    });
}

/// Give the player [`PLAYER_CREDITS`], the station [`STATION_CREDITS`], and
/// the hold up to [`PARTS_PER_ITEM`] of every part a catalogue section's
/// recipe asks for, in id order, as far as it has room.
fn set_the_fixture(world: &mut World, player: Entity) {
    let items = world.resource::<GameItems>().clone();
    let parts: std::collections::BTreeSet<ItemDesignId> = world
        .resource::<GameSections>()
        .iter()
        .flat_map(|section| section.base.build.parts.keys().cloned())
        .collect();
    let mut given: Vec<(ItemDesignId, u32)> = Vec::new();
    if let Some(mut hold) = world.get_mut::<ShipInventory>(player) {
        for part in parts {
            let unit_g = items.design(&part).stack_mass_g(1).max(1);
            let room = u64::from(hold.free_g(&items)) / unit_g;
            let count = PARTS_PER_ITEM.min(u32::try_from(room).unwrap_or(u32::MAX));
            if count > 0 {
                hold.add(&items, &part, count);
                given.push((part, count));
            }
        }
    }
    if let Some(mut credits) = world.get_mut::<ShipCredits>(player) {
        credits.0 = PLAYER_CREDITS;
    }
    let hold_g = world
        .get::<ShipInventory>(player)
        .map_or(0, |hold| hold.used_g(&items));
    if let Some(station) = station_root(world) {
        if let Some(mut credits) = world.get_mut::<ShipCredits>(station) {
            credits.0 = STATION_CREDITS;
        }
        // A hull buy moves the hold into the station's storage, so the
        // storage grows by the hold, keeping what it stores.
        if let Some(mut storage) = world.get_mut::<StationStorage>(station) {
            let mut grown = StationStorage::new(storage.cap_g().saturating_add(hold_g));
            for (item, count) in storage.stacks() {
                grown.add(&items, item, count);
            }
            *storage = grown;
        }
    }
    info!(
        "station sandbox: docked; player {PLAYER_CREDITS} cr, station {STATION_CREDITS} cr, \
         parts in the hold: {given:?}"
    );
}

/// Open the TAB interface on the Ship tab, and say how to get back to it.
fn open_the_ship_tab(world: &mut World) {
    *world.resource_mut::<InterfacePaneType>() = InterfacePaneType::Ship;
    world
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Interface);
    info!("station sandbox: the Ship tab is open; TAB closes and reopens the interface");
}

#[cfg(feature = "debug")]
mod smoke {
    use std::sync::Arc;

    use nova_protocol::{
        nova_debug::harness::{press_action, release_action, shoot, AutopilotPlugin, Predicate},
        nova_interface::ship::{ship_ghost_aim, ship_view_window_px, ShipRuntime},
    };

    use super::*;

    /// Seconds the world gets to start, stream and dock on a software GPU.
    const SETUP_SECS: f32 = 600.0;
    /// Seconds a shot gets to land.
    const SHOT_SECS: f32 = 120.0;
    /// Frames the Ship pane gets to build its panel and lists.
    const SETTLE_FRAMES: u32 = 10;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;
    /// The PNG of the docked Ship tab.
    const SHIP_SHOT: &str = "sandbox-ship.png";
    /// The PNG of the valid ghost before the click.
    const GHOST_SHOT: &str = "sandbox-ghost.png";
    /// Frames the pointer rests on one free socket before the walk tries the
    /// next one.
    const AIM_FRAMES: u32 = 6;
    /// How far the aim sits from a socket toward its section's centre, as a
    /// share of the way.
    const AIM_INSET: f32 = 0.15;
    /// The PNG of the Ship tab after the add.
    const PLACED_SHOT: &str = "sandbox-placed.png";
    /// The PNG of the Map tab a click opens after the add.
    const MAP_AFTER_SHOT: &str = "sandbox-map-after.png";
    /// The Station tab, reached with the next-tab key from Inventory.
    const STATION_SHOT: &str = "sandbox-station.png";
    /// Seconds a click or a key gets to land.
    const BEAT_SECS: f32 = 60.0;
    /// The Ship pane's side panel, which scrolls under the wheel.
    const SHIP_PANEL: &str = "ShipPanel";

    /// The Place button the walk clicks, the player's section count before,
    /// the ship-local points it aims the ghost through, and the one it is on
    /// (`usize::MAX` before the first).
    #[derive(Resource, Clone, Debug)]
    struct SmokeAdd {
        button: String,
        sections: usize,
        aims: Vec<Vec3>,
        aiming: usize,
    }

    /// The kinetic PDC's Place button, which every refit catalogue the
    /// fixture's hold stocks lists.
    fn place_button() -> String {
        format!("ShipPlace-{PDC_KINETIC_TURRET_SECTION_ID}")
    }

    /// A point beside each free socket of each live design section, in
    /// design order, where the pointer aims the ghost at that socket.
    fn free_socket_aims(world: &mut World) -> Vec<Vec3> {
        let Some((design, children)) = world
            .query_filtered::<(&ShipDesignSections, &Children), With<PlayerSpaceshipMarker>>()
            .iter(world)
            .next()
        else {
            return Vec::new();
        };
        let live: Vec<&str> = children
            .iter()
            .filter_map(|child| world.get::<EntityId>(child))
            .map(|id| id.0.as_str())
            .collect();
        let placed: Vec<RefitPlacedSection<'_>> = design
            .0
            .iter()
            .map(|section| RefitPlacedSection {
                position: section.position,
                rotation: section.rotation,
                config: &section.config,
            })
            .collect();
        design
            .0
            .iter()
            .enumerate()
            .filter(|(_, section)| live.contains(&section.id.as_str()))
            .flat_map(|(index, section)| {
                free_link_points(&placed, index)
                    .into_iter()
                    .map(move |point| {
                        let (socket, _) = link_point_pose(
                            section.position,
                            section.rotation,
                            &section.config.base.link_points[point],
                        );
                        socket.lerp(section.position, AIM_INSET)
                    })
            })
            .collect()
    }

    /// Point at the free sockets in turn, `AIM_FRAMES` frames each, until the
    /// ghost stands valid at one.
    fn aim_the_ghost(world: &mut World, _elapsed: f32, frame: u32) {
        if ghost_is_valid(world) || frame % AIM_FRAMES != 0 {
            return;
        }
        let Some(add) = world.get_resource::<SmokeAdd>().cloned() else {
            return;
        };
        let aiming = add.aiming.wrapping_add(1);
        let Some(&aim) = add.aims.get(aiming) else {
            return;
        };
        let Some(pixel) = ship_view_window_px(world, aim) else {
            return;
        };
        move_cursor(pixel)(world);
        world.resource_mut::<SmokeAdd>().aiming = aiming;
    }

    fn ghost_is_valid(world: &World) -> bool {
        ship_ghost_aim(world).is_some_and(|(_, _, valid)| valid)
    }

    /// How many live sections the player ship has.
    fn player_sections(world: &World) -> usize {
        let Some(mut players) =
            world.try_query_filtered::<&Children, With<PlayerSpaceshipMarker>>()
        else {
            return 0;
        };
        players.iter(world).next().map_or(0, |children| {
            children
                .iter()
                .filter(|child| world.get::<SectionMarker>(*child).is_some())
                .count()
        })
    }

    fn smoke_button(world: &World) -> Option<String> {
        world
            .get_resource::<SmokeAdd>()
            .map(|add| add.button.clone())
    }

    fn button_in_view(world: &World) -> bool {
        let Some(button) = smoke_button(world) else {
            return false;
        };
        match (
            ui_node_rect(world, &button),
            ui_node_rect(world, SHIP_PANEL),
        ) {
            (Some(button), Some(panel)) => {
                button.min.y >= panel.min.y && button.max.y <= panel.max.y
            }
            _ => false,
        }
    }

    fn the_interface_shows(pane: InterfacePaneType) -> Arc<Predicate> {
        Arc::new(move |world: &World| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
                && world.get_resource::<InterfacePaneType>() == Some(&pane)
        })
    }

    /// Log the pane tabs and what gates a tab click, so a tab that does not
    /// answer says why.
    fn report_the_tabs(world: &mut World) {
        let pane = world.get_resource::<InterfacePaneType>().copied();
        let rows: Vec<String> = world
            .query::<(
                &Name,
                Has<nova_ui::widget::Selected>,
                Has<bevy::ui::InteractionDisabled>,
            )>()
            .iter(world)
            .filter(|(name, ..)| name.as_str().starts_with("InterfaceTab"))
            .map(|(name, selected, disabled)| {
                format!("{name} selected={selected} disabled={disabled}")
            })
            .collect();
        info!("station sandbox: tabs, pane {pane:?}: {rows:?}");
    }

    /// Hold the virtual clock's clamp every frame: a scenario load hands
    /// `Time<Virtual>` back at its default.
    pub(super) fn hold_the_walk_clock(mut time: ResMut<Time<Virtual>>) {
        let clamp = std::time::Duration::from_secs_f32(WALK_CLOCK_SECS);
        if time.max_delta() != clamp {
            time.set_max_delta(clamp);
        }
    }

    fn ready_on_the_ship_tab() -> Arc<Predicate> {
        Arc::new(|world: &World| {
            world
                .get_resource::<Sandbox>()
                .is_some_and(|sandbox| sandbox.phase == SandboxPhaseType::Ready)
                && world
                    .get_resource::<State<PauseStates>>()
                    .is_some_and(|pause| *pause.get() == PauseStates::Interface)
                && world.get_resource::<InterfacePaneType>() == Some(&InterfacePaneType::Ship)
        })
    }

    /// The smoke walk: the setup lands docked on the Ship tab, and a frame of
    /// it is shot.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("station sandbox: docked on the Ship tab")
            .until(ready_on_the_ship_tab())
            .diagnose(|world| format!("{:?}", world.get_resource::<Sandbox>()))
            .deadline(SETUP_SECS)
            .add()
            .step("station sandbox: settle the schematic")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: shoot the Ship tab")
            .on_enter(|world: &mut World| shoot(world, SHIP_SHOT))
            .until(shot_written(SHIP_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("station sandbox: find the Place button")
            .each(|world, _, _| {
                if world.get_resource::<SmokeAdd>().is_none()
                    && ui_node_rect(world, &place_button()).is_some()
                {
                    let sections = player_sections(world);
                    let aims = free_socket_aims(world);
                    world.insert_resource(SmokeAdd {
                        button: place_button(),
                        sections,
                        aims,
                        aiming: usize::MAX,
                    });
                }
            })
            .until(Arc::new(|world: &World| smoke_button(world).is_some()))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: scroll the Place button into view")
            .each(|world, _, frame| {
                keep_hovering_named(SHIP_PANEL)(world);
                if frame % 2 == 1 && !button_in_view(world) {
                    scroll_lines(-1.0)(world);
                }
            })
            .until(Arc::new(button_in_view))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: aim at the Place button")
            .each(|world, _, _| {
                if let Some(button) = smoke_button(world) {
                    keep_hovering_named(button)(world);
                }
            })
            .until(Arc::new(|world: &World| {
                smoke_button(world).is_some_and(|button| pointer_over_node(button)(world))
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: press the Place button")
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: release the Place button")
            .on_enter(release_mouse(MouseButton::Left))
            .until(Arc::new(|world: &World| {
                world.resource::<ShipRuntime>().held() == Some(PDC_KINETIC_TURRET_SECTION_ID)
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: aim the ghost at a free socket")
            .each(aim_the_ghost)
            .until(Arc::new(ghost_is_valid))
            .diagnose(|world| {
                format!(
                    "{:?}, aim {:?}",
                    world.get_resource::<SmokeAdd>(),
                    ship_ghost_aim(world)
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: settle the ghost")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: shoot the ghost")
            .on_enter(|world: &mut World| shoot(world, GHOST_SHOT))
            .until(shot_written(GHOST_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("station sandbox: press on the ghost")
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: release on the ghost")
            .on_enter(release_mouse(MouseButton::Left))
            .until(Arc::new(|world: &World| {
                world
                    .get_resource::<SmokeAdd>()
                    .is_some_and(|add| player_sections(world) > add.sections)
            }))
            .diagnose(|world| format!("{:?}", world.get_resource::<SmokeAdd>()))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: settle the placed section")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: report the tabs after the add")
            .on_enter(report_the_tabs)
            .add()
            .step("station sandbox: shoot the placed section")
            .on_enter(|world: &mut World| shoot(world, PLACED_SHOT))
            .until(shot_written(PLACED_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .click_named(
                "station sandbox: open the Map tab after the add",
                "InterfaceTabMap",
                the_interface_shows(InterfacePaneType::Map),
                BEAT_SECS,
            )
            .step("station sandbox: shoot the Map tab after the add")
            .on_enter(|world: &mut World| shoot(world, MAP_AFTER_SHOT))
            .until(shot_written(MAP_AFTER_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .click_named(
                "station sandbox: open the Inventory tab",
                "InterfaceTabInventory",
                the_interface_shows(InterfacePaneType::Inventory),
                BEAT_SECS,
            )
            .step("station sandbox: press the next-tab key")
            .on_enter(press_action("interface_next_tab"))
            .until(frames(1))
            .add()
            .step("station sandbox: let the next-tab key up")
            .on_enter(release_action("interface_next_tab"))
            .until(the_interface_shows(InterfacePaneType::Station))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: settle the Station tab")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: shoot the Station tab")
            .on_enter(|world: &mut World| shoot(world, STATION_SHOT))
            .until(shot_written(STATION_SHOT))
            .deadline(SHOT_SECS)
            .add()
    }
}
