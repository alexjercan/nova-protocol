//! loop_helm_orders: an AI gunship under a PatrolShip order, interrupted by a
//! hostile and given its leg back, watched from above.
//!
//! The gunship flies one authored leg with the helm order owning its helm. A
//! bystander crosses the route and turns hostile as it does; the ship's own
//! interruption policy (`OnHostileContact`) takes the helm from the order, the
//! gunship breaks off and fights, and when the contact is gone the order gets
//! the helm back and the same leg resumes from where it was left. Nothing here
//! is scripted but the order itself and the crossing.
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load the set, run the cycle, exit clean.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also record the cycle as
//!   `news-0130-helm-orders.webm` (staged under `NOVA_CAPTURE_DIR`).

use std::collections::HashSet;

use bevy::prelude::*;
use nova_authoring::prelude::*;
use nova_protocol::prelude::*;

const GUNSHIP_ID: &str = "orders_gunship";
const CROSSER_ID: &str = "orders_crosser";

/// Where the gunship starts, and the mark its one leg flies to: 800 m due
/// east along the route, so the leg runs left to right across the frame.
const LEG_START: Meters3 = Meters3::new(-400.0, 0.0, 0.0);
const LEG_END: Meters3 = Meters3::new(400.0, 0.0, 0.0);
/// The margin the gunship's computer leaves at a waypoint. Small, so the leg
/// visibly runs to its mark instead of ending 500 m short of it.
const WAYPOINT_MARGIN: Meters = Meters(30.0);

/// Where the crosser waits, north of the route and ahead of the gunship, and
/// the speed it crosses at once sent. Slow: a skiff takes some ten seconds of
/// gunfire to lose its computer, and it carries its wreck across the route at
/// this speed for the whole of that, so the fight stays in the frame instead
/// of running off the bottom of it.
const CROSSER_START: Meters3 = Meters3::new(120.0, 0.0, -300.0);
const CROSSER_SPEED: MetersPerSecond = MetersPerSecond(35.0);
/// The gunship's position along the route that sends the crosser: early in
/// the leg, so the loop opens on a few seconds of the order being flown and
/// spends the rest on the interruption.
const SEND_AT_X: Meters = Meters(-250.0);

/// The order's authored key, as it would read in a scenario's log.
const ORDER_KEY: &str = "sweep_east";

fn main() -> bevy::app::AppExit {
    let mut app = AppBuilder::new().with_game_plugins(orders_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(orders_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_systems(Update, drive_fight_camera);
    }

    app.run()
}

fn orders_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_orders);
}

fn load_orders(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    ships: Res<GameShips>,
) {
    let scenario = orders(&game_assets);
    refuse_broken(&scenario, &sections, &ships);
    commands.trigger(LoadScenario(scenario));
}

fn orders(game_assets: &GameAssets) -> ScenarioConfig {
    let objects = [gunship(), crosser()];

    ScenarioConfig {
        description: "An AI gunship on a helm order, a hostile across its route".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: objects
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .chain(ThreePointRig::around("orders", Meters3::ZERO, 40.0).actions())
                .collect(),
        }],
        ..ScenarioConfig::new(
            "loop_helm_orders".to_string(),
            "Helm Orders".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The friendly gunship, flying itself, with a policy that lets its own
/// judgement interrupt an order while a hostile is in contact.
fn gunship() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: GUNSHIP_ID.to_string(),
            name: "Patrol Gunship".to_string(),
            position: LEG_START,
            rotation: Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::X),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig {
                order_interruption: Some(AIOrderInterruption::OnHostileContact),
                arrival_standoff: Some(WAYPOINT_MARGIN),
                ..default()
            }),
            allegiance: Some(Allegiance::Player),
            hull: hull(BLOCK_GUNSHIP_SHIP_ID),
            ..default()
        }),
    }
}

/// The crosser: an unarmed salvage skiff nobody drives, a bystander until the
/// script sends it across the route and declares it.
fn crosser() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: CROSSER_ID.to_string(),
            name: "Crosser".to_string(),
            position: CROSSER_START,
            rotation: Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::Z),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance: Some(Allegiance::Neutral),
            hull: hull(BLOCK_SKIFF_SHIP_ID),
            ..default()
        }),
    }
}

