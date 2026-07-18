# RCS HUD indication on the velocity sphere (active palette + cap ring)

- STATUS: OPEN
- PRIORITY: 3
- TAGS: v0.7.0,feature,hud,spike

## Goal

Diegetic indication that RCS fine-adjust mode is active, on the velocity sphere
that already orbits the player and shows speed/gravity (hud/velocity.rs):

- Give the sphere an RCS-active state with a distinct palette, reusing the
  existing autopilot-presence palette switch (manual white/blue -> engaged cyan)
  as the pattern.
- Optionally render the `rcs_cap` as a bounding ring/shell so the pilot can see
  the small speed ceiling their nudges settle at.
- Active when the ship is in RCS mode (SHIFT held / `RcsIntent` present).

## Notes

Spike: tasks/20260718-122508/SPIKE.md. Depends on the RCS core primitive
(task 20260718-122906) and reads its active state. HUD widget + palette switch
in hud/velocity.rs (VelocityHudSource, DirectionMagnitudeMaterial). Needs a
/plan pass to break into steps.
