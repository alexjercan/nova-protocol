# Phase A: the instrument

What this phase built and measured. It makes nothing faster. The only game
behaviour that can change is behind a measurement knob that is off by default.

## How everything below was measured

- Tree: `a68fb8a1` plus this task's harness changes, on branch
  `perf-baselines`. Every number here was taken at that SHA and is comparable
  with nothing taken after it: two other lanes were landing changes while this
  ran (`SectionConfig.mass`, the `nova_ship` attitude model).
- Host: NVIDIA GeForce RTX 3060 Ti, vulkan, 12th Gen i9-12900F, 31 GiB, NixOS,
  Xvfb `:96`, 1280x720, vsync off.
- Build: `dev` (first-party `opt-level = 1`, dependencies `3`). Settled: these
  RANK, they do not certify. No release run was taken.
- Window: the standard 180 warm-up + 900 captured frames. One capture per
  PROCESS - repeats sharing a process would share the warmed caches and the
  thermal state that are the reason for taking repeats at all.
- Frame numbers come from the frame-time pass, system numbers from the traced
  pass. The two are never compared with each other.
- The raw rows are in `measurements/` beside this file: the three repeat sets
  as `frametime.csv` schema-v3 rows, and every capture's fixed-step summary
  line. Every table below is derived from them.
- Nothing was built WHILE anything was measured. That is not the same as
  measuring on a quiet box, and the difference is one of this phase's
  results: the first `broadside` set began seconds AFTER a build finished and
  spent eight captures recovering. The settled set the tolerance comes from
  was taken after that.

## 1. The metric

### What was built

`probe run <subject> --repeat <n>` runs the frame-time pass `n` times, one
process each, writing rows labelled `<subject>#1..#n`. The report then reads
them as a SET:

- the reference is the MEDIAN of the captures' means, and of their medians, so
  one bad capture cannot drag the band over itself;
- a capture whose mean or median falls outside +/-`REPEAT_GATE_TOLERANCE` of
  its reference is DISCARDED;
- the tail is read only across what survives, as the median of the admitted
  p99 values, printed with the spread of that group. The slowest single frame
  gets the same treatment and is printed beside it as a reading.

`checks.json` carries the whole set under `repeats`, `graded: false`, beside
`frames`. Nothing passes or fails on a frame-time number. A discarded capture
is a statement about the machine, never about the code.

Two knobs came with it, and both earned their place in section 3: every
capture now records how many FIXED STEPS ran inside each frame, and
`NOVA_PERF_MAX_DELTA` forces the fixed-step ceiling for a run.

### The distribution, measured

Ten captures of `wfc_arena` 4v4, unchanged scene, unchanged binary, back to
back on an otherwise idle box:

| run | mean | median | p99 | worst |
|---|---:|---:|---:|---:|
| #1 | 86.68 | 80.31 | 160.97 | 199.87 |
| #2 | 94.49 | 89.06 | 147.28 | 188.71 |
| #3 | 79.00 | 68.90 | 148.21 | 177.78 |
| #4 | 100.82 | 97.27 | 176.46 | 233.74 |
| #5 | 101.38 | 77.12 | 405.96 | **553.68** |
| #6 | 100.98 | 91.59 | 192.79 | 465.00 |
| #7 | 83.36 | 77.40 | 145.02 | 182.84 |
| #8 | 114.90 | 99.27 | 355.07 | 518.58 |
| #9 | 87.46 | 67.14 | 307.45 | 409.89 |
| #10 | 93.41 | 85.41 | 151.52 | 211.91 |

| statistic | median | range, as % of the median | cv |
|---|---:|---:|---:|
| mean | 93.95 ms | 38.2% | 10.7% |
| median | 82.86 ms | 38.8% | 12.6% |
| p99 | 168.71 ms | 154.7% | 42.7% |
| worst | 222.83 ms | 168.7% | 46.4% |

