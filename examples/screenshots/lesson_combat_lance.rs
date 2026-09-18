//! lesson_combat_lance: the handbook's three demonstrations about a hull being
//! killed - `combat_bore_sight`, the instrument that prices a shot before it is
//! taken, `combat_railgun`, a spinal lance committing to one, and
//! `combat_collapse`, the hull that shot guts letting go of the rest of itself.
//!
//! ## One scene, two rigs, three sheets
//!
//! All three lessons are about the same act - a lance slug going through
//! structure - read at three points along it, so they are photographed on one
//! walk. The scene carries two ranges on parallel lanes, far enough apart that
//! neither shot can reach the other's subject (see [`WRECK_LANE_X`]):
//!
//! - the LANCE lane at the origin: a player gunboat with a lance on its spine,
//!   bore down -Z, and a base Patrol Gunship downrange bow-on so the shot rakes
//!   its long axis;
//! - the WRECK lane, off to starboard: a siege lance fixture on a bench, and
//!   a block hull built so that ONE slug takes it under the collapse threshold.
//!
//! ## `combat_bore_sight` is a LOOP, and the loop is the AIMING
//!
//! "Aiming down a ship's long axis reads differently from catching its
//! shoulder." That claim is a COMPARISON, and a still can only ever be one half
//! of one. So this sheet holds the lens and the target still and AIMS: the
//! gunboat yaws [`BORE_YAW_SWING`] off the bow-on hull's centreline and back,
//! and the sight is re-priced under it every frame. Down the spine the trace
//! runs the hull's whole length and rings nine or ten sections; out on the
//! shoulder it clips a corner and rings two. Same gun, same range, same hull.
//!
//! It is the GUNBOAT that moves rather than the target, and the first cut of
//! this frame had it the other way round. Turning the hull through its own beam
//! looks like the lesson's sentence and is not it: the corridor stayed seven to
//! ten sections deep at every aspect, because what the rake takes off a hull
//! this cheap barely depends on which way it is standing. What the sentence is
//! actually about is where on a hull the bore is laid, and that is an aiming
//! gesture.
//!
//! The swing is EASED rather than steady - one cosine over the whole sheet, so
//! it RESTS at each end of itself - which buys held cells of each half of the
//! comparison with the crossing between them, and wraps: the recorder may
//! double the cell it opens on, and that cell is one of a bore that is not
//! moving.
//!
//! It is photographed FIRST, before either gun is committed, and it has to be:
//! `combat_railgun` opens this same hull, and a sight drawn over a wreck prices
//! what is left rather than what the lesson is about.
//!
//! What it cannot be is the railgun frame with a swing on it. That one stands
//! abeam of the whole lane so both hulls fit, which leaves a kill ring 11 m
//! across about thirty pixels of the cell and puts the gunship's far side to
//! the lens. This one gives the gunboat up and stands on the target's port bow
//! instead ([`BORE_EYE`]), because a ring is drawn on the section it marks and
//! every ring but the entry one is inside an opaque hull.
//!
//! [`the_swing_changed_the_rings`] is this frame's delivery guard, and it is
//! the one it needs: a sheet of a thread crossing a hull that priced the same
//! sections throughout would tile perfectly and say nothing.
//!
//! ## `combat_railgun` is a LOOP, and the loop is the COMMIT
//!
//! "A tap starts a 1.5 second charge that only lowering your weapons aborts; it
//! never re-checks the nose." That claim is a length of TIME, and a still
//! cannot make it: a photograph of a lit bore says the gun is charging and says
//! nothing about how long the pilot is committed for.
//!
//! So the sheet runs at REAL TIME and is sized to the charge. Twenty cells at
//! ten a second is two seconds; the authored charge is 1.5 s, which is fifteen
//! of them, and [`the_charge_fits_the_sheet`] refuses to record if content ever
//! moves that number far enough to crowd the shot out of the end. What the
//! reader gets is three quarters of the loop watching a line thicken across a
//! hull it is already committed to, and then the shell. That is the lesson.
//!
//! The camera stands OFF the bore rather than behind it. The sight is a 0.6 m
//! holo thread: seen end-on it is a sub-pixel dot, and seen from the side it
//! draws the whole length of the shot with the kill ring at the face it enters.
//! It also holds still, because this is an ACTION loop - the charge and the
//! shot are the motion, and a camera that drifted would be competing with them.
//!
//! The flight itself is not in the sheet and cannot be: the slug leaves at
//! 15,000 m/s and the target is 150 m away, so its whole crossing is 10 ms -
//! under one fixed step. The shell is away on one cell and home on the same
//! one, which is what the weapon does.
//!
//! ## `combat_collapse` is a LOOP because the collapse is the EVENT
//!
//! "A hull carrying less than a twentieth of the structure it was built with
//! collapses." The interesting half of that is what the word "collapses"
//! covers: the hull is not despawned and it is not shot off - the engine
//! disables everything still standing and the ordinary destroy chain then peels
//! it apart, each piece bursting its own debris. A still of a wreck cannot tell
//! that apart from a wreck somebody shot, so the sheet has to hold the block
//! whole, the shot, and the letting go, in that order.
//!
//! The recording opens a breath BEFORE the shell - see
//! [`COLLAPSE_OPEN_CHARGE`] - with the world at [`COLLAPSE_SLOWDOWN`], and the
//! reason for the slow clock is worth stating because it is NOT the collapse.
//! The cascade is driven per FRAME and per GENERATION of the structure graph:
//! everything disabled that is a leaf dies, which makes the next ring of cells
//! leaves, and a spar three cells long is therefore gone in two frames however
//! slowly the world is running. What the slow clock buys is the other
//! seventeen cells - the corridor blowing out, and the block opening into a
//! cloud that is still hull-shaped inside the cell when the loop wraps, rather
//! than debris leaving the frame around a fireball for half of it.
//!
//! ## Why the wreck lane is a BLOCK and the gun on it is a siege lance
//!
//! The threshold is a RATIO, and that is the whole of the sizing problem. For a
//! hull to be under a twentieth of itself with sections still standing, what it
//! LOST has to be about twenty times what it kept - so the subject cannot be a
//! catalog ship taking a normal shot, and it cannot be a hull ground down by
//! gunfire either (gunfire kills sections one at a time, so the last one
//! standing is the only one that ever satisfies the ratio, and a one-cell peel
//! is not a collapse anybody can read).
//!
//! What does satisfy it is the weapon whose corridor is most of a hull. A
//! siege lance rakes [`SIEGE_RAKE_CELLS`] cells either side of its bore with
//! a pierce budget nothing in this scene can exhaust, so a block seven cells
//! across loses everything but its four corner spars in one flush. The spars
//! are the cheapest plate in the catalog and the core is the dearest, which is
//! what turns a 45-to-4 count into the 20-to-1 of structure the rule asks for -
//! [`the_block_goes_under_the_threshold`] does that arithmetic off the mounted
//! content and fails naming the numbers if the catalog or the fixture moves.
//!
//! Read the block the way `stress_hull_collapse` asks its own to be read: it is
//! the shape the RULE needs to be visible on, not a design figure. No shipped
//! hull is a solid prism.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole walk, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile both sheets (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_combat_lance --features debug
//! ```

