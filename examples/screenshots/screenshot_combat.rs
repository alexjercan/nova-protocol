//! screenshot_combat: "Rock hollow" - the travel, combat and HUD set.
//!
//! One flight in two acts, flown by the script the way a player would fly it.
//!
//! ACT 1, the leg: the racer sits in open space with a nav beacon roughly 750
//! units downrange. The weapons-lowered radar latches it (a TRAVEL lock, far
//! enough out that its bracket sits in open sky instead of on top of the
//! player's own hull), the travel computer engages, and the ship flies the real
//! GOTO leg - align, burn, coast, FLIP, brake. Two shots come off that leg and
//! nothing about it is faked: the plume, the trajectory ribbon and the flip are
//! the autopilot's.
//!
//! ACT 2, the ambush: the beacon doubles as its own trigger area, so crossing it
//! fires `OnEnter` and spawns the whole fight - a raider dead ahead, two
//! friendly racers on the near flanks and a hostile pair across the hollow.
//! That is scenario data, not script, so the OWNER's plain run gets the ambush
//! too: fly to the beacon and the hollow fills up. The rest of the set is as it
//! was - travel lock cleared, weapons raised, combat lock latched, guns live.
//!
//! WHAT THE PROOF RUN SHOWED (2026-08-05), and why the fight is shaped like
//! this: two AI flights DO fight each other with no player in the scene - a
//! flight spawned `allegiance: Some(Player)` acquires the default-Enemy flight
//! within a second and both sides run their guns continuously. Nothing in
//! acquisition is player-specific (`crates/nova_gameplay/src/input/ai/
//! acquisition.rs`), and now it has been run. What they will NOT do is brawl:
//! the engage maneuver flies to `AI_STANDOFF_RANGE` (250 units) and EXTENDS
//! AWAY once inside the band, so a fight is a wide slow ring, and turret bullets
//! (100 u/s, 5 s life) never connect at that range - 45 seconds of continuous
//! fire took zero sections off. So the AI pairs are the set's live background -
//! tracer streams and moving hulls - while every close beat is authored: the
//! lock subject is not AI-flown (it drifts on a nudged velocity), and the
//! destruction is scripted through the real damage path.
//!
//! The player's guns are REAL input: its turret sections carry `Mouse(Left)`
//! bindings from [`turret_bindings`], the script holds the trigger, and the
//! tracers in the combat frames are the player's own rounds hitting the lock.
//!
//! It replaces the old `screenshot_combat` range (three primitive blocks on an
//! empty backdrop) and absorbs `screenshot_juice`, whose scripted section blow
//! is one beat here.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - reach Playing, drive the whole
//!   script, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_REEL=1`: also capture the shots (staged under
//!   `NOVA_SHOT_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/reel NOVA_AUTOPILOT=1 NOVA_REEL=1 \
//!   cargo run --example screenshot_combat --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_combat --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[path = "shared/kit.rs"]
mod kit;

use bevy::{platform::collections::HashMap, prelude::*};
use bevy_enhanced_input::prelude::Binding;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_combat")]
#[command(version = "1.0.0")]
#[command(about = "The Rock hollow travel, combat and HUD set", long_about = None)]
struct Cli;

/// Scenario id of the player's ship - the ambush event filters on it.
const PLAYER_ID: &str = "hollow_player";
/// Where the leg starts: up, out and roughly 750 units short of the hollow, so
/// the GOTO has room for a full align-burn-coast-flip-brake profile.
const START_POSITION: Vec3 = Vec3::new(0.0, -60.0, 754.0);

/// The nav beacon: the travel lock's subject, the GOTO's destination and the
/// trigger that springs the ambush. Sitting a long way downrange is the point -
/// a destination a few units off the nose puts its bracket over the player's own
/// hull, which is exactly what this framing must not do.
const BEACON_ID: &str = "hollow_beacon";
/// Just past where the leg comes to rest: GOTO parks at
/// `FlightSettings::arrival_standoff` (50 units), so the ship ends up near the
/// origin - where the hollow's geometry and every combat framing is measured
/// from.
const BEACON_POSITION: Vec3 = Vec3::new(0.0, 0.0, -46.0);
/// Radar signature, world units of lock range per unit being 30: the default 20
/// (600 units) is short of this leg, so the beacon authors the range the leg
/// needs, the way the doc comment on the field asks.
const BEACON_SIGNATURE: f32 = 40.0;
/// The beacon's trigger radius. Wider than the arrival standoff, so the ambush
/// springs while the ship is still braking and the flights are in the world
/// before it comes to rest.
const BEACON_AREA_RADIUS: f32 = 90.0;

