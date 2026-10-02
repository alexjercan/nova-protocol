//! mine_and_sell: cut ore out of a rock, take one of its canisters aboard and
//! sell exactly that canister's ore to a docked trader.
//!
//! `screenshot_mining_beam` proves the beam and the canisters, `trade_loop`
//! proves the Inventory pane's trades, and the intake's ECS examples prove the
//! take. None of them proves that the ore a pulse cuts is the ore a player
//! sells. This example runs the three in one session on their production
//! paths: the real `mine` key on the shipped line warship's bow emitter, a
//! canister the rock drops taken whole through its dorsal cargo intake, the
//! real `dock` key on a trader's port, and Sell through the pane.
//!
//! The trader is a live spar that is not lootable and holds credits. It
//! exists only in this example: no shipped scenario carries it.
//!
//! What to do:
//!
//! 1. lock the rock with the radar and hold the mine key (`V`) until ore
//!    pays, then let it up.
//! 2. fly the dorsal intake up under a canister the rock drops, slowly.
//! 3. fly the starboard collar onto the trader's port, lock the trader and
//!    press the dock key.
//! 4. press TAB, pick Inventory, click Stone ore on your side, pick Sell and
//!    All, and Confirm.
//!
//! ```text
//! cargo run --example mine_and_sell --features debug
//! ```
//!
//! Under `NOVA_AUTOPILOT=1` the script plays all four steps. It keys the beam
//! until at least five ore is cut, which drops more than one canister, and
//! tracks the one holding the most ore. It flies the warship with the flight
//! computer's `MatchVelocity`, the primitive every AI pilot steers with: down
//! clear of the canisters, level under the tracked one, and slowly up onto it.
//! It then holds the berth beside the trader
//! and docks with the real key. The other canisters stay in space, so the ore
//! sold is the ore collected, not the ore mined. It fails loudly if the rock's
//! lost corners, the pulse log and the ore in canisters, the queue and the
//! hold disagree, if a canister leaves without a take, if the intake takes
//! anything but the tracked canister or adds other than exactly its ore and
//! mass to the hold, or if the Sell does not move exactly that ore and its
//! bid between the two ships. `NOVA_CAPTURE=1` also writes the mining, the
//! pickup, the take and the Sell frames, and records the last meters of the
//! lift through the take as the site's `loop-section-cargo-intake` webm.
//!
//! The walk is long on lavapipe: set `NOVA_AUTOPILOT_DEADLINE` past the
//! default backstop.

#[path = "../screenshots/shared/kit.rs"]
mod kit;

use bevy::prelude::*;
use clap::Parser;
use docking_pair::spar;
use nova_protocol::prelude::*;

#[path = "shared/docking_pair.rs"]
#[expect(
    dead_code,
    reason = "this example docks with the spar alone; the tender is the other examples' hull"
)]
mod docking_pair;

#[derive(Parser)]
#[command(name = "mine_and_sell")]
#[command(version = "1.0.0")]
#[command(about = "Mine ore, take its canister aboard and sell it to a docked trader", long_about = None)]
struct Cli;

/// Scenario id of the warship the player flies.
const PLAYER_ID: &str = "miner";
/// The warship's name.
const PLAYER_NAME: &str = "Miner";
/// Scenario id of the ore rock.
const ROCK_ID: &str = "ore_rock";
/// Scenario id of the trader.
const TRADER_ID: &str = "trader";
/// The trader's name, which the pane's note line reads.
const TRADER_NAME: &str = "Trader";

/// The rock of `screenshot_mining_beam`: dead ahead of the line warship's
/// bow emitter, its near face about 58 m out, clear of the bow.
const ROCK_CENTRE: Vec3 = Vec3::new(1.0, 1.0, -23.5);
const ROCK_RADIUS: Meters = Meters(25.0);
const ROCK_SEED: u32 = 7;

/// Where the warship's root rests to dock, in engine units: aft of the rock
/// and below the line the canister drifts along, so no canister crosses the
/// berth.
const BERTH: Vec3 = Vec3::new(0.0, -5.0, 12.0);

/// Where the trader stands, in engine units: to starboard of the berth,
/// turned so its port faces the warship's starboard collar. The collar face
/// stands 25 m off the warship's root and the port face 25 m off the spar's,
/// which leaves a 5 m face gap, half the capture distance.
const TRADER_POSITION: Vec3 = Vec3::new(BERTH.x + 5.5, BERTH.y, BERTH.z);

