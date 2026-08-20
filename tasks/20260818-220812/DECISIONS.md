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

## D13 - the desk is shared, the floor anomaly has a real cause, and Prepare was mis-attributed

`tasks/20260819-173219/notes-prepare.md`, landed `d15479b5`.

### Captures may run on a hidden workspace

**H/V median 0.984-0.993 on `min_ms` over 14 paired captures**, every spread
straddling 1.00. i3 unmapping a hidden workspace does not change what a capture
measures, so measurement and the owner's desk can share one machine.

**Only after a fix, though.** Before it, hidden read **16.66/16.68 ms at
`mean_fps` 60.0** against 3.37 ms visible. `WinitSettings::game()` sets
`unfocused_mode: reactive_low_power(1/60)` - and the capture's own comment
claimed it "keeps the loop running flat out even when the window is unfocused".
The comment was the opposite of the behaviour.

### That is `notes-floor.md`'s anomaly, and the diagnosis there was wrong

The floor lane read one reference at 16.67 ms and blamed "a 60 Hz composited
path". **There is no compositor on this host and the output is 165 Hz**, so
60 Hz was never its refresh. Reproduced on demand, same signature to two
decimals. Corrected in that file.

Two wrong environment diagnoses in two days, both landing on a plausible number.
Hence three refusals rather than three warnings: `ABORT_WINDOW_SIZE`,
`ABORT_UPDATE_THROTTLED`, `ABORT_REFRESH_CAPPED`. The last is calibrated against
measured `fifo` 0.76-0.79 against `immediate` 0.03-0.44 - the first threshold
guessed was wrong and a genuinely capped window passed it.

### The gallery figures SURVIVE

Re-measured from scratch, interleaved, 3 passes: **3.45 / 28.41 / 40.71 / 43.33
ms** against the recorded 3.02 / 27.60 / 34.82 / 44.04. **D11, D12 and the
release DoD need no retraction.** The `IdleOrbit` defect was real and cost less
than this box's noise.

The window-size defect touches a LABEL, not a number: real-display captures ran
at the WM's geometry, so `notes-floor.md`'s "1.10x the window" was 960x1057.
Fill binds nothing there, so the figures stand.

### `PrepareMeshes` is not a lever, and I said it was

The brief I wrote said "`Prepare` + `PrepareMeshes` is 16.1 ms". `PrepareAssets`
chains outside the top-level chain, so its time landed in a neighbour. Corrected:
**`Prepare` 13.66, `PrepareAssets` 4.30, `PrepareMeshes` 0.30** - 1.3%, noise.

### The law worth keeping: a hull costs what it INTRODUCES

**The frame tracks DISTINCT MESH ASSETS, not instances.** R^2 0.996 against
0.910; marginal per-instance cost drifts 4.9x across the sweep while
per-distinct-mesh drifts 1.58x.

So drawing the same ship twice is nearly free and drawing two different ships is
not. This is the same shape as the crack-bucket win - that was distinct
MATERIALS - and it re-aims everything after it. D7's "every hull instantiates its
own ~35 section materials" is now the ranked item it predicted it would be.

### Landed and ranked

1. **DONE**: `thruster_shader_update_system` called `Assets::get_mut`
   unconditionally, re-uploading every drive material every frame.
   `PrepareAssets` **0.175x**, least-contended one-hull frame **0.733x**.
2. Fewer distinct MESHES per hull - 0.170 ms each per frame, 120 at one hull.
3. Fewer distinct MATERIALS - 0.285 ms each, 74 at one hull.
4. `Shadow` costs ~0.64 ms CPU with NO `render/shadows` GPU pass - correcting the
   floor lane's "shadow maps: 0.000 ms" ruling, which read the GPU side only.

**Standing caveat**: on this box a whole run lands 2-3x from the one before it
with every phase scaling together. Quote `min_ms` and phase shares. A mean whose
spread straddles 1.00 measured nothing.

## D14 - the projectile broad phase is REJECTED, on the case built to isolate it

`tasks/20260819-173219/notes-pd-stress.md`, landed `4871514d`.

