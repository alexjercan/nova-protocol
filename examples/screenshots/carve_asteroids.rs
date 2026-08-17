//! carve_asteroids: one rock, shot progressively to bits, five times over.
//!
//! THE GATE for phase 4 of the erosion epic (task 20260813-224826). A ship's
//! cladding carves one cell deep and then stops on the glTF hull underneath; a
//! rock is solid all the way down, so this is where a carve gets to be as deep
//! as the hit deserves and where the representation is really on trial.
//!
//! Five copies of the SAME fixed-seed rock, left to right, having taken 0, 1,
//! 3, 6 and 12 hits at points scattered over their own surfaces. Nothing is
//! poked directly: each hit goes through `apply_damage` with a world hit point,
//! exactly as a turret round does, so what the row shows is what a fight would
//! do.
//!
//! What to judge:
//!
//! - Does a carved rock still read as THE SAME ROCK? The field is seeded from
//!   the same noise the shipped mesh is displaced by, so the first remesh
//!   should reproduce the silhouette rather than swap in a different one. A
//!   visible change between the control and the once-hit rock ANYWHERE except
//!   the crater is a bug in that translation.
//! - Do the craters read as craters, or as dents? A rock can be carved deeper
//!   than a hull plate can, and this is the first place that shows.
//! - Is the FACETING still the game's faceting? The mesher emits flat per-face
//!   normals at a coarse resolution on purpose. If the carved rock looks
//!   smoother than its neighbours, the resolution is too high, not too low.
//! - The SPEW: every carve throws shards out of the crater. Do they read as
//!   material coming off, or as sparks?
//!
//! Costs are logged rather than guessed - run with
//! `RUST_LOG=nova_scenario=debug` to see the seeding, remesh and collider-build
//! times per rock, which is what the async-offload decision should be made on.
//!
//! Hand-run:
//! ```text
//! cargo run --example carve_asteroids --features debug
//! ```
//!
//! Harnessed, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: load the row, shoot it, frame it, exit clean.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot `carve-asteroids.png` (the
//!   whole row) and one `carve-asteroids-<hits>.png` per rock.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "carve_asteroids")]
#[command(version = "1.0.0")]
#[command(about = "One rock at five levels of being shot to bits", long_about = None)]
struct Cli;

/// How many hits each rock in the row has taken.
///
/// Ends at 12 and not at the mark budget (24): past a dozen craters the
/// silhouette is mostly crater and the row stops telling anybody anything new.
const HITS: [usize; 5] = [0, 1, 3, 6, 12];

/// What one hit costs, in health.
///
/// Sized so `mark_radius` prices it into a crater about one and a half world
/// units across - a bite a rock this size visibly loses, rather than the
/// pockmark a PDC round would leave.
const HIT_DAMAGE: f32 = 600.0;

/// One noise seed for every rock: the row varies the damage only.
const ROCK_SEED: u32 = 20260817;

/// Nominal radius of the row rocks. The noise reaches several times past the
/// unit sphere, so this draws roughly 7 units across.
const ROCK_RADIUS: f32 = 1.2;

/// Health well past what the row spends, so no rock dies mid-gallery and takes
/// its own column out of the shot.
const ROCK_HEALTH: f32 = 100_000.0;

/// How far apart the rocks stand.
const COLUMN_PITCH: f32 = 16.0;

/// The scenario id each rock is spawned under.
fn column_id(index: usize) -> String {
    format!("rock_{index}")
}

/// Where a rock stands.
fn column_position(index: usize) -> Vec3 {
    Vec3::new(index as f32 * COLUMN_PITCH, 0.0, 0.0)
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(gallery_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_gallery);
}

fn setup_gallery(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(gallery(&game_assets)));
}

/// The `nth` of `count` directions spread evenly over the sphere.
///
/// The golden-angle spiral, so a rock's hits land all over it rather than
/// bunching, and DETERMINISTICALLY, so the row is the same row on every run.
fn hit_direction(nth: usize, count: usize) -> Vec3 {
    let height = 1.0 - 2.0 * (nth as f32 + 0.5) / count as f32;
    let ring = (1.0 - height * height).max(0.0).sqrt();
    // The golden angle, which is what keeps successive points from lining up.
    let turn = nth as f32 * 2.399_963_2;
    Vec3::new(ring * turn.cos(), height, ring * turn.sin()).normalize_or(Vec3::Y)
}

/// Where a rock's surface actually is along `direction`, in world units.
///
/// Computed from the SAME sampler the mesh is built with rather than from
/// `BodyRadius`: the published radius is the rock's furthest reach, and a hit
/// placed there in a direction where the noise happens to be low would land in
/// empty space and carve nothing.
fn surface_point(direction: Vec3) -> Vec3 {
    let planet = PlanetHeight::default().with_seed(ROCK_SEED).sampler();
    let height = planet.get_point(direction) as f32;
    direction * (1.0 + height) * ROCK_RADIUS
}

