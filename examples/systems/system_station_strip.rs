//! system_station_strip: the Inventory pane's strip block strips a whole
//! docked generated derelict on the game clock, one section at a time with the berth port
//! last, then the wreck is gone and the player is free. An undock part-way
//! stops the strip and keeps what it took, across a retire and a thaw; a
//! living station shows the Station tab, no strip block, and refuses a strip.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed the way the Create World modal does, and lets the window stream in
//! around the session's starting station, where the start docks the player.
//! The walk undocks and jumps to the origin, the spawn point before the
//! docked start: stations are a civilization apart, so the station it wants
//! never streams in round the start. Once the window stands round the origin
//! it picks the nearest generated derelict ship (lootable, with a free live
//! port) and the nearest living Player or Neutral station. The player jumps
//! beside each and is put on a berth at its collar each frame until the dock
//! takes; the walk never flies an approach (`system_station_docking` does).
//!
//! Both strips start from the Inventory pane's Strip button, the code path a
//! player clicks, and run with the TAB interface shut. The first is stopped
//! by an undock once it has taken a section; the second, after a retire and
//! a thaw and a new dock, runs to the end. The strip at the living station is
//! a `HullStripCommand`, the message the Strip button writes.
//!
//! DISCLOSED FIXTURE STATE: the berth points the mouse rig where it points
//! the hull, so an undock does not turn the hull back through the wreck. The
//! crossing JUMPS the player past the active radius from the wreck, so its
//! cell retires and freezes, then back beside it, so it thaws.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: an undock stops a whole-ship strip and keeps what it took` | a strip that has taken a section stops when the player undocks; the taken sections are gone, the rest and the berth stand, and the hold gained the parts of each `OnStripped` |
//! | 2 | `outcome: a part-stripped wreck stays part-stripped across a retire and a thaw` | the frozen record holds exactly the sections left standing; the thawed wreck has exactly those live |
//! | 3 | `outcome: a whole-ship strip runs with the HUD closed and removes the docking port last` | every section came off while the TAB interface was shut, one `OnStripped` each, and the last is the berth port |
//! | 4 | `outcome: a finished strip demolishes the wreck and releases the player` | the wreck is gone, the player has no dock and no connection names it, and no strip runs |
//! | 5 | `outcome: a finished strip's yield is in the hold` | the hold gained the whole-hull plan's parts, the sum of every `OnStripped` |
//! | 6 | `outcome: a wreck a strip demolished stays gone across a retire` | the retired cell's frozen record holds no wreck, and none stands when the cell streams back |
//! | 7 | `outcome: a living station shows the Station tab and refuses a strip` | docked at a living station, the Station tab shows and the Inventory pane shows no strip block; a strip command shows the living-station reason, starts no strip and moves nothing |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=3600 \
//!   cargo run --example system_station_strip --features debug
//! # look for: `station strip: PASS a living station shows the Station tab`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! Add `NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=<dir>` to shoot the Inventory pane
//! before the strip as `strip-plan.png`, the wreck early and late in the
//! strip as `strip-wreck-early.png` and `strip-wreck-late.png`, the HUD chip
//! as `strip-hud.png` and the Inventory pane mid-strip as
//! `strip-inventory-mid.png`.

use bevy::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "system_station_strip")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed: dock at a generated derelict, start a whole-ship strip \
             from the Inventory pane, undock part-way, leave the wreck's cell and come back, strip \
             the rest with the port last until the wreck is gone, then refuse a strip at a \
             living station. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

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

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // The fleet's 1920x1080, the size the tab is shot at.
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
        // No frame-time claim: the range holds markers, holds and ids.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::StripWatchPlugin);
        app.add_plugins(walk::script());
    }

    app.run()
}

