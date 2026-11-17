//! system_ship_consumables: each base consumable does what it says, and a
//! refused use keeps the item.
//!
//! The player's hull floats alone with a torpedo stand 2000 m off its nose:
//! a controller over a hull block (the hold), a port on its nose and a drive
//! on its tail. THE HOLD AND THE HULL DAMAGE ARE DISCLOSED FIXTURE STATE: two
//! repair kits, two boost cells and two decoy flares, and the hold section
//! set 60 HP below its maximum. The walk writes the same
//! [`ConsumableUseRequest`] the flight keys 1/2/3 and the Inventory Use
//! button write. The stand has no controller; a scripted order fires one
//! torpedo at the player.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 0 | `outcome: a repair kit heals over time` | the kit started and took one kit; a second kit while it ran was refused `Running` and kept the item; the hold section came back to full health and the effect ended |
//! | 1 | `outcome: a boost cell scales thrust` | the cell started and took one cell; the ship's thrust factor read 1.5; a second cell while it ran was refused `Running` and kept the item |
//! | 2 | `outcome: a decoy flare pulls a torpedo off` | the flare decoyed the one homing torpedo and took one flare; the torpedo lost its lock and flew to a point; it fuzed there and no player section lost health |
//! | 3 | `outcome: a flare with no torpedo is refused` | a second flare with no torpedo in its band was refused `NoTorpedo` and kept the item |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_ship_consumables --features debug
//! # look for: `ship consumables probe: PASS` (four lines),
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_ship_consumables")]
#[command(version = "1.0.0")]
#[command(about = "A repair kit heals, a boost cell scales thrust, a decoy flare pulls a torpedo off, and a refused use keeps the item. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario id the void loads under.
const SCENARIO_ID: &str = "ship_consumables_rig";
/// The player object id.
const PLAYER_ID: &str = "player_spaceship";
/// The torpedo stand's object id.
const STAND_ID: &str = "torpedo_stand";

/// The base consumable items.
const KIT: &str = "RepairKit";
const CELL: &str = "BoostCell";
const FLARE: &str = "DecoyFlare";

/// The disclosed fixture: two of each consumable, and the hold section
/// 60 HP down, under the kit's 80 HP.
const HOLD_EACH: u32 = 2;
const HULL_DAMAGE: f32 = 60.0;

/// The torpedo stand's range, inside the base flare band of 400 to 3000 m.
const STAND_RANGE: Meters = Meters(2000.0);

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

