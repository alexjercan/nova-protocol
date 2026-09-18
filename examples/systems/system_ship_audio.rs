//! system_ship_audio: what the ship's soundtrack ASKS the engine for, read off
//! the live voices while the run is muted.
//!
//! The audio layer has focused unit coverage for routing, the rolloff, the pan
//! law, the throttle and the hum curve, and every one of those tests builds its
//! own two-entity `World`. None of them asks what a real hull, a real camera
//! and a real magazine produce together. This range asks exactly that and
//! nothing else.
//!
//! The subject is the REQUEST, not the sound. A probe run is
//! [`HarnessMute`]d, and mute masks the output gain only - it changes no route,
//! no placement, no level a cue asks for and no voice that gets spawned - so
//! every claim below is an ECS reading and none of them needs a device. The
//! range asserts that mute is on before it asserts anything else.
//!
//! Two hulls, and the difference between them is the whole routing model: one
//! is the player's and one is not.
//!
//! - One frame rakes both hulls with several hits each. Each hull's burst
//!   collapses to ONE report, because impacts are throttled per struck BODY,
//!   and two hulls hit in the same frame are two reports rather than one.
//! - The two reports carry the SAME authored sound and different routes: the
//!   player's own plate is heard through the hull, the bogey's across the gap.
//! - The exterior one is parked on the listener's own emitter sphere, on the
//!   true bearing to the contact, which is where the engine hears a placed cue
//!   from.
//! - The player's gun runs its magazine dry under a held trigger. The dead
//!   trigger clicks ONCE; holding it clicks no more; letting go and pulling
//!   again clicks again.
//! - The drive's hum tracks the throttle: two settled levels in the ratio the
//!   throttles were in, and a released throttle retires the voice outright.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_ship_audio --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `ship audio: the burst read as 2 reports`,
//! #           `ship audio: the exterior report is parked ...`,
//! #           `ship audio: the dead trigger clicked 2 times over 2 pulls`,
//! #           `ship audio: the hum settled at ... and ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use std::collections::BTreeMap;

use bevy::prelude::*;
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_ship_audio")]
#[command(version = "1.0.0")]
#[command(about = "A test range for the ship's soundtrack, read off the live voices under a muted harness. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario the range loads under.
const SCENARIO_ID: &str = "ship_audio";

/// The player's hull, and its four section slots.
///
/// Section ids are globally unique across both ships on purpose: the range
/// looks sections up by [`EntityId`], and two hulls sharing a slot name would
/// make that lookup answer whichever one the query reached first.
const PILOT_ID: &str = "pilot";
const PILOT_HELM: &str = "pilot_helm";
const PILOT_DRIVE: &str = "pilot_drive";
const PILOT_GUN: &str = "pilot_gun";
const PILOT_PLATE: &str = "pilot_plate";

/// The other hull: same parts, no pilot. It exists to be NOT the player, which
/// is the only input the routing decision takes.
const BOGEY_ID: &str = "bogey";
const BOGEY_HELM: &str = "bogey_helm";
const BOGEY_PLATE_A: &str = "bogey_plate_a";
const BOGEY_PLATE_B: &str = "bogey_plate_b";

/// Where the bogey sits.
///
/// Off the bow rather than dead ahead, so the bearing to it has a real left/right
/// component and the emitter reading below is a direction rather than a sign.
/// Close enough that the rolloff leaves the report well over
/// `SFX_AUDIBLE_THRESHOLD` - a cue the engine judges inaudible is never given a
/// voice at all, which would make the burst claim vacuous.
const BOGEY_AT: Meters3 = Meters3::new(400.0, 0.0, -200.0);

/// The authored voice each claim reads, matched on the tail of the resolved
/// asset path.
///
/// Suffixes rather than whole paths: the content authors them as `self://`
/// refs against the base mod, and what this range is about is WHICH sound the
/// layer picked, not how the mod root spells itself. No two of these are
/// suffixes of each other.
#[cfg(feature = "debug")]
const IMPACT_SOUND: &str = "impact.wav";
#[cfg(feature = "debug")]
const TURRET_FIRE_SOUND: &str = "turret_fire.wav";
#[cfg(feature = "debug")]
const DRY_FIRE_SOUND: &str = "dry_fire.wav";

/// The `Name` the loop pass gives the drive's hum voice. The marker component
/// behind it is private to `nova_ship`, and the name is the layer's own public
/// handle on the voice.
#[cfg(feature = "debug")]
const HUM_VOICE_NAME: &str = "Thruster Loop Sfx";

