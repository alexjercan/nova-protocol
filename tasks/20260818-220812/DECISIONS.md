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

## D9 - the floor is promoted above the phase 3 batch

Recorded 2026-08-20, on the owner's own post-fix hand-run:
`tasks/20260819-173219/notes-owner-handrun.md`. 17 ships went 7 -> 16 FPS, and
ONE ship reads 36-48 FPS.

The plan ranked the ~17-21 ms floor in phase 4, behind a phase 3 batch of items
worth 2-10% each. That ranking was set when a ship cost 10.37 ms and the floor
was 13% of an 11-ship frame. It is now the largest single term at every ship
count the owner plays: 75-90% of a single-ship frame, and 34% at seventeen.

**So the floor moves ahead of the phase 3 batch.** Phase 3 keeps
`ThrusterExhaustMaterial` - it is now 32% of what a ship costs and is the same
defect class as the cracks - but the dressing-geometry item goes behind the
floor.

Reversed by: a floor investigation that finds the 21.5 ms is irreducible - a
driver, a present-mode or a vsync artefact rather than work. In that case the
phase 3 order stands unchanged and the epic's FPS target needs restating
instead.

## D10 - 60 FPS at 4v4 is out of reach for this epic's rules, 30 is not

The bound, from measured parts: eight fighting hulls cost 8 x 2.00 ms of
NON-drawn work - physics, colliders, AI, health, integrity - which the epic's
"physics and gameplay logic are not negotiable" rule protects. That is 16.0 ms
before the floor and before a single triangle. The 60 FPS budget is 16.67 ms.

So even deleting the renderer and the floor entirely leaves ~62 FPS, and any
real scene is under it. **60 FPS at 4v4 requires touching the protected half.**

Not asking, per rule 6 - this is a target question, not a change to what the
game does. Presented at the end of the run. The reachable target on presentation
work alone is ~30 FPS at 4v4, needing the floor halved AND per-ship from 4.76 to
3.0; both are named work, neither is speculative.

Reversed by: the owner restating the target, or a decision that a round is not
a physics body (which phase 4 already flags as needing the owner).

## D11 - the target is 1v1 at 60 FPS, and it REPLACES the 4v4 target

Owner, 2026-08-20: "1v1 is at ~30 FPS, I think if we can get 1v1 to 60 FPS
that's a big win." This supersedes D10's framing - the question is no longer
whether 4v4 reaches 60, it is what 1v1 needs.

**It is a much better target, and not only because it is smaller.** The epic
protects non-drawn work - physics, colliders, AI, health, integrity - at a
measured 2.00 ms a ship. At 4v4 that is 16.0 ms of the 16.67 ms budget spent
before a triangle is drawn, so the target was unreachable by construction. At
1v1 it is **4.00 ms**, leaving 12.67 ms of real room. The protected half stops
being the binding constraint.

### The budget

| item | today | needed for 60 FPS |
|---|--:|--:|
| floor (measured empty scene) | 16.74 | **<= 12.2** |
| 2 x protected non-drawn | 4.00 | 4.00 (fixed) |
| 2 x drawn | 5.52 | 0.46 (phase 3 batch) |
| **total** | **26.3 ms (38 FPS)** | **16.67 ms** |

Two things follow, and the first is the whole finding:

1. **The floor ALONE is 100.4% of a 60 FPS frame.** 16.74 ms against a 16.67 ms
   budget. No ship-side work of any size reaches 60 while that stands, so the
   floor is not merely the largest term - it is a hard precondition on the
   owner's stated target.
2. **With the floor cut and the phase 3 batch taken, 1v1 at 60 FPS is
   reachable.** The floor needs to reach ~12.2 ms, a 27% cut. That is a far
   more modest ask than the 4v4 case, which needed the protected half.

### Vsync makes it a cliff, so aim lower than the budget

The game ships `PresentMode::AutoVsync` (`nova_core/src/lib.rs:376`); the probe
forces `AutoNoVsync` (`nova_probe/src/capabilities/frametime.rs:586`). So every
harness number is uncapped work and every hand-read number is quantised to
refresh intervals.

