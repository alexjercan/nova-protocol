//! system_turret_gunnery: a turret acquires, tracks, fires and hits at range.
//!
//! One player ship carrying a single turret sits at the origin. A spread of
//! asteroid "gates" sits in the turret's firing arc, and one gate sweeps back and
//! forth across the front so you can watch the turret track a mover - the point
//! being to judge how cleanly the turret aims and fires, and to make the aiming
//! cheap to tune and regression-test.
//!
//! The range points the turret at the moving gate (the crosshair does this in the
//! full game); the barrel slews to follow and fires when you hold the trigger.
//!
//! What it shows:
//! - Bullet gravity: a planetoid slung below the firing lane bends rounds that
//!   cross its sphere of influence downward toward it. The shooter's own
//!   gravity is stripped so it stays a fixed frame; only the rounds curve, so
//!   the straight-line lead pip visibly misses low as a round nears the rock.
//!   Spike: docs/spikes/20260712-112113-bullets-affected-by-gravity.md.
//! - Aim gizmos: a line down the barrel (green when it is on target, yellow while
//!   it lags) and a red line + sphere at the point the turret is aiming for. The
//!   gap between the barrel line and the target line is the tracking lag - the
//!   "clunky" aiming this range exists to expose.
//! - `turret: aim error N.N deg, M bullets in flight` (throttled) so tracking
//!   quality is visible headless.
//! - A tuning panel (top-left) of live sliders for the turret's knobs - yaw/pitch
//!   speed, pitch limits, fire rate, muzzle speed - so you can retune while
//!   watching the aim-error readout instead of editing the config and re-running.
//!
//! Controls: hold right mouse to raise weapons (the safety keeps a lowered
//! ship cold), Space (or right trigger) fires. Aiming is automatic (tracks the
//! moving gate); in the full game you aim with the mouse. Drag the sliders to
//! retune the turret live (they are inert headless under the autopilot).
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_turret_gunnery --features debug
//! # look for: `nova harness: reached Playing`, `turret: aim error ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[path = "system_turret_gunnery/slider.rs"]
mod slider;

use std::collections::BTreeMap;
#[cfg(feature = "debug")]
use std::sync::Arc;

use avian3d::prelude::*;
use bevy::{
    color::palettes::tailwind,
    prelude::*,
    ui_widgets::{observe, ValueChange},
};
use clap::Parser;
use nova_protocol::prelude::*;
use slider::{slider, SliderWidgetPlugin};

#[derive(Parser)]
#[command(name = "system_turret_gunnery")]
#[command(version = "1.0.0")]
#[command(about = "A test range for the PDC turret section in nova_protocol. Autopilot-only correctness range - the hand-held trigger is a debug aid, not the subject", long_about = None)]
struct Cli;

/// Id of the gate that sweeps across the front for the turret to track.
const MOVING_GATE_ID: &str = "gate_moving";

/// Centre of the sweeping gate's path.
const MOVING_GATE_ORIGIN: Vec3 = Vec3::new(-35.0, 6.0, -55.0);

/// Every authored range gate: id, display name, and position. The single
/// roster - the scenario spawns exactly these and [`tag_gate`] tags exactly
/// these, so the range's OTHER asteroid (the gravity planetoid) cannot drift
/// into the gate count.
const RANGE_GATES: [(&str, &str, Vec3); 5] = [
    ("gate_front", "Front Gate", Vec3::new(0.0, 3.0, -55.0)),
    ("gate_left", "Left Gate", Vec3::new(-32.0, 4.0, -45.0)),
    ("gate_right", "Right Gate", Vec3::new(30.0, 3.0, -48.0)),
    ("gate_high", "High Gate", Vec3::new(6.0, 26.0, -40.0)),
    // The sweeping gate is the one target the script REQUIRES alive: the
    // tracking beat waits on its live position, so its despawn stalls the run
    // outright. It is also the most punished object on the range - it is what
    // the turret aims at, and its sweep drives it through the static gates'
    // lane, where contact damage alone took it from full to destroyed inside
    // one round. The single-round version of this range ended before that
    // mattered; a multi-round one has to outlast it, so this gate gets the
    // planetoid's durability rather than a gate's. Still damageable, which is
    // what invariant 2 observes.
    (MOVING_GATE_ID, "Moving Gate", MOVING_GATE_ORIGIN),
];

/// Half-width of the sweep, in world units. Chosen so the path stays clear of
/// the static gates - `gate_front` sits at x = 0, and a sweep that reached it
/// put the two within their combined radii once per pass, where contact damage
/// destroyed the mover mid-run.
const SWEEP_AMPLITUDE: f32 = 22.0;

