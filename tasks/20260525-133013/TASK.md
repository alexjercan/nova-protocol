# Add spawn-less visual mode for low-end machines

- STATUS: OPEN
- PRIORITY: 22
- TAGS: v0.7.0,performance,chore

Flag to skip particles/shaders for performance. Legacy #127.

## v0.7.0 (20260716, spike tasks/20260716-122954/SPIKE.md)

Pulled into v0.7.0 (p22), re-framed: not a bare flag but the graphics-quality
preset the settings menu (20260711-180511) exposes, tuned against the
frame-time baseline (20260716-123551) so what it skips is what the numbers say
is expensive. Plan: docs/plans/20260716-v0.7.0-plan.md, strand 2.
