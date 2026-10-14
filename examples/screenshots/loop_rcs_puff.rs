//! loop_rcs_puff: another ship's RCS push, seen and heard from escort range.
//!
//! One inert catalog interceptor (not the player's ship) at the origin, and a
//! fixed lens 300 m off it - the range an escort holds its slot at. The script
//! writes the ship's `RcsIntent` the way the autopilot does: a full push to
//! starboard, a full push back to port, a light quarter push up, then nothing.
//! `rcs_burn_system` delivers it, the hiss and the puffs follow
//! `RcsBudget::applied`.
//!
//! What the run asserts, per push: the engine resolved an RCS loop voice
//! following the ship at an exterior level at least the pilot's own full hull
//! hiss, the cap did not hold it silent, and RCS puffs were in the world. After
//! the release: no puffs and no RCS voice.
//!
//! The listener is the posed scenario camera, so the level is the one heard
//! 300 m off. A capture run renders the loop's audio offline from the same
//! `VoiceMix` (`nova_debug::sfx_sidecar`), so the WebM's Opus track carries
//! the hiss even under a muted master.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the full walk and its asserts,
//!   recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also record `rcs-puff.webm` under
//!   `NOVA_CAPTURE_DIR`.
//!
//! Capture:
//! ```text
//! NOVA_CAPTURE_DIR=target/loop-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example loop_rcs_puff --features debug
//! ```

#[path = "shared/kit.rs"]
mod kit;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "loop_rcs_puff")]
#[command(version = "1.0.0")]
#[command(about = "Another ship's RCS push seen and heard from escort range", long_about = None)]
struct Cli;

/// The loop this example records - the webm's file stem.
#[cfg(feature = "debug")]
const LOOP_NAME: &str = "rcs-puff";

/// Scenario id of the pushing ship.
const SHIP_ID: &str = "rcs_ship";

/// Where the lens stands, from the ship: about 305 m out, ahead of its nose
/// and above, square to its beam, so a push to either side crosses the frame
/// and both flanks the gas leaves from face the lens.
#[cfg(feature = "debug")]
const LENS_OFFSET: Meters3 = Meters3::new(-110.0, 90.0, 270.0);

/// The pilot's own RCS hiss at full deflection: `RCS_MAX_VOLUME` in
/// `crates/nova_ship/src/ship_audio/levels.rs`. Another ship's push at escort
/// range must resolve to at least this.
#[cfg(feature = "debug")]
const HULL_HISS_CEILING: f32 = 0.05;

/// The command the script holds on the ship this frame, ship-local.
#[cfg(feature = "debug")]
#[derive(Resource, Clone, Copy)]
struct RcsPush(Vec3);

/// What one push phase resolved: the settled RCS voice level, whether the cap
/// ever held it, and the puffs alive at its end.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug, Default)]
struct PhaseSample {
    frames: u32,
    /// The RCS voice level on the phase's last frame, when the smoothing has
    /// settled on the push.
    level: f32,
    capped: bool,
    /// Puffs alive on the phase's last frame.
    puffs: usize,
}

/// The per-phase samples, keyed by phase name, and the current phase.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct RcsTrace {
    phase: &'static str,
    samples: Vec<(&'static str, PhaseSample)>,
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(rcs_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_systems(Update, hold_push.run_if(resource_exists::<RcsPush>));
        app.add_systems(Update, trace_rcs.run_if(resource_exists::<RcsTrace>));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    commands.trigger(LoadScenario(rcs_range(&game_assets, &ships)));
}

/// The set: one inert interceptor and the photo rig. No player ship, so every
/// ship cue routes exterior and is heard from the posed lens.
fn rcs_range(game_assets: &GameAssets, ships: &GameShipDesigns) -> ScenarioConfig {
    let ship = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: SHIP_ID.to_string(),
            name: "Escort".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::from_rotation_y(std::f32::consts::PI - 0.4),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance: Some(Allegiance::Player),
            design: ShipDesignSource::Inline(kit::catalog_ship(ships, "interceptor")),
            ..default()
        }),
    });
    ScenarioConfig {
        description: "One ship pushing on its RCS, watched from escort range.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![ship],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "loop_rcs_puff".to_string(),
            "RCS Puff Loop".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The ship root carrying [`SHIP_ID`].
#[cfg(feature = "debug")]
fn ship_root(world: &mut World) -> Option<Entity> {
    world
        .query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()
        .iter(world)
        .find(|(_, id)| id.0 == SHIP_ID)
        .map(|(entity, _)| entity)
}

/// Advance once the ship is in the world.
#[cfg(feature = "debug")]
fn ship_present() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&EntityId, With<SpaceshipRootMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|id| id.0 == SHIP_ID))
    })
}

/// Write the held command onto the ship every frame, as the autopilot does.
#[cfg(feature = "debug")]
fn hold_push(
    mut commands: Commands,
    push: Res<RcsPush>,
    q_ship: Query<(Entity, &EntityId, Option<&RcsIntent>), With<SpaceshipRootMarker>>,
) {
    for (ship, id, intent) in &q_ship {
        if id.0 != SHIP_ID || intent.is_some_and(|intent| intent.0 == push.0) {
            continue;
        }
        commands.entity(ship).insert(RcsIntent(push.0));
    }
}

