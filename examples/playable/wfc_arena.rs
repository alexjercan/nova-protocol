//! wfc_arena: a roster of wave-function-collapse ships fights over a dressed
//! arena.
//!
//! The generator is `shared/wfc.rs`, the same collapse `wfc_ships` poses in a
//! row. Where the row neuters its subjects (`SpaceshipController::None`, no
//! allegiance), the arena flips exactly those two fields: some hulls fly the
//! player's colors, the rest the enemy's, all under the campaign's AI pilot.
//! This is the flyability bench for wfc ships - thrust against a collapsed
//! hull's mass, turret arcs on a random silhouette, torpedo lanes that were
//! only ever checked geometrically. After collapse, two arena-only stamps go
//! on outside the grammar: a seeded stern stamp fitting one capital drive or
//! two to three vector drives, and a bow stamp bolting a SPINAL LANCE to the
//! keel cell at the nose. Both are simple PoCs - one for judging large
//! propulsion, one for benching a gun the whole ship aims - and neither is the
//! future game's ship grammar.
//!
//! Combatants are DRAFTED: the collapse arms hulls with wild variance, so the
//! arena walks the seed stream from its head and fields the first hulls that
//! clear an armament floor ([`MIN_TURRETS`], [`MIN_BAYS`]). A hand-run rolls a
//! fresh head and logs it; `--seed` pins one, and a scripted run stays on
//! [`DEFAULT_SEED`] ([`resolve_seed`]). Reproducible rather than fixed - one
//! seed and one roster reproduce the matchup - and every skipped seed is logged
//! with the armament that disqualified it. Half of every hull's tubes carry
//! Lances ([`load_lances`]), which stages the owner's decoy doctrine: Serpents
//! drain the defender's point defense and the Lance behind them arrives on a
//! straight line into spent guns.
//!
//! # Roster
//!
//! `--ship` is repeatable and carries one hull each:
//!
//! ```text
//! --ship TEAM[:STYLE[:SEED]][:player]
//! ```
//!
//! - `TEAM` is `amber` or `onyx` ([`TEAMS`]), the only required field.
//! - `STYLE` is a style id. `--style` initializes both sides; explicit ship
//!   styles apply in command order, so the last one for a side wins.
//! - `SEED` pins that hull's collapse. A pin is an instruction: the hull spawns
//!   as rolled even under the armament floor, and the log says so.
//! - `player` puts YOU in that hull (at most one slot).
//!
//! With no `--ship` the roster is one drafted hull per team; a probe
//! measurement pass fields [`MEASURED_SHIPS_PER_TEAM`] instead.
//!
//! A `:player` slot spawns under the game's real player controller, and the
//! example's own cameras stand down - no `Q`/`E`/`1-4`, no idle orbit, the
//! chase-camera authority owns the view. REFUSED under `NOVA_AUTOPILOT`: a hull
//! waiting on human input would stall the fight predicate into its deadline, so
//! a run that asks for both fails loudly. Its guns come bound
//! ([`player_bindings`]): turrets on the left mouse, tubes on `F`, the bow
//! lance on `R`.
//!
//! ```text
//! cargo run --example wfc_arena --features debug
//! cargo run --example wfc_arena --features debug -- --seed 7 --style salvage
//! cargo run --example wfc_arena --features debug -- \
//!     --ship amber --ship amber:salvage --ship onyx --ship onyx --ship onyx
//! cargo run --example wfc_arena --features debug -- --ship amber::7 --ship onyx
//! cargo run --example wfc_arena --features debug -- --ship amber:player --ship onyx
//! ```
//!
//! # The match
//!
//! A hand-run opens the match configurator: per-side style, seed edit and
//! reroll, up to four ships a side, at most one player slot. Escape freezes a
//! live match for Resume, exact Restart, Return to Lobby and Quit. A player can
//! rebind a bindable section from NOVA OS `ship` with `B`; conflicts are
//! refused and accepted overrides survive restart.
//!
//! The lines spawn ~3.05 km apart and fly in COLD: an arrival grace holds both
//! teams on their center-crossing patrols, and the weapons-free gate
//! ([`ENGAGE_RANGE`]) keeps them passive until the closing lines are inside it.
//! From 2.8 km hot, one torpedo alpha strike decided the fight before a gun
//! bore.
//!
//! Scoring is PER TEAM. Rows come from projectile-carried `DamageType` variants
//! and authored torpedo names, so a new ammunition type needs no arena edit;
//! damage is counted through per-team pool deltas, which see the plate damage
//! the isolated-cladding rule keeps off the roots. Losing every live flight
//! computer freezes the fight and opens the result board. A 180-second global
//! inactivity window resolves a deadlock by remaining structure. Ships outside
//! the 200 km sphere are warned for 30 seconds, then lose their flight
//! computers.
//!
//! # Dressing and view
//!
//! A three-point rig, rock rings below the fight plane for parallax, one pinned
//! planetoid, and JUNK: wreckage fragments scattered into flank blobs
//! ([`DERELICT_BLOBS`]). Junk carries no controller and no allegiance so the AI
//! never targets it, and is pinned static ([`freeze_junk`]) because a
//! controller-less hull's massless body must never touch the physics the fight
//! reads.
//!
//! Every pose is computed off the live fight each frame, so a vantage keeps its
//! subject framed:
//!
//! - `Q` auto-framing whole-fight view (the default). Left alone for six
//!   seconds it falls into a slow orbit around the midpoint; a pose key or
//!   free-fly input stops it and restarts the clock.
//! - `E` tactical overview. It holds its bearing and only re-centres.
//! - `1`-`4` follow one roster slot over the shoulder, standing on the threat
//!   axis and looking across it at the living enemy mean (smoothed, because a
//!   kill moves the mean). An empty or dead slot falls back to the frame pose.
//!
//! `Q` and `E` sit clear of the free-fly rig (WASD, mouse, Space/Shift), so a
//! mode key never doubles as camera input. Grave/tilde cycles the game HUD.
//!
//! Combatants wear a TEAM CHEVRON: the HUD allegiance-marker's visual language,
//! redrawn here and tinted per team ([`Team::tint`]). Redrawn because the stock
//! widget's colours are semantic ally/threat, it marks every non-player ship
//! including the junk, and it is instrument-tier, so a cinematic capture would
//! hide it. The example tags the scenario camera as the indicator projector and
//! retires the stock allegiance layers for arena ships.
//!
//! Harnessed (`NOVA_AUTOPILOT=1`, plus `NOVA_CAPTURE=1` for the shot): wait for
//! the arena, hold until both teams have fired AND both have dealt damage, then
//! shoot the brawl mid-swing. The step deadline makes a fight that never
//! happens a loud failure.
//!
//! A CAPTURE then STAGES the strike ([`Strike`]): both sides put their bores on
//! each other and hold, the camera cuts in over one hull's shoulder, and every
//! tube and every lance is cued together. What is scripted is the cue and
//! the framing - the ordnance, the guidance, the point defense, the charge and
//! the damage are all the shipped ones. Without it the recording is what the AI
//! happens to be doing, and the AI settles its orbit at ~1 km: two specks
//! trading tracers, no impact and no lance in six seconds of film.

// Only for freezing the junk, which is a FRAMING choice - see `freeze_junk`.
use std::collections::BTreeMap;

use avian3d::prelude::RigidBody;
use bevy::prelude::*;
// The player slot's weapon bindings are authored in the game's own binding
// type, exactly as the campaign scenarios author theirs.
use clap::Parser;
use nova_debug::prelude::capturing;
// Direct, not through `nova_protocol::nova_debug`: that path only exists under
// the `debug` feature, and `capturing()` gates the idle orbit and chevrons
// in EVERY build.
use nova_input::prelude::InputSource;
use nova_protocol::prelude::*;
// The derelict dressing - fragment hulls, where they scatter, how they tumble -
// and the opening seed of an unpinned run. The FIGHT's hulls come off the
// generator's own seed stream.
use rand::{rngs::StdRng, RngExt, SeedableRng};

#[path = "wfc_arena/lobby.rs"]
mod lobby;
#[path = "wfc_arena/pause.rs"]
mod pause;
#[path = "wfc_arena/result.rs"]
mod result;
// The generator is `nova_wfc`, base-game code the editor's Generate verb draws
// from too. What stays HERE is the arena's own post-collapse stamps.
#[path = "wfc_arena/stamps.rs"]
mod stamps;
use nova_wfc::prelude::*;

#[derive(Parser)]
#[command(name = "wfc_arena")]
#[command(version = "1.0.0")]
#[command(about = "A roster of wave-function-collapse ships fights in a dressed arena", long_about = None)]
struct Cli {
    /// Where the draft starts reading the seed stream; each drafted hull takes
    /// the next viable seed past the last one. Unset, a hand-run rolls a fresh
    /// head and logs it - see [`resolve_seed`].
    #[arg(long)]
    seed: Option<u64>,
    /// One hull of the roster, repeatable: `TEAM[:STYLE[:SEED]][:player]`,
    /// where TEAM is `amber` or `onyx`, STYLE initializes that side's style,
    /// SEED pins the collapse and a trailing `player` puts you
    /// in the hull (at most one). No `--ship` at all fields one hull per
    /// team.
    #[arg(long = "ship", value_name = "TEAM[:STYLE[:SEED]][:player]", value_parser = parse_ship)]
    ships: Vec<ShipSpec>,
    /// Initialize both sides with this style id instead of the first style.
    #[arg(long)]
    style: Option<String>,
}

/// The seed head a SCRIPTED run starts from. Not `wfc_ships`' default on
/// purpose: the two examples should not photograph the same hulls.
const DEFAULT_SEED: u64 = 20_260_816;

/// Where the draft starts reading, in priority order: `--seed`, then
/// [`SEED_ENV`], then the fixed [`DEFAULT_SEED`] for a scripted run, and
/// otherwise a fresh roll from the OS.
///
/// A hand-run wants a NEW matchup each time - that is the whole point of a
/// generator bench, and a fixed default meant every launch fielded the same
/// two hulls. A capture or a probe sweep wants the opposite: the hero media
/// photographs particular ships and a measurement compares like with like, so
/// a harness run stays on the pinned head unless it names its own.
///
/// The head is logged by [`draft_roster`] - here it would be written before
/// `AppBuilder` installs the log plugin, so nothing would carry it - and the
/// lobby shows it in an editable field. [`draft_roster`] also prints the whole
/// replay line, head plus one `--ship` per slot, because a pinned slot is not
/// reproduced by the head alone.
fn resolve_seed(asked: Option<u64>) -> u64 {
    if let Some(seed) = asked.or_else(seed_from_env) {
        return seed;
    }
    if HARNESS_ENVS
        .iter()
        .any(|key| std::env::var_os(key).is_some())
    {
        return DEFAULT_SEED;
    }
    rand::random::<u64>()
}

/// The two teams. Only Player<->Enemy reads as hostile in the relation model,
/// so an AI-vs-AI fight needs one team flying the player's colors - the same
/// trick every AI-vs-AI backdrop uses.
struct Team {
    /// Callsign for the result board, log and `--ship` argument, since
    /// "player" would be a lie about who is driving.
    callsign: &'static str,
    allegiance: Allegiance,
    /// The team chevron's colour. TEAM identity, not relation semantics: the
    /// HUD's ally-green would say "friend", and neither of these teams is the
    /// viewer's friend. AMBER wears the HUD's amber family (the objective
    /// accent), ONYX the hostile family red.
    tint: Color,
    /// The PLAYER slot's facing at spawn: toward the other line, so the
    /// viewer's first frame holds the fight. AI slots ignore it and spawn
    /// aligned with their first patrol leg instead - see [`combatant`] for
    /// why (the autopilot's Align phase was eating the opening).
    yaw: f32,
    /// Passive patrol ring near the center, shared by every hull on the team.
    /// Its centroid anchors the leash, which is what keeps the fight over the
    /// dressed ground instead of drifting off into the void.
    patrol: [Vec3; 3],
}

const TEAMS: [Team; 2] = [
    Team {
        callsign: "AMBER",
        allegiance: Allegiance::Player,
        tint: nova_ui::theme::semantic::OBJECTIVE,
        yaw: -std::f32::consts::FRAC_PI_2,
        patrol: [
            Vec3::new(-70.0, 10.0, 50.0),
            Vec3::new(70.0, 15.0, -50.0),
            Vec3::new(0.0, 5.0, 70.0),
        ],
    },
    Team {
        callsign: "ONYX",
        allegiance: Allegiance::Enemy,
        tint: nova_ui::theme::semantic::THREAT,
        yaw: std::f32::consts::FRAC_PI_2,
        patrol: [
            Vec3::new(70.0, -5.0, -50.0),
            Vec3::new(-70.0, -10.0, 50.0),
            Vec3::new(0.0, -15.0, -70.0),
        ],
    },
];

/// The two lines face each other across the arena with a vertical and lateral
/// split, so the approach lines cross instead of meeting nose to nose.
/// ~3.05 km apart - well past the 1.8 km PDC fire gate (reach x 0.9) - which
/// only works because the approach is COLD (see [`ENGAGE_GRACE_SECS`] and
/// [`ENGAGE_RANGE`]): hot from 2.8 km, one 8-tube alpha strike ended the fight
/// in 11 seconds with no reply, which is why the last cut spawned at 1.63 km.
/// Cold, the long spawn buys a real approach and the fight still opens near
/// gun range. Not further: the passive closing rate is ~25-40 m/s (measured),
/// so at 3.45 km the quiet leg ran 45 s and read as dead air, not tension.
///
/// Engine world units: [`spawn_position`] also poses the spawn `Transform`, so
/// the layout stays in Bevy space and crosses to meters at the config field.
const LINE_STANDOFF: f32 = 150.0;
const LINE_LIFT: f32 = 12.0;
const LINE_OFFSET: f32 = 30.0;
/// Seconds both teams hold their passive patrols after spawn, weapons cold
/// (`AIControllerConfig::engage_delay`). The patrols cross the center, so the
/// grace reads as two formations flying in, not two formations parked.
const ENGAGE_GRACE_SECS: f32 = 10.0;
/// The weapons-free gate (`AIControllerConfig::engage_range`): even past
/// the grace a line stays passive until a hostile closes inside this. This
/// gate, not the grace, is what actually times the first shot - the passive
/// closing is slow, so the lines cross it long after the grace expires.
///
/// AT the 1.8 km gun gate (reach x 0.9), so the fight opens with guns and
/// torpedoes TOGETHER. Wider gates all lost fights to the torpedo alpha:
/// at 2.4 km the gate opened one-sidedly (AMBER salvoed and wiped ONYX before
/// ONYX ever fired), and at 2.2 km the bout was a coin flip - the bigger
/// battery intercepts the smaller salvo OUTRIGHT (6 torpedoes into 16
/// turrets land nothing), so unless the loser's guns connect in the few
/// seconds both sides are alive, it dies having dealt zero and the walk's
/// both-sides-dealt predicate fails. Guns from the first second of the
/// engagement are what keep the fight mutual.
const ENGAGE_RANGE: Meters = Meters(1_800.0);
/// Centre-to-centre spacing along a line, in engine world units beside the
/// other [`LINE_STANDOFF`] figures. Three times the widest hull the grid can
/// grow, so a line is a formation rather than a pile-up, and short enough that
/// the far end of one line still opens at gun range on the far end of the
/// other.
const LINE_SPACING: f32 = 34.0;

/// Combat breaks off past this distance from the patrol centroid. Wide enough
/// for real chases, tight enough that the fight stays over the rock ring.
const LEASH: Meters = Meters(2_800.0);

