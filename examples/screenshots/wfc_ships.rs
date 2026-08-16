//! wfc_ships: a row of random spaceships grown by wave function collapse,
//! where the adjacency rules ARE the catalog's link points.
//!
//! The generator knows nothing about ships. It knows that a cell of the build
//! grid holds one section at one rotation, that a socket may never press into a
//! face that has none - two sockets meeting is exactly what
//! `derive_link_point_graph` calls a mate - and that nothing may stand in the
//! space a part fires, launches or exhausts into. Everything that reads as
//! design comes back out of those two rules.
//!
//! What is collapsed is STRUCTURE, and only structure: hull, a bridge, drives,
//! bays and mounts - the sections that make it a ship - on a half grid that is
//! then mirrored.
//!
//! - A PDC mount carries ONE socket, on its base plate, so the collapse can
//!   only ever bolt it down by the foot. Turrets end up on the skin, pointing
//!   out, because there is nowhere else they fit.
//! - A drive carries one too, on the flat end it mounts by. Its other five
//!   faces carry nothing to mate, which is not the same as being closed to the
//!   world: two drives may sit flank to flank, and a bank of nozzles is a ship
//!   part rather than a fault.
//! - A torpedo bay's bow face is a hole, not a socket, so nothing mates across
//!   the muzzle.
//!
//! Mating alone is not enough for any of those three, because a muzzle, a
//! nozzle and a bay's mouth are all faces with nothing to mate: the rule that
//! lets two drives sit flank to flank also let one exhaust into the other's
//! side. CLEARANCE is the second rule, read off the part's KIND rather than
//! guessed from an absent socket, and it says that the whole LANE in front of
//! an exit is void - no structure, and nothing beside it demanding cladding in
//! it, since the skin covers every face of structure that would otherwise look
//! at vacuum. `compatible` holds the one cell a binary rule can see and
//! `erode_blocked_exits` holds the rest of the lane; `refuse_blocked_exits`
//! fails the run if either ever misses.
//!
//! The clearance rule ITSELF is not in this file. It is
//! `nova_ship::sections::clearance`, which the EDITOR refuses placements with
//! too, so a hull the collapse may not draw is a hull a player may not build
//! either. What lives here is the reading of a ship that has been collapsed
//! onto a grid and mirrored, and the pass that takes off whatever cannot fire.
//!
//! The SKIN is not built here at all, and that is the point of the example as
//! much as a convenience. Nothing in this file places a plate, names one or
//! knows what one looks like: a ship asks for cladding with a single flag on
//! its config and the GAME derives the whole of it from the structure at spawn
//! (`nova_ship`'s `shell_skin`). A skin that closes over a hull this generator
//! never mentioned is the proof that the derivation reads structure and nothing
//! else - and `--bare` is the same ships with the flag off, which is what the
//! second shot of the pair is for.
//!
//! Nothing here masks a part's link points any more. The example used to carry
//! one mask, for the drive: its nozzle fires out of a face its six sockets
//! called a perfectly good place to bolt a hull slab. The drive now carries a
//! single socket on the flat end it mounts by, like the mount and like the bay,
//! so every part says what it is in its own link points and the generator only
//! reads them. What it does assert is that the two never contradict each other:
//! a part may not carry a socket on the face it fires through.
//!
//! What is still NOT a link-point rule, and is marked as such where it appears:
//! the draw weights, the vacuum taper that gives the silhouette a nose, the keel
//! the collapse starts from, the drive deck seeded at the far end of it, the one
//! part that may only point one way, the mirror across the centreline, and the
//! smoothing passes.
//!
//! The last two of those are what give a ship a BACK, and neither could be a
//! link-point rule. Mating is BINARY - one cell and its neighbour - so it cannot
//! hold a fact about the whole hull, and a drive carries its single socket on
//! its forward end, which means the face it is bolted to IS the direction it
//! exhausts. So the mating rule was as happy to bolt a nozzle to the roof as to
//! the transom, and a ship came out with its engines pointing six ways.
//! `Part::aim` says a drive fires AFT and nothing else, as a unary constraint on
//! the opening domains; `seed_stern` lays the aft-facing surface for it to stand
//! on, because a rule that says where a part may not go cannot conjure the place
//! where it may.
//!
//! Every ship is checked by the REAL content lint (`lint_scenario`) before it
//! is posed. A generated ship that the game's own gate would reject is a
//! broken subject, not something to photograph around, so the producer panics
//! with the findings rather than shooting it.
//!
//! Hand-run (free-fly with WASD, `R` re-rolls the whole row, `C` strips or
//! restores the cladding on the same seeds, `L` cycles the look the row wears):
//! ```text
//! cargo run --example wfc_ships --features debug
//! cargo run --example wfc_ships --features debug -- --seed 7 --ships 3
//! cargo run --example wfc_ships --features debug -- --ships 1 --bare
//! cargo run --example wfc_ships --features debug -- --style salvage
//! ```
//!
//! `L` - not `S`, which the free-fly camera owns - is what makes four authored
//! looks COMPARABLE: the same three hulls at the same pose, redressed in place,
//! instead of four checkouts and four runs.
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - collapse the row, frame it, strip the
//!   cladding, exit clean. This is the path `probe run` takes.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot the row clad and then bare
//!   (staged under `NOVA_SHOT_DIR`). Same ships in both frames, so the pair is
//!   a before and after rather than two rolls.

use std::collections::{HashSet, VecDeque};

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;
use rand::{rngs::StdRng, RngExt, SeedableRng};

#[derive(Parser)]
#[command(name = "wfc_ships")]
#[command(version = "1.0.0")]
#[command(about = "Wave-function-collapse spaceships built from link-point rules", long_about = None)]
struct Cli {
    /// First seed of the row; ship `n` collapses from `seed + n`.
    #[arg(long, default_value_t = DEFAULT_SEED)]
    seed: u64,
    /// How many ships to stand in the row.
    #[arg(long, default_value_t = DEFAULT_SHIPS)]
    ships: usize,
    /// Strip the cladding and show the bare structural collapse.
    #[arg(long)]
    bare: bool,
    /// Start on this style id instead of the first the content offers. `S`
    /// cycles from wherever this leaves the row.
    #[arg(long)]
    style: Option<String>,
}

/// The row's default first seed.
const DEFAULT_SEED: u64 = 20_260_815;
/// How many ships the row holds by default. Three of these hulls fill a 16:9
/// frame at a size where the skin is legible; more and they are thumbnails.
const DEFAULT_SHIPS: usize = 3;
/// Ships per row of the stand. Past three a row reads as a thin band in a 16:9
/// frame, so the stand wraps into a grid instead.
const COLUMNS: usize = 3;
/// Centre-to-centre spacing across a row, a little wider than the widest hull
/// the grid can produce (`2 * HALF_WIDTH` cells, plus a plate on each flank).
const SHIP_SPACING: f32 = 11.0;
/// Centre-to-centre spacing between rows, sized against hull LENGTH the same
/// way.
const ROW_SPACING: f32 = 13.0;
/// Every ship stands at the same quarter yaw, so the camera reads flank and
/// top at once and the fixed photo rig lights all of them the same way.
const SHIP_YAW: f32 = -0.5;

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    let roster = Roster {
        seed: cli.seed,
        ships: cli.ships.max(1),
        clad: !cli.bare,
        style: 0,
    };
    let requested = cli.style.clone();
    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| {
            wfc_plugin(app, roster, StyleRequest(requested.clone()))
        })
        .build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PERF_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - a
        // posed row holds no steady-state load worth measuring.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        // Clean frames at the fleet's known 16:9; dev overlays and the
        // fps/version bar out of shot (the seed readout is NOT HUD-tier, so it
        // stays - it is what makes a frame reproducible).
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        // A subject is a dynamic body, and in zero-g nothing damps a spawn
        // impulse: the row drifts and TURNS while the harness settles, so two
        // runs that take different wall-clock time photograph the same ships at
        // different attitudes. That made a pair of shots useless as an A/B of
        // anything but the clock. Pinning the bodies static makes the frame a
        // function of the seeds again.
        app.add_systems(Update, freeze_bodies.run_if(capturing));
        app.add_plugins(wfc_script());
    }

    app.run()
}

fn wfc_plugin(app: &mut App, roster: Roster, requested: StyleRequest) {
    app.insert_resource(roster);
    app.insert_resource(requested);
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_row);
    app.add_systems(
        Update,
        (
            reroll_on_key.run_if(in_state(GameStates::Playing)),
            frame_new_camera,
            update_readout,
        ),
    );
}

/// Which row is on the stage: the first seed, how many ships stand in it,
/// whether they wear their skin, and WHICH look they wear.
///
/// The look is an INDEX into the merged catalog rather than an id, because this
/// example is a producer and must not know what a style is called. `L` steps it;
/// the index is taken modulo the catalog, so it survives a mod adding one.
#[derive(Resource, Clone, Copy)]
struct Roster {
    seed: u64,
    ships: usize,
    clad: bool,
    style: usize,
}

/// The style id `--style` asked for, before the content it names has loaded.
///
/// Resolved to a `Roster` index once, on the first row: the catalog does not
/// exist when the CLI is parsed, and a producer holds an index from then on.
#[derive(Resource)]
struct StyleRequest(Option<String>);

