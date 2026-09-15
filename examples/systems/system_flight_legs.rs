//! system_flight_legs: the production autopilot flies a whole composed errand,
//! on the fleet's smallest hull and on its largest.
//!
//! One lane per hull, flown twice: a STOP, a GOTO at a beacon, a GotoPos at a
//! mark and an ORBIT around a well, engaged one after another through the
//! production [`Autopilot`] seam and each one waited on by the condition the
//! game itself reports. The subject is the CHAIN - that the computer hands one
//! leg to the next on a real hull with real sections, and that the hull's size
//! does not decide whether it can - plus the two seams a chain is made of: the
//! coast an arrival plan flies before it commits to the brake, and a new action
//! taking the helm off a live one.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the composed leg chain completes on both hulls` | each hull reported STOP, GOTO and GotoPos complete through the production completion seam, moved inward past each mark to do it, and reached station-keeping on a ring the well's own stable band accepts |
//! | 2 | `outcome: an unobstructed goto coasts before it brakes` | each hull's staged GOTO published a coast with its flip still ahead, gave way to the brake exactly once, and never went back to coasting |
//! | 3 | `outcome: a replacement action owns the helm on the next flight tick` | a second action written over a live one is flown by the very next fixed step, the helm is never handed back in between, and the leg it replaced reports no completion |
//! | 4 | `outcome: the flown lanes are recorded` | RECORD: hull radius, the end state of every leg, the coast/brake split and the replacement window, per hull |
//!
//! Sampled ONCE A FIXED STEP for claims 2 and 3. `autopilot_system` runs in
//! `FixedUpdate`, and on a software rasterizer one rendered frame holds a dozen
//! fixed steps: a per-frame census could not see "the NEXT flight tick" at all,
//! and would read a continuous coast-to-brake handover as a jump.
//!
//! What this range does NOT claim, because focused tests already own it:
//! standalone arrival geometry, the park radius a sized or well-bearing target
//! gets, orbit laps, and orbit disengagement
//! (`crates/nova_ship/src/flight/tests/`). Claim 3 is a REPLACEMENT, not an
//! interruption: nothing here is a helm order, and the order layer's
//! interrupt-and-resume machine belongs to `system_helm_orders`.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=600 \
//!   cargo run --example system_flight_legs --features debug
//! # look for: `flight legs: block_skiff flew the chain ...`,
//! #           `flight legs: block_carrier flew the chain ...`,
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
#[command(name = "system_flight_legs")]
#[command(version = "1.0.0")]
#[command(
    about = "One composed STOP/GOTO/GotoPos/ORBIT chain on the smallest and the largest shipped hull. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

/// The scenario id every lane's hull is spawned under: one id, so a beat never
/// has to know which hull is up.
const HULL_ID: &str = "flight_legs_hull";

/// The GOTO leg's target. A beacon is the cheapest shipped thing that is
/// lockable, sized and static, and it carries no well - so the arrival is a
/// plain standoff park rather than the well handoff `arrival.rs` already owns.
const BEACON_ID: &str = "flight_legs_beacon";

/// The ORBIT leg's well.
const WELL_ID: &str = "flight_legs_well";

/// The orbited well's published body radius and strength.
///
/// The sanity well the flight tests plan rings around (`mu` 1200 over a 200 m
/// body): its sphere of influence is 600 m and its stable ring band is
/// 315-530 m, so gravity in this range is a 600 m bubble at the end of the lane
/// and every leg outside it is flown in clean space. A stronger well would buy
/// this range nothing and cost every leg an arrival budget.
const WELL_RADIUS: Meters = Meters(200.0);
const WELL_MU: f32 = 1_200.0;

/// The beacon's own body radius: small, so the GOTO park point is the shipped
/// margin plus the flying hull rather than the target's size.
const BEACON_RADIUS: Meters = Meters(60.0);

/// Where the lane's pieces sit on the +X line out of the well.
///
/// The hull starts furthest out and works inward, so every leg after the first
/// begins where the previous one parked and the chain needs no repositioning
/// between beats. Short, because length is not the subject: at the shipped
/// 500 m margin each leg is still several times the park envelope it has to
/// find, which is what makes a coast and a brake out of it, and the carrier
/// pays for every extra metre in software-rendered frames.
const HULL_START: Meters = Meters(2_600.0);
const BEACON_AT: Meters = Meters(1_000.0);

/// The mark the replaced leg is aimed at: further OUT than the hull starts, so
/// the action taken off the helm was flying away from every mark the chain then
/// uses and cannot have helped it reach one.
#[cfg(feature = "debug")]
const OUTBOUND_MARK: Meters = Meters(12_000.0);

/// The closing speed the replaced leg must reach before it is replaced.
///
/// A LIVE leg, not a freshly engaged one: below this the hull is still swinging
/// its nose onto the burn, and the replacement would be taking the helm off an
/// action that had not yet used it.
#[cfg(feature = "debug")]
const REPLACE_SPEED: MetersPerSecond = MetersPerSecond(30.0);

/// Fixed steps the replacement window is sampled over.
///
/// Three, not one: the claim is that the new action is carried on the NEXT
/// flight tick AND keeps the helm, and one sample cannot tell a handover from
/// a flicker.
#[cfg(feature = "debug")]
const SWAP_STEPS: usize = 3;

/// How many of the shipped rest epsilons a completed STOP may still carry.
///
/// The flight layer declares rest at [`FlightSettings::stop_speed_epsilon`] and
/// then lets the actuators wind down, so the speed this range reads a beat
/// later is that epsilon plus a spool tail - never a multiple of it.
#[cfg(feature = "debug")]
const REST_EPSILONS: f32 = 2.5;

