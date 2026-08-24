# Make particle effects credible in vacuum

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.12.0,art,vfx

Rewritten 2026-08-24 for v0.12.0 with the round 4 inventory:
`tasks/20260815-231945/CONTENT-AND-ART.md` section 3. The torpedo baseline
this task used to scope is LANDED (88d7322a, b374c172): vacuum ejecta, no
blast sphere, punch pass accepted.

## Goal

Audit every player-visible effect and give each an explicit vacuum role -
brief flashes, incandescent ejecta, vapor, fragments, directional momentum;
no rolling fireballs, no gravity-driven smoke. Keep effects readable at
gameplay distance.

## The inventory (the audit's gift: it is SMALL)

Exactly three hanabi graphs exist, all built-in defaults - no shipped
content overrides any of them:

1. Torpedo blast (torpedo_section/render.rs:247-318) - already retuned for
   vacuum; the reference treatment.
2. Torpedo launch puff (render.rs:395-503) - cold propellant flash. NOTE:
   its default asset is minted PER BAY SPAWNER (render.rs:484-494) unlike
   the other two shared assets - record the cheap sharing fix here.
3. Turret muzzle flash (turret_section/render.rs:353-409) - 3-px
   screen-space dots.

Non-hanabi families the audit also owns: damage sparks
(damage_sparks.rs, threshold :38), damage cracks, damage plume, the shader
exhaust plume (thruster_section.rs:119-170), juice impact rings + shake
(nova_gameplay/juice.rs:134-141, shake.rs). The authored vocabulary is
`DamageEffect` (damage_effects.rs:59-74: Cracks, Sparks, Plume).

## Order of work

1. **Cross-cutting fixes first, one lane, all families**: the Hanabi
   extraction delay (first visible ejecta lands frames after the event) and
   the square billboards at close range. Both are written into the accepted
   torpedo baseline as deliberate refinement targets; fix them as common VFX
   direction, not per-family.
2. **Per-family vacuum roles**: for each family record (a) its stated role
   (flash / ejecta / vapor / fragments / momentum), (b) momentum correctness
   - the blast inherits NONE of the torpedo's velocity today; puff and
   muzzle carry `base_velocity` properties, (c) capacity and instance cost
   (shared asset vs per-instance buffer), (d) tier behavior at the spawn
   gate.
3. **Define the missing budgets BEFORE adding complexity**: burst
   concurrency and transient light budgets have NO machinery today -
   `GraphicsBudget.particles: bool` (nova_gameplay/settings.rs:188-236) is
   the only lever, capacities are per-family consts, and no effect owns a
   dynamic light. Nothing gets a light until the budget exists.

## Constraints

- Preserve authored effect overrides (blast_effect / launch_effect /
  muzzle_effect config fields) and WASM support - hanabi needs compute, the
  web build forces WebGPU (nova_core/Cargo.toml:28-37).
- Deterministic captures per family for isolated shots, impacts,
  destruction, and salvo load, before accepting each family. The pattern:
  examples/screenshots/loop_torpedo_blast.rs (scripted, seeded,
  re-capture reproduces frames).

## Done when

- Every shipped effect family has an explicit vacuum visual role recorded
  here.
- The two cross-cutting fixes are verified on every family they touch.
- Representative isolated and stress captures reviewed.
- Graphics tiers and concurrent-effect budgets are defined, measured, and
  documented.
- Player and creator documentation reflects any authored-effect contract
  changes.