**The premise the gate rests on does not hold for this subject.** The task
records the mean as stable to 0.5% on `editor_sandbox` and `broadside`. On the
4v4 it moves 38% between honest repeats. There is nothing for a validity gate
to be stable against.

### What the gate can and cannot detect

Bootstrap over the ten captures (resample `n` with replacement, run the gate,
read the tail; the smallest detectable improvement is the difference between
two independent sets separating at 1.96 sigma):

| n | tolerance | captures admitted | reported worst | cv | smallest detectable |
|--:|--:|--:|--:|--:|--:|
| 1 | - (today) | 1.00 | 314 ms | 46.4% | **129%** |
| 3 | 5% | 1.43 | 297 ms | 46.9% | 130% |
| 5 | 5% | 1.84 | 288 ms | 47.2% | 131% |
| 8 | 5% | 2.60 | 181 ms | 37.9% | 105% |
| 8 | 12% | 4.48 | 290 ms | 31.8% | 88% |
| 8 | 20% | 6.91 | 301 ms | 20.7% | 57% |
| 8 | no gate | 8.00 | 314 ms | 16.5% | **46%** |

Read the last two rows together. **On this subject the gate makes the number
WORSE at every tolerance that gates anything.** A tight band admits two
captures out of eight and the survivors are arbitrary, so the reported tail
inherits the full spread; only turning the gate off and taking the median of
all eight brings the smallest detectable improvement down, and it stops at
46%. Reading p99 instead of the worst frame lands in the same place (69% at
n=8 ungated) - which is the tell, because on a subject that IS repeatable p99
does far better than that (27%, below).

That is not a defect in the gate. It is the gate reporting, correctly, that
the ten captures did not measure the same thing.

### Why they did not: the 4v4 window is not one scene

The fixed-step tally added for section 3 found it, and it is unambiguous.

1. **Two of the ten captures ran past the end of the fight.** `#5` and `#10`
   log `match ended`, and the arena's result screen PAUSES `Time<Virtual>`.
   `#10` spent **555 of its 900 frames** with the simulation stopped, at
   88.3 ms a frame - the result screen still draws the whole arena, so the
   window reads as a plausible 93.41 ms mean while measuring a still picture
   for 62% of its length.
2. **Four of the ten sat in the fixed-step clamp** (section 3): `#5` 34
   frames, `#6` 4, `#8` 36, `#9` 17. Those four are exactly the four with a
   worst frame past 400 ms; the six that never clamped all land between 178
   and 234 ms.
3. **The window has no upper bound.** The capture opens on a scoreboard
   predicate (both sides have fired and connected) and then runs 900 frames
   whatever happens - approach, brawl, stern chase, victory, result screen.
   The tail of the fight is a long-range drift: the last logged range in `#5`
   is 2023 units, against an engagement range of 1-2 km.

**So `wfc_arena` 4v4 cannot serve as the release's headline benchmark until
its window is bounded.** Every candidate ranked against it is ranked against a
number with a 46% floor on what it can detect, and against a window that
contains a paused result screen one run in five. This is the finding that
blocks the rest of the epic, and it is a fix to the EXAMPLE, not to the
engine: end the capture when the match does, or field a roster that cannot
finish inside the window.

### The second subject, and the contamination the gate was built for

Eight captures of the shipped `broadside` chapter, taken back to back
immediately after a `cargo clippy` finished:

| run | mean | median | p99 | worst | game simulated in the window |
|---|---:|---:|---:|---:|---:|
| #1 | 102.76 | 100.00 | 140.72 | 147.18 | 92.5 s |
| #2 | 45.00 | 38.48 | 112.32 | 180.34 | 40.5 s |
| #3 | 52.51 | 48.70 | 103.08 | 130.83 | 47.3 s |
| #4 | 40.38 | 34.97 | 94.27 | 152.09 | 36.3 s |
| #5 | 34.14 | 31.69 | 73.43 | 88.45 | 30.7 s |
| #6 | 28.11 | 25.86 | 53.21 | 66.08 | 25.3 s |
| #7 | 26.28 | 24.39 | 38.51 | 61.55 | 23.7 s |
| #8 | 26.57 | 24.14 | 49.20 | 59.10 | 23.9 s |

