//! system_mission_hud: the mission a scenario AUTHORS, read back off the
//! pixels it reaches.
//!
//! Task 20260909-213623. Every surface a beat writes to has a live widget on
//! the other side of it, and nothing in the tree checks that the two halves
//! meet. `system_scenario_grammar` proves the objective STATE transitions -
//! that `GameObjectives` gains and loses entries - and `objective_feedback`'s
//! unit tests prove the completion chime. Neither looks at a chip. This range
//! looks at nothing else: it authors one mission beat and reads the rendered
//! result.
//!
//! What one `OnStart` writes, and what this range reads back:
//!
//! - two `Objective`s -> two chips on the top-centre stack, each carrying its
//!   own id and its message upper-cased;
//! - two `ObjectiveMarkerTarget`s -> two world-anchored gold chip layers, one
//!   per marked rock, each standing clear of its target's projection and
//!   carrying the live range to it;
//! - the far mark, deliberately staged off the side of the frame -> its chip
//!   pinned to the viewport edge with its chevron turned toward the rock;
//! - one `NarrativeCue` -> a comms card in the speaker's own voice;
//! - one `HudReadout` bound to a scenario variable -> a strip row rendering
//!   that variable through the authored format, and following it when the
//!   mission moves it;
//! - and the near rock taken out of the world -> its chip layer gone with it
//!   while every other mission surface stands.
//!
//! The scene: one player ship at the origin facing -Z (the objective stack is
//! grown by an `On<Add, PlayerSpaceshipMarker>` observer and the marker chips
//! measure their range from the player, so a mission HUD with no player is a
//! blank one), a rock 600 m dead ahead and a second rock far off the port bow
//! where it cannot project onto the frame.
//!
//! Controls: none needed; fly and look around freely in interactive runs.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_mission_hud --features debug
//! # look for: `mission hud: the stack carries 2 chips`,
//! #           `mission hud: the near chip stands 36.0 px clear ...`,
//! #           `mission hud: the near chip reads 'DERELICT  600 m'`,
//! #           `mission hud: the far chip is pinned at x = 30.0 px ...`,
//! #           `mission hud: the panel is reading MERIDIAN`,
//! #           `mission hud: the strip reads 'SALVAGE 12.3'`,
//! #           `mission hud: the strip followed its variable to 'SALVAGE 87.6'`,
//! #           `mission hud: the near mark took its chip with it`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use std::collections::BTreeMap;

use avian3d::prelude::{AngularVelocity, LinearVelocity};
use bevy::prelude::*;
use clap::Parser;
use nova_authoring::scenario_helpers::prelude::*;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_mission_hud")]
#[command(version = "1.0.0")]
#[command(about = "A test range for the mission surfaces a scenario authors, read off the live HUD. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario this example builds and loads.
const SCENARIO_ID: &str = "mission_hud";

/// The two objectives, with the message each chip must be showing. Authored in
/// sentence case on purpose: the stack upper-cases them, and a chip that
/// echoed the authored string would pass a test written against the authored
/// case.
const OBJECTIVE_CUT: &str = "cut_the_derelict";
const MESSAGE_CUT: &str = "Cut the derelict free";
const OBJECTIVE_HAUL: &str = "haul_the_plate";
const MESSAGE_HAUL: &str = "Haul the plate home";

/// The near mark: dead ahead, inside the frame, and the one taken away at the
/// end of the run.
const NEAR_ID: &str = "rock_ahead";
const NEAR_LABEL: &str = "DERELICT";
const NEAR_AT: Meters3 = Meters3::new(0.0, 0.0, -600.0);

/// The far mark: off the port bow, far enough round that it cannot land on the
/// frame at any window shape this range might be run at, and still ahead of
/// the camera so it projects rather than falling back to the behind-camera
/// direction. Roughly 72 degrees off the nose.
const FAR_ID: &str = "rock_wide";
const FAR_LABEL: &str = "PLATE";
const FAR_AT: Meters3 = Meters3::new(-9000.0, 0.0, -3000.0);

/// Both marks are the same rock, so the two chips differ only in where they
/// are.
const ROCK_RADIUS: Meters = Meters(60.0);

/// The comms line: one speaker, one sentence, held for the whole proof.
///
/// The dwell is the panel's authored maximum. The card's dwell clock starts
/// when it goes up, and the beats below walk the whole HUD before they reach
/// the panel; the shipped default would let the line fade mid-run and turn a
/// real claim into a flaky one.
const SPEAKER: &str = "Meridian";
const LINE: &str = "Cut it loose and bring the plate home.";
const DWELL_SECS: f32 = 30.0;

/// The bound readout: a slot, the variable behind it, the caption, and the
/// value the scenario seeds.
///
/// A fixed number rather than `scenario_elapsed`: the claim is that the strip
/// renders the variable THROUGH the authored format, and a value that moves
/// every frame cannot be compared to a rendered string without inventing a
/// tolerance the format does not have.
const READOUT_SLOT: &str = "salvage";
const READOUT_VARIABLE: &str = "salvage_tonnes";
const READOUT_LABEL: &str = "Salvage";
const READOUT_VALUE: f64 = 12.34;

