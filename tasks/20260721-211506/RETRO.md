# Retro: Shakedown pacing pass (20260721-211506)

## What went well

- Diagnostic-first paid off. Reading the script before touching it showed the
  rush was STRUCTURAL, not a tuning problem: every beat was position-gated and
  posted the next objective the instant the gesture landed, and shakedown had
  ZERO scenario-clock pacing (clock gates were a Lifeline/Final Tally tool).
  That reframed the task as "add a pacing layer" rather than "retune numbers".
- Reused the established idiom instead of inventing. The opening and breathers
  ride the same `scenario_elapsed` gate + one-shot-flag pattern lifeline's
  `paced_line` already uses; the only new primitive was a relative gate
  (`beat_gate` stamp + `scenario_elapsed > beat_gate + delay`), built from the
  existing expression-node helpers. Two sequencer variables (`open_step`,
  `breather_last`) kept the flag count down instead of one flag per line.
- Lazy beacon 1 closed the obvious soft-lock before review could find it: with
  the first objective deferred ~40s, spawning beacon 1 at the hand-off (not
  OnStart) means a blind burn cannot skip a beacon that is not there yet.
- The highest-fidelity check was cheap: `menu_newgame` probe runs the REAL
  boot flow through the REAL loader, so it exercised the clock-gated opening
  handlers in production (0 panics, 0 invariant violations) - stronger evidence
  than the minimal scripted_app unit harness, which never advances the clock.

## What went wrong / was tricky

- The test harness does NOT advance `scenario_elapsed` (shakedown never needed
  it before), so the walk test would have silently stalled on the deferred
  objective. Caught it by tracing WHY OBJ_B1 would not post, then adding
  `set_clock`/`finish_opening`. Lesson: when a change adds a clock dependency,
  the existing event-driven rigs need an explicit clock pump - firing events
  directly bypasses time-gated handlers.
- Objective-text simplification risked dropping a teaching cue. Kept one key
  hint per objective ([Alt]/[CTRL]/[G]/...) rather than trusting the hint
  cluster to surface every key, and moved only the FLAVOR to comms - the safe
  reading of "simpler", pending the owner's replay.

## Lessons / what to do differently

- A time-gated content change needs a clock-pumping test helper from the start;
  budget it as part of the change, not an afterthought when a walk test hangs.
- Prefer sequencer counters over per-item one-shot flags for ordered content
  (5 lines -> 1 `open_step`), and a single "last done" counter over N flags for
  a monotonic series (breathers -> 1 `breather_last`) - fewer vars to init and
  keep in phase.

## Follow-ups

- None blocking. Manual owner replay batched at Finish (rush gone, opening
  reads well). Voice/callsign names across the base cast remain
  owner-placeholder (cast.rs header), tracked with the arc spike.
