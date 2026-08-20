# Running a RANGE without a GPU: the transport, and which ranges survive it

`a47c6247` made `--norender` real but left it unreachable from an example. This
file wires the transport and then answers the question the wiring exists for:
**which `systems/` ranges still prove what they claim when nothing draws.**

The audit is the deliverable. The plumbing is a handful of small edits.

## 1. `NOVA_NORENDER`, and where it is documented

One environment variable, read in ONE place:

```rust
// crates/nova_core/src/lib.rs
pub const NORENDER_ENV: &str = "NOVA_NORENDER";

pub fn new() -> Self {
    Self::assemble(std::env::var_os(NORENDER_ENV).is_none())
}
```

Set to anything, empty included, and every `AppBuilder::new()` in the process
assembles what `AppBuilder::headless()` assembles. `headless()` itself stays
unconditional, so the pair reads "render unless told otherwise" and "never
render".

**Why the environment and not a flag.** There are 48 `AppBuilder::new()` sites
across `examples/` and `nova_perf_web`, and none of them takes a command line.
Per-example flag parsing would have to be added 48 times and remembered every
time an example is added; a variable read inside the constructor cannot drift
and covers examples that do not exist yet. It is also the contract the rest of
the harness already uses - `NOVA_PERF`, `NOVA_AUTOPILOT`, `NOVA_CAPTURE` are all
env-armed and inert unless set.

A constructor whose behaviour an environment variable changes is a trap unless
it says so where it is read, so it is documented on `AppBuilder::new` itself, on
the `AppBuilder` type, on `editor_app` (whose `render: true` also yields to it),
and by contrast on `headless()`. Reader-facing: `docs/development.md`, in the
`--norender` bullet that already owns this concept, with both invocations; and
`docs/performance.md` for the probe flag. `CHANGELOG.md` folds it into the
existing `--norender` entry rather than adding a second, since the two are one
capability arriving at two doors (Changelog rule 2).

`nova_probe` re-exports the constant (`capabilities/frametime.rs`, beside
`PERF_ENV`, which took the same route) so the probe host can push it into a
child without naming the string twice.

## 2. `probe run --norender`

**Not `--render off`.** `--render` picks a BACKEND; `--norender` decides whether
anything draws. Keeping them separate makes the word `norender` mean the same
thing on the game binary, in the environment and on the probe, rather than
inventing a third spelling for one concept. The two are refused together: there
is no backend to pick when nothing draws.

It pushes exactly one variable, `NOVA_NORENDER=1`, and does two other things:

- **It starts no Xvfb.** `run.rs` skips `ensure_display` entirely, and the pass
  env builders push no `DISPLAY` when there is none to push. This is not a
  micro-optimisation: `ensure_display` fails when `:80-:89` are all taken, and
  failing a run that needs no X server for want of an X server would make the
  headline claim false in practice. It also drops a 2 s sleep and a process.
- **It keeps the baseline capture window.** The `sw` arm shortens the window to
  20/120 because lavapipe is glacial. A headless run has no fill cost to shorten
  around, and a shortened window would make its rows incomparable with the
  rendered ones they exist to be read against. The parser refusing
  `--render sw --norender` is what guarantees the short window cannot leak in.

### A defect found on the way, and fixed

`--render sw` was applied to the CLEAN pass only, through `sweep_cell_env`. The
frame-time pass - the pass that writes the number `--render sw` exists to
produce - never got the lavapipe ICD vars and measured the host GPU. So a
`--render sw` run wrote a GPU number into a row labelled as a software floor.

The renderer selection now lives in its own `render_env(render, norender)` and
is applied to every native pass: clean, sweep cells, fps, trace and samply.
`sweep_cell_env` is back to what its name says - scenario, preset, label.

This is a behaviour change to `--render sw`, and it is the behaviour the flag
always claimed.

## 2b. The probe CLI is clap now

Separate commit, owner-requested, landed after the audit so the audit survives
on its own.

`crates/nova_probe_cli/src/native/cli.rs` hand-rolled three `iter.next()` loops
and re-parsed `--samply`, `--correctness-only`, `--release` and `--baseline` in
each. `run` and `scenario` share a `MeasureArgs` now, flattened into both, so
the pair cannot drift - which is exactly how `--render` came to mean different
things in different passes.

**The surface is preserved deliberately.** Same flag names, same defaults, same
positional shapes (a bare `probe run` still parses and lets RESOLUTION print the
catalog), same exit codes. Three places needed explicit work to keep it:

- **A bare `probe` still exits 1.** Clap's instinct for a missing subcommand is
  to render help, and a caller that treats that as success turns a scripted typo
  into a silent pass. `MissingSubcommand` maps back to
  `a subcommand is required`.
