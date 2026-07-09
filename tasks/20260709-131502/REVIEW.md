# Review: Torpedo takes no contact damage from its own ship at launch

- TASK: 20260709-131502
- BRANCH: torpedo-bay-spawn

## Round 1

- VERDICT: APPROVE (no BLOCKER/MAJOR; the MINORs below were addressed on the
  branch anyway before merge, see Responses)

Reviewed with an independent fresh-eyes pass over `git diff master...torpedo-bay-spawn`,
TASK.md, the avian 0.7.0 source (broad phase, hooks, ColliderOf) and the
bevy_common_systems damage path. Verified clean: hook logic and both ownership
lookup paths (ColliderOf is self-referential for body-is-collider entities, so
the turret path holds; filtering is fail-open if ColliderOf is missing);
either-collider hook activation matches the spawn-side flags; pair-creation-time
filtering confirms the rejected arming-gated design; turret behavior is
equivalent to master's hook; the rename is complete workspace-wide; the
test-support change is backward compatible; TASK.md/docs/CHANGELOG claims match
the code; module/prelude conventions followed.

- [x] R1.1 (MINOR) crates/nova_gameplay/src/sections/projectile_hooks.rs:126 -
  the physics tests hand-build a torpedo replica and register their own hooks,
  so deleting `ActiveCollisionHooks::FILTER_PAIRS` from the real spawn
  (torpedo_section/mod.rs) or reverting plugin.rs:36 would not fail any test;
  the only wiring evidence is a manual A/B run. Suggested: assert the wiring in
  the existing headless smoke path (e.g. an observer in
  examples/06_torpedo_range.rs that fails the autopilot run on torpedo-vs-owner
  damage).
  - Response: fixed in the follow-up commit - `06_torpedo_range` now installs
    `assert_no_owner_pair_damage`, an `On<HealthApplyDamage>` observer that
    panics when a damage event pairs a torpedo section with the firing ship's
    section (blast damage has the blast entity as source, so it does not
    trip). It is active in every run of the range, so the autopilot smoke
    fails on regression of either the flag or the hook registration.
- [x] R1.2 (MINOR) tasks/20260709-131502/TASK.md:95 - the Resolution defers the
  full suite and clippy to "CI on every PR", but .github/workflows contains only
  deploy-page.yaml and release.yaml on master and branch: no PR workflow runs
  cargo test in-repo. Suggested: run the suite once pre-merge, add a PR
  workflow, or correct the wording.
  - Response: wording corrected in TASK.md (tests deferred per the repo's
    AGENTS.md instruction, with a note that no in-repo PR test workflow
    exists). Not silently overriding the standing no-local-tests instruction;
    the gap is surfaced to the user in the flow report instead.
- [x] R1.3 (NIT) crates/nova_gameplay/src/sections/projectile_hooks.rs:66 - the
  orientation symmetry the comment promises is untested; `filter_pairs` is
  directly callable via `SystemState`, cheap to lock in.
  - Response: fixed in the follow-up commit - added
    `filtering_is_symmetric_in_pair_orientation`, which builds a bare `World`,
    inserts `ColliderOf` by hand, and asserts `filter_pairs` returns false for
    both orientations of the owner pair and true for a non-owner pair.
- [ ] R1.4 (NIT) crates/nova_gameplay/src/sections/projectile_hooks.rs:1 - the
  module is cross-cutting projectile infrastructure, not a section; a top-level
  nova_gameplay module (like flight.rs, juice.rs) would be a more accurate home.
  Acceptable as-is.
  - Response: leaving as-is, with reasoning: both consumers are section modules,
    the component rides on section-spawned projectiles, and the sections prelude
    is where its consumers already import from. Worth revisiting only if a
    non-section projectile source appears.
- [x] R1.5 (NIT) crates/nova_gameplay/src/sections/projectile_hooks.rs:39 - the
  single-element tuple queries (`Query<(Read<ProjectileOwner>,)>`) copy master's
  style but the tuple is noise; plain `Query<Read<ProjectileOwner>>` reads
  cleaner.
  - Response: fixed in the follow-up commit - both queries and their
    destructuring de-tupled.

## Round 2

- VERDICT: APPROVE

Verified the R1.1/R1.3/R1.5 fixes on the updated branch: the range observer
matches only torpedo-section-vs-ship-section pairs in both directions and is
exercised by the headless autopilot run (re-ran green, no owner-pair panic, 3
fired / armed / detonated, exit 0); the symmetry test asserts both orientations
plus the non-owner control and passes; the de-tupled queries compile clean.
R1.2's wording fix is accurate. R1.4 pushback accepted. No new findings.
