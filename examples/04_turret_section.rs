//! 04_turret_section: a focused test range for the PDC turret section.
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
//! BCS_AUTOPILOT=1 cargo run --example 04_turret_section --features debug
//! # look for: `nova harness: reached Playing`, `turret: aim error ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[path = "04_turret_section/slider.rs"]
mod slider;

use avian3d::prelude::*;
use bevy::{
    color::palettes::tailwind,
    platform::collections::HashMap,
    prelude::*,
    ui_widgets::{observe, ValueChange},
};
use clap::Parser;
use nova_protocol::prelude::*;
use slider::{slider, SliderWidgetPlugin};

#[derive(Parser)]
#[command(name = "04_turret_section")]
#[command(version = "1.0.0")]
#[command(about = "A test range for the PDC turret section in nova_protocol", long_about = None)]
struct Cli;

/// Id of the gate that sweeps across the front for the turret to track.
const MOVING_GATE_ID: &str = "gate_moving";

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // Headless smoke-test harness: inert in a normal run. Under BCS_AUTOPILOT
    // it holds the fire key and asserts the range's PURPOSE before the window
    // closes: rounds left the barrel and a gate actually took hits (task
    // 20260712-211352 - reach-Playing alone let a turret that never connects
    // pass). Scene is built on `GameAssetsStates::Loaded` so the screenshot's
    // forced Playing does not re-run setup.
    #[cfg(feature = "debug")]
    {
        app.init_resource::<RangeOutcome>();
        app.add_observer(
            |_: On<Add, TurretBulletProjectileMarker>, mut outcome: ResMut<RangeOutcome>| {
                outcome.fired = true;
            },
        );
        app.add_observer(
            |damage: On<HealthApplyDamage>,
             q_gate: Query<(), With<RangeGateMarker>>,
             mut outcome: ResMut<RangeOutcome>| {
                if q_gate.contains(damage.entity) {
                    outcome.gate_damaged = true;
                }
            },
        );
        app.add_plugins(nova_autopilot().input(autopilot_fire_and_assert));
        app.add_plugins(nova_screenshot());
    }

    app.run();
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
}

/// Marks an asteroid the range treats as a target gate.
#[derive(Component)]
struct RangeGateMarker;

/// Marks the gate that sweeps across the front (the turret's tracking target).
#[derive(Component)]
struct RangeMovingTarget;

/// Throttle for the status readout.
#[derive(Resource)]
struct StatusTimer(Timer);

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(turret_range(&game_assets, &sections)));
}

fn turret_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };

    let ship = SpaceshipConfig {
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::from([(
                "turret".to_string(),
                vec![KeyCode::Space.into(), GamepadButton::RightTrigger.into()],
            )]),
            speed_cap: None,
            // Dev/tuning harness: fire freely.
            infinite_ammo: true,
        }),
        sections: vec![
            SpaceshipSectionConfig {
                id: "controller".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                config: section("basic_controller_section"),
            },
            SpaceshipSectionConfig {
                id: "hull".to_string(),
                position: Vec3::new(0.0, 0.0, 1.0),
                rotation: Quat::IDENTITY,
                config: section("reinforced_hull_section"),
            },
            SpaceshipSectionConfig {
                id: "turret".to_string(),
                position: Vec3::new(0.0, 0.0, -1.0),
                // Matches the turret placement in the asteroid_field ship so the
                // base sits upright.
                rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                config: section("better_turret_section"),
            },
        ],
    };

    // Gates in the turret's firing arc (ahead, -Z, within its pitch range), plus
    // the sweeping one. Health is high so they survive and the turret keeps
    // tracking/firing rather than clearing the range in a burst.
    let gate = |id: &str, name: &str, pos: Vec3| ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position: pos,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: 2.0,
            texture: game_assets.asteroid_texture.clone().into(),
            health: 2000.0,
            surface_gravity: None,
            invulnerable: false,
            lock_signature: None,
        }),
    };

    let objects = vec![
        ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "player_ship".to_string(),
                name: "Turret Ship".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(ship),
        },
        gate("gate_front", "Front Gate", Vec3::new(0.0, 3.0, -55.0)),
        gate("gate_left", "Left Gate", Vec3::new(-32.0, 4.0, -45.0)),
        gate("gate_right", "Right Gate", Vec3::new(30.0, 3.0, -48.0)),
        gate("gate_high", "High Gate", Vec3::new(6.0, 26.0, -40.0)),
        gate(MOVING_GATE_ID, "Moving Gate", Vec3::new(-35.0, 6.0, -55.0)),
        // A gravity planetoid slung below the firing lane so rounds crossing
        // its sphere of influence curve downward toward it - the range is where
        // you eyeball bullet gravity (docs/spikes/20260712-112113). It is
        // authored strong (surface_gravity 10) and big (radius 16 -> SOI 128u
        // at the default soi_factor), which is why the shooter's own gravity is
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
                radius: 16.0,
                texture: game_assets.asteroid_texture.clone().into(),
                health: 100_000.0,
                surface_gravity: Some(10.0),
                invulnerable: false,
                lock_signature: None,
            }),
        },
    ];

    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions: objects
            .into_iter()
            .map(EventActionConfig::SpawnScenarioObject)
            .collect(),
    }];

    ScenarioConfig {
        id: "turret_range".to_string(),
        name: "Turret Range".to_string(),
        description: "A test range for the PDC turret section.".to_string(),
        cubemap: game_assets.cubemap.clone().into(),
        events,
    }
}

