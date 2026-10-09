//! system_world_resume: a real resumable New Game world, carried across a
//! fresh process the way a saved game must survive a quit.
//!
//! Phase `create` (the default, no `--phase` flag) boots the shipped app,
//! clicks New Game, names and creates "Probe World", then injects fixture
//! state directly through production constructors and components: the
//! nearest streamed asteroid's `BodyRadius` is set to a carved value, a
//! mined canister is spawned with [`cargo_canister`] and a minted
//! [`CargoCanisterRuntimeId`], and the player's credits and cargo stock are
//! set. THIS IS DISCLOSED FIXTURE STATE, NOT MINING OR LOOT GAMEPLAY: no key
//! is held, no beam fires, no remesh runs. The round trip under test is
//! whether a save written by one process opens correctly in another, so the
//! proof only needs the state in place before the save, not the gesture that
//! would normally produce it. It then crosses a sector boundary, waits for
//! the crossing save, records the expected state to `expected.json` beside
//! the sandboxed worlds root, then leaves through the pause menu's Back to
//! Main Menu with the world folder read-only: the leave save fails, the
//! failed overlay is shot, the last good save is asserted unchanged. The
//! folder is made writable again, and right before the successful Try
//! again click the create phase fires three live combat transients through
//! REAL production paths on the player's own ship - a PDC round (held
//! `combat_stance` + trigger, [`FireHeld`]/[`hold_fire_inputs`]), a scripted
//! torpedo order ([`ScriptedTorpedoOrder`]), and a one-hit kill on a second
//! PDC bay for its detached piece - shoots the frame right before the leave,
//! then clicks Try again. Once that leave save lands, it reopens the just-
//! written world folder with [`open_world`] (the same typed read a real Load
//! uses, not a live-component snapshot and not hand-parsed RON) to pull the
//! AUTHORITATIVE transient records - owner, pose, remaining lifetime -
//! straight off disk, and merges them into `expected.json`. On success it
//! spawns itself again with `--phase load`, inheriting the same sandboxed
//! `NOVA_CONFIG_ROOT` and stdio, and waits for it.
//!
//! Phase `load` is a second, independent process. It never creates a world:
//! it opens the Load screen, asserts a world with no `world.ron` is listed
//! and refused, selects "Probe World" and loads it, then waits for all three
//! transient kinds to resume. EVERY saved transient is then paired, BY
//! STABLE SAVE-LIST INDEX, against the THAW-TIME entry an observer recorded
//! the instant that body was given its resumed lifetime (not a live body
//! read on this later frame): `spawn_resumed` thaws every saved transient in
//! saved-list order on one command buffer, so the Nth recorded entry is
//! always the Nth saved record. For a round, a torpedo or a detached piece,
//! the pair must also agree on owner, pose (within 1.0 m) and remaining
//! lifetime (within 0.01 s). Only once that match succeeds
//! does it shoot the frame and report (never assert) the mean per-channel
//! pixel difference against the pre-leave shot, then assert the rest of the
//! fixture state against `expected.json` - the player's pose, credits,
//! stock, the mined canister and the carved rock's own BodyRadius, and the
//! sector the player was in. It then waits for all three transient kinds to
//! expire on their own saved clocks. It also watches that [`ResumedWorld`]
//! appeared and was consumed, the one seam that proves the saved ledger -
//! not the seed - built this window. It then leaves through the pause Exit,
//! which waits for ITS OWN leave save before the process exits.
//!
//! ONE SUBJECT: a save written by one process opens correctly in another.
//! The save and load screens, the leave overlay and the status line are
//! `nova_menu`'s own range; the save format and the writer are
//! `nova_world_base`'s. This range is the player-visible round trip only.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_world_resume --features debug
//! # look for: `world_resume: create phase wrote generation >= 2 for 'Probe World'`,
//! #           `world_resume load: the resumed ledger seam ran`,
//! #           `world_resume load: PASS every fixture value matched`,
//! #           `autopilot: cycle complete, no panic` (create phase only: the
//! #           load phase's Exit ends the process before the script does)
//! ```

#[cfg(feature = "debug")]
use avian3d::prelude::{AngularVelocity, LinearVelocity, Position};
#[cfg(feature = "debug")]
use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    prelude::*,
};
use clap::Parser;
#[cfg(feature = "debug")]
use nova_protocol::prelude::*;
#[cfg(feature = "debug")]
use nova_ui::widget::TextFieldValue;
#[cfg(feature = "debug")]
use nova_world::prelude::{CurrentSector, WorldConfig};

#[derive(Parser)]
#[command(name = "system_world_resume")]
#[command(version = "1.0.0")]
#[command(
    about = "A real resumable New Game world across a fresh process: create, carve, cross a \
             sector, leave, then reopen and check the saved state. Autopilot-only correctness \
             range - play the game to fly the world",
    long_about = None
)]
struct Cli {
    /// Which half of the round trip this process runs. The create phase
    /// spawns the load phase itself; a human never passes this.
    #[cfg(feature = "debug")]
    #[arg(long, value_enum, default_value_t = Phase::Create)]
    phase: Phase,
}

/// The two processes one round trip needs.
#[cfg(feature = "debug")]
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Make the world, inject the fixture, cross a sector, leave, then spawn
    /// the `Load` phase and wait for it.
    Create,
    /// A fresh process: open the saved world and check it against
    /// `expected.json`.
    Load,
}

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();

    #[cfg(not(feature = "debug"))]
    {
        editor_app(true, None).run()
    }

    #[cfg(feature = "debug")]
    match cli.phase {
        Phase::Create => run_create(),
        Phase::Load => run_load(),
    }
}

/// The world name every phase reads and writes.
#[cfg(feature = "debug")]
const WORLD_NAME: &str = "Probe World";

/// The slug [`WORLD_NAME`] gives: `world_slug` lowercases and hyphenates it.
#[cfg(feature = "debug")]
const WORLD_SLUG: &str = "probe-world";

/// A folder with no `world.ron`: the Load list's refused row.
#[cfg(feature = "debug")]
const BROKEN_WORLD_SLUG: &str = "broken-world";

/// A fixed seed, so the window around spawn is the same shape every run.
#[cfg(feature = "debug")]
const WORLD_SEED: &str = "20261007";

/// The credits the fixture sets, chosen to be nothing the base ship spawns
/// with.
#[cfg(feature = "debug")]
const FIXTURE_CREDITS: u32 = 918_273;

/// The main menu's way in.
#[cfg(feature = "debug")]
const NEW_GAME_BUTTON: &str = "New Game Button";
#[cfg(feature = "debug")]
const WORLD_NAME_FIELD: &str = "World Name Field";
#[cfg(feature = "debug")]
const SEED_FIELD: &str = "World Seed Field";
#[cfg(feature = "debug")]
const CREATE_WORLD_BUTTON: &str = "Create World Button";
#[cfg(feature = "debug")]
const LOAD_BUTTON: &str = "Load Button";
#[cfg(feature = "debug")]
const LOAD_WORLD_BUTTON: &str = "Load World Button";
#[cfg(feature = "debug")]
const PAUSE_EXIT_BUTTON: &str = "Pause Exit Button";
#[cfg(feature = "debug")]
const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
#[cfg(feature = "debug")]
const LEAVE_TRY_AGAIN_BUTTON: &str = "Leave Try Again Button";
#[cfg(feature = "debug")]
const LOAD_SCREEN: &str = "Scenario Loading Screen";
#[cfg(feature = "debug")]
const LEAVE_OVERLAY: &str = "Leave Overlay";
#[cfg(feature = "debug")]
const SAVE_STATUS_LINE: &str = "Save Status Line";

/// Seconds a load or a stream gets on a software-rendered CI GPU.
#[cfg(feature = "debug")]
const SESSION_SECS: f32 = 90.0;
/// Seconds a single click-and-settle beat gets.
#[cfg(feature = "debug")]
const BEAT_DEADLINE_SECS: f32 = 30.0;
/// Seconds the crossing save gets to reach generation 2. Generous: the fresh
/// sector streams a full 125-cell window (`active_radius` 2) around the
/// player on a software-rendered GPU, which alone measured 25s under
/// lavapipe with the Open World scenario's own combat running at the same
/// time.
#[cfg(feature = "debug")]
const CROSSING_DEADLINE_SECS: f32 = 180.0;
/// Seconds a leave save gets to fail or land: the create phase's return to
/// the menu, or the load phase's Exit, which production ends with `AppExit`
/// itself.
#[cfg(feature = "debug")]
const LEAVE_DEADLINE_SECS: f32 = 60.0;

/// Seconds the fixture gets to deploy a stowed PDC, land its first round,
/// launch the torpedo and detach the piece - and, on the load side, for the
/// same three markers to come back after a Load. Generous rather than tight:
/// the PDC's own `projectile_lifetime` is only 2.0 s once it fires, but
/// GETTING to that first round crosses a stowed-to-deployed travel first,
/// which a software-rendered GPU can stretch well past the round's own
/// lifetime. Keeping this short relative to the whole script (not relative
/// to 2.0 s) is what minimizes the round's exposure: the fewer frames queued
/// between "a round exists" and the leave click, the less of its 2 s window
/// the detour spends.
#[cfg(feature = "debug")]
const PDC_FIRE_DEADLINE_SECS: f32 = 180.0;

/// Seconds the "then expires" wait gets, on the load side, for the slowest
/// of the three saved transients to reach zero and despawn: the torpedo,
/// whose `projectile_lifetime` is 100 s. Sized off that worst remaining
/// lifetime, not the round's or the piece's shorter ones, plus margin for a
/// software-rendered run (NOVA_AUTOPILOT_DEADLINE must be raised to match
/// for both processes when running this walk - see the module header).
#[cfg(feature = "debug")]
const TRANSIENT_EXPIRE_DEADLINE_SECS: f32 = 240.0;

/// This run's own sandboxed config root: a world folder Create makes, and
/// nothing a player's own saves or another run's "Probe World" can collide
/// with.
#[cfg(feature = "debug")]
fn sandbox_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "target/example-profiles/{}-{}-{}",
        env!("CARGO_CRATE_NAME"),
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock is after 1970")
            .as_nanos(),
    ))
}