/// Scenario id of the raider the player locks, shoots and finally blows a
/// section off.
const RAIDER_ID: &str = "hollow_raider";
/// Where it appears: dead ahead of the parked player, far enough back that the
/// frame has depth between the two hulls, close enough that the target reads.
const RAIDER_POSITION: Vec3 = Vec3::new(0.0, 0.6, -34.0);
/// The raider section the scripted blow takes off - a forward hull cube on the
/// camera's side of the ship, so the fragments and the hole are both in frame.
const RAIDER_BLOWN_SECTION: &str = "racer_cube_i0_j0_km2";

/// Seconds each AI flight holds fire after it spawns, so the shots are taken of
/// a fight that has settled rather than of four ships still sorting out where
/// they are.
const ENGAGE_DELAY: f32 = 3.0;
/// How far an AI ship may stray from its post before it breaks off and comes
/// back. Wider than the standoff range the engage maneuver flies to (250), so
/// the fight is not permanently interrupted, tight enough that the hollow keeps
/// its ships instead of watching them leave.
const AI_LEASH: f32 = 320.0;

/// Frames a capture step holds after requesting its PNG. `capture_window`
/// spawns a bare `Screenshot` and is NOT a completion collector, so the last
/// step's hold is the only thing giving `save_to_disk` room to land before the
/// driver reports done and the app exits. A smoke run captures nothing and only
/// needs the step to be observable.
#[cfg(feature = "debug")]
fn capture_settle_frames(capturing: bool) -> u32 {
    if capturing {
        20
    } else {
        2
    }
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // NOT debug-gated: the rig is the set's look, so a plain run shows what a
    // capture would shoot.
    app.add_plugins(kit::photo_rig());

    #[cfg(feature = "debug")]
    {
        let capturing = std::env::var_os(nova_protocol::nova_debug::harness::REEL_ENV).is_some();
        // One step per beat, and every capture gets its OWN step: Bevy services
        // one primary-window capture per frame, so the rule is structural here
        // rather than a guard inside a shared step.
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                .step("load the hollow")
                .enter(GameStates::Loading)
                .until(player_ship_present())
                .deadline(30.0)
                .add()
                // ACT 1 - the leg.
                .step("settle at the start")
                .on_enter(hud_instrument)
                .until(elapsed(1.0))
                .add()
                // Weapons lowered, hold CTRL: the nav-slot radar opens and the
                // white NAV crosshair sweeps onto the beacon downrange. Waiting
                // on the LOCK rather than on a guessed second - and naming the
                // beacon - so a run that latches a rock instead aborts here and
                // says so.
                .step("sweep the nav radar")
                .on_enter(hold_radar)
                .until(travel_locked_on_beacon())
                .deadline(12.0)
                .add()
                .step("capture the radar lock")
                .on_enter(move |world| shoot(world, capturing, "tutorial-radar-lock.png"))
                .until(elapsed(0.2))
                .add()
                // The radar instrument as its own subject: the same latched nav
                // sweep from a tighter, lower camera.
                .step("frame the radar instrument")
                .on_enter(|world| {
                    pose(
                        world,
                        START_POSITION + Vec3::new(-3.6, 1.0, 6.0),
                        START_POSITION + Vec3::new(0.0, 0.3, -10.0),
                    )
                })
                .until(elapsed(0.4))
                .add()
                .step("capture the radar")
                .on_enter(move |world| shoot(world, capturing, "wiki-radar.png"))
                .until(elapsed(0.2))
                .add()
                // Hand the camera back before the leg: the travel beats are shot
                // over the game's own follow camera, because a pinned world pose
                // watches a ship that is leaving.
                .step("engage the travel computer")
                .on_enter(|world| {
                    release_radar(world);
                    unpose(world);
                    engage_goto(world);
                })
                .until(player_burning())
                .deadline(25.0)
                .add()
                .step("capture the burn")
                .on_enter(move |world| shoot(world, capturing, "feature-autopilot.png"))
                .until(elapsed(0.2))
                .add()
                // Coast, then the flip: the computer swings the ship end-for-end
                // and lights the drive back down the path. The wait is on the
                // BRAKING transition (the telemetry drops its flip point once
                // the brake is planned), not on a stopwatch.
                .step("coast to the flip")
                .until(player_braking())
                .deadline(150.0)
                .add()
                // The money frame of a flip-and-burn is the END of the swing:
                // the hull is round, the drive is lit back down the path and
                // the plume points at the camera. Waiting for the phase, not
                // for a fraction of a rotation nobody can time.
                .step("flip and burn")
                .until(player_retro_burning())
                .deadline(20.0)
                .add()
                .step("capture the flip")
                .on_enter(move |world| shoot(world, capturing, "wiki-flight.png"))
                .until(elapsed(0.2))
                .add()
                // ACT 2 - the ambush. Crossing the beacon's trigger already
                // spawned the flights (scenario data, so a plain run gets them
                // too); this waits for the leg to actually end.
                .step("arrive")
                .until(player_arrived())
                .deadline(90.0)
                .add()
                // Station-keeping starts HERE, not at load: the leg needs the
                // ship free to fly. See [`pin_player`] for why the combat act
                // cannot tolerate drift.
                .step("hold station in the hollow")
                .on_enter(|world| {
                    hold_station(world);
                    // The travel lock keeps its white bracket and GOTO chip on
                    // screen, and the combat shots are about the RED lock - two
                    // locks in one frame is chrome competing with itself.
                    clear_travel_lock(world);
                    despawn_by_id(world, BEACON_ID);
                    nudge_raider(world);
                })
                .until(elapsed(ENGAGE_DELAY + 3.0))
                .add()
                // Quiet stance first: weapons lowered, no lock, the contextual
                // rules keeping idle chrome off the frame. That IS the shot.
                .step("capture the contextual HUD")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "news-090-contextual-hud.png");
                })
                .until(elapsed(0.3))
                .add()
                // Raise weapons (RMB), then hold radar (CTRL) a beat later - the
                // natural order. At the hold threshold the radar latches the
                // combat slot on the raider and the reticle + inset come up.
                .step("raise the weapons")
                .on_enter(raise_stance)
                .until(elapsed(0.3))
                .add()
                .step("latch the combat lock")
                .on_enter(hold_radar)
                .until(elapsed(1.8))
                .add()
                // Guns live: the player's own turret streams tracers at the
                // locked raider, so the combat frames have the player's fire in
                // them and not just the AI's.
                .step("open fire")
                .on_enter(open_fire)
                .until(elapsed(0.8))
                .add()
                .step("capture the combat frame")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "feature-combat.png");
                })
                .until(elapsed(0.2))
                .add()
                .step("capture the combat lock")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "tutorial-combat-lock.png");
                })
                .until(elapsed(0.2))
                .add()
                // The same fight from the ship's shoulder: the HUD showcase,
                // every situational readout up with the hull in frame.
                .step("frame the HUD showcase")
                .on_enter(|world| pose(world, Vec3::new(5.0, 1.6, 7.0), Vec3::new(0.0, 0.4, -14.0)))
                .until(elapsed(0.4))
                .add()
                .step("capture the HUD in combat")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "feature-hud.png");
                })
                .until(elapsed(0.2))
                .add()
                .step("frame the HUD reference")
                .on_enter(|world| {
                    pose(world, Vec3::new(-6.0, 2.8, 5.0), Vec3::new(0.0, 0.2, -16.0))
                })
                .until(elapsed(0.4))
                .add()
                .step("capture the HUD reference")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, capturing, "wiki-hud.png");
                })
                .until(elapsed(0.2))
                .add()
                // The wide beats: the HUD goes cinematic and the camera leaves
                // the player, so the chrome would be reading a ship the frame
                // is no longer with. They are framed on the PLAYER's tracer
                // stream, not on the AI pairs: the AI holds a 250-unit standoff
                // where its ships are specks, while the player's fire crosses
                // 34 units of open hollow into a hull.
                .step("frame the exchange")
                .on_enter(|world| {
                    hud_cinematic(world);
                    pose(world, Vec3::new(9.0, 2.5, 6.0), Vec3::new(0.0, 0.3, -14.0));
                })
                .until(elapsed(0.5))
                .add()
                .step("capture the exchange")
                .on_enter(move |world| shoot(world, capturing, "wiki-combat.png"))
                .until(elapsed(0.2))
                .add()
                // The receiving end: past the raider's shoulder back down the
                // stream, so the frame is bullets, impact flashes and a hull
                // taking them.
                .step("frame the readability shot")
                .on_enter(|world| {
                    // Off the LIVE raider, not its spawn point: it drifts, and
                    // a stream of turret rounds pushes it further - by this
                    // beat it is a good ten units off, which throws a
                    // ten-unit-away camera clean off the subject.
                    let raider = raider_position(world);
                    pose(world, raider + Vec3::new(-12.0, 3.5, 8.0), raider)
                })
                .until(elapsed(0.5))
                .add()
                .step("capture the readability shot")
                .on_enter(move |world| shoot(world, capturing, "news-090-combat-readability.png"))
                .until(elapsed(0.2))
                .add()
                // The juice: one section blown off the raider through the
                // production damage path, shot while the hit rings are still
                // live. Three-quarter on the raider from inside the fire line,
                // so the frame carries the player's incoming tracers, the impact
                // flashes and the burnt-out section together. The destruction is
                // a BURNT HULL, not a fireball: a dead section is graded to
                // `DEAD_COLOR` in place (`sections/damage_tint.rs`) and the burst
                // is over in a frame or two, so the shot is the damage, shown
                // while the rounds are still arriving.
                .step("frame the raider")
                .on_enter(|world| {
                    let raider = raider_position(world);
                    pose(world, raider + Vec3::new(6.5, 2.2, 9.0), raider)
                })
                .until(elapsed(0.5))
                .add()
                .step("blow a section off the raider")
                .on_enter(blow_raider_section)
                .until(frames(12))
                .add()
                .step("capture the juice")
                .on_enter(move |world| shoot(world, capturing, "feature-juice.png"))
                .until(frames(capture_settle_frames(capturing)))
                .add(),
        );
        app.add_systems(Startup, (force_resolution, hide_dev_overlays));
        // Only under the script (`NOVA_AUTOPILOT`, named literally - the
        // harness re-exports `REEL_ENV` but not the autopilot's own): a plain
        // run is the owner flying this set, and a pinned ship cannot be flown.
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.add_systems(Update, pin_player.run_if(resource_exists::<HoldStation>));
        }
    }

    app.run()
}

