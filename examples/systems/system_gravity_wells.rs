//! system_gravity_wells: which well owns a hull that is inside two of them.
//!
//! One piloted `block_skiff` flies, on the production autopilot, straight down
//! the line between two equal authored wells whose spheres of influence cover
//! the whole segment between them. The subject is the OWNERSHIP policy the
//! force system runs while both wells reach the ship: a hull is pulled by one
//! well, never a blend of two, and the well it is pulled by does not change the
//! instant a neighbour becomes stronger - it changes when the neighbour clears
//! [`GravitySettings::switch_hysteresis`].
//!
//! Sampled ONCE A FIXED STEP, not once a frame. `gravity_well_system` chooses
//! the owner every fixed step, and on a software rasterizer a frame holds a
//! dozen of them: a per-frame census of a moving hull would step straight over
//! the handoff it is here to watch.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a hull inside two wells is owned by one that reaches it` | every sampled step of the crossing had at least two wells reaching the hull and exactly one owner, and that owner always reached it |
//! | 2 | `outcome: the incumbent well holds until a challenger clears the margin` | the crossing handed off exactly once; the incumbent kept the hull while strictly weaker, and the flip came only once the challenger beat it by more than the authored margin |
//! | 3 | `outcome: the well crossing is recorded` | RECORD: steps sampled, where the handoff landed, the pull ratio the incumbent held to and the ratio it lost at |
//!
//! What this range does NOT claim, because focused tests already do:
//! inward pull, SOI release, well removal, orbit laps
//! (`nova_gameplay/src/gravity.rs`), and a gun round curving under a well
//! against a straight control (`a_round_curves_under_a_well_and_flies_straight_without_one`,
//! `nova_gameplay/src/rounds.rs`). It says nothing at all about the lead pip.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_gravity_wells --features debug
//! # look for: `gravity wells: the crossing handed off ...`,
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
#[command(name = "system_gravity_wells")]
#[command(version = "1.0.0")]
#[command(
    about = "Which of two overlapping gravity wells owns a flying hull. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

/// The hull that flies the crossing: the smallest shipped hull there is.
///
/// Gravity is an acceleration, so it is mass-independent by construction
/// (`gravity_well_system` applies a linear ACCELERATION) and a capital hull
/// would buy this range nothing but frames. Hull scale is staged where it is
/// part of the claim, in `system_flight_legs`.
const SKIFF: &str = "block_skiff";

/// The flying hull's scenario id.
const SKIFF_ID: &str = "gravity_skiff";

/// The well the crossing starts inside.
const WELL_A_ID: &str = "gravity_well_a";

/// The well the crossing ends inside.
const WELL_B_ID: &str = "gravity_well_b";

/// Both wells' published body radius.
const WELL_RADIUS: Meters = Meters(800.0);

/// Both wells' authored strength (`mu`, the designer dial `AnchorConfig::mass`
/// carries straight into [`GravityWell::from_mass`]).
///
/// Well under the strength cap for this radius
/// (`max_surface_gravity * radius^2` is 64 000 engine units at the shipped
/// 10 u/s^2 and an 80 u radius), so the authored number is the number the well
/// gets and the sphere of influence below is the one it derives.
const WELL_MU: f32 = 20_000.0;

/// How far apart the two wells sit.
///
/// Each well's SOI is `sqrt(mu / soi_cutoff_accel)` - 2 828 m at the shipped
/// cutoff - so at this separation each one reaches more than a kilometre past
/// the midpoint and the whole crossing below happens inside both.
const WELL_SEPARATION: Meters = Meters(4_000.0);

/// Where the crossing starts, measured from well A along the line to well B.
///
/// Inside both spheres of influence and still nearer A, so A takes the hull
/// first and is the incumbent the margin is measured against.
const CROSS_START: Meters = Meters(1_900.0);

/// Where the crossing ends.
///
/// Past the point where B beats A by the shipped 1.1 margin (2 048 m for equal
/// wells at this separation), so the handoff is inside the flown segment
/// rather than beyond it - and still inside A's sphere of influence, so the
/// hull never leaves the overlap.
#[cfg(feature = "debug")]
const CROSS_END: Meters = Meters(2_150.0);

