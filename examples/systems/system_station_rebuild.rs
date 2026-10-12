//! system_station_rebuild: a section the ship lost is rebuilt at a station
//! from its recipe, and the rebuild is part of the ship from then on.
//!
//! A base waystation stands in a void. The player's hull is a controller over
//! a hull block, a port on its nose, a drive on its tail and two PDCs: `pdc_a`
//! on the controller's top and `pdc_b` on its starboard face, both away from
//! the dock face. The player rebinds `pdc_a` and loses it, docks at the living
//! station, opens the Ship pane and clicks Rebuild on the lost list. The
//! player's hold covers the panels and the actuator; the turret body and the
//! barrel are bought from the station's stock at ask. Then the player undocks
//! and the hull is frozen and thawed the way a retiring cell does it.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a lost section is rebuilt at a station` | `pdc_a` is live at full health under its old code and its rebound key; the hold parts, the bought parts and the price left the player, the price and the bought parts reached the station, and `OnRebuilt` fired once with the code, the parts and the price |
//! | 2 | `outcome: a rebuilt section survives a freeze and thaw` | the frozen record carries `pdc_a`, and the thawed hull has it live at full health under the same key |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_station_rebuild --features debug
//! # look for: `station rebuild probe: rebuilt pdc_a as PDC-1`,
//! #           `station rebuild probe: pdc_a survives the freeze and thaw`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_input::prelude::InputSource;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_station_rebuild")]
#[command(version = "1.0.0")]
#[command(about = "A lost PDC is rebuilt at a living station from the Ship pane, from hold parts and parts bought at ask, and survives a freeze and thaw. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The waystation's park, down -Z from the player.
const STATION_POSITION: Meters3 = Meters3::new(0.0, 0.0, -150.0);
/// The station object id.
const STATION_ID: &str = "waystation";
/// The scenario id the void loads under.
const SCENARIO_ID: &str = "station_rebuild_rig";
/// The player object id.
const PLAYER_ID: &str = "player_spaceship";

/// The station is turned a quarter turn about +Y, so its collar, which stands
/// off its -X flank, faces +Z: back toward the player.
const STATION_TURN: f32 = std::f32::consts::FRAC_PI_2;

/// Where the player's hull parks relative to the station, engine units: the
/// turned collar's face stands 2.5 cells along +Z from the station origin and
/// the player's port face 1.5 ahead of its hull origin, half a cell apart.
const BERTH_OFFSET: Vec3 = Vec3::new(0.0, 0.0, 4.5);

/// The PDC the player loses and rebuilds, and its sibling. `pdc_a` sorts
/// first, so the design code is `PDC-1`; the live-count rule the pane used to
/// follow would have handed the rebuild `PDC-3`.
const LOST_PDC: &str = "pdc_a";
const KEPT_PDC: &str = "pdc_b";
const LOST_CODE: &str = "PDC-1";

/// The key `pdc_a` is authored on, and the key the player rebinds it to
/// before the loss: the rebuild must come back on the rebound one.
const AUTHORED_KEY: KeyCode = KeyCode::KeyV;
const REBOUND_KEY: KeyCode = KeyCode::KeyB;

/// The player's balance and the parts the hold starts with: the PDC recipe's
/// two panels and its actuator. The station sells the body and the barrel.
const PLAYER_CREDITS: u32 = 1_000;
const HOLD_PANELS: u32 = 2;
const HOLD_ACTUATORS: u32 = 1;

/// The PDC recipe's work and the station's ask for the two bought parts.
const RECIPE_CREDITS: u32 = 100;
const BODY_ASK: u32 = 200;
const BARREL_ASK: u32 = 60;
const PRICE: u32 = RECIPE_CREDITS + BODY_ASK + BARREL_ASK;

/// Item ids the recipe names.
const PANEL: &str = "SectionPanel";
const ACTUATOR: &str = "ActuatorPack";
const BODY: &str = "TurretBody";
const BARREL: &str = "TurretBarrel";

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
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::RebuildWatchPlugin);
        app.add_plugins(walk::script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_void);
}

fn setup_void(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(station_void(&game_assets, &sections)));
}

