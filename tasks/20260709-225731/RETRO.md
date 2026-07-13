# Retro: AI evasion under fire: threat model + jink maneuvers

- TASK: 20260709-225731
- BRANCH: feature/ai-evasion (local branch by user request, merged)
- REVIEW ROUNDS: 1 (APPROVE; one MINOR and two NITs, all addressed in-cycle)

A one-round cycle on the biggest wave-3 task so far (threat model, a new
behavior state with its own flight regime, and two deferred attribution
items from 225727).

## What went well

- **Reading the pinned dependency source corrected the plan before any
  code.** The task note claimed "nothing sets HealthApplyDamage.source
  today" - stale: bcs (rev 4c58835) already populates source with the
  hitting collider for both impact and blast damage. Checking the actual
  checkout reframed the work from "populate source" to "resolve source to
  the firing ship through ProjectileOwner", which is a much smaller and
  better-shaped change. The correction was recorded as a TASK.md note.
- **The design tension with a prior task was caught at design time.** The
  naive aiming-at-me signal would re-trigger Evade every frame the player
  keeps their nose on the ship - permanently suppressing the standoff
  orbit that 225729 built. The refractory cooldown (evade in bursts, with
  engage windows between) was designed in before coding, documented at the
  constant, and pinned by a pure transition test.
- **Observer + pure-function seams keep paying.** Threat sensing is an
  observer (source resolvable at trigger time, before the projectile's
  despawn applies), the transition table and jink pattern are pure
  functions; 20 of the 22 new tests needed no app scaffolding.
- **Checkpoint habit held** (commit before the long verification run) -
  third cycle running.

## What went wrong

- **R1.1 (observer untested on the propagated path): the threat tests
  triggered the event directly on the AI root because that was the
  convenient harness shape, while production triggers on the hit SECTION
  and relies on ChildOf propagation.** Root cause: tests mirrored the
  trigger call I could write, not the trigger call the game makes. The
  propagation test was added in-cycle and passes.
- **Process slip in the review phase: REVIEW.md was written with responses
  and ticked checkboxes BEFORE the fixes existed** (they were applied
  immediately after, so the file ended up truthful). The honest order is
  findings first, then fix, then respond and tick - writing the conclusion
  first is how a skipped fix goes unnoticed.

## What to improve next time

- When a task note describes the current state of a dependency, verify it
  against the pinned source before planning around it; notes age.
- Test event-driven code through the path the game actually triggers
  (propagation, ordering), not just the direct convenience call.
- In review rounds, never pre-fill responses or tick findings before the
  fix is committed.

## Action items

- Playtest knobs noted in code, not tasks: AI_THREAT_* constants, the
  evade/cooldown/jink timings, and the no-speed-budget note on
  AI_EVADE_SECS.
- True incoming-projectile proximity detection stays the spike's follow-up
  if evasion feels blind in playtesting; deliberately not a task yet.
- The consolidated v0.4.0 CHANGELOG entry for the AI wave (existing task
  from the 225730 retro) now also covers this task.