fn refuse_broken(scenario: &ScenarioConfig, sections: &GameSections, ships: &GameShips) {
    let known = KnownSections::from_configs(sections.iter());
    let issues = lint_scenario(
        scenario,
        &known,
        &KnownShips::from_configs(ships.iter()),
        &HashSet::from([scenario.id.clone()]),
        &build_channels()
            .into_iter()
            .map(|channel| channel.id)
            .collect(),
    );
    let errors: Vec<_> = issues
        .iter()
        .filter(|issue| issue.severity == LintSeverity::Error)
        .map(|issue| issue.message.as_str())
        .collect();
    assert!(
        errors.is_empty(),
        "loop_helm_orders: the set failed content lint:\n  {}",
        errors.join("\n  "),
    );
}

#[cfg(feature = "debug")]
const LOOP_NAME: &str = "news-0130-helm-orders";

/// The lens hangs over the fight and follows it: the gunship's own attack
/// runs carry it several hundred meters off the route, and a fixed frame
/// wide enough for those makes both hulls specks. The lowest it stands, in
/// meters, and the clear space it keeps inside the frame's edges.
#[cfg(feature = "debug")]
const LENS_MIN_HEIGHT: Meters = Meters(420.0);
#[cfg(feature = "debug")]
const LENS_MARGIN: Meters = Meters(120.0);
/// How far south of what it frames the lens stands, as a share of its
/// height: a little tilt, so the hulls read as hulls and not as plan views.
#[cfg(feature = "debug")]
const LENS_TILT: f32 = 0.28;
/// How fast the lens settles on its solved pose, per second.
#[cfg(feature = "debug")]
const LENS_SETTLE: f32 = 2.5;
/// What the lens covers at 45 degrees tall in a 16:9 frame, per meter of
/// height: north to south, and east to west.
#[cfg(feature = "debug")]
const LENS_SPAN_TALL: f32 = 0.83;
#[cfg(feature = "debug")]
const LENS_SPAN_WIDE: f32 = 1.47;

/// How long the loop holds after the order has its helm back.
#[cfg(feature = "debug")]
const RESUMED_HOLD_SECS: f32 = 2.0;
/// Real seconds the fight may take before the run fails: the contact has to
/// be destroyed for the sky to clear, and a gunship that cannot do that is a
/// different picture.
#[cfg(feature = "debug")]
const FIGHT_DEADLINE_SECS: f32 = 120.0;

#[cfg(feature = "debug")]
fn entity_by_id(world: &mut World, id: &str) -> Option<Entity> {
    let mut query = world.query::<(Entity, &EntityId)>();
    query
        .iter(world)
        .find(|(_, live)| live.0 == id)
        .map(|(entity, _)| entity)
}

#[cfg(feature = "debug")]
fn entity_by_id_ref(world: &World, id: &str) -> Option<Entity> {
    world
        .iter_entities()
        .find(|entity| entity.get::<EntityId>().is_some_and(|live| live.0 == id))
        .map(|entity| entity.id())
}

/// The filtered pose of the lens over the fight.
#[cfg(feature = "debug")]
#[derive(Resource, Clone, Copy)]
struct FightLens {
    position: Meters3,
    look_at: Meters3,
}

/// Arm the lens over the route and take the HUD down.
#[cfg(feature = "debug")]
fn frame_from_above(world: &mut World) {
    hide_hud(world);
    drive_fight_camera(world);
}

/// A ship's position in meters, while it is a ship: a root the reaper has
/// neutralized is a wreck, and the lens stops framing it.
#[cfg(feature = "debug")]
fn live_ship_position(world: &World, id: &str) -> Option<Meters3> {
    let ship = entity_by_id_ref(world, id)?;
    if world.get::<NeutralizedMarker>(ship).is_some() {
        return None;
    }
    // Engine boundary: the avian position is world units.
    world
        .get::<avian3d::prelude::Position>(ship)
        .map(|position| Meters3::from_engine(position.0))
}

/// Re-solve the lens over the gunship and, while it is a live ship, the
/// crosser: high enough to hold both inside the margin, never lower than
/// [`LENS_MIN_HEIGHT`], and settling there over a second or so.
#[cfg(feature = "debug")]
fn drive_fight_camera(world: &mut World) {
    let Some(gunship) = live_ship_position(world, GUNSHIP_ID) else {
        return;
    };
    let (centre, height) = match live_ship_position(world, CROSSER_ID) {
        Some(crosser) => {
            let span = (gunship.get() - crosser.get()).abs();
            let clear = 2.0 * LENS_MARGIN.get();
            let height = LENS_MIN_HEIGHT
                .get()
                .max((span.z + clear) / LENS_SPAN_TALL)
                .max((span.x + clear) / LENS_SPAN_WIDE);
            (Meters3((gunship.get() + crosser.get()) * 0.5), height)
        }
        None => (gunship, LENS_MIN_HEIGHT.get()),
    };
    let desired_position = centre + Meters3(Vec3::new(0.0, height, height * LENS_TILT));
    let delta = world.resource::<Time>().delta_secs().min(0.1);
    let alpha = 1.0 - (-LENS_SETTLE * delta).exp();
    let mut lens = world.remove_resource::<FightLens>().unwrap_or(FightLens {
        position: desired_position,
        look_at: centre,
    });
    lens.position = Meters3(lens.position.get().lerp(desired_position.get(), alpha));
    lens.look_at = Meters3(lens.look_at.get().lerp(centre.get(), alpha));
    pose_camera(world, lens.position, lens.look_at);
    world.insert_resource(lens);
}

