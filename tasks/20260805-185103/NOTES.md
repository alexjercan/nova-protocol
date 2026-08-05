# NOTES - what the investigation established

Read-only investigation, 2026-08-05. Three rounds: an initial six-lane parallel
review, a revision after owner pushback, and a correction round driven by two
out-of-context verification agents whose findings were then re-verified by hand.
Scratchpad artifacts lived in `/tmp/nova-invest/` and are NOT durable - this file
is the record.

Every `file:line` below was checked against the tree at HEAD on 2026-08-05.
Line references from earlier rounds ran 2-3 low; these are the corrected ones.
bcs is pinned at `v0.19.5` (`Cargo.lock:817-819`) while the bcs working tree is
at `v0.19.6-6-g127f311` - verify bcs claims with `git show v0.19.5:<path>`.

## Problem Statement

The screenshot and autopilot tooling is hacky, and the v0.10.0 task set consumes
the engine to produce images rather than improving it. The concrete symptom: the
first `pose` call in `screenshot_combat` makes the camera flicker.

What this is NOT: a rewrite, a bcs divorce, or a demand that v0.10.0 change
direction. The epic is self-consistent and consumer-shaped by design.

## The thesis

The examples are not hacky from laziness. **Each capture need hit a missing
engine capability and routed around it instead of the engine growing one.** The
flicker proves it: `pose()` is correct - it inserts a `ScriptedCameraPose`
marker and uses the supported path. What is unfinished is the engine's camera
ordering contract.