fn load_row(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    requested: Res<StyleRequest>,
    mut roster: ResMut<Roster>,
) {
    if let Some(id) = requested.0.as_deref() {
        match styles.iter().position(|style| style.id == id) {
            Some(index) => roster.style = index,
            // Loud, not silent: a typo would otherwise photograph the first
            // look and read as the one that was asked for.
            None => panic!("--style '{id}' is not in the merged content"),
        }
    }
    let style = style_at(&styles, roster.style);
    commands.trigger(LoadScenario(wfc_row(
        &game_assets,
        &sections,
        *roster,
        style,
    )));
    spawn_readout(&mut commands);
}

/// The id of the style the row's clad ships wear, read off the merged content.
type StyleId<'a> = Option<&'a str>;

/// The style at this index of the merged catalog, wrapped.
///
/// The wrap is what lets `L` be a plain increment, and the catalog is whatever
/// the content shipped - a mod that adds a fifth look joins the rotation with
/// nothing here changing.
fn style_at(styles: &GameStyles, index: usize) -> StyleId<'_> {
    if styles.is_empty() {
        return None;
    }
    styles
        .get(index % styles.len())
        .map(|style| style.id.as_str())
}

/// `R` re-rolls the whole row: a fresh scenario off the next seed block,
/// through the same `LoadScenario` path the first row took (which tears the
/// old one down for us). `C` strips or restores the cladding on the spot, and
/// `L` steps to the next authored look on the SAME seeds - which is the only
/// way to judge four looks against one hull rather than against four hulls.
fn reroll_on_key(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    mut roster: ResMut<Roster>,
) {
    if keyboard.just_pressed(KeyCode::KeyR) {
        roster.seed = roster.seed.wrapping_add(roster.ships as u64);
    } else if keyboard.just_pressed(KeyCode::KeyC) {
        roster.clad = !roster.clad;
    } else if keyboard.just_pressed(KeyCode::KeyL) {
        roster.style = roster.style.wrapping_add(1);
        // A style change is invisible on a bare row, so asking for one asks
        // for the skin back.
        roster.clad = true;
    } else {
        return;
    }
    let style = style_at(&styles, roster.style);
    commands.trigger(LoadScenario(wfc_row(
        &game_assets,
        &sections,
        *roster,
        style,
    )));
}

// ---------------------------------------------------------------------------
// The tile set: catalog prototypes read as grid tiles.
// ---------------------------------------------------------------------------

/// The six cardinal faces of a grid cell, in the order the tile face arrays
/// use. The layout is what makes `face ^ 1` the OPPOSITE face, which is the
/// only index arithmetic the adjacency rule needs.
const FACES: [Vec3; 6] = [
    Vec3::X,
    Vec3::NEG_X,
    Vec3::Y,
    Vec3::NEG_Y,
    Vec3::Z,
    Vec3::NEG_Z,
];

/// Slack for the socket geometry the grid reads: positions and normals come
/// out of a quaternion multiply, so nothing lands exactly on 0.5.
const GRID_EPSILON: f32 = 1e-4;

/// Reciprocal of the quantum a derived placement is snapped onto - a power of
/// two, so the snap itself is exact.
///
/// Half a cell comes back out of a quaternion multiply as 0.49999997, and the
/// section then sits 6e-8 off its cell centre. That is invisible on screen and
/// fatal to the overlap lint, which compares centre distances against a sum of
/// half-extents with a strict `<`: two DIAGONAL neighbours a hair under one
/// cell apart read as interpenetrating. Snapping puts the exact value back.
const PLACEMENT_SNAP: f32 = 4096.0;

/// One prototype the generator may draw.
///
/// STRUCTURE only. There is no cladding entry and never could be: a plate's
/// shape is decided by the neighbourhood its cell sits in, and the game works
/// that out for itself once the structure exists.
struct Part {
    /// The catalog section id.
    prototype: &'static str,
    /// Draw weight for the PART, spent across whatever orientations of it are
    /// legal in a given cell (see [`draw`]). Taste, not a rule - it decides how
    /// OFTEN a part is offered, never WHERE it may go.
    weight: f32,
    /// The only face this part may fire through, in [`FACES`] order, or `None`
    /// for a part that may point any way the mating rule allows.
    ///
    /// The one place this file says a part belongs somewhere ON A SHIP rather
    /// than beside something, and the only such claim the mating rule can never
    /// make for itself: it reads one cell and its neighbour, so it cannot tell
    /// a main drive from a manoeuvring thruster. See [`aim_allowed`].
    aim: Option<usize>,
}

/// The face a ship's business end points down. The bow is `z = 0`, which is
/// where [`hull_vacuum_weight`] tapers the hull away to a nose, so aft is `+Z`.
const AFT: usize = 4;

/// The draw: the shipped prototypes a camera can see.
///
/// Every one of them renders - the thruster and controller carry no authored
/// mesh but the runtime builds them primitives (a nozzle cone, a blue hulled
/// cockpit with a window), so a cell holding one reads as a part rather than as
/// a gap. The unit-cube turret is the one weapon left out: it REPLACES a hull
/// face with a gun and a bare mounting plate, where the PDC mount sits on one.
///
/// The three fitting weights are all far above what a headcount would suggest,
/// and they have to be. A fitting is not competing with hull for a cell, it is
/// competing for a cell WITH ROOM AROUND IT: exit clearance voids its whole
/// lane, and the skin then refuses every cell the part turns a blind face to.
/// Most of the ones the draw offers are taken back off by
/// [`erode_blocked_exits`], so the number that survives is a small fraction of
/// the number rolled. Over 48 seeds, raising these three (2.5 / 0.6 / 0.9 ->
/// 3.2 / 1.4 / 1.6) against a cheaper vacuum took drives 636 -> 988, bays
/// 144 -> 338 and mounts 268 -> 526, with the hull no thinner for it
/// (6630 -> 7126 cells).
const PARTS: [Part; 5] = [
    Part {
        prototype: "reinforced_hull_section",
        weight: 6.0,
        aim: None,
    },
    Part {
        // A ship needs one, and the keel already lays one down (see
        // [`keel_prototype`]), so this weight only decides how often a hull
        // grows a second bridge blister. Harmless either way: the section is
        // the ship's heart, not a resource, and a hull carrying two simply has
        // two.
        prototype: "basic_controller_section",
        weight: 0.15,
        aim: None,
    },
    Part {
        // One socket, so its other five faces carry nothing to mate. The
        // nozzle and its plume fire aft, out of +Z - and aft is the only way
        // it may point. A drive bolts by its FORWARD end, so which way it
        // fires is decided entirely by which hull face it found, and the
        // mating rule is as happy to bolt one to the roof as to the transom.
        //
        // Priced at twice what it was, because the aim is a rule and rules are
        // paid for in weight: an aft-only drive competes for a small set of
        // cells (an aft-facing surface with its lane clear) instead of for
        // every exposed face on the ship. Measured over 12 seeds, aim alone
        // took drives 258 -> 74; at 6.4 they come back to 102, which is 8 a
        // ship in one stern bank instead of 21 scattered over six faces.
        prototype: "basic_thruster_section",
        weight: 6.4,
        aim: Some(AFT),
    },
    Part {
        // A bay is the fitting clearance costs most: a torpedo is BORN two
        // cells out, so its whole lane has to be void and nothing beside the
        // lane may want cladding in it. Most of the ones drawn are eroded.
        //
        // Left free to point anywhere, unlike the drive: a broadside tube is a
        // real warship, and nothing about a bay's direction contradicts the
        // ship's own.
        prototype: "torpedo_section",
        weight: 1.4,
        aim: None,
    },
    Part {
        // A mount punches a gun well through the skin around it, and its lane
        // is the traverse it needs to shoot along. The dearest of the three per
        // cell it occupies, which is why it is priced above the bay.
        //
        // Free for the bay's reason and one better: a turret TRAVERSES, so the
        // face it fires through is only the direction it rests at.
        prototype: "pdc_kinetic_turret_section",
        weight: 1.6,
        aim: None,
    },
];

/// Index of the vacuum tile, which [`tile_set`] always puts first.
const VACUUM: usize = 0;

/// What one cell may hold.
struct Tile {
    /// The prototype placed in the cell, or `None` for vacuum.
    part: Option<TileBody>,
    /// Which entry of [`PARTS`] this tile is an orientation of, or `None` for
    /// vacuum. Draw weights are authored per PART, so the draw needs to know
    /// which tiles are the same part wearing different rotations.
    family: Option<usize>,
    /// Per-face sockets this tile presents to its neighbours, in [`FACES`]
    /// order, read off the rotated link points.
    faces: [bool; 6],
    /// The face this tile fires, launches or exhausts through, in [`FACES`]
    /// order, or `None` for a part that does none of those. See
    /// [`exit_normal`].
    exit: Option<usize>,
}

impl Tile {
    fn is_solid(&self) -> bool {
        self.part.is_some()
    }
}

/// A prototype at one of the rotations the grid can use it at.
#[derive(Clone)]
struct TileBody {
    prototype: String,
    rotation: Quat,
    /// Where inside its cell the section sits, so that every socket it kept
    /// lands exactly on the cell face it points through. Zero for a part that
    /// fills its cell; a quarter cell for the half-size PDC mount, which
    /// stands on the boundary rather than in the middle.
    offset: Vec3,
}

