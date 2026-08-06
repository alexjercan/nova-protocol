# Cleanup and maintenance: close the engine gaps the screenshot pipeline routed around

- PRIORITY: 72
- TAGS: v0.10.0, chore, refactor, tooling, testing
- ACTIVITY: WORKING
- GATES: PLAN
- RESOLUTION: -
- PARENT: 20260802-115955

## Context

A session-long read-only investigation (three rounds, two out-of-context
verification agents) reviewed the screenshot/autopilot tooling, the game crates
and the `bevy_common_systems` boundary against KISS/YAGNI. `NOTES.md` holds the
full finding set with `file:line` evidence and the corrections each round made
to the last; this task is the parent that sequences the work.

The thesis: the screenshot examples are not hacky from laziness. Each capture
need hit a missing engine capability and routed around it. The flicker the owner
reported in `screenshot_combat`'s first `pose` is the proof - `pose()` is
correct, and the engine's camera ordering contract is what is unfinished.

Nova does NOT meaningfully hack bcs - no `#[allow]`, newtype or shim exists to
make a bcs type fit. The real boundary problems are inverted from the
hypothesis: bcs holds nova's code (`integrity`'s three combat constants,
`ui/health_display.rs` in ship-section vocabulary nova never uses), nova
re-implemented bcs `persist` twice, and nova's own prelude leaks bcs's retired
harness twins into every example.

This is a CLEANUP AND MAINTENANCE parent, deliberately slotted above the
in-flight PNG refresh (priority 70). The investigation's own scope finding is
that v0.10.0's two open engine tasks sit at priorities 50 and 66 while an image
task sits at 70 - the ordering encodes the problem it is trying to fix.

## Inputs

| Input | Where | Note |
| --- | --- | --- |
| Full finding set | `NOTES.md` in this task | Round-3 corrected; supersedes rounds 1-2 |
| The flicker | `crates/nova_scenario/src/loader/mod.rs:376-402` | one ordering edge, against the wrong writer |
| The fix pattern, already in-tree | `crates/nova_gameplay/src/camera_controller/mod.rs:106-114` | diagnoses this exact class eight lines from the bug |
| bcs pin | `Cargo.lock:817-819` (`v0.19.5`) | the bcs WORKING TREE is at `v0.19.6-6-g127f311`; verify bcs claims with `git show v0.19.5:<path>`, not the checkout |
| Owner decisions | `NOTES.md` "Owner rulings" | two round-3 conclusions were reversed by the owner on stated conditions |

## Steps

Ordering rationale for the two non-obvious constraints: step 4 wires
`nova_timeline` into six screenshot examples, two of which step 3 rewrites -
reverse them and both files get edited twice. And
`examples_name_drivers_through_the_nova_harness` cannot be deleted while it
still has a subject, so step 2 precedes step 6.

Each step below becomes its own child task at planning time.

- [x] 1. **Fix the torn `timeline.jsonl`.** `crates/nova_probe/src/recorder.rs:201`
      uses truncating `File::create` with no singleton guard while an earlier
      instance's `BufWriter` holds its own offset. Two recorders, one path.
      Small, and load-bearing for steps 4-6, which make the timeline the sole
      verdict.
      commits: `faa4011d`
- [x] 2. **Stop leaking the bcs prelude.**
      `crates/nova_gameplay/src/lib.rs:70` is the ONLY
      `pub use bevy_common_systems::prelude::*` in the workspace (the other 35
      sites are private `use` and stay). Delete it; compile; add back only what
      breaks as explicit named re-exports. Explicitly do NOT re-export
      `AutopilotPlugin`, `AutopilotLoop`, `ScreenshotPlugin`,
      `ScreenshotReelPlugin`, `HarnessCompletion` - the inert twins that boot an
      example dead, and exactly the `DRIVERS` list at
      `tests/examples_smoke.rs:233-239`. Then delete
      `examples_name_drivers_through_the_nova_harness` (`:227`). Front half of
      the `20260731-205553` warning cleanup: the prelude is why that task is
      stalled.
      commits: `26bc29e0`
