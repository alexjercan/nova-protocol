# Bug: sandbox nova_probe mod cache from installed local mods

- PRIORITY: 81
- TAGS: v0.9.0, bug, tooling, probe, modding
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

As a developer running `nova_probe` across older commits, I want harness runs to ignore my local Nova profile state, so that a backward-incompatible cached mod structure under `~/.local/share/nova-protocol` or a saved enabled-mod preference cannot make probe fail for reasons unrelated to the commit being measured.

`nova_assets::mod_cache` already supports `NOVA_MOD_CACHE_ROOT`; tests use it to point the downloaded-mod cache at a temp root. The bug is that `nova_probe` spawned native runs currently inherit the user's environment unless the caller remembered to set this manually, so examples can read `dirs::data_dir()/nova-protocol/installed.mods.ron` and cached `mods://` files from the real profile. `nova_assets::mod_prefs` also reads `dirs::config_dir()/nova-protocol/enabled_mods.ron`, and `nova_menu::settings_store` reads `settings.ron`; those should not make probe output depend on the operator's desktop profile either.

## Steps

- [x] Reproduce FIRST, cross-process: add `crates/nova_probe/tests/profile_sandbox.rs` that poisons a fake profile (`<tmp>/data/nova-protocol/installed.mods.ron` + `<tmp>/config/nova-protocol/enabled_mods.ron`) and spawns a real child process (the test binary re-executed with a sentinel env var, running a marker-gated `resolver_child` test that prints what `nova_assets::mod_cache::read_index()`, `nova_assets::mod_prefs::load_enabled_ids()` and `dirs::data_dir()`/`config_dir()` resolve to). Child inheriting the poisoned env WITHOUT the sandbox must resolve INTO the poisoned dirs - that red is the repro. SHIPPED: the poisoned profile is staged through `HOME` (the desktop default `dirs` falls back to), the dev-deps are `nova_assets` + `dirs` + `tempfile`, and `nova_menu::settings_store` is private so the settings store is pinned through the shared `dirs::config_dir()` it resolves on.
- [x] Add `crates/nova_probe/src/profile_sandbox.rs` (new lib module, exported from `lib.rs`): `env(run_dir) -> Vec<(String, String)>` (pure) returning `NOVA_MOD_CACHE_ROOT=<run_dir>/profile/mods`, `XDG_DATA_HOME=<run_dir>/profile/data`, `XDG_CONFIG_HOME=<run_dir>/profile/config`, plus `env_with` (injected lookup, so the override policy is testable without touching the process env) and `prepare(run_dir)`, which WIPES and recreates the tree - a re-run into the same run dir must not inherit the previous run's profile - and announces inherited variables.
- [x] Wire it into both native child-run env builders in `crates/nova_probe/src/bin/probe.rs` - `clean_pass_env` (used by the clean, sweep and fps passes) and `trace_pass_env` - so every `run_supervised` child gets the sandbox. `build_example`'s cargo invocation stays untouched; the web/chromium pass is out of scope (browser profile, no Nova profile state).
- [x] Override policy: an operator-set `NOVA_MOD_CACHE_ROOT` / `XDG_DATA_HOME` / `XDG_CONFIG_HOME` in probe's OWN environment is PRESERVED per-variable (probe does not push its value for that var) and probe prints one `probe: profile sandbox: <VAR> inherited from the environment` line, so "probe my real installed mods" stays possible. Pin that choice in a test.
- [x] Document in `web/src/wiki/dev/development.md` (probe section): probe runs are profile-sandboxed by default, the sandbox lives under the run dir, and the per-variable env override is the escape hatch. Add the same in the `profile_sandbox` module rustdoc.
- [x] Re-read the produced task/docs/code artifacts and verify the wording matches the final behavior; run `tatr check` and the fmt/check gates.

## Definition of Done