/// Vacuum: the tile a cell holds when it holds nothing.
fn vacuum_tile() -> Tile {
    Tile {
        part: None,
        family: None,
        faces: [false; 6],
        exit: None,
    }
}

/// Read one prototype at one rotation as a grid tile, or reject it.
///
/// A prototype earns a place on the grid by its sockets alone: each must point
/// down a cardinal axis, sit ON that axis, and be inset from its cell face by
/// the SAME distance as every other socket the part has - because that common
/// inset is the one placement offset that puts all of them on their cell faces
/// at once. A part that cannot satisfy that (an oblique socket, two different
/// insets, a body too big for a cell) is simply not a tile, and the generator
/// never sees it.
fn tile(family: usize, config: &SectionConfig, rotation: Quat) -> Option<Tile> {
    let mut faces = [false; 6];
    let mut offset: Option<Vec3> = None;

    for point in &config.base.link_points {
        let face = face_index(rotation * point.normal)?;
        let axis = FACES[face];
        let position = rotation * point.position;
        let inset = position.dot(axis);
        if !position.abs_diff_eq(axis * inset, GRID_EPSILON) {
            return None;
        }
        let candidate = axis * (0.5 - inset);
        match offset {
            Some(previous) if !previous.abs_diff_eq(candidate, GRID_EPSILON) => return None,
            _ => offset = Some(candidate),
        }
        faces[face] = true;
    }

    // A part with no sockets at all can never mate, so it can never be part of
    // one connected ship.
    let offset = offset?;

    // The body has to stay inside its own cell. This is what makes the
    // overlap lint unreachable by construction rather than by luck: two
    // sections in different cells cannot interpenetrate if neither leaves its
    // cell, and a cell holds one section.
    let half = rotated_half_extents(config.base.collider.unwrap_or_default(), rotation);
    if (offset.abs() + half).max_element() > 0.5 + GRID_EPSILON {
        return None;
    }

    let exit = exit_normal(&config.kind).and_then(|normal| face_index(rotation * normal));
    assert!(
        !exit.is_some_and(|face| faces[face]),
        "wfc_ships: '{}' carries a socket on the face it fires through, so a \
         hull slab could be bolted across its muzzle",
        config.base.id
    );

    Some(Tile {
        part: Some(TileBody {
            prototype: config.base.id.clone(),
            rotation,
            offset: snapped(offset),
        }),
        family: Some(family),
        faces,
        exit,
    })
}

/// Put a derived placement back on the grid's exact arithmetic, see
/// [`PLACEMENT_SNAP`].
fn snapped(value: Vec3) -> Vec3 {
    (value * PLACEMENT_SNAP).round() / PLACEMENT_SNAP
}

/// The index in [`FACES`] of the cardinal direction `direction` points along,
/// or `None` if it points somewhere between two of them.
fn face_index(direction: Vec3) -> Option<usize> {
    FACES.iter().position(|face| face.dot(direction) > 0.999)
}

/// Half-extents of a collider's AABB after `rotation`, which for the cardinal
/// rotations the grid uses is just a permutation of the authored ones.
fn rotated_half_extents(collider: SectionCollider, rotation: Quat) -> Vec3 {
    let basis = Mat3::from_quat(rotation);
    Mat3::from_cols(basis.x_axis.abs(), basis.y_axis.abs(), basis.z_axis.abs())
        * collider.aabb_half_extents()
}

/// Build the tile set: vacuum first ([`VACUUM`]), then every distinct
/// ORIENTATION each drawn prototype can stand at.
///
/// Rotations are swept coarsely (three quarter-turn axes, 64 combinations) and
/// deduplicated by where they send the part's own axes, which leaves the 24
/// ways a cube can sit.
///
/// Deduplicating by SOCKET PATTERN instead would be tempting - it is all the
/// collapse can tell apart - and it was wrong. Sockets do not determine a
/// shape. A ramp blind on +Y and +X is the same socket pattern whether it lies
/// on its -Y face sloping toward +X or on its -X face sloping toward +Y, and
/// those are different models; keeping one and throwing away the other let the
/// generator plant ramps on their sides. The camera can tell orientations apart
/// even where the rule cannot, so they are kept apart.
fn tile_set(sections: &GameSections) -> Vec<Tile> {
    let quarter = std::f32::consts::FRAC_PI_2;
    let mut tiles = vec![vacuum_tile()];

    for (family, part) in PARTS.iter().enumerate() {
        let config = sections
            .get_section(part.prototype)
            .unwrap_or_else(|| panic!("wfc_ships: no section prototype '{}'", part.prototype));
        assert!(
            // The structural half is built once and reflected, so every part in
            // it has to survive the reflection.
            mirror_symmetric(config),
            "wfc_ships: '{}' has an x-asymmetric socket set, so a mirrored copy \
             of it would not carry the same sockets - see `mirrored`",
            part.prototype
        );

        let mut seen = HashSet::new();
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    let rotation = (Quat::from_rotation_x(x as f32 * quarter)
                        * Quat::from_rotation_y(y as f32 * quarter)
                        * Quat::from_rotation_z(z as f32 * quarter))
                    .normalize();
                    let Some(basis) = rotated_basis(rotation) else {
                        continue;
                    };
                    let Some(tile) = tile(family, config, rotation) else {
                        continue;
                    };
                    if seen.insert(basis) {
                        tiles.push(tile);
                    }
                }
            }
        }
    }

    tiles
}

/// Where a rotation sends the three axes, as [`FACES`] indices. Two rotations
/// with the same answer are the same rotation.
fn rotated_basis(rotation: Quat) -> Option<[usize; 3]> {
    Some([
        face_index(rotation * Vec3::X)?,
        face_index(rotation * Vec3::Y)?,
        face_index(rotation * Vec3::Z)?,
    ])
}

/// Whether a prototype's socket set survives the centreline mirror.
///
/// The structural half is built once and reflected through x = 0, and a
/// reflection is not a rotation - it can only be re-expressed as one
/// ([`mirrored`]) for a part whose sockets come in +x/-x pairs. Every shipped
/// structural prototype here does; the assertion is what stops a mod part that
/// does not from being silently mirrored into a ship whose halves no longer
/// mate.
fn mirror_symmetric(config: &SectionConfig) -> bool {
    config.base.link_points.iter().all(|point| {
        let flipped = Vec3::new(-point.normal.x, point.normal.y, point.normal.z);
        config
            .base
            .link_points
            .iter()
            .any(|other| other.normal.abs_diff_eq(flipped, GRID_EPSILON))
    })
}

/// The rotation a part reflected through x = 0 wears.
///
/// A reflection flips handedness, so it is not a rotation - but conjugating a
/// rotation by one is (the axis reflects, the angle negates), which for a
/// quaternion is just negating y and z.
fn mirrored(rotation: Quat) -> Quat {
    Quat::from_xyzw(rotation.x, -rotation.y, -rotation.z, rotation.w)
}

// ---------------------------------------------------------------------------
// The grid.
// ---------------------------------------------------------------------------

/// A rectangular block of cells with a place in ship space.
#[derive(Clone, Copy)]
struct Grid {
    size: UVec3,
    /// Ship-space centre of cell `(0, 0, 0)`.
    origin: Vec3,
}

impl Grid {
    fn cells(&self) -> usize {
        (self.size.x * self.size.y * self.size.z) as usize
    }

    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        (x * self.size.y as usize + y) * self.size.z as usize + z
    }

    fn coords(&self, cell: usize) -> (usize, usize, usize) {
        let (height, length) = (self.size.y as usize, self.size.z as usize);
        (
            cell / (height * length),
            (cell / length) % height,
            cell % length,
        )
    }

    fn centre(&self, cell: usize) -> Vec3 {
        let (x, y, z) = self.coords(cell);
        self.origin + Vec3::new(x as f32, y as f32, z as f32)
    }

    /// The neighbour across one face, or `None` at the grid edge.
    fn neighbour(&self, cell: usize, face: usize) -> Option<usize> {
        let (x, y, z) = self.coords(cell);
        let step = |value: usize, delta: i32, limit: u32| {
            let next = value as i32 + delta;
            (next >= 0 && (next as u32) < limit).then_some(next as usize)
        };
        match face {
            0 => step(x, 1, self.size.x).map(|x| self.index(x, y, z)),
            1 => step(x, -1, self.size.x).map(|x| self.index(x, y, z)),
            2 => step(y, 1, self.size.y).map(|y| self.index(x, y, z)),
            3 => step(y, -1, self.size.y).map(|y| self.index(x, y, z)),
            4 => step(z, 1, self.size.z).map(|z| self.index(x, y, z)),
            _ => step(z, -1, self.size.z).map(|z| self.index(x, y, z)),
        }
    }
}

