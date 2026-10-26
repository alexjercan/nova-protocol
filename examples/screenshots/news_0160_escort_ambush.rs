//! news_0160_escort_ambush: the v0.16.0 news escort loops - a held Escort's
//! raiders springing on its hauler with the escort in formation, and the
//! hauler reaching its mark.
//!
//! The run is the escort range's walk ([`located_jobs::walk_to_done`]) on its
//! pinned seed as the armed `military` start: find a living station posting
//! an Escort, dock, accept it off the Jobs tab, undock, fly GOTO to the
//! waiting hauler, hold formation on its leg and fight the ambush with the
//! hull's own turrets until the hauler reaches its mark. Nothing teleports
//! the hauler, the raiders or the player inside a loop. The walk asserts its
//! eight escort claims; this producer adds two loops cut off world state and
//! asserts each phase they show:
//!
//! | loop | opens | closes |
//! | - | - | - |
//! | `news-0160-escort-ambush` | the leg is flown, the player is in formation, no raider has sprung and one stands inside [`AMBUSH_LEAD_IN`] of the hauler or the player; the hauler's chip reads `JOB-1 Escort` | a raider sprang inside the loop, a round from the player's point-defense mounts damaged a raider, [`AMBUSH_HIT_HOLD_FRAMES`] after that hit, the chip still on the hauler |
//! | `news-0160-escort-done` | the hauler is inside [`DONE_LEAD_IN`] of its mark, the job not yet done | [`DONE_HOLD_FRAMES`] after [`JobDone`]: the HUD notice reads `JOB-1 done: hand in at STN-n`, the hauler's chip is gone and the poster wears `JOB-1 Hand in` |
//!
//! A done job's chip does not change in place: the world takes `JOB-1 Escort`
//! off the hauler and puts `JOB-1 Hand in` on the posting station, and the HUD
//! posts the done notice. The spring and the done are about 30 s of play
//! apart, the rest of the hauler's leg, so they are two loops in time order,
//! each inside the default frame cap. A phase that does not happen inside
//! its loop's frames stops the run, naming it.
//!
//! Each loop poses the scenario camera wide on the player, the hauler and
//! the raider nearest them ([`LoopFraming`]); the chase camera holds between
//! the loops and after the done. The camera does not steer the ship: the
//! fight aims on its component lock.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the whole walk with its
//!   asserts, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write both loops (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! The leg is real flight: raise the 120 s completion backstop, which counts
//! play seconds on an armed run.
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0160-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   NOVA_AUTOPILOT_DEADLINE=1800 \
//!   cargo run --example news_0160_escort_ambush --features debug
//! ```

#[path = "../systems/shared/located_jobs.rs"]
pub mod located_jobs;

use std::sync::Arc;

use avian3d::prelude::Position;
use bevy::prelude::*;
use clap::Parser;
use located_jobs::{
    walk_to_done, EscortMarkers, LocatedJobMarkers, LocatedJobWalkPlugin, LocatedKindType,
};
use nova_protocol::{
    nova_debug::harness::{AutopilotPlugin, Predicate},
    prelude::*,
};

#[derive(Parser)]
#[command(name = "news_0160_escort_ambush")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the v0.16.0 escort loops: the ambush on a held Escort's leg and the hauler \
             reaching its mark",
    long_about = None
)]
struct Cli;

/// The loop of the spring and the first exchange of fire.
const AMBUSH_LOOP: &str = "news-0160-escort-ambush";

/// The loop of the hauler reaching its mark.
const DONE_LOOP: &str = "news-0160-escort-done";

/// The mark code the post names: the walk holds one job.
const MARK_CODE: &str = "JOB-1";

/// How close a waiting raider must stand to the hauler or the player before
/// the ambush loop opens: 100 m outside the raiders' 3 km engage range
/// (`RAIDER_ENGAGE_RANGE` in `nova_world_base::jobs`), a few seconds of the
/// leg before the spring. At 3.3 km the seed-2 spring came at frame 155 and
/// the player's first PDC hit at frame 560, too late for its hold under
/// [`LOOP_MAX_FRAMES`].
const AMBUSH_LEAD_IN: Meters = Meters(3_100.0);

/// How far from the hauler the player counts as in formation: GOTO parks
/// 500 m off the hull (the walk's `FORMATION`).
const FORMATION: Meters = Meters(1_500.0);

/// Frames the ambush loop holds after the player's fire first lands, so the
/// tracers and the hit read.
const AMBUSH_HIT_HOLD_FRAMES: u32 = 60;

