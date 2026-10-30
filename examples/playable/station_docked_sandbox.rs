//! station_docked_sandbox: start docked at a living station, rich, with
//! parts in the hold, and try the station UI by hand.
//!
//! Boots the shipped app and starts New Game on a pinned seed, as the Create
//! World modal does, but without a save folder: nothing this run does is
//! saved. When the window has streamed in, the player lets go of the
//! starting station and jumps to the origin. When the origin's window has
//! streamed in, it picks the nearest generated station that is living, not
//! on the Enemy side, offers Refit and sells hulls. The player jumps beside
//! it and is put on a berth at its collar each frame until the dock takes.
//! Then the fixture lands, once:
//!
//! - the player holds [`PLAYER_CREDITS`], enough to buy the station, and
//!   the station holds [`STATION_CREDITS`], a fixed balance its price counts;
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
//! 1. the Ship tab: rebuild sections, open the Catalogue to swap the
//!    selected section or place a part with the ghost (R rolls it, F cycles
//!    its mating socket; a click buys the parts the hold lacks at the
//!    station's ask), pick a part off the ship with Q, drag the view with the
//!    right button to the docked station (dim outlines, on the ship's -X
//!    side) and aim a Cargo Intake end-on (F) at a plate facing it to see it
//!    refused, rebind a gun or a thruster with B (Backspace sends a
//!    thruster back to the Main Drive), and delete a selected section with Delete
//!    (the Salvage block under it lists what goes back in the hold, or why the
//!    disabled button is refused);
//! 2. the Station tab: buy a hull, hire an escort, buy the station;
//! 3. switch tabs after a change, and check the schematic, the plating and
//!    the greebles follow it.
//!
//! ```text
//! nix develop --command cargo run --example station_docked_sandbox --features debug
//! ```
//!
//! The Ship pane's footer names the keys that apply now. The diagnostics
//! layer (`nova_debug::diagnostics`, F3, on at start) in the Ship view draws the free sockets near the target (green rings,
//! a grey cross where the lane is closed), the target socket, the part in
//! hand's bounds (green valid, red refused) with its roll as a cyan arrow,
//! and the collision volume of the station and the ships at its docks
//! (violet). Its panel names the part in hand and the verdict; Shift+F3 adds
//! the target socket, the parts the click buys, the selected section's
//! binding, what a delete gives back or why it is refused, and the socket
//! counts.
//!
//! It needs `--features debug`, which owns the diagnostics layer. Under
//! `NOVA_AUTOPILOT=1` a smoke walk waits for
//! the docked Ship tab, opens the Catalogue on the kinetic PDC's category and
//! places it in hand,
//! points at free sockets until the ghost stands valid, rolls the part and
//! steps its socket, steps the socket back and rolls until it fits, clicks the
//! view and checks the view eases onto the placed section and that the panel
//! shows the part in hand under Placing, presses Esc and checks the hand is
//! empty, the panel back on the selection and the interface still open, rebinds the new
//! gun to `T` and checks the selection did not step, opens the Catalogue for
//! the cards, counts the muted tiles, clicks Gallery Stock Available and
//! checks it keeps only the placeable listed parts of the category with the
//! narrowed count on the Gallery Page line, shoots it, clicks Gallery Stock
//! All and checks the full count is back, at 640x600 in the first theme
//! shows the muted card's tile and checks its `Tooltip` is its note, hovers
//! it until the note shows and shoots it, shows the twin's tile and checks
//! it carries no `Tooltip`, rests the pointer, then reads the focus card of
//! the twin kinetic PDC, of the twin buying a barrel, of the twin short of a
//! panel and of a part the station does not build in the Phosphor and
//! Hardware themes at 640x600 and 1024x768 (Place, Swap and Back and the
//! Stock buttons inside the card or the header, a muted part's Place off
//! with its note as the tooltip, every recipe row whole with its covered
//! count and, when it bought anything, the bought total, the bought row's
//! split on hover), presses Enter and clicks the muted Place and
//! checks the hand stays empty, then puts the capture shape and theme back,
//! checks the Salvage block lists the gun's yield, deletes the gun with
//! Delete and checks the note names the parts that went to the hold, selects
//! the port the dock holds by and checks its Delete is muted with the reason
//! as its tooltip and that the Delete key leaves it on with the reason on the
//! note line, cycles to a section whose delete would split the ship and
//! checks the verdict names the sections that would lose the dock, then in
//! the Phosphor and Hardware themes at 640x600 and 1024x768 hovers its muted
//! Delete until the tooltip shows and shoots it, cycles on to a section that
//! may come off and shoots its enabled Delete in each theme, puts the shape
//! and theme back, picks a hull part off the ship with Q, puts it away with Q over empty space and
//! checks the hand is empty, takes the Cargo Intake from the Catalogue, drags the
//! view to the station and points the intake at the sockets nearest it, through every
//! mating socket, until the station refuses it (or warns that none reaches),
//! and switches tabs after it: a Map tab click, an Inventory tab click and the
//! next-tab key twice, which steps on to the Jobs tab and then the Station
//! tab while docked. Last it
//! selects the first thruster and checks its Control line names the Main
//! Drive keys, rebinds it to `J` and checks the binding, closes the
//! interface, lets go of the dock with `dock`, holds `J` and checks the
//! thruster's throttle reaches 1, then reopens the Ship tab, rebinds and
//! presses Backspace and checks the Main Drive line is back. Before the
//! Ship tab walk it closes the interface, waits for both sleeves of every
//! berth to reach out, checks the player and every hull for sale stand one
//! cell off their station ports with no two envelopes overlapping, and frames
//! a close-up of the player's berth and of the first hull for sale's. A gap
//! that a station port recessed in a plated surface hides from every eye is
//! shot from outside, with a warning that names the blocking root.
//! `NOVA_CAPTURE=1` shoots `sandbox-berth.png`, `sandbox-sale-berth.png`,
//! `sandbox-ship.png`,
//! `sandbox-catalogue-all.png`, `sandbox-catalogue.png`, `sandbox-ghost.png`, `sandbox-mount.png`, `sandbox-placed.png`, `sandbox-esc-put-away.png`,
//! `sandbox-rebind-armed.png`, `sandbox-rebind.png`,
//! `sandbox-catalogue-available.png`, `sandbox-tile-muted-tooltip-640.png`,
//! `sandbox-card-<twin|muted|muted-tooltip>-<phosphor|hardware>-<640|1024>.png`,
//! `sandbox-delete-yield.png`,
//! `sandbox-delete.png`,
//! `sandbox-delete-refused.png`,
//! `sandbox-delete-split-<phosphor|hardware>-<640|1024>.png`,
//! `sandbox-delete-enabled-<phosphor|hardware>.png`,
//! `sandbox-pick.png`, `sandbox-catalogue-intake.png`,
//! `sandbox-station-refusal.png`,
//! `sandbox-map-after.png`, `sandbox-station.png`,
//! `sandbox-thruster-default.png`, `sandbox-thruster-own-key.png` and
//! `sandbox-thruster-cleared.png`.
//! Unset, no walk and no deadline run.

use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{nova_debug::diagnostics, nova_interface::pane::InterfacePaneType, prelude::*};
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

/// The world seed: `system_station_buy`'s, whose window around the origin
/// holds a living Neutral station that refits, sells hulls and builds every
/// part in [`WALK_PARTS`].
const SANDBOX_SEED: u32 = 10;
/// The start role the player spawns as: the scavenger's salvage hauler has
/// the largest start hold, so it takes the most recipe parts.
const SANDBOX_ROLE: &str = "scavenger";
/// The player's balance once docked: above the station's own price, which
/// counts the station's balance twice.
const PLAYER_CREDITS: u32 = 50_000_000;
/// The station's balance once docked, fixed so its price is too.
const STATION_CREDITS: u32 = 1_000_000;
/// The most of each recipe part the hold is given.
const PARTS_PER_ITEM: u32 = 4;
/// The three-cell intake the walk aims into the docked station. `nova_ship`
/// exports no id for it: fixture ids stay local.
const CARGO_INTAKE_SECTION_ID: &str = "cargo_intake_section";
/// The twin kinetic PDC, whose focus card the walk reads: a four-item recipe
/// and a swap for the placed PDC. Local for the same reason.
const PDC_TWIN_KINETIC_TURRET_SECTION_ID: &str = "pdc_twin_kinetic_turret_section";
/// The parts the walk takes from the Catalogue. The chosen station must
/// build every one: a muted tile cannot be placed.
const WALK_PARTS: [&str; 2] = [PDC_KINETIC_TURRET_SECTION_ID, CARGO_INTAKE_SECTION_ID];
/// How far from the station the player parks before the berth.
const PARK_DISTANCE: Meters = Meters(2_000.0);
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
    app.add_plugins((
        diagnostics::DiagnosticsHintPlugin::new(INTRO),
        ship_overlay::ShipOverlayPlugin,
    ));
    app.add_systems(OnEnter(GameStates::MainMenu), start_the_world);
    app.add_systems(
        Update,
        drive_the_sandbox.run_if(in_state(GameStates::Playing)),
    );

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

    app.run()
}

