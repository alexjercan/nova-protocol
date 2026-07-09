# Review: AI aim and turret lock-on anchor at the ship root origin

- TASK: 20260709-150711
- BRANCH: fix/live-structure-anchor (implementation commit ef03f6c)

## Round 1

- VERDICT: APPROVE

Verified independently: fmt clean, `cargo check --workspace` green, the new
tests pass (2 helper unit tests incl. the render-scale trap, 2 AI behavioral
tests, the player aim-ray test, and a cone-discrimination test whose
candidate sits inside the cone from the anchor but 33 degrees off the origin
bearing - it fails without the fix by construction), and the refactored
camera's 4 existing anchor tests stay green. Grep confirms no
player-position stragglers in ai.rs; all four AI reads, both player origins
and the camera share the single helper. The GlobalTransform -> Transform
switch on the targeting Single is sound (ship roots are top-level; values
identical, and it matches the camera's read).

One prose observation, no finding: AI ships still measure FROM their own
root origin (to_player = anchor - own translation). The angular error is
bounded by own-COM-offset over target distance (a few degrees at combat
ranges, under the 0.95 alignment gates), and the own-anchor half naturally
belongs to the AI rotation-path task (20260709-155921). Not worth blocking;
noted for that task.

No findings.
