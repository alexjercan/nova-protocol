//! world_encounters: open-world ship-encounter rules, proved on GENERATED
//! hulls flying a staged cast instead of a hand-authored fixture.
//!
//! Eight ships, each the world's own `generate_ship` output for its role at
//! the lowest advancement the snapshot serves: the PLAYER (Armored), an
//! Enemy raider (Scavenger) that arrives on a grace timer, an armed Neutral
//! picket (Armored) patrolling two waypoints, a second armed Neutral warden
//! (Armored) of the same civilization holding station, an unarmed Neutral
//! trader (Civilian) parked for docking, an unarmed Enemy hulk (Civilian)
//! that never fights and carries a stock and credits stated here, and a
//! live-fire pair far from the player: an armed Player-aligned escort
//! (Armored) and an armed Enemy marauder (Scavenger).
//! The scenario itself spawns the same way any hand-authored one does -
//! `SpaceshipConfig` with an inline design - so nothing here spawns through
//! the world-streaming path; only the GENERATOR is the open world's own.
//!
//! Only the escort and the marauder carry ammunition. Every other hull flies
//! with empty magazines, and the marauder is staged 30 km from the escort and
//! 60 km from the player, outside every sensor, until beat 9 moves it in, so
//! no live round is ever aimed at the player.
//!
//! # Hand-run
//!
//! ```text
//! cargo run --example world_encounters --features debug
//! ```
//!
//! Harnessed mode (`NOVA_AUTOPILOT=1`) drives fourteen beats in order, each
//! advancing only once the world state it claims is actually true, and
//! aborting the run - named - if a beat's deadline expires first:
//!
//! | # | beat | claim |
//! | - | - | - |
//! | 1 | grace | the raider holds its passive routine while `AIEngageGrace` runs, then engages once it finishes |
//! | 2 | patrol hold | the picket arrives at a waypoint, holds (`AIPatrolRoute.hold` is `Some`), then advances |
//! | 3 | calm dock | the player's admitted request to the trader docks, then releases |
//! | 4 | retaliation | hitting the picket sets its `RetaliationTarget`, makes `ship_relation` read Hostile, paints its HUD triangle threat red while its side stays Neutral, and, set port to port, its dock request is refused |
//! | 5 | calm bystander | the warden, Neutral like the picket, answers nobody and does not engage while the picket answers the player |
//! | 6 | newest shooter | a hit from the hulk replaces the player as the picket's target; the warden stays calm |
//! | 7 | leash | the hulk leaving the picket's territory clears the target and returns the picket to a passive routine |
//! | 8 | forced undock | docking with the trader again, then hitting it, ends the dock at once |
//! | 9 | ally engages | the escort and the marauder target each other, engage, and each lands a real projectile hit on the other |
//! | 10 | ally stays allied | the player hits the escort as the pair engages; the escort answers nobody, keeps the marauder as its target through the exchange of fire, and the pair reads `Own` |
//! | 11 | unarmed Enemy | the hulk sees the player as hostile inside its engage range and still targets nothing; set port to port, its dock request is refused |
//! | 12 | last thruster | the hulk with one working thruster left is not neutralized; losing it neutralizes the hulk, which ends the picket's retaliation |
//! | 13 | boarding | the neutralized Enemy hulk admits the player's dock |
//! | 14 | loot | through the Inventory pane's widgets, All and Confirm Take each whole seeded stack and Take credits takes the whole balance; each move is exact, never partial |
//!
//! Beat 10's hit lands between beat 9's engagement and beat 9's record: the
//! escort can neutralize the marauder within a second of the first exchange,
//! so the hit cannot wait for beat 9's frame.
//!
//! Every hit that reaches a ship root is written to a ledger; the run ends by
//! asserting no fired round struck the player. Contact damage still counts:
//! the docking beats set hulls down port to port, and the pair can bump as
//! they part, so the last step logs those hits rather than failing on them.
//! Beats 4, 6, 8, 10 and 12 apply damage with a named source rather than
//! flying a shot; beat 9 is the only one whose hits are real rounds.
//!
//! `NOVA_CAPTURE=1` additionally writes one
//! `world-encounters-beat<N>-<name>.png` per beat under `NOVA_CAPTURE_DIR`.

#[path = "../screenshots/shared/kit.rs"]
mod kit;

#[cfg(feature = "debug")]
use std::sync::Arc;

#[cfg(feature = "debug")]
use avian3d::prelude::{AngularVelocity, ComputedCenterOfMass, LinearVelocity, Position, Rotation};
#[cfg(feature = "debug")]
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use nova_probe::probe_marker;
use nova_protocol::prelude::*;
use nova_world::prelude::{CivilizationId, ShipRoleType};

#[derive(Parser)]
#[command(name = "world_encounters")]
#[command(version = "1.0.0")]
#[command(
    about = "Open-world ship-encounter rules, proved on generated hulls flying a staged cast",
    long_about = None
)]
struct Cli;

/// The world seed every generation request names. Pinned so the cast is the
/// same hull set on every run.
const WORLD_SEED: u32 = 20_260_930;

/// Advancement steps the lowest eligible advancement of a role is searched on.
const ADVANCEMENT_STEPS: u32 = 20;

/// Hull seeds, one per cast member, chosen only for variety: any seed that
/// lays out its role at its lowest eligible advancement proves the same
/// rules.
const PLAYER_SEED: u32 = 1;
const RAIDER_SEED: u32 = 2;
const PICKET_SEED: u32 = 3;
const TRADER_SEED: u32 = 4;
const HULK_SEED: u32 = 5;
const WARDEN_SEED: u32 = 6;
const ESCORT_SEED: u32 = 7;
const MARAUDER_SEED: u32 = 8;

/// Scenario ids, read back by the script through `staged_hull`.
const PLAYER_ID: &str = "encounters_player";
const RAIDER_ID: &str = "encounters_raider";
const PICKET_ID: &str = "encounters_picket";
const WARDEN_ID: &str = "encounters_warden";
const TRADER_ID: &str = "encounters_trader";
const HULK_ID: &str = "encounters_hulk";
const ESCORT_ID: &str = "encounters_escort";
const MARAUDER_ID: &str = "encounters_marauder";

/// The raider's arrival grace: it spawns already able to see the player but
/// holds its passive routine for this long. Comfortably inside the 20 km
/// sensor default and the 400 u (4 km) engage range, and well past the guns'
/// ~180 u fire gate, so beat 1 can despawn it a few frames after it engages
/// with no chance it has closed to where it could land a hit.
const RAIDER_ENGAGE_DELAY_SECS: f32 = 8.0;
const RAIDER_DISTANCE_M: f32 = 3_500.0;

/// The picket's patrol: two close waypoints with a real first leg to fly, so
/// beat 2 observes an actual arrival rather than an instant hold at the spawn
/// point. Its leash is centred on the waypoints' centroid, (-800, 0, 200).
const PICKET_START: Vec3 = Vec3::new(-800.0, 0.0, -200.0);
const PICKET_WAYPOINT_0: Vec3 = Vec3::new(-800.0, 0.0, 0.0);
const PICKET_WAYPOINT_1: Vec3 = Vec3::new(-800.0, 0.0, 400.0);
const PICKET_STOP_SECS: f32 = 6.0;
const PICKET_LEASH_M: f32 = 1_500.0;

/// The warden holds station near the picket, inside sensor reach of every
/// hit beats 4 and 6 land on it.
const WARDEN_POSITION: Vec3 = Vec3::new(-1_600.0, 0.0, -800.0);

/// Where the trader stands.
const TRADER_POSITION: Vec3 = Vec3::new(0.0, 0.0, -100.0);

/// The hulk starts inside the picket's leash (1.17 km from its centre) and
/// 1.84 km from the player, inside the 4 km engage range an armed Enemy would
/// leave its routine for. Beat 7 moves it to `HULK_OUTSIDE_LEASH`, 2.3 km from
/// the leash centre and still 2.62 km from the player; beat 12 moves it back.
const HULK_POSITION: Vec3 = Vec3::new(-1_400.0, 0.0, 1_200.0);

/// What the hulk carries and holds at spawn, stated here rather than drawn by
/// `ship_stock` so beat 14 asserts exact counts. The player spawns with an
/// empty hold and no credits, so after the Takes it holds exactly this.
const HULK_STOCK: &[(&str, u32)] = &[("Rations", 5), ("SalvagedParts", 2)];
const HULK_CREDITS: u32 = 340;
#[cfg(feature = "debug")]
const HULK_OUTSIDE_LEASH: Vec3 = Vec3::new(-800.0, 0.0, 2_500.0);