The plan's phase 4 named the PD case's cost and said: "**If this is the
projectile broad phase, the honest fix is 'a round should not be a physics body'
- which is gameplay logic and needs the owner.** Do not decide it in this run."

**It is not the broad phase.** Traced over 110 frames of the saturated hold at
26.06 ms, with 2,256 colliders of which 91% are rounds:

| ms/f | share | system |
|--:|--:|---|
| 9.285 | 35.6% | `prepare_erased_assets<ExtendedMaterial<StandardMaterial, ThrusterExhaustMaterial>>` |
| 6.327 | 24.3% | `prepare_material_bind_groups` |
| 5.668 | 21.7% | `bevy_ui::widget::text::text_system` |
| 5.173 | 19.8% | the whole avian physics step |
| **0.192** | **0.7%** | **`collect_collision_pairs<ProjectileHooks>`** |

The entire targeting and point-defence chain sums under 0.2 ms. **So the owner
never has to answer the gameplay question this epic was saving for them**, and
the phase 4 item is closed rather than escalated.

### The sweep says it scales with BAYS and is flat in MOUNTS

Mounts 4 -> 8 -> 12 takes the BVH from 859 to 2,309 colliders and `min_ms` goes
18.06 -> 18.00 -> **17.09**. Three times the bodies and the frame gets FASTER.

Bays 4 -> 12 -> 24 is a straight line: **4.52 ms + 1.12 ms per bay**. A bay puts
about 9.2 torpedoes in the sky, so **0.12 ms a frame per live torpedo**.

Census says why: `Torpedo Controller` is 110 instances, **1 mesh, 110 distinct
materials**, plus 406 exhaust instances each carrying its own extended material.

### This is the fourth sighting of one bug

Cracks per section, plume rewritten per frame, placeholder art per entity, and
now one exhaust material and one cracks material per LIVE TORPEDO. The
distinct-asset law holds on a scene nothing like the frozen gallery it was
derived on.

Items 1 and 2 of the cut list are both that defect and both presentation, so
both are takeable without asking. Item 4 - "a round is an entity with a collider
and rigid body", command flush 1.571 ms, 6% - stays GAMEPLAY, is sized rather
than decided, and **buys nothing until 1 and 2 land**.

### The instrument, and the bug in it

Pinned, the case resolves **6%** against the 4v4's 46% and `broadside`'s 27% -
4.5x better than the best subject in the suite, and one capture beats eight of
either. Free-running it resolves 39%, because one capture in eight sits in the
fixed-step clamp with p99 179.7 ms while its mean and median stay inside the 20%
band. **The validity gate cannot see a spiral that does not move the middle.**

The range itself was NOT green: its rounds floor was drawn from a contended
3.4 fps reading, and free-running the yield spans 708-2,425 because the trigger
is decided in `Update` and the rounds are spent in `FixedUpdate`. It failed
about two runs in five, **including the probe clean pass CI runs**. Floor
redrawn under the worst measured free-running yield.

### The owner's view-dependence observation did NOT survive measurement

Owner flew the case and reported the frame rate changing with where the camera
pointed. Measured with a new `NOVA_STRESS_PD_VIEW` knob: every paired ratio
straddles 1.00 at 720p AND at 2560x1440, and `Render/graph` reads 4.203 / 4.006
/ 4.196 ms for battery / lanes / away.

**Emptying the frustum does not move the draw phase, because the cost is asset
PREPARATION and culling never touches it.** The likely thing they felt is the
clamp spiral above - it is real, it is a 3.4x swing, and it is not the camera.
`notes-ablation.md`'s "pixels do not bind" therefore survives on this case too;
640x360 to 1920x1080 costs 5%.

### A harness footgun worth knowing

`probe run`'s profiled pass leaves a `debug,trace` binary at the same path, so
the NEXT hand-driven run is silently a traced run. It cost a whole seven-capture
sweep, caught only because it wrote 400-900 MB `trace-*.json` files into the
repo root.

## D15 - the torpedo material lane: what it fixed, and the two it left

Landed `d4670f32`. This is the FIFTH and SIXTH sighting of the same bug, and
the first time the fix had to be two fixes for one symptom.

