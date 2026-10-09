# SW-PT11: P-T11 proof - DONE

## File ownership (kept)

Only these were touched:
- `crates/nova_core/tests/world_resume_refusal.rs` (new, 347 lines)
- `crates/nova_core/Cargo.toml` `[dev-dependencies]` section (added)

`crates/nova_core/src/loading_screen.rs` was fixed by its owner mid-session
(a real bug my rig hit - see "Defect the test caught" below); I did not
touch it. `crates/nova_world_base/src/save/transients.rs` was mutated and
restored byte-identical via `/tmp` copy + `cmp`, never `git checkout`. No
files were staged, committed, or pushed.

## Claim

`crates/nova_core/tests/world_resume_refusal.rs::a_load_that_cannot_fill_its_window_is_refused_and_releases_the_clocks`
proves P-T11 end to end on the production path: `NovaMenuPlugin` (Retry,
leave/retry pipeline, `refuse_resumed_world`) + `nova_core::loading_screen::LoadingScreenPlugin`
(the "RESTORING SECTORS" line, the "stays up" gate) +
`nova_world_base`'s real `restore_resumed_transients`, exercised through
`nova_world_base::test_support::{WorldSaveTestPlugin, arm_save_fixture}` (a
dev-only fixture, no production type/function added).

## Evidence

- `nix develop --command cargo check -p nova_core --tests -j 8`: clean, no
  errors (only the pre-existing `proc-macro-error2` future-incompat notice).
- `nix develop --command cargo test -p nova_core --test world_resume_refusal -j 8`:
  ```
  running 1 test
  test a_load_that_cannot_fill_its_window_is_refused_and_releases_the_clocks ... ok

  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
  ```
  Reran 3x clean in addition to the final run above; deterministic.

## What the test drives (production rig, public surface only)

`crates/nova_core/tests/world_resume_refusal.rs:121-342`. Visibility
barriers and how each was crossed, all with the real logic still running,
no stand-ins:
- `WorldsRoot` is `pub(crate)` to `nova_menu` (`crates/nova_menu/src/world_setup.rs`) -
  `NovaMenuPlugin::build` falls back to `nova_assets::storage::worlds_root()`
  when nothing set it first, which reads `NOVA_CONFIG_ROOT`
  (`crates/nova_core/tests/world_resume_refusal.rs:130`). The test binary
  runs exactly one `#[test]`, so mutating that env var is not a race.
- `PendingLeave` is `pub(crate)` to `nova_menu` - the test instead polls the
  public `ResumedWorld` resource, which `resume_world` inserts synchronously
  as the first thing after clearing `WorldConfig`
  (`crates/nova_core/tests/world_resume_refusal.rs:214-216`).
- `LoadingResumeLineMarker`/`WorldListings`/`LoadWorldRow` are all private to
  their crates - the test scans every `&Text` component instead
  (`all_texts`/`texts_now`, `world_resume_refusal.rs:64-77`).

## Proof points checked (all from P-T11/D-T8)

1. New Game -> Retry ("Load last save") with an initially empty
   `WorldListings` (the world was created via `create_world`, never listed)
   - lines 171-216.
2. Re-armed at `active_radius: 0` after the resume, standing in for the
   scenario arm this rig has no loader for (documented at lines 218-223) -
   no `SectorRoot` ever spawns, so the one desired sector never comes.
3. While held: `ClockFreeze::is_held_by(FreezeOwner::WorldResume)`,
   `Time<Virtual>::is_paused()`, `WorldResumeProgress { live: 0, desired: 1 }`,
   and the loading screen shows `"RESTORING SECTORS 0 / 1"` - lines 234-249.
4. Comfortably under `WORLD_RESUME_SECONDS_MAX` (239 s in): still held, no
   `WorldResumeRefused` yet - lines 251-264.
5. Just past the bound: on the *exact frame* `refuse` runs (still
   `GameStates::Playing`, before `StateTransition` applies `MainMenu` and
   before `nova_menu::pause::force_unpause`'s `OnExit(Playing)` safety net
   (`clocks.release_all()`) could run), the clocks are already released -
   lines 266-286. This isolates `end_resume`'s own release from that later,
   unconditional safety net (see "Mutation test" below for why this
   distinction mattered).