- Native probe child runs no longer read `~/.local/share/nova-protocol` or the platform data-dir mod cache by default: a spawned child under a poisoned profile env resolves the mod cache inside the run-dir sandbox, and the same child without the sandbox env resolves the poisoned path (fail-first proof recorded). (test: `cargo test -p nova_probe --test profile_sandbox child_run_resolves_mod_cache_inside_the_sandbox`)
- Native probe child runs no longer read the user's `enabled_mods.ron` or saved settings by default. (test: `cargo test -p nova_probe --test profile_sandbox child_run_resolves_prefs_and_settings_inside_the_sandbox`)
- Caller-provided profile-state overrides (`NOVA_MOD_CACHE_ROOT`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`) are preserved per-variable and announced, and a test pins that choice. (test: `cargo test -p nova_probe profile_sandbox_preserves_operator_overrides`)
- Every native child-run env builder carries the sandbox - clean/sweep/fps, profiled, samply - so no pass ships unsandboxed. (test: `cargo test -p nova_probe --bin probe every_child_run_env_carries_the_profile_sandbox`)
- The relevant probe docs mention the default profile-state sandbox and override behavior. (cmd: `rg -n "NOVA_MOD_CACHE_ROOT|XDG_CONFIG_HOME|XDG_DATA_HOME|mod cache|installed mods|enabled_mods" web/src/wiki/dev/development.md crates/nova_probe crates/nova_assets/src/mod_cache.rs crates/nova_assets/src/mod_prefs.rs`)
- The task remains flow-managed and waits for plan approval before work starts. (cmd: `tatr check 20260729-015406 --ledger LESSONS.md`)

## Notes

- Scheduling: v0.9.0 high-priority probe/tooling bug, slotted above the existing priority 80 probe regressions because local installed state can falsify any older-commit measurement.
- Relevant files: `crates/nova_probe/src/bin/probe.rs`, `crates/nova_assets/src/mod_cache.rs`, `web/src/wiki/dev/development.md`.
- Assumption: this task should sandbox local profile state for probe children; shipped `assets/mods.catalog.ron` remains part of the app and should still load normally.
- Sandbox location: `<run-dir>/profile/{mods,data,config}` (run dir = `probe-runs/<short-commit>/<example>/`), so isolation is per run by construction and the sandbox is visible next to the report instead of hidden in a temp dir. Consequence: downloaded-mod state is never shared between runs (probe runs do not download mods; shipped `assets/` content is unaffected).
- Platform note: `XDG_DATA_HOME`/`XDG_CONFIG_HOME` are the `dirs` crate's Linux resolution. On macOS `dirs` uses `~/Library/...` and ignores XDG, so there the sandbox rests on `NOVA_MOD_CACHE_ROOT` alone; probe already requires Xvfb, so Linux is the supported host. Documented, not worked around.
- Risk accepted: redirecting `XDG_DATA_HOME` also hides user-local Vulkan ICDs from the child; system ICDs (`/usr/share`, `/run/opengl-driver`) and the explicit `VK_ICD_FILENAMES` the `--render sw` path sets are unaffected. `XDG_CACHE_HOME` is deliberately NOT redirected, so the wgpu/mesa shader cache keeps working and FPS numbers stay comparable.
- Implementation hint: prefer setting child-process environment variables in `nova_probe` to probe-owned empty directories over changing the game/mod-cache/settings defaults globally. `NOVA_MOD_CACHE_ROOT` is the direct mod-cache override; XDG config/data isolation is the belt-and-suspenders protection for older commits and for enabled-mod/settings stores that do not have a dedicated override.

## Evidence

- Reproduction (cross-process, `crates/nova_probe/tests/profile_sandbox.rs`): a child spawned with a poisoned `HOME` and NO sandbox env prints `INDEX=Some([InstalledModRecord { id: "poison-mod-from-the-operator-profile", .. }])` and `ENABLED=Some(["poison-mod-from-the-operator-profile"])`. That is the bug: a run reading the operator's installed mods. With the sandbox env both read `None` and `DATA_DIR`/`CONFIG_DIR` sit under `<run-dir>/profile/`. An isolated-lever leg pins that `NOVA_MOD_CACHE_ROOT` alone moves the index while the config stores still read the poison.
- End-to-end (`cargo run -p nova_probe -- run playable`, commit e109b5cf): verdict OK (process_exit / run_completed / reached_playing / invariants_held / log_clean PASS, fps SKIPPED - not measured), and probe printed `probe: profile sandbox: .../probe-runs/e109b5cf/playable/profile (mod cache, data, config)`. The live proof the wiring holds: the run WROTE its `enabled_mods.ron` into `probe-runs/e109b5cf/playable/profile/config/nova-protocol/` instead of the operator's `~/.config/nova-protocol/`.
- Skipped locally per AGENTS.md: the full `cargo test` / `cargo clippy` suites (CI runs them). Run here: the new + touched tests (`--test profile_sandbox` 3 passed, `--lib profile_sandbox` 3 passed, `--bin probe` 26 passed), `cargo check --workspace --all-targets`, `cargo fmt`, and `cargo doc -p nova_probe --no-deps` (warning-free).
