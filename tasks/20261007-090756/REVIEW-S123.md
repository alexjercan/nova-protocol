# Independent review: resumable worlds, slices 1-3 (format, disk, session)

Scope: `crates/nova_world/`, `crates/nova_world_base/` (including the new
`save/` module), `crates/nova_scenario/`, `crates/nova_ship/`,
`crates/nova_gameplay/`, `crates/nova_assets/` diffs in the `resumable-worlds`
worktree, against `tasks/20261007-090756/GATE.md` and the owner decisions in
the review request. Slice 4 (menu UI) is not written; findings below note
what it must honor, not defects in it.

Tests run (focused, `-j 4`, no workspace-wide run, no Clippy):
- `cargo test -p nova_world --lib frozen::` - 11 passed.
- `cargo test -p nova_world_base --lib save::` - 11 passed.
- `cargo test -p nova_scenario --features serde --lib spawn::tests::a_resumed_player_spawn_thaws_the_saved_ship` - passed.
- `cargo test -p nova_scenario --features serde --lib mining::` - 7 passed.
- `cargo test -p nova_ship --features serde --lib sections::frozen::tests` - passed.
- `cargo test -p nova_gameplay --features serde --lib inventory::tests::cargo_canister_id_allocator_resumes_past_a_saved_world` - passed.

All named proofs I could run pass and are not vacuous: each asserts a
concrete value (RON text equality, AABB equality, panic message, generation
number, file list) that a wrong implementation would change, and I confirmed
`write_world`'s and `freeze_fixture`'s own worker already demonstrated the
deliberate-break-then-restore check for the fixture collider test.

## Findings

### 1. MAJOR, CONFIRMED - serde is mandatory, not feature-gated, in `nova_world`

Path: `crates/nova_world/Cargo.toml` (`serde = { version = "1", features =
["derive", "rc"] }` as a plain dependency; `nova_gameplay`, `nova_scenario`,
`nova_ship` pinned with `features = ["serde"]` unconditionally) and
`crates/nova_world/src/frozen.rs:60,264,306,372` (`FrozenSectors`,
`FrozenSector`, `FrozenBody`, `FrozenBodyType` derive `Serialize,
Deserialize` directly, with no `cfg_attr(feature = "serde", ...)`).

GATE.md section 4 states: "`serde` derives are mechanical and gated the same
way as the existing ones (`cfg_attr(feature = "serde", ...)`)." `nova_world`
has no `serde` feature of its own (`grep` of its `[features]` table shows
only `debug`), so this instruction was not followed for the crate that owns
the format. The consequence is concrete: every build of `nova_world` (and
transitively `nova_world_base`, since its own `Cargo.toml` now also pins
`nova_scenario = { features = ["serde"] }` unconditionally, new in this diff)
always compiles the full save-serialization surface of `nova_gameplay`,
`nova_scenario` and `nova_ship`, and always turns on `bevy/serialize` -
including for the `wasm32` web target, which the same diff's own doc change
says has "no saved worlds" (`nova_world_base/src/lib.rs:32`, `save/mod.rs:18`).
Nothing is functionally broken on web (the `save` module itself is `#[cfg(not(
target_arch = "wasm32"))]`), but there is no way to build `nova_world` without
this surface, which is a real deviation from the approved design text and
from the opt-in pattern every other crate in this diff uses correctly
(`nova_ship`, `nova_gameplay`, `nova_scenario` all gate their own derives
behind `cfg_attr(feature = "serde", ...)` with `dep:serde` optional).

Pre-existing exception, for context: `nova_world_base` already forced
`nova_ship/serde` unconditionally before this diff, for an unrelated reason
(the ship-part snapshot hash). That precedent does not extend to `nova_world`
itself, which is new in this diff and has no such pre-existing excuse.

### 2. MAJOR, CONFIRMED - `restore_resumed_world` has no test

Path: `crates/nova_world_base/src/save/session.rs:207-220`.

This is the exact seam the review brief called out by name: "its `is_added`
on an exclusive system's first run." I traced the logic by hand: the system
is gated `run_if(resource_exists::<ResumedWorld>)`
(`crates/nova_world_base/src/lib.rs:186`), so as long as nothing else removes
`ResumedWorld`, the system runs on every frame from insertion through the
arming frame, which keeps its own `last_run` tick current and makes
`config.is_added()` correctly true on exactly the frame `WorldConfig` is
inserted. This reasoning holds, but it is unverified: there is no test in
`crates/nova_world_base/src/tests.rs` or elsewhere that drives the real
`NovaWorldBasePlugin` schedule through an arm with a `ResumedWorld` present,
so neither the happy path (the ledger is actually seeded from the save) nor
the panic path (a `ResumedWorld` still present after the world already armed)
is exercised by anything in this diff. `grep` for `restore_resumed_world` and
`ResumedWorld` confirms no test file references either symbol.

