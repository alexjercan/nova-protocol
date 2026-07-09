# Review: PR #54 - Combat juice (camera shake + gizmo hit/impact flash)

- PR: https://github.com/alexjercan/nova-protocol/pull/54
- BRANCH: feature/hit-feedback-juice (2 commits) -> master
- REVIEWED AT: 1301813 (branch tip)
- TASK: 20260708-162013 (1 review round on-branch, APPROVE, both MINORs addressed)
- VERDICT: **APPROVE** (no blockers or majors; merge as is. The findings below
  are follow-ups, filed as tasks - see "Follow-ups" at the end)

## Scope

Adds moment-to-moment combat feedback: trauma-based camera shake (reusing the
bcs `CameraShakePlugin`) and camera-facing gizmo flash rings, driven off the
same `On<HealthApplyDamage>` / `On<Add, IntegrityDestroyMarker>` seams the
audio layer uses. Distance-attenuated from the gameplay camera, per-area-cell
throttled, all tunables on a reflected `JuiceSettings` resource. One new module
`crates/nova_gameplay/src/juice.rs` (867 lines incl. 15 tests), two wiring
lines, a design note, the task record with its on-branch review, and a retro.

## Verification (re-run at branch tip, fresh worktree)

Every claim in the PR body was re-verified rather than trusted:

- `cargo fmt --check` clean; `cargo clippy --all-targets` clean.
- `cargo test --workspace` green: 74 nova_gameplay unit tests (15 juice: 10
  pure-helper + 5 observer-level integration), 5 nova_scenario, and the
  `examples_smoke` harness test (44s) all pass.
- `cargo check --target wasm32-unknown-unknown -p nova_gameplay` passes, so the
  wasm-safety claim (gizmos, no particles) is real.
- TASK.md steps, the design note, and the retro all match what the code
  actually does. The Round-1 REVIEW.md responses (R1.1 full reflect tree, R1.2
  observer-level tests) are genuinely resolved in the diff.
- Feel itself is not headlessly verifiable, but the retro records a real
  playtest iteration (shake retuned down, tighter falloff than audio), so the
  effect has been seen running.

## What is good

- **Right reuse.** The drift-free shake is fed, not reimplemented - the exact
  "check bcs first" lesson from the audio retro, applied. `ensure_camera_shake`
  + `sync_camera_shake_config` handle camera respawn and live settings edits.
- **Right rendering tradeoff, well argued.** Gizmo rings are wasm-safe (the
  particle system is still wasm-blocked, 162908), zero-churn under blast
  bursts, and sidestep the shared-gltf-material recoloring trap. The rejected
  alternatives are written down in the design note.
- **The audio review's test finding (F2) was learned, not repeated.** Juice
  ships with observer-level integration tests from day one: trauma lands,
  flashes queue, co-located bursts collapse, distinct cells both fire, master
  switch suppresses. The audio module still lacks these (open task
  20260708-224303).
- **Tunable for real.** The whole `JuiceSettings` tree (nested structs + enum)
  is registered for reflection, so the inspector / future settings menu can
  actually traverse it. `default_settings_are_sane` guards the invariants.
- **Honest docs.** Design note, retro, and TASK.md are accurate, including
  limitations (entity-transform sourcing instead of contact points, R1.3).

## Findings

Severities: BLOCKER / MAJOR / MINOR / NIT. None are blocking; the first two
are real and got follow-up tasks.

- [ ] F1 (MINOR, confirmed empirically) `crates/nova_gameplay/src/juice.rs:488,518`
  (`on_damage_juice`, and `on_destroy_juice` is exposed via the same seam) -
  `HealthApplyDamage` is `#[entity_event(propagate, auto_propagate)]` and the
  game *depends* on the bubbling (ship death: the fatal section hit propagates
  to the root, see `integrity/glue.rs:118`). A global observer fires once per
  propagation hop, so one hit on a section fires juice twice: once at the
  section, once at the ship root. The per-cell throttle collapses the pair only
  when both positions quantize to the same 6-unit cell; when they straddle a
  boundary (grid cells are absolute-aligned, ships move continuously), one hit
  yields **2x trauma (0.16 vs the tuned 0.08) and a phantom second ring at the
  ship's root origin**. Verified with a scratch minimal-App test: parent one
  cell away from the damaged child -> `flashes = 2, trauma = 0.16`. This is the
  juice twin of PR #53 review F3 (audio), where the fix ("key by the entity's
  `IntegrityRoot`") was noted but never filed as a task - it is now twice-
  duplicated behavior with no tracking. Bevy 0.19 exposes
  `On::original_event_target()` on propagating entity events, so the minimal
  fix is a one-line guard (`if damage.entity != damage.original_event_target()
  { return; }`) in each damage observer (audio + juice). Filed as a follow-up
  task (see below) rather than a re-open: it equally affects shipped audio
  code, the right fix is shared, and severity matched the precedent set when
  F3 was deferred.

