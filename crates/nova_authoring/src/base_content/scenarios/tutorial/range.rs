//! Everything Basic Training puts on the range: the player's ship, the five
//! target hulks, the two drones, the belts, the planetoid the autopilot is
//! flown out to, and the three marks the pattern and the leg home are flown
//! against.
//!
//! Layout provenance: the editor's stock range (`nova_editor::scenario`), the
//! free-flight world every builder plays in. This is that world in metres,
//! with a training picket in place of the builder's own hull.
//!
//! The handbook's practice ranges (`super::super::drills`) are built from this
//! same furniture, on purpose: a lesson sends the player back to the place
//! they first flew, with everything the lesson is not about taken away. What
//! they share beyond the objects is here too: the range's voice, the pairing
//! every helm event is filtered by, and the keybind chips a lesson pulses.

use std::collections::BTreeMap;

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_input::prelude::InputSource;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use crate::{
    base_content::{
        scenarios::{marks::Mark, SCATTER_SEED},
        ships,
    },
    scenario_helpers::prelude::*,
};

// --- the voices --------------------------------------------------------------

/// The range's own voice: the instructor on the comms channel, in Basic
/// Training and in every practice range.
pub(crate) const RANGE_CONTROL: &str = "Range Control";

/// The player, when the card puts words in their mouth.
pub(crate) const PLAYER: &str = "You";

/// The two faces on the channel. Both are the base game's own art.
pub(crate) fn portrait(speaker: &str) -> Option<AssetRef<Image>> {
    let name = match speaker {
        RANGE_CONTROL => "range-control",
        PLAYER => "player",
        _ => return None,
    };
    Some(AssetRef::from(format!("self://portraits/{name}.png")))
}

/// Give every cue on `events` the face of whoever speaks it.
///
/// A pass over the finished events rather than an argument to each line: the
/// speaker already says which face it is, and a scenario that authored the
/// icon by hand keeps it.
pub(crate) fn apply_portraits(events: &mut [ScenarioEventConfig]) {
    for event in events {
        for action in &mut event.actions {
            action.walk_mut(&mut |action| {
                if let EventActionConfig::NarrativeCue(cue) = action {
                    if cue.icon.is_none() {
                        cue.icon = portrait(&cue.speaker);
                    }
                }
            });
        }
    }
}

// --- the player ---------------------------------------------------------------

/// The player's ship: a training picket, and the id every helm event names.
pub(crate) const ID_TRAINER: &str = "trainer";
/// Its callsign, used in objective and banner text.
pub(crate) const TRAINER_NAME: &str = "Trainer Seven";

/// Soft manual-speed cap for the whole card: a player on a first flight stays
/// controllable inside the range, and the pattern is a few hundred metres.
pub(crate) const TRAINER_SPEED_CAP: MetersPerSecond = MetersPerSecond(150.0);

/// The trainer's one gun: the picket's nose mount, by the section id the
/// catalog entry gives it. The input mapping and the range's fire lesson both
/// name it.
pub(crate) const TRAINER_GUN: &str = ships::BLOCK_CLEANUP_TURRET_ID;

/// The trainer's gun mount and flight computer, armoured for the card.
///
/// Losing the gun is the one way the range ends in a loss the player cannot
/// shoot their way out of, so the mount is built to outlast everything the
/// drones can put into it: both drones' whole magazines together
/// (`DRONE_MAGAZINE` rounds each at the shared PDC's per-hit damage) fall
/// short of either figure, so on this range the trainer is disarmed or
/// destroyed only by flying into something.
pub(crate) const TRAINER_GUN_HEALTH: f32 = 1_300.0;
pub(crate) const TRAINER_BRIDGE_HEALTH: f32 = 1_300.0;

/// The trainer on the line at the range origin, facing down the range.
///
/// The helm capabilities are withheld at spawn on the ROOT and handed back one
/// lesson at a time: they apply from the instant the ship is built and only to
/// this spawn. The gun is not withheld - there is no capability for it - so
/// the fire lessons are written to survive a player who shoots early.
pub(crate) fn trainer() -> ScenarioObjectConfig {
    trainer_with(WITHHELD_CAPABILITIES)
}