/// Angular rate of the sweep, in radians per second (a ~10.5 s round trip).
const SWEEP_RATE: f32 = 0.6;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // Headless smoke-test harness: inert in a normal run. Under NOVA_AUTOPILOT
    // it holds the fire key and asserts the range's PURPOSE: rounds left the
    // barrel, a range target actually took hits (task 20260712-211352 -
    // reach-Playing alone let a turret that never connects pass), and the
    // barrel converged
    // on a MOVING target. Scene is built on `GameAssetsStates::Loaded` so the
    // screenshot's forced Playing does not re-run setup.
    #[cfg(feature = "debug")]
    {
        app.init_resource::<RangeOutcome>();
        app.init_resource::<HeldInput>();
        app.add_observer(
            |_: On<Add, TurretBulletProjectileMarker>, mut outcome: ResMut<RangeOutcome>| {
                if !outcome.fired {
                    info!("range: first round left the barrel");
                }
                outcome.fired = true;
            },
        );
        // The SOURCE clause is load-bearing: the gates drift into each other
        // and into the planetoid, and that collision damage tripped this flag
        // within a frame of the range loading - long before any round could
        // cross the ~55 u to the nearest gate. Requiring the source to be a
        // turret round is what makes "a target took hits" a claim about the
        // turret. `resolve_bullet_hit` (nova_ship turret_section/firing.rs)
        // passes the bullet's body as `source` through `spend_piercing_damage`
        // -> `apply_damage`; the contact path names the other COLLIDER
        // instead (nova_gameplay integrity/core.rs), which is the drift this
        // clause rejects.
        app.add_observer(
            |damage: On<HealthApplyDamage>,
             q_target: Query<(), Or<(With<RangeGateMarker>, With<RangeBackstopMarker>)>>,
             q_bullet: Query<(), With<TurretBulletProjectileMarker>>,
             mut outcome: ResMut<RangeOutcome>| {
                if q_target.contains(damage.entity)
                    && damage
                        .source
                        .is_some_and(|source| q_bullet.contains(source))
                {
                    if !outcome.target_hit {
                        info!("range: a turret round connected with a range target");
                    }
                    outcome.target_hit = true;
                }
            },
        );
        // Probe wiring (task 20260719-210443; each plugin is inert without
        // its NOVA_PROBE_* env): run timeline + engine-bound invariants +
        // frame-time capture, so `probe run` can measure this example.
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(nova_screenshot(turret_script()));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.insert_resource(StatusTimer(Timer::from_seconds(0.5, TimerMode::Repeating)));
    app.add_systems(
        OnEnter(GameAssetsStates::Loaded),
        (setup_range, setup_tuning_ui),
    );
    app.add_systems(
        Update,
        (
            // Aim after the player crosshair system so the range aim wins - the
            // range points the turret at the moving gate rather than straight
            // ahead.
            range_aim.after(SpaceshipInputSystems),
            drive_moving_gate,
            anchor_shooter_against_gravity,
            draw_turret_aim,
            report_status,
            update_knob_labels,
        ),
    );
    app.add_observer(tag_gate);
    app.add_plugins(SliderWidgetPlugin);
    #[cfg(feature = "debug")]
    {
        app.init_resource::<AimSettle>();
        app.add_systems(Update, track_aim_settle);
    }
}

/// How long the barrel's aim error has stopped changing - i.e. how long the
/// slew has been in steady state rather than still swinging onto the target.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct AimSettle {
    last_deg: Option<f32>,
    still_frames: u32,
}

/// Sample the live aim error and count the frames its RATE of change has stayed
/// under [`AIM_STEADY_RATE_DEG_PER_SEC`].
#[cfg(feature = "debug")]
fn track_aim_settle(world: &mut World) {
    let error = aim_error_deg(world);
    let dt = world.resource::<Time>().delta_secs();
    let mut settle = world.resource_mut::<AimSettle>();
    match (settle.last_deg, error) {
        (Some(last), Some(now))
            if dt > 0.0 && (last - now).abs() / dt < AIM_STEADY_RATE_DEG_PER_SEC =>
        {
            settle.still_frames += 1;
        }
        _ => settle.still_frames = 0,
    }
    settle.last_deg = error;
}

/// Marks an asteroid the range treats as a target gate.
#[derive(Component)]
struct RangeGateMarker;

/// Marks the gravity planetoid - a range target, but not a gate.
///
/// Separate from [`RangeGateMarker`] because the two answer different
/// questions. The gate marker is the aim roster and the gate COUNT; the backstop
/// is where the range's whole point sends most rounds. The turret aims straight
/// and the well bends its rounds down, so a barrel converged to 0.5 deg puts
/// 12-16 rounds a second UNDER the gate and into this rock. Tagging the
/// planetoid a gate hid both facts at once: `report_status` printed 6 gates for
/// 5, and "a gate took hits" was answered by a rock.
#[derive(Component)]
struct RangeBackstopMarker;

/// Marks the gate that sweeps across the front (the turret's tracking target).
#[derive(Component)]
struct RangeMovingTarget;

/// Throttle for the status readout.
#[derive(Resource)]
struct StatusTimer(Timer);

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(turret_range(
        &game_assets,
        &sections,
        RANGE_ID,
    )));
}

/// The scenario id the first load runs under. The reload uses
/// [`RELOADED_RANGE_ID`] instead, so the reload gate can observe WHICH load is
/// live rather than guessing from world contents.
const RANGE_ID: &str = "turret_range";

/// The scenario id the reload runs under.
#[cfg(feature = "debug")]
const RELOADED_RANGE_ID: &str = "turret_range_reloaded";

/// The round label the reload pass runs under, so invariant 4 fires on that
/// pass and only there.
#[cfg(feature = "debug")]
const RELOADED_ROUND: &str = "round 2";