/// The velocity the hull holds across the segment.
///
/// Slow enough that the 48 m of hysteresis margin between the crossover and
/// the handoff is tens of fixed steps wide, fast enough that the segment is
/// seconds of world time rather than a minute of them.
#[cfg(feature = "debug")]
const CROSS_SPEED: MetersPerSecond = MetersPerSecond(50.0);

/// In-step seconds a beat gets to reach its world condition.
///
/// A backstop that names a hung beat, not a budget: it is several times the
/// slowest healthy beat here, with room for the software rasterizer CI runs
/// the correctness pass on. The run-level deadline
/// (`NOVA_AUTOPILOT_DEADLINE`) is a separate, shorter ceiling on the whole
/// script, so which of the two names a stall depends on where the stall
/// lands.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 120.0;

/// How much slip the sampling cadence is allowed against the decision.
///
/// `gravity_well_system` chooses the owner from the pose at the TOP of a fixed
/// step; this census reads the pose after the solver has moved the hull on by
/// one step of travel, so every ratio it computes is the decision's ratio plus
/// one step of drift. At [`CROSS_SPEED`] that is under a tenth of a metre, and
/// the ratio moves about 2% per metre here, so 5% is orders of magnitude of
/// room without letting a missing margin through - a well selection with no
/// hysteresis at all hands off at a ratio of 1.0.
#[cfg(feature = "debug")]
const SAMPLE_SLIP: f32 = 1.05;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(crossing_script());
    }

    app.run()
}

fn range_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
    // ONCE A FIXED STEP, after the solver: the ownership decision is a
    // fixed-step decision, and a render frame on a software rasterizer holds
    // several of them. A per-frame census would sample the last step of each
    // frame and read a continuous handoff as a jump.
    #[cfg(feature = "debug")]
    app.add_systems(
        FixedPostUpdate,
        census_the_crossing
            .after(avian3d::prelude::PhysicsSystems::Last)
            .run_if(resource_exists::<Crossing>),
    );
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    commands.trigger(LoadScenario(crossing_range(&game_assets, &ships)));
}

/// The range: two invisible equal wells on one line, and a piloted skiff on the
/// line between them.
///
/// Anchors rather than planets or rocks: an anchor publishes an AUTHORED radius
/// and an authored strength with no mesh, no collider and no `BodyRadius`, so
/// the geometry this range computes its own pulls from is the geometry the
/// scenario wrote, not a noise seed's.
fn crossing_range(game_assets: &GameAssets, ships: &GameShipDesigns) -> ScenarioConfig {
    let well = |id: &str, name: &str, at: Meters3| {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: name.to_string(),
                position: at,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Anchor(AnchorConfig {
                body_radius: WELL_RADIUS,
                mass: Some(WELL_MU),
            }),
        })
    };

    let skiff = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: SKIFF_ID.to_string(),
            name: "Skiff".to_string(),
            position: Meters3::new(CROSS_START.get(), 0.0, 0.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            design: ShipDesignSource::Inline(kit::catalog_ship(ships, SKIFF)),
            controller: SpaceshipController::Player(PlayerControllerConfig::default()),
            allegiance: Some(Allegiance::Player),
            ..default()
        }),
    });

    ScenarioConfig {
        description: "A piloted hull crossing the overlap between two equal gravity wells."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    well(WELL_A_ID, "Well A", Meters3::ZERO),
                    well(
                        WELL_B_ID,
                        "Well B",
                        Meters3::new(WELL_SEPARATION.get(), 0.0, 0.0),
                    ),
                    skiff,
                ],
                ThreePointRig::around(
                    "gravity wells",
                    Meters3::new(WELL_SEPARATION.get() / 2.0, 0.0, 0.0),
                    40.0,
                )
                .actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "gravity_wells_range".to_string(),
            "Gravity Wells Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The scripted run: stage the two wells and the hull, let the hull take its
