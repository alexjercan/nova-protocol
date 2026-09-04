//! planet_types: the planetoid-look lineup for the 2026-08-15 research round
//! (task 20260815-231945) - six seeded planet TYPES over a row of one type at
//! four seeds, a focus planet the keys re-draw, and today's planetoid standing
//! beside them at the same framing for contrast.
//!
//! What the row is FOR: a big body today is an asteroid with a big radius, so
//! it wears the rock material - one photo tiled hundreds of times across a
//! kilometre of surface, which reads as a flat grey crust. These planets carry
//! no texture at all. Elevation and latitude pick a band out of the type's
//! palette, and the variation inside a band comes from a procedural field on
//! the body's own direction, which has no tile to repeat.
//!
//! Hand-run (the comparison itself - WASD+mouse free-fly):
//! ```text
//! cargo run --example planet_types --features debug
//! # keys 1-9 re-draw the focus planet; arrows cycle; labels name the row
//! ```
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load, frame, walk the draws, exit clean.
//!   This is the path `probe run` takes.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot the lineup, the
//!   backdrop-framed before/after pair, three focus draws and a close pass
//!   (staged under `NOVA_CAPTURE_DIR`).

#[path = "shared/compare.rs"]
mod compare;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "planet_types")]
#[command(version = "1.0.0")]
#[command(about = "Seeded planet types beside today's textured planetoid", long_about = None)]
struct Cli;

/// Radius of a lineup planet. The menu planetoid's own nominal radius, so the
/// row is at the size the game already authors.
const ROW_RADIUS: Meters = Meters(200.0);

/// Gap between lineup planets, centre to centre.
const ROW_SPACING: f32 = 700.0;

/// Height of the type row: one planet per [`PlanetType`], all at one seed, so
/// the row varies by TYPE only.
const TYPE_ROW_Y: f32 = 520.0;

/// Height of the seed row: one type at four seeds, so the row varies by SEED
/// only.
const SEED_ROW_Y: f32 = -520.0;

/// The type the seed row walks.
const SEED_ROW_TYPE: PlanetType = PlanetType::DustWorld;

/// The seed the type row is drawn at.
const TYPE_ROW_SEED: u32 = 7;

/// The seeds the seed row walks.
const ROW_SEEDS: [u32; 4] = [3, 11, 4242, 20_260_904];

/// Icosphere subdivisions a lineup planet is meshed at. Below the default: ten
/// of them are meshed in one frame at load, and at row range a facet is well
/// under a pixel anyway.
const ROW_SUBDIVISIONS: u32 = 32;

/// Icosphere subdivisions the focus planet is meshed at. The close pass is the
/// only shot where a facet edge could show.
const FOCUS_SUBDIVISIONS: u32 = 64;

/// Where the focus planet sits: far off the lineup's axis, so one scene holds
/// both without either photobombing the other.
const FOCUS_CENTER: Meters3 = Meters3::new(0.0, -14_000.0, 0.0);

/// Radius of the focus planet. Sized so that at the menu's own camera offset
/// it subtends what today's 200 m planetoid does - the rock generator reaches
/// about four times past its nominal radius, so 800 m is the honest match.
const FOCUS_RADIUS: Meters = Meters(800.0);

/// Where today's planetoid stands: the mirror of the focus planet, so the
/// before and after shots are the same pose about a different body.
const TODAY_CENTER: Meters3 = Meters3::new(0.0, 14_000.0, 0.0);

/// Nominal radius of today's planetoid - the menu's own number.
const TODAY_RADIUS: Meters = Meters(200.0);

/// Silhouette seed for today's planetoid. Pinned so the before shot is the
/// same rock on every run.
const TODAY_SEED: u32 = 20_260_904;

/// Where the lineup is framed from. Far enough back to hold both rows, close
/// enough that a 200 m planet is a third of the frame height.
const LINEUP_CAMERA: Meters3 = Meters3::new(0.0, 0.0, 3_100.0);