/// How close the hauler must come to its mark before the done loop opens:
/// 600 m outside the 500 m standoff its Move stops at (the leg's order names
/// none, so `FlightSettings::arrival_standoff` applies), a few seconds of the
/// braking leg before the order completes and [`JobDone`] fires. A raised
/// standoff or a tighter lead-in opens the loop after the done, and
/// [`note_the_done`] stops the run.
const DONE_LEAD_IN: Meters = Meters(1_100.0);

/// Frames the done loop holds after [`JobDone`], inside the notice's 4 s
/// full-strength dwell.
const DONE_HOLD_FRAMES: u32 = 75;

/// The most frames either loop may record before its closing phase: under
/// the recorder's 600-frame cap, which aborts the run with no phase named.
const LOOP_MAX_FRAMES: u32 = 570;

/// Room around each framed ship's centre, so its hull, chip and brackets stay
/// inside the frame.
const FRAME_PAD: Meters = Meters(200.0);

/// How far past the subjects' bounding sphere the framing eye stands back.
const FRAME_MARGIN: f32 = 1.1;

/// Seconds the done hold gets before its deadline. The hold ends on frames
/// the cut counts and asserts; this is only the backstop for a stalled run,
/// sized for a software rasterizer on the smoke path, where it counts real
/// seconds.
const DONE_HOLD_SECS: f32 = 600.0;

/// Play seconds the done loop gets to encode once its last frame is in.
const LOOP_CLOSE_SECS: f32 = 60.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings: no player's saved world may be in the way.
    std::env::set_var(
        nova_assets::storage::CONFIG_ROOT_ENV,
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "target/example-profiles/{}-{}-{}",
            env!("CARGO_CRATE_NAME"),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is after 1970")
                .as_nanos(),
        )),
    );

    // The escort range's claims: this walk is that range's, up to the done.
    const MARKERS: LocatedJobMarkers = LocatedJobMarkers {
        accepted: "outcome: a escort job is posted and accepted",
        refused: "outcome: the hand-in is refused until the escort job is done",
        done: "outcome: the world marks the escort job done",
        paid: "outcome: the hand-in pays the escort job",
        escort: Some(EscortMarkers {
            waits: "outcome: the escort's hauler waits for the rendezvous",
            leaves: "outcome: the hauler leaves when the escort arrives",
            springs: "outcome: the raiders spring with the escort in formation",
            survives: "outcome: the escorted hauler survives the ambush to its mark",
        }),
    };

    let mut app = editor_app(true, None);
    app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
        bevy::ecs::error::panic,
    ));
    // No frame-time claim: the walk holds the job's path.
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
    app.add_plugins(LocatedJobWalkPlugin(LocatedKindType::Escort, MARKERS));
    app.add_plugins(EscortLoopsPlugin);
    app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    app.add_plugins(news_script());
    app.run()
}

/// Cuts the two loops off world state and asserts the phases they show.
struct EscortLoopsPlugin;

impl Plugin for EscortLoopsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EscortLoops::default());
        app.add_observer(count_player_pdc_hits);
        app.add_observer(note_the_done);
        app.add_systems(Update, cut_the_loops);
    }
}

/// Where the loops stand.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum LoopPhaseType {
    /// Waiting for the raiders to come inside the lead-in.
    #[default]
    BeforeAmbush,
    /// The ambush loop records.
    Ambush,
    /// The ambush loop is closed; waiting for the hauler to near its mark.
    BeforeDone,
    /// The done loop records; the walk's appended beats close it.
    Done,
    /// The done loop is closed; the walk hands in under the chase camera.
    Closed,
}

/// What the loops saw.
#[derive(Resource, Debug, Default)]
struct EscortLoops {
    phase: LoopPhaseType,
    /// The held escort, once the walk accepted it.
    job: Option<JobId>,
    /// Frames the open loop has recorded.
    frames: u32,
    /// The held escort's living raiders when the ambush loop opened.
    raiders: Vec<Entity>,
    /// Hits a round of the player's point-defense mounts landed on a raider
    /// since the ambush loop opened.
    pdc_hits: u32,
    /// The ambush loop frame a raider first engaged on.
    sprung_at: Option<u32>,
    /// The ambush loop frame a player's point-defense round first damaged a
    /// raider on.
    hit_at: Option<u32>,
    /// The done loop frame [`JobDone`] fired on for the held escort.
    done_at: Option<u32>,
    /// The open loop's camera framing.
    framing: Option<LoopFraming>,
}

