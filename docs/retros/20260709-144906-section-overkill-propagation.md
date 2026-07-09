# Retro: Overkill damage no longer kills the whole ship

- TASK: 20260709-144906
- BRANCH: fix/section-overkill-propagation (squash-merged as 9b7d031)
- REVIEW ROUNDS: 0 (no formal /review this cycle - self-review only; the user
  drove the landing steps manually)

What shipped is in the task's Resolution and
`docs/2026-07-09-section-overkill-propagation.md`. The headline: a two-line
clamp in the shared crate's damage propagation, but the cycle's real work was
proving the mechanism and threading a two-repo, git-rev-pinned dependency.

## What went well

- **Proved the risky assumption against source before writing the fix.** The
  whole approach hinged on "does mutating an entity event during propagation
  carry to ancestors?" Instead of guessing, I read bevy 0.19's `event/trigger.rs`
  and confirmed the propagation loop reuses the same event pointer per target.
  The fix was correct on the first try because the uncertainty was retired
  first, not discovered by a failing test later.
- **Surfaced the fork at the cheapest moment.** The task had two real forks -
  clamp-in-bcs vs nova-only, and how to handle the outward-facing bcs push. I
  asked both up front with one AskUserQuestion before building, and got a clean
  mandate (clamp in bcs, verify-local-you-push) instead of building the wrong
  half and reworking.
- **Decoupled verification from the push.** nova pins bcs by git rev, so the fix
  was unverifiable end-to-end until bcs shipped. A temporary, uncommitted
  `[patch]` to the local bcs worktree let the full nova pipeline run against the
  fix with zero outward-facing action - so "does it actually work" was answered
  before anything was pushed, and the push became a mechanical, already-proven
  step.
- **Layered the tests to the layers of the bug.** bcs unit tests pinned the
  mechanism (overkill clamp, preserved fatal bubble, corpse-hit charges zero);
  the nova physics test pinned the real integration (ship survives overkill).

## What went wrong

- **Exact-equality assertion on a physics-touched value - for the third retro
  running.** The first regression asserted `root.current == 100.0`; it came back
  `99.978`, from negligible contact damage between the two touching unit-cube
  sections. Root cause: I reached for `assert_eq!` on a float that a real avian
  step perturbs, even though the last two retros (COM, and the 06 observer
  before it) already flagged exact/false-precision assertions as a trap. The
  lesson was on the shelf and I still had to be bitten to apply it. Fixed with a
  1.0 hp tolerance that still cleanly separates ~100 from 0.
- **Flow's "land on the main checkout" collided with background isolation.** The
  squash-merge staged fine via git, but editing TASK.md's close-out in the main
  checkout was rejected by the bg-isolation guard. I had to reset the staged
  squash, make the doc edits on the branch, re-commit, and re-squash. Not wrong,
  but wasted a round-trip. Root cause: I edited close-out docs after starting the
  merge into the guarded checkout instead of finishing all file edits on the
  branch first.

## What to improve next time

- **Default to tolerance for any assertion over a value a physics/float step can
  touch; reserve `==` for integers or values exact by construction.** This is now
  a repeated pattern - promote it from retro to a standing rule (action below).
- **In a background /flow, finish every file edit (incl. TASK.md close-out and
  the retro) on the branch, then do only git operations in the main checkout.**
  The shared checkout is edit-guarded; treat it as git-only.
- **When a cycle skips /review, say so and consider whether the self-review was
  enough.** Here the fix was small and source-verified, so self-review held - but
  the skip should be a conscious call, not a silent omission.

## Action items

- [ ] Propose AGENTS.md "Testing and examples" note: prefer tolerance-based
      assertions over exact equality for any value produced by a physics step or
      float accumulation (3rd occurrence across retros - proposed to the user,
      pending their OK to edit the global file).
- [x] Documented the cross-repo verify-via-local-patch pattern in
      docs/2026-07-09-section-overkill-propagation.md for the next git-rev-pinned
      dependency fix.
