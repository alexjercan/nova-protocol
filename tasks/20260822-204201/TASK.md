# Make particle effects credible in vacuum

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog

## Goal

Audit every player-visible particle effect and replace atmospheric or generic
VFX language with a coherent vacuum treatment.

## Direction

- Use brief flashes, incandescent ejecta, vapor, fragments, and directional
  momentum instead of rolling fireballs or gravity-driven smoke.
- Keep effects readable at gameplay distance without drawing literal damage
  volumes.
- Define graphics-tier behavior, burst concurrency limits, GPU capacity, and
  transient light budgets before increasing visual complexity.
- Preserve authored effect overrides and WASM support.
- Compare deterministic captures for isolated shots, impacts, destruction, and
  salvo load before accepting each family.

## Immediate torpedo boundary

The current direct-on-master torpedo pass is intentionally small: remove the
orange blast-radius sphere and retune the existing shared Hanabi burst into a
brief white-hot-to-amber ejecta cloud. Its accepted punch pass stays inside the
same graph and budget: HDR colour for bloom, velocity-oriented radial streaks,
and a slightly faster front, with no extra particles or lifetime. Do not add
dynamic lights, custom shaders, new effect assets, or extra particle populations.
This task owns the later cross-effect art direction and refinement. The reviewed
landing and close-range captures accepted this particle-only punch as the
current baseline.

The baseline capture leaves two deliberate refinement targets: Hanabi extraction
puts the first visible ejecta a few frames after detonation, and close views
expose the particles as square billboards. Fix these as part of the common VFX
direction instead of adding a torpedo-only mesh or persistent effect instance.

## Done when

- Every shipped particle family has an explicit vacuum visual role.
- Representative isolated and stress captures have been reviewed.
- Graphics tiers and concurrent-effect costs are measured and documented.
- Player and creator documentation reflects any authored-effect contract changes.
