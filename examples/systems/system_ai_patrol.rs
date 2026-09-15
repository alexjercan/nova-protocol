//! system_ai_patrol: what happens to a patrolling AI ship's ROUTE when a
//! hostile crosses its beat, and what its territorial tether does about the
//! chase.
//!
//! One armed shipped combatant - the cleanup group's `block_picket` - flies a
//! four-mark diamond beat around its own patrol centroid, on the shipped AI
//! defaults and the production GOTO autopilot. A parked armed intruder sits
//! outside the beat, far enough out that the picket's own tether snaps before
//! the chase reaches it. The subject is the ROUTE across all of that: the
//! patrol is interrupted, the tether walks the ship home, the intruder goes
//! out of the fight, and the same stored route picks up where it left off.
//!
//! The picket flies with an EMPTY MAGAZINE, for the reason `system_ai_combat`
//! states: the subject here is flying, not gunnery, and a live gun would put a
//! second clock on the intruder's life. Its death is applied at the beat
//! boundary through the production damage entry point instead, so it lands
//! where the script says it lands.
//!
//! Sampled ONCE A FIXED STEP, not once a frame. The tether decision is taken
//! against a live pose, and on a software rasterizer one frame holds a dozen
//! fixed steps: a per-frame census would step straight over the hand-back it
//! is here to watch.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a hostile takes a flying patrol off its leg` | the picket flew a real stretch of its route, then a hostile in detection range put it on the combat helm - and the stored route survived the flip untouched |
//! | 2 | `outcome: the patrol resumes on the route it kept` | with the intruder out of the fight, the picket returns to the same waypoint list on the same leg index, re-engages a GOTO on that mark, and then turns onto the next leg |
//! | 3 | `outcome: the leash walks a dragged-out picket home` | the chase carried the picket past its leash radius, combat broke off there, the picket flew its own route back, and nothing re-entered the fight until it was inside the re-engagement band |
//! | 4 | `outcome: the beat is recorded` | RECORD: steps sampled, metres flown on the routine, the radius the tether let go at, the closest the walk home came, and the leg index at every mark |
//!
//! The harness NEVER writes `AIPatrolRoute` and never writes the picket's
//! `Autopilot`. Everything the route does here, the ship's own passive pilot
//! did.
//!
//! What this range does NOT claim, because focused tests already do: patrol
//! leg advancement, sized-body detours and the waypoint gate
//! (`nova_ship/src/input/ai/passive.rs`), target acquisition and its
//! hysteresis (`.../acquisition.rs`), the pure leash thresholds
//! (`leash_hysteresis_uses_a_reengage_band` and
//! `the_leash_breaks_off_combat_beyond_its_radius`, `.../behavior.rs`), and
//! Evade, which is `system_ai_evade`'s.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_ai_patrol --features debug
//! # look for: `ai patrol: the tether let go at ...`,
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
#[command(name = "system_ai_patrol")]
#[command(version = "1.0.0")]
#[command(
    about = "What a hostile and a territorial leash do to a flying AI patrol route. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

/// The hull under test: the cleanup group's armed picket, one nose gun.
///
/// Armed on purpose - the claims are about a combatant that can leave its
/// routine for a fight and be pulled back out of one. `block_carrier` cannot
/// stand in: it carries no weapon, so it spawns an `AINonCombatant` and never
/// engages at all.
const PICKET: &str = "block_picket";

/// The hostile that crosses the beat: the scavenger raider.
///
/// Armed, and so a hull the production neutralize rule can take out of the
/// fight by its bridge alone; an unarmed hull is never "out of the fight"
/// (`nova_gameplay/src/integrity/neutralize.rs`) and would have to be blown
/// apart instead.
const RAIDER: &str = "block_raider";

/// The picket's scenario id.
const PICKET_ID: &str = "ai_patrol_picket";

/// The intruder's scenario id.
const RAIDER_ID: &str = "ai_patrol_intruder";

/// How far each patrol mark sits from the beat's centre.
///
/// The four marks are the diamond `-Z, -X, +Z, +X` at this radius, so their
/// centroid - which is what the authored leash anchors on - is the origin
/// exactly, and every mark is the same distance from the tether.
const BEAT_RADIUS: Meters = Meters(2_000.0);