/// What the warship holds at spawn: an empty hold and a few credits.
const PLAYER_CREDITS: u32 = 50;
/// What the trader carries at spawn: no ore, so every ore it ends with is
/// the ore it bought.
const TRADER_STOCK: &[(ItemType, u32)] = &[(ItemType::Rations, 10)];
/// The trader's credits at spawn: enough for any one pulse's ore.
const TRADER_CREDITS: u32 = 1_000;

/// Every [`MiningPulse`] outcome in order.
#[derive(Resource, Default)]
struct PulseLog(Vec<Result<u32, MiningRefusalType>>);

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new()
        .with_game_plugins(mine_and_sell_plugin)
        .build();

    #[cfg(feature = "debug")]
    walk::add(&mut app);

    app.run()
}

fn mine_and_sell_plugin(app: &mut App) {
    app.init_resource::<PulseLog>();
    app.add_observer(|pulse: On<MiningPulse>, mut log: ResMut<PulseLog>| {
        log.0.push(pulse.outcome);
    });
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn ship_object(
    id: &str,
    name: &str,
    position: Vec3,
    rotation: Quat,
    ship: SpaceshipConfig,
) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position: Meters3::from_engine(position),
            rotation,
        },
        kind: ScenarioObjectKind::Spaceship(ship),
    })
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    let player = ship_object(
        PLAYER_ID,
        PLAYER_NAME,
        Vec3::ZERO,
        Quat::IDENTITY,
        SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig::default()),
            design: ShipDesignSource::Inline(kit::catalog_ship(&ships, "block_line_warship")),
            inventory: ShipInventoryStock::new([]),
            credits: PLAYER_CREDITS,
            ..default()
        },
    );
    // Live and not lootable: a docked ship the pane trades with rather than
    // takes from. Its credits pay for the Sell.
    let trader = ship_object(
        TRADER_ID,
        TRADER_NAME,
        TRADER_POSITION,
        Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        SpaceshipConfig {
            controller: SpaceshipController::None,
            design: ShipDesignSource::Inline(spar()),
            allegiance: Some(Allegiance::Neutral),
            inventory: ShipInventoryStock::new(TRADER_STOCK.iter().copied()),
            lootable: false,
            credits: TRADER_CREDITS,
            ..default()
        },
    );
    let rock = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ROCK_ID.to_string(),
            name: "Ore Rock".to_string(),
            position: Meters3::from_engine(ROCK_CENTRE),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: ROCK_RADIUS,
            texture: game_assets.asteroid_texture.clone().into(),
            kind: KIND_ROCK.into(),
            destroy_sound: None,
            mass: None,
            lock_signature: None,
            seed: Some(ROCK_SEED),
        }),
    });
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "Mine a rock, take the ore aboard and sell it to a docked trader".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![player, rock, trader],
                ThreePointRig::around("mine", Meters3::from_engine(Vec3::new(0.0, -2.0, 0.0)), 8.0)
                    .actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "mine_and_sell".to_string(),
            "Mine and Sell".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// The harnessed walk and the flight computer it flies the pickup and the
/// berth with.
#[cfg(feature = "debug")]
mod walk {
    use std::{collections::BTreeMap, sync::Arc};

    use avian3d::prelude::LinearVelocity;
    use nova_protocol::{
        nova_debug::harness::{hide_status_bar, AutopilotPlugin, LoopCapturePlugin, Predicate},
        nova_interface::pane::InterfacePaneType,
    };

    use super::*;

    /// The line warship's emitter face centre, in engine units from its root:
    /// the -Z face of its cell at (1, 1, -7).
    const EMITTER_FACE: Vec3 = Vec3::new(1.0, 1.0, -7.5);

    /// The rock's near face on the beam line, as meshed from its seed.
    const ROCK_NEAR_FACE: Vec3 = Vec3::new(1.0, 1.0, -13.3);

    /// The ore the rock's kind yields.
    const ORE: ItemType = ItemType::StoneOre;

    /// Real-seconds backstop for a state wait, long enough for lavapipe
    /// frames.
    const STEP_DEADLINE: f32 = 120.0;
    /// Real-seconds backstop for a flown leg: the canister drifts at 2 m/s
    /// from the rock to the intake.
    const FLIGHT_DEADLINE: f32 = 900.0;

