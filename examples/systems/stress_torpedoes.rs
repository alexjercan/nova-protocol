//! stress_torpedoes: a thousand torpedoes under guidance at once, then none.
//!
//! The ORDNANCE lifecycle at a scale no fight will ever reach. Every torpedo in
//! the sky is running the full chain - proportional-navigation guidance toward
//! its committed point, the terminal weave, arming, and a proximity fuze - and
//! the range holds [`TORPEDOES_IN_FLIGHT`] of them at once. A turret round is a
//! ballistic body with a lifetime (`stress_bullets`); this is the guided,
//! per-frame-steered kind.
//!
//! What it claims, beside the frame time:
//!
//! - the rack is WHOLE - exactly the bays it was built with, so a run that
//!   quietly lost tubes cannot report an easy number;
//! - the sky actually FILLED - the peak live-torpedo count clears
//!   [`TORPEDOES_IN_FLIGHT`], so a dud trigger fails loudly instead of measuring
//!   an empty scene;
//! - the ordnance CLEARS - once the tubes close, every torpedo left flies its
//!   lane out and fuzes, and the count is exactly zero before anything is torn
//!   down;
//! - the teardown leaves NOTHING - not a torpedo, not a section, not a root.
//!
//! Every torpedo is committed to a POSITION down its own bay's lane. A torpedo
//! whose target is decided by nobody has no guidance and no thrust, so linear
//! damping stalls it a few units off the tube; the lanes fan out from the wall
//! of tubes and are all the SAME length, so the sky fills evenly and the drain
//! after the tubes close is bounded by one flight time rather than by the
//! longest diagonal.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example stress_torpedoes --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `stress_torpedoes: N bays up`,
//! #           `stress_torpedoes: peak N torpedoes in the sky`,
//! #           `stress_torpedoes: teardown returned to baseline`,
//! #           `autopilot: cycle complete, no panic`
//!
//! # with the frame-time capture armed:
//! cargo run --features debug probe run stress_torpedoes
//! ```

use bevy::{platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "stress_torpedoes")]
#[command(version = "1.0.0")]
#[command(about = "Stress range: a thousand guided torpedoes in flight at once. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// ABSURD SCALE, and deliberately so: this number must never reflect real
/// content. A salvo that decides a fight is six torpedoes; a thousand under
/// guidance at once is a load the code has to survive, not an engagement anyone
/// will fly. Read it as a ceiling the ordnance lifecycle is proven under, never
/// as a balance figure.
#[cfg(feature = "debug")]
const TORPEDOES_IN_FLIGHT: usize = 1000;

/// ABSURD SCALE (see [`TORPEDOES_IN_FLIGHT`]): two hundred tubes on one hull.
/// No ship in the game carries more than a few.
///
/// A bay launches once a second and a lane takes ~5 s to fly, so the sky
/// settles around 1300 torpedoes and reaches the range's scale five seconds
/// after the trigger goes down. The tube count is what buys that: population is
/// launch RATE times flight time, and both the fill and the drain are one
/// flight time long, so a slower rack would spend the whole completion budget
/// climbing to its own scale. Measured under lavapipe: 200 tubes on 150 u lanes
/// walk the whole script in 48 s of wall clock, against 117 s for 100 tubes on
/// 400 u ones at the same population.
const TORPEDO_BAYS: usize = 200;

/// The sections the hull carries: the flight computer, plus the rack. The
/// exact-count invariant reads this.
#[cfg(feature = "debug")]
const SECTIONS_ON_THE_HULL: usize = 1 + TORPEDO_BAYS;

/// The bay content id every tube is built from: the shipped Serpent bay, so the
/// weave and the fuze under load are the shipped ones.
const BAY_SECTION: &str = "torpedo_section";