/// How far out the intruder is parked, on the beat's `-X` axis.
///
/// Two gates decide this number, both of them the engine's own. It must be
/// over the shipped 4 km detection range from the mark the picket starts on
/// (5 385 m here) so the beat is really flown before anything interrupts it,
/// and it must be far enough past the leash below that the tether snaps while
/// the picket is still closing rather than after it has settled on its
/// standoff (the shipped standoff is about 1 080 m of centre distance for
/// these two hulls, which would leave the picket at 3 920 m).
const INTRUDER_RANGE: Meters = Meters(5_000.0);

/// The picket's authored territory: combat breaks off beyond this distance
/// from the patrol centroid.
///
/// Over `BEAT_RADIUS / 0.8` so the whole route lies inside the tighter
/// re-engagement band the passive states are held to (the shipped
/// `LEASH_REENGAGE_FRACTION`), and under the intruder's range so the chase
/// crosses it.
const LEASH_RADIUS: Meters = Meters(3_200.0);

/// The fraction of the leash radius a PASSIVE ship must be inside of before it
/// may engage again - the shipped `LEASH_REENGAGE_FRACTION`, restated here
/// because it is not exported and this range asserts against it.
#[cfg(feature = "debug")]
const REENGAGE_FRACTION: f32 = 0.8;

/// How much of its route the picket must have flown before the run calls the
/// patrol established.
///
/// A third of the first 2 828 m leg: far enough that the ship is demonstrably
/// under way on a real leg rather than sitting on the mark it spawned on, and
/// well short of the 2 048 m at which the leg's own arrival gate would turn it
/// onto the next one.
#[cfg(feature = "debug")]
const PATROL_FLOWN_MARK: Meters = Meters(1_000.0);

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
        app.add_plugins(patrol_script());
    }

    app.run()
}

fn range_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
    // ONCE A FIXED STEP, after the solver: see the module comment.
    #[cfg(feature = "debug")]
    app.add_systems(
        FixedPostUpdate,
        census_the_beat
            .after(avian3d::prelude::PhysicsSystems::Last)
            .run_if(resource_exists::<Beat>),
    );
}

fn setup_range(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    ships: Res<GameShips>,
    sections: Res<GameSections>,
) {
    commands.trigger(LoadScenario(patrol_range(&game_assets, &ships, &sections)));
}