/// The junk blobs: where each debris cluster anchors, and how many wreckage
/// FRAGMENTS it scatters. All three sit on the Z FLANKS (|z| >= 1.6 km),
/// because the fight runs along X: the spawn lines stand at x = +/-1.5 km
/// ([`LINE_STANDOFF`]) with |z| <= ~800 m, and the approach corridor between
/// them is the axis every torpedo flies down - junk there would eat ordnance
/// and decide fights.
/// Two blobs sit at negative z, which is the BACKGROUND of the capture frame
/// (the frame camera stands on +Z); the third is on the south flank for the
/// idle orbit and the follow cameras to sweep past.
const DERELICT_BLOBS: [(Meters3, usize); 3] = [
    (Meters3::new(400.0, -340.0, -1_900.0), 8),
    (Meters3::new(-1_500.0, 300.0, -1_600.0), 7),
    (Meters3::new(900.0, -250.0, 2_000.0), 5),
];
/// How many sections one fragment carries: a broken-off chunk of structure,
/// never anything that could read as an intact vessel. The first cut spawned
/// five FULL derelict hulls (912 sections) and the owner called it - junk is
/// "multiple small things", so the blob budget went from hulls to fragments
/// and the section total dropped by an order of magnitude.
const FRAGMENT_MIN_SECTIONS: usize = 2;
const FRAGMENT_MAX_SECTIONS: usize = 8;
/// The scatter shell fragments land in around their blob anchor, and the
/// closest two fragments may stand: ~30 m chunks 120+ m apart read as a drifted
/// debris field, not a pile.
const FRAGMENT_SHELL: (Meters, Meters) = (Meters(60.0), Meters(400.0));
const FRAGMENT_SEPARATION: Meters = Meters(120.0);
/// Salt on the roster's stream head for the fragment rolls: the junk follows
/// the lobby's resolved seed head and never duplicates a combatant's seed (the
/// draft scans at most [`DRAFT_SCAN_CAP`] past the head).
const DERELICT_SEED_SALT: u64 = 0xDEAD;
/// The scenario id prefix a fragment spawns under - distinct from
/// [`FIGHTER_ID_PREFIX`], so the follow cameras can never latch onto junk.
const DERELICT_ID_PREFIX: &str = "arena_derelict_";

/// The distant landmark, well outside the leash + its own 4 km sphere of
/// influence (`mu = soi_cutoff_accel * soi^2` at the shipped 0.25 cutoff), so
/// it is scenery and never a well the fight falls into.
const PLANETOID_POSITION: Meters3 = Meters3::new(-6_200.0, -1_400.0, -4_200.0);
const PLANETOID_RADIUS: Meters = Meters(240.0);
const PLANETOID_MASS: f32 = 40_000.0;
/// Pinned silhouette, so the landmark is the same landmark every load.
const PLANETOID_SEED: u32 = 20_260_816;

/// The prototypes the arena reads by name: the two bays it swaps between, and
/// the two mounts the collapse draws. Every one of them is shipped content -
/// the arena names them, it does not author them.
const SERPENT_BAY: &str = "torpedo_section";
const LANCE_BAY: &str = "lance_torpedo_section";
const KINETIC_MOUNT: &str = "pdc_kinetic_turret_section";
const PIERCE_MOUNT: &str = "pdc_pierce_turret_section";
/// The bow gun every arena hull is seeded with. Aliased rather than
/// spelled out: the stamp and the keybind have to name the same section.
const SPINAL_LANCE: &str = RAILGUN_LANCE_SECTION_ID;

/// The scenario id prefix every combatant spawns under, so the follow cameras
/// can find a roster SLOT in the live world.
const FIGHTER_ID_PREFIX: &str = "wfc_fighter_";

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    let ships = if cli.ships.is_empty() {
        default_roster()
    } else {
        cli.ships.clone()
    };
    let players = ships.iter().filter(|ship| ship.player).count();
    assert!(
        players <= 1,
        "wfc_arena: {players} `:player` slots - one viewer, one hull, at most one"
    );
    // REFUSED, not ignored: the driven walk proves an AI-vs-AI fight, and a
    // hull waiting on human input would stall the fight predicate into its
    // deadline. Quietly drafting an AI where a player was asked for would
    // make the run a lie about what it exercised. The env name is pinned by
    // nova_autopilot's env-contract test; the const is not visible from here.
    assert!(
        players == 0 || std::env::var_os("NOVA_AUTOPILOT").is_none(),
        "wfc_arena: a `:player` slot cannot run under NOVA_AUTOPILOT - drop the \
         flag or the env"
    );
    // The ordinary duel owns the hero media. An explicit 2v2 roster uses the
    // same arena and driven walk for the landing combat row without adding a
    // second capture harness.
    #[cfg(feature = "debug")]
    let capture_loop = if ships.len() == 4 {
        LANDING_2V2_LOOP
    } else {
        HERO_LOOP
    };
    #[cfg(feature = "debug")]
    let capture_thumbnail = ships.len() == 2;
    let roster = Roster {
        seed: resolve_seed(cli.seed),
        ships,
        drafted: Vec::new(),
        style: 0,
        binding_overrides: BTreeMap::new(),
    };
    let requested = cli.style.clone();
    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| {
            arena_plugin(app, roster.clone(), StyleRequest(requested.clone()))
        })
        .build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (inert without its NOVA_PROBE_* env): run timeline,
        // engine-bound invariants, and the frame-time capture over the 4v4
        // brawl - the release's headline profiling case.
        //
        // The capture is GATED on the fight instead of on `Playing`: the
        // opening is a passive approach that spends 15-25 s before a shot is
        // legal, so an ungated warm-up would spend the whole window measuring
        // two lines of ships flying at each other. It opens on the same
        // scoreboard predicate the driven walk advances on - both teams have
        // fired AND both have connected.
        //
        // And it is BOUNDED, because the far end of this window is a match that
        // can be WON. See [`MEASURED_WINDOW`].
        //
        // The gate alone is not enough, because the frame a fight is DECIDED is
        // also a frame the fight has happened in: a wipe is what credits the
        // last of the damage the gate waits for, so the gate can open onto an
        // empty arena and the window then measures the aftermath. The result
        // screen pauses the clock a second or two later, which is too late -
        // the window has already opened, and a scene that is over reads as an
        // ordinary cheap row. Hence the liveness half: both teams standing.
        app.add_plugins(
            nova_probe::NovaProbePlugin::default()
                .ready_frametime(|world: &World| {
                    world
                        .get_resource::<Scoreboard>()
                        .is_some_and(Scoreboard::fight_happened)
                })
                .live_frametime(|world: &World| {
                    world
                        .get_resource::<Scoreboard>()
                        .is_some_and(Scoreboard::both_teams_standing)
                })
                .frametime_window(MEASURED_WINDOW.0, MEASURED_WINDOW.1),
        );
        // Clean frames at the fleet's 16:9, dev overlays out of shot. The HUD
        // drops to cinematic only under capture: a hand-run keeps the level On
        // so grave/tilde still controls the combat instruments and chevrons.
        //
        // The shot resolution stands down for a MEASURED run: the frame-time
        // capture sizes the window before winit creates it, and a second
        // Startup writer asking for 1920x1080 is ambiguous with it - and now
        // refused outright, since a capture that is not the size it reports is
        // comparable with nothing.
        if !nova_probe::probe_armed() {
            app.add_systems(Startup, force_capture_resolution);
        }
        app.add_systems(Startup, hide_dev_overlays);
        if capturing() {
            app.add_systems(Startup, hide_hud);
        }
        // The media recorder extends the same driven walk below. It is inert on
        // probe and hand runs; adding a second autopilot would be a duplicate
        // driver rather than another camera.
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::new(
            arena_loop_profile(capture_loop),
        ));
        // NO freeze_bodies here, unlike wfc_ships: the whole point is that
        // these bodies fly.
        app.add_plugins(arena_script(capture_loop, capture_thumbnail));
    }

    app.run()
}

fn arena_plugin(app: &mut App, roster: Roster, requested: StyleRequest) {
    app.insert_resource(roster);
    app.insert_resource(requested);
    app.init_resource::<Scoreboard>();
    app.init_resource::<FollowAim>();
    // The frame vantage until a pose key or the free-fly rig says otherwise -
    // and under capture too, because framing the fight IS the capture framing.
    app.insert_resource(Vantage::Frame);
    // Enabled only for a hand-run: a capture composes its own frame, and an
    // orbit under it would photograph a different bearing every run.
    app.insert_resource(IdleOrbit::new(!capturing()));
    // The staged strike is capture machinery: the counters exist only where a
    // loop is being recorded, and the driven walk is the only thing that ever
    // arms them.
    if capturing() {
        app.init_resource::<Strike>();
        app.add_systems(Update, count_strike_hits);
        app.add_observer(count_strike_shots);
    }
    lobby::register(app);
    pause::register(app);
    result::register(app);
    app.add_systems(
        Update,
        (
            track_damage,
            count_shots,
            report_score.run_if(in_state(GameStates::Playing).and_then(lobby::match_active)),
            // The example's whole camera rig stands down in player mode: the
            // game's chase-camera authority owns the view, and a vantage or
            // orbit writing the scenario camera under it would fight it for
            // every frame.
            (select_vantage, free_camera_on_input, track_orbit_idle).run_if(ai_cameras),
            freeze_junk,
            (
                // In player mode the game tags the chase camera itself
                // (`SpaceshipCameraController` -> `ScreenIndicatorCamera`),
                // and a second tagged camera would leave the projection on
                // whichever it found first.
                tag_indicator_camera.run_if(ai_cameras),
                retire_stock_markers,
                spawn_team_chevrons,
                reap_team_chevrons,
            ),
        ),
    );
    // After the projection, or the gate loses to its per-frame Visible write -
    // see `gate_team_chevrons` for the ordering account.
    app.add_systems(
        PostUpdate,
        gate_team_chevrons
            .after(ScreenIndicatorSystems)
            .before(bevy::ui::UiSystems::Layout),
    );
    // PostUpdate, after the free-fly rig's own write and before the transform
    // propagates, for wfc_ships' reason: the rig syncs in PostUpdate and an
    // unordered Update system loses to it every frame. The orbit is chained
    // AFTER the pose, so an idle-armed orbit wins the frame and re-arms off
    // the bearing the pose last wrote.
    app.add_systems(
        PostUpdate,
        (pose_vantage_camera, orbit_idle_camera)
            .chain()
            .run_if(ai_cameras)
            .after(WASDCameraSystems::Sync)
            .before(TransformSystems::Propagate),
    );
}

/// The example's cameras run only while every hull is AI-flown: a `:player`
/// slot hands the view to the game's chase-camera authority, and the vantage
/// poses, the idle orbit and the indicator-camera tag all stand down with it.
fn ai_cameras(roster: Res<Roster>) -> bool {
    roster.player_slot().is_none()
}

/// One hull the roster asks for: which team it fights for, the look it insists
/// on (if any), the seed it insists on (if any), and whether the viewer flies
/// it. The `--ship` value.
#[derive(Clone)]
struct ShipSpec {
    /// Index into [`TEAMS`].
    team: usize,
    /// A style id out of the merged content, or `None` for the run's look.
    /// Held as an id rather than an index because it comes off a command line,
    /// where a catalog position would mean nothing.
    style: Option<String>,
    /// A pinned collapse seed, or `None` to take the next one the draft finds.
    seed: Option<u64>,
    /// This slot is the PLAYER's: the hull spawns under the game's player
    /// controller instead of the AI, and the example's cameras stand down.
    player: bool,
}

/// One `--ship` field, trimmed, where an empty field reads as absent - so
/// `amber::7` pins a seed without naming a style.
fn field(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|field| !field.is_empty())
}

/// Parse one `--ship` value: `TEAM[:STYLE[:SEED]][:player]`, colon separated,
/// an empty field meaning "the default for that field".
///
/// Colons rather than `key=value` pairs because the common case is a bare team
/// name and the whole grammar is three fields deep; the flag is repeated per
/// hull, so it is typed more often than it is read. The `player` token is
/// positional-last rather than a field of its own, so `amber:player` and
/// `amber:armoured:7:player` both read the way they are said; it shadows a
/// style literally named "player", which the content does not ship.
fn parse_ship(value: &str) -> Result<ShipSpec, String> {
    let mut fields: Vec<&str> = value.split(':').collect();
    let player = fields
        .last()
        .is_some_and(|last| last.trim().eq_ignore_ascii_case("player"));
    if player {
        fields.pop();
    }
    let name = fields.first().copied().unwrap_or_default().trim();
    let team = TEAMS
        .iter()
        .position(|team| team.callsign.eq_ignore_ascii_case(name))
        .ok_or_else(|| {
            format!(
                "'{name}' is not a team: expected {}",
                TEAMS
                    .iter()
                    .map(|team| team.callsign.to_ascii_lowercase())
                    .collect::<Vec<_>>()
                    .join(" or "),
            )
        })?;
    let style = field(fields.get(1).copied()).map(str::to_string);
    let seed = match field(fields.get(2).copied()) {
        Some(seed) => Some(
            seed.parse::<u64>()
                .map_err(|error| format!("'{seed}' is not a seed: {error}"))?,
        ),
        None => None,
    };
    if fields.len() > 3 {
        return Err(format!(
            "'{value}' has more fields than TEAM[:STYLE[:SEED]][:player]"
        ));
    }
    Ok(ShipSpec {
        team,
        style,
        seed,
        player,
    })
}

/// Hulls per side a MEASURED run fields with no `--ship`: the 4v4 brawl the
/// frame-time budget is recorded against. A photograph and a hand-run stay the
/// duel - eight AI hulls is a load, not a composition.
const MEASURED_SHIPS_PER_TEAM: usize = 4;

/// The frame-time capture's window, `(warm-up, captured)` frames, in place of
/// the probe's 180 + 900 baseline.
///
/// The far end of this window is a match that can be WON, and the result screen
/// PAUSES the simulation while still drawing the whole arena - so a window that
/// runs past the end measures a still picture at a plausible cost. Two of ten
/// baseline captures did exactly that; one spent 555 of its 900 frames stopped.
/// The capture now refuses such a window outright, which turns an over-long
/// window from a quiet lie into a failed run - so the size here is what decides
/// whether the subject is measurable at all.
///
/// Both halves are read off those ten captures, counting from the frame the
/// readiness gate opens:
///
/// - the SHORTEST fight ran 525 frames past the gate before the result screen
///   took it, so the whole window has to fit inside that with room to spare;
/// - the warm-up can be short here in a way it cannot be for a capture that
///   opens on `Playing`, because the gate does not open until both teams have
///   fired AND both have connected - a minute of live combat, with the guns,
///   the projectile pipelines and the impact effects all already exercised.
///   60 frames covers the transient at the gate itself, and the 120 frames it
///   gives back are 120 more frames of fight inside the bound.
///
/// 60 + 360 = 420 frames past the gate, 20% clear of the shortest fight
/// measured. Shortening the window costs percentile resolution - p99 is the
/// fourth-worst frame of 360, against the ninth-worst of 900 - and that is the
/// price of measuring one scene instead of two.
// Only the `debug` build wires a capture, so the window it sizes is dead
// weight without it.
#[cfg(feature = "debug")]
const MEASURED_WINDOW: (u32, u32) = (60, 360);

/// Whether this binary is a probe MEASUREMENT pass rather than a photograph.
///
/// Both halves matter and they are read differently: `probe_armed` is the
/// frame-time pass (env), `feature = "trace"` is the profiled pass (a
/// build probe makes only for the chrome trace). If only one of them fielded
/// the 4v4, the top-systems table would rank a lighter scene than the
/// frame-time numbers describe, which is the exact way a profile lies.
fn measuring() -> bool {
    nova_probe::probe_armed() || cfg!(feature = "trace")
}

/// The roster a run with no `--ship` fields: one drafted hull per team, on the
/// run's look - the duel this example started as - or
/// [`MEASURED_SHIPS_PER_TEAM`] per side under a measurement pass.
fn default_roster() -> Vec<ShipSpec> {
    let per_team = if measuring() {
        MEASURED_SHIPS_PER_TEAM
    } else {
        1
    };
    (0..TEAMS.len())
        .flat_map(|team| {
            (0..per_team).map(move |_| ShipSpec {
                team,
                style: None,
                seed: None,
                player: false,
            })
        })
        .collect()
}

/// Which matchup is on: where the DRAFT starts reading the seed stream, the
/// hulls asked for, which seeds they actually collapsed from (see
/// [`draft_roster`]), and the look a ship wears when it named none - an index
/// into the merged style catalog, for `wfc_ships`' reason: a producer must not
/// know what a style is called.
#[derive(Resource, Clone)]
struct Roster {
    seed: u64,
    ships: Vec<ShipSpec>,
    drafted: Vec<u64>,
    style: usize,
    binding_overrides: BTreeMap<(usize, String), Vec<InputSource>>,
}

impl Roster {
    /// How many hulls a team fields.
    fn strength(&self, team: usize) -> usize {
        self.ships.iter().filter(|ship| ship.team == team).count()
    }

