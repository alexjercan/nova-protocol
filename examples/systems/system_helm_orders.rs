//! system_helm_orders: what a scenario's own HELM ORDER owns on an AI hull,
//! and what an interruption is allowed to take from it.
//!
//! One armed shipped combatant flies a route it was GIVEN - a real
//! `PatrolShip` scenario action, installed by the scenario layer out of the
//! same `OnStart` batch that spawns the hull, not by the harness. The ship
//! carries the authored `OnHostileContact` policy, so its own judgement may
//! take the helm back while something hostile is in contact and must give it
//! up again when the sky clears. The subject is the DIRECTIVE across that: the
//! order flies the hull, a real contact takes the helm away without touching
//! what the ship was told, and the resume picks the same leg back up.
//!
//! The hostile is a parked armed raider the ordered hull cannot SEE yet. Its
//! `sensor_range` is authored short so the contact happens where the route
//! takes the ship rather than on the first frame: with the engine's 20 km
//! scanner every hull in a range this size is in contact at spawn, and an
//! order interrupted before it ever engaged proves nothing about flying one.
//! The engage gate stays the shipped 4 km default, which the authored reach
//! is well inside, so seeing the raider and leaving the routine for it are the
//! same moment.
//!
//! Both hulls fly with EMPTY MAGAZINES, for the reason `system_ai_combat`
//! states: the subject is the helm, not gunnery, and a live gun would put a
//! second clock on the raider's life. Its death is applied at the beat
//! boundary through the production damage entry point instead.
//!
//! Sampled ONCE A FIXED STEP, not once a frame. Helm authority changes hands
//! twice in this run and on a software rasterizer one frame holds a dozen
//! fixed steps: a per-frame census would step over the hand-back it is here to
//! watch.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a scenario order takes the helm and flies the hull` | the authored order holds `ShipOrderHelmAuthority`, engaged a GOTO on the mark its own directive names, and carried the hull a real distance and a whole leg under it |
//! | 2 | `outcome: a hostile contact takes the helm and leaves the order alone` | a real contact applied the authored policy: the helm went to the AI and the ship flew its fight, while the stored directive kept its route and its leg for every step of the interruption |
//! | 3 | `outcome: the cleared sky hands the same leg back` | the contact out of the fight, the helm came back once, on the same key, the same route and the same leg, and the hull flew on under it |
//! | 4 | `outcome: the order log is recorded` | RECORD: steps sampled, metres flown under each helm, the helm hand-offs, and the leg index at every mark |
//!
//! The harness NEVER writes `ShipHelmOrder`, `ShipOrderHelmAuthority` or
//! `AIOrderInterrupted`. The scenario installs the order and the ship's own
//! mission policy hands the helm back and forth.
//!
//! What this range does NOT claim, because focused tests already do: the order
//! lifecycle's exactly-once reporting, which the scenario tracker's own tests
//! own; the pure interrupt/resume component moves
//! (`an_interrupted_order_resumes_from_its_own_directive`,
//! `nova_ship/src/flight/order.rs`); patrol leg arithmetic
//! (`a_patrol_loop_ends_where_it_started`, same file); the built-in
//! `AIPatrolRoute` routine and its leash, which are `system_ai_patrol`'s.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_helm_orders --features debug
//! # look for: `helm orders: the order took its helm back ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[path = "../screenshots/shared/kit.rs"]
mod kit;

#[cfg(feature = "debug")]
use std::sync::Arc;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_helm_orders")]
#[command(version = "1.0.0")]
#[command(
    about = "What a scenario helm order owns on an AI hull across a hostile interruption. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

/// The ordered hull: the armed patrol gunship, the same hull the
/// `loop_helm_orders` capture flies an order on.
///
/// Armed on purpose - the claims are about a ship whose own judgement can
/// take a helm back for a fight. An unarmed hull spawns an `AINonCombatant`
/// and never acquires anything, so its order could never be interrupted.
const GUNSHIP: &str = "block_gunship";

/// The hostile the order is interrupted for: the scavenger raider.
///
/// Armed, and so a hull the production neutralize rule can take out of the
/// fight by its bridge alone; an unarmed hull is never "out of the fight"
/// (`nova_gameplay/src/integrity/neutralize.rs`) and would have to be blown
/// apart instead.
const RAIDER: &str = "block_raider";

/// The ordered ship's scenario id.
const GUNSHIP_ID: &str = "helm_orders_gunship";

