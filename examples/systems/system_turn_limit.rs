//! system_turn_limit: the hull's own metal decides how hard it can keep turning.
//!
//! Two identical nine-cell hulls hold ONE unchanging 90-degree right-turn
//! demand. Neither is flown and neither input ever moves again, so whatever
//! rate they settle at is the flight model's answer, not a pilot's. Partway
//! through, one of them loses its two nose cells.
//!
//! The claim under test is the G-force limit: a hull may pull
//! [`LOAD_LIMIT`] at its furthest face and no more, so a long hull is
//! committed to a gentler turn than a short one. Lose the nose and the arm
//! shortens, which RAISES the rate the survivor may hold - the same rule read
//! the other way.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: turn rate holds at the structural limit` | a held demand settles at `sqrt(LOAD_LIMIT / arm)`, not at whatever the controller gains would run to |
//! | 2 | `outcome: a shortened hull turns harder` | losing the nose shortens the arm and the survivor accelerates to its own higher limit |
//! | 3 | `outcome: the intact hull is unmoved by its neighbour` | the untouched hull holds the rate it already had |
//!
//! The demand is deliberately held THROUGH the damage beat: the interesting
//! failure is a hull that keeps accelerating past its limit because the
//! authority it is handed to shed rate can be spent speeding up instead.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_turn_limit --features debug
//! # look for: `turn limit: both hulls hold ...`,
//! #           `turn limit: the shortened hull turns harder ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use std::f32::consts::FRAC_PI_2;
#[cfg(feature = "debug")]
use std::sync::Arc;

use avian3d::prelude::{AngularVelocity, ComputedCenterOfMass, Rotation};
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_turn_limit")]
#[command(version = "1.0.0")]
#[command(
    about = "Structural turn limit: two held-demand hulls, one of which loses its nose. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

/// The hull that keeps all nine cells.
const INTACT_ID: &str = "intact_hull";

/// The hull whose nose comes off mid-turn.
const SHORTENED_ID: &str = "shortened_hull";

/// The two nose cells the damage beat destroys, by section id.
const NOSE: [&str; 2] = ["hull_-4", "hull_-3"];

/// The demand both hulls hold, forever: 90 degrees to starboard of wherever
/// the hull currently points.
///
/// Relative and re-read every tick, so the attitude error never shrinks and
/// the controller never stops asking for more turn. That is what makes the
/// settled rate a property of the HULL rather than of how far it had left to
/// go.
const TURN_DEMAND: f32 = FRAC_PI_2;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(turn_script());
    }

    app.run()
}

fn range_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
    app.add_systems(
        FixedUpdate,
        drive_turn_demand.before(ControllerSectionSystems::SyncRotationInput),
    );
}

/// The scripted run: hold the turn until both hulls settle, break one nose,
/// then read both hulls again under the unchanged demand.
#[cfg(feature = "debug")]
fn turn_script() -> Script {
    Script::new()
        .step("load both hulls")
        .enter(GameStates::Loading)
        .until(both_hulls_present())
        .deadline(30.0)
        .add()
        .step("hold the turn until both hulls settle")
        .until(elapsed(SETTLE_SECS))
        .add()
        .step("assert both hulls hold their structural rate")
        .on_enter(assert_both_hulls_hold)
        .add()
        .step("break the nose off one hull")
        .on_enter(break_the_nose)
        .until(elapsed(REACT_SECS))
        .add()
        .step("assert the shortened hull turns harder")
        .on_enter(assert_the_shortened_hull_turns_harder)
        .add()
}

/// How long a hull is given to reach its held-demand rate.
///
/// The authority a hull has left tapers to nothing as it approaches its own
/// structural rate, so the approach is an asymptote and the window has to be
/// generous. Six seconds is an order of magnitude more than the shipped
/// controller's 0.5 s steering lag.
#[cfg(feature = "debug")]
const SETTLE_SECS: f32 = 6.0;

/// How long the shortened hull is given to climb to its NEW rate after the
/// nose comes off.
///
/// Shorter than [`SETTLE_SECS`] because it starts from the old rate rather
/// than from rest, and the gap is a tenth of a turn per second.
#[cfg(feature = "debug")]
const REACT_SECS: f32 = 3.0;

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(turn_range(&game_assets, &sections)));
}