### One root cause did not explain both items

The warhead material was minted per LAUNCH from `TorpedoType::tint` - a
per-instance asset carrying a per-TYPE value, which is a plain mistake. Because
`SectionCracksMaterials` keys buckets on the source `AssetId`, a private source
could never collapse, so every torpedo also minted its own crack buckets as
point defence chewed on it. `forget_dead_sources` existed only to stop that
being a leak rather than a cost.

**But fixing the root collapsed ONE item.** The exhaust plume is private for a
different reason: its material carries a per-frame VALUE - the throttle - not an
identity. So the `8a26ae31` read-before-write guard, which covers a parked
fleet, covers a guided torpedo NOT AT ALL: its thrust genuinely moves every
frame. Two defects wearing one symptom. The plume needed quantising
(`EXHAUST_PLUME_BUCKETS = 16`, swap never write), which is D1's pattern from
`6b3bfc87` reapplied.

### The result, and the honest part of it

Traced, saturated: `prepare_erased_assets<...ThrusterExhaustMaterial>` 8.576 ms
-> 0.008, `prepare_material_bind_groups` 5.804 -> 0.262. **64.0% of the frame to
1.8%.** Paired clean pass: `min_ms` 17.83 -> 5.47, ratio **0.304**. Census 105
distinct drawn materials -> 17.

The frame TOTAL only moves to 0.785x, and the reason matters more than the
number: **the render world stopped being the pacer** - 98.0% of the traced frame
down to 48.5%. What is left is the main world. The fix also made the main world
MORE expensive, because a faster `Update` feeds the point-defence chain harder:
the fix arm carries 2365 rounds / 2565 colliders against the base arm's 2036 /
2239, about 15% more work. The 0.785 is conservative by that much, and **the
next candidate on this range is the simulation, not the renderer** - which is
phase 6, arrived at from the other direction.

`Render/graph` 4.01 -> 4.31 straddles 1.00. That is the control: nothing left
the picture.

### The visual gate, and how it was argued

RMSE was measured against the run-to-run FLOOR rather than reported as a bare
number: within-base 0.268-1.025%, within-fix 0.311-0.473%, cross-arm
0.400-0.967%. Every cross-arm pair lands INSIDE the unchanged binary's own
spread. The decisive argument is not statistical though - it is in the shader:
one bucket step moves the flame tip by 0.0667 local units and
`thruster_exhaust.wgsl:56` already jitters that same tip by up to 0.1 every
frame. **The quantiser is finer than the wobble already applied on top of it.**

Also learned: `wiki-combat-aftermath.png` is USELESS as a visual gate. Two base
runs differ by 13.78% because the debris scatter is unseeded. A gate that
cannot reproduce itself measures nothing.

### Two more sightings, recorded and NOT fixed

Both are the same defect in asset types the census cannot see:

- `insert_blast_radius_visual` mints one `StandardMaterial` per detonation and
  writes it every frame. Bounded by a 0.4 s lifetime - peak 3 concurrent on this
  case - so it is real but small.
- `insert_particle_effect` builds a fresh 32768-particle `EffectAsset` FROM
  LITERALS per detonation. This is the `cbc86980` placeholder-art defect in an
  asset type nothing counts, and nothing counts it precisely because it is not a
  mesh or a material.

Turret rounds were checked and are CLEAN: 1349 instances, 1 mesh, 1 material.
`DefaultProjectileRender` remains the exemplary pattern.

**The instrument gap is the finding here, not the two sites.** Five of six
sightings were found because a census counted the asset type; these two were
found by reading code, which does not scale. The census learned
`plume_material_assets` in `362caf96`; it still cannot see `EffectAsset` or a
per-detonation `StandardMaterial`.

## D16 - point defence is frame-rate invariant now, and 2.4x stronger at 60 fps

Landed `22991fef` (merged). The owner authorised the fix explicitly. The
STRENGTH change below is a consequence I did not retune and am not deciding.

### My hypothesis was wrong, and the shape of the error is worth keeping

