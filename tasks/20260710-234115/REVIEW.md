# Review: Engaged-state tint for the velocity sphere

- TASK: 20260710-234115
- BRANCH: feature/engaged-palette-tint

## Round 1

- VERDICT: APPROVE (1 MINOR, fix at implementer's discretion)

Verified: diff read in full; fmt clean, `cargo check --workspace
--examples` clean, 6 velocity tests pass. The scope refinement (sphere
is the tint's whole payload; holos are engaged-only) is recorded in
TASK.md and correct - checked that ribbon/ring/gate/spoke all despawn
outside engagement. Change-guard logic is right: `*palette == desired`
reads through Deref (no change-detection dirty), the write happens only
on a flip. A dead target reads as disengaged for the teardown frame gap,
which is harmless. Gravity widgets are skipped by source, not palette
value, so even a yellow-equals-engaged coincidence could not tint them.
The unticked by-eye step is honestly reported in the closing notes.

- [x] R1.1 (MINOR) crates/nova_gameplay/src/hud/velocity.rs tests - the
  tests assert the palette component seam but never the material
  rewrite, which is the actual visible effect; the child-walk and the
  two `get_mut` branches run uncovered. Hand-spawn a child with
  VelocityHudIndicatorMarker + a real material handle (ChildOf(widget)),
  run the system, and assert the asset's base_color flipped.
  - Response: fixed in-round - engaging_rewrites_the_child_materials
    hand-builds both children with live assets and asserts both
    base_colors flip to the ENGAGED values.

Round 1 close-out: fix verified (test reads the assets back, 7/7
velocity tests green). APPROVE stands.
