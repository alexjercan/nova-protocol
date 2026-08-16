//! torpedo_section: a focused test range for the torpedo bay section.
//!
//! One player ship carrying a single torpedo bay fires across two scenes. In the
//! GATE scene it sits at the origin facing a spread of asteroid "gates" - near,
//! mid and far straight ahead, one off to the side, and one that drifts slowly
//! across the range - and you watch torpedoes arm, home and detonate. In the
//! CROSSING scene a single target crosses laterally and fast, which is the case
//! proportional navigation has to solve by LEADING it; the range measures how
//! far ahead of the line of sight the torpedo steers, and the script asserts it.
//! This is the interactive harness for the torpedo work (arming
//! `20260707-100003`, target loss `20260707-100004`, PN guidance
//! `20260525-133021`, blast tuning `20260706-162913`).
//!
//! What it shows:
//! - Guidance gizmos: each torpedo draws a red line to the point it is steering
//!   toward, a red sphere at that target point, a sphere on the torpedo that is
//!   yellow while un-armed and green once armed (so you can see the arming
//!   delay from `20260707-100003` directly), and a blue TRAIL of the path it
//!   has flown - the only way to see the terminal weave, a corkscrew being
//!   invisible in any single frame.
//! - Lifecycle logging: `range: torpedo fired`, `range: torpedo ... armed`, and
//!   `range: torpedo detonated` trace one shot from launch to blast.
//! - `guidance: lead angle N.N deg` - how far ahead of the line of sight the
//!   torpedo is steering, which is what separates a lead solution from a stern
//!   chase - plus `guidance: closest approach N.N` and `guidance: torpedo speed
//!   N.N` (throttled) as the readable quality readouts alongside it.
//!
//! Targeting: for the range each fresh torpedo is auto-assigned the nearest gate
//! (so homing is always exercised, even hands-off); in the full game you aim with
//! the mouse and the player targeting picks the target instead.
//!
//! Controls:
//! - Right mouse (held): raise weapons - the safety keeps a lowered ship cold.
//! - Space (or right trigger): fire a torpedo. Held, it fires at the bay's rate.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example torpedo_section --features debug
//! # look for: `nova harness: reached Playing`, `range: torpedo fired`,
//! #           `range: torpedo ... armed`, `guidance: lead angle ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use std::sync::Arc;

use avian3d::prelude::*;
use bevy::{color::palettes::tailwind, platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "torpedo_section")]
#[command(version = "1.0.0")]
#[command(about = "A test range for the torpedo bay section in nova_protocol", long_about = None)]
struct Cli;

/// Id of the one gate that drifts, so the range can single it out and drive it.
const MOVING_GATE_ID: &str = "gate_moving";

/// Id of the crossing target in the second scene.
const CROSSER_ID: &str = "crosser";

/// The scenario id the gate scene loads under.
const RANGE_ID: &str = "torpedo_range";

#[cfg(feature = "debug")]
/// The scenario id the crossing scene loads under. DISTINCT from [`RANGE_ID`]
/// so the script's scene-change gate can observe WHICH scene is live rather
/// than guessing from world contents.
const CROSSING_ID: &str = "torpedo_crossing";

/// Lateral speed of the crossing target (units/s). Fast enough that pointing
/// straight at it misses: a torpedo capping at 35 u/s covering ~90 u takes long
/// enough for a 15 u/s crosser to move most of a target width off the line of
/// sight, so only a lead solution intercepts.
const TARGET_CROSS_SPEED: f32 = 15.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // Headless smoke-test harness: inert in a normal run. Under NOVA_AUTOPILOT
    // it walks the two scenes and asserts the range's PURPOSE rather than its
    // survival (task 20260712-211352 - reach-Playing alone let a silently dud
    // torpedo pass): a torpedo fired, armed, detonated and damaged a gate, the
    // guidance LED a fast crosser, and the whole launch chain repeated in the
    // second scene. Under NOVA_SHOT it captures a PNG. The scene is built on
    // `GameAssetsStates::Loaded` (below) so the screenshot's forced Playing
    // does not re-run setup.
    #[cfg(feature = "debug")]
    {
        app.init_resource::<RangeOutcome>();
        app.init_resource::<HeldInput>();
        app.add_observer(
            |_: On<Add, TorpedoProjectileMarker>, mut outcome: ResMut<RangeOutcome>| {
                outcome.fired = true;
            },
        );
        app.add_observer(|_: On<Add, NovaBlast>, mut outcome: ResMut<RangeOutcome>| {
            outcome.detonated = true;
        });
        app.add_observer(
            |damage: On<HealthApplyDamage>,
             q_gate: Query<(), With<RangeGateMarker>>,
             mut outcome: ResMut<RangeOutcome>| {
                if q_gate.contains(damage.entity) {
                    outcome.gate_damaged = true;
                }
            },
        );
        app.add_systems(
            Update,
            |q_armed: Query<&TorpedoArming, With<TorpedoProjectileMarker>>,
             mut outcome: ResMut<RangeOutcome>| {
                outcome.armed |= q_armed.iter().any(|arming| arming.is_armed());
            },
        );
        // Probe wiring (task 20260719-210443; each plugin is inert without
        // its NOVA_PERF_* env): run timeline + engine-bound invariants +
        // frame-time capture, so `probe run` can measure this example.
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(torpedo_script());
        app.add_plugins(nova_screenshot());
    }

    app.run()
}