/// The range: two hulls, no player, no AI, offset along Y so their turning
/// circles never meet.
fn turn_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    ScenarioConfig {
        description: "Two identical hulls hold one turn demand; the lower one loses its nose."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    hull_action(sections, INTACT_ID, "Intact Hull", Meters(60.0)),
                    hull_action(sections, SHORTENED_ID, "Nose Lost", Meters(-60.0)),
                ],
                ThreePointRig::around("turn limit", Meters3::ZERO, 12.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "turn_limit_range".to_string(),
            "Structural Turn Limit".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// One nine-cell line: seven reinforced hull cells, a flight computer and a
/// thruster, mounted nose-forward along -Z.
///
/// A line and not a shipped hull because the arm is the whole subject: nine
/// cells in a row put the furthest face far enough out that the structural
/// limit is well below the rate the controller gains alone would run to, and
/// removing two of them moves it by a fifth.
fn hull_action(sections: &GameSections, id: &str, name: &str, y: Meters) -> EventActionConfig {
    let section = |kind: &str| {
        sections
            .get_section(kind)
            .unwrap_or_else(|| panic!("section '{kind}' is in the base catalog"))
            .clone()
    };
    let cells = [
        ("hull_-4", -4, "reinforced_hull_section"),
        ("hull_-3", -3, "reinforced_hull_section"),
        ("hull_-2", -2, "reinforced_hull_section"),
        ("hull_-1", -1, "reinforced_hull_section"),
        ("hull_0", 0, "reinforced_hull_section"),
        ("hull_1", 1, "reinforced_hull_section"),
        ("hull_2", 2, "reinforced_hull_section"),
        ("controller", 3, "basic_controller_section"),
        ("thruster", 4, "basic_thruster_section"),
    ];
    let hull = cells
        .into_iter()
        .map(|(cell_id, cell, kind)| SpaceshipSectionConfig {
            id: cell_id.to_string(),
            // Build-grid cells, which is the one authored vector that is not a
            // distance.
            position: Vec3::new(0.0, 0.0, cell as f32),
            rotation: Quat::IDENTITY,
            source: SectionSource::Inline(section(kind)),
            modifications: vec![],
        })
        .collect();

    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position: Meters3::new(0.0, y.get(), 0.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance: None,
            hull: ShipSource::Inline(ShipHull {
                sections: hull,
                skin: true,
                style: Some("industrial".to_string()),
                // The shortened hull must SURVIVE losing two of nine cells:
                // this range is about what a damaged hull flies like, not
                // about when one comes apart.
                collapse_threshold: Some(0.0),
                ..default()
            }),
            ..default()
        }),
    })
}

/// Hold the same relative turn on every flight computer, every fixed tick.
///
/// Written into [`ControllerSectionRotationInput`], the seam the mouse, the AI
/// and the autopilot all write, so the range drives the controller exactly as
/// a pilot leaning on the stick would.
fn drive_turn_demand(
    hulls: Query<&Rotation, With<SpaceshipRootMarker>>,
    mut controllers: Query<
        (&ChildOf, &mut ControllerSectionRotationInput),
        With<ControllerSectionMarker>,
    >,
) {
    for (parent, mut input) in &mut controllers {
        if let Ok(rotation) = hulls.get(parent.parent()) {
            input.0 = rotation.0 * Quat::from_rotation_y(TURN_DEMAND);
        }
    }
}

/// What one hull is doing, and what its own metal allows.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug)]
struct HullReading {
    /// Live cells, so the damage beat is visible in the reading itself.
    cells: usize,
    /// Centre of mass to the furthest live face.
    arm: Meters,
    /// `sqrt(LOAD_LIMIT / arm)`, rad/s - the rate at which the centripetal
    /// load alone spends the whole budget.
    sustained: f32,
    /// The rate the hull is actually turning at, rad/s.
    spin: f32,
    /// What the flight computers were handed this tick, rad/s2.
    authority: f32,
}

