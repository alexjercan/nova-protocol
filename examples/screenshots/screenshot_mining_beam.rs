//! screenshot_mining_beam: the line warship's bow emitter cutting a locked rock.
//!
//! The shipped `block_line_warship` parks at the origin with one ore rock dead
//! ahead of its `mining_beam` section. The script travel-locks the rock and
//! holds the real `mine` action (`V`), so the doors, the tip, the pulses, the
//! carve and the canisters are all the production path.
//!
//! Shots, each held until its state is reached:
//! - `mining-beam-stowed.png`: the first rendered frame of the emitter's door
//!   and tip nodes, the emitter shut.
//! - `mining-beam-rock-before.png`: the rock's near face before the press.
//! - `mining-beam-doors.png`: the doors parting, the tip still in.
//! - `mining-beam-deploying.png`: the doors open, the tip partway out.
//! - `mining-beam-deployed.png`: the emitter out, the beam leaving its face.
//! - `wiki-section-mining-beam.png`: the level view from starboard after three
//!   paying pulses, the beam clear of the hull and ending on the rock.
//! - `mining-beam-sparks.png`: close on the hit 0.15 game seconds after the
//!   next pulse, its sparks in flight.
//! - `mining-beam-retracted.png`: the key released and the emitter shut again.
//! - `wiki-section-mining-beam-carve.png`: the carved face once every
//!   canister left.
//!
//! Then one more press and release, unpaused, recorded as the site's
//! `loop-section-mining-beam` webm: the doors part, the tip runs out, the beam
//! pulses with its sparks, and on release the tip and doors go back in.
//!
//! The door and tip tracks take 0.3 s each, which one lavapipe frame can
//! cross. While they move, the script caps a frame at [`TRACK_FRAME_STEP`] of
//! game time, so the in-between shots land on in-between poses.
//!
//! The run fails loudly if the emitter's nodes first render in any pose but
//! the stowed one, if the tip moves while the doors are not fully open, if a
//! pulse is refused, if too few pulses pay ore, if the rock has pushed the
//! warship off its mark, if a beam hit is left after the release, if the rock's
//! lost solid corners, the pulse log and the ore in canisters and the hold
//! disagree, or if the pulse sound did not play once per pulse through the
//! warship's hull.
//!
//! Capture (windowed, real GPU). An armed run pins the clock to the loop's
//! 30 fps, one frame a 1/30 s step - the same step [`TRACK_FRAME_STEP`] caps
//! a frame at:
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_mining_beam --features debug
//! ```
//!
//! Hand-run: lock the rock with the radar and hold `V`.
//! ```text
//! cargo run --example screenshot_mining_beam --features debug
//! ```

#[path = "shared/kit.rs"]
mod kit;

use std::collections::BTreeMap;

use bevy::prelude::*;
use clap::Parser;
use nova_input::prelude::InputSource;
#[cfg(feature = "debug")]
use nova_input::prelude::{dispatch, InputPhase};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_mining_beam")]
#[command(version = "1.0.0")]
#[command(about = "Capture the line warship's mining beam deploying, cutting a locked rock and retracting", long_about = None)]
struct Cli;

const PLAYER_ID: &str = "player";
const ROCK_ID: &str = "ore_rock";

/// The line warship's emitter face centre, in engine units from its root:
/// the -Z face of its cell at (1, 1, -7).
const EMITTER_FACE: Vec3 = Vec3::new(1.0, 1.0, -7.5);

/// Dead ahead of the emitter. The authored radius is nominal: this seed's
/// meshed surface stands about 10 engine units out from its centre, so its
/// near face is about 58 m ahead of the emitter, clear of the bow.
const ROCK_CENTRE: Vec3 = Vec3::new(1.0, 1.0, -23.5);
const ROCK_NEAR_FACE: Vec3 = Vec3::new(1.0, 1.0, -13.3);
const ROCK_RADIUS: Meters = Meters(25.0);
const ROCK_SEED: u32 = 7;

/// How far the parked warship may drift before the framings stop meaning
/// what they say, in engine units.
#[cfg(feature = "debug")]
const PARKED_DRIFT: f32 = 0.2;

/// Paying pulses to wait for before the hit shot.
#[cfg(feature = "debug")]
const PAID_PULSES: usize = 3;