**There is no partial credit at 60 Hz**: 16.6 ms of work reads 60 FPS, 16.8 ms
reads 30. So the working target is **~14 ms**, not 16.67, which puts the floor
at **<= 9.5 ms** - a 43% cut. Use the harness number, not the FPS counter, to
judge a change near the boundary; the counter cannot resolve 21 ms from 27 ms.

Reversed by: the floor investigation finding the 16.74 ms is irreducible, in
which case 60 FPS is off the table at any ship count and the target restates.

## D12 - RETRACTED: there is no floor. It was Xvfb.

`tasks/20260819-173219/notes-floor.md`, landed 2026-08-20. This retracts D9's
re-rank and D11's central arithmetic, and it invalidates every ABSOLUTE
millisecond measured through `xvfb-run` in this epic.

**The 16.74 ms "empty scene" is 13.7 ms of Xvfb and 3.0 ms of game.** Same
binary, same 1280x720 window, same fixed-step pin, same `Immediate`
presentation, `DISPLAY=:0` instead of `xvfb-run`: **3.02 ms, 331 FPS**, over
three repeats reading 3.01-3.03.

The mechanism is not subtle in hindsight. `submit+present` on the empty scene
is linear in WINDOW pixels - 1.45 ms at 160x90, 5.05 at 640x360, 11.45 at 720p,
50.35 at 1440p - and the discriminator settles it: rendering at 320x180 into a
720p window leaves the block at 11.17 vs 11.45 ms. **One sixteenth the shading,
block unmoved.** A software X server has no scanout, so presenting is a CPU copy
of every pixel in the window. We were timing the display server.

### What survives and what does not

- **SURVIVES: every ratio.** The crack-bucket fix's 0.592, the per-ship 2.9x,
  every interleaved arm in `notes-ablation.md`. Xvfb's cost is ADDITIVE and
  per-pixel, so a paired ratio divides it out.
- **DIES: every absolute.** The 16.74 ms floor, the 21.5 ms fitted intercept as
  a SCENE cost, D8's per-ship milliseconds, D10's and D11's budget tables,
  and D11's "the floor alone is 100.4% of a 60 FPS budget".
- **SURVIVES, and this is the check that should have caught it earlier: the
  owner's own hand-runs.** They were always on a real display. One hull reads
  36-48 FPS by hand and 36 FPS on the lane's real display; 1v1 reads ~30 FPS by
  hand and 29 FPS on the lane's. **Two instruments, independently, agree.** The
  owner's numbers were right the whole time and the harness was the wrong one.

### The fitted intercept was real - it just was not the scene

Both fits put ~20 ms of cost at zero ships (21.2 from the owner's 3/11/17 row,
19.4 from the lane's 1/2/3 row). The empty scene is 3.02 ms. Those are
consistent, because the intercept is not an empty-scene floor: it is a fixed
cost that appears the moment there is ONE hull, and a straight line through
ship counts >= 1 reports it at x = 0.

So the term is real, it is worth ~16 ms, and it now has a name.

### The actual lead

**`Prepare` + `PrepareMeshes` is 16.1 ms of a 26 ms one-hull frame.** CPU, in
the render world, building per-instance buffers and bind groups over 986 mesh
instances. It is PRESENTATION under the epic's rule, so it is takeable without
asking.

Real display, frozen gallery: empty 3.02, one hull 27.60 (36 FPS), two 34.82
(29 FPS), three 44.04. Note it is SUBLINEAR - the first hull costs 24.6 ms and
each further hull about 8. Why the first is 3x the marginal is the open
question, and the answer probably names the fix.

**1v1 misses 60 FPS by 18 ms, and 100% of that is per-ship.** The target stands;
only the route changed.

### How this happened

I inherited "never measure on a real display" as a harness convention and never
asked what the harness itself cost. Then I derived a budget from it, wrote the
budget into the release's definition of done, and re-ranked the board around it
- twice - without once checking the instrument against the owner's own readings,
which disagreed and were sitting in the same task folder. **The cross-check was
available the entire time and it was free.**

Same failure as D7 and as the section-density scare: a number taken from the
apparatus and reasoned from, rather than validated against an independent
measurement first.

**Reversed by**: nothing. The real-display figure is confirmed by three repeats
AND by an independent instrument (the owner's hand-runs) agreeing to within a
frame at two separate ship counts.