#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

#[path = "../shared/dev_fixtures/mod.rs"]
mod dev_fixtures;
#[path = "shared/kit.rs"]
mod kit;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, LESSON_FPS, LESSON_GRID};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_combat_lance")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's railgun commit and hull collapse", long_about = None)]
struct Cli;

/// The sheet for "The bore sight".
#[cfg(feature = "debug")]
const BORE_LESSON: &str = "combat_bore_sight";
/// The sheet for "The hull is the aim".
#[cfg(feature = "debug")]
const RAILGUN_LESSON: &str = "combat_railgun";
/// The sheet for "When a ship comes apart".
#[cfg(feature = "debug")]
const COLLAPSE_LESSON: &str = "combat_collapse";

/// The scenario id the range loads under.
const RANGE_ID: &str = "lesson_lance_range";

/// The lance lane: the gunboat that fires, the gun on its spine, and the hull
/// it is aimed at.
const BOAT_ID: &str = "lance_boat";
const LANCE_ID: &str = "lance";
const TARGET_ID: &str = "target_gunship";

/// The catalog hull downrange: a base Patrol Gunship, whose spine is long
/// enough that a bow-on shot has something to bore THROUGH.
const TARGET_HULL: &str = "block_gunship";

/// How far downrange the target stands, in meters.
///
/// Well inside the lance's 18 km reach - the distance is chosen for the
/// FRAMING, so the gunboat takes one third of the cell and the hull it is
/// aimed at the other, with the sight thread strung between them.
const TARGET_Z: Meters = Meters(-150.0);

/// How far to starboard the wreck lane stands, in meters.
///
/// Set by the LENS, not by the corridors. Debris is the cheap half of the
/// problem - the wider of the two corridors is [`SIEGE_RAKE_CELLS`], 30 m, and
/// nothing crosses even a tenth of this. What costs is that the collapse sheet
/// is photographed looking back up its own lane, so the lance lane lies ahead
/// of it: at 900 m the gunship the first sheet guts sat in the top left corner
/// of every collapse cell as a handful of drifting plates. Three kilometers
/// puts it outside the frustum ([`BLOCK_EYE`] carries the arithmetic) and still
/// inside one directional rig.
const WRECK_LANE_X: Meters = Meters(3000.0);

/// The wreck lane: the bench the siege lance stands on, the gun, and the block.
const SIEGE_ID: &str = "siege_bench";
const SIEGE_LANCE_ID: &str = "siege_lance";
const BLOCK_ID: &str = "target_block";

/// Cells either side of the bore in each lateral axis, so the block is seven
/// across and what bounds the corridor is the RAKE rather than the edge of the
/// target.
const BLOCK_HALF: i32 = 3;
/// Layers of block along the bore.
///
/// Three, and the count is a compromise the sheet sets: every layer adds four
/// corner cells to the peel and 49 sections to the spawn, and the cascade takes
/// a section apart about every other frame - so three layers is already more
/// spars than twenty cells of sheet can get through.
const BLOCK_DEPTH: i32 = 3;

/// Where the block's one computer sits, in cells: a corner, outside the
/// corridor. A computer on the bore would be destroyed with the core and take
/// the hull's authority with it, and a ship the stack has stopped recognizing
/// is a different subject.
const BLOCK_BRIDGE_CELL: IVec3 = IVec3::new(BLOCK_HALF, BLOCK_HALF, 0);

/// The authored siege rake, in build cells. A DELIVERY GUARD, not an authored
/// number: [`the_block_goes_under_the_threshold`] reads the mounted section and
/// fails naming this when the catalog moves.
const SIEGE_RAKE_CELLS: f32 = 3.0;

/// The gap between the siege muzzle and the block's entry face, in cells.
///
/// Short on purpose, for the reason `stress_hull_collapse` gives: a slug at
/// 15 km/s crosses some 23 cells in one fixed step, so a block this shallow is
/// raked in ONE sweep and every layer of the corridor opens on the same frame.
const MUZZLE_GAP_CELLS: f32 = 4.0;

/// Slack on a lattice distance, in build cells. The arithmetic is exact; this
/// only absorbs f32 accumulation.
#[cfg(feature = "debug")]
const CELL_EPSILON: f32 = 1.0e-3;

/// The collapse threshold the handbook's words are written against: a hull
/// under a twentieth of its built structure comes apart. A DELIVERY GUARD -
/// the engine's own default is the number that runs.
#[cfg(feature = "debug")]
const COLLAPSE_FRACTION: f32 = 0.05;

/// The fewest cells of the railgun sheet that have to be left for the shot and
/// what it opens.
///
/// The charge is allowed to eat the rest. Four cells is four tenths of a second
/// of hull coming apart, which is the least that reads as an event rather than
/// as a frame of noise at the end of a loop.
#[cfg(feature = "debug")]
const MIN_AFTERMATH_CELLS: u32 = 4;

/// Charge fraction the walk hands the siege charge over to the slow clock at.
///
/// The handover exists because a charge can only be READ once a frame, and at
/// real time an armed frame is a fifteenth of it: the samples land on 0.87,
/// 0.93, and then 1.0 with the shell already away, so there is no frame late in
/// the charge to open a recording on. Under [`COLLAPSE_SLOWDOWN`] a frame is
/// less than a hundredth of the charge instead, which is fine enough to stop
/// anywhere. Two thirds, so the rest of the wait is some fifty unrecorded
/// frames rather than a hundred and forty.
#[cfg(feature = "debug")]
const COLLAPSE_HANDOVER_CHARGE: f32 = 0.60;

