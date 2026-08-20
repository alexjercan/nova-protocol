# Where the fixed step's 9 ms goes, and what actually shrinks it

Measured 2026-08-21 on `ddbc6425` plus two throwaway instrumentation commits
on this branch (`NOVA_SUBSTEPS`, `NOVA_STEP_DIAG`, `NOVA_SLEEP_CENSUS`,
`NOVA_NO_PREPHYS_PROPAGATE` in `crates/nova_gameplay/src/plugin.rs` - none of
them to ship), headless (`NOVA_NORENDER=1`), never under Xvfb, `dev` profile,
i9-12900F. MEASURED numbers come from runs in this session; ESTIMATED numbers
are arithmetic on top of them.

The instrument that carries this note: avian 0.7 populates
`CollisionDiagnostics`, `SolverDiagnostics` and `SpatialQueryDiagnostics`
unconditionally, so a per-step CSV (`NOVA_STEP_DIAG`) reads the engine's own
phase timers plus a `FixedFirst -> FixedLast` wall stamp, with NO tracing
overhead and no 5 GB file. Selection of "fight" steps is done on the CSV's own
columns (contacts, dynamic bodies), not on window placement.

## Structural answers first - what needed no measurement

- **Sections of the same hull do NOT test against each other.** A hull is ONE
  `RigidBody::Dynamic` on the root (`nova_scenario/src/objects/spaceship.rs:294`)
  and every section, skin plate and greeble is a child COLLIDER of that body
  (`nova_ship/src/sections/base_section.rs:367`, `shell_skin.rs:728`, `:753`).
  Avian's broad phase skips same-body pairs outright
  (`avian3d-0.7.0/src/collision/broad_phase/bvh_broad_phase.rs:261`,
  `proxy1.body == proxy2.body`). There is no intra-hull pair work to remove.
- **Everything runs avian DEFAULTS.** No `CollisionLayers`, `SubstepCount`,
  `SolverConfig`, `NarrowPhaseConfig`, `SpeculativeMargin`, `SweptCcd` or
  sleeping tuning anywhere in `crates/` (exhaustive grep). Substeps = 6,
  speculative margin unbounded, 64 Hz from Bevy's default.
- **No physics joints exist.** Turret "joints" are animated transforms.
  Substeps therefore buy contact-resolution stiffness only - not joint
  stability, not stacking (gravity is zero), not bullet hit detection (bullets
  are `Sensor` colliders; sensor pairs resolve in the narrow phase once per
  STEP, and speculative AABBs cover tunnelling independent of substep count).
- **The 4,663-collider census was the 4v4.** A true 1v1 carries ~1,500 scene
  colliders (2 hulls' sections + plates + greebles + asteroids) plus up to
  ~930 LIVE BULLETS, each a dynamic body with a sensor collider.

## Where a step's milliseconds actually sit (MEASURED, untraced)

One full 1v1 (`wfc_arena --ship amber --ship onyx`, 937 steps, heavy fight:
dynamic bodies peaked at 929). Fight-regime steps (contacts >= 1812, n=59),
mean per phase from avian's own timers, wall from a `FixedFirst -> FixedLast`
stamp:

| phase | ms | share of step |
|---|--:|--:|
| wall, whole step | 10.48 | 100% |
| broad phase | 1.26 | 12% |
| narrow phase | 0.96 | 9% |
| solver: prepare + substep loop + finalize | 1.32 | 13% |
| ...of which the 6-substep loop itself | 1.20 | 11% |
| spatial query update | 0.00 | 0% |
| EVERYTHING ELSE in FixedMain | 6.94 | 66% |

The same run's full-world QUIET steps (few contacts, ~770 bullets in flight)
still cost 7.5 ms wall. The step's cost is dominated NOT by the substep
solver and not by contact pair work, but by per-step bookkeeping that scales
with body and collider count - see the attribution below.

Contact constraint count never exceeded 51 (mean 2-4): almost nothing ever
TOUCHES. The 3-4k "contacts" are AABB-overlap pairs (speculative margins on
fast bullets), which cost broad/narrow time but almost never reach the
solver. Six substeps re-integrate ~900 bodies six times to resolve, on a
typical step, TWO contact manifolds.

## What the "everything else" is (traced, shares not levels)

One traced 1v1 (6.5 GB chrome trace, fight seconds only, per-step figures
scaled from per-frame spans by the measured 7.44 frames/step; tracing
inflates absolutes ~10-20%, so read SHARES):

- `FixedPostUpdate` total ~75% of the step; `PhysicsSchedule` ~59%; the
  `SubstepSchedule` inside it ~23% of the step at 6 substeps.
- Inside `PhysicsSchedule` but OUTSIDE the substep loop: avian re-runs
  `mark_dirty_trees` + `propagate_parent_transforms` +
  `sync_simple_transforms` over the WHOLE entity tree every step
  (`PhysicsTransformConfig::propagate_before_physics`, default true), plus
  `update_solver_body_aabbs`, collider-transform sync, moved-collider AABB
  refit, solver-body prep `par_for_each` passes, `transform_to_position` /
  `position_to_transform` writeback, and `trigger_collision_events`.
- Nova's own `FixedUpdate` share of a step is ~10% (shoot_spawn_projectile
  and its command flush are the biggest single item).
