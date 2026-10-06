//! loop_rcs_magazine: the RCS magazine chip drains one pip at a time while
//! the thrusters shove the hull side to side, holds empty for a beat, and
//! refills once the push lets go.
//!
//! The push flips sign on a fixed cadence instead of holding one way: a full
//! magazine is 300 m/s of delta-v, and one long slide crosses the hollow's
//! 480 m clearance and grinds the hull into the rock shell, where the wall,
//! not the empty magazine, holds the speed still. The shuffle spends the same
//! delta-v in place, and the script fails if the hull touches anything.
//!
//! Real player input only: the Shift `rcs_modifier` held through
//! `press_action`/`release_action`, and a `MouseMotion` fed every frame the
//! modifier is down, the same idiom `lesson_flight_basics` uses for its RCS
//! sheet. The chip, the pips and the magazine itself
//! (`RcsBudget`/`FlightSettings::rcs_budget`) are the shipped flight HUD and
//! flight model, not a pose.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - fly the whole script, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also record the loop (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/news-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example loop_rcs_magazine --features debug
//! ```

#[path = "shared/hollow.rs"]
mod hollow;
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{lesson_chase_plugin, LessonChase};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "loop_rcs_magazine")]
#[command(version = "1.0.0")]
#[command(about = "Record the RCS magazine chip draining and refilling")]
struct Cli;

/// The capture file name: `news-0150-rcs-magazine.webm`.
#[cfg(feature = "debug")]
const LOOP_NAME: &str = "news-0150-rcs-magazine";

/// Where the eye rides: the overhead chase from `lesson_flight_basics`'s RCS
/// sheet, so the side-to-side push crosses the frame instead of the lens axis.
#[cfg(feature = "debug")]
const RCS_EYE: Meters3 = Meters3::new(25.0, 140.0, 85.0);

/// The mouse motion fed every frame the push is held, in the pixels a real
/// mouse would report. Sideways, and large enough to saturate `on_rcs_aim`'s
/// unit clamp: the RCS gain is 0.03 per pixel at its lowest setting
/// (`MousePath::Rcs`), so 40 px is full deflection on any install. A partial
/// push decays to a trickle between frames and runs the drain past the
/// 600-frame loop cap.
#[cfg(feature = "debug")]
const RCS_PUSH_PIXELS: Vec2 = Vec2::new(40.0, 0.0);

/// Frames of the first push, half of [`RCS_FLIP_FRAMES`], so the hull swings
/// about where it started instead of creeping one way.
#[cfg(feature = "debug")]
const RCS_LEAD_FRAMES: u32 = 15;

/// Frames between push flips. At full deflection this swings the hull about
/// +-16 m/s and a few meters, well inside the hollow.
#[cfg(feature = "debug")]
const RCS_FLIP_FRAMES: u32 = 30;

/// How many pips the magazine chip carries
/// (`nova_hud::flight_status::RCS_PIPS`).
#[cfg(feature = "debug")]
const MAGAZINE_PIPS: usize = 10;

/// Slack against float drift when comparing `RcsBudget::spent` to the
/// magazine's cap or to zero.
#[cfg(feature = "debug")]
const MAGAZINE_EPSILON: f32 = 1e-3;

/// Present while the magazine beat is holding the mouse over, counting the
/// frames it has pushed so [`push_rcs`] knows which way to push.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct RcsPush {
    frames: u32,
}

/// Frames on which the player hull touched another solid body, counted from
/// the start of the drain.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct HullContacts(u32);

/// Which leg of the magazine walk is current, so [`trace_pips`] can label the
/// pip counts it samples.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MagazinePhaseType {
    Drain,
    Hold,
    Refill,
}

/// The pip count `trace_pips` has sampled, one entry per frame it ran, with
/// the phase current on that frame.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct PipTrace {
    phase: MagazinePhaseType,
    samples: Vec<(MagazinePhaseType, usize)>,
}

#[cfg(feature = "debug")]
impl PipTrace {
    fn new(phase: MagazinePhaseType) -> Self {
        Self {
            phase,
            samples: Vec::new(),
        }
    }
}

/// The ship's speed at the start of the Hold beat, for `assert_hold`.
#[cfg(feature = "debug")]
#[derive(Resource, Clone, Copy)]
struct HoldStartSpeed(MetersPerSecond);

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(rcs_script());
        app.add_plugins(lesson_chase_plugin);
        app.add_systems(Update, push_rcs.run_if(resource_exists::<RcsPush>));
        app.add_systems(Update, trace_pips.run_if(resource_exists::<PipTrace>));
        app.add_systems(
            Update,
            trace_hull_contacts.run_if(resource_exists::<HullContacts>),
        );
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    commands.trigger(LoadScenario(hollow::flight_hollow(&game_assets, &ships)));
}