- [x] 3. **One capture idiom: promote `shoot`, delete the reel.** Promote
      `shoot` to `nova_debug::harness::shoot` (collapses three copies:
      `screenshot_flight.rs:646`, `screenshot_nova_os.rs:282`,
      `screenshot_combat.rs:856`); fold `examples/ui/widget_zoo.rs:605` and
      `examples/ui/menu_scenarios.rs:267` onto it, deleting the duplicated
      `NOVA_SHOT_DIR` resolution (also duplicated in a SHIPPING crate at
      `crates/nova_scenario/src/actions/view.rs:70`); **convert
      `examples/screenshots/screenshot_sections.rs:40` and
      `screenshot_scene.rs:77` from `nova_reel(beats)` to autopilot steps** -
      this is the real cost, two files rewritten; then delete
      `crates/nova_autopilot/src/reel.rs`, `ReelBeat`, `ScreenshotReelPlugin`,
      `crates/nova_autopilot/tests/reel.rs` and `nova_reel()`
      (`crates/nova_debug/src/harness.rs:372-392`). **Keep `capture_window`** -
      it is the primitive `shoot` wraps.
      commits: `63cc7bd2`
- [ ] 4. **Capture ack + one uniform scene settle.** `capture_window` becomes a
      completion collector / emits a per-shot ack; steps use
      `until(shot_written(name))`. That deletes the save-latency settle outright
      - it was never a duration, it was a missing await
      (`screenshot_combat.rs:161-165` says so). Then ONE scene-settle value on
      both paths, replacing the 90/6, 40/6 and 20/2 splits. Also makes the
      `FIGURES` manifest at `scripts/gen-web-screenshots.py:74-105` checkable.
- [ ] 5. **Make probe cover the `screenshots/` category.**
      `crates/nova_probe/src/catalog.rs:181-188` -> `probed: true,
      frame_time: false`, and rewrite the comment (its claim is true only of
      frame-time). **Wire `nova_timeline` into all six screenshot examples** -
      none has it today, and probe's `reached_playing`
      (`run_report/checks.rs:279-315`) returns `Skipped` without a timeline, so
      probe would run them and assert NOTHING. **Port the WARN command-error
      gate into `log_clean`**: fail on `"Encountered an error in command"` at
      any level - the one assertion probe lacks, and the reason it exists is
      that `remove`/`despawn` bake in the WARN handler at queue time
      (`examples_smoke.rs:338-346`, task `20260713-203709`). Also:
      `CheckStatus::Skipped` must never fold into a passing verdict and
      `CheckStatus::Warn` must stop being a category that never fails
      (`checks.rs:36`, `:393`), or this step rebuilds "green and wrong" one
      layer down. Confirm `scene_baseline` and `render_scale_shot` (both in
      `NOT_SMOKED`) are covered by policy rather than by omission.
- [ ] 6. **Delete the smoke suite; converge the verdicts.** Move
      `catalog_matches_disk` (`:119`) and `every_category_has_a_probe_policy`
      (`:195`) to `crates/nova_probe/tests/`, deriving the repo root as
      `env!("CARGO_MANIFEST_DIR")` + `../..`; `catalog_matches_disk` gets
      simpler once the smoke lists stop existing. Delete
      `sections_assert_their_invariant_roster` (`:468`) and its 27-slug roster.
      Delete the THIRD verdict implementation -
      `crates/nova_autopilot/tests/autopilot_example.rs:37-90` and its
      byte-identical `fn tail` copy (`:173` vs `examples_smoke.rs:368`). Swap
      CI's "Examples smoke test" step (`.github/workflows/ci.yaml:95-101`) for
      a probe correctness run (there is no probe step in CI today). Delete
      `tests/examples_smoke.rs`. **Keep `log_clean`** - see Notes.
- [ ] 7. **Camera authority sets.** Independent of 1-6; touches no automation
      code. One nova-owned set chain in `PostUpdate`:
      `CameraShakeSystems::Restore -> CameraAuthority::Solve` (chase + WASD
      sync) `-> ::Override` (`enforce_scripted_camera_pose`) `-> ::Additive`
      (`CameraShakeSystems::Apply`) `-> TransformSystems::Propagate`. **Zero bcs
      changes** - every writer already exports its set. Kills the flicker, the
      two missing `Propagate` edges, and the duplicate edge registration at
      `camera_controller/mod.rs:112-114` / `framing.rs:475`. Highest
      value-to-effort in the whole investigation.
