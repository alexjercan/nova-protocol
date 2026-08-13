# Notes

## Release result

All v0.10.0 child work is closed. The delivered release demonstrates itself
through one Nova-owned pipeline:

- `nova_autopilot` owns named predicate-driven steps, input synthesis, capture
  acknowledgement, deadlines, and completion.
- The five-category example fleet drives real systems and declares probe
  capabilities instead of inheriting launch-side exemptions.
- Probe reports carry correctness, timeline, invariant, frame, trace, build,
  renderer, and host evidence while reporting undeclared measurements as N/A.
- Screenshot producers and `gen-web-screenshots.py` maintain current website
  images. Scenario thumbnails remain deliberate generated placeholders pending
  owner art.
- Warning cleanup, tutorial refresh, semantic ship parts, link-point structure,
  typed scenario queries, lifecycle edges, and routed documentation all landed
  in the same release.

## Acceptance

- Owner accepted the example fleet, generated images, scenario placeholders,
  tutorial at desktop and narrow widths, and probe evidence.
- All 27 current cataloged examples pass across the final completed probe sweep
  and focused reruns.
- The v0.10.0 feature post was reviewed and accepted on 2026-08-13.
- A new 1920x1080 exploded Racer capture from `parts_viewer` was inspected and
  selected for the post and news card.

## Release-gate correction

The first CI run at the release frontier passed default-feature, wasm, and
license jobs but stopped in clippy on an `unnecessary_sort_by` finding in
`parts_viewer`. The sort now uses `sort_by_key`; this must pass the next CI run
before the release tag is created.