    /// The roster slot the viewer flies, if any. `Some` is PLAYER MODE: the
    /// example's cameras stand down.
    fn player_slot(&self) -> Option<usize> {
        self.ships.iter().position(|ship| ship.player)
    }
}

/// The style id `--style` asked for, resolved to an index on the first load.
#[derive(Resource)]
struct StyleRequest(Option<String>);

/// What a hull brings to a fight, by flavour: the collapse decides the guns
/// and the tubes, [`load_lances`] decides what the tubes carry.
#[derive(Clone, Copy, Default)]
struct Armament {
    kinetic: usize,
    pierce: usize,
    serpents: usize,
    lances: usize,
}

impl Armament {
    fn turrets(&self) -> usize {
        self.kinetic + self.pierce
    }

    fn bays(&self) -> usize {
        self.serpents + self.lances
    }

    /// Armed enough to be DRAFTED as a combatant: guns enough that some bear
    /// on the target whatever the arcs rolled, and tubes enough to answer a
    /// salvo with a salvo.
    fn viable(&self) -> bool {
        self.turrets() >= MIN_TURRETS && self.bays() >= MIN_BAYS
    }
}

impl std::fmt::Display for Armament {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} kinetic + {} pierce turrets, {} serpent + {} lance bays",
            self.kinetic, self.pierce, self.serpents, self.lances,
        )
    }
}

/// Count what a hull carries, by prototype.
fn armament(hull: &ShipHull) -> Armament {
    let count = |prototype: &str| {
        hull.sections
            .iter()
            .filter(|section| {
                matches!(&section.source, SectionSource::Prototype(id) if id == prototype)
            })
            .count()
    };
    Armament {
        kinetic: count(KINETIC_MOUNT),
        pierce: count(PIERCE_MOUNT),
        serpents: count(SERPENT_BAY),
        lances: count(LANCE_BAY),
    }
}

/// The armament floor a drafted hull must clear. The collapse arms hulls with
/// wild variance - the first driven run rolled 10 turrets + 8 bays against 2
/// turrets + 0 bays, and the second ship died in eight seconds without firing
/// once. An under-armed roll is a fine SUBJECT (wfc_ships will pose it) but not
/// a combatant, so the arena drafts past it.
const MIN_TURRETS: usize = 4;
const MIN_BAYS: usize = 2;
/// How many seeds the draft may read per hull before failing loudly.
const DRAFT_SCAN_CAP: u64 = 64;

/// The cell of a mirrored pair, off a section id (`starboard_3_2_7` and
/// `port_3_2_7` are the same tube either side of the centreline).
fn mirror_cell(id: &str) -> &str {
    id.split_once('_').map_or(id, |(_, cell)| cell)
}

/// Load half of a hull's tubes with Lances.
///
/// The two shipped bays are the same housing on the same sockets with the same
/// warhead, differing only in the ordnance they carry, so re-sourcing one is a
/// LOADOUT change and not a change of hull: the lane the collapse cleared, the
/// skin it refused and the lint it passed are all untouched. That is why the
/// generator draws one bay and the arena decides what is in it - `wfc_ships`
/// photographs structure, and what a tube is loaded with is not structure.
///
/// ALTERNATING rather than rolling, because the point is that a fight shows
/// both: a roll can hand a two-tube hull two Serpents, and the draft floor only
/// promises two tubes. Every hull that clears the floor comes out carrying at
/// least one of each, and the seed decides only which end of the list starts on
/// a Lance. The two halves of a mirrored pair always carry the same ordnance -
/// a Lance to port against a Serpent to starboard is an accident, not a
/// loadout.
fn load_lances(hull: &mut ShipHull, seed: u64) {
    let mut pairs: Vec<&str> = Vec::new();
    for section in &hull.sections {
        if matches!(&section.source, SectionSource::Prototype(id) if id == SERPENT_BAY) {
            let cell = mirror_cell(&section.id);
            if !pairs.contains(&cell) {
                pairs.push(cell);
            }
        }
    }
    let lances: Vec<String> = pairs
        .iter()
        .enumerate()
        .filter(|(index, _)| (*index as u64).wrapping_add(seed) % 2 == 1)
        .map(|(_, cell)| (*cell).to_string())
        .collect();
    for section in &mut hull.sections {
        let SectionSource::Prototype(id) = &section.source else {
            continue;
        };
        if id.as_str() != SERPENT_BAY {
            continue;
        }
        if lances.iter().any(|cell| cell == mirror_cell(&section.id)) {
            section.source = SectionSource::Prototype(LANCE_BAY.to_string());
        }
    }
}

/// The arena's tile set: the shipped grammar read against the merged catalog.
///
/// Built where it is needed rather than held in a resource, because the lobby
/// rebuilds one per reroll and the arena one per match - both are one-shot, and
/// a stale set is worse than a rebuilt one.
fn arena_tiles(sections: &GameSections, grammars: &GameGrammars) -> TileSet {
    let mut grammar = grammars
        .get_grammar(STANDARD_HULL_GRAMMAR_ID)
        .unwrap_or_else(|| panic!("wfc_arena: no ship grammar '{STANDARD_HULL_GRAMMAR_ID}'"))
        .clone();
    // The whole point of the arena: a spinal weapon benched on hulls nobody
    // designed around one. The base warship seats no lance, so this one names
    // it as its bow gun and the collapse seeds the pair itself.
    grammar.keel.bow_gun = Some(SPINAL_LANCE.to_string());
    TileSet::build(sections, &grammar).unwrap_or_else(|error| panic!("wfc_arena: {error}"))
}

/// Collapse one hull for a roster slot and load its tubes.
fn combat_hull(tiles: &TileSet, seed: u64, style: StyleId, sections: &GameSections) -> ShipHull {
    let mut hull = tiles
        .hull(seed, true, style)
        .unwrap_or_else(|error| panic!("wfc_arena: {error}"));
    stamps::stamp_large_drives(&mut hull, seed, sections, tiles.grid());
    load_lances(&mut hull, seed);
    hull
}

/// Field a hull per roster slot: a pinned seed as asked for, everything else
/// off the seed stream from `from`, taking the first hull that clears the
/// armament floor and carrying the cursor on so no two slots draft the same
/// seed. Deterministic - the same stream head and the same roster always field
/// the same ships - and skipped seeds are logged with the armament that
/// disqualified them.
fn draft_roster(
    tiles: &TileSet,
    ships: &[ShipSpec],
    looks: &[StyleId],
    from: u64,
    sections: &GameSections,
) -> Vec<(u64, ShipHull)> {
    let pinned = ships.iter().filter(|ship| ship.seed.is_some()).count();
    info!(
        "wfc_arena: drafting from seed {from} ({pinned} of {} slots pinned)",
        ships.len()
    );
    let mut cursor = from;
    let mut drafted = Vec::new();
    for (slot, ship) in ships.iter().enumerate() {
        let style = looks[slot];
        if let Some(seed) = ship.seed {
            let hull = combat_hull(tiles, seed, style, sections);
            let arms = armament(&hull);
            if !arms.viable() {
                // A pin is an instruction, so it is honored - but a hull that
                // cannot fight is why a fight might not happen, and the run
                // should say so before the step deadline does.
                warn!("wfc_arena: pinned seed {seed} is under the armament floor ({arms})");
            }
            drafted.push((seed, hull));
            continue;
        }
        let mut found = None;
        for offset in 0..DRAFT_SCAN_CAP {
            let seed = cursor.wrapping_add(offset);
            let hull = combat_hull(tiles, seed, style, sections);
            let arms = armament(&hull);
            if arms.viable() {
                cursor = seed.wrapping_add(1);
                found = Some((seed, hull));
                break;
            }
            info!("wfc_arena: seed {seed} not drafted ({arms})");
        }
        match found {
            Some(pair) => drafted.push(pair),
            None => panic!(
                "wfc_arena: no combat-viable hull in seeds {cursor}..{} \
                 (viable: >= {MIN_TURRETS} turrets and >= {MIN_BAYS} bays)",
                cursor.wrapping_add(DRAFT_SCAN_CAP),
            ),
        }
    }
    // The head alone replays a matchup only when it drafted every slot. A
    // pinned slot came from `--ship`, and the head still dresses the arena
    // either way, so the line a player copies carries both - and stays right
    // when the lobby pins every slot before it starts the match.
    info!(
        "wfc_arena: `--seed {from} {}` fields this matchup again",
        replay_ships(ships, looks, &drafted)
    );
    drafted
}

/// The `--ship` arguments that pin `drafted`, in slot order.
fn replay_ships(ships: &[ShipSpec], looks: &[StyleId], drafted: &[(u64, ShipHull)]) -> String {
    ships
        .iter()
        .zip(looks)
        .zip(drafted)
        .map(|((ship, style), (seed, _))| {
            // An empty STYLE field is legal and means "the run's look", which
            // is what a slot with no style of its own is flying.
            format!(
                "--ship {}:{}:{seed}{}",
                TEAMS[ship.team].callsign.to_ascii_lowercase(),
                style.unwrap_or_default(),
                if ship.player { ":player" } else { "" }
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Where a slot stands in its own team's line, and how long that line is.
fn line_places(ships: &[ShipSpec]) -> Vec<(usize, usize)> {
    let mut standing = [0usize; TEAMS.len()];
    ships
        .iter()
        .map(|ship| {
            let index = standing[ship.team];
            standing[ship.team] += 1;
            let strength = ships.iter().filter(|other| other.team == ship.team).count();
            (index, strength)
        })
        .collect()
}

/// Where one hull spawns: its team's line, standing off the centre, split
/// vertically and laterally from the other line, with the line itself spread
/// along the arena's long axis.
fn spawn_position(team: usize, index: usize, strength: usize) -> Vec3 {
    let side = if team == 0 { -1.0 } else { 1.0 };
    let along = (index as f32 - (strength as f32 - 1.0) * 0.5) * LINE_SPACING;
    Vec3::new(
        side * LINE_STANDOFF,
        side * LINE_LIFT,
        along - side * LINE_OFFSET,
    )
}

/// The player's weapon bindings for a drafted hull, by prototype: every gun
/// the collapse mounted on the left mouse (the campaign's own turret binding,
/// gamepad right trigger beside it), every tube on `F`, and the bow lance on
/// `R` - the mouse's right button is the raise-weapons gesture and the
/// reserved flight-rig sources (`flight_rig_reserved_sources`) are all keys
/// the rig already spends.
///
/// The lance gets a key of its OWN rather than joining the turrets on the
/// mouse: it is one committed shot on a twelve-second reload, and a pilot
/// holding fire on a swarm would burn it on the first frame of every reload.
/// `R` is free in the flight context - NOVA OS spends it, but that is the
/// viewer context, exactly as it already spends the tubes' `F`.
fn player_bindings(
    hull: &ShipHull,
    slot: usize,
    overrides: &BTreeMap<(usize, String), Vec<InputSource>>,
) -> BTreeMap<String, Vec<InputSource>> {
    hull.sections
        .iter()
        .filter_map(|section| {
            let SectionSource::Prototype(id) = &section.source else {
                return None;
            };
            let bindings: Vec<InputSource> = match id.as_str() {
                KINETIC_MOUNT | PIERCE_MOUNT => vec![
                    MouseButton::Left.into(),
                    GamepadButton::RightTrigger2.into(),
                ],
                SERPENT_BAY | LANCE_BAY => vec![KeyCode::KeyF.into()],
                SPINAL_LANCE => vec![KeyCode::KeyR.into()],
                _ => return None,
            };
            Some((
                section.id.clone(),
                overrides
                    .get(&(slot, section.id.clone()))
                    .cloned()
                    .unwrap_or(bindings),
            ))
        })
        .collect()
}

/// One combatant: a drafted hull, clad, on its team's colors and in its
/// team's line - under the same AI pilot the campaign's raiders fly, or
/// under the VIEWER for the one slot `:player` names.
fn combatant(
    slot: usize,
    seed: u64,
    hull: ShipHull,
    ship: &ShipSpec,
    place: (usize, usize),
    binding_overrides: &BTreeMap<(usize, String), Vec<InputSource>>,
) -> ScenarioObjectConfig {
    let team = &TEAMS[ship.team];
    // The armament roll, per ship: the draft floor bounds it from below but one
    // team can still out-gun the other, and a lopsided fight reads differently
    // knowing that. This line is the roll's disclosure, and the only place the
    // loadout the arena chose is stated.
    let arms = armament(&hull);
    info!(
        "wfc_arena: {} {} seed {}: {} sections - {}{}",
        team.callsign,
        slot,
        seed,
        hull.sections.len(),
        arms,
        if ship.player { " - PLAYER" } else { "" },
    );
    let position = spawn_position(ship.team, place.0, place.1);
    // Spawn ALIGNED with the first thing the hull will do. The autopilot
    // opens every GOTO in an Align phase - the nose has to swing onto the
    // burn bearing before it burns - and these hulls turn slowly, so a spawn
    // yaw that merely faced the other line spent the opening seconds visibly
    // re-aiming at the first patrol leg instead of flying it. An AI slot
    // therefore spawns LOOKING AT its first waypoint; the player slot keeps
    // the team's toward-the-enemy yaw, because a viewer's opening frame
    // should hold the fight they are about to join, not a patrol mark.
    let rotation = if ship.player {
        Quat::from_rotation_y(team.yaw)
    } else {
        Transform::from_translation(position)
            .looking_at(team.patrol[0], Vec3::Y)
            .rotation
    };
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: format!("{FIGHTER_ID_PREFIX}{slot}"),
            name: format!("{} {seed}", team.callsign),
            position: Meters3::from_engine(position),
            rotation,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(team.allegiance),
            controller: if ship.player {
                // The game's REAL player controller, bindings derived off the
                // drafted hull. No speed cap (the arena is open space) and
                // real magazines - the AI ships fight with theirs.
                SpaceshipController::Player(PlayerControllerConfig {
                    input_mapping: player_bindings(&hull, slot, binding_overrides),
                    speed_cap: None,
                })
            } else {
                SpaceshipController::AI(AIControllerConfig {
                    patrol: team.patrol.map(Meters3::from_engine).to_vec(),
                    // Anchored on the center-hugging patrol centroid, so the
                    // fight gravitates to the dressed middle of the arena.
                    leash: Some(LEASH),
                    // The cold opening: hold the patrol through the grace,
                    // then keep holding until the lines close inside the
                    // gate. See ENGAGE_GRACE_SECS / ENGAGE_RANGE for the
                    // sizing. The player gets no such leash: they fire and
                    // fly whenever they like.
                    engage_delay: Some(ENGAGE_GRACE_SECS),
                    engage_range: Some(ENGAGE_RANGE),
                    ..Default::default()
                })
            },
            hull: ShipSource::Inline(hull),
            ..Default::default()
        }),
    }
}

/// One ring of dressing rocks, as a seeded scatter action - the editor
/// sandbox's belt idiom bent into the duel backdrop's ring, kept off the
/// fight plane so cover never decides the fight.
fn rock_ring(
    game_assets: &GameAssets,
    id_prefix: &str,
    center: Meters3,
    seed: u64,
    count: u32,
    inner: Meters,
    outer: Meters,
    y: (Meters, Meters),
    radius: (Meters, Meters),
    separation: Meters,
) -> EventActionConfig {
    EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: id_prefix.to_string(),
        count,
        seed,
        region: ScatterRegion::Ring {
            center,
            inner,
            outer,
            y_min: y.0,
            y_max: y.1,
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id_prefix.to_string(),
                name: "Arena Rock".to_string(),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                // DIRECT paths, not dep://: this scenario is built at runtime
                // outside the mod merge, so scheme refs would never rewrite.
                material: KIND_ROCK.to_string(),
                destroy_sound: Some(AssetRef::from("base/sounds/destroy_rock.wav")),
                radius: radius.0,
                texture: AssetRef::from(game_assets.asteroid_texture.clone()),
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some(radius),
        asteroid_kinds: vec![(KIND_ROCK.to_string(), 1)],
        min_separation: Some(separation),
    })
}