### 3. MAJOR, CONFIRMED - docked-pair-at-save-time (D1b) has no test

Path: `crates/nova_world/src/frozen.rs:738-808` (`snapshot_sectors`),
`crates/nova_world/src/streaming.rs:1007-1040` (`freeze_sector_bodies`,
now parameterized by `DockPolicy`).

`DockPolicy::Undock` is new in this diff and is exactly the mechanism D1b
depends on: it is what lets `snapshot_sectors` freeze a docked ship (the
existing retirement path keeps `DockPolicy::Refuse`, which panics on a docked
ship rather than split the pair across the window). The two existing docking
tests (`a_docked_partner_moves_to_the_cell_it_stands_in_when_its_home_cell_retires`,
`a_docked_partner_waits_out_a_retiring_cell_it_stands_in`) only exercise
`DockPolicy::Refuse` through `retire_sectors`, and by design they never
actually reach a docked entity in `freeze_body` (the partner is kept live and
reparented instead). I read the code path for `DockPolicy::Undock` and it
looks correct (a docked ship freezes via `freeze_ship`, which does not touch
`DockedShip` and so naturally omits it from the record; the live world is
untouched since `snapshot_sectors` despawns nothing), but no test in this
diff calls `snapshot_sectors` on a world holding a docked pair, so the
specific, owner-approved behavior that the save keeps both ships "at their
poses with no dock" is unproven, not just undocumented.

### 4. MINOR, PLAUSIBLE - fail-then-retry-then-succeed generation reuse is not tested as one sequence

Path: `crates/nova_world_base/src/save/mod.rs:305-333` (`write_world`),
`crates/nova_world_base/src/save/session.rs:336-369` (generation computed as
`session.generation + 1`, and `session.generation` is bumped only on `Ok` in
`poll_world_writer`).

Two tests separately cover the two halves (`a_failed_write_leaves_the_last_good_save`
proves a failure leaves the prior generation readable;
`a_written_world_opens_as_it_was_saved` proves two successive successful
writes each replace the previous state file). I traced the connecting case by
hand: because `session.generation` does not advance on a failed write, a
retry recomputes the *same* target generation number, and `write_atomic`'s
rename-over-whatever-is-there semantics mean a stray file left by the earlier
failed attempt is silently and correctly overwritten by the successful retry.
This is not exercised end-to-end by any single test, so it is a reasoned
conclusion, not a run proof.

## Checked and found correct (no finding)

- Write order in `write_world` (state, then header, then remove previous) and
  the crash-between-steps tolerance, including the first-write case
  (`generation 0 -> 1`, where removing "generation 0" is a silent no-op via
  `ErrorKind::NotFound`): confirmed by reading and by
  `a_failed_write_leaves_the_last_good_save` /
  `opening_a_world_removes_the_files_its_header_does_not_name`.
- `FrozenSectors` Arc copy-on-write: `snapshot_sectors` clones the ledger
  (incrementing refcounts, not deep-copying), and `visit`/`arrive`'s
  `Arc::make_mut` therefore always clones privately for the snapshot rather
  than mutating the live resource's shared record, because the live world's
  own reference keeps the strong count above 1 for the whole snapshot.
  Confirmed by reading and by
  `a_snapshot_of_the_live_world_reads_back_as_the_same_world`, which also
  proves the live rock/ship entities are not despawned by a snapshot.
- Serde strictness: every new/changed frozen record and the new save types
  carry `deny_unknown_fields` (checked file-by-file across
  `nova_world`, `nova_scenario`, `nova_ship`, `nova_gameplay`); the one
  non-`deny_unknown_fields` cases are closed enums, consistent with the
  pre-existing convention. `AIThreat.attacker: Option<Entity>` is `serde(skip)`
  with a comment naming why. The `visibility_serde` remote derive round-trips
  through the existing test. No `Handle` reaches a frozen type; the asteroid
  texture was converted from a `Handle<Image>` to a path constant
  (`ASTEROID_TEXTURE_PATH`, `nova_assets/src/collections.rs`) specifically to
  remove the one `Handle` that used to ride along.
- `ResumedSpaceship` consumption in `nova_scenario/src/actions/spawn.rs`: a
  record for a non-matching id is correctly put back, proven by
  `a_resumed_player_spawn_thaws_the_saved_ship` (run, passed) and by the
  `sync_open_world` panic in `nova_world_base/src/lib.rs:264-271` if a record
  is still unconsumed when the world arms.
