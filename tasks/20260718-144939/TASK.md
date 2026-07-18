# RCS cap ring on the velocity sphere (needs visual design + playtest)

- STATUS: CLOSED
- PRIORITY: 1
- TAGS: v0.7.0,feature,hud,spike

## Close-out (2026-07-18): WON'T-DO

Closed without implementing, by user decision. The delta-driven control change
(task 20260718-185826) plus the sensitivity retune (20260718-192708) made RCS
feel self-limiting in the hand: the pilot feels the speed cap through the
control response and does not need to read the ceiling magnitude off the HUD.
The RCS-active PALETTE (violet velocity sphere, parent task 20260718-122923)
already communicates the important bit - that RCS mode is engaged.

Weighed against that marginal payoff, the cap ring is speculative geometry/
shader work with an unresolved visual-design question (latitude ring vs. torus
child vs. shell, each reading differently) that can only be judged by a live
playtest. Not worth the cost now. Reopen only if a future playtest surfaces a
concrete "I can't tell how fast RCS will let me go" need.

## Goal

Split out of the RCS HUD task (20260718-122923), which delivered the RCS-active
PALETTE but deferred the "cap ring/shell". Render the `rcs_speed_cap` ceiling on
the velocity sphere so the pilot can see the small speed ceiling their nudges
settle at.

Why it was deferred (not a quick win):

- The velocity sphere is a FIXED-radius orbiting widget; speed drives a shader
  MAGNITUDE (`magnitude_input = speed / 100`, direction_shader_update_system in
  hud/velocity.rs:282-349), not the physical radius. So "a ring at the cap"
  needs a design decision: is it a latitude ring on the sphere at the shader
  magnitude for `rcs_cap/100`, a separate torus child, or a shell? Each reads
  differently.
- It is new geometry/shader work whose "does it read right" cannot be judged
  headless - it needs a by-eye pass in the running game (a `/verify` / playtest),
  unlike the palette which is a testable color swap.

## Notes

Spike: tasks/20260718-122508/SPIKE.md (the RCS family). Parent HUD task:
20260718-122923 (palette landed). Reference: the torus pattern in
hud/maneuver_instruments.rs (OrbitRingMarker); the sphere material
`DirectionSphereMaterial` (radius+sharpness uniforms) and its shader
`assets/shaders/directional_sphere.wgsl`. Only pick this up with a way to
playtest the visual. Needs a /plan pass.