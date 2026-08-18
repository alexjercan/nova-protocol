//! carve_asteroids: shipped weapons against a shipped-size carvable rock.
//!
//! THE GATE for phase 4c of the erosion epic (task 20260813-224826). The old
//! row used 600-damage synthetic hits against a radius-1.2 rock. It proved the
//! mesher and hid the player path: a 4-damage PDC round
//! was sub-cell, repeated rounds in one spot were discarded, and a shipped rock
//! died before a visible hole formed.
//!
//! Four copies of the SAME radius-3 fixed-seed rock, left to right:
//!
//! - pristine control;
//! - 300 rounds from a real `better_turret_section`, held on one point;
//! - one shipped 750-damage torpedo-scale blast;
//! - a cut walked across one plane until a piece severs.
//!
//! The PDC column is the load-bearing one. The gallery spawns a player ship,
//! raises the real weapons safety, holds its real trigger and waits until 300
//! rounds have actually reached the rock. The torpedo and cut enter
//! through `apply_damage`, the same seam their blast uses.
//!
//! Judge the PDC column first: it must have one unmistakable hole, while the
//! control stays whole. Then check that the torpedo makes a visible one-hit
//! crater and that the cut opens the rock and throws dust off it. The cut's
//! islands are all crumbs at this radius, so no severed BODY is expected here -
//! a carve throws dust, and only an island big enough to be worth simulating
//! becomes a body. All rocks retain the same silhouette and coarse faceting
//! away from the damage.
//!
//! Costs are logged with `RUST_LOG=nova_scenario=debug`. With `NOVA_PERF=1`,
//! the frame-time probe also drives one accumulating 4-damage hit per frame
//! into the PDC rock for the whole capture window.
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

#[cfg(feature = "debug")]
use avian3d::prelude::RigidBody;
use bevy::{platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "carve_asteroids")]
#[command(version = "1.0.0")]
#[command(about = "One rock at five levels of being shot to bits, and one cut in two", long_about = None)]
struct Cli;

/// What is done to each rock in the row, left to right.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shot {
    Control,
    Pdc,
    Torpedo,
    Cut,
}

impl Shot {
    fn name(self) -> &'static str {
        match self {
            Self::Control => "pristine",
            Self::Pdc => "300 real PDC rounds",
            Self::Torpedo => "one 750-damage torpedo",
            Self::Cut => "cut in two",
        }
    }

    #[cfg(feature = "debug")]
    fn shot_name(self) -> &'static str {
        match self {
            Self::Control => "carve-asteroids-control.png",
            Self::Pdc => "carve-asteroids-pdc.png",
            Self::Torpedo => "carve-asteroids-torpedo.png",
            Self::Cut => "carve-asteroids-cut.png",
        }
    }
}

/// The row, left to right.
const ROW: [Shot; 4] = [Shot::Control, Shot::Pdc, Shot::Torpedo, Shot::Cut];

/// The exact shipped kinetic PDC hit and the sustained-fire acceptance point.
#[cfg(feature = "debug")]
const PDC_DAMAGE: f32 = 4.0;
#[cfg(feature = "debug")]
const PDC_ROUNDS: usize = 300;
#[cfg(feature = "debug")]
const PDC_PAID_DAMAGE: f32 = PDC_DAMAGE * PDC_ROUNDS as f32;

/// The shipped standard torpedo blast.
#[cfg(feature = "debug")]
const TORPEDO_DAMAGE: f32 = 750.0;

/// What one hit of the cut costs.
///
/// Prices a crater of 2.44 in the rock's own unit space, just over
/// [`CUT_SPACING`], so the seven overlap into a solid slab about four units
/// thick rather than a row of holes with material between them.
#[cfg(feature = "debug")]
const CUT_DAMAGE: f32 = 4200.0;

/// How far apart the cut's craters sit, in the rock's own unit space.
///
/// The centre blast plus six at this radius covers a disk 4.8 across, which is
/// wider than the rock's furthest reach - so the slab goes all the way through
/// rather than leaving a rim holding the two halves together.
#[cfg(feature = "debug")]
const CUT_SPACING: f32 = 2.4;

/// One noise seed for every rock: the row varies the damage only.
const ROCK_SEED: u32 = 20260817;

/// A common shipped arena size, large enough that one PDC round is sub-cell.
const ROCK_RADIUS: f32 = 3.0;

/// How far apart rocks stand at their real 3.5x-6x geometric reach.
const COLUMN_PITCH: f32 = 42.0;

/// The firing ship sits on the PDC rock's +Z axis, inside the gun's 200u reach.
const PDC_SHIP_STANDOFF: f32 = 60.0;

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
        app.init_resource::<HeldInput>();
        app.init_resource::<PdcLanded>();
        app.add_observer(tally_real_pdc_hits);
        app.add_plugins(
            nova_probe::NovaProbePlugin::default().drive_frametime(sustained_pdc_driver),
        );
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
#[cfg(feature = "debug")]
fn surface_point(direction: Vec3) -> Vec3 {
    let rock = RockHeight::default().with_seed(ROCK_SEED).sampler();
    direction * rock.radius(direction) * ROCK_RADIUS
}

