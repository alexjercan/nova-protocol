# Destroyed things detach and tumble; delete the slicer

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: archive,performance,destruction,ship

Epic: `20260818-220812`.

Owner decision, 2026-08-18: **fly-off, and simplify.** Verbatim principle -
"unplayable but pretty is unplayable". Performance beats fidelity wherever the
two disagree, and the simpler mechanism wins where they do not.

## The one rule

**Nothing computes geometry when it dies. It detaches.**

A destroyed thing stops being part of its parent, becomes its own rigid body,
keeps the mesh it already had, tumbles, and despawns on a timer. Sections,
shells and greebles all obey the same rule - one mechanism, no per-kind path.

Asteroids are NOT in scope: they already pulverize through the carve field and
throw severed islands, with no health and no slicer
(`asteroid_carve.rs`). Do not touch that path.

## Why - the slicer is slow AND wrong

`explode_mesh` (`crates/nova_gameplay/src/mesh/explode.rs:214`) is 33.1 ms and
the largest single item in the measured death frame. It is also broken in three
ways at once, which is the "weird stuff on section destroyed" the owner sees:

```rust
match mesh_builder.slice(plane_normal, Vec3::ZERO) {
```

1. **Every plane passes through `Vec3::ZERO`**, at every recursion level. After
   the first cut a fragment sitting off to one side is cut by a plane through a
   point outside itself. It either misses - carried forward intact, direction
   `Vec3::ZERO`, so it flies nowhere - or shears off a sliver.
2. **Un-split lumps and motionless pieces** follow directly from (1).
3. **`rand::rng()`**, the thread RNG, not seeded `bevy_rand`. Section death is
   NONDETERMINISTIC today. The owner has ruled against nondeterminism.

Fixing the slicer would cost the same 33 ms. Deleting it costs nothing and
removes all three defects by construction.

## The deletion this unlocks

`base_section.rs:378` is the ONLY place `ExplodableEntity` is inserted, and
asteroids explicitly opt out of the health finale. Ship sections are the
slicer's sole consumer.

Take slicing off the section path and `crates/nova_gameplay/src/mesh/explode.rs`
and `crates/nova_gameplay/src/mesh/slice.rs` are both dead. Delete them. Delete
their tests. NEVER BACKWARD COMPATIBLE - do not leave them behind a flag.

`slice.rs` currently carries a module doc arguing it was deliberately kept by
the erosion epic. That argument is now superseded and the doc goes with the
file.

## What each kind does

**Sections.** On destruction: detach from the ship, become a dynamic body with
the collider it already has, take an impulse and a spin, despawn on the
existing fragment lifetime. Whole - no fragmentation.

**Shells and greebles.** Same, and nearly free already: `skin_decor.rs` parents
greebles to plates and plates to sections, and a greeble already carries a
CUBOID collider rather than its model. Reparent, add `RigidBody`, add
`TempEntity`. A destroyed section takes its plates and greebles with it, so
decide deliberately whether they detach INDIVIDUALLY (more clutter, more
bodies) or ride the section down (cheaper). Measure before choosing; default to
riding it down.

**Hit feedback** stays as it is: sparks on metal, spew on rock.

## Determinism

Whatever impulse and spin are randomized from, use SEEDED `bevy_rand`, not
`rand::rng()`. Same death, same tumble. This is a hard requirement, not a
preference.

## Explicitly deferred - do NOT build

Baked per-section fragment sets, spawned as pre-made pieces. Discussed and
deliberately not scheduled: fly-off is the simplification the owner asked for,
and baking is a fidelity upgrade that only makes sense once frame rate is a
solved problem. If it is ever picked up:

- ONE fragmentation per prototype, not K variants. Randomize impulse and spin
  instead - tumble variation carries the visual read at zero geometry cost.
- Bake into MEMORY at load, not as shipped assets: no asset pipeline change, no
  wasm download cost, reversible. A deterministic load-time bake is just as
  reviewable through a gallery example.
- The baker must cut through each fragment's OWN centroid. Reusing
  `explode_mesh`'s origin-plane algorithm would bake the current bug in
  permanently, which is worse than the bug.
- Fly-off remains the fallback for anything with no baked set (mod sections,
  failed bakes). Baked pieces would be of the PRISTINE section, so a section
  shot to pieces still shatters clean - an accepted cost, recorded here so it
  is not rediscovered as a defect.

## Done when

- No geometry is computed on any death path. Grep proves the slicer is gone.
- `mesh/explode.rs` and `mesh/slice.rs` deleted, with their tests and doc.
- Section death is deterministic: same seed, same tumble, asserted by a test.
- `destruction_finale` green. Its "no fallback cube burst" assertion must still
  hold, and its meaning is now stronger: there is no fragmenter to fail.
- Death-frame cost measured before and after. The 33.1 ms slicer item should be
  absent, not smaller.
- RUN a ship death and LOOK at it. The read should be a ship coming apart, not
  a ship dissolving. If whole sections tumbling reads as disassembly rather than
  destruction, say so with a capture - that is the known risk of this design and
  the owner accepted it on performance grounds, but it should be reported
  honestly rather than quietly tolerated.
- `CHANGELOG.md` entries: Performance, and Fixes for the three slicer defects.
