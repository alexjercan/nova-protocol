# Retro: AI line-of-sight fire gate

- TASK: 20260717-112622
- BRANCH: work/ai-los-fire-gate (landed 2d006707)
- REVIEW ROUNDS: 1 (APPROVE; 2 MINOR + 1 NIT, all addressed post-approve)

## What went well

- Reading avian's cast_ray_predicate SOURCE during planning (not its doc
  comment, which is wrong about predicate semantics) prevented a real
  semantic bug before any code existed; the reviewer re-derived the same
  conclusion independently.
- The ledger paid for itself three times in one plan: the
  required-component-in-shared-query lesson predicted the rig sweep
  (SpatialQuery param would panic six bare rigs), reuse-known-good-stack
  pointed at the integrity physics harness, and commit-before-sabotage
  shaped the A/B. Zero surprise breakage at test time.
- Same-test delivery guards (re-assert FIRE after despawning the rock) made
  the sabotage A/B sharp: exactly the two blocking tests went red.

## What went wrong

- R1.2 (docs overclaim): the CHANGELOG/wiki said "maneuvers for a clear
  angle" - a reposition feature that was DELIBERATELY scoped out in the
  same task (the motion is the pre-existing orbit). Root cause: the
  player-facing prose was written from the task TITLE's aspiration ("hold
  fire and reposition") rather than from the shipped diff. When a title
  names two behaviors and one ships, every prose surface must be swept for
  the other.
- Two avoidable compile rounds: `ColliderTrees` is not in avian's prelude,
  and `TorpedoSectionConfigHelper` cannot be tuple-constructed in tests
  (private field) - the second exists because the rig was hand-written
  instead of copied from torpedo_world four screens up, which already used
  the production `torpedo_section()` bundle. That is the third occurrence
  of reuse-production-helpers-in-tests.
- R1.1 (eager torpedo ray): the plan's own perf direction ("ray only for a
  shot that would otherwise fire") was applied to the turret consumer and
  forgotten on the torpedo consumer. Two consumers of one helper deserve
  one checklist pass each.

## What to improve next time

- Write CHANGELOG/wiki text from the final diff, then re-read the task
  title asking "does the prose claim anything the diff does not do?".
- Before hand-writing any test rig component list, grep the same file for
  an existing rig spawning the same entity kind.

## Action items

- [x] docs/LESSONS.md: reuse-production-helpers-in-tests bumped to x3 ->
  Pending promotions (work skill candidate).
- [x] docs/LESSONS.md: new lesson prose-from-diff-not-intent (x1).
- [x] docs/LESSONS.md: verify-engine-guarantees-in-source bumped (positive:
  wrong upstream doc comment caught by reading the implementation).
