# SW-P6: system_world_resume, API-injected fixture state

## Scope

Files touched, final state:
- `examples/systems/system_world_resume.rs` (new file, 885 lines).
- `Cargo.toml`: `[[example]]` block for `system_world_resume`, `required-features
  = ["debug"]`, beside `system_open_world`'s block (lines 572-576). Present
  from earlier in this task; unchanged this round.
- `crates/nova_menu/src/load_screen.rs`: touched only for the mutation test
  (see below), then reverted. `git status` shows it `??` (untracked, another
  worker's new file, not yet committed by anyone) — `git diff` is moot for an
  untracked file, so the revert was confirmed by direct `Read`, not `git
  diff`.
- `crates/nova_probe_cli/tests/catalog_drift.rs`: confirmed unmodified and
  needing no edit — it is a generic disk/manifest-agreement test with no
  example names hardcoded.

Not touched: `crates/nova_ship/src/camera/*`, `crates/nova_world_base/src/save/session.rs`
(sw-camproof's temporary mutation range), the 12 examples and
`crates/nova_bench/src/manual.*` (sw-examples' range), or any other file
outside the above.

## The redirected design: API-injected fixture state

Per the owner's instruction, the earlier "hold the real mining key" approach
is gone. `inject_fixture` now sets all three pieces of fixture state in one
`on_enter` call, directly through production constructors, never through a
gesture:

- **Carve**: `world.entity_mut(rock).insert(BodyRadius(body_radius_before *
  0.9))`. This does not rebuild a collider or mesh — it stands in for a
  carve's result, because only the saved-vs-resumed `BodyRadius` equality is
  under test.
- **Canister**: `world.spawn((cargo_canister(CargoCanister::new(ItemType::IronOre,
  3), transform, Vec3::ZERO, AssetRef::from(MINED_CANISTER_MESH)),
  canister_ids.mint(), ScenarioScopedMarker))` — the exact constructor and
  bundle `eject_mined_canisters` uses, called directly instead of through the
  beam-pulse-to-ejector pipeline.
- **Credits/stock**: unchanged from the earlier approved design —
  `ShipCredits` field set, `ShipInventory::add` called.

**THIS IS DISCLOSED FIXTURE STATE, NOT MINING OR LOOT GAMEPLAY.** No key is
held, no beam fires, no remesh runs, no `TravelLock` is set. The module doc
comment and `inject_fixture`'s own doc comment both say this explicitly. The
round trip under test — whether a save written by one process opens
correctly in another — only needs the state in place before the save lands,
not the gesture that would normally produce it.

Per instruction: `Health` is never touched (no top-up), and `run_create`
never retries from `main()` on failure — a failed phase panics with the
sandbox path kept on disk, same as before.

Removed as dead code: `hold_mine_key`, `set_mining`, `canister_seen`,
`observe_mined_canisters`, `rock_node_aabb`, `mining_reach_engine`,
`Fixture.mining`, `Fixture.pin`, the `TravelLock`/`ColliderAabb`/`Rotation`
imports, `CARVE_DEADLINE_SECS`, and the "hold the mine key" / "release the
mine key" script steps.

## Camera: deferred, not implemented

`SavedPlayer.view` / `CameraView` assertions are **not implemented**. I read
`tasks/20261007-090756/CAMERA-GATE.md` proof 5: it describes a P6 probe of
its own — phase 1 turning the Normal rig, zooming, holding FreeLook through
the crossing save and recording the camera `GlobalTransform` relative to the
ship at the leave save; phase 2 asserting the first rendered frame's camera
pose against that record, then the rig and zoom. That is a materially
different, separately-scoped probe (new rig-control steps, a new
first-rendered-frame assertion point, new interface decisions about what
"first frame" means for this harness) — out of this task's narrow scope, and
it touches `crates/nova_ship/src/camera`, which sw-camproof currently owns.
Left for later, as instructed.

## A bug found and fixed within my own file: the fixture rock's sector

**Symptom** (run 1 of 7): phase 2 panicked —
`world_resume load: no resumed asteroid named
'sector_0_n1_0_cluster_0_n1_0_rock_9'`. The rock chosen by `nearest_asteroid`
right after the world arms (sector `(0, -1, 0)` relative to spawn) is one
sector off the player's *destination* sector `(1, 0, 0)` after the crossing
teleport. `armed_around_a_player` only checks that `WorldConfig` exists and a
player stands — it does not wait for the full streaming window (125 cells,
`active_radius` 2) to finish loading around the resumed position, so the
assertion step could run before that rock's sector had thawed back in.

**Fix**, inside `examples/systems/system_world_resume.rs` only: added
`fixture_rock_streamed_in(rock_id)`, a predicate that polls for an
`AsteroidMarker` entity carrying the saved `EntityId`, and a new load-phase
step — "wait for the fixture rock to stream back in" — between "the world
arms with the thawed player" and "check the resumed state against the save",
deadline `BEAT_DEADLINE_SECS` (30s). No production code changed; this is the
same wait-on-a-predicate idiom every other step in the file already uses.

## A second bug found and fixed: the crossing-save deadline

**Symptom** (run 2 of 7): `` step `world_resume: cross a sector boundary`
stalled after 60.1s ``. The log shows the full 125-cell window took ~25s to
stream on lavapipe with the Open World scenario's combat running
concurrently (`nova_world: all 125 desired sector(s) around (1, 0, 0) are
live` at +25s), and `saved_past_first_write()` never reached generation 2
within the original 60s `CROSSING_DEADLINE_SECS`.

**Fix**: raised `CROSSING_DEADLINE_SECS` from 60.0 to 180.0, with a comment
recording the measured 25s streaming cost. A timing *budget*, not a timing
*assertion* — consistent with `AGENTS.md`'s "never assert timing."

## Verification

`cargo check --features debug --example system_world_resume -j 4`: clean,
no warnings in this file. (One pre-existing unused-import warning in
`crates/nova_ship/src/camera/resume.rs`, sw-camproof's file, not mine.)

Ran under Xvfb (`:160`, started and stopped by me this session) + lavapipe
(`VK_DRIVER_FILES` set to the `lvp_icd.x86_64.json` in this nix closure),
`ALSA_CONFIG_PATH=` empty, `NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=600
NOVA_CAPTURE=1`, via `cargo run --features debug --example system_world_resume
-j 4`:

- Run 1: failed on the sector bug above (fixed).
- Run 2: failed on the crossing-deadline bug above (fixed).
- Runs 3-7 (5 consecutive runs after both fixes): **all 5 passed**, exit 0,
  `world_resume load: PASS every fixture value matched` logged each time. No
  `NameTaken`, no stall, no panic, no combat death.

**Known exposure, measured**: 0 deaths out of 5 post-fix runs. The direct
API-injection approach removed the multi-second-to-minutes window the old
"hold the mining key" design spent stationary inside the Open World
scenario's live combat; fixture injection now lands in a single frame, so
the ship is only exposed to that combat for the few seconds between New Game
arming and the sector crossing. I did not see a death in 5 runs, but 5 runs
is not a guarantee — the Open World scenario still puts the player ship into
an active engagement immediately at spawn by design (confirmed in
`system_open_world.rs`'s own script), so a slow or unlucky run could still
take the ship below a threshold `detect_neutralized` reads before the
crossing save. Not fixed further per instruction: combat damage before the
save is accepted as part of the saved state, not something to prevent.

## Frames inspected

- `world_resume-load-screen.png`: **correct**. Shows the Load screen with
  `broken-world` selected and refused (`unreadable: world.ron: No such file
  or directory (os error 2)`, red text, Load button greyed) and `Probe
  World` listed below it, unselected. Matches
  `assert_broken_world_disabled`.
- `world_resume-status-line.png`: **wrong frame, disclosed as a gap, not
  fixed**. The captured frame (run 7) shows a `LOADING SCENARIO` /
  `LOADING` progress-bar overlay, not the save status line. Root cause: the
  step's gate, `saved_past_first_write()`, is satisfied as soon as the
  crossing's generation-2 write lands, which happens well before the full
  125-cell streaming window (and whatever UI it drives) settles — this step
  and its gate predate my redirected work and I did not change them, since
  doing so is a step-ordering/predicate design decision, not a mechanical
  fix. Flagging rather than silently calling it a pass.
- `world_resume-leave-overlay.png`: **never written, disclosed as a gap, not
  fixed**. The request logs (`nova capture: world_resume-leave-overlay.png`)
  but is immediately followed by `WARN bevy_render::view::window::screenshot:
  Unknown window for screenshot, skipping: 66v0`, then the create process
  exits clean within ~300ms. Root cause, read from
  `crates/nova_menu/src/leave.rs` (not mine, not edited): Exit's leave save
  calls `session.request_leave()`, which only becomes idle+saved once a
  *fresh* write completes; `drive_pending_leave` then fires `AppExit`
  directly, with no frame budget reserved for anything else in flight. For
  this sandboxed, already-generation-2 world the fresh write is fast enough
  that `AppExit` can land before the async screenshot's 2-3 frame
  request-to-disk round trip finishes. Because `AppExit` stops the Bevy
  schedule outright, the step's own `until(shot_written(..))` wait and its
  deadline-stall check never get another chance to run, so the step exits
  silently instead of failing loudly. This is a property of the leave-save
  timing in `nova_menu` interacting with the async screenshot pipeline, not
  something introduced by this round's rewrite or fixable from the example
  alone without changing that production timing — out of scope here.

## Mutation test

Target: `crates/nova_menu/src/load_screen.rs`, `on_load_world`'s `Ok(..)`
arm. Replaced the `commands.queue(move |world| resume_world(..))` call with
a no-op (`let _ = (folder, lock, header, world_state);`), leaving
`pick.0 = None`, `*mode = GameMode::NewGame`, and the `Playing` transition
untouched, so the world arms from a bare New Game path instead of the save.

Result: the load phase panicked immediately in `sync_open_world`
(`crates/nova_world_base/src/lib.rs:234`):
```
nova_world_base: an OpenWorld scenario is live with no OpenWorldSession; the
world has no seed. Start it through New Game, which inserts the session.
```
A loud, specific, meaningful failure — production code itself refused to
proceed with a broken resume, before my own `assert_resumed_state` ever got
a chance to run. Reverted the mutation with `Edit`; `cargo check --features
debug --example system_world_resume` is clean again; `git diff
crates/nova_menu/src/load_screen.rs` is empty (the file is untracked by git
in this worktree, so I additionally confirmed the revert by reading the file
back).

## Cleanup

Removed the 4 leftover `target/example-profiles/system_world_resume-*`
sandboxes from the two failed diagnostic runs and the mutation-test run (the
5 passing runs self-cleaned). Removed the two screenshot PNGs left at the
repo root by the last run. Started and stopped my own `Xvfb :160` (did not
touch `:97` or `:150`, which belong to other workers/sessions).
