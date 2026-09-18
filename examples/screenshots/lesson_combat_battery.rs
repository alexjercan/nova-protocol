//! lesson_combat_battery: the two COMBAT demonstrations about the BATTERY
//! ITSELF - `combat_magazines` (a magazine draining through a burst and coming
//! back in one lump) and `combat_point_defense` (the flight computer working
//! the idle mounts against inbound torpedoes).
//!
//! One producer, two frames, one set: an armed player parked square at the
//! origin with real magazines in its guns, and three hostile torpedo boats
//! stood off beyond the reach of those guns. Both lessons are about what the
//! ship's own weapons do while nobody is flying them - one about the rate limit
//! the magazine imposes, the other about who holds the mounts when the player
//! does not - so both want the same hull, the same battery and the same HUD.
//!
//! The boats are authored at more than 2.2 km AND at least 150 m off the axis
//! the guns fire down, which is deliberate: a PDC round dies at 2.0 km
//! (1,000 m/s for a 2.0 s lifetime), so the magazine burst cannot reach them,
//! and the ships that launch the salvo for the second frame are still whole
//! when it is shot.
//!
//! ## The magazine frame is a LOOP, and it is shot at 2.75x
//!
//! The claim is a CYCLE: 500 rounds, 100 a second, and 200 back for every three
//! quiet seconds. Two seconds of sheet cannot hold a two-second burst AND the
//! three-second interval that answers it, so the world runs at
//! [`MAGAZINE_HASTE`] for the length of the recording and five and a half
//! seconds of ship time fit in the twenty cells. Nothing is faked: the gun
//! fires at its authored rate, the magazine spends a round a shot, and the
//! batch arrives on the section's own reload clock. Only the clock the CAMERA
//! runs on is different, the way `lesson_combat_rounds` slows the same clock to
//! 5% to follow two rounds down a lane.
//!
//! The arithmetic is what makes the loop WRAP, and the budget is tight: twenty
//! cells at [`MAGAZINE_HASTE`] is six ship seconds, and three of them belong to
//! the reload interval alone. That leaves about six cells for the burst and
//! four for the refilled ring, which is what [`BURST_SECS`] is sized against.
//! The burst spends around 180 of 500 rounds - the ring falling from eight lit
//! pips to five - and the reload's authored batch is 200, so it more than
//! covers that and the magazine clamps back to FULL. Cell twenty holds the ring
//! cell one opened on.
//!
//! The first cut ran at 2.75x with a two-second burst and photographed the
//! failure exactly: the ring fell from eight pips to four over nine cells and
//! then sat at four for the other eleven, because the batch was due at cell
//! nineteen-and-a-bit and the sheet closed first. A demonstration of a magazine
//! that only ever empties is the opposite of the lesson's claim, so the clock
//! and the burst are both sized to put the batch four cells inside the end.
//!
//! The camera holds still (an ACTION loop, see `shared/lesson.rs`): the motion
//! in the cell is the gun firing and the ring emptying, and an eye that moved
//! as well would read as a pan rather than as a drain.
//!
//! It stands off the STARBOARD QUARTER at 40 degrees rather than abeam. Abeam
//! is the bearing that shows a burst best, but the two forward dorsal mounts
//! are separated across the hull's X axis, so an eye on that axis stacks them
//! one behind the other - and two ammo rings within four pixels of each other
//! FOLD INTO ONE badge (`nova_hud::ammo_readout` clusters crowded gauges), so
//! the frame would have shown one gauge where the ship has two. From the
//! quarter the mounts sit 280 pixels apart and each keeps its own ring, while
//! the rounds still cross the cell instead of flying up the lens.
//!
//! ## The point-defence frame is a STILL, and the reason is pixels
//!
//! A point-defence line is authored 0.75 pixels wide at 35% alpha
//! (`POINT_DEFENSE_LINE_WIDTH`, `POINT_DEFENSE_LINE_COLOR`) - an ambient
//! readout, deliberately under the 2.0 px gizmo default, so that a full battery
//! answering a salvo reads as a fight and not as a diagram. A lesson STILL
//! ships at the full 1920x1080 capture, so the line arrives at the width the
//! game draws it. A loop cell is 960x540: every frame is lanczos-downscaled 2x
//! on its way into the sheet, which halves a sub-pixel line into roughly a
//! third of a pixel and spreads its already-low alpha over the neighbours.
//!
//! That is not a guess. The captured still was put through the tiler's own
//! downscale and read side by side with itself: at the full width six lines run
//! out of the battery, each plainly thinner and cooler than the tracer stream
//! beside it; at cell width two of them survive as broken traces and the other
//! four are gone inside the bloom of the rounds they run alongside. The thing
//! the lesson is ABOUT is the first thing that downscale destroys.
//!
//! So this frame buys resolution instead of time, and what it gives up is
//! small: the mounts swinging is a second of travel, and every claim the lesson
//! makes - no lock, weapons down, a line from each mount to its own pick - is
//! readable in one frame of the battery already on the threats.
//!
//! The eye stands off the starboard quarter and looks PAST the hull down the
//! threat bearing, so the gunship takes the lower left of the frame at 100 m
//! and the torpedoes converge toward the upper right at a kilometre. That is
//! the one composition that holds both ends of a line a kilometre long: framed
//! on the ship the picks are off the edge, and framed on the salvo the mounts
//! are five pixels of grey.
//!
//! Nothing here is staged around the feature: the player's hull carries
//! `Allegiance::Player` (the assignment pass requires the defender to have one)
//! and the boats carry `Allegiance::Enemy`, their torpedoes inherit it, the
//! script commits the salvo to the player exactly the way the AI's own launch
//! does, and from there the ownership precedence, the assignment, the deploy
//! and the trigger are the shipped ones.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole script, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the sheet and shoot the still
//!   (staged under `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_combat_battery --features debug
//! ```