/// Report the mouse as still moving, one frame at a time - `on_rcs_aim` is
/// delta-driven and the intent decays the moment the mouse stops, so a held
/// push has to arrive every frame rather than as one message. The direction
/// flips every [`RCS_FLIP_FRAMES`] after a [`RCS_LEAD_FRAMES`] lead.
#[cfg(feature = "debug")]
fn push_rcs(mut push: ResMut<RcsPush>, mut motion: MessageWriter<bevy::input::mouse::MouseMotion>) {
    let flips = (push.frames + RCS_FLIP_FRAMES - RCS_LEAD_FRAMES) / RCS_FLIP_FRAMES;
    let sign = if flips.is_multiple_of(2) { 1.0 } else { -1.0 };
    push.frames += 1;
    motion.write(bevy::input::mouse::MouseMotion {
        delta: RCS_PUSH_PIXELS * sign,
    });
}

/// Count a frame on which the player hull is in solid contact with anything.
/// `Collisions::iter` also yields touching sensors, which carry no contact
/// points, so only pairs with a manifold point count.
#[cfg(feature = "debug")]
fn trace_hull_contacts(
    mut contacts: ResMut<HullContacts>,
    collisions: avian3d::prelude::Collisions,
    ship: Single<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let ship = *ship;
    let touching = collisions.iter().any(|pair| {
        (pair.body1 == Some(ship) || pair.body2 == Some(ship))
            && pair
                .manifolds
                .iter()
                .any(|manifold| !manifold.points.is_empty())
    });
    if touching {
        contacts.0 += 1;
    }
}

/// Count the lit pips on the magazine chip this frame, and append them to the
/// trace under the walk's current phase. Silent while the chip is unanchored:
/// the HUD stops repainting hidden pips, so their colors would be stale.
/// `flight_hollow` spawns one ship, so every pip is the player's.
#[cfg(feature = "debug")]
fn trace_pips(
    mut trace: ResMut<PipTrace>,
    chips: Query<&ScreenIndicatorAnchor, With<RcsBudgetChipUIMarker>>,
    pips: Query<&BackgroundColor, With<RcsBudgetPipUIMarker>>,
) {
    if !chips.iter().any(|anchor| anchor.0.is_some()) {
        return;
    }
    let lit = pips.iter().filter(|color| color.0.alpha() > 0.5).count();
    let phase = trace.phase;
    trace.samples.push((phase, lit));
}

/// The player ship's live `RcsBudget` and the magazine's cap, or `None` while
/// either the player or `FlightSettings` is not yet in the world.
#[cfg(feature = "debug")]
fn player_rcs_budget(world: &World) -> Option<(RcsBudget, f32)> {
    let cap = world.get_resource::<FlightSettings>()?.rcs_budget;
    let mut ships = world
        .try_query_filtered::<&RcsBudget, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>(
        )?;
    ships.iter(world).next().map(|budget| (*budget, cap))
}

/// The player ship's current speed, zero while it is not yet in the world.
#[cfg(feature = "debug")]
fn player_speed(world: &mut World) -> MetersPerSecond {
    let mut ships = world.query_filtered::<&avian3d::prelude::LinearVelocity, (
        With<SpaceshipRootMarker>,
        With<PlayerSpaceshipMarker>,
    )>();
    let engine = ships
        .iter(world)
        .next()
        .map_or(0.0, |velocity| velocity.length());
    MetersPerSecond::from_engine(engine)
}

/// Advance once the player's magazine is spent down to its cap - empty.
#[cfg(feature = "debug")]
fn magazine_empty() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        player_rcs_budget(world).is_some_and(|(budget, cap)| budget.spent >= cap - MAGAZINE_EPSILON)
    })
}

/// Advance once the player's magazine has recovered all its spend - full.
#[cfg(feature = "debug")]
fn magazine_full() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        player_rcs_budget(world).is_some_and(|(budget, _)| budget.spent <= MAGAZINE_EPSILON)
    })
}

/// A command held on an empty magazine must deliver nothing: the hull's speed
/// must not move, and the magazine must still read empty.
#[cfg(feature = "debug")]
fn assert_hold(world: &mut World) {
    let start = world.resource::<HoldStartSpeed>().0;
    let end = player_speed(world);
    let delta = (end - start).abs().get();
    assert!(
        delta < 0.5,
        "rcs magazine: hold speed drifted {delta:.2} m/s (start {:.2}, end {:.2}) while the \
         magazine was supposed to be empty and pushing nothing",
        start.get(),
        end.get()
    );
    let (budget, cap) = player_rcs_budget(world)
        .expect("rcs magazine: no player RcsBudget to check at the end of Hold");
    assert!(
        budget.spent >= cap - MAGAZINE_EPSILON,
        "rcs magazine: spent {} came off the cap {cap} during Hold - the empty magazine refilled \
         while the push was still held",
        budget.spent
    );
}

