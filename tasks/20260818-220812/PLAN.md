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

## Phase 3 - render-world Prepare, then the batch too small to measure alone

**Re-ranked twice. Read D12 before this section.** D9 promoted "the floor"
above this batch; D12 retracted the floor entirely - it was 13.7 ms of Xvfb,
and the real empty scene is 3.02 ms. What the floor investigation DID find is
a bigger and better-aimed lead, so the promotion stands and only its subject
changed.

1. **`Prepare` + `PrepareMeshes`: 16.1 ms of a 26 ms one-hull frame.** CPU, in
   the render world, building per-instance buffers and bind groups over 986
   mesh instances. This is the single largest named cost in the game and it is
   presentation, so it is takeable without asking.
   **Start with the sublinearity**: the first hull costs 24.6 ms and each
   further hull about 8. A 3x gap between the first and the marginal is a
   mechanism, not a curve, and naming it probably names the fix.
   **Measure on a real display.** Xvfb adds ~13.7 ms of per-pixel CPU copy at
   720p and would swamp this.
2. ~~**`ThrusterExhaustMaterial`** re-prepared every frame per thruster.~~
   **DONE, `d4670f32`. See D15.** It needed TWO fixes rather than one: the
   warhead material was a per-instance asset holding a per-type value, and the
   plume was a per-frame value that no read-before-write guard could ever cover.
   On `stress_point_defense` the pair took `min_ms` 17.83 -> 5.47.
   **What it changed about this list**: the render world stopped being the pacer
   on that range (98.0% of the traced frame -> 48.5%), so item 1 below is no
   longer obviously the top of the board and phase 6 is where the argument moved.
3. **Per-hull section-material duplication** (D7). Every hull instantiates its
   own copies of the same ~35 section materials: 381 bins at 11 ships where 35
   would do. A gltf / `WorldAssetRoot` instancing question.
4. **Dressing geometry**: rocks, derelicts and the planetoid are 86% of all
   vertices for under 10% of instances (0.90 alone).

Items 2-4 are individually 2-10% and invisible against the noise floor; land
them as one batch with one before/after. Item 1 is measurable alone.

## Phase 4 - the next round of stress and ablation

Using the promoted harness, so a round is cheap. Known targets:

- **The PD case's ~110 ms.** Two hulls, so cracks cannot explain it; 1,978
  rounds and 86 torpedoes can. **If this is the projectile broad phase, the
  honest fix is "a round should not be a physics body" - which is gameplay
  logic and needs the owner.** Do not decide it in this run.
- **The 7 ms spread at ONE ship** (36-48 FPS). Steady state does not do that.
  Two candidates: the un-pinned fixed-step amplifier alternating 1 and 2 steps
  a frame, or `process_pipeline_queue_system` below. Cheap to separate - the
  pin costs +1.5% on this subject.
- **`process_pipeline_queue_system`, 68 ms mid-run.** A deliberate main-thread
  block, kept because async compilation SIGSEGVs one run in five
  (`nova_core/src/lib.rs:390-397`). Any "never block the main thread" rule owes
  this one an answer.
- Coverage holes the map named: carving has one case, NOVA OS and the editor one
  each, WFC generation unreachable from any scenario.

## Phase 5 - is the documentation UNDERSTANDABLE, and is the design minimal

Queued 2026-08-20 by the owner, to run AFTER the `docs/` and `/wiki/` passes
land. **Report only - no changes.** The two passes before it fix what is WRONG;
this one asks whether what is right is also comprehensible, and whether the
thing being described should be smaller.

Owner's framing: "check if it's understandable, or if some decisions are
weird... stuff that helps keeping things minimal".

### The exemplar, and it holds up

Owner: "why do we need `NOVA_SHOT` and `NOVA_CAPTURE`, aren't they the same
thing?"

They are not, and the reason is real:

- **`NOVA_SHOT` is a DRIVER.** It arms `ScreenshotPlugin`, which forces state
  to `Playing` and shoots one settled frame. It moves the app on its own.
- **`NOVA_CAPTURE` is a BRANCH on someone else's driver.** It arms the capture
  path of an autopilot script; the script reads `capturing()` while building
  its steps and shoots at its own beats instead of driving straight through.
  It drives nothing by itself and is meaningless without `NOVA_AUTOPILOT` -
  every example sets the pair.

They collide because both would write `NextState`, which is why `NOVA_SHOT`
stands down with a warning when `NOVA_AUTOPILOT` is set.

