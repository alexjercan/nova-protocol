//! lesson_combat_moves: the three COMBAT demonstrations that are the player's
//! own ship going to work - `combat_stance` (the weapons coming up),
//! `combat_components` (a lock stepping onto one section of a target) and
//! `combat_turrets` (the guns that bear, and the ones that do not).
//!
//! One producer, three frames, one set: `hollow::duel_hollow`, the armed
//! player and a parked raider with nothing else in the pocket. The three are
//! one chain of gestures - raise the weapons, sweep a lock onto the target,
//! step the lock down onto its sections, open fire - and each one needs the
//! state the one before it leaves behind, so shooting them apart would mean
//! staging the same duel three times.
//!
//! ## What each frame actually shows
//!
//! - `combat_stance` is an ACTION loop with the camera held (a zero-arc
//!   [`LessonSweep`], the `lesson_combat_radar` device): the sheet opens on a
//!   ship at rest with its mounts down, the stance goes up inside the
//!   recording, the housing lids part and the gun rises out of the deck. The
//!   lesson's SENTENCE is about the flight computer holding the nose where the
//!   guns can reach; that cannot be photographed on a parked hull in two
//!   seconds. What can is the ship going hot, which is the same gesture and
//!   the thing a player is about to do.
//! - `combat_components` is an ACTION loop for the same reason
//!   `lesson_combat_radar` is: the act - the bright marker stepping from one
//!   section of the raider to the next - happens inside the cells.
//! - `combat_turrets` is a STILL, and it is the awkward one. The game draws no
//!   firing-arc overlay, so this cannot be a picture of arcs. It is a picture
//!   of what an arc IS: the whole battery seen from overhead, every mount
//!   swung onto the same bearing and firing along it. See
//!   [`FIRING_YAW_DEGREES`] for why the masked-mount shot the lesson's
//!   sentence suggests cannot be staged on this hull, and what was corrected
//!   in the authored text instead.
//!
//! ## What aims what
//!
//! The camera here is FRAMING ONLY. Both the radar sweep and the component
//! snap aim down the player rig's own look ray from the player's own hull
//! (`ActiveLookRay`, `live_structure_anchor`), and the duel set parks the
//! player square with the world with the raider a few degrees off its nose.
//! So a beat may stand the camera anywhere that reads without moving what the
//! gesture picks - which is what lets the component sheet be shot from 150 m
//! off the TARGET while the lock it records is swept from 340 m away.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole script, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the two sheets and shoot the
//!   still (staged under `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_combat_moves --features debug
//! ```

#[path = "shared/hollow.rs"]
mod hollow;
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, sweep_lesson_camera, LessonSweep, LESSON_GRID};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_combat_moves")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's stance, component-lock and turret demonstrations", long_about = None)]
struct Cli;

/// The sheet for "Raising weapons".
#[cfg(feature = "debug")]
const STANCE_LESSON: &str = "combat_stance";
/// The sheet for "Lock a component".
#[cfg(feature = "debug")]
const COMPONENT_LESSON: &str = "combat_components";
/// The still for "Turret arcs".
#[cfg(feature = "debug")]
const TURRET_SHOT: &str = "combat_turrets.png";

/// Cells before the act in each sheet.
///
/// A sheet has to open on the state BEFORE the gesture, or a player sees the
/// answer and never sees the question. Four cells is four tenths of a second -
/// long enough to read as "nothing yet", short enough to leave the deploy and
/// a held result inside twenty.
#[cfg(feature = "debug")]
const LEAD_IN_CELLS: u32 = 3;

/// How far the stance camera stands off the MOUNT it watches.
///
/// Off the mount, not off the ship, and this is the whole framing problem of
/// the lesson. A point-defence mount is a few metres of hardware that rises
/// eight tenths of a metre; from a framing that holds the whole hundred-metre
/// gunship it is a pixel twitching on a hull, and from one tight enough to
/// fill the cell with the housing the picture is an anonymous grey wedge. This
/// is the distance where the gun coming up is a real object AND enough of the
/// deck, the hull line and the sky past it is in shot to say what it is bolted
/// to.
#[cfg(feature = "debug")]
const MOUNT_RANGE: Meters = Meters(58.0);
/// How far above the mount the stance camera rides.
///
/// Barely any, and that is the fix for the first cut of this frame. Looking
/// DOWN on the deck, the gun comes up out of a grey field into a grey field
/// and the travel is a few pixels of contrast against its own housing. Level
/// with the mount, the deck is a horizon and the barrels rise across open sky
/// - the same reason the HUD lesson stands off the beam rather than overhead.
#[cfg(feature = "debug")]
const MOUNT_HEIGHT: Meters = Meters(5.0);
/// The bearing it stands on, about the mount. Three-quarters on, so the barrel
/// that comes up is seen along its length rather than end-on.
#[cfg(feature = "debug")]
const MOUNT_BEARING_DEGREES: f32 = 35.0;

