//! system_station_storage: a living generated station trades with the docked
//! player and keeps what the player stores, through a retire, a thaw and a
//! save.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed the way the Create World modal does, and lets the window stream in.
//! It picks the nearest generated station that is living, not on the Enemy
//! side, offers Trade and Storage, and stands outside the spawn's cell. THE
//! HOLD AND THE BALANCE ARE DISCLOSED FIXTURE STATE: the player's hold is set
//! to 12 section panels and 3 rations and its balance to 1,000 cr, so the
//! walk does not depend on what the warship spawns with. The player jumps
//! beside the station and is put on a berth at its collar each frame until the
//! dock takes; the walk never flies the approach (`system_station_docking`
//! does).
//!
//! Every move after the dock goes through the Inventory pane by clicks: buy a
//! turret barrel, sell a ration, store five panels and the barrel. The player
//! undocks and jumps away until the station's cell retires, and the cell's
//! record must hold the station's storage. The jump back thaws a new station
//! entity; the player docks again and withdraws both stacks, and the counts
//! must be the ones stored. Then a store of eleven panels (110 kg) must refuse
//! on the form with Confirm disabled, two panels are stored, and the player
//! leaves to the main menu. The leave save is read back off disk through
//! [`open_world`], and the world is loaded again in this process through the
//! Load screen: the thawed station must hold the two panels.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the docked player buys and sells at a living station` | a Buy and a Sell through the pane move items and credits by the item's ask and bid, and fire `OnTraded` once each |
//! | 2 | `outcome: the player stores parts in the station's storage` | a Store through the pane moves the stacks from the hold to the station's storage with no credits, and fires `OnStored` once per stack with the station as subject |
//! | 3 | `outcome: the retired station's record holds its storage` | the retired cell's record holds the station with the stored stacks |
//! | 4 | `outcome: the thawed station gives back the same stacks` | the thawed station is a new entity holding the stored stacks; a Withdraw of each moves it back, the hold counts are the ones before the store, and `OnWithdrawn` fires once per stack |
//! | 5 | `outcome: a store past the cap refuses on the form` | eleven panels against an empty 100 kg storage show `Refused: storage full, 100 kg free` with Confirm disabled, and nothing moves |
//! | 6 | `outcome: the leave save holds the station's storage` | the leave save, read with [`open_world`], holds the station in its cell with the two stored panels |
//! | 7 | `outcome: a loaded world gives the station its storage back` | after Load, the station stands under its id with the two stored panels |
//!
//! ONE SUBJECT: station storage and trade through the pane, kept across a
//! retire and a save. The planners' refusal order is unit-tested in
//! `nova_gameplay::inventory`; the pane's gate per partner in
//! `nova_interface::inventory::tests`.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=1800 \
//!   cargo run --example system_station_storage --features debug
//! # look for: `station storage: PASS the loaded station holds 2 SectionPanel`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "system_station_storage")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed, then dock at a living generated station, buy, sell and \
             store through the Inventory pane, retire and thaw the station, withdraw, refuse an \
             over-cap store, and check the storage through a save and a Load. Autopilot-only \
             correctness range",
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
        // No frame-time claim: the range holds counts, credits and records.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::StorageWatchPlugin);
        app.add_plugins(walk::script());
    }

    app.run()
}

#[cfg(feature = "debug")]
mod walk {
    use std::{collections::BTreeMap, sync::Arc};

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use bevy::ui::InteractionDisabled;
    use nova_protocol::{
        nova_debug::harness::{press_action, release_action, AutopilotPlugin, Predicate},
        nova_events::engine::GameEvent,
        nova_interface::pane::InterfacePaneType,
        prelude::*,
    };
    use nova_ui::widget::TextFieldValue;
    use nova_world::prelude::{
        desired_sectors, FrozenBodyType, FrozenSector, FrozenSectors, SectorRoot, WorldConfig,
    };

    use super::*;

    /// The seed the range plays. Its window around the spawn holds one
    /// station, a living Neutral waystation in cell (1, 0, 0), 48 km out.
    /// Seeds 6 to 9 and 11 to 13 hold only an Enemy, a dead or a spawn-cell
    /// station.
    const STATION_SEED: u32 = 10;

    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const WORLD_NAME: &str = "Storage World";
    const SEED_FIELD: &str = "World Seed Field";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const PAUSE_EXIT_BUTTON: &str = "Pause Exit Button";
    const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
    const LOAD_BUTTON: &str = "Load Button";
    const LOAD_WORLD_BUTTON: &str = "Load World Button";

