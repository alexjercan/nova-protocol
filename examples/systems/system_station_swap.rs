//! system_station_swap: a live section is swapped at a station for another
//! part of the same kind and footprint, and the swap is part of the ship
//! from then on.
//!
//! A base waystation stands in a void. The player's hull is a catalogue
//! design (a prototype this range registers): a controller over a hull block,
//! a port on its nose, a drive on its tail and two kinetic PDCs, `pdc_a` on
//! the controller's top and `pdc_b` on its starboard face. The player rebinds
//! `pdc_a`, takes damage on it and docks at the living station. A swap of
//! `pdc_a` for a thruster is refused with its reason and changes nothing.
//! Then the player selects `pdc_a` in the Ship pane, opens the Catalogue,
//! clicks the pierce PDC's tile and Swap on its card. The hold covers the recipe's panels and actuator; the
//! turret body and barrel are bought from the station at ask. Then the player
//! undocks and the hull is frozen and thawed the way a retiring cell does it.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a cross-family swap is refused` | the note names both kinds; the design, the section, the holds and the balances are as they were |
//! | 2 | `outcome: a section is swapped at a station` | `pdc_a` is a pierce PDC at full health under its old code and its rebound key; the design is inline and names the new prototype; the parts and the price moved; `OnSwapped` fired once |
//! | 3 | `outcome: a swapped section survives a freeze and thaw` | the thawed hull has `pdc_a` as a pierce PDC at full health on the same key |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_station_swap --features debug
//! # look for: `station swap probe: refused pdc_a for a thruster`,
//! #           `station swap probe: swapped pdc_a for the pierce PDC`,
//! #           `station swap probe: pdc_a survives the freeze and thaw`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! Add `NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=<dir>` to shoot the swap offer, the
//! pierce PDC's card in the catalogue gallery, as `station-swap-offer.png`.

use bevy::prelude::*;
use clap::Parser;
use nova_input::prelude::InputSource;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_station_swap")]
#[command(version = "1.0.0")]
#[command(about = "A PDC is swapped for another PDC at a living station from the Ship pane, a thruster swap is refused, and the swap survives a freeze and thaw. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The waystation's park, down -Z from the player.
const STATION_POSITION: Meters3 = Meters3::new(0.0, 0.0, -150.0);
/// The station object id.
const STATION_ID: &str = "waystation";
/// The scenario id the void loads under.
const SCENARIO_ID: &str = "station_swap_rig";
/// The player object id.
const PLAYER_ID: &str = "player_spaceship";
/// The catalogue design the player's hull spawns from.
const PLAYER_DESIGN: &str = "station_swap_hull";

/// The station is turned a quarter turn about +Y, so its collar, which stands
/// off its -X flank, faces +Z: back toward the player.
const STATION_TURN: f32 = std::f32::consts::FRAC_PI_2;

/// Where the player's hull parks relative to the station, engine units: the
/// turned collar's face stands 2.5 cells along +Z from the station origin and
/// the player's port face 1.5 ahead of its hull origin, half a cell apart.
const BERTH_OFFSET: Vec3 = Vec3::new(0.0, 0.0, 4.5);

/// The PDC the player swaps, and its sibling.
const SWAPPED_PDC: &str = "pdc_a";
const KEPT_PDC: &str = "pdc_b";
const SWAPPED_CODE: &str = "PDC-1";

/// The key `pdc_a` is authored on, and the key the player rebinds it to
/// before the swap: the new part must fly on the rebound one.
const AUTHORED_KEY: KeyCode = KeyCode::KeyV;
const REBOUND_KEY: KeyCode = KeyCode::KeyB;

/// The health `pdc_a` is left at before the swap; the new part spawns whole.
const DAMAGED_HEALTH: f32 = 40.0;

/// The player's balance and the parts the hold starts with: the pierce PDC
/// recipe's two panels and its actuator. The station sells the body and the
/// barrel.
const PLAYER_CREDITS: u32 = 1_000;
const HOLD_PANELS: u32 = 2;
const HOLD_ACTUATORS: u32 = 1;

