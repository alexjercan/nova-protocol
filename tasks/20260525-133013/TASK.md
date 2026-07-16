# Add spawn-less visual mode for low-end machines

- STATUS: CLOSED
- PRIORITY: 22
- TAGS: v0.7.0, performance, chore

Flag to skip particles/shaders for performance. Legacy #127.

## v0.7.0 (20260716, spike tasks/20260716-122954/SPIKE.md)

Pulled into v0.7.0 (p22), re-framed: not a bare flag but the graphics-quality
preset the settings menu (20260711-180511) exposes, tuned against the
frame-time baseline (20260716-123551) so what it skips is what the numbers say
is expensive. Plan: docs/plans/20260716-v0.7.0-plan.md, strand 2.

## Seam already landed by 20260711-180511 (settings menu)

The settings task shipped the surface this task extends, so the work here is
purely "make Low/Medium skip more, measured":

- `nova_gameplay/src/settings.rs` owns `GraphicsQuality { Low, Medium, High }`
  (a `Resource`), persisted cross-platform by the menu, with a segmented
  Low/Medium/High button in Settings > Graphics (main menu + pause).
- `apply_graphics_quality` (runs on `resource_changed::<GraphicsQuality>`) is
  the single seam. TODAY it maps only onto `JuiceSettings` (High = shake +
  flash on; Medium = flash on, shake off; Low = juice master off), because that
  is the one visual-cost knob that existed pre-baseline.
- THIS task extends that same `match` to also gate the expensive things the
  frame-time baseline (20260716-123551) flags - hanabi particle spawns and
  asteroid-scatter density are the candidates named in the plan. Add a
  particle/scatter gate keyed off `GraphicsQuality` and fold it into
  `apply_graphics_quality` (or a sibling system reading the same resource);
  keep each tier observably distinct. No new UI or persistence needed.
- The `advertised-but-unwired` lesson applies: whatever a tier claims to skip
  must actually be skipped and shown to be, tuned to the baseline numbers.
