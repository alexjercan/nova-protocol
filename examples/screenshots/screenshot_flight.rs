//! screenshot_flight: "The ring" - the orbit and flight-computer set.
//!
//! A gravity planetoid with a rock ring around it and a Kenney racer that flies
//! both flight-computer verbs around it, in two acts.
//!
//! ACT 1, the ring: the ORBIT verb burns the ship from rest onto the
//! circular-orbit velocity and holds it there while the maneuver instruments
//! draw the holo ring and the radius spoke around the body. That is the shipped
//! `tutorial-orbit` image, the insertion burn and two cinematic variants.
//!
//! ACT 2, the departure: ORBIT is a HELD state, and a held state photographs as
//! a ship sitting still on a ring - the flight computer's loud beats are on a
//! LEG. So the ship leaves: a real GOTO out to a survey beacon over the pole,
//! align, burn, coast, FLIP, brake, park. The camera flies the leg with it -
//! every beat is posed off the ship's own track and off the light rig, twice
//! with the HUD up for the flight computer's chrome and twice clean for the
//! picture: the burn, the wide of the ship climbing out of the well, the flip,
//! and the arrival at the beacon.
//!
//! Everything here is the production flight stack. The script inserts the same
//! [`Autopilot`] components the ORBIT and GOTO keybinds do and then watches the
//! phase (`Align -> Burn -> Hold`) and the telemetry; no attitude, velocity,
//! ring or trajectory is faked.
//!
//! It replaces `screenshot_orbit`, whose set was a three-block primitive prop
//! ship, a 12-unit planetoid and a single 5.6-second stopwatch before the shot.
//!
//! WHAT IT SHIPS. Three manifest images: `tutorial-orbit` (act 1, the ring),
//! `feature-autopilot` (act 2, the departure burn - `AP GOTO - BURN`, the
//! trajectory ribbon and the destination readout) and `wiki-flight` (act 2, the
//! flip-and-burn). The last two were the Rock hollow's until the owner picked
//! these on 2026-08-05; `screenshot_combat` still flies its leg but no longer
//! shoots it. The other five frames are `variant-*.png` - alternates staged by a
//! capture run and named by no manifest entry, so they cost nothing.
//!
//! Sizing, so the numbers are not magic (the well math is
//! `crates/nova_gameplay/src/gravity.rs`):
//! - MEASURED, not assumed: the noise displaces the rock mesh outward, and the
//!   derived `BodyRadius` of this planetoid comes out at 79-108 units for an
//!   authored 20 - a factor of about 4.5, not the 2 a diameter-based reading of
//!   the task note suggests. The spread is per RUN, since the displacement is
//!   seeded from the global RNG, so nothing here may sit close to a boundary.
//!   Everything - the well strength, the orbit band, the body in frame -
//!   measures from that radius, and [`engage_orbit`] logs it, because the first
//!   cut of this scene put the ring 19 units off the surface and shot a frame of
//!   nothing but rock.
//! - The orbit band runs from `1.5 * (body_radius + 1)` out to `0.9 * 0.85 *
//!   SOI`, and the SOI comes from the authored mass alone
//!   (`sqrt(mu / soi_cutoff_accel)` = 490 units) - roughly 122 to 375 units
//!   here. [`ORBIT_RADIUS`] sits mid-band, so the explicit plan is the ring the
//!   ship actually flies and the body reads as a body rather than as terrain.
//! - The ring's circular speed is `sqrt(mu / r)`, around 14 u/s here: fast
//!   enough that the drive is lit and the hull is
//!   visibly banked into its plane, slow enough that a pinned camera keeps its
//!   subject for the length of a beat.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - reach Playing, drive the whole
//!   script, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also capture the shots (staged under
//!   `NOVA_SHOT_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_flight --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_flight --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[path = "shared/kit.rs"]
mod kit;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_flight")]
#[command(version = "1.0.0")]
#[command(about = "The Ring orbit and flight-computer set", long_about = None)]
struct Cli;

