# Review: Objective posts before its intro dialogue finishes - match the beat gap to the comms dwell

- TASK: 20260722-142341
- BRANCH: fix/objective-gap-matches-comms-dwell

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context (round-1 findings), in-session (verification + fixes)

Round-1 findings were produced by an out-of-context reviewer with no sight of
the implementing session. In-session verification re-derived load-bearing
claims before adopting: confirmed `BEAT_GAP = COMMS_DWELL_SECS +
COMMS_FADE_OUT_SECS = 8.4s > 8.0s` dwell (referenced directly from
`nova_gameplay::prelude`, so gap and dwell cannot drift); confirmed the
exhaustive structural pin `no_mainline_handler_posts_an_objective_alongside_a_conversation`
(crates/nova_assets/src/scenario.rs:499) iterates every mainline scenario and
fails if any single handler emits both a StoryMessage and an Objective - so the
"objective gated behind dialogue" guarantee is enforced across all four
scenarios, and this test would have caught any shakedown transition left
posting both. Checks re-run in-session: `cargo fmt --check` clean, `cargo check`
clean, `cargo test -p nova_assets --lib scenario::` 21/21, `cargo test
-p nova_gameplay --lib hud::comms_panel` 5/5. probe: lifeline / broadside /
menu_newgame all OK (0 invariant violations, 0 errors).

No BLOCKER or MAJOR findings. The three non-blocking findings below were
addressed in-session (MINOR comment drift is real drift worth fixing while the
code is fresh; the NIT was resolved by aligning the spec with the deferral).

- [x] R1.1 (MINOR) crates/nova_assets/src/scenario/shakedown.rs:174-176 - stale
  comment described the old `breather`/`breather_last` mechanism.
  - Response: Fixed. Rewrote the var-block comment to describe `setup_last` and
    the `beat_setup` handler (which posts the objective/beacon/markers, not just
    a line).
- [x] R1.2 (MINOR) crates/nova_assets/src/scenario/shakedown.rs:669 and the
  config-pin test prose - more "breather" drift after the rename.
  - Response: Fixed. Reworded line 669 ("the next beat's setup is timed from
    here") and the `the_opening_converses_...` test doc/asserts to "beat gate" /
    "beat transition lines". The `gate_stamps >= 9` assertion is unchanged and
    still valid. No remaining `breather` mentions in the file.
- [x] R1.3 (NIT) crates/nova_assets/src/scenario/pacing.rs:41 - TASK.md step 2
  asked for a `line_gap(dwell)` helper that was not added (would be dead code
  today).
  - Response: Resolved by aligning the spec: TASK.md step 2 now records the
    helper as DEFERRED (YAGNI - no scenario line sets a per-line dwell override,
    so it would be dead code), with the BEAT_GAP doc-comment noting where it
    would slot in. Spec and code now agree.
