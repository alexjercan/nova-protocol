//! bug_carve_apply: one cut that severs, and what the SWAP-IN pays for it.
//!
//! The carve path builds a rock's field and its new surface on the compute
//! pool, and a measured run of `carve_asteroids` showed the whole path costing
//! 0.12 ms per frame under sustained PDC fire - except for one frame, which
//! cost 19.24 ms on its own against a 0.02 ms median and was half that run's
//! worst frame. The compute was never the problem. What owned the frame was
//! `collect_asteroid_remeshes`, the step that takes a finished result and makes
//! it observable: a severed piece arrived as a whole [`SignedField`] on the
//! rock's own grid, and the main thread then scanned it for its middle, scanned
//! it again for its volume, meshed it and hulled it - once per piece, and a cut
//! across a rock frees about twenty.
//!
//! So the range CUTS a rock in two and reads the swap. The cut is the shipped
//! gallery's pattern (`carve_asteroids`, the `cut` column): rings of craters
//! through one plane of a fixed-seed radius-3 rock, applied in a single frame,
//! which leaves a cap above, a cap below and a rim of crumbs.
//!
//! TWO named invariants and one recording:
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the cut severed bodies off the rock` | the reproduction happened at all |
//! | 2 | `outcome: the swap takes one grid per rock` | the swap does no geometry |
//! | 3 | `outcome: the swap cost is recorded` | milliseconds, for a reviewer |
//!
//! Invariant 2 is the defect, stated as the COUNT underneath it rather than as
//! a stopwatch. A `SignedField` is a quarter of a megabyte and every question
//! you can ask one is a scan of all of it, so the number of grids the swap
//! takes delivery of IS its geometry bill: one per rock is its own new solid
//! and nothing else, and one per PIECE is the whole defect. It reads the same
//! on any box, which a millisecond does not - see `examples/systems/README.md`.
//!
//! The cost is RECORDED beside it and flagged past [`APPLY_NOTICE_MS`] with a
//! WARN. It is a fact about the host that ran it and it never fails the range.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example bug_carve_apply --features debug
//! # look for: `carve apply: 1 delivery(s) took 1 grid(s) ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use avian3d::prelude::RigidBody;
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "bug_carve_apply")]
#[command(version = "1.0.0")]
#[command(about = "A cut that severs, and the geometry bill its swap-in pays. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario object id of the rock the cut goes through.
const ROCK: &str = "rock";

/// One seed, so the cut frees the same pieces on every run. The pattern below
/// is laid out against THIS silhouette.
const ROCK_SEED: u32 = 20260817;

/// A common shipped arena size (30 m), and the one the gallery's cut column
/// uses. An ENGINE world-unit figure: the cut pattern and the damage pricing
/// below are both world-unit geometry, so the rock's authored radius crosses to
/// meters once, at the config.
const ROCK_RADIUS: f32 = 3.0;

/// One cut crater, in the rock's own UNIT space.
///
/// The gallery's number, and it is a band rather than a size: over the covering
/// radius of [`CUT_RINGS`] and the slab keeps a gap and the rock stays one
/// piece, under the spacing of it and each crater falls inside the last one's
/// merge reach and the whole pattern collapses into one round hole.
#[cfg(feature = "debug")]
const CUT_RADIUS: f32 = 1.6;

/// The cut: rings of craters through the rock's middle, `(radius, count)` in
/// the rock's own UNIT space.
///
/// A centre crater plus these tiles the whole y = 0 slice, so the rock is left
/// with a cap above the cut, a cap below it, and the rim of crumbs that a cut
/// through rock always leaves. The crumbs are the point here: they are what
/// made one apply frame twenty times the work of every other one.
#[cfg(feature = "debug")]
const CUT_RINGS: [(f32, usize); 2] = [(1.85, 6), (3.7, 12)];

/// Where a recorded swap cost is worth a WARN, in milliseconds.
///
/// Read against the measurement this range was built from: the defect put 19.24
/// ms in one frame and every other apply in that run cost 0.04 ms or less. Two
/// milliseconds is two orders above the healthy number and an order under the
/// broken one, so a flag means the SHAPE of the work changed rather than that
/// the host was busy. Same nature as the probe's `fps_within_baseline`: a
/// reference for a reviewer, never a gate.
#[cfg(feature = "debug")]
const APPLY_NOTICE_MS: f32 = 2.0;

/// How long the run gives the cut to seed a field, remesh it off the pool and
/// swap the result in. Generous: each of the three is a frame of its own, and
/// the seed is tens of thousands of noise samples.
#[cfg(feature = "debug")]
const CARVE_SECS: f32 = 20.0;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(cut_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(range(&game_assets)));
}

/// What one cut crater costs, which is the damage pricing run backwards over a
/// [`CUT_RADIUS`] hemisphere at the rock's world scale.
///
/// Derived and not authored: a hand-typed number drifts from the pricing curve
/// the moment either end of it moves, and this cut only severs while its
/// craters are the size the pattern was laid out for.
#[cfg(feature = "debug")]
fn cut_damage() -> f32 {
    let world_radius = CUT_RADIUS * ROCK_RADIUS;
    DAMAGE_PER_UNIT_VOLUME * (2.0 * std::f32::consts::PI / 3.0) * world_radius.powi(3)
}

/// Every place the cut lands, in the rock's own UNIT space.
#[cfg(feature = "debug")]
fn cut_pattern() -> Vec<Vec3> {
    let mut places = vec![Vec3::ZERO];
    for (radius, count) in CUT_RINGS {
        places.extend((0..count).map(|step| {
            let turn = step as f32 / count as f32 * std::f32::consts::TAU;
            Vec3::new(turn.cos(), 0.0, turn.sin()) * radius
        }));
    }
    places
}