/// Pin every junk fragment static the moment it lands.
///
/// Junk is SCENERY: wreckage that holds its pose keeps the blobs composed the
/// way the seed placed them, and a fight that drifts its own set is not the
/// same capture twice. Same command-swap idiom as the harness
/// `freeze_bodies`, scoped to the junk prefix; every frame because scenario
/// reloads respawn the junk with the fight.
///
/// This used to claim a second reason - that a `SpaceshipController::None`
/// hull spawns MASSLESS and NaN-poisons the spatial queries combat aims
/// through. That was investigated and is not true (task 20260817-091716): a
/// controller-less hull composes its mass from its sections like any other,
/// no fragment was ever massless across five instrumented runs, and unfrozen
/// runs fight exactly like frozen ones. The pin stays for the framing, not
/// for the physics.
fn freeze_junk(
    mut commands: Commands,
    q_junk: Query<(Entity, &RigidBody, &EntityId), With<SpaceshipRootMarker>>,
) {
    for (entity, body, id) in &q_junk {
        if matches!(body, RigidBody::Dynamic) && id.0.starts_with(DERELICT_ID_PREFIX) {
            commands.entity(entity).insert(RigidBody::Static);
        }
    }
}

/// One wreckage fragment: a seeded random walk of catalog hull cubes,
/// [`FRAGMENT_MIN_SECTIONS`]..=[`FRAGMENT_MAX_SECTIONS`] cells, sometimes
/// keeping a drive nozzle on a broken-off end.
///
/// Built from the same shipped prototypes the collapse draws, but NOT through
/// the collapse: a fragment is a chunk of ship, not a ship, and the full grid
/// only makes vessels. The hull cube mates on all six faces, so any
/// face-connected walk is one connected link-point graph by construction and
/// passes the same `lint_scenario` gate every other inline hull does. The
/// nozzle is legal on a LEAF cell only: the drive's one socket (its forward
/// face, `NEG_Z * 0.5` in the catalog) is rotated onto the leaf's single
/// neighbour, so it mates exactly and its exhaust points into vacuum.
fn fragment_hull(seed: u64, clad: bool, style: StyleId) -> ShipHull {
    let mut rng = StdRng::seed_from_u64(seed);
    let target = rng.random_range(FRAGMENT_MIN_SECTIONS..=FRAGMENT_MAX_SECTIONS);
    let directions = [
        IVec3::X,
        IVec3::NEG_X,
        IVec3::Y,
        IVec3::NEG_Y,
        IVec3::Z,
        IVec3::NEG_Z,
    ];
    let mut cells: Vec<IVec3> = vec![IVec3::ZERO];
    // Bounded, not exact: a walk that keeps re-hitting itself simply yields a
    // smaller chunk, which is still junk.
    for _ in 0..64 {
        if cells.len() >= target {
            break;
        }
        let from = cells[rng.random_range(0..cells.len())];
        let next = from + directions[rng.random_range(0..directions.len())];
        if !cells.contains(&next) {
            cells.push(next);
        }
    }

    // A leaf (one neighbour) may keep a nozzle - the look of a drive assembly
    // torn off with a cell of hull still bolted to it.
    let neighbours = |cell: IVec3| {
        directions
            .iter()
            .filter(|direction| cells.contains(&(cell + **direction)))
            .count()
    };
    let nozzle = (cells.len() > 1 && rng.random_range(0..2) == 0)
        .then(|| {
            cells
                .iter()
                .position(|cell| neighbours(*cell) == 1)
                .map(|leaf| {
                    let toward = directions
                        .iter()
                        .find(|direction| cells.contains(&(cells[leaf] + **direction)))
                        .expect("a leaf has its one neighbour");
                    (leaf, toward.as_vec3())
                })
        })
        .flatten();

    // Recentred on the bounding-box middle (shape_bench's idiom: the offset
    // stays on the half-cell phase the skin derivation buckets on), so the
    // root's tumble turns the chunk about its own middle.
    let (low, high) = cells
        .iter()
        .fold((IVec3::MAX, IVec3::MIN), |(low, high), cell| {
            (low.min(*cell), high.max(*cell))
        });
    let centre = (low + high).as_vec3() * 0.5;
    let sections = cells
        .iter()
        .enumerate()
        .map(|(index, cell)| {
            let (prototype, rotation) = match nozzle {
                Some((leaf, toward)) if leaf == index => (
                    "basic_thruster_section",
                    // The drive bolts by its forward face: map that socket
                    // onto the leaf's one neighbour and the exhaust faces
                    // away from the chunk on its own.
                    Quat::from_rotation_arc(Vec3::NEG_Z, toward),
                ),
                _ => ("reinforced_hull_section", Quat::IDENTITY),
            };
            SpaceshipSectionConfig {
                id: format!("junk_{index}"),
                position: cell.as_vec3() - centre,
                rotation,
                source: SectionSource::Prototype(prototype.to_string()),
                modifications: vec![],
            }
        })
        .collect();

    ShipHull {
        sections,
        skin: clad,
        style: clad.then_some(style).flatten().map(str::to_string),
        ..default()
    }
}

/// The junk: many SMALL wreckage fragments in loose blobs on the flanks, each
/// blob under its own shell of small debris rocks.
///
/// Fragments carry no controller and no allegiance (so the AI never targets
/// one and the scoreboard never counts one), and clad/bare alternate for
/// variety - a skinned chunk next to a stripped frame is what a debris field
/// reads as. Positions scatter in a seeded shell around the blob anchor with
/// a minimum separation, tumbled per fragment. Everything derives from the
/// salted stream head, so a restarted match stays reproducible and a lobby
/// reroll changes its dressing with its roster.
fn derelicts(
    game_assets: &GameAssets,
    styles: &GameStyles,
    roster: &Roster,
) -> Vec<EventActionConfig> {
    let style = style_at(styles, roster.style);
    let mut actions = Vec::new();
    let mut index = 0usize;
    let mut sections = 0usize;
    for (blob, (anchor, count)) in DERELICT_BLOBS.iter().enumerate() {
        let mut placed: Vec<Meters3> = Vec::new();
        let mut rng = StdRng::seed_from_u64(
            (roster.seed ^ DERELICT_SEED_SALT).wrapping_add((blob as u64) << 8),
        );
        for _ in 0..*count {
            // Rejection-sampled scatter: fragments are ~30 m across, so a
            // handful of retries always finds standing room in the shell.
            let mut offset = Meters3::ZERO;
            for _ in 0..32 {
                let radius = rng.random_range(FRAGMENT_SHELL.0.get()..FRAGMENT_SHELL.1.get());
                let yaw = rng.random_range(0.0..std::f32::consts::TAU);
                let lift = rng.random_range(-0.4..0.4f32);
                offset = Meters3::new(yaw.cos() * radius, lift * radius, yaw.sin() * radius);
                if placed
                    .iter()
                    .all(|other| other.distance(offset) >= FRAGMENT_SEPARATION)
                {
                    break;
                }
            }
            placed.push(offset);
            let seed = (roster.seed ^ DERELICT_SEED_SALT).wrapping_add(index as u64);
            // Seeded tumble: wreckage holds no attitude, and a shared quat
            // would read as a formation.
            let mut angle = || rng.random_range(0.0..std::f32::consts::TAU);
            let rotation = Quat::from_euler(EulerRot::XYZ, angle(), angle(), angle());
            let hull = fragment_hull(seed, index.is_multiple_of(2), style);
            sections += hull.sections.len();
            actions.push(EventActionConfig::SpawnScenarioObject(
                ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: format!("{DERELICT_ID_PREFIX}{index}"),
                        name: format!("Wreckage {seed}"),
                        position: *anchor + offset,
                        rotation,
                    },
                    kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                        allegiance: None,
                        controller: SpaceshipController::None,
                        hull: ShipSource::Inline(hull),
                        ..Default::default()
                    }),
                },
            ));
            index += 1;
        }
        // The debris shell: the ring dressing idiom shrunk onto the blob, so
        // the fragments sit IN a field of junk instead of floating beside one.
        actions.push(rock_ring(
            game_assets,
            &format!("arena_junk_{blob}_"),
            *anchor,
            roster.seed ^ DERELICT_SEED_SALT ^ ((blob as u64) << 8),
            6,
            Meters(160.0),
            Meters(480.0),
            (Meters(-140.0), Meters(140.0)),
            (Meters(8.0), Meters(24.0)),
            Meters(100.0),
        ));
    }
    // The budget disclosure: the junk adds real sections (and skin on the clad
    // fragments), and this line is where a heavy junkyard would say so.
    info!(
        "wfc_arena: {index} junk fragments in {} blobs, {sections} sections adrift",
        DERELICT_BLOBS.len(),
    );
    actions
}

/// The landmark: one large, invulnerable, PINNED rock, far enough out to be
/// scenery rather than a wall or a well.
fn planetoid(game_assets: &GameAssets) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "arena_planetoid".to_string(),
            name: "Arena Planetoid".to_string(),
            position: PLANETOID_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            material: KIND_ROCK.to_string(),
            destroy_sound: Some(AssetRef::from("base/sounds/destroy_rock.wav")),
            radius: PLANETOID_RADIUS,
            texture: AssetRef::from(game_assets.asteroid_texture.clone()),
            mass: Some(PLANETOID_MASS),
            invulnerable: true,
            seed: Some(PLANETOID_SEED),
            lock_signature: None,
        }),
    })
}

/// The look one roster slot wears: the style it named, or the run's.
fn ship_style<'a>(styles: &'a GameStyles, ship: &ShipSpec, run: StyleId<'a>) -> StyleId<'a> {
    let Some(id) = ship.style.as_deref() else {
        return run;
    };
    match styles.iter().find(|style| style.id == id) {
        Some(style) => Some(style.id.as_str()),
        // Loud for `--style`'s reason: a typo would otherwise dress the hull in
        // the run's look and read as the one that was asked for.
        None => panic!("--ship style '{id}' is not in the merged content"),
    }
}

/// The arena: the drafted roster, the standard three-point rig, two dressing
/// rings and the landmark, under the game's own sky. Checked by the REAL
/// content lint before it is fought over, exactly like the posed row.
fn arena(
    game_assets: &GameAssets,
    sections: &GameSections,
    grammars: &GameGrammars,
    styles: &GameStyles,
    roster: &mut Roster,
) -> ScenarioConfig {
    let tiles = arena_tiles(sections, grammars);
    let run_style = style_at(styles, roster.style);
    let looks: Vec<StyleId> = roster
        .ships
        .iter()
        .map(|ship| ship_style(styles, ship, run_style))
        .collect();
    let drafted = draft_roster(&tiles, &roster.ships, &looks, roster.seed, sections);
    roster.drafted = drafted.iter().map(|(seed, _)| *seed).collect();

    let places = line_places(&roster.ships);
    let ships: Vec<EventActionConfig> = drafted
        .into_iter()
        .enumerate()
        .map(|(slot, (seed, hull))| {
            EventActionConfig::SpawnScenarioObject(combatant(
                slot,
                seed,
                hull,
                &roster.ships[slot],
                places[slot],
                &roster.binding_overrides,
            ))
        })
        .collect();

    let scenario = ScenarioConfig {
        description: "Wave-function-collapse ships fight in a dressed arena".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: ships
                .into_iter()
                .chain(ThreePointRig::around("arena", Meters3::ZERO, 8.0).actions())
                .chain(derelicts(game_assets, styles, roster))
                .chain([
                    planetoid(game_assets),
                    // Depth parallax under the fight plane, and a sparser far
                    // ring so the void has a middle distance.
                    rock_ring(
                        game_assets,
                        "arena_rock_low_",
                        Meters3::ZERO,
                        roster.seed ^ 0x0A11,
                        14,
                        Meters(1_600.0),
                        Meters(2_400.0),
                        (Meters(-700.0), Meters(-350.0)),
                        (Meters(15.0), Meters(40.0)),
                        Meters(500.0),
                    ),
                    rock_ring(
                        game_assets,
                        "arena_rock_far_",
                        Meters3::ZERO,
                        roster.seed ^ 0x0FA2,
                        10,
                        Meters(3_200.0),
                        Meters(4_000.0),
                        (Meters(-200.0), Meters(800.0)),
                        (Meters(20.0), Meters(50.0)),
                        Meters(600.0),
                    ),
                ])
                .collect(),
        }],
        ..ScenarioConfig::new(
            "wfc_arena".to_string(),
            "WFC Arena".to_string(),
            game_assets.cubemap.clone().into(),
        )
    };
    refuse_broken_ships(&scenario, sections);
    scenario
}

/// Run the arena through the game's OWN content gate before it is fought over.
///
/// Nothing here re-implements a check: `lint_errors` runs `lint_scenario`, the
/// same function the `content lint` gate and the runtime loader run, so a
/// clean arena is one the game would accept.
fn refuse_broken_ships(scenario: &ScenarioConfig, sections: &GameSections) {
    let errors = lint_errors(scenario, sections);
    assert!(
        errors.is_empty(),
        "wfc_arena: the collapse produced content the game would refuse:\n  {}",
        errors.join("\n  ")
    );
}

/// What one team has actually put in the air, keyed by projectile-carried
/// ammunition identity. A new damage variant or authored torpedo name creates a
/// new row without changing this example.
#[derive(Clone, Default)]
struct Salvo(BTreeMap<String, u32>);

impl Salvo {
    fn record(&mut self, ammunition: String) {
        *self.0.entry(ammunition).or_default() += 1;
    }

    /// Everything fired, which is what "did this team fight" asks.
    fn total(&self) -> u32 {
        self.0.values().sum()
    }
}

impl std::fmt::Display for Salvo {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            return formatter.write_str("nothing");
        }
        formatter.write_str(
            &self
                .0
                .iter()
                .map(|(name, count)| format!("{count} {name}"))
                .collect::<Vec<_>>()
                .join(" + "),
        )
    }
}

/// The fight's evidence: per TEAM, what was FIRED and how much section damage
/// was DEALT to the other team. Aggregated over however many hulls a team
/// fields, because the roster decides the head count and the question the
/// board answers - did both sides fight - does not.
///
/// Damage is read as per-team health-pool deltas rather than off the damage
/// event, because cladding is `HealthIsolated`: a hit soaked by a plate never
/// bubbles to the root, and most of what lands on a clad hull lands on
/// plates. The pool sums every `Health` under every ship root on the team, so
/// plate damage counts like any other. `pool` is `None` while a team has no
/// live root (loading, restarting, or wiped out), which is what keeps a
/// teardown from reading as a massacre.
#[derive(Resource, Default)]
struct Scoreboard {
    fired: [Salvo; TEAMS.len()],
    dealt: [f32; TEAMS.len()],
    pool: [Option<f32>; TEAMS.len()],
}

impl Scoreboard {
    /// Both teams have fired and both have dealt damage: the fight happened.
    /// The driven walk's advance condition, so it exists only where the walk
    /// does.
    #[cfg(feature = "debug")]
    fn fight_happened(&self) -> bool {
        self.fired.iter().all(|salvo| salvo.total() > 0)
            && self.dealt.iter().all(|dealt| *dealt > 0.0)
    }

    /// Both teams still have a ship flying: the fight is LIVE, not decided.
    ///
    /// `pool` is `None` for a team with no live root, so this reads false for
    /// a wipe and for the teardown between matches - the two ways the 4v4 the
    /// capture measures can stop existing while the clock keeps running. It
    /// deliberately says nothing about how MANY ships are left: a four-on-one
    /// is a fight in progress, and refusing it would be a judgement about
    /// workload that nothing here has measured.
    #[cfg(feature = "debug")]
    fn both_teams_standing(&self) -> bool {
        self.pool.iter().all(Option::is_some)
    }
}

/// The team index of an allegiance, or `None` for a neutral bystander.
fn team_of(allegiance: &Allegiance) -> Option<usize> {
    TEAMS.iter().position(|team| team.allegiance == *allegiance)
}

/// Count every round and torpedo the moment it spawns, by flavour, credited to
/// the team of the ship that fired it (`ProjectileOwner` names the ship root,
/// which wears the `Allegiance`).
fn count_shots(
    mut score: ResMut<Scoreboard>,
    q_new: Query<
        (
            &ProjectileOwner,
            Option<&ProjectileDamage>,
            Option<&TorpedoType>,
        ),
        Or<(
            Added<TurretBulletProjectileMarker>,
            Added<TorpedoProjectileMarker>,
        )>,
    >,
    q_team: Query<&Allegiance>,
) {
    for (owner, round, torpedo) in &q_new {
        let Ok(allegiance) = q_team.get(owner.0) else {
            continue;
        };
        let Some(team) = team_of(allegiance) else {
            continue;
        };
        let salvo = &mut score.fired[team];
        if let Some(torpedo) = torpedo {
            salvo.record(format!("{} torpedoes", torpedo.name));
        } else if let Some(round) = round {
            salvo.record(format!("{:?} rounds", round.kind));
        }
    }
}