- [ ] 8. **Then, independently** (each its own child, no ordering between them):
      move bcs `integrity` + `ui/health_display.rs` to nova and scrub the nova
      task IDs from `bcs/src/physics/pd_controller.rs:535`; use bcs `persist`
      and delete nova's two copies plus the shadowed `feedback::flash`
      (`juice.rs:275`) and hand-rolled `time::Cooldown`; drop the physics pair
      from `base_scenario_object` (`crates/nova_scenario/src/actions/spawn.rs:97-113`)
      and delete the misaligned test at `:772-786`, moving its rationale comment
      onto the ship/asteroid bundles; the env-var pass; the `hud/mod.rs`
      registry refactor; the M9 dead-code sweep.
- [ ] 9. **Move the whole run onto a branch and review it.** Steps 1-8 land
      directly on `master`, so there is no branch for `/review` to diff. Cut
      `task/20260805-185103` from the commit BEFORE this run's first work commit
      (`cafae048`, the task record), cherry-pick every commit listed in the
      `commits:` lines above onto it in order, reset `master` back to that same
      base, then run `/review` against the branch and drive it to a verdict.
      Do this LAST - each step above appends its hashes as it lands, and the
      branch is only complete once step 8's children are in. Record the branch
      name and the review verdict here.
      commits: `cafae048` (task record; the branch base is its parent)

## Definition of Done

- [ ] Every step above has a child task, each with its own DoD. This task closes
      when the children do. (manual: owner confirms the child set is complete)
- [ ] The flicker is gone: `screenshot_combat`'s first `pose` holds a stable
      frame across repeated runs. (manual: owner watches the capture run)
- [ ] `tests/examples_smoke.rs` no longer exists, and CI runs a probe
      correctness pass covering the `screenshots/` category.
      (cmd: `test ! -f tests/examples_smoke.rs && rg -q "nova_probe" .github/workflows/ci.yaml`)
- [ ] Nova's prelude no longer re-exports the bcs prelude.
      (cmd: `! rg -n "pub use bevy_common_systems::prelude" crates`)
- [ ] The screenshot reel is gone and `shoot` is the single capture idiom.
      (cmd: `test ! -f crates/nova_autopilot/src/reel.rs && ! rg -n "ScreenshotReelPlugin|ReelBeat" crates examples`)
- [ ] No example branches its step timing on whether it is capturing.
      (cmd: `! rg -n "if capturing" examples/screenshots`)

## Notes

**Owner rulings that reverse the investigation's round-3 conclusions.** Both are
conditional, and the conditions are steps above, not assumptions:

