# Damage cracks cost a material per section, and it halves the frame rate

- STATUS: OPEN
- PRIORITY: 95
- TAGS: v0.11.0,performance,render,ship

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