/// Charge fraction the collapse sheet opens at.
///
/// Late, and the lateness is the point: the siege charge is another 1.5 s of a
/// bench nobody is being shown, and the block is the subject. What is left of
/// the charge at this mark is 0.06 s of world, which at [`COLLAPSE_SLOWDOWN`]
/// is five frames; the screenshot readback costs the first one or two of them,
/// so the sheet opens on three cells of an untouched hull in front of the
/// shell - what the reader needs to see the collapse take something away. A cut
/// at 0.978 left a single cell of it, and a single cell reads as a loop that
/// starts mid-explosion.
#[cfg(feature = "debug")]
const COLLAPSE_OPEN_CHARGE: f32 = 0.96;

/// How slowly the world runs while the collapse sheet is recorded.
///
/// Measured, not chosen. The peel is unaffected (it is frame-driven - see the
/// module docs); what this buys is the corridor and the cloud. A hundred and
/// thirty five cells leave the block on one frame with an impulse that carries
/// the plates outward at something near 120 m/s, so the wreck grows about a
/// metre for every centisecond of world. The sheet is worth its twenty cells
/// only while the wreck is still hull-shaped inside the cell: a first cut at a
/// quarter ran 0.5 s of world and spent its last ten cells on debris drifting
/// out of frame around a fireball. An eleventh holds the whole loop to 0.22 s,
/// which ends with the block opened into a cloud that still fits the cell.
#[cfg(feature = "debug")]
const COLLAPSE_SLOWDOWN: f32 = 0.11;

/// How far the gunboat yaws off the target's spine at the far end of the
/// bore-sight swing, in degrees.
///
/// MEASURED, and the measurement is not the hull's half-width. The gunship is
/// 70 m across the bow, but what the rake finds out at the wings is one cell of
/// plate: the trace still prices three sections at 10.3 degrees off the
/// centreline and none at all at 11.8, so the shoulder this sheet is about is a
/// 27 m band and the edge of the hull is a metre and a half past it.
///
/// Ten degrees, which lands inside that band: the far end of the swing is a
/// shot catching a corner, not a shot missing. A sheet that swung wider spent
/// seven of its twenty cells on a clean pass, which is a different lesson.
#[cfg(feature = "debug")]
const BORE_YAW_SWING: f32 = 10.0;

/// Where the lens stands for the bore-sight sheet, and what it looks at.
///
/// On the target's PORT BOW quarter - 40 degrees off the lane, 16 above it and
/// 97 m out - and both halves of that bearing are forced.
///
/// The BOW, because a kill ring is drawn on the section it marks and every ring
/// but the first is inside a hull that is opaque: what a reader can see is the
/// ring at the face the bore ENTERS. A first cut of this frame stood astern and
/// showed a thread crossing a hull with nothing marked on it.
///
/// PORT, because of the light. The range's key light stands at (-60, 50, 60)
/// off its aim point (`ThreePointRig`), so a lens on the starboard bow takes
/// the shadow side and the same cut came back as a navy silhouette with a bright
/// deck. This is within five degrees of the key's own bearing.
///
/// The stand-off is set by the ring. At 97 m the frame is 143 m across, the
/// hull takes six tenths of it, and an 11 m ring is some seventy pixels of the
/// cell - which is the smallest a ring can be drawn and still be read as a ring
/// rather than as a smudge on a plate. The gunboat is 60 degrees off the axis
/// and therefore out of shot; its thread arrives over the frame edge instead,
/// which is the trade this frame makes for a mark a reader can see.
#[cfg(feature = "debug")]
const BORE_EYE: Meters3 = Meters3::new(-60.0, 27.0, -79.0);
#[cfg(feature = "debug")]
const BORE_AIM: Meters3 = Meters3::new(0.0, 0.0, -150.0);

/// Where the lens stands for the railgun sheet, and what it looks at.
///
/// Across the gap rather than down it, and close enough in that the cell's own
/// 2x downscale leaves the sight thread a couple of pixels wide: at full charge
/// the line is 0.72 m of holo geometry, which at this stand-off is about five
/// pixels of the master frame. The aim point sits between the two hulls and a
/// little short of the target, so the boat lands in the left third, the gunship
/// in the right, and the thread crosses the black between them.
#[cfg(feature = "debug")]
const SIGHT_EYE: Meters3 = Meters3::new(150.0, 44.0, 20.0);
#[cfg(feature = "debug")]
const SIGHT_AIM: Meters3 = Meters3::new(0.0, 0.0, -70.0);

/// Where the lens stands for the collapse sheet, and what it looks at.
///
/// Off the block's entry corner and high, so the corridor opens toward the lens
/// and the flank shows the spars leaving. The stand-off is set by the END of
/// the loop rather than by the start: 175 m puts 145 m of world across the
/// cell's height, the block is 70 m of that, and the wreck it becomes by the
/// twentieth cell is about 135 m - so the last cell is still a hull coming
/// apart and not a wall of plates. The bench keeps its own corner of the frame
/// at this distance, low and left of the block, which is where the shell has to
/// come from for the loop to read.
///
/// The lance lane lies up this bearing, a little left of the block and just
/// over the top edge: the lens sits 0.573 of a half-width off it, and the frame
/// holds 0.736, so the lane has to stand past about 2.7 km for its wreck to
/// leave the cell. See [`WRECK_LANE_X`].
#[cfg(feature = "debug")]
const BLOCK_EYE: Meters3 = Meters3::new(WRECK_LANE_X.get() + 125.0, 82.0, 9.0);
#[cfg(feature = "debug")]
const BLOCK_AIM: Meters3 = Meters3::new(WRECK_LANE_X.get(), 0.0, -85.0);

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // no frame-time capture, for the reason `screenshot_railgun` gives -
        // a posed one-shot walk never fills the baseline window.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::new(
            lesson_profile(),
        ));
        app.init_resource::<LanceProbe>();
        app.init_resource::<BoreAim>();
        app.add_observer(count_shots);
        app.add_observer(note_collapse);
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        // Hold the gunboat where the framing was measured from. NOT
        // `freeze_bodies`, which would make every dynamic body static and pin
        // the slug in the bore: the recoil is one impulse on a free hull, so
        // zeroing the gunboat's own velocities is the whole fix.
        app.add_systems(Update, hold_the_boat);
        // The sight IS the subject of the first sheet, so it survives the
        // cinematic that takes the rest of the chrome away.
        app.add_systems(Update, unmanage_the_sight);
        app.add_plugins(lance_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_range);
}

fn load_range(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    ships: Res<GameShipDesigns>,
) {
    commands.trigger(LoadScenario(lance_range(&game_assets, &sections, &ships)));
}