/// Sum every `Health` pool under each team's ship roots and charge any drop to
/// the OTHER team as damage dealt - see [`Scoreboard`] for why deltas rather
/// than damage events.
fn track_damage(
    mut score: ResMut<Scoreboard>,
    q_health: Query<(Entity, &Health)>,
    q_parents: Query<&ChildOf>,
    q_roots: Query<&Allegiance, With<SpaceshipRootMarker>>,
) {
    let mut pool = [None::<f32>; TEAMS.len()];
    for (entity, health) in &q_health {
        let mut current = entity;
        let team = loop {
            if let Ok(allegiance) = q_roots.get(current) {
                break team_of(allegiance);
            }
            match q_parents.get(current) {
                Ok(ChildOf(parent)) => current = *parent,
                Err(_) => break None,
            }
        };
        if let Some(team) = team {
            *pool[team].get_or_insert(0.0) += health.current;
        }
    }
    for team in 0..TEAMS.len() {
        let rival = (team + 1) % TEAMS.len();
        match (score.pool[team], pool[team]) {
            (Some(previous), Some(now)) if now < previous - f32::EPSILON => {
                score.dealt[rival] += previous - now;
                debug!(
                    "wfc_arena: {} took {:.1} damage (pool {:.1} -> {:.1})",
                    TEAMS[team].callsign,
                    previous - now,
                    previous,
                    now,
                );
            }
            // The whole team vanished: a WIPE, credited to the rival. A
            // torpedo alpha strike can take a ship from intact to despawned
            // between two frames of this system - the first driven run did
            // exactly that and the scoreboard read 0.0 while a ship died on
            // camera - and under load BOTH sides of a mutual annihilation
            // land this way, seconds apart, so the credit cannot require the
            // rival to still be standing. A reload cannot reach this arm:
            // match restart resets the score before the teardown lands (see the
            // ordering note in `arena_plugin`).
            (Some(previous), None) => {
                score.dealt[rival] += previous;
                info!(
                    "wfc_arena: {} wiped out ({:.1} structure erased)",
                    TEAMS[team].callsign, previous,
                );
            }
            _ => {}
        }
        score.pool[team] = pool[team];
    }
}

/// Seconds between scoreboard log lines: often enough to watch a run in the
/// log, rare enough not to drown it.
const REPORT_PERIOD_SECS: f32 = 5.0;

/// Restate the score on the log clock, so a driven run's transcript IS the
/// evidence: per team, hulls still standing, what was fired BY FLAVOUR, damage,
/// remaining structure, and the range between the two teams' centroids - the
/// last one is the flyability readout, closing speed by eye.
fn report_score(
    score: Res<Scoreboard>,
    time: Res<Time>,
    mut last: Local<f32>,
    q_ships: Query<(&Transform, &Allegiance), With<SpaceshipRootMarker>>,
) {
    if time.elapsed_secs() - *last < REPORT_PERIOD_SECS {
        return;
    }
    *last = time.elapsed_secs();
    let mut sum = [Vec3::ZERO; TEAMS.len()];
    let mut standing = [0.0f32; TEAMS.len()];
    for (transform, allegiance) in &q_ships {
        if let Some(team) = team_of(allegiance) {
            sum[team] += transform.translation;
            standing[team] += 1.0;
        }
    }
    let range = if standing.iter().all(|count| *count > 0.0) {
        (sum[0] / standing[0]).distance(sum[1] / standing[1])
    } else {
        0.0
    };
    let pool = |team: usize| score.pool[team].unwrap_or(0.0);
    info!(
        "wfc_arena: {} x{} fired {} - dealt {:.1} - left {:.0} | {} x{} fired {} - dealt {:.1} - left {:.0} | range {:.0}",
        TEAMS[0].callsign,
        standing[0],
        score.fired[0],
        score.dealt[0],
        pool(0),
        TEAMS[1].callsign,
        standing[1],
        score.fired[1],
        score.dealt[1],
        pool(1),
        range,
    );
}

/// How the frame vantage stands off the fight: direction (broadside to the
/// engagement axis and a little above), plus a floor and a rate on the ships'
/// spread so every hull stays in frame from merge to knife range.
///
/// A camera pose read off transforms, so the two lengths are ENGINE world
/// units - one unit is 10 m.
///
/// The RATE has to clear 0.68 or the fight outgrows the picture: the lens
/// spans 1.47 times its distance at 16:9, so a standoff of `k * spread` frames
/// `1.47 * k * spread` and the formation only fits while that exceeds the
/// spread. 0.80 leaves the widest moment two thirds of the frame.
///
/// The FLOOR is what the frame settles to when the ships merge, and it used to
/// be 55 - 550 m of standoff at knife range, where the whole fight was a fifth
/// of the frame and each 85 m hull about a tenth of it. At 20 the pass
/// itself is the shot.
const CAMERA_DIRECTION: Vec3 = Vec3::new(0.0, 0.45, 1.0);
const CAMERA_BASE: f32 = 20.0;
const CAMERA_PER_SPREAD: f32 = 0.80;

/// Where the tactical overview stands: steeply above the fight and leaning
/// back off it, at a multiple of the frame standoff. High enough to hold a
/// whole engagement of lines rather than a duel, tilted enough that the hulls
/// keep their silhouettes instead of reading as plan-view dots.
const OVERVIEW_DIRECTION: Vec3 = Vec3::new(0.0, 1.0, 0.5);
const OVERVIEW_STANDOFF: f32 = 1.9;

/// How far a follow pose stands behind its ship and how far above it: close
/// enough to read as attached, far enough that the widest roll's stern never
/// fills the frame. "Behind" is measured on the THREAT AXIS - the line from
/// the hull to the living enemies' mean position - so the shot is over the
/// shoulder: subject low in the foreground, the fight it is closing on ahead.
/// [`FOLLOW_LEAD`] only matters in the no-enemies fallback, where the camera
/// chases the hull's own heading and leads it by this much.
const FOLLOW_BACK: f32 = 34.0;
const FOLLOW_LIFT: f32 = 10.0;
const FOLLOW_LEAD: f32 = 12.0;
/// How the CAPTURE framing stands off the hull it frames: back along the threat
/// axis, out to one side of it and lifted, in engine world units.
///
/// ~320 m out, against the frame vantage's kilometre. The lens spans 1.47 times
/// its distance, so the shot is ~470 m wide: an 85 m hull fills a fifth of it
/// - close enough to read the plating, the open irises and a muzzle flash - and
/// a 300 m warhead going off on that hull still fits. Standing BEHIND the
/// subject on the threat axis is what puts incoming ordnance on a line into the
/// middle of frame instead of across a corner of it.
#[cfg(feature = "debug")]
const CINEMA_BACK: f32 = 25.0;
#[cfg(feature = "debug")]
const CINEMA_SIDE: f32 = 18.0;
#[cfg(feature = "debug")]
const CINEMA_LIFT: f32 = 8.0;

/// Exponential smoothing rate (1/s) on the follow pose's aim point. The mean
/// enemy position JUMPS when a ship dies or a reload lands; at 1.2 the camera
/// crosses ~70% of such a swing in the first second and settles in about
/// three - a deliberate pan, not a snap.
const FOLLOW_AIM_RATE: f32 = 1.2;

/// How many roster slots the number row can follow: `1` through `4`.
const FOLLOW_SLOTS: usize = 4;

/// The camera pose in charge. Every pose except `Free` is recomputed from the
/// live fight each frame, so it keeps its subject framed while the ships
/// move; `Free` writes nothing and the free-fly rig keeps whatever the viewer
/// flies.
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
enum Vantage {
    /// `Q`: the whole fight in frame off its midpoint - the default, the
    /// capture framing, and the only pose the idle orbit ever takes over.
    Frame,
    /// `E`: the tactical overview, high and wide over the engagement, holding
    /// its bearing.
    Overview,
    /// `1`..`4`: over the shoulder of one roster slot, looking across it at
    /// the living enemies' mean position.
    Follow(usize),
    /// The staged strike's framing: one roster slot close in, with the fight
    /// it is trading with down the threat axis beyond it. Bound to no key - a hand-run
    /// composes with `Q`, `E` and the number row - because it is the pose a
    /// capture is shot from and nothing else, and the whole staging is behind
    /// the debug feature the capture harness lives in.
    #[cfg(feature = "debug")]
    Cinema(usize),
    /// The viewer took the camera; no pose writes until a camera key re-arms
    /// one.
    Free,
}

/// The camera bindings: the number row FOLLOWS, because the roster is a
/// numbered list and the ships are what a viewer wants one key each for, with
/// the two whole-fight vantages on `Q` and `E` beside them. All six are clear
/// of the free-fly rig (WASD, mouse, Space/Shift).
const VANTAGE_KEYS: [(KeyCode, Vantage); 6] = [
    (KeyCode::KeyQ, Vantage::Frame),
    (KeyCode::KeyE, Vantage::Overview),
    (KeyCode::Digit1, Vantage::Follow(0)),
    (KeyCode::Digit2, Vantage::Follow(1)),
    (KeyCode::Digit3, Vantage::Follow(2)),
    (KeyCode::Digit4, Vantage::Follow(3)),
];

/// Arm the pose under a camera key. A chosen pose is attention, exactly like
/// flying: the idle orbit stands down and its resume clock starts over.
fn select_vantage(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut vantage: ResMut<Vantage>,
    mut orbit: ResMut<IdleOrbit>,
) {
    for (key, pose) in VANTAGE_KEYS {
        if keyboard.just_pressed(key) {
            *vantage = pose;
            orbit.idle_secs = 0.0;
        }
    }
}

/// Hand back the camera the moment the free-fly rig is asked for anything,
/// reading the rig's own input component so a binding change cannot leave a
/// pose fighting the player.
fn free_camera_on_input(mut vantage: ResMut<Vantage>, q_input: Query<&WASDCameraInput>) {
    if *vantage == Vantage::Free {
        return;
    }
    let touched = q_input
        .iter()
        .any(|input| input.pan != Vec2::ZERO || input.wasd != Vec2::ZERO || input.vertical != 0.0);
    if touched {
        *vantage = Vantage::Free;
    }
}

/// The live fight as the camera reads it: the midpoint of every standing
/// COMBATANT root, the spread-derived standoff that keeps the whole engagement
/// in frame, the transform and team of each followable roster slot whose ship
/// still stands, and each team's summed positions for the follow poses' enemy
/// mean. Wrecks keep their root marker, so a pose holds the aftermath too
/// instead of snapping away on the kill.
struct FightRead {
    midpoint: Vec3,
    standoff: f32,
    followed: [Option<(Transform, usize)>; FOLLOW_SLOTS],
    team_sum: [Vec3; TEAMS.len()],
    team_count: [usize; TEAMS.len()],
}

impl FightRead {
    /// The mean position of every standing ship NOT on `team` - the point a
    /// follow camera looks over its subject toward. `None` once the rival
    /// team is gone, which is the heading-chase fallback's cue.
    fn hostile_mean(&self, team: usize) -> Option<Vec3> {
        let mut sum = Vec3::ZERO;
        let mut count = 0;
        for rival in 0..TEAMS.len() {
            if rival != team {
                sum += self.team_sum[rival];
                count += self.team_count[rival];
            }
        }
        (count > 0).then(|| sum / count as f32)
    }
}

/// The camera-side ship query: transforms plus the scenario id that names a
/// root's roster slot and the allegiance that names its team, shared by the
/// poses and the idle orbit.
type ShipQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        Option<&'static EntityId>,
        Option<&'static Allegiance>,
    ),
    (With<SpaceshipRootMarker>, Without<ScenarioCameraMarker>),
>;

/// The roster slot a scenario id names, when it is one of the arena's own
/// fighters.
fn fighter_slot(id: &EntityId) -> Option<usize> {
    id.0.strip_prefix(FIGHTER_ID_PREFIX)?.parse().ok()
}

/// [`fighter_slot`], capped to the slots the follow row can reach.
fn follow_slot(id: &EntityId) -> Option<usize> {
    fighter_slot(id).filter(|slot| *slot < FOLLOW_SLOTS)
}

fn read_fight(q_ships: &ShipQuery) -> Option<FightRead> {
    let mut positions = Vec::new();
    let mut followed = [None; FOLLOW_SLOTS];
    let mut team_sum = [Vec3::ZERO; TEAMS.len()];
    let mut team_count = [0usize; TEAMS.len()];
    for (transform, id, allegiance) in q_ships {
        // No allegiance = a derelict: scenery, not a subject. Counting the
        // junk blobs here would drag the auto-frame's midpoint (and the
        // orbit's pivot) off the fight and toward the flanks.
        let Some(team) = allegiance.and_then(team_of) else {
            continue;
        };
        positions.push(transform.translation);
        team_sum[team] += transform.translation;
        team_count[team] += 1;
        if let Some(slot) = id.and_then(follow_slot) {
            followed[slot] = Some((*transform, team));
        }
    }
    if positions.is_empty() {
        return None;
    }
    let midpoint = positions.iter().sum::<Vec3>() / positions.len() as f32;
    let spread = positions
        .iter()
        .map(|position| position.distance(midpoint))
        .fold(0.0f32, f32::max)
        * 2.0;
    Some(FightRead {
        midpoint,
        standoff: CAMERA_BASE + spread * CAMERA_PER_SPREAD,
        followed,
        team_sum,
        team_count,
    })
}

/// One resolved camera pose: where to stand, what to look at, which way is
/// up.
struct Pose {
    stand: Vec3,
    target: Vec3,
    up: Vec3,
}

/// The default framing: stand on the fight's midpoint, backed off with the
/// ships' spread, broadside to the engagement axis and a little above.
fn frame_pose(fight: &FightRead) -> Pose {
    Pose {
        stand: fight.midpoint + CAMERA_DIRECTION.normalize() * fight.standoff,
        target: fight.midpoint,
        up: Vec3::Y,
    }
}

/// The follow poses' smoothed aim point, and the slot it belongs to. The raw
/// aim - the living enemies' mean - JUMPS when a ship dies; chasing it through
/// an exponential lag turns the jump into a pan. Keyed by slot so switching
/// subjects SNAPS instead of sweeping the camera through a stale bearing from
/// the last hull followed.
#[derive(Resource, Default)]
struct FollowAim {
    slot: Option<usize>,
    point: Vec3,
}

/// The over-the-shoulder pose: stand behind the followed hull on the threat
/// axis, look across it at the (smoothed) mean of the living enemies. With no
/// living enemy there is no threat axis, so the pose chases the hull's own
/// heading instead - the aftermath framing.
fn follow_pose(slot: usize, fight: &FightRead, aim: &mut FollowAim, dt: f32) -> Option<Pose> {
    let (ship, team) = fight.followed.get(slot).copied().flatten()?;
    let Some(threat) = fight.hostile_mean(team) else {
        aim.slot = None;
        return Some(Pose {
            stand: ship.translation + ship.back() * FOLLOW_BACK + Vec3::Y * FOLLOW_LIFT,
            target: ship.translation + ship.forward() * FOLLOW_LEAD,
            up: Vec3::Y,
        });
    };
    let point = if aim.slot == Some(slot) {
        aim.point.lerp(threat, 1.0 - (-dt * FOLLOW_AIM_RATE).exp())
    } else {
        threat
    };
    aim.slot = Some(slot);
    aim.point = point;
    // A threat directly overhead has no horizontal axis to stand back along;
    // the hull's own stern is the one bearing that always exists.
    let axis = (point - ship.translation)
        .try_normalize()
        .unwrap_or(*ship.back());
    Some(Pose {
        stand: ship.translation - axis * FOLLOW_BACK + Vec3::Y * FOLLOW_LIFT,
        target: point,
        up: Vec3::Y,
    })
}

