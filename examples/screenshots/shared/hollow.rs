//! The Rock hollow: the cast, the rock shell and the beats every hollow
//! screenshot drives.
//!
//! The player starts parked on station at the origin, which is where every
//! combat framing in the set is measured from. Included by each hollow producer
//! with `#[path = "shared/hollow.rs"] mod hollow;`. It pulls in `shared/kit.rs`
//! ITSELF, so a producer that includes this must not also include the kit by
//! `#[path]` - two path copies of one file are two distinct modules with two
//! distinct `NearField` types.
//!
//! What it holds:
//!
//! - [`ambush_hollow`]: the fighting set - the player, the raider it locks, two
//!   friendly corvettes, a hostile pair and a friendly torpedo boat.
//! - [`ordnance_hollow`]: the quiet set - player, raider and boat only, for the
//!   torpedo run.
//! - The `debug`-only beat helpers: stance, radar, trigger, station-keeping,
//!   scripted section death and the torpedo salvo.

// Each producer includes the whole module and uses the part its scene needs;
// the unused half is not dead code, it is another scene's tool.
#![allow(
    dead_code,
    reason = "one source, many example targets: what one producer leaves unused another needs, so no single build can fulfil an expectation"
)]

#[path = "../../shared/dev_fixtures/mod.rs"]
mod dev_fixtures;
#[path = "kit.rs"]
mod kit;

use std::collections::BTreeMap;

use bevy::prelude::*;
use nova_input::prelude::InputSource;
use nova_protocol::prelude::*;

/// Scenario id of the player's ship.
pub const PLAYER_ID: &str = "hollow_player";

/// Scenario id of the raider the player locks, shoots and finally blows a
/// section off.
pub const RAIDER_ID: &str = "hollow_raider";
/// Where it appears: dead ahead of the parked player, far enough back that the
/// frame has depth between the two hulls, close enough that the target reads.
pub const RAIDER_POSITION: Meters3 = Meters3::new(0.0, 6.0, -340.0);
/// The raider section the scripted blow takes off - the bridge stands forward
/// and proud on the dorsal deck, so the fragments and the hole are both in
/// frame.
pub const RAIDER_BLOWN_SECTION: &str = "bridge";
/// The section the torpedo beat takes off on the raider's port side, where a
/// blast arriving from above lands - a BACKSTOP, not the damage itself.
///
/// It has to be a section this hull actually carries: the salvage raider's
/// outrigger drive hangs off its port arm, which is the port-most thing on it.
///
/// The blow was written when a Serpent carried 100 blast damage and left a
/// 70-100 health section standing, so the frame needed help to show a hole. A
/// Serpent carries 750 over a 300 m radius now, which is enough to take the
/// whole raider apart in the same tick, root and all. So this usually fires
/// into an already-dead section and warns, harmlessly: the torpedo did the job
/// the blow was there to guarantee. Worth revisiting whether the beat still
/// earns its place - and worth NOT deleting until someone has looked at what
/// the aftermath frame actually captures.
pub const RAIDER_BLAST_SECTION: &str = "drive_outrigger";

/// Scenario id of the friendly torpedo boat - the only hull in the set carrying
/// a launch bay, and the ship the ordnance beats are shot off.
pub const LANCE_ID: &str = "hollow_lance";
/// Where it sits: high and off the raider's far quarter, so the run comes DOWN
/// onto the target - and, the reason for the height, through open sky. The rock
/// shell is 460 m thick in Y, and a torpedo fired across the hollow at the
/// shell's own height flies into a rock: this bearing clears it. It is also what
/// keeps the blast (300 m across) off the player, parked 340 m from the raider.
pub const LANCE_POSITION: Meters3 = Meters3::new(-380.0, 300.0, -560.0);
/// How far short of its target a torpedo detonates: the proximity fuze fires at
/// half the bay's blast radius (`torpedo_section/projectile.rs`), and the
/// standard assault bay is authored at 300 m. The ordnance camera is framed off
/// this, not off the raider - 150 m is a third of the frame at a close camera.
pub const TORPEDO_FUZE_RANGE: Meters = Meters(150.0);
/// How many bays the cleanup leader fixture carries, and so how many torpedoes
/// one salvo is. One flank Serpent tube.
pub const EXPECTED_TORPEDO_COUNT: usize = 1;

/// Scenario id of the boat that keeps its shipped bay, for the frame that runs
/// the two torpedo types against each other.
pub const WEAVER_ID: &str = "hollow_weaver";
/// Scenario id of its twin, the same hull with the other bay in it.
pub const STRAIGHT_ID: &str = "hollow_straight";
/// The shipped bay, and the bay it is swapped for: the two catalog sections
/// that differ in nothing but the torpedo they hold
/// (`base_content/sections/torpedo_bay.rs`).
pub const SERPENT_BAY_SECTION: &str = "torpedo_section";
/// The straight-running bay, the other half of that pair.
pub const LANCE_BAY_SECTION: &str = "lance_torpedo_section";
/// Where the pair sits, and where it shoots: the two ends of a 2.4 km run
/// laid ACROSS the hollow at one height.
///
/// LONG, because this frame's subject is the RUN rather than the launch. The
/// ordnance boat's own post is 530 m off its target, which the drive covers in
/// about two seconds - the whole length of a sheet - so a camera framed there
/// gets the cold drop and the detonation and nothing in between. Two and a
/// half kilometres buys seven seconds of cruise, and the sheet is cut out of
/// the middle of it, with both rounds at cap and neither near its fuze.
///
/// LEVEL and at 360 m, because the run has to be clear of rock and the rock has
/// to be in the picture. The shell is 460 m thick about the hollow's waist, so
/// 360 m is over the top of it; the line passes within 360 m of the hollow's
/// centre, which is inside the shell's own 480 m hole; and the shell therefore
/// stands BEHIND and BELOW the two tracks for the whole sheet, which is what
/// makes a pair of rounds holding station in frame read as moving at all.
pub const TYPES_POSITION: Meters3 = Meters3::new(1_200.0, 360.0, 700.0);
/// The far end of that run: where this set parks its raider.
pub const TYPES_TARGET: Meters3 = Meters3::new(-900.0, 360.0, -500.0);
/// How far apart the two boats stand - one ABOVE the other, on the same
/// bearing.
///
/// Stacked rather than abreast, because the camera rides abeam: a boat set to
/// one side of its partner is that much further from the lens than the other,
/// and the two rounds then arrive on the sheet at different SIZES. Set wide,
/// the straight round read as a distant spark beside a weaving arrow, which is
/// a frame that has already told the reader which of the two to look at.
/// Stacked, both rounds are the same distance from the camera and the same size
/// in the cell, and nothing separates them but the ordnance.
///
/// Seventy meters, because the hull is fifty across: closer than that and the
/// two boats are inside each other, and the set loads with one of them missing.
pub const TYPES_SEPARATION: Meters = Meters(70.0);
/// One bay on each of two boats.
pub const EXPECTED_TYPE_TORPEDO_COUNT: usize = 2;

