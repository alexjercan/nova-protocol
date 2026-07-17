//! Production-faithful behavior + layout rig for The Ledger chapter 2's
//! two-part encounter (task 20260717-112630, spike
//! tasks/20260717-111808/SPIKE.md). Loads the ACTUAL shipped
//! `webmods/the-ledger/ledger_ch2.content.ron` and `ledger_ch2b.content.ron`,
//! registers their real handlers the way the loader does, and drives the
//! act machines with the same event infos the engine emits - plus computed
//! GEOMETRY pins over the shipped spawn positions, so the fairness rework
//! cannot silently rot back into the pre-spike shape (mook better-turrets,
//! point-blank spawns, bracketing crossfire, no cover):
//!
//! 1. spawn RANGE: wave one >= 500u out, the heavies >= 800u out - the
//!    approach IS the breathing room;
//! 2. one BEARING per wave: both attackers of a wave arrive inside a
//!    35-degree cone, so one rock can block both guns;
//! 3. loadout discipline: zero better turrets in wave one, exactly one in
//!    wave two;
//! 4. real COVER: invulnerable boulders sit in each wave's threat corridor
//!    (between the Dray Mule and the attack lane), and worst-case 6x
//!    asteroid bodies overlap nothing they must not;
//! 5. the act-SPLIT retry: each part's Defeat handlers requeue THAT part,
//!    part one's Victory chains into part two, part two's into chapter 3.
//!
//! Harness mirrors broadside_assault.rs / gauntlet_course.rs (mod content
//! stays out of the deep core-CI behavior suite; nova_assets unifies the
//! serde feature so this compiles standalone:
//! `cargo test -p nova_assets --test ledger_ch2_encounter`).

use bevy::{ecs::system::RunSystemOnce, math::Vec3, prelude::*};
use bevy_common_systems::prelude::{
    CommandsGameEventExt, EventHandler, GameEventsPlugin, GameObjectives,
};
use nova_events::prelude::{
    OnDestroyedEvent, OnDestroyedEventInfo, OnUpdateEvent, OnUpdateEventInfo,
};
use nova_modding::prelude::Content;
use nova_scenario::prelude::*;

const CH2A_RON: &str = include_str!("../../../webmods/the-ledger/ledger_ch2.content.ron");
const CH2B_RON: &str = include_str!("../../../webmods/the-ledger/ledger_ch2b.content.ron");
const LEDGER_BUNDLE_RON: &str = include_str!("../../../webmods/the-ledger/the-ledger.bundle.ron");

/// Minimum distance (u) from the player spawn to a wave-one hostile spawn.
const WAVE_ONE_MIN_RANGE: f32 = 500.0;
/// Minimum distance (u) from the player spawn to a heavies spawn.
const WAVE_TWO_MIN_RANGE: f32 = 800.0;
/// Maximum angular spread (degrees) between one wave's attack bearings.
const BEARING_SPREAD_MAX_DEG: f32 = 35.0;
/// A rock counts as corridor cover within this distance (u) of the
/// Mule-to-threat axis.
const CORRIDOR_HALF_WIDTH: f32 = 120.0;
/// Clearance (u) required between a worst-case rock body and the Mule's or
/// player's station (a ship is ~4u; this leaves comfortable room).
const STATION_CLEARANCE: f32 = 20.0;

fn scenario_from(ron_str: &str) -> ScenarioConfig {
    let items: Vec<Content> = ron::de::from_str(ron_str).expect("content RON parses");
    items
        .into_iter()
        .find_map(|c| match c {
            Content::Scenario(s) => Some(s),
            Content::Section(_) => None,
        })
        .expect("content contains a Scenario")
}

fn on_start(scenario: &ScenarioConfig) -> &ScenarioEventConfig {
    scenario
        .events
        .iter()
        .find(|e| matches!(e.name, EventConfig::OnStart))
        .expect("has OnStart")
}