/// The planetoid's scenario id.
const PLANETOID_ID: &str = "ring_planetoid";
/// Authored radius. The mesh draws well past it - about 91 units of real body
/// for this 20 - and the well, the orbit band and every framing measure from
/// that derived radius, not from this number.
const PLANETOID_RADIUS: f32 = 20.0;
/// The body's mass parameter (mu, u^3/s^2) - the one authored gravity number,
/// setting both the pull and the SOI (`soi = sqrt(mu / soi_cutoff_accel)`, so
/// 490u here). Sized so [`ORBIT_RADIUS`] sits mid-band on every mesh seed:
/// this set's whole subject is the flight computer flying a REAL well, so a
/// tuned-down one would be a prop.
const PLANETOID_MASS: f32 = 60_000.0;

/// The player ship's scenario id.
const PLAYER_ID: &str = "ring_player";
/// The ring the ship holds: mid-band (roughly 122 to 375 units for this body),
/// so the explicit plan below is flown as authored rather than clamped. The
/// distance is chosen off the DRAWN body, at about three and a half of its
/// radii - close enough that the planetoid fills the lower third of a framing
/// with a curved limb, far enough that it is a body and not the ground.
const ORBIT_RADIUS: f32 = 320.0;
/// The ring's plane: the world horizontal, so the rock ring and the holo ring
/// are the same circle and the shots have one horizon rather than two.
const ORBIT_NORMAL: Vec3 = Vec3::Y;
/// Which way out of the ring the ship starts, and so WHERE ON THE RING every
/// act-1 shot happens - the framings are built off the ship's own radial and
/// track, but the light rig is fixed in WORLD space, so the phase decides
/// whether the outboard cameras see a lit planetoid or a black one.
///
/// This is the rim light's horizontal direction (`shared/kit.rs` puts the rim
/// at `(3, 4, -8)`, and at 16000 lux it is the brightest lamp on the set), so
/// the cameras that sit outboard of the ship and look back in - the tutorial
/// figure, the limb, the departure wide - are looking down the rim at the body's
/// lit face. A run that starts a quarter turn away photographs the night side:
/// the insertion is a real burn whose duration moves with the (per-run) derived
/// body radius, so the shot phase cannot be left to drift out of the light.
const START_RADIAL: Vec3 = Vec3::new(0.35, 0.0, -0.94);

/// Where the racer is parked before the verb takes it: on the ring, out along
/// [`START_RADIAL`].
fn start_position() -> Vec3 {
    START_RADIAL.normalize() * ORBIT_RADIUS
}

/// Pointing along the ring's travel direction (`normal x radial`), so the racer
/// is already squared with the track it is about to be put on rather than
/// swinging through 90 degrees on the first frame of the insertion.
fn start_rotation() -> Quat {
    Quat::from_rotation_arc(Vec3::NEG_Z, ORBIT_NORMAL.cross(START_RADIAL.normalize()))
}

/// The survey beacon the departure leg flies to.
const BEACON_ID: &str = "ring_beacon";
/// Where it sits: over the well's pole, and far enough out for a full
/// align-burn-coast-FLIP-brake profile. Polar on purpose - the ring is
/// horizontal, so a leg straight up out of it clears the body from EVERY point
/// on the ring (the closest a path from the ring to here passes the planetoid is
/// about 300 units, against a 92-unit body), and the departure does not depend
/// on where in its orbit the ship happened to be when the verb changed.
const BEACON_POSITION: Vec3 = Vec3::new(0.0, 760.0, 0.0);
/// Radar signature; lock range is 30 world units per unit of it. Not needed to
/// fly the leg - GOTO takes an entity, not a lock - but it is what puts the
/// beacon's own bracket on screen for the whole approach.
const BEACON_SIGNATURE: f32 = 40.0;

