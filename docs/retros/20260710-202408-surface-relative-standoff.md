# Retro: Surface-relative GOTO standoff

- TASK: 20260710-202408
- BRANCH: fix/surface-relative-standoff (squashed to master as 6a2357c,
  round-1 fixes recovered as b67a9ba)
- REVIEW ROUNDS: 1 (APPROVE; 2 MINOR + 2 NIT, fixed in-round)

## What went well

- The design decision (dedicated BodyRadius vs LockSignature-as-radius)
  was settled in the plan by reading both producers first: LockSignature
  is a scanner magnitude that will diverge from geometry when ships get
  signatures, so a one-line geometric component kept the meanings apart.
- The radius folded into the standoff parameter the pure helpers already
  take - zero changes to the arrival math one cycle after it was
  hardened, which is what compute-at-the-truth buys.
- The first park-tolerance failure (78.9u vs the 90u point) was read
  correctly as pre-existing terminal-creep release behavior, verified
  against the flat-space tests' -45 bound on master, documented in Known
  limits, and pinned on the queued ORBIT-parking task instead of being
  papered over with a silent tolerance bump.

## What went wrong

- NEARLY LOST WORK at landing: the pre-squash command chain gated the
  branch commit behind `cargo check ... | grep -cE "^error"` - grep -c
  prints 0 AND EXITS NONZERO on zero matches, so the `&&` chain silently
  skipped `git add && git commit`, the squash merged the pre-fix tree,
  and `sprout rm` deleted the worktree with the round-1 fixes
  uncommitted. Recovered only because every change was small and still
  in-context (re-applied as b67a9ba). Root causes, both process:
  1. a match-COUNTING grep used as a chain link - its exit code means
     "found matches", not "command succeeded";
  2. `sprout rm` ran without verifying the branch tip contained the
     fixes (`git log` on the branch, or `git status` in the worktree,
     would have caught it - the "contains modified or untracked files"
     fatal was even printed, after the branch was already force-deleted).

## What to improve next time

- Never gate a commit behind a grep in an `&&` chain. Put verification
  greps in their own command, or append `|| true` when the count itself
  is the output.
- Before `sprout rm` / squash-merge: confirm the worktree is clean
  (`git -C <worktree> status --porcelain` empty) and the branch tip is
  the commit just made. The squash's content should be checked against
  the intended tree, not assumed from the chain's last echo.
- Proposed for AGENTS.md (second landing-hygiene incident class after
  the no-worktree staging rule): a one-line "verify clean worktree +
  expected tip before sprout rm" convention.

## Action items

- [x] tatr 20260710-214316: ribbon terminates at the park point (review
  NIT R1.4, HUD polish, p15).
- [ ] Propose the AGENTS.md landing-hygiene line (do it in this session's
  wrap-up or next cycle's start).