/// Tag each spawned asteroid as a gate, and single out the sweeping one.
fn tag_gate(add: On<Add, AsteroidMarker>, mut commands: Commands, q_id: Query<&EntityId>) {
    let entity = add.entity;
    commands.entity(entity).insert(RangeGateMarker);

    if q_id
        .get(entity)
        .map(|id| id.0 == MOVING_GATE_ID)
        .unwrap_or(false)
    {
        commands
            .entity(entity)
            .insert((RangeMovingTarget, LinearVelocity::default()));
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
fn drive_moving_gate(
    time: Res<Time>,
    mut q_gate: Query<&mut LinearVelocity, With<RangeMovingTarget>>,
) {
    let speed = (time.elapsed_secs() * 0.6).sin() * 18.0;
    for mut velocity in &mut q_gate {
        velocity.0 = Vec3::new(speed, 0.0, 0.0);
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
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    let Some(aim) = q_turret.iter().next().and_then(|a| **a) else {
        return;
    };
    let Ok(muzzle) = q_muzzle.single() else {
        return;
    };
    let barrel = muzzle.forward();
    let to_aim = (aim - muzzle.translation()).normalize_or_zero();
    let error_deg = barrel.dot(to_aim).clamp(-1.0, 1.0).acos().to_degrees();
    info!(
        "turret: aim error {:.1} deg, {} bullets in flight",
        error_deg,
        q_bullets.iter().count()
    );
}

/// What the headless range run has observed so far; asserted complete just
/// before the autopilot window closes.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct RangeOutcome {
    fired: bool,
    gate_damaged: bool,
    asserted: bool,
}

/// Autopilot input: hold the fire key while in Playing so the range fires
/// headless, and just before the window closes assert rounds were fired and
/// a gate took hits (the nearest gate sits ~55 u out; the PDC stream reaches
/// it in about a second, so the chain completes well inside the window).
#[cfg(feature = "debug")]
fn autopilot_fire_and_assert(world: &mut World, elapsed: f32) {
    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }
    // Raise the combat stance (RMB held) and only then press fire: the
    // weapons safety (task 20260713-082337) derives WeaponsHot from the
    // HELD combat input, DENIES a press that arrives while cold, and a
    // held key produces no fresh Start edge once hot - so pressing fire
    // before the stance settles latches nothing, forever. The outcome
    // assertion below caught exactly that: both ranges fired nothing
    // since the safety landed (task 20260712-211352). Mirror a real
    // player: raise, wait until hot, then hold fire.
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Right);
    let hot = {
        let mut q_hot = world.query_filtered::<&WeaponsHot, With<PlayerSpaceshipMarker>>();
        q_hot.single(world).is_ok_and(|hot| hot.0)
    };
    if hot {
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
    }

    if elapsed > nova_protocol::nova_debug::harness::NOVA_AUTOPILOT_SECS - 0.5 {
        let mut outcome = world.resource_mut::<RangeOutcome>();
        if outcome.asserted {
            return;
        }
        outcome.asserted = true;
        assert!(outcome.fired, "range: no turret round fired in the window");
        assert!(
            outcome.gate_damaged,
            "range: no gate took turret hits in the window"
        );
        info!("range: fired -> gate damaged, all observed");
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
            Knob::YawSpeed => c.yaw_speed.to_degrees(),
            Knob::PitchSpeed => c.pitch_speed.to_degrees(),
            Knob::MinPitch => c.min_pitch.unwrap_or(0.0).to_degrees(),
            Knob::MaxPitch => c.max_pitch.unwrap_or(0.0).to_degrees(),
            Knob::FireRate => c.fire_rate,
            Knob::MuzzleSpeed => c.muzzle_speed,
        }
    }

    /// Write the knob into a config from a slider (display) value.
    fn write(self, c: &mut TurretSectionConfig, v: f32) {
        match self {
            Knob::YawSpeed => c.yaw_speed = v.to_radians(),
            Knob::PitchSpeed => c.pitch_speed = v.to_radians(),
            Knob::MinPitch => c.min_pitch = Some(v.to_radians()),
            Knob::MaxPitch => c.max_pitch = Some(v.to_radians()),
            Knob::FireRate => c.fire_rate = v,
            Knob::MuzzleSpeed => c.muzzle_speed = v,
        }
    }
}

/// Marks a knob's value-readout text.
#[derive(Component, Clone, Copy)]
struct KnobLabel(Knob);

/// The turret config the range spawns its ship with; the sliders start from these values.
fn range_turret_config(sections: &GameSections) -> TurretSectionConfig {
    let section = sections
        .get_section("better_turret_section")
        .expect("section 'better_turret_section' not found");
    match &section.kind {
        SectionKind::Turret(config) => config.clone(),
        _ => panic!("section 'better_turret_section' is not a turret"),
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
