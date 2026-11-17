//! system_ship_consumables: a repair kit heals over time, and a refused use
//! keeps the item.
//!
//! The player's hull floats alone: a controller over a hull block (the
//! hold), a port on its nose and a drive on its tail. THE HOLD AND THE HULL
//! DAMAGE ARE DISCLOSED FIXTURE STATE: two repair kits, and the hold section
//! set 60 HP below its maximum. The walk writes the same
//! [`ConsumableUseRequest`] the Inventory Use button writes.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 0 | `outcome: a repair kit heals over time` | the kit started and took one kit; a second kit while it ran was refused `Running` and kept the item; the hold section came back to full health and the effect ended |
//! | 1 | `outcome: a kit with nothing to repair is refused` | a kit on the healed ship was refused `NothingToRepair` and kept the item |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_ship_consumables --features debug
//! # look for: `ship consumables probe: PASS` (two lines),
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_ship_consumables")]
#[command(version = "1.0.0")]
#[command(about = "A repair kit heals over time, and a refused use keeps the item. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario id the void loads under.
const SCENARIO_ID: &str = "ship_consumables_rig";
/// The player object id.
const PLAYER_ID: &str = "player_spaceship";

/// The base repair kit item.
const KIT: &str = "RepairKit";

/// The disclosed fixture: two kits, and the hold section 60 HP down, under
/// the kit's 80 HP.
const HOLD_KITS: u32 = 2;
const HULL_DAMAGE: f32 = 60.0;

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
        app.add_plugins(walk::ConsumableWatchPlugin);
        app.add_plugins(walk::script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_void);
}

fn setup_void(mut commands: Commands, game_assets: Res<GameAssets>) {
    let player = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLAYER_ID.to_string(),
            name: "Player".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(player_ship()),
    };
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "A lone player ship with repair kits aboard.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![EventActionConfig::SpawnScenarioObject(player)],
                ThreePointRig::around("rig", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Ship Consumables Rig".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// The player's hull: a controller over a hull block (the hold), a port on
/// its nose and a drive on its tail, with the fixture hold.
fn player_ship() -> SpaceshipConfig {
    let section =
        |id: &str, prototype: &str, position: Vec3, rotation: Quat| SpaceshipSectionConfig {
            id: id.to_string(),
            position,
            rotation,
            source: SectionSource::prototype(prototype),
        };
    SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig::default()),
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
            ],
            ..default()
        }),
        inventory: ShipInventoryStock::new([(KIT.into(), HOLD_KITS)]),
        ..default()
    }
}

#[cfg(feature = "debug")]
mod walk {
    use std::sync::Arc;

    use nova_protocol::nova_debug::harness::{AutopilotPlugin, Predicate};

    use super::*;

    /// Seconds a load or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 120.0;

    /// Every consumable use the run saw.
    #[derive(Resource, Default, Debug)]
    struct ConsumableWatch(Vec<ConsumableUsed>);

    pub(super) struct ConsumableWatchPlugin;