/// The site's mining beam loop.
#[cfg(feature = "debug")]
const MINING_LOOP: &str = "loop-section-mining-beam";

/// Pulses the loop's press fires before the key comes up: the first at once
/// and one a second after it.
#[cfg(feature = "debug")]
const LOOP_PULSES: usize = 2;

/// Game seconds the loop holds the shut emitter at each end.
#[cfg(feature = "debug")]
const LOOP_HOLD_SECS: f32 = 0.5;

/// Game seconds after a pulse the sparks shot waits for, so the burst has
/// left the crater.
#[cfg(feature = "debug")]
const SPARKS_AFTER_PULSE_SECS: f32 = 0.15;

/// Real-seconds backstop for a state wait, long enough for lavapipe frames.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 90.0;

/// The game time one frame may advance while the door and tip tracks move:
/// about a ninth of a 0.3 s track, so a pause lands mid-travel.
#[cfg(feature = "debug")]
const TRACK_FRAME_STEP: std::time::Duration = std::time::Duration::from_micros(33_333);

/// Door progress at or below which the doors-parting shot is taken. The
/// pause lands one frame later, one track step further on.
#[cfg(feature = "debug")]
const DOORS_PARTING: f32 = 0.6;

/// The node-name prefixes the emitter's door and tip tracks drive.
#[cfg(feature = "debug")]
const EMITTER_NODE_PREFIXES: [&str; 2] = ["stow_lid_", "beam_tip"];

/// Every [`MiningPulse`] outcome in order, for the waits and the final check.
#[derive(Resource, Default)]
struct PulseLog(Vec<Result<u32, MiningRefusalType>>);

/// The route of every mining pulse sound the game played, in order.
#[derive(Resource, Default)]
struct PulseSounds(Vec<AudioRoute>);

/// The pulse sound's file, as the base bundle ships it.
const PULSE_SOUND: &str = "sounds/mining_pulse.wav";