/// What the current round has observed so far. Reset when the script changes
/// scene, so the crossing round's assertions are claims about THAT scene rather
/// than leftovers from the gate round.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct RangeOutcome {
    fired: bool,
    armed: bool,
    detonated: bool,
    gate_damaged: bool,
}

fn custom_plugin(app: &mut App) {
    app.insert_resource(BestApproach(f32::INFINITY));
    app.init_resource::<BestLeadDeg>();
    app.insert_resource(SpeedReportTimer(Timer::from_seconds(
        0.5,
        TimerMode::Repeating,
    )));
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
    app.add_systems(
        Update,
        (
            range_autotarget,
            drive_moving_gate,
            drive_crosser,
            track_closest_approach,
            track_lead_angle,
            report_torpedo_speed,
            log_torpedo_armed,
            // Chained: the trail must be sampled before it is drawn, or the
            // gizmo lags a frame behind the torpedo it belongs to.
            record_flight_paths,
            draw_guidance_gizmos,
        )
            .chain(),
    );
    app.add_observer(tag_gate);
    app.add_observer(log_torpedo_fired);
    app.add_observer(log_torpedo_detonated);
    app.add_observer(assert_no_owner_pair_damage);
}

/// Marks an asteroid the range treats as a target gate.
#[derive(Component)]
struct RangeGateMarker;

/// Marks the single drifting gate in the gate scene.
#[derive(Component)]
struct RangeMovingTarget;

/// Marks the laterally crossing target in the crossing scene.
#[derive(Component)]
struct RangeCrosser;

/// Marks a torpedo whose "armed" transition has already been logged.
#[derive(Component)]
struct RangeArmLogged;

/// Smallest torpedo-to-target distance seen so far, the readable PN readout.
#[derive(Resource)]
struct BestApproach(f32);

/// The midcourse lead readout. See [`track_lead_angle`].
#[derive(Resource, Default)]
struct BestLeadDeg {
    /// Largest LEAD angle a torpedo has held in late midcourse, in degrees -
    /// how far ahead of the line of sight it is steering, measured along the
    /// direction the target is drifting across that line.
    deg: f32,
    /// Run time of the FIRST midcourse sample this round, or `None` while no
    /// torpedo has reached [`LEAD_SAMPLE_RANGE`] yet.
    ///
    /// Recorded whatever the angle was: it is what separates "the guidance is
    /// chasing" (samples taken, all near 0) from "no torpedo got close enough
    /// to sample", which [`deg`](Self::deg) alone cannot tell apart. The lead
    /// beat waits on this so the assertion decides the ANGLE.
    first_sample_at: Option<f32>,
}

/// The range window, in units, the lead angle is sampled over.
///
/// Both ends are load-bearing. The FAR end excludes the launch transient: a
/// torpedo leaves the bay pointing down the ship's nose, which for this scene's
/// geometry already sits ~24 deg off the line of sight and on the same side as
/// the lead, so a sample taken early cannot tell "has not turned yet" from
/// "is leading". The NEAR end sits just outside the proximity fuze (15 u), so
/// the sample is taken while the torpedo is still steering rather than in the
/// frame it detonates.
const LEAD_SAMPLE_RANGE: std::ops::RangeInclusive<f32> = 16.0..=30.0;

/// Throttle for the torpedo-speed readout.
#[derive(Resource)]
struct SpeedReportTimer(Timer);

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(torpedo_range(&game_assets, &sections)));
}

/// The player ship both scenes fly: a controller (so it holds attitude and is
/// aimable) and BOTH shipped torpedo bays, side by side on one trigger.
///
/// Two bays and not one, because what is worth looking at on this range is the
/// DIFFERENCE. A Serpent and a Lance launched at the same target in the same
/// frame draw their paths in the same picture, each in its type's own colour,
/// and a corkscrew is only legible next to the straight line it departs from.
/// Both bays fire on the same key: the range is a comparison, not a loadout.
fn torpedo_ship(sections: &GameSections) -> SpaceshipConfig {
    let trigger = || vec![KeyCode::Space.into(), GamepadButton::RightTrigger.into()];
    fixtures::ship(
        sections,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::from([
                ("torpedo".to_string(), trigger()),
                ("lance".to_string(), trigger()),
            ]),
            speed_cap: None,
            // Dev/tuning harness: fire freely.
            infinite_ammo: true,
        }),
        &[
            SectionSpec::new("controller", "basic_controller_section", Vec3::ZERO),
            SectionSpec::new("hull", "reinforced_hull_section", Vec3::new(0.0, 0.0, 1.0)),
            SectionSpec::new("torpedo", "torpedo_section", Vec3::new(0.0, 0.0, -1.0)),
            SectionSpec::new("lance", "lance_torpedo_section", Vec3::new(1.0, 0.0, -1.0)),
        ],
    )
}

/// An asteroid target. Health is the knob that decides whether it survives being
/// hit: the gate scene wants gates a blast can destroy (arm -> home -> hit
/// feedback), the crossing scene wants a target that outlives several torpedoes
/// so the approach metric keeps accumulating.
fn gate(
    game_assets: &GameAssets,
    id: &str,
    name: &str,
    pos: Vec3,
    radius: f32,
    health: f32,
) -> ScenarioObjectConfig {
    // Unsigned: the gates are shot at over open sights, never radar-locked.
    fixtures::asteroid(game_assets, id, name, pos, radius, health, None)
}

fn player_ship_object(sections: &GameSections) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "player_ship".to_string(),
            name: "Torpedo Ship".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(torpedo_ship(sections)),
    }
}

