//! wfc_arena: a roster of wave-function-collapse ships fights over a dressed
//! arena.
//!
//! The generator is `shared/wfc.rs`, the same collapse `wfc_ships` poses in a
//! row - real prototype sections, turrets on the skin, drives aft, bays with
//! cleared exit lanes, every hull passed through the game's own content lint
//! before it spawns. Where the row NEUTERS its subjects
//! (`SpaceshipController::None`, no allegiance), the arena flips exactly those
//! two fields: some hulls fly the player's colors, the rest the enemy's, all
//! under the same AI pilot the campaign's raiders use, and the combat systems
//! do the rest.
//!
//! The combatants are DRAFTED, not taken blind: the collapse arms hulls with
//! wild variance (a roll can carry ten turrets and eight bays, or none of
//! either), so the arena walks the seed stream from `--seed` and fields the
//! first hulls that clear a small armament floor ([`MIN_TURRETS`],
//! [`MIN_BAYS`]). Deterministic - one number and one roster still reproduce
//! the matchup - and every skipped seed is logged with the armament that
//! disqualified it.
//!
//! That flip is the example's value. Nobody had FLOWN a collapsed hull before
//! this: thrust against the mass of a solid-built hull, turret arcs on a random
//! silhouette, torpedo lanes that were only ever checked geometrically - this
//! is the flyability bench for wfc ships, not just a spectacle. Every ship is
//! clad (`skin: true`), so it doubles as the skin's combat-motion showcase.
//!
//! ALL FOUR WEAPON FLAVOURS are on the field, because a fight with one gun and
//! one torpedo in it is not a fight about weapons. The collapse draws both PDC
//! mounts (the kinetic punch and the pierce rake are one housing on one socket,
//! so the grid cannot tell them apart and does not have to), and the arena
//! LOADS half of every hull's tubes with Lances - see [`load_lances`]. What
//! that stages is the owner's decoy doctrine: Serpents weave in and drain the
//! defender's point defense, and the Lance that follows arrives on a straight
//! line into guns with nothing left to spend.
//!
//! The ROSTER is authored on the command line. `--ship` is repeatable and
//! carries one hull each:
//!
//! ```text
//! --ship TEAM[:STYLE[:SEED]]
//! ```
//!
//! - `TEAM` is a callsign, `amber` or `onyx` ([`TEAMS`]), and is the only
//!   field a ship must carry.
//! - `STYLE` is a style id out of the merged content, for a hull that wears
//!   its own look. Empty means the run's look - what `--style` sets and `L`
//!   cycles - so `amber::7` is a default-look ship on a pinned seed.
//! - `SEED` pins that hull's collapse instead of drafting one off the stream.
//!   A pin is an instruction: the hull spawns as rolled even under the
//!   armament floor (the log says so), and `R` re-rolls the rest around it.
//!
//! With no `--ship` at all the roster is one hull per team, drafted off the
//! stream - the two-ship duel this example started as.
//!
//! Hand-run (`R` re-rolls the roster on fresh seeds, `L` cycles the look, WASD
//! and the mouse take the camera free):
//! ```text
//! cargo run --example wfc_arena --features debug
//! cargo run --example wfc_arena --features debug -- --seed 7 --style salvage
//! cargo run --example wfc_arena --features debug -- \
//!     --ship amber --ship amber:salvage --ship onyx --ship onyx --ship onyx
//! cargo run --example wfc_arena --features debug -- --ship amber::7 --ship onyx
//! ```
//!
//! The fight is MEASURED, not presumed, and measured PER TEAM rather than per
//! hull: a scoreboard counts every round and torpedo each team fires, BY
//! FLAVOUR (`ProjectileOwner` resolves the shooter, which wears its team's
//! allegiance; the projectile names its own kind), which is what turns "all
//! four flavours were in this fight" into a reading of the run rather than a
//! claim about the draft. It also counts every point of section health the
//! other team takes (per-team pool
//! deltas, which sees plate damage the isolated-cladding rule keeps off the
//! roots). The readout wears the score and the log restates it every few
//! seconds, so a frame or a log line is evidence the fight happened rather than
//! a picture of some hulls parked near each other.
//!
//! The arena reads as a place, not a void: the standard three-point rig, a
//! ring of rocks below the fight plane for depth parallax (the editor
//! sandbox's dressing idiom), a sparser outer ring, and one distant pinned
//! planetoid as a landmark.
//!
//! The camera keys pose the view, and every pose is computed off the LIVE
//! fight each frame - midpoint, spread and per-ship transforms - so a vantage
//! keeps its subject framed while the ships move:
//!
//! - `Q` the auto-framing whole-fight view (the default and the capture frame),
//! - `E` the tactical overview, high and wide enough to hold the engagement,
//! - `1`/`2`/`3`/`4` follow one roster slot each, over the shoulder and along
//!   the hull's own heading. A slot the roster never filled, or one whose ship
//!   is dead, falls back to the frame pose rather than freezing.
//!
//! `Q` and `E` sit clear of the free-fly rig, which binds WASD, the mouse and
//! Space/Shift and nothing else, so a mode key never doubles as camera input.
//!
//! The idle orbit belongs to `Q` and only `Q`: left alone there for six
//! seconds the camera falls into a slow orbit around the fight's midpoint -
//! `wfc_ships`' turntable bent around a moving pivot - and a pose key or
//! free-fly input stops it and restarts the clock. Every other vantage, the
//! free camera included, parks and STAYS parked; press `Q` to get the
//! turntable back. `E` is the one that never turns at all: it holds its
//! bearing over the fight and only re-centres, so it reads as a tactical plot
//! rather than a sweep. Grave/tilde cycles the game HUD and the scoreboard
//! readout follows it in a hand-run.
//!
//! Harnessed (`NOVA_AUTOPILOT=1`, plus `NOVA_CAPTURE=1` to stage the shot):
//! wait for the arena, hold until BOTH teams have fired and BOTH have dealt
//! damage - the step deadline makes a fight that never happens a loud failure,
//! not a quiet pose - then shoot the brawl mid-swing.

