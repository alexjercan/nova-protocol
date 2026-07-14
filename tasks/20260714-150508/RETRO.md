# Retro: content model + generic kind-router

- TASK: 20260714-150508
- BRANCH: modding/content-model
- REVIEW ROUNDS: 1 (APPROVE)

Process only; what/why in TASK.md, design in the spike (150410), family status in
110502's fix-record.

## What went well

- **The generic-first re-plan (spike 150410) delivered its promise.** This foundation
  makes every future content kind "one `Content` variant + one router arm" instead of a
  bespoke catalog - the structural fix for the "fold" bump. And the migration was
  behavior-preserving (same 7 sections + 5 scenario ids), verified by parity + both
  windowed examples.
- **`generate-data-from-code` + the parity write-on-missing was the recovery mechanism.**
  When the implementing agent left the 5 `.content.ron` files ungenerated, re-running the
  content parity test regenerated them deterministically and then guarded them - no
  hand-authoring, no guesswork.
- **Lessons held:** `git add -A` in the worktree caught the new `serde` dep + its
  `Cargo.lock` line (no stale-lock); the out-of-context reviewer adversarially proved the
  parity guard genuine (appended junk -> fail); windowed behavior verification, not just
  unit tests.

## What went wrong

- **The implementing agent stalled mid-build and left an ambiguous partial state.** Its
  notifications read "still building" for a long time; when I inspected the worktree the
  code changes + old-file deletions were done but the 5 regenerated RON files were
  missing (only the hand-migrated demo file existed). Root cause: the agent hit a very
  long `cargo test --workspace` build and its loop ended before running the generation
  step; the "in progress" notifications were misleading, and its full (successful) report
  arrived only much later. I could not trust the notification as the final state.

## What to improve next time

- When a subagent reports an ambiguous or "in progress" state (or a stall), INSPECT THE
  WORKTREE (`git status` + compile + run the deterministic generators) before concluding
  done-or-broken. For data-file work, the parity/generator write-on-missing usually
  completes it deterministically - run it rather than re-dispatching or hand-finishing.

## Action items

- [x] Lessons ledger: added `agent-interrupted-verify-worktree`.
- Family continues at 134115 (ship kind = Content::Ship) -> 134119 (folder bundle) ->
  134123 (base-as-bundle) -> 134127 (mods + demo). All now "add a variant / package into
  folders" on this foundation.