/// first well, fly it across the overlap, read the census.
#[cfg(feature = "debug")]
fn crossing_script() -> Script {
    Script::new()
        .step("load the crossing")
        .enter(GameStates::Loading)
        .until(crossing_staged())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let the hull take its first well")
        .until(hull_owned())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("fly the hull across the overlap")
        .on_enter(start_the_crossing)
        .until(hull_past_the_far_mark())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Deliberately the last beat, and deliberately NOT gated on a handoff
        // having happened: whether ownership changed, and how often, is what
        // the assertions decide. A beat that waited for it would make them
        // unfailable and turn a broken policy into a deadline stall.
        .step("read the crossing")
        .on_enter(read_the_crossing)
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

/// Both wells carry their derived [`GravityWell`] and the hull has been
/// weighed: a root avian has not measured yet has no centre of mass, and every
/// distance below would be taken from the build origin instead.
#[cfg(feature = "debug")]
fn crossing_staged() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        [WELL_A_ID, WELL_B_ID].iter().all(|id| {
            staged(world, id).is_some_and(|well| world.get::<GravityWell>(well).is_some())
        }) && staged(world, SKIFF_ID).is_some_and(|hull| {
            world.get::<HullRadius>(hull).is_some()
                && world
                    .get::<avian3d::prelude::ComputedCenterOfMass>(hull)
                    .is_some()
        })
    })
}

/// The hull has been taken by a well - the incumbent the margin is measured
/// against exists.
#[cfg(feature = "debug")]
fn hull_owned() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        staged(world, SKIFF_ID).is_some_and(|hull| world.get::<DominantWell>(hull).is_some())
    })
}

/// The hull has reached the far end of the staged segment.
///
/// Position only. Nothing here reads ownership: see the beat's comment.
#[cfg(feature = "debug")]
fn hull_past_the_far_mark() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        staged(world, SKIFF_ID).is_some_and(|hull| {
            world
                .get::<avian3d::prelude::Position>(hull)
                .is_some_and(|position| position.x >= CROSS_END.to_engine())
        })
    })
}

/// Put the hull on a held velocity down the line, and open the census.
///
/// [`AutopilotAction::MatchVelocity`] rather than a GOTO: it never
/// self-completes, so the production flight layer flies the whole segment at
/// one speed and the crossing is not also a reading of an arrival plan.
#[cfg(feature = "debug")]
fn start_the_crossing(world: &mut World) {
    let hull = staged(world, SKIFF_ID).expect("gravity wells: the skiff must be staged");
    let well_a = staged(world, WELL_A_ID).expect("gravity wells: well A must be staged");
    let well_b = staged(world, WELL_B_ID).expect("gravity wells: well B must be staged");
    world
        .entity_mut(hull)
        .insert(Autopilot::engage(AutopilotAction::MatchVelocity {
            velocity: Vec3::X * CROSS_SPEED.to_engine(),
            facing: Some(Dir3::X),
        }));
    world.insert_resource(Crossing::new(hull, well_a, well_b));
    info!("gravity wells: flying the crossing");
}

/// What the crossing looked like, filled in one fixed step at a time.
#[cfg(feature = "debug")]
#[derive(Resource, Debug)]
struct Crossing {
    /// The flying hull.
    hull: Entity,
    /// The well the crossing starts inside.
    well_a: Entity,
    /// The well the crossing ends inside.
    well_b: Entity,
    /// Fixed steps sampled.
    steps: usize,
    /// Steps on which no well owned the hull at all.
    steps_unowned: usize,
    /// Steps on which the owning well's sphere of influence did not reach the
    /// hull.
    steps_owner_out_of_reach: usize,
    /// Steps on which either staged well had the hull in its outer fade band,
    /// where the pull is no longer `mu / r^2` and this range's own arithmetic
    /// would stop matching the engine's.
    steps_outside_the_core: usize,
    /// Fewest wells reaching the hull on any sampled step.
    fewest_reaching: usize,
    /// How many times ownership changed hands.
    switches: usize,
    /// Steps on which the owner was strictly the weaker of the two wells.
    steps_incumbent_weaker: usize,
    /// The largest `challenger / incumbent` pull ratio the incumbent survived.
    widest_ratio_held: f32,
    /// The ratio the flip was observed at, and where along the line it landed.
    switch_ratio: Option<f32>,
    switch_at: Option<Meters>,
    /// Who owned the hull on the first and last sampled step.
    first_owner: Option<Entity>,
    last_owner: Option<Entity>,
    /// Where the sampled segment started and ended.
    first_at: Option<Meters>,
    last_at: Option<Meters>,
}

#[cfg(feature = "debug")]
impl Crossing {
    fn new(hull: Entity, well_a: Entity, well_b: Entity) -> Self {
        Self {
            hull,
            well_a,
            well_b,
            steps: 0,
            steps_unowned: 0,
            steps_owner_out_of_reach: 0,
            steps_outside_the_core: 0,
            fewest_reaching: usize::MAX,
            switches: 0,
            steps_incumbent_weaker: 0,
            widest_ratio_held: 0.0,
            switch_ratio: None,
            switch_at: None,
            first_owner: None,
            last_owner: None,
            first_at: None,
            last_at: None,
        }
    }
}

