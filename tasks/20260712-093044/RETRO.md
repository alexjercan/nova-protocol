# Retro: Nav beacon and salvage crate scenario objects

- TASK: 20260712-093044
- BRANCH: shakedown-run (family branch; landed as 2cac4b3)
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- The full-pipeline pickup test (real physics -> OnEnter -> filtered
  handler -> scenario variable, with a delivery guard) did double duty:
  it proved the new object AND empirically settled the pre-existing
  avian body1/body2 ordering worry in area.rs without a speculative fix.
  Writing the test at the deepest real layer answered an architecture
  question for free.
- The despawn action's scope restriction (ScenarioScopedMarker) was
  caught at design time by asking "what else carries EntityId" before
  writing the query; the reviewer's independent re-derivation confirmed
  it load-bearing (ship sections would have been despawned globally).
- One review round. The fresh-context review pass verified rather than
  re-litigated: three findings, none blocking, one applied.

## What went wrong

- The plan asserted "beacons are aim-lockable (candidates are any
  RigidBody)" from reading the candidates QUERY signature; the collection
  LAMBDA right below it gates out non-Dynamic non-well bodies, so Static
  beacons were unlockable and the GOTO beat would have silently broken.
  Root cause: the plan-time verification stopped at the type signature
  instead of walking the consumer to its end. Cost was small only because
  the gate was hit during implementation, not playtest.

## What to improve next time

- When a plan step claims "X will be accepted/processed by system Y",
  the verification must follow the data INTO Y's filter/gate logic, not
  stop at the query or function signature. Signature-level reads verify
  shape, not admission.

## Action items

- [x] Bumped `verify-first-plan-steps` in docs/LESSONS.md with the
      consumer-gate variant.
- [x] Spike fix record appended (tasks/20260712-092926/SPIKE.md).