/// Cells across the STARBOARD half; the ship is this wide either side of the
/// centreline.
///
/// Every one of these three numbers is set by the SKIN rather than by the hull.
/// A plate is a flat run only where it has neighbours on all four sides, so a
/// surface has to be at least three cells across before it has any interior at
/// all - and a hull that is all rim comes out a heap of ramps and pyramids
/// instead of a ship. Each rise here is one more row of deck on every face.
const HALF_WIDTH: usize = 4;
/// Cells tall. A flank wants a middle for the same reason a deck does.
const HEIGHT: usize = 5;
/// Cells nose to tail. Half again what the hull is wide, which is most of why
/// the results read as craft rather than as boxes.
const LENGTH: usize = 11;
/// The row the keel is laid along.
const KEEL_ROW: usize = HEIGHT / 2;

/// The half grid the structural collapse runs on. It starts half a cell off
/// the centreline, so x = 0 is the mirror plane and the two halves meet flush.
const HULL_GRID: Grid = Grid {
    size: UVec3::new(HALF_WIDTH as u32, HEIGHT as u32, LENGTH as u32),
    origin: Vec3::new(
        0.5,
        -(HEIGHT as f32 - 1.0) * 0.5,
        -(LENGTH as f32 - 1.0) * 0.5,
    ),
};

/// What the keel cell at `z` is made of.
///
/// The spine is laid down before the generator gets a say: hull the whole way,
/// with the flight computer forward of centre where a bridge reads. Taste, and
/// one guarantee - a seeded keel is one connected structure to grow on, so a
/// ship is never a cloud of islands.
///
/// The drive used to be seeded here too, at the stern, and it cannot be any
/// more: a drive carries one socket on its forward end, so it has no -x face to
/// meet its own reflection with, and the centreline is the one place on this
/// grid it may not stand. It is left to the roll instead, and lands on the
/// transom because that is the only place its one socket fits.
///
/// The keel is mirrored like everything else, so a ship carries a symmetric
/// PAIR of the computer. One would do (it is the ship's heart, not a resource)
/// and two are no worse.
fn keel_prototype(z: usize) -> &'static str {
    match z {
        z if z == LENGTH / 3 => "basic_controller_section",
        _ => "reinforced_hull_section",
    }
}

/// Base draw weight of vacuum, against the solid weights in [`PARTS`].
///
/// Low, and lower than it was before the hulls were clad. A porous hull is a
/// lattice, and skin on a lattice is one plate per strut - noise, not a
/// surface. Cladding wants broad faces to run across, so the structure it wraps
/// has to be built solid enough to have some.
///
/// Lower again once the fittings were priced up. Clearance makes a fitting
/// carry its own void with it, so a hull full of them thins out on its own;
/// buying that void back here is what keeps a well-armed ship from reading as
/// a frame with parts hung off it.
const VACUUM_BASE: f32 = 0.22;
/// How much likelier vacuum gets per cell of distance off the keel.
const VACUUM_TAPER: f32 = 1.4;
/// How much likelier vacuum gets in the LAST row, so the drives have somewhere
/// to stand.
///
/// A drive carries one socket and needs its other five faces clear, so on a
/// hull built this solid it has nowhere to be at all - three ships in a row
/// with no engines on them is what that looked like. Keeping the transom's own
/// row sparse is what gives the collapse a drive deck to put them on, and it
/// is why the engines come out as a bank of separate nacelles.
const VACUUM_STERN: f32 = 9.0;
/// How much likelier vacuum gets at the bow than at the stern. This is the
/// whole silhouette: a ship with a nose and a broad tail rather than a brick.
const VACUUM_BOW_TAPER: f32 = 24.0;

/// Draw weight of vacuum in one cell of the STRUCTURAL grid: the taper that
/// shapes the silhouette. Pure taste - it decides WHERE the generator may be
/// sparse, never WHAT may sit next to what.
fn hull_vacuum_weight(cell: usize) -> f32 {
    let (x, y, z) = HULL_GRID.coords(cell);
    let off_keel = x as f32 + (y as f32 - KEEL_ROW as f32).abs();
    let toward_bow = (LENGTH - 1 - z) as f32 / (LENGTH - 1) as f32;
    let stern = if z == LENGTH - 1 { VACUUM_STERN } else { 0.0 };
    VACUUM_BASE * (1.0 + VACUUM_TAPER * off_keel + VACUUM_BOW_TAPER * toward_bow + stern)
}

// ---------------------------------------------------------------------------
// The collapse.
// ---------------------------------------------------------------------------

/// THE RULE. A socket may never press into a face that has none, and nothing
/// may stand in front of an exit.
///
/// Two sockets facing each other is precisely what `derive_link_point_graph`
/// will later call a mate, and one socket against a face that cannot answer it
/// is the thing that must never be built. A socket facing VACUUM is fine, for
/// skin as much as for structure: most of a hull is exposed, and a tile at the
/// rim of the skin has sockets nothing answers.
///
/// Where NEITHER face has a socket the two may touch. A face without a socket
/// is not a keep-out zone, it is a face with nothing to mate: a drive's flank
/// is the side of a cylinder and a mount's is its housing, and two of those
/// resting against each other is an engine cluster rather than a fault.
///
/// CLEARANCE is the second clause, and it is why the first is not enough. A
/// muzzle, a nozzle and a bay's mouth are all faces with nothing to mate, so
/// the mating rule alone let a drive exhaust into the flank of the drive beside
/// it and a bay fire into a mount's housing. What may not be there is read off
/// the part's KIND ([`exit_normal`]) rather than guessed from the absent
/// socket, because a drive's flank has no socket either and nothing comes out
/// of it.
///
/// This is only the LOCAL half of clearance - the one cell a binary rule can
/// see. The rest of the lane, and the cladding the lane must not be given, is
/// [`erode_blocked_exits`].
fn compatible(tiles: &[Tile], here: usize, face: usize, there: usize) -> bool {
    let (here, there) = (&tiles[here], &tiles[there]);
    if !here.is_solid() || !there.is_solid() {
        return true;
    }
    if here.exit == Some(face) || there.exit == Some(face ^ 1) {
        return false;
    }
    here.faces[face] == there.faces[face ^ 1]
}

/// The centreline face, which has no neighbour cell because its neighbour is
/// this cell's own reflection. What the reflection turns toward the seam is
/// the mirror of this tile's own -x face, which carries a socket exactly when
/// this tile's -x face does - so [`compatible`] against the mirror collapses
/// to a rule about one tile: a solid on the centreline must carry a -x socket.
///
/// It must also FILL its cell. A half-size mount on the centreline would meet
/// its own reflection base to base - legal by the rule and absurd on a ship.
fn seam_allows(tile: &Tile) -> bool {
    let Some(part) = &tile.part else {
        return true;
    };
    tile.faces[1] && part.offset.x.abs() < GRID_EPSILON
}

/// Whether a tile points the way its part is allowed to point
/// ([`Part::aim`]).
///
/// A UNARY constraint, like [`seam_allows`] beside it, and the only thing in
/// this file that knows a ship has a BACK. [`compatible`] is binary: it sees a
/// cell and its neighbour, and a drive bolted to the roof is as legal to it as
/// one bolted to the transom. Which way a drive fires is not a free choice
/// either - it carries one socket, on its forward end, so the face it is bolted
/// to IS the direction it exhausts. Six drives on one hull facing six ways is
/// what that adds up to.
///
/// Free, and safe by construction. Striking options out of an OPENING domain is
/// the standard way to add a unary constraint to a constraint solve, and it
/// cannot empty a domain here because [`VACUUM`] is compatible with everything
/// and is never struck. There is no backtracking in this collapse and none is
/// needed.
fn aim_allowed(tile: &Tile) -> bool {
    tile.family
        .and_then(|family| PARTS[family].aim)
        .is_none_or(|aim| tile.exit == Some(aim))
}

/// The structural pass's opening domains: structure only, the seam and the aim
/// ruled on as unary constraints, every other grid edge treated as the vacuum
/// it is.
fn hull_domains(tiles: &[Tile]) -> Vec<Vec<bool>> {
    (0..HULL_GRID.cells())
        .map(|cell| {
            let on_seam = HULL_GRID.coords(cell).0 == 0;
            (0..tiles.len())
                .map(|index| {
                    if !aim_allowed(&tiles[index]) {
                        return false;
                    }
                    if on_seam && !seam_allows(&tiles[index]) {
                        return false;
                    }
                    (0..FACES.len()).all(|face| {
                        (face == 1 && on_seam)
                            || HULL_GRID.neighbour(cell, face).is_some()
                            || compatible(tiles, index, face, VACUUM)
                    })
                })
                .collect()
        })
        .collect()
}

/// The tile that places `prototype` unrotated - the orientation that faces the
/// way the ship does, which for the drive is the nozzle pointing aft. Always
/// present: the rotation sweep starts from the identity.
fn upright_tile(tiles: &[Tile], prototype: &str) -> usize {
    tiles
        .iter()
        .position(|tile| {
            tile.part.as_ref().is_some_and(|part| {
                part.prototype == prototype
                    && part.rotation.abs_diff_eq(Quat::IDENTITY, GRID_EPSILON)
            })
        })
        .unwrap_or_else(|| panic!("wfc_ships: no upright tile for '{prototype}'"))
}

