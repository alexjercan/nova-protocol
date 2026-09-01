//! system_railgun_lance: the spinal lance - commit, charge, rake, recoil, reload.
//!
//! One player ship sits at the origin with a railgun lance on its spine, bore
//! down -Z. A stack of six free-floating plates stands downrange on that exact
//! line. The range taps the commit for ONE tick, releases it, and then only
//! watches: everything after the tap is the weapon's own, which is the point.
//!
//! FIVE named invariants:
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the commit outlives the trigger` | the tap starts a charge that a release cannot stop |
//! | 2 | `outcome: the charge bolt tracks the charge` | the cue walks the bore with the gameplay clock and resets on the shot |
//! | 3 | `outcome: one slug rakes every layer` | ONE shot deals its authored damage to all six plates |
//! | 4 | `outcome: recoil shoves the ship that fired` | the hull ends up moving BACK along its own bore |
//! | 5 | `outcome: the lance holds one shell` | the trigger held down through the reload raises no second charge |
//!
//! Invariant 3 is why the plates are 500 hp: a layer that DIES stops being a
//! reading. Alive, each one's remaining health is the arithmetic - one slug's
//! authored damage, subtracted once, six times over.
//!
//! Invariant 4 reads a VELOCITY, not a position. The recoil is one impulse at
//! the muzzle and the ship is otherwise free and unpowered, so the sign of its
//! velocity along the bore is the whole claim and no distance threshold has to
//! be invented for it.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//!
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_railgun_lance --features debug
//! # look for: `railgun: every lance invariant held`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use std::collections::BTreeMap;

use avian3d::prelude::*;
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_railgun_lance")]
#[command(version = "1.0.0")]
#[command(about = "A test range for the spinal railgun lance in nova_protocol. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario id the range loads under.
const RANGE_ID: &str = "railgun_range";

/// The lance's scenario-object id, so the probe and the rig cannot drift apart.
const LANCE_ID: &str = "lance";

/// How many plates stand in the bore's line.
const PLATES: usize = 6;

/// Plate health. Well above the lance's authored per-layer damage, so every
/// layer SURVIVES its rake and its remaining health is still readable.
const PLATE_HEALTH: f32 = 500.0;

/// Plate thickness and spacing along the line of flight, in world units. Two
/// separate facts, so a plate cannot be met twice by one sweep.
const PLATE_THICKNESS: f32 = 4.0;
const PLATE_PITCH: f32 = 6.0;

/// How far downrange the first plate stands. Comfortably past the muzzle and
/// well inside the lance's reach, so the slug is in free flight when it
/// arrives and the whole stack is crossed inside its lifetime.
const FIRST_PLATE_Z: f32 = -30.0;

/// Slack on a health reading, in hit points. The rake is exact arithmetic; this
/// only absorbs f32 accumulation.
const HEALTH_EPSILON: f32 = 0.05;

/// How long the range holds the trigger DOWN after the shot, in frames, before
/// reading invariant 5. Past the charge time at any plausible frame rate and
/// nowhere near the twelve-second reload, so a second charge would have both
/// started and finished inside the window.
const RELOAD_PROBE_FRAMES: u32 = 240;

/// How many `Playing` frames the whole walk gets before the range calls itself
/// stalled and PANICS with what it was still waiting on.
///
/// A range that hangs proves nothing and costs a CI slot the whole deadline;
/// this turns "it never verified" into a named failure that says which reading
/// was missing. Generous - the walk needs a charge, a flight and a reload
/// window, and every one of those is measured in frames rather than asserted.
const STALL_FRAMES: u32 = 3_600;

/// How often the walk says where it is, in frames.
const STATUS_EVERY: u32 = 120;

/// Marks one downrange plate, with the layer it stands at.
#[derive(Component, Clone, Copy, Debug)]
struct RangePlate(usize);

/// What the range has watched happen.
#[derive(Resource, Default)]
struct LanceProbe {
    /// The lance section, once the scenario has spawned it.
    lance: Option<Entity>,
    /// The ship the lance is bolted to.
    ship: Option<Entity>,
    /// Frames spent in `Playing`.
    frames: u32,
    /// Set once the plates are downrange.
    staged: bool,
    /// Set on the tick the commit is tapped.
    committed: bool,
    /// The charge state read one tick AFTER the trigger was released. `Some`
    /// means the read happened; `true` means the lance was still charging.
    charging_after_release: Option<bool>,
    /// The highest charge-cue progress seen while the lance was charging.
    peak_cue: f32,
    /// The charge-cue progress read after the shot left.
    cue_after_shot: Option<f32>,
    /// How many slugs this lance has fired.
    shots: u32,
    /// The frame the first shot left on.
    shot_frame: Option<u32>,
    /// The ship's velocity along its own bore after the shot, in units/s.
    /// Negative means it was pushed BACK.
    bore_velocity_after_shot: Option<f32>,
    /// Set once every invariant has been read and reported.
    verified: bool,
    exit_delay: u32,
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();
    app.add_plugins(nova_probe::NovaProbePlugin::default());
    app.run()
}

fn range_plugin(app: &mut App) {
    app.init_resource::<LanceProbe>();
    app.add_observer(count_shots);
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_range);
    // The safety is not something a range gets to write around: `WeaponsHot`
    // is DERIVED every frame from the held combat stance, so a range that
    // pokes the flag has it stomped back before the lance ever reads it - the
    // first run of this range charged for two ticks and dumped it. Hold the
    // real button instead, in `PreUpdate` after the frame's input has been
    // collected, so the whole safety chain runs the way it does in flight.
    app.add_systems(
        PreUpdate,
        hold_combat_stance
            .after(bevy::input::InputSystems)
            .run_if(in_state(GameStates::Playing)),
    );

    // Sampled every frame, not only on the beats: the charge cue is a moving
    // value and invariant 2 is about the WALK, which no single frame carries.
    app.add_systems(
        Update,
        (sample_charge, drive_range)
            .chain()
            .run_if(in_state(GameStates::Playing)),
    );
}

/// Hold the combat stance for the frame, which is what raises the weapons.
fn hold_combat_stance(mut mouse: ResMut<ButtonInput<MouseButton>>) {
    mouse.press(MouseButton::Right);
}

fn load_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(lance_rig(&game_assets, &sections)));
}

/// The rig scenario: one PLAYER ship, bore down -Z, nothing in front of it.
///
/// Player-controlled with an EMPTY input mapping. The range writes
/// [`RailgunSectionInput`] straight onto the section instead of synthesizing a
/// button, so what is under test is the weapon rather than the binding - and
/// nothing else the ship carries can move it while the recoil is being read.
///
/// Spine layout, in cells: the lance is THREE cells long and centred on its
/// own origin, so at -1 it fills -2, -1 and 0 and the controller behind it
/// starts at +1. Cell -3 is empty, which the exit-clearance rule requires: a
/// lance cannot traverse off its bore, so anything standing there is a shot
/// taken through the ship's own hull.
fn lance_rig(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let at = |id: &str, kind: &str, position: Vec3| SpaceshipSectionConfig {
        id: id.to_string(),
        position,
        rotation: Quat::IDENTITY,
        source: SectionSource::Inline(section(kind)),
        modifications: vec![],
    };

    let ship = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::new(),
            speed_cap: None,
            // NOT infinite: invariant 5 is about the magazine, and an
            // unlimited one would make it unfalsifiable.
            infinite_ammo: false,
        }),
        hull: ShipSource::Inline(ShipHull {
            sections: vec![
                at(
                    LANCE_ID,
                    RAILGUN_LANCE_SECTION_ID,
                    Vec3::new(0.0, 0.0, -1.0),
                ),
                at(
                    "controller",
                    BASIC_CONTROLLER_SECTION_ID,
                    Vec3::new(0.0, 0.0, 1.0),
                ),
                at(
                    "thruster",
                    BASIC_THRUSTER_SECTION_ID,
                    Vec3::new(0.0, 0.0, 2.0),
                ),
            ],
            ..default()
        }),
        ..default()
    };

    ScenarioConfig {
        description: "A ship with a spinal lance, and a stack of plates to rake.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            // The rig lights itself: the engine spawns no light, so a
            // scenario that authors none renders black.
            actions: [
                vec![EventActionConfig::SpawnScenarioObject(
                    ScenarioObjectConfig {
                        base: BaseScenarioObjectConfig {
                            id: "player_ship".to_string(),
                            name: "Lance Rig".to_string(),
                            position: Vec3::ZERO,
                            rotation: Quat::IDENTITY,
                        },
                        kind: ScenarioObjectKind::Spaceship(ship),
                    },
                )],
                ThreePointRig::around("rig", Vec3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            RANGE_ID.to_string(),
            "Railgun Lance Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Stand the plates downrange.
///
/// Free-floating bodies with no `ConnectedTo`, deliberately: a plate that
/// reached zero health would be DESTROYED, and destruction pulls in render
/// observers this range has nothing to say about. At [`PLATE_HEALTH`] none of
/// them gets close, so each stays a plain readable health pool.
fn stage_plates(world: &mut World) {
    for layer in 0..PLATES {
        let z = FIRST_PLATE_Z - layer as f32 * PLATE_PITCH;
        let body = world
            .spawn((
                Name::new(format!("plate {layer}")),
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::Z * z),
            ))
            .id();
        world.spawn((
            ChildOf(body),
            SectionMarker,
            RangePlate(layer),
            Transform::default(),
            Collider::cuboid(8.0, 8.0, PLATE_THICKNESS),
            ColliderDensity(1.0),
            Health::new(PLATE_HEALTH),
        ));
    }
}

/// Count the shots this lance actually fired, off the weapon's own report.
fn count_shots(fired: On<RailgunFired>, mut probe: ResMut<LanceProbe>) {
    if probe.lance == Some(fired.entity) {
        probe.shots += 1;
    }
}

/// Follow the charge cue every frame: its peak while the bolt is climbing, and
/// where it lands once the shot is away.
fn sample_charge(q_animations: Query<&SectionAnimations>, mut probe: ResMut<LanceProbe>) {
    let Some(lance) = probe.lance else { return };
    let Ok(animations) = q_animations.get(lance) else {
        return;
    };
    let Some(progress) = animations.cue_progress(SectionAnimationCue::Charge) else {
        return;
    };
    if probe.shots == 0 {
        probe.peak_cue = probe.peak_cue.max(progress);
    } else if probe.cue_after_shot.is_none() {
        probe.cue_after_shot = Some(progress);
    }
}

/// The walk: find the lance, stage the plates, tap the commit, release it, and
/// then read what the weapon did on its own.
fn drive_range(world: &mut World) {
    world.resource_mut::<LanceProbe>().frames += 1;

    if !world.resource::<LanceProbe>().verified {
        let frames = world.resource::<LanceProbe>().frames;
        if frames % STATUS_EVERY == 0 {
            let plates = plate_health(world);
            let probe = world.resource::<LanceProbe>();
            info!(
                "railgun: frame {frames} - lance {:?}, staged {}, committed {}, charging after \
                 release {:?}, peak cue {:.2}, shots {}, plates {plates:?}",
                probe.lance,
                probe.staged,
                probe.committed,
                probe.charging_after_release,
                probe.peak_cue,
                probe.shots,
            );
        }
        assert!(
            frames <= STALL_FRAMES,
            "railgun: the walk stalled - it never reached a verdict inside {STALL_FRAMES} frames"
        );
    }

    if world.resource::<LanceProbe>().verified {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            let exit = {
                let mut probe = world.resource_mut::<LanceProbe>();
                probe.exit_delay += 1;
                probe.exit_delay >= 30
            };
            if exit {
                world.write_message(AppExit::Success);
            }
        }
        return;
    }

    if world.resource::<LanceProbe>().lance.is_none() {
        let found = world
            .try_query_filtered::<(Entity, &ChildOf), With<RailgunSectionMarker>>()
            .and_then(|mut query| query.iter(world).next().map(|(a, b)| (a, b.0)));
        let Some((lance, ship)) = found else { return };
        let mut probe = world.resource_mut::<LanceProbe>();
        probe.lance = Some(lance);
        probe.ship = Some(ship);
        return;
    }

    if !world.resource::<LanceProbe>().staged {
        stage_plates(world);
        world.resource_mut::<LanceProbe>().staged = true;
        return;
    }

    let (lance, ship) = {
        let probe = world.resource::<LanceProbe>();
        (probe.lance.unwrap(), probe.ship.unwrap())
    };

    // Nothing to tap until the stance has actually gone hot: a cold ship
    // refuses the commit, which is a rule this range is not testing.
    if !world.get::<WeaponsHot>(ship).is_some_and(|hot| hot.0) {
        return;
    }

    // The tap: trigger down, for exactly one frame.
    if !world.resource::<LanceProbe>().committed {
        if let Some(mut input) = world.get_mut::<RailgunSectionInput>(lance) {
            input.0 = true;
        }
        world.resource_mut::<LanceProbe>().committed = true;
        return;
    }

    // The release, and invariant 1's reading one frame behind it. A lance that
    // needed the trigger held would be back at `Ready` here.
    if world
        .resource::<LanceProbe>()
        .charging_after_release
        .is_none()
    {
        if let Some(mut input) = world.get_mut::<RailgunSectionInput>(lance) {
            if input.0 {
                input.0 = false;
                return;
            }
        }
        let charging = matches!(
            world.get::<RailgunCharge>(lance),
            Some(RailgunCharge::Charging { .. })
        );
        world.resource_mut::<LanceProbe>().charging_after_release = Some(charging);
        return;
    }

    // Nothing to read until the shot has actually left.
    if world.resource::<LanceProbe>().shots == 0 {
        return;
    }

    // The recoil reading, taken on the first frame after the shot: one impulse
    // at the muzzle on an unpowered ship, so its velocity along the bore is
    // the whole of it.
    if world.resource::<LanceProbe>().shot_frame.is_none() {
        let frame = world.resource::<LanceProbe>().frames;
        let bore = world
            .get::<GlobalTransform>(lance)
            .map(|pose| pose.rotation() * Vec3::NEG_Z)
            .unwrap_or(Vec3::NEG_Z);
        let velocity = world
            .get::<LinearVelocity>(ship)
            .map(|velocity| velocity.0)
            .unwrap_or_default();
        let mut probe = world.resource_mut::<LanceProbe>();
        probe.shot_frame = Some(frame);
        probe.bore_velocity_after_shot = Some(velocity.dot(bore));
        // Hold the trigger DOWN from here: invariant 5 wants a lance that
        // refuses a second commit while it is asked for one.
        drop(probe);
        if let Some(mut input) = world.get_mut::<RailgunSectionInput>(lance) {
            input.0 = true;
        }
        return;
    }

    // Every plate has to have been raked before the readings mean anything.
    let plates = plate_health(world);
    if plates.len() != PLATES || plates.iter().any(|health| *health >= PLATE_HEALTH) {
        return;
    }

    // And the reload window has to have run long enough that a second charge
    // would have completed inside it.
    let shot_frame = world.resource::<LanceProbe>().shot_frame.unwrap();
    if world.resource::<LanceProbe>().frames < shot_frame + RELOAD_PROBE_FRAMES {
        return;
    }

    verify(world, lance, &plates);
}

/// Every plate's remaining health, in layer order.
fn plate_health(world: &mut World) -> Vec<f32> {
    let Some(mut query) = world.try_query::<(&RangePlate, &Health)>() else {
        return Vec::new();
    };
    let mut found: Vec<(usize, f32)> = query
        .iter(world)
        .map(|(plate, health)| (plate.0, health.current))
        .collect();
    found.sort_by_key(|(layer, _)| *layer);
    found.into_iter().map(|(_, health)| health).collect()
}

fn verify(world: &mut World, lance: Entity, plates: &[f32]) {
    let probe_readings = {
        let probe = world.resource::<LanceProbe>();
        (
            probe.charging_after_release.unwrap_or(false),
            probe.peak_cue,
            probe.cue_after_shot.unwrap_or(1.0),
            probe.shots,
            probe.bore_velocity_after_shot.unwrap_or(0.0),
        )
    };
    let (charging_after_release, peak_cue, cue_after_shot, shots, bore_velocity) = probe_readings;

    assert!(
        charging_after_release,
        "railgun: releasing the trigger cancelled the commit - the lance was not charging one \
         frame after the release"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the commit outlives the trigger",
        serde_json::json!({ "charging_after_release": charging_after_release }),
    );

    assert!(
        peak_cue > 0.0 && peak_cue <= 1.0 && cue_after_shot <= f32::EPSILON,
        "railgun: the charge cue did not walk the bore and reset: peak {peak_cue}, after the shot \
         {cue_after_shot}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the charge bolt tracks the charge",
        serde_json::json!({ "peak": peak_cue, "after_shot": cue_after_shot }),
    );

    let authored = world
        .get::<RailgunSectionConfigHelper>(lance)
        .map(|config| config.slug_damage)
        .expect("the lance carries its authored config");
    let expected = PLATE_HEALTH - authored;
    assert!(
        shots == 1
            && plates
                .iter()
                .all(|health| (health - expected).abs() <= HEALTH_EPSILON),
        "railgun: one slug did not rake every layer for its authored damage: {shots} shot(s), \
         expected {expected} hp left on each of {PLATES}, got {plates:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: one slug rakes every layer",
        serde_json::json!({ "shots": shots, "layers": plates.len(), "left": plates }),
    );

    assert!(
        bore_velocity < 0.0,
        "railgun: the shot did not shove the ship back along its bore: {bore_velocity} u/s"
    );
    nova_probe::probe_marker(
        world,
        "outcome: recoil shoves the ship that fired",
        serde_json::json!({ "bore_velocity": bore_velocity }),
    );

    let rounds = world.get::<SectionAmmo>(lance).map(|ammo| ammo.rounds);
    assert!(
        shots == 1 && rounds == Some(0),
        "railgun: the lance took a second commit while reloading: {shots} shot(s), magazine \
         {rounds:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the lance holds one shell",
        serde_json::json!({ "shots": shots, "rounds": rounds }),
    );

    world.resource_mut::<LanceProbe>().verified = true;
    info!("railgun: every lance invariant held");
}
