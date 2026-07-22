# Review: OnStart objective gates read undefined scenario_elapsed (20260722-114541)

- VERDICT: APPROVE

Out-of-context review of branch `fix/onstart-gate-undefined` against `master`.
The fix is correct, complete, and well-pinned. All five checks in the review
brief hold. No blocking findings.

## Summary

The fix cleanly separates the two gate-stamp cases the pacing pass conflated:
an OnStart gate now takes an ABSOLUTE deadline via the new `pacing::open_gate`
(`set(gate, num(delay))`, no clock read), while `mark_clock` (which reads the
undefined-at-OnStart `scenario_elapsed`) is re-documented and used only in
mid-scenario handlers. Every transition gate a `gated_once` filter can reach
before its stamp is now seeded to 0 at OnStart. Verified end to end.

## Verification performed

- `cargo test -p nova_assets --lib scenario::` -> 21 passed, 0 failed
  (includes the new `no_onstart_handler_reads_the_scenario_clock`).
- `nova_probe run lifeline` (chains final_tally) -> OK:
  `log_clean` PASS (0 panic/ERROR lines, was 174), `run_completed` PASS (frame
  271), `reached_playing` PASS, `invariants_held` PASS (0 violations / 271
  frames). `fps_within_baseline` SKIPPED (no baseline; not gameplay-relevant).

## Findings by focus area

### 1. Absolute-deadline correctness - CORRECT

`open_gate(gate, BEAT_GAP)` sets `gate = BEAT_GAP`; the paired `gated_once`
fires when `scenario_elapsed > BEAT_GAP` (`pacing.rs:70-73`, `pacing.rs:95-96`).
OnStart fires exactly once at load (`nova_scenario/src/loader.rs:892`), so the
opening beat is at `t == 0` with no drift. The would-have-been relative stamp
`mark_clock(gate, BEAT_GAP)` = `scenario_elapsed + BEAT_GAP` also evaluates to
`~= BEAT_GAP` at `t == 0`, so absolute == relative here. No off-by-delta. The
absolute form is in fact strictly more robust (it cannot drift if the handler
were ever delayed). Intended timing (objective posts ~BEAT_GAP into the run) is
preserved.

### 2. Completeness - COMPLETE

Grepped every `mark_clock`/`open_gate` use. Every remaining `mark_clock` is in
a MID-SCENARIO handler:

- `broadside.rs:383` DEFEND_GATE - OnEnter
- `final_tally.rs:453` PICKET_GATE - OnTravelLock
- `final_tally.rs:519` CAST_AT - OnUpdate
- `final_tally.rs:551` BREAK_GATE - OnUpdate
- `final_tally.rs:573` EPILOGUE_AT - OnDestroyed
- `shakedown.rs:938` SCAV_GATE - OnDestroyed
- `shakedown.rs:323` `stamp_gate()` helper -> only called at beat transitions,
  never OnStart (guarded structurally by the invariant test below).

Every `gated_once` deadline_key is defined at OnStart:

- SURVEY_GATE / CONTACT_GATE / GUN_OBJ_GATE / SCREEN_GATE -> seeded by
  `open_gate` (the opening stamp itself).
- PICKET_GATE, BREAK_GATE -> newly seeded `set(_, num(0.0))`
  (`final_tally.rs:354-355`).
- DEFEND_GATE -> newly seeded (`broadside.rs:325`).
- SCAV_GATE -> newly seeded (`shakedown.rs:613`).

The two direct `clock_past` reads that are NOT behind a `gated_once` guard were
checked for undefined-read reachability:

- `clock_past(VAR_CAST_AT)` (`final_tally.rs:537`) is the LAST filter in an
  event whose prior filter requires `VAR_TAUNT_SAID == 1`; TAUNT_SAID is set in
  the SAME block that stamps CAST_AT (`final_tally.rs:518-519`). Filters
  short-circuit via `filters.iter().all(...)` (bevy_common_systems
  `modding/events.rs:149`), so `clock_past(VAR_CAST_AT)` is never evaluated
  before CAST_AT is stamped. CAST_AT and EPILOGUE_AT are additionally already
  seeded to 0 at OnStart (`final_tally.rs:345-346`, pre-existing), so even a
  reordering would read a defined 0, not undefined. Safe.

Conclusion: no OnStart handler reads the clock and no reachable filter reads an
undefined gate.

### 3. Invariant test - CATCHES THE BUG CLASS

`no_onstart_handler_reads_the_scenario_clock` (`scenario.rs:611-628`) scans
every OnStart `VariableSet` expression across the five mainline configs and
fails if the rendered expression contains `scenario_elapsed`. This is exactly
the bug shape (an OnStart set built from the clock). Verified it runs and passes.

False-negative surface (documented, not blocking):
- It only inspects `VariableSet` actions. An OnStart clock read via a FILTER
  (e.g. `clock_past` on an OnStart handler) or via a non-VariableSet action
  carrying an expression would slip past. In today's content OnStart handlers
  carry no filters and no other expression-bearing action, and the sibling
  `no_mainline_scenario_posts_an_objective_at_onstart` constrains OnStart
  shape, so the gap is not currently reachable. A stricter form would walk any
  OnStart expression (action or filter) for the clock var; noted as a possible
  hardening, not required for this fix.
- Scope is the five mainline configs (`mainline_scenarios()`), matching the
  other structural pins in this module. That is the right scope: these are the
  hand-authored scenarios where the footgun lives. A brand-new scenario would
  need to be added to `mainline_scenarios()` to be covered - same limitation
  all pins here share, and acceptable by convention.

### 4. Frame-0 safety of the seeds - NO BEHAVIOR CHANGE

Seeding a gate to 0 cannot fire its objective early: `gated_once` guards with
`gt_num(deadline_key, 0.0)` (`pacing.rs:95`), so `gate == 0` reads "not yet
stamped" and the handler is inert until a transition stamps a positive value.
This is the exact guard the module doc calls out (`pacing.rs:78-82`). For
shakedown's SCAV_GATE the only change is that its pre-stamp frames now read a
defined 0 instead of erroring on undefined - the objective still posts only at
beat 12 (`shakedown.rs:949` filter `eq_num(VAR_BEAT, 12.0)` + the gt-guard). No
opening objective fires on frame 0; the pre-existing
`opening_objectives_are_deferred_past_frame_one` pin still passes.

### 5. Tests + probe - PASS

See "Verification performed" above. 21/21 lib tests, lifeline probe log_clean
PASS with the `screen_convoy` objective-present assertion
(`examples/gameplay/lifeline.rs:250-257`) holding - fail-first, since the
objective was absent before the fix. The asserted id `screen_convoy` matches
`OBJ_SCREEN` (`lifeline.rs:53`).

## Content regeneration

`broadside.content.ron`, `lifeline.content.ron`, `final_tally.content.ron`,
`broadside_gunship.content.ron`, `shakedown_run.content.ron` all regenerated
consistently with the source: the opening gate expressions changed from
`Add(Name("scenario_elapsed"), ...)` to `Term(Factor(Literal(Number(4.0))))`
and the new `_gate` seeds appear at OnStart. Consistent, no drift.

## Nitpicks (non-blocking)

- The invariant test could also guard OnStart FILTERS and non-VariableSet
  expression actions to fully close the class (see finding 3). Optional
  hardening, not needed for this fix.
- The deferred deeper root cause (evaluator errors on an undefined numeric
  read while the engine's `scenario_elapsed()` defaults None -> 0) is correctly
  documented in TASK.md as a separate, riskier follow-up. Agreed with deferring.
