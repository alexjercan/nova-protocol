//! carve_asteroids: shipped weapons against a shipped-size carvable rock.
//!
//! THE GATE for phase 4c of the erosion epic (task 20260813-224826). The old
//! row used 600-damage synthetic hits against a radius-1.2 rock. It proved the
//! mesher and hid the player path: a 4-damage PDC round
//! was sub-cell, repeated rounds in one spot were discarded, and a shipped rock
//! died before a visible hole formed.
//!
//! Five copies of the SAME radius-3 fixed-seed rock, left to right:
//!
//! - pristine control;
//! - 300 rounds from a real `pdc_kinetic_turret_section`, held on one point;
//! - one shipped torpedo warhead, as a real blast at its contact-fuze standoff;
//! - a cut walked across one plane until a piece severs;
//! - two PDC bursts held on two separate places of the same face.
//!
//! The PDC column is the load-bearing one. The gallery spawns a player ship,
//! raises the real weapons safety, holds its real trigger and waits until 300
//! rounds have actually reached the rock. The torpedo column spawns the shipped
//! `nova_blast` and lets the pressure pass resolve it, because a point hit at
//! the warhead's number is not a torpedo and photographing one is how the fuze
//! defect went unseen. The cut and the two bursts enter through `apply_damage`.
//!
//! Judge the PDC column first: it must have one unmistakable hole, while the
//! control stays whole. Then check that the torpedo blows a wide bowl out of the
//! face it went off against and that the cut takes the rock's whole middle slice
//! out, leaving a cap above and a cap below as separate BODIES - this column is the only place
//! in the fleet where severing is shown at all. All rocks retain the same
//! silhouette and coarse faceting away from the damage.
//!
//! The two-burst column is the merge-reach gate. Its second burst lands closer
//! to the first crater than that crater has GROWN, which is the exact band a
//! hole used to swallow: the rock has to end up with a bite under EACH aim
//! point, not one round hole centred on the first.
//!
//! Costs are logged with `RUST_LOG=nova_scenario=debug`. With `NOVA_PROBE=1`,
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
    TwoSpots,
}

impl Shot {
    fn name(self) -> &'static str {
        match self {
            Self::Control => "pristine",
            Self::Pdc => "300 real PDC rounds",
            Self::Torpedo => "one shipped torpedo",
            Self::Cut => "cut in two",
            Self::TwoSpots => "two bursts, two places",
        }
    }

    #[cfg(feature = "debug")]
    fn shot_name(self) -> &'static str {
        match self {
            Self::Control => "carve-asteroids-control.png",
            Self::Pdc => "carve-asteroids-pdc.png",
            Self::Torpedo => "carve-asteroids-torpedo.png",
            Self::Cut => "carve-asteroids-cut.png",
            Self::TwoSpots => "carve-asteroids-two-spots.png",
        }
    }
}

/// The row, left to right.
const ROW: [Shot; 5] = [
    Shot::Control,
    Shot::Pdc,
    Shot::Torpedo,
    Shot::Cut,
    Shot::TwoSpots,
];

/// The exact shipped kinetic PDC hit and the sustained-fire acceptance point.
#[cfg(feature = "debug")]
const PDC_DAMAGE: f32 = 4.0;
#[cfg(feature = "debug")]
const PDC_ROUNDS: usize = 300;
#[cfg(feature = "debug")]
const PDC_PAID_DAMAGE: f32 = PDC_DAMAGE * PDC_ROUNDS as f32;

/// The shipped standard torpedo warhead, as `standard_section_prototypes`
/// authors it.
#[cfg(feature = "debug")]
const TORPEDO_DAMAGE: f32 = 750.0;
/// The shipped standard torpedo blast radius, from the same bay.
#[cfg(feature = "debug")]
const TORPEDO_BLAST_RADIUS: f32 = 30.0;

/// How far off the rock's skin the torpedo column detonates, in world units.
///
/// The torpedo's own contact fuze (`torpedo_section::projectile::CONTACT_FUZE`),
/// which is what a real one arrives at. It used to be half a blast radius off
/// the rock's CENTRE, which on a rock this size is fifteen units inside a
/// surface that starts at twelve - so the column photographed a crater cut in
/// solid rock nobody could see, and then only because it never spawned a blast
/// at all.
#[cfg(feature = "debug")]
const TORPEDO_STANDOFF: f32 = 1.0;