/// The void: the living waystation and the player on its berth, lit by a
/// three-point rig because the engine spawns no light.
fn station_void(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let station = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: STATION_ID.to_string(),
            name: "WAYSTATION".to_string(),
            position: STATION_POSITION,
            rotation: Quat::from_rotation_y(STATION_TURN),
        },
        kind: ScenarioObjectKind::Station(StationConfig {
            design: BLOCK_WAYSTATION_STATION_ID.into(),
            allegiance: Some(Allegiance::Neutral),
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
        kind: ScenarioObjectKind::Spaceship(player_ship(sections)),
    };
    ScenarioConfig {
        description: "A waystation that rebuilds, and a player who lost a gun.".to_string(),
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
            "Station Rebuild Rig".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The player's hull, out of the base catalog: a controller over a hull block
/// (the hold), a port on its nose, a drive on its tail, and a PDC on the
/// controller's top and one on its starboard face. A PDC is half a cell, so
/// its base plate meets the controller's face three quarters of a cell out.
fn player_ship(sections: &GameSections) -> SpaceshipConfig {
    let section =
        |id: &str, prototype: &str, position: Vec3, rotation: Quat| SpaceshipSectionConfig {
            id: id.to_string(),
            position,
            rotation,
            source: SectionSource::Inline(
                sections
                    .get_section(prototype)
                    .unwrap_or_else(|| panic!("section '{prototype}' not found"))
                    .clone(),
            ),
        };
    SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: [
                (
                    LOST_PDC.to_string(),
                    vec![InputSource::Keyboard(AUTHORED_KEY)],
                ),
                (
                    KEPT_PDC.to_string(),
                    vec![InputSource::Keyboard(KeyCode::KeyN)],
                ),
            ]
            .into(),
        }),
        design: ShipDesignSource::Inline(ShipDesign {
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
                    LOST_PDC,
                    PDC_KINETIC_TURRET_SECTION_ID,
                    Vec3::new(0.0, 0.75, 0.0),
                    Quat::IDENTITY,
                ),
                // A quarter turn about -Z puts the base plate's -Y on -X,
                // against the controller's starboard face.
                section(
                    KEPT_PDC,
                    PDC_KINETIC_TURRET_SECTION_ID,
                    Vec3::new(0.75, 0.0, 0.0),
                    Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
                ),
            ],
            ..default()
        }),
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
        nova_debug::harness::{press_action, release_action, AutopilotPlugin, Predicate},
        nova_events::engine::GameEvent,
        nova_interface::{pane::InterfacePaneType, ship::SectionCode},
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
    /// Frames the Ship pane gets to build its panel and list.
    const SETTLE_FRAMES: u32 = 10;

    /// What the run saw and recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct RebuildWatch {
        /// Every rebuild and balance event, by name, with its payload.
        events: Vec<(String, serde_json::Value)>,
        /// `pdc_a`'s code before the loss.
        code_before: Option<String>,
        /// The station's balance and stock of the two bought parts before the
        /// rebuild.
        station_before: Option<(u32, u32, u32)>,
        /// The player root the freeze despawned.
        frozen_root: Option<Entity>,
    }

    impl RebuildWatch {
        fn named(&self, name: &str) -> Vec<&serde_json::Value> {
            self.events
                .iter()
                .filter(|(event, _)| event == name)
                .map(|(_, data)| data)
                .collect()
        }
    }

    pub(super) struct RebuildWatchPlugin;

    impl Plugin for RebuildWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<RebuildWatch>();
            app.add_observer(|event: On<GameEvent>, mut watch: ResMut<RebuildWatch>| {
                let name = event.name();
                if !matches!(name, "onrebuilt" | "oncreditschanged") {
                    return;
                }
                let data = event.info().data.clone().unwrap_or_default();
                info!("station rebuild probe: {name} {data}");
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
        player_root(world).expect("station rebuild probe: the player is up")
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("station rebuild probe: the station is up")
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
            .expect("station rebuild probe: every ship carries a hold")
            .count(&item.into())
    }

    fn credits(world: &World, root: Entity) -> u32 {
        world
            .get::<ShipCredits>(root)
            .expect("station rebuild probe: every ship carries a balance")
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
            .step(format!("station rebuild: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("station rebuild: let {label} up"))
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
            .expect("station rebuild probe: the station has a physics position")
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

    /// Rebind `pdc_a`, record its code, and shoot it away: zero health and
    /// the depletion marker, the state the damage path leaves a dead section
    /// in.
    fn lose_the_pdc(world: &mut World) {
        let player = player(world);
        let pdc = section_of(world, player, LOST_PDC)
            .expect("station rebuild probe: pdc_a is live before the loss");
        assert_eq!(
            turret_key(world, pdc),
            Some(vec![InputSource::Keyboard(AUTHORED_KEY)]),
            "station rebuild probe: pdc_a spawns on its authored key"
        );
        let code = world
            .get::<SectionCode>(pdc)
            .expect("station rebuild probe: the pane coded pdc_a")
            .0
            .clone();
        assert_eq!(code, LOST_CODE, "station rebuild probe: pdc_a codes PDC-1");
        world.resource_mut::<RebuildWatch>().code_before = Some(code);
        world
            .entity_mut(pdc)
            .insert(SpaceshipTurretInputBinding(vec![InputSource::Keyboard(
                REBOUND_KEY,
            )]));
        world.flush();
        world
            .get_mut::<Health>(pdc)
            .expect("station rebuild probe: pdc_a has health")
            .current = 0.0;
        world.entity_mut(pdc).insert(HealthZeroMarker);
    }

    fn pdc_coded() -> Arc<Predicate> {
        when(|world| {
            player_root(world)
                .and_then(|player| section_of(world, player, LOST_PDC))
                .is_some_and(|pdc| world.get::<SectionCode>(pdc).is_some())
        })
    }

    fn pdc_lost() -> Arc<Predicate> {
        when(|world| {
            player_root(world).is_some_and(|player| section_of(world, player, LOST_PDC).is_none())
        })
    }

    fn pdc_live() -> Arc<Predicate> {
        when(|world| {
            player_root(world).is_some_and(|player| section_of(world, player, LOST_PDC).is_some())
        })
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

    /// The station's balance and stock of the two parts the rebuild buys.
    fn record_the_station(world: &mut World) {
        let station = station(world);
        let before = (
            credits(world, station),
            held(world, station, BODY),
            held(world, station, BARREL),
        );
        world.resource_mut::<RebuildWatch>().station_before = Some(before);
    }

    fn rebuild_reported() -> Arc<Predicate> {
        when(|world| {
            let watch = world.resource::<RebuildWatch>();
            !watch.named("onrebuilt").is_empty() && watch.named("oncreditschanged").len() >= 2
        })
    }

    /// Claim 1: the rebuilt section, the moved parts and credits, and the
    /// one `OnRebuilt`.
    fn assert_the_rebuild(world: &mut World) {
        let player = player(world);
        let station = station(world);
        let pdc = section_of(world, player, LOST_PDC)
            .expect("station rebuild probe: pdc_a is live after the rebuild");
        assert!(
            full_health(world, pdc),
            "station rebuild probe: pdc_a must come back at full health: {:?}",
            world.get::<Health>(pdc)
        );
        let key = turret_key(world, pdc);
        assert_eq!(
            key,
            Some(vec![InputSource::Keyboard(REBOUND_KEY)]),
            "station rebuild probe: pdc_a must come back on the key it was rebound to"
        );
        let code = world.get::<SectionCode>(pdc).map(|code| code.0.clone());
        let (code_before, station_before) = {
            let watch = world.resource::<RebuildWatch>();
            (
                watch.code_before.clone(),
                watch
                    .station_before
                    .expect("station rebuild probe: the station was recorded"),
            )
        };
        assert_eq!(
            code, code_before,
            "station rebuild probe: pdc_a must keep its code"
        );
        let kept = section_of(world, player, KEPT_PDC)
            .and_then(|kept| world.get::<SectionCode>(kept))
            .map(|code| code.0.clone());
        assert_eq!(
            kept.as_deref(),
            Some("PDC-2"),
            "station rebuild probe: pdc_b keeps PDC-2"
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
            "station rebuild probe: the player pays the work and the bought parts"
        );
        let (station_cr, station_bodies, station_barrels) = station_before;
        assert_eq!(
            credits(world, station),
            station_cr + PRICE,
            "station rebuild probe: the price reaches the station"
        );
        assert_eq!(held(world, station, BODY), station_bodies - 1);
        assert_eq!(held(world, station, BARREL), station_barrels - 1);

        let watch = world.resource::<RebuildWatch>();
        let rebuilt = watch.named("onrebuilt");
        assert_eq!(rebuilt.len(), 1, "station rebuild probe: one OnRebuilt");
        let event = rebuilt[0].clone();
        assert_eq!(event["id"], PLAYER_ID);
        assert_eq!(event["section"], LOST_CODE);
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
            ["Rebuild", "Rebuild"],
            "station rebuild probe: one Rebuild balance change per party"
        );

        let elapsed = world.resource::<Time>().elapsed_secs();
        info!("station rebuild probe: rebuilt pdc_a as {LOST_CODE} for {PRICE} cr");
        nova_probe::probe_marker(
            world,
            "outcome: a lost section is rebuilt at a station",
            serde_json::json!({ "t": elapsed, "code": code, "price": PRICE, "event": event }),
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
            .expect("station rebuild probe: an undocked hull at rest is settled");
        let FrozenBodyType::Ship(record) = frozen.body() else {
            panic!(
                "station rebuild probe: the player froze as {:?}, not a ship",
                frozen.body()
            );
        };
        assert!(
            record.has_section(LOST_PDC),
            "station rebuild probe: the frozen hull must carry the rebuilt pdc_a"
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
        world.resource_mut::<RebuildWatch>().frozen_root = Some(player);
    }

    fn thawed_pdc_built() -> Arc<Predicate> {
        when(|world| {
            let frozen = world.resource::<RebuildWatch>().frozen_root;
            player_root(world).is_some_and(|player| {
                Some(player) != frozen
                    && section_of(world, player, LOST_PDC)
                        .is_some_and(|pdc| turret_key(world, pdc).is_some())
            })
        })
    }

    /// Claim 2: the thawed hull has `pdc_a` live, whole and on its key.
    fn assert_the_thaw(world: &mut World) {
        let player = player(world);
        let pdc = section_of(world, player, LOST_PDC)
            .expect("station rebuild probe: the thawed hull has pdc_a");
        assert!(
            full_health(world, pdc),
            "station rebuild probe: the thawed pdc_a must be whole: {:?}",
            world.get::<Health>(pdc)
        );
        assert_eq!(
            turret_key(world, pdc),
            Some(vec![InputSource::Keyboard(REBOUND_KEY)]),
            "station rebuild probe: the thawed pdc_a must keep its rebound key"
        );
        let elapsed = world.resource::<Time>().elapsed_secs();
        info!("station rebuild probe: pdc_a survives the freeze and thaw");
        nova_probe::probe_marker(
            world,
            "outcome: a rebuilt section survives a freeze and thaw",
            serde_json::json!({ "t": elapsed }),
        );
    }

    /// The walk. Every beat waits on a world value, so a slow load delays it
    /// instead of truncating it.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("station rebuild: load the void")
            .enter(GameStates::Loading)
            .until(when(|world| {
                station_root(world).is_some_and(|station| world.get::<Position>(station).is_some())
                    && player_root(world)
                        .is_some_and(|player| world.get::<Position>(player).is_some())
            }))
            .deadline(SESSION_SECS)
            .add()
            .step("station rebuild: the pane codes the PDCs")
            .until(pdc_coded())
            .deadline(SESSION_SECS)
            .add()
            .step("station rebuild: rebind and lose pdc_a")
            .on_enter(lose_the_pdc)
            .until(pdc_lost())
            .deadline(SESSION_SECS)
            .add()
            .step("station rebuild: settle on the berth")
            .on_enter(berth_player)
            .until(player_berthed())
            .deadline(SESSION_SECS)
            .add()
            .step("station rebuild: dock")
            .each(|world, _, _| request_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("station rebuild: record the station")
            .on_enter(record_the_station)
            .until(frames(1))
            .add();
        let lost_button = format!("ShipRebuild-{LOST_CODE}");
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(InterfacePaneType::Map),
        )
        .click_named(
            "station rebuild: open the Ship pane",
            "InterfaceTabShip",
            the_interface_shows(InterfacePaneType::Ship),
            SESSION_SECS,
        )
        .step("station rebuild: settle the schematic")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click_named(
            "station rebuild: rebuild pdc_a",
            &lost_button,
            pdc_live(),
            SESSION_SECS,
        );
        // The pane pushed its reports under the interface pause; the tracker
        // drains them on the first fixed tick after the clock runs again.
        tap(script, "the interface key", "interface_toggle", unpaused())
            .step("station rebuild: the rebuild reports")
            .until(rebuild_reported())
            .deadline(SESSION_SECS)
            .add()
            .step("station rebuild: assert the rebuild and undock")
            .on_enter(assert_the_rebuild)
            .until(player_free())
            .deadline(SESSION_SECS)
            .add()
            .step("station rebuild: freeze and thaw the player")
            .on_enter(freeze_and_thaw_the_player)
            .until(thawed_pdc_built())
            .deadline(SESSION_SECS)
            .add()
            .step("station rebuild: assert the thaw")
            .on_enter(assert_the_thaw)
            .add()
    }
}
