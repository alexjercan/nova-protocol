# What moved where

Seven commits, one per part of the spec, on `env-vars-one-home` off `d5f6591d`.

## 1. `NOVA_SHOT` collapsed into the capture path

`crates/nova_autopilot/src/screenshot.rs` is deleted, with `ScreenshotPlugin`,
`SCREENSHOT_ENV`, `MAX_WAIT_FRAMES`, the stand-down rule, the `OnEnter(Loaded)`
caveat and both `tests/screenshot*.rs`. `SHOT_DIR_ENV` is `CAPTURE_DIR_ENV`
holding `NOVA_CAPTURE_DIR`.

`nova_screenshot()` now takes the script it extends and returns it:

```rust
pub fn nova_screenshot(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates>
```

It appends two beats - hide the dev overlays and settle `SETTLE_FRAMES`, then
`shoot(NOVA_SCREENSHOT_PATH)` held on `shot_written` with `SHOT_DEADLINE_SECS`.

**The signature change was forced, not chosen.** All 20 ranges already add an
`AutopilotPlugin<GameStates>` of their own; a preset that RETURNED one would be
a duplicate-plugin panic in every single one. Composition is also what the
collapse means - the shot is a beat in the run's one script.

`completion::SCREENSHOT` went with the driver. The two tests that used it as
"a second, slower collector" use `loops::LOOP_CAPTURE`, which is a real one.

## 2-4. The renames

- `NOVA_PERF*` -> `NOVA_PROBE*` everywhere, including the computed form.
  `PERF_ENV`/`perf_param`/`perf_armed` are `PROBE_ENV`/`probe_param`/
  `probe_armed`. The `nova_perf_web` CRATE keeps its name - it is an app that
  links the game, not a knob.
- `NOVA_MUTE` gets `--mute` on the game binary, inserted after the builder so
  it beats whatever the environment resolved. `NOVA_NORENDER` and `NOVA_MUTE`
  each name the other in their docs.
- `NOVA_MOD_CACHE_ROOT` / `NOVA_PORTAL_URL` -> `NOVA_MODDING_*`.
  `NOVA_CONFIG_ROOT` untouched.

## 5. One home each

`nova_probe::probe_env(param)` is the single place the `NOVA_PROBE_` prefix is
spelled; `nova_probe_cli`'s child-env builder calls it instead of writing the
names by hand, so the host and the child cannot disagree.

`tests/env_contract.rs` in the ROOT package - the only crate that can see every
other one. Six pin tests plus a SCAN that walks `crates/`, `src/` and `tests/`
for `NOVA_*` literals and fails on anything off the roster. Negative-checked:
appending `pub const FAKE: &str = "NOVA_SUBSTEPS";` to `src/lib.rs` fails it.

## 6. Step diagnostics promoted

`crates/nova_probe/src/capabilities/stepdiag.rs`, wired into
`NovaProbePlugin` beside the census and the frame-cost breakdown.
`NOVA_PROBE_STEPDIAG` names the CSV; `NOVA_PROBE_STEPDIAG_BODIES` is the regime
floor its end-of-run summary is taken over.

Reworked rather than copied from the sprout:

- `OpenOptions` + `set_len(0)` + `BufWriter`, not `File::create` +
  `Arc<Mutex<File>>` - the pattern `timeline.rs` and `snapshot.rs` were fixed
  away from, because a plain create truncates to offset 0 while an earlier
  `BufWriter` keeps writing at its own.
- `Option<Res<..>>` on avian's diagnostics: they are registered in avian's
  `Plugin::finish`, so an app without `PhysicsPlugins` must not panic.
- Regime selection is IN the capability, not left to offline awk. The summary
  reports the regime's step count, mean bodies and mean constraints beside the
  percentiles, which is what lets a reader see two arms weighed the same world.
- `NOVA_SUBSTEPS`, `NOVA_SLEEP_CENSUS` and `NOVA_NO_PREPHYS_PROPAGATE` do NOT
  come across.

