//! lesson_combat_reach: the two COMBAT demonstrations about what a gun CANNOT
//! do - `combat_reaches` (a burst that stops existing before it arrives) and
//! `combat_barrel_discipline` (a mount holding fire while its barrels chase
//! the aim).
//!
//! One set, two eyes. Both lessons are the same sentence read at two scales:
//! a round is a thing with a lifetime and a barrel is a thing with a slew
//! rate, and each of them is a limit the player meets as SILENCE - either the
//! stream ends short or it never leaves. So both want one armed hull with real
//! magazines in it, one hostile parked further away than the guns can throw,
//! and nothing else in the pocket to explain away what the battery is doing.
//!
//! The player hull has no controller, the way every unmanaged rig in this
//! repo's ranges is driven (`lesson_combat_rounds`, `loop_vfx_range`): the
//! script writes the two turret inputs the game already has -
//! `TurretSectionTargetInput` for where the mounts are laid and
//! `TurretSectionInput` for the trigger - and everything downstream of those is
//! the shipped path. An unmanaged ship has no `WeaponsHot`, which the stow
//! machine and the fire path both read as HOT, so the mounts deploy at spawn
//! and stay out. Nothing in either frame is a HUD, because neither claim is:
//! the subject is the rounds and the barrels.
//!
//! ## `combat_reaches` is a STILL, and the subject is a GAP
//!
//! Reach is a static fact - 1,000 m/s for a 2.0 s lifetime is 2 km - so what
//! has to be photographed is a DISTANCE, and a distance is read off the two
//! things at its ends. The hostile stands at [`BOAT_POSITION`], 2.7 km down the
//! lane, and the battery is laid on it and held down: the stream runs the
//! [`PDC_REACH`] the authored numbers buy and then simply ends, with 700 m of
//! open space between the last round and the hull it was aimed at.
//!
//! That gap is the whole frame, so the eye is set to make it big. From
//! [`REACH_EYE`], square on the lane at 2.2 km, the sheet is 3.2 km across: the
//! gunship sits 166 px in from the left edge, the stream runs about 1,180 px,
//! and the gap past the end of it is about 410 px of nothing. An oblique eye
//! was tried first and is much worse - at 1.5 km off the lane's near end the
//! foreshortening crushes the same 700 m to 150 px, and a frame whose subject
//! is a gap cannot afford to spend it on perspective.
//!
//! A STILL rather than a sheet, for the reason `lesson_combat_battery`'s
//! point-defence frame is one: a tracer is drawn about 6 m long at this speed
//! (`turret_section::render::tracer_length`), which is four pixels at the full
//! capture width and two in a downscaled loop cell. The thing the lesson is
//! about is the first thing the tiler's downscale destroys.
//!
//! The TORPEDO in the gap is the lesson's other half. Reach is a ladder, and a
//! picture of one rung is a picture of nothing; the hostile is a torpedo boat,
//! so the script has it loose a salvo and commits it to the player exactly the
//! way `lesson_combat_battery` does. The shot is taken while the lead torpedo
//! is still between the end of the stream and the boat that launched it -
//! [`TORPEDO_GAP`] - which puts the two ends of the ladder in one frame:
//! ordnance crossing ground the guns cannot reach. It is also what keeps the
//! salvo out of the 1.5 km point-defence envelope, so no mount is borrowed by
//! the flight computer and re-aimed out of the frame the script laid it in.
//!
//! ## `combat_barrel_discipline` is a LOOP, and the loop is the SILENCE
//!
//! "A mount shoots while it is tracking and holds while it is slewing" is a
//! claim about two states, so a still can only ever show one of them. This
//! sheet holds the hull and the eye still and STEPS the aim: the commanded
//! point sits [`SWING_DEGREES`] off one bow for ten cells and the other bow for
//! ten, and the barrels spend the cells after each step chasing it with the
//! trigger still down and nothing coming out.
//!
//! A step and not a sweep, and the first cut of this frame is why. It eased the
//! bearing across on a cosine and photographed almost nothing: a commanded
//! POINT has no body to measure, so the gate the barrels are graded on is
//! `POINT_AIM_ON_TARGET_RAD` - 0.92 of one degree - and an eased bearing only
//! ever sat still enough to be caught in the two cells either side of its own
//! rest point. Six firing cells out of twenty, and four of those landed past
//! the end of the sheet.
//!
//! Two steps half a period apart is also what makes the sheet WRAP, with no
//! arithmetic and no measured hinge. Frame twenty IS frame zero of the next
//! loop, so whatever the slew turns out to cost, the barrels are in the same
//! state at both ends of the sheet. [`SWING_DEGREES`] then only has to buy a
//! readable SPLIT: both hinges turn at 180 deg/s, so 90 degrees is about half a
//! second of travel, which is five of the twenty cells held and five firing on
//! each side of the loop.
//!
//! The eye is HIGH, at 150 m and 37 degrees over the deck, and that is the
//! other thing the first cut got wrong. Traverse is a rotation in the deck's
//! own plane, so an eye level with the mounts - `lesson_combat_battery`'s
//! magazine framing, borrowed whole - projects the entire swing onto a few
//! pixels of foreshortened barrel against a wall of grey flank. From up here
//! all four dorsal mounts sit on their deck, the swing is a swing, and the
//! cell is wide enough that a round stays in it for a hundred metres of
//! flight.
//!
//! ## What is measured
//!
//! Neither frame is judged on having been written. The walk samples the
//! battery's own magazines once a cell and keeps the differences, which is the
//! rounds the mounts actually SPENT in that cell - so
//! [`the_guns_held_and_came_back`] can say that the sheet holds cells where
//! nothing was fired AND cells where something was, and fail the run rather
//! than ship twenty cells of an uninterrupted burst. The reach frame is graded
//! on the stream itself: the furthest live round has to sit within
//! [`REACH_TOLERANCE`] of [`PDC_REACH`] when the shutter opens, so a frame that
//! captured a half-filled lane is a failed run and not a shorter gun.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole script, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the sheet and shoot the still
//!   (staged under `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_combat_reach --features debug
//! ```