/// The hostile's scenario id.
const RAIDER_ID: &str = "helm_orders_contact";

/// The authored key the order's lifecycle is reported under.
const ORDER_KEY: &str = "helm_orders_sweep";

/// Where the hostile waits: off the route's axis, and out of the ordered
/// ship's authored sensor reach from both the spawn mark and the whole of the
/// first leg.
const CONTACT_AT: Meters3 = Meters3::new(1_800.0, 0.0, 2_400.0);

/// The ordered ship's authored scanner reach.
///
/// Short, and well inside the shipped 4 km engage gate, so the contact and the
/// decision to leave the routine for it are one moment. See the module
/// comment for why the default 20 km reach cannot stage this.
const SENSOR_REACH: Meters = Meters(2_000.0);

/// How far the hull must have flown under the ORDER's helm before the run
/// calls the order established.
///
/// Comfortably more than the first leg, so the mark cannot be met by a leg
/// that completed without the ship moving.
#[cfg(feature = "debug")]
const ORDER_FLOWN_MARK: Meters = Meters(1_500.0);

/// How far the hull must have flown under the AI's helm before the run clears
/// the sky.
///
/// The interruption has to be a real one: a helm that changed hands and never
/// flew anything would leave claim 2 resting on a component swap. Wide enough
/// that the directive is watched holding still over a stretch of the fight,
/// not over the frame the helm changed hands.
#[cfg(feature = "debug")]
const FREE_FLOWN_MARK: Meters = Meters(800.0);

/// How far the resumed order must carry the hull before the run reads it.
#[cfg(feature = "debug")]
const RESUMED_FLOWN_MARK: Meters = Meters(300.0);

/// In-step seconds a beat gets to reach its world condition.
///
/// A backstop that names a hung beat, not a budget: it is about three times
/// the slowest healthy beat here. The run-level deadline
/// (`NOVA_AUTOPILOT_DEADLINE`) is a separate, shorter ceiling on the whole
/// script, so which of the two names a stall depends on where the stall lands.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 120.0;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(orders_script());
    }

    app.run()
}

fn range_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
    // ONCE A FIXED STEP, after the solver: see the module comment.
    #[cfg(feature = "debug")]
    app.add_systems(
        FixedPostUpdate,
        census_the_order
            .after(avian3d::prelude::PhysicsSystems::Last)
            .run_if(resource_exists::<OrderLog>),
    );
}

fn setup_range(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    ships: Res<GameShips>,
    sections: Res<GameSections>,
) {
    commands.trigger(LoadScenario(orders_range(&game_assets, &ships, &sections)));
}

/// The route the scenario orders the gunship round, in the order it is flown.
///
/// Three marks, so the loop is four legs and "the leg it was on" is a real
/// index rather than the only one there is. The first leg runs AWAY from the
/// hostile and the second runs back past it, which is what puts the contact
/// mid-route instead of on the first frame.
fn sweep_route() -> Vec<Meters3> {
    vec![
        Meters3::new(0.0, 0.0, -1_200.0),
        Meters3::new(0.0, 0.0, 6_000.0),
        Meters3::new(-2_500.0, 0.0, 0.0),
    ]
}

/// Every weapon section on `hull`, given a hard magazine of nothing.
///
/// Read off the hull's own section list rather than written out by id, so a
/// re-armed catalog ship arrives here dry as well.
fn dry_magazines(hull: &ShipHull, sections: &GameSections) -> Vec<ShipSectionModification> {
    hull.sections
        .iter()
        .filter(|section| {
            let config = match &section.source {
                SectionSource::Inline(config) => Some(config),
                SectionSource::Prototype(id) => sections.get_section(id),
            };
            config.is_some_and(|config| {
                matches!(
                    config.kind,
                    SectionKind::Turret(_) | SectionKind::Torpedo(_) | SectionKind::Railgun(_)
                )
            })
        })
        .map(|section| ShipSectionModification {
            section: section.id.clone(),
            modifications: vec![SectionModification::SetAmmo(0)],
        })
        .collect()
}

