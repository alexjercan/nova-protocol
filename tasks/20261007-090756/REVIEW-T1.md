# Review: resumable-worlds T1 slice (gun/railgun round resume)

Scope: T1 of the transient-resume feature (rounds only; torpedoes/shed
fixtures/rock chunks/detached pieces are T2/T3, out of scope). Reviewed the
uncommitted working-tree diff on branch `resumable-worlds` against the design
doc read earlier this session (`tasks/20261007-090756/TRANSIENT-GATE.md` rev 2,
`T1-PLAN.md`). Read-only: no files edited, staged, or committed; no cargo,
tests, or Clippy run.

## Process note (not a code defect)

`tasks/20261007-090756/` no longer contains `TRANSIENT-GATE.md` or
`T1-PLAN.md`. Between this review's first and second halves the folder was
overwritten with a different sub-task's scratch files (`CAMERA-GATE.md`,
`GATE.md`, `SCOUT-ART.md`, `PROOF-*.md`, `REVIEW-S123.md`, `REVIEW-S4.md`,
all untracked, timestamped today). Only `TASK.md` (git-tracked, confirms this
is the "Deliver resumable New Game worlds" epic) survived. This matches the
known failure mode where parallel lanes share one worktree's scratchpad.

This review's code findings are unaffected: every path:line citation below was
re-verified against the current working tree just before writing this report,
not reconstructed from memory of the missing docs. The design intent recalled
from the earlier read of TRANSIENT-GATE.md (sections 2-4, 10, 12 D-T1..D-T8)
is used only as context for judging whether the code matches its own stated
contract, not as something I re-read from disk. Flagging this so the design
docs get restored or re-committed before another reviewer relies on them.

## Checked and found clean

- **`crates/nova_gameplay/src/saved_refs.rs`** (new, 98 lines): `SavedOwner`,
  `SavedBodyRef`, `SavedSectionRef`, `TransientFreezeFault`. Doc comments,
  serde gating (`#[cfg_attr(feature = "serde", ...)]`, `deny_unknown_fields`),
  and `SavedOwner::of` all correct. No issues.
- **`crates/nova_gameplay/src/lifetime.rs`**: `SavedLifetime::of`,
  `resumed_lifetime` build a `TempEntityState` with `elapsed = total -
  remaining`; the pre-existing `on_insert_temp_entity` observer leaves an
  already-present `TempEntityState` alone, so a thaw's seeded elapsed time
  survives. Correct.
- **`crates/nova_gameplay/src/freeze.rs`**: `FreezeOwner::WorldResume` added
  as a fourth variant, included in `ALL`. `Clocks`/`ClockFreeze` logic
  unchanged and still correct for the new owner.
- **ResumedRound zero-impact skip** (`crates/nova_gameplay/src/rounds.rs:944`,
  `:1034`, both `if resumed && hit.impact == 0.0 { bitten.remember(...);
  walk.advance(...); continue; }`): traced this against the task's explicit
  worry ("could it skip a real hit at TOI 0 on a body it was not inside?").
  Every body in the resumed window shares one identical start-of-step pose on
  the first physics tick after a Load (the whole window unfreezes together
  via the single `FreezeOwner::WorldResume` release), so a TOI=0 result can
  only reflect a genuine saved-time overlap, never a body that moved into the
  round's position during the hold. Exercised by a real (non-mocked) physics
  test, `a_resumed_round_inside_a_plate_does_not_bite_it_again` (rounds.rs,
  ~line 3098), which asserts exactly one hit of damage, not two. Sound.
- **Owner resolution, Gone vs PLACEHOLDER** (`transients.rs:373-...`,
  `resolve_all`): `SavedOwner::Gone` -> `Entity::PLACEHOLDER`,
  `SavedOwner::Ship(id)` -> `live_by_id` filtered `With<SpaceshipRootMarker>`
  (Missing/Ambiguous handled). `Entity::PLACEHOLDER` is bevy's reserved
  sentinel, never allocated to a real spawn, so this cannot later alias a
  live entity. Correct, matches the live-play "authorless round" semantics
  `ProjectileOwner`'s absence already carries.
