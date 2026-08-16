//! screenshot_combat: "Rock hollow" - the travel, combat and HUD set.
//!
//! One flight in two acts, flown by the script the way a player would fly it.
//!
//! ACT 1, the leg: the corvette sits in open space with a nav beacon roughly 750
//! units downrange. The weapons-lowered radar latches it (a TRAVEL lock, far
//! enough out that its bracket sits in open sky instead of on top of the
//! player's own hull), the travel computer engages, and the ship flies the real
//! GOTO leg - align, burn, coast, FLIP, brake. It SHIPS NOTHING: the leg's two
//! shots (`feature-autopilot`, `wiki-flight`) moved to `screenshot_flight`,
//! which flies the same verb around a real gravity well with a camera that flies
//! the leg with the ship (owner's pick, 2026-08-05). The approach stays because
//! it is how the player reaches the hollow, and the flip is where the script
//! cuts.
//!
//! ACT 2, the ambush: the beacon doubles as its own trigger area, so crossing it
//! fires `OnEnter` and spawns the whole fight - a raider dead ahead, two
//! friendly corvettes on the near flanks, a hostile pair across the hollow and a
//! friendly torpedo boat off the raider's far quarter. That is scenario data,
//! not script, so the OWNER's plain run gets the ambush too: fly to the beacon
//! and the hollow fills up. The rest of the set is as it was - travel lock
//! cleared, weapons raised, combat lock latched, guns live.
//!
//! The scripted run CUTS between the acts (see [`cut_to_hollow`]): the flip is
//! the last thing worth shooting on a GOTO, so the script ends the leg there and
//! sets the ship down on station inside the trigger, rather than filming half a
//! minute of braking and opening the fight with the player still on autopilot.
//! The owner's plain run flies the approach out; the cut is an edit, and only
//! the capture makes it.
//!
//! ACT 3, the ordnance: the capture replaces the live fight with a private
//! in-memory scenario containing a fresh raider and torpedo boat. The boat
//! looses a salvo at the raider, guided and fuzed by the production torpedo
//! path. Scenario replacement removes prior rounds, debris, and actors before
//! this shot. It exists because the set's juice was one graded hull on one ship
//! - a launch burst, a drive plume and a 30-unit blast sphere eating sections
//! belong to a SECOND ship, which is what makes the hollow read as a fight. Two of its frames SHIP: `wiki-combat-torpedo` (the
//! salvo in flight) and `wiki-combat-aftermath` (what the blast left), which the
//! combat wiki page carries under Torpedoes and Damage types (owner's pick,
//! 2026-08-05). The tight exchange and hollow neutralized-wreck marker stay
//! `variant-*.png` shots - staged by a capture run, named by no manifest entry.
//!
//! WHAT THE PROOF RUN SHOWED (2026-08-05), and why the fight is shaped like
//! this: two AI flights DO fight each other with no player in the scene - a
//! flight spawned `allegiance: Some(Player)` acquires the default-Enemy flight
//! within a second and both sides run their guns continuously. Nothing in
//! acquisition is player-specific (`crates/nova_gameplay/src/input/ai/
//! acquisition.rs`), and now it has been run. The engage maneuver flies to
//! `AI_STANDOFF_RANGE` and EXTENDS AWAY once inside the band, so a fight is a
//! ring rather than a brawl. That ring used to be 250 units against rounds
//! that died at 500, and 45 seconds of continuous fire took zero sections
//! off; since the v0.11.0 range pass it is 100 units against rounds that
//! reach 200. Re-measured here 2026-08-15: the pairs open fire between 69 and
//! 180 units, i.e. inside their own gate rather than at the edge of it. They
//! are still only the set's live background - tracer streams and moving hulls
//! - and every close beat stays authored: the lock subject is not AI-flown (it
//! drifts on a nudged velocity), and the destruction is scripted through the
//! real damage path, so the capture is deterministic instead of hostage to a
//! fight.
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
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also capture the shots (staged under
//!   `NOVA_SHOT_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_SHOT_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_combat --features debug
//! ```
//!
//! No-display correctness run:
//! ```text
//! env -u DISPLAY -u WAYLAND_DISPLAY cargo run --features debug -- \
//!   probe run screenshot_combat --correctness-only
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
/// Private scenario ids divide the capture into deterministic chapters.
const APPROACH_CHAPTER_ID: &str = "rock_hollow";
/// Only the script loads the second chapter, so it exists with the script.
#[cfg(feature = "debug")]
const ORDNANCE_CHAPTER_ID: &str = "rock_hollow_ordnance";
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
#[cfg(feature = "debug")]
/// The raider section the scripted blow takes off - the semantic nose part is
/// forward and camera-facing, so the fragments and the hole are both in frame.
const RAIDER_BLOWN_SECTION: &str = "nose";
#[cfg(feature = "debug")]
/// The section the torpedo beat takes off on the raider's port side, where a
/// blast arriving from above lands. A torpedo alone will NOT do this: the fuze
/// goes 15 units out, and 100 blast damage with falloff at half the blast radius
/// leaves a 70-100 health section standing. The frame is of a real detonation
/// and a real section death, timed together.
const RAIDER_BLAST_SECTION: &str = "wing_port";

/// Scenario id of the friendly torpedo boat - the only hull in the set carrying
/// torpedo pods, and the ship the ordnance beats are shot off.
const LANCE_ID: &str = "hollow_lance";
/// Where it sits: high and off the raider's far quarter, so the run comes DOWN
/// onto the target - and, the reason for the height, through open sky. The rock
/// shell is 46 units thick in Y, and a torpedo fired across the hollow at the
/// shell's own height flies into a rock: this bearing clears it. It is also what
/// keeps the blast (30 units across) off the player, parked 34 from the raider.
const LANCE_POSITION: Vec3 = Vec3::new(-38.0, 30.0, -56.0);
/// How far short of its target a torpedo detonates: the proximity fuze fires at
/// half the bay's blast radius (`torpedo_section/projectile.rs`), and the
/// cargo-B's bays are authored at 30. The ordnance camera is framed off this,
/// not off the raider - 15 units is a third of the frame at a close camera.
#[cfg(feature = "debug")]
const TORPEDO_FUZE_RANGE: f32 = 15.0;
#[cfg(feature = "debug")]
const EXPECTED_TORPEDO_COUNT: usize = 2;

/// Seconds each AI flight holds fire after it spawns, so the shots are taken of
/// a fight that has settled rather than of four ships still sorting out where
/// they are.
const ENGAGE_DELAY: f32 = 3.0;
/// How far an AI ship may stray from its post before it breaks off and comes
/// back. Wider than the standoff range the engage maneuver flies to (100), so
/// the fight is not permanently interrupted, tight enough that the hollow keeps
/// its ships instead of watching them leave.
const AI_LEASH: f32 = 320.0;

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
                .on_enter(move |world| shoot(world, "tutorial-radar-lock.png"))
                .until(shot_written("tutorial-radar-lock.png"))
                .deadline(SHOT_DEADLINE_SECS)
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
                .on_enter(move |world| shoot(world, "wiki-radar.png"))
                .until(shot_written("wiki-radar.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                // Hand the camera back before the leg: it is flown over the
                // game's own follow camera, because a pinned world pose watches a
                // ship that is leaving.
                //
                // The leg SHOOTS NOTHING. It used to stage `feature-autopilot`
                // (the burn) and `wiki-flight` (the flip-and-burn); both moved to
                // `screenshot_flight`, which flies the same verb around a real
                // gravity well with a camera that flies the leg with the ship
                // (owner's pick, 2026-08-05). The leg itself stays: this act is
                // the approach that puts the player in the hollow, and the flip
                // is where the script cuts.
                .step("engage the travel computer")
                .on_enter(|world| {
                    release_radar(world);
                    unpose(world);
                    engage_goto(world);
                })
                .until(player_burning())
                .deadline(25.0)
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
                // ACT 2 - the ambush. The flip is the last thing worth shooting
                // on the leg, so the script CUTS here rather than sitting
                // through the rest of the brake: the player is set down on
                // station inside the beacon's trigger, which springs the ambush
                // (`OnEnter`, scenario data) with the ship already parked. The
                // owner's plain run still flies the whole approach - this is an
                // edit, and only the scripted run makes it.
                .step("cut to the hollow")
                .on_enter(cut_to_hollow)
                // A frame for the disengage's camera handback to run, so
                // [`hold_station`] re-seeds the rig AFTER it and not before.
                .until(frames(4))
                .add()
                // Station-keeping starts HERE, not at load: the leg needs the
                // ship free to fly. See [`pin_player`] for why the combat act
                // cannot tolerate drift. Waiting on the RAIDER, because the
                // ambush is the trigger's to spring: a cut that landed outside
                // the area aborts this step by name instead of shooting an
                // empty hollow.
                .step("hold station in the hollow")
                .on_enter(|world| {
                    hold_station(world);
                    // The travel lock keeps its white bracket and GOTO chip on
                    // screen, and the combat shots are about the RED lock - two
                    // locks in one frame is chrome competing with itself.
                    clear_travel_lock(world);
                })
                .until(raider_present())
                .deadline(10.0)
                .add()
                // Only once the ambush is in the world: the beacon is the
                // trigger that spawned it, so it goes after it has fired.
                .step("let the ambush settle")
                .on_enter(|world| {
                    despawn_by_id(world, BEACON_ID);
                    nudge_raider(world);
                })
                .until(elapsed(ENGAGE_DELAY + 1.5))
                .add()
                // Quiet stance first: weapons lowered, no lock, the contextual
                // rules keeping idle chrome off the frame. That IS the shot.
                .step("capture the contextual HUD")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, "news-090-contextual-hud.png");
                })
                .until(shot_written("news-090-contextual-hud.png"))
                .deadline(SHOT_DEADLINE_SECS)
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
                    shoot(world, "feature-combat.png");
                })
                .until(shot_written("feature-combat.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                .step("capture the combat lock")
                .on_enter(move |world| {
                    hud_instrument(world);
                    shoot(world, "tutorial-combat-lock.png");
                })
                .until(shot_written("tutorial-combat-lock.png"))
                .deadline(SHOT_DEADLINE_SECS)
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
                    shoot(world, "feature-hud.png");
                })
                .until(shot_written("feature-hud.png"))
                .deadline(SHOT_DEADLINE_SECS)
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
                    shoot(world, "wiki-hud.png");
                })
                .until(shot_written("wiki-hud.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                // The wide beats: the HUD goes cinematic and the camera leaves
                // the player, so the chrome would be reading a ship the frame
                // is no longer with. They are framed on the PLAYER's tracer
                // stream, not on the AI pairs: the AI holds a 100-unit standoff
                // well outside this framing, while the player's fire crosses
                // 34 units of open hollow into a hull.
                .step("frame the exchange")
                .on_enter(|world| {
                    hud_cinematic(world);
                    pose(world, Vec3::new(9.0, 2.5, 6.0), Vec3::new(0.0, 0.3, -14.0));
                })
                .until(elapsed(0.5))
                .add()
                .step("capture the exchange")
                .on_enter(move |world| shoot(world, "wiki-combat.png"))
                .until(shot_written("wiki-combat.png"))
                .deadline(SHOT_DEADLINE_SECS)
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
                .on_enter(move |world| shoot(world, "news-090-combat-readability.png"))
                .until(shot_written("news-090-combat-readability.png"))
                .deadline(SHOT_DEADLINE_SECS)
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
                .on_enter(|world| blow_raider_section(world, RAIDER_BLOWN_SECTION))
                .until(frames(12))
                .add()
                .step("capture the juice")
                .on_enter(move |world| shoot(world, "feature-juice.png"))
                .until(shot_written("feature-juice.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                // ACT 3 - the ordnance. Replace the live battle with a private
                // scenario chapter. Scenario ownership removes the old cast,
                // rounds, debris, and effects; the new chapter contains only
                // the player camera rig, raider, lance, hollow, and lighting.
                .step("load the ordnance chapter")
                .on_enter(load_ordnance_chapter)
                .until(ordnance_chapter_ready())
                .deadline(15.0)
                .add()
                .step("settle the ordnance chapter")
                .on_enter(assert_ordnance_chapter)
                .until(frames(12))
                .add()
                // A dead section is a graded hull; a torpedo is the engine's loudest
                // effect (launch burst, drive plume, a 30-unit blast sphere
                // eating sections), and it belongs to a SECOND ship, which is
                // what makes the hollow read as a fight rather than a duel.
                // Everything here is shot to `variant-*.png`, which the
                // packaging manifest does not name: these are candidates for
                // `feature-juice` and `wiki-combat`, to be picked at capture
                // time, not extra shipped images.
                // One camera for both ordnance frames, so the pick is a
                // before/after of the same shot. It is framed on the midpoint
                // between the raider and where the fuze will go, NOT on the
                // raider: a proximity fuze detonates 15 units short of its
                // target, which at a close camera throws the blast a third of
                // the way across the frame from the ship it is hitting. It
                // stands on the far side from the boat, so the run comes down
                // the lens and the blast opens behind the target, and far
                // enough back that the 30-unit sphere stays in front of it.
                .step("frame the torpedo run")
                .on_enter(|world| {
                    let subject = ordnance_subject(world);
                    // From BELOW, looking up the run. The rock field is a
                    // horizontal annulus 46 units thick, so any level camera in
                    // the hollow frames its subject against the far wall and
                    // the shot is rock soup; tipping the lens up puts open sky
                    // behind the target and the torpedo dives into frame.
                    pose(world, subject + Vec3::new(16.0, -14.0, 12.0), subject)
                })
                .until(elapsed(0.4))
                .add()
                .step("loose the torpedoes")
                .on_enter(loose_torpedoes)
                .until(torpedo_salvo_in_flight(EXPECTED_TORPEDO_COUNT))
                .deadline(6.0)
                .add()
                // The salvo is committed to the raider the frame after launch,
                // the way both production commit systems do it, and the trigger
                // drops so the boat fires once.
                .step("commit the salvo")
                .on_enter(commit_torpedoes)
                .until(elapsed(0.1))
                .add()
                // Inbound: a beat before the fuze, with the drive still lit and
                // the target intact.
                .step("track the torpedoes in")
                .each(assert_salvo_still_live)
                .until(torpedo_within(TORPEDO_FUZE_RANGE + 8.0))
                .deadline(12.0)
                .add()
                .step("capture the torpedo run")
                .on_enter(move |world| shoot(world, "wiki-combat-torpedo.png"))
                .until(shot_written("wiki-combat-torpedo.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                .step("wait for the detonation")
                .until(no_torpedo_in_flight())
                .deadline(8.0)
                .add()
                // The AFTERMATH, not the flash. The detonation itself is not
                // worth a beat: its blast visual is a solid 60-unit sphere on a
                // 0.1 s temp entity (`insert_blast_radius_visual`), which at 30
                // fps is a frame or two of flat orange filling any camera close
                // enough to see the ship it hit. Half a second later the sphere
                // is gone and what it did is not - the particle burst, the
                // tumbling debris and a hull short some sections.
                .step("let the blast clear")
                .on_enter(|world| {
                    blow_raider_section(world, RAIDER_BLAST_SECTION);
                    // Re-framed off the LIVE raider: it has been drifting and
                    // taking rounds since the run was framed, and a rock in the
                    // wall behind it is one drift away from being in front of
                    // it.
                    let raider = raider_position(world);
                    pose(world, raider + Vec3::new(16.0, -14.0, 12.0), raider)
                })
                .until(elapsed(0.5))
                .add()
                .step("capture the aftermath")
                .on_enter(move |world| shoot(world, "wiki-combat-aftermath.png"))
                .until(shot_written("wiki-combat-aftermath.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                // The exchange again, tighter and lower: the wide `wiki-combat`
                // framing reads as a diorama at the site's 16:9 crop, so this is
                // the same beat shot as a gun camera would see it.
                .step("frame the exchange tight")
                .on_enter(|world| {
                    let raider = raider_position(world);
                    // Down the player's own fire line and tipped up, so the
                    // target reads against sky rather than against the far wall
                    // of the hollow.
                    pose(world, raider + Vec3::new(7.0, -5.0, 18.0), raider)
                })
                .until(elapsed(0.4))
                .add()
                .step("capture the tight exchange")
                .on_enter(move |world| shoot(world, "variant-combat-tight.png"))
                .until(shot_written("variant-combat-tight.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
                // Presentation pin: a neutralized enemy remains in frame and
                // lockable, but its solid threat triangle becomes a hollow
                // wreck chevron.
                .step("mark the raider neutralized")
                .on_enter(|world| {
                    hud_instrument(world);
                    if let Some(raider) = raider_root(world) {
                        world.entity_mut(raider).insert(NeutralizedMarker);
                    }
                })
                .until(elapsed(0.2))
                .add()
                .step("capture the neutralized wreck marker")
                .on_enter(move |world| shoot(world, "variant-neutralized-wreck.png"))
                .until(shot_written("variant-neutralized-wreck.png"))
                .deadline(SHOT_DEADLINE_SECS)
                .add(),
        );
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        // Only under the script (`NOVA_AUTOPILOT`, named literally - the
        // harness re-exports `CAPTURE_ENV` but not the autopilot's own): a
        // plain run is the owner flying this set, and a pinned ship cannot be
        // flown.
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.add_systems(Update, pin_player.run_if(resource_exists::<HoldStation>));
        }
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(rock_hollow(&game_assets, &sections)));
}

#[cfg(feature = "debug")]
fn load_ordnance_chapter(world: &mut World) {
    let game_assets = world.resource::<GameAssets>().clone();
    let sections = world.resource::<GameSections>().clone();
    world.remove_resource::<HoldStation>();
    world.trigger(LoadScenario(ordnance_chapter(&game_assets, &sections)));
    info!("combat: loading scenario chapter `{ORDNANCE_CHAPTER_ID}`");
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
            input_mapping: turret_bindings(sections, "cargoa"),
            speed_cap: None,
            // The player holds fire through several beats; running dry
            // mid-capture would leave a reload where the tracers should be.
            infinite_ammo: true,
        }),
        None,
        kit::kenney_hull(sections, "cargoa"),
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
    // The hollow itself - and it is a HOLLOW: the field starts outside the
    // raider's station (34 units) with room to spare, so the pocket the fight
    // happens in is clear and the rocks read as the wall around it. Tried
    // tighter (28 units): rocks land on the raider, every close framing has one
    // in front of the subject, and a torpedo run into it hits stone.
    let shell = kit::NearField {
        id_prefix: "hollow_rock_",
        count: 48,
        seed: 40507,
        distance: (48.0, 130.0),
        radius: (1.2, 3.2),
        y_spread: 46.0,
    };

    ScenarioConfig {
        description: "A run into a rock field, and the ambush waiting in it.".to_string(),
        events: vec![
            ScenarioEventConfig {
                name: EventConfig::OnStart,
                filters: vec![],
                // The photo rig, now authored content rather than an
                // example-side observer swap: scale 1.0 around the origin
                // reproduces the kit's exact key/rim/fill numbers, so the
                // captured frames are unchanged.
                actions: [
                    vec![
                        corridor.action(game_assets),
                        shell.action(game_assets),
                        player,
                        beacon(),
                    ],
                    ThreePointRig::around("photo", Vec3::ZERO, 1.0).actions(),
                ]
                .concat(),
            },
            ambush(sections),
        ],
        ..ScenarioConfig::new(
            APPROACH_CHAPTER_ID.to_string(),
            "Rock Hollow".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Private ordnance chapter: the same hollow, but no unrelated combatants or
/// live guns. Loading it replaces every scenario-scoped transient from the gun
/// exchange instead of trying to enumerate and clean them in the script.
///
/// Only `load_ordnance_chapter` builds it, and only the script calls that, so
/// it is script-only like every other beat helper here.
#[cfg(feature = "debug")]
fn ordnance_chapter(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let player = ship(
        PLAYER_ID,
        "Player Ship",
        Vec3::ZERO,
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::new(),
            speed_cap: None,
            infinite_ammo: true,
        }),
        None,
        kit::kenney_hull(sections, "cargoa"),
    );
    let raider = ship(
        RAIDER_ID,
        "Raider",
        RAIDER_POSITION,
        Quat::from_rotation_y(std::f32::consts::PI - 0.4),
        SpaceshipController::None,
        Some(Allegiance::Enemy),
        kit::kenney_hull(sections, "cargoa"),
    );
    let lance = ship(
        LANCE_ID,
        "Lance",
        LANCE_POSITION,
        Transform::from_translation(LANCE_POSITION)
            .looking_at(RAIDER_POSITION, Vec3::Y)
            .rotation,
        SpaceshipController::None,
        Some(Allegiance::Player),
        kit::kenney_hull(sections, "cargob"),
    );
    let shell = kit::NearField {
        id_prefix: "ordnance_rock_",
        count: 48,
        seed: 40507,
        distance: (48.0, 130.0),
        radius: (1.2, 3.2),
        y_spread: 46.0,
    };

    ScenarioConfig {
        description: "The controlled ordnance chapter of Rock Hollow.".to_string(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: [
                vec![shell.action(game_assets), player, raider, lance],
                ThreePointRig::around("photo", Vec3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            ORDNANCE_CHAPTER_ID.to_string(),
            "Rock Hollow - Ordnance".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The ambush: everything that fights, spawned when the player crosses the
/// beacon's trigger area.
///
/// Scenario data rather than script on purpose - this is the engine's own
/// `OnEnter` ambush pattern, and it means the owner's plain run gets the fight
/// by flying to the beacon, not only the scripted capture run.
fn ambush(sections: &GameSections) -> ScenarioEventConfig {
    // The lock subject: not AI, because an AI hostile flies to a 100-unit
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
        kit::kenney_hull(sections, "cargoa"),
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
        kit::kenney_hull(sections, "cargoa"),
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
        kit::kenney_hull(sections, "cargoa"),
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
        kit::kenney_hull(sections, "cargoa"),
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

    // The torpedo boat: a cargo-B, which is the only Kenney hull in the catalog
    // with launch bays. Posed, not AI - the AI's own launch envelope opens at
    // 3x the blast radius and its cadence is a 10-second playtest knob, so a
    // capture that waited for it would be waiting on a coin flip. The script
    // pulls the trigger instead ([`loose_torpedoes`]) and the bay, the
    // projectile, the guidance and the blast are all the production path.
    let lance = ship(
        LANCE_ID,
        "Lance",
        LANCE_POSITION,
        Transform::from_translation(LANCE_POSITION)
            .looking_at(RAIDER_POSITION, Vec3::Y)
            .rotation,
        SpaceshipController::None,
        Some(Allegiance::Player),
        kit::kenney_hull(sections, "cargob"),
    );

    ScenarioEventConfig {
        name: EventConfig::OnEnter,
        filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
            id: Some(BEACON_ID.to_string()),
            other_id: Some(PLAYER_ID.to_string()),
            ..Default::default()
        })],
        actions: vec![raider, wingman_a, wingman_b, hostile_a, hostile_b, lance],
    }
}

/// Bind every turret section of a built hull to the trigger, the way the
/// shipped scenarios do (`shakedown_run` maps its two corvette turret cubes to
/// `Mouse(Left)` + `Gamepad(RightTrigger2)`).
///
/// Derived from the catalog rather than typed out, for the same reason
/// [`kit::kenney_hull`] is: the ids ARE the layout, and a hand-listed pair goes
/// stale the moment a hull gains a gun. The map is keyed by INSTANCE id
/// (`nova_scenario` snapshots bindings by section id at spawn), which for a
/// [`kit::kenney_hull`] build is the prototype id minus the `<hull>_` prefix;
/// the `_light` catalog variants are not part of any built hull, so they are
/// skipped.
fn turret_bindings(sections: &GameSections, hull: &str) -> HashMap<String, Vec<Binding>> {
    let prefix = format!("{hull}_");
    sections
        .iter()
        .filter(|section| matches!(section.kind, SectionKind::Turret(_)))
        .filter_map(|section| section.base.id.strip_prefix(&prefix))
        .filter(|id| !id.ends_with("_light"))
        .map(|id| {
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
            hull: ShipSource::Inline(ShipHull {
                sections,
                ..default()
            }),
            ..default()
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

/// Pin the camera for a framing the follow camera does not give.
#[cfg(feature = "debug")]
fn pose(world: &mut World, position: Vec3, look_at: Vec3) {
    pose_camera(world, position, look_at);
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
/// from that attitude (`camera/handback.rs`) - so pinning the hull
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

/// End the leg early: drop the travel computer and set the ship down on
/// station.
///
/// The cut the module docs describe. Everything worth shooting on a GOTO is over
/// once the flip has lit the retro burn - what follows is half a minute of
/// braking with nothing new in frame - so the scripted run ends the leg here
/// instead of flying it out. Dropping [`Autopilot`] is the same disengage the
/// computer performs at the goal (the camera handback keys off its removal), and
/// the ship is placed at the hollow's origin, INSIDE the beacon's trigger area,
/// so the ambush springs the way it would have on arrival.
#[cfg(feature = "debug")]
fn cut_to_hollow(world: &mut World) {
    let Some(player) = player_root(world) else {
        warn!("combat: no player to cut to the hollow");
        return;
    };
    world
        .entity_mut(player)
        .remove::<Autopilot>()
        .remove::<ManeuverTelemetry>();
    if let Some(mut transform) = world.entity_mut(player).get_mut::<Transform>() {
        transform.translation = Vec3::ZERO;
        transform.rotation = Quat::IDENTITY;
    }
    if let Some(mut linear) = world
        .entity_mut(player)
        .get_mut::<avian3d::prelude::LinearVelocity>()
    {
        linear.0 = Vec3::ZERO;
    }
    if let Some(mut angular) = world
        .entity_mut(player)
        .get_mut::<avian3d::prelude::AngularVelocity>()
    {
        angular.0 = Vec3::ZERO;
    }
    info!("combat: cut to the hollow, the leg ends here");
}

/// Advance once the ambush has actually spawned - the raider is the beat's
/// subject and the last of the flight to matter, so its presence is the proof
/// the beacon's trigger fired.
#[cfg(feature = "debug")]
fn raider_present() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&EntityId, With<SpaceshipRootMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|id| id.0 == RAIDER_ID))
    })
}

#[cfg(feature = "debug")]
fn ordnance_chapter_ready() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let current = world.resource::<CurrentScenario>();
        if current
            .as_ref()
            .is_none_or(|scenario| scenario.id != ORDNANCE_CHAPTER_ID)
        {
            return false;
        }
        let Some(mut ships) = world.try_query_filtered::<&EntityId, With<SpaceshipRootMarker>>()
        else {
            return false;
        };
        let ids: Vec<&str> = ships.iter(world).map(|id| id.0.as_str()).collect();
        ids.len() == 3
            && [PLAYER_ID, RAIDER_ID, LANCE_ID]
                .iter()
                .all(|required| ids.contains(required))
            && world
                .try_query_filtered::<Entity, With<TorpedoSectionMarker>>()
                .is_some_and(|mut bays| bays.iter(world).count() == 2)
            && world
                .try_query_filtered::<Entity, With<TurretBulletProjectileMarker>>()
                .is_none_or(|mut bullets| bullets.iter(world).next().is_none())
            && world
                .try_query_filtered::<Entity, With<TorpedoProjectileMarker>>()
                .is_none_or(|mut torpedoes| torpedoes.iter(world).next().is_none())
    })
}

#[cfg(feature = "debug")]
fn assert_ordnance_chapter(world: &mut World) {
    let current = world.resource::<CurrentScenario>();
    assert_eq!(
        current.as_ref().map(|scenario| scenario.id.as_str()),
        Some(ORDNANCE_CHAPTER_ID),
        "combat: ordnance cast settled under the wrong scenario"
    );
    let ids: Vec<String> = world
        .query_filtered::<&EntityId, With<SpaceshipRootMarker>>()
        .iter(world)
        .map(|id| id.0.clone())
        .collect();
    assert_eq!(
        ids.len(),
        3,
        "combat: ordnance chapter must have three ships: {ids:?}"
    );
    for required in [PLAYER_ID, RAIDER_ID, LANCE_ID] {
        assert!(
            ids.iter().any(|id| id == required),
            "combat: ordnance chapter is missing `{required}`"
        );
    }
    info!("combat: scenario chapter `{ORDNANCE_CHAPTER_ID}` ready");
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
fn blow_raider_section(world: &mut World, section: &str) {
    let Some(node) = raider_section_health(world, section) else {
        warn!("combat: no health node under section '{section}' to blow");
        return;
    };
    world.trigger(HealthApplyDamage {
        entity: node,
        source: None,
        amount: 1.0e6,
    });
    info!("combat: blew '{section}' off the raider");
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
    ship_by_id(world, RAIDER_ID)
}

/// Where the raider actually is right now; its spawn point if it has gone.
#[cfg(feature = "debug")]
fn raider_position(world: &mut World) -> Vec3 {
    raider_root(world)
        .and_then(|raider| world.get::<GlobalTransform>(raider))
        .map(|transform| transform.translation())
        .unwrap_or(RAIDER_POSITION)
}

/// Pull the torpedo boat's triggers: every bay on the lance, fired at once, so
/// the beat is a salvo rather than a single round.
///
/// The bays are the ship root's own children (which is how the AI's launch
/// system finds them), and writing [`TorpedoSectionInput`] is exactly what the
/// player's trigger observer and the AI's envelope do - from here on the launch
/// is the production path.
#[cfg(feature = "debug")]
fn loose_torpedoes(world: &mut World) {
    let Some(lance) = ship_by_id(world, LANCE_ID) else {
        warn!("combat: no torpedo boat to fire");
        return;
    };
    let bays: Vec<Entity> = world
        .query_filtered::<(Entity, &ChildOf), With<TorpedoSectionMarker>>()
        .iter(world)
        .filter(|(_, parent)| parent.parent() == lance)
        .map(|(bay, _)| bay)
        .collect();
    if bays.is_empty() {
        warn!("combat: the torpedo boat has no bays");
        return;
    }
    for bay in &bays {
        if let Some(mut input) = world.entity_mut(*bay).get_mut::<TorpedoSectionInput>() {
            **input = true;
        }
    }
    info!("combat: {} torpedo bay(s) firing", bays.len());
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
fn commit_torpedoes(world: &mut World) {
    let Some(raider) = raider_root(world) else {
        warn!("combat: no raider to commit the salvo to");
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
        EXPECTED_TORPEDO_COUNT,
        "combat: ordnance chapter must commit the complete salvo"
    );
    for torpedo in &torpedoes {
        world
            .entity_mut(*torpedo)
            .insert((TorpedoTargetChosen, TorpedoTargetEntity(raider)));
    }
    info!(
        "combat: {} torpedo(es) committed to the raider",
        torpedoes.len()
    );
}

/// What the ordnance frames are about: the midpoint of the raider and the point
/// the fuze will go off at, which is [`TORPEDO_FUZE_RANGE`] short of it along
/// the boat's bearing. Framing on the raider alone puts the blast at the edge of
/// the frame; framing on this holds both.
#[cfg(feature = "debug")]
fn ordnance_subject(world: &mut World) -> Vec3 {
    let raider = raider_position(world);
    let bearing = (LANCE_POSITION - raider).normalize_or_zero();
    raider + bearing * (TORPEDO_FUZE_RANGE * 0.5)
}

/// Advance once a torpedo is actually in the world.
#[cfg(feature = "debug")]
fn torpedo_salvo_in_flight(
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
fn no_torpedo_in_flight() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| torpedo_range(world).is_none())
}

/// Advance once the leading torpedo is within `distance` of the raider.
#[cfg(feature = "debug")]
fn torpedo_within(distance: f32) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        torpedo_range(world).is_some_and(|range| range < distance)
    })
}

#[cfg(feature = "debug")]
fn assert_salvo_still_live(world: &mut World, _: f32) {
    let live = world
        .query_filtered::<Entity, With<TorpedoProjectileMarker>>()
        .iter(world)
        .count();
    assert!(
        live > 0,
        "combat: the complete torpedo salvo was lost before reaching the capture range"
    );
}

/// How far the closest live torpedo is from the raider, if there is one of each.
#[cfg(feature = "debug")]
fn torpedo_range(world: &World) -> Option<f32> {
    let raider = world
        .try_query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()?
        .iter(world)
        .find(|(_, id)| id.0 == RAIDER_ID)
        .map(|(entity, _)| entity)?;
    let target = world.get::<GlobalTransform>(raider)?.translation();
    world
        .try_query_filtered::<&GlobalTransform, With<TorpedoProjectileMarker>>()?
        .iter(world)
        .map(|transform| transform.translation().distance(target))
        .min_by(f32::total_cmp)
}

/// The ship root carrying scenario id `id`.
#[cfg(feature = "debug")]
fn ship_by_id(world: &mut World, id: &str) -> Option<Entity> {
    let mut query = world.query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>();
    query
        .iter(world)
        .find(|(_, live)| live.0 == id)
        .map(|(entity, _)| entity)
}

/// One of the raider's section entities, by prototype id. Picked BY SHIP: the
/// corvettes in the set share section ids, as every shipped multi-ship scenario
/// does.
#[cfg(feature = "debug")]
fn raider_section(world: &mut World, section: &str) -> Option<Entity> {
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
fn raider_section_health(world: &mut World, section: &str) -> Option<Entity> {
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
