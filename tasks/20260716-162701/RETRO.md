# Retro: ghost ship at 0 HP - structural death backstop

- TASK: 20260716-162701
- BRANCH: fix/ghost-ship-at-zero-hp (landed 02ed8a45)
- REVIEW ROUNDS: 2 (round 1 REQUEST_CHANGES: 1 MAJOR record-accuracy, 2 MINOR; round 2 APPROVE)

## What went well

- Rig-before-fix on an unreproducible report: five candidate mechanisms
  were encoded as boundary tests BEFORE any code change; exactly one went
  red, which WAS the diagnosis, the fail-first A/B came free, and the four
  green cases became permanent pins. The right shape for "happened once,
  can't repro" - the fix then landed at a seam that closes the whole class,
  not just the one path.
- The plan's consumer-sweep step (not the rig) surfaced the second
  candidate explanation of the sighting - HUD sub-1% rounding displays 0%
  on a living ship - filed as 20260716-165617 instead of widening the diff.
- The out-of-context reviewer re-ran the sabotage A/B itself, and read
  bevy_ecs and bcs SOURCE to verify observer re-trigger semantics and the
  no-false-positive argument - and caught the one thing the implementer
  could not see (below).
- A mid-cycle user process request (the tatr tagging rule) was absorbed
  into AGENTS.md + memory without derailing the cycle.

## What went wrong

- R1.1 (MAJOR): the record declared `handle_parent_destroy` nonexistent and
  "fixed" a comment that was correct - the symbol lives in the bcs
  DEPENDENCY and is the very observer the fix relies on. Root cause: the
  existence grep covered nova crates only, never the dependency checkout. A
  nonexistence claim is only as good as the search's scope.
- R1.2: the rig delivered less than its ticked plan step promised (despawn
  asserted, OnDestroyed not; one case silently swapped). Root cause:
  adapting to rig constraints during implementation without amending the
  step text - the silent-narrowing variant of half-ticking.
- A known promoted lesson recurred: two cargo test filters chained in one
  invocation (one-cargo-test-filter, x4 at the time) - the run silently
  tested nothing; caught one command later.

## What to improve next time

- Any "X does not exist / is never called" claim must grep the WHOLE
  dependency surface (~/.cargo/git checkouts, local dep repos), not just
  the workspace - especially before editing a comment that asserts it.
- When a rig cannot deliver a step's clause, amend the step in the same
  edit that adapts the rig; the checklist is the contract.
- The one-filter cargo rule keeps recurring under time pressure; consider
  a shell alias/wrapper if it hits x6.

## Action items

- [x] Ledger: bumped out-of-context-review-pass, half-ticked-compound-steps,
      one-cargo-test-filter, verify-engine-guarantees-in-source (sharpened
      with the dependency-blind-grep variant); added positive
      rig-before-fix-on-unreproducible.
- [x] Sibling display fix filed and widened: 20260716-165617 (v0.7.0, p50).
- [ ] If the ghost recurs in playtest after 165617 also lands, reopen with
      the new evidence per the close record.