6. Settled: `GameStates::MainMenu`, no `WorldSaveSession`, `world.lock`
   free (`open_world` succeeds), the progress line is gone (empty), the Load
   row names the refusal (`"...did not come back within 240 s..."`), and
   the save files are byte-identical to before Retry - lines 288-341.

## Gate's second case (saved owner id with no live ship)

Not covered. Reported as not covered per the task's own allowance; the
first case already exercises the full width of this rig (menu + loading
screen + save/resume + refusal teardown), and a second phase would need its
own saved-transient fixture with a dangling `SavedOwner`, which
`arm_save_fixture` does not provide and which is out of this file's
ownership to add.

## Defect the test caught (not mine to fix, owner fixed it)

Before any of my assertions ran, the rig panicked on the very first
`app.update()` inside Bevy's own schedule init (B0001, hard query conflict,
unconditional - not data-dependent):
```
thread '...' panicked at .../bevy_ecs-0.19.0/src/query/state.rs:216:13:
error[B0001]: ... accesses component(s) ... in a way that conflicts with a
previous system parameter. Consider using `Without<T>` ...
```
The backtrace named `nova_core::loading_screen::animate_loading_screen`;
its only two `&mut Text` queries were `Query<&mut Text, With<LoadingDotsMarker>>`
and `Query<&mut Text, With<LoadingResumeLineMarker>>`, neither with a
`Without<>` to prove them disjoint - this would panic the first time that
system runs in *any* app, not only this rig. `git blame` showed both lines
as uncommitted (in-flight D-T8 work by another worker in this shared
worktree), not mine to fix. I reported it via `subagent_ask` instead of
touching `loading_screen.rs`; the owner fixed it
(`q_resume_line: Query<&mut Text, (With<LoadingResumeLineMarker>, Without<LoadingDotsMarker>)>`,
`crates/nova_core/src/loading_screen.rs:490-493`, formatted with
`cargo fmt -p nova_core`) and I re-ran my test unchanged against the fix.

## Required mutation test

Copied `crates/nova_world_base/src/save/transients.rs` to `/tmp/transients.rs.orig`
before any edit.

