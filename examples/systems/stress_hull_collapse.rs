//! stress_hull_collapse: one siege lance slug into a capital block hull, and
//! the whole collapse it starts.
//!
//! The load this range exists for is not the shot. It is the AFTERMATH: a
//! corridor of [`CORRIDOR_CELLS_PER_LAYER`] cells per layer leaves the hull in
//! one command flush, every one of them a wreck piece born kinematic inside the
//! body it came off, and every carve along the way throws its chips. Those
//! populations are created on ONE frame and then all become physical together,
//! which is the frame cost `tasks/20260904-155338` attributed to avian rather
//! than to nova.
//!
//! FIVE named claims - three asserted, two recorded:
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: one siege slug opens exactly its rake corridor` | every cell inside the authored rake radius is charged, exactly once, and nothing outside it |
//! | 2 | `outcome: every corridor cell left the hull` | the corridor is DESTROYED, so the section count falls by exactly its size |
//! | 3 | `outcome: every wreck piece went physical` | the collapse ran to completion - nothing is still waiting on its grace when the window closes |
//! | 4 | `outcome: the collapse frame cost is recorded` | RECORD: the worst frame of the collapse window and the fixed steps it paid for |
//! | 5 | `outcome: the debris the collapse threw is recorded` | RECORD: peak shards, wreck pieces, pieces pending activation, entities |
//!
//! Claim 3 is the one a debris BUDGET has to keep. Spreading activation over
//! frames is safe in the direction the grace exists for - a piece stays
//! kinematic longer, never shorter - but a budget that dropped activations
//! would leave ghost wreckage nothing can fly into, and this is what says so.
//!
//! Claims 4 and 5 assert NOTHING. Milliseconds are a statement about the host:
//! this range's numbers are read against a named reference in a task's
//! before/after, never against a threshold. Same reading as `bug_sandbox_soak`
//! and as the probe's own `fps_within_baseline`. Do not turn them into asserts.
//!
//! # The content here is PINNED
//!
//! Every cell, the block's size, the stand-off and the weapon are constants in
//! this file. Nothing is read from a campaign scene, because a reference that
//! moves when a story beat is retimed is not a reference. `SIEGE_RAKE_RADIUS`
//! and `CELL_HEALTH` are DELIVERY GUARDS against the catalog rather than
//! authored numbers - the range reads the mounted section and fails loudly if
//! what content authored is no longer what this arithmetic was written for.
//!
//! `examples/playable/first_shift_08_attack_salvo.rs` stays the hand-run cross
//! check against real mainline content: a skinned carrier, a scripted salvo and
//! a camera. It is not a range and it makes no claim; when this one moves, that
//! is the scene to fly and look at.
//!
//! # No frame-time capture
//!
//! `NovaProbePlugin::default().without_frametime()`, for the reason
//! `system_railgun_lance` gives: a one-shot walk that exits seconds after the
//! collapse settles can never fill the 900-frame baseline window, and an armed
//! capture would report `armed and silent` on every run. The collapse frame
//! cost is the range's own reading, recorded on claim 4, and the profiled pass
//! (`probe run stress_hull_collapse`) is what attributes it by system name.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//!
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example stress_hull_collapse --features debug
//! # look for: `hull_collapse: every collapse invariant held`,
//! #           `autopilot: cycle complete, no panic`
//!
//! cargo run --features debug probe run stress_hull_collapse
//! ```

use std::collections::{BTreeMap, BTreeSet};

use avian3d::prelude::*;
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "stress_hull_collapse")]
#[command(version = "1.0.0")]
#[command(about = "Stress range: one siege lance slug into a capital block hull, and the collapse it starts. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario id the range loads under.
const RANGE_ID: &str = "hull_collapse_range";

/// The lance's scenario-object id, so the probe and the rig cannot drift apart.
const LANCE_ID: &str = "lance";

// --- the hull, at stress scale ---------------------------------------------
//
// ABSURD SCALE, and deliberately so: these numbers must NEVER reflect real
// content. No shipped hull is a solid nine-by-nine prism, and no campaign beat
// destroys a corridor this size in one flush. Read them as the load the
// collapse pipeline is proven under, never as a design figure. The largest
// thing the base game spawns is the industrial carrier at 2079 build-grid
// cells; this block is a bit under half that and loses more of itself in one
// shot than that carrier did in the run this range was written from.

/// Cells either side of the bore, in each of the two lateral axes. Nine across,
/// so the corridor the authored rake cuts is strictly INSIDE the block and what
/// bounds it is the rake rather than the edge of the target.
const HULL_HALF_WIDE: i32 = 4;