/// How much of the shipped arrival margin a parked leg may sit outside it.
///
/// The leg completes INSIDE the envelope and then coasts to rest, so the gap
/// this range measures afterwards is the margin plus that creep.
#[cfg(feature = "debug")]
const PARK_SLACK: f32 = 1.25;

/// In-step seconds a load beat gets. A HANG detector, not a budget: measured
/// at 2.3 s for the first load and 0.5 s for the reload on a software
/// rasterizer.
#[cfg(feature = "debug")]
const LOAD_DEADLINE_SECS: f32 = 30.0;

/// In-step seconds a flight beat gets, in the REAL seconds a deadline counts -
/// not the sim seconds a leg is measured in. Under a software rasterizer a
/// frame of a shipped hull costs real time and `Time<Virtual>`'s clamp then
/// runs the world slower than the wall, so a bound on a leg has to be several
/// times its sim length.
///
/// A HANG detector sized at about three times the slowest healthy leg: the
/// carrier's inbound GOTO and its run to the mark, each 25 s under
/// [`WALK_MAX_DELTA`] on this software rasterizer. Without that hold they were
/// past this bound and the clamp, not the flight computer, was what a stall
/// reported. The SUM of every beat's deadline is far past any run-level
/// deadline, so which of the two names a stall depends on where the stall
/// lands - late enough in the walk and the run collector wins. That is the
/// price of per-beat bounds loose enough to survive a slower runner, and the
/// run's own log names the beat it was in either way.
#[cfg(feature = "debug")]
const LEG_DEADLINE_SECS: f32 = 90.0;

/// How far the game clock may advance in one frame of a harnessed lane.
///
/// Bevy clamps `Time<Virtual>` to a quarter second a frame. The carrier lane
/// flies the largest hull the game ships, and on a software rasterizer one of
/// its frames costs seconds - the draw count, not the pixels - so the clamp ran
/// the lane at a fraction of wall speed and the carrier's GOTO outran both its
/// own beat deadline and the run's. Nothing in this range reads a FRAME: both
/// censuses sample in `FixedPostUpdate`, so a frame that carries more world
/// time runs more of the fixed steps they watch, not fewer, and the flight
/// computer integrates the same steps either way.
///
/// A harnessed lane only: an interactive run has a viewer, and a two-second
/// clock step is a stutter to one.
#[cfg(feature = "debug")]
const WALK_MAX_DELTA: std::time::Duration = std::time::Duration::from_secs(2);

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

/// Which hull is flying the lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Subject {
    /// The fleet's smallest hull.
    Skiff,
    /// The largest hull the base game ships. Only the scripted walk flies it,
    /// so the variant is gated with the script that constructs it.
    #[cfg(feature = "debug")]
    Carrier,
}

impl Subject {
    /// The catalog ship id this subject spawns.
    fn ship(self) -> &'static str {
        match self {
            Subject::Skiff => "block_skiff",
            #[cfg(feature = "debug")]
            Subject::Carrier => "block_carrier",
        }
    }

    /// The scenario id this subject's lane loads under, so the run log says
    /// which hull is up.
    fn scenario(self) -> &'static str {
        match self {
            Subject::Skiff => "flight_legs_skiff",
            #[cfg(feature = "debug")]
            Subject::Carrier => "flight_legs_carrier",
        }
    }
}

/// Which leg of the chain a completion belongs to.
///
/// ORBIT is deliberately absent: an orbit is not a destination and never
/// self-completes, so the chain's last leg is read off the station-keeping
/// phase the scenario layer reads instead.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Leg {
    Stop,
    Goto,
    GotoPos,
}

/// An engaged action WITHOUT its payload, so a census can say which verb owns
/// the helm without carrying a target entity around.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verb {
    Stop,
    Goto,
    GotoPos,
    Orbit,
    MatchVelocity,
}

#[cfg(feature = "debug")]
impl Verb {
    fn of(action: &AutopilotAction) -> Self {
        match action {
            AutopilotAction::Stop => Verb::Stop,
            AutopilotAction::Goto { .. } => Verb::Goto,
            AutopilotAction::GotoPos { .. } => Verb::GotoPos,
            AutopilotAction::Orbit { .. } => Verb::Orbit,
            AutopilotAction::MatchVelocity { .. } => Verb::MatchVelocity,
        }
    }
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(legs_script());
    }

    app.run()
}

fn range_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
    #[cfg(feature = "debug")]
    {
        app.init_resource::<Legs>();
        app.add_observer(note_the_hull);
        app.add_observer(note_a_completion);
        // ONCE A FIXED STEP, after the solver. Both censuses read decisions
        // `autopilot_system` takes in `FixedUpdate`, and a rendered frame on a
        // software rasterizer holds a dozen of those.
        app.add_systems(
            FixedPostUpdate,
            (census_the_goto, census_the_swap)
                .after(avian3d::prelude::PhysicsSystems::Last)
                .run_if(resource_exists::<Legs>),
        );
        if harness_env_active() {
            app.add_systems(First, hold_the_lane_clock.before(bevy::time::TimeSystems));
        }
    }
}

/// Held every frame, not set once: a scenario load hands `Time<Virtual>` back
/// at its default, and this walk loads a lane twice.
#[cfg(feature = "debug")]
fn hold_the_lane_clock(mut time: ResMut<Time<Virtual>>) {
    if time.max_delta() != WALK_MAX_DELTA {
        time.set_max_delta(WALK_MAX_DELTA);
    }
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    commands.trigger(LoadScenario(lane(&game_assets, &ships, Subject::Skiff)));
}