/// Read one hull off the world, deriving its limit from its own live geometry
/// rather than from anything the flight model wrote.
#[cfg(feature = "debug")]
fn read_hull(world: &mut World, id: &str) -> HullReading {
    let root = hull_root(world, id);
    let center = world
        .get::<ComputedCenterOfMass>(root)
        .map_or(Vec3::ZERO, |center| center.0);
    let spin = world
        .get::<AngularVelocity>(root)
        .map_or(0.0, |velocity| velocity.length());
    let cells: Vec<(Vec3, Quat, Vec3)> = world
        .query_filtered::<(&Transform, Option<&SectionCollider>, &ChildOf), (
            With<SectionMarker>,
            Without<SectionInactiveMarker>,
        )>()
        .iter(world)
        .filter(|(_, _, parent)| parent.parent() == root)
        .map(|(transform, collider, _)| {
            (
                transform.translation,
                transform.rotation,
                collider.copied().unwrap_or_default().aabb_half_extents(),
            )
        })
        .collect();
    let authority: f32 = world
        .query_filtered::<(&PDController, &ChildOf), With<ControllerSectionMarker>>()
        .iter(world)
        .filter(|(_, parent)| parent.parent() == root)
        .map(|(pd, _)| pd.max_angular_acceleration)
        .sum();
    // Engine boundary: `structural_arm` measures the hull off its avian
    // colliders, so the arm arrives in world units.
    let arm = Meters::from_engine(structural_arm(center, cells.iter().copied()));
    HullReading {
        cells: cells.len(),
        arm,
        // Torque and inertia deliberately left out of the oracle: only the
        // structural half of the envelope is under test here, and the range
        // must not read its answer off the same numbers the flight model did.
        sustained: AttitudeEnvelope::new(f32::INFINITY, 1.0, arm).sustained_turn_rate(),
        spin,
        authority,
    }
}

#[cfg(feature = "debug")]
fn hull_root(world: &mut World, id: &str) -> Entity {
    world
        .query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()
        .iter(world)
        .find(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("turn limit: hull '{id}' is present"))
}

#[cfg(feature = "debug")]
fn log_hull(label: &str, reading: HullReading) {
    info!(
        "turn limit: {label}: cells={} arm={:.1} m sustained={:.3} rad/s spin={:.3} rad/s \
         authority={:.3} rad/s2",
        reading.cells,
        reading.arm.get(),
        reading.sustained,
        reading.spin,
        reading.authority
    );
}

/// How far a settled hull may sit from its own structural rate, as a fraction
/// of that rate.
///
/// A hull approaches the rate from below on tapering authority and then
/// overshoots by at most one tick of it, so the band has to admit a small
/// excess - both hulls measure inside 0.1% of their own rate. It stays narrow
/// enough to exclude the failure this range exists for: the controller gains
/// alone run these hulls to pi rad/s, 2.4x the intact limit.
#[cfg(feature = "debug")]
const RATE_TOLERANCE: f32 = 0.05;

/// The rate a hull must clear to count as turning at all, as a fraction of its
/// structural rate. A hull that never left rest would otherwise pass the
/// "not above the limit" half of the claim for free.
#[cfg(feature = "debug")]
const TURNING_FLOOR: f32 = 0.5;

#[cfg(feature = "debug")]
fn assert_holds_its_limit(reading: HullReading, label: &str) {
    assert!(
        reading.spin > reading.sustained * TURNING_FLOOR,
        "turn limit ({label}): the hull is barely turning at {:.3} rad/s under a held \
         90-degree demand; nothing about a limit can be read off a hull that never moved",
        reading.spin
    );
    let excess = (reading.spin - reading.sustained) / reading.sustained;
    assert!(
        excess.abs() <= RATE_TOLERANCE,
        "turn limit ({label}): a hull with a {:.1} m arm may hold {:.3} rad/s \
         ({} m/s2 at its furthest face), but it is turning at {:.3} rad/s - \
         {:+.0}% of its own structural limit",
        reading.arm.get(),
        reading.sustained,
        LOAD_LIMIT.get(),
        reading.spin,
        excess * 100.0
    );
}

/// Beat 3: neither hull has been touched, so both must sit on the same limit -
/// the one their shared geometry gives them, not the one their controller
/// gains would run to.
#[cfg(feature = "debug")]
fn assert_both_hulls_hold(world: &mut World) {
    let intact = read_hull(world, INTACT_ID);
    let shortened = read_hull(world, SHORTENED_ID);
    log_hull("intact, before", intact);
    log_hull("to be shortened, before", shortened);

    assert_holds_its_limit(intact, "intact");
    assert_holds_its_limit(shortened, "before the nose comes off");
    info!(
        "turn limit: both hulls hold {:.3} rad/s against a {:.3} rad/s limit",
        (intact.spin + shortened.spin) / 2.0,
        intact.sustained
    );

    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: turn rate holds at the structural limit",
        serde_json::json!({
            "t": elapsed,
            "arm_m": intact.arm.get(),
            "sustained_rad_per_s": intact.sustained,
            "intact_spin_rad_per_s": intact.spin,
            "other_spin_rad_per_s": shortened.spin,
        }),
    );
    world.insert_resource(BeforeTheNose {
        intact: intact.spin,
        sustained: intact.sustained,
    });
}