/// Where the component camera looks: the raider itself, which
/// `hollow::RAIDER_POSITION` parks 340 m down the player's nose.
#[cfg(feature = "debug")]
const COMPONENT_SUBJECT: Meters3 = hollow::RAIDER_POSITION;
/// How far it stands off the raider.
///
/// The subject is a marker drawn on ONE section of a ship, among a marker on
/// every OTHER section of it. They size themselves against their own section's
/// on-screen extent, so how well the act reads is set by how big the raider is
/// in the cell: from the player's own station, 340 m back, the whole ship is a
/// silhouette and the markers tile into one orange slab with nothing stepping
/// across it. This is close enough that the raider fills the cell and the
/// selected marker - bigger and brighter than its neighbours - is a square the
/// eye can follow from one section to the next.
#[cfg(feature = "debug")]
const COMPONENT_RANGE: Meters = Meters(85.0);
/// How far above the raider the component camera rides.
#[cfg(feature = "debug")]
const COMPONENT_HEIGHT: Meters = Meters(18.0);
/// The bearing it stands on, about the raider: back on the player's own side
/// of it, so the framing is one a pilot could be looking from.
#[cfg(feature = "debug")]
const COMPONENT_BEARING_DEGREES: f32 = 30.0;

/// Cells the first step is held for before the second one.
///
/// The lesson is a lock STEPPING, which needs two steps to read as stepping
/// rather than as one jump, and a dwell between them long enough to see where
/// it landed. Six cells is a little over half a second - and the pin a cycle
/// press takes lasts two seconds, so both steps stay pinned for the whole
/// sheet instead of snapping back to the crosshair mid-recording.
#[cfg(feature = "debug")]
const COMPONENT_DWELL_CELLS: u32 = 6;

/// Where the turret still stands: high over the player's own deck, looking
/// straight down the ship.
///
/// From overhead the hull is a plan of itself, and which mounts are on the
/// target's side of the spine and which are behind it IS the lesson. That
/// relationship is invisible from any framing at the hull's own level, where
/// the near mounts stand in front of the far ones.
#[cfg(feature = "debug")]
const TURRET_EYE: Meters3 = Meters3::new(18.0, 175.0, 55.0);

/// How long the guns are left running before the still is taken, so the shot
/// catches rounds in flight rather than the instant the trigger went down.
#[cfg(feature = "debug")]
const FIRING_SECS: f32 = 1.0;

/// How far the hull is turned off the target for the turret still, in degrees
/// of yaw.
///
/// A COMPOSITION figure, and it is worth saying what it is not. The lesson
/// teaches that a hull masks part of a mount's arc, and the obvious shot is a
/// mount that cannot bear sitting on its stops beside one that is firing. That
/// shot cannot be taken on this hull: the gunship carries its whole point
/// defence on the open dorsal deck, and the raider sits in that deck's own
/// plane, so every mount clears the hull and every mount fires however the
/// ship is turned. Masking here needs a target well below the deck, and the
/// target is the lock the frame before this one was built on.
///
/// So the yaw buys framing instead: turned across, the hull lies on the cell's
/// diagonal with the whole battery in plan and the rounds raking corner to
/// corner, rather than pointing out of the top of the frame. The lesson's own
/// authored description is corrected to match what this photographs - guns
/// tracking one target together - because a demonstration must not claim an
/// overlay the game does not draw.
#[cfg(feature = "debug")]
const FIRING_YAW_DEGREES: f32 = 145.0;

/// Present while the run is holding hulls still, and absent for the last beat.
///
/// The two loops need a scene that does not travel: a rock drifting under an
/// action loop means the last cell does not hand back to the first. The
/// FIRING beat needs the opposite - live rounds have to leave the muzzles and
/// cross to the target, and a blanket freeze turns each one static the frame
/// it spawns and parks a wall of stalled bullets on the deck.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct HoldTheScene;

