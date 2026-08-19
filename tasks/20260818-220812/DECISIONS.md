# Decisions taken while driving, for review at the end

Owner asked for these to be RECORDED and presented when the run ends, rather
than stopping the run. Each says what was decided, why, and what would reverse
it.

## D1 - Damage cracks quantise to 8 shared buckets

**Decided by the owner**, recorded here because it sets the pattern. A section
snaps to the nearest of N buckets and uses that bucket's shared material, so
material count is capped at N instead of growing with the fleet.

Owner's reasoning: the signal a player needs is "that section looks wrong", not
"that section is 47% damaged". Damage rendering is not a main mechanic.

Rejected: restoring the old allegiance gate - it reinstates "unaligned ships
never show damage", the defect `0ee9cbb0` set out to fix. Deferred: per-instance
data in a storage buffer, correct but multi-day and only worth it if damage
rendering were a main mechanic.

**Reversed by**: stepped cracks reading badly in the screenshot gate.

## D2 - N = 8, provisionally

Not measured, chosen. 8 is small enough that the material count is irrelevant at
any fleet size and large enough that a section visibly degrades in stages.

**Reversed by**: the screenshot gate wanting more steps, or 8 still costing
enough to matter (it will not - 8 against 2,046).

## D3 - The 2x lands alone, the small fixes land batched

The metric resolves ~46% on the 4v4 and ~27% on `broadside`, so a 2% fix is
unmeasurable by itself and a 2x fix is unmistakable. Landing them together would
make it impossible to tell which worked.

**Reversed by**: the promoted harness resolving well enough that small fixes can
be graded individually.

## D4 - Ablation switches are feature-gated, not only env-gated

Env-only leaves a switch that disables collision inside a shipped binary,
reachable from a stray environment variable. Behind `dev`/`debug` the path is
absent from release.

**Reversed by**: a build-configuration reason that makes the feature gate
impractical - in which case say so rather than silently dropping to env-only.

## D5 - N = 8 VALIDATED, with the arithmetic that validates it

D2 chose 8 without a number. The ablation supplies it: the 11-ship gallery draws
through **32 distinct source materials**
(`tasks/20260819-173219/notes-ablation.md`).

What matters for batching is INSTANCES PER BIN, and a bin is keyed on the
material:

| scheme | bins | instances/bin | measured ms/ship |
|---|--:|--:|--:|
| as shipped, one material per mesh | 2,652 | **1** (worst case) | 10.37 |
| 8 buckets x 32 sources | 256 | ~10 | not yet measured |
| share by source material only | 32 | ~83 | 4.95 |
| material type removed outright | - | - | 3.77 |

So 8 buckets sits an order of magnitude closer to the shared case than to
today's, and should recover most of the measured 82%. The residual 1.18 ms per
ship is the extra material TYPE and its pipeline, which quantising cannot touch
and which is NOT worth chasing - removing it means removing the feature.

**Reversed by**: the fix measuring materially worse than 4.95 ms/ship, which
would mean bin count is not the axis after all.

## D6 - the fix lands BEFORE the clean pre-fix 4v4 number is taken

Re-sequences PLAN.md phase 0. `arena-window` was queued to produce a trustworthy
absolute 4v4 number on the current tree - but that number is obsolete the moment
a 2x lands, and the fix already has everything it needs to grade itself: the
ablation established the paired protocol AND both baselines (10.37 ms/ship as
shipped, 3.77 with the plugin off, on three ship counts).

So `arena-window` measures the tree AFTER the fix, where its number is the one
anybody will cite. `pd-stress` follows, unaffected either way.

**Reversed by**: the fix stalling, in which case the pre-fix number is worth
taking on its own.

## D7 - D5's arithmetic was WRONG, and the error names the next target

D5 read "32 distinct source materials" off the ablation and concluded eight
buckets would be a constant 256-bin bound. **Those 32 are the PLATE materials.**

Section meshes draw through their own set, and that set scales with the fleet:
**115 sources at 3 ships, 381 at 11, 563 at 17 - about 35 a hull.** So the bound
is `sources x buckets` and the sources are not constant.

The fix's conclusion survives, because what matters is bins-per-instance and the
split went the useful way: 4v4 measured 2,046 materials -> 288, and per-ship
cost 10.50 -> 4.76 ms, slightly BETTER than the 4.95 the share-by-source arm
predicted.

But the error names the next thing: **every hull instantiates its own copies of
the same ~35 section materials.** Sharing them would take the 11-ship gallery
from 381 bins to 35. That is a gltf / `WorldAssetRoot` instancing question, not
a damage question.

**How this happened**: I derived a bound from one number in someone else's notes
without checking what it counted. The measurement was right and the reading was
wrong - the same failure as reading `part()`'s seventh argument as mass.

## D8 - what a ship costs now, and what is left to take

| item | before | after | protected? |
|---|--:|--:|---|
| private per-section material | 6.60 | ~0.99 | no - fixed |
| `ThrusterExhaustMaterial` per-frame write | 1.54 | 1.54 | no |
| everything NOT drawn (physics, AI, health, colliders) | 2.00 | 2.00 | **YES** |
| rest of the render | 0.23 | 0.23 | no |
| **total per ship** | **10.37** | **4.76** | |

Two consequences worth stating before the next round picks a target:

1. **`ThrusterExhaustMaterial` is now the largest DRAWN cost per ship** - 1.54 of
   4.76 ms, 32%, up from 15%. It was Phase 3 batch filler; it is now the
   headline of the next fix round.
2. **"Everything not drawn" is 2.00 ms, 42% of a ship, and it is the protected
   half.** Physics, AI, health and colliders are what the owner ruled off the
   table. So per-ship cost has a floor around 2 ms that this epic may not go
   below without a gameplay decision.

Arithmetic that follows, and it should be said plainly rather than discovered at
the end: taking BOTH remaining drawn costs gives ~2.2 ms a ship. Eight ships is
then 17.8 ms on top of a **16.74 ms empty-scene floor** - about 34 ms, 29 FPS.
**The floor is now the dominant term and the epic's 60 FPS goal cannot be
reached without it.**