/// What the script records to compare later.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct MiningProof {
    /// The emitter's door and tip node poses on their first rendered frame.
    first_pose: Option<Vec<(Entity, Transform)>>,
    /// The same nodes' poses once the emitter is deployed.
    deployed_pose: Option<Vec<(Entity, Transform)>>,
    /// The rock's solid corners when its field first exists, before any
    /// pulse carved it.
    seeded_corners: Option<u32>,
    /// `Time<Virtual>::max_delta` before [`TRACK_FRAME_STEP`] replaced it.
    free_max_delta: Option<std::time::Duration>,
    /// The pulse count when the sparks shot started waiting.
    pulses_before_sparks: Option<usize>,
    /// Game seconds at the last pulse.
    last_pulse_at: f32,
    /// The pulse count when the loop opened.
    pulses_before_loop: Option<usize>,
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        app.init_resource::<MiningProof>();
        app.add_observer(
            |_: On<MiningPulse>, time: Res<Time<Virtual>>, mut proof: ResMut<MiningProof>| {
                proof.last_pulse_at = time.elapsed_secs();
            },
        );
        // `Last`: after the scene spawner and every pose writer, so what
        // these read is what the frame renders.
        app.add_systems(Last, (assert_emitter_frame, record_mining_proof).chain());
        app.add_plugins(mining_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.init_resource::<PulseLog>();
    app.add_observer(|pulse: On<MiningPulse>, mut log: ResMut<PulseLog>| {
        log.0.push(pulse.outcome);
    });
    app.init_resource::<PulseSounds>();
    app.add_observer(
        |sfx: On<PlaySfx>, server: Res<AssetServer>, mut sounds: ResMut<PulseSounds>| {
            if server
                .get_path(sfx.handle.id())
                .is_some_and(|path| path.path().ends_with(PULSE_SOUND))
            {
                sounds.0.push(sfx.route);
            }
        },
    );
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    let player = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLAYER_ID.to_string(),
            name: "Player Ship".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: BTreeMap::from([(
                    "mining_beam".to_string(),
                    vec![InputSource::Keyboard(KeyCode::KeyV)],
                )]),
            }),
            allegiance: None,
            design: ShipDesignSource::Inline(kit::catalog_ship(&ships, "block_line_warship")),
            inventory: ShipInventoryStock::new([]),
            ..default()
        }),
    });
    let rock = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ROCK_ID.to_string(),
            name: "Ore Rock".to_string(),
            position: Meters3::from_engine(ROCK_CENTRE),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: ROCK_RADIUS,
            texture: game_assets.asteroid_texture.clone().into(),
            kind: KIND_ROCK.into(),
            destroy_sound: None,
            initial_velocity: MetersPerSecond3::ZERO,
            lock_signature: None,
            seed: Some(ROCK_SEED),
        }),
    });
    let subject = Meters3::from_engine((EMITTER_FACE + ROCK_NEAR_FACE) * 0.5);
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "The line warship's mining beam on one locked rock.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![player, rock],
                ThreePointRig::around("photo", subject, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "mining_beam".to_string(),
            "Mining Beam".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// The emitter's door and tip progress (1 is stowed) and whether it has a hit.
#[cfg(feature = "debug")]
fn emitter_state(world: &World) -> Option<(f32, f32, bool)> {
    let mut emitters = world
        .try_query_filtered::<(&SectionAnimations, Has<MiningBeamHit>), With<MiningEmitter>>()?;
    let (animations, hit) = emitters.iter(world).next()?;
    Some((
        animations.cue_progress(SectionAnimationCue::StowDoors)?,
        animations.cue_progress(SectionAnimationCue::StowLift)?,
        hit,
    ))
}

#[cfg(feature = "debug")]
fn when(
    check: impl Fn(&World) -> bool + Send + Sync + 'static,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(check)
}

#[cfg(feature = "debug")]
fn paid(world: &World) -> usize {
    world
        .resource::<PulseLog>()
        .0
        .iter()
        .filter(|outcome| outcome.is_ok_and(|corners| corners > 0))
        .count()
}

/// The emitter's door and tip scene nodes with their poses, in entity order;
/// empty until its scene has spawned.
#[cfg(feature = "debug")]
fn emitter_nodes(world: &mut World) -> Vec<(Entity, Transform)> {
    let mut state = bevy::ecs::system::SystemState::<(
        Query<Entity, With<MiningEmitter>>,
        Query<&Children>,
        Query<(&Name, &Transform)>,
    )>::new(world);
    let (emitters, children, named) = state.get(world).expect("queries always validate");
    let Some(emitter) = emitters.iter().next() else {
        return Vec::new();
    };
    let mut nodes: Vec<(Entity, Transform)> = children
        .iter_descendants(emitter)
        .filter_map(|node| {
            let (name, transform) = named.get(node).ok()?;
            EMITTER_NODE_PREFIXES
                .iter()
                .any(|prefix| name.as_str().starts_with(prefix))
                .then_some((node, *transform))
        })
        .collect();
    nodes.sort_by_key(|(node, _)| *node);
    nodes
}

/// Whether two node pose lists are the same nodes at the same poses.
#[cfg(feature = "debug")]
fn same_pose(a: &[(Entity, Transform)], b: &[(Entity, Transform)]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|((a_node, a_pose), (b_node, b_pose))| {
            a_node == b_node
                && a_pose.translation.abs_diff_eq(b_pose.translation, 1e-4)
                && a_pose.rotation.abs_diff_eq(b_pose.rotation, 1e-4)
        })
}

/// The locked rock's solid corners, once its field exists.
#[cfg(feature = "debug")]
fn solid_corners(world: &mut World) -> Option<u32> {
    let mut fields = world.query::<&AsteroidField>();
    let field = fields.iter(world).next()?.solid();
    Some((field.solid_volume() / field.cell_size().powi(3)).round() as u32)
}

/// Corners the pulse log says were flipped.
#[cfg(feature = "debug")]
fn logged_corners(world: &World) -> u32 {
    world
        .resource::<PulseLog>()
        .0
        .iter()
        .filter_map(|outcome| outcome.ok())
        .sum()
}

/// Cap a frame's game time while the tracks move, or give the cap back.
#[cfg(feature = "debug")]
fn hold_track_frame_step(world: &mut World, held: bool) {
    let current = world.resource::<Time<Virtual>>().max_delta();
    let free = *world
        .resource_mut::<MiningProof>()
        .free_max_delta
        .get_or_insert(current);
    world
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(if held { TRACK_FRAME_STEP } else { free });
}

