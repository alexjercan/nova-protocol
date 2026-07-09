# Retro: Torpedo launch self-damage fix (projectile owner collision filter)

- TASK: 20260709-131502
- BRANCH: torpedo-bay-spawn (squash-merged as 6cd7406)
- REVIEW ROUNDS: 2 (round 1 APPROVE with 2 MINOR + 3 NIT, all addressed or
  answered; round 2 APPROVE)

What shipped and why is in the task's Resolution and
`docs/2026-07-09-projectile-owner-collision-filter.md`; this is about the
process.

## What went well

- **Reading the physics engine's source before planning.** The exact avian
  semantics (one CollisionHooks type per app, either-collider flag activation,
  filtering at broad-phase pair creation) were pinned down from the vendored
  source before the plan was written. The design survived an adversarial
  review with zero correctness findings, and two dead-end designs (a second
  torpedo-specific hook, an arming-gated filter) died on paper instead of in
  code.
- **A/B smoke as evidence.** Running the identical headless range command on
  master and on the branch turned the fix from "tests pass" into "master logs
  a 0.30-damage owner pair on every launch, the branch logs none". Cheapest
  possible proof for a physics bug with an existing range example; repeat this
  pattern.
- **The review finding that mattered got implemented immediately and earned
  its keep the same hour.** R1.1 said the replica-based tests could not catch
  wiring regressions; the new in-range assertion
  (`assert_no_owner_pair_damage`) panicked on its very first run - on exactly
  the class of failure it exists for (see below for what that failure really
  was). A wiring-level assertion beats N more unit tests for this bug class.

## What went wrong

- **A shared target dir ran the wrong code and faked a bug.** Following
  docs/development.md's advice, the worktree smoke was built with
  CARGO_TARGET_DIR pointed at the main checkout's target. The master A/B run
  in between rebuilt master's `nova_gameplay` into that dir, and the next
  worktree build linked it: the "branch" smoke ran master code (no filter),
  producing an "intermittent filter failure" panic, and a later build failed
  on a symbol that only exists in the worktree. Root cause: the doc's
  "sharing is safe" claim is false once the two checkouts diverge in
  workspace crates - same package names and versions, colliding artifacts.
  Cost: one wild-goose investigation. The doc is corrected in the same
  branch.
- **The headless control test tripped the destroy pipeline.** Giving the
  control torpedo its realistic 1 hp meant the (intended) contact damage
  killed it, and the render-facing explode observer panicked in the headless
  harness (needs `Assets<StandardMaterial>` and a `GlobalRng`). Root cause:
  the test was designed around "damage lands", but 1 hp turns any damage into
  a death cascade the harness cannot host. Rework: huge health on both sides;
  the invariant is the damage, not the dying.
- **Review bookkeeping got ahead of reality.** REVIEW.md's responses and the
  Round 2 verification text were drafted before the fixes existed, on the
  assumption they would verify cleanly - and the very next smoke run
  panicked. Everything was re-verified before commit, so the trail on disk is
  truthful, but the ordering invited a false record. Same family as the
  multi-thruster retro's "never pre-commit to text you have not just read".
- **`tatr new` silently ate a task.** Two `tatr new` calls in the same second
  produced the same timestamp ID; the second overwrote the first's TASK.md.
  Caught only because the duplicate ID was noticed in the output. The tatr
  CLI needs collision handling (suffix, retry, or refuse); that fix belongs
  in the tatr project, not this repo.
- **The "CI runs tests on every PR" premise is unbacked.** The repo's
  AGENTS.md (and session memory) defer cargo test/clippy to CI, but review
  found no in-repo PR workflow - only deploy and release. Either the workflow
  is missing or the checks live somewhere not visible here. Follow-up task
  filed rather than silently re-litigating the standing instruction.

## What to improve next time

- When a run contradicts an earlier run of "the same" binary, check binary
  provenance first (which target dir, which source built it) before debugging
  the behavior. Wrong-code-ran is cheaper to rule out than physics.
- Never point CARGO_TARGET_DIR at another checkout of the same workspace
  unless the workspace crates are identical; docs now say so.
- In physics-level tests, keep every entity far from zero health unless the
  death cascade itself is the subject (second retro mention: flight retros
  hit the harness-hosting problem from the resource side; this is becoming a
  rule).
- Write REVIEW.md responses and round verdicts only after the fixes exist and
  are verified; findings first, fixes second, bookkeeping last.
- Back-to-back `tatr new` calls: pause between them or verify distinct IDs
  until tatr handles same-second collisions.

## Action items

- [x] docs/development.md worktree-cache advice corrected (shipped with the
  task, 6cd7406).
- [x] tatr 20260709-140620: center-of-mass staleness bug (user report, filed
  during this cycle).
- [x] tatr 20260709-140559: torpedo blast self-harm / salvo fratricide
  balance question (observed in the smoke logs).
- [ ] tatr 20260709-140816: reconcile the CI test story (add a PR workflow or
  correct AGENTS.md's claim).
- [ ] Report the tatr same-second ID collision to the tatr project (external
  repo; not filable here).
