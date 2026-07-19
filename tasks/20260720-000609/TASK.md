# Harness completion protocol (bcs upstream): collectors register/done, deadline names laggards; nova adoption deletes exit-ownership folklore

- STATUS: OPEN
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

- [ ] bcs: protocol resource + register/done + deadline system with
      laggard naming; AutopilotPlugin + ScreenshotPlugin converted
      (assert-then-done ordered + tested); bcs examples/tests green.
- [ ] bcs release: version + tag decided in-cycle; push + tag needs the
      user's go (outward-facing), requested at the stop point.
- [ ] nova: [patch]-based dev loop; capture converted; perf_baseline
      conditional deleted; broadside adapted; pin bumped when the tag
      exists; [patch] removed.
- [ ] Tests: protocol units in bcs (register/done/empty-set exit,
      deadline + laggard names, single-collector parity); nova capture
      conversion pins.
- [ ] E2E in nova: `probe run perf_baseline` (both collectors, no
      conditional), `probe run scenario --fps` (capture outlives the 6s
      autopilot and the app WAITS - the spike's headline case),
      `probe run broadside` (self-ending + guard). process_exit clean on
      all three; a forced-laggard run (capture armed, window impossible)
      exits error naming "capture".
- [ ] Verify: fmt both repos; cargo test -p nova_probe; affected
      examples compile; degrade path (deadline) exercised once for real.

## Notes

- Spike: tasks/20260719-235305/SPIKE.md (adjudicated 2026-07-20).
- S2 (20260720-000616) builds on this: always-split fps pass + scene
  looping + reload lines. 233732 (partial-emit net) comes after S2.
- The 210443 exclusive-ownership lesson is SUPERSEDED by
  no-unilateral-exit - record the supersession in LESSONS if a ledger
  entry exists for it.