/// Fail on any emitter frame the sequence forbids: a tip off its stop while
/// the doors are not fully open, or a deployed emitter with either part short
/// of out.
#[cfg(feature = "debug")]
fn assert_emitter_frame(
    q_emitters: Query<(Ref<MiningEmitter>, &SectionAnimations, Has<MiningBeamHit>)>,
) {
    for (emitter, animations, hit) in &q_emitters {
        let doors = animations
            .cue_progress(SectionAnimationCue::StowDoors)
            .expect("the emitter has a door track");
        let tip = animations
            .cue_progress(SectionAnimationCue::StowLift)
            .expect("the emitter has a tip track");
        if emitter.is_added() {
            // The ship spawns after the arm ran this frame, so the snap lands
            // next frame, before the scene resolves; the nodes are what render.
            info!("mining_beam: emitter spawned at doors {doors}, tip {tip}, hit {hit}");
        }
        assert!(
            doors == 0.0 || tip == 1.0,
            "the tip moved while the doors were at {doors}: tip {tip}"
        );
        if emitter.is_deployed() {
            assert_eq!((doors, tip), (0.0, 0.0), "a deployed emitter is fully out");
        }
    }
}

/// Record the emitter's first rendered pose and shoot that frame, and record
/// the rock's solid corners when its field first exists.
#[cfg(feature = "debug")]
fn record_mining_proof(world: &mut World) {
    if world.resource::<MiningProof>().first_pose.is_none() {
        // Posed each frame until the nodes spawn, so the frame they first
        // render in is already framed; the pose lands a frame after it is set.
        frame_emitter(world);
        let nodes = emitter_nodes(world);
        if !nodes.is_empty() {
            let state = emitter_state(world);
            info!(
                "mining_beam: first rendered emitter frame {state:?}, {} node(s)",
                nodes.len()
            );
            assert_eq!(
                state,
                Some((1.0, 1.0, false)),
                "the emitter's nodes first render stowed"
            );
            world.resource_mut::<MiningProof>().first_pose = Some(nodes);
            shoot(world, "mining-beam-stowed.png");
        }
    }
    if world.resource::<MiningProof>().seeded_corners.is_some() {
        return;
    }
    if let Some(corners) = solid_corners(world) {
        let carved = logged_corners(world);
        info!("mining_beam: seeded field holds {corners} solid corner(s)");
        assert_eq!(carved, 0, "a pulse carved the field before it was counted");
        world.resource_mut::<MiningProof>().seeded_corners = Some(corners);
    }
}

/// Frame the rock's near face from above and to starboard, off the beam line
/// the canisters drift along, so the crater shows past them.
#[cfg(feature = "debug")]
fn frame_rock(world: &mut World) {
    pose_camera(
        world,
        Meters3::from_engine(ROCK_NEAR_FACE + Vec3::new(5.0, 4.0, 3.0)),
        Meters3::from_engine(ROCK_NEAR_FACE),
    );
}

/// Frame the emitter face close, from ahead, above and to starboard.
#[cfg(feature = "debug")]
fn frame_emitter(world: &mut World) {
    pose_camera(
        world,
        Meters3::from_engine(EMITTER_FACE + Vec3::new(2.4, 1.5, -2.4)),
        Meters3::from_engine(EMITTER_FACE),
    );
}

/// Frame the bow, the beam and the rock's near face from starboard, level
/// with the beam, so the gap between the ray and the hull below it shows.
#[cfg(feature = "debug")]
fn frame_beam(world: &mut World) {
    let subject = (EMITTER_FACE + ROCK_NEAR_FACE) * 0.5;
    pose_camera(
        world,
        Meters3::from_engine(subject + Vec3::new(9.0, 0.0, 0.0)),
        Meters3::from_engine(subject),
    );
}

/// Frame the beam's hit close, from above and to starboard of the beam, so
/// the sparks thrown back up the beam show against space.
#[cfg(feature = "debug")]
fn frame_hit(world: &mut World) {
    let hit = world
        .try_query::<&MiningBeamHit>()
        .and_then(|mut hits| hits.iter(world).next().copied())
        .expect("the beam holds a hit while the key is held");
    pose_camera(
        world,
        Meters3::from_engine(hit.at + Vec3::new(3.5, 2.5, 4.0)),
        Meters3::from_engine(hit.at + Vec3::new(0.0, 0.0, 1.0)),
    );
}

