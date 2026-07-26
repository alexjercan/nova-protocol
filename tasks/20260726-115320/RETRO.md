# Retro: NOVA OS monitor shell and visual treatment

- TASK: 20260726-115320
- BRANCH: feature/nova-os-monitor-shell
- REVIEW ROUNDS: 2

## What went well

- The flow gate forced the concrete artifact decision before implementation:
  a drawer-owned Bevy UI monitor tree, not shared status chrome or restyled side
  panels. That kept the code change centered on `hud/drawer.rs` and
  `hud/mod.rs`.
- The test-first structure check paid off. The first drawer test failed before
  the monitor markers existed, and later became the place to pin the review
  fix for overlay z order.
- The doc sweep caught live player-facing surfaces (`web/src/wiki/hud.md` and
  `CHANGELOG.md`) that still described the old side-panel drawer.

## What went wrong

- R1.1 escaped implementation because the first monitor test asserted that CRT
  overlay marker nodes existed, but not that they rendered above terminal
  content. Root cause: I treated a spawned tree as equivalent to visual stacking
  even though Bevy UI child order and local z are load-bearing for overlays.
- R1.2 was a stale comment in a nearby HUD helper. Root cause: the first sweep
  focused on drawer-side wording and did not fully audit comments around the
  changed `HudDrawerExempt` semantics.
- I initially ran a `cargo test` command with two filters, then briefly ran
  cargo checks in parallel in a fresh worktree. The former failed immediately;
  the latter contended on Cargo locks. Root cause: I moved too quickly through
  the red-check step instead of keeping one cargo command per proof.

## What to improve next time

- For visual overlay work, assert the stacking invariant, not only the marker
  existence. If an overlay must affect content, pin local `ZIndex` or child
  order against the content node.
- When changing an exemption marker's semantics, grep and read every function
  that mentions the marker, not just the spawn site and the main test.
- Run one Cargo proof at a time in fresh worktrees, and use one test filter per
  `cargo test` invocation.

## Action items

- [x] Bumped `out-of-context-review-pass` in `LESSONS.md` for the overlay-z
  issue caught by review.
- [x] Bumped `one-cargo-test-filter` in `LESSONS.md` for the failed two-filter
  command.
