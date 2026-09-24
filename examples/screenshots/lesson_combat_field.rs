//! lesson_combat_field: three demonstrations about the space BETWEEN ships -
//! `combat_allegiance` (who is on whose side, in three colours), `start_markers`
//! (the same three colours, and the player's own hull wearing none) and
//! `combat_cover` (a burst breaking up on a rock instead of on the hostile
//! behind it).
//!
//! `start_markers` is a STILL for the same reason `combat_allegiance` is one,
//! set out below: the triangle is a fixed 14 by 9 pixels and a loop cell would
//! halve it. It is a SEPARATE still because it argues a different thing - not
//! which colour means what, but that your own ship shows no marker at all - and
//! that needs the player's hull in the foreground rather than at an edge.
//!
//! One producer, three frames, one set built for them: the player parked square
//! with the world, a rock sitting on its firing line, a hostile behind that
//! rock, and a friendly, a second hostile and an unaligned drifter spread
//! across the near field. Both lessons are about RELATIONS - which ship is
//! which, and what stands between two of them - so both need a field with more
//! than two hulls in it, and neither of the fleet's fighting sets
//! (`shared/hollow.rs`) has a rock anywhere near the line the guns fire down.
//!
//! ## One still and one loop, and why the still is a still
//!
//! `combat_cover` is a LOOP: a burst leaving the player, crossing the pocket
//! and breaking up against stone is a thing that HAPPENS, and the hostile
//! standing untouched on the far side is only proof if the reader has watched
//! the rounds stop.
//!
//! Both frames are shot down the SAME beam at two ranges, because the cast is
//! one tableau: the shooter, the stone on its line, the hostile the stone
//! hides, a friendly above the line and an unaligned hull below it. The covered
//! hostile carries the red triangle in the still as well - a contact behind
//! cover is still a contact, and the marker says so whether the guns can reach
//! it or not.
//!
//! `combat_allegiance` is a STILL, and the reason is pixels rather than taste.
//! Its whole subject is the allegiance triangle, and that marker is authored at
//! a FIXED 14 by 9 pixels (`nova_hud::allegiance_markers`) - it does not grow
//! with the hull it sits over or with the window. A lesson still ships at the
//! full 1920x1080 capture, so the triangle arrives at the size the game draws
//! it; a loop cell is 960x540, which halves it to 7 by 4 and turns three
//! colours into three specks. The claim is static anyway - Player, Enemy or
//! Neutral, green, red or grey - so there is nothing to spend the other
//! nineteen cells on, and resolution is the one thing this frame cannot have
//! too much of.
//!
//! ## What aims the guns
//!
//! Nothing here aims. The player's turrets fire down the rig's own look ray,
//! which opens along world -Z whatever the camera is doing
//! (`lesson_combat_moves` writes this out), and the set parks the player square
//! at the origin with the rock and the covered hostile on that axis. So the
//! cover framing may stand wherever it reads - abeam, which is the only bearing
//! that has the shooter, the stone and the hostile behind it all in one cell -
//! without moving what the burst hits.
//!
//! The rock is REAL cover and not a prop: an asteroid's collider carries no
//! `Health` (`nova_scenario::objects::asteroid`), and a collider with no health
//! is a wall to either round type, so the rounds die on it through the same
//! path they would die on a hull. Every asteroid can be carved, so each round
//! that dies on it also takes a bite out of it.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole script, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot the still and tile the sheet
//!   (staged under `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_combat_field --features debug
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
use lesson::{lesson_profile, sweep_lesson_camera, LessonSweep, LESSON_GRID};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_combat_field")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's allegiance and cover demonstrations", long_about = None)]
struct Cli;

/// The still for "Who shoots whom".
#[cfg(feature = "debug")]
const ALLEGIANCE_SHOT: &str = "combat_allegiance.png";
/// The still for "Who is who".
#[cfg(feature = "debug")]
const MARKERS_SHOT: &str = "start_markers.png";
/// The sheet for "Cover and the firing line".
#[cfg(feature = "debug")]
const COVER_LESSON: &str = "combat_cover";

/// Scenario ids of the cast.
const PLAYER_ID: &str = "field_player";
/// The hostile the player cannot reach: parked on the firing line with the
/// rock between.
const COVERED_ID: &str = "field_covered";
/// The friendly in the near field - the green triangle.
const WING_ID: &str = "field_wing";
/// The unaligned hull - the grey triangle. Spawned with no allegiance at all,
/// which is what `NEUTRAL_GREY` is for.
const DRIFTER_ID: &str = "field_drifter";
/// The cover itself.
const ROCK_ID: &str = "field_rock";