/// Force the window to 1920x1080 (the 16:9 the web figures use) at startup.
#[cfg(feature = "debug")]
fn force_resolution(mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>) {
    if let Ok(mut window) = windows.single_mut() {
        window.resolution.set(1920.0, 1080.0);
        window.resizable = false;
    }
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(rock_hollow(&game_assets, &sections)));
}

/// The set: an empty start, a beacon 750 units downrange, a rock hollow around
/// it, and an ambush that springs when the player gets there.
fn rock_hollow(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let player = ship(
        PLAYER_ID,
        "Player Ship",
        START_POSITION,
        // Square with the world, NOT nosed at the beacon: the radar picks by
        // the CAMERA's look ray (`ActiveLookRay`), which opens down world -Z
        // whatever the hull is doing, and the start offset is chosen so the
        // beacon sits a few degrees off that ray - inside the 18-degree radar
        // cone, and clear of the player's own hull in frame.
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig {
            // Without this the trigger is bound to NOTHING: turret bindings are
            // per-section, snapshotted from this map by section id at spawn
            // (`nova_scenario/src/objects/spaceship.rs`), so an empty map is a
            // ship whose guns no button reaches.
            input_mapping: turret_bindings(sections, "racer"),
            speed_cap: None,
            // The player holds fire through several beats; running dry
            // mid-capture would leave a reload where the tracers should be.
            infinite_ammo: true,
            lock_refire_secs: None,
        }),
        None,
        kit::kenney_hull(sections, "racer"),
    );

    // The corridor: big rocks spread wide around the hollow, so the leg has
    // something with parallax passing it instead of an empty starfield.
    let corridor = kit::NearField {
        id_prefix: "hollow_far_",
        count: 26,
        seed: 90727,
        distance: (200.0, 640.0),
        radius: (4.0, 10.0),
        y_spread: 160.0,
    };
    // The hollow itself: denser and bigger-bodied than the drift set's belt, and
    // it starts outside the raider so the lock framings keep a clear sightline.
    let shell = kit::NearField {
        id_prefix: "hollow_rock_",
        count: 44,
        seed: 40507,
        distance: (28.0, 120.0),
        radius: (1.0, 2.8),
        y_spread: 42.0,
    };

    ScenarioConfig {
        id: "rock_hollow".to_string(),
        name: "Rock Hollow".to_string(),
        description: "A run into a rock field, and the ambush waiting in it.".to_string(),
        cubemap: game_assets.cubemap.clone().into(),
        events: vec![
            ScenarioEventConfig {
                name: EventConfig::OnStart,
                filters: vec![],
                actions: vec![
                    corridor.action(game_assets),
                    shell.action(game_assets),
                    player,
                    beacon(),
                ],
            },
            ambush(sections),
        ],
        ..Default::default()
    }
}