That is not noise, it is a **monotone recovery**: 102.8 ms down to 26.3 ms
over eight consecutive captures, converging on 26.28 / 26.57 - which is the
26.25 ms `20260819-123928` recorded for this scenario. The box had just spent
two and a half hours under continuous build and capture load, and it took
roughly eight minutes to come back.

Two things follow, and they are the most useful things this phase learned
about the protocol:

1. **A repeat set taken immediately after a build measures the build.** The
   first capture of this set was 3.9x the settled value. Nothing in the
   harness says so, and nothing in a single capture ever would.
2. **The gate as designed does not catch a DRIFT.** Its reference is the
   median of the set's means, which lands in the MIDDLE of a monotone slide;
   a +/-5% band around 34.14 ms admits one capture out of eight and rejects
   both the contaminated end and the settled end. It is built for an outlier
   and this is a ramp.

The gate does report the situation correctly - a set it empties is a set that
did not measure one machine - but "re-run it" is the only action it can
support. The protocol has to keep the machine out of the measurement in the
first place.

### The settled set, and where the numbers come from

Eight more captures of `broadside`, taken once the box had come back, and this
time inside a probe-style profile sandbox (its own `XDG_CONFIG_HOME`,
`XDG_DATA_HOME`, `NOVA_MOD_CACHE_ROOT`) so the operator's saved settings and
enabled mods are ruled out as an explanation:

| run | mean | median | p99 | worst |
|---|---:|---:|---:|---:|
| #1 | 29.47 | 29.47 | 39.98 | 54.01 |
| #2 | 32.70 | 31.09 | 65.46 | 88.95 |
| #3 | 34.15 | 31.65 | 95.78 | 177.87 |
| #4 | 30.05 | 26.69 | 72.09 | 97.87 |
| #5 | 31.21 | 30.09 | 63.45 | 72.65 |
| #6 | 33.58 | 31.31 | 75.42 | 172.64 |
| #7 | 32.18 | 31.08 | 80.02 | 192.07 |
| #8 | 23.25 | 22.79 | 36.27 | 66.57 |

| statistic | median | range, as % of the median | cv |
|---|---:|---:|---:|
| mean | 31.70 ms | 34.4% | 10.5% |
| median | 30.58 ms | 29.0% | 9.8% |
| p99 | 68.77 ms | 86.5% | 28.2% |
| worst | 93.41 ms | 147.8% | 45.5% |

(The sandbox is not the explanation for anything: the settled sandboxed mean,
31.7 ms, sits with the settled un-sandboxed one, 26.3-28.1 ms. The operator's
`settings.ron` holds `graphics_quality: High`, which is also the app default,
and their `enabled_mods.ron` names three mods that are not installed.)

### The metric, written down

Everything below is derived from the eight settled captures by bootstrap:
resample `n` of them with replacement, run the gate, read the tail, and take
the smallest detectable improvement as the difference between two independent
sets separating at 1.96 sigma.

| reported statistic | n | tolerance | admitted | cv | smallest detectable |
|---|--:|--:|--:|--:|--:|
| worst frame, one capture (today) | 1 | - | 1.00 | 45.5% | **126%** |
| worst frame, median of admitted | 8 | 20% | 7.03 | 32.2% | 89% |
| worst frame, mean of admitted | 8 | 20% | 7.03 | 16.6% | 46% |
| p99, one capture | 1 | - | 1.00 | 28.2% | 78% |
| p99, median of admitted | 5 | 20% | 4.42 | 13.5% | 37% |
| p99, median of admitted | 8 | 12% | 6.45 | 11.9% | 33% |
| **p99, median of admitted** | **8** | **20%** | **7.00** | **9.9%** | **27%** |
| p99, median of all (no gate) | 8 | - | 8.00 | 12.6% | 35% |

