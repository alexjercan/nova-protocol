//! system_ai_evade: what an AI ship does when a gun is held on it, staged on
//! two hull scales at once.
//!
//! Task `20260909-213708` re-derives the jink from the hull that flies it: a
//! leg is a DISPLACEMENT - it runs until the ship has carried its whole hull
//! plus 10 m off the line the gun was holding - and the speed and the length
//! of the leg follow from that and from the drive. The figure it replaced was
//! a flat 1.2 s leg, which let a capital turn a sixth of the way onto its new
//! heading and never thrust at all. This range is where the weave is read.
//!
//! Two weaves run side by side, far enough apart that neither scanner can hear
//! the other: an escort weave (`block_picket`) and a capital weave
//! (`block_warship`, the largest hull that can fly a fight at all - see
//! [`WARSHIP`]). Each mover closes on a parked hostile `block_skiff` whose
//! nose is held on it, which is the threat signal by itself - no shot is fired
//! anywhere on this range, and every magazine on it is empty. A hull that
//! jinks because it was hit is the same cycle with a different trigger, and
//! `threat.rs` unit-tests that edge; what needs a live hull is the FLYING.
//!
//! Both movers start 2.4 km out, outside the 2 km the aiming-at-me gate
//! reaches and inside the 4 km engage gate. So each one joins the fight, flies
//! its approach, and breaks into the weave only once it is close enough to be
//! under the gun - which is also late enough that its hull has been weighed
//! and the leg is armed from a live arm rather than from a zero.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a gun held on a hull breaks it into a weave` | a hostile nose alone takes an engaged hull out of its standoff into a jink cycle of three legs, and the cycle ENDS - the hull is back in the fight afterwards, not parked on a leg it can never finish |
//! | 2 | `outcome: a jink leg carries the whole hull off the line` | each cycle flies at least one leg that displaces the hull its own arm plus 10 m, so what a leg is worth is read off the hull and not off a constant |
//! | 3 | `outcome: the weave is recorded` | RECORD: per hull, the live arm, the drive and turn authority, and every leg's asked-for clearance, achieved displacement and duration |
//!
//! Claim 2 is ONE leg and not all three on purpose. A leg ends on the
//! clearance it achieved OR on its liveness deadline, and the deadline is
//! there for the hull that cannot - a ship that breaks off while still
//! carrying its approach speed spends a leg killing that speed, and the
//! measured escort cycle does exactly that: 60 m, then -32 m against the new
//! heading, then 61 m, against 60 m owed. Asserting every leg would be
//! asserting that a hull never enters the weave with momentum, which is not
//! what the cycle promises and not what a fight looks like. What the cycle
//! promises is that the leg is sized by the HULL, and a cycle none of whose
//! legs reaches its own arm has stopped being that. Every leg is recorded
//! under claim 3 either way.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_ai_evade --features debug
//! # look for: `ai evade: escort: ...`,
//! #           `ai evade: capital: ...`,
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
#[command(name = "system_ai_evade")]
#[command(version = "1.0.0")]
#[command(
    about = "What an AI ship does when a gun is held on it, staged on two hull scales. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

/// The escort weave's mover: the cleanup group's armed picket.
const PICKET: &str = "block_picket";

/// The capital weave's mover: the stolen Earth warship, the only capital
/// combatant the base game ships.
///
/// NOT `block_carrier`, which is bigger. A hull that ships no weapon section
/// is an `AINonCombatant` by the shipped rule - it never acquires a target,
/// so it never engages, and a ship that never engages can never be broken out
/// of an engagement. The carrier is staged as the thing an AI ship fights
/// (`system_ai_combat`) and as an arm to read (`system_hull_scaling`); the
/// jink belongs to the largest hull that can fly one.
const WARSHIP: &str = "block_warship";

/// What holds the gun on each mover: the cleanup group's unarmed needle,
/// parked and pointed. The threat model reads a hostile's HULL AXIS, so the
/// smallest shipped hull is as much of a threat as a capital and costs the
/// range the least to stage.
const THREAT: &str = "block_skiff";

/// The escort mover's scenario id.
const PICKET_ID: &str = "ai_evade_picket";

/// The scenario id of the hull pointed at the escort mover.
const PICKET_THREAT_ID: &str = "ai_evade_picket_threat";

/// The capital mover's scenario id.
const WARSHIP_ID: &str = "ai_evade_warship";

/// The scenario id of the hull pointed at the capital mover.
const WARSHIP_THREAT_ID: &str = "ai_evade_warship_threat";