- R3 said keep `tests/examples_smoke.rs`, because probe refuses the
  `screenshots/` category BY DESIGN (`catalog.rs:181-188`: *"Outside probe's
  scope: a probe verdict on one would assert nothing"*). The owner's answer is
  to change that design - valid, but the three coverage losses R3 documented are
  real and each gets an explicit fix in step 5.
- R3 said withdraw the reel delete, because `ScreenshotReelPlugin` has two live
  example users through the `nova_reel()` wrapper. Correct while the users
  exist. Converting them is budgeted in step 3.

**Things the investigation got wrong and later corrected - do not re-derive
them.** Each was verified against source:

- **Keep `log_clean`.** It is whole-token matching after ANSI stripping, not a
  substring grep (`crates/nova_probe/src/run_report/checks.rs:472-491`, with a
  comment saying substring matching was the previous BROKEN version). And
  conflating a panic with a wgpu teardown race is a false POSITIVE, not "green
  and wrong".
- **Do not change bcs's camera shake.** `CameraShakeSystems` exists
  (`bcs/src/camera/shake.rs:166`), is preluded, and `:205-209` already orders
  `Restore.before(Chase::Sync)` / `Apply.after(Chase::Sync)`. The real gap is
  that neither `Apply` nor `enforce_scripted_camera_pose` has an edge to
  `TransformSystems::Propagate`.
- **Do not add a deadline-panic to the autopilot.** Expiry ALREADY aborts with
  `AppExit::error()` naming the step (`crates/nova_autopilot/src/autopilot.rs:467-484`,
  with a NOTE comment saying exactly that). The real gap is a step with NO
  deadline and an unsatisfiable `until`, which hangs forever - and both
  subprocess harnesses block on `Command::output()` with no timeout, so that is
  a silent 60-minute CI hang. Make `deadline` mandatory or defaulted.
- **Do not build a flat `HashMap<String, Entity>` scenario index.** `EntityId`s
  are NOT unique - `crates/nova_scenario/src/actions/spawn.rs:36-39` states that
  spaceship SECTIONS carry their own `EntityId`s and an unscoped match would rip
  a section out of every ship in the scene. Five insertion sites, not one. Key
  it scoped (`(root_id, local_id)`) or skip it.

**Counts to distrust.** The investigation's numbers drifted; the shapes held.
Not reproducible: "253 of 477 prelude exports" (actual ~222 of ~387), "8
zero-caller items in the automation crates" (did not reproduce under three
passes; what exists is ~79 items with no CROSS-crate reference, i.e. internal
API), "27 env vars" (24 unique `NOVA_*`), "11 setup/remove pairs in
`hud/mod.rs`" (never enumerated). Wrong: `JuiceSettings` has 5 fields, not 14,
and is NOT dead - read every frame and `#[reflect(Resource)]`-registered, so it
is editor-tunable; `AIBehaviorState::Retreat` appears in 2 test functions, not
3, and none asserts the stub; `StepBuilder` has 5 setters, not 11; THREE of four
scenario object kinds override `RigidBody::Dynamic` (`asteroid.rs:280` too), not
two. Line references throughout the investigation run 2-3 low - it was written
against a slightly earlier tree.

**What the investigation did NOT find**, which constrains what is worth doing:
no `*_legacy`/`*_old`/`*_v2` anywhere; one damage path, one spawn path, one
camera path; all declared code-map boundaries hold except an undocumented
`nova_debug -> nova_autopilot` edge; no bcs version drift; the modding scaffold
is clean; no test asserts on doc prose. The suspected "testing documentation"
category is real but small - one WGSL source-text test worth KEEPING, three
tests asserting Bevy's log wording, and the invariant roster. Going-forward
rule: no test whose subject is source text or log text.

**The bcs boundary rule**, twice-corrected. "Safe iff it exports a `SystemSet`"
is wrong - only `meth` and the modding scaffold are opinion-free; orbit, PD and
persist are all plugins with schedule opinions, and every bcs module that writes
shared state already exports a set. The set is table stakes, not a
differentiator. What holds: **(a) order, don't disable** - ordering needs only
the exported set, costs one redundant write per frame, and survives bcs adding a
new writer, whereas a gate breaks silently; **(b) import behavior, not
presentation, and never a renderer you will not use.** The closest thing to a
real bcs hack in the tree supports (b): nova adds bcs's objectives plugin purely
for its Resource, discards its renderer (`crates/nova_gameplay/src/hud/mod.rs:293-298`),
then hand-diffs `GameObjectives` (`crates/nova_scenario/src/world.rs:52-72`) to
dodge a per-frame despawn/respawn in `bcs/src/ui/objectives.rs:104-107`. That
conflict is change-detection and renderer ownership - no SystemSet fixes it.

**Deliberately NOT in scope**, decided with the owner:

- Storing perf baselines in the repo. `git checkout <tag>` + re-run is the
  policy; `probe-runs/` stays gitignored. The residual is only that
  `fps_within_baseline` is armed and always skips while the run prints OK -
  delete the check, and let perf be `nova probe --compare <run-dir>` run by
  hand. Folded into step 8's dead-code sweep.