use bevy::prelude::*;
use clap::Parser;
// Direct, not through `nova_protocol::nova_debug`: that path only exists under
// the `debug` feature, and `capturing()` gates the idle orbit and the readout
// in EVERY build.
use nova_debug::prelude::capturing;
use nova_protocol::prelude::*;

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
    /// One hull of the roster, repeatable: `TEAM[:STYLE[:SEED]]`, where TEAM is
    /// `amber` or `onyx`, STYLE is a style id (empty = the run's look) and SEED
    /// pins the collapse. No `--ship` at all fields one hull per team.
    #[arg(long = "ship", value_name = "TEAM[:STYLE[:SEED]]", value_parser = parse_ship)]
    ships: Vec<ShipSpec>,
    /// Start on this style id instead of the first the content offers. `L`
    /// cycles from wherever this leaves the roster.
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
    /// Callsign for the readout, the log and the `--ship` argument, since
    /// "player" would be a lie about who is driving.
    callsign: &'static str,
    allegiance: Allegiance,
    /// Facing at spawn: toward the other line, so the opening move is a merge
    /// rather than a search turn.
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
/// ~160 u apart, INSIDE the PDC fire gate (180 u = reach x 0.9): the guns bear
/// from the opening frame, so both teams trade fire before the torpedo salvos
/// land. The first cut spawned at 280 u and the fight was over before either
/// ship closed to gun range - one 8-tube alpha strike, 11 seconds, no reply.
const LINE_STANDOFF: f32 = 75.0;
const LINE_LIFT: f32 = 12.0;
const LINE_OFFSET: f32 = 30.0;
/// Centre-to-centre spacing along a line. Three times the widest hull the grid
/// can grow, so a line is a formation rather than a pile-up, and short enough
/// that the far end of one line still opens at gun range on the far end of the
/// other.
const LINE_SPACING: f32 = 34.0;