- Session state machine: coalescing (one `wanted` slot, newest reason wins),
  a dropped Leave becoming `Failed` while a dropped Crossing is silent, the
  settling bound counting only advancing frames (proven by
  `a_rock_that_never_settles_fails_at_the_bound` in `nova_world` and
  `a_leave_save_that_never_settles_fails_at_the_bound` in `nova_world_base`,
  both run and passed), `poll_world_writer` before `snapshot_world` in the
  chain, and `is_idle()` never alone implying a saved leave (the
  never-settles test ends `Failed` while `is_idle()` is true, matching the
  owner decision).
- Carved asteroid rebuild: `prepare_frozen_asteroid` /
  `AsteroidFieldSnapshot::geometry` are pure and worker-safe, and
  `thaw_asteroid`'s new consistency assert (geometry's carved-ness must match
  the record's) and its panic on a carved field that meshes to no trimesh are
  both reachable guards, not dead code. Proven indirectly by
  `a_mined_fought_and_looted_cell_returns_as_it_was_left` and the snapshot
  round-trip test (both assert `ColliderAabb` equality after thaw).
- Fixture collider rebuild: `FrozenFixture.collider` is gone; a plate rebuilds
  from `ShellShape`, decor rebuilds from the new `DecorColliderSize`
  component. Proven by the new, run-and-passing
  `a_frozen_plate_and_decor_thaw_with_the_colliders_they_froze_with`, including
  a deliberate-break check recorded in the sub-worker's own report.
  `CargoCanisterIdAllocator::resume_after` is a correct max-of-two-counters
  design, proven by `cargo_canister_id_allocator_resumes_past_a_saved_world`
  (run, passed).
- Lock lifetime: `WorldLock` holds the `File` only to keep the OS lock alive
  (`#[expect(dead_code, ...)]` is accurate, not a mask for a real bug);
  `create_world` only attempts the lock after `create_dir` has already
  excluded a same-slug race, and `open_world` on a world already locked
  (including by the same process, since a lock on a distinct `File`/fd
  conflicts with an existing exclusive lock regardless of process identity)
  correctly refuses with `WorldRefusal::Locked`, proven by
  `a_world_another_game_holds_open_is_refused`.
- Stale leftovers: no adapter, alias or dual path found for the removed
  `FrozenAsteroid.collider`/`drawn_mesh`, `FrozenDrawnMesh`, or
  `FrozenFixture.collider` - each caller was updated, not wrapped. No test
  for the old Handle-carrying mesh/collider fields was left behind; the old
  `FrozenDrawnMesh`-specific comments were rewritten rather than left stale.

## What slice 4 (menu) must honor, not a defect here

- **Retry under D3a must drop the current `WorldSaveSession` (releasing its
  `WorldLock`) before calling `open_world` on the same slug.** `open_world`
  takes its own `File::try_lock` (`save/mod.rs:368-384`); if the session's
  existing lock is still held when Retry calls `open_world` again, it will
  refuse itself with `WorldRefusal::Locked`. Nothing in slices 1-3 handles
  this ordering, because it is a menu-level call sequence, not a `save/`
  responsibility.
- **D2's "release virtual time until the snapshot succeeds" is not
  implemented anywhere in slices 1-3.** `snapshot_world`'s settling counter
  (`SAVE_SETTLING_FRAMES_MAX`, `session.rs:35`) only counts advancing frames;
  it does not itself unpause `Time<Virtual>`. The pause menu/leave overlay
  (slice 4) is the one place that can hold input and release virtual time
  behind the overlay per the gate's D2a recommendation - without that, a
  leave save taken while the game is still paused (as today) will sit
  `Waiting` and eventually fail at the 600-frame bound, never settling,
  because nothing is advancing virtual time for it to count.
- `nova_world_base::save::worlds_root()` (`nova_assets::storage::worlds_root`)
  exists and is correct per D5, but is unused by anything in slices 1-3; the
  menu is the first caller.

## Skipped checks and uncertainties

- No workspace-wide test or Clippy run, per instruction; only the targeted
  suites listed above.
- No GPU/game run, and no measurement (P7 is explicitly deferred/unmeasured
  in the gate itself).
- I did not re-derive or stress-test Bevy's own change-detection tick
  semantics for `is_added()` beyond reading the scheduler's documented
  behavior; finding 2 reports this as an unproven seam rather than a
  confirmed defect because I found no counter-example, only an absence of
  proof.
- Slice 4 (menu UI) is not written; I did not review it and the two notes
  above are forward-looking, not findings against existing code.