`tasks/20260819-173219/notes-fixed-step.md` and its `measurements/fixed-step/`
are already on master, byte-identical to the sprout's. The instrument was the
only thing the sprout still carried, so it can be removed - I did not run
`sprout rm`, that is the owner's call.

## 7. The page

`docs/environment-variables.md`, in `SUMMARY.md` and routed from
`keeping-docs-in-sync.md`. It is an INDEX: one line per variable, owning crate,
audience, links out. To keep it from becoming a third copy,
`automation-harness.md` lost the four rows that were never driver variables
(`NOVA_PROBE_CONTRACT`, `NOVA_PROBE_SNAPSHOT`, `NOVA_PROBE_SNAPSHOT_FRAMES`,
`NOVA_MENU_BACKDROP`) and points here instead.

# Decided rather than followed

1. **Where each range's picture is taken.** Appending the capture beat to the
   end of a script photographs whatever the script ENDS on. `stress_one_structure`
   verified this the hard way: a 13 KB pure-black PNG, because its last beats
   tear the hull down. Five stress ranges and `system_menu_boot` therefore wrap
   only the beats up to their PEAK and chain the teardown behind the call. The
   other fourteen append. This is a behaviour change: the picture is now the
   scene the range is ABOUT rather than the scene at load, which is strictly
   better - `system_hud_indicators` previously shot black under `NOVA_SHOT` and
   said so in a comment.
2. **`bug_menu_picker` gets no capture beat.** It already shoots
   `scenarios-picker-<id>.png` per selection, and it OWNS its completion via
   `script_reports_done()` - the watcher exits the frame the walk reports done,
   so an appended beat is unreachable. A beat that silently never runs is the
   defect, so it was removed rather than left.
3. **Three names are still spelled twice.** `nova_gameplay`'s `HARNESS_ENVS`,
   `nova_scenario`'s `CAPTURE_DIR_ENV` and `nova_probe_cli`'s `SANDBOXED_VARS[0]`
   each need a name another crate owns. `CONVENTIONS.md` Nova rule 5 says move
   the constant DOWN rather than add a dependency edge - but `nova_autopilot`
   depends on `bevy` alone by design, so there is no lower crate to move it to,
   and making a shipping crate depend on dev tooling for one string is worse.
   The contract test asserts each pair equal instead.
4. **`examples/` keeps its `"NOVA_AUTOPILOT"` literals.** The spec's headline
   counts 18 of them, but the DoD scopes the rule to `crates/` and `src/`, and
   `CONVENTIONS.md` Nova rule 5 exempts `examples/` on purpose: a probe run goes
   red when one drifts. The roster scan skips `examples/` for the same reason.
5. **Released text keeps the old names.** The `[0.9.x]` CHANGELOG sections and
   `web/src/news/0.9.0.md` describe what those versions shipped, so the sed
   stopped at the `[Unreleased]` boundary.
6. **`benchmark/keys/tier1.json` was renamed.** It cites env names in a grading
   key; leaving them would have made the key cite variables that do not exist.
   Its `file:line` citations were already stale and were not touched.

# Found and NOT fixed

- **`stress_many_structures` fails on this host, before anything I changed.**
  `assert the fleet acquired across itself` panics with 98/100 and then 99/100
  hulls acquired on two consecutive runs - a non-deterministic assertion at
  beat 3, ahead of every beat this task touched (`git diff` on that file adds
  nothing before it). Its picture is therefore the one range of the twenty I
  could not verify by running.
- **Seven `[Unreleased]` CHANGELOG entries exceed the 200-character hard max.**
  All pre-existing, none of them mine, none pushed over by the rename.
- **`nova_probe_cli/src/native/env.rs` doc-comment line numbers** and
  `benchmark/keys/tier1.json`'s citations point at lines that moved long ago.

# Next time

The 20-range edit was mechanical only in shape. What actually cost time was
that "append the beat" is not a uniform answer - half the value of this part
was in looking at the PICTURES, and the two that were black were invisible to
`cargo check`, to `cargo test` and to `probe run --correctness-only` alike. A
range that produces an artifact needs the artifact opened, every time.