- **The retired verbs keep their pointed errors.** `trace`, `sweep`, `web` and
  `profile` are hidden subcommands with `trailing_var_arg`, so a muscle-memory
  command still gets told where it went instead of "unrecognized subcommand". A
  test asserts they stay OUT of the rendered help.
- **`unknown subcommand <x>`** is kept over clap's "unrecognized subcommand".

**What got better.** `--help` is generated from the parser, so it cannot
describe a flag that does not exist - the 40-line `USAGE` constant it replaces
was a second copy maintained by hand. `--render` and `--norender` now conflict
through `conflicts_with` rather than a hand-tracked `render_seen` bool, and
`--repeat` gets its `1..` range from `value_parser` rather than a custom
function. The errors carry clap's usage line, so `native::main` no longer dumps
the whole help behind every refusal.

**One thing worth reporting rather than preserving in silence.** The flag
inventory this conversion was scoped against listed `--nope`, `--fps` and
`--profile`. None of them is a flag. `--nope` is a TEST FIXTURE - the
"an unknown flag is refused" case - and `--fps`/`--profile` are RETIRED flags a
test asserts are still refused. `--scenario-file` is likewise not a probe flag:
it is an argument probe passes to the game binary. The real inventory is 14
flags across three verbs.

## 3. The range audit

**What headless actually removes.** Not "the renderer" in the abstract - a
specific list of `if self.render` branches, all of them in plugin `build`:

| Dropped | Where |
|-|-|
| hanabi (particles) | `nova_gameplay/src/plugin.rs:78` |
| `nova_ui` (widget layer) | `nova_gameplay/src/plugin.rs:96` |
| `nova_hud`, `nova_os_ui` | not added at all (`nova_core/src/lib.rs`) |
| skybox, post-processing | `nova_ship/src/lib.rs:73` |
| section `Mesh3d` + materials (hull, turret, torpedo, controller, thruster) | `nova_ship/src/sections/*` |
| skin plates and decor | `nova_ship/src/sections/shell_skin.rs:1126` |
| damage cracks, sparks, plume, effect fitting | `nova_ship/src/sections/{mod.rs:260,damage_effects.rs:133}` |
| point-defense aim lines (gizmos) | `nova_ship/src/input/point_defense/mod.rs:121` |
| asteroid surface material + render, salvage, beacon, light | `nova_scenario/src/objects/*` |
| render-scale lever | `nova_scenario/src/lib.rs:76` |

Plus, from bevy: no window, so no pointer target, no `Camera::world_to_viewport`
answer, and no winit input of any kind.

That list predicts the audit almost exactly: a range fails headless when it
asserts on something in it, or when it drives the app through a pointer.

### The verdict

`probe run systems --norender --correctness-only`, 25 ranges, one clean pass
each. Every `screenshots/` example is out of scope by definition - it produces
pixels, so it needs a device - and that is the last this file says about them.

**13 run headless and pass every claim they make.**

| Range | Headless |
|-|-|
| `system_attitude_hold` | runs |
| `system_destruction_finale` | runs |
| `bug_carve_apply` | runs |
| `system_turret_gunnery` | runs |
| `system_torpedo_launch` | runs |
| `system_scenario_grammar` | runs |
| `system_player_path` | runs |
| `system_outcomes` | runs |
| `bug_neutralized_quiet` | runs |
| `stress_bullets` | runs |
| `stress_torpedoes` | runs |
| `stress_one_structure` | runs |
| `stress_many_structures` | runs |

**9 cannot**, in three distinct kinds:

| Range | Why | Inherent? |
|-|-|-|
| `system_thrust_and_plume` | asserts the exhaust plume MATERIAL exists; `insert_thruster_shader` is render-gated (`thruster_section.rs:366`) | **inherent** - it is checking a visual |
| `system_hud_indicators` | "the viewfinder must be rendering the moment the lock exists"; `nova_hud` is not added at all | **inherent** - it is checking the HUD |
| `system_nova_os` | stalls on `press Tab`; `nova_os_ui` is not added, so there is no terminal to open | **inherent** - it is checking the cockpit monitor |
| `system_ship_editor` | bevy `ui_layout_system` panics (below) | incidental in cause, inherent in intent - it reads screen positions through `Camera::world_to_viewport`, which has no answer without a window |
| `system_menu_boot` | same bevy panic | as above - it clicks a button at its own screen position |
| `bug_menu_picker` | same bevy panic | as above - and it exists to measure real text layout |
| `bug_sandbox_soak` | same bevy panic | incidental: a soak has no visual claim, it just carries editor UI |
| `system_hull_damage` | `freeze_spawn_com` never fires: it wants two consecutive fixed steps with the rig fully assembled, and headless leaves no fixed step between "assembled" and the first damage | **incidental** - a timing assumption, and a latent one (a fast enough machine breaks it rendered too) |
| `system_borrowed_battery` | the staged torpedo is outside the 150 u envelope by the time the claim is read | **incidental** - a step-boundary race the headless tick rate exposes |