/// The menu backdrop's own camera offset from the body it frames
/// (`main_menu/shared.rs` documents `(0, 570, 1920)` as the reference pose).
/// Both halves of the before/after pair are shot from here.
const BACKDROP_OFFSET: Meters3 = Meters3::new(0.0, 570.0, 1_920.0);

/// Camera offset for a focus shot: the whole planet in frame with air around
/// it. At the scenario camera's 45-degree vertical field the body fills about
/// three quarters of the frame height from here.
const FOCUS_OFFSET: Meters3 = Meters3::new(0.0, 350.0, 2_600.0);

/// Camera offset for the close pass: about 300 m over the surface of an 800 m
/// body.
const CLOSE_OFFSET: Meters3 = Meters3::new(0.0, 0.0, 1_150.0);

/// Where the close pass looks: just inside the horizon, so the limb cuts
/// across the frame and the relief is read side-on rather than from overhead.
const CLOSE_LOOK: Meters3 = Meters3::new(0.0, 430.0, 640.0);

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(planet_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - a
        // posed lineup holds no steady-state load worth measuring.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        // Clean frames at the fleet's known 16:9; dev overlays and the
        // fps/version bar out of shot (the subject labels are NOT HUD-tier,
        // so they stay - they are the point of the frame).
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        app.add_plugins(lineup_script());
    }

    app.run()
}

fn planet_plugin(app: &mut App) {
    // The planet material's pipeline. Added HERE and not by
    // `ScenarioObjectsPlugin`: this round adds a look, not a scenario object,
    // so nothing an authored scenario spawns changes.
    app.add_plugins(PlanetSurfacePlugin);
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
    app.add_systems(
        Update,
        (
            (select_by_keys, redraw_focus)
                .chain()
                .run_if(resource_exists::<PlanetRoster>),
            compare::position_labels,
        ),
    );
}

/// One entry the focus planet can be re-drawn as.
struct PlanetEntry {
    /// What the entry is called on screen.
    label: String,
    /// The config it draws from.
    config: PlanetConfig,
}

/// The roster the number keys drive. Changing `selected` re-meshes and
/// re-dresses the focus planet, so a script that writes it exercises the same
/// path a key press does.
#[derive(Resource)]
struct PlanetRoster {
    entries: Vec<PlanetEntry>,
    selected: usize,
}

/// Marks the focus planet: the big body the keys re-draw.
#[derive(Component)]
struct FocusPlanet;