/// Fixture state the create phase records for the load phase to check.
#[cfg(feature = "debug")]
#[derive(Resource, Default, Debug)]
struct Fixture {
    /// The nearest streamed asteroid's root entity, once chosen.
    rock: Option<Entity>,
    /// That rock's scenario id, so the load phase can find it again.
    rock_id: Option<EntityId>,
    /// The fixture canister, set the same frame it is spawned.
    canister: Option<Entity>,
}

/// Watch for the resumed ledger seam, in the load phase only: whether
/// [`ResumedWorld`] was ever seen, so the check survives it being consumed
/// the same frame the world arms.
#[cfg(feature = "debug")]
#[derive(Resource, Default, Debug)]
struct LoadWatch {
    saw_resumed_world: bool,
}

#[cfg(feature = "debug")]
fn watch_resumed_world(resumed: Option<Res<ResumedWorld>>, mut watch: ResMut<LoadWatch>) {
    if resumed.is_some() {
        watch.saw_resumed_world = true;
    }
}

/// The one player ship, if exactly one stands.
#[cfg(feature = "debug")]
fn the_player(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

/// Write `text` into the node called `name`'s field.
#[cfg(feature = "debug")]
fn set_field_text(world: &mut World, name: &str, text: &str) {
    let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
    let (_, mut value) = fields
        .iter_mut(world)
        .find(|(found, _)| found.as_str() == name)
        .unwrap_or_else(|| panic!("world_resume: no field named '{name}'"));
    value.0 = text.to_string();
}

/// Advance once one player ship stands in an armed world.
#[cfg(feature = "debug")]
fn armed_around_a_player() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world.contains_resource::<WorldConfig<NovaLayeredWorld>>() && the_player(world).is_some()
    })
}

/// Advance once the crossing save has reached at least generation 2: the
/// first-arm write (D8) took generation 1 before the fixture ever moved.
#[cfg(feature = "debug")]
fn saved_past_first_write() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<WorldSaveSession>(
        |session| matches!(session.status(), WorldSaveStatus::Saved { generation } if *generation >= 2),
    )
}

/// Advance once `rock_id` is resident in the live ECS. The Load phase's
/// streaming budget loads sectors over several frames, so a rock one sector
/// off the player's exact cell can lag a frame or two behind
/// `armed_around_a_player`'s own, coarser check.
#[cfg(feature = "debug")]
fn fixture_rock_streamed_in(
    rock_id: String,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        let Some(mut rocks) = world.try_query_filtered::<&EntityId, With<AsteroidMarker>>() else {
            return false;
        };
        rocks.iter(world).any(|id| id.0 == rock_id)
    })
}

/// The nearest `AsteroidMarker` root to `from`, with its id.
#[cfg(feature = "debug")]
fn nearest_asteroid(world: &mut World, from: Vec3) -> (Entity, EntityId) {
    let mut rocks =
        world.query_filtered::<(Entity, &GlobalTransform, &EntityId), With<AsteroidMarker>>();
    rocks
        .iter(world)
        .min_by(|a, b| {
            let da = a.1.translation().distance(from);
            let db = b.1.translation().distance(from);
            da.partial_cmp(&db).expect("asteroid distances are finite")
        })
        .map(|(entity, _, id)| (entity, id.clone()))
        .expect("world_resume: the live window holds at least one asteroid")
}

/// Inject the fixture state the load phase will check: a carved rock, a
/// mined canister and the player's credits and cargo stock. Every value is
/// set through a production component or a production constructor
/// (`BodyRadius`, [`cargo_canister`], `CargoCanisterIdAllocator::mint`,
/// `ShipCredits`, `ShipInventory::add`), in the same frame - this is
/// DISCLOSED FIXTURE STATE, not the mining or looting gameplay that would
/// normally produce it. Nothing here holds a key, fires a beam or waits on a
/// remesh.
#[cfg(feature = "debug")]
fn inject_fixture(world: &mut World) {
    let player = the_player(world).expect("world_resume: exactly one player ship");
    let player_pos = world
        .get::<GlobalTransform>(player)
        .expect("the player ship has a transform")
        .translation();
    let (rock, rock_id) = nearest_asteroid(world, player_pos);

    // Carve: set BodyRadius straight to a value distinct from the rock's
    // untouched radius. This stands in for a real carve's result; it does
    // not rebuild a collider or a mesh, because only the saved-vs-resumed
    // equality is under test here.
    let body_radius_before = world
        .get::<BodyRadius>(rock)
        .expect("an asteroid root carries BodyRadius")
        .0;
    let body_radius_after = body_radius_before * 0.9;
    world.entity_mut(rock).insert(BodyRadius(body_radius_after));

    // Canister: spawn one with the same constructor the real ejector uses,
    // minting its own runtime id, right beside the rock.
    let canister_id = world.resource_mut::<CargoCanisterIdAllocator>().mint();
    let rock_pos = world
        .get::<GlobalTransform>(rock)
        .expect("the fixture rock has a transform")
        .translation();
    let canister_contents = CargoCanister::new(ItemType::IronOre, 3);
    let canister = world
        .spawn((
            cargo_canister(
                canister_contents,
                Transform::from_translation(rock_pos + Vec3::X * 5.0),
                Vec3::ZERO,
                AssetRef::from(MINED_CANISTER_MESH),
            ),
            canister_id,
            ScenarioScopedMarker,
        ))
        .id();

    world
        .get_mut::<ShipCredits>(player)
        .expect("the player ship carries ShipCredits")
        .0 = FIXTURE_CREDITS;

    {
        let mut inventory = world
            .get_mut::<ShipInventory>(player)
            .expect("the player ship carries ShipInventory");
        let unit_mass = u64::from(ItemType::Rations.mass_g());
        let max_count = (u64::from(inventory.free_g()) / unit_mass) as u32;
        assert!(
            max_count >= 1,
            "world_resume: the line warship has no room for fixture stock (free {}g)",
            inventory.free_g()
        );
        inventory.add(ItemType::Rations, max_count.min(5).max(1));
    }

    let mut fixture = world.resource_mut::<Fixture>();
    fixture.rock = Some(rock);
    fixture.rock_id = Some(rock_id.clone());
    fixture.canister = Some(canister);
    info!(
        "world_resume: fixture state injected directly, not through gameplay: rock {} \
         BodyRadius {body_radius_before:.2} -> {body_radius_after:.2}, canister {} minted, \
         credits set to {FIXTURE_CREDITS}",
        rock_id.0, canister_id.0
    );
    nova_probe::probe_marker(
        world,
        "outcome: the create phase injects carve, canister and credit fixture state directly \
         through production APIs, disclosed as fixture state and not gameplay",
        serde_json::json!({
            "rock_id": rock_id.0,
            "body_radius_before": body_radius_before,
            "body_radius_after": body_radius_after,
            "canister_id": canister_id.0,
        }),
    );
}

/// Teleport the player past the open world's own sector edge, straight into
/// the next cell along +X: the fixture crosses a boundary without flying it,
/// the same direct-teleport technique `system_world_sectors`'s
/// `park_observer` uses for its own camera.
#[cfg(feature = "debug")]
fn cross_sector_edge(world: &mut World) {
    let edge = world
        .resource::<WorldConfig<NovaLayeredWorld>>()
        .sector_edge;
    let current = world.resource::<CurrentSector>().0;
    let beyond = current.offset(1, 0, 0).centre(edge).to_engine();
    let player = the_player(world).expect("world_resume: exactly one player ship");
    world.entity_mut(player).insert((
        Position(beyond),
        LinearVelocity::ZERO,
        AngularVelocity::ZERO,
    ));
    info!("world_resume: crossing from {current} toward {beyond}");
}

/// The JSON object the create phase writes and the load phase reads. Its
/// `player`/`stock` fields are the LIVE pre-click sample, recorded before any
/// leave attempt - log-only context once `record_transient_fixture_state`
/// merges in `saved_player`, the leave save's own record and the only one
/// `assert_resumed_state` checks the player's pose, credits and stock
/// against.
#[cfg(feature = "debug")]
fn record_expected_state(world: &mut World) -> serde_json::Value {
    let player = the_player(world).expect("world_resume: exactly one player ship");
    let pose = world
        .get::<Position>(player)
        .expect("the player ship has a physics pose")
        .0;
    let credits = world
        .get::<ShipCredits>(player)
        .expect("the player ship carries ShipCredits")
        .0;
    let stock: Vec<_> = world
        .get::<ShipInventory>(player)
        .expect("the player ship carries ShipInventory")
        .stacks()
        .map(|(item, count)| serde_json::json!({ "item": format!("{item:?}"), "count": count }))
        .collect();
    let fixture = world.resource::<Fixture>();
    let rock_id = fixture
        .rock_id
        .clone()
        .expect("world_resume: the fixture rock was chosen")
        .0;
    let rock = fixture
        .rock
        .expect("world_resume: the fixture rock was chosen");
    let rock_radius = world
        .get::<BodyRadius>(rock)
        .expect("the fixture rock still carries BodyRadius")
        .0;
    let canister_entity = fixture
        .canister
        .expect("world_resume: a mined canister was observed");
    let canister_id = world
        .get::<CargoCanisterRuntimeId>(canister_entity)
        .expect("a mined canister carries CargoCanisterRuntimeId")
        .0;
    let canister_contents: Vec<_> = world
        .get::<CargoCanister>(canister_entity)
        .expect("a mined canister carries CargoCanister")
        .stacks()
        .map(|(item, count)| serde_json::json!({ "item": format!("{item:?}"), "count": count }))
        .collect();
    let sector = world.resource::<CurrentSector>().0;

    serde_json::json!({
        "world_name": WORLD_NAME,
        "sector": [sector.x, sector.y, sector.z],
        "player": { "pose": [pose.x, pose.y, pose.z], "credits": credits },
        "stock": stock,
        "rock": { "id": rock_id, "radius": rock_radius },
        "canister": { "id": canister_id, "contents": canister_contents },
    })
}