/// Seconds each AI flight holds fire after it spawns, so the shots are taken of
/// a fight that has settled rather than of four ships still sorting out where
/// they are.
pub const ENGAGE_DELAY: f32 = 3.0;
/// How far an AI ship may stray from its post before it breaks off and comes
/// back. Wider than the standoff range the engage maneuver flies to (1 km), so
/// the fight is not permanently interrupted, tight enough that the hollow keeps
/// its ships instead of watching them leave.
pub const AI_LEASH: Meters = Meters(3_200.0);

/// The fighting set: the player on station, the raider it locks, the live
/// background of four AI craft, the torpedo boat, and the rock shell around
/// all of it.
///
/// The whole cast spawns `OnStart`, so a plain run gets the fight by loading the
/// example - there is no trigger to fly into.
pub fn ambush_hollow(
    game_assets: &GameAssets,
    sections: &GameSections,
    ships: &GameShipDesigns,
) -> ScenarioConfig {
    let player_hull = kit::catalog_ship(ships, "block_gunship");
    let player = ship(
        PLAYER_ID,
        "Player Ship",
        Meters3::ZERO,
        // Square with the world: the radar picks by the CAMERA's look ray
        // (`ActiveLookRay`), which opens down world -Z whatever the hull is
        // doing, and the raider is parked a few degrees off that ray - inside
        // the 18-degree radar cone, and clear of the player's own hull in frame.
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig {
            // Without this the trigger is bound to NOTHING: turret bindings are
            // per-section, snapshotted from this map by section id at spawn
            // (`nova_scenario/src/objects/spaceship.rs`), so an empty map is a
            // ship whose guns no button reaches.
            input_mapping: turret_bindings(sections, &player_hull.sections),
            speed_cap: None,
        }),
        None,
        // The player holds fire through several beats; running dry mid-capture
        // would leave a reload where the tracers should be.
        unlimited_turrets(sections, player_hull.clone()),
    );

    // The lock subject: not AI, because an AI hostile flies to a 1 km
    // standoff and no close framing survives that. It is not dead still either -
    // [`nudge_raider`] gives it a slow drift, so the lock's DST and CLS readouts
    // are of a moving target.
    let raider = ship(
        RAIDER_ID,
        "Raider",
        RAIDER_POSITION,
        // Nose toward the player, turned off square: a hostile bearing down
        // reads better than a hull presenting its flank, and it puts the
        // section the juice beat blows on the camera's side of the ship.
        Quat::from_rotation_y(std::f32::consts::PI - 0.4),
        SpaceshipController::None,
        Some(Allegiance::Enemy),
        dev_fixtures::raider(),
    );

    // The live background: two friendlies working the near flanks, two hostiles
    // across the hollow, all four FLYING a route while the engage grace runs -
    // a fight that opens on four parked hulls reads as a diorama. The grace
    // (`engage_delay`) holds them in `Patrol`, so they are mid-leg and banking
    // when the first shot is taken; leashed so the ring they fly afterwards
    // stays in the set.
    let wingman_a = ship(
        "hollow_wing_a",
        "Wingman",
        Meters3::new(-640.0, 120.0, -440.0),
        Quat::from_rotation_y(0.2),
        fighter(vec![
            Meters3::new(-640.0, 120.0, -440.0),
            Meters3::new(-300.0, 40.0, -960.0),
            Meters3::new(-860.0, -60.0, -700.0),
        ]),
        Some(Allegiance::Player),
        kit::catalog_ship(ships, "block_gunship"),
    );
    let wingman_b = ship(
        "hollow_wing_b",
        "Wingman",
        Meters3::new(620.0, -140.0, -580.0),
        Quat::from_rotation_y(-0.2),
        fighter(vec![
            Meters3::new(620.0, -140.0, -580.0),
            Meters3::new(960.0, 60.0, -1_040.0),
            Meters3::new(400.0, -200.0, -1_100.0),
        ]),
        Some(Allegiance::Player),
        kit::catalog_ship(ships, "block_gunship"),
    );
    let hostile_a = ship(
        "hollow_hostile_a",
        "Raider",
        Meters3::new(-1_500.0, 340.0, -2_300.0),
        Quat::from_rotation_y(3.0),
        fighter(vec![
            Meters3::new(-1_500.0, 340.0, -2_300.0),
            Meters3::new(-700.0, 180.0, -2_900.0),
            Meters3::new(-1_900.0, 60.0, -3_000.0),
        ]),
        None,
        dev_fixtures::raider(),
    );
    let hostile_b = ship(
        "hollow_hostile_b",
        "Raider",
        Meters3::new(1_760.0, -380.0, -2_620.0),
        Quat::from_rotation_y(3.3),
        fighter(vec![
            Meters3::new(1_760.0, -380.0, -2_620.0),
            Meters3::new(900.0, -140.0, -3_200.0),
            Meters3::new(2_100.0, -40.0, -3_300.0),
        ]),
        None,
        dev_fixtures::raider(),
    );

    // The torpedo boat: the cleanup leader fixture, the only small craft in
    // the set with a launch bay. Posed, not AI - the AI's envelope opens at
    // 3x the blast radius and its cadence is a 10-second playtest knob, so a
    // capture that waited for it would be waiting on a coin flip. The script
    // pulls the trigger instead ([`loose_torpedoes`]) and the bay, the
    // projectile, the guidance and the blast are all the production path.
    let lance = ship(
        LANCE_ID,
        "Lance",
        LANCE_POSITION,
        Transform::from_translation(LANCE_POSITION.to_engine())
            .looking_at(RAIDER_POSITION.to_engine(), Vec3::Y)
            .rotation,
        SpaceshipController::None,
        Some(Allegiance::Player),
        dev_fixtures::cleanup_leader(),
    );

    ScenarioConfig {
        description: "A rock hollow, and the ambush waiting in it.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            // The photo rig is authored content rather than an example-side
            // observer swap: scale 1.0 around the origin reproduces the kit's
            // exact key/rim/fill numbers, so the captured frames are unchanged.
            actions: [
                vec![
                    shell().action(game_assets),
                    player,
                    raider,
                    wingman_a,
                    wingman_b,
                    hostile_a,
                    hostile_b,
                    lance,
                ],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "rock_hollow".to_string(),
            "Rock Hollow".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The ordnance set: the same hollow with no unrelated combatants and no live
/// guns, so the only thing moving in frame is the salvo.
pub fn ordnance_hollow(game_assets: &GameAssets, ships: &GameShipDesigns) -> ScenarioConfig {
    let player = ship(
        PLAYER_ID,
        "Player Ship",
        Meters3::ZERO,
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::new(),
            speed_cap: None,
        }),
        None,
        kit::catalog_ship(ships, "block_gunship"),
    );
    let raider = ship(
        RAIDER_ID,
        "Raider",
        RAIDER_POSITION,
        Quat::from_rotation_y(std::f32::consts::PI - 0.4),
        SpaceshipController::None,
        Some(Allegiance::Enemy),
        dev_fixtures::raider(),
    );
    let lance = ship(
        LANCE_ID,
        "Lance",
        LANCE_POSITION,
        Transform::from_translation(LANCE_POSITION.to_engine())
            .looking_at(RAIDER_POSITION.to_engine(), Vec3::Y)
            .rotation,
        SpaceshipController::None,
        Some(Allegiance::Player),
        dev_fixtures::cleanup_leader(),
    );
    let shell = kit::NearField {
        id_prefix: "ordnance_rock_",
        count: 48,
        seed: 40507,
        center: Meters3::ZERO,
        distance: (Meters(480.0), Meters(1_300.0)),
        radius: (Meters(12.0), Meters(32.0)),
        y_spread: Meters(460.0),
    };

    ScenarioConfig {
        description: "The rock hollow with only the ordnance cast in it.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![shell.action(game_assets), player, raider, lance],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "rock_hollow_ordnance".to_string(),
            "Rock Hollow - Ordnance".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The TWO-TYPE set: the ordnance pocket with a SECOND boat beside the first,
/// carrying the other torpedo.
///
/// The two types are one authored difference - a weave angle and a cruise cap
/// (`base_content/sections/torpedo_bay.rs`) - so a frame that shows them one at a
/// time shows nothing: a reader cannot tell a corkscrew from a straight line
/// without the straight line beside it. Two hulls, side by side, firing on the
/// same bearing in the same second, is the only staging where the difference
/// is the ONLY thing that differs.
///
/// Same hull twice, not two classes. The boats are both the cleanup leader
/// fixture, and one of them has its bay prototype swapped - which is the
/// lesson's own sentence ("Two normal bays ship, and only the run-in differs")
/// built rather than asserted.
pub fn torpedo_types_hollow(game_assets: &GameAssets, ships: &GameShipDesigns) -> ScenarioConfig {
    let run = (TYPES_TARGET - TYPES_POSITION).get().normalize_or_zero();
    let across = run.cross(Vec3::Y).normalize_or_zero();
    let apart = Meters3(Vec3::Y * (TYPES_SEPARATION.get() * 0.5));
    // BROADSIDE to the target, so the bay fires ALONG the run.
    //
    // A bay ejects across its own hull (`examples/shared/dev_fixtures/block.rs`
    // mounts the leader's tube on the port flank, turned a quarter turn), and
    // a boat pointed at its target therefore drops the round out sideways and
    // leaves the guidance to haul it round. That turn takes longer than a whole
    // sheet: measured on this set, a round nosed at the target was still 90
    // percent of the way along its EJECTION line two seconds after launch, and
    // the inner boat's round crossed the gap and hit the outer boat before the
    // guidance had bent it anywhere.
    //
    // Turning the hulls a quarter turn puts the tube on the run instead, so
    // both rounds leave on the line they will fly and the sheet is cut from a
    // cruise rather than from a turn. `looking_to(across)` is that quarter
    // turn: a hull's port axis is `up x back`, which for this facing is the run.
    let heading = |_: Meters3| Transform::default().looking_to(across, Vec3::Y).rotation;

    let weaver_at = TYPES_POSITION + apart;
    let weaver = ship(
        WEAVER_ID,
        "Serpent Boat",
        weaver_at,
        heading(weaver_at),
        SpaceshipController::None,
        Some(Allegiance::Player),
        dev_fixtures::cleanup_leader(),
    );
    let straight_at = TYPES_POSITION - apart;
    let straight = ship(
        STRAIGHT_ID,
        "Lance Boat",
        straight_at,
        heading(straight_at),
        SpaceshipController::None,
        Some(Allegiance::Player),
        lance_loaded(dev_fixtures::cleanup_leader()),
    );
    let raider = ship(
        RAIDER_ID,
        "Raider",
        TYPES_TARGET,
        Quat::from_rotation_y(std::f32::consts::PI - 0.4),
        SpaceshipController::None,
        Some(Allegiance::Enemy),
        dev_fixtures::raider(),
    );
    // The player's hull is in the set but not in the frame: the camera rides
    // the salvo a kilometre out. It is here because the game is a game about a
    // player's ship - the HUD, the camera stack and the load gate all key off
    // one - and a set without it is a set that behaves differently from the
    // one the lesson is about.
    let player = ship(
        PLAYER_ID,
        "Player Ship",
        Meters3::ZERO,
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::new(),
            speed_cap: None,
        }),
        None,
        kit::catalog_ship(ships, "block_gunship"),
    );
    // A shell, because a camera riding two rounds that hold station in frame
    // needs something standing still to move against - without it the sheet is
    // two lit streaks on black and nothing in it says 300 m/s.
    //
    // PUSHED WAY BACK, unlike the other sets' shells. Those frame a hull at a
    // hundred meters and want the rock close; this one rides a torpedo through
    // the middle of the hollow, and the frame it needs is about 200 m across -
    // wide enough for a 90 m corkscrew - in which a ten-meter round is already
    // small. Rock anywhere near that range is BIGGER than the subject and
    // brighter, and two cuts of this frame came back as a picture of boulders
    // with ordnance in the gaps. Three kilometres out the same rock is thirty
    // pixels. SPARSE as well as far: a shell dense enough to always have rock
    // in the cell puts a bright clump in the same corner of all twenty of them,
    // which reads as a photograph rather than as a loop.
    let shell = kit::NearField {
        id_prefix: "types_rock_",
        count: 14,
        seed: 40511,
        center: Meters3::ZERO,
        distance: (Meters(3_200.0), Meters(5_200.0)),
        radius: (Meters(40.0), Meters(100.0)),
        y_spread: Meters(2_400.0),
    };

    ScenarioConfig {
        description: "The rock hollow with one boat of each torpedo in it.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![shell.action(game_assets), player, raider, weaver, straight],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "rock_hollow_types".to_string(),
            "Rock Hollow - Torpedo Types".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The same design with every Serpent bay on it swapped for a Lance bay.
///
/// By PROTOTYPE rather than by patching the loaded torpedo: the two bays are
/// both shipped catalog sections, so a swapped id gives the boat the ordnance a
/// player would actually have fitted, tint and cruise cap and weave together,
/// with nothing authored example-side that the game does not already ship.
#[cfg(feature = "debug")]
fn lance_loaded(mut design: ShipDesign) -> ShipDesign {
    let mut swapped = 0usize;
    for section in &mut design.sections {
        let SectionSource::Prototype { id, .. } = &section.source else {
            continue;
        };
        if id != SERPENT_BAY_SECTION {
            continue;
        }
        section.source = SectionSource::prototype(LANCE_BAY_SECTION);
        swapped += 1;
    }
    assert_eq!(
        swapped, EXPECTED_TORPEDO_COUNT,
        "the cleanup leader fixture carries exactly the bays this swap was written for"
    );
    design
}

/// The SOLO set: the player's hull alone in a thinned rock shell.
///
/// For the lessons whose subject is one ship and the instruments drawn around
/// it. The fighting sets put another hull, a torpedo boat and a wall of close
/// rock in every framing, and all three read as clutter when the thing being
/// pointed at is a glow on the player's own shell.
///
/// The shell is thinned and pushed back rather than removed: with no rock at
/// all the hull floats on a star field with no sense of place or scale, and
/// the frame stops looking like the game.
pub fn solo_hollow(game_assets: &GameAssets, ships: &GameShipDesigns) -> ScenarioConfig {
    let player = ship(
        PLAYER_ID,
        "Player Ship",
        Meters3::ZERO,
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::new(),
            speed_cap: None,
        }),
        None,
        kit::catalog_ship(ships, "block_gunship"),
    );
    let shell = kit::NearField {
        id_prefix: "solo_rock_",
        count: 20,
        seed: 40507,
        center: Meters3::ZERO,
        distance: (Meters(700.0), Meters(1_600.0)),
        radius: (Meters(14.0), Meters(38.0)),
        y_spread: Meters(520.0),
    };

    ScenarioConfig {
        description: "The rock hollow with one ship in it.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![shell.action(game_assets), player],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "rock_hollow_solo".to_string(),
            "Rock Hollow - Solo".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The DUEL set: the player armed and the raider parked in front of it, with
/// nothing else in the hollow.
///
/// [`ambush_hollow`] with the background fight taken out. The combat lessons
/// each point at ONE thing - the mounts coming up, a bracket stepping onto a
/// section, a gun that will not bear - and four AI craft working the flanks
/// put tracers and banking hulls over every one of them. What it keeps from
/// the ambush set is the armament: the trigger is bound to the player's guns
/// and their magazines never run out, because a lesson that shows a gun firing
/// cannot cut to a reload.
pub fn duel_hollow(
    game_assets: &GameAssets,
    sections: &GameSections,
    ships: &GameShipDesigns,
) -> ScenarioConfig {
    let player_hull = kit::catalog_ship(ships, "block_gunship");
    let player = ship(
        PLAYER_ID,
        "Player Ship",
        Meters3::ZERO,
        // Square with the world, for the reason `ambush_hollow` gives: the
        // radar picks by the CAMERA's look ray, which opens down world -Z, and
        // the raider is parked a few degrees off it.
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: turret_bindings(sections, &player_hull.sections),
            speed_cap: None,
        }),
        None,
        unlimited_turrets(sections, player_hull.clone()),
    );
    let raider = ship(
        RAIDER_ID,
        "Raider",
        RAIDER_POSITION,
        Quat::from_rotation_y(std::f32::consts::PI - 0.4),
        SpaceshipController::None,
        Some(Allegiance::Enemy),
        dev_fixtures::raider(),
    );

    ScenarioConfig {
        description: "The rock hollow with one armed ship and one target in it.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![shell().action(game_assets), player, raider],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "rock_hollow_duel".to_string(),
            "Rock Hollow - Duel".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// How much clear space the HUNTER set's raider is authored to want between
/// the two skins, instead of the engine's 1 km default.
///
/// The number is a FRAMING decision and nothing else. The flight computer's
/// shape - close while outside the band, circle once inside, nose on the
/// target throughout - is the same at any clearance, but at the shipped
/// kilometre the two hulls cannot share a frame that shows either of them: an
/// eye far enough back to hold both draws a raider forty pixels wide. At 140 m
/// the circle is about 215 m of centre distance, which one wide framing holds
/// with the geometry legible.
///
/// Authored through `standoff_clearance`, which is the supported knob for
/// exactly this ("author what a player should SEE between the two skins" -
/// `nova_scenario::objects::spaceship::AIControllerConfig`), so what the
/// demonstration shows is the production maneuver and not a posed copy of it.
pub const HUNTER_STANDOFF: Meters = Meters(140.0);

/// How far out the HUNTER set's raider starts: about 1.5 km on the player's
/// port bow.
///
/// Far enough outside its own band that the CLOSE is a long run rather than a
/// twitch, and well inside the 4 km detection default so the ship never has to
/// be told to look.
pub const HUNTER_START: Meters3 = Meters3::new(400.0, 140.0, -1_400.0);

/// The HUNTER set: the player's hull on station and ONE live AI hostile
/// flying the real engage maneuver around it.
///
/// The only set in the kit whose subject is an AI ship rather than a posed
/// one. Everything else here poses its second hull on purpose - a maneuvering
/// hostile will not hold a framing - and this set exists because one lesson's
/// whole claim is what the flight computer DOES, so its demonstration cannot
/// be a pose.
///
/// One hostile, not the ambush set's four: four AI craft put three other
/// fights over the one the lesson is about. The player keeps no turret
/// bindings either, so the only thing flying in the pocket is the subject.
pub fn hunter_hollow(
    game_assets: &GameAssets,
    sections: &GameSections,
    ships: &GameShipDesigns,
) -> ScenarioConfig {
    let player_hull = armoured(
        sections,
        kit::catalog_ship(ships, "block_gunship"),
        HUNTER_ARMOUR,
    );
    let player = ship(
        PLAYER_ID,
        "Player Ship",
        Meters3::ZERO,
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig {
            // EMPTY, unlike the fighting sets: this lesson is about what the
            // OTHER ship does, and a player trigger bound to live guns is a
            // second thing in the pocket that can shoot.
            input_mapping: BTreeMap::new(),
            speed_cap: None,
        }),
        None,
        player_hull,
    );
    let raider = ship(
        RAIDER_ID,
        "Raider",
        HUNTER_START,
        Quat::from_rotation_y(std::f32::consts::PI),
        SpaceshipController::AI(AIControllerConfig {
            // A ROUTE, short and local, and it is not decoration: an AI ship
            // authored with an empty patrol station-keeps, and a
            // station-keeping ship that acquires a target shoots from where it
            // is instead of closing. Measured - the first cut of this set gave
            // it no route and it sat at its spawn distance for the whole run,
            // firing and never moving. Two legs either side of the start is
            // enough to have it under way when the grace ends.
            patrol: vec![
                HUNTER_START,
                Meters3::new(
                    HUNTER_START.0.x - 260.0,
                    HUNTER_START.0.y,
                    HUNTER_START.0.z - 180.0,
                ),
            ],
            leash: Some(AI_LEASH),
            engage_delay: Some(1.0),
            standoff_clearance: Some(HUNTER_STANDOFF),
            ..default()
        }),
        Some(Allegiance::Enemy),
        dev_fixtures::raider(),
    );

    // ITS OWN SHELL, pushed out and scaled up, and this is the one thing in
    // the set that is not the standard pocket. The fighting sets' wall starts
    // at 480 m, which is a pocket a run-in crosses in four seconds - and worse,
    // a camera standing far enough back to hold a 500 m run-in stands INSIDE
    // that wall and photographs one rock. This wall starts at 1.1 km, so both
    // the run-in and the eye that watches it are in clear space, with the
    // field where a field belongs: behind them.
    //
    // FEWER rocks as well as farther ones. The first cut of this pocket kept
    // the standard count at the new distance, and sixty-four boulders across
    // the far wall is a busy enough picture that the two ships in front of it
    // stop being the subject.
    let shell = kit::NearField {
        id_prefix: "hunter_rock_",
        count: 36,
        seed: 40507,
        center: Meters3::ZERO,
        distance: (Meters(1_100.0), Meters(2_400.0)),
        radius: (Meters(22.0), Meters(60.0)),
        y_spread: Meters(800.0),
    };

    ScenarioConfig {
        description: "The rock hollow with one hostile flying its engage maneuver.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![shell.action(game_assets), player, raider],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "rock_hollow_hunter".to_string(),
            "Rock Hollow - Hunter".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The gap the HUNTER set's sheet opens at.
///
/// The sheet records the CLOSE, not the circle, and that is a framing
/// decision forced by the clock. Two seconds is twenty cells: at the 215 m
/// band this set settles into, the orbit term is allowed a quarter of the
/// hull's authority (`AI_ORBIT_AUTHORITY_RESERVE`), and the arc it covers in
/// two seconds is a few degrees - twenty cells of a hull that looks parked.
/// The approach is the part of the same maneuver that MOVES: a hundred metres
/// a second and more, straight down the bearing, with the nose already on the
/// target.
///
/// So the walk waits for the raider to fall inside this, and the sheet is the
/// last of its run-in. Wide enough that the two hulls are still a pair on the
/// screen, tight enough that the raider is a ship rather than a dot.
pub const HUNTER_SHEET_GAP: Meters = Meters(500.0);

/// The live centre-to-centre gap between the player and the hostile, or
/// `None` while either of them is missing.
///
/// `try_query` rather than `query`: a predicate holds the world by shared
/// reference, and the caching form needs it mutable.
#[cfg(feature = "debug")]
pub fn hunter_gap(world: &World) -> Option<Meters> {
    let mut ships = world.try_query::<(&GlobalTransform, &EntityId)>()?;
    let mut player = None;
    let mut raider = None;
    for (transform, id) in ships.iter(world) {
        match id.0.as_str() {
            PLAYER_ID => player = Some(transform.translation()),
            RAIDER_ID => raider = Some(transform.translation()),
            _ => {}
        }
    }
    Some(Meters::from_engine(player?.distance(raider?)))
}

/// Advance once the hostile has closed to the gap the sheet opens at.
#[cfg(feature = "debug")]
pub fn the_hunter_has_closed() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        hunter_gap(world).is_some_and(|gap| gap.get() <= HUNTER_SHEET_GAP.get())
    })
}