/// The ambush: everything that fights, spawned when the player crosses the
/// beacon's trigger area.
///
/// Scenario data rather than script on purpose - this is the engine's own
/// `OnEnter` ambush pattern, and it means the owner's plain run gets the fight
/// by flying to the beacon, not only the scripted capture run.
fn ambush(sections: &GameSections) -> ScenarioEventConfig {
    // The lock subject: not AI, because an AI hostile flies to a 250-unit
    // standoff and no close framing survives that (see the module docs). It is
    // not dead still either - [`nudge_raider`] gives it a slow drift, so the
    // lock's DST and CLS readouts are of a moving target.
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
        kit::kenney_hull(sections, "racer"),
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
        Vec3::new(-64.0, 12.0, -44.0),
        Quat::from_rotation_y(0.2),
        fighter(vec![
            Vec3::new(-64.0, 12.0, -44.0),
            Vec3::new(-30.0, 4.0, -96.0),
            Vec3::new(-86.0, -6.0, -70.0),
        ]),
        Some(Allegiance::Player),
        kit::kenney_hull(sections, "racer"),
    );
    let wingman_b = ship(
        "hollow_wing_b",
        "Wingman",
        Vec3::new(62.0, -14.0, -58.0),
        Quat::from_rotation_y(-0.2),
        fighter(vec![
            Vec3::new(62.0, -14.0, -58.0),
            Vec3::new(96.0, 6.0, -104.0),
            Vec3::new(40.0, -20.0, -110.0),
        ]),
        Some(Allegiance::Player),
        kit::kenney_hull(sections, "racer"),
    );
    let hostile_a = ship(
        "hollow_hostile_a",
        "Raider",
        Vec3::new(-150.0, 34.0, -230.0),
        Quat::from_rotation_y(3.0),
        fighter(vec![
            Vec3::new(-150.0, 34.0, -230.0),
            Vec3::new(-70.0, 18.0, -290.0),
            Vec3::new(-190.0, 6.0, -300.0),
        ]),
        None,
        kit::kenney_hull(sections, "racer"),
    );
    let hostile_b = ship(
        "hollow_hostile_b",
        "Raider",
        Vec3::new(176.0, -38.0, -262.0),
        Quat::from_rotation_y(3.3),
        fighter(vec![
            Vec3::new(176.0, -38.0, -262.0),
            Vec3::new(90.0, -14.0, -320.0),
            Vec3::new(210.0, -4.0, -330.0),
        ]),
        None,
        kit::kenney_hull(sections, "cargob"),
    );

    ScenarioEventConfig {
        name: EventConfig::OnEnter,
        filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
            id: Some(BEACON_ID.to_string()),
            other_id: Some(PLAYER_ID.to_string()),
            ..Default::default()
        })],
        actions: vec![raider, wingman_a, wingman_b, hostile_a, hostile_b],
    }
}