First attempt matched the task's literal wording - made `refuse` skip its
call to `end_resume` (`transients.rs:436-440`) - but the test still passed.
Root cause: `restore_resumed_transients` has a *second*, unconditional
`end_resume(world)` call for "the session ended under the Load" (`transients.rs:317-324`),
and `nova_menu::pause::force_unpause`'s `OnExit(Playing)` safety net
(`crates/nova_menu/src/pause.rs:338-343`, `clocks.release_all()`, "every
surface that took a hold is being torn down with the scene") independently
releases `FreezeOwner::WorldResume` one frame later regardless of what
`refuse` itself did. Both are genuine, intentional redundancy already in
the codebase, not an artifact of my rig.

To exercise the mechanism P-T11 actually names (`end_resume` is what
releases the clocks on refusal), I mutated `end_resume` itself instead -
commented out its `clocks.release(FreezeOwner::WorldResume)` call
(`transients.rs:443-449`, leaving the two `remove_resource` calls intact) -
and added the "exact frame, still `Playing`" assertion (lines 272-286 above)
that checks the clocks *before* `force_unpause`'s safety net could ever run.
With that mutation:
```
thread '...' panicked at crates/nova_core/tests/world_resume_refusal.rs:278:5:
the refusal releases the clocks on the frame it happens, before `OnExit(Playing)`'s own safety net could
```
Confirmed this is the *only* mutation of the two I tried that the test
actually catches; the first (skipping `refuse`'s own `end_resume` call) is
masked by the redundant paths above and left unreported, since it is not
part of the final file state (reverted with the rest before the second
attempt).

Restored and verified byte-identical:
```
$ cp /tmp/transients.rs.orig crates/nova_world_base/src/save/transients.rs
$ cmp /tmp/transients.rs.orig crates/nova_world_base/src/save/transients.rs
byte-identical restore confirmed
```
Re-ran the test clean afterward (`ok`, see Evidence above).

## Verification

- `nix develop --command cargo fmt -p nova_core`: applied, no other diffs.
- `git status --short -- crates/nova_core/tests/world_resume_refusal.rs crates/nova_core/Cargo.toml crates/nova_world_base/src/save/transients.rs`:
  the two owned paths show as new/modified; `transients.rs` matches its
  pre-mutation content exactly (confirmed by `cmp` above, not by `git diff`,
  since nothing in this worktree is committed yet).
- Nothing staged, committed, or pushed.

## Case two: a saved owner id with no live ship - DONE

Added to the same `#[test]` (the binary still has exactly one, since it sets
`NOVA_CONFIG_ROOT`), as a second phase after case one's assertions, in
`crates/nova_core/tests/world_resume_refusal.rs:318-526`.

### File ownership (kept)

Still only the two owned paths:
- `crates/nova_core/tests/world_resume_refusal.rs` (grew from 347 to 530 lines).
- `crates/nova_core/Cargo.toml` `[dev-dependencies]`: added `ron = { version
  = "0.12" }` (`Cargo.toml:60-61`) to read the case-two save file directly
  and count its transients - `nova_world_base`'s own `ResumedTransients` is
  `pub(crate)`, so that check is closed to an outside crate; `WorldSaveState`
  (the whole-save RON type) is `pub`, the same type `open_world` parses with
  `ron::from_str` (`nova_world_base/src/save/mod.rs:288-289`).
- `crates/nova_world_base/src/save/transients.rs`: mutated and restored
  byte-identical via `/tmp` copy + `cmp`, same as case one; never
  `git checkout`.
- `nova_ship` needed no `Cargo.toml` change: it was already a normal (not
  dev) dependency of `nova_core` (`nova_core/Cargo.toml:20`), so
  `nova_ship::prelude::{thaw_round, FrozenRound, RoundSourceType}` was
  already reachable.

### Claim

The same test function now also proves P-T11's second case: a Load whose
window **is** live (unlike case one) but whose saved transient's owner id
has no live match is refused the same way - clocks released, `MainMenu`, no
`WorldSaveSession`, lock free, progress line empty, save files unchanged,
and the Load row names the owner.

### What it drives

- `crates/nova_core/tests/world_resume_refusal.rs:333-337`: the fixture
  player from case one is still alive (`PlayerSpaceshipMarker` query) - this
  minimal rig runs no `nova_world::Cleanup`, which is what despawns bodies
  in the full game (`nova_world_base/src/lib.rs:324-326`) when a config is
  removed. It fires a round built the same way
  `nova_world_base::save::tests::spawn_round` builds its fixture round
  (`save/tests.rs:574-598`, not importable - `pub(crate)` - so rebuilt here
  from the same public `thaw_round` + `resumed_lifetime`): lines 339-366.
- Lines 368-388: a second saved world ("Resume2"), armed and played until
  its first save reaches `Saved { generation: 1 }`. `CurrentSector` is
  re-inserted right before arming (lines 378-379) to mark it changed -
  this is not the app's first `app.update()` (case one already ran many),
  so `request_world_save`'s `current.is_changed()` gate
  (`nova_world_base/src/save/session.rs:301-303`) would otherwise never see
  a reason to save and the session would idle forever. Caught live: the
  first attempt without this line hung at `update_until`'s 2000-iteration
  cap, panicking "the second save never happened".
- Lines 390-393: the save file `state.1.ron` is read directly and parsed as
  `WorldSaveState` (the same type and the same `ron::from_str` call
  `open_world` uses, `save/mod.rs:288-289`, done here without the lock
  since `open_world` would refuse - this session still holds it) - asserts
  `transients.len() == 1`.
- Line 397: the owner is despawned - standing in for the `Cleanup` teardown
  and the missing scenario loader, the same kind of stand-in case one uses
  for the never-spawning `SectorRoot`. Confirmed safe against a spurious
  autosave crash: `snapshot_world`'s "no player ship" branch
  (`save/session.rs:338-342`) drops the request with a kept-last-save
  reason, it does not panic, and nothing in this rig re-requests a save
  after a plain despawn (`request_world_save`'s own gates are config
  change, scenario change under an armed session, or `CurrentSector`
  change - none of which a despawn trips).
- Lines 399-431: ESC, Retry, re-arm at `active_radius: 0`, then a
  `SectorRoot` at the origin (P-T1's `a_load_holds_the_world_until_every_saved_sector_is_live`
  pattern, `save/tests.rs:696`) - the window is live, unlike case one.
  `WorldResumeProgress { live: 1, desired: 1 }` and the loading screen
  shows `"RESTORING SECTORS 1 / 1"` while `resolve_all`
  (`save/transients.rs:373-411`) still waits on the owner.
- Lines 433-472: same shape as case one - comfortably under
  `WORLD_RESUME_SECONDS_MAX`, still held; past it, refused on the exact
  `Playing` frame, clocks released before `OnExit(Playing)`'s safety net.
- Lines 474-513: settled - `MainMenu`, no `WorldSaveSession`, the progress
  line and resource gone, and the Load row contains
  `"the ship 'player' is not in the window"` (the literal `resolve_all`
  wait reason, `save/transients.rs:393-395`, wrapped by `restore_resumed_transients`'s
  refusal message, `save/transients.rs:357-360`).
- Lines 515-525: the lock is free (`open_world` on "Resume2" succeeds) and
  `state.1.ron`/`world.lock` are byte-identical to before Retry.

### Defect caught while building this (not a product bug, my own setup bug)

Case one's bound-breach step leaves `TimeUpdateStrategy::ManualDuration` at
2 real seconds per frame. Every case-two setup frame after that (the round
thaw, the second save's wait loop, the Retry wait loop, the re-arm) ran at
that same 2 s/frame until I added an explicit reset back to 16 ms
(`world_resume_refusal.rs:322-325`). Without it, elapsed `Time<Real>` had
already passed `WORLD_RESUME_SECONDS_MAX` before the deliberate "under the
bound" check ran, so the test refused far too early:
```
thread '...' panicked at crates/nova_core/tests/world_resume_refusal.rs:441:5:
still under the bound
```
Caught by the same "still under the bound" assertion case one already
relied on, not a new one. Fixed by resetting `TimeUpdateStrategy` back to
16 ms/frame at the top of case two (`world_resume_refusal.rs:328-331`).

### Required mutation test (case two)

Copied `crates/nova_world_base/src/save/transients.rs` to
`/tmp/transients.rs.orig` before any edit (confirmed identical by `cmp`).

Mutated `resolve_all` (`save/transients.rs:393-395`) so
`TransientRefFault::Missing` pushes `Entity::PLACEHOLDER` into `resolved`
instead of recording a wait reason - the literal change the task specified.
With that mutation, case two's resolve succeeds immediately instead of
waiting, so `restore_resumed_transients` spawns the (now owner-less)
transient and calls `end_resume` on the very first frame the window is
live, before the Loading screen's text system ever draws
`"RESTORING SECTORS 1 / 1"`. The test failed exactly there:
```
thread 'a_load_that_cannot_fill_its_window_is_refused_and_releases_the_clocks' panicked at crates/nova_core/tests/world_resume_refusal.rs:89:5:
the resume progress line to render (case two) never happened
```
(line 89 is `update_until`'s own `panic!`; I gave the case-two call its own
label, `"... (case two)"`, so this failure is unambiguous against case
one's identically-worded wait.)

Restored and verified byte-identical:
```
$ cp /tmp/transients.rs.orig crates/nova_world_base/src/save/transients.rs
$ cmp /tmp/transients.rs.orig crates/nova_world_base/src/save/transients.rs
byte-identical restore confirmed
```

### Evidence

- `nix develop --command cargo check -p nova_core --tests -j 8`: clean.
- `nix develop --command cargo test -p nova_core --test world_resume_refusal -j 8`,
  run 3x clean after the restore:
  ```
  running 1 test
  test a_load_that_cannot_fill_its_window_is_refused_and_releases_the_clocks ... ok

  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
  ```
  (repeated at `0.08s` and `0.06s` on the other two runs; deterministic.)
- `nix develop --command cargo fmt -p nova_core`: applied (import list and
  one wrapped line); re-ran the test clean afterward.
- Nothing staged, committed, or pushed.