/// The staged strike's pose: stand behind and off the shoulder of one roster
/// slot and look AT that hull, so the ordnance arriving on it is the subject
/// and the enemy trading with it sits beyond.
///
/// [`follow_pose`] looks the other way - past the subject at the fight - which
/// is the right read for a viewer flying along and the wrong one for a warhead
/// arriving. The side vector is taken against world up rather than the hull's,
/// so a rolling ship does not roll the horizon with it.
#[cfg(feature = "debug")]
fn cinema_pose(slot: usize, fight: &FightRead) -> Option<Pose> {
    let (ship, team) = fight.followed.get(slot).copied().flatten()?;
    let axis = fight
        .hostile_mean(team)
        .and_then(|threat| (threat - ship.translation).try_normalize())
        .unwrap_or(*ship.forward());
    let side = axis.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
    Some(Pose {
        stand: ship.translation - axis * CINEMA_BACK + side * CINEMA_SIDE + Vec3::Y * CINEMA_LIFT,
        target: ship.translation,
        up: Vec3::Y,
    })
}

/// Resolve the armed vantage against the live fight. A pose that needs a ship
/// the fight no longer has - a follow slot the roster never filled, or one
/// whose hull is dead - falls back to the frame pose rather than freezing.
fn vantage_pose(vantage: Vantage, fight: &FightRead, aim: &mut FollowAim, dt: f32) -> Option<Pose> {
    match vantage {
        Vantage::Free => None,
        Vantage::Frame => Some(frame_pose(fight)),
        Vantage::Overview => Some(Pose {
            stand: fight.midpoint
                + OVERVIEW_DIRECTION.normalize() * fight.standoff * OVERVIEW_STANDOFF,
            target: fight.midpoint,
            up: Vec3::Y,
        }),
        Vantage::Follow(slot) => {
            follow_pose(slot, fight, aim, dt).or_else(|| Some(frame_pose(fight)))
        }
        #[cfg(feature = "debug")]
        Vantage::Cinema(slot) => cinema_pose(slot, fight).or_else(|| Some(frame_pose(fight))),
    }
}

/// Write the armed pose onto the scenario camera.
fn pose_vantage_camera(
    vantage: Res<Vantage>,
    time: Res<Time>,
    mut aim: ResMut<FollowAim>,
    q_ships: ShipQuery,
    mut q_camera: Query<&mut Transform, With<ScenarioCameraMarker>>,
) {
    if !matches!(*vantage, Vantage::Follow(_)) {
        // Forget the aim whenever nothing is following: coming BACK to a
        // follow after a spell on another pose should open on the live threat,
        // not pan in from wherever the fight stood minutes ago.
        aim.slot = None;
    }
    let Some(fight) = read_fight(&q_ships) else {
        return;
    };
    let Some(pose) = vantage_pose(*vantage, &fight, &mut aim, time.delta_secs()) else {
        return;
    };
    for mut camera in &mut q_camera {
        *camera = Transform::from_translation(pose.stand).looking_at(pose.target, pose.up);
    }
}

/// Radians per second the idle orbit turns at. Slow enough to read the fight
/// without smearing it, and to sit under a capture's own framing.
const ORBIT_RATE: f32 = 0.25;

/// Seconds the free-fly rig and the pose keys must sit untouched, in the
/// frame vantage, before the orbit re-arms.
///
/// Six: long enough that a viewer pausing over a detail is not yanked away
/// the moment their hands leave the keys, short enough that a parked window
/// goes back to turning before it reads as frozen.
const ORBIT_RESUME_SECS: f32 = 6.0;

/// The idle orbit's state: whether it may ever run, how long the viewer has
/// sat quiet, and the bearing the orbit stands at.
///
/// The angle is a PHASE that is stepped, not read off the clock: the clock
/// keeps counting while the viewer flies, so `elapsed * ORBIT_RATE` would
/// teleport a re-armed camera onto whatever bearing it had drifted to.
/// Holding the phase, and re-deriving it from the parked camera on each
/// re-arm, is what lets the orbit pick up from where the viewer left it.
#[derive(Resource)]
struct IdleOrbit {
    /// Never set under a capture: a capture composes its own frame, and an
    /// orbit under it would photograph a different bearing every run.
    enabled: bool,
    /// Seconds since the free-fly rig last reported input or a pose key was
    /// pressed.
    idle_secs: f32,
    /// The orbit's current azimuth around the fight's midpoint, in radians.
    angle: f32,
    /// Whether the orbit owned the camera last frame, so the first re-armed
    /// frame can read the parked bearing before the orbit writes over it.
    driving: bool,
}

impl IdleOrbit {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            // Born idle-for-long-enough, so a fresh hand-run orbits at once.
            idle_secs: ORBIT_RESUME_SECS,
            angle: 0.0,
            driving: false,
        }
    }
}

/// Hand back the camera the moment the free-fly rig is asked for anything,
/// and count the quiet seconds that re-arm the orbit once the flying stops.
///
/// Reads the rig's own input component rather than the keyboard, so it cannot
/// disagree with what actually moves the camera - and so a binding change does
/// not silently leave the orbit fighting the player.
fn track_orbit_idle(
    mut orbit: ResMut<IdleOrbit>,
    time: Res<Time>,
    q_input: Query<&WASDCameraInput>,
) {
    let touched = q_input
        .iter()
        .any(|input| input.pan != Vec2::ZERO || input.wasd != Vec2::ZERO || input.vertical != 0.0);
    if touched {
        orbit.idle_secs = 0.0;
    } else {
        // Saturated at the threshold: the timer is a re-arm gate, not a
        // stopwatch, so there is nothing to count past it.
        orbit.idle_secs = (orbit.idle_secs + time.delta_secs()).min(ORBIT_RESUME_SECS);
    }
}

/// Sweep the fight on a slow turntable while nobody is flying, the way
/// `wfc_ships` turns its row - except the pivot MOVES: the orbit re-centres
/// on the live midpoint every frame, and the spread-derived standoff keeps
/// the whole engagement in frame at every bearing. Runs after the free-fly rig
/// writes its transform, because that rig writes every frame and would
/// otherwise win.
///
/// The orbit is the FRAME vantage's idle behaviour, not an overlay on every
/// pose: a viewer who chose the overview or a follow chose that view for as
/// long as they care to hold it, so only `Vantage::Frame` ever re-arms the
/// turntable.
///
/// On re-arm the azimuth is read off the parked camera's own xz offset from
/// the midpoint, so the orbit drifts on from wherever the viewer - or the
/// vantage pose it took over from - left it.
fn orbit_idle_camera(
    mut orbit: ResMut<IdleOrbit>,
    vantage: Res<Vantage>,
    time: Res<Time>,
    q_ships: ShipQuery,
    mut q_camera: Query<&mut Transform, With<ScenarioCameraMarker>>,
) {
    if !orbit.enabled || *vantage != Vantage::Frame {
        return;
    }
    if orbit.idle_secs < ORBIT_RESUME_SECS {
        orbit.driving = false;
        return;
    }
    let Some(fight) = read_fight(&q_ships) else {
        return;
    };
    if !orbit.driving {
        let Some(parked) = q_camera.iter().next() else {
            return;
        };
        let offset = parked.translation - fight.midpoint;
        orbit.angle = offset.x.atan2(offset.z);
        orbit.driving = true;
    }
    orbit.angle += time.delta_secs() * ORBIT_RATE;
    // The frame vantage's own ring: bearing zero IS the frame pose, so the
    // orbit at rest and the auto-frame agree and a resume never jumps.
    let ring = Vec3::new(
        orbit.angle.sin() * CAMERA_DIRECTION.z,
        CAMERA_DIRECTION.y,
        orbit.angle.cos() * CAMERA_DIRECTION.z,
    );
    for mut camera in &mut q_camera {
        *camera = Transform::from_translation(fight.midpoint + ring.normalize() * fight.standoff)
            .looking_at(fight.midpoint, Vec3::Y);
    }
}

/// The chevron's geometry: the HUD allegiance-marker triangle verbatim - a
/// zero-content `ContentBox` node whose coloured top border renders as a
/// filled down-pointing triangle - floated the same 40 px above the hull.
/// The same numbers on purpose: this IS that visual language, re-tinted.
const CHEVRON_HALF_WIDTH_PX: f32 = 7.0;
const CHEVRON_HEIGHT_PX: f32 = 9.0;
const CHEVRON_SIZE: Vec2 = Vec2::new(2.0 * CHEVRON_HALF_WIDTH_PX, CHEVRON_HEIGHT_PX);
const CHEVRON_OFFSET: Vec2 = Vec2::new(0.0, -40.0);

/// One team-chevron layer, and the fighter root it tracks.
#[derive(Component)]
struct TeamChevron(Entity);

/// The chevron's projected indicator node - the one the visibility gate must
/// overwrite, because the projection re-asserts `Visibility::Visible` on it
/// every frame.
#[derive(Component)]
struct TeamChevronIndicator;

/// Tag the scenario camera as the screen-indicator projector. The game only
/// tags the player's chase camera and this arena has no player, so without
/// this every indicator - the chevrons included - hides. `Added` re-fires per
/// reload: `LoadScenario` tears the camera down and spawns a fresh one.
fn tag_indicator_camera(mut commands: Commands, q_new: Query<Entity, Added<ScenarioCameraMarker>>) {
    for camera in &q_new {
        commands.entity(camera).insert(ScreenIndicatorCamera);
    }
}

/// Retire the stock HUD allegiance markers for every arena-spawned wfc ship:
/// the fighters wear team chevrons instead (the stock triangle is semantic
/// ally/threat, and its recolour system owns that tint), and the junk must
/// wear nothing - twenty grey neutral triangles over the debris blobs would
/// read as a contact swarm. Every frame rather than an observer, because the
/// stock layers spawn from a deferred observer command on every (re)load.
fn retire_stock_markers(
    mut commands: Commands,
    q_layers: Query<(Entity, &AllegianceMarkerTargetEntity), With<AllegianceMarkerHudMarker>>,
    q_ids: Query<&EntityId>,
) {
    for (layer, target) in &q_layers {
        let Ok(id) = q_ids.get(**target) else {
            continue;
        };
        if id.0.starts_with(FIGHTER_ID_PREFIX) || id.0.starts_with(DERELICT_ID_PREFIX) {
            commands.entity(layer).despawn();
        }
    }
}

/// Spawn a chevron over every fighter as it lands: an indicator anchored on
/// the ship root with the border-triangle under it, tinted by team
/// ([`Team::tint`]). Junk never matches [`FIGHTER_ID_PREFIX`], so it can
/// never grow one. Fixed-size, so it reads the same at overview range and in
/// a follow - that is the point of a screen-space marker.
fn spawn_team_chevrons(
    mut commands: Commands,
    roster: Res<Roster>,
    q_new: Query<(Entity, &EntityId, Option<&Allegiance>), Added<SpaceshipRootMarker>>,
) {
    for (ship, id, allegiance) in &q_new {
        let Some(slot) = fighter_slot(id) else {
            continue;
        };
        // The stock HUD skips the player's own marker (you know where you
        // are); the team chevron skips the slot the viewer flies for the
        // same reason.
        if roster.player_slot() == Some(slot) {
            continue;
        }
        let Some(team) = allegiance.and_then(team_of) else {
            continue;
        };
        commands.spawn((
            Name::new("TeamChevron"),
            TeamChevron(ship),
            screen_indicator_layer(),
            children![(
                TeamChevronIndicator,
                screen_indicator(ScreenIndicatorConfig {
                    anchor: Some(ScreenIndicatorAnchorKind::Entity(ship)),
                    size: ScreenIndicatorSize::Fixed(CHEVRON_SIZE),
                    offset: CHEVRON_OFFSET,
                    offscreen: ScreenIndicatorOffscreen::Hide,
                }),
                children![(
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        // Zero CONTENT box: the borders below are the whole
                        // visible shape, and the default BorderBox would
                        // collapse them to nothing.
                        width: Val::Px(0.0),
                        height: Val::Px(0.0),
                        box_sizing: BoxSizing::ContentBox,
                        border: UiRect {
                            left: Val::Px(CHEVRON_HALF_WIDTH_PX),
                            right: Val::Px(CHEVRON_HALF_WIDTH_PX),
                            top: Val::Px(CHEVRON_HEIGHT_PX),
                            bottom: Val::Px(0.0),
                        },
                        ..default()
                    },
                    BorderColor {
                        top: TEAMS[team].tint,
                        left: Color::NONE,
                        right: Color::NONE,
                        bottom: Color::NONE,
                    },
                    Pickable::IGNORE,
                )],
            )],
        ));
    }
}

/// The chevrons follow the arena visibility rule, not the HUD tiers: on
/// with the HUD in a hand-run (grave/tilde round-trips them) and always on in
/// a capture, where two identifiable sides are part of the frame's evidence.
///
/// PostUpdate, after the projection, because that is the only place the rule
/// can land: `update_screen_indicators` re-asserts `Visibility::Visible` on
/// every on-screen indicator each frame (ancestor visibility included), so a
/// "hidden" chevron must be overwritten downstream - the same ordering the
/// HUD's own `apply_hud_visibility` uses. While shown, the projection's
/// on-screen/off-screen answer is left alone.
/// OPTIONAL because `NovaHudPlugin` is render-gated (`AppBuilder::build`), so a
/// `--norender` run has no [`HudVisibility`] to follow - and a required `Res`
/// there is not a skipped system but a PANIC that takes the arena down before
/// it fields a ship.
fn gate_team_chevrons(
    hud: Option<Res<HudVisibility>>,
    mut q_indicators: Query<&mut Visibility, With<TeamChevronIndicator>>,
) {
    let Some(hud) = hud else {
        return;
    };
    if capturing() || hud.shows() {
        return;
    }
    for mut visibility in &mut q_indicators {
        visibility.set_if_neq(Visibility::Hidden);
    }
}

/// A chevron dies with its fighter: the kill, the wipe and the reload
/// teardown all land here as a vanished root.
fn reap_team_chevrons(
    mut commands: Commands,
    q_layers: Query<(Entity, &TeamChevron)>,
    q_ships: Query<(), With<SpaceshipRootMarker>>,
) {
    for (layer, chevron) in &q_layers {
        if q_ships.get(chevron.0).is_err() {
            commands.entity(layer).despawn();
        }
    }
}

/// The capture profile the arena records at: the documentation profile with a
/// coarser CRF, and coarser again for the 2v2.
///
/// This is the busiest scene the site ships. Two hulls trading tracers is
/// already the highest-entropy frame in the set, and the 2v2 doubles it: four
/// hulls, two salvos of ordnance, point defense answering all of it, and the
/// debris, shed cladding and fireballs a duel leaves. VP9 spends bitrate on
/// exactly that, and at the fleet CRF the 2v2 encoded 3.2 MB against the
/// packager's 3 MB per-file budget. Every other loop lands between 100 KB and
/// 900 KB at the fleet setting, so this is the one composition that pays for
/// its own quality rather than the whole set paying for it.
///
/// The two rosters are split because they are not the same picture. The duel
/// is the landing page's lead and holds at 37 with room to spare; the 2v2 is a
/// background band behind text, carries twice the moving debris, and went back
/// over budget the day cladding started flying off a dying hull. It pays the
/// difference alone rather than costing the lead its sharpness.
#[cfg(feature = "debug")]
fn arena_loop_profile(loop_name: &str) -> nova_protocol::nova_debug::harness::LoopProfile {
    nova_protocol::nova_debug::harness::LoopProfile {
        crf: if loop_name == LANDING_2V2_LOOP {
            41
        } else {
            37
        },
        ..default()
    }
}

/// The roster slot the capture frames and aims the strike at: the first hull
/// AMBER fields. Slot 0 is always filled - the default roster opens with it and
/// a `--ship` roster is parsed in order - and it is inside the follow row's
/// reach, which is what lets the pose read it off [`FightRead`].
#[cfg(feature = "debug")]
const CINEMA_SLOT: usize = 0;

/// How near the framed hull a warhead must fuze to count as a hit IN SHOT: one
/// blast radius. A detonation further out than its own pressure sphere lit
/// something else and does not belong to this frame.
const STRIKE_HIT_RANGE: Meters = Meters(300.0);