/// Seconds the ship holds the ring before the shots, so the last of the
/// circularization wobble is out of the hull's attitude.
#[cfg(feature = "debug")]
const STEADY_SECS: f32 = 2.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PERF_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window,
        // so a captured fps would measure the script, not the engine.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        // One step per beat, and every capture gets its OWN step: Bevy services
        // one primary-window capture per frame, so the rule is structural here
        // rather than a guard inside a shared step.
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                // Wait for the ship to EXIST rather than for a guessed load
                // duration: a slow load delays the beats instead of eating them.
                .step("load the ring")
                .enter(GameStates::Loading)
                .until(player_ship_present())
                .deadline(30.0)
                .add()
                .step("settle at the start")
                .on_enter(hud_instrument)
                .until(elapsed(1.0))
                .add()
                // The insertion burn: the ship is at rest on the ring radius, so
                // the verb has a real circularization to fly - full drive, the
                // hull swinging onto the plane, the holo ring and the radius
                // spoke already up. Waiting on the PHASE, not on a stopwatch.
                .step("engage the orbit computer")
                .on_enter(engage_orbit)
                .until(orbit_burning())
                .deadline(20.0)
                .add()
                // Close on the hull for the burn, from INBOARD of the ring. Two
                // rules, both paid for by a frame: the follow camera sits far
                // enough back that the racer is a hundred pixels of the shot, so
                // if the subject is the ship, get in front of it - and from
                // outboard the only thing behind the ship is the planetoid, which
                // swallows a small grey hull whole. Shot from inside the ring the
                // body is behind the CAMERA, and the burn reads against open
                // space.
                .step("frame the insertion burn")
                .on_enter(|world| {
                    let ship = ship_position(world);
                    let out = ship.normalize_or_zero();
                    let track = ship_heading(world);
                    pin(
                        world,
                        ship - out * 26.0 + Vec3::Y * 10.0 - track * 30.0,
                        ship,
                    );
                })
                .until(elapsed(0.3))
                .add()
                .step("capture the insertion burn")
                .on_enter(move |world| shoot(world, "variant-autopilot-ring.png"))
                .until(shot_written("variant-autopilot-ring.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                // Circularized: the verb reports Hold once the velocity error is
                // inside the hold tolerance. A long deadline, because the
                // insertion is a real burn against a real well and its duration
                // is the ship's thrust-to-mass, not a number this file picks.
                .step("settle onto the ring")
                .until(and(
                    orbit_holding(),
                    scenario_variable_is("orbit_stable", 1.0),
                ))
                .deadline(120.0)
                .add()
                .step("let the ring steady")
                .until(elapsed(STEADY_SECS))
                .add()
                // The shipped image. The follow camera looks down the TRACK, and
                // on a ring the body is 90 degrees off it - the game's own view
                // of an orbit has the thing being orbited out of frame, which is
                // a fine thing to fly and a useless thing to teach from. So the
                // tutorial figure is posed: outboard of the ship and slightly
                // above it, aimed most of the way at the body, so the whole
                // planetoid, the holo ring across it, the radius spoke and the
                // ship on the far end of that spoke are one picture. HUD ON - the
                // ring, the spoke and the AP chip are the subject.
                //
                // The offsets are small next to the aim distance ON PURPOSE. Ship
                // and body are ~250 units apart and the camera sits ~50 from the
                // ship: every unit of lateral offset swings the two apart in
                // frame, and at `Y * 30, -track * 20` the pair spanned more than
                // the lens had and the body was cropped off the top edge.
                .step("frame the orbit shot")
                .on_enter(|world| {
                    hud_instrument(world);
                    let ship = ship_position(world);
                    let out = ship.normalize_or_zero();
                    let track = ship_heading(world);
                    pin(
                        world,
                        ship + out * 45.0 + Vec3::Y * 15.0 - track * 10.0,
                        // Well short of the ship, most of the way to the body:
                        // the planetoid takes the middle of the frame and the
                        // ship falls out to the low corner on the end of its
                        // spoke, which is the relationship the figure teaches.
                        ship - out * 120.0,
                    );
                })
                .until(elapsed(0.4))
                .add()
                .step("capture the orbit shot")
                .on_enter(move |world| shoot(world, "tutorial-orbit.png"))
                .until(shot_written("tutorial-orbit.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                // The limb: from outside the ring, looking in past the ship at
                // the planetoid it is falling around. The camera leaves the
                // player, so the HUD goes cinematic - chrome anchored to a ship
                // the frame is only a spectator of reads as a mistake.
                .step("frame the limb")
                .on_enter(|world| {
                    hud_cinematic(world);
                    let ship = ship_position(world);
                    // Outward along the ship's own radius, and BELOW the ring
                    // plane. The height is the whole framing: a camera above the
                    // plane looking at an in-plane subject throws everything
                    // further down the plane toward the top of the frame, which
                    // is what put the planetoid in the corner on the first cut.
                    // From under the ring the body sits behind and below the
                    // hull, where a limb belongs. The offset is a fixed distance,
                    // not a fraction of the radius - the subject is the ship, and
                    // how far away it reads must not move with the ring.
                    pin(
                        world,
                        ship + ship.normalize_or_zero() * 24.0 - Vec3::Y * 3.0,
                        ship,
                    );
                })
                .until(elapsed(0.5))
                .add()
                .step("capture the limb")
                .on_enter(move |world| shoot(world, "variant-flight-limb.png"))
                .until(shot_written("variant-flight-limb.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                // The chase: behind and above the ship, canted inboard. The
                // height is the point - level with the plane, the holo ring
                // projects as a straight line across the frame and reads as
                // scaffolding; from above it curves, and the shot is a ship on a
                // ring. The body is NOT in this one and cannot be: from a camera
                // on the ring, the hull lies down the track and the planetoid
                // lies 90 degrees inboard of it, and no lens holds both without
                // backing out to where the ship stops reading (which is what the
                // tutorial framing does instead). The HUD comes back ON - the
                // instruments are world-space holo geometry, so they project into
                // a pinned camera too.
                .step("frame the chase")
                .on_enter(|world| {
                    hud_instrument(world);
                    let ship = ship_position(world);
                    let out = ship.normalize_or_zero();
                    let track = ship_heading(world);
                    pin(
                        world,
                        ship - track * 22.0 + out * 7.0 + Vec3::Y * 11.0,
                        ship + track * 4.0 - out * 6.0,
                    );
                })
                .until(elapsed(0.5))
                .add()
                .step("capture the chase")
                .on_enter(move |world| shoot(world, "variant-flight-chase.png"))
                .until(shot_written("variant-flight-chase.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                // ACT 2 - the departure. ORBIT is a held state and its shots are
                // of a ship sitting on a ring; the flight computer's loud beats
                // are on a LEG, so the ship leaves: it drops the ring for a real
                // GOTO out to the survey beacon over the pole, and the burn and
                // the flip-and-burn come off that. Same `Autopilot` component as
                // the G keybind, engaged straight over the orbit - nothing here
                // is a scripted animation.
                //
                // The camera FLIES the leg rather than sitting on the follow
                // camera. Each beat is posed off the ship's own track ([`chase`],
                // [`lead`]) and off the light rig ([`lit_side`]), because the
                // follow camera has one framing and the leg has four different
                // subjects: the burn, the well the ship is leaving, the flip, and
                // the arrival.
                .step("engage the travel computer")
                .on_enter(|world| {
                    hud_instrument(world);
                    unpose(world);
                    engage_goto(world);
                })
                .until(player_burning())
                .deadline(30.0)
                .add()
                // The burn, close and from behind: the drive is lit and the plume
                // is pointed at the lens, with the ribbon running up out of frame
                // to the beacon. HUD ON - this one is FOR the chrome (`AP GOTO -
                // BURN`, the destination readout, the ribbon).
                .step("frame the departure burn")
                .on_enter(|world| {
                    hud_instrument(world);
                    chase(world, 26.0, 20.0, 4.0, 10.0);
                })
                .until(elapsed(0.3))
                .add()
                .step("capture the departure burn")
                .on_enter(move |world| shoot(world, "feature-autopilot.png"))
                .until(shot_written("feature-autopilot.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                // The same burn, wide and clean: back along the ship's own radius
                // so the planetoid it is climbing away from sits behind it in
                // frame. This is the shot the leg exists for - a ship leaving a
                // well - and it is a picture rather than an instrument panel, so
                // the HUD goes cinematic and the camera goes far enough out that
                // the body, the ring and the hull are all in one frame.
                .step("frame the departure wide")
                .on_enter(|world| {
                    hud_cinematic(world);
                    let ship = ship_position(world);
                    let radial = ship.normalize_or_zero();
                    let side = lit_side(radial);
                    // Looking SHORT of the ship, back down the radius: it puts the
                    // hull in the upper third and gives the planetoid the rest.
                    pin(
                        world,
                        ship + radial * 105.0 + side * 34.0,
                        ship - radial * 42.0,
                    );
                })
                .until(elapsed(0.4))
                .add()
                .step("capture the departure wide")
                .on_enter(move |world| shoot(world, "variant-flight-departure.png"))
                .until(shot_written("variant-flight-departure.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                // Coast, then the flip: the computer swings the ship end-for-end
                // and lights the drive back down the path. The wait is on the
                // BRAKING transition (the telemetry drops its flip point once the
                // brake is planned), not on a stopwatch. The camera is handed back
                // for the coast - a pinned pose would spend it watching a ship
                // shrink.
                .step("coast to the flip")
                .on_enter(|world| {
                    hud_instrument(world);
                    unpose(world);
                })
                .until(player_braking())
                .deadline(150.0)
                .add()
                // The money frame of a flip-and-burn is the END of the swing: the
                // hull is round, the drive is lit back down the path and the plume
                // points where the ship is going. Waiting for the phase, not for a
                // fraction of a rotation nobody can time.
                .step("flip and burn")
                .until(player_retro_burning())
                .deadline(25.0)
                .add()
                // So the camera is AHEAD of the ship for this one, not behind it:
                // braking, the drive fires down the track, and the plume that was
                // at the lens during the departure is now on the far side of the
                // hull. It also fixes the frame the first cut of this beat got
                // wrong - the swing puts the hull's shadow side to a fixed-
                // direction rig, and [`lit_side`] is what keeps the lens on the key.
                .step("frame the flip")
                .on_enter(|world| {
                    hud_instrument(world);
                    lead(world, 21.0, 11.0, 6.0);
                })
                .until(elapsed(0.3))
                .add()
                .step("capture the flip")
                .on_enter(move |world| shoot(world, "wiki-flight.png"))
                .until(shot_written("wiki-flight.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                // The arrival: the leg ends parked off the beacon, which is the
                // only frame in the set that shows a flight-computer leg being
                // COMPLETED rather than flown. Clean screen again - the subject is
                // the two objects and the space between them.
                .step("close on the beacon")
                .on_enter(hud_cinematic)
                .until(player_arrived())
                .deadline(90.0)
                .add()
                // Over the ship's shoulder, up the closing line: the hull in the
                // near corner and the beacon it has been flying at for half a
                // minute glowing ahead of it. The framing is built off the LINE to
                // the beacon rather than off the ship's track - parked at the end
                // of a leg the velocity is nearly zero and the hull is still
                // pointing backwards from the brake, so neither says where the
                // shot is.
                .step("frame the arrival")
                .on_enter(|world| {
                    let ship = ship_position(world);
                    let line = (BEACON_POSITION - ship).normalize_or_zero();
                    pin(
                        world,
                        ship - line * 26.0 + lit_side(line) * 16.0 + Vec3::Y * 5.0,
                        ship + line * 24.0,
                    );
                })
                .until(elapsed(0.4))
                .add()
                .step("capture the arrival")
                .on_enter(move |world| shoot(world, "variant-flight-arrival.png"))
                .until(shot_written("variant-flight-arrival.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add(),
        );
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        // The leg camera re-solves against the ship every frame; it is inert
        // until a beat installs a [`LegCamera`].
        app.add_systems(Update, drive_leg_camera);
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShips>) {
    commands.trigger(LoadScenario(the_ring(&game_assets, &ships)));
}

/// The set: a gravity planetoid at the origin, a rock ring outside the flight
/// path, and the player's racer parked on the ring radius.
fn the_ring(game_assets: &GameAssets, ships: &GameShips) -> ScenarioConfig {
    let player = ship(
        PLAYER_ID,
        "Player Ship",
        start_position(),
        start_rotation(),
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: bevy::platform::collections::HashMap::new(),
            speed_cap: None,
            infinite_ammo: true,
        }),
        None,
        kit::kenney_hull(ships, "racer"),
    );

    // The ring debris: OUTSIDE the flight path, and small. Two rules, both
    // learned from a frame:
    // - The scatter region is an annulus centred on the origin - the same centre
    //   the orbit is - so a band overlapping the ring puts rocks in the ship's
    //   track, and a band INSIDE it walls off the one thing this set is about.
    //   Everything the camera looks inward through has to be empty.
    // - The 4.5x draw factor applies to these too, so an authored 2-7 is a field
    //   of 9-32 unit boulders. Authored small, they read as the debris the ring
    //   sweeps up rather than as a second asteroid field.
    let debris = kit::NearField {
        id_prefix: "ring_rock_",
        count: 34,
        seed: 71104,
        distance: (420.0, 760.0),
        radius: (1.0, 3.0),
        y_spread: 110.0,
    };

    ScenarioConfig {
        description: "A planetoid, its debris ring, and a ship flying the ORBIT verb around it."
            .to_string(),
        events: vec![
            ScenarioEventConfig {
                name: EventConfig::OnStart,
                filters: vec![],
                // The photo rig, now authored content rather than an example-side
                // observer swap: scale 1.0 around the origin reproduces the kit's
                // exact key/rim/fill numbers, so the captured frames are unchanged.
                actions: [
                    vec![
                        planetoid(game_assets),
                        debris.action(game_assets),
                        player,
                        beacon(),
                        EventActionConfig::VariableSet(VariableSetActionConfig {
                            key: "orbit_stable".to_string(),
                            expression: VariableExpressionNode::new_term(
                                VariableTermNode::new_factor(VariableFactorNode::new_literal(
                                    VariableLiteral::Number(0.0),
                                )),
                            ),
                        }),
                    ],
                    ThreePointRig::around("photo", Vec3::ZERO, 1.0).actions(),
                ]
                .concat(),
            },
            ScenarioEventConfig {
                name: EventConfig::OnOrbitStable,
                filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                    id: Some(PLANETOID_ID.to_string()),
                    other_id: Some(PLAYER_ID.to_string()),
                    ..default()
                })],
                actions: vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: "orbit_stable".to_string(),
                    expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                    )),
                })],
            },
        ],
        ..ScenarioConfig::new(
            "the_ring".to_string(),
            "The Ring".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The planetoid: the set's subject and the well the whole scene is about.
/// Invulnerable - nothing should be able to shoot the scenery out of a shot.
fn planetoid(game_assets: &GameAssets) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLANETOID_ID.to_string(),
            name: "Planetoid".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: PLANETOID_RADIUS,
            texture: game_assets.asteroid_texture.clone().into(),
            health: 5000.0,
            impact_sound: None,
            destroy_sound: None,
            mass: Some(PLANETOID_MASS),
            invulnerable: true,
            seed: None,
            lock_signature: None,
        }),
    })
}

/// The survey beacon: the departure leg's destination, and the only thing in
/// the set the player is given a reason to fly to.
fn beacon() -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: BEACON_ID.to_string(),
            name: "Survey Beacon".to_string(),
            position: BEACON_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: "SURVEY".to_string(),
            // Bigger than a nav point needs to be: the arrival frame is a
            // picture of the thing at the end of the leg, and a 3-unit orb at
            // the far end of a standoff is a pixel.
            radius: 6.0,
            color: Color::srgb(0.4, 0.75, 1.0),
            // No trigger area: nothing in this set springs on arrival.
            area_radius: None,
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
            hull: ShipSource::Inline(ShipHull {
                sections,
                ..default()
            }),
            ..default()
        }),
    })
}

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