- [ ] F2 (MINOR) `crates/nova_gameplay/src/juice.rs:309,398,552` - the PR adds
  three more call sites that assume "the first `Camera3d` is the gameplay
  camera": `listener_position` (attenuation), `ensure_camera_shake` (attaches
  `CameraShake` to *any* `Camera3d`, including the editor camera - it is not
  state-gated), and `draw_juice_flashes` (ring facing). `emit_juice` also
  broadcasts trauma to every `CameraShakeInput` in the world. Open task
  20260708-224254 (PR #53 review F1: dedicated listener marker) is scoped to
  `audio.rs` only, so fixing it there would leave juice jittering the same way
  the moment a second camera coexists. The open task's scope has been extended
  to cover the juice call sites (task file updated in this PR's branch).

- [ ] F3 (MINOR, tests) `crates/nova_gameplay/src/juice.rs` (tests) - all five
  integration tests run without a camera, so the attenuation path through the
  observers is only covered at the pure-helper level. Nothing pins the two
  behaviors that depend on the listener being present: a fully-attenuated event
  does nothing *and does not stamp the throttle*, and a mid-range event scales
  trauma by the falloff. One test spawning a `Camera3d` past `far_distance`
  (and one mid-ramp) would cover the `falloff <= 0.0` early return, which is
  load-bearing for the "far skirmish stays quiet without consuming throttle
  state" claim. Folded into the F1 follow-up task since the fix touches the
  same observers.

- [ ] F4 (NIT) `crates/nova_gameplay/src/juice.rs:2` - the module doc opens
  with "Three effects" and then lists two seams and two effects (shake +
  flash). The task's original three (shake, per-section hit flash, contact
  impact FX) deliberately collapsed to two during design; the doc header just
  kept the old count. One-word fix, folded into the F1 task.

- [ ] F5 (NIT) `crates/nova_gameplay/src/juice.rs:587` - each trailing ring
  lags by a hardcoded `0.15` of the flash lifetime while everything else moved
  into `FlashSettings`; and at the `MAX_ACTIVE_FLASHES` cap the *newest* flash
  is dropped (the most recent event is arguably the one the player should
  see). Both are backstop/polish territory; take or leave.

- [ ] F6 (NIT) toggling `shake.enabled` / `master_enabled` off mid-shake stops
  new trauma but lets the in-flight shake decay out naturally instead of
  snapping to rest (`CameraShakeInput.reset` exists for that). For a "reduce
  motion" accessibility switch, an instant reset on the disable edge would be
  the more correct behavior. Worth one line in the eventual settings-menu
  task; not filed separately.

## Notes (not findings)

- The audio/juice scaffold duplication (throttle map + `area_cell` +
  `listener_position` + two observers, ~80 lines, deliberately different
  attenuation curves) is at two copies - acceptable under the rule of three.
  The spike below records the extraction/promotion direction so a third
  consumer (rumble, HUD damage direction, hit-stop) triggers it deliberately.
- Impact/destroy cues at the same location within one frame both fire (their
  throttle keys differ by kind). That is correct: a killing blow should read
  as hit + explosion, and it matches the audio layer.
- `JuiceSettings` is not persisted anywhere yet; that is explicitly the
  settings-menu follow-up already recorded in the task/docs, not a gap.
- TASK.md's `Spike:` header line points at the modding-language spike for the
  roadmap; the reprioritization that made this task the p88 headliner is
  `docs/spikes/20260708-203517-roadmap-reprioritization-and-juice.md`.
  Pre-existing on master, cosmetic, not part of this diff.

## In this PR or follow-ups?

Follow-ups. The branch is approved, retro'd, and its two commits are clean;
none of the findings is severe enough to reopen an APPROVE (the F1 behavior
already ships today in the audio layer, deferred there under the same
severity). Reopening for the NITs would buy nothing; F1's correct fix spans a
module this PR does not touch (`audio.rs`), so it is its own change with its
own tests either way.

## Follow-ups (filed on this branch)

- Spike: `docs/spikes/20260709-091536-combat-cue-propagation-dedup.md` -
  weighs original-target guard vs IntegrityRoot keying vs shared cue seam.
- tatr 20260709-091756: dedup `HealthApplyDamage` propagation in audio + juice
  observers (F1), with hierarchy + attenuation observer tests (F3) and the doc
  count fix (F4).
- tatr 20260708-224254 (existing, updated): listener robustness scope extended
  from `audio.rs` to the juice call sites (F2).

## Recommendation

Merge (squash), same as PR #53. The follow-ups ride the branch so they land
with it.

## Addendum: GitHub Copilot PR comments (post-review)

Copilot left four inline comments on the PR; triaged against this review:

- **F7 (MINOR, valid - missed by this review): flash was distance-culled, not
  distance-attenuated.** Three of the comments are one finding: `emit_juice`
  scaled the *trauma* by the falloff but used it only as an on/off gate for the
  flash - `Flash` recorded no strength and `flash_alpha` was purely
  lifetime-based, so a ring at 150u drew at the same alpha as one at 5u,
  contradicting the module doc / design note / PR body ("distant events flash
  weaker"). Fixed on-branch rather than filed: `Flash` now captures the emit-
  time falloff as `strength` and the draw scales alpha by it (alpha only -
  radius stays world-scale, since perspective already shrinks a distant ring
  and scaling radius too would double-attenuate). Two observer-level tests
  added (mid-ramp camera -> trauma and flash strength at exactly 0.5; camera
  past `far_distance` -> no cue and no throttle stamp), which also discharges
  F3; the F4 doc-count nit was fixed in the same pass. Full check suite re-run
  green (fmt, clippy, 76 nova_gameplay tests + smoke, wasm check).
- **The fourth comment is F1** (propagation double-fire), independently found
  and confirmed empirically above; it stays a follow-up via spike
  20260709-091536 and task 20260709-091756. Copilot's suggested mechanism
  (skip entities matching `IntegrityRoot + SpaceshipRootMarker`) filters by
  what the *parent* is and would still double-fire for any future intermediate
  hierarchy levels (and asteroid bodies without the ship marker); the spike's
  `original_event_target()` guard dedups by propagation itself, which is the
  cause.