/// Bind every turret section of a built hull to the trigger, the way the
/// shipped scenarios do (`shakedown_run` maps its two racer turret cubes to
/// `Mouse(Left)` + `Gamepad(RightTrigger2)`).
///
/// Derived from the catalog rather than typed out, for the same reason
/// [`kit::kenney_hull`] is: the ids ARE the layout, and a hand-listed pair goes
/// stale the moment a hull gains a gun.
fn turret_bindings(sections: &GameSections, hull: &str) -> HashMap<String, Vec<Binding>> {
    let prefix = format!("{hull}_cube_");
    sections
        .iter()
        .filter(|section| section.base.id.starts_with(&prefix))
        .filter(|section| matches!(section.kind, SectionKind::Turret(_)))
        .map(|section| {
            (
                section.base.id.clone(),
                vec![
                    MouseButton::Left.into(),
                    GamepadButton::RightTrigger2.into(),
                ],
            )
        })
        .collect()
}

/// A fighting AI ship's routine: fly `patrol` until the engage grace expires,
/// then fight, and come back when the fight drags it past the leash.
///
/// The route is what makes the set move before the first shot: the grace holds
/// the ship in `Patrol`, which flies the waypoint loop through the real GOTO
/// autopilot instead of station-keeping.
fn fighter(patrol: Vec<Vec3>) -> SpaceshipController {
    SpaceshipController::AI(AIControllerConfig {
        patrol,
        leash: Some(AI_LEASH),
        engage_delay: Some(ENGAGE_DELAY),
        ..default()
    })
}