/// Pin the camera for a framing the follow camera does not give.
#[cfg(feature = "debug")]
fn pose(world: &mut World, position: Vec3, look_at: Vec3) {
    pose_camera(world, position, look_at);
}

/// Pin a STILL framing: the pose, plus stopping any leg camera that would
/// otherwise re-solve it out from under this one on the next frame.
#[cfg(feature = "debug")]
fn pin(world: &mut World, position: Vec3, look_at: Vec3) {
    world.remove_resource::<LegCamera>();
    pose(world, position, look_at);
}

/// Where the photo rig's key light comes FROM (`shared/kit.rs`). The rig is
/// direction-only, so which half of a hull is lit depends on the hull's
/// ATTITUDE, and a maneuvering ship changes attitude for a living.
#[cfg(feature = "debug")]
const KEY_FROM: Vec3 = Vec3::new(-6.0, 5.0, 6.0);

/// A camera offset perpendicular to `track` that keeps the lens on the key
/// light's side of the subject: [`KEY_FROM`] with its along-track component
/// removed. Every leg framing offsets along this rather than along a world axis,
/// which is the difference between a hull with shape on it and the flat dark
/// silhouette an arbitrary side gives once the ship has swung around.
#[cfg(feature = "debug")]
fn lit_side(track: Vec3) -> Vec3 {
    let key = KEY_FROM.normalize();
    (key - track * key.dot(track))
        .try_normalize()
        .unwrap_or(Vec3::Y)
}