    /// The least z the warship's root is flown to, in engine units, so its
    /// bow keeps clear of the rock while it waits under the canister.
    const SAFE_Z: f32 = 0.0;
    /// How far below the canister the intake face waits while the warship
    /// levels under it, in engine units: clear of the bridge and the turrets.
    const HOVER_CLEARANCE: f32 = 2.5;
    /// The gap the lift closes to between the face and the canister's nearest
    /// side, in engine units: inside the intake's 1 m capture gap.
    const LIFT_GAP: f32 = 0.02;
    /// How near the face centre must stand under the canister, across the
    /// face, before the lift starts, in engine units.
    const LIFT_ALIGN: f32 = 0.1;
    /// The fastest the lift closes on the canister, in engine units per
    /// second: slow, so the canister meets the trigger slab in front of the
    /// opening and not the hull beside it.
    const LIFT_SPEED: f32 = 0.25;
    /// Velocity per engine unit of position error, per second.
    const GAIN: f32 = 0.8;
    /// The largest correction the helm asks for, in engine units per second:
    /// small enough for the RCS to trim with the nose held.
    const MAX_CORRECTION: f32 = 0.4;
    /// How near a held point the warship must rest, in engine units and
    /// engine units per second.
    const HOLD_ARRIVAL: f32 = 0.08;
    const HOLD_REST: f32 = 0.03;
    /// Ore to cut before the mine key comes up: more than one, so a count of
    /// canisters or of pulses cannot pass for a count of ore.
    const MINED_ORE_FLOOR: u32 = 5;
    /// The canister's nearest-side gap to the face at which the pickup shot
    /// is taken, in engine units.
    const PICKUP_SHOT_GAP: f32 = 0.5;

    /// Where the warship closes on the berth from: beside it, clear of the
    /// spar, so the last leg is straight across the gap.
    const BERTH_APPROACH: Vec3 = Vec3::new(BERTH.x - 2.0, BERTH.y, BERTH.z);

    /// The frames the capture path writes.
    const MINING_SHOT: &str = "mine-sell-mining.png";
    const PICKUP_SHOT: &str = "wiki-section-cargo-intake.png";
    const TAKEN_SHOT: &str = "mine-sell-taken.png";
    const SOLD_SHOT: &str = "mine-sell-sold.png";
    /// The site's intake loop: from the pickup shot through the take.
    const INTAKE_LOOP: &str = "loop-section-cargo-intake";
    /// Game seconds the loop holds after the take.
    const INTAKE_LOOP_TAIL_SECS: f32 = 1.0;

    /// What the flight computer is asked to do.
    #[derive(Resource, Default, Clone, Copy, PartialEq)]
    enum Helm {
        /// Nothing: the script owns the ship.
        #[default]
        Off,
        /// Hold the intake face under `canister`, then close on it.
        Track { canister: Entity, lift: bool },
        /// Come to rest with the root at a world point.
        Hold(Vec3),
    }

    /// What the walk records to compare later.
    #[derive(Resource, Default)]
    struct FlowProof {
        /// The rock's solid corners when its field first exists.
        seeded_corners: Option<u32>,
        /// Every canister seen, with the ore it held.
        canisters: BTreeMap<Entity, u32>,
        /// [`CargoCanisterTaken`] events.
        taken: u32,
        /// The canister the helm tracks: the one holding the most ore once
        /// the rock has dropped them all.
        target: Option<Entity>,
        /// The ore the hold took, once the take is checked.
        collected: Option<u32>,
    }

    pub(super) fn add(app: &mut App) {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(LoopCapturePlugin::default());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.init_resource::<Helm>();
        app.init_resource::<FlowProof>();
        app.add_observer(|_: On<CargoCanisterTaken>, mut proof: ResMut<FlowProof>| {
            proof.taken += 1;
        });
        app.add_systems(Update, steer_helm);
        // `Last`: after every canister spawn and take this frame.
        app.add_systems(Last, record_proof);
        app.add_plugins(script());
    }