fn turret_range(game_assets: &GameAssets, sections: &GameSections, id: &str) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };

    let ship = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::from([(
                "turret".to_string(),
                vec![KeyCode::Space.into(), GamepadButton::RightTrigger.into()],
            )]),
            speed_cap: None,
            // Dev/tuning harness: fire freely.
            infinite_ammo: true,
        }),
        hull: ShipSource::Inline(ShipHull {
            sections: vec![
                SpaceshipSectionConfig {
                    id: "controller".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Inline(section("basic_controller_section")),
                    modifications: vec![],
                },
                SpaceshipSectionConfig {
                    id: "hull".to_string(),
                    position: Vec3::new(0.0, 0.0, 1.0),
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Inline(section("reinforced_hull_section")),
                    modifications: vec![],
                },
                SpaceshipSectionConfig {
                    id: "turret".to_string(),
                    position: Vec3::new(0.0, 0.0, -0.75),
                    // The shipped ships mount their turrets unrotated on a face
                    // that already points up; this one sits on the -Z end, so it
                    // needs the quarter turn to stand its base upright.
                    rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                    source: SectionSource::Inline(section("pdc_kinetic_turret_section")),
                    modifications: vec![],
                },
            ],
            ..default()
        }),
        ..default()
    };

    // Geometry-authoritative gates in the turret's firing arc, plus the
    // sweeping one. Their full rock volume keeps the range alive under fire.
    let gate = |id: &str, name: &str, pos: Vec3| ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position: pos,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            material: None,
            destroy_sound: Some("base/sounds/destroy_rock.wav".into()),
            radius: 2.0,
            texture: game_assets.asteroid_texture.clone().into(),
            mass: None,
            invulnerable: false,
            seed: None,
            lock_signature: None,
        }),
    };

    let mut objects = vec![ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "player_ship".to_string(),
            name: "Turret Ship".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(ship),
    }];
    objects.extend(
        RANGE_GATES
            .iter()
            .map(|(id, name, position)| gate(id, name, *position)),
    );
    objects.push(
        // A gravity planetoid slung below the firing lane so rounds crossing
        // its sphere of influence curve downward toward it - the range is where
        // you eyeball bullet gravity (docs/spikes/20260712-112113). It is
        // authored heavy (mass 30 000 -> SOI 346u, and 3.3-9.6 u/s^2 at the
        // geometric surface), which is why the shooter's own gravity is
        // stripped below: otherwise the ship, sitting inside that SOI, would
        // fall out of frame instead of holding still as a reference.
        ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "gravity_rock".to_string(),
                name: "Planetoid".to_string(),
                position: Vec3::new(0.0, -22.0, -50.0),
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                material: None,
                destroy_sound: Some("base/sounds/destroy_rock.wav".into()),
                radius: 16.0,
                texture: game_assets.asteroid_texture.clone().into(),
                mass: Some(30_000.0),
                invulnerable: true,
                seed: None,
                lock_signature: None,
            }),
        },
    );

    let events = vec![ScenarioEventConfig {
        label: None,
        name: EventConfig::OnStart,
        once: false,
        filters: vec![],
        // The range lights itself: the engine spawns no light, so a scenario
        // that authors none renders black.
        actions: objects
            .into_iter()
            .map(EventActionConfig::SpawnScenarioObject)
            .chain(ThreePointRig::around("range", Vec3::ZERO, 3.0).actions())
            .collect(),
    }];

    ScenarioConfig {
        description: "A test range for the PDC turret section.".to_string(),
        events,
        ..ScenarioConfig::new(
            id.to_string(),
            "Turret Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Tag each spawned range asteroid as a GATE or the BACKSTOP, and single out the
/// sweeping gate.
///
/// Split by the roster rather than tagging every `Add<AsteroidMarker>` a gate:
/// the gravity planetoid is an asteroid too, so one marker for both made
/// `report_status` print 6 gates for 5 and let the rock answer the
/// "a round connected with a gate" claim. Both are still range TARGETS, which is
/// what invariant 2 observes - see [`RangeBackstopMarker`].
fn tag_gate(add: On<Add, AsteroidMarker>, mut commands: Commands, q_id: Query<&EntityId>) {
    let entity = add.entity;
    let Ok(id) = q_id.get(entity) else {
        return;
    };
    if !RANGE_GATES.iter().any(|(gate_id, ..)| *gate_id == id.0) {
        commands.entity(entity).insert(RangeBackstopMarker);
        return;
    }
    commands.entity(entity).insert(RangeGateMarker);

    if id.0 == MOVING_GATE_ID {
        // `RigidBody::Kinematic` is load-bearing, not a tweak: it is what makes
        // the gate follow the path `drive_moving_gate` authors instead of
        // negotiating with the solver. As a dynamic body it was pulled by the
        // planetoid, shoved by bullet impacts and stopped dead on the driven
        // axis. Kinematic bodies still collide and still take damage, so
        // invariant 2 is unaffected. `SleepingDisabled` keeps avian from
        // parking it during the quiet beats.
        commands.entity(entity).insert((
            RangeMovingTarget,
            RigidBody::Kinematic,
            LinearVelocity::default(),
            SleepingDisabled,
        ));
    }
}

/// Keep the range ship a fixed reference frame despite the gravity planetoid.
/// Ship roots opt into `GravityAffected` on spawn (nova_gameplay), and the
/// shooter sits inside the planetoid's SOI; without this it would slowly fall
/// toward the rock and drift the whole range. Rounds still feel gravity (they
/// get their own `GravityAffected` at spawn), so this strips only the shooter's
/// pull, not the effect the range exists to show. Range-local: the real game
/// leaves ships gravity-affected. `DominantWell` is removed alongside because
/// the force system may have inserted it in the frame before the strip and only
/// cleans it up for still-affected entities.
fn anchor_shooter_against_gravity(
    mut commands: Commands,
    q_ship: Query<Entity, (With<SpaceshipRootMarker>, With<GravityAffected>)>,
) {
    for ship in &q_ship {
        commands
            .entity(ship)
            .remove::<GravityAffected>()
            .remove::<DominantWell>();
    }
}

/// Sweep the moving gate across the front so the turret has to track it.
///
/// The gate is placed ON its path each frame rather than pushed along it by
/// `LinearVelocity`. As a dynamic body it was a rock in a scene full of forces:
/// the planetoid pulled it, bullet impacts shoved it, contacts with the static
/// gates damaged it, and - the flake that cost the most to find - a velocity
/// written every frame in `Update` simply did not translate into motion on the
/// driven axis, so the gate carried 18 u/s while sitting still. The tracking
/// beat waits on this gate having TRAVELLED, so that stalled the run about half
/// the time. Authored motion also makes the beat repeatable, which is what an
/// assertion about tracking wants.
///
/// [`LinearVelocity`] is still written, because it is what `range_aim` feeds the
/// turret's lead solution; here it is the analytic derivative of the path rather
/// than a request to the solver.
fn drive_moving_gate(
    time: Res<Time>,
    mut q_gate: Query<(&mut Transform, &mut LinearVelocity), With<RangeMovingTarget>>,
) {
    let phase = time.elapsed_secs() * SWEEP_RATE;
    for (mut transform, mut velocity) in &mut q_gate {
        transform.translation.x = MOVING_GATE_ORIGIN.x + phase.sin() * SWEEP_AMPLITUDE;
        transform.translation.y = MOVING_GATE_ORIGIN.y;
        transform.translation.z = MOVING_GATE_ORIGIN.z;
        velocity.0 = Vec3::new(phase.cos() * SWEEP_AMPLITUDE * SWEEP_RATE, 0.0, 0.0);
    }
}

/// Point the turret at the sweeping gate (falls back to any gate), so the range
/// exercises tracking without a mouse. Runs after the crosshair aim so it wins.
/// Also feeds the gate's velocity so the turret leads the mover.
fn range_aim(
    mut q_turret: Query<
        (
            &mut TurretSectionTargetInput,
            &mut TurretSectionTargetVelocity,
        ),
        With<TurretSectionMarker>,
    >,
    q_moving: Query<(&GlobalTransform, &LinearVelocity), With<RangeMovingTarget>>,
    q_gates: Query<&GlobalTransform, With<RangeGateMarker>>,
) {
    let (target, velocity) = if let Some((transform, linear_velocity)) = q_moving.iter().next() {
        (Some(transform.translation()), linear_velocity.0)
    } else if let Some(transform) = q_gates.iter().next() {
        (Some(transform.translation()), Vec3::ZERO)
    } else {
        (None, Vec3::ZERO)
    };

    for (mut turret_target, mut turret_velocity) in &mut q_turret {
        **turret_target = target;
        **turret_velocity = velocity;
    }
}

/// Draw the barrel direction (green on target, yellow while lagging) and the line
/// to the turret's lead aim point, so the tracking lag is visible.
fn draw_turret_aim(
    mut gizmos: Gizmos,
    q_muzzle: Query<&GlobalTransform, With<TurretSectionBarrelMuzzleMarker>>,
    q_turret: Query<&TurretSectionAimPoint, With<TurretSectionMarker>>,
) {
    let Some(aim) = q_turret.iter().next().and_then(|a| **a) else {
        return;
    };
    gizmos.sphere(Isometry3d::from_translation(aim), 1.5, tailwind::RED_400);

    for muzzle in &q_muzzle {
        let pos = muzzle.translation();
        let barrel = muzzle.forward();
        let to_aim = (aim - pos).normalize_or_zero();
        let color = if barrel.dot(to_aim) > 0.999 {
            tailwind::GREEN_400
        } else {
            tailwind::YELLOW_400
        };
        gizmos.line(pos, pos + barrel * 60.0, color);
        gizmos.line(pos, aim, tailwind::RED_400);
    }
}

/// Throttled readout of the aiming error (barrel vs. the turret's lead aim point)
/// and the number of bullets in flight, so tracking quality and firing are legible
/// headless. With the lead solution this should stay in the single digits even
/// against the sweeping gate.
fn report_status(
    time: Res<Time>,
    mut timer: ResMut<StatusTimer>,
    q_muzzle: Query<&GlobalTransform, With<TurretSectionBarrelMuzzleMarker>>,
    q_turret: Query<&TurretSectionAimPoint, With<TurretSectionMarker>>,
    q_bullets: Query<(), With<TurretBulletProjectileMarker>>,
    q_moving: Query<(), With<RangeMovingTarget>>,
    q_gates: Query<(), With<RangeGateMarker>>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    // Say WHICH piece is missing rather than returning silently: a range whose
    // turret or aim point has gone away looks identical to a quiet log, and
    // that ambiguity cost a diagnosis once already.
    let Some(aim) = q_turret.iter().next().and_then(|a| **a) else {
        info!(
            "turret: no aim point ({} turrets, {} gates up)",
            q_turret.iter().count(),
            q_gates.iter().count()
        );
        return;
    };
    let Ok(muzzle) = q_muzzle.single() else {
        info!(
            "turret: no single muzzle ({} found)",
            q_muzzle.iter().count()
        );
        return;
    };
    let barrel = muzzle.forward();
    let to_aim = (aim - muzzle.translation()).normalize_or_zero();
    let error_deg = barrel.dot(to_aim).clamp(-1.0, 1.0).acos().to_degrees();
    info!(
        "turret: aim error {:.1} deg, {} bullets in flight, mover {}",
        error_deg,
        q_bullets.iter().count(),
        if q_moving.iter().next().is_some() {
            "up"
        } else {
            "DOWN"
        }
    );
}

/// What the headless range run has observed in the CURRENT round. Reset by the
/// reload beat, so round 2 has to earn every flag again rather than inheriting
/// round 1's.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct RangeOutcome {
    fired: bool,
    /// A turret round damaged a range target - a gate or the backstop. Not
    /// "a gate", because the range's own gravity demo sends most rounds into the
    /// planetoid; see [`RangeBackstopMarker`].
    target_hit: bool,
    /// The moving gate's position when the round's fire beat opened, so the
    /// aim assertion can show the turret converged on a target that MOVED.
    gate_at_round_start: Option<Vec3>,
}

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
/// so the range aimed perfectly and never fired a round.
#[cfg(feature = "debug")]
fn hold_inputs(world: &mut World, _elapsed: f32) {
    let held = world.resource::<HeldInput>();
    let (combat, fire) = (held.combat, held.fire);
    if combat {
        drive_action(world, "combat_stance", InputPhase::Press);
    }
    if fire {
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
    }
}

/// How close to the aim point the barrel must come, in degrees, for the turret
/// to count as tracking. Healthy lavapipe runs settle below 8 degrees while a
/// turret that cannot follow the sweep settles near 20 degrees or worse.
#[cfg(feature = "debug")]
const AIM_TOLERANCE_DEG: f32 = 8.0;

/// How far the moving gate must have travelled during a round before "the
/// barrel is on target" means anything. Below this the turret could be holding
/// a nearly static point.
#[cfg(feature = "debug")]
const GATE_TRAVEL_MIN: f32 = 8.0;

/// How much margin the tracking beat's travel clause carries over the assert's,
/// in units.
///
/// The travel is `|x(t) - x(t0)|` on a SINE sweep, so it is not monotonic: it
/// returns to 0 once per ~10.5s period and passes through
/// [`GATE_TRAVEL_MIN`] downward on the way. The autopilot enters the next step
/// on the FOLLOWING frame (`nova_autopilot::autopilot`: "entry gets its own
/// frame"), so a beat that opened on a FALLING edge at exactly the assert's
/// threshold would hand the assert a lower value and panic a healthy turret.
/// Half a second of the sweep's PEAK speed ([`SWEEP_AMPLITUDE`] *
/// [`SWEEP_RATE`] = 13.2 u/s) is orders of magnitude more than a frame at any
/// framerate this runs at.
#[cfg(feature = "debug")]
const GATE_TRAVEL_BEAT_MARGIN: f32 = SWEEP_AMPLITUDE * SWEEP_RATE * 0.5;

/// How long the trigger beat holds after the first round leaves the barrel, in
/// seconds.
///
/// A settle beat, and deliberately NOT "a gate took damage" - that is the
/// comparison [`assert_fired_and_connected`] makes, and gating on it left
/// invariants 1 and 2 unfailable: a turret that fires into empty space could
/// only surface as a deadline stall on this beat's name, never as the message
/// that explains it. The beat delivers the stimulus (rounds are leaving the
/// barrel); whether one CONNECTED is the assertion's question.
///
/// Sized off the range: the joints slew at ~140 deg/s and the nearest gates sit
/// 48-65 u out at a 100 u/s muzzle speed, so a healthy turret is on target and
/// has landed several rounds inside ~1.7s. This window is more than twice that.
#[cfg(feature = "debug")]
const HIT_SETTLE_SECS: f32 = 4.0;

/// How fast the aim error may still be moving and count as steady, in degrees
/// per SECOND. A rate, not a per-frame delta: the joints slew via
/// `SmoothLookRotation`, whose `speed` is rad/s, so a per-frame delta means a
/// different physical threshold at every framerate. At the ~140 deg/s these
/// joints slew at, a fixed 0.5 deg/frame bound is refused correctly on the
/// ~14fps llvmpipe box and satisfied MID-SLEW above ~280fps, letting invariant
/// 3 read an error tens of degrees out.
///
/// An order of magnitude under the slew rate, and above the wobble a barrel
/// holding a swept target shows.
#[cfg(feature = "debug")]
const AIM_STEADY_RATE_DEG_PER_SEC: f32 = 10.0;

/// How many consecutive steady samples the tracking beat waits for before
/// letting invariant 3 read the error.
#[cfg(feature = "debug")]
const AIM_STEADY_FRAMES: u32 = 5;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

/// The scripted run: raise the stance, hold the trigger through a firing
/// window, assert the hits, let the barrel converge on the mover, then do the
/// whole thing again on a reloaded range.
///
/// Not the stock `nova_autopilot()` preset: every beat waits on the value it
/// depends on - `WeaponsHot`, the first round leaving the barrel, the live aim
/// error - so a slow load or a slow slew delays the walk instead of truncating
/// it. No beat reads a quantity an assertion decides; the one settle states its
/// reason on [`HIT_SETTLE_SECS`]. The
/// per-step deadlines NAME the beat that stalled; their runtime sum
/// (15 + 36 + 10 + 36 = 97s, counting `fire_round`'s beats once per CALL rather
/// than once per source line) stays under `DEFAULT_DEADLINE_SECS` (120s) so a
/// named stall wins the race against the generic collector deadline.
#[cfg(feature = "debug")]
fn turret_script() -> Script {
    let script = Script::new()
        // Every step re-presses whatever the script is holding down; see
        // `hold_inputs` for why this must be the plugin hook and not a system.
        .input(hold_inputs)
        .step("load the range")
        .enter(GameStates::Loading)
        .until(and(player_ship_present(), moving_gate_present()))
        .deadline(15.0)
        .add();
    let script = fire_round(script, "round 1");
    let script = script
        // Wait for a range that is DEMONSTRABLY the fresh one: the reload
        // carries a DISTINCT scenario id, so `CurrentScenario` naming it can
        // only mean the second load landed. Paired with the ship being up,
        // since that is what round 2 goes on to fire.
        //
        // Not "wait for the ship to be gone, then back": there is no such
        // frame. `on_load_scenario` queues the scoped despawns and fires
        // `OnStartEvent` on the SAME `Commands`
        // (nova_scenario/src/loader/lifecycle.rs:162-253), so the teardown and
        // the respawn land in one flush and a gone-then-present pair stalls out
        // its deadline instead.
        //
        // Not "every gate is back at full health" either: the gates collide
        // with each other and with the planetoid, so a pristine range is not
        // a state this scene holds for long.
        .step("reload the range")
        .on_enter(reload_range)
        .until(and(
            scenario_is(RELOADED_RANGE_ID),
            and(player_ship_present(), moving_gate_present()),
        ))
        .deadline(10.0)
        .add();
    fire_round(script, RELOADED_ROUND)
}

/// One full pass of invariants 1-3, appended to `script`. Called twice - once
/// on the booted range and once on the reloaded one - which is what makes
/// invariant 4 a claim about the whole set rather than a spot check.
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
        .on_enter(open_fire)
        .until(and(range_fired(), elapsed(HIT_SETTLE_SECS)))
        .deadline(15.0)
        .add()
        .step("assert the range fired and connected")
        .on_enter(move |world: &mut World| assert_fired_and_connected(world, round))
        .add()
        // Keep the trigger down: the barrel converging is about the SLEW
        // keeping up with the sweep, and the rounds in flight are what make
        // the gizmo readable while it does.
        .step("track the sweeping gate")
        .until(aim_converged())
        .deadline(15.0)
        .add()
        .step("assert the barrel tracks the mover")
        .on_enter(move |world: &mut World| assert_aim_tracks_mover(world, round))
        .add()
}