#[path = "shared/hollow.rs"]
mod hollow;
use hollow::{dev_fixtures, kit};
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, LESSON_GRID};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_combat_battery")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's magazine and point-defence demonstrations", long_about = None)]
struct Cli;

/// The sheet for "Reloading magazines".
#[cfg(feature = "debug")]
const MAGAZINE_LESSON: &str = "combat_magazines";
/// The still for "Your battery defends itself".
#[cfg(feature = "debug")]
const DEFENSE_SHOT: &str = "combat_point_defense.png";

/// Scenario ids of the cast.
const PLAYER_ID: &str = "battery_player";
/// The three hostile torpedo boats, port, starboard and high.
const BOAT_IDS: [&str; 3] = [
    "battery_boat_port",
    "battery_boat_stbd",
    "battery_boat_high",
];

/// Where the three boats stand.
///
/// All of them beyond 2.2 km and at least 150 m off the world -Z axis the
/// player's guns fire down. Both numbers are load-bearing: a PDC round reaches
/// 2.0 km, so the magazine burst cannot touch a boat at this range, and a boat
/// on the axis would be raked by it anyway. Three bearings rather than one so
/// the salvo arrives spread and the mounts have something to SPLIT over - the
/// assignment's whole subject is which mount took which torpedo.
const BOAT_POSITIONS: [Meters3; 3] = [
    Meters3::new(-320.0, 90.0, -2_300.0),
    Meters3::new(300.0, -70.0, -2_500.0),
    Meters3::new(60.0, 240.0, -2_200.0),
];