/// One lane: the well at the origin, the beacon out along +X, and the subject
/// hull further out still, piloted.
///
/// Piloted rather than uncontrolled because [`PlayerAutopilotCompleted`] - the
/// production seam a scenario reads a finished errand off - is published for
/// the player's ship. ONE hull per lane for the same reason: two player ships
/// in one world is not a shape the game ships.
fn lane(game_assets: &GameAssets, ships: &GameShipDesigns, subject: Subject) -> ScenarioConfig {
    let well = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: WELL_ID.to_string(),
            name: "Orbit Well".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Anchor(AnchorConfig {
            body_radius: WELL_RADIUS,
            mass: Some(WELL_MU),
        }),
    });

    let beacon = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: BEACON_ID.to_string(),
            name: "Leg Beacon".to_string(),
            position: Meters3::new(BEACON_AT.get(), 0.0, 0.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: "LEG".to_string(),
            radius: BEACON_RADIUS,
            color: Color::WHITE,
            area_radius: None,
            lock_signature: None,
        }),
    });

    let hull = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: HULL_ID.to_string(),
            name: "Leg Hull".to_string(),
            position: Meters3::new(HULL_START.get(), 0.0, 0.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            design: ShipDesignSource::Inline(kit::catalog_ship(ships, subject.ship())),
            controller: SpaceshipController::Player(PlayerControllerConfig::default()),
            allegiance: Some(Allegiance::Player),
            ..default()
        }),
    });

    ScenarioConfig {
        description: format!(
            "A composed STOP/GOTO/GotoPos/ORBIT chain flown by {}.",
            subject.ship()
        ),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![well, beacon, hull],
                ThreePointRig::around("flight legs", Meters3::new(BEACON_AT.get(), 0.0, 0.0), 60.0)
                    .actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            subject.scenario().to_string(),
            format!("Flight Legs Range ({})", subject.ship()),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The scripted run: fly the whole lane on the skiff, reload it under the
/// carrier, fly it again, then read both.
#[cfg(feature = "debug")]
fn legs_script() -> Script {
    let script = Script::new()
        .step("load the skiff lane")
        .enter(GameStates::Loading)
        .until(lane_staged(Subject::Skiff))
        .deadline(LOAD_DEADLINE_SECS)
        .add();
    let script = lane_beats(script, Subject::Skiff);
    let script = script
        .step("load the carrier lane")
        .on_enter(|world: &mut World| open_lane(world, Subject::Carrier))
        .until(lane_staged(Subject::Carrier))
        .deadline(LOAD_DEADLINE_SECS)
        .add();
    // Deliberately the last beat, and deliberately gated on nothing: what the
    // lanes did is what the assertions decide, and a beat that waited for it
    // would make them unfailable.
    lane_beats(script, Subject::Carrier)
        .step("read the lanes")
        .on_enter(read_the_lanes)
        .add()
}

/// One lane's beats, appended to `script`.
#[cfg(feature = "debug")]
fn lane_beats(script: Script, subject: Subject) -> Script {
    let hull = subject.ship();
    script
        .step(format!("{hull}: fly a leg to replace"))
        .on_enter(engage_the_outbound_leg)
        .until(leg_is_flying())
        .deadline(LEG_DEADLINE_SECS)
        .add()
        .step(format!("{hull}: replace the live leg"))
        .on_enter(replace_the_live_leg)
        .until(swap_sampled())
        .deadline(LEG_DEADLINE_SECS)
        .add()
        .step(format!("{hull}: read the replacement"))
        .on_enter(read_the_replacement)
        .add()
        .step(format!("{hull}: come to rest"))
        .until(leg_completed(Leg::Stop))
        .deadline(LEG_DEADLINE_SECS)
        .add()
        .step(format!("{hull}: fly the goto"))
        .on_enter(engage_the_goto)
        .until(leg_completed(Leg::Goto))
        .deadline(LEG_DEADLINE_SECS)
        .add()
        .step(format!("{hull}: fly the mark"))
        .on_enter(engage_the_goto_pos)
        .until(leg_completed(Leg::GotoPos))
        .deadline(LEG_DEADLINE_SECS)
        .add()
        .step(format!("{hull}: hold the orbit"))
        .on_enter(engage_the_orbit)
        .until(orbit_is_holding())
        .deadline(LEG_DEADLINE_SECS)
        .add()
        .step(format!("{hull}: close the lane"))
        .on_enter(close_the_lane)
        .add()
}

/// Everything the run accumulates, across both lanes.
#[cfg(feature = "debug")]
#[derive(Resource, Debug)]
struct Legs {
    /// The lane being flown now. The skiff's lane is the one `setup_range`
    /// loads, so the run opens on it.
    subject: Subject,
    /// The live lane's hull root, and the one a reload is replacing.
    ship: Option<Entity>,
    replacing: Option<Entity>,
    /// Legs the production completion seam reported for the live lane.
    completed: Vec<Leg>,
    /// The coast/brake census while a GOTO leg is being watched, and the one it
    /// left behind when the beat closed it.
    watching: Option<Coast>,
    coast: Option<Coast>,
    /// The replacement census.
    swap: Option<Swap>,
    /// What the live lane has measured so far.
    marks: Marks,
    /// What each finished lane did.
    lanes: Vec<Lane>,
}

#[cfg(feature = "debug")]
impl Default for Legs {
    fn default() -> Self {
        Self {
            subject: Subject::Skiff,
            ship: None,
            replacing: None,
            completed: Vec::new(),
            watching: None,
            coast: None,
            swap: None,
            marks: Marks::default(),
            lanes: Vec::new(),
        }
    }
}

/// What one GOTO leg published, one fixed step at a time.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug, Default)]
struct Coast {
    /// Steps on which the leg published anything at all.
    steps: usize,
    /// Steps with the flip still AHEAD: a published flip point and no brake.
    coast_steps: usize,
    /// Steps the leg reported itself braking on.
    brake_steps: usize,
    /// Steps that were neither - inside the standoff, or a closing speed under
    /// the estimate floor, where the leg publishes no honest flip point.
    quiet_steps: usize,
    /// Edges between the two, counted in both directions. A coast that gives
    /// way to a brake exactly once is one and zero.
    coast_to_brake: usize,
    brake_to_coast: usize,
    /// The last classified step, for the edge count.
    last_braking: Option<bool>,
    /// The published hull-to-surface gap at the first sampled step, at the last
    /// coast step and at the first brake step.
    start_gap: Meters,
    last_coast_gap: Meters,
    first_brake_gap: Meters,
}

/// What the replacement window looked like, one fixed step at a time.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug)]
struct Swap {
    /// The verb that owned the helm when the replacement was written, and the
    /// one written over it.
    replaced: Verb,
    installed: Verb,
    /// Fixed steps sampled since the write.
    steps: usize,
    /// The verb owning the helm on the FIRST fixed step after the write.
    first_verb: Option<Verb>,
    /// Steps on which no autopilot was engaged at all.
    steps_adrift: usize,
    /// Completions the live lane had reported when the window opened, and the
    /// ones it reported INSIDE the window.
    ///
    /// The second is counted by the census, not read at the verdict: the driver
    /// polls once a FRAME, and a frame carries as many fixed steps as
    /// [`WALK_MAX_DELTA`] allows, so a count taken after the window closed
    /// would charge this claim with legs that finished seconds of world time
    /// later - the replacement's own STOP among them.
    completions_at_open: usize,
    completions_in_window: usize,
}

/// What the live lane has measured at its beat boundaries.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug, Default)]
struct Marks {
    /// The hull's own outer radius - the number every park point is measured
    /// against - and where it started.
    hull_radius: Meters,
    start_at: Meters,
    /// Where the STOP leg came to rest, and how fast it still was.
    stop_at: Meters,
    rest_speed: MetersPerSecond,
    /// Where the GOTO leg parked, and the hull-to-surface gap it published on
    /// its last sampled step.
    goto_at: Meters,
    /// Where the GotoPos leg parked, and how far its own face ended from the
    /// mark.
    goto_pos_at: Meters,
    goto_pos_gap: Meters,
}

/// One flown lane.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug)]
struct Lane {
    subject: Subject,
    marks: Marks,
    /// The ring ORBIT planned, the band it had to plan inside, and the radius
    /// and speed the hull held when it reached station-keeping.
    orbit_plan: Meters,
    orbit_band: (Meters, Meters),
    orbit_radius: Meters,
    orbit_speed: MetersPerSecond,
    /// The two censuses.
    coast: Coast,
    swap: Swap,
}

/// The lane's hull root, tracked as the scenario spawns it.
///
/// An observer rather than a polling system: a reload's teardown and its
/// respawn land in ONE flush, so there is no frame in which a poll could see
/// the lane empty and know the replacement had begun.
#[cfg(feature = "debug")]
fn note_the_hull(add: On<Add, PlayerSpaceshipMarker>, mut legs: ResMut<Legs>) {
    legs.ship = Some(add.entity);
}

/// Record a completion the production layer published.
///
/// An observer because the component is TRANSIENT:
/// `track_player_autopilot_completions` turns it into scenario events and
/// removes it in the same frame, so a beat polling for it would be racing the
/// scenario layer for the one frame it exists.
#[cfg(feature = "debug")]
fn note_a_completion(
    add: On<Add, PlayerAutopilotCompleted>,
    q_completed: Query<&PlayerAutopilotCompleted>,
    mut legs: ResMut<Legs>,
) {
    let Ok(completed) = q_completed.get(add.entity) else {
        return;
    };
    let leg = match completed.action {
        AutopilotAction::Stop => Leg::Stop,
        AutopilotAction::Goto { .. } => Leg::Goto,
        AutopilotAction::GotoPos { .. } => Leg::GotoPos,
        AutopilotAction::Orbit { .. } | AutopilotAction::MatchVelocity { .. } => return,
    };
    info!("flight legs: {} reported {leg:?}", legs.subject.ship());
    legs.completed.push(leg);
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

/// The lane is up: the subject's own hull root (not the one a reload replaced)
/// has been weighed, and the beacon and the well are both live.
///
/// The weighing matters. A root avian has not measured yet carries no centre of
/// mass, and every leg below is planned from one.
#[cfg(feature = "debug")]
fn lane_staged(subject: Subject) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(move |world: &World| {
        let Some(legs) = world.get_resource::<Legs>() else {
            return false;
        };
        if legs.subject != subject {
            return false;
        }
        let Some(ship) = legs.ship.filter(|ship| Some(*ship) != legs.replacing) else {
            return false;
        };
        world.get::<HullRadius>(ship).is_some()
            && world
                .get::<avian3d::prelude::ComputedCenterOfMass>(ship)
                .is_some()
            && staged(world, BEACON_ID).is_some()
            && staged(world, WELL_ID).is_some_and(|well| world.get::<GravityWell>(well).is_some())
    })
}

/// Tear the lane down and build it again under `subject`.
#[cfg(feature = "debug")]
fn open_lane(world: &mut World, subject: Subject) {
    let config = {
        let game_assets = world.resource::<GameAssets>();
        let ships = world.resource::<GameShipDesigns>();
        lane(game_assets, ships, subject)
    };
    {
        let mut legs = world.resource_mut::<Legs>();
        legs.subject = subject;
        legs.replacing = legs.ship;
        legs.completed.clear();
        legs.watching = None;
        legs.coast = None;
        legs.swap = None;
        legs.marks = Marks::default();
    }
    info!("flight legs: loading the {} lane", subject.ship());
    world.trigger(LoadScenario(config));
}

/// The lane's hull, for a beat that cannot run without one.
#[cfg(feature = "debug")]
fn lane_hull(world: &World) -> Entity {
    world
        .resource::<Legs>()
        .ship
        .expect("flight legs: the lane must have staged a hull")
}

/// Where the hull's centre of mass is on the lane's line, and how fast it is.
#[cfg(feature = "debug")]
fn hull_state(world: &World, hull: Entity) -> (Vec3, MetersPerSecond) {
    let at = world
        .get::<avian3d::prelude::Position>(hull)
        .expect("flight legs: the hull must have a pose")
        .0;
    let com = world
        .get::<avian3d::prelude::ComputedCenterOfMass>(hull)
        .map(|com| com.0)
        .unwrap_or(Vec3::ZERO);
    let rotation = world
        .get::<avian3d::prelude::Rotation>(hull)
        .map_or(Quat::IDENTITY, |rotation| rotation.0);
    let speed = world
        .get::<avian3d::prelude::LinearVelocity>(hull)
        .map_or(0.0, |velocity| velocity.length());
    (
        at + rotation.mul_vec3(com),
        MetersPerSecond::from_engine(speed),
    )
}

/// Engage an outbound leg with something to take away.
#[cfg(feature = "debug")]
fn engage_the_outbound_leg(world: &mut World) {
    let hull = lane_hull(world);
    let (at, _) = hull_state(world, hull);
    let radius = world.get::<HullRadius>(hull).map_or(0.0, |radius| **radius);
    {
        let mut legs = world.resource_mut::<Legs>();
        legs.marks.hull_radius = Meters::from_engine(radius);
        legs.marks.start_at = Meters::from_engine(at.x);
    }
    world
        .entity_mut(hull)
        .insert(Autopilot::engage(AutopilotAction::GotoPos {
            position: Vec3::X * OUTBOUND_MARK.to_engine(),
        }));
}

/// The engaged leg is demonstrably FLYING, not merely engaged.
#[cfg(feature = "debug")]
fn leg_is_flying() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some(hull) = world.get_resource::<Legs>().and_then(|legs| legs.ship) else {
            return false;
        };
        world
            .get::<ManeuverTelemetry>(hull)
            .is_some_and(|numbers| numbers.closing_speed >= REPLACE_SPEED.to_engine())
    })
}