#[path = "shared/hollow.rs"]
mod hollow;
use hollow::{dev_fixtures, kit};
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{lesson_cell, lesson_profile, LESSON_CELL_SECS, LESSON_GRID, LESSON_SECS};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_combat_reach")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's reach and barrel-discipline demonstrations", long_about = None)]
struct Cli;

/// The still for "How far each weapon reaches".
#[cfg(feature = "debug")]
const REACH_SHOT: &str = "combat_reaches.png";
/// The sheet for "Barrel discipline".
#[cfg(feature = "debug")]
const DISCIPLINE_LESSON: &str = "combat_barrel_discipline";

/// Scenario id of the armed hull the frames are shot off.
const PLAYER_ID: &str = "reach_player";
/// Scenario id of the hostile torpedo boat at the far end of the lane.
const BOAT_ID: &str = "reach_boat";

/// How far a PDC round gets: 1,000 m/s for a 2.0 s lifetime
/// (`base_content/sections/turret.rs`). Not authored anywhere as a range -
/// this is the product, and the frame exists to show it as a place.
#[cfg(feature = "debug")]
const PDC_REACH: Meters = Meters(2_000.0);

/// Where the hostile stands: square down the lane, 700 m past the end of the
/// player's reach.
///
/// Far enough that the gap is a third of the stream's own length and reads as
/// deliberate rather than as a round or two falling short, close enough that
/// one framing holds the gunship, the whole stream and the hull it cannot
/// touch.
const BOAT_POSITION: Meters3 = Meters3::new(0.0, 0.0, -2_700.0);

/// Where the reach still's eye stands, and what it looks at.
///
/// Square on the lane at 2.2 km, so nothing about the 2.7 km the frame has to
/// hold is spent on perspective, and 340 m up looking at 190 m so the lane sits
/// in the lower third with the rock field above it rather than cutting the
/// picture in half.
#[cfg(feature = "debug")]
const REACH_EYE: Meters3 = Meters3::new(2_200.0, 340.0, -1_350.0);
/// What the reach still aims at: the lane's midpoint, lifted.
#[cfg(feature = "debug")]
const REACH_AIM: Meters3 = Meters3::new(0.0, 190.0, -1_350.0);

/// How far the furthest live round may sit from [`PDC_REACH`] when the shutter
/// opens.
///
/// Generous on purpose: the mounts are spread over a 105 m hull, so the round
/// that has flown furthest left a muzzle up to half that ahead of the origin
/// the reach is measured from, and a round is only sampled once a frame.
#[cfg(feature = "debug")]
const REACH_TOLERANCE: Meters = Meters(120.0);