/// Count what the crossing did, once a fixed step.
///
/// The pulls are this range's OWN arithmetic - the inverse square of the
/// authored strength over the live separation - not a second call into the
/// engine's `well_accel`. That is why the fade band is excluded above: inside
/// the unfaded core the two are the same number, so an independent oracle is
/// available for free; inside the band it would not be.
#[cfg(feature = "debug")]
fn census_the_crossing(
    mut crossing: ResMut<Crossing>,
    settings: Res<GravitySettings>,
    q_hull: Query<(&avian3d::prelude::Position, Option<&DominantWell>)>,
    q_wells: Query<(Entity, &avian3d::prelude::Position, &GravityWell)>,
) {
    let Ok((hull_position, owner)) = q_hull.get(crossing.hull) else {
        return;
    };
    let at = hull_position.0;

    // Every well that reaches the hull right now, with the pull it exerts.
    let mut reaching = 0usize;
    let pull_of = |well: Entity| -> Option<f32> {
        let (_, position, geometry) = q_wells.get(well).ok()?;
        let r = (position.0 - at).length();
        (r < geometry.soi_radius).then(|| geometry.mu / (r * r))
    };
    let pull_a = pull_of(crossing.well_a);
    let pull_b = pull_of(crossing.well_b);
    for (well, position, geometry) in &q_wells {
        if (position.0 - at).length() < geometry.soi_radius {
            reaching += 1;
        }
        let _ = well;
    }

    // The unfaded-core guard: both staged wells must be pulling by the plain
    // inverse square for the ratios below to be this range's own claim.
    let in_core = |well: Entity| -> bool {
        q_wells.get(well).is_ok_and(|(_, position, geometry)| {
            (position.0 - at).length() <= geometry.soi_radius * (1.0 - settings.fade_fraction)
        })
    };
    if !in_core(crossing.well_a) || !in_core(crossing.well_b) {
        crossing.steps_outside_the_core += 1;
    }

    crossing.steps += 1;
    crossing.fewest_reaching = crossing.fewest_reaching.min(reaching);
    let along = Meters::from_engine(at.x);
    crossing.last_at = Some(along);
    if crossing.first_at.is_none() {
        crossing.first_at = Some(along);
    }

    let Some(owner) = owner.map(|dominant| **dominant) else {
        crossing.steps_unowned += 1;
        return;
    };
    if !q_wells
        .get(owner)
        .is_ok_and(|(_, position, geometry)| (position.0 - at).length() < geometry.soi_radius)
    {
        crossing.steps_owner_out_of_reach += 1;
    }

    // The two staged wells, as (what the owner pulls with, what the other one
    // pulls with). Either being out of reach is already counted above.
    let contest = match (owner == crossing.well_a, pull_a, pull_b) {
        (true, Some(incumbent), Some(challenger)) => Some((incumbent, challenger)),
        (false, Some(challenger), Some(incumbent)) => Some((incumbent, challenger)),
        _ => None,
    };
    if let Some((incumbent, challenger)) = contest {
        if challenger > incumbent {
            crossing.steps_incumbent_weaker += 1;
            crossing.widest_ratio_held = crossing.widest_ratio_held.max(challenger / incumbent);
        }
    }

    if crossing
        .last_owner
        .is_some_and(|previous| previous != owner)
    {
        crossing.switches += 1;
        // The ratio the NEW owner won by, over the one it took the hull from.
        crossing.switch_ratio = contest.map(|(new, old)| new / old);
        crossing.switch_at = Some(along);
    }
    crossing.last_owner = Some(owner);
    if crossing.first_owner.is_none() {
        crossing.first_owner = Some(owner);
    }
}