/// Scene 1: the gate range - one player torpedo ship plus the target gates.
fn torpedo_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    // Gates ahead of the ship (forward is -Z). Health is low enough that a
    // torpedo blast destroys one, so you get arm -> home -> hit feedback.
    let objects = vec![
        player_ship_object(sections),
        gate(
            game_assets,
            "gate_near",
            "Near Gate",
            Vec3::new(0.0, 0.0, -30.0),
            2.0,
            60.0,
        ),
        gate(
            game_assets,
            "gate_mid",
            "Mid Gate",
            Vec3::new(0.0, 0.0, -70.0),
            3.0,
            60.0,
        ),
        gate(
            game_assets,
            "gate_far",
            "Far Gate",
            Vec3::new(6.0, 0.0, -120.0),
            3.0,
            60.0,
        ),
        gate(
            game_assets,
            "gate_side",
            "Side Gate",
            Vec3::new(-25.0, 0.0, -60.0),
            2.0,
            60.0,
        ),
        gate(
            game_assets,
            MOVING_GATE_ID,
            "Moving Gate",
            Vec3::new(25.0, 0.0, -90.0),
            2.0,
            60.0,
        ),
    ];

    ScenarioConfig {
        description: "A test range for the torpedo bay section.".to_string(),
        // The scene lights itself: the engine spawns no light, so a scenario
        // that authors none renders black.
        events: fixtures::spawn_on_start(
            [
                objects,
                ThreePointRig::around("range", Vec3::ZERO, 5.0).objects(),
            ]
            .concat(),
        ),
        ..ScenarioConfig::new(
            RANGE_ID.to_string(),
            "Torpedo Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

#[cfg(feature = "debug")]
/// Scene 2: the crossing range - the same ship against ONE target crossing
/// laterally, fast, some way ahead. This is the scene built to answer one
/// question the gate scene cannot: does the torpedo LEAD a mover?
///
/// The crosser gets a lot of health so it survives being hit and the harness
/// can keep measuring approach across several torpedoes.
fn crossing_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let objects = vec![
        player_ship_object(sections),
        gate(
            game_assets,
            CROSSER_ID,
            "Crossing Target",
            Vec3::new(-40.0, 0.0, -90.0),
            3.0,
            100_000.0,
        ),
    ];

    ScenarioConfig {
        description: "A harness for the torpedo PN guidance.".to_string(),
        // The scene lights itself: the engine spawns no light, so a scenario
        // that authors none renders black.
        events: fixtures::spawn_on_start(
            [
                objects,
                ThreePointRig::around("crossing", Vec3::ZERO, 5.0).objects(),
            ]
            .concat(),
        ),
        ..ScenarioConfig::new(
            CROSSING_ID.to_string(),
            "Torpedo Crossing Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Tag each spawned asteroid as a gate, and single out the two driven ones.
fn tag_gate(add: On<Add, AsteroidMarker>, mut commands: Commands, q_id: Query<&EntityId>) {
    let entity = add.entity;
    commands.entity(entity).insert(RangeGateMarker);

    let Ok(id) = q_id.get(entity) else {
        return;
    };
    if id.0 == MOVING_GATE_ID {
        commands
            .entity(entity)
            .insert((RangeMovingTarget, LinearVelocity::default()));
    }
    if id.0 == CROSSER_ID {
        commands
            .entity(entity)
            .insert((RangeCrosser, LinearVelocity::default()));
    }
}

/// Drift the moving gate side to side so torpedoes have to lead it.
fn drive_moving_gate(
    time: Res<Time>,
    mut q_gate: Query<&mut LinearVelocity, With<RangeMovingTarget>>,
) {
    let speed = (time.elapsed_secs() * 0.5).sin() * 10.0;
    for mut velocity in &mut q_gate {
        velocity.0 = Vec3::new(speed, 0.0, 0.0);
    }
}

/// Drive the crossing target across at a CONSTANT lateral velocity - the case PN
/// should solve exactly by leading it, which is why this scene does not reuse
/// the gate scene's sinusoidal drift.
fn drive_crosser(mut q_target: Query<&mut LinearVelocity, With<RangeCrosser>>) {
    for mut velocity in &mut q_target {
        velocity.0 = Vec3::new(TARGET_CROSS_SPEED, 0.0, 0.0);
    }
}

/// Range convenience: assign each fresh torpedo the nearest gate so homing is
/// always exercised. In the full game the player's aim picks the target instead.
/// Mirrors the game's commit-at-launch contract: the assignment happens once
/// (`TorpedoTargetChosen`), and a torpedo whose gate dies keeps flying to the
/// frozen position instead of re-targeting another gate.
///
/// In the crossing scene the crosser is the only gate, so this is also that
/// scene's lock-at-launch.
fn range_autotarget(
    mut commands: Commands,
    q_torpedo: Query<
        (Entity, &GlobalTransform),
        (
            With<TorpedoProjectileMarker>,
            Without<TorpedoTargetEntity>,
            Without<TorpedoTargetChosen>,
        ),
    >,
    q_gates: Query<(Entity, &GlobalTransform), With<RangeGateMarker>>,
) {
    for (torpedo, torpedo_transform) in &q_torpedo {
        let origin = torpedo_transform.translation();
        let nearest = q_gates
            .iter()
            .min_by(|(_, a), (_, b)| {
                let da = a.translation().distance_squared(origin);
                let db = b.translation().distance_squared(origin);
                da.total_cmp(&db)
            })
            .map(|(gate, _)| gate);

        if let Some(gate) = nearest {
            commands
                .entity(torpedo)
                .insert((TorpedoTargetEntity(gate), TorpedoTargetChosen));
        }
    }
}

/// Track the closest any torpedo gets to its target - the readable PN quality
/// readout, and a DIAGNOSTIC rather than the invariant: the proximity fuze
/// despawns a torpedo at half the blast radius (15 u,
/// `torpedo_section/projectile.rs:91`), so no torpedo that works is ever
/// observed nearer than that and "closest approach under 15" only restates
/// "it detonated". [`track_lead_angle`] carries the claim instead.
///
/// `TorpedoTargetPosition` is rewritten to the target's live translation every
/// frame while the target is alive (`torpedo_section/projectile.rs:41`), so this
/// is a torpedo-to-TARGET distance, not a distance to a stale aim point.
fn track_closest_approach(
    mut best: ResMut<BestApproach>,
    q_torpedo: Query<(&GlobalTransform, &TorpedoTargetPosition), With<TorpedoProjectileMarker>>,
) {
    for (transform, target) in &q_torpedo {
        let distance = transform.translation().distance(**target);
        if distance < best.0 - 0.1 {
            best.0 = distance;
            info!("guidance: closest approach {:.1}", distance);
        }
    }
}

/// Track how far AHEAD of the line of sight a torpedo is steering - the one
/// observable that separates a lead solution from a stern chase, and the
/// quantity invariant 5 asserts.
///
/// Decompose the torpedo's heading against the line of sight to its target: the
/// component along the direction the TARGET is drifting across that line is the
/// lead. Pure pursuit points straight down the line of sight, so its lead is 0
/// whatever the range; a converged PN solution holds
/// `asin(|target drift| / torpedo speed)` all the way in - about 26 deg for this
/// scene's 15 u/s crosser and 31 u/s torpedo.
///
/// Sampled only inside [`LEAD_SAMPLE_RANGE`]; see that constant for why both
/// ends matter. Measured at 33 deg, 30 u out, on the run this was written
/// against.
fn track_lead_angle(
    time: Res<Time>,
    mut best: ResMut<BestLeadDeg>,
    q_torpedo: Query<
        (
            &GlobalTransform,
            &TorpedoTargetPosition,
            &TorpedoTargetEntity,
            &LinearVelocity,
        ),
        With<TorpedoProjectileMarker>,
    >,
    q_velocity: Query<&LinearVelocity>,
) {
    for (transform, target_position, target, velocity) in &q_torpedo {
        let to_target = **target_position - transform.translation();
        let range = to_target.length();
        if !LEAD_SAMPLE_RANGE.contains(&range) {
            continue;
        }
        let los = to_target / range;
        let Ok(target_velocity) = q_velocity.get(target.0) else {
            continue;
        };
        let across = target_velocity.0 - los * target_velocity.0.dot(los);
        let (Some(across), Some(heading)) = (across.try_normalize(), velocity.0.try_normalize())
        else {
            continue;
        };
        let lead = heading.dot(across).clamp(-1.0, 1.0).asin().to_degrees();
        best.first_sample_at.get_or_insert(time.elapsed_secs());
        if lead > best.deg + 0.5 {
            best.deg = lead;
            info!("guidance: lead angle {lead:.1} deg at {range:.1} u");
        }
    }
}

/// Throttled readout of the fastest torpedo in flight, to reveal whether the
/// torpedo is building enough speed to intercept.
fn report_torpedo_speed(
    time: Res<Time>,
    mut timer: ResMut<SpeedReportTimer>,
    q_torpedo: Query<&LinearVelocity, With<TorpedoProjectileMarker>>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    if let Some(speed) = q_torpedo.iter().map(|v| v.length()).max_by(f32::total_cmp) {
        info!("guidance: torpedo speed {:.1}", speed);
    }
}

/// The path one torpedo has flown, sampled for the range's trail gizmo.
///
/// It exists because the terminal weave is a CORKSCREW, and a corkscrew is
/// invisible in a single frame: the nose sits a couple of degrees off the
/// bearing and nothing else about the picture says "evading". Only the flown
/// path separates evasive ordnance from a drunk one, so the range draws it.
#[derive(Component, Default)]
struct RangeFlightPath(Vec<Vec3>);

/// How far a torpedo must move before the trail takes another sample.
const PATH_SAMPLE_SPACING: f32 = 0.5;

/// Samples one torpedo's trail holds, at [`PATH_SAMPLE_SPACING`] apart: enough
/// for the whole flight of the longest shot on either range.
const PATH_SAMPLE_LIMIT: usize = 600;

/// Trail length the weave frame waits for, in UNITS of flight - more than one
/// full turn of the helix at the Serpent's corkscrew rate.
///
/// In units and not in samples: a sample is taken at most once per frame, so a
/// sample count is a frame count on any run whose frames are longer than
/// [`PATH_SAMPLE_SPACING`] of travel - and it silently asks a slow run to fly
/// several times as far.
#[cfg(feature = "debug")]
const WEAVE_TRAIL_UNITS: f32 = 50.0;

/// Append each live torpedo's position to its trail.
fn record_flight_paths(
    mut commands: Commands,
    mut q_torpedo: Query<
        (Entity, &GlobalTransform, Option<&mut RangeFlightPath>),
        With<TorpedoProjectileMarker>,
    >,
) {
    for (torpedo, transform, path) in &mut q_torpedo {
        let position = transform.translation();
        let Some(mut path) = path else {
            commands
                .entity(torpedo)
                .insert(RangeFlightPath(vec![position]));
            continue;
        };
        if path
            .0
            .last()
            .is_none_or(|last| last.distance(position) >= PATH_SAMPLE_SPACING)
        {
            path.0.push(position);
            if path.0.len() > PATH_SAMPLE_LIMIT {
                path.0.remove(0);
            }
        }
    }
}

/// Draw the torpedo -> target line-of-sight, an armed/un-armed status sphere,
/// and the flown path behind it.
fn draw_guidance_gizmos(
    mut gizmos: Gizmos,
    q_torpedo: Query<
        (
            &GlobalTransform,
            &TorpedoTargetPosition,
            &TorpedoArming,
            Option<&RangeFlightPath>,
            Option<&TorpedoType>,
        ),
        With<TorpedoProjectileMarker>,
    >,
) {
    for (torpedo_transform, target, arming, path, torpedo_type) in &q_torpedo {
        let pos = torpedo_transform.translation();
        let status = if arming.is_armed() {
            tailwind::GREEN_400
        } else {
            tailwind::YELLOW_400
        };
        gizmos.sphere(Isometry3d::from_translation(pos), 0.6, status);
        gizmos.line(pos, **target, tailwind::RED_400);
        gizmos.sphere(
            Isometry3d::from_translation(**target),
            1.0,
            tailwind::RED_400,
        );
        if let Some(path) = path {
            // In the TYPE's own colour, so two trails in one frame say which
            // ordnance flew which path without a legend. A torpedo with no
            // type (nothing shipped) keeps the range's old trail blue.
            let trail = torpedo_type
                .map(|torpedo_type| torpedo_type.tint)
                .unwrap_or(tailwind::SKY_400.into());
            gizmos.linestrip(path.0.iter().copied(), trail);
        }
    }
}

fn log_torpedo_fired(add: On<Add, TorpedoProjectileMarker>) {
    info!("range: torpedo fired ({:?})", add.entity);
}

/// Log the moment a torpedo arms (once), so the arming delay is visible in logs.
fn log_torpedo_armed(
    mut commands: Commands,
    q_torpedo: Query<
        (Entity, &TorpedoArming),
        (With<TorpedoProjectileMarker>, Without<RangeArmLogged>),
    >,
) {
    for (torpedo, arming) in &q_torpedo {
        if arming.is_armed() {
            info!("range: torpedo {:?} armed", torpedo);
            commands.entity(torpedo).insert(RangeArmLogged);
        }
    }
}

fn log_torpedo_detonated(add: On<Add, NovaBlast>) {
    info!("range: torpedo detonated ({:?})", add.entity);
}

/// Fail the range loudly if a torpedo section and a ship section ever exchange
/// contact damage - the launch self-damage bug (task 20260709-131502). In this
/// range every torpedo is fired by the only ship, so any such pair is a
/// projectile hitting its owner, which the owner collision filter must prevent.
/// Blast damage does not trip this: its source is the standalone blast entity,
/// not a child of a torpedo or ship root. Under the headless autopilot the
/// panic fails the smoke run, so the filter wiring (the FILTER_PAIRS flag on
/// torpedo sections and the ProjectileHooks registration) stays regression-tested.
fn assert_no_owner_pair_damage(
    damage: On<HealthApplyDamage>,
    q_child_of: Query<&ChildOf>,
    q_torpedo: Query<(), With<TorpedoProjectileMarker>>,
    q_ship: Query<(), With<SpaceshipRootMarker>>,
    q_collider_of: Query<&ColliderOf>,
    q_owner: Query<&ProjectileOwner>,
) {
    let Some(source) = damage.source else {
        return;
    };
    let torpedo_section = |entity: Entity| {
        q_child_of
            .get(entity)
            .is_ok_and(|&ChildOf(root)| q_torpedo.contains(root))
    };
    let ship_section = |entity: Entity| {
        q_child_of
            .get(entity)
            .is_ok_and(|&ChildOf(root)| q_ship.contains(root))
    };
    let entity = damage.entity;
    if (torpedo_section(entity) && ship_section(source))
        || (ship_section(entity) && torpedo_section(source))
    {
        let describe = |e: Entity| {
            let collider_of = q_collider_of.get(e).map(|c| c.body).ok();
            let body_owner = collider_of.and_then(|b| q_owner.get(b).map(|o| o.0).ok());
            format!("{e:?} (ColliderOf {collider_of:?}, body's ProjectileOwner {body_owner:?})")
        };
        panic!(
            "range: owner-pair contact damage, {} <- {} ({:.2}): \
             the projectile owner collision filter is broken",
            describe(entity),
            describe(source),
            damage.amount
        );
    }
}

// --- The scripted run --------------------------------------------------------

/// The inputs the script holds down. A resource re-applied every frame rather
/// than a one-shot press: `ButtonInput` is a live map the game's own systems
/// read every frame, and the weapons safety (task 20260713-082337) derives
/// `WeaponsHot` from the HELD combat input - a press applied once on a step's
/// entry would be a press, not a hold.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct HeldInput {
    /// RMB - the combat stance the safety gates firing on.
    combat: bool,
    /// Space - the trigger.
    fire: bool,
}

/// Re-press the held inputs for the frame.
///
/// Wired as the script's [`AutopilotPlugin::input`] hook, NOT as an `Update`
/// system. That slot runs in `PreUpdate` after `InputSystems`
/// (nova_autopilot/src/autopilot.rs:384), which is the only place a synthesized
/// press is still `just_pressed` when the game's `Update` input systems read
/// it. As an unordered `Update` system this raced `SpaceshipInputSystems` and
/// lost: the stance still went hot (the safety reads the HELD button), but the
/// trigger's single `just_pressed` edge was cleared before the weapon saw it,
/// so the range never fired a round.
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
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
    }
}

/// The round label the gate scene runs under.
#[cfg(feature = "debug")]
const GATE_ROUND: &str = "gate round";

/// The round label the crossing scene runs under, so invariants 5 and 6 fire on
/// that pass and only there.
#[cfg(feature = "debug")]
const CROSSING_ROUND: &str = "crossing round";

/// How far ahead of the line of sight the torpedo must steer, in degrees, for
/// the guidance to count as LEADING the crosser rather than chasing it.
///
/// Pure pursuit scores 0 by construction, the ideal constant-bearing lead for
/// this scene is ~26 deg, and the live guidance holds ~33 (see
/// [`track_lead_angle`]) - it flies the lead slightly hot on the way in. The
/// bound sits below all of that with margin, so it fails a chase and passes any
/// solution that actually leads.
#[cfg(feature = "debug")]
const LEAD_MIN_DEG: f32 = 12.0;

/// How long the launch beat holds the trigger after a torpedo has ARMED, in
/// seconds of in-step time.
///
/// A settle beat, and deliberately NOT the whole `fired && armed && detonated
/// && gate_damaged` chain - that is exactly what [`assert_launch_chain`] then
/// asserts, and gating on it left four invariants unfailable: a dud torpedo
/// could only surface as a deadline stall on this beat's name, never as the
/// message that names WHICH link broke. The beat delivers the stimulus (an
/// armed torpedo is in flight); whether it DETONATED and DAMAGED the target is
/// the assertion's question.
///
/// Sized off the flight: the gate sits ~90 u out and a torpedo caps at ~35 u/s
/// from a standing start, so a healthy launch chain closes in ~5s. The bay
/// keeps launching while the trigger is down, so this window also covers a
/// second shot.
#[cfg(feature = "debug")]
const LAUNCH_SETTLE_SECS: f32 = 10.0;

/// How long the lead beat holds after the FIRST midcourse sample, in seconds of
/// in-step time.
///
/// A settle beat, for the same reason: gating on the lead ANGLE is the
/// comparison [`assert_leads_the_crosser`] makes, so a pure-pursuit guidance
/// would stall this beat instead of failing invariant 5 with its measured
/// angle. The stimulus is a torpedo flying the midcourse leg; the angle it held
/// is the assertion's question.
///
/// Sized off [`LEAD_SAMPLE_RANGE`]: the window is 14 u wide and a midcourse
/// torpedo crosses it at ~31 u/s, so ~0.5s covers the whole leg. This is three
/// times that, so the running maximum has seen the leg out.
#[cfg(feature = "debug")]
const LEAD_SETTLE_SECS: f32 = 1.5;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

/// The scripted run: raise the stance and hold the trigger through a launch
/// window on the gate range, then load the CROSSING range and do it again
/// against a fast lateral mover, where the guidance also has to hold a lead
/// angle over [`LEAD_MIN_DEG`].
///
/// Not the stock `nova_autopilot()` preset: every beat waits on the value it
/// depends on - `WeaponsHot`, a torpedo arming, a midcourse leg being flown -
/// so a slow load or a slow intercept delays the walk instead of truncating it.
/// No beat reads a quantity an assertion decides; the two settles state their
/// reasons on [`LAUNCH_SETTLE_SECS`] and [`LEAD_SETTLE_SECS`]. The
/// per-step deadlines NAME the beat that stalled; their
/// runtime sum (20 + 28 + 15 + 28 + 20 + 5 = 116s, counting `fire_round`'s
/// beats once per CALL) stays under `DEFAULT_DEADLINE_SECS` (120s) so a named
/// stall wins the race against the generic collector deadline.
#[cfg(feature = "debug")]
fn torpedo_script() -> Script {
    let script = Script::new()
        // Every step re-presses whatever the script is holding down; see
        // `hold_inputs` for why this must be the plugin hook and not a system.
        .input(hold_inputs)
        .step("load the range")
        .enter(GameStates::Loading)
        .until(and(scenario_is(RANGE_ID), player_ship_present()))
        .deadline(20.0)
        .add();
    let script = fire_round(script, GATE_ROUND);
    let script = script
        // Wait for a scene that is DEMONSTRABLY the crossing one: it carries a
        // DISTINCT scenario id, so `CurrentScenario` naming it can only mean the
        // second load landed. Paired with the ship and the crosser being up,
        // since those are what the round goes on to use.
        //
        // Not "wait for the ship to be gone, then back": there is no such
        // frame. `on_load_scenario` queues the scoped despawns and fires
        // `OnStartEvent` on the SAME `Commands`
        // (nova_scenario/src/loader/lifecycle.rs:162-253), so the teardown and
        // the respawn land in one flush and a gone-then-present pair stalls out
        // its deadline instead.
        .step("load the crossing range")
        .on_enter(load_crossing_range)
        .until(and(
            scenario_is(CROSSING_ID),
            and(player_ship_present(), crosser_present()),
        ))
        .deadline(15.0)
        .add();
    fire_round(script, CROSSING_ROUND)
        // Keep the trigger down: the bay keeps launching, and each fresh
        // torpedo gets another run at the intercept while the crosser keeps
        // moving.
        .step("lead the crosser")
        .until(lead_sample_settled())
        .deadline(20.0)
        .add()
        .step("assert the guidance led the crosser")
        .on_enter(assert_leads_the_crosser)
        .add()
        // The weave frame, taken LAST and on the CROSSING range: the crosser
        // walks the corridor sideways, so by now a torpedo's leg is several
        // times the gate range's and long enough to draw a turn of the helix.
        // The gate scene cannot show the weave at all - every gate sits inside
        // the terminal fade band, where the weave is deliberately off.
        .step("let a torpedo fly a trail worth framing")
        .until(weave_trail_flown())
        .deadline(8.0)
        .add()
        .step("frame the terminal weave")
        .on_enter(frame_the_weave)
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the terminal weave")
        .on_enter(|world: &mut World| shoot(world, "variant-torpedo-weave.png"))
        .until(shot_written("variant-torpedo-weave.png"))
        .deadline(5.0)
        .add()
}

/// Put the camera over ONE torpedo's flown trail for the weave frame: a
/// near-PLAN view, framed to the trail's own extent.
///
/// The framing is the whole point of the shot. Down the chase camera's axis a
/// corkscrew is foreshortened into a smear and reads as nothing; from above,
/// the helix projects onto the flight plane as a wave and the trail says
/// plainly whether the ordnance is evading or staggering.
///
/// It frames the LONGEST trail in the air rather than the whole salvo, and
/// sizes the camera off that trail. Framed to fit every torpedo at once, the
/// corridor is several hundred units wide and an ~11 u swing is a couple of
/// pixels, which is a picture of nothing.
#[cfg(feature = "debug")]
fn frame_the_weave(world: &mut World) {
    // The HUD chrome sits over the corridor at this framing.
    nova_protocol::nova_debug::harness::hide_hud(world);
    // The longest WEAVING trail: the range fires both types off one trigger, so
    // the longest trail outright is as likely to be a Lance running dead
    // straight, and this shot exists to show the corkscrew.
    let trail = world
        .iter_entities()
        .filter(|entity| entity.contains::<TorpedoProjectileMarker>() && is_weaving(entity))
        .filter_map(|entity| entity.get::<RangeFlightPath>())
        .max_by(|a, b| trail_extent(a).total_cmp(&trail_extent(b)))
        .map(|path| path.0.clone())
        .unwrap_or_default();
    // Nothing in the air: frame the corridor the crossing scene uses, so the
    // shot is still a picture of the range rather than of the origin.
    let (centre, extent) = if trail.is_empty() {
        (Vec3::new(0.0, 0.0, -60.0), 120.0)
    } else {
        let first = trail[0];
        let last = trail[trail.len() - 1];
        ((first + last) * 0.5, first.distance(last).max(40.0))
    };
    // Height set off the trail so it fills the frame; tilted ~15 degrees off
    // vertical because looking straight down -Y is a `look_at` singularity
    // against the default up vector.
    let height = extent * 0.8;
    nova_protocol::nova_debug::harness::pose_camera(
        world,
        centre + Vec3::new(0.0, height, height * 0.27),
        centre,
    );
}

/// One full launch chain - fired, armed, detonated, target damaged - appended to
/// `script`. Called once per scene, which is what makes invariant 6 a claim
/// about the chain holding somewhere other than where it was written.
#[cfg(feature = "debug")]
fn fire_round(script: Script, round: &'static str) -> Script {
    script
        // Raise the combat stance and wait for the safety to actually go hot.
        // Pressing fire before it does latches nothing, forever: the safety
        // DENIES a press that arrives while cold, and a held key produces no
        // fresh Start edge once hot. The outcome assertions below caught
        // exactly that - both ranges fired nothing for a while after the
        // safety landed (task 20260712-211352).
        .step("raise the weapons")
        .on_enter(|world: &mut World| world.resource_mut::<HeldInput>().combat = true)
        .until(weapons_are_hot())
        .deadline(6.0)
        .add()
        .step("hold the trigger")
        .on_enter(|world: &mut World| world.resource_mut::<HeldInput>().fire = true)
        .until(and(torpedo_armed(), elapsed(LAUNCH_SETTLE_SECS)))
        .deadline(22.0)
        .add()
        .step("assert the launch chain")
        .on_enter(move |world: &mut World| assert_launch_chain(world, round))
        .add()
}

/// Load the crossing range, and clear the per-round state the fresh scene has to
/// re-establish: the observed outcomes, the approach metric (the gate round's
/// torpedoes already drove it down, and invariant 5 is a claim about the
/// CROSSING scene) and the held trigger. The stance stays raised - the fresh
/// ship's safety re-derives it from the held input, and `raise the weapons`
/// waits for that.
#[cfg(feature = "debug")]
fn load_crossing_range(world: &mut World) {
    let config = {
        let game_assets = world.resource::<GameAssets>();
        let sections = world.resource::<GameSections>();
        crossing_range(game_assets, sections)
    };
    *world.resource_mut::<RangeOutcome>() = RangeOutcome::default();
    world.resource_mut::<BestApproach>().0 = f32::INFINITY;
    *world.resource_mut::<BestLeadDeg>() = BestLeadDeg::default();
    // RELEASE the trigger, do not merely stop re-pressing it. `ButtonInput`
    // only yields a fresh `just_pressed` on a not-pressed -> pressed edge, and
    // nothing else releases a synthesized key: leaving it latched from the gate
    // round would make the crossing round's press a no-op, so the fresh ship
    // would never fire.
    world.resource_mut::<HeldInput>().fire = false;
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::Space);
    info!("range: loading the crossing range");
    world.trigger(LoadScenario(config));
}