/// Assert the sandboxed worlds root now lists "Probe World" at generation 2
/// or later (the first arm plus the crossing write), straight off disk.
/// Done here, in-process, because the resource `list_worlds` needs is gone
/// by the time `app.run()` returns.
#[cfg(feature = "debug")]
fn assert_probe_world_on_disk(world: &mut World) {
    let root = nova_assets::storage::worlds_root()
        .expect("world_resume: the sandboxed CONFIG_ROOT must give a worlds root");
    let packs = world.resource::<LoadedSectionPacks>();
    let listings = list_worlds(&root, packs)
        .unwrap_or_else(|e| panic!("world_resume: cannot list the sandboxed worlds root: {e}"));
    let probe = listings
        .iter()
        .find(|listing| listing.folder.slug == WORLD_SLUG)
        .unwrap_or_else(|| panic!("world_resume: '{WORLD_SLUG}' is not in {listings:?}"));
    let header = probe.header.as_ref().unwrap_or_else(|refusal| {
        panic!("world_resume: '{WORLD_SLUG}' header is refused: {refusal}")
    });
    assert!(
        header.generation >= 2,
        "world_resume: the saved header is generation {}, expected at least 2 (first arm + a \
         crossing)",
        header.generation
    );
    assert_eq!(header.name, WORLD_NAME);
    info!(
        "world_resume: create phase wrote generation {} for '{}'",
        header.generation, header.name
    );
}

/// Build the fixture's expected-state JSON and write it to `path`.
#[cfg(feature = "debug")]
fn write_expected_state(path: std::path::PathBuf) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let expected = record_expected_state(world);
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&expected).expect("expected state must encode"),
        )
        .unwrap_or_else(|e| panic!("world_resume: cannot write {}: {e}", path.display()));
        info!("world_resume: wrote expected state to {}", path.display());
        nova_probe::probe_marker(
            world,
            "outcome: the create phase records the fixture's expected state",
            expected,
        );
    }
}

/// The inputs the fixture holds to fire a PDC: a resource re-applied every
/// frame, same reason `system_turret_gunnery`'s `HeldInput` is - `ButtonInput`
/// is a live map the game's own systems read every frame, and the weapons
/// safety derives `WeaponsHot` from the HELD combat stance, so a one-shot
/// press applied on a step's entry would be a press, not a hold.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct FireHeld {
    /// `combat_stance` (RMB by default) - raises `WeaponsRaised`, which the
    /// deploy demand and `WeaponsHot` derive from every frame. Set true one
    /// step ahead of `pdc`: the torpedo bay's own fire gate
    /// (`crates/nova_ship/src/sections/torpedo_section/bay.rs:466-470`,
    /// "same rule as the turret") also checks `WeaponsHot` on a managed
    /// ship, so `combat` must already read true before the torpedo order
    /// commits, not just before the PDC trigger does.
    combat: bool,
    /// The PDC's own per-section trigger: raw `MouseButton::Left`, the
    /// device state every spawned turret's `input_mapping` reads directly
    /// (`crates/nova_authoring/src/base_content/scenarios/open_world.rs:93`).
    /// Set directly by the step that wants the round, one step after
    /// `combat` goes true - see [`hold_fire_inputs`] for why a same-frame
    /// `combat`+`pdc` press cannot be allowed to happen.
    pdc: bool,
}

/// Re-press [`FireHeld`]'s inputs for the frame.
///
/// Wired as the script's `AutopilotPlugin::input` hook, NOT an `Update`
/// system - the same `hold_inputs` idiom `system_turret_gunnery.rs` uses and
/// explains: `PreUpdate`, after `InputSystems`, is the only place a
/// synthesized press is still `just_pressed` when the weapon's own `Update`
/// system reads it.
///
/// Each field presses straight through, no latch: `combat` is set a full
/// step ahead of `pdc` by the script itself (see [`FireHeld`]), so by the
/// frame `pdc` goes true `WeaponsHot` is already true. A same-frame
/// `combat`+`pdc` press would fire `Start<TurretInput>` once while
/// `WeaponsHot` (an `Update`-derived value) is still false that frame, so
/// `on_turret_input` would return and the action would latch `Fired` with no
/// further `Start` to retry it - the round would never fire.
#[cfg(feature = "debug")]
fn hold_fire_inputs(world: &mut World, _elapsed: f32, _frame: u32) {
    let held = world.resource::<FireHeld>();
    let (combat, pdc) = (held.combat, held.pdc);
    if combat {
        drive_action(world, "combat_stance", InputPhase::Press);
    }
    if pdc {
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
    }
}

/// Two distinct PDC bays (one to fire, one to kill) and the torpedo bay, all
/// children of `player` - the same `(Entity, &ChildOf)` idiom
/// `nova_scenario::objects::spaceship`'s own section tests use to find a
/// ship's children, since `ChildOf` carries no queryable value, only a value
/// to read per match.
#[cfg(feature = "debug")]
fn player_weapon_fixtures(world: &mut World, player: Entity) -> (Entity, Entity, Entity) {
    let mut turrets = world.query_filtered::<(Entity, &ChildOf), With<TurretSectionMarker>>();
    let mut pdc_bays: Vec<Entity> = turrets
        .iter(world)
        .filter(|(_, child_of)| child_of.parent() == player)
        .map(|(entity, _)| entity)
        .collect();
    pdc_bays.sort();
    assert!(
        pdc_bays.len() >= 2,
        "world_resume: the player ship needs at least two PDC bays (one to fire, one to kill), \
         found {}",
        pdc_bays.len()
    );
    let fire_pdc = pdc_bays[0];
    let kill_pdc = pdc_bays[1];

    let mut bays = world.query_filtered::<(Entity, &ChildOf), With<TorpedoSectionMarker>>();
    let torpedo_bay = bays
        .iter(world)
        .find(|(_, child_of)| child_of.parent() == player)
        .map(|(entity, _)| entity)
        .expect("world_resume: the player ship has a torpedo bay");

    (fire_pdc, kill_pdc, torpedo_bay)
}

/// Arm `combat_stance` via [`FireHeld`], launch a scripted one-shot torpedo
/// order on the bay, and deal a second PDC bay's own `Health.max` as damage -
/// one hit, no overkill, so nothing propagates up `ChildOf` to the hull.
/// `combat` must already be arming here, a full step ahead of the PDC
/// trigger: the torpedo bay's own fire gate checks `WeaponsHot` on a managed
/// ship too (`crates/nova_ship/src/sections/torpedo_section/bay.rs:466-470`,
/// "same rule as the turret"), so without it the torpedo order would sit
/// held and never commit. The PDC round itself is fired by the following
/// step, which only sets `FireHeld.pdc` (see [`hold_fire_inputs`]): firing
/// the torpedo and the kill first, ahead of the round, shrinks the round's
/// own exposure to the rest of this sequence.
///
/// Every action here is a scripted production path - an autopilot-held
/// trigger, a scripted torpedo order, scripted lethal damage - not player
/// interaction: the round trip under test is the save/Load fidelity of
/// whatever is already in flight, not the gesture that puts it there.
#[cfg(feature = "debug")]
fn fire_transient_fixture(world: &mut World) {
    let player = the_player(world).expect("world_resume: exactly one player ship");
    let (_fire_pdc, kill_pdc, torpedo_bay) = player_weapon_fixtures(world, player);

    world.resource_mut::<FireHeld>().combat = true;

    // The named production mechanism for a controller-less launch: target
    // Entity::PLACEHOLDER is not a live SpaceshipRootMarker, so this commits
    // as a real DumbFire torpedo (torpedo_section/scripted.rs:56-61).
    world.entity_mut(torpedo_bay).insert(ScriptedTorpedoOrder {
        target: Entity::PLACEHOLDER,
    });

    let max = world
        .get::<Health>(kill_pdc)
        .expect("a turret section carries Health")
        .max;
    world.trigger(HealthApplyDamage {
        entity: kill_pdc,
        source: None,
        amount: max,
    });

    info!(
        "world_resume: armed combat_stance, fired a scripted torpedo order on {torpedo_bay} and \
         scripted lethal damage on {kill_pdc}; the PDC round follows in the next step"
    );
}

/// At least one of each fixture transient is live: the load phase's "first
/// restored frame" gate, and (reused, since the shape is the same) the
/// create phase's own wait for the torpedo and the piece once the round is
/// confirmed. This only checks that each kind exists, not how many - a held
/// PDC trigger fires continuously, so a run can save well over one round;
/// the load phase's one-to-one match against every saved record of a kind
/// (see the `assert_resumed_state` step in `load_script`) is what pairs each
/// one, not this predicate.
#[cfg(feature = "debug")]
fn fixture_transients_restored() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    and(
        any_entity::<With<TurretBulletProjectileMarker>>(),
        and(
            any_entity::<With<TorpedoProjectileMarker>>(),
            any_entity::<With<DetachedPieceMarker>>(),
        ),
    )
}

/// All three fixture transients are gone: the "then expires" gate. The
/// deadline watcher on this wait, not an assertion on elapsed time, is what
/// fails if one never despawns.
#[cfg(feature = "debug")]
fn fixture_transients_expired() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    and(
        nova_autopilot::predicate::not(any_entity::<With<TurretBulletProjectileMarker>>()),
        and(
            nova_autopilot::predicate::not(any_entity::<With<TorpedoProjectileMarker>>()),
            nova_autopilot::predicate::not(any_entity::<With<DetachedPieceMarker>>()),
        ),
    )
}

