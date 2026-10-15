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
//!    side) and aim a Railgun Lance end-on (F) at a plate facing it to see it
//!    refused, rebind a gun with B, and delete a selected section with Delete
//!    (the button names what goes back in the hold, or is disabled with the
//!    reason in its hover tooltip);
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
//! view and checks the view eases onto the placed section, rebinds the new
//! gun to `T` and checks the selection did not step,
//! checks the Delete button names the gun's yield, deletes the gun with
//! Delete and checks the note names the parts that went to the hold, selects
//! the port the dock holds by and checks its Delete is disabled with the
//! reason in its tooltip, that the Delete key leaves it on and that hovering
//! it shows the tooltip, picks a
//! hull part off the ship with Q, puts it away with Q over empty space and
//! checks the hand is empty, takes the Railgun Lance from the Catalogue, drags the
//! view to the station and points the lance at the sockets nearest it, through every
//! mating socket, until the station refuses it (or warns that none reaches),
//! and switches tabs after it: a Map tab click, an Inventory tab click and the
//! next-tab key, which steps on to the Station tab while docked. `NOVA_CAPTURE=1` shoots `sandbox-ship.png`,
//! `sandbox-catalogue-all.png`, `sandbox-catalogue.png`, `sandbox-ghost.png`, `sandbox-mount.png`, `sandbox-placed.png`,
//! `sandbox-rebind-armed.png`, `sandbox-rebind.png`, `sandbox-delete-yield.png`,
//! `sandbox-delete.png`,
//! `sandbox-delete-refused.png`,
//! `sandbox-pick.png`, `sandbox-catalogue-lance.png`,
//! `sandbox-station-refusal.png`,
//! `sandbox-map-after.png` and `sandbox-station.png`.
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