/// The player ship's weapons safety has gone hot.
#[cfg(feature = "debug")]
fn weapons_are_hot() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .try_query_filtered::<&WeaponsHot, With<PlayerSpaceshipMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|hot| hot.0))
    })
}

/// The live scenario is the one named `id`.
#[cfg(feature = "debug")]
fn scenario_is(id: &'static str) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<CurrentScenario>(move |current| {
        current.0.as_ref().is_some_and(|config| config.id == id)
    })
}

/// The crossing target exists.
///
/// Gated on alongside the ship, not implied by it: the round's approach
/// invariant is measured against this target, and the ship spawning does not
/// imply the scenario's objects have.
#[cfg(feature = "debug")]
fn crosser_present() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    any_entity::<With<RangeCrosser>>()
}

/// A torpedo has left the bay and ARMED - the strictly weaker prefix of the
/// chain [`assert_launch_chain`] decides.
#[cfg(feature = "debug")]
fn torpedo_armed() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<RangeOutcome>(|outcome| outcome.fired && outcome.armed)
}

/// Some torpedo in the air has flown far enough for its trail to SHOW the
/// weave.
///
/// The weave frame needs a torpedo mid-flight with a path behind it, and the
/// launch beat can perfectly well end on a frame between a detonation and the
/// next launch - which framed an empty corridor. Sized at
/// [`WEAVE_TRAIL_UNITS`] of flown ground.
#[cfg(feature = "debug")]
fn weave_trail_flown() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .iter_entities()
            .filter(|entity| is_weaving(entity))
            .filter_map(|entity| entity.get::<RangeFlightPath>())
            .any(|path| trail_extent(path) >= WEAVE_TRAIL_UNITS)
    })
}