/// Where the magazine loop's eye stands, and what it looks at.
///
/// Off the STARBOARD QUARTER at about 40 degrees and 70 m, looking at the
/// forward dorsal pair. The module docs carry the reasoning: abeam stacks the
/// two mounts on one another and folds their two gauges into one cluster badge,
/// and dead astern points the burst up the lens.
#[cfg(feature = "debug")]
const MAGAZINE_EYE: Meters3 = Meters3::new(46.0, 30.0, 36.0);
/// What the magazine loop aims at: just over the dorsal mounts, two cells up
/// and two cells forward of the hull's centre.
///
/// Level-ish with the deck rather than looking down on it. The gauge stands off
/// its weapon's projected edge up and to the RIGHT
/// (`nova_hud::ammo_readout::GAUGE_CLEARANCE`), so an eye that rides near the
/// mounts' own height puts every ring on black sky instead of on the armour
/// behind them - and the first cut, shot from fifteen metres higher, laid four
/// amber rings over a wall of grey plate.
#[cfg(feature = "debug")]
const MAGAZINE_AIM: Meters3 = Meters3::new(0.0, 21.0, -18.0);

/// How much faster than real time the world runs while the magazine sheet is
/// recording.
///
/// Twenty cells at ten a second is two seconds of sheet, and the cycle the
/// lesson describes is [`BURST_SECS`] of fire plus the section's authored three
/// second reload interval. At 3x the sheet holds six seconds of ship time: the
/// burst in the first six cells, the three-second interval in the next ten, and
/// the refilled ring in the last four.
///
/// Higher than it looks like it needs to be on purpose. At 2.75x the batch
/// landed a fraction of a cell PAST the end of the sheet and the recording
/// showed a magazine that never came back - see the module docs. The cell is
/// the quantum here (a beat cannot end mid-frame), so the interval needs whole
/// cells of margin rather than tenths.
#[cfg(feature = "debug")]
const MAGAZINE_HASTE: f32 = 3.0;

/// How long the trigger is held inside the magazine sheet, in ship seconds.
///
/// It buys five cells of trigger, and the gun keeps firing for about one cell
/// past the release (the beat ends on a frame boundary and the fire gate is
/// downstream of it), so the burst is six cells - 1.8 ship seconds, or about
/// 180 of the magazine's 500 rounds at the PDC's 100 a second. That is the ring
/// falling from eight lit pips to five.
///
/// The number it has to stay under is the reload's authored batch of 200: a
/// burst that spent more than one batch would leave the ring short in the last
/// cell and jump at the wrap.
#[cfg(feature = "debug")]
const BURST_SECS: f32 = 1.3;

/// How long the sheet holds after the trigger comes up, in ship seconds.
///
/// The section's authored reload interval is three seconds from the LAST shot,
/// so the quiet stretch has to outlast it or the batch arrives after the sheet
/// has closed. Half a second of margin past it, which is also what leaves the
/// last cell and a half showing the refilled ring - the state cell one hands
/// back to.
///
/// The recording waits on BOTH this and the sheet, and the clock is the reason:
/// a run with nothing recording satisfies `sheet_written` the instant it is
/// asked, so a beat that waited on the sheet alone let the smoke path skip the
/// whole interval and report a magazine that never refilled.
#[cfg(feature = "debug")]
const QUIET_SECS: f32 = 3.5;

/// Where the point-defence still's eye stands, and what it looks at.
///
/// Off the starboard quarter at about 115 m, aimed 90 m PAST the hull down the
/// threat bearing. That puts the gunship in the lower left of the frame at a
/// third of the frame's height and sends world -Z away to the upper right, so a
/// line drawn from a mount to a torpedo a kilometre out runs diagonally across
/// the picture with both of its ends inside it.
#[cfg(feature = "debug")]
const DEFENSE_EYE: Meters3 = Meters3::new(75.0, 34.0, 78.0);
/// What the point-defence still aims at: a point down the threat bearing
/// rather than the hull, so the hull and its picks share the frame.
#[cfg(feature = "debug")]
const DEFENSE_AIM: Meters3 = Meters3::new(0.0, 2.0, -90.0);

/// How many mounts must have a pick before the point-defence still is shot.
///
/// More than one, because the claim is per-MOUNT: a line from each mount to its
/// own pick is only visible as a rule when there are several of them.
#[cfg(feature = "debug")]
const ENGAGED_MOUNTS: usize = 2;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::new(
            lesson_profile(),
        ));
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(combat_battery_script());
        // Pinned from the moment the script says so: both framings are measured
        // from a hull at the origin, and the second one has three torpedoes
        // driving at that hull.
        app.add_systems(
            Update,
            hollow::pin_player.run_if(resource_exists::<hollow::HoldStation>),
        );
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    ships: Res<GameShipDesigns>,
) {
    commands.trigger(LoadScenario(the_battery(&game_assets, &sections, &ships)));
}

