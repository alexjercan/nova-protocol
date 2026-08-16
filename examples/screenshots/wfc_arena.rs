//! wfc_arena: two wave-function-collapse ships fight over a dressed arena.
//!
//! The generator is `shared/wfc.rs`, the same collapse `wfc_ships` poses in a
//! row - real prototype sections, turrets on the skin, drives aft, bays with
//! cleared exit lanes, every hull passed through the game's own content lint
//! before it spawns. Where the row NEUTERS its subjects
//! (`SpaceshipController::None`, no allegiance), the arena flips exactly those
//! two fields: one hull flies the player's colors, the other the enemy's, both
//! under the same AI pilot the campaign's raiders use, and the combat systems
//! do the rest.
//!
//! The combatants are DRAFTED, not taken blind: the collapse arms hulls with
//! wild variance (a roll can carry ten turrets and eight bays, or none of
//! either), so the arena walks the seed stream from `--seed` and fields the
//! first two hulls that clear a small armament floor ([`MIN_TURRETS`],
//! [`MIN_BAYS`]). Deterministic - one number still reproduces the matchup -
//! and every skipped seed is logged with the armament that disqualified it.
//!
//! That flip is the example's value. Nobody had FLOWN a collapsed hull before
//! this: thrust against the mass of a solid-built hull, turret arcs on a random
//! silhouette, torpedo lanes that were only ever checked geometrically - this
//! is the flyability bench for wfc ships, not just a spectacle. Both ships are
//! clad (`skin: true`), so it doubles as the skin's combat-motion showcase.
//!
//! The fight is MEASURED, not presumed: a scoreboard counts every round and
//! torpedo each side fires (`ProjectileOwner` resolves the shooter) and every
//! point of section health the other side takes (per-side pool deltas, which
//! sees plate damage the isolated-cladding rule keeps off the roots). The
//! readout wears the score and the log restates it every few seconds, so a
//! frame or a log line is evidence the fight happened rather than a picture of
//! two hulls parked near each other.
//!
//! The arena reads as a place, not a void: the standard three-point rig, a
//! ring of rocks below the fight plane for depth parallax (the editor
//! sandbox's dressing idiom), a sparser outer ring, and one distant pinned
//! planetoid as a landmark.
//!
//! Hand-run (`R` re-rolls both hulls on fresh seeds, `L` cycles the look, WASD
//! takes the camera off the auto-frame):
//! ```text
//! cargo run --example wfc_arena --features debug
//! cargo run --example wfc_arena --features debug -- --seed 7 --style salvage
//! ```
//!
//! Harnessed (`NOVA_AUTOPILOT=1`, plus `NOVA_CAPTURE=1` to stage the shot):
//! wait for the arena, hold until BOTH sides have fired and BOTH have dealt
//! damage - the step deadline makes a fight that never happens a loud failure,
//! not a quiet pose - then shoot the brawl mid-swing.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[path = "shared/wfc.rs"]
mod wfc;
use wfc::{refuse_broken_ships, style_at, tile_set, wfc_hull, StyleId};

#[derive(Parser)]
#[command(name = "wfc_arena")]
#[command(version = "1.0.0")]
#[command(about = "Two wave-function-collapse ships fight in a dressed arena", long_about = None)]
struct Cli {
    /// Seed of the first ship; its rival collapses from `seed + 1`.
    #[arg(long, default_value_t = DEFAULT_SEED)]
    seed: u64,
    /// Start on this style id instead of the first the content offers. `L`
    /// cycles from wherever this leaves the pair.
    #[arg(long)]
    style: Option<String>,
}

/// The default first seed. Not `wfc_ships`' default on purpose: the two
/// examples should not photograph the same hulls.
const DEFAULT_SEED: u64 = 20_260_816;