/// Combat breaks off past this distance from the patrol centroid. Wide enough
/// for real chases, tight enough that the fight stays over the rock ring.
const LEASH: f32 = 280.0;

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
/// The NAME the Lance type wears in flight (`TorpedoType`), which is how the
/// scoreboard tells the two ordnances apart once they have left the tube. The
/// one content string this file has to know: a projectile carries its type,
/// not the id of the bay that launched it.
const LANCE_TYPE: &str = "Lance";
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
    let roster = Roster {
        seed: cli.seed,
        ships,
        drafted: Vec::new(),
        style: 0,
    };
    let requested = cli.style.clone();
    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| {
            arena_plugin(app, roster.clone(), StyleRequest(requested.clone()))
        })
        .build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (inert without its NOVA_PERF_* env): run timeline +
        // engine-bound invariants. No frame-time capture - a brawl's load
        // varies with the roll, so there is no steady state to grade.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        // Clean frames at the fleet's 16:9, dev overlays out of shot. The HUD
        // drops to cinematic only under capture: a hand-run keeps the level On
        // so grave/tilde round-trips the readout with the rest of the HUD.
        // The scoreboard readout is the example's own UI and STAYS in every
        // capture - it is what makes a frame evidence.
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        if capturing() {
            app.add_systems(Startup, hide_hud);
        }
        // NO freeze_bodies here, unlike wfc_ships: the whole point is that
        // these bodies fly.
        app.add_plugins(arena_script());
    }

    app.run()
}

fn arena_plugin(app: &mut App, roster: Roster, requested: StyleRequest) {
    app.insert_resource(roster);
    app.insert_resource(requested);
    app.init_resource::<Scoreboard>();
    // The frame vantage until a pose key or the free-fly rig says otherwise -
    // and under capture too, because framing the fight IS the capture framing.
    app.insert_resource(Vantage::Frame);
    // Enabled only for a hand-run: a capture composes its own frame, and an
    // orbit under it would photograph a different bearing every run.
    app.insert_resource(IdleOrbit::new(!capturing()));
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_arena);
    app.add_systems(
        Update,
        (
            // The tracker runs BEFORE the reroll so a reload can never leave
            // a freshly-written pool behind: the reroll resets the score in
            // the same frame it triggers the load, the ships only despawn at
            // the flush, and a tracker running after the reset would read
            // them alive and re-arm the pools - which the teardown would then
            // cash in as a phantom mutual kill one frame later.
            (
                track_damage,
                reroll_on_key.run_if(in_state(GameStates::Playing)),
            )
                .chain(),
            count_shots,
            report_score.run_if(in_state(GameStates::Playing)),
            update_readout,
            select_vantage,
            free_camera_on_input,
            track_orbit_idle,
        ),
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
            .after(WASDCameraSystems::Sync)
            .before(TransformSystems::Propagate),
    );
}

// ---------------------------------------------------------------------------
// The roster.
// ---------------------------------------------------------------------------

/// One hull the roster asks for: which team it fights for, the look it insists
/// on (if any), and the seed it insists on (if any). The `--ship` value.
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
}

/// One `--ship` field, trimmed, where an empty field reads as absent - so
/// `amber::7` pins a seed without naming a style.
fn field(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|field| !field.is_empty())
}

/// Parse one `--ship` value: `TEAM[:STYLE[:SEED]]`, colon separated, an empty
/// field meaning "the default for that field".
///
/// Colons rather than `key=value` pairs because the common case is a bare team
/// name and the whole grammar is three fields deep; the flag is repeated per
/// hull, so it is typed more often than it is read.
fn parse_ship(value: &str) -> Result<ShipSpec, String> {
    let mut fields = value.split(':');
    let name = fields.next().unwrap_or_default().trim();
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
    let style = field(fields.next()).map(str::to_string);
    let seed = match field(fields.next()) {
        Some(seed) => Some(
            seed.parse::<u64>()
                .map_err(|error| format!("'{seed}' is not a seed: {error}"))?,
        ),
        None => None,
    };
    if fields.next().is_some() {
        return Err(format!(
            "'{value}' has more fields than TEAM[:STYLE[:SEED]]"
        ));
    }
    Ok(ShipSpec { team, style, seed })
}