/// What the mission moves that variable to while the HUD is up.
///
/// A second reading with a different integer part and a different fraction, so
/// a strip that latched the first value, rounded it, or re-rendered a cached
/// string cannot pass the second read by accident.
#[cfg(feature = "debug")]
const READOUT_MOVED_VALUE: f64 = 87.6;

/// Frames the moved variable is given to reach the strip.
///
/// The event world copies each readout's CURRENT value into `HudReadouts` once
/// a frame and the strip reconciles its rows from there, so the new value is
/// at least a frame behind the write. A propagation window and NOT the claim:
/// the beat never looks at the row, which is what the assertion below reads.
#[cfg(feature = "debug")]
const READOUT_SETTLE_FRAMES: u32 = 10;

/// How far (px) a chip's centre may sit from the fresh projection of its
/// target on the axis the stand-off does not push along. The HUD places nodes
/// from last frame's propagated transforms, so one frame of camera motion is
/// expected slack; the same figure `system_hud_indicators` measured 0.0-0.1 px
/// of drift against.
#[cfg(feature = "debug")]
const CENTER_TOLERANCE_PX: f32 = 10.0;

/// The floor on the objective chip's stand-off from its target, mirrored from
/// `hud/objective_markers.rs`'s private `CHIP_CLEARANCE.min_px`. The push is
/// `(silhouette + gap + half the chip).max(this)`, so this is the least a chip
/// may ever float above the thing it points at.
#[cfg(feature = "debug")]
const STANDOFF_FLOOR_PX: f32 = 36.0;

/// What "the camera has stopped" means: a rig moving slower than this in
/// translation and in turn, held for this long. A rig that is still easing
/// moves far faster than either.
///
/// Per SECOND of the clock the ease runs on, not per frame. A frame's worth of
/// easing is a frame's worth of clock, so a per-frame figure says "stopped" at
/// sixty frames a second and "still moving" at six - and on a software
/// rasteriser the beat waited out its whole deadline while the camera sat
/// where it had parked eighty seconds earlier.
#[cfg(feature = "debug")]
const CAMERA_PARKED_UNITS_PER_SEC: f32 = 0.6;
#[cfg(feature = "debug")]
const CAMERA_PARKED_RAD_PER_SEC: f32 = 0.03;
#[cfg(feature = "debug")]
const CAMERA_PARKED_SECS: f32 = 0.5;

/// The rendered frame this range shoots, so the mission surfaces can be looked
/// at rather than only measured. Named once: the beat that waits for the write
/// and the call that makes it must agree on the string.
#[cfg(feature = "debug")]
const MISSION_SHOT: &str = "mission_hud.png";

/// Seconds a beat gets. The scenario is small, but a software rasterizer
/// starting a scene is not, and every beat here waits on a world condition
/// rather than on a clock.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 90.0;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        app.add_plugins(nova_screenshot(mission_script()));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    // On assets-Loaded, not on Playing: `assert_scenario_loaded` checks the
    // load happened by OnEnter(Playing), and loading in that same schedule is
    // an unordered race against the check.
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_mission);
    app.add_systems(Update, park_every_body);
}

/// Hold every body still for the whole run.
///
/// Nothing in this range is about motion, and a mark 70 degrees off the nose is
/// a lever: the frame's own projection of it swings about ten thousand px per
/// radian of camera rotation, so a hull still drifting onto its rig moves the
/// mark across the screen faster than any widget could be measured against it.
/// A parked scene is what makes the chip readings comparisons rather than
/// races. That the rig settles at all is the camera layer's claim, not this
/// range's.
fn park_every_body(mut bodies: Query<(&mut LinearVelocity, &mut AngularVelocity)>) {
    for (mut linear, mut angular) in &mut bodies {
        if **linear != Vec3::ZERO {
            **linear = Vec3::ZERO;
        }
        if **angular != Vec3::ZERO {
            **angular = Vec3::ZERO;
        }
    }
}

fn setup_mission(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
) {
    commands.trigger(LoadScenario(mission(&game_assets, &sections)));
}