/// Lay the keel: collapse the spine by hand, before the generator gets a say.
///
/// It stops ONE CELL SHORT of the transom, and that cell belongs to
/// [`seed_stern`]: the drive standing beside it turns a blind flank at the
/// seam, and a keel cube there would press a socket into it.
fn seed_keel(tiles: &[Tile], domains: &mut [Vec<bool>]) {
    for z in 0..LENGTH - 1 {
        let cell = HULL_GRID.index(0, KEEL_ROW, z);
        let keel = upright_tile(tiles, keel_prototype(z));
        assert!(
            domains[cell][keel],
            "a keel section sits ON the centreline, so it has to be able to \
             meet its own reflection: `{}` cannot",
            keel_prototype(z)
        );
        assign(domains, cell, keel);
    }
}

/// Lay the DRIVE DECK: a block beside the last keel cell and a nozzle bolted to
/// its aft face, which the mirror makes a pair either side of the centreline.
///
/// Taste and one guarantee, exactly as [`seed_keel`] is: the keel says a ship is
/// one connected structure, and this says it has an engine at the back of it.
/// [`Part::aim`] alone does not - it says where a drive may NOT go, and a hull
/// with no aft-facing surface simply comes out with no engines. The seed is what
/// makes the aft-facing surface, and the roll then fills the transom around it.
///
/// A drive may not stand ON the centreline: its one socket is on its forward
/// face, so it has no `-x` socket to meet its own reflection with
/// ([`seam_allows`]). So it stands one cell off, and the keel stops one cell
/// short of the transom to leave the seam cell beside it free - a hull cube
/// there would press a socket into the drive's blind flank.
fn seed_stern(tiles: &[Tile], domains: &mut [Vec<bool>]) {
    for (cell, prototype) in [
        (
            HULL_GRID.index(1, KEEL_ROW, LENGTH - 2),
            "reinforced_hull_section",
        ),
        (
            HULL_GRID.index(1, KEEL_ROW, LENGTH - 1),
            "basic_thruster_section",
        ),
    ] {
        let tile = upright_tile(tiles, prototype);
        assert!(
            domains[cell][tile],
            "the drive deck is seeded before anything is drawn, so `{prototype}` \
             has to stand there",
        );
        assign(domains, cell, tile);
    }
}

/// Collapse a grid: one tile index per cell.
///
/// `vacuum_weight` prices emptiness per cell and `bias` prices one tile against
/// the other orientations of its own part. Both are taste; neither can make an
/// illegal placement legal, because the domain they draw from was already
/// filtered by the rule.
fn collapse<W: Fn(usize) -> f32, B: Fn(usize, usize) -> f32>(
    grid: &Grid,
    tiles: &[Tile],
    mut domains: Vec<Vec<bool>>,
    vacuum_weight: W,
    bias: B,
    seed: u64,
) -> Vec<usize> {
    let mut rng = StdRng::seed_from_u64(seed);
    propagate(grid, tiles, &mut domains, (0..grid.cells()).collect());

    while let Some(cell) = lowest_entropy(&domains, &mut rng) {
        let pick = draw(tiles, &domains[cell], cell, &vacuum_weight, &bias, &mut rng);
        assign(&mut domains, cell, pick);
        propagate(grid, tiles, &mut domains, VecDeque::from([cell]));
    }

    domains
        .iter()
        .enumerate()
        .map(|(cell, domain)| {
            domain
                .iter()
                .position(|allowed| *allowed)
                .unwrap_or_else(|| panic!("wfc_ships: cell {cell} collapsed to nothing"))
        })
        .collect()
}

fn assign(domains: &mut [Vec<bool>], cell: usize, tile: usize) {
    for (index, allowed) in domains[cell].iter_mut().enumerate() {
        *allowed = index == tile;
    }
}

/// Arc-consistency: strike from every neighbour the options nothing left in
/// this cell can sit beside, and follow the change outward.
fn propagate(grid: &Grid, tiles: &[Tile], domains: &mut [Vec<bool>], mut pending: VecDeque<usize>) {
    while let Some(cell) = pending.pop_front() {
        for face in 0..FACES.len() {
            let Some(next) = grid.neighbour(cell, face) else {
                continue;
            };
            let mut changed = false;
            for candidate in 0..tiles.len() {
                if !domains[next][candidate] {
                    continue;
                }
                let supported = (0..tiles.len())
                    .any(|here| domains[cell][here] && compatible(tiles, here, face, candidate));
                if !supported {
                    domains[next][candidate] = false;
                    changed = true;
                }
            }
            if changed {
                pending.push_back(next);
            }
        }
    }
}

/// The next cell to observe: fewest options left, ties broken by the roll.
fn lowest_entropy(domains: &[Vec<bool>], rng: &mut StdRng) -> Option<usize> {
    let mut fewest = usize::MAX;
    let mut candidates = Vec::new();
    for (cell, domain) in domains.iter().enumerate() {
        let options = domain.iter().filter(|allowed| **allowed).count();
        if options < 2 {
            continue;
        }
        if options < fewest {
            fewest = options;
            candidates.clear();
        }
        if options == fewest {
            candidates.push(cell);
        }
    }
    (!candidates.is_empty()).then(|| candidates[rng.random_range(0..candidates.len())])
}

/// Draw one of a cell's remaining options, by weight.
///
/// A weight is authored per PART and spent per part: whatever orientations of
/// it are still standing in this cell SHARE it. That keeps the authored number
/// meaning "how often do I want one of these, where one is allowed" - immune
/// both to how symmetric a part happens to be (the hull cube presents one tile,
/// the bay six) and to how many of its orientations this particular cell has
/// already ruled out. Without it a mount, which by its single socket has at
/// most one legal orientation anywhere, would be outvoted six to one by parts
/// that merely have more ways to sit.
///
/// `bias` then divides a part's share between those orientations. It is the
/// only thing that can, because the rule cannot tell them apart at all.
fn draw<W: Fn(usize) -> f32, B: Fn(usize, usize) -> f32>(
    tiles: &[Tile],
    domain: &[bool],
    cell: usize,
    vacuum_weight: &W,
    bias: &B,
    rng: &mut StdRng,
) -> usize {
    let mut standing = [0.0f32; PARTS.len()];
    for tile in (0..tiles.len()).filter(|index| domain[*index]) {
        if let Some(family) = tiles[tile].family {
            standing[family] += bias(tile, cell);
        }
    }
    let weight = |index: usize| match tiles[index].family {
        Some(family) => PARTS[family].weight * bias(index, cell) / standing[family],
        None => vacuum_weight(cell),
    };
    let total: f32 = (0..tiles.len())
        .filter(|index| domain[*index])
        .map(weight)
        .sum();
    let mut roll = rng.random_range(0.0..total);
    let mut last = 0;
    for index in (0..tiles.len()).filter(|index| domain[*index]) {
        roll -= weight(index);
        if roll <= 0.0 {
            return index;
        }
        last = index;
    }
    last
}

/// How many neighbours a section wants before it counts as part of a hull
/// rather than as a stud on one.
const SPIKE_SUPPORT: usize = 3;