/// The same trainer with whatever helm this range hands over.
///
/// Basic Training withholds all five and grants them one lesson at a time; a
/// practice range gives back the helm the player already has and takes away
/// only the verb the lesson is about - so the momentum drill is flown without
/// STOP because that IS the drill, not because the player has not met STOP.
pub(crate) fn trainer_with(capabilities: ShipCapabilities) -> ScenarioObjectConfig {
    let mut input_mapping = BTreeMap::new();
    input_mapping.insert(
        TRAINER_GUN.to_string(),
        vec![
            InputSource::Mouse(MouseButton::Left),
            InputSource::Gamepad(GamepadButton::RightTrigger2),
        ],
    );
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_TRAINER.to_string(),
            name: TRAINER_NAME.to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping,
                speed_cap: Some(TRAINER_SPEED_CAP),
            }),
            capabilities,
            design: ships::patched_design(
                ships::BLOCK_PICKET_SHIP_ID,
                [
                    ships::on_section(
                        ships::BLOCK_BRIDGE_SECTION_ID,
                        ships::section_health(TRAINER_BRIDGE_HEALTH),
                    ),
                    ships::on_section(TRAINER_GUN, ships::section_health(TRAINER_GUN_HEALTH)),
                ],
            ),
        }),
    }
}

/// The trainer paired with `id`: the shape a lock, a GOTO arrival, an orbit
/// and an area event all report, the target first.
pub(crate) fn trainer_at(id: impl Into<String>) -> EventFilterConfig {
    entity_pair(id, ID_TRAINER)
}

/// The helm the card teaches, withheld until its lesson. Every capability the
/// player has not been shown is one they can fly the pattern with by accident,
/// so the trainer spawns with all five off. Point defence and docking are left
/// alone: the trainer carries neither a point-defense mount nor a docking port
/// to stand down.
pub(super) const WITHHELD_CAPABILITIES: ShipCapabilities = ShipCapabilities {
    stop_enabled: false,
    rcs_enabled: false,
    lock_enabled: false,
    goto_enabled: false,
    orbit_enabled: false,
    point_defense_enabled: true,
    dock_enabled: true,
};

// --- the targets -------------------------------------------------------------

/// How many hulks stand on the line.
pub(crate) const TARGET_COUNT: usize = 5;

/// The line: five hulks staggered down the range's port side, the first a
/// kilometre past mark ALPHA and the last four kilometres down.
const TARGET_POSITIONS: [Meters3; TARGET_COUNT] = [
    Meters3::new(-600.0, 200.0, -1_700.0),
    Meters3::new(-1_400.0, -250.0, -2_400.0),
    Meters3::new(-700.0, 500.0, -3_000.0),
    Meters3::new(-2_000.0, 150.0, -3_600.0),
    Meters3::new(-950.0, -550.0, -4_300.0),
];

/// The scenario id of the `nth` target, counted from one.
pub(crate) fn target_id(nth: usize) -> String {
    format!("target_{nth}")
}

/// What the `nth` target's HUD chip and Range Control call it.
pub(crate) fn target_label(nth: usize) -> String {
    format!("Target {nth}")
}