/// Read the just-written leave save's OWN records back off disk with
/// [`open_world`] - the save's own types, the same typed read a real Load
/// uses, never hand-parsed RON. Returns `{ "transients": [...],
/// "saved_player": {...} }`.
///
/// `transients` holds every `state.transients` record, in the SAME order
/// `state.transients` keeps (the order `spawn_resumed`,
/// `crates/nova_world_base/src/save/transients.rs:707-736`, thaws them in),
/// tagged by kind, with owner/pose/remaining lifetime where that kind
/// carries them. This stable order is what lets the load phase pair each
/// thaw-time entry against its saved record by index.
///
/// `saved_player` holds the player's pose (`state.player.transform`),
/// credits (`header.credits`) and every stock entry
/// (`state.player.ship`'s own `Serialize`, since `FrozenShip`'s fields are
/// private to `nova_scenario` - read through the trait, not a new
/// accessor). This, not the live pre-click sample `write_expected_state`
/// wrote earlier (`expected["player"]`/`expected["stock"]`), is the
/// authoritative reference `assert_resumed_state` checks the Load against:
/// the live sample can still move during the leave save's own settle
/// window (`drive_pending_leave`, `crates/nova_menu/src/leave.rs:100-105`,
/// releases `FreezeOwner::PauseMenu` while a write is in flight).
///
/// # Panics
///
/// If the leave save's `transients` list has no turret round, no torpedo, or
/// no detached piece - named, not silently accepted - or if any round's or
/// torpedo's saved owner is not the player ship.
#[cfg(feature = "debug")]
fn record_transient_fixture_state(world: &mut World) -> serde_json::Value {
    let root = nova_assets::storage::worlds_root()
        .expect("world_resume: the sandboxed CONFIG_ROOT must give a worlds root");
    let (_folder, _lock, header, state) =
        open_world(&root, WORLD_SLUG, world.resource::<LoadedSectionPacks>()).unwrap_or_else(|e| {
            panic!(
                "world_resume: cannot open the just-saved '{WORLD_SLUG}' to read its leave save's \
                 transients: {e}"
            )
        });
    // _lock drops at the end of this function, well before the load phase is
    // spawned as its own process, so it never contends with the real Load's
    // own lock on the same folder.

    // The saved owner, as just the id string ("player") rather than a Debug
    // rendering of SavedOwner, so the load phase can compare it straight
    // against the live owner's EntityId with no re-parsing.
    let owner_id = |owner: &SavedOwner, what: &str, index: usize| -> String {
        match owner {
            SavedOwner::Ship(id) => id.0.clone(),
            SavedOwner::Gone => panic!(
                "world_resume: the saved {what} record {index}'s owner is Gone, expected the \
                 player ship"
            ),
        }
    };

    let transients_json: Vec<_> = state
        .transients
        .iter()
        .enumerate()
        .map(|(index, transient)| match &transient.body {
            FrozenTransientType::Round(round) => {
                let owner = owner_id(&round.flight.owner, "round", index);
                assert_eq!(
                    owner, "player",
                    "world_resume: the saved round record {index}'s owner is '{owner}', \
                     expected 'player'"
                );
                serde_json::json!({
                    "kind": "round",
                    "owner": owner,
                    "pose": [
                        round.flight.translation.x,
                        round.flight.translation.y,
                        round.flight.translation.z,
                    ],
                    "lifetime_remaining": transient.lifetime.remaining,
                })
            }
            FrozenTransientType::Torpedo(torpedo) => {
                let owner = owner_id(&torpedo.owner, "torpedo", index);
                assert_eq!(
                    owner, "player",
                    "world_resume: the saved torpedo record {index}'s owner is '{owner}', \
                     expected 'player'"
                );
                serde_json::json!({
                    "kind": "torpedo",
                    "owner": owner,
                    "pose": [torpedo.translation.x, torpedo.translation.y, torpedo.translation.z],
                    "lifetime_remaining": transient.lifetime.remaining,
                })
            }
            FrozenTransientType::DetachedPiece(piece) => {
                let translation = piece.translation();
                serde_json::json!({
                    "kind": "piece",
                    "owner": null,
                    "pose": [translation.x, translation.y, translation.z],
                    "lifetime_remaining": transient.lifetime.remaining,
                })
            }
            FrozenTransientType::ShedFixture(_) | FrozenTransientType::RockChunk(_) => {
                serde_json::json!({
                    "kind": "other",
                    "owner": null,
                    "pose": null,
                    "lifetime_remaining": transient.lifetime.remaining,
                })
            }
        })
        .collect();

    let kinds: Vec<_> = transients_json.iter().map(|v| v["kind"].clone()).collect();
    let count_of = |kind: &str| transients_json.iter().filter(|v| v["kind"] == kind).count();
    let (round_count, torpedo_count, piece_count) =
        (count_of("round"), count_of("torpedo"), count_of("piece"));
    assert!(
        round_count > 0,
        "world_resume: the leave save has no turret round transient; saved kinds: {kinds:?}"
    );
    assert!(
        torpedo_count > 0,
        "world_resume: the leave save has no torpedo transient; saved kinds: {kinds:?}"
    );
    assert!(
        piece_count > 0,
        "world_resume: the leave save has no detached piece transient; saved kinds: {kinds:?}"
    );

    info!(
        "world_resume: the leave save holds {} transients: {round_count} round(s), \
         {torpedo_count} torpedo(es), {piece_count} piece(s)",
        transients_json.len(),
    );
    nova_probe::probe_marker(
        world,
        "outcome: the leave save keeps every live transient, in save-list order, with a kind \
         tag and (for a round, a torpedo or a detached piece) its owner, pose and remaining \
         lifetime",
        serde_json::json!({
            "kinds": kinds,
            "count": transients_json.len(),
            "rounds": round_count,
            "torpedoes": torpedo_count,
            "pieces": piece_count,
        }),
    );

    // The player's saved pose, credits and stock, read straight off
    // `state.player`/`header` - not the live pre-click sample. `FrozenShip`'s
    // fields (`credits`, `state.inventory`) are private to `nova_scenario`
    // (only `has_section` is public), so `state.player.ship`'s stock is read
    // through its own `Serialize` impl rather than a new accessor: that impl
    // runs inside `nova_scenario`, where the fields are visible, and exposes
    // them through the public `serde_json::to_value` call here, not through
    // any new API surface. `header.credits` is already a public field, the
    // simpler path for credits.
    let saved_ship = serde_json::to_value(&state.player.ship)
        .expect("world_resume: the saved player's ship encodes to JSON");
    let mut saved_stock: Vec<_> = saved_ship["state"]["inventory"]["stacks"]
        .as_object()
        .expect("world_resume: a saved ship's inventory stacks are a JSON object")
        .iter()
        .map(|(item, count)| serde_json::json!({ "item": item, "count": count }))
        .collect();
    saved_stock.sort_by(|a, b| a["item"].as_str().cmp(&b["item"].as_str()));
    let saved_pose = state.player.transform.translation;
    let saved_player = serde_json::json!({
        "pose": [saved_pose.x, saved_pose.y, saved_pose.z],
        "credits": header.credits,
        "stock": saved_stock,
    });

    serde_json::json!({
        "transients": transients_json,
        "saved_player": saved_player,
    })
}

/// Mean absolute per-channel difference between two captured PNGs under
/// `NOVA_CAPTURE_DIR` (mirroring [`nova_autopilot::capture::capture_path`]'s
/// own relative-path resolution, which is crate-private), decoded with
/// `Image::from_buffer` exactly as `examples/playable/shared/compare.rs`
/// already does for its candidate textures - a plain CPU decode, no
/// `Assets<Image>` needed.
///
/// REPORTED, never asserted: two independently-rendered frames across two
/// processes differ in UI state, antialiasing and HUD timing even when the
/// resumed world is pixel-faithful, so a hard threshold here would really be
/// asserting how fast the two frames converged - exactly what "never assert
/// timing" (AGENTS.md) rules out. A dimension mismatch still panics: that is
/// a real bug, not noise. A no-op (logged) off the capture path, mirroring
/// how `shoot` itself behaves.
///
/// # Panics
///
/// If either PNG cannot be read or decoded, or the two frames are not the
/// same size.
#[cfg(feature = "debug")]
fn report_pixel_difference(world: &mut World, before_path: &str, after_path: &str) {
    if !capturing() {
        info!("world_resume: not on the capture path, skipping the pixel-difference figure");
        return;
    }

    let capture_dir = std::env::var("NOVA_CAPTURE_DIR")
        .ok()
        .filter(|dir| !dir.is_empty());
    let resolve = |path: &str| -> std::path::PathBuf {
        let path = std::path::Path::new(path);
        match &capture_dir {
            Some(dir) if !path.is_absolute() => std::path::Path::new(dir).join(path),
            _ => path.to_path_buf(),
        }
    };
    let decode = |path: &std::path::Path| -> Image {
        let bytes = std::fs::read(path)
            .unwrap_or_else(|e| panic!("world_resume: read {}: {e}", path.display()));
        Image::from_buffer(
            &bytes,
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            false,
            ImageSampler::default(),
            RenderAssetUsages::default(),
        )
        .unwrap_or_else(|e| panic!("world_resume: decode {}: {e}", path.display()))
    };

    let before = resolve(before_path);
    let after = resolve(after_path);
    let before_image = decode(&before);
    let after_image = decode(&after);

    assert_eq!(
        before_image.texture_descriptor.size,
        after_image.texture_descriptor.size,
        "world_resume: {} and {} are different sizes ({:?} vs {:?}); a real bug, not noise",
        before.display(),
        after.display(),
        before_image.texture_descriptor.size,
        after_image.texture_descriptor.size
    );

    let before_data = before_image
        .data
        .as_ref()
        .expect("world_resume: a decoded PNG carries pixel data");
    let after_data = after_image
        .data
        .as_ref()
        .expect("world_resume: a decoded PNG carries pixel data");
    let diff_sum: f64 = before_data
        .iter()
        .zip(after_data.iter())
        .map(|(a, b)| f64::from((i32::from(*a) - i32::from(*b)).unsigned_abs()))
        .sum();
    #[expect(
        clippy::cast_precision_loss,
        reason = "a byte count as an f64 divisor for a reported mean, not an assertion"
    )]
    let mean_abs_diff = diff_sum / before_data.len() as f64;

    info!(
        "world_resume: pixel-difference figure between {} and {}: mean |channel diff| = {mean_abs_diff:.3} \
         (0-255 scale) - a measurement for a person to judge, never asserted",
        before.display(),
        after.display()
    );
    nova_probe::probe_marker(
        world,
        "outcome: the frame before the leave and the first frame after the Load are compared by \
         a reported pixel-difference figure, never asserted",
        serde_json::json!({
            "before": before.display().to_string(),
            "after": after.display().to_string(),
            "mean_abs_channel_diff": mean_abs_diff,
        }),
    );
}