/// What to print when the close never happened: the gap itself, so a stall
/// says whether the hostile was still coming, parked wide, or gone.
#[cfg(feature = "debug")]
pub fn hunter_diagnosis(world: &World) -> String {
    match hunter_gap(world) {
        Some(gap) => format!(
            "the hostile is {:.0} m from the player, and the sheet opens at {:.0} m",
            gap.get(),
            HUNTER_SHEET_GAP.get()
        ),
        None => "one of the two hulls is no longer in the pocket".to_string(),
    }
}

/// The FLYING set: the player's hull alone in the standard shell, for the
/// lessons whose subject is the ship actually moving.
///
/// The shell is the fighting sets' own ([`shell`]) rather than the solo set's
/// thinned one, and that is the whole difference: a lesson about momentum, a
/// braking order or a thruster nudge is READ off the background going past,
/// and rocks pushed out to 1600 m barely move in the two seconds a sheet
/// lasts. The pocket is 480 m of clear space, which is more than a burn
/// covers before the sheet closes.
pub fn flight_hollow(game_assets: &GameAssets, ships: &GameShipDesigns) -> ScenarioConfig {
    let player = ship(
        PLAYER_ID,
        "Player Ship",
        Meters3::ZERO,
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::new(),
            speed_cap: None,
        }),
        None,
        kit::catalog_ship(ships, "block_gunship"),
    );

    ScenarioConfig {
        description: "The rock hollow with one ship flying in it.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![shell().action(game_assets), player],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "rock_hollow_flight".to_string(),
            "Rock Hollow - Flight".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The hollow itself - and it is a HOLLOW: the field starts outside the raider's
/// station (340 m) with room to spare, so the pocket the fight happens in is
/// clear and the rocks read as the wall around it. Tried tighter (280 m):
/// rocks land on the raider, every close framing has one in front of the
/// subject, and a torpedo run into it hits stone.
fn shell() -> kit::NearField {
    kit::NearField {
        id_prefix: "hollow_rock_",
        count: 48,
        seed: 40507,
        center: Meters3::ZERO,
        distance: (Meters(480.0), Meters(1_300.0)),
        radius: (Meters(12.0), Meters(32.0)),
        y_spread: Meters(460.0),
    }
}

/// One posed ship in the set.
pub fn ship(
    id: &str,
    name: &str,
    position: Meters3,
    rotation: Quat,
    controller: SpaceshipController,
    allegiance: Option<Allegiance>,
    hull: ShipDesign,
) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller,
            allegiance,
            design: ShipDesignSource::Inline(hull),
            ..default()
        }),
    })
}

