# Reorganize examples/ by purpose (sections, gameplay, ui, screenshots, perf): category dirs + per-category smoke tests

- STATUS: OPEN
- PRIORITY: 62
- TAGS: v0.8.0,refactor,testing,examples

## Goal

Group the 21 flat examples by PURPOSE so testing by category is one command:
section correctness, full-gameplay autopilot runs, UI flows, screenshot
producers, FPS-traced perf scenes. Bevy-repo style: category directories,
plain slug names, NO number prefixes (user decision 2026-07-19). The
curriculum reading order the numbers used to encode moves into the docs
(see the curriculum step below) instead of living in filenames.

Layout and renames (file stem == example name, so `cargo run --example
<slug>`; names stay unique package-wide since cargo's example namespace is
flat):

- `examples/sections/` - controller_section, thruster_section,
  hull_section, turret_section (its sibling `turret_section/slider.rs`
  module dir renames with it - `mod slider;` resolves relative to the
  file, so the pattern survives), torpedo_section, torpedo_guidance,
  com_range (was 01-07)
- `examples/gameplay/` - scenario, playable, broadside (was 08, 10, 19;
  the timeline/invariant-wired autopilot scenario runs)
- `examples/ui/` - editor, hud_range, menu_newgame (was 09, 11, 12;
  staged UI flows with their own AutopilotPlugin timelines)
- `examples/screenshots/` - screenshot_reel, screenshot_ui,
  screenshot_combat, screenshot_sections, screenshot_juice,
  screenshot_orbit, render_scale_shot (was 13-18, 21; keep the
  `screenshot_` stems - `cargo run --example ui` colliding with the ui/
  category reads terribly), plus `data/reel.content.ron` moving WITH its
  only consumer (embedded via `include_str!("data/...")` - file-relative,
  so co-moving keeps the include working unchanged)
- `examples/perf/` - perf_baseline (was 20; the `--fps`-wired scene;
  future frame-time scenes land here)

RENAME CONSEQUENCE, priced in deliberately: every downstream surface
addresses examples by NAME, so this is a rename-sweep task, not just a
file move - smoke lists, probe invocations and its skill's wired table,
gen-web-screenshots.py's SHOTS mapping and commands, AGENTS.md, the wiki,
crate doc comments. The sweep steps below enumerate them; task history
keeps the old names (records are records).

MECHANICS: cargo auto-discovery only sees `examples/*.rs` and
`examples/*/main.rs` - category subdirs REQUIRE explicit `[[example]]`
name+path blocks in the root Cargo.toml. That catalog becomes a feature,
not a cost: set `autoexamples = false` so the 21 blocks (grouped under one
comment header per category, listed in curriculum order) are the SINGLE
source of truth and a stray file can never become a phantom target.

## Steps

- [ ] `git mv` the 21 examples into the five category dirs WITH the slug
      renames above (+ `turret_section/` module dir, + `data/`); add
      `autoexamples = false` and the 21 `[[example]]` blocks grouped by
      category in curriculum order; `cargo check --examples --features
      debug` proves every path resolves. With auto-discovery off, a
      missing block means an example silently vanishes from the build -
      the drift pin below is the count guard, not eyeballs.
- [ ] examples_smoke.rs: split HARNESSED_EXAMPLES into per-category consts
      (SECTIONS, GAMEPLAY, UI, SCREENSHOTS) carrying the NEW names, one
      `#[test]` per category sharing the existing runner fn - same
      assertions, sequential within a category. `cargo test --test
      examples_smoke sections` (test-name filter) then runs exactly one
      category; the CI job is unchanged (it runs the whole file).
- [ ] Drift pin (display-free `#[test]`, runs on bare `cargo test`): walk
      `examples/` from CARGO_MANIFEST_DIR, collect every example target on
      disk, assert (a) each has a `[[example]]` block in Cargo.toml whose
      path points at it, (b) every example in a smoke-covered category dir
      appears in exactly ONE smoke const, (c) total count matches disk. A
      new example dropped into a dir without joining the catalog or its
      smoke list fails THIS test - no display, no Xvfb.
- [ ] Curriculum order relocation: the numbers are gone, so the reading
      order must live SOMEWHERE explicit - rewrite AGENTS.md's "examples
      01-19 the curriculum" bullet as a short ordered list (or point it at
      the Cargo.toml catalog, which the first step keeps in curriculum
      order); the wiki dev page that introduces examples gets the same
      treatment. Losing the order silently is the failure mode this step
      exists to prevent.
- [ ] required-features, decided honestly per example: declare
      `required-features = ["debug"]` ONLY where the example actually fails
      to build/run without the feature (check what the harness imports
      need); if they run fine without it, declare nothing - a gate that is
      not real is dishonest. Record the decision here either way.
- [ ] Rename sweep (names AND paths changed): tests/examples_smoke.rs
      consts (previous step); probe SKILL.md wired table (scenario,
      playable, perf_baseline + category grouping); AGENTS.md build-block
      command (`cargo run --example scenario`) and curriculum bullet;
      gen-web-screenshots.py SHOTS mapping + its `cargo run --example`
      commands + comments; wiki literal names/paths (architecture.md:117,
      scenario-system.md:26, guide-author-scenario.md:965,
      modding-ron.md:194 - re-grep, lines drift); root Cargo.toml comments
      (perf/reel notes); crate doc comments (nova_core lib.rs, nova_debug
      harness.rs, nova_scenario asteroid.rs, src/main.rs); example-file
      doc headers that name themselves (e.g. 21's header + its inline
      NOVA_SHOT_PATH commands); CHANGELOG Unreleased notes the renames.
      Re-grep `[0-9][0-9]_[a-z]` and `examples/[0-9]` afterwards: only
      task history remains.
- [ ] E2E: `probe run scenario` from the new layout (one real run proves
      the whole harness surface survived the move+rename), plus the
      cheapest category under Xvfb (`xvfb-run cargo test --test
      examples_smoke -- ui`) if a display is available locally - else CI
      owns the full smoke and this step says so. The drift pin runs
      locally regardless.
- [ ] Verify: fmt; `cargo check --examples --features debug`; newly
      written tests (drift pin); CI runs the full per-category smoke.

## Notes

- Trigger: user request 2026-07-19 - group examples so category-scoped
  testing is easy (section correctness vs FPS-traced perf runs). Naming
  decision same day: bevy-style plain slugs, no number prefixes ("it's
  how bevy does it in their repo").
- Ordering: land BEFORE the docs strand (audit 20260718-152214 p60, README
  20260718-152205 p65) so those document the final layout ONCE - that is
  why this sits above the audit in priority.
- Category names are adjustable at implementation start if the user
  prefers others (e.g. `autopilot/` for `gameplay/`); the 7/3/3/7/1 split
  is the substance.
- perf_baseline (old 20) is NOT in the smoke list today (not harnessed) -
  perf/ gets no smoke test; probe owns that category (`probe run
  perf_baseline --fps`). Not a regression; recorded so the absence reads
  as a decision.
- Old names in probe-runs/ dirs and task history are historical records -
  do not rewrite them.
- NON-GOALS: new examples; probe wiring changes (the wired table only
  re-groups and renames).