/// Where the lead torpedo has to be when the reach still is taken: past the end
/// of the guns' reach and still short of the boat that launched it.
///
/// Both ends are load-bearing. Inside the near end the ordnance is in ground
/// the battery covers, which is the opposite of the frame's claim; past the far
/// end it is still in the tube. The near end also keeps the salvo outside the
/// 1.5 km envelope the flight computer borrows idle mounts to answer, so no
/// mount is re-aimed out of the lane the script laid it in.
#[cfg(feature = "debug")]
const TORPEDO_GAP: (Meters, Meters) = (Meters(2_100.0), Meters(2_620.0));

/// Where the barrel-discipline sheet's eye stands, and what it looks at.
///
/// Where the barrel-discipline sheet's eye stands, and what it looks at.
///
/// High and dead ASTERN: 190 m out and 55 degrees up, on the hull's own axis.
/// TRAVERSE IS A ROTATION IN THE DECK'S OWN PLANE, so an eye up near the deck's
/// normal is the one that shows all of it - the mounts turn through their real
/// angle instead of a foreshortened fraction of it, and every stream leaves the
/// hull radially across open sky.
///
/// On the AXIS, because that is what makes the two bearings read as a pair. The
/// sheet's whole subject is one bearing against the other, and from anywhere
/// off the ship's centreline they project into the cell at different lengths
/// and different angles - one stream running out across the frame and the other
/// foreshortened into the lens. From astern they fan symmetrically, up and out
/// to either side of a hull that stands upright between them.
///
/// Both of those were learned the hard way. The first cut borrowed
/// `lesson_combat_battery`'s magazine framing, level with the mounts, and spent
/// the whole swing on a few pixels of barrel against a wall of grey flank. The
/// second stood 37 degrees up on the starboard quarter, which read well for one
/// of the two bearings and put the other one's rounds STRAIGHT DOWN THROUGH THE
/// HULL - a picture of a ship being shot at, in a lesson about a ship shooting.
///
/// The RANGE is set by the stream, not by the ship. At 190 m the cell is 280 m
/// across, so the gunship stands about half its height and a round stays in
/// frame for well over a hundred metres of flight - long enough to read as a
/// stream leaving rather than as a muzzle flash.
#[cfg(feature = "debug")]
const DISCIPLINE_EYE: Meters3 = Meters3::new(25.0, 162.0, 101.0);
/// What the barrel-discipline sheet aims at: the hull's own centre.
#[cfg(feature = "debug")]
const DISCIPLINE_AIM: Meters3 = Meters3::new(0.0, 6.0, -4.0);

/// How far apart the two bearings the commanded point STEPS between are, in
/// degrees.
///
/// A step and not a sweep, because the lesson's sentence is about a mount
/// CHASING something: a player's crosshair snaps to a new bearing and the
/// barrels spend the next half second getting there. Both hinges turn at
/// 180 deg/s (`base_content/sections/turret.rs`), so this is about half a
/// second of travel - five of the sheet's twenty cells held, and five firing,
/// on each side of the loop.
///
/// The first cut of this frame swept the bearing on a cosine ease instead and
/// photographed almost nothing: the gate a commanded point is graded on is
/// `POINT_AIM_ON_TARGET_RAD`, 0.92 of one degree, and a barrel only ever fell
/// inside that in the two cells either side of the ease's own rest point. Six
/// firing cells out of twenty, four of them past the end of the sheet.
#[cfg(feature = "debug")]
const SWING_DEGREES: f32 = 90.0;

/// How far out the commanded point sits while it walks, and how high.
///
/// The distance is free - a commanded POINT has no body, so the gate is
/// [`POINT_AIM_ON_TARGET_RAD`] at any range - and this is chosen so the rounds
/// have somewhere to go. The height is NOT free: it is about the dorsal
/// mounts' own, so the pair the eye is framed on is laid level rather than
/// craning.
#[cfg(feature = "debug")]
const SWING_RADIUS: Meters = Meters(600.0);
/// The height the commanded point walks at. See [`SWING_RADIUS`].
#[cfg(feature = "debug")]
const SWING_HEIGHT: Meters = Meters(20.0);

/// The fewest cells of the sheet that have to show rounds leaving, and the
/// fewest that have to show none.
///
/// Both halves, because either one alone is a frame that says nothing: a sheet
/// with no silent cell is a gun that never held, and a sheet with no firing
/// cell is a gun that never shot.
#[cfg(feature = "debug")]
const MIN_CELLS_EACH_WAY: usize = 3;

