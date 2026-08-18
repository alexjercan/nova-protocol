# Carve fields must never cost a frame: cap, offload, bake

- STATUS: IN_PROGRESS
- PRIORITY: 95
- TAGS: v0.11.0, performance, bug, asteroid

Epic: `20260818-220812`. **Blocks play.** The `asteroid_field` sandbox is
unplayable at the current head.

## What happened

`0ee9cbb0` changed the carve grid from a FIXED 32 cells per axis to a count
DERIVED from rock size, capped at 64. That was right for the feature - a
world-fixed 0.5u cell is what lets a PDC round mark a big rock at all - and
wrong for the frame budget, because everything about a field is `count^3`.

The constant's own doc records the bill at 64^3, measured on one desktop core:

| | 32^3 (before) | 64^3 (now) |
| --- | --- | --- |
| seed | 2.3 ms | 12.7 ms |
| remesh | 1.6 ms | 10.7 ms |
| collider rebuild | 2.2 ms | 10.0 ms |

All three run SYNCHRONOUSLY in `carve_asteroid_fields`
(`crates/nova_scenario/src/objects/asteroid_carve.rs`). The sandbox
(`crates/nova_authoring/src/base_content/scenarios/sandbox/asteroid_field.rs`)
scatters 20 rocks at radius 1.0-3.0 plus a radius-20 gravity well, and every
one of them pays the seed once it is hit.

## The fix, in landable order

1. **Cap the real cost.** A per-axis cap of 64 is a cell cap of 262,144. Cap
   what is actually paid for, so no single rock can own a frame. Cheapest, and
   it lands first on its own.
2. **Offload.** Seed, remesh and collider build go to a worker; the previous
   mesh keeps drawing until the new one resolves. `meshed_volume` already
   tolerates a stale mesh, so the seam exists. Shares machinery with
   `PERF-OFFLOAD`.
3. **Seed at load** for rocks a scenario scatters, so the cost lands in the
   loading screen. Shares machinery with `PERF-BAKE`.

Surface-only meshing is the biggest win and is `PERF-SURFACE`, not this task.

## Also suspect, not yet measured

- `field_from_mesh` / solidify on a SECTION's first hit
  (`nova_ship/sections/damage_carve.rs`, `mesh/solidify.rs`): 10-16 ms per
  mesh, several sections in one frame. Matches "lag spike when a ship takes
  damage", reported earlier and never diagnosed.
- Carve shard population: every accepted mark emits 2+ shards on a 2.5 s
  lifetime; one turret at 100 rounds/s sustains ~500 entities. From the epic
  review's performance follow-ups (`tasks/20260813-224826/REVIEW.md`).
- Permanent section traversal: a non-empty root mark list makes every
  `DamageCarve` section walk its descendant art every frame. Same review.

## Done when

- Sandbox and a `wfc_arena` fight both hold frame rate, MEASURED, worst frame
  and top system self-time, before and after.
- `carve_asteroids` still shows carving. A fix that makes rocks unshootable is
  not a fix.
- `CHANGELOG.md` Performance entry.

## In flight

Branch `perf-sandbox` (sprout), forked at `0ee9cbb0`.