- Removing god mode from perf capture. A mix of god-mode and non-god-mode
  examples is wanted. The residual defect is that
  `crates/nova_probe/src/capture.rs:574-584` force-heals every `Health`
  UNCONDITIONALLY, so a scenario measuring death/despawn cost is
  unconstructible. Make it a per-EXAMPLE flag - note it cannot live in
  `CATEGORY_POLICIES`, which is keyed by category, and the driver is selected by
  `NOVA_PERF_COMBAT` (`scene_baseline.rs:95`).
- Promoting `nova_autopilot` out of nova. Per-game autopilot until the design is
  understood well enough to promote. Nova's is already the better design
  (predicate-driven vs bcs's frame-driven).

## Close-out - step 1: the torn `timeline.jsonl`

**What.** `ProbeTimeline::create` no longer truncates first and asks questions
later. It opens the path non-truncating, takes an EXCLUSIVE advisory file lock
(`File::try_lock`, std since 1.89), and only then `set_len(0)`. A second
recorder aiming at a live path gets `WouldBlock` and is refused with a message
naming the path. The plugin's arm-failure log moves `warn!` -> `error!`.

**Why.** The old `File::create` truncated to offset 0 while an earlier
recorder's `BufWriter` kept writing at its own offset, splicing two streams
into one line - the `malformed timeline line 143` failure recorded on
`20260804-174231` and in `20260804-094021`'s review. Steps 4-6 make the
timeline the SOLE verdict, so a silently torn artifact is the worst possible
foundation.

The log-level change is the second half of the fix: a refused recorder writes
nothing, and probe's `reached_playing` / `run_completed` return `Skipped` on a
missing timeline, which today folds into a passing verdict. `log_clean` greps
ANSI-stripped whole-word `ERROR`, so `error!` turns "probe asked for a timeline
and did not get one" into a run FAILURE instead of a thinner OK. That is the
same "green and wrong" class this parent task exists to close.

**Alternatives.** (a) A process-global registry of held paths - the fix
suggested on `20260804-174231`. Rejected: bevy already rejects a duplicate
unique plugin, so the only in-process double-arm is two `App`s, and a registry
does nothing for the two-process case that the run directory
(`probe-runs/<commit>/<example>/`, reused across invocations at one commit)
makes reachable. The file lock covers both with one mechanism and no new state.
(b) Open in append mode. Rejected: a stale timeline from a previous invocation
would silently prefix the new one; probe wants a fresh artifact per run, and
truncation under the lock gives exactly that.

**Difficulties.** The historical splice was never reproduced from first
principles - `t_real` 5.08 and 4.79 on the same `frame 168` says the two
writers had near-identical uptimes, which two sequential probe passes do not
explain. The fix is written against the MECHANISM (truncate-vs-offset), which
is proven regardless of which pair of writers raced, rather than against a
reproduction that six recorded runs could not produce.

**Evidence.**

- `recorder::tests::a_second_recorder_on_one_path_is_refused_not_torn` - the
  real plugin over two real `App`s on one path. Written first, failed on
  "the second recorder is refused, not armed on a shared path", passes now.
  It also pins re-arming after the holder drops, which probe's
  directory-reuse depends on.
- `cargo test -p nova_probe --lib` - 74 passed, 0 failed.
- `cargo check -p nova_probe --tests --bins` clean; `cargo fmt --all -- --check`
  clean.
- End to end: `cargo run -p nova_probe -- run menu_scenarios` - the example
  whose timeline tore. **OK, measured 5/6**, `run_completed` PASS (`run_end` at
  frame 178), `reached_playing` PASS, `invariants_held` PASS (0 violations /
  178 frames), `log_clean` PASS. The recorder still arms and still closes its
  bracket under the lock.

**Reflection.** One run is not proof of a race fixed, and this close-out does
not claim it is - what is proven is that the tearing MECHANISM is gone by
construction and that the guard does not cost the normal path. The `warn!` ->
`error!` move was not in the step text; it is in scope because a guard that
disables the artifact silently would trade a torn timeline for a missing one,
and steps 5-6 are explicitly about `Skipped` not folding into green.

## Close-out - step 2: the leaked bcs prelude

**What.** `crates/nova_gameplay/src/lib.rs`'s prelude no longer globs
`bevy_common_systems::prelude::*`. In its place is an explicit 28-name list of
the bcs vocabulary nova's own gameplay code is written in, with a comment naming
the five harness twins that are never on it. Two consumers that were reaching
bcs THROUGH that glob now import bcs directly, because their own crate already
depends on it: `crates/nova_scenario/src/objects/area.rs` (swapped a
now-unused `use nova_gameplay::prelude::*` for
`bevy_common_systems::prelude::CommandsGameEventExt`) and
`crates/nova_assets/src/scenario/shakedown/tests/walk.rs`
(`CommandsGameEventExt, EventHandler, GameEventsPlugin`).
`examples_name_drivers_through_the_nova_harness` is deleted from
`tests/examples_smoke.rs` - its subject is gone.

**Why.** The glob dragged bcs's retired `AutopilotPlugin`, `AutopilotLoop`,
`ScreenshotPlugin`, `ScreenshotReelPlugin` and `HarnessCompletion` into every
`use nova_protocol::prelude::*`, where a bare name silently resolved to the
inert twin and booted the example dead (`20260802-183403`). A source-grep test
policed that; deleting the glob turns the same failure into a compile error, so
the grep test is dead weight rather than a safety net.

**Alternatives.** (a) Keep the glob and widen the grep test - rejected: it
polices a symptom the type system can police. (b) Push every re-exported name
down to a direct bcs import in each consuming crate - rejected for
`nova_editor`, `nova_menu` and `nova_core`, which do NOT depend on bcs; adding
the dependency to three crates to avoid 28 re-export names is a bigger boundary
change than the step asks for. The rule actually applied: a crate that already
depends on bcs imports directly; a crate that does not gets the name through
nova_gameplay's prelude.

**Difficulties.** The blast radius could not be predicted, only measured, and it
came in five compile rounds because each fix unblocked the next crate in the
dependency order: nova_gameplay (42 errors) -> nova_scenario (`fire`) ->
nova_assets + nova_editor + nova_menu -> nova_core (the bcs status-bar
helpers) -> examples (`ChaseCamera`). Trait-method errors (`fire`,
`play_sfx_volume`) do not name the trait, so each needed a lookup in the pinned
bcs tree (`v0.19.5` = `30d1bef`) to find `CommandsGameEventExt` /
`SfxCommandsExt`.

**Evidence.**
- `cargo check --workspace --all-targets` clean; `cargo fmt --all -- --check`
  clean.
- Warning set is IDENTICAL to master's (11 dead-code/unused-import warnings in
  examples, verified by running the same check in the main checkout). This step
  neither adds nor removes a warning - it unblocks `20260731-205553`, it does
  not do it.
