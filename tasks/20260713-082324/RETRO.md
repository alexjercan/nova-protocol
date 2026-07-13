# Retro: Look-ray + camera-mode infrastructure

- TASK: 20260713-082324
- BRANCH: feature/look-ray-infrastructure (landed a123f36)
- REVIEW ROUNDS: 1 (APPROVE; one MINOR carried forward as a contract note)

Process notes only; what/why/evidence in TASK.md.

## What went well

- Planning against an adversarially-reviewed corpus paid off exactly as hoped:
  the dead 231141 body was lifted nearly verbatim and every file:line anchor
  was still accurate - near-zero re-derivation. The pattern "supersede the
  design, INHERIT the infrastructure analysis" is worth repeating whenever a
  design pivots but the plumbing problems stay.
- The e2e-faithful test harness (InputPlugin + EnhancedInput + production
  action shape, real key/button presses) hit one snag - the context registry
  finalizes in App::finish - and the fix was ALREADY documented in an existing
  test's comment (the wheel e2e). Reading how the last person solved the same
  rig problem beat debugging from scratch.
- Decoy-rig discipline: putting the dormant turret rig 90 degrees off makes
  "read the wrong rig" fail loudly in every test that touches aim, not just
  the dedicated regression - the delivery-guard None assertion doubles as a
  decoy detector.
- The A/B sabotage (seed source reverted to the normal rig) was cheap (~3 min)
  and turned "the test should fail pre-fix" into "the test DOES fail pre-fix".

## What went wrong

- Minor: I initially spawned the enhanced-input action rig before
  App::finish/cleanup and got an opaque ECS param-validation panic in an
  observer. Cost one debug cycle; the answer was one grep away in the
  existing e2e test. Root cause: assembled a new-style test app from memory
  instead of cloning the closest working example first.
- The first test-run attempt used two filters on one `cargo test` invocation
  (ledger lesson `one-cargo-test-filter` at x3) and silently matched nothing.
  The ledger note fired only AFTER the empty output - the lesson is known but
  the reflex still is not.

## What to improve next time

- When building a test that needs a subsystem's runtime plumbing (input
  contexts, asset servers, schedules), start by copying the nearest existing
  test that already boots that subsystem, then mutate - do not assemble from
  the plugin list.

## Action items

- [x] Ledger: bump `one-cargo-test-filter` (x4).
- Contract carried to 082330/082337 (recorded in their plans + review R1.1):
  radar/safety observers MUST be pause-gated, because WeaponsRaised derivation
  deliberately is not.
