# Bug: sandbox nova_probe mod cache from installed local mods

- STATUS: OPEN
- PRIORITY: 81
- TAGS: v0.9.0,bug,tooling,probe,modding

## Story

As a developer running `nova_probe` across older commits, I want harness runs to ignore my local Nova profile state, so that a backward-incompatible cached mod structure under `~/.local/share/nova-protocol` or a saved enabled-mod preference cannot make probe fail for reasons unrelated to the commit being measured.

`nova_assets::mod_cache` already supports `NOVA_MOD_CACHE_ROOT`; tests use it to point the downloaded-mod cache at a temp root. The bug is that `nova_probe` spawned native runs currently inherit the user's environment unless the caller remembered to set this manually, so examples can read `dirs::data_dir()/nova-protocol/installed.mods.ron` and cached `mods://` files from the real profile. `nova_assets::mod_prefs` also reads `dirs::config_dir()/nova-protocol/enabled_mods.ron`, and `nova_menu::settings_store` reads `settings.ron`; those should not make probe output depend on the operator's desktop profile either.

## Steps

- [ ] Reproduce the failure with a probe/native child-run test or harness that creates incompatible installed-mod cache state outside the intended run and proves the spawned example would read it without sandboxing.
- [ ] Update `crates/nova_probe` native run environment construction to set `NOVA_MOD_CACHE_ROOT` plus config/data home isolation (`XDG_CONFIG_HOME`/`XDG_DATA_HOME` on Linux or the appropriate cross-platform equivalent) to empty probe-owned locations unless the caller intentionally supplies an override.
- [ ] Ensure the sandbox paths are per run or otherwise isolated enough that probe runs do not share downloaded-mod state, enabled-mod preferences, settings, or stale prior probe state with the user's real profile.
- [ ] Keep the behavior documented in the probe/development docs so agents know probe isolates mod cache state by default and how to override it when intentionally testing installed mods.
- [ ] Re-read the produced task/docs/code artifacts and verify the wording matches the final behavior.

## Definition of Done

- Native probe child runs no longer read `~/.local/share/nova-protocol` or the platform data-dir mod cache by default. (test: `probe_native_env_sets_mod_cache_root` or equivalent)
- Native probe child runs no longer read the user's `enabled_mods.ron` or saved settings by default. (test: `probe_native_env_sandboxes_config_state` or equivalent)
- Caller-provided profile-state overrides (`NOVA_MOD_CACHE_ROOT`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, or their supported equivalents) are either preserved intentionally or rejected with explicit documented behavior, and tests pin that choice. (test: `probe_native_env_profile_override_behavior` or equivalent)
- The relevant probe docs mention the default profile-state sandbox and override behavior. (cmd: `rg -n "NOVA_MOD_CACHE_ROOT|XDG_CONFIG_HOME|XDG_DATA_HOME|mod cache|installed mods|enabled_mods" web/src/wiki/dev/development.md crates/nova_probe crates/nova_assets/src/mod_cache.rs crates/nova_assets/src/mod_prefs.rs`)
- The task remains flow-managed and waits for plan approval before work starts. (cmd: `tatr check 20260729-015406 --ledger LESSONS.md`)

## Notes

- Scheduling: v0.9.0 high-priority probe/tooling bug, slotted above the existing priority 80 probe regressions because local installed state can falsify any older-commit measurement.
- Relevant files: `crates/nova_probe/src/bin/probe.rs`, `crates/nova_assets/src/mod_cache.rs`, `web/src/wiki/dev/development.md`.
- Assumption: this task should sandbox local profile state for probe children; shipped `assets/mods.catalog.ron` remains part of the app and should still load normally.
- Implementation hint: prefer setting child-process environment variables in `nova_probe` to probe-owned empty directories over changing the game/mod-cache/settings defaults globally. `NOVA_MOD_CACHE_ROOT` is the direct mod-cache override; XDG config/data isolation is the belt-and-suspenders protection for older commits and for enabled-mod/settings stores that do not have a dedicated override.

## Flow State

- FLOW STEP: PLANNING