/// Erode the studs off the structure.
///
/// This is here because of the SKIN. A hull cell hanging off the ship by one
/// face is a stud, and the coverage rule then wraps it in five plates - which
/// is a pyramid standing on a pin. Cladding does not hide a lumpy hull, it
/// draws a hard outline around one, so the lumps are taken off before the skin
/// goes on rather than dressed afterwards.
///
/// What counts as enough is read off the section, not authored: a part is asked
/// for two neighbours or for as many as it has sockets, whichever is fewer. A
/// PDC mount carries one socket and so is never eroded for having one
/// neighbour; a six-socket hull cube standing on one is.
///
/// Erosion only removes, and a removal can only lower a neighbour's count, so
/// it settles. It can strand a limb, which is why [`keel_component`] runs
/// after.
fn erode_studs(tiles: &[Tile], chosen: &[usize], kept: &mut [bool]) {
    loop {
        let mut changed = false;
        for cell in 0..HULL_GRID.cells() {
            let (x, y, _) = HULL_GRID.coords(cell);
            if !kept[cell] || (x == 0 && y == KEEL_ROW) {
                continue;
            }
            let tile = &tiles[chosen[cell]];
            if !tile.is_solid() {
                continue;
            }
            // A cell on the centreline meets its own reflection there, and the
            // seam rule already guarantees that mate.
            let mut neighbours = usize::from(x == 0);
            for face in 0..FACES.len() {
                if let Some(next) = HULL_GRID.neighbour(cell, face) {
                    neighbours += usize::from(kept[next] && tiles[chosen[next]].is_solid());
                }
            }
            let wanted = tile.faces.iter().filter(|open| **open).count();
            if neighbours < wanted.min(SPIKE_SUPPORT) {
                kept[cell] = false;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

/// How many solid faces a hole needs before it stops being a feature.
const FILL_SUPPORT: usize = 3;

/// Fill the pits.
///
/// The other half of [`erode_studs`], and there for the same reason. A one-cell
/// dent in the hull is a hole the skin then has to line with five plates, and
/// lining dents is how a hull ends up looking like a sponge. Any empty cell with
/// [`FILL_SUPPORT`] solid faces around it is packed with hull instead - cheaper
/// in sections, and far cheaper to look at.
///
/// A cell that any section turns a BLIND face to is never filled. Those are the
/// gun wells and the muzzle mouths, and they are dents on purpose: the same
/// sentence that keeps the skin out of them keeps the filler out too. Filling
/// is otherwise always legal - a hull cube offers a socket on all six faces, and
/// every neighbour it meets was checked to offer one back.
fn fill_pits(tiles: &[Tile], chosen: &mut [usize], kept: &mut [bool], lanes: &HashSet<IVec3>) {
    let hull = upright_tile(tiles, "reinforced_hull_section");
    loop {
        let mut changed = false;
        for cell in 0..HULL_GRID.cells() {
            if kept[cell] && tiles[chosen[cell]].is_solid() {
                continue;
            }
            // Filling is the one pass that ADDS structure, so it is the one
            // that can block a lane after the lanes have been cleared. A hull
            // cube offers a socket on all six faces, so packing a dent BESIDE a
            // lane demands cladding inside it - which is the same conflict as
            // packing the lane itself.
            let (x, y, z) = HULL_GRID.coords(cell);
            let here = IVec3::new(x as i32, y as i32, z as i32);
            if lanes.contains(&here)
                || FACES
                    .iter()
                    .any(|face| lanes.contains(&(here + face.as_ivec3())))
            {
                continue;
            }
            // A cell on the centreline is backed by its own reflection.
            let mut solid = usize::from(x == 0);
            let mut blind = false;
            for face in 0..FACES.len() {
                let Some(next) = HULL_GRID.neighbour(cell, face) else {
                    continue;
                };
                if !kept[next] || !tiles[chosen[next]].is_solid() {
                    continue;
                }
                solid += 1;
                blind |= !tiles[chosen[next]].faces[face ^ 1];
            }
            if !blind && solid >= FILL_SUPPORT {
                chosen[cell] = hull;
                kept[cell] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

/// Keep only the structure hanging off the keel.
///
/// The collapse can leave an island: a mount whose one socket faced a cell
/// that ended up empty, or a pocket of hull cut off by vacuum. Those are legal
/// tiles and an ILLEGAL ship - `derive_link_point_graph` rejects a
/// disconnected section graph outright - so they are dropped here rather than
/// linted about later. Dropping solids only ever turns a mate into an exposed
/// socket, which no rule forbids, so what survives is still valid.
///
/// Adjacency is enough to walk: the rule guarantees two adjacent solids mate.
/// The skin needs no pass of its own - a plate has to bolt to structure, so it
/// cannot float in the first place.
fn keel_component(tiles: &[Tile], chosen: &[usize], standing: &[bool]) -> Vec<bool> {
    let mut kept = vec![false; HULL_GRID.cells()];
    let start = HULL_GRID.index(0, KEEL_ROW, 0);
    let mut pending = VecDeque::from([start]);
    kept[start] = true;
    while let Some(cell) = pending.pop_front() {
        for face in 0..FACES.len() {
            let Some(next) = HULL_GRID.neighbour(cell, face) else {
                continue;
            };
            // MATING, not adjacency. Two solids may now sit face to face
            // without either carrying a socket there - a drive's flank against
            // a mount's housing - and a pair like that is touching, not joined.
            // Walking plain adjacency would call such a piece attached and
            // leave it in the hull for the graph check to find floating.
            let joined = tiles[chosen[cell]].faces[face] && tiles[chosen[next]].faces[face ^ 1];
            if !kept[next] && standing[next] && tiles[chosen[next]].is_solid() && joined {
                kept[next] = true;
                pending.push_back(next);
            }
        }
    }
    kept
}

// ---------------------------------------------------------------------------
// Exit clearance.
// ---------------------------------------------------------------------------

/// One placed part of the WHOLE ship - both halves, in cells - as the skin
/// reads it.
struct ShipCell {
    cell: IVec3,
    prototype: &'static str,
    faces: [bool; 6],
    exit: Option<usize>,
}

/// The face index a reflection through x = 0 sends `face` to.
fn mirror_face(face: usize) -> usize {
    match face / 2 {
        0 => face ^ 1,
        _ => face,
    }
}

/// The collapsed half mirrored into the whole ship, in cells.
///
/// The cells are the ones the SKIN will bucket the spawned sections into: the
/// starboard half stands at `x + 0.5` and the port half at `-x - 0.5`, so on a
/// lattice with a half-cell phase they come out as `x` and `-1 - x`.
///
/// Clearance has to be judged on the whole ship rather than on the half that
/// was collapsed, because a lane pointing at the centreline crosses it - and
/// what is waiting on the far side is the part's own reflection, firing back.
fn ship_cells(tiles: &[Tile], chosen: &[usize], kept: &[bool]) -> Vec<ShipCell> {
    let mut cells = Vec::new();
    for cell in 0..HULL_GRID.cells() {
        if !kept[cell] || !tiles[chosen[cell]].is_solid() {
            continue;
        }
        let tile = &tiles[chosen[cell]];
        let family = tile
            .family
            .expect("a solid tile is an orientation of a part");
        let (x, y, z) = HULL_GRID.coords(cell);
        let (x, y, z) = (x as i32, y as i32, z as i32);
        cells.push(ShipCell {
            cell: IVec3::new(x, y, z),
            prototype: PARTS[family].prototype,
            faces: tile.faces,
            exit: tile.exit,
        });
        cells.push(ShipCell {
            cell: IVec3::new(-1 - x, y, z),
            prototype: PARTS[family].prototype,
            faces: std::array::from_fn(|face| tile.faces[mirror_face(face)]),
            exit: tile.exit.map(mirror_face),
        });
    }
    cells
}

/// The half-grid cell a mirrored one is a copy of.
fn starboard_of(cell: IVec3) -> usize {
    let x = if cell.x < 0 { -1 - cell.x } else { cell.x };
    HULL_GRID.index(x as usize, cell.y as usize, cell.z as usize)
}

/// The mirrored ship as `nova_ship`'s clearance rule reads it: what each cell
/// presents, and the exits those cells carry.
///
/// The RULE is not here. It is `nova_ship::sections::clearance`, which the
/// editor refuses placements with, so a hull this generator cannot draw is also
/// a hull a player cannot build. What is here is only the reading of a ship
/// that has been collapsed onto a grid and mirrored.
fn ship_lattice(cells: &[ShipCell]) -> (SkinStructure, Vec<ShipExit>) {
    let mut structure = SkinStructure::default();
    for placed in cells {
        structure.insert(placed.cell, placed.faces);
        if let Some(out) = placed.exit {
            structure.insert_exit(placed.cell, out);
        }
    }
    let exits = cells
        .iter()
        .filter_map(|placed| {
            Some(ShipExit {
                cell: placed.cell,
                out: placed.exit?,
            })
        })
        .collect();
    (structure, exits)
}

/// One finding, spelled with the prototypes standing in it.
fn blocked_exit_finding(cells: &[ShipCell], blocked: &BlockedExit) -> String {
    let named = |cell: IVec3| -> &str {
        cells
            .iter()
            .find(|placed| placed.cell == cell)
            .map(|placed| placed.prototype)
            .unwrap_or("vacuum")
    };
    match blocked.reason {
        BlockedExitReason::Structure => format!(
            "`{}` at {:?} fires into `{}` at {:?}",
            named(blocked.part),
            blocked.part,
            named(blocked.blocker),
            blocked.lane,
        ),
        BlockedExitReason::Cladding => format!(
            "`{}` at {:?} fires, and `{}` at {:?} wants cladding in the lane at {:?}",
            named(blocked.part),
            blocked.part,
            named(blocked.blocker),
            blocked.blocker,
            blocked.lane,
        ),
    }
}

/// Take off the parts that cannot fire, launch or exhaust where they stand.
///
/// The non-local half of clearance, and it has to be a PASS rather than a rule
/// because no rule between two neighbouring cells can see a lane. It belongs
/// beside [`erode_studs`] for the same reason that one does: the collapse says
/// what may sit next to what, and these two say what a hull is allowed to look
/// like once it has.
///
/// The PART goes, never the hull in front of it. Dropping a solid only ever
/// turns a mate into an exposed socket, which no rule forbids, and dropping the
/// hull instead would open a lane by gutting the ship it is bolted to. One part
/// per pass, so a bay blocked only by another bay is not taken off with it.
///
/// Removal cannot create work: taking a part away deletes its lane and frees
/// everyone else's, so this settles.
fn erode_blocked_exits(tiles: &[Tile], chosen: &[usize], kept: &mut [bool]) {
    loop {
        let (structure, exits) = ship_lattice(&ship_cells(tiles, chosen, kept));
        let Some(blocked) = blocked_exits(&structure, &exits)
            .into_iter()
            .min_by_key(|blocked| starboard_of(blocked.part))
        else {
            return;
        };
        kept[starboard_of(blocked.part)] = false;
    }
}

/// Refuse a ship whose exits are blocked, in the same spirit as
/// [`refuse_broken_ships`]: a generator that can draw a bay firing into its own
/// hull has to FAIL the run rather than photograph it.
///
/// This is a check, not the fix. [`compatible`] and [`erode_blocked_exits`] are
/// what make it pass; if it ever fires, one of those two is wrong.
fn refuse_blocked_exits(cells: &[ShipCell], seed: u64) {
    let (structure, exits) = ship_lattice(cells);
    let blocked: Vec<String> = blocked_exits(&structure, &exits)
        .iter()
        .map(|blocked| blocked_exit_finding(cells, blocked))
        .collect();
    assert!(
        blocked.is_empty(),
        "wfc_ships: seed {seed}: the collapse blocked its own exits:\n  {}",
        blocked.join("\n  "),
    );
}

// ---------------------------------------------------------------------------
// The scenario.
// ---------------------------------------------------------------------------

/// One collapsed ship: structure mirrored into a whole hull, and a flag asking
/// the game to clad it.
fn wfc_ship(tiles: &[Tile], seed: u64, roster: Roster, style: StyleId) -> SpaceshipConfig {
    let mut domains = hull_domains(tiles);
    seed_keel(tiles, &mut domains);
    seed_stern(tiles, &mut domains);
    let chosen = collapse(
        &HULL_GRID,
        tiles,
        domains,
        hull_vacuum_weight,
        |_, _| 1.0,
        seed,
    );
    // Prune, then smooth: take the studs off, take off whatever cannot fire
    // where it stands, pack the dents, and prune again because erosion can
    // strand whatever a stud was holding on.
    //
    // Clearing the lanes BEFORE the dents are packed is what keeps the hull
    // solid: the cell a blocked bay is taken out of is a dent like any other,
    // and packing it afterwards leaves hull rather than a hole for the skin to
    // line. The filler is handed the surviving lanes so it cannot undo them.
    let mut chosen = chosen;
    let mut kept = keel_component(tiles, &chosen, &vec![true; HULL_GRID.cells()]);
    erode_studs(tiles, &chosen, &mut kept);
    erode_blocked_exits(tiles, &chosen, &mut kept);
    let (structure, exits) = ship_lattice(&ship_cells(tiles, &chosen, &kept));
    let lanes: HashSet<IVec3> = exit_lanes(&structure, &exits).into_iter().collect();
    fill_pits(tiles, &mut chosen, &mut kept, &lanes);
    let kept = keel_component(tiles, &chosen, &kept);
    refuse_blocked_exits(&ship_cells(tiles, &chosen, &kept), seed);

    let mut sections = Vec::new();
    for cell in 0..HULL_GRID.cells() {
        if !kept[cell] {
            continue;
        }
        let Some(part) = &tiles[chosen[cell]].part else {
            continue;
        };
        let (x, y, z) = HULL_GRID.coords(cell);
        let position = HULL_GRID.centre(cell) + part.offset;
        let source = SectionSource::Prototype(part.prototype.clone());
        sections.push(SpaceshipSectionConfig {
            id: format!("starboard_{x}_{y}_{z}"),
            position,
            rotation: part.rotation,
            source: source.clone(),
            modifications: vec![],
        });
        sections.push(SpaceshipSectionConfig {
            id: format!("port_{x}_{y}_{z}"),
            position: Vec3::new(-position.x, position.y, position.z),
            rotation: mirrored(part.rotation),
            source,
            modifications: vec![],
        });
    }

    SpaceshipConfig {
        // The whole of the skin, from this side. The plates, their shapes and
        // where they stand are the game's business, derived from the sections
        // above at spawn - which is what makes the clad frame evidence rather
        // than a picture of what this file already decided.
        skin: roster.clad,
        // The look, by id, out of the MERGED content - never a literal. A clad
        // ship wears whatever style the content shipped, so nothing in this
        // file names a greeble, a rule or a cell, and a mod that overlays the
        // style changes the shot without changing the example.
        style: roster.clad.then_some(style).flatten().map(str::to_string),
        collapse_threshold: None,
        allegiance: None,
        // Scenery: these are subjects, not craft. Nothing flies them and
        // nothing shoots them.
        controller: SpaceshipController::None,
        sections,
    }
}

/// Where one ship stands on the grid stand: filled row by row, each row
/// centred on its own count so a short last row does not hang off one side.
fn stand_position(index: usize, ships: usize) -> Vec3 {
    let row = index / COLUMNS;
    let rows = ships.div_ceil(COLUMNS);
    let in_row = (ships - row * COLUMNS).min(COLUMNS);
    let column = index % COLUMNS;
    Vec3::new(
        (column as f32 - (in_row as f32 - 1.0) * 0.5) * SHIP_SPACING,
        0.0,
        (row as f32 - (rows as f32 - 1.0) * 0.5) * ROW_SPACING,
    )
}

/// The stage: a row of collapsed ships under the game's own sky and the
/// repo's standard three-point rig, so the subjects render the way the game
/// would draw them.
fn wfc_row(
    game_assets: &GameAssets,
    sections: &GameSections,
    roster: Roster,
    style: StyleId,
) -> ScenarioConfig {
    let tiles = tile_set(sections);
    let ships = (0..roster.ships).map(|index| {
        let seed = roster.seed.wrapping_add(index as u64);
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: format!("wfc_ship_{index}"),
                name: format!("WFC {seed}"),
                position: stand_position(index, roster.ships),
                rotation: Quat::from_rotation_y(SHIP_YAW),
            },
            kind: ScenarioObjectKind::Spaceship(wfc_ship(&tiles, seed, roster, style)),
        })
    });

    let scenario = ScenarioConfig {
        description: "Wave-function-collapse ships built from link-point rules".to_string(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: ships
                .chain(ThreePointRig::around("photo", Vec3::ZERO, 3.0).actions())
                .collect(),
        }],
        ..ScenarioConfig::new(
            "wfc_ships".to_string(),
            "WFC Ships".to_string(),
            game_assets.cubemap.clone().into(),
        )
    };
    refuse_broken_ships(&scenario, sections);
    scenario
}

/// Slack for deciding two bodies are flush, against snapped placements.
const CONTACT_EPSILON: f32 = 1e-3;

/// The face of `a` that `b` lies flush against, or `None` if the two bodies do
/// not meet face to face.
///
/// Face to face means what it says: flush on ONE axis and overlapping on the
/// other two. Two sections that share only an edge or a corner are not in
/// contact, and neither is a pair that meets on two axes at once.
fn contact_face(a: (Vec3, Vec3), b: (Vec3, Vec3)) -> Option<usize> {
    let delta = (b.0 - a.0).to_array();
    let reach = (a.1 + b.1).to_array();
    let mut face = None;
    for axis in 0..3 {
        let gap = delta[axis].abs() - reach[axis];
        if gap > CONTACT_EPSILON {
            return None;
        }
        if gap >= -CONTACT_EPSILON {
            if face.is_some() {
                return None;
            }
            face = Some(axis * 2 + usize::from(delta[axis] < 0.0));
        }
    }
    face
}

/// Whether a placed section carries a socket on the cell face `face`, in ship
/// space. Read off the placement rather than off the prototype, because which
/// face a socket ends up on is decided by the rotation.
fn offers(section: &Placed, face: usize) -> bool {
    section
        .config
        .base
        .link_points
        .iter()
        .any(|point| (section.rotation * point.normal).dot(FACES[face]) > 0.5)
}

/// One section as the graph derivation and the contact test both want it.
struct Placed<'a> {
    config: &'a SectionConfig,
    position: Vec3,
    rotation: Quat,
    /// Centre and half-extents of the body, in ship space.
    body: (Vec3, Vec3),
}

fn place<'a>(ship: &SpaceshipConfig, sections: &'a GameSections) -> Vec<Placed<'a>> {
    ship.sections
        .iter()
        .map(|section| {
            let SectionSource::Prototype(id) = &section.source else {
                panic!("wfc_ships: every generated section is a catalog prototype");
            };
            let config = sections
                .get_section(id)
                .unwrap_or_else(|| panic!("wfc_ships: no section prototype '{id}'"));
            Placed {
                config,
                position: section.position,
                rotation: section.rotation,
                body: (
                    section.position,
                    rotated_half_extents(
                        config.base.collider.unwrap_or_default(),
                        section.rotation,
                    ),
                ),
            }
        })
        .collect()
}

/// Every pair of sections that touch has to MATE, unless neither of them has
/// anything at the face to mate WITH.
///
/// The game's own graph check only asks for one connected component, so a
/// section that meets two others and bolts to one of them passes it. That is
/// not what anyone means by fitting: fill the inside corner of an L and the
/// piece has to answer both arms, not whichever arm it found first. That is
/// what this enforces, and it is the check that would catch a multi-socket part
/// whose sockets do not line up where its faces do.
///
/// The exemption is the one [`compatible`] already makes, held to from the
/// other side. A cladding tile's surface dies to the cell floor along a dead
/// edge, so it has no material at that face and offers no socket there. Two
/// tiles like that meet as plates resting together - each bolted to the hull
/// through its own floor, neither carrying the other - and there is no socket
/// on either side left unanswered. Demanding a mate there would be demanding a
/// socket on nothing. What is NOT exempt is a socket on one side and a bare
/// face on the other: that is the plug pressed into a blank face, and the
/// second half of this function refuses it wherever it appears.
fn refuse_unmated_contacts(placed: &[Placed], ship: &SpaceshipConfig) {
    let points: Vec<PlacedSectionLinkPoints> = placed
        .iter()
        .map(|section| PlacedSectionLinkPoints {
            position: section.position,
            rotation: section.rotation,
            link_points: &section.config.base.link_points,
        })
        .collect();
    let mates = derive_link_point_graph(&points)
        .unwrap_or_else(|error| panic!("wfc_ships: the collapse built no graph: {error:?}"));
    let mated: HashSet<(usize, usize)> = mates
        .iter()
        .map(|mate| (mate.a.section_index, mate.b.section_index))
        .collect();

    let mut unmated = Vec::new();
    for (a, first) in placed.iter().enumerate() {
        for (b, second) in placed.iter().enumerate().skip(a + 1) {
            let Some(face) = contact_face(first.body, second.body) else {
                continue;
            };
            if !offers(first, face) && !offers(second, face ^ 1) {
                continue;
            }
            if !mated.contains(&(a, b)) && !mated.contains(&(b, a)) {
                unmated.push(format!(
                    "`{}` and `{}` are flush and do not mate",
                    ship.sections[a].id, ship.sections[b].id
                ));
            }
        }
    }

    // And the same claim from the other end: a socket may point into vacuum,
    // but it may NOT point into another section's body without mating it. That
    // is what a connection which does not match looks like from the link
    // points' side - a plug pressed into a blank face - and the pair test above
    // cannot see it, because two bodies can be flush over a whole face while
    // the sockets that were supposed to answer each other sit somewhere else on
    // it.
    let answered: HashSet<LinkPointRef> = mates.iter().flat_map(|mate| [mate.a, mate.b]).collect();
    for (index, section) in placed.iter().enumerate() {
        for (socket, point) in section.config.base.link_points.iter().enumerate() {
            let reference = LinkPointRef {
                section_index: index,
                link_point_index: socket,
            };
            if answered.contains(&reference) {
                continue;
            }
            let probe = section.position
                + section.rotation * point.position
                + section.rotation * point.normal * SOCKET_PROBE;
            let Some(other) = placed
                .iter()
                .position(|body| body_holds(body.body, probe) && !std::ptr::eq(body, section))
            else {
                continue;
            };
            unmated.push(format!(
                "`{}` socket `{}` presses into `{}` with nothing to mate",
                ship.sections[index].id, point.id, ship.sections[other].id
            ));
        }
    }

    assert!(
        unmated.is_empty(),
        "wfc_ships: sections meet without mating:\n  {}",
        unmated.join("\n  ")
    );
}

/// How far past a socket to look for the body it should have mated.
const SOCKET_PROBE: f32 = 0.05;

/// Whether a point is strictly inside a body.
fn body_holds((centre, half): (Vec3, Vec3), point: Vec3) -> bool {
    (point - centre)
        .abs()
        .cmplt(half - Vec3::splat(CONTACT_EPSILON))
        .all()
}

/// Run the row through the game's OWN content gate before posing it.
///
/// This is the claim the generator actually makes: not "the tiles fit" but
/// "what came out is a ship the game would accept" - one connected link-point
/// graph per hull, no socket with two suitors, no unmated overlap. Nothing
/// here re-implements those checks; `lint_scenario` is the same function the
/// `content lint` gate and the runtime loader run. What IS extra is
/// [`refuse_unmated_contacts`], which asks for something the game does not.
fn refuse_broken_ships(scenario: &ScenarioConfig, sections: &GameSections) {
    let known = KnownSections::from_configs(sections.iter());
    let issues = lint_scenario(scenario, &known, &HashSet::from([scenario.id.clone()]));
    let errors: Vec<&str> = issues
        .iter()
        .filter(|issue| issue.severity == LintSeverity::Error)
        .map(|issue| issue.message.as_str())
        .collect();
    assert!(
        errors.is_empty(),
        "wfc_ships: the collapse produced content the game would refuse:\n  {}",
        errors.join("\n  ")
    );

    let hulls: Vec<&SpaceshipConfig> = scenario.events[0]
        .actions
        .iter()
        .filter_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object) => match &object.kind {
                ScenarioObjectKind::Spaceship(ship) => Some(ship),
                _ => None,
            },
            _ => None,
        })
        .collect();
    let count = |prototype: &str| {
        hulls
            .iter()
            .flat_map(|ship| &ship.sections)
            .filter(|section| {
                matches!(&section.source, SectionSource::Prototype(id) if id == prototype)
            })
            .count()
    };
    let structure = PARTS
        .iter()
        .map(|part| format!("{}x {}", count(part.prototype), part.prototype))
        .collect::<Vec<_>>()
        .join(", ");
    for ship in &hulls {
        refuse_unmated_contacts(&place(ship, sections), ship);
    }

    info!(
        "wfc_ships: {} ship(s) lint clean, every contact mates, {} sections, \
         {} warning(s), skin {}\n  structure: {}",
        hulls.len(),
        hulls.iter().map(|ship| ship.sections.len()).sum::<usize>(),
        issues.len(),
        if hulls.iter().all(|ship| ship.skin) {
            "derived at spawn"
        } else {
            "off"
        },
        structure,
    );
}