/// The beat's four marks, in the order the route flies them.
fn beat_route() -> Vec<Meters3> {
    let radius = BEAT_RADIUS.get();
    vec![
        Meters3::new(0.0, 0.0, -radius),
        Meters3::new(-radius, 0.0, 0.0),
        Meters3::new(0.0, 0.0, radius),
        Meters3::new(radius, 0.0, 0.0),
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

/// The range: one leashed picket on a four-mark beat, and one parked intruder
/// outside it.
fn patrol_range(
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

    // Everything about how the picket sees and fights is the engine's own -
    // the 4 km detection range, the 20 km scanner, the shipped standoff - so
    // the geometry above is measured against shipped numbers. The route and
    // the tether are the two authored things, because they ARE the subject.
    let picket = SpaceshipConfig {
        controller: SpaceshipController::AI(AIControllerConfig {
            patrol: beat_route(),
            leash: Some(LEASH_RADIUS),
            ..default()
        }),
        allegiance: Some(Allegiance::Enemy),
        ..default()
    };
    // The intruder is a parked hostile with nobody at the helm: it is what the
    // picket leaves its beat for, not half of a duel.
    let intruder = SpaceshipConfig {
        controller: SpaceshipController::None,
        allegiance: Some(Allegiance::Player),
        ..default()
    };
    // ...and parked NOSE-UP, out of the plane the beat is flown in. A hostile
    // holding its nose on the picket inside the threat aim range is a threat
    // whether or not anyone is at its helm, and a threatened ship jinks -
    // which is `system_ai_evade`'s subject, not this one's.
    let nose_up = Quat::from_rotation_x(core::f32::consts::FRAC_PI_2);

    ScenarioConfig {
        description:
            "A leashed AI picket flying a patrol beat, and one parked intruder outside it."
                .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    ship(
                        PICKET_ID,
                        "Picket",
                        PICKET,
                        beat_route()[0],
                        Quat::IDENTITY,
                        picket,
                    ),
                    ship(
                        RAIDER_ID,
                        "Intruder",
                        RAIDER,
                        Meters3::new(-INTRUDER_RANGE.get(), 0.0, 0.0),
                        nose_up,
                        intruder,
                    ),
                ],
                ThreePointRig::around("ai patrol", Meters3::ZERO, 40.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "ai_patrol_range".to_string(),
            "AI Patrol Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The scripted run: stage the beat, let the patrol get under way, let the
/// intruder take it, ride the tether out and back, take the intruder out of the
/// fight, and watch the route pick up.
#[cfg(feature = "debug")]
fn patrol_script() -> Script {
    Script::new()
        .step("load the beat")
        .enter(GameStates::Loading)
        .until(beat_staged())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("fly the beat")
        .on_enter(open_the_census)
        .until(patrol_under_way())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("meet the intruder")
        .on_enter(mark_the_routine)
        .until(patrol_interrupted())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("ride the tether")
        .on_enter(mark_the_interrupt)
        .until(tether_cycled())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("lose the intruder")
        .on_enter(end_the_intruder)
        .until(patrol_resumed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("turn onto the next leg")
        .on_enter(mark_the_resume)
        .until(next_leg_flown())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("read the beat")
        .on_enter(read_the_beat)
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

/// Both hulls are up and weighed, and the picket carries the route and the
/// tether the scenario authored.
///
/// A root avian has not measured yet has no centre of mass and no hull radius,
/// so every distance this range reads would be taken from the build origin
/// instead.
#[cfg(feature = "debug")]
fn beat_staged() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let weighed = |id: &str| {
            staged(world, id).is_some_and(|root| {
                world.get::<HullRadius>(root).is_some()
                    && world
                        .get::<avian3d::prelude::ComputedCenterOfMass>(root)
                        .is_some()
            })
        };
        weighed(PICKET_ID)
            && weighed(RAIDER_ID)
            && staged(world, PICKET_ID).is_some_and(|picket| {
                world.get::<AIPatrolRoute>(picket).is_some()
                    && world.get::<AILeash>(picket).is_some()
                    && world.get::<AISpaceshipMarker>(picket).is_some()
            })
    })
}

/// The patrol is really being flown: the routine is up, the passive pilot has
/// a GOTO on the current mark, and the hull has covered a real stretch of it.
#[cfg(feature = "debug")]
fn patrol_under_way() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .get_resource::<Beat>()
            .is_some_and(|beat| beat.patrol_flown >= PATROL_FLOWN_MARK)
            && on_its_leg(world)
    })
}

/// The picket is on its passive routine with a GOTO engaged on the mark its
/// own route says it is flying to.
#[cfg(feature = "debug")]
fn on_its_leg(world: &World) -> bool {
    let Some(picket) = staged(world, PICKET_ID) else {
        return false;
    };
    let Some(route) = world.get::<AIPatrolRoute>(picket) else {
        return false;
    };
    world.get::<AIBehaviorState>(picket) == Some(&AIBehaviorState::Patrol)
        && goal_of(world.get::<Autopilot>(picket)) == current_mark(route)
        && current_mark(route).is_some()
}

/// The mark a route is flying to right now, in engine units.
#[cfg(feature = "debug")]
fn current_mark(route: &AIPatrolRoute) -> Option<Vec3> {
    (!route.waypoints.is_empty()).then(|| route.waypoints[route.current % route.waypoints.len()])
}

/// Where a GOTO leg is aimed, if what is engaged is a GOTO leg at all.
#[cfg(feature = "debug")]
fn goal_of(autopilot: Option<&Autopilot>) -> Option<Vec3> {
    match autopilot.map(|autopilot| autopilot.action) {
        Some(AutopilotAction::GotoPos { position }) => Some(position),
        _ => None,
    }
}

/// The hostile has taken the picket: the behavior state is the fight and the
/// combat driver owns the helm.
#[cfg(feature = "debug")]
fn patrol_interrupted() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some(picket) = staged(world, PICKET_ID) else {
            return false;
        };
        world.get::<AIBehaviorState>(picket) == Some(&AIBehaviorState::Engage)
            && matches!(
                world
                    .get::<Autopilot>(picket)
                    .map(|autopilot| autopilot.action),
                Some(AutopilotAction::MatchVelocity { .. })
            )
            && world
                .get::<AITarget>(picket)
                .is_some_and(|target| **target == staged(world, RAIDER_ID))
    })
}

/// The tether has run its whole cycle: it let go outside the radius, and
/// something re-entered the fight afterwards.
#[cfg(feature = "debug")]
fn tether_cycled() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .get_resource::<Beat>()
            .is_some_and(|beat| beat.broke_off_at.is_some() && beat.re_engaged_at.is_some())
    })
}

