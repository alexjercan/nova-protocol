//! docking_approach: fly a ported tender onto a moored spar and dock it.
//!
//! The docking range in `systems/` proves the joint. It cannot prove the only
//! thing that matters to a pilot: whether the approach can be FLOWN. This is
//! where that is answered - two hulls, one port each, a hundred and twenty
//! metres apart and deliberately out of square, with the production camera,
//! HUD, drive and RCS.
//!
//! What to do:
//!
//! 1. HOLD the radar key on the spar until the lock takes. The DOCK chip and
//!    the docking sight both need a lock - it is how the game knows which
//!    hull you mean. A tap is the clear, not the lock.
//! 2. fly the sight. Two crosses stand across the two port faces, and the line
//!    between them is the gap. Square the crosses, stand the line up
//!    perpendicular to them, and the pair is lined up. The line and the
//!    crosses turn green as each half of the capture comes good.
//! 3. close the gap. The ticks along the line are one capture distance apart;
//!    when the last one goes, you are inside it.
//! 4. press the dock key. The clamp takes hold and the sleeves reach across.
//!
//! A dock starts neutral: the spar's side flies the pair and your drive, RCS
//! and helm are inert. Press the helm key to fly the pair yourself, and the
//! dock key again to let go.
//!
//! Roll is not assisted and not drawn: the ports are round and the capture
//! ignores how the two hulls are clocked about the axis.
//!
//! Neither hull is shipped content. The tender and the spar are built here
//! from catalog PROTOTYPE sections, so mounting a port on a playable ship
//! costs the base fleet nothing.
//!
//! ```text
//! cargo run --example docking_approach --features debug
//! ```
//!
//! Under `NOVA_AUTOPILOT=1` the script locks the spar, checks that the lock
//! and the sight both came up, and exits - the flying is the human's half.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "docking_approach")]
#[command(version = "1.0.0")]
#[command(about = "Fly a ported tender onto a moored spar and dock it", long_about = None)]
struct Cli;

/// Scenario id of the hull the player flies.
const TENDER_ID: &str = "docking_tender";
/// Scenario id of the hull it docks with.
const SPAR_ID: &str = "docking_spar";

/// Where the spar stands, in meters. Ahead and a little up and across, so the
/// approach starts with a lateral error to fly out rather than a straight
/// line to burn down.
///
/// The distance is chosen against the sight's own draw range: both port faces
/// stand 25 m off their hull's origin, which leaves a face gap of about 70 m -
/// inside the 80 m the sight comes up at, so the instrument is there from the
/// first frame and the whole session is the approach.
const SPAR_POSITION: Meters3 = Meters3::new(20.0, 8.0, -120.0);

/// How far the spar is turned off square. A half turn to face the tender, plus
/// a yaw and a pitch that are the pilot's problem: a spar that arrived already
/// lined up would make the sight decorative.
const SPAR_YAW: f32 = std::f32::consts::PI + 0.28;
const SPAR_PITCH: f32 = -0.16;

/// Seconds the harnessed walk gives the scenario to load.
#[cfg(feature = "debug")]
const LOAD_DEADLINE: f32 = 30.0;

/// Seconds the script holds the radar key. Well past the gesture threshold
/// and the acquisition dwell at this range, so the walk grades the SCENE
/// rather than the timing of a keypress.
#[cfg(feature = "debug")]
const LOCK_HOLD: f32 = 3.0;

/// The frame the capture path writes: the sight as a pilot first sees it,
/// locked and out of square, with the whole approach still to fly.
#[cfg(feature = "debug")]
const APPROACH_SHOT: &str = "docking-approach.png";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(approach_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Run timeline and engine invariants only: an approach a human flies
        // holds no steady-state load worth a frame-time claim.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(approach_script());
        app.add_systems(Startup, hide_dev_overlays);
    }

    app.run()
}

fn approach_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_approach);
}

fn load_approach(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(approach(&game_assets)));
}

/// One prototype section at a build cell, square with the hull.
fn part(id: &str, prototype: &str, cell: Vec3) -> SpaceshipSectionConfig {
    SpaceshipSectionConfig {
        id: id.to_string(),
        position: cell,
        rotation: Quat::IDENTITY,
        source: SectionSource::prototype(prototype),
    }
}

/// The tender: a bow port, a spine, two shoulder plates and a drive.
///
/// The port stands at the very bow with nothing in front of it, which is the
/// port's own clearance rule - its face is what the capture is measured from,
/// and a plate bolted over it would be a hatch that opens into a wall.
fn tender() -> ShipDesign {
    clad(
        vec![
            part(
                "dock_bow",
                DOCKING_PORT_SECTION_ID,
                Vec3::new(0.0, 0.0, -2.0),
            ),
            part("bow", REINFORCED_HULL_SECTION_ID, Vec3::new(0.0, 0.0, -1.0)),
            part("bridge", BASIC_CONTROLLER_SECTION_ID, Vec3::ZERO),
            part("shoulder_port", LIGHT_HULL_SECTION_ID, Vec3::NEG_X),
            part("shoulder_starboard", LIGHT_HULL_SECTION_ID, Vec3::X),
            part("drive", BASIC_THRUSTER_SECTION_ID, Vec3::Z),
        ],
        "industrial",
    )
}

