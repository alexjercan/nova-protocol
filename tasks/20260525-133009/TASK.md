# Minimal example for nova_gameplay crate

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.4.0, example

Shows a ship with sections, health, one weapon. [new]

## Approach

`03_scenario` already exists, but it loads a ready-made named scenario (`asteroid_field`) from
`nova_assets` - the ship assembly is hidden in the catalog and it is wrapped in objective/variable
scripting, so it reads as a `nova_scenario` showcase, not a "here is how nova_gameplay works" one.
Rather than overload `03`, add a focused example that builds the ship *inline* from the section
catalog and demonstrates the core mechanics end to end.

## Steps

- [x] Add `examples/10_gameplay.rs`: a player ship assembled inline from one of every structural
      section (controller, hull, thruster, turret), plus a few asteroids as targets.
- [x] Demonstrate the three headline mechanics: sections (the inline `SpaceshipConfig`), health
      (a throttled readout of the ship's aggregate health and asteroids remaining), and a weapon
      (the turret auto-aims at the nearest asteroid and fires, running shoot -> damage -> destroy).
- [x] Wire the autopilot + screenshot harness; register `10_gameplay` in `examples_smoke`.
- [x] Green: headless run (ship assembles, turret clears 3 -> 0 asteroids, ship health steady at
      500/500, cycle complete no panic); `cargo clippy --workspace --all-targets`; `cargo test
      --workspace` (examples_smoke runs 10_gameplay).

## Resolution

`examples/10_gameplay.rs` builds a minimal player ship inline (controller + hull + thruster +
turret = 500 aggregate health), spawns three low-health asteroids in the turret's arc, auto-aims
the turret at the nearest one, and fires under the autopilot. The throttled readout
(`gameplay: N asteroids left, ship health X/Y, M bullets in flight`) makes the section/health/
weapon mechanics legible headless; the headless run shows the turret clearing 3 -> 0 asteroids.
Registered in `examples_smoke` so it runs in CI.

## Notes

Diagnosed and worked around a spawn pitfall: an asteroid's collider is a good deal larger than
its nominal `radius`, so targets spawned ~13 units apart shoved each other apart and self-
destructed (impact damage ~250) before the turret fired - the low-health targets exposed what
`08`'s 2000-health gates mask. Spreading the asteroids ~30 units apart keeps them independent
until the weapon clears them. (The oversized-collider-vs-radius mismatch itself is arguably a
separate asteroid bug worth a follow-up.)
