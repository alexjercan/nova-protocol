//! lesson_combat_field: the two COMBAT demonstrations about the space BETWEEN
//! ships - `combat_allegiance` (who is on whose side, in three colours) and
//! `combat_cover` (a burst breaking up on a rock instead of on the hostile
//! behind it).
//!
//! One producer, two frames, one set built for them: the player parked square
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
//! path they would die on a hull.
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
// The kit is included here as well as inside `shared/hollow.rs`, which that
// module warns against - two path copies are two distinct modules, so a
// `kit::NearField` built here is NOT the type hollow's own copy takes. Nothing
// crosses: every value this file hands to `hollow::` is a prelude type
// (`ShipDesign`, `EventActionConfig`), and the scatter this copy builds goes
// straight into the scenario.
#[path = "shared/kit.rs"]
mod kit;
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
/// stops. The drawn body is much bigger than the authored radius here: a
/// scatter asteroid's noise mesh reaches three and a half to six times past its
/// designation, and the factor is re-rolled per run, so this is authored small
/// and generously wider than the hostile's silhouette needs.
const ROCK_POSITION: Meters3 = Meters3::new(0.0, 0.0, -170.0);
/// The cover rock's authored radius.
const ROCK_RADIUS: Meters = Meters(16.0);
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
const ALLEGIANCE_EYE: Meters3 = Meters3::new(430.0, 70.0, -240.0);
/// What the allegiance still aims at: the middle of the line, so the shooter
/// and the covered hostile sit at opposite edges of the same frame.
const ALLEGIANCE_AIM: Meters3 = Meters3::new(0.0, -5.0, -245.0);

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

/// How long the trigger is held before the cover sheet opens.
///
/// Long enough for the first rounds to have crossed the 300 m to the stone and
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
            speed_cap: None,
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
        kit::catalog_ship(ships, "block_raider"),
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
            // Nothing in this producer shoots the scenery on purpose, but the
            // burst is aimed straight at it for two seconds. Cover that can be
            // shot away is cover that stops covering mid-sheet.
            invulnerable: true,
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
        // with it dead would be a sheet of cover that did not work.
        .step("the covered hostile is still there")
        .on_enter(|world: &mut World| {
            assert!(
                hollow::ship_by_id(world, COVERED_ID).is_some(),
                "the covered hostile did not survive the burst: the demonstration would show \
                 rounds reaching a ship the lesson says they cannot. Check that the rock still \
                 stands on the firing line."
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
}