- Schedule executor self-time (`FixedPostUpdate` self) is ~0.6 ms/step even
  traced - the multithreaded executor is a fixed toll per schedule per step.

## Sleeping (MEASURED)

`NOVA_SLEEP_CENSUS`: 40-41 bodies are ALREADY asleep through the whole run
(asteroids, derelict wrecks). Ships never sleep and never will while a PD
controller is alive: `sync_controller_section_forces` applies a torque every
fixed tick and avian wakes a body on any non-zero force
(`avian3d-0.7.0/src/dynamics/rigid_body/forces/query_data.rs:301`). Bullets
never sleep (constant velocity). So sleeping is already doing all it can in
a fight scene; there is no idle-hull win available in the arena, and the
distant-hull question only matters for chapter scenes with parked ships -
which already sleep unless a controller section is alive on them.

## The substep sweep (MEASURED): the substep count is NOT the lever

`wfc_arena --ship amber --ship onyx`, five arms interleaved x 8 rounds, 40
runs, every capture landed, no refusals. Fight regime = steps with >= 500
dynamic bodies, selected per run from its own step CSV; per-run medians, then
rank stats across runs (Mann-Whitney vs the substeps=6 arm).

| arm | fight-step median (ms) | fight-step p95 | substep-loop (solve) ms | 1% low fps (frame window) |
|--:|--:|--:|--:|--:|
| 6 | 8.33 (7.67-9.73) | 15.08 | 0.970 | 26.1 |
| 4 | 8.31 (p=0.92) | 14.35 (p=0.21) | 0.763 | 27.2 (p=0.40) |
| 2 | 10.55 (p=0.06, WORSE) | 14.84 (p=0.92) | 0.571 | 27.5 (p=0.60) |
| 1 | 9.24 (p=0.21) | 13.26 (p=0.001) | 0.307 | 29.3 (p=0.60) |

- The solve column is the proof the knob works: it scales EXACTLY with the
  substep count (0.97 -> 0.31 ms, 6 -> 1). The step total does not follow,
  because the loop is only ~11% of a fight step.
- The only significant frame-level effect anywhere: fight-step p95 down
  1.8 ms at substeps=1. The 1% low never separates at n=8. Nothing else
  reaches p<0.05 in the improving direction.
- **Ruled out as the primary fix: substeps 6 -> 4 buys nothing measurable
  (p=0.92 on the fight-step median); 6 -> 1 buys ~0.7 ms mean / ~1.8 ms p95
  per step, an ~8% effect, invisible on the 1% low.** The quality question
  the sweep was meant to price is therefore moot - the win is too small to
  spend any quality on. (PD's own invariants across the same arms are in the
  PD section as the quality readout anyway.)

