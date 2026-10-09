# SW-PT9: Plan for the P-T9 two-process extension of `system_world_resume`

Scope: `examples/systems/system_world_resume.rs` only (plus this report). No
production edits proposed. Approved proof text: TRANSIENT-GATE.md "P-T9
two-process" bullet (section 10, near line 665); section 12 is current
decisions, none of which touch this example.

## 1. Getting each transient into the world before the leave

All three are fired through REAL production paths on the player's own New
Game ship (`BLOCK_LINE_WARSHIP_SHIP_ID`, the "Line Warship" the open-world
bootstrap spawns: `crates/nova_authoring/src/base_content/scenarios/open_world.rs:56-85`).
No DISCLOSED fixture state is needed for any of the three - real paths exist
for all of them.

### Round (turret)

The warship carries six PDC bays, each a real `turret_section`
(`PDC_KINETIC_TURRET_SECTION_ID`, `crates/nova_authoring/src/base_content/ships/block.rs:56-78,473-478,859-861`),
authored `ammunition: AmmoCapacity::Limited(500)`
(`crates/nova_authoring/src/base_content/sections/turret.rs:455`), so every
spawned PDC starts with a full `SectionAmmo` - no ammo setup needed
(`crates/nova_ship/src/sections/ammo.rs:17-19` "An installed magazine starts
full without drawing on the inventory").

A PDC only fires while `TurretStowPhase::Deployed`, and a managed ship's
deploy demand is `WeaponsHot` (`crates/nova_ship/src/sections/turret_section/stow.rs:161-178`).
`WeaponsHot` is derived every frame from `WeaponsRaised`
(`crates/nova_ship/src/input/targeting/safety.rs:10-31`), which the player
raises with the real, always-on `"combat_stance"` action - RMB by default
(`crates/nova_ship/src/input/bindings.rs:133-135`, part of
`camera_bindings()`'s fixed flight rig, reachable by every player ship, not
range-specific). The PDC's own fire trigger is the per-section
`input_mapping` bound to `InputSource::Mouse(MouseButton::Left)`
(`crates/nova_authoring/src/base_content/scenarios/open_world.rs:92-99`),
read directly off the device state by a `bevy_enhanced_input` context built
at spawn (`crates/nova_ship/src/input/bindings.rs:1-9` "built from content
`input_mapping` at spawn, keyed by section entity").

Precedent for driving both at once, frame-held, from an autopilot script:
`examples/systems/system_turret_gunnery.rs:753-783` (`HeldInput` +
`hold_inputs`, wired as `AutopilotPlugin::input`, NOT an `Update` system -
the doc there explains why: `PreUpdate` timing is required for the trigger's
`just_pressed` edge to survive to the weapon's `Update` read). My
`hold_inputs`-equivalent will call
`nova_debug::harness::drive_action(world, "combat_stance", InputPhase::Press)`
and press `ButtonInput<MouseButton>::press(MouseButton::Left)` directly
(same trick as that file's raw `KeyCode::Space` press), every frame while a
local flag is set.

**Risk, flagged up front:** the PDC's authored `projectile_lifetime` is only
**2.0 s** (`crates/nova_authoring/src/base_content/sections/turret.rs:445`).
This is the single tightest constraint in the whole plan - see section 3 and
the ordering note below.

### Torpedo

The warship carries two torpedo bays (`torpedo_port`, `torpedo_starboard`,
`crates/nova_authoring/src/base_content/ships/block.rs:69,462-470`),
authored `projectile_lifetime: 100.0` (`crates/nova_authoring/src/base_content/sections/torpedo_bay.rs:180`)
- no timing risk.

Fired with `ScriptedTorpedoOrder { target: Entity::PLACEHOLDER }`, inserted
directly on the bay entity. This is the named production mechanism (not a
fixture bypass): "an authored one-shot order that fires a bay and commits
the projectile with no controller involved (scenario-timer emplacements)"
(`crates/nova_ship/src/sections/torpedo_section/mod.rs:42-44`). The trigger
system holds the bay's `TorpedoSectionInput` until the real fire gates
(cooldown, ammo, inactive section) let it launch
(`crates/nova_ship/src/sections/torpedo_section/scripted.rs:26-37`). A
target that is not a live `SpaceshipRootMarker` commits as dumb-fire
(`scripted.rs:56-61`), so `Entity::PLACEHOLDER` gives a real `DumbFire`
torpedo with no extra setup.

### Detached piece

Trigger a real, one-hit kill of one of the OTHER PDC bays (not the one used
to fire the round) with `world.trigger(HealthApplyDamage { entity, source:
None, amount: health.max })` - the exact idiom
`examples/systems/system_section_severing.rs:255-259` already uses on a
bespoke rig, here aimed at a section of the REAL player ship. `amount ==
health.max` zeroes the section with no overkill, so nothing propagates up
`ChildOf` to the hull (`crates/nova_gameplay/src/integrity/health.rs:78-80`
"propagate only what landed") - the ship is untouched, satisfying "a player
section that does not end the run."

`detach_destroyed_body` is filtered `Without<IntegrityRoot>`
(`crates/nova_gameplay/src/integrity/explode.rs:312-356`); `IntegrityRoot`
marks only the hull root (`pyre.rs:1224`), never a turret section, so
killing a PDC bay detaches a real `DetachedPieceMarker` piece, not a
whole-ship death. Its `TempEntity(PIECE_LIFETIME_SECS)` is 30 s
(`explode.rs:86`) - ample margin.

Finding the two PDC entities and the torpedo bay entity: query the player's
children for `With<TurretSectionMarker>` and `With<TorpedoSectionMarker>`
(both public, re-exported from `nova_gameplay::prelude` and used exactly this
way by `crates/nova_ship/src/sections/signature.rs:23-26`), filtered by
`ChildOf(player)`. No new id lookup needed - same `world.query_filtered`
idiom `inject_fixture`/`nearest_asteroid` already use in this file.

## 2. Owner ids

The player ship root is spawned with `EntityId::new("player")` from the
scenario object's `base.id`
(`crates/nova_scenario/src/actions/spawn.rs:110,984`, `PLAYER_ID` in
`open_world.rs:29`), so `SavedOwner::of` resolves it to
`SavedOwner::Ship(EntityId("player"))` for both the round and the torpedo.
The torpedo's bay also needs an `EntityId` for `SavedSectionRef` - every
spawned section gets one (`nova_scenario/src/objects/spaceship.rs:913-916`,
already cited in TRANSIENT-GATE.md section 12). The piece carries no owner
field at all (`FrozenDetachedPiece` has none); its `DetachedPieceSource`
stamp is read straight off the dying section, not an id.

## 3. Capturing expected values, and surviving to the leave save

After the leave save lands, read the newest `state.N.ron` and extract each
transient. But given the round's 2.0 s TTL, I will NOT fire from the
existing "inject the fixture" step (which sits well before the sector
crossing, the crossing-save wait, and the loading-screen-down wait - too
much elapsed sim time, even before the first leave attempt). The existing
script also runs a full FAILED-leave round trip first (read-only folder,
click, failed overlay shot, assert, make writable, click Try again) before
the save that the load phase actually reads lands. A round fired any
earlier could not survive that detour.

Plan: insert the new "fire round + launch torpedo + kill piece" step
**between** "make the world folder writable" and "click Try again"
(`system_world_resume.rs:592-600` today) - i.e., immediately before the
successful leave click, with nothing else queued in between but a short
`until()` wait for all three markers to exist
(`With<TurretBulletProjectileMarker>>`, `With<TorpedoProjectileMarker>>`,
`With<DetachedPieceMarker>>`, each known to be unique in this scenario - no
other entity fires a turret round, launches a torpedo, or kills a section
anywhere else in this script). That keeps the gap between "round exists"
and "the leave save writes" to a handful of frames, not a whole UI round
trip.

Once all three exist, record (by component read, not RON) into
`expected.json`:
- round: pose = `Position`/`Rotation` (or `Transform`, matching what
  `record_expected_state` already reads for the player), owner = "player"
  (asserted, not re-derived), `SavedLifetime::of(world, round).remaining`
  (`nova_gameplay::prelude::SavedLifetime`, public, exactly the function the
  real freeze calls - `crates/nova_gameplay/src/lifetime.rs:84-92`);
- torpedo: same pose/velocity fields already on `Transform`/`LinearVelocity`,
  owner = "player", `SavedLifetime::of(world, torpedo).remaining`;
- piece: `Transform`, `SavedLifetime::of` does NOT apply (a piece in grace
  carries `ChunkGrace`, not `TempEntityState`, while grace lasts - confirm at
  implementation; if `TempEntity`/`TempEntityState` are not yet present on a
  still-kinematic piece, read `ChunkGrace::remaining()` instead
  (`crates/nova_gameplay/src/integrity/chunk.rs:186`) and record which of the
  two was read, so the load-phase assert reads the same one).

This is the one open item I'd confirm by running the create phase under
`NOVA_AUTOPILOT=1` before touching the load phase's assert: whether a
freshly-detached piece already carries `TempEntity`/`TempEntityState` (via
`detach_destroyed_body`'s own `TempEntity(PIECE_LIFETIME_SECS)` insert,
`explode.rs:430`) at the moment our predicate sees `DetachedPieceMarker`, or
only `ChunkGrace`. Both are plausible from the code; I would log both and
pick the one actually present rather than guess in this plan.

Shoot `"world_resume-before-leave.png"` right after this step (same `shoot`/
`shot_written` idiom the file already uses twice).

## 4. Finding each one on the load phase's first restored frame

New predicate, same shape as `fixture_rock_streamed_in`:

```rust
fn fixture_transients_restored() -> Arc<Predicate> {
    Arc::new(|world: &World| {
        world.query_filtered::<Entity, With<TurretBulletProjectileMarker>>().iter(world).next().is_some()
            && world.query_filtered::<Entity, With<TorpedoProjectileMarker>>().iter(world).next().is_some()
            && world.query_filtered::<Entity, With<DetachedPieceMarker>>().iter(world).next().is_some()
    })
}
```

Waited on after `armed_around_a_player()` and `fixture_rock_streamed_in`,
same position in the walk the existing rock-streaming wait already
occupies. The step that follows (`on_enter`) is "the first frame after the
Load" for the pixel capture and the extended `assert_resumed_state`.

Match tolerance: pose equality within float noise, the same
`distance(...) < 1.0` the player pose check already uses
(`system_world_resume.rs:721-723`) for translation, and a small epsilon
(e.g. `< 0.01`) for `SavedLifetime::of(..).remaining` / `ChunkGrace::remaining()`
equality to the saved value - exact equality is wrong because the clocks
are held but the comparison still crosses a RON float round-trip, same
reasoning `assert_resumed_state` already applies to the rock's `BodyRadius`
(`:754-759`).

## 5. "Then expires", without asserting timing

Each of the three carries its own authored despawn: the round and torpedo
via `TempEntityState`, the piece via `ChunkGrace` then `TempEntity` once
clear. None of these needs a new mechanism - add one more step after the
extended assert: `.until(not(exists(round)) && not(exists(torpedo)) &&
not(exists(piece)))` with a generous `.deadline(...)` (sized off the
WORST of the three's remaining lifetime at thaw time plus settle, never off
a wall-clock guess) - the same "bounded wait, named deadline, no raw
`sleep`" idiom every other step in this file already uses. The assertion is
that the wait predicate resolves before its deadline; the deadline watcher,
not an assertion on elapsed time, is what fails if one never despawns.

## 6. Frame capture and the pixel-difference figure

Capture idiom confirmed by grep (`crates/nova_autopilot/src/capture.rs`,
`crates/nova_debug/src/harness.rs:570-575`): `shoot(world, path)` ->
`capture_window` -> async `Screenshot::primary_window` + `save_to_disk`,
acked into `CaptureLog`; `shot_written(path)` awaits the ack and reads
`true` at once on the non-capturing (smoke) path
(`crates/nova_autopilot/src/predicate.rs:196-200`). This file already uses
the idiom twice (`"world_resume-status-line.png"`,
`"world_resume-leave-overlay.png"`). I will shoot
`"world_resume-before-leave.png"` (section 3) and
`"world_resume-after-load.png"` (section 4), both resolved under
`NOVA_CAPTURE_DIR` the same as the existing two shots.

No image-diff utility exists anywhere in the repo today (checked
`crates/nova_bench`, `crates/nova_authoring`, `examples/playable/shared/compare.rs`
- the latter loads candidate textures for material comparison, not a PNG-vs-PNG
pixel diff). `bevy` is already a `[dev-dependencies]` entry for the root
package (`Cargo.toml:1079`), so no new Cargo dependency is needed: decode
both captured PNGs with `bevy::image::Image::from_buffer` exactly as
`examples/playable/shared/compare.rs:104-120` already does (pure CPU decode,
no `Assets<Image>` needed - I only need the raw `.data` bytes and
`.texture_descriptor.size`), then compute a plain mean-absolute-per-channel
difference over every pixel as an `f64` (0-255 scale), guarding on equal
dimensions first (panic naming both paths and their sizes on a mismatch -
that is a real bug, not noise).

This number is **reported, not asserted** - logged via `info!` and a
`nova_probe::probe_marker`, the same "measurement, no assertion" idiom
P-T7 already uses for its record-size log (TRANSIENT-GATE.md section 10,
P-T7 bullet). A hard threshold on two independently-rendered frames across
two processes (different UI state transiently, antialiasing, and HUD
timing) is exactly the kind of claim "inspect rendered output for visual
claims" (AGENTS.md) asks a person to judge, not an assertion to decide -
and it is consistent with "never assert timing," since any such threshold
would really be asserting how fast the two frames converged.

Only runs on the `NOVA_CAPTURE=1` path (gated the same way `shoot` already
is - on the plain `NOVA_AUTOPILOT=1` smoke path, `shot_written` returns
`true` at once and no file exists to diff, so the diff step itself must
also check `nova_autopilot::capture::capturing()` and skip with a logged
no-op otherwise, exactly mirroring how `shoot` itself behaves).

## 7. New fns/types/consts in the example

```rust
/// Combat-stance + PDC trigger held every frame, like system_turret_gunnery's HeldInput/hold_inputs.
#[derive(Resource, Default)]
struct FireHeld { combat: bool, pdc: bool }
fn hold_fire_inputs(world: &mut World, _elapsed: f32, _frame: u32);

/// Finds two distinct PDC bays and the torpedo bay on `player`, by ChildOf + marker.
fn player_weapon_fixtures(world: &mut World, player: Entity) -> (Entity /* fire PDC */, Entity /* kill PDC */, Entity /* torpedo bay */);

/// Fires the PDC (via FireHeld), launches the torpedo (ScriptedTorpedoOrder),
/// and kills the second PDC (HealthApplyDamage). on_enter of the new step.
fn fire_transient_fixture(world: &mut World);

/// All three transient markers are live - the load phase's "first restored frame" gate.
fn fixture_transients_restored() -> Arc<Predicate>;

/// All three transient markers are gone - the "then expires" gate.
fn fixture_transients_expired() -> Arc<Predicate>;

/// Extends record_expected_state's JSON with round/torpedo/piece pose + remaining lifetime.
fn record_transient_fixture_state(world: &mut World) -> serde_json::Value; // merged into the existing expected object

/// Mean abs per-channel difference between two captured PNGs under NOVA_CAPTURE_DIR. Reported, not asserted.
fn report_pixel_difference(before_path: &str, after_path: &str);

const PDC_FIRE_DEADLINE_SECS: f32 = ...; // short: minimizes the round's 2.0s exposure
const TRANSIENT_EXPIRE_DEADLINE_SECS: f32 = ...; // sized off the piece's 30s grace, not the round
```

No change to `Fixture`/`LoadWatch`; the new state threads through local
closures and `expected.json`, matching the file's existing shape.

## 8. How I will run it

```
Xvfb :99 &
XVFB_PID=$!
export DISPLAY=:99
export VK_DRIVER_FILES=<path-to-lvp_icd.json>   # resolve at implementation time, same as other lavapipe runs
export ALSA_CONFIG_PATH=/dev/null               # empty, mirrors CI no-audio
export NOVA_AUTOPILOT=1
export NOVA_CAPTURE=1                            # required for the two new shots and the pixel-diff
export NOVA_CAPTURE_DIR=<scratch dir>
export NOVA_AUTOPILOT_DEADLINE=<generous>        # this is a long two-process walk with a real streamed window
nix develop --command cargo run --example system_world_resume --features debug -j 8
kill "$XVFB_PID"                                  # only this recorded PID, never a broad pkill
```

I will record `$XVFB_PID` (and the load phase's own child PID, which the
create phase's `run_create` already spawns and waits on synchronously - no
separate kill needed for it, since `Command::status()` blocks until it
exits on its own).

## Summary of open items before implementation

1. Whether a freshly-detached piece exposes `TempEntityState` or only
   `ChunkGrace` at the moment our predicate first sees it (section 3) -
   resolved by a quick instrumented run, not a guess.
2. The PDC round's 2.0 s `projectile_lifetime` is the tightest timing margin
   in the plan; mitigated by firing it immediately before the successful
   "Try again" click rather than earlier in the walk, but this is the one
   piece of the plan I'd want to see survive a real lavapipe run before
   trusting it.
3. The pixel-difference figure is a logged measurement, not an assertion,
   per section 6 - flagging this explicitly in case the owner wants a
   different call.

## Round 2: implementation (2026-10-09)

Scope held: only `examples/systems/system_world_resume.rs` edited, plus this
report. No production file touched or proposed. Owner-approved items only:
`FireHeld`, `hold_fire_inputs`, `player_weapon_fixtures`,
`fire_transient_fixture`, `fixture_transients_restored`,
`fixture_transients_expired`, `record_transient_fixture_state`,
`report_pixel_difference`, `PDC_FIRE_DEADLINE_SECS`,
`TRANSIENT_EXPIRE_DEADLINE_SECS`. No other new fn/type/const/test was added.

**Status: blocked on a live-run finding, not finished.** The CREATE phase's
fixture-firing step still stalls every run. I found and fixed one real bug in
my own script (the clock-freeze ordering below), which is worth keeping; the
remaining blocker looks production-side and I'm stopping to report it rather
than guessing further, per the owner's "if a production change looks needed,
stop and write why" instruction.

### What I built

- `FireHeld` (`:550`), `hold_fire_inputs` (`:568`, wired via
  `.input(hold_fire_inputs)` on `create_script`) - held `combat_stance` +
  raw `MouseButton::Left`, re-pressed every `PreUpdate` frame after
  `InputSystems`, same idiom as `system_turret_gunnery.rs`'s `hold_inputs`.
- `player_weapon_fixtures` (`:587`) - finds two PDC bays and the torpedo bay
  as children of the player ship, sorted by `Entity` for a deterministic
  `(fire_pdc, kill_pdc, torpedo_bay)` pick.
- `fire_transient_fixture` (`:626`) - `on_enter` of the fire step: sets
  `FireHeld{combat:true, pdc:true}`, inserts `ScriptedTorpedoOrder{target:
  Entity::PLACEHOLDER}` on the torpedo bay, and triggers
  `HealthApplyDamage{amount: max}` on the second PDC bay.
- `fixture_transients_restored`/`fixture_transients_expired` (`:667`,
  `:681`), `record_transient_fixture_state` (`:705`, reads the leave save's
  own `state.N.ron` via `open_world()` - never hand-parsed RON),
  `report_pixel_difference` (`:843`) - all built as planned, not yet
  exercised end-to-end because the walk never reaches the leave save.
- `assert_resumed_state` extended to check the round's `Transform`, the
  torpedo's avian `Position`, and all three `SavedLifetime.remaining`
  against `expected["transients"]`; the piece's live pose is logged, not
  asserted (`FrozenDetachedPiece` exposes no pose accessor - see "Found
  limitation" below).
- `load_script` extended: wait for `fixture_transients_restored()`, shoot
  `world_resume-after-load.png`, call `report_pixel_difference`, then (after
  the existing `assert_resumed_state`) wait for `fixture_transients_expired()`
  with `TRANSIENT_EXPIRE_DEADLINE_SECS`.

### Bug found and fixed: firing inside the paused leave detour never ran

My first script put the fire step between "make the world folder writable"
and the successful "Try again" click (`:1019` area before the fix) - i.e.
*after* "open the pause menu" (`:1062`). That is wrong: the pause overlay
holds `FreezeOwner::PauseMenu` (`crates/nova_gameplay/src/freeze.rs:30-44,
146-157`), which calls `Time<Virtual>::pause()` and `Time<Physics>::pause()`
outright while held. Nothing timed ticks behind it - not a muzzle cooldown,
not a stow deploy, not a `TempEntity` lifetime. Two runs confirmed this
exactly: the autopilot's own "run {elapsed}s" figure (the whole script's
accumulated `Time<Virtual>`, not the step-local one) read `8.5s` and then
`8.9s` on two separate runs with the fire step held open for 60.1s and
180.2s of *real* time respectively - the virtual clock was not moving at
all, because it had already been frozen by the pause overlay several steps
earlier.

Fix: moved the whole fire sequence (fire, release-on-first-round,
wait-for-torpedo-and-piece, shoot-before-leave) to run *before* "open the
pause menu", right after "assert the saved header on disk" (`:987`). This
also removes the tight 2.0s round-lifetime timing risk the Round 1 plan
flagged (section 8, open item 2) in a different and better way than
originally planned: once paused, the transients' pose and remaining
lifetime are locked at whatever they were the instant pause took the
clock, so the read-only-folder failed-leave detour can take as long as it
needs with no decay risk at all.

After this fix, a third run (`NOVA_AUTOPILOT_DEADLINE=600`,
`RUST_LOG=info,nova_ship=debug`) showed `run 187.4s` against a 180.0s
step-local stall, confirming the clock was now genuinely live and advancing
near 1:1 with real time. The round still never fired.

### Remaining blocker: `TurretSectionInput` never goes true

Added a temporary `.diagnose()` closure to the fire step (`:1005`,
anonymous closure, not a new named fn) to read live state on stall. Output
from the fourth run (a 20s deadline, just to get the reading faster):

```
WeaponsRaised=Some(true) WeaponsHot=Some(true) CombatLock=Some(None)
turrets(entity,stow,input)=[(1721v1, Some(Deployed), Some(false)),
(1808v1, Some(Deployed), Some(false)), (1722v1, Some(Deployed), Some(false)),
(1719v1, Some(Deployed), Some(false)), (1720v1, Some(Deployed), Some(false))]
```

So: `combat_stance` is raised, `WeaponsHot` derives to `true`
(`crates/nova_ship/src/input/targeting/safety.rs:26-33`), every PDC bay is
fully `Deployed` (`crates/nova_ship/src/sections/turret_section/stow.rs`) -
deploy is fast and was never the bottleneck. But `TurretSectionInput` is
`false` on every one of the five remaining PDC bays (the sixth is the
`kill_pdc`, already destroyed - confirmed by the `integrity: destroyed 1
node (pdc_kinetic_turret_section x1)` log line appearing immediately after
`fire_transient_fixture` runs). This held across all four runs (hundreds of
sim-frames with the raw `MouseButton::Left` press re-asserted every
`PreUpdate` frame), so it is not a one-frame scheduling skew - the press
never registers at all.

`TurretSectionInput` is set only by
`crates/nova_ship/src/input/player/weapons.rs:183-218`
(`on_turret_input`, an observer on `On<Start<TurretInput>>`), which itself
only returns early on a frozen pause state (ruled out, confirmed Unpaused)
or a cold safety (ruled out, `WeaponsHot=true`). So the blocker is upstream:
the `Action<TurretInput>` bound per PDC section via
`SpaceshipTurretInputBinding`/`on_turret_input_binding` (`weapons.rs:149-181`)
never emits `Start<TurretInput>` for a raw `ButtonInput<MouseButton>::press`
synthesized this way, even though the *exact same* raw-press mechanism
(`drive_action`, which itself "writes the button state and nothing else" -
`crates/nova_debug/src/harness.rs:613-616`) correctly drives `combat_stance`
in the same run, and the same `.input()`-hook idiom is documented as
working for `TurretInput`-shaped fire actions in
`system_turret_gunnery.rs`'s simpler, standalone test rig.

I did not find a production reason this should fail by reading
`on_turret_input`, `update_point_defense_ownership`
(`crates/nova_ship/src/input/point_defense/ownership.rs` - `MountAuthority`
re-resolves every frame from live `WeaponsRaised`, is not sticky, and
`on_turret_input` does not consult it anyway), or the `TurretInput` action
definition itself (a plain `#[derive(InputAction)] #[action_output(bool)]`,
same shape as the working `ThrusterInput`). The two candidates I could not
rule out without either instrumenting production code or more running time
I did not have left in-budget:

1. A real scheduling-order difference between the full `editor_app` and
   `system_turret_gunnery.rs`'s minimal rig: `autopilot_drive` is only
   ordered `.after(InputSystems)` (`crates/nova_autopilot/src/
   autopilot.rs:531`), with no explicit edge against
   `bevy_enhanced_input`'s own evaluation set, so their relative order in
   the full app's larger schedule DAG is not something I could confirm
   from source alone.
2. Something ship-design-specific to `BLOCK_LINE_WARSHIP` that keeps its
   `SpaceshipTurretInputBinding`/`Actions<TurretInputMarker>` from ever
   being live on these entities at all, which I could not distinguish from
   (1) without instrumenting `on_turret_input_binding`/`on_turret_input`
   directly (a production edit I did not make).

### Runs (4, foreground, owned `Xvfb :117`, lavapipe)

All four used the same env shape: `DISPLAY=:117`,
`VK_DRIVER_FILES=.../mesa-26.2.3/share/vulkan/icd.d/lvp_icd.x86_64.json`,
`ALSA_CONFIG_PATH=<empty file>`, `NOVA_AUTOPILOT=1`, `NOVA_CAPTURE=1`,
`NOVA_CAPTURE_DIR=/tmp/nova-pt9/capture`,
`CARGO_TARGET_DIR=./target`, `cargo run --example system_world_resume
--features debug -j 8 -- --phase create`.

1. `PDC_FIRE_DEADLINE_SECS=60.0`, no extra logging: stalled at `run 8.5s`
   after 60.1s real - first sign of the frozen-clock bug.
2. Same script, `PDC_FIRE_DEADLINE_SECS=180.0`,
   `RUST_LOG=info,nova_ship=debug`: stalled at `run 8.9s` after 180.2s real
   - confirmed the clock was not moving, not just slow.
3. After the pause-ordering fix, same deadline: stalled at `run 187.4s`
   after 180.0s real - clock genuinely live now, round still never fired.
4. Added the `.diagnose()` closure, `PDC_FIRE_DEADLINE_SECS=20.0` to get
   the reading faster: produced the state dump above.

I did not run a fifth. Xvfb (`:117`, PID recorded at start) was stopped by
that exact PID and verified gone with `ps -p` before writing this report.
No screenshot comparison, load-phase assertion log, or saved-transient
listing exists yet - the walk never reaches the leave save, so there is no
`state.N.ron` with real transients to read and no `before-leave.png`/
`after-load.png` pair to compare. Nothing here is asserted against made-up
numbers; both pending sections below are explicitly unverified.

### Found limitation (not a stop-and-ask, documented per owner's instruction)

`FrozenDetachedPiece` (`crates/nova_ship/src/sections/frozen_piece.rs`)
exposes no public pose accessor, only `style()` and `validate()`. The load
phase's `assert_resumed_state` therefore logs the resumed piece's live
`Transform` rather than asserting it against the saved record; the piece's
`remaining` lifetime IS asserted, since `FrozenTransient.lifetime` wraps
every transient kind uniformly. This narrows the piece's own assertion
coverage but does not block the test, so I judged it did not need a
stop-and-ask; flagging it here as asked.

### Handoff

Code change kept: the clock-freeze reorder (real fix, confirmed by run 3's
`run 187.4s` vs runs 1-2's frozen `~8.5s`/`8.9s`). Code change kept: the
`.diagnose()` closure on the fire step (useful for whoever runs this next;
trivial to remove). `PDC_FIRE_DEADLINE_SECS` restored to `180.0`.
`cargo build --example system_world_resume --features debug` passes;
`rustfmt --edition 2024` applied to this file only (`git diff --stat`
checked after - no other file touched).

Needed to proceed: either (a) a quick instrumented production run (adding
a `trace!`/`debug!` to `on_turret_input_binding`/`on_turret_input` to see
whether the observer fires at all and with what `context` entity - a
production file edit I did not make), or (b) owner guidance on whether
`BLOCK_LINE_WARSHIP`'s PDC bays are expected to be player-fireable by raw
mouse input outside of a live render/window-focus loop the way this
harness drives every other input, since that is the one precondition nothing
in the source rules out for me.

## Round 3: the WeaponsHot sequencing fix, plus a timing gap it exposed (2026-10-09)

Scope held: only `examples/systems/system_world_resume.rs` edited, plus this
report. No production file touched. Owner-approved items only: the
`hold_fire_inputs`/`fire_transient_fixture` sequencing fix, the piece pose
assertion via the new `FrozenDetachedPiece::translation()`, and removing the
temporary `.diagnose()` closure. No new named fn/type/const was added.

**Status: the round trip now runs end to end and all three transients
survive; one pre-existing, unrelated assertion (ammo stock) blocks the final
PASS line and needs a decision.**

### What I changed

1. `hold_fire_inputs` (`:579`) no longer presses the PDC trigger in the same
   frame as `combat_stance`. It presses `combat_stance` every frame as
   before, then only sets `FireHeld.pdc = true` (latched, gated on `combat`
   also being true so a stale-true `WeaponsHot` reading can never re-arm the
   trigger after release) once `WeaponsHot` reads true on the player ship.
   `fire_transient_fixture` (`:643`) now sets only `FireHeld.combat = true`
   on entry; it no longer sets `pdc` directly. This was round 2's diagnosed
   root cause: a same-frame `combat_stance` + LMB press fired
   `Start<TurretInput>` once while `WeaponsHot` was still false that frame,
   so `on_turret_input` returned and the action latched `Fired` with no
   further `Start` to retry it. Confirmed fixed: every one of this round's 4
   runs fired at least one round, where round 2's 4 runs fired none.
2. `record_transient_fixture_state` (`:808`, `:829`) and
   `assert_resumed_state`'s piece section (`:1415`, `:1426`) now read and
   assert the piece's saved `translation()` the same way the round and
   torpedo already did, using the new `FrozenDetachedPiece::translation()`
   accessor (`crates/nova_ship/src/sections/frozen_piece.rs:67-70`, added by
   the main worker). This closes the "Found limitation" gap round 2's report
   flagged.
3. The temporary `.diagnose()` closure round 2's `pf-lmb` trace left on the
   fire step is gone. It was the only `.diagnose()` call in this file, so
   per the owner's "only if the file already uses the idiom on other steps"
   the whole closure was removed, not trimmed.

### A timing gap the fix exposed, and how I closed it

Runs 1-2 fired correctly but the round never reached the leave save: the
final `state.N.ron` held `DetachedPiece` and 2-3 `Torpedo` entries but zero
`Round`s (run 1: `["DetachedPiece", "Torpedo", "Torpedo", "Torpedo"]`; run 2:
`["DetachedPiece", "Torpedo", "Torpedo"]`). Trace logging on
`nova_gameplay::rounds` (run 3, `nova_gameplay::rounds=trace`) recorded zero
`"struck"` lines across the whole run, ruling out a collision despawn. The
remaining explanation is the round's authored 2.0s `projectile_lifetime`
expiring before the pause overlay's `FreezeOwner::PauseMenu` hold actually
takes `Time<Virtual>` (and so `update_temp_entities`'s `Res<Time>` tick,
`crates/nova_gameplay/src/lifetime.rs:155-161`) to zero: on lavapipe each
step-transition between "round confirmed" and "pause menu opens" cost
measurable real time (runs 1-3 each showed 1.6-2.3s elapsed there, against a
2.0s budget), and round 2's reordering (fire before pause) only removed the
*decay-while-unpaused* risk after pause engages, not the real time the
engine still needs to get there.

Two mechanical changes, using only already-approved items, closed the gap:

- Merged the "release the trigger" step and the "wait for the torpedo and
  the detached piece too" step into one (`:1028`): the release still fires
  on entry, at the exact same moment as before (the moment the round is
  confirmed, not once every transient is - preserving the existing
  anti-"more rounds every reload" guard), but the torpedo/piece wait is now
  this step's own exit condition instead of a separate step, removing one
  lavapipe frame-transition from the critical path.
- The "shoot the frame right before the leave" step (`:1054`) no longer
  blocks on `shot_written(...)` before pressing Escape. The screenshot
  command still captures the exact same live, chrome-free frame (the
  capture happens on that frame regardless of when the async disk write
  lands); the wait for the file to land was moved onto the following "open
  the pause menu" step's own `.until()` (`:1057`, `and(ui_node_present(...),
  shot_written(...))`), which the engine reaches on its own schedule
  regardless of the autopilot script's step tracking - so Escape now presses
  one frame after the shot command instead of after its disk round trip.

Run 4 (same code, `info,nova_ship=debug`, no trace overhead) confirms this
was enough: the leave save held 198 transients - `DetachedPiece`, 2
`Torpedo`s, and **196 `Round`s** - with the representative round at
`0.19/2.00s` remaining. That is a thin margin (under 10% of the round's
total lifetime), reproducible but fragile; flagging this for the owner in
case a larger margin is wanted (see Decision needed, below).

The 196-round count is itself a finding: every deployed, hot PDC bay fires
*continuously* while the raw trigger is held, not once per press (the doc
comment already said "not aimed at one specific bay," but I had read that as
one round per bay, not one round per bay per frame). That also explains why
earlier runs' saved `Torpedo` counts varied (2-3): those were not re-fires
of our single `ScriptedTorpedoOrder`, but other Open World NPC traffic
already in flight, unrelated to the fixture.

### Runs (4, foreground, owned `Xvfb :117`, lavapipe)

Env shape for all 4: `DISPLAY=:117`,
`VK_DRIVER_FILES=/nix/store/1vkzwrp4ny20iljfnv9valq0bfn8czlv-mesa-26.2.3/share/vulkan/icd.d/lvp_icd.x86_64.json`,
`ALSA_CONFIG_PATH=<empty file>`, `NOVA_AUTOPILOT=1`, `NOVA_CAPTURE=1`,
`NOVA_CAPTURE_DIR=/tmp/nova-pt9/capture`, `NOVA_AUTOPILOT_DEADLINE=900`,
`CARGO_TARGET_DIR=./target`, `cargo run --example system_world_resume
--features debug -j 8 -- --phase create`.

1. `RUST_LOG=info,nova_ship=debug`: fix confirmed working (first round ever
   fired), but leave save held 0 `Round`s, 3 `Torpedo`s, 1 `DetachedPiece`.
   Log: `/tmp/nova-pt9/run-r3-1.log`.
2. Same, after merging the release/wait steps: 0 `Round`s, 2 `Torpedo`s, 1
   `DetachedPiece`. Log: `/tmp/nova-pt9/run-r3-2.log`.
3. `RUST_LOG=info,nova_ship=debug,nova_gameplay::rounds=trace,nova_gameplay::lifetime=trace`
   (diagnostic only, no code change): confirmed zero collision ("struck")
   events, ruling out a hit-despawn. 0 `Round`s, 2 `Torpedo`s, 1
   `DetachedPiece`. Log: `/tmp/nova-pt9/run-r3-3.log`.
4. `RUST_LOG=info,nova_ship=debug`, after decoupling the screenshot's disk
   wait: create phase succeeded end to end, leave save held 196 `Round`s, 2
   `Torpedo`s, 1 `DetachedPiece`; load phase resumed all three, shot the
   after-load frame, computed the pixel-difference figure, then panicked on
   the pre-existing stock assertion (see Decision needed). Log:
   `/tmp/nova-pt9/run-r3-4.log`. Sandbox kept at
   `target/example-profiles/system_world_resume-3524124-1791518200378981694/`
   (the run's own panic path; not cleaned up since it holds the evidence
   below).

All 4 used one `Xvfb :117 -screen 0 1920x1080x24`, PID `3521142` (the real
Xvfb PID, not the backgrounding subshell's `$!`), started once and reused
across runs. Stopped by that exact PID and verified gone with `ps -p` after
run 4; `ps -p 3521142` exited 1 (no such process).

### The newest state.N.ron (run 4)

`target/example-profiles/system_world_resume-3524124-1791518200378981694/worlds/probe-world/state.4.ron`,
**454531 bytes**. 198 transients total: `DetachedPiece` x1, `Torpedo` x2,
`Round` x196 (every deployed PDC bay firing continuously while the trigger
was held, not a bug in this fixture - see above). The representative one of
each kind (read back via `open_world`, the same typed read a real Load
uses, in `record_transient_fixture_state`):

| kind | owner | translation | lifetime (remaining/total) |
| --- | --- | --- | --- |
| Round | `Ship(EntityId("player"))` | `(3199.91, 1.14, -161.99)` | `0.19s / 2.00s` |
| Torpedo | `Ship(EntityId("player"))` | `(3198.00, 0.00, -53.28)` | `96.96s / 100.00s` |
| DetachedPiece | (no owner field) | `(3169.59, -53.22, 0.00)` | `25.82s / 30.00s` |

### Load phase

The load phase reached `GameStates::Playing`, waited for
`fixture_transients_restored()` (all three marker kinds live - confirmed by
the step `world_resume load: wait for the fixture transients to resume`
completing), shot `world_resume-after-load.png`, computed the
pixel-difference figure, then entered `assert_resumed_state`. Inside that
function, execution reached `info!("...the resumed ledger seam ran")`
(`nova_debug: nova harness: reached Playing` confirms `ResumedWorld` seeded
and consumed), which is logged *after* the player pose and credits
assertions pass and *before* the stock assertion - so pose and credits
PASSED; stock FAILED (see below) before the code ever reaches the rock,
canister, sector, or transient-specific (round/torpedo/piece pose and
lifetime) assertions. Those assertion lines did not run this round; the
only evidence for round/torpedo/piece fidelity on the load side is the
`fixture_transients_restored()` wait succeeding (all three kinds exist) and
the saved reference values above, not yet a line-by-line pose/lifetime
check.

### Decision needed: the stock assertion

```
assertion `left == right` failed: world_resume load: resumed stock does not match the saved stock
  left:  [..., {"item": "PdcRound", "count": 5736}, ...]
  right: [..., {"item": "PdcRound", "count": 6000}, ...]
```

This is NOT a P-T9 bug: it is the pre-existing stock assertion (predates
this plan) catching a real, expected side effect of the fixture now
actually firing 196 live rounds through production code - each bay draws
its magazine refill from the ship's `ShipInventory` reserve, so the
player's `PdcRound` stock genuinely drops between "record the expected
state" (before combat) and "check the resumed state" (after it). Every
other stock line matched exactly (`HullPlate`, `RailSlug`, `Torpedo`,
`Rations` all equal). I did not change this assertion or its expected-value
capture point, since it is outside my approved scope and is an error-policy
call (AGENTS.md: "Stop for ... error policy"). Options, as I see them:

- **A.** Re-capture `expected["stock"]` a second time right after
  `fire_transient_fixture` confirms all three transients, and assert
  against that post-fire snapshot instead of the pre-fire one. Keeps exact
  equality everywhere; needs a new capture point (not just a decision, a
  small new function).
- **B.** Special-case `PdcRound` in `assert_resumed_state` to allow
  `resumed <= expected` instead of `==`, leaving every other stock line
  exact. Smallest change, but weakens an assertion that was exact before.
- **C.** Leave the stock assertion exact and drop it from this script's
  scope entirely once P-T9's transients are added (its coverage - that
  credits/stock survive a leave/load - is arguably subsumed by the already
  separately-asserted credits line plus the fact nothing else touches
  `ShipInventory` here). Loses stock coverage entirely, not just for
  `PdcRound`.

No change made pending a decision; the rest of round/torpedo/piece
assertions are untested beyond "all three exist" until this is resolved and
the script runs past this line, which may still have its own findings. I
did not run a 5th time, per the 4-run budget.

### Screenshots

Both `world_resume-before-leave.png` and `world_resume-after-load.png`
(`/tmp/nova-pt9/capture/`) frame the player ship from the same chase-camera
angle, facing away from camera, no pause chrome in either (matching the
module doc's "plain gameplay" requirement). `before-leave.png` shows a
bright muzzle tracer just above the dorsal PDC cluster (the just-fired
round, consistent with it having 0.19s left by the time of the later save).
`after-load.png` shows the same framing and ship pose, with the tracer
mostly faded to a small point near the same spot (consistent with the
near-expired round) and a visibly lower FPS counter (33 -> 5, expected right
after a full 125-sector re-stream on lavapipe). Pixel-difference figure
(reported, not asserted, per section 6 of the plan):

```
world_resume: pixel-difference figure between .../world_resume-before-leave.png and .../world_resume-after-load.png: mean |channel diff| = 0.487 (0-255 scale)
```

### Paths

- Logs: `/tmp/nova-pt9/run-r3-1.log`, `run-r3-2.log`, `run-r3-3.log`,
  `run-r3-4.log`.
- Screenshots: `/tmp/nova-pt9/capture/world_resume-before-leave.png`,
  `world_resume-leave-overlay.png`, `world_resume-status-line.png`,
  `world_resume-after-load.png` (all from run 4; earlier runs' shots were
  overwritten in place since every run used the same `NOVA_CAPTURE_DIR`).
- Kept sandbox (run 4, holds `state.4.ron` and `expected.json` above):
  `target/example-profiles/system_world_resume-3524124-1791518200378981694/`.

### Handoff

`cargo build --example system_world_resume --features debug` passes.
`rustfmt --edition 2024` applied to this file only after every edit;
`git diff --stat` checked each time - no other tracked file touched (the
file itself is still untracked, not yet committed by anyone).

Needed to proceed: a decision on the stock assertion (A/B/C above, or
another option). Once that lands, one more run should reach the
round/torpedo/piece pose and lifetime assertions this round never got to
exercise, which may surface further findings of their own. Separately
flagging the round's thin (0.19s/2.0s, under 10%) survival margin in case
the owner wants more headroom than the mechanical fixes in this round
provide - a real margin increase would mean either reworking what happens
between "round confirmed" and "pause menu opens" further, or no longer
racing the PDC's authored 2.0s lifetime at all (e.g., firing it closer to
when the torpedo and piece are already ready, which decision #1's wording
("hold combat_stance first") did not anticipate).

## Round 4: stock fix, tighter margin, and the round's load-side expiry (2026-10-09)

Scope held: only `examples/systems/system_world_resume.rs` edited, plus this
report. No production file touched. Owner-approved items only: moving the
`write_expected_state` step (decision 1), reordering the fire sequence into
four steps (decision 2), correcting the fire-step comment against
`crates/nova_menu/src/leave.rs` (decision 3), and - after a mid-round
owner ask below - re-splitting which step arms `combat_stance` vs. the PDC
trigger. No new named fn/type/const was added; `hold_fire_inputs` lost its
`WeaponsHot`-latch branch (fewer lines, not a new item) on the owner's own
instruction.

**Status: the create phase now runs end to end with the stock assertion
passing exactly, and reaches every load-side assertion decision 4 asked
about. Four of those PASS (pose, credits, stock, rock, canister, sector);
the round-specific one FAILS, reproducibly, for a reason evidenced below -
not fixed, per scope.**

### 1. The stock capture point (decision 1)

Moved the `"world_resume: record the expected state"` step (`write_expected_state`)
from right after "shoot the save status line" to right after "let escape
go" (after the pause menu opens and the fire sequence completes), and
before "make the world folder read-only". Pure step reordering, no function
touched. Confirmed in both runs that reached it (runs 2 and 3 below): the
stock assertion in `assert_resumed_state` now passes with no change to the
assertion itself - execution proceeds past it every time (see "Load
assertions" below).

### 2. The fire-sequence reorder (decision 2) - and a design gap it exposed

Implemented the literal spec first: Step A fires the torpedo order and the
kill-damage trigger, waits for `TorpedoProjectileMarker` AND
`DetachedPieceMarker`; Step B arms `combat_stance`/fires the PDC trigger. Run
1 (`/tmp/nova-pt9/run-r4-1.log`) stalled Step A for the full 180.0s
`PDC_FIRE_DEADLINE_SECS` and panicked (exit 101): the torpedo never
launched. Cause: `crates/nova_ship/src/sections/torpedo_section/bay.rs:466-470`,
`shoot_spawn_projectile`'s own comment - "Live weapons-safety gate, same
rule as the turret" - gates torpedo launch on `WeaponsHot` on a MANAGED
ship exactly like the PDC turret does; Step A never raised `combat_stance`,
so `WeaponsHot` stayed false and the torpedo sat held forever. This was a
design gap in decision 2's spec, not a mechanical bug, so I stopped and
asked before using another run (`subagent_ask`, full exchange preserved in
session history). The owner's answer, used as given: `combat_stance` now
arms in Step A (ahead of the torpedo/damage trigger, so `WeaponsHot` is
already true when the torpedo order commits); Step B only sets
`FireHeld.pdc`; `hold_fire_inputs` no longer auto-latches `pdc` off
`WeaponsHot` - it presses whichever of `combat`/`pdc` the script has already
set, nothing more. `FireHeld`'s and `hold_fire_inputs`'s doc comments were
updated to state this order and the reason (the bay gate, and
`on_turret_input`'s own hot check), with no history narration, per
instruction.

Runs 2 and 3 (below) both fired correctly end to end on the first try after
this fix: Step A satisfied `TorpedoProjectileMarker` + `DetachedPieceMarker`
in each case, Step B then satisfied `TurretBulletProjectileMarker` with no
stall.

### 3. The fire-step comment (decision 3)

Checked `crates/nova_menu/src/leave.rs:89-118` (`drive_pending_leave`): while
a leave save is in flight (`!session.is_idle()`), it calls
`clocks.release(FreezeOwner::PauseMenu)` every frame - the pause hold IS
released while a save settles - and only calls `clocks.hold(FreezeOwner::PauseMenu)`
again once that attempt resolves (saved or failed). So the old claim
("every step from here through the read-only-folder detour runs PAUSED")
was wrong for the settle frames of each save attempt, including the final
successful one. Rewrote the comment to state that: the hold stays engaged
while idle, but each write - the failed read-only attempt and the eventual
successful retry - briefly releases it and ticks a few frames of real decay
right before it lands. This also explains why the recorded round margins
below (0.16s-0.69s) are thinner than round 3's 196-round, single-snapshot
0.19s figure: decision 2's reorder means the round now fires LAST in the
pre-pause sequence (closest to the leave), but the settle-window decay on
the final successful "Try again" click still eats into whatever margin is
left by the time that write actually lands.

### Runs (3, foreground, owned `Xvfb :117`, lavapipe)

Env shape for all 3: `DISPLAY=:117`,
`VK_DRIVER_FILES=/nix/store/1vkzwrp4ny20iljfnv9valq0bfn8czlv-mesa-26.2.3/share/vulkan/icd.d/lvp_icd.x86_64.json`,
`ALSA_CONFIG_PATH=<empty file>`, `NOVA_AUTOPILOT=1`, `NOVA_CAPTURE=1`,
`NOVA_CAPTURE_DIR=/tmp/nova-pt9/capture-r4`, `NOVA_AUTOPILOT_DEADLINE=900`,
`CARGO_TARGET_DIR=./target`, `cargo run --example system_world_resume
--features debug -j 8 -- --phase create`. One `Xvfb :117 -screen 0
1920x1080x24`, real PID `3526400` (verified via `ps aux`, not the
backgrounding subshell's `$!` - a first `pgrep -f "Xvfb :117"` false-matched
my own shell wrapper's command line, caught and corrected before relying on
it), started once and reused across all 3 runs. Stopped by that exact PID
after run 3 and verified gone: `ps -p 3526400` exited 1 (no such process).

1. Literal decision-2 spec (combat arms in Step B): Step A stalled the full
   180.0s deadline, torpedo never launched, create phase panicked (exit
   101) at `examples/systems/system_world_resume.rs:1179`. Log:
   `/tmp/nova-pt9/run-r4-1.log`. Stopped and asked the owner (see above);
   no further runs used on this attempt.
2. After the owner's fix (combat arms in Step A): create phase ran end to
   end - 205 transients saved (`DetachedPiece` x1, `Torpedo` x2, `Round`
   x202), stock assertion passed, load phase reached the round-specific
   assertion and panicked there (see "Load assertions" below). Log:
   `/tmp/nova-pt9/run-r4-2.log`. Sandbox kept:
   `target/example-profiles/system_world_resume-3528295-1791519347457891513/`.
3. Same code, to check reproducibility: create phase saved 212 transients
   (`DetachedPiece` x1, `Torpedo` x2, `Round` x209), same stock pass, same
   round-specific panic at the same line. Log: `/tmp/nova-pt9/run-r4-3.log`.
   Sandbox kept:
   `target/example-profiles/system_world_resume-3529017-1791519492196957295/`.

### The newest state.N.ron (run 3)

`target/example-profiles/system_world_resume-3529017-1791519492196957295/worlds/probe-world/state.4.ron`,
**470937 bytes**. 212 transients: `DetachedPiece` x1 (no owner field, as
documented), `Torpedo` x2, `Round` x209 - all 211 owned entries read
`Ship("player")` (counted directly off the RON: `grep -c 'owner:Ship'`-style
scan, 211 matches, zero of any other owner). Per-kind remaining lifetime
(parsed from the RON's own `(lifetime:(total:...,remaining:...),body:KIND(...))`
pairing, not estimated):

| kind | count | smallest remaining | newest (largest remaining) |
| --- | --- | --- | --- |
| Round | 209 | **0.1626s** / 2.00s | **0.5660s** / 2.00s |
| Torpedo | 2 | 97.45s / 100.00s | 98.53s / 100.00s |
| DetachedPiece | 1 | 26.17s / 30.00s (only one) | - |

Run 2's state.4.ron (468499 bytes, 205 transients, 202 Rounds) showed the
same shape: smallest remaining 0.3638s, newest (largest remaining) 0.6854s.
In both runs, even the NEWEST-fired round (the one with the most lifetime
left at save time) had well under 1s remaining - nowhere close to the ~2s
real-time gap between resume and the load-side assertion (see below).

### Load assertions (decision 4)

The function is a straight sequence of asserts with no early return before
a failure, so reaching one line without a panic is evidence every assert
above it passed. In both runs 2 and 3, execution logged
`"world_resume load: the resumed ledger seam ran"` (confirming `ResumedWorld`
was seeded and consumed), then proceeded all the way to the round entity
lookup, where it panicked:

```
thread 'main' panicked at examples/systems/system_world_resume.rs:1351:14:
world_resume load: the resumed round is not in the live window
```

That is `round_entity = world.query_filtered::<Entity, With<TurretBulletProjectileMarker>>().iter(world).next().expect(...)`
finding zero matches. Reaching this line without an earlier panic means, in
order: player pose, `ShipCredits`, `ShipInventory` stock (the decision-1
fix), the carved rock's `BodyRadius`, the resumed canister's contents, and
`CurrentSector` ALL PASSED in both runs - these are the rock/canister/sector
assertions decision 4 asked about, confirmed running and passing. There is
no separate per-assertion PASS log line in the code (only the final
`"world_resume load: PASS..."` line, which this run never reaches, and the
panic message itself on failure) - so "passed" here is read off control
flow, not a printed line; labeling that as the evidence basis, not a
stronger claim.

Why the round query comes back empty: between the "saved world is back"
restore (`nova_world_base::save::transients`, logged the instant the full
125-sector stream finishes) and the `"check the resumed state"` step, the
load script runs "shoot the first restored frame" (disk write) and "report
the pixel difference" (decode + diff two PNGs) first. Measured gap in both
runs: run 2, restore at `04:16:47.045` to assert-step-begins at
`04:16:49.181` = **2.14s**; run 3, restore at `04:19:09.038` to
assert-step-begins at `04:19:11.015` = **1.98s**. Every saved round's total
lifetime is 2.0s and its remaining-at-save was under 0.7s in both runs (see
table above), so EVERY round - including the newest - is mathematically
guaranteed to expire before this gap closes. This reproduced identically on
both runs that reached it; not a flake.

This is a real, reproducible finding, not a mechanical bug in this round's
reordering - decisions 1-3 did not touch the load script's own step order,
and nothing in my scope (no new fn/type/const, CREATE-script reordering
only) covers restructuring it. Separately flagging a second, pre-existing
issue the 200+-round count exposes: `fixture_transients_restored`'s and
`assert_resumed_state`'s round matching is "first found of the kind", not a
stable index - the file's own comment ("each marker is known to be unique in
this run... matching by kind IS the deterministic mapping") was accurate
when exactly one round existed, but round 3 already found PDC bays fire
continuously while held (~200 rounds/run), so that uniqueness premise no
longer holds for the round kind specifically. Neither run got far enough to
observe whether this actually caused a WRONG match (both failed on zero
matches, not a wrong one), so this is reported as a latent gap, not a
confirmed mismatch.

The torpedo and piece assertions (decision 4's other two) were never
reached in either run, since the round check panics first in file order;
their fidelity is unverified this round beyond `fixture_transients_restored()`
succeeding (all three kinds existed at the moment that predicate was
checked) and the saved reference values in the table above.

### Screenshots

Both `world_resume-before-leave.png` and `world_resume-after-load.png`
(`/tmp/nova-pt9/capture-r4/`, run 3's versions - `NOVA_CAPTURE_DIR` was
shared across all 3 runs, each overwriting the last) frame the player ship
with no pause chrome and "World saved" still showing top-right, matching
the "plain gameplay" requirement. `before-leave.png` (33 fps) is a
chase-camera view from behind/above looking up the ship's dorsal spine,
with two bright muzzle streaks rising from the dorsal PDC cluster - visible
evidence of the continuous fire this round's reorder relies on.
`after-load.png` (5 fps, expected right after a full 125-sector re-stream
on lavapipe) is a markedly CLOSER, more top-down framing of the ship's bow
and engine - the two shots are not the same camera angle, worth noting
since the pixel-difference figure below is reported, never asserted, partly
for exactly this reason (per the function's own doc). No muzzle tracers are
visible in `after-load.png`, which is consistent with - not proof of, but
consistent with - the round-expiry finding above: by the time this frame
was captured every saved round's remaining lifetime had already run out.
Pixel-difference figures (reported, not asserted):

```
run 2: mean |channel diff| = 10.214 (0-255 scale)
run 3: mean |channel diff| = 10.198 (0-255 scale)
```

### Paths

- Logs: `/tmp/nova-pt9/run-r4-1.log`, `run-r4-2.log`, `run-r4-3.log`.
- Screenshots: `/tmp/nova-pt9/capture-r4/world_resume-before-leave.png`,
  `world_resume-leave-overlay.png`, `world_resume-load-screen.png`,
  `world_resume-status-line.png`, `world_resume-after-load.png` (all run 3's,
  shared capture dir).
- Kept sandboxes: run 2
  `target/example-profiles/system_world_resume-3528295-1791519347457891513/`
  (holds `state.4.ron`, `expected.json`); run 3
  `target/example-profiles/system_world_resume-3529017-1791519492196957295/`
  (same).

### Diff summary

The file is still untracked (`git status --porcelain` shows only `??
examples/systems/system_world_resume.rs` throughout this round; nothing
staged, no other tracked file touched - `nix develop --command rustfmt
--edition 2024` run after every edit). No `git diff --stat` baseline exists
for an untracked file, so summarizing by section instead: `FireHeld`'s
fields and `hold_fire_inputs` (combat/pdc doc + body rewritten per the
owner's mid-round answer), `fire_transient_fixture` (now arms `combat` too,
doc rewritten), `create_script`'s fire block (one step split into four: A
launch torpedo+kill, B fire PDC, C release+shoot merged, D open pause menu
- same as before, unchanged), the long comment above step A (rewritten per
decision 3), and the `write_expected_state` step (moved, decision 1). No
other function signature changed; no fn/type/const added or removed. Build
clean: `cargo build --example system_world_resume --features debug` passes
after every edit in this round.

### Handoff

3 of 3 runs used. Two things need the owner's call before another round:

1. **The round's load-side expiry (blocking a clean PASS).** The round's
   2.0s authored lifetime cannot survive the load script's own
   restore-to-assert real-time cost (~2.0-2.1s measured, twice,
   reproducibly) on lavapipe - this is independent of decisions 1-3 and of
   how thin the CREATE-side margin is, since even the newest-fired round
   only had ~0.6s left at save time. Options I see, undecided: (a) extend
   the PDC round's authored `projectile_lifetime` for this fixture somehow
   (a content/design change, out of this round's scope), (b) move the
   load script's screenshot+pixel-diff steps to AFTER the transient
   assertions instead of before (a load-script reorder, which decisions
   1-3 never authorized me to touch), (c) assert the round more loosely
   (e.g., tolerate it already being despawned and skip its pose/lifetime
   check, the same shape as the piece's existing pose-not-asserted carve-out),
   or (d) something else.
2. **The round-matching latent gap.** Flagged above, not confirmed as a
   wrong match (both runs failed before any match was attempted), but
   worth a decision alongside (1) since fixing the expiry will surface it
   next: matching "the" round by kind alone is no longer unique once ~200
   rounds exist per run.

Xvfb :117 (PID 3526400) stopped and verified gone. Nothing staged, nothing
committed, only `examples/systems/system_world_resume.rs` touched.

## Round 5: the match-before-screenshot reorder, and why the 0.01s lifetime
tolerance cannot hold (2026-10-09)

Scope held: only `examples/systems/system_world_resume.rs` edited, plus this
report. No production file touched. Owner-approved items only: decision 1's
load-order reorder, decision 2's per-kind one-to-one match replacing "first
of kind". No new named fn/type/const was added - the matcher and its saved-
record reader are local closures inside the new step's `on_enter`, the same
`let resolve = |..| ..; let decode = |..| ..;` local-closure idiom
`report_pixel_difference` already uses. Per decision 3, no matching
tolerance was loosened and no production API was added when the proof
failed; this round stops and reports, as instructed.

**Status: blocked on a measured, reproduced timing conflict between decision
1 and decision 2, not a mechanical bug. 2 of the 3 allowed runs used; the
3rd was not spent, since both runs already reproduce the same structural
finding and no code change in this file can act on it without new owner
guidance.**

### 1. Decision 1 - the load-order reorder, and what it answers

`load_script` (`:1492` onward) now runs a new step, "world_resume load:
match and assert the resumed transients", immediately after "wait for the
fixture transients to resume" and BEFORE "shoot the first restored frame"
and "report the pixel difference across the leave". `assert_resumed_state`
(`:1287`) no longer touches the three transients at all - its body now only
asserts player pose, credits, stock, rock, canister and sector, still
running after the screenshot/diff as before, per the owner's "may stay where
they are" option. The expiry wait (`fixture_transients_expired()`) stays
last, unchanged.

**Does this on_enter run before the first physics tick after the restore?
No - confirmed twice, not just reasoned from source.** Evidence:

- `restore_resumed_transients` is scheduled in `Update`, `after(NovaWorldSystems::Retire)`
  (`crates/nova_world_base/src/lib.rs:189-204`); it both spawns the
  transients AND releases `FreezeOwner::WorldResume` in the same exclusive
  call (`spawn_resumed` then `end_resume`, `crates/nova_world_base/src/save/transients.rs:425-429`).
- `autopilot_drive` is scheduled in `PreUpdate`, `after(InputSystems)`
  (`crates/nova_autopilot/src/autopilot.rs:531`). Its own comment, "Entry
  gets its own frame, so the state transition has applied... before any
  predicate is polled" (`autopilot.rs:557-558`), and its code
  (`autopilot.rs:647-651`) confirm that the frame a step's `until` predicate
  FIRST returns true, the driver only advances `st.index` and sets
  `entered = false` - it does not call the new step's `on_enter` until the
  driver's NEXT call, i.e. the FOLLOWING frame's `PreUpdate`.
- So between the `Update` that spawns the transients and the `PreUpdate`
  that finally runs this step's `on_enter`, the schedule has already run, at
  minimum: that frame's `PostUpdate`/`Last`, the next frame's full
  `First -> PreUpdate -> RunFixedMainLoop` (a physics tick) `-> Update` (a
  full `update_temp_entities` lifetime decrement plus any body movement),
  `PostUpdate`/`Last`, and only THEN the following frame's `PreUpdate`
  reaches `on_enter`. At least one physics tick and one lifetime-decrement
  `Update` land on the just-restored bodies first.
- Measured, not just reasoned: both runs below log
  `nova_world_base: the saved world is back with N transient(s)` and then
  `autopilot: step \`world_resume load: match and assert the resumed
  transients\` begins`. Run 1: `04:36:33.014418` -> `04:36:33.385118` =
  **0.3707s**. Run 2: `04:40:21.380566` -> `04:40:21.791739` = **0.4112s**.
  Both land in the same ~0.37-0.41s band - a real, reproduced cost, not a
  fluke - consistent with round 4's own finding that the first frame after a
  full 125-sector restream on lavapipe is unusually slow (its "33fps ->
  5fps" observation).

### 2. Decision 2 - the one-to-one matcher

`record_transient_fixture_state` (`:725` onward, an existing fn, body
rewritten) now collects EVERY round, torpedo and piece record from the
leave save via `open_world` (never hand-parsed RON) into JSON arrays under
`expected["transients"]["round"|"torpedo"|"piece"]`, instead of one
representative of each kind. Every round's and torpedo's saved owner is
asserted `"player"` for ALL records, not just the first.

The new load-phase step's `on_enter` closure defines a local `match_kind`
matcher (and `read_saved`/`live_owner` helpers), reused for all three kinds:
for each saved record, candidates are live bodies not already used whose
owner equals the saved owner (`ProjectileOwner` -> the owner's `EntityId`,
compared by its string, for Round/Torpedo; no owner field for
DetachedPiece, matching the production type), translation is within 1.0 m,
and remaining lifetime (`SavedLifetime::of`) is within 0.01 s. Zero or more
than one candidate is a loud panic naming the kind, the record index, its
saved values, and - since the failure path collects the closest same-owner
live body before panicking, rather than stopping at the first failure -
every failing record and how far off its closest candidate actually was,
not just the first. The "matching by kind IS the deterministic mapping"
comment is gone (removed in both the module doc and `fixture_transients_restored`'s
doc per decision 2's instruction). No pose was hand-parsed from RON: Round's
pose comes from `FrozenRoundFlight::translation` (a public field, same as
before), Torpedo's from `FrozenTorpedo::translation` (public field), and
piece's from the existing `FrozenDetachedPiece::translation()` accessor the
main worker added in round 3 - no new accessor was needed, so I did not ask.

### 3. What actually happened: two runs, two reproductions of the same
structural conflict

Run 1 (272 saved rounds) panicked on the third Round record with the
original (first-failure) panic message: `0 live candidate(s), expected
exactly 1`. I then enriched the matcher (still the same closure, no new
named item) to collect every failing record's diagnosis before panicking
once, so one run would name the full pattern instead of just the first
record - this is diagnostic enrichment of an already-approved panic path,
not a loosened match (the pass/fail criteria are byte-for-byte the same;
only the failure MESSAGE grew richer). Run 2 (202 saved rounds) used that
enriched matcher and is the one quoted below.

Run 2's `Round` match failed on **193 of 202** records. Three things, all
independently measured from that run's own `expected.json` and panic
output, explain why:

1. **The 202 saved rounds collapse to only 3 distinct remaining-lifetime
   values**: `0.2637s` (what the panic's first failures showed), `0.4310s`,
   `0.6209s` - confirmed by reading `expected.json` directly
   (`round count 202`, `distinct remaining values: 3`). Up to ~103 records
   share one value. Lifetime alone cannot discriminate within a group that
   size, regardless of tolerance.
2. **Position does not discriminate the groups either**: the "closest
   same-owner live body" pose delta across the run's 193 failures ranged
   from **1.0968 m to 46.0996 m** - the PDC round travels fast enough that
   even same-instant siblings end up many metres apart by the time this
   runs, and a fast-moving body's OWN position at matching-time is already
   many metres from its OWN saved position (see point 3), so 1.0 m is not
   enough margin for this kind specifically.
3. **The lifetime delta itself clusters into exactly the 3 saved-value
   buckets' matching decay amounts**: `0.0070s` (86 records - inside the
   0.01s tolerance), `0.0296s` (4 records), `0.1969s` (103 records, over
   19x the tolerance). Even the `0.0070s` bucket - which passes the
   lifetime check on its own - still fails to pair 1:1, because up to ~86
   OTHER records share that same bucket and the position check (point 2)
   cannot break the tie reliably at a fast round's travel speed.

None of this is volume-only: even the records in the tightest, least-decayed
bucket (closest to the ~0.37-0.41s restore-to-match gap measured in section
1) still fail, because a PDC round moves far enough in that time that 1.0 m
of pose tolerance is not enough to re-identify it, and because many rounds
share that same decay bucket. The core conflict is decision 1 vs decision
2: decision 1 places the match at the first on_enter after the restore,
which section 1 shows is itself ~0.37-0.41s after the restore (never
sooner, by nova_autopilot's own design) - and decision 2's 0.01s lifetime
tolerance has no margin against a gap roughly 40x its size. This conflict
does not depend on the round's authored 2.0s lifetime being short, or on
~200 rounds existing: a torpedo or piece with a long lifetime would ALSO
show a lifetime delta of ~0.4s at this point, which is also outside 0.01s -
the match step never got there to confirm it this round (Round panics
first, in kind order), so torpedo/piece fidelity is UNVERIFIED this round,
flagged as such, not claimed.

### Runs (2 of 3 allowed, foreground, owned `Xvfb :117`, lavapipe)

Env shape for both: `DISPLAY=:117`,
`VK_DRIVER_FILES=/nix/store/1vkzwrp4ny20iljfnv9valq0bfn8czlv-mesa-26.2.3/share/vulkan/icd.d/lvp_icd.x86_64.json`,
`ALSA_CONFIG_PATH=/tmp/nova-pt9/alsa-empty.conf` (empty file),
`NOVA_AUTOPILOT=1`, `NOVA_CAPTURE=1`, `NOVA_CAPTURE_DIR=/tmp/nova-pt9/capture-r5`,
`NOVA_AUTOPILOT_DEADLINE=900`, `RUST_LOG=info,nova_ship=debug`,
`CARGO_TARGET_DIR=./target`, `cargo run --example system_world_resume
--features debug -j 8 -- --phase create`. One `Xvfb :117 -screen 0
1920x1080x24`, real PID `3532663` (verified via `pgrep -af "Xvfb :117"`,
not a backgrounding subshell's `$!`), started once and reused across both
runs. Stopped by that exact PID after run 2 and verified gone: `ps -p
3532663` exited 1 (no such process).

1. Original (first-failure) matcher: create phase saved 275 transients
   (`DetachedPiece` x1, `Torpedo` x2, `Round` x272), load phase reached the
   new match step and panicked on `Round record 2` (0 candidates). Create
   process then panicked on the load phase's exit status 101, as designed
   (`run_create`'s own assertion that the load phase exits clean). Log:
   `/tmp/nova-pt9/run-r5-1.log`. Sandbox kept:
   `target/example-profiles/system_world_resume-3532705-1791520537060556678/`
   (`state.4.ron`, 493463 bytes).
2. Enriched (collect-all-failures) matcher, same fixture/script otherwise:
   create phase saved 205 transients (`DetachedPiece` x1, `Torpedo` x2,
   `Round` x202), load phase's match step failed 193 of 202 Round records,
   full per-record diagnosis in the log (see section 3). Log:
   `/tmp/nova-pt9/run-r5-2.log`. Sandbox kept:
   `target/example-profiles/system_world_resume-3534086-1791520765029492582/`
   (`state.4.ron`, 468379 bytes; `expected.json` at this sandbox's root, not
   under `worlds/probe-world/`).

### Per-kind counts, deltas, expiry, pixel figure, screenshots - as asked,
labeled where unverified

- **Per-kind saved and live counts**: Round - saved 202, live 202 (run 2;
  the overall `live.len() == saved.len()` assert never fired, so the live
  count matched the saved count going in - nothing had despawned yet).
  Torpedo and DetachedPiece - saved 2 and 1 in both runs (from the "the
  leave save holds..." log line); their LIVE counts and matches are
  **unverified** this round, since the Round match panics first in kind
  order and the step never reaches them.
- **Max translation and lifetime deltas per kind**: Round only (torpedo and
  piece unverified, as above) - these are "closest candidate" diagnostic
  deltas, NOT real 1:1-matched pairs (the match never succeeded for 193 of
  202 records): max pose delta **46.0996 m**, min **1.0968 m** (both far
  over the 1.0 m tolerance); lifetime delta buckets **0.0070s** (86
  records, inside tolerance on its own), **0.0296s** (4), **0.1969s** (103,
  ~20x tolerance). The 9 records NOT in the failure list matched cleanly
  (exactly 1 candidate, both criteria satisfied) - their actual deltas were
  not separately logged this round; flagging that gap for anyone who reruns
  this with the current code.
- **Before-physics-tick evidence**: see section 1 - NO, measured at
  0.3707s (run 1) and 0.4112s (run 2) after the restore, not before it.
- **Expiry step result**: not reached, either run (the match step panics
  first).
- **Final PASS line or precise failure**: no PASS. Precise failure (run 2):
  `world_resume load: Round had 193 of 202 record(s) that did not pair
  one-to-one with a live body` (full per-record breakdown in
  `/tmp/nova-pt9/run-r5-2.log`).
- **Pixel figure**: not computed, either run - the screenshot and
  pixel-diff steps now run AFTER the match step (decision 1), which panics
  first.
- **What both PNGs show**: only `world_resume-before-leave.png`,
  `world_resume-leave-overlay.png`, `world_resume-load-screen.png` and
  `world_resume-status-line.png` exist in `/tmp/nova-pt9/capture-r5/` (all
  from steps that ran before the panic). `before-leave.png` frames the
  player ship with visible muzzle tracers, matching round 4's description.
  `world_resume-after-load.png` does **not exist** - the load phase never
  reached that step - so there is no after-load frame to describe or diff
  this round.
- **State file size and log paths**: run 1 `state.4.ron` 493463 bytes
  (`target/example-profiles/system_world_resume-3532705-1791520537060556678/worlds/probe-world/state.4.ron`);
  run 2 `state.4.ron` 468379 bytes (same path shape under its own sandbox
  id). Logs: `/tmp/nova-pt9/run-r5-1.log`, `/tmp/nova-pt9/run-r5-2.log`.

### Diff summary

File is still untracked (`git status --porcelain` shows only `??
examples/systems/system_world_resume.rs` and `?? tasks/20261007-090756/SW-PT9.md`
throughout this round; nothing staged, no other tracked file touched -
confirmed against the pre-existing, unrelated `M`-modified files already in
this worktree before this round started). Changes, by section: the module
doc's "Phase \`load\`" paragraph and `fixture_transients_restored`'s doc
(uniqueness claims removed, per decision 2); `record_transient_fixture_state`
(rewritten to collect every record per kind instead of the first, with a
local `owner_id` closure replacing the old `format!("{:?}", ..)` owner
string); `assert_resumed_state` (the ~120-line transient block removed,
replaced with a one-line note pointing at the new step); `load_script` (one
new step inserted between the existing "wait for the fixture transients to
resume" and "shoot the first restored frame" steps, containing the
`match_kind`/`read_saved`/`live_owner` local closures). No function
signature outside these edits changed; no fn/type/const added or removed.
`cargo build --example system_world_resume --features debug` passes after
every edit; `rustfmt --edition 2024 --check` reports clean on the final
version.

### Handoff

2 of 3 runs used; the 3rd was withheld deliberately (see Status). This is
blocked on a design decision, not a mechanical bug: decision 1's match
point (first on_enter after the restore) and decision 2's 0.01s lifetime
tolerance conflict with a measured, reproduced ~0.37-0.41s real-time cost
that is not under this file's control (nova_autopilot's own one-frame-late
step-entry design, compounded by lavapipe's slow first frame after a full
125-sector restream). Separately, even once/if that gap is addressed, the
Round kind's ~200-strong volley collapses to only 3 distinct saved lifetime
values and spans up to 46 m of position spread among same-bucket siblings,
so lifetime+translation cannot uniquely re-identify most individual rounds
at today's fixture volume regardless of timing.

Per the owner's item 3, I did NOT loosen the 1.0 m / 0.01 s tolerances and
did NOT add a production API to work around either finding. Proposed
redesign options, examples-file-only, all still needing a decision:

- **A. Shrink the round volley.** `fire_transient_fixture`/the PDC-fire step
  currently holds the raw mouse-button press across however many frames the
  "a round now exists" predicate takes to resolve (itself at least one
  frame late, per section 1's mechanism), and EVERY deployed bay fires on
  every one of those frames - there is no per-bay fire control available
  through the raw-input harness idiom (the trigger is a global device-state
  read by every deployed bay's own section, not a bay-specific action; see
  the original plan's section 1, "not aimed at one specific bay"). Pressing
  for the fewest possible frames (e.g. exactly one, then release
  immediately) would shrink the volley from ~200 toward roughly one round
  per currently-deployed bay (~5-6), cutting the same-bucket collision rate
  a great deal. This does NOT by itself fix the lifetime-tolerance conflict
  in section 1/3, since even a single round still decays ~0.4s before the
  match runs - it only narrows the position/lifetime bucketing problem.
- **B. Accept that Round, Torpedo and DetachedPiece fidelity cannot be
  checked at the exact "first on_enter after the restore" point under a
  0.01s lifetime tolerance, and ask whether the tolerance itself should move
  (not loosen past the engine's own measured minimum gap, but acknowledge
  it) OR whether the match should run at a different point in the load
  script that decision 1 did not consider - both are design decisions I
  should not make myself per the owner's own instruction, surfaced here
  rather than guessed.
- **C. Something else** - these are the two angles this round's evidence
  points at; there may be a redesign I have not seen.

I did not pick one or combine any of these without asking.

## Round 6: thaw-time observer replaces the on_enter match, and the ambiguity
it exposes is spatial, not temporal (2026-10-09)

Scope held: only `examples/systems/system_world_resume.rs` edited, plus this
report. No production file touched. Owner-approved items only: an
`On<Insert, TempEntityState>` observer, registered once in `run_load`,
gated on the public `WorldResumeProgress` (`nova_world_base` prelude),
capturing each resumed round/torpedo/piece's owner, translation and
remaining lifetime the instant `spawn_resumed` gives it that component;
the "match and assert" step's `on_enter` now matches the saved records
against those RECORDED entries instead of a live world query; the create
phase's PDC step now holds the trigger for exactly one frame (a bare step,
per `nova_autopilot::predicate::frames`'s own doc: "`frames(1)` ... and no
`.until` at all advance together. Write the bare step"), with the release
step gaining the `.until()` the merged step no longer needed. No new named
fn/type/const: the observer and the match step's helpers are inline
closures; the shared recorder is an `Arc<Mutex<Vec<(&'static str,
Option<String>, Vec3, f32)>>>` threaded through `load_script`'s existing
signature as a new parameter, not a new type. Per decision 3, no matching
tolerance was loosened and no production API was added when the proof
still failed - this round stops and reports, as instructed.

**Status: the observer itself works exactly as designed - every recorded
thaw-time entry's lifetime matches its saved record to 0.0000s, proving the
callback sees the full pose, owner and lifetime with no decay. The proof
still does not PASS, reproducibly, 2 of 2 runs: not from timing anymore, but
from a SPATIAL ambiguity the timing fix exposed - a single-frame PDC press
fires every deployed bay at once, and several of those bays' muzzles sit
within 1.0m of each other, so position+lifetime still cannot re-identify
every round 1:1 at the approved tolerances.**

### 1. The observer: built as specified, and it sees everything asked

`run_load` (before `app.add_plugins(load_script(...))`) now builds one
`Arc<Mutex<Vec<(&'static str, Option<String>, Vec3, f32)>>>` and registers
one `app.add_observer(move |insert: On<Insert, TempEntityState>, progress:
Option<Res<WorldResumeProgress>>, bodies: Query<(&TempEntityState,
Option<&Transform>, Option<&ProjectileOwner>,
Has<TurretBulletProjectileMarker>, Has<TorpedoProjectileMarker>,
Has<DetachedPieceMarker>)>, owners: Query<&EntityId>| ...)`. It returns
immediately if `WorldResumeProgress` is absent (gates out anything outside
a Load - `ResumedTransients` itself is `pub(crate)`,
`crates/nova_world_base/src/save/transients.rs:169`, so
`WorldResumeProgress` is the public seam this file can gate on; it is
inserted in `hold_resumed_transients` before the window streams in and
removed only in `end_resume`, after `spawn_resumed`'s own `state.apply`,
transients.rs:768-774) and returns if the inserted entity carries none of
the three markers (filters out RockChunk/ShedFixture thaws, which also
carry `TempEntityState`). For a match, it reads `Transform` directly (not
avian's `Position`/`Rotation`): confirmed by reading `thaw_round_flight`
(`crates/nova_gameplay/src/rounds.rs:295-318`), `spawn_torpedo`
(`crates/nova_ship/src/sections/torpedo_section/bay.rs:280-302`) and
`thaw_detached_piece` (`crates/nova_ship/src/sections/frozen_piece.rs:645-646`)
- all three insert `Transform` as a direct bundle member in the SAME
command as the entity's spawn, before `spawn_resumed`'s own loop
(transients.rs:707-736) queues that entity's `resumed_lifetime` insert as a
separate, later command on the same buffer; avian's `Position` sync runs on
a later schedule pass this observer's synchronous command-application does
not wait for, so `Position` would not yet be correct at this instant -
`Transform` is the only pose source guaranteed landed. Remaining lifetime
reads `TempEntityState::remaining_secs()` straight off the `Query` (the
same math `SavedLifetime::of` does internally) rather than calling
`SavedLifetime::of` itself, since that free function takes `&World` and an
observer cannot combine `&World` with the `Query`/`Res` params this closure
also needs.

Evidence the callback sees the full pose, owner and lifetime, not a partial
or stale read: every single failure line in both runs below reports
`lifetime delta 0.0000s` - the recorded remaining lifetime equals the saved
remaining lifetime to the printed 4 decimal places, every time, for every
record. This is the result this round set out to get: Round 5's whole
blocker was a ~0.37-0.41s gap between the restore and the match; capturing
at the actual thaw event removes that gap entirely. No STOP was needed
under the "cannot see the full pose, owner and lifetime" clause - it saw
all three, precisely.

### 2. The create-phase fire step: one-frame hold, confirmed to still fire

The PDC step (`world_resume: fire a PDC round`) is now a bare step - entry
sets `FireHeld.pdc = true`, no `.until()`, so it advances on the very next
poll (one frame of `hold_fire_inputs`' press). The round volley shrank from
round 5's ~200-272/run to **12** (run 1) and **22** (run 2) - both runs
confirm a round exists by the time the release step's own
`.until(any_entity::<With<TurretBulletProjectileMarker>>())` resolves, so
the "no round ever fires" STOP condition in the owner's item 4 was never
hit. The volley did not shrink to "about one round per bay" as round 5's
option A speculated; see section 3 for why.

### 3. What actually happened: the SAME structural failure, twice, for a
NEW reason

Both runs panic in `match_kind`'s own loud, named path (zero or more than
one candidate - here, always exactly 2), never silently. In both runs every
failing record's saved and recorded remaining lifetime are IDENTICAL
(`0.7918s` in run 1, `0.8065s` in run 2, for every one of that run's
rounds) - a single LMB press-and-release this frame fires EVERY deployed
PDC bay (and, from the screenshot below, more than one muzzle per bay) in
the same tick, not one representative round. With lifetime unable to
discriminate at all (every candidate ties), the match falls entirely on the
1.0m position tolerance - and several bays/muzzles sit close enough
together (closest failing candidate pose deltas as low as **0.0000m**,
i.e. two recorded entries both sit within 1.0m of one saved pose) that a
saved record's TRUE match (delta 0.0000m, the thaw copied the saved
translation verbatim - confirmed in both runs' "closest" diagnostics) is
not the ONLY candidate inside 1.0m; a neighboring bay's round is too. This
is spatial, not temporal: round 5's finding was that the match ran too late
(lifetime had decayed ~0.4s by the time it ran); this round's observer
closes that gap to 0.0000s and the SAME kind of ambiguity still happens,
now caused by how close together the ship's PDC muzzles are mounted, not by
when the match runs.

### Runs (2 of 3 allowed, foreground, owned `Xvfb :117`, lavapipe)

Env shape for both: `DISPLAY=:117`,
`VK_DRIVER_FILES=/nix/store/1vkzwrp4ny20iljfnv9valq0bfn8czlv-mesa-26.2.3/share/vulkan/icd.d/lvp_icd.x86_64.json`,
`ALSA_CONFIG_PATH=/tmp/nova-pt9/alsa-empty.conf` (empty file),
`NOVA_AUTOPILOT=1`, `NOVA_CAPTURE=1`, `NOVA_CAPTURE_DIR=/tmp/nova-pt9/capture-r6`,
`NOVA_AUTOPILOT_DEADLINE=900`, `RUST_LOG=info,nova_ship=debug`,
`CARGO_TARGET_DIR=./target`, `cargo run --example system_world_resume
--features debug -j 8 -- --phase create`. One `Xvfb :117 -screen 0
1920x1080x24`, real PID `3537933` (verified via `pgrep -af "^Xvfb :117"`
after a first bare `pgrep -af "Xvfb :117"` again false-matched my own shell
wrapper, same trap round 4 hit and caught), started once and reused across
both runs. Stopped by that exact PID after run 2 and verified gone: `ps -p
3537933` exited 1 (no such process).

1. Create phase saved 15 transients (`DetachedPiece` x1, `Torpedo` x2,
   `Round` x12) - the one-frame tap's smallest volley. Load phase's match
   step: recorded Round count 12 matched saved Round count 12 (the
   `assert_eq!` ahead of the per-record loop never fired), then failed 8 of
   those 12 records, panic at `system_world_resume.rs:1591`. Log:
   `/tmp/nova-pt9/run-r6-1.log`. Sandbox kept:
   `target/example-profiles/system_world_resume-3537961-1791521973835254562/`
   (`state.4.ron` 400642 bytes, `expected.json`).
2. Same code: create phase saved 25 transients (`DetachedPiece` x1,
   `Torpedo` x2, `Round` x22). Load phase's match step: recorded 22 matched
   saved 22, then failed 18 of 22, same panic line, same `0.0000s` lifetime
   delta on every failure. Log: `/tmp/nova-pt9/run-r6-2.log`. Sandbox kept:
   `target/example-profiles/system_world_resume-3538606-1791522111542925873/`
   (`state.4.ron` 404234 bytes, `expected.json`).

### Per-kind counts, deltas, expiry, pixel figure, screenshots - as asked,
labeled where unverified

- **Recorded vs saved counts per kind**: Round - run 1: recorded 12, saved
  12 (equal; the `assert_eq!` that would name a mismatch never fired).
  Run 2: recorded 22, saved 22 (same). Torpedo and DetachedPiece - saved 2
  and 1 in both runs (`expected.json`, confirmed by direct read); their
  RECORDED and live counts are **unverified** this round, since the Round
  match panics first in kind order and the step never reaches the
  `thawed("torpedo")`/`thawed("piece")` calls or their
  `assert_live_at_most_recorded` checks.
- **Max translation and lifetime deltas per kind**: Round only (torpedo and
  piece unverified, as above) - these are "closest candidate" diagnostic
  deltas for the FAILING records, not real matched pairs (the match never
  succeeded for 8 of 12 / 18 of 22): lifetime delta is **0.0000s on every
  single failure, both runs, no exceptions** - the strongest evidence this
  round produced. Pose delta: run 1 ranges **0.0000m to 6.3128m**; run 2
  ranges **0.0000m to 6.4733m** (both far over the 1.0m tolerance on the
  high end; the LOW end, near-0.0000m, is exactly what makes the match
  ambiguous - a second candidate is often AS close as the true one). The 4
  records (run 1) / 4 records (run 2) not in the failure list matched
  cleanly (exactly 1 candidate); their actual deltas were not separately
  logged this round, the same gap round 5 flagged for anyone who reruns
  this with the current code.
- **Expiry step result**: not reached, either run (the match step panics
  first, same as round 5).
- **Final PASS line or precise failure**: no PASS. Precise failure (run 2,
  the larger/clearer reproduction): `world_resume load: Round had 18 of 22
  record(s) that did not pair one-to-one with a recorded thaw-time
  entry` (full per-record breakdown, including every "2 candidate(s)"
  diagnosis, in `/tmp/nova-pt9/run-r6-2.log`).
- **Pixel figure**: not computed, either run - the screenshot and
  pixel-diff steps still run AFTER the match step, which panics first.
- **What both PNGs show**: only `world_resume-before-leave.png`,
  `world_resume-leave-overlay.png`, `world_resume-load-screen.png` and
  `world_resume-status-line.png` exist in `/tmp/nova-pt9/capture-r6/` (all
  from steps that ran before the panic, run 2's versions - the capture dir
  is shared across both runs, each overwriting the last).
  `world_resume-before-leave.png` (34 fps) frames the player ship from
  behind/above: FOUR distinct bright tracer streaks rise from the dorsal
  hull, in two visibly separate pairs (left pair, right pair), each pair's
  two tracers originating within roughly a muzzle-width of each other -
  direct visual confirmation of section 3's finding, multiple muzzles
  firing in the same single-frame press, close enough together to collide
  with a 1.0m match tolerance. `world_resume-after-load.png` does **not
  exist** - the load phase never reached that step, same as round 5.
- **State file size and log paths**: run 1 `state.4.ron` 400642 bytes
  (`target/example-profiles/system_world_resume-3537961-1791521973835254562/worlds/probe-world/state.4.ron`);
  run 2 `state.4.ron` 404234 bytes (same path shape under its own sandbox
  id). Logs: `/tmp/nova-pt9/run-r6-1.log`, `/tmp/nova-pt9/run-r6-2.log`.

### Diff summary

The file is still untracked (`git status --porcelain` shows only `??
examples/systems/system_world_resume.rs` and `?? tasks/20261007-090756/SW-PT9.md`
throughout this round; nothing staged, no other tracked file touched).
Changes, by section: the module doc's "Phase `load`" paragraph (now
describes matching against a thaw-time recorded entry, not a live body, and
drops the earlier "On that first frame" wording that implied the match ran
inside the restore frame); `create_script`'s PDC step (now bare, one frame,
comment rewritten to cite `frames`'s own "write the bare step" doc) and its
following release step (gained the `.until()`/`.deadline()` the merged step
used to hold); `load_script`'s signature (one new parameter, the shared
`Arc<Mutex<Vec<(&'static str, Option<String>, Vec3, f32)>>>` recorder - not
a new type); the "match and assert" step's `on_enter` (the live-world
`round_live`/`torpedo_live`/`piece_live` queries and the `live_owner`
closure are gone; `match_kind` now takes a `recorded` slice instead of a
`live` slice of `(Entity, ...)` tuples, deduplicating by index instead of
`Entity`; a new `thawed` closure reads one kind out of the shared recorder;
a new `assert_live_at_most_recorded` closure checks the on_enter-time live
count never exceeds what the thaw recorded); `run_load` (the new
`Arc<Mutex<...>>` and its `app.add_observer(...)` registration, placed
before `app.add_plugins(load_script(...))`, with `recorded` now passed
into `load_script`). No other function signature changed; no fn/type/const
added or removed - every new binding is a `let`-bound closure or a plain
value. Build clean: `cargo build --example system_world_resume --features
debug -j 8` passes after every edit. `rustfmt --edition 2024 --check`
reports clean on the final version (`nix develop --command rustfmt
--edition 2024` was run once, right after the edits, same as every prior
round).

### Handoff

2 of 3 runs used; the 3rd was withheld deliberately, the same way round 5
did - both runs already reproduce the identical structural shape (every
failure's lifetime delta exactly 0.0000s, several pose deltas near 0.0000m
alongside the true match) at two different volley sizes (12 and 22
rounds), so a third run would not add a new fact, only another sample of
the same one. Per the owner's item 3, I did NOT loosen the 1.0m/0.01s
tolerances and did NOT add a production API to work around this. This
round answers decision 1's original question cleanly: thaw-time capture
removes the timing gap entirely (0.0000s lifetime delta, always) - the
remaining blocker is that several of the ship's PDC bays mount their
muzzles within about a metre of each other, and a single press fires all of
them in the same tick, so pose+lifetime at ANY capture point cannot
re-identify them 1:1 at today's tolerances. This needs the owner's call,
not a guess: (a) an index/order-based match instead of (or alongside)
pose+lifetime - the thaw loop and the save both iterate the SAME saved
order, so the Nth recorded round-kind entry already pairs with the Nth
saved round-kind record positionally, with no geometry involved at all; (b)
fire from only ONE bay (not a global trigger-key press) if the harness idiom
allows aiming the press at a single section - round 5's option A already
noted the raw-input harness idiom has no per-bay fire control; (c)
something else. I did not pick or combine any of these without asking.

Xvfb :117 (PID 3537933) stopped and verified gone. Nothing staged, nothing
committed, only `examples/systems/system_world_resume.rs` touched (plus
this report).

## Round 7: pair by stable save-list index - the match step itself now
passes clean, 0.0000m/0.0000s, both times it ran (2026-10-09)

Scope held: only `examples/systems/system_world_resume.rs` edited, plus
this report. No production file touched. Owner's decision, used as given:
pair each thaw-time recorded entry against its saved record by stable
save-list index (the Nth `Insert<TempEntityState>` the observer sees is
always the Nth saved transient - `spawn_resumed`,
`crates/nova_world_base/src/save/transients.rs:707-736`, queues every
saved transient's spawn then its `resumed_lifetime` insert, in saved-list
order, on one command buffer applied once). No new named fn/type/const:
every new piece is a `let`-bound closure or a plain value, same style as
round 6.

### 1. What changed

- `record_transient_fixture_state` (`system_world_resume.rs:731`) no
  longer groups saved records by kind into three separate arrays. It now
  returns ONE ordered JSON array mirroring `state.transients` exactly - one
  object per saved transient, in save order, each tagged `"kind"`
  (`"round"` / `"torpedo"` / `"piece"` / `"other"`), carrying `owner`
  (`"player"` for round/torpedo, `null` for piece/other, validated the
  same way round 6 did) and `pose/lifetime_remaining` where that kind
  defines them (`null` pose for `"other"`). This is what lets the load
  phase pair by index across the WHOLE list, not just within one kind's
  own sub-list.
- The thaw-time observer in `run_load` (`system_world_resume.rs:1659`) now
  tags an insert that carries none of the three checked markers as
  `"other"` instead of returning early - every resumed transient is now
  recorded, not just round/torpedo/piece, so the recorded list's length
  and order always mirror the saved list's (this scenario never actually
  produces a rock chunk or a shed fixture, so `"other"` never appears in
  practice here, but the recorder no longer silently drops a kind it
  wasn't told to check).
- The "match and assert" step's `on_enter`
  (`system_world_resume.rs:1471`) deleted `match_kind`, `read_saved`,
  `thawed` and the old `assert_live_at_most_recorded` (which compared
  against the thaw-time RECORDED count) entirely - no more candidate
  search, no `HashSet` of used indices. In their place: one
  `assert_eq!(recorded_entries.len(), saved.len(), ...)` naming both
  counts, then one `for (index, (saved_entry, recorded_entry)) in
  saved.iter().zip(recorded_entries.into_iter()).enumerate()` loop that
  asserts the recorded kind equals the saved kind at that index (panic
  names the index and both kinds), and - for `"round"`/`"torpedo"`/`"piece"`
  only - asserts owner equality, translation within 1.0 m and remaining
  lifetime within 0.01 s (every panic names the index, the kind, and both
  the saved and recorded values; `"other"` records are skipped after the
  kind check, per the owner's scope). A second, smaller
  `assert_live_at_most_saved` closure then checks each kind's live count
  at this on_enter against the SAVED count for that kind (not the
  recorded count, per the owner's item - a live body can have already
  expired between the thaw and this later frame, but it can never exceed
  what the leave save actually held). No tolerance was loosened - still
  1.0 m / 0.01 s, same comparison operators (`<`, `<`).
- The big round-6 comment above the match (citing `SW-PT9.md Round 5`'s
  ~0.4 s gap finding, explaining the one-to-one candidate search) is gone.
  The comment now reads: `spawn_resumed` queues each saved transient's
  spawn then its `resumed_lifetime` insert, in saved-list order, on one
  command buffer applied once, so the observer's Nth insert is always the
  Nth saved transient - the values compared are what that observer
  captured at the instant of that insert, not a live read on this later
  frame. Two sentences, states the ordering evidence and the thaw-time
  capture point, nothing else.
- The module doc's "Phase `load`" paragraph was rewritten to describe
  index-based pairing instead of content-matching.

Build clean: `cargo build --example system_world_resume --features debug
-j 8` passes after every edit. `rustfmt --edition 2024` was run once on
this file only, right after the edits; `rustfmt --edition 2024 --check`
reports clean on the final version.

### 2. Runs (3 of 3 allowed, foreground, owned `Xvfb :117`, lavapipe)

Env shape for all attempts: `DISPLAY=:117`,
`VK_DRIVER_FILES=/nix/store/1vkzwrp4ny20iljfnv9valq0bfn8czlv-mesa-26.2.3/share/vulkan/icd.d/lvp_icd.x86_64.json`,
`ALSA_CONFIG_PATH=/tmp/nova-pt9/alsa-empty.conf` (empty file),
`NOVA_AUTOPILOT=1`, `NOVA_CAPTURE=1`, `NOVA_CAPTURE_DIR=/tmp/nova-pt9/capture-r7`,
`NOVA_AUTOPILOT_DEADLINE=900`, `RUST_LOG=info,nova_ship=debug`,
`CARGO_TARGET_DIR=./target`, `cargo run --example system_world_resume
--features debug -j 8 -- --phase create`. One `Xvfb :117 -screen 0
1920x1080x24`, real PID `3541122` (verified via `pgrep -af "^Xvfb :117"`,
the same false-match trap rounds 4 and 6 hit was checked for and avoided),
started once and reused across all 3 attempts. Stopped by that exact PID
after the last attempt and verified gone: `ps -p 3541122` printed no
process line and exited 1.

Host load was shared with other agents' sessions this round
(`uptime` read 8.41 before the first attempt, 8.81/7.22/4.33 before the
last) - see below for the effect this had.

1. First attempt (log overwritten by attempt 2 below, same filename):
   stalled 180.1 s in the CREATE phase's own `world_resume: release the
   trigger and shoot the frame right before the leave` step, waiting for
   `any_entity::<With<TurretBulletProjectileMarker>>()` that never
   appeared - `create_script` was not touched this round, and this step's
   own deadline (`PDC_FIRE_DEADLINE_SECS`, 180.0 s, unchanged since round
   4) is the same one rounds 4-6 ran under without a stall. Read as host
   contention, not a regression: the load-side code this round actually
   changed never ran.
2. Second attempt (`/tmp/nova-pt9/run-r7-1.log`): create phase fired a
   12-round volley (same one-frame-press shape as round 6's run 1), saved
   15 transients (`Round` x12, `Torpedo` x2, `DetachedPiece` x1), spawned
   the load phase. Load phase's match step: **`world_resume load: matched
   15 transient(s) by stable save-list index - round max pose delta
   0.0000m/lifetime delta 0.0000s, torpedo max pose delta
   0.0000m/lifetime delta 0.0000s, piece max pose delta 0.0000m/lifetime
   delta 0.0000s`** - no panic, no failing index, the `assert_eq!` on
   `recorded_entries.len()`/`saved.len()` never fired either (both 15).
   This is the step this round exists to fix, and it passed completely:
   every one of the 15 saved records, across all three checked kinds,
   paired with its thaw-time entry at the SAME index with ZERO measured
   delta. Execution then continued three more steps (screenshot, pixel
   report, `assert_resumed_state`) before panicking on a DIFFERENT, later
   assertion - see section 3. Sandbox kept:
   `target/example-profiles/system_world_resume-3541811-1791522980904146306/`
   (`state.4.ron` 400658 bytes, `expected.json` 15 kind-tagged records: 12
   round, 2 torpedo, 1 piece, 0 other).
3. Third attempt (`/tmp/nova-pt9/run-r7-2.log`), the planned repeat: same
   stall as attempt 1, same step, same 180.1 s deadline - host load was
   higher at this point (`uptime` 8.81/7.22/4.33 vs. 8.41 before attempt
   1). The repeat could not re-exercise the match step because the
   create-phase fixture never got that far; it reproduces attempt 1's
   stall, not attempt 2's pass. The 3-run budget is now spent.

### 3. The downstream stock-assertion failure: pre-existing, unrelated,
out of this round's scope

Attempt 2's load phase panicked at `assert_resumed_state`
(`system_world_resume.rs`, the "resumed stock does not match the saved
stock" assert) with saved `PdcRound` 6000 vs. resumed `PdcRound` 5988 -
every other stock line (`HullPlate`, `RailSlug`, `Torpedo`, `Rations`)
matched exactly. This is the SAME shape of failure Round 3 first hit and
Round 4 believed it had fixed by moving `write_expected_state` to right
after "let escape go" (SW-PT9.md Round 4, section 1) - that fix holds for
a single fired round, but round 6's create-phase redesign (a bare
one-frame PDC press that lets every deployed bay/muzzle fire a 12-22-round
burst, SW-PT9.md Round 6 section 2) means the burst can still be
depositing rounds, and consuming `PdcRound` stock, for a few frames AFTER
the capture point - the 12-unit gap (6000-5988) matches this run's own
12-round volley exactly. This is NOT the match step this round was scoped
to fix (which passed, see section 2.2), is NOT something
`match_kind`/the index pairing touches, and fixing it would need a new
capture-point decision (move `write_expected_state` again, or something
else) that is outside this round's owner-approved items. Per scope, I did
not touch it.

### 4. What was asked for, with evidence, labeled where unverified

- **Diff summary**: section 1 above.
- **Recorded vs saved total and per-kind counts**: attempt 2, the only
  attempt that reached the match step - recorded 15 total (12 round, 2
  torpedo, 1 piece, 0 other), saved 15 total (same breakdown,
  `expected.json` read directly). Equal at every level; the whole-list
  `assert_eq!` never fired. Attempt 3's counts are **unverified** - it
  never reached this step (see section 2.3).
- **Max translation and lifetime deltas per kind**: attempt 2 - round,
  torpedo and piece ALL report max pose delta 0.0000m and max lifetime
  delta 0.0000s, straight from the log line quoted in section 2.2. Unlike
  round 6, these are real matched-pair deltas for every record, not
  closest-candidate diagnostics for failures - every index passed on its
  first and only candidate.
- **Expiry step result**: not reached, attempt 2 (panicked at the stock
  assertion, several steps before `world_resume load: the fixture
  transients expire on their own saved clocks`); not reached, attempt 3
  (stalled in the create phase, never spawned a load phase). **Unverified
  this round**, same gap round 6 had for a different reason.
- **Final PASS line or precise failure**: no PASS. Precise failure
  (attempt 2): `` assertion `left == right` failed: world_resume load:
  resumed stock does not match the saved stock / left: [...PdcRound 5988...]
  / right: [...PdcRound 6000...] `` (full arrays in
  `/tmp/nova-pt9/run-r7-1.log`) - a pre-existing, unrelated assertion (see
  section 3), not the transient-matching logic this round changed.
- **Pixel figure**: attempt 2 only - `mean |channel diff| = 10.041` (0-255
  scale), reported (never asserted) between `world_resume-before-leave.png`
  and `world_resume-after-load.png` as captured during attempt 2 itself.
- **What before-leave.png and after-load.png show**: both files on disk
  under `/tmp/nova-pt9/capture-r7/` were LOOKED AT with `Read`, but
  `NOVA_CAPTURE_DIR` is shared across all 3 attempts and each attempt's
  `on_enter` re-shoots on step entry regardless of whether that attempt
  then stalls - so the file timestamps matter: `world_resume-after-load.png`
  (mtime 08:17:18) and `world_resume-leave-overlay.png`/`world_resume-load-screen.png`
  (08:16) are from attempt 2, the one that reached those steps. But
  `world_resume-before-leave.png` and `world_resume-status-line.png` were
  BOTH overwritten by attempt 3 (mtime 08:19, after attempt 2 finished) -
  attempt 3 stalled waiting for a round that never existed, so its
  `before-leave.png` shows the ship from behind/above with the dorsal PDC
  bays visible and CLEAN, no tracer streak, no round - it is NOT
  attempt 2's pre-leave frame and does not show the 12-round volley.
  `world_resume-after-load.png` (attempt 2, genuine) shows the resumed
  ship head-on at 5 fps, "World saved" status visible, both PDC bay radar
  rings lit, and one small bright point above the dorsal hull near the
  left bay - consistent with a resumed round still in flight, though not
  verified pixel-by-pixel against a specific saved pose. Labeled:
  before-leave.png's content this round is unverified for attempt 2 and
  known-wrong for what it currently shows on disk.
- **State file size and log paths**: attempt 2's `state.4.ron` 400658
  bytes
  (`target/example-profiles/system_world_resume-3541811-1791522980904146306/worlds/probe-world/state.4.ron`).
  Logs: `/tmp/nova-pt9/run-r7-1.log` (attempt 2, the one that reached the
  match step), `/tmp/nova-pt9/run-r7-2.log` (attempt 3, stalled in create).
  Attempt 1's log was overwritten by attempt 2 under the same filename;
  its own stall is described in section 2.1 from direct observation
  before the retry.

### Handoff

The match step this round exists to fix now passes cleanly, with zero
measured deltas, the one time it ran under this round's own fixture -
pairing by stable save-list index resolves round 6's spatial ambiguity
completely, as the owner's evidence predicted: index order sidesteps the
close-muzzle collision entirely, since it never compares pose between
different saved records at all. The planned repeat (attempt 3) could not
confirm this a second time because host load stalled the CREATE phase's
unrelated PDC-fire step before the load phase ever ran, the same way
attempt 1 did - this round's 3-run budget is spent on 2 stalls and 1 pass,
not 2 passes, so the index-pairing fix's reproducibility is **only
single-run-confirmed**, not double-confirmed, through no fault in the
code touched this round. A clean re-run on a quieter host would most
likely repeat attempt 2's result, since nothing about the match logic is
load-sensitive (it is pure index/value comparison, no timing or geometry
involved), but that is a prediction, not a second measurement.

Separately, attempt 2 surfaced a real, reproducible-by-shape, pre-existing
gap: round 6's one-frame-press redesign broke round 4's stock-capture-point
fix for any run where the resulting burst is more than a couple of rounds.
This needs the owner's call, not a guess, the same way round 6's spatial
ambiguity did: (a) move `write_expected_state` later still, after the
burst has fully settled, if there is a reliable signal for "the burst is
done"; (b) special-case `PdcRound` to `resumed <= expected` the way round
3's option B proposed, now for a burst-sized gap instead of a single-round
one; (c) something else. I did not pick or combine any of these without
asking, and did not touch the stock assertion or the fire sequence.

Xvfb :117 (PID 3541122) stopped and verified gone. I did not stage or
commit anything myself (`git status --cached`/`git diff --cached` were
empty throughout). Note: an external `wip` commit (`9ce136a54`, author
Alex Jercan, 2026-10-09 08:20:13, 175 files) landed on this branch mid-run,
after my edits to `examples/systems/system_world_resume.rs` were already
made and formatted - it swept the whole worktree, including that file (its
committed content matches my edited version exactly, confirmed by `git
diff HEAD -- examples/systems/system_world_resume.rs` being empty) and the
pre-Round-7 state of this report. This commit was not made by me; flagging
it since it changes what "nothing committed" means for this round. Only
`examples/systems/system_world_resume.rs` was touched by me (plus this
report).

## Round 8: P6 authority, observer ordering audit, PDC-latch fix, 2 clean
repeats (2026-10-09)

Scope held: only `examples/systems/system_world_resume.rs` edited, plus
this report. No production file touched, nothing staged or committed. No
new named fn/type/const - every new piece below is a `let`-bound closure,
tuple or plain value.

### 1. What changed

- **P6 authority.** `record_transient_fixture_state`
  (`system_world_resume.rs`) now also builds a `saved_player` JSON object
  from the SAME `open_world()` read the transients already use: `pose`
  from `state.player.transform.translation`, `credits` from
  `header.credits`, `stock` from `serde_json::to_value(&state.player.ship)`
  walked to `["state"]["inventory"]["stacks"]` (sorted by item name - no
  new accessor needed, `FrozenShip` already derives `Serialize`). The
  create script's "record the leave save's own transients" step keeps the
  live pre-click sample only to log save-time deltas (`info!`), never to
  assert; a comment states this once.
- **The actual assertion target moved to thaw time**, on the owner's
  correction (the first `on_enter` after resume already lands ~0.4s of sim
  later - long enough for a saved reload timer to run, which run 1 of this
  round proved: live-read PdcRound 5908 vs saved 6000). The SAME inline
  `On<Insert, TempEntityState>` observer in `run_load` that records the
  transient list now also snapshots the player's `Transform` translation,
  `ShipCredits` and `ShipInventory::stacks()` on its first callback only,
  into a second `Arc<Mutex<Option<(Vec3, u32, Vec<(ItemType, u32)>)>>>`
  (`player_snapshot`) - panicking if more than one `PlayerSpaceshipMarker`
  ever matches. Pose reads `Transform`, not `Position`: avian only writes
  `Position` from `Transform` in `FixedPostUpdate`, which does not run
  while `WorldResume` holds the clocks (confirmed in avian3d 0.7.0 source,
  `src/physics_transform/mod.rs:106-110`, gated on Bevy's fixed-timestep
  accumulator, itself frozen with `Time<Virtual>`) - run 3 of this round
  hit exactly this (thaw-time `Position` read 0,0,0 against a saved
  3200,0,0 pose) before the fix. `Transform` is also what the save itself
  records (`SavedPlayer.transform`), so this is the apples-to-apples read,
  not a loosened one. The match step's `on_enter` asserts this snapshot
  (pose <1.0m, credits and stock exact) against `saved_player`, and
  `assert_resumed_state` no longer re-checks player pose/credits/stock at
  all - removed, so the same claim is never checked twice against state
  that may have already drifted live.
- **Observer ordering audit.** The match step still fails loudly on a
  missing/extra callback (`recorded_entries.len() != saved.len()`) and on
  a duplicate entity (`HashSet` over recorded entities). It does NOT fail
  on a `TempEntityState` insert recorded after `WorldResumeProgress` was
  removed - that assert was removed mid-round on the owner's correction
  (see "A design turn" below); the count is still logged
  (`post_resume_inserts`), never asserted. Ordering evidence stays in one
  comment: `spawn_resumed` (`transients.rs:707-736`) queues every saved
  transient's spawn then its `resumed_lifetime` insert, in saved-list
  order, on the ONE Commands buffer `state.apply(world)` applies once
  (`transients.rs:753`).
- **PDC-latch fix**, confirmed against `run-r7-2.log` before changing
  anything (per the task's own requirement): the "fire a PDC round" step
  now `.until()`s a live `TurretSectionInput` reading true on any player
  PDC bay (`try_query_filtered`, since the predicate closure only gets
  `&World`) before the next step releases the trigger, closing the race
  between the one-frame input tap and `bevy_enhanced_input`'s PreUpdate
  evaluation (both scheduled `.after(InputSystems)` with no edge between
  them).

### 2. A design turn: the post-resume-insert assert was unsound

Run 2 (first full attempt this round) failed on
`assert_eq!(post_resume_inserts, 0, ...)`: a torpedo scripted to ignite
right in the resume window (`ignite_cold_torpedoes`,
`torpedo_section/projectile.rs:97-124`) triggered `TorpedoIgnited` ->
`on_torpedo_ignition` (`torpedo_section/render.rs:639-648`) -> `LightFlash`
-> `light_the_flash` (`transient_light.rs:153-178`), which spawns a brand
new `TempEntity`-tagged light for ANY flash anywhere in the game.
`TempEntity`'s own `On<Insert, TempEntity>` observer
(`crates/nova_gameplay/src/lifetime.rs:128-148`) universally inserts
`TempEntityState` - the same component this audit watches - on that
unrelated entity, 0.3ms after `end_resume` removed `WorldResumeProgress`.
I asked before changing the approved design (two proposals: hard-fail as
specified, or narrow the count to round/torpedo/piece kinds only). Neither
was taken - the owner pointed out B still breaks on a live hostile round
fired right after resume, and that a genuinely late RESUMED insert is
already caught by the existing `recorded_entries.len() != saved.len()`
check, making the post-removal count redundant as a hard failure either
way. Verdict: log the count, never assert it; one-sentence comment says
why.

Run 3 then hit the `Position`-vs-`Transform` gap described above (also
asked before changing, with the avian source citation). Both turns are
"ask before acting on a design decision", used as the task directs, not
guesses.

### 3. Runs

Four runs total (budget: 4), Xvfb :117 reused throughout (owner's PID
3555128, confirmed alive before every run, stopped and verified gone at
the end). `VK_DRIVER_FILES`/`VK_ICD_FILENAMES` pointed at the host's
`lvp_icd*.json`, `ALSA_CONFIG_PATH` at a genuinely empty file,
`NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 NOVA_AUTOPILOT_DEADLINE=900
RUST_LOG=info,nova_ship=debug`, one `--phase create` invocation per run
(it internally spawns `--phase load` as a child and reports both).

- **Run 1** (pre-dates this report's P6/pose fixes, same session): loadavg
  not sampled under this round's protocol yet. Failed on the live-read P6
  stock mismatch this round's thaw-time fix was built to solve (PdcRound
  5908 live vs 6000 saved) - this is the finding that produced the P6
  design turn above, not a passing run (confirmed explicitly by the owner:
  "Run 1 does not count as a passing run").
- **Run 2**: loadavg 1.93/1.61/2.17 before. Failed on
  `post_resume_inserts == 0` (the unsound assert above) - pairing,
  duplicate and kind checks for this run's 87 transients were never
  reached (the post-resume assert runs first). `run-r8-2-create.log`.
- **Run 3**: loadavg 1.04/1.32/1.99 before. The post-resume count fix
  held (logged "1", not asserted); pairing/duplicate/count/kind for 55
  transients and P6 credits+stock all passed; failed on thaw-time pose
  (`Position` read 0,0,0 vs saved 3200,0,0 - the avian sync gap above).
  `run-r8-3-create.log`.
- **Run 4** (first full PASS): loadavg 2.36/2.71/2.55 before. 47
  transients: round 44, torpedo 2, piece 1, recorded == saved per kind,
  duplicates 0, post-resume inserts 1 (logged, the same live-torpedo-light
  shape as run 2 - confirms this really is ordinary gameplay, not
  resume-specific). Max deltas 0.0000m/0.0000s on every kind. P6: pose
  delta 0.0000m, credits 918273 == 918273, stock matched for all 5 items.
  Pixel-difference figure 10.312 (mean |channel diff|, 0-255 scale,
  informational). Expiry step (`fixture_transients_expired`, deadline
  `TRANSIENT_EXPIRE_DEADLINE_SECS`) completed inside its deadline - the
  walk reached "open the pause menu" next with no panic. Final line:
  `world_resume load: PASS every fixture value matched`. `run-r8-4-create.log`.
- **Run 5** (second full PASS, confirming): loadavg sampled at 4.92 first
  (above the 4.0 threshold - waited, logged, re-sampled 4.53 then 3.25,
  proceeded per the task's wait protocol). 32 transients: round 29,
  torpedo 2, piece 1, recorded == saved per kind, duplicates 0, post-resume
  inserts 1 (same live-torpedo-light shape again). Max deltas
  0.0000m/0.0000s on every kind. P6: pose delta 0.0000m, credits 918273 ==
  918273, stock matched for all 5 items. Pixel-difference figure 10.386.
  Expiry step completed inside its deadline. Final line: `world_resume
  load: PASS every fixture value matched`. `run-r8-5-create.log`.

Two consecutive full passes (runs 4 and 5), as required. Neither run's
sandbox was kept (the harness only retains the sandbox on failure, unlike
runs 2/3's kept sandboxes); a `state.4.ron` from run 3's kept sandbox
(same fixture shape, 47 transients) is 414919 bytes, for scale.

Both PASS runs' before/after screenshots
(`/tmp/nova-pt9/capture-r8-5/world_resume-before-leave.png` and
`-after-load.png`, read via the Read tool) show the same cargoa corvette
hull, the same drifting detached-piece chunk upper-left, and the same
orange PDC-fire markers on both turret mounts, across the leave/load
boundary - the only visible difference is the follow-camera's angle
(closer-in over the deck before leaving, pulled back to frame the whole
hull plus engine glow after loading), which is the known, already-
documented driver of the ~10/255 pixel-difference figure, not a content
regression.

### Handoff

This round's three owner-directed fixes (P6 moved to a thaw-time
`Transform` read, the post-resume-insert count demoted from assert to log,
and the PDC one-frame-press race closed with an input-latch wait) are now
double-confirmed: runs 4 and 5 both reached the final PASS line with zero
measured deltas on every checked kind and on P6, under two different
transient-count shapes (47 and 32) and two different loadavg conditions.
The `post_resume_inserts == 1` seen on both passing runs (always tied to
`ignite_cold_torpedoes` lighting a torpedo right as `WorldResumeProgress`
is removed) is expected, ordinary live-gameplay noise under this audit's
current, corrected design, not a resume defect - flagging it here only so
a future reader does not mistake a nonzero count for a regression.

Xvfb :117 (PID 3555128) stopped and verified gone (`ps -p` exit 1). Nothing
staged, nothing committed (`git status --porcelain` showed only
`examples/systems/system_world_resume.rs` and this report throughout).