fn spawns(event: &ScenarioEventConfig) -> Vec<&ScenarioObjectConfig> {
    event
        .actions
        .iter()
        .filter_map(|a| match a {
            EventActionConfig::SpawnScenarioObject(config) => Some(config),
            _ => None,
        })
        .collect()
}

fn spawn_by_id<'a>(event: &'a ScenarioEventConfig, id: &str) -> &'a ScenarioObjectConfig {
    spawns(event)
        .into_iter()
        .find(|s| s.base.id == id)
        .unwrap_or_else(|| panic!("OnStart spawns '{id}'"))
}

/// The wave's hostile ships: AI-controlled spaceships with no authored
/// allegiance (Neutral bystanders like the Mule author `Some(Neutral)`).
fn hostiles(event: &ScenarioEventConfig) -> Vec<(&str, Vec3, &SpaceshipConfig)> {
    spawns(event)
        .into_iter()
        .filter_map(|s| match &s.kind {
            ScenarioObjectKind::Spaceship(ship)
                if matches!(ship.controller, SpaceshipController::AI(_))
                    && ship.allegiance.is_none() =>
            {
                Some((s.base.id.as_str(), s.base.position, ship))
            }
            _ => None,
        })
        .collect()
}

fn rocks(event: &ScenarioEventConfig) -> Vec<(&str, Vec3, &AsteroidConfig)> {
    spawns(event)
        .into_iter()
        .filter_map(|s| match &s.kind {
            ScenarioObjectKind::Asteroid(rock) => Some((s.base.id.as_str(), s.base.position, rock)),
            _ => None,
        })
        .collect()
}

fn prototype_count(ship: &SpaceshipConfig, prototype: &str) -> usize {
    ship.sections
        .iter()
        .filter(|s| matches!(&s.source, SectionSource::Prototype(p) if p == prototype))
        .count()
}

/// Angle (degrees) between the player-spawn bearings of two positions.
fn bearing_spread_deg(player: Vec3, a: Vec3, b: Vec3) -> f32 {
    (a - player)
        .normalize()
        .dot((b - player).normalize())
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees()
}

/// Distance from `p` to the segment `a`->`b`, plus the normalized progress
/// of the closest point (so "between" is checkable, not just "near").
fn point_to_segment(p: Vec3, a: Vec3, b: Vec3) -> (f32, f32) {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 <= f32::EPSILON {
        return (p.distance(a), 0.0);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    (p.distance(a + ab * t), t)
}

/// The invulnerable rocks sitting in the corridor between the Mule and the
/// wave's mean attack bearing - the cover the fairness rework promises.
fn corridor_cover(event: &ScenarioEventConfig) -> Vec<&str> {
    let mule = spawn_by_id(event, "dray_mule").base.position;
    let wave = hostiles(event);
    let threat = wave.iter().map(|(_, p, _)| *p).sum::<Vec3>() / wave.len() as f32;
    rocks(event)
        .into_iter()
        .filter(|(_, pos, rock)| {
            let (dist, t) = point_to_segment(*pos, mule, threat);
            rock.invulnerable && dist <= CORRIDOR_HALF_WIDTH && t > 0.05 && t < 0.95
        })
        .map(|(id, _, _)| id)
        .collect()
}

// --- app harness (mirrors broadside_assault.rs) -----------------------------

fn slice_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
    app.init_resource::<NovaEventWorld>();
    app.init_resource::<GameObjectives>();
    app.init_resource::<CurrentOutcome>();
    app.add_systems(Update, |mut commands: Commands| {
        commands.fire::<OnUpdateEvent>(OnUpdateEventInfo);
    });
    app
}

fn seed_var(app: &mut App, key: &str, value: f64) {
    app.world_mut()
        .resource_mut::<NovaEventWorld>()
        .insert_variable(key.to_string(), VariableLiteral::Number(value));
}