/// Sample the ship's RCS voice and the live puffs into the current phase.
#[cfg(feature = "debug")]
fn trace_rcs(
    mut trace: ResMut<RcsTrace>,
    q_ship: Query<(Entity, &EntityId, &RcsBudget), With<SpaceshipRootMarker>>,
    q_voices: Query<(&Name, &SfxVoice, &VoiceMix)>,
    q_puffs: Query<(), With<RcsPuffMarker>>,
) {
    let Some((ship, _, budget)) = q_ship.iter().find(|(_, id, _)| id.0 == SHIP_ID) else {
        return;
    };
    let voice = q_voices.iter().find(|(name, voice, _)| {
        name.as_str() == "RCS Loop Sfx" && voice.source == SfxSource::Follow(ship)
    });
    let puffs = q_puffs.iter().count();
    let phase = trace.phase;
    if trace.samples.last().is_none_or(|(last, _)| *last != phase) {
        trace.samples.push((phase, PhaseSample::default()));
    }
    let (_, sample) = trace.samples.last_mut().expect("pushed above");
    sample.frames += 1;
    sample.level = voice.map_or(0.0, |(_, _, mix)| mix.level);
    sample.capped |= voice.is_some_and(|(_, _, mix)| mix.capped);
    sample.puffs = puffs;
    if sample.frames % 15 == 1 {
        info!(
            "rcs puff loop: {phase} applied {:.2} voice level {:.4} capped {} puffs {puffs}",
            budget.applied,
            voice.map_or(0.0, |(_, _, mix)| mix.level),
            voice.is_some_and(|(_, _, mix)| mix.capped),
        );
    }
}

/// Start a push phase: hold `command` and sample under `phase`.
#[cfg(feature = "debug")]
fn push(phase: &'static str, command: Vec3) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        world.insert_resource(RcsPush(command));
        world.resource_mut::<RcsTrace>().phase = phase;
    }
}

/// Every push phase was heard at escort range and shown, and the release left
/// nothing behind.
#[cfg(feature = "debug")]
fn assert_trace(world: &mut World) {
    let trace = world.resource::<RcsTrace>();
    info!("rcs puff loop: trace {:?}", trace.samples);
    for phase in ["starboard", "port", "light"] {
        let sample = trace
            .samples
            .iter()
            .find(|(name, _)| *name == phase)
            .map(|(_, sample)| *sample)
            .unwrap_or_else(|| panic!("rcs puff loop: no '{phase}' samples"));
        assert!(
            sample.level >= HULL_HISS_CEILING,
            "rcs puff loop: '{phase}' resolved the RCS hiss at {:.4}, under the pilot's own \
             full hiss {HULL_HISS_CEILING}",
            sample.level
        );
        assert!(
            !sample.capped,
            "rcs puff loop: the cap held the '{phase}' hiss silent"
        );
        assert!(
            sample.puffs > 0,
            "rcs puff loop: '{phase}' showed no RCS puff"
        );
    }
    let ship = ship_root(world).expect("rcs puff loop: the ship is gone");
    let puffs = world
        .query_filtered::<(), With<RcsPuffMarker>>()
        .iter(world)
        .count();
    assert_eq!(puffs, 0, "rcs puff loop: puffs outlived the release");
    let voices = world
        .query::<(&Name, &SfxVoice)>()
        .iter(world)
        .filter(|(name, voice)| {
            name.as_str() == "RCS Loop Sfx" && voice.source == SfxSource::Follow(ship)
        })
        .count();
    assert_eq!(voices, 0, "rcs puff loop: the hiss outlived the release");
}

#[cfg(feature = "debug")]
fn rcs_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the range")
        .enter(GameStates::Loading)
        .until(ship_present())
        .deadline(60.0)
        .add()
        .step("frame the ship from escort range")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            pose_camera(world, LENS_OFFSET, Meters3::ZERO);
            world.insert_resource(RcsTrace {
                phase: "quiet",
                ..default()
            });
        })
        .until(elapsed(1.0))
        .add()
        .step("open the loop")
        .on_enter(|world: &mut World| loop_start(world, LOOP_NAME))
        .until(elapsed(0.5))
        .add()
        .step("push to starboard")
        .on_enter(push("starboard", Vec3::X))
        .until(elapsed(1.0))
        .add()
        .step("push back to port")
        .on_enter(push("port", Vec3::NEG_X))
        .until(elapsed(1.0))
        .add()
        .step("push up lightly")
        .on_enter(push("light", Vec3::Y * 0.25))
        .until(elapsed(1.2))
        .add()
        .step("let go")
        .on_enter(push("release", Vec3::ZERO))
        .until(elapsed(1.2))
        .add()
        .step("close the loop")
        .on_enter(|world: &mut World| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(60.0)
        .add()
        .step("check the trace")
        .on_enter(assert_trace)
        .add()
}