/// How much ground a trail covers, end to end. Both types run essentially
/// straight over this distance, so the end-to-end span is a fair stand-in for
/// the flown length and costs nothing to take.
#[cfg(feature = "debug")]
fn trail_extent(path: &RangeFlightPath) -> f32 {
    match (path.0.first(), path.0.last()) {
        (Some(first), Some(last)) => first.distance(*last),
        _ => 0.0,
    }
}

/// Whether this entity is a torpedo that actually WEAVES.
///
/// The range flies both shipped types off one trigger, so "the longest trail in
/// the air" is a coin flip between a corkscrew and a straight run - and the
/// weave shot is worthless framed on the straight one. Everything about the
/// picture that has to be a weave asks here first.
#[cfg(feature = "debug")]
fn is_weaving(entity: &bevy::ecs::world::EntityRef) -> bool {
    entity
        .get::<TorpedoWeave>()
        .is_some_and(|weave| weave.angle > 0.0)
}

/// A torpedo has flown the midcourse leg [`track_lead_angle`] samples over.
#[cfg(feature = "debug")]
fn lead_sample_settled() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some(first) = world
            .get_resource::<BestLeadDeg>()
            .and_then(|best| best.first_sample_at)
        else {
            return false;
        };
        world
            .get_resource::<Time>()
            .is_some_and(|time| time.elapsed_secs() - first >= LEAD_SETTLE_SECS)
    })
}