/// The routine is back, on the leg the ship was holding when its target went
/// away, with nothing hostile left to hold.
#[cfg(feature = "debug")]
fn patrol_resumed() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some(picket) = staged(world, PICKET_ID) else {
            return false;
        };
        let held = world
            .get_resource::<Beat>()
            .and_then(|beat| beat.marks.lost.as_ref().map(|mark| mark.current));
        world
            .get::<AITarget>(picket)
            .is_some_and(|target| target.is_none())
            && on_its_leg(world)
            && world
                .get::<AIPatrolRoute>(picket)
                .map(|route| route.current)
                == held
    })
}

/// The loop has moved on: the route is one leg past where the interruption
/// left it, and the GOTO is on the new mark.
#[cfg(feature = "debug")]
fn next_leg_flown() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some(picket) = staged(world, PICKET_ID) else {
            return false;
        };
        let Some(route) = world.get::<AIPatrolRoute>(picket) else {
            return false;
        };
        let Some(held) = world
            .get_resource::<Beat>()
            .and_then(|beat| beat.marks.lost.as_ref().map(|mark| mark.current))
        else {
            return false;
        };
        let next = (held + 1) % route.waypoints.len().max(1);
        route.current == next && on_its_leg(world)
    })
}

/// Open the census on the staged beat.
///
/// The tether is read off the SHIP, not off the authored constant: the centre
/// is the patrol centroid the spawn derived, and reading it back is what makes
/// every distance below a distance from the tether the picket actually has.
#[cfg(feature = "debug")]
fn open_the_census(world: &mut World) {
    let picket = staged(world, PICKET_ID).expect("ai patrol: the picket must be staged");
    let intruder = staged(world, RAIDER_ID).expect("ai patrol: the intruder must be staged");
    let (centre, radius) = world
        .get::<AILeash>(picket)
        .map(|leash| (leash.center, leash.radius))
        .expect("ai patrol: the picket must carry the authored tether");
    world.insert_resource(Beat::new(picket, intruder, centre, radius));
    info!("ai patrol: flying the beat");
}

/// Snapshot the routine the interruption is about to land on.
#[cfg(feature = "debug")]
fn mark_the_routine(world: &mut World) {
    let mark = read_mark(world);
    world.resource_mut::<Beat>().marks.routine = Some(mark);
}

/// Snapshot the ship the moment the hostile has it.
#[cfg(feature = "debug")]
fn mark_the_interrupt(world: &mut World) {
    let mark = read_mark(world);
    world.resource_mut::<Beat>().marks.interrupt = Some(mark);
}

/// Snapshot the resumed routine.
#[cfg(feature = "debug")]
fn mark_the_resume(world: &mut World) {
    let mark = read_mark(world);
    world.resource_mut::<Beat>().marks.resumed = Some(mark);
}