/// A fighting AI ship's routine: fly `patrol` until the engage grace expires,
/// then fight, and come back when the fight drags it past the leash.
///
/// The route is what makes the set move before the first shot: the grace holds
/// the ship in `Patrol`, which flies the waypoint loop through the real GOTO
/// autopilot instead of station-keeping.
pub fn fighter(patrol: Vec<Meters3>) -> SpaceshipController {
    SpaceshipController::AI(AIControllerConfig {
        patrol,
        leash: Some(AI_LEASH),
        engage_delay: Some(ENGAGE_DELAY),
        ..default()
    })
}

/// How much of its own health the HUNTER set's player hull is given.
///
/// The set is the only one in the kit that is SHOT AT for its whole run: the
/// hostile is live, it holds a band 140 m off the skin, and the walk then
/// waits there for as long as the maneuver takes. At stock health that ended
/// the same way every time - a controller node gone, the structure severed,
/// the AI reporting `maneuver complete, disengaging` over a wreck, and the
/// sheet never opening.
///
/// Health is the right knob because it is the only thing in the set nothing
/// in frame reads: the screen is cinematic, so no bar, chip or marker shows a
/// number that this changes. The maneuver, the guns, the rounds and the
/// impacts are all the production path.
pub const HUNTER_ARMOUR: f32 = 1_000.0;

