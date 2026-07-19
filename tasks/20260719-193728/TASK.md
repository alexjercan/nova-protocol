# Reorganize examples/ by purpose (sections, gameplay, ui, screenshots, perf): category dirs + per-category smoke tests

- STATUS: OPEN
- PRIORITY: 62
- TAGS: v0.8.0,refactor,testing,examples

## Goal

Group the 21 flat examples by PURPOSE so testing by category is one command:
section correctness, full-gameplay autopilot runs, UI flows, screenshot
producers, FPS-traced perf scenes. Directory = purpose; the global number
KEEPS being the curriculum reading order (AGENTS.md: "examples 01-19 the
curriculum") - two orthogonal axes, no renumbering.

Proposed layout (names are a taste call, the split is the substance):

- `examples/sections/` - 01_controller_section .. 07_com_range (7; 04 keeps
  its sibling `04_turret_section/slider.rs` module dir - `mod slider;`
  resolves relative to the file, so the pattern survives the move intact)
- `examples/gameplay/` - 08_scenario, 10_playable, 19_broadside (the
  timeline/invariant-wired autopilot scenario runs)
- `examples/ui/` - 09_editor, 11_hud_range, 12_menu_newgame (staged UI
  flows with their own AutopilotPlugin timelines)
- `examples/screenshots/` - 13_screenshot_reel .. 18_screenshot_orbit +
  21_render_scale_shot, plus `data/reel.content.ron` moving WITH its only
  consumer (13 embeds it via `include_str!("data/...")` - file-relative,
  so co-moving keeps the include working unchanged)
- `examples/perf/` - 20_perf_baseline (the `--fps`-wired scene; future
  frame-time scenes land here)

KEY DECISION (recommended; challenge it in review): example NAMES stay
exactly as today - only paths move. Every downstream surface addresses
examples by NAME, not path: the smoke list, `probe run 08_scenario`,
gen-web-screenshots.py, the wiki, AGENTS.md, and all task history. Renaming
churns every one of them for zero testing gain, and per-category
renumbering would destroy the curriculum reading order the numbers encode.

MECHANICS: cargo auto-discovery only sees `examples/*.rs` and
`examples/*/main.rs` - category subdirs REQUIRE explicit `[[example]]`
name+path blocks in the root Cargo.toml. That catalog becomes a feature,
not a cost: set `autoexamples = false` so the 21 blocks (grouped under one
comment header per category) are the SINGLE source of truth and a stray
file can never become a phantom target.

## Steps

- [ ] `git mv` the 21 examples (+ 04's module dir, + `data/`) into the five
      category dirs; add `autoexamples = false` and the 21 `[[example]]`
      blocks grouped by category; `cargo check --examples --features debug`
      proves every path resolves. With auto-discovery off, a missing block
      means an example silently vanishes from the build - the drift pin
      below is the count guard, not eyeballs.
- [ ] examples_smoke.rs: split HARNESSED_EXAMPLES into per-category consts
      (SECTIONS, GAMEPLAY, UI, SCREENSHOTS) with one `#[test]` per category
      sharing the existing runner fn - same assertions, sequential within a
      category. `cargo test --test examples_smoke sections` (test-name
      filter) then runs exactly one category; the CI job is unchanged (it
      runs the whole file).
- [ ] Drift pin (display-free `#[test]`, runs on bare `cargo test`): walk
      `examples/` from CARGO_MANIFEST_DIR, collect every example target on
      disk, assert (a) each has a `[[example]]` block in Cargo.toml whose
      path points at it, (b) every example in a smoke-covered category dir
      appears in exactly ONE smoke const, (c) total count matches disk. A
      new example dropped into a dir without joining the catalog or its
      smoke list fails THIS test - no display, no Xvfb.
- [ ] required-features, decided honestly per example: declare
      `required-features = ["debug"]` ONLY where the example actually fails
      to build/run without the feature (check what the harness imports
      need); if they run fine without it, declare nothing - a gate that is
      not real is dishonest. Record the decision here either way.
- [ ] Reference sweep (paths changed, names did not): literal
      `examples/NN` paths in the wiki (architecture.md:117,
      scenario-system.md:26, guide-author-scenario.md:965,
      modding-ron.md:194), root Cargo.toml comments (perf/reel notes),
      crate doc comments (nova_core lib.rs, nova_debug harness.rs,
      nova_scenario asteroid.rs, src/main.rs), probe SKILL.md (wired table
      gains the category grouping), AGENTS.md (curriculum bullet mentions
      the category layout), gen-web-screenshots.py comments (its `cargo
      run --example` commands address by name - unchanged), CHANGELOG
      Unreleased. Re-grep `examples/[0-9]` afterwards: only task history
      and this file remain.
- [ ] E2E: `probe run 08_scenario` from the new layout (probe addresses by
      name; one real run proves the whole harness surface survived the
      move), plus the cheapest category under Xvfb
      (`xvfb-run cargo test --test examples_smoke -- ui`) if a display is
      available locally - else CI owns the full smoke and this step says
      so. The drift pin runs locally regardless.
- [ ] Verify: fmt; `cargo check --examples --features debug`; newly
      written tests (drift pin); CI runs the full per-category smoke.

## Notes

- Trigger: user request 2026-07-19 - group examples so category-scoped
  testing is easy (section correctness vs FPS-traced perf runs).
- Ordering: land BEFORE the docs strand (audit 20260718-152214 p60, README
  20260718-152205 p65) so those document the final layout ONCE - that is
  why this sits above the audit in priority.
- Category names are adjustable at implementation start if the user
  prefers others (e.g. `autopilot/` for `gameplay/`); the 7/3/3/7/1 split
  is the substance.
- 20_perf_baseline is NOT in the smoke list today (not harnessed) - perf/
  gets no smoke test; probe owns that category (`probe run 20_perf_baseline
  --fps`). Not a regression; recorded so the absence reads as a decision.
- NON-GOALS: renaming or renumbering examples (see key decision); new
  examples; probe wiring changes (the wired table only re-groups).