/// Frame the emitter face, the beam and its hit together, from above, to
/// starboard and ahead of the face.
#[cfg(feature = "debug")]
fn frame_cycle(world: &mut World) {
    pose_camera(
        world,
        Meters3::from_engine(Vec3::new(7.0, 6.0, -12.0)),
        Meters3::from_engine(Vec3::new(1.0, 1.0, -10.3)),
    );
}

/// Refuse a shot of a warship the rock has pushed off its mark.
#[cfg(feature = "debug")]
fn assert_parked(world: &mut World) {
    let player = kit::ship_root(world, PLAYER_ID).expect("the warship spawned");
    let at = world.get::<GlobalTransform>(player).unwrap().translation();
    assert!(
        at.length() < PARKED_DRIFT,
        "the warship drifted to {at:?}; the rock touched it"
    );
}

/// Press or release whatever the warship's mining section is bound to, read
/// live rather than hard-coded, so a rebind of the section still drives the
/// real key.
#[cfg(feature = "debug")]
fn drive_mine_key(world: &mut World, phase: InputPhase) {
    let source = world
        .query::<&SpaceshipMiningInputBinding>()
        .iter(world)
        .next()
        .and_then(|binding| binding.0.first().copied())
        .expect("the warship's mining section is bound to a key");
    dispatch::press_source(world, source, phase);
}

#[cfg(feature = "debug")]
fn press_mine_key(world: &mut World) {
    drive_mine_key(world, InputPhase::Press);
}

#[cfg(feature = "debug")]
fn release_mine_key(world: &mut World) {
    drive_mine_key(world, InputPhase::Release);
}

#[cfg(feature = "debug")]
fn set_paused(world: &mut World, paused: bool) {
    let mut time = world.resource_mut::<Time<Virtual>>();
    if paused {
        time.pause();
    } else {
        time.unpause();
    }
}