/// The same hull with every section's health multiplied, so a set that is shot
/// at for its whole run still has its subject at the end of it.
///
/// A section is rebuilt as an INLINE copy of the resolved prototype, the way
/// [`unlimited_turrets`] does it: a prototype reference carries no overrides,
/// so the only way to change a number on one section is to author it.
pub fn armoured(sections: &GameSections, mut hull: ShipDesign, factor: f32) -> ShipDesign {
    hull.sections = hull
        .sections
        .into_iter()
        .map(|mut section| {
            let SectionSource::Prototype { id: prototype, .. } = &section.source else {
                return section;
            };
            let Some(resolved) = sections.get_section(prototype) else {
                return section;
            };
            let mut tougher = resolved.clone();
            tougher.base.health *= factor;
            section.source = SectionSource::Inline(tougher);
            section
        })
        .collect();
    hull
}

/// The same hull with every turret rebuilt without a magazine, so a capture that
/// holds fire never cuts to a reload.
///
/// A prototype section resolves to an inline copy of the catalog entry: the
/// magazine lives in the section config, so a rig that wants unlimited fire has
/// to author the gun rather than reference it. The hull is rewritten in place,
/// so it keeps the cladding it came with.
pub fn unlimited_turrets(sections: &GameSections, mut hull: ShipDesign) -> ShipDesign {
    hull.sections = hull
        .sections
        .into_iter()
        .map(|mut section| {
            let SectionSource::Prototype { id: prototype, .. } = &section.source else {
                return section;
            };
            let Some(resolved) = sections.get_section(prototype) else {
                return section;
            };
            if matches!(resolved.kind, SectionKind::Turret(_)) {
                section.source = SectionSource::Inline(resolved.clone().without_magazine());
            }
            section
        })
        .collect();
    hull
}

