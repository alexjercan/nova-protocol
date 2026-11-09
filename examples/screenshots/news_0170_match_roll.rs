//! news_0170_match_roll: the v0.17.0 news flight loop - Q and E roll the
//! ship about its nose, then Shift + X matches the locked hauler's velocity.
//!
//! The set is `screenshot_key_hints`'s: a line warship beside a hauler that
//! coasts on a circular orbit round [`ring::planetoid`], with the hauler as the
//! travel lock. The walk is real player input only, through `press_action`:
//! `roll_right` held, then `roll_left` held, then the MATCH chord
//! (`rcs_modifier` + `autopilot_stop`). The camera is the player's own chase
//! camera, which rolls with the hull, so the roll reads as the planetoid and
//! the hauler turning round the frame.
//!
//! The script checks what the loop is about and panics when it is wrong, so
//! the smoke run is a proof without the webm: each held roll key turns the
//! hull's up axis by at least [`ROLL_MIN_DEG`] about its nose, and the chord
//! engages a MATCH on the hauler that still flies, or completed, when the
//! loop closes.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path, writing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write `news-0170-match-roll.webm`
//!   (staged under `NOVA_CAPTURE_DIR`).
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0170-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0170_match_roll --features debug
//! ```

#[path = "shared/ring.rs"]
mod ring;

use std::sync::Arc;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{nova_debug::harness::Predicate, prelude::*};

#[derive(Parser)]
#[command(name = "news_0170_match_roll")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the v0.17.0 flight loop: Q and E roll the warship, then Shift+X matches a hauler on an orbit",
    long_about = None
)]
struct Cli;

const PLAYER_ID: &str = "match_roll_player";
const TARGET_ID: &str = "match_roll_hauler";

/// The player's start: outside the hauler's orbit, on the hauler's side of
/// the planetoid. The same pose as `screenshot_key_hints`.
const START: Meters3 = Meters3::new(-600.0, 0.0, -4_300.0);
/// The hauler's start: 4 km from the planetoid's centre, where a circular
/// orbit holds.
const TARGET_START: Meters3 = Meters3::new(0.0, 0.0, -4_000.0);

/// Real seconds the scene gets to load on a software rasterizer. An armed run
/// counts them as frames at the loop rate.
const LOAD_SECS: f32 = 300.0;

/// Frames the loop holds before the first key, each roll key is held, the
/// pause between them, and the MATCH is watched.
const OPEN_FRAMES: u32 = 15;
const ROLL_FRAMES: u32 = 50;
const BETWEEN_FRAMES: u32 = 10;
const MATCH_FRAMES: u32 = 75;

/// The least turn of the hull's up axis about its nose that one held roll key
/// must make, in degrees.
const ROLL_MIN_DEG: f32 = 5.0;

/// Real seconds the loop gets to encode once its last frame is in.
const LOOP_CLOSE_SECS: f32 = 60.0;

/// The loop this producer writes.
const LOOP_NAME: &str = "news-0170-match-roll";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    // NovaMenuPlugin explicitly: the interface's clock hold lives in
    // nova_menu, and `with_game_plugins` turns the menu plugin off.
    let mut app = AppBuilder::new()
        .with_game_plugins((custom_plugin, NovaMenuPlugin))
        .build();
    app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
    app.add_plugins(match_roll_script());
    app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    app.init_resource::<RollMark>();
    app.init_resource::<MatchCompleted>();
    app.add_observer(note_a_match);
    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

/// The scene: [`ring::planetoid`]'s well, the player at [`START`] facing the
/// hauler, and the hauler at [`TARGET_START`] on a circular orbit, so the
/// lock has a velocity to match.
fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    let design = |id: &str| {
        ships
            .get_design(&id.into())
            .unwrap_or_else(|| panic!("news_0170_match_roll: unknown ship '{id}'"))
            .design
            .clone()
    };
    let facing = Quat::from_rotation_arc(
        Vec3::NEG_Z,
        (TARGET_START.get() - START.get())
            .try_normalize()
            .expect("news_0170_match_roll: START and TARGET_START must differ"),
    );
    let player = ring::ship(
        PLAYER_ID,
        "Match Roll Player",
        START,
        facing,
        SpaceshipController::Player(PlayerControllerConfig {
            // The walk never fires: nothing here needs a weapon binding.
            input_mapping: std::collections::BTreeMap::new(),
        }),
        None,
        design("line_warship"),
    );
    // Circular orbit speed: v = sqrt(mu / r), in engine units because mu is
    // authored in them, then converted at the scenario edge.
    let radius = TARGET_START.to_engine().length();
    let orbit_speed = (ring::PLANETOID_MASS / radius).sqrt();
    let hauler = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: TARGET_ID.to_string(),
            name: "Orbit Hauler".to_string(),
            position: TARGET_START,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance: Some(Allegiance::Neutral),
            design: ShipDesignSource::Inline(design("light_freighter")),
            initial_velocity: MetersPerSecond3::from_engine(Vec3::X * orbit_speed),
            ..default()
        }),
    });
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "A warship beside a hauler on an orbit, for the roll and MATCH loop."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![ring::planetoid(), player, hauler],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "news_0170_match_roll".to_string(),
            "Match Roll".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// The player hull's up axis when the current roll key went down.
#[derive(Resource, Default)]
struct RollMark(Option<Vec3>);