/// The hull must not have touched anything since the drain began: a contact
/// would mean a wall, not the empty magazine, held the speed through Hold.
#[cfg(feature = "debug")]
fn assert_no_contact(world: &World) {
    let frames = world.resource::<HullContacts>().0;
    assert_eq!(
        frames, 0,
        "rcs magazine: the hull touched another body on {frames} frame(s) of the walk"
    );
}

/// During Drain the lit count must never rise, and it must reach zero by the
/// end of Hold.
#[cfg(feature = "debug")]
fn assert_drain(world: &World) {
    let trace = world.resource::<PipTrace>();
    let drain: Vec<usize> = trace
        .samples
        .iter()
        .filter(|(phase, _)| *phase == MagazinePhaseType::Drain)
        .map(|(_, lit)| *lit)
        .collect();
    assert_eq!(
        drain.first().copied(),
        Some(MAGAZINE_PIPS),
        "rcs magazine: the chip did not show a full magazine when the push began: {drain:?}"
    );
    assert!(
        drain.windows(2).all(|pair| pair[1] <= pair[0]),
        "rcs magazine: a Drain pip count rose mid-push: {drain:?}"
    );
    let last_hold = trace
        .samples
        .iter()
        .filter(|(phase, _)| *phase == MagazinePhaseType::Hold)
        .map(|(_, lit)| *lit)
        .next_back();
    assert_eq!(
        last_hold,
        Some(0),
        "rcs magazine: the chip was not down to zero pips by the end of Hold (last Hold sample \
         {last_hold:?}, drain trace {drain:?})"
    );
}

/// During Refill the chip must start empty, never lose a pip, and show all ten
/// before it hides on a full, idle magazine. Recovery adds about 1% of the
/// magazine per frame, so the last shown frame always rounds up to ten.
#[cfg(feature = "debug")]
fn assert_refill(world: &World) {
    let trace = world.resource::<PipTrace>();
    let refill: Vec<usize> = trace
        .samples
        .iter()
        .filter(|(phase, _)| *phase == MagazinePhaseType::Refill)
        .map(|(_, lit)| *lit)
        .collect();
    assert_eq!(
        refill.first().copied(),
        Some(0),
        "rcs magazine: the chip was not empty when the push let go: {refill:?}"
    );
    assert!(
        refill.windows(2).all(|pair| pair[1] >= pair[0]),
        "rcs magazine: a Refill pip count fell mid-recharge: {refill:?}"
    );
    assert_eq!(
        refill.last().copied(),
        Some(MAGAZINE_PIPS),
        "rcs magazine: the chip hid before it showed a full magazine: {refill:?}"
    );
    assert!(
        player_rcs_budget(world).is_some_and(|(budget, _)| budget.spent <= MAGAZINE_EPSILON),
        "rcs magazine: the magazine's spend was not back at zero after the refill"
    );
}

/// Fly one magazine walk: drain it under a held push, hold it empty, let go
/// and watch it refill.
#[cfg(feature = "debug")]
fn rcs_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the hollow")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("raise the instruments and take up the chase")
        .on_enter(|world: &mut World| {
            hollow::hud_instrument(world);
            world.insert_resource(LessonChase::new(RCS_EYE));
        })
        .until(elapsed(1.0))
        .add()
        .step("open the magazine loop")
        .on_enter(|world: &mut World| loop_start(world, LOOP_NAME))
        .add()
        .step("hold the full magazine")
        .until(elapsed(0.8))
        .add()
        .step("drain the magazine")
        .on_enter(|world: &mut World| {
            press_action("rcs_modifier")(world);
            world.insert_resource(RcsPush::default());
            world.insert_resource(HullContacts::default());
            world.insert_resource(PipTrace::new(MagazinePhaseType::Drain));
        })
        .until(magazine_empty())
        .deadline(30.0)
        .add()
        .step("hold the empty magazine")
        .on_enter(|world: &mut World| {
            world.resource_mut::<PipTrace>().phase = MagazinePhaseType::Hold;
            let speed = player_speed(world);
            world.insert_resource(HoldStartSpeed(speed));
        })
        .until(elapsed(1.0))
        .add()
        .step("let the modifier up and refill")
        .on_enter(|world: &mut World| {
            assert_hold(world);
            world.resource_mut::<PipTrace>().phase = MagazinePhaseType::Refill;
            release_action("rcs_modifier")(world);
            world.remove_resource::<RcsPush>();
        })
        .until(magazine_full())
        .deadline(30.0)
        .add()
        .step("let the magazine settle")
        .until(elapsed(0.5))
        .add()
        .step("close the magazine loop")
        .on_enter(|world: &mut World| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(60.0)
        .add()
        .step("check the pip trace")
        .on_enter(|world: &mut World| {
            assert_no_contact(world);
            assert_drain(world);
            assert_refill(world);
        })
        .add()
}
