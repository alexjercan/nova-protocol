//! screenshot_railgun: the spinal lance, from the bore to the hole it leaves -
//! `wiki-section-railgun.png`, `wiki-section-railgun-sight.png`,
//! `wiki-combat-railgun.png`, `wiki-section-railgun-corridor.png`, the siege
//! lance's catalog card, and the `loop-section-railgun` loop.
//!
//! One player gunboat sits at the origin with a lance on its spine, bore down
//! -Z. A base Patrol Gunship stands downrange on that line, bow-on, so the shot
//! rakes its long axis rather than clipping a shoulder - which is the whole
//! skill the weapon asks for and the difference the bore sight is drawn to show.
//!
//! The walk raises the combat stance (the sight is gated on `WeaponsHot`),
//! commits the shot with a [`ScriptedRailgunOrder`], and frames the four beats
//! the charge hands it: the bolt partway up the bore, the sight fat with a
//! charge nearly run, the gap at the moment the slug lands, and the corridor
//! afterwards.
//!
//! The clock is slowed for the whole walk. A lance charges for 1.5 s and its
//! slug crosses the range at 15,000 m/s, and this capture runs on a software
//! rasterizer at some tens of frames a second: at real speed the entire charge
//! is a handful of frames and the flight is none. Slowing gameplay time is what
//! turns each beat into a framing that can be posed, settled and shot.
//!
//! The siege lance is a separate bench off the line, unfired. Nothing in the
//! base fleet but the stolen warship carries one, and its card is a product
//! photo rather than an event.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole walk, exit
//!   clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write each PNG and the loop
//!   (staged under `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_railgun --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_railgun --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[path = "shared/kit.rs"]
mod kit;

#[derive(Parser)]
#[command(name = "screenshot_railgun")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the spinal lance: the bore, the sight, the shot and the corridor it leaves. Autopilot-only: a scripted firing walk on a fixed range",
    long_about = None
)]
struct Cli;

/// The scenario id the range loads under.
const RANGE_ID: &str = "railgun_range";

/// Scenario id of the gunboat that fires, and the id of the lance on its spine.
const BOAT_ID: &str = "lance_boat";
const LANCE_ID: &str = "lance";

/// Scenario id of the hull that is shot at.
const TARGET_ID: &str = "target_gunship";

/// The catalog hull downrange. A base Patrol Gunship: 53 cells with a spine
/// long enough that a bow-on shot has something to bore THROUGH, which is the
/// claim the corridor figure makes.
const TARGET_HULL: &str = "block_gunship";

/// How far downrange the target stands. Well inside the lance's 18 km reach -
/// the distance is chosen for the FRAMING, so the gunboat and the hull it is
/// aimed at both read in one wide shot.
const TARGET_Z: Meters = Meters(-220.0);

/// Scenario id of the siege lance bench, and where it stands: off the bore
/// line in every axis, so nothing on it is in the firing shots and the shot
/// never reaches it.
const BENCH_ID: &str = "siege_bench";
const BENCH_AT: Meters3 = Meters3::new(200.0, -70.0, 90.0);

/// The loop this walk records around the shot.
#[cfg(feature = "debug")]
const RAILGUN_LOOP: &str = "loop-section-railgun";

/// How fast gameplay time runs for the walk.
///
/// A quarter speed puts the 1.5 s charge across some six seconds of wall clock,
/// which is a few hundred frames even on a software rasterizer - room to pose,
/// settle and shoot two framings inside one charge.
#[cfg(feature = "debug")]
const CHARGE_TIME_SCALE: f32 = 0.25;

/// How fast gameplay time runs from the last of the charge onward.
///
/// The slug leaves at 15,000 m/s and the target is 320 m away, so at any speed
/// worth charging at the whole flight falls inside one rendered frame. A
/// twentieth of real time spreads it over roughly a dozen, which is what lets
/// the wide shot land on a hull that is opening rather than on one that has
/// already finished coming apart.
#[cfg(feature = "debug")]
const SHOT_TIME_SCALE: f32 = 0.05;

/// Charge fraction each of the two charge-time framings is shot at.
///
/// The first is the bolt visibly partway up the bore; the second is the sight
/// line at nearly its full [`CHARGE_THICKEN`](nova_hud) weight.
#[cfg(feature = "debug")]
const CLOSEUP_CHARGE: f32 = 0.35;
#[cfg(feature = "debug")]
const SIGHT_CHARGE: f32 = 0.80;

