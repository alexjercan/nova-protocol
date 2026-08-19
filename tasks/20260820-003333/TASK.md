# Damage cracks cost a material per section, and it halves the frame rate

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: archive,done

Epic: `20260818-220812`. **The single biggest measured win in the epic: ratio
0.52, a 2x frame-rate improvement on `wfc_arena` 4v4.**

## The cause, measured and dated

Every section mesh carries its own PRIVATE material asset. At 4v4:

```
crack_entities=2046  distinct_crack_materials=2046  crack_assets=2046
```

2,046 mesh entities, 2,046 distinct materials, one each. Instances only batch
WITHIN a bin, and every section is its own bin. Turn the plugin off and the same
9,866 mesh instances draw through 320 shared materials, and the frame HALVES.

Ablation table, paired design, quiet box, one fixed step per frame:

| arm | ratio |
|---|---|
| cracks off | **0.52** |
| cracks off + no cladding | 0.50 (cladding adds nothing on top) |
| hulls hidden, still simulated and colliding | 0.27 |
| hulls hidden, 1v1 instead of 4v4 | 0.24 |
| no cladding alone | 0.89 |
| 64x fewer pixels | 0.89 |
| no AI | 1.05 |
| bodies frozen | 0.98 |

Rows 3 and 4 together: hidden hulls cost 0.27 at 4v4 and 0.24 at 1v1, so **the
ship-count scaling disappears when nothing is drawn.** Every ship costs what it
costs by being DRAWN, not by AI, physics, colliders or health.

## Why plates are free, and the shape of the fix

`owning_section` (`crates/nova_ship/src/sections/damage_cracks.rs`) returns
`None` on a `SectionFixture`, so cladding and greebles keep their SHARED
material and batch. That is why removing 11,660 entities changed nothing. The
codebase already demonstrates the working path; sections are the only things
that opted out.

## The regression, and when

`damage_cracks.rs` arrived in `0ee9cbb0` on 2026-08-18, replacing
`damage_tint.rs`, which cloned per section too but with one gate that decides
this:

```rust
// damage_tint.rs at 0ee9cbb0^
Ok(Allegiance::Neutral) | Err(_) => continue,
```

Its module doc: "Neutral / unmarked bodies are never tinted." `SectionCracksPlugin`
has NO allegiance gate - zero references to `Allegiance` in the file.

**`wfc_ships` hulls have no allegiance**, so the gallery went from zero private
materials to one per section mesh. That is the owner's "feels slower than when
we initially added them", and the diff is the bisect.

## The fix - OWNER DECIDED

**Quantise damage into N shared buckets (start at 8).** A section snaps to the
nearest bucket and uses that bucket's SHARED material. Material count is capped
at N regardless of fleet size or how long a fight runs.

Owner: the signal a player needs is "that section looks wrong", not "that
section is 47% damaged". Presentation is negotiable; see the epic's "What may be
traded for frame rate".