#[cfg(feature = "debug")]
fn rock_node(world: &World, shot: Shot) -> Option<Entity> {
    let index = ROW.iter().position(|candidate| *candidate == shot)?;
    let id = column_id(index);
    world.iter_entities().find_map(|entity| {
        if !entity.contains::<DamageMarks>() {
            return None;
        }
        let root = entity.get::<ChildOf>()?.0;
        (world.get::<EntityId>(root)?.as_str() == id).then_some(entity.id())
    })
}

/// Apply the blast-scale cases. The PDC column is deliberately absent: only a
/// real fired round is allowed to make that hole.
#[cfg(feature = "debug")]
fn apply_blast_cases(world: &mut World) {
    if let Some(node) = rock_node(world, Shot::Torpedo) {
        let at = column_position(2) + surface_point(Vec3::Z);
        let mut commands = world.commands();
        apply_damage(&mut commands, node, None, TORPEDO_DAMAGE, Some(at));
        world.flush();
    }

    if let Some(node) = rock_node(world, Shot::Cut) {
        let count = 7;
        for nth in 0..count {
            let local = match nth {
                0 => Vec3::ZERO,
                _ => {
                    let turn = (nth - 1) as f32 / (count - 1) as f32 * std::f32::consts::TAU;
                    Vec3::new(turn.cos(), 0.0, turn.sin()) * CUT_SPACING * ROCK_RADIUS
                }
            };
            let mut commands = world.commands();
            apply_damage(
                &mut commands,
                node,
                None,
                CUT_DAMAGE,
                Some(column_position(3) + local),
            );
            world.flush();
        }
    }
}

#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct PdcLanded {
    rounds: usize,
    damage: f32,
}

/// Count only shipped turret rounds that reached the PDC rock. Synthetic perf
/// hits carry no source and cannot satisfy the player-path gate.
#[cfg(feature = "debug")]
fn tally_real_pdc_hits(
    hit: On<HealthApplyDamage>,
    q_bullets: Query<(), With<TurretBulletProjectileMarker>>,
    q_nodes: Query<&ChildOf, With<DamageMarks>>,
    q_ids: Query<&EntityId, With<AsteroidMarker>>,
    mut landed: ResMut<PdcLanded>,
) {
    if hit.entity != hit.original_event_target()
        || !hit.source.is_some_and(|source| q_bullets.contains(source))
    {
        return;
    }
    let Ok(ChildOf(root)) = q_nodes.get(hit.entity) else {
        return;
    };
    let Ok(id) = q_ids.get(*root) else {
        return;
    };
    if id.as_str() != column_id(1) {
        return;
    }
    landed.rounds += 1;
    landed.damage += hit.amount;
}

#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct HeldInput {
    combat: bool,
    fire: bool,
}

#[cfg(feature = "debug")]
fn hold_inputs(world: &mut World, _elapsed: f32) {
    let held = world.resource::<HeldInput>();
    let (combat, fire) = (held.combat, held.fire);
    if combat {
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
    }
    if fire {
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
    }
}

#[cfg(feature = "debug")]
fn pin_range_and_aim(world: &mut World) {
    let roots: Vec<Entity> = world
        .iter_entities()
        .filter(|entity| {
            entity.contains::<AsteroidMarker>() || entity.contains::<PlayerSpaceshipMarker>()
        })
        .map(|entity| entity.id())
        .collect();
    for root in roots {
        world.entity_mut(root).insert(RigidBody::Static);
    }

    let target = column_position(1);
    nova_protocol::nova_debug::harness::pose_camera(
        world,
        target + Vec3::new(0.0, 5.0, PDC_SHIP_STANDOFF - 4.0),
        target,
    );
    world.resource_mut::<HeldInput>().combat = true;
}

#[cfg(feature = "debug")]
fn weapons_are_hot() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&WeaponsHot, With<PlayerSpaceshipMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|hot| hot.0))
    })
}

#[cfg(feature = "debug")]
fn pdc_rounds_landed() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<PdcLanded>()
            .is_some_and(|landed| landed.damage >= PDC_PAID_DAMAGE)
    })
}

#[cfg(feature = "debug")]
fn open_fire(world: &mut World) {
    world.resource_mut::<HeldInput>().fire = true;
}

#[cfg(feature = "debug")]
fn cease_fire(world: &mut World) {
    let mut held = world.resource_mut::<HeldInput>();
    held.fire = false;
    held.combat = false;
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Right);
}

#[cfg(feature = "debug")]
fn report_pdc_result(world: &mut World) {
    let node = rock_node(world, Shot::Pdc).expect("the PDC rock still exists");
    let landed = world.resource::<PdcLanded>();
    let marks = world
        .get::<DamageMarks>(node)
        .expect("actual rounds recorded marks");
    let largest = marks
        .0
        .iter()
        .map(|mark| mark.radius * ROCK_RADIUS)
        .fold(0.0f32, f32::max);
    info!(
        "carve asteroids: {} real PDC rounds paid {:.0} damage into {} crater(s), largest radius {largest:.2}u",
        landed.rounds,
        landed.damage,
        marks.0.len(),
    );
    assert!(
        landed.damage >= PDC_PAID_DAMAGE && landed.rounds >= PDC_ROUNDS,
        "the real PDC landed {} round(s) for {:.0} damage",
        landed.rounds,
        landed.damage
    );
}