/// The range: one interruptible gunship under a scenario-authored patrol
/// order, and one parked hostile off its route.
fn orders_range(
    game_assets: &GameAssets,
    ships: &GameShips,
    sections: &GameSections,
) -> ScenarioConfig {
    let ship = |id: &str,
                name: &str,
                catalog: &str,
                at: Meters3,
                rotation: Quat,
                spec: SpaceshipConfig| {
        let hull = kit::catalog_ship(ships, catalog);
        let modifications = dry_magazines(&hull, sections);
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: name.to_string(),
                position: at,
                rotation,
            },
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                hull: ShipSource::Inline(hull),
                modifications,
                ..spec
            }),
        })
    };

    // No patrol route and no leash: the built-in routine is `system_ai_patrol`'s
    // subject, and everything this ship flies before the interruption must come
    // from the ORDER. What is authored here is the scanner reach that stages
    // the contact and the policy that makes the order interruptible at all.
    let gunship = SpaceshipConfig {
        controller: SpaceshipController::AI(AIControllerConfig {
            sensor_range: Some(SENSOR_REACH),
            order_interruption: Some(AIOrderInterruption::OnHostileContact),
            ..default()
        }),
        allegiance: Some(Allegiance::Enemy),
        ..default()
    };
    // The contact is a parked hostile with nobody at the helm: it is what the
    // ordered ship breaks off for, not half of a duel.
    let contact = SpaceshipConfig {
        controller: SpaceshipController::None,
        allegiance: Some(Allegiance::Player),
        ..default()
    };
    // ...and parked NOSE-UP, out of the plane the route is flown in. A hostile
    // holding its nose on the gunship inside the threat aim range is a threat
    // whether or not anyone is at its helm, and a threatened ship jinks -
    // which is `system_ai_evade`'s subject, not this one's.
    let nose_up = Quat::from_rotation_x(core::f32::consts::FRAC_PI_2);

    // ORDER OF THE BATCH IS THE POINT: the scenario layer drains one queued
    // command at a time, applying each before the next is built, so the
    // `PatrolShip` here resolves the hull the spawn above just made. An order
    // that found nothing would only warn.
    let orders = vec![
        ship(
            GUNSHIP_ID,
            "Patrol Gunship",
            GUNSHIP,
            Meters3::ZERO,
            Quat::IDENTITY,
            gunship,
        ),
        ship(RAIDER_ID, "Contact", RAIDER, CONTACT_AT, nose_up, contact),
        EventActionConfig::PatrolShip(PatrolShipActionConfig {
            order: ORDER_KEY.to_string(),
            ship: GUNSHIP_ID.to_string(),
            waypoints: sweep_route(),
        }),
    ];

    ScenarioConfig {
        description: "An AI gunship under a scenario patrol order, and one hostile off its route."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                orders,
                ThreePointRig::around("helm orders", Meters3::ZERO, 40.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "helm_orders_range".to_string(),
            "Helm Orders Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The scripted run: stage the order, let it fly the hull, let the contact
/// take the helm, let the AI fly it, clear the sky, and watch the same leg
/// come back.
#[cfg(feature = "debug")]
fn orders_script() -> Script {
    Script::new()
        .step("load the order")
        .enter(GameStates::Loading)
        .until(order_staged())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("fly the order")
        .on_enter(open_the_log)
        .until(order_under_way())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("meet the contact")
        .on_enter(mark_the_order)
        .until(order_interrupted())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let the gunship fight")
        .on_enter(mark_the_interrupt)
        .until(fight_under_way())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("clear the contact")
        .on_enter(end_the_contact)
        .until(order_resumed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("fly the resumed leg")
        .on_enter(mark_the_resume)
        .until(resumed_leg_flown())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("read the order")
        .on_enter(read_the_order)
        .add()
}

/// The scenario object with this id, off a borrowed world.
#[cfg(feature = "debug")]
fn staged(world: &World, id: &str) -> Option<Entity> {
    let mut objects = world.try_query::<(Entity, &EntityId)>()?;
    objects
        .iter(world)
        .find(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
}

/// Both hulls are up and weighed, and the scenario's order is installed on the
/// gunship with the helm in its hands.
///
/// A root avian has not measured yet has no centre of mass and no hull radius,
/// so every distance this range reads would be taken from the build origin
/// instead.
#[cfg(feature = "debug")]
fn order_staged() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let weighed = |id: &str| {
            staged(world, id).is_some_and(|root| {
                world.get::<HullRadius>(root).is_some()
                    && world
                        .get::<avian3d::prelude::ComputedCenterOfMass>(root)
                        .is_some()
            })
        };
        weighed(GUNSHIP_ID)
            && weighed(RAIDER_ID)
            && staged(world, GUNSHIP_ID).is_some_and(|gunship| {
                world.get::<ShipHelmOrder>(gunship).is_some()
                    && world.get::<ShipOrderHelmAuthority>(gunship).is_some()
                    && world.get::<AIOrderInterruption>(gunship).is_some()
                    && world.get::<AISpaceshipMarker>(gunship).is_some()
            })
    })
}

/// The route and leg a patrol directive is carrying, in engine units.
#[cfg(feature = "debug")]
fn patrol_of(order: &ShipHelmOrder) -> Option<(Vec<Vec3>, usize)> {
    match &order.directive {
        ShipOrderDirective::Patrol { waypoints, leg } => Some((waypoints.clone(), *leg)),
        _ => None,
    }
}

/// Where a GOTO leg is aimed, if what is engaged is a GOTO leg at all.
#[cfg(feature = "debug")]
fn goal_of(autopilot: Option<&Autopilot>) -> Option<Vec3> {
    match autopilot.map(|autopilot| autopilot.action) {
        Some(AutopilotAction::GotoPos { position }) => Some(position),
        _ => None,
    }
}

/// The ORDER owns the helm and has a GOTO engaged on the mark its own
/// directive names.
#[cfg(feature = "debug")]
fn flying_its_leg(world: &World) -> bool {
    let Some(gunship) = staged(world, GUNSHIP_ID) else {
        return false;
    };
    let Some((waypoints, leg)) = world.get::<ShipHelmOrder>(gunship).and_then(patrol_of) else {
        return false;
    };
    if waypoints.is_empty() {
        return false;
    }
    world.get::<ShipOrderHelmAuthority>(gunship).is_some()
        && goal_of(world.get::<Autopilot>(gunship)) == Some(waypoints[leg % waypoints.len()])
}

/// The order is really flying the hull: it holds the helm, it is on a leg past
/// the first, and it has carried the ship a real distance.
#[cfg(feature = "debug")]
fn order_under_way() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let flown = world
            .get_resource::<OrderLog>()
            .is_some_and(|log| log.order_flown >= ORDER_FLOWN_MARK);
        let past_the_first_leg = staged(world, GUNSHIP_ID)
            .and_then(|gunship| world.get::<ShipHelmOrder>(gunship))
            .and_then(patrol_of)
            .is_some_and(|(_, leg)| leg >= 1);
        flown && past_the_first_leg && flying_its_leg(world)
    })
}