/// One inline section of a hand-built hull.
fn at(sections: &GameSections, id: String, kind: &str, position: Vec3) -> SpaceshipSectionConfig {
    let section = sections
        .get_section(kind)
        .unwrap_or_else(|| panic!("section '{kind}' not found"))
        .clone();
    SpaceshipSectionConfig {
        id,
        position,
        rotation: Quat::IDENTITY,
        source: SectionSource::Inline(section),
    }
}

/// The gunboat's hull, in BUILD-GRID cells.
///
/// The lance is three cells long and centred on its own origin, so at `-1` it
/// fills cells -2, -1 and 0 and everything else starts at +1. Cell -3 is left
/// empty because it has to be: a lance cannot traverse off its bore, so a
/// section standing there is a shot taken through the ship's own hull.
///
/// Bare structure rather than a clad hull, and the reason is the bore: cladding
/// fills the empty cells AROUND a hull, and the one cell this design must keep
/// empty is the one the shell leaves through. The gunship downrange wears the
/// skin it ships with, which is where the frame's "this is the game" reading
/// comes from.
fn boat_hull(sections: &GameSections) -> Vec<SpaceshipSectionConfig> {
    let hull = |id: &str, position: Vec3| {
        at(
            sections,
            id.to_string(),
            REINFORCED_HULL_SECTION_ID,
            position,
        )
    };

    vec![
        at(
            sections,
            LANCE_ID.to_string(),
            RAILGUN_LANCE_SECTION_ID,
            Vec3::new(0.0, 0.0, -1.0),
        ),
        // Beside the lance's aft cell, so the gun reads as MOUNTED on a spine
        // rather than as a spike floating in front of one.
        hull("mount_port", Vec3::new(-1.0, 0.0, 0.0)),
        hull("mount_starboard", Vec3::new(1.0, 0.0, 0.0)),
        hull("spine_fore", Vec3::new(0.0, 0.0, 1.0)),
        hull("waist_port", Vec3::new(-1.0, 0.0, 1.0)),
        hull("waist_starboard", Vec3::new(1.0, 0.0, 1.0)),
        at(
            sections,
            "bridge".to_string(),
            BASIC_CONTROLLER_SECTION_ID,
            Vec3::new(0.0, 0.0, 2.0),
        ),
        hull("flank_port", Vec3::new(-1.0, 0.0, 2.0)),
        hull("flank_starboard", Vec3::new(1.0, 0.0, 2.0)),
        hull("dorsal", Vec3::new(0.0, 1.0, 2.0)),
        hull("spine_aft", Vec3::new(0.0, 0.0, 3.0)),
        at(
            sections,
            "drive_port".to_string(),
            BASIC_THRUSTER_SECTION_ID,
            Vec3::new(-1.0, 0.0, 3.0),
        ),
        at(
            sections,
            "drive_starboard".to_string(),
            BASIC_THRUSTER_SECTION_ID,
            Vec3::new(1.0, 0.0, 3.0),
        ),
    ]
}

/// How far a lattice cell's NEAREST point lies from the bore, in build cells.
///
/// The same distance the swept corridor tests against (`corridor_contact` in
/// `nova_gameplay::rounds`), so a cell inside the rake radius is a cell the
/// rake owes a bite. Pure, so the block can be classified without a running
/// app.
fn cell_offset(cell: IVec3) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a lattice index, bounded by BLOCK_HALF"
    )]
    let reach = |n: i32| (n.abs() as f32 - 0.5).max(0.0);
    reach(cell.x).hypot(reach(cell.y))
}

/// Every cell of the block, in a stable order.
fn block_cells() -> Vec<IVec3> {
    let mut cells = Vec::new();
    for layer in 0..BLOCK_DEPTH {
        for y in -BLOCK_HALF..=BLOCK_HALF {
            for x in -BLOCK_HALF..=BLOCK_HALF {
                cells.push(IVec3::new(x, y, -layer));
            }
        }
    }
    cells
}

/// What plate one block cell is built from.
///
/// The CORE is the dearest plate in the catalog and the SPARS are the cheapest,
/// and that choice is the whole reason this hull can cross the threshold: the
/// rake takes 45 of the 49 cells in a layer, and 45 reinforced blocks against 4
/// light ones is a twenty-to-one of structure where the count alone is only
/// eleven-to-one.
fn block_plate(cell: IVec3) -> &'static str {
    if cell == BLOCK_BRIDGE_CELL {
        BASIC_CONTROLLER_SECTION_ID
    } else if cell_offset(cell) <= SIEGE_RAKE_CELLS {
        REINFORCED_HULL_SECTION_ID
    } else {
        LIGHT_HULL_SECTION_ID
    }
}