/// Layers of hull along the bore.
const HULL_DEPTH: i32 = 16;

/// The build-grid pitch, in engine world units (one cell is one unit is 10 m).
/// The lattice every shipped hull is built on, so the corridor arithmetic below
/// is the game's own geometry and not a rig's.
const HULL_CELL: f32 = 1.0;

/// The plate every cell is built from, and the one computer that makes the
/// block a ship the rest of the stack recognizes.
const HULL_PLATE: &str = REINFORCED_HULL_SECTION_ID;
const HULL_BRIDGE: &str = BASIC_CONTROLLER_SECTION_ID;

/// Where the bridge sits, in cells: the far corner, well outside the corridor.
/// A computer in the bore would be destroyed with everything else and take the
/// hull's authority with it, and a ship the stack has stopped recognizing is a
/// different subject.
const BRIDGE_CELL: IVec3 = IVec3::new(HULL_HALF_WIDE, HULL_HALF_WIDE, -(HULL_DEPTH - 1));

/// The gap between the lance's muzzle and the block's entry face, in engine
/// world units.
///
/// Short on purpose, and [`the_block_is_inside_one_sweep`] is why: a slug at
/// the authored 15 km/s crosses about 23 units in one fixed step, so a block
/// this deep is raked in ONE sweep. Two sweeps would rake it just as
/// completely, but the cells between the two steps' reaches are picked up on
/// the second pass, and a corridor whose size depends on which step reached
/// which layer is an arithmetic nobody can read.
const MUZZLE_GAP: f32 = 4.0;

/// The block's own health per cell, and the authored rake radius, in engine
/// world units.
///
/// DELIVERY GUARDS, not authored numbers: the range reads both off the mounted
/// content and fails naming them when the catalog moves. Every count below is
/// written against these two values.
const CELL_HEALTH: f32 = 200.0;
const SIEGE_RAKE_RADIUS: f32 = 3.0;

/// How many cells of one layer lie inside [`SIEGE_RAKE_RADIUS`].
///
/// The seven-by-seven square about the bore minus its four corners: on the unit
/// lattice a cell at ring 3 reaches 2.5 from the bore, a 3-and-2 diagonal
/// 2.915, and a 3-and-3 diagonal 3.536 - which is the first offset outside the
/// radius. Ring 4 starts at 3.5 and is never touched, which is what makes the
/// block wider than the corridor.
const CORRIDOR_CELLS_PER_LAYER: usize = 45;

/// The whole corridor: every layer of it, and what claims 1 and 2 count.
const CORRIDOR_CELLS: usize = CORRIDOR_CELLS_PER_LAYER * HULL_DEPTH as usize;

/// Slack on a distance reading, in engine world units. The lattice arithmetic
/// is exact; this only absorbs f32 accumulation through the transform stack.
const OFFSET_EPSILON: f32 = 1.0e-3;

// --- the walk ---------------------------------------------------------------

/// How long the collapse window stays open, measured from the frame the shot
/// left, in seconds.
///
/// Past the slug's own 1.2 s lifetime, past the grace every piece drifts
/// through and past the shard lifetime, so claim 3 reads a settled field and
/// claims 4 and 5 cover the whole aftermath rather than its first frame.
/// Generous, because deferring activation over frames is exactly the change
/// this range is the instrument for.
const SETTLE_SECS: f32 = 8.0;

/// How many `Playing` frames the whole walk gets before the range calls itself
/// stalled and PANICS with what it was still waiting on.
///
/// A range that hangs proves nothing and costs a CI slot the whole deadline.
/// Generous: a thousand-odd collider-bearing sections spawn slowly under
/// llvmpipe, and the charge, the flight and the settle are all measured in
/// frames rather than asserted.
const STALL_FRAMES: u32 = 12_000;

/// How often the walk says where it is, in frames.
const STATUS_EVERY: u32 = 120;

/// Marks one block cell with its lattice address, so a bite can be read as a
/// place rather than as a count.
#[derive(Component, Clone, Copy, Debug)]
struct HullCell(IVec3);

/// One cell the range watched the slug pay for, and WHERE the corridor met it.
#[derive(Clone, Copy, Debug)]
struct CorridorBite {
    section: Entity,
    cell: IVec3,
    offset: f32,
}

/// The worst frame of the collapse window, and what it paid for.
#[derive(Clone, Copy, Debug, Default)]
struct WorstFrame {
    ms: f32,
    steps: u32,
    /// Seconds into the window it landed.
    at: f32,
}

