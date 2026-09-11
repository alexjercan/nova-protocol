//! system_ai_combat: what an AI ship actually does when it fights, staged on
//! the fleet's two scales at once.
//!
//! Task `20260909-213708` re-derives the AI's flight figures - the standoff it
//! settles at, the speed it circles at, the length of a jink leg - from the
//! hulls in the fight instead of from a constant tuned on one ship. Each of
//! those items is a BALANCE change, so each needs a reading of the fight
//! BEFORE it lands and a reading after. This range is where the fight is read.
//!
//! Two engagements run side by side, far enough apart that neither scanner can
//! hear the other: an escort fight (`block_picket` against a parked
//! `block_skiff`) and a capital fight (`block_warship`, the only capital
//! combatant the base game ships, against a parked `block_carrier`, the
//! largest hull it ships). The two are chosen to put a factor of three between
//! the summed hull radii of the two fights, because that sum is what the
//! standoff item is about.
//!
//! Both movers fly with EMPTY MAGAZINES. The subject here is how an AI ship
//! flies a fight, and a live one is over before the flying settles: a picket
//! guts a skiff in nineteen seconds and the warship's first torpedo salvo
//! breaks the carrier into six bodies inside two. A dry gun changes nothing
//! about the flight - an armed hull still acquires, commits and flies its
//! envelope - and it leaves a target whose radius is still the radius the
//! standoff was computed against. Gunnery has its own ranges.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: an engaged hull is flown by its flight computer` | a ship that has joined a fight holds a velocity on the autopilot, with the nose asked for the ship it is fighting, instead of writing thruster input of its own |
//! | 2 | `outcome: an engaged hull holds its standoff instead of closing` | both movers finish the settle circling their target rather than still flying at it |
//! | 3 | `outcome: the engagement geometry is recorded` | RECORD: per fight, the centre gap, the face gap, both live hull radii, the speed and closing speed, the asked-for and achieved nose angles, and the largest live throttle |
//!
//! Claim 3 asserts NOTHING. It is the table `tasks/20260909-213118/FEEDBACK.md`
//! quotes for the standoff, orbit-speed and jink items, read against the
//! ledger and never against a threshold. The hull's ACHIEVED nose angle lives
//! there rather than under claim 1 on purpose: the facing is a request the
//! hull settles onto at its own turn rate, so on a capital hull it lags an
//! orbit by tens of degrees and a snapshot of it is a sample of that lag.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_ai_combat --features debug
//! # look for: `ai combat: escort: ...`,
//! #           `ai combat: capital: ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[path = "../screenshots/shared/kit.rs"]
mod kit;

#[cfg(feature = "debug")]
use std::sync::Arc;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_ai_combat")]
#[command(version = "1.0.0")]
#[command(
    about = "What an AI ship does when it fights, staged on the fleet's two scales. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

/// The escort fight's mover: the cleanup group's armed picket, one nose gun.
const PICKET: &str = "block_picket";

/// The escort fight's target: the cleanup group's unarmed needle.
const SKIFF: &str = "block_skiff";

/// The capital fight's mover: the stolen Earth warship, the only capital
/// combatant the base game ships.
const WARSHIP: &str = "block_warship";

/// The capital fight's target: the campaign's home, and the largest hull the
/// base game ships.
const CARRIER: &str = "block_carrier";

/// The escort mover's scenario id.
const PICKET_ID: &str = "ai_combat_picket";

/// The escort target's scenario id.
const SKIFF_ID: &str = "ai_combat_skiff";

/// The capital mover's scenario id.
const WARSHIP_ID: &str = "ai_combat_warship";

/// The capital target's scenario id.
const CARRIER_ID: &str = "ai_combat_carrier";

/// How far apart the two fights are staged, m.
///
/// Over the engine's 20 km sensor default, so neither fight can hear the other
/// and each mover has exactly one contact to choose between. A shorter stage
/// would leave the picket's target selection to read as a measurement of this
/// range's layout.
const FIGHT_SEPARATION: Meters = Meters(30_000.0);