/// The mission: one player, two marked rocks, and one `OnStart` that writes
/// every mission surface the HUD owns.
fn mission(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    // The player: the smallest hull that flies. `PlayerSpaceshipMarker` is
    // what grows the objective stack and what the marker chips measure their
    // range from, so the controller is the fixture, not a convenience.
    let player = fixtures::ship(
        sections,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::new(),
            speed_cap: None,
        }),
        &[
            SectionSpec::new("helm", "basic_controller_section", Vec3::ZERO),
            SectionSpec::new("plate", "reinforced_hull_section", Vec3::new(0.0, 0.0, 1.0)),
        ],
    );

    let mut start_actions = vec![
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "cutter".to_string(),
                name: "Cutter".to_string(),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(player),
        }),
        EventActionConfig::SpawnScenarioObject(fixtures::asteroid(
            game_assets,
            NEAR_ID,
            "Derelict",
            NEAR_AT,
            ROCK_RADIUS,
            // Unsigned: nothing here radar-locks, and a signature would put a
            // second chip family on the same rock.
            None,
        )),
        EventActionConfig::SpawnScenarioObject(fixtures::asteroid(
            game_assets,
            FAR_ID,
            "Plate",
            FAR_AT,
            ROCK_RADIUS,
            None,
        )),
        post_objective(OBJECTIVE_CUT, MESSAGE_CUT),
        post_objective(OBJECTIVE_HAUL, MESSAGE_HAUL),
        attach_objective_marker(NEAR_ID, NEAR_LABEL),
        attach_objective_marker(FAR_ID, FAR_LABEL),
        EventActionConfig::NarrativeCue(NarrativeCueActionConfig {
            accent: default_comms_accent(),
            speaker: SPEAKER.to_string(),
            text: LINE.to_string(),
            dwell: Some(DWELL_SECS),
            icon: None,
        }),
        EventActionConfig::HudReadout(HudReadoutActionConfig {
            slot: READOUT_SLOT.to_string(),
            variable: READOUT_VARIABLE.to_string(),
            format: HudReadoutFormatConfig::Number,
            label: Some(READOUT_LABEL.to_string()),
            visible: true,
        }),
        set_number(READOUT_VARIABLE, READOUT_VALUE),
    ];
    // The engine spawns no light: a scenario that authors none renders black,
    // and this range is read off a rendered frame as well as off the tree.
    start_actions.extend(ThreePointRig::around("mission", Meters3::ZERO, 60.0).actions());

    ScenarioConfig {
        description: "The mission surfaces a scenario authors, read off the live HUD".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: start_actions,
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Mission HUD".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

// -- the script --

/// The run: load the mission, then walk every surface it wrote. The comms card
/// is read first of the rendered claims because it is the only one on a dwell
/// clock; everything else stands until the scenario is torn down.
#[cfg(feature = "debug")]
fn mission_script() -> Script {
    Script::new()
        .step("load the mission")
        .enter(GameStates::Loading)
        .until(and(
            player_ship_present(),
            scenario_variable_is(READOUT_VARIABLE, READOUT_VALUE),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("wait for the HUD to take delivery")
        .until(the_mission_is_on_screen())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // The chase camera eases onto its rig, and every projection below is
        // taken through it. A mark 70 degrees off the nose swings thousands of
        // px across the frame for a degree of camera yaw, so the marks are read
        // only once the camera has stopped moving.
        .step("let the camera come to rest")
        .until(the_camera_is_parked())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("keep the frame for a human to look at")
        .on_enter(|world: &mut World| shoot(world, MISSION_SHOT))
        .until(shot_written(MISSION_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("read the comms panel")
        .on_enter(assert_the_cue_reached_the_panel)
        .add()
        .step("read the objective stack")
        .on_enter(assert_the_objectives_reached_the_stack)
        .add()
        .step("read the readout strip")
        .on_enter(assert_the_readout_renders_its_variable)
        .add()
        .step("move the variable under the readout")
        .on_enter(move_the_readout_variable)
        .until(and(
            scenario_variable_is(READOUT_VARIABLE, READOUT_MOVED_VALUE),
            frames(READOUT_SETTLE_FRAMES),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("read the readout strip again")
        .on_enter(assert_the_readout_followed_its_variable)
        .add()
        .step("read the near mark's chip")
        .on_enter(assert_the_near_chip_rides_its_target)
        .add()
        .step("read the far mark's chip")
        .on_enter(assert_the_far_chip_pins_to_the_edge)
        .add()
        .step("take the near mark out of the world")
        .on_enter(take_the_near_mark_away)
        .until(one_mark_is_left())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("assert the chip left with its target")
        .on_enter(assert_the_chip_left_with_its_target)
        .add()
}

/// Every mission surface has reached the screen: both marker chips are up, the
/// comms card is showing its line and the readout strip has grown its row.
///
/// Deliberately NOT the comparison any assertion below makes - it waits for the
/// widgets to EXIST, and the assertions decide whether they are right.
#[cfg(feature = "debug")]
fn the_mission_is_on_screen() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let chips = world
            .try_query_filtered::<&AnchoredChipTarget, With<ObjectiveMarkerChipHudMarker>>()
            .map_or(0, |mut query| query.iter(world).count());
        let carded = world
            .try_query::<&Text>()
            .is_some_and(|mut query| query.iter(world).any(|text| ***text == *LINE));
        let rowed = world.try_query::<&Name>().is_some_and(|mut query| {
            query
                .iter(world)
                .any(|name| name.as_str() == readout_row_name())
        });
        chips == 2 && carded && rowed
    })
}

/// The screen-indicator camera has stopped moving: its pose this frame is the
/// pose it will project through.
///
/// Sampled against the run's own previous reading rather than against a target
/// pose - where the rig ends up is the camera's business, and this beat only
/// needs it to have got there.
#[cfg(feature = "debug")]
fn the_camera_is_parked() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    let watch = std::sync::Mutex::new((Option::<(Vec3, Quat)>::None, 0.0_f32));
    std::sync::Arc::new(move |world: &World| {
        let Some(pose) = world
            .try_query_filtered::<&GlobalTransform, With<ScreenIndicatorCamera>>()
            .and_then(|mut query| query.iter(world).next().map(GlobalTransform::to_isometry))
        else {
            return false;
        };
        // The ease runs on the game clock, so the rate is read on it too: a
        // frame that advanced a quarter of a second of world moved a quarter of
        // a second's worth of camera, however long it took to draw.
        let dt = world.resource::<Time<Virtual>>().delta_secs();
        if dt <= 0.0 {
            return false;
        }
        let now = (Vec3::from(pose.translation), pose.rotation);
        let (last, still) = &mut *watch.lock().expect("mission hud: camera watch poisoned");
        let parked = last.is_some_and(|(at, facing)| {
            at.distance(now.0) / dt < CAMERA_PARKED_UNITS_PER_SEC
                && facing.angle_between(now.1) / dt < CAMERA_PARKED_RAD_PER_SEC
        });
        *last = Some(now);
        *still = if parked { *still + dt } else { 0.0 };
        *still >= CAMERA_PARKED_SECS
    })
}

/// Exactly one objective marker chip layer is left.
///
/// Stated in the positive rather than as `not(...)`: the harness predicate
/// combinator and Bevy's run-condition of the same name are both in scope here,
/// and the run-condition is the one the prelude wins with.
#[cfg(feature = "debug")]
fn one_mark_is_left() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&AnchoredChipTarget, With<ObjectiveMarkerChipHudMarker>>()
            .is_some_and(|mut query| query.iter(world).count() == 1)
    })
}