/// Write a second action straight over the live one and open the census.
///
/// No disengage first, no helm handback, no order layer: this is the plain
/// production write, which is what the input layer, a scenario action and the
/// AI flight computer all do when the goal changes mid-leg.
#[cfg(feature = "debug")]
fn replace_the_live_leg(world: &mut World) {
    let hull = lane_hull(world);
    let replaced = Verb::of(
        &world
            .get::<Autopilot>(hull)
            .expect("flight legs: a leg must be live to be replaced")
            .action,
    );
    let completions_at_open = world.resource::<Legs>().completed.len();
    world
        .entity_mut(hull)
        .insert(Autopilot::engage(AutopilotAction::Stop));
    world.resource_mut::<Legs>().swap = Some(Swap {
        replaced,
        installed: Verb::Stop,
        steps: 0,
        first_verb: None,
        steps_adrift: 0,
        completions_at_open,
        completions_in_window: 0,
    });
}

/// Sample who owns the helm, once a fixed step, for the length of the window.
#[cfg(feature = "debug")]
fn census_the_swap(mut legs: ResMut<Legs>, q_hull: Query<&Autopilot>) {
    let Some(hull) = legs.ship else { return };
    let Some(mut swap) = legs.swap else { return };
    if swap.steps >= SWAP_STEPS {
        return;
    }
    let verb = q_hull
        .get(hull)
        .ok()
        .map(|autopilot| Verb::of(&autopilot.action));
    let completed = legs.completed.len();
    swap.steps += 1;
    swap.completions_in_window = completed.saturating_sub(swap.completions_at_open);
    match verb {
        Some(verb) => {
            if swap.first_verb.is_none() {
                swap.first_verb = Some(verb);
            }
        }
        None => swap.steps_adrift += 1,
    }
    legs.swap = Some(swap);
}