/// Read the census: one owner throughout, one handoff, and the handoff on the
/// margin rather than on the crossover.
#[cfg(feature = "debug")]
fn read_the_crossing(world: &mut World) {
    let hysteresis = world.resource::<GravitySettings>().switch_hysteresis;
    let crossing = world.remove_resource::<Crossing>().expect(
        "gravity wells: the crossing census must survive the flight - it is inserted with the \
         hull's held velocity",
    );
    let elapsed = world.resource::<Time>().elapsed_secs();

    // Claim 1: the hull was inside two wells for the whole sampled segment, and
    // exactly one of them owned it on every step of it.
    assert!(
        crossing.steps > 0,
        "gravity wells: the crossing sampled no fixed steps at all"
    );
    assert!(
        crossing.fewest_reaching >= 2,
        "gravity wells: the staged spheres of influence must overlap across the \
         whole crossing; the thinnest step had {} well(s) reaching the hull",
        crossing.fewest_reaching
    );
    assert_eq!(
        crossing.steps_unowned, 0,
        "gravity wells: a hull inside a live sphere of influence must always \
         carry a dominant well ({} of {} steps carried none)",
        crossing.steps_unowned, crossing.steps
    );
    assert_eq!(
        crossing.steps_owner_out_of_reach, 0,
        "gravity wells: the owning well must be one that actually reaches the \
         hull ({} of {} steps were owned from outside a sphere of influence)",
        crossing.steps_owner_out_of_reach, crossing.steps
    );
    assert_eq!(
        crossing.steps_outside_the_core, 0,
        "gravity wells: the crossing must stay inside both wells' unfaded core, \
         where this range's own inverse-square oracle is the engine's pull ({} \
         of {} steps were in a fade band)",
        crossing.steps_outside_the_core, crossing.steps
    );
    nova_probe::probe_marker(
        world,
        "outcome: a hull inside two wells is owned by one that reaches it",
        serde_json::json!({
            "t": elapsed,
            "steps": crossing.steps,
            "fewest_wells_reaching": crossing.fewest_reaching,
        }),
    );

    // Claim 2: one handoff, taken on the margin.
    assert_eq!(
        crossing.first_owner,
        Some(crossing.well_a),
        "gravity wells: the crossing must start owned by the near well"
    );
    assert_eq!(
        crossing.last_owner,
        Some(crossing.well_b),
        "gravity wells: the crossing must end owned by the far well"
    );
    assert_eq!(
        crossing.switches, 1,
        "gravity wells: a straight crossing of two equal wells hands off once \
         and does not flicker (saw {} changes of owner)",
        crossing.switches
    );
    assert!(
        crossing.steps_incumbent_weaker > 0,
        "gravity wells: the crossing never held the hull on the weaker well, so \
         nothing here tested the margin at all"
    );
    assert!(
        crossing.widest_ratio_held > 1.0,
        "gravity wells: the incumbent must keep a hull a challenger has already \
         out-pulled; the widest ratio it survived was {:.4}",
        crossing.widest_ratio_held
    );
    assert!(
        crossing.widest_ratio_held <= hysteresis * SAMPLE_SLIP,
        "gravity wells: the incumbent held to a pull ratio of {:.4}, past the \
         authored {hysteresis} margin",
        crossing.widest_ratio_held
    );
    let switch_ratio = crossing
        .switch_ratio
        .expect("gravity wells: a counted handoff must have both wells reaching");
    assert!(
        switch_ratio > hysteresis,
        "gravity wells: ownership changed at a pull ratio of {switch_ratio:.4}, \
         inside the authored {hysteresis} margin - the strongest well does not \
         simply win"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the incumbent well holds until a challenger clears the margin",
        serde_json::json!({
            "t": elapsed,
            "hysteresis": hysteresis,
            "widest_ratio_held": crossing.widest_ratio_held,
            "switch_ratio": switch_ratio,
        }),
    );

    let switch_at = crossing.switch_at.map(Meters::get);
    info!(
        "gravity wells: the crossing handed off once, at {:?} m, on a pull ratio \
         of {switch_ratio:.4} (margin {hysteresis}); the incumbent held to \
         {:.4} over {} steps",
        switch_at, crossing.widest_ratio_held, crossing.steps_incumbent_weaker
    );
    nova_probe::probe_marker(
        world,
        "outcome: the well crossing is recorded",
        serde_json::json!({
            "t": elapsed,
            "steps": crossing.steps,
            "from_m": crossing.first_at.map(Meters::get),
            "to_m": crossing.last_at.map(Meters::get),
            "handoff_m": switch_at,
            "steps_on_the_weaker_well": crossing.steps_incumbent_weaker,
            "widest_ratio_held": crossing.widest_ratio_held,
            "switch_ratio": switch_ratio,
        }),
    );
}
