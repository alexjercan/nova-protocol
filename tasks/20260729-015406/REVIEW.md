# Review: Bug: sandbox nova_probe mod cache from installed local mods

- TASK: 20260729-015406
- BRANCH: fix/probe-profile-sandbox

## Round 1

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context

- [x] R1.1 (MAJOR) crates/nova_probe/src/bin/probe.rs:1361 - the samply pass
  builds its child env inline (`samply_env = vec![BCS_AUTOPILOT,
  BEVY_ASSET_ROOT, DISPLAY]`) and never gets the sandbox, so `probe run <ex>
  --samply` spawns `target/profiling/examples/<ex>` with the operator's real
  `HOME`/XDG and reads (and writes) `~/.local/share/nova-protocol/installed.mods.ron`,
  `~/.config/nova-protocol/enabled_mods.ron` and `settings.ron`. That is exactly
  the failure mode the task exists to close, and it contradicts three claims
  already shipped in this diff: the DoD line "Every native child-run env builder
  carries the sandbox (no pass ships unsandboxed)", the wiki's "Every native
  child run is pointed at an empty, probe-owned profile", and the SKILL.md "Runs
  are PROFILE-SANDBOXED". Fix: build it as `let mut samply_env =
  profile_sandbox::env(out); samply_env.extend(vec![...]);` - and, since the
  point of the bin test is "no pass ships unsandboxed", lift the samply env into
  a `samply_pass_env(root, out, display)` builder and add it to the
  `clean_and_trace_env_carry_the_profile_sandbox` loop, otherwise the same gap
  can reopen silently.
  - Response: Confirmed and fixed as suggested - I re-read probe.rs:1361 rather
    than taking the finding on trust, and the samply pass was indeed a third
    `run_supervised` child with a hand-rolled env. Extracted `samply_pass_env`
    (probe.rs:958) and added it to the test loop. The test is renamed
    `every_child_run_env_carries_the_profile_sandbox` (it now covers clean, fps,
    profiled and samply), and the DoD line names the new test and enumerates the
    passes so a future pass cannot quietly claim coverage it does not have.

- [x] R1.2 (MINOR) crates/nova_probe/src/bin/probe.rs:2395 -
  `clean_and_trace_env_carry_the_profile_sandbox` computes `expected` by
  subtracting `profile_sandbox::inherited()` from `SANDBOXED_VARS`, so on a host
  that exports all three variables `expected` is empty, the inner loop body never
  runs, and the test passes while asserting nothing about the wiring. (It is not
  vacuous today - I confirmed the nix devshell exports none of the three - but it
  is one CI env var away from being so.) Suggest either
  `assert!(!expected.is_empty(), ...)` as a guard, or better, test the wiring
  through an injected lookup (`env_with(..., |_| false)`-style) the way
  `tests/profile_sandbox.rs` already does, so the assertion set is independent of
  the host env.
  - Response: Fixed with the guard, and strengthened: `expected` is now
    `profile_sandbox::env(out)` (pairs, not just names), so each builder's value
    is compared against the exact path the sandbox specifies, and an
    all-three-exported host fails loudly with an explanatory message instead of
    passing empty. Kept the real builders under test rather than an injected
    lookup - the claim being pinned is "the builders carry what the sandbox
    yields", which an injected policy would no longer exercise.

- [x] R1.3 (MINOR) tasks/20260729-015406/TASK.md:3 - STATUS is flipped to CLOSED
  while `## Flow State` still says `FLOW STEP: WORKING` and no REVIEW.md/RETRO.md
  exists, so the task's own DoD proof command fails:
  `tatr check 20260729-015406 --ledger LESSONS.md` exits 1 with
  `closed-missing-review` and `closed-missing-retro`. Revert STATUS to OPEN and
  let the flow close it after review + retro land, so the last DoD line is
  actually green.
  - Response: Pushback (accepted risk, not a defect). The ordering is the flow
    skill's: `/work` sets CLOSED, then `/review` writes REVIEW.md and
    `/compound` writes RETRO.md - both on this branch, before the squash-land -
    which is exactly what `closed-missing-review`/`closed-missing-retro` are
    asking for. The transient rc=1 is the lint correctly naming files that do
    not exist YET; it goes green once this file and RETRO.md are committed, and
    it is re-run at the flow Finish. Reverting to OPEN would instead trip
    `unplanned-in-progress`-style drift and leave the task open after it lands.
    FLOW STEP is advanced per phase, and is now REVIEWING.

- [x] R1.4 (NIT) crates/nova_probe/Cargo.toml:56 - the pre-existing comment
  `# Wasm-only: the harness reads its config from the URL query string in the
  browser (no process env there).` documented the
  `[target.'cfg(target_arch = "wasm32")'.dependencies]` block that followed it;
  the new dev-deps comment and `[dev-dependencies]` are now wedged between them,
  so the Wasm-only comment reads as a description of
  `dirs`/`nova_assets`/`tempfile`. Move `[dev-dependencies]` (and its comment)
  above the Wasm-only comment, or below the wasm target block.
  - Response: Fixed - `[dev-dependencies]` and its comment now sit below the
    wasm target block, so the Wasm-only comment again documents the table it
    introduces.