/// How long each of the two settle beats before the sheet holds, in ship
/// seconds.
///
/// [`SETTLE_FRAMES`] read off the CLOCK rather than off the rendered frames.
/// The two are one thing while the recorder pins the frame clock to the
/// handbook's cadence, and nothing alike anywhere else. On a host that renders
/// quickly the thirty frames went by inside the scenario load, so the guns were
/// laid and the trigger put down on a battery that did not exist yet and the
/// sheet recorded a hull that never fired. Under software rendering the same
/// thirty frames held the trigger down for five seconds and emptied the
/// magazines the sheet is graded on. Three seconds is what the recorded lesson
/// was made with, on both counts.
#[cfg(feature = "debug")]
const SETTLE_SECS: f32 = SETTLE_FRAMES as f32 * LESSON_CELL_SECS;

/// How long the battery is held down before the reach still's lane is judged
/// full, in seconds.
///
/// One round lifetime plus margin: the stream cannot be [`PDC_REACH`] long
/// until a round fired at the trigger has lived its whole 2.0 s.
#[cfg(feature = "debug")]
const FILL_SECS: f32 = 2.4;

/// How far the game clock may advance in one frame of a HARNESSED run of this
/// set: one cell of the handbook's own sheet.
///
/// The two frames here are graded on how long the STREAM of rounds is, and a
/// round's life is ticked with the FRAME delta
/// (`nova_gameplay::lifetime::TempEntity`) while its flight is integrated by
/// the solver. So a round dies up to one frame of travel short of the reach it
/// was authored with: 100 m at the recorder's ten cells a second, which is what
/// [`REACH_TOLERANCE`] is sized to absorb, and 250 m on a software rasterizer
/// left at Bevy's quarter-second clamp, which it is not - the lane there tops
/// out at 1,750 m and never reads full, so the beat stalls on the host rather
/// than on a change.
///
/// Holding the clock rather than widening the tolerance is what keeps the
/// still's claim: an unarmed run measures the stream on the same clock the
/// recorded one does. Nothing in this set reads a frame, so a frame that
/// carries less world time runs fewer fixed steps of the same flight, not a
/// different one.
#[cfg(feature = "debug")]
fn hold_the_lane_clock(mut time: ResMut<Time<Virtual>>) {
    let cell = std::time::Duration::from_secs_f32(LESSON_CELL_SECS);
    if time.max_delta() != cell {
        time.set_max_delta(cell);
    }
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        // Held every frame, not set once: a scenario load hands `Time<Virtual>`
        // back at its default.
        if harness_env_active() {
            app.add_systems(First, hold_the_lane_clock.before(bevy::time::TimeSystems));
        }
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::new(
            lesson_profile(),
        ));
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.init_resource::<SpentRounds>();
        app.add_plugins(combat_reach_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    commands.trigger(LoadScenario(the_gun_range(&game_assets, &ships)));
}