/// A camera that FLIES with the ship: the offsets are in the ship's own frame
/// (`side` along [`lit_side`], `along` down its track, `up` in world Y) and
/// [`drive_leg_camera`] re-solves them every frame while this is present.
///
/// A leg camera CANNOT be a single pinned pose. The ship crosses the transfer at
/// 65 u/s, so the ~0.3 s a framing beat holds is twenty units of travel - the
/// first cut of the flip beat pinned its camera 22 units ahead of the ship and
/// the ship flew straight through it, leaving an empty frame.
#[cfg(feature = "debug")]
#[derive(Resource, Clone, Copy)]
struct LegCamera {
    /// Offset onto the key light's side of the hull.
    side: f32,
    /// Offset down the track: negative is behind the ship, positive ahead.
    along: f32,
    /// Offset in world Y.
    up: f32,
    /// How far up the track the lens aims past the ship. Positive drops the hull
    /// low in the frame and gives the space it is flying into to the shot.
    look_ahead: f32,
}

/// Re-solve the leg camera against the ship's live position and track.
#[cfg(feature = "debug")]
fn drive_leg_camera(world: &mut World) {
    let Some(rig) = world.get_resource::<LegCamera>().copied() else {
        return;
    };
    let ship = ship_position(world);
    let track = ship_heading(world);
    let offset = lit_side(track) * rig.side + track * rig.along + Vec3::Y * rig.up;
    pose(world, ship + offset, ship + track * rig.look_ahead);
}