/// Rounds left in the gun when the trigger goes down, so the magazine empties
/// under a held trigger rather than before one.
///
/// Three, not one: the claim is that a gun that RAN dry clicks, and a magazine
/// that was already empty when the trigger fell would prove only that an empty
/// gun clicks.
#[cfg(feature = "debug")]
const DRY_ROUNDS: u32 = 3;

/// The key the gun is bound to.
const TRIGGER_KEY: KeyCode = KeyCode::KeyH;

/// The two main-drive burns the hum is read at.
///
/// Written as [`FlightIntent::burn`] - the pilot's own analog throttle - and
/// not onto the thrusters. The flight model OWNS `ThrusterSectionInput`: it
/// allocates the commanded burn across the live engines and spools each one
/// toward its share every fixed tick, so a range that wrote the section input
/// directly would be overwritten on an unordered schedule and would measure its
/// own race. What the drive is actually holding is therefore measured, not
/// assumed - see [`HumSamples`].
#[cfg(feature = "debug")]
const FULL_BURN: f32 = 1.0;
#[cfg(feature = "debug")]
const EASED_BURN: f32 = 0.4;

/// How far the measured hum ratio may sit from the measured throttle ratio.
///
/// Both halves are this run's own readings, taken in the same beat off the same
/// ship: the hum curve is linear in the ship's average throttle, so two settled
/// levels must stand in the ratio their two settled throttles do. The allowance
/// covers the last of the loop's easing and the f32 arithmetic on either side
/// of the ratio - nothing here is graded on a level, which is just as well,
/// since the ceiling the curve saturates at is private to `nova_ship`.
#[cfg(feature = "debug")]
const HUM_RATIO_TOLERANCE: f32 = 0.02;

/// In-step seconds a throttle setting is held before its level is read.
///
/// The loops EASE, which is the design (a throttle change that clicked would be
/// a fault), so a level read on the frame the throttle moved is a reading of
/// the easing and not of the curve. This is not a budget and nothing is graded
/// on it: it is how long the range waits before it measures.
#[cfg(feature = "debug")]
const HUM_SETTLE_SECS: f32 = 2.0;

/// In-step seconds the dead trigger is held down after the magazine empties.
///
/// The claim is that a HELD empty trigger does not repeat, and that only means
/// anything over a window: at 100 rounds a second the cue pass runs a few
/// hundred times inside this one.
#[cfg(feature = "debug")]
const DEAD_TRIGGER_HOLD_SECS: f32 = 1.0;

/// In-step seconds between letting the dead trigger go and pulling it again.
#[cfg(feature = "debug")]
const TRIGGER_RELEASE_SECS: f32 = 0.5;

/// How far (world units) the placed emitter may sit from where the pan law puts
/// it.
///
/// The engine writes the emitter from the SAME listener pose the range reads,
/// one system earlier in the same frame, so the two agree exactly bar f32
/// rounding over a 2.5-unit radius. The camera is parked before this is read so
/// a frame of rig easing cannot open the gap either.
#[cfg(feature = "debug")]
const EMITTER_TOLERANCE_UNITS: f32 = 0.01;

/// Which hull a contact belongs to, decided by range: every section of one hull
/// is far inside this of its own root and far outside it of the other's.
#[cfg(feature = "debug")]
const HULL_REACH_UNITS: f32 = 20.0;

/// What "the camera has stopped" means: a frame's movement under this much
/// translation and this much rotation, held for this many consecutive frames.
#[cfg(feature = "debug")]
const CAMERA_PARKED_UNITS: f32 = 0.01;
#[cfg(feature = "debug")]
const CAMERA_PARKED_RAD: f32 = 0.0005;
#[cfg(feature = "debug")]
const CAMERA_PARKED_FRAMES: u32 = 30;

/// In-step seconds a beat gets to reach its world condition. A backstop that
/// names a hung beat, not a budget: the correctness pass runs on a software
/// rasterizer where a frame costs real time.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 120.0;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.init_resource::<VoiceLog>();
        app.init_resource::<HumSamples>();
        app.init_resource::<HeldInput>();
        // The log is taken HERE and not by sampling the voice set once a beat.
        // A one-shot is retired the moment its clip ends (or, with no output
        // device, a couple of seconds after it was asked for), so "how many
        // reports did that frame ask for" has an answer only at the frame the
        // voices were born.
        app.add_observer(
            |add: On<Add, SfxVoice>,
             asset_server: Res<AssetServer>,
             q_voice: Query<&SfxVoice>,
             mut log: ResMut<VoiceLog>| {
                let Ok(voice) = q_voice.get(add.entity) else {
                    return;
                };
                log.0.push(VoiceRequest {
                    voice: add.entity,
                    sound: asset_server
                        .get_path(voice.handle.id())
                        .map(|path| path.to_string())
                        .unwrap_or_default(),
                    route: voice.route,
                    source: voice.source,
                    volume: voice.volume,
                });
            },
        );
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        app.add_plugins(nova_screenshot(audio_script()));
    }

    app.run()
}