/// One cut crater, in the rock's own UNIT space.
///
/// Sized against the pattern below rather than against a damage number, because
/// the cut has to land in a band: over the covering radius of
/// [`CUT_RINGS`] or the slab keeps a gap and the rock stays one piece, and under
/// the spacing of it or each crater falls inside the last one's merge reach and
/// nineteen hits collapse into a single round hole.
#[cfg(feature = "debug")]
const CUT_RADIUS: f32 = 1.6;

/// The cut: rings of craters through the rock's middle, `(radius, count)` in
/// the rock's own UNIT space.
///
/// A centre crater plus these tiles the whole y = 0 slice, so the rock is left
/// with a cap above the cut and a cap below it and nothing joining them. The
/// outer ring plus one [`CUT_RADIUS`] has to clear the rock's furthest reach in
/// that plane - about 4.5 units, where the published `radius` is a SCALE over
/// this space and not a distance in it.
#[cfg(feature = "debug")]
const CUT_RINGS: [(f32, usize); 2] = [(1.85, 6), (3.7, 12)];

/// What one cut crater costs, which is [`mark_radius`] run backwards over a
/// [`CUT_RADIUS`] hemisphere at the rock's world scale.
///
/// Derived and not authored: a hand-typed number drifts from the pricing curve
/// the moment either end of it moves, and this cut only severs while the
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

/// How far apart the two-burst column holds its two aim points, in world units.
///
/// Chosen to sit in the band the old merge rule swallowed: further out than a
/// crater may ever reach for a new hit (`MERGE_MAX`, 1u) and well inside the
/// crater the first burst grows to (4.92u). A hole that captures by its own
/// accumulated size eats the second burst whole and stays one round pit; a hole
/// that captures only as far as the last round's own hole keeps a bite under
/// each aim point.
#[cfg(feature = "debug")]
const TWO_SPOT_SEPARATION: f32 = 3.5;

/// Rounds into the first place, then into the second.
///
/// Sized so the first burst outgrows [`TWO_SPOT_SEPARATION`] by a comfortable
/// margin - 4.92u against 3.5u - because there has to be a hole big enough to
/// swallow the second burst before "it did not" is worth photographing. A tenth
/// of what this used to be: rock is now ten times softer, and the old counts
/// would take a bowl two thirds of the way through the rock instead of a pit in
/// its face.
#[cfg(feature = "debug")]
const TWO_SPOT_BURSTS: [usize; 2] = [500, 250];

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