// -- the assertions --

/// The comms card: the authored line, in the authored speaker's voice.
///
/// The card's own marker components are private, so the panel is read the way
/// a screenshot reads it - every `Text` hanging under the entity the HUD names
/// `CommsPanelHUD`. The header is the claim: the speaker is authored in
/// sentence case and the panel renders it upper-cased, so the upper-cased form
/// can only be on screen because the panel put it there.
#[cfg(feature = "debug")]
fn assert_the_cue_reached_the_panel(world: &mut World) {
    let shown = comms_panel_text(world);
    let header = SPEAKER.to_uppercase();
    assert!(
        shown.contains(&header),
        "mission hud: the comms panel should be headed '{header}', showing {shown:?}"
    );
    assert!(
        shown.iter().any(|line| line == LINE),
        "mission hud: the comms panel should be reading '{LINE}', showing {shown:?}"
    );
    // The log behind the panel carries the same line: the panel is a view of
    // `StoryFeed`, and a card with no line behind it would be a leak.
    let logged = world
        .resource::<StoryFeed>()
        .0
        .iter()
        .any(|line| line.speaker == SPEAKER && line.text == LINE);
    assert!(
        logged,
        "mission hud: the story feed should carry the line the panel is showing"
    );
    info!("mission hud: the panel is reading {header}");
    nova_probe::probe_marker(
        world,
        "outcome: a comms cue reaches the panel in the speaker's own voice",
        serde_json::json!({ "header": header, "body": LINE, "shown": shown }),
    );
}

/// The objective stack: one chip per posted objective, each carrying its own
/// id and showing its own message upper-cased.
#[cfg(feature = "debug")]
fn assert_the_objectives_reached_the_stack(world: &mut World) {
    let chips = stack_chips(world);
    assert_eq!(
        chips.len(),
        2,
        "mission hud: the stack should carry one chip per posted objective, carrying {chips:?}"
    );
    let mut read: Vec<(String, String)> = Vec::new();
    for (id, message) in [(OBJECTIVE_CUT, MESSAGE_CUT), (OBJECTIVE_HAUL, MESSAGE_HAUL)] {
        let shown = chips
            .iter()
            .find(|(chip, _)| chip == id)
            .map(|(_, text)| text.clone())
            .unwrap_or_else(|| {
                panic!("mission hud: no stack chip carries objective '{id}', carrying {chips:?}")
            });
        assert_eq!(
            shown,
            message.to_uppercase(),
            "mission hud: objective '{id}' should read as its message upper-cased"
        );
        read.push((id.to_string(), shown));
    }
    info!("mission hud: the stack carries {} chips", chips.len());
    nova_probe::probe_marker(
        world,
        "outcome: a posted objective reaches the stack in its own chip",
        serde_json::json!({ "chips": read }),
    );
}