/// The pierce PDC recipe's work and the station's ask for the two bought
/// parts.
const RECIPE_CREDITS: u32 = 100;
const BODY_ASK: u32 = 200;
const BARREL_ASK: u32 = 60;
const PRICE: u32 = RECIPE_CREDITS + BODY_ASK + BARREL_ASK;

/// Item ids the recipe names.
const PANEL: &str = "SectionPanel";
const ACTUATOR: &str = "ActuatorPack";
const BODY: &str = "TurretBody";
const BARREL: &str = "TurretBarrel";

/// The PNG the capture path shoots of the swap offer.
const OFFER_SHOT: &str = "station-swap-offer.png";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // The fleet's 1920x1080, the size the catalogue gallery is walked at.
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::SwapWatchPlugin);
        app.add_plugins(walk::script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_void);
}

/// Register the player's hull as a catalogue design and load the void.
fn setup_void(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut designs: ResMut<GameShipDesigns>,
) {
    designs.0.push(ShipDesignPrototype {
        id: PLAYER_DESIGN.into(),
        name: "Swap Range Hull".to_string(),
        design: player_hull(),
    });
    commands.trigger(LoadScenario(station_void(&game_assets)));
}

/// The void: the living waystation and the player on its berth, lit by a
/// three-point rig because the engine spawns no light.
fn station_void(game_assets: &GameAssets) -> ScenarioConfig {
    let station = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: STATION_ID.to_string(),
            name: "WAYSTATION".to_string(),
            position: STATION_POSITION,
            rotation: Quat::from_rotation_y(STATION_TURN),
        },
        kind: ScenarioObjectKind::Station(StationConfig {
            design: StationDesignSource::Prototype(BLOCK_WAYSTATION_STATION_ID.into()),
            allegiance: Allegiance::Neutral,
            derelict: false,
            initial_velocity: MetersPerSecond3::ZERO,
        }),
    };
    let player = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLAYER_ID.to_string(),
            name: "Player".to_string(),
            position: Meters3::from_engine(STATION_POSITION.to_engine() + BERTH_OFFSET),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(player_ship()),
    };
    ScenarioConfig {
        description: "A waystation that refits, and a player who wants another gun.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    EventActionConfig::SpawnScenarioObject(station),
                    EventActionConfig::SpawnScenarioObject(player),
                ],
                ThreePointRig::around("rig", STATION_POSITION, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Station Swap Rig".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The player's hull as catalogue references: a controller over a hull block
/// (the hold), a port on its nose, a drive on its tail, and a kinetic PDC on
/// the controller's top and one on its starboard face. A PDC is half a cell,
/// so its base plate meets the controller's face three quarters of a cell
/// out.
fn player_hull() -> ShipDesign {
    let section =
        |id: &str, prototype: &str, position: Vec3, rotation: Quat| SpaceshipSectionConfig {
            id: id.to_string(),
            position,
            rotation,
            source: SectionSource::prototype(prototype),
        };
    ShipDesign {
        sections: vec![
            section(
                "controller",
                BASIC_CONTROLLER_SECTION_ID,
                Vec3::ZERO,
                Quat::IDENTITY,
            ),
            section(
                "hold",
                REINFORCED_HULL_SECTION_ID,
                Vec3::NEG_Y,
                Quat::IDENTITY,
            ),
            section("port", DOCKING_PORT_SECTION_ID, Vec3::NEG_Z, Quat::IDENTITY),
            section("drive", BASIC_THRUSTER_SECTION_ID, Vec3::Z, Quat::IDENTITY),
            section(
                SWAPPED_PDC,
                PDC_KINETIC_TURRET_SECTION_ID,
                Vec3::new(0.0, 0.75, 0.0),
                Quat::IDENTITY,
            ),
            // A quarter turn about -Z puts the base plate's -Y on -X, against
            // the controller's starboard face.
            section(
                KEPT_PDC,
                PDC_KINETIC_TURRET_SECTION_ID,
                Vec3::new(0.75, 0.0, 0.0),
                Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
            ),
        ],
        ..default()
    }
}

/// The player's spawn of [`PLAYER_DESIGN`], with its bindings, hold and
/// balance.
fn player_ship() -> SpaceshipConfig {
    SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: [
                (
                    SWAPPED_PDC.to_string(),
                    vec![InputSource::Keyboard(AUTHORED_KEY)],
                ),
                (
                    KEPT_PDC.to_string(),
                    vec![InputSource::Keyboard(KeyCode::KeyN)],
                ),
            ]
            .into(),
        }),
        design: ShipDesignSource::prototype(PLAYER_DESIGN),
        inventory: ShipInventoryStock::new([
            (PANEL.into(), HOLD_PANELS),
            (ACTUATOR.into(), HOLD_ACTUATORS),
        ]),
        credits: PLAYER_CREDITS,
        ..default()
    }
}