/// The spar: a plain moored stack with one port on its near end and nothing
/// that flies. It is the thing you dock WITH, not a second ship.
fn spar() -> ShipDesign {
    clad(
        vec![
            part(
                "dock_fore",
                DOCKING_PORT_SECTION_ID,
                Vec3::new(0.0, 0.0, -2.0),
            ),
            part(
                "fore",
                REINFORCED_HULL_SECTION_ID,
                Vec3::new(0.0, 0.0, -1.0),
            ),
            part("midships", REINFORCED_HULL_SECTION_ID, Vec3::ZERO),
            part("aft", REINFORCED_HULL_SECTION_ID, Vec3::Z),
            part("mast_high", LIGHT_HULL_SECTION_ID, Vec3::Y),
            part("mast_low", LIGHT_HULL_SECTION_ID, Vec3::NEG_Y),
        ],
        "industrial",
    )
}

/// A hand-built cell list wearing the derived skin, the way every block hull
/// in the fleet is dressed. A design that leaves this off renders as bare
/// cells, which is a look the game ships nowhere.
fn clad(sections: Vec<SpaceshipSectionConfig>, style: &str) -> ShipDesign {
    ShipDesign {
        sections,
        presentation: ShipPresentationConfig {
            skin: true,
            style: Some(style.to_string()),
            ..ShipPresentationConfig::base_voice()
        },
        ..default()
    }
}

fn ship_object(
    id: &str,
    name: &str,
    position: Meters3,
    rotation: Quat,
    controller: SpaceshipController,
    design: ShipDesign,
) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller,
            design: ShipDesignSource::Inline(design),
            ..default()
        }),
    })
}

fn approach(game_assets: &GameAssets) -> ScenarioConfig {
    let tender = ship_object(
        TENDER_ID,
        "Tender",
        Meters3::ZERO,
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig::default()),
        tender(),
    );
    let spar = ship_object(
        SPAR_ID,
        "Mooring Spar",
        SPAR_POSITION,
        Quat::from_rotation_y(SPAR_YAW) * Quat::from_rotation_x(SPAR_PITCH),
        SpaceshipController::None,
        spar(),
    );

    ScenarioConfig {
        description: "A ported tender and the spar it docks with".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![tender, spar],
                ThreePointRig::around("approach", SPAR_POSITION, 8.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "docking_approach".to_string(),
            "Docking Approach".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The lock the sight and the verb both need landed on the spar.
#[cfg(feature = "debug")]
fn check_lock(world: &mut World) {
    let mut ships = world.query_filtered::<&TravelLock, With<PlayerSpaceshipMarker>>();
    let locked = ships
        .iter(world)
        .next()
        .and_then(|lock| lock.0)
        .expect("docking_approach: the radar tap must lock the spar");
    let name = world
        .get::<EntityId>(locked)
        .map(|id| id.0.clone())
        .unwrap_or_default();
    assert_eq!(
        name, SPAR_ID,
        "docking_approach: the tap must lock the spar, not {name}"
    );
}

/// The tender flies with a voice: the lock that just landed beeped, and the
/// RCS the approach is flown on will hiss. A hull built in Rust authors
/// nothing by default, and a mute approach reads as a broken scene.
#[cfg(feature = "debug")]
fn check_voice(world: &mut World) {
    let mut voices = world.query_filtered::<&ShipFeedbackSounds, With<PlayerSpaceshipMarker>>();
    let voice = voices
        .iter(world)
        .next()
        .expect("docking_approach: the tender must carry its feedback sounds");
    assert!(
        voice.lock_on.is_some() && voice.rcs_loop.is_some(),
        "docking_approach: the tender flies mute - author its voice"
    );
}

/// The sight is up: the spar is staged inside the instrument's draw range, so
/// a pilot who locks it is immediately given the line to fly.
#[cfg(feature = "debug")]
fn check_sight(world: &mut World) {
    let mut parts = world.query::<&DockingSightPart>();
    let drawn = parts.iter(world).count();
    assert!(
        drawn > 0,
        "docking_approach: the spar must be staged inside the docking sight's range"
    );
}

/// The harnessed walk: load, lock the spar, and check the pilot has both
/// halves of what the approach needs. The flying itself is not scripted - it
/// is what the example exists for.
#[cfg(feature = "debug")]
fn approach_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the tender and the spar")
        .enter(GameStates::Loading)
        .until(and(state_is(GameStates::Playing), player_ship_present()))
        .deadline(LOAD_DEADLINE)
        .add()
        .step("settle the chase camera")
        .until(elapsed(1.0))
        .add()
        // A lock is a HOLD, not a tap: the gesture latches a slot past its
        // threshold and the acquisition dwell runs on top of that. A tap is
        // the CLEAR.
        .step("hold the radar onto the spar")
        .on_enter(press_action("radar_hold"))
        .until(elapsed(LOCK_HOLD))
        .add()
        .step("release the radar")
        .on_enter(release_action("radar_hold"))
        .until(elapsed(0.5))
        .add()
        .step("check the lock, the voice and the sight")
        .on_enter(check_lock)
        .on_enter(check_voice)
        .on_enter(check_sight)
        .until(elapsed(0.5))
        .add()
        .step("shoot the approach")
        .on_enter(|world: &mut World| shoot(world, APPROACH_SHOT))
        .until(shot_written(APPROACH_SHOT))
        .deadline(LOAD_DEADLINE)
        .add()
}
