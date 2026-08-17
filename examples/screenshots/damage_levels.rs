//! damage_levels: the same ship at five damage levels, side by side.
//!
//! THE GATE for the erosion epic (task 20260813-224826, phase 2). Damage is now
//! one number - the share of a body's own health that is gone - and every
//! damage EFFECT is a component turning that number into its own look. Whether
//! those looks are any good is not a question a test can answer, so this stands
//! them in a row and lets somebody decide.
//!
//! Five identical clad ships, left to right at levels 0.0, 0.25, 0.5, 0.75 and
//! 0.9. Nothing shoots anything: each ship's health is SET to the fraction its
//! column stands for, which is the honest way to show a derived look - if the
//! picture is a function of health, then setting health has to be enough to
//! produce it, and anything that needed a hit to happen would be a bug.
//!
//! What to judge, per effect:
//!
//! - EROSION, on the skin plates. Each plate's shape steps down through the
//!   shell vocabulary as its own health falls, so the hull wears through where
//!   it has been raked. Does a worn hull read as battle damage, or as mush? How
//!   many of the five steps can you actually tell apart?
//! - SCORCH, on the section materials. Reddening, darkening, then a burnt
//!   endpoint. It has lost its allegiance split, so an enemy now shows this
//!   too - is that too much information, or the right amount?
//! - SPARKS, on the turret and thruster. These never lose geometry, because a
//!   turret that has been eroded cannot convincingly still shoot. Does a
//!   sparking turret read as failing without looking broken?
//!
//! The thing NOT here is SHED - expendable pieces coming off a section that
//! keeps working. It needs art with separable pieces and the shipped turret has
//! none, so it is a content question rather than a code one.
//!
//! Hand-run:
//! ```text
//! cargo run --example damage_levels --features debug
//! ```
//! - `L` steps the camera along the row, framing one level at a time.
//!
//! Harnessed, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: load the row, frame it, exit clean.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot `damage-levels.png` (the
//!   whole row) and one `damage-levels-<level>.png` per column.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "damage_levels")]
#[command(version = "1.0.0")]
#[command(about = "One ship at five damage levels, side by side", long_about = None)]
struct Cli;

/// The levels the row stands at, left to right.
///
/// Not a linear sweep to 1.0: a body at 1.0 is dead and the finale takes it, so
/// the last column is the worst a LIVING ship gets. The low end is close
/// together on purpose - if two neighbouring columns are indistinguishable, the
/// effect has fewer usable steps than the number suggests, and that is exactly
/// what this row is for finding out.
const LEVELS: [f32; 5] = [0.0, 0.25, 0.5, 0.75, 0.9];

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
        app.add_plugins(gallery_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_gallery);
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
/// The whole claim of the epic is that the look is a function of health, so
/// this touches health and nothing else: no effect is poked directly, and
/// anything that fails to change here is not actually reading the level.
///
/// Sections and plates are both walked, and for the same reason they are
/// separate pools in the first place - a plate is `HealthIsolated`, so damaging
/// the hull does not wear its cladding and this has to say what both are at.
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
fn gallery_script() -> Script {
    let script = Script::new()
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
        .step("damage the row")
        .on_enter(set_column_levels)
        .add()
        // Long enough for the wear to re-dress and for the worst columns to
        // throw a few sparks, so the shot catches them mid-flight.
        .step("let the damage show")
        .until(elapsed(2.0))
        .add()
        .step("frame the whole row")
        .on_enter(|world: &mut World| {
            let centre = row_centre();
            nova_protocol::nova_debug::harness::pose_camera(
                world,
                centre + Vec3::new(0.0, 5.0, 32.0),
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
    // squinting at a fifth of a wide frame.
    LEVELS
        .iter()
        .enumerate()
        .fold(script, |script, (index, level)| {
            let name = format!("damage-levels-{}.png", (level * 100.0).round() as u32);
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

/// Point the scenario camera at one column, close in.
///
/// Through `pose_camera` rather than by writing the transform: the scenario
/// camera is free-fly by default and its controller would drive the pose
/// straight back the next frame.
#[cfg(feature = "debug")]
fn frame_column(world: &mut World, index: usize) {
    let centre = Vec3::new(index as f32 * COLUMN_PITCH, 0.0, 1.0);
    nova_protocol::nova_debug::harness::pose_camera(
        world,
        centre + Vec3::new(0.0, 1.6, 5.0),
        centre,
    );
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
                at("turret", "light_turret_section", Vec3::new(1.0, 0.0, 0.0)),
                at(
                    "thruster",
                    "basic_thruster_section",
                    Vec3::new(0.0, 0.0, 2.0),
                ),
            ],
            // Clad, which is the whole point: the plates are what erode.
            skin: true,
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
        description: "One ship at five damage levels, side by side.".to_string(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
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
