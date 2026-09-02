//! screenshot_damage_levels: the same ship at all eight crack levels, side by side.
//!
//! THE GATE for the erosion epic (task 20260813-224826, phase 2). Damage is now
//! one number - the share of a body's own health that is gone - and every
//! damage EFFECT is a component turning that number into its own look. Whether
//! those looks are any good is not a question a test can answer, so this stands
//! them in a row and lets somebody decide.
//!
//! Eight identical clad ships, left to right, taking progressively worse
//! punishment. Damage is TWO readings and this drives both, because the effects
//! read different ones:
//!
//! - each ship's health is SET to the fraction its column stands for, which is
//!   what the whole-body effects grade off;
//! - and each ship past the first takes ONE real hit, through the same
//!   `apply_damage` a turret uses, landing on the same place on its hull with a
//!   bigger warhead each time. That is what takes the cladding off.
//!
//! What to judge, per effect:
//!
//! - PLATE LOSS, on the cladding. The plate the round lands on spends its own
//!   health and comes off, opening a hole onto the hull underneath. That is the
//!   ONLY thing that changes a ship's shape now: sections carved their drawn
//!   geometry for one release and it was removed for costing a two-second frame
//!   on a ship's first hit (Phase 4e). Does a hull that is cracked and missing
//!   plates read as BROKEN without the crater?
//! - SCORCH, on the section materials. Reddening, darkening, then a burnt
//!   endpoint. It has lost its allegiance split, so an enemy now shows this
//!   too - is that too much information, or the right amount?
//! - SPARKS, on the turret and thruster. Nothing on a ship loses geometry now,
//!   and these two never could: a turret whose silhouette has been chewed
//!   cannot convincingly still shoot. Does a sparking turret read as failing
//!   without looking broken?
//! - PLUME, on the thruster. Every drive in the row is held at full throttle,
//!   so the cone is drawn and the damaged ones can be compared against the
//!   clean one. Does a guttering plume read as a failing drive rather than as
//!   a drive that is shutting down?
//!
//! Which of those a section wears is AUTHORED, in its content
//! (`base.damage_effects`), so this row is also the check that the shipped
//! catalog authored what it meant to: the hull cracks, the turret sparks, the
//! thruster sparks and gutters.
//!
//! The thing NOT here is SHED - expendable pieces coming off a section that
//! keeps working. It needs art with separable pieces and the shipped turret has
//! none, so it is a content question rather than a code one.
//!
//! Hand-run:
//! ```text
//! cargo run --example screenshot_damage_levels --features debug
//! ```
//! - `L` steps the camera along the row, framing one level at a time.
//!
//! Harnessed, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: load the row, frame it, exit clean.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot `damage-levels.png` (the
//!   whole row) and one `damage-levels-<level>.png` per column.

// Only the harness pins the row, and only the harness runs under `debug`.
#[cfg(feature = "debug")]
use avian3d::prelude::RigidBody;
use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use nova_debug::prelude::capturing;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_damage_levels")]
#[command(version = "1.0.0")]
#[command(about = "One ship at all eight crack levels, side by side. Autopilot-only: a posed row with no controls of its own", long_about = None)]
struct Cli;

/// The levels the row stands at, left to right.
///
/// Not a linear sweep to 1.0: a body at 1.0 is dead and the finale takes it, so
/// the last column is the worst a LIVING ship gets. The low end is close
/// together on purpose - if two neighbouring columns are indistinguishable, the
/// effect has fewer usable steps than the number suggests, and that is exactly
/// what this row is for finding out.
const LEVELS: [f32; 9] = [
    0.0,
    1.0 / 7.0,
    2.0 / 7.0,
    3.0 / 7.0,
    4.0 / 7.0,
    5.0 / 7.0,
    6.0 / 7.0,
    0.99,
    0.0,
];

/// The hit each column takes, in health, on the same place on its hull.
///
/// Authored as damage rather than as a radius because damage is what a weapon
/// deals: `mark_radius` prices it into a sphere, and these levels buy roughly
/// nothing, then enough to take the plate it lands on, then enough to carry
/// through it into the hull. The spread is the point - a scrape, a plate gone,
/// a hole with the hull cracked open behind it.
#[cfg(feature = "debug")]
const HITS: [f32; 9] = [0.0, 21.0, 40.0, 86.0, 150.0, 290.0, 500.0, 977.0, 150.0];