/// The set: an armed player at the origin with the magazines it was authored
/// with, and three hostile torpedo boats stood off beyond its guns' reach.
fn the_battery(
    game_assets: &GameAssets,
    sections: &GameSections,
    ships: &GameShipDesigns,
) -> ScenarioConfig {
    let player_hull = kit::catalog_ship(ships, "block_gunship");
    let player = hollow::ship(
        PLAYER_ID,
        "Player Ship",
        Meters3::ZERO,
        // Square with the world: the aim ray the turrets follow opens along
        // world -Z, and the boats are authored off that axis on purpose.
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: hollow::turret_bindings(sections, &player_hull.sections),
        }),
        // An explicit allegiance, unlike the other lesson sets. The assignment
        // pass requires a defender to HAVE one - it reads `&Allegiance` and
        // compares it with the torpedo's - so a player hull spawned with none
        // is a hull the point defence never looks at.
        Some(Allegiance::Player),
        // The AUTHORED magazines, not `hollow::unlimited_turrets`: the ammo
        // gauge only exists for a section that carries a `SectionAmmo`, so a
        // rig with unlimited guns has nothing for this lesson to photograph.
        player_hull,
    );

    let boats: Vec<EventActionConfig> = BOAT_IDS
        .iter()
        .zip(BOAT_POSITIONS)
        .map(|(id, position)| {
            hollow::ship(
                id,
                "Torpedo Boat",
                position,
                Transform::from_translation(position.to_engine())
                    .looking_at(Vec3::ZERO, Vec3::Y)
                    .rotation,
                SpaceshipController::None,
                Some(Allegiance::Enemy),
                dev_fixtures::cleanup_leader(),
            )
        })
        .collect();

    ScenarioConfig {
        description: "An armed hull at the origin with three hostile torpedo boats stood off."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![backdrop().action(game_assets), player],
                boats,
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "the_battery".to_string(),
            "The Battery".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The rock shell around the set: far out and thin, so the hull has somewhere
/// to be without putting stone on the torpedoes' run-in.
fn backdrop() -> kit::NearField {
    kit::NearField {
        id_prefix: "battery_rock_",
        count: 30,
        seed: 61_204,
        center: Meters3::new(0.0, 0.0, -600.0),
        distance: (Meters(1_500.0), Meters(3_000.0)),
        radius: (Meters(9.0), Meters(22.0)),
        y_spread: Meters(800.0),
    }
}

/// Advance once every one of the player's mounts has finished deploying.
///
/// The guns are safe until they are out of their housings, so a trigger pulled
/// on the way up is a trigger pulled at nothing.
#[cfg(feature = "debug")]
fn the_mounts_are_up() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(player) = hollow::player_root_ref(world) else {
            return false;
        };
        let Some(mut mounts) = world.try_query::<(&TurretStow, &ChildOf)>() else {
            return false;
        };
        let mut any = false;
        for (stow, ChildOf(parent)) in mounts.iter(world) {
            if *parent != player {
                continue;
            }
            any = true;
            if !stow.is_deployed() {
                return false;
            }
        }
        any
    })
}

/// Run the world at [`MAGAZINE_HASTE`] for the length of the magazine sheet.
#[cfg(feature = "debug")]
fn hasten_the_world(world: &mut World) {
    world
        .resource_mut::<Time<Virtual>>()
        .set_relative_speed(MAGAZINE_HASTE);
}

/// Put the clock back to real time for the point-defence frame.
#[cfg(feature = "debug")]
fn restore_the_world(world: &mut World) {
    world
        .resource_mut::<Time<Virtual>>()
        .set_relative_speed(1.0);
}

