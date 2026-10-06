//! screenshot_generated_ships: four of the open world's generated ships, one
//! per [`ShipRoleType`] (civilian, industrial, scavenger, armored, left to
//! right), standing beside the ruin [`generate_wreck`] makes of the armored
//! ship's own request (`news-0150-generated-ships.png`).
//!
//! In a streamed world a derelict comes from an extinct civilization, so this
//! pairing -- a wreck beside its own intact twin -- is staged for the shot,
//! never a scene the game loads on its own. The wreck is `generate_wreck` of
//! the EXACT [`ShipLayoutRequest`] the twin beside it was built from.
//!
//! `NOVA_AUTOPILOT=1` alone walks the smoke path and captures nothing;
//! `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1` also writes the PNG, staged under
//! `NOVA_CAPTURE_DIR`:
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_generated_ships --features debug
//! # headless: drop NOVA_CAPTURE; needs a display (e.g. Xvfb :99 & DISPLAY=:99)
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;
use nova_wfc::prelude::lint_errors;
use nova_world::prelude::{CivilizationId, ShipRoleType};

/// The world seed the other v0.15 figures fly. Only a failure message reads it.
const WORLD_SEED: u32 = 115;

/// The first hull seed a role's stand tries.
const FIRST_HULL_SEED: u32 = 1;

/// How many hull seeds a role tries before its request is a reported gap.
const HULL_SEED_SEARCH: u32 = 32;

/// The advancement steps a role's highest eligible advancement is searched on.
const ADVANCEMENT_STEPS: u32 = 20;

/// Clear space between two neighbouring stands' clearance spheres, in engine units.
const STAND_GAP: f32 = 2.0;

/// Every posed ship stands at the same quarter yaw, as the `wfc_ships` row does.
const SHIP_YAW: f32 = -0.5;

/// The wreck's scenario id and `Name`: the ruin beside its intact twin.
const WRECK_NAME: &str = "armored-wreck";

/// The scenario id this lineup loads.
const SCENARIO_ID: &str = "generated_ships_lineup";

/// The still this producer writes.
#[cfg(feature = "debug")]
const SHOT_FILE: &str = "news-0150-generated-ships.png";

#[derive(Parser)]
#[command(name = "screenshot_generated_ships")]
#[command(version = "1.0.0")]
#[command(about = "Capture the open world's generated ships beside the ruin of the armored one. Autopilot-only", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    let packs = nova_authoring::lint_walk::repo_ship_part_packs(&["base"]);
    let snapshot = ShipPartSnapshot::build(&packs).unwrap_or_else(|faults| {
        panic!(
            "screenshot_generated_ships: the base snapshot is refused:\n  {}",
            faults
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n  ")
        )
    });
    let civilization = CivilizationId {
        world_seed: WORLD_SEED,
        node: [0, 0, 0],
    };

    // One stand per role, keeping the armored one's exact request for the wreck.
    let mut clearances = Vec::new();
    let mut designs: Vec<(String, ShipDesign)> = Vec::new();
    let mut armored_request = None;
    let mut armored_sections = 0;
    for role in ShipRoleType::ALL {
        let advancement = highest_eligible_advancement(&snapshot, role);
        let (request, layout) = first_hull_layout(&snapshot, civilization, role, advancement);
        if role == ShipRoleType::Armored {
            armored_request = Some(request);
            armored_sections = layout.design.sections.len();
        }
        clearances.push(layout.clearance.to_engine());
        designs.push((role.label().to_string(), layout.design));
    }
    let armored_request = armored_request.expect("ShipRoleType::ALL carries Armored");

    let wreck_layout = generate_wreck(&snapshot, armored_request)
        .unwrap_or_else(|failure| panic!("screenshot_generated_ships: wreck: {failure}"));
    let wreck_sections = wreck_layout.design.sections.len();
    assert!(
        wreck_sections < armored_sections,
        "screenshot_generated_ships: wreck of armored seed {} at advancement {:.2} kept {} of \
         its twin's {armored_sections} sections -- the ruin is not smaller than its twin",
        armored_request.seed,
        armored_request.advancement,
        wreck_sections,
    );
    clearances.push(wreck_layout.clearance.to_engine());
    designs.push((WRECK_NAME.to_string(), wreck_layout.design));

    let positions = stand_positions(&clearances);
    let mut ships: Vec<LineupShip> = designs
        .into_iter()
        .zip(&positions)
        .map(|((name, design), position)| LineupShip {
            name,
            position: *position,
            config: SpaceshipConfig {
                controller: SpaceshipController::None,
                design: ShipDesignSource::Inline(design),
                ..default()
            },
        })
        .collect();
    let mut wreck = ships.pop().expect("the wreck was just pushed");
    wreck.config.allegiance = Some(Allegiance::Neutral);
    wreck.config.lootable = true;

    let light_scale = clearances.iter().copied().fold(1.0, f32::max) * 0.5;
    let reach = positions
        .iter()
        .zip(&clearances)
        .map(|(position, clearance)| position.length() + clearance)
        .fold(1.0, f32::max);
    let stage = Stage {
        ships,
        wreck,
        light_scale,
        // Off the bows (-Z is ahead of an unturned hull): from astern the frame
        // was a row of drive bells.
        camera_position: Meters3::from_engine(Vec3::new(0.0, reach * 0.5, -reach * 1.3)),
        camera_look_at: Meters3::ZERO,
    };

    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| {
            app.insert_resource(stage.clone());
            app.add_systems(OnEnter(GameAssetsStates::Loaded), load_lineup);
            app.add_observer(stand_the_wreck);
        })
        .build();

    #[cfg(feature = "debug")]
    {
        // Inert without NOVA_PROBE_*: run timeline + engine-bound invariants, no
        // frame-time capture (a posed lineup has no steady-state window).
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // Known 16:9 size, no dev chrome, a held scene (zero-g would drift it
        // between a settle and its shot).
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_systems(Update, freeze_bodies.run_if(capturing));
        app.add_plugins(lineup_script());
    }

    app.run()
}