/// Where the cover rock sits on the player's firing line.
///
/// On the axis, because that is where the guns point, and CLOSE - a third of
/// the way to the hostile - so the burst is seen to cross open space before it
/// stops. The drawn body is much bigger than the authored radius here: this
/// pinned noise mesh reaches well past its designation, so the rock is authored
/// small while remaining wider than the hostile's silhouette.
const ROCK_POSITION: Meters3 = Meters3::new(0.0, 0.0, -170.0);
/// The cover rock's authored radius.
///
/// Set by carving, not only silhouette: every stopped round bites into the
/// rock. Twenty meters is the smallest proven whole-meter radius that survives
/// the full sheet while leaving the shooter's nose and speed tag clear.
const ROCK_RADIUS: Meters = Meters(20.0);
/// The cover rock's mesh seed. Pinned, so the body that does the covering is
/// the same body in every re-shoot.
const ROCK_SEED: u32 = 51_507;

/// Where the covered hostile stands: on the same axis, three times the rock's
/// range, so it is unmistakably BEHIND the stone rather than beside it.
const COVERED_POSITION: Meters3 = Meters3::new(0.0, 0.0, -480.0);
/// Where the friendly sits - up and past the rock, well off the firing line.
const WING_POSITION: Meters3 = Meters3::new(-90.0, 80.0, -300.0);
/// Where the unaligned hull drifts - low and to starboard, under the line.
///
/// It sat 60 m nearer the shooter at first, and that put its grey triangle
/// thirty pixels off the cover rock's lower lobe - nearer the stone than the
/// hull it labelled, because an allegiance marker rides a fixed gap above its
/// ship's screen bounds and this hull's bounds are wide. Carrying it down the
/// axis pushes the marker clear across the black instead of shrinking the
/// tableau.
const DRIFTER_POSITION: Meters3 = Meters3::new(100.0, -85.0, -300.0);

/// The allegiance still's eye, and what it looks at.
///
/// The SAME beam the cover loop is shot from, closer in. That is not economy:
/// the two lessons are one tactical picture seen at two ranges, and the beam is
/// the only bearing that has the shooter, the stone, the hull behind the stone,
/// the friendly and the drifter laid out side by side instead of stacked down
/// the middle. Standing 430 m off puts every hull between 360 and 530 m from
/// the lens, which is close enough that a 60 m hull is a sixth of the frame and
/// the 14-pixel triangle over it is plainly ATTACHED to it.
///
/// The first cut of this frame stood high off the port quarter with the cast
/// spread over 1.2 km, and it recorded the player's own hull cut off in a
/// corner, the cover rock filling the middle and three triangles floating in
/// black a hundred pixels from the ships they belonged to.
#[cfg(feature = "debug")]
const ALLEGIANCE_EYE: Meters3 = Meters3::new(430.0, 70.0, -240.0);
/// What the allegiance still aims at: the middle of the line, so the shooter
/// and the covered hostile sit at opposite edges of the same frame.
#[cfg(feature = "debug")]
const ALLEGIANCE_AIM: Meters3 = Meters3::new(0.0, -5.0, -245.0);