/// Seconds of WORLD time the recording holds after the slug is away, and so
/// the moment every aftermath still is frozen at.
///
/// The whole set turns on this one number: the loop closes here, the clock
/// stops in the same breath, and `wiki-combat-railgun.png` and
/// `wiki-section-railgun-corridor.png` are both taken of the world as it
/// stands. Chosen off the FRAME rather than off the physics - at 0.13 s the
/// hull still reads as a hull with a corridor bored through it; by 0.45 s it
/// is a cloud of cells and nothing in the picture says what opened it.
#[cfg(feature = "debug")]
const CORRIDOR_WINDOW_SECS: f32 = 0.13;

/// Charge fraction the loop's recording opens at.
///
/// The loop is the SHOT, not the wait for it. Late enough that the recording
/// starts on a bore about to let go and early enough that the muzzle flash is
/// inside the window rather than one frame before it.
#[cfg(feature = "debug")]
const LOOP_OPEN_CHARGE: f32 = 0.97;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.init_resource::<RangeProbe>();
        app.add_observer(count_shots);
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        // Hold the gunboat where the framings were measured from. NOT
        // `freeze_bodies`, which makes every dynamic body static and would
        // pin the slug in the bore: the recoil is one impulse on a free hull,
        // so zeroing the gunboat's own velocities is the whole fix and leaves
        // the shot, the debris and the target alone.
        app.add_systems(Update, hold_the_boat.run_if(capturing));
        // The sight IS the subject of two of these framings, so it survives the
        // cinematic that takes the rest of the chrome away.
        app.add_systems(Update, unmanage_the_sight);
        app.add_plugins(range_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    ships: Res<GameShips>,
) {
    commands.trigger(LoadScenario(lance_range(&game_assets, &sections, &ships)));
}

/// The gunboat's hull, in BUILD-GRID cells.
///
/// The lance is three cells long and centred on its own origin, so at `-1` it
/// fills cells -2, -1 and 0 and everything else starts at +1. Cell -3 is left
/// empty because it has to be: a lance cannot traverse off its bore, so a
/// section standing there is a shot taken through the ship's own hull.
///
/// Small on purpose. The subject is the gun, and a hull big enough to bury it
/// would answer a question nobody asked of this producer.
fn boat_hull(sections: &GameSections) -> Vec<SpaceshipSectionConfig> {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let at = |id: &str, kind: &str, position: Vec3| SpaceshipSectionConfig {
        id: id.to_string(),
        position,
        rotation: Quat::IDENTITY,
        source: SectionSource::Inline(section(kind)),
        modifications: vec![],
    };
    let hull = |id: &str, position: Vec3| at(id, REINFORCED_HULL_SECTION_ID, position);

    vec![
        at(
            LANCE_ID,
            RAILGUN_LANCE_SECTION_ID,
            Vec3::new(0.0, 0.0, -1.0),
        ),
        // Beside the lance's aft cell, so the gun reads as MOUNTED on a spine
        // rather than as a spike floating in front of one.
        hull("mount_port", Vec3::new(-1.0, 0.0, 0.0)),
        hull("mount_starboard", Vec3::new(1.0, 0.0, 0.0)),
        hull("spine_fore", Vec3::new(0.0, 0.0, 1.0)),
        hull("waist_port", Vec3::new(-1.0, 0.0, 1.0)),
        hull("waist_starboard", Vec3::new(1.0, 0.0, 1.0)),
        at(
            "bridge",
            BASIC_CONTROLLER_SECTION_ID,
            Vec3::new(0.0, 0.0, 2.0),
        ),
        hull("flank_port", Vec3::new(-1.0, 0.0, 2.0)),
        hull("flank_starboard", Vec3::new(1.0, 0.0, 2.0)),
        hull("dorsal", Vec3::new(0.0, 1.0, 2.0)),
        hull("spine_aft", Vec3::new(0.0, 0.0, 3.0)),
        at(
            "drive_port",
            BASIC_THRUSTER_SECTION_ID,
            Vec3::new(-1.0, 0.0, 3.0),
        ),
        at(
            "drive_starboard",
            BASIC_THRUSTER_SECTION_ID,
            Vec3::new(1.0, 0.0, 3.0),
        ),
    ]
}