/// The range: both lanes and one light rig over them.
fn lance_range(
    game_assets: &GameAssets,
    sections: &GameSections,
    ships: &GameShipDesigns,
) -> ScenarioConfig {
    let boat = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: BOAT_ID.to_string(),
            name: "Lance Gunboat".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Player),
            // An EMPTY input mapping: the walk drives the gun with a scripted
            // order and the stance with the bindings registry, so the ship
            // needs no button of its own - and nothing it carries can fly it
            // out of a framing that was posed frames earlier.
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: std::collections::BTreeMap::new(),
            }),
            design: ShipDesignSource::Inline(ShipDesign {
                sections: boat_hull(sections),
                ..default()
            }),
            ..default()
        }),
    };

    let target = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: TARGET_ID.to_string(),
            name: "Target Gunship".to_string(),
            position: Meters3::new(0.0, 0.0, TARGET_Z.get()),
            // Turned to face the bore, so the slug runs the length of the hull.
            // A shot across a beam crosses two or three cells and leaves a
            // dent; this one is the corridor the sight's rings promise.
            rotation: Quat::from_rotation_y(std::f32::consts::PI),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Enemy),
            controller: SpaceshipController::None,
            design: ShipDesignSource::Inline(kit::catalog_ship(ships, TARGET_HULL)),
            ..default()
        }),
    };

    // The siege lance is three cells long centred on its own origin, so its
    // muzzle sits a cell and a half ahead of the cell it is placed in, and the
    // block's entry face stands MUZZLE_GAP_CELLS past that.
    let muzzle_z = -1.0 - 1.5;
    let entry_face_z = muzzle_z - MUZZLE_GAP_CELLS;
    let block_origin_z = Meters::from_engine(entry_face_z - 0.5).get();

    let bench = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: SIEGE_ID.to_string(),
            name: "Siege Lance Bench".to_string(),
            position: Meters3::new(WRECK_LANE_X.get(), 0.0, 0.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            // UNMANAGED, and that is the point: a ship with no weapons safety
            // fires freely, so this bench needs neither a combat stance nor a
            // second player hull to hold one. The scripted order is the whole
            // of its trigger.
            controller: SpaceshipController::None,
            design: ShipDesignSource::Inline(ShipDesign {
                sections: vec![
                    SpaceshipSectionConfig {
                        id: SIEGE_LANCE_ID.to_string(),
                        position: Vec3::new(0.0, 0.0, -1.0),
                        rotation: Quat::IDENTITY,
                        source: SectionSource::Inline(dev_fixtures::sections::siege_railgun_lance()),
                    },
                    at(
                        sections,
                        "mount".to_string(),
                        REINFORCED_HULL_SECTION_ID,
                        Vec3::new(0.0, 0.0, 1.0),
                    ),
                ],
                ..default()
            }),
            ..default()
        }),
    };

    let block = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: BLOCK_ID.to_string(),
            name: "Target Block".to_string(),
            position: Meters3::new(WRECK_LANE_X.get(), 0.0, block_origin_z),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::None,
            design: ShipDesignSource::Inline(ShipDesign {
                sections: block_cells()
                    .into_iter()
                    .map(|cell| {
                        at(
                            sections,
                            format!("cell_{}_{}_{}", cell.x, cell.y, cell.z),
                            block_plate(cell),
                            cell.as_vec3(),
                        )
                    })
                    .collect(),
                // NO derived skin. The subject is the STRUCTURE coming apart,
                // and a plate per exposed face would put a second population
                // between the lens and every piece that leaves.
                ..default()
            }),
            ..default()
        }),
    };

    ScenarioConfig {
        description: "A gunboat with a spinal lance, a hull downrange, and a siege lance on a \
                      block to collapse."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            // The range lights itself: the engine spawns no light, so a
            // scenario that authors none renders black. THREE DIRECTIONAL
            // lights aimed between the two lanes - a directional light's
            // bearing is the whole of it, so one rig lights a gunboat, a hull
            // 150 m downrange and a block 3 km to starboard alike.
            actions: [
                vec![
                    EventActionConfig::SpawnScenarioObject(boat),
                    EventActionConfig::SpawnScenarioObject(target),
                    EventActionConfig::SpawnScenarioObject(bench),
                    EventActionConfig::SpawnScenarioObject(block),
                ],
                ThreePointRig::around(
                    "range",
                    Meters3::new(WRECK_LANE_X.get() * 0.5, 0.0, -80.0),
                    20.0,
                )
                .actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            RANGE_ID.to_string(),
            "Lance Lesson Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// What the walk has watched the guns do.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct LanceProbe {
    /// Slugs that have left either lance.
    shots: u32,
    /// Sections the target gunship had before the shell reached it.
    target_sections: usize,
    /// Sections the block had before the siege shell reached it.
    block_sections: usize,
    /// Kill rings the bore sight drew, one sample per frame of the turn.
    ///
    /// The whole series rather than a running minimum and maximum, because it
    /// is what [`report_the_sight`] prints when the beat stalls: a count that
    /// never moved and a count that collapsed to nothing fail the same
    /// assertion and want different fixes.
    sight_rings: Vec<usize>,
    /// Set the frame the block crossed the collapse threshold.
    ///
    /// Latched rather than read off the world, because the root does not
    /// survive its own cascade: by the time the sheet closes there is no ship
    /// left to carry the marker, so a predicate that looked for one would go
    /// false again halfway through the recording.
    collapsed: bool,
}

/// Count the slugs that leave, so the walk waits on the SHOT rather than on a
/// guessed number of frames.
#[cfg(feature = "debug")]
fn count_shots(_: On<RailgunFired>, mut probe: ResMut<LanceProbe>) {
    probe.shots += 1;
}

/// Latch the frame the block crossed the threshold.
#[cfg(feature = "debug")]
fn note_collapse(
    add: On<Add, StructuralCollapseMarker>,
    q_id: Query<&EntityId>,
    mut probe: ResMut<LanceProbe>,
) {
    if q_id.get(add.entity).is_ok_and(|id| id.0 == BLOCK_ID) {
        probe.collapsed = true;
    }
}

/// Where the gunboat is pointing, in radians of yaw off the lane.
///
/// Zero for every frame but the bore-sight sheet's, which is the one beat that
/// AIMS: see [`sweep_the_bore`]. Held as a resource rather than driven into the
/// hull directly because [`hold_the_boat`] would put it straight back.
#[cfg(feature = "debug")]
#[derive(Resource, Default, Clone, Copy)]
struct BoreAim(f32);

/// Hold the gunboat on its mark, pointing where [`BoreAim`] says.
///
/// The recoil is a real impulse on a free hull and the framing is measured from
/// a boat at the origin, square with the world. Zeroing its velocities each
/// frame is the posed-capture equivalent of `hollow::pin_player`.
///
/// Avian's [`Rotation`](avian3d::prelude::Rotation) is written beside the
/// transform, because a posed heading is not a held one: the transform alone
/// says where the hull is drawn and the rotation is what the bore is traced
/// from, and the bore-sight sheet is a picture of the trace.
#[cfg(feature = "debug")]
fn hold_the_boat(
    aim: Res<BoreAim>,
    mut boat: Query<
        (
            &mut Transform,
            &mut avian3d::prelude::Rotation,
            &mut avian3d::prelude::LinearVelocity,
            &mut avian3d::prelude::AngularVelocity,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    let heading = Quat::from_rotation_y(aim.0);
    for (mut transform, mut rotation, mut linear, mut angular) in &mut boat {
        transform.translation = Vec3::ZERO;
        transform.rotation = heading;
        rotation.0 = heading;
        linear.0 = Vec3::ZERO;
        angular.0 = Vec3::ZERO;
    }
}

/// Take the bore sight out of HUD management as it is drawn.
///
/// `nova_hud` tags every sight segment and kill ring `HudTier::Instrument` so a
/// cinematic takes the line away with the rest of the flight chrome - the right
/// call in the game, and the wrong one here, where the sight is what the first
/// sheet is OF. Stripping the tag is enough: `apply_hud_visibility` filters on
/// it, so an untagged segment is nobody's to hide. The visibility it already
/// wrote is cleared in the same pass, because nothing restores a widget the HUD
/// has stopped tracking.
#[cfg(feature = "debug")]
fn unmanage_the_sight(
    mut commands: Commands,
    mut sight: Query<
        (Entity, &mut Visibility),
        (
            With<HudTier>,
            Or<(With<BoreSightSegment>, With<BoreSightMark>)>,
        ),
    >,
) {
    for (entity, mut visibility) in &mut sight {
        *visibility = Visibility::Inherited;
        commands.entity(entity).remove::<HudTier>();
    }
}

/// The lance section carrying `id`, found the way every other posed scene finds
/// one: by the marker the section carries.
#[cfg(feature = "debug")]
fn lance_by_id(world: &mut World, id: &str) -> Option<Entity> {
    world
        .try_query_filtered::<(Entity, &EntityId), With<RailgunSectionMarker>>()?
        .iter(world)
        .find(|(_, held)| held.0 == id)
        .map(|(entity, _)| entity)
}

/// The ship root carrying `id`.
#[cfg(feature = "debug")]
fn root_by_id(world: &mut World, id: &str) -> Option<Entity> {
    world
        .try_query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()?
        .iter(world)
        .find(|(_, held)| held.0 == id)
        .map(|(entity, _)| entity)
}

/// How many sections the ship `id` still has on it.
#[cfg(feature = "debug")]
fn sections_of(world: &mut World, id: &str) -> usize {
    let Some(root) = root_by_id(world, id) else {
        return 0;
    };
    let Some(children) = world.get::<Children>(root).map(|kids| kids.to_vec()) else {
        return 0;
    };
    children
        .into_iter()
        .filter(|child| world.get::<SectionMarker>(*child).is_some())
        .count()
}

/// Swing the bore off the target's spine and back, `frame` frames into the
/// sheet.
///
/// Frame-indexed rather than clocked, for the reason `frames` gives: an armed
/// run pins the clock to [`LESSON_FPS`] and the smoke run does not, so a swing
/// driven by seconds would complete on the sheet and barely start on the walk
/// that has to prove the same thing without one.
///
/// Eased by a cosine over one whole period, so the sheet RESTS at each end of
/// itself - held cells of the shot down the spine, held cells of the shot on
/// the shoulder, and the crossing in between. That is the lesson's own
/// comparison, and it is also what makes the loop wrap: the recorder may double
/// the cell it opens on, and that cell is one of a bore that is not moving.
///
/// Yawed toward the lens - a positive rotation about Y walks the bore toward
/// -X, and [`BORE_EYE`] stands on the port bow - so the shoulder the swing ends
/// on is the one facing the camera, where its kill ring is drawn on a face in
/// shot rather than on the far side of the hull.
#[cfg(feature = "debug")]
fn sweep_the_bore(world: &mut World, frame: u32) {
    let phase = std::f32::consts::TAU * frame as f32 / LESSON_GRID.frames() as f32;
    let yaw = BORE_YAW_SWING.to_radians() * 0.5 * (1.0 - phase.cos());
    world.insert_resource(BoreAim(yaw));
}

/// How many sections the bore sight is promising to destroy right now.
#[cfg(feature = "debug")]
fn kill_rings(world: &mut World) -> usize {
    world
        .try_query_filtered::<Entity, With<BoreSightMark>>()
        .map_or(0, |mut rings| rings.iter(world).count())
}

/// Sample the sight for this frame of the swing.
#[cfg(feature = "debug")]
fn note_the_rings(world: &mut World) {
    let rings = kill_rings(world);
    world.resource_mut::<LanceProbe>().sight_rings.push(rings);
}

/// Whether both lances have arrived.
#[cfg(feature = "debug")]
fn both_lances_present() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(mut query) = world.try_query_filtered::<&EntityId, With<RailgunSectionMarker>>()
        else {
            return false;
        };
        let mut lance = false;
        let mut siege = false;
        for id in query.iter(world) {
            lance |= id.0 == LANCE_ID;
            siege |= id.0 == SIEGE_LANCE_ID;
        }
        lance && siege
    })
}

