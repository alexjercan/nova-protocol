# Settings menu content: graphics quality, keybinds, audio

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.7.0,ui,menu,spike

Goal: fill the Settings placeholder panel with real content: visual quality
options (relates to task 20260525-133013, spawn-less visual mode), keybinds,
and audio volume. Deferred to backlog; the main menu task 20260711-180426
ships only an empty panel with a Back button.

Notes:
- Spike: tasks/20260711-180500/SPIKE.md
- Parent task: 20260711-174915


- 2026-07-13 (deliberate-radar spike 20260713-082207, decision D6): keybinds
  in settings should cover the radar-era bindings (radar hold/tap, raise,
  wheel section-cycle) and gamepad alternatives (press-toggle radar) - see
  task 20260710-231927 for the remap mechanics; this task owns the settings
  UI surface.

## v0.7.0 scope (20260716, spike tasks/20260716-122954/SPIKE.md)

Pulled into v0.7.0 (p45). Scope for this release: audio volume, graphics
quality preset (consumes the low-end spawn-less mode 20260525-133013, tuned
against the perf baseline 20260716-123551), and a READ-ONLY keybind reference.
Full remapping + hint icons stay backlog (20260710-231927). Plan:
docs/plans/20260716-v0.7.0-plan.md, strand 3.

- 2026-07-16 (v0.7.0 planning, user note): the settings panel should be
  reachable from the PAUSE menu too, not only the main menu - same modal,
  both entry points.