/// One inert target: a ship-shaped cross of bare hull, no controller, no
/// pilot, no allegiance. It has never carried a weapon section, so the
/// integrity layer never NEUTRALIZES it either - it is a silhouette that
/// takes damage and comes apart, and nothing else.
pub(crate) fn target_hulk(nth: usize) -> ScenarioObjectConfig {
    let plate = |id: &str, cell: Vec3, prototype: &str| SpaceshipSectionConfig {
        id: id.to_string(),
        position: cell,
        rotation: Quat::IDENTITY,
        source: SectionSource::prototype(prototype),
    };
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: target_id(nth),
            name: target_label(nth),
            position: TARGET_POSITIONS[nth - 1],
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::None,
            design: ships::inline_design(vec![
                plate("spine", Vec3::ZERO, REINFORCED_HULL_SECTION_ID),
                plate("bow", Vec3::new(0.0, 0.0, -1.0), LIGHT_HULL_SECTION_ID),
                plate("stern", Vec3::new(0.0, 0.0, 1.0), LIGHT_HULL_SECTION_ID),
                plate("port", Vec3::new(-1.0, 0.0, 0.0), LIGHT_HULL_SECTION_ID),
                plate("starboard", Vec3::new(1.0, 0.0, 0.0), LIGHT_HULL_SECTION_ID),
            ]),
            ..Default::default()
        }),
    }
}

// --- the drones --------------------------------------------------------------

/// The two range drones: id, name, and where they hold, one off each side of
/// the line's far half.
pub(crate) const DRONES: [(&str, &str, Meters3); 2] = [
    (
        "drone_1",
        "Range Drone 1",
        Meters3::new(2_200.0, -300.0, -3_200.0),
    ),
    (
        "drone_2",
        "Range Drone 2",
        Meters3::new(-2_600.0, 400.0, -3_800.0),
    ),
];

/// The distance a woken drone will chase from its station before it gives up
/// and returns. Long enough to follow a player who runs to the line, and short
/// enough that a drone still under control never reaches the range boundary
/// from its station: only a crippled drone, coasting, ever crosses it.
pub(crate) const DRONE_LEASH: Meters = Meters(3_500.0);

/// Every section of a drone starts at this health: a few rounds of the
/// shared PDC. A drone is a target that moves and answers, not a duel; a
/// burst that connects anywhere takes pieces off it, and the first hit on its
/// mount or its bridge ends it.
pub(crate) const DRONE_SECTION_HEALTH: f32 = 24.0;

/// A drone's gun holds this many rounds and never reloads: a second and a
/// half of fire, enough to put tracers past the player and teach that the
/// range shoots back, and too little to threaten an armoured trainer even if
/// every round lands.
pub(crate) const DRONE_MAGAZINE: u32 = 150;

/// One dormant drone: the same picket the player flies, under AI, spawned
/// NEUTRAL and handicapped for a first fight.
///
/// Neutral is the dormancy. The AI runs its passive routine and never
/// acquires, because acquisition only looks at hostile contacts. The live
/// beat flips the allegiance and the same pilot starts fighting - no
/// controller swap, no second spawn. The handicap is a spawn patch on every
/// section, so the catalog picket the player flies is untouched.
pub(crate) fn drone(id: &str, name: &str, position: Meters3) -> ScenarioObjectConfig {
    let section_patches = ships::picket_section_ids().into_iter().map(|section| {
        let kind = (section == TRAINER_GUN).then(|| {
            SectionKindPatch::Turret(TurretSectionConfigPatch {
                ammunition: Some(AmmoCapacity::Limited(DRONE_MAGAZINE)),
                ..Default::default()
            })
        });
        ships::on_section(
            &section,
            SpaceshipSectionConfigPatch {
                config: SectionConfigPatch {
                    health: Some(DRONE_SECTION_HEALTH),
                    kind,
                },
                ..Default::default()
            },
        )
    });
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Neutral),
            controller: SpaceshipController::AI(AIControllerConfig {
                leash: Some(DRONE_LEASH),
                ..Default::default()
            }),
            design: ships::patched_design(ships::BLOCK_PICKET_SHIP_ID, section_patches),
            ..Default::default()
        }),
    }
}

/// Wake one drone: the allegiance flip that turns a parked picket hostile.
pub(crate) fn wake_drone(id: &str) -> EventActionConfig {
    EventActionConfig::SetAllegiance(SetAllegianceActionConfig {
        id: id.to_string(),
        allegiance: Allegiance::Enemy,
    })
}

