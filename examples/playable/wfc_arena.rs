//! wfc_arena: a roster of wave-function-collapse ships fights over a dressed
//! arena.
//!
//! The generator is `shared/wfc.rs`, the same collapse `wfc_ships` poses in a
//! row. Where the row neuters its subjects (`SpaceshipController::None`, no
//! allegiance), the arena flips exactly those two fields: some hulls fly the
//! player's colors, the rest the enemy's, all under the campaign's AI pilot.
//! This is the flyability bench for wfc ships - thrust against a collapsed
//! hull's mass, turret arcs on a random silhouette, torpedo lanes that were
//! only ever checked geometrically.
//!
//! Combatants are DRAFTED: the collapse arms hulls with wild variance, so the
//! arena walks the seed stream from `--seed` and fields the first hulls that
//! clear an armament floor ([`MIN_TURRETS`], [`MIN_BAYS`]). Deterministic - one
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
//! a run that asks for both fails loudly.
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
//! The lines spawn ~305 u apart and fly in COLD: an arrival grace holds both
//! teams on their center-crossing patrols, and the weapons-free gate
//! ([`ENGAGE_RANGE`]) keeps them passive until the closing lines are inside it.
//! From 280 u hot, one torpedo alpha strike decided the fight before a gun
//! bore.
//!
//! Scoring is PER TEAM. Rows come from projectile-carried `DamageType` variants
//! and authored torpedo names, so a new ammunition type needs no arena edit;
//! damage is counted through per-team pool deltas, which see the plate damage
//! the isolated-cladding rule keeps off the roots. Losing every live flight
//! computer freezes the fight and opens the result board. A 180-second global
//! inactivity window resolves a deadlock by remaining structure. Ships outside
//! the 20 km sphere are warned for 30 seconds, then lose their flight
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
//! - `Q` auto-framing whole-fight view (default, and the capture frame). Left
//!   alone for six seconds it falls into a slow orbit around the midpoint; a
//!   pose key or free-fly input stops it and restarts the clock.
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
// Only for the derelicts' tumble: everything the fight depends on stays on the
// generator's own seed stream.
use rand::{rngs::StdRng, RngExt, SeedableRng};

#[path = "wfc_arena/lobby.rs"]
mod lobby;
#[path = "wfc_arena/pause.rs"]
mod pause;
#[path = "wfc_arena/result.rs"]
mod result;
#[path = "shared/wfc.rs"]
mod wfc;
use wfc::{refuse_broken_ships, style_at, tile_set, wfc_hull, StyleId};