- The deleted test's guarantee, re-proven at the type level: a throwaway
  `tests/` file doing `use nova_protocol::prelude::*` then naming
  `AutopilotPlugin` / `ScreenshotPlugin` / `ScreenshotReelPlugin` fails to
  compile with E0433/E0425 (`--features debug`). The twins are unreachable, not
  merely unused. File removed after the check.
- `cargo test -p nova_assets --lib shakedown` 16/16;
  `cargo test --test examples_smoke -- catalog every_category sections_assert`
  3/3 (the display-free survivors of the file the deleted test lived in).
- RUN, not just checked: `cargo run -p nova_probe -- run
  hull_section,menu_scenarios` - both **OK, measured 5/6**, `reached_playing`
  and `invariants_held` PASS on each. `hull_section` is the example that needed
  `ChaseCamera` back; a prelude regression would boot it inert, and inert is
  exactly what probe reports as a failure.

**Reflection.** The 28-name list is the honest shape of the coupling, not a
tidy one: nova_gameplay's prelude now visibly re-exports bcs status-bar UI
helpers that only `nova_core` wants. That is worth leaving ugly - it names a
real dependency that the glob was hiding, and it is the kind of thing a future
step can move to `nova_core` depending on bcs directly. The step's estimate
that the blast radius was unmeasurable was right, and the measurement cost five
builds; nothing outside the workspace broke, which is the useful finding.