/// Take the intruder out of the fight, through the production damage entry
/// point: an exact kill on its bridge.
///
/// Exact rather than overkill, for the reason `system_destruction_finale`
/// gives: `HealthApplyDamage` propagates up `ChildOf`, so an overkill would
/// drain the root's aggregate health as well and blow the hull apart instead
/// of leaving the powerless wreck the neutralize rule describes.
#[cfg(feature = "debug")]
fn end_the_intruder(world: &mut World) {
    let mark = read_mark(world);
    world.resource_mut::<Beat>().marks.lost = Some(mark);

    let intruder = world.resource::<Beat>().intruder;
    let (bridge, health) = live_bridge(world, intruder)
        .expect("ai patrol: the intruder must carry a live flight computer to lose");
    info!("ai patrol: killing the intruder's flight computer ({health} hp)");
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

/// Everything one beat boundary says about the ship and its route.
#[cfg(feature = "debug")]
#[derive(Clone, Debug)]
struct Mark {
    /// What the ship was doing.
    state: Option<AIBehaviorState>,
    /// The route it was carrying, mark for mark, in engine units.
    waypoints: Vec<Vec3>,
    /// The leg index the route was on.
    current: usize,
    /// What the helm was flying.
    verb: &'static str,
    /// Where the engaged GOTO leg was aimed, when one was.
    goal: Option<Vec3>,
    /// Whether the ship was holding a hostile.
    target: Option<Entity>,
    /// How far the ship was from its own tether's anchor.
    from_centre: Meters,
    /// How far the ship had flown on its passive routine by this boundary.
    flown: Meters,
}

/// Read the picket's whole state at a beat boundary.
#[cfg(feature = "debug")]
fn read_mark(world: &World) -> Mark {
    let beat = world.resource::<Beat>();
    let picket = beat.picket;
    let route = world.get::<AIPatrolRoute>(picket);
    let autopilot = world.get::<Autopilot>(picket);
    let at = world
        .get::<avian3d::prelude::Position>(picket)
        .map_or(Vec3::ZERO, |position| position.0);
    Mark {
        state: world.get::<AIBehaviorState>(picket).copied(),
        waypoints: route
            .map(|route| route.waypoints.clone())
            .unwrap_or_default(),
        current: route.map_or(usize::MAX, |route| route.current),
        verb: verb_of(autopilot),
        goal: goal_of(autopilot),
        target: world.get::<AITarget>(picket).and_then(|target| **target),
        from_centre: Meters::from_engine((at - beat.centre).length()),
        flown: beat.patrol_flown,
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
    /// The flying patrol, just before the hostile takes it.
    routine: Option<Mark>,
    /// The ship on the combat helm.
    interrupt: Option<Mark>,
    /// The ship the instant before its target goes out of the fight.
    lost: Option<Mark>,
    /// The routine, back.
    resumed: Option<Mark>,
}

/// What the beat looked like, filled in one fixed step at a time.
#[cfg(feature = "debug")]
#[derive(Resource, Debug)]
struct Beat {
    /// The hull under test.
    picket: Entity,
    /// The hostile it leaves its beat for.
    intruder: Entity,
    /// The tether's anchor, engine units.
    centre: Vec3,
    /// The tether's radius.
    radius: Meters,
    /// Fixed steps sampled.
    steps: usize,
    /// How far the hull has flown while on its passive routine.
    patrol_flown: Meters,
    /// Where the hull was on the previous sampled step.
    last_at: Option<Vec3>,
    /// Whether the previous sampled step had the hull in a combat state.
    was_fighting: bool,
    /// The furthest from the anchor the hull ever got while fighting.
    fought_out_to: Meters,
    /// How far out the tether let go: the distance at the first step that
    /// handed a fighting hull back to its routine beyond the radius.
    broke_off_at: Option<Meters>,
    /// The closest the walk home came to the anchor, between the tether
    /// letting go and anything re-entering the fight.
    walked_home_to: Meters,
    /// Sampled steps of that same stretch, and how many of them held a GOTO on
    /// the hull's own current mark.
    homeward_steps: usize,
    homeward_on_route: usize,
    /// How far out the hull was when it re-entered the fight.
    re_engaged_at: Option<Meters>,
    /// The beat boundaries.
    marks: Marks,
}

#[cfg(feature = "debug")]
impl Beat {
    fn new(picket: Entity, intruder: Entity, centre: Vec3, radius: f32) -> Self {
        Self {
            picket,
            intruder,
            centre,
            radius: Meters::from_engine(radius),
            steps: 0,
            patrol_flown: Meters::ZERO,
            last_at: None,
            was_fighting: false,
            fought_out_to: Meters::ZERO,
            broke_off_at: None,
            walked_home_to: Meters(f32::MAX),
            homeward_steps: 0,
            homeward_on_route: 0,
            re_engaged_at: None,
            marks: Marks::default(),
        }
    }

    /// The band a PASSIVE hull has to be back inside before it may engage.
    fn band(&self) -> Meters {
        self.radius * REENGAGE_FRACTION
    }
}

/// Count what the beat did, once a fixed step.
#[cfg(feature = "debug")]
fn census_the_beat(
    mut beat: ResMut<Beat>,
    q_picket: Query<(
        &avian3d::prelude::Position,
        &AIBehaviorState,
        Option<&Autopilot>,
        Option<&AIPatrolRoute>,
    )>,
) {
    let Ok((position, state, autopilot, route)) = q_picket.get(beat.picket) else {
        return;
    };
    let at = position.0;
    let from_centre = Meters::from_engine((at - beat.centre).length());
    beat.steps += 1;

    // The passive routine is the only thing this counts as route flying: the
    // chase is not patrol distance, and calling it that would let a long
    // enough fight satisfy the "under way" gate on its own.
    let fighting = !matches!(
        state,
        AIBehaviorState::Idle | AIBehaviorState::Patrol | AIBehaviorState::Orbit
    );
    if let Some(previous) = beat.last_at {
        if !fighting {
            beat.patrol_flown += Meters::from_engine((at - previous).length());
        }
    }
    beat.last_at = Some(at);

    if fighting {
        beat.fought_out_to = beat.fought_out_to.max(from_centre);
    }
    // The tether letting go: a fighting hull handed back to its routine while
    // outside the radius. Recorded once - the cycle after it is the beat's.
    if beat.broke_off_at.is_none() && beat.was_fighting && !fighting && from_centre > beat.radius {
        beat.broke_off_at = Some(from_centre);
    }
    // The walk home is the stretch the TETHER owns: from the hand-back to
    // whatever re-enters the fight. What the ship does after that is the
    // routine's again, and counting it here would read a later patrol leg as
    // part of the walk.
    if beat.broke_off_at.is_some() && beat.re_engaged_at.is_none() {
        if fighting {
            beat.re_engaged_at = Some(from_centre);
        } else {
            beat.walked_home_to = beat.walked_home_to.min(from_centre);
            beat.homeward_steps += 1;
            let on_route = route
                .and_then(current_mark)
                .is_some_and(|mark| goal_of(autopilot) == Some(mark));
            if on_route {
                beat.homeward_on_route += 1;
            }
        }
    }
    beat.was_fighting = fighting;
}

/// Read the beat: the interruption, the tether cycle and the resumed route.
#[cfg(feature = "debug")]
fn read_the_beat(world: &mut World) {
    let advanced = read_mark(world);
    let beat = world
        .remove_resource::<Beat>()
        .expect("ai patrol: the census must survive the beat - it is inserted with the first step");
    let elapsed = world.resource::<Time>().elapsed_secs();
    let authored: Vec<Vec3> = beat_route().iter().map(|mark| mark.to_engine()).collect();

    let routine = beat
        .marks
        .routine
        .as_ref()
        .expect("ai patrol: the flying routine must have been marked");
    let interrupt = beat
        .marks
        .interrupt
        .as_ref()
        .expect("ai patrol: the interruption must have been marked");
    let lost = beat
        .marks
        .lost
        .as_ref()
        .expect("ai patrol: the lost target must have been marked");
    let resumed = beat
        .marks
        .resumed
        .as_ref()
        .expect("ai patrol: the resumed routine must have been marked");

    // Claim 1: a flown patrol, taken off its leg by a hostile, with the route
    // left exactly as it was.
    assert_eq!(
        routine.state,
        Some(AIBehaviorState::Patrol),
        "ai patrol: the ship must be on its patrol routine before the interruption"
    );
    assert_eq!(
        routine.waypoints, authored,
        "ai patrol: the flown route must be the one the scenario authored"
    );
    assert_eq!(
        routine.verb, "GotoPos",
        "ai patrol: a flying patrol leg is a GOTO on the passive pilot's helm, not {}",
        routine.verb
    );
    assert!(
        routine.flown >= PATROL_FLOWN_MARK,
        "ai patrol: the patrol must be really flown before it is interrupted ({} m covered, {} m \
         wanted)",
        routine.flown.get(),
        PATROL_FLOWN_MARK.get()
    );
    assert_eq!(
        interrupt.state,
        Some(AIBehaviorState::Engage),
        "ai patrol: a hostile in detection range must take the ship off its routine"
    );
    assert_eq!(
        interrupt.target,
        Some(beat.intruder),
        "ai patrol: the interrupted ship must be holding the intruder"
    );
    assert_eq!(
        interrupt.verb, "MatchVelocity",
        "ai patrol: the combat driver must own the helm once engaged, not {}",
        interrupt.verb
    );
    assert_eq!(
        interrupt.waypoints, routine.waypoints,
        "ai patrol: the interruption must not touch the stored route"
    );
    assert_eq!(
        interrupt.current, routine.current,
        "ai patrol: the interruption must not move the route's leg index"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a hostile takes a flying patrol off its leg",
        serde_json::json!({
            "t": elapsed,
            "flown_m": routine.flown.get(),
            "leg": routine.current,
            "verb_before": routine.verb,
            "verb_after": interrupt.verb,
        }),
    );

    // Claim 2: the target gone, the same route on the same leg.
    assert!(
        lost.target.is_some(),
        "ai patrol: the ship must still be holding the intruder when it is taken out of the fight"
    );
    assert_eq!(
        resumed.state,
        Some(AIBehaviorState::Patrol),
        "ai patrol: losing the target must hand the ship back to its routine"
    );
    assert_eq!(
        resumed.target, None,
        "ai patrol: a ship out of the fight must be holding nothing"
    );
    assert_eq!(
        resumed.waypoints, authored,
        "ai patrol: the resumed route must be the authored one, mark for mark"
    );
    assert_eq!(
        resumed.current, lost.current,
        "ai patrol: the resumed route must pick up on the leg the fight left it on"
    );
    assert_eq!(
        resumed.goal,
        authored.get(lost.current % authored.len()).copied(),
        "ai patrol: the resumed GOTO must be aimed at the mark the stored route names"
    );
    let next = (lost.current + 1) % authored.len();
    assert_eq!(
        advanced.current, next,
        "ai patrol: the resumed patrol must go on to turn onto its next leg"
    );
    assert_eq!(
        advanced.goal,
        authored.get(next).copied(),
        "ai patrol: the next leg's GOTO must be aimed at the next mark"
    );
    assert_eq!(
        advanced.waypoints, authored,
        "ai patrol: turning onto the next leg must not rewrite the route"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the patrol resumes on the route it kept",
        serde_json::json!({
            "t": elapsed,
            "leg_held": lost.current,
            "leg_resumed": resumed.current,
            "leg_next": advanced.current,
            "marks": authored.len(),
        }),
    );

    // Claim 3: dragged out, let go, and walked home.
    let broke_off = beat
        .broke_off_at
        .expect("ai patrol: the tether must have let go - the beat waited for it");
    let re_engaged = beat
        .re_engaged_at
        .expect("ai patrol: the ship must have re-entered the fight - the beat waited for it");
    assert!(
        broke_off > beat.radius,
        "ai patrol: combat must break off outside the tether, not at {} m inside a {} m radius",
        broke_off.get(),
        beat.radius.get()
    );
    assert!(
        beat.homeward_steps > 0 && beat.homeward_on_route == beat.homeward_steps,
        "ai patrol: the ship must fly its own route home, not drift ({} of {} homeward steps held \
         a GOTO on its current mark)",
        beat.homeward_on_route,
        beat.homeward_steps
    );
    assert!(
        re_engaged <= beat.band(),
        "ai patrol: nothing may re-enter the fight until the ship is back inside the band ({} m of \
         a {} m band)",
        re_engaged.get(),
        beat.band().get()
    );
    nova_probe::probe_marker(
        world,
        "outcome: the leash walks a dragged-out picket home",
        serde_json::json!({
            "t": elapsed,
            "leash_radius_m": beat.radius.get(),
            "band_m": beat.band().get(),
            "broke_off_m": broke_off.get(),
            "walked_home_to_m": beat.walked_home_to.get(),
            "re_engaged_m": re_engaged.get(),
        }),
    );

    info!(
        "ai patrol: the tether let go at {:.0} m and the walk home reached {:.0} m of a {:.0} m \
         band; the route held leg {} across the fight",
        broke_off.get(),
        beat.walked_home_to.get(),
        beat.band().get(),
        lost.current,
    );
    nova_probe::probe_marker(
        world,
        "outcome: the beat is recorded",
        serde_json::json!({
            "t": elapsed,
            "steps": beat.steps,
            "routine_flown_m": beat.patrol_flown.get(),
            "fought_out_to_m": beat.fought_out_to.get(),
            "broke_off_m": broke_off.get(),
            "walked_home_to_m": beat.walked_home_to.get(),
            "re_engaged_m": re_engaged.get(),
            "homeward_steps": beat.homeward_steps,
            "homeward_on_route": beat.homeward_on_route,
            "leg_routine": routine.current,
            "leg_interrupt": interrupt.current,
            "leg_lost": lost.current,
            "leg_resumed": resumed.current,
            "leg_next": advanced.current,
            "from_centre_m": {
                "routine": routine.from_centre.get(),
                "interrupt": interrupt.from_centre.get(),
                "lost": lost.from_centre.get(),
                "resumed": resumed.from_centre.get(),
            },
            "flown_m": {
                "routine": routine.flown.get(),
                "interrupt": interrupt.flown.get(),
                "lost": lost.flown.get(),
                "resumed": resumed.flown.get(),
            },
        }),
    );
}