/// The emptiest magazine on the player's hull, as (rounds, capacity).
#[cfg(feature = "debug")]
fn lowest_magazine(world: &mut World) -> Option<(u32, u32)> {
    let player = hollow::player_root(world)?;
    let mut query = world.query::<(&SectionAmmo, &ChildOf)>();
    query
        .iter(world)
        .filter(|(_, ChildOf(parent))| *parent == player)
        .map(|(ammo, _)| (ammo.rounds, ammo.capacity))
        .min_by_key(|(rounds, _)| *rounds)
}

/// Pull every torpedo bay's trigger. The boats are the only hulls in the set
/// with bays, so this is their salvo.
#[cfg(feature = "debug")]
fn loose_the_salvo(world: &mut World) {
    let bays: Vec<Entity> = world
        .query_filtered::<Entity, With<TorpedoSectionMarker>>()
        .iter(world)
        .collect();
    assert!(
        !bays.is_empty(),
        "no torpedo bays in the set: the point-defence frame would be a picture of an idle \
         battery. Check the boats are still built from the cleanup leader fixture."
    );
    for bay in bays {
        if let Some(mut input) = world.entity_mut(bay).get_mut::<TorpedoSectionInput>() {
            **input = true;
        }
    }
}

/// Commit every fresh torpedo to the PLAYER and drop the bays' triggers.
///
/// A torpedo's target is decided exactly once, right after launch, by whoever
/// fired it - the player from the crosshair lock and the AI from its own
/// `AITarget`, both by inserting [`TorpedoTargetChosen`] and a
/// [`TorpedoTargetEntity`]. These boats have no controller, so the script does
/// that one write and the guidance, arming and fuze run themselves. Releasing
/// the bays here keeps it to ONE salvo: they would otherwise relaunch on their
/// own fire-rate clock and bury the frame in ordnance.
#[cfg(feature = "debug")]
fn commit_the_salvo(world: &mut World) {
    let Some(player) = hollow::player_root(world) else {
        warn!("battery: no player hull to commit the salvo to");
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
    assert!(
        !torpedoes.is_empty(),
        "the boats launched nothing: the point-defence frame has no threats to answer. Check the \
         bays carried ammunition and that the trigger beat was long enough to clear the tube."
    );
    for torpedo in &torpedoes {
        world
            .entity_mut(*torpedo)
            .insert((TorpedoTargetChosen, TorpedoTargetEntity(player)));
    }
    info!("battery: {} torpedo(es) committed", torpedoes.len());
}

/// How many mounts the flight computer is actually WORKING: it owns them and
/// has something for them to shoot. The same pair the line gizmo draws on, so
/// this counts exactly the lines the still will show.
#[cfg(feature = "debug")]
fn mounts_working(world: &World) -> usize {
    let Some(mut mounts) = world.try_query::<(&PointDefenseMount, &TurretDefenseTarget)>() else {
        return 0;
    };
    mounts
        .iter(world)
        .filter(|(mount, assignment)| flight_computer_works(Some(mount), Some(assignment)))
        .count()
}

/// Advance once at least `count` mounts are being worked by the computer.
#[cfg(feature = "debug")]
fn mounts_are_working(
    count: usize,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| mounts_working(world) >= count)
}

