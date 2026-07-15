# Retro: local mod cache + mods:// asset source

- TASK: 20260715-142906
- BRANCH: feature/mod-cache (landed on master as 02b0e5ad)
- REVIEW ROUNDS: 2 (R1 REQUEST_CHANGES with one MAJOR, R2 APPROVE)

## What went well

- First cycle of this flow with a DELEGATED implementation (a background agent
  worked the precise plan in the worktree while the orchestrator stayed free).
  The agent's close-out was verifiably honest: every claim the reviewer
  re-derived held, it ran its own sabotage matrix unprompted, flagged a
  pre-existing master failure (content_ron_parity -> task 172138) instead of
  absorbing or hiding it, and recorded five plan deviations with reasons.
- The rescope decision (splitting the network half to 163508 at plan time)
  kept the branch reviewable at ~1700 lines and let this half ship with a
  no-network e2e (install_local + real gauntlet mod through the production
  source and merge).
- Review pushback worked as designed: the reviewer's R1.1 escape scenario was
  half-right (the validation gap was real; the live exploit was blocked by
  bevy's unapproved_path_mode Forbid default), and the implementer proved the
  correction empirically (sabotaged only the sandbox, watched the e2e stay
  green) instead of arguing - then kept the sandbox anyway as
  defense-in-depth. Both sides updated their records.

## What went wrong

- R1.1 (MAJOR): the cache API validated its inputs but the LOAD path trusted
  the index it read back - the same validate-at-the-boundary thinking that
  produced 142900's R1.1 (membership vs existence) recurring one layer up.
  Root cause: guards were attached to the WRITE side (store/install) and the
  read side was assumed clean because "we wrote it" - but the index is
  user-writable local data and 163508 will write portal-derived records into
  it. Trust boundaries need guards on EVERY crossing, not just the first.
- The implementer agent went silent past the fallback heartbeat once
  (finished 40+ minutes of work without interim signal); the
  agent-interrupted-verify-worktree lesson (inspect the worktree, not the
  transcript) resolved it in one command.

## What to improve next time

- When data crosses a trust boundary twice (written by one path, read back by
  another), validate at BOTH crossings - and say in the plan which reads are
  trusted and why.

## Action items

- [x] docs/LESSONS.md: bumped `validate-membership-not-existence` with the
  read-back variant; bumped `out-of-context-review-pass` (empirical premise
  correction) and `agent-interrupted-verify-worktree`.