    impl Plugin for ConsumableWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<ConsumableWatch>();
            app.add_systems(
                Update,
                (|mut uses: MessageReader<ConsumableUsed>, mut watch: ResMut<ConsumableWatch>| {
                    for used in uses.read() {
                        info!("ship consumables probe: {used:?}");
                        watch.0.push(used.clone());
                    }
                })
                .after(ConsumableSystems),
            );
        }
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn player_ship(world: &World) -> Option<Entity> {
        let mut ships =
            world.try_query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()?;
        ships
            .iter(world)
            .find(|(_, entity_id)| ***entity_id == *PLAYER_ID)
            .map(|(entity, _)| entity)
    }

    fn player(world: &World) -> Entity {
        player_ship(world).expect("ship consumables probe: the player is up")
    }

    /// The player's repair kits.
    fn kits(world: &World) -> u32 {
        world
            .get::<ShipInventory>(player(world))
            .expect("ship consumables probe: every ship carries a hold")
            .count(&KIT.into())
    }

    fn hold_section(world: &World) -> (Entity, Health) {
        let children = world
            .get::<Children>(player(world))
            .expect("ship consumables probe: the player has sections");
        children
            .iter()
            .find_map(|child| {
                let id = world.get::<EntityId>(child)?;
                if id.0 != "hold" {
                    return None;
                }
                Some((child, world.get::<Health>(child)?.clone()))
            })
            .expect("ship consumables probe: the hold section is up")
    }

    fn running(world: &World) -> bool {
        world.get::<ShipConsumableEffect>(player(world)).is_some()
    }

    fn uses(world: &World) -> usize {
        world.resource::<ConsumableWatch>().0.len()
    }

    fn use_kit(world: &mut World) {
        let ship = player(world);
        world.write_message(ConsumableUseRequest {
            ship,
            item: KIT.into(),
        });
    }

    fn answered(count: usize) -> Arc<Predicate> {
        when(move |world| uses(world) >= count)
    }

    fn assert_use(world: &World, outcome: ConsumableOutcomeType) {
        let used = world
            .resource::<ConsumableWatch>()
            .0
            .last()
            .cloned()
            .expect("ship consumables probe: a use was answered");
        assert_eq!(
            (used.ship, used.item, used.outcome),
            (player(world), ItemDesignId::from(KIT), outcome),
            "ship consumables probe: the use of a kit"
        );
    }

    fn marker(world: &mut World, name: &str, payload: serde_json::Value) {
        let elapsed = world.resource::<Time>().elapsed_secs();
        let mut payload = payload;
        payload["t"] = serde_json::json!(elapsed);
        nova_probe::probe_marker(world, name, payload);
    }

    fn assert_the_fixture(world: &mut World) {
        assert_eq!(kits(world), HOLD_KITS);
        assert!(
            !running(world),
            "ship consumables probe: nothing runs at the start"
        );
        let (_, health) = hold_section(world);
        assert!(
            health.max > HULL_DAMAGE,
            "ship consumables probe: the hold section outlives the fixture damage"
        );
    }

    fn damage_the_hold(world: &mut World) {
        let (section, _) = hold_section(world);
        let mut health = world
            .get_mut::<Health>(section)
            .expect("ship consumables probe: a section carries health");
        health.current = health.max - HULL_DAMAGE;
    }

    fn assert_the_kit_started(world: &mut World) {
        assert_use(world, ConsumableOutcomeType::Started);
        assert_eq!(
            kits(world),
            HOLD_KITS - 1,
            "ship consumables probe: one kit spent"
        );
        assert!(running(world), "ship consumables probe: the kit runs");
    }

    fn assert_the_kit_refused(world: &mut World) {
        assert_use(
            world,
            ConsumableOutcomeType::Refused(ConsumableRefusalType::Running),
        );
        assert_eq!(
            kits(world),
            HOLD_KITS - 1,
            "ship consumables probe: the refused kit kept"
        );
    }

    /// Claim 0: the hold section is back to full and the effect ended.
    fn assert_the_repair(world: &mut World) {
        let (_, health) = hold_section(world);
        assert!(
            (health.max - health.current).abs() < 1e-3,
            "ship consumables probe: the hold section healed to full, read {health:?}"
        );
        assert!(!running(world), "ship consumables probe: the kit ended");
        info!(
            "ship consumables probe: PASS a repair kit healed the hold section to {}",
            health.current
        );
        marker(
            world,
            "outcome: a repair kit heals over time",
            serde_json::json!({ "healed": HULL_DAMAGE }),
        );
    }

    /// Claim 1: a kit on the healed ship was refused and kept.
    fn assert_nothing_to_repair(world: &mut World) {
        assert_use(
            world,
            ConsumableOutcomeType::Refused(ConsumableRefusalType::NothingToRepair),
        );
        assert_eq!(
            kits(world),
            HOLD_KITS - 1,
            "ship consumables probe: the refused kit kept"
        );
        info!("ship consumables probe: PASS a kit with nothing to repair was refused and kept");
        marker(
            world,
            "outcome: a kit with nothing to repair is refused",
            serde_json::json!({}),
        );
    }

    /// The walk. Every beat waits on a world value, so a slow load delays it
    /// instead of truncating it.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("ship consumables: load the void")
            .enter(GameStates::Loading)
            .until(when(|world| {
                player_ship(world).is_some_and(|player| {
                    world.get::<ShipInventory>(player).is_some()
                        && world
                            .get::<Children>(player)
                            .is_some_and(|children| children.len() >= 4)
                })
            }))
            .deadline(SESSION_SECS)
            .add()
            .step("ship consumables: check the fixture")
            .on_enter(assert_the_fixture)
            .until(frames(1))
            .add()
            .step("ship consumables: damage the hold section")
            .on_enter(damage_the_hold)
            .until(frames(1))
            .add()
            .step("ship consumables: use a repair kit")
            .on_enter(use_kit)
            .until(answered(1))
            .deadline(SESSION_SECS)
            .add()
            .step("ship consumables: check the kit started")
            .on_enter(assert_the_kit_started)
            .until(frames(1))
            .add()
            .step("ship consumables: use a second kit while it runs")
            .on_enter(use_kit)
            .until(answered(2))
            .deadline(SESSION_SECS)
            .add()
            .step("ship consumables: check the second kit was refused")
            .on_enter(assert_the_kit_refused)
            .until(frames(1))
            .add()
            .step("ship consumables: run the repair on the flight clock")
            .until(when(|world| !running(world)))
            .deadline(SESSION_SECS)
            .add()
            .step("ship consumables: check the repair")
            .on_enter(assert_the_repair)
            .until(frames(1))
            .add()
            .step("ship consumables: use a kit with nothing to repair")
            .on_enter(use_kit)
            .until(answered(3))
            .deadline(SESSION_SECS)
            .add()
            .step("ship consumables: check the refused kit")
            .on_enter(assert_nothing_to_repair)
            .add()
    }
}