/// The create phase's walk: New Game through the leave save.
#[cfg(feature = "debug")]
fn create_script(
    expected_path: std::path::PathBuf,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        // Every step re-presses whatever FireHeld is holding down; see
        // hold_fire_inputs for why this must be the plugin hook and not a
        // system.
        .input(hold_fire_inputs)
        .step("world_resume: reach the main menu")
        .until(ui_node_present(NEW_GAME_BUTTON))
        .deadline(SESSION_SECS)
        .add()
        .click_named(
            "world_resume: click New Game",
            NEW_GAME_BUTTON,
            ui_node_present(CREATE_WORLD_BUTTON),
            BEAT_DEADLINE_SECS,
        )
        .step("world_resume: name and seed the world")
        .on_enter(|world: &mut World| {
            set_field_text(world, WORLD_NAME_FIELD, WORLD_NAME);
            set_field_text(world, SEED_FIELD, WORLD_SEED);
        })
        .until(frames(2))
        .add()
        .click_named(
            "world_resume: click Create",
            CREATE_WORLD_BUTTON,
            state_is(GameStates::Playing),
            SESSION_SECS,
        )
        .step("world_resume: the world arms around the player")
        .until(armed_around_a_player())
        .deadline(SESSION_SECS)
        .add()
        .step("world_resume: inject the fixture state")
        .on_enter(inject_fixture)
        .add()
        .step("world_resume: cross a sector boundary")
        .on_enter(cross_sector_edge)
        .until(saved_past_first_write())
        .deadline(CROSSING_DEADLINE_SECS)
        .add()
        // The crossing streams a full window under the Loading screen; the
        // status line is only on screen once that comes down.
        .step("world_resume: wait for the Loading screen to come down")
        .until(nova_autopilot::predicate::and(
            nova_autopilot::predicate::not(ui_node_present(LOAD_SCREEN)),
            ui_node_present(SAVE_STATUS_LINE),
        ))
        .deadline(SESSION_SECS)
        .add()
        .step("world_resume: shoot the save status line")
        .on_enter(|world: &mut World| shoot(world, "world_resume-status-line.png"))
        .until(shot_written("world_resume-status-line.png"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("world_resume: assert the saved header on disk")
        .on_enter(assert_probe_world_on_disk)
        .add()
        // Fired HERE, LIVE, before the pause menu ever opens: the pause
        // overlay holds `FreezeOwner::PauseMenu`
        // (crates/nova_gameplay/src/freeze.rs), which pauses `Time<Virtual>`
        // and `Time<Physics>` outright - nothing ticks behind it, not a
        // muzzle cooldown, not a stow deploy, not a TempEntity lifetime. A
        // turret cannot even finish deploying there, let alone fire. Once
        // the pause menu opens, the hold stays engaged EXCEPT for a brief
        // release during each leave-save attempt's own settle window:
        // `drive_pending_leave` (crates/nova_menu/src/leave.rs) releases
        // `FreezeOwner::PauseMenu` while the session is still writing, then
        // re-holds it once that attempt resolves (saved or failed). So the
        // three transients' pose and remaining lifetime are locked while
        // idle, but each write - the failed read-only attempt and the
        // eventual successful retry - ticks a few settle frames of real
        // decay right before it lands.
        //
        // The torpedo order and the piece-killing damage fire first, ahead
        // of the PDC round: both are confirmed the moment they are
        // triggered, with no stowed-to-deployed travel to cross first, so
        // firing them before the round shrinks the round's own exposure -
        // the fewer frames queued between "the round exists" and the leave
        // click, the less of its tight 2.0s lifetime the rest of this
        // sequence spends.
        .step("world_resume: launch the torpedo order and kill a bay for its piece")
        .on_enter(fire_transient_fixture)
        .until(and(
            any_entity::<With<TorpedoProjectileMarker>>(),
            any_entity::<With<DetachedPieceMarker>>(),
        ))
        .deadline(PDC_FIRE_DEADLINE_SECS)
        .add()
        // combat_stance is already arming (the previous step), so WeaponsHot
        // already reads true on this frame - safe to press the PDC trigger
        // straight away; see hold_fire_inputs for why the two cannot press
        // on the same frame to begin with. A one-frame press is not enough:
        // `autopilot_drive` is only `.after(InputSystems)`
        // (crates/nova_autopilot/src/autopilot.rs:531), with no edge against
        // `bevy_enhanced_input`'s PreUpdate evaluation of the turret's
        // `Action<TurretInput>`, so the next step can release the press
        // before the action reads it. This step waits for
        // `TurretSectionInput` to read true on a player PDC bay, the
        // production state that proves the press latched
        // (crates/nova_ship/src/input/player/weapons.rs:183-218), before the
        // next step releases the trigger and waits for the round.
        .step("world_resume: fire a PDC round")
        .on_enter(|world: &mut World| {
            world.resource_mut::<FireHeld>().pdc = true;
        })
        .until(std::sync::Arc::new(|world: &World| {
            let Some(player) = the_player(world) else {
                return false;
            };
            let Some(mut turrets) = world
                .try_query_filtered::<(&TurretSectionInput, &ChildOf), With<TurretSectionMarker>>()
            else {
                return false;
            };
            turrets
                .iter(world)
                .any(|(input, child_of)| child_of.parent() == player && input.0)
        }))
        .deadline(PDC_FIRE_DEADLINE_SECS)
        .add()
        // Release on entry, then wait: the previous step's own `.until()`
        // only proves the press latched (`TurretSectionInput` read true),
        // not that a round yet exists - the turret's own fire cadence can
        // still take a frame or two past that, so this step waits again,
        // for the round itself. The screenshot command still captures this
        // exact live, chrome-free frame once the round is confirmed; the
        // write is confirmed by the next step's own `.until()` once the
        // pause menu is up, which the engine reaches on its own schedule.
        .step("world_resume: release the trigger and shoot the frame right before the leave")
        .on_enter(|world: &mut World| {
            {
                let mut held = world.resource_mut::<FireHeld>();
                held.combat = false;
                held.pdc = false;
            }
            world
                .resource_mut::<ButtonInput<MouseButton>>()
                .release(MouseButton::Left);
            drive_action(world, "combat_stance", InputPhase::Release);
            shoot(world, "world_resume-before-leave.png");
        })
        .until(any_entity::<With<TurretBulletProjectileMarker>>())
        .deadline(PDC_FIRE_DEADLINE_SECS)
        .add()
        .step("world_resume: open the pause menu")
        .on_enter(press_key(KeyCode::Escape))
        .until(and(
            ui_node_present(PAUSE_EXIT_BUTTON),
            shot_written("world_resume-before-leave.png"),
        ))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("world_resume: let escape go")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        .step("world_resume: record the expected state")
        .on_enter(write_expected_state(expected_path.clone()))
        .add()
        // A sandboxed save lands in a frame or two, faster than a screenshot
        // reaches the disk. A read-only world folder fails the leave save, and
        // the failed overlay stays until the player answers it.
        .step("world_resume: make the world folder read-only")
        .on_enter(|_: &mut World| set_world_folder_read_only(true))
        .add()
        // Back to Main Menu, not Exit: the load phase covers Exit.
        .click_named(
            "world_resume: click Back to Main Menu",
            BACK_TO_MENU_BUTTON,
            ui_node_present(LEAVE_TRY_AGAIN_BUTTON),
            LEAVE_DEADLINE_SECS,
        )
        .step("world_resume: shoot the failed leave overlay")
        .on_enter(|world: &mut World| shoot(world, "world_resume-leave-overlay.png"))
        .until(shot_written("world_resume-leave-overlay.png"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("world_resume: the failed leave kept the last good save")
        .on_enter(assert_probe_world_on_disk)
        .add()
        .step("world_resume: make the world folder writable")
        .on_enter(|_: &mut World| set_world_folder_read_only(false))
        .add()
        .click_named(
            "world_resume: click Try again",
            LEAVE_TRY_AGAIN_BUTTON,
            state_is(GameStates::MainMenu),
            LEAVE_DEADLINE_SECS,
        )
        .step("world_resume: record the leave save's own transients")
        .on_enter(move |world: &mut World| {
            let recorded = record_transient_fixture_state(world);
            let text = std::fs::read_to_string(&expected_path).unwrap_or_else(|e| {
                panic!("world_resume: cannot read {}: {e}", expected_path.display())
            });
            let mut expected: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| {
                panic!("world_resume: {} is not JSON: {e}", expected_path.display())
            });

            // Log, never assert, what the leave save's own settle window
            // changed since the live pre-click sample `expected["player"]`/
            // `expected["stock"]` was written - e.g. an idle reload drawing
            // down PdcRound while drive_pending_leave
            // (crates/nova_menu/src/leave.rs:100-105) released
            // FreezeOwner::PauseMenu for that write. The live sample stays in
            // `expected.json` for exactly this log; assert_resumed_state
            // checks `saved_player` below instead.
            let saved_player = recorded["saved_player"].clone();
            if let (Some(live_stock), Some(saved_stock)) = (
                expected["stock"].as_array(),
                saved_player["stock"].as_array(),
            ) {
                for live in live_stock {
                    let item = live["item"]
                        .as_str()
                        .expect("a live stock entry carries an item name");
                    let live_count = live["count"]
                        .as_u64()
                        .expect("a live stock entry carries a count");
                    let saved_count = saved_stock
                        .iter()
                        .find(|entry| entry["item"] == live["item"])
                        .and_then(|entry| entry["count"].as_u64())
                        .unwrap_or(0);
                    if live_count != saved_count {
                        info!(
                            "world_resume: live {item} {live_count} -> saved {saved_count} \
                             (settled during the leave save's own write)"
                        );
                    }
                }
            }
            let live_credits = expected["player"]["credits"].as_u64();
            let saved_credits = saved_player["credits"].as_u64();
            if live_credits != saved_credits {
                info!(
                    "world_resume: live credits {live_credits:?} -> saved {saved_credits:?} \
                     (settled during the leave save's own write)"
                );
            }

            expected["transients"] = recorded["transients"].clone();
            expected["saved_player"] = saved_player;
            std::fs::write(
                &expected_path,
                serde_json::to_string_pretty(&expected).expect("expected state must encode"),
            )
            .unwrap_or_else(|e| {
                panic!(
                    "world_resume: cannot write {}: {e}",
                    expected_path.display()
                )
            });
            info!(
                "world_resume: merged the leave save's own transients and saved_player into {}",
                expected_path.display()
            );
        })
        .add()
}

/// Make the sandboxed world folder read-only, or writable again. A
/// read-only folder takes no temp file, so the atomic write fails before it
/// touches the last good save.
#[cfg(feature = "debug")]
fn set_world_folder_read_only(read_only: bool) {
    let folder = nova_assets::storage::worlds_root()
        .expect("world_resume: the sandboxed CONFIG_ROOT must give a worlds root")
        .join(WORLD_SLUG);
    let mut permissions = std::fs::metadata(&folder)
        .unwrap_or_else(|e| panic!("world_resume: cannot read {}: {e}", folder.display()))
        .permissions();
    permissions.set_readonly(read_only);
    std::fs::set_permissions(&folder, permissions)
        .unwrap_or_else(|e| panic!("world_resume: cannot set {}: {e}", folder.display()));
}

/// Boot, run the create phase's own app, then verify the generation it wrote
/// from a plain disk read, spawn the load phase, and wait for it.
#[cfg(feature = "debug")]
fn run_create() -> bevy::app::AppExit {
    let sandbox = sandbox_root();
    std::env::set_var(nova_assets::storage::CONFIG_ROOT_ENV, &sandbox);
    let broken_world_dir = sandbox.join("worlds").join(BROKEN_WORLD_SLUG);
    std::fs::create_dir_all(&broken_world_dir).unwrap_or_else(|e| {
        panic!(
            "world_resume: cannot make the broken world fixture {}: {e}",
            broken_world_dir.display()
        )
    });
    let expected_path = sandbox.join("expected.json");

    let mut app = editor_app(true, None);
    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
    }
    app.init_resource::<Fixture>();
    app.init_resource::<FireHeld>();
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.add_plugins(create_script(expected_path.clone()));

    let exit = app.run();
    if !matches!(exit, bevy::app::AppExit::Success) {
        panic!(
            "world_resume: the create phase exited with {exit:?}; sandbox kept at {}",
            sandbox.display()
        );
    }
    drop(app);

    info!("world_resume: spawning the load phase");
    let status = std::process::Command::new(
        std::env::current_exe().expect("world_resume: cannot read this process's own exe path"),
    )
    .arg("--phase")
    .arg("load")
    .status()
    .unwrap_or_else(|e| panic!("world_resume: cannot spawn the load phase: {e}"));
    if !status.success() {
        panic!(
            "world_resume: the load phase failed ({status}); sandbox kept at {}",
            sandbox.display()
        );
    }

    std::fs::remove_dir_all(&sandbox).unwrap_or_else(|e| {
        warn!(
            "world_resume: both phases passed but the sandbox at {} would not clean up: {e}",
            sandbox.display()
        );
    });
    bevy::app::AppExit::Success
}

/// Read `expected.json` beside the sandboxed worlds root.
#[cfg(feature = "debug")]
fn read_expected_state() -> serde_json::Value {
    let root = nova_assets::storage::worlds_root()
        .expect("world_resume: the sandboxed CONFIG_ROOT must give a worlds root");
    let path = root
        .parent()
        .expect("the worlds root has a sandbox parent")
        .join("expected.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("world_resume: cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| {
        panic!(
            "world_resume: {} is not the expected JSON: {e}",
            path.display()
        )
    })
}

/// The load phase's assertion that the resumed state matches `expected`.
#[cfg(feature = "debug")]
fn assert_resumed_state(
    expected: serde_json::Value,
) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let watch = world.resource::<LoadWatch>();
        assert!(
            watch.saw_resumed_world,
            "world_resume load: ResumedWorld was never seen; the Load click did not seed it \
             before the world armed"
        );
        assert!(
            !world.contains_resource::<ResumedWorld>(),
            "world_resume load: ResumedWorld is still here; restore_resumed_world never consumed \
             it"
        );
        info!("world_resume load: the resumed ledger seam ran");

        // The player pose, credits and stock are checked earlier, in the
        // "match and assert the resumed transients" step, against the
        // thaw-time snapshot (while `FreezeOwner::WorldResume` still held
        // the clocks) - not read here, live, which would check the same
        // claim twice against state the idle reload or physics may have
        // already moved.

        let rock_id = expected["rock"]["id"].as_str().unwrap();
        let mut rocks = world.query_filtered::<(&EntityId, &BodyRadius), With<AsteroidMarker>>();
        let (_, rock_radius) = rocks
            .iter(world)
            .find(|(id, _)| id.0 == rock_id)
            .unwrap_or_else(|| panic!("world_resume load: no resumed asteroid named '{rock_id}'"));
        assert!(
            (rock_radius.0 - expected["rock"]["radius"].as_f64().unwrap() as f32).abs() < 0.01,
            "world_resume load: the resumed rock's BodyRadius {} does not match the saved {}",
            rock_radius.0,
            expected["rock"]["radius"]
        );

        let expected_canister_id = expected["canister"]["id"].as_u64().unwrap();
        let mut canisters = world.query::<(&CargoCanisterRuntimeId, &CargoCanister)>();
        let (_, canister) = canisters
            .iter(world)
            .find(|(id, _)| id.0 == expected_canister_id)
            .unwrap_or_else(|| {
                panic!("world_resume load: no resumed canister with id {expected_canister_id}")
            });
        let contents: Vec<_> = canister
            .stacks()
            .map(|(item, count)| serde_json::json!({ "item": format!("{item:?}"), "count": count }))
            .collect();
        assert_eq!(
            serde_json::Value::Array(contents),
            expected["canister"]["contents"],
            "world_resume load: the resumed canister's contents do not match the saved ones"
        );

        let sector = world.resource::<CurrentSector>().0;
        let expected_sector = expected["sector"].as_array().unwrap();
        assert_eq!(
            [sector.x, sector.y, sector.z],
            [
                expected_sector[0].as_i64().unwrap() as i32,
                expected_sector[1].as_i64().unwrap() as i32,
                expected_sector[2].as_i64().unwrap() as i32,
            ],
            "world_resume load: the resumed sector does not match the saved one"
        );

        // The three fixture transient KINDS are matched and asserted in an
        // earlier step (right after `fixture_transients_restored()`, before
        // the screenshot and the pixel-diff) - see `load_script`. By the
        // time this runs, that step has already panicked if any record
        // failed to pair one-to-one with a live body.

        info!("world_resume load: PASS every fixture value matched");
        nova_probe::probe_marker(
            world,
            "outcome: the load phase resumes every piece of the saved fixture state",
            expected.clone(),
        );
    }
}