/// Whether the whole block has arrived. Read off the section count rather than
/// off the root, because the block spawns its cells over several frames and a
/// framing posed against half a hull is a framing of half a hull.
#[cfg(feature = "debug")]
fn the_block_is_standing() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    let wanted = block_cells().len();
    std::sync::Arc::new(move |world: &World| {
        let Some(mut roots) =
            world.try_query_filtered::<(&EntityId, &Children), With<SpaceshipRootMarker>>()
        else {
            return false;
        };
        roots.iter(world).any(|(id, children)| {
            id.0 == BLOCK_ID
                && children
                    .iter()
                    .filter(|child| world.get::<SectionMarker>(*child).is_some())
                    .count()
                    == wanted
        })
    })
}

/// Whether `count` slugs have left.
#[cfg(feature = "debug")]
fn shots_fired(count: u32) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .get_resource::<LanceProbe>()
            .is_some_and(|probe| probe.shots >= count)
    })
}

/// Whether the block has crossed the threshold and started tearing itself
/// apart.
#[cfg(feature = "debug")]
fn the_block_collapsed() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<LanceProbe>()
            .is_some_and(|probe| probe.collapsed)
    })
}

/// Whether the lance `id` has run at least `fraction` of its authored charge.
#[cfg(feature = "debug")]
fn charge_at_least(
    id: &'static str,
    fraction: f32,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        let Some(mut query) = world.try_query_filtered::<(
            &EntityId,
            &RailgunCharge,
            &RailgunSectionConfigHelper,
        ), With<RailgunSectionMarker>>() else {
            return false;
        };
        query.iter(world).any(|(held, charge, config)| {
            held.0 == id && charge.progress(config.charge_seconds) >= fraction
        })
    })
}

/// Run gameplay time at `scale`.
#[cfg(feature = "debug")]
fn set_time_scale(world: &mut World, scale: f32) {
    world
        .resource_mut::<Time<Virtual>>()
        .set_relative_speed(scale);
}

/// Raise the weapons. The bore sight is gated on `WeaponsHot`, and the gun
/// refuses a commit while the ship is cold, so this is the beat everything
/// after it depends on.
#[cfg(feature = "debug")]
fn raise_weapons(world: &mut World) {
    press_action("combat_stance")(world);
}

/// Lower them again, so the walk does not exit with a key held.
#[cfg(feature = "debug")]
fn lower_weapons(world: &mut World) {
    release_action("combat_stance")(world);
}