/// The readout strip: the bound variable, rendered through the authored format
/// under the authored caption.
///
/// The expectation is built from the EVENT WORLD's variable, not from the
/// HUD's own copy of it - that is the seam the action claims to close, and a
/// comparison against `HudReadouts` would only restate the strip's input.
#[cfg(feature = "debug")]
fn assert_the_readout_renders_its_variable(world: &mut World) {
    let value = match world
        .resource::<NovaEventWorld>()
        .get_variable(READOUT_VARIABLE)
    {
        Some(VariableLiteral::Number(value)) => *value,
        other => panic!("mission hud: '{READOUT_VARIABLE}' should be a number, got {other:?}"),
    };
    let expected = expected_readout_row(value);
    let shown = readout_row_text(world);
    assert_eq!(
        shown, expected,
        "mission hud: the '{READOUT_SLOT}' row should render {value} through the authored format"
    );
    info!("mission hud: the strip reads '{shown}'");
    nova_probe::probe_marker(
        world,
        "outcome: a bound readout renders its variable in the authored format",
        serde_json::json!({ "variable": READOUT_VARIABLE, "value": value, "shown": shown }),
    );
}

/// What the strip should be reading for `value`: the authored caption, upper
/// cased by the strip, and the number through the authored format.
#[cfg(feature = "debug")]
fn expected_readout_row(value: f64) -> String {
    format!(
        "{} {}",
        READOUT_LABEL.to_uppercase(),
        HudReadoutFormat::Number.render(value)
    )
}

/// Move the bound variable the way a mission does.
///
/// Written straight into the event world's variable table because that is
/// where the authored action ends: `VariableSetActionConfig::action` evaluates
/// its expression and calls `insert_variable`, so a scripted `set_number` and
/// this write are the same write. What the range is proving is the SYNC below
/// it - the per-frame copy into `HudReadouts` and the row that renders it -
/// not the expression evaluator, which has its own tests.
#[cfg(feature = "debug")]
fn move_the_readout_variable(world: &mut World) {
    world.resource_mut::<NovaEventWorld>().insert_variable(
        READOUT_VARIABLE.to_string(),
        VariableLiteral::Number(READOUT_MOVED_VALUE),
    );
    info!("mission hud: the mission moved '{READOUT_VARIABLE}' to {READOUT_MOVED_VALUE}");
}

/// The row followed the variable: the strip is a live view of mission state
/// and not a snapshot taken when the readout was posted.
///
/// Read against the value the EVENT WORLD holds now, for the same reason the
/// first read is: the strip's input is the variable, and a comparison against
/// the HUD's own copy would only restate it.
#[cfg(feature = "debug")]
fn assert_the_readout_followed_its_variable(world: &mut World) {
    let value = match world
        .resource::<NovaEventWorld>()
        .get_variable(READOUT_VARIABLE)
    {
        Some(VariableLiteral::Number(value)) => *value,
        other => panic!("mission hud: '{READOUT_VARIABLE}' should be a number, got {other:?}"),
    };
    assert!(
        (value - READOUT_MOVED_VALUE).abs() < f64::EPSILON,
        "mission hud: the mission should have moved '{READOUT_VARIABLE}' to \
         {READOUT_MOVED_VALUE}, and it reads {value}"
    );
    let expected = expected_readout_row(value);
    let shown = readout_row_text(world);
    assert_eq!(
        shown, expected,
        "mission hud: the '{READOUT_SLOT}' row should have followed its variable to {value}, \
         and it still reads '{shown}'"
    );
    info!("mission hud: the strip followed its variable to '{shown}'");
    nova_probe::probe_marker(
        world,
        "outcome: a moved variable moves the readout with it",
        serde_json::json!({
            "variable": READOUT_VARIABLE,
            "from": READOUT_VALUE,
            "to": value,
            "shown": shown,
        }),
    );
}

/// The near mark's chip: standing clear of its target's projection, and
/// carrying the live range to it.
///
/// Two claims, because the chip does two jobs. WHERE it sits is the widget's:
/// the chip is pushed straight up off the target's silhouette, so its centre
/// shares the projection's x and floats at least the clearance floor above it -
/// a chip sitting ON the rock would be unreadable, and one that had drifted off
/// the x axis would be pointing at nothing. WHAT it says is the chip family's:
/// the label the beat authored, then the range the player is at right now.
#[cfg(feature = "debug")]
fn assert_the_near_chip_rides_its_target(world: &mut World) {
    let target = scoped_entity(world, NEAR_ID);
    let layer = chip_layer_for(world, target);
    let pill = chip_pill(world, layer);
    let (center, _) = chip_box(world, pill);
    let anchor = world
        .get::<GlobalTransform>(target)
        .expect("mission hud: the near mark has no GlobalTransform")
        .translation();
    let projected = project_through_indicator_camera(world, anchor);

    let drift = (center.x - projected.x).abs();
    let standoff = projected.y - center.y;
    assert!(
        drift <= CENTER_TOLERANCE_PX,
        "mission hud: the near chip drifted {drift:.1} px off its target's column"
    );
    assert!(
        standoff >= STANDOFF_FLOOR_PX,
        "mission hud: the near chip stands only {standoff:.1} px clear of its target, \
         under the {STANDOFF_FLOOR_PX} px floor"
    );
    info!("mission hud: the near chip stands {standoff:.1} px clear of its target");
    nova_probe::probe_marker(
        world,
        "outcome: the marker chip stands off its target's projection",
        serde_json::json!({
            "chip_center_px": [center.x, center.y],
            "projected_px": [projected.x, projected.y],
            "column_drift_px": drift,
            "standoff_px": standoff,
        }),
    );

    let player = world
        .try_query_filtered::<&GlobalTransform, With<PlayerSpaceshipMarker>>()
        .and_then(|mut query| query.iter(world).next().map(GlobalTransform::translation))
        .expect("mission hud: no player ship to measure the range from");
    let range = Meters::from_engine(player.distance(anchor));
    let expected = format!("{NEAR_LABEL}  {}", nova_ui::units::distance(range));
    let shown = chip_label_text(world, pill);
    assert_eq!(
        shown, expected,
        "mission hud: the near chip should read its label and the live range"
    );
    info!("mission hud: the near chip reads '{shown}'");
    nova_probe::probe_marker(
        world,
        "outcome: the marker chip carries the live range to its target",
        serde_json::json!({ "range_m": range.get(), "shown": shown }),
    );
}