/// The window has its samples.
#[cfg(feature = "debug")]
fn swap_sampled() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<Legs>(|legs| legs.swap.is_some_and(|swap| swap.steps >= SWAP_STEPS))
}

/// Read the replacement window: the new verb took the helm on the next flight
/// tick, kept it, and the leg it replaced reported nothing.
#[cfg(feature = "debug")]
fn read_the_replacement(world: &mut World) {
    let legs = world.resource::<Legs>();
    let swap = legs
        .swap
        .expect("flight legs: the replacement census must be open");
    let completions = swap.completions_in_window;
    let hull = legs.subject.ship();

    assert_eq!(
        swap.first_verb,
        Some(swap.installed),
        "flight legs ({hull}): an action written over a live one must own the \
         helm on the NEXT flight tick; the step after the write was flying {:?} \
         (replaced {:?})",
        swap.first_verb,
        swap.replaced
    );
    assert_eq!(
        swap.steps_adrift, 0,
        "flight legs ({hull}): a replacement is not a disengage - the hull must \
         never be handed back to manual in between ({} of {} sampled steps \
         carried no autopilot)",
        swap.steps_adrift, swap.steps
    );
    assert_eq!(
        completions, 0,
        "flight legs ({hull}): a leg taken off the helm did not finish, so it \
         must report no completion (saw {completions})"
    );
}