/// Where each mover starts, m from its target.
///
/// Inside the engine's 4 km engage gate so the fight joins on the first tick
/// rather than after a cruise, and well outside the 1 km standoff the fight is
/// supposed to settle at, so the approach leg is really flown.
const APPROACH_RANGE: Meters = Meters(2_500.0);

/// In-step seconds a beat gets to reach its world condition.
///
/// Over the fleet's usual step deadline for the reason `system_hud_shell`
/// gives: CI runs the correctness pass on a software rasterizer, where one
/// frame of a 2 081-section carrier costs seconds, and the settle beat here
/// waits on a capital hull swinging at its own torque ceiling. A backstop that
/// names a hung beat, not a budget the range is held to.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 600.0;

/// In-step seconds both fights get to settle once both movers have committed.
///
/// Budgeted for the slower hull: a capital ship turns at its torque ceiling,
/// and the warship has to swing its nose onto the carrier and fly 1.5 km in
/// before anything it does is a reading of the standoff rather than of the
/// approach. Both fights are holding station by around half of this.
#[cfg(feature = "debug")]
const SETTLE_SECS: f32 = 60.0;

/// How far off the line of sight the facing an engaged mover ASKS FOR may
/// sit, degrees.
///
/// A DELIVERY GUARD on the request, not an authored number. The AI rewrites
/// the facing from live anchors every tick, so the only distance between the
/// two is the fraction of a tick the hulls have moved since - a shipped fight
/// holds this under a degree. What this catches is the request going stale or
/// going somewhere else: a maneuver the AI stopped refreshing, or a helm that
/// kept an old action.
#[cfg(feature = "debug")]
const FACING_ERROR_CEILING_DEG: f32 = 5.0;

/// How much of a settled mover's speed may still be closing the range.
///
/// A DELIVERY GUARD on the envelope, not an authored number: the whole point
/// of a standoff is that a fight SETTLES at a distance, so a mover still
/// spending a quarter of its speed on the approach has not settled into
/// anything. Both shipped fights hold under a tenth.
#[cfg(feature = "debug")]
const CLOSING_SHARE_CEILING: f32 = 0.25;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(combat_script());
    }

    app.run()
}

fn range_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    ships: Res<GameShips>,
    sections: Res<GameSections>,
) {
    commands.trigger(LoadScenario(combat_range(&game_assets, &ships, &sections)));
}

/// Every weapon section on `hull`, given a hard magazine of nothing.
///
/// Read off the hull's own section list rather than written out by id, so a
/// re-armed catalog ship arrives here dry as well instead of quietly shooting
/// the range's target apart again.
fn dry_magazines(hull: &ShipHull, sections: &GameSections) -> Vec<ShipSectionModification> {
    hull.sections
        .iter()
        .filter(|section| {
            let config = match &section.source {
                SectionSource::Inline(config) => Some(config),
                SectionSource::Prototype(id) => sections.get_section(id),
            };
            config.is_some_and(|config| {
                matches!(
                    config.kind,
                    SectionKind::Turret(_) | SectionKind::Torpedo(_) | SectionKind::Railgun(_)
                )
            })
        })
        .map(|section| ShipSectionModification {
            section: section.id.clone(),
            modifications: vec![SectionModification::SetAmmo(0)],
        })
        .collect()
}