fn register_non_start_handlers(app: &mut App, scenario: &ScenarioConfig) {
    for event in scenario
        .events
        .iter()
        .filter(|e| !matches!(e.name, EventConfig::OnStart))
    {
        let mut handler = EventHandler::<NovaEventWorld>::from(event.name);
        for filter in event.filters.iter() {
            handler.add_filter(filter.clone());
        }
        for action in event.actions.iter() {
            handler.add_action(action.clone());
        }
        app.world_mut().spawn(handler);
    }
}

fn destroy(app: &mut App, id: &str) {
    let info = OnDestroyedEventInfo {
        id: id.to_string(),
        type_name: "spaceship".to_string(),
    };
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.fire::<OnDestroyedEvent>(info.clone());
        })
        .expect("fire OnDestroyed");
    app.update();
    app.update();
}

fn number_var(app: &App, key: &str) -> Option<f64> {
    match app.world().resource::<NovaEventWorld>().get_variable(key) {
        Some(VariableLiteral::Number(n)) => Some(*n),
        _ => None,
    }
}

fn outcome_kind(app: &App) -> Option<ScenarioOutcomeKind> {
    app.world()
        .resource::<CurrentOutcome>()
        .0
        .as_ref()
        .map(|outcome| outcome.outcome)
}

fn queued_next(app: &App) -> Option<(String, bool)> {
    app.world()
        .resource::<NovaEventWorld>()
        .next_scenario
        .as_ref()
        .map(|next| (next.scenario_id.clone(), next.linger))
}

/// A part's rig with its act machine seeded the way OnStart does; the
/// OnStart wiring itself is pinned structurally in
/// `on_start_seeds_the_act_machine` (rig-supplies-precondition).
fn armed_app(scenario: &ScenarioConfig) -> App {
    let mut app = slice_app();
    register_non_start_handlers(&mut app, scenario);
    seed_var(&mut app, "act", 1.0);
    seed_var(&mut app, "kills", 0.0);
    app
}

// --- geometry pins ----------------------------------------------------------

#[test]
fn wave_one_spawns_far_light_and_on_one_bearing() {
    let scenario = scenario_from(CH2A_RON);
    let start = on_start(&scenario);
    let player = spawn_by_id(start, "player_spaceship").base.position;
    let wave = hostiles(start);
    assert_eq!(wave.len(), 2, "wave one is a pair");

    for (id, pos, ship) in &wave {
        let range = pos.distance(player);
        assert!(
            range >= WAVE_ONE_MIN_RANGE,
            "'{id}' spawns {range:.0}u out (< {WAVE_ONE_MIN_RANGE}u): no approach phase"
        );
        assert_eq!(
            prototype_count(ship, "better_turret_section"),
            0,
            "'{id}': wave-one mooks never carry the better turret (spike F1)"
        );
        assert_eq!(
            prototype_count(ship, "light_turret_section"),
            1,
            "'{id}' carries the light mook gun"
        );
        let SpaceshipController::AI(ai) = &ship.controller else {
            unreachable!("hostiles() filters on AI");
        };
        assert!(
            !ai.patrol.is_empty() && ai.leash.is_some(),
            "'{id}' flies a patrol lane with a leash (aggro stagger + escape valve)"
        );
        assert_leash_covers_spawn(id, ai, player);
    }

    let spread = bearing_spread_deg(player, wave[0].1, wave[1].1);
    assert!(
        spread <= BEARING_SPREAD_MAX_DEG,
        "wave one brackets the player ({spread:.1} degree spread > {BEARING_SPREAD_MAX_DEG}): \
         one rock can no longer block both guns (spike F3)"
    );
}

/// The leash sphere (centered on the patrol centroid, spaceship.rs:330) must
/// cover the player spawn, or the ship yo-yos on the re-engage hysteresis at
/// the arena's edge instead of fighting in it (review R1.1).
fn assert_leash_covers_spawn(id: &str, ai: &AIControllerConfig, player: Vec3) {
    let centroid = ai.patrol.iter().sum::<Vec3>() / ai.patrol.len() as f32;
    let reach = ai.leash.expect("caller asserted a leash");
    let need = centroid.distance(player);
    assert!(
        need <= reach,
        "'{id}': the leash sphere (r{reach} around the patrol centroid) misses the \
         player spawn by {:.0}u - the fight would sit on the break-off edge",
        need - reach
    );
}