/// The set: an armed hull at the origin with the magazines it was authored
/// with, and one hostile torpedo boat parked past the end of its guns.
fn the_gun_range(game_assets: &GameAssets, ships: &GameShipDesigns) -> ScenarioConfig {
    // The AUTHORED magazines rather than `hollow::unlimited_turrets`: the
    // barrel-discipline guard counts the rounds the mounts spend per cell, and
    // a gun with no `SectionAmmo` spends nothing it can be asked about.
    let player = hollow::ship(
        PLAYER_ID,
        "Player Ship",
        Meters3::ZERO,
        // Square with the world. The lane is world -Z, and both frames are
        // measured off a hull that is not turned in it.
        Quat::IDENTITY,
        SpaceshipController::None,
        Some(Allegiance::Player),
        kit::catalog_ship(ships, "block_gunship"),
    );
    let boat = hollow::ship(
        BOAT_ID,
        "Torpedo Boat",
        BOAT_POSITION,
        Transform::from_translation(BOAT_POSITION.to_engine())
            .looking_at(Vec3::ZERO, Vec3::Y)
            .rotation,
        SpaceshipController::None,
        Some(Allegiance::Enemy),
        dev_fixtures::cleanup_leader(),
    );

    ScenarioConfig {
        description: "An armed hull at the origin and one hostile torpedo boat parked past the \
                      end of its guns."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![backdrop().action(game_assets), player, boat],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "the_gun_range".to_string(),
            "The Gun Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The rock shell around the set.
///
/// Drawn around a centre lifted well ABOVE the lane and spread thin: the reach
/// frame's whole subject is 700 m of empty space on that lane, and one rock
/// standing in it would eat the rounds and the claim together. What the field
/// is for is the upper two thirds of the still, which is otherwise 2 km of
/// black with a thread across it.
fn backdrop() -> kit::NearField {
    kit::NearField {
        id_prefix: "reach_rock_",
        count: 34,
        seed: 82_117,
        center: Meters3::new(0.0, 700.0, -1_350.0),
        distance: (Meters(900.0), Meters(2_400.0)),
        radius: (Meters(14.0), Meters(34.0)),
        y_spread: Meters(500.0),
    }
}

/// The rounds the player's battery has spent, sampled once a captured cell.
///
/// Magazine totals rather than round entities: an entity count falls as rounds
/// expire and rises as they leave, so it answers "how full is the lane" and not
/// "did this mount shoot". The magazine only ever goes down while a trigger is
/// held, and the difference between two samples IS the cell's rounds.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct SpentRounds {
    /// Total rounds left in the battery at each sample.
    samples: Vec<u32>,
}

/// Every turret section bolted to the ship carrying scenario id `id`.
///
/// By OWNER rather than by marker: the hostile is built from a catalog hull
/// too, and a trigger written across every `TurretSectionInput` in the world
/// would have the target shooting back in a frame about what the player's guns
/// cannot reach.
#[cfg(feature = "debug")]
fn turrets_of(world: &mut World, id: &str) -> Vec<Entity> {
    let Some(root) = kit::ship_root(world, id) else {
        return vec![];
    };
    world
        .query_filtered::<(Entity, &ChildOf), With<TurretSectionMarker>>()
        .iter(world)
        .filter(|(_, ChildOf(parent))| *parent == root)
        .map(|(turret, _)| turret)
        .collect()
}

/// Lay the player's battery on one world point.
#[cfg(feature = "debug")]
fn lay_the_guns(world: &mut World, aim: Meters3) {
    let aim = aim.to_engine();
    for turret in turrets_of(world, PLAYER_ID) {
        if let Some(mut target) = world
            .entity_mut(turret)
            .get_mut::<TurretSectionTargetInput>()
        {
            **target = Some(aim);
        }
    }
}

/// Hold or release the player's triggers.
#[cfg(feature = "debug")]
fn set_triggers(world: &mut World, firing: bool) {
    for turret in turrets_of(world, PLAYER_ID) {
        if let Some(mut trigger) = world.entity_mut(turret).get_mut::<TurretSectionInput>() {
            **trigger = firing;
        }
    }
}

/// Where the commanded point stands at `cell` of the sheet.
///
/// A SQUARE wave on the cell index, not a curve: the point holds one bearing
/// for the first half of the sheet and the other for the second, and the
/// barrels spend the cells after each step chasing it. Two steps per loop, half
/// a period apart, is also what makes the sheet WRAP without any arithmetic -
/// cell twenty is cell zero of the next loop, so the barrels are in exactly the
/// state there that they were in here, whatever the hinges turn out to cost.
///
/// Indexed on the cell the CLOCK is in ([`lesson_cell`]) rather than on the
/// rendered frame, so the smoke path walks the same two steps at the same
/// speed and the guard below means something unarmed. The two readings agree
/// while the recorder holds the clock at the handbook's cadence.
#[cfg(feature = "debug")]
fn swing_bearing(cell: u32) -> Meters3 {
    let half = LESSON_GRID.frames() / 2;
    let to_starboard = cell % LESSON_GRID.frames() < half;
    let bearing = if to_starboard {
        SWING_DEGREES.to_radians() * 0.5
    } else {
        -SWING_DEGREES.to_radians() * 0.5
    };
    Meters3::new(
        SWING_RADIUS.get() * bearing.sin(),
        SWING_HEIGHT.get(),
        -SWING_RADIUS.get() * bearing.cos(),
    )
}

/// Walk the commanded point to where `cell` of the sheet wants it.
#[cfg(feature = "debug")]
fn swing_the_aim(world: &mut World, cell: u32) {
    lay_the_guns(world, swing_bearing(cell));
}

/// How many rounds the player's battery has left, across every magazine on it.
#[cfg(feature = "debug")]
fn rounds_left(world: &mut World) -> u32 {
    let turrets = turrets_of(world, PLAYER_ID);
    turrets
        .into_iter()
        .filter_map(|turret| world.entity(turret).get::<SectionAmmo>().map(|a| a.rounds))
        .sum()
}

/// Take one sample of the battery's magazines, the first one taken inside each
/// cell of the sheet.
///
/// One per CELL, not one per rendered frame: a frame is a cell only while the
/// recorder holds the clock at the handbook's cadence, and on a run with
/// nothing recording the whole twenty go by in a few tens of milliseconds of
/// ship time - less than the interval between two rounds, so every cell reads
/// as spending nothing and the guard below calls a working battery broken.
///
/// Capped at the sheet's own length. The beat waits on the tiler as well as on
/// the clock, so it outlives the twenty cells by a frame or two, and a sample
/// taken after the sheet closed is a cell nobody will ever see.
#[cfg(feature = "debug")]
fn note_the_rounds(world: &mut World, cell: u32) {
    let left = rounds_left(world);
    let mut spent = world.resource_mut::<SpentRounds>();
    if spent.samples.len() > LESSON_GRID.frames() as usize || spent.samples.len() as u32 > cell {
        return;
    }
    spent.samples.push(left);
}

/// What each captured cell spent, from the samples taken across it.
///
/// Saturating, because a batch landing mid-sheet would raise the total and a
/// signed difference has no meaning here: what is being asked is whether the
/// mounts shot, and a cell that gained rounds shot nothing.
#[cfg(feature = "debug")]
fn spent_per_cell(samples: &[u32]) -> Vec<u32> {
    samples
        .windows(2)
        .map(|pair| pair[0].saturating_sub(pair[1]))
        .collect()
}

/// What to print when the swing beat stalls.
#[cfg(feature = "debug")]
fn report_the_swing(world: &World) -> String {
    let spent = spent_per_cell(&world.resource::<SpentRounds>().samples);
    format!("rounds spent per cell of the swing: {spent:?}")
}

/// The sheet holds cells where the battery fired and cells where it held.
#[cfg(feature = "debug")]
fn the_guns_held_and_came_back(world: &mut World) {
    let spent = spent_per_cell(&world.resource::<SpentRounds>().samples);
    let firing = spent.iter().filter(|rounds| **rounds > 0).count();
    let held = spent.iter().filter(|rounds| **rounds == 0).count();
    let samples = &world.resource::<SpentRounds>().samples;
    info!(
        "barrel discipline: {firing} firing cell(s), {held} held, cell by cell {spent:?}, \
         magazines {samples:?}"
    );
    assert!(
        firing >= MIN_CELLS_EACH_WAY,
        "the battery spent rounds in only {firing} cell(s) of the sheet: the loop would read as a \
         gun that is broken rather than one that is slewing. Check SWING_DEGREES is not so wide \
         that the barrels never catch the point at either end."
    );
    assert!(
        held >= MIN_CELLS_EACH_WAY,
        "the battery fired in all but {held} cell(s) of the sheet: at this swing the barrels are \
         following the point all the way round, so the frame shows no discipline at all. \
         SWING_DEGREES has to peak past the hinges' 180 deg/s."
    );
}

/// How far the furthest live round has flown from the hull that fired it.
#[cfg(feature = "debug")]
fn furthest_round(world: &mut World) -> Meters {
    let furthest = world
        .query_filtered::<&GlobalTransform, With<TurretBulletProjectileMarker>>()
        .iter(world)
        .map(|at| at.translation().length())
        .fold(0.0_f32, f32::max);
    Meters::from_engine(furthest)
}

/// Advance once the lane is full: a round fired at the trigger has lived out
/// its lifetime and the stream is as long as it will ever be.
#[cfg(feature = "debug")]
fn the_lane_is_full() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let furthest = world
            .try_query_filtered::<&GlobalTransform, With<TurretBulletProjectileMarker>>()
            .map_or(0.0, |mut rounds| {
                rounds
                    .iter(world)
                    .map(|at| at.translation().length())
                    .fold(0.0_f32, f32::max)
            });
        Meters::from_engine(furthest).get() >= PDC_REACH.get() - REACH_TOLERANCE.get()
    })
}