/// Fly the camera behind the ship on the key side. The framing for a ship under
/// power, since the drive is at the back and so is the lens.
#[cfg(feature = "debug")]
fn chase(world: &mut World, side: f32, back: f32, up: f32, look_ahead: f32) {
    world.insert_resource(LegCamera {
        side,
        along: -back,
        up,
        look_ahead,
    });
}

/// Fly the camera ahead of the ship on the key side. The framing for a ship
/// under RETRO power: braking, the drive fires down the track, so the lens has
/// to be down the track with it.
#[cfg(feature = "debug")]
fn lead(world: &mut World, side: f32, ahead: f32, up: f32) {
    world.insert_resource(LegCamera {
        side,
        along: ahead,
        up,
        look_ahead: 0.0,
    });
}

/// Engage the ORBIT verb on the planetoid's well with an explicit plan - the
/// same [`Autopilot`] component the keybind inserts, so the ring the ship flies
/// is the flight computer's and not an animation.
///
/// The plan is explicit rather than left to the verb because the ring is the
/// SET's geometry: an unplanned engage circularizes at whatever radius the ship
/// happens to be at, and every camera here is measured off [`ORBIT_RADIUS`].
#[cfg(feature = "debug")]
fn engage_orbit(world: &mut World) {
    let well = world
        .query_filtered::<Entity, With<GravityWell>>()
        .iter(world)
        .next();
    let (Some(well), Some(player)) = (well, player_root(world)) else {
        warn!("flight: no well or player to engage ORBIT on");
        return;
    };
    // The derived body radius, logged because it is the number this whole set
    // is sized off and it is NOT the authored one: the noise displaces the mesh
    // outward by a seeded factor, so a run that drifts far from the sizing note
    // above shows it here rather than in a frame full of rock.
    let body_radius = world.get::<GravityWell>(well).map(|well| well.body_radius);
    world
        .entity_mut(player)
        .insert(Autopilot::engage(AutopilotAction::Orbit {
            well,
            plan: Some(OrbitPlan {
                radius: ORBIT_RADIUS,
                normal: ORBIT_NORMAL,
            }),
        }));
    info!("flight: ORBIT engaged at r = {ORBIT_RADIUS} on a body of radius {body_radius:?}");
}