#[test]
fn the_heavies_spawn_farther_with_exactly_one_better_gun() {
    let scenario = scenario_from(CH2B_RON);
    let start = on_start(&scenario);
    let player = spawn_by_id(start, "player_spaceship").base.position;
    let wave = hostiles(start);
    assert_eq!(wave.len(), 2, "wave two is a pair");

    let mut better_total = 0;
    for (id, pos, ship) in &wave {
        let range = pos.distance(player);
        assert!(
            range >= WAVE_TWO_MIN_RANGE,
            "'{id}' spawns {range:.0}u out (< {WAVE_TWO_MIN_RANGE}u): the heavies' long \
             approach is the post-checkpoint breather"
        );
        assert_eq!(
            prototype_count(ship, "reinforced_hull_section"),
            1,
            "'{id}' is a reinforced heavy"
        );
        better_total += prototype_count(ship, "better_turret_section");
        let SpaceshipController::AI(ai) = &ship.controller else {
            unreachable!("hostiles() filters on AI");
        };
        assert!(
            !ai.patrol.is_empty() && ai.leash.is_some(),
            "'{id}' flies a patrol lane with a leash"
        );
        assert_leash_covers_spawn(id, ai, player);
    }
    assert_eq!(
        better_total, 1,
        "exactly one better turret in the whole second wave (spike F1)"
    );

    let spread = bearing_spread_deg(player, wave[0].1, wave[1].1);
    assert!(
        spread <= BEARING_SPREAD_MAX_DEG,
        "the heavies bracket the player ({spread:.1} degree spread)"
    );
}

#[test]
fn invulnerable_cover_sits_in_both_threat_corridors() {
    for (part, ron_str) in [("part one", CH2A_RON), ("part two", CH2B_RON)] {
        let scenario = scenario_from(ron_str);
        let start = on_start(&scenario);

        let invulnerable = rocks(start)
            .iter()
            .filter(|(_, _, r)| r.invulnerable)
            .count();
        let chaff = rocks(start)
            .iter()
            .filter(|(_, _, r)| !r.invulnerable)
            .count();
        assert!(
            invulnerable >= 4,
            "{part}: at least four invulnerable boulders anchor the field (found {invulnerable})"
        );
        assert!(
            chaff >= 2,
            "{part}: some destructible chaff keeps the field readable (found {chaff})"
        );

        let cover = corridor_cover(start);
        assert!(
            cover.len() >= 2,
            "{part}: needs >= 2 invulnerable rocks in the Mule-to-threat corridor \
             (within {CORRIDOR_HALF_WIDTH}u of the axis), found {cover:?} - the fight \
             must offer hard cover on the attack lane (spike F4)"
        );
    }
}

#[test]
fn worst_case_rock_bodies_overlap_nothing() {
    for (part, ron_str) in [("part one", CH2A_RON), ("part two", CH2B_RON)] {
        let scenario = scenario_from(ron_str);
        let start = on_start(&scenario);
        let field = rocks(start);

        // Rock vs rock: 6x worst-case bodies must not merge into a wall.
        for i in 0..field.len() {
            for j in (i + 1)..field.len() {
                let (id_a, pos_a, rock_a) = &field[i];
                let (id_b, pos_b, rock_b) = &field[j];
                let worst = (rock_a.radius + rock_b.radius) * ASTEROID_GEOMETRIC_FACTOR_MAX;
                let gap = pos_a.distance(*pos_b) - worst;
                assert!(
                    gap > 0.0,
                    "{part}: rocks '{id_a}' and '{id_b}' can overlap at the 6x factor \
                     (centres {:.0}u apart, worst bodies {worst:.0}u)",
                    pos_a.distance(*pos_b)
                );
            }
        }

        // Rock vs the two stations the fight is anchored on.
        let player = spawn_by_id(start, "player_spaceship").base.position;
        let mule = spawn_by_id(start, "dray_mule").base.position;
        for (id, pos, rock) in &field {
            let body = rock.radius * ASTEROID_GEOMETRIC_FACTOR_MAX;
            for (station, spot) in [("player spawn", player), ("Dray Mule", mule)] {
                let clearance = pos.distance(spot) - body;
                assert!(
                    clearance >= STATION_CLEARANCE,
                    "{part}: rock '{id}' worst-case body ({body:.0}u) leaves {clearance:.0}u \
                     at the {station} (< {STATION_CLEARANCE}u)"
                );
            }
        }
    }
}

