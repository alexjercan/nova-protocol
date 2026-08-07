# nova_probe

9,890 LOC / 37 files. The crate the owner flagged as "messy and hacked
together". The diagnosis was right; the proposed cure did not fit the code.

## The central fact: two programs, one crate

`nova_probe` is not a pipeline. It is:

- **An in-game library** (`src/lib.rs:82-168`), linked into examples:
  `capture` (frame-time), `recorder` (timeline JSONL), `invariants`, `contract`,
  `fixtures`.
- **A host CLI** (`src/bin/probe/`, 2,401 LOC under `native/`) that spawns the
  example as a **child process** and reads its files back.

**The IPC boundary is the filesystem plus environment variables.** That is the
real architecture, and no module name states it.

## Actual data flow

1. `native/sweep.rs::run_spec` -> `native/run.rs::run` (`run.rs:60`)
2. `run.rs:75` `profile_sandbox::prepare`; `run.rs:86` `ensure_display` (Xvfb);
   `run.rs:124` `build_example`; then `run_supervised` per pass
   (clean / fps / trace / samply)
3. **Collection happens in the child**, armed by env vars set at
   `native/env.rs:93-108`: `NOVA_PERF_TIMELINE`, `NOVA_PERF_INVARIANTS`,
   `NOVA_PERF_CONTRACT`, `NOVA_PERF`. Plugins self-arm (`recorder.rs:83`,
   `invariants.rs:109`, `capture.rs:333`) and each writes its own artifact:
   `recorder.rs:220` (JSONL), `capture.rs:516`/`:522` (`<label>.json` +
   `frametime.csv`), `contract.rs:169` (`probe-contract.json`)
4. **Back in the parent**: `run.rs:303 finish_report` writes `probe-run.json`;
   `RunArtifacts::load` (`run_report/artifacts.rs:44`) re-parses everything off
   disk; `evaluate_checks` (`run_report/checks/mod.rs:139`, roster at `:113`)
   scores it; `render_run_report` -> `report.html`; `checks_json` ->
   `checks.json` (`run.rs:330-341`)
5. Aggregate `index.html` / `index.json` / `probe-all.json` at
   `native/sweep.rs:235-245`. `probe report` re-runs step 4 only
   (`native/report.rs:53-58`)

Note the real path is `crates/nova_probe/src/run_report/`, **not** under
`src/bin/`.

## Against the owner's original sketch

| Proposed | Verdict |
| --- | --- |
| `collect -> evaluate -> report` as three stages | **Half right.** evaluate -> report fits; collect does not |
| One `NovaProbePlugin` orchestrating all three | **Impossible.** The plugin lives in the child, evaluation in the parent. The truncated-timeline check specifically requires the writer process to be dead |
| `capabilities/` module behind `trait Capability` | Viable for declare+arm only. See below |
| Abstract read/write to remove wasm gates | Removes ~5 of ~20. The rest guard process spawning and CLI code |
| `evaluation/` module doing check runs | **Already exists.** Rename, do not rebuild. ~~The best code in the crate.~~ **Amended 2026-08-07** - the structure is good, but the loader in front of it carries four gate defects, three failing OPEN. See `15-review-probe.md` |
| `report/` module | Same - `run_report/html.rs` already does this |

`run_report/` is exactly `RunArtifacts` (load) -> `checks/*` (evaluate) ->
`html.rs` (render), with each check a module exposing
`evaluate(&RunArtifacts) -> Check` and a single roster table. Leave the design;
fix the name.

## The three capabilities

Not crates. Free functions returning plugins:

| Function | At | Builder |
| --- | --- | --- |
| `nova_timeline() -> RunRecorderPlugin` | `recorder.rs:64` | `.out(PathBuf)` (`recorder.rs:77`) |
| `nova_invariants() -> InvariantsPlugin` | `invariants.rs:70` | `.strict(bool)` (`:89`), `.monotonic::<I,S>(keys)` (`:99`) |
| `nova_frametime() -> FrameTimePlugin` | `capture.rs:105` | `.drive(impl Fn(&mut World,u32))` (`:120`) |

Common shape is real: each `Plugin::build` calls
`contract::declare(app, Capability::X)` above an arming guard, reads
`perf_param(PARAM)`, and writes one artifact.

A `trait Capability` could cover roughly `const CAPABILITY`, `const PARAM`, and
`fn arm(&self, app: &mut App, param: String)`. Four blockers:

1. **Name collision** - `enum Capability` already exists at `contract.rs:38`.
2. **The builders do not unify.** `.out()`, `.strict()/.monotonic()`, `.drive()`
   are three different config types. The trait cannot cover configuration.