/// Where the strike is staged from, engine world units: the mark every other
/// combatant is sent to, measured from the subject along the bearing it is
/// already on.
///
/// 1.5 km, and it is squeezed between two numbers that both have to hold.
///
/// The FLOOR is the warhead's. A Serpent's pressure sphere is
/// [`STRIKE_HIT_RANGE`], 300 m across, and an earlier cut staged the pair at
/// 500 m and let them close: by the end of the recording the two hulls were
/// inside one sphere, so a single warhead killed both and the loop read as a
/// mutual annihilation rather than as a duel. At 1.5 km a hit belongs to the
/// ship it lands on with five blast radii to spare, and
/// [`STRIKE_CLOSING_SPEED`] spends under a fifth of the gap across the whole
/// recording.
///
/// The CEILING is the guns'. Every shipped PDC reaches 200 u and the AI holds
/// its fire outside 0.9 of that - 180 u, `AI_FIRE_RANGE_FACTOR` - so a pair
/// staged at 2 km stood outside its own mounts' gate and the loop had no
/// tracers in it at all: two hulls throwing ordnance at each other in silence.
/// Inside the gate the point-defence batteries work the salvo the tubes threw,
/// which is the answering half of the exchange.
///
/// The lens cannot hold both hulls at this range and nothing is gained by
/// trying: it spans 1.47 times its distance, so framing a 1.5 km spread means
/// standing a kilometre off, where an 85 m gunship is a seventeenth of the
/// frame width. The capture stands over the subject's shoulder instead
/// ([`CINEMA_BACK`]), which puts the rival down the threat axis as a lit
/// contact in the middle of frame and every round that crosses between them on
/// a line into the shot.
///
/// What it costs is arrival time: 1.5 km is about four seconds of torpedo
/// flight, so a salvo cued at the top of the recording lands during
/// [`STRIKE_AFTERMATH_SECS`] rather than between the beats.
#[cfg(feature = "debug")]
const STRIKE_STAGE_RANGE: f32 = 150.0;
/// The range the recording is staged at, engine world units: the walk parks the
/// merge the first moment the two sides are this close.
///
/// A hair over [`STRIKE_STAGE_RANGE`], because that is the range the shot
/// wants and this beat only has to stop the walk from flying past it. The AI
/// coasts to about 3 km on its own, so the pair is usually already outside
/// this and `close_the_range` brings it in.
///
/// It used to be the tuned number, back when the staging was knife range and
/// POINT DEFENSE decided everything: parked and held at 550 m the two hulls
/// shot down all thirty-six warheads of a two-wave strike. That is still true
/// and is no longer a failure - at 1.5 km the mounts get part of a salvo, and
/// what the recording is of is the exchange, not the kill.
#[cfg(feature = "debug")]
const STRIKE_CLOSE_BAND: f32 = 160.0;
/// How near [`STRIKE_STAGE_RANGE`] the staged move has to park before the
/// recording opens, engine world units.
///
/// 200 m on 1.5 km. The move order parks on its own arrival standoff and two
/// hulls drifting at [`STRIKE_CLOSING_SPEED`] are never exactly anywhere, so
/// the test is a band; it is tight enough that "staged" still means the range
/// the framing and the warhead were both chosen against.
#[cfg(feature = "debug")]
const STRIKE_STAGE_TOLERANCE: f32 = 20.0;
/// The arrival standoff a staged move flies to, engine world units. The ship's
/// own 500 m is coarser than the whole staging distance.
#[cfg(feature = "debug")]
const STRIKE_ARRIVAL_STANDOFF: f32 = 5.0;

/// How close a staged bore must come before the charge is allowed to run,
/// degrees. Coarse against the AI's own ~8-degree commit gate, and it can be:
/// the hull HOLDS this bearing for the whole strike, so what the tolerance buys
/// is a hull that has stopped swinging rather than a solved firing solution.
#[cfg(feature = "debug")]
const STRIKE_ALIGN_TOLERANCE_DEGREES: f32 = 2.0;

/// The order keys the staging installs under, so a log line about one says
/// which beat of the capture put it there.
#[cfg(feature = "debug")]
const STRIKE_CLOSE_ORDER: &str = "arena_strike_close";
#[cfg(feature = "debug")]
const STRIKE_ALIGN_ORDER: &str = "arena_strike_align";

/// The staged strike: the hull the capture frames, and the two beats the
/// recording waits for.
///
/// The fight stays the AI's. What is scripted is WHEN both sides open up
/// together and where the camera stands while they do, because the AI settles
/// its orbit at ~1 km and the auto-frame backs off with the spread - so an
/// unstaged six seconds is two specks trading tracers, with the torpedoes dying
/// to point defense a kilometre from the lens and the lance never commiting at
/// all. Every gate under the cue is still the shipped one: the bay's cooldown
/// and magazine, the lance's charge, its single shell and its reload, the
/// guidance, the point defense that shoots the ordnance down and the blast that
/// lands when it does not.
///
/// Nothing here is armed outside a capture. A hand-run, a probe pass and the
/// smoke walk fight exactly as they always did.
#[derive(Resource, Default)]
struct Strike {
    /// The framed hull, once the walk has picked it.
    subject: Option<Entity>,
    /// Warheads that have fuzed within [`STRIKE_HIT_RANGE`] of it.
    hits: u32,
    /// Lances that have actually FIRED - the shell leaving, not the trigger
    /// pull, so a gun whose charge was dumped does not count as a shot.
    shots: u32,
}

#[cfg(feature = "debug")]
impl Strike {
    /// A lance has put its shell downrange.
    fn shot(&self) -> bool {
        self.shots > 0
    }

    /// A warhead has gone off on the framed hull.
    fn hit(&self) -> bool {
        self.hits > 0
    }
}

/// Count the warheads that go off on the framed hull.
///
/// A SYSTEM because `Added` is change detection: read from the plain `&World` a
/// harness predicate gets, it is silently always false.
fn count_strike_hits(
    mut strike: ResMut<Strike>,
    q_blast: Query<&Transform, Added<NovaBlast>>,
    q_ships: Query<&Transform, With<SpaceshipRootMarker>>,
) {
    let Some(subject) = strike.subject else {
        return;
    };
    let Ok(hull) = q_ships.get(subject) else {
        return;
    };
    let at = hull.translation;
    let range = STRIKE_HIT_RANGE.to_engine();
    strike.hits += q_blast
        .iter()
        .filter(|blast| blast.translation.distance(at) < range)
        .count() as u32;
}

/// Count a lance discharge the moment the shell leaves.
fn count_strike_shots(_: On<RailgunFired>, mut strike: ResMut<Strike>) {
    strike.shots += 1;
}

/// Every standing combatant root: entity, team, where it is, and the roster
/// slot its scenario id names. Junk carries no allegiance and is filtered out
/// here, exactly as it is in [`read_fight`].
#[cfg(feature = "debug")]
fn combatant_roots(world: &mut World) -> Vec<(Entity, usize, Vec3, Option<usize>)> {
    let mut query = world.query_filtered::<(
        Entity,
        &Transform,
        &Allegiance,
        Option<&EntityId>,
    ), With<SpaceshipRootMarker>>();
    query
        .iter(world)
        .filter_map(|(entity, transform, allegiance, id)| {
            Some((
                entity,
                team_of(allegiance)?,
                transform.translation,
                id.and_then(fighter_slot),
            ))
        })
        .collect()
}

/// The nearest combatant on another team, and where it is.
#[cfg(feature = "debug")]
fn nearest_hostile(
    combatants: &[(Entity, usize, Vec3, Option<usize>)],
    team: usize,
    from: Vec3,
) -> Option<(Entity, Vec3)> {
    combatants
        .iter()
        .filter(|(_, rival, ..)| *rival != team)
        .min_by(|(_, _, a, _), (_, _, b, _)| {
            from.distance_squared(*a)
                .total_cmp(&from.distance_squared(*b))
        })
        .map(|&(entity, _, position, _)| (entity, position))
}

/// Bring the fight back to the staged range: the subject stops where it is,
/// and everything hostile to it flies in to [`STRIKE_STAGE_RANGE`].
///
/// The beat exists because a duel that has PROVED itself is a duel that has
/// already happened. The lines merge at a closing speed neither side brakes
/// off, cross inside 400 m, and are a kilometre and a half apart and opening by
/// the time the scoreboard can say both teams fired and both connected - so the
/// unstaged recording opens on two hulls receding from each other. Both halves
/// are the shipped helm orders (`StopShip` and `MoveShipTo`), flown by the
/// game's own autopilot with its own flip-and-burn, and all of it happens
/// before the loop opens.
#[cfg(feature = "debug")]
fn close_the_range(world: &mut World) {
    let combatants = combatant_roots(world);
    let (subject, subject_at) = strike_subject(&combatants);
    for &(ship, _, position, _) in &combatants {
        cancel_ship_order(world, ship);
        let directive = if ship == subject {
            ShipOrderDirective::Stop
        } else {
            let bearing = (position - subject_at)
                .try_normalize()
                .unwrap_or(Vec3::NEG_Z);
            ShipOrderDirective::Move {
                position: subject_at + bearing * STRIKE_STAGE_RANGE,
                arrival_standoff: Some(STRIKE_ARRIVAL_STANDOFF),
            }
        };
        install_strike_order(world, ship, STRIKE_CLOSE_ORDER, directive);
    }
}

/// Advance once some hostile pair is inside `range` of each other, engine world
/// units - the staging beat's arrival test, read off the fight rather than off
/// an order report so a ship that gives up short still lets the capture go on.
///
/// Range is the WHOLE test, and rest deliberately is not part of it. A staged
/// `Move` hands the helm back the moment it arrives, so the AI is flying again
/// a second later and the pair is never simultaneously close and at rest; a
/// walk that waited for both spent its whole minute waiting and opened on two
/// specks three kilometres apart. Rest is taken rather than waited for -
/// [`settle_the_fight`] replaces the merge speed with a slow closing drift at
/// the staging beat, before the loop opens.
#[cfg(feature = "debug")]
fn fight_within(range: f32) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        let Some(mut query) =
            world.try_query_filtered::<(&Transform, &Allegiance), With<SpaceshipRootMarker>>()
        else {
            return false;
        };
        let ships: Vec<(Vec3, usize)> = query
            .iter(world)
            .filter_map(|(transform, allegiance)| {
                Some((transform.translation, team_of(allegiance)?))
            })
            .collect();
        ships.iter().any(|&(a, team)| {
            ships
                .iter()
                .any(|&(b, rival)| rival != team && a.distance(b) <= range)
        })
    })
}

/// True once the nearest hostile pair stands at `range`, give or take
/// `tolerance` - engine world units.
///
/// A BAND and not a ceiling, which [`fight_within`] is. The staged range is
/// wider than the range the AI picks for itself, so the walk's job here is to
/// push the sides APART and the "are we there yet" test has to be able to say
/// no while they are too close. Gating that on a ceiling is how an earlier cut
/// staged nothing at all: the pair was already inside it when the beat opened,
/// the beat passed on its first frame, and the next one cancelled the orders it
/// had just installed.
#[cfg(feature = "debug")]
fn fight_staged(
    range: f32,
    tolerance: f32,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        let Some(mut query) =
            world.try_query_filtered::<(&Transform, &Allegiance), With<SpaceshipRootMarker>>()
        else {
            return false;
        };
        let ships: Vec<(Vec3, usize)> = query
            .iter(world)
            .filter_map(|(transform, allegiance)| {
                Some((transform.translation, team_of(allegiance)?))
            })
            .collect();
        ships
            .iter()
            .flat_map(|&(a, team)| {
                ships
                    .iter()
                    .filter(move |&&(_, rival)| rival != team)
                    .map(move |&(b, _)| a.distance(b))
            })
            .min_by(f32::total_cmp)
            .is_some_and(|nearest| (nearest - range).abs() <= tolerance)
    })
}

/// The hull the capture frames, and where it is. Fails the run by name: a walk
/// that cannot find its subject would otherwise photograph an empty sky.
#[cfg(feature = "debug")]
fn strike_subject(combatants: &[(Entity, usize, Vec3, Option<usize>)]) -> (Entity, Vec3) {
    combatants
        .iter()
        .find(|(.., slot)| *slot == Some(CINEMA_SLOT))
        .map(|&(entity, _, position, _)| (entity, position))
        .unwrap_or_else(|| {
            panic!("wfc_arena: the capture found no combatant in roster slot {CINEMA_SLOT}")
        })
}

/// Install one staged helm order, replacing whatever the last beat left.
///
/// The three components `ForceAlign` and `MoveShipTo` install, plus the
/// cancellation every install runs first: an AI ship stops FLYING itself while
/// an order owns its helm and picks its routine back up when the order ends. It
/// keeps SHOOTING throughout - only the flight writers stand down - which is
/// what makes the staged lance shot a real one, down a line the hull was
/// actually holding when its charge finished.
#[cfg(feature = "debug")]
fn install_strike_order(world: &mut World, ship: Entity, key: &str, directive: ShipOrderDirective) {
    cancel_ship_order(world, ship);
    let mut entity = world.entity_mut(ship);
    if !entity.contains::<ShipOrderReports>() {
        entity.insert(ShipOrderReports::default());
    }
    entity.insert((
        ShipHelmOrder::new(key.to_string(), directive),
        ShipOrderHelmAuthority,
    ));
}

/// Pick the hull the capture frames, put every combatant's bore on its nearest
/// hostile and hold it there, disarm every lance but the subject's, and cut the
/// camera in behind the subject.
///
/// The bearing is read off transforms and handed to a directive that is
/// compared against avian positions, so both ends are ENGINE world units and
/// nothing converts.
#[cfg(feature = "debug")]
fn stage_the_strike(world: &mut World) {
    let combatants = combatant_roots(world);
    let (subject, _) = strike_subject(&combatants);
    // Zeroed, not just pointed: both counters run from app start, and the walk
    // only reaches here after the AI has fought its way into
    // `STRIKE_CLOSE_BAND` - up to `FIGHT_DEADLINE_SECS` of free fighting. One
    // lance shot or one stray warhead during the approach would otherwise
    // satisfy the beats below on their first evaluation, collapsing
    // `STRIKE_CHARGE_SECS` and `STRIKE_SALVO_GAP_SECS` to a single frame each.
    // Zeroing here is the whole fix because `disarm_the_rival_lances` runs a
    // line later: from this frame on, the subject's is the only lance that can
    // fire at all.
    *world.resource_mut::<Strike>() = Strike {
        subject: Some(subject),
        hits: 0,
        shots: 0,
    };
    world.insert_resource(Vantage::Cinema(CINEMA_SLOT));
    settle_the_fight(world, &combatants);
    disarm_the_rival_lances(world, subject);

    let tolerance = STRIKE_ALIGN_TOLERANCE_DEGREES.to_radians();
    for &(ship, team, position, _) in &combatants {
        let Some((_, mark)) = nearest_hostile(&combatants, team, position) else {
            continue;
        };
        install_strike_order(
            world,
            ship,
            STRIKE_ALIGN_ORDER,
            ShipOrderDirective::Align {
                look_at: mark,
                tolerance,
            },
        );
    }
}

/// What a staged hull is left doing, engine world units per second: 15 m/s
/// each, so the pair closes at 30.
///
/// Not zero. A pair with the velocity written flat out of them holds its gap to
/// the metre for the whole recording, and a fight photographs as a diorama -
/// the stillness is the first thing an eye picks up, before any of the weapons.
///
/// Aimed rather than merely capped, because the direction is the second thing
/// the recording needs: the salvo crosses a range that is shrinking under it.
/// And SLOW, because the first cut of this closed at 50 m/s and spent 350 m of
/// the gap it was given - the pair ended the loop nose to nose, which is the
/// framing the closure was added to fix. Thirty a second is under 300 m across
/// the whole recording, a fifth of [`STRIKE_STAGE_RANGE`]: motion the eye
/// reads, and a gap that is still a gap at the end of it.
#[cfg(feature = "debug")]
const STRIKE_CLOSING_SPEED: f32 = 1.5;