#[derive(Parser)]
#[command(name = "wfc_arena")]
#[command(version = "1.0.0")]
#[command(about = "A roster of wave-function-collapse ships fights in a dressed arena", long_about = None)]
struct Cli {
    /// Where the draft starts reading the seed stream; each drafted hull takes
    /// the next viable seed past the last one.
    #[arg(long, default_value_t = DEFAULT_SEED)]
    seed: u64,
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

/// The default first seed. Not `wfc_ships`' default on purpose: the two
/// examples should not photograph the same hulls.
const DEFAULT_SEED: u64 = 20_260_816;

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
/// ~305 u apart - well past the 180 u PDC fire gate (reach x 0.9) - which
/// only works because the approach is COLD (see [`ENGAGE_GRACE_SECS`] and
/// [`ENGAGE_RANGE`]): hot from 280 u, one 8-tube alpha strike ended the fight
/// in 11 seconds with no reply, which is why the last cut spawned at 163 u.
/// Cold, the long spawn buys a real approach and the fight still opens near
/// gun range. Not further: the passive closing rate is ~2.5-4 u/s (measured),
/// so at 345 u the quiet leg ran 45 s and read as dead air, not tension.
const LINE_STANDOFF: f32 = 150.0;
const LINE_LIFT: f32 = 12.0;
const LINE_OFFSET: f32 = 30.0;
/// Seconds both teams hold their passive patrols after spawn, weapons cold
/// (`AIControllerConfig::engage_delay`). The patrols cross the center, so the
/// grace reads as two formations flying in, not two formations parked.
const ENGAGE_GRACE_SECS: f32 = 10.0;
/// The weapons-free gate (u, `AIControllerConfig::engage_range`): even past
/// the grace a line stays passive until a hostile closes inside this. This
/// gate, not the grace, is what actually times the first shot - the passive
/// closing is slow, so the lines cross it long after the grace expires.
///
/// AT the 180 u gun gate (reach x 0.9), so the fight opens with guns and
/// torpedoes TOGETHER. Wider gates all lost fights to the torpedo alpha:
/// at 240 the gate opened one-sidedly (AMBER salvoed and wiped ONYX before
/// ONYX ever fired), and at 220 the bout was a coin flip - the bigger
/// battery intercepts the smaller salvo OUTRIGHT (6 torpedoes into 16
/// turrets land nothing), so unless the loser's guns connect in the few
/// seconds both sides are alive, it dies having dealt zero and the walk's
/// both-sides-dealt predicate fails. Guns from the first second of the
/// engagement are what keep the fight mutual.
const ENGAGE_RANGE: f32 = 180.0;
/// Centre-to-centre spacing along a line. Three times the widest hull the grid
/// can grow, so a line is a formation rather than a pile-up, and short enough
/// that the far end of one line still opens at gun range on the far end of the
/// other.
const LINE_SPACING: f32 = 34.0;

/// Combat breaks off past this distance from the patrol centroid. Wide enough
/// for real chases, tight enough that the fight stays over the rock ring.
const LEASH: f32 = 280.0;

/// The junk blobs: where each debris cluster anchors, and how many wreckage
/// FRAGMENTS it scatters. All three sit on the Z FLANKS (|z| >= 160), because
/// the fight runs along X: the spawn lines stand at x = +/-[`LINE_STANDOFF`]
/// with |z| <= ~80, and the approach corridor between them is the axis every
/// torpedo flies down - junk there would eat ordnance and decide fights.
/// Two blobs sit at negative z, which is the BACKGROUND of the capture frame
/// (the frame camera stands on +Z); the third is on the south flank for the
/// idle orbit and the follow cameras to sweep past.
const DERELICT_BLOBS: [(Vec3, usize); 3] = [
    (Vec3::new(40.0, -34.0, -190.0), 8),
    (Vec3::new(-150.0, 30.0, -160.0), 7),
    (Vec3::new(90.0, -25.0, 200.0), 5),
];
/// How many sections one fragment carries: a broken-off chunk of structure,
/// never anything that could read as an intact vessel. The first cut spawned
/// five FULL derelict hulls (912 sections) and the owner called it - junk is
/// "multiple small things", so the blob budget went from hulls to fragments
/// and the section total dropped by an order of magnitude.
const FRAGMENT_MIN_SECTIONS: usize = 2;
const FRAGMENT_MAX_SECTIONS: usize = 8;
/// The scatter shell fragments land in around their blob anchor, and the
/// closest two fragments may stand: ~3 u chunks 12+ u apart read as a drifted
/// debris field, not a pile.
const FRAGMENT_SHELL: (f32, f32) = (6.0, 40.0);
const FRAGMENT_SEPARATION: f32 = 12.0;
/// Salt on the roster's stream head for the fragment rolls: the junk follows
/// the lobby's resolved seed head and never duplicates a combatant's seed (the
/// draft scans at most [`DRAFT_SCAN_CAP`] past the head).
const DERELICT_SEED_SALT: u64 = 0xDEAD;
/// The scenario id prefix a fragment spawns under - distinct from
/// [`FIGHTER_ID_PREFIX`], so the follow cameras can never latch onto junk.
const DERELICT_ID_PREFIX: &str = "arena_derelict_";

/// The distant landmark, well outside the leash + its own 400 u sphere of
/// influence (`mu = soi_cutoff_accel * soi^2` at the shipped 0.25 cutoff), so
/// it is scenery and never a well the fight falls into.
const PLANETOID_POSITION: Vec3 = Vec3::new(-620.0, -140.0, -420.0);
const PLANETOID_RADIUS: f32 = 24.0;
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
        seed: cli.seed,
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
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
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

/// Collapse one hull for a roster slot and load its tubes.
fn combat_hull(tiles: &[wfc::Tile], seed: u64, style: StyleId) -> ShipHull {
    let mut hull = wfc_hull(tiles, seed, true, style);
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
    tiles: &[wfc::Tile],
    ships: &[ShipSpec],
    looks: &[StyleId],
    from: u64,
) -> Vec<(u64, ShipHull)> {
    let mut cursor = from;
    let mut drafted = Vec::new();
    for (slot, ship) in ships.iter().enumerate() {
        let style = looks[slot];
        if let Some(seed) = ship.seed {
            let hull = combat_hull(tiles, seed, style);
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
            let hull = combat_hull(tiles, seed, style);
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
    drafted
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
/// gamepad right trigger beside it) and every tube on `F` - the mouse's right
/// button is the raise-weapons gesture and the reserved flight-rig sources
/// (`flight_rig_reserved_sources`) are all keys the rig already spends.
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
            position,
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
                    infinite_ammo: false,
                })
            } else {
                SpaceshipController::AI(AIControllerConfig {
                    patrol: team.patrol.to_vec(),
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
    center: Vec3,
    seed: u64,
    count: u32,
    inner: f32,
    outer: f32,
    y: (f32, f32),
    radius: (f32, f32),
    separation: f32,
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
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                // DIRECT paths, not dep://: this scenario is built at runtime
                // outside the mod merge, so scheme refs would never rewrite.
                impact_sound: Some(AssetRef::from("base/sounds/impact.wav")),
                destroy_sound: Some(AssetRef::from("base/sounds/explosion.wav")),
                radius: radius.0,
                texture: AssetRef::from(game_assets.asteroid_texture.clone()),
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some(radius),
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
        let mut placed: Vec<Vec3> = Vec::new();
        let mut rng = StdRng::seed_from_u64(
            (roster.seed ^ DERELICT_SEED_SALT).wrapping_add((blob as u64) << 8),
        );
        for _ in 0..*count {
            // Rejection-sampled scatter: fragments are ~3 u across, so a
            // handful of retries always finds standing room in the shell.
            let mut offset = Vec3::ZERO;
            for _ in 0..32 {
                let radius = rng.random_range(FRAGMENT_SHELL.0..FRAGMENT_SHELL.1);
                let yaw = rng.random_range(0.0..std::f32::consts::TAU);
                let lift = rng.random_range(-0.4..0.4f32);
                offset = Vec3::new(yaw.cos() * radius, lift * radius, yaw.sin() * radius);
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
            16.0,
            48.0,
            (-14.0, 14.0),
            (0.8, 2.4),
            10.0,
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
            impact_sound: Some(AssetRef::from("base/sounds/impact.wav")),
            destroy_sound: Some(AssetRef::from("base/sounds/explosion.wav")),
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
    styles: &GameStyles,
    roster: &mut Roster,
) -> ScenarioConfig {
    let tiles = tile_set(sections);
    let run_style = style_at(styles, roster.style);
    let looks: Vec<StyleId> = roster
        .ships
        .iter()
        .map(|ship| ship_style(styles, ship, run_style))
        .collect();
    let drafted = draft_roster(&tiles, &roster.ships, &looks, roster.seed);
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
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: ships
                .into_iter()
                .chain(ThreePointRig::around("arena", Vec3::ZERO, 8.0).actions())
                .chain(derelicts(game_assets, styles, roster))
                .chain([
                    planetoid(game_assets),
                    // Depth parallax under the fight plane, and a sparser far
                    // ring so the void has a middle distance.
                    rock_ring(
                        game_assets,
                        "arena_rock_low_",
                        Vec3::ZERO,
                        roster.seed ^ 0x0A11,
                        14,
                        160.0,
                        240.0,
                        (-70.0, -35.0),
                        (1.5, 4.0),
                        50.0,
                    ),
                    rock_ring(
                        game_assets,
                        "arena_rock_far_",
                        Vec3::ZERO,
                        roster.seed ^ 0x0FA2,
                        10,
                        320.0,
                        400.0,
                        (-20.0, 80.0),
                        (2.0, 5.0),
                        60.0,
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
const CAMERA_DIRECTION: Vec3 = Vec3::new(0.0, 0.45, 1.0);
const CAMERA_BASE: f32 = 55.0;
const CAMERA_PER_SPREAD: f32 = 0.85;

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

/// Seconds a load step may sit before the run aborts naming it (llvmpipe
/// headroom).
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 30.0;

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

/// Web media emitted by the arena's one capture walk.
#[cfg(feature = "debug")]
const HERO_LOOP: &str = "hero-wfc-duel";
#[cfg(feature = "debug")]
const LANDING_2V2_LOOP: &str = "landing-wfc-2v2";
#[cfg(feature = "debug")]
const HERO_THUMBNAIL: &str = "thumb-news-0.11.0.png";

/// The driven walk: load the arena, hold until the scoreboard proves both
/// teams fired and both dealt damage, then capture the brawl. The AI controllers
/// fly both ships; this one harness driver only observes and records them. The
/// auto-frame camera is already the capture framing, so no step poses one.
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
        .add()
        .step("both teams fire and connect")
        .until(resource_where::<Scoreboard>(Scoreboard::fight_happened))
        .deadline(FIGHT_DEADLINE_SECS)
        .add();

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
        .step("open the arena loop")
        .on_enter(move |world: &mut World| loop_start(world, loop_name))
        .until(frames(1))
        .add()
        .step("record the live duel")
        .until(elapsed(6.0))
        .add()
        .step("close the arena loop")
        .on_enter(move |world: &mut World| loop_end(world, loop_name))
        .until(loop_written(loop_name))
        .deadline(60.0)
        .add()
}