/// The two combatants. Only Player<->Enemy reads as hostile in the relation
/// model, so an AI-vs-AI fight needs one ship flying the player's colors -
/// the same trick every AI-vs-AI backdrop uses.
struct Side {
    /// Callsign for the readout and the log, since "player" would be a lie
    /// about who is driving.
    callsign: &'static str,
    allegiance: Allegiance,
    spawn: Vec3,
    /// Facing at spawn: toward the rival, so the opening move is a merge
    /// rather than a search turn.
    yaw: f32,
    /// Passive patrol ring near the center. Its centroid anchors the leash,
    /// which is what keeps the fight over the dressed ground instead of
    /// drifting off into the void.
    patrol: [Vec3; 3],
}

/// Spawns face each other across the arena with a small vertical and lateral
/// split, so the approach lines cross instead of meeting nose to nose.
/// ~160 u apart, INSIDE the PDC fire gate (180 u = reach x 0.9): the guns
/// bear from the opening frame, so both sides trade fire before the torpedo
/// salvos land. The first cut spawned at 280 u and the fight was over before
/// either ship closed to gun range - one 8-tube alpha strike, 11 seconds, no
/// reply.
const SIDES: [Side; 2] = [
    Side {
        callsign: "AMBER",
        allegiance: Allegiance::Player,
        spawn: Vec3::new(-75.0, 12.0, 30.0),
        yaw: -std::f32::consts::FRAC_PI_2,
        patrol: [
            Vec3::new(-70.0, 10.0, 50.0),
            Vec3::new(70.0, 15.0, -50.0),
            Vec3::new(0.0, 5.0, 70.0),
        ],
    },
    Side {
        callsign: "ONYX",
        allegiance: Allegiance::Enemy,
        spawn: Vec3::new(75.0, -12.0, -30.0),
        yaw: std::f32::consts::FRAC_PI_2,
        patrol: [
            Vec3::new(70.0, -5.0, -50.0),
            Vec3::new(-70.0, -10.0, 50.0),
            Vec3::new(0.0, -15.0, -70.0),
        ],
    },
];

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

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    let matchup = Matchup {
        seed: cli.seed,
        drafted: [cli.seed, cli.seed],
        style: 0,
    };
    let requested = cli.style.clone();
    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| {
            arena_plugin(app, matchup, StyleRequest(requested.clone()))
        })
        .build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (inert without its NOVA_PERF_* env): run timeline +
        // engine-bound invariants. No frame-time capture - a brawl's load
        // varies with the roll, so there is no steady state to grade.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        // Clean frames at the fleet's 16:9; dev overlays and the fps bar out
        // of shot. The scoreboard readout is the example's own UI and STAYS -
        // it is what makes a frame evidence.
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        // NO freeze_bodies here, unlike wfc_ships: the whole point is that
        // these bodies fly.
        app.add_plugins(arena_script());
    }

    app.run()
}

fn arena_plugin(app: &mut App, matchup: Matchup, requested: StyleRequest) {
    app.insert_resource(matchup);
    app.insert_resource(requested);
    app.init_resource::<Scoreboard>();
    // Armed until the free-fly rig is touched, like wfc_ships' idle orbit -
    // and armed under capture too, because framing the fight IS the capture
    // framing.
    app.insert_resource(FollowCamera(true));
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
            stop_follow_on_input,
        ),
    );
    // PostUpdate, after the free-fly rig's own write and before the transform
    // propagates, for wfc_ships' reason: the rig syncs in PostUpdate and an
    // unordered Update system loses to it every frame.
    app.add_systems(
        PostUpdate,
        follow_fight_camera
            .after(WASDCameraSystems::Sync)
            .before(TransformSystems::Propagate),
    );
}

/// Which matchup is on: where the DRAFT starts reading the seed stream,
/// which two seeds it actually drafted (see [`draft_pair`]), and the look
/// both ships wear - an index into the merged style catalog, for
/// `wfc_ships`' reason: a producer must not know what a style is called.
#[derive(Resource, Clone, Copy)]
struct Matchup {
    seed: u64,
    drafted: [u64; 2],
    style: usize,
}

/// The style id `--style` asked for, resolved to an index on the first load.
#[derive(Resource)]
struct StyleRequest(Option<String>);