I said the split was the TRIGGER: decided in `Update`, rounds spent in
`FixedUpdate`. That was measured and it is not the cause. The cadence timer
already ticked on the fixed clock, so the spawn rate was invariant GIVEN AN OPEN
GATE. Implementing my version took the spread 7.4x -> 4.0x and stopped: it
lifted the slow end and left the fast end alone.

The real cause is the BARREL'S POSE. Everything that decides whether a mount
bears - intercept solve, hinge demand, `SmoothLookRotation` output, joint
`Transform` - advanced once per FRAME. The barrel's angle was a staircase whose
step size was the frame period, its tracking residual scaled with that, and
`TURRET_ON_TARGET_RAD` is a 0.92 deg cone sitting INSIDE the band the residual
sweeps. So a fixed-step branch sampled a render-rate staircase.

Dead linear on the measurement: trigger duty 0.097 at 20 fps against 0.606 at
106 fps, same scene, same seed.

**The generalisable rule** - now in `docs/architecture.md`: put a system on the
fixed clock when what it computes DECIDES a fixed-step consequence, EVEN WHEN
THE SAME VALUE IS ALSO DRAWN. This one is easy to get wrong precisely because
the aim chain has an on-screen output, so it reads as belonging beside the
camera and the HUD. The tell is not what the value looks like. It is whether
anything on the fixed clock branches on it.

### The evidence, which is a COUNT as required

- **Rounds yield 2,204-2,437 free-running across 20-503 fps (1.11x).** Was
  708-2,425 (3.4x). Pinned was 2,236-2,401. **That is the pinned spread, without
  the pin** - which is the strongest form this claim could take.
- **Rendered and headless converged**: envelope fill 7.2 s against 7.1 s (1.4%),
  peak rounds 2,455 against 2,411 (1.8%), across a 13x frame-rate gap. Was 9.5 s
  against 86.3 s.
- Raw intercept tallies are NOT comparable across transports and the earlier
  64-against-793 figure was partly an artefact: `HOLD_FRAMES` is a frame count,
  so the window itself scales with frame rate. The invariant reading is
  intercepts at 90 s: 824-845 (2.5%).

### THE OWNER'S CALL: point defence got stronger

Duty 0.353 -> 0.814, rounds per step 6.37 -> 15.14, mean aim error 11.3 -> 3.6
deg, all at 60 fps. **Roughly 2.4x more effective, and nothing was retuned.**

This is not separable from the fix. The lane tried a strength-neutral ordering
that keeps one tick of staleness; it measured identically. The gain comes from
removing several frames of ACCUMULATED latency through the chain, not from the
last hop, so there is no ordering that buys invariance and leaves strength
alone.

What that means in the game: a 60 fps player's battery now behaves the way a
fast machine's always did. The old behaviour was not a balance point anybody
chose - it was whatever the host produced. So the honest framing is not "point
defence was buffed", it is "point defence stopped being a function of your
frame rate, and the value it settled on is the fast one".

**If it is too strong, the levers are CONTENT, not the schedule**:
`ROUNDS_PER_MOUNT` (left at 40; the lane proposes ~120/mount and did not apply
it) and `TURRET_ON_TARGET_RAD`. Recorded for the owner rather than actioned.

One forced range change: a working battery outguns twelve bays, so the
saturation gate became unreachable at any frame rate. `INBOUND_PER_BAY` 6 -> 4,
sized under the measured 5.0-5.6 per bay settle. That is the instrument, not the
game.

### Presentation debt this created, and it is on the owner's own display

Joint angles now advance at 64 Hz, so **the barrel is the only un-interpolated
moving part of a `TransformInterpolation`-smoothed hull**. Bounded arithmetic:
up to 2.81 deg per step slewing, about 1.2 deg tracking. A wash at 60 Hz;
visible in principle above it, and the host here runs 165 Hz. The lane did not
eyeball it in motion - the screenshot drivers fire 30 frames after `Playing`,
which under Xvfb lands mid-scenario-load.

Not deferred silently: measured below, and the clean follow-up is to interpolate
`SmoothLookRotationOutput` for render, which makes smoothness and decision clock
independent instead of trading one for the other.