/// Where the cast stands for the markers still, and where it is watched from.
///
/// A STAGED tableau rather than another bearing on the tactical one, and shot
/// LAST for that reason. The cover lesson needs a stone on the firing line at
/// 170 m, and that stone sits in the middle of every shot taken from behind
/// the player - which is the only place the player's own hull can be the
/// nearest thing in frame. Two cuts were tried from over the field and both
/// photographed the rock with triangles around it.
///
/// So this frame is set up after the cover sheet is tiled and the guns are
/// down: the stone is taken away, the three marked hulls are parked in a clean
/// row against black, and the player is left where it has been all along. What
/// the reader gets is the claim itself - four hulls, three triangles, and the
/// one nearest the camera wearing nothing.
/// ## Why the row is abeam of the player rather than beyond it
///
/// A marker is a small triangle over a hull, so the reader has to be able to
/// tell WHICH hull each one sits over. That needs two things at once: the four
/// hulls separated across the frame, and all four drawn at about the same size,
/// because a triangle over a distant hull is the same triangle and reads as
/// floating free. Both fall out of parking the cast ABEAM - one row, one range,
/// the player at the near end of it - and neither survives a shot down the
/// field's own axis, where the far hulls shrink and their marks pile up.
///
/// 120 m apart, which is about twice a hull, so every triangle has a hull under
/// it and empty black either side.
#[cfg(feature = "debug")]
const MARKERS_WING: Meters3 = Meters3::new(120.0, 8.0, -10.0);
/// Where the hostile is parked for it - the middle of the row, because red is
/// the mark the reader looks for first.
#[cfg(feature = "debug")]
const MARKERS_HOSTILE: Meters3 = Meters3::new(240.0, -6.0, 10.0);
/// Where the unaligned hull is parked for it, at the far end of the row.
#[cfg(feature = "debug")]
const MARKERS_DRIFTER: Meters3 = Meters3::new(360.0, 6.0, -5.0);
/// The markers still's eye: square on the middle of the row, far enough back
/// that all four hulls are in the lens.
///
/// 389 m is what the row's half-span asks for. The lens is about 73 degrees
/// across at 16:9, the outermost hull sits 180 m off the row's centre plus
/// about a hull's half length, and 215 m has to land inside three quarters of
/// that width - so the range is 215 / tan(29 degrees).
#[cfg(feature = "debug")]
const MARKERS_EYE: Meters3 = Meters3::new(180.0, 55.0, 389.0);
/// What the markers still aims at: the middle of the row, a little under the
/// hulls, so the row sits just below centre and the triangles keep their black.
#[cfg(feature = "debug")]
const MARKERS_AIM: Meters3 = Meters3::new(180.0, 20.0, 0.0);

/// Where the cover loop's eye stands, as a `shared/lesson.rs` sweep.
///
/// ABEAM, at 90 degrees, because this is the one bearing with all three
/// subjects in it: the shooter at one edge, the stone in the middle and the
/// hostile it is hiding at the other edge. Shot from behind the player the
/// three are stacked down the middle of the cell and the burst is a dot that
/// stops. The +X side, because the set's three-point rig keys from +X/+Z and a
/// camera on the other beam films the whole tableau in its own shadow.
///
#[cfg(feature = "debug")]
const COVER_SUBJECT: Meters3 = Meters3::new(0.0, 0.0, -245.0);
/// How far abeam the cover eye stands: far enough that the shooter and the
/// covered hostile sit about twenty-two degrees off the axis, which is inside
/// the lens's thirty-six with room for the burst to cross the middle.
#[cfg(feature = "debug")]
const COVER_RANGE: Meters = Meters(620.0);
/// How far above the line the cover eye rides.
#[cfg(feature = "debug")]
const COVER_HEIGHT: Meters = Meters(90.0);
/// The cover eye's bearing: abeam to +X.
#[cfg(feature = "debug")]
const COVER_BEARING_DEGREES: f32 = 90.0;
/// Half the pendulum the cover eye swings through over the sheet.
///
/// A held pose was tried first and photographs as a still. The battery fires at
/// a fixed rate and the rounds cross the gap in under two frames, so a stream
/// of them lands on the same pixels in every cell: twenty cells of an
/// identical dashed line, which reads as a drawn overlay rather than as metal
/// in flight. The sine `LessonSweep` runs completes exactly one period over the
/// sheet, so a pendulum costs nothing at the wrap and buys the one thing a
/// fixed eye cannot give - parallax, which is what says the stone is BETWEEN
/// the shooter and the hostile rather than merely near them on screen.
///
/// Six degrees, because the shooter already sits twenty-two off the axis at
/// this range and the lens is thirty-six wide: a wider swing walks its hull off
/// the left edge at the end of the stroke.
#[cfg(feature = "debug")]
const COVER_ARC_DEGREES: f32 = 6.0;

/// How long the mounts are given to fold back into the deck before the markers
/// still is taken. Lowering the stance is animated, and the shutter does not
/// wait on an animation it cannot ask about.
#[cfg(feature = "debug")]
const MOUNTS_FOLD_SECS: f32 = 2.5;

/// How far a parked hull may be from its mark when the shutter opens, in engine
/// units. A body still settling against its own station keeping is a frame that
/// was composed for somewhere else.
#[cfg(feature = "debug")]
const MARKERS_PARK_EPSILON: f32 = 2.0;