/// Invariants 1-4: a torpedo fired, armed, detonated, and its blast damaged a
/// target. On the crossing round this is also invariant 6 - the same chain held
/// in a second scene - since reaching it means every earlier beat resolved.
#[cfg(feature = "debug")]
fn assert_launch_chain(world: &mut World, round: &str) {
    let outcome = world.resource::<RangeOutcome>();
    assert!(
        outcome.fired,
        "range ({round}): no torpedo fired in the window"
    );
    assert!(
        outcome.armed,
        "range ({round}): no torpedo armed in the window"
    );
    assert!(
        outcome.detonated,
        "range ({round}): no torpedo detonated in the window"
    );
    assert!(
        outcome.gate_damaged,
        "range ({round}): no gate took blast damage in the window"
    );
    info!("range: {round} - fired -> armed -> detonated -> gate damaged, all observed");
    let elapsed = world.resource::<Time>().elapsed_secs();
    let beat = serde_json::json!({ "t": elapsed, "round": round });
    nova_probe::probe_marker(world, "outcome: torpedo fired", beat.clone());
    nova_probe::probe_marker(world, "outcome: torpedo armed", beat.clone());
    nova_probe::probe_marker(world, "outcome: torpedo detonated", beat.clone());
    nova_probe::probe_marker(world, "outcome: gate damaged", beat);
    if round == CROSSING_ROUND {
        nova_probe::probe_marker(
            world,
            "outcome: launch chain holds in the crossing scene",
            serde_json::json!({ "t": elapsed }),
        );
    }
}

/// Invariant 5: the guidance LED the crossing target rather than chasing it.
///
/// The PN solution was only ever LOGGED before; asserting it is what turns it
/// into a regression-tested claim.
#[cfg(feature = "debug")]
fn assert_leads_the_crosser(world: &mut World) {
    let lead = world.resource::<BestLeadDeg>().deg;
    let approach = world.resource::<BestApproach>().0;
    assert!(
        lead > LEAD_MIN_DEG,
        "range ({CROSSING_ROUND}): the torpedo steered {lead:.1} deg ahead of the \
         line of sight (need more than {LEAD_MIN_DEG}) - the guidance is chasing \
         the crosser, not leading it"
    );
    info!("range: {CROSSING_ROUND} - lead angle {lead:.1} deg, closest approach {approach:.1} u");
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: torpedo leads the crosser",
        serde_json::json!({ "t": elapsed, "lead_deg": lead, "closest_approach": approach }),
    );
}