/// Whether each column's ship wears cladding.
///
/// The last one does NOT, and it is there for the HULL's own surface. A clad
/// ship is covered in plates, and a plate is what a hit meets first, so the
/// hull material is behind the skin until the skin is shot off. The bare column
/// shows the same hit landing on the hull with nothing over it.
///
/// It stands at level 0.0 and takes a smaller hit than the column before it,
/// and both numbers are load-bearing. The level is zero so the column isolates
/// what the HIT alone does to a section nothing else has touched. The hit is
/// 150 because a hull section holds 200 - one that spends more than its health
/// bar is destroyed outright and there is nothing left to photograph.
const CLAD: [bool; 9] = [true, true, true, true, true, true, true, true, false];

/// How far apart the ships stand, in units. Wide enough that debris and sparks
/// from one column do not read as belonging to the next.
const COLUMN_PITCH: f32 = 6.0;

/// The scenario id each column's ship is spawned under.
fn column_id(index: usize) -> String {
    format!("level_{index}")
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(gallery_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_gallery);
    app.add_systems(Update, hold_the_throttle_open);
}

/// Set once the row is bolted down, which is what lets the drives run.
#[derive(Resource, Default)]
struct RowIsPinned;

/// Run every drive in the row at full throttle, forever.
///
/// PLUME grades the exhaust cone, and a cone is only drawn for a drive that is
/// firing - so a row of parked ships would show "the effect is broken" and "the
/// effect is not authored" as the same picture. These ships have no controller
/// (`SpaceshipController::None`), so nothing else writes this.
///
/// Gated on the row being PINNED, and that gate is not paranoia: thrust is
/// real. The first cut of this held the throttle open from the moment the ships
/// spawned and every one of them accelerated clean out of frame, so the whole row
/// captures came back black.
fn hold_the_throttle_open(
    pinned: Option<Res<RowIsPinned>>,
    mut q_thrusters: Query<&mut ThrusterSectionInput>,
) {
    if pinned.is_none() {
        return;
    }
    for mut input in &mut q_thrusters {
        if input.0 != 1.0 {
            input.0 = 1.0;
        }
    }
}

/// Bolt every ship in the row down, so it can be shot at and run its drives
/// without leaving the frame.
///
/// A gallery is a row of specimens, not a scenario: nothing here should move at
/// all. Static rather than locked axes because it also settles the shards a hit
/// throws off, which would otherwise nudge the row apart.
#[cfg(feature = "debug")]
fn prepare_row(world: &mut World) {
    hide_hud(world);
    pin_the_row(world);
}

#[cfg(feature = "debug")]
fn pin_the_row(world: &mut World) {
    let roots: Vec<Entity> = world
        .query_filtered::<Entity, With<SpaceshipRootMarker>>()
        .iter(world)
        .collect();
    let pinned = roots.len();
    for root in roots {
        world.entity_mut(root).insert(RigidBody::Static);
    }
    world.insert_resource(RowIsPinned);
    info!("damage levels: pinned {pinned} ships");
}

fn setup_gallery(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
) {
    commands.trigger(LoadScenario(gallery(&game_assets, &sections)));
}

/// Put every body in a column at that column's level, by SETTING health.
///
/// What the WHOLE-BODY effects read. Cracks and sparks grade off how far gone a
/// body is, which is a reading of its own health and nothing else, so this
/// touches health and pokes no effect directly: anything that fails to change
/// here is not actually reading the level.
///
/// Sections and plates are both walked, and for the same reason they are
/// separate pools in the first place - a plate is `HealthIsolated`, so damaging
/// the hull does not crack its cladding and this has to say what both are at.
#[cfg(feature = "debug")]
fn set_column_levels(world: &mut World) {
    let mut roots: Vec<(Entity, f32)> = Vec::new();
    {
        let mut q_roots = world.query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>();
        for (root, id) in q_roots.iter(world) {
            let Some(level) = LEVELS
                .iter()
                .enumerate()
                .find(|(index, _)| column_id(*index) == id.as_str())
                .map(|(_, level)| *level)
            else {
                continue;
            };
            roots.push((root, level));
        }
    }

    for (root, level) in roots {
        let mut bodies: Vec<Entity> = Vec::new();
        collect_descendants(world, root, &mut bodies);
        for body in bodies {
            let Some(mut health) = world.get_mut::<Health>(body) else {
                continue;
            };
            // Never all the way to zero: a body at zero health is destroyed,
            // and the finale would take the column apart before it could be
            // looked at.
            health.current = health.max * (1.0 - level);
        }
        info!("damage levels: column {root:?} set to {level}");
    }
}