/// The camera framing of an open loop: the player, the hauler and the raider
/// nearest either when the loop opened, from the side of the plane the three
/// stood in at the open. The eye follows their centre every frame and stands
/// back far enough to hold all three.
#[derive(Debug, Clone, Copy)]
struct LoopFraming {
    player: Entity,
    hauler: Entity,
    /// `None` when no raider stands at the open. A raider that falls out of
    /// the world inside the loop drops out of the frame.
    raider: Option<Entity>,
    /// Unit direction from the subjects' centre to the eye.
    side: Vec3,
}

impl LoopFraming {
    fn open(world: &World, player: Entity, hauler: Entity, raider: Option<Entity>) -> Self {
        let player_at = at(world, player).get();
        let along = at(world, hauler).get() - player_at;
        let normal = raider
            .map(|raider| along.cross(at(world, raider).get() - player_at))
            .filter(|normal| normal.length() > 1.0)
            .unwrap_or_else(|| along.normalize().any_orthonormal_vector())
            .normalize();
        let side = if normal.y < 0.0 { -normal } else { normal };
        Self {
            player,
            hauler,
            raider,
            side,
        }
    }

    /// Pose the scenario camera on where the subjects stand now.
    fn pose(&self, world: &mut World) {
        let subjects = [self.player, self.hauler]
            .into_iter()
            .map(|subject| at(world, subject).get())
            .chain(self.raider.and_then(|raider| {
                world
                    .get::<Position>(raider)
                    .map(|position| Meters3::from_engine(position.0).get())
            }))
            .collect::<Vec<_>>();
        let centre = subjects.iter().sum::<Vec3>() / subjects.len() as f32;
        let radius = subjects
            .iter()
            .map(|subject| subject.distance(centre))
            .fold(0.0, f32::max)
            + FRAME_PAD.get();
        let fov = world
            .query_filtered::<&Projection, With<ScenarioCameraMarker>>()
            .single(world)
            .map(|projection| match projection {
                Projection::Perspective(perspective) => perspective.fov,
                other => panic!("escort loops: the scenario camera is not perspective: {other:?}"),
            })
            .expect("escort loops: one scenario camera stands");
        let back = radius / (fov / 2.0).sin() * FRAME_MARGIN;
        let meters = |v: Vec3| Meters3::new(v.x, v.y, v.z);
        pose_camera(world, meters(centre + self.side * back), meters(centre));
    }
}

/// Frame the open loop on this frame's positions.
fn pose_the_framing(world: &mut World) {
    let framing = world
        .resource::<EscortLoops>()
        .framing
        .expect("escort loops: an open loop has a framing");
    framing.pose(world);
}

/// Take the framing off and hand the camera back to the chase rig.
fn release_the_framing(world: &mut World) {
    world.resource_mut::<EscortLoops>().framing = None;
    let cameras = world
        .query_filtered::<Entity, With<ScriptedCameraPose>>()
        .iter(world)
        .collect::<Vec<_>>();
    for camera in cameras {
        world.entity_mut(camera).remove::<ScriptedCameraPose>();
    }
}