/// How far the lead torpedo is from the hull it is driving at.
#[cfg(feature = "debug")]
fn lead_torpedo_range(world: &World) -> Option<Meters> {
    let mut torpedoes =
        world.try_query_filtered::<&GlobalTransform, With<TorpedoProjectileMarker>>()?;
    torpedoes
        .iter(world)
        .map(|at| Meters::from_engine(at.translation().length()))
        .min_by(|a, b| a.get().total_cmp(&b.get()))
}

/// Advance once the lead torpedo has crossed into the gap the guns cannot
/// reach. See [`TORPEDO_GAP`].
#[cfg(feature = "debug")]
fn the_salvo_is_in_the_gap() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        lead_torpedo_range(world).is_some_and(|range| {
            range.get() <= TORPEDO_GAP.1.get() && range.get() >= TORPEDO_GAP.0.get()
        })
    })
}

/// What to print when the reach beat stalls: the two numbers it is waiting on.
#[cfg(feature = "debug")]
fn report_the_lane(world: &World) -> String {
    let furthest = world
        .try_query_filtered::<&GlobalTransform, With<TurretBulletProjectileMarker>>()
        .map_or(0.0, |mut rounds| {
            rounds
                .iter(world)
                .map(|at| at.translation().length())
                .fold(0.0_f32, f32::max)
        });
    let torpedo = lead_torpedo_range(world).map(|range| range.get());
    format!(
        "furthest round {:.0} m, lead torpedo {:?} m",
        Meters::from_engine(furthest).get(),
        torpedo.map(|range| range.round())
    )
}