fn range_plugin(app: &mut App) {
    // On assets-Loaded, not on Playing: `assert_scenario_loaded` checks the
    // load happened by OnEnter(Playing), and loading in that same schedule is
    // an unordered race against the check.
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(audio_range(&game_assets, &sections)));
}

/// The scene: the player's hull with a drive and a gun, and a second hull off
/// the bow that nobody is flying.
fn audio_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    // Only the gun is bound. The drive is flown by writing the pilot's analog
    // burn (see `command_the_burn`), because a key can only say "all of it"
    // and the hum claim needs a part throttle as well as a full one.
    let pilot = fixtures::ship(
        sections,
        SpaceshipController::Player(PlayerControllerConfig {
            // H, and deliberately not the shipped trigger key: Space is also
            // the default main-drive burn, so a range that fired the gun with
            // it would be flying the ship at the same time and the throttle
            // act below would be reading a burn the trigger left behind.
            input_mapping: BTreeMap::from([(PILOT_GUN.to_string(), vec![TRIGGER_KEY.into()])]),
            speed_cap: None,
        }),
        &[
            SectionSpec::new(PILOT_HELM, "basic_controller_section", Vec3::ZERO),
            SectionSpec::new(
                PILOT_DRIVE,
                "basic_thruster_section",
                Vec3::new(0.0, 0.0, 1.0),
            ),
            // 0.75 above the helm, unrotated: the mount's one link point sits
            // 0.25 under its own origin pointing down, and the helm's upward
            // face is half a cell up, so this is where the two meet. The
            // shipped hulls mount their turrets this way round - a gun that
            // does not link leaves the ship's graph disconnected and the
            // scenario refuses to start.
            SectionSpec::new(
                PILOT_GUN,
                "pdc_kinetic_turret_section",
                Vec3::new(0.0, 0.75, 0.0),
            ),
            SectionSpec::new(
                PILOT_PLATE,
                "reinforced_hull_section",
                Vec3::new(0.0, 0.0, -1.0),
            ),
        ],
    );

    // No controller at all: the bogey is a hull to hit, and an AI on it would
    // put a second ship's guns and drives into every reading below.
    let bogey = fixtures::ship(
        sections,
        SpaceshipController::None,
        &[
            SectionSpec::new(BOGEY_HELM, "basic_controller_section", Vec3::ZERO),
            SectionSpec::new(
                BOGEY_PLATE_A,
                "reinforced_hull_section",
                Vec3::new(0.0, 0.0, 1.0),
            ),
            SectionSpec::new(
                BOGEY_PLATE_B,
                "reinforced_hull_section",
                Vec3::new(0.0, 0.0, -1.0),
            ),
        ],
    );

    let mut start_actions = vec![
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: PILOT_ID.to_string(),
                name: "Pilot".to_string(),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(pilot),
        }),
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: BOGEY_ID.to_string(),
                name: "Bogey".to_string(),
                position: BOGEY_AT,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(bogey),
        }),
    ];
    // The engine spawns no light: a scenario that authors none renders black,
    // and this range is run with a display like every other.
    start_actions.extend(ThreePointRig::around("audio", Meters3::ZERO, 40.0).actions());

    ScenarioConfig {
        description: "The ship's soundtrack, read off the live voices".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: start_actions,
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Ship Audio".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

// --- Range state -------------------------------------------------------------

/// One voice the layer asked the engine for, as it was asked for.
///
/// Recorded at `Add` time rather than read off the live set, and kept as plain
/// data so a claim can still be made about a voice that has since retired.
#[cfg(feature = "debug")]
#[derive(Clone, Debug)]
struct VoiceRequest {
    /// The voice entity, for the placement reading, which is only meaningful
    /// while the voice is alive.
    voice: Entity,
    /// The resolved asset path, or the empty string for a handle with none.
    sound: String,
    /// Which track scales it, and whether it is placed.
    route: AudioRoute,
    /// Where it is heard from.
    source: SfxSource,
    /// The owner's requested level, before the bus gain and the master.
    volume: f32,
}

/// Every voice asked for since the log was last cleared.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct VoiceLog(Vec<VoiceRequest>);

#[cfg(feature = "debug")]
impl VoiceLog {
    /// The requests for the sound whose path ends in `suffix`.
    fn asking_for(&self, suffix: &str) -> Vec<VoiceRequest> {
        self.0
            .iter()
            .filter(|request| request.sound.ends_with(suffix))
            .cloned()
            .collect()
    }
}

/// One settled reading: what the hum was asking for, and what the drive was
/// actually holding when it asked.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug)]
struct HumSample {
    /// The hum voice's requested level.
    level: f32,
    /// The ship's average live thruster throttle - the same average the loop
    /// pass takes, read off the same components.
    throttle: f32,
    /// Which track the hum is on.
    route: AudioRoute,
    /// What the hum is following.
    source: SfxSource,
}

/// The two settled readings, filled in by the beats that move the throttle.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct HumSamples {
    /// The reading at [`FULL_BURN`].
    full: Option<HumSample>,
    /// The reading at [`EASED_BURN`].
    eased: Option<HumSample>,
}