/// Present while the last beat holds the hull turned across its target.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct TurnedAcross;

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
        app.add_plugins(combat_moves_script());
        // Nothing in the WORLD travels under these loops: what changes between
        // the first cell and the last is hardware on a hull and a marker on a
        // HUD. `sweep_lesson_camera` with a zero arc rewrites the same pose
        // every frame, because the scenario camera eases back toward its own
        // target and a pose set once drifts a cell at a time.
        app.add_systems(Update, sweep_lesson_camera);
        app.add_systems(
            Update,
            freeze_bodies.run_if(resource_exists::<HoldTheScene>),
        );
        // Only under the script, like the other hollow producers: the set's
        // geometry - and so which body the sweep marks - is measured from a
        // player at the origin, and a plain run is the owner flying it.
        //
        // The scene is held for the two loops and let go for the still - see
        // [`HoldTheScene`] - and the hulls are pinned on top of it, because a
        // freeze is what stops the ROCKS and a pin is what keeps a hull on the
        // station the framing was measured from.
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.add_systems(
                Update,
                (
                    hollow::pin_player.run_if(not(resource_exists::<TurnedAcross>)),
                    pin_player_across.run_if(resource_exists::<TurnedAcross>),
                    pin_raider,
                )
                    .run_if(resource_exists::<hollow::HoldStation>),
            );
        }
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
    // The DUEL set: an armed player and one target. Every frame here points at
    // exactly one thing, and `ambush_hollow`'s four AI craft put tracers and
    // banking hulls over all three of them.
    commands.trigger(LoadScenario(hollow::duel_hollow(
        &game_assets,
        &sections,
        &ships,
    )));
}

/// Hold the player at the origin, turned across its target.
///
/// A copy of `hollow::pin_player` with one thing changed, rather than a knob on
/// it: every other producer that shares the hollow wants a hull square with the
/// world, and this one wants it square for three beats and turned for the
/// fourth.
#[cfg(feature = "debug")]
fn pin_player_across(
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
        transform.rotation = Quat::from_rotation_y(FIRING_YAW_DEGREES.to_radians());
        linear.0 = Vec3::ZERO;
        angular.0 = Vec3::ZERO;
    }
}

/// Hold the raider on its station, the way `hollow::pin_player` holds the
/// player.
///
/// It is unmanned and under no power, so nothing accelerates it - but it is
/// being SHOT AT for the last frame, and an unpinned hull takes the impulse
/// and drifts out of a framing the beat before it measured.
#[cfg(feature = "debug")]
fn pin_raider(
    mut raider: Query<
        (
            &mut Transform,
            &mut avian3d::prelude::LinearVelocity,
            &mut avian3d::prelude::AngularVelocity,
            &EntityId,
        ),
        With<SpaceshipRootMarker>,
    >,
) {
    for (mut transform, mut linear, mut angular, id) in &mut raider {
        if id.0 != hollow::RAIDER_ID {
            continue;
        }
        transform.translation = hollow::RAIDER_POSITION.to_engine();
        linear.0 = Vec3::ZERO;
        angular.0 = Vec3::ZERO;
    }
}

/// Where the player's highest-riding turret mount is, in world meters.
///
/// The stance sheet is framed on a MOUNT, and which sections a hull carries
/// and where it bolts them is the ship design's business, not this producer's.
/// So the framing is measured off the live hull: the dorsal mount - the one
/// with the most sky over it - is the one whose lids and column a camera above
/// the deck can actually see open.
#[cfg(feature = "debug")]
fn dorsal_mount(world: &mut World) -> Option<Meters3> {
    let player = hollow::player_root(world)?;
    let mut mounts =
        world.query_filtered::<(&GlobalTransform, &ChildOf), With<TurretSectionMarker>>();
    mounts
        .iter(world)
        .filter(|(_, ChildOf(parent))| *parent == player)
        .map(|(transform, _)| transform.translation())
        .max_by(|a, b| a.y.total_cmp(&b.y))
        .map(Meters3::from_engine)
}

/// Stand the camera over one of the player's own mounts, holding still.
#[cfg(feature = "debug")]
fn frame_the_mount(world: &mut World) {
    let Some(subject) = dorsal_mount(world) else {
        warn!("lesson_combat_moves: the player carries no turret to frame");
        return;
    };
    world.insert_resource(LessonSweep::new(
        subject,
        MOUNT_RANGE,
        MOUNT_HEIGHT,
        MOUNT_BEARING_DEGREES,
        0.0,
    ));
}

/// Stand the camera in close on the raider, holding still.
#[cfg(feature = "debug")]
fn frame_the_target(world: &mut World) {
    world.insert_resource(LessonSweep::new(
        COMPONENT_SUBJECT,
        COMPONENT_RANGE,
        COMPONENT_HEIGHT,
        COMPONENT_BEARING_DEGREES,
        0.0,
    ));
}

/// Advance once every retractable mount on the player has finished rising.
///
/// The gate the stance sheet is worth shooting against: the deploy is the
/// picture, and a run whose mounts were already up would record twenty cells
/// of a ship doing nothing.
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

