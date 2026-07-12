# Retro: Nova typed-damage core

- TASK: 20260712-133343
- BRANCH: feature/typed-damage-core (landed as d355299)
- REVIEW ROUNDS: 1 (APPROVE, two NITs)

See TASK.md for what/why and docs/2026-07-12-typed-damage-core.md for the
implementation write-up. This is process only.

## What went well

- **Two spikes before a line of code meant zero design churn.** The architecture
  spike (own-the-trigger) and the taxonomy spike (the four types + the exact
  table) settled every hard decision up front, so `/plan` was mechanical and the
  implementation had no "wait, how should this actually work" moments. The single
  review round with only NITs is the payoff.
- **Read the dependency's real source before coding against it.** Grepping bcs's
  `on_impact` formula + constants, its `blast_damage` bundle, and the gravity
  `apply_linear_acceleration` call meant the neutralization (near-zero mass) and
  the `NovaBlast` mirror were correct on the first compile, and the authored
  damage numbers were derived, not guessed.
- **Proactive consumer sweep caught a silent regression.** Grepping
  `BlastDamageMarker` before finishing found the torpedo blast VFX/SFX observers
  (and two examples, two test assertions) keyed on it. Swapping to `NovaBlast`
  would have killed the detonation visuals with no failing test - nothing covers
  particle spawning. The sweep, not a test, caught it.
- **Out-of-context review re-derived the load-bearing math.** The fresh-context
  agent independently reproduced the ~0 neutralization residual and the 20.25 /
  3.825 authored values, and confirmed the `BlastDamageMarker` sweep was total -
  confidence a same-session read could not give.

## What went wrong

- **First-pass integration tests were not production-faithful and read a false
  zero.** The initial target entities were a single entity with
  `destructible_body` + `Collider` and NO `RigidBody`; bcs's impact/blast
  observers read `body1`/`body2` (the RigidBody entities), which were `None`, so
  no damage landed and the test reported drop 0. Root cause: I built the test
  target from a simplified mental model instead of mirroring how sections are
  actually spawned - a RigidBody root with child colliders that hold the Health.
  Cost one debug iteration; fixed with a `spawn_target` helper matching the real
  hierarchy.
- **Bundle arity overflow.** Adding `ProjectileDamage` as a new top-level element
  pushed the bullet spawn tuple past Bevy's 16-element `impl Bundle` limit. The
  file already nests tuples for exactly this reason; I just didn't nest the new
  element until the first `cargo check` flagged it. Cheap (compile-time), but
  avoidable.

## What to improve next time

- When a headless test's target must be seen by an ENGINE observer (collision,
  damage, integrity), build it with the SAME entity shape production uses - grep
  a real spawn site first - not a flattened stand-in. A body-vs-collider
  hierarchy mismatch makes the observer silently no-op and the test read a false
  pass/zero.
- When adding a component to an existing spawn bundle that is already near the
  16-tuple limit, nest it with an adjacent component from the start.

## Action items

- [x] Retro written; LESSONS.md ledger updated (production-faithful-rigs x5 with
  the observer-hierarchy flavor; sweep-then-delete x5 with the observer-consumer
  variant; out-of-context-review-pass x10).
- No follow-up code tasks: the two family successors (20260712-133349 magazines/
  reload, 20260712-133356 alt-fire) already exist and are unblocked by this land.