/// The nav beacon: travel lock, GOTO destination and ambush trigger in one
/// object.
fn beacon() -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: BEACON_ID.to_string(),
            name: "Waypoint".to_string(),
            position: BEACON_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: "WAYPOINT".to_string(),
            radius: 3.0,
            color: Color::srgb(0.4, 0.75, 1.0),
            area_radius: Some(BEACON_AREA_RADIUS),
            lock_signature: Some(BEACON_SIGNATURE),
        }),
    })
}

/// One posed ship in the set.
fn ship(
    id: &str,
    name: &str,
    position: Vec3,
    rotation: Quat,
    controller: SpaceshipController,
    allegiance: Option<Allegiance>,
    sections: Vec<SpaceshipSectionConfig>,
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
            sections,
        }),
    })
}

/// Present once the leg is over and the ship should hold the station every
/// combat framing is measured from.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct HoldStation;

/// Put the HUD on (the contextual rules decide what is actually in shot).
#[cfg(feature = "debug")]
fn hud_instrument(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::On;
    }
}

/// Clean the screen, for the beats whose camera has left the player's ship.
#[cfg(feature = "debug")]
fn hud_cinematic(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::Cinematic;
    }
}

/// Request one shot of the primary window. Captures only when `NOVA_REEL` is
/// set, so the plain autopilot smoke run drives the same path without writing
/// files.
#[cfg(feature = "debug")]
fn shoot(world: &mut World, capturing: bool, path: &str) {
    if capturing {
        capture_window(world, path);
        info!("combat capture: {path}");
    }
}

/// Pin the camera for a framing the follow camera does not give.
#[cfg(feature = "debug")]
fn pose(world: &mut World, position: Vec3, look_at: Vec3) {
    reel_pose_camera(world, position, look_at);
}

/// Hand the camera back to the game.
#[cfg(feature = "debug")]
fn unpose(world: &mut World) {
    let camera = {
        let mut query = world.query_filtered::<Entity, With<ScenarioCameraMarker>>();
        query.iter(world).next()
    };
    if let Some(camera) = camera {
        world.entity_mut(camera).remove::<ScriptedCameraPose>();
    }
}

