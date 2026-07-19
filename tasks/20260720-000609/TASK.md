# Harness completion protocol (bcs upstream): collectors register/done, deadline names laggards; nova adoption deletes exit-ownership folklore

- STATUS: CLOSED
- PRIORITY: 62
- TAGS: v0.8.0,tooling,testing,refactor

## Goal

The harness completion protocol (spike tasks/20260719-235305/SPIKE.md,
D1 + R1-R3): collectors NEGOTIATE the exit instead of racing clocks.
Upstream-first in bevy-common-systems (the proven rhythm - v0.19.2), then
nova adoption + pin bump.

### bcs side (the protocol)

- `HarnessCompletion` (name TBD at implementation): armed collectors
  REGISTER at plugin build ("autopilot", "screenshot", plus externals
  like nova's capture); each reports DONE when its own clock completes;
  when the pending set empties the coordinator writes `AppExit::Success`.
  Single-collector behavior is IDENTICAL to today by construction.
- Deadline backstop: if the pending set is not empty after a configurable
  budget (env-overridable; must resolve BELOW any supervisor timeout -
  spike R3), exit with `AppExit::error` and LOG THE LAGGARDS BY NAME -
  probe's process_exit + log then show "capture never completed" instead
  of a silent SIGKILL.
- AutopilotPlugin: stops writing AppExit directly; after the last hold
  it runs that frame's input hook, THEN reports done (assert-then-done,
  spike R1 - a failing assert must panic before the exit is negotiated).
  ScreenshotPlugin converted the same way.
- bcs's OWN examples/tests checked and updated (spike R2 - exit
  semantics change; consumers beyond nova must not be assumed).
- Release + tag; nova bumps the pin (dev loop via a temporary [patch]
  path dep, removed before landing).

### nova side (adoption)

- FrameTimePlugin: registers "capture" when armed, reports done at
  window close; its direct AppExit write goes through the protocol.
- perf_baseline: the `!perf_armed()` conditional DELETES - autopilot +
  probe wiring become unconditional; with both collectors registered the
  exit happens when both finish (the sweep still disarms the recorder
  surfaces via env, unchanged).
- broadside: the script's self-exit becomes its done report; the
  completion guard stays as the belt over the protocol's braces.
- In-example assertions KEEP their elapsed-based timing (they fire
  before the autopilot's done by construction); migrating them onto a
  completion hook is offered by the new API but NOT this task's sweep.

## Steps

- [x] bcs: protocol resource + register/done + deadline system with
      laggard naming; AutopilotPlugin + ScreenshotPlugin converted
      (assert-then-done ordered + tested); bcs examples/tests green.
- [x] bcs release: version + tag decided in-cycle; push + tag needs the
      user's go (outward-facing), requested at the stop point.
- [x] nova: [patch]-based dev loop; capture converted; perf_baseline
      conditional deleted; broadside adapted; pin bumped when the tag
      exists; [patch] removed.
- [x] Tests: protocol units in bcs (register/done/empty-set exit,
      deadline + laggard names, single-collector parity); nova capture
      conversion pins.
- [x] E2E in nova: `probe run perf_baseline` (both collectors, no
      conditional), `probe run scenario --fps` (capture outlives the 6s
      autopilot and the app WAITS - the spike's headline case),
      `probe run broadside` (self-ending + guard). process_exit clean on
      all three; a forced-laggard run (capture armed, window impossible)
      exits error naming "capture".
- [x] Verify: fmt both repos; cargo test -p nova_probe; affected
      examples compile; degrade path (deadline) exercised once for real.

## Notes

- Spike: tasks/20260719-235305/SPIKE.md (adjudicated 2026-07-20).
- S2 (20260720-000616) builds on this: always-split fps pass + scene
  looping + reload lines. 233732 (partial-emit net) comes after S2.
- The 210443 exclusive-ownership lesson is SUPERSEDED by
  no-unilateral-exit - record the supersession in LESSONS if a ledger
  entry exists for it.

## Progress (2026-07-20, at the release stop point)

bcs branch feature/harness-completion @ f8ed9db (worktree
~/.cache/sprouts/bevy-common-systems/feature/harness-completion):
completion.rs at the CRATE ROOT (ungated - nova's capture builds
featureless for wasm; re-exported at debug::harness::completion),
autopilot + screenshot converted, self_completing() added, 8 harness lib
tests + 5 protocol tests green in BOTH feature configurations, examples
check link-free (full bcs suite NOT run - it OOMs the box, see
bcs-no-full-test-suite), version 0.19.3 + CHANGELOG.

Nova adoption (this branch): capture registers "capture" and reports
done (CAPTURE_COLLECTOR); perf_baseline's exit-ownership conditional
DELETED; broadside .self_completing() with the script's negotiated done
(sentinel log kept for the smoke grep). Dev loop: temporary PATH deps in
all FIVE bcs-dependent manifests ([patch] rejects a version-bumped patch
of a git-tag dep - see upstream-dev-via-patch-not-premature-push);
restore all five + bump to git tag v0.19.3 in the landing commit.

E2E (all live, patched local bcs):
- A perf_baseline: exit 0, negotiated single-collector exit, no
  conditional.
- B THE HEADLINE: `probe run scenario --fps` with DEFAULT windows ->
  frametime.csv with the FULL 900 frames; run.log shows "autopilot done
  (1 still pending)" - the app WAITED for the capture that previously
  lost 229 samples by 11 frames. (The tail measures post-script idle -
  S2's scene looping owns that.)
- C broadside: script sentinel + negotiated exit, guard quiet, exit 0.
- D forced laggard (BCS_HARNESS_DEADLINE=10, impossible window): exit 1,
  "deadline (10s) expired with collectors still pending: [autopilot,
  capture]" in the log, probe FAILs the run with artifacts intact.

REMAINING: user pushes bcs branch + tags v0.19.3 -> restore the five dep
lines to git+tag v0.19.3 -> retest -> land both repos.

## Close-out (2026-07-20, released + repinned)

bcs v0.19.3 SHIPPED: master 3f6f7c8, tag pushed (user-authorized). All
five nova dep lines restored to git+tag v0.19.3; Cargo.lock resolves the
pushed commit; nova_probe 80/80 against the PUBLISHED tag (not the local
path). The exit-ownership folklore is gone: one protocol, negotiated
success, aborting failures, laggards named at the deadline.
