# nova_probe: runner CLI + example opt-in + docs (one command per autopilot example; fold perf scripts into subcommands)

- STATUS: OPEN
- PRIORITY: 66
- TAGS: v0.8.0, spike, tooling, docs

## Goal

Runner CLI + example opt-in + docs: one entrypoint (`cargo run -p nova_probe
-- ...`) that runs a named autopilot example headless (native or `--platform
web`) in TWO PASSES - pass 1 clean (FPS + timeline + invariants), pass 2
profiled (chrome trace + samply, FPS discarded; review M2) - collects the
artifacts into the run directory, and writes the report; `--profile` enables
pass 2, `--export csv,html,json`. Fold `perf-baseline.sh` / `perf-web.sh`
into subcommands. Wire the tool into the post-feature workflow and document it.

## Notes

- Spike: tasks/20260719-112011/SPIKE.md (Architecture, REVISED after review
  round 1: two-pass runner, run-directory artifact).
- The plugin (RunProbePlugin) is opt-in per example, inert unless armed.
- Docs: dev wiki development.md (the Performance section) + README tools section
  (task 20260718-152205) + how to run it as the post-feature correctness+perf
  check.
- Depends on T5 (the report) being usable end to end.