/// Start holding station: from here on the scripted run keeps the ship exactly
/// where the combat set was measured from, and the camera looks the way the
/// pinned hull does.
///
/// Re-seeding the rig is not optional. A GOTO ends nose-RETROGRADE (it braked
/// facing back down its own path), and the disengage re-seeds the mouse rig
/// from that attitude (`camera_controller/handback.rs`) - so pinning the hull
/// square without re-seeding leaves the camera parked on the wrong side of the
/// ship, filming the combat act over its shoulder from in front.
#[cfg(feature = "debug")]
fn hold_station(world: &mut World) {
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

/// Hold the player at the hollow's origin, for the combat act of a scripted run.
///
/// Not cosmetic, and not the STOP autopilot: the set's geometry is measured from
/// a player at the origin, and the radar picks the body nearest the AIM RAY
/// (`crates/nova_gameplay/src/input/targeting/radar.rs`), so a player a few tens
/// of units off station swings the parked raider off the ray and latches a
/// hostile two kilometres out instead. GOTO parks the ship within a few units of
/// here and pointing back down its own path (it braked nose-first), so this also
/// squares the hull up - a cut between beats, and the camera cuts with it.
#[cfg(feature = "debug")]
fn pin_player(
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
fn nudge_raider(world: &mut World) {
    let Some(raider) = raider_root(world) else {
        warn!("combat: no raider to nudge");
        return;
    };
    if let Some(mut velocity) = world
        .entity_mut(raider)
        .get_mut::<avian3d::prelude::LinearVelocity>()
    {
        velocity.0 = Vec3::new(0.35, 0.12, -0.25);
    }
}

/// Engage the travel computer on the beacon - the same `Autopilot` component
/// the G keybind inserts (`input/player/flight_rig.rs`), so the leg the ship
/// flies is the player's GOTO and not a scripted animation.
#[cfg(feature = "debug")]
fn engage_goto(world: &mut World) {
    let (Some(player), Some(beacon)) = (player_root(world), entity_by_id(world, BEACON_ID)) else {
        warn!("combat: no player or beacon to engage the travel computer on");
        return;
    };
    world
        .entity_mut(player)
        .insert(Autopilot::engage(AutopilotAction::Goto { target: beacon }));
    info!("combat: GOTO engaged on the beacon");
}

/// Advance once the travel lock is on the beacon (and not on some rock the aim
/// ray happened to cross).
#[cfg(feature = "debug")]
fn travel_locked_on_beacon() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(player) = player_root_ref(world) else {
            return false;
        };
        let Some(TravelLock(Some(target))) = world.get::<TravelLock>(player) else {
            return false;
        };
        world
            .get::<EntityId>(*target)
            .is_some_and(|id| id.0 == BEACON_ID)
    })
}

/// Advance once the leg is actually under way: the computer has published its
/// numbers and the ship is closing on the destination.
#[cfg(feature = "debug")]
fn player_burning() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        maneuver(world).is_some_and(|telemetry| telemetry.closing_speed > 20.0)
    })
}

/// Advance the frame the flip starts: the telemetry drops its flip point once
/// the brake is planned, which is the same instant the computer turns the ship
/// around.
#[cfg(feature = "debug")]
fn player_braking() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        maneuver(world).is_some_and(|telemetry| {
            telemetry.flip_point.is_none() && telemetry.closing_speed > 5.0
        })
    })
}

/// Advance once the flip has finished and the retro burn is lit: braking, and
/// past the align phase the swing spends its time in.
#[cfg(feature = "debug")]
fn player_retro_burning() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let braking = maneuver(world).is_some_and(|telemetry| telemetry.flip_point.is_none());
        braking
            && player_root_ref(world).is_some_and(|player| {
                world
                    .get::<Autopilot>(player)
                    .is_some_and(|autopilot| autopilot.phase == AutopilotPhase::Burn)
            })
    })
}

/// Advance once the leg is over: the autopilot disengages itself at the goal,
/// taking its telemetry with it.
#[cfg(feature = "debug")]
fn player_arrived() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        player_root_ref(world).is_some_and(|player| world.get::<Autopilot>(player).is_none())
    })
}

/// The live numbers of the player's engaged leg, if there is one.
#[cfg(feature = "debug")]
fn maneuver(world: &World) -> Option<&ManeuverTelemetry> {
    world.get::<ManeuverTelemetry>(player_root_ref(world)?)
}