/// The siege lance on the one hull cell it needs to be a ship at all.
///
/// A three-cell lance placed at cell 0 fills -1, 0 and +1, so its mount sits at
/// +2. Unpowered, uncontrolled and off the line: this rig exists to be
/// photographed.
fn siege_bench(sections: &GameSections) -> ScenarioObjectConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let at = |id: &str, kind: &str, position: Vec3| SpaceshipSectionConfig {
        id: id.to_string(),
        position,
        rotation: Quat::IDENTITY,
        source: SectionSource::Inline(section(kind)),
        modifications: vec![],
    };

    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: BENCH_ID.to_string(),
            name: "Siege Lance Bench".to_string(),
            position: BENCH_AT,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::None,
            // Unclad, like the gunboat: this rig is a product photo OF the
            // section, and a skin would photograph the plate over it.
            hull: ShipSource::Inline(ShipHull {
                sections: vec![
                    at(
                        "siege_lance",
                        SIEGE_RAILGUN_LANCE_SECTION_ID,
                        Vec3::new(0.0, 0.0, 0.0),
                    ),
                    at(
                        "mount",
                        REINFORCED_HULL_SECTION_ID,
                        Vec3::new(0.0, 0.0, 2.0),
                    ),
                ],
                ..default()
            }),
            ..default()
        }),
    }
}