/// Shoot every rock in the row the number of times its column stands for.
///
/// Through `apply_damage` - the one path a turret, a torpedo and a ram all use
/// - rather than by writing marks. If a scripted hit and a fired round did not
/// produce the same crater the seam would be wrong, and a gallery that wrote
/// marks directly would be hiding exactly that.
#[cfg(feature = "debug")]
fn shoot_the_row(world: &mut World) {
    // The carvable node is the CHILD that carries the marks, not the asteroid
    // root: the root is the rigid body, the child is the mesh and collider.
    let mut nodes: Vec<(Entity, Vec3, usize)> = Vec::new();
    {
        let mut q_nodes = world.query_filtered::<(Entity, &ChildOf), With<DamageMarks>>();
        let mut roots = world.query::<&EntityId>();
        let found: Vec<(Entity, Entity)> = q_nodes
            .iter(world)
            .map(|(node, ChildOf(root))| (node, *root))
            .collect();
        for (node, root) in found {
            let Ok(id) = roots.get(world, root) else {
                continue;
            };
            let Some(index) = (0..HITS.len()).find(|index| column_id(*index) == id.as_str()) else {
                continue;
            };
            nodes.push((node, column_position(index), HITS[index]));
        }
    }

    for (node, centre, hits) in nodes {
        for nth in 0..hits {
            let at = centre + surface_point(hit_direction(nth, hits.max(1)));
            let mut commands = world.commands();
            apply_damage(&mut commands, node, None, HIT_DAMAGE, Some(at));
            world.flush();
        }
        info!("carve asteroids: rock at {centre} took {hits} hit(s)");
    }
}

#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

#[cfg(feature = "debug")]
fn gallery_script() -> Script {
    let script = Script::new()
        .step("load the row")
        .enter(GameStates::Loading)
        .until(scenario_camera_present())
        .deadline(25.0)
        .add()
        .step("let the rocks settle")
        .until(elapsed(1.0))
        .add()
        .step("shoot the row")
        .on_enter(shoot_the_row)
        .add()
        // The field is seeded on one frame and carved on the next, so a rock
        // needs a handful of frames to reach its final shape - measured at
        // about 150 ms for the whole row. The rest of this wait is the SPEW:
        // shards live 2.5 s, so the row has to be framed and shot INSIDE that
        // or the capture shows craters with nothing coming out of them.
        .step("let the rock come apart")
        .until(elapsed(0.7))
        .add()
        .step("frame the whole row")
        .on_enter(|world: &mut World| {
            let centre = row_centre();
            nova_protocol::nova_debug::harness::pose_camera(
                world,
                centre + Vec3::new(0.0, 10.0, 86.0),
                centre,
            );
        })
        .until(elapsed(0.8))
        .add()
        .step("shoot the whole row")
        .on_enter(|world: &mut World| {
            nova_protocol::nova_debug::harness::shoot(world, "carve-asteroids.png")
        })
        .until(elapsed(0.5))
        .add();

    HITS.iter()
        .enumerate()
        .fold(script, |script, (index, hits)| {
            let name = format!("carve-asteroids-{hits}.png");
            script
                .step("frame the next rock")
                .on_enter(move |world: &mut World| frame_column(world, index))
                .add()
                .step("settle on the rock")
                .until(elapsed(0.6))
                .add()
                .step("shoot the rock")
                .on_enter(move |world: &mut World| {
                    nova_protocol::nova_debug::harness::shoot(world, &name)
                })
                // The capture is handed to the render world and written a frame or
                // two later, so the run must not end on the request.
                .until(elapsed(0.5))
                .add()
        })
}

/// The middle of the row, which the establishing shot is centred on.
fn row_centre() -> Vec3 {
    Vec3::new((HITS.len() as f32 - 1.0) * COLUMN_PITCH * 0.5, 0.0, 0.0)
}

/// Point the scenario camera at one rock, close in.
#[cfg(feature = "debug")]
fn frame_column(world: &mut World, index: usize) {
    let centre = column_position(index);
    nova_protocol::nova_debug::harness::pose_camera(
        world,
        centre + Vec3::new(4.0, 5.0, 11.0),
        centre,
    );
}

fn rock(game_assets: &GameAssets, index: usize) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: column_id(index),
            name: format!("{} hit(s)", HITS[index]),
            position: column_position(index),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: ROCK_RADIUS,
            texture: game_assets.asteroid_texture.clone().into(),
            health: ROCK_HEALTH,
            impact_sound: None,
            destroy_sound: None,
            mass: None,
            invulnerable: false,
            lock_signature: None,
            // One seed for the whole row: the only thing that differs between
            // rocks is what has been shot off them.
            seed: Some(ROCK_SEED),
        }),
    }
}

fn gallery(game_assets: &GameAssets) -> ScenarioConfig {
    let rocks: Vec<EventActionConfig> = (0..HITS.len())
        .map(|index| EventActionConfig::SpawnScenarioObject(rock(game_assets, index)))
        .collect();

    ScenarioConfig {
        description: "One rock at five levels of being shot to bits.".to_string(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: [
                rocks,
                ThreePointRig::around("row", row_centre(), 8.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "carve_asteroids".to_string(),
            "Carve Asteroids".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}