/// How long every lane is: the distance from the tube to the aim point the
/// torpedo fuzes on. The SAME for every bay, so one flight time bounds both the
/// fill and the drain; the alternative (aim points on a flat plane ahead of the
/// wall) makes the outer lanes twice as long as the inner ones and the drain
/// waits for the worst of them. Short enough that the drain is seconds rather
/// than the torpedo's 100 s lifetime.
const LANE_LENGTH: f32 = 150.0;

/// How wide the lanes fan: the forward component of each lane's direction, in
/// the same units as the tube's own offset from the hull axis. Smaller fans
/// wider (a corner tube sits ~10 u off the axis, so 6.0 puts its lane ~58 deg
/// off the nose).
///
/// A wide fan is not decoration, it is the cheap arrangement: a thousand
/// torpedoes packed into a narrow column were measured SLOWER in wall clock
/// than the same population spread over a cone, despite a third less sim time,
/// because a dense column gives the broad phase far more pairs to sort. Spread
/// the streams out.
const LANE_FAN: f32 = 6.0;

/// The scenario id the range loads under.
const SCENARIO_ID: &str = "stress_torpedoes";

/// The first step's name, so `loop_from` restarts the cycle at the spawn
/// without repeating the string.
#[cfg(feature = "debug")]
const LOAD_STEP: &str = "spawn the rack";

/// How long the sky is held full once the scale is reached: the steady-state
/// stretch a frame capture is actually interested in.
#[cfg(feature = "debug")]
const HOLD_SECS: f32 = 2.0;

/// Settle window between the unload and the baseline assertion.
#[cfg(feature = "debug")]
const TEARDOWN_SETTLE_SECS: f32 = 0.5;

/// Deadlines, in the clamped sim seconds every gate here counts in. Generous
/// for llvmpipe, where a frame of this scene is hundreds of milliseconds, and
/// still summing well under the 120 s harness completion backstop.
#[cfg(feature = "debug")]
const SPAWN_DEADLINE_SECS: f32 = 45.0;
#[cfg(feature = "debug")]
const HOT_DEADLINE_SECS: f32 = 15.0;
#[cfg(feature = "debug")]
const FILL_DEADLINE_SECS: f32 = 40.0;
/// Above the ~5 s a lane takes at the Serpent's cruise, which is the true bound
/// once the tubes close: nothing new launches, so every torpedo left flies its
/// lane out and fuzes. Generous by a factor the weave and the fan can spend.
#[cfg(feature = "debug")]
const DRAIN_DEADLINE_SECS: f32 = 30.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new()
        .with_game_plugins(|app: &mut App| {
            app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
            app.add_systems(Update, commit_fresh_torpedoes);
        })
        .build();

    #[cfg(feature = "debug")]
    {
        app.init_resource::<HeldInput>();
        app.init_resource::<PeakTorpedoes>();
        app.add_systems(Update, track_peak_torpedoes);
        // The picture is taken at the range's PEAK. `nova_screenshot`
        // appends its beat to whatever it is handed, and the beats after
        // this call drain and tear the range down - a shot behind them
        // photographs an empty world.
        app.add_plugins(
            nova_screenshot(
                nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                    .input(hold_inputs)
                    .step(LOAD_STEP)
                    .enter(GameStates::Loading)
                    .until(the_rack_is_up())
                    .deadline(SPAWN_DEADLINE_SECS)
                    .add()
                    // The scene is live again: close the reload interval so a frame
                    // capture excludes it. A no-op on the first cycle.
                    .step("close the reload interval")
                    .on_enter(nova_probe::capture_reload_end)
                    .on_enter(assert_the_rack_is_whole)
                    .add()
                    // Raise the stance and WAIT for the safety to go hot: a press
                    // that arrives cold latches nothing, forever.
                    .step("raise the weapons")
                    .on_enter(|world: &mut World| world.resource_mut::<HeldInput>().combat = true)
                    .until(weapons_are_hot())
                    .deadline(HOT_DEADLINE_SECS)
                    .add()
                    .step("fill the sky")
                    .on_enter(|world: &mut World| world.resource_mut::<HeldInput>().fire = true)
                    .until(the_sky_is_full())
                    .deadline(FILL_DEADLINE_SECS)
                    .add()
                    .step("hold the saturation")
                    .until(elapsed(HOLD_SECS))
                    .add()
                    .step("assert the sky filled")
                    .on_enter(assert_the_sky_filled)
                    .add(),
            )
            .step("let the ordnance drain")
            .on_enter(close_the_tubes)
            .until(no_torpedoes_in_flight())
            .deadline(DRAIN_DEADLINE_SECS)
            .add()
            .step("assert the ordnance drained")
            .on_enter(assert_the_ordnance_drained)
            .add()
            .step("tear the range down")
            .on_enter(tear_the_range_down)
            .until(elapsed(TEARDOWN_SETTLE_SECS))
            .add()
            .step("check the world came back to baseline")
            .on_enter(assert_back_to_baseline)
            .add()
            .loop_from(LOAD_STEP)
            .on_loop(respawn_the_range),
        );
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        app.add_plugins(nova_probe::NovaProbePlugin::default());
    }

    app.run()
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(range_scenario(&game_assets, &sections)));
}