/// The rock's root and the mesh node that carries its marks and its field.
#[cfg(feature = "debug")]
fn rock_and_node(world: &World) -> Option<(Entity, Entity)> {
    let mut q_nodes = world.try_query_filtered::<(Entity, &ChildOf), With<DamageMarks>>()?;
    let found: Vec<(Entity, Entity)> = q_nodes
        .iter(world)
        .map(|(node, ChildOf(root))| (*root, node))
        .collect();
    found.into_iter().find(|(root, _)| {
        world
            .get::<EntityId>(*root)
            .is_some_and(|id| id.as_str() == ROCK)
    })
}

#[cfg(feature = "debug")]
fn rock_present() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| rock_and_node(world).is_some())
}

#[cfg(feature = "debug")]
fn pieces_present() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<(), With<CarvedChunkMarker>>()
            .is_some_and(|mut query| query.iter(world).next().is_some())
    })
}

/// Pin the rock and put the whole cut into it in ONE frame.
///
/// Static first, because the pattern is authored in the rock's own space and
/// entered in the world's: a rock that turned between the first crater and the
/// last would take the cut along a different plane and free different pieces,
/// and the range's whole subject is what one known set of pieces costs.
///
/// The marks all land before any of them is carved - the field is not even
/// seeded until the first one arrives - so this is one carve, one remesh and
/// one swap, which is the frame the range is here to read.
#[cfg(feature = "debug")]
fn cut_the_rock(world: &mut World) {
    let (root, node) = rock_and_node(world).expect("carve apply: the range must have its rock");
    world.entity_mut(root).insert(RigidBody::Static);

    let centre = world
        .get::<GlobalTransform>(root)
        .map_or(Vec3::ZERO, GlobalTransform::translation);
    let damage = cut_damage();
    let places = cut_pattern();
    info!(
        "carve apply: cutting {node:?} with {} crater(s) of {damage:.0} damage",
        places.len()
    );
    for local in places {
        let mut commands = world.commands();
        apply_damage(
            &mut commands,
            node,
            None,
            damage,
            DamageType::Kinetic,
            // The pattern is unit space; the rock's node carries the scale.
            Some(centre + local * ROCK_RADIUS),
        );
        world.flush();
    }
}

/// The claim, and the number beside it.
///
/// One grid per delivery is the whole invariant: the rock's own new solid is
/// the only whole grid a swap has any business holding, because it is the only
/// one it stores. A piece's grid is holdable only to ask it questions, and
/// every question is a scan.
#[cfg(feature = "debug")]
fn read_the_swap(world: &mut World) {
    let report = *world.resource::<CarveApplyReport>();
    let (delivered, grids, pieces, millis) =
        (report.delivered, report.grids, report.pieces, report.millis);

    info!(
        "carve apply: {delivered} delivery(s) took {grids} grid(s), cut {pieces} body(s) free, \
         {millis:.2} ms"
    );

    assert!(
        pieces > 0 && delivered > 0,
        "the cut severed nothing: {delivered} delivery(s) left {pieces} body(s), so there is \
         no swap here to measure"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the cut severed bodies off the rock",
        serde_json::json!({ "delivered": delivered, "pieces": pieces }),
    );

    assert_eq!(
        grids, delivered,
        "{grids} whole grid(s) reached the swap for {delivered} rock(s): a severed piece is \
         arriving as a field rather than as finished geometry, and the main thread pays a \
         scan of the whole grid for every question it then asks one"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the swap takes one grid per rock",
        serde_json::json!({ "grids": grids, "deliveries": delivered }),
    );

    nova_probe::probe_marker(
        world,
        "outcome: the swap cost is recorded",
        serde_json::json!({
            "apply_ms": millis,
            "pieces": pieces,
            "notice_ms": APPLY_NOTICE_MS,
        }),
    );
    if millis >= APPLY_NOTICE_MS {
        warn!(
            "carve apply: the swap read expensive ({millis:.2} ms against a \
             {APPLY_NOTICE_MS:.2} ms notice). Host-noisy by nature - judge it against a quiet \
             run before believing it"
        );
    }
}

#[cfg(feature = "debug")]
fn cut_script() -> Script {
    Script::new()
        .step("load the range")
        .enter(GameStates::Loading)
        .until(scenario_camera_present())
        .deadline(30.0)
        .add()
        .step("wait for the rock")
        .until(rock_present())
        .deadline(30.0)
        .add()
        .step("cut the rock in two")
        .on_enter(cut_the_rock)
        .until(pieces_present())
        .deadline(CARVE_SECS)
        .add()
        .step("read what the swap paid")
        .on_enter(read_the_swap)
        .add()
}

fn rock(game_assets: &GameAssets) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ROCK.to_string(),
            name: "Cut rock".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: Meters::from_engine(ROCK_RADIUS),
            texture: game_assets.asteroid_texture.clone().into(),
            material: KIND_ROCK.to_string(),
            destroy_sound: None,
            mass: None,
            invulnerable: false,
            lock_signature: None,
            seed: Some(ROCK_SEED),
        }),
    }
}

fn range(game_assets: &GameAssets) -> ScenarioConfig {
    ScenarioConfig {
        description: "One rock, cut through the middle in a single frame.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![EventActionConfig::SpawnScenarioObject(rock(game_assets))],
                ThreePointRig::around("cut", Meters3::ZERO, 8.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "bug_carve_apply".to_string(),
            "Carve Apply".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}