Corollary, visible in the git history: the sprint's only two genuine engine
fixes - the shared exit describer (`crates/nova_autopilot/src/exit.rs:27`,
`f3153cf7`) and `synchronous_pipeline_compilation: true`
(`crates/nova_core/src/lib.rs:223`, `64f5eb21`, *"0 failures in 60 runs with 0
kernel segfaults against a 2/20 baseline"*) - exist ONLY because the screenshot
automation kept dying. The engine improved as a side effect of trying to
photograph it.

## Findings, by area

### The flicker (the highest value-to-effort item)

The camera `Transform` has four to five independent writers: bcs chase sync, bcs
WASD sync, bcs shake `Restore` and `Apply`, and nova's
`enforce_scripted_camera_pose`. The ordering lattice is PARTIAL - seven edges
exist, the rest is executor readiness, i.e. a per-frame coin flip.

The specific hole: `enforce_scripted_camera_pose`
(`crates/nova_scenario/src/loader/mod.rs:376-379`) carries ONLY
`.after(WASDCameraSystems::Sync)`. But
`crates/nova_scenario/src/loader/lifecycle.rs:347-360` swaps the player to
`SpaceshipCameraController` on spawn, so the writer that actually runs is
CHASE - against which the enforcer is unordered. `grep -rn ChaseCameraSystems
crates/nova_scenario/` returns nothing.

The engine diagnoses this exact class eight lines from the bug, at
`crates/nova_gameplay/src/camera_controller/mod.rs:106-114`, and fixes one edge
of it.

Second, previously-unnamed gap: **neither `CameraShakeSystems::Apply` nor
`enforce_scripted_camera_pose` has an edge to `TransformSystems::Propagate`.**

Two things that look like writers and are not:
- `LegCamera` (`examples/screenshots/screenshot_flight.rs:696`, driver `:711`)
  runs in `Update` and calls `pose()` -> `reel_pose_camera`
  (`crates/nova_debug/src/harness.rs:468-481`), which removes
  `WASDCameraController` and inserts `ScriptedCameraPose`. It FEEDS the enforcer.
- `update_camera_rig` (`camera_controller/framing.rs:178-191`) writes
  `ChaseCamera`, not `Transform`.

One subtlety worth keeping: bcs's WASD sync query is
`Query<(&mut Transform, &WASDCameraState), Changed<WASDCameraState>>` with **no**
`With<WASDCameraController>` filter (`bcs/src/camera/wasd.rs:209`), which is why
removing the controller does not stop it - `loader/mod.rs:371-374` already says
this.

### The five missing engine capabilities

| capability | shape | what it deletes |
| --- | --- | --- |
| authorable lighting (owner already working it, `20260805-111534`) | `lighting:` block in the scenario | per-example photo rigs: `pin()` at `screenshot_flight.rs:662-665`, `KEY_FROM` `:671`, `lit_side()` `:673-682` |
| photo-mode freeze | a state pausing fixed-update + AI + despawn | `pin()`'s `world.remove_resource::<LegCamera>()`, most scene settle |
| scoped scenario-entity index | see the caveat below | ~7 hand-rolled linear scans |
| capture-complete ack | per-shot signal; `until(shot_written(name))` | the save-latency settle in all four capture examples |
| action-level input | intents with observed preconditions, replacing raw synthesized events in `crates/nova_autopilot/src/input.rs` | the click/wheel race cluster |

Two of the five SUBTRACT code rather than adding API.

**The index caveat.** A flat `HashMap<String, Entity>` does not work.
`crates/nova_scenario/src/actions/spawn.rs:36-39` states that spaceship SECTIONS
carry their own `EntityId`s (`controller`, etc.) and *"an unscoped match on such
an id would rip that section out of every ship in the scene"*. There are five
insertion sites, not one: `actions/spawn.rs:101` (base), `:319` (trigger areas),
`objects/spaceship.rs:338` (sections), `:442` (well refs), plus salvage. This is
also why existing code needs an 8-deep parent walk - the very hack the index was
meant to delete. Key it scoped or skip it.

Actual scan sites: `screenshot_combat.rs:1068, 1133, 1296, 1311, 1324`;
`screenshot_flight.rs:811`. Note `screenshot_combat.rs:980` is a direct
`world.get::<EntityId>()`, not a scan, and `screenshot_ui.rs:182` scans `&Name` -
an `EntityId`-keyed index would not delete it.

### Smoke and capture are different runs of the same file

`NOVA_REEL` changes step TIMING, not just whether a PNG is written:

| file:line | values (capturing / not) |
| --- | --- |
| `examples/screenshots/screenshot_combat.rs:167` | 20 / 2 |
| `examples/screenshots/screenshot_flight.rs:161` | 20 / 2 |
| `examples/screenshots/screenshot_nova_os.rs:57` | scene 40 / 6; `after_capture` 20 / 2 at `:58` |
| `examples/screenshots/screenshot_ui.rs:56` | scene 90 / 6, `after_capture` 20 / 0, `after_nav` 30 / 6 |

CI smokes the short path; images render on the long path. A timing-dependent bug
on the long path is invisible to CI by construction - and the flicker is exactly
a timing-dependent bug.

Caveat: in combat and flight `capture_settle_frames` has ONE call site each
(`combat:526`, `flight:469`) - the final capture step. In `nova_os` and `ui` the
settle threads through many steps. Real in all four; much larger surface in two.

**Two settles wear one flag.** `screenshot_combat.rs:161-165` states the first
verbatim: *"`capture_window` spawns a bare `Screenshot` and is NOT a completion
collector, so the last step's hold is the only thing giving `save_to_disk` room
to land before the driver reports done and the app exits."* Confirmed at
`crates/nova_autopilot/src/reel.rs:251-257`. **A missing await, not a duration.**
The second is scene convergence (`screenshot_ui.rs`'s 90 vs 6), which is a real
wait and should simply be uniform.

### Three implementations of "did the run pass"

1. `tests/examples_smoke.rs` - stderr assertions
2. `crates/nova_probe/src/run_report/checks.rs` - timeline JSON
3. `crates/nova_autopilot/tests/autopilot_example.rs:37-90` - hand-rolled, with
   `fn tail` duplicated byte-for-byte from `examples_smoke.rs:368`

They disagree on facts. The resolution settled with the owner is to converge
them onto probe and delete the other two - but probe does NOT currently cover
what smoke covers, and the gaps are specific:

- `crates/nova_probe/src/catalog.rs:181-188` sets `screenshots` to
  `probed: false` **by design** (*"Outside probe's scope: a probe verdict on one
  would assert nothing"*); `bin/probe/native/spec.rs:112-118` makes
  `probe run screenshots` an ERROR. Smoke covers all six (`examples_smoke.rs:71-78`).
- Smoke fails on stderr containing `"Encountered an error in command"`
  (`:350-359`, task `20260713-203709`) because `remove`/`despawn` bake in the
  **WARN** handler at queue time (`:338-346`). Probe's `log_clean` catches only
  `"panicked at"` or a whole-word `ERROR`. **A WARN-level command error passes
  probe clean.**
- Probe's `reached_playing` (`checks.rs:279-315`) reads `timeline.jsonl` and
  returns `Skipped` when absent - and `checks.rs:36-46` says *"`Skipped` means
  the input artifact was not captured, not that the property held."* Only 17 of
  24 examples wire `nova_timeline`, **none of the six screenshots**.

Also: `scene_baseline` is in `NOT_SMOKED` (`examples_smoke.rs:88`) precisely
because probe owns it - the suites are complementary by design, not nested. And
CI has **no probe step today** (`.github/workflows/ci.yaml:95-101` runs smoke).

Related bug found in the review round, never in any report draft:
`ProbeTimeline::create` uses truncating `File::create`
(`crates/nova_probe/src/recorder.rs:201`) with no singleton guard while an
earlier instance's `BufWriter` holds its own offset. Two recorders, one path, a
torn `timeline.jsonl` - which undermines making the timeline the sole verdict.

### The bcs boundary - the hypothesis was inverted

**Nova does NOT meaningfully hack bcs.** No `#[allow]`, newtype or shim exists to
make a bcs type fit. The modding scaffold is clean, as the owner believed. The
real problems run the other way:

- **bcs holds nova's code.** `bcs/src/integrity/plugin.rs:31-33` carries exactly
  three magic combat-feel constants (`RESTITUTION_COEFFICIENT = 0.5`,
  `IMPULSE_DAMAGE_MODIFIER = 0.1`, `ENERGY_DAMAGE_MODIFIER = 0.05`).
  `bcs/src/ui/health_display.rs` is written in ship-section vocabulary and has
  ZERO hits in nova's `crates/` and `examples/`. `bcs/src/physics/pd_controller.rs:535`
  cites nova task IDs in its tests.
- **Nova re-implemented bcs `persist` twice.** `crates/nova_assets/src/mod_prefs.rs:11`
  (*"third-party persistence crate would be a version-compat liability for a UI
  feature"*) and `crates/nova_menu/src/settings_store.rs:12` (*"a third-party
  settings crate would be a version-compat liability"*). bcs is first-party and
  version-pinned, so the stated reason does not apply, and neither file mentions
  bcs. Same class: `feedback::flash` shadowed at
  `crates/nova_gameplay/src/juice.rs:275`, and `time::Cooldown` hand-rolled as
  `Timer::from_seconds(.., Once)` in every firing system.
- **Nova's prelude leaks bcs's retired harness twins.**
  `crates/nova_gameplay/src/lib.rs:70` is the ONLY
  `pub use bevy_common_systems::prelude::*` in the workspace - the other 35
  sites are private `use` and leak nothing. Through it, a bare
  `AutopilotPlugin::new()` compiles but arms on a retired bcs env var and boots
  the example INERT. That is how `playable` broke (task `20260802-183403`), and
  `examples_name_drivers_through_the_nova_harness` (`examples_smoke.rs:227`)
  exists solely to guard it.

**The boundary rule, twice-corrected.** "Safe iff it exports a `SystemSet`" is
wrong - the set is table stakes, not a differentiator:

| bcs module | writes shared state | schedule opinion | exports a set |
| --- | --- | --- | --- |
| `meth` (math) | no | no - no plugin, no systems | n/a |
| orbit transform | no | **yes** - `src/transform/sphere_orbit.rs:59` Plugin, `:65-66` `PostUpdate` + observer | yes (`:49`) |
| PD controller | no | **yes** - `src/physics/pd_controller.rs:44` Plugin, `:50-51` `FixedUpdate` | yes (`:37`) |
| persist | no | **yes** - `src/persist/mod.rs:103-104` `Update` + `PreStartup` | yes (`:53`) |
| chase camera | yes | yes | yes |
| WASD camera | yes | yes | yes |
| camera shake | yes | yes | **yes** (`src/camera/shake.rs:166`) |
| integrity | yes | yes | yes (`integrity/plugin.rs:36`) |
| health_display | yes | yes | yes (`ui/health_display.rs:64`) |
| modding scaffold | no | no | n/a |

Only `meth` and the modding scaffold are opinion-free. What actually holds:

> **(a) Order, don't disable.** Ordering needs only the exported set, costs one
> redundant write per frame, and survives bcs adding a new writer - it lands
> inside `Solve` and is still overwritten. A gate breaks silently when that
> happens. (`run_if` on a bcs set is legitimate for turning a whole FEATURE off,
> e.g. photo-mode freeze - just not for arbitrating two writers.)
>
> **(b) Import behavior, not presentation, and never a renderer you will not
> use.**

Rule (b) comes from the closest thing to a real bcs hack in the tree: nova adds
bcs's objectives plugin purely for its Resource, discards its renderer
(`crates/nova_gameplay/src/hud/mod.rs:293-298`), then hand-diffs `GameObjectives`
(`crates/nova_scenario/src/world.rs:52-72`) to dodge a per-frame
despawn/respawn in `bcs/src/ui/objectives.rs:104-107`. That conflict is
change-detection and renderer ownership - **no SystemSet fixes it.** Second
support: removing a bcs marker is the only "off switch", and doing so destroys
and rebuilds the entire `bevy_enhanced_input` action tree
(`bcs/src/helpers/wasd.rs:86-149`).

One caveat on the otherwise-clean modding scaffold: bcs's `modding::registry` is
494 lines and nova ships a typed RON answer instead, so
`EntityHandler::from_event_name` exists solely for a registry nova does not use.
Clean boundary, unused half.

### The step machine's failure mode - much narrower than first thought

`crates/nova_autopilot/src/autopilot.rs:467-484` ALREADY aborts on deadline
expiry: `error!("autopilot: step \`{}\` stalled after {step_elapsed:.1}s ...")` +
`AppExit::error()`, index not incremented, with a NOTE comment reading *"an
expired step is an ABORT, not a completion"*. The builder doc at `:290-292`
agrees. What remains:

1. A step with **no** deadline and an unsatisfiable `until` hangs forever. Both
   subprocess harnesses block on `Command::output()` with no timeout (task
   `20260803-094601`; call site `autopilot_example.rs:167`), so that is a silent
   60-minute CI hang. Make `deadline` mandatory or defaulted.
2. `StepBuilder`'s **5** setters (`autopilot.rs:263-293`) are last-write-wins.
   `debug_assert!` on double-set. Task `20260805-015136` names only `on_enter` -
   and also names a `ScriptBuilder` that does not exist.
3. `autopilot_drive` hand-rolls `resource_scope` across six return paths while
   `reel_drive` uses the real one.

### Size and speculation

- `crates/nova_gameplay/src/hud/mod.rs`: 1463 lines, 60 commits - the highest
  churn in the workspace. `hud/` overall is 33,822 lines across 58 files, ~25%
  of game-crate code. The repeated setup/remove pairs are a registry wearing a
  copy-paste costume.
- `nova_gameplay`'s prelude: a large majority of exports has no external
  consumer. `hud/mod.rs` glob-re-exports 28 sub-preludes, which is why the count
  is unbounded and why `20260731-205553` is stalled.
- Confirmed speculative: `with_main_menu` (`crates/nova_core/src/lib.rs:132`) has
  **0** callers; `AIBehaviorState::Retreat` (`input/ai/behavior.rs:46`) is never
  constructed in production and stubs via `engages()` (`:55`);
  `nova_probe/src/bin/probe/native/web.rs` (239 lines, reachable only via
  `--platform web`), `perf_web.rs`, `--samply`, retired-verb branches, four dead
  `debug` feature flags.
- `base_scenario_object` (`crates/nova_scenario/src/actions/spawn.rs:96-114`)
  adds `RigidBody::Dynamic` (`:103`) + `TransformInterpolation` (`:110`) to every
  object. **Three of four kinds override Dynamic**: `objects/beacon.rs:69`,
  `objects/salvage.rs:72`, `objects/asteroid.rs:280` (conditionally, comment at
  `:252-255`). Only spaceship inherits it as intended.
  `TransformInterpolation` is removed NOWHERE - dead weight on every static
  beacon and crate, though not a live bug (`hud/beacon_chips.rs:234` reads only
  `GlobalTransform`). The blocker to fixing it is
  `scenario_objects_interpolate_their_transforms` (`spawn.rs:772-786`), whose
  docstring says "dynamic" but which asserts on the BASE bundle that static
  kinds also use; the better behavioral pin is
  `examples/sections/hull_section.rs:867-868`.

### Testing shape

The suspected "testing documentation" category is real but SMALL: one WGSL
source-text test (pins a genuine invariant - KEEP), three tests asserting Bevy's
log wording, and `sections_assert_their_invariant_roster`
(`examples_smoke.rs:468`) - `SECTION_INVARIANTS: usize = 27` plus a
hand-maintained 27-slug roster matched against source TEXT, whose own docstring
concedes *"this only pins WHICH"* (`:460`). Nothing asserts on doc prose
(`tools/nova_meta_gen/tests/generate.rs:98,122` assert on paths in a synthetic
fixture).

Going-forward rule: **no test whose subject is source text or log text.**

## What the investigation did NOT find

This constrains what is worth doing.

- No `*_legacy` / `*_old` / `*_v2` filenames. One damage path, one spawn path,
  one camera path. (`CSV_HEADER_V1/V2` at `nova_probe/src/stats.rs:137` are
  deliberate format versions.)
- All declared code-map boundaries hold, except an undocumented
  `nova_debug -> nova_autopilot` edge.
- 8 `#[allow(dead_code)]`; 9 TODOs, all carrying task IDs.
- No bcs version drift.
- No test asserts on doc prose.

## Counts to distrust

The numbers drifted between rounds; the shapes held. Do not re-cite these
without recounting:

- "253 of 477 prelude exports" - actual is ~222 of ~387.
- "8 zero-caller items in the automation crates" - did not reproduce under three
  passes. What exists is ~79 items with no CROSS-crate reference, i.e. internal
  API.
- "27 env vars" - 24 unique `NOVA_*` names.
- "11 setup/remove pairs in `hud/mod.rs`" - never enumerated.
- `JuiceSettings` has **5** fields, not 14, and is NOT dead - read every frame
  (`juice.rs:434-437, 451, 515, 548, 579`) and `#[reflect(Resource)]`-registered
  at `:372`, i.e. editor-tunable.
- `AIBehaviorState::Retreat` appears in **2** test functions, not 3, and none
  asserts the stub.
- `fps_within_baseline` (`checks.rs:396-470`) skips when no baseline exists -
  under the stated policy that is always, but it is a property of usage.

## Errors made and corrected - do not re-derive

Each was verified against source after being flagged by an out-of-context agent:

- **`ScreenshotReelPlugin` is NOT unused.** Two examples reach it through
  `nova_reel()` (`crates/nova_debug/src/harness.rs:372-392`, which CONSTRUCTS it,
  not a re-export): `examples/screenshots/screenshot_sections.rs:40` and
  `screenshot_scene.rs:77`. A grep for the bcs symbol misses the wrapper.
- **bcs's camera shake already exports a set and is already ordered.**
  `bcs/src/camera/shake.rs:166` declares `CameraShakeSystems{Restore, Apply}`,
  it is preluded at `:69`, and `:205-209` orders `Restore.before(Chase::Sync)` /
  `Apply.after(Chase::Sync)`. No bcs change is needed for the camera fix.
- **Probe refuses `screenshots/` BY DESIGN.** Deleting the smoke suite without
  changing that design silently drops six examples from CI.
- **Deadline expiry already aborts.** See above.
- **`log_clean` is not a substring grep.** `checks.rs:472-491` strips ANSI then
  matches `ERROR` as a whole token, with a comment saying substring matching was
  the previous, broken version. And conflating a panic with a wgpu teardown race
  is a false POSITIVE, not "green and wrong". Keep it.

## Scope context - v0.10.0

Supporting the owner's original complaint, with a caveat.

Of 33 `v0.10.0`-tagged tasks: 3 touch the shipping game, 12 are automation
architecture, 11 mixed, 7 pure consumer. The Aug 2-3 `nova_autopilot`
extraction is real architecture and worth defending. Of what remains OPEN
(excluding the epic tracker): 4 consumer vs 2 engine.

**The tell is the priority ordering:** both open engine tasks sit at 50
(`20260731-205553`) and 66 (`20260805-111534`) while the image refresh
(`20260805-105154`) sits at 70. This task is slotted at 72 on that basis.

Tag histogram, showing this is a phase change and not a blip:

- Jul 22-28 (v0.9.0): ui 62, hud 53, feature 46, scenario 31 - engine-dominant.
- Jul 29 - Aug 5: `tooling+testing+examples+autopilot+probe = 64` vs
  `gameplay = 4`; `hud` collapses 53 -> 11.
- The v0.10.0 portion contains **zero** tasks tagged `gameplay`, `hud`,
  `weapons`, `audio`, `modding` or `ai`.

The epic is self-consistent - its Out of Scope list is where the engine work
went (in-editor scenario authoring, gamepad/mobile input, further crate cleanup,
golden-image tests, a serialized DSL), all deferred as *"unrelated to the
automated demonstration dependency chain."*

Two tracker bugs found in passing, both outside this task's scope but worth
fixing: `20260804-095507` is falsely blocked (`TASK.md:51-52` blocks on
`20260805-091151`, which is DONE and landed in `87bcb956`), and
`20260730-111146/TASK.md:9` declares `DEPENDS ON: 20260724-082856`, which is
`SUPERSEDED` / `DUPLICATE OF: 20260805-105154`.

## Decided out of scope

- **Storing perf baselines in the repo.** `git checkout <tag>` + re-run is the
  policy; `probe-runs/` stays gitignored (`.gitignore:252`). Residual: delete
  `fps_within_baseline`, and make perf `nova probe --compare <run-dir>` run by
  hand. Also `CheckStatus::Warn` must stop being a category that never fails
  (`checks.rs:36`, `:393`).
- **Removing god mode from perf capture.** A mix is wanted. Residual:
  `crates/nova_probe/src/capture.rs:574-584` force-heals every `Health`
  UNCONDITIONALLY, so a scenario measuring death/despawn cost is
  unconstructible. Note the fix cannot live in `CATEGORY_POLICIES` (keyed by
  category, `catalog.rs:144`) - the driver is selected by `NOVA_PERF_COMBAT`
  (`scene_baseline.rs:95`).
- **Promoting `nova_autopilot` out of nova.** Per-game autopilot until the
  design is understood well enough to promote. Nova's is already the better
  design (predicate-driven vs bcs's frame-driven).
- **The v0.10.0 epic's direction.** Not this task's business.