/// The stream in frame is the reach the authored numbers buy.
#[cfg(feature = "debug")]
fn the_stream_stops_where_it_should(world: &mut World) {
    let furthest = furthest_round(world);
    let error = (furthest.get() - PDC_REACH.get()).abs();
    info!(
        "reach: furthest live round {:.0} m against an authored {:.0} m",
        furthest.get(),
        PDC_REACH.get()
    );
    assert!(
        error <= REACH_TOLERANCE.get(),
        "the furthest live round is {:.0} m out and the lesson names {:.0} m: the still would \
         show a lane that is half filled, or a gun that is not the one the text describes. Check \
         the battery was held down for the whole fill beat.",
        furthest.get(),
        PDC_REACH.get()
    );
}

/// Pull the hostile's torpedo bays. It is the only hull in the set with one.
#[cfg(feature = "debug")]
fn loose_the_salvo(world: &mut World) {
    let bays: Vec<Entity> = world
        .query_filtered::<Entity, With<TorpedoSectionMarker>>()
        .iter(world)
        .collect();
    assert!(
        !bays.is_empty(),
        "no torpedo bays in the set: the reach still would show one rung of a ladder. Check the \
         hostile is still built from the cleanup leader fixture."
    );
    for bay in bays {
        if let Some(mut input) = world.entity_mut(bay).get_mut::<TorpedoSectionInput>() {
            **input = true;
        }
    }
}

/// Commit the salvo to the player and drop the bays' triggers.
///
/// The one write a controllerless hull cannot do for itself: a torpedo's target
/// is chosen exactly once, right after launch, by whoever fired it. Releasing
/// the bays here keeps it to ONE salvo instead of a stream on the bay's own
/// fire-rate clock.
#[cfg(feature = "debug")]
fn commit_the_salvo(world: &mut World) {
    let Some(player) = kit::ship_root(world, PLAYER_ID) else {
        warn!("reach: no player hull to commit the salvo to");
        return;
    };
    let bays: Vec<Entity> = world
        .query_filtered::<Entity, With<TorpedoSectionMarker>>()
        .iter(world)
        .collect();
    for bay in bays {
        if let Some(mut input) = world.entity_mut(bay).get_mut::<TorpedoSectionInput>() {
            **input = false;
        }
    }
    let torpedoes: Vec<Entity> = world
        .query_filtered::<Entity, (With<TorpedoProjectileMarker>, Without<TorpedoTargetChosen>)>()
        .iter(world)
        .collect();
    assert!(
        !torpedoes.is_empty(),
        "the hostile launched nothing: the reach still has no long rung to show. Check the bay \
         carried ammunition and that the trigger beat was long enough to clear the tube."
    );
    for torpedo in &torpedoes {
        world
            .entity_mut(*torpedo)
            .insert((TorpedoTargetChosen, TorpedoTargetEntity(player)));
    }
    info!("reach: {} torpedo(es) committed", torpedoes.len());
}