**So the design is sound and the NAMES are the defect.** Two names built on the
same noun, for a driver and a sub-flag of a different driver. The justification
exists - `crates/nova_autopilot/src/lib.rs:48`, "deliberately distinct" - but it
lives in a crate rustdoc block a `/dev/` reader never opens, and it explains
that they differ without making it obvious HOW. A name like
`NOVA_AUTOPILOT_CAPTURE` would answer the owner's question in the name.

This is the SHAPE to look for: not a wrong document, but a correct one whose
reasoning is only reachable if you already know the answer.

### The other lead already found

**Over 40 `NOVA_*` environment variables**, and about twenty are `NOVA_OS_*`
CRT tuning knobs - `CRT_WARP`, `CRT_OVERSCAN`, `CRT_POWER_EPSILON`, `CASE_EDGE`,
`CASE_RADIUS_TOP_PX`, `CASE_RADIUS_BOTTOM_PX`, `SCREEN_RADIUS_PX`, `CONTENT_Z`,
`SCAN_DETENTS`, `BRIGHT_DETENTS`, `DEGAUSS_DURATION`, `COIL_VOLUME`,
`BED_VOLUME`, `PHOSPHOR`, `PHOSPHOR_DIM`, `PHOSPHOR_MUTED`, `AMBER`, `TEXT`,
`SCREEN`, `TERMINAL_HINTS`, `RTT_LAYER`.

Those read as playtest-tuning knobs that were never baked once the look
settled. Each is a live configuration surface, a thing to document, and a thing
that can be set wrong. Ask which survived their tuning phase.

### What the pass reports

For each finding: what is confusing or redundant, the evidence, and what the
minimal version would be. No edits, no tasks filed. The owner ranks it after.

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

## Phase 6 - the simulation itself, agreed 2026-08-20

Owner set this order and authorised running it without stopping between steps.

**The target: physics and logic under 10 ms a frame.** Owner's words, and it is
the right axis - but aim LOWER than 10. The fixed step is 1/64 s = **15.625 ms**,
so a 10 ms step sits at 64% of its own interval; one overrun runs two steps,
which costs 20 ms, which overruns again. That is the clamp spiral already
measured at `fixed_steps max=16` reading 457 ms a frame. It is a cliff, not a
slope. **Treat ~8 ms as the ceiling and 5 ms as the target**, for roughly 2x
margin before a hitch can cascade. For reference, PD stress today is avian at
5.17 ms with the whole targeting and point-defence chain under 0.2 ms.

1. **Fix point defence being frame-rate dependent.** In flight. Gameplay logic,
   authorised explicitly. Evidence is a COUNT: rounds yield spread 708-2,425
   free-running should close on its pinned 2,236-2,401 without the pin, and the
   rendered-vs-headless gap of 64 against 793 intercepts should collapse.
2. **Headless PD stress**, once (1) lands - so the number describes the
   corrected game rather than the frame-rate-dependent one. This is also the
   deferred simulation-only figure the epic has never had.
3. **Headless 4v4 arena**, and more ranges as they earn it. 13 of 25 already run
   headless (`notes-headless-transport.md`); the four `stress_*` ranges are
   among them.
4. **The 586 archetypes on an EMPTY scene** (`perf-map.html`, card 01). Expected
   to matter MORE for the simulation number than the render one, because
   archetype fragmentation taxes every query in the schedule rather than one
   system.
5. Whatever those turn up.

**Steps 1-3 are DONE.** See D16 and D17. The result re-aimed the epic (D18):
the arena misses 60 fps with the renderer deleted, so the presentation cut list
in phase 3 cannot reach the owner's target however completely it lands, and the
simulation goes first. Phase 3 items 1, 3 and 4 are deferred, not dropped.

Rank inside the simulation, from the traced headless 1v1:

- the archetype count (step 4 above);
- the per-scene constant that makes 1v1 and 4v4 overlap within 30%;
- `PostUpdate`'s 1.23 ms a frame of unattributed time;
- `state_to_world_system` 0.23 ms, `update_ai_target` 0.21 ms.

And ONE instrument item ranked with them because everything above is measured
through it: the three FRAME-COUNT windows (`HOLD_FRAMES`,
`DEFAULT_CENSUS_FRAME`, the arena's 360) want re-indexing to simulated time.
Three wrong answers came out of them in one session.

**Do not count the headless visibility work as a frame-rate win** (D18). It is
~1.15 ms a frame and cutting it buys the player nothing - rendered, that work is
required. It makes a headless capture honest; it does not make the game fast.

**Read every headless number against this**: headless is NOT the same simulation
minus pixels. Some systems run per frame and headless ticks faster, which is the
defect step 1 exists to fix - and there may be more of them. A headless figure is
a simulation-cost measurement, never a gameplay one.