/// Take the merge speed off every combatant and leave it closing slowly on its
/// nearest hostile, so the `Align` that follows holds a pair that stays in
/// frame without freezing in it.
///
/// `Align` turns a hull WITHOUT translating it: whatever speed the approach
/// left on the hull is speed the recording inherits, and two hulls coasting off
/// a 200 m/s merge are three kilometres apart by the time the torpedoes arrive.
/// Braking them with an order instead would cost most of a minute of a fight
/// that decides itself in one, so the velocities are written directly.
///
/// This is a cut, and it belongs to the STAGING - it happens before the loop
/// opens, so nothing on film jumps. What the recording then shows is the
/// shipped fight: real tubes, real charges, real point defense, real blasts.
#[cfg(feature = "debug")]
fn settle_the_fight(world: &mut World, combatants: &[(Entity, usize, Vec3, Option<usize>)]) {
    use avian3d::prelude::{AngularVelocity, LinearVelocity};

    for &(ship, team, position, _) in combatants {
        let closing = nearest_hostile(combatants, team, position)
            .and_then(|(_, mark)| (mark - position).try_normalize())
            .map_or(Vec3::ZERO, |bearing| bearing * STRIKE_CLOSING_SPEED);
        let mut entity = world.entity_mut(ship);
        if let Some(mut velocity) = entity.get_mut::<LinearVelocity>() {
            velocity.0 = closing;
        }
        if let Some(mut spin) = entity.get_mut::<AngularVelocity>() {
            spin.0 = Vec3::ZERO;
        }
    }
}

/// Advance once every hull the strike aligned has settled on its bearing.
///
/// False while any is still swinging, and false before the orders are in - so
/// the beat it gates carries a time cap beside it: a hull that lost its flight
/// computer cannot turn, and a capture must degrade into a worse shot rather
/// than stall on one.
#[cfg(feature = "debug")]
fn bores_settled() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(mut query) =
            world.try_query_filtered::<Has<ScriptedAlignSettled>, With<ScriptedAlign>>()
        else {
            return false;
        };
        let mut aligning = false;
        for settled in query.iter(world) {
            aligning = true;
            if !settled {
                return false;
            }
        }
        aligning
    })
}

/// Who each combatant is shooting at: its nearest hostile.
#[cfg(feature = "debug")]
fn strike_targets(combatants: &[(Entity, usize, Vec3, Option<usize>)]) -> BTreeMap<Entity, Entity> {
    combatants
        .iter()
        .filter_map(|&(ship, team, position, _)| {
            nearest_hostile(combatants, team, position).map(|(hostile, _)| (ship, hostile))
        })
        .collect()
}

/// Run the charge on the SUBJECT's lance.
///
/// The lance is cued FIRST and on its own beat, because the two weapons are on
/// different clocks and the shorter one buries the longer: the charge is 1.5 s,
/// the torpedoes cross the staged range in under half of that, and a hull that
/// has just lost its controller to a warhead never finishes charging. Cued
/// first, the slug leaves while both hulls are whole and the salvo arrives
/// behind it.
///
/// The subject's alone, because a spinal lance is not a thing a picture can
/// hold twice. It is one flash and a slug that crosses 1.5 km in a tenth of a
/// second, so the near hull's shot is three frames of the loop and the rival's
/// is three frames of a contact the frame is a kilometre and a half away from
/// - unreadable in itself, and it takes the near hull's bow out from under the
/// camera on arrival. [`disarm_the_rival_lances`] is what makes that true of
/// the AI's own trigger as well as of this cue.
///
/// The shipped scripted-weapon seam, not a new fire path - `ScriptedRailgunOrder`
/// is the component `ForceRailgunFire` installs - and it is ONE-SHOT: it retires
/// itself on the discharge, so this is a cue and not a held trigger.
#[cfg(feature = "debug")]
fn cue_the_lances(world: &mut World) {
    let combatants = combatant_roots(world);
    let (subject, _) = strike_subject(&combatants);
    let lances: Vec<Entity> = world
        .query_filtered::<(Entity, &ChildOf), With<RailgunSectionMarker>>()
        .iter(world)
        .filter(|(_, ChildOf(parent))| *parent == subject)
        .map(|(section, _)| section)
        .collect();
    for lance in lances {
        world.entity_mut(lance).insert(ScriptedRailgunOrder);
    }
}

/// Leave the framed hull the only lance in the arena.
///
/// Every hull the arena collapses carries a bow gun - that is what the tile set
/// is for - and the AI pulls its own lance trigger the moment the bore comes on
/// and the target is inside 0.6 of an 18 km reach, which at the staged range is
/// always. So not cueing the rival's lance does not keep it quiet;
/// `SectionInactiveMarker` does. It is the shipped "this section is not
/// working" component, and BOTH fire paths already read it - the AI's trigger
/// system and `charge_and_fire_railgun` alike skip a section carrying it - so a
/// disarmed lance is a lance that cannot fire rather than one nothing happens
/// to be asking.
///
/// A staging cut, like [`settle_the_fight`]: it lands before the loop opens, on
/// a hull a kilometre and a half away, and nothing on film changes when it does.
#[cfg(feature = "debug")]
fn disarm_the_rival_lances(world: &mut World, subject: Entity) {
    let lances: Vec<Entity> = world
        .query_filtered::<(Entity, &ChildOf), With<RailgunSectionMarker>>()
        .iter(world)
        .filter(|(_, ChildOf(parent))| *parent != subject)
        .map(|(section, _)| section)
        .collect();
    for lance in lances {
        world.entity_mut(lance).insert(SectionInactiveMarker);
    }
}

/// Pull every tube on both sides at once, homing on that ship's nearest hostile.
///
/// `ScriptedTorpedoOrder` is the component `ForceTorpedoFire` installs, and it
/// is ONE-SHOT the same way: an alpha strike, not a held trigger. The AI's own
/// trigger systems run in the input set and these holds in the section set
/// after it, so the cue wins the frame it is armed in without either side
/// having to know about the other.
///
/// Both sides, not just the shooter: the framed hull's own tubes are what the
/// camera is closest to, and a strike only one side throws reads as a firing
/// range rather than a fight.
#[cfg(feature = "debug")]
fn cue_the_tubes(world: &mut World) {
    let combatants = combatant_roots(world);
    let targets = strike_targets(&combatants);
    let bays: Vec<(Entity, Entity)> = world
        .query_filtered::<(Entity, &ChildOf), With<TorpedoSectionMarker>>()
        .iter(world)
        .map(|(section, ChildOf(parent))| (section, *parent))
        .collect();
    for (section, parent) in bays {
        if let Some(&target) = targets.get(&parent) {
            world
                .entity_mut(section)
                .insert(ScriptedTorpedoOrder { target });
        }
    }
}

/// Seconds the fight gets to prove itself: both teams firing and both dealt
/// damage. The cold opening now sits IN FRONT of the predicate - grace plus
/// the passive closing to [`ENGAGE_RANGE`] spends ~15-25 s before a shot is
/// even legal - so this is sized for approach plus fight, still UNDER the
/// harness completion watchdog's 120 s default: a fight that never happens
/// fails naming THIS step - the honest failure mode - instead of the
/// watchdog's anonymous laggard exit. A slow matchup can be given more room
/// with `NOVA_AUTOPILOT_DEADLINE`.
#[cfg(feature = "debug")]
const FIGHT_DEADLINE_SECS: f32 = 100.0;

/// Seconds the staged approach gets to bring the sides back inside
/// [`STRIKE_CLOSE_BAND`]. A hull braking off a 200 m/s merge and flying back in
/// spends twenty of them, and none of that is on film - but the cap is what
/// decides how much of the fight is spent staging, and this fight decides
/// itself in about a minute. Past this the walk parks whatever range it has
/// rather than filming a hull that has already lost its controller.
#[cfg(feature = "debug")]
const STRIKE_CLOSE_SECS: f32 = 25.0;
/// Seconds the staged bearings get to settle before the loop opens. Two hulls
/// this size take a couple of seconds to swing; past that the shot is worth
/// more than the bore.
#[cfg(feature = "debug")]
const STRIKE_ALIGN_SECS: f32 = 4.0;
/// The LEAD the lance gets over the tubes, seconds - not its whole charge.
///
/// Cueing the lance first is what stops the salvo burying it: the charge is
/// 1.5 s, so a second of head start puts the slug downrange before the tubes
/// clear. The slug crosses [`STRIKE_STAGE_RANGE`] in a fraction of the four
/// seconds the warheads behind it need, which is the order the two weapons are
/// meant to read in.
#[cfg(feature = "debug")]
const STRIKE_CHARGE_SECS: f32 = 0.9;
/// Seconds between the two salvos, and the reason there are two.
///
/// One alpha strike does not get through. Six point-defence mounts a side
/// engage an inbound each and reload faster than a torpedo crosses the range
/// between them, so a single salvo is shot down to the last warhead - thirty-
/// six of them, on the run that established this. What beats a battery is
/// SATURATION, and the bay supplies it: this is the tube cooldown, so the
/// second cue is the next launch the magazine allows and not a second trigger
/// invented for the camera. The mounts are still busy with the first wave when
/// the second arrives.
#[cfg(feature = "debug")]
const STRIKE_SALVO_GAP_SECS: f32 = 1.4;
/// Seconds the recording waits on the second salvo before it settles for what
/// it has. The tubes take about a second to clear; the rest is the wave
/// leaving the ship and crossing into the shot, which at
/// [`STRIKE_STAGE_RANGE`] is most of what this beat is for - the warheads are
/// still in flight when it ends, and [`STRIKE_AFTERMATH_SECS`] holds for the
/// arrival.
#[cfg(feature = "debug")]
const STRIKE_WINDOW_SECS: f32 = 4.0;
/// Seconds held after the beats land, so the last fireball blooms and fades
/// inside the loop instead of being cut off by it.
///
/// It carries the crossing as well now. At [`STRIKE_STAGE_RANGE`] a salvo cued
/// at the top of the recording needs about four seconds to arrive, which is
/// after the salvo beats have run - so this is what the point defense answering
/// it is recorded in.
#[cfg(feature = "debug")]
const STRIKE_AFTERMATH_SECS: f32 = 3.5;

/// Web media emitted by the arena's one capture walk.
#[cfg(feature = "debug")]
const HERO_LOOP: &str = "hero-wfc-duel";
#[cfg(feature = "debug")]
const LANDING_2V2_LOOP: &str = "landing-wfc-2v2";
#[cfg(feature = "debug")]
const HERO_THUMBNAIL: &str = "thumb-news-0.11.0.png";

/// The driven walk: load the arena, hold until the scoreboard proves both teams
/// fired and both dealt damage, then stage and record the strike.
///
/// The fight is the AI's from the first frame to the last; the walk observes it
/// until the scoreboard says it is real, and only then cues the beats it wants
/// on film (see [`Strike`]). Every recording beat carries a TIME CAP beside its
/// event, so a salvo point defense eats whole, or a lance that loses its charge
/// to a hit, costs the loop a beat rather than failing the capture run.
#[cfg(feature = "debug")]
fn arena_script(
    loop_name: &'static str,
    capture_thumbnail: bool,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the arena")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add();

    // The two walks open on different evidence, and a capture CANNOT use the
    // scoreboard's. This duel is decided at the first merge: the lines cross
    // inside 500 m, maul each other in a couple of seconds, and one hull comes
    // out of the pass without its controller - and `fight_happened` (both teams
    // fired AND both dealt damage) is not true until that merge is already
    // under way. A capture that waited for it staged its strike onto a wreck
    // that no longer fires. So the recording walk advances on the APPROACH, at
    // the first moment the two sides are inside the staging band, and stages
    // there with both hulls whole. The smoke and probe walks keep the scoreboard:
    // proving the fight is the only thing they are for.
    script = if capturing() {
        script
            .step("wait for the merge")
            .until(fight_within(STRIKE_CLOSE_BAND))
            .deadline(FIGHT_DEADLINE_SECS)
            .add()
    } else {
        script
            .step("both teams fire and connect")
            .until(resource_where::<Scoreboard>(Scoreboard::fight_happened))
            .deadline(FIGHT_DEADLINE_SECS)
            .add()
    };

    if capture_thumbnail {
        script = script
            .step("shoot the fight")
            .on_enter(|world: &mut World| shoot(world, HERO_THUMBNAIL))
            .until(shot_written(HERO_THUMBNAIL))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
    }

    // Keep the ordinary smoke/probe walk short. NOVA_CAPTURE selects this tail
    // at construction time, but it remains part of the SAME AutopilotPlugin.
    if !capturing() {
        return script;
    }

    script
        // Insurance, and normally a single frame: the walk arrives here already
        // inside the band, so the orders are installed and the beat passes on
        // the next test. It earns its place on the run where the approach was
        // reached on a deadline instead - a pair that never merged is flown
        // back together rather than photographed across three kilometres.
        .step("close the range")
        .on_enter(close_the_range)
        .until(or(
            fight_staged(STRIKE_STAGE_RANGE, STRIKE_STAGE_TOLERANCE),
            elapsed(STRIKE_CLOSE_SECS),
        ))
        .deadline(STRIKE_CLOSE_SECS * 2.0)
        .add()
        .step("stage the strike")
        .on_enter(stage_the_strike)
        .until(or(bores_settled(), elapsed(STRIKE_ALIGN_SECS)))
        .add()
        .step("open the arena loop")
        .on_enter(move |world: &mut World| loop_start(world, loop_name))
        .add()
        .step("the lance fires")
        .on_enter(cue_the_lances)
        .until(or(
            resource_where::<Strike>(Strike::shot),
            elapsed(STRIKE_CHARGE_SECS),
        ))
        .add()
        .step("both sides open up")
        .on_enter(cue_the_tubes)
        .until(or(
            resource_where::<Strike>(Strike::hit),
            elapsed(STRIKE_SALVO_GAP_SECS),
        ))
        .add()
        .step("the second salvo")
        .on_enter(cue_the_tubes)
        .until(or(
            resource_where::<Strike>(Strike::hit),
            elapsed(STRIKE_WINDOW_SECS),
        ))
        .add()
        .step("hold the aftermath")
        .until(elapsed(STRIKE_AFTERMATH_SECS))
        .add()
        .step("close the arena loop")
        .on_enter(move |world: &mut World| loop_end(world, loop_name))
        .until(loop_written(loop_name))
        .deadline(60.0)
        .add()
}

#[cfg(test)]
mod binding_tests {
    use super::*;

    /// The shipped catalog and the ARENA's own grammar read against each
    /// other, out of the builders rather than off disk. The arena's, not the
    /// shipped one, because the lance these tests bind is a role the arena
    /// seats and the base warship does not.
    fn catalog_tiles() -> (GameSections, TileSet) {
        let sections = GameSections(nova_authoring::generation::build_section_catalog());
        let grammars = GameGrammars(nova_authoring::generation::build_grammars());
        let tiles = arena_tiles(&sections, &grammars);
        (sections, tiles)
    }

    /// A bound weapon that the flight rig ALSO answers double-drives the ship:
    /// both rigs run with `consume_input: false`, so one press would fire the
    /// gun and fly the hull. This is the guard the doc on [`player_bindings`]
    /// promises, over every gun the arena hands a player.
    #[test]
    fn no_arena_weapon_binding_lands_on_a_key_the_flight_rig_spends() {
        let (sections, tiles) = catalog_tiles();
        let reserved: Vec<InputSource> = flight_rig_reserved_sources()
            .into_iter()
            .map(|(source, _)| source)
            .collect();

        for seed in 0..6u64 {
            let mut hull = tiles.hull(seed, true, None).expect("the seed collapses");
            stamps::stamp_large_drives(&mut hull, seed, &sections, tiles.grid());
            for (id, sources) in player_bindings(&hull, 0, &BTreeMap::new()) {
                for source in sources {
                    assert!(
                        !reserved.contains(&source),
                        "seed {seed}: '{id}' is bound to a source the flight rig \
                         already answers: {source:?}"
                    );
                }
            }
        }
    }

    /// The bow gun is useless to a player it is not bound for, and it does NOT
    /// join the turrets on the mouse: one shell on a long reload cannot share
    /// a trigger with guns a pilot holds down.
    #[test]
    fn the_bow_lance_gets_its_own_key_and_not_the_turrets_button() {
        let (_, tiles) = catalog_tiles();
        let hull = tiles.hull(0, false, None).expect("the seed collapses");
        let lance = hull
            .sections
            .iter()
            .find(|section| {
                matches!(&section.source, SectionSource::Prototype(id) if id == SPINAL_LANCE)
            })
            .expect("every arena hull is seeded with one")
            .id
            .clone();

        let bindings = player_bindings(&hull, 0, &BTreeMap::new());
        assert_eq!(
            bindings.get(&lance),
            Some(&vec![InputSource::from(KeyCode::KeyR)]),
            "the lance answers R"
        );
        assert!(
            !bindings[&lance].contains(&InputSource::from(MouseButton::Left)),
            "and never the turrets' held trigger"
        );
    }
}