- **`ResumedTransients` guard vs save**
  (`crates/nova_world_base/src/save/transients.rs:161-165`, inside
  `freeze_transients`): `if world.contains_resource::<ResumedTransients>() {
  return Err(unsettled("the saved transients", "the world is still
  resuming")) }`, checked first, before the body query. Confirmed by test
  `a_load_saves_nothing_until_its_transients_are_back` (save/tests.rs, ~line
  722): the arming-frame save stays `Waiting(...)` and the on-disk file is
  byte-unchanged while the resource exists. Correct.
- **`snapshot_world` get_resource guard** (`save/session.rs`): early-returns
  without `WorldSaveSession`; drops the wanted save with no single player
  ship or no armed `WorldConfig`; ordering `CameraView::capture ->
  snapshot_sectors -> freeze_transients -> freeze_ship` matches the design.
  Correct.
- **`refuse_resumed_world` panel-not-yet path**
  (`crates/nova_menu/src/load_screen.rs:165-196`): unconditionally clears
  `WorldSaveSession`/state/pause first (idempotent on repeat), then `let Some
  (mut panel) = panel else { return; }` if `LoadPanel` is not spawned yet.
  The system is scheduled `run_if(resource_exists::<WorldResumeRefused>)`, so
  it reruns next frame once `LoadPanel` spawns at `OnEnter(MainMenu)`; only
  then does it rebuild the listing, write the refusal reason onto the row,
  show the panel, and remove `WorldResumeRefused`, which stops the run_if.
  No infinite loop, no double teardown.
- **`FreezeOwner::WorldResume` release on every exit path**: restore success
  -> `spawn_resumed` + `end_resume` (`transients.rs:340-341`); refusal (bound
  exceeded or `Resolve::Refuse`) -> `refuse` -> `end_resume`
  (`transients.rs:436-439`), confirmed `end_resume` is still called on the
  current disk state of `refuse` (re-checked at the end of this review, after
  being told another worker is actively mutating this file); session gone
  mid-Load -> `end_resume` (`transients.rs:321-323`); any other state exit
  (e.g. closing the pause menu, leaving to main menu) ->
  `clocks.release_all()` (`crates/nova_menu/src/pause.rs:342`), which
  releases every named hold including `WorldResume`. All four exit paths
  verified to end the hold.
- **Launch-cue move (D-T2), leftover-adapter search**: grepped every
  `On<Add, (TurretBulletProjectileMarker|TorpedoProjectileMarker|
  RailgunSlugProjectileMarker)>` and every `RoundFired`/`TorpedoLaunched` use
  across `turret_section/{firing.rs,mod.rs,render.rs}`,
  `torpedo_section/{bay.rs,mod.rs,render.rs}`, `ship_audio/combat.rs`,
  `railgun_section/{mod.rs,render.rs}`. The 4 one-shot cue observers
  (`on_projectile_marker_effect`, `on_turret_fire_play_sfx`,
  `on_torpedo_launch_effect`, `on_torpedo_launch_play_sfx`) are moved to
  `On<RoundFired>`/`On<TorpedoLaunched>`, triggered only from the real fire
  paths (`turret_section/firing.rs:393`, `torpedo_section/bay.rs` near the
  launch bundle insert). The persistent render/visual-state observers
  (`insert_projectile_render`, `insert_railgun_slug_render`,
  `insert_torpedo_render`) correctly remain on `On<Add, Marker>`, since both
  a live fire and a thaw need render children rebuilt. No leftover dual path,
  no test or example found spawning a bare marker and expecting a cue.
  `frozen_rounds.rs`'s own test, `a_resumed_projectile_plays_no_launch_cue`
  (~line 306), exercises this against the real `TurretSectionPlugin` and
  `ShipAudioPlugin`, not hand-rolled triggers.
- **`FrozenRound`/`freeze_round`/`thaw_round`**
  (`crates/nova_ship/src/sections/frozen_rounds.rs`): panics on a missing
  `BulletProjectileRenderMesh`, panics if the render mesh handle has no asset
  path ("cannot be saved"), panics if the entity carries neither projectile
  marker. `thaw_round` correctly omits `TurretSectionPartOf`/
  `TurretSectionMuzzleEntity`/wake per spec. Fail-loud, matches the gate's
  table.