/// Bind every turret section of a built hull to the trigger, the way the
/// shipped scenarios do (`shakedown_run` maps its two corvette turret cubes to
/// `Mouse(Left)` + `Gamepad(RightTrigger2)`).
///
/// Read off the BUILT hull rather than typed out, for the same reason
/// [`kit::catalog_ship`] is: the ids ARE the layout, and a hand-listed pair goes
/// stale the moment a hull gains a gun. The map is keyed by INSTANCE id
/// (`nova_scenario` snapshots bindings by section id at spawn), which is the id
/// the assembly gave the mount.
///
/// This used to walk the section CATALOG and strip a `<hull>_` prefix off every
/// turret prototype. Every craft mounts the one shared PDC now, whose id
/// carries no hull prefix, so that filter matched nothing and handed back an
/// empty map - a ship whose guns no button reaches, silently.
pub fn turret_bindings(
    sections: &GameSections,
    hull: &[SpaceshipSectionConfig],
) -> BTreeMap<String, Vec<InputSource>> {
    hull.iter()
        .filter(|section| {
            let SectionSource::Prototype { id: prototype, .. } = &section.source else {
                return false;
            };
            sections
                .get_section(prototype)
                .is_some_and(|section| matches!(section.kind, SectionKind::Turret(_)))
        })
        .map(|section| {
            let id = section.id.as_str();
            (
                id.to_string(),
                vec![
                    MouseButton::Left.into(),
                    GamepadButton::RightTrigger2.into(),
                ],
            )
        })
        .collect()
}

/// Present while the scripted run holds the ship on the station every combat
/// framing is measured from.
#[cfg(feature = "debug")]
#[derive(Resource)]
pub struct HoldStation;

/// Put the HUD on (the contextual rules decide what is actually in shot) and
/// take the fps/version status bar back out.
///
/// The bar is not part of the instrument set a shot is showing: its version
/// item names the commit the debug build came from, so every re-shoot bakes a
/// different hash into the image, and its fps item puts the capture rig's
/// cadence on the page. Dropped here rather than at each call site, so a
/// producer cannot ship the bar by forgetting a line.
#[cfg(feature = "debug")]
pub fn hud_instrument(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::On;
    }
    hide_status_bar(world);
}

/// Clean the screen, for the beats whose camera has left the player's ship.
#[cfg(feature = "debug")]
pub fn hud_cinematic(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::Cinematic;
    }
}

/// Pin the camera for a framing the follow camera does not give.
#[cfg(feature = "debug")]
pub fn pose(world: &mut World, position: Meters3, look_at: Meters3) {
    pose_camera(world, position, look_at);
}

/// Start holding station: from here on the scripted run keeps the ship exactly
/// where the combat set was measured from, and the camera looks the way the
/// pinned hull does.
///
/// Re-seeding the mouse rig is not optional. The rig carries whatever attitude
/// the ship was last flown at (`camera/handback.rs`), so pinning the hull square
/// without re-seeding leaves the camera parked on the wrong side of the ship,
/// filming the combat act over its shoulder from in front.
#[cfg(feature = "debug")]
pub fn hold_station(world: &mut World) {
    world.insert_resource(HoldStation);
    let rigs: Vec<Entity> = world
        .query_filtered::<Entity, With<PointRotationOutput>>()
        .iter(world)
        .collect();
    for rig in rigs {
        world.entity_mut(rig).insert((
            PointRotation {
                initial_rotation: Quat::IDENTITY,
            },
            PointRotationOutput(Quat::IDENTITY),
        ));
    }
}