/// The range boundary: one sphere over the whole card, from the line to the
/// deep belt. A drone that loses its drive or its flight computer keeps the
/// speed it had, and a player cannot always run it down before it is gone. A
/// drone that crosses the boundary is off the range for good, and the range
/// counts it rather than asking for a chase into the dark.
pub(crate) const ID_RANGE_BOUNDARY: &str = "range_boundary";
const RANGE_BOUNDARY_CENTER: Meters3 = Meters3::new(0.0, 0.0, -2_500.0);
const RANGE_BOUNDARY_RADIUS: Meters = Meters(7_000.0);

/// Raise the boundary. It stays up for the whole card: it is the range's
/// edge, not a mark.
pub(crate) fn raise_range_boundary() -> EventActionConfig {
    EventActionConfig::CreateScenarioArea(ScenarioAreaConfig {
        id: ID_RANGE_BOUNDARY.to_string(),
        name: "Range Boundary".to_string(),
        position: RANGE_BOUNDARY_CENTER,
        rotation: Quat::IDENTITY,
        radius: RANGE_BOUNDARY_RADIUS,
    })
}

/// OnExit of the range boundary by one drone.
pub(crate) fn drone_left_range(id: &str) -> EventFilterConfig {
    entity_pair(ID_RANGE_BOUNDARY, id)
}

/// The flag that one drone has been counted, whichever way it went.
pub(crate) fn drone_down_var(id: &str) -> String {
    format!("{id}_down")
}

// --- the scenery -------------------------------------------------------------

/// The range's one planetoid, off the far corner: the well the autopilot
/// drill flies out to and parks in orbit around, and the range's backdrop.
/// It is scenery to the gun.
///
/// Its mass sets the pull, the reach and the player's clock, and a test pins the
/// first two with the engine's own rules: ORBIT's ring band must contain the
/// point GOTO parks at (the surface plus the arrival standoff), or the verb is
/// asked for somewhere it will not fly.
///
/// The clock is the third, and it is why the mass is what it is rather than the
/// guardrail maximum it used to be. GOTO hands the trainer back parked one
/// standoff off the surface with ORBIT still withheld, so the player is falling
/// while Range Control talks: at the guardrail the rock is under the hull in
/// under five seconds, which is not a lesson. At this mass the fall runs about
/// nine, and the ORBIT card lands inside the first two of them.
pub(crate) const ID_PLANETOID: &str = "range_planetoid";
/// What the planetoid's HUD chip reads.
pub(crate) const PLANETOID_LABEL: &str = "PLANETOID";
const PLANETOID_POS: Meters3 = Meters3::new(-5_600.0, -1_100.0, -3_800.0);
const PLANETOID_RADIUS: Meters = Meters(600.0);
const PLANETOID_MASS: f32 = 10_000.0;
const PLANETOID_SEED: u32 = 20_260_815;

/// The planetoid: indestructible and pinned, because the map is authored
/// against it still being there.
pub(crate) fn planetoid() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ID_PLANETOID.to_string(),
            name: "Range Planetoid".to_string(),
            position: PLANETOID_POS,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Planet(
            PlanetConfig::new(PlanetType::BarrenRock, PLANETOID_RADIUS, PLANETOID_SEED)
                .anchored(PLANETOID_MASS),
        ),
    }
}

/// One scattered belt of the range's two: a box of rocks, drawn from a mix.
struct Belt {
    id_prefix: &'static str,
    name: &'static str,
    count: u32,
    seed: u64,
    min: Meters3,
    max: Meters3,
    radius: (Meters, Meters),
    min_separation: Meters,
}

