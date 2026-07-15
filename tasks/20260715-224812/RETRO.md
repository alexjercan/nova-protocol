# Retro: Make Demo Mod Arena a playable target-destruction challenge

- TASK: 20260715-224812
- BRANCH: feat/arena-combat (landed as master 7636d43a)
- REVIEW ROUNDS: 1 (APPROVE; two discretionary findings applied in-cycle)

## What went well

- Carried the previous cycle's lesson forward on purpose: the OnStart
  structural test (`onstart_spawns_the_player_targets_and_seeds_the_counter`)
  shipped in the SAME commit as the behavior test, so the "playable wiring is
  untested" gap that cost the gauntlet a review round (R1.1 there) never
  materialised here. See `rig-supplies-precondition-hides-regression`.
- Reused the RIGHT proof boundary: the physical destroy->OnDestroyed bridge is
  already owned by nova_scenario's asteroid test, so this task tested the arena
  DATA's consumption of that event (counter + one-shot win) by firing the exact
  event info the bridge emits, instead of re-proving the bridge or hand-waving.
- Independent review verification found a real robustness gap (the `==3` win
  gate) before it could ship - by re-deriving the AND-filter semantics from the
  framework source and reasoning about the repo's double-fire history.

## What went wrong

- Nothing serious. Two small friction points: `commands.fire` needs the
  `CommandsGameEventExt` trait in scope (first build failed E0599; the compiler
  named the fix), and I first wrote the condition variant as `Greater` when the
  enum is `GreaterThan` (caught by verifying against variables.rs before
  building). Both were cheap because I checked types against source rather than
  guessing.

## What to improve next time

- For any milestone gate on a COUNTER that is incremented by an event which can
  fire more than once (collisions, per-collider pairs, multi-hit), gate on
  `> N-1` / `>= N`, never `== N` - an overshoot past the exact value skips the
  gate forever. This is the counting-counterpart of the repo's
  `collisionstart-is-per-collider-pair` lesson.

## Action items

- [x] Bumped `rig-supplies-precondition-hides-regression` (now x2, applied
      preventively) and added `count-gate-use-gt-not-eq` to docs/LESSONS.md.
- No follow-up code tasks.