/// What the script is holding down.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct HeldInput {
    /// RMB - the combat stance the safety gates firing on.
    combat: bool,
    /// [`TRIGGER_KEY`] - the trigger.
    fire: bool,
}

/// Re-press the held inputs for the frame.
///
/// Wired as the script's `input` hook and not as an `Update` system: that slot
/// runs in `PreUpdate` after `InputSystems`, which is the only place a
/// synthesized press is still `just_pressed` when the game's `Update` input
/// systems read it.
#[cfg(feature = "debug")]
fn hold_inputs(world: &mut World, _elapsed: f32, _frame: u32) {
    let held = world.resource::<HeldInput>();
    let (combat, fire) = (held.combat, held.fire);
    if combat {
        drive_action(world, "combat_stance", InputPhase::Press);
    }
    if fire {
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(TRIGGER_KEY);
    }
}

// --- The script --------------------------------------------------------------

/// The run: rake both hulls, run the gun dry, then work the throttle.
///
/// The burst goes first because it is the only claim that wants a still scene -
/// the emitter reading is taken against the listener's live pose - and the
/// throttle goes last because it is the one beat that moves the ship.
#[cfg(feature = "debug")]
fn audio_script() -> Script {
    let script = Script::new()
        .input(hold_inputs)
        .step("load the range")
        .enter(GameStates::Loading)
        .until(and(player_ship_present(), both_hulls_are_up()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // The emitter is parked relative to the listener, so the reading is
        // only a comparison once the listener has stopped moving.
        .step("let the camera come to rest")
        .until(the_camera_is_parked())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("rake both hulls in one frame")
        .on_enter(rake_both_hulls)
        // Two frames, not zero: the cue observer plays through `Commands`, so
        // the `PlaySfx` triggers land on the next flush and the engine places
        // the voices the frame after that.
        .until(frames(3))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("read the reports the burst asked for")
        .on_enter(assert_the_burst_reads_as_two_reports)
        .add();
    let script = run_the_gun_dry(script);
    work_the_throttle(script)
}

/// The gun's act: raise the mount, spend three rounds, then hold and re-pull a
/// dead trigger.
#[cfg(feature = "debug")]
fn run_the_gun_dry(script: Script) -> Script {
    script
        .step("raise the guns")
        .on_enter(|world: &mut World| world.resource_mut::<HeldInput>().combat = true)
        .until(the_mount_is_deployed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("load three rounds and pull the trigger")
        .on_enter(load_three_rounds_and_fire)
        .until(and(
            the_magazine_is_empty(),
            elapsed(DEAD_TRIGGER_HOLD_SECS),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let the dead trigger go")
        .on_enter(release_the_trigger)
        .until(elapsed(TRIGGER_RELEASE_SECS))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("pull the dead trigger again")
        .on_enter(|world: &mut World| world.resource_mut::<HeldInput>().fire = true)
        .until(elapsed(TRIGGER_RELEASE_SECS))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("read what the dead trigger asked for")
        .on_enter(assert_a_dead_trigger_clicks_once_a_pull)
        .add()
}

/// The drive's act: full throttle, a partial one, then nothing.
#[cfg(feature = "debug")]
fn work_the_throttle(script: Script) -> Script {
    script
        .step("open the throttle")
        .on_enter(|world: &mut World| {
            release_the_trigger(world);
            command_the_burn(world, FULL_BURN);
        })
        .until(elapsed(HUM_SETTLE_SECS))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("ease the throttle back")
        .on_enter(sample_the_hum_and_ease_off)
        .until(elapsed(HUM_SETTLE_SECS))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("close the throttle")
        .on_enter(sample_the_hum_and_close_the_throttle)
        .until(the_hum_is_gone())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("read what the drive asked for")
        .on_enter(assert_the_hum_tracks_the_drive)
        .add()
}

// --- Staging -----------------------------------------------------------------

/// Hit every section of both hulls, all in the same frame.
///
/// One `SurfaceImpact` per section, at the section's own world point, all of
/// one damage type. That is the shape of a blast raking a hull: many contacts,
/// one event, and the audio layer's job is to decide how many of them are a
/// sound.
#[cfg(feature = "debug")]
fn rake_both_hulls(world: &mut World) {
    world.resource_mut::<VoiceLog>().0.clear();
    let contacts: Vec<(Entity, Vec3)> = [
        PILOT_HELM,
        PILOT_DRIVE,
        PILOT_GUN,
        PILOT_PLATE,
        BOGEY_HELM,
        BOGEY_PLATE_A,
        BOGEY_PLATE_B,
    ]
    .iter()
    .map(|id| {
        let section = section_by_id(world, id)
            .unwrap_or_else(|| panic!("ship audio: no live section called '{id}'"));
        let at = world
            .get::<GlobalTransform>(section)
            .expect("ship audio: a live section has a GlobalTransform")
            .translation();
        (section, at)
    })
    .collect();
    info!(
        "ship audio: raking {} sections in one frame",
        contacts.len()
    );
    for (section, at) in contacts {
        world.trigger(SurfaceImpact {
            entity: section,
            kind: DamageType::Kinetic,
            at,
        });
    }
}

/// Put three rounds in the gun, take the reload off it, and pull the trigger.
///
/// The reload is REMOVED rather than waited out: the shipped magazine refills
/// on its own after a lull, and a range that let it would be asserting that a
/// gun which came back up does not click. The claim is about a trigger held on
/// an empty gun, so the gun has to stay empty.
#[cfg(feature = "debug")]
fn load_three_rounds_and_fire(world: &mut World) {
    let gun = section_by_id(world, PILOT_GUN).expect("ship audio: the player's gun is gone");
    world.entity_mut(gun).remove::<SectionReload>();
    let mut ammo = world
        .get_mut::<SectionAmmo>(gun)
        .expect("ship audio: the shipped PDC carries a magazine");
    ammo.rounds = DRY_ROUNDS;
    world.resource_mut::<VoiceLog>().0.clear();
    world.resource_mut::<HeldInput>().fire = true;
    info!("ship audio: {DRY_ROUNDS} rounds in the gun, trigger down");
}

/// Drop the trigger, and RELEASE the key with it.
///
/// The fire input is a latched bool, so merely not re-pressing leaves it down:
/// the release is what ends the pull.
#[cfg(feature = "debug")]
fn release_the_trigger(world: &mut World) {
    world.resource_mut::<HeldInput>().fire = false;
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(TRIGGER_KEY);
}

/// Read the settled full-burn sample, then ease the throttle back.
#[cfg(feature = "debug")]
fn sample_the_hum_and_ease_off(world: &mut World) {
    let hum = sample_the_hum(world).expect("ship audio: full burn and no hum voice");
    info!(
        "ship audio: the hum settled at {:.4} with the drive at {:.4}",
        hum.level, hum.throttle
    );
    world.resource_mut::<HumSamples>().full = Some(hum);
    command_the_burn(world, EASED_BURN);
}

/// Read the settled eased sample, then close the throttle.
#[cfg(feature = "debug")]
fn sample_the_hum_and_close_the_throttle(world: &mut World) {
    let hum = sample_the_hum(world).expect("ship audio: part burn and no hum voice");
    info!(
        "ship audio: the hum settled at {:.4} with the drive at {:.4}",
        hum.level, hum.throttle
    );
    world.resource_mut::<HumSamples>().eased = Some(hum);
    command_the_burn(world, 0.0);
}

/// Put the pilot's analog throttle at `burn`.
///
/// One write, not a per-frame hold: nothing in the game clears a burn the pilot
/// is holding, and the flight model reads this every fixed tick from here on.
#[cfg(feature = "debug")]
fn command_the_burn(world: &mut World, burn: f32) {
    let player = player_root(world).expect("ship audio: no player ship to fly");
    let mut intent = world
        .get_mut::<FlightIntent>(player)
        .expect("ship audio: a flyable hull carries a FlightIntent");
    intent.burn = burn;
}

// --- Claims ------------------------------------------------------------------

/// The burst: one report per struck hull, each on its own hull's route, and the
/// exterior one parked where the pan law puts it.
///
/// Three claims off one staged frame, because they are three readings of the
/// same two voices and the voices do not outlive the frame by much. Seven
/// contacts went in.
#[cfg(feature = "debug")]
fn assert_the_burst_reads_as_two_reports(world: &mut World) {
    assert!(
        world.resource::<HarnessMute>().0,
        "ship audio: the whole range is a claim about what a MUTED run still asks for"
    );

    let reports = world.resource::<VoiceLog>().asking_for(IMPACT_SOUND);
    assert_eq!(
        reports.len(),
        2,
        "ship audio: seven contacts on two hulls should read as one report each, got {reports:?}"
    );
    info!("ship audio: the burst read as {} reports", reports.len());
    nova_probe::probe_marker(
        world,
        "outcome: a burst raking a hull collapses to one report per hull",
        serde_json::json!({
            "contacts": 7,
            "hulls": 2,
            "reports": reports.len(),
            "sound": IMPACT_SOUND,
        }),
    );

    let pilot = object_by_id(world, PILOT_ID).expect("ship audio: the player's hull is gone");
    let bogey = object_by_id(world, BOGEY_ID).expect("ship audio: the bogey is gone");
    let mine = report_on(world, &reports, pilot);
    let theirs = report_on(world, &reports, bogey);
    assert_eq!(
        (mine.route, theirs.route),
        (AudioRoute::Hull, AudioRoute::Exterior),
        "ship audio: the same authored hit should reach the pilot through their own hull and \
         across the gap from the bogey's"
    );
    assert_eq!(
        mine.sound, theirs.sound,
        "ship audio: both hulls are plate struck by the same round, so the two reports are the \
         same authored voice heard two ways"
    );
    info!(
        "ship audio: '{}' reached the pilot as {:?} and the bogey as {:?}",
        mine.sound, mine.route, theirs.route
    );
    nova_probe::probe_marker(
        world,
        "outcome: an authored cue is heard through the hull it belongs to",
        serde_json::json!({
            "sound": mine.sound,
            "player_route": format!("{:?}", mine.route),
            "other_route": format!("{:?}", theirs.route),
            "player_level": mine.volume,
            "other_level": theirs.volume,
        }),
    );

    let SfxSource::At(contact) = theirs.source else {
        panic!(
            "ship audio: an exterior report must carry the point it happened at, got {:?}",
            theirs.source
        );
    };
    let listener = listener_pose(world).expect("ship audio: the scenario camera has the ears");
    let bearing = nova_gameplay::audio::local_bearing(&listener, contact);
    let expected = nova_gameplay::audio::emitter_point(&listener, bearing);
    let placed = world
        .get::<GlobalTransform>(theirs.voice)
        .expect("ship audio: the exterior report retired before it could be read")
        .translation();
    let drift = placed.distance(expected);
    let radius = placed.distance(listener.translation());
    assert!(
        drift <= EMITTER_TOLERANCE_UNITS,
        "ship audio: the exterior report is placed at {placed:?}, {drift:.4} units off the \
         {expected:?} the pan law puts a contact at {contact:?} on"
    );
    info!(
        "ship audio: the exterior report is parked {radius:.3} units from the listener on the \
         bearing to its contact, {drift:.4} off"
    );
    nova_probe::probe_marker(
        world,
        "outcome: an exterior cue is parked on the listener's bearing to it",
        serde_json::json!({
            "contact": [contact.x, contact.y, contact.z],
            "listener": [
                listener.translation().x,
                listener.translation().y,
                listener.translation().z,
            ],
            "placed": [placed.x, placed.y, placed.z],
            "emitter_radius_units": radius,
            "drift_units": drift,
        }),
    );
}

/// The dead trigger: one click a pull, however long the pull is held.
#[cfg(feature = "debug")]
fn assert_a_dead_trigger_clicks_once_a_pull(world: &mut World) {
    let gun = section_by_id(world, PILOT_GUN).expect("ship audio: the player's gun is gone");
    let rounds = world
        .get::<SectionAmmo>(gun)
        .expect("ship audio: the gun lost its magazine")
        .rounds;
    assert_eq!(
        rounds, 0,
        "ship audio: the gun has to have RUN dry for the click to be about running dry"
    );
    let shots = world.resource::<VoiceLog>().asking_for(TURRET_FIRE_SOUND);
    assert!(
        !shots.is_empty(),
        "ship audio: the gun emptied without asking for its own fire voice once"
    );

    let clicks = world.resource::<VoiceLog>().asking_for(DRY_FIRE_SOUND);
    assert_eq!(
        clicks.len(),
        2,
        "ship audio: two pulls on a dead trigger - one held for {DEAD_TRIGGER_HOLD_SECS}s - \
         should click twice, got {clicks:?}"
    );
    assert!(
        clicks.iter().all(|click| click.route == AudioRoute::Hull),
        "ship audio: a dead trigger is the pilot's own gun, so its click is heard through the \
         hull: {clicks:?}"
    );
    info!(
        "ship audio: the dead trigger clicked {} times over 2 pulls, after {} fire requests",
        clicks.len(),
        shots.len()
    );
    nova_probe::probe_marker(
        world,
        "outcome: a dead trigger clicks once a pull and not once a frame",
        serde_json::json!({
            "rounds_loaded": DRY_ROUNDS,
            "fire_requests": shots.len(),
            "pulls": 2,
            "clicks": clicks.len(),
            "held_secs": DEAD_TRIGGER_HOLD_SECS,
        }),
    );
}

/// The hum: two settled levels in the ratio their throttles were in, and
/// nothing left when the throttle closes.
#[cfg(feature = "debug")]
fn assert_the_hum_tracks_the_drive(world: &mut World) {
    let samples = world.resource::<HumSamples>();
    let full = samples.full.expect("ship audio: no full-burn sample");
    let eased = samples.eased.expect("ship audio: no eased sample");
    assert!(
        full.level > 0.0 && full.throttle > 0.0,
        "ship audio: a hull at full burn must be holding a throttle and asking for a hum, \
         got {full:?}"
    );
    assert!(
        eased.throttle < full.throttle,
        "ship audio: easing the pilot's burn from {FULL_BURN} to {EASED_BURN} has to leave the \
         drive turning slower, got {:.4} against {:.4}",
        eased.throttle,
        full.throttle
    );

    let pilot = object_by_id(world, PILOT_ID).expect("ship audio: the player's hull is gone");
    assert_eq!(
        (full.route, full.source),
        (AudioRoute::Hull, SfxSource::Follow(pilot)),
        "ship audio: the pilot's own drive is heard through their hull, following their hull"
    );

    let wanted = eased.throttle / full.throttle;
    let measured = eased.level / full.level;
    assert!(
        (measured - wanted).abs() <= HUM_RATIO_TOLERANCE,
        "ship audio: the drive dropped to {wanted:.4} of its throttle and the hum to \
         {measured:.4} of its level - the hum is not tracking the drive \
         ({:.4} -> {:.4} against {:.4} -> {:.4})",
        full.throttle,
        eased.throttle,
        full.level,
        eased.level
    );
    assert!(
        hum_voice(world).is_none(),
        "ship audio: a closed throttle must retire the hum voice, not hold an open sink at zero"
    );
    info!(
        "ship audio: the hum settled at {:.4} and {:.4} while the drive held {:.4} and {:.4} - \
         a level ratio of {measured:.4} against the drive's {wanted:.4}, and gone at rest",
        full.level, eased.level, full.throttle, eased.throttle
    );
    nova_probe::probe_marker(
        world,
        "outcome: the drive's hum tracks the throttle and leaves at rest",
        serde_json::json!({
            "full_burn": FULL_BURN,
            "eased_burn": EASED_BURN,
            "full_throttle": full.throttle,
            "eased_throttle": eased.throttle,
            "full_level": full.level,
            "eased_level": eased.level,
            "level_ratio": measured,
            "throttle_ratio": wanted,
            "route": format!("{:?}", full.route),
        }),
    );
}

// --- Predicates --------------------------------------------------------------

/// Both hulls have reached the world.
#[cfg(feature = "debug")]
fn both_hulls_are_up() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        object_by_id(world, PILOT_ID).is_some() && object_by_id(world, BOGEY_ID).is_some()
    })
}

/// The player's mount has finished coming up.
#[cfg(feature = "debug")]
fn the_mount_is_deployed() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&TurretStow, With<TurretSectionMarker>>()
            .is_some_and(|mut query| query.iter(world).any(TurretStow::is_deployed))
    })
}