// ---------------------------------------------------------------------------
// Framing and readout.
// ---------------------------------------------------------------------------

/// What the camera aims at: the middle of the stand.
const CAMERA_TARGET: Vec3 = Vec3::ZERO;

/// Where the camera stands for a row of `ships`, backed off far enough to hold
/// whatever [`stand_position`] laid out. High and in front: the interesting
/// face of these hulls is the top skin, where the mounts and the bays surface.
///
/// Derived rather than pinned so `--ships 1` is a close-up of one hull and not
/// a speck in a frame composed for six.
fn camera_position(ships: usize) -> Vec3 {
    // One ship of margin on the count: the stand is as wide as the gaps BETWEEN
    // its ships plus the hulls on the ends, and a hull is about a gap wide.
    let columns = ships.min(COLUMNS) as f32 + 1.0;
    let rows = ships.div_ceil(COLUMNS) as f32 + 1.0;
    let span = (columns * SHIP_SPACING).max(rows * ROW_SPACING);
    Vec3::new(0.0, span * 0.34, span * 0.58)
}

/// Frame every camera the loader spawns, so a re-roll comes back to the same
/// composition instead of the loader's default perch.
fn frame_new_camera(
    roster: Res<Roster>,
    mut q_camera: Query<&mut Transform, (With<ScenarioCameraMarker>, Added<ScenarioCameraMarker>)>,
) {
    for mut transform in &mut q_camera {
        *transform = Transform::from_translation(camera_position(roster.ships))
            .looking_at(CAMERA_TARGET, Vec3::Y);
    }
}