#[cfg(feature = "debug")]
fn remove_firing_ship(world: &mut World) {
    let roots: Vec<Entity> = world
        .iter_entities()
        .filter(|entity| entity.contains::<PlayerSpaceshipMarker>())
        .map(|entity| entity.id())
        .collect();
    for root in roots {
        world.entity_mut(root).despawn();
    }
}

/// Worst-case perf drive: one paid sub-cell PDC hit every rendered frame. The
/// field changes every frame; expensive geometry work should not.
#[cfg(feature = "debug")]
fn sustained_pdc_driver(world: &mut World, _frame: u32) {
    let Some(node) = rock_node(world, Shot::Pdc) else {
        return;
    };
    let at = column_position(1) + surface_point(Vec3::Z);
    let mut commands = world.commands();
    apply_damage(&mut commands, node, None, PDC_DAMAGE, Some(at));
    world.flush();
}

#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

#[cfg(feature = "debug")]
fn gallery_script() -> Script {
    let script = Script::new()
        .input(hold_inputs)
        .step("load the shipped-size row and firing ship")
        .enter(GameStates::Loading)
        .until(scenario_camera_present())
        .deadline(30.0)
        .add()
        .step("let the range assemble")
        .until(elapsed(1.0))
        .add()
        .step("pin the range and aim at the PDC rock")
        .on_enter(pin_range_and_aim)
        .until(weapons_are_hot())
        .deadline(10.0)
        .add()
        .step("hold actual PDC fire on one point")
        .on_enter(open_fire)
        .until(pdc_rounds_landed())
        .deadline(15.0)
        .add()
        .step("cease fire and let the last rounds land")
        .on_enter(cease_fire)
        .until(elapsed(1.0))
        .add()
        .step("remove the firing ship and apply blast cases")
        .on_enter(report_pdc_result)
        .on_enter(remove_firing_ship)
        .on_enter(apply_blast_cases)
        .add()
        .step("let the fields and severed piece settle")
        .until(elapsed(1.0))
        .add()
        .step("frame the whole row")
        .on_enter(|world: &mut World| {
            let centre = row_centre();
            nova_protocol::nova_debug::harness::pose_camera(
                world,
                centre + Vec3::new(0.0, 36.0, 170.0),
                centre,
            );
        })
        .until(elapsed(0.8))
        .add()
        .step("capture the whole row")
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
                .step("capture the rock")
                .on_enter(move |world: &mut World| {
                    nova_protocol::nova_debug::harness::shoot(world, name)
                })
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
    // Look down the PDC's firing line. A side view turns a deep tunnel into a
    // shallow silhouette change and lets the gate hide the hole it made.
    let firing_line = Vec3::new(0.0, 5.0, PDC_SHIP_STANDOFF - 4.0).normalize();
    nova_protocol::nova_debug::harness::pose_camera(world, centre + firing_line * 42.0, centre);
}

fn rock(game_assets: &GameAssets, index: usize) -> ScenarioObjectConfig {
    let shot = ROW[index];
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: column_id(index),
            name: shot.name().to_string(),
            position: column_position(index),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: ROCK_RADIUS,
            texture: game_assets.asteroid_texture.clone().into(),
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

fn firing_ship() -> ScenarioObjectConfig {
    let sections = vec![
        SpaceshipSectionConfig {
            id: "controller".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            source: SectionSource::Prototype("basic_controller_section".to_string()),
            modifications: vec![],
        },
        SpaceshipSectionConfig {
            id: "hull".to_string(),
            position: Vec3::Z,
            rotation: Quat::IDENTITY,
            source: SectionSource::Prototype("reinforced_hull_section".to_string()),
            modifications: vec![],
        },
        SpaceshipSectionConfig {
            id: "pdc".to_string(),
            position: Vec3::Y,
            rotation: Quat::IDENTITY,
            source: SectionSource::Prototype("better_turret_section".to_string()),
            modifications: vec![],
        },
    ];
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "pdc_ship".to_string(),
            name: "PDC firing rig".to_string(),
            position: column_position(1) + Vec3::Z * PDC_SHIP_STANDOFF,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            hull: ShipSource::Inline(ShipHull {
                sections,
                ..default()
            }),
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: HashMap::from([("pdc".to_string(), vec![MouseButton::Left.into()])]),
                speed_cap: None,
                infinite_ammo: true,
            }),
            ..default()
        }),
    }
}

fn gallery(game_assets: &GameAssets) -> ScenarioConfig {
    let mut objects: Vec<EventActionConfig> = (0..ROW.len())
        .map(|index| EventActionConfig::SpawnScenarioObject(rock(game_assets, index)))
        .collect();
    objects.push(EventActionConfig::SpawnScenarioObject(firing_ship()));

    ScenarioConfig {
        description: "Real PDC fire, one torpedo and one severing cut.".to_string(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: [
                objects,
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