/// The range: two fights in flat space, each an AI mover closing on a parked
/// hostile of its own scale.
fn combat_range(
    game_assets: &GameAssets,
    ships: &GameShips,
    sections: &GameSections,
) -> ScenarioConfig {
    let ship = |id: &str, name: &str, catalog: &str, at: Meters3, spec: SpaceshipConfig| {
        let hull = kit::catalog_ship(ships, catalog);
        let modifications = dry_magazines(&hull, sections);
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: name.to_string(),
                position: at,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                hull: ShipSource::Inline(hull),
                modifications,
                ..spec
            }),
        })
    };

    // The mover flies itself on the shipped AI defaults - a 4 km engage gate
    // and a 20 km scanner - because the figures this range reads are the
    // engine's, and an override here would be a reading of the override.
    let mover = SpaceshipConfig {
        controller: SpaceshipController::AI(AIControllerConfig::default()),
        allegiance: Some(Allegiance::Enemy),
        ..default()
    };
    // The target is a parked hostile with nobody at the helm: it is what the
    // mover flies around, not half of a duel.
    let target = SpaceshipConfig {
        controller: SpaceshipController::None,
        allegiance: Some(Allegiance::Player),
        ..default()
    };

    let escort = Meters3::new(-FIGHT_SEPARATION.get() / 2.0, 0.0, 0.0);
    let capital = Meters3::new(FIGHT_SEPARATION.get() / 2.0, 0.0, 0.0);
    let approach = Meters3::new(0.0, 0.0, APPROACH_RANGE.get());

    ScenarioConfig {
        description:
            "Two AI ships joining two fights, one escort-scale and one capital-scale, with \
             every magazine on the range empty."
                .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    ship(SKIFF_ID, "Skiff", SKIFF, escort, target.clone()),
                    ship(
                        PICKET_ID,
                        "Picket",
                        PICKET,
                        escort + approach,
                        mover.clone(),
                    ),
                    ship(CARRIER_ID, "Carrier", CARRIER, capital, target),
                    ship(WARSHIP_ID, "Warship", WARSHIP, capital + approach, mover),
                ],
                ThreePointRig::around("ai combat", Meters3::ZERO, 40.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "ai_combat_range".to_string(),
            "AI Combat Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The scripted run: load four hulls, wait for both movers to commit, let both
/// fights settle, read them.
#[cfg(feature = "debug")]
fn combat_script() -> Script {
    Script::new()
        .step("load both fights")
        .enter(GameStates::Loading)
        .until(every_hull_weighed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let both movers commit to their fights")
        .until(both_movers_engaged())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let both fights settle")
        .until(elapsed(SETTLE_SECS))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("measure both fights")
        .on_enter(measure_both_fights)
        .add()
}

/// The two fights, named once so the script, the log and the record agree.
#[cfg(feature = "debug")]
const FIGHTS: [Fight; 2] = [
    Fight {
        label: "escort",
        mover: PICKET,
        mover_id: PICKET_ID,
        target: SKIFF,
        target_id: SKIFF_ID,
    },
    Fight {
        label: "capital",
        mover: WARSHIP,
        mover_id: WARSHIP_ID,
        target: CARRIER,
        target_id: CARRIER_ID,
    },
];

/// One staged engagement: who is flying it and who they are flying at.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug)]
struct Fight {
    /// The fight's name in the log and in the record.
    label: &'static str,
    /// Catalog id of the hull under test.
    mover: &'static str,
    /// Scenario id of the hull under test.
    mover_id: &'static str,
    /// Catalog id of the hull it is fighting.
    target: &'static str,
    /// Scenario id of the hull it is fighting.
    target_id: &'static str,
}

/// Every staged hull is present AND has been weighed: a root avian has not
/// measured yet has no centre of mass and no hull radius, so every distance
/// this range reads would be taken from the build origin instead.
#[cfg(feature = "debug")]
fn every_hull_weighed() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        [PICKET_ID, SKIFF_ID, WARSHIP_ID, CARRIER_ID]
            .iter()
            .all(|id| {
                staged_hull(world, id).is_some_and(|root| {
                    world.get::<HullRadius>(root).is_some()
                        && world
                            .get::<avian3d::prelude::ComputedCenterOfMass>(root)
                            .is_some()
                })
            })
    })
}

/// Both movers have chosen their target and left their passive routine for it.
#[cfg(feature = "debug")]
fn both_movers_engaged() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        FIGHTS.iter().all(|fight| {
            staged_hull(world, fight.mover_id).is_some_and(|root| {
                world
                    .get::<AIBehaviorState>(root)
                    .is_some_and(|state| *state == AIBehaviorState::Engage)
            })
        })
    })
}

