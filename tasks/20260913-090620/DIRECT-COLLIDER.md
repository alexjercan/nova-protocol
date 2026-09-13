# Direct-collider experiment

## Scope and decision

The user approved profiling sustained combat and comparing a direct-collider
replacement for the second spatial search in `rest_frame_impact`. Firing,
damage, lifetime, target selection, and collision rules must stay unchanged.
Result: correctness holds and the isolated dev workload is faster, but the
release arena sets show no clear gain. This does not fix the regression.
The user approved keeping the optimization and tests independently. The
changelog records removal of the redundant search, not an arena FPS gain.
The task folder stays Markdown-only as requested; results and limits stay
here, while raw runs and helper scripts remain outside the repository.

Base: `7b07528d22b603e3e250fc1b1befc94432218d08`, `master`.
Verification used an uncommitted delta to that base. The task remains OPEN,
priority 100, scheduled for v0.14.0.

## Claim and evidence

`TipWalk::next` already selects a candidate through Avian's spatial trees.
Previously, `rest_frame_impact` searched those trees again for that entity.
The replacement reads the selected collider's `Position`, `Rotation`, and
scaled shape, then uses the same `parry::query::cast_shapes` operation.

Candidate selection is at `crates/nova_gameplay/src/rounds.rs:643`; the new
sampled-collider lookup is at line 1122, inside `rest_frame_impact` (line 1100).
The added query is in `SweepWorld` (line 437).

The reference is Avian 0.7.0, `SpatialQuery::cast_shape_predicate`, in
`src/spatial_query/system_param.rs:522-595`. Preserve:

- Target shape first, round shape second, and the same sampled collider pose.
- Relative velocity and the elapsed-time origin correction on later pierces.
- Zero target distance, stop at penetration, and compute penetration geometry.
- A strict `< max_distance` bound, including exclusion of endpoint contacts.
- Candidate filtering, owner exclusion, sensor transparency, and bite rules.

The body pose is NOT interchangeable with the child collider pose. Avian
samples a child at the top of the step; the body pose can be one step newer.
The existing closing/crossing torpedo tests exercise that production layout.

## Change and blast radius

Private `SweepWorld` gains a sampled-collider query. Private
`rest_frame_impact` takes `&SweepWorld` instead of `&SpatialQuery`.
Bullets and railgun tips use it. No public API, content, ordering, or platform
feature changes. The direct Parry API comes from Avian's existing re-export;
there is no added dependency.

Temporary `rounds_step`, `round_candidate`, and `round_exact` trace spans
were removed before correctness checks and the release build. Trace summaries
cover the final 360 Main schedules.

## Correctness

Local real-Avian unit tests are sufficient for the exact collision contract.
The existing tests cover moving torpedoes, piercing, owner exclusion, sensors,
debris near misses, and railgun rake behavior. Rendered arena and lifecycle
checks remain separate; the unit tests do not prove appearance or arena FPS.

New tests in `crates/nova_gameplay/src/rounds/tests/exact.rs`:

- An already selected collider needs no tree search. Clear the tree resource
  while retaining the collider pose. The old helper returns no hit; the direct
  helper finds it. This is a work-removal assertion, not a new game rule.
- 324 reference comparisons across scaled/rotated spheres, cuboids, and
  capsules, relative velocities, overlaps, misses, and partial-step segments.
- A hit exactly at the segment endpoint remains excluded.

Baseline: 32 passed; the new no-tree-search assertion failed as expected.
After: 33 passed, 1 ignored (manual cost comparison), no failures.

The first test compilation needed `SystemState::get(...).expect(...)` for
Bevy 0.19. The first fixture assumed spawning did not update the tree; Avian's
observer does, so the fixture now explicitly replaces the tree with an empty
one. Those setup failures are not gameplay regressions. Logs remain in scratch.

Commands:

```sh
nix develop --command cargo test -p nova_gameplay --lib rounds::tests::
nix develop --command cargo test -p nova_gameplay --lib \
  rounds::tests::exact::known_collider_cost_against_the_spatial_reference \
  -- --ignored --nocapture --test-threads=1
```

## Fixed-workload diagnostic

The ignored test compares both helpers in one process. It uses the same 1,024
static child colliders, 65,536 queries per arm per repeat, an equal hit/miss
mix, and alternating arm order. Five pairs follow warm-up. Both arms must
return 32,768 hits and exactly the same summed impact times in every pair.
No timing assertion is made.

Dev-profile reference median totals: spatial 42.551366 ms, direct 23.693071 ms
(-44.3%). Spatial range 41.265400-50.167013 ms; direct 23.174033-23.851604 ms.
This establishes a local gain for this workload, NOT release arena FPS.
It does not pin live populations in an autonomous arena fight.

## Profiling and release verification

Scratch root: `/tmp/nova-perf-20260913/optimization/`.

The first active-combat trace completed its gated 360-frame window, with
315 fixed steps. It counted 112-992 live rounds (median 704), 218,100 candidate
searches, and 39,193 exact tests. Exact tests occupied 7.4% of the round-step
span; candidate searches occupied 59.3%. The direct replacement cannot recover
the full regression at that cost split.

LIMIT: that first dev, headless trace overlapped a test build. Its timing is
diagnostic only. It was repeated with no build running, followed by separate
clean release captures. No trace timing is clean FPS evidence.