/// Open the trigger and record where the moving gate was, so the aim
/// assertion can prove the target travelled while the turret held it.
#[cfg(feature = "debug")]
fn open_fire(world: &mut World) {
    // The round-opening beat waited for this gate, so its absence here is a
    // broken script rather than a slow load. Fail on the spot: recorded as
    // `None` it produced a silent 30s stall on the tracking beat instead.
    let gate = moving_gate_position(world)
        .expect("range: the sweeping gate must exist when a round's trigger opens");
    let mut outcome = world.resource_mut::<RangeOutcome>();
    outcome.gate_at_round_start = Some(gate);
    world.resource_mut::<HeldInput>().fire = true;
}

/// Re-trigger the range load, and clear the per-round state the fresh range
/// has to re-establish: the observed outcomes and the held trigger. The
/// stance stays raised - the fresh ship's safety re-derives it from the held
/// input, and `raise the weapons` waits for that.
#[cfg(feature = "debug")]
fn reload_range(world: &mut World) {
    let config = {
        let game_assets = world.resource::<GameAssets>();
        let sections = world.resource::<GameSections>();
        turret_range(game_assets, sections, RELOADED_RANGE_ID)
    };
    *world.resource_mut::<RangeOutcome>() = RangeOutcome::default();
    // RELEASE the trigger, do not merely stop re-pressing it. `ButtonInput`
    // only yields a fresh `just_pressed` on a not-pressed -> pressed edge, and
    // nothing else releases a synthesized key: leaving it latched from round 1
    // made round 2's press a no-op, so the reloaded range aimed perfectly and
    // never fired a round.
    world.resource_mut::<HeldInput>().fire = false;
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::Space);
    info!("range: reloading the range");
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

