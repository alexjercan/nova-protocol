//! carve_asteroids: one rock, shot progressively to bits, and one cut in two.
//!
//! THE GATE for phase 4 of the erosion epic (task 20260813-224826). A ship's
//! cladding carves one cell deep and then stops on the glTF hull underneath; a
//! rock is solid all the way down, so this is where a carve gets to be as deep
//! as the hit deserves and where the representation is really on trial.
//!
//! Six copies of the SAME fixed-seed rock, left to right. Five have taken 0, 1,
//! 3, 6 and 12 hits at points scattered over their own surfaces; the sixth is
//! CUT IN TWO by a salvo walked across one plane through it. Nothing is poked
//! directly: each hit goes through `apply_damage` with a world hit point,
//! exactly as a turret round and a torpedo blast do, so what the row shows is
//! what a fight would do.
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
//! - The CUT column: the half the slab severs is a rigid body of its own from
//!   the moment it comes free, drifting away on the motion it inherited. If it
//!   sits welded in place, connectivity is not being checked; if it appears
//!   somewhere else or at the wrong size, its frame is wrong.
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
//!   whole row), one `carve-asteroids-<hits>.png` per scattered rock, and
//!   `carve-asteroids-cut.png` for the severed one.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "carve_asteroids")]
#[command(version = "1.0.0")]
#[command(about = "One rock at five levels of being shot to bits, and one cut in two", long_about = None)]
struct Cli;

/// What is done to each rock in the row, left to right.
#[derive(Clone, Copy)]
enum Shot {
    /// Craters scattered over the whole surface: what a firefight does to a
    /// rock it is not trying to break.
    Scatter(usize),
    /// Craters walked across ONE PLANE through the rock, so their union is a
    /// slab and what is above it comes free. A torpedo resolves its blast at a
    /// point in space rather than on a surface, so a salvo walked across a rock
    /// is what this is - and it is the only thing in the row that severs.
    ///
    /// A ring of surface craters cannot do it. Cutting deep enough to reach the
    /// axis means each crater is nearly as wide as the rock, and their union
    /// then swallows the caps it was supposed to leave behind: the rock does not
    /// come apart, it goes away.
    Cut(usize),
}

impl Shot {
    /// How many hits this column takes.
    fn hits(self) -> usize {
        match self {
            Shot::Scatter(hits) | Shot::Cut(hits) => hits,
        }
    }

    /// What one of its hits costs, in health.
    fn damage(self) -> f32 {
        match self {
            Shot::Scatter(_) => HIT_DAMAGE,
            Shot::Cut(_) => CUT_DAMAGE,
        }
    }

    /// What the column is called in the shot.
    fn name(self) -> String {
        match self {
            Shot::Scatter(hits) => format!("{hits} hit(s)"),
            Shot::Cut(hits) => format!("cut, {hits} hit(s)"),
        }
    }

    /// The capture this column is shot into.
    fn shot_name(self) -> String {
        match self {
            Shot::Scatter(hits) => format!("carve-asteroids-{hits}.png"),
            Shot::Cut(_) => "carve-asteroids-cut.png".to_string(),
        }
    }

    /// Where its `nth` hit lands, relative to the rock's own centre, in world
    /// units.
    fn hit_at(self, nth: usize) -> Vec3 {
        match self {
            // The golden-angle spiral over the rock's own surface, so hits land
            // all over it rather than bunching.
            Shot::Scatter(count) => {
                let height = 1.0 - 2.0 * (nth as f32 + 0.5) / count.max(1) as f32;
                let ring = (1.0 - height * height).max(0.0).sqrt();
                let turn = nth as f32 * 2.399_963_2;
                let direction =
                    Vec3::new(ring * turn.cos(), height, ring * turn.sin()).normalize_or(Vec3::Y);
                surface_point(direction)
            }
            // One blast on the axis and the rest in a ring around it, all in the
            // y = 0 plane: the craters overlap into a slab wider than the rock.
            Shot::Cut(count) => {
                let at = match nth {
                    0 => Vec3::ZERO,
                    _ => {
                        let turn =
                            (nth - 1) as f32 / (count - 1).max(1) as f32 * std::f32::consts::TAU;
                        Vec3::new(turn.cos(), 0.0, turn.sin()) * CUT_SPACING
                    }
                };
                at * ROCK_RADIUS
            }
        }
    }
}