3. **Arming is asymmetric.** timeline/invariants/contract arm on a named param;
   frametime arms on bare `NOVA_PERF` via `perf_armed()` (`capture.rs:71`).
4. **`invariants` is not a peer.** It writes *into* the recorder's sink and
   orders against it - `.before(crate::recorder::record_variable_changes)`
   (`invariants.rs:143`).

Also: **two of six checks have no in-app collector at all.** `process_exit` and
`log_clean` come from the supervisor and the child's stdio
(`native/supervise.rs:125`). Any `Capability` abstraction must not imply all
evidence flows through it.

## Opt-in today

**There is no `NovaProbePlugin`.** An example opts in with three independent
`add_plugins` lines inside `#[cfg(feature = "debug")]` -
`examples/systems/player_path.rs:190,197,200`.

Wiring is inert without env vars, and wiring *is* the claim
(`contract.rs:129 declare`). A single collection-side bundle plugin is a real
improvement, but per-example `.monotonic([...])` and `.drive(...)` config must
survive it.

## Every wasm gate

| Site | Guards | Removable by an IO trait? |
| --- | --- | --- |
| `lib.rs:82,85,94,99,104,109` | module gating for `aggregate`, `catalog`, `profile_sandbox`, `invariants`, `fixtures`, `recorder`, `run_report` | **No** - host tooling, not IO |
| `lib.rs:113-141` | hand-written wasm stub `mod recorder` duplicating `nova_timeline`/`RunRecorderPlugin`/`probe_marker` | **Yes** - a no-op writer |
| `lib.rs:145,154,157,161,163` | re-export gating | follows the modules |
| `contract.rs:137,149,163` | Startup writer + atomic `write_to` | **Yes** |
| `capture.rs:57,63` `perf_param`; `:72,76` `perf_armed`; `:83` `query_param` | env var vs URL query | config input, already abstracted at function level - a trait just moves it |
| `capture.rs:262,287,297` | `resolve_git_sha` (shells out to git), `resolve_host` (`/etc/hostname` vs `"browser"`) | **No** - process/host facts |
| `bin/probe/main.rs:27-33` | stub `main()` so `cargo check --target wasm32` stays green | no |

Honest count: ~5 of ~20 removable (recorder stub, contract writer, frametime CSV
write).

## Coupling and misplaced files

Depends on `avian3d`, `bevy`, `nova_autopilot`, `nova_gameplay`, `nova_events`,
`nova_scenario`, and **`nova-protocol` (the root crate)** - because
`bin/perf_web.rs` builds the whole game.

Autopilot coupling is narrow and one-way, and fine: `capture.rs:21` uses
`completion::{self, HarnessCompletion}`; `native/env.rs:6` uses `AUTOPILOT_ENV`,
`DEADLINE_ENV`.

Three files that should not be here (owner approved evicting all three):

| File | Why it is here | Belongs |
| --- | --- | --- |
| `src/fixtures.rs` | scenario/ship builders parked here to dodge the `catalog_drift` scan - **admitted at `fixtures.rs:9-15`** | nova_assets or examples |
| `src/profile_sandbox.rs` | mod-cache / XDG knowledge | nova_assets / nova_menu |
| `src/bin/perf_web.rs` | the sole reason probe depends on the whole game | root crate |

Removing these cuts the dependency list roughly in half.

## Other

- **No prelude.** 184 deep-path imports - worst in the workspace.
- Comment ratio is fine, ~85% why. The noise is concentrated: `lib.rs` carries
  **100 comment lines in 168**, a user manual duplicated in
  `.claude/skills/probe/SKILL.md`, and every `#[cfg]` there carries a 2-4 line
  justification paragraph. Folding into `capabilities`/`evaluation`/`report`
  deletes most of that prose as a side effect.
- **Probe is itself the CI gate** (`cargo run -p nova_probe -- run --all`). A
  mistake here blinds the gate, so this work goes first among the code moves and
  needs its own verification. **The 2026-08-07 review found the gate is already
  blind in four ways** - see `15-review-probe.md`. Those fixes are now a
  prerequisite to the restructure, not part of it: every other lane in the epic
  is verified by this gate.

## Check roster

`crates/nova_probe/src/run_report/checks/`: `fps_within_baseline.rs`,
`invariants_held.rs`, `log_clean.rs`, `process_exit.rs`, `reached_playing.rs`,
`run_completed.rs`, `mod.rs`. Roster table at `mod.rs:113`:

```rust
const CHECKS: &[(&str, Option<Capability>, fn(&RunArtifacts) -> Check)] = &[
```