#[cfg(feature = "debug")]
fn range_fired() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<RangeOutcome>(|outcome| outcome.fired)
}

/// The sweeping gate exists.
///
/// Gated on by BOTH round-opening beats, not just the ship: the round's
/// tracking invariant is measured against this gate's travel, and the ship
/// spawning does not imply the scenario's objects have. A round that opened one
/// frame early captured no start position and then stalled the tracking beat for
/// its whole deadline - intermittently, since it depended on the spawn order
/// landing inside a single frame.
#[cfg(feature = "debug")]
fn moving_gate_present() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| moving_gate_position(world).is_some())
}

/// The moving gate's current position, or `None` while the range is down.
#[cfg(feature = "debug")]
fn moving_gate_position(world: &World) -> Option<Vec3> {
    let mut query = world.try_query_filtered::<&GlobalTransform, With<RangeMovingTarget>>()?;
    query.iter(world).next().map(|gt| gt.translation())
}

/// The angle between the barrel and the point the turret is aiming for, in
/// degrees - the same quantity `report_status` prints, read for the assertion
/// rather than the log.
#[cfg(feature = "debug")]
fn aim_error_deg(world: &World) -> Option<f32> {
    let aim = {
        let mut query =
            world.try_query_filtered::<&TurretSectionAimPoint, With<TurretSectionMarker>>()?;
        (**query.iter(world).next()?)?
    };
    let muzzle = {
        let mut query = world
            .try_query_filtered::<&GlobalTransform, With<TurretSectionBarrelMuzzleMarker>>()?;
        *query.iter(world).next()?
    };
    let barrel = muzzle.forward();
    let to_aim = (aim - muzzle.translation()).normalize_or_zero();
    Some(barrel.dot(to_aim).clamp(-1.0, 1.0).acos().to_degrees())
}

