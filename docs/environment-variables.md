# Environment variables

Every `NOVA_*` variable the game reads, in one place: what it gates, which
crate owns it, and who it is for. This page is the INDEX. It does not restate
what each variable means - the page that owns the mechanism does that, and the
link in each row goes there.

**Nothing type-checks an environment variable.** Renaming one compiles clean
and fails at load, or - worse - silently stops arming something and the run
looks fine while measuring nothing. So there is one rule, and
`tests/env_contract.rs` enforces it:

- **Every variable is a named constant, declared once, in the crate that owns
  the behaviour it gates.** No bare literals at a use site in `crates/` or
  `src/`. The measurement knobs are computed from one prefix by
  `nova_probe::probe_env`, so the host that pushes a name into a child run and
  the child that reads it back cannot disagree.
- **The test is also a roster.** It walks `crates/`, `src/` and `tests/` for
  `NOVA_*` literals and fails on anything it does not name, so a new variable
  cannot arrive undeclared.
- **`examples/` keeps its literals** on purpose. A range that drifts turns a
  probe run red, which is detection shipped code does not have
  (`AGENTS.md` Nova behavior).

Three names are spelled twice, deliberately: `nova_gameplay`'s mute policy,
`nova_scenario`'s `Screenshot` action and the probe sandbox each need a name
another crate owns, and a dependency edge from a shipping crate to a
dev-tooling one - or from the host harness to the whole asset stack - is worse
than the repetition. The contract test asserts each pair equal.

## Who the columns are for

- **harness** - set by a run script, by CI, or by `probe`. A player never sees
  it, and an unarmed run pays nothing for it.
- **tooling** - set by a test or a contributor's shell to keep a run off the
  real profile on disk.
- **player** - usable on a normal `cargo run` or a shipped build.

## The harness: what drives a run, and where its pictures go

Owned by `nova_autopilot`. The mechanism is
[The automation harness](automation-harness.md), which carries the values and
the stall semantics.

| Variable | Gates | For |
| --- | --- | --- |
| `NOVA_AUTOPILOT` | arms the scripted state driver; unset, the plugin adds nothing | harness |
| `NOVA_AUTOPILOT_DEADLINE` | seconds before the completion watcher error-exits naming the laggards | harness |
| `NOVA_CAPTURE` | puts a script on its CAPTURE path - its shot beats write PNGs, its loops record | harness |
| `NOVA_CAPTURE_DIR` | directory relative capture paths stage under; absolute paths ignore it | harness |

`NOVA_CAPTURE` arms the SHOTS, never a driver, so a capturing run sets
`NOVA_AUTOPILOT` too and one script owns the window. `NOVA_CAPTURE_DIR` is also
read by the scenario `Screenshot` action, which is the in-game photo-mode lever
rather than a harness one.

## Measurement: what a run records about itself

Owned by `nova_probe`, all under one prefix, all inert unless set. The full
table - defaults, units, and the wasm URL-query twin of each - is the crate's
own rustdoc (`cargo doc --open -p nova_probe`), and
[Measuring performance](performance.md) is what to read before quoting a
number from any of them.

| Variable | Gates | For |
| --- | --- | --- |
| `NOVA_PROBE` | arms frame-time capture, the scene census and the frame-cost breakdown | harness |
| `NOVA_PROBE_MODE` | `correctness` drops the measuring passes from a child run | harness |
| `NOVA_PROBE_WARMUP` / `_FRAMES` | the capture window, in frames; wins over an example's declared one | harness |
| `NOVA_PROBE_OUT` | directory the run writes `frametime.csv`, `<label>.json` and `census.json` into | harness |
| `NOVA_PROBE_LABEL` | the row label a capture records itself under | harness |
| `NOVA_PROBE_RES` | forced primary-window resolution for the measured run | harness |
| `NOVA_PROBE_RENDER_SCALE` | forces the render-scale lever, holding the rest of the quality preset fixed | harness |
| `NOVA_PROBE_MAX_DELTA` | ceiling on how many fixed steps one frame may run | harness |
| `NOVA_PROBE_PRESENT` | presentation mode forced on the primary window | harness |
| `NOVA_PROBE_QUALITY` | graphics preset for the run, recorded in the metadata | harness |
| `NOVA_PROBE_SCENARIO` | the scenario a sweep cell measures | harness |
| `NOVA_PROBE_SHA` / `_HOST` | override the recorded git SHA and host tag | harness |
| `NOVA_PROBE_CENSUS_FRAME` | frames after `Playing` at which the scene census is taken | harness |
| `NOVA_PROBE_FRAMECOST_FRAMES` | frames averaged into one frame-cost report | harness |
| `NOVA_PROBE_RENDER_DIAG` | asks the renderer for GPU timestamp queries, so passes can be named | harness |
| `NOVA_PROBE_TIMELINE` | JSONL path for the run timeline: states, events, variables, markers | harness |
| `NOVA_PROBE_INVARIANTS` | arms the continuous engine-bound invariant checks | harness |
| `NOVA_PROBE_CONTRACT` | JSON path the run declares its wired capabilities to | harness |
| `NOVA_PROBE_SNAPSHOT` / `_SNAPSHOT_FRAMES` | JSONL path for world-state snapshots, and the frames to take them at | harness |
| `NOVA_PROBE_STEPDIAG` | CSV path for the per-fixed-step physics diagnostics | harness |
| `NOVA_PROBE_STEPDIAG_BODIES` | the body-count REGIME floor its end-of-run summary is taken over | harness |
| `NOVA_PROBE_SANDBOX_RESOLVER_CHILD` | marks the re-executed child in the probe host's own sandbox test | harness |

