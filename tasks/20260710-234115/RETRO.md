# Retro: Engaged-state tint for the velocity sphere

- TASK: 20260710-234115
- BRANCH: feature/engaged-palette-tint (squashed to master as b96e2e5)
- REVIEW ROUNDS: 1 (APPROVE; 1 MINOR, fixed in-round)

## What went well

- The plan-time scope refinement was the whole task: reading velocity.rs
  and the holo modules BEFORE writing steps revealed that ribbon, ring,
  gate, and spoke only exist while engaged - so "family-wide shader
  treatment" (the spike's phrasing) collapsed to one palette-swap system
  on the velocity sphere. The vague version would have grown a shared
  material extension for elements that cannot even display the manual
  state.
- The change-guard idiom (compare through Deref, write only on flip)
  avoided per-frame material re-uploads and, incidentally, per-frame
  change-detection dirtying of the palette component.

## What went wrong

- R1.1: the first test pass covered the decision seam (palette
  component) but not the effect (material rewrite) - same lesson class
  as the spoke's untested well-death exit one cycle earlier: coverage
  followed what was EASY to assert headless, not what the system is FOR.
  The fix (hand-building children with live assets) was cheap once
  actually attempted.
- Trivial: Assets::get_mut returns a guard needing a `mut` binding; one
  compile round lost.

## What to improve next time

- When a system's output is an asset/material/resource mutation, the
  lifecycle test must read that output back, even if a component seam is
  more convenient - "the seam test covers it" is the smell to catch at
  writing time. Second occurrence of the coverage-shape lesson in two
  cycles; if it appears a third time it belongs in the work skill's
  test guidance.

## Action items

- [x] R1.1 fixed in-round.
- [ ] User by-eye pass outstanding (chip offsets, spoke thickness, tint
  colors) - noted in TASK.md 20260710-234115's unticked step; file
  constant tweaks as a small task if anything looks off in play.