/// Land one hit on the same place on every column's hull, bigger each time.
///
/// What the CARVE reads, and it goes through `apply_damage` - the one path a
/// turret, a torpedo and a ram all use - rather than writing marks directly.
/// That is the honest demonstration: if a scripted hit and a fired round do not
/// produce the same crater, the seam is wrong and this row would be hiding it.
///
/// The target is whichever plate is nearest the aim point rather than a named
/// one, because the skin is DERIVED and no column knows in advance which cell
/// it grew there. That plate spends its own health and comes off; what is left
/// over pierces into the section behind it.
#[cfg(feature = "debug")]
fn hit_columns(world: &mut World) {
    for (index, amount) in HITS.iter().enumerate() {
        if *amount <= 0.0 {
            continue;
        }
        let aim = column_aim(index);
        // A bare column has no plates, so the hit lands on the hull section
        // itself - which is the whole point of that column.
        let (target, at) = match nearest_plate(world, aim) {
            // The plate's own centre, so the sphere bites into the hull rather
            // than grazing the surface it was aimed at.
            Some(plate) => (
                plate,
                world
                    .get::<GlobalTransform>(plate)
                    .map_or(aim, |pose| pose.translation()),
            ),
            // On the section's top FACE, half a cell below the aim point. A
            // crater centred on a section's middle would swallow a one-cell
            // hull whole, and one centred where the plate would have been sits
            // half a cell clear of the surface and only dishes it.
            None => match nearest_section(world, aim) {
                Some(section) => (section, aim - Vec3::Y * 0.5),
                None => {
                    warn!("damage levels: column {index} has nothing to hit near {aim}");
                    continue;
                }
            },
        };
        let mut commands = world.commands();
        apply_damage(
            &mut commands,
            target,
            None,
            *amount,
            DamageType::Kinetic,
            Some(at),
        );
        world.flush();
        info!("damage levels: column {index} took {amount} at {at}");
    }
}

/// Where a column is shot: the top of its hull cell, which is one cell forward
/// of the controller and one cell up.
#[cfg(feature = "debug")]
fn column_aim(index: usize) -> Vec3 {
    Vec3::new(index as f32 * COLUMN_PITCH, 1.0, 1.0)
}

/// The skin plate nearest `aim`, IN ITS OWN COLUMN.
///
/// The reach is what makes it its own column's: the row is one world and a
/// bare column has no plates at all, so an unbounded search happily answers
/// with the neighbour's cladding four units away. That put the bare column's
/// hit on the wrong ship entirely.
#[cfg(feature = "debug")]
fn nearest_plate(world: &mut World, aim: Vec3) -> Option<Entity> {
    let mut q_plates = world.query_filtered::<(Entity, &GlobalTransform), With<ShipSkinMarker>>();
    nearest_within(q_plates.iter(world), aim)
}

/// The nearest of `candidates` to `aim`, or `None` when the nearest belongs to
/// another column.
#[cfg(feature = "debug")]
fn nearest_within<'a>(
    candidates: impl Iterator<Item = (Entity, &'a GlobalTransform)>,
    aim: Vec3,
) -> Option<Entity> {
    candidates
        .filter(|(_, at)| at.translation().distance(aim) < COLUMN_PITCH * 0.5)
        .min_by(|(_, a), (_, b)| {
            a.translation()
                .distance_squared(aim)
                .total_cmp(&b.translation().distance_squared(aim))
        })
        .map(|(entity, _)| entity)
}