/// Hold the player at the hollow's origin, for the combat beats of a scripted
/// run.
///
/// Not cosmetic, and not the STOP autopilot: the set's geometry is measured from
/// a player at the origin, and the radar picks the body nearest the AIM RAY
/// (`crates/nova_gameplay/src/input/targeting/radar.rs`), so a player a few
/// hundred meters off station swings the parked raider off the ray and latches
/// a hostile two kilometers out instead.
#[cfg(feature = "debug")]
pub fn pin_player(
    mut player: Query<
        (
            &mut Transform,
            &mut avian3d::prelude::LinearVelocity,
            &mut avian3d::prelude::AngularVelocity,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    for (mut transform, mut linear, mut angular) in &mut player {
        transform.translation = Vec3::ZERO;
        transform.rotation = Quat::IDENTITY;
        linear.0 = Vec3::ZERO;
        angular.0 = Vec3::ZERO;
    }
}

/// Set the raider drifting: slow, and across the line of sight rather than
/// along it, so it stays on the aim ray the radar picks by while the lock's
/// distance and closing-speed readouts have something to say.
#[cfg(feature = "debug")]
pub fn nudge_raider(world: &mut World) {
    let Some(raider) = raider_root(world) else {
        warn!("hollow: no raider to nudge");
        return;
    };
    if let Some(mut velocity) = world
        .entity_mut(raider)
        .get_mut::<avian3d::prelude::LinearVelocity>()
    {
        velocity.0 = Vec3::new(0.35, 0.12, -0.25);
    }
}

/// Hold the radar gesture. Which slot it latches depends on the stance.
#[cfg(feature = "debug")]
pub fn hold_radar(world: &mut World) {
    press_action("radar_hold")(world);
}

/// Release the radar gesture.
#[cfg(feature = "debug")]
pub fn release_radar(world: &mut World) {
    release_action("radar_hold")(world);
}

/// Raise the weapons, switching the radar from the nav slot to combat.
#[cfg(feature = "debug")]
pub fn raise_stance(world: &mut World) {
    press_action("combat_stance")(world);
}

/// Hold the trigger (LMB) so the player's turret is firing in the combat shots.
#[cfg(feature = "debug")]
pub fn open_fire(world: &mut World) {
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
}

/// Blow one hull section off the raider through the production damage path -
/// the same `HealthApplyDamage` a bullet delivers, so the shot is of the real
/// destruction, not of a prop.
#[cfg(feature = "debug")]
pub fn blow_raider_section(world: &mut World, section: &str) {
    let Some(node) = raider_section_health(world, section) else {
        warn!("hollow: no health node under section '{section}' to blow");
        return;
    };
    world.trigger(HealthApplyDamage {
        entity: node,
        source: None,
        amount: 1.0e6,
    });
    info!("hollow: blew '{section}' off the raider");
}

/// The player's ship root.
#[cfg(feature = "debug")]
pub fn player_root(world: &mut World) -> Option<Entity> {
    let mut query =
        world.query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>();
    query.iter(world).next()
}

/// The player's ship root, from a read-only world (what a predicate gets).
#[cfg(feature = "debug")]
pub fn player_root_ref(world: &World) -> Option<Entity> {
    world
        .try_query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()?
        .iter(world)
        .next()
}

/// The raider's ship root.
#[cfg(feature = "debug")]
pub fn raider_root(world: &mut World) -> Option<Entity> {
    ship_by_id(world, RAIDER_ID)
}

/// Where the raider actually is right now; its spawn point if it has gone.
#[cfg(feature = "debug")]
pub fn raider_position(world: &mut World) -> Meters3 {
    raider_root(world)
        .and_then(|raider| world.get::<GlobalTransform>(raider))
        .map(|transform| Meters3::from_engine(transform.translation()))
        .unwrap_or(RAIDER_POSITION)
}

/// Advance once the raider is in the world - it is the subject of every close
/// beat, so a set that came up without it aborts here by name.
#[cfg(feature = "debug")]
pub fn raider_present() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&EntityId, With<SpaceshipRootMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|id| id.0 == RAIDER_ID))
    })
}

/// The ship root carrying scenario id `id`.
#[cfg(feature = "debug")]
pub fn ship_by_id(world: &mut World, id: &str) -> Option<Entity> {
    let mut query = world.query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>();
    query
        .iter(world)
        .find(|(_, live)| live.0 == id)
        .map(|(entity, _)| entity)
}

/// The first entity carrying scenario id `id`.
#[cfg(feature = "debug")]
pub fn entity_by_id(world: &mut World, id: &str) -> Option<Entity> {
    let mut query = world.query::<(Entity, &EntityId)>();
    query
        .iter(world)
        .find(|(_, live)| live.0 == id)
        .map(|(entity, _)| entity)
}

/// One of the raider's section entities, by prototype id. Picked BY SHIP: the
/// corvettes in the set share section ids, as every shipped multi-ship scenario
/// does.
#[cfg(feature = "debug")]
pub fn raider_section(world: &mut World, section: &str) -> Option<Entity> {
    let raider = raider_root(world)?;
    let mut query = world.query_filtered::<(Entity, &EntityId), With<SectionMarker>>();
    let candidates: Vec<Entity> = query
        .iter(world)
        .filter(|(_, id)| id.0 == section)
        .map(|(entity, _)| entity)
        .collect();
    candidates
        .into_iter()
        .find(|&entity| under(world, entity, raider))
}

/// The `Health` node of one of the raider's sections: the health lives on the
/// section entity or on one of its children.
#[cfg(feature = "debug")]
pub fn raider_section_health(world: &mut World, section: &str) -> Option<Entity> {
    let section = raider_section(world, section)?;
    if world.get::<Health>(section).is_some() {
        return Some(section);
    }
    let children: Vec<Entity> = world
        .get::<Children>(section)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    children
        .into_iter()
        .find(|&child| world.get::<Health>(child).is_some())
}

/// Whether `entity` sits under `root` in the hierarchy.
#[cfg(feature = "debug")]
pub fn under(world: &World, entity: Entity, root: Entity) -> bool {
    let mut current = entity;
    for _ in 0..8 {
        match world.get::<ChildOf>(current) {
            Some(parent) if parent.parent() == root => return true,
            Some(parent) => current = parent.parent(),
            None => return false,
        }
    }
    false
}

/// Pull the torpedo boat's triggers: every bay on the lance, fired at once, so
/// the beat is a salvo rather than a single round.
///
/// The bays are the ship root's own children (which is how the AI's launch
/// system finds them), and writing [`TorpedoSectionInput`] is exactly what the
/// player's trigger observer and the AI's envelope do - from here on the launch
/// is the production path.
#[cfg(feature = "debug")]
pub fn loose_torpedoes(world: &mut World) {
    loose_torpedoes_from(world, LANCE_ID);
}