/// The fight's evidence: per side, rounds and torpedoes FIRED and section
/// damage DEALT to the other side.
///
/// Damage is read as per-side health-pool deltas rather than off the damage
/// event, because cladding is `HealthIsolated`: a hit soaked by a plate never
/// bubbles to the root, and most of what lands on a clad hull lands on
/// plates. The pool sums every `Health` under each ship root, so plate damage
/// counts like any other. `pool` is `None` while a side has no live root
/// (loading, rerolling, or fully destroyed), which is what keeps a teardown
/// from reading as a massacre.
#[derive(Resource, Default)]
struct Scoreboard {
    shots: [u32; 2],
    dealt: [f32; 2],
    pool: [Option<f32>; 2],
}

impl Scoreboard {
    /// Both sides have fired and both have dealt damage: the fight happened.
    /// The driven walk's advance condition, so it exists only where the walk
    /// does.
    #[cfg(feature = "debug")]
    fn fight_happened(&self) -> bool {
        self.shots.iter().all(|shots| *shots > 0) && self.dealt.iter().all(|dealt| *dealt > 0.0)
    }
}

/// The side index of an allegiance, or `None` for a neutral bystander.
fn side_of(allegiance: &Allegiance) -> Option<usize> {
    SIDES.iter().position(|side| side.allegiance == *allegiance)
}

fn load_arena(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    requested: Res<StyleRequest>,
    mut matchup: ResMut<Matchup>,
) {
    if let Some(id) = requested.0.as_deref() {
        match styles.iter().position(|style| style.id == id) {
            Some(index) => matchup.style = index,
            // Loud, not silent: a typo would otherwise dress the fight in the
            // first look and read as the one that was asked for.
            None => panic!("--style '{id}' is not in the merged content"),
        }
    }
    let style = style_at(&styles, matchup.style);
    commands.trigger(LoadScenario(arena(
        &game_assets,
        &sections,
        &mut matchup,
        style,
    )));
    spawn_readout(&mut commands);
}

/// `R` re-rolls both hulls from the seed stream past the last draft, `L`
/// steps both to the next authored look on the SAME seeds. Either way the
/// scenario reloads through `LoadScenario` (which tears the old fight down,
/// in-flight ordnance included) and the scoreboard starts over - the score
/// of a fight that no longer exists is not evidence of anything.
fn reroll_on_key(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    mut matchup: ResMut<Matchup>,
    mut score: ResMut<Scoreboard>,
) {
    if keyboard.just_pressed(KeyCode::KeyR) {
        matchup.seed = matchup.drafted[1].wrapping_add(1);
    } else if keyboard.just_pressed(KeyCode::KeyL) {
        matchup.style = matchup.style.wrapping_add(1);
    } else {
        return;
    }
    *score = Scoreboard::default();
    let style = style_at(&styles, matchup.style);
    commands.trigger(LoadScenario(arena(
        &game_assets,
        &sections,
        &mut matchup,
        style,
    )));
}

/// Turret and bay count of a hull - the two numbers that decide whether it
/// can fight at all.
fn armament(hull: &ShipHull) -> (usize, usize) {
    let count = |prototype: &str| {
        hull.sections
            .iter()
            .filter(|section| {
                matches!(&section.source, SectionSource::Prototype(id) if id == prototype)
            })
            .count()
    };
    (
        count("pdc_kinetic_turret_section"),
        count("torpedo_section"),
    )
}

/// Minimum armament to be DRAFTED as a combatant: guns enough that some
/// bear on the target whatever the arcs rolled, and tubes enough to answer a
/// salvo with a salvo. The collapse arms hulls with wild variance - the
/// first driven run rolled 10 turrets + 8 bays against 2 turrets + 0 bays,
/// and the second ship died in eight seconds without firing once. An
/// under-armed roll is a fine SUBJECT (wfc_ships will pose it) but not a
/// combatant, so the arena drafts past it.
const MIN_TURRETS: usize = 4;
const MIN_BAYS: usize = 2;
/// How many seeds the draft may read before failing loudly.
const DRAFT_SCAN_CAP: u64 = 64;