/// The section nearest `aim`, anywhere in the row: what a bare column is hit on.
#[cfg(feature = "debug")]
fn nearest_section(world: &mut World, aim: Vec3) -> Option<Entity> {
    let mut q_sections = world.query_filtered::<(Entity, &GlobalTransform), With<SectionMarker>>();
    q_sections
        .iter(world)
        .min_by(|(_, a), (_, b)| {
            a.translation()
                .distance_squared(aim)
                .total_cmp(&b.translation().distance_squared(aim))
        })
        .map(|(section, _)| section)
}

/// Every descendant of `root`, root excluded.
#[cfg(feature = "debug")]
fn collect_descendants(world: &World, root: Entity, out: &mut Vec<Entity>) {
    let Some(children) = world.get::<Children>(root) else {
        return;
    };
    for child in children.iter() {
        out.push(child);
        collect_descendants(world, child, out);
    }
}

#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

#[cfg(feature = "debug")]
const DAMAGE_LOOP: &str = "news-0110-damage-levels";

#[cfg(feature = "debug")]
fn gallery_script() -> Script {
    let mut script = Script::new()
        .step("load the row")
        .enter(GameStates::Loading)
        .until(scenario_camera_present())
        .deadline(25.0)
        .add()
        // The skin is derived on the spawn batch, so the plates a column wears
        // have to exist before their health is set.
        .step("let the ships finish dressing")
        .until(elapsed(1.5))
        .add()
        .step("bolt the row down")
        .on_enter(prepare_row)
        .add()
        .step("grade the row by health")
        .on_enter(set_column_levels)
        .add()
        .step("hit the row")
        .on_enter(hit_columns)
        .add()
        // Long enough for the wear to re-dress and for the worst columns to
        // throw a few sparks, so the shot catches them mid-flight.
        .step("let the damage show")
        .until(elapsed(2.0))
        .add();

    if capturing() {
        script = (0..8).fold(
            script
                .step("open the damage-level loop")
                .on_enter(|world| loop_start(world, DAMAGE_LOOP))
                .add(),
            |script, index| {
                script
                    .step("frame the next crack bucket")
                    .on_enter(move |world: &mut World| frame_column(world, index))
                    .until(elapsed(0.65))
                    .add()
            },
        );
        script = script
            .step("close the damage-level loop")
            .on_enter(|world| loop_end(world, DAMAGE_LOOP))
            .until(loop_written(DAMAGE_LOOP))
            .deadline(60.0)
            .add();
    }

    let script = script
        .step("frame the whole row")
        .on_enter(|world: &mut World| {
            let centre = row_centre();
            // High and looking down, for the same reason each column is: the
            // craters are in the hulls' top faces and a broadside shot of the
            // row shows nine silhouettes and no damage. Back far enough to hold
            // the whole row - it is nine columns wide now, not five.
            nova_protocol::nova_debug::harness::pose_camera(
                world,
                centre + Vec3::new(0.0, 21.0, 28.0),
                centre,
            );
        })
        .until(elapsed(0.8))
        .add()
        .step("shoot the row")
        .on_enter(|world: &mut World| {
            nova_protocol::nova_debug::harness::shoot(world, "damage-levels.png")
        })
        .add();

    // One shot per column, so a level can be looked at on its own rather than
    // squinting at a ninth of a wide frame.
    LEVELS
        .iter()
        .enumerate()
        .fold(script, |script, (index, level)| {
            // The bare column stands at the same level as the one before it,
            // so it is named for what it IS rather than for what it is at.
            let name = match CLAD[index] {
                true => format!("damage-levels-{}.png", (level * 100.0).round() as u32),
                false => "damage-levels-bare.png".to_string(),
            };
            script
                .step("frame the next column")
                .on_enter(move |world: &mut World| frame_column(world, index))
                .add()
                .step("settle on the column")
                .until(elapsed(0.6))
                .add()
                .step("shoot the column")
                .on_enter(move |world: &mut World| {
                    nova_protocol::nova_debug::harness::shoot(world, &name)
                })
                // The capture is handed to the render world and written a
                // frame or two later, so the run must not end on the request
                // - the last column's shot was lost to a closed channel.
                .until(elapsed(0.5))
                .add()
        })
}

/// The middle of the row, which the establishing shot is centred on.
fn row_centre() -> Vec3 {
    Vec3::new((LEVELS.len() as f32 - 1.0) * COLUMN_PITCH * 0.5, 0.0, 0.0)
}