### No changelog entry, deliberately

Every point-defence and fire-gate entry in `[Unreleased]` is unreleased, so this
is intra-cycle and Changelog rule 3 says it never existed for a reader. The dev
book DID owe one, and got it.

### Other schedule splits of this class: none

The lane surveyed every `FixedUpdate`/`FixedPostUpdate` registration against its
`Update` writers. The torpedo bay's scripted and AI triggers share the shape but
are levels behind a fixed-clock cooldown; `sever_disconnected_structures` is a
one-shot into `FixedPostUpdate` but is topological and applies a flat constant.

## D17 - the simulation is not the problem on PD, and IS the problem on the arena

Phase 6 steps 2 and 3, measured on `cfdfd397`. Full numbers in
`tasks/20260819-173219/notes-headless-simulation.md`.

### The epic's question, answered twice, differently

**Point defence: nowhere near the budget.** Four saturated headless captures at
2,400 rounds and 2,600 colliders read 1.81-3.15 ms mean, 0-2 fixed steps a
frame, 64 steps a second exactly. Envelope fill 7.1 s and peak rounds
2,415-2,421 reproduce the point-defence lane's 7.1 s and 2,411 on a different
day. That range's cost was never physics; it was assets, and two lanes have now
cut it.

**The arena: misses 60 fps with the renderer DELETED.** 1v1 reads 9.8-16.4 ms
mean with a 1% low of 11-15 fps; 4v4 reads 11.7-13.2 ms with a 1% low of 9-12,
and both run up to six or eight fixed steps in a single frame. No window, no
adapter, no render world.

**So the "1v1 at 60 fps" target cannot be reached by render work alone.** That
is the headline and it changes what the rest of this epic should do.

### The two transports are limited by DIFFERENT things

Rendered, the line is roughly 8 ms per ship. Headless, **four times the ships
costs 15-30%** - 1v1 and 4v4 overlap. Whatever dominates the headless arena is a
per-SCENE constant.

That is not a contradiction, it is the distinct-asset law again from the other
side: per-ship cost is per-ship ASSETS, and headless has no assets to prepare.
What it means practically is that a fix aimed at one transport should not be
expected to show in the other, and that the arena needs BOTH.

### What the trace names

Per frame, headless 1v1: `PostUpdate` unattributed 1.23 ms, **visibility ~1.15
ms**, `Update` unattributed 0.53, `propagate_parent_transforms` 0.29,
`state_to_world_system` 0.23, `update_ai_target` 0.21.

**The visibility line is a defect, not a cost.** `reset_view_visibility`,
`check_visibility_cpu_culling` and `mark_newly_hidden_entities_invisible` run
every frame over thousands of entities in a run that draws nothing - about 10%
of the headless frame computing what is visible to a view that does not exist.
Recorded rather than fixed, and it means every headless figure above is
PESSIMISTIC by roughly that much.

Also worth knowing: `wfc_arena::track_damage`, the arena's own measurement
instrument, costs 1.2% of the arena's own frame.

### The instrument defect that produced three wrong answers in one session

Three instruments index by FRAME COUNT: `HOLD_FRAMES = 120`,
`DEFAULT_CENSUS_FRAME = 90`, and the arena's 360-frame window. Headless
multiplies frame rate by 5-10x, so each becomes a window of a fifth to a tenth
of the simulated time it was sized for.

1. `stress_point_defense`'s own summary line read duty 0.401 and aim error 20.0
   deg where the lane that fixed it reported 0.811 and 3.6. **Both are true
   readings of different windows.** The invariants - fill, peak rounds - agreed
   to 0.3%. I nearly reported a correct fix as a regression.
2. The census reported the SAME scene for 1v1 and 4v4 (6,443 against 6,446
   entities, an identical 1,686 skin plates), because 90 frames is 0.3 s
   headless. Moved to frame 1,200 the same rosters read 11,811 entities and
   6,169. **The headless census currently measures whatever happens to exist.**
3. Arena captures do not reproduce at all.