/// Walk the seed stream from `from` and draft the first two hulls that can
/// actually fight. Deterministic: the same `from` always drafts the same
/// pair, so a matchup is reproducible from one number. Skipped seeds are
/// logged with the armament that disqualified them.
fn draft_pair(tiles: &[wfc::Tile], from: u64, style: StyleId) -> Vec<(u64, ShipHull)> {
    let mut drafted = Vec::new();
    for offset in 0..DRAFT_SCAN_CAP {
        let seed = from.wrapping_add(offset);
        let hull = wfc_hull(tiles, seed, true, style);
        let (turrets, bays) = armament(&hull);
        if turrets >= MIN_TURRETS && bays >= MIN_BAYS {
            drafted.push((seed, hull));
            if drafted.len() == SIDES.len() {
                return drafted;
            }
        } else {
            info!("wfc_arena: seed {seed} not drafted ({turrets} turrets, {bays} bays)");
        }
    }
    panic!(
        "wfc_arena: fewer than {} combat-viable hulls in seeds {from}..{} \
         (viable: >= {MIN_TURRETS} turrets and >= {MIN_BAYS} bays)",
        SIDES.len(),
        from.wrapping_add(DRAFT_SCAN_CAP),
    );
}

/// One combatant: a drafted hull, clad, under the same AI pilot the
/// campaign's raiders fly, on its side's colors.
fn combatant(index: usize, seed: u64, hull: ShipHull) -> ScenarioObjectConfig {
    let side = &SIDES[index];
    // The armament roll, per ship: the draft floor bounds it from below but
    // one side can still out-gun the other, and a lopsided fight reads
    // differently knowing that. This line is the roll's disclosure.
    let (turrets, bays) = armament(&hull);
    info!(
        "wfc_arena: {} seed {}: {} sections - {} turrets, {} bays",
        side.callsign,
        seed,
        hull.sections.len(),
        turrets,
        bays,
    );
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: format!("wfc_fighter_{index}"),
            name: format!("{} {seed}", side.callsign),
            position: side.spawn,
            rotation: Quat::from_rotation_y(side.yaw),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(side.allegiance),
            controller: SpaceshipController::AI(AIControllerConfig {
                patrol: side.patrol.to_vec(),
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

/// The arena: two collapsed combatants, the standard three-point rig, two
/// dressing rings and the landmark, under the game's own sky. Checked by the
/// REAL content lint before it is fought over, exactly like the posed row.
fn arena(
    game_assets: &GameAssets,
    sections: &GameSections,
    matchup: &mut Matchup,
    style: StyleId,
) -> ScenarioConfig {
    let tiles = tile_set(sections);
    let pair = draft_pair(&tiles, matchup.seed, style);
    for (index, (seed, _)) in pair.iter().enumerate() {
        matchup.drafted[index] = *seed;
    }
    let ships = pair
        .into_iter()
        .enumerate()
        .map(|(index, (seed, hull))| {
            EventActionConfig::SpawnScenarioObject(combatant(index, seed, hull))
        })
        .collect::<Vec<_>>();

    let scenario = ScenarioConfig {
        description: "Two wave-function-collapse ships fight in a dressed arena".to_string(),
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
                        matchup.seed ^ 0x0A11,
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
                        matchup.seed ^ 0x0FA2,
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

/// Count every round and torpedo the moment it spawns, credited to the side
/// of the ship that fired it (`ProjectileOwner` names the ship root, which
/// wears the `Allegiance`).
fn count_shots(
    mut score: ResMut<Scoreboard>,
    q_new: Query<
        &ProjectileOwner,
        Or<(
            Added<TurretBulletProjectileMarker>,
            Added<TorpedoProjectileMarker>,
        )>,
    >,
    q_side: Query<&Allegiance>,
) {
    for owner in &q_new {
        let Ok(allegiance) = q_side.get(owner.0) else {
            continue;
        };
        if let Some(side) = side_of(allegiance) {
            score.shots[side] += 1;
        }
    }
}

/// Sum every `Health` pool under each side's ship root and charge any drop to
/// the OTHER side as damage dealt - see [`Scoreboard`] for why deltas rather
/// than damage events.
fn track_damage(
    mut score: ResMut<Scoreboard>,
    q_health: Query<(Entity, &Health)>,
    q_parents: Query<&ChildOf>,
    q_roots: Query<&Allegiance, With<SpaceshipRootMarker>>,
) {
    let mut pool = [None::<f32>; 2];
    for (entity, health) in &q_health {
        let mut current = entity;
        let side = loop {
            if let Ok(allegiance) = q_roots.get(current) {
                break side_of(allegiance);
            }
            match q_parents.get(current) {
                Ok(ChildOf(parent)) => current = *parent,
                Err(_) => break None,
            }
        };
        if let Some(side) = side {
            *pool[side].get_or_insert(0.0) += health.current;
        }
    }
    for side in 0..SIDES.len() {
        let rival = (side + 1) % SIDES.len();
        match (score.pool[side], pool[side]) {
            (Some(previous), Some(now)) if now < previous - f32::EPSILON => {
                score.dealt[rival] += previous - now;
                debug!(
                    "wfc_arena: {} took {:.1} damage (pool {:.1} -> {:.1})",
                    SIDES[side].callsign,
                    previous - now,
                    previous,
                    now,
                );
            }
            // The whole side vanished: a KILL, credited to the rival. A
            // torpedo alpha strike can take a ship from intact to despawned
            // between two frames of this system - the first driven run did
            // exactly that and the scoreboard read 0.0 while a ship died on
            // camera - and under load BOTH deaths of a mutual annihilation
            // land this way, seconds apart, so the credit cannot require the
            // rival to still be standing. A reload cannot reach this arm:
            // the reroll resets the score before the teardown lands (see the
            // ordering note in `arena_plugin`).
            (Some(previous), None) => {
                score.dealt[rival] += previous;
                info!(
                    "wfc_arena: {} destroyed ({:.1} structure erased)",
                    SIDES[side].callsign, previous,
                );
            }
            _ => {}
        }
        score.pool[side] = pool[side];
    }
}

/// Seconds between scoreboard log lines: often enough to watch a run in the
/// log, rare enough not to drown it.
const REPORT_PERIOD_SECS: f32 = 5.0;

/// Restate the score on the log clock, so a driven run's transcript IS the
/// evidence: shots, damage, remaining structure and the range between the
/// hulls - the last one is the flyability readout, closing speed by eye.
fn report_score(
    score: Res<Scoreboard>,
    time: Res<Time>,
    mut last: Local<f32>,
    q_ships: Query<&Transform, With<SpaceshipRootMarker>>,
) {
    if time.elapsed_secs() - *last < REPORT_PERIOD_SECS {
        return;
    }
    *last = time.elapsed_secs();
    let positions: Vec<Vec3> = q_ships.iter().map(|ship| ship.translation).collect();
    let range = match positions.as_slice() {
        [first, second, ..] => first.distance(*second),
        _ => 0.0,
    };
    let pool = |side: usize| score.pool[side].unwrap_or(0.0);
    info!(
        "wfc_arena: {} fired {} dealt {:.1} left {:.0} | {} fired {} dealt {:.1} left {:.0} | range {:.0}",
        SIDES[0].callsign,
        score.shots[0],
        score.dealt[0],
        pool(0),
        SIDES[1].callsign,
        score.shots[1],
        score.dealt[1],
        pool(1),
        range,
    );
}

// ---------------------------------------------------------------------------
// Framing and readout.
// ---------------------------------------------------------------------------

/// How the auto-frame stands off the fight: direction (broadside to the
/// engagement axis and a little above), plus a floor and a rate on the ships'
/// spread so both hulls stay in frame from merge to knife range.
const CAMERA_DIRECTION: Vec3 = Vec3::new(0.0, 0.45, 1.0);
const CAMERA_BASE: f32 = 55.0;
const CAMERA_PER_SPREAD: f32 = 0.85;

/// Whether the auto-frame still owns the camera. Cleared the first time the
/// free-fly rig is touched, and never re-armed - a viewer who took the camera
/// keeps it.
#[derive(Resource, Default)]
struct FollowCamera(bool);

/// Hand back the camera the moment the free-fly rig is asked for anything,
/// reading the rig's own input component so a binding change cannot leave the
/// auto-frame fighting the player.
fn stop_follow_on_input(mut follow: ResMut<FollowCamera>, q_input: Query<&WASDCameraInput>) {
    if !follow.0 {
        return;
    }
    let touched = q_input
        .iter()
        .any(|input| input.pan != Vec2::ZERO || input.wasd != Vec2::ZERO || input.vertical != 0.0);
    if touched {
        follow.0 = false;
    }
}

/// Keep both combatants in frame: stand on the fight's midpoint, backed off
/// with their spread. Wrecks keep their root marker, so the camera holds the
/// aftermath too instead of snapping away on the kill.
fn follow_fight_camera(
    follow: Res<FollowCamera>,
    q_ships: Query<&Transform, (With<SpaceshipRootMarker>, Without<ScenarioCameraMarker>)>,
    mut q_camera: Query<&mut Transform, With<ScenarioCameraMarker>>,
) {
    if !follow.0 {
        return;
    }
    let positions: Vec<Vec3> = q_ships.iter().map(|ship| ship.translation).collect();
    if positions.is_empty() {
        return;
    }
    let midpoint = positions.iter().sum::<Vec3>() / positions.len() as f32;
    let spread = positions
        .iter()
        .map(|position| position.distance(midpoint))
        .fold(0.0f32, f32::max)
        * 2.0;
    let stand =
        midpoint + CAMERA_DIRECTION.normalize() * (CAMERA_BASE + spread * CAMERA_PER_SPREAD);
    for mut camera in &mut q_camera {
        *camera = Transform::from_translation(stand).looking_at(midpoint, Vec3::Y);
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
fn update_readout(
    matchup: Res<Matchup>,
    styles: Res<GameStyles>,
    score: Res<Scoreboard>,
    mut q_readout: Query<&mut Text, With<ScoreReadout>>,
) {
    let line = format!(
        "WFC arena - {} {} vs {} {} - {} - fired {}/{} - dealt {:.0}/{:.0} - [R] re-roll  [L] look",
        SIDES[0].callsign,
        matchup.drafted[0],
        SIDES[1].callsign,
        matchup.drafted[1],
        style_at(&styles, matchup.style).unwrap_or("bare"),
        score.shots[0],
        score.shots[1],
        score.dealt[0],
        score.dealt[1],
    );
    for mut text in &mut q_readout {
        if text.as_str() != line {
            **text = line.clone();
        }
    }
}

/// Seconds a load step may sit before the run aborts naming it (llvmpipe
/// headroom).
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 30.0;

/// Seconds the fight gets to prove itself: both sides firing and both dealt
/// damage. Sized UNDER the harness completion watchdog's 120 s default, so a
/// fight that never happens fails naming THIS step - the honest failure mode
/// - instead of the watchdog's anonymous laggard exit. A slow matchup can be
/// given more room with `NOVA_AUTOPILOT_DEADLINE`.
#[cfg(feature = "debug")]
const FIGHT_DEADLINE_SECS: f32 = 90.0;

/// The driven walk: load the arena, hold until the scoreboard proves both
/// sides fired and both dealt damage, then shoot the brawl. The auto-frame
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
        .step("both sides fire and connect")
        .until(resource_where::<Scoreboard>(Scoreboard::fight_happened))
        .deadline(FIGHT_DEADLINE_SECS)
        .add()
        .step("shoot the fight")
        .on_enter(|world: &mut World| shoot(world, "wfc-arena-fight.png"))
        .until(shot_written("wfc-arena-fight.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
