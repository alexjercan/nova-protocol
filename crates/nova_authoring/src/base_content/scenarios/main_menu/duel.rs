//! The duel-cycle main-menu backdrop: an armoured gunship and a salvage
//! raider fight inside a bounded arena, the winner is erased by a siege
//! torpedo, and after a beat two fresh ships fly in.
//!
//! The two hulls are the fleet's LOOK argument: same cube vocabulary, opposite
//! read. The gunship is squared off, symmetric and armoured, with six mounts;
//! the raider is the same tonnage worn down to an outrigger, a scrap boom and
//! two guns bolted where they fitted. A menu visitor should be able to call
//! the fight before a shot lands.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::shared::{backdrop_camera, backdrop_rig, planetoid_glow};
use crate::{
    base_content::{scenarios::SCATTER_SEED, ships},
    scenario_helpers::{entity, entity_pair, number, number_equals, set_number},
};

/// The finisher battery's park: far outside the ~+-2.3 km camera frame AND
/// beyond the winner's leash + turret reach (2.5 km + 2 km) of the arena
/// center, so a victor lunging off a blast hit can never bring its guns onto
/// the battery before the leash walks it home. With the launch SCRIPTED there
/// is no AI envelope to stay inside of.
const BATTERY_POS: Meters3 = Meters3::new(-9_500.0, 0.0, 0.0);
/// Off-screen entrances, one per side, with a little vertical split so the
/// approach lines cross instead of meeting head-on.
const VICTOR_SPAWN: Meters3 = Meters3::new(-4_200.0, 250.0, 1_000.0);
const RIVAL_SPAWN: Meters3 = Meters3::new(4_200.0, -150.0, -1_000.0);

/// The out-of-bounds shell, centered on the fight. A backdrop is a SHOT
/// before it is a fight, and the AI leash cannot keep it in one: `beyond_leash`
/// is overridden while a ship is `recently_damaged`
/// (crates/nova_ship/src/input/ai/behavior.rs), which is exactly the state a
/// running duel holds both hulls in. So the arena, not the leash, is what keeps
/// the act on camera - observed live, a winner sat at the left frame edge
/// firing tracers at a rival that had already left the shot.
///
/// 1,800 m is past the widest frame half-width at the fight's depth (~1,410 m
/// at 16:9, ~1,060 m at 4:3), so a ship reaching the wall is already out of
/// frame; inside the dressing ring's 2,200 m outer edge, so the boundary has
/// something drawn on it; and twice the patrol triangle's ~870 m reach, so an
/// ordinary merge and overshoot never touches it.
const ARENA_ID: &str = "duel_arena";
const ARENA_RADIUS: Meters = Meters(1_800.0);
/// The act is decided ONCE - by a defeat or by a forfeit, whichever lands
/// first. Without the latch a neutralized wreck coasting out of the arena
/// would re-arm the finisher clock mid-flight and put a second siege torpedo
/// in the air, the doubling the 20 s re-arm exists to prevent.
const VAR_DECIDED: &str = "duel_decided";

/// One duelist: a block warship that flies in from off-screen onto an in-frame
/// patrol triangle. The arrival grace holds the entrance (ships spawn in
/// the Engage state and hold on ANY acquired target, so an ungraced spawn
/// would turn and burn from the spawn point instead of flying in); the
/// leash, anchored on the patrol centroid at the frame's center, pulls a
/// wandering hull back. The leash is the SOFT bound only - a ship under fire
/// ignores it - so the arena shell behind it is what actually keeps the act in
/// shot. With no rock and no gravity anywhere in the scene, every chase line
/// through the center is clear - the fight happens IN the middle of the frame,
/// not pinned against a planetoid.
fn duelist(
    id: &str,
    name: &str,
    spawn: Meters3,
    patrol: [Meters3; 3],
    ship: &str,
    allegiance: Option<Allegiance>,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position: spawn,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance,
            controller: SpaceshipController::AI(AIControllerConfig {
                patrol: patrol.to_vec(),
                // The leash anchors on center-hugging patrol centroids, so
                // the fight gravitates to the middle of the frame. Wide
                // enough for real chases, tight enough that the act plays
                // over the same ground every wave.
                leash: Some(Meters(2_500.0)),
                engage_delay: Some(6.0),
                ..Default::default()
            }),
            // Hardened bridges on BOTH duelists: the tight rings make the
            // merge a nose-to-nose joust, and a stock controller dies to the
            // opening burst - which under the brain-death defeat rule would
            // end the act seconds after it starts. The 500 keeps the DOGFIGHT
            // on screen; however the loser finally cripples (guns, computer,
            // or full destruction), the defeat chain fires and the finale
            // plays. The block hulls bury their computers under plate, so
            // this is now belt-and-braces rather than the only thing holding
            // the act up - and the exposed guns are what actually decide it.
            hull: ships::hull(ship),
            modifications: vec![ships::on_section(
                ships::BLOCK_BRIDGE_SECTION_ID,
                vec![SectionModification::SetHealth(500.0)],
            )],
        }),
    }
}