`NOVA_PROBE_RENDER_DIAG` is declared in `nova_core` rather than `nova_probe`:
the wgpu feature can only be requested where `RenderPlugin` is built, and
`nova_core` is the lowest crate both name.

## Outputs off

One variable per output device, and a matching debug-only flag on the game
binary. An example has no command line of its own, which is why the
environment half exists at all. See
[Building and running](development.md).

| Variable | Flag | Gates | Owner | For |
| --- | --- | --- | --- | --- |
| `NOVA_NORENDER` | `--norender` | every `AppBuilder::new()` in the process assembles a headless app: no device, no window, no winit | `nova_core` | harness |
| `NOVA_MUTE` | `--mute` | zeroes the audio OUTPUT; the volume setting is untouched | `nova_gameplay` | player |

`NOVA_MUTE` unset still mutes a run that has `NOVA_AUTOPILOT` or
`NOVA_CAPTURE` set; `NOVA_MUTE=0` forces sound through one. The flag wins over
both, and a muted run says `nova audio: output muted for this run` once.

## The replay seed

Owned by `nova_gameplay`, next to the entropy plugin it seeds.

| Variable | Gates | For |
| --- | --- | --- |
| `NOVA_SEED` | seeds the gameplay RNG with one `u64`, so a driven run replays byte for byte; unset, the OS seeds it and no two runs agree | harness |

A value that does not parse as a `u64` refuses the boot rather than running
unseeded - a replay that silently lost its seed is the failure the knob exists
to prevent.

## Modding, and the settings store

Owned by `nova_assets`. See `/create/publish-a-mod/` for the portal.

| Variable | Gates | For |
| --- | --- | --- |
| `NOVA_MODDING_CACHE_ROOT` | moves the local mod cache off the platform data dir | tooling |
| `NOVA_MODDING_PORTAL_URL` | points a native build at another portal tree | tooling |
| `NOVA_CONFIG_ROOT` | moves the settings store off the platform config dir | tooling |

`NOVA_CONFIG_ROOT` is deliberately NOT in the modding family. It is the
settings store root, and its name is already right.

## The menu

| Variable | Gates | Owner | For |
| --- | --- | --- | --- |
| `NOVA_MENU_BACKDROP` | pins the menu backdrop to one `menu_backdrop` scenario id instead of re-rolling the draw; an unknown id warns and falls back | `nova_menu` | harness |

## Not on the roster

- **Example-local knobs.** `NOVA_STRESS_PD_*`, `NOVA_EDITOR_FRAMELOG`,
  `NOVA_SOAK_SCENARIO`, `NOVA_SOAK_SECS` belong to one example each and stay
  literals beside the range that reads them.
- **`NOVA_OS_*`.** Around 180 of these exist and NONE is an environment
  variable: they are `const Color`, layout and volume values in `nova_os_ui`,
  `nova_os` and `nova_gameplay::audio`. A grep for `NOVA_[A-Z_]*` is dominated
  by them, so count `env::var` call sites instead of identifiers.
- **Shell-only.** `NOVA_UI_PORT`, `NOVA_GAME_PORT`, `NOVA_MODS_PORT`,
  `NOVA_MODS_DIR`, `NOVA_PORT_LO`/`_HI` are read by `scripts/` and
  `web/webpack.config.js`; `NOVA_BENCH_*` by `benchmark/`. No Rust reads any of
  them.
- **Foreign variables the code legitimately reads**: `DISPLAY`,
  `WAYLAND_DISPLAY`, `RUST_LOG`, `BEVY_ASSET_ROOT`, `CARGO_*`, `CI`,
  `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, `LVP_ICD`, `RUSTFLAGS`, `VK_ICD_FILENAMES`,
  `VK_DRIVER_FILES`, `WGPU_BACKEND`, `TRACE_CHROME`.

## Adding one

1. Declare it as a named constant in the crate that owns the behaviour it
   gates. A measurement knob belongs in `nova_probe`, never in a gameplay
   plugin - the four fixed-step knobs that lived in `NovaGameplayPlugin`, one
   of them able to `panic!` on a malformed value, are the worked example of
   getting this wrong.
2. Export it through that module's prelude.
3. Add it to the roster in `tests/env_contract.rs`. The test fails until you
   do, which is the point.
4. Add a row above.