/// Record a magazine draining and coming back, then shoot the battery
/// answering a salvo on its own.
#[cfg(feature = "debug")]
fn combat_battery_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the battery set")
        .enter(GameStates::Loading)
        .until(and(player_ship_present(), scenario_camera_present()))
        .deadline(30.0)
        .add()
        // A MAGAZINE IS A RATE LIMIT. The eye goes to the forward dorsal pair
        // and stays there: the drain is the motion.
        .step("raise the instruments and frame the forward mounts")
        .on_enter(|world: &mut World| {
            hollow::hud_instrument(world);
            hollow::hold_station(world);
            pose_camera(world, MAGAZINE_EYE, MAGAZINE_AIM);
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("raise the weapons")
        .on_enter(hollow::raise_stance)
        .until(the_mounts_are_up())
        .deadline(30.0)
        .add()
        .step("run the clock at the recording speed")
        .on_enter(hasten_the_world)
        .until(frames(1))
        .add()
        // The sheet opens ON the trigger: the first cell is the ring full and
        // the first rounds leaving, which is the state the last cell hands back
        // to.
        .step("open the sheet and open fire")
        .on_enter(|world: &mut World| {
            sheet_start(world, MAGAZINE_LESSON, LESSON_GRID);
            hollow::open_fire(world);
        })
        .until(elapsed(BURST_SECS))
        .add()
        // A burst that spent nothing is the one failure this frame cannot
        // survive, and it is silent: the sheet still tiles, of a full ring.
        .step("cease fire with the magazine down")
        .on_enter(|world: &mut World| {
            world
                .resource_mut::<ButtonInput<MouseButton>>()
                .release(MouseButton::Left);
            let magazine = lowest_magazine(world);
            let (rounds, capacity) = magazine.expect(
                "the player's guns carry no magazine: the ammo gauge this lesson is about only \
                 exists for a section with a SectionAmmo, so the hull must keep its authored \
                 turrets.",
            );
            assert!(
                rounds < capacity,
                "{BURST_SECS}s on the trigger spent nothing ({rounds}/{capacity}): the sheet \
                 would be twenty cells of a full ring. Check the stance went up and the turret \
                 bindings reached the mounts."
            );
        })
        .until(and(sheet_written(MAGAZINE_LESSON), elapsed(QUIET_SECS)))
        .deadline(120.0)
        .add()
        // The batch has to have landed inside the sheet, or the reader is shown
        // a magazine that only ever empties.
        .step("the batch came back")
        .on_enter(|world: &mut World| {
            let (rounds, capacity) = lowest_magazine(world).expect("the magazines went missing");
            // The RING, not the round count: what has to match at the wrap is
            // what the reader sees, and the gauge shows a coarse fraction of
            // RING_SEGMENTS pips. The same rounding the HUD's own
            // `turret_lit_segments` does.
            let lit = (rounds as f32 / capacity as f32 * RING_SEGMENTS as f32).round() as usize;
            assert_eq!(
                lit, RING_SEGMENTS,
                "the gauge still reads {lit} of {RING_SEGMENTS} pips ({rounds}/{capacity}) after \
                 the sheet closed: the loop would wrap from a drained ring to a full one. Check \
                 MAGAZINE_HASTE still fits the section's authored reload delay into twenty cells \
                 and that BURST_SECS spends less than one batch."
            );
        })
        .until(frames(1))
        .add()
        // YOUR BATTERY DEFENDS ITSELF. Weapons down, no lock, and the clock back
        // to real time - the run-in is a real approach, not a hastened one.
        .step("lower the weapons and stand off the threat bearing")
        .on_enter(|world: &mut World| {
            release_action("combat_stance")(world);
            restore_the_world(world);
            pose_camera(world, DEFENSE_EYE, DEFENSE_AIM);
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("the boats loose a salvo")
        .on_enter(loose_the_salvo)
        .until(elapsed(1.5))
        .add()
        .step("commit the salvo to the player")
        .on_enter(commit_the_salvo)
        .until(frames(1))
        .add()
        // The computer takes the idle mounts on its own from here: the run-in
        // has to cross into the 1.5 km point-defence envelope before there is
        // anything to draw.
        .step("wait for the computer to take the mounts")
        .until(mounts_are_working(ENGAGED_MOUNTS))
        .deadline(300.0)
        .add()
        .step("let the mounts finish swinging")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the point-defence still")
        .on_enter(|world: &mut World| {
            let working = mounts_working(world);
            assert!(
                working >= ENGAGED_MOUNTS,
                "only {working} mount(s) still working the salvo: the still would show fewer \
                 lines than the lesson claims. Check the torpedoes are still in flight."
            );
            shoot(world, DEFENSE_SHOT);
        })
        .until(shot_written(DEFENSE_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("stand down")
        .add()
}