/// Where bay `i` sits: a flat 10-wide wall of tubes, centered on the hull's
/// axis and facing -Z, so every muzzle opens into empty space instead of into
/// the section in front of it.
fn bay_position(i: usize) -> Vec3 {
    let side = (TORPEDO_BAYS as f32).sqrt().ceil();
    let half = (side - 1.0) / 2.0;
    Vec3::new(
        (i % side as usize) as f32 - half,
        (i / side as usize) as f32 - half,
        0.0,
    )
}

/// The rack: a flight computer bolted to the edge of a wall of tubes, every one
/// of them bound to the same trigger.
///
/// The wall shares faces cell to cell and the computer shares one with it, so
/// the integrity graph gets one connected structure - a cloud of islands would
/// spend the run coming apart instead of launching.
fn rack(sections: &GameSections) -> SpaceshipConfig {
    let side = (TORPEDO_BAYS as f32).sqrt().ceil();
    let half = (side - 1.0) / 2.0;
    let mut specs = vec![SectionSpec::new(
        "controller",
        "basic_controller_section",
        Vec3::new(-half - 1.0, -half, 0.0),
    )];
    specs.extend(
        (0..TORPEDO_BAYS).map(|i| SectionSpec::new(bay_slot(i), BAY_SECTION, bay_position(i))),
    );

    fixtures::ship(
        sections,
        SpaceshipController::Player(PlayerControllerConfig {
            // Every tube on one trigger: the rack is a load, not a loadout.
            input_mapping: (0..TORPEDO_BAYS)
                .map(|i| {
                    (
                        bay_slot(i),
                        vec![
                            MouseButton::Left.into(),
                            GamepadButton::RightTrigger2.into(),
                        ],
                    )
                })
                .collect::<HashMap<_, _>>(),
            speed_cap: None,
            // Saturation harness: the six-round rack and its regen are not the
            // subject, the thousand torpedoes are.
            infinite_ammo: true,
        }),
        &specs,
    )
}

/// The per-ship slot id of bay `i`, shared by the layout and the bindings.
fn bay_slot(i: usize) -> String {
    format!("bay_{i}")
}