Rejected: restoring the allegiance gate (reinstates "unaligned ships never show
damage", which is the defect `0ee9cbb0` set out to fix). Deferred: per-instance
data in a storage buffer so ONE material serves every damage level - correct,
multi-day, and only worth it if damage rendering were a main mechanic. It is not.

Lazy cloning (share until first damage) is subsumed: bucket 0 IS the shared
pristine material, so an undamaged fleet batches completely.

## Do NOT

- Do not touch physics, collision or damage PROPAGATION. Only how damage is
  DRAWN. The epic's trade rule is explicit and this task is the worked example
  of the presentation side of it.
- Do not delete the feature. `0ee9cbb0` shipped damage a body wears in its own
  geometry, and that stays - it just stops costing a material per section.

## Done when

- Distinct crack materials at 4v4 is bounded by N, not by section count.
  Measure it with the count instrument; the number is the proof.
- Before and after frame times on `wfc_arena` 4v4 AND `wfc_ships` at 3/11/17,
  same protocol, paired. Expect roughly 2x; report what actually happens.
- A screenshot of a battered hull at N=8, judged by eye, before this lands.
  Stepped cracks reading badly is the one way this fix fails.

## What was built

`SECTION_CRACK_BUCKETS = 8`. A section's [`DamageLevel`] snaps to the NEAREST
of eight steps and its meshes SWAP to the shared material for their
`(source material, bucket)` pair. `SectionCracksMaterials` is the registry that
owns every cracked material and the only place one is created; nothing is ever
written into a built material, so grading a fight touches no asset at all.

Three things the design did not say, all forced by the code:

1. **Buckets are LAZY, not eager.** A torpedo warhead's `StandardMaterial`
   carries the launched type's tint and is built per LAUNCH, so eager buckets
   would cost eight materials a shot. Only the bucket something actually
   reaches is built.
2. **The registry FORGETS.** It holds no strong handle to a source; the mesh's
   `SectionCracks` does. `forget_dead_sources` reads `AssetEvent` and drops a
   source's whole bucket set when the last mesh drawn from it is gone -
   otherwise the same per-launch tint would leak an entry per shot fired.
3. **The bucket is read at CAPTURE too**, not only in grading, so a gltf node
   that finishes loading onto an already-battered section is drawn right on its
   first frame.

Bucket 0 is the pristine step, so an undamaged fleet draws through one material
per source and pays what the effect being absent would pay.

## What was measured

Host: RTX 3060 Ti, vulkan, i9-12900F, NixOS, Xvfb 1280x720, `dev` profile,
quiet box, no other lane running. Protocol is `20260819-173219` phase B3's:
90 warm-up + 200 captured frames, one capture per PROCESS, probe-style profile
sandbox, `NOVA_PERF_MAX_DELTA=0.015625`, three passes, the legacy arm captured
immediately before every bucket arm. **Both arms are the same binary**: the
pre-bucket path was kept behind a throwaway `ABL_LEGACYCRACKS` env knob for the
measurement and removed before landing, so nothing between the two arms differs
except the code under test.

`fixed_steps min=1 max=1` in every loaded capture. The empty-gallery captures
read `min=0 max=1 mean=0.98-0.99`: at 16.9 ms a frame the scene is faster than
the 15.625 ms step, so some frames run none. That affects the FLOOR point only.

One honest gap between the measured binary and the landed one: the measured
bucket arm carried the knob's branch and a `OnceLock` read per section mesh per
frame, and took a hash entry where the landed code takes a lookup. Both are on
the bucket path, so the landed code can only be the same or marginally faster
than the number reported here.

### Distinct materials - the count is the proof

| subject | section meshes | as shipped | bucketed |
|---|--:|--:|--:|
| `wfc_arena` 4v4 | 2,046 | **2,046** | **288** |
| `wfc_ships` 11, pristine | 2,652 | 2,652 | 381 |
| `wfc_ships` 11, battered | 2,652 | 2,652 | **416** |
| `wfc_ships` 3 | 760 | 760 | 115 |
| `wfc_ships` 17 | 4,116 | 4,116 | 563 |

The battered row is the one that grades the BUCKETS: every section is put at a
different damage level, so all eight steps are occupied at once. It costs 35
materials over the pristine row - 416 against 381 - because the section meshes
are distributed very unevenly over their source materials. Most sources draw one
mesh and cannot split; a few draw hundreds and split into eight.

### Frame times, paired, three passes

| subject | as shipped | bucketed | ratio | spread |
|---|--:|--:|--:|--:|
| `wfc_ships` 0 (floor) | 16.94 | 16.72 | 0.995 | 0.978-0.999 |
| `wfc_ships` 3 | 65.69 | 41.24 | 0.636 | 0.596-0.674 |
| `wfc_ships` 11 | 129.26 | 75.81 | 0.587 | 0.583-0.598 |
| `wfc_ships` 17 | 205.44 | 100.44 | 0.489 | 0.486-0.496 |
| `wfc_ships` 11, battered | 132.06 | 80.28 | 0.589 | 0.582-0.609 |
| **`wfc_arena` 4v4** | **117.22** | **69.36** | **0.592** | 0.588-0.604 |

No spread straddles 1.00, and none is wider than 8% of its median.

Least squares over ships = 0, 3, 11, 17:

| line | floor | per ship | R^2 |
|---|--:|--:|--:|
| as shipped | 22.94 ms | **10.50 ms** | 0.987 |
| bucketed | 21.70 ms | **4.76 ms** | 0.985 |

**4.76 ms a ship against a shipped 10.50**, and the ablation's two reference
points reproduce on this tree to within 1.3% (10.37 shipped, 3.77 with the
effect removed outright). So buckets recover **5.74 of the 6.73 ms per ship,
85%** of what deleting the feature would buy, with the shader and the pipeline
still in the frame - slightly BETTER than the 4.95 ms the `ABL_SHARECRACKS` arm
predicted, and a battered fleet costs the same as a pristine one.

### The screenshot gate: N=8 PASSES

`tasks/20260820-003333/cracks-legacy-vs-buckets.png` is the same battered hull
at 4x, continuous on the left and eight buckets on the right;
`battered-hull-buckets.png` is the whole hull under buckets. Full-frame RMSE
between the two arms is 0.76%.

Judgement: nothing reads as stepped. The effect is spatial noise thresholded by
one scalar, so quantising the scalar shows slightly more or slightly less of the
same fracture network - there is no banding and no boundary to see. A handful of
sections read one step heavier than they did. On a hull whose sections were
given RANDOM damage levels - the worst case there is for visible stepping,
because neighbours land on different steps - the two frames are hard to tell
apart without flipping between them. **8 is enough; no more steps are needed.**

## What the design got wrong

**D5's arithmetic.** It read "32 distinct source materials" off the ablation's
count line and concluded 32 x 8 = 256 bins. The 32 are the materials still on a
`MeshMaterial3d<StandardMaterial>` - the PLATES. Section meshes draw through
their own set, which the ablation's own `c11` row already showed at 413, and
that set is **per ship**: 115 sources at 3 ships, 381 at 11, 563 at 17, about 35
a hull. So the bound is not a constant 256; it is "sources times buckets" and
sources still scale with the fleet.

The conclusion survives - measured 4.76 ms a ship - because the split is what
matters, not the constant, and because the split is uneven in the useful
direction. It does name the next thing to look at, though: **each hull
instantiates its own copies of the same ~35 section materials.** Sharing those
across hulls would take the 11-ship gallery from 381 bins to 35 without touching
the effect. That is a `WorldAssetRoot` / gltf instancing question, not a damage
question, and it is not in this task's lane.