/// The living raider nearest the hauler or the player.
fn nearest_raider(
    world: &World,
    raiders: &[(Entity, Option<AIBehaviorState>)],
    hauler: Entity,
    player: Entity,
) -> Option<(Entity, f32)> {
    raiders
        .iter()
        .map(|(raider, _)| {
            let distance = distance_between(world, *raider, hauler)
                .min(distance_between(world, *raider, player));
            (*raider, distance)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

/// The one player ship, if exactly one stands.
fn the_player(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

/// The entity wearing generated id `id`, if one stands.
fn body(world: &World, id: &GeneratedBodyId) -> Option<Entity> {
    world
        .try_query::<(Entity, &GeneratedBodyId)>()?
        .iter(world)
        .find(|(_, worn)| *worn == id)
        .map(|(entity, _)| entity)
}

/// The held escort, once the walk accepted it.
fn held_escort(world: &World) -> Option<Job> {
    world
        .resource::<Jobs>()
        .held()
        .iter()
        .find(|job| matches!(job.kind, JobType::Escort { .. }))
        .cloned()
}

/// The held escort's hauler, standing.
fn the_hauler(world: &World, job: &Job) -> Entity {
    let id = job
        .located_body()
        .expect("escort loops: an Escort names its hauler");
    body(world, id).unwrap_or_else(|| panic!("escort loops: no hauler '{id}' stands"))
}

fn at(world: &World, entity: Entity) -> Meters3 {
    Meters3::from_engine(
        world
            .get::<Position>(entity)
            .unwrap_or_else(|| panic!("escort loops: {entity:?} has no position"))
            .0,
    )
}

fn distance_between(world: &World, a: Entity, b: Entity) -> f32 {
    at(world, a).distance(at(world, b)).get()
}

/// The held escort's standing raiders that are not neutralized, with each
/// one's behavior state.
fn living_raiders(world: &World, job: &Job) -> Vec<(Entity, Option<AIBehaviorState>)> {
    job.raider_ids()
        .iter()
        .filter_map(|id| body(world, id))
        .filter(|raider| world.get::<NeutralizedMarker>(*raider).is_none())
        .map(|raider| (raider, world.get::<AIBehaviorState>(raider).copied()))
        .collect()
}

/// The label texts of every objective chip on `target`.
fn chip_texts(world: &World, target: Entity) -> Vec<String> {
    let Some(mut labels) =
        world.try_query_filtered::<(&Text, &ChildOf), With<AnchoredChipLabelMarker>>()
    else {
        return Vec::new();
    };
    labels
        .iter(world)
        .filter(|(_, ChildOf(chip))| {
            world
                .get::<ChildOf>(*chip)
                .and_then(|ChildOf(layer)| {
                    world
                        .get::<ObjectiveMarkerChipHudMarker>(*layer)
                        .and(world.get::<AnchoredChipTarget>(*layer))
                })
                .is_some_and(|anchored| **anchored == target)
        })
        .map(|(text, _)| text.0.clone())
        .collect()
}

/// Assert `target` wears the objective tag `label` and exactly one chip that
/// reads it.
fn assert_the_chip(world: &World, target: Entity, label: &str, what: &str) {
    let tag = world
        .get::<ObjectiveMarkerTarget>(target)
        .map(|tag| tag.label.clone());
    assert_eq!(
        tag.as_deref(),
        Some(label),
        "escort loops: the {what} wears the objective tag {tag:?}, not '{label}'"
    );
    let texts = chip_texts(world, target);
    assert!(
        texts.len() == 1 && texts[0].starts_with(label),
        "escort loops: the {what}'s chips read {texts:?}, not one '{label}' chip"
    );
}

/// Count each hit a round of the player's point-defense mounts lands on a
/// raider: the damage's source is a turret round the player owns, and what
/// reaches the raider's root is above zero. Torpedo blasts, rams and other
/// shooters' rounds do not count.
fn count_player_pdc_hits(
    damage: On<HealthApplyDamage>,
    rounds: Query<&ProjectileOwner, With<TurretBulletProjectileMarker>>,
    players: Query<(), With<PlayerSpaceshipMarker>>,
    mut loops: ResMut<EscortLoops>,
) {
    if loops.phase != LoopPhaseType::Ambush
        || damage.amount <= 0.0
        || !loops.raiders.contains(&damage.entity)
    {
        return;
    }
    let player_round = damage
        .source
        .and_then(|source| rounds.get(source).ok())
        .is_some_and(|ProjectileOwner(owner)| players.contains(*owner));
    if player_round {
        loops.pdc_hits += 1;
    }
}

/// Note the frame [`JobDone`] fires on for the held escort.
///
/// # Panics
///
/// When the escort turns done outside the done loop: the loop missed it.
fn note_the_done(done: On<JobDone>, jobs: Res<Jobs>, mut loops: ResMut<EscortLoops>) {
    let escort = jobs
        .job(&done.id)
        .is_some_and(|job| matches!(job.kind, JobType::Escort { .. }));
    if !escort {
        return;
    }
    assert_eq!(
        loops.phase,
        LoopPhaseType::Done,
        "escort loops: '{}' turned done outside the done loop",
        done.id
    );
    loops.done_at = Some(loops.frames);
    info!(
        "escort loops: '{}' is done at done-loop frame {}",
        done.id, loops.frames
    );
}

/// One frame of the cut: open the ambush loop at the lead-in, close it after
/// the first exchange of fire, and open the done loop as the hauler nears
/// its mark. The walk's appended beats close the done loop.
fn cut_the_loops(world: &mut World) {
    let phase = world.resource::<EscortLoops>().phase;
    if phase == LoopPhaseType::Closed {
        return;
    }
    if phase == LoopPhaseType::Done {
        pose_the_framing(world);
        world.resource_mut::<EscortLoops>().frames += 1;
        let loops = world.resource::<EscortLoops>();
        // The hold after the done must fit under the cap too.
        let budget = LOOP_MAX_FRAMES - DONE_HOLD_FRAMES;
        assert!(
            loops.done_at.is_some() || loops.frames <= budget,
            "escort loops: the hauler reached no mark in {budget} done-loop frames"
        );
        return;
    }
    let jobs = world.resource::<Jobs>().clone();
    if let Some(id) = &world.resource::<EscortLoops>().job {
        // A dropped job is no longer held, so the lookup below would wait
        // on it forever.
        assert!(
            !jobs.is_dropped(id),
            "escort loops: '{id}' was lost before its loops closed"
        );
    }
    let Some(job) = held_escort(world) else {
        return;
    };
    world.resource_mut::<EscortLoops>().job = Some(job.id.clone());
    let Some((mark, started)) = jobs
        .legs()
        .find(|(id, _)| **id == job.id)
        .map(|(_, leg)| (leg.mark, leg.started))
    else {
        return;
    };
    if !started {
        return;
    }
    let Some(player) = the_player(world) else {
        return;
    };
    let hauler = the_hauler(world, &job);
    let raiders = living_raiders(world, &job);
    let sprung = raiders
        .iter()
        .any(|(_, state)| *state == Some(AIBehaviorState::Engage));
    match phase {
        LoopPhaseType::BeforeAmbush => {
            let Some((nearest_entity, nearest)) = nearest_raider(world, &raiders, hauler, player)
            else {
                return;
            };
            if nearest > AMBUSH_LEAD_IN.get() {
                return;
            }
            assert!(
                !sprung,
                "escort loops: a raider sprang before the ambush loop opened"
            );
            let formation = distance_between(world, player, hauler);
            assert!(
                formation <= FORMATION.get(),
                "escort loops: the player is {formation:.0} m off the hauler at the lead-in, out \
                 of formation"
            );
            assert_eq!(
                jobs.mark_code(&job.id).as_deref(),
                Some(MARK_CODE),
                "escort loops: the held escort's mark code"
            );
            assert_the_chip(world, hauler, &format!("{MARK_CODE} Escort"), "hauler");
            let raiders = raiders.iter().map(|(raider, _)| *raider).collect();
            hide_status_bar(world);
            let framing = LoopFraming::open(world, player, hauler, Some(nearest_entity));
            framing.pose(world);
            loop_start(world, AMBUSH_LOOP);
            let mut loops = world.resource_mut::<EscortLoops>();
            loops.phase = LoopPhaseType::Ambush;
            loops.frames = 0;
            loops.raiders = raiders;
            loops.framing = Some(framing);
            info!(
                "escort loops: the ambush loop opens, a raider {nearest:.0} m out, the player \
                 {formation:.0} m off the hauler"
            );
        }
        LoopPhaseType::Ambush => {
            pose_the_framing(world);
            let (frame, closing) = {
                let mut loops = world.resource_mut::<EscortLoops>();
                loops.frames += 1;
                let frame = loops.frames;
                if sprung && loops.sprung_at.is_none() {
                    loops.sprung_at = Some(frame);
                    info!("escort loops: a raider springs at ambush-loop frame {frame}");
                }
                if loops.pdc_hits > 0 && loops.hit_at.is_none() {
                    loops.hit_at = Some(frame);
                    info!("escort loops: a player PDC round damages a raider at ambush-loop frame {frame}");
                }
                let closing = loops.sprung_at.is_some()
                    && loops
                        .hit_at
                        .is_some_and(|hit| frame >= hit + AMBUSH_HIT_HOLD_FRAMES);
                assert!(
                    closing || frame <= LOOP_MAX_FRAMES,
                    "escort loops: the ambush loop ran {LOOP_MAX_FRAMES} frames: spring at {:?}, \
                     {} player PDC hit(s) on a raider, first at {:?}",
                    loops.sprung_at,
                    loops.pdc_hits,
                    loops.hit_at
                );
                (frame, closing)
            };
            if !closing {
                return;
            }
            assert_the_chip(world, hauler, &format!("{MARK_CODE} Escort"), "hauler");
            loop_end(world, AMBUSH_LOOP);
            release_the_framing(world);
            let mut loops = world.resource_mut::<EscortLoops>();
            loops.phase = LoopPhaseType::BeforeDone;
            info!(
                "escort loops: PASS the ambush loop closes after {frame} frames: spring at {:?}, \
                 {} player PDC hit(s) on a raider, first at {:?}",
                loops.sprung_at, loops.pdc_hits, loops.hit_at
            );
        }
        LoopPhaseType::BeforeDone => {
            let to_mark = at(world, hauler).distance(mark).get();
            if to_mark > DONE_LEAD_IN.get() {
                return;
            }
            assert!(
                loop_written(AMBUSH_LOOP)(&*world),
                "escort loops: the ambush loop is not written when the done loop opens"
            );
            assert!(
                !jobs.is_done(&job.id),
                "escort loops: '{}' is done before the done loop opened",
                job.id
            );
            assert_the_chip(world, hauler, &format!("{MARK_CODE} Escort"), "hauler");
            let raider = nearest_raider(world, &raiders, hauler, player).map(|(raider, _)| raider);
            let framing = LoopFraming::open(world, player, hauler, raider);
            framing.pose(world);
            loop_start(world, DONE_LOOP);
            let mut loops = world.resource_mut::<EscortLoops>();
            loops.phase = LoopPhaseType::Done;
            loops.frames = 0;
            loops.framing = Some(framing);
            info!("escort loops: the done loop opens, the hauler {to_mark:.0} m off its mark");
        }
        LoopPhaseType::Done | LoopPhaseType::Closed => unreachable!("handled above"),
    }
}

/// Assert the done as the player sees it: the notice, the hauler's chip
/// gone and the hand-in chip on the posting station.
fn assert_the_done_on_screen(world: &mut World) {
    let (done_at, frames) = {
        let loops = world.resource::<EscortLoops>();
        let done_at = loops
            .done_at
            .expect("escort loops: JobDone fired inside the done loop");
        (done_at, loops.frames)
    };
    let job = held_escort(world).expect("escort loops: the done escort is still held");
    let station = world
        .resource::<Chart>()
        .station_code(&job.target)
        .unwrap_or_else(|| panic!("escort loops: the poster '{}' is not charted", job.target));
    let notice = format!("{MARK_CODE} done: hand in at {station}");
    let shown = world
        .query_filtered::<(&Text, &TextColor), With<HudNoticeText>>()
        .iter(world)
        .map(|(text, color)| (text.0.clone(), color.0.alpha()))
        .collect::<Vec<_>>();
    assert!(
        matches!(shown.as_slice(), [(text, alpha)] if *text == notice && *alpha > 0.0),
        "escort loops: the HUD notice reads {shown:?}, not '{notice}'"
    );
    let hauler = the_hauler(world, &job);
    assert!(
        world.get::<ObjectiveMarkerTarget>(hauler).is_none()
            && chip_texts(world, hauler).is_empty(),
        "escort loops: the done hauler still wears its chip: {:?}",
        chip_texts(world, hauler)
    );
    let poster = body(world, &job.target)
        .unwrap_or_else(|| panic!("escort loops: the poster '{}' does not stand", job.target));
    assert_the_chip(world, poster, &format!("{MARK_CODE} Hand in"), "poster");
    info!(
        "escort loops: PASS the done loop holds {} frames after the done at frame {done_at}: \
         '{notice}', the hauler's chip gone, '{MARK_CODE} Hand in' on {station}",
        frames - done_at
    );
}

fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
    Arc::new(check)
}

/// The escort walk through the done, then the done loop's hold and close.
fn news_script() -> AutopilotPlugin<GameStates> {
    walk_to_done(LocatedKindType::Escort)
        .step("escort loops: hold the done in frame")
        .until(when(|world| {
            let loops = world.resource::<EscortLoops>();
            loops.phase == LoopPhaseType::Done
                && loops
                    .done_at
                    .is_some_and(|at| loops.frames >= at + DONE_HOLD_FRAMES)
        }))
        .deadline(DONE_HOLD_SECS)
        .add()
        .step("escort loops: assert the done on screen")
        .on_enter(assert_the_done_on_screen)
        .add()
        .step("escort loops: close the done loop")
        .on_enter(|world: &mut World| {
            loop_end(world, DONE_LOOP);
            release_the_framing(world);
            world.resource_mut::<EscortLoops>().phase = LoopPhaseType::Closed;
        })
        .until(loop_written(DONE_LOOP))
        .deadline(LOOP_CLOSE_SECS)
        .add()
}