/// The range: the rack alone, firing into empty space.
fn range_scenario(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let objects = vec![ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "rack".to_string(),
            name: "Rack".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(rack(sections)),
    }];

    ScenarioConfig {
        description: "A rack holding a thousand torpedoes under guidance.".to_string(),
        hidden: true,
        // The scene lights itself: the engine spawns no light, so a scenario
        // that authors none renders black.
        events: fixtures::spawn_on_start(
            [
                objects,
                ThreePointRig::around("lights", Vec3::ZERO, 20.0).objects(),
            ]
            .concat(),
        ),
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Stress: Torpedoes".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Commit every fresh torpedo to a point down its own lane.
///
/// A torpedo's target is decided exactly once, right after launch: the player
/// commits from the crosshair lock and the AI from its own `AITarget`. Neither
/// runs here (nobody is aiming a thousand torpedoes), so this does that one
/// write, and the guidance, the weave, the arming and the fuze run themselves.
///
/// The lane is derived from the torpedo's own launch position, so it is a pure
/// function of where it came from: the same range replays identically, and the
/// streams fan apart ([`LANE_FAN`]) instead of converging into one column.
fn commit_fresh_torpedoes(
    mut commands: Commands,
    q_fresh: Query<
        (Entity, &Transform),
        (
            With<TorpedoProjectileMarker>,
            Without<TorpedoTargetChosen>,
            Without<TorpedoTargetEntity>,
        ),
    >,
) {
    for (torpedo, transform) in &q_fresh {
        let from = transform.translation;
        let fan = Vec3::new(from.x, from.y, -LANE_FAN).normalize_or(Vec3::NEG_Z);
        let aim = from + fan * LANE_LENGTH;
        commands
            .entity(torpedo)
            .insert((TorpedoTargetChosen, TorpedoTargetPosition(aim)));
    }
}

/// The inputs the script holds down, re-applied every frame: the safety derives
/// `WeaponsHot` from the HELD combat input, so a press applied once on a step's
/// entry would be a press, not a hold.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct HeldInput {
    /// RMB - the combat stance the safety gates firing on.
    combat: bool,
    /// LMB - the trigger.
    fire: bool,
}

/// Re-press the held inputs for the frame. Wired as the script's `input` hook,
/// which runs in `PreUpdate` after `InputSystems` - the only place a
/// synthesized press is still `just_pressed` when the game's `Update` input
/// systems read it.
#[cfg(feature = "debug")]
fn hold_inputs(world: &mut World, _elapsed: f32) {
    let held = world.resource::<HeldInput>();
    let (combat, fire) = (held.combat, held.fire);
    if combat {
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
    }
    if fire {
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
    }
}

/// The high-water mark of torpedoes in flight this cycle.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct PeakTorpedoes(usize);

#[cfg(feature = "debug")]
fn track_peak_torpedoes(
    q_torpedoes: Query<(), With<TorpedoProjectileMarker>>,
    mut peak: ResMut<PeakTorpedoes>,
) {
    peak.0 = peak.0.max(q_torpedoes.iter().count());
}

/// How many torpedoes are alive right now.
#[cfg(feature = "debug")]
fn live_torpedoes(world: &World) -> usize {
    world
        .iter_entities()
        .filter(|entity| entity.contains::<TorpedoProjectileMarker>())
        .count()
}

/// Live sections and ship roots - what the teardown claim covers beside the
/// ordnance.
#[cfg(feature = "debug")]
fn live_structure(world: &World) -> (usize, usize) {
    let sections = world
        .iter_entities()
        .filter(|entity| entity.contains::<SectionMarker>())
        .count();
    let roots = world
        .iter_entities()
        .filter(|entity| entity.contains::<SpaceshipRootMarker>())
        .count();
    (sections, roots)
}

/// Advance once the whole rack is in the world.
#[cfg(feature = "debug")]
fn the_rack_is_up() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| live_structure(world).0 >= SECTIONS_ON_THE_HULL)
}

/// The player ship's weapons safety has gone hot.
#[cfg(feature = "debug")]
fn weapons_are_hot() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&WeaponsHot, With<PlayerSpaceshipMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|hot| hot.0))
    })
}

/// The sky holds the range's whole scale at once.
#[cfg(feature = "debug")]
fn the_sky_is_full() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| live_torpedoes(world) >= TORPEDOES_IN_FLIGHT)
}

/// Every torpedo has fuzed or expired.
#[cfg(feature = "debug")]
fn no_torpedoes_in_flight() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| live_torpedoes(world) == 0)
}