**The scheme, in the words the epic asked for:**

- **Statistics recorded**, per capture, unchanged plus one: frames, mean, min,
  max, p50, p95, p99, p999, mean fps, 1% low - and now the number of fixed
  steps each frame ran, bucketed by count.
- **Gate**: a capture COUNTS when its mean AND its median are both within
  `REPEAT_GATE_TOLERANCE` of the set's own median-of-means and
  median-of-medians.
- **Tolerance: 20%.** Derived, and deliberately loose. The settled set's means
  themselves spread 34%; a band under about 12% admits fewer than half the
  captures, and the survivor is arbitrary enough that the reported tail gets
  NOISIER - the table above shows 5% at n=8 costing 53% detectability against
  20%'s 27%. At 20% the band still throws out the contamination it is for: the
  102.8 ms capture from the drifting set sits TEN bands from that set's
  reference.
- **Repeat count: 8.** Five gets 37%, six gets 37%, eight gets 27%. Eight is
  also the largest set taken here, so the curve is not measured past it - a
  bigger set may or may not keep paying, and on this host it costs about ten
  minutes of continuous load, which is its own risk.
- **Reported tail: the median of the admitted p99**, with the spread of the
  admitted group printed beside it. The slowest single frame gets the same
  treatment and is printed next to it as a READING - it is one sample out of
  nine hundred and behaves like one.
- **Smallest detectable improvement: 27%** on `broadside`, on this host, in a
  dev build, with eight repeats. Anything smaller than that has not been
  measured, and a claim about the slowest single frame needs 46% even at the
  best setting the data supports.

**And the number that matters most for the epic: on `wfc_arena` 4v4 nothing
gets below 46%,** with the gate off and eight repeats, because that subject's
own load is not repeatable. The headline case cannot support a claim smaller
than a halving until its window is fixed.

Two protocol rules fall out, and neither is in the code:

1. **Do not start a repeat set on a machine that has just been building.** The
   first set here began seconds after a `clippy` run and its first capture was
   3.9x the settled value.
2. **A set the gate empties is not a result.** It means measure it again, on a
   quiet box, or change the subject.

### Why the window drifts, and the one lever that moved it

The capture window is a fixed number of FRAMES. On a simulation driven by the
wall clock that is a VARIABLE amount of game: across the ten runs the same 900
frames covered 71 s to 101 s of simulated fight (`total_steps` 4550 to 6453 at
15.625 ms a step). A slow frame buys more simulation, which puts more of the
fight in the window, which makes the next frame slower. The window's CONTENT
is coupled to the machine's speed.

Pinning `NOVA_PERF_MAX_DELTA` low enough to bind every frame breaks that
coupling on paper - virtual time then advances a constant amount per frame, so
900 frames are always the same span of game. It was tried (section 3) and it
does not help:

| arm | runs | mean spread across runs |
|---|--:|--:|
| default `max_delta` (0.25 s) | 10 | 38.2% |
| pinned at 0.0625 s (4 steps every frame) | 4 | 44.1% |

The fight still diverges - `cap4#1` fired 8112 kinetic rounds, `cap4#2` 4787 -
because the AI decision chain runs on the render clock and the capture's
readiness gate opens at a different fight state each run. **A fixed span of
game is not a fixed amount of game.** For a repeatable subject the content has
to not depend on a live fight at all, which is what the `stress_*` shape
already is and what the 4v4 is not.


## 2. The coverage map

### How it was measured, and what the instrument cannot see

Each subject was run once under `bevy/trace` (`TRACE_CHROME`,
`RUST_LOG=bevy_ecs=info` so the per-system spans are not filtered away, plus an
armed frame capture so the completion protocol holds the app for 600 frames
instead of letting the autopilot walk end it at ~230). The chrome trace is
reduced to the set of system paths that ran and the total span time of each,
and the paths are grouped into subsystems.