    const QUANTITY_FIELD: &str = "InventoryDraftQuantity";
    const DRAFT_SUMMARY: &str = "InventoryDraftSummary";
    const CONFIRM: &str = "InventoryDraftConfirm";

    /// Item ids the walk moves.
    const PANEL: &str = "SectionPanel";
    const RATIONS: &str = "Rations";
    const BARREL: &str = "TurretBarrel";

    /// The disclosed fixture hold and balance.
    const HOLD_PANELS: u32 = 12;
    const HOLD_RATIONS: u32 = 3;
    const PLAYER_CREDITS: u32 = 1_000;

    /// The base catalog's ask for a barrel and bid for a ration.
    const BARREL_ASK: u32 = 60;
    const RATIONS_BID: u32 = 6;

    /// What the first visit stores, and what the second stores for the save.
    const STORED_PANELS: u32 = 5;
    const SAVED_PANELS: u32 = 2;
    /// A store past the empty waystation's 100 kg cap: 110 kg of panels.
    const OVER_CAP_PANELS: u32 = 11;
    const OVER_CAP_REFUSAL: &str = "Refused: storage full, 100 kg free";

    /// How far from the station the player parks before the berth.
    const PARK_DISTANCE: Meters = Meters(2_000.0);
    /// The gap the berth leaves between the two port faces, engine units:
    /// half a cell, inside the 10 m capture distance.
    const BERTH_GAP: f32 = 0.5;
    /// Where a port's face sits in its own frame, as the docking verb reads it.
    const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);

    /// Seconds a load, a stream or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Seconds a click gets.
    const BEAT_SECS: f32 = 60.0;
    /// Frames a pane gets to draw its rows or its form.
    const SETTLE_FRAMES: u32 = 4;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;

    /// What the run saw and recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct StorageWatch {
        /// Every trade and storage event, by name, with its payload.
        events: Vec<(String, serde_json::Value)>,
        /// The chosen station's typed id.
        id: Option<GeneratedBodyId>,
        /// The cell it stands in.
        cell: Option<SectorCoord>,
        /// The cell the retire jump lands in.
        away: Option<SectorCoord>,
        /// Where it stood before the retire.
        position: Option<Meters3>,
        /// The entity that wore the id before the retire.
        before: Option<Entity>,
        /// The player's panels and barrels right before the first store.
        before_store: Option<(u32, u32)>,
        /// The player's panels and barrels right after the two withdraws.
        after_withdraw: Option<(u32, u32)>,
        /// The storage right before the retire.
        stored: Option<BTreeMap<String, u32>>,
    }

    impl StorageWatch {
        fn named(&self, name: &str) -> Vec<&serde_json::Value> {
            self.events
                .iter()
                .filter(|(event, _)| event == name)
                .map(|(_, data)| data)
                .collect()
        }
    }

    pub(super) struct StorageWatchPlugin;

    impl Plugin for StorageWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<StorageWatch>();
            app.add_systems(Update, hold_the_walk_clock);
            app.add_observer(|event: On<GameEvent>, mut watch: ResMut<StorageWatch>| {
                let name = event.name();
                if !matches!(name, "ontraded" | "onstored" | "onwithdrawn") {
                    return;
                }
                let data = event.info().data.clone().unwrap_or_default();
                info!("station storage: {name} {data}");
                watch.events.push((name.to_string(), data));
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
        the_player(world).expect("station storage: exactly one player ship")
    }

    /// The entity wearing the chosen station's id, if one does.
    fn station_root(world: &World) -> Option<Entity> {
        let id = world.resource::<StorageWatch>().id.clone()?;
        let mut query = world.try_query::<(Entity, &GeneratedBodyId)>()?;
        query
            .iter(world)
            .find(|(_, worn)| **worn == id)
            .map(|(entity, _)| entity)
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("station storage: the station stands")
    }

    fn station_id(world: &World) -> GeneratedBodyId {
        world
            .resource::<StorageWatch>()
            .id
            .clone()
            .expect("station storage: the station is chosen")
    }

    fn held(world: &World, root: Entity, item: &str) -> u32 {
        world
            .get::<ShipInventory>(root)
            .expect("station storage: every ship carries a hold")
            .count(&item.into())
    }

    fn credits(world: &World, root: Entity) -> u32 {
        world
            .get::<ShipCredits>(root)
            .expect("station storage: every ship carries a balance")
            .0
    }

    fn storage_map(storage: &StationStorage) -> BTreeMap<String, u32> {
        storage
            .stacks()
            .map(|(item, count)| (item.to_string(), count))
            .collect()
    }

    fn stored(world: &World) -> BTreeMap<String, u32> {
        storage_map(
            world
                .get::<StationStorage>(station(world))
                .expect("station storage: a station root carries StationStorage"),
        )
    }

    fn stored_count(world: &World, item: &str) -> u32 {
        stored(world).get(item).copied().unwrap_or(0)
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

    fn the_station_stands() -> Arc<Predicate> {
        when(|world| station_root(world).is_some())
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

    fn the_interface_shows(pane: Option<InterfacePaneType>) -> Arc<Predicate> {
        when(move |world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
                && pane.is_none_or(|pane| {
                    world
                        .get_resource::<InterfacePaneType>()
                        .is_some_and(|shown| *shown == pane)
                })
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
        info!("station storage: the player jumps to {position:?}");
    }

    /// Where the player parks: [`PARK_DISTANCE`] from the station toward the
    /// spawn, in the station's own cell.
    fn park_beside(world: &mut World) {
        let watch = world.resource::<StorageWatch>();
        let position = watch.position.expect("the station is chosen");
        let toward_spawn = (-position.get()).normalize_or_zero();
        let beside = Meters3(position.get() + toward_spawn * PARK_DISTANCE.get());
        let (edge, _) = world_shape(world);
        assert_eq!(
            SectorCoord::containing(beside, edge),
            watch.cell.expect("the station is chosen"),
            "station storage: the park point stands in the station's cell"
        );
        park_player(world, beside);
    }

    /// `section`'s position and rotation in `root`'s frame, by its mount chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("station storage: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("station storage: a section's chain reaches its root")
                .parent();
        }
        (position, rotation)
    }

    /// The lowest free, live docking port mounted on `root`.
    fn free_port(world: &mut World, root: Entity) -> Entity {
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
            .unwrap_or_else(|| panic!("station storage: {root:?} has no free docking port"))
    }

    /// Put the player on the berth at the station's collar: its port facing
    /// the station's across [`BERTH_GAP`], moving and turning with the
    /// station, then ask to dock. Every frame until the dock takes, so a
    /// station's own drift or the player's attitude hold cannot carry the
    /// player off the berth between frames.
    fn berth_and_dock(world: &mut World) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let station = station(world);
        let station_port = free_port(world, station);
        let player_port = free_port(world, player);
        let body = |entity: Entity| {
            (
                world
                    .get::<Position>(entity)
                    .expect("a hull has a position")
                    .0,
                world
                    .get::<Rotation>(entity)
                    .expect("a hull has a rotation")
                    .0,
                world
                    .get::<LinearVelocity>(entity)
                    .map_or(Vec3::ZERO, |v| v.0),
                world
                    .get::<AngularVelocity>(entity)
                    .map_or(Vec3::ZERO, |w| w.0),
            )
        };
        let (station_at, station_rot, station_velocity, station_spin) = body(station);
        let (port_at, port_rot) = local_pose(world, station_port, station);
        let station_face_rot = station_rot * port_rot;
        let station_face = station_at + station_rot * port_at + station_face_rot * PORT_FACE;
        let station_axis = station_face_rot * Vec3::NEG_Z;
        let (own_at, own_rot) = local_pose(world, player_port, player);
        let rotation = Quat::from_rotation_arc((own_rot * Vec3::NEG_Z).normalize(), -station_axis);
        let face_offset = rotation * own_at + rotation * own_rot * PORT_FACE;
        let position = station_face + station_axis * BERTH_GAP - face_offset;
        world.entity_mut(player).insert((
            Position(position),
            Rotation(rotation),
            LinearVelocity(station_velocity),
            AngularVelocity(station_spin),
        ));
        world.trigger(DockingConnectionRequest {
            entity: player,
            target: station,
        });
    }

    fn set_field_text(world: &mut World, name: &str, text: &str) {
        let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
        let (_, mut value) = fields
            .iter_mut(world)
            .find(|(found, _)| found.as_str() == name)
            .unwrap_or_else(|| panic!("station storage: no field named '{name}'"));
        value.0 = text.to_string();
    }

    fn node_text(world: &mut World, name: &str) -> String {
        let mut texts = world.query::<(&Name, &Text)>();
        texts
            .iter(world)
            .find(|(found, _)| found.as_str() == name)
            .map(|(_, text)| text.0.clone())
            .unwrap_or_else(|| panic!("station storage: no text named '{name}'"))
    }

    fn confirm_disabled(world: &mut World) -> bool {
        let mut buttons = world.query::<(&Name, Has<InteractionDisabled>)>();
        buttons
            .iter(world)
            .find(|(found, _)| found.as_str() == CONFIRM)
            .map(|(_, disabled)| disabled)
            .expect("station storage: the form has a Confirm")
    }

    /// Pick the nearest station that is living, not on the Enemy side, offers
    /// Trade and Storage, and stands outside the spawn's cell; choose the
    /// retire jump; set the disclosed fixture hold and balance.
    fn choose_the_station(world: &mut World) {
        let (edge, radius) = world_shape(world);
        let seen: Vec<String> = world
            .query_filtered::<(
                &GeneratedBodyId,
                Option<&StationServices>,
                Has<DerelictShipMarker>,
                Option<&Allegiance>,
            ), With<StationMarker>>()
            .iter(world)
            .map(|(id, services, derelict, allegiance)| {
                format!("{id} derelict={derelict} side={allegiance:?} services={services:?}")
            })
            .collect();
        info!(
            "station storage: the window holds {} stations: {seen:#?}",
            seen.len()
        );
        let mut stations: Vec<(f32, GeneratedBodyId, Entity)> = world
            .query_filtered::<(
                Entity,
                &GeneratedBodyId,
                &Position,
                Option<&StationServices>,
                Has<DerelictShipMarker>,
                Option<&Allegiance>,
            ), With<StationMarker>>()
            .iter(world)
            .filter(|(_, _, position, services, derelict, allegiance)| {
                station_offers(*services, *derelict, StationServiceType::Storage)
                    && station_offers(*services, *derelict, StationServiceType::Trade)
                    && *allegiance != Some(&Allegiance::Enemy)
                    && SectorCoord::containing(Meters3::from_engine(position.0), edge)
                        != SectorCoord::ORIGIN
            })
            .map(|(entity, id, position, ..)| {
                (
                    Meters3::from_engine(position.0)
                        .distance(Meters3::ZERO)
                        .get(),
                    id.clone(),
                    entity,
                )
            })
            .collect();
        stations.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let Some((distance, id, entity)) = stations.into_iter().next() else {
            panic!(
                "station storage: the window on seed {STATION_SEED} holds no living, non-Enemy \
                 station with Trade and Storage outside the spawn's cell"
            );
        };
        let position = Meters3::from_engine(world.get::<Position>(entity).expect("a body").0);
        let cell = SectorCoord::containing(position, edge);
        let axes = [cell.x, cell.y, cell.z];
        let (axis, _) = axes
            .iter()
            .enumerate()
            .max_by_key(|(_, index)| index.abs())
            .expect("three axes");
        let mut away = [0; 3];
        away[axis] = axes[axis] - axes[axis].signum() * (radius + 1);
        let away = SectorCoord::new(away[0], away[1], away[2]);
        assert!(
            !desired_sectors(away, radius).contains(&cell),
            "the window around {away} leaves {cell} out"
        );
        let stock: BTreeMap<String, u32> = world
            .get::<ShipInventory>(entity)
            .expect("a station carries a hold")
            .stacks()
            .map(|(item, count)| (item.to_string(), count))
            .collect();
        let station_cr = credits(world, entity);
        info!(
            "station storage: chose '{id}' in cell {cell}, {distance:.0} m out; stock {stock:?}, \
             {station_cr} cr"
        );

        // Disclosed fixture state: the hold and the balance the walk spends.
        let player = player(world);
        let capacity_g = world
            .get::<ShipInventory>(player)
            .expect("the player carries a hold")
            .capacity_g();
        let items = world.resource::<GameItems>().clone();
        let hold = ShipInventory::new(
            &items,
            capacity_g,
            [(PANEL.into(), HOLD_PANELS), (RATIONS.into(), HOLD_RATIONS)],
        );
        world
            .entity_mut(player)
            .insert((hold, ShipCredits(PLAYER_CREDITS)));
        info!(
            "station storage: fixture hold {HOLD_PANELS} {PANEL}, {HOLD_RATIONS} {RATIONS} in \
             {capacity_g} g, {PLAYER_CREDITS} cr"
        );

        let mut watch = world.resource_mut::<StorageWatch>();
        watch.id = Some(id);
        watch.cell = Some(cell);
        watch.away = Some(away);
        watch.position = Some(position);
        watch.before = Some(entity);
    }

    /// Press and release a key action, one frame each.
    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("station storage: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("station storage: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Open the interface on the Inventory pane.
    fn open_inventory(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "station storage: open the Inventory pane",
            "InterfaceTabInventory",
            the_interface_shows(Some(InterfacePaneType::Inventory)),
            BEAT_SECS,
        )
        .step("station storage: settle the pane")
        .until(frames(SETTLE_FRAMES))
        .add()
    }

    /// Click a row, which opens its action at one unit, and let the form draw.
    fn pick_row(script: AutopilotPlugin<GameStates>, row: &str) -> AutopilotPlugin<GameStates> {
        script.click_named(
            &format!("station storage: pick {row}"),
            row,
            frames(SETTLE_FRAMES),
            BEAT_SECS,
        )
    }

    /// Switch the open own-row draft to Store and type `quantity`.
    fn store_draft(
        script: AutopilotPlugin<GameStates>,
        quantity: u32,
    ) -> AutopilotPlugin<GameStates> {
        script
            .click_named(
                "station storage: switch the draft to Store",
                "InventoryDraftStore",
                frames(SETTLE_FRAMES),
                BEAT_SECS,
            )
            .step(format!("station storage: type {quantity}"))
            .on_enter(move |world: &mut World| {
                set_field_text(world, QUANTITY_FIELD, &quantity.to_string());
            })
            .until(frames(SETTLE_FRAMES))
            .add()
    }

    fn confirm(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script.click_named(
            &format!("station storage: confirm {label}"),
            CONFIRM,
            landed,
            BEAT_SECS,
        )
    }

    /// Claims 1 and 2: the trades and the stores moved what they should and
    /// reported once each; then undock.
    fn assert_trades_and_stores(world: &mut World) {
        let player = player(world);
        let id = station_id(world).to_string();
        let (panels, barrels) = world
            .resource::<StorageWatch>()
            .before_store
            .expect("recorded before the store");
        assert_eq!(barrels, 1, "station storage: the Buy brought one barrel");
        assert_eq!(
            held(world, player, RATIONS),
            HOLD_RATIONS - 1,
            "the Sell took one ration"
        );
        assert_eq!(
            credits(world, player),
            PLAYER_CREDITS - BARREL_ASK + RATIONS_BID,
            "the Buy paid the ask and the Sell earned the bid"
        );
        assert_eq!(held(world, player, PANEL), panels - STORED_PANELS);
        assert_eq!(held(world, player, BARREL), 0);
        let expected: BTreeMap<String, u32> =
            [(PANEL.to_string(), STORED_PANELS), (BARREL.to_string(), 1)].into();
        let storage = stored(world);
        assert_eq!(
            storage, expected,
            "station storage: the storage holds the stored stacks"
        );
        let watch = world.resource::<StorageWatch>();
        let traded = watch.named("ontraded");
        assert_eq!(traded.len(), 2, "one OnTraded per trade: {traded:?}");
        assert!(traded.iter().all(|data| data["id"] == id.as_str()));
        let stored_events = watch.named("onstored");
        assert_eq!(
            stored_events.len(),
            2,
            "one OnStored per stack: {stored_events:?}"
        );
        for (item, count) in [(PANEL, STORED_PANELS), (BARREL, 1)] {
            assert!(
                stored_events.iter().any(|data| data["id"] == id.as_str()
                    && data["other_id"] == "player"
                    && data["item"] == item
                    && data["count"] == count),
                "station storage: OnStored names the station, the player, {count} {item}: \
                 {stored_events:?}"
            );
        }
        nova_probe::probe_marker(
            world,
            "outcome: the docked player buys and sells at a living station",
            serde_json::json!({ "station": id, "credits": credits(world, player), "traded": traded.len() }),
        );
        nova_probe::probe_marker(
            world,
            "outcome: the player stores parts in the station's storage",
            serde_json::json!({ "station": id, "storage": storage }),
        );
        info!("station storage: stored {storage:?} at '{id}'");
        world.resource_mut::<StorageWatch>().stored = Some(storage);
        world.trigger(DockingReleaseRequest { entity: player });
    }

    /// Claim 3: the retired cell's record holds the station with its storage.
    fn assert_the_retire(world: &mut World) {
        let id = station_id(world);
        let watch = world.resource::<StorageWatch>();
        let cell = watch.cell.expect("chosen");
        let before = watch.before.expect("chosen");
        let expected = watch.stored.clone().expect("stored before the retire");
        assert!(
            !live_cells(world).contains(&cell),
            "the station's cell {cell} is retired"
        );
        assert!(
            world.get_entity(before).is_err(),
            "the retire despawned the station"
        );
        let record = match world.resource::<FrozenSectors>().get(cell) {
            Some(FrozenSector::Visited(bodies)) => bodies
                .iter()
                .find(|body| body.id() == Some(id.to_string().as_str()))
                .map(|body| match body.body() {
                    FrozenBodyType::Station(station) => storage_map(station.storage()),
                    other => panic!("station storage: the station froze as {other:?}"),
                })
                .expect("station storage: the cell's record holds the station"),
            other => panic!("station storage: {cell} froze as {other:?}, not a visit"),
        };
        assert_eq!(
            record, expected,
            "station storage: the record holds the stored stacks"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the retired station's record holds its storage",
            serde_json::json!({ "id": id.to_string(), "cell": cell.to_string(), "storage": record }),
        );
        info!("station storage: sector {cell} retired with storage {record:?}");
    }

    /// The thawed station is a new entity holding the stored stacks.
    fn assert_the_thawed_storage(world: &mut World) {
        let watch = world.resource::<StorageWatch>();
        let before = watch.before.expect("chosen");
        let expected = watch.stored.clone().expect("stored");
        let thawed = station(world);
        assert_ne!(thawed, before, "the thaw is a new entity");
        assert_eq!(
            stored(world),
            expected,
            "station storage: the thaw kept the storage"
        );
    }

    /// Claim 4: both stacks came back and the hold counts are the ones before
    /// the store.
    fn assert_the_withdraw(world: &mut World) {
        let id = station_id(world).to_string();
        let watch = world.resource::<StorageWatch>();
        let (panels, barrels) = watch.before_store.expect("recorded before the store");
        assert_eq!(
            watch.after_withdraw,
            Some((panels, barrels)),
            "station storage: the withdraw gave back the stored stacks"
        );
        let withdrawn = world.resource::<StorageWatch>().named("onwithdrawn");
        assert_eq!(
            withdrawn.len(),
            2,
            "one OnWithdrawn per stack: {withdrawn:?}"
        );
        for (item, count) in [(PANEL, STORED_PANELS), (BARREL, 1)] {
            assert!(
                withdrawn.iter().any(|data| data["id"] == id.as_str()
                    && data["item"] == item
                    && data["count"] == count),
                "station storage: OnWithdrawn names the station and {count} {item}: {withdrawn:?}"
            );
        }
        nova_probe::probe_marker(
            world,
            "outcome: the thawed station gives back the same stacks",
            serde_json::json!({ "station": id, "panels": panels, "barrels": barrels }),
        );
        info!("station storage: withdrew {panels} {PANEL} and {barrels} {BARREL}");
    }

    /// Claim 5: eleven panels refuse on the form, Confirm disabled.
    fn assert_the_over_cap_refusal(world: &mut World) {
        let player = player(world);
        let summary = node_text(world, DRAFT_SUMMARY);
        assert_eq!(
            summary, OVER_CAP_REFUSAL,
            "station storage: the form refuses the store"
        );
        assert!(
            confirm_disabled(world),
            "station storage: Confirm is disabled on a refusal"
        );
        assert!(stored(world).is_empty(), "nothing moved");
        let panels = held(world, player, PANEL);
        nova_probe::probe_marker(
            world,
            "outcome: a store past the cap refuses on the form",
            serde_json::json!({ "summary": summary, "panels": panels, "tried": OVER_CAP_PANELS }),
        );
        info!("station storage: '{summary}'");
    }

    /// Claim 6: the leave save holds the station with the two panels.
    fn assert_the_saved_storage(world: &mut World) {
        let id = station_id(world).to_string();
        let cell = world.resource::<StorageWatch>().cell.expect("chosen");
        let root = nova_assets::storage::worlds_root()
            .expect("station storage: the sandboxed CONFIG_ROOT gives a worlds root");
        let slug = world_slug(WORLD_NAME).expect("the world name has a slug");
        let (_folder, _lock, header, state) = open_world(
            &root,
            &slug,
            world.resource::<LoadedSectionPacks>(),
            world.resource::<GameItems>(),
            &RegisteredScenarios::from(world.resource::<GameScenarios>()),
        )
        .unwrap_or_else(|refusal| panic!("station storage: cannot open the leave save: {refusal}"));
        let saved = match state.sectors.get(cell) {
            Some(FrozenSector::Visited(bodies)) => bodies
                .iter()
                .find(|body| body.id() == Some(id.as_str()))
                .map(|body| match body.body() {
                    FrozenBodyType::Station(station) => storage_map(station.storage()),
                    other => panic!("station storage: the save holds the station as {other:?}"),
                })
                .expect("station storage: the save's cell holds the station"),
            other => panic!("station storage: the save holds {cell} as {other:?}"),
        };
        let expected: BTreeMap<String, u32> = [(PANEL.to_string(), SAVED_PANELS)].into();
        assert_eq!(
            saved, expected,
            "station storage: the save holds the stored panels"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the leave save holds the station's storage",
            serde_json::json!({
                "slug": slug,
                "generation": header.generation,
                "format": header.format,
                "storage": saved,
            }),
        );
        info!(
            "station storage: save generation {} (format {}) holds {saved:?} at '{id}'",
            header.generation, header.format
        );
    }

    /// Claim 7: the loaded station holds the two panels.
    fn assert_the_loaded_storage(world: &mut World) {
        let id = station_id(world).to_string();
        let storage = stored(world);
        let expected: BTreeMap<String, u32> = [(PANEL.to_string(), SAVED_PANELS)].into();
        assert_eq!(
            storage, expected,
            "station storage: the Load kept the storage"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a loaded world gives the station its storage back",
            serde_json::json!({ "station": id, "storage": storage }),
        );
        info!("station storage: PASS the loaded station holds {SAVED_PANELS} {PANEL}");
    }

    /// The walk. Every beat waits on a world value.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("station storage: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "station storage: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                BEAT_SECS,
            )
            .step("station storage: name the world and enter the seed")
            .on_enter(|world: &mut World| {
                set_field_text(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field_text(world, SEED_FIELD, &STATION_SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "station storage: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("station storage: the window streams in around the spawn")
            .until(window_is(|_| Some(SectorCoord::ORIGIN)))
            .deadline(SESSION_SECS)
            .add()
            .step("station storage: choose the station and set the fixture hold")
            .on_enter(choose_the_station)
            .add()
            .step("station storage: jump to the station's side")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<StorageWatch>().cell),
                the_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station storage: berth and dock")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add();
        let script = open_inventory(script);
        let script = pick_row(script, &format!("InventoryRowPartner{BARREL}"));
        let script = confirm(
            script,
            "the Buy",
            when(|world| held(world, player(world), BARREL) == 1),
        );
        let script = pick_row(script, &format!("InventoryRowOwn{RATIONS}"));
        let script = confirm(
            script,
            "the Sell",
            when(|world| held(world, player(world), RATIONS) == HOLD_RATIONS - 1),
        )
        .step("station storage: record the hold before the store")
        .on_enter(|world: &mut World| {
            let player = player(world);
            let counts = (held(world, player, PANEL), held(world, player, BARREL));
            world.resource_mut::<StorageWatch>().before_store = Some(counts);
        })
        .add();
        let script = pick_row(script, &format!("InventoryRowOwn{PANEL}"));
        let script = store_draft(script, STORED_PANELS);
        let script = confirm(
            script,
            "the panel store",
            when(|world| stored_count(world, PANEL) == STORED_PANELS),
        );
        let script = pick_row(script, &format!("InventoryRowOwn{BARREL}"));
        let script = store_draft(script, 1);
        let script = confirm(
            script,
            "the barrel store",
            when(|world| stored_count(world, BARREL) == 1),
        );
        // The pane pushed its reports under the interface pause; the tracker
        // drains them once the clock runs again.
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("station storage: the trades and stores report")
            .until(when(|world| {
                let watch = world.resource::<StorageWatch>();
                watch.named("ontraded").len() >= 2 && watch.named("onstored").len() >= 2
            }))
            .deadline(SESSION_SECS)
            .add()
            .step("station storage: assert the trades and stores, and undock")
            .on_enter(assert_trades_and_stores)
            .until(player_free())
            .deadline(SESSION_SECS)
            .add()
            .step("station storage: jump away until the station's cell retires")
            .on_enter(|world: &mut World| {
                let (edge, _) = world_shape(world);
                let away = world.resource::<StorageWatch>().away.expect("chosen");
                park_player(world, away.centre(edge));
            })
            .until(window_is(|world| world.resource::<StorageWatch>().away))
            .deadline(SESSION_SECS)
            .add()
            .step("station storage: assert the retired record")
            .on_enter(assert_the_retire)
            .add()
            .step("station storage: jump back to the station's side")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<StorageWatch>().cell),
                the_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station storage: the thawed station holds the storage")
            .on_enter(assert_the_thawed_storage)
            .add()
            .step("station storage: berth and dock again")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add();
        let script = open_inventory(script);
        let script = pick_row(script, &format!("InventoryRowStorage{PANEL}"));
        let script = script.click_named(
            "station storage: withdraw every panel",
            "InventoryDraftAll",
            frames(SETTLE_FRAMES),
            BEAT_SECS,
        );
        let script = confirm(
            script,
            "the panel withdraw",
            when(|world| stored_count(world, PANEL) == 0),
        );
        let script = pick_row(script, &format!("InventoryRowStorage{BARREL}"));
        let script = confirm(
            script,
            "the barrel withdraw",
            when(|world| stored_count(world, BARREL) == 0),
        )
        .step("station storage: record the hold after the withdraw")
        .on_enter(|world: &mut World| {
            assert!(
                stored(world).is_empty(),
                "station storage: the withdraws empty the storage"
            );
            let player = player(world);
            let counts = (held(world, player, PANEL), held(world, player, BARREL));
            world.resource_mut::<StorageWatch>().after_withdraw = Some(counts);
        })
        .add();
        let script = pick_row(script, &format!("InventoryRowOwn{PANEL}"));
        let script = store_draft(script, OVER_CAP_PANELS)
            .step("station storage: assert the over-cap refusal")
            .on_enter(assert_the_over_cap_refusal)
            .add()
            .step(format!("station storage: type {SAVED_PANELS}"))
            .on_enter(|world: &mut World| {
                set_field_text(world, QUANTITY_FIELD, &SAVED_PANELS.to_string());
            })
            .until(frames(SETTLE_FRAMES))
            .add();
        let script = confirm(
            script,
            "the store for the save",
            when(|world| stored_count(world, PANEL) == SAVED_PANELS),
        );
        tap(script, "the interface key", "interface_toggle", unpaused())
            .step("station storage: the withdraws report")
            .until(when(|world| {
                let watch = world.resource::<StorageWatch>();
                watch.named("onwithdrawn").len() >= 2 && watch.named("onstored").len() >= 3
            }))
            .deadline(SESSION_SECS)
            .add()
            .step("station storage: assert the withdraw, and undock")
            .on_enter(|world: &mut World| {
                assert_the_withdraw(world);
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(player_free())
            .deadline(SESSION_SECS)
            .add()
            .step("station storage: open the pause menu")
            .on_enter(press_key(KeyCode::Escape))
            .until(ui_node_present(PAUSE_EXIT_BUTTON))
            .deadline(BEAT_SECS)
            .add()
            .step("station storage: let escape go")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "station storage: click Back to Main Menu",
                BACK_TO_MENU_BUTTON,
                state_is(GameStates::MainMenu),
                SESSION_SECS,
            )
            .step("station storage: assert the leave save")
            .on_enter(assert_the_saved_storage)
            .add()
            .click_named(
                "station storage: click Load",
                LOAD_BUTTON,
                ui_node_present(format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("the world name has a slug")
                )),
                BEAT_SECS,
            )
            .click_named(
                "station storage: select the world",
                &format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("the world name has a slug")
                ),
                frames(2),
                BEAT_SECS,
            )
            .click_named(
                "station storage: click Load World",
                LOAD_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("station storage: the loaded window streams the station back")
            .until(and(
                window_is(|world| world.resource::<StorageWatch>().cell),
                the_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station storage: assert the loaded storage")
            .on_enter(assert_the_loaded_storage)
            .add()
    }
}