/// The rack is EXACTLY what it was built with.
#[cfg(feature = "debug")]
fn assert_the_rack_is_whole(world: &mut World) {
    let (sections, roots) = live_structure(world);
    let bays = world
        .iter_entities()
        .filter(|entity| entity.contains::<TorpedoSectionMarker>())
        .count();
    assert_eq!(
        (sections, roots, bays),
        (SECTIONS_ON_THE_HULL, 1, TORPEDO_BAYS),
        "stress_torpedoes: the range must stand up as exactly one hull of \
         {SECTIONS_ON_THE_HULL} sections carrying {TORPEDO_BAYS} bays"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the rack assembled every bay",
        serde_json::json!({ "sections": sections, "bays": bays }),
    );
    info!("stress_torpedoes: {bays} bays up on {sections} sections");
}

/// The scale claim: the sky held a thousand guided torpedoes at once.
#[cfg(feature = "debug")]
fn assert_the_sky_filled(world: &mut World) {
    let peak = world.resource::<PeakTorpedoes>().0;
    let live = live_torpedoes(world);
    assert!(
        peak >= TORPEDOES_IN_FLIGHT,
        "stress_torpedoes: peaked at only {peak} torpedoes in flight (want >= \
         {TORPEDOES_IN_FLIGHT}) - the rack never reached the range's scale"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a thousand torpedoes under guidance at once",
        serde_json::json!({ "peak": peak, "live": live }),
    );
    info!("stress_torpedoes: peak {peak} torpedoes in the sky, {live} live now");
}

/// RELEASE the trigger, do not merely stop re-pressing it: nothing else
/// releases a synthesized button, and a latched one makes the next cycle's
/// press a no-op.
#[cfg(feature = "debug")]
fn close_the_tubes(world: &mut World) {
    world.resource_mut::<HeldInput>().fire = false;
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
}

/// The lifecycle claim: the sky came back to EXACTLY zero, on the ordnance's
/// own guidance and fuzes, before anything was torn down.
#[cfg(feature = "debug")]
fn assert_the_ordnance_drained(world: &mut World) {
    let live = live_torpedoes(world);
    assert_eq!(
        live, 0,
        "stress_torpedoes: {live} torpedoes still in flight after the drain \
         gate opened - ordnance that neither fuzed nor expired"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the ordnance drained to nothing",
        serde_json::json!({ "live": live }),
    );
    info!("stress_torpedoes: the ordnance drained to zero");
}

/// Drop the whole scene.
#[cfg(feature = "debug")]
fn tear_the_range_down(world: &mut World) {
    nova_probe::probe_marker(world, "stress: teardown", serde_json::json!({}));
    world.trigger(UnloadScenario);
}

/// The teardown claim: nothing survived.
#[cfg(feature = "debug")]
fn assert_back_to_baseline(world: &mut World) {
    let torpedoes = live_torpedoes(world);
    let (sections, roots) = live_structure(world);
    assert_eq!(
        (torpedoes, sections, roots),
        (0, 0, 0),
        "stress_torpedoes: {torpedoes} torpedoes, {sections} sections and \
         {roots} ship roots survived UnloadScenario - the range did not return \
         to baseline"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the teardown left nothing behind",
        serde_json::json!({ "torpedoes": torpedoes, "sections": sections, "roots": roots }),
    );
    info!("stress_torpedoes: teardown returned to baseline");
}

/// Rebuild the range for the next capture cycle. The peak resets with it: the
/// scale claim is per-cycle.
#[cfg(feature = "debug")]
fn respawn_the_range(world: &mut World) {
    let (Some(game_assets), Some(sections)) = (
        world.get_resource::<GameAssets>().cloned(),
        world.get_resource::<GameSections>().cloned(),
    ) else {
        return;
    };
    world.resource_mut::<PeakTorpedoes>().0 = 0;
    nova_probe::capture_reload_begin(world);
    world.trigger(LoadScenario(range_scenario(&game_assets, &sections)));
}