/// Shoot the first rendered frame, lock the rock and shoot it, hold the key
/// and shoot the doors, the deploy and the hit, release and shoot the
/// retract, wait for the ore to leave the rock, shoot the carve, check the
/// pulse log against the rock and the canisters, then record one more press
/// and release as the site's loop.
#[cfg(feature = "debug")]
fn mining_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the rock and the warship")
        .enter(GameStates::Loading)
        .until(and(player_ship_present(), scenario_camera_present()))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the first rendered emitter frame")
        .until(and(
            when(|world| world.resource::<MiningProof>().first_pose.is_some()),
            shot_written("mining-beam-stowed.png"),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("lock the rock")
        .on_enter(|world| {
            let player = kit::ship_root(world, PLAYER_ID).expect("the warship spawned");
            let rock = {
                let mut rocks = world.query_filtered::<(Entity, &EntityId), With<AsteroidMarker>>();
                rocks
                    .iter(world)
                    .find(|(_, id)| id.0 == ROCK_ID)
                    .map(|(entity, _)| entity)
                    .expect("the rock spawned")
            };
            world.entity_mut(player).insert(TravelLock(Some(rock)));
            frame_rock(world);
        })
        .until(and(scenario_is_built(), frames(10)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the rock before the beam")
        .on_enter(|world| shoot(world, "mining-beam-rock-before.png"))
        .until(shot_written("mining-beam-rock-before.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("hold the mine key until the doors part")
        .on_enter(|world| {
            hold_track_frame_step(world, true);
            frame_emitter(world);
            press_mine_key(world);
        })
        .until(when(|world| {
            emitter_state(world)
                .is_some_and(|(doors, tip, _)| doors > 0.0 && doors <= DOORS_PARTING && tip == 1.0)
        }))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the doors parting")
        .on_enter(|world| {
            set_paused(world, true);
            shoot(world, "mining-beam-doors.png");
        })
        .until(shot_written("mining-beam-doors.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("wait for the doors to open")
        .on_enter(|world| {
            // Paused since the frame after the shot, so this is its pose.
            let shot = emitter_state(world);
            info!("mining_beam: doors shot pose {shot:?}");
            assert!(
                shot.is_some_and(|(doors, tip, _)| doors > 0.0 && doors < 1.0 && tip == 1.0),
                "the doors shot shows the doors parting with the tip in: {shot:?}"
            );
            set_paused(world, false);
        })
        .until(when(|world| {
            emitter_state(world).is_some_and(|(doors, _, _)| doors == 0.0)
        }))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the deploy")
        .on_enter(|world| {
            set_paused(world, true);
            shoot(world, "mining-beam-deploying.png");
        })
        .until(shot_written("mining-beam-deploying.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("wait for the beam")
        .on_enter(|world| {
            let shot = emitter_state(world);
            info!("mining_beam: deploy shot pose {shot:?}");
            assert!(
                shot.is_some_and(|(doors, tip, _)| doors == 0.0 && tip > 0.0 && tip < 1.0),
                "the deploy shot shows the doors open with the tip partway out: {shot:?}"
            );
            set_paused(world, false);
        })
        .until(when(|world| {
            emitter_state(world).is_some_and(|(doors, tip, hit)| doors == 0.0 && tip == 0.0 && hit)
        }))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the deployed emitter")
        .on_enter(|world| {
            assert_parked(world);
            hold_track_frame_step(world, false);
            let nodes = emitter_nodes(world);
            world.resource_mut::<MiningProof>().deployed_pose = Some(nodes);
            shoot(world, "mining-beam-deployed.png");
        })
        .until(shot_written("mining-beam-deployed.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("wait for paying pulses")
        .on_enter(frame_beam)
        .until(when(|world| paid(world) >= PAID_PULSES))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the hit")
        .on_enter(|world| {
            assert_parked(world);
            assert!(
                emitter_state(world).is_some_and(|(_, _, hit)| hit),
                "the beam holds a hit while it pays"
            );
            // Paused until the release, so a slow screenshot readback cannot
            // let more pulses fire before the key comes up.
            set_paused(world, true);
            let pulses = world.resource::<PulseLog>().0.len();
            info!("mining_beam: hit shot after {pulses} pulse(s)");
            shoot(world, "wiki-section-mining-beam.png");
        })
        .until(shot_written("wiki-section-mining-beam.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("wait for the next pulse's sparks")
        .on_enter(|world| {
            let pulses = world.resource::<PulseLog>().0.len();
            world.resource_mut::<MiningProof>().pulses_before_sparks = Some(pulses);
            frame_hit(world);
            set_paused(world, false);
        })
        .until(when(|world| {
            let proof = world.resource::<MiningProof>();
            let now = world.resource::<Time<Virtual>>().elapsed_secs();
            proof
                .pulses_before_sparks
                .is_some_and(|before| world.resource::<PulseLog>().0.len() > before)
                && now - proof.last_pulse_at >= SPARKS_AFTER_PULSE_SECS
        }))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the sparks")
        .on_enter(|world| {
            set_paused(world, true);
            let log = world.resource::<PulseLog>().0.clone();
            info!("mining_beam: sparks shot after {} pulse(s)", log.len());
            assert!(
                log.last().is_some_and(Result::is_ok),
                "the sparks shot follows a pulse that passed: {log:?}"
            );
            shoot(world, "mining-beam-sparks.png");
        })
        .until(shot_written("mining-beam-sparks.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("release the mine key until the emitter is shut")
        .on_enter(|world| {
            let pulses = world.resource::<PulseLog>().0.len();
            info!("mining_beam: release after {pulses} pulse(s)");
            hold_track_frame_step(world, true);
            release_mine_key(world);
            set_paused(world, false);
            frame_emitter(world);
        })
        .until(when(|world| {
            emitter_state(world) == Some((1.0, 1.0, false))
        }))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the retracted emitter")
        .on_enter(|world| {
            hold_track_frame_step(world, false);
            let nodes = emitter_nodes(world);
            let proof = world.resource::<MiningProof>();
            let first = proof
                .first_pose
                .as_deref()
                .expect("the first pose was recorded");
            let deployed = proof
                .deployed_pose
                .as_deref()
                .expect("the deployed pose was recorded");
            assert!(
                same_pose(first, &nodes),
                "the retracted nodes match their first rendered pose: {first:?} vs {nodes:?}"
            );
            assert!(
                !same_pose(first, deployed),
                "the first rendered pose differs from the deployed one: {first:?}"
            );
            shoot(world, "mining-beam-retracted.png");
        })
        .until(shot_written("mining-beam-retracted.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("wait for the ore to leave the rock")
        .on_enter(frame_rock)
        .until(when(|world| {
            // A pulse's ore is owed until its remesh lands, then queued until
            // its canister is born.
            !world_has::<MinedOre>(world) && !world_has::<MinedCanisterQueue>(world)
        }))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the carved rock")
        .on_enter(|world| shoot(world, "wiki-section-mining-beam-carve.png"))
        .until(shot_written("wiki-section-mining-beam-carve.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("check the pulse log against the rock and the canisters")
        .on_enter(|world| {
            let log = world.resource::<PulseLog>().0.clone();
            info!("mining_beam: pulse log {log:?}");
            assert!(
                log.iter().all(Result::is_ok),
                "a pulse was refused: {log:?}"
            );
            assert!(paid(world) >= PAID_PULSES, "too few paying pulses: {log:?}");
            let flipped = logged_corners(world);
            let seeded = world
                .resource::<MiningProof>()
                .seeded_corners
                .expect("the seeded field was counted");
            let left = solid_corners(world).expect("the rock keeps its field");
            let canisters: Vec<u32> = world
                .query::<&CargoCanister>()
                .iter(world)
                .map(|canister| {
                    canister
                        .stacks()
                        .filter(|(item, _)| item.as_str() == ITEM_STONE_ORE)
                        .map(|(_, count)| count)
                        .sum()
                })
                .collect();
            let player = kit::ship_root(world, PLAYER_ID).expect("the warship spawned");
            let hold = world
                .get::<ShipInventory>(player)
                .expect("the warship has a hold")
                .count(&ITEM_STONE_ORE.into());
            let in_canisters: u32 = canisters.iter().sum();
            info!(
                "mining_beam: {flipped} corner(s) flipped, the field lost {}, \
                 {} canister(s) hold {canisters:?} stone ore, the hold {hold}",
                seeded - left,
                canisters.len()
            );
            assert_eq!(
                seeded - left,
                flipped,
                "the field lost what the pulses flipped"
            );
            assert_eq!(
                in_canisters + hold,
                flipped,
                "every flipped corner is one stone ore in a canister or the hold"
            );
            let sounds = world.resource::<PulseSounds>().0.clone();
            info!(
                "mining_beam: {} pulse sound(s) for {} pulse(s), routes {sounds:?}",
                sounds.len(),
                log.len()
            );
            assert_eq!(sounds.len(), log.len(), "one pulse sound per pulse");
            assert!(
                sounds.iter().all(|route| *route == AudioRoute::Hull),
                "the player's pulses are heard through the hull: {sounds:?}"
            );
        })
        .until(frames(1))
        .add()
        .step("open the loop on the shut emitter")
        .on_enter(|world| {
            frame_cycle(world);
            let pulses = world.resource::<PulseLog>().0.len();
            world.resource_mut::<MiningProof>().pulses_before_loop = Some(pulses);
            loop_start(world, MINING_LOOP);
        })
        .until(elapsed(LOOP_HOLD_SECS))
        .add()
        .step("hold the mine key through the loop's pulses")
        .on_enter(press_mine_key)
        .until(when(|world| {
            world
                .resource::<MiningProof>()
                .pulses_before_loop
                .is_some_and(|before| world.resource::<PulseLog>().0.len() >= before + LOOP_PULSES)
        }))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("release and wait for the emitter to shut")
        .on_enter(release_mine_key)
        .until(when(|world| {
            emitter_state(world) == Some((1.0, 1.0, false))
        }))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("hold the shut emitter")
        .until(elapsed(LOOP_HOLD_SECS))
        .add()
        .step("close the loop")
        .on_enter(|world| {
            let log = world.resource::<PulseLog>().0.clone();
            assert!(
                log.iter().all(Result::is_ok),
                "a loop pulse was refused: {log:?}"
            );
            loop_end(world, MINING_LOOP);
        })
        .until(loop_written(MINING_LOOP))
        .deadline(STEP_DEADLINE_SECS)
        .add()
}

#[cfg(feature = "debug")]
fn world_has<T: Component>(world: &World) -> bool {
    world
        .try_query_filtered::<(), With<T>>()
        .is_some_and(|mut query| query.iter(world).next().is_some())
}