fn setup_void(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    let placed =
        |id: &str, name: &str, position: Meters3, ship: SpaceshipConfig| ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: name.to_string(),
                position,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(ship),
        };
    let player = placed(PLAYER_ID, "Player", Meters3::ZERO, player_ship());
    // A lone bay with no controller: the scripted order is its whole brain.
    let stand = placed(
        STAND_ID,
        "Torpedo Stand",
        Meters3(Vec3::new(0.0, 0.0, -STAND_RANGE.0)),
        fixtures::ship(
            &sections,
            SpaceshipController::None,
            &[SectionSpec::new("bay", "torpedo_section", Vec3::ZERO)],
        ),
    );
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "A lone player ship with consumables aboard, and a torpedo stand.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    EventActionConfig::SpawnScenarioObject(player),
                    EventActionConfig::SpawnScenarioObject(stand),
                ],
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
        inventory: ShipInventoryStock::new([
            (KIT.into(), HOLD_EACH),
            (CELL.into(), HOLD_EACH),
            (FLARE.into(), HOLD_EACH),
        ]),
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

    /// The player's summed section health when the flare went off.
    #[derive(Resource, Default, Debug)]
    struct HealthAtFlare(f32);

    pub(super) struct ConsumableWatchPlugin;

    impl Plugin for ConsumableWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<ConsumableWatch>();
            app.init_resource::<HealthAtFlare>();
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

    fn ship_by_id(world: &World, id: &str) -> Option<Entity> {
        let mut ships =
            world.try_query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()?;
        ships
            .iter(world)
            .find(|(_, entity_id)| ***entity_id == *id)
            .map(|(entity, _)| entity)
    }

    fn player(world: &World) -> Entity {
        ship_by_id(world, PLAYER_ID).expect("ship consumables probe: the player is up")
    }

    /// The player's repair kits, boost cells and decoy flares.
    fn hold(world: &World) -> (u32, u32, u32) {
        let hold = world
            .get::<ShipInventory>(player(world))
            .expect("ship consumables probe: every ship carries a hold");
        (
            hold.count(&KIT.into()),
            hold.count(&CELL.into()),
            hold.count(&FLARE.into()),
        )
    }

    /// The player's sections, by section id.
    fn sections(world: &World) -> Vec<(String, Entity, Health)> {
        let Some(children) = world.get::<Children>(player(world)) else {
            return Vec::new();
        };
        children
            .iter()
            .filter_map(|child| {
                let id = world.get::<EntityId>(child)?;
                let health = world.get::<Health>(child)?;
                Some((id.0.clone(), child, health.clone()))
            })
            .collect()
    }

    fn hold_section(world: &World) -> (Entity, Health) {
        sections(world)
            .into_iter()
            .find(|(id, _, _)| id == "hold")
            .map(|(_, entity, health)| (entity, health))
            .expect("ship consumables probe: the hold section is up")
    }

    fn health_sum(world: &World) -> f32 {
        sections(world)
            .iter()
            .map(|(_, _, health)| health.current)
            .sum()
    }

    fn running(world: &World, slot: ConsumableSlotType) -> Option<f32> {
        let consumables = world.resource::<GameConsumables>();
        world
            .get::<ShipConsumableEffects>(player(world))?
            .remaining_secs(consumables, slot)
    }

    fn uses(world: &World) -> usize {
        world.resource::<ConsumableWatch>().0.len()
    }

    fn last_use(world: &World) -> ConsumableUsed {
        world
            .resource::<ConsumableWatch>()
            .0
            .last()
            .cloned()
            .expect("ship consumables probe: a use was answered")
    }

    fn use_slot(slot: ConsumableSlotType) -> impl Fn(&mut World) + Send + Sync + 'static {
        move |world: &mut World| {
            let ship = player(world);
            world.write_message(ConsumableUseRequest { ship, slot });
        }
    }

    fn answered(count: usize) -> Arc<Predicate> {
        when(move |world| uses(world) >= count)
    }

    fn assert_use(world: &World, item: &str, outcome: ConsumableOutcomeType) {
        let used = last_use(world);
        assert_eq!(
            (used.ship, used.item, used.outcome),
            (player(world), ItemDesignId::from(item), outcome),
            "ship consumables probe: the use of {item}"
        );
    }

    fn marker(world: &mut World, name: &str, payload: serde_json::Value) {
        let elapsed = world.resource::<Time>().elapsed_secs();
        let mut payload = payload;
        payload["t"] = serde_json::json!(elapsed);
        nova_probe::probe_marker(world, name, payload);
    }

    fn assert_the_fixture(world: &mut World) {
        assert_eq!(hold(world), (HOLD_EACH, HOLD_EACH, HOLD_EACH));
        assert!(
            world.get::<ShipConsumableEffects>(player(world)).is_none(),
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
        assert_use(world, KIT, ConsumableOutcomeType::Started);
        assert_eq!(
            hold(world).0,
            HOLD_EACH - 1,
            "ship consumables probe: one kit spent"
        );
        assert!(running(world, ConsumableSlotType::Repair).is_some());
    }

    fn assert_the_kit_refused(world: &mut World) {
        assert_use(
            world,
            KIT,
            ConsumableOutcomeType::Refused(ConsumableRefusalType::Running),
        );
        assert_eq!(
            hold(world).0,
            HOLD_EACH - 1,
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
        assert!(running(world, ConsumableSlotType::Repair).is_none());
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

    fn assert_the_boost_started(world: &mut World) {
        assert_use(world, CELL, ConsumableOutcomeType::Started);
        assert_eq!(
            hold(world).1,
            HOLD_EACH - 1,
            "ship consumables probe: one cell spent"
        );
        assert!(
            world.get::<Autopilot>(player(world)).is_none(),
            "ship consumables probe: the player flies by hand"
        );
        let factor = world
            .get::<ShipConsumableEffects>(player(world))
            .expect("ship consumables probe: the boost runs")
            .thrust_factor(world.resource::<GameConsumables>());
        assert_eq!(factor, 1.5, "ship consumables probe: the base boost factor");
    }

    /// Claim 1: the second cell was refused and kept.
    fn assert_the_boost_refused(world: &mut World) {
        assert_use(
            world,
            CELL,
            ConsumableOutcomeType::Refused(ConsumableRefusalType::Running),
        );
        assert_eq!(
            hold(world).1,
            HOLD_EACH - 1,
            "ship consumables probe: the refused cell kept"
        );
        info!("ship consumables probe: PASS a boost cell scaled thrust by 1.5");
        marker(
            world,
            "outcome: a boost cell scales thrust",
            serde_json::json!({ "factor": 1.5 }),
        );
    }

    fn order_torpedo(world: &mut World) {
        let target = player(world);
        let stand = ship_by_id(world, STAND_ID).expect("ship consumables probe: the stand is up");
        let bays: Vec<Entity> = world
            .query_filtered::<(Entity, &ChildOf), With<TorpedoSectionMarker>>()
            .iter(world)
            .filter(|(_, parent)| parent.parent() == stand)
            .map(|(bay, _)| bay)
            .collect();
        assert_eq!(
            bays.len(),
            1,
            "ship consumables probe: the stand has one bay"
        );
        world
            .entity_mut(bays[0])
            .insert(ScriptedTorpedoOrder { target });
    }

    /// The torpedoes homing on the player, with their distance in meters.
    fn homing(world: &World) -> Vec<(Entity, Meters)> {
        let player = player(world);
        let Some(at) = world.get::<Transform>(player).map(|pose| pose.translation) else {
            return Vec::new();
        };
        world
            .try_query_filtered::<(Entity, &Transform, &TorpedoTargetEntity), With<TorpedoProjectileMarker>>()
            .map(|mut torpedoes| {
                torpedoes
                    .iter(world)
                    .filter(|(_, _, target)| ***target == player)
                    .map(|(torpedo, pose, _)| {
                        (torpedo, Meters::from_engine(pose.translation.distance(at)))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn torpedoes(world: &World) -> usize {
        world
            .try_query_filtered::<Entity, With<TorpedoProjectileMarker>>()
            .map(|mut query| query.iter(world).count())
            .unwrap_or_default()
    }

    fn a_torpedo_in_band() -> Arc<Predicate> {
        when(|world| {
            homing(world)
                .iter()
                .any(|(_, distance)| distance.0 > 400.0 && distance.0 < 3000.0)
        })
    }

    fn record_health(world: &mut World) {
        let sum = health_sum(world);
        world.resource_mut::<HealthAtFlare>().0 = sum;
    }

    fn fire_flare(world: &mut World) {
        record_health(world);
        use_slot(ConsumableSlotType::Flare)(world);
    }

    fn assert_the_decoy(world: &mut World) {
        assert_use(
            world,
            FLARE,
            ConsumableOutcomeType::Decoyed { torpedoes: 1 },
        );
        assert_eq!(
            hold(world).2,
            HOLD_EACH - 1,
            "ship consumables probe: one flare spent"
        );
        assert!(
            homing(world).is_empty(),
            "ship consumables probe: no torpedo homes on the player after the flare"
        );
        let mut decoyed = world.query_filtered::<(), (
            With<TorpedoProjectileMarker>,
            With<TorpedoTargetPosition>,
            Without<TorpedoTargetEntity>,
        )>();
        assert_eq!(
            decoyed.iter(world).count(),
            1,
            "ship consumables probe: the torpedo flies to a point"
        );
    }

    /// Claim 2: the decoyed torpedo fuzed away from the ship.
    fn assert_the_miss(world: &mut World) {
        let before = world.resource::<HealthAtFlare>().0;
        let after = health_sum(world);
        assert!(
            (after - before).abs() < 1e-3,
            "ship consumables probe: no player section lost health, {before} then {after}"
        );
        info!("ship consumables probe: PASS a decoy flare pulled a torpedo off the ship");
        marker(
            world,
            "outcome: a decoy flare pulls a torpedo off",
            serde_json::json!({ "health": after }),
        );
    }

    /// Claim 3: a flare with no torpedo in its band was refused and kept.
    fn assert_the_flare_refused(world: &mut World) {
        assert_use(
            world,
            FLARE,
            ConsumableOutcomeType::Refused(ConsumableRefusalType::NoTorpedo),
        );
        assert_eq!(
            hold(world).2,
            HOLD_EACH - 1,
            "ship consumables probe: the refused flare kept"
        );
        info!("ship consumables probe: PASS a flare with no torpedo was refused and kept");
        marker(
            world,
            "outcome: a flare with no torpedo is refused",
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
                ship_by_id(world, STAND_ID).is_some()
                    && ship_by_id(world, PLAYER_ID).is_some_and(|player| {
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
            .on_enter(use_slot(ConsumableSlotType::Repair))
            .until(answered(1))
            .deadline(SESSION_SECS)
            .add()
            .step("ship consumables: check the kit started")
            .on_enter(assert_the_kit_started)
            .until(frames(1))
            .add()
            .step("ship consumables: use a second kit while it runs")
            .on_enter(use_slot(ConsumableSlotType::Repair))
            .until(answered(2))
            .deadline(SESSION_SECS)
            .add()
            .step("ship consumables: check the second kit was refused")
            .on_enter(assert_the_kit_refused)
            .until(frames(1))
            .add()
            .step("ship consumables: run the repair on the flight clock")
            .until(when(|world| {
                running(world, ConsumableSlotType::Repair).is_none()
            }))
            .deadline(SESSION_SECS)
            .add()
            .step("ship consumables: check the repair")
            .on_enter(assert_the_repair)
            .until(frames(1))
            .add()
            .step("ship consumables: use a boost cell")
            .on_enter(use_slot(ConsumableSlotType::ThrustBoost))
            .until(answered(3))
            .deadline(SESSION_SECS)
            .add()
            .step("ship consumables: check the boost started")
            .on_enter(assert_the_boost_started)
            .until(frames(1))
            .add()
            .step("ship consumables: use a second cell while it runs")
            .on_enter(use_slot(ConsumableSlotType::ThrustBoost))
            .until(answered(4))
            .deadline(SESSION_SECS)
            .add()
            .step("ship consumables: check the second cell was refused")
            .on_enter(assert_the_boost_refused)
            .until(frames(1))
            .add()
            .step("ship consumables: fire a torpedo at the player")
            .on_enter(order_torpedo)
            .until(a_torpedo_in_band())
            .deadline(SESSION_SECS)
            .add()
            .step("ship consumables: fire a decoy flare")
            .on_enter(fire_flare)
            .until(answered(5))
            .deadline(SESSION_SECS)
            .add()
            .step("ship consumables: check the decoy")
            .on_enter(assert_the_decoy)
            .until(frames(1))
            .add()
            .step("ship consumables: the decoyed torpedo fuzes")
            .until(when(|world| torpedoes(world) == 0))
            .deadline(SESSION_SECS)
            .add()
            .step("ship consumables: check the miss")
            .on_enter(assert_the_miss)
            .until(frames(1))
            .add()
            .step("ship consumables: fire a flare with no torpedo")
            .on_enter(use_slot(ConsumableSlotType::Flare))
            .until(answered(6))
            .deadline(SESSION_SECS)
            .add()
            .step("ship consumables: check the refused flare")
            .on_enter(assert_the_flare_refused)
            .add()
    }
}
