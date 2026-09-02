//! system_railgun_lance: the spinal lance - commit, charge, rake, recoil, reload.
//!
//! One player ship sits at the origin with a railgun lance on its spine, bore
//! down -Z. A stack of six free-floating plates stands downrange on that exact
//! line. The range taps the commit for ONE tick, releases it, and then only
//! watches: everything after the tap is the weapon's own, which is the point.
//!
//! Off to one side stands the MEASUREMENT BANK: five blocks of unit reinforced
//! cells, each cut by one copy of this lance's authored round with nothing but
//! the rake width changed. It is what turns "the corridor got wider" into a
//! number, and what the base `rake_radius` was chosen from.
//!
//! NINE named invariants:
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the commit outlives the trigger` | the tap starts a charge that a release cannot stop |
//! | 2 | `outcome: the charge bolt tracks the charge` | the cue walks the bore with the gameplay clock and resets on the shot |
//! | 3 | `outcome: one slug rakes every layer` | ONE shot deals its authored damage to all six plates |
//! | 4 | `outcome: recoil shoves the ship that fired` | the hull ends up moving BACK along its own bore |
//! | 5 | `outcome: the lance holds one shell` | the trigger held down through the reload raises no second charge |
//! | 6 | `outcome: the authored rake rides the shot` | the shell that left carries the width the catalog authored |
//! | 7 | `outcome: the rake widens the corridor` | the rake opens every cell inside its radius and none outside it, marked where it met them |
//! | 8 | `outcome: the rake spends one budget` | what stops the rake is `slug_power`, not a cap and not the target |
//! | 9 | `outcome: a wide rake craters instead of boring` | the blast era's 4.0 spends the same budget on the entry face |
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

use std::collections::{BTreeMap, BTreeSet};

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
    /// The rake the slug the lance ACTUALLY fired carried, read one frame
    /// after the shot. `Some(None)` means the shot was read and carried none.
    fired_rake: Option<Option<f32>>,
    /// The authored round the measurement bank fires, read off the section.
    spec: Option<SlugSpec>,
    /// Set once the bank's slugs are away.
    stands_fired: bool,
    /// Every cell the bank watched a slug pay for, in the order they were paid.
    bites: Vec<StandBite>,
    /// Set once every invariant has been read and reported.
    verified: bool,
    exit_delay: u32,
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();
    // No frame-time capture: the range is a one-shot commit-charge-rake walk
    // that exits 30 frames after the last invariant verifies, so it can never
    // fill the 900-frame baseline window the capture arms. Armed, it reports
    // `fps_within_baseline FAIL - armed and silent` on every run and measures
    // nothing. Steady-state lance cost belongs in a stress range.
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.run()
}

fn range_plugin(app: &mut App) {
    app.init_resource::<LanceProbe>();
    app.add_observer(count_shots);
    app.add_observer(record_stand_bites);
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
        stage_stands(world);
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

    // The tap: trigger down until the gun has actually taken the commit.
    //
    // Held, not pulsed for one frame. The walk runs on the RENDER clock and the
    // gun on the FIXED one, so a tap released after a single frame can fall
    // entirely between two of the gun's ticks - and then the range sits out the
    // rest of its run waiting for a shot nothing ever asked for. Releasing on
    // the charge instead of on a frame count is the same tap either way, and it
    // is the release that invariant 1 is about.
    if !world.resource::<LanceProbe>().committed {
        if let Some(mut input) = world.get_mut::<RailgunSectionInput>(lance) {
            input.0 = true;
        }
        if matches!(
            world.get::<RailgunCharge>(lance),
            Some(RailgunCharge::Charging { .. })
        ) {
            world.resource_mut::<LanceProbe>().committed = true;
        }
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
        // Invariant 6 is read HERE, while the lance's own shell is the only
        // round in the air: the bank spawns rounds wearing the same marker one
        // statement later, and after that "the slug the lance fired" is no
        // longer a thing a query can name.
        let fired_rake = world
            .try_query_filtered::<Option<&RoundRake>, With<RailgunSlugProjectileMarker>>()
            .and_then(|mut query| {
                query
                    .iter(world)
                    .next()
                    .map(|rake| rake.map(RoundRake::radius))
            });
        let spec = slug_spec(world, lance);
        {
            let mut probe = world.resource_mut::<LanceProbe>();
            probe.fired_rake = fired_rake;
            probe.spec = Some(spec);
            probe.stands_fired = true;
        }
        fire_stands(world, spec, ship);
        return;
    }

    // Every plate has to have been raked before the readings mean anything.
    let plates = plate_health(world);
    if plates.len() != PLATES || plates.iter().any(|health| *health >= PLATE_HEALTH) {
        return;
    }

    // Nothing the bank recorded means anything until every stand's slug has
    // spent itself and left the world.
    let flying = world
        .try_query_filtered::<(), With<RailgunSlugProjectileMarker>>()
        .is_some_and(|mut query| query.iter(world).next().is_some());
    if flying {
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

    // --- the measurement bank ---

    let spec = world
        .resource::<LanceProbe>()
        .spec
        .expect("the bank read the lance's authored round before it fired");
    let fired_rake = world.resource::<LanceProbe>().fired_rake;
    assert!(
        fired_rake.is_some_and(|fired| rake_matches(fired, spec.rake)),
        "railgun: the shot did not carry the width the catalog authored: authored {:?}, flew \
         {fired_rake:?}",
        spec.rake
    );
    nova_probe::probe_marker(
        world,
        "outcome: the authored rake rides the shot",
        serde_json::json!({ "authored": spec.rake, "fired": fired_rake.flatten() }),
    );

    let radius = spec.rake.expect(
        "the base lance authors a rake radius; the bank has nothing to measure without one",
    );
    let results = stand_results(world, spec);
    let stand = |wanted: Stand| {
        results
            .iter()
            .find(|result| result.stand == wanted)
            .unwrap_or_else(|| panic!("stand {} was staged", wanted.label()))
    };
    let narrow_line = stand(Stand::NarrowLine);
    let raked_line = stand(Stand::RakedLine);
    let raked_wall = stand(Stand::RakedWall);
    let wide_wall = stand(Stand::WideWall);

    // The corridor, against the four-cell line the balance argument is written
    // against: wider than the bore, no wider than the authored radius, and
    // recorded where the corridor MET each cell rather than on the axis.
    assert!(
        results.iter().all(|result| result.twice == 0),
        "railgun: a stand cell was charged more than once: {results:?}"
    );
    let widened = narrow_line.cells == STAND_DEEP as usize
        && narrow_line.spread <= STAND_CELL * 0.25
        && raked_line.cells > narrow_line.cells
        && raked_line.cells == raked_line.candidates
        && raked_line.spread > STAND_CELL * 0.25
        && raked_line.spread <= radius;
    assert!(
        widened,
        "railgun: the rake did not open a wider corridor than the bore at radius {radius}: narrow \
         {narrow_line:?}, raked {raked_line:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the rake widens the corridor",
        serde_json::json!({
            "radius": radius,
            "cycle_seconds": spec.cycle,
            "before_dps": narrow_line.dps,
            "after_dps": raked_line.dps,
            "stands": stand_payload(&results),
        }),
    );

    // And what stops it is the POWER, not a cap: the wall offers the rake more
    // cells than the slug can pay for, and the slug pays for what the budget
    // buys and no more.
    let cost = STAND_CELL_HEALTH / pierce_power_multiplier(spec.speed);
    let spent = raked_wall.cells as f32 * cost;
    let budgeted = raked_wall.candidates > raked_wall.cells
        && (spent - spec.power).abs() <= cost
        && raked_wall.spread <= radius
        && raked_wall.widest < WALL_HALF;
    assert!(
        budgeted,
        "railgun: the wall did not stop the rake on its own power: {raked_wall:?}, each cell costs \
         {cost}, the budget is {}",
        spec.power
    );
    nova_probe::probe_marker(
        world,
        "outcome: the rake spends one budget",
        serde_json::json!({
            "cells": raked_wall.cells,
            "candidates": raked_wall.candidates,
            "cell_cost": cost,
            "spent": spent,
            "budget": spec.power,
            "widest_ring": raked_wall.widest,
            "wall_edge_ring": WALL_HALF,
        }),
    );

    // Why the base radius is not the blast era's 4.0: the same budget, the same
    // wall, spent sideways on the entry face instead of forward through the
    // hull. A wide rake removes as much and BORES less.
    let craters = wide_wall.depth < raked_wall.depth
        && wide_wall.spread > raked_wall.spread
        && (wide_wall.removed - raked_wall.removed).abs() <= STAND_CELL_HEALTH;
    assert!(
        craters,
        "railgun: the wide seed did not spend one budget differently: raked {raked_wall:?}, wide \
         {wide_wall:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a wide rake craters instead of boring",
        serde_json::json!({
            "authored": { "radius": radius, "deepest_layer": raked_wall.depth, "spread": raked_wall.spread, "removed": raked_wall.removed },
            "wide": { "radius": WIDE_SEED_RADIUS, "deepest_layer": wide_wall.depth, "spread": wide_wall.spread, "removed": wide_wall.removed },
        }),
    );

    world.resource_mut::<LanceProbe>().verified = true;
    info!("railgun: every lance invariant held");
}

/// Did the shot carry the width the catalog authored?
fn rake_matches(fired: Option<f32>, authored: Option<f32>) -> bool {
    match (fired, authored) {
        (None, None) => true,
        (Some(fired), Some(authored)) => (fired - authored).abs() <= f32::EPSILON,
        _ => false,
    }
}

// --- the measurement bank ---------------------------------------------------

/// A stand cell's edge, and the lattice pitch, in units. One unit cell on a one
/// unit lattice - the spacing shipped hulls are built on - so a face
/// neighbour's nearest point lies 0.5 from the bore, a diagonal's 0.707, and a
/// second-ring cell's 1.5.
const STAND_CELL: f32 = 1.0;

/// A stand cell's health: the reinforced hull section's authored pool, so the
/// power arithmetic this bank reports is the game's own and not a rig's.
const STAND_CELL_HEALTH: f32 = 200.0;

/// How deep every stand is, in cells. The four-cell line the lance's balance
/// argument is written against.
const STAND_DEEP: i32 = 4;

/// Half-width of a LINE stand, in cells: the four-cell line with exactly one
/// cell of structure either side of it, which is the corvette that argument
/// describes.
const LINE_HALF: i32 = 1;

/// Half-width and half-height of a WALL stand, in cells: dense material on
/// every side of the bore, wider than the corridor the authored rake cuts, so
/// what stops the rake is the power budget and never the edge of the target.
const WALL_HALF: i32 = 2;

/// The rake radius this task refused to assume, kept as a MEASURED comparison
/// rather than an argument. It is the seed a blast design left behind.
const WIDE_SEED_RADIUS: f32 = 4.0;

/// Where the first stand stands, and how far apart they are, in units. Well
/// clear of the plate stack, which is only eight units across.
const FIRST_STAND_X: f32 = 60.0;
const STAND_PITCH_X: f32 = 40.0;

/// How far downrange a stand's first layer sits, and how far in front of it the
/// stand's slug starts, in units.
const STAND_Z: f32 = -30.0;
const STAND_MUZZLE_LEAD: f32 = 10.0;

/// One measurement stand: a slug of a chosen width, and a block for it to cut.
///
/// Every stand fires the LANCE'S OWN authored round - damage, power, speed and
/// lifetime read straight off this range's section - and varies nothing but the
/// rake width. The gun's cycle is not in this loop on purpose: invariant 6 pins
/// that what content authored is what flew, and the bank then measures what the
/// round does with it. That is what lets one run hold five readings instead of
/// one shell behind a twelve second reload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stand {
    /// The BEFORE, against a corvette's four-cell line.
    NarrowLine,
    /// The AFTER, against the same line.
    RakedLine,
    /// The BEFORE, against dense material.
    NarrowWall,
    /// The AFTER, against the same wall: the corridor, and the budget that
    /// bounds it.
    RakedWall,
    /// The blast-era seed, against the same wall and the same budget.
    WideWall,
}

/// The bank, in the order it is staged along +X and reported in.
const STANDS: [Stand; 5] = [
    Stand::NarrowLine,
    Stand::RakedLine,
    Stand::NarrowWall,
    Stand::RakedWall,
    Stand::WideWall,
];

impl Stand {
    /// This stand's name in a marker payload and a panic.
    const fn label(self) -> &'static str {
        match self {
            Stand::NarrowLine => "narrow_line",
            Stand::RakedLine => "raked_line",
            Stand::NarrowWall => "narrow_wall",
            Stand::RakedWall => "raked_wall",
            Stand::WideWall => "wide_wall",
        }
    }

    /// Half-width and half-height of this stand's block, in cells.
    const fn half(self) -> (i32, i32) {
        match self {
            Stand::NarrowLine | Stand::RakedLine => (LINE_HALF, 0),
            Stand::NarrowWall | Stand::RakedWall | Stand::WideWall => (WALL_HALF, WALL_HALF),
        }
    }

    /// The rake this stand's slug carries, given what the catalog authored.
    fn radius(self, authored: Option<f32>) -> Option<f32> {
        match self {
            Stand::NarrowLine | Stand::NarrowWall => None,
            Stand::RakedLine | Stand::RakedWall => authored,
            Stand::WideWall => Some(WIDE_SEED_RADIUS),
        }
    }
}

/// Marks one stand cell with the stand it belongs to and its lattice address.
#[derive(Component, Clone, Copy, Debug)]
struct StandCell {
    stand: usize,
    x: i32,
    y: i32,
    layer: i32,
}

/// One cell the bank watched a slug pay for, and WHERE the corridor met it.
///
/// The position is the reading a health total cannot give: a lateral bite
/// recorded on the bore's own axis would put its impact cue and its carve mark
/// inside the cell it cut rather than on the face the corridor opened.
#[derive(Clone, Copy, Debug)]
struct StandBite {
    section: Entity,
    cell: StandCell,
    at: Vec3,
}

/// The authored round a stand fires, copied off this range's own lance.
#[derive(Clone, Copy, Debug)]
struct SlugSpec {
    damage: f32,
    power: f32,
    speed: f32,
    lifetime: f32,
    rake: Option<f32>,
    /// Charge plus reload delay, in seconds: what ONE shot costs, and the
    /// divisor every damage-per-second number in this bank is taken over.
    cycle: f32,
}

/// What one stand's slug did to its block.
#[derive(Clone, Debug)]
struct StandResult {
    stand: Stand,
    /// Cells the slug paid for.
    cells: usize,
    /// How many of those it paid for MORE than once. Always zero: a raked
    /// section takes the bite once, across steps and across internal gaps.
    twice: usize,
    /// Health it actually removed.
    removed: f32,
    /// The deepest layer it reached, counting from the entry face.
    depth: i32,
    /// The furthest from the bore a bite was RECORDED, in units.
    spread: f32,
    /// How many cells the corridor took out of each layer, from the entry
    /// face inward. The corridor's shape in one line.
    profile: Vec<usize>,
    /// The furthest lattice ring a bite reached, in cells. A corridor that
    /// touches its block's edge was cut short by the target rather than by the
    /// slug, and its reading means nothing.
    widest: i32,
    /// Cells inside this stand's corridor that the slug could have paid for.
    candidates: usize,
    /// Effective damage per second over the lance's whole cycle.
    dps: f32,
}

/// Where a stand's block stands.
fn stand_origin(index: usize) -> Vec3 {
    Vec3::new(FIRST_STAND_X + index as f32 * STAND_PITCH_X, 0.0, STAND_Z)
}

/// How far a lattice cell's NEAREST point lies from the bore, in units. The
/// same distance the swept corridor tests against, so a cell inside this radius
/// is a cell the rake owes a bite.
fn cell_offset(x: i32, y: i32) -> f32 {
    let reach = |n: i32| (n.abs() as f32 * STAND_CELL - STAND_CELL * 0.5).max(0.0);
    reach(x).hypot(reach(y))
}

/// Stand the measurement blocks up.
///
/// STATIC bodies, so the block cannot drift and the corridor's geometry is the
/// lattice's own. A cell the rake empties is destroyed and leaves, which is
/// what makes the corridor a hole you can see in a rendered run - and why the
/// reading is taken from the round's own impacts rather than from health that
/// walked out of the world.
fn stage_stands(world: &mut World) {
    let mesh = world
        .resource_mut::<Assets<Mesh>>()
        .add(Cuboid::from_length(STAND_CELL));
    let intact = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::srgb(0.45, 0.47, 0.5),
            perceptual_roughness: 0.9,
            ..default()
        });

    for (index, stand) in STANDS.iter().enumerate() {
        let (half_wide, half_tall) = stand.half();
        let body = world
            .spawn((
                Name::new(format!("stand {}", stand.label())),
                RigidBody::Static,
                Transform::from_translation(stand_origin(index)),
                // The block's cells are drawn, so the block itself has to be
                // in the visibility hierarchy or every one of them warns.
                Visibility::Visible,
            ))
            .id();
        for layer in 0..STAND_DEEP {
            for y in -half_tall..=half_tall {
                for x in -half_wide..=half_wide {
                    world.spawn((
                        ChildOf(body),
                        SectionMarker,
                        StandCell {
                            stand: index,
                            x,
                            y,
                            layer,
                        },
                        Transform::from_translation(Vec3::new(
                            x as f32 * STAND_CELL,
                            y as f32 * STAND_CELL,
                            -(layer as f32 + 0.5) * STAND_CELL,
                        )),
                        Collider::cuboid(STAND_CELL, STAND_CELL, STAND_CELL),
                        ColliderDensity(1.0),
                        Health::new(STAND_CELL_HEALTH),
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(intact.clone()),
                    ));
                }
            }
        }
    }
}

/// Read the lance's authored round off the section itself.
fn slug_spec(world: &World, lance: Entity) -> SlugSpec {
    let config = world
        .get::<RailgunSectionConfigHelper>(lance)
        .expect("the lance carries its authored config");
    SlugSpec {
        damage: config.slug_damage,
        power: config.slug_power,
        speed: config.slug_speed,
        lifetime: config.slug_lifetime,
        // FILTERED, exactly as `charge_and_fire_railgun` filters it: an
        // authored zero spawns no rake component at all, so the bank must read
        // it as no rake rather than as a rake of width nothing.
        rake: config.rake_radius.filter(|radius| *radius > 0.0),
        cycle: config.charge_seconds + config.reload.map_or(0.0, |reload| reload.delay),
    }
}

/// Fire every stand's slug on the same tick, down its own bore.
fn fire_stands(world: &mut World, spec: SlugSpec, owner: Entity) {
    for (index, stand) in STANDS.iter().enumerate() {
        let start = stand_origin(index) + Vec3::Z * STAND_MUZZLE_LEAD;
        let mut slug = world.spawn((
            Name::new(format!("stand slug {}", stand.label())),
            RailgunSlugProjectileMarker,
            ProjectileOwner(owner),
            Transform::from_translation(start),
            RoundVelocity(Vec3::NEG_Z * spec.speed),
            // The lance's own bundle: power is the only bound, exactly as
            // `charge_and_fire_railgun` spawns it.
            ProjectileDamage {
                amount: spec.damage,
                power: spec.power,
                layers: u32::MAX,
                kind: DamageType::Pierce,
            },
            TempEntity(spec.lifetime),
            Visibility::Visible,
        ));
        if let Some(radius) = stand.radius(spec.rake) {
            slug.insert(RoundRake::new(radius));
        }
    }
}

/// Record every stand bite.
///
/// The ROUND's own report, not the block's health: a cell the rake empties is
/// destroyed and leaves the world, which is what makes the corridor a visible
/// hole and what makes surviving health useless as a reading.
fn record_stand_bites(
    impact: On<SurfaceImpact>,
    q_cell: Query<&StandCell>,
    mut probe: ResMut<LanceProbe>,
) {
    let Ok(&cell) = q_cell.get(impact.entity) else {
        return;
    };
    probe.bites.push(StandBite {
        section: impact.entity,
        cell,
        at: impact.at,
    });
}

/// Fold the bank's cells and bites into one reading per stand.
fn stand_results(world: &mut World, spec: SlugSpec) -> Vec<StandResult> {
    // A stand cell is full when the slug arrives and is bitten at most once,
    // so what one bite removed is the whole of what one cell had to give.
    let per_cell = spec.damage.min(STAND_CELL_HEALTH);
    let bites = world.resource::<LanceProbe>().bites.clone();
    STANDS
        .iter()
        .enumerate()
        .map(|(index, &stand)| {
            let mine: Vec<&StandBite> = bites
                .iter()
                .filter(|bite| bite.cell.stand == index)
                .collect();
            let once: BTreeSet<Entity> = mine.iter().map(|bite| bite.section).collect();
            let origin = stand_origin(index);
            let spread = mine
                .iter()
                .map(|bite| (bite.at - origin).truncate().length())
                .fold(0.0f32, f32::max);
            let depth = mine.iter().map(|bite| bite.cell.layer).max().unwrap_or(-1);
            let profile = (0..STAND_DEEP)
                .map(|layer| mine.iter().filter(|bite| bite.cell.layer == layer).count())
                .collect();
            let widest = mine
                .iter()
                .map(|bite| bite.cell.x.abs().max(bite.cell.y.abs()))
                .max()
                .unwrap_or(-1);
            let (half_wide, half_tall) = stand.half();
            let radius = stand.radius(spec.rake).unwrap_or(0.0);
            let per_layer = (-half_tall..=half_tall)
                .flat_map(|y| (-half_wide..=half_wide).map(move |x| cell_offset(x, y)))
                .filter(|offset| *offset <= radius)
                .count();
            let candidates = per_layer * STAND_DEEP as usize;
            StandResult {
                stand,
                cells: once.len(),
                twice: mine.len() - once.len(),
                removed: once.len() as f32 * per_cell,
                depth,
                spread,
                profile,
                widest,
                candidates,
                dps: once.len() as f32 * per_cell / spec.cycle,
            }
        })
        .collect()
}

/// Every stand's reading as one JSON payload, so a marker carries the numbers
/// and not just the verdict.
fn stand_payload(results: &[StandResult]) -> serde_json::Value {
    serde_json::Value::Object(
        results
            .iter()
            .map(|result| {
                (
                    result.stand.label().to_string(),
                    serde_json::json!({
                        "cells": result.cells,
                        "charged_twice": result.twice,
                        "candidates": result.candidates,
                        "removed": result.removed,
                        "deepest_layer": result.depth,
                        "cells_per_layer": result.profile,
                        "spread": result.spread,
                        "widest_ring": result.widest,
                        "dps": result.dps,
                    }),
                )
            })
            .collect(),
    )
}