/// The ship's own judgement has the helm: the order is interrupted, the
/// authority is gone, and the combat driver is flying the hull at the contact.
#[cfg(feature = "debug")]
fn order_interrupted() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some(gunship) = staged(world, GUNSHIP_ID) else {
            return false;
        };
        world.get::<AIOrderInterrupted>(gunship).is_some()
            && world.get::<ShipOrderHelmAuthority>(gunship).is_none()
            && world.get::<AIBehaviorState>(gunship) == Some(&AIBehaviorState::Engage)
            && matches!(
                world
                    .get::<Autopilot>(gunship)
                    .map(|autopilot| autopilot.action),
                Some(AutopilotAction::MatchVelocity { .. })
            )
            && world
                .get::<AITarget>(gunship)
                .is_some_and(|target| **target == staged(world, RAIDER_ID))
    })
}

/// The AI has flown the hull a real distance on the helm it took.
#[cfg(feature = "debug")]
fn fight_under_way() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .get_resource::<OrderLog>()
            .is_some_and(|log| log.free_flown >= FREE_FLOWN_MARK)
    })
}

/// The order has its helm back, with nothing hostile left to hold, and is
/// flying the mark its directive still names.
#[cfg(feature = "debug")]
fn order_resumed() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some(gunship) = staged(world, GUNSHIP_ID) else {
            return false;
        };
        world.get::<AIOrderInterrupted>(gunship).is_none()
            && world
                .get::<AITarget>(gunship)
                .is_some_and(|target| target.is_none())
            && flying_its_leg(world)
    })
}

/// The resumed order has carried the hull on, under its own helm.
#[cfg(feature = "debug")]
fn resumed_leg_flown() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some(log) = world.get_resource::<OrderLog>() else {
            return false;
        };
        let Some(resumed) = log.marks.resumed.as_ref() else {
            return false;
        };
        log.order_flown >= resumed.order_flown + RESUMED_FLOWN_MARK && flying_its_leg(world)
    })
}