/// The player hull's rotation.
fn player_rotation(world: &mut World) -> Quat {
    let player = ring::player_root(world).expect("news_0170_match_roll: the player is present");
    world
        .get::<Transform>(player)
        .expect("news_0170_match_roll: the player root has a Transform")
        .rotation
}

/// Mark the hull's up axis, then hold `action`.
fn start_roll(action: &'static str) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let up = player_rotation(world) * Vec3::Y;
        world.resource_mut::<RollMark>().0 = Some(up);
        press_action(action)(world);
    }
}

/// Let `action` up and panic unless the hull's up axis turned at least
/// [`ROLL_MIN_DEG`] about its nose since the key went down.
fn end_roll(action: &'static str) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        release_action(action)(world);
        let rotation = player_rotation(world);
        let nose = rotation * Vec3::NEG_Z;
        let up = rotation * Vec3::Y;
        let before = world
            .resource::<RollMark>()
            .0
            .expect("news_0170_match_roll: the roll was marked");
        // The turn about the nose: both up axes projected onto the plane
        // normal to the current nose.
        let flat = |axis: Vec3| (axis - nose * axis.dot(nose)).normalize_or_zero();
        let turned = flat(before).angle_between(flat(up)).to_degrees();
        assert!(
            turned >= ROLL_MIN_DEG,
            "news_0170_match_roll: '{action}' turned the hull {turned:.1} deg about its nose, under {ROLL_MIN_DEG}"
        );
        info!("match roll: PASS '{action}' rolled the hull {turned:.1} deg");
    }
}

/// True once the player flies a MATCH on its lock.
fn match_engaged() -> Arc<Predicate> {
    Arc::new(|world: &World| {
        world
            .try_query_filtered::<&Autopilot, With<PlayerSpaceshipMarker>>()
            .and_then(|mut query| {
                query.iter(world).next().map(|autopilot| {
                    matches!(autopilot.action, AutopilotAction::MatchTarget { .. })
                })
            })
            .unwrap_or(false)
    })
}

/// The player's MATCH reported complete: the hull holds the hauler's
/// velocity. The scenario tracker drops [`PlayerAutopilotCompleted`] the frame
/// it lands, so an observer records it.
#[derive(Resource, Default)]
struct MatchCompleted(bool);

/// Record a completed MATCH.
fn note_a_match(
    add: On<Add<PlayerAutopilotCompleted>>,
    q_completed: Query<&PlayerAutopilotCompleted>,
    mut done: ResMut<MatchCompleted>,
) {
    if q_completed
        .get(add.entity)
        .is_ok_and(|completed| matches!(completed.action, AutopilotAction::MatchTarget { .. }))
    {
        info!("match roll: the MATCH reported complete");
        done.0 = true;
    }
}

/// True while the player's MATCH flies on, or once it completed.
fn match_flies_or_completed(world: &World) -> bool {
    match_engaged()(world) || world.resource::<MatchCompleted>().0
}

/// The walk: lock the hauler, then, recorded, roll right, roll left and fly
/// the MATCH chord.
fn match_roll_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the scene")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(LOAD_SECS)
        .add()
        .step("show the HUD")
        .on_enter(ring::hud_instrument)
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("lock the hauler")
        .on_enter(|world: &mut World| {
            let player =
                ring::player_root(world).expect("news_0170_match_roll: the player is present");
            let target = ring::entity_by_id(world, TARGET_ID)
                .expect("news_0170_match_roll: the hauler is present");
            world.entity_mut(player).insert(TravelLock(Some(target)));
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        // ---- the recorded flight ----
        .step("open the match-roll loop")
        .on_enter(|world: &mut World| loop_start(world, LOOP_NAME))
        .until(frames(OPEN_FRAMES))
        .add()
        .step("hold E: roll right")
        .on_enter(start_roll("roll_right"))
        .until(frames(ROLL_FRAMES))
        .add()
        .step("let E up")
        .on_enter(end_roll("roll_right"))
        .until(frames(BETWEEN_FRAMES))
        .add()
        .step("hold Q: roll left")
        .on_enter(start_roll("roll_left"))
        .until(frames(ROLL_FRAMES))
        .add()
        .step("let Q up")
        .on_enter(end_roll("roll_left"))
        .until(frames(BETWEEN_FRAMES))
        .add()
        .step("hold the RCS modifier")
        .on_enter(press_action("rcs_modifier"))
        .until(frames(2))
        .add()
        .step("press the STOP key under the modifier: MATCH")
        .on_enter(press_action("autopilot_stop"))
        .until(match_engaged())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("let both keys up")
        .on_enter(|world: &mut World| {
            release_action("autopilot_stop")(world);
            release_action("rcs_modifier")(world);
            info!("match roll: PASS the chord engaged MATCH on the hauler");
        })
        .until(frames(MATCH_FRAMES))
        .add()
        .step("close the match-roll loop")
        .on_enter(|world: &mut World| {
            assert!(
                match_flies_or_completed(world),
                "news_0170_match_roll: the MATCH ended without a match before the loop closed"
            );
            info!("match roll: PASS the MATCH flies on or matched the hauler");
            loop_end(world, LOOP_NAME);
        })
        .until(loop_written(LOOP_NAME))
        .deadline(LOOP_CLOSE_SECS)
        .add()
}
