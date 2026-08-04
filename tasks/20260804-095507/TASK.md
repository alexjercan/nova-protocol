# Run the rebuilt fleet as CI will and record the sprint's correctness+perf evidence

- PRIORITY: 74
- TAGS: v0.10.0, examples, testing, perf
- KIND: STORY
- ACTIVITY: PLANNING
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-115955
- DEPENDS ON: 20260804-093910, 20260804-094021, 20260804-094006, 20260804-093934, 20260804-093950, 20260804-093855

## Story

Run the rebuilt fleet the way CI and probe will, and record the resulting
report as the sprint's correctness + perf evidence.

This is `20260802-120029`'s last Step and one of the epic's Done Means. It was
dropped when the roster spike (`20260804-003244`) redistributed that task, and
is restored here as the closing task of the chain: it is the only place the
rebuilt fleet is exercised as a WHOLE rather than per category.

## Steps

- [ ] Run what CI ACTUALLY runs, first and separately:
      `xvfb-run --auto-servernum cargo test -p nova-protocol --test
      examples_smoke --features debug` (`.github/workflows/ci.yaml:108`). This
      is the fleet's real gate; probe is not in any workflow.
- [ ] Check the smoke step still fits the job's `timeout-minutes: 60`
      (ci.yaml:24) with the rebuilt fleet. The smoked count rises (~22 -> ~25:
      `widget_zoo` joins, three `stress/` runs are new and heavy) while three
      runs retire. If it does not fit, that is a CI-budget fix, not a
      "record the evidence" fix - file it.
- [ ] Run the evidence pass:
      `nix develop --command cargo run -p nova_probe -- run --all --fps`
      under Xvfb :99 with the full rebuilt fleet in place.
- [ ] Confirm the per-category run policy did what the contract says:
      `screenshots/` excluded from `--all`, `stress/` the only category with
      frame-time passes, everything else correctness-only.
- [ ] Commit the evidence under `tasks/20260804-095507/probe-results/`
      (report.html + checks.json + frametime.csv), following the
      `tasks/20260716-123551/perf-results/` precedent for the v0.7.0 baseline.
- [ ] Compare frame times against the v0.7.0 baseline where a comparison
      EXISTS, and say plainly where it does not: `scene_baseline` still loads
      `asteroid_field`, so that series is comparable; the `broadside-*` series
      dies with the retired example; `many_bodies` / `many_sections` /
      `many_projectiles` are new, so this run IS their baseline.
- [ ] FILE anything the full-fleet run surfaces that the per-category runs did
      not. Fix only one-line corrections - this task is the sprint's last, and
      it is where leftover work goes to hide.

## Definition of Done

- The fleet passes the gate CI actually enforces.
  (cmd: `xvfb-run --auto-servernum cargo test -p nova-protocol --test examples_smoke --features debug`)
- The full fleet runs green as one probe invocation, with the report retained
  under `tasks/20260804-095507/probe-results/`.
  (cmd: `nix develop --command cargo run -p nova_probe -- run --all --fps`)
- No example carries a hand-rolled completion guard or beat-boolean script.
  (cmd: `! rg -n "run ended with the scripted run unfinished|playing_since" examples`)
- The catalog, the on-disk layout and the smoke lists agree.
  (test: `catalog_matches_disk`)

## Notes

- Carried forward from `20260802-120029` Step 9 and its `playing_since` absence
  grep, which would otherwise have been lost when that task closed SUPERSEDED.
- Depends on every other task in the chain; it is the last one.
- Examples must be RUN under Xvfb :99, not only checked.

The title says "as CI will", and that was imprecise. Two different runs:

| | Gate | Evidence |
| --- | --- | --- |
| What | `cargo test --test examples_smoke` | `probe run --all --fps` |
| Who | CI, every PR (`ci.yaml:108`) | the owner, once, here |
| Proves | reaches Playing, no panic, no command errors | correctness + frame time, per category |
| Fails the build | yes | no - probe is in no workflow |

Both matter and neither substitutes for the other. The smoke gate is what
regressions actually hit; the probe report is what the epic's Done Means asks
for as the v0.10.0 demonstration. Run the gate FIRST - a red smoke makes the
probe report meaningless, and the smoke run is much cheaper.

- Runtime is worth estimating before committing to "one green invocation" as a
  proof shape. `stress/` uses the full 180 + 900 frame window
  (`capture.rs:94,97`), and `env.rs` sizes each fps pass's deadline at
  `1080 / FPS_FLOOR(2.0) + 45s` = 585s. That is a worst-case bound, not the
  expected time, but four `stress/` fps passes plus a correctness pass over
  ~21 other examples is a long single command. If it proves unwieldy, running
  `--all` and `stress --fps` separately still discharges the Done Means.
- Evidence location follows `tasks/20260716-123551/perf-results/`, which
  committed per-scene JSON + `frametime.csv` for the v0.7.0 baseline. Same
  shape, same reason: a generated artifact is only evidence if it outlives
  `target/`.
