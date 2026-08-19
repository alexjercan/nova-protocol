# Driving plan, agreed 2026-08-20

Owner: turn the conversation into a plan and drive it full send, no stopping.
Decisions that would otherwise be a question get RECORDED in `DECISIONS.md` and
presented when the run ends, rather than blocking.

## Standing rules for this run

1. **Presentation is negotiable, physics and gameplay logic are not.** The epic
   section "What may be traded for frame rate" is the test. Only "would a player
   notice how it LOOKS" is on the table without asking.
2. **One measurement lane at a time.** Every sprout runs Bevy on the same single
   GPU; a contended absolute number is worthless and the contamination is
   non-linear. Code lanes may overlap; measuring lanes may not.
3. **A performance claim without a before and an after is not a result.** Report
   the worst frame and the top system self-time, on a paired protocol.
4. **Never assert a millisecond in a range.** Count the thing that causes the
   cost - grids, materials, bodies - so an assertion survives a different host.
5. **A rejection is a result.** Four tasks in this epic died having been ranked
   against costs that had already moved. Say what was ruled OUT.
6. **Stop and ask only for**: a change to what the game DOES, anything
   irreversible, or a finding that invalidates this plan.

## Phase 0 - finish the measurement queue (in flight)

One lane at a time, in this order:

1. `arena-ablation` - `wfc_ships` at 3/11/17, the zero-ship floor, the owner's
   no-weapons ablation. Holds the box now.
2. `arena-window` - the clean 4v4 number, validating the 420-frame bound, and
   the numeric replacement for the retracted 295.76 ms. Must field a
   menu-equivalent 4v4 so it is comparable to the owner's hand-flown 12 FPS.
3. `pd-stress` - smallest detectable improvement, and the mounts-vs-bays sweep
   that separates round cost from torpedo cost.

## Phase 1 - the 2x, alone (`20260820-003333`)

Quantise damage cracks into N shared buckets. Landed BY ITSELF so the ratio
confirms or refutes the mechanism cleanly - everything else is small enough to
hide inside the noise floor.

Gate: a screenshot of a battered hull at N=8, judged by eye, before it lands.

## Phase 2 - promote the instruments (`20260820-003401`)

Census and ablation as probe capabilities, before `arena-ablation` is deleted
and they have to be reinvented. Feature-gated, loud on a typo, ablate by
`SystemSet` from the probe side.

## Phase 3 - the batch too small to measure alone

Individually 2-10% and invisible against the noise floor; together they should
clear it. Land as one batch with one before/after.

- `ThrusterExhaustMaterial` re-prepared every frame per thruster (0.98 alone) -
  same family of defect as the cracks: a material rewritten per frame.
- Dressing geometry: rocks, derelicts and the planetoid are 86% of all vertices
  for under 10% of instances (0.90 alone).
- Whatever the `wfc_ships` ablation adds.

## Phase 4 - the next round of stress and ablation

Using the promoted harness, so a round is cheap. Known targets:

- **The PD case's ~110 ms.** Two hulls, so cracks cannot explain it; 1,978
  rounds and 86 torpedoes can. **If this is the projectile broad phase, the
  honest fix is "a round should not be a physics body" - which is gameplay
  logic and needs the owner.** Do not decide it in this run.
- **The ~17 ms zero-ship floor.** An empty scene eating a 60 FPS budget.
- **`process_pipeline_queue_system`, 68 ms mid-run.** A deliberate main-thread
  block, kept because async compilation SIGSEGVs one run in five
  (`nova_core/src/lib.rs:390-397`). Any "never block the main thread" rule owes
  this one an answer.
- Coverage holes the map named: carving has one case, NOVA OS and the editor one
  each, WFC generation unreachable from any scenario.

## The loop - phases 1 to 4 REPEAT

Owner, 2026-08-20: fix the damage buckets, add probe capabilities, do more
ablation and measurement, find new bottlenecks, repeat.

So phase 4 is not the end of the run - it feeds phase 1 of the next turn. Each
turn:

1. **Fix** the largest thing the last round measured, alone if it is big enough
   to measure alone.
2. **Extend the instrument** with whatever the fix needed and the harness did
   not have. Every round has produced one: materials were uncounted, the fixed
   step was unpinned, a paused clock was measurable.
3. **Ablate and measure** to find the next bottleneck, on the cheapest subject
   that isolates it - `wfc_ships` for per-ship cost, `stress_point_defense` for
   projectiles, a new `stress_*` for whatever is next.
4. **Record what was ruled OUT**, not only what was found.

A turn ends when the next bottleneck is named and sized. Stop the loop when the
largest remaining measured item is smaller than what the metric can resolve -
at that point the honest move is to improve the metric or stop, not to guess.

**The instrument compounds and that is the point.** The first round needed a
count instrument, a peace switch and a fixed-step pin built by hand on a
throwaway branch. The second round should need none of them, so it can spend its
budget on the question instead of the apparatus.

## What this run does NOT do

- Does not touch physics, collision, damage propagation or flight.
- Does not act on the attitude findings (torpedoes 21% slower, WFC hulls 1.1x
  to 3.5x from torque-bound, stacking costing peak rate). Owner has flown it and
  said fighting feels good; they stay recorded, not actioned.
- Does not chase a frame-rate VERDICT. The epic wants 60; the measured line is
  roughly 8 ms per ship plus a ~17 ms floor, so 8 ships cannot reach 60 without
  an order-of-magnitude change. Report the gap honestly rather than declaring it.