/// How far apart the two weaves are staged, m.
///
/// Over the engine's 20 km sensor default, so neither weave can hear the other
/// and each mover has exactly one contact to break off from.
const WEAVE_SEPARATION: Meters = Meters(30_000.0);

/// Where each mover starts, m from the hull pointed at it.
///
/// Outside the 2 km the aiming-at-me gate reaches and inside the 4 km engage
/// gate: the mover joins the fight at once and flies a real approach before
/// anything it does is a reading of the weave.
const APPROACH_RANGE: Meters = Meters(2_400.0);

/// In-step seconds a beat gets to reach its world condition.
///
/// Over the fleet's usual step deadline for the reason `system_hud_shell`
/// gives: CI runs the correctness pass on a software rasterizer, where one
/// frame of a 2 081-section carrier costs seconds, and the weave beat here
/// waits on a capital hull flying three legs at its own drive ceiling. A
/// backstop that names a hung beat, not a budget the range is held to.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 900.0;

/// How many legs one jink cycle owes, from `AI_EVADE_LEGS`.
///
/// Mirrored rather than imported: the constant is `pub(super)` inside
/// `nova_ship::input::ai`, and a range that re-states the figure fails when
/// the engine's moves under it, which is the point of a range.
#[cfg(feature = "debug")]
const WEAVE_LEGS: usize = 3;

/// Lateral clearance one leg buys ON TOP of the evading hull's own arm, m.
/// Mirrored from `AI_EVADE_CLEARANCE`, about a section's width.
#[cfg(feature = "debug")]
const LEG_CLEARANCE: Meters = Meters(10.0);

/// How far the held heading has to turn before the range calls it a new leg,
/// degrees.
///
/// The jink pattern puts consecutive legs about 93 degrees apart, and a leg's
/// own heading drifts only with the line of sight - under 10 degrees over a
/// leg at this range. Halfway between the two, so neither a slow drift nor a
/// noisy frame can be read as a leg boundary.
#[cfg(feature = "debug")]
const LEG_TURN_DEG: f32 = 45.0;

/// How much of a leg's asked-for clearance counts as flown.
///
/// A DELIVERY GUARD, not an authored number: the leg ends on the frame the
/// displacement is reached, so the reading is one frame of overshoot past the
/// clearance, never short of it. The slack is for the sample itself - the
/// range reads the offset in `Last`, a frame after the cycle advanced.
#[cfg(feature = "debug")]
const LEG_FLOWN_SHARE: f32 = 0.9;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.init_resource::<WeaveProbe>();
        app.add_systems(Last, watch_the_weaves);
        app.add_plugins(evade_script());
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
    commands.trigger(LoadScenario(evade_range(&game_assets, &ships, &sections)));
}

/// Every weapon section on `hull`, given a hard magazine of nothing.
///
/// Read off the hull's own section list rather than written out by id, so a
/// re-armed catalog ship arrives here dry as well. Nothing on this range is
/// supposed to fire: the threat it reads is a nose, not a hit.
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