/// How long the trigger is held before the cover sheet opens.
///
/// Long enough for the first rounds to have crossed the 170 m to the stone and
/// started breaking up on it: a sheet opened on the trigger records a cell of
/// empty space before anything is in flight, and the reader's first frame is
/// the one thing the lesson is not about.
#[cfg(feature = "debug")]
const BURST_LEAD_SECS: f32 = 1.2;

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
        app.add_plugins(combat_field_script());
        // The player is pinned on station from the moment the script says so:
        // the set's whole geometry - the rock on the axis, the hostile behind
        // it - is measured from a shooter at the origin, and a hull that drifts
        // a hundred metres is a hull firing past its own cover.
        app.add_systems(
            Update,
            (
                hollow::pin_player.run_if(resource_exists::<hollow::HoldStation>),
                sweep_lesson_camera,
            ),
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
    commands.trigger(LoadScenario(the_field(&game_assets, &sections, &ships)));
}

/// The set: an armed player at the origin, a rock on its firing line with a
/// hostile behind it, and three more hulls spread across the near field, one of
/// each allegiance the game has.
fn the_field(
    game_assets: &GameAssets,
    sections: &GameSections,
    ships: &GameShipDesigns,
) -> ScenarioConfig {
    let player_hull = kit::catalog_ship(ships, "block_gunship");
    let player = hollow::ship(
        PLAYER_ID,
        "Player Ship",
        Meters3::ZERO,
        // Square with the world: the look ray the turrets fire down opens along
        // world -Z, and the rock and the covered hostile are authored on that
        // axis.
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: hollow::turret_bindings(sections, &player_hull.sections),
        }),
        None,
        // The trigger is held for the whole cover sheet; a reload inside the
        // recording would read as the rock stopping the gun rather than the
        // rounds.
        hollow::unlimited_turrets(sections, player_hull.clone()),
    );

    let covered = hollow::ship(
        COVERED_ID,
        "Raider",
        COVERED_POSITION,
        Quat::from_rotation_y(std::f32::consts::PI),
        SpaceshipController::None,
        Some(Allegiance::Enemy),
        dev_fixtures::raider(),
    );
    let wing = hollow::ship(
        WING_ID,
        "Wingman",
        WING_POSITION,
        Quat::from_rotation_y(std::f32::consts::PI - 0.5),
        SpaceshipController::None,
        Some(Allegiance::Player),
        kit::catalog_ship(ships, "block_gunship"),
    );
    // No allegiance AT ALL, rather than a third faction: a ship the relation
    // model cannot place is exactly what the grey triangle means.
    let drifter = hollow::ship(
        DRIFTER_ID,
        "Surveyor",
        DRIFTER_POSITION,
        Quat::from_rotation_y(1.1),
        SpaceshipController::None,
        None,
        kit::catalog_ship(ships, "block_cutter"),
    );

    ScenarioConfig {
        description: "A firing line with cover on it, and one hull of every allegiance."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    backdrop().action(game_assets),
                    cover_rock(game_assets),
                    player,
                    covered,
                    wing,
                    drifter,
                ],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "the_field".to_string(),
            "The Field".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The one rock that matters: authored by hand rather than scattered, because
/// its whole job is to be in ONE place - on the axis the guns fire down, a
/// third of the way to the hull it hides.
fn cover_rock(game_assets: &GameAssets) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ROCK_ID.to_string(),
            name: "Rock".to_string(),
            position: ROCK_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: ROCK_RADIUS,
            texture: game_assets.asteroid_texture.clone().into(),
            kind: KIND_ROCK.to_string(),
            destroy_sound: None,
            // No well: a body strong enough to pull the pinned shooter would
            // drag the whole geometry off its axis over a capture run.
            mass: None,
            seed: Some(ROCK_SEED),
            lock_signature: None,
        }),
    })
}