**Not fixed here, deliberately.** `HOLD_FRAMES` is a frame count so a measured
hold outlasts the capture's own frame window; the point-defence lane has just
tuned against it. This wants ONE change across all three - index by simulated
time or by fixed steps - not three local patches, and it should be done by
whoever owns the harness next.

### Two headless holes closed

`wfc_arena` could not run headless: `gate_team_chevrons` required
`Res<HudVisibility>` and `lobby::load_or_open_lobby` required `Res<UiSkin>`,
both from render-gated plugins, so the app panicked before fielding a ship. Now
`Option<Res<...>>` (`cfdfd397`). Same class as the nine `systems/` ranges that
still cannot run headless: **a render-gated plugin's resource, required by an
example.**

### The traced-binary footgun caught its second victim

`probe run` leaves a `debug,trace` binary at the plain build's path, so the next
hand-run is silently traced. Two captures here were contaminated and discarded.
It was already written down and it caught the next person anyway - which argues
for a different path, not a better note.

## D18 - the owner's two calls on D16 and D17

Asked 2026-08-20, both answered.

### Point defence keeps its new strength, unretuned

**"Leave it, fly it first."** No content change. `ROUNDS_PER_MOUNT` stays 40 and
the lane's proposed ~120/mount is NOT applied; `TURRET_ON_TARGET_RAD` is
untouched.

The reasoning that survives writing down: the old value was never chosen. It was
whatever the host's frame rate produced, so there is no correct number to
restore - and a mean aim error of 3.6 deg against 11.3 is the fix working, not a
side effect of it. If it plays too strong, content is the lever and it is
reversible; flying it is cheaper than picking a target feel for a build nobody
has flown yet.

### The next round aims at the SIMULATION

**"Simulation first."** The remaining render cut list - per-hull section
materials, dressing geometry, bake-at-load, preload - is deferred behind it.

D17 is why: the arena misses 60 fps with the renderer deleted, so the cut list
cannot reach the target however completely it lands. It is all still real and
still ranked; it is no longer FIRST.

**The trap to avoid while doing this, stated here because it is easy to walk
into.** Roughly 1.15 ms a frame of the headless arena is visibility work for a
view that does not exist. That is the largest single named item and **cutting it
buys the player NOTHING** - rendered, that work is necessary and stays. It is an
INSTRUMENT fix: it makes a headless capture an honest simulation measurement
instead of one carrying 10% of render bookkeeping. Worth doing, worth doing
early, and worth never counting as a frame-rate win.

What is a real win, because it is present in BOTH transports:

1. The archetype count (phase 6 step 4).
2. The per-scene constant that makes 1v1 and 4v4 overlap.
3. `PostUpdate`'s 1.23 ms a frame of unattributed time.
4. `state_to_world_system` 0.23 ms and `update_ai_target` 0.21 ms.

## D19 - what the frame actually spikes on, and a correction to D17

### The correction first

**D17's "1v1" captures were all 4v4, and its trace never reached the fight.**

`wfc_arena::default_roster()` fields `MEASURED_SHIPS_PER_TEAM = 4` per side
whenever `measuring()` holds, and `measuring()` is `perf_armed() ||
cfg!(feature = "trace")`. **Arming the capture changes the subject.** A real duel
needs an explicit `--ship amber --ship onyx`. So D17's claim that "4x the ships
costs 15-30%, so the headless arena is limited by a per-scene constant" compared
4v4 against 4v4 and is WITHDRAWN.

Measured properly, a true headless 1v1 reads 6.7-8.2 ms mean (122-148 fps) with
a 1% low of 16-22 fps, against 4v4's 9.8-16.4 ms and 9-15 fps. Ship count does
scale it, sublinearly - roughly 1.5-2x for 4x the hulls.

Separately, D17's per-system table came from a trace that stops at first contact:
`trace_pass_env` arms `TRACE_CHROME` without `NOVA_PERF`, so nothing holds the
process open. It measured LOAD and APPROACH, which is why visibility and
`PostUpdate` topped it - that is what dominates a cheap frame.

**What survives, and it is the half that mattered**: the mean has headroom and
the TAIL is the defect, at both roster sizes, with no renderer present.