/// The world seed: `system_station_buy`'s, whose window around the spawn
/// holds a living Neutral station that refits and sells hulls.
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
            Option<&ParkedHulls>,
            Option<&PendingParkedHulls>,
        ), With<StationMarker>>()
        .iter(world)
        .filter(
            |(_, position, services, derelict, allegiance, parked, pending)| {
                station_offers(*services, *derelict, StationServiceType::Refit)
                    && (parked.is_some() || pending.is_some())
                    && *allegiance != Some(&Allegiance::Enemy)
                    && SectorCoord::containing(Meters3::from_engine(position.0), edge)
                        != SectorCoord::ORIGIN
            },
        )
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
        ship_hovered_section, ship_placement, socket_lane_open, ShipPlacement, ShipRuntime,
        ShipSectionView, ShipSections,
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
            .map(|verdict| verdict.map(str::to_string).map_err(str::to_string));
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
                if let Some(line) = placement.verdict.as_ref().ok() {
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
                    let cost: u32 = payment
                        .bought
                        .iter()
                        .map(|(item, count)| count * items.design(item).ask_cr)
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
            let binding = view.binding_text();
            if let Some(verdict) = &delete {
                details.push(match verdict {
                    Ok(salvage) => format!("delete: {salvage}"),
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
        nova_debug::harness::{press_action, release_action, shoot, AutopilotPlugin, Predicate},
        nova_interface::ship::{
            ship_ghost_aim, ship_hovered_section, ship_placement, ship_view_window_px, ShipRuntime,
        },
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
    /// The PNG of the ghost rolled a quarter turn and on its next socket.
    const MOUNT_SHOT: &str = "sandbox-mount.png";
    /// The PNG of the Ship tab after the add.
    const PLACED_SHOT: &str = "sandbox-placed.png";
    /// The PNG of the placed section after its rebind.
    const REBIND_SHOT: &str = "sandbox-rebind.png";
    /// The PNG of the armed rebind: the lit button and the prompt banner.
    const REBIND_ARMED_SHOT: &str = "sandbox-rebind-armed.png";
    /// The PNG of the rebound gun's enabled Delete, its label naming the
    /// parts it gives back.
    const DELETE_YIELD_SHOT: &str = "sandbox-delete-yield.png";
    /// The PNG of the Ship tab once the rebound gun is deleted: the gun gone,
    /// the note naming the parts that went to the hold.
    const DELETE_SHOT: &str = "sandbox-delete.png";
    /// The PNG of the docked port's disabled Delete under the pointer, its
    /// reason in the tooltip under it.
    const DELETE_REFUSED_SHOT: &str = "sandbox-delete-refused.png";
    /// The Ship panel's Delete button.
    const DELETE_BUTTON: &str = "ShipDelete";
    /// The PNG of the Catalogue open on the kinetic PDC's category.
    const CATALOGUE_SHOT: &str = "sandbox-catalogue.png";
    /// The PNG of the Catalogue as it opens, on every part the station
    /// builds: each tile's name, kind and price line inside the tile.
    const CATALOGUE_ALL_SHOT: &str = "sandbox-catalogue-all.png";
    /// The PNG of the Catalogue open on the Railgun Lance's category.
    const CATALOGUE_LANCE_SHOT: &str = "sandbox-catalogue-lance.png";
    /// The PNG of the hull part the pipette took off the ship.
    const PICK_SHOT: &str = "sandbox-pick.png";
    /// Mouse travel per frame while the walk drags the view, in pixels:
    /// rightward, the short way round to the station side. About 35 frames.
    const DRAG_PX: Vec2 = Vec2::new(16.0, 0.0);
    /// The PNG of the ghost refused against the docked station.
    const STATION_REFUSAL_SHOT: &str = "sandbox-station-refusal.png";
    /// How far from the station a free socket's aim may be for the walk to
    /// try it: the railgun lance's length.
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
    /// The Station tab, reached with the next-tab key from Inventory.
    const STATION_SHOT: &str = "sandbox-station.png";
    /// Seconds a click or a key gets to land.
    const BEAT_SECS: f32 = 60.0;
    /// How far inside the Ship view's top-left corner the walk points at
    /// empty space, in pixels: clear of the ship, which the view frames in
    /// its middle.
    const EMPTY_INSET_PX: f32 = 24.0;
    /// The Ship pane's 3D view.
    const SHIP_VIEWPORT: &str = "ShipViewport";

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
        if refused_by_the_station(world) || frame % AIM_FRAMES != 0 {
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

    /// The Delete button's tooltip: why it is refused, empty while it holds.
    fn delete_reason(world: &World) -> Option<String> {
        let mut buttons = world.try_query::<(&Name, &nova_ui::widget::Tooltip)>()?;
        buttons
            .iter(world)
            .find(|(name, _)| name.as_str() == DELETE_BUTTON)
            .map(|(_, tooltip)| tooltip.0.clone())
    }

    /// Whether the Delete button's tooltip panel is up and laid out.
    fn delete_tooltip_shown(world: &World) -> bool {
        let Some(button) = world.try_query::<(Entity, &Name)>().and_then(|mut names| {
            names
                .iter(world)
                .find(|(_, name)| name.as_str() == DELETE_BUTTON)
                .map(|(button, _)| button)
        }) else {
            return false;
        };
        world
            .try_query_filtered::<(&ChildOf, &Node, &ComputedNode), With<nova_ui::widget::TooltipPanel>>()
            .is_some_and(|mut panels| {
                panels.iter(world).any(|(parent, node, computed)| {
                    parent.parent() == button
                        && node.display == Display::Flex
                        && computed.size().x > 0.0
                })
            })
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
        let script = AutopilotPlugin::<GameStates>::new()
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
            .step("station sandbox: shoot the placed section")
            .on_enter(|world: &mut World| shoot(world, PLACED_SHOT))
            .until(shot_written(PLACED_SHOT))
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
            .add()
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
            .step("station sandbox: the Delete button names the yield")
            .until(Arc::new(|world: &World| {
                delete_button(world)
                    .is_some_and(|(label, enabled)| enabled && label.starts_with("Delete: +"))
                    && delete_reason(world).is_some_and(|reason| reason.is_empty())
            }))
            .diagnose(|world| {
                format!(
                    "button {:?}, reason {:?}",
                    delete_button(world),
                    delete_reason(world)
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: settle the enabled Delete")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station sandbox: shoot the enabled Delete")
            .on_enter(|world: &mut World| {
                info!("station sandbox: delete button {:?}", delete_button(world));
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
                        note.starts_with("deleted ") && note.ends_with(" to the hold")
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
            .step("station sandbox: the docked port's Delete is disabled")
            .until(Arc::new(|world: &World| {
                delete_button(world).is_some_and(|(label, enabled)| !enabled && label == "Delete")
                    && delete_reason(world)
                        .is_some_and(|reason| reason.starts_with("delete: the dock holds"))
            }))
            .diagnose(|world| {
                format!(
                    "button {:?}, reason {:?}",
                    delete_button(world),
                    delete_reason(world)
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
            .until(frames(SETTLE_FRAMES))
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
                        && !world
                            .resource::<ShipRuntime>()
                            .note()
                            .is_some_and(|note| note.starts_with("delete:"))
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
            .step("station sandbox: hover the disabled Delete for its reason")
            .on_enter(hover_named(DELETE_BUTTON))
            .each(|world, _, _| keep_hovering_named(DELETE_BUTTON)(world))
            .until(Arc::new(delete_tooltip_shown))
            .deadline(BEAT_SECS)
            .add()
            .step("station sandbox: shoot the disabled Delete")
            .on_enter(|world: &mut World| {
                info!(
                    "station sandbox: delete button {:?}, tooltip {:?}, shown {}",
                    delete_button(world),
                    delete_reason(world),
                    delete_tooltip_shown(world)
                );
                shoot(world, DELETE_REFUSED_SHOT);
            })
            .until(shot_written(DELETE_REFUSED_SHOT))
            .deadline(SHOT_SECS)
            .add();
        let script = script
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
        // The three-cell railgun lance: the fixture's berth leaves 2.5 cells
        // between the station-facing plates and the station's boxes, so a
        // one- or two-cell part never reaches them. The station is on the
        // ship's -X side, away from the opening view, so the view turns
        // first, by the right-button drag the editor's camera looks with.
        let script = take_the_part(script, RAILGUN_LANCE_SECTION_ID, None, CATALOGUE_LANCE_SHOT);
        script
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