Observations from the round-1 reviewer (not findings):

- Nothing else in the repo reads operator profile state that this diff misses:
  `dirs::`/`data_dir()`/`config_dir()` across `crates/` and `src/` yields only
  `nova_assets::mod_cache::data_root` (covered by `NOVA_MOD_CACHE_ROOT` +
  `XDG_DATA_HOME`), `nova_assets::mod_prefs` and `nova_menu::settings_store`
  (both `XDG_CONFIG_HOME`), plus `nova_debug::screenshot`'s
  `dirs::download_dir()`, which is an output path under the `debug` feature, not
  read state. `nova_modding` has no independent profile path.
- The `prepare()` wipe is load-bearing rather than redundant: `clean_out_dir`
  only unlinks the named `RUN_ARTIFACTS` + `probe-run.json`, never the
  `profile/` subtree, so a re-run into the same run dir really would inherit the
  previous run's profile without it.
- The cross-process rig is a real fail-first reproduction, not a test that passes
  either way: both integration tests spawn an unsandboxed leg first and assert it
  reads `poison-mod-from-the-operator-profile`, and the isolated-lever leg pins
  that `NOVA_MOD_CACHE_ROOT` alone moves the index while `ENABLED=` still reads
  the poison. Deleting `profile_sandbox::env_with`'s output would turn both
  sandboxed legs red.
- The `!settings.ron.exists()` assertion in
  `child_run_resolves_prefs_and_settings_inside_the_sandbox` is implied by the
  preceding "config dir is under the run dir" assertion and cannot independently
  fail; harmless as documentation, worth knowing it carries no extra proof.
- Docs prose otherwise matches the diff (sandbox layout, per-variable
  preservation, the `XDG_CACHE_HOME` non-goal, the macOS caveat), with R1.1's
  "every native child run" being the one overclaim.

Proof commands the round-1 reviewer ran (worktree, via `nix develop --command`;
full suite and clippy skipped per AGENTS.md): `cargo test -p nova_probe --test
profile_sandbox` (3 passed), `--lib profile_sandbox` (3 passed), `--bin probe
clean_and_trace_env_carry_the_profile_sandbox` (1 passed), `cargo check
--workspace --all-targets` (clean), `cargo fmt --check` (rc=0), the docs `rg`
(hits at development.md 333/334/345), and `tatr check` (rc=1 - see R1.3).

## Round 2

- VERDICT: APPROVE
- REVIEWER: out-of-context

Round-1 findings, verified against `git show b4fb78ca` and the full branch diff:

- R1.1 (MAJOR) RESOLVED. `samply_pass_env` (probe.rs:963) opens with
  `profile_sandbox::env(out)` and is the only source of the samply child's env.
  All four `run_supervised` call sites - clean/sweep (:1262), fps (:1305),
  profiled (:1345), samply (:1380) - now carry the sandbox. The other
  `Command::new` sites (git, Xvfb, cargo, trunk, chromium) run no Nova example
  natively; the web pass is the documented out-of-scope case.
- R1.2 (MINOR) RESOLVED, both halves verified rather than taken on trust.
  (a) With all three variables exported the test now FAILS loudly on the
  `!expected.is_empty()` guard instead of passing empty. (b) Mutation check: the
  reviewer temporarily replaced `samply_pass_env`'s `profile_sandbox::env(out)`
  with an empty vec, the test FAILED, and the file was restored (working tree
  confirmed clean afterwards).
- R1.3 (MINOR) PUSHBACK ACCEPTED, finding withdrawn. With REVIEW.md committed,
  `tatr check` drops `closed-missing-review`; the remaining
  `closed-not-approved` (this round flips it) and `closed-missing-retro`
  (/compound writes it on this branch) are flow ordering artifacts, and the DoD
  command goes green at Finish.
- R1.4 (NIT) RESOLVED. The wasm target block sits under its own comment again.

No new findings. The round-2 diff touches only the four things asked for.

- Tradeoff noted, not changed: the guard turns the previously-silent vacuous
  case into a hard failure on a host exporting all three variables. Right
  direction (a wrong answer beats no answer) and the nix devshell exports none
  of them, but a CI that sets `XDG_DATA_HOME`/`XDG_CONFIG_HOME` would go red for
  environmental reasons. Accepted as shipped.

Proof commands re-run this round (worktree, `nix develop --command`; full suite
and clippy skipped per AGENTS.md): `--test profile_sandbox` (3 passed),
`--lib profile_sandbox` (3 passed, incl. `profile_sandbox_preserves_operator_overrides`),
`--bin probe every_child_run_env_carries_the_profile_sandbox` (1 passed, plus the
two deliberate negative runs above), `cargo check --workspace --all-targets`
(clean), `cargo fmt --check` (rc=0), the docs `rg`, and `tatr check` (rc=1 with
only the two ordering lints above).

No open `manual:` DoD items - every proof on this task is a `test:` or `cmd:`.