/// The hull with this scenario id, off a borrowed world.
#[cfg(feature = "debug")]
fn staged_hull(world: &World, id: &str) -> Option<Entity> {
    let mut hulls = world.try_query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()?;
    hulls
        .iter(world)
        .find(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
}

/// Everything one settled fight says about how the AI flies.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug)]
struct Engagement {
    /// Live radius of the hull flying the fight.
    mover_radius: Meters,
    /// Live radius of the hull it is fighting.
    target_radius: Meters,
    /// Centre of mass to centre of mass - the distance the standoff envelope
    /// is currently written against.
    centre_gap: Meters,
    /// The same gap with both live radii taken out: the space a player sees
    /// between the two skins. Negative means the hulls overlap.
    face_gap: Meters,
    /// How fast the mover is moving relative to its target.
    speed: MetersPerSecond,
    /// How much of that speed is closing the range; negative is opening it.
    closing: MetersPerSecond,
    /// Angle between the mover's nose and the line of sight to its target:
    /// where the hull has actually got to, lag included.
    nose_error_deg: f32,
    /// Angle between the facing the mover ASKS the helm for and the line of
    /// sight. `None` when it is holding no velocity, or holding one with no
    /// facing asked for.
    facing_error_deg: Option<f32>,
    /// Whether the mover is flying on a held velocity rather than on thruster
    /// input of its own.
    on_the_computer: bool,
    /// The largest throttle any live drive on the mover is carrying.
    throttle: f32,
}

/// Read one staged fight out of the live world.
#[cfg(feature = "debug")]
fn read_engagement(world: &World, fight: Fight) -> Engagement {
    use avian3d::prelude::{ComputedCenterOfMass, LinearVelocity};

    let find = |id: &str| {
        staged_hull(world, id).unwrap_or_else(|| panic!("ai combat: hull '{id}' is present"))
    };
    let mover = find(fight.mover_id);
    let target = find(fight.target_id);

    let pose = |root: Entity| {
        let transform = world
            .get::<Transform>(root)
            .copied()
            .unwrap_or(Transform::IDENTITY);
        let anchor = live_structure_anchor(&transform, world.get::<ComputedCenterOfMass>(root));
        let velocity = world
            .get::<LinearVelocity>(root)
            .map_or(Vec3::ZERO, |v| **v);
        (transform, anchor, velocity)
    };
    let (mover_pose, mover_anchor, mover_velocity) = pose(mover);
    let (_, target_anchor, target_velocity) = pose(target);
    let radius =
        |root: Entity| Meters::from_engine(world.get::<HullRadius>(root).map_or(0.0, |r| **r));

    let to_target = target_anchor - mover_anchor;
    let centre_gap = Meters::from_engine(to_target.length());
    let mover_radius = radius(mover);
    let target_radius = radius(target);

    // Engine boundary: avian keeps positions and velocities in world units, so
    // both cross back to SI here and nothing below this line is in units.
    let relative = mover_velocity - target_velocity;
    let los = to_target.normalize_or_zero();
    let angle_to_los = |direction: Vec3| direction.dot(los).clamp(-1.0, 1.0).acos().to_degrees();
    let nose_error_deg = angle_to_los(*mover_pose.forward());

    let held = world
        .get::<Autopilot>(mover)
        .map(|autopilot| autopilot.action)
        .and_then(|action| match action {
            AutopilotAction::MatchVelocity { facing, .. } => Some(facing),
            _ => None,
        });

    let throttle = world
        .try_query_filtered::<(&ThrusterSectionInput, &ChildOf), Without<SectionInactiveMarker>>()
        .map_or(0.0, |mut drives| {
            drives
                .iter(world)
                .filter(|(_, parent)| parent.parent() == mover)
                .map(|(input, _)| **input)
                .fold(0.0_f32, f32::max)
        });

    Engagement {
        mover_radius,
        target_radius,
        centre_gap,
        face_gap: centre_gap - mover_radius - target_radius,
        speed: MetersPerSecond::from_engine(relative.length()),
        closing: MetersPerSecond::from_engine(relative.dot(los)),
        nose_error_deg,
        facing_error_deg: held.flatten().map(|facing| angle_to_los(*facing)),
        on_the_computer: held.is_some(),
        throttle,
    }
}

#[cfg(feature = "debug")]
fn log_engagement(fight: Fight, engagement: Engagement) {
    info!(
        "ai combat: {}: {} vs {}: centre_gap={:.0} m face_gap={:.0} m radii={:.0} m / {:.0} m \
         speed={:.0} m/s closing={:+.0} m/s nose_error={:.1} deg facing_error={:.1} deg \
         on_the_computer={} throttle={:.3}",
        fight.label,
        fight.mover,
        fight.target,
        engagement.centre_gap.get(),
        engagement.face_gap.get(),
        engagement.mover_radius.get(),
        engagement.target_radius.get(),
        engagement.speed.get(),
        engagement.closing.get(),
        engagement.nose_error_deg,
        engagement.facing_error_deg.unwrap_or(f32::NAN),
        engagement.on_the_computer,
        engagement.throttle,
    );
}

#[cfg(feature = "debug")]
fn engagement_payload(fight: Fight, engagement: Engagement) -> serde_json::Value {
    serde_json::json!({
        "mover": fight.mover,
        "target": fight.target,
        "mover_radius_m": engagement.mover_radius.get(),
        "target_radius_m": engagement.target_radius.get(),
        "centre_gap_m": engagement.centre_gap.get(),
        "face_gap_m": engagement.face_gap.get(),
        "speed_mps": engagement.speed.get(),
        "closing_mps": engagement.closing.get(),
        "nose_error_deg": engagement.nose_error_deg,
        "facing_error_deg": engagement.facing_error_deg,
        "on_the_computer": engagement.on_the_computer,
        "throttle": engagement.throttle,
    })
}

/// The one beat: read both settled fights, assert what the AI owes a fight it
/// has joined, and record the table the ledger quotes.
#[cfg(feature = "debug")]
fn measure_both_fights(world: &mut World) {
    let read = FIGHTS.map(|fight| (fight, read_engagement(world, fight)));
    for (fight, engagement) in read {
        log_engagement(fight, engagement);
    }

    for (fight, engagement) in read {
        assert!(
            engagement.on_the_computer,
            "ai combat: {}: {} has joined a fight but is not flying it on its flight computer - \
             the AI is supposed to hand the helm a velocity and let the computer choose the \
             engines, so a hull with no held velocity is steering itself again",
            fight.label, fight.mover,
        );
        let facing_error = engagement.facing_error_deg.unwrap_or(f32::INFINITY);
        assert!(
            facing_error <= FACING_ERROR_CEILING_DEG,
            "ai combat: {}: {} is holding a velocity but asking the helm for a facing {:.1} deg \
             off {} (ceiling {:.0} deg) - the maneuver is rewritten from live anchors every \
             tick, so a request this stale means it stopped being rewritten",
            fight.label,
            fight.mover,
            facing_error,
            fight.target,
            FACING_ERROR_CEILING_DEG,
        );
    }
    nova_probe::probe_marker(
        world,
        "outcome: an engaged hull is flown by its flight computer",
        serde_json::json!({
            "movers": FIGHTS.map(|fight| fight.mover),
            "facing_error_deg": read.map(|(_, engagement)| engagement.facing_error_deg),
        }),
    );

    for (fight, engagement) in read {
        let closing_share =
            engagement.closing.get().abs() / engagement.speed.get().max(f32::EPSILON);
        assert!(
            engagement.speed > MetersPerSecond::ZERO && closing_share <= CLOSING_SHARE_CEILING,
            "ai combat: {}: {} finished the settle flying {:.0} m/s with {:+.0} m/s of it still \
             closing on {} ({:.0}% of its speed, ceiling {:.0}%) - a standoff envelope is \
             supposed to SETTLE at a distance and circle there, not arrive at one",
            fight.label,
            fight.mover,
            engagement.speed.get(),
            engagement.closing.get(),
            fight.target,
            closing_share * 100.0,
            CLOSING_SHARE_CEILING * 100.0,
        );
    }
    nova_probe::probe_marker(
        world,
        "outcome: an engaged hull holds its standoff instead of closing",
        serde_json::json!({
            "centre_gap_m": read.map(|(_, engagement)| engagement.centre_gap.get()),
            "closing_mps": read.map(|(_, engagement)| engagement.closing.get()),
        }),
    );

    let mut record = serde_json::Map::new();
    for (fight, engagement) in read {
        record.insert(
            fight.label.to_string(),
            engagement_payload(fight, engagement),
        );
    }
    nova_probe::probe_marker(
        world,
        "outcome: the engagement geometry is recorded",
        serde_json::Value::Object(record),
    );
    info!("ai combat: both fights measured");
}