/// The live lane reported this leg complete.
#[cfg(feature = "debug")]
fn leg_completed(leg: Leg) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<Legs>(move |legs| legs.completed.contains(&leg))
}

/// Read the STOP leg's end state, then engage the GOTO at the beacon and open
/// the coast census on it.
#[cfg(feature = "debug")]
fn engage_the_goto(world: &mut World) {
    let hull = lane_hull(world);
    let beacon = staged(world, BEACON_ID).expect("flight legs: the beacon must be staged");
    let (at, speed) = hull_state(world, hull);
    {
        let mut legs = world.resource_mut::<Legs>();
        legs.marks.stop_at = Meters::from_engine(at.x);
        legs.marks.rest_speed = speed;
        legs.watching = Some(Coast::default());
    }
    world
        .entity_mut(hull)
        .insert(Autopilot::engage(AutopilotAction::Goto { target: beacon }));
}

/// Classify one fixed step of the watched GOTO leg.
///
/// The three states are the leg's OWN published ones, not this range's
/// arithmetic: a coast is a published flip point with `braking` clear, a brake
/// is `braking` set, and the rest is the leg saying it has no honest estimate -
/// inside the standoff, or under the closing-speed floor.
#[cfg(feature = "debug")]
fn census_the_goto(mut legs: ResMut<Legs>, q_hull: Query<&ManeuverTelemetry>) {
    let Some(hull) = legs.ship else { return };
    let Some(mut coast) = legs.watching else {
        return;
    };
    let Ok(numbers) = q_hull.get(hull) else {
        return;
    };
    let gap = Meters::from_engine(numbers.distance);

    if coast.steps == 0 {
        coast.start_gap = gap;
    }
    coast.steps += 1;
    let braking = if numbers.braking {
        coast.brake_steps += 1;
        if coast.brake_steps == 1 {
            coast.first_brake_gap = gap;
        }
        Some(true)
    } else if numbers.flip_point.is_some() {
        coast.coast_steps += 1;
        coast.last_coast_gap = gap;
        Some(false)
    } else {
        coast.quiet_steps += 1;
        None
    };
    if let Some(braking) = braking {
        match coast.last_braking {
            Some(false) if braking => coast.coast_to_brake += 1,
            Some(true) if !braking => coast.brake_to_coast += 1,
            _ => {}
        }
        coast.last_braking = Some(braking);
    }
    legs.watching = Some(coast);
}

/// Close the coast census, read the GOTO leg's end state, and engage the
/// GotoPos at the well's own centre.
///
/// The census is closed HERE, on the beat boundary: the GOTO's telemetry dies
/// with the leg, but the NEXT leg publishes its own, and a census left open
/// would fold one leg's numbers into another's.
#[cfg(feature = "debug")]
fn engage_the_goto_pos(world: &mut World) {
    let hull = lane_hull(world);
    let (at, _) = hull_state(world, hull);
    {
        let mut legs = world.resource_mut::<Legs>();
        legs.coast = legs.watching.take();
        legs.marks.goto_at = Meters::from_engine(at.x);
    }
    world
        .entity_mut(hull)
        .insert(Autopilot::engage(AutopilotAction::GotoPos {
            position: Vec3::ZERO,
        }));
}

/// Read the GotoPos leg's end state, then engage the ORBIT at the staged well.
///
/// `plan: None` is the production shape: the computer picks the ring and the
/// plane on its first engaged tick.
#[cfg(feature = "debug")]
fn engage_the_orbit(world: &mut World) {
    let hull = lane_hull(world);
    let well = staged(world, WELL_ID).expect("flight legs: the well must be staged");
    let (at, _) = hull_state(world, hull);
    {
        let radius = world.resource::<Legs>().marks.hull_radius;
        let mut legs = world.resource_mut::<Legs>();
        legs.marks.goto_pos_at = Meters::from_engine(at.x);
        // The gap between the hull's own FACE and the mark it was sent to: the
        // quantity the shipped arrival margin is expressed in.
        legs.marks.goto_pos_gap = Meters(Meters::from_engine(at.length()).get() - radius.get());
    }
    world
        .entity_mut(hull)
        .insert(Autopilot::engage(AutopilotAction::Orbit {
            well,
            plan: None,
        }));
}

/// The orbit reached the station-keeping phase the scenario layer reads as a
/// stable orbit.
#[cfg(feature = "debug")]
fn orbit_is_holding() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some(hull) = world.get_resource::<Legs>().and_then(|legs| legs.ship) else {
            return false;
        };
        world
            .get::<Autopilot>(hull)
            .is_some_and(|autopilot| autopilot.phase == AutopilotPhase::Hold)
    })
}

