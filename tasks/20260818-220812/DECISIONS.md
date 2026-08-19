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