/// Where the two-burst column holds its `place`-th aim point, relative to that
/// rock, both on the face the camera looks at.
///
/// The second is stepped straight sideways off the first rather than resampled
/// from a second direction: the separation is the whole point of the column,
/// and this rock's surface wanders by several units between directions, so a
/// direction chosen by eye lands anywhere. The step leaves it a fraction of a
/// unit off the true surface, which is well inside the crater the first burst
/// has already dug there.
#[cfg(feature = "debug")]
fn two_spot_surface(place: usize) -> Vec3 {
    let first = surface_point(Vec3::Z);
    match place {
        0 => first,
        _ => first + Vec3::X * TWO_SPOT_SEPARATION,
    }
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
///
/// The torpedo column spawns a real [`nova_blast`] rather than calling
/// `apply_damage` with the warhead's number. That shortcut is what let the fuze
/// defect live: a point hit at the surface is not a torpedo, so the column
/// photographed a crater no torpedo could ever have cut and nothing noticed that
/// a real one removed exactly nothing. The blast is spawned where the contact
/// fuze puts it - just off the skin - and the pressure pass, the per-body sum
/// and the crater are all the shipped path's.
#[cfg(feature = "debug")]
fn apply_blast_cases(world: &mut World) {
    if let Some(_node) = rock_node(world, Shot::Torpedo) {
        let surface = surface_point(Vec3::Z);
        let at = column_position(2) + surface + surface.normalize() * TORPEDO_STANDOFF;
        world.spawn((
            nova_blast(TORPEDO_BLAST_RADIUS, TORPEDO_DAMAGE, DamageType::Explosive),
            Transform::from_translation(at),
            // The lifetime the shipped detonation gives its blast: long enough
            // for one fixed tick's overlap set, short enough to be gone before
            // the row is framed.
            TempEntity(0.1),
        ));
        world.flush();
    }

    if let Some(node) = rock_node(world, Shot::TwoSpots) {
        for (place, rounds) in TWO_SPOT_BURSTS.into_iter().enumerate() {
            let at = column_position(4) + two_spot_surface(place);
            for _ in 0..rounds {
                let mut commands = world.commands();
                apply_damage(
                    &mut commands,
                    node,
                    None,
                    PDC_DAMAGE,
                    DamageType::Kinetic,
                    Some(at),
                );
                world.flush();
            }
        }
    }

    if let Some(node) = rock_node(world, Shot::Cut) {
        let damage = cut_damage();
        for local in cut_pattern() {
            let mut commands = world.commands();
            apply_damage(
                &mut commands,
                node,
                None,
                damage,
                DamageType::Kinetic,
                // The pattern is unit space; the rock's node carries the scale.
                Some(column_position(3) + local * ROCK_RADIUS),
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

/// The sustained-fire readout, and the two numbers the merge rules are tuned
/// against.
///
/// The SPREAD is how far a held burst actually wanders on a rock 60 units out -
/// what `MERGE_MAX` has to cover for the burst to read as one hole - and the
/// crater COUNT against the mark budget is how close real fire comes to
/// saturating the list. Both are properties of the gun and the range, not of the
/// carve, so they have to be read off a real burst rather than assumed.
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
    let spread = marks
        .0
        .iter()
        .flat_map(|one| {
            marks
                .0
                .iter()
                .map(move |other| one.at.distance(other.at) * ROCK_RADIUS)
        })
        .fold(0.0f32, f32::max);
    info!(
        "carve asteroids: {} real PDC rounds paid {:.0} damage into {} crater(s), \
         largest radius {largest:.2}u, aim spread {spread:.2}u",
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

/// The torpedo gate: a real warhead has to take real material off a rock.
///
/// The column's failure was silent in every observable it had. A 750-damage
/// point hit priced at the cladding's toughness cut a 0.72 unit crater into a
/// rock gridded at 1.02 unit cells, which is zero cubic units removed and zero
/// triangles changed - and the column never spawned a blast at all, so the fuze
/// that would have put that crater fifteen units inside the rock went unseen too.
/// A crater ADDS surface, so the triangle count against the untouched control is
/// the reading that cannot be faked.
#[cfg(feature = "debug")]
fn report_torpedo_result(world: &mut World) {
    let node = rock_node(world, Shot::Torpedo).expect("the torpedo rock still exists");
    let marks = world
        .get::<DamageMarks>(node)
        .expect("the blast recorded a mark");
    let largest = marks
        .0
        .iter()
        .map(|mark| mark.radius * ROCK_RADIUS)
        .fold(0.0f32, f32::max);

    let triangles = |world: &World, shot: Shot| -> usize {
        let Some(node) = rock_node(world, shot) else {
            return 0;
        };
        let Some(mesh) = world.get::<Mesh3d>(node) else {
            return 0;
        };
        world
            .resource::<Assets<Mesh>>()
            .get(&mesh.0)
            .and_then(|mesh| mesh.indices().map(|indices| indices.len() / 3))
            .unwrap_or(0)
    };
    let control = triangles(world, Shot::Control);
    let holed = triangles(world, Shot::Torpedo);

    info!(
        "carve asteroids: one shipped torpedo left {} crater(s), largest radius {largest:.2}u, \
         {control} -> {holed} triangle(s) against the control",
        marks.0.len(),
    );
    assert!(
        largest > 1.0,
        "the torpedo cut a {largest:.2}u crater, which is under one grid cell"
    );
    assert!(
        holed > control,
        "the torpedo rock draws {holed} triangle(s) against the control's {control}: \
         a crater ADDS surface, so this one removed nothing"
    );
}

/// The merge-reach gate: every aim point the two-burst column held has to have
/// its own bite under it.
///
/// A hole that captures by its own accumulated size swallows the second burst
/// and leaves ONE crater centred on the first aim point, with nothing under the
/// second - which is the defect this column exists to catch.
#[cfg(feature = "debug")]
fn report_two_spot_result(world: &mut World) {
    let node = rock_node(world, Shot::TwoSpots).expect("the two-burst rock still exists");
    let aims = [two_spot_surface(0), two_spot_surface(1)];
    let marks = world
        .get::<DamageMarks>(node)
        .expect("the bursts recorded marks");
    // Marks live in the mesh node's unit space; the aim points are world.
    let craters: Vec<(Vec3, f32)> = marks
        .0
        .iter()
        .map(|mark| (mark.at * ROCK_RADIUS, mark.radius * ROCK_RADIUS))
        .collect();

    info!(
        "carve asteroids: two bursts {:.2}u apart left {} crater(s), radii {}",
        aims[0].distance(aims[1]),
        craters.len(),
        craters
            .iter()
            .map(|(_, radius)| format!("{radius:.2}u"))
            .collect::<Vec<_>>()
            .join(" "),
    );

    for (place, aim) in aims.iter().enumerate() {
        let nearest = craters
            .iter()
            .map(|(at, _)| at.distance(*aim))
            .fold(f32::INFINITY, f32::min);
        assert!(
            nearest < 0.5,
            "place {place} has no bite of its own: nearest crater is {nearest:.2}u away"
        );
    }
}

/// The severing gate: the cut has to leave a second BODY, not just a hole.
///
/// This column is the only place in the fleet where carving is shown severing
/// anything, so a slab that left a rim joining the two halves - or islands that
/// all came back as crumbs - has to fail here instead of passing for a deep
/// crater.
#[cfg(feature = "debug")]
fn report_cut_result(world: &mut World) {
    let cut = column_position(3);
    let mut q_pieces = world.query_filtered::<&GlobalTransform, With<CarvedChunkMarker>>();
    // By place, because every column carves and only this one should sever.
    let reaches: Vec<f32> = q_pieces
        .iter(world)
        .map(|at| at.translation().distance(cut))
        .filter(|reach| *reach < COLUMN_PITCH * 0.5)
        .collect();

    // The crater count rides along because the pattern only severs while its
    // craters stay separate marks: a merge rule that swallowed them would
    // collapse the slab into one round hole.
    let craters = rock_node(world, Shot::Cut)
        .and_then(|node| world.get::<DamageMarks>(node))
        .map_or(0, |marks| marks.0.len());
    info!(
        "carve asteroids: {} cut crater(s) severed {} body(s), {} from the rock's centre",
        craters,
        reaches.len(),
        reaches
            .iter()
            .map(|reach| format!("{reach:.2}u"))
            .collect::<Vec<_>>()
            .join(" "),
    );
    assert!(
        !reaches.is_empty(),
        "the cut severed nothing: it dug a hole where it should have cut the rock in two"
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
    apply_damage(
        &mut commands,
        node,
        None,
        PDC_DAMAGE,
        DamageType::Kinetic,
        Some(at),
    );
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
            // Stood off by the row's own width, so adding a column reframes the
            // establishing shot instead of cropping it.
            let width = row_width();
            nova_protocol::nova_debug::harness::pose_camera(
                world,
                centre + Vec3::new(0.0, width * 0.29, width * 1.35),
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

    let script = ROW
        .iter()
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
        });

    // LAST, after the captures: a gate that fails still leaves the picture that
    // shows why it failed.
    script
        .step("check every aim point kept its own bite")
        .on_enter(report_torpedo_result)
        .on_enter(report_two_spot_result)
        .on_enter(report_cut_result)
        .add()
}

/// End to end, in world units.
fn row_width() -> f32 {
    (ROW.len() as f32 - 1.0) * COLUMN_PITCH
}

/// The middle of the row, which the establishing shot is centred on.
fn row_centre() -> Vec3 {
    Vec3::new(row_width() * 0.5, 0.0, 0.0)
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
            // 0.75, not 1.0: the PDC is a HALF-size section whose only socket is
            // its base plate, at -0.25 in its own frame, and the controller's
            // stock unit-cube socket is at +0.5. A whole-unit step would hang
            // the plate a quarter unit above the face and the graph would
            // report the gun disconnected. A real ship never has to know this -
            // `ships::shared::link_points` moves the HULL's socket out by
            // `PDC_MOUNT_OFFSET` to meet the turret - but a rig hand-built from
            // stock prototypes gets the generic sockets and has to place the
            // turret to meet them.
            position: Vec3::Y * 0.75,
            rotation: Quat::IDENTITY,
            source: SectionSource::Prototype("pdc_kinetic_turret_section".to_string()),
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
