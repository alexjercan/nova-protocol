# Retro: OnStart objective gates read undefined scenario_elapsed

- Landed: d320e1dc (squash), 1 review round, out-of-context APPROVE.

## What changed and why

A regression from the pacing pass (task 20260722-092421): opening-objective
gates were stamped with `mark_clock` at OnStart, which reads `scenario_elapsed`.
That engine variable is UNDEFINED at OnStart (the first `tick_scenario_clock`
has not run), and the content expression evaluator ERRORS on an undefined read
(unlike the engine's own `scenario_elapsed()` reader, which defaults None -> 0).
Result: the OnStart gate stamp silently failed, so the opening objectives
(lifeline "keep the convoy alive", broadside contact, final_tally survey,
gunship objectives) never posted; plus 174 UndefinedVariable log lines from
gated_once filters reading unstamped gates.

Fix (targeted, content-level, no evaluator change): a new `pacing::open_gate`
stamps an ABSOLUTE OnStart deadline (`set(gate, num(delay))`) instead of reading
the clock - correct because OnStart is t~=0. Every transition gate is seeded to
0 at OnStart so its filter reads a defined 0 before its stamp. A new invariant
`no_onstart_handler_reads_the_scenario_clock` pins the class; the lifeline probe
asserts the opening objective is live.

## How it was found, and the miss

Found by PROBING the lifeline example while verifying the convoy-loiter task
(20260722-092432) - `log_clean` failed with 174 UndefinedVariable lines, and
the log showed the "keep the convoy alive" objective never posted.

The miss is the important part. Task 1:
- I skipped probe on task 1, reasoning "data-only pacing, no perf surface." That
  was wrong: a data-only content change CAN carry a behavioral bug (gate timing
  that never fires). Probe is a behavioral check, not just a perf one.
- Task 1's out-of-context reviewer explicitly flagged the OnStart clock read as
  "the one real risk", tested it in a synthetic scenario, and concluded it was
  safe because "tick_scenario_clock seeds the clock before the fired OnStart
  drains." That synthetic test differed from the real load path (in the real
  loader, OnStart fires before the first tick). A review that BUILDS a bespoke
  rig to check a risk can get a false GREEN if the rig doesn't match production.
- Task 1's own retro even noted the None-vs-0 subtlety but filed it as "purely a
  test artifact." It was a production bug.

## Self-reflection - what to do differently

- PROBE SCENARIO CONTENT CHANGES. "Data-only" is not a reason to skip probe -
  the highest-fidelity harness the project has (real engine, real clock, real
  handlers) is exactly what catches gate-never-fires and objective-never-posts.
  Task 1 would have been caught pre-land. -> a lessons entry.
- When a reviewer's confidence rests on a BESPOKE rig ("I drove a synthetic
  scenario and it stamped positive"), distrust it more than a check against the
  real artifact. The real load path is the authority; a hand-built rig that
  "proves" a risk safe should reproduce the exact production ordering or be
  treated as inconclusive.
- The deeper footgun is the evaluator's undefined-read semantics diverging from
  the engine's own None -> 0 convention. The content fix is safe, but the class
  will recur for future authors. Deferred as a possible follow-up (reconciling
  the evaluator, or a lint that flags an OnStart scenario_elapsed read) - noted
  in the task DoD.

## Follow-ups

- None filed as blocking. Optional hardening surfaced in review (extend the
  OnStart invariant beyond VariableSet to filters/other actions) and in the Fix
  note (evaluator undefined -> 0, or a lint guard) - left as judgment calls for
  when the footgun next bites, not pre-emptive work.