/// The far mark's chip: pinned to the frame's edge with its chevron turned
/// toward the rock it cannot reach.
///
/// The angular tolerance is MEASURED, not chosen. The chevron is aimed from the
/// pushed-up chip rather than from the bare projection, so the two directions
/// can only disagree by the angle that push subtends at the projection's
/// distance from the frame's centre. The matched reference for the push is this
/// same run's NEAR chip, read in this same beat: the stand-off is
/// `(silhouette + gap + half the chip).max(floor)`, the two marks are the same
/// 60 m rock wearing chips of the same height, and the near one is the closer
/// of the two - so it carries the larger silhouette and the larger push, and no
/// chip in this frame is pushed further than it is.
#[cfg(feature = "debug")]
fn assert_the_far_chip_pins_to_the_edge(world: &mut World) {
    let target = scoped_entity(world, FAR_ID);
    let layer = chip_layer_for(world, target);
    let pill = chip_pill(world, layer);
    let (center, size) = chip_box(world, pill);
    let anchor = world
        .get::<GlobalTransform>(target)
        .expect("mission hud: the far mark has no GlobalTransform")
        .translation();
    let projected = project_through_indicator_camera(world, anchor);
    let viewport = indicator_viewport(world);

    assert!(
        projected.x < 0.0,
        "mission hud: the far mark should project off the left of the frame, at x = {:.1}",
        projected.x
    );
    let left = center.x - size.x / 2.0;
    assert!(
        (left - CHIP_EDGE_MARGIN_PX).abs() <= 1.0,
        "mission hud: the far chip's near edge should be pinned at the {CHIP_EDGE_MARGIN_PX} px \
         margin, drawn at {left:.1} px"
    );
    assert!(
        center.y - size.y / 2.0 >= CHIP_EDGE_MARGIN_PX
            && center.y + size.y / 2.0 <= viewport.y - CHIP_EDGE_MARGIN_PX,
        "mission hud: the far chip's whole box should stay inside the margin, \
         drawn at {center:?} sized {size:?} in a {viewport:?} frame"
    );

    let (rotation, visibility) = chip_chevron(world, pill);
    assert_ne!(
        visibility,
        Visibility::Hidden,
        "mission hud: a clamped chip's chevron is the only thing saying which way its mark is"
    );
    let from_center = projected - viewport / 2.0;
    let wanted = from_center.x.atan2(-from_center.y);
    let reference = near_chip_standoff(world);
    let tolerance = (reference / from_center.length()).atan();
    let off = angle_between(rotation, wanted);
    info!(
        "mission hud: the far mark projects {:.0} px off centre, chevron {off:.4} rad off the \
         bearing ({:.0} px of aim-point offset) against the {tolerance:.4} rad the near chip's \
         {reference:.1} px stand-off accounts for",
        from_center.length(),
        from_center.length() * off.tan()
    );
    assert!(
        off <= tolerance,
        "mission hud: the far chip's chevron is {off:.4} rad off the bearing to its mark, \
         outside the {tolerance:.4} rad the stand-off can account for"
    );
    info!(
        "mission hud: the far chip is pinned at x = {left:.1} px, chevron {off:.4} rad off the mark"
    );
    nova_probe::probe_marker(
        world,
        "outcome: an off-screen mark pins to the edge and points home",
        serde_json::json!({
            "projected_px": [projected.x, projected.y],
            "chip_left_px": left,
            "margin_px": CHIP_EDGE_MARGIN_PX,
            "chevron_error_rad": off,
            "chevron_tolerance_rad": tolerance,
            "reference_standoff_px": reference,
        }),
    );
}