/// The comparison stage: the skybox, the repo's three-point rig, and today's
/// planetoid as a real authored asteroid object, so the before shot goes
/// through the shipped spawn path rather than through a lookalike.
fn planet_stage(game_assets: &GameAssets) -> ScenarioConfig {
    let mut actions = ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions();
    actions.push(EventActionConfig::SpawnScenarioObject(
        ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "todays_planetoid".to_string(),
                name: "Today's Planetoid".to_string(),
                position: TODAY_CENTER,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                radius: TODAY_RADIUS,
                texture: game_assets.asteroid_texture.clone().into(),
                material: None,
                destroy_sound: None,
                mass: None,
                invulnerable: true,
                lock_signature: None,
                seed: Some(TODAY_SEED),
            }),
        },
    ));

    ScenarioConfig {
        description: "Planet Types - seeded planet surfaces beside today's planetoid".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions,
        }],
        ..ScenarioConfig::new(
            "planet_types".to_string(),
            "Planet Types".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Load the stage, mesh both rows, and stand the focus planet up.
fn load_scene(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PlanetSurfaceMaterial>>,
) {
    commands.trigger(LoadScenario(planet_stage(&game_assets)));

    let types: Vec<PlanetConfig> = PlanetType::ALL
        .iter()
        .map(|planet_type| PlanetConfig::new(*planet_type, ROW_RADIUS, TYPE_ROW_SEED))
        .collect();
    spawn_row(
        &mut commands,
        &mut meshes,
        &mut materials,
        &types,
        TYPE_ROW_Y,
        |surface| format!("{} - seed {}", surface.planet_type.name(), surface.seed),
    );

    let seeds: Vec<PlanetConfig> = ROW_SEEDS
        .iter()
        .map(|seed| PlanetConfig::new(SEED_ROW_TYPE, ROW_RADIUS, *seed))
        .collect();
    spawn_row(
        &mut commands,
        &mut meshes,
        &mut materials,
        &seeds,
        SEED_ROW_Y,
        |surface| format!("seed {}", surface.seed),
    );

    // The focus planet: dressed by `redraw_focus` on the roster's insert
    // frame, and re-drawn on every key press.
    commands.spawn((
        FocusPlanet,
        Transform::from_translation(FOCUS_CENTER.to_engine())
            .with_scale(Vec3::splat(FOCUS_RADIUS.to_engine())),
        Visibility::Inherited,
    ));

    compare::spawn_focus_readout(&mut commands);
    commands.insert_resource(PlanetRoster {
        entries: focus_roster(),
        selected: 0,
    });
}

/// The draws the keys walk on the focus planet: every type at one seed, then
/// the same type at two more, so both axes of the config are reachable by hand.
fn focus_roster() -> Vec<PlanetEntry> {
    let mut entries: Vec<PlanetEntry> = PlanetType::ALL
        .iter()
        .map(|planet_type| PlanetEntry {
            label: format!("{} - seed {TYPE_ROW_SEED}", planet_type.name()),
            config: PlanetConfig::new(*planet_type, FOCUS_RADIUS, TYPE_ROW_SEED),
        })
        .collect();
    for seed in [4242u32, 20_260_904] {
        entries.push(PlanetEntry {
            label: format!("{} - seed {seed}", SEED_ROW_TYPE.name()),
            config: PlanetConfig::new(SEED_ROW_TYPE, FOCUS_RADIUS, seed),
        });
    }
    entries
}

/// Mesh, dress and stand up one row of planets, spread about the origin on X.
fn spawn_row(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<PlanetSurfaceMaterial>,
    configs: &[PlanetConfig],
    height: f32,
    label: impl Fn(&PlanetSurface) -> String,
) {
    let offset = (configs.len() as f32 - 1.0) / 2.0;
    for (index, config) in configs.iter().enumerate() {
        let visual = PlanetVisual::build(config, ROW_SUBDIVISIONS);
        let position = Meters3::new((index as f32 - offset) * ROW_SPACING, height, 0.0);
        // Engine boundary: a Bevy transform is world units, and the unit-space
        // mesh is scaled to the authored radius here.
        let subject = commands
            .spawn((
                Mesh3d(meshes.add(visual.mesh)),
                MeshMaterial3d(materials.add(visual.material)),
                Transform::from_translation(position.to_engine())
                    .with_scale(Vec3::splat(config.radius.to_engine())),
            ))
            .id();
        compare::spawn_subject_label(
            commands,
            subject,
            &label(&visual.surface),
            Vec3::Y * (config.radius.to_engine() * 1.4),
        );
    }
}

/// `Digit1`..`Digit9`, in roster order.
const DIGIT_KEYS: [KeyCode; 9] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];

/// Number keys pick a roster entry for the focus planet; left/right arrows
/// cycle (WASD stays free for the scenario camera).
fn select_by_keys(keyboard: Res<ButtonInput<KeyCode>>, mut roster: ResMut<PlanetRoster>) {
    let count = roster.entries.len();
    if count == 0 {
        return;
    }
    for (index, key) in DIGIT_KEYS.iter().enumerate().take(count) {
        if keyboard.just_pressed(*key) {
            roster.selected = index;
        }
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        roster.selected = (roster.selected + 1) % count;
    }
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        roster.selected = (roster.selected + count - 1) % count;
    }
}