/// What the range has watched happen.
#[derive(Resource, Default)]
struct CollapseProbe {
    /// The lance section, once the scenario has spawned it.
    lance: Option<Entity>,
    /// The ship the lance is bolted to.
    shooter: Option<Entity>,
    /// The block, once its cells are addressed.
    block: Option<Entity>,
    /// Frames spent in `Playing`.
    frames: u32,
    /// Set once every cell carries its lattice address.
    addressed: bool,
    /// Live sections, read the frame before the slug left.
    sections_before: usize,
    /// Set on the tick the commit is tapped.
    committed: bool,
    /// How many slugs this lance has fired.
    shots: u32,
    /// Every cell the slug charged, in the order it paid for them.
    bites: Vec<CorridorBite>,
    /// When the collapse window opened, in app seconds: the frame the shot
    /// left. The flush the corridor costs lands on that frame, and a window
    /// that opened when the slug finally expired 1.2 s later measured only the
    /// aftermath.
    window_opened: Option<f32>,
    /// The frame cost of the collapse window.
    worst: WorstFrame,
    /// Frames the window covered. Over a fixed span of APP seconds this is the
    /// window's aggregate rate, which is the reading a tail cannot give.
    window_frames: u32,
    /// How many of those overran the fixed timestep and therefore handed their
    /// overrun to the next frame as extra steps.
    slow_frames: u32,
    /// Fixed steps since the last rendered frame, for [`WorstFrame::steps`].
    steps_this_frame: u32,
    /// Peak populations over the window.
    peak_shards: usize,
    peak_pieces: usize,
    peak_pending: usize,
    peak_entities: u32,
    /// Set once every claim has been read and reported.
    verified: bool,
    exit_delay: u32,
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();
    // No frame-time capture - see the module docs.
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.run()
}

fn range_plugin(app: &mut App) {
    app.init_resource::<CollapseProbe>();
    app.add_observer(count_shots);
    app.add_observer(record_corridor_bites);
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_range);
    // The safety is DERIVED every frame from the held combat stance, so a range
    // that pokes `WeaponsHot` has it stomped back before the lance reads it.
    // Hold the real button instead, after the frame's input has been collected.
    app.add_systems(
        PreUpdate,
        hold_combat_stance
            .after(bevy::input::InputSystems)
            .run_if(in_state(GameStates::Playing)),
    );
    app.add_systems(FixedUpdate, count_fixed_steps);
    app.add_systems(
        Update,
        (sample_the_collapse, drive_range)
            .chain()
            .run_if(in_state(GameStates::Playing)),
    );
}

/// Hold the combat stance for the frame, which is what raises the weapons.
fn hold_combat_stance(mut mouse: ResMut<ButtonInput<MouseButton>>) {
    mouse.press(MouseButton::Right);
}

fn count_fixed_steps(mut probe: ResMut<CollapseProbe>) {
    probe.steps_this_frame += 1;
}

fn load_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(collapse_rig(&game_assets, &sections)));
}

/// Every cell of the block, in a stable order.
///
/// Purely a function of the two size constants, so a rerun builds the same
/// hull. The bridge cell is not here: it REPLACES the plate at
/// [`BRIDGE_CELL`], which is why the total is the lattice's own size.
fn block_cells() -> Vec<IVec3> {
    let mut cells = Vec::new();
    for layer in 0..HULL_DEPTH {
        for y in -HULL_HALF_WIDE..=HULL_HALF_WIDE {
            for x in -HULL_HALF_WIDE..=HULL_HALF_WIDE {
                cells.push(IVec3::new(x, y, -layer));
            }
        }
    }
    cells
}

/// How far a lattice cell's NEAREST point lies from the bore, in engine world
/// units.
///
/// The same distance the swept corridor tests against (`corridor_contact` in
/// `nova_gameplay::rounds`), so a cell inside the rake radius is a cell the
/// rake owes a bite. Pure, so the corridor can be counted without a running
/// app.
fn cell_offset(cell: IVec3) -> f32 {
    let reach = |n: i32| (n.abs() as f32 * HULL_CELL - HULL_CELL * 0.5).max(0.0);
    reach(cell.x).hypot(reach(cell.y))
}

/// How many cells of one layer the lattice puts inside `radius`.
///
/// [`CORRIDOR_CELLS_PER_LAYER`] is what every count here is written against,
/// and this is what proves the constant still describes the block: the same
/// nearest-point rule, counted over the block's own cross section.
fn corridor_cells_per_layer(radius: f32) -> usize {
    (-HULL_HALF_WIDE..=HULL_HALF_WIDE)
        .flat_map(|y| (-HULL_HALF_WIDE..=HULL_HALF_WIDE).map(move |x| IVec3::new(x, y, 0)))
        .filter(|cell| cell_offset(*cell) <= radius)
        .count()
}