/// How far the NEAR mark's chip is pushed off its target's projection, right
/// now, in this frame.
///
/// The matched reference the far chip's chevron is graded against. Both chips
/// are the same widget over the same 60 m rock, so the push differs only by the
/// silhouette term, which shrinks with range - the nearer mark is pushed the
/// furthest of the two.
#[cfg(feature = "debug")]
fn near_chip_standoff(world: &mut World) -> f32 {
    let target = scoped_entity(world, NEAR_ID);
    let anchor = world
        .get::<GlobalTransform>(target)
        .expect("mission hud: the near mark has no GlobalTransform")
        .translation();
    let projected = project_through_indicator_camera(world, anchor);
    let layer = chip_layer_for(world, target);
    let pill = chip_pill(world, layer);
    let (center, _) = chip_box(world, pill);
    projected.y - center.y
}

/// Take the near mark out of the world, the way anything that dies leaves it.
#[cfg(feature = "debug")]
fn take_the_near_mark_away(world: &mut World) {
    let target = scoped_entity(world, NEAR_ID);
    world.entity_mut(target).despawn();
}

/// The chip went with its target, and nothing else did.
///
/// The shared despawn observer takes down only the layers aimed at the entity
/// that left, so the other mark's chip is the control: one family, two chips,
/// one removal. The stack and the strip are untouched because neither is a
/// world-anchored widget - an objective is mission state, not a waypoint.
#[cfg(feature = "debug")]
fn assert_the_chip_left_with_its_target(world: &mut World) {
    let far = scoped_entity(world, FAR_ID);
    let aimed: Vec<Entity> = world
        .try_query_filtered::<&AnchoredChipTarget, With<ObjectiveMarkerChipHudMarker>>()
        .map(|mut query| query.iter(world).map(|chip| **chip).collect())
        .unwrap_or_default();
    assert_eq!(
        aimed,
        vec![far],
        "mission hud: only the surviving mark's chip should be left"
    );
    let chips = stack_chips(world);
    assert_eq!(
        chips.len(),
        2,
        "mission hud: losing a marked rock must not disturb the objective stack, carrying {chips:?}"
    );
    info!("mission hud: the near mark took its chip with it");
    nova_probe::probe_marker(
        world,
        "outcome: the marker chip leaves with its target and the stack stands",
        serde_json::json!({ "chips_left": aimed.len(), "stack_chips": chips.len() }),
    );
}

// -- reading the HUD --

/// The scenario-scoped entity carrying `id`. Mandatory: every id this range
/// looks up is one its own `OnStart` spawned.
#[cfg(feature = "debug")]
fn scoped_entity(world: &mut World, id: &str) -> Entity {
    world
        .query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>()
        .iter(world)
        .find(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("mission hud: no scoped object called '{id}'"))
}

/// The objective marker chip LAYER aimed at `target`. The layer is what carries
/// both the family marker and the target, so one lookup answers "whose chip"
/// and "which family".
#[cfg(feature = "debug")]
fn chip_layer_for(world: &mut World, target: Entity) -> Entity {
    world
        .query_filtered::<(Entity, &AnchoredChipTarget), With<ObjectiveMarkerChipHudMarker>>()
        .iter(world)
        .find(|(_, chip)| ***chip == target)
        .map(|(layer, _)| layer)
        .unwrap_or_else(|| panic!("mission hud: no objective marker chip aimed at {target:?}"))
}

/// The pill under a chip layer: the one node the indicator widget positions.
#[cfg(feature = "debug")]
fn chip_pill(world: &mut World, layer: Entity) -> Entity {
    descendants(world, layer)
        .into_iter()
        .find(|entity| world.get::<AnchoredChipNodeMarker>(*entity).is_some())
        .unwrap_or_else(|| panic!("mission hud: chip layer {layer:?} has no pill"))
}

/// A placed chip's centre and size, both in LOGICAL px.
///
/// `ComputedNode::size` is PHYSICAL and `Node::left`/`top` are logical, so the
/// size is divided back the way the widget's own `Content` branch does. Reading
/// the size off `Node` instead would not work at all here: a `Content` chip
/// leaves its width and height at `Val::Auto` and lets taffy measure it.
#[cfg(feature = "debug")]
fn chip_box(world: &mut World, pill: Entity) -> (Vec2, Vec2) {
    let node = world
        .get::<Node>(pill)
        .expect("mission hud: the chip pill has no Node");
    let px = |val: Val, name: &str| match val {
        Val::Px(px) => px,
        other => panic!("mission hud: the chip pill's {name} is {other:?}, expected Val::Px"),
    };
    let corner = Vec2::new(px(node.left, "left"), px(node.top, "top"));
    let computed = world
        .get::<bevy::ui::ComputedNode>(pill)
        .expect("mission hud: the chip pill never reached UI layout");
    let size = computed.size() * computed.inverse_scale_factor();
    (corner + size / 2.0, size)
}

/// The chip's label text: the leaf child the pill grows around.
#[cfg(feature = "debug")]
fn chip_label_text(world: &mut World, pill: Entity) -> String {
    descendants(world, pill)
        .into_iter()
        .find(|entity| world.get::<AnchoredChipLabelMarker>(*entity).is_some())
        .and_then(|entity| world.get::<Text>(entity))
        .map(|text| text.0.clone())
        .unwrap_or_else(|| panic!("mission hud: chip pill {pill:?} has no label"))
}

