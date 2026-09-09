//! loop_goto_standoff: two catalog hulls sent to one mark, watched from above.
//!
//! The stolen warship and the patrol gunship start the same distance from a
//! nav beacon and fly the production GOTO to it. Both park the same margin
//! clear of the orb's FACE, measured from their own skin, so the warship's
//! park point sits further out than the gunship's by the difference of the
//! two hulls. That difference is the whole picture: nothing here is staged but
//! the margin, which is authored small so the hulls and the gap read at one
//! scale.
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load the set, fly the legs, exit clean.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also record the two arrivals as
//!   `news-0130-goto-standoff.webm` (staged under `NOVA_CAPTURE_DIR`).

use std::collections::HashSet;

use bevy::prelude::*;
use nova_authoring::prelude::*;
use nova_protocol::prelude::*;

/// The mark both hulls are sent to.
const BEACON_ID: &str = "standoff_mark";
/// Where it sits: the origin, so every distance in this set is a radius.
const BEACON_POSITION: Meters3 = Meters3::ZERO;
/// The orb's radius. Big for a nav point, so the thing the hulls park off has
/// a visible face from a lens 900 m up.
const BEACON_RADIUS: Meters = Meters(40.0);

const WARSHIP_ID: &str = "standoff_warship";
const GUNSHIP_ID: &str = "standoff_gunship";

/// How far from the mark's centre each hull starts, along its own bearing.
///
/// Short on purpose: a loop is capped at 20 s, and the warship's leg is the
/// long one - it parks about 250 m out, so this is a 350 m run under power.
const START_DISTANCE: Meters = Meters(600.0);
/// The two bearings, in world XZ, as unit directions from the mark: 32
/// degrees apart, so the tracks run side by side without the warship's beam
/// shadowing the gunship from above.
const WARSHIP_BEARING: Vec3 = Vec3::new(-0.2756, 0.0, 0.9613);
const GUNSHIP_BEARING: Vec3 = Vec3::new(0.2756, 0.0, 0.9613);

/// The margin each hull leaves between its own skin and the orb's face, the
/// same number on both. Authored well under the 500 m default so the gap
/// between the two park points (the difference of the two hull radii) is a
/// visible fraction of the frame rather than a rounding error on a long
/// standoff.
///
/// What the take actually shows, from the park log this producer writes: the
/// computer's arrival plan is tracked with lag, and a hull carries past the
/// park boundary by an amount that grows with the speed it arrives at. At a
/// 60 m margin the warship came to rest 21 m inside the orb's face and the
/// gunship 47 m inside it; at 150 m the warship stops 86 m clear and the
/// gunship still 15 m inside. Until the arrival holds its margin on a light
/// hull, this loop is a reproduction of the creep and not a figure of the
/// rule, which is why the site does not ship it yet.
const ARRIVAL_MARGIN: Meters = Meters(150.0);

fn main() -> bevy::app::AppExit {
    let mut app = AppBuilder::new().with_game_plugins(standoff_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(standoff_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn standoff_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_standoff);
}

fn load_standoff(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    ships: Res<GameShips>,
) {
    let scenario = standoff(&game_assets);
    refuse_broken(&scenario, &sections, &ships);
    commands.trigger(LoadScenario(scenario));
}

fn standoff(game_assets: &GameAssets) -> ScenarioConfig {
    let objects = [
        beacon(),
        ship_object(
            WARSHIP_ID,
            "Stolen Warship",
            WARSHIP_BEARING,
            BLOCK_WARSHIP_SHIP_ID,
        ),
        ship_object(
            GUNSHIP_ID,
            "Patrol Gunship",
            GUNSHIP_BEARING,
            BLOCK_GUNSHIP_SHIP_ID,
        ),
    ];

    ScenarioConfig {
        description: "Two hulls sent to one mark from the same distance".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: objects
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .chain(
                    ThreePointRig::around("standoff", Meters3::new(0.0, 0.0, 300.0), 40.0)
                        .actions(),
                )
                .collect(),
        }],
        ..ScenarioConfig::new(
            "loop_goto_standoff".to_string(),
            "GOTO Standoff".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

fn beacon() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: BEACON_ID.to_string(),
            name: "Mark".to_string(),
            position: BEACON_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: "MARK".to_string(),
            radius: BEACON_RADIUS,
            color: Color::srgb(0.4, 0.75, 1.0),
            area_radius: None,
            lock_signature: None,
        }),
    }
}

/// One catalog hull on its bearing, nose on the mark, with nobody at the helm:
/// the script engages the flight computer, and nothing else drives it.
fn ship_object(id: &str, name: &str, bearing: Vec3, ship: &str) -> ScenarioObjectConfig {
    let position = Meters3(bearing * START_DISTANCE.0);
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::from_rotation_arc(Vec3::NEG_Z, -bearing),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            hull: hull(ship),
            ..default()
        }),
    }
}

fn refuse_broken(scenario: &ScenarioConfig, sections: &GameSections, ships: &GameShips) {
    let known = KnownSections::from_configs(sections.iter());
    let issues = lint_scenario(
        scenario,
        &known,
        &KnownShips::from_configs(ships.iter()),
        &HashSet::from([scenario.id.clone()]),
        &build_channels()
            .into_iter()
            .map(|channel| channel.id)
            .collect(),
    );
    let errors: Vec<_> = issues
        .iter()
        .filter(|issue| issue.severity == LintSeverity::Error)
        .map(|issue| issue.message.as_str())
        .collect();
    assert!(
        errors.is_empty(),
        "loop_goto_standoff: the set failed content lint:\n  {}",
        errors.join("\n  "),
    );
}