/// Commit the shot on one lance.
///
/// A scripted order rather than a synthesized trigger: the walk has no input
/// mapping to press, and the order holds the gun's own trigger down until the
/// shell actually leaves, so every gate the weapon has stays in force.
#[cfg(feature = "debug")]
fn commit(world: &mut World, lance: Entity) {
    world.entity_mut(lance).insert(ScriptedRailgunOrder);
}

/// What the sight priced at each frame of the swing, for a stalled beat.
#[cfg(feature = "debug")]
fn report_the_sight(world: &World) -> String {
    let rings = &world.resource::<LanceProbe>().sight_rings;
    format!("kill rings per frame of the swing: {rings:?}")
}

/// The fewest sections the two ends of the swing have to differ by.
///
/// The lesson's claim is a COMPARISON, and a comparison needs a gap a reader
/// can see. A shot down this hull's spine prices some nine or ten sections and
/// one on its shoulder a couple, so four is a wide margin on the delivered
/// scene and still fails the moment the two ends of the swing stop saying
/// different things.
#[cfg(feature = "debug")]
const MIN_RING_SPREAD: usize = 4;

/// The DELIVERY GUARD for the bore-sight sheet: the sight has to have drawn,
/// and the swing has to have changed what it drew.
///
/// A sheet of a bore crossing a hull under a line that priced the same sections
/// throughout tiles just as cleanly as the demonstration and says nothing, and
/// it is the failure this frame cannot survive. Read off the rings the walk
/// counted rather than off the hull, because what is being checked is the
/// INSTRUMENT.
#[cfg(feature = "debug")]
fn the_swing_changed_the_rings(world: &mut World) {
    let rings = world.resource::<LanceProbe>().sight_rings.clone();
    let most = rings.iter().copied().max().unwrap_or(0);
    let fewest = rings.iter().copied().min().unwrap_or(0);
    info!(
        "bore sight: {fewest} to {most} sections priced across the swing, frame by frame {rings:?}"
    );
    assert!(
        fewest > 0,
        "the bore sight came off the hull somewhere in the swing and marked nothing, so the sheet \
         spends cells on a clean miss instead of on the shoulder the lesson names"
    );
    assert!(
        most >= fewest + MIN_RING_SPREAD,
        "the sight priced {fewest} to {most} sections across the whole swing, which is under the \
         {MIN_RING_SPREAD} a reader can see, so the sheet cannot show that a shot down the long \
         axis reads differently from one catching a shoulder"
    );
}

/// The DELIVERY GUARD for the railgun sheet: the authored charge has to leave
/// room in twenty cells for the shot and what it opens.
///
/// Read off the mounted section, because the charge is content this producer
/// does not own. A lance retimed to two seconds would otherwise ship a sheet
/// that is twenty cells of a line getting thicker and no shell in it, and say
/// nothing.
#[cfg(feature = "debug")]
fn the_charge_fits_the_sheet(world: &mut World, lance: Entity) {
    let seconds = world
        .get::<RailgunSectionConfigHelper>(lance)
        .expect("the lance carries its authored config")
        .charge_seconds;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a ceil of an authored positive number of seconds"
    )]
    let charge_cells = (seconds * LESSON_FPS as f32).ceil() as u32;
    assert!(
        charge_cells + MIN_AFTERMATH_CELLS <= LESSON_GRID.frames(),
        "a {seconds} s charge is {charge_cells} of the sheet's {} cells at {LESSON_FPS} fps, \
         which leaves under {MIN_AFTERMATH_CELLS} for the shell and the hull it opens",
        LESSON_GRID.frames()
    );
}

/// The DELIVERY GUARD for the collapse sheet: one siege slug has to take this
/// block under a twentieth of the structure it was built with, with its spars
/// still standing.
///
/// Every number is read off the mounted section and the catalog rather than
/// written down here, because all of it is content this producer does not own.
/// The three ways it can break each get their own message: a rake that no
/// longer bounds the corridor inside the block, a slug that damages a core cell
/// instead of clearing it, and a ratio of lost to kept structure that has
/// stopped reaching the rule.
#[cfg(feature = "debug")]
fn the_block_goes_under_the_threshold(world: &mut World, lance: Entity) {
    let config = world
        .get::<RailgunSectionConfigHelper>(lance)
        .expect("the siege lance carries its authored config")
        .clone();
    let rake = config
        .rake()
        .map(Meters::to_engine)
        .expect("the siege lance authors a rake radius");
    assert!(
        (rake - SIEGE_RAKE_CELLS).abs() <= CELL_EPSILON,
        "the siege lance now rakes {} m, not the {} m this block is cut around",
        Meters::from_engine(rake).get(),
        Meters::from_engine(SIEGE_RAKE_CELLS).get()
    );

    // The block's own structure, read off the sections that actually spawned.
    let Some(root) = root_by_id(world, BLOCK_ID) else {
        panic!("the block never spawned");
    };
    let children = world
        .get::<Children>(root)
        .map(|kids| kids.to_vec())
        .unwrap_or_default();
    let mut core_max = 0.0f32;
    let mut spar_max = 0.0f32;
    let mut dearest_core = 0.0f32;
    let mut core_cells = 0usize;
    for (cell, child) in block_cells().into_iter().zip(children) {
        let Some(health) = world.get::<Health>(child) else {
            continue;
        };
        if cell_offset(cell) <= rake {
            core_max += health.max;
            dearest_core = dearest_core.max(health.max);
            core_cells += 1;
        } else {
            spar_max += health.max;
        }
    }

    assert!(
        config.slug_damage >= dearest_core,
        "a slug dealing {} no longer clears this block's dearest core cell at {dearest_core} hp, \
         so the corridor is damaged rather than destroyed",
        config.slug_damage
    );
    // Priced at the pierce ceiling, which a 15 km/s slug is far past: a
    // crossing costs the layer's MAX health over the closing-speed multiplier.
    let cheapest = dearest_core / 3.0;
    assert!(
        config.slug_power >= core_cells as f32 * cheapest,
        "{} of pierce budget cannot pay for {core_cells} core cells at {cheapest} each, so what \
         stops the rake is the budget and not the block",
        config.slug_power
    );
    assert!(
        spar_max <= (core_max + spar_max) * COLLAPSE_FRACTION,
        "the block keeps {spar_max} of structure outside a {rake} cell rake and loses {core_max} \
         inside it, so what is left is over the {COLLAPSE_FRACTION} of built structure a hull \
         collapses under"
    );
}