#### The bevy panic four of them share

Four ranges die identically, and not in our code:

```text
thread 'Compute Task Pool' panicked at core/src/num/f32.rs:
min > max, or either was NaN. min = 0.0, max = -12.0
  2: <bevy_ui::ui_node::BorderRadius>::resolve
  3: bevy_ui::layout::ui_layout_system::update_uinode_geometry_recursive
  8: bevy_ui::layout::ui_layout_system
```

With no window there is no `ComputedUiRenderTargetInfo` size, a node resolves to
a negative extent, and `BorderRadius::resolve` clamps against unordered bounds.
This is the third bevy 0.19 headless hole this task has found, after the
`SyncComponentPlugin` despawn panic the previous lane worked around. Unlike that
one there is no public plugin to bolt on: bevy's own headless examples spawn no
UI. **Any range carrying a `Node` with a border radius panics headless**, which
is why the four UI-driving ranges are unreachable rather than merely wrong.
(`system_turret_gunnery` spawns `Node`s and survives - its knob labels carry no
radius.)

**2 are not headless at all, and said nothing about it.**

`system_blast_penetration` and `system_section_severing` build with
`App::new()` + `DefaultPlugins` + `NovaGameplayPlugin { render: true }`,
bypassing `AppBuilder` entirely. `NOVA_NORENDER` never reaches them. They
opened a window and ran on the GPU inside a `--norender` sweep, and passed -
their logs carry `bevy_render::view::window: Couldn't get swap chain texture`,
which nothing headless can print.

**That is the transport's one hole: the variable is exactly as complete as
`AppBuilder`'s reach.** Reported, not fixed - moving those two onto `AppBuilder`
changes what they assemble, which is its own change. (Both already fail
`reached_playing` in any run, headless or not: minimal rigs, no `GameStates`
machine. Their probe verdict is unrelated to this lane.)

### Two findings worth more than the table

**1. `log_clean` failed on EVERY headless run, and now does not.**

`bevy_render::extract_resource` logs at ERROR when a resource wants a render
sub-app that a backendless `RenderPlugin` never built - `<ClearColor>` reaches
every boot. `nova_probe_cli`'s `log_clean` check grades ERROR lines, so all 23
`AppBuilder` runs in the sweep failed it, including the 13 that passed
everything else. A headless mode that can never be green is not a mode.

Fixed at the source: a headless `log_plugin` appends
`bevy_render::extract_resource=off` (`nova_core/src/lib.rs`,
`NORENDER_LOG_FILTER`). Scoped to that one module and to headless boots, so an
ERROR from anywhere else under `bevy_render` still lands and a rendered boot is
untouched. Verified after: `system_player_path` headless is now green on all
eight checks, `log_clean` included.

**2. Headless is NOT the same simulation with the pixels removed.**

`stress_point_defense`, same binary, same seed, same assertions, the two runs
side by side:

| | rendered (Xvfb) | headless |
|-|-|-|
| envelope fill (`open the tubes`) | 9.5 s | 86.3 s |
| torpedoes shot down | 64 | 793 |
| peak rounds in the sky | 1668 | 2418 |

The point-defense chain runs per FRAME. Headless ticks roughly six times
faster, so the battery gets roughly six times the shots per second of
simulated time, and the envelope takes nine times as long to fill. The range
still passes - every count clears its floor - but it measured a DIFFERENT
scene, and it did so 0.4 s inside its own 90 s `FILL_DEADLINE_SECS`. Under the
sweep's load it went over and the range failed.

Those durations are STEP lengths from two pass/fail runs on a loaded box, not
measurements: they are evidence that the two runs simulated different fights,
and they say nothing about what a frame costs either way.

This is the same mechanism behind `system_hull_damage` and
`system_borrowed_battery`, and it is the thing to carry out of this audit:
**anything gated per-frame rather than per-fixed-step changes meaning
headless.** A range that passes headless has not thereby proven it measures the
same thing.


## 4. `stress_point_defense` headless

**It runs, and every claim passes.** Hand-run on a quiet box:

```sh
BEVY_ASSET_ROOT=$PWD NOVA_AUTOPILOT=1 NOVA_NORENDER=1 \
  ./target/debug/examples/stress_point_defense
```