/// Read the lane's orbit and file the whole lane.
#[cfg(feature = "debug")]
fn close_the_lane(world: &mut World) {
    let hull = lane_hull(world);
    let well = staged(world, WELL_ID).expect("flight legs: the well must be staged");
    let well_at = world
        .get::<avian3d::prelude::Position>(well)
        .expect("flight legs: the well must be on rails")
        .0;
    let band = {
        let geometry = world
            .get::<GravityWell>(well)
            .expect("flight legs: the well must publish its geometry");
        orbit_radius_band(
            geometry,
            world.resource::<GravitySettings>(),
            world.resource::<FlightSettings>(),
        )
        .expect("flight legs: the staged well must have a stable ring band")
    };
    let (at, speed) = hull_state(world, hull);
    let plan = match world
        .get::<Autopilot>(hull)
        .map(|autopilot| autopilot.action)
    {
        Some(AutopilotAction::Orbit {
            plan: Some(plan), ..
        }) => plan.radius,
        other => panic!("flight legs: the lane must end on a planned orbit, got {other:?}"),
    };

    let legs = world.resource::<Legs>();
    let lane = Lane {
        subject: legs.subject,
        marks: legs.marks,
        orbit_plan: Meters::from_engine(plan),
        orbit_band: (Meters::from_engine(band.0), Meters::from_engine(band.1)),
        orbit_radius: Meters::from_engine((at - well_at).length()),
        orbit_speed: speed,
        coast: legs
            .coast
            .expect("flight legs: the lane must have watched a GOTO leg"),
        swap: legs
            .swap
            .expect("flight legs: the lane must have replaced a leg"),
    };
    info!(
        "flight legs: {} flew the chain - radius {:.0} m, rest {:.2} m/s at {:.0} m, \
         goto {:.0} m, mark gap {:.0} m, orbit {:.0} m on a {:.0} m ring at {:.1} m/s; \
         goto coast {}/brake {}/quiet {} steps, {} coast-to-brake, {} back",
        lane.subject.ship(),
        lane.marks.hull_radius.get(),
        lane.marks.rest_speed.get(),
        lane.marks.stop_at.get(),
        lane.marks.goto_at.get(),
        lane.marks.goto_pos_gap.get(),
        lane.orbit_radius.get(),
        lane.orbit_plan.get(),
        lane.orbit_speed.get(),
        lane.coast.coast_steps,
        lane.coast.brake_steps,
        lane.coast.quiet_steps,
        lane.coast.coast_to_brake,
        lane.coast.brake_to_coast,
    );
    world.resource_mut::<Legs>().lanes.push(lane);
}