/// The range: two weaves in flat space, each an AI mover closing on a parked
/// hostile whose nose is already on it.
fn evade_range(
    game_assets: &GameAssets,
    ships: &GameShips,
    sections: &GameSections,
) -> ScenarioConfig {
    let ship = |id: &str,
                name: &str,
                catalog: &str,
                at: Meters3,
                rotation: Quat,
                spec: SpaceshipConfig| {
        let hull = kit::catalog_ship(ships, catalog);
        let modifications = dry_magazines(&hull, sections);
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: name.to_string(),
                position: at,
                rotation,
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
    // The hull holding the gun: hostile, parked, with nobody at the helm, so
    // the nose it points stays pointed for the whole run.
    let threat = SpaceshipConfig {
        controller: SpaceshipController::None,
        allegiance: Some(Allegiance::Player),
        ..default()
    };
    // Each mover is staged straight down +Z from the hull pointed at it, so
    // the nose stays on it through the whole approach: a threat that has to
    // slew would make the break-off time a reading of the stage.
    let pointed = Transform::default().looking_at(Vec3::Z, Vec3::Y).rotation;

    let escort = Meters3::new(-WEAVE_SEPARATION.get() / 2.0, 0.0, 0.0);
    let capital = Meters3::new(WEAVE_SEPARATION.get() / 2.0, 0.0, 0.0);
    let approach = Meters3::new(0.0, 0.0, APPROACH_RANGE.get());

    ScenarioConfig {
        description: "Two AI ships breaking off from a gun held on them, one escort-scale and \
                      one capital-scale, with every magazine on the range empty."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    ship(
                        PICKET_THREAT_ID,
                        "Escort Threat",
                        THREAT,
                        escort,
                        pointed,
                        threat.clone(),
                    ),
                    ship(
                        PICKET_ID,
                        "Picket",
                        PICKET,
                        escort + approach,
                        Quat::IDENTITY,
                        mover.clone(),
                    ),
                    ship(
                        WARSHIP_THREAT_ID,
                        "Capital Threat",
                        THREAT,
                        capital,
                        pointed,
                        threat,
                    ),
                    ship(
                        WARSHIP_ID,
                        "Warship",
                        WARSHIP,
                        capital + approach,
                        Quat::IDENTITY,
                        mover,
                    ),
                ],
                ThreePointRig::around("ai evade", Meters3::ZERO, 40.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "ai_evade_range".to_string(),
            "AI Evade Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The scripted run: load four hulls, let both movers fly in, break off and
/// come back out of the weave, then read both cycles.
#[cfg(feature = "debug")]
fn evade_script() -> Script {
    Script::new()
        .step("load both weaves")
        .enter(GameStates::Loading)
        .until(every_hull_weighed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let both movers break off and come back out of it")
        .until(both_weaves_closed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("measure both weaves")
        .on_enter(measure_both_weaves)
        .add()
}

/// The two weaves, named once so the script, the log and the record agree.
#[cfg(feature = "debug")]
const WEAVES: [Staging; 2] = [
    Staging {
        label: "escort",
        mover: PICKET,
        mover_id: PICKET_ID,
        threat_id: PICKET_THREAT_ID,
    },
    Staging {
        label: "capital",
        mover: WARSHIP,
        mover_id: WARSHIP_ID,
        threat_id: WARSHIP_THREAT_ID,
    },
];

/// One staged weave: who flies it and who is pointing at them.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug)]
struct Staging {
    /// The weave's name in the log and in the record.
    label: &'static str,
    /// Catalog id of the hull under test.
    mover: &'static str,
    /// Scenario id of the hull under test.
    mover_id: &'static str,
    /// Scenario id of the hull holding the gun on it.
    threat_id: &'static str,
}

/// One jink leg, as the range can see it from outside the AI: the heading the
/// hull was asked to hold, and what it did about it.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug, Default)]
struct WeaveLeg {
    /// Unit heading the leg is held on, in the threat's frame.
    heading: Vec3,
    /// Where the hull stood relative to the threat when the leg began.
    origin: Vec3,
    /// Where it stood when the leg ended.
    end: Vec3,
    /// How long the leg was held.
    seconds: f32,
}

#[cfg(feature = "debug")]
impl WeaveLeg {
    /// How far the leg actually carried the hull ALONG its own heading.
    fn flown(self) -> Meters {
        Meters::from_engine((self.end - self.origin).dot(self.heading))
    }
}

/// One mover's cycle, assembled a frame at a time by [`watch_the_weaves`].
#[cfg(feature = "debug")]
#[derive(Clone, Debug, Default)]
struct Weave {
    /// Live arm of the hull flying it, sampled inside the cycle.
    arm: Meters,
    /// What the hull's live drive can do to its live mass.
    drive_accel: MetersPerSecondSquared,
    /// How fast it can still swing its nose.
    turn_rate_deg: f32,
    /// Whether the mover reached Engage before the weave.
    engaged: bool,
    /// Whether the cycle has been left again, legs flown.
    closed: bool,
    /// Whether the mover was back in the fight after the cycle.
    back_in_the_fight: bool,
    /// The legs, in the order they were held.
    legs: Vec<WeaveLeg>,
    /// How long the whole cycle took.
    cycle_secs: f32,
}

#[cfg(feature = "debug")]
impl Weave {
    /// The displacement one leg of THIS hull owes: its own arm plus the
    /// authored clearance.
    fn clearance(&self) -> Meters {
        self.arm + LEG_CLEARANCE
    }

    /// The furthest any one leg of the cycle carried the hull along its own
    /// heading.
    fn biggest_leg(&self) -> Meters {
        self.legs
            .iter()
            .map(|leg| leg.flown())
            .fold(Meters::ZERO, Meters::max)
    }
}

/// Both staged cycles, in the order of [`WEAVES`].
#[cfg(feature = "debug")]
#[derive(Resource, Debug, Default, Clone)]
struct WeaveProbe([Weave; 2]);

/// Watch both movers, a frame at a time, and cut their held heading into legs.
///
/// In `Last` so the sample is the settled frame: the behavior state, the
/// maneuver it produced and the hull's own motion all landed in `Update`, and
/// reading any earlier would pair this frame's state with last frame's helm.
///
/// The AI's leg counter is private to `nova_ship`, which is the right place
/// for it - so the range counts legs the way a player sees them, off the
/// heading the hull is holding. Consecutive legs of the jink pattern sit about
/// 93 degrees apart and a single leg's heading drifts with the line of sight
/// alone, so a turn past [`LEG_TURN_DEG`] is a leg boundary and nothing else
/// is.
#[cfg(feature = "debug")]
fn watch_the_weaves(
    time: Res<Time>,
    mut ticks: Local<u32>,
    mut probe: ResMut<WeaveProbe>,
    q_hulls: Query<
        (
            Entity,
            &EntityId,
            &Transform,
            Option<&avian3d::prelude::ComputedCenterOfMass>,
        ),
        With<SpaceshipRootMarker>,
    >,
    q_movers: Query<(
        &AIBehaviorState,
        Option<&Autopilot>,
        Option<&HullRadius>,
        Option<&FlightAuthority>,
    )>,
) {
    let anchor = |id: &str| {
        q_hulls
            .iter()
            .find(|(_, entity_id, _, _)| entity_id.0 == id)
            .map(|(entity, _, transform, com)| (entity, live_structure_anchor(transform, com)))
    };
    let dt = time.delta_secs();
    *ticks += 1;
    // A progress line, because the weave beat waits on two hulls flying real
    // distances and a silent wait is indistinguishable from a hung one.
    let shout = ticks.is_multiple_of(300);

    for (staged, weave) in WEAVES.iter().zip(probe.0.iter_mut()) {
        if weave.closed {
            continue;
        }
        let (Some((mover, mover_at)), Some((_, threat_at))) =
            (anchor(staged.mover_id), anchor(staged.threat_id))
        else {
            continue;
        };
        let Ok((state, autopilot, arm, authority)) = q_movers.get(mover) else {
            continue;
        };
        if shout {
            info!(
                "ai evade: {}: frame {} - {:?} at {:.0} m, legs {}",
                staged.label,
                *ticks,
                state,
                Meters::from_engine(mover_at.distance(threat_at)).get(),
                weave.legs.len(),
            );
        }
        if *state != AIBehaviorState::Evade {
            // Out of the cycle: either not into it yet, or done with it.
            if weave.legs.is_empty() {
                weave.engaged |= *state == AIBehaviorState::Engage;
            } else {
                weave.closed = true;
                weave.back_in_the_fight = *state == AIBehaviorState::Engage;
            }
            continue;
        }
        // Engine boundary: avian and the transform keep world units, so the
        // offset stays in them until a leg is reported in meters.
        let offset = mover_at - threat_at;
        weave.cycle_secs += dt;
        weave.arm = Meters::from_engine(arm.map_or(0.0, |radius| **radius));
        let authority = authority.copied().unwrap_or_default();
        weave.drive_accel = MetersPerSecondSquared::from_engine(authority.linear_acceleration);
        weave.turn_rate_deg = authority.turn_rate.to_degrees();

        let held = autopilot
            .map(|autopilot| autopilot.action)
            .and_then(|action| match action {
                AutopilotAction::MatchVelocity { velocity, .. } => velocity.try_normalize(),
                _ => None,
            });
        let Some(heading) = held else {
            continue;
        };
        match weave.legs.last_mut() {
            Some(leg) if leg.heading.dot(heading) > LEG_TURN_DEG.to_radians().cos() => {
                leg.end = offset;
                leg.seconds += dt;
            }
            _ => weave.legs.push(WeaveLeg {
                heading,
                origin: offset,
                end: offset,
                seconds: 0.0,
            }),
        }
    }
}

/// Every staged hull is present AND has been weighed: a root avian has not
/// measured yet has no centre of mass and no hull radius, so every distance
/// this range reads would be taken from the build origin instead.
#[cfg(feature = "debug")]
fn every_hull_weighed() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        [PICKET_ID, PICKET_THREAT_ID, WARSHIP_ID, WARSHIP_THREAT_ID]
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

/// Both movers have flown a whole jink cycle and left it again.
#[cfg(feature = "debug")]
fn both_weaves_closed() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .get_resource::<WeaveProbe>()
            .is_some_and(|probe| probe.0.iter().all(|weave| weave.closed))
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

#[cfg(feature = "debug")]
fn log_weave(staged: Staging, weave: &Weave) {
    info!(
        "ai evade: {}: {}: arm={:.0} m clearance={:.0} m legs={} cycle={:.1} s \
         drive_accel={:.1} m/s2 turn_rate={:.1} deg/s engaged={} back_in_the_fight={}",
        staged.label,
        staged.mover,
        weave.arm.get(),
        weave.clearance().get(),
        weave.legs.len(),
        weave.cycle_secs,
        weave.drive_accel.get(),
        weave.turn_rate_deg,
        weave.engaged,
        weave.back_in_the_fight,
    );
    for (index, leg) in weave.legs.iter().enumerate() {
        info!(
            "ai evade: {}: leg {} flew {:.0} m of {:.0} m in {:.1} s",
            staged.label,
            index,
            leg.flown().get(),
            weave.clearance().get(),
            leg.seconds,
        );
    }
}

#[cfg(feature = "debug")]
fn weave_payload(staged: Staging, weave: &Weave) -> serde_json::Value {
    serde_json::json!({
        "mover": staged.mover,
        "arm_m": weave.arm.get(),
        "leg_clearance_m": weave.clearance().get(),
        "legs": weave.legs.len(),
        "leg_flown_m": weave.legs.iter().map(|leg| leg.flown().get()).collect::<Vec<_>>(),
        "biggest_leg_m": weave.biggest_leg().get(),
        "leg_secs": weave.legs.iter().map(|leg| leg.seconds).collect::<Vec<_>>(),
        "cycle_secs": weave.cycle_secs,
        "drive_accel_mps2": weave.drive_accel.get(),
        "turn_rate_deg_s": weave.turn_rate_deg,
    })
}

/// The one beat: read both closed cycles, assert what a jink owes the hull
/// flying it, and record the table the ledger quotes.
#[cfg(feature = "debug")]
fn measure_both_weaves(world: &mut World) {
    // CLONED, not taken: `watch_the_weaves` runs every frame for the rest of
    // the run, and a system whose resource went missing is a panic.
    let probe = world.resource::<WeaveProbe>().clone();
    let read: Vec<(Staging, &Weave)> = WEAVES.iter().copied().zip(probe.0.iter()).collect();
    for (staged, weave) in &read {
        log_weave(*staged, weave);
    }

    for (staged, weave) in &read {
        assert!(
            weave.engaged && weave.back_in_the_fight,
            "ai evade: {}: {} went engaged={} and came back out into the fight={} - a nose held \
             on a hull is supposed to break an ENGAGED ship into a weave and hand it back after \
             the legs, not park it on one",
            staged.label,
            staged.mover,
            weave.engaged,
            weave.back_in_the_fight,
        );
        assert!(
            weave.legs.len() == WEAVE_LEGS,
            "ai evade: {}: {} flew {} jink legs, not {} - the cycle is stocked with legs on entry \
             and counted down as each is flown, so a different count means legs were skipped or \
             the heading was read as changing inside one",
            staged.label,
            staged.mover,
            weave.legs.len(),
            WEAVE_LEGS,
        );
    }
    nova_probe::probe_marker(
        world,
        "outcome: a gun held on a hull breaks it into a weave",
        serde_json::json!({
            "movers": WEAVES.map(|staged| staged.mover),
            "legs": read.iter().map(|(_, weave)| weave.legs.len()).collect::<Vec<_>>(),
            "cycle_secs": read.iter().map(|(_, weave)| weave.cycle_secs).collect::<Vec<_>>(),
        }),
    );

    for (staged, weave) in &read {
        let owed = weave.clearance() * LEG_FLOWN_SHARE;
        assert!(
            weave.biggest_leg() >= owed,
            "ai evade: {}: {} flew its whole cycle without one leg reaching the {:.0} m it owes \
             ({:.0} m of arm plus {:.0} m of clearance); its best was {:.0} m - a leg is a \
             DISPLACEMENT sized by the hull, and a cycle that never makes one leaves the ship on \
             the line the gun was already holding",
            staged.label,
            staged.mover,
            weave.clearance().get(),
            weave.arm.get(),
            LEG_CLEARANCE.get(),
            weave.biggest_leg().get(),
        );
    }
    nova_probe::probe_marker(
        world,
        "outcome: a jink leg carries the whole hull off the line",
        serde_json::json!({
            "leg_clearance_m": read.iter().map(|(_, weave)| weave.clearance().get()).collect::<Vec<_>>(),
            "biggest_leg_m": read
                .iter()
                .map(|(_, weave)| weave.biggest_leg().get())
                .collect::<Vec<_>>(),
        }),
    );

    let mut record = serde_json::Map::new();
    for (staged, weave) in &read {
        record.insert(staged.label.to_string(), weave_payload(*staged, weave));
    }
    nova_probe::probe_marker(
        world,
        "outcome: the weave is recorded",
        serde_json::Value::Object(record),
    );
    info!("ai evade: both weaves measured");
}