/// The player's gun has nothing left to fire.
#[cfg(feature = "debug")]
fn the_magazine_is_empty() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&SectionAmmo, With<TurretSectionMarker>>()
            .is_some_and(|mut query| query.iter(world).any(SectionAmmo::is_empty))
    })
}

/// No hum voice is left in the world.
#[cfg(feature = "debug")]
fn the_hum_is_gone() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| hum_voice(world).is_none())
}

/// The listener camera has stopped moving.
///
/// Stated positively and measured over consecutive frames, because one still
/// frame is not a stopped camera: the rig eases, so any single frame near the
/// turn of the ease reads still.
#[cfg(feature = "debug")]
fn the_camera_is_parked() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    use std::sync::{
        atomic::{AtomicU32, Ordering},
        Mutex,
    };

    let last: Mutex<Option<(Vec3, Quat)>> = Mutex::new(None);
    let still = AtomicU32::new(0);
    std::sync::Arc::new(move |world: &World| {
        let Some(pose) = listener_pose(world) else {
            return false;
        };
        let now = (pose.translation(), pose.rotation());
        let mut last = last.lock().expect("ship audio: camera watch poisoned");
        let parked = last.is_some_and(|(at, facing)| {
            at.distance(now.0) < CAMERA_PARKED_UNITS
                && facing.angle_between(now.1) < CAMERA_PARKED_RAD
        });
        *last = Some(now);
        if parked {
            still.fetch_add(1, Ordering::Relaxed) + 1 >= CAMERA_PARKED_FRAMES
        } else {
            still.store(0, Ordering::Relaxed);
            false
        }
    })
}

