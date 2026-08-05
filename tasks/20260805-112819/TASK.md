# Rebuild screenshot_combat as a two-faction fight (Rock hollow), absorbing screenshot_juice

- PRIORITY: 71
- TAGS: v0.10.0,screenshot,examples
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- PARENT: 20260805-105154
- DEPENDS ON: 20260805-112749

## Context

The biggest scene of the refresh (`20260805-105154`): 10 of the 27 images. Today
`screenshot_combat` is the player ship plus one target dead ahead on an empty
range, and `screenshot_juice` is a second example that poses a free-fly camera
on a target and blows one section off. Both become one scene, "Rock hollow":
two Kenney flights split by allegiance fighting inside a dense near asteroid
field, with a player ship off to one side for the lock/HUD beats.

The faction fight is INFERRED, never run. `Allegiance` is
`Player | Enemy | Neutral` (`crates/nova_gameplay/src/relations.rs:26`),
`SpaceshipConfig::allegiance` is an explicit override over the controller
default (`crates/nova_scenario/src/objects/spaceship.rs:234`), and AI target
acquisition runs off the relation model, not the player marker
(`crates/nova_gameplay/src/input/ai/acquisition.rs`). So an AI flight carrying
`allegiance: Some(Player)` SHOULD fight a default-Enemy flight with no player
involved. Prove that before building the set out - if it does not hold, the
scene design changes.

Depends on the photo kit from `20260805-112749`.

## Steps

- [ ] Prove the faction fight first, throwaway: two small AI flights, one with
      `allegiance: Some(Player)`, in an otherwise empty scene. Watch whether
      they acquire, raise weapons and shoot each other. If they do not, stop and
      bring the finding back - the scene design depends on it.
- [ ] Build `data/combat.content.ron`: a dense near asteroid field, two Kenney
      flights (gunship hulls from the shipped menu backdrops) split by
      allegiance with `engage_delay` so they ARRIVE and then engage, and a
      player ship positioned for the lock and HUD beats.
- [ ] Fold `screenshot_juice` in: keep its scripted section blow as a beat of
      this example (a live brawl gives the shatter but destroys the fixed close
      pose the shot needs), then delete
      `examples/screenshots/screenshot_juice.rs`, its `[[example]]` block
      (`Cargo.toml:156-158`) and its `SCREENSHOTS` entry
      (`tests/examples_smoke.rs:76`).
- [ ] Frame the ten beats: `feature-combat`, `feature-hud`, `wiki-combat`,
      `wiki-hud`, `tutorial-combat-lock`, `tutorial-radar-lock`, `wiki-radar`,
      `feature-juice`, `news-090-combat-readability`,
      `news-090-contextual-hud`. The four wiki names are their OWN framings now,
      not aliases.
- [ ] Drop the four entries from `ALIASES` in `scripts/gen-web-screenshots.py`
      (lines 113-118) and give the two `news-090-*` shots FIGURES slots naming
      `screenshot_combat`; move `feature-juice` from `screenshot_juice` to
      `screenshot_combat` (line 88).
- [ ] Update the `screenshots/` roster in `web/src/wiki/dev/development.md`
      (lines 195-196) and the capture commands.
- [ ] Hand it to the owner: run plainly, watch the fight, verdict.

## Definition of Done

- Two AI flights on opposing allegiances actually engage each other with no
  player involved. (manual: run the scene and watch them acquire and fire)
- The example builds and the catalog agrees with disk.
  (cmd: `nix develop --command cargo check --examples --features debug`)
- `screenshot_juice` is gone from disk, `Cargo.toml` and the smoke lists.
  (test: `catalog_matches_disk`)
- The scene reaches `Playing` headless without a panic.
  (test: `screenshots_reach_playing_without_panic`)
- The report lists no aliases and names `screenshot_combat` for all ten shots.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- The owner watches the fight and accepts the look as good enough to shoot.
  (manual: `cargo run --example screenshot_combat --features debug`, no NOVA_REEL)

## Notes

- No PNG is captured or committed in this task.
- Bloom (`Bloom::NATURAL`) and `TonyMcMapface` are already on every scenario
  camera, so muzzle flashes, plumes and explosions glow for free once the frame
  has something bright in it.
- If the faction fight does not work, the fallback is a player-allegiance ship
  the AI already fights plus a scripted second attacker - but bring the finding
  back before choosing it.
