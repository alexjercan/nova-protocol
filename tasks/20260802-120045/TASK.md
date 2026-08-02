# Generate showcase evidence and web assets with one command

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.10.0, tooling, probe, screenshot, web
- KIND: STORY
- FLOW STEP: BACKLOG
- PLAN STATUS: DRAFT
- PARENT: 20260802-115955
- DEPENDS ON: 20260802-120025, 20260802-120029

## Story

Extend the existing probe front door so one showcase run produces correctness
reports, optional frame benchmarks, declared PNG captures, and strict packaged
web assets. The command must fail on a broken run, missing producer, missing
capture, stale staging output, or invalid image dimensions.

## Steps

- [ ] Add `--capture` to `probe run`; accept it for native showcase/category
      runs and include capture outputs in each run manifest and aggregate report.
- [ ] Drive capture requests through `nova_autopilot` checkpoints into a clean
      per-run staging directory. Record producer, checkpoint, dimensions, and
      destination for every PNG.
- [ ] Replace the hand-maintained producer split between screenshot examples and
      `gen-web-screenshots.py` with one validated manifest read by both probe and
      packaging.
- [ ] Add strict `--check` packaging: every referenced/generated screenshot has
      one producer, every expected capture exists, dimensions match, and staging
      contains no undeclared leftovers.
- [ ] Make `probe run showcase --capture --fps --release` run correctness,
      eligible FPS windows, captures, packaging, and aggregate reporting with a
      non-zero exit on any failed phase.
- [ ] Document fast correctness-only, capture, and full release-evidence forms.

## Definition of Done

- The one-command run emits aggregate correctness, frame, and capture evidence.
  (cmd: `nix develop --command cargo run -p nova_probe -- run showcase --capture --fps --release`)
- Strict packaging rejects missing, duplicate, stale, or wrongly sized assets.
  (test: `strict_asset_manifest_rejects_incomplete_capture_set`)
- Reports link each generated image to its example and checkpoint producer.
  (test: `aggregate_report_lists_capture_provenance`)
- The packaging script can verify committed assets without recapturing them.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --check`)

## Notes

- Reuse `nova_probe` run manifests and `scripts/gen-web-screenshots.py`; do not
  create a second report format or image copier.
- Pixel comparison remains deferred. v0.10.0 validates provenance, inventory,
  dimensions, and gameplay invariants.