/// The range: the gunboat, the hull it is aimed at, the siege bench and a light
/// rig over all three.
fn lance_range(
    game_assets: &GameAssets,
    sections: &GameSections,
    ships: &GameShips,
) -> ScenarioConfig {
    let boat = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: BOAT_ID.to_string(),
            name: "Lance Gunboat".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Player),
            // An EMPTY input mapping: the walk drives the gun with a scripted
            // order and the stance with the bindings registry, so the ship
            // needs no button of its own - and nothing it carries can fly it
            // out of a framing that was posed frames earlier.
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: std::collections::BTreeMap::new(),
                speed_cap: None,
            }),
            // Bare structure, not a clad hull. Everything else this producer
            // spawns wears the skin it ships with, and these two rigs must
            // not: cladding fills the empty cells around a hull, so a lance
            // dressed like a warship is a lance behind a plate. Two of the
            // five framings here are of the gun itself - the closeup and the
            // siege card - and a finished hull would hide exactly the thing
            // being shown. Same call, same reason, as `shared/showcase.rs`.
            hull: ShipSource::Inline(ShipHull {
                sections: boat_hull(sections),
                ..default()
            }),
            ..default()
        }),
    };

    let target = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: TARGET_ID.to_string(),
            name: "Target Gunship".to_string(),
            position: Meters3::new(0.0, 0.0, TARGET_Z.get()),
            // Turned to face the bore, so the slug runs the length of the hull.
            // A shot across a beam crosses two or three cells and leaves a
            // dent; this one is the corridor.
            rotation: Quat::from_rotation_y(std::f32::consts::PI),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Enemy),
            controller: SpaceshipController::None,
            hull: ShipSource::Inline(kit::catalog_ship(ships, TARGET_HULL)),
            ..default()
        }),
    };

    ScenarioConfig {
        description: "A gunboat with a spinal lance, a hull downrange, and a siege lance on a \
                      bench."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            // The range lights itself: the engine spawns no light, so a
            // scenario that authors none renders black. Three DIRECTIONAL
            // lights, so one rig lights the gunboat, the target 320 m away and
            // the bench alike.
            actions: [
                vec![
                    EventActionConfig::SpawnScenarioObject(boat),
                    EventActionConfig::SpawnScenarioObject(target),
                    EventActionConfig::SpawnScenarioObject(siege_bench(sections)),
                ],
                ThreePointRig::around("range", Meters3::ZERO, 6.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            RANGE_ID.to_string(),
            "Railgun Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// What the walk has watched the gun do.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct RangeProbe {
    /// How many slugs have left the range's lance.
    shots: u32,
}

/// Count the shots that leave, so the walk can wait on the SHOT rather than on
/// a guessed number of frames.
#[cfg(feature = "debug")]
fn count_shots(_: On<RailgunFired>, mut probe: ResMut<RangeProbe>) {
    probe.shots += 1;
}

/// Hold the gunboat on its mark.
///
/// The recoil is a real impulse on a free hull and the framings are measured
/// from a boat at the origin, square with the world. Zeroing its velocities
/// each frame is the posed-capture equivalent of `hollow::pin_player`.
#[cfg(feature = "debug")]
fn hold_the_boat(
    mut boat: Query<
        (
            &mut Transform,
            &mut avian3d::prelude::LinearVelocity,
            &mut avian3d::prelude::AngularVelocity,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    for (mut transform, mut linear, mut angular) in &mut boat {
        transform.translation = Vec3::ZERO;
        transform.rotation = Quat::IDENTITY;
        linear.0 = Vec3::ZERO;
        angular.0 = Vec3::ZERO;
    }
}

/// Take the bore sight out of HUD management as it is drawn.
///
/// `nova_hud` tags every sight segment and kill ring `HudTier::Instrument` so a
/// cinematic takes the line away with the rest of the flight chrome - the right
/// call in the game, and the wrong one here, where the sight is what two of
/// these framings are of. Stripping the tag is enough: `apply_hud_visibility`
/// filters on it, so an untagged segment is nobody's to hide. The visibility it
/// already wrote is cleared in the same pass, because nothing restores a widget
/// the HUD has stopped tracking.
#[cfg(feature = "debug")]
fn unmanage_the_sight(
    mut commands: Commands,
    mut sight: Query<
        (Entity, &mut Visibility),
        (
            With<HudTier>,
            Or<(With<BoreSightSegment>, With<BoreSightMark>)>,
        ),
    >,
) {
    for (entity, mut visibility) in &mut sight {
        *visibility = Visibility::Inherited;
        commands.entity(entity).remove::<HudTier>();
    }
}

/// One framing: where the lens stands, what it looks at, and the file it writes.
#[cfg(feature = "debug")]
struct RangeShot {
    eye: Meters3,
    look_at: Meters3,
    path: &'static str,
}

/// The lance on the gunboat's spine, from three-quarters on and close, with the
/// bore across the frame so the capacitor bolt has somewhere to walk.
#[cfg(feature = "debug")]
const CLOSEUP: RangeShot = RangeShot {
    eye: Meters3::new(36.0, 13.0, 8.0),
    look_at: Meters3::new(-3.0, 1.0, -16.0),
    path: "wiki-section-railgun.png",
};

/// Across the gap rather than down it. The sight line is a 0.6 m holo thread:
/// seen end-on from behind the boat it is a sub-pixel dot, and seen from the
/// side it draws the whole length of the shot. So the lens stands off the bore,
/// far enough back to hold the whole gunboat at one edge and the target at the
/// other, with the thread strung between them.
///
/// Of the kill rings the sight also draws, this framing gets the FIRST one and
/// only that one: a ring is centred on the cell it condemns, so the entry
/// ring sits at the hull's near face and reads, while every ring behind it is
/// a cell deep inside solid structure with the plating in front of it. That
/// is what the shot has to say - where the bolt goes in, and that the gun has
/// already condemned what is behind it - so the wiki's figure note says one
/// ring at the entry face, not a row of them.
#[cfg(feature = "debug")]
const SIGHT: RangeShot = RangeShot {
    eye: Meters3::new(248.0, 76.0, -32.0),
    look_at: Meters3::new(0.0, 0.0, -112.0),
    path: "wiki-section-railgun-sight.png",
};

/// The HIT, across the target's starboard bow quarter.
///
/// This framing used to stand BEHIND the target looking back down the bore,
/// which put the stern between the lens and everything worth seeing: the slug
/// goes in at the bow, the corridor runs the length of the hull, and from
/// astern all of that happens on the far side of a hull plate. What came out
/// was a few sparks around a silhouette.
///
/// So: off the starboard bow instead, 150 m out. The lens spans 1.47 times its
/// distance, so the frame is some 220 m wide and the 110 m hull fills half of
/// it. The slug crosses from the right, the entry is on a face the camera can
/// see, and the sections the rake condemns die down the spine AWAY from the
/// lens - a chain of fireballs walking into the hull, which is the one thing a
/// still of a lance hit has to say.
#[cfg(feature = "debug")]
const GAP: RangeShot = RangeShot {
    eye: Meters3::new(140.0, 36.0, -180.0),
    look_at: Meters3::new(0.0, 0.0, -222.0),
    path: "wiki-combat-railgun.png",
};

/// The wound: the raked hull from off its stern quarter, low, 117 m out.
///
/// Downrange of the entry on purpose. The lance comes in over the bow, so a
/// lens on that side is looking into the fire the rake lights along its own
/// line; from behind and below the drive bell, the intact stern is nearest,
/// the wake runs out of frame both ways past it, and the bow beyond is the
/// part that is open to space - the corridor read as a length of ship rather
/// than as a hole in a plate.
#[cfg(feature = "debug")]
const CORRIDOR: RangeShot = RangeShot {
    eye: Meters3::new(96.0, 12.0, -298.0),
    look_at: Meters3::new(0.0, 0.0, -232.0),
    path: "wiki-section-railgun-corridor.png",
};

/// The siege lance's catalog card, off the line and unfired.
#[cfg(feature = "debug")]
const BENCH: RangeShot = RangeShot {
    eye: Meters3::new(242.0, -52.0, 122.0),
    look_at: Meters3::new(BENCH_AT.0.x, BENCH_AT.0.y, BENCH_AT.0.z),
    path: "catalog-siege-railgun-lance-section.png",
};

/// Pin the camera on one framing.
#[cfg(feature = "debug")]
fn frame(world: &mut World, shot: &RangeShot) {
    pose_camera(world, shot.eye, shot.look_at);
}

/// The range's lance section, found the way every other posed scene finds one:
/// by the marker the section carries.
#[cfg(feature = "debug")]
fn range_lance(world: &mut World) -> Option<Entity> {
    world
        .try_query_filtered::<(Entity, &EntityId), With<RailgunSectionMarker>>()?
        .iter(world)
        .find(|(_, id)| id.0 == LANCE_ID)
        .map(|(entity, _)| entity)
}

/// Whether the range's lance exists yet.
#[cfg(feature = "debug")]
fn lance_present() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&EntityId, With<RailgunSectionMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|id| id.0 == LANCE_ID))
    })
}

