//! The duel-cycle main-menu backdrop. Two AI ships fight inside a bounded
//! arena. The surviving ship becomes the target of a scripted torpedo, then
//! the scenario resets.
//!
//! The silhouettes stay distinct at menu scale: one is symmetric with six
//! mounts; the other has an outrigger, a boom, and two mounts.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use super::shared::{backdrop_camera, backdrop_rig, planetoid_glow};
use crate::{
    base_content::{assets::BaseContentAssets, scenarios::SCATTER_SEED, ships},
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
/// Mirrored center loops, shared by each ship's passive routine and its
/// scenario helm order. The order owns translation during combat while the AI
/// keeps acquiring, aiming and firing, so target-relative combat motion cannot
/// carry the pair out of the camera shot.
const VICTOR_PATROL: [Meters3; 3] = [
    Meters3::new(-700.0, 100.0, 500.0),
    Meters3::new(700.0, 150.0, -500.0),
    Meters3::new(0.0, 50.0, 700.0),
];
const RIVAL_PATROL: [Meters3; 3] = [
    Meters3::new(700.0, -50.0, -500.0),
    Meters3::new(-700.0, -100.0, 500.0),
    Meters3::new(0.0, -150.0, -700.0),
];
const VICTOR_PATROL_ORDER: &str = "duel_victor_patrol";
const RIVAL_PATROL_ORDER: &str = "duel_rival_patrol";

/// The two duelists, by the ids every spawn, order, filter and forfeit names.
const VICTOR_ID: &str = "duel_victor";
const RIVAL_ID: &str = "duel_rival";
/// The off-screen battery, and the scatter prefix its arena dressing takes.
const FINISHER_ID: &str = "duel_finisher";
const ROCK_ID_PREFIX: &str = "duel_rock_";
/// The act's timer keys: the finisher clock, the aftermath drift, the spawn
/// batch's flush, and the stall watchdog.
const TIMER_FINISHER_BEAT: &str = "duel_finisher_beat";
const TIMER_RESET: &str = "duel_reset";
const TIMER_RESPAWN: &str = "duel_respawn";
const TIMER_WATCHDOG: &str = "duel_watchdog";

/// The out-of-bounds fail-safe, centered on the ordered fight. The patrol
/// orders keep ordinary combat inside the shot; this shell still resolves an
/// act if a collision, blast or crippled drive throws one actor clear.
const ARENA_ID: &str = "duel_arena";
const ARENA_RADIUS: Meters = Meters(1_800.0);
/// The act is decided ONCE - by a defeat or by a forfeit, whichever lands
/// first. Without the latch a neutralized wreck coasting out of the arena
/// would re-arm the finisher clock mid-flight and put a second siege torpedo
/// in the air, the doubling the 20 s re-arm exists to prevent.
const VAR_DECIDED: &str = "duel_decided";

/// The finisher battery's only section, named so the scripted launch can reach
/// it by id.
const FINISHER_BAY: &str = "siege_bay";

/// The Breaker bay's tube, in build cells: one cell square, two deep.
const BAY_CELLS: Vec3 = Vec3::new(1.0, 1.0, 2.0);

/// Scenario-local bay used only by the scripted menu cycle.
///
/// Its values exceed supported player-buildable content, so the section stays
/// inline instead of appearing in the editor catalog.
///
/// `blast_damage` 2000 over a 450 m radius destroys either target through the
/// 65% transmission rule. `projectile_health` 50000 exceeds the damage six PDC
/// mounts can deal during the approximately 6 s closing window, so the round
/// remains visible under fire and still reaches the target.
fn siege_bay(assets: &BaseContentAssets) -> SectionConfig {
    SectionConfig {
        base: BaseSectionConfig {
            id: FINISHER_BAY.to_string(),
            damage_effects: DamageEffects(vec![DamageEffect::Cracks, DamageEffect::Sparks]),
            name: "Siege Torpedo Bay Section".to_string(),
            description: "A capital-grade siege torpedo battery: slow salvo, \
                          armored ordnance, ship-killing blast."
                .to_string(),
            health: 100.0,
            destroy_sound: Some(assets.section_destroy_sound.clone()),
            collider: Some(SectionCollider::Cuboid { size: BAY_CELLS }),
            link_points: bay_link_points(),
            // Nothing mounts this but the battery, and the battery is not a
            // hull anybody edits.
            hide_in_editor: true,
            animations: bay_muzzle_door(),
        },
        kind: SectionKind::Torpedo(TorpedoSectionConfig {
            render_mesh: Some(assets.torpedo_bay.clone()),
            render_mesh_transform: None,
            projectile_render_mesh: None,
            // The muzzle point on the door plane, and the birth centred one
            // cell back inside the tube, so the round slides out through the
            // open iris.
            spawn_offset: Vec3::NEG_Z * BAY_CELLS.z * 0.5,
            // The launch axis is the spawner's +Y; this turns it onto the
            // section's -Z, the one face `bay_link_points` leaves unlinkable
            // so it can be a muzzle. Without it the tube ejects through its
            // own roof.
            spawn_rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            spawn_recess: BAY_CELLS.z * 0.5,
            fire_rate: 1.0,
            spawner_speed: MetersPerSecond(80.0),
            // Long enough to cross the arena, short enough that a round whose
            // target died mid-flight cleans itself up.
            projectile_lifetime: 60.0,
            arm_time: 0.5,
            arm_distance: Meters(50.0),
            // Dropped, then lit: the bay ejects on a cold charge and the motor
            // catches once the round is clear.
            ignition_delay: 0.6,
            nav_constant: 4.0,
            linear_damping: 0.4,
            blast_radius: Meters(450.0),
            blast_damage: 2000.0,
            blast_effect: None,
            launch_effect: None,
            launch_sound: Some(assets.torpedo_launch_sound.clone()),
            door_sound: Some(assets.torpedo_door_sound.clone()),
            detonation_sound: Some(assets.torpedo_detonation_sound.clone()),
            projectile_health: 50000.0,
            torpedo_type: breaker(),
            ammunition: AmmoCapacity::Limited(6),
            reload: ReloadConfig::Batch(SectionReloadConfig {
                delay: 10.0,
                amount: 1,
            }),
        }),
    }
}

/// **Breaker** - the capital siege warhead this act ends on.
///
/// Half the Serpent's weave amplitude at twice the cruise, so the flight path
/// swings about as wide (the swing scales with both) and the round reads as a
/// committed run rather than a dance. It needs the evasion least: its armour
/// already beats point defense outright.
fn breaker() -> TorpedoTypeConfig {
    TorpedoTypeConfig {
        name: "Breaker".to_string(),
        // Deep crimson, so it reads apart from the duelists' own ordnance.
        tint: Color::srgb(0.75, 0.1, 0.12),
        // The closing window through a point-defense envelope is what makes a
        // siege round hard to stop.
        max_speed: MetersPerSecond(700.0),
        weave_angle: 0.22,
        weave_rate: 1.4,
    }
}

/// The bay's sockets: the breech plate, and one per cell on each flank.
///
/// The muzzle face (-Z) carries NONE: a socket there is an invitation to bolt
/// a plate over the iris the round leaves through.
fn bay_link_points() -> Vec<LinkPoint> {
    let mut points = vec![LinkPoint {
        id: "positive_z".to_string(),
        position: Vec3::Z * (BAY_CELLS.z * 0.5),
        normal: Vec3::Z,
    }];
    for (face, normal) in [
        ("positive_x", Vec3::X),
        ("negative_x", Vec3::NEG_X),
        ("positive_y", Vec3::Y),
        ("negative_y", Vec3::NEG_Y),
    ] {
        for (cell, z) in [("fore", -0.5), ("aft", 0.5)] {
            points.push(LinkPoint {
                id: format!("{face}_{cell}"),
                position: normal * 0.5 + Vec3::Z * z,
                normal,
            });
        }
    }
    points
}

/// The iris over the tube mouth, opened on the launch cue and shut after it.
fn bay_muzzle_door() -> Vec<SectionAnimation> {
    vec![SectionAnimation {
        cue: SectionAnimationCue::MuzzleDoor,
        node_prefix: "door_petal_".to_string(),
        // Past vertical, so the open petals read as a flared crown around the
        // dark throat rather than six posts.
        motion: SectionAnimationMotion::RotateX { degrees: 105.0 },
        open_seconds: 0.25,
        close_seconds: 0.7,
    }]
}

/// What a duelist's bridge is built to take, so the act runs its length.
const DUELIST_BRIDGE_HEALTH: f32 = 500.0;

/// One duelist: a block hull that flies in from off-screen onto an in-frame
/// patrol triangle. The arrival grace keeps its guns quiet on the entrance;
/// the scenario patrol order owns its helm before and during combat. The AI
/// still acquires, aims and fires under an order, but cannot replace the
/// centered route with target-relative combat motion.
fn duelist(
    id: &str,
    name: &str,
    spawn: Meters3,
    patrol: [Meters3; 3],
    design: ShipDesignSource,
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
                // The leash anchors on the same center route as a fallback if
                // an order ends or fails.
                leash: Some(Meters(2_500.0)),
                engage_delay: Some(6.0),
                // Ordered patrols use the ship's arrival standoff, not the AI
                // route's waypoint slack. Press close to each mark so the
                // triangles do not shrink by the default 500 m at each corner.
                arrival_standoff: Some(Meters(100.0)),
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
            design: ships::patched_section(
                design,
                ships::BLOCK_BRIDGE_SECTION_ID,
                ships::section_health(DUELIST_BRIDGE_HEALTH),
            ),
            ..Default::default()
        }),
    }
}