/// Record a battery holding fire through a swing, then shoot the lane its
/// rounds cannot cross.
#[cfg(feature = "debug")]
fn combat_reach_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        // BOTH, and the build is the half that matters: the scenario camera is
        // up while the loader is still spawning, and the settle beats below are
        // counted in FRAMES, which keep coming through a hold that stops the
        // clock. On a host that renders quickly all of them passed inside the
        // build, so `lay_the_guns` and `set_triggers` wrote to a battery that
        // did not exist yet - the trigger was never put down, and the sheet
        // recorded a hull that fired nothing.
        .step("load the gun range")
        .enter(GameStates::Loading)
        .until(and(scenario_camera_present(), scenario_is_built()))
        .deadline(30.0)
        .add()
        // BARREL DISCIPLINE first, because it is the frame with a magazine
        // budget: it is graded on the rounds the mounts spend, and the reach
        // frame that follows holds the trigger down for seconds.
        // Laid on the bearing the sheet's OWN first frame steps away from, not
        // on the one it steps to. Frame zero is a step, and a hull that walked
        // in already pointing where the step leads spends the first cells
        // firing where the loop will have it chasing - the sheet then wraps
        // from a barrel at one bearing to a barrel at the other.
        .step("frame the dorsal deck and lay the guns on the bearing it steps off")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            swing_the_aim(world, LESSON_GRID.frames() - 1);
            pose_camera(world, DISCIPLINE_EYE, DISCIPLINE_AIM);
        })
        .until(elapsed(SETTLE_SECS))
        .add()
        // The trigger goes down BEFORE the sheet opens, so cell one is a gun
        // already firing rather than a gun starting - which is the state the
        // last cell hands back to.
        .step("open fire on that bearing")
        .on_enter(|world: &mut World| set_triggers(world, true))
        .until(elapsed(SETTLE_SECS))
        .add()
        .step("step the aim across the bow and record the chase")
        .on_enter(|world: &mut World| {
            world.resource_mut::<SpentRounds>().samples.clear();
            swing_the_aim(world, 0);
            note_the_rounds(world, 0);
            sheet_start(world, DISCIPLINE_LESSON, LESSON_GRID);
        })
        .each(|world: &mut World, elapsed, _| {
            let cell = lesson_cell(elapsed);
            swing_the_aim(world, cell);
            note_the_rounds(world, cell);
        })
        // BOTH, and the swing is the half that matters: `sheet_written` holds
        // the instant it is asked on the smoke path, so a wait on it alone
        // would drive the unarmed run past the discipline it exists to prove.
        .until(and(sheet_written(DISCIPLINE_LESSON), elapsed(LESSON_SECS)))
        .deadline(240.0)
        .diagnose(report_the_swing)
        .add()
        .step("the battery held fire somewhere in the swing")
        .on_enter(the_guns_held_and_came_back)
        .until(frames(1))
        .add()
        // HOW FAR EACH WEAPON REACHES. The guns come off the trigger and go
        // quiet long enough for the magazines to take their batch back, so the
        // long hold below cannot run a mount dry mid-frame.
        .step("cease fire and let the magazines come back")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            set_triggers(world, false);
            lay_the_guns(world, BOAT_POSITION);
            pose_camera(world, REACH_EYE, REACH_AIM);
        })
        .until(elapsed(4.0))
        .add()
        .step("the hostile looses a salvo")
        .on_enter(loose_the_salvo)
        .until(elapsed(1.5))
        .add()
        .step("commit the salvo to the player")
        .on_enter(commit_the_salvo)
        .until(frames(1))
        .add()
        .step("hold the battery down and fill the lane")
        .on_enter(|world: &mut World| set_triggers(world, true))
        .until(and(the_lane_is_full(), elapsed(FILL_SECS)))
        .deadline(120.0)
        .diagnose(report_the_lane)
        .add()
        .step("wait for the salvo to cross into the gap")
        .until(the_salvo_is_in_the_gap())
        .deadline(120.0)
        .diagnose(report_the_lane)
        .add()
        .step("capture the reach still")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            the_stream_stops_where_it_should(world);
            shoot(world, REACH_SHOT);
        })
        .until(shot_written(REACH_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("stand down")
        .on_enter(|world: &mut World| set_triggers(world, false))
        .add()
}