Exit 0. All nine autopilot steps, and each assertion's own line: 12 mounts up
against 12 bays, peak 81 inbound inside the envelope, peak 12 mounts working a
torpedo, 793 torpedoes shot down, peak 2418 rounds and 2622 colliders, the sky
drained to zero, teardown returned to baseline. Then
`autopilot: cycle complete, no panic (t=100.4s)` and
`harness completion: all collectors done, exiting`.

**The capture writes.** Armed with `NOVA_PERF`, the same headless run wrote
`stress_point_defense.json`, `census.json` and a `frametime.csv` row of 900
frames. The row names itself as headless exactly as designed:

```text
label,frames,...,backend,adapter,resolution,...
stress_point_defense,900,...,unknown,unknown,1280x720,...
```

`backend` and `adapter` read `unknown` because there is no adapter to name.
`resolution` is `DEFAULT_RESOLUTION` falling through for a run with no window -
the known limitation the previous lane recorded, unchanged here.

**Through probe it hits its own fill deadline, and that is not a plumbing
failure.** `probe run stress_point_defense --norender --correctness-only` ends
at `step 'open the tubes' stalled after 90.0s`. The hand-run took 86.3 s for the
same step; the probe adds a cold profile sandbox and the box carried two other
lanes throughout. See finding 2 below for why headless makes that step slower
rather than faster. After the log fix the only offending `log_clean` line in
that run is the stall itself - the bevy noise is gone.

## 5. The rendered path is unchanged

Re-run afterwards, same binary, `NOVA_NORENDER` unset, under
`xvfb-run -a -s "-screen 0 1280x720x24"` with `NOVA_AUTOPILOT=1`:

```text
AdapterInfo { name: "NVIDIA GeForce RTX 3060 Ti", ..., backend: Vulkan, ... }
bevy_hanabi::plugin: Initializing Hanabi for GPU adapter NVIDIA GeForce RTX 3060 Ti
```

Exit 0, every assertion passing, real device, hanabi initialised. Nothing in
this lane touches the rendered path: `AppBuilder::new()` with `NOVA_NORENDER`
unset assembles exactly what it assembled before, and `log_plugin` only appends
its directive when `render` is false.

Xvfb was used for pass/fail ONLY. A software X server adds about 13.7 ms a
frame, so no number from that run is comparable with anything.

## 6. What is lost, and the canary set

**A headless run stops catching render-side panics.** The failures that need a
device are exactly the ones this repo keeps hitting: duplicate-component panics,
material and pipeline breakage, and the async-compile SIGSEGV that
`synchronous_pipeline_compilation: true` exists to prevent. `cargo check` sees
none of them. Only a rendered run does.

Finding 2 adds a second loss the original scoping did not have: a range gated
per-FRAME measures a different scene headless. So headless does not only see
less, it can also see something else - and a green headless run is not evidence
that the rendered run would be green.

So headless is a SPEED option and never a replacement. If the suite ever moves,
it must SPLIT, not move. The audit sizes the split for free:

- **The 13 that pass, headless.** That is a little over half the roster, and it
  includes the four `stress_*` cases that dominate the CI wall clock today.
- **The 9 that cannot, rendered** - they have no choice, and between them they
  already cover UI layout, the menu, the editor, the NOVA OS CRT path, the HUD
  and the thruster shader. That is most of a canary set without choosing one.
- **Plus the `screenshots/` producers**, rendered by definition. Free canaries
  for the material and pipeline paths.
- **`stress_point_defense` is the exception to watch.** It passes headless but
  measures a different fight, and it sits 0.4 s inside its own fill deadline. If
  it moves, its `FILL_DEADLINE_SECS` has to move with it, or it becomes a
  load-flaky test.

Nothing here changes CI, and none of it was measured for speed.

### Two defects this audit found that are NOT about headless

Both are real today, neither is in this lane's scope, and both are worth a
tracker entry the owner can decide on:

1. `system_hull_damage`'s `freeze_spawn_com` assumes wall-clock slack between
   the rig finishing assembly and the first damage. There is no such guarantee;
   a fast enough machine breaks it with a device attached.
2. `system_blast_penetration` and `system_section_severing` bypass `AppBuilder`.
   Beyond ignoring `NOVA_NORENDER`, that means they do not assemble the app the
   game ships, which is the reason the shared builder exists.

## Deferred

**No comparative timing was taken.** Two other lanes were on the box for the
whole of this work - one measuring, one building - and the one-minute load
average ran between 5 and 23, mostly in the teens. The frame numbers quoted in
this file are evidence that the capture pipeline works headless, not
measurements of anything, and they must not be
read against a rendered number.