/// Point the scenario camera at the place a column was SHOT, from above and
/// off to one side.
///
/// Over the aim point rather than level with the ship, and that is not a taste
/// call: the crater is a dish in the hull's top face, and a camera at eye level
/// sees only the silhouette it leaves. The first cut of this row framed the
/// ships broadside and the finding came back "it just looks like a smaller
/// ship" - which was true of the effect at the time AND of the angle.
///
/// Through `pose_camera` rather than by writing the transform: the scenario
/// camera is free-fly by default and its controller would drive the pose
/// straight back the next frame.
#[cfg(feature = "debug")]
fn frame_column(world: &mut World, index: usize) {
    let aim = column_aim(index);
    nova_protocol::nova_debug::harness::pose_camera(world, aim + Vec3::new(2.2, 3.0, 3.2), aim);
}

/// One clad ship: a hull cell to wear through, a turret and a thruster to
/// spark, and a controller to hold them together.
///
/// Deliberately small. The question is what ONE cell of hull and ONE turret
/// look like at each level, and a big hull would hide both behind its own
/// silhouette.
fn column_ship(sections: &GameSections, level_index: usize) -> ScenarioObjectConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let at = |id: &str, kind: &str, position: Vec3| SpaceshipSectionConfig {
        id: id.to_string(),
        position,
        rotation: Quat::IDENTITY,
        source: SectionSource::Inline(section(kind)),
        modifications: vec![],
    };

    let ship = SpaceshipConfig {
        allegiance: None,
        // NO player ship anywhere in the row. A player ship brings the chase
        // camera, which frames the ship it is following rather than the row -
        // the gallery needs the scenario's own free camera, posed by hand.
        controller: SpaceshipController::None,
        hull: ShipSource::Inline(ShipHull {
            sections: vec![
                at(
                    "controller",
                    "basic_controller_section",
                    Vec3::new(0.0, 0.0, 0.0),
                ),
                at("hull", "reinforced_hull_section", Vec3::new(0.0, 0.0, 1.0)),
                SpaceshipSectionConfig {
                    id: "turret".to_string(),
                    // Seated on the controller's +X face, not a whole cell out
                    // from it. The shared PDC is a HALF-size section whose only
                    // socket is its base plate, a quarter unit in from its own
                    // centre, and the stock controller offers the unit cube's
                    // face centre at +0.5 - so the mount stands at 0.75 and is
                    // rolled to put that plate against the face. A whole-unit
                    // step leaves the plate hanging in vacuum and the graph
                    // reports the gun disconnected. A real ship never has to
                    // know this: `ships::shared::link_points` moves the HULL's
                    // socket out by `PDC_MOUNT_OFFSET` to meet the turret,
                    // where a rig built from stock prototypes gets the generic
                    // sockets and has to place the turret to meet them.
                    position: Vec3::new(0.75, 0.0, 0.0),
                    rotation: Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
                    source: SectionSource::Inline(section("pdc_kinetic_turret_section")),
                    modifications: vec![],
                },
                at(
                    "thruster",
                    "basic_thruster_section",
                    Vec3::new(0.0, 0.0, 2.0),
                ),
            ],
            // Clad in every column but the last - see `CLAD`.
            skin: CLAD[level_index],
            // Nothing may collapse out from under the camera while it is being
            // looked at, and the worst column sits at 0.9 of its health gone.
            collapse_threshold: Some(0.0),
            ..default()
        }),
        ..default()
    };

    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: column_id(level_index),
            name: format!("Level {}", LEVELS[level_index]),
            position: Vec3::new(level_index as f32 * COLUMN_PITCH, 0.0, 0.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(ship),
    }
}

fn gallery(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let ships: Vec<EventActionConfig> = (0..LEVELS.len())
        .map(|index| EventActionConfig::SpawnScenarioObject(column_ship(sections, index)))
        .collect();

    ScenarioConfig {
        description: "One ship at all eight crack levels, side by side.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                ships,
                // Centred on the middle column so the establishing shot holds
                // the whole row.
                ThreePointRig::around("row", row_centre(), 3.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "damage_levels".to_string(),
            "Damage Levels".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}