/// The highest advancement at which the snapshot serves `role`: full
/// advancement if eligible there, else the highest step of
/// [`ADVANCEMENT_STEPS`] below it that is. Panics when none serves the role.
fn highest_eligible_advancement(snapshot: &ShipPartSnapshot, role: ShipRoleType) -> f32 {
    if snapshot.eligible_roles(1.0).contains(&role) {
        return 1.0;
    }
    (0..ADVANCEMENT_STEPS)
        .rev()
        .map(|step| step as f32 / ADVANCEMENT_STEPS as f32)
        .find(|advancement| snapshot.eligible_roles(*advancement).contains(&role))
        .unwrap_or_else(|| {
            panic!(
                "screenshot_generated_ships: the base snapshot serves no {} at any advancement",
                role.label()
            )
        })
}

/// The request and layout of the first hull seed in
/// `FIRST_HULL_SEED..FIRST_HULL_SEED + HULL_SEED_SEARCH` that lays out. Panics
/// when every seed fails, naming the last failure's seed, role and constraint.
fn first_hull_layout(
    snapshot: &ShipPartSnapshot,
    civilization: CivilizationId,
    role: ShipRoleType,
    advancement: f32,
) -> (ShipLayoutRequest, ShipLayout) {
    let mut last_failure = None;
    for seed in FIRST_HULL_SEED..FIRST_HULL_SEED + HULL_SEED_SEARCH {
        let request = ShipLayoutRequest {
            seed,
            civilization,
            role,
            advancement,
        };
        match generate_ship(snapshot, request) {
            Ok(layout) => return (request, layout),
            Err(failure) => last_failure = Some(failure),
        }
    }
    panic!(
        "screenshot_generated_ships: no hull seed in {FIRST_HULL_SEED}..{} lays out: {}",
        FIRST_HULL_SEED + HULL_SEED_SEARCH,
        last_failure.expect("the search range is non-empty"),
    );
}

/// One row of stands, left to right as seen from the bow-side camera (so
/// along -X), each cell as wide as the widest clearance sphere plus
/// [`STAND_GAP`], centred on the origin.
fn stand_positions(clearances: &[f32]) -> Vec<Vec3> {
    let pitch = clearances.iter().copied().fold(0.0, f32::max) * 2.0 + STAND_GAP;
    let columns = clearances.len() as f32;
    (0..clearances.len())
        .map(|index| Vec3::NEG_X * ((index as f32 - (columns - 1.0) * 0.5) * pitch))
        .collect()
}

/// One stand of the lineup: its id/`Name`, where it stands, and its ship.
#[derive(Clone)]
struct LineupShip {
    name: String,
    position: Vec3,
    config: SpaceshipConfig,
}

/// Everything the loader needs to stand the lineup, computed once at boot.
#[derive(Resource, Clone)]
struct Stage {
    /// The four role ships, spawned by the scenario.
    ships: Vec<LineupShip>,
    /// The wreck, spawned beside them by [`stand_the_wreck`].
    wreck: LineupShip,
    light_scale: f32,
    camera_position: Meters3,
    camera_look_at: Meters3,
}

