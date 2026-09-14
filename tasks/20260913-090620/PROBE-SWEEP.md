# Probe sweep, master against v0.13.2 at 8 workers, 2026-09-14

## Claim

The remaining regression is local to `wfc_arena`. On every other example
that produced frames, master `026ecdf4e` sits within 0.6% to 6.1% of
v0.13.2 at the same 8-worker pool, and master's numbers came from a
contaminated host, which can only make master look worse. The sweep also
found that master's probe cannot capture frames on six examples because the
new scenario-load clock hold trips the capture abort, and three master-only
correctness failures that are not performance.

The sweep was stopped by the owner after v0.13.2 finished 10 of 18 examples.

## Method

Both sides ran `probe run <spec> --release --repeat 5 --display :0`
(release with `--features debug`, 1280x720 window, `autonovsync` present
mode, 180 warm-up and 900 captured frames per repeat, five fps repeats in
separate processes after a clean pass, then a traced pass). The probe's
window and frame counts differ from the 1920x1080 sets in `SHARDS.md`, so
the two families of numbers must not be mixed.

- master: the main checkout at `026ecdf4e`, 24 examples, 09:55 to 14:26
  local. rust-analyzer ran `cargo check --workspace` against the same
  target dir throughout, and another session ran `cargo test -p
  nova_console` at 10:15 to 10:25. Load sampler: median 1.43, p90 6.07,
  max 12.76. Which peaks fell inside a capture window is not recorded.
- v0.13.2: a `git archive b3c6f579c` export under
  `/tmp/nova-perf-20260913/sources/`, patched only with the
  `NOVA_EXPERIMENT_COMPUTE_CAP` environment cap, run with the cap at 8 and
  the same 18-example spec (master's 24 minus the six headless examples
  that do not exist there). 14:34 to 16:45 local, stopped by PID. All 50
  fps logs carry `worker experiment: compute=8 io=4 async=4`. Load stayed
  under 3.5 inside every measured window; the sampler peaks were the
  release builds between examples.
- Comparison: `/tmp/nova-probe-20260914/tools/compare.py`, which
  recomputes admission the way `nova_probe_cli/src/evaluation/frames.rs`
  does (20% band around the low median of repeat means and medians) and
  reports (master - v0.13.2) / v0.13.2. Its recomputation matched every
  `checks.json` on both sides.

## Evidence: timing

Mean and median are the set references over all captures, p99 the median
over admitted repeats, in milliseconds.

| Example | v0.13.2@8 mean / p99 | Adm | master mean / p99 | Adm | Mean delta | Ranges |
|---|---:|---:|---:|---:|---:|---|
| wfc_arena | 9.45 / 14.96 | 5/5 | 13.74 / 37.53 | 3/5 | +45.3% | disjoint |
| stress_bullets | 2.34 / 3.33 | 5/5 | 2.43 / 3.38 | 5/5 | +3.7% | overlap |
| stress_torpedoes | 8.67 / 44.65 | 5/5 | 9.13 / 53.50 | 3/5 | +5.4% | overlap |
| system_player_path | 2.72 / 4.26 | 3/5 | 2.73 / 4.34 | 3/5 | +0.6% | overlap |
| system_thrust_and_plume | 1.97 / 2.68 | 5/5 | 2.09 / 2.74 | 5/5 | +6.1% | disjoint |

- Repeat-mean ranges: wfc_arena 8.79-10.39 against 11.91-13.96;
  system_thrust_and_plume 1.88-2.03 against 2.04-2.13 (a 0.1 ms gap that
  the noise test calls disjoint, on a contaminated master).
- The master `wfc_arena` row is not the arena number. It admitted three of
  five repeats and its p99 is 2.5x v0.13.2's, on a host running a workspace
  check. The clean quiet-host set on the same commit in `SHARDS.md`
  (9.77 mean against 8.10, +20.6%; single fixed step 10.35 against 9.20,
  +12.5%) stands as the measured gap.
- Every non-arena example is within 6.1%. The regression is not in the
  renderer, the physics step, or the projectile engine at large; it lives
  in what the 4v4 arena does with ships, weapons, and wrecks. That matches
  the ranked candidates in `REVIEW.md`.

## Evidence: correctness

Verdicts per example on the ten that finished on both sides.

