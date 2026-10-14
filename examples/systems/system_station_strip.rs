//! system_station_strip: a section stripped off a docked generated derelict
//! breaks down into recipe parts in the player's hold, stays gone across a
//! retire and a thaw, and a strip is refused twice over the same section and
//! on a hull that is not lootable.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed the way the Create World modal does, and lets the window stream in.
//! It picks the nearest generated derelict ship (lootable, with a free live
//! port) and the nearest living Player or Neutral station. The player jumps
//! beside each and is put on a berth at its collar each frame until the dock
//! takes; the walk never flies an approach (`system_station_docking` does).
//!
//! The first strip goes through the Ship pane's Strip button, the code path a
//! player clicks; it strips the section with the longest strip the rule
//! allows. The second strip of that section, the strip that is stopped by an
//! undock and the strip at the living station are `SectionStripCommand`s,
//! the message a Strip button writes, so a stale or a refused click is the
//! same message.
//!
//! DISCLOSED FIXTURE STATE: while a strip runs, `Time<Virtual>` runs at a
//! quarter speed, so the software rasterizer's long frames leave a few frames
//! to shoot the progress in; it goes back to full speed after the undock.
//! The berth points the mouse rig where it points the hull, so an undock does
//! not turn the hull back through the wreck. The crossing JUMPS the player
//! past the active radius from the wreck, so its cell retires and freezes,
//! then back beside it, so it thaws.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a docked derelict's section strips into recipe parts` | the section is gone from the wreck; the hold gained each recipe part times the strip yield times the section's health share, rounded down; `OnStripped` fired once with the wreck, the section code and the parts |
//! | 2 | `outcome: a second strip of a stripped section refuses` | the same command again shows `strip: <code> is already gone`, starts no strip and moves nothing |
//! | 3 | `outcome: a strip stops with no change on an undock` | a strip running when the player undocks is gone, its section still stands and the hold did not move |
//! | 4 | `outcome: a stripped section stays gone across a retire and a thaw` | the wreck's frozen record holds no state for the stripped section and holds the stopped one; the thawed wreck has no live stripped section and has the stopped one |
//! | 5 | `outcome: a hull that is not lootable refuses a strip` | docked at a living station, the strip list is hidden and a strip command shows the not-lootable reason, starts no strip and moves nothing |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=3600 \
//!   cargo run --example system_station_strip --features debug
//! # look for: `station strip: PASS a hull that is not lootable refuses a strip`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! Add `NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=<dir>` to shoot the strip list as
//! `station-strip-list.png`, the HUD chip of the running strip as
//! `station-strip-hud.png` and the list's progress line as
//! `station-strip-progress.png`.