// --- World access ------------------------------------------------------------

/// The scenario-scoped root carrying `id`.
#[cfg(feature = "debug")]
fn object_by_id(world: &World, id: &str) -> Option<Entity> {
    world
        .try_query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>()
        .and_then(|mut query| {
            query
                .iter(world)
                .find(|(_, live)| live.0 == id)
                .map(|(entity, _)| entity)
        })
}

/// The live SECTION carrying `id`.
#[cfg(feature = "debug")]
fn section_by_id(world: &World, id: &str) -> Option<Entity> {
    world
        .try_query_filtered::<(Entity, &EntityId), With<SectionMarker>>()
        .and_then(|mut query| {
            query
                .iter(world)
                .find(|(_, live)| live.0 == id)
                .map(|(entity, _)| entity)
        })
}

/// Where the engine hears from: the marked scenario camera's pose.
#[cfg(feature = "debug")]
fn listener_pose(world: &World) -> Option<GlobalTransform> {
    world
        .try_query_filtered::<&GlobalTransform, With<SfxListenerMarker>>()
        .and_then(|mut query| query.iter(world).next().copied())
}

/// The live hum voice entity, if the drive has one.
#[cfg(feature = "debug")]
fn hum_voice(world: &World) -> Option<Entity> {
    world
        .try_query_filtered::<(Entity, &Name), With<SfxVoice>>()
        .and_then(|mut query| {
            query
                .iter(world)
                .find(|(_, name)| name.as_str() == HUM_VOICE_NAME)
                .map(|(entity, _)| entity)
        })
}