/// Whether the lance's charge has run at least `fraction` of its authored
/// seconds.
#[cfg(feature = "debug")]
fn charge_at_least(fraction: f32) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        let Some(mut query) =
            world.try_query_filtered::<(&EntityId, &RailgunCharge, &RailgunSectionConfigHelper), With<RailgunSectionMarker>>()
        else {
            return false;
        };
        query.iter(world).any(|(id, charge, config)| {
            id.0 == LANCE_ID && charge.progress(config.charge_seconds) >= fraction
        })
    })
}

/// Whether the slug has left the bore.
#[cfg(feature = "debug")]
fn shot_away() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<RangeProbe>()
            .is_some_and(|probe| probe.shots >= 1)
    })
}

/// Raise the weapons. The bore sight is gated on `WeaponsHot`, and the gun
/// refuses a commit while the ship is cold, so this is the beat everything
/// after it depends on.
#[cfg(feature = "debug")]
fn raise_weapons(world: &mut World) {
    press_action("combat_stance")(world);
}

/// Run gameplay time at `scale`.
#[cfg(feature = "debug")]
fn set_time_scale(world: &mut World, scale: f32) {
    world
        .resource_mut::<Time<Virtual>>()
        .set_relative_speed(scale);
}

/// Stop gameplay time.
///
/// The firing beat is over by here and every framing left is a still of what it
/// left behind. A running clock would carry the debris out of frame between the
/// poses, and each shot would be of a different wreck.
#[cfg(feature = "debug")]
fn pause_the_clock(world: &mut World) {
    world.resource_mut::<Time<Virtual>>().pause();
}

/// Commit the shot on the range's lance.
///
/// A scripted order rather than a synthesized trigger: the walk has no input
/// mapping to press, and the order holds the gun's own trigger down until the
/// shell actually leaves, so every gate the weapon has stays in force.
#[cfg(feature = "debug")]
fn commit_the_shot(world: &mut World) {
    let Some(lance) = range_lance(world) else {
        warn!("railgun range: no lance to commit");
        return;
    };
    world.entity_mut(lance).insert(ScriptedRailgunOrder);
}

