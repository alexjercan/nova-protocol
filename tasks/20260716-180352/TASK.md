# More menu backdrops: two simple cute ambience scenes for the rotation

- STATUS: OPEN
- PRIORITY: 59
- TAGS: v0.7.0, content, scenario

## Goal

The menu_backdrop rotation (task 20260716-155849) has one member. Add two
more base backdrops - simple scenes in the menu_ambience spirit (an AI
ship doing something around a planetoid), but each with its own cozy,
Factorio-cute character, so menu entries feel varied. User request
2026-07-16.

## Steps

- [ ] Read the menu_ambience builder (crates/nova_assets/src/scenario.rs)
      for the required contract: hidden: true, menu_backdrop: true, a
      gravity well with entity id "menu_planetoid" (the camera framing
      anchor), an AI ship flying a real behavior, seeded ScatterObjects
      (no runtime RNG).
- [ ] Design two SIMPLE variants with distinct character, e.g. a busy
      little freight loop (a hauler shuttling between two beacons near
      the planetoid - the "factory at work" vibe) and a quiet salvage
      yard (drifting wrecks/rocks with a lazy orbiter and a warm-colored
      beacon). Reuse existing object kinds only (asteroid, beacon, ship,
      well); no new engine features.
- [ ] Add the two builders in crates/nova_assets/src/scenario.rs (or a
      sibling module), each with menu_planetoid + the flags; wire them
      into scenario_generation::build_scenarios and the base bundle
      content list.
- [ ] Regenerate RON: cargo run -p nova_assets --bin gen_content; the
      parity test's bundle-set guard forces the bundle list to match.
- [ ] Verify: check --all-targets, fmt, content_ron_parity, the
      demo_scenario built-in list (add the new ids), and eyeball at
      least one backdrop in the real app if a display is available
      (render-output-eyeball lesson); otherwise say so honestly.
- [ ] CHANGELOG Unreleased line (menu gets varied backdrops).

## Notes

- Backdrops are hidden from the picker, so no thumbnail needed.
- The rotation test (nova_menu) is fixture-based and unaffected by real
  content counts.
- Depends on: 20260716-155849 (landed 4313cf96).
