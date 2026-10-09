# Review: T2/T3 transient save/restore integration

Scope: `nova_world_base/src/save/{transients,mod,session}.rs`,
`nova_world/src/frozen.rs`, `nova_ship/src/sections/{torpedo_section/frozen,frozen,fixture,frozen_piece}.rs`
(production code only in `frozen_piece.rs`), `nova_scenario/src/objects/asteroid_carve.rs`,
`nova_world_base/src/save/tests.rs`, `nova_menu/src/tests/leave.rs`.
Reviewed against `tasks/20261007-090756/TRANSIENT-GATE.md` rev 2 + section 12 (section 12 entries
taken as authoritative over earlier text, per instructions).

## Summary

The implementation matches the gate closely. The two-phase restore, the `resolve_all`
wait/refuse/ambiguity policy, `check_saved_ids` at both snapshot and open, the dead-target-link
rule, and the torpedo-part-at-zero rule are all implemented as specified and are backed by real
behavioral tests (not exit-code-only checks). I found no BLOCKER. Two gaps and one stale-doc nit
are worth the owner's attention.

## Findings

### 1. [RISK] `resolve_all`'s "ambiguity refuses at once" path has no test coverage

- **Evidence**: `crates/nova_world_base/src/save/transients.rs:561-566,607-612,625-630` (the three
  `Err(TransientRefFault::Ambiguous(count)) => return Err(Resolve::Refuse(...))` branches for
  owner, section-ship, and body target). `grep -n "Ambiguous\|live ships have\|live bodies have\|live
  sections of" crates/nova_world_base/src/save/tests.rs` returns nothing.
- **Consequence**: the gate's "Transient integration" deviation (section 12) explicitly
  distinguishes "ambiguity refuses at once" from "missing waits, then refuses at the 240 s bound" -
  two different, easy-to-conflate code paths (`Resolve::Refuse` vs `Resolve::Wait`). Nothing in the
  reviewed test files exercises the ambiguous-id branch, so a regression that silently turned
  "refuse at once" into "wait, then time out at 240 s" (or vice versa) would not fail any test.
- **Suggested fix**: add a focused test (two live ships sharing a saved id resolving a torpedo
  owner, or two live sections of one ship sharing the saved section id) asserting an immediate
  `WorldResumeRefused`, not a multi-frame wait.

### 2. [NIT] Stale "no caller re-inserts `TempEntity`" claim in the design doc, not in code

