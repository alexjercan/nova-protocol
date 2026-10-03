//! lesson_cargo: the three demonstrations of the item loop - `interface_cargo_transfer`
//! (Take from a docked derelict), `interface_jettison` (the intake dropping a
//! canister) and `flight_cargo_pickup` (the intake taking it back).
//!
//! One producer, three sheets, because they are one session with one load of
//! hull plates: the plates the Take moves into the hold are the plates the
//! Jettison drops, and the canister the Jettison drops is the canister the
//! pickup takes.
//!
//! ## The scene is a staged berth
//!
//! The shipped `block_line_warship` and a Derelict Tender
//! (`block_frame_tender_damaged`, lootable, 8 hull plates) stand berthed: the
//! two port collars square and 5 m apart, inside the 10 m capture gap. New
//! Game streams its wrecks at seeded poses, so this scene stages its own
//! derelict to make the take repeatable. The walk docks with the real `dock`
//! key and drives the Inventory pane with the pointer, so the Take, the
//! Jettison, the door and the take are the production path.
//!
//! ## The three sheets
//!
//! - `interface_cargo_transfer`: the pane docked to the derelict; the
//!   derelict's hull plates selected, All, Confirm, and the plates move into
//!   the warship's column.
//! - `interface_jettison`: from the door half open, the intake's door folding
//!   the rest of the way and the canister of plates leaving it. The camera
//!   rides beside the hull in world axes ([`LessonChase`](lesson::LessonChase)).
//! - `flight_cargo_pickup`: the warship rising the last meters under the
//!   canister on the flight computer's `MatchVelocity`, the primitive every AI
//!   pilot steers with, and the canister going into the hold.
//!
//! The run fails loudly if the dock does not take, if the Take does not move
//! all 8 plates, if the Jettison does not queue exactly one canister of them,
//! if the intake drops anything else, or if the take does not bring all 8
//! back.
//!
//! ## The web loop mode
//!
//! `NOVA_CARGO_WEB_LOOP=1` records the jettison as the site's
//! `loop-section-cargo-jettison` webm on the default loop profile (30 fps,
//! 720p) instead of the three sheets on the lesson profile (10 fps), and ends
//! once the loop is written. `scripts/capture-web-media.sh` sets it.
//!
//! Run modes, all under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the whole walk, recording
//!   nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the three sheets (staged
//!   under `NOVA_CAPTURE_DIR`).
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 NOVA_CARGO_WEB_LOOP=1`: record the
//!   jettison webm instead.
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_cargo --features debug
//! ```

#[path = "shared/kit.rs"]
mod kit;
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
use nova_input::prelude::InputSource;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_cargo")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's Take, Jettison and pickup demonstrations", long_about = None)]
struct Cli;

const PLAYER_ID: &str = "player";
const DERELICT_ID: &str = "derelict_tender";

/// The hull plates the derelict carries.
const DERELICT_PLATES: u32 = 8;

