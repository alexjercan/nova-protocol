# Review: Torpedoes commit to their launch target and never retarget

- TASK: 20260707-143001
- BRANCH: feature/torpedo-pn-guidance (PR #31, same PR as the PN work per user)

## Round 1

- VERDICT: APPROVE

Delivers exactly the requested behavior: the targeting decision is made once, at
launch. `TorpedoTargetChosen` is stamped the first time the input system processes
an owned torpedo - with the locked target when one exists, alone (dumb-fire) when
not - and every targeting query now requires `Without<TorpedoTargetChosen>`, so
neither a later lock (the reported bullet case) nor a post-target-death re-lock can
ever re-target a torpedo. Target death still drops the link (freeze-and-continue
from 20260707-100004 preserved); detonation on the frozen position still works
since the fuze checks `TorpedoTargetPosition`, not the live entity. Both example
autotargets mirror the contract, keeping the harnesses faithful to the game rule.

Verified independently in the worktree:

- `cargo test -p nova_gameplay`: 26/26 green, including the two new regressions -
  `dumbfire_torpedo_ignores_later_locks` (lock appearing after a no-lock launch is
  NOT assigned - the exact reported scenario) and
  `committed_torpedo_does_not_retarget_after_target_loss`.
- `cargo clippy -p nova_gameplay`: clean.
- Headless smoke (Xvfb): 06 = 3 fired / 3 detonated, 07 = 2 detonations, 0 panics -
  no regression from the example autotarget changes.

Design check: bullets remain valid targets by decision (no cast filtering added);
an un-owned torpedo (future AI ships) is deliberately left uncommitted for its own
controller's targeting system to commit. Sensible.

- [ ] R1.1 (NIT) examples/06_torpedo_range.rs + examples/07_torpedo_guidance.rs -
  in an example, the range autotarget and the player targeting system can both
  process the same fresh torpedo in the same frame (both see it uncommitted until
  commands apply), so which target wins that one frame is command-order dependent.
  Harmless here (both assign a legitimate target, commitment is identical), and it
  resolves permanently after one frame. Not worth ordering machinery in a dev
  example.
  - Response: Agreed, leaving as-is; documented by this finding.