/// Open the census on the staged order.
#[cfg(feature = "debug")]
fn open_the_log(world: &mut World) {
    let gunship = staged(world, GUNSHIP_ID).expect("helm orders: the gunship must be staged");
    let contact = staged(world, RAIDER_ID).expect("helm orders: the contact must be staged");
    world.insert_resource(OrderLog::new(gunship, contact));
    info!("helm orders: flying the authored order");
}

/// Snapshot the order the contact is about to take the helm from.
#[cfg(feature = "debug")]
fn mark_the_order(world: &mut World) {
    let mark = read_mark(world);
    world.resource_mut::<OrderLog>().marks.ordered = Some(mark);
}

/// Snapshot the ship the moment its own judgement has the helm.
#[cfg(feature = "debug")]
fn mark_the_interrupt(world: &mut World) {
    let mark = read_mark(world);
    world.resource_mut::<OrderLog>().marks.interrupt = Some(mark);
}

/// Snapshot the order with its helm back.
#[cfg(feature = "debug")]
fn mark_the_resume(world: &mut World) {
    let mark = read_mark(world);
    world.resource_mut::<OrderLog>().marks.resumed = Some(mark);
}

/// Take the contact out of the fight, through the production damage entry
/// point: an exact kill on its bridge.
///
/// Exact rather than overkill, for the reason `system_destruction_finale`
/// gives: `HealthApplyDamage` propagates up `ChildOf`, so an overkill would
/// drain the root's aggregate health as well and blow the hull apart instead
/// of leaving the powerless wreck the neutralize rule describes.
#[cfg(feature = "debug")]
fn end_the_contact(world: &mut World) {
    let mark = read_mark(world);
    world.resource_mut::<OrderLog>().marks.fought = Some(mark);

    let contact = world.resource::<OrderLog>().contact;
    let (bridge, health) = live_bridge(world, contact)
        .expect("helm orders: the contact must carry a live flight computer to lose");
    info!("helm orders: killing the contact's flight computer ({health} hp)");
    world.trigger(HealthApplyDamage {
        entity: bridge,
        source: None,
        amount: health,
    });
}

/// The live flight computer under `root`, with the health left in it.
#[cfg(feature = "debug")]
fn live_bridge(world: &World, root: Entity) -> Option<(Entity, f32)> {
    let mut sections =
        world.try_query_filtered::<(Entity, &ChildOf, &Health), With<ControllerSectionMarker>>()?;
    sections
        .iter(world)
        .find(|(_, ChildOf(parent), health)| *parent == root && health.current > 0.0)
        .map(|(entity, _, health)| (entity, health.current))
}

/// Everything one beat boundary says about the ship, its helm and its order.
#[cfg(feature = "debug")]
#[derive(Clone, Debug)]
struct Mark {
    /// Whether the ORDER held the helm.
    helm: bool,
    /// Whether the ship's own judgement had taken it.
    interrupted: bool,
    /// Whether the order's transient execution was installed.
    engaged: bool,
    /// The authored key the order reports under.
    key: Option<String>,
    /// The route the directive was carrying, mark for mark, in engine units.
    waypoints: Vec<Vec3>,
    /// The leg the directive was on.
    leg: usize,
    /// What the helm was flying.
    verb: &'static str,
    /// Where the engaged GOTO leg was aimed, when one was.
    goal: Option<Vec3>,
    /// Whether the ship was holding a hostile.
    target: Option<Entity>,
    /// What the ship's own judgement was doing.
    state: Option<AIBehaviorState>,
    /// How far the hull had flown under the ORDER's helm by this boundary.
    order_flown: Meters,
}

/// Read the gunship's whole state at a beat boundary.
#[cfg(feature = "debug")]
fn read_mark(world: &World) -> Mark {
    let log = world.resource::<OrderLog>();
    let gunship = log.gunship;
    let order = world.get::<ShipHelmOrder>(gunship);
    let route = order.and_then(patrol_of);
    let autopilot = world.get::<Autopilot>(gunship);
    Mark {
        helm: world.get::<ShipOrderHelmAuthority>(gunship).is_some(),
        interrupted: world.get::<AIOrderInterrupted>(gunship).is_some(),
        engaged: world.get::<ShipOrderEngaged>(gunship).is_some(),
        key: order.map(|order| order.key.clone()),
        waypoints: route
            .as_ref()
            .map(|(waypoints, _)| waypoints.clone())
            .unwrap_or_default(),
        leg: route.map_or(usize::MAX, |(_, leg)| leg),
        verb: verb_of(autopilot),
        goal: goal_of(autopilot),
        target: world.get::<AITarget>(gunship).and_then(|target| **target),
        state: world.get::<AIBehaviorState>(gunship).copied(),
        order_flown: log.order_flown,
    }
}