/// Marks the seed readout.
#[derive(Component)]
struct SeedReadout;

fn spawn_readout(commands: &mut Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                SeedReadout,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Written every frame rather than on `Roster` change: the readout is spawned
/// by a command in the same run as the roster's first change, so a
/// change-gated write lands before the text exists and never runs again.
fn update_readout(
    roster: Res<Roster>,
    styles: Res<GameStyles>,
    mut q_readout: Query<&mut Text, With<SeedReadout>>,
) {
    // The style is NAMED, because a shot of a row is only evidence about a look
    // if the frame says which look it is.
    let dress = if roster.clad {
        format!(
            "clad: {}",
            style_at(&styles, roster.style).unwrap_or("bare")
        )
    } else {
        "bare".to_string()
    };
    let line = format!(
        "WFC ships - seeds {}..{} - {dress} - [R] re-roll  [C] cladding  [L] look",
        roster.seed,
        roster.seed + roster.ships as u64 - 1,
    );
    for mut text in &mut q_readout {
        if text.as_str() != line {
            **text = line.clone();
        }
    }
}

/// Seconds a step may sit before the run aborts naming it (llvmpipe headroom).
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 30.0;

/// Pose the harness camera on the row that is actually standing.
#[cfg(feature = "debug")]
fn frame_row(world: &mut World) {
    let ships = world.resource::<Roster>().ships;
    pose_camera(world, camera_position(ships), CAMERA_TARGET);
}

/// The driven walk: collapse a row, shoot it, strip the skin, shoot that.
#[cfg(feature = "debug")]
fn wfc_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the first row")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("frame the row")
        .on_enter(|world: &mut World| frame_row(world))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the row")
        .on_enter(|world: &mut World| shoot(world, "wfc-ships-row.png"))
        .until(shot_written("wfc-ships-row.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // Strip the cladding off the SAME seeds. This is the load path again,
        // so it also proves a collapsed row survives being torn down and
        // rebuilt in place - and it is the frame that shows what the skin is
        // actually doing, since both shots are the same ships.
        .step("strip the cladding")
        .on_enter(|world: &mut World| {
            let assets = world.resource::<GameAssets>().clone();
            let sections = world.resource::<GameSections>().clone();
            let styles = world.resource::<GameStyles>().clone();
            let next = Roster {
                clad: false,
                ..*world.resource::<Roster>()
            };
            *world.resource_mut::<Roster>() = next;
            let style = style_at(&styles, next.style);
            world.trigger(LoadScenario(wfc_row(&assets, &sections, next, style)));
        })
        .until(and(scenario_camera_present(), frames(SETTLE_FRAMES)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the bare hulls")
        .on_enter(|world: &mut World| {
            frame_row(world);
            shoot(world, "wfc-ships-bare.png");
        })
        .until(shot_written("wfc-ships-bare.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