/// What the hulls read before the damage beat, so the comparison after it is
/// against a measurement rather than against a constant.
#[cfg(feature = "debug")]
#[derive(Resource, Clone, Copy)]
struct BeforeTheNose {
    intact: f32,
    sustained: f32,
}

/// Destroy the two nose cells of one hull, mid-turn, with the demand
/// unchanged.
#[cfg(feature = "debug")]
fn break_the_nose(world: &mut World) {
    let root = hull_root(world, SHORTENED_ID);
    let nose: Vec<Entity> = world
        .query_filtered::<(Entity, &ChildOf, &EntityId), With<SectionMarker>>()
        .iter(world)
        .filter(|(_, parent, id)| parent.parent() == root && NOSE.contains(&id.0.as_str()))
        .map(|(entity, _, _)| entity)
        .collect();
    assert_eq!(
        nose.len(),
        NOSE.len(),
        "turn limit: the shortened hull must still have both nose cells to lose"
    );
    for cell in nose {
        world.trigger(HealthApplyDamage {
            entity: cell,
            source: None,
            amount: 1.0e6,
        });
    }
    info!("turn limit: destroyed {} and {}", NOSE[0], NOSE[1]);
}

/// Beat 5: the survivor is shorter, so its limit is higher, so it must be
/// turning harder - on an input that never changed.
#[cfg(feature = "debug")]
fn assert_the_shortened_hull_turns_harder(world: &mut World) {
    let before = *world.resource::<BeforeTheNose>();
    let intact = read_hull(world, INTACT_ID);
    let shortened = read_hull(world, SHORTENED_ID);
    log_hull("intact, after", intact);
    log_hull("shortened, after", shortened);

    assert_eq!(
        shortened.cells,
        intact.cells - NOSE.len(),
        "turn limit: the damage beat must have removed exactly the two nose cells"
    );
    assert!(
        shortened.arm < intact.arm,
        "turn limit: losing the nose must shorten the arm, {:.1} m against {:.1} m",
        shortened.arm.get(),
        intact.arm.get()
    );
    assert!(
        shortened.sustained > before.sustained,
        "turn limit: a shorter arm must raise the rate the hull may hold, \
         {:.3} rad/s against {:.3} rad/s",
        shortened.sustained,
        before.sustained
    );

    assert_holds_its_limit(shortened, "shortened");
    assert!(
        shortened.spin > intact.spin,
        "turn limit: the shortened hull turns at {:.3} rad/s and the intact one at \
         {:.3} rad/s; on an unchanged demand the shorter hull is the one that may \
         turn harder",
        shortened.spin,
        intact.spin
    );

    assert_holds_its_limit(intact, "intact, after its neighbour was hit");
    assert!(
        (intact.spin - before.intact).abs() <= before.sustained * RATE_TOLERANCE,
        "turn limit: the intact hull moved from {:.3} to {:.3} rad/s while nothing \
         touched it",
        before.intact,
        intact.spin
    );

    info!(
        "turn limit: the shortened hull turns harder: {:.3} rad/s over a {:.1} m arm \
         against {:.3} rad/s over {:.1} m ({:.0}% faster)",
        shortened.spin,
        shortened.arm.get(),
        intact.spin,
        intact.arm.get(),
        (shortened.spin / intact.spin - 1.0) * 100.0
    );

    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: a shortened hull turns harder",
        serde_json::json!({
            "t": elapsed,
            "arm_m": shortened.arm.get(),
            "sustained_rad_per_s": shortened.sustained,
            "spin_rad_per_s": shortened.spin,
            "intact_spin_rad_per_s": intact.spin,
        }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: the intact hull is unmoved by its neighbour",
        serde_json::json!({
            "t": elapsed,
            "before_rad_per_s": before.intact,
            "after_rad_per_s": intact.spin,
        }),
    );
}

/// Both hulls are up and named.
#[cfg(feature = "debug")]
fn both_hulls_present() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .try_query_filtered::<&EntityId, With<SpaceshipRootMarker>>()
            .is_some_and(|mut hulls| {
                let ids: Vec<&str> = hulls.iter(world).map(|id| id.0.as_str()).collect();
                ids.contains(&INTACT_ID) && ids.contains(&SHORTENED_ID)
            })
    })
}
