# Retro: content_lint mount-base adjacency check

- TASK: 20260717-162121
- BRANCH: feat/lint-mount-adjacency (landed as squash eeef46dd)
- REVIEW ROUNDS: 2 (R1 APPROVE with 1 MINOR + 3 NITs, all addressed; R2
  verified all four and re-approved)

## What went well

- Plan-time re-derivation rewrote the spec before any code: the seed prose
  said "for every ship section", but a 28-ship content survey plus
  re-reading the seeding task's NOTES (not recalling it) narrowed the
  check to Turret/Torpedo kinds and surfaced the load-bearing fact that
  most shipped ships seat the aft turret against the CONTROLLER cell. A
  to-the-letter implementation of the seed would have false-flagged every
  shipped ship (thrusters all point -Y at empty space).
- Two independent fail-firsts on the real shipped bug: the builder-side
  revert of one gunship roll (lint exit 1, restore regenerated
  byte-identical) and the out-of-context reviewer's own RON-side sabotage
  (gate red with the exact message, restored clean).
- The out-of-context review pass caught a real MINOR shared-session eyes
  missed (non-unit hand-typed quats defeat the axis snap) - and
  re-deriving the agent's claim in-session caught the agent's own wrong
  sub-claim (glam mul_vec3 with q = 0 returns v, not 0), so the finding
  landed with a corrected derivation.
- Ledger lessons applied preventively and none recurred: fixtures swept
  before the first lint run (lint-arm-sweeps-own-fixtures, zero
  collisions), and `--features serde` on the very first nova_scenario
  test invocation after five prior cold-compile burns.

## What went wrong

- One compile-error cycle: check_mount_adjacency used Vec3 with no import
  because the module's non-test half never imports bevy math and the code
  was written by analogy to the test module (which does). Root cause:
  did not read the target module's own import block. Cost one cold check.
- Piping discipline slipped: a `cargo test ... | tail -3` chain ate the
  lib `test result:` line (only the last test binary's line survived the
  window), forcing a re-run to learn whether the lint tests passed.
  Result-line variant of piped-cargo-masks-exit-code; bumped to x4.
- The master merge surfaced a red content_ron_parity inherited from
  sibling 20260717-201534: a hand-added RON comment in a GENERATED
  artifact, which gen_content cannot reproduce. Diagnosed per
  merge-red-check-preexisting (git show, not blame-this-branch), fixed as
  a named merge-integration regen on the branch; the sibling's linger
  flip survived via its builder change, only the comment dropped.

## What to improve next time

- New code in a module you have not written in before: read ITS import
  block first, not a sibling module's.
- When one command runs several test binaries, filter output with
  grep "test result" (keeps every binary's line), never tail -N.
- Prose about generated content belongs in the BUILDER that generates it;
  the parity test is the contract hand-edits violate.

## Action items

- [x] docs/LESSONS.md: generate-data-from-code x3 (hand-edit mirror
  variant, moved to Pending promotions), merge-red-check-preexisting x2,
  out-of-context-review-pass x29 (verify-the-verifier clause),
  piped-cargo-masks-exit-code x4 (result-line variant),
  lint-arm-sweeps-own-fixtures preventive-application note.
- No follow-up code tasks: the parity gate already guards generated
  artifacts, and the inherited red landed fixed with this task's squash.