/// Hand the camera back to the game: stop flying any leg rig, then drop the
/// pinned pose.
#[cfg(feature = "debug")]
fn unpose(world: &mut World) {
    world.remove_resource::<LegCamera>();
    let camera = {
        let mut query = world.query_filtered::<Entity, With<ScenarioCameraMarker>>();
        query.iter(world).next()
    };
    if let Some(camera) = camera {
        world.entity_mut(camera).remove::<ScriptedCameraPose>();
    }
}

/// Engage the travel computer on the survey beacon - the same [`Autopilot`] the
/// G keybind inserts, engaged straight over the held orbit (the component is
/// replaced, which is what a pilot changing verbs does).
#[cfg(feature = "debug")]
fn engage_goto(world: &mut World) {
    let (Some(player), Some(beacon)) = (player_root(world), entity_by_id(world, BEACON_ID)) else {
        warn!("flight: no player or beacon to engage the travel computer on");
        return;
    };
    world
        .entity_mut(player)
        .insert(Autopilot::engage(AutopilotAction::Goto { target: beacon }));
    info!("flight: GOTO engaged on the survey beacon");
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
        braking && autopilot_phase(world) == Some(AutopilotPhase::Burn)
    })
}

/// Advance once the leg is essentially over: inside the arrival standoff
/// (`FlightSettings::arrival_standoff` is 50 units, and the telemetry's distance
/// is to the target SURFACE), with the closing speed off the top of the brake.
#[cfg(feature = "debug")]
fn player_arrived() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        maneuver(world)
            .is_some_and(|telemetry| telemetry.distance < 80.0 && telemetry.closing_speed < 60.0)
    })
}