### The propagate-off arm is attribution, not a fix

Arm P (substeps 6, `propagate_before_physics = false`): fight-step median
7.74 ms (p=0.036 BETTER) with fight-step p95 16.93 (p=0.006 WORSE) - and the
arm CHANGED THE FIGHT: median 1,169 dynamic bodies against ~690 in every
other arm, fights ~60% longer. Stale collider poses change turret aim, so
this arm is a different workload, kept only as attribution: a step that
skips whole-tree propagation reads cheaper at the median DESPITE 70% more
bullets, which brackets the per-step propagation slice at roughly 1-3 ms in
a fight. Do not ship this switch.

## The quality answer on substeps: contact resolution COLLAPSES below 4-6

The count that prices it, from the same 40 arena runs (per run: the PEAK
simultaneous contact-constraint count, and how many steps spent over 1.5 ms
in the narrow phase):

| arm | peak contact constraints (range over 8 runs) | steps with narrow > 1.5 ms |
|--:|---|---|
| 6 | 51-62 | 0-16 |
| 4 | 117-196 | 5-22 |
| 2 | **1,085-1,149** | 24-458 |
| 1 | 412-518 | 30-144 |

Mechanism: with fewer substeps each impulse iteration resolves less
penetration, so when hulls RAM - which the arena does every match - they sink
into each other and stay overlapped for whole seconds. At substeps=2 the
manifold count during a ram is 20x the substeps=6 figure, the narrow phase
alone spikes to 4-6 ms/step, and two of the eight runs carried 1% lows of
16.4-16.9 fps against the arm-6 median of 26.1. **The solver saving is
smaller than the contact-management bill it creates, and the failure is
gameplay-visible (interpenetrating hulls).** This is the "what actually
degrades" answer: not tunnelling (speculative contacts are per-step and
unbounded by default; bullet sensor hits are narrow-phase per-step;
PD interception invariants held in every arm), but penetration recovery
under sustained hull-hull contact.

## PD cross-check (MEASURED): the pure saving exists where nothing rams

`stress_point_defense`, 4 arms x 6 interleaved runs. Saturated steps
(>= 1,500 bodies), per-run medians, U vs arm 6:

- step median: 6: 3.20 ms -> 2: 2.84 (p=0.004), 1: 2.86 (p=0.010), 4: 2.92
  (p=0.078). An ~11% step saving, matching the solve share.
- frame level: substeps 2 and 4 move NOTHING significantly. Substeps 1 makes
  frames WORSE: 1% low 122 -> 62 fps (p=0.016), p99 8.95 -> 16.10 ms
  (p=0.016), and the scene's outcome changes (40-57 torpedoes shot down
  against 31-48 at six; runs run 1-2 s longer).
- quality invariants (fill 7.2 s, peak ~2,350 rounds, aim error 10.4-13.0
  deg, hits/round 0.0050-0.0069) stay inside the arm-6 spread at 2 and 4.

PD has sensor-only rounds and no hull-hull contact, so it shows the substep
knob's true upside - and the arena shows what the same knob costs the moment
things actually touch. **Verdict on the substep option: keep 6. REJECTED as
a fix: 6 -> 4 buys nothing measurable anywhere; 6 -> 2 is net NEGATIVE on
the fight (median step +2.2 ms, contact blowups); 6 -> 1 degrades PD frames
and changes outcomes.**

## What a bullet actually costs (MEASURED on two more subjects)

- `stress_bullets` (8 turret mounts, empty sky, ~16 section colliders): step
  wall 0.52 ms at ~0 rounds -> 4.14 ms at ~1,550 rounds. **~2.3 us per round
  per step all-in** (run 1; run 2 noisier but same order).
- `stress_point_defense`: 1.74 ms at 80 bodies -> 4.01 ms at ~2,110. **~1.1
  us per body per step.**