Two general lessons, both worth more than the numbers: **a measurement flag that
changes the subject**, and **a trace that does not cover the phase you are
asking about**. Both produced confident, wrong attributions.

### The ranked causes

1. **A fixed step nearly eats its own interval.** 58.3% of the fight window's
   wall time is inside `FixedMain`, at **9.02 ms mean per step against a 15.625
   ms budget**. Per-step cost RISES with frame time (7.53 / 9.93 / 9.93 / 19.55
   ms by bucket), which is the opposite of what a pure catch-up artefact does -
   so here the steps are cause, not effect. Over 90% is avian: 6 substeps over
   **4,663 colliders, because every hull section is one.** Nova's own share
   inside a step is `shoot_spawn_projectile` 0.39 ms and
   `on_impact_collision_deal_damage` 0.18 ms.
2. **`torpedo_detonate_system` (`projectile.rs:98`).**
   `project_point_predicate` with a DEFAULT filter and the target test inside
   the predicate, so the BVH cannot prune and it walks toward the whole tree -
   per torpedo, per frame, in `Update`. 17.39 ms on one frame, 802 ms total.
3. **The blast and sever cascade.** Worst fight frame 112.78 ms, 99.47 ms of it
   in four steps: `resolve_nova_blast_hits` 11.1 ms plus a 14.4 ms flush,
   `trigger_collision_events` 9.9 ms, `queue_depleted_section_sever` x773, and
   **1,522 collider-tree edits in a single frame**.
4. **The load hitch is the scenario spawn drain.** `state_to_world_system` 568.4
   ms over frames 0-399, max 23.1 ms: `SPAWN_DRAIN_BUDGET` is 3 ms and is checked
   AFTER the command, and one authored hull is one command, so it overruns 4-8x
   on 43 consecutive frames.

### Ruled OUT, with the numbers that rule them out

- **`synchronous_pipeline_compilation`**: there is no render sub-app headless.
- **Mid-run asset loading**: 2,034 spans, 11.78 ms total, max 0.07 ms, and never
  on tid 0.
- **Log volume**: suppressing 52,446 lines made p99 WORSE (94.08 against 73.57).
- **Archetype fragmentation**: 527 -> 525 flat, and 20 archetypes hold 89% of
  entities. **This closes phase 6 step 4.** The 586 figure was real and
  irrelevant.

### The scenario engine is NOT a frame-rate problem

**150.6 us of a 12.24 ms fight frame - 1.2%. The interpreter itself
(`queue_system`) is 2.7 us, or 0.02%.** Nothing in the dispatcher iterates
entities, filters match a `serde_json::Value` payload, and handlers are
name-bucketed. A scenario needs roughly **1,000 `OnUpdate` handlers before
dispatch costs 1 ms**.

**This settles the Lua question on the performance axis: moving the interpreter
cannot buy frame time, because it is not spending any.** Whatever argues for or
against a script language, it is not this.

Where `nova_scenario`'s cost actually sits, and none of it is the interpreter:

- Two GLOBAL `add_observer` collision observers (`area::on_collision_start_event`,
  `salvage::on_crate_pickup_play_sfx`): 23,363 invocations each in 4.1 s in a
  scenario with **zero areas and zero crates**. 56.5 us/frame, 21x the
  interpreter.
- `sample_scenario_queries`: 24.1 us/frame, ungated, two String allocations per
  matching entity per frame whether or not any watch exists.
- Four `asteroid_carve` systems: 14.4 us/frame after the fields are seeded.

A mod author CAN tank it, but through entity count and clone-per-frame growth in
`StoryMessage`/`Objective`, not through rule complexity. `SpawnScenarioObject` on
`OnUpdate` self-gates and silently STOPS the scenario, which is its own defect.

### What to fix first

`torpedo_detonate_system`'s spatial query: one function, on the critical path,
and the target entity is already in hand. Then the two global observers, one line
each. Then the real one - **4,663 per-section colliders at 6 substeps a step is
the fixed step**, and only that moves the 1% low.

Explicitly NOT the headless visibility work. D18 already records that it buys the
player nothing.
