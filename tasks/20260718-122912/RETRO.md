# Retro: RCS player input (SHIFT + mouse/scroll fine-adjust)

- TASK: 20260718-122912
- BRANCH: feat/rcs-input (landed as master 27baaa2c)
- REVIEW ROUNDS: 1 (REQUEST_CHANGES -> APPROVE after fixes)

Process only; what/why live in TASK.md + NOTES.md.

## What went well

- Caught a real bug DURING implementation, before review: freezing the camera by
  merely skipping `on_rotation_input` would have left a stale `PointRotationInput`
  rate, and the bcs `point_rotation_update_system` integrates that rate every
  frame - so the view would drift. Found by tracing the consumer of the value I
  was freezing, not by testing after. Fixed by zeroing (commit 1393b365).
- The honest self-review earned its keep: as reviewer I flagged two paths (the
  drift-freeze fix and the scroll-Y branch) that would NOT fail if reverted, and
  REQUEST_CHANGES forced regression-guard tests. Both new tests fail on revert.
  The temptation in a same-session review is to wave through "it compiles and the
  happy path passes" - resisting that is the whole point.
- Reused the existing `Autopilot`-presence gating pattern almost verbatim for
  `RcsActive` (helm freeze via `Without<RcsActive>`), and the CTRL-layer
  "modifier decides" precedent for the scroll repurpose - small, idiomatic diff.

## What went wrong

- Two untested paths shipped in the first work pass (drift-freeze, scroll-Y) -
  root cause: I wrote the mouse-aim test first, it passed, and I under-weighted
  that the scroll and drift paths were DIFFERENT code with no coverage. The
  "would it fail if reverted?" check should run per-path, not per-feature.
- Burned time on Bevy 0.19 test-input API discovery: `write_event` (wrong) ->
  `write_message`, then `MouseWheel` missing its `phase` field. Two compile
  round-trips (~5min) that reading the message struct up front would have saved.
- Hit `piped-cargo-masks-exit-code` AGAIN (x5 now): a `cargo test | grep` in a
  background run returned empty, hiding pass/fail, and I re-ran with `tail`. This
  lesson is already pending promotion - I keep doing the thing it warns against.

## What to improve next time

- After any FREEZE/skip of a value another system consumes, find that consumer
  and confirm it does not keep acting on a stale value (the drift class).
- When a feature has multiple input paths (mouse vs scroll vs modifier), write a
  test per path and ask "fails if reverted?" for EACH before calling it done.
- Read a dependency's message/event struct fields before constructing it in a
  test; and STOP piping cargo through grep in these runs - print the tail.

## Action items

- [x] Ledger: bumped `modal-input-observer-dispatch` (x2), `bei-app-finish-in-tests`
  (x2), `assert-each-gesture-step` (x2), `half-ticked-compound-steps` (x3 ->
  Pending); added `bevy-input-is-messages-in-tests` and
  `changed-shared-observer-run-the-module-suites`.
- [x] Consolidated a duplicate I filed last cycle
  (`per-crate-test-needs-gated-features`) back into the mature
  `crate-solo-tests-miss-unified-features` (now x6).
- [ ] `piped-cargo-masks-exit-code` (x5) and `crate-solo-tests-miss-unified-features`
  (x6) are both well past the promotion threshold and already parked under
  Pending promotions - recommend the user fold them into AGENTS.md / the work
  skill so they stop recurring.
- No follow-up code tasks: the deferred RCS work (HUD 20260718-122923, autopilot
  20260718-122932) was already seeded by the spike; the mouse-Y sign is a
  documented playtest call (review R1.4).