**The discriminator, and why it is not "did it run".** A registered system that
early-returns still opens a span every frame, so every subsystem "runs" in
every subject: the first pass of this table had a cell for carving in the menu
backdrop scenes. So the floor is MEASURED instead of assumed - for each
subsystem it is the cost paid by the quietest subject in the table, which is
what wired-and-idle costs - and a subject counts as LOADING that subsystem when
it paid at least twice the floor. `-` still means no system under that path ran
at all.

Three limits, stated because they change how the table reads:

1. **The floor is relative to the subjects measured.** A subsystem every
   subject loads has no idle sample to calibrate against, and reads as loaded
   by nobody. Read a column together with its floor, not on its own.
2. **The trace resolves to SYSTEM granularity, not function.** The thruster
   balance QP (`nova_ship::flight::thrusters::balance_throttles`) is a function
   called from `autopilot_system` and `manual_burn_system`, so the table can
   only say the HOST system ran. The `balancer host` column is named for that
   and must not be read as "the QP solved".
3. **One traced run per subject**, 600 frames from `Playing`. Coverage is a
   presence and a magnitude, not a baseline; the numbers in this table rank
   nothing, and a subsystem that only engages late in a long chapter can still
   be missed.

### What was measured, and what was NOT

Measured: every scenario in the shipped registry plus `editor_sandbox`
(through the game binary, `probe scenario`'s path), and the examples listed in
the table.

Not measured, and why - this is a bound on the table, not a claim about the
subjects:

- **The `screenshots/` category, except `screenshot_gravity`.** A producer
  takes at most three frames of a scene it poses; its coverage is a subset of
  the scenario or rig it poses, and 22 of them do not fit the session.
  `screenshot_gravity` was kept because gravity is on the required subsystem
  list and it is the one example named for it.
- **`system_hud_indicators`, `system_borrowed_battery`, `bug_neutralized_quiet`,
  `system_menu_boot`, `bug_menu_picker`, `block_bench`, `greeble_catalog`,
  `parts_viewer`, `compare_asteroids`, `compare_planets`, `widget_zoo`.**
  Cut for time. None of them is the only case for a subsystem on the required
  list, so no column's verdict rests on them - but a `-` in a column is a
  statement about the SUBJECTS IN THE TABLE, not about the whole catalog.

### The table

34 subjects: every shipped scenario plus `editor_sandbox` through the game
binary, and 24 examples. The full grid is
`measurements/coverage-table.md`; this is the reading of it.

**Who LOADS each subsystem** - the cases where it costs at least twice what the
median subject pays for merely having it wired:

| subsystem | median subject (%) | subjects that LOAD it |
|---|--:|---|
| scenario | 0.50 | `bug_sandbox_soak`, `scenario_editor_sandbox`, `scenario_menu_weave`, `scenario_shakedown_run`, `screenshot_gravity`, `stress_many_structures` |
| gravity | 0.02 | `stress_bullets`, `stress_many_structures`, `stress_torpedoes` |
| AI pilot | 0.06 | `stress_many_structures` |
| flight autopilot | 0.02 | 6 scenarios, `screenshot_gravity`, `wfc_arena` |
| balancer host | 0.05 | `scenario_final_tally`, `scenario_lifeline`, `wfc_arena` |
| targeting / point defense | 0.09 | **NOTHING** |
| turret gunnery | 0.04 | `stress_bullets`, `stress_many_structures`, `carve_asteroids`, `system_turret_gunnery`, `wfc_arena`, `system_player_path` |
| torpedo guidance | 0.05 | `stress_torpedoes`, `system_torpedo_launch`, `wfc_arena` |
| carving | 0.02 | `carve_asteroids` |
| section severing | 0.07 | `system_section_severing`, `system_blast_penetration`, `stress_one_structure`, `stress_many_structures`, `stress_torpedoes`, `wfc_ships` |
| cladding / greebles | 0.01 | `shape_bench`, `stress_one_structure`, `wfc_ships`, `wfc_arena` |
| WFC generation | 0.14 | (only `wfc_arena` and `wfc_ships` run it AT ALL) |
| NOVA OS | 0.30 | `system_nova_os` |
| editor | 0.07 | `system_ship_editor` |

### The holes

1. **Targeting and point defense are exercised by NOTHING.** Every subject
   ticks `nova_ship::input::targeting` and `input::point_defense` - they are in
   every scene - and not one of the 34 puts them above twice the idle level.
   The subsystem that picks locks, runs the radar and assigns point defense per
   turret has no case that loads it. **This is the biggest gap in the suite**
   and it is exactly the kind a `stress_*` range is for: many contacts, many
   inbound torpedoes, one hull working them.
2. **Carving has exactly ONE case.** `carve_asteroids`, at 0.41% against a
   0.02% idle level - a 20x contrast, so the case is a good one. But
   `bug_carve_apply`, the range named for the carve APPLY spike, does not
   register above idle: it reproduces a defect, it does not load the path.
   One case for a subsystem with a known frame-owning spike is thin.
3. **NOVA OS and the editor have exactly one case each**, and both are
   `system_` ranges rather than load cases. NOVA OS is also the most expensive
   always-on subsystem in the table: 0.30% of system time in EVERY subject,
   in scenes where nothing has opened it.
4. **WFC generation exists only in two examples**, and no scenario touches it.
   That is by construction - the generator is example-side code in
   `examples/playable/shared/wfc.rs` - but it means the release's headline
   hull-generation path is not reachable from any shipped scenario, and cannot
   be measured through `probe scenario` at all.
5. **The thruster balance question is settled: a 4v4 DOES touch it.**
   `autopilot_system`, which is where `balance_throttles` is called, cost
   224 ms over 5230 calls in `wfc_arena` against 22 ms over 897 calls in
   `scenario_menu_duel` - 42.8 us a call against 24.5, and the largest flight
   row in the arena by 3x. The AI's passive behaviours engage the flight
   autopilot (`GotoPos`, `Orbit`, `Stop`), so the QP runs on every AI ship
   that is not in a combat maneuver.
6. **Two subjects produced PARTIAL traces**: `system_section_severing` and
   `system_blast_penetration` are headless rigs with no `GameStates`, and they
   crashed the moment the frame capture was armed (see section 4 - the crash
   is fixed, and their traces here are from the fixed build but cover only
   their script, not a 600-frame window). Their `-` cells mean "the run ended
   before that subsystem could appear", not "absent".

## 3. Candidate 1: fixed-timestep amplification

The candidate: `Time<Virtual>::max_delta` defaults to 0.25 s against a
1/64 s = 15.625 ms fixed timestep, so one slow frame can queue up to **16**
fixed steps, which makes the next frame slower - a spiral. Capping `max_delta`
would then bound the tail without making anything faster.

The capture now records how many fixed steps ran inside each captured frame,
bucketed by count (`fixed_steps` in the per-run JSON and on the summary line),
and `NOVA_PERF_MAX_DELTA` forces the ceiling so the intervention can be run.

### The identity that makes the naive reading useless

Over a window where the clamp never fires and virtual time is never paused,
total simulated time equals total real time. So

```
mean steps per frame == mean frame time / 15.625 ms
```

is an IDENTITY. Every default run matches it to three digits (`#1`: 5.547
steps, 86.681 ms, 86.681 / 15.625 = 5.548). A bucket table of frame time
against step count therefore rises along a line of slope ~15.6 ms/step
whatever the cause, and a least-squares fit on it - which comes out at 13.96
to 15.22 ms/step across the ten runs, always just under the timestep - proves
nothing about where the time goes. Only an intervention separates "the fixed
loop made this frame slow" from "this frame was slow, so the fixed loop had
more to do".

### The ceiling IS reached, and it holds

| run | steps/frame | frames at the 16-step ceiling | their mean cost | worst frame |
|---|---:|---:|---:|---:|
| #1 | 5.55 | 0 | - | 199.9 |
| #2 | 6.05 | 0 | - | 188.7 |
| #3 | 5.06 | 0 | - | 177.8 |
| #4 | 6.45 | 0 | - | 233.7 |
| #5 | 5.42 | 34 | 354.1 | 553.7 |
| #6 | 6.44 | 4 | 322.7 | 465.0 |
| #7 | 5.34 | 0 | - | 182.8 |
| #8 | 7.17 | 36 | 321.6 | 518.6 |
| #9 | 5.50 | 17 | 328.9 | 409.9 |
| #10 | 2.49 | 0 | - | 211.9 |

The four runs that reached the ceiling are exactly the four whose worst frame
passed 400 ms. The six that never reached it all land between 178 and 234 ms.

And the clamped frames do not pay for themselves: 16 steps is 250 ms of
simulation, and those frames cost **322-354 ms of real time**, 29-42% more
than they simulate. Once a run is in that regime the next frame's delta is
clamped to 250 ms again, so it runs 16 steps again, so it costs ~330 ms again.
It is a fixed point, and it held for 4, 17, 34 and 36 frames in the runs that
entered it - a third of a second per frame for up to half a second at a time.

Read on its own that is the candidate's mechanism, and it looks confirmed.
The intervention says otherwise.

### The intervention, and the rejection

`NOVA_PERF_MAX_DELTA = 0.0625` pins the ceiling at four steps - 62.5 ms of
simulation, less than half what the scene needs, so it binds every single
frame. Four captures:

| arm | steps/frame | mean | median | p99 | worst |
|---|---:|---:|---:|---:|---:|
| default (10 runs, median) | 4-16, mean 5.4 | 93.95 | 82.86 | 168.7 | 222.8 |
| pinned at 4 steps, #1 | 4.00 exactly | 132.9 | 127.2 | 248.7 | 293.4 |
| pinned at 4 steps, #2 | 4.00 exactly | 130.9 | 123.6 | 253.7 | 337.2 |
| pinned at 4 steps, #3 | 4.00 exactly | 113.7 | 111.5 | 171.7 | 263.4 |
| pinned at 4 steps, #4 | 4.00 exactly | 171.9 | 150.9 | 394.3 | **600.2** |

**`cap4#4` is the whole answer.** With the ceiling pinned at four steps - at
most 62.5 ms of simulation in any frame, a quarter of the default ceiling -
one frame still cost **600 ms**. A frame that cannot contain more than four
fixed steps and costs 600 ms is not made of fixed steps.

So the association in the table above runs the other way round. A frame that
is slow FOR SOME OTHER REASON hands the next frame a delta clamped to 250 ms,
which is 16 steps, which is why the runs with the deepest tails are the runs
that reach the ceiling. **The ceiling is a symptom of the tail, not its
cause.**

### Verdict: REJECTED

- Capping `max_delta` does NOT bound the tail. Measured: a 600 ms frame with
  the ceiling at four steps.
- The other numbers in the `20260819-123928` ranking were NOT measured through
  an amplifier. Outside the clamped regime the step count is an arithmetic
  consequence of the frame time, and inside it the clamp is following a frame
  that was already slow.
- Capping is not free either, and not in the direction the candidate assumed:
  the pinned arm's mean is 21-83% HIGHER than the default's, because at half
  speed the same 900 frames cover 56 s of fight instead of 71-101 s and sit in
  the dense early brawl rather than running out into the long-range chase.
  `cap4#1` fired 8112 kinetic rounds where `wfc_arena#1` fired 734.
- The pinned arm is no more repeatable than the default one (mean spread 44%
  over four runs against 38% over ten), so it is not a fix for the metric
  either.

`run_fixed_main_schedule`'s 138 ms in the old ranking stands as a measurement
of a slow frame, not as an explanation of one. **What makes the first 250 ms
frame is still unanswered, and it is where the next phase should look.**

The `NOVA_PERF_MAX_DELTA` knob and the fixed-step record stay: they are what
made this answerable, they are what caught the paused result screen in section
1, and the next person to propose this will want the same evidence.


## 4. What changed in the tree

All of it is harness. No gameplay system was added, moved or reordered.

| file | change |
|---|---|
| `crates/nova_probe/src/capabilities/frametime.rs` | counts fixed steps per captured frame (`FixedFirst` tally drained in `Update`, before any early return); `NOVA_PERF_MAX_DELTA` forces `Time<Virtual>::max_delta`, held every frame in `First` before `TimeSystems` |
| `crates/nova_probe/src/stats.rs` | `FixedStepStats` - the per-step-count buckets, JSON-only so the comparable CSV schema does not move - plus `parse_fixed_steps_line`, the scrape the report reads them back through |
| `crates/nova_probe_cli/src/native/cli.rs` | `--repeat <n>` on `run` and `scenario`, refused where there is no frame-time pass to repeat |
| `crates/nova_probe_cli/src/native/run.rs` | the frame-time pass loops, one process and one `fps-run-<i>.log` per repeat |
| `crates/nova_probe_cli/src/evaluation/frames.rs` | the repeat gate: `read_repeats`, `REPEAT_GATE_TOLERANCE`, `repeats_json`, and `fixed_steps_json` |
| `crates/nova_probe_cli/src/evaluation/artifacts.rs` | `fps-run-<n>.log` joins the scanned logs - a repeat that panicked has to reach the log checks |
| `crates/nova_probe_cli/src/report/mod.rs` | the repeat-gate table showing every capture and the gate's call on it, and the fixed-step table |
| `crates/nova_probe_cli/src/evaluation/checks/mod.rs` | `repeats` and `fixed_steps` beside `frames` in `checks.json`, both `graded: false` |

**One defect fixed, found by the coverage sweep.** `perf_capture` took
`Res<State<GameStates>>` as a REQUIRED parameter. A `systems/` rig that wires
`NovaProbePlugin` on a bare `App` - `system_section_severing` and
`system_blast_penetration` both do - has no such state, so arming the capture
failed parameter validation and took the whole run down with a panic and a
core dump. Since the contract is declared by WIRING, `probe run` arms the fps
pass for exactly those two examples, so this was reachable from the front door.
The parameter is now optional and the capture stands down with a warning,
releasing its collector instead of holding the app to the deadline.

`NOVA_PERF_MAX_DELTA` is a MEASUREMENT knob and stays one. Lowering
`max_delta` in a shipping build trades a bounded stutter for simulation time
the world never runs; section 3 says what that costs.

## What Phase B needs before it can rank anything

1. **Bound the 4v4's window.** The capture opens on the scoreboard predicate
   and then runs 900 frames whatever happens; the autopilot script has already
   finished by then (it ends at the screenshot step), so nothing stops the
   window at the end of the match. Two of ten runs measured a paused result
   screen. End the capture when the match ends, or field a roster that cannot
   finish inside the window.
2. **Build the baseline suite on the `stress_*` shape, not on the 4v4.** A
   fixed spawn count with no live fight is the only subject shape measured
   here whose content does not depend on how fast the machine ran it. Keep the
   4v4 as the release's demonstration case; it is not a baseline.
3. **Find what makes the first 250 ms frame.** Section 3 rejects the fixed
   loop as the cause. The candidates that remain (render batching, the
   projectile broad phase) were not measured in this phase.
4. **Fill the targeting hole first.** It is the only subsystem on the epic's
   list that NOTHING in the suite loads, and it is the one whose cost nobody
   can currently argue about in either direction.

## What this phase deliberately did not do

- No fix. Candidates 2-4 (render batching, the projectile broad phase,
  `ThrusterExhaustMaterial`) were not touched and not measured.
- No new `stress_*` case. The coverage table decides which are needed, and it
  is the output of this phase, not an input.
- No release build. Ranking only, as settled.

