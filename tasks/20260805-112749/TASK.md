# Photo kit + rebuild screenshot_reel as screenshot_scene (Drydock drift)

- PRIORITY: 72
- TAGS: v0.10.0,screenshot,examples
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- PARENT: 20260805-105154

## Context

First scene of the six-scene refresh (`20260805-105154`; problem, map designs
and rationale in its `NOTES.md` / `DECISION.md`). It lands the shared photo kit
with its first consumer and sets the look every later scene inherits, so it goes
first.

`screenshot_reel` becomes `screenshot_scene`. Today its scene
(`examples/screenshots/data/reel.content.ron`) is a prop ship of engine
primitives at the origin, a planetoid 26 units off, and 28 rocks in a
`Ring(inner: 90, outer: 180)` - far enough out that the frames read as empty
space. It produces `feature-gravity`, `wiki-gravity`, `wiki-sections`.

CONSTRAINT found while planning: `tests/examples_smoke.rs:120`
(`catalog_matches_disk`) treats every `.rs` file DIRECTLY under a category dir
as an example and asserts disk == the `[[example]]` catalog. A shared
`examples/screenshots/kit.rs` would therefore fail the test. It must live one
level down and be pulled in per example with `#[path = ...] mod`, the precedent
being `examples/sections/turret_section.rs:40`.

Round one captures NOTHING. The gate is the owner running the example plainly
(free-fly WASD camera, no `NOVA_REEL`) and accepting the look.

## Steps

- [ ] Add `examples/screenshots/shared/kit.rs`, bounded to three things: a
      three-light photo rig (key + rim + fill, scenario-scoped, spawned by the
      example so the shipped one-light rig is untouched), the Kenney hull
      section lists lifted from `assets/base/scenarios/menu_scrapyard.content.ron`
      (`racer_cube_*` prototypes), and a near-field asteroid dressing helper.
      Nothing else goes in it.
- [ ] Pull it in with `#[path = "shared/kit.rs"] mod kit;` and confirm
      `catalog_matches_disk` still passes - the subdir must stay invisible to
      the disk scan.
- [ ] Rename the example: `examples/screenshots/screenshot_reel.rs` ->
      `screenshot_scene.rs`, its `[[example]]` block in `Cargo.toml:136-138`,
      and its entry in `SCREENSHOTS` (`tests/examples_smoke.rs:72`).
- [ ] Rewrite the scene as `data/scene.content.ron`: planetoid at a distance
      where its surface reads, near-field rocks scattered roughly 15-60 units
      with radius variance, a hero Kenney racer posed in the foreground, and two
      more Kenney hulls drifting mid-field on AI `orbit` so the frame is alive.
- [ ] Re-frame the three beats (`feature-gravity`, `wiki-gravity`,
      `wiki-sections`) against the new set. Beats are updated, not run.
- [ ] Rename the producer in `scripts/gen-web-screenshots.py` FIGURES
      (lines 76-78) and in the docstring's capture commands (line 40).
- [ ] Update `web/src/wiki/dev/development.md` (the `screenshots/` roster at
      line 195 and the capture block at line 407).
- [ ] Hand it to the owner: `nix develop --command cargo run --example
      screenshot_scene --features debug`, free-fly the set, verdict.

## Definition of Done

- The example builds and the catalog agrees with disk.
  (cmd: `nix develop --command cargo check --examples --features debug`)
- `screenshot_scene` is cataloged, smoked, and `screenshot_reel` is gone from
  every list. (test: `catalog_matches_disk`)
- The scene reaches `Playing` headless without a panic.
  (test: `screenshots_reach_playing_without_panic`)
- The coverage report names `screenshot_scene` as the producer for the three
  scene figures.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- The owner flies the scene and accepts the look as good enough to shoot.
  (manual: `cargo run --example screenshot_scene --features debug`, no NOVA_REEL)

## Notes

- No PNG is captured or committed in this task.
- The photo rig is example-side only. Authorable scenario lighting is
  `20260805-111534` and is not a dependency.
- `crates/nova_probe/src/bin/probe/native/fixtures.rs:14` names
  `screenshot_reel` in a STAND-IN catalog for parser tests; it is not checked
  against disk. Rename it for tidiness only if the tests stay green.
- Examples must be RUN, not just `cargo check`ed - a check misses
  duplicate-component panics. Headless needs a display (`Xvfb :99`).
