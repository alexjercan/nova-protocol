# Retro: Shakedown Run - the five-beat New Game starter

- TASK: 20260711-180506
- BRANCH: shakedown-run (family branch; landed as 2449120)
- REVIEW ROUNDS: 2 (REQUEST_CHANGES -> APPROVE)

## What went well

- Two "advertised but unwired" platform gaps were caught by asking "who
  actually fires/admits this" before building on the config surface:
  task 1's targeting gate, and this task's EventConfig::OnUpdate (the
  variant and docs existed; nothing ever fired the event - the milestone
  handlers would have silently never run). Same probe, two catches, one
  day.
- The script avoided handler-order dependence by design (OnUpdate
  value-gated milestones instead of piggybacking the pickup event), and
  the reviewer's independent walk confirmed the ordering-safety analysis
  rather than finding holes in it.
- The five-beat walk test at the real-event-pipeline layer gave the
  review something to verify against; both endings and the re-entry
  non-refire case were cheap to add once the rig existed.

## What went wrong

- R1.2 (MAJOR, real softlock): the beat-4 geometry was authored against
  a factor band (4.0-4.55) taken from an OBSERVED comment in
  menu_ambience, not a measured bound. The review demanded a sweep; 256
  seeds measured [3.70, 5.64] - real seeds exceeded the assumed max, and
  on those the ORBIT ring parked outside the 160u gate: beat 4 would
  never complete, with every test green. Root cause: applying the
  authored-vs-derived lesson HALFWAY - I used the derived formulas but
  fed them a folklore input range. A number that only exists in a
  comment is not a bound; only a test that measures it is.
- R1.1 (MAJOR): the OnUpdate pulse changed the load profile of an
  existing system (state_to_world now runs every frame) and I did not
  ask what its unconditional writes would do downstream - the objectives
  panel rebuilt its text entities every frame. Root cause: the pulse was
  added as "the missing producer" without auditing the consumers of the
  queue-not-empty condition it flipped from rare to constant.

## What to improve next time

- When content math consumes a numeric range, grep for where the range
  comes from: formula constants (fine, cite them) vs observed/comment
  numbers (not fine - write the measuring test first, export the bound
  as a const, make the content test cite the const).
- A change that makes a rare condition constant (event queue non-empty,
  a run_if flipping true every frame) needs a consumer audit of that
  condition before shipping - same family as the state-gate audit
  lesson from the menu cycle.

## Action items

- [x] LESSONS.md: new `advertised-but-unwired` (x2), variants appended
      to `authored-vs-derived-values` and
      `audit-state-gates-on-new-entry-path`; `out-of-context-review-pass`
      bumped (two MAJORs, one mutation-tested verification).
- [x] Spike fix record updated (tasks/20260712-092926/SPIKE.md).
- [ ] Human visual playtest of the run (honestly un-ticked in TASK.md:
      beacon readability, pickup radius feel, pirate difficulty, orbit
      gate moment). Conveyance visuals task 20260712-093831 stays queued
      and should follow that playtest.