## Close-out - step 3: one capture idiom

**What.** The screenshot reel is deleted and `shoot` is the fleet's single
capture idiom.

- `crates/nova_autopilot/src/reel.rs` and `tests/reel.rs` are gone. The
  primitive they carried moved to a new `crates/nova_autopilot/src/capture.rs`:
  `capture_window`, the `NOVA_SHOT_DIR` resolution and its unit test,
  `CAPTURE_RESOLUTION`, and a new `capturing()` reading `CAPTURE_ENV`.
  `ScreenshotReelPlugin`, `ReelBeat` and `completion::REEL` are gone with the
  driver.
- `nova_debug::harness` gains `shoot(world, path)` - capture + log, gated on
  `capturing()` - which collapses the three identical `fn shoot(world,
  capturing, path)` copies (`screenshot_flight`, `screenshot_nova_os`,
  `screenshot_combat`) and `screenshot_ui`'s closure. `nova_reel`,
  `NovaReelPlugin`, `reel_beat`, `ReelCamera` and `hide_reel_chrome` are gone;
  `reel_pose_camera -> pose_camera` and `reel_freeze_bodies -> freeze_bodies`
  (now public) survive the driver they were named for, and
  `scenario_camera_present` became a public `Predicate` builder so a script can
  hold a step on it.