    /// Fly [`Helm`] with the flight computer's `MatchVelocity`, the nose held
    /// forward.
    fn steer_helm(
        mut commands: Commands,
        mut helm: ResMut<Helm>,
        mut q_player: Query<
            (Entity, &GlobalTransform, Option<&mut Autopilot>),
            With<PlayerSpaceshipMarker>,
        >,
        q_intakes: Query<
            (&ChildOf, &GlobalTransform, &SectionCollider),
            With<CargoIntakeSectionMarker>,
        >,
        q_canisters: Query<(&GlobalTransform, &LinearVelocity), With<CargoCanister>>,
    ) {
        if *helm == Helm::Off {
            return;
        }
        let Ok((ship, ship_pose, autopilot)) = q_player.single_mut() else {
            return;
        };
        let at = ship_pose.translation();
        let velocity = match *helm {
            Helm::Off => return,
            Helm::Hold(point) => ((point - at) * GAIN).clamp_length_max(MAX_CORRECTION),
            Helm::Track { canister, lift } => {
                let Some((_, intake, collider)) = q_intakes
                    .iter()
                    .find(|(&ChildOf(parent), ..)| parent == ship)
                else {
                    panic!("mine_and_sell: the warship has no cargo intake");
                };
                let (face, normal) =
                    cargo_intake_face(intake.translation(), intake.rotation(), *collider);
                match q_canisters.get(canister) {
                    // Taken or gone: hold still for the checks.
                    Err(_) => Vec3::ZERO,
                    Ok((canister_pose, canister_velocity)) => {
                        let centre = canister_pose.translation();
                        let half = (canister_pose.rotation().inverse() * normal)
                            .abs()
                            .dot(CARGO_CANISTER_SIZE * 0.5);
                        let clearance = if lift {
                            half + LIFT_GAP
                        } else {
                            HOVER_CLEARANCE
                        };
                        let mut goal = at + (centre - normal * clearance - face);
                        let held_back = goal.z < SAFE_Z;
                        goal.z = goal.z.max(SAFE_Z);
                        let mut feed = canister_velocity.0;
                        if held_back {
                            feed.z = 0.0;
                        }
                        let error = goal - at;
                        let mut velocity = feed + (error * GAIN).clamp_length_max(MAX_CORRECTION);
                        if lift {
                            let along = velocity.dot(normal);
                            velocity += normal * (along.clamp(-LIFT_SPEED, LIFT_SPEED) - along);
                        } else if !held_back
                            && error.reject_from_normalized(normal).length() < LIFT_ALIGN
                            && error.dot(normal).abs() < LIFT_ALIGN
                        {
                            info!("mine_and_sell: level under the canister, lifting");
                            *helm = Helm::Track {
                                canister,
                                lift: true,
                            };
                        }
                        velocity
                    }
                }
            }
        };
        let action = AutopilotAction::MatchVelocity {
            velocity,
            facing: Some(Dir3::NEG_Z),
        };
        match autopilot {
            Some(mut autopilot) => {
                if autopilot.action != action {
                    autopilot.action = action;
                }
            }
            None => {
                commands.entity(ship).insert(Autopilot::engage(action));
            }
        }
    }

    /// Record the rock's solid corners when its field first exists, and every
    /// canister's ore.
    fn record_proof(world: &mut World) {
        let seen: Vec<(Entity, u32, u32)> = world
            .query::<(Entity, &CargoCanister)>()
            .iter(world)
            .map(|(entity, canister)| (entity, ore_in(canister), canister.total_mass_g()))
            .collect();
        for (entity, ore, mass) in seen {
            assert_eq!(
                mass,
                ore * ORE.mass_g(),
                "mine_and_sell: a canister's mass is not its ore's"
            );
            let mut proof = world.resource_mut::<FlowProof>();
            match proof.canisters.insert(entity, ore) {
                None => info!("mine_and_sell: canister {entity:?} holds {ore} ore"),
                Some(recorded) => {
                    assert_eq!(recorded, ore, "mine_and_sell: a canister's ore changed");
                }
            }
        }
        if world.resource::<FlowProof>().seeded_corners.is_some() {
            return;
        }
        if let Some(corners) = solid_corners(world) {
            assert_eq!(
                logged_corners(world),
                0,
                "a pulse carved the field before it was counted"
            );
            info!("mine_and_sell: seeded field holds {corners} solid corner(s)");
            world.resource_mut::<FlowProof>().seeded_corners = Some(corners);
        }
    }

    /// The ore one canister holds.
    fn ore_in(canister: &CargoCanister) -> u32 {
        canister
            .stacks()
            .filter(|(item, _)| *item == ORE)
            .map(|(_, count)| count)
            .sum()
    }

    /// The rock's solid corners, once its field exists.
    fn solid_corners(world: &mut World) -> Option<u32> {
        let mut fields = world.query::<&AsteroidField>();
        let field = fields.iter(world).next()?.solid();
        Some((field.solid_volume() / field.cell_size().powi(3)).round() as u32)
    }