/// The rock shell around the whole tableau: far out and thin, so the pocket has
/// somewhere to be without putting stone between any two of the five hulls.
fn backdrop() -> kit::NearField {
    kit::NearField {
        id_prefix: "field_rock_",
        count: 26,
        seed: 88_311,
        center: Meters3::new(0.0, 0.0, -400.0),
        distance: (Meters(1_700.0), Meters(2_900.0)),
        radius: (Meters(8.0), Meters(20.0)),
        y_spread: Meters(700.0),
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

/// How many of the player's rounds are in the air right now.
#[cfg(feature = "debug")]
fn rounds_in_flight(world: &mut World) -> usize {
    world
        .query_filtered::<(), With<TurretBulletProjectileMarker>>()
        .iter(world)
        .count()
}

/// Shoot one still of the field, then hold the trigger down and record the
/// burst dying on the rock.
#[cfg(feature = "debug")]
fn combat_field_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    // Every health pool on the covered hostile, taken before the trigger goes
    // down. The end check reads this list rather than walking the hull again:
    // a cladding plate absorbs its own hits (`HealthIsolated`) and is detached
    // when it dies, so a walk after the burst would not see it and the root
    // would still read full.
    let covered_pools = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Entity>::new()));
    let record_pools = covered_pools.clone();
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the field")
        .enter(GameStates::Loading)
        .until(and(player_ship_present(), scenario_camera_present()))
        .deadline(30.0)
        .add()
        // WHO SHOOTS WHOM. Weapons stay DOWN for this frame: the triangles are
        // instrument-tier markers the HUD draws whatever the stance is, and a
        // raised battery would put gun housings and a combat reticle over a
        // picture whose subject is three coloured marks.
        .step("raise the instruments and stand off the field")
        .on_enter(|world: &mut World| {
            hollow::hud_instrument(world);
            hollow::hold_station(world);
            pose_camera(world, ALLEGIANCE_EYE, ALLEGIANCE_AIM);
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the allegiance still")
        .on_enter(|world: &mut World| shoot(world, ALLEGIANCE_SHOT))
        .until(shot_written(ALLEGIANCE_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // COVER AND THE FIRING LINE. The eye goes abeam, the weapons come up,
        // and the trigger goes down and stays down.
        .step("come abeam of the line")
        .on_enter(|world: &mut World| {
            world.insert_resource(LessonSweep::new(
                COVER_SUBJECT,
                COVER_RANGE,
                COVER_HEIGHT,
                COVER_BEARING_DEGREES,
                COVER_ARC_DEGREES,
            ));
        })
        .until(frames(2))
        .add()
        .step("raise the weapons")
        .on_enter(hollow::raise_stance)
        .until(the_mounts_are_up())
        .deadline(20.0)
        .add()
        .step("open fire")
        .on_enter(move |world: &mut World| {
            let covered = hollow::ship_by_id(world, COVERED_ID)
                .expect("the covered hostile is in the field before the trigger goes down");
            let mut pools = record_pools.lock().expect("the pool list is not poisoned");
            let mut nodes = vec![covered];
            while let Some(node) = nodes.pop() {
                if world.get::<Health>(node).is_some() {
                    pools.push(node);
                }
                if let Some(children) = world.get::<Children>(node) {
                    nodes.extend(children.iter());
                }
            }
        })
        .on_enter(hollow::open_fire)
        .until(elapsed(BURST_LEAD_SECS))
        .add()
        // A burst that never left the mounts is the one failure this frame
        // cannot survive, and it is silent: the sheet still tiles, of a rock.
        .step("the guns are actually firing")
        .on_enter(|world: &mut World| {
            assert!(
                rounds_in_flight(world) > 0,
                "no rounds in flight after {BURST_LEAD_SECS}s on the trigger: the cover sheet \
                 would be a picture of a rock. Check the stance went up and the turret bindings \
                 reached the mounts."
            );
        })
        .until(frames(1))
        .add()
        .step("record the burst dying on the rock")
        .on_enter(|world: &mut World| sheet_start(world, COVER_LESSON, LESSON_GRID))
        .until(sheet_written(COVER_LESSON))
        .deadline(60.0)
        .add()
        // The hostile behind the stone is the whole point: a sheet that closed
        // with it scratched would be a sheet of cover that did not work. Alive
        // is not enough - the rock is carved by every round, and once it bores
        // through, the hostile takes hits for many cells before it could die.
        .step("the covered hostile is untouched")
        .on_enter(move |world: &mut World| {
            let Some(covered) = hollow::ship_by_id(world, COVERED_ID) else {
                panic!(
                    "the covered hostile did not survive the burst: the demonstration would \
                     show rounds reaching a ship the lesson says they cannot. Check that the \
                     rock still stands on the firing line."
                );
            };
            let pools = covered_pools.lock().expect("the pool list is not poisoned");
            assert!(
                !pools.is_empty(),
                "no health pools were recorded on the covered hostile"
            );
            let mut hurt = Vec::new();
            for &node in pools.iter() {
                match world.get::<Health>(node) {
                    None => hurt.push(format!("{node} is gone")),
                    Some(_) if node != covered && world.get::<ChildOf>(node).is_none() => {
                        hurt.push(format!("{node} came off the hull"));
                    }
                    Some(health) if health.current < health.max => {
                        hurt.push(format!("{node} at {}/{}", health.current, health.max));
                    }
                    Some(_) => {}
                }
            }
            assert!(
                hurt.is_empty(),
                "the covered hostile took hits through the cover: {}. The rock bored through \
                 during the sheet; it needs more body on the firing line.",
                hurt.join(", ")
            );
        })
        .until(frames(1))
        .add()
        .step("stop firing")
        .on_enter(|world: &mut World| {
            world
                .resource_mut::<ButtonInput<MouseButton>>()
                .release(MouseButton::Left);
            release_action("combat_stance")(world);
        })
        .add()
        // WHO IS WHO, off the same cast re-staged. Last, because it takes the
        // cover rock away and parks the hulls somewhere the other two frames
        // would not recognise.
        .step("clear the stone and park the cast in a row")
        .on_enter(stage_the_markers)
        .until(frames(SETTLE_FRAMES))
        .add()
        // The stance was up for the cover loop and comes down over several
        // frames, so the mounts are given time to fold before the shutter: the
        // triangles are instrument tier and a raised battery would put gun
        // housings across a picture whose subject is three coloured marks.
        .step("let the mounts fold away")
        .on_enter(|world: &mut World| pose_camera(world, MARKERS_EYE, MARKERS_AIM))
        .until(elapsed(MOUNTS_FOLD_SECS))
        .add()
        .step("the cast is where the frame was composed for")
        .on_enter(|world: &mut World| {
            for (id, at) in [
                (WING_ID, MARKERS_WING),
                (COVERED_ID, MARKERS_HOSTILE),
                (DRIFTER_ID, MARKERS_DRIFTER),
            ] {
                let ship = hollow::ship_by_id(world, id)
                    .unwrap_or_else(|| panic!("{id} is still in the field for the markers still"));
                let here = world
                    .get::<Transform>(ship)
                    .expect("a ship root carries a transform")
                    .translation;
                assert!(
                    here.abs_diff_eq(at.to_engine(), MARKERS_PARK_EPSILON),
                    "{id} drifted off its mark before the markers still: it is at {here:?} and \
                     the frame was composed for {at:?}"
                );
            }
        })
        .add()
        .step("capture the markers still")
        .on_enter(|world: &mut World| shoot(world, MARKERS_SHOT))
        .until(shot_written(MARKERS_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}

/// Take the cover rock away and park the three marked hulls in a clean row.
///
/// Transform and velocities together, the way `hollow::pin_player` does it: a
/// body moved by its transform alone keeps whatever the burst and its own
/// station keeping left in it, and arrives somewhere else by the shutter.
#[cfg(feature = "debug")]
fn stage_the_markers(world: &mut World) {
    // The cover loop's sweep is still in the world, and `sweep_lesson_camera`
    // re-solves the eye from it every frame. A `pose_camera` left standing
    // against it is overwritten before the shutter, so the still comes back
    // composed from the cover loop's bearing instead of this one.
    world.remove_resource::<LessonSweep>();
    // The WHOLE field, not just the cover stone. A marker is a small coloured
    // triangle floating clear of the hull it points at, and a rock behind one
    // takes the mark for its own: the first cut of this frame put an asteroid
    // directly over the green triangle, and the picture then claimed the
    // handbook's wingman was a boulder. Black is the only ground a triangle
    // cannot be read against something else on.
    let rocks: Vec<Entity> = world
        .query_filtered::<Entity, With<AsteroidMarker>>()
        .iter(world)
        .collect();
    if rocks.is_empty() {
        warn!("lesson markers: the field is already clear of rock");
    }
    for rock in rocks {
        world.entity_mut(rock).despawn();
    }
    for (id, at) in [
        (WING_ID, MARKERS_WING),
        (COVERED_ID, MARKERS_HOSTILE),
        (DRIFTER_ID, MARKERS_DRIFTER),
    ] {
        let Some(ship) = hollow::ship_by_id(world, id) else {
            warn!("lesson markers: {id} is not in the field");
            continue;
        };
        let mut entity = world.entity_mut(ship);
        if let Some(mut transform) = entity.get_mut::<Transform>() {
            transform.translation = at.to_engine();
        }
        if let Some(mut linear) = entity.get_mut::<avian3d::prelude::LinearVelocity>() {
            linear.0 = Vec3::ZERO;
        }
        if let Some(mut angular) = entity.get_mut::<avian3d::prelude::AngularVelocity>() {
            angular.0 = Vec3::ZERO;
        }
    }
}
