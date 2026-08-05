# Rebuild screenshot_orbit as screenshot_flight (The ring): autopilot, wiki-flight, tutorial-orbit

- PRIORITY: 70
- TAGS: v0.10.0,screenshot,examples
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- PARENT: 20260805-105154
- DEPENDS ON: 20260805-112749

## Context

The flight/nav scene of the refresh (`20260805-105154`): `feature-autopilot`,
`wiki-flight`, `tutorial-orbit`. Today `feature-autopilot` and `wiki-flight`
come out of the combat range (the latter as an ALIAS of the former) and
`tutorial-orbit` is its own example, `screenshot_orbit` - a planetoid, a player
ship at `ORBIT_RADIUS` 45, and nothing else in frame.

They belong together: all three want a player ship, the HUD on, and a maneuver
running - and none of them wants a fight. `screenshot_orbit` becomes
`screenshot_flight` and takes all three.

Depends on the photo kit from `20260805-112749`.

## Steps

- [ ] Rename the example: `examples/screenshots/screenshot_orbit.rs` ->
      `screenshot_flight.rs`, its `[[example]]` block (`Cargo.toml:160-162`) and
      its `SCREENSHOTS` entry (`tests/examples_smoke.rs`).
- [ ] Build `data/flight.content.ron` on the kit: the gravity planetoid, a
      player Kenney racer, and rocks scattered along the ring so the orbit reads
      as motion instead of a ship on black.
- [ ] Keep the live ORBIT maneuver (the HUD ring + radius spoke is the point of
      `tutorial-orbit`), and add beats for `feature-autopilot` and
      `wiki-flight` framed distinctly - `wiki-flight` is its own shot now, not
      an alias.
- [ ] Move the three names to `screenshot_flight` in
      `scripts/gen-web-screenshots.py` FIGURES, and drop `wiki-flight.png` from
      `ALIASES`.
- [ ] Update the `screenshots/` roster and capture commands in
      `web/src/wiki/dev/development.md`.
- [ ] Hand it to the owner: run plainly, watch the orbit settle, verdict.

## Definition of Done

- The example builds and the catalog agrees with disk.
  (cmd: `nix develop --command cargo check --examples --features debug`)
- `screenshot_orbit` is gone from disk, `Cargo.toml` and the smoke lists.
  (test: `catalog_matches_disk`)
- The scene reaches `Playing` headless without a panic.
  (test: `screenshots_reach_playing_without_panic`)
- The report names `screenshot_flight` for all three shots and no longer aliases
  `wiki-flight`.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- The owner watches the ship settle onto the ring and accepts the look.
  (manual: `cargo run --example screenshot_flight --features debug`, no NOVA_REEL)

## Notes

- No PNG is captured or committed in this task.
- `tutorial-orbit` needs the maneuver HUD up (ring + radius spoke), so the HUD
  stays ON for these beats - unlike the pure-3D scene shots.
- Today's settle is 5.6s onto a radius-45 ring; re-tune it if the new ring
  radius or ship mass changes.
