# Retro: Objective posts after its intro dialogue finishes (comms-dwell gap)

- TASK: 20260722-142341
- BRANCH: fix/objective-gap-matches-comms-dwell (merged to master, ff)
- REVIEW ROUNDS: 1 (APPROVE)

See TASK.md / NOTES.md for what changed and why. Process observations only.

## What went well

- Asked the shakedown ordering fork via AskUserQuestion instead of guessing:
  the tutorial's reverse ordering (objective-first, trailing line) made
  "wait for the dialogue" a genuine feel decision, not a mechanical one.
- Read the actual beacon handlers AND their config tests before editing.
  That revealed shakedown's reverse ordering (the exploration summary had
  glossed it) and let me interleave the new `beat_setup` handlers in beat
  order so the order-asserting marker-hand-off and emphasis-pairing tests
  stayed green unchanged - a layout I would have gotten wrong if I'd appended
  the setups at the end.
- A pre-existing exhaustive pin, `no_mainline_handler_posts_an_objective_alongside_a_conversation`,
  made the whole-mainline guarantee mechanical and would have caught any
  transition left posting both a line and an objective. Leaned on it in review
  instead of eyeballing four scenarios.
- probe on lifeline / broadside / menu_newgame gave real end-to-end
  confidence where no autopilot example runs the mainline shakedown directly.

## What went wrong

- Initial scope was "bump a constant": the task body assumed every gate
  followed a `story()` line, so shakedown would be a free ride. Shakedown
  actually used the REVERSE ordering and needed a full beat-chain restructure
  (~350 lines). Root cause: scoped from the exploration subagent's summary
  ("shakedown DOES NOT USE the pacing module") before reading the shakedown
  handlers myself. Corrected before implementation by reading them, but the
  written task body was briefly wrong.
- Deferring the objective opened two race windows against already-live world
  state that the first design missed: the salvage crates exist from OnStart
  (a pickup during the intro line would count against an unposted objective),
  and the coast-ring exit is edge-triggered with `[Z]` granted from the start
  (a fast break-away could reach beat 10 before the delayed setup spawned the
  derelict). Surfaced by reasoning through fast-player skips, then fixed with
  a `setup_last` guard and a transition-time derelict spawn.

## What to improve next time

- When introducing a delay between "the world is live" and "the objective/
  marker posts," enumerate every consumer that can fire in the gap
  (OnStart-spawned interactables, edge-triggered area events) and guard each
  on the deferral latch - the same discipline as gating a producer's
  consumers, applied to a TIMING gap.
- Treat an exploration agent's "X does not use Y" as a lead to confirm in the
  code before it becomes a scoping fact.

## Action items

- [x] Ledger: added `defer-opens-a-consumer-race`; added this id to the paid
  record of `verify-stale-brief-against-tree`.
- [ ] Follow-up (optional, surfaced to user, not filed): no probe example
  autopilots the mainline shakedown beats - end-to-end confidence rests on
  the `the_five_beats_walk_end_to_end` unit walk plus the menu_newgame boot.
  Worth a small probe-example wiring task if shakedown pacing changes again.