/// Install the one-leg patrol on the gunship: the same three components the
/// `PatrolShip` action installs, on the same ship, minus the scenario that
/// would report its lifecycle.
#[cfg(feature = "debug")]
fn give_the_order(world: &mut World) {
    let Some(ship) = entity_by_id(world, GUNSHIP_ID) else {
        panic!("loop_helm_orders: no gunship '{GUNSHIP_ID}' to order");
    };
    cancel_ship_order(world, ship);
    let mut entity = world.entity_mut(ship);
    if !entity.contains::<ShipOrderReports>() {
        entity.insert(ShipOrderReports::default());
    }
    entity.insert((
        ShipHelmOrder::new(
            ORDER_KEY.to_string(),
            ShipOrderDirective::Patrol {
                // Engine boundary: the leg is flown against an avian position.
                waypoints: vec![LEG_END.to_engine()],
                leg: 0,
            },
        ),
        ShipOrderHelmAuthority,
    ));
    info!("loop_helm_orders: '{ORDER_KEY}' installed on '{GUNSHIP_ID}'");
}

/// Send the crosser across the route and declare it: the moment it reads
/// hostile it is a contact, and the gunship's policy takes the helm.
#[cfg(feature = "debug")]
fn send_the_crosser(world: &mut World) {
    let Some(ship) = entity_by_id(world, CROSSER_ID) else {
        panic!("loop_helm_orders: no crosser '{CROSSER_ID}' to send");
    };
    world.entity_mut(ship).insert((
        // Engine boundary: the crossing is an avian velocity.
        avian3d::prelude::LinearVelocity(Vec3::Z * CROSSER_SPEED.to_engine()),
        Allegiance::Enemy,
    ));
    info!("loop_helm_orders: crosser sent across the route, hostile");
}

/// The gunship has flown east of `x`, in meters.
#[cfg(feature = "debug")]
fn gunship_past(x: Meters) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        entity_by_id_ref(world, GUNSHIP_ID)
            .and_then(|ship| world.get::<avian3d::prelude::Position>(ship))
            // Engine boundary: the avian position is world units.
            .is_some_and(|position| Meters::from_engine(position.0.x) > x)
    })
}

/// The gunship's own judgement holds its helm: the order is interrupted.
#[cfg(feature = "debug")]
fn order_interrupted() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        entity_by_id_ref(world, GUNSHIP_ID)
            .is_some_and(|ship| world.get::<AIOrderInterrupted>(ship).is_some())
    })
}

/// The order has its helm back: the interruption is gone and the authority
/// is on the ship again.
#[cfg(feature = "debug")]
fn order_resumed() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        entity_by_id_ref(world, GUNSHIP_ID).is_some_and(|ship| {
            world.get::<AIOrderInterrupted>(ship).is_none()
                && world.get::<ShipOrderHelmAuthority>(ship).is_some()
        })
    })
}

#[cfg(feature = "debug")]
fn orders_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the set")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("frame the route from above")
        .on_enter(frame_from_above)
        .until(elapsed(0.4))
        .add()
        .step("open the loop and give the order")
        .on_enter(|world| {
            loop_start(world, LOOP_NAME);
            give_the_order(world);
        })
        .until(gunship_past(SEND_AT_X))
        .deadline(FIGHT_DEADLINE_SECS)
        .add()
        .step("send the crosser")
        .on_enter(send_the_crosser)
        .until(order_interrupted())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let the gunship fight")
        .until(order_resumed())
        .deadline(FIGHT_DEADLINE_SECS)
        .add()
        .step("hold the resumed leg")
        .until(elapsed(RESUMED_HOLD_SECS))
        .add()
        .step("close the loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(60.0)
        .add()
}
