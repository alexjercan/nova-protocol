# Retro: Center of mass after section destruction (camera anchor fix)

- TASK: 20260709-140620
- BRANCH: fix/com-section-destroy (squash-merged as 0c188cd)
- REVIEW ROUNDS: 2 (round 1 APPROVE with 4 MINOR + 5 NIT + 2 doc corrections,
  all addressed; round 2 APPROVE)

What shipped is in the task's Resolution and
`docs/retros/20260709-com-section-destroy.md`. The headline: two hours of physics
suspicion ended in a one-expression camera fix - the physics was right, the
viewpoint lied.

## What went well

- **Three-level elimination cornered the bug without touching physics.**
  avian harness test, real-pipeline test, then a scripted real-app range -
  each PASS shrank where the lie could live until only the camera was left.
  The instinct after the user report was to patch mass handling; the
  discipline of "prove each layer before blaming it" prevented a fix to
  something that was not broken.
- **The user's observable beat the theory.** The cycle started from a
  feel-model hypothesis (PD inertia normalization) and was one question away
  from a design fork - then the user's clarification, "it spins around a
  non-existing point", collapsed the space to a positional bug. Asking what
  they SEE, in their words, was worth more than all the arithmetic. Second
  retro in a row where a cheap user interaction redirected the work; the
  ask-first lesson keeps paying.
- **Last retro's bookkeeping lesson held.** REVIEW.md was written findings
  first, responses only after each fix was verified (the previous cycle wrote
  round-2 text before the fixes existed and got burned). Order: findings ->
  fixes -> re-run -> responses. No false claims survived to disk.
- **The reproduce example doubled as the regression net.** `11_com_range`
  found nothing wrong (physics passed), but the same scripted run now asserts
  the camera anchor, so the actual fix got a wiring-level test for free.

## What went wrong

- **I wrote a plausible-but-false rationale into a comment.** The
  `Option<&ComputedCenterOfMass>` fallback was justified as "editor preview",
  but the preview never matches that system, and every real root's RigidBody
  requires the component. Root cause: I reused a nearby doc phrase
  ("preview_section carries no physics") as a justification without checking
  that the preview reaches this query. The reviewer caught it in three
  places. A comment that explains WHY must be verified like code, not
  pattern-matched from neighboring prose.
- **The docs overstated a perception claim.** "Both flip faster than
  perception" was written from one ship's numbers and did not survive the
  reviewer recomputing with the example's own five-section ship (0.83s vs
  0.40s flips - clearly perceivable). Numbers in docs need their ship and
  axis named, or they are vibes with decimals.
- **The first smoke script could pass vacuously.** Timeline keyed on total
  elapsed inside a fixed 6s autopilot window: a slow load would have skipped
  every assertion and still exited 0. Same silent-skip family as the optional
  camera `if let`. Harness assertions need a "did I actually run" backstop -
  this is the second cycle where an example-level check needed hardening
  after review (06's observer last cycle).

## What to improve next time

- For "physics feels wrong" reports: instrument reality (gizmos + logged
  live values in a range example) BEFORE forming a fix theory; and get the
  user's raw observable early - "what do you see" - not their (or my)
  diagnosis.
- Verify every WHY-comment against the code path it claims, especially when
  borrowed from nearby docs.
- Scripted example assertions: always end with an asserted-at-exit guard and
  make every lookup mandatory (expect, not if-let) so refactors fail loud.
- Perception/tuning numbers in docs: name the entity, axis, and config they
  were computed from.

## Action items

- [x] tatr 20260709-144906: overkill HealthApplyDamage propagation can kill
  the whole ship (filed on master during the cycle).
- [x] tatr 20260709-150711: AI aim + turret lock-on still anchor at the root
  origin (filed from review R1.2, landed with the merge).
- [ ] The deferred feel-model fork (fly-by-wire vs hardware torque) goes to
  the user with the cycle report; if pursued it lands in the flight-feel
  retune 20260709-095043 (+ a bevy-common-systems task).
