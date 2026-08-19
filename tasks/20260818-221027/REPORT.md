# What the probe measures, what it costs, and what nobody was looking at

Measurement report for `PERF-HARNESS` (`20260818-221027`), epic
`20260818-220812`.

Every number here was measured on ONE host, in the DEV build profile, on a real
GPU. Read [Measurement conditions](#measurement-conditions) before quoting one.

---

## Read this first

### 1. The probe runs 3 of the 12 scenarios a player can reach

The task was framed as "the probe lacks loads worth profiling". It is worse
than that: the probe does not run most of the game. Twelve scenario ids are
reachable by a player. **Five have no example or probe coverage at all, and
four more are reached only by a random 1-in-4 draw.** The entire shipped
CAMPAIGN - `broadside`, `broadside_gunship`, `lifeline`, `final_tally` - is
unmeasured. The [coverage table](#the-coverage-table) is the accounting, and it
is probably the most useful section of this document.

Any of those four chapters could be at 2 FPS today and nothing in the
repository would say so. The `editor_sandbox` hole is not a one-off; it is the
first one somebody happened to fly.

### 2. `editor_sandbox` cannot be reached by any id-driven rig, and the one example that does reach it stopped measuring before it got there

`editor_sandbox` is registered into `GameScenarios` at editor-Play time
(`crates/nova_editor/src/scenario.rs:203-212`), which is after the
`--scenario <id>` membership check has already run
(`crates/nova_core/src/lib.rs:257-266`). So `nova-protocol --scenario
editor_sandbox` is REFUSED, `probe run scene_baseline --scenario
editor_sandbox` is refused, and the web perf page cannot ask for it either.
Only the editor can load it.

`examples/systems/ship_editor.rs` is the only thing in the tree that does - it
clicks through the real editor and presses Play. It has wired
`nova_probe::NovaProbePlugin::default()`, and therefore a frame-time capture,
the whole time. But the capture opens on `GameStates::Playing`, and **the editor
itself runs inside `Playing`** (`crates/nova_editor/src/lib.rs:107`). Measured,
one run:

```text
21:34:41  nova perf: warm-up done, capturing 900 frames
21:36:27  nova perf: label=ship_editor mean=117.851ms max=2377.557ms
21:36:48  on_load_scenario: loaded scenario 'editor_sandbox'
```

The window closed 21 seconds before the sandbox existed. `probe run
ship_editor` has been writing a `frametime.csv` row labelled `ship_editor`
whose numbers are the editor's BUILD UI, and no reader could tell. That is the
same failure as the 2 FPS one in a quieter form: a number that is real,
repeatable, and about the wrong thing.

Two things follow, and they are separate:

- **The editor's build UI costs a mean of 118 ms and has a 2.38 s frame** in a
  dev build at 1280x720. That is not on the epic's list and it should be.
- `FrameTimePlugin::ready_when` (added by this task) fixes the aim: the capture
  now holds until `CurrentScenario` names `editor_sandbox`. Whether the case is
  USABLE is a different question, and the answer is currently no - see
  [`ship_editor`](#case-ship_editor--editor_sandbox-no-budget).

### 3. `scene_baseline asteroid_field` is not mismeasuring - it measures a scene at rest

Recorded because it was the release's leading suspect for a day, and because
the same blind spot applies to any case built the same way.

The rig reports ~23 ms/frame. It spawns the whole scenario correctly - all 27
objects, real GPU, `player_spaceship` included - and the id is exactly the one
the menu picker lists. Nothing is missing. What it never does is ACT: across a
900-frame window the run logs 37 collision events and exactly ONE carve-field
seed, and that one was the drifting `other_spaceship` bumping a rock:

```text
carve_asteroid_fields: seeded 1681v0 at 60^3 (0.50u cells) in 17.0 ms
```

No weapon is fired in the entire run. `combat_burst_driver` exists for exactly
this and is attached only when `NOVA_PERF_COMBAT` is set, which probe never
sets. The carve path - what `0ee9cbb0` changed and `b61547fc` fixed - is
therefore not on the measured path at all, which is why the case showed no
delta across either commit.

**The general rule this gives the release: a case that does not drive input
measures the scene at rest, and most of this game's cost is downstream of
input.** The stress ranges all drive their own input, which is why their
numbers are alive. `scene_baseline` is the one that does not, and it is the one
that read wrong.

---

## The coverage table

Every scenario a player can reach, against what runs it. Sources: the shipped
inventory at `crates/nova_authoring/src/base_content/scenarios/mod.rs:17-33`,
`crates/nova_editor/src/scenario.rs:23`, the picker filter at
`crates/nova_menu/src/scenarios.rs:106`, and every `LoadScenario` under
`examples/`.

| scenario | reachable how | probe / example coverage |
| --- | --- | --- |
| `shakedown_run` | menu New Game (the default start), picker, direct boot | `menu_boot` walks the menu into it |
| `asteroid_field` | picker, chain from `asteroid_next`, direct boot | `scene_baseline` (its clap default), `render_scale_shot`, `menu_picker`, the web perf page |
| `broadside` | picker, chain from `shakedown_run`, direct boot | **NONE** - `screenshot_ui` and `menu_picker` select its ROW and never load it |
| `broadside_gunship` | campaign header, chain from `broadside`, direct boot | **NONE** |
| `lifeline` | picker, chain from `broadside_gunship`, direct boot | **NONE** |
| `final_tally` | campaign header, chain from `lifeline`, direct boot | **NONE** |
| `asteroid_next` | chain from an `asteroid_field` victory ONLY, direct boot | **NONE** |
| `editor_sandbox` | editor Play ONLY - not direct-bootable | `ship_editor`, which was not measuring it (finding 2) and cannot yet (below) |
| `menu_gauntlet` | menu backdrop draw, chain, direct boot | incidental: a 1-in-4 random draw inside `menu_boot` / `menu_picker` / `screenshot_ui` |
| `menu_weave` | menu backdrop draw, chain, direct boot | incidental, same draw |
| `menu_duel` | menu backdrop draw, chain, direct boot | incidental, same draw |
| `menu_waystation` | menu backdrop draw, chain, direct boot | incidental, same draw |

Two facts the table does not fit:

- **34 of the 40 examples load a Rust FIXTURE rather than shipped content** - 38
  scenario ids that exist only inside an example file. That is correct for a
  correctness range (a fixture is stable and the compiler checks it) and it is
  exactly wrong for a claim about what the game costs, because no player ever
  loads one. Every stress range and `wfc_arena` is in this group: they measure
  real ENGINE cost at a chosen scale, and they measure no scene that ships.
- `examples/screenshots/screenshot_scene.rs:17` says outright that its fixture
  is "the same list, typed out" as shipped content - a hand-copied duplicate
  that can drift from what it copies with nothing to catch it.

**Nothing pins the backdrop draw either.** `NOVA_MENU_BACKDROP` exists and no
example or script sets it, so which of the four menu scenarios a run measures
is a coin toss recorded nowhere.

---

## Measurement conditions

Unless a row says otherwise:

| | |
| --- | --- |
| host | 12th Gen Core i9-12900F, 24 threads, 31 GiB |
| GPU | NVIDIA RTX 3060 Ti, `vulkan`, Xvfb `:77` at 1280x720 |
| build | DEV profile, `--features debug` (`opt-level = 1` first-party, 3 for dependencies) |
| window | forced 1280x720, vsync off, `WinitSettings::game()` |
| capture | 180 warm-up frames discarded, then 900 frames of wall-clock deltas |
| tree | `perf-harness` merged with master `364d5e0a`, so the carve fix `b61547fc` is IN |
| logging | the `debug` feature's default filter (`nova_*=debug`) to a file |

Three caveats that matter more than usual here:

1. **A busy host moves these numbers by 4x.** The same `scene_baseline` binary
   read 22.5-23.0 ms mean on an idle box and **90.9 ms** at a one-minute load
   average of 4.6 - with the MINIMUM frame moving 18.5 -> 80.8 ms. That is not
   noise around a mean, it is a different measurement. Every budget below
   assumes a quiet host; a run on a loaded box will false-fail the budget check
   and must be re-run rather than believed. This box had a second agent
   building on it for most of the session, and it is why the samples below are
   reported as ranges instead of single values.
2. **Dev-profile numbers are not release numbers.** The capture labels them
   `dev` for that reason, and the budget check refuses to grade a `release` row
   against a `dev` budget.
3. **The frame times include debug-level logging and the profile does not.**
   The `debug` feature's filter is `nova_gameplay=debug` and up, which writes a
   line per impact collision per frame - `stress_bullets` produced 12 MB of log
   in one 900-frame window. A shipped build does none of it. Worse, the
   PROFILED pass sets `RUST_LOG=bevy_ecs=info`, which silences the game's
   logging, so the frame-time pass and the top-systems pass are not measuring
   the same program. Pre-existing, unfixed, and listed under [what I could not
   measure](#what-i-could-not-measure).

---

## The ranked list

Most expensive first, by measured frame time on the loads that exist. Read the
right-hand column before reading the number: half of these are engine ceilings
at a scale no content reaches, and saying so is the point.

| # | case | mean ms | worst ms | what it actually measures |
| --: | --- | --: | --: | --- |
| 1 | `stress_torpedoes` | 114-150 | 536-582 | 1000 guided torpedoes from 200 bays. Engine ceiling. No player load resembles it. |
| 2 | `ship_editor` (the editor's BUILD UI) | ~~118~~ | ~~2378~~ | **RETRACTED 2026-08-19 - this row measured a CONTENDED BOX, not the editor.** Quiet host, same binary, same walk: 17.4 ms mean / 16.6 p50. The row's own minimum frame is 83.05 ms, which is a per-frame cost, not a stall. Nor was the window "at rest": it holds 96 autopilot beats, most of the walk. See `tasks/20260819-012130/`. |
| 3 | `stress_one_structure` | 39-44 | 132-145 | One 1000-section hull. Engine ceiling. |
| 4 | `wfc_arena` 4v4 | 62-72 | 123-487 | Eight collapsed hulls fighting, with deaths inside the window. A REAL fight, on a fixture roster no scenario ships. Tail not repeatable. |
| 5 | `stress_many_structures` | 25-34 | 65-145 | 100 hulls of 10 sections. Engine ceiling. |
| 6 | `stress_bullets` | 19-25 | 36-74 | 1000 rounds in flight from 8 mounts. Engine ceiling. |
| 7 | `scene_baseline` (`asteroid_field`) | 20-24 | 40-43 | The shipped sandbox AT REST. **The only shipped scenario on this list, and nothing happens in it.** |
| - | `editor_sandbox` | UNMEASURED | UNMEASURED | The load the owner reports at 2 FPS. No repeatable case exists - see below. |

**The most useful row is the one with no number.** Six of the seven measured
cases are fixtures that no player can load. The seventh is a shipped scenario
measured while idle. The list is honest about cost per case and it does not
answer "what does the game cost", because almost nothing here is the game.

### Against the epic's suspects

The epic blamed the carve field (`FIELD_RESOLUTION_MAX = 64`, 12.7 ms seed +
10.7 ms remesh + 10.0 ms collider, all synchronous). Three corrections, in
order of how much they change the plan:

1. **Section-mesh solidify does not exist.** `crates/nova_gameplay/src/mesh/solidify.rs`
   and `crates/nova_ship/src/sections/damage_carve.rs` are absent from `HEAD`
   and from every ancestor of it; `carve_section_meshes` has zero hits under
   `crates/`. They were deleted by `c1753a3c`, which was squashed into
   `0ee9cbb0`, and that commit's own message records the measurement that
   killed them (325 meshes / 2002.6 ms in one frame). Ship sections keep their
   geometry and grade a shader uniform instead
   (`crates/nova_ship/src/sections/damage_cracks.rs`). The 10-16 ms suspect is
   not slow, it is gone, and the task records that listed it as a suspect were
   corrected while this was being written.
2. **The carve field was not on any measured path.** See finding 3 above:
   `scene_baseline asteroid_field` never fires a weapon, so it seeds one carve
   field per run by accident and none on purpose. The carve fix `b61547fc`
   correctly showed no delta there, and the case's ~23 ms mean is unchanged
   across it (20.0/24.3 ms before the merge, 23.0/22.5 ms after).
3. **A per-frame full-grid rescan was in the carve system and is now gone.**
   Before `b61547fc`, `carve_asteroid_fields` called `field.field.solid_volume()`
   - a full `(n+1)^3` corner scan - on the left-hand side of an `||`, so it ran
   every frame for every rock with any damage mark, whether or not anything
   remeshed. At 64^3 that is ~275k reads per marked rock per frame. Found
   independently here by reading the code; already fixed on master, so it is
   reported as a closed lead rather than a finding.

### The suspect populations, answered

The task asked whether three known-suspect populations cost anything. Two of
the three answers are "the instrument could not see them, and here is why",
which is a finding about the harness, not a clean bill of health.

- **Carve shards.** 2 to 7 entities per accepted mark, not "2+":
  `shard_count(radius) = (radius * 4.0).round().clamp(2, 7)`
  (`crates/nova_gameplay/src/integrity/spew.rs:137`), 2.5 s lifetime (`:93`),
  no collider, `RigidBody::Kinematic`, sharing one mesh and one material handle
  minted once (`:214`). They are spawned by an OBSERVER (`add_observer`,
  `:187`), so they have no `system:` span and never appeared in a top-N table
  at any point in this project's history. This task extended the trace reader
  to count `system_commands` spans, which is where an observer's work actually
  lands - so they are now VISIBLE, but I did not get a trace of a scene that
  sustains hundreds of them (no case shoots rocks). **The pooling question is
  therefore still unanswered, and it is unanswered for a reason worth writing
  down: the profile could not see the population at all until this week.**
- **Section mesh solidify.** Does not exist (above). Not slow, not fast: gone.
- **The mesh slicer at section death.** Still present
  (`crates/nova_gameplay/src/mesh/explode.rs:224`), reached from
  `handle_explosion`, which is also an OBSERVER (`:70`) - same invisibility.
  The scheduled part of that path is
  `nova_gameplay::integrity::explode::spawn_pending_finales` in `PreUpdate`
  (`crates/nova_gameplay/src/integrity/explode.rs:522`, registered `:193`),
  which drains `FINALE_BODY_BUDGET = 8` bodies a frame and builds a convex hull
  per fragment. `wfc_arena` kills ships inside its capture window, so its worst
  frames (123-487 ms) contain deaths - but with no trace of that window I
  cannot attribute them, and I will not guess.

**On pooling specifically**, the honest answer is: no measurement in this
report argues for a pool. Not "pools are unnecessary" - no case in the harness
sustains the churn a pool would address, and the two churning populations
(shards, death fragments) run in observers that produced no profile rows until
the reader was extended this week. The next person should trace a scene that
holds sustained fire on rock before deciding.

---

## Per case

Every case below runs from `probe run`, which does a clean pass, a frame-time
pass and a profiled pass, then writes `report.html` + `checks.json` into
`probe-runs/<short-sha>/<example>/`. The `frame_within_budget` row prints the
worst frame on stdout.

### Case: `scene_baseline` -> `asteroid_field` (budgeted)

```text
cargo run --features debug probe run scene_baseline
```

`asteroid_field` is the rig's clap default, so the bare command IS the sandbox
case. Naming it explicitly (`--scenario asteroid_field`) switches probe to its
sweep path, which relabels the row and skips the dedicated capture pass; prefer
the bare form.

- mean 20.0 / 24.3 / 23.0 / 22.5 ms across four runs
- worst 43.1 / 43.3 / 40.5 / 41.3 ms
- budget **86.6 ms**, twice the worst of the four
- **player-representative: PARTLY.** Shipped content, shipped ids, real GPU -
  and completely idle. It will catch a regression in scene setup, physics at
  rest, gravity and render. It cannot catch anything downstream of firing a
  weapon, which is where this release's regression lived.

### Case: `wfc_arena` 4v4 (NO BUDGET - not repeatable)

```text
cargo run --features debug probe run wfc_arena
```

Changed by this task: a measurement pass (frame-time armed, or a `--features
trace` build) now fields four hulls a side instead of the duel, and the capture
holds until the scoreboard says both teams have FIRED and both have CONNECTED.
Before the gate, the warm-up plus the window landed on the cold approach - the
arena spends 15-25 s closing to weapons-free before a shot is legal - so the
capture measured two lines of ships flying at each other. Verified from the log
of the first run: eight `wfc_fighter_*` spawn, the gate opens 11 s after
`Playing`, and ships die inside the window.

The case is real and it works. It does not get a budget:

| run | mean ms | worst ms | 1% low fps |
| --- | --: | --: | --: |
| 1 | 61.8 | 123.3 | 10.3 |
| 2 | 71.8 | 486.8 | 5.5 |

**The worst frame moved by 4x between two runs of the same binary on the same
seed.** The mean is stable to 16%; the tail is not stable at all, and the tail
is the number a budget is written against. A third and fourth sample were
attempted and both were killed by something outside this session before they
finished, so two is what I have - and two samples 4x apart are already enough
to say "not yet".

Why the tail moves is worth someone's time on its own: the fight is seeded, but
a brawl is not frame-deterministic, so how many ships die inside a given window
- and therefore how many mesh explosions and collider rebuilds land in it -
varies per run. `wfc_arena` is the case most likely to expose the death-frame
cost the epic cares about, and it needs either a longer window (so every run
contains the same number of deaths) or a driver that holds the fight in a
steady state, the way `combat_burst_driver` does for a burst. Until then its
numbers rank, and they do not gate.

### Case: `ship_editor` -> `editor_sandbox` (NO BUDGET)

```text
cargo run --features debug probe run ship_editor
```

Changed by this task: the capture now holds until `CurrentScenario` names
`editor_sandbox`, so the window lands in the range the editor hands off to
rather than in the editor's build UI (finding 2).

**It still cannot carry a budget, and the reason is not the gate.** The editor
walk itself is unstable, at the beat `editor: raise a tower, first course: it
built` (`examples/systems/ship_editor.rs:1155`):

| configuration | runs | failed |
| --- | --: | --: |
| clean pass, no capture at all (code paths untouched by this task) | 3 | 1 |
| capture armed, 1280x720 (probe's default) | 5 | 5 |
| capture armed, pinned to the editor's own 1024x768 | 2 | 2 |

The clean-pass failure is the load-bearing row: with `NOVA_PERF` unset the
frame-time plugin adds nothing at all, so that run is byte-identical to the
example before this task, and it still fails. The flakiness is pre-existing.
Pinning the resolution does not fix it either, which rules out the obvious
suspect.

The likely mechanism, unconfirmed: the walk settles on a FRAME COUNT
(`SETTLE = 10` frames, `SHIP_SETTLE = 40`), and a capture turns vsync off and
sets `WinitSettings::game()`, so a frame is many times shorter in wall-clock
terms and the editor's asynchronous work - collider preparation for a freshly
placed section - has not finished when the next click lands. That would explain
why arming the capture takes it from 1-in-3 to 5-in-5 without the resolution
mattering.

**So `editor_sandbox` has a case and no number.** The gate is in and correct;
the walk has to be made deterministic before anything can be budgeted through
it. That is a bug of its own and it blocks coverage of the scenario the owner
is currently unable to play.

### Cases: the four stress ranges (budgeted)

```text
cargo run --features debug probe run stress_torpedoes
cargo run --features debug probe run stress_one_structure
cargo run --features debug probe run stress_many_structures
cargo run --features debug probe run stress_bullets
```

All four already wired `NovaProbePlugin::default()` and already captured
frames; what they lacked was a recorded number that a regression would break.
Each now has one.

| case | mean ms (runs) | worst ms (runs) | budget | player-representative |
| --- | --- | --- | --: | --- |
| `stress_torpedoes` | 114.1 / 129.9 / 150.1 | 536.1 / 557.8 / 582.2 | 1164.4 | **NO** - 1000 torpedoes from 200 bays is a ceiling, not a fight |
| `stress_one_structure` | 38.6 / 42.5 / 43.9 | 132.5 / 139.4 / 144.5 | 289.0 | **NO** - a 1000-section hull |
| `stress_many_structures` | 25.4 / 30.6 / 33.8 | 65.4 / 81.4 / 144.6 | 289.2 | **NO** - 100 hulls |
| `stress_bullets` | 18.9 / 19.8 / 21.0 / 22.1 / 25.1 | 35.8 / 48.2 / 59.0 / 61.9 / 74.0 | 148.0 | **NO** - 1000 rounds from 8 mounts |

Every budget is twice the worst frame measured. That is what the samples allow,
not caution: `stress_many_structures` moved 65.4 -> 144.6 ms in the tail
between two runs of the same binary, so a tighter gate fires on noise and gets
muted. These catch a DOUBLING of the tail, which is the size of regression this
release exists to stop.

The stress ranges are the healthiest thing in the harness - exact counts, a
drain to zero, a teardown assertion, and they drive their own input so they
measure an active scene. They are also, all four, loads no player will meet.
Treat a regression in one as a signal about the ENGINE, and never as a
statement about how the game runs.

---

## What I could not measure

Named, because "unmeasured" is a finding and "assumed fine" is not.

1. **`editor_sandbox`, the load the owner reports at 2 FPS.** A case exists and
   is correctly aimed; the editor walk that reaches it fails about a third of
   the time with no capture and every time with one (table above). No number.
2. **Any campaign chapter.** `broadside`, `broadside_gunship`, `lifeline`,
   `final_tally` have no example that loads them. Nothing here measures the
   game's actual content.
3. **Carve cost under sustained fire.** No case in the harness shoots a rock on
   purpose. `scene_baseline` seeds exactly one carve field per run, by
   accident. The `carve_asteroids` example does hold PDC fire on one rock and
   is the right seed for this case, but it is a fixture with one rock, not a
   field.
4. **Whether carve shards or death fragments cost anything.** Both spawn in
   observers. The trace reader now counts the `system_commands` spans they land
   in - which it did not before this task - but I have no trace of a scene that
   sustains them.
5. **Per-case top-5 systems.** The profiled pass needs a `--features
   debug,trace` build, and cargo keeps one artifact per target: flipping
   between the frame-time build and the trace build rebuilds the whole Bevy
   graph (~7 minutes) each way, twice per example. I spent the budget on frame
   numbers and coverage instead, and the section is empty rather than guessed.
   `probe run <case>` produces the table into `report.html` for anyone who
   wants it for one case.
6. **Release-profile numbers.** Everything here is `dev`. The budget check
   refuses to grade a `release` row against a `dev` budget rather than pretend
   they compare.
7. **A quiet host.** A second agent was building on this box for most of the
   session. The 4x swing that produced is itself reported above, and it is why
   every number is a range.

---

## What changed in the harness

All of it extends what was already there; nothing parallel was built.

- **`crates/nova_probe_cli/src/evaluation/budgets.rs`** (new) - the recorded
  budgets, keyed by the `frametime.csv` row LABEL, which is the only key both
  capture paths agree on (a bare `probe run <example>` labels the row with the
  example name; a `--scenario <id>` sweep cell labels it with the scenario id).
  Each entry carries the case in words, the literal command, the measured
  numbers it was set from, its justification, and the build profile and GPU
  backend it was measured on.
- **`crates/nova_probe_cli/src/evaluation/checks/frame_within_budget.rs`**
  (new) - the hard gate. Grades `max_ms` against the recorded budget; FAILS the
  run, needing no `--baseline` and no reviewer. A label with no budget is N/A,
  and a row captured on another profile or backend is N/A rather than a
  pass - an absolute millisecond budget does not carry across those, which is
  why one was refused before (`tasks/20260719-112304/TASK.md:79-81`). It sits
  beside `fps_within_baseline`, which stays a soft WARN on the MEAN against a
  previous run: the two answer different questions and neither replaces the
  other.
- **`crates/nova_probe/src/capabilities/frametime.rs`** -
  `FrameTimePlugin::ready_when(|&World| -> bool)`. The capture holds in its
  wait phase until the example's own predicate holds, so the window lands on
  the load the example names instead of on whatever the scene was doing when it
  reached `Playing`. Watched by a READ-ONLY system, deliberately: an exclusive
  system would put a command-flush barrier in the middle of `Update` for every
  armed capture, and a measurement may not reorder the schedule it measures.
  Nothing is scheduled at all unless an example names a gate.
- **`crates/nova_probe_cli/src/evaluation/profile.rs`** - the chrome-trace
  reader now aggregates `system_commands` spans beside `system` spans, tagged
  `(commands)` in the table. This is the one that changes what the profile can
  SEE: an observer (`add_observer`) and a `commands.queue(|world| ...)` closure
  have no span of their own and run inside the flush of whoever spawned them.
  Counting only system bodies reported ZERO for carve-shard spawning, for mesh
  slicing at death, and for every other observer in the codebase - which reads
  as "cheap" and means "invisible".
- **`examples/screenshots/wfc_arena.rs`** - captures now, fields 4v4 under a
  measurement pass, gated on the fight being joined.
- **`examples/systems/ship_editor.rs`** - the capture is gated on
  `CurrentScenario` naming `editor_sandbox`.

### How to re-run everything here

```text
# the budgeted cases, each printing its worst frame
cargo run --features debug probe run stress_torpedoes
cargo run --features debug probe run stress_one_structure
cargo run --features debug probe run stress_many_structures
cargo run --features debug probe run stress_bullets
cargo run --features debug probe run scene_baseline      # = asteroid_field

# measured, unbudgeted
cargo run --features debug probe run wfc_arena           # 4v4 under a capture
cargo run --features debug probe run ship_editor         # editor_sandbox; flaky walk

# all of them, with the aggregate index
cargo run --features debug probe run stress_torpedoes,stress_one_structure,stress_many_structures,stress_bullets,scene_baseline,wfc_arena
```

Each writes `report.html`, `checks.json` and `frametime.csv` into
`probe-runs/<short-sha>/<example>/`. `checks.json` carries
`frame_within_budget` with the worst frame, the budget, the case and the
command in its `data` object.

---

## What the release should do with this

In the order I would do it.

1. **Make the `editor_sandbox` case work.** The gate is in; the editor walk has
   to stop failing under a capture. Until then the scenario the owner cannot
   play has no standing measurement, and that is the single biggest hole.
2. **Give the campaign a case.** Four shipped chapters, zero coverage. One rig
   that boots a named chapter and captures would close all four, and
   `scene_baseline` already does exactly that for any id in `GameScenarios` -
   the work is choosing where the capture window sits in a scripted mission,
   not writing a new rig.
3. **Give `asteroid_field` a case that FIRES.** `NOVA_PERF_COMBAT=1` exists and
   attaches `combat_burst_driver`; nothing sets it. A sandbox case that holds
   the trigger is the one that would have caught `0ee9cbb0` and it is close to
   free.
4. **Decide whether the editor's build UI at a 2.38 s frame is acceptable.** It
   is a player-facing surface with a worse tail than any stress range, and it
   is not on the epic's list.
5. **Trace a scene under sustained fire before arguing about pooling.** The
   populations are visible in the profile now; nothing has looked at them yet.