/// The shallow belt sits to starboard of the line, in reach of the pattern;
/// the deep belt closes the far end of the range past the last target.
const BELTS: [Belt; 2] = [
    Belt {
        id_prefix: "shallow_rock_",
        name: "Shallow Belt Rock",
        count: 30,
        seed: SCATTER_SEED ^ 0x5A11_0B37,
        min: Meters3::new(700.0, -500.0, -4_400.0),
        max: Meters3::new(4_300.0, 500.0, -400.0),
        radius: (Meters(10.0), Meters(30.0)),
        min_separation: Meters(450.0),
    },
    Belt {
        id_prefix: "deep_rock_",
        name: "Deep Belt Rock",
        count: 34,
        seed: SCATTER_SEED ^ 0xDEE9_0C11,
        min: Meters3::new(-2_800.0, -600.0, -8_400.0),
        max: Meters3::new(3_000.0, 600.0, -5_400.0),
        radius: (Meters(15.0), Meters(40.0)),
        min_separation: Meters(550.0),
    },
];

/// Both belts as scatter actions, each reproducible from its own seed.
pub(crate) fn belts(texture: &AssetRef<Image>) -> Vec<EventActionConfig> {
    BELTS
        .iter()
        .map(|belt| {
            EventActionConfig::ScatterObjects(ScatterObjectsConfig {
                id_prefix: belt.id_prefix.to_string(),
                count: belt.count,
                seed: belt.seed,
                region: ScatterRegion::Box {
                    min: belt.min,
                    max: belt.max,
                },
                template: ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: belt.id_prefix.to_string(),
                        name: belt.name.to_string(),
                        position: Meters3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                        kind: KIND_ROCK.to_string(),
                        destroy_sound: Some(AssetRef::from("self://sounds/destroy_rock.wav")),
                        radius: belt.radius.0,
                        texture: texture.clone(),
                        mass: None,
                        invulnerable: false,
                        seed: None,
                        lock_signature: None,
                    }),
                },
                asteroid_radius: Some(belt.radius),
                asteroid_kinds: vec![
                    (KIND_ROCK.to_string(), 10),
                    (KIND_CARBON.to_string(), 3),
                    (KIND_ICE.to_string(), 2),
                    (KIND_METAL.to_string(), 1),
                ],
                min_separation: Some(belt.min_separation),
            })
        })
        .collect()
}

/// The range lights itself: there is no engine light in this game.
///
/// `key` prefixes the rig's object ids, so two scenarios built from this range
/// need not agree about what their lights are called.
pub(crate) fn lights(key: &str) -> Vec<ScenarioObjectConfig> {
    ThreePointRig::around(key, Meters3::new(0.0, 0.0, -2_000.0), 25.0).objects()
}

// --- the pattern's marks -----------------------------------------------------

/// The burn lesson's mark: dead ahead of the line, far enough that the player
/// has to hold the throttle open, close enough that they are not still
/// braking when the next lesson starts. The wide volume is for a first
/// flight; the STOP lesson that follows is what makes it a place.
pub(crate) const MARK_ALPHA: Mark = Mark {
    id: "mark_alpha",
    label: "ALPHA",
    position: Meters3::new(0.0, 0.0, -900.0),
    area: Meters(300.0),
};

/// The thruster lesson's mark: a few hundred metres straight across from
/// ALPHA, so the leg is a real translation at the RCS cap and a nudge rather
/// than a burn. The tight volume is on purpose - the lesson is placing the
/// hull, and a wide sphere would pass a player who merely drifted past.
pub(crate) const MARK_BRAVO: Mark = Mark {
    id: "mark_bravo",
    label: "BRAVO",
    position: Meters3::new(350.0, 0.0, -900.0),
    area: Meters(100.0),
};

/// The leg home's mark: on the line, a few hundred metres off Target 1, so
/// the gun lesson that follows opens inside the PDC's reach. It is a mark
/// and not Target 1 itself because a player who shot Target 1 apart from
/// BRAVO would have nothing to fly home to. The volume is sized for a leg the
/// autopilot flies: GOTO parks an arrival standoff short of the beacon, and
/// the gate must contain that park point.
pub(crate) const MARK_CHARLIE: Mark = Mark {
    id: "mark_charlie",
    label: "CHARLIE",
    position: Meters3::new(0.0, 0.0, -1_500.0),
    area: Meters(700.0),
};