/// The warship at the berth, where its port collar face stands 5 m from the
/// derelict's.
const PLAYER_POSITION: Meters3 = Meters3::new(0.0, 0.0, -140.0);
/// The derelict, off the warship's port collar.
const DERELICT_POSITION: Meters3 = Meters3::new(-55.0, 0.0, -170.0);
/// Turned 180 degrees about Y, so its port collar faces the warship's.
const DERELICT_ROTATION: Quat = Quat::from_xyzw(0.0, 1.0, 0.0, 0.0);

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    walk::add(&mut app);

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    let player = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLAYER_ID.to_string(),
            name: "Line Warship".to_string(),
            position: PLAYER_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: [(
                    "mining_beam".to_string(),
                    vec![InputSource::Keyboard(KeyCode::KeyV)],
                )]
                .into(),
            }),
            design: ShipDesignSource::Inline(kit::catalog_ship(&ships, "block_line_warship")),
            inventory: ShipInventoryStock::new([]),
            ..default()
        }),
    });
    let derelict = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: DERELICT_ID.to_string(),
            name: "Derelict Tender".to_string(),
            position: DERELICT_POSITION,
            rotation: DERELICT_ROTATION,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Neutral),
            controller: SpaceshipController::None,
            design: ShipDesignSource::Inline(kit::catalog_ship(
                &ships,
                "block_frame_tender_damaged",
            )),
            inventory: ShipInventoryStock::new([(ItemType::HullPlate, DERELICT_PLATES)]),
            lootable: true,
            credits: 0,
            ..default()
        }),
    });
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "The line warship at the Derelict Tender's berth.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![player, derelict],
                ThreePointRig::around("cargo", PLAYER_POSITION, 25.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "lesson_cargo".to_string(),
            "Cargo".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// The harnessed walk and the flight computer it flies the pickup with.
#[cfg(feature = "debug")]
mod walk {
    use std::sync::Arc;

    use avian3d::prelude::LinearVelocity;
    use nova_protocol::{
        nova_debug::harness::{AutopilotPlugin, LoopCapturePlugin, Predicate},
        nova_interface::pane::InterfacePaneType,
    };

    use super::*;
    use crate::lesson::{lesson_chase_plugin, lesson_profile, LessonChase, LESSON_GRID};

    /// The sheet for "Take and give".
    const TRANSFER_LESSON: &str = "interface_cargo_transfer";
    /// The sheet for "Jettisoning cargo".
    const JETTISON_LESSON: &str = "interface_jettison";
    /// The sheet for "Picking up canisters".
    const PICKUP_LESSON: &str = "flight_cargo_pickup";
    /// The site's jettison loop, recorded in the web loop mode.
    const JETTISON_LOOP: &str = "loop-section-cargo-jettison";

    /// Real-seconds backstop for a state wait, long enough for lavapipe
    /// frames.
    const STEP_DEADLINE: f32 = 120.0;
    /// Real-seconds backstop for the flown pickup: the canister drifts at
    /// 3 m/s and the warship closes on it slowly.
    const FLIGHT_DEADLINE: f32 = 900.0;

    /// Cells the transfer sheet opens on before the first click.
    const LEAD_CELLS: u32 = 2;
    /// Intake door progress (1 is open) at which the jettison sheet opens:
    /// half the door's 1.2 s, so the sheet holds the rest of the fold and
    /// the canister leaving.
    const JETTISON_SHEET_DOOR: f32 = 0.5;
    /// Game seconds the web loop keeps recording after the canister leaves.
    const JETTISON_LOOP_DRIFT_SECS: f32 = 3.0;
    /// Game seconds the canister drifts off before the warship goes after it,
    /// so the pickup starts with the canister clear of the hull.
    const PICKUP_LEAD_SECS: f32 = 5.0;

    /// How far below the canister the intake face waits while the warship
    /// levels under it, in engine units. The canister rose straight off the
    /// face, so nothing of the hull stands between them.
    const HOVER_CLEARANCE: f32 = 1.5;
    /// The gap the lift closes to between the face and the canister's
    /// nearest side, in engine units: inside the intake's 1 m capture gap.
    const LIFT_GAP: f32 = 0.02;
    /// How near the face centre must stand under the canister, across the
    /// face, before the lift starts, in engine units.
    const LIFT_ALIGN: f32 = 0.1;
    /// The fastest the lift closes on the canister, relative to it, in engine
    /// units per second: slow, so the canister meets the trigger slab and not
    /// the hull.
    const LIFT_SPEED: f32 = 0.25;
    /// Velocity per engine unit of position error, per second.
    const GAIN: f32 = 0.8;
    /// The largest correction the helm asks for, in engine units per second.
    const MAX_CORRECTION: f32 = 0.4;
    /// The canister's nearest-side gap to the face at which the pickup sheet
    /// opens, in engine units: about 1.3 s of lift before the take.
    const PICKUP_SHEET_GAP: f32 = 0.25;

    /// Where the jettison eye stands from the intake face, in world axes and
    /// engine units: to starboard, above and aft.
    const JETTISON_EYE: Vec3 = Vec3::new(5.0, 2.0, 5.0);
    /// What the jettison eye looks at from the intake face, along its normal:
    /// the space the canister is born into.
    const JETTISON_AIM: f32 = 0.8;
    /// Where the pickup eye stands from the intake face, in world axes and
    /// engine units.
    const PICKUP_EYE: Vec3 = Vec3::new(4.0, 2.5, 4.0);
    /// What the pickup eye looks at from the intake face, along its normal.
    const PICKUP_AIM: f32 = 0.6;

    /// What the flight computer is asked to do.
    #[derive(Resource, Default, Clone, Copy, PartialEq)]
    enum Helm {
        /// Nothing: the script owns the ship.
        #[default]
        Off,
        /// Hold the intake face under `canister`, then close on it.
        Track { canister: Entity, lift: bool },
    }

    /// What the walk records to compare later.
    #[derive(Resource, Default)]
    struct CargoProof {
        /// Canisters the intake dropped.
        ejected: u32,
        /// Canisters an intake took.
        taken: u32,
        /// Game seconds at the last drop.
        ejected_at: f32,
    }

    /// Whether this run records the site's jettison webm instead of the
    /// lesson sheets.
    fn web_loop() -> bool {
        std::env::var_os("NOVA_CARGO_WEB_LOOP").is_some()
    }

    pub(super) fn add(app: &mut App) {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(LoopCapturePlugin::new(if web_loop() {
            LoopProfile::default()
        } else {
            lesson_profile()
        }));
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(lesson_chase_plugin);
        app.init_resource::<Helm>();
        app.init_resource::<CargoProof>();
        app.add_observer(
            |_: On<CargoCanisterEjected>,
             time: Res<Time<Virtual>>,
             mut proof: ResMut<CargoProof>| {
                proof.ejected += 1;
                proof.ejected_at = time.elapsed_secs();
            },
        );
        app.add_observer(|_: On<CargoCanisterTaken>, mut proof: ResMut<CargoProof>| {
            proof.taken += 1;
        });
        app.add_systems(Update, steer_helm);
        app.add_plugins(script());
    }

    /// Open a lesson sheet, unless this run records the web loop.
    fn open_sheet(world: &mut World, name: &str) {
        if !web_loop() {
            sheet_start(world, name, LESSON_GRID);
        }
    }

    /// Advance once a lesson sheet is written, or at once in the web loop
    /// mode, which records none.
    fn sheet_done(name: &str) -> Arc<Predicate> {
        if web_loop() {
            when(|_| true)
        } else {
            sheet_written(name)
        }
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn world_has<T: Component>(world: &World) -> bool {
        world
            .try_query_filtered::<(), With<T>>()
            .is_some_and(|mut query| query.iter(world).next().is_some())
    }

    fn root(world: &mut World, id: &str) -> Entity {
        kit::ship_root(world, id).unwrap_or_else(|| panic!("lesson_cargo: no ship {id}"))
    }

    /// The hull plates a ship holds.
    fn plates(world: &mut World, id: &str) -> u32 {
        let ship = root(world, id);
        world
            .get::<ShipInventory>(ship)
            .unwrap_or_else(|| panic!("lesson_cargo: {id} has no ShipInventory"))
            .count(ItemType::HullPlate)
    }

    /// Advance once the warship's hold holds `count` hull plates.
    fn the_hold_holds(count: u32) -> Arc<Predicate> {
        when(move |world| {
            world
                .try_query_filtered::<&ShipInventory, With<PlayerSpaceshipMarker>>()
                .is_some_and(|mut ships| {
                    ships
                        .iter(world)
                        .any(|inventory| inventory.count(ItemType::HullPlate) == count)
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

    fn unpaused() -> Arc<Predicate> {
        resource_where::<State<PauseStates>>(|pause| *pause.get() == PauseStates::Unpaused)
    }

    /// The warship's intake face centre and outward normal.
    fn intake_face(world: &mut World) -> (Vec3, Vec3) {
        let mut intakes = world
            .query_filtered::<(&GlobalTransform, &SectionCollider), With<CargoIntakeSectionMarker>>(
            );
        let (intake, collider) = intakes
            .iter(world)
            .next()
            .expect("the warship has an intake");
        cargo_intake_face(intake.translation(), intake.rotation(), *collider)
    }

    /// The intake door's progress, 1 open.
    fn door_progress(world: &World) -> Option<f32> {
        let mut intakes =
            world.try_query_filtered::<&SectionAnimations, With<CargoIntakeSectionMarker>>()?;
        intakes
            .iter(world)
            .next()?
            .cue_progress(SectionAnimationCue::IntakeDoor)
    }

    /// The one live canister, while it exists.
    fn the_canister(world: &mut World) -> Option<Entity> {
        let mut canisters = world.query_filtered::<Entity, With<CargoCanister>>();
        canisters.iter(world).next()
    }

    /// Ride beside the hull, `eye` from the intake face and looking `aim`
    /// out along its normal.
    fn chase_the_intake(world: &mut World, eye: Vec3, aim: f32) {
        let player = root(world, PLAYER_ID);
        let hull = world
            .get::<Transform>(player)
            .expect("the warship has a pose")
            .translation;
        let (face, normal) = intake_face(world);
        let subject = face + normal * aim - hull;
        world.insert_resource(
            LessonChase::new(Meters3::from_engine(subject + eye))
                .looking(Meters3::from_engine(subject)),
        );
    }

    /// The tracked canister's nearest-side gap to the intake face, while it
    /// exists.
    fn pickup_gap(world: &World) -> Option<f32> {
        let Helm::Track { canister, .. } = *world.resource::<Helm>() else {
            return None;
        };
        let canister = world.get::<GlobalTransform>(canister)?;
        let mut intakes = world.try_query_filtered::<(&GlobalTransform, &SectionCollider), With<CargoIntakeSectionMarker>>()?;
        let (intake, collider) = intakes.iter(world).next()?;
        let (face, normal) = cargo_intake_face(intake.translation(), intake.rotation(), *collider);
        let half = (canister.rotation().inverse() * normal)
            .abs()
            .dot(CARGO_CANISTER_SIZE * 0.5);
        Some((canister.translation() - face).dot(normal) - half)
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
        let Helm::Track { canister, lift } = *helm else {
            return;
        };
        let Ok((ship, ship_pose, autopilot)) = q_player.single_mut() else {
            return;
        };
        let at = ship_pose.translation();
        let Some((_, intake, collider)) = q_intakes
            .iter()
            .find(|(&ChildOf(parent), ..)| parent == ship)
        else {
            panic!("lesson_cargo: the warship has no cargo intake");
        };
        let (face, normal) = cargo_intake_face(intake.translation(), intake.rotation(), *collider);
        let velocity = match q_canisters.get(canister) {
            // Taken: hold still for the checks.
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
                let goal = at + (centre - normal * clearance - face);
                let error = goal - at;
                let mut correction = (error * GAIN).clamp_length_max(MAX_CORRECTION);
                if lift {
                    // The closing speed is relative to the canister, which
                    // rises off the face at the drop's own speed.
                    let along = correction.dot(normal);
                    correction += normal * (along.clamp(-LIFT_SPEED, LIFT_SPEED) - along);
                } else if error.reject_from_normalized(normal).length() < LIFT_ALIGN
                    && error.dot(normal).abs() < LIFT_ALIGN
                {
                    info!("lesson_cargo: level under the canister, lifting");
                    *helm = Helm::Track {
                        canister,
                        lift: true,
                    };
                }
                canister_velocity.0 + correction
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

    /// The Take moved every plate from the derelict into the hold.
    fn check_take(world: &mut World) {
        let own = plates(world, PLAYER_ID);
        let partner = plates(world, DERELICT_ID);
        info!("lesson_cargo: after the Take the hold has {own} plate(s), the derelict {partner}");
        assert_eq!(own, DERELICT_PLATES, "the Take moved every plate");
        assert_eq!(partner, 0, "the derelict is empty after the Take");
    }

    /// The Jettison emptied the hold into one queued canister of every plate.
    fn check_jettison(world: &mut World) {
        assert_eq!(plates(world, PLAYER_ID), 0, "the Jettison emptied the hold");
        let queued: Vec<Vec<(ItemType, u32)>> = world
            .query::<&CargoIntakeEjectionQueue>()
            .iter(world)
            .flat_map(|queue| queue.0.iter().map(|canister| canister.stacks().collect()))
            .collect();
        info!("lesson_cargo: the intake queues {queued:?}");
        assert_eq!(
            queued,
            vec![vec![(ItemType::HullPlate, DERELICT_PLATES)]],
            "the Jettison queued one canister of every plate"
        );
    }

    /// The intake dropped the one canister, holding every plate.
    fn check_drop(world: &mut World) {
        assert!(
            !world_has::<CargoIntakeEjectionQueue>(world),
            "the queue is spent"
        );
        let live: Vec<Vec<(ItemType, u32)>> = world
            .query::<&CargoCanister>()
            .iter(world)
            .map(|canister| canister.stacks().collect())
            .collect();
        info!("lesson_cargo: live canisters {live:?}");
        assert_eq!(world.resource::<CargoProof>().ejected, 1);
        assert_eq!(
            live,
            vec![vec![(ItemType::HullPlate, DERELICT_PLATES)]],
            "the intake dropped one canister of every plate"
        );
    }

    /// The take brought every plate back and left nothing in space.
    fn check_pickup(world: &mut World) {
        assert_eq!(world.resource::<CargoProof>().taken, 1);
        assert!(
            !world_has::<CargoCanister>(world),
            "no canister is left in space"
        );
        let own = plates(world, PLAYER_ID);
        info!("lesson_cargo: after the take the hold has {own} plate(s)");
        assert_eq!(own, DERELICT_PLATES, "the take brought every plate back");
    }

    /// Press and release a key action, one frame each.
    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(STEP_DEADLINE)
            .add()
    }

    /// Dock, Take, undock, Jettison and take the canister back.
    fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("load the warship and the derelict")
            .enter(GameStates::Loading)
            .until(and(player_ship_present(), scenario_camera_present()))
            .deadline(STEP_DEADLINE)
            .add()
            .step("settle the scene and drop the status bar")
            .on_enter(hide_status_bar)
            .until(and(scenario_is_built(), frames(10)))
            .deadline(STEP_DEADLINE)
            .add()
            .step("lock the derelict")
            .on_enter(|world: &mut World| {
                let player = root(world, PLAYER_ID);
                let derelict = root(world, DERELICT_ID);
                world.entity_mut(player).insert(TravelLock(Some(derelict)));
            })
            .until(frames(1))
            .add();
        // TAKE.
        let script = tap(
            script,
            "the dock key",
            "dock",
            any_entity::<(With<PlayerSpaceshipMarker>, With<DockedShip>)>(),
        );
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(InterfacePaneType::Map),
        )
        .click_named(
            "open the Inventory pane",
            "InterfaceTabInventory",
            ui_node_present("InventoryRowPartnerHullPlate"),
            STEP_DEADLINE,
        )
        // The pick and All land before the sheet opens: on the sheet they
        // took 18 of the 20 cells, which left two for the moved plates.
        .click_named(
            "take: pick the derelict's hull plates",
            "InventoryRowPartnerHullPlate",
            ui_node_present("InventoryDraftConfirm"),
            STEP_DEADLINE,
        )
        .click_named(
            "take: all",
            "InventoryDraftAll",
            pointer_released(),
            STEP_DEADLINE,
        )
        .step("open the transfer sheet on the Take form")
        .on_enter(|world: &mut World| open_sheet(world, TRANSFER_LESSON))
        .until(frames(LEAD_CELLS))
        .add()
        .click_named(
            "take: confirm",
            "InventoryDraftConfirm",
            the_hold_holds(DERELICT_PLATES),
            STEP_DEADLINE,
        )
        .step("check the Take")
        .on_enter(check_take)
        .until(frames(1))
        .add()
        .step("hold the moved plates until the transfer sheet is written")
        .until(sheet_done(TRANSFER_LESSON))
        .deadline(60.0)
        .add();
        let script = tap(script, "the interface key", "interface_toggle", unpaused());
        // JETTISON. Undocked first: docked, the pane offers Give, not
        // Jettison.
        let script = tap(
            script,
            "the dock key",
            "dock",
            when(|world| !world_has::<DockedShip>(world)),
        )
        .step("let the sleeves come in")
        .until(elapsed(1.5))
        .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(InterfacePaneType::Inventory),
        )
        .click_named(
            "jettison: pick the hull plates",
            "InventoryRowOwnHullPlate",
            ui_node_present("InventoryDraftConfirm"),
            STEP_DEADLINE,
        )
        .click_named(
            "jettison: all",
            "InventoryDraftAll",
            pointer_released(),
            STEP_DEADLINE,
        )
        .click_named(
            "jettison: confirm",
            "InventoryDraftConfirm",
            the_hold_holds(0),
            STEP_DEADLINE,
        )
        .step("check the Jettison")
        .on_enter(check_jettison)
        .until(frames(1))
        .add()
        .step("stand the eye beside the intake")
        .on_enter(|world: &mut World| chase_the_intake(world, JETTISON_EYE, JETTISON_AIM))
        .until(frames(1))
        .add();
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("open the web loop as the door starts to fold")
            .on_enter(|world: &mut World| {
                if web_loop() {
                    loop_start(world, JETTISON_LOOP);
                }
            })
            .until(when(|world| {
                door_progress(world).is_some_and(|door| door >= JETTISON_SHEET_DOOR)
            }))
            .deadline(STEP_DEADLINE)
            .add()
            .step("open the jettison sheet with the door half open")
            .on_enter(|world: &mut World| open_sheet(world, JETTISON_LESSON))
            .until(when(|world| world.resource::<CargoProof>().ejected >= 1))
            .deadline(STEP_DEADLINE)
            .add()
            .step("check the drop")
            .on_enter(check_drop)
            .until(frames(1))
            .add()
            .step("hold the leaving canister until the jettison sheet is written")
            .until(sheet_done(JETTISON_LESSON))
            .deadline(60.0)
            .add();
        if web_loop() {
            return script
                .step("let the canister drift off")
                .until(when(|world| {
                    let now = world.resource::<Time<Virtual>>().elapsed_secs();
                    now - world.resource::<CargoProof>().ejected_at >= JETTISON_LOOP_DRIFT_SECS
                }))
                .deadline(STEP_DEADLINE)
                .add()
                .step("close the web loop")
                .on_enter(|world: &mut World| loop_end(world, JETTISON_LOOP))
                .until(loop_written(JETTISON_LOOP))
                .deadline(STEP_DEADLINE)
                .add();
        }
        // PICKUP.
        script
            .step("let the canister drift clear")
            .until(when(|world| {
                let now = world.resource::<Time<Virtual>>().elapsed_secs();
                now - world.resource::<CargoProof>().ejected_at >= PICKUP_LEAD_SECS
            }))
            .deadline(STEP_DEADLINE)
            .add()
            .step("fly the intake under the canister and up onto it")
            .on_enter(|world: &mut World| {
                let canister = the_canister(world).expect("the dropped canister is in space");
                *world.resource_mut::<Helm>() = Helm::Track {
                    canister,
                    lift: false,
                };
                chase_the_intake(world, PICKUP_EYE, PICKUP_AIM);
            })
            .until(when(|world| {
                matches!(*world.resource::<Helm>(), Helm::Track { lift: true, .. })
                    && pickup_gap(world).is_some_and(|gap| gap <= PICKUP_SHEET_GAP)
            }))
            .deadline(FLIGHT_DEADLINE)
            .add()
            .step("open the pickup sheet and wait for the take")
            .on_enter(|world: &mut World| open_sheet(world, PICKUP_LESSON))
            .until(when(|world| world.resource::<CargoProof>().taken >= 1))
            .deadline(STEP_DEADLINE)
            .add()
            .step("check the take")
            .on_enter(check_pickup)
            .until(frames(1))
            .add()
            .step("hold the taken canister until the pickup sheet is written")
            .until(sheet_done(PICKUP_LESSON))
            .deadline(60.0)
            .add()
    }
}