/// A repeating three-act battle behind the menu. Act one: an armoured patrol
/// gunship (Player allegiance) and a salvage raider fly in from opposite sides
/// and dogfight through the open center of the frame; the gun gap - six mounts
/// against two - makes the outcome all but certain. A duelist that leaves the
/// arena shell FORFEITS and the act resolves without it, so the survivor holds
/// the middle of the frame instead of chasing a runner off the edge of the
/// shot. Act two: the rival's defeat (or forfeit) starts a short beat, then the
/// off-screen siege battery is SCRIPTED to launch an
/// armored ship-killing torpedo at the winner - point defense hammers the
/// ordnance and loses - re-firing on a slow clock until one connects. Act
/// three: the aftermath drifts for a beat, then the carousel turns to the
/// next backdrop - the scenario switch is a genuine full reset that clears
/// wrecks, debris and in-flight ordnance.
pub(crate) fn menu_duel(
    cubemap: AssetRef<Image>,
    asteroid_texture: AssetRef<Image>,
) -> ScenarioConfig {
    let mut stage = Vec::new();

    // An OPEN arena: no planetoid (the first cut proved chase lines pin
    // against a central rock and its SOI drags the fight onto it), just a
    // sparse dressing ring well below the fight plane. A crippled loser
    // drifting into the rocks is harmless now - brain-death neutralizes it
    // the moment its computer (or last gun) goes, rocks or no rocks.
    stage.extend(backdrop_rig("duel").objects());
    stage.push(planetoid_glow("duel_lamp"));

    // The finisher: one siege bay, nothing else. No controller - the launch
    // is scripted, so the battery needs no AI and no detection range. It
    // sits NEUTRAL until the beat flips it Enemy: an Enemy battery would be
    // a live acquisition target, and the freshly-victorious ship - still in
    // its combat hold, which keeps ANY acquired hostile - would break off
    // its victory lap and drift toward the frame's left edge chasing it
    // (the observed end-of-duel drift). Neutral is invisible to
    // acquisition; the flip lands only for the kill window, when the
    // ordnance must inherit Enemy so the victor's PD engages (and loses).
    stage.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "duel_finisher".to_string(),
            name: "Duel Finisher".to_string(),
            position: BATTERY_POS,
            rotation: Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Neutral),
            controller: SpaceshipController::None,
            hull: ships::inline_hull(vec![SpaceshipSectionConfig {
                id: "siege_bay".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                source: SectionSource::Prototype("heavy_torpedo_section".to_string()),
                modifications: vec![],
            }]),
            ..Default::default()
        }),
    });

    // Sparse dressing ring, below the fight plane - depth parallax the
    // rockless first cut of this arena visibly missed.
    let rock_scatter = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: "duel_rock_".to_string(),
        count: 12,
        seed: SCATTER_SEED ^ 0x5,
        region: ScatterRegion::Ring {
            center: Meters3::ZERO,
            inner: Meters(1_500.0),
            outer: Meters(2_200.0),
            y_min: Meters(-700.0),
            y_max: Meters(-350.0),
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "duel_rock_".to_string(),
                name: "Duel Rock".to_string(),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                material: None,
                destroy_sound: Some(AssetRef::from("self://sounds/destroy_rock.wav")),
                radius: Meters(10.0),
                texture: asteroid_texture,
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((Meters(10.0), Meters(30.0))),
        min_separation: None,
    });

    // The victor's routine is a TIGHT ring on the frame center: it is the
    // fight's center of gravity while the duel runs (the leash anchors on
    // the patrol centroid), and after the win it parks the victory lap right
    // where the finisher's torpedo will land - mid-shot, not drifting at the
    // frame edge.
    let spawn_victor = EventActionConfig::SpawnScenarioObject(duelist(
        "duel_victor",
        "Duel Victor",
        VICTOR_SPAWN,
        [
            Meters3::new(-700.0, 100.0, 500.0),
            Meters3::new(700.0, 150.0, -500.0),
            Meters3::new(0.0, 50.0, 700.0),
        ],
        ships::BLOCK_GUNSHIP_SHIP_ID,
        // The relation model only makes Player<->Enemy hostile: one duelist
        // must fly the player's colors for AI-vs-AI combat to exist. It also
        // makes the Enemy finisher's ordnance hostile to the winner.
        Some(Allegiance::Player),
    ));
    // The rival's ring mirrors the victor's tight center ring, so the
    // fight's whole geometry - approach, merge, chase - happens in the
    // middle of the frame instead of wandering to the edges.
    let spawn_rival = EventActionConfig::SpawnScenarioObject(duelist(
        "duel_rival",
        "Duel Rival",
        RIVAL_SPAWN,
        [
            Meters3::new(700.0, -50.0, -500.0),
            Meters3::new(-700.0, -100.0, 500.0),
            Meters3::new(0.0, -150.0, -700.0),
        ],
        ships::BLOCK_RAIDER_SHIP_ID,
        None,
    ));

    let timer = |key: &str, seconds: f64| {
        EventActionConfig::TimerStart(TimerStartActionConfig {
            key: key.to_string(),
            seconds: number(seconds),
        })
    };
    let forfeit = |id: &str| {
        EventActionConfig::SetAllegiance(SetAllegianceActionConfig {
            id: id.to_string(),
            allegiance: Allegiance::Neutral,
        })
    };

    let events = vec![
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: stage
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                // The scene poses its own camera: the reference backdrop
                // shot, dead on the arena center the duel fights over.
                .chain([
                    backdrop_camera(Meters3::new(0.0, 570.0, 1_920.0)),
                    rock_scatter,
                    EventActionConfig::CreateScenarioArea(ScenarioAreaConfig {
                        id: ARENA_ID.to_string(),
                        name: "Duel Arena".to_string(),
                        position: Meters3::ZERO,
                        rotation: Quat::IDENTITY,
                        radius: ARENA_RADIUS,
                    }),
                    set_number(VAR_DECIDED, 0.0),
                    timer("duel_respawn", 0.5),
                    // Stall watchdog: a duelist can end up crippled without
                    // ever counting as DEFEATED (observed live: a rival lost
                    // its flight computer, drifted out of the victor's leash
                    // reach, and the cycle sat frozen for 11 minutes). Every
                    // healthy cycle reloads the scenario long before this
                    // fires - and the reload re-arms it - so the watchdog
                    // only ever catches a wedged state.
                    timer("duel_watchdog", 300.0),
                ])
                .collect(),
        },
        // The single spawn site for both duelists.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: false,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "duel_respawn".to_string(),
            })],
            actions: vec![spawn_victor, spawn_rival],
        },
        // Act two, armed by the rival's defeat (destroyed OR neutralized -
        // AI stops shooting a neutralized wreck, so waiting for full
        // destruction could wait forever).
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDefeated,
            once: false,
            filters: vec![entity("duel_rival"), number_equals(VAR_DECIDED, 0.0)],
            actions: vec![
                set_number(VAR_DECIDED, 1.0),
                timer("duel_finisher_beat", 4.0),
            ],
        },
        // The forfeit rule, both ways: a duelist that crosses the arena wall
        // is out, and the act resolves as if it had lost. The leaver goes
        // NEUTRAL rather than being despawned - `update_ai_target` re-picks
        // every frame and keeps only HOSTILE candidates, so the hull still in
        // frame drops its lock the moment the other crosses, its damage memory
        // lapses, and the leash finally walks it home to the patrol centroid
        // at the frame's center. Neutral also stops the leaver shooting back,
        // which is what was overriding the leash. The disqualified ship keeps
        // flying its own routine and drifts back into shot as a bystander; a
        // despawn would pop a ship out of the sky in full view.
        //
        // Both duelists SPAWN outside the arena and fly in, so the entry is
        // always an OnEnter first - a ship that never reaches the frame never
        // forfeits, and the defeat chain covers it.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnExit,
            once: false,
            filters: vec![
                entity_pair(ARENA_ID, "duel_rival"),
                number_equals(VAR_DECIDED, 0.0),
            ],
            actions: vec![
                set_number(VAR_DECIDED, 1.0),
                forfeit("duel_rival"),
                // The rival is out, so the gunship has won: the same beat its
                // defeat would have armed, and the finale plays unchanged.
                timer("duel_finisher_beat", 4.0),
            ],
        },
        // The mirror branch. There is no victor left in frame for the siege
        // torpedo to erase, so act two is skipped and the aftermath runs
        // straight into the hand-off on the usual eight-second drift.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnExit,
            once: false,
            filters: vec![
                entity_pair(ARENA_ID, "duel_victor"),
                number_equals(VAR_DECIDED, 0.0),
            ],
            actions: vec![
                set_number(VAR_DECIDED, 1.0),
                forfeit("duel_victor"),
                timer("duel_reset", 8.0),
            ],
        },
        // The finisher clock: launch at the winner and re-arm itself, so a
        // miss (or a launch skipped because the victor just died to a wreck
        // collision) retries instead of stalling the cycle. The re-arm is
        // LONGER than the ~16 s flight from the park, so exactly one siege
        // torpedo is ever in the air - the first cut re-armed at 12 s and
        // doubled up. Expired keys are removed before dispatch, so the
        // self-restart is legal.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: false,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "duel_finisher_beat".to_string(),
            })],
            actions: vec![
                // Hostile only for the kill window (see the battery's spawn
                // comment); the full reset restores the authored Neutral.
                EventActionConfig::SetAllegiance(SetAllegianceActionConfig {
                    id: "duel_finisher".to_string(),
                    allegiance: Allegiance::Enemy,
                }),
                EventActionConfig::ForceTorpedoFire(ForceTorpedoFireActionConfig {
                    ship: "duel_finisher".to_string(),
                    section: "siege_bay".to_string(),
                    target: "duel_victor".to_string(),
                }),
                timer("duel_finisher_beat", 20.0),
            ],
        },
        // Act three: stop the finisher clock and let the aftermath drift for
        // a beat - the wrecks stay in shot - then the carousel turns.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDefeated,
            once: false,
            filters: vec![entity("duel_victor")],
            actions: vec![
                EventActionConfig::TimerCancel(TimerCancelActionConfig {
                    key: "duel_finisher_beat".to_string(),
                }),
                timer("duel_reset", 8.0),
            ],
        },
        // The hand-off: teardown despawns every scoped entity (wrecks,
        // debris, in-flight ordnance - runtime projectiles are
        // scenario-scoped too) and the next backdrop starts fresh. In its
        // own handler with a short delay: an instant switch consumed in the
        // same flush would discard sibling handlers' queued commands.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: false,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "duel_reset".to_string(),
            })],
            actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: "menu_waystation".to_string(),
                linger: false,
                delay: Some(1.0),
            })],
        },
        // The watchdog's own reset (see OnStart).
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: false,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "duel_watchdog".to_string(),
            })],
            actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: "menu_waystation".to_string(),
                linger: false,
                delay: Some(1.0),
            })],
        },
    ];

    ScenarioConfig {
        description: "A gunship and a raider duel; a siege torpedo erases the winner; repeat."
            .to_string(),
        hidden: true,
        menu_backdrop: true,
        events,
        ..ScenarioConfig::new("menu_duel".to_string(), "Duel Cycle".to_string(), cubemap)
    }
}
