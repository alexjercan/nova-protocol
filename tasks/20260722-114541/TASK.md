# Bug: OnStart objective gates read undefined scenario_elapsed - opening objectives never post + error spam

- STATUS: OPEN
- PRIORITY: 88
- TAGS: v0.8.0, bug, scenario, pacing

## Story

Regression from the pacing pass (task 20260722-092421), caught by the lifeline
probe while verifying the convoy-loiter task (20260722-092432). The pacing pass
added `mark_clock(gate, delay)` calls at OnStart to stamp the deadline for each
opening objective (broadside contact, broadside_gunship objectives, lifeline
screen, final_tally survey). `mark_clock` evaluates `gate = scenario_elapsed +
delay`, but at OnStart the engine clock `scenario_elapsed` is UNDEFINED (the
first `tick_scenario_clock` has not run yet). The content expression evaluator
errors on an undefined variable (unlike the engine's own `scenario_elapsed()`
reader, which defaults None -> 0), so:

- the OnStart set fails (`VariableSetActionConfig: failed to evaluate expression
  for key 'screen_gate': UndefinedVariable("scenario_elapsed")`) - the gate
  never stamps, so the opening objective NEVER posts. In lifeline the "Keep the
  convoy alive" objective simply never appears; the later `complete()` warns
  "no active objective ... to complete".
- every `gated_once` filter reading an unstamped gate errors each frame
  (`VariableFilterConfig: failed to evaluate condition:
  UndefinedVariable("screen_gate"/"survey_gate"/"break_gate"/"picket_gate")`) -
  174 offending log lines in one lifeline->final_tally probe run.

The task-1 review's claim that "tick_scenario_clock seeds the clock before the
fired OnStart drains" was WRONG (the reviewer's synthetic test differed from the
real load path), and task 1 skipped probe as "data-only" - which is exactly
what would have caught this. Lesson: probe scenario CONTENT changes too.

## Steps

- [ ] Verify-first: a test (or probe) that a mainline scenario's opening
      objective actually POSTS in a real run - fails today for lifeline
      (screen), broadside (contact), broadside_gunship, final_tally (survey).
- [ ] Fix the gate stamping. Two sub-issues:
      (a) an OPENING gate must not read scenario_elapsed at OnStart - set it to
          an ABSOLUTE deadline (`set(gate, num(BEAT_GAP))`), correct because the
          opening is at t~=0;
      (b) every gate variable must be INITIALIZED at OnStart (`set(gate,
          num(0.0))`) so a `gated_once` filter reading it before its transition
          stamps it evaluates cleanly (0) instead of erroring on undefined -
          the shakedown convention every var already follows.
- [ ] Apply to broadside, broadside_gunship, lifeline, final_tally. Regen
      content; lint clean.
- [ ] Consider a shared pacing helper (an `open_gate`/absolute-deadline stamp)
      so the OnStart-vs-transition distinction is encoded, not re-remembered.
- [ ] PROBE broadside, lifeline (which chains final_tally) end to end and
      confirm 0 offending log lines AND the opening objectives are live during
      the run (add an objective-present assertion to the walks if practical).

## Definition of Done

- Every mainline opening objective posts in a real run, and no
  UndefinedVariable errors appear in the scenario logs
  (probe: `nova_probe run lifeline` (+ broadside) log_clean PASS;
  test: a walk asserts the opening objective is live).
- Content regenerated, lint clean.

## Notes

- Root: nova_scenario content evaluator errors on undefined vars, while the
  engine's `scenario_elapsed()` (loader.rs:391) defaults None -> 0. Reconciling
  the evaluator (undefined numeric read -> 0) is a broader, riskier alternative
  considered and deferred - the targeted content fix (absolute opening gates +
  gate inits) is safe and matches the existing init-your-vars convention.
- Shared helper `mark_clock`/`clock_past`/`gated_once` in
  crates/nova_assets/src/scenario/pacing.rs.
