# Retro: Lock-on acquisition dwell (radar hold-to-lock)

- TASK: 20260708-165703
- BRANCH: feat/lock-dwell-mechanic (landed, tangled - see below)
- REVIEW ROUNDS: 1 (APPROVE)

See TASK.md for what changed and the evidence; this is process only.

## What went well

- Reading the REAL `update_radar_search` before planning caught a stale premise:
  the spike/task said the lock was "instant", but the deliberate-radar rework had
  already made it a hold gesture. So the change was a clean one-point gate, not a
  new subsystem. The plan-from-code discipline (a repeated nova lesson) paid off.
- An independent out-of-context correctness pass (subagent given only the diff +
  targeted questions) found the `f32::clamp` panic on misordered knobs that the
  shared implementer/reviewer session would likely have rationalized away. Worth
  the tokens on any load-bearing branch.
- Neutralizing the dwell to zero in `gesture_app` kept the ~30 legacy latch /
  keep-last / tap-clear tests testing THEIR axis, and gave the dwell its own five
  focused tests, instead of churning every gesture test with +13 frames.

## What went wrong

- A test-only `RadarState { .. }` struct literal in `hud/lock_crosshairs.rs` broke
  the test build after I added two fields to the `Default` struct. `cargo check`
  (non-test) was green and hid it; only `cargo test` surfaced it. Root cause:
  trusting a non-test `check` after a struct-shape change.
- THE BIG ONE - shared-checkout WRITE leak. During the squash-land I ran
  `git merge --squash` in one tool call, then a SEPARATE call to inspect the
  staged file list, then a THIRD to commit. In that multi-second window a parallel
  `/compound` job ran `git commit -a`/`git add -A` on the shared main checkout and
  swept my entire squash-staged index into ITS commit (`fa828aca`, the 002105
  compound). My work landed correctly on master but tangled inside an unrelated
  commit, violating "one commit per task". Root cause: I left the index dirty
  across tool calls in a checkout I do not exclusively own.

## What to improve next time

- Land as ONE atomic command: `git merge --squash <b> && git commit ...` with the
  branch check in the same line, no inspection step in between. Inspect the diff
  on the BRANCH before landing, never in the staged-but-uncommitted window.
- After changing a struct's field set, run the test-target build (`cargo test`,
  not just `cargo check`) or grep `StructName {` across the crate before trusting
  green.

## Action items

- [x] Ledger: added `shared-checkout-write-leak` (write-side sibling of
  `shared-checkout-reads-race`).
- [ ] No follow-up code task: the leaked commit `fa828aca` contains the complete,
  correct, green task-1 deliverable; rewriting shared master history while parallel
  jobs commit would be more dangerous than the messy provenance. Left as-is.
