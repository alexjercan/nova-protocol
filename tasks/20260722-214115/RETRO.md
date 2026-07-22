# Retro: Ledger per-chapter look (20260722-214115)

## What went well

- The minimal-look constraint (two base cubemaps, no new art) was delivered
  honestly: a deliberate per-chapter starting `cubemap:` assignment plus ONE
  motivated `SetSkybox` accent at each chapter's dramatic peak (ch1 4th-ping
  reveal, ch3 debris pinch, ch4 sell-path Auditor arrival), each in the
  beat's own handler reusing that beat's existing one-shot guard so it fires
  exactly once. No accent at OnStart (that is what `cubemap:` is for).
- The right fix for "SetSkybox panics without an AssetServer in the rig" was the
  PRODUCTION-FAITHFUL one: add `AssetPlugin` + `init_asset::<Image>()` to the
  ch3/ch4 walk rigs (production HAS an AssetServer), NOT to make the engine
  action tolerate an absence production never sees. `production-faithful-rigs`
  favors giving the rig what production has over softening the code. The engine
  (`crates/nova_scenario/`) stayed untouched, honoring the task's data-only scope.
- The `ledger_skybox` test ties each accent to its beat via unique StoryMessage
  text and forbids OnStart/new-art targets, so relocating or dropping an accent
  fails a test (not a shallow presence check).

## What went wrong / was tricky

- PROCESS FAILURE (mine, not the implementer's): I treated the work agent as
  finished on its FIRST notification, which was a spurious "Waiting for the
  background completion notification now" non-report - the agent was still alive.
  I then started editing `actions.rs` (an engine null-safety guard for SetSkybox)
  IN THE SAME WORKTREE while the agent kept working and committing. We collided:
  my guard edit was clobbered by the agent's churn, the agent saw my edit as a
  "parallel process" and stashed a copy, and I burned effort on an approach the
  agent had already solved better. Recovery: once the agent's REAL report landed,
  I discarded my uncommitted engine changes (`git checkout` actions.rs to HEAD,
  dropped the stray stash) and adopted the agent's clean committed state, then
  re-verified green.
- The two approaches diverged on merit and the agent's won: engine guard (tolerate
  missing AssetServer) vs rig fix (give the rig an AssetServer). The rig fix is
  more production-faithful and stays in scope - so discarding my guard was correct,
  not just expedient.

## Lessons / what to do differently

- A background agent is NOT done until its notification carries a real, complete
  report. A vague "waiting..." / "no action needed" message means STILL RUNNING or
  confused - do NOT start editing its worktree on the strength of it. Never edit a
  sprout worktree that has a live agent in it; one writer per worktree. (This is
  the `shared-checkout` discipline applied to agent worktrees.)
- When a rig can't execute a shipped action for lack of a resource, prefer giving
  the rig the resource production has over loosening the action - unless the
  action genuinely runs in production without it. (`production-faithful-rigs`.)
- A latent robustness note remains: `SetSkybox` still hard-unwraps `AssetServer`
  (actions.rs:324) and would panic in any future rig that drives it without an
  AssetPlugin. Left as an optional hardening follow-up, not forced into this
  data-only task.

## Follow-ups

- (Optional, not filed) Engine hardening: make `SetSkybox` warn-and-skip on a
  missing AssetServer so future content rigs need no AssetPlugin. Deferred -
  raise at Finish if the owner wants the robustness.
- Owner Finish replay confirms the actual VISUAL look (the tests are data-level
  wired-proofs, not renderer checks) - batched in GOAL.md.