/// The tracking beat: the gate has travelled far enough for holding it to mean
/// anything, and the slew has reached STEADY STATE - the aim error stopped
/// changing, whatever value it stopped at.
///
/// Deliberately NOT `error < AIM_TOLERANCE_DEG`, which is what invariant 3 then
/// asserts. Gating on the assert's own bound made the assert unfailable: a
/// turret that settles 20 deg off the mover could only ever surface as a
/// deadline stall on this beat's name, never as the invariant message that
/// explains it. Steady state is the precondition; WHERE it settled is the
/// assertion's question.
///
/// The travel clause IS shared with the assert's guard, so it carries
/// [`GATE_TRAVEL_BEAT_MARGIN`] over it - see that constant for why a bare
/// threshold flakes on the sweep's falling edge.
#[cfg(feature = "debug")]
fn aim_converged() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        gate_travel(world).is_some_and(|travel| travel > GATE_TRAVEL_MIN + GATE_TRAVEL_BEAT_MARGIN)
            && world
                .get_resource::<AimSettle>()
                .is_some_and(|settle| settle.still_frames >= AIM_STEADY_FRAMES)
    })
}

/// How far the moving gate has travelled since the round's trigger opened.
#[cfg(feature = "debug")]
fn gate_travel(world: &World) -> Option<f32> {
    let start = world.get_resource::<RangeOutcome>()?.gate_at_round_start?;
    Some(moving_gate_position(world)?.distance(start))
}