/// The chip's edge chevron: the rotation the widget wrote and whether it is
/// being drawn.
#[cfg(feature = "debug")]
fn chip_chevron(world: &mut World, pill: Entity) -> (f32, Visibility) {
    descendants(world, pill)
        .into_iter()
        .find(|entity| world.get::<ScreenIndicatorArrowMarker>(*entity).is_some())
        .map(|entity| {
            let rotation = world
                .get::<UiTransform>(entity)
                .expect("mission hud: the chevron has no UiTransform")
                .rotation
                .as_radians();
            let visibility = world
                .get::<Visibility>(entity)
                .copied()
                .expect("mission hud: the chevron has no Visibility");
            (rotation, visibility)
        })
        .unwrap_or_else(|| panic!("mission hud: chip pill {pill:?} has no chevron"))
}

/// Every objective stack chip, as (objective id, the text it is showing).
#[cfg(feature = "debug")]
fn stack_chips(world: &mut World) -> Vec<(String, String)> {
    let chips: Vec<(Entity, String)> = world
        .query::<(Entity, &ObjectiveStackChip)>()
        .iter(world)
        .map(|(entity, chip)| (entity, (**chip).clone()))
        .collect();
    let mut read: Vec<(String, String)> = chips
        .into_iter()
        .map(|(entity, id)| {
            let text = descendants(world, entity)
                .into_iter()
                .find_map(|child| world.get::<Text>(child).map(|text| text.0.clone()))
                .unwrap_or_else(|| panic!("mission hud: stack chip '{id}' has no label"));
            (id, text)
        })
        .collect();
    read.sort();
    read
}

/// The name the readout strip gives the row bound to this range's slot.
#[cfg(feature = "debug")]
fn readout_row_name() -> String {
    format!("HudReadoutRow({READOUT_SLOT})")
}

/// The text of the readout row bound to this range's slot.
#[cfg(feature = "debug")]
fn readout_row_text(world: &mut World) -> String {
    let wanted = readout_row_name();
    world
        .query::<(&Name, &Text)>()
        .iter(world)
        .find(|(name, _)| name.as_str() == wanted)
        .map(|(_, text)| text.0.clone())
        .unwrap_or_else(|| panic!("mission hud: no readout row called '{wanted}'"))
}

/// Every `Text` hanging under the comms panel, in tree order.
///
/// The card's own markers are `pub(crate)`, so the panel is read from its
/// display `Name` down - the same handle a screenshot has.
#[cfg(feature = "debug")]
fn comms_panel_text(world: &mut World) -> Vec<String> {
    let panel = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, name)| name.as_str() == "CommsPanelHUD")
        .map(|(entity, _)| entity)
        .expect("mission hud: the comms panel is not in the tree");
    descendants(world, panel)
        .into_iter()
        .filter_map(|entity| world.get::<Text>(entity).map(|text| text.0.clone()))
        .collect()
}

/// Every descendant of `root`, breadth first, excluding `root` itself.
#[cfg(feature = "debug")]
fn descendants(world: &mut World, root: Entity) -> Vec<Entity> {
    let mut found = Vec::new();
    let mut frontier = vec![root];
    while let Some(current) = frontier.pop() {
        let Some(children) = world.get::<Children>(current) else {
            continue;
        };
        let children: Vec<Entity> = children.iter().collect();
        found.extend(children.iter().copied());
        frontier.extend(children);
    }
    found
}

/// Fresh projection of `world_pos` through the screen-indicator camera. The
/// camera is mandatory: without one every indicator hides, so a missing one
/// must fail the run rather than skip the claim.
#[cfg(feature = "debug")]
fn project_through_indicator_camera(world: &mut World, world_pos: Vec3) -> Vec2 {
    let (camera_transform, camera) = world
        .query_filtered::<(&GlobalTransform, &Camera), With<ScreenIndicatorCamera>>()
        .iter(world)
        .next()
        .expect("mission hud: no ScreenIndicatorCamera - the camera glue observer failed");
    camera
        .world_to_viewport(camera_transform, world_pos)
        .expect("mission hud: the mark does not project - it is behind the camera")
}

/// The frame the indicators are placed in, in logical px.
#[cfg(feature = "debug")]
fn indicator_viewport(world: &mut World) -> Vec2 {
    world
        .query_filtered::<&Camera, With<ScreenIndicatorCamera>>()
        .iter(world)
        .next()
        .and_then(Camera::logical_viewport_size)
        .expect("mission hud: the indicator camera has no viewport")
}

/// The unsigned angle between two headings, in radians, the short way round.
#[cfg(feature = "debug")]
fn angle_between(a: f32, b: f32) -> f32 {
    let delta = (a - b).rem_euclid(std::f32::consts::TAU);
    delta.min(std::f32::consts::TAU - delta)
}