/// The roster a run with no `--ship` fields: one drafted hull per team, on the
/// run's look. The duel this example started as.
fn default_roster() -> Vec<ShipSpec> {
    (0..TEAMS.len())
        .map(|team| ShipSpec {
            team,
            style: None,
            seed: None,
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
}

impl Roster {
    /// How many hulls a team fields.
    fn strength(&self, team: usize) -> usize {
        self.ships.iter().filter(|ship| ship.team == team).count()
    }
}

/// The style id `--style` asked for, resolved to an index on the first load.
#[derive(Resource)]
struct StyleRequest(Option<String>);

fn load_arena(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    requested: Res<StyleRequest>,
    mut roster: ResMut<Roster>,
) {
    if let Some(id) = requested.0.as_deref() {
        match styles.iter().position(|style| style.id == id) {
            Some(index) => roster.style = index,
            // Loud, not silent: a typo would otherwise dress the fight in the
            // first look and read as the one that was asked for.
            None => panic!("--style '{id}' is not in the merged content"),
        }
    }
    commands.trigger(LoadScenario(arena(
        &game_assets,
        &sections,
        &styles,
        &mut roster,
    )));
    spawn_readout(&mut commands);
}

/// `R` re-rolls the roster from the seed stream past the last draft, `L` steps
/// every ship to the next authored look. Either way the same roster comes back
/// - same teams, same styles, same pinned seeds - and the scenario reloads
/// through `LoadScenario` (which tears the old fight down, in-flight ordnance
/// included) with the scoreboard starting over, since the score of a fight that
/// no longer exists is not evidence of anything.
fn reroll_on_key(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    mut roster: ResMut<Roster>,
    mut score: ResMut<Scoreboard>,
) {
    if keyboard.just_pressed(KeyCode::KeyR) {
        // Past the whole draft, so a re-roll never re-fields a hull the last
        // one already stood up. A pinned ship keeps its seed either way.
        roster.seed = roster
            .drafted
            .iter()
            .copied()
            .max()
            .unwrap_or(roster.seed)
            .wrapping_add(1);
    } else if keyboard.just_pressed(KeyCode::KeyL) {
        roster.style = roster.style.wrapping_add(1);
    } else {
        return;
    }
    *score = Scoreboard::default();
    commands.trigger(LoadScenario(arena(
        &game_assets,
        &sections,
        &styles,
        &mut roster,
    )));
}

// ---------------------------------------------------------------------------
// The draft and the loadout.
// ---------------------------------------------------------------------------

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

/// One combatant: a drafted hull, clad, under the same AI pilot the campaign's
/// raiders fly, on its team's colors and in its team's line.
fn combatant(
    slot: usize,
    seed: u64,
    hull: ShipHull,
    ship: &ShipSpec,
    place: (usize, usize),
) -> ScenarioObjectConfig {
    let team = &TEAMS[ship.team];
    // The armament roll, per ship: the draft floor bounds it from below but one
    // team can still out-gun the other, and a lopsided fight reads differently
    // knowing that. This line is the roll's disclosure, and the only place the
    // loadout the arena chose is stated.
    let arms = armament(&hull);
    info!(
        "wfc_arena: {} {} seed {}: {} sections - {}",
        team.callsign,
        slot,
        seed,
        hull.sections.len(),
        arms,
    );
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: format!("{FIGHTER_ID_PREFIX}{slot}"),
            name: format!("{} {seed}", team.callsign),
            position: spawn_position(ship.team, place.0, place.1),
            rotation: Quat::from_rotation_y(team.yaw),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(team.allegiance),
            controller: SpaceshipController::AI(AIControllerConfig {
                patrol: team.patrol.to_vec(),
                // Anchored on the center-hugging patrol centroid, so the
                // fight gravitates to the dressed middle of the arena.
                leash: Some(LEASH),
                ..Default::default()
            }),
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
            center: Vec3::ZERO,
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
                health: 100.0,
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
            health: 100.0,
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
            ))
        })
        .collect();

    let scenario = ScenarioConfig {
        description: "Wave-function-collapse ships fight in a dressed arena".to_string(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: ships
                .into_iter()
                .chain(ThreePointRig::around("arena", Vec3::ZERO, 8.0).actions())
                .chain([
                    planetoid(game_assets),
                    // Depth parallax under the fight plane, and a sparser far
                    // ring so the void has a middle distance.
                    rock_ring(
                        game_assets,
                        "arena_rock_low_",
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

// ---------------------------------------------------------------------------
// The scoreboard: proof the fight happened.
// ---------------------------------------------------------------------------

/// What a team has actually put in the air, by flavour.
///
/// Counted off the PROJECTILE rather than off the hull's section list, because
/// a bay a ship carries is not a torpedo a ship launched: this is what makes
/// "all four flavours were in this fight" a reading of the run instead of a
/// claim about the draft. A round names its own flavour (`ProjectileDamage`'s
/// damage type) and so does a torpedo (`TorpedoType`), so nothing here has to
/// trace a projectile back to the section that fired it.
#[derive(Clone, Copy, Default)]
struct Salvo {
    kinetic: u32,
    pierce: u32,
    serpents: u32,
    lances: u32,
}

impl Salvo {
    /// Everything fired, which is what "did this team fight" asks.
    fn total(&self) -> u32 {
        self.kinetic + self.pierce + self.serpents + self.lances
    }
}

impl std::fmt::Display for Salvo {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} kinetic + {} pierce rounds, {} serpent + {} lance torpedoes",
            self.kinetic, self.pierce, self.serpents, self.lances,
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
/// live root (loading, rerolling, or wiped out), which is what keeps a
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
        match round.map(|damage| damage.kind) {
            Some(DamageType::Kinetic) => salvo.kinetic += 1,
            Some(DamageType::Pierce) => salvo.pierce += 1,
            _ => {}
        }
        if let Some(torpedo) = torpedo {
            // The ordnance names itself in flight, which is the only place the
            // two bays are distinguishable once the tube is behind them.
            if torpedo.name == LANCE_TYPE {
                salvo.lances += 1;
            } else {
                salvo.serpents += 1;
            }
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
            // the reroll resets the score before the teardown lands (see the
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

// ---------------------------------------------------------------------------
// Framing and readout.
// ---------------------------------------------------------------------------

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

/// How far a follow pose stands behind its ship, how far above it, and how far
/// ahead of the hull it looks: close enough to read as attached, far enough
/// that the widest roll's stern never fills the frame, and led enough that the
/// hull sits low in shot with the space it is flying into above it.
const FOLLOW_BACK: f32 = 34.0;
const FOLLOW_LIFT: f32 = 10.0;
const FOLLOW_LEAD: f32 = 12.0;

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
    /// `1`..`4`: over the shoulder of one roster slot, along its heading.
    Follow(usize),
    /// The viewer took the camera; no pose writes until a camera key re-arms
    /// one.
    Free,
}

/// The camera bindings: the number row FOLLOWS, because the roster is a
/// numbered list and the ships are what a viewer wants one key each for, with
/// the two whole-fight vantages on `Q` and `E` beside them. All six are clear
/// of the free-fly rig (WASD, mouse, Space/Shift) and of `R`/`L`, which
/// already reroll and restyle.
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

/// The live fight as the camera reads it: the midpoint of every standing root,
/// the spread-derived standoff that keeps the whole engagement in frame, and
/// the transform of each followable roster slot whose ship still stands.
/// Wrecks keep their root marker, so a pose holds the aftermath too instead of
/// snapping away on the kill.
struct FightRead {
    midpoint: Vec3,
    standoff: f32,
    followed: [Option<Transform>; FOLLOW_SLOTS],
}

/// The camera-side ship query: transforms plus the scenario id that names a
/// root's roster slot, shared by the poses and the idle orbit.
type ShipQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Transform, Option<&'static EntityId>),
    (With<SpaceshipRootMarker>, Without<ScenarioCameraMarker>),
>;

/// The roster slot a scenario id names, when it is one of the arena's own
/// fighters and one the follow row can reach.
fn follow_slot(id: &EntityId) -> Option<usize> {
    let slot: usize = id.0.strip_prefix(FIGHTER_ID_PREFIX)?.parse().ok()?;
    (slot < FOLLOW_SLOTS).then_some(slot)
}

fn read_fight(q_ships: &ShipQuery) -> Option<FightRead> {
    let mut positions = Vec::new();
    let mut followed = [None; FOLLOW_SLOTS];
    for (transform, id) in q_ships {
        positions.push(transform.translation);
        if let Some(slot) = id.and_then(follow_slot) {
            followed[slot] = Some(*transform);
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

/// Resolve the armed vantage against the live fight. A pose that needs a ship
/// the fight no longer has - a follow slot the roster never filled, or one
/// whose hull is dead - falls back to the frame pose rather than freezing.
fn vantage_pose(vantage: Vantage, fight: &FightRead) -> Option<Pose> {
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
            let Some(ship) = fight.followed.get(slot).copied().flatten() else {
                return Some(frame_pose(fight));
            };
            // The ship's OWN heading, not the line to a rival: a follow camera
            // is bolted to the hull, so it swings with the hull and shows what
            // the pilot is flying at.
            Some(Pose {
                stand: ship.translation + ship.back() * FOLLOW_BACK + Vec3::Y * FOLLOW_LIFT,
                target: ship.translation + ship.forward() * FOLLOW_LEAD,
                up: Vec3::Y,
            })
        }
    }
}

/// Write the armed pose onto the scenario camera.
fn pose_vantage_camera(
    vantage: Res<Vantage>,
    q_ships: ShipQuery,
    mut q_camera: Query<&mut Transform, With<ScenarioCameraMarker>>,
) {
    let Some(fight) = read_fight(&q_ships) else {
        return;
    };
    let Some(pose) = vantage_pose(*vantage, &fight) else {
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

/// Marks the scoreboard readout.
#[derive(Component)]
struct ScoreReadout;

fn spawn_readout(commands: &mut Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                ScoreReadout,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Written every frame for `wfc_ships`' reason: the readout spawns in the
/// same command flush as the first resource change, so a change-gated write
/// would land before the text exists and never run again.
///
/// The line carries the STREAM HEAD rather than every drafted seed: that one
/// number plus the `--ship` flags reproduces the whole matchup, and a roster
/// of five would otherwise spend the frame's width on seeds. The per-hull
/// seeds and loadouts go to the log at the draft.
///
/// The readout follows the grave/tilde HUD cycle in a hand-run, so "no hud"
/// clears the whole top of the frame. Captures are exempt: they run at
/// cinematic from startup, and the readout is the frame's evidence.
fn update_readout(
    roster: Res<Roster>,
    styles: Res<GameStyles>,
    score: Res<Scoreboard>,
    hud: Res<HudVisibility>,
    mut q_readout: Query<(&mut Text, &mut Visibility), With<ScoreReadout>>,
) {
    let line = format!(
        "WFC arena - {} x{} vs {} x{} - seed {} - {} - fired {}/{} - dealt {:.0}/{:.0} - \
         [R] re-roll  [L] look  [Q] frame  [E] overview  [1-4] follow",
        TEAMS[0].callsign,
        roster.strength(0),
        TEAMS[1].callsign,
        roster.strength(1),
        roster.seed,
        style_at(&styles, roster.style).unwrap_or("bare"),
        score.fired[0].total(),
        score.fired[1].total(),
        score.dealt[0],
        score.dealt[1],
    );
    let shown = capturing() || hud.shows();
    for (mut text, mut visibility) in &mut q_readout {
        visibility.set_if_neq(if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if text.as_str() != line {
            **text = line.clone();
        }
    }
}

/// Seconds a load step may sit before the run aborts naming it (llvmpipe
/// headroom).
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 30.0;

/// Seconds the fight gets to prove itself: both teams firing and both dealt
/// damage. Sized UNDER the harness completion watchdog's 120 s default, so a
/// fight that never happens fails naming THIS step - the honest failure mode
/// - instead of the watchdog's anonymous laggard exit. A slow matchup can be
/// given more room with `NOVA_AUTOPILOT_DEADLINE`.
#[cfg(feature = "debug")]
const FIGHT_DEADLINE_SECS: f32 = 90.0;

/// The driven walk: load the arena, hold until the scoreboard proves both
/// teams fired and both dealt damage, then shoot the brawl. The auto-frame
/// camera is the capture framing, so no step poses one.
#[cfg(feature = "debug")]
fn arena_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
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
        .add()
        .step("shoot the fight")
        .on_enter(|world: &mut World| shoot(world, "wfc-arena-fight.png"))
        .until(shot_written("wfc-arena-fight.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