/// Fire both lances and record what each one leaves behind.
#[cfg(feature = "debug")]
fn lance_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the range")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            and(
                scenario_camera_present(),
                and(both_lances_present(), the_block_is_standing()),
            ),
        ))
        .deadline(180.0)
        .add()
        // The stance first, and its own settle: `WeaponsHot` is derived from
        // the held stance every frame and the sight is spawned off that, so a
        // commit taken in the same breath as the press charges a gun whose
        // sight has not been drawn yet.
        .step("raise the weapons and stand off the bore")
        .on_enter(|world: &mut World| {
            hide_status_bar(world);
            raise_weapons(world);
            pose_camera(world, BORE_EYE, BORE_AIM);
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        // The bore sight FIRST, on a whole hull: the railgun frame opens this
        // same gunship, and a sight drawn over a wreck prices what is left of
        // it. The swing and the wait are both counted in FRAMES so the sheet
        // lands on the armed run and on the smoke walk alike.
        .step("swing the bore across the target and record it")
        .on_enter(|world: &mut World| {
            world.resource_mut::<LanceProbe>().sight_rings.clear();
            sweep_the_bore(world, 0);
            sheet_start(world, BORE_LESSON, LESSON_GRID);
        })
        .each(|world: &mut World, _, frame| {
            sweep_the_bore(world, frame);
            note_the_rings(world);
        })
        // BOTH, and the swing is the half that matters: `sheet_written` holds
        // the instant it is asked on the smoke path, so a wait on it alone
        // would drive the unarmed run past the aiming it exists to prove.
        .until(and(
            sheet_written(BORE_LESSON),
            frames(LESSON_GRID.frames()),
        ))
        .deadline(240.0)
        .diagnose(report_the_sight)
        .add()
        .step("the sight drew, and the swing changed what it drew")
        .on_enter(the_swing_changed_the_rings)
        .until(frames(1))
        .add()
        // Square on the lane again, and back to its own framing: the railgun
        // frame is measured from a boat bore-on to the hull it opens.
        .step("square the gunboat back on the lane")
        .on_enter(|world: &mut World| {
            sweep_the_bore(world, 0);
            pose_camera(world, SIGHT_EYE, SIGHT_AIM);
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        // The sheet opens in the same breath as the commit, because the commit
        // is what the sheet is of: an armed run is frame-clocked at the
        // handbook's ten a second, so the charge's authored 1.5 s IS fifteen
        // cells of it and the shell lands with four to spare.
        .step("commit the lance and open the railgun sheet")
        .on_enter(|world: &mut World| {
            let lance = lance_by_id(world, LANCE_ID).expect("the gunboat's lance is mounted");
            the_charge_fits_the_sheet(world, lance);
            world.resource_mut::<LanceProbe>().target_sections = 0;
            let standing = sections_of(world, TARGET_ID);
            world.resource_mut::<LanceProbe>().target_sections = standing;
            sheet_start(world, RAILGUN_LESSON, LESSON_GRID);
            commit(world, lance);
        })
        // BOTH, and the shot is the half that matters: `sheet_written` holds
        // the instant it is asked on the smoke path, so a wait on it alone
        // would drive the unarmed run straight past the charge it exists to
        // prove.
        .until(and(sheet_written(RAILGUN_LESSON), shots_fired(1)))
        .deadline(240.0)
        .add()
        // A sheet of a lit bore and an untouched hull tiles just as cleanly as
        // a sheet of the demonstration, and is the one failure this frame
        // cannot survive.
        .step("the shell opened the target")
        .on_enter(|world: &mut World| {
            let before = world.resource::<LanceProbe>().target_sections;
            let after = sections_of(world, TARGET_ID);
            assert!(
                after < before,
                "the target gunship still carries all {before} of its sections, so the sheet \
                 ends on a hull the shell never reached"
            );
        })
        .until(frames(1))
        .add()
        // The lens leaves the player's ship here, so the flight chrome goes
        // with it: nothing on the block's frame is an instrument reading.
        .step("stand off the block")
        .on_enter(|world: &mut World| {
            lower_weapons(world);
            pose_camera(world, BLOCK_EYE, BLOCK_AIM);
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        // NOT recorded: the siege charge is 1.5 s of a bench nobody is being
        // shown. The recording opens on its last breath instead - see
        // COLLAPSE_OPEN_CHARGE.
        .step("charge the siege lance")
        .on_enter(|world: &mut World| {
            let lance =
                lance_by_id(world, SIEGE_LANCE_ID).expect("the bench's siege lance is mounted");
            the_block_goes_under_the_threshold(world, lance);
            let standing = sections_of(world, BLOCK_ID);
            world.resource_mut::<LanceProbe>().block_sections = standing;
            commit(world, lance);
        })
        .until(charge_at_least(SIEGE_LANCE_ID, COLLAPSE_HANDOVER_CHARGE))
        .deadline(240.0)
        .add()
        // The clock changes HERE, a beat before the recorder opens, and
        // COLLAPSE_HANDOVER_CHARGE is why: the slow clock is what makes a
        // charge fine enough to stop on.
        .step("slow the world for the collapse")
        .on_enter(|world: &mut World| set_time_scale(world, COLLAPSE_SLOWDOWN))
        .until(charge_at_least(SIEGE_LANCE_ID, COLLAPSE_OPEN_CHARGE))
        .deadline(240.0)
        .add()
        .step("record the block letting go")
        .on_enter(|world: &mut World| sheet_start(world, COLLAPSE_LESSON, LESSON_GRID))
        // BOTH, and the collapse is the half that matters: `sheet_written`
        // holds the instant it is asked on the smoke path, so a wait on it
        // alone would drive the unarmed run past the event it exists to prove.
        .until(and(sheet_written(COLLAPSE_LESSON), the_block_collapsed()))
        .deadline(240.0)
        .add()
        .step("the block came apart on its own")
        .on_enter(|world: &mut World| {
            let before = world.resource::<LanceProbe>().block_sections;
            let after = sections_of(world, BLOCK_ID);
            assert!(
                world.resource::<LanceProbe>().collapsed,
                "the block never crossed the collapse threshold, so the sheet is a picture of a \
                 hull somebody shot a hole in"
            );
            assert!(
                after * 4 < before,
                "the block stood at {before} sections and still has {after} after its own \
                 cascade, so the sheet ends on a wreck holding together"
            );
        })
        .until(frames(1))
        .add()
        .step("stand down")
        .on_enter(|world: &mut World| {
            lower_weapons(world);
            set_time_scale(world, 1.0);
        })
        .add()
}