- **`sections/frozen.rs` collider refactor**: drops the raw `Collider` field
  (no serde impl existed for it) for a `collider_size: Vec3` on `Decor` and a
  `ShellShape` rebuild on `Plate`; round-trip test
  `a_frozen_plate_and_decor_thaw_with_the_colliders_they_froze_with` asserts
  AABB equality pre/post. Sound, correctly scoped prep work.
- **Scheduling** (`crates/nova_world_base/src/lib.rs:150-220`):
  `restore_resumed_world` ordered `.after(Cleanup).before(Observe)`;
  `(restore_resumed_transients, save_systems()).chain().after(Retire)`, both
  `run_if` resource-gated and `#[cfg(not(target_arch = "wasm32"))]`. Matches
  the design's stated ordering rationale.
- **`nova_core`/`nova_menu` window-close and leave wiring**: native
  `close_when_requested: false` with a `nova_menu`-owned
  `WindowCloseRequested` handler, `nova_core`'s own fallback registered only
  when `has_menu` is false; `refuse_resumed_world` registered ungated by
  `GameStates` with a comment explaining a refusal can land while still
  `Playing`. Correctly scoped.

## Findings

### MINOR: resume progress line can read 100% while genuinely stuck on an unresolved owner

- **Claim**: `WorldResumeProgress{live, desired}` is computed only from the
  sector window, not from whether every saved owner has resolved, so the
  Loading screen can show "RESTORING SECTORS N / N" for up to
  `WORLD_RESUME_SECONDS_MAX` (240 s) before a refusal fires, with no visible
  indication that the hold is actually stuck on a different cause.
- **Evidence**: `crates/nova_world_base/src/save/transients.rs:325-331` sets
  `WorldResumeProgress` from `window_still_missing` alone, before
  `resolve_all` runs at `:338`. If `missing.is_empty()` but `resolve_all`
  returns `Resolve::Wait(reasons)` (`:344`), the function falls through to
  the real-seconds bound check (`:352-362`) without touching
  `WorldResumeProgress` again, so it stays at `live == desired`.
- **Failure scenario**: a saved world references a ship id that for any
  reason never resolves inside the window (a genuinely corrupt/edited save,
  or a future regression elsewhere in id-resolution). The player sees a
  progress line claiming the restore is complete for up to 240 real seconds
  before the screen flips to the refusal panel, with no on-screen signal of
  what it is actually waiting on in the interim.