/// The driven walk: raise, commit, and frame each beat of the charge and what
/// follows it.
#[cfg(feature = "debug")]
fn range_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let shoot_step = |script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
                      path: &'static str| {
        script
            .step(format!("shoot {path}"))
            .on_enter(move |world: &mut World| shoot(world, path))
            .until(shot_written(path))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
    };

    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the range")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            and(scenario_camera_present(), lance_present()),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // The stance first, and its own settle: `WeaponsHot` is derived from
        // the held stance every frame, and the sight is spawned off that -
        // so a commit taken in the same breath as the press charges a gun
        // whose sight has not been drawn yet.
        .step("raise the weapons")
        .on_enter(|world: &mut World| {
            raise_weapons(world);
            set_time_scale(world, CHARGE_TIME_SCALE);
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("frame the lance")
        .on_enter(|world: &mut World| frame(world, &CLOSEUP))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("commit the shot")
        .on_enter(commit_the_shot)
        .until(charge_at_least(CLOSEUP_CHARGE))
        .deadline(STEP_DEADLINE_SECS)
        .add();

    script = shoot_step(script, CLOSEUP.path);

    script = script
        .step("frame the sight")
        .on_enter(|world: &mut World| frame(world, &SIGHT))
        .until(charge_at_least(SIGHT_CHARGE))
        .deadline(STEP_DEADLINE_SECS)
        .add();

    script = shoot_step(script, SIGHT.path);

    script = script
        // The framing is posed BEFORE the slug leaves, because the loop this
        // beat records is the shot itself and a camera that moved into place
        // afterwards would have recorded the aftermath.
        .step("frame the gap")
        .on_enter(|world: &mut World| {
            frame(world, &GAP);
            set_time_scale(world, SHOT_TIME_SCALE);
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        // Run the charge out to its last breath BEFORE recording. At a
        // twentieth of real time the remaining fifth of a 1.5 s charge is six
        // seconds of wall clock, and that used to be the first half of the
        // loop: a static frame with a lit bore in the corner of it.
        .step("run the charge out")
        .until(charge_at_least(LOOP_OPEN_CHARGE))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("open the railgun loop")
        .on_enter(|world: &mut World| loop_start(world, RAILGUN_LOOP))
        .add()
        .step("wait for the slug")
        .until(shot_away())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // A tenth of a second of world, which at a twentieth of real time is
        // about two and a half seconds of footage: the muzzle flash, the slug
        // crossing the gap, the entry blowing out and the corridor opening
        // down the hull behind it. Longer than this and the recording runs on
        // into a debris cloud - and, because the stills are the frame this
        // beat ends on, so do they.
        .step("watch the corridor open")
        .until(elapsed(CORRIDOR_WINDOW_SECS))
        .add()
        // The clock stops in the SAME breath as the loop closes, not after the
        // encode. Encoding 290 frames of software render is five seconds of
        // wall clock, and a world left running through it is a third of a
        // second of drift the recording never shows: the loop ends on a hull
        // with a corridor bored down it and the stills used to be taken of the
        // cloud that hull had become. Now every still is the frame the loop
        // ended on.
        .step("close the railgun loop")
        .on_enter(|world: &mut World| {
            loop_end(world, RAILGUN_LOOP);
            pause_the_clock(world);
        })
        .until(loop_written(RAILGUN_LOOP))
        .deadline(120.0)
        .add();

    // Nothing may be shot while a loop is open: both take the same window
    // capture, the second is dropped as a duplicate render target, and the loop
    // then drains forever waiting for a frame that was never taken. So the
    // stills of the aftermath come after the encode, off the frozen world.

    script = shoot_step(script, GAP.path);

    script = script
        .step("frame the corridor")
        .on_enter(|world: &mut World| frame(world, &CORRIDOR))
        .until(frames(SETTLE_FRAMES))
        .add();

    script = shoot_step(script, CORRIDOR.path);

    script = script
        .step("frame the siege bench")
        .on_enter(|world: &mut World| frame(world, &BENCH))
        .until(frames(SETTLE_FRAMES))
        .add();

    shoot_step(script, BENCH.path)
}