/// Drop the travel lock, so only the combat lock is on screen.
#[cfg(feature = "debug")]
fn clear_travel_lock(world: &mut World) {
    let Some(player) = player_root(world) else {
        return;
    };
    if let Some(mut travel) = world.entity_mut(player).get_mut::<TravelLock>() {
        travel.0 = None;
    }
}

/// Hold CTRL: the radar gesture. Which slot it latches depends on the stance.
#[cfg(feature = "debug")]
fn hold_radar(world: &mut World) {
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
}

/// Release the radar gesture.
#[cfg(feature = "debug")]
fn release_radar(world: &mut World) {
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::ControlLeft);
}

/// Raise the weapons (RMB), switching the radar from the nav slot to combat.
#[cfg(feature = "debug")]
fn raise_stance(world: &mut World) {
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Right);
}

/// Hold the trigger (LMB) so the player's turret is firing in the combat shots.
#[cfg(feature = "debug")]
fn open_fire(world: &mut World) {
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
}

/// Despawn a scenario object by its scenario id.
#[cfg(feature = "debug")]
fn despawn_by_id(world: &mut World, id: &str) {
    if let Some(entity) = entity_by_id(world, id) {
        world.entity_mut(entity).despawn();
    }
}

/// The first entity carrying scenario id `id`.
#[cfg(feature = "debug")]
fn entity_by_id(world: &mut World, id: &str) -> Option<Entity> {
    let mut query = world.query::<(Entity, &EntityId)>();
    query
        .iter(world)
        .find(|(_, live)| live.0 == id)
        .map(|(entity, _)| entity)
}

/// Blow one forward hull section off the raider through the production damage
/// path - the same `HealthApplyDamage` a bullet delivers, so the shot is of the
/// real destruction, not of a prop.
#[cfg(feature = "debug")]
fn blow_raider_section(world: &mut World) {
    let Some(node) = raider_section_health(world) else {
        warn!("combat: no health node under section '{RAIDER_BLOWN_SECTION}' to blow");
        return;
    };
    world.trigger(HealthApplyDamage {
        entity: node,
        source: None,
        amount: 1.0e6,
    });
    info!("combat: blew '{RAIDER_BLOWN_SECTION}' off the raider");
}

/// The player's ship root.
#[cfg(feature = "debug")]
fn player_root(world: &mut World) -> Option<Entity> {
    let mut query =
        world.query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>();
    query.iter(world).next()
}

/// The player's ship root, from a read-only world (what a predicate gets).
#[cfg(feature = "debug")]
fn player_root_ref(world: &World) -> Option<Entity> {
    world
        .try_query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()?
        .iter(world)
        .next()
}

/// The raider's ship root.
#[cfg(feature = "debug")]
fn raider_root(world: &mut World) -> Option<Entity> {
    let mut query = world.query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>();
    query
        .iter(world)
        .find(|(_, id)| id.0 == RAIDER_ID)
        .map(|(entity, _)| entity)
}

/// Where the raider actually is right now; its spawn point if it has gone.
#[cfg(feature = "debug")]
fn raider_position(world: &mut World) -> Vec3 {
    raider_root(world)
        .and_then(|raider| world.get::<GlobalTransform>(raider))
        .map(|transform| transform.translation())
        .unwrap_or(RAIDER_POSITION)
}

/// The raider's blown section entity. Picked BY SHIP: the racers in the set
/// share section ids, as every shipped multi-ship scenario does.
#[cfg(feature = "debug")]
fn raider_section(world: &mut World) -> Option<Entity> {
    let raider = raider_root(world)?;
    let mut query = world.query_filtered::<(Entity, &EntityId), With<SectionMarker>>();
    let candidates: Vec<Entity> = query
        .iter(world)
        .filter(|(_, id)| id.0 == RAIDER_BLOWN_SECTION)
        .map(|(entity, _)| entity)
        .collect();
    candidates
        .into_iter()
        .find(|&entity| under(world, entity, raider))
}

/// The `Health` node of the raider's blown section: the health lives on the
/// section entity or on one of its children.
#[cfg(feature = "debug")]
fn raider_section_health(world: &mut World) -> Option<Entity> {
    let section = raider_section(world)?;
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
fn under(world: &World, entity: Entity, root: Entity) -> bool {
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