- **Suggested fix**: either fold the owner-wait reasons into
  `WorldResumeProgress` (e.g. an optional "waiting on <reason>" line read by
  `loading_screen.rs`'s `animate_loading_screen`), or accept this as a known
  simplification since the owner-resolve step should resolve in the same
  frame the window completes in the designed case (the saved `CurrentSector`
  reproduces the same desired set, and every saved owner ship is inside it).
  Not a functional bug for the designed path, since P-T1's own test only
  exercises the sector-missing phase; this matters only for an
  already-unexpected stuck state. I did not find a test exercising
  "window live, owner still unresolved" to confirm the intended behavior
  here — flag as unverified against design intent rather than a confirmed
  defect; recommend pointing this out to whoever owns D-T1/D-T8 for an
  explicit call.

### MINOR: docs not yet updated for this change (listed per the task's instruction; slice 5 owns the fix)

- **`docs/architecture.md:382-383`**: "`ClockFreeze` counts
  `FreezeOwner::{PauseMenu, Interface, ScenarioLoad}`" no longer lists the
  new `FreezeOwner::WorldResume` (added at
  `crates/nova_gameplay/src/freeze.rs`, 4th variant in `ALL`). Stale
  enumeration; a reader will not know a Load can hold the clocks.
- **`CHANGELOG.md`**: the `## [Unreleased] > ### Gameplay & Flight` section
  has no entry for this slice's player-visible behavior change: a fired
  round or railgun slug no longer vanishes on a save/Load, it resumes on its
  saved trajectory and expires on its saved lifetime, and a resumed round
  plays no launch cue. AGENTS.md requires one entry per released change
  under `[Unreleased]`; this is squarely player-visible gameplay behavior,
  not an internals-only change.
- Both confirmed to have zero uncommitted diff (`git diff HEAD --stat`
  against each path is empty), so this is not an oversight mid-edit, it is
  simply not done yet. Listed per the task's instruction ("slice 5 owns most
  docs, so just list them"), not fixed here.

### Unverified / theoretical, not a grounded defect

- A body spawned by a scenario action at the exact coordinate a saved round
  occupies, landing on the very first resumed tick before the round's next
  cast, could in principle produce a TOI=0 the `resumed` skip would treat as
  a pre-existing overlap. I found no mechanism that makes this a real risk
  (scenario spawns are not placed at a specific prior round's saved
  position), and no evidence of it happening; raising only because the
  design doc itself flags the underlying avian cast-at-TOI=0 behavior as
  "Unverified" and I could not independently confirm avian's cast semantics
  beyond what the existing test already covers.
- `crates/nova_menu/src/leave.rs`: pressing Leave while
  `FreezeOwner::WorldResume` is still held (mid-Load) correctly stalls any
  concurrent save attempt via the `freeze_transients` guard above, but I did
  not reach a full closed-form proof that `refuse_resumed_world`'s teardown
  and a simultaneous `PendingLeave` teardown cannot both try to tear down
  `WorldSaveSession`/game state in the same frame. No evidence of an actual
  double-teardown found; flagging as unclosed rather than asserting a bug.

## Skipped / out of explicit scope

- No workspace-wide tests, `cargo check`, or Clippy run (per constraint).
- No game or GPU measurement run.
- Reviewed only the files in the task's explicit list, plus
  `crates/nova_menu/src/leave.rs` and `crates/nova_menu/src/pause.rs`
  for `FreezeOwner` exit-path context (not in the explicit list, read for
  cross-reference only).
- Not reviewed in depth: `crates/nova_menu/src/menu_ui.rs`,
  `crates/nova_menu/src/world_setup.rs`, `crates/nova_scenario/**`,
  `crates/nova_ship/src/camera/**`, `crates/nova_ship/src/input/ai/**`,
  `crates/nova_assets/**`, `crates/nova_world/**` beyond the one
  `SectorSnapshotError::NoDurableId` variant check — these appear in the
  branch's overall diff but are prior committed "frozen sectors" work
  (confirmed via `git log --oneline -- <path>` / `git merge-base`), not part
  of the uncommitted T1 slice, or are untouched serde-prep groundwork
  (`ammo.rs`, `cargo_intake_section.rs`, `integrity.rs`,
  `section_animation.rs`, `railgun_section/mod.rs`,
  `turret_section/{setup.rs,stow.rs}`, `skin_style.rs`,
  `cooldown.rs`, `damage.rs`, `integrity/{carve.rs,health.rs}`,
  `inventory.rs`, `mesh/field.rs`) reviewed only enough to confirm they are
  additive `#[cfg_attr(feature = "serde", ...)]` annotations with no logic
  change.
- `web/src/wiki/*.md` checked via `git diff HEAD --stat`: zero uncommitted
  diff, not touched by T1.

## Verdict

No BLOCKER or MAJOR defects found in the T1 slice. The core restore
mechanism (`transients.rs`), the freeze/thaw round-trip
(`rounds.rs`/`frozen_rounds.rs`), the durable-reference resolution
(`saved_refs.rs`), the launch-cue redesign, and the clock-hold lifecycle all
match their stated design intent and are backed by real (non-mocked) proofs
already in the diff. Two MINOR findings: a progress-indicator precision gap
during an unresolved-owner stall, and two docs not yet updated for this
slice's player-visible behavior change.

## Main worker response

- Process note: false. `TRANSIENT-GATE.md` (889 lines) and `T1-PLAN.md`
  (97 lines) are present and intact in the sprout worktree
  `tasks/20261007-090756/`. The reviewer most likely listed another
  checkout. No action.
- MINOR progress line: accepted as is. The line names sectors, and
  "RESTORING SECTORS N / N" is true while the hold waits for owner ships.
  The 240 s refusal gives the full reason on the Load row
  (`transients.rs:352-361`). A wait-reason line would be a new
  `WorldResumeProgress` field and screen text, which is not approved. Raise
  it with the owner at slice 5 if wanted.
- MINOR docs: slice 5 owns `docs/architecture.md:382-383` and the
  CHANGELOG entry. Added to the slice 5 list.
- Unverified Leave plus refusal teardown: no defect claimed. P-T11 covers
  the refusal teardown; the leave tests cover the leave teardown.