/// Where the sandbox's setup stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SandboxPhaseType {
    /// The world has not started yet.
    #[default]
    Menu,
    /// New Game is up; the window around the start is streaming in and the
    /// start is docking the player at its starting station.
    Streaming,
    /// The player was asked to let go of the starting station.
    Undocking,
    /// The player jumped to the origin, the spawn before the docked start;
    /// the origin's window is streaming in.
    Returning,
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
    loaded: Res<LoadedSectionPacks>,
    styles: Res<GameStyles>,
    items: Res<GameItems>,
    mut mode: ResMut<GameMode>,
    mut pick: ResMut<NewGameScenario>,
    mut state: ResMut<NextState<GameStates>>,
) {
    if sandbox.phase != SandboxPhaseType::Menu {
        return;
    }
    let session = start_session(SANDBOX_SEED, SANDBOX_ROLE.into(), &loaded, &styles, &items)
        .unwrap_or_else(|refusal| {
            panic!("station sandbox: seed {SANDBOX_SEED} refused: {refusal}")
        });
    info!("station sandbox: the start docks at '{}'", session.start.id);
    commands.insert_resource(session);
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
            let Some(player) = the_player(world) else {
                return;
            };
            let home = player_cell(world);
            if window_is(world, home) && world.get::<DockedShip>(player).is_some() {
                // The start docked the player at its starting station; the
                // sandbox stages a station of its own.
                world.trigger(DockingReleaseRequest { entity: player });
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Undocking;
            }
        }
        SandboxPhaseType::Undocking => {
            let Some(player) = the_player(world) else {
                return;
            };
            if world.get::<DockedShip>(player).is_none() {
                // Stations are a civilization apart: the station the sandbox
                // wants never streams in round the start.
                world.entity_mut(player).insert((
                    Position(Vec3::ZERO),
                    LinearVelocity::ZERO,
                    AngularVelocity::ZERO,
                ));
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Returning;
            }
        }
        SandboxPhaseType::Returning => {
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
                    && world.get::<PendingParkedHulls>(station).is_none()
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

/// The player's cell, once the player stands in an armed world.
fn player_cell(world: &mut World) -> Option<SectorCoord> {
    let edge = world
        .get_resource::<WorldConfig<NovaLayeredWorld>>()?
        .sector_edge;
    let player = the_player(world)?;
    let at = world.get::<Position>(player)?.0;
    Some(SectorCoord::containing(Meters3::from_engine(at), edge))
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
/// Refit, sells hulls and builds every part in [`WALK_PARTS`], outside the
/// origin's cell.
fn choose_the_station(world: &mut World) {
    let edge = world
        .resource::<WorldConfig<NovaLayeredWorld>>()
        .sector_edge;
    let home = player_cell(world).expect("station sandbox: the player stands in the armed world");
    let start = world
        .get_resource::<OpenWorldSession>()
        .map(|session| session.start.id.clone());
    let mut rejected: Vec<(GeneratedBodyId, Vec<String>)> = Vec::new();
    let mut stations: Vec<(f32, GeneratedBodyId, Vec3, Vec<String>)> = world
        .query_filtered::<(
            &GeneratedBodyId,
            &Position,
            Option<&StationServices>,
            Has<DerelictShipMarker>,
            Option<&Allegiance>,
            Option<&ParkedHulls>,
            Option<&PendingParkedHulls>,
            Option<&StationBuilds>,
        ), With<StationMarker>>()
        .iter(world)
        .filter(
            |(id, position, services, derelict, allegiance, parked, pending, builds)| {
                let passes = station_offers(*services, *derelict, StationServiceType::Refit)
                    && (parked.is_some() || pending.is_some())
                    && *allegiance != Some(&Allegiance::Enemy)
                    && SectorCoord::containing(Meters3::from_engine(position.0), edge) != home
                    && start.as_ref() != Some(*id);
                if !passes {
                    return false;
                }
                // A living station without the component is invalid content:
                // the site that reads it panics, as its contract says.
                let builds = builds
                    .unwrap_or_else(|| {
                        panic!("station sandbox: station '{id}' has no StationBuilds")
                    })
                    .0
                    .clone();
                let builds_every_part = WALK_PARTS
                    .iter()
                    .all(|part| builds.iter().any(|known| known == part));
                if !builds_every_part {
                    rejected.push(((*id).clone(), builds));
                }
                builds_every_part
            },
        )
        .map(|(id, position, _, _, _, _, _, builds)| {
            (
                Meters3::from_engine(position.0)
                    .distance(Meters3::ZERO)
                    .get(),
                id.clone(),
                position.0,
                builds
                    .expect("station sandbox: the filter read the station's StationBuilds")
                    .0
                    .clone(),
            )
        })
        .collect();
    stations.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let Some((distance, id, at, builds)) = stations.into_iter().next() else {
        panic!(
            "station sandbox: the origin's window on seed {SANDBOX_SEED} holds no living, \
             non-Enemy station with Refit and hulls outside the origin's cell that builds \
             {WALK_PARTS:?}; rejected: {rejected:?}"
        );
    };
    let position = Meters3::from_engine(at);
    let cell = SectorCoord::containing(position, edge);
    info!(
        "station sandbox: chose '{id}' in cell {cell}, {distance:.0} m out; it builds {builds:?}"
    );
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
    let (own_at, own_rot) = local_pose(world, player_port, player);
    let (at, turn) = parked_hull_pose((position, rotation), (port_at, port_rot), (own_at, own_rot));
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

/// What the sandbox is for, for the diagnostics intro.
const INTRO: &str = "Docked at a Refit station: edit the ship on the Ship tab.";

/// The Ship view's diagnostics on the shared layer: the free sockets near the
/// target, the target socket and its roll, the ghost's bounds and the
/// station's collision volume as gizmos; the part in hand and the verdict as
/// panel lines, the rest as detail lines; and the keys that apply now as
/// hints.
mod ship_overlay {
    use bevy::{camera::visibility::RenderLayers, ecs::system::SystemState};
    use nova_protocol::nova_interface::ship::{
        delete_yield_lines, ship_hovered_section, ship_placement, socket_lane_open, ShipPlacement,
        ShipRuntime, ShipSectionView, ShipSections,
    };

    use super::{diagnostics, diagnostics::palette, *};

    /// How far from the target socket a free socket is still drawn, in
    /// cells: the sockets of the target block and its neighbours, where the
    /// part in hand moves next.
    const NEAR_TARGET_CELLS: f32 = 1.0;

    /// What the gizmos draw this frame, read from the world.
    #[derive(Resource, Default)]
    struct ShipOverlay {
        /// The free sockets of live sections near the target socket, in
        /// ship-local space: where each sits, which way it faces, and whether
        /// its lane is open.
        free_points: Vec<(Vec3, Vec3, bool)>,
        /// The ghost's placement and its part's collider half extents.
        placement: Option<(ShipPlacement, Vec3)>,
        /// The docked station's collision volume as the add fit measures it:
        /// each box's centre and its ship-local half extents.
        station: Vec<(Vec3, Vec3)>,
    }

    /// Reads the Ship pane into [`ShipOverlay`] and the panel, and draws it.
    pub(super) struct ShipOverlayPlugin;

    impl Plugin for ShipOverlayPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<ShipOverlay>();
            app.add_systems(
                PostUpdate,
                (read_the_ship, draw_the_ship)
                    .chain()
                    .in_set(diagnostics::DiagnosticsSystems)
                    .after(TransformSystems::Propagate),
            );
        }
    }

    /// The Ship view camera's render layers, while it is open.
    fn ship_view_layers(world: &mut World) -> Option<RenderLayers> {
        world
            .query::<(&Name, &RenderLayers)>()
            .iter(world)
            .find(|(name, _)| name.as_str() == "NovaOsShipCamera")
            .map(|(_, layers)| layers.clone())
    }

    /// Each free socket of each live design section, with whether its lane
    /// is open.
    fn free_points(world: &mut World, views: &[ShipSectionView]) -> Vec<(Vec3, Vec3, bool)> {
        let Some(design) = world
            .query_filtered::<&ShipDesignSections, With<PlayerSpaceshipMarker>>()
            .iter(world)
            .next()
        else {
            return Vec::new();
        };
        let placed: Vec<RefitPlacedSection<'_>> = design
            .0
            .iter()
            .map(|section| RefitPlacedSection {
                position: section.position,
                rotation: section.rotation,
                config: &section.config,
                working: true,
            })
            .collect();
        let mut points = Vec::new();
        for (index, section) in design.0.iter().enumerate() {
            let Some(view) = views.iter().find(|view| {
                world
                    .get::<EntityId>(view.entity)
                    .is_some_and(|id| id.0 == section.id)
            }) else {
                continue;
            };
            for point in free_link_points(&placed, index) {
                let (anchor, normal) = link_point_pose(
                    section.position,
                    section.rotation,
                    &section.config.base.link_points[point],
                );
                let open = socket_lane_open(anchor, normal, view.entity, views);
                points.push((anchor, normal, open));
            }
        }
        points
    }

    /// Read the Ship view: put the gizmos on its layers, collect what they
    /// draw, and write the panel lines.
    fn read_the_ship(world: &mut World, state: &mut SystemState<ShipSections<'static, 'static>>) {
        let layers = ship_view_layers(world);
        diagnostics::draw_on_layers(
            &mut world.resource_mut::<GizmoConfigStore>(),
            layers.as_ref(),
        );
        let (Some(_), Some(runtime)) = (&layers, world.get_resource::<ShipRuntime>()) else {
            *world.resource_mut::<ShipOverlay>() = ShipOverlay::default();
            return;
        };
        let held = world.resource::<HeldPart>().0.clone();
        let rebind_armed = runtime.rebind_armed();
        let docked = !runtime.station_colliders().is_empty();
        let mount = runtime.mount();
        let selected = runtime.selected();
        let delete = runtime
            .delete_verdict()
            .map(|verdict| verdict.cloned().map_err(str::to_string));
        let station: Vec<(Vec3, Vec3)> = runtime
            .station_colliders()
            .iter()
            .map(|placed| (placed.position, placed.half_extents()))
            .collect();
        let views: Vec<ShipSectionView> = state
            .get(world)
            .map(|sections| sections.collect())
            .unwrap_or_default();
        let hovered = ship_hovered_section(world)
            .and_then(|hovered| views.iter().find(|view| view.entity == hovered));
        let free_points = free_points(world, &views);
        let placement = ship_placement(world).cloned().map(|placement| {
            let half = world
                .get_resource::<GameSections>()
                .and_then(|sections| sections.get_section(&placement.prototype))
                .map(|part| part.base.collider.unwrap_or_default().aabb_half_extents())
                .unwrap_or(Vec3::splat(0.5));
            (placement, half)
        });

        let mut lines = Vec::new();
        let mut details = Vec::new();
        if rebind_armed {
            lines.push("rebind: waiting for a key".to_string());
        }
        if let Some(part) = &held {
            lines.push(format!(
                "held: {part} roll {}/4 socket {}",
                mount.roll, mount.source
            ));
        }
        if let Some(view) = hovered.filter(|_| docked && !rebind_armed) {
            details.push(format!("hovered: {}", view.code));
        }
        match &placement {
            Some((placement, _)) => {
                lines.push(match &placement.verdict {
                    Ok(_) => "VALID".to_string(),
                    Err(line) => format!("REFUSED: {line}"),
                });
                details.push(format!(
                    "target: {} socket {}",
                    placement.section, placement.point
                ));
                if let Ok(line) = placement.verdict.as_ref() {
                    details.push(line.clone());
                }
                if let (Some(payment), Some(items)) =
                    (&placement.payment, world.get_resource::<GameItems>())
                {
                    let bought: Vec<String> = payment
                        .bought
                        .iter()
                        .map(|(item, count)| format!("{count} {}", items.design(item).name))
                        .collect();
                    let cost: u64 = payment
                        .bought
                        .iter()
                        .map(|(item, count)| items.design(item).reference_price().buy_cr(*count))
                        .sum();
                    details.push(if bought.is_empty() {
                        "quick buy: none".to_string()
                    } else {
                        format!("quick buy: {} = {cost} cr", bought.join(", "))
                    });
                }
            }
            None if held.is_some() => details.push("target: none".to_string()),
            None => {}
        }
        if let Some(view) =
            selected.and_then(|selected| views.iter().find(|view| view.entity == selected))
        {
            let binding = view.binding_text(world.resource::<nova_input::prelude::InputBindings>());
            if let Some(verdict) = &delete {
                details.push(match verdict {
                    Ok(parts) => {
                        let lines = delete_yield_lines(world.resource::<GameItems>(), parts);
                        format!(
                            "delete: {}",
                            if lines.is_empty() {
                                "nothing to salvage".to_string()
                            } else {
                                lines.join(", ")
                            }
                        )
                    }
                    Err(line) => line.clone(),
                });
            }
            details.push(format!(
                "selected: {} bind {}",
                view.code,
                binding.unwrap_or_else(|| "none".to_string())
            ));
        }
        let open = free_points.iter().filter(|(.., open)| *open).count();
        details.push(format!(
            "sockets: {open} open, {} closed; station boxes {}",
            free_points.len() - open,
            station.len()
        ));
        // Only the sockets the part in hand could move to next.
        let near: Vec<(Vec3, Vec3, bool)> = placement
            .as_ref()
            .map(|(placement, _)| {
                free_points
                    .iter()
                    .copied()
                    .filter(|(anchor, ..)| anchor.distance(placement.anchor) <= NEAR_TARGET_CELLS)
                    .collect()
            })
            .unwrap_or_default();

        let mut panel = world.resource_mut::<diagnostics::DiagnosticsPanel>();
        for line in lines {
            panel.line(line);
        }
        for line in details {
            panel.detail(line);
        }
        *world.resource_mut::<ShipOverlay>() = ShipOverlay {
            free_points: near,
            placement,
            station,
        };
    }

    /// Draw the free sockets, the target socket and the part's roll, the
    /// ghost's bounds and the station's collision volume.
    fn draw_the_ship(
        overlay: Res<ShipOverlay>,
        mut gizmos: Gizmos<diagnostics::DiagnosticsGizmos>,
    ) {
        for &(anchor, normal, lane_open) in &overlay.free_points {
            if lane_open {
                diagnostics::ring(&mut gizmos, anchor, normal, 0.15, palette::OK);
                gizmos.line(anchor, anchor + normal * 0.3, palette::OK);
            } else {
                let (a, b) = normal.any_orthonormal_pair();
                let size = 0.12;
                gizmos.line(
                    anchor - (a + b) * size,
                    anchor + (a + b) * size,
                    palette::MUTED,
                );
                gizmos.line(
                    anchor - (a - b) * size,
                    anchor + (a - b) * size,
                    palette::MUTED,
                );
            }
        }
        for &(centre, half) in &overlay.station {
            gizmos.cube(
                Transform::from_translation(centre).with_scale(half * 2.0),
                palette::ZONE,
            );
        }
        let Some((placement, half)) = &overlay.placement else {
            return;
        };
        diagnostics::vector(
            &mut gizmos,
            placement.anchor,
            placement.normal,
            0.8,
            palette::TARGET,
        );
        let Some(pose) = placement.pose else {
            return;
        };
        let bounds = if placement.verdict.is_ok() {
            palette::OK
        } else {
            palette::ALERT
        };
        gizmos.cube(pose.with_scale(*half * 2.0), bounds);
        // The roll: the part's own +X, which turns a quarter per R.
        diagnostics::vector(
            &mut gizmos,
            pose.translation,
            pose.rotation * Vec3::X,
            half.x + 0.4,
            palette::VELOCITY,
        );
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_input::prelude::InputSource;
    use nova_protocol::{
        nova_debug::harness::{
            press_action, release_action, shoot, AutopilotPlugin, Predicate, CAPTURE_RESOLUTION,
        },
        nova_interface::ship::{
            ship_ghost_aim, ship_hovered_section, ship_placement, ship_view_window_px, SectionCode,
            ShipRuntime, SHIP_HELD_PANEL,
        },
        nova_ship::sections::docking_section::GAP_SLACK,
    };
    use nova_ui::{
        theme::{ActiveUiTheme, SelectedUiTheme},
        widget::{Tooltip, TooltipPanel},
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
    /// The PNG of a thruster on the Main Drive, its default.
    const THRUSTER_DEFAULT_SHOT: &str = "sandbox-thruster-default.png";
    /// The PNG of the thruster on its own key.
    const THRUSTER_OWN_SHOT: &str = "sandbox-thruster-own-key.png";
    /// The PNG of the thruster back on the Main Drive after Backspace.
    const THRUSTER_CLEARED_SHOT: &str = "sandbox-thruster-cleared.png";
    /// The thruster's own key: free on every rig.
    const THRUSTER_KEY: KeyCode = KeyCode::KeyJ;
    /// The PNG of the valid ghost before the click.
    const GHOST_SHOT: &str = "sandbox-ghost.png";
    /// Frames the pointer rests on one free socket before the walk tries the
    /// next one.
    const AIM_FRAMES: u32 = 6;
    /// How far the aim sits from a socket toward its section's centre, as a
    /// share of the way.
    const AIM_INSET: f32 = 0.15;
    /// The PNG of the ghost rolled a quarter turn and on its next socket.
    const MOUNT_SHOT: &str = "sandbox-mount.png";
    /// The PNG of the Ship tab after the add.
    const PLACED_SHOT: &str = "sandbox-placed.png";
    /// The PNG of the Ship tab after Esc put the part in hand away: no ghost,
    /// the interface still open.
    const ESC_SHOT: &str = "sandbox-esc-put-away.png";
    /// The PNG of the placed section after its rebind.
    const REBIND_SHOT: &str = "sandbox-rebind.png";
    /// The PNG of the armed rebind: the lit button and the prompt banner.
    const REBIND_ARMED_SHOT: &str = "sandbox-rebind-armed.png";
    /// The PNG of the rebound gun's enabled Delete, the Salvage block under it
    /// naming the parts it gives back.
    const DELETE_YIELD_SHOT: &str = "sandbox-delete-yield.png";
    /// The PNG of the Ship tab once the rebound gun is deleted: the gun gone,
    /// the note naming the parts that went to the hold.
    const DELETE_SHOT: &str = "sandbox-delete.png";
    /// The PNG of the docked port's muted Delete under Salvage.
    const DELETE_REFUSED_SHOT: &str = "sandbox-delete-refused.png";
    /// The Ship panel's Delete button, inside the Salvage block.
    const DELETE_BUTTON: &str = "ShipDelete";
    /// The Ship panel's Salvage block.
    const SALVAGE_BLOCK: &str = "ShipSalvage";
    /// The muted Delete's tooltip on the port the dock holds by.
    const DOCKED_PORT_REASON: &str = "The dock holds the ship here";
    /// The muted Delete's tooltip on a section whose delete splits the ship.
    const SPLIT_REASON: &str = "Splits the ship: red sections lose the dock";
    /// The PNG of the Catalogue open on the kinetic PDC's category.
    const CATALOGUE_SHOT: &str = "sandbox-catalogue.png";
    /// The PNG of the Catalogue as it opens, on every part: each tile's name,
    /// kind and price or reason inside the tile, a part the station cannot
    /// craft muted.
    const CATALOGUE_ALL_SHOT: &str = "sandbox-catalogue-all.png";
    /// The PNG of the Catalogue open on the cargo intake's category.
    const CATALOGUE_INTAKE_SHOT: &str = "sandbox-catalogue-intake.png";
    /// The PNG of the hull part the pipette took off the ship.
    const PICK_SHOT: &str = "sandbox-pick.png";
    /// Mouse travel per frame while the walk drags the view, in pixels:
    /// rightward, the short way round to the station side. About 35 frames.
    const DRAG_PX: Vec2 = Vec2::new(16.0, 0.0);
    /// The PNG of the ghost refused against the docked station.
    const STATION_REFUSAL_SHOT: &str = "sandbox-station-refusal.png";
    /// How far from the station a free socket's aim may be for the walk to
    /// try it: the cargo intake's length end-on.
    const STATION_REACH_CELLS: f32 = 3.0;
    /// How far the Ship view turns toward the station before the walk aims:
    /// the camera looks along ship-local +X at least this much.
    const STATION_VIEW_FACING: f32 = 0.5;
    /// Seconds the walk gets to try every free socket against the station.
    const STATION_AIM_SECS: f32 = 600.0;
    /// The key the walk binds the placed gun to: `T`, the view's reset key
    /// and free of the flight table, so the panel binds it. `]` is the
    /// flight lock-next key and the panel refuses it.
    const REBIND_KEY: KeyCode = KeyCode::KeyT;
    /// How far the view's facing may drift over a key press and still count
    /// as not turned.
    const FACING_EPSILON: f32 = 1e-4;
    /// The PNG of the Map tab a click opens after the add.
    const MAP_AFTER_SHOT: &str = "sandbox-map-after.png";
    /// The Station tab, reached with two next-tab presses from Inventory,
    /// through Jobs.
    const STATION_SHOT: &str = "sandbox-station.png";
    /// The close-up of the player's berth at the station's port.
    const BERTH_SHOT: &str = "sandbox-berth.png";
    /// The close-up of the first hull for sale's berth.
    const SALE_BERTH_SHOT: &str = "sandbox-sale-berth.png";
    /// How far from the middle of a berth's gap the close-up may stand,
    /// engine units, nearest first: the first with a clear line of sight wins.
    const BERTH_VIEW_CELLS: [f32; 3] = [6.0, 9.0, 12.0];
    /// The most a close-up's line of sight may lean along the berth's axis,
    /// as the cosine to it: near square to the axis, so the gap shows end to
    /// end.
    const BERTH_VIEW_SQUARE: f32 = 0.35;
    /// How far a port's mount may stand from its design position and count
    /// as that section, engine units.
    const MOUNT_EPSILON: f32 = 1e-3;
    /// Points the close-up's line of sight is tested at, and the side of the
    /// cube tested at each, engine units.
    const BERTH_SIGHT_SAMPLES: u32 = 24;
    const BERTH_SIGHT_PROBE: f32 = 0.2;
    /// Seconds a click or a key gets to land.
    const BEAT_SECS: f32 = 60.0;
    /// How far inside the Ship view's top-left corner the walk points at
    /// empty space, in pixels: clear of the ship, which the view frames in
    /// its middle.
    const EMPTY_INSET_PX: f32 = 24.0;
    /// The Ship pane's 3D view.
    const SHIP_VIEWPORT: &str = "ShipViewport";
    /// The window shapes the focus cards are read at: the smallest the Ship
    /// pane supports, then the editor's stock shape.
    const CARD_SIZES: [(f32, f32); 2] = [(640.0, 600.0), (1024.0, 768.0)];
    /// The themes the focus cards are read in, with the name their PNGs use.
    const CARD_THEMES: [(&str, &str); 2] = [
        (nova_ui::theme::PHOSPHOR_THEME_ID, "phosphor"),
        (nova_ui::theme::HARDWARE_THEME_ID, "hardware"),
    ];
    /// Frames a theme switch gets to re-tint the open gallery.
    const THEME_FRAMES: u32 = 10;
    /// How far a card control may stand past the card's edge and count as
    /// inside it, in logical pixels: layout rounding.
    const CARD_SLACK_PX: f32 = 0.5;

    /// The part in hand before the pipette, and the section it picked from
    /// once Q is pressed.
    #[derive(Resource, Clone, Debug)]
    struct SmokePick {
        before: Option<String>,
        section: Option<Entity>,
    }

    /// The gun the delete takes off and the player's section count before.
    #[derive(Resource, Clone, Copy, Debug)]
    struct SmokeDelete {
        gun: Option<Entity>,
        sections: usize,
    }

    /// The section whose delete would split the ship, found by cycling the
    /// selection; the themed shots check it stays selected.
    #[derive(Resource, Clone, Copy, Debug)]
    struct SmokeSplit(Entity);

    /// The Ship view's eye and facing when the walk pressed on the ghost.
    #[derive(Resource, Clone, Copy, Debug)]
    struct SmokePivot {
        eye: Vec3,
        look: Vec3,
    }

    /// The share of its offset at the press the view's line of sight may keep
    /// from the placed section and count as eased onto it.
    const PIVOT_EASED: f32 = 0.5;

    /// The Ship view camera's position and facing, in the ship-local scene.
    fn ship_view_pose(world: &World) -> Option<(Vec3, Vec3)> {
        let mut cameras = world.try_query::<(&Name, &GlobalTransform)>()?;
        cameras
            .iter(world)
            .find(|(name, _)| name.as_str() == "NovaOsShipCamera")
            .map(|(_, pose)| (pose.translation(), *pose.forward()))
    }

    /// Where the live section `section` stands in the player's design.
    fn design_position(world: &World, section: Entity) -> Option<Vec3> {
        let id = world.get::<EntityId>(section)?;
        let mut designs =
            world.try_query_filtered::<&ShipDesignSections, With<PlayerSpaceshipMarker>>()?;
        let design = designs.iter(world).next()?;
        design.get(&id.0).map(|section| section.position)
    }

    /// Where the placed kinetic PDC stands, once the add selected it.
    fn placed_position(world: &World) -> Option<Vec3> {
        let section = world.resource::<ShipRuntime>().selected()?;
        if built_part(world, section).as_deref() != Some(PDC_KINETIC_TURRET_SECTION_ID) {
            return None;
        }
        design_position(world, section)
    }

    /// How far the placed section stands off the view's line of sight at the
    /// press and now, in scene units.
    fn pivot_offsets(world: &World) -> Option<(f32, f32)> {
        let pivot = world.get_resource::<SmokePivot>()?;
        let placed = placed_position(world)?;
        let (eye, look) = ship_view_pose(world)?;
        let off_sight = |eye: Vec3, look: Vec3| {
            let to = placed - eye;
            (to - look * to.dot(look)).length()
        };
        Some((off_sight(pivot.eye, pivot.look), off_sight(eye, look)))
    }

    /// The Ship view's facing and selection when the rebind armed.
    #[derive(Resource, Clone, Copy, Debug)]
    struct SmokeArmed {
        facing: Vec3,
        selected: Option<Entity>,
    }

    /// The player's section count before the add, the ship-local points the
    /// walk aims the ghost through, and the one it is on (`usize::MAX` before
    /// the first).
    #[derive(Resource, Clone, Debug)]
    struct SmokeAdd {
        sections: usize,
        aims: Vec<Vec3>,
        aiming: usize,
        /// Socket steps taken at the current aim.
        steps: u32,
    }

    /// Whether the selected section fires on [`REBIND_KEY`].
    fn rebound(world: &World) -> bool {
        let Some(runtime) = world.get_resource::<ShipRuntime>() else {
            return false;
        };
        runtime
            .selected()
            .and_then(|section| world.get::<SpaceshipTurretInputBinding>(section))
            .is_some_and(|binding| binding.0.contains(&InputSource::from(REBIND_KEY)))
    }

    /// The Ship panel's Catalogue button, which opens the parts gallery.
    const CATALOGUE_BUTTON: &str = "ShipCatalogue";

    /// The gallery tile of `part`, named by its display name.
    fn part_tile(world: &World, part: &str) -> String {
        let name = &world
            .resource::<GameSections>()
            .get_section(part)
            .expect("the walk's parts are in the catalog")
            .base
            .name;
        format!("Gallery Tile {name}")
    }

    /// The category chip `part` lives under: the station lists more parts
    /// than the All page shows on a small window.
    fn part_category(world: &World, part: &str) -> GalleryCategory {
        GalleryCategory::of(
            &world
                .resource::<GameSections>()
                .get_section(part)
                .expect("the walk's parts are in the catalog")
                .kind,
        )
    }

    /// Open the Catalogue, narrow it to the category of `part`, shoot it as
    /// `shot`, focus the tile of `part` and press Place, so `part` is in
    /// hand. Records the section count and the free socket aims the first
    /// time.
    fn take_the_part(
        script: AutopilotPlugin<GameStates>,
        part: &'static str,
        opened_shot: Option<&'static str>,
        shot: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        let script = script
            .step(format!("station sandbox: note the ship before {part}"))
            .on_enter(|world: &mut World| {
                if world.get_resource::<SmokeAdd>().is_some() {
                    return;
                }
                let sections = player_sections(world);
                let aims = free_socket_aims(world);
                world.insert_resource(SmokeAdd {
                    sections,
                    aims,
                    aiming: usize::MAX,
                    steps: 0,
                });
            })
            .until(frames(1))
            .add()
            .click_named(
                &format!("station sandbox: open the Catalogue for {part}"),
                CATALOGUE_BUTTON,
                ui_node_present("Gallery Grid"),
                BEAT_SECS,
            );
        let script = match opened_shot {
            Some(opened_shot) => script
                .step(format!(
                    "station sandbox: settle the opened Catalogue for {part}"
                ))
                .until(frames(SETTLE_FRAMES))
                .add()
                .step(format!(
                    "station sandbox: shoot the opened Catalogue for {part}"
                ))
                .on_enter(move |world: &mut World| shoot(world, opened_shot))
                .until(shot_written(opened_shot))
                .deadline(SHOT_SECS)
                .add(),
            None => script,
        };
        script
            .step(format!("station sandbox: narrow the Catalogue to {part}"))
            .on_enter(move |world: &mut World| {
                let chip = format!("Gallery Category {}", part_category(world, part).label());
                click_named(chip)(world);
            })
            .until(pointer_pressed())
            .deadline(BEAT_SECS)
            .add()
            .step(format!("station sandbox: let the {part} chip up"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(Arc::new(move |world: &World| {
                world.resource::<GalleryState>().category == part_category(world, part)
                    && ui_node_present(part_tile(world, part))(world)
            }))
            .diagnose(move |world| {
                format!(
                    "{:?}, tile {}",
                    world.resource::<GalleryState>(),
                    part_tile(world, part)
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step(format!("station sandbox: settle the Catalogue for {part}"))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step(format!("station sandbox: shoot the Catalogue for {part}"))
            .on_enter(move |world: &mut World| shoot(world, shot))
            .until(shot_written(shot))
            .deadline(SHOT_SECS)
            .add()
            .step(format!("station sandbox: press the {part} tile"))
            .on_enter(move |world: &mut World| click_named(part_tile(world, part))(world))
            .until(pointer_pressed())
            .deadline(BEAT_SECS)
            .add()
            .step(format!("station sandbox: let the {part} tile up"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(ui_node_present("Gallery Place Button"))
            .deadline(BEAT_SECS)
            .add()
            .click_named(
                &format!("station sandbox: place {part}"),
                "Gallery Place Button",
                Arc::new(move |world: &World| world.resource::<HeldPart>().id() == Some(part)),
                BEAT_SECS,
            )
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
                working: true,
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
        if ghost_is_valid(world) || !frame.is_multiple_of(AIM_FRAMES) {
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

    /// The catalogue part the live section `section` of the player ship was
    /// built from.
    fn built_part(world: &World, section: Entity) -> Option<String> {
        let id = world.get::<EntityId>(section)?;
        let mut designs =
            world.try_query_filtered::<&ShipDesignSections, With<PlayerSpaceshipMarker>>()?;
        let design = designs.iter(world).next()?;
        design
            .get(&id.0)
            .map(|section| section.config.base.id.clone())
    }

    /// Point at the centre of a hull section whose part is not in hand, and
    /// record the part in hand for the pipette.
    fn point_at_a_hull(world: &mut World) {
        let before = world.resource::<HeldPart>().0.clone();
        let Some(sections) = world
            .query_filtered::<&ShipDesignSections, With<PlayerSpaceshipMarker>>()
            .iter(world)
            .next()
            .map(|design| design.0.clone())
        else {
            return;
        };
        let pixel = sections.iter().find_map(|section| {
            let part = &section.config.base;
            if before.as_deref() == Some(part.id.as_str())
                || section.config.kind.class() != SectionClass::Hull
                || part.hide_in_editor
            {
                return None;
            }
            ship_view_window_px(world, section.position)
        });
        let Some(pixel) = pixel else {
            return;
        };
        move_cursor(pixel)(world);
        world.insert_resource(SmokePick {
            before,
            section: None,
        });
    }

    /// Whether Q put the part of the section under the pointer in hand.
    fn picked_the_hull(world: &World) -> bool {
        let Some(pick) = world.get_resource::<SmokePick>() else {
            return false;
        };
        let held = world
            .get_resource::<HeldPart>()
            .and_then(|held| held.0.clone());
        let built = pick.section.and_then(|section| built_part(world, section));
        held.is_some() && held != pick.before && held == built
    }

    /// Which way the Ship view camera looks, in the ship-local scene.
    fn ship_view_facing(world: &World) -> Vec3 {
        let Some(mut cameras) = world.try_query::<(&Name, &GlobalTransform)>() else {
            return Vec3::ZERO;
        };
        cameras
            .iter(world)
            .find(|(name, _)| name.as_str() == "NovaOsShipCamera")
            .map_or(Vec3::ZERO, |(_, pose)| *pose.forward())
    }

    /// Whether the ghost stands refused because it would hit the station.
    fn refused_by_the_station(world: &World) -> bool {
        ship_placement(world).is_some_and(|placement| {
            matches!(&placement.verdict, Err(line) if line.contains("would hit the station"))
        })
    }

    /// Aim again from the free sockets nearest the docked station.
    fn aim_from_the_station(world: &mut World) {
        let mut aims = free_socket_aims(world);
        let station: Vec<(Vec3, Vec3)> = world
            .resource::<ShipRuntime>()
            .station_colliders()
            .iter()
            .map(|placed| (placed.position, placed.half_extents()))
            .collect();
        // The gap from an aim to the nearest station box's surface.
        let reach = |aim: &Vec3| {
            station
                .iter()
                .map(|(centre, half)| ((*aim - *centre).abs() - *half).max(Vec3::ZERO).length())
                .fold(f32::INFINITY, f32::min)
        };
        aims.sort_by(|a, b| reach(a).total_cmp(&reach(b)));
        let free = aims.len();
        aims.retain(|aim| reach(aim) <= STATION_REACH_CELLS);
        info!(
            "station sandbox: {free} free sockets, {} within reach, {} station boxes, nearest \
             gaps {:?} cells",
            aims.len(),
            station.len(),
            aims.iter().take(3).map(reach).collect::<Vec<_>>()
        );
        let mut add = world.resource_mut::<SmokeAdd>();
        add.aims = aims;
        add.aiming = usize::MAX;
        add.steps = 0;
    }

    /// Point at the free sockets in turn until the ghost hits the station,
    /// trying each mating socket of the part at each one; past the last aim,
    /// `aiming` stands at the aim count.
    fn aim_into_the_station(world: &mut World, _elapsed: f32, frame: u32) {
        if frame % AIM_FRAMES == 1 {
            release_action("ship_part_socket")(world);
        }
        if refused_by_the_station(world) || !frame.is_multiple_of(AIM_FRAMES) {
            return;
        }
        let Some(add) = world.get_resource::<SmokeAdd>().cloned() else {
            return;
        };
        debug!(
            "station sandbox: aim {} at {:?} step {}: {:?}",
            add.aiming,
            add.aims.get(add.aiming),
            add.steps,
            ship_placement(world).map(|placement| (
                &placement.section,
                &placement.point,
                placement.mount,
                &placement.verdict
            )),
        );
        // Every socket of the part in hand gets a turn at each aim.
        let tries = world
            .resource::<HeldPart>()
            .id()
            .and_then(|part| world.resource::<GameSections>().get_section(part))
            .map_or(0, |part| part.base.link_points.len());
        if add.aiming < add.aims.len() && (add.steps as usize) < tries {
            world.resource_mut::<SmokeAdd>().steps += 1;
            press_action("ship_part_socket")(world);
            return;
        }
        let aiming = add.aiming.wrapping_add(1).min(add.aims.len());
        let mut next = world.resource_mut::<SmokeAdd>();
        next.aiming = aiming;
        next.steps = 0;
        if let Some(pixel) = add
            .aims
            .get(aiming)
            .and_then(|&aim| ship_view_window_px(world, aim))
        {
            move_cursor(pixel)(world);
        }
    }

    fn aims_spent(world: &World) -> bool {
        world
            .get_resource::<SmokeAdd>()
            .is_some_and(|add| add.aiming == add.aims.len())
    }

    fn ghost_is_valid(world: &World) -> bool {
        ship_ghost_aim(world).is_some_and(|(_, _, valid)| valid)
    }

    /// How the part in hand mates its socket.
    fn mount(world: &World) -> MountPose {
        world
            .get_resource::<ShipRuntime>()
            .map(ShipRuntime::mount)
            .unwrap_or_default()
    }

    /// Tap `action` while `more` holds: press on even frames, release on odd
    /// ones, so each tap is one step of the pose.
    fn tap_while(
        action: &'static str,
        more: fn(&World) -> bool,
    ) -> impl Fn(&mut World, f32, u32) + Send + Sync + 'static {
        move |world, _, frame| {
            if frame % 2 == 0 && more(world) {
                press_action(action)(world);
            } else {
                release_action(action)(world);
            }
        }
    }

    /// The player's port the dock holds by, while it is docked.
    /// The player ship's first thruster by code (`THR-1`).
    fn first_thruster(world: &World) -> Option<Entity> {
        let player = world
            .try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?
            .single(world)
            .ok()?;
        let mut thrusters = world
            .try_query_filtered::<(Entity, &ChildOf, &SectionCode), With<ThrusterSectionMarker>>(
            )?;
        thrusters
            .iter(world)
            .filter(|(_, parent, _)| parent.parent() == player)
            .min_by(|(_, _, a), (_, _, b)| a.0.cmp(&b.0))
            .map(|(thruster, ..)| thruster)
    }

    /// The first thruster's own key, or `None` while it fires on the Main
    /// Drive.
    fn thruster_own_binding(world: &World) -> Option<Vec<InputSource>> {
        first_thruster(world)
            .and_then(|thruster| world.get::<SpaceshipThrusterInputBinding>(thruster))
            .map(|binding| binding.0.clone())
    }

    /// The first thruster's throttle.
    fn thruster_throttle(world: &World) -> Option<f32> {
        first_thruster(world)
            .and_then(|thruster| world.get::<ThrusterSectionInput>(thruster))
            .map(|input| input.0)
    }

    /// The Ship panel's Control row text.
    fn control_text(world: &World) -> Option<String> {
        let mut labels = world.try_query::<(&Name, &Text)>()?;
        labels
            .iter(world)
            .find(|(name, _)| name.as_str() == "ShipControl")
            .map(|(_, text)| text.0.clone())
    }

    /// Whether the player ship is docked.
    fn player_docked(world: &World) -> bool {
        world
            .try_query_filtered::<(), (With<PlayerSpaceshipMarker>, With<DockedShip>)>()
            .is_some_and(|mut docked| docked.iter(world).next().is_some())
    }

    /// The Control row names the Main Drive keys, read off the live table.
    fn on_the_main_drive(world: &World) -> bool {
        thruster_own_binding(world).is_none()
            && control_text(world)
                .is_some_and(|text| text.starts_with("Main Drive: ") && text.contains("Space"))
    }

    fn select_the_thruster(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        script
            .step("station sandbox: select the first thruster")
            .each(tap_while("viewer_next", |world| {
                world.resource::<ShipRuntime>().selected() != first_thruster(world)
            }))
            .until(Arc::new(|world: &World| {
                first_thruster(world).is_some()
                    && world.resource::<ShipRuntime>().selected() == first_thruster(world)
            }))
            .diagnose(|world| {
                format!(
                    "selected {:?}, thruster {:?}",
                    world.resource::<ShipRuntime>().selected(),
                    first_thruster(world)
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let the thruster select key up")
            .on_enter(release_action("viewer_next"))
            .until(frames(SETTLE_FRAMES))
            .add()
    }

    fn arm_the_thruster_rebind(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        script
            .step("station sandbox: arm a rebind on the thruster")
            .on_enter(press_action("ship_rebind"))
            .until(Arc::new(|world: &World| {
                world
                    .get_resource::<ShipRuntime>()
                    .is_some_and(ShipRuntime::rebind_armed)
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let the thruster rebind key up")
            .on_enter(release_action("ship_rebind"))
            .until(frames(SETTLE_FRAMES))
            .add()
    }

    fn docked_port(world: &World) -> Option<Entity> {
        let mut players =
            world.try_query_filtered::<(Entity, &DockedShip), With<PlayerSpaceshipMarker>>()?;
        let (player, docked) = players.single(world).ok()?;
        let connection = world.get::<DockingConnection>(docked.connection)?;
        Some(if connection.first_ship == player {
            connection.first_section
        } else {
            connection.second_section
        })
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

    /// The Delete button's label and whether it takes a click.
    fn delete_button(world: &World) -> Option<(String, bool)> {
        let mut buttons =
            world.try_query::<(Entity, &Name, Has<bevy::ui::InteractionDisabled>)>()?;
        let (button, enabled) = buttons
            .iter(world)
            .find(|(_, name, _)| name.as_str() == DELETE_BUTTON)
            .map(|(button, _, disabled)| (button, !disabled))?;
        let mut labels = world.try_query::<(&ChildOf, &Text)>()?;
        let label = labels
            .iter(world)
            .find(|(parent, _)| parent.parent() == button)
            .map(|(_, text)| text.0.clone())?;
        Some((label, enabled))
    }

    /// The Delete button's tooltip text: why it is refused, empty while it
    /// holds.
    fn delete_tooltip_text(world: &World) -> Option<String> {
        world
            .get::<Tooltip>(named_entity(world, DELETE_BUTTON)?)
            .map(|tooltip| tooltip.0.clone())
    }

    /// The text of the tooltip panel under Delete, when the panel shows.
    fn delete_tooltip(world: &World) -> Option<String> {
        shown_tooltip(world, named_entity(world, DELETE_BUTTON)?)
    }

    /// The player ship's root.
    fn player_root(world: &World) -> Option<Entity> {
        world
            .try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?
            .single(world)
            .ok()
    }

    /// The selected section's code, for the log.
    fn selected_code(world: &World) -> Option<String> {
        let selected = world.resource::<ShipRuntime>().selected()?;
        world
            .get::<SectionCode>(selected)
            .map(|code| code.0.clone())
    }

    /// The Salvage block's texts in tree order while the panel shows it;
    /// `None` while it is hidden.
    fn salvage_texts(world: &World) -> Option<Vec<String>> {
        let mut blocks = world.try_query::<(Entity, &Name, &Node)>()?;
        let (block, _, node) = blocks
            .iter(world)
            .find(|(_, name, _)| name.as_str() == SALVAGE_BLOCK)?;
        if node.display != Display::Flex {
            return None;
        }
        let mut lines = Vec::new();
        let mut stack = vec![block];
        while let Some(entity) = stack.pop() {
            if let Some(text) = world.get::<Text>(entity).filter(|text| !text.0.is_empty()) {
                lines.push(text.0.clone());
            }
            if let Some(children) = world.get::<Children>(entity) {
                stack.extend(children.iter().rev());
            }
        }
        Some(lines)
    }

    /// The Placing block's texts in tree order while the panel shows it in
    /// place of the selection; `None` while it is hidden.
    fn held_panel_lines(world: &World) -> Option<Vec<String>> {
        let mut blocks = world.try_query::<(Entity, &Name, &Node)>()?;
        let (block, _, node) = blocks
            .iter(world)
            .find(|(_, name, _)| name.as_str() == SHIP_HELD_PANEL)?;
        if node.display != Display::Flex {
            return None;
        }
        let mut lines = Vec::new();
        let mut stack = vec![block];
        while let Some(entity) = stack.pop() {
            if let Some(text) = world.get::<Text>(entity).filter(|text| !text.0.is_empty()) {
                lines.push(text.0.clone());
            }
            if let Some(children) = world.get::<Children>(entity) {
                stack.extend(children.iter().rev());
            }
        }
        Some(lines)
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

    /// The berths the close-ups judge, as `(station port, hull, hull port)`:
    /// the player's, then every hull for sale docked at the station, in park
    /// order.
    fn station_berths(world: &World) -> Vec<(Entity, Entity, Entity)> {
        let ports = |ship: Entity| -> Option<(Entity, Entity, Entity)> {
            let docked = world.get::<DockedShip>(ship)?;
            let connection = world.get::<DockingConnection>(docked.connection)?;
            Some(if connection.first_ship == ship {
                (connection.second_section, ship, connection.first_section)
            } else {
                (connection.first_section, ship, connection.second_section)
            })
        };
        let Some(player) = world
            .try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
            .and_then(|mut players| players.single(world).ok())
        else {
            return Vec::new();
        };
        let Some(station) = berth_station(world, player) else {
            return Vec::new();
        };
        let sales = world
            .get::<ParkedHulls>(station)
            .into_iter()
            .flat_map(|parked| parked.iter())
            .filter(|&hull| world.get::<ParkedHullType>(hull) == Some(&ParkedHullType::Sale));
        std::iter::once(player)
            .chain(sales)
            .filter_map(ports)
            .collect()
    }

    /// The station `player` is docked at.
    fn berth_station(world: &World, player: Entity) -> Option<Entity> {
        let docked = world.get::<DockedShip>(player)?;
        let connection = world.get::<DockingConnection>(docked.connection)?;
        Some(if connection.first_ship == player {
            connection.second_ship
        } else {
            connection.first_ship
        })
    }

    /// `root`'s envelope ([`design_envelope`]) in `station`'s frame, from
    /// both roots' live poses.
    fn station_frame_envelope(
        world: &World,
        station: Entity,
        root: Entity,
    ) -> Vec<PlacedSectionCollider> {
        let pose = |entity: Entity| {
            (
                world
                    .get::<Position>(entity)
                    .expect("a berthed root stands")
                    .0,
                world
                    .get::<Rotation>(entity)
                    .expect("a berthed root stands")
                    .0,
            )
        };
        let (station_at, station_turn) = pose(station);
        let (at, turn) = pose(root);
        let into_station = station_turn.inverse();
        let design = world
            .get::<ShipDesignSections>(root)
            .expect("station sandbox: a berthed root has its design sections");
        let ShipSkin(skin) = *world
            .get::<ShipSkin>(root)
            .expect("station sandbox: a berthed root has its skin flag");
        design_envelope(
            &design.0,
            skin,
            (into_station * (at - station_at), into_station * turn),
        )
    }

    /// The box `port` takes in `root`'s `envelope`: the docking section of
    /// `root`'s design mounted where the port is.
    fn port_box(
        world: &World,
        root: Entity,
        port: Entity,
        envelope: &[PlacedSectionCollider],
    ) -> PlacedSectionCollider {
        let (at, _) = local_pose(world, port, root);
        let design = world
            .get::<ShipDesignSections>(root)
            .expect("station sandbox: a berthed root has its design sections");
        let index = design
            .0
            .iter()
            .position(|section| {
                section.config.kind.class() == SectionClass::Docking
                    && section.position.distance(at) < MOUNT_EPSILON
            })
            .unwrap_or_else(|| {
                panic!("station sandbox: port {port:?} is no docking section of {root:?}'s design")
            });
        envelope[index]
    }

    /// Every berth's measured gap between the facing faces of its two port
    /// boxes, along the station port's axis, and every pair of the station
    /// and its berthed hulls whose envelopes overlap
    /// ([`section_colliders_overlap`]). Measured in the station's frame.
    fn berth_report(world: &World) -> (Vec<(Entity, f32)>, Vec<(Entity, Entity)>) {
        let berths = station_berths(world);
        let Some(station) = berths
            .first()
            .and_then(|&(_, player, _)| berth_station(world, player))
        else {
            return (Vec::new(), Vec::new());
        };
        let bodies: Vec<(Entity, Vec<PlacedSectionCollider>)> = std::iter::once(station)
            .chain(berths.iter().map(|&(_, hull, _)| hull))
            .map(|root| (root, station_frame_envelope(world, station, root)))
            .collect();
        let gaps = berths
            .iter()
            .zip(&bodies[1..])
            .map(|(&(station_port, hull, hull_port), (_, envelope))| {
                let near = port_box(world, station, station_port, &bodies[0].1);
                let far = port_box(world, hull, hull_port, envelope);
                let axis = near.rotation * Vec3::NEG_Z;
                let reach = (near.half_extents() + far.half_extents()).dot(axis.abs());
                (hull, (far.position - near.position).dot(axis) - reach)
            })
            .collect();
        let mut overlaps = Vec::new();
        for (index, (first, first_envelope)) in bodies.iter().enumerate() {
            for (second, second_envelope) in &bodies[index + 1..] {
                let touches = first_envelope.iter().any(|&a| {
                    second_envelope
                        .iter()
                        .any(|&b| section_colliders_overlap(a, b))
                });
                if touches {
                    overlaps.push((*first, *second));
                }
            }
        }
        (gaps, overlaps)
    }

    /// Whether the player and at least one hull for sale stand berthed, each
    /// [`BERTH_GAP`] off its station port within [`GAP_SLACK`], with no two
    /// envelopes overlapping.
    fn berths_hold(world: &World) -> bool {
        let (gaps, overlaps) = berth_report(world);
        gaps.len() >= 2
            && gaps
                .iter()
                .all(|(_, gap)| (gap - BERTH_GAP).abs() <= GAP_SLACK)
            && overlaps.is_empty()
    }

    /// Whether both sleeves of every berth are fully out.
    fn sleeves_out(world: &World) -> bool {
        let berths = station_berths(world);
        berths.len() >= 2
            && berths.iter().all(|&(station_port, _, hull_port)| {
                [station_port, hull_port].iter().all(|&port| {
                    world.get::<DockingSectionState>(port) == Some(&DockingSectionState::Extended)
                })
            })
    }

    /// A close-up of `hull`'s berth: the middle of its gap, seen near square
    /// to the berth's axis ([`BERTH_VIEW_SQUARE`]), level, above or below,
    /// from the nearest distance in [`BERTH_VIEW_CELLS`] whose line of sight
    /// crosses no box of the station or a berthed hull. With none clear, the
    /// first eye tried, and every tried eye in the station's frame with the
    /// root whose box blocks it and how far from the middle the block stands.
    /// `None` when `hull` is not berthed at the player's station.
    fn berth_view(
        world: &World,
        hull: Entity,
    ) -> Option<(ScriptedCameraPose, Option<Vec<(Vec3, Entity, f32)>>)> {
        let berths = station_berths(world);
        let station = berth_station(world, berths.first()?.1)?;
        let &(station_port, _, hull_port) =
            berths.iter().find(|&&(_, berthed, _)| berthed == hull)?;
        let boxes: Vec<(Entity, PlacedSectionCollider)> = std::iter::once(station)
            .chain(berths.iter().map(|&(_, berthed, _)| berthed))
            .flat_map(|root| {
                station_frame_envelope(world, station, root)
                    .into_iter()
                    .map(move |boxed| (root, boxed))
            })
            .collect();
        let near = port_box(
            world,
            station,
            station_port,
            &station_frame_envelope(world, station, station),
        );
        let far = port_box(
            world,
            hull,
            hull_port,
            &station_frame_envelope(world, station, hull),
        );
        let middle = (near.position + far.position) * 0.5;
        let axis = near.rotation * Vec3::NEG_Z;
        // The first box the line from `eye` to the middle crosses, eye first.
        let blocker = |eye: Vec3| {
            (1..BERTH_SIGHT_SAMPLES).find_map(|step| {
                let at = eye.lerp(middle, step as f32 / BERTH_SIGHT_SAMPLES as f32);
                let probe = PlacedSectionCollider {
                    position: at,
                    rotation: Quat::IDENTITY,
                    collider: SectionCollider::Cuboid {
                        size: Vec3::splat(BERTH_SIGHT_PROBE),
                    },
                };
                boxes
                    .iter()
                    .find(|&&(_, boxed)| section_colliders_overlap(probe, boxed))
                    .map(|&(root, _)| (root, at.distance(middle)))
            })
        };
        let at = world.get::<Position>(station)?.0;
        let turn = world.get::<Rotation>(station)?.0;
        // Every neighbour direction near square to the axis, highest in the
        // world first: the camera's up is world +Y, so a side near it is
        // dropped.
        let mut sides: Vec<Vec3> = (-1..=1)
            .flat_map(|x| (-1..=1).flat_map(move |y| (-1..=1).map(move |z| (x, y, z))))
            .filter(|&step| step != (0, 0, 0))
            .map(|(x, y, z)| Vec3::new(x as f32, y as f32, z as f32).normalize())
            .filter(|side| side.dot(axis).abs() < BERTH_VIEW_SQUARE && (turn * *side).y.abs() < 0.9)
            .collect();
        sides.sort_by(|a, b| (turn * *b).y.total_cmp(&(turn * *a).y));
        let pose = |eye: Vec3| ScriptedCameraPose {
            position: Meters3::from_engine(at + turn * eye),
            look_at: Meters3::from_engine(at + turn * middle),
        };
        let eyes: Vec<Vec3> = BERTH_VIEW_CELLS
            .iter()
            .flat_map(|&cells| sides.iter().map(move |side| middle + *side * cells))
            .collect();
        let mut blocked = Vec::new();
        for &eye in &eyes {
            match blocker(eye) {
                None => return Some((pose(eye), None)),
                Some((root, from_middle)) => blocked.push((eye, root, from_middle)),
            }
        }
        Some((pose(*eyes.first()?), Some(blocked)))
    }

    /// Pin the scenario camera on the close-up of the `index`th berth, if it
    /// is berthed.
    fn frame_the_berth(index: usize) -> impl Fn(&mut World) + Send + Sync + 'static {
        move |world: &mut World| {
            let Some(&(_, hull, _)) = station_berths(world).get(index) else {
                return;
            };
            let Some((view, blocked)) = berth_view(world, hull) else {
                error!("station sandbox: berth {index} ({hull:?}) is not berthed");
                return;
            };
            // A station port recessed in a plated surface rings the gap with
            // its own plates: no eye sees in, so the shot shows the pocket's
            // rim against the hull's collar from the first eye.
            if let Some(blocked) = blocked {
                warn!(
                    "station sandbox: berth {index} ({hull:?}) gap is hidden; \
                     (eye, blocking root, block's distance from the gap) {blocked:?}"
                );
            }
            info!(
                "station sandbox: berth {index} ({hull:?}) close-up from {:?} looking at {:?}",
                view.position, view.look_at
            );
            let cameras: Vec<Entity> = world
                .query_filtered::<Entity, With<ScenarioCameraMarker>>()
                .iter(world)
                .collect();
            for camera in cameras {
                world.entity_mut(camera).insert(view);
            }
        }
    }

    /// Whether the scenario camera is pinned on a close-up.
    fn camera_pinned(world: &World) -> bool {
        world
            .try_query_filtered::<(), (With<ScenarioCameraMarker>, With<ScriptedCameraPose>)>()
            .is_some_and(|mut cameras| cameras.iter(world).next().is_some())
    }

    /// The berth close-ups: the interface closes so the clocks run and the
    /// sleeves reach out, the berths are measured, and the player's berth and
    /// the first hull for sale's are shot; then the camera goes back to its
    /// rig and the Ship tab opens again.
    fn berth_shots(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        let script = script
            .step("station sandbox: close the interface for the berth shots")
            .on_enter(press_action("interface_toggle"))
            .until(Arc::new(|world: &World| {
                world
                    .get_resource::<State<PauseStates>>()
                    .is_some_and(|pause| *pause.get() == PauseStates::Unpaused)
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let the close key up for the berth shots")
            .on_enter(release_action("interface_toggle"))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: the berth sleeves reach out")
            .until(Arc::new(sleeves_out))
            .diagnose(|world| {
                let states: Vec<_> = station_berths(world)
                    .into_iter()
                    .map(|(station_port, hull, hull_port)| {
                        (
                            hull,
                            world.get::<DockingSectionState>(station_port).copied(),
                            world.get::<DockingSectionState>(hull_port).copied(),
                        )
                    })
                    .collect();
                format!("berths (hull, station sleeve, hull sleeve): {states:?}")
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: the berths stand one cell off and clear")
            .on_enter(|world: &mut World| {
                let (gaps, overlaps) = berth_report(world);
                info!(
                    "station sandbox: berth gaps {gaps:?} (want {BERTH_GAP} within {GAP_SLACK}), \
                     overlapping envelopes {overlaps:?}"
                );
            })
            .until(Arc::new(berths_hold))
            .diagnose(|world| {
                let (gaps, overlaps) = berth_report(world);
                format!("gaps {gaps:?}, overlaps {overlaps:?}")
            })
            .deadline(BEAT_SECS)
            .add();
        let script = [(0, BERTH_SHOT), (1, SALE_BERTH_SHOT)].into_iter().fold(
            script,
            |script, (index, shot)| {
                script
                    .step(format!("station sandbox: frame berth {index}"))
                    .on_enter(frame_the_berth(index))
                    .until(Arc::new(camera_pinned))
                    .deadline(BEAT_SECS)
                    .add()
                    .step(format!("station sandbox: settle berth {index}"))
                    .until(frames(SETTLE_FRAMES))
                    .add()
                    .step(format!("station sandbox: shoot berth {index}"))
                    .on_enter(move |world: &mut World| shoot(world, shot))
                    .until(shot_written(shot))
                    .deadline(SHOT_SECS)
                    .add()
                    .step(format!("station sandbox: unpin berth {index}"))
                    .on_enter(|world: &mut World| {
                        let cameras: Vec<Entity> = world
                            .query_filtered::<Entity, With<ScriptedCameraPose>>()
                            .iter(world)
                            .collect();
                        for camera in cameras {
                            world.entity_mut(camera).remove::<ScriptedCameraPose>();
                        }
                    })
                    .until(Arc::new(|world: &World| !camera_pinned(world)))
                    .deadline(BEAT_SECS)
                    .add()
            },
        );
        script
            .step("station sandbox: reopen the Ship tab after the berth shots")
            .on_enter(open_the_ship_tab)
            .until(ready_on_the_ship_tab())
            .deadline(BEAT_SECS)
            .add()
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

    /// The open Catalogue's listing.
    fn listing(world: &mut World) -> GalleryListing {
        world
            .query::<&GalleryListing>()
            .single(world)
            .expect("the open Catalogue carries one listing")
            .clone()
    }

    /// The chosen station's [`StationBuilds`].
    fn station_builds(world: &mut World) -> Vec<String> {
        let (station, ..) = world
            .resource::<Sandbox>()
            .station
            .clone()
            .expect("the sandbox chose a station");
        world
            .query::<(&GeneratedBodyId, &StationBuilds)>()
            .iter(world)
            .find(|(id, _)| **id == station)
            .map(|(_, builds)| builds.0.clone())
            .expect("the chosen station carries StationBuilds")
    }

    /// The card the sweep reads `card` on: the twin kinetic PDC, a four-item
    /// recipe and, with the rebound PDC selected, a Swap button; or the first
    /// listed part the station does not build, muted. The seed-10 station
    /// builds a subset, so a station with no such part fails here.
    fn card_part(world: &mut World, card: &str) -> String {
        if card != "muted" {
            return PDC_TWIN_KINETIC_TURRET_SECTION_ID.to_string();
        }
        let builds = station_builds(world);
        listing(world)
            .0
            .into_iter()
            .find(|entry| !builds.contains(&entry.id))
            .expect("the sandbox station does not build some part")
            .id
    }

    /// The text of the tooltip panel under the focus card's Place, when the
    /// panel shows.
    fn place_tooltip(world: &World) -> Option<String> {
        shown_tooltip(world, named_entity(world, "Gallery Place Button")?)
    }

    /// The text of the tooltip panel under `node`, when the panel shows.
    fn shown_tooltip(world: &World, node: Entity) -> Option<String> {
        let panel = world
            .get::<Children>(node)?
            .iter()
            .find(|child| world.get::<TooltipPanel>(*child).is_some())?;
        if world.get::<Node>(panel)?.display == Display::None {
            return None;
        }
        world
            .get::<Children>(panel)?
            .iter()
            .find_map(|line| world.get::<Text>(line).map(|text| text.0.clone()))
    }

    /// The first entity called `name`.
    fn named_entity(world: &World, name: &str) -> Option<Entity> {
        world
            .try_query::<(Entity, &Name)>()?
            .iter(world)
            .find(|(_, named)| named.as_str() == name)
            .map(|(entity, _)| entity)
    }

    /// The text of the header's `Gallery Page` line.
    fn gallery_page_text(world: &World) -> Option<String> {
        world
            .get::<Text>(named_entity(world, "Gallery Page")?)
            .map(|text| text.0.clone())
    }

    /// The muted card's Place took nothing: the hand is empty and the card
    /// still up. `gesture` names what was tried, for the failure.
    fn muted_card_stays(gesture: &'static str) -> impl Fn(&mut World) + Send + Sync {
        move |world: &mut World| {
            let held = world.resource::<HeldPart>().id().map(str::to_string);
            let state = world.resource::<GalleryState>().clone();
            info!(
                "station sandbox: {gesture} on the muted card: hand {held:?}, open {}, focused {}",
                state.open, state.focused
            );
            assert_eq!(
                held, None,
                "{gesture} on the muted card leaves the hand empty"
            );
            assert!(
                state.open && state.focused,
                "{gesture} on the muted card keeps the card up"
            );
        }
    }

    /// The hold the buy and short cards read the twin under: one recipe
    /// item and its count in the player's hold and in the station's stock.
    /// The twin needs two of each.
    fn card_hold(card: &str) -> Option<(ItemDesignId, u32, u32)> {
        match card {
            // One barrel from the hold and one bought.
            "buy" => Some(("TurretBarrel".into(), 1, 1)),
            // One panel from the hold and one short: the twin cannot be placed.
            "short" => Some(("SectionPanel".into(), 1, 0)),
            _ => None,
        }
    }

    /// The item and its counts in the player's hold and the station's stock
    /// before a card's [`card_hold`], put back after the card.
    #[derive(Resource)]
    struct CardHoldBefore(ItemDesignId, u32, u32);

    /// How many parts of the current category [`Gallery Stock Available`]
    /// narrowed from, read before the click.
    #[derive(Resource)]
    struct CatalogueStockBefore(usize);

    /// The muted card's part id, read once: [`card_part`] needs `&mut
    /// World`, which a step's `until` predicate never gets.
    #[derive(Resource)]
    struct SmokeMutedTile(String);

    /// Set `item`'s count in the player's hold and in the station's stock,
    /// and return the counts it replaced.
    fn set_hold_counts(
        world: &mut World,
        item: &ItemDesignId,
        player_count: u32,
        station_count: u32,
    ) -> (u32, u32) {
        let items = world.resource::<GameItems>().clone();
        let player = the_player(world).expect("the player ship stands");
        let station = station_root(world).expect("the sandbox station stands");
        let mut before = Vec::new();
        for (ship, count) in [(player, player_count), (station, station_count)] {
            let mut hold = world
                .get_mut::<ShipInventory>(ship)
                .expect("the player ship and the station carry a hold");
            let held = hold.count(item);
            if held > count {
                hold.remove(item, held - count);
            } else if count > held {
                hold.add(&items, item, count - held);
            }
            before.push(held);
        }
        (before[0], before[1])
    }

    /// Open the focus card of `card`'s part, as a tile click under its
    /// category does: the page that holds the tile depends on the shape.
    fn focus_the_card(card: &'static str) -> impl Fn(&mut World) + Send + Sync {
        move |world: &mut World| {
            let part = card_part(world, card);
            let listing = listing(world);
            let sections = world.resource::<GameSections>();
            let category = GalleryCategory::of(
                &sections
                    .get_section(&part)
                    .expect("a listed part is in the catalog")
                    .kind,
            );
            let index = sections
                .iter()
                .position(|section| section.base.id == part)
                .expect("a listed part is in the catalog");
            let selected = browsable(sections, Some(&listing), category, "", false)
                .iter()
                .position(|listed| *listed == index)
                .expect("the Catalogue lists every part");
            *world.resource_mut::<GalleryState>() = GalleryState {
                open: true,
                category,
                filter: String::new(),
                filter_focused: false,
                available_only: false,
                selected,
                focused: true,
            };
        }
    }

    /// Put `card`'s part's grid tile on the shown page: the grid and the
    /// focus card are mutually exclusive, so a card's tile only exists while
    /// no card is focused.
    fn show_the_grid_tile(card: &'static str) -> impl Fn(&mut World) + Send + Sync {
        move |world: &mut World| {
            let part = card_part(world, card);
            let listing = listing(world);
            let sections = world.resource::<GameSections>();
            let category = GalleryCategory::of(
                &sections
                    .get_section(&part)
                    .expect("a listed part is in the catalog")
                    .kind,
            );
            let index = sections
                .iter()
                .position(|section| section.base.id == part)
                .expect("a listed part is in the catalog");
            let selected = browsable(sections, Some(&listing), category, "", false)
                .iter()
                .position(|listed| *listed == index)
                .expect("the Catalogue lists every part");
            *world.resource_mut::<GalleryState>() = GalleryState {
                open: true,
                category,
                filter: String::new(),
                filter_focused: false,
                available_only: false,
                selected,
                focused: false,
            };
        }
    }

    /// Every text under `node` but its tooltip panel's, in tree order.
    fn texts_under(world: &World, node: Entity) -> Vec<String> {
        let mut texts = Vec::new();
        if world.get::<TooltipPanel>(node).is_some() {
            return texts;
        }
        if let Some(text) = world.get::<Text>(node) {
            texts.push(text.0.clone());
        }
        for child in world.get::<Children>(node).into_iter().flatten() {
            texts.extend(texts_under(world, *child));
        }
        texts
    }

    /// The recipe row tooltip for `item`, as [`recipe_row`] in `nova_ship`
    /// builds it: the count required, then the hold, the bought cost and the
    /// missing count, each only when it applies. `None` when the host does
    /// not sell the item.
    fn recipe_row_tooltip(item: &GalleryRecipeItem) -> Option<String> {
        let supply = item.supply?;
        let mut lines = vec![format!("Required {}", item.count)];
        if supply.held() > 0 {
            lines.push(format!("From the hold {}", supply.held()));
        }
        if supply.bought() > 0 {
            lines.push(format!(
                "Bought {} at {}, {}",
                supply.bought(),
                lot_cr_text(supply.price().ask_cr, supply.price().lot),
                cr_text(supply.total_cr())
            ));
        }
        if supply.short() > 0 {
            lines.push(format!("Missing {}", supply.short()));
        }
        Some(lines.join("\n"))
    }

    /// Each recipe row on the card reads `[covered/count, name]` for an item
    /// the station sells (`count` alone for one it does not), then, only
    /// when the row bought an item, the bought total as a third text. Its
    /// tooltip names the split exactly when the station sells the item,
    /// `None` otherwise. `rows` are the card's `Gallery Recipe Line` nodes.
    fn read_the_rows(
        world: &World,
        shape: &str,
        part: &str,
        entry: &GalleryEntry,
        rows: &[Entity],
    ) {
        for (row, item) in rows.iter().zip(&entry.recipe) {
            let texts = texts_under(world, *row);
            let tooltip = world.get::<Tooltip>(*row).map(|tooltip| tooltip.0.clone());
            info!("station sandbox: {shape} {part} recipe row {texts:?}, tooltip {tooltip:?}");
            let quantity = match item.supply {
                Some(supply) => format!("{}/{}", supply.covered(), item.count),
                None => item.count.to_string(),
            };
            let mut want = vec![quantity, item.name.clone()];
            if let Some(supply) = item.supply.filter(|supply| supply.bought() > 0) {
                want.push(cr_text(supply.total_cr()));
            }
            assert_eq!(
                texts, want,
                "at {shape} the {part} {} row reads {want:?}: {texts:?}",
                item.name
            );
            assert_eq!(
                tooltip,
                recipe_row_tooltip(item),
                "at {shape} the {part} {} row's tooltip names its split exactly when \
                 the station sells it",
                item.name
            );
        }
    }

    /// The focus card keeps Place, Swap and Back inside it at `shape`, a
    /// muted part's Place is off, and the clipping details column shows
    /// every recipe row whole, each with its marks and price. Logs every rect
    /// and the entry.
    fn read_the_card(shape: String, card: &'static str) -> impl Fn(&mut World) + Send + Sync {
        move |world: &mut World| {
            let part = card_part(world, card);
            let entry = listing(world)
                .entry(&part)
                .cloned()
                .expect("the Catalogue lists every part");
            // The twin card exists to hold the longest recipe and a Swap button;
            // a muted twin would read only Place and Back and still pass.
            if card == "twin" {
                assert!(
                    entry.placeable && entry.alternate.is_some(),
                    "at {shape} the twin card offers Place and Swap: {entry:?}"
                );
            }
            let supplies: Vec<_> = entry.recipe.iter().map(|item| item.supply).collect();
            match card {
                "buy" => assert!(
                    entry.placeable
                        && supplies
                            .iter()
                            .any(|supply| supply.is_some_and(|supply| supply.bought() > 0)),
                    "at {shape} the buy card can be placed and buys an item: {entry:?}"
                ),
                "short" => assert!(
                    !entry.placeable
                        && supplies
                            .iter()
                            .any(|supply| supply.is_some_and(|supply| supply.short() > 0)),
                    "at {shape} the short card cannot be placed and is short of an item: \
                     {entry:?}"
                ),
                "muted" => assert!(
                    supplies.iter().all(Option::is_none),
                    "at {shape} the muted card's rows carry no supply: {entry:?}"
                ),
                _ => {}
            }
            let card_rect =
                ui_node_rect(world, "Gallery Focus Card").expect("the focus card is laid out");
            let mut controls = vec!["Gallery Place Button", "Gallery Back Button"];
            if entry.alternate.is_some() {
                controls.push("Gallery Alternate Button");
            }
            for name in controls {
                let rect = ui_node_rect(world, name)
                    .unwrap_or_else(|| panic!("at {shape} the {part} card draws `{name}`"));
                let inside = rect.min.x >= card_rect.min.x - CARD_SLACK_PX
                    && rect.min.y >= card_rect.min.y - CARD_SLACK_PX
                    && rect.max.x <= card_rect.max.x + CARD_SLACK_PX
                    && rect.max.y <= card_rect.max.y + CARD_SLACK_PX;
                info!("station sandbox: {shape} {part} `{name}` {rect:?} in card {card_rect:?}");
                assert!(
                    inside,
                    "at {shape} the {part} card's `{name}` {rect:?} leaves the card {card_rect:?}"
                );
            }
            // Only at the stock shape: the header stays this wide at every
            // theme, so one shape is enough to catch a stock button that
            // wrapped off the header.
            if shape.starts_with("640x600") {
                let header_rect =
                    ui_node_rect(world, "Gallery Header").expect("the header is laid out");
                for name in ["Gallery Stock All", "Gallery Stock Available"] {
                    let rect = ui_node_rect(world, name)
                        .unwrap_or_else(|| panic!("at {shape} the header draws `{name}`"));
                    let inside = rect.min.x >= header_rect.min.x - CARD_SLACK_PX
                        && rect.min.y >= header_rect.min.y - CARD_SLACK_PX
                        && rect.max.x <= header_rect.max.x + CARD_SLACK_PX
                        && rect.max.y <= header_rect.max.y + CARD_SLACK_PX;
                    info!("station sandbox: {shape} `{name}` {rect:?} in header {header_rect:?}");
                    assert!(
                        inside,
                        "at {shape} the header's `{name}` {rect:?} leaves the header \
                         {header_rect:?}"
                    );
                }
            }
            let disabled = world
                .query::<(&Name, Has<bevy::ui::InteractionDisabled>)>()
                .iter(world)
                .find(|(name, _)| name.as_str() == "Gallery Place Button")
                .map(|(_, disabled)| disabled)
                .expect("the focus card draws Place");
            assert_eq!(
                disabled, !entry.placeable,
                "at {shape} the {part} card's Place is off exactly when the part cannot be placed"
            );
            let details =
                ui_node_rect(world, "Gallery Focus Details").expect("the focus card has details");
            // In the card's order, which is the recipe's.
            let rows: Vec<Entity> = named_entity(world, "Gallery Recipe")
                .and_then(|recipe| world.get::<Children>(recipe))
                .map(|rows| rows.to_vec())
                .unwrap_or_default();
            let lines: Vec<(Rect, f32)> = rows
                .iter()
                .map(|row| {
                    let rect = node_rect(world, *row).expect("a recipe row is laid out");
                    let shown =
                        (rect.max.y.min(details.max.y) - rect.min.y.max(details.min.y)).max(0.0);
                    (rect, shown)
                })
                .collect();
            let whole = lines
                .iter()
                .filter(|(rect, shown)| *shown >= rect.height() - CARD_SLACK_PX)
                .count();
            // A muted part's note is only its Place tooltip, and its tile
            // line is gone with it.
            let note = ui_node_rect(world, "Gallery Note");
            assert_eq!(
                note.is_some(),
                entry.placeable,
                "at {shape} the {part} card draws its note line exactly when it can be placed"
            );
            assert_eq!(
                entry.tile.is_some(),
                entry.placeable,
                "at {shape} the {part} tile has a line exactly when it can be placed"
            );
            let tooltip = world
                .query::<(&Name, &Tooltip)>()
                .iter(world)
                .find(|(name, _)| name.as_str() == "Gallery Place Button")
                .map(|(_, tooltip)| tooltip.0.clone());
            assert_eq!(
                tooltip,
                (!entry.placeable).then(|| entry.note.clone()),
                "at {shape} the {part} card's Place says why exactly when it is off"
            );
            info!(
                "station sandbox: {shape} {part} card: placeable {}, tile {:?}, note {:?} \
                 {note:?}, alternate {:?}, recipe {:?}, details {details:?}, recipe lines {} \
                 of {} whole, shown px {:?}",
                entry.placeable,
                entry.tile,
                entry.note,
                entry.alternate,
                entry.recipe,
                whole,
                lines.len(),
                lines.iter().map(|(_, shown)| *shown).collect::<Vec<_>>()
            );
            assert_eq!(
                lines.len(),
                entry.recipe.len(),
                "at {shape} the {part} card draws one line per recipe item"
            );
            // The twin's four items are the most any base part's recipe has.
            if card == "twin" {
                assert_eq!(
                    lines.len(),
                    4,
                    "at {shape} the twin card lists its four recipe items"
                );
            }
            assert_eq!(
                whole,
                lines.len(),
                "at {shape} the {part} card shows every recipe line whole"
            );
            read_the_rows(world, &shape, &part, &entry, &rows);
        }
    }

    /// Before the first card is focused, at the 640x600 shape in the first
    /// theme: the muted card's tile carries its note as a `Tooltip`, and
    /// shows it on hover, which is shot; the twin's placeable tile carries
    /// none. The pointer then rests on the grid's title, which has no
    /// tooltip, so the twin card's own shot shows none left over. Tiles only
    /// exist while no card is focused, so this is the walk's one window on
    /// them.
    fn hover_the_muted_tile(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        let shot = "sandbox-tile-muted-tooltip-640.png";
        script
            .step("station sandbox: show the muted tile")
            .on_enter(|world: &mut World| {
                let part = card_part(world, "muted");
                world.insert_resource(SmokeMutedTile(part));
                show_the_grid_tile("muted")(world);
            })
            .until(Arc::new(|world: &World| {
                let part = &world.resource::<SmokeMutedTile>().0;
                ui_node_present(part_tile(world, part))(world)
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: the muted tile carries its note as a tooltip")
            .on_enter(|world: &mut World| {
                let part = world.resource::<SmokeMutedTile>().0.clone();
                let entry = listing(world)
                    .entry(&part)
                    .cloned()
                    .expect("the Catalogue lists every part");
                let tile = named_entity(world, &part_tile(world, &part))
                    .expect("the muted tile is laid out");
                let carried = world.get::<Tooltip>(tile).map(|tooltip| tooltip.0.clone());
                info!("station sandbox: the muted tile carries {carried:?}");
                assert_eq!(
                    carried,
                    Some(entry.note.clone()),
                    "the muted tile's Tooltip is its note"
                );
            })
            .add()
            .step("station sandbox: point at the muted tile")
            .each(|world, _, _| {
                let part = world.resource::<SmokeMutedTile>().0.clone();
                keep_hovering_named(part_tile(world, &part))(world);
            })
            .until(Arc::new(|world: &World| {
                let part = &world.resource::<SmokeMutedTile>().0;
                named_entity(world, &part_tile(world, part))
                    .is_some_and(|tile| shown_tooltip(world, tile).is_some())
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: read the muted tile's shown tooltip")
            .on_enter(|world: &mut World| {
                let part = world.resource::<SmokeMutedTile>().0.clone();
                let entry = listing(world)
                    .entry(&part)
                    .cloned()
                    .expect("the Catalogue lists every part");
                let tile = named_entity(world, &part_tile(world, &part))
                    .expect("the muted tile is laid out");
                let shown = shown_tooltip(world, tile);
                info!("station sandbox: the muted tile's shown tooltip reads {shown:?}");
                assert_eq!(
                    shown.as_deref(),
                    Some(entry.note.as_str()),
                    "the hovered muted tile shows its note"
                );
            })
            .add()
            .step("station sandbox: shoot the muted tile's tooltip")
            .on_enter(move |world: &mut World| shoot(world, shot))
            .until(shot_written(shot))
            .deadline(SHOT_SECS)
            .add()
            .step("station sandbox: show the twin's tile")
            .on_enter(|world: &mut World| {
                world.remove_resource::<SmokeMutedTile>();
                show_the_grid_tile("twin")(world);
            })
            .until(Arc::new(|world: &World| {
                ui_node_present(part_tile(world, PDC_TWIN_KINETIC_TURRET_SECTION_ID))(world)
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: the twin's placeable tile carries no tooltip")
            .on_enter(|world: &mut World| {
                let tile =
                    named_entity(world, &part_tile(world, PDC_TWIN_KINETIC_TURRET_SECTION_ID))
                        .expect("the twin's tile is laid out");
                let carried = world.get::<Tooltip>(tile).map(|tooltip| tooltip.0.clone());
                info!("station sandbox: the twin's tile carries {carried:?}");
                assert_eq!(carried, None, "a placeable tile carries no Tooltip");
            })
            .add()
            .step("station sandbox: rest the pointer after the muted tile")
            .on_enter(|world: &mut World| {
                let title = ui_node_rect(world, "Gallery Title").expect("the grid has a title");
                move_cursor(title.center())(world);
            })
            .until(frames(2))
            .add()
    }

    /// After the docked port: cycle to a section whose delete would split the
    /// ship and check the verdict names the sections that would lose the dock
    /// (live sections of the player ship, not the selection) under a muted
    /// Delete with the split reason as its tooltip. With it selected, in each
    /// theme at each of [`CARD_SIZES`] hover Delete until the tooltip shows
    /// and shoot it. Then cycle on to a section that may come off and shoot
    /// its enabled Delete, unhovered, in each theme at the stock shape, for
    /// comparison. Last, put the theme and the capture shape back. The red
    /// fill and outline on the severed blocks are read on the PNGs.
    fn read_the_split(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        let mut script = script
            .step("station sandbox: cycle to a delete that would split the ship")
            .each(tap_while("viewer_next", |world| {
                world.resource::<ShipRuntime>().delete_severed().is_empty()
            }))
            .until(Arc::new(|world: &World| {
                !world.resource::<ShipRuntime>().delete_severed().is_empty()
            }))
            .diagnose(|world| {
                format!(
                    "selected {:?}, verdict {:?}",
                    selected_code(world),
                    world.resource::<ShipRuntime>().delete_verdict()
                )
            })
            .deadline(SETUP_SECS)
            .add()
            .step("station sandbox: let the select key up after the split")
            .on_enter(release_action("viewer_next"))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: the split names the sections that would lose the dock")
            .on_enter(|world: &mut World| {
                let runtime = world.resource::<ShipRuntime>();
                let split = runtime.selected().expect("a section is selected");
                let severed = runtime.delete_severed().to_vec();
                let player = player_root(world).expect("the player ship is live");
                assert!(!severed.is_empty(), "the split names what would come off");
                assert!(
                    !severed.contains(&split),
                    "the deleted section itself is not severed"
                );
                for section in &severed {
                    assert!(
                        world.get::<SectionMarker>(*section).is_some()
                            && world
                                .get::<ChildOf>(*section)
                                .is_some_and(|parent| parent.parent() == player),
                        "{section:?} is a live section of the player ship"
                    );
                }
                let codes: Vec<String> = severed
                    .iter()
                    .filter_map(|section| world.get::<SectionCode>(*section))
                    .map(|code| code.0.clone())
                    .collect();
                info!(
                    "station sandbox: deleting {:?} would cut {codes:?} off the dock",
                    selected_code(world)
                );
                assert_eq!(
                    delete_button(world),
                    Some(("Delete".to_string(), false)),
                    "a split mutes Delete"
                );
                assert_eq!(
                    delete_tooltip_text(world).as_deref(),
                    Some(SPLIT_REASON),
                    "the split reason is the muted Delete's tooltip"
                );
                world.insert_resource(SmokeSplit(split));
            })
            .add();
        for (theme, theme_name) in CARD_THEMES {
            script = switch_theme(script, theme, theme_name, "the split");
            for (width, height) in CARD_SIZES {
                let shape = format!("{width}x{height} {theme_name}");
                let shot = format!("sandbox-delete-split-{theme_name}-{width}.png");
                script = script
                    .step(format!("station sandbox: reshape to {shape} for the split"))
                    .on_enter(move |world: &mut World| set_size(world, width, height))
                    .until(window_size_is(width, height))
                    .deadline(BEAT_SECS)
                    .add()
                    .step(format!(
                        "station sandbox: point at the split's muted Delete at {shape}"
                    ))
                    .each(|world, _, _| keep_hovering_named(DELETE_BUTTON)(world))
                    .until(Arc::new(|world: &World| delete_tooltip(world).is_some()))
                    .diagnose(|world| {
                        format!(
                            "selected {:?}, tooltip {:?}, shown {:?}",
                            selected_code(world),
                            delete_tooltip_text(world),
                            delete_tooltip(world)
                        )
                    })
                    .deadline(BEAT_SECS)
                    .add()
                    .step(format!(
                        "station sandbox: read the split's tooltip at {shape}"
                    ))
                    .on_enter(|world: &mut World| {
                        let shown = delete_tooltip(world);
                        info!("station sandbox: the split's Delete tooltip reads {shown:?}");
                        assert_eq!(
                            shown.as_deref(),
                            Some(SPLIT_REASON),
                            "the hovered muted Delete shows the split reason"
                        );
                        let runtime = world.resource::<ShipRuntime>();
                        assert_eq!(
                            runtime.selected(),
                            Some(world.resource::<SmokeSplit>().0),
                            "the reshape and the theme keep the split selected"
                        );
                        assert!(
                            !runtime.delete_severed().is_empty(),
                            "the severed sections stay named while the split is selected"
                        );
                    })
                    .add()
                    .step(format!("station sandbox: settle the split at {shape}"))
                    .until(frames(SETTLE_FRAMES))
                    .add()
                    .step(format!("station sandbox: shoot the split at {shape}"))
                    .on_enter({
                        let shot = shot.clone();
                        move |world: &mut World| shoot(world, &shot)
                    })
                    .until(shot_written(shot))
                    .deadline(SHOT_SECS)
                    .add();
            }
        }
        script = script
            .step("station sandbox: cycle to a delete that holds")
            .each(tap_while("viewer_next", |world| {
                !world
                    .resource::<ShipRuntime>()
                    .delete_verdict()
                    .is_some_and(|verdict| verdict.is_ok())
            }))
            .until(Arc::new(|world: &World| {
                world
                    .resource::<ShipRuntime>()
                    .delete_verdict()
                    .is_some_and(|verdict| verdict.is_ok())
            }))
            .diagnose(|world| {
                format!(
                    "selected {:?}, verdict {:?}",
                    selected_code(world),
                    world.resource::<ShipRuntime>().delete_verdict()
                )
            })
            .deadline(SETUP_SECS)
            .add()
            .step("station sandbox: let the select key up after the deletable")
            .on_enter(|world: &mut World| {
                release_action("viewer_next")(world);
                info!(
                    "station sandbox: {:?} may come off, for the enabled comparison",
                    selected_code(world)
                );
                // Off the button, so the enabled face is shot unhovered.
                let view = ui_node_rect(world, SHIP_VIEWPORT).expect("the Ship view is laid out");
                move_cursor(view.min + Vec2::splat(EMPTY_INSET_PX))(world);
            })
            .until(Arc::new(|world: &World| {
                delete_button(world).is_some_and(|(_, enabled)| enabled)
                    && delete_tooltip(world).is_none()
            }))
            .diagnose(|world| {
                format!(
                    "selected {:?}, button {:?}, shown {:?}",
                    selected_code(world),
                    delete_button(world),
                    delete_tooltip(world)
                )
            })
            .deadline(BEAT_SECS)
            .add();
        // The window is at the last of CARD_SIZES, the stock shape; the last
        // theme is live, so it is shot first.
        for (theme, theme_name) in CARD_THEMES.into_iter().rev() {
            let shot = format!("sandbox-delete-enabled-{theme_name}.png");
            script = switch_theme(script, theme, theme_name, "the enabled Delete")
                .step(format!(
                    "station sandbox: shoot the enabled Delete in {theme_name}"
                ))
                .on_enter({
                    let shot = shot.clone();
                    move |world: &mut World| shoot(world, &shot)
                })
                .until(shot_written(shot))
                .deadline(SHOT_SECS)
                .add();
        }
        let (width, height) = CAPTURE_RESOLUTION;
        let (width, height) = (width as f32, height as f32);
        script
            .step("station sandbox: back to the capture shape and theme after the split")
            .on_enter(move |world: &mut World| {
                *world.resource_mut::<SelectedUiTheme>() = SelectedUiTheme::default();
                set_size(world, width, height);
            })
            .until(window_size_is(width, height))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: settle the capture shape after the split")
            .until(frames(SETTLE_FRAMES))
            .add()
    }

    /// Switch to `theme` and let it re-tint, for the `what` shots.
    fn switch_theme(
        script: AutopilotPlugin<GameStates>,
        theme: &'static str,
        theme_name: &str,
        what: &str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!(
                "station sandbox: switch to the {theme_name} theme for {what}"
            ))
            .on_enter(move |world: &mut World| {
                world.resource_mut::<SelectedUiTheme>().0 = theme.to_string();
            })
            .until(Arc::new(move |world: &World| {
                world.resource::<ActiveUiTheme>().id() == theme
            }))
            .deadline(BEAT_SECS)
            .add()
            .step(format!(
                "station sandbox: settle the {theme_name} theme for {what}"
            ))
            .until(frames(THEME_FRAMES))
            .add()
    }

    /// With the rebound PDC selected, read two focus cards in each theme at
    /// each of [`CARD_SIZES`], and shoot each; then close the Catalogue and
    /// put the theme and the capture shape back for the shots after.
    fn read_the_cards(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        let mut script = script
            .click_named(
                "station sandbox: open the Catalogue for the cards",
                CATALOGUE_BUTTON,
                ui_node_present("Gallery Grid"),
                BEAT_SECS,
            )
            .step("station sandbox: count the muted tiles")
            .on_enter(|world: &mut World| {
                let builds = station_builds(world);
                let listing = listing(world);
                let muted = listing.0.iter().filter(|entry| !entry.placeable).count();
                let not_built = listing
                    .0
                    .iter()
                    .filter(|entry| !builds.contains(&entry.id))
                    .count();
                info!(
                    "station sandbox: the Catalogue lists {} parts, {muted} muted, {not_built} \
                     not built here",
                    listing.0.len()
                );
                let category = world.resource::<GalleryState>().category;
                let sections = world.resource::<GameSections>();
                let full = browsable(sections, Some(&listing), category, "", false).len();
                world.insert_resource(CatalogueStockBefore(full));
            })
            .until(frames(1))
            .add()
            .click_named(
                "station sandbox: keep only the available parts",
                "Gallery Stock Available",
                Arc::new(|world: &World| world.resource::<GalleryState>().available_only),
                BEAT_SECS,
            )
            .step("station sandbox: Available keeps only the placeable listed parts")
            .on_enter(|world: &mut World| {
                let listing = listing(world);
                let category = world.resource::<GalleryState>().category;
                let sections = world.resource::<GameSections>();
                let narrowed = browsable(sections, Some(&listing), category, "", true);
                let placeable = listing
                    .0
                    .iter()
                    .filter(|entry| {
                        entry.placeable
                            && sections
                                .get_section(&entry.id)
                                .is_some_and(|section| category.accepts(&section.kind))
                    })
                    .count();
                let stocked =
                    browsable(sections, Some(&listing), GalleryCategory::All, "", false).len();
                let page = gallery_page_text(world);
                info!(
                    "station sandbox: Available lists {} of {stocked} parts, page {page:?}",
                    narrowed.len()
                );
                assert_eq!(
                    narrowed.len(),
                    placeable,
                    "Available keeps only the placeable listed parts of this category"
                );
                assert!(
                    page.is_some_and(|page| {
                        page.starts_with(&format!("{} of {stocked} parts", narrowed.len()))
                    }),
                    "the Gallery Page line opens with the narrowed count"
                );
            })
            .add()
            .step("station sandbox: shoot the available stock")
            .on_enter(|world: &mut World| shoot(world, "sandbox-catalogue-available.png"))
            .until(shot_written("sandbox-catalogue-available.png"))
            .deadline(SHOT_SECS)
            .add()
            .click_named(
                "station sandbox: show every stocked part again",
                "Gallery Stock All",
                Arc::new(|world: &World| !world.resource::<GalleryState>().available_only),
                BEAT_SECS,
            )
            .step("station sandbox: All brings every listed part of this category back")
            .on_enter(|world: &mut World| {
                let before = world
                    .remove_resource::<CatalogueStockBefore>()
                    .expect("the stock was counted before Available")
                    .0;
                let listing = listing(world);
                let category = world.resource::<GalleryState>().category;
                let sections = world.resource::<GameSections>();
                let listed = browsable(sections, Some(&listing), category, "", false).len();
                info!("station sandbox: All lists {listed} parts, {before} before Available");
                assert_eq!(
                    listed, before,
                    "All brings back every listed part Available narrowed"
                );
            })
            .add();
        for (theme, theme_name) in CARD_THEMES {
            script = script
                .step(format!("station sandbox: switch to the {theme_name} theme"))
                .on_enter(move |world: &mut World| {
                    world.resource_mut::<SelectedUiTheme>().0 = theme.to_string();
                })
                .until(Arc::new(move |world: &World| {
                    world.resource::<ActiveUiTheme>().id() == theme
                }))
                .deadline(BEAT_SECS)
                .add()
                .step(format!("station sandbox: settle the {theme_name} theme"))
                .until(frames(THEME_FRAMES))
                .add();
            for (width, height) in CARD_SIZES {
                let shape = format!("{width}x{height} {theme_name}");
                script = script
                    .step(format!("station sandbox: reshape to {shape}"))
                    .on_enter(move |world: &mut World| set_size(world, width, height))
                    .until(window_size_is(width, height))
                    .deadline(BEAT_SECS)
                    .add();
                if theme_name == CARD_THEMES[0].1 && width == CARD_SIZES[0].0 {
                    script = hover_the_muted_tile(script);
                }
                for card in ["twin", "buy", "short", "muted"] {
                    let shot = format!("sandbox-card-{card}-{theme_name}-{width}.png");
                    script = script
                        .step(format!("station sandbox: focus the {card} card at {shape}"))
                        .on_enter(move |world: &mut World| {
                            if let Some((item, player, station)) = card_hold(card) {
                                let (player, station) =
                                    set_hold_counts(world, &item, player, station);
                                world.insert_resource(CardHoldBefore(item, player, station));
                            }
                            focus_the_card(card)(world);
                        })
                        .until(ui_node_present("Gallery Back Button"))
                        .deadline(BEAT_SECS)
                        .add()
                        .step(format!(
                            "station sandbox: settle the {card} card at {shape}"
                        ))
                        .until(frames(SETTLE_FRAMES))
                        .add()
                        .step(format!("station sandbox: read the {card} card at {shape}"))
                        .on_enter(read_the_card(shape.clone(), card))
                        .add()
                        .step(format!("station sandbox: shoot the {card} card at {shape}"))
                        .on_enter({
                            let shot = shot.clone();
                            move |world: &mut World| shoot(world, &shot)
                        })
                        .until(shot_written(shot))
                        .deadline(SHOT_SECS)
                        .add();
                    if card == "buy" {
                        script = point_at_the_bought_row(script, &shape, theme_name, width);
                    }
                    if card_hold(card).is_some() {
                        script = script
                            .step(format!(
                                "station sandbox: put the hold back after the {card} card at \
                                 {shape}"
                            ))
                            .on_enter(|world: &mut World| {
                                let CardHoldBefore(item, player, station) = world
                                    .remove_resource::<CardHoldBefore>()
                                    .expect("the card changed the hold");
                                set_hold_counts(world, &item, player, station);
                            })
                            .until(frames(2))
                            .add();
                    }
                    if card == "muted" {
                        script = try_the_muted_place(script, &shape, theme_name, width);
                    }
                }
            }
        }
        let (width, height) = CAPTURE_RESOLUTION;
        let (width, height) = (width as f32, height as f32);
        script
            .click_named(
                "station sandbox: close the Catalogue after the cards",
                "Gallery Close Button",
                Arc::new(|world: &World| !world.resource::<GalleryState>().open),
                BEAT_SECS,
            )
            .step("station sandbox: back to the capture shape and theme")
            .on_enter(move |world: &mut World| {
                *world.resource_mut::<SelectedUiTheme>() = SelectedUiTheme::default();
                set_size(world, width, height);
            })
            .until(window_size_is(width, height))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: settle the capture shape")
            .until(frames(SETTLE_FRAMES))
            .add()
    }

    /// The first recipe row on the card that prices a bought item: the row
    /// carrying three texts, the quantity, the name and the bought total.
    fn bought_row(world: &World) -> Option<Entity> {
        let recipe = named_entity(world, "Gallery Recipe")?;
        world
            .get::<Children>(recipe)?
            .iter()
            .find(|row| texts_under(world, *row).len() == 3)
    }

    /// `node`'s laid-out rect, in logical pixels.
    fn node_rect(world: &World, node: Entity) -> Option<Rect> {
        let transform = world.get::<UiGlobalTransform>(node)?;
        let computed = world.get::<ComputedNode>(node)?;
        let scale = computed.inverse_scale_factor();
        Some(Rect::from_center_size(
            transform.translation * scale,
            computed.size() * scale,
        ))
    }

    /// On the focused buy card: the pointer on the bought row shows its
    /// split as its tooltip, which is shot. The pointer then rests on the
    /// card title, which has no tooltip, so the next card's shot shows none.
    fn point_at_the_bought_row(
        script: AutopilotPlugin<GameStates>,
        shape: &str,
        theme_name: &str,
        width: f32,
    ) -> AutopilotPlugin<GameStates> {
        let shot = format!("sandbox-card-buy-tooltip-{theme_name}-{width}.png");
        script
            .step(format!(
                "station sandbox: point at the bought row at {shape}"
            ))
            .on_enter(|world: &mut World| {
                let row = bought_row(world).expect("the buy card marks a bought row");
                let rect = node_rect(world, row).expect("the bought row is laid out");
                move_cursor(rect.center())(world);
            })
            .until(Arc::new(|world: &World| {
                bought_row(world).is_some_and(|row| shown_tooltip(world, row).is_some())
            }))
            .deadline(BEAT_SECS)
            .add()
            .step(format!(
                "station sandbox: read the bought row's tooltip at {shape}"
            ))
            .on_enter(|world: &mut World| {
                let part = card_part(world, "buy");
                let entry = listing(world)
                    .entry(&part)
                    .cloned()
                    .expect("the Catalogue lists every part");
                let item = entry
                    .recipe
                    .iter()
                    .find(|item| item.supply.is_some_and(|supply| supply.bought() > 0))
                    .expect("the buy card buys an item");
                let want = recipe_row_tooltip(item);
                let shown = bought_row(world).and_then(|row| shown_tooltip(world, row));
                info!("station sandbox: the bought row's tooltip reads {shown:?}");
                assert_eq!(shown, want, "the bought row's tooltip names its split");
            })
            .add()
            .step(format!(
                "station sandbox: shoot the bought row's tooltip at {shape}"
            ))
            .on_enter({
                let shot = shot.clone();
                move |world: &mut World| shoot(world, &shot)
            })
            .until(shot_written(shot))
            .deadline(SHOT_SECS)
            .add()
            .step(format!("station sandbox: rest the pointer at {shape}"))
            .on_enter(|world: &mut World| {
                let title =
                    ui_node_rect(world, "Gallery Focus Title").expect("the card has a title");
                move_cursor(title.center())(world);
            })
            .until(frames(2))
            .add()
    }

    /// On the focused muted card: Enter and a click on the disabled Place
    /// take nothing, and the pointer on Place shows the note as its tooltip,
    /// which is shot.
    fn try_the_muted_place(
        script: AutopilotPlugin<GameStates>,
        shape: &str,
        theme_name: &str,
        width: f32,
    ) -> AutopilotPlugin<GameStates> {
        let shot = format!("sandbox-card-muted-tooltip-{theme_name}-{width}.png");
        script
            .step(format!(
                "station sandbox: press Enter on the muted card at {shape}"
            ))
            .on_enter(press_key(KeyCode::Enter))
            .until(frames(2))
            .add()
            .step(format!("station sandbox: let Enter up at {shape}"))
            .on_enter(release_key(KeyCode::Enter))
            .until(frames(2))
            .add()
            .step(format!("station sandbox: Enter took nothing at {shape}"))
            .on_enter(muted_card_stays("Enter"))
            .add()
            .step(format!(
                "station sandbox: point at the muted Place at {shape}"
            ))
            .each(|world, _, _| keep_hovering_named("Gallery Place Button")(world))
            .until(Arc::new(|world: &World| place_tooltip(world).is_some()))
            .deadline(BEAT_SECS)
            .add()
            .step(format!(
                "station sandbox: read the muted Place tooltip at {shape}"
            ))
            .on_enter(|world: &mut World| {
                let part = card_part(world, "muted");
                let note = listing(world)
                    .entry(&part)
                    .expect("the Catalogue lists every part")
                    .note
                    .clone();
                let shown = place_tooltip(world);
                info!("station sandbox: the muted Place tooltip reads {shown:?}");
                assert_eq!(shown, Some(note), "the muted Place's tooltip is its note");
            })
            .add()
            .step(format!(
                "station sandbox: shoot the muted Place tooltip at {shape}"
            ))
            .on_enter({
                let shot = shot.clone();
                move |world: &mut World| shoot(world, &shot)
            })
            .until(shot_written(shot))
            .deadline(SHOT_SECS)
            .add()
            .step(format!("station sandbox: click the muted Place at {shape}"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(frames(2))
            .add()
            .step(format!(
                "station sandbox: let the muted Place up at {shape}"
            ))
            .on_enter(release_mouse(MouseButton::Left))
            .until(frames(2))
            .add()
            .step(format!(
                "station sandbox: the click took nothing at {shape}"
            ))
            .on_enter(muted_card_stays("a click on Place"))
            .add()
    }

    /// Reshape the window, the way dragging its corner does.
    fn set_size(world: &mut World, width: f32, height: f32) {
        world
            .query_filtered::<&mut Window, With<bevy::window::PrimaryWindow>>()
            .single_mut(world)
            .expect("one primary window")
            .resolution
            .set(width, height);
    }

    /// The smoke walk: the setup lands docked on the Ship tab, and a frame of
    /// it is shot.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("station sandbox: docked on the Ship tab")
            .until(ready_on_the_ship_tab())
            .diagnose(|world| format!("{:?}", world.get_resource::<Sandbox>()))
            .deadline(SETUP_SECS)
            .add();
        let script = berth_shots(script)
            .step("station sandbox: settle the schematic")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: shoot the Ship tab")
            .on_enter(|world: &mut World| shoot(world, SHIP_SHOT))
            .until(shot_written(SHIP_SHOT))
            .deadline(SHOT_SECS)
            .add();
        let script = take_the_part(
            script,
            PDC_KINETIC_TURRET_SECTION_ID,
            Some(CATALOGUE_ALL_SHOT),
            CATALOGUE_SHOT,
        );
        let script = script
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
            .step("station sandbox: roll the part a quarter turn")
            .on_enter(press_action("ship_part_roll"))
            .until(Arc::new(|world: &World| mount(world).roll == 1))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let the roll key up")
            .on_enter(release_action("ship_part_roll"))
            .until(frames(2))
            .add()
            // The kinetic PDC mounts by one socket, so F leaves it in place;
            // the frame after it shows the view did not tilt.
            .step("station sandbox: press the socket key")
            .on_enter(press_action("ship_part_socket"))
            .until(frames(3))
            .add()
            .step("station sandbox: let the socket key up")
            .on_enter(release_action("ship_part_socket"))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: shoot the rolled ghost")
            .on_enter(|world: &mut World| shoot(world, MOUNT_SHOT))
            .until(shot_written(MOUNT_SHOT))
            .deadline(SHOT_SECS)
            .diagnose(|world| format!("mount {:?}, aim {:?}", mount(world), ship_ghost_aim(world)))
            .add()
            .step("station sandbox: cycle the socket back to the drawn one")
            .each(tap_while("ship_part_socket", |world| {
                mount(world).source != 0
            }))
            .until(Arc::new(|world: &World| mount(world).source == 0))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: roll until the ghost fits")
            .each(tap_while("ship_part_roll", |world| !ghost_is_valid(world)))
            .until(Arc::new(ghost_is_valid))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let the pose keys up")
            .on_enter(|world: &mut World| {
                release_action("ship_part_roll")(world);
                release_action("ship_part_socket")(world);
            })
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: press on the ghost")
            .on_enter(|world: &mut World| {
                if let Some((eye, look)) = ship_view_pose(world) {
                    world.insert_resource(SmokePivot { eye, look });
                }
                press_mouse(MouseButton::Left)(world);
            })
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
            .step("station sandbox: the view eases onto the placed section")
            .until(Arc::new(|world: &World| {
                pivot_offsets(world).is_some_and(|(before, now)| now < before * PIVOT_EASED)
            }))
            .diagnose(|world| {
                format!(
                    "{:?}, offsets {:?}, placed {:?}",
                    world.get_resource::<SmokePivot>(),
                    pivot_offsets(world),
                    placed_position(world)
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: settle the placed section")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: report the pivot after the add")
            .on_enter(|world: &mut World| {
                // The ease moves the pivot only: the facing stays, and the
                // eye stands off the placed section at the orbit radius.
                let pivot = world.get_resource::<SmokePivot>().copied();
                let pose = ship_view_pose(world);
                let facing = pivot.zip(pose).map(|(pivot, (_, look))| pivot.look.dot(look));
                let reach = pose
                    .zip(placed_position(world))
                    .map(|((eye, _), placed)| eye.distance(placed));
                info!(
                    "station sandbox: pivot offsets {:?}, facing kept {facing:?}, eye to placed {reach:?}",
                    pivot_offsets(world)
                );
            })
            .add()
            .step("station sandbox: report the tabs after the add")
            .on_enter(report_the_tabs)
            .add()
            .step("station sandbox: the panel shows the part in hand")
            .until(Arc::new(|world: &World| {
                // The put-away cue sits in the footer, matched by text: the
                // hint component is private to nova_interface.
                let footer_shows_put_away = world
                    .try_query::<(&Text, &Node)>()
                    .is_some_and(|mut texts| {
                        texts.iter(world).any(|(text, node)| {
                            text.0 == "Q/Esc Put away" && node.display == Display::Flex
                        })
                    });
                footer_shows_put_away
                    && held_panel_lines(world).is_some_and(|lines| {
                        lines.first().is_some_and(|line| line == "Placing")
                            && lines.iter().any(|line| line.ends_with(" cr"))
                            && !lines.iter().any(|line| line.contains("put away"))
                    })
            }))
            .diagnose(|world| format!("placing block {:?}", held_panel_lines(world)))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: report the part in hand")
            .on_enter(|world: &mut World| {
                info!("station sandbox: placing block {:?}", held_panel_lines(world));
            })
            .add()
            .step("station sandbox: shoot the placed section")
            .on_enter(|world: &mut World| shoot(world, PLACED_SHOT))
            .until(shot_written(PLACED_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("station sandbox: Esc puts the part in hand away")
            .on_enter(press_key(KeyCode::Escape))
            .until(Arc::new(|world: &World| {
                world.resource::<HeldPart>().id().is_none()
                    && ship_ghost_aim(world).is_none()
                    && held_panel_lines(world).is_none()
            }))
            .diagnose(|world| {
                format!(
                    "held {:?}, aim {:?}",
                    world.resource::<HeldPart>().id(),
                    ship_ghost_aim(world)
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let Esc up")
            .on_enter(release_key(KeyCode::Escape))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: the Esc that emptied the hand left the Ship tab open")
            .until(the_interface_shows(InterfacePaneType::Ship))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: shoot the empty hand")
            .on_enter(|world: &mut World| {
                info!(
                    "station sandbox: Esc put the part away, held {:?}, placing block {:?}, pause {:?}",
                    world.resource::<HeldPart>().id(),
                    held_panel_lines(world),
                    world.get_resource::<State<PauseStates>>().map(State::get)
                );
                shoot(world, ESC_SHOT);
            })
            .until(shot_written(ESC_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("station sandbox: arm a rebind on the placed gun")
            .on_enter(press_action("ship_rebind"))
            .until(Arc::new(|world: &World| {
                world
                    .get_resource::<ShipRuntime>()
                    .is_some_and(ShipRuntime::rebind_armed)
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let the rebind key up")
            .on_enter(|world: &mut World| {
                release_action("ship_rebind")(world);
                let facing = ship_view_facing(world);
                let selected = world.resource::<ShipRuntime>().selected();
                world.insert_resource(SmokeArmed { facing, selected });
            })
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: shoot the armed rebind")
            .on_enter(|world: &mut World| shoot(world, REBIND_ARMED_SHOT))
            .until(shot_written(REBIND_ARMED_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("station sandbox: press the new key")
            .on_enter(press_key(REBIND_KEY))
            .until(Arc::new(rebound))
            .diagnose(|world| {
                format!(
                    "note {:?}",
                    world
                        .get_resource::<ShipRuntime>()
                        .and_then(|runtime| runtime.note().map(str::to_string))
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let the new key up")
            .on_enter(release_key(REBIND_KEY))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: the captured key kept the selection")
            .until(Arc::new(|world: &World| {
                world.get_resource::<SmokeArmed>().is_some_and(|armed| {
                    world.resource::<ShipRuntime>().selected() == armed.selected
                })
            }))
            .diagnose(|world| {
                format!(
                    "selected {:?}, armed {:?}",
                    world.resource::<ShipRuntime>().selected(),
                    world.get_resource::<SmokeArmed>()
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: shoot the rebound gun")
            .on_enter(|world: &mut World| shoot(world, REBIND_SHOT))
            .until(shot_written(REBIND_SHOT))
            .deadline(SHOT_SECS)
            .add();
        let script = read_the_cards(script)
            .step("station sandbox: the rebound gun can come off")
            .until(Arc::new(|world: &World| {
                world
                    .get_resource::<ShipRuntime>()
                    .and_then(ShipRuntime::delete_verdict)
                    .is_some_and(|verdict| verdict.is_ok())
            }))
            .diagnose(|world| {
                format!(
                    "verdict {:?}",
                    world
                        .get_resource::<ShipRuntime>()
                        .and_then(ShipRuntime::delete_verdict)
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: the Salvage block names the yield")
            .until(Arc::new(|world: &World| {
                delete_button(world).is_some_and(|(label, enabled)| enabled && label == "Delete")
                    && delete_tooltip_text(world).is_some_and(|reason| reason.is_empty())
                    && salvage_texts(world).is_some_and(|texts| {
                        texts.iter().any(|text| text == "+1")
                            && texts.iter().any(|text| text == "Section panel")
                    })
            }))
            .diagnose(|world| {
                format!(
                    "button {:?}, tooltip {:?}, salvage {:?}",
                    delete_button(world),
                    delete_tooltip_text(world),
                    salvage_texts(world)
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: settle the enabled Delete")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: shoot the enabled Delete")
            .on_enter(|world: &mut World| {
                info!(
                    "station sandbox: delete button {:?}, salvage {:?}",
                    delete_button(world),
                    salvage_texts(world)
                );
                shoot(world, DELETE_YIELD_SHOT);
            })
            .until(shot_written(DELETE_YIELD_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("station sandbox: delete the rebound gun")
            .on_enter(|world: &mut World| {
                let gun = world.resource::<ShipRuntime>().selected();
                let sections = player_sections(world);
                world.insert_resource(SmokeDelete { gun, sections });
                press_action("ship_delete")(world);
            })
            .until(Arc::new(|world: &World| {
                world.get_resource::<SmokeDelete>().is_some_and(|delete| {
                    player_sections(world) < delete.sections
                        && delete.gun.is_some_and(|gun| world.get_entity(gun).is_err())
                }) && world
                    .get_resource::<ShipRuntime>()
                    .and_then(ShipRuntime::note)
                    .is_some_and(|note| {
                        note.starts_with("deleted ") && note.contains(" to the hold:\n+")
                    })
            }))
            .diagnose(|world| {
                format!(
                    "{:?}, sections {}, note {:?}",
                    world.get_resource::<SmokeDelete>(),
                    player_sections(world),
                    world
                        .get_resource::<ShipRuntime>()
                        .and_then(|runtime| runtime.note().map(str::to_string))
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let the delete key up")
            .on_enter(|world: &mut World| {
                release_action("ship_delete")(world);
                let note = world
                    .get_resource::<ShipRuntime>()
                    .and_then(|runtime| runtime.note().map(str::to_string));
                info!("station sandbox: delete note {note:?}");
            })
            // Shot at once: the note line lasts 2.5 s.
            .until(frames(1))
            .add()
            .step("station sandbox: shoot the deleted gun")
            .on_enter(|world: &mut World| shoot(world, DELETE_SHOT))
            .until(shot_written(DELETE_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("station sandbox: select the port the dock holds by")
            .each(tap_while("viewer_next", |world| {
                world.resource::<ShipRuntime>().selected() != docked_port(world)
            }))
            .until(Arc::new(|world: &World| {
                docked_port(world).is_some()
                    && world.resource::<ShipRuntime>().selected() == docked_port(world)
            }))
            .diagnose(|world| {
                format!(
                    "selected {:?}, port {:?}",
                    world.resource::<ShipRuntime>().selected(),
                    docked_port(world)
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let the select key up")
            .on_enter(release_action("viewer_next"))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: the docked port's Delete is muted")
            .until(Arc::new(|world: &World| {
                delete_button(world).is_some_and(|(label, enabled)| !enabled && label == "Delete")
                    && delete_tooltip_text(world).as_deref() == Some(DOCKED_PORT_REASON)
                    && salvage_texts(world)
                        .is_some_and(|texts| !texts.iter().any(|text| text.starts_with('+')))
            }))
            .diagnose(|world| {
                format!(
                    "button {:?}, tooltip {:?}, salvage {:?}",
                    delete_button(world),
                    delete_tooltip_text(world),
                    salvage_texts(world)
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: press Delete on the docked port")
            .on_enter(|world: &mut World| {
                let port = docked_port(world);
                let sections = player_sections(world);
                world.insert_resource(SmokeDelete {
                    gun: port,
                    sections,
                });
                press_action("ship_delete")(world);
            })
            // The refused key puts the reason on the note line, which lasts
            // 2.5 s: read it while the key is down.
            .until(Arc::new(|world: &World| {
                world.resource::<ShipRuntime>().note() == Some(DOCKED_PORT_REASON)
            }))
            .diagnose(|world| format!("note {:?}", world.resource::<ShipRuntime>().note()))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let the refused delete key up")
            .on_enter(release_action("ship_delete"))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: the Delete key left the docked port on")
            .until(Arc::new(|world: &World| {
                world.get_resource::<SmokeDelete>().is_some_and(|kept| {
                    player_sections(world) == kept.sections
                        && kept.gun.is_some_and(|port| world.get_entity(port).is_ok())
                })
            }))
            .diagnose(|world| {
                format!(
                    "{:?}, sections {}, note {:?}",
                    world.get_resource::<SmokeDelete>(),
                    player_sections(world),
                    world.resource::<ShipRuntime>().note()
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: settle the muted Delete")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: shoot the muted Delete")
            .on_enter(|world: &mut World| {
                info!(
                    "station sandbox: delete button {:?}, tooltip {:?}, salvage {:?}",
                    delete_button(world),
                    delete_tooltip_text(world),
                    salvage_texts(world)
                );
                shoot(world, DELETE_REFUSED_SHOT);
            })
            .until(shot_written(DELETE_REFUSED_SHOT))
            .deadline(SHOT_SECS)
            .add();
        let script = read_the_split(script)
            .step("station sandbox: point at a hull section to pick")
            .on_enter(point_at_a_hull)
            .until(Arc::new(|world: &World| {
                world.get_resource::<SmokePick>().is_some_and(|pick| {
                    ship_hovered_section(world)
                        .and_then(|hovered| built_part(world, hovered))
                        .is_some_and(|part| Some(part) != pick.before)
                })
            }))
            .diagnose(|world| {
                format!(
                    "{:?}, hovered {:?}",
                    world.get_resource::<SmokePick>(),
                    ship_hovered_section(world)
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: pick its part with Q")
            .on_enter(|world: &mut World| {
                let hovered = ship_hovered_section(world);
                if let Some(mut pick) = world.get_resource_mut::<SmokePick>() {
                    pick.section = hovered;
                }
                press_action("ship_pick")(world);
            })
            .until(Arc::new(picked_the_hull))
            .diagnose(|world| {
                format!(
                    "{:?}, held {:?}",
                    world.get_resource::<SmokePick>(),
                    world
                        .get_resource::<HeldPart>()
                        .and_then(|held| held.0.clone())
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let the pick key up")
            .on_enter(release_action("ship_pick"))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: the pick key did not turn the view")
            .until(Arc::new(|world: &World| {
                world.get_resource::<SmokeArmed>().is_some_and(|armed| {
                    ship_view_facing(world).distance(armed.facing) < FACING_EPSILON
                })
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: shoot the picked part")
            .on_enter(|world: &mut World| shoot(world, PICK_SHOT))
            .until(shot_written(PICK_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("station sandbox: point at empty space in the view")
            .each(|world, _, _| {
                if let Some(view) = ui_node_rect(world, SHIP_VIEWPORT) {
                    move_cursor(view.min + Vec2::splat(EMPTY_INSET_PX))(world);
                }
            })
            .until(Arc::new(|world: &World| {
                pointer_over_node(SHIP_VIEWPORT)(world) && ship_hovered_section(world).is_none()
            }))
            .diagnose(|world| format!("hovered {:?}", ship_hovered_section(world)))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: Q over empty space puts the part away")
            .on_enter(press_action("ship_pick"))
            .until(Arc::new(|world: &World| {
                world.resource::<HeldPart>().id().is_none() && ship_ghost_aim(world).is_none()
            }))
            .diagnose(|world| {
                format!(
                    "held {:?}, aim {:?}",
                    world.resource::<HeldPart>().id(),
                    ship_ghost_aim(world)
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let the put-away key up")
            .on_enter(|world: &mut World| {
                release_action("ship_pick")(world);
                info!(
                    "station sandbox: Q over empty space, held {:?}",
                    world.resource::<HeldPart>().id()
                );
            })
            .until(frames(SETTLE_FRAMES))
            .add();
        // The cargo intake is three cells long when mated by an end socket:
        // the fixture's berth leaves 2.5 cells between the station-facing
        // plates and the station's boxes, so a one- or two-cell part never
        // reaches them. The station is on the ship's -X side, away from the
        // opening view, so the view turns first, by the right-button drag
        // the editor's camera looks with.
        let script = take_the_part(script, CARGO_INTAKE_SECTION_ID, None, CATALOGUE_INTAKE_SHOT);
        let script = script
            // Over the view first: the drag looks only while the pointer is
            // on it.
            .step("station sandbox: point at the view to drag it")
            .each(|world, _, _| keep_hovering_named(SHIP_VIEWPORT)(world))
            .until(pointer_over_node(SHIP_VIEWPORT))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: drag the view to the station")
            .on_enter(press_mouse(MouseButton::Right))
            .each(|world, _, _| {
                world.write_message(bevy::input::mouse::MouseMotion { delta: DRAG_PX });
            })
            .until(Arc::new(|world: &World| {
                ship_view_facing(world).x >= STATION_VIEW_FACING
            }))
            .diagnose(|world| format!("facing {:?}", ship_view_facing(world)))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let the drag button up")
            .on_enter(release_mouse(MouseButton::Right))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: aim from the sockets nearest the station")
            .on_enter(aim_from_the_station)
            .until(frames(1))
            .add()
            .step("station sandbox: aim the ghost into the station")
            .each(aim_into_the_station)
            .until(Arc::new(|world: &World| {
                refused_by_the_station(world) || aims_spent(world)
            }))
            .diagnose(|world| {
                format!(
                    "{:?}, verdict {:?}",
                    world.get_resource::<SmokeAdd>(),
                    ship_placement(world).map(|placement| &placement.verdict)
                )
            })
            .deadline(STATION_AIM_SECS)
            .add()
            .step("station sandbox: settle the refused ghost")
            .on_enter(|world: &mut World| {
                let verdict = ship_placement(world).map(|placement| placement.verdict.clone());
                if refused_by_the_station(world) {
                    info!("station sandbox: the station refuses the ghost: {verdict:?}");
                } else {
                    warn!("station sandbox: no free socket reaches the station: {verdict:?}");
                }
            })
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: shoot the ghost against the station")
            .on_enter(|world: &mut World| shoot(world, STATION_REFUSAL_SHOT))
            .until(shot_written(STATION_REFUSAL_SHOT))
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
            .step("station sandbox: let the next-tab key up on the Jobs tab")
            .on_enter(release_action("interface_next_tab"))
            .until(the_interface_shows(InterfacePaneType::Jobs))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: press the next-tab key again")
            .on_enter(press_action("interface_next_tab"))
            .until(frames(1))
            .add()
            .step("station sandbox: let the next-tab key up on the Station tab")
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
            .add();
        thruster_rebind_walk(script)
    }

    /// Rebind the first thruster to its own key, fire it on that key off
    /// the dock, and send it back to the Main Drive with Backspace.
    fn thruster_rebind_walk(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        let script = script
            .click_named(
                "station sandbox: back to the Ship tab",
                "InterfaceTabShip",
                the_interface_shows(InterfacePaneType::Ship),
                BEAT_SECS,
            )
            .step("station sandbox: put away the part in hand before the thruster")
            .on_enter(|world: &mut World| {
                if world.resource::<HeldPart>().id().is_some() {
                    press_key(KeyCode::Escape)(world);
                }
            })
            .until(Arc::new(|world: &World| {
                world.resource::<HeldPart>().id().is_none()
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let Esc up before the thruster")
            .on_enter(release_key(KeyCode::Escape))
            .until(the_interface_shows(InterfacePaneType::Ship))
            .deadline(BEAT_SECS)
            .add();
        let script = select_the_thruster(script)
            .step("station sandbox: the thruster shows its Main Drive default")
            .until(Arc::new(on_the_main_drive))
            .diagnose(|world| {
                format!(
                    "control {:?}, own {:?}",
                    control_text(world),
                    thruster_own_binding(world)
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: shoot the thruster on the Main Drive")
            .on_enter(|world: &mut World| {
                info!(
                    "station sandbox: thruster default, control {:?}",
                    control_text(world)
                );
                shoot(world, THRUSTER_DEFAULT_SHOT);
            })
            .until(shot_written(THRUSTER_DEFAULT_SHOT))
            .deadline(SHOT_SECS)
            .add();
        let script = arm_the_thruster_rebind(script)
            .step("station sandbox: press the thruster's own key")
            .on_enter(press_key(THRUSTER_KEY))
            .until(Arc::new(|world: &World| {
                thruster_own_binding(world) == Some(vec![InputSource::Keyboard(THRUSTER_KEY)])
            }))
            .diagnose(|world| format!("own {:?}", thruster_own_binding(world)))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let the thruster's key up")
            .on_enter(release_key(THRUSTER_KEY))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: the Control row names the own key")
            .until(Arc::new(|world: &World| {
                control_text(world).is_some_and(|text| text == "J")
            }))
            .diagnose(|world| format!("control {:?}", control_text(world)))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: shoot the thruster on its own key")
            .on_enter(|world: &mut World| {
                info!(
                    "station sandbox: thruster own key {:?}, control {:?}",
                    thruster_own_binding(world),
                    control_text(world)
                );
                shoot(world, THRUSTER_OWN_SHOT);
            })
            .until(shot_written(THRUSTER_OWN_SHOT))
            .deadline(SHOT_SECS)
            .add()
            // The world is frozen while the interface is open, and a docked
            // hull's own throttle is inert without the helm, which a station
            // keeps: the ship lets go of the dock to fly.
            .step("station sandbox: close the interface to fly")
            .on_enter(press_action("interface_toggle"))
            .until(Arc::new(|world: &World| {
                world
                    .get_resource::<State<PauseStates>>()
                    .is_some_and(|pause| *pause.get() == PauseStates::Unpaused)
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let the close key up")
            .on_enter(release_action("interface_toggle"))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: let go of the dock")
            .on_enter(press_action("dock"))
            .until(Arc::new(|world: &World| !player_docked(world)))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let the dock key up")
            .on_enter(release_action("dock"))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: the thruster's own key fires it")
            .on_enter(press_key(THRUSTER_KEY))
            .until(Arc::new(|world: &World| {
                thruster_throttle(world).is_some_and(|throttle| throttle >= 1.0)
            }))
            .diagnose(|world| format!("throttle {:?}", thruster_throttle(world)))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: report the fired thruster")
            .on_enter(|world: &mut World| {
                info!(
                    "station sandbox: thruster throttle on its key {:?}",
                    thruster_throttle(world)
                );
            })
            .add()
            .step("station sandbox: let the thruster's key up in flight")
            .on_enter(release_key(THRUSTER_KEY))
            .until(Arc::new(|world: &World| {
                thruster_throttle(world).is_some_and(|throttle| throttle == 0.0)
            }))
            .diagnose(|world| format!("throttle {:?}", thruster_throttle(world)))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: open the interface again")
            .on_enter(press_action("interface_toggle"))
            .until(Arc::new(|world: &World| {
                world
                    .get_resource::<State<PauseStates>>()
                    .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let the open key up")
            .on_enter(release_action("interface_toggle"))
            .until(frames(SETTLE_FRAMES))
            .add()
            .click_named(
                "station sandbox: the Ship tab again",
                "InterfaceTabShip",
                the_interface_shows(InterfacePaneType::Ship),
                BEAT_SECS,
            );
        arm_the_thruster_rebind(select_the_thruster(script))
            .step("station sandbox: Backspace puts the thruster back on the Main Drive")
            .on_enter(press_key(KeyCode::Backspace))
            .until(Arc::new(on_the_main_drive))
            .diagnose(|world| {
                format!(
                    "control {:?}, own {:?}",
                    control_text(world),
                    thruster_own_binding(world)
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: let Backspace up")
            .on_enter(release_key(KeyCode::Backspace))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: shoot the thruster back on the Main Drive")
            .on_enter(|world: &mut World| {
                info!(
                    "station sandbox: thruster cleared, own {:?}, control {:?}",
                    thruster_own_binding(world),
                    control_text(world)
                );
                shoot(world, THRUSTER_CLEARED_SHOT);
            })
            .until(shot_written(THRUSTER_CLEARED_SHOT))
            .deadline(SHOT_SECS)
            .add()
    }
}