/// The broken world's Load button, in the details pane, carries
/// `InteractionDisabled` while it is selected.
#[cfg(feature = "debug")]
fn assert_broken_world_disabled(world: &mut World) {
    let mut buttons = world.query::<(&Name, Has<bevy::ui::InteractionDisabled>)>();
    let (_, disabled) = buttons
        .iter(world)
        .find(|(name, _)| name.as_str() == LOAD_WORLD_BUTTON)
        .expect("world_resume load: the Load World Button exists");
    assert!(
        disabled,
        "world_resume load: the broken world's Load button is not disabled"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a world folder with no world.ron lists refused and disabled",
        serde_json::json!({ "slug": BROKEN_WORLD_SLUG }),
    );
    info!("world_resume load: the broken world is refused up front");
}

#[cfg(feature = "debug")]
fn assert_probe_world_enabled(world: &mut World) {
    let mut buttons = world.query::<(&Name, Has<bevy::ui::InteractionDisabled>)>();
    let (_, disabled) = buttons
        .iter(world)
        .find(|(name, _)| name.as_str() == LOAD_WORLD_BUTTON)
        .expect("world_resume load: the Load World Button exists");
    assert!(
        !disabled,
        "world_resume load: Probe World's Load button is still disabled"
    );
    info!("world_resume load: Probe World's Load button is enabled");
}