/// Advance once the player holds a COMBAT lock on the raider and has held it
/// long enough for the fine lock to unlock.
///
/// Both halves matter. The component layer only exists while the focus dwell
/// is complete (`LockFocus::focused_on`), so a sheet opened on a fresh lock
/// would record its lead-in cells before any marker was drawn at all, and the
/// cycle presses would land on nothing.
#[cfg(feature = "debug")]
fn the_raider_is_focused() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(mut ships) = world
            .try_query_filtered::<(&CombatLock, &LockFocus), (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()
        else {
            return false;
        };
        ships.iter(world).any(|(lock, focus)| {
            lock.0.is_some_and(|marked| {
                world
                    .get::<EntityId>(marked)
                    .is_some_and(|id| id.0 == hollow::RAIDER_ID)
                    && focus.focused_on(marked)
            })
        })
    })
}

/// Step the fine lock onto the next section - the same `component_next` a
/// player taps or rolls the wheel for.
#[cfg(feature = "debug")]
fn step_component(world: &mut World) {
    press_action("component_next")(world);
}

/// Let the step key up, so the next one is a fresh press rather than a held
/// key the rig has already spent.
#[cfg(feature = "debug")]
fn release_component(world: &mut World) {
    release_action("component_next")(world);
}

/// Raise the weapons, sweep a lock, step it down a section, and open fire.
#[cfg(feature = "debug")]
fn combat_moves_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the hollow")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle the hollow")
        .on_enter(|world: &mut World| {
            hollow::hold_station(world);
            world.insert_resource(HoldTheScene);
        })
        .until(elapsed(2.0))
        .add()
        // RAISING WEAPONS.
        .step("raise the instruments and frame a mount")
        .on_enter(|world: &mut World| {
            hollow::hud_instrument(world);
            frame_the_mount(world);
        })
        .until(elapsed(0.5))
        .add()
        .step("open the stance sheet on a ship at rest")
        .on_enter(|world: &mut World| sheet_start(world, STANCE_LESSON, LESSON_GRID))
        .until(frames(LEAD_IN_CELLS))
        .add()
        // The stance is HELD from here to the end of the run: `WeaponsRaised`
        // mirrors the button every frame, so letting go would fold the mounts
        // back down and re-safe the guns the last beat has to fire.
        .step("raise the weapons inside the recording")
        .on_enter(hollow::raise_stance)
        .until(sheet_written(STANCE_LESSON))
        .deadline(60.0)
        .add()
        // The deploy is about six tenths of a second of a two-second sheet. A
        // set that somehow started with its mounts up would tile twenty cells
        // of a ship doing nothing, and that must fail the run rather than
        // ship.
        .step("the sheet caught the mounts coming up")
        .on_enter(|world: &mut World| {
            assert!(
                the_mounts_are_up()(world),
                "the stance sheet closed with the mounts still in their housings: the lesson \
                 would show a gesture that never lands. Give the deploy more cells \
                 (LEAD_IN_CELLS) or check that the hull carries retractable mounts."
            );
        })
        .until(frames(1))
        .add()
        // LOCK A COMPONENT. The stance is up, so the sweep latches the COMBAT
        // slot rather than the nav one, which is what the fine lock hangs off.
        .step("sweep a combat lock onto the raider")
        .on_enter(hollow::hold_radar)
        .until(the_raider_is_focused())
        .deadline(30.0)
        .add()
        .step("release the sweep and keep the lock")
        .on_enter(hollow::release_radar)
        .until(elapsed(0.5))
        .add()
        .step("move in on the target")
        .on_enter(frame_the_target)
        .until(elapsed(0.5))
        .add()
        .step("open the component sheet on the whole-ship lock")
        .on_enter(|world: &mut World| sheet_start(world, COMPONENT_LESSON, LESSON_GRID))
        .until(frames(LEAD_IN_CELLS))
        .add()
        .step("step the lock onto a section")
        .on_enter(step_component)
        .until(frames(1))
        .add()
        .step("hold that section")
        .on_enter(release_component)
        .until(frames(COMPONENT_DWELL_CELLS))
        .add()
        .step("step it on again")
        .on_enter(step_component)
        .until(frames(1))
        .add()
        .step("hold the second section for the rest of the sheet")
        .on_enter(release_component)
        .until(sheet_written(COMPONENT_LESSON))
        .deadline(60.0)
        .add()
        // TURRET ARCS: overhead, guns live, so which mounts bear is the
        // picture.
        .step("turn the hull across the target")
        .on_enter(|world: &mut World| {
            world.remove_resource::<LessonSweep>();
            world.remove_resource::<HoldTheScene>();
            world.insert_resource(TurnedAcross);
            pose_camera(world, TURRET_EYE, Meters3::ZERO);
        })
        .until(elapsed(0.5))
        .add()
        .step("stand over the deck and open fire")
        .on_enter(hollow::open_fire)
        .until(elapsed(FIRING_SECS))
        .add()
        .step("capture the turret still")
        .on_enter(|world: &mut World| shoot(world, TURRET_SHOT))
        .until(shot_written(TURRET_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
