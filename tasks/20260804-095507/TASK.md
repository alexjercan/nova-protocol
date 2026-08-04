# Run the rebuilt fleet as CI will and record the sprint's correctness+perf evidence

- PRIORITY: 76
- TAGS: v0.10.0, examples, testing, perf
- KIND: STORY
- ACTIVITY: -
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

- [ ] Run `nix develop --command cargo run -p nova_probe -- run --all --fps`
      under Xvfb :99 with the full rebuilt fleet in place.
- [ ] Confirm the per-category run policy did what the contract says:
      `screenshots/` excluded from `--all`, `stress/` the only category with
      frame-time passes, everything else correctness-only.
- [ ] Record the report (report.html + checks.json) as the sprint's evidence
      and note the frame-time numbers against the previous baseline.
- [ ] Fix or file anything the full-fleet run surfaces that the per-category
      runs did not.

## Definition of Done

- The full fleet runs green as one invocation, with the report retained.
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