#[test]
fn the_mule_sits_off_both_threat_axes() {
    // The escort dies to STRAYS aimed at the player (enemies never target
    // neutrals), and a round that misses keeps flying PAST the player - so
    // the danger corridor is the hostile->player line EXTENDED beyond the
    // player by a weapon range, not just the segment between them. The
    // spawn bearing is a proxy for the approach phase (orbiting fire comes
    // from everywhere); the approach is where one lane carries sustained
    // fire, so that lane is what the layout must keep off the Mule.
    const OVERSHOOT: f32 = 500.0;
    for (part, ron_str) in [("part one", CH2A_RON), ("part two", CH2B_RON)] {
        let scenario = scenario_from(ron_str);
        let start = on_start(&scenario);
        let player = spawn_by_id(start, "player_spaceship").base.position;
        let mule = spawn_by_id(start, "dray_mule").base.position;
        for (id, pos, _) in hostiles(start) {
            let through = player + (player - pos).normalize() * OVERSHOOT;
            let (dist, _) = point_to_segment(mule, pos, through);
            assert!(
                dist >= 60.0,
                "{part}: the Mule sits {dist:.0}u from the '{id}'->player fire lane \
                 (incl. {OVERSHOOT}u overshoot; < 60u) - strays the player dodges \
                 would funnel into her (spike F6)"
            );
        }
    }
}

// --- behavior walks ---------------------------------------------------------

#[test]
fn wave_one_kills_checkpoint_into_the_heavies() {
    let scenario = scenario_from(CH2A_RON);
    let mut app = armed_app(&scenario);

    destroy(&mut app, "magpie_1");
    assert_eq!(number_var(&app, "kills"), Some(1.0));
    assert_eq!(outcome_kind(&app), None, "one kill is not the wave");

    destroy(&mut app, "magpie_2");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "breaking the pair wins part one"
    );
    let (next, linger) = queued_next(&app).expect("the checkpoint queues part two");
    assert_eq!(next, "ledger_ch2b_the_heavies");
    assert!(linger, "the player chooses when to fly into wave two");
}

#[test]
fn wave_one_deaths_retry_wave_one() {
    for casualty in ["player_spaceship", "dray_mule"] {
        let scenario = scenario_from(CH2A_RON);
        let mut app = armed_app(&scenario);

        destroy(&mut app, casualty);
        assert_eq!(
            outcome_kind(&app),
            Some(ScenarioOutcomeKind::Defeat),
            "losing '{casualty}' loses the wave"
        );
        let (next, linger) = queued_next(&app).expect("a retry is queued");
        assert_eq!(next, "ledger_ch2_claim_jumpers", "retry is THIS part");
        assert!(linger);
    }
}

#[test]
fn heavies_kills_clear_the_lane_to_chapter_three() {
    let scenario = scenario_from(CH2B_RON);
    let mut app = armed_app(&scenario);

    destroy(&mut app, "magpie_3");
    assert_eq!(number_var(&app, "kills"), Some(1.0));
    assert_eq!(outcome_kind(&app), None);

    destroy(&mut app, "magpie_4");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "breaking the heavies wins the chapter"
    );
    let (next, linger) = queued_next(&app).expect("victory chains on");
    assert_eq!(next, "ledger_ch3_quiet_channel");
    assert!(linger);
}