/// The row, left to right.
///
/// The scatter columns end at 12 and not at the mark budget (24): past a dozen
/// craters the silhouette is mostly crater and the row stops telling anybody
/// anything new. The cut column is the sixth because severance is a different
/// claim from cratering - the piece that comes off is a body of its own.
const ROW: [Shot; 6] = [
    Shot::Scatter(0),
    Shot::Scatter(1),
    Shot::Scatter(3),
    Shot::Scatter(6),
    Shot::Scatter(12),
    Shot::Cut(7),
];

/// What one scattering hit costs, in health.
///
/// Sized so `mark_radius` prices it into a crater about one and a half world
/// units across - a bite a rock this size visibly loses, rather than the
/// pockmark a PDC round would leave.
const HIT_DAMAGE: f32 = 600.0;

/// What one hit of the cut costs.
///
/// Prices a crater of 2.44 in the rock's own unit space, just over
/// [`CUT_SPACING`], so the seven overlap into a solid slab about four units
/// thick rather than a row of holes with material between them.
const CUT_DAMAGE: f32 = 4200.0;

/// How far apart the cut's craters sit, in the rock's own unit space.
///
/// The centre blast plus six at this radius covers a disk 4.8 across, which is
/// wider than the rock's furthest reach - so the slab goes all the way through
/// rather than leaving a rim holding the two halves together.
const CUT_SPACING: f32 = 2.4;

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

/// Where a rock's surface actually is along `direction`, in world units.
///
/// Computed from the SAME sampler the mesh is built with rather than from
/// `BodyRadius`: the published radius is the rock's furthest reach, and a hit
/// placed there in a direction where the noise happens to be low would land in
/// empty space and carve nothing.
fn surface_point(direction: Vec3) -> Vec3 {
    let rock = RockHeight::default().with_seed(ROCK_SEED).sampler();
    direction * rock.radius(direction) * ROCK_RADIUS
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
    let mut nodes: Vec<(Entity, Vec3, Shot)> = Vec::new();
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
            let Some(index) = (0..ROW.len()).find(|index| column_id(*index) == id.as_str()) else {
                continue;
            };
            nodes.push((node, column_position(index), ROW[index]));
        }
    }

    for (node, centre, shot) in nodes {
        for nth in 0..shot.hits() {
            let at = centre + shot.hit_at(nth);
            let mut commands = world.commands();
            apply_damage(&mut commands, node, None, shot.damage(), Some(at));
            world.flush();
        }
        info!(
            "carve asteroids: rock at {centre} took {} hit(s)",
            shot.hits()
        );
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

    ROW.iter()
        .enumerate()
        .fold(script, |script, (index, shot)| {
            let name = shot.shot_name();
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
    Vec3::new((ROW.len() as f32 - 1.0) * COLUMN_PITCH * 0.5, 0.0, 0.0)
}

/// Point the scenario camera at one rock, close in.
#[cfg(feature = "debug")]
fn frame_column(world: &mut World, index: usize) {
    let centre = column_position(index);
    nova_protocol::nova_debug::harness::pose_camera(
        world,
        centre + Vec3::new(6.0, 7.5, 17.0),
        centre,
    );
}

fn rock(game_assets: &GameAssets, index: usize) -> ScenarioObjectConfig {
    let shot = ROW[index];
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: column_id(index),
            name: shot.name(),
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
    let rocks: Vec<EventActionConfig> = (0..ROW.len())
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
