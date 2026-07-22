# Retro: sequence objectives after conversations + breathing room

- Landed: 0ae5c7f9 (squash), 1 review round, out-of-context APPROVE.

## What changed and why

Owner playtest: objectives appeared in the same frame as the comms line that
introduced them, and completing one objective instantly popped the next. Fixed
across the whole base campaign by deferring every objective a beat past its
conversation.

Design decision - one shared mechanism, not three. The codebase already had
TWO clock-gate implementations (shakedown's `stamp_gate`/`past_gate`,
final_tally's `mark_clock`/`clock_past`) plus lifeline's `paced_line` - the
same idea copy-pasted with different arithmetic (delay added at the gate vs
baked into the stamp). Rather than add a fourth, I unified them into a shared
`scenario/pacing.rs` (`mark_clock` / `clock_past` / `gated_once`) and refactored
shakedown + final_tally onto it. The refactor is algebraically
behaviour-preserving (`elapsed > stamp + delay` <-> `stamp = elapsed + delay;
elapsed > stamp`), which the zero RON-regen drift on the untouched beats and the
shakedown walk tests both confirm.

`gated_once` carries a `gate > 0` guard so an unstamped deadline (an unread var
reads 0, and the clock starts at 0) does not look "already passed" and fire the
objective on frame one - the exact bug being removed.

Owner questionnaire mid-task: the Shakedown opening panel stays EMPTY during
the captain conversation (I had assumed keeping the "stand by" holding line;
the owner wanted none). Removed OBJ_OPENING entirely.

## Difficulties / bugs

- The invariant test I wrote as the spec ("no handler posts a StoryMessage AND
  an Objective") FAILED first on two handlers I had not yet reached - shakedown's
  scavenger reveal and final_tally's cast-off, both threat-reveal beats posting
  a warning line + objective together. Fail-first working as intended: the test
  found the cases before I did, and I deferred both objectives the same way.
- Two over-strict versions of a "deferred objectives are clock-gated" test
  false-positived on legitimate non-clock deferrals: shakedown's OBJ_B1 gates on
  the conversation-step counter (itself clock-paced), and the crate-tally
  handlers re-post an objective gated on a gameplay COUNTER, not the clock.
  Settled on scoping the deferral test to the explicit OPENING objective ids per
  scenario, and leaning on the two exhaustive invariants + the behavioural walk
  for the rest.
- A walk test soft-failed because it never set the scenario clock, so
  `scenario_elapsed` was None when `mark_clock` ran (`None + 4` -> unset var ->
  the gated objective never posted). Pure test artifact: production ticks the
  clock every frame before the fired OnStart drains (the reviewer verified this
  empirically). Fixed by seeding a clock baseline in the test before the kill.

## Self-reflection - what to do differently

- The invariant-as-spec / fail-first loop paid off twice (caught the two reveal
  beats, then caught my own over-strict test). Writing the exhaustive structural
  invariant BEFORE editing scenarios was the right order - it turned "did I get
  every case" from a manual grep into a green bar.
- I over-reached twice on the "clock-gated" test before scoping it correctly.
  Lesson: a cross-scenario invariant must budget for MORE THAN ONE valid
  mechanism (clock gate, conversation latch, gameplay counter) - a single-shape
  assertion will false-positive on legitimate variety. Pin the exact thing the
  owner flagged (the opening objective), not a broad structural proxy.
- The None-vs-0 variable subtlety (mark_clock on an unstamped clock) is a real
  gotcha for anyone writing headless scenario tests: the harness does not tick
  the clock, so any handler that READS `scenario_elapsed` needs a `set_clock`
  baseline first. Worth a lessons entry.

## Follow-ups

- None blocking. Shakedown's beacon-to-beacon nav swaps stay instant by design
  (continuous waypoint flight, no colliding conversation); if the owner replays
  and wants those delayed too, that is a small follow-up, not a gap.
- Manual acceptance (owner): replay shakedown/broadside/lifeline - no objective
  during an opening conversation, no completed+new-objective in the same instant.