/// Build and load the lineup's scenario, once, on reaching the state where
/// the merged catalog already carries every base section.
fn load_lineup(
    mut commands: Commands,
    stage: Res<Stage>,
    sections: Res<GameSections>,
    game_assets: Res<GameAssets>,
) {
    commands.trigger(LoadScenario(build_scenario(
        &game_assets,
        &sections,
        &stage,
    )));
}

/// Spawn the wreck with the bundle the open world spawns a derelict with:
/// [`DerelictShipMarker`] in the same bundle as the root, so every section but
/// hull and docking port spawns inactive and its drives stay dark. A scenario
/// action has no derelict field.
fn stand_the_wreck(_: On<ScenarioLoaded>, mut commands: Commands, stage: Res<Stage>) {
    let wreck = &stage.wreck;
    commands.spawn((
        base_scenario_object(&BaseScenarioObjectConfig {
            id: wreck.name.clone(),
            name: wreck.name.clone(),
            position: Meters3::from_engine(wreck.position),
            rotation: Quat::from_rotation_y(SHIP_YAW),
        }),
        spaceship_scenario_object(wreck.config.clone()),
        Allegiance::Neutral,
        DerelictShipMarker,
        LootableShipMarker,
    ));
}

/// `name` standing at `position`, facing [`SHIP_YAW`], flying `ship`.
fn ship_action(name: &str, position: Vec3, ship: SpaceshipConfig) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: name.to_string(),
            name: name.to_string(),
            position: Meters3::from_engine(position),
            rotation: Quat::from_rotation_y(SHIP_YAW),
        },
        kind: ScenarioObjectKind::Spaceship(ship),
    })
}

/// The scenario the lineup loads: every stand, the light rig and a cut to the
/// framing pose, checked against the merged catalog before it is handed over.
fn build_scenario(
    game_assets: &GameAssets,
    sections: &GameSections,
    stage: &Stage,
) -> ScenarioConfig {
    let actions = stage
        .ships
        .iter()
        .map(|ship| ship_action(&ship.name, ship.position, ship.config.clone()))
        .chain(ThreePointRig::around("stand", Meters3::ZERO, stage.light_scale).actions())
        .chain(std::iter::once(EventActionConfig::SetCamera(
            SetCameraActionConfig {
                position: stage.camera_position,
                look_at: stage.camera_look_at,
                blend: None,
            },
        )))
        .collect();
    let scenario = ScenarioConfig {
        description: "Four generated open-world ships beside the ruin of their armored twin"
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions,
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Generated Ships".to_string(),
            game_assets.cubemap.clone().into(),
        )
    };
    let errors = lint_errors(&scenario, sections);
    assert!(
        errors.is_empty(),
        "screenshot_generated_ships: a posed ship is content the game would refuse:\n  {}",
        errors.join("\n  ")
    );
    scenario
}

/// Advance once exactly the lineup's roots stand: one per role plus the wreck.
#[cfg(feature = "debug")]
fn ships_standing() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<(), With<SpaceshipRootMarker>>()
            .is_some_and(|mut query| query.iter(world).count() == ShipRoleType::ALL.len() + 1)
    })
}

/// Exactly five [`SpaceshipRootMarker`] roots stand, named for the four roles
/// plus the wreck -- nothing missing, nothing extra.
#[cfg(feature = "debug")]
fn assert_lineup(world: &mut World) {
    let mut expected: Vec<String> = ShipRoleType::ALL
        .into_iter()
        .map(|role| role.label().to_string())
        .collect();
    expected.push(WRECK_NAME.to_string());
    expected.sort();

    let mut standing: Vec<String> = world
        .query_filtered::<&Name, With<SpaceshipRootMarker>>()
        .iter(world)
        .map(|name| name.as_str().to_string())
        .collect();
    standing.sort();

    assert_eq!(
        standing, expected,
        "screenshot_generated_ships: the lineup stands {standing:?}, expected exactly {expected:?}"
    );
}

/// The driven walk: reach the lineup, settle it, assert it, shoot it.
#[cfg(feature = "debug")]
fn lineup_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("reach the lineup")
        .enter(GameStates::Loading)
        .until(and(scenario_is_built(), ships_standing()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("settle the lineup")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("the lineup is the one asked for")
        .on_enter(assert_lineup)
        .until(frames(1))
        .add()
        .step(format!("shoot {SHOT_FILE}"))
        .on_enter(|world: &mut World| {
            hide_hud(world);
            shoot(world, SHOT_FILE);
        })
        .until(shot_written(SHOT_FILE))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