/// The live numbers of the player's engaged leg, if there is one.
#[cfg(feature = "debug")]
fn maneuver(world: &World) -> Option<&ManeuverTelemetry> {
    world.get::<ManeuverTelemetry>(player_root_ref(world)?)
}

/// Advance once the insertion burn is actually lit.
#[cfg(feature = "debug")]
fn orbit_burning() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| autopilot_phase(world) == Some(AutopilotPhase::Burn))
}

/// Advance once the ship is circularized: ORBIT reports `Hold` when the velocity
/// error is inside the hold tolerance (`flight/autopilot.rs`), which is the
/// engine's own definition of "on the ring" and better than any settling time
/// this file could guess.
#[cfg(feature = "debug")]
fn orbit_holding() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| autopilot_phase(world) == Some(AutopilotPhase::Hold))
}

/// The phase of the player's engaged maneuver, if there is one.
#[cfg(feature = "debug")]
fn autopilot_phase(world: &World) -> Option<AutopilotPhase> {
    Some(world.get::<Autopilot>(player_root_ref(world)?)?.phase)
}

/// Where the ship actually is right now; its spawn point if it has gone.
#[cfg(feature = "debug")]
fn ship_position(world: &mut World) -> Vec3 {
    player_root(world)
        .and_then(|player| world.get::<GlobalTransform>(player))
        .map(|transform| transform.translation())
        .unwrap_or_else(start_position)
}

/// The direction the ship is travelling: its velocity, because on a ring the
/// track and the nose are not the same thing (the hull leads its own turn), and
/// the chase camera is a camera on the TRACK. Falls back to the hull's forward
/// while the ship is still at rest.
#[cfg(feature = "debug")]
fn ship_heading(world: &mut World) -> Vec3 {
    let Some(player) = player_root(world) else {
        return Vec3::NEG_Z;
    };
    let velocity = world
        .get::<avian3d::prelude::LinearVelocity>(player)
        .map(|velocity| velocity.0)
        .unwrap_or(Vec3::ZERO);
    velocity.try_normalize().unwrap_or_else(|| {
        world
            .get::<GlobalTransform>(player)
            .map(|transform| transform.forward().as_vec3())
            .unwrap_or(Vec3::NEG_Z)
    })
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