/// The name of whatever maneuver is engaged.
#[cfg(feature = "debug")]
fn verb_of(autopilot: Option<&Autopilot>) -> &'static str {
    match autopilot.map(|autopilot| autopilot.action) {
        None => "none",
        Some(AutopilotAction::Stop) => "Stop",
        Some(AutopilotAction::Goto { .. }) => "Goto",
        Some(AutopilotAction::GotoPos { .. }) => "GotoPos",
        Some(AutopilotAction::Orbit { .. }) => "Orbit",
        Some(AutopilotAction::MatchVelocity { .. }) => "MatchVelocity",
    }
}

/// The boundaries the script stops at, in the order it reaches them.
#[cfg(feature = "debug")]
#[derive(Default, Debug)]
struct Marks {
    /// The order flying the hull, just before the contact takes the helm.
    ordered: Option<Mark>,
    /// The ship on the combat helm.
    interrupt: Option<Mark>,
    /// The ship the instant before its target goes out of the fight.
    fought: Option<Mark>,
    /// The order, with its helm back.
    resumed: Option<Mark>,
}

/// What the order did, filled in one fixed step at a time.
#[cfg(feature = "debug")]
#[derive(Resource, Debug)]
struct OrderLog {
    /// The ordered hull.
    gunship: Entity,
    /// The hostile it breaks off for.
    contact: Entity,
    /// Fixed steps sampled.
    steps: usize,
    /// Where the hull was on the previous sampled step.
    last_at: Option<Vec3>,
    /// Whether the ORDER held the helm on the previous sampled step.
    had_helm: bool,
    /// How far the hull has flown while the ORDER held the helm.
    order_flown: Meters,
    /// How far the hull has flown while the AI held it.
    free_flown: Meters,
    /// How many times the order lost the helm, and took it back.
    helm_losses: usize,
    helm_gains: usize,
    /// Sampled steps with the order interrupted.
    interrupted_steps: usize,
    /// The directive read at the first interrupted step, and how many later
    /// interrupted steps disagreed with it.
    held_directive: Option<(Vec<Vec3>, usize)>,
    directive_drifts: usize,
    /// The beat boundaries.
    marks: Marks,
}

#[cfg(feature = "debug")]
impl OrderLog {
    fn new(gunship: Entity, contact: Entity) -> Self {
        Self {
            gunship,
            contact,
            steps: 0,
            last_at: None,
            // The census opens on a step the order already holds, so the first
            // edge this counts is a real hand-over rather than the install.
            had_helm: true,
            order_flown: Meters::ZERO,
            free_flown: Meters::ZERO,
            helm_losses: 0,
            helm_gains: 0,
            interrupted_steps: 0,
            held_directive: None,
            directive_drifts: 0,
            marks: Marks::default(),
        }
    }
}

/// Count what the order did, once a fixed step.
#[cfg(feature = "debug")]
fn census_the_order(
    mut log: ResMut<OrderLog>,
    q_gunship: Query<(
        &avian3d::prelude::Position,
        Has<ShipOrderHelmAuthority>,
        Has<AIOrderInterrupted>,
        Option<&ShipHelmOrder>,
    )>,
) {
    let Ok((position, helm, interrupted, order)) = q_gunship.get(log.gunship) else {
        return;
    };
    let at = position.0;
    log.steps += 1;

    // Which helm flew the metres is the whole point of counting them: an order
    // that "moved the ship" on distance its AI covered would prove nothing.
    if let Some(previous) = log.last_at {
        let step = Meters::from_engine((at - previous).length());
        if helm {
            log.order_flown += step;
        } else {
            log.free_flown += step;
        }
    }
    log.last_at = Some(at);

    if helm && !log.had_helm {
        log.helm_gains += 1;
    }
    if !helm && log.had_helm {
        log.helm_losses += 1;
    }
    log.had_helm = helm;

    // The DIRECTIVE has to sit still for the whole interruption: that is what
    // makes the hand-back a resume rather than a re-issue, and a mark taken at
    // each end alone could not see a directive that moved in between.
    if interrupted {
        log.interrupted_steps += 1;
        let now = order.and_then(patrol_of);
        let drifted = log
            .held_directive
            .as_ref()
            .is_some_and(|held| now.as_ref() != Some(held));
        if log.held_directive.is_none() {
            log.held_directive = now;
        } else if drifted {
            log.directive_drifts += 1;
        }
    }
}