/// The load phase's walk: Load through the leave save.
#[cfg(feature = "debug")]
fn load_script(
    expected: serde_json::Value,
    recorded: std::sync::Arc<
        std::sync::Mutex<Vec<(Entity, &'static str, Option<String>, Vec3, f32)>>,
    >,
    post_resume_inserts: std::sync::Arc<std::sync::Mutex<u32>>,
    player_snapshot: std::sync::Arc<std::sync::Mutex<Option<(Vec3, u32, Vec<(ItemType, u32)>)>>>,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("world_resume load: reach the main menu")
        .until(ui_node_present(NEW_GAME_BUTTON))
        .deadline(SESSION_SECS)
        .add()
        .click_named(
            "world_resume load: click Load",
            LOAD_BUTTON,
            and(
                ui_node_present(format!("Load World Row: {WORLD_SLUG}")),
                ui_node_present(format!("Load World Row: {BROKEN_WORLD_SLUG}")),
            ),
            BEAT_DEADLINE_SECS,
        )
        .step("world_resume load: the broken world is listed and refused")
        .on_enter(assert_broken_world_disabled)
        .add()
        .step("world_resume load: shoot the Load screen")
        .on_enter(|world: &mut World| shoot(world, "world_resume-load-screen.png"))
        .until(shot_written("world_resume-load-screen.png"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .click_named(
            "world_resume load: select Probe World",
            &format!("Load World Row: {WORLD_SLUG}"),
            frames(2),
            BEAT_DEADLINE_SECS,
        )
        .step("world_resume load: the probe world's Load button enables")
        .on_enter(assert_probe_world_enabled)
        .add()
        .click_named(
            "world_resume load: click Load World",
            LOAD_WORLD_BUTTON,
            state_is(GameStates::Playing),
            SESSION_SECS,
        )
        .step("world_resume load: the world arms with the thawed player")
        .until(armed_around_a_player())
        .deadline(SESSION_SECS)
        .add()
        .step("world_resume load: wait for the fixture rock to stream back in")
        .until(fixture_rock_streamed_in(
            expected["rock"]["id"]
                .as_str()
                .expect("expected.rock.id is a string")
                .to_string(),
        ))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("world_resume load: wait for the fixture transients to resume")
        .until(fixture_transients_restored())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // Matches and asserts every saved round, torpedo and piece BEFORE
        // the screenshot and the pixel-diff: those two cost real lavapipe
        // time (a disk write, a decode, a full-image diff) that would
        // otherwise tick the tightest transient (the round's 2.0s lifetime)
        // further down before this match ever runs.
        .step("world_resume load: match and assert the resumed transients")
        .on_enter({
            let expected_transients = expected["transients"].clone();
            let saved_player = expected["saved_player"].clone();
            let recorded = std::sync::Arc::clone(&recorded);
            let post_resume_inserts = std::sync::Arc::clone(&post_resume_inserts);
            let player_snapshot = std::sync::Arc::clone(&player_snapshot);
            move |world: &mut World| {
                // Ordering evidence for every check below: `spawn_resumed`
                // (crates/nova_world_base/src/save/transients.rs:707-736)
                // queues each saved transient's spawn, then its
                // `resumed_lifetime` insert, in saved-list order, all on the
                // ONE Commands buffer `state.apply(world)` applies once
                // (transients.rs:753) - so the thaw-time observer's Nth
                // `Insert<TempEntityState>` is always the Nth saved
                // transient, each insert landing while `WorldResumeProgress`
                // is still present. This holds for the kinds this fixture
                // saves; a rock chunk thaw inserts `TempEntity` in its own
                // spawn bundle and then gets `resumed_lifetime`, so it would
                // be seen twice and fail the duplicate check. The values compared
                // below are the ones that observer captured at the instant
                // of that insert, not a live read on this later frame.
                let saved = expected_transients
                    .as_array()
                    .expect("world_resume load: expected.transients is an array");
                let recorded_entries = recorded
                    .lock()
                    .expect("world_resume load: the thaw-time recorder is never held across a panic")
                    .clone();

                // Fail loudly on a missing or an extra callback: the thaw
                // loop and the observer must agree on how many inserts
                // happened.
                assert_eq!(
                    recorded_entries.len(),
                    saved.len(),
                    "world_resume load: thaw-time recorded {} transient(s), the leave save \
                     holds {}",
                    recorded_entries.len(),
                    saved.len()
                );

                // Fail loudly on a duplicate entity: one entity recorded
                // twice would mean the observer saw two Insert<TempEntityState>
                // for the same body, which the index pairing above cannot
                // tell apart from a missing one.
                let mut seen_entities = std::collections::HashSet::new();
                for (index, (entity, ..)) in recorded_entries.iter().enumerate() {
                    assert!(
                        seen_entities.insert(*entity),
                        "world_resume load: thaw-time entity {entity} was recorded twice; first \
                         seen again at record index {index}"
                    );
                }

                // Logged, not asserted: TempEntity (and so TempEntityState)
                // is a shared, game-wide transient-lifetime marker - live
                // gameplay spawns its own after the resume window too (a
                // light flash off a torpedo ignition, an NPC round), and
                // those are not a resume defect. A genuinely late RESUMED
                // insert is already caught above: it would make
                // `recorded_entries.len()` fall short of `saved.len()`.
                let post_resume_inserts = *post_resume_inserts
                    .lock()
                    .expect("world_resume load: the post-resume counter is never held across a panic");
                info!(
                    "world_resume load: TempEntityState insert(s) after the resume window: \
                     {post_resume_inserts}"
                );

                // Compared against the thaw-time snapshot taken by the
                // same observer, while `FreezeOwner::WorldResume` still held
                // the clocks (see the comment beside `player_snapshot` in
                // `run_load`) - not a live read on this later frame, which
                // would let an idle reload or a physics step drift the
                // values before this check ever ran.
                let (snapshot_pose, snapshot_credits, mut snapshot_stock) = player_snapshot
                    .lock()
                    .expect("world_resume load: the player snapshot is never held across a panic")
                    .clone()
                    .expect(
                        "world_resume load: the thaw-time observer never matched exactly one \
                         player ship",
                    );
                let saved_pose = saved_player["pose"]
                    .as_array()
                    .expect("expected.saved_player.pose is an array");
                let saved_pose = Vec3::new(
                    saved_pose[0].as_f64().unwrap() as f32,
                    saved_pose[1].as_f64().unwrap() as f32,
                    saved_pose[2].as_f64().unwrap() as f32,
                );
                let pose_delta = snapshot_pose.distance(saved_pose);
                assert!(
                    pose_delta < 1.0,
                    "world_resume load: thaw-time player pose delta {pose_delta:.4}m exceeds \
                     1.0m: saved {saved_pose:?}, thaw-time {snapshot_pose:?}"
                );
                let saved_credits = saved_player["credits"]
                    .as_u64()
                    .expect("expected.saved_player.credits is a number") as u32;
                assert_eq!(
                    snapshot_credits, saved_credits,
                    "world_resume load: thaw-time player credits {snapshot_credits} does not \
                     match the saved credits {saved_credits}"
                );
                let mut saved_stock: Vec<(String, u32)> = saved_player["stock"]
                    .as_array()
                    .expect("expected.saved_player.stock is an array")
                    .iter()
                    .map(|entry| {
                        (
                            entry["item"]
                                .as_str()
                                .expect("a saved stock entry carries an item name")
                                .to_string(),
                            entry["count"].as_u64().expect("a saved stock entry carries a count")
                                as u32,
                        )
                    })
                    .collect();
                saved_stock.sort_by(|a, b| a.0.cmp(&b.0));
                snapshot_stock.sort_by(|a, b| format!("{:?}", a.0).cmp(&format!("{:?}", b.0)));
                let snapshot_stock: Vec<(String, u32)> = snapshot_stock
                    .into_iter()
                    .map(|(item, count)| (format!("{item:?}"), count))
                    .collect();
                assert_eq!(
                    snapshot_stock, saved_stock,
                    "world_resume load: thaw-time player stock does not match the saved stock"
                );

                let mut max_deltas: std::collections::HashMap<&str, (f32, f32)> =
                    std::collections::HashMap::new();
                for (index, (saved_entry, (_entity, recorded_kind, recorded_owner, recorded_pose, recorded_remaining))) in
                    saved.iter().zip(recorded_entries.into_iter()).enumerate()
                {
                    let saved_kind = saved_entry["kind"]
                        .as_str()
                        .expect("a saved transient record carries a kind tag");
                    assert_eq!(
                        recorded_kind, saved_kind,
                        "world_resume load: record {index}'s thaw-time kind is \
                         '{recorded_kind}', the saved kind is '{saved_kind}'"
                    );
                    if saved_kind == "other" {
                        continue;
                    }
                    let saved_owner = saved_entry["owner"].as_str().map(str::to_string);
                    assert_eq!(
                        recorded_owner, saved_owner,
                        "world_resume load: record {index} ({saved_kind}) thaw-time owner \
                         {recorded_owner:?} does not match the saved owner {saved_owner:?}"
                    );
                    let saved_pose = saved_entry["pose"]
                        .as_array()
                        .expect("a saved round/torpedo/piece record carries a pose");
                    let saved_pose = Vec3::new(
                        saved_pose[0].as_f64().unwrap() as f32,
                        saved_pose[1].as_f64().unwrap() as f32,
                        saved_pose[2].as_f64().unwrap() as f32,
                    );
                    let saved_remaining = saved_entry["lifetime_remaining"].as_f64().unwrap() as f32;
                    let pose_delta = recorded_pose.distance(saved_pose);
                    let lifetime_delta = (recorded_remaining - saved_remaining).abs();
                    assert!(
                        pose_delta < 1.0,
                        "world_resume load: record {index} ({saved_kind}) translation delta \
                         {pose_delta:.4}m exceeds 1.0m: saved {saved_pose:?}, thaw-time \
                         {recorded_pose:?}"
                    );
                    assert!(
                        lifetime_delta < 0.01,
                        "world_resume load: record {index} ({saved_kind}) lifetime delta \
                         {lifetime_delta:.4}s exceeds 0.01s: saved {saved_remaining:.4}s, \
                         thaw-time {recorded_remaining:.4}s"
                    );
                    let entry = max_deltas.entry(saved_kind).or_insert((0.0_f32, 0.0_f32));
                    entry.0 = entry.0.max(pose_delta);
                    entry.1 = entry.1.max(lifetime_delta);
                }

                // A live body can have already expired by this on_enter
                // (the restore-to-here gap); it can never exceed what the
                // leave save actually holds for that kind.
                let saved_count = |kind: &str| saved.iter().filter(|v| v["kind"] == kind).count();
                let assert_live_at_most_saved = |kind: &str, live: usize, saved: usize| {
                    assert!(
                        live <= saved,
                        "world_resume load: {kind} has {live} live body(ies) at this on_enter, \
                         more than the {saved} the leave save holds"
                    );
                };
                assert_live_at_most_saved(
                    "Round",
                    world
                        .query_filtered::<Entity, With<TurretBulletProjectileMarker>>()
                        .iter(world)
                        .count(),
                    saved_count("round"),
                );
                assert_live_at_most_saved(
                    "Torpedo",
                    world
                        .query_filtered::<Entity, With<TorpedoProjectileMarker>>()
                        .iter(world)
                        .count(),
                    saved_count("torpedo"),
                );
                assert_live_at_most_saved(
                    "DetachedPiece",
                    world
                        .query_filtered::<Entity, With<DetachedPieceMarker>>()
                        .iter(world)
                        .count(),
                    saved_count("piece"),
                );

                let (round_max_t, round_max_l) = max_deltas.get("round").copied().unwrap_or_default();
                let (torpedo_max_t, torpedo_max_l) =
                    max_deltas.get("torpedo").copied().unwrap_or_default();
                let (piece_max_t, piece_max_l) = max_deltas.get("piece").copied().unwrap_or_default();

                info!(
                    "world_resume load: matched {} transient(s) by stable save-list index \
                     (recorded == saved per kind: round {}, torpedo {}, piece {}; duplicates 0; \
                     post-resume inserts {post_resume_inserts}) - round max pose delta \
                     {round_max_t:.4}m/lifetime delta {round_max_l:.4}s, torpedo max pose delta \
                     {torpedo_max_t:.4}m/lifetime delta {torpedo_max_l:.4}s, piece max pose \
                     delta {piece_max_t:.4}m/lifetime delta {piece_max_l:.4}s - thaw-time \
                     player pose delta {pose_delta:.4}m, credits saved {saved_credits} == \
                     thaw-time {snapshot_credits}, stock saved == thaw-time for all {} item(s)",
                    saved.len(),
                    saved_count("round"),
                    saved_count("torpedo"),
                    saved_count("piece"),
                    saved_stock.len(),
                );
                nova_probe::probe_marker(
                    world,
                    "outcome: every saved transient pairs with the thaw-time entry at the same \
                     save-list index, agreeing on kind and, for a round, a torpedo or a \
                     detached piece, owner, pose and remaining lifetime; the observer saw no \
                     duplicate entity (post-resume TempEntityState inserts, if any, are \
                     unrelated live gameplay, logged not asserted)",
                    serde_json::json!({
                        "count": saved.len(),
                        "round": { "count": saved_count("round"), "max_pose_delta": round_max_t, "max_lifetime_delta": round_max_l },
                        "torpedo": { "count": saved_count("torpedo"), "max_pose_delta": torpedo_max_t, "max_lifetime_delta": torpedo_max_l },
                        "piece": { "count": saved_count("piece"), "max_pose_delta": piece_max_t, "max_lifetime_delta": piece_max_l },
                        "duplicates": 0,
                        "post_resume_inserts": post_resume_inserts,
                        "p6_player": {
                            "pose_delta": pose_delta,
                            "credits_match": snapshot_credits == saved_credits,
                            "stock_match": snapshot_stock == saved_stock,
                        },
                    }),
                );
            }
        })
        .add()
        .step("world_resume load: shoot the first restored frame")
        .on_enter(|world: &mut World| shoot(world, "world_resume-after-load.png"))
        .until(shot_written("world_resume-after-load.png"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("world_resume load: report the pixel difference across the leave")
        .on_enter(|world: &mut World| {
            report_pixel_difference(
                world,
                "world_resume-before-leave.png",
                "world_resume-after-load.png",
            )
        })
        .add()
        .step("world_resume load: check the resumed state against the save")
        .on_enter(assert_resumed_state(expected))
        .add()
        .step("world_resume load: the fixture transients expire on their own saved clocks")
        .until(fixture_transients_expired())
        .deadline(TRANSIENT_EXPIRE_DEADLINE_SECS)
        .add()
        .step("world_resume load: open the pause menu")
        .on_enter(press_key(KeyCode::Escape))
        .until(ui_node_present(PAUSE_EXIT_BUTTON))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("world_resume load: let escape go")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        .click_named(
            "world_resume load: click Pause Exit",
            PAUSE_EXIT_BUTTON,
            ui_node_present(LEAVE_OVERLAY),
            BEAT_DEADLINE_SECS,
        )
        .step("world_resume load: wait for the leave save to land")
        .until(std::sync::Arc::new(|_: &World| false))
        .deadline(LEAVE_DEADLINE_SECS)
        .add()
}

/// Boot this fresh process's own app and run the load phase's walk. Never
/// creates a world: it reads the sandboxed `NOVA_CONFIG_ROOT` the create
/// phase already set in this process's inherited environment.
#[cfg(feature = "debug")]
fn run_load() -> bevy::app::AppExit {
    assert!(
        std::env::var_os(nova_assets::storage::CONFIG_ROOT_ENV).is_some(),
        "world_resume load: {} is not set; the load phase must inherit it from the create \
         phase, never set its own",
        nova_assets::storage::CONFIG_ROOT_ENV
    );
    let expected = read_expected_state();

    let mut app = editor_app(true, None);
    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
    }
    app.init_resource::<LoadWatch>();
    app.add_systems(Update, watch_resumed_world);

    // Thaw-time capture: `spawn_resumed` (crates/nova_world_base/src/save/
    // transients.rs:707-736) queues each saved transient's spawn, then its
    // `resumed_lifetime` insert, in saved-list order, all on the ONE
    // Commands buffer `state.apply(world)` applies once (transients.rs:753)
    // - so for a round, a torpedo, a piece or a shed fixture the Nth
    // `Insert<TempEntityState>` this observer sees is the Nth saved
    // transient. A rock chunk thaw also inserts `TempEntity` in its own
    // spawn bundle, so it is seen twice; this fixture saves none. Gated on
    // `WorldResumeProgress`, which stays until `end_resume` runs right
    // after that same `state.apply` (transients.rs:768-774) -
    // `ResumedTransients` itself is pub(crate), so this is the public seam
    // this file can gate on. Every resumed kind is recorded, tagged "other"
    // when it carries none of the three checked markers (a rock chunk or a
    // shed fixture), so the recorded list's length and order always mirror
    // the saved list's. An insert seen with `WorldResumeProgress` already
    // gone is NEVER recorded - only counted, in `post_resume_inserts`, once
    // a Load has already produced at least one recorded entry (so a
    // process that has not yet Loaded does not count as "after removal").
    let recorded: std::sync::Arc<
        std::sync::Mutex<Vec<(Entity, &'static str, Option<String>, Vec3, f32)>>,
    > = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let post_resume_inserts: std::sync::Arc<std::sync::Mutex<u32>> =
        std::sync::Arc::new(std::sync::Mutex::new(0));
    // The player's pose, credits and stock, snapshotted the first time
    // this observer fires after a Load (any insert, not only one of the
    // three checked kinds) rather than read live later. `FreezeOwner::
    // WorldResume` holds `Time<Virtual>`/`Time<Physics>` until `end_resume`
    // (transients.rs:768-774), which runs after this whole `spawn_resumed`
    // call - so nothing has ticked on the player yet: no reload has drawn
    // on `ShipInventory`, no physics step has moved it. A later live read
    // would race the PDC idle reload (crates/nova_ship/src/sections/ammo.rs),
    // which refills a fired magazine from `ShipInventory`. Pose
    // reads `Transform`, not `Position`: avian only writes `Position` from
    // `Transform` in `FixedPostUpdate`, which does not run while
    // `WorldResume` holds the clocks, so `Position` would still be the
    // player's pre-thaw default here - `Transform` is also what the save
    // itself records (`SavedPlayer.transform`).
    let player_snapshot: std::sync::Arc<
        std::sync::Mutex<Option<(Vec3, u32, Vec<(ItemType, u32)>)>>,
    > = std::sync::Arc::new(std::sync::Mutex::new(None));
    {
        let recorded = std::sync::Arc::clone(&recorded);
        let post_resume_inserts = std::sync::Arc::clone(&post_resume_inserts);
        let player_snapshot = std::sync::Arc::clone(&player_snapshot);
        app.add_observer(
            move |insert: On<Insert, TempEntityState>,
                  progress: Option<Res<WorldResumeProgress>>,
                  bodies: Query<(
                &TempEntityState,
                Option<&Transform>,
                Option<&ProjectileOwner>,
                Has<TurretBulletProjectileMarker>,
                Has<TorpedoProjectileMarker>,
                Has<DetachedPieceMarker>,
            )>,
                  owners: Query<&EntityId>,
                  players: Query<
                (&Transform, &ShipCredits, &ShipInventory),
                With<PlayerSpaceshipMarker>,
            >| {
                if progress.is_none() {
                    let recorded = recorded.lock().expect(
                        "world_resume load: the thaw-time recorder is never held across a panic",
                    );
                    if !recorded.is_empty() {
                        *post_resume_inserts.lock().expect(
                            "world_resume load: the post-resume counter is never held across a \
                             panic",
                        ) += 1;
                        warn!(
                            "world_resume load: a TempEntityState insert on {} landed after \
                             WorldResumeProgress was removed; not recorded, only counted",
                            insert.entity
                        );
                    }
                    drop(recorded);
                    return;
                }
                {
                    let mut snapshot = player_snapshot.lock().expect(
                        "world_resume load: the player snapshot is never held across a panic",
                    );
                    if snapshot.is_none() {
                        let mut matched = players.iter();
                        if let Some((transform, credits, inventory)) = matched.next() {
                            assert!(
                                matched.next().is_none(),
                                "world_resume load: more than one player ship matched while \
                                 snapshotting thaw-time state, expected exactly 1"
                            );
                            *snapshot = Some((
                                transform.translation,
                                credits.0,
                                inventory.stacks().collect(),
                            ));
                        }
                    }
                }
                let Ok((state, transform, projectile_owner, is_round, is_torpedo, is_piece)) =
                    bodies.get(insert.entity)
                else {
                    return;
                };
                let kind = if is_round {
                    "round"
                } else if is_torpedo {
                    "torpedo"
                } else if is_piece {
                    "piece"
                } else {
                    "other"
                };
                let translation = transform
                    .unwrap_or_else(|| {
                        panic!(
                            "world_resume load: a resumed {kind} carries no Transform at thaw \
                             time"
                        )
                    })
                    .translation;
                // Mirrors `SavedLifetime::of`'s own math (the countdown's
                // own remaining_secs); read straight off the Query instead
                // of `&World`, since an observer cannot combine `&World`
                // with the Query/Res params this closure also needs.
                let remaining = state.remaining_secs();
                let owner = projectile_owner.and_then(|&ProjectileOwner(owner)| {
                    owners.get(owner).ok().map(|id| id.0.clone())
                });
                recorded
                    .lock()
                    .expect(
                        "world_resume load: the thaw-time recorder is never held across a panic",
                    )
                    .push((insert.entity, kind, owner, translation, remaining));
            },
        );
    }

    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.add_plugins(load_script(
        expected,
        recorded,
        post_resume_inserts,
        player_snapshot,
    ));

    app.run()
}