/// The live-fire pair. The escort waits 30 km from the player and the
/// marauder 60 km out, both past the 20 km sensor default, so neither sees
/// anything before beat 9 moves the marauder to `MARAUDER_ENGAGE`: 400 m
/// from the escort, well inside the guns' 1.8 km fire gate, and 29.6 km from
/// the player. Both keep `FIGHT_STANDOFF_M` of daylight between their hulls
/// instead of the 1 km default, so the fight stays close enough for one
/// frame to resolve both hulls.
const ESCORT_POSITION: Vec3 = Vec3::new(0.0, 0.0, -30_000.0);
const MARAUDER_STAGED: Vec3 = Vec3::new(0.0, 0.0, -60_000.0);
#[cfg(feature = "debug")]
const MARAUDER_ENGAGE: Vec3 = Vec3::new(0.0, 0.0, -29_600.0);
const FIGHT_STANDOFF_M: f32 = 300.0;

/// Damage a staged hit spends: enough to register, far below any hull's pool.
#[cfg(feature = "debug")]
const STAGED_HIT: f32 = 10.0;

/// Every `.deadline()` in the script shares this ceiling: a real-seconds
/// backstop generous enough for a software-rendered frame to cost seconds and
/// an in-sim wait (the 8 s grace, the two 6 s patrol holds) to stretch well
/// past its own length on lavapipe. Never read as a measurement of how long a
/// beat SHOULD take.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 600.0;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new()
        .with_game_plugins(encounters_range)
        .build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.init_resource::<HitLedger>();
        app.add_observer(record_hit);
        app.add_plugins(encounters_script());
    }

    app.run()
}

fn encounters_range(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_encounters);
}

fn setup_encounters(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
) {
    commands.trigger(LoadScenario(encounters_scenario(&game_assets, &sections)));
}

/// Build one hull for `role` at the snapshot's lowest eligible advancement.
fn generated_hull(
    snapshot: &ShipPartSnapshot,
    civilization: CivilizationId,
    role: ShipRoleType,
    seed: u32,
) -> ShipDesign {
    let advancement = (0..=ADVANCEMENT_STEPS)
        .map(|step| step as f32 / ADVANCEMENT_STEPS as f32)
        .find(|advancement| snapshot.eligible_roles(*advancement).contains(&role))
        .unwrap_or_else(|| {
            panic!(
                "world_encounters: the snapshot serves no {} at any advancement",
                role.label()
            )
        });
    let request = ShipLayoutRequest {
        seed,
        civilization,
        role,
        advancement,
    };
    generate_ship(snapshot, request)
        .unwrap_or_else(|failure| panic!("world_encounters: {failure}"))
        .design
}