#[cfg(feature = "debug")]
const LOOP_NAME: &str = "news-0130-goto-standoff";

/// Where the lens stands, in meters: high over the tracks and a little down
/// range, so the frame holds the mark near the top and both start marks near
/// the bottom, and the lens is not looking straight down its own up vector.
/// The lens is 45 degrees tall, so 900 m up it spans about 740 m of track:
/// the mark sits a tenth down from the top edge, the starts a tenth up from
/// the bottom.
#[cfg(feature = "debug")]
const EYE: Meters3 = Meters3::new(0.0, 900.0, 460.0);
/// What it looks at: the middle of the two legs.
#[cfg(feature = "debug")]
const LOOK: Meters3 = Meters3::new(0.0, 0.0, 260.0);

/// How long the loop holds after the second hull has parked.
#[cfg(feature = "debug")]
const PARKED_HOLD_SECS: f32 = 1.5;
/// The real seconds the two legs may take before the run fails: the frame
/// cap bounds the recording, this bounds a leg that never parks.
#[cfg(feature = "debug")]
const LEGS_DEADLINE_SECS: f32 = 120.0;

#[cfg(feature = "debug")]
fn entity_by_id(world: &mut World, id: &str) -> Option<Entity> {
    let mut query = world.query::<(Entity, &EntityId)>();
    query
        .iter(world)
        .find(|(_, live)| live.0 == id)
        .map(|(entity, _)| entity)
}

/// Put the lens over the set and take the HUD down.
#[cfg(feature = "debug")]
fn frame_from_above(world: &mut World) {
    hide_hud(world);
    pose_camera(world, EYE, LOOK);
}

/// Engage the production GOTO on both hulls, at the authored margin.
///
/// The margin is installed as the hull's own arrival standoff, which is the
/// dial the flight computer reads; the target's radius and the hull's radius
/// are the computer's own business.
#[cfg(feature = "debug")]
fn send_both_to_the_mark(world: &mut World) {
    let Some(mark) = entity_by_id(world, BEACON_ID) else {
        panic!("loop_goto_standoff: no beacon '{BEACON_ID}' to send the hulls to");
    };
    for id in [WARSHIP_ID, GUNSHIP_ID] {
        let Some(ship) = entity_by_id(world, id) else {
            panic!("loop_goto_standoff: no ship '{id}' in the set");
        };
        world.entity_mut(ship).insert((
            // Engine boundary: the arrival rule compares this against an
            // avian position every tick.
            FlightArrivalStandoff(ARRIVAL_MARGIN.to_engine()),
            Autopilot::engage(AutopilotAction::Goto { target: mark }),
        ));
        info!("loop_goto_standoff: GOTO engaged on '{id}'");
    }
}

/// Both legs have run to completion: the computer disengages itself on
/// arrival, so a hull with no autopilot is a parked one.
#[cfg(feature = "debug")]
fn both_parked() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let mut engaged = 0;
        for (id, autopilot) in world
            .iter_entities()
            .filter_map(|entity| Some((entity.get::<EntityId>()?, entity.get::<Autopilot>())))
        {
            if (id.0 == WARSHIP_ID || id.0 == GUNSHIP_ID) && autopilot.is_some() {
                engaged += 1;
            }
        }
        engaged == 0
    })
}

/// Log where each hull came to rest against the rule it was flown under:
/// centre distance, hull radius, and the gap left between skin and orb face.
#[cfg(feature = "debug")]
fn report_the_park_points(world: &mut World) {
    for id in [WARSHIP_ID, GUNSHIP_ID] {
        let Some(ship) = entity_by_id(world, id) else {
            continue;
        };
        let entity = world.entity(ship);
        // Engine boundary: the avian position and the hull radius are both
        // world units, converted here for the log.
        let centre = entity
            .get::<avian3d::prelude::Position>()
            .map_or(0.0, |position| Meters::from_engine(position.0.length()).0);
        let radius = entity
            .get::<HullRadius>()
            .map_or(0.0, |radius| Meters::from_engine(**radius).0);
        info!(
            "loop_goto_standoff: '{id}' parked {centre:.0} m from the mark's centre, hull radius \
             {radius:.0} m, gap to the orb face {:.0} m (margin {} m)",
            centre - radius - BEACON_RADIUS.0,
            ARRIVAL_MARGIN.0
        );
    }
}

#[cfg(feature = "debug")]
fn standoff_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the set")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("frame the set from above")
        .on_enter(frame_from_above)
        .until(elapsed(0.4))
        .add()
        .step("open the loop and send both hulls")
        .on_enter(|world| {
            loop_start(world, LOOP_NAME);
            send_both_to_the_mark(world);
        })
        .until(elapsed(0.2))
        .add()
        .step("fly both legs to the park")
        .until(both_parked())
        .deadline(LEGS_DEADLINE_SECS)
        .add()
        .step("hold the two park points")
        .on_enter(report_the_park_points)
        .until(elapsed(PARKED_HOLD_SECS))
        .add()
        .step("close the loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(60.0)
        .add()
}