/// The same trigger, on a boat named by its scenario id.
///
/// The two-type set fires two boats in the SAME beat, so each one is pulled by
/// id: a salvo that left one tube a beat before the other is a picture of two
/// rounds at different points of the same run, which is the one reading this
/// frame must not give.
#[cfg(feature = "debug")]
pub fn loose_torpedoes_from(world: &mut World, id: &str) {
    let Some(boat) = ship_by_id(world, id) else {
        warn!("hollow: no torpedo boat `{id}` to fire");
        return;
    };
    let bays: Vec<Entity> = world
        .query_filtered::<(Entity, &ChildOf), With<TorpedoSectionMarker>>()
        .iter(world)
        .filter(|(_, parent)| parent.parent() == boat)
        .map(|(bay, _)| bay)
        .collect();
    if bays.is_empty() {
        warn!("hollow: the torpedo boat `{id}` has no bays");
        return;
    }
    for bay in &bays {
        if let Some(mut input) = world.entity_mut(*bay).get_mut::<TorpedoSectionInput>() {
            **input = true;
        }
    }
    info!("hollow: {} torpedo bay(s) firing on `{id}`", bays.len());
}

/// Commit the salvo to the raider and drop the trigger.
///
/// A torpedo's target is decided exactly once, right after launch: the player
/// commits from the crosshair lock and the AI from its own `AITarget`
/// (`input/player/intent.rs`, `input/ai/torpedo.rs`), both by inserting
/// [`TorpedoTargetChosen`] and a [`TorpedoTargetEntity`] on the fresh
/// projectile. The boat is neither, so the script does that one write and the
/// guidance, arming, fuze and blast run themselves. Releasing the bays here is
/// what keeps it to one salvo - the bays would otherwise relaunch on their own
/// fire-rate clock.
#[cfg(feature = "debug")]
pub fn commit_torpedoes(world: &mut World) {
    commit_torpedo_salvo(world, EXPECTED_TORPEDO_COUNT);
}

/// The same commit, for a set whose salvo is not one round.
#[cfg(feature = "debug")]
pub fn commit_torpedo_salvo(world: &mut World, expected: usize) {
    let Some(raider) = raider_root(world) else {
        warn!("hollow: no raider to commit the salvo to");
        return;
    };
    let bays: Vec<Entity> = world
        .query_filtered::<Entity, With<TorpedoSectionMarker>>()
        .iter(world)
        .collect();
    for bay in bays {
        if let Some(mut input) = world.entity_mut(bay).get_mut::<TorpedoSectionInput>() {
            **input = false;
        }
    }
    let torpedoes: Vec<Entity> = world
        .query_filtered::<Entity, (With<TorpedoProjectileMarker>, Without<TorpedoTargetChosen>)>()
        .iter(world)
        .collect();
    assert_eq!(
        torpedoes.len(),
        expected,
        "hollow: the ordnance set must commit the complete salvo"
    );
    for torpedo in &torpedoes {
        world
            .entity_mut(*torpedo)
            .insert((TorpedoTargetChosen, TorpedoTargetEntity(raider)));
    }
    info!(
        "hollow: {} torpedo(es) committed to the raider",
        torpedoes.len()
    );
}

/// What the ordnance frames are about: the midpoint of the raider and the point
/// the fuze will go off at, which is [`TORPEDO_FUZE_RANGE`] short of it along
/// the boat's bearing. Framing on the raider alone puts the blast at the edge of
/// the frame; framing on this holds both.
#[cfg(feature = "debug")]
pub fn ordnance_subject(world: &mut World) -> Meters3 {
    let raider = raider_position(world);
    let bearing = (LANCE_POSITION - raider).get().normalize_or_zero();
    raider + Meters3(bearing * (TORPEDO_FUZE_RANGE.get() * 0.5))
}

/// Advance once the whole salvo is actually in the world.
#[cfg(feature = "debug")]
pub fn torpedo_salvo_in_flight(
    expected: usize,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .try_query_filtered::<Entity, With<TorpedoProjectileMarker>>()
            .is_some_and(|mut torpedoes| torpedoes.iter(world).count() == expected)
    })
}

/// Advance once the last torpedo is gone - the fuze despawns it and spawns the
/// blast in the same frame, so this IS the detonation.
#[cfg(feature = "debug")]
pub fn no_torpedo_in_flight() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| torpedo_range(world).is_none())
}

/// Advance once the leading torpedo is within `distance` of the raider.
#[cfg(feature = "debug")]
pub fn torpedo_within(
    distance: Meters,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        torpedo_range(world).is_some_and(|range| range < distance)
    })
}

/// Fail the run if the salvo dies before the capture range, rather than shooting
/// an empty approach.
#[cfg(feature = "debug")]
pub fn assert_salvo_still_live(world: &mut World, _: f32, _: u32) {
    let live = world
        .query_filtered::<Entity, With<TorpedoProjectileMarker>>()
        .iter(world)
        .count();
    assert!(
        live > 0,
        "hollow: the complete torpedo salvo was lost before reaching the capture range"
    );
}

/// Where the leading torpedo is, if one is in flight.
///
/// The framing half of [`torpedo_range`]: a still of a run-in has to hold the
/// round AND the hull it is diving on, and the round is the half that moves.
#[cfg(feature = "debug")]
pub fn lead_torpedo_position(world: &World) -> Option<Meters3> {
    let raider = world
        .try_query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()?
        .iter(world)
        .find(|(_, id)| id.0 == RAIDER_ID)
        .map(|(entity, _)| entity)?;
    let target = world.get::<GlobalTransform>(raider)?.translation();
    world
        .try_query_filtered::<&GlobalTransform, With<TorpedoProjectileMarker>>()?
        .iter(world)
        .map(|transform| transform.translation())
        .min_by(|a, b| f32::total_cmp(&a.distance(target), &b.distance(target)))
        .map(Meters3::from_engine)
}

/// How far the closest live torpedo is from the raider, if there is one of each.
#[cfg(feature = "debug")]
pub fn torpedo_range(world: &World) -> Option<Meters> {
    let raider = world
        .try_query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()?
        .iter(world)
        .find(|(_, id)| id.0 == RAIDER_ID)
        .map(|(entity, _)| entity)?;
    let target = world.get::<GlobalTransform>(raider)?.translation();
    world
        .try_query_filtered::<&GlobalTransform, With<TorpedoProjectileMarker>>()?
        .iter(world)
        .map(|transform| Meters::from_engine(transform.translation().distance(target)))
        .min_by(|a, b| f32::total_cmp(&a.get(), &b.get()))
}
