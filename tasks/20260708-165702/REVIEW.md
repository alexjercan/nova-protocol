# Review: Locked-target info readout (HUD)

- TASK: 20260708-165702
- BRANCH: weapons-hud (implementation commit 36c87a4)

## Round 1

- VERDICT: APPROVE

Verified independently: fmt clean, `cargo check --workspace` green, all 35
hud tests pass, and the 12_hud_range scripted run passes with the readout
stages - the shown distance matches the actual separation ("DST   150m" at
150 m), the closing-speed sign convention is proven live (rest: "CLS  -0.0
u/s"; approach burn: "CLS +13.5 u/s" - positive while closing, computed as
-(rel_velocity dot los_dir), consistent with the unit tests), and the full
target's bar renders 100%. The child-of-reticle attachment matches the plan
decision and buys edge-tracking and visibility inheritance with zero readout
projection code; the degradation paths (velocity-less body -> "CLS   ---",
health-less target -> hidden bar) are tested behaviorally. The
one-enum-component-per-line design (instead of two marker types) avoids the
conflicting-Text-queries problem cleanly.

- [x] R1.1 (NIT) crates/nova_gameplay/src/hud/torpedo_target.rs:299-304 -
  the health-fill `Node.width` and `BackgroundColor` are rewritten every
  frame while locked, unlike the guarded `Text` writes just above; guard
  both on inequality for consistency (the fill only changes when the target
  takes damage).
  - Response: fixed in 3290ca6 - width and color writes guarded on inequality.
- [x] R1.2 (NIT) crates/nova_gameplay/src/hud/torpedo_target.rs:373 -
  `let _ = readout;` at the end of `readout_rides_the_reticle_node`; bind
  the tuple field as `_` in the destructuring pattern instead.
  - Response: fixed in 3290ca6 - destructures &ChildOf only; also dropped the redundant query_filtered.

## Round 2

- VERDICT: APPROVE

Both nits verified in 3290ca6; 35 hud tests green, fmt clean. No new findings.