| Example | v0.13.2@8 | master | Cause on master |
|---|---|---|---|
| wfc_arena | OK | OK | |
| stress_bullets | OK | OK | |
| stress_torpedoes | FAIL | FAIL | audio-underrun log lines, both sides |
| system_player_path | FAIL | FAIL | "gun the prey down" stall, both sides |
| system_thrust_and_plume | OK | OK | |
| carve_asteroids | FAIL, 5/5 frames | FAIL, no frames | PDC stall both; capture abort master |
| stress_many_structures | OK, 4/5 frames | FAIL, no frames | capture abort at frame 129 |
| system_hull_damage | OK, 5/5 frames | FAIL, no frames | capture abort at frame 32 |
| stress_one_structure | OK, 5/5 frames | FAIL, panic | mass 0 after aggregation |
| system_blast_penetration | FAIL, no frames | FAIL, no frames | armed and silent, both sides |

Master-only, on examples v0.13.2 never reached before the stop:

- `system_menu_boot`, `system_scenario_grammar`: capture abort at warm-up
  frame 0. `system_cinematic`: capture abort at warm-up frame 143.
- `system_field_controls`: panic at `system_field_controls.rs:410`, "a grip
  on one axis moves by the ROW's step ... the step is being resolved a
  second time from the axis path".
- `system_headless_crt`: autopilot step "novaos_next cycles the ring onto
  a contact" stalled after 30 s.

## Finding: the load hold aborts master's captures

Every capture abort on master reads `ABORTED reason=simulation_stopped`
and is preceded by `hold_for_scenario_load: holding the world while the
scenario builds`. The hold is `crates/nova_scenario/src/loader/gate.rs:76`
(`hold_for_scenario_load`, on `ScenarioLoaded`, `clocks.hold(
FreezeOwner::ScenarioLoad)`), released by `release_when_scenario_is_built`
once the event world has settled. The file is new since v0.13.2
(`6864a8d9c`).

The probe arms its capture at "reached Playing" and aborts when
`Time<Virtual>` is paused inside warm-up or capture. Two shapes:

- Playing arrives before the release. `carve_asteroids`: Playing at
  07:31:39.494, abort at .564 in warm-up frame 0, release at .606.
  `system_menu_boot` and `system_scenario_grammar` abort at frame 0 the
  same way; `system_cinematic` at frame 143.
- The example reloads its scenario. `stress_many_structures` (`loop_from`
  re-triggers `LoadScenario`, `stress_many_structures.rs:179` and `:463`)
  and `system_hull_damage` (`system_hull_damage.rs:336` and `:934`) take a
  second hold a few seconds in, at frame 129 and 32. v0.13.2 reloads the
  same way (four `ScenarioLoaded` lines per run) but has no hold, so it
  captured 4.47 ms and 2.34 ms.

The focus-loss pause is not involved: `FocusPause` defaults to off under a
harness environment (`crates/nova_menu/src/pause.rs:176`), and no run log
shows it. This is a harness defect, not a game one: the capture should not
arm, or should suspend, while `ScenarioLoadGate` is held. Until it is
fixed, six of the 18 measurable examples produce no frames on master and
any further sweep is blind on them. No fix was started.

## Limits

- Master's timing rows are contaminated; only their direction is safe to
  read. A quiet-host re-run of the nine frame-producing master examples
  was planned and not run.
- Eight v0.13.2 examples were never run: `system_attitude_hold` (partial),
  `system_menu_boot`, `widget_zoo`, `system_cinematic`,
  `system_scenario_grammar`, `system_turn_limit`, `system_input_modes`,
  `system_field_controls`. Their master-only failures are therefore
  master-only by absence of a reference, except `stress_one_structure`,
  which v0.13.2 passes 5/5.
- The v0.13.2 tree has no `.git`, so its commit dir is `unknown` and the
  probe's index was never written. A stray empty `/tmp/.git` broke the
  bare `nix develop` flakeref; the lane copied `flake.nix`, `flake.lock`,
  `rust-toolchain.toml` and `Cargo.toml` verbatim into
  `/tmp/nova-probe-20260914/flakeenv` and used a `path:` flakeref. The
  dev shell reads nothing else from the tree. `/tmp/.git` was left alone.
- The probe window is 1280x720, not the 1920x1080 of the acceptance sets.
- Set C (the ten examples whose sources changed since v0.13.2) was not
  run.

Raw data: `/tmp/nova-probe-20260914/{master,v0132-cap8}/`, sweep logs,
lane records `*-sweep.meta.txt`, load samples `v0132-load.csv`, the
comparison report generator under `tools/`. Nothing was pushed.