- **Evidence**: `tasks/20261007-090756/TRANSIENT-GATE.md:186-189` ("Verified (SCOUT-TRIGGERS.md 4):
  no caller re-inserts `TempEntity` on an entity that has one.") is contradicted by
  `crates/nova_scenario/src/objects/asteroid_carve.rs:1300-1314` (`thaw_rock_chunk` calls
  `spawn_carved_chunk`, whose bundle at `crates/nova_gameplay/src/integrity/chunk.rs:259-260`
  already inserts `TempEntity(CHUNK_LIFETIME_SECS)`), followed by
  `crates/nova_world_base/src/save/transients.rs:732-734` (`spawn_resumed` unconditionally inserts
  `resumed_lifetime(transient.lifetime)` on the same entity afterward) - a genuine second insert of
  `TempEntity` on an entity that already has one.
- **Is it a bug?** No. Traced through `bevy_ecs::world::command_queue::RawCommandQueue::apply_or_drop_queued`
  (`command.apply(world); world.flush();` per queued command): the spawn's `On<Insert, TempEntity>`
  observer-queued "give it a fresh `TempEntityState`" command is flushed *before* the next queued
  command runs, and `spawn_resumed`'s later `resumed_lifetime` insert is one atomic bundle that
  writes both `TempEntity` and `TempEntityState` directly, so it always wins regardless of
  ordering. The task's own `tasks/20261007-090756/SW-CHUNK.md:286-295` independently re-derives and
  confirms this exact override behavior for chunks. So the code is correct; only the quoted design-
  doc sentence (written for T1/rounds, before T3 added the chunk reuse of a live-spawn builder) is
  now inaccurate as a universal claim.
- **Suggested fix**: none required for code. If the gate doc is revised again, narrow or drop the
  "no caller re-inserts" sentence, or scope it explicitly to rounds/torpedoes.

### 3. [NIT] CHANGELOG `[Unreleased]` entry undersells what T3 already does

- **Evidence**: `CHANGELOG.md:17-21` only describes frozen *sectors* (mined rocks, ships, canisters,
  wrecks). Nothing in `[Unreleased]` mentions that an in-flight round, torpedo, shed fixture,
  rock chunk or detached piece now survives a save/Load - a player-visible behavior change
  this review confirms is implemented and tested (`crates/nova_world_base/src/save/tests.rs:718-1225`,
  `crates/nova_ship/src/sections/frozen.rs:621-866`).
- **Consequence**: low - the feature is explicitly phased (T4/P-T9/P-T11 remain), so deferring the
  changelog line until the slice is complete may be intentional. Flagging per AGENTS.md's "reread
  all `[Unreleased]` entries" rule so it isn't forgotten when T4 lands.
- **Suggested fix**: add the transient-resume line when the full T1-T4 slice ships, not necessarily
  now.

## Correctness checks that passed

- **Two-phase restore** (`transients.rs:688-757`, `spawn_resumed`): one `SystemState<ThawResources>`
  borrowed once, every thaw queued on the same `Commands`, the target-link pass queued after, one
  `state.apply(world)` - no tick between spawn and `TorpedoTargetEntity` link. Matches gate
  "Transient integration" bullet.
- **`resolve_all` policy** (`transients.rs:473-668`): style prevalidation and render-resource-by-
  name checks run before any ref resolution; missing owner/section/body/canister refs push into
  `waiting` and return `Resolve::Wait`; ambiguity returns `Resolve::Refuse` immediately; nothing is
  pushed to `spawn_resumed` unless `waiting` is empty. Matches gate.
- **`check_saved_ids`** (`save/mod.rs:356-455`): shared by `open_world` (`mod.rs:308-309`,
  `Unreadable`) and `snapshot_world` (`session.rs:467-478`, `Failed`, nothing written). Covered by
  `tests.rs:1345-1600` (duplicates across cells/player, malformed vs. legitimately-nested wreck ids,
  dangling owner/body/section/transient refs, self and out-of-range `Transient(i)`, no write on
  refusal).
- **Dead target link** (`torpedo_section/frozen.rs:238-256` in `transients.rs`'s `target` closure,
  consumed at `torpedo_section/frozen.rs:267-281`): a despawned target gives `Ok(None)`, freezing
  to `Frozen(last)` or `DumbFire` per the "Dead target link" owner decision. Proven live, under the
  real pause gate, by `nova_menu/src/tests/leave.rs:130-195`
  (`leaving_a_saved_world_waits_for_the_leave_save_with_input_still_paused`) and
  `tests.rs:1106-1224` (`a_torpedo_tracking_a_torpedo_resumes_on_it`).
- **Torpedo part at zero** (`torpedo_section/frozen.rs:144-166`, `part_health`): `Unsettled` while
  `Health.current <= 0.0` or `HealthZeroMarker`; on-disk validation enforces `(0, max]` in
  `save/mod.rs:116-126` (`FrozenTransient::validate`). Covered live by `tests.rs:863-887`.
- **Index consistency** (`transients.rs:223-297`): `indices` is computed once over `bodies` before
  the freeze loop, and the loop pushes into `frozen` in the same order, skipping only `Visual`
  kinds (which are also excluded from `indices`); every freeze failure (`?` or an explicit early
  `return Err` for `Blast`) aborts the whole function before anything partial is returned, so no
  inconsistent partial list can ever be written.
- **Shed fixture / detached piece style gate**: `resolve_all` prevalidates
  `FrozenShedFixture.style`/`FrozenDetachedPiece.style()` against `GameStyles::get_style`
  (`skin_style.rs:563-565`), the exact function `ShipStyle::resolve` (used by the thaw-time panic
  guard) calls, so the precheck and the thaw-time programming-error panic can never disagree.
  `FrozenShedFixture.grace` is confirmed removed (per the T2/T3 deviation); the shed fixture's
  remaining time now lives only on `FrozenTransient.lifetime`.
- **Rock chunk thaw** (`asteroid_carve.rs:1278-1338`): takes `&mut Assets<Mesh>`,
  `&mut Assets<AsteroidSurfaceMaterial>`, `&AssetServer` as explicit arguments, borrowed once via
  the shared `SystemState` in `spawn_resumed`, matching the owner-approved option A signature.
- **`FrozenShip::has_section`/`FrozenCanister::id`**: both exist exactly where the gate names them
  (`nova_scenario/src/objects/spaceship.rs:600`, `nova_ship/src/sections/cargo_intake_section.rs:289`).
- **No stale task citations, TODO/FIXME/XXX, or bare `#[allow]`** found in any of the reviewed
  production files.

## Skipped checks / uncertainties

- Did not run `cargo test`/Clippy, per instructions. All test-strength judgments above are from
  reading assertions, not from executing the suite.
- Did not review `crates/nova_ship/src/sections/frozen_piece.rs`'s `#[cfg(test)]` module (another
  worker is editing it, per instructions); the production code in that file (through line ~805) was
  reviewed in full.
- P-T7, P-T10, P-T11 (rock chunk / detached piece round-trip, and the bounded-refusal integration
  test) live in `nova_scenario`'s and `nova_ship`'s own test modules and in `nova_core/tests/`
  respectively, per the gate's own file placement - outside this review's listed scope, so not
  independently verified here beyond the production code they exercise.
- Did not verify the Bevy 0.19 `Commands`/Observer flush ordering claim (finding #2) against a live
  run; the analysis is from reading `bevy_ecs-0.19.1`'s `command_queue.rs` source plus the task's
  own `SW-CHUNK.md` scout note, not from instrumenting a real app.
- Did not check whether a live-only "ambiguous owner id" scenario (two spawned ships sharing a
  saved id) can actually arise given the rest of the id-minting rules (D-T7/T-IDRULE) - finding #1
  is about missing *test* coverage for an existing code path, not a claim that the path is
  reachable in practice.
