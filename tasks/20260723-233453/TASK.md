# Perf: ch5 raid drops to ~40 FPS - profile and optimize the many-entity scene (big ship + 6 fighters + base + scatter)

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog, performance, gameplay

## Story

Playtest of the ch5 raid (`webmods/the-ledger/ledger_ch5_the_raid.content.ron`,
task 20260723-182855) dropped to ~40 FPS. It is the heaviest scene in the game:
a 42-section player gunship, 2 wingmen + 4 fighters (~19 sections each), a
~13-section base station, ~34 scattered asteroids and 3 planetoids - so a LOT of
section entities, colliders, turrets and projectiles at once. Profile it and
find where the frame time goes.

## Notes / pointers

- Reproduce/measure with the `nova_probe` run-harness (the `/probe` skill) - it
  produces an fps report + a top-N systems profile. Get a real before-number for
  the ch5-scale scene, not a guess. (ch5 is a webmod; if the probe cannot drive a
  webmod directly, build an equivalent-density base scenario or an example that
  spawns the same entity count.)
- Likely suspects to check with the profiler first (do not pre-optimize): per-
  section physics/collider cost, turret aim/fire systems scaling with ship count,
  projectile volume, damage-tint / HUD systems over many entities, gravity
  affected-body queries. Let the top-N table point, then fix the real hotspot.
- Relevant crates: `crates/nova_gameplay/src/sections/`, `.../input/ai.rs`
  (turret aim + engage per ship), the flight/physics integration, and
  `crates/nova_probe/` for the harness.
- This is engine-wide, not ch5-specific - ch5 is just the worst-case repro. A win
  here helps every busy scene.

## Definition of Done (sketch - refine when picked up)

- A probe report quantifies the ch5-scale frame time before + after, with a
  measurable improvement (target: hold 60 FPS on the ch5 scene, or a documented
  honest limit).
- The fix targets the profiler's actual hotspot (evidence recorded), not a guess.