/// Invariants 1 and 2: rounds left the barrel, and one of them connected with a
/// range target.
///
/// Invariant 2 is "a target took hits", not "a GATE took hits": the well bends
/// the rounds down, so a barrel converged to 0.5 deg still lands most of them on
/// the planetoid, and 11s of continuous fire over a full sweep period reaches a
/// gate only about half the time. What the invariant is for (task
/// 20260712-211352 - reach-Playing alone let a turret that never connects pass)
/// is answered either way: rounds are hitting the range rather than flying
/// through it.
#[cfg(feature = "debug")]
fn assert_fired_and_connected(world: &mut World, round: &str) {
    let outcome = world.resource::<RangeOutcome>();
    assert!(
        outcome.fired,
        "range ({round}): no turret round fired in the window"
    );
    assert!(
        outcome.target_hit,
        "range ({round}): no turret round connected with a range target in the window"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    info!("range: {round} - fired -> target hit, all observed");
    nova_probe::probe_marker(
        world,
        "outcome: turret fired",
        serde_json::json!({ "t": elapsed, "round": round }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: range target hit",
        serde_json::json!({ "t": elapsed, "round": round }),
    );
}

/// Invariant 3: the barrel converged on the aim point while the gate was
/// MOVING, so the slew keeps up with a mover rather than settling on a static
/// bearing. On round 2 this is also invariant 4 - the whole set held again on
/// a reloaded range - since reaching it means every earlier beat resolved.
#[cfg(feature = "debug")]
fn assert_aim_tracks_mover(world: &mut World, round: &str) {
    let travel = gate_travel(world).expect("range: the moving gate must exist at assert time");
    assert!(
        travel > GATE_TRAVEL_MIN,
        "range ({round}): the gate only travelled {travel:.1} u this round \
         (need more than {GATE_TRAVEL_MIN}); tracking it proves nothing"
    );
    let error = aim_error_deg(world).expect("range: the turret must exist at assert time");
    assert!(
        error < AIM_TOLERANCE_DEG,
        "range ({round}): the barrel is {error:.1} deg off the aim point \
         (tolerance {AIM_TOLERANCE_DEG}) after the gate travelled {travel:.1} u - \
         the turret is not tracking the mover"
    );
    info!(
        "range: {round} - the barrel tracks the mover ({error:.1} deg off, \
         gate travelled {travel:.1} u)"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: turret tracks the mover",
        serde_json::json!({ "t": elapsed, "round": round, "aim_error_deg": error, "gate_travel": travel }),
    );
    if round == RELOADED_ROUND {
        nova_probe::probe_marker(
            world,
            "outcome: turret invariants hold after reload",
            serde_json::json!({ "t": elapsed }),
        );
    }
}

// --- Live tuning sliders -----------------------------------------------------
//
// A small panel of sliders bound to the turret's tuning knobs, so they can be
// adjusted while watching the aim-error readout (task 20260707-150002). Each
// slider writes the live `TurretSectionConfigHelper`; the turret section keeps
// its child rotators/fire-timer in sync (`apply_turret_config_to_children`),
// and `muzzle_speed` is read live by the aim/shoot systems. Under autopilot
// there is no pointer to drag the sliders, so they stay inert. The slider widget
// itself lives in the sibling `slider` module.

/// One tunable turret knob, and the mapping between its config field and the
/// slider's display units (degrees for angles, so the panel reads naturally).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Knob {
    YawSpeed,
    PitchSpeed,
    MinPitch,
    MaxPitch,
    FireRate,
    MuzzleSpeed,
}

impl Knob {
    const ALL: [Knob; 6] = [
        Knob::YawSpeed,
        Knob::PitchSpeed,
        Knob::MinPitch,
        Knob::MaxPitch,
        Knob::FireRate,
        Knob::MuzzleSpeed,
    ];

    fn label(self) -> &'static str {
        match self {
            Knob::YawSpeed => "yaw speed",
            Knob::PitchSpeed => "pitch speed",
            Knob::MinPitch => "min pitch",
            Knob::MaxPitch => "max pitch",
            Knob::FireRate => "fire rate",
            Knob::MuzzleSpeed => "muzzle speed",
        }
    }

    fn unit(self) -> &'static str {
        match self {
            Knob::YawSpeed | Knob::PitchSpeed => "deg/s",
            Knob::MinPitch | Knob::MaxPitch => "deg",
            Knob::FireRate => "rps",
            Knob::MuzzleSpeed => "u/s",
        }
    }

    fn range(self) -> (f32, f32) {
        match self {
            Knob::YawSpeed | Knob::PitchSpeed => (0.0, 720.0),
            Knob::MinPitch => (-90.0, 45.0),
            Knob::MaxPitch => (0.0, 90.0),
            Knob::FireRate => (1.0, 200.0),
            Knob::MuzzleSpeed => (20.0, 300.0),
        }
    }

    /// Read the knob from a config, in slider (display) units.
    fn read(self, c: &TurretSectionConfig) -> f32 {
        match self {
            Knob::YawSpeed => find_axis(&c.root, Vec3::Y)
                .map(|j| j.speed.to_degrees())
                .unwrap_or(0.0),
            Knob::PitchSpeed => find_axis(&c.root, Vec3::X)
                .map(|j| j.speed.to_degrees())
                .unwrap_or(0.0),
            Knob::MinPitch => find_axis(&c.root, Vec3::X)
                .and_then(|j| j.min)
                .unwrap_or(0.0)
                .to_degrees(),
            Knob::MaxPitch => find_axis(&c.root, Vec3::X)
                .and_then(|j| j.max)
                .unwrap_or(0.0)
                .to_degrees(),
            Knob::FireRate => find_muzzle(&c.root).map(|m| m.fire_rate).unwrap_or(0.0),
            Knob::MuzzleSpeed => c.muzzle_speed,
        }
    }

    /// Write the knob into a config from a slider (display) value.
    fn write(self, c: &mut TurretSectionConfig, v: f32) {
        match self {
            Knob::YawSpeed => {
                if let Some(j) = find_axis_mut(&mut c.root, Vec3::Y) {
                    j.speed = v.to_radians();
                }
            }
            Knob::PitchSpeed => {
                if let Some(j) = find_axis_mut(&mut c.root, Vec3::X) {
                    j.speed = v.to_radians();
                }
            }
            Knob::MinPitch => {
                if let Some(j) = find_axis_mut(&mut c.root, Vec3::X) {
                    j.min = Some(v.to_radians());
                }
            }
            Knob::MaxPitch => {
                if let Some(j) = find_axis_mut(&mut c.root, Vec3::X) {
                    j.max = Some(v.to_radians());
                }
            }
            Knob::FireRate => {
                if let Some(m) = find_muzzle_mut(&mut c.root) {
                    m.fire_rate = v;
                }
            }
            Knob::MuzzleSpeed => c.muzzle_speed = v,
        }
    }
}

