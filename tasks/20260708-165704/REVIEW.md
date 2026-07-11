# Review: Off-screen target/threat edge indicators (HUD)

- TASK: 20260708-165704
- BRANCH: feature/edge-indicators

## Round 1

- VERDICT: APPROVE

Verified independently: hud:: filter 74/74 green in the worktree (5 new
edge-indicator tests), cargo check --workspace green, fmt clean. Re-derived
the chevron geometry from the stroke placement instead of trusting the
constants: both strokes' rotated endpoints meet at (7.0, 3.8) in the 14 px
box - an up-pointing apex at top-center, which is exactly the orientation
`arrow_angle` in screen_indicator.rs expects for its rotation math. Checked
the widget contract the whole design leans on: `update_arrows` sets the
arrow Hidden while the anchor is on-screen and Inherited+rotated while
clamped (screen_indicator.rs), so an indicator whose only child is the
arrow renders nothing on-screen - and the new
`indicator_content_is_the_arrow_only` test pins that single-child
structure so a later content addition cannot silently break the property.
The tracked-set rule is a pure function with membership tests covering the
dedup (locked candidate = one Lock indicator), the own/uncommitted torpedo
exclusions, and kind reassignment on lock change.

The diff honestly matches TASK.md, including the deviation notes (respawn
on kind change instead of restyle, node-built chevron instead of an image
asset - both with reasons I find convincing at this scale). The initially
ticked-but-missing spike Fix record was caught and added before this
review. No findings.