#[test]
fn heavies_deaths_retry_the_heavies_only() {
    // THE checkpoint pin: dying to wave two must never send the player
    // back through wave one (spike F7 - full-chapter restarts were the
    // wall).
    for casualty in ["player_spaceship", "dray_mule"] {
        let scenario = scenario_from(CH2B_RON);
        let mut app = armed_app(&scenario);

        destroy(&mut app, casualty);
        assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Defeat));
        let (next, linger) = queued_next(&app).expect("a retry is queued");
        assert_eq!(
            next, "ledger_ch2b_the_heavies",
            "retry is part two, not part one"
        );
        assert!(linger);
    }
}

#[test]
fn deaths_after_the_win_declare_nothing() {
    // A post-victory death (debris under the gold banner) must not flip an
    // earned win (the act-gating lesson); the live-act death tests above
    // are this test's delivery guard.
    for ron_str in [CH2A_RON, CH2B_RON] {
        let scenario = scenario_from(ron_str);
        let mut app = slice_app();
        register_non_start_handlers(&mut app, &scenario);
        seed_var(&mut app, "act", 2.0);
        seed_var(&mut app, "kills", 2.0);

        destroy(&mut app, "player_spaceship");
        assert_eq!(outcome_kind(&app), None, "no Defeat over the earned win");
        assert_eq!(queued_next(&app), None, "no retry queued over the win");
    }
}

// --- structural pins --------------------------------------------------------

#[test]
fn on_start_seeds_the_act_machine() {
    // The behavior rigs seed act/kills by hand; this pins that OnStart
    // actually establishes them (rig-supplies-precondition) and spawns the
    // cast each walk assumes.
    for (part, ron_str, wave) in [
        ("part one", CH2A_RON, ["magpie_1", "magpie_2"]),
        ("part two", CH2B_RON, ["magpie_3", "magpie_4"]),
    ] {
        let scenario = scenario_from(ron_str);
        let start = on_start(&scenario);
        let mut seeded = start.actions.iter().filter_map(|a| match a {
            EventActionConfig::VariableSet(set) => Some(set.key.as_str()),
            _ => None,
        });
        assert!(seeded.any(|k| k == "act"), "{part}: OnStart seeds 'act'");
        let mut seeded = start.actions.iter().filter_map(|a| match a {
            EventActionConfig::VariableSet(set) => Some(set.key.as_str()),
            _ => None,
        });
        assert!(
            seeded.any(|k| k == "kills"),
            "{part}: OnStart seeds 'kills'"
        );

        spawn_by_id(start, "player_spaceship");
        spawn_by_id(start, "dray_mule");
        for id in wave {
            spawn_by_id(start, id);
        }
    }
}

#[test]
fn the_bundle_ships_both_parts_and_the_bump() {
    assert!(
        LEDGER_BUNDLE_RON.contains("ledger_ch2.content.ron")
            && LEDGER_BUNDLE_RON.contains("ledger_ch2b.content.ron"),
        "the bundle lists both chapter-two parts"
    );
    // The durable intent is "bumped PAST the pre-rework 1.0.0", not one
    // frozen literal: sibling tasks legitimately keep bumping (the Auditor
    // cycles took it to 1.3.0 and this exact-version pin went red on
    // master - the sibling-change-leaves-stale-fixture lesson).
    let version = LEDGER_BUNDLE_RON
        .split("version: \"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("the bundle declares a version");
    let parts: Vec<u32> = version
        .split('.')
        .map(|p| p.parse().expect("numeric version parts"))
        .collect();
    assert!(
        parts.as_slice() > [1, 0, 0].as_slice(),
        "the bundle ({version}) is bumped past the pre-rework 1.0.0"
    );
}
