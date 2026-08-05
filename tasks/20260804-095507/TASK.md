# Run the rebuilt fleet as CI will and record the sprint's correctness+perf evidence

- PRIORITY: 74
- TAGS: v0.10.0, examples, testing, perf
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE
- PARENT: 20260802-115955
- DEPENDS ON: 20260804-093910, 20260804-094021, 20260804-094006, 20260804-093934, 20260804-093950, 20260804-093855

## Story

Run the rebuilt fleet as ONE invocation - the only place that happens - and
read the resulting report as the sprint's correctness + perf evidence.

This is the closing task of the example-fleet chain and one of the epic's Done
Means (`20260802-115955:37,45`). Each of the six preceding tasks proved its own
category with `probe run <category>`; nothing had exercised them together, so
cross-category effects were unproven.

Scope was cut by the owner before planning - see `DECISION.md`. The report is
INSPECTED, not committed; there is no baseline to compare against; the CI
budget question is left to CI.

## Steps

- [x] Run the evidence pass:
      `nix develop --command xvfb-run --auto-servernum cargo run -p nova_probe -- run --all --fps`
      on `master`, in place, writing to probe's default `probe-runs/`
      (gitignored - `.gitignore:252`). No `--baseline`, no custom `--out`.
- [x] Confirm the per-category run policy did what the contract says, against
      the RUN and not just `CATEGORY_POLICIES`: `screenshots/` excluded from
      `--all`, `stress/` the only category with frame-time passes, everything
      else correctness-only.
- [x] Read the verdict from `checks.json` - together with `measured`, never
      alone - and account for every unmeasured check.
- [x] Write the frame-time numbers into `NOTES.md`. Nothing else outlives
      `target/`, so this is the only record they get.
- [x] FILE anything the full-fleet run surfaced that the per-category runs did
      not; fix only one-line corrections here.
      -> `20260805-091146` (`many_projectiles` frame spikes).
- [x] Record the owner's intermittent `examples_smoke` failure. Diagnosed and
      REPRODUCED, not fixed - owner call. -> `20260805-091151`
      (`click_named` same-frame press), moved into the v0.10.0 sprint at
      priority 84 because it gates CI.
- [x] One-line correction the full-suite run surfaced: `nova_probe`'s
      `frame_time_categories_capture_and_the_rest_record_a_reason` still
      asserted `perf/` was a frame-time category. `perf/` was absorbed into
      `stress/` by `20260804-094006`; the stale "transitional row" is gone.
- [x] Verify the whole suite is green before closing. UNBLOCKED:
      `20260805-091151` landed (`87bcb956`) and both DoD commands were re-run
      green - probe `--all --fps` exit 0 with every category PASS on
      correctness, workspace suite exit 0 at 1543 passed / 0 failed,
      `examples_smoke` 9/9. See `NOTES.md`, "The closing run".

## Definition of Done

- The full fleet runs green as one probe invocation.
  (cmd: `nix develop --command xvfb-run --auto-servernum cargo run -p nova_probe -- run --all --fps`)
- No example carries a hand-rolled completion guard or beat-boolean script.
  (cmd: `! rg -n "run ended with the scripted run unfinished|playing_since" examples`)
- The catalog, the on-disk layout and the smoke lists agree.
  (test: `catalog_matches_disk`)
- Every category on disk has an explicit probe policy row.
  (test: `every_category_has_a_probe_policy`)
- The workspace suite passes.
  (cmd: `nix develop --command xvfb-run --auto-servernum cargo test --workspace --features debug`)
- The frame-time numbers and the run's two caveats are recorded in `NOTES.md`,
  because no artifact is committed. (manual: read `NOTES.md`)

## Notes

- Carried forward from `20260802-120029` Step 9 and its `playing_since` absence
  grep, which would otherwise have been lost when that task closed SUPERSEDED.
- Depends on every other task in the chain; it is the last one.
- Examples must be RUN under Xvfb, not only checked.
- "as CI will" was imprecise. CI's fleet gate is
  `cargo test -p nova-protocol --test examples_smoke --features debug`
  (`.github/workflows/ci.yaml:108`); probe is in no workflow. The two runs are
  tabulated in `NOTES.md` and neither substitutes for the other.
- Out of band, on owner instruction and not part of this plan: the
  `nova_autopilot` prelude re-export bookkeeping tests were removed
  (`2a8bd05b`).