#[cfg(feature = "debug")]
mod walk {
    use std::sync::Arc;

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use nova_protocol::{
        nova_debug::harness::{
            press_action, release_action, shoot, AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS,
        },
        nova_events::engine::GameEvent,
        nova_interface::{
            pane::InterfacePaneType,
            ship::{SectionCode, SectionSwapCommand, ShipRuntime},
        },
    };
    use nova_world::prelude::{
        freeze_body, thaw_record, DockPolicy, FrozenBodyType, PreparedFrozenBody, SectorCoord,
    };

    use super::*;

    /// Seconds a load, a pane or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 120.0;
    /// How close a teleported hull must settle to its berth, and how slow.
    const BERTH_TOLERANCE: f32 = 0.05;
    const BERTH_REST: f32 = 0.01;
    /// Frames the Ship pane gets to build its panel and lists.
    const SETTLE_FRAMES: u32 = 10;

    /// What the run saw and recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct SwapWatch {
        /// Every swap and balance event, by name, with its payload.
        events: Vec<(String, serde_json::Value)>,
        /// `pdc_a`'s entity before the refused swap.
        pdc_before: Option<Entity>,
        /// The station's balance and stock of the two bought parts before the
        /// swap.
        station_before: Option<(u32, u32, u32)>,
        /// The player root the freeze despawned.
        frozen_root: Option<Entity>,
    }

    impl SwapWatch {
        fn named(&self, name: &str) -> Vec<&serde_json::Value> {
            self.events
                .iter()
                .filter(|(event, _)| event == name)
                .map(|(_, data)| data)
                .collect()
        }
    }

    pub(super) struct SwapWatchPlugin;

    impl Plugin for SwapWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<SwapWatch>();
            app.add_observer(|event: On<GameEvent>, mut watch: ResMut<SwapWatch>| {
                let name = event.name();
                if !matches!(name, "onswapped" | "oncreditschanged") {
                    return;
                }
                let data = event.info().data.clone().unwrap_or_default();
                info!("station swap probe: {name} {data}");
                watch.events.push((name.to_string(), data));
            });
        }
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn station_root(world: &World) -> Option<Entity> {
        let mut stations =
            world.try_query_filtered::<(Entity, &EntityId), With<StationMarker>>()?;
        stations
            .iter(world)
            .find(|(_, id)| id.0 == STATION_ID)
            .map(|(entity, _)| entity)
    }

    fn player_root(world: &World) -> Option<Entity> {
        let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
        players.iter(world).next()
    }

    fn player(world: &World) -> Entity {
        player_root(world).expect("station swap probe: the player is up")
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("station swap probe: the station is up")
    }

    /// The live section `id` of `root`, or `None` while it has none.
    fn section_of(world: &World, root: Entity, id: &str) -> Option<Entity> {
        world
            .get::<Children>(root)?
            .iter()
            .find(|child| world.get::<EntityId>(*child).is_some_and(|eid| eid.0 == id))
    }

    fn held(world: &World, root: Entity, item: &str) -> u32 {
        world
            .get::<ShipInventory>(root)
            .expect("station swap probe: every ship carries a hold")
            .count(&item.into())
    }

    fn credits(world: &World, root: Entity) -> u32 {
        world
            .get::<ShipCredits>(root)
            .expect("station swap probe: every ship carries a balance")
            .0
    }

    fn turret_key(world: &World, section: Entity) -> Option<Vec<InputSource>> {
        world
            .get::<SpaceshipTurretInputBinding>(section)
            .map(|binding| binding.0.clone())
    }

    fn full_health(world: &World, section: Entity) -> bool {
        world
            .get::<Health>(section)
            .is_some_and(|health| health.max > 0.0 && health.current == health.max)
    }

    /// The prototype id design section `id` of `root` resolved from.
    fn resolved_part(world: &World, root: Entity, id: &str) -> String {
        world
            .get::<ShipDesignSections>(root)
            .and_then(|design| design.get(id))
            .unwrap_or_else(|| panic!("station swap probe: the design has section '{id}'"))
            .config
            .base
            .id
            .clone()
    }

    /// The source design section `id` of `root` is authored with, as the
    /// prototype id it names, or `None` when the design is still a catalogue
    /// reference or the section is inline.
    fn authored_part(world: &World, root: Entity, id: &str) -> Option<String> {
        let design = world
            .get::<SpaceshipDesign>(root)
            .expect("station swap probe: the player carries its design");
        let ShipDesignSource::Inline(design) = &design.0 else {
            return None;
        };
        let section = design
            .sections
            .iter()
            .find(|section| section.id == id)
            .unwrap_or_else(|| panic!("station swap probe: the design has section '{id}'"));
        match &section.source {
            SectionSource::Prototype { id, .. } => Some(id.to_string()),
            SectionSource::Inline(_) => None,
        }
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

    fn unpaused() -> Arc<Predicate> {
        when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Unpaused)
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
            .step(format!("station swap: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("station swap: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Park the player's hull on the berth, at rest, on the attitude its
    /// computer holds.
    fn berth_player(world: &mut World) {
        let berth = world
            .get::<Position>(station(world))
            .expect("station swap probe: the station has a physics position")
            .0
            + BERTH_OFFSET;
        let player = player(world);
        world.entity_mut(player).insert((
            Position(berth),
            Rotation(Quat::IDENTITY),
            LinearVelocity(Vec3::ZERO),
            AngularVelocity(Vec3::ZERO),
        ));
    }

    fn player_berthed() -> Arc<Predicate> {
        when(|world| {
            let (Some(player), Some(station)) = (player_root(world), station_root(world)) else {
                return false;
            };
            let (Some(position), Some(velocity), Some(station_at)) = (
                world.get::<Position>(player),
                world.get::<LinearVelocity>(player),
                world.get::<Position>(station),
            ) else {
                return false;
            };
            position.0.distance(station_at.0 + BERTH_OFFSET) < BERTH_TOLERANCE
                && velocity.0.length() < BERTH_REST
        })
    }

    fn pdc_coded() -> Arc<Predicate> {
        when(|world| {
            player_root(world)
                .and_then(|player| section_of(world, player, SWAPPED_PDC))
                .is_some_and(|pdc| world.get::<SectionCode>(pdc).is_some())
        })
    }

    /// Rebind `pdc_a` and leave it damaged, the state a fight leaves it in.
    fn rebind_and_damage_the_pdc(world: &mut World) {
        let player = player(world);
        let pdc = section_of(world, player, SWAPPED_PDC)
            .expect("station swap probe: pdc_a is live before the swap");
        assert_eq!(
            turret_key(world, pdc),
            Some(vec![InputSource::Keyboard(AUTHORED_KEY)]),
            "station swap probe: pdc_a spawns on its authored key"
        );
        assert_eq!(
            world.get::<SectionCode>(pdc).map(|code| code.0.as_str()),
            Some(SWAPPED_CODE),
            "station swap probe: pdc_a codes PDC-1"
        );
        world
            .entity_mut(pdc)
            .insert(SpaceshipTurretInputBinding(vec![InputSource::Keyboard(
                REBOUND_KEY,
            )]));
        world
            .get_mut::<Health>(pdc)
            .expect("station swap probe: pdc_a has health")
            .current = DAMAGED_HEALTH;
        world.resource_mut::<SwapWatch>().pdc_before = Some(pdc);
    }

    fn request_dock(world: &mut World) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let station = station(world);
        world.trigger(DockingConnectionRequest {
            entity: player,
            target: station,
        });
    }

    fn player_docked() -> Arc<Predicate> {
        when(|world| {
            player_root(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
        })
    }

    fn player_free() -> Arc<Predicate> {
        when(|world| {
            player_root(world).is_some_and(|player| world.get::<DockedShip>(player).is_none())
        })
    }

    /// The station's balance and stock of the two parts the swap buys.
    fn record_the_station(world: &mut World) {
        let station = station(world);
        let before = (
            credits(world, station),
            held(world, station, BODY),
            held(world, station, BARREL),
        );
        world.resource_mut::<SwapWatch>().station_before = Some(before);
    }

    /// Ask the pane to swap `pdc_a` for a thruster, a part the catalogue
    /// never offers to swap for a turret.
    fn ask_for_a_thruster(world: &mut World) {
        world.write_message(SectionSwapCommand {
            section: SWAPPED_PDC.to_string(),
            prototype: BASIC_THRUSTER_SECTION_ID.to_string(),
        });
    }

    fn the_note_reads_swap() -> Arc<Predicate> {
        when(|world| {
            world
                .resource::<ShipRuntime>()
                .note()
                .is_some_and(|note| note.starts_with("swap: "))
        })
    }

    /// Claim 1: the refusal names both kinds and changes nothing.
    fn assert_the_refusal(world: &mut World) {
        let player = player(world);
        let station = station(world);
        let note = world
            .resource::<ShipRuntime>()
            .note()
            .expect("station swap probe: the refusal shows a note")
            .to_string();
        assert!(
            note.starts_with(&format!("swap: {SWAPPED_CODE} is a turret, "))
                && note.ends_with(" is a thruster"),
            "station swap probe: the refusal must name both kinds: {note:?}"
        );
        let pdc = section_of(world, player, SWAPPED_PDC);
        assert_eq!(
            pdc,
            world.resource::<SwapWatch>().pdc_before,
            "station swap probe: a refused swap keeps the old section"
        );
        assert_eq!(
            resolved_part(world, player, SWAPPED_PDC),
            PDC_KINETIC_TURRET_SECTION_ID
        );
        assert!(
            matches!(
                world.get::<SpaceshipDesign>(player).map(|design| &design.0),
                Some(ShipDesignSource::Prototype { .. })
            ),
            "station swap probe: a refused swap leaves the catalogue design"
        );
        assert_eq!(credits(world, player), PLAYER_CREDITS);
        assert_eq!(held(world, player, PANEL), HOLD_PANELS);
        assert_eq!(held(world, player, ACTUATOR), HOLD_ACTUATORS);
        let (station_cr, station_bodies, station_barrels) = world
            .resource::<SwapWatch>()
            .station_before
            .expect("station swap probe: the station was recorded");
        assert_eq!(credits(world, station), station_cr);
        assert_eq!(held(world, station, BODY), station_bodies);
        assert_eq!(held(world, station, BARREL), station_barrels);
        info!("station swap probe: refused pdc_a for a thruster: {note}");
        let elapsed = world.resource::<Time>().elapsed_secs();
        nova_probe::probe_marker(
            world,
            "outcome: a cross-family swap is refused",
            serde_json::json!({ "t": elapsed, "note": note }),
        );
    }

    /// The pierce PDC's tile in the catalogue gallery, named by its
    /// catalogue name.
    fn pierce_tile(world: &World) -> String {
        let sections = world.resource::<GameSections>();
        let pierce = sections
            .get_section(PDC_PIERCE_TURRET_SECTION_ID)
            .expect("station swap: the catalogue lists the pierce PDC");
        format!("Gallery Tile {}", pierce.base.name)
    }

    fn pdc_selected(world: &World) -> bool {
        let Some(player) = player_root(world) else {
            return false;
        };
        let selected = world.resource::<ShipRuntime>().selected();
        selected.is_some() && selected == section_of(world, player, SWAPPED_PDC)
    }

    /// Step the selection with the viewer key, one press and one release per
    /// two frames, until `pdc_a` is selected.
    fn step_to_the_pdc(world: &mut World, _elapsed: f32, frame: u32) {
        if pdc_selected(world) {
            release_action("viewer_next")(world);
            return;
        }
        if frame % 2 == 0 {
            press_action("viewer_next")(world);
        } else {
            release_action("viewer_next")(world);
        }
    }

    fn pdc_swapped() -> Arc<Predicate> {
        when(|world| {
            let before = world.resource::<SwapWatch>().pdc_before;
            player_root(world)
                .and_then(|player| section_of(world, player, SWAPPED_PDC))
                .is_some_and(|pdc| Some(pdc) != before)
        })
    }

    fn swap_reported() -> Arc<Predicate> {
        when(|world| {
            let watch = world.resource::<SwapWatch>();
            !watch.named("onswapped").is_empty() && watch.named("oncreditschanged").len() >= 2
        })
    }

    /// Claim 2: the new part, the inline design, the moved parts and
    /// credits, and the one `OnSwapped`.
    fn assert_the_swap(world: &mut World) {
        let player = player(world);
        let station = station(world);
        let pdc = section_of(world, player, SWAPPED_PDC)
            .expect("station swap probe: pdc_a is live after the swap");
        assert!(
            full_health(world, pdc),
            "station swap probe: the new pdc_a must spawn whole: {:?}",
            world.get::<Health>(pdc)
        );
        assert_eq!(
            turret_key(world, pdc),
            Some(vec![InputSource::Keyboard(REBOUND_KEY)]),
            "station swap probe: the new pdc_a must fly on the key the old one was rebound to"
        );
        assert_eq!(
            world.get::<SectionCode>(pdc).map(|code| code.0.as_str()),
            Some(SWAPPED_CODE),
            "station swap probe: pdc_a keeps its code"
        );
        assert_eq!(
            resolved_part(world, player, SWAPPED_PDC),
            PDC_PIERCE_TURRET_SECTION_ID,
            "station swap probe: the design resolves pdc_a to the pierce PDC"
        );
        assert_eq!(
            authored_part(world, player, SWAPPED_PDC).as_deref(),
            Some(PDC_PIERCE_TURRET_SECTION_ID),
            "station swap probe: the owned inline design names the pierce PDC"
        );
        assert_eq!(
            authored_part(world, player, KEPT_PDC).as_deref(),
            Some(PDC_KINETIC_TURRET_SECTION_ID),
            "station swap probe: pdc_b keeps its catalogue part"
        );

        assert_eq!(
            held(world, player, PANEL),
            0,
            "the hold's panels were spent"
        );
        assert_eq!(
            held(world, player, ACTUATOR),
            0,
            "the hold's actuator was spent"
        );
        assert_eq!(
            held(world, player, BODY),
            0,
            "a bought part never enters the hold"
        );
        assert_eq!(
            credits(world, player),
            PLAYER_CREDITS - PRICE,
            "station swap probe: the player pays the work and the bought parts"
        );
        let (station_cr, station_bodies, station_barrels) = world
            .resource::<SwapWatch>()
            .station_before
            .expect("station swap probe: the station was recorded");
        assert_eq!(
            credits(world, station),
            station_cr + PRICE,
            "station swap probe: the price reaches the station"
        );
        assert_eq!(held(world, station, BODY), station_bodies - 1);
        assert_eq!(held(world, station, BARREL), station_barrels - 1);

        let watch = world.resource::<SwapWatch>();
        let swapped = watch.named("onswapped");
        assert_eq!(swapped.len(), 1, "station swap probe: one OnSwapped");
        let event = swapped[0].clone();
        assert_eq!(event["id"], PLAYER_ID);
        assert_eq!(event["section"], SWAPPED_CODE);
        assert_eq!(event["from"], PDC_KINETIC_TURRET_SECTION_ID);
        assert_eq!(event["to"], PDC_PIERCE_TURRET_SECTION_ID);
        assert_eq!(event["price_cr"], PRICE);
        assert_eq!(
            event["parts"],
            serde_json::json!({ ACTUATOR: 1, PANEL: 2, BARREL: 1, BODY: 1 })
        );
        let causes: Vec<String> = watch
            .named("oncreditschanged")
            .iter()
            .map(|data| data["cause"].as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(
            causes,
            ["Swap", "Swap"],
            "station swap probe: one Swap balance change per party"
        );

        let elapsed = world.resource::<Time>().elapsed_secs();
        info!("station swap probe: swapped pdc_a for the pierce PDC for {PRICE} cr");
        nova_probe::probe_marker(
            world,
            "outcome: a section is swapped at a station",
            serde_json::json!({ "t": elapsed, "price": PRICE, "event": event }),
        );
        world.trigger(DockingReleaseRequest { entity: player });
    }

    /// Freeze the undocked player as a retiring cell would, check the record
    /// carries `pdc_a`, despawn the live hull and thaw the record.
    fn freeze_and_thaw_the_player(world: &mut World) {
        let player = player(world);
        world
            .entity_mut(player)
            .insert((LinearVelocity(Vec3::ZERO), AngularVelocity(Vec3::ZERO)));
        let frozen = freeze_body(world, player, DockPolicy::Refuse)
            .expect("station swap probe: an undocked hull at rest is settled");
        let FrozenBodyType::Ship(record) = frozen.body() else {
            panic!(
                "station swap probe: the player froze as {:?}, not a ship",
                frozen.body()
            );
        };
        assert!(
            record.has_section(SWAPPED_PDC),
            "station swap probe: the frozen hull must carry pdc_a"
        );
        world.entity_mut(player).despawn();
        let root = world
            .spawn((
                Name::new("thaw root"),
                Transform::IDENTITY,
                Visibility::default(),
            ))
            .id();
        let prepared = PreparedFrozenBody::prepare(SectorCoord::new(0, 0, 0), frozen, &[], &[]);
        let mut commands = world.commands();
        thaw_record(&mut commands, root, vec![prepared]);
        world.flush();
        world.resource_mut::<SwapWatch>().frozen_root = Some(player);
    }

    fn thawed_pdc_built() -> Arc<Predicate> {
        when(|world| {
            let frozen = world.resource::<SwapWatch>().frozen_root;
            player_root(world).is_some_and(|player| {
                Some(player) != frozen
                    && section_of(world, player, SWAPPED_PDC)
                        .is_some_and(|pdc| turret_key(world, pdc).is_some())
            })
        })
    }

    /// Claim 3: the thawed hull has `pdc_a` as the pierce PDC, whole and on
    /// its key.
    fn assert_the_thaw(world: &mut World) {
        let player = player(world);
        let pdc = section_of(world, player, SWAPPED_PDC)
            .expect("station swap probe: the thawed hull has pdc_a");
        assert_eq!(
            resolved_part(world, player, SWAPPED_PDC),
            PDC_PIERCE_TURRET_SECTION_ID,
            "station swap probe: the thawed pdc_a must be the pierce PDC"
        );
        assert!(
            full_health(world, pdc),
            "station swap probe: the thawed pdc_a must be whole: {:?}",
            world.get::<Health>(pdc)
        );
        assert_eq!(
            turret_key(world, pdc),
            Some(vec![InputSource::Keyboard(REBOUND_KEY)]),
            "station swap probe: the thawed pdc_a must keep its rebound key"
        );
        let elapsed = world.resource::<Time>().elapsed_secs();
        info!("station swap probe: pdc_a survives the freeze and thaw");
        nova_probe::probe_marker(
            world,
            "outcome: a swapped section survives a freeze and thaw",
            serde_json::json!({ "t": elapsed }),
        );
    }

    /// The walk. Every beat waits on a world value, so a slow load delays it
    /// instead of truncating it.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("station swap: load the void")
            .enter(GameStates::Loading)
            .until(when(|world| {
                station_root(world).is_some_and(|station| world.get::<Position>(station).is_some())
                    && player_root(world)
                        .is_some_and(|player| world.get::<Position>(player).is_some())
            }))
            .deadline(SESSION_SECS)
            .add()
            .step("station swap: the pane codes the PDCs")
            .until(pdc_coded())
            .deadline(SESSION_SECS)
            .add()
            .step("station swap: rebind and damage pdc_a")
            .on_enter(rebind_and_damage_the_pdc)
            .until(frames(1))
            .add()
            .step("station swap: settle on the berth")
            .on_enter(berth_player)
            .until(player_berthed())
            .deadline(SESSION_SECS)
            .add()
            .step("station swap: dock")
            .each(|world, _, _| request_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("station swap: record the station")
            .on_enter(record_the_station)
            .until(frames(1))
            .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(InterfacePaneType::Map),
        )
        .click_named(
            "station swap: open the Ship pane",
            "InterfaceTabShip",
            the_interface_shows(InterfacePaneType::Ship),
            SESSION_SECS,
        )
        .step("station swap: settle the schematic")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("station swap: ask for a thruster")
        .on_enter(ask_for_a_thruster)
        .until(the_note_reads_swap())
        .deadline(SESSION_SECS)
        .add()
        .step("station swap: assert the refusal")
        .on_enter(assert_the_refusal)
        .until(frames(1))
        .add()
        .step("station swap: select pdc_a")
        .each(step_to_the_pdc)
        .until(when(pdc_selected))
        .deadline(SESSION_SECS)
        .add()
        .step("station swap: let the viewer key up")
        .on_enter(release_action("viewer_next"))
        .until(frames(SETTLE_FRAMES))
        .add()
        .click_named(
            "station swap: open the catalogue",
            "ShipCatalogue",
            ui_node_present("Gallery Grid"),
            SESSION_SECS,
        )
        .step("station swap: press the pierce PDC's tile")
        .on_enter(|world: &mut World| click_named(pierce_tile(world))(world))
        .until(pointer_pressed())
        .deadline(SESSION_SECS)
        .add()
        .step("station swap: release the pierce PDC's tile")
        .on_enter(release_mouse(MouseButton::Left))
        .until(ui_node_present("Gallery Alternate Button"))
        .deadline(SESSION_SECS)
        .add()
        .step("station swap: settle the card")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("station swap: shoot the offer")
        .on_enter(|world: &mut World| shoot(world, OFFER_SHOT))
        .until(shot_written(OFFER_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click_named(
            "station swap: swap pdc_a for the pierce PDC",
            "Gallery Alternate Button",
            pdc_swapped(),
            SESSION_SECS,
        );
        // The pane pushed its reports under the interface pause; the tracker
        // drains them on the first fixed tick after the clock runs again.
        tap(script, "the interface key", "interface_toggle", unpaused())
            .step("station swap: the swap reports")
            .until(swap_reported())
            .deadline(SESSION_SECS)
            .add()
            .step("station swap: assert the swap and undock")
            .on_enter(assert_the_swap)
            .until(player_free())
            .deadline(SESSION_SECS)
            .add()
            .step("station swap: freeze and thaw the player")
            .on_enter(freeze_and_thaw_the_player)
            .until(thawed_pdc_built())
            .deadline(SESSION_SECS)
            .add()
            .step("station swap: assert the thaw")
            .on_enter(assert_the_thaw)
            .add()
    }
}