/// The rig scenario: the shooter at the origin bore down -Z, and the block
/// downrange on that exact line.
///
/// The shooter is PLAYER-controlled with an EMPTY input mapping, so nothing it
/// carries can move it while the shot is set up and the range writes
/// [`RailgunSectionInput`] straight onto the section rather than synthesizing a
/// binding.
///
/// Spine layout, in cells: the lance is THREE cells long and centred on its own
/// origin, so at -1 it fills -2, -1 and 0 and the computer behind it starts at
/// +1. Cell -3 is empty, which the exit-clearance rule requires - a lance
/// cannot traverse off its bore.
fn collapse_rig(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let at = |id: String, kind: &str, position: Vec3| SpaceshipSectionConfig {
        id,
        position,
        rotation: Quat::IDENTITY,
        source: SectionSource::Inline(section(kind)),
        modifications: vec![],
    };

    let shooter = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::new(),
            speed_cap: None,
        }),
        hull: ShipSource::Inline(ShipHull {
            sections: vec![
                at(
                    LANCE_ID.to_string(),
                    SIEGE_RAILGUN_LANCE_SECTION_ID,
                    Vec3::new(0.0, 0.0, -1.0),
                ),
                at(
                    "controller".to_string(),
                    HULL_BRIDGE,
                    Vec3::new(0.0, 0.0, 1.0),
                ),
                at(
                    "thruster".to_string(),
                    BASIC_THRUSTER_SECTION_ID,
                    Vec3::new(0.0, 0.0, 2.0),
                ),
            ],
            ..default()
        }),
        ..default()
    };

    let block = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::None,
        hull: ShipSource::Inline(ShipHull {
            sections: block_cells()
                .into_iter()
                .map(|cell| {
                    let plate = if cell == BRIDGE_CELL {
                        HULL_BRIDGE
                    } else {
                        HULL_PLATE
                    };
                    at(
                        format!("cell_{}_{}_{}", cell.x, cell.y, cell.z),
                        plate,
                        cell.as_vec3() * HULL_CELL,
                    )
                })
                .collect(),
            // NO derived skin. The subject is what a collapse CREATES, and a
            // plate per exposed face would put a second population's spawn
            // cost and a second set of colliders in the same number. The
            // mainline cross-check in `examples/playable/` is the skinned one.
            ..default()
        }),
        ..default()
    };

    // The lance's muzzle sits on the brake face, half the lance's own length
    // ahead of its cell, so the entry face stands [`MUZZLE_GAP`] past it.
    let muzzle_z = -1.0 - 1.5;
    let entry_face_z = muzzle_z - MUZZLE_GAP;
    let block_origin_z = entry_face_z - HULL_CELL * 0.5;

    ScenarioConfig {
        description: "A siege lance, and a capital block hull to collapse.".to_string(),
        hidden: true,
        // The rig lights itself: the engine spawns no light, so a scenario that
        // authors none renders black.
        events: nova_probe::fixtures::spawn_on_start(
            [
                vec![
                    ScenarioObjectConfig {
                        base: BaseScenarioObjectConfig {
                            id: "shooter".to_string(),
                            name: "Siege Rig".to_string(),
                            position: Meters3::ZERO,
                            rotation: Quat::IDENTITY,
                        },
                        kind: ScenarioObjectKind::Spaceship(shooter),
                    },
                    ScenarioObjectConfig {
                        base: BaseScenarioObjectConfig {
                            id: "block".to_string(),
                            name: "Capital Block".to_string(),
                            position: Meters3::from_engine(Vec3::Z * block_origin_z),
                            rotation: Quat::IDENTITY,
                        },
                        kind: ScenarioObjectKind::Spaceship(block),
                    },
                ],
                ThreePointRig::around("rig", Meters3::ZERO, 30.0).objects(),
            ]
            .concat(),
        ),
        ..ScenarioConfig::new(
            RANGE_ID.to_string(),
            "Stress: Hull Collapse".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Count the shots this lance actually fired, off the weapon's own report.
fn count_shots(fired: On<RailgunFired>, mut probe: ResMut<CollapseProbe>) {
    if probe.lance == Some(fired.entity) {
        probe.shots += 1;
    }
}

/// Record every cell the slug charged.
///
/// The ROUND's own report, not the block's health: a cell the rake empties is
/// destroyed and leaves the world, which is what makes the corridor a visible
/// hole and what makes surviving health useless as a reading.
///
/// PIERCE only, and that filter is load-bearing rather than tidy. The collapse
/// this range exists for throws several hundred wreck pieces through the hull
/// they came off, and every one of those contacts is a `SurfaceImpact` on a
/// cell as well - Kinetic, from the ram model in `integrity::core`. Unfiltered,
/// the corridor reading was six times its own size and counted the block's
/// outer skin. The slug is the only Pierce weapon in the rig.
fn record_corridor_bites(
    impact: On<SurfaceImpact>,
    q_cell: Query<&HullCell>,
    mut probe: ResMut<CollapseProbe>,
) {
    if impact.kind != DamageType::Pierce {
        return;
    }
    let Ok(&HullCell(cell)) = q_cell.get(impact.entity) else {
        return;
    };
    probe.bites.push(CorridorBite {
        section: impact.entity,
        cell,
        offset: cell_offset(cell),
    });
}

/// How many entities of one kind are alive right now.
fn live_count<M: Component>(world: &mut World) -> usize {
    world
        .try_query_filtered::<(), With<M>>()
        .map_or(0, |mut query| query.iter(world).count())
}

/// Address every block cell from its own local transform.
///
/// Read off the placed section rather than parsed out of its id: the authored
/// position IS the lattice address, and a range that trusted a string would
/// keep agreeing with itself after the layout moved.
fn address_the_block(world: &mut World) -> Option<Entity> {
    let mut query =
        world.try_query_filtered::<(Entity, &ChildOf, &Transform), With<SectionMarker>>()?;
    let found: Vec<(Entity, Entity, IVec3)> = query
        .iter(world)
        .map(|(section, parent, local)| {
            (
                section,
                parent.0,
                (local.translation / HULL_CELL).round().as_ivec3(),
            )
        })
        .collect();

    // The block is the root carrying the most sections - the shooter has three.
    let mut per_root: BTreeMap<Entity, usize> = BTreeMap::new();
    for (_, root, _) in &found {
        *per_root.entry(*root).or_default() += 1;
    }
    let (block, held) = per_root.into_iter().max_by_key(|(_, held)| *held)?;
    if held < block_cells().len() {
        return None;
    }

    for (section, root, cell) in found {
        if root == block {
            world.entity_mut(section).insert(HullCell(cell));
        }
    }
    Some(block)
}

/// The walk: find the lance, address the block, tap the commit, and then watch
/// the collapse the slug started run itself out.
fn drive_range(world: &mut World) {
    world.resource_mut::<CollapseProbe>().frames += 1;

    if !world.resource::<CollapseProbe>().verified {
        let frames = world.resource::<CollapseProbe>().frames;
        if frames.is_multiple_of(STATUS_EVERY) {
            let cells = live_count::<HullCell>(world);
            let shards = live_count::<CarveShardMarker>(world);
            let pieces = live_count::<DetachedPieceMarker>(world);
            let pending = live_count::<ChunkGrace>(world);
            let probe = world.resource::<CollapseProbe>();
            info!(
                "hull_collapse: frame {frames} - lance {:?}, addressed {}, committed {}, shots \
                 {}, bites {}, window {:?}, cells {cells}, shards {shards}, pieces \
                 {pieces}, pending {pending}",
                probe.lance,
                probe.addressed,
                probe.committed,
                probe.shots,
                probe.bites.len(),
                probe.window_opened,
            );
        }
        assert!(
            frames <= STALL_FRAMES,
            "hull_collapse: the walk stalled - it never reached a verdict inside {STALL_FRAMES} \
             frames"
        );
    }

    if world.resource::<CollapseProbe>().verified {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            let exit = {
                let mut probe = world.resource_mut::<CollapseProbe>();
                probe.exit_delay += 1;
                probe.exit_delay >= 30
            };
            if exit {
                world.write_message(AppExit::Success);
            }
        }
        return;
    }

    if world.resource::<CollapseProbe>().lance.is_none() {
        let found = world
            .try_query_filtered::<(Entity, &ChildOf), With<RailgunSectionMarker>>()
            .and_then(|mut query| query.iter(world).next().map(|(a, b)| (a, b.0)));
        let Some((lance, shooter)) = found else {
            return;
        };
        let mut probe = world.resource_mut::<CollapseProbe>();
        probe.lance = Some(lance);
        probe.shooter = Some(shooter);
        return;
    }

    if !world.resource::<CollapseProbe>().addressed {
        let Some(block) = address_the_block(world) else {
            return;
        };
        let cells = live_count::<HullCell>(world);
        assert_eq!(
            cells,
            block_cells().len(),
            "hull_collapse: the block stood up as {cells} cells, not the {} this range pins",
            block_cells().len()
        );
        let mut probe = world.resource_mut::<CollapseProbe>();
        probe.block = Some(block);
        probe.sections_before = cells;
        probe.addressed = true;
        return;
    }

    let (lance, shooter) = {
        let probe = world.resource::<CollapseProbe>();
        (probe.lance.unwrap(), probe.shooter.unwrap())
    };

    // Nothing to tap until the stance has actually gone hot: a cold ship
    // refuses the commit, which is a rule this range is not testing.
    if !world.get::<WeaponsHot>(shooter).is_some_and(|hot| hot.0) {
        return;
    }

    // The tap: trigger down until the gun has taken the commit, then released.
    // Held rather than pulsed for one frame, because the walk runs on the render
    // clock and the gun on the fixed one.
    if !world.resource::<CollapseProbe>().committed {
        if let Some(mut input) = world.get_mut::<RailgunSectionInput>(lance) {
            input.0 = true;
        }
        if matches!(
            world.get::<RailgunCharge>(lance),
            Some(RailgunCharge::Charging { .. })
        ) {
            the_block_is_inside_one_sweep(world, lance);
            world.resource_mut::<CollapseProbe>().committed = true;
        }
        return;
    }
    if let Some(mut input) = world.get_mut::<RailgunSectionInput>(lance) {
        input.0 = false;
    }

    // Nothing to watch until the shot has left.
    if world.resource::<CollapseProbe>().shots == 0 {
        return;
    }

    // Nothing is read until the slug has spent itself and left the world: what
    // it charged is only complete once it is gone.
    let flying = world
        .try_query_filtered::<(), With<RailgunSlugProjectileMarker>>()
        .is_some_and(|mut query| query.iter(world).next().is_some());
    if flying {
        return;
    }

    let Some(opened) = world.resource::<CollapseProbe>().window_opened else {
        return;
    };
    if world.resource::<Time>().elapsed_secs() - opened < SETTLE_SECS {
        return;
    }

    verify(world);
}

/// The DELIVERY GUARD for every count in this file: one fixed step of flight at
/// the authored slug speed has to cover the whole block, plus the rake radius
/// its trailing sphere lags the tip by.
///
/// Read at the commit, off the mounted section and the live timestep, because
/// both are content this range does not own. A corridor raked over two sweeps
/// is still complete, but which layer each step reached stops being readable -
/// see [`MUZZLE_GAP`].
fn the_block_is_inside_one_sweep(world: &mut World, lance: Entity) {
    let config = world
        .get::<RailgunSectionConfigHelper>(lance)
        .expect("the lance carries its authored config");
    let speed = config.slug_speed.to_engine();
    let radius = config
        .rake()
        .map(Meters::to_engine)
        .expect("the siege lance authors a rake radius");
    let damage = config.slug_damage;
    let power = config.slug_power;
    let step = world.resource::<Time<Fixed>>().timestep().as_secs_f32();

    assert!(
        (radius - SIEGE_RAKE_RADIUS).abs() <= OFFSET_EPSILON,
        "hull_collapse: the siege lance now rakes {} m, not the {} m every count in this range is \
         written against",
        Meters::from_engine(radius).get(),
        Meters::from_engine(SIEGE_RAKE_RADIUS).get()
    );
    assert!(
        damage >= CELL_HEALTH,
        "hull_collapse: a slug dealing {damage} no longer clears a {CELL_HEALTH} hp cell in one \
         bite, so the corridor is damaged rather than destroyed"
    );
    // Priced at the pierce ceiling, which a 15 km/s slug is far past: a cell
    // costs its max health divided by the closing-speed multiplier.
    let cheapest = CELL_HEALTH / 3.0;
    assert!(
        power >= CORRIDOR_CELLS as f32 * cheapest,
        "hull_collapse: {power} of pierce budget cannot pay for {CORRIDOR_CELLS} cells at \
         {cheapest} each, so what stops the rake is the budget and not the target"
    );

    let per_layer = corridor_cells_per_layer(radius);
    assert_eq!(
        per_layer, CORRIDOR_CELLS_PER_LAYER,
        "hull_collapse: a {radius} unit rake covers {per_layer} of this block's cells per layer, \
         not the {CORRIDOR_CELLS_PER_LAYER} every count here is written against"
    );

    let deepest = MUZZLE_GAP + (HULL_DEPTH - 1) as f32 * HULL_CELL;
    let reach = speed * step - radius;
    assert!(
        deepest <= reach,
        "hull_collapse: the block's far layer stands {deepest} units downrange and one sweep \
         reaches {reach} ({speed} u/s over a {step} s step, less a {radius} unit rake) - the \
         corridor would be raked over two steps"
    );
}

/// Follow the collapse window every frame: its worst frame, and the peak of
/// every population it created.
fn sample_the_collapse(world: &mut World) {
    // Opened HERE rather than in the walk, so the frame that fired is inside
    // the window: `Time::delta` reports the previous frame, and the flush the
    // corridor costs is charged to the frame the shot left.
    if world.resource::<CollapseProbe>().window_opened.is_none() {
        if world.resource::<CollapseProbe>().shots == 0 {
            world.resource_mut::<CollapseProbe>().steps_this_frame = 0;
            return;
        }
        let now = world.resource::<Time>().elapsed_secs();
        world.resource_mut::<CollapseProbe>().window_opened = Some(now);
    }

    let ms = world.resource::<Time>().delta_secs() * 1000.0;
    let at = {
        let probe = world.resource::<CollapseProbe>();
        world.resource::<Time>().elapsed_secs() - probe.window_opened.unwrap_or_default()
    };
    let shards = live_count::<CarveShardMarker>(world);
    let pieces = live_count::<DetachedPieceMarker>(world);
    let pending = live_count::<ChunkGrace>(world);
    let entities: u32 = world
        .archetypes()
        .iter()
        .map(bevy::ecs::archetype::Archetype::len)
        .sum();

    let mut probe = world.resource_mut::<CollapseProbe>();
    let steps = std::mem::take(&mut probe.steps_this_frame);
    probe.window_frames += 1;
    if f64::from(ms) >= FIXED_TIMESTEP_MS {
        probe.slow_frames += 1;
    }
    if ms > probe.worst.ms {
        probe.worst = WorstFrame { ms, steps, at };
    }
    probe.peak_shards = probe.peak_shards.max(shards);
    probe.peak_pieces = probe.peak_pieces.max(pieces);
    probe.peak_pending = probe.peak_pending.max(pending);
    probe.peak_entities = probe.peak_entities.max(entities);
}

/// A smoothed avian diagnostic by path, or zero when it has no reading yet.
fn avian_reading(world: &World, path: &str) -> f64 {
    world
        .resource::<bevy::diagnostic::DiagnosticsStore>()
        .iter()
        .find(|diagnostic| diagnostic.path().as_str() == path)
        .and_then(bevy::diagnostic::Diagnostic::smoothed)
        .unwrap_or_default()
}

/// The shipped fixed timestep, in milliseconds. What a recorded frame cost is
/// READ against - a frame over it hands its overrun to the next as extra steps,
/// which is the amplifier the collapse tail is made of. A reference for a
/// reviewer, never a gate.
const FIXED_TIMESTEP_MS: f64 = 1000.0 / 64.0;

fn verify(world: &mut World) {
    // --- claim 1: the corridor the rake opened ---

    let bites = world.resource::<CollapseProbe>().bites.clone();
    let charged: BTreeSet<Entity> = bites.iter().map(|bite| bite.section).collect();
    let twice = bites.len() - charged.len();
    let outside: Vec<IVec3> = bites
        .iter()
        .filter(|bite| bite.offset > SIEGE_RAKE_RADIUS + OFFSET_EPSILON)
        .map(|bite| bite.cell)
        .collect();
    let widest = bites.iter().map(|bite| bite.offset).fold(0.0f32, f32::max);
    let layers: BTreeSet<i32> = bites.iter().map(|bite| bite.cell.z).collect();

    assert!(
        charged.len() == CORRIDOR_CELLS && twice == 0 && outside.is_empty(),
        "hull_collapse: one siege slug did not open exactly its rake corridor: {} of \
         {CORRIDOR_CELLS} cells charged, {twice} charged twice, {} outside the {SIEGE_RAKE_RADIUS} \
         unit radius ({outside:?}), across {} of {HULL_DEPTH} layers",
        charged.len(),
        outside.len(),
        layers.len()
    );
    nova_probe::probe_marker(
        world,
        "outcome: one siege slug opens exactly its rake corridor",
        serde_json::json!({
            "cells": charged.len(),
            "expected": CORRIDOR_CELLS,
            "per_layer": CORRIDOR_CELLS_PER_LAYER,
            "layers": layers.len(),
            "charged_twice": twice,
            "outside_the_radius": outside.len(),
            "widest_offset": widest,
            "rake_radius": SIEGE_RAKE_RADIUS,
        }),
    );

    // --- claim 2: and the corridor LEFT ---

    let before = world.resource::<CollapseProbe>().sections_before;
    let after = live_count::<HullCell>(world);
    let destroyed = before.saturating_sub(after);
    assert_eq!(
        destroyed, CORRIDOR_CELLS,
        "hull_collapse: {destroyed} sections left the field, not the {CORRIDOR_CELLS} cells of \
         corridor the slug charged ({before} before, {after} after) - the collapse destroyed \
         something other than what was shot"
    );
    nova_probe::probe_marker(
        world,
        "outcome: every corridor cell left the hull",
        serde_json::json!({
            "sections_before": before,
            "sections_after": after,
            "destroyed": destroyed,
            "expected": CORRIDOR_CELLS,
        }),
    );

    // --- claim 3: the collapse ran to completion ---

    let pending = live_count::<ChunkGrace>(world);
    let pieces = live_count::<DetachedPieceMarker>(world);
    let kinematic = world
        .try_query_filtered::<&RigidBody, With<DetachedPieceMarker>>()
        .map_or(0, |mut query| {
            query
                .iter(world)
                .filter(|body| **body != RigidBody::Dynamic)
                .count()
        });
    assert!(
        pending == 0 && kinematic == 0,
        "hull_collapse: {SETTLE_SECS} s after the collapse {pending} pieces are still waiting on \
         their grace and {kinematic} of {pieces} are still kinematic - a deferred activation was \
         dropped, and ghost wreckage is wreckage a ship can fly through"
    );
    nova_probe::probe_marker(
        world,
        "outcome: every wreck piece went physical",
        serde_json::json!({
            "pieces": pieces,
            "pending_activation": pending,
            "still_kinematic": kinematic,
            "settle_secs": SETTLE_SECS,
        }),
    );

    // --- claims 4 and 5: RECORDED, and asserted nowhere ---

    let step_ms = avian_reading(world, "avian/total_step_time");
    let contacts_ms = avian_reading(world, "avian/collision/update_contacts");
    let pairs = avian_reading(world, "avian/collision/contact_count");
    let constraints = avian_reading(world, "avian/solver/contact_constraint_count");
    let probe_readings = {
        let probe = world.resource::<CollapseProbe>();
        (
            probe.worst,
            probe.window_frames,
            probe.slow_frames,
            probe.peak_shards,
            probe.peak_pieces,
            probe.peak_pending,
            probe.peak_entities,
        )
    };
    let (worst, window_frames, slow_frames, peak_shards, peak_pieces, peak_pending, peak_entities) =
        probe_readings;

    nova_probe::probe_marker(
        world,
        "outcome: the collapse frame cost is recorded",
        serde_json::json!({
            "worst_frame_ms": worst.ms,
            "worst_frame_fixed_steps": worst.steps,
            "worst_frame_at_secs": worst.at,
            "window_frames": window_frames,
            "frames_over_timestep": slow_frames,
            "window_secs": SETTLE_SECS,
            "timestep_ms": FIXED_TIMESTEP_MS,
            "step_ms": step_ms,
            "update_contacts_ms": contacts_ms,
            "contact_pairs": pairs,
            "contact_constraints": constraints,
        }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: the debris the collapse threw is recorded",
        serde_json::json!({
            "corridor_cells": CORRIDOR_CELLS,
            "peak_shards": peak_shards,
            "peak_wreck_pieces": peak_pieces,
            "peak_pending_activation": peak_pending,
            "peak_entities": peak_entities,
        }),
    );

    world.resource_mut::<CollapseProbe>().verified = true;
    info!(
        "hull_collapse: {CORRIDOR_CELLS} corridor cells destroyed, {peak_pieces} wreck pieces and \
         {peak_shards} shards at peak, {peak_entities} entities"
    );
    info!(
        "hull_collapse: worst collapse frame {:.1} ms over {} fixed steps ({:.1} s into the \
         window) against a {FIXED_TIMESTEP_MS:.2} ms timestep; {slow_frames} of {window_frames} \
         window frames overran it; step {step_ms:.2} ms, narrow phase {contacts_ms:.2} ms over \
         {pairs:.0} pairs holding {constraints:.0} constraints",
        worst.ms, worst.steps, worst.at
    );
    if worst.ms as f64 >= FIXED_TIMESTEP_MS {
        warn!(
            "hull_collapse: the collapse overran the timestep ({:.1} ms over {} steps). \
             Host-noisy by nature - read it against a named reference on a quiet box, never as a \
             verdict",
            worst.ms, worst.steps
        );
    }
    info!("hull_collapse: every collapse invariant held");
}