/// Read the order: the flying order, the interruption and the resumed leg.
#[cfg(feature = "debug")]
fn read_the_order(world: &mut World) {
    let advanced = read_mark(world);
    let log = world.remove_resource::<OrderLog>().expect(
        "helm orders: the census must survive the run - it is inserted with the first step",
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    let authored: Vec<Vec3> = sweep_route().iter().map(|mark| mark.to_engine()).collect();

    let ordered = log
        .marks
        .ordered
        .as_ref()
        .expect("helm orders: the flying order must have been marked");
    let interrupt = log
        .marks
        .interrupt
        .as_ref()
        .expect("helm orders: the interruption must have been marked");
    let fought = log
        .marks
        .fought
        .as_ref()
        .expect("helm orders: the fight must have been marked");
    let resumed = log
        .marks
        .resumed
        .as_ref()
        .expect("helm orders: the resumed order must have been marked");

    // Claim 1: the scenario's order holds the helm and flies the hull.
    assert!(
        ordered.helm,
        "helm orders: the authored order must hold the helm while it is being flown"
    );
    assert!(
        ordered.engaged,
        "helm orders: a flying order must have its transient execution installed"
    );
    assert!(
        !ordered.interrupted,
        "helm orders: an order flying the hull must not be interrupted"
    );
    assert_eq!(
        ordered.key.as_deref(),
        Some(ORDER_KEY),
        "helm orders: the installed order must be the one the scenario authored"
    );
    assert_eq!(
        ordered.waypoints, authored,
        "helm orders: the flown route must be the one the scenario authored"
    );
    assert_eq!(
        ordered.verb, "GotoPos",
        "helm orders: an order leg is a GOTO on the order driver's helm, not {}",
        ordered.verb
    );
    assert_eq!(
        ordered.goal,
        authored.get(ordered.leg % authored.len()).copied(),
        "helm orders: the engaged leg must be aimed at the mark the directive names"
    );
    assert!(
        ordered.leg >= 1,
        "helm orders: the order must have flown a whole leg before the contact, not leg {}",
        ordered.leg
    );
    assert!(
        ordered.order_flown >= ORDER_FLOWN_MARK,
        "helm orders: the order must physically move the hull ({} m covered under its helm, {} m \
         wanted)",
        ordered.order_flown.get(),
        ORDER_FLOWN_MARK.get()
    );
    nova_probe::probe_marker(
        world,
        "outcome: a scenario order takes the helm and flies the hull",
        serde_json::json!({
            "t": elapsed,
            "key": ordered.key,
            "leg": ordered.leg,
            "marks": authored.len(),
            "order_flown_m": ordered.order_flown.get(),
            "verb": ordered.verb,
        }),
    );

    // Claim 2: a real contact takes the helm and leaves the directive alone.
    assert!(
        !interrupt.helm,
        "helm orders: the authored policy must take helm authority off the ship"
    );
    assert!(
        interrupt.interrupted,
        "helm orders: an interrupted order must be marked as interrupted"
    );
    assert_eq!(
        interrupt.target,
        Some(log.contact),
        "helm orders: the interrupted ship must be holding the contact it broke off for"
    );
    assert_eq!(
        interrupt.state,
        Some(AIBehaviorState::Engage),
        "helm orders: the ship that took its helm back must be in the fight"
    );
    assert_eq!(
        interrupt.verb, "MatchVelocity",
        "helm orders: the combat driver must own the helm once engaged, not {}",
        interrupt.verb
    );
    assert_eq!(
        interrupt.key, ordered.key,
        "helm orders: the interruption must not retire the order"
    );
    assert_eq!(
        interrupt.waypoints, ordered.waypoints,
        "helm orders: the interruption must not touch the stored route"
    );
    assert_eq!(
        interrupt.leg, ordered.leg,
        "helm orders: the interruption must not move the directive's leg"
    );
    assert!(
        log.interrupted_steps > 0 && log.directive_drifts == 0,
        "helm orders: the stored directive must not move for any of the {} interrupted steps ({} \
         disagreed)",
        log.interrupted_steps,
        log.directive_drifts
    );
    assert!(
        log.free_flown >= FREE_FLOWN_MARK,
        "helm orders: the ship must really fly its own fight ({} m covered off the order's helm, \
         {} m wanted)",
        log.free_flown.get(),
        FREE_FLOWN_MARK.get()
    );
    nova_probe::probe_marker(
        world,
        "outcome: a hostile contact takes the helm and leaves the order alone",
        serde_json::json!({
            "t": elapsed,
            "leg_before": ordered.leg,
            "leg_held": interrupt.leg,
            "verb_before": ordered.verb,
            "verb_after": interrupt.verb,
            "interrupted_steps": log.interrupted_steps,
            "directive_drifts": log.directive_drifts,
            "free_flown_m": log.free_flown.get(),
        }),
    );

    // Claim 3: the sky clear, the same order on the same leg.
    assert!(
        fought.target.is_some(),
        "helm orders: the ship must still be holding the contact when it is taken out of the fight"
    );
    assert!(
        resumed.helm,
        "helm orders: clearing the threat must give the order its helm back"
    );
    assert!(
        !resumed.interrupted,
        "helm orders: a resumed order must no longer be marked as interrupted"
    );
    assert_eq!(
        resumed.target, None,
        "helm orders: a ship out of the fight must be holding nothing"
    );
    assert_eq!(
        resumed.key, ordered.key,
        "helm orders: the resumed order must be the same order, not a re-issue"
    );
    assert_eq!(
        resumed.waypoints, authored,
        "helm orders: the resumed route must be the authored one, mark for mark"
    );
    assert_eq!(
        resumed.leg, interrupt.leg,
        "helm orders: the resumed order must pick up on the leg the fight left it on"
    );
    assert_eq!(
        resumed.goal,
        authored.get(interrupt.leg % authored.len()).copied(),
        "helm orders: the resumed leg must be aimed at the mark the stored directive names"
    );
    assert!(
        advanced.order_flown > resumed.order_flown,
        "helm orders: the resumed order must carry the hull on under its own helm"
    );
    assert_eq!(
        (log.helm_losses, log.helm_gains),
        (1, 1),
        "helm orders: the helm must change hands exactly once each way, not {} out and {} back",
        log.helm_losses,
        log.helm_gains
    );
    nova_probe::probe_marker(
        world,
        "outcome: the cleared sky hands the same leg back",
        serde_json::json!({
            "t": elapsed,
            "leg_held": interrupt.leg,
            "leg_resumed": resumed.leg,
            "helm_losses": log.helm_losses,
            "helm_gains": log.helm_gains,
            "resumed_flown_m": (advanced.order_flown - resumed.order_flown).get(),
        }),
    );

    info!(
        "helm orders: the order took its helm back on leg {} after {:.0} m of its own fight; it \
         flew {:.0} m under the order and {:.0} m under the AI",
        resumed.leg,
        log.free_flown.get(),
        log.order_flown.get(),
        log.free_flown.get(),
    );
    nova_probe::probe_marker(
        world,
        "outcome: the order log is recorded",
        serde_json::json!({
            "t": elapsed,
            "steps": log.steps,
            "order_flown_m": log.order_flown.get(),
            "free_flown_m": log.free_flown.get(),
            "helm_losses": log.helm_losses,
            "helm_gains": log.helm_gains,
            "interrupted_steps": log.interrupted_steps,
            "directive_drifts": log.directive_drifts,
            "leg_ordered": ordered.leg,
            "leg_interrupt": interrupt.leg,
            "leg_fought": fought.leg,
            "leg_resumed": resumed.leg,
            "leg_advanced": advanced.leg,
            "order_flown_at_m": {
                "ordered": ordered.order_flown.get(),
                "interrupt": interrupt.order_flown.get(),
                "fought": fought.order_flown.get(),
                "resumed": resumed.order_flown.get(),
                "advanced": advanced.order_flown.get(),
            },
            "verbs": {
                "ordered": ordered.verb,
                "interrupt": interrupt.verb,
                "fought": fought.verb,
                "resumed": resumed.verb,
                "advanced": advanced.verb,
            },
        }),
    );
}