    /// Corners the pulse log says were flipped.
    fn logged_corners(world: &World) -> u32 {
        world
            .resource::<PulseLog>()
            .0
            .iter()
            .filter_map(|outcome| outcome.ok())
            .sum()
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn world_has<T: Component>(world: &World) -> bool {
        world
            .try_query_filtered::<(), With<T>>()
            .is_some_and(|mut query| query.iter(world).next().is_some())
    }

    fn set_paused(world: &mut World, paused: bool) {
        let mut time = world.resource_mut::<Time<Virtual>>();
        if paused {
            time.pause();
        } else {
            time.unpause();
        }
    }

    fn root(world: &mut World, id: &str) -> Entity {
        kit::ship_root(world, id).unwrap_or_else(|| panic!("mine_and_sell: no ship {id}"))
    }

    fn inventory(world: &mut World, id: &str) -> ShipInventory {
        let ship = root(world, id);
        world
            .get::<ShipInventory>(ship)
            .unwrap_or_else(|| panic!("mine_and_sell: {id} has no ShipInventory"))
            .clone()
    }

    fn credits(world: &mut World, id: &str) -> u32 {
        let ship = root(world, id);
        world
            .get::<ShipCredits>(ship)
            .unwrap_or_else(|| panic!("mine_and_sell: {id} has no ShipCredits"))
            .0
    }

    /// Check the ore ledger and return the ore picked up.
    ///
    /// Every corner the pulses flipped left the rock's field, and is one ore
    /// in a live canister, in the rock's queue, or in a canister an intake
    /// took. Every canister that left the world left through a take, and
    /// the ore taken is the ore both holds carry: nothing made, lost or
    /// counted twice.
    fn check_ledger(world: &mut World, stage: &str) -> u32 {
        assert!(
            !world_has::<MinedOre>(world),
            "mine_and_sell {stage}: ore is still owed"
        );
        let flipped = logged_corners(world);
        let seeded = world
            .resource::<FlowProof>()
            .seeded_corners
            .expect("the seeded field was counted");
        let left = solid_corners(world).expect("the rock keeps its field");
        assert_eq!(
            seeded - left,
            flipped,
            "mine_and_sell {stage}: the field lost what the pulses flipped"
        );
        let live: BTreeMap<Entity, u32> = world
            .query::<(Entity, &CargoCanister)>()
            .iter(world)
            .map(|(entity, canister)| (entity, ore_in(canister)))
            .collect();
        let queued: u32 = world
            .query::<&MinedCanisterQueue>()
            .iter(world)
            .flat_map(MinedCanisterQueue::canisters)
            .map(ore_in)
            .sum();
        let proof = world.resource::<FlowProof>();
        let gone: Vec<u32> = proof
            .canisters
            .iter()
            .filter(|(entity, _)| !live.contains_key(entity))
            .map(|(_, ore)| *ore)
            .collect();
        let taken = proof.taken;
        let spawned: u32 = live.values().sum();
        let picked: u32 = gone.iter().sum();
        let held = inventory(world, PLAYER_ID).count(ORE) + inventory(world, TRADER_ID).count(ORE);
        info!(
            "mine_and_sell {stage}: {flipped} ore mined = {spawned} in {} live canister(s) \
             + {queued} queued + {picked} picked up in {} canister(s); holds carry {held}",
            live.len(),
            gone.len()
        );
        assert_eq!(
            gone.len(),
            taken as usize,
            "mine_and_sell {stage}: a canister left the world without a take"
        );
        assert_eq!(
            spawned + queued + picked,
            flipped,
            "mine_and_sell {stage}: mined ore is not spawned + queued + picked up"
        );
        assert_eq!(
            held, picked,
            "mine_and_sell {stage}: the holds carry other ore than the intake took"
        );
        picked
    }

    /// The step after the take: the tracked canister, and no other, is gone
    /// through a take, and the hold gained exactly its ore and mass.
    fn check_take(world: &mut World) {
        let proof = world.resource::<FlowProof>();
        let target = proof.target.expect("a canister was tracked");
        let target_ore = proof.canisters[&target];
        assert_eq!(
            proof.taken, 1,
            "mine_and_sell: the intake took one canister"
        );
        assert!(
            world.get_entity(target).is_err(),
            "mine_and_sell: the tracked canister is still in the world"
        );
        let picked = check_ledger(world, "after the take");
        assert_eq!(
            picked, target_ore,
            "the intake took the tracked canister alone"
        );
        let hold = inventory(world, PLAYER_ID);
        assert_eq!(hold.count(ORE), picked, "the hold holds the picked-up ore");
        assert_eq!(
            hold.used_g(),
            picked * ORE.mass_g(),
            "the hold's mass is the picked-up ore's"
        );
        assert_eq!(credits(world, PLAYER_ID), PLAYER_CREDITS);
        world.resource_mut::<FlowProof>().collected = Some(picked);
    }

    /// The step after the Sell: exactly the collected ore moved at its bid,
    /// and nothing else changed.
    fn check_sell(world: &mut World) {
        let collected = world
            .resource::<FlowProof>()
            .collected
            .expect("the take was checked");
        assert_eq!(check_ledger(world, "after the Sell"), collected);
        let price = collected * ORE.bid_cr();
        let own = inventory(world, PLAYER_ID);
        let partner = inventory(world, TRADER_ID);
        assert!(own.is_empty(), "the warship's hold is empty after the Sell");
        let mut expected: Vec<(ItemType, u32)> = TRADER_STOCK.to_vec();
        expected.push((ORE, collected));
        expected.sort_by_key(|(item, _)| *item);
        let mut stacks: Vec<(ItemType, u32)> = partner.stacks().collect();
        stacks.sort_by_key(|(item, _)| *item);
        assert_eq!(stacks, expected, "the trader holds its stock and the ore");
        let own_credits = credits(world, PLAYER_ID);
        let partner_credits = credits(world, TRADER_ID);
        info!(
            "mine_and_sell: sold {collected} ore for {price} cr: warship {own_credits} cr, \
             trader {partner_credits} cr"
        );
        assert_eq!(own_credits, PLAYER_CREDITS + price);
        assert_eq!(partner_credits, TRADER_CREDITS - price);
    }

    /// The note line the Sell shows.
    fn sell_note(world: &World) -> Option<String> {
        let collected = world.resource::<FlowProof>().collected?;
        Some(format!(
            "Sold {collected} {} to {TRADER_NAME} for {} cr",
            ORE.label(),
            collected * ORE.bid_cr()
        ))
    }

    /// Advance once exactly one laid-out, visible UI text reads the Sell's
    /// note.
    fn sell_note_shown() -> Arc<Predicate> {
        when(|world| {
            let Some(note) = sell_note(world) else {
                return false;
            };
            world
                .try_query::<(&Text, &ComputedNode, &InheritedVisibility)>()
                .is_some_and(|mut texts| {
                    texts
                        .iter(world)
                        .filter(|(text, node, visible)| {
                            text.0 == note && visible.get() && node.size().x > 0.0
                        })
                        .count()
                        == 1
                })
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

    /// The player's root pose and speed.
    fn player_motion(world: &World) -> Option<(Vec3, f32)> {
        let mut query = world
            .try_query_filtered::<(&GlobalTransform, &LinearVelocity), With<PlayerSpaceshipMarker>>(
            )?;
        let (pose, velocity) = query.iter(world).next()?;
        Some((pose.translation(), velocity.0.length()))
    }

    fn rests_at(point: Vec3) -> Arc<Predicate> {
        when(move |world| {
            player_motion(world)
                .is_some_and(|(at, speed)| at.distance(point) < HOLD_ARRIVAL && speed < HOLD_REST)
        })
    }

    /// The tracked canister's nearest-side gap to the intake face, while it
    /// exists.
    fn pickup_gap(world: &World) -> Option<f32> {
        let target = world.resource::<FlowProof>().target?;
        let canister = world.get::<GlobalTransform>(target)?;
        let mut intakes = world.try_query_filtered::<(&GlobalTransform, &SectionCollider), With<CargoIntakeSectionMarker>>()?;
        let (intake, collider) = intakes.iter(world).next()?;
        let (face, normal) = cargo_intake_face(intake.translation(), intake.rotation(), *collider);
        let half = (canister.rotation().inverse() * normal)
            .abs()
            .dot(CARGO_CANISTER_SIZE * 0.5);
        Some((canister.translation() - face).dot(normal) - half)
    }

    fn lock(world: &mut World, target: Entity) {
        let player = root(world, PLAYER_ID);
        world.entity_mut(player).insert(TravelLock(Some(target)));
    }

    /// Frame the bow, the beam and the rock's near face from starboard, level
    /// with the beam.
    fn frame_beam(world: &mut World) {
        let subject = (EMITTER_FACE + ROCK_NEAR_FACE) * 0.5;
        pose_camera(
            world,
            Meters3::from_engine(subject + Vec3::new(9.0, 0.0, 0.0)),
            Meters3::from_engine(subject),
        );
    }

    /// Frame the intake face and the canister over it from starboard, aft and
    /// above.
    fn frame_intake(world: &mut World) {
        let mut intakes = world
            .query_filtered::<(&GlobalTransform, &SectionCollider), With<CargoIntakeSectionMarker>>(
            );
        let (intake, collider) = intakes
            .iter(world)
            .next()
            .expect("the warship has an intake");
        let (face, normal) = cargo_intake_face(intake.translation(), intake.rotation(), *collider);
        let subject = face + normal * 0.6;
        pose_camera(
            world,
            Meters3::from_engine(subject + Vec3::new(4.0, 2.5, 4.0)),
            Meters3::from_engine(subject),
        );
    }

    /// Pause, frame and shoot one checkpoint, and wait for it.
    fn shoot_checkpoint(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        frame: fn(&mut World),
        path: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("frame the {label}"))
            .on_enter(move |world: &mut World| {
                set_paused(world, true);
                frame(world);
            })
            .until(frames(3))
            .add()
            .step(format!("shoot the {label}"))
            .on_enter(move |world: &mut World| shoot(world, path))
            .until(shot_written(path))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step(format!("resume after the {label}"))
            .on_enter(|world: &mut World| set_paused(world, false))
            .until(frames(1))
            .add()
    }

    /// Mine, take, dock and sell, checking the ledger at each hand-off.
    fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("load the warship, the rock and the trader")
            .enter(GameStates::Loading)
            .until(and(player_ship_present(), scenario_camera_present()))
            .deadline(STEP_DEADLINE)
            .add()
            .step("lock the rock")
            .on_enter(|world: &mut World| {
                let rock = {
                    let mut rocks =
                        world.query_filtered::<(Entity, &EntityId), With<AsteroidMarker>>();
                    rocks
                        .iter(world)
                        .find(|(_, id)| id.0 == ROCK_ID)
                        .map(|(entity, _)| entity)
                        .expect("the rock spawned")
                };
                lock(world, rock);
                frame_beam(world);
                hide_status_bar(world);
            })
            .until(and(scenario_is_built(), frames(10)))
            .deadline(STEP_DEADLINE)
            .add()
            .step("hold the mine key until enough ore is cut")
            .on_enter(press_action("mine"))
            .until(when(|world| logged_corners(world) >= MINED_ORE_FLOOR))
            .deadline(STEP_DEADLINE)
            .add()
            .step("check the beam holds a hit")
            .on_enter(|world: &mut World| {
                // Paused until the release, so no further pulse pays.
                set_paused(world, true);
                let hit = world_has::<MiningBeamHit>(world);
                assert!(hit, "mine_and_sell: the paying beam holds a hit");
            })
            .until(frames(1))
            .add();
        let script = script
            .step("shoot the mining")
            .on_enter(|world: &mut World| shoot(world, MINING_SHOT))
            .until(shot_written(MINING_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("release the mine key and wait for the canisters")
            .on_enter(|world: &mut World| {
                release_action("mine")(world);
                set_paused(world, false);
            })
            .until(when(|world| {
                !world_has::<MinedOre>(world)
                    && !world_has::<MinedCanisterQueue>(world)
                    && !world_has::<MiningBeamHit>(world)
                    && !world.resource::<FlowProof>().canisters.is_empty()
            }))
            .deadline(STEP_DEADLINE)
            .add()
            .step("check the mined ore is all in canisters")
            .on_enter(|world: &mut World| {
                let log = world.resource::<PulseLog>().0.clone();
                info!("mine_and_sell: pulse log {log:?}");
                assert!(
                    log.iter().all(Result::is_ok),
                    "a pulse was refused: {log:?}"
                );
                let picked = check_ledger(world, "after the mining");
                assert_eq!(picked, 0, "nothing is picked up before the helm flies");
                let target = {
                    let mut proof = world.resource_mut::<FlowProof>();
                    let (&target, _) = proof
                        .canisters
                        .iter()
                        .max_by_key(|&(entity, ore)| (*ore, std::cmp::Reverse(*entity)))
                        .expect("the rock dropped a canister");
                    proof.target = Some(target);
                    target
                };
                info!("mine_and_sell: tracking canister {target:?}");
                *world.resource_mut::<Helm>() = Helm::Track {
                    canister: target,
                    lift: false,
                };
            })
            .until(frames(1))
            .add()
            .step("fly the intake under the canister and up onto it")
            .until(when(|world| {
                matches!(*world.resource::<Helm>(), Helm::Track { lift: true, .. })
                    && pickup_gap(world).is_some_and(|gap| gap <= PICKUP_SHOT_GAP)
            }))
            .deadline(FLIGHT_DEADLINE)
            .add();
        let script = shoot_checkpoint(script, "pickup", frame_intake, PICKUP_SHOT)
            .step("open the intake loop")
            .on_enter(|world: &mut World| loop_start(world, INTAKE_LOOP))
            .until(frames(1))
            .add()
            .step("wait for the take")
            .until(when(|world| {
                let proof = world.resource::<FlowProof>();
                proof.taken >= 1
                    && proof
                        .target
                        .is_some_and(|target| world.get_entity(target).is_err())
            }))
            .deadline(STEP_DEADLINE)
            .add()
            .step("check the take")
            .on_enter(check_take)
            .until(frames(1))
            .add()
            .step("hold the take to the end of the intake loop")
            .until(elapsed(INTAKE_LOOP_TAIL_SECS))
            .add()
            .step("close the intake loop")
            .on_enter(|world: &mut World| loop_end(world, INTAKE_LOOP))
            .until(loop_written(INTAKE_LOOP))
            .deadline(STEP_DEADLINE)
            .add();
        let script = shoot_checkpoint(script, "take", frame_intake, TAKEN_SHOT)
            .step("fly beside the berth")
            .on_enter(|world: &mut World| {
                *world.resource_mut::<Helm>() = Helm::Hold(BERTH_APPROACH);
            })
            .until(rests_at(BERTH_APPROACH))
            .deadline(FLIGHT_DEADLINE)
            .add()
            .step("fly onto the berth")
            .on_enter(|world: &mut World| {
                *world.resource_mut::<Helm>() = Helm::Hold(BERTH);
            })
            .until(rests_at(BERTH))
            .deadline(FLIGHT_DEADLINE)
            .add()
            .step("hand the ship back, lock the trader and press the dock key")
            .on_enter(|world: &mut World| {
                *world.resource_mut::<Helm>() = Helm::Off;
                let player = root(world, PLAYER_ID);
                world.entity_mut(player).remove::<Autopilot>();
                let trader = root(world, TRADER_ID);
                lock(world, trader);
                press_action("dock")(world);
            })
            .until(frames(1))
            .add()
            .step("let the key up and wait for the dock")
            .on_enter(release_action("dock"))
            .until(any_entity::<(With<PlayerSpaceshipMarker>, With<DockedShip>)>())
            .deadline(STEP_DEADLINE)
            .add()
            .step("check nothing moved on the way to the dock")
            .on_enter(|world: &mut World| {
                let collected = world.resource::<FlowProof>().collected;
                assert_eq!(Some(check_ledger(world, "docked")), collected);
                assert_eq!(inventory(world, PLAYER_ID).count(ORE), collected.unwrap());
            })
            .until(frames(1))
            .add()
            .step("press the interface key")
            .on_enter(press_action("interface_toggle"))
            .until(frames(1))
            .add()
            .step("let the key up and wait for the interface")
            .on_enter(release_action("interface_toggle"))
            .until(the_interface_shows(InterfacePaneType::Map))
            .deadline(STEP_DEADLINE)
            .add()
            .click_named(
                "open the Inventory pane",
                "InterfaceTabInventory",
                ui_node_present("InventoryRowOwnStoneOre"),
                STEP_DEADLINE,
            )
            .click_named(
                "sell: pick Stone ore",
                "InventoryRowOwnStoneOre",
                ui_node_present("InventoryDraftSell"),
                STEP_DEADLINE,
            )
            .click_named(
                "sell: switch to Sell",
                "InventoryDraftSell",
                pointer_released(),
                STEP_DEADLINE,
            )
            .click_named(
                "sell: take all",
                "InventoryDraftAll",
                pointer_released(),
                STEP_DEADLINE,
            )
            .click_named(
                "sell: confirm",
                "InventoryDraftConfirm",
                sell_note_shown(),
                STEP_DEADLINE,
            )
            .step("check the Sell")
            .on_enter(check_sell)
            .until(frames(1))
            .add();
        script
            .step("shoot the Sell")
            .on_enter(|world: &mut World| shoot(world, SOLD_SHOT))
            .until(shot_written(SOLD_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
    }
}