/// Re-mesh and re-dress the focus planet from the selected entry, and rewrite
/// the readout with what the seed actually drew.
///
/// A full rebuild, not a swapped texture: the whole point is that a type and a
/// seed generate a body, so the demonstration has to generate one.
fn redraw_focus(
    roster: Res<PlanetRoster>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PlanetSurfaceMaterial>>,
    q_focus: Query<Entity, With<FocusPlanet>>,
    mut q_readout: Query<&mut Text, With<compare::FocusReadout>>,
) {
    if !roster.is_changed() || roster.entries.is_empty() {
        return;
    }
    let entry = &roster.entries[roster.selected];
    let visual = PlanetVisual::build(&entry.config, FOCUS_SUBDIVISIONS);

    for focus in &q_focus {
        commands.entity(focus).insert((
            Mesh3d(meshes.add(visual.mesh.clone())),
            MeshMaterial3d(materials.add(visual.material.clone())),
        ));
    }
    for mut text in &mut q_readout {
        **text = format!(
            "FOCUS: {}   [{}]   [keys 1-{} pick, arrows cycle]",
            entry.label,
            visual.surface.summary(),
            roster.entries.len()
        );
    }
}

/// The focus draws the walk shoots, as roster indices.
#[cfg(feature = "debug")]
const FOCUS_SHOTS: [(usize, &str); 3] = [
    (1, "planet-types-focus-dust.png"),
    (3, "planet-types-focus-volcanic.png"),
    (5, "planet-types-focus-temperate.png"),
];

/// The roster index the backdrop-framed and close shots are taken on.
#[cfg(feature = "debug")]
const HERO_INDEX: usize = 1;

/// The driven walk: frame the lineup, shoot the before/after pair at the
/// menu's own camera offset, walk three focus draws, and finish with a close
/// pass over the hero planet.
#[cfg(feature = "debug")]
fn lineup_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the planet scene")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("frame the lineup")
        .on_enter(|world: &mut World| {
            pose_camera(world, LINEUP_CAMERA, Meters3::ZERO);
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the lineup")
        .on_enter(|world: &mut World| shoot(world, "planet-types-lineup.png"))
        .until(shot_written("planet-types-lineup.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // The before shot: today's planetoid at the menu's own camera offset.
        .step("frame today's planetoid")
        .on_enter(|world: &mut World| {
            pose_camera(world, TODAY_CENTER + BACKDROP_OFFSET, TODAY_CENTER);
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot today's planetoid")
        .on_enter(|world: &mut World| shoot(world, "planet-types-today-planetoid.png"))
        .until(shot_written("planet-types-today-planetoid.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // The after shot: the same pose about the generated planet.
        .step("draw the hero planet")
        .on_enter(|world: &mut World| {
            world.resource_mut::<PlanetRoster>().selected = HERO_INDEX;
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("frame the hero planet at backdrop range")
        .on_enter(|world: &mut World| {
            pose_camera(world, FOCUS_CENTER + BACKDROP_OFFSET, FOCUS_CENTER);
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the hero planet at backdrop range")
        .on_enter(|world: &mut World| shoot(world, "planet-types-backdrop.png"))
        .until(shot_written("planet-types-backdrop.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add();

    for (index, path) in FOCUS_SHOTS {
        script = script
            .step(format!("draw the focus planet as roster entry {index}"))
            .on_enter(move |world: &mut World| {
                world.resource_mut::<PlanetRoster>().selected = index;
            })
            .until(frames(SETTLE_FRAMES))
            .add()
            .step(format!("frame {path}"))
            .on_enter(|world: &mut World| {
                pose_camera(world, FOCUS_CENTER + FOCUS_OFFSET, FOCUS_CENTER);
            })
            .until(frames(SETTLE_FRAMES))
            .add()
            .step(format!("shoot {path}"))
            .on_enter(move |world: &mut World| shoot(world, path))
            .until(shot_written(path))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
    }

    script
        .step("draw the hero planet for the close pass")
        .on_enter(|world: &mut World| {
            world.resource_mut::<PlanetRoster>().selected = HERO_INDEX;
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("frame the close pass")
        .on_enter(|world: &mut World| {
            pose_camera(
                world,
                FOCUS_CENTER + CLOSE_OFFSET,
                FOCUS_CENTER + CLOSE_LOOK,
            );
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the close pass")
        .on_enter(|world: &mut World| shoot(world, "planet-types-close.png"))
        .until(shot_written("planet-types-close.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