#[cfg(feature = "debug")]
mod walk {
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::Arc,
    };

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use nova_protocol::{
        nova_debug::harness::{
            press_action, release_action, shoot, AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS,
        },
        nova_events::engine::GameEvent,
        nova_interface::{
            inventory::{
                HullStripCommand, InventoryRuntime, INVENTORY_STRIP_BLOCK, INVENTORY_STRIP_BUTTON,
            },
            pane::InterfacePaneType,
            ship::design_section_code,
        },
        prelude::*,
    };
    use nova_ui::widget::TextFieldValue;
    use nova_world::prelude::{
        desired_sectors, FrozenBodyType, FrozenSectors, SectorRoot, WorldConfig,
    };

    use super::*;

    /// The seed the range plays: its window around the origin holds a
    /// generated derelict ship and a living Player or Neutral station.
    const STRIP_SEED: u32 = 115;

    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const WORLD_NAME: &str = "Station Strip World";
    const SEED_FIELD: &str = "World Seed Field";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const WORLD_NEXT_BUTTON: &str = "World Next Button";
    /// The start role the walk picks: the scavenger's salvage hauler, a compact
    /// box with a deep hold. The first role's mining tug has a bow boom that
    /// can strike a berthed partner.
    const START_ROLE_BUTTON: &str = "Start Role Button: scavenger";

    /// The interface tabs, named `InterfaceTab<label>`.
    const INVENTORY_TAB: &str = "InterfaceTabInventory";
    const STATION_TAB: &str = "InterfaceTabStation";

    /// The PNGs the capture path shoots.
    const PLAN_SHOT: &str = "strip-plan.png";
    const WRECK_EARLY_SHOT: &str = "strip-wreck-early.png";
    const HUD_SHOT: &str = "strip-hud.png";
    const TAB_SHOT: &str = "strip-inventory-mid.png";
    const WRECK_LATE_SHOT: &str = "strip-wreck-late.png";

    /// How far from a target the player parks before the berth.
    const PARK_DISTANCE: Meters = Meters(2_000.0);
    /// How many cells past the active radius the crossing lands from the
    /// wreck, so its cell retires.
    const CROSSING_MARGIN: i32 = 2;

    /// Seconds a load, a stream or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Seconds the whole rest of the wreck gets to come off.
    const STRIP_SECS: f32 = 1_200.0;
    /// Frames the Inventory pane gets to build its columns.
    const SETTLE_FRAMES: u32 = 10;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;
    /// The shares of the second strip done when the wreck, the HUD chip and
    /// the tab are shot, and the wreck again.
    const EARLY_SHOT_PROGRESS: f32 = 0.1;
    const MID_SHOT_PROGRESS: f32 = 0.4;
    const LATE_SHOT_PROGRESS: f32 = 0.7;

    /// The living-station refusal line, as the strip handler writes it.
    const LIVING_STATION_NOTE: &str = "strip: a living station is not salvage";

    /// One target the walk docks at.
    #[derive(Clone, Debug)]
    struct Target {
        id: GeneratedBodyId,
        cell: SectorCoord,
        position: Meters3,
    }

    /// One `OnStripped`.
    #[derive(Clone, Debug)]
    struct Stripped {
        data: serde_json::Value,
    }

    /// What the run saw and recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct StripWatch {
        stripped: Vec<Stripped>,
        /// Each frame a running strip took sections off the wreck: how many,
        /// and whether the TAB interface was shut that frame.
        drops: Vec<(usize, bool)>,
        /// The wreck's live sections last frame, while a strip ran.
        seen: Option<usize>,
        wreck: Option<Target>,
        station: Option<Target>,
        /// Every section of the wreck at the first dock: id to pane code.
        codes: BTreeMap<String, String>,
        /// The wreck port the player docks through.
        berth: Option<String>,
        /// The whole-hull plan's parts at the first dock.
        planned: BTreeMap<ItemDesignId, u32>,
        /// The player's hold at the first dock.
        start_hold: Option<ShipInventory>,
        /// The sections standing after the undock.
        left: BTreeSet<String>,
        /// The player's hold right before the step that checks it.
        hold: Option<ShipInventory>,
        /// The cell the crossing landed in.
        crossed: Option<SectorCoord>,
    }

    pub(super) struct StripWatchPlugin;

    impl Plugin for StripWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<StripWatch>();
            app.add_systems(Update, hold_the_walk_clock);
            app.add_systems(Last, watch_the_drops);
            app.add_observer(|event: On<GameEvent>, mut watch: ResMut<StripWatch>| {
                if event.name() != "onstripped" {
                    return;
                }
                let data = event.info().data.clone().unwrap_or_default();
                info!("station strip: onstripped {data}");
                watch.stripped.push(Stripped { data });
            });
        }
    }

    /// Record each frame a running strip takes sections off the wreck, with
    /// the pause state of that frame. Runs in `Last`, after the strip's
    /// commands, so a section taken this frame is gone and the berth's frame
    /// shows the whole wreck gone.
    fn watch_the_drops(world: &mut World) {
        let running =
            the_player(world).is_some_and(|player| world.get::<HullStripJob>(player).is_some());
        let live = wreck_root(world).map_or(0, |wreck| live_sections(world, wreck).len());
        let unpaused = world
            .get_resource::<State<PauseStates>>()
            .is_some_and(|pause| *pause.get() == PauseStates::Unpaused);
        let mut watch = world.resource_mut::<StripWatch>();
        if let Some(seen) = watch.seen.filter(|seen| live < *seen) {
            watch.drops.push((seen - live, unpaused));
        }
        watch.seen = running.then_some(live);
    }

    /// Hold the virtual clock's clamp at [`WALK_CLOCK_SECS`] every frame: a
    /// scenario load hands `Time<Virtual>` back at its default.
    fn hold_the_walk_clock(mut time: ResMut<Time<Virtual>>) {
        let clamp = std::time::Duration::from_secs_f32(WALK_CLOCK_SECS);
        if time.max_delta() != clamp {
            time.set_max_delta(clamp);
        }
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    /// The one player ship, if exactly one stands.
    fn the_player(world: &World) -> Option<Entity> {
        let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
        let mut players = players.iter(world);
        let player = players.next()?;
        players.next().is_none().then_some(player)
    }

    fn player(world: &World) -> Entity {
        the_player(world).expect("station strip: exactly one player ship")
    }

    /// The entity wearing `id`, if one does.
    fn wearing(world: &World, id: &GeneratedBodyId) -> Option<Entity> {
        let mut query = world.try_query::<(Entity, &GeneratedBodyId)>()?;
        query
            .iter(world)
            .find(|(_, worn)| *worn == id)
            .map(|(entity, _)| entity)
    }

    fn wreck_target(world: &World) -> Target {
        world
            .resource::<StripWatch>()
            .wreck
            .clone()
            .expect("station strip: the wreck is chosen")
    }

    fn station_target(world: &World) -> Target {
        world
            .resource::<StripWatch>()
            .station
            .clone()
            .expect("station strip: the station is chosen")
    }

    fn wreck_root(world: &World) -> Option<Entity> {
        wearing(world, &world.resource::<StripWatch>().wreck.as_ref()?.id)
    }

    fn wreck(world: &World) -> Entity {
        wreck_root(world).expect("station strip: the wreck stands")
    }

    fn station_root(world: &World) -> Option<Entity> {
        wearing(world, &world.resource::<StripWatch>().station.as_ref()?.id)
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("station strip: the station stands")
    }

    fn hold(world: &World) -> ShipInventory {
        world
            .get::<ShipInventory>(player(world))
            .expect("station strip: the player has a hold")
            .clone()
    }

    fn record_hold(world: &mut World) {
        let hold = hold(world);
        world.resource_mut::<StripWatch>().hold = Some(hold);
    }

    fn recorded_hold(world: &World) -> ShipInventory {
        world
            .resource::<StripWatch>()
            .hold
            .clone()
            .expect("station strip: the hold is recorded")
    }

    /// Every live section of `root`: entity, design section id, health share
    /// and neighbours.
    fn live_sections(world: &World, root: Entity) -> Vec<(Entity, String, f32, Vec<Entity>)> {
        world
            .get::<Children>(root)
            .into_iter()
            .flatten()
            .filter(|child| world.get::<SectionMarker>(**child).is_some())
            .filter_map(|child| {
                let id = world.get::<EntityId>(*child)?;
                let health = world.get::<Health>(*child)?;
                let share = if health.max > 0.0 {
                    health.current / health.max
                } else {
                    0.0
                };
                let neighbours = world
                    .get::<ConnectedTo>(*child)
                    .map(|connected| connected.0.clone())
                    .unwrap_or_default();
                Some((*child, id.0.clone(), share, neighbours))
            })
            .collect()
    }

    fn world_shape(world: &World) -> (Meters, i32) {
        let config = world.resource::<WorldConfig<NovaLayeredWorld>>();
        (config.sector_edge, config.active_radius)
    }

    /// The cell of the session's starting station, where the start docks the
    /// player, once the session and the world config stand.
    fn start_cell(world: &World) -> Option<SectorCoord> {
        let edge = world
            .get_resource::<WorldConfig<NovaLayeredWorld>>()?
            .sector_edge;
        let start = world.get_resource::<OpenWorldSession>()?.start.position;
        Some(SectorCoord::containing(start, edge))
    }

    fn live_cells(world: &World) -> std::collections::BTreeSet<SectorCoord> {
        world
            .try_query::<&SectorRoot>()
            .map(|mut query| query.iter(world).map(|root| root.0).collect())
            .unwrap_or_default()
    }

    /// The cell the player stands in.
    fn player_cell(world: &World) -> Option<SectorCoord> {
        let player = the_player(world)?;
        let (edge, _) = world_shape(world);
        Some(SectorCoord::containing(
            Meters3::from_engine(world.get::<Position>(player)?.0),
            edge,
        ))
    }

    /// Advance once the world is armed around one player ship and the live
    /// set is exactly the window around `centre`.
    fn window_is(
        centre: impl Fn(&World) -> Option<SectorCoord> + Send + Sync + 'static,
    ) -> Arc<Predicate> {
        when(move |world| {
            if !world.contains_resource::<WorldConfig<NovaLayeredWorld>>()
                || the_player(world).is_none()
            {
                return false;
            }
            let Some(centre) = centre(world) else {
                return false;
            };
            let (_, radius) = world_shape(world);
            live_cells(world) == desired_sectors(centre, radius)
        })
    }

    /// The root `root` stands with its sections spawned.
    fn stands(world: &World, root: Option<Entity>) -> bool {
        root.is_some_and(|root| {
            world.get::<Position>(root).is_some()
                && world.get::<ShipDesignSections>(root).is_some()
                && !live_sections(world, root).is_empty()
        })
    }

    fn the_wreck_stands() -> Arc<Predicate> {
        when(|world| stands(world, wreck_root(world)))
    }

    fn the_station_stands() -> Arc<Predicate> {
        when(|world| {
            stands(world, station_root(world))
                && world.get::<StationServices>(station(world)).is_some()
        })
    }

    fn player_docked() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
        })
    }

    fn player_free() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_none())
        })
    }

    fn the_interface_shows(pane: InterfacePaneType) -> Arc<Predicate> {
        when(move |world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
                && world
                    .get_resource::<InterfacePaneType>()
                    .is_some_and(|shown| *shown == pane)
        })
    }

    fn interface_open() -> Arc<Predicate> {
        when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
        })
    }

    fn unpaused() -> Arc<Predicate> {
        when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Unpaused)
        })
    }

    /// Teleport the player to `position`, at rest.
    fn park_player(world: &mut World, position: Meters3) {
        let player = player(world);
        world.entity_mut(player).insert((
            Position(position.to_engine()),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
        ));
        info!("station strip: the player jumps to {position:?}");
    }

    /// Park [`PARK_DISTANCE`] from `target` toward the origin.
    fn park_beside(world: &mut World, target: &Target) {
        let toward_origin = (-target.position.get()).normalize_or(Vec3::X);
        park_player(
            world,
            Meters3(target.position.get() + toward_origin * PARK_DISTANCE.get()),
        );
    }

    /// `section`'s position and rotation in `root`'s frame, by its mount chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("station strip: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("station strip: a section's chain reaches its root")
                .parent();
        }
        (position, rotation)
    }

    /// The free, live docking ports mounted on `root`, lowest first.
    fn free_ports(world: &mut World, root: Entity) -> Vec<Entity> {
        let ports: Vec<Entity> = world
            .query_filtered::<Entity, (
                With<DockingSectionMarker>,
                Without<DockedPort>,
                Without<SectionInactiveMarker>,
            )>()
            .iter(world)
            .collect();
        let mut ports: Vec<Entity> = ports
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
            .collect();
        ports.sort();
        ports
    }

    fn free_port(world: &mut World, root: Entity) -> Entity {
        free_ports(world, root)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("station strip: {root:?} has no free docking port"))
    }

    /// The free port of `target` that faces most squarely away from its
    /// root, so the berth sets the player outside the hull. The lowest port
    /// of a station can face into its own structure, and a berth there puts
    /// the two hulls through each other.
    fn outward_port(world: &mut World, target: Entity) -> Entity {
        let ports = free_ports(world, target);
        ports
            .into_iter()
            .map(|port| {
                let (at, rotation) = local_pose(world, port, target);
                let facing = (rotation * Vec3::NEG_Z).dot(at.normalize_or_zero());
                (port, facing)
            })
            .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
            .map(|(port, _)| port)
            .unwrap_or_else(|| panic!("station strip: {target:?} has no free docking port"))
    }

    /// Put the player on the berth at `target`'s collar, moving and turning
    /// with it, then ask to dock. Every frame until the dock takes, as
    /// `system_station_own` does.
    fn berth_and_dock(world: &mut World, target: Entity) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let target_port = outward_port(world, target);
        let player_port = free_port(world, player);
        let position = world.get::<Position>(target).expect("a hull stands").0;
        let rotation = world.get::<Rotation>(target).expect("a hull stands").0;
        let velocity = world
            .get::<LinearVelocity>(target)
            .map_or(Vec3::ZERO, |v| v.0);
        let spin = world
            .get::<AngularVelocity>(target)
            .map_or(Vec3::ZERO, |w| w.0);
        let (port_at, port_rot) = local_pose(world, target_port, target);
        let (own_at, own_rot) = local_pose(world, player_port, player);
        let (at, turn) =
            parked_hull_pose((position, rotation), (port_at, port_rot), (own_at, own_rot));
        world.entity_mut(player).insert((
            Position(at),
            Rotation(turn),
            LinearVelocity(velocity),
            AngularVelocity(spin),
        ));
        // Point the mouse rig where the berth points the hull, as a pilot who
        // flew in would. Left on the spawn heading, it turns the hull back
        // the moment it undocks and sweeps it through the wreck.
        let rigs: Vec<Entity> = world
            .query_filtered::<Entity, With<PointRotationOutput>>()
            .iter(world)
            .collect();
        for rig in rigs {
            world.entity_mut(rig).insert((
                PointRotation {
                    initial_rotation: turn,
                },
                PointRotationOutput(turn),
            ));
        }
        world.trigger(DockingConnectionRequest {
            entity: player,
            target,
        });
    }

    fn set_field_text(world: &mut World, name: &str, text: &str) {
        let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
        let (_, mut value) = fields
            .iter_mut(world)
            .find(|(found, _)| found.as_str() == name)
            .unwrap_or_else(|| panic!("station strip: no field named '{name}'"));
        value.0 = text.to_string();
    }

    /// Press and release a key action, one frame each.
    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("station strip: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("station strip: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Close the TAB interface, so the clock runs again.
    fn close_the_interface(
        script: AutopilotPlugin<GameStates>,
        label: &str,
    ) -> AutopilotPlugin<GameStates> {
        tap(
            script,
            &format!("the interface key {label}"),
            "interface_toggle",
            unpaused(),
        )
    }

    fn shoot_step(
        script: AutopilotPlugin<GameStates>,
        shot: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("station strip: shoot {shot}"))
            .on_enter(move |world: &mut World| shoot(world, shot))
            .until(shot_written(shot))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
    }

    /// Pick the nearest generated derelict ship that is lootable and has a
    /// free live port, and the nearest living Player or Neutral station.
    fn choose_the_targets(world: &mut World) {
        let (edge, _) = world_shape(world);
        let seen: Vec<(
            Entity,
            GeneratedBodyId,
            Vec3,
            bool,
            bool,
            bool,
            Option<Allegiance>,
        )> = world
            .query::<(
                Entity,
                &GeneratedBodyId,
                &Position,
                Has<StationMarker>,
                Has<DerelictShipMarker>,
                Has<LootableShipMarker>,
                Option<&Allegiance>,
            )>()
            .iter(world)
            .map(
                |(entity, id, position, station, derelict, lootable, side)| {
                    (
                        entity,
                        id.clone(),
                        position.0,
                        station,
                        derelict,
                        lootable,
                        side.copied(),
                    )
                },
            )
            .collect();
        let distance = |at: Vec3| Meters3::from_engine(at).distance(Meters3::ZERO).get();
        for (_, id, at, station, derelict, lootable, side) in &seen {
            info!(
                "station strip: the window holds '{id}', station {station}, derelict {derelict}, \
                 lootable {lootable}, side {side:?}, {:.0} m out",
                distance(*at)
            );
        }
        let target = |(id, at): (GeneratedBodyId, Vec3)| {
            let position = Meters3::from_engine(at);
            Target {
                id,
                cell: SectorCoord::containing(position, edge),
                position,
            }
        };
        let mut wrecks: Vec<(f32, GeneratedBodyId, Vec3)> = Vec::new();
        for (entity, id, at, station, derelict, lootable, _) in &seen {
            if !*station
                && *derelict
                && *lootable
                && world.get::<ShipDesignSections>(*entity).is_some()
                && !free_ports(world, *entity).is_empty()
            {
                wrecks.push((distance(*at), id.clone(), *at));
            }
        }
        wrecks.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let mut stations: Vec<(f32, GeneratedBodyId, Vec3)> = seen
            .iter()
            .filter(|(_, _, _, station, derelict, _, side)| {
                *station
                    && !*derelict
                    && matches!(side, Some(Allegiance::Player | Allegiance::Neutral))
            })
            .map(|(_, id, at, ..)| (distance(*at), id.clone(), *at))
            .collect();
        stations.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let Some((_, wreck_id, wreck_at)) = wrecks.into_iter().next() else {
            panic!(
                "station strip: the window on seed {STRIP_SEED} holds no lootable generated \
                 derelict ship with a free port"
            );
        };
        let Some((_, station_id, station_at)) = stations.into_iter().next() else {
            panic!(
                "station strip: the window on seed {STRIP_SEED} holds no living Player or \
                 Neutral station"
            );
        };
        let (wreck, station) = (
            target((wreck_id, wreck_at)),
            target((station_id, station_at)),
        );
        info!(
            "station strip: chose the wreck '{}' in cell {} and the station '{}' in cell {}",
            wreck.id, wreck.cell, station.id, station.cell
        );
        let mut watch = world.resource_mut::<StripWatch>();
        watch.wreck = Some(wreck);
        watch.station = Some(station);
    }

    /// The berth the player is docked through on `partner`.
    fn partner_berth(world: &World, partner: Entity) -> Entity {
        let docked = world
            .get::<DockedShip>(player(world))
            .expect("station strip: the player is docked");
        let connection = world
            .get::<DockingConnection>(docked.connection)
            .expect("station strip: a dock has a connection");
        if connection.first_ship == partner {
            connection.first_section
        } else {
            connection.second_section
        }
    }

    /// Jump the player past the active radius from the wreck, along +X.
    fn jump_out(world: &mut World) {
        let (edge, radius) = world_shape(world);
        let wreck = wreck_target(world);
        let at = Meters3(
            wreck.position.get() + Vec3::X * edge.get() * (radius + CROSSING_MARGIN) as f32,
        );
        park_player(world, at);
        let cell = SectorCoord::containing(at, edge);
        info!("station strip: the player jumps to cell {cell}");
        world.resource_mut::<StripWatch>().crossed = Some(cell);
    }

    fn wreck_cell_frozen() -> Arc<Predicate> {
        when(|world| {
            world
                .resource::<StripWatch>()
                .wreck
                .as_ref()
                .is_some_and(|wreck| world.resource::<FrozenSectors>().get(wreck.cell).is_some())
        })
    }

    /// Whether the wreck's frozen record keeps state for each of `sections`;
    /// `None` when no record holds the wreck as a ship.
    fn frozen_sections(sectors: &FrozenSectors, id: &str, sections: &[&str]) -> Option<Vec<bool>> {
        sectors.iter().find_map(|(_, record)| {
            record
                .bodies()
                .iter()
                .find(|body| body.id() == Some(id))
                .and_then(|body| match body.body() {
                    FrozenBodyType::Ship(ship) => Some(
                        sections
                            .iter()
                            .map(|section| ship.has_section(section))
                            .collect(),
                    ),
                    _ => None,
                })
        })
    }

    fn job(world: &World) -> Option<&HullStripJob> {
        world.get::<HullStripJob>(the_player(world)?)
    }

    fn note(world: &World) -> Option<&str> {
        world.get_resource::<InventoryRuntime>()?.note()
    }

    fn start_hold(world: &World) -> ShipInventory {
        world
            .resource::<StripWatch>()
            .start_hold
            .clone()
            .expect("station strip: the start hold is recorded")
    }

    fn berth_id(world: &World) -> String {
        world
            .resource::<StripWatch>()
            .berth
            .clone()
            .expect("station strip: the berth is recorded")
    }

    fn live_ids(world: &World, root: Entity) -> BTreeSet<String> {
        live_sections(world, root)
            .into_iter()
            .map(|(_, id, ..)| id)
            .collect()
    }

    /// The parts every `OnStripped` so far carried, summed.
    fn stripped_parts(world: &World) -> BTreeMap<ItemDesignId, u32> {
        let mut parts: BTreeMap<ItemDesignId, u32> = BTreeMap::new();
        for stripped in &world.resource::<StripWatch>().stripped {
            let map = stripped.data["parts"]
                .as_object()
                .expect("station strip: OnStripped carries parts");
            for (item, count) in map {
                let count = u32::try_from(count.as_u64().expect("a part count is a number"))
                    .expect("a part count fits u32");
                *parts.entry(ItemDesignId::from(item.as_str())).or_default() += count;
            }
        }
        parts.retain(|_, count| *count > 0);
        parts
    }

    /// What the hold gained since `before`, per item. Panics on a loss.
    fn gained(before: &ShipInventory, after: &ShipInventory) -> BTreeMap<ItemDesignId, u32> {
        let mut items: BTreeSet<ItemDesignId> =
            before.stacks().map(|(item, _)| item.clone()).collect();
        items.extend(after.stacks().map(|(item, _)| item.clone()));
        items
            .into_iter()
            .filter_map(|item| {
                let (was, is) = (before.count(&item), after.count(&item));
                assert!(is >= was, "station strip: the hold lost '{item}'");
                (is > was).then_some((item, is - was))
            })
            .collect()
    }

    /// Plan the whole-hull strip of the docked wreck and record its sections,
    /// its berth and the hold as the press leaves it: a Strip first takes the
    /// wreck's whole hold, so the strip's parts are counted from there.
    fn plan_the_strip(world: &mut World) {
        let wreck = wreck(world);
        let items = world.resource::<GameItems>().clone();
        let before = hold(world);
        let loot = world
            .get::<ShipInventory>(wreck)
            .expect("station strip: a generated wreck has a hold")
            .clone();
        let mut stacks: BTreeMap<ItemDesignId, u32> = before
            .stacks()
            .map(|(item, count)| (item.clone(), count))
            .collect();
        for (item, count) in loot.stacks() {
            *stacks.entry(item.clone()).or_default() += count;
        }
        // Panics when the loot does not fit: Strip would refuse, and the walk
        // needs a seed whose wreck it strips.
        let own = ShipInventory::new(&items, before.capacity_g(), stacks);
        let design = world
            .get::<ShipDesignSections>(wreck)
            .expect("station strip: a generated wreck keeps its design")
            .clone();
        let live = live_sections(world, wreck);
        let sections = live
            .iter()
            .map(|(_, id, share, _)| StripTarget {
                recipe: &design
                    .get(id)
                    .expect("station strip: a live section is a design section")
                    .config
                    .base
                    .build,
                health_share: *share,
            })
            .collect();
        let strip = HullStripPartner {
            living_station: false,
            owned_station: false,
            lootable: true,
            owned_hulls: 0,
            sections,
            loot: LootTake::default(),
        }
        .plan();
        let mass_g: u64 = strip
            .parts
            .iter()
            .map(|(item, count)| items.design(item).stack_mass_g(*count))
            .sum();
        let free_g = own.free_g(&items);
        info!(
            "station strip: {} sections, {:.0} s, {:?} ({mass_g} g; the hold has {free_g} g free)",
            live.len(),
            strip.duration_secs,
            strip.parts
        );
        // A strip that cannot fit holds at a section until there is room;
        // the walk would stall there, so a seed that turns this up fails here.
        assert!(
            mass_g <= u64::from(free_g),
            "station strip: the wreck's {mass_g} g of parts fit the hold's {free_g} g free"
        );
        let codes: BTreeMap<String, String> = live
            .iter()
            .map(|(_, id, ..)| {
                let code = design_section_code(&design, id).expect("a design section has a code");
                (id.clone(), code)
            })
            .collect();
        {
            let mut watch = world.resource_mut::<StripWatch>();
            watch.codes = codes;
            watch.planned = strip.parts;
            watch.start_hold = Some(own);
        }
        record_the_berth(world);
    }

    /// Record the wreck port the player is docked through: the berth a
    /// strip from this dock takes last.
    fn record_the_berth(world: &mut World) {
        let berth = partner_berth(world, wreck(world));
        let berth = world
            .get::<EntityId>(berth)
            .expect("station strip: the berth wears an id")
            .0
            .clone();
        let code = world.resource::<StripWatch>().codes[&berth].clone();
        info!("station strip: docked through {berth} ({code})");
        world.resource_mut::<StripWatch>().berth = Some(berth);
    }

    /// Open the TAB interface on the `tab` pane and let it settle.
    fn open_the_pane(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        tab: &'static str,
        pane: InterfacePaneType,
    ) -> AutopilotPlugin<GameStates> {
        tap(
            script,
            &format!("the interface key {label}"),
            "interface_toggle",
            interface_open(),
        )
        .click_named(
            &format!("station strip: open the {pane:?} pane {label}"),
            tab,
            the_interface_shows(pane),
            SESSION_SECS,
        )
        .step(format!("station strip: settle the {pane:?} pane {label}"))
        .until(frames(SETTLE_FRAMES))
        .add()
    }

    /// Open the Inventory pane, press Strip and shut the interface.
    fn start_the_strip(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        plan_shot: Option<&'static str>,
    ) -> AutopilotPlugin<GameStates> {
        let script = open_the_pane(script, label, INVENTORY_TAB, InterfacePaneType::Inventory);
        let script = match plan_shot {
            Some(shot) => shoot_step(script, shot),
            None => script,
        };
        let script = script.click_named(
            &format!("station strip: press Strip {label}"),
            INVENTORY_STRIP_BUTTON,
            when(|world| job(world).is_some()),
            SESSION_SECS,
        );
        close_the_interface(script, label)
    }

    fn the_strip_reaches(share: f32) -> Arc<Predicate> {
        when(move |world| job(world).is_some_and(|job| job.progress() >= share))
    }

    fn undock(world: &mut World) {
        let job = job(world).cloned().expect("station strip: a strip runs");
        info!(
            "station strip: undocks after {} sections at {:.0}%",
            job.taken,
            job.progress() * 100.0
        );
        let player = player(world);
        world.trigger(DockingReleaseRequest { entity: player });
    }

    fn free_and_idle() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| {
                world.get::<DockedShip>(player).is_none()
                    && world.get::<HullStripJob>(player).is_none()
            })
        })
    }

    /// Claim 1: the undock stopped the strip and kept what it took.
    fn assert_the_stop(world: &mut World) {
        let wreck = wreck(world);
        let left = live_ids(world, wreck);
        let watch = world.resource::<StripWatch>();
        let taken: Vec<&String> = watch
            .codes
            .keys()
            .filter(|id| !left.contains(*id))
            .collect();
        let taken_codes: BTreeSet<&str> =
            taken.iter().map(|id| watch.codes[*id].as_str()).collect();
        let event_codes: BTreeSet<&str> = watch
            .stripped
            .iter()
            .map(|stripped| {
                stripped.data["section"]
                    .as_str()
                    .expect("OnStripped names a section")
            })
            .collect();
        assert!(!taken.is_empty(), "station strip: the strip took a section");
        assert_eq!(
            watch.stripped.len(),
            taken.len(),
            "station strip: one OnStripped per taken section"
        );
        assert_eq!(
            taken_codes, event_codes,
            "station strip: the OnStripped sections are the ones gone"
        );
        assert!(
            left.contains(&berth_id(world)),
            "station strip: the berth still stands"
        );
        assert_eq!(
            gained(&start_hold(world), &hold(world)),
            stripped_parts(world),
            "station strip: the hold gained the parts of each OnStripped"
        );
        let taken: Vec<String> = taken_codes.iter().map(|code| code.to_string()).collect();
        nova_probe::probe_marker(
            world,
            "outcome: an undock stops a whole-ship strip and keeps what it took",
            serde_json::json!({
                "wreck": wreck_target(world).id.to_string(),
                "taken": taken,
                "left": left.len(),
            }),
        );
        info!("station strip: the undock stopped the strip after {taken:?}");
        world.resource_mut::<StripWatch>().left = left;
    }

    /// Claim 2, first half: the frozen wreck holds exactly the sections left.
    fn assert_the_retire(world: &mut World) {
        let wreck = wreck_target(world);
        assert!(
            wreck_root(world).is_none(),
            "station strip: the wreck retired with its cell"
        );
        let watch = world.resource::<StripWatch>();
        let ids: Vec<&str> = watch.codes.keys().map(String::as_str).collect();
        let expected: Vec<bool> = ids.iter().map(|id| watch.left.contains(*id)).collect();
        assert_eq!(
            frozen_sections(
                world.resource::<FrozenSectors>(),
                &wreck.id.to_string(),
                &ids
            ),
            Some(expected),
            "station strip: the frozen wreck holds the sections left and no taken one"
        );
        info!("station strip: the wreck froze part-stripped");
    }

    /// Claim 2: the thawed wreck has exactly the sections left.
    fn assert_the_thaw(world: &mut World) {
        let wreck = wreck(world);
        let left = world.resource::<StripWatch>().left.clone();
        assert_eq!(
            live_ids(world, wreck),
            left,
            "station strip: the thawed wreck has the sections left"
        );
        let crossed = world.resource::<StripWatch>().crossed;
        nova_probe::probe_marker(
            world,
            "outcome: a part-stripped wreck stays part-stripped across a retire and a thaw",
            serde_json::json!({
                "wreck": wreck_target(world).id.to_string(),
                "left": left.len(),
                "cell": wreck_target(world).cell.to_string(),
                "crossed": crossed.map(|cell| cell.to_string()),
            }),
        );
        info!(
            "station strip: the wreck thawed with {} sections",
            left.len()
        );
    }

    fn the_wreck_is_gone() -> Arc<Predicate> {
        when(|world| {
            wreck_root(world).is_none()
                && the_player(world).is_some_and(|player| {
                    world.get::<DockedShip>(player).is_none()
                        && world.get::<HullStripJob>(player).is_none()
                })
        })
    }

    /// Claims 3, 4 and 5: the strip ran shut, took the berth last, removed
    /// the wreck, freed the player and filled the hold.
    fn assert_the_finish(world: &mut World) {
        let player = player(world);
        let watch = world.resource::<StripWatch>();
        let berth_code = watch.codes[&berth_id(world)].clone();
        let sections: Vec<String> = watch
            .stripped
            .iter()
            .map(|stripped| {
                assert_eq!(
                    stripped.data["hull"],
                    wreck_target(world).id.to_string(),
                    "station strip: OnStripped names the wreck"
                );
                stripped.data["section"]
                    .as_str()
                    .expect("OnStripped names a section")
                    .to_string()
            })
            .collect();
        let all: BTreeSet<&str> = watch.codes.values().map(String::as_str).collect();
        assert_eq!(
            sections.len(),
            all.len(),
            "station strip: one OnStripped per section"
        );
        assert_eq!(
            sections.iter().map(String::as_str).collect::<BTreeSet<_>>(),
            all,
            "station strip: every section came off"
        );
        assert_eq!(
            sections.last(),
            Some(&berth_code),
            "station strip: the berth port came off last"
        );
        let dropped: usize = watch.drops.iter().map(|(count, _)| count).sum();
        assert_eq!(
            dropped,
            all.len(),
            "station strip: the strip took every section off the wreck"
        );
        assert!(
            watch.drops.iter().all(|(_, unpaused)| *unpaused),
            "station strip: every section came off with the interface shut: {:?}",
            watch.drops
        );
        let frames = watch.drops.len();
        nova_probe::probe_marker(
            world,
            "outcome: a whole-ship strip runs with the HUD closed and removes the docking port last",
            serde_json::json!({ "order": sections, "berth": berth_code, "frames": frames }),
        );

        assert!(
            wreck_root(world).is_none(),
            "station strip: the wreck is gone"
        );
        assert!(world.get::<DockedShip>(player).is_none());
        assert!(world.get::<HullStripJob>(player).is_none());
        let holding: Vec<Entity> = world
            .try_query::<(Entity, &DockingConnection)>()
            .map(|mut query| {
                query
                    .iter(world)
                    .filter(|(_, connection)| {
                        connection.first_ship == player || connection.second_ship == player
                    })
                    .map(|(entity, _)| entity)
                    .collect()
            })
            .unwrap_or_default();
        assert!(
            holding.is_empty(),
            "station strip: no connection holds the player: {holding:?}"
        );
        let speed = world
            .get::<LinearVelocity>(player)
            .map_or(0.0, |velocity| velocity.0.length());
        nova_probe::probe_marker(
            world,
            "outcome: a finished strip demolishes the wreck and releases the player",
            serde_json::json!({
                "wreck": wreck_target(world).id.to_string(),
                "player_speed": speed,
            }),
        );

        let planned = world.resource::<StripWatch>().planned.clone();
        let gained = gained(&start_hold(world), &hold(world));
        assert_eq!(
            gained, planned,
            "station strip: the hold gained the whole-hull plan"
        );
        assert_eq!(
            gained,
            stripped_parts(world),
            "station strip: the hold gained the sum of every OnStripped"
        );
        let parts: BTreeMap<String, u32> = gained
            .iter()
            .map(|(item, count)| (item.to_string(), *count))
            .collect();
        nova_probe::probe_marker(
            world,
            "outcome: a finished strip's yield is in the hold",
            serde_json::json!({ "parts": parts }),
        );
        info!("station strip: the wreck is stripped whole into {parts:?}");
    }

    /// Whether any frozen record holds a body wearing `id`.
    fn frozen_holds(sectors: &FrozenSectors, id: &str) -> bool {
        sectors
            .iter()
            .any(|(_, record)| record.bodies().iter().any(|body| body.id() == Some(id)))
    }

    /// Claim 6, first half: the retired cell holds no wreck.
    fn assert_the_empty_retire(world: &mut World) {
        let id = wreck_target(world).id.to_string();
        assert!(
            !frozen_holds(world.resource::<FrozenSectors>(), &id),
            "station strip: no frozen record holds the demolished wreck"
        );
    }

    /// Claim 6: the cell streamed back without the wreck.
    fn assert_the_empty_thaw(world: &mut World) {
        assert!(
            wreck_root(world).is_none(),
            "station strip: the demolished wreck did not come back"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a wreck a strip demolished stays gone across a retire",
            serde_json::json!({
                "wreck": wreck_target(world).id.to_string(),
                "cell": wreck_target(world).cell.to_string(),
            }),
        );
        info!("station strip: the demolished wreck stayed gone");
    }

    /// Whether the interface tab named `name` shows.
    fn tab_shows(world: &mut World, name: &str) -> bool {
        world
            .query::<(&Name, &Node)>()
            .iter(world)
            .find(|(found, _)| found.as_str() == name)
            .is_some_and(|(_, node)| node.display != Display::None)
    }

    fn send_strip(world: &mut World) {
        info!("station strip: sends a strip command");
        world.write_message(HullStripCommand);
    }

    /// Claim 7: the Station tab shows, the Inventory pane shows no strip
    /// block, and the strip refused with nothing moved.
    fn assert_the_station_refusal(world: &mut World) {
        assert!(
            tab_shows(world, STATION_TAB),
            "station strip: the Station tab shows at a living station"
        );
        assert!(
            !tab_shows(world, INVENTORY_STRIP_BLOCK),
            "station strip: the Inventory pane shows no strip block at a living station"
        );
        assert!(job(world).is_none(), "station strip: no strip started");
        assert_eq!(
            hold(world),
            recorded_hold(world),
            "station strip: the refused strip moved nothing"
        );
        let count = world.resource::<StripWatch>().codes.len();
        assert_eq!(world.resource::<StripWatch>().stripped.len(), count);
        nova_probe::probe_marker(
            world,
            "outcome: a living station shows the Station tab and refuses a strip",
            serde_json::json!({
                "station": station_target(world).id.to_string(),
                "note": note(world),
            }),
        );
        info!("station strip: PASS a living station shows the Station tab and refuses a strip");
    }

    /// The walk. Every beat waits on a world value.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("station strip: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "station strip: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                SESSION_SECS,
            )
            .step("station strip: name the world and enter the seed")
            .on_enter(|world: &mut World| {
                set_field_text(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field_text(world, SEED_FIELD, &STRIP_SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "station strip: click Next",
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                BEAT_DEADLINE_SECS,
            )
            .click_named(
                "station strip: pick the start role",
                START_ROLE_BUTTON,
                pointer_released(),
                BEAT_DEADLINE_SECS,
            )
            .click_named(
                "station strip: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("station strip: the window streams in around the start")
            .until(window_is(start_cell))
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: the start docks the player at its starting station")
            .until(and(player_docked(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: undock from the starting station")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(and(player_free(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: jump to the origin, the spawn before the docked start")
            .on_enter(|world: &mut World| park_player(world, Meters3::ZERO))
            .until(window_is(|_| Some(SectorCoord::ORIGIN)))
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: choose the wreck and the station")
            .on_enter(choose_the_targets)
            .add()
            .step("station strip: jump to the wreck's side")
            .on_enter(|world: &mut World| park_beside(world, &wreck_target(world)))
            .until(and(window_is(player_cell), the_wreck_stands()))
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: berth and dock at the wreck")
            .each(|world, _, _| berth_and_dock(world, wreck(world)))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: plan the strip")
            .on_enter(plan_the_strip)
            .until(frames(1))
            .add();
        let script = start_the_strip(script, "to strip", Some(PLAN_SHOT))
            .step("station strip: the strip takes a section in flight")
            .until(when(|world| job(world).is_some_and(|job| job.taken >= 1)))
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: undock mid-strip")
            .on_enter(undock)
            .until(free_and_idle())
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: assert the stop")
            .on_enter(assert_the_stop)
            .add()
            .step("station strip: jump past the active radius")
            .on_enter(jump_out)
            .until(and(
                window_is(|world| world.resource::<StripWatch>().crossed),
                wreck_cell_frozen(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: assert the retire")
            .on_enter(assert_the_retire)
            .add()
            .step("station strip: jump back beside the wreck")
            .on_enter(|world: &mut World| park_beside(world, &wreck_target(world)))
            .until(and(window_is(player_cell), the_wreck_stands()))
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: assert the thaw")
            .on_enter(assert_the_thaw)
            .add()
            .step("station strip: berth and dock at the wreck again")
            .each(|world, _, _| berth_and_dock(world, wreck(world)))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            // The new dock may take another free port: the strip ends on it.
            .step("station strip: record the new berth")
            .on_enter(record_the_berth)
            .add();
        let script = start_the_strip(script, "to strip the rest", None)
            .step("station strip: the strip runs early")
            .until(the_strip_reaches(EARLY_SHOT_PROGRESS))
            .deadline(STRIP_SECS)
            .add();
        let script = shoot_step(script, WRECK_EARLY_SHOT)
            .step("station strip: the strip runs mid-way")
            .until(the_strip_reaches(MID_SHOT_PROGRESS))
            .deadline(STRIP_SECS)
            .add();
        let script = shoot_step(script, HUD_SHOT);
        let script = open_the_pane(
            script,
            "to watch the strip",
            INVENTORY_TAB,
            InterfacePaneType::Inventory,
        );
        let script = shoot_step(script, TAB_SHOT);
        let script = close_the_interface(script, "to finish the strip")
            .step("station strip: the strip runs late")
            .until(the_strip_reaches(LATE_SHOT_PROGRESS))
            .deadline(STRIP_SECS)
            .add();
        let script = shoot_step(script, WRECK_LATE_SHOT)
            .step("station strip: the strip ends with the wreck")
            .until(the_wreck_is_gone())
            .deadline(STRIP_SECS)
            .add()
            .step("station strip: assert the finish")
            .on_enter(assert_the_finish)
            .add()
            .step("station strip: jump past the active radius again")
            .on_enter(jump_out)
            .until(and(
                window_is(|world| world.resource::<StripWatch>().crossed),
                wreck_cell_frozen(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: assert the empty retire")
            .on_enter(assert_the_empty_retire)
            .add()
            .step("station strip: jump back to the wreck's cell")
            .on_enter(|world: &mut World| park_beside(world, &wreck_target(world)))
            .until(window_is(player_cell))
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: assert the empty thaw")
            .on_enter(assert_the_empty_thaw)
            .add()
            .step("station strip: jump to the station's side")
            .on_enter(|world: &mut World| park_beside(world, &station_target(world)))
            .until(and(window_is(player_cell), the_station_stands()))
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: berth and dock at the station")
            .each(|world, _, _| berth_and_dock(world, station(world)))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: record the hold")
            .on_enter(record_hold)
            .add();
        open_the_pane(
            script,
            "at the station",
            INVENTORY_TAB,
            InterfacePaneType::Inventory,
        )
        .step("station strip: strip the station")
        .on_enter(send_strip)
        .until(when(|world| note(world) == Some(LIVING_STATION_NOTE)))
        .deadline(SESSION_SECS)
        .add()
        .step("station strip: assert the station refusal")
        .on_enter(assert_the_station_refusal)
        .add()
    }
}