/// The hum's requested level beside the throttle the drive is holding, both
/// read in the same frame.
#[cfg(feature = "debug")]
fn sample_the_hum(world: &mut World) -> Option<HumSample> {
    let voice = hum_voice(world)?;
    let throttle = player_throttle(world);
    world.get::<SfxVoice>(voice).map(|hum| HumSample {
        level: hum.volume,
        throttle,
        route: hum.route,
        source: hum.source,
    })
}

/// The player's average live thruster throttle: the same average the hum pass
/// takes, over the same components, so the two readings are of one thing.
#[cfg(feature = "debug")]
fn player_throttle(world: &mut World) -> f32 {
    let player = player_root(world).expect("ship audio: no player ship to read a throttle off");
    let mut sum = 0.0;
    let mut count = 0u32;
    let mut query = world.query_filtered::<(&ThrusterSectionInput, &ChildOf), (
        With<ThrusterSectionMarker>,
        Without<SectionInactiveMarker>,
    )>();
    for (input, &ChildOf(ship)) in query.iter(world) {
        if ship == player {
            sum += input.abs();
            count += 1;
        }
    }
    assert!(count > 0, "ship audio: the player's hull has no live drive");
    sum / count as f32
}

/// The player ship root.
#[cfg(feature = "debug")]
fn player_root(world: &World) -> Option<Entity> {
    world
        .try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .and_then(|mut query| query.iter(world).next())
}

/// The one report among `reports` whose contact point is on `hull`.
///
/// By RANGE, because a placed cue carries where it happened and not what it
/// happened to: the two hulls are 45 units apart and each is a couple of units
/// across, so [`HULL_REACH_UNITS`] separates them with two orders of magnitude
/// to spare.
#[cfg(feature = "debug")]
fn report_on(world: &World, reports: &[VoiceRequest], hull: Entity) -> VoiceRequest {
    let at = world
        .get::<GlobalTransform>(hull)
        .expect("ship audio: a live hull has a GlobalTransform")
        .translation();
    let mut near = reports.iter().filter(|report| match report.source {
        SfxSource::At(point) => point.distance(at) <= HULL_REACH_UNITS,
        _ => false,
    });
    let found = near
        .next()
        .unwrap_or_else(|| panic!("ship audio: no report landed on {hull:?} at {at:?}"))
        .clone();
    assert!(
        near.next().is_none(),
        "ship audio: one hull's burst must be one report, and {hull:?} got more than one"
    );
    found
}