/// The first joint whose hinge axis is roughly `axis`, in DFS order.
fn find_axis(joint: &TurretJoint, axis: Vec3) -> Option<&TurretJoint> {
    if joint
        .axis
        .is_some_and(|a| a.normalize().dot(axis.normalize()) > 0.99)
    {
        return Some(joint);
    }
    joint.children.iter().find_map(|c| find_axis(c, axis))
}

fn find_axis_mut(joint: &mut TurretJoint, axis: Vec3) -> Option<&mut TurretJoint> {
    if joint
        .axis
        .is_some_and(|a| a.normalize().dot(axis.normalize()) > 0.99)
    {
        return Some(joint);
    }
    joint
        .children
        .iter_mut()
        .find_map(|c| find_axis_mut(c, axis))
}

/// The first joint carrying a muzzle, in DFS order.
fn find_muzzle(joint: &TurretJoint) -> Option<&MuzzleConfig> {
    if let Some(m) = &joint.muzzle {
        return Some(m);
    }
    joint.children.iter().find_map(find_muzzle)
}

fn find_muzzle_mut(joint: &mut TurretJoint) -> Option<&mut MuzzleConfig> {
    if joint.muzzle.is_some() {
        return joint.muzzle.as_mut();
    }
    joint.children.iter_mut().find_map(find_muzzle_mut)
}

/// Marks a knob's value-readout text.
#[derive(Component, Clone, Copy)]
struct KnobLabel(Knob);

/// The turret config the range spawns its ship with; the sliders start from these values.
fn range_turret_config(sections: &GameSections) -> TurretSectionConfig {
    let section = sections
        .get_section("pdc_kinetic_turret_section")
        .expect("section 'pdc_kinetic_turret_section' not found");
    match &section.kind {
        SectionKind::Turret(config) => config.clone(),
        _ => panic!("section 'pdc_kinetic_turret_section' is not a turret"),
    }
}

fn setup_tuning_ui(mut commands: Commands, sections: Res<GameSections>) {
    let config = range_turret_config(&sections);

    let root = commands
        .spawn((
            Name::new("Turret Tuning Panel"),
            Node {
                position_type: PositionType::Absolute,
                top: px(8),
                left: px(8),
                width: px(320),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                padding: UiRect::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        ))
        .id();

    for knob in Knob::ALL {
        let (min, max) = knob.range();
        let value = knob.read(&config);

        let row = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(2),
                    ..default()
                },
                ChildOf(root),
            ))
            .id();

        commands.spawn((
            KnobLabel(knob),
            Text::new(String::new()),
            TextFont {
                font_size: FontSize::Px(11.0),
                ..default()
            },
            ChildOf(row),
        ));

        commands.spawn((
            slider(min, max, value),
            ChildOf(row),
            observe(
                move |change: On<ValueChange<f32>>,
                      mut q_turret: Query<
                    &mut TurretSectionConfigHelper,
                    With<TurretSectionMarker>,
                >| {
                    for mut helper in &mut q_turret {
                        knob.write(&mut helper, change.value);
                    }
                },
            ),
        ));
    }
}

/// Keep each knob's readout text in sync with the live turret config, so the
/// panel reflects both slider edits and the config's starting values.
fn update_knob_labels(
    q_turret: Query<&TurretSectionConfigHelper, With<TurretSectionMarker>>,
    mut q_labels: Query<(&KnobLabel, &mut Text)>,
) {
    let Some(config) = q_turret.iter().next() else {
        return;
    };
    for (label, mut text) in &mut q_labels {
        let knob = label.0;
        text.0 = format!("{}: {:.0} {}", knob.label(), knob.read(config), knob.unit());
    }
}