The idle-queue trace confirms the ranking. Its final 360 Main schedules
contain 319 fixed steps, matching the capture JSON. Live rounds: 129-990,
median 634, with 179,921 round-step visits. Candidate searches: 211,012;
exact tests: 35,612. Candidate spans cost 1.308 ms/step, exact spans
0.156 ms/step, and the round-step span 2.208 ms/step. Exact tests are 7.1%
of that span. These are instrumented dev, headless timings, not clean FPS.

Other inclusive diagnostic costs: `update_sensor_contacts` 2.184 ms/Update,
round command flushes 1.139 ms/fixed step, physics 4.721 ms/fixed step.
Physics contains its solver spans; do not add nested totals. The headless
trace does not attribute render cost. It does not yet split sensor collection
from occlusion or damage command flushes into their individual operations.

The clean release build completed in 14m 43s. Four five-repeat sets ran with
no concurrent build or GPU run, after settling. Display :0, RTX 3060 Ti,
Vulkan, immediate presentation, 1920x1080, default quality, stock 4v4,
60/360 warm-up/capture frames. Draft and gameplay seed: 20260816.

| Set | Mean ref ms | Median ref ms | P99 ref ms | Mean range ms | P99 range ms |
|---|---:|---:|---:|---:|---:|
| Stock before | 20.5745 | 18.0566 | 49.6878 | 19.4590-21.5006 | 42.7862-51.5893 |
| Stock after | 19.9920 | 18.3313 | 46.0289 | 19.4400-21.7724 | 43.1798-51.3547 |
| Ceiling before | 17.7910 | 16.6228 | 39.6817 | 16.9817-20.1378 | 35.8542-45.5653 |
| Ceiling after | 18.2353 | 16.8359 | 38.9660 | 17.1957-19.7284 | 37.3790-45.9537 |

Every set admits 5/5, with no refresh-cap suspicion. All 20 captures completed
without capture refusal or ERROR lines. Stock mean changes -2.8%, median
+1.5%. With `NOVA_PROBE_MAX_DELTA=0.015625`, mean changes +2.5%, median +1.3%.
The directions disagree and repeat ranges overlap. No arena mean, median,
or p99 improvement is established. No new v0.13.2 set was run.

Fixed-step totals, in repeat order:

- Stock before: 450, 488, 474, 449, 495.
- Stock after: 501, 455, 448, 461, 473.
- Ceiling before: 333, 337, 347, 338, 331.
- Ceiling after: 338, 338, 337, 330, 350.

All 20 runs have the same eight-ship section/weapon draft. Their third status
reports retain four roots per team. Fired rounds at that nominal 15-second
report range from 3116-3567 before and 3273-3605 after (stock), and 4668-5152
before and 4594-5088 after (ceiling). These are not exact live-population
matches. The ceiling also changes the Update-to-fixed-step ratio.

## Flow checks and appearance

Correctness-only, with `NOVA_SEED=20260816`, built-in autopilots, fresh profile
sandboxes, and the harness's bounded completion watchdog:

- `wfc_arena`: OK; combat occurred; no invariant violations across 1198 frames.
  This is the example's smoke duel, not the measured 4v4 contract.
- `stress_bullets`: OK; 8 mounts / 16 sections, peak 1624 live rounds, then
  zero before teardown, followed by a clean baseline. 980 invariant frames.
- `stress_point_defense`: OK; 12 mounts / 12 bays, peak 67 inbound torpedoes,
  all 12 mounts working, 56 shot down, peak 2244 rounds, then zero and clean
  teardown. 2400 invariant frames.

All three logs are clean, all artifacts parse, and all claimed checks pass.
Frame-time checks are N/A in these correctness-only runs, not performance
passes. No full workspace tests or Clippy. No agent play: the local collision
contract has real-Avian tests and no new player interaction was introduced.

The separate 1920x1080 combat image was inspected: ships, projectile trails,
impact effects, rocks, and debris render during combat. This is appearance
coverage, not proof of exact impact geometry. That focused screenshot run is
explicitly excluded from every timing set.

`rustfmt --check` on `rounds.rs` and `git diff --check` also pass. All owned
build/run supervisors completed; nothing remains running.

## Evidence location and limits

This task retains Markdown findings only. Raw runs remain under
`/tmp/nova-perf-20260913/optimization/`: `stock-{before,after}`,
`ceiling-{before,after}`, `trace-idle`, `correctness/7b07528d2`, and
`visual-after`. Unit logs and the old/new binaries are there too.
The capture and trace-summary helpers are in `/tmp/nova-perf-20260913/tools/`.
These scratch files can disappear; rerun checks when fresh proof is needed.

The correctness harness reports the base SHA only, not the uncommitted delta.
The runtime was unchanged throughout verification. The tested binary SHA-256s:

- Before: `8ec00280bf12357a5cdaaa2a7a728f8aca672102bdb7859af4265d0baa214164`.
- After: `2f77a46698919e5a1e112a210a3cc6be6b1fcd2b24ddf2cf0b63860b2848c50b`.

Seed correction: the earlier identification captures passed `--seed`, which
pins the arena draft only. `NOVA_SEED` seeds the gameplay RNG separately;
without it `NovaGameplayPlugin` draws entropy. Fresh optimization comparisons
set both to 20260816. Earlier frame sets are not exact gameplay-RNG matches,
and the fresh arms must be compared to each other, not treated as the same
control as the earlier v0.13.2 figures. The first trace also lacked this pin.

The ceiling permits zero-step frames; it is not a fixed-tick/live-population
control. Dynamic arena workload matching and full cost attribution remain
open. A local exact-test gain alone does not meet the task's acceptance goal.