/// The range: a player and seven cast ships, each the world's own generated
/// hull for its role, spawned through the ordinary scenario path.
fn encounters_scenario(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let ids = ["base"];
    let packs = nova_authoring::lint_walk::repo_ship_part_packs(&ids);
    let snapshot = ShipPartSnapshot::build(&packs).unwrap_or_else(|faults| {
        panic!(
            "world_encounters: the snapshot of {ids:?} is refused:\n  {}",
            faults
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n  ")
        )
    });
    let civilization = CivilizationId {
        world_seed: WORLD_SEED,
        node: [0, 0, 0],
    };

    // Empty magazines for every hull outside the live-fire pair: the subject
    // here is the encounter rules, not gunnery, and a dry raider is one beat
    // 1 can despawn without ever risking a live shot at the player.
    let dry = |role: ShipRoleType, seed: u32| {
        let mut design = generated_hull(&snapshot, civilization, role, seed);
        kit::dry_magazines(&mut design, sections);
        design
    };
    let live = |role: ShipRoleType, seed: u32| generated_hull(&snapshot, civilization, role, seed);

    let ship = |id: &str, name: &str, design: ShipDesign, at: Vec3, spec: SpaceshipConfig| {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: name.to_string(),
                position: Meters3::new(at.x, at.y, at.z),
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                design: ShipDesignSource::Inline(design),
                ..spec
            }),
        })
    };
    let ai = |allegiance: Allegiance| SpaceshipConfig {
        controller: SpaceshipController::AI(AIControllerConfig::default()),
        allegiance: Some(allegiance),
        ..default()
    };
    let fighter = |allegiance: Allegiance| SpaceshipConfig {
        controller: SpaceshipController::AI(AIControllerConfig {
            standoff_clearance: Some(Meters(FIGHT_STANDOFF_M)),
            ..default()
        }),
        allegiance: Some(allegiance),
        ..default()
    };

    ScenarioConfig {
        description: "A player and seven generated ships proving the open world's encounter \
                       rules: arrival grace, patrol holds, calm and refused docking, \
                       retaliation, an allied ship in a live fight, unarmed Enemy restraint, \
                       and boarding and looting after the last thruster."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    ship(
                        PLAYER_ID,
                        "Player",
                        dry(ShipRoleType::Armored, PLAYER_SEED),
                        Vec3::ZERO,
                        SpaceshipConfig {
                            controller: SpaceshipController::Player(default()),
                            ..default()
                        },
                    ),
                    ship(
                        RAIDER_ID,
                        "Raider",
                        dry(ShipRoleType::Scavenger, RAIDER_SEED),
                        Vec3::new(0.0, 0.0, RAIDER_DISTANCE_M),
                        SpaceshipConfig {
                            controller: SpaceshipController::AI(AIControllerConfig {
                                engage_delay: Some(RAIDER_ENGAGE_DELAY_SECS),
                                ..default()
                            }),
                            ..default()
                        },
                    ),
                    ship(
                        PICKET_ID,
                        "Picket",
                        dry(ShipRoleType::Armored, PICKET_SEED),
                        PICKET_START,
                        SpaceshipConfig {
                            controller: SpaceshipController::AI(AIControllerConfig {
                                patrol: vec![
                                    Meters3::new(
                                        PICKET_WAYPOINT_0.x,
                                        PICKET_WAYPOINT_0.y,
                                        PICKET_WAYPOINT_0.z,
                                    ),
                                    Meters3::new(
                                        PICKET_WAYPOINT_1.x,
                                        PICKET_WAYPOINT_1.y,
                                        PICKET_WAYPOINT_1.z,
                                    ),
                                ],
                                patrol_stops: vec![PICKET_STOP_SECS, PICKET_STOP_SECS],
                                leash: Some(Meters(PICKET_LEASH_M)),
                                ..default()
                            }),
                            allegiance: Some(Allegiance::Neutral),
                            ..default()
                        },
                    ),
                    ship(
                        WARDEN_ID,
                        "Warden",
                        dry(ShipRoleType::Armored, WARDEN_SEED),
                        WARDEN_POSITION,
                        ai(Allegiance::Neutral),
                    ),
                    ship(
                        TRADER_ID,
                        "Trader",
                        dry(ShipRoleType::Civilian, TRADER_SEED),
                        TRADER_POSITION,
                        ai(Allegiance::Neutral),
                    ),
                    ship(
                        HULK_ID,
                        "Hulk",
                        dry(ShipRoleType::Civilian, HULK_SEED),
                        HULK_POSITION,
                        SpaceshipConfig {
                            inventory: ShipInventoryStock::new(
                                HULK_STOCK.iter().map(|&(id, count)| (id.into(), count)),
                            ),
                            credits: HULK_CREDITS,
                            ..ai(Allegiance::Enemy)
                        },
                    ),
                    ship(
                        ESCORT_ID,
                        "Escort",
                        live(ShipRoleType::Armored, ESCORT_SEED),
                        ESCORT_POSITION,
                        fighter(Allegiance::Player),
                    ),
                    ship(
                        MARAUDER_ID,
                        "Marauder",
                        live(ShipRoleType::Scavenger, MARAUDER_SEED),
                        MARAUDER_STAGED,
                        fighter(Allegiance::Enemy),
                    ),
                ],
                ThreePointRig::around("world encounters", Meters3::ZERO, 40.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "world_encounters_range".to_string(),
            "World Encounters Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

#[cfg(feature = "debug")]
fn encounters_script() -> Script {
    let script = Script::new()
        .step("load the cast")
        .enter(GameStates::Loading)
        .until(every_hull_weighed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Beat 1: grace.
        .step("beat 1: the raider holds its grace")
        .until(elapsed(RAIDER_ENGAGE_DELAY_SECS * 0.5))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 1: mid-grace, the raider does not engage")
        .on_enter(assert_raider_passive_mid_grace)
        .add()
        .step("beat 1: the raider engages once its grace ends")
        .until(engages_id(RAIDER_ID))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 1: record")
        .on_enter(record_grace_beat)
        .until(shot_written(grace_shot()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Keep the player safe: despawn the raider once its shot is on disk,
        // a few frames after it engages and long before it could close to gun
        // range. Despawning it in the record hook would empty the frame.
        .step("beat 1: clear the raider")
        .on_enter(|world: &mut World| {
            let raider = hull(world, RAIDER_ID);
            world.despawn(raider);
        })
        .add()
        // Beat 2: patrol hold.
        .step("beat 2: the picket arrives and holds")
        .until(picket_holds_at(0))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 2: the picket advances off the hold")
        .until(picket_reaches_index(1))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 2: record")
        .on_enter(record_patrol_beat)
        .until(shot_written(patrol_shot()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Beat 3: calm dock.
        .step("beat 3: the player docks with the trader")
        .on_enter(request_player_trader_dock)
        .until(both_docked(PLAYER_ID, TRADER_ID))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 3: record, then release")
        .on_enter(record_calm_dock_beat)
        .until(shot_written(calm_dock_shot()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 3: the dock releases")
        .until(undocked(PLAYER_ID))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Beat 4: retaliation.
        .step("beat 4: the player hits the picket")
        .on_enter(|world: &mut World| hit_by(world, PICKET_ID, PLAYER_ID))
        .until(answers(PICKET_ID, PLAYER_ID))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 4: hostile now, a dock request is refused")
        .on_enter(assert_retaliation_hostile_then_request_dock)
        .until(frames(10))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 4: record")
        .on_enter(record_retaliation_beat)
        .until(shot_written(retaliation_shot()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Beat 5: calm bystander. Waits a few frames past the hit so the
        // warden's own acquisition and behavior systems have run on it.
        .step("beat 5: the warden watches the picket answer the player")
        .until(frames(30))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 5: record")
        .on_enter(record_calm_bystander_beat)
        .until(shot_written(calm_bystander_shot()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Beat 6: newest shooter.
        .step("beat 6: the hulk hits the picket")
        .on_enter(|world: &mut World| hit_by(world, PICKET_ID, HULK_ID))
        .until(answers(PICKET_ID, HULK_ID))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 6: record")
        .on_enter(record_newest_shooter_beat)
        .until(shot_written(newest_shooter_shot()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Beat 7: leash.
        .step("beat 7: the hulk leaves the picket's territory")
        .on_enter(|world: &mut World| {
            let hulk = hull(world, HULK_ID);
            place(world, hulk, HULK_OUTSIDE_LEASH);
        })
        .until(released_and_calm(PICKET_ID))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 7: record")
        .on_enter(record_leash_beat)
        .until(shot_written(leash_shot()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Beat 8: forced undock.
        .step("beat 8: the player docks with the trader again")
        .on_enter(request_player_trader_dock)
        .until(both_docked(PLAYER_ID, TRADER_ID))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 8: the player hits the docked trader")
        .on_enter(|world: &mut World| hit_by(world, TRADER_ID, PLAYER_ID))
        .until(undocked(PLAYER_ID))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 8: record")
        .on_enter(record_forced_undock_beat)
        .until(shot_written(forced_undock_shot()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Beat 9: the ally engages the Enemy.
        .step("beat 9: the marauder moves in on the escort")
        .on_enter(|world: &mut World| {
            let marauder = hull(world, MARAUDER_ID);
            place(world, marauder, MARAUDER_ENGAGE);
        })
        .until(mutual_engagement())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Beat 10: the ally stays allied. The hit lands as soon as the pair
        // engages, before the fight can end: the escort neutralized this
        // marauder within a second of the first exchange of hits on lavapipe.
        // Both beats record only once fire has flowed both ways after it.
        .step("beat 10: the player hits the escort mid-fight")
        .on_enter(|world: &mut World| {
            world.resource_mut::<HitLedger>().mark();
            hit_by(world, ESCORT_ID, PLAYER_ID);
        })
        .until(live_fire_exchanged())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 9: record")
        .on_enter(record_ally_engages_beat)
        .until(shot_written(ally_engages_shot()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 10: record")
        .on_enter(record_ally_stays_allied_beat)
        .until(shot_written(ally_stays_allied_shot()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // The fight is proved; take the only live Enemy gun off the board.
        .step("beat 10: clear the marauder")
        .on_enter(|world: &mut World| {
            let marauder = hull(world, MARAUDER_ID);
            world.despawn(marauder);
        })
        .add()
        // Beat 11: unarmed Enemy. The hulk has stood 2.62 km from the player
        // since beat 7; the wait gives its systems frames to act on that.
        .step("beat 11: the hulk watches the player")
        .until(frames(60))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 11: record")
        .on_enter(record_unarmed_enemy_beat)
        .until(shot_written(unarmed_enemy_shot()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 11: a dock request is refused")
        .on_enter(|world: &mut World| {
            let hulk = hull(world, HULK_ID);
            assert_aligned_dock_refused(world, hulk, "beat 11");
        })
        .add()
        // Beat 12: last thruster.
        .step("beat 12: the hulk returns and hits the picket")
        .on_enter(|world: &mut World| {
            let hulk = hull(world, HULK_ID);
            place(world, hulk, HULK_POSITION);
            hit_by(world, PICKET_ID, HULK_ID);
        })
        .until(answers(PICKET_ID, HULK_ID))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 12: all but one hulk thruster is destroyed")
        .on_enter(destroy_all_but_one_hulk_thruster)
        .until(hulk_working_thrusters_are(1))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 12: one working thruster keeps the hulk in the world")
        .until(frames(10))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 12: the last thruster is destroyed")
        .on_enter(assert_hulk_not_neutralized_then_destroy_last_thruster)
        .until(hulk_neutralized_and_picket_released())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 12: record")
        .on_enter(record_last_thruster_beat)
        .until(shot_written(last_thruster_shot()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Beat 13: boarding.
        .step("beat 13: the player docks with the neutralized hulk")
        .on_enter(request_player_hulk_dock)
        .until(both_docked(PLAYER_ID, HULK_ID))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 13: record")
        .on_enter(record_boarding_beat)
        .until(shot_written(boarding_shot()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Beat 14: loot.
        .step("beat 14: the hulk holds its seeded stock and credits")
        .on_enter(assert_loot_seeded)
        .add()
        .step("beat 14: press the interface key")
        .on_enter(press_action("interface_toggle"))
        .until(frames(1))
        .add()
        .step("beat 14: let the key up and wait for the interface")
        .on_enter(release_action("interface_toggle"))
        .until(state_is(PauseStates::Interface))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .click_named(
            "beat 14: open the Inventory pane",
            "InterfaceTabInventory",
            ui_node_present(partner_row(HULK_STOCK[0].0)),
            STEP_DEADLINE_SECS,
        );
    let script = HULK_STOCK.iter().fold(script, |script, &(item, count)| {
        take_all(script, item, count)
    });
    script
        .click_named(
            "beat 14: take the credits",
            "InventoryTakeCredits",
            hulk_credits_moved(),
            STEP_DEADLINE_SECS,
        )
        .step("beat 14: record")
        .on_enter(record_loot_beat)
        .until(shot_written(loot_shot()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 14: press the interface key to close it")
        .on_enter(press_action("interface_toggle"))
        .until(frames(1))
        .add()
        .step("beat 14: let the key up and wait for flight")
        .on_enter(release_action("interface_toggle"))
        .until(state_is(PauseStates::Unpaused))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("beat 14: the dock releases")
        .on_enter(|world: &mut World| {
            let player = hull(world, PLAYER_ID);
            world.trigger(DockingReleaseRequest { entity: player });
        })
        .until(undocked(PLAYER_ID))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("no fired round hit the player")
        .on_enter(assert_no_round_hit_the_player)
        .add()
}

// --- shared lookups -------------------------------------------------------

/// The hull with this scenario id, off a borrowed world.
#[cfg(feature = "debug")]
fn staged_hull(world: &World, id: &str) -> Option<Entity> {
    let mut hulls = world.try_query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()?;
    hulls
        .iter(world)
        .find(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
}

/// The hull with this scenario id; panics naming it when the cast is not
/// staged yet, since every caller below runs only after the load step.
#[cfg(feature = "debug")]
fn hull(world: &World, id: &str) -> Entity {
    staged_hull(world, id).unwrap_or_else(|| panic!("world_encounters: '{id}' is not staged"))
}

/// Whether an `AIBehaviorState` runs the engage-style chase/aim/fire
/// pipeline. Reimplemented here rather than called: `AIBehaviorState::engages`
/// is `pub(crate)` to `nova_ship`, but every variant it matches on is public.
#[cfg(feature = "debug")]
fn engages(state: AIBehaviorState) -> bool {
    matches!(
        state,
        AIBehaviorState::Engage | AIBehaviorState::Evade | AIBehaviorState::Retreat
    )
}

#[cfg(feature = "debug")]
fn behavior(world: &World, ship: Entity) -> AIBehaviorState {
    world
        .get::<AIBehaviorState>(ship)
        .copied()
        .unwrap_or_default()
}

#[cfg(feature = "debug")]
fn ai_target(world: &World, ship: Entity) -> Option<Entity> {
    world.get::<AITarget>(ship).and_then(|target| target.0)
}

#[cfg(feature = "debug")]
fn answering(world: &World, ship: Entity) -> Option<Entity> {
    world
        .get::<RetaliationTarget>(ship)
        .and_then(|target| target.0)
}

/// `ship_relation` between two staged hulls, read the way the AI and HUD read
/// it.
#[cfg(feature = "debug")]
fn relation_between(world: &World, a: Entity, b: Entity) -> Relation {
    let party = |entity| RelationParty {
        entity,
        allegiance: world.get::<Allegiance>(entity),
        retaliation: world.get::<RetaliationTarget>(entity),
    };
    ship_relation(party(a), party(b))
}

/// The painted colour of `ship`'s HUD allegiance triangle, if it has one.
#[cfg(feature = "debug")]
fn marker_color(world: &mut World, ship: Entity) -> Option<Color> {
    world
        .query_filtered::<(&AllegianceMarkerTargetEntity, &BorderColor), With<AllegianceMarkerTriangleMarker>>()
        .iter(world)
        .find(|(target, _)| target.0 == ship)
        .map(|(_, border)| border.top)
}

/// Whether `target` admits a new dock from `ship`, through the same
/// `DockAdmission` the `DOCK` chip and the request read.
#[cfg(feature = "debug")]
fn dock_admits(world: &mut World, ship: Entity, target: Entity) -> bool {
    world
        .run_system_once(move |admission: DockAdmission| admission.admits(ship, target))
        .expect("world_encounters: DockAdmission runs as an ordinary system")
}

/// Set `target` port to port with the player, assert the port geometry alone
/// would accept the pair, send the player's real dock request, and assert it
/// made no connection: a refusal here can only come from the admission check,
/// not from distance. The target goes back to its pose before any physics
/// step runs, because a fighting AI ship left against the player's hull rams
/// it.
#[cfg(feature = "debug")]
fn assert_aligned_dock_refused(world: &mut World, target: Entity, beat: &str) {
    let player = hull(world, PLAYER_ID);
    let saved = {
        let entity = world.entity(target);
        (
            *entity.get::<Position>().unwrap(),
            *entity.get::<Rotation>().unwrap(),
            *entity.get::<Transform>().unwrap(),
            *entity.get::<LinearVelocity>().unwrap(),
            *entity.get::<AngularVelocity>().unwrap(),
        )
    };
    align_for_docking(world, player, target);
    let ports_allow = world
        .run_system_once(move |ports: DockingPorts| ports.best_candidate(player, target).is_some())
        .expect("world_encounters: DockingPorts runs as an ordinary system");
    assert!(
        ports_allow,
        "world_encounters: {beat}: the ports refuse the aligned pair, so a refusal would prove nothing"
    );
    world.trigger(DockingConnectionRequest {
        entity: player,
        target,
    });
    world.flush();
    let connected = world
        .query::<&DockingConnection>()
        .iter(world)
        .any(|connection| {
            [connection.first_ship, connection.second_ship].contains(&player)
                && [connection.first_ship, connection.second_ship].contains(&target)
        });
    assert!(
        !connected,
        "world_encounters: {beat}: the player's request made a dock the ports allowed"
    );

    let (position, rotation, transform, linear, angular) = saved;
    let mut entity = world.entity_mut(target);
    *entity.get_mut::<Position>().unwrap() = position;
    *entity.get_mut::<Rotation>().unwrap() = rotation;
    *entity.get_mut::<Transform>().unwrap() = transform;
    *entity.get_mut::<LinearVelocity>().unwrap() = linear;
    *entity.get_mut::<AngularVelocity>().unwrap() = angular;
    info!("encounters: {beat}: set port to port, the ports allowed the pair and the player's request was refused");
}

/// Every thruster section mounted directly on `root`.
#[cfg(feature = "debug")]
fn thruster_sections_of(world: &mut World, root: Entity) -> Vec<Entity> {
    world
        .query_filtered::<(Entity, &ChildOf), With<ThrusterSectionMarker>>()
        .iter(world)
        .filter(|(_, parent)| parent.parent() == root)
        .map(|(entity, _)| entity)
        .collect()
}

/// The thrusters on `root` the neutralization rule counts as working: still
/// mounted and not inactive.
#[cfg(feature = "debug")]
fn working_thrusters_of(world: &World, root: Entity) -> Vec<Entity> {
    let Some(mut thrusters) = world.try_query_filtered::<(Entity, &ChildOf), (
        With<ThrusterSectionMarker>,
        Without<SectionInactiveMarker>,
    )>() else {
        return Vec::new();
    };
    thrusters
        .iter(world)
        .filter(|(_, parent)| parent.parent() == root)
        .map(|(entity, _)| entity)
        .collect()
}

/// Spend a section's whole health pool, the way a hit that depletes it would.
#[cfg(feature = "debug")]
fn destroy_section(world: &mut World, section: Entity, source: Entity) {
    let amount = world
        .get::<Health>(section)
        .expect("world_encounters: a thruster section has a Health pool")
        .current;
    world.trigger(HealthApplyDamage {
        entity: section,
        source: Some(source),
        amount,
    });
}

/// A staged hit on the hull `target_id`, attributed to the hull `source_id`.
#[cfg(feature = "debug")]
fn hit_by(world: &mut World, target_id: &str, source_id: &str) {
    let target = hull(world, target_id);
    let source = hull(world, source_id);
    world.trigger(HealthApplyDamage {
        entity: target,
        source: Some(source),
        amount: STAGED_HIT,
    });
}

/// Set `ship` down at rest at `at` (meters). Writes the `Transform` as well
/// as the `Position`: the AI's leash and retaliation checks read the
/// `Transform`, and a beat that moves a ship and hits with it in one hook
/// must not have those checks see the old place before physics syncs it.
#[cfg(feature = "debug")]
fn place(world: &mut World, ship: Entity, at: Vec3) {
    let engine = Meters3::new(at.x, at.y, at.z).to_engine();
    let mut entity = world.entity_mut(ship);
    *entity.get_mut::<Position>().unwrap() = Position(engine);
    entity.get_mut::<Transform>().unwrap().translation = engine;
    *entity.get_mut::<LinearVelocity>().unwrap() = LinearVelocity(Vec3::ZERO);
    *entity.get_mut::<AngularVelocity>().unwrap() = AngularVelocity(Vec3::ZERO);
}

/// Move `mover` so its best free docking port sits `gap`-close and exactly
/// opposed to `anchor`'s, by applying one rigid delta transform (rotation
/// then translation) to `mover`'s whole body - the delta that carries its
/// CURRENT port pose onto the target pose carries the port with the root,
/// whatever the port's own mount offset is. Reads the pair the production
/// `DockingPorts::nearest_pair` would pick, so the request this sets up for
/// is the same pair the engine resolves.
#[cfg(feature = "debug")]
fn align_for_docking(world: &mut World, anchor: Entity, mover: Entity) {
    let pair = world
        .run_system_once(move |ports: DockingPorts| ports.nearest_pair(anchor, mover))
        .expect("world_encounters: DockingPorts runs as an ordinary system")
        .unwrap_or_else(|| {
            panic!("world_encounters: {anchor:?} and {mover:?} share no free docking port pair")
        });
    let (anchor_pose, mover_pose) = (pair.first, pair.second);
    // Half the stricter envelope's capture distance: comfortably inside the
    // gate regardless of which docking part the generator picked.
    let gap = pair.envelope.capture_distance * 0.5;

    let target_axis = -anchor_pose.axis;
    let target_face = anchor_pose.face + anchor_pose.axis * gap;
    let delta_rotation = Quat::from_rotation_arc(mover_pose.axis, target_axis);
    let delta_translation = target_face - delta_rotation * mover_pose.face;

    let (position, rotation) = {
        let position = world
            .get::<Position>(mover)
            .copied()
            .expect("a ship root has a Position");
        let rotation = world
            .get::<Rotation>(mover)
            .copied()
            .expect("a ship root has a Rotation");
        (position.0, rotation.0)
    };
    let mut entity = world.entity_mut(mover);
    *entity.get_mut::<Position>().unwrap() =
        Position(delta_rotation * position + delta_translation);
    *entity.get_mut::<Rotation>().unwrap() = Rotation(delta_rotation * rotation);
    *entity.get_mut::<LinearVelocity>().unwrap() = LinearVelocity(Vec3::ZERO);
    *entity.get_mut::<AngularVelocity>().unwrap() = AngularVelocity(Vec3::ZERO);

    // Zero the anchor's own residual momentum too: an earlier beat's docking
    // bump can leave it drifting, and the envelope's relative-speed check
    // would then reject every port pair regardless of how well-aligned they
    // are.
    let mut anchor_entity = world.entity_mut(anchor);
    *anchor_entity.get_mut::<LinearVelocity>().unwrap() = LinearVelocity(Vec3::ZERO);
    *anchor_entity.get_mut::<AngularVelocity>().unwrap() = AngularVelocity(Vec3::ZERO);
}

/// Shot file names, one per beat.
#[cfg(feature = "debug")]
fn grace_shot() -> String {
    "world-encounters-beat1-grace.png".to_string()
}
#[cfg(feature = "debug")]
fn patrol_shot() -> String {
    "world-encounters-beat2-patrol.png".to_string()
}
#[cfg(feature = "debug")]
fn calm_dock_shot() -> String {
    "world-encounters-beat3-calm-dock.png".to_string()
}
#[cfg(feature = "debug")]
fn retaliation_shot() -> String {
    "world-encounters-beat4-retaliation.png".to_string()
}
#[cfg(feature = "debug")]
fn calm_bystander_shot() -> String {
    "world-encounters-beat5-calm-bystander.png".to_string()
}
#[cfg(feature = "debug")]
fn newest_shooter_shot() -> String {
    "world-encounters-beat6-newest-shooter.png".to_string()
}
#[cfg(feature = "debug")]
fn leash_shot() -> String {
    "world-encounters-beat7-leash.png".to_string()
}
#[cfg(feature = "debug")]
fn forced_undock_shot() -> String {
    "world-encounters-beat8-forced-undock.png".to_string()
}
#[cfg(feature = "debug")]
fn ally_engages_shot() -> String {
    "world-encounters-beat9-ally-engages.png".to_string()
}
#[cfg(feature = "debug")]
fn ally_stays_allied_shot() -> String {
    "world-encounters-beat10-ally-stays-allied.png".to_string()
}
#[cfg(feature = "debug")]
fn unarmed_enemy_shot() -> String {
    "world-encounters-beat11-unarmed-enemy.png".to_string()
}
#[cfg(feature = "debug")]
fn last_thruster_shot() -> String {
    "world-encounters-beat12-last-thruster.png".to_string()
}
#[cfg(feature = "debug")]
fn boarding_shot() -> String {
    "world-encounters-beat13-boarding.png".to_string()
}
#[cfg(feature = "debug")]
fn loot_shot() -> String {
    "world-encounters-beat14-loot.png".to_string()
}

/// `subject`'s live centre of mass (engine units) and containment radius.
#[cfg(feature = "debug")]
fn live_center(world: &World, subject: Entity) -> (Vec3, f32) {
    let entity = world.entity(subject);
    let position = entity
        .get::<Position>()
        .expect("a ship root has a Position")
        .0;
    let rotation = entity
        .get::<Rotation>()
        .expect("a ship root has a Rotation")
        .0;
    let com = entity
        .get::<ComputedCenterOfMass>()
        .expect("every cast hull is weighed before the first beat")
        .0;
    let radius = entity
        .get::<HullEnvelopeRadius>()
        .expect("a ship root derives its HullEnvelopeRadius")
        .0;
    (position + rotation * com, radius)
}

/// Point the dev camera at `subject`'s live centre of mass from three of its
/// containment radii out, and shoot `name`. Aimed at the live hull, not an
/// authored point: by the time a beat records, the trader has been moved to
/// its dock and the picket has flown on, and a fixed eye near the player
/// lands inside the player's hull and frames only the sky.
#[cfg(feature = "debug")]
fn capture(world: &mut World, name: &str, subject: Entity) {
    let (center, radius) = live_center(world, subject);
    let eye = center + Vec3::new(1.0, 0.6, 1.0).normalize() * radius * 3.0;
    pose_camera(
        world,
        Meters3::from_engine(eye),
        Meters3::from_engine(center),
    );
    shoot(world, name);
}

/// Shoot `name` from the side of the line between `a` and `b`: the eye
/// stands off the pair's midpoint, square to that line and a little above it,
/// far enough back that both hulls and whatever passes between them are in
/// frame. An eye behind either hull fills the frame with it and leaves a
/// distant opponent a marker-sized speck.
#[cfg(feature = "debug")]
fn capture_pair(world: &mut World, name: &str, a: Entity, b: Entity) {
    let (a_center, _) = live_center(world, a);
    let (b_center, _) = live_center(world, b);
    let line = b_center - a_center;
    let side = line.cross(Vec3::Y).normalize_or(Vec3::X);
    let focus = a_center.lerp(b_center, 0.5);
    let eye = focus + (side * 1.2 + Vec3::Y * 0.15) * line.length();
    pose_camera(
        world,
        Meters3::from_engine(eye),
        Meters3::from_engine(focus),
    );
    shoot(world, name);
}

// --- hit ledger -----------------------------------------------------------

/// One damage event that reached a ship root.
#[cfg(feature = "debug")]
#[derive(Debug, Clone, Copy)]
struct Hit {
    /// The ship root behind it: a projectile's owner, else the ship root the
    /// source collider belongs to, else the source itself.
    shooter: Entity,
    /// The ship root it reached.
    struck: Entity,
    /// Whether a fired projectile dealt it, rather than a staged source or a
    /// hull contact.
    projectile: bool,
}

/// Every hit the run has landed on a ship root, in order.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct HitLedger {
    hits: Vec<Hit>,
    /// Where the hits after the latest `mark` start: beat 10 marks before its
    /// hit, so the fire it waits for is fire that followed the hit.
    mark: usize,
}

#[cfg(feature = "debug")]
impl HitLedger {
    fn mark(&mut self) {
        self.mark = self.hits.len();
    }

    /// Whether such a hit landed since the latest `mark`.
    fn landed_since_mark(&self, shooter: Entity, struck: Entity, projectile: bool) -> bool {
        self.hits[self.mark..].iter().any(|hit| {
            hit.shooter == shooter && hit.struck == struck && hit.projectile == projectile
        })
    }
}

/// Write each damage event to the [`HitLedger`] once it propagates to a ship
/// root, the same point `on_damage_track_threat` reads it at.
#[cfg(feature = "debug")]
fn record_hit(
    damage: On<HealthApplyDamage>,
    roots: Query<(), With<SpaceshipRootMarker>>,
    owners: Query<&ProjectileOwner>,
    parents: Query<&ChildOf>,
    mut ledger: ResMut<HitLedger>,
) {
    if damage.amount <= 0.0 || !roots.contains(damage.entity) {
        return;
    }
    let Some(source) = damage.source else {
        return;
    };
    let (shooter, projectile) = match owners.get(source) {
        Ok(owner) => (owner.0, true),
        Err(_) => (
            std::iter::once(source)
                .chain(parents.iter_ancestors(source))
                .find(|entity| roots.contains(*entity))
                .unwrap_or(source),
            false,
        ),
    };
    ledger.hits.push(Hit {
        shooter,
        struck: damage.entity,
        projectile,
    });
}

#[cfg(feature = "debug")]
fn assert_no_round_hit_the_player(world: &mut World) {
    let player = hull(world, PLAYER_ID);
    let ledger = world.resource::<HitLedger>();
    let (rounds, contacts): (Vec<Hit>, Vec<Hit>) = ledger
        .hits
        .iter()
        .filter(|hit| hit.struck == player)
        .partition(|hit| hit.projectile);
    assert!(
        rounds.is_empty(),
        "world_encounters: fired rounds hit the player: {rounds:?}"
    );
    let contact_shooters: Vec<Entity> = contacts.iter().map(|hit| hit.shooter).collect();
    info!(
        "encounters: PASS player safety: none of the run's {} ship hits was a fired round on the player; contact hits on the player: {} from {contact_shooters:?}",
        ledger.hits.len(),
        contacts.len()
    );
}

// --- load -------------------------------------------------------------

/// Every cast member is present AND has been weighed: a root avian has not
/// measured yet has no centre of mass, so every pose this script reads would
/// be taken before the physics body exists.
#[cfg(feature = "debug")]
fn every_hull_weighed() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        [
            PLAYER_ID,
            RAIDER_ID,
            PICKET_ID,
            WARDEN_ID,
            TRADER_ID,
            HULK_ID,
            ESCORT_ID,
            MARAUDER_ID,
        ]
        .iter()
        .all(|id| {
            staged_hull(world, id)
                .is_some_and(|root| world.get::<ComputedCenterOfMass>(root).is_some())
        })
    })
}

// --- shared predicates ----------------------------------------------------

#[cfg(feature = "debug")]
fn engages_id(id: &'static str) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(move |world: &World| {
        staged_hull(world, id).is_some_and(|ship| engages(behavior(world, ship)))
    })
}

/// The hull `ship_id` answers the hull `target_id`.
#[cfg(feature = "debug")]
fn answers(
    ship_id: &'static str,
    target_id: &'static str,
) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(move |world: &World| {
        let Some(target) = staged_hull(world, target_id) else {
            return false;
        };
        staged_hull(world, ship_id).is_some_and(|ship| answering(world, ship) == Some(target))
    })
}

/// The hull `ship_id` answers nobody, targets nothing and flies a passive
/// routine.
#[cfg(feature = "debug")]
fn released_and_calm(ship_id: &'static str) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(move |world: &World| {
        staged_hull(world, ship_id).is_some_and(|ship| {
            answering(world, ship).is_none()
                && ai_target(world, ship).is_none()
                && !engages(behavior(world, ship))
        })
    })
}

#[cfg(feature = "debug")]
fn both_docked(
    first_id: &'static str,
    second_id: &'static str,
) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(move |world: &World| {
        let Some(first) = staged_hull(world, first_id) else {
            return false;
        };
        let Some(second) = staged_hull(world, second_id) else {
            return false;
        };
        world.get::<DockedShip>(first).is_some() && world.get::<DockedShip>(second).is_some()
    })
}

#[cfg(feature = "debug")]
fn undocked(id: &'static str) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(move |world: &World| {
        staged_hull(world, id).is_some_and(|root| world.get::<DockedShip>(root).is_none())
    })
}

/// Panic unless the warden answers nobody, targets nothing, flies a passive
/// routine and still reads Neutral to the player.
#[cfg(feature = "debug")]
fn assert_warden_calm(world: &World, beat: &str) {
    let player = hull(world, PLAYER_ID);
    let warden = hull(world, WARDEN_ID);
    let state = behavior(world, warden);
    assert_eq!(
        answering(world, warden),
        None,
        "world_encounters: {beat}: the warden answers a ship nobody fired at it"
    );
    assert_eq!(
        ai_target(world, warden),
        None,
        "world_encounters: {beat}: the warden picked a target"
    );
    assert!(
        !engages(state),
        "world_encounters: {beat}: the warden is engaging ({state:?})"
    );
    let relation = relation_between(world, player, warden);
    assert_eq!(
        relation,
        Relation::Neutral,
        "world_encounters: {beat}: ship_relation(player, warden) is {relation:?}"
    );
}

// --- beat 1: grace ------------------------------------------------------

#[cfg(feature = "debug")]
fn assert_raider_passive_mid_grace(world: &mut World) {
    let raider = hull(world, RAIDER_ID);
    let has_grace = world.get::<AIEngageGrace>(raider).is_some();
    let state = behavior(world, raider);
    assert!(
        has_grace,
        "world_encounters: beat 1: the raider lost its AIEngageGrace before its grace ran out"
    );
    assert!(
        !engages(state),
        "world_encounters: beat 1: the raider is already engaging ({state:?}) mid-grace"
    );
    info!(
        "encounters: mid-grace the raider holds AIEngageGrace and stays in {state:?} (not engaging)"
    );
}

#[cfg(feature = "debug")]
fn record_grace_beat(world: &mut World) {
    let raider = hull(world, RAIDER_ID);
    let state = behavior(world, raider);
    info!("encounters: PASS grace: the raider engages ({state:?}) once its {RAIDER_ENGAGE_DELAY_SECS}s grace ends");
    probe_marker(
        world,
        "outcome: an arrival-grace raider holds passive then engages",
        serde_json::json!({
            "engage_delay_s": RAIDER_ENGAGE_DELAY_SECS,
            "state": format!("{state:?}"),
            "engaged": engages(state),
        }),
    );
    capture(world, &grace_shot(), raider);
}

// --- beat 2: patrol hold -------------------------------------------------

#[cfg(feature = "debug")]
fn picket_holds_at(index: usize) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(move |world: &World| {
        staged_hull(world, PICKET_ID).is_some_and(|picket| {
            world
                .get::<AIPatrolRoute>(picket)
                .is_some_and(|route| route.current == index && route.hold.is_some())
        })
    })
}

#[cfg(feature = "debug")]
fn picket_reaches_index(index: usize) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(move |world: &World| {
        staged_hull(world, PICKET_ID).is_some_and(|picket| {
            world
                .get::<AIPatrolRoute>(picket)
                .is_some_and(|route| route.current == index)
        })
    })
}

#[cfg(feature = "debug")]
fn record_patrol_beat(world: &mut World) {
    let picket = hull(world, PICKET_ID);
    let current = world
        .get::<AIPatrolRoute>(picket)
        .map(|route| route.current)
        .unwrap_or_default();
    info!("encounters: PASS patrol hold: the picket held waypoint 0 on its authored {PICKET_STOP_SECS} s stop, then advanced to waypoint {current}");
    probe_marker(
        world,
        "outcome: a patrolling ship holds its waypoint then advances",
        serde_json::json!({ "stop_s": PICKET_STOP_SECS, "advanced_to": current }),
    );
    capture(world, &patrol_shot(), picket);
}

// --- beat 3: calm dock ----------------------------------------------------

#[cfg(feature = "debug")]
fn request_player_trader_dock(world: &mut World) {
    let player = hull(world, PLAYER_ID);
    let trader = hull(world, TRADER_ID);
    align_for_docking(world, player, trader);
    world.trigger(DockingConnectionRequest {
        entity: player,
        target: trader,
    });
}

#[cfg(feature = "debug")]
fn record_calm_dock_beat(world: &mut World) {
    let player = hull(world, PLAYER_ID);
    let trader = hull(world, TRADER_ID);
    info!("encounters: PASS calm dock: the player's admitted request to the trader docked");
    probe_marker(
        world,
        "outcome: a calm admitted dock request docks",
        serde_json::json!({ "docked": true }),
    );
    capture(world, &calm_dock_shot(), trader);
    world.trigger(DockingReleaseRequest { entity: player });
}

// --- beat 4: retaliation --------------------------------------------------

#[cfg(feature = "debug")]
fn assert_retaliation_hostile_then_request_dock(world: &mut World) {
    let player = hull(world, PLAYER_ID);
    let picket = hull(world, PICKET_ID);
    let relation = relation_between(world, player, picket);
    assert_eq!(
        relation,
        Relation::Hostile,
        "world_encounters: beat 4: ship_relation(player, picket) is {relation:?}, not Hostile"
    );
    assert!(
        !dock_admits(world, player, picket),
        "world_encounters: beat 4: DockAdmission admits the player to a picket answering it"
    );
    info!("encounters: ship_relation(player, picket) reads {relation:?} after the hit; DockAdmission refuses the player");
    assert_aligned_dock_refused(world, picket, "beat 4");
}

#[cfg(feature = "debug")]
fn record_retaliation_beat(world: &mut World) {
    let player = hull(world, PLAYER_ID);
    let picket = hull(world, PICKET_ID);
    assert!(
        world.get::<DockedShip>(player).is_none(),
        "world_encounters: beat 4: the player docked with a picket that is retaliating against it"
    );
    assert_eq!(
        marker_color(world, picket),
        Some(allegiance_color(Some(&Allegiance::Enemy))),
        "world_encounters: beat 4: the picket's HUD triangle is not threat red while it answers the player"
    );
    assert_eq!(
        world.get::<Allegiance>(picket),
        Some(&Allegiance::Neutral),
        "world_encounters: beat 4: answering the player changed the picket's side"
    );
    info!("encounters: PASS retaliation: the picket answers the player, reads Hostile, shows a threat-red marker, stays Neutral, and refuses a dock");
    probe_marker(
        world,
        "outcome: an armed Neutral ship retaliates and refuses the shooter a dock",
        serde_json::json!({ "retaliation_target": format!("{player:?}"), "relation": "Hostile", "marker": "threat", "dock_refused": true }),
    );
    capture(world, &retaliation_shot(), picket);
}

// --- beat 5: calm bystander ------------------------------------------------

#[cfg(feature = "debug")]
fn record_calm_bystander_beat(world: &mut World) {
    let player = hull(world, PLAYER_ID);
    let picket = hull(world, PICKET_ID);
    let warden = hull(world, WARDEN_ID);
    assert_eq!(
        answering(world, picket),
        Some(player),
        "world_encounters: beat 5: the picket stopped answering the player"
    );
    assert_warden_calm(world, "beat 5");
    let state = behavior(world, warden);
    info!("encounters: PASS calm bystander: the warden answers nobody and stays {state:?} while the picket answers the player");
    probe_marker(
        world,
        "outcome: a second Neutral ship stays calm while the first retaliates",
        serde_json::json!({ "warden_state": format!("{state:?}"), "warden_answering": null }),
    );
    capture(world, &calm_bystander_shot(), warden);
}

// --- beat 6: newest shooter -----------------------------------------------

#[cfg(feature = "debug")]
fn record_newest_shooter_beat(world: &mut World) {
    let picket = hull(world, PICKET_ID);
    let hulk = hull(world, HULK_ID);
    assert_warden_calm(world, "beat 6");
    info!("encounters: PASS newest shooter: the hulk's hit replaced the player as the picket's target");
    probe_marker(
        world,
        "outcome: the most recent shooter replaces a Neutral ship's target",
        serde_json::json!({ "retaliation_target": format!("{hulk:?}") }),
    );
    capture(world, &newest_shooter_shot(), picket);
}

// --- beat 7: leash --------------------------------------------------------

#[cfg(feature = "debug")]
fn record_leash_beat(world: &mut World) {
    let player = hull(world, PLAYER_ID);
    let picket = hull(world, PICKET_ID);
    let hulk = hull(world, HULK_ID);
    let leash = world
        .get::<AILeash>(picket)
        .cloned()
        .expect("world_encounters: the picket carries its authored AILeash");
    let beyond_m = Meters3::from_engine(live_center(world, hulk).0 - leash.center)
        .length()
        .get();
    let radius_m = Meters::from_engine(leash.radius).get();
    assert!(
        beyond_m > radius_m,
        "world_encounters: beat 7: the hulk is {beyond_m:.0} m from the leash centre, inside its {radius_m:.0} m"
    );
    let state = behavior(world, picket);
    let relation = relation_between(world, player, picket);
    assert_eq!(
        relation,
        Relation::Neutral,
        "world_encounters: beat 7: ship_relation(player, picket) is {relation:?} after the release"
    );
    info!("encounters: PASS leash: the hulk stands {beyond_m:.0} m from the picket's leash centre (radius {radius_m:.0} m); the picket answers nobody and returned to {state:?}");
    probe_marker(
        world,
        "outcome: a target leaving the territory ends a Neutral ship's retaliation",
        serde_json::json!({ "state": format!("{state:?}"), "relation_to_player": format!("{relation:?}") }),
    );
    capture(world, &leash_shot(), picket);
}

// --- beat 8: forced undock ------------------------------------------------

#[cfg(feature = "debug")]
fn record_forced_undock_beat(world: &mut World) {
    let trader = hull(world, TRADER_ID);
    info!("encounters: PASS forced undock: the player hitting the docked trader ended the dock at once");
    probe_marker(
        world,
        "outcome: the player hitting a docked Neutral ship forces the undock",
        serde_json::json!({ "forced_undock": true }),
    );
    capture(world, &forced_undock_shot(), trader);
}

// --- beat 9: the ally engages the Enemy -----------------------------------

/// Whether the escort and the marauder target each other and both engage.
#[cfg(feature = "debug")]
fn engaged_pair(world: &World) -> Option<(Entity, Entity)> {
    let escort = staged_hull(world, ESCORT_ID)?;
    let marauder = staged_hull(world, MARAUDER_ID)?;
    (ai_target(world, escort) == Some(marauder)
        && ai_target(world, marauder) == Some(escort)
        && engages(behavior(world, escort))
        && engages(behavior(world, marauder)))
    .then_some((escort, marauder))
}

#[cfg(feature = "debug")]
fn mutual_engagement() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| engaged_pair(world).is_some())
}

/// The pair is still engaged and each has landed at least one fired round on
/// the other since the ledger's mark.
#[cfg(feature = "debug")]
fn live_fire_exchanged() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        engaged_pair(world).is_some_and(|(escort, marauder)| {
            let ledger = world.resource::<HitLedger>();
            ledger.landed_since_mark(escort, marauder, true)
                && ledger.landed_since_mark(marauder, escort, true)
        })
    })
}

/// Fired rounds still in flight, per owning ship.
#[cfg(feature = "debug")]
fn rounds_in_flight(world: &mut World, owner: Entity) -> usize {
    world
        .query_filtered::<&ProjectileOwner, With<TurretBulletProjectileMarker>>()
        .iter(world)
        .filter(|projectile| projectile.0 == owner)
        .count()
}

#[cfg(feature = "debug")]
fn record_ally_engages_beat(world: &mut World) {
    let player = hull(world, PLAYER_ID);
    let escort = hull(world, ESCORT_ID);
    let marauder = hull(world, MARAUDER_ID);
    let escort_rounds = rounds_in_flight(world, escort);
    let marauder_rounds = rounds_in_flight(world, marauder);
    let ledger = world.resource::<HitLedger>();
    let count = |shooter, struck| {
        ledger
            .hits
            .iter()
            .filter(|hit| hit.shooter == shooter && hit.struck == struck && hit.projectile)
            .count()
    };
    let (escort_hits, marauder_hits) = (count(escort, marauder), count(marauder, escort));
    let relation = relation_between(world, player, escort);
    assert_eq!(
        relation,
        Relation::Own,
        "world_encounters: beat 9: ship_relation(player, escort) is {relation:?}"
    );
    info!("encounters: PASS ally engages: the escort and the marauder target each other and engage; fired-round hit events at the root escort->marauder {escort_hits}, marauder->escort {marauder_hits}; rounds in flight escort {escort_rounds}, marauder {marauder_rounds}");
    probe_marker(
        world,
        "outcome: an armed Player-aligned ship engages an armed Enemy with real fire",
        serde_json::json!({
            "escort_hits": escort_hits,
            "marauder_hits": marauder_hits,
            "escort_rounds_in_flight": escort_rounds,
            "marauder_rounds_in_flight": marauder_rounds,
        }),
    );
    capture_pair(world, &ally_engages_shot(), escort, marauder);
}

// --- beat 10: the ally stays allied ---------------------------------------

#[cfg(feature = "debug")]
fn record_ally_stays_allied_beat(world: &mut World) {
    let player = hull(world, PLAYER_ID);
    let escort = hull(world, ESCORT_ID);
    let marauder = hull(world, MARAUDER_ID);
    assert!(
        world
            .resource::<HitLedger>()
            .landed_since_mark(player, escort, false),
        "world_encounters: beat 10: the player's hit never reached the escort"
    );
    assert_eq!(
        answering(world, escort),
        None,
        "world_encounters: beat 10: the escort answers a ship after the player hit it"
    );
    let relation = relation_between(world, player, escort);
    assert_eq!(
        relation,
        Relation::Own,
        "world_encounters: beat 10: ship_relation(player, escort) is {relation:?} after the hit"
    );
    assert_eq!(
        world.get::<Allegiance>(escort),
        Some(&Allegiance::Player),
        "world_encounters: beat 10: the escort changed side"
    );
    let marauder_neutralized = world.get::<NeutralizedMarker>(marauder).is_some();
    info!("encounters: PASS ally stays allied: after the player's mid-fight hit the escort answers nobody, kept the marauder as its target through an exchange of fire, and reads {relation:?}; marauder neutralized: {marauder_neutralized}");
    probe_marker(
        world,
        "outcome: a Player-aligned ship hit by the player stays allied",
        serde_json::json!({
            "relation": format!("{relation:?}"),
            "kept_marauder_target": true,
            "marauder_neutralized": marauder_neutralized,
        }),
    );
    capture_pair(world, &ally_stays_allied_shot(), escort, marauder);
}

// --- beat 11: unarmed Enemy -----------------------------------------------

#[cfg(feature = "debug")]
fn record_unarmed_enemy_beat(world: &mut World) {
    let player = hull(world, PLAYER_ID);
    let hulk = hull(world, HULK_ID);
    let sees_player = world.get::<SensorContacts>(hulk).is_some_and(|contacts| {
        contacts
            .iter()
            .any(|contact| contact.entity == player && contact.in_sight && contact.is_hostile())
    });
    let distance_m =
        Meters3::from_engine(live_center(world, hulk).0 - live_center(world, player).0)
            .length()
            .get();
    let state = behavior(world, hulk);
    assert!(
        world.get::<AINonCombatant>(hulk).is_some(),
        "world_encounters: beat 11: the unarmed hulk is not an AINonCombatant"
    );
    assert_eq!(
        world.get::<Allegiance>(hulk),
        Some(&Allegiance::Enemy),
        "world_encounters: beat 11: the hulk is not on the Enemy side"
    );
    assert!(
        sees_player,
        "world_encounters: beat 11: the hulk has no hostile in-sight contact on the player"
    );
    assert_eq!(
        ai_target(world, hulk),
        None,
        "world_encounters: beat 11: the unarmed hulk picked a target"
    );
    assert!(
        !engages(state),
        "world_encounters: beat 11: the unarmed hulk is engaging ({state:?})"
    );
    assert!(
        !dock_admits(world, player, hulk),
        "world_encounters: beat 11: DockAdmission admits the player to an active Enemy"
    );
    info!("encounters: PASS unarmed Enemy: the hulk sees the player as hostile at {distance_m:.0} m, targets nothing, stays {state:?}, and refuses a dock");
    probe_marker(
        world,
        "outcome: an unarmed Enemy sees the player and does not chase",
        serde_json::json!({ "distance_m": distance_m, "state": format!("{state:?}") }),
    );
    capture(world, &unarmed_enemy_shot(), hulk);
}

// --- beat 12: last thruster -----------------------------------------------

#[cfg(feature = "debug")]
fn destroy_all_but_one_hulk_thruster(world: &mut World) {
    let player = hull(world, PLAYER_ID);
    let hulk = hull(world, HULK_ID);
    let thrusters = thruster_sections_of(world, hulk);
    assert!(
        thrusters.len() >= 2,
        "world_encounters: beat 12: the hulk has {} thruster section(s); the rule needs two to show the last one matters",
        thrusters.len()
    );
    for section in &thrusters[1..] {
        destroy_section(world, *section, player);
    }
    info!(
        "encounters: destroyed {} of the hulk's {} thruster sections",
        thrusters.len() - 1,
        thrusters.len()
    );
}

#[cfg(feature = "debug")]
fn hulk_working_thrusters_are(count: usize) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(move |world: &World| {
        staged_hull(world, HULK_ID)
            .is_some_and(|hulk| working_thrusters_of(world, hulk).len() == count)
    })
}

#[cfg(feature = "debug")]
fn assert_hulk_not_neutralized_then_destroy_last_thruster(world: &mut World) {
    let player = hull(world, PLAYER_ID);
    let hulk = hull(world, HULK_ID);
    assert!(
        world.get::<NeutralizedMarker>(hulk).is_none(),
        "world_encounters: beat 12: the hulk was neutralized with a working thruster left"
    );
    assert_eq!(
        answering(world, hull(world, PICKET_ID)),
        Some(hulk),
        "world_encounters: beat 12: the picket let go of a hulk that is still in the world"
    );
    let last = working_thrusters_of(world, hulk);
    assert_eq!(
        last.len(),
        1,
        "world_encounters: beat 12: the hulk has {} working thrusters, not one",
        last.len()
    );
    info!("encounters: one working thruster left, the hulk is not neutralized and the picket still answers it");
    destroy_section(world, last[0], player);
}

/// The hulk is neutralized, and the picket answering it has let go.
#[cfg(feature = "debug")]
fn hulk_neutralized_and_picket_released() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let (Some(hulk), Some(picket)) =
            (staged_hull(world, HULK_ID), staged_hull(world, PICKET_ID))
        else {
            return false;
        };
        world.get::<NeutralizedMarker>(hulk).is_some() && answering(world, picket).is_none()
    })
}

#[cfg(feature = "debug")]
fn record_last_thruster_beat(world: &mut World) {
    let hulk = hull(world, HULK_ID);
    info!("encounters: PASS last thruster: losing its last working thruster neutralized the hulk, and the picket let go of it");
    probe_marker(
        world,
        "outcome: an unarmed ship losing its last thruster is neutralized and released",
        serde_json::json!({ "neutralized": true, "picket_released": true }),
    );
    capture(world, &last_thruster_shot(), hulk);
}

// --- beat 13: boarding ----------------------------------------------------

#[cfg(feature = "debug")]
fn request_player_hulk_dock(world: &mut World) {
    let player = hull(world, PLAYER_ID);
    let hulk = hull(world, HULK_ID);
    assert!(
        dock_admits(world, player, hulk),
        "world_encounters: beat 13: DockAdmission refuses a neutralized Enemy"
    );
    align_for_docking(world, player, hulk);
    world.trigger(DockingConnectionRequest {
        entity: player,
        target: hulk,
    });
}

#[cfg(feature = "debug")]
fn record_boarding_beat(world: &mut World) {
    let hulk = hull(world, HULK_ID);
    info!(
        "encounters: PASS boarding: the neutralized Enemy hulk admitted and took the player's dock"
    );
    probe_marker(
        world,
        "outcome: a neutralized Enemy ship admits a boarding dock",
        serde_json::json!({ "docked": true }),
    );
    capture(world, &boarding_shot(), hulk);
}

// --- beat 14: loot --------------------------------------------------------

/// The Inventory pane's row for `item` on the docked partner's side.
#[cfg(feature = "debug")]
fn partner_row(item: &str) -> String {
    format!("InventoryRowPartner{item}")
}

/// A ship's stacks in item order and its credits.
#[cfg(feature = "debug")]
fn holdings(world: &World, ship: Entity) -> (Vec<(ItemDesignId, u32)>, u32) {
    let stacks = world
        .get::<ShipInventory>(ship)
        .expect("world_encounters: every ship root has a ShipInventory")
        .stacks()
        .map(|(item, count)| (item.clone(), count))
        .collect();
    let credits = world
        .get::<ShipCredits>(ship)
        .expect("world_encounters: every ship root has ShipCredits")
        .0;
    (stacks, credits)
}

/// The boarded hulk still holds what it spawned with, the player holds
/// nothing, and the hulk's Take eligibility comes from its neutralization
/// alone: it is not a lootable derelict and is still on the Enemy side.
#[cfg(feature = "debug")]
fn assert_loot_seeded(world: &mut World) {
    let player = hull(world, PLAYER_ID);
    let hulk = hull(world, HULK_ID);
    assert_eq!(
        holdings(world, hulk),
        (
            HULK_STOCK
                .iter()
                .map(|&(id, count)| (id.into(), count))
                .collect(),
            HULK_CREDITS,
        ),
        "world_encounters: beat 14: the boarded hulk does not hold its seeded stock and credits"
    );
    assert_eq!(
        holdings(world, player),
        (Vec::new(), 0),
        "world_encounters: beat 14: the player did not spawn with an empty hold and no credits"
    );
    assert!(
        world.get::<NeutralizedMarker>(hulk).is_some()
            && world.get::<LootableShipMarker>(hulk).is_none(),
        "world_encounters: beat 14: the hulk's Take eligibility does not come from its neutralization alone"
    );
    assert_eq!(
        world.get::<Allegiance>(hulk),
        Some(&Allegiance::Enemy),
        "world_encounters: beat 14: the hulk is not on the Enemy side"
    );
}

/// Pick the hulk's `item` row, set All, and Confirm. The step advances once
/// the hulk's count of `item` moves, then the next asserts it moved whole:
/// a partial Take fails here, not at a deadline.
#[cfg(feature = "debug")]
fn take_all(script: Script, item: &'static str, count: u32) -> Script {
    // The script is built before content loads, so steps name the item id.
    let label = item;
    script
        .click_named(
            &format!("beat 14: pick {label} on the hulk's side"),
            &partner_row(item),
            ui_node_present("InventoryDraftConfirm"),
            STEP_DEADLINE_SECS,
        )
        .click_named(
            &format!("beat 14: set All {label}"),
            "InventoryDraftAll",
            pointer_released(),
            STEP_DEADLINE_SECS,
        )
        .click_named(
            &format!("beat 14: confirm the {label} Take"),
            "InventoryDraftConfirm",
            hulk_count_moved(item, count),
            STEP_DEADLINE_SECS,
        )
        .step(format!("beat 14: the whole {label} stack moved"))
        .on_enter(move |world: &mut World| {
            let player = hull(world, PLAYER_ID);
            let hulk = hull(world, HULK_ID);
            let item_id = item.into();
            let taken = world
                .get::<ShipInventory>(player)
                .map(|own| own.count(&item_id));
            let left = world
                .get::<ShipInventory>(hulk)
                .map(|hold| hold.count(&item_id));
            assert_eq!(
                (taken, left),
                (Some(count), Some(0)),
                "world_encounters: beat 14: the {label} Take moved part of the hulk's {count}"
            );
            info!("encounters: took all {count} {label} from the hulk");
        })
        .add()
}

#[cfg(feature = "debug")]
fn hulk_count_moved(
    item: &'static str,
    count: u32,
) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(move |world: &World| {
        staged_hull(world, HULK_ID)
            .and_then(|hulk| world.get::<ShipInventory>(hulk))
            .is_some_and(|hold| hold.count(&item.into()) != count)
    })
}

#[cfg(feature = "debug")]
fn hulk_credits_moved() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        staged_hull(world, HULK_ID)
            .and_then(|hulk| world.get::<ShipCredits>(hulk))
            .is_some_and(|credits| credits.0 != HULK_CREDITS)
    })
}

/// Every seeded item and credit moved whole to the player, nothing was made
/// or lost, and the hulk is left empty at zero.
#[cfg(feature = "debug")]
fn record_loot_beat(world: &mut World) {
    let player = hull(world, PLAYER_ID);
    let hulk = hull(world, HULK_ID);
    assert_eq!(
        holdings(world, hulk),
        (Vec::new(), 0),
        "world_encounters: beat 14: the hulk kept part of its stock or credits"
    );
    assert_eq!(
        holdings(world, player),
        (
            HULK_STOCK.iter().map(|&(id, count)| (id.into(), count)).collect(),
            HULK_CREDITS,
        ),
        "world_encounters: beat 14: the player does not hold exactly the hulk's seeded stock and credits"
    );
    info!("encounters: PASS loot: the Inventory pane took the neutralized Enemy hulk's whole stock {HULK_STOCK:?} and {HULK_CREDITS} cr");
    probe_marker(
        world,
        "outcome: a boarded neutralized Enemy ship gives up its whole stock and credits",
        serde_json::json!({ "stock": format!("{HULK_STOCK:?}"), "credits": HULK_CREDITS }),
    );
    shoot(world, &loot_shot());
}