/// Read both lanes: the chain completed on each hull, each GOTO coasted before
/// it braked, and each replacement was carried on the next flight tick.
#[cfg(feature = "debug")]
fn read_the_lanes(world: &mut World) {
    let elapsed = world.resource::<Time>().elapsed_secs();
    let lanes = world.resource::<Legs>().lanes.clone();
    assert_eq!(
        lanes.len(),
        2,
        "flight legs: both hulls must have flown the lane"
    );
    let rest_ceiling = MetersPerSecond::from_engine(
        REST_EPSILONS * world.resource::<FlightSettings>().stop_speed_epsilon,
    );
    let park_ceiling =
        Meters::from_engine(PARK_SLACK * world.resource::<FlightSettings>().arrival_standoff);

    // Claim 1: the chain completed, and the hull moved to do it.
    for lane in &lanes {
        let hull = lane.subject.ship();
        let marks = lane.marks;
        assert!(
            marks.rest_speed.get() <= rest_ceiling.get(),
            "flight legs ({hull}): a completed STOP hands back a hull at rest; it \
             still carried {:.2} m/s (ceiling {:.2})",
            marks.rest_speed.get(),
            rest_ceiling.get()
        );
        assert!(
            marks.goto_at.get() < marks.stop_at.get(),
            "flight legs ({hull}): the GOTO must have flown the hull in toward \
             the beacon; it went from {:.0} m to {:.0} m",
            marks.stop_at.get(),
            marks.goto_at.get()
        );
        assert!(
            marks.goto_pos_at.get() < marks.goto_at.get(),
            "flight legs ({hull}): the GotoPos must have flown the hull in past \
             the beacon park; it went from {:.0} m to {:.0} m",
            marks.goto_at.get(),
            marks.goto_pos_at.get()
        );
        assert!(
            marks.goto_pos_gap.get() > 0.0 && marks.goto_pos_gap.get() <= park_ceiling.get(),
            "flight legs ({hull}): the GotoPos leg must rest one margin off its \
             own face; the gap was {:.0} m (ceiling {:.0})",
            marks.goto_pos_gap.get(),
            park_ceiling.get()
        );
        assert!(
            lane.orbit_plan.get() >= lane.orbit_band.0.get()
                && lane.orbit_plan.get() <= lane.orbit_band.1.get(),
            "flight legs ({hull}): the parked orbit must ring inside the well's \
             own stable band; planned {:.0} m against {:.0}-{:.0} m",
            lane.orbit_plan.get(),
            lane.orbit_band.0.get(),
            lane.orbit_band.1.get()
        );
    }
    nova_probe::probe_marker(
        world,
        "outcome: the composed leg chain completes on both hulls",
        serde_json::json!({
            "t": elapsed,
            "hulls": lanes.iter().map(|lane| lane.subject.ship()).collect::<Vec<_>>(),
            "rest_speed_m_s": lanes.iter().map(|lane| lane.marks.rest_speed.get()).collect::<Vec<_>>(),
            "orbit_plan_m": lanes.iter().map(|lane| lane.orbit_plan.get()).collect::<Vec<_>>(),
            "orbit_radius_m": lanes.iter().map(|lane| lane.orbit_radius.get()).collect::<Vec<_>>(),
        }),
    );

    // Claim 2: one coast, then one brake, and no going back.
    for lane in &lanes {
        let hull = lane.subject.ship();
        let coast = lane.coast;
        assert!(
            coast.coast_steps > 0,
            "flight legs ({hull}): the staged GOTO never published a flip point \
             ahead of it, so nothing here saw it coast ({} steps: {} braking, {} \
             with no estimate)",
            coast.steps,
            coast.brake_steps,
            coast.quiet_steps
        );
        assert!(
            coast.brake_steps > 0,
            "flight legs ({hull}): the staged GOTO never committed to its brake \
             ({} steps: {} coasting, {} with no estimate)",
            coast.steps,
            coast.coast_steps,
            coast.quiet_steps
        );
        assert_eq!(
            coast.coast_to_brake, 1,
            "flight legs ({hull}): an unobstructed arrival gives way to its brake \
             exactly ONCE (saw {} handovers over {} steps)",
            coast.coast_to_brake, coast.steps
        );
        assert_eq!(
            coast.brake_to_coast, 0,
            "flight legs ({hull}): a committed brake must not fall back to \
             coasting (saw {} reversals over {} steps)",
            coast.brake_to_coast, coast.steps
        );
        assert!(
            coast.first_brake_gap.get() < coast.last_coast_gap.get(),
            "flight legs ({hull}): the brake must begin closer in than the last \
             coasting step; coast ended at {:.0} m and the brake began at {:.0} m",
            coast.last_coast_gap.get(),
            coast.first_brake_gap.get()
        );
    }
    nova_probe::probe_marker(
        world,
        "outcome: an unobstructed goto coasts before it brakes",
        serde_json::json!({
            "t": elapsed,
            "hulls": lanes.iter().map(|lane| lane.subject.ship()).collect::<Vec<_>>(),
            "coast_steps": lanes.iter().map(|lane| lane.coast.coast_steps).collect::<Vec<_>>(),
            "brake_steps": lanes.iter().map(|lane| lane.coast.brake_steps).collect::<Vec<_>>(),
            "handovers": lanes.iter().map(|lane| lane.coast.coast_to_brake).collect::<Vec<_>>(),
            "reversals": lanes.iter().map(|lane| lane.coast.brake_to_coast).collect::<Vec<_>>(),
            "brake_gap_m": lanes.iter().map(|lane| lane.coast.first_brake_gap.get()).collect::<Vec<_>>(),
        }),
    );

    // Claim 3: the replacement, already asserted per lane as it happened. This
    // is the same window read across both hulls, so a range that silently
    // skipped one cannot pass.
    for lane in &lanes {
        let hull = lane.subject.ship();
        assert_eq!(
            lane.swap.first_verb,
            Some(lane.swap.installed),
            "flight legs ({hull}): the replacement window must have handed the \
             helm to the written action"
        );
        assert_eq!(
            lane.swap.steps, SWAP_STEPS,
            "flight legs ({hull}): the replacement window must have been sampled"
        );
        assert_eq!(
            lane.swap.steps_adrift, 0,
            "flight legs ({hull}): the helm must not have been handed back"
        );
    }
    nova_probe::probe_marker(
        world,
        "outcome: a replacement action owns the helm on the next flight tick",
        serde_json::json!({
            "t": elapsed,
            "hulls": lanes.iter().map(|lane| lane.subject.ship()).collect::<Vec<_>>(),
            "replaced": lanes.iter().map(|lane| format!("{:?}", lane.swap.replaced)).collect::<Vec<_>>(),
            "installed": lanes.iter().map(|lane| format!("{:?}", lane.swap.installed)).collect::<Vec<_>>(),
            "steps_sampled": lanes.iter().map(|lane| lane.swap.steps).collect::<Vec<_>>(),
        }),
    );

    nova_probe::probe_marker(
        world,
        "outcome: the flown lanes are recorded",
        serde_json::json!({
            "t": elapsed,
            "lanes": lanes.iter().map(|lane| serde_json::json!({
                "hull": lane.subject.ship(),
                "hull_radius_m": lane.marks.hull_radius.get(),
                "start_m": lane.marks.start_at.get(),
                "stop_m": lane.marks.stop_at.get(),
                "rest_speed_m_s": lane.marks.rest_speed.get(),
                "goto_m": lane.marks.goto_at.get(),
                "goto_pos_m": lane.marks.goto_pos_at.get(),
                "goto_pos_gap_m": lane.marks.goto_pos_gap.get(),
                "orbit_plan_m": lane.orbit_plan.get(),
                "orbit_band_m": [lane.orbit_band.0.get(), lane.orbit_band.1.get()],
                "orbit_radius_m": lane.orbit_radius.get(),
                "orbit_speed_m_s": lane.orbit_speed.get(),
                "goto_steps": lane.coast.steps,
                "goto_coast_steps": lane.coast.coast_steps,
                "goto_brake_steps": lane.coast.brake_steps,
                "goto_quiet_steps": lane.coast.quiet_steps,
                "goto_start_gap_m": lane.coast.start_gap.get(),
                "goto_last_coast_gap_m": lane.coast.last_coast_gap.get(),
                "goto_first_brake_gap_m": lane.coast.first_brake_gap.get(),
            })).collect::<Vec<_>>(),
        }),
    );
}