/// One loop of a duelist's scenario-owned center route. `PatrolShip` completes
/// after a lap, and its completion handler issues this same order again.
fn ordered_patrol(order: &str, ship: &str, waypoints: [Meters3; 3]) -> EventActionConfig {
    EventActionConfig::PatrolShip(PatrolShipActionConfig {
        order: order.to_string(),
        ship: ship.to_string(),
        waypoints: waypoints.to_vec(),
    })
}

/// A repeating menu combat cycle. Two ships enter from opposite sides. A ship
/// outside the arena is removed so the remaining AI does not chase it beyond
/// the camera frame. After one ship remains, a scripted bay fires at intervals
/// until the target is destroyed. The scenario switch then clears all spawned
/// ships, debris, and ordnance before the next backdrop.
pub(crate) fn menu_duel(assets: &BaseContentAssets) -> ScenarioConfig {
    let cubemap = assets.cubemap.clone();
    let asteroid_texture = assets.asteroid_texture.clone();
    let mut stage = Vec::new();

    // An OPEN arena: no planetoid - chase lines pin against a central rock
    // and its SOI drags the fight onto it - just a
    // sparse dressing ring well below the fight plane. A crippled loser
    // drifting into the rocks is harmless now - brain-death neutralizes it
    // the moment its computer (or last gun) goes, rocks or no rocks.
    stage.extend(backdrop_rig("duel").objects());
    stage.push(planetoid_glow("duel_lamp"));

    // One scripted siege bay with no controller. The launch action does not
    // need AI or detection range. It
    // sits NEUTRAL until the beat flips it Enemy: an Enemy battery would be
    // a live acquisition target, and the freshly-victorious ship - still in
    // its combat hold, which keeps ANY acquired hostile - would break off
    // its victory lap and drift toward the frame's left edge chasing it
    // (the observed end-of-duel drift). Neutral is invisible to
    // acquisition; the flip lands only for the kill window, when the
    // ordnance must inherit Enemy so the victor's PD engages (and loses).
    stage.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: FINISHER_ID.to_string(),
            name: "Duel Finisher".to_string(),
            position: BATTERY_POS,
            rotation: Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Neutral),
            controller: SpaceshipController::None,
            design: ships::inline_design(vec![SpaceshipSectionConfig {
                id: FINISHER_BAY.to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(siege_bay(assets)),
            }]),
            ..Default::default()
        }),
    });

    // Sparse dressing ring, below the fight plane: without it the arena has
    // no depth parallax.
    let rock_scatter = EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: ROCK_ID_PREFIX.to_string(),
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
                id: ROCK_ID_PREFIX.to_string(),
                name: "Duel Rock".to_string(),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                kind: KIND_ROCK.into(),
                destroy_sound: Some(AssetRef::from("self://sounds/destroy_rock.wav")),
                radius: Meters(10.0),
                texture: asteroid_texture,
                mass: None,
                seed: None,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((Meters(10.0), Meters(30.0))),
        // Scenery, and it has to stay scenery: this ring sits far below a
        // gunfight that owns the frame's light. Stone and dark carbon only -
        // carbon reads as depth without competing with a muzzle flash, and a
        // glossy ice body down there would pull the eye off the duel.
        asteroid_kinds: vec![(KIND_ROCK.into(), 7), (KIND_CARBON.into(), 3)],
        min_separation: None,
    });

    // The victor's TIGHT center loop is both its fallback routine and its
    // ordered route. The order keeps the live fight and the victory lap where
    // the finisher's torpedo will land - mid-shot, not at the frame edge.
    let spawn_victor = EventActionConfig::SpawnScenarioObject(duelist(
        VICTOR_ID,
        "Duel Victor",
        VICTOR_SPAWN,
        VICTOR_PATROL,
        ships::design(ships::BLOCK_GUNSHIP_SHIP_ID),
        // The relation model only makes Player<->Enemy hostile: one duelist
        // must fly the player's colors for AI-vs-AI combat to exist. It also
        // makes the Enemy finisher's ordnance hostile to the winner.
        Some(Allegiance::Player),
    ));
    // The rival's ring mirrors the victor's tight center ring, so the
    // fight's whole geometry - approach, merge, chase - happens in the
    // middle of the frame instead of wandering to the edges.
    let spawn_rival = EventActionConfig::SpawnScenarioObject(duelist(
        RIVAL_ID,
        "Duel Rival",
        RIVAL_SPAWN,
        RIVAL_PATROL,
        // An inline copy, so this actor comes apart below half of its built
        // health rather than holding together at the engine's 5% floor and
        // lingering as a nearly empty hull.
        ships::inline_raider(assets, 0.5),
        None,
    ));
    let victor_patrol = ordered_patrol(VICTOR_PATROL_ORDER, VICTOR_ID, VICTOR_PATROL);
    let rival_patrol = ordered_patrol(RIVAL_PATROL_ORDER, RIVAL_ID, RIVAL_PATROL);

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
                    timer(TIMER_RESPAWN, 0.5),
                    // Stall watchdog: a duelist can end up crippled without
                    // ever counting as DEFEATED: a rival that loses its
                    // flight computer and drifts out of the victor's leash
                    // reach freezes the cycle indefinitely. Every
                    // healthy cycle reloads the scenario long before this
                    // fires - and the reload re-arms it - so the watchdog
                    // only ever catches a wedged state.
                    timer(TIMER_WATCHDOG, 300.0),
                ])
                .collect(),
        },
        // The single spawn site for both duelists.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: false,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: TIMER_RESPAWN.to_string(),
            })],
            // Actions flush in order, so each patrol resolves the ship spawned
            // immediately before it in this batch.
            actions: vec![
                spawn_victor,
                spawn_rival,
                victor_patrol.clone(),
                rival_patrol.clone(),
            ],
        },
        // A patrol order is one lap. Reissue each named order on completion so
        // scenario-owned movement holds for the whole duel and finale.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnShipOrderComplete,
            once: false,
            filters: vec![EventFilterConfig::ShipOrder(ShipOrderFilterConfig {
                order: Some(VICTOR_PATROL_ORDER.to_string()),
                ship: Some(VICTOR_ID.to_string()),
                kind: Some(ShipOrderKind::Patrol),
            })],
            actions: vec![victor_patrol],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnShipOrderComplete,
            once: false,
            filters: vec![EventFilterConfig::ShipOrder(ShipOrderFilterConfig {
                order: Some(RIVAL_PATROL_ORDER.to_string()),
                ship: Some(RIVAL_ID.to_string()),
                kind: Some(ShipOrderKind::Patrol),
            })],
            actions: vec![rival_patrol],
        },
        // Act two, armed by the rival's defeat (destroyed OR neutralized -
        // AI stops shooting a neutralized wreck, so waiting for full
        // destruction could wait forever).
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDefeated,
            once: false,
            filters: vec![entity(RIVAL_ID), number_equals(VAR_DECIDED, 0.0)],
            actions: vec![
                set_number(VAR_DECIDED, 1.0),
                timer(TIMER_FINISHER_BEAT, 4.0),
            ],
        },
        // The forfeit rule, both ways: a duelist that crosses the arena wall
        // is out, and the act resolves as if it had lost. The leaver goes
        // NEUTRAL rather than being despawned - `update_ai_target` re-picks
        // every frame and keeps only HOSTILE candidates, so the hull still in
        // frame drops its lock the moment the other crosses. Neutral also
        // stops the leaver shooting back. Its scenario patrol order remains,
        // so the disqualified hull returns through the shot as a bystander; a
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
                entity_pair(ARENA_ID, RIVAL_ID),
                number_equals(VAR_DECIDED, 0.0),
            ],
            actions: vec![
                set_number(VAR_DECIDED, 1.0),
                forfeit(RIVAL_ID),
                // The rival is out, so the gunship has won: the same beat its
                // defeat would have armed, and the finale plays unchanged.
                timer(TIMER_FINISHER_BEAT, 4.0),
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
                entity_pair(ARENA_ID, VICTOR_ID),
                number_equals(VAR_DECIDED, 0.0),
            ],
            actions: vec![
                set_number(VAR_DECIDED, 1.0),
                forfeit(VICTOR_ID),
                timer(TIMER_RESET, 8.0),
            ],
        },
        // The finisher clock: launch at the winner and re-arm itself, so a
        // miss (or a launch skipped because the victor just died to a wreck
        // collision) retries instead of stalling the cycle. The re-arm is
        // LONGER than the ~16 s flight from the park, so exactly one siege
        // torpedo is ever in the air; a 12 s re-arm puts two of them up at
        // once. Expired keys are removed before dispatch, so the
        // self-restart is legal.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: false,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: TIMER_FINISHER_BEAT.to_string(),
            })],
            actions: vec![
                // Hostile only for the kill window (see the battery's spawn
                // comment); the full reset restores the authored Neutral.
                EventActionConfig::SetAllegiance(SetAllegianceActionConfig {
                    id: FINISHER_ID.to_string(),
                    allegiance: Allegiance::Enemy,
                }),
                EventActionConfig::ForceTorpedoFire(ForceTorpedoFireActionConfig {
                    ship: FINISHER_ID.to_string(),
                    section: FINISHER_BAY.to_string(),
                    target: VICTOR_ID.to_string(),
                }),
                timer(TIMER_FINISHER_BEAT, 20.0),
            ],
        },
        // Act three: stop the finisher clock and let the aftermath drift for
        // a beat - the wrecks stay in shot - then the carousel turns.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDefeated,
            once: false,
            filters: vec![entity(VICTOR_ID)],
            actions: vec![
                EventActionConfig::TimerCancel(TimerCancelActionConfig {
                    key: TIMER_FINISHER_BEAT.to_string(),
                }),
                timer(TIMER_RESET, 8.0),
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
                key: TIMER_RESET.to_string(),
            })],
            actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: super::MENU_WAYSTATION_SCENARIO_ID.to_string(),
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
                key: TIMER_WATCHDOG.to_string(),
            })],
            actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: super::MENU_WAYSTATION_SCENARIO_ID.to_string(),
                linger: false,
                delay: Some(1.0),
            })],
        },
    ];

    ScenarioConfig {
        description: "A gunship and a raider duel; a siege torpedo erases the winner; repeat."
            .to_string(),
        role: ScenarioRole::Backdrop,
        events,
        ..ScenarioConfig::new(
            super::MENU_DUEL_SCENARIO_ID.to_string(),
            "Duel Cycle".to_string(),
            cubemap,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rival_is_a_one_off_half_health_hull() {
        let assets = BaseContentAssets::from_paths();
        let scenario = menu_duel(&assets);
        let rival = scenario
            .events
            .iter()
            .flat_map(|event| &event.actions)
            .find_map(|action| match action {
                EventActionConfig::SpawnScenarioObject(object) if object.base.id == RIVAL_ID => {
                    let ScenarioObjectKind::Spaceship(ship) = &object.kind else {
                        panic!("duel_rival must be a ship");
                    };
                    Some(ship)
                }
                _ => None,
            })
            .expect("the duel must spawn its rival");
        let ShipDesignSource::Inline(design) = &rival.design else {
            panic!("the duel rival must not retune the shared raider prototype");
        };
        assert_eq!(
            design.integrity.collapse_threshold,
            Some(0.5),
            "the set-piece rival must collapse below half of its built health"
        );
    }

    #[test]
    fn both_duelists_hold_center_patrol_orders() {
        let assets = BaseContentAssets::from_paths();
        let scenario = menu_duel(&assets);

        for (ship, order, route) in [
            (VICTOR_ID, VICTOR_PATROL_ORDER, VICTOR_PATROL),
            (RIVAL_ID, RIVAL_PATROL_ORDER, RIVAL_PATROL),
        ] {
            let matching: Vec<&PatrolShipActionConfig> = scenario
                .events
                .iter()
                .flat_map(|event| &event.actions)
                .filter_map(|action| match action {
                    EventActionConfig::PatrolShip(config)
                        if config.ship == ship && config.order == order =>
                    {
                        Some(config)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(
                matching.len(),
                2,
                "{ship} needs one initial center patrol and one completion-loop patrol"
            );
            assert!(
                matching.iter().all(|config| config.waypoints == route),
                "{ship}'s ordered route must keep the authored center loop"
            );

            let repeat_handlers = scenario
                .events
                .iter()
                .filter(|event| {
                    matches!(event.name, EventConfig::OnShipOrderComplete)
                        && event.filters.iter().any(|filter| {
                            matches!(
                                filter,
                                EventFilterConfig::ShipOrder(config)
                                    if config.order.as_deref() == Some(order)
                                        && config.ship.as_deref() == Some(ship)
                                        && config.kind == Some(ShipOrderKind::Patrol)
                            )
                        })
                })
                .count();
            assert_eq!(
                repeat_handlers, 1,
                "{ship}'s completed lap must re-arm only its own patrol"
            );
        }
    }
}