So ~700-900 live rounds account for only 1-2 ms of an 8-10 ms arena fight
step. The earlier cross-run slope of 8-9 us/body conflated bullets with the
FIGHT: the arena's extra cost rides on hits, damage observers,
spawn/despawn churn and pair work against hull colliders, all of which
correlate with the round count without being caused per round.

## The 4v4 anchor (MEASURED, untraced)

Two 4v4 diag runs (default roster under NOVA_PERF, 5,700-6,200 colliders
with rounds included - the census's 4,663 plus bullets):

- fight steps (>= 800 bodies): wall mean 10.9 / 13.9 ms, p95 24-27, max 34.
  Split: broad 1.3-1.9, narrow ~0.5, solver 1.4-1.7, **other 7.7-9.8 ms
  (70%)**.
- frame captures: 1% lows 10.9 and 6.8 fps - the regime the epic measured.
- quiet full-world steps (8 hulls spawned, ~50 bodies, pre-fight): **3.85
  ms**, against the 1v1's 2.65 ms and a first-64-steps near-empty world at
  2.3 ms. The base is a NEAR-CONSTANT ~2.3-2.7 ms of per-step machinery plus
  only ~0.4 us per additional collider - doubling the collider census from
  1v1 to 4v4 adds just 1.2 ms to a quiet step.

## What the 70% "other" is made of (traced 4v4 fight window, SHARES)

One traced 4v4 (4.5 GB), fight seconds only, per-step figures scaled by the
measured 4.24 frames/step. Tracing inflates absolutes and the traced fight
ran lighter (~816 peak bodies against the diag runs' ~1,260), so read
SHARES of the 7.64 ms traced step, not levels:

| group | ms/step | share |
|---|--:|--:|
| SubstepSchedule (the 6-substep solver loop) | 1.48 | 19% |
| avian pair + bookkeeping: broad 0.69, narrow 0.25, solver-body AABBs 0.78, collider transform sync 0.29, moved-AABB refit 0.17, writeback 0.23 | ~2.4 | 31% |
| `trigger_collision_events` + the observers it fires (`on_impact_collision_deal_damage`, damage audio, bullet-hit resolve) | 1.15 | 15% |
| nova `FixedUpdate` (shoot_spawn_projectile + its command flush 0.73, AI/controller/gravity) | 1.23 | 16% |
| schedule executor self-time inside the fixed loop (FixedPostUpdate self 1.00 + PhysicsSchedule 0.19 + SubstepSchedule 0.25) | ~1.44 | 19% |
| whole-tree transform propagation (per-step avian pass merged with the per-frame pass - upper bound) | <= 1.08 | - |

Cross-thread `par_for_each` sums overlap wall time, so groups exceed 100%
slightly; the shape is what matters. The near-constant ~2.3 ms empty-world
step from the CSVs is the executor + propagation + fixed avian passes: **the
fixed loop pays ~15% of its entire 15.625 ms budget in machinery before any
gameplay work runs.**

Dev-profile caveat: first-party code is opt-level 1, bevy/avian are 3, so
the nova slices (FixedUpdate, observers) shrink in release while the avian
bookkeeping shares hold. Every ranking below survives that skew; the exact
milliseconds do not.

## RULED OUT, each with its number

- **Intra-hull section pairs**: structurally absent (same-body skip in the
  broad phase; one body per hull). No fix exists to make.
- **Substep count**: 6 -> 4 moves nothing measurable anywhere (arena
  fight-step median p=0.92, PD p=0.078); 6 -> 2 is net negative in a fight
  (median +2.2 ms, peak contact constraints 55 -> 1,100+); 6 -> 1 degrades
  PD frames (1% low 122 -> 62 fps, p=0.016) and changes outcomes.
- **Sleeping / idle hulls**: 40-41 bodies already sleep (asteroids,
  wrecks); a fighting or PD-controlled ship cannot sleep because controller
  torque wakes it every tick, and bullets never sleep. Nothing left to take
  in a fight scene.
- **Raw collider count as the story**: doubling the scene's colliders (1v1
  -> 4v4) adds only ~1.2 ms to a quiet step (~0.4 us/collider marginal);
  the pure bullet cost is 1.1-2.3 us/round/step. The census number (4,663)
  was never the multiplier the plan assumed - the multiplier is the fight.
- **Log volume, asset loads, archetypes**: ruled out by the previous round
  (`notes-frame-spikes.md`); nothing here contradicts that.

## Ranked recommendations

Mechanical, no gameplay change (a fix lane can start these):

1. **Batch the collision-event consumers.** Convert
   `on_impact_collision_deal_damage`, the damage-audio observer and
   `resolve_bullet_hit` from per-event observers into systems draining
   avian's `CollisionStart` messages once per step. The whole event+observer
   pipeline measures 1.15 ms/step (15%) in a 4v4 fight and it is the slice
   that EXPLODES during ram/sever cascades (12,673 impact events in one 1v1
   run, 5,087 in one second). ESTIMATED win 0.4-0.8 ms/step typical, several
   ms on cascade steps - it attacks the tail, which is the stated goal.
   Semantics: within-step event order must stay irrelevant (verify).
2. **Executor overhead experiment.** ~1.4 ms/step of schedule executor
   self-time (plus its share of the 2.3 ms empty-step base). Try
   single-threaded execution for the small fixed-loop schedules
   (SubstepSchedule especially - 6 spins/step) and measure; many fixed-loop
   systems are too small to amortise a 24-thread fan-out. ESTIMATED 0.5-1.5
   ms/step, config-level effort. Reject with a number if it does not pay.
3. **Substeps: leave at 6.** Rejected above.

Needing the OWNER (gameplay-adjacent, flagged as such):

4. **A round is not a physics body** - the decision D10/TASK.md already
   flagged. Rounds become nova-swept projectiles (segment cast against the
   collider tree per step; no RigidBody, no Collider, no per-round tree
   churn). Removes the per-round bookkeeping (1.1-2.3 us x 700-1,500
   rounds), the shoot flush (0.73 ms/step measured), and the bulk of the
   3-4k speculative pair graph. Damage localisation is PRESERVED: the sweep
   yields the same section hit + relative velocity, so `apply_damage` gets
   identical inputs. ESTIMATED 2.5-4.5 ms off a 4v4 fight step, 1.5-2.5 off
   a 1v1. The single biggest lever on the table.
5. **Two-tier hull collision (the owner's sketch), SIZED, not built.**
   Hull-level proxy in the broad phase; per-section resolution on demand
   against a nova-owned per-hull static BVH (sections are rigid in body
   space; refit only on sever). Same damage answer, computed when a hull
   test hits. What it can win: the ~0.4 us/collider quiet-step slope
   (~1.7 ms at the 4v4 census) plus most of broad 0.69 + solver-body AABBs
   0.78 + collider sync ~0.46 - ESTIMATED 2.5-3.5 ms/step at 4v4, ~1-1.5 at
   1v1. What it costs: bullet-hit routing, blast queries, torpedo fuze,
   mass aggregation and severing all re-route through the new path;
   severed fragments must re-materialise real colliders. Weeks-scale, the
   deepest rework here - and if item 4 lands first, rounds stop paying the
   pair bill, which shrinks this item's remaining value to the base slope.
   Recommendation: do 4 first, re-measure, then decide 5.

Arithmetic on the target: the 1v1 fight-step median is 8.33 ms. Items 1+2+4
estimate to 2.3-4.1 ms off, landing the step at ~4.5-6 ms - under the 8 ms
ceiling with the 5 ms goal in reach. The 4v4 lands ~7-10 from 10.9-13.9.
Item 5 is the reserve if 5 ms must hold at 4v4 scale. Every estimate above
is arithmetic on measured slices, not a measured fix; the fix lane must
re-measure each landing.

## Raw data

`measurements/fixed-step/` beside this note: per-arm run tables
(`arena_cmp.txt`, `pd_cmp.txt`), the step-split summaries, and the traced
span tables. Step CSVs and traces stayed outside the repo (gigabytes).