- The reel's non-beat responsibilities became named, shared pieces rather than
  plugin internals: `force_capture_resolution` (collapsing FOUR copies of
  `fn force_resolution`), `hide_hud` (from `screenshot_ui`'s local copy plus
  `hide_reel_chrome`'s HUD half), `freeze_bodies`.
- `screenshot_sections.rs` and `screenshot_scene.rs` are rewritten from
  `nova_reel(beats)` onto autopilot steps - the real cost of the step. Each is
  now one script (`section_script` / `scene_script`) that waits on
  `scenario_camera_present()`, then walks present/frame -> settle -> shoot per
  shot. Both files lost `nova_autopilot()`: the ONE script is now both paths.
- `examples/ui/widget_zoo.rs` shoots through `capture_window` (queued as a
  command, since the driving system holds `Commands`), deleting its own
  `NOVA_SHOT_DIR` resolution; its capture pass moved behind `--features debug`
  because `capture_window` lives there. `examples/ui/menu_scenarios.rs` uses
  `capturing()` + `shoot`.
- **Env rename, `NOVA_REEL -> NOVA_CAPTURE`** (`REEL_ENV -> CAPTURE_ENV`), and
  the staging dir in the docs/script `target/reel -> target/shots`. Also
  `HarnessMute`'s env list (`nova_gameplay/src/settings.rs`), the wiki
  automation-harness + development pages (with a new "Capturing: one idiom"
  section), `scripts/gen-web-screenshots.py`, and the CHANGELOG's
  still-Unreleased `BCS_* -> NOVA_*` entry, amended in place rather than given a
  second breaking note.
- Out of scope but found on the way: master did not compile. Step 2's prelude
  narrowing dropped two names still used by examples, so `cargo check --features
  debug --examples` failed on `screenshot_combat` (`PointRotation`) and
  `hud_range` (`DirectionalSphereOrbitOutput`). Both added to nova_gameplay's
  explicit bcs list - step 2's own stated procedure, just missed.

**Why.** A beat list is built away from the script that produces the state each
beat frames, so timing and framing lived in different files; as steps they read
act -> frame -> shoot in source order. `NOVA_REEL` naming the capture flag was a
lie once the reel was gone, and it had never shipped (the rename entry is still
under `[Unreleased]`), so amending it costs no released consumer.

**Alternatives.** (a) Keep `REEL_ENV`'s value and rename only the const -
rejected: a const/value mismatch is worse drift than either name alone.
(b) Give scene/sections a small "capture preset" plugin carrying the resolution,
chrome and freeze wiring - rejected as a mini-reel; the three pieces are three
named functions an example adds itself, which is what makes the file readable.
(c) Make `shoot` take `capturing: bool` like the copies it replaces - rejected:
every caller passed the same expression, so the gate belongs inside.
(d) Fold `crates/nova_scenario/src/actions/view.rs`'s third `NOVA_SHOT_DIR`
resolution onto `capture_window` - **NOT DONE**, see below.

**Deliberately not done.** `crates/nova_scenario/src/actions/view.rs:70` still
resolves `NOVA_SHOT_DIR` itself. `nova_scenario` is a SHIPPING crate and does
not depend on `nova_autopilot`; folding it would put an automation-driver crate
into the game binary to save a six-line path join. The task text flags it
parenthetically as a known third copy rather than instructing the fold, and this
reading is recorded here so the reviewer can overrule it cheaply.

**Difficulties.** The reel owned four things, not one, and only the beat cadence
was dead: the window resize, the overlay/HUD hide, the body freeze and the
scene-ready gate all had to land somewhere before the driver could go. The
freeze and the gate are the load-bearing ones - without the freeze the posed set
drifts between shots, and without the gate `pose_camera` warns and poses nothing
(the step then shoots an unframed frame rather than failing). Both converted
scripts hold their first step on `and(state_is(Playing),
scenario_camera_present())` for exactly that reason.

The first headless run appeared to prove the conversion broken - `step 'wait for
the drydock scene' stalled after 30.0s, state Loading`. It was the rig, not the
code: running `./target/debug/examples/screenshot_scene` directly resolves
assets against the binary's directory, so the load never completed. Under
`cargo run` from the repo root both examples pass.

**Evidence.**
- `cargo check --workspace --features debug --all-targets` clean;
  `cargo fmt --all` clean. `cargo doc --no-deps -p nova_autopilot -p nova_debug`
  produces the SAME 5 pre-existing `nova_autopilot is both a function and a
  crate` warnings as master (counted on both trees) - no new rustdoc warning.
- `cargo test -p nova_autopilot --lib capture` 1/1;
  `--test env_contract` 1/1 (rewritten for `CAPTURE_ENV`, `REEL` collector
  dropped); `cargo test -p nova_debug --lib harness` 5/5, including the new
  `shoot_captures_nothing_when_the_run_is_not_armed`.
- RUN, not just checked (Xvfb `:99`, `cargo run --features debug`):
  - `screenshot_scene` capture path (`NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`): exit 0,
    `reached Playing`, `cycle complete, no panic (t=5.6s)`, all three PNGs on
    disk at 1920x1080.
  - `screenshot_sections` capture path: exit 0, `cycle complete (t=8.1s)`, all
    five section PNGs at 1920x1080.
  - Both SMOKE paths (`NOVA_AUTOPILOT=1` alone): exit 0, `cycle complete
    (t=1.7s)` each, ZERO `nova capture` lines and the shot dir never created -
    the one script really is both paths, and the smoke path is ~3-5x shorter.
  - Framing preserved across the rewrite: the new `feature-gravity.png` is the
    shipped `web/src/assets/feature-gravity.png` framing (same camera, same hero
    placement); the differences are rock mesh/lighting from the intervening
    gravity and scenario-lighting tasks, not from this step.
- DoD proof `test ! -f crates/nova_autopilot/src/reel.rs` passes. The companion
  grep `! rg -n "ScreenshotReelPlugin|ReelBeat" crates examples` has ONE
  surviving hit, reported rather than papered over:
  `crates/nova_gameplay/src/lib.rs:73`, the comment naming the bcs harness twins
  that must never be re-exported. `bevy_common_systems` still ships that type;
  the comment is a guard on the explicit list, so it stays.
- The step-4 proof `! rg -n "if capturing" examples/screenshots` still fails, as
  expected: the per-path settle split is exactly what step 4 unifies.

**Reflection.** The step's cost estimate was right about which part was
expensive and wrong about why. The two file rewrites were mechanical once the
step shape was settled; the expensive part was inventorying what the plugin did
BESIDES walk beats, because none of it was in the plugin's name. A driver that
also owns window sizing, chrome, physics and a readiness gate does not announce
those in its type - the only way to find them was to read `build`. Worth
remembering when the next "delete the driver" step is scoped: budget for the
driver's undeclared side jobs, not for its headline behaviour.