use bevy::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "system_station_strip")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed: dock at a generated derelict, strip a section from the \
             Ship pane into recipe parts, refuse a second strip, stop one with an undock, leave \
             the wreck's cell and come back, then refuse a strip at a living station. \
             Autopilot-only correctness range",
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
        // The fleet's 1920x1080, the size the list is shot at.
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
    use std::{collections::BTreeMap, sync::Arc};

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use nova_protocol::{
        nova_debug::harness::{
            press_action, release_action, shoot, AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS,
        },
        nova_events::engine::GameEvent,
        nova_interface::{
            pane::InterfacePaneType,
            ship::{
                design_section_code, SectionStripCommand, ShipRuntime, STRIP_BUTTON_PREFIX,
                STRIP_LIST, STRIP_PROGRESS,
            },
        },
        prelude::*,
    };
    use nova_ui::widget::TextFieldValue;
    use nova_world::prelude::{
        desired_sectors, FrozenBodyType, FrozenSectors, SectorRoot, WorldConfig,
    };

    use super::*;

    /// The seed the range plays: its window around the spawn holds a
    /// generated derelict ship and a living Player or Neutral station.
    const STRIP_SEED: u32 = 115;

    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const WORLD_NAME: &str = "Station Strip World";
    const SEED_FIELD: &str = "World Seed Field";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";

    /// The Ship pane's side panel, which scrolls under the wheel.
    const SHIP_PANEL: &str = "ShipPanel";

    /// The PNGs the capture path shoots.
    const LIST_SHOT: &str = "station-strip-list.png";
    const HUD_SHOT: &str = "station-strip-hud.png";
    const PROGRESS_SHOT: &str = "station-strip-progress.png";

    /// How far from a target the player parks before the berth.
    const PARK_DISTANCE: Meters = Meters(2_000.0);
    /// The gap the berth leaves between the two port faces, engine units:
    /// half a cell, inside the 10 m capture distance.
    const BERTH_GAP: f32 = 0.5;
    /// Where a port's face sits in its own frame, as the docking verb reads it.
    const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);
    /// How many cells past the active radius the crossing lands from the
    /// wreck, so its cell retires.
    const CROSSING_MARGIN: i32 = 2;

    /// Seconds a load, a stream or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Frames the Ship pane gets to build its panel and lists.
    const SETTLE_FRAMES: u32 = 10;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;
    /// The disclosed virtual clock speed while a strip runs.
    const STRIP_CLOCK_SPEED: f32 = 0.25;
    /// The share of the first strip done before the HUD is shot.
    const HUD_SHOT_PROGRESS: f32 = 0.2;

    /// The not-lootable refusal line, as the Ship pane writes it.
    const NOT_LOOTABLE_NOTE: &str = "strip: the docked hull is not a wreck you may loot";

    /// One target the walk docks at.
    #[derive(Clone, Debug)]
    struct Target {
        id: GeneratedBodyId,
        cell: SectorCoord,
        position: Meters3,
    }

    /// A section the walk strips, as planned before the strip.
    #[derive(Clone, Debug)]
    struct Planned {
        id: String,
        code: String,
        parts: BTreeMap<ItemDesignId, u32>,
    }

    /// What the run saw and recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct StripWatch {
        /// Every strip event's payload.
        stripped: Vec<serde_json::Value>,
        wreck: Option<Target>,
        station: Option<Target>,
        /// The section the Strip button strips.
        first: Option<Planned>,
        /// The section whose strip the undock stops.
        stopped: Option<Planned>,
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
            app.add_observer(|event: On<GameEvent>, mut watch: ResMut<StripWatch>| {
                if event.name() != "onstripped" {
                    return;
                }
                let data = event.info().data.clone().unwrap_or_default();
                info!("station strip: onstripped {data}");
                watch.stripped.push(data);
            });
        }
    }

    /// Hold the virtual clock's clamp at [`WALK_CLOCK_SECS`] every frame: a
    /// scenario load hands `Time<Virtual>` back at its default.
    fn hold_the_walk_clock(mut time: ResMut<Time<Virtual>>) {
        let clamp = std::time::Duration::from_secs_f32(WALK_CLOCK_SECS);
        if time.max_delta() != clamp {
            time.set_max_delta(clamp);
        }
    }

    fn set_clock_speed(world: &mut World, speed: f32) {
        world
            .resource_mut::<Time<Virtual>>()
            .set_relative_speed(speed);
        info!("station strip: fixture sets the virtual clock to {speed}x");
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

    fn first(world: &World) -> Planned {
        world
            .resource::<StripWatch>()
            .first
            .clone()
            .expect("station strip: the first strip is planned")
    }

    fn stopped(world: &World) -> Planned {
        world
            .resource::<StripWatch>()
            .stopped
            .clone()
            .expect("station strip: the stopped strip is planned")
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

    fn job(world: &World) -> Option<&SectionStripJob> {
        world.get::<SectionStripJob>(the_player(world)?)
    }

    fn note(world: &World) -> Option<&str> {
        world.get_resource::<ShipRuntime>()?.note()
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

    fn has_live_section(world: &World, root: Entity, id: &str) -> bool {
        live_sections(world, root)
            .iter()
            .any(|(_, live, ..)| live == id)
    }

    fn world_shape(world: &World) -> (Meters, i32) {
        let config = world.resource::<WorldConfig<NovaLayeredWorld>>();
        (config.sector_edge, config.active_radius)
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

    fn the_note_is(expected: impl Fn(&World) -> String + Send + Sync + 'static) -> Arc<Predicate> {
        when(move |world| note(world) == Some(expected(world).as_str()))
    }

    /// Whether the named node lies inside the Ship panel's box, where a
    /// click lands on it and a shot shows it.
    fn in_panel(world: &World, name: &str) -> bool {
        match (ui_node_rect(world, name), ui_node_rect(world, SHIP_PANEL)) {
            (Some(node), Some(panel)) => node.min.y >= panel.min.y && node.max.y <= panel.max.y,
            _ => false,
        }
    }

    /// Whether the named node sits below the Ship panel's bottom edge.
    fn below_panel(world: &World, name: &str) -> bool {
        match (ui_node_rect(world, name), ui_node_rect(world, SHIP_PANEL)) {
            (Some(node), Some(panel)) => node.max.y > panel.max.y,
            _ => true,
        }
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

    /// Park [`PARK_DISTANCE`] from `target` toward the spawn.
    fn park_beside(world: &mut World, target: &Target) {
        let toward_spawn = (-target.position.get()).normalize_or(Vec3::X);
        park_player(
            world,
            Meters3(target.position.get() + toward_spawn * PARK_DISTANCE.get()),
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
        let face_rot = rotation * port_rot;
        let face = position + rotation * port_at + face_rot * PORT_FACE;
        let axis = face_rot * Vec3::NEG_Z;
        let (own_at, own_rot) = local_pose(world, player_port, player);
        let turn = Quat::from_rotation_arc((own_rot * Vec3::NEG_Z).normalize(), -axis);
        let face_offset = turn * own_at + turn * own_rot * PORT_FACE;
        let at = face + axis * BERTH_GAP - face_offset;
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

    /// Open the TAB interface on the Ship pane and let it settle.
    fn open_the_ship_pane(
        script: AutopilotPlugin<GameStates>,
        label: &str,
    ) -> AutopilotPlugin<GameStates> {
        tap(
            script,
            &format!("the interface key {label}"),
            "interface_toggle",
            interface_open(),
        )
        .click_named(
            &format!("station strip: open the Ship pane {label}"),
            "InterfaceTabShip",
            the_interface_shows(InterfacePaneType::Ship),
            SESSION_SECS,
        )
        .step(format!("station strip: settle the schematic {label}"))
        .until(frames(SETTLE_FRAMES))
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

    /// Scroll the Ship panel until the node `shown` names is inside it, then
    /// settle.
    fn scroll_into_view(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        shown: impl Fn(&World) -> String + Send + Sync + Clone + 'static,
    ) -> AutopilotPlugin<GameStates> {
        let seen = shown.clone();
        script
            .step(format!("station strip: scroll {label} into view"))
            .each(move |world, _, frame| {
                keep_hovering_named(SHIP_PANEL)(world);
                let name = shown(world);
                if frame % 2 == 1 && !in_panel(world, &name) {
                    let lines = if below_panel(world, &name) { -1.0 } else { 1.0 };
                    scroll_lines(lines)(world);
                }
            })
            .until(when(move |world| in_panel(world, &seen(world))))
            .deadline(SESSION_SECS)
            .add()
            .step(format!("station strip: settle the scroll {label}"))
            .until(frames(SETTLE_FRAMES))
            .add()
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

    /// Plan every strip the rule allows on the docked wreck. The first strip
    /// is the longest, so the strip runs over the most frames; the stopped
    /// one is the next.
    fn plan_the_strips(world: &mut World) {
        let wreck = wreck(world);
        let berth = partner_berth(world, wreck);
        let items = world.resource::<GameItems>().clone();
        let own = hold(world);
        let design = world
            .get::<ShipDesignSections>(wreck)
            .expect("station strip: a generated wreck keeps its design")
            .clone();
        let live = live_sections(world, wreck);
        let graph: Vec<(Entity, &[Entity])> = live
            .iter()
            .map(|(section, _, _, neighbours)| (*section, neighbours.as_slice()))
            .collect();
        let mut allowed: Vec<(f32, Planned)> = Vec::new();
        for (section, id, share, _) in &live {
            let recipe = &design
                .get(id)
                .expect("station strip: a live section is a design section")
                .config
                .base
                .build;
            let code = design_section_code(&design, id).expect("a design section has a code");
            let planned = plan_section_strip(
                &items,
                Some(true),
                false,
                Some(StripTarget {
                    recipe,
                    health_share: *share,
                    berth: *section == berth,
                    splits_hull: strip_splits_hull(*section, graph.iter().copied()),
                }),
                &own,
            );
            info!(
                "station strip: {code} ('{id}', {:.0}% health): {planned:?}",
                share * 100.0
            );
            let Ok(strip) = planned else {
                continue;
            };
            // The claim's arithmetic, apart from the rule: each part times
            // the yield times the health share, rounded down.
            let parts: BTreeMap<ItemDesignId, u32> = recipe
                .parts
                .iter()
                .map(|(item, &count)| {
                    let returned = (f64::from(count)
                        * f64::from(recipe.strip_yield)
                        * f64::from(share.clamp(0.0, 1.0)))
                    .floor() as u32;
                    (item.clone(), returned)
                })
                .filter(|(_, count)| *count > 0)
                .collect();
            assert_eq!(
                strip.parts, parts,
                "station strip: the plan gives recipe x yield x health for {code}"
            );
            allowed.push((
                strip.duration_secs,
                Planned {
                    id: id.clone(),
                    code,
                    parts,
                },
            ));
        }
        allowed.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.id.cmp(&b.1.id)));
        let mut allowed = allowed.into_iter().map(|(_, planned)| planned);
        let first = allowed
            .next()
            .expect("station strip: the wreck has a section the rule lets strip");
        let stopped = allowed
            .next()
            .expect("station strip: the wreck has a second section the rule lets strip");
        info!(
            "station strip: strips {} for {:?}, stops {}",
            first.code, first.parts, stopped.code
        );
        record_hold(world);
        let mut watch = world.resource_mut::<StripWatch>();
        watch.first = Some(first);
        watch.stopped = Some(stopped);
    }

    fn strip_button(world: &World) -> String {
        format!("{STRIP_BUTTON_PREFIX}{}", first(world).code)
    }

    /// The first strip ended: the section is gone and an event fired.
    fn the_first_strip_ends() -> Arc<Predicate> {
        when(|world| {
            job(world).is_none()
                && !world.resource::<StripWatch>().stripped.is_empty()
                && wreck_root(world)
                    .is_some_and(|wreck| !has_live_section(world, wreck, &first(world).id))
        })
    }

    /// Claim 1: the section is gone, the hold gained the parts, and one
    /// `OnStripped` names the wreck, the code and the parts.
    fn assert_the_strip(world: &mut World) {
        let wreck = wreck(world);
        let first = first(world);
        let before = recorded_hold(world);
        let after = hold(world);
        assert!(
            !has_live_section(world, wreck, &first.id),
            "station strip: {} is gone from the wreck",
            first.code
        );
        let mut items: std::collections::BTreeSet<ItemDesignId> =
            before.stacks().map(|(item, _)| item.clone()).collect();
        items.extend(after.stacks().map(|(item, _)| item.clone()));
        for item in &items {
            let gained = after.count(item) - before.count(item).min(after.count(item));
            assert_eq!(
                gained,
                first.parts.get(item).copied().unwrap_or(0),
                "station strip: the hold gained recipe x yield x health of '{item}'"
            );
            assert!(
                after.count(item) >= before.count(item),
                "station strip: the strip took no '{item}'"
            );
        }
        let watch = world.resource::<StripWatch>();
        assert_eq!(watch.stripped.len(), 1, "station strip: one OnStripped");
        let event = &watch.stripped[0];
        assert_eq!(event["hull"], wreck_target(world).id.to_string());
        assert_eq!(event["section"], first.code);
        let parts: BTreeMap<String, u32> = first
            .parts
            .iter()
            .map(|(item, count)| (item.to_string(), *count))
            .collect();
        assert_eq!(
            event["parts"],
            serde_json::to_value(&parts).expect("parts serialize"),
            "station strip: OnStripped carries the parts"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a docked derelict's section strips into recipe parts",
            serde_json::json!({
                "wreck": wreck_target(world).id.to_string(),
                "section": first.code,
                "parts": parts,
            }),
        );
        info!("station strip: {} stripped into {parts:?}", first.code);
        record_hold(world);
    }

    fn send_strip(world: &mut World, section: String) {
        info!("station strip: sends a strip of '{section}'");
        world.write_message(SectionStripCommand { section });
    }

    /// Claim 2: the second strip refused and moved nothing.
    fn assert_the_second_refusal(world: &mut World) {
        let first = first(world);
        assert!(job(world).is_none(), "station strip: no strip started");
        assert_eq!(
            hold(world),
            recorded_hold(world),
            "station strip: the refused strip moved nothing"
        );
        assert_eq!(
            world.resource::<StripWatch>().stripped.len(),
            1,
            "station strip: the refused strip reported nothing"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a second strip of a stripped section refuses",
            serde_json::json!({ "section": first.code, "note": note(world) }),
        );
        info!("station strip: the second strip of {} refused", first.code);
    }

    /// The stopped strip runs with the clock.
    fn the_stopped_strip_runs() -> Arc<Predicate> {
        when(|world| job(world).is_some_and(|job| job.elapsed_secs > 0.0))
    }

    fn undock(world: &mut World) {
        let job = job(world).cloned().expect("station strip: a strip runs");
        assert_eq!(job.section_id, stopped(world).id);
        info!(
            "station strip: undocks with {} at {:.0}%",
            job.code,
            job.progress() * 100.0
        );
        let player = player(world);
        world.trigger(DockingReleaseRequest { entity: player });
    }

    fn free_and_idle() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| {
                world.get::<DockedShip>(player).is_none()
                    && world.get::<SectionStripJob>(player).is_none()
            })
        })
    }

    /// Claim 3: the undock stopped the strip with no change.
    fn assert_the_stop(world: &mut World) {
        let stopped = stopped(world);
        assert!(
            has_live_section(world, wreck(world), &stopped.id),
            "station strip: the stopped section still stands"
        );
        assert_eq!(
            hold(world),
            recorded_hold(world),
            "station strip: the stopped strip moved nothing"
        );
        assert_eq!(
            world.resource::<StripWatch>().stripped.len(),
            1,
            "station strip: the stopped strip reported nothing"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a strip stops with no change on an undock",
            serde_json::json!({ "section": stopped.code }),
        );
        info!(
            "station strip: the undock stopped the strip of {}",
            stopped.code
        );
        set_clock_speed(world, 1.0);
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

    /// Claim 4, first half: the frozen wreck holds no stripped section.
    fn assert_the_retire(world: &mut World) {
        let wreck = wreck_target(world);
        let (first, stopped) = (first(world), stopped(world));
        assert!(
            wreck_root(world).is_none(),
            "station strip: the wreck retired with its cell"
        );
        assert_eq!(
            frozen_sections(
                world.resource::<FrozenSectors>(),
                &wreck.id.to_string(),
                &[&first.id, &stopped.id]
            ),
            Some(vec![false, true]),
            "station strip: the frozen wreck holds the stopped section and not the stripped one"
        );
        info!("station strip: the wreck froze without {}", first.code);
    }

    /// Claim 4: the thawed wreck has no stripped section.
    fn assert_the_thaw(world: &mut World) {
        let wreck = wreck(world);
        let (first, stopped) = (first(world), stopped(world));
        assert!(
            !has_live_section(world, wreck, &first.id),
            "station strip: the thawed wreck has no {}",
            first.code
        );
        assert!(
            has_live_section(world, wreck, &stopped.id),
            "station strip: the thawed wreck keeps {}",
            stopped.code
        );
        let watch = world.resource::<StripWatch>();
        let crossed = watch.crossed;
        nova_probe::probe_marker(
            world,
            "outcome: a stripped section stays gone across a retire and a thaw",
            serde_json::json!({
                "wreck": wreck_target(world).id.to_string(),
                "section": first.code,
                "cell": wreck_target(world).cell.to_string(),
                "crossed": crossed.map(|cell| cell.to_string()),
            }),
        );
        info!("station strip: the wreck thawed without {}", first.code);
    }

    /// A live section id of the docked station, for the refused strip.
    fn station_section(world: &World) -> String {
        live_sections(world, station(world))
            .into_iter()
            .map(|(_, id, ..)| id)
            .min()
            .expect("station strip: the station has sections")
    }

    fn strip_list_hidden(world: &mut World) -> bool {
        world
            .query::<(&Name, &Node)>()
            .iter(world)
            .find(|(name, _)| name.as_str() == STRIP_LIST)
            .is_some_and(|(_, node)| node.display == Display::None)
    }

    /// Claim 5: a living station refuses the strip and nothing moves.
    fn assert_the_station_refusal(world: &mut World) {
        assert!(
            strip_list_hidden(world),
            "station strip: the strip list hides at a living station"
        );
        assert!(job(world).is_none(), "station strip: no strip started");
        assert_eq!(
            hold(world),
            recorded_hold(world),
            "station strip: the refused strip moved nothing"
        );
        assert_eq!(world.resource::<StripWatch>().stripped.len(), 1);
        nova_probe::probe_marker(
            world,
            "outcome: a hull that is not lootable refuses a strip",
            serde_json::json!({
                "station": station_target(world).id.to_string(),
                "note": note(world),
            }),
        );
        info!("station strip: PASS a hull that is not lootable refuses a strip");
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
                ui_node_present(CREATE_WORLD_BUTTON),
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
                "station strip: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("station strip: the window streams in around the spawn")
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
            .step("station strip: plan the strips")
            .on_enter(plan_the_strips)
            .until(frames(1))
            .add();
        let script = open_the_ship_pane(script, "to strip");
        let script = scroll_into_view(script, "the Strip button", strip_button);
        // The button's name holds the section code, known only once the wreck
        // is docked, so the click is spelled out rather than `click_named`.
        let script = shoot_step(script, LIST_SHOT)
            .step("station strip: aim at the Strip button")
            .each(|world, _, _| {
                let name = strip_button(world);
                keep_hovering_named(name)(world);
            })
            .until(when(|world| pointer_over_node(strip_button(world))(world)))
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: press Strip")
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: release Strip")
            .on_enter(|world: &mut World| {
                set_clock_speed(world, STRIP_CLOCK_SPEED);
                release_mouse(MouseButton::Left)(world);
            })
            .until(when(|world| job(world).is_some()))
            .deadline(SESSION_SECS)
            .add();
        let script = close_the_interface(script, "to run the strip")
            .step("station strip: the strip runs in flight")
            .until(when(|world| {
                job(world).is_some_and(|job| job.progress() >= HUD_SHOT_PROGRESS)
            }))
            .deadline(SESSION_SECS)
            .add();
        let script = shoot_step(script, HUD_SHOT);
        let script = open_the_ship_pane(script, "to watch the strip");
        let script = scroll_into_view(script, "the strip progress", |_| STRIP_PROGRESS.to_string());
        let script = shoot_step(script, PROGRESS_SHOT);
        let script = close_the_interface(script, "to finish the strip")
            .step("station strip: the strip ends")
            .until(the_first_strip_ends())
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: assert the strip")
            .on_enter(assert_the_strip)
            .add();
        let script = open_the_ship_pane(script, "to strip again")
            .step("station strip: strip the stripped section again")
            .on_enter(|world: &mut World| send_strip(world, first(world).id))
            .until(the_note_is(|world| {
                format!("strip: {} is already gone", first(world).code)
            }))
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: assert the second refusal")
            .on_enter(assert_the_second_refusal)
            .add()
            .step("station strip: start the strip the undock stops")
            .on_enter(|world: &mut World| send_strip(world, stopped(world).id))
            .until(when(|world| job(world).is_some()))
            .deadline(SESSION_SECS)
            .add();
        let script = close_the_interface(script, "to run the stopped strip")
            .step("station strip: the stopped strip runs")
            .until(the_stopped_strip_runs())
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
        open_the_ship_pane(script, "at the station")
            .step("station strip: strip a station section")
            .on_enter(|world: &mut World| send_strip(world, station_section(world)))
            .until(the_note_is(|_| NOT_LOOTABLE_NOTE.to_string()))
            .deadline(SESSION_SECS)
            .add()
            .step("station strip: assert the station refusal")
            .on_enter(assert_the_station_refusal)
            .add()
    }
}
