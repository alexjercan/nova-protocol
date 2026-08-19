# probe measures a scenario directly, no example required

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.11.0,harness,performance,scenario

Epic: `20260818-220812`. Owner, 2026-08-19: "`scene_baseline` should be a
FEATURE of `nova_probe_cli`... like `probe` subcommand should allow you to load
a scenario from a .ron file basically. It's like I want to probe a scenario,
well use `probe` - why would you use an example for it."

## The point

Measuring a scenario should not require an example. `probe` already owns
running, measuring and reporting; a scenario is data. Point the tool at the
data.

```
probe scenario <path-to.ron>      # or an id from GameScenarios
```

It loads the scenario, runs it, and reports the same frame data any probe run
reports. No `[[example]]` entry, no Rust file per scenario, no example name
baked into library code.

## What this replaces

`examples/screenshots/scene_baseline.rs`, deleted with the screenshot split. It
took `--scenario <id>` or `NOVA_PERF_SCENARIO` and measured whatever it loaded,
which is exactly this subcommand wearing an example costume.

It also removes the reason `nova_probe_cli` ever wanted to know an example's
NAME - see the deleted `budgets.rs`, which hardcoded example names and frame
numbers into library code. **Nothing in `nova_probe_cli` may name a real
example.** Test fixtures use obviously fake names.

## Design notes

- **Report, do not judge.** The probe measures and presents; a human reads the
  HTML report and decides whether the frame rate is acceptable. No pass/fail on
  a frame-time number, no per-scenario budget table. This is settled - it is
  why `budgets.rs` was deleted.
- Loading from a PATH matters as much as loading by id: a modder or a
  contributor should be able to measure a scenario that is not in the shipped
  catalog, without adding it to one.
- `editor_sandbox` is the awkward case and worth handling deliberately. It is
  registered into `GameScenarios` at editor-Play time, AFTER the `--scenario`
  membership check runs, so it cannot be loaded by id at all today
  (`crates/nova_editor/src/scenario.rs:203-212` against
  `crates/nova_core/src/lib.rs:257-266`). Either the check learns about late
  registration, or the sandbox registers up front like everything else. That
  hole is what hid a 2 FPS bug for a whole cycle.
- `crates/nova_perf_web/src/main.rs` mirrors what `scene_baseline` did for the
  web perf page. Decide whether it becomes a caller of this or stays separate.

## Done when

- `probe` can measure any shipped scenario by id, and any scenario file by
  path, with no example involved.
- The report presents frame data clearly enough that a human can judge it in
  one look.
- No real example name appears anywhere in `nova_probe_cli` outside test
  fixtures, and those use fake names.

## What was decided

### The subject of a scenario run is the GAME BINARY

```
probe scenario <id>          # from the merged GameScenarios registry
probe scenario <file.ron>    # a loose content file, catalog or not
```

Every existing pass runs unchanged - clean, frame time, profiled, optional
samply - against `target/debug/nova-protocol` launched with `--scenario <id>`
or `--scenario-file <path>`. `run.rs` grew three helpers (`build_subject`,
`subject_bin`, `subject_args`); the passes themselves did not change.

The binary carries the collectors: `src/main.rs` adds
`nova_probe::NovaProbePlugin::default()` and `nova_debug::harness::nova_autopilot()`
under the `debug` feature. Both are env-gated, so `cargo run --features debug`
is unchanged. This is what removes the need for an example: an example existed
only to be a program that wires the collectors, and the game already is one.

Not `AppBuilder`, deliberately. Several examples call `editor_app()` AND add
`NovaProbePlugin` themselves; wiring it into the composition root would
double-add and panic. The binary is the one place no example passes through.

`.ron` suffix decides file-vs-id, so parsing stays filesystem-free like the
rest of the CLI. A missing file is refused by the child, which is also the half
that can say what the file contained.

### `editor_sandbox`: the sandbox registers up front

Chosen over teaching the `--scenario` membership check about late registration.
The check is not the only id-driven consumer - the DEFEAT overlay's Retry, the
picker's hidden launch, a `menu_backdrop` pin and now the probe all resolve ids
against `GameScenarios`. Teaching one of them about a special case leaves the
rest broken; publishing the id fixes all of them at once. An id nothing can
name is not content.

`NovaEditorPlugin` now registers the sandbox at `OnEnter(GameAssetsStates::Loaded)`
with the DEFAULT hull, in `EditorSandboxSystems`, and `nova_core`'s startup
handoff is ordered `.after` that set. The Play hand-off still overwrites the
entry with the hull the editor just built, so Retry reloads the ship you flew.
A `PostUpdate` repair re-registers it whenever the bundle merge replaces the
registry: `register_bundles` rebuilds `GameScenarios` from content files, and
this is the one scenario with no content file behind it.

`--scenario editor_sandbox` works for a human now too. That was the hole.

### Loose files: `self://` resolves against the enclosing bundle

`nova_assets::loose::read_loose_scenarios` parses the file and rewrites its
`self://` refs against the nearest ancestor directory holding a `*.bundle.ron`,
using that manifest's declared `resources` - the same rewrite and the same
membership gate the merge applies. Without it the mode would have been hollow:
every shipped scenario names its art `self://`, so an unrewritten load measures
a scene with no textures in it. `dep://` is left literal - reaching into
another mod needs a resolved dependency graph, and a loose file has none.

### `nova_perf_web` stays separate

It is the WASM app. `probe scenario` drives a native child with env vars and
command-line arguments; a browser build has neither, and the web capture
already has its own driver (trunk build, serve, headless Chromium, scrape the
console line). More decisively, `perf_web` builds through `with_game_plugins`,
which suppresses `NovaEditorPlugin` - so `editor_sandbox` does not exist in its
registry and never can. The case that matters most is exactly the case the web
app cannot reach.

What it stops being is the NATIVE scenario runner. Nothing drives it natively
any more; `probe scenario` is that door.

### `trace` had to move to `nova_core`

The root package's `bevy` entry is a DEV-dependency, so `bevy/trace` in the
root `trace` feature never reached a `cargo build --bin`. The first profiled
pass of a scenario ran and produced no `trace.json` at all. `nova_core` now
owns `trace = ["bevy/trace", "bevy/trace_chrome"]` and the root forwards to it.
Examples were unaffected (they resolve dev-dependencies), which is why this
never showed up before.
