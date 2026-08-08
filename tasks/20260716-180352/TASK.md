# More menu backdrops: two simple cute ambience scenes for the rotation

- STATUS: CLOSED
- PRIORITY: 59
- TAGS: v0.7.0, content, scenario

## Goal

The menu_backdrop rotation (task 20260716-155849) has one member. Add two
more base backdrops - simple scenes in the menu_ambience spirit (an AI
ship doing something around a planetoid), but each with its own cozy,
Factorio-cute character, so menu entries feel varied. User request
2026-07-16.

## Steps

- [x] Read the menu_ambience builder (crates/nova_assets/src/scenario.rs)
      for the required contract: hidden: true, menu_backdrop: true, a
      gravity well with entity id "menu_planetoid" (the camera framing
      anchor), an AI ship flying a real behavior, seeded ScatterObjects
      (no runtime RNG).
- [x] Design two SIMPLE variants with distinct character, e.g. a busy
      little freight loop (a hauler shuttling between two beacons near
      the planetoid - the "factory at work" vibe) and a quiet salvage
      yard (drifting wrecks/rocks with a lazy orbiter and a warm-colored
      beacon). Reuse existing object kinds only (asteroid, beacon, ship,
      well); no new engine features.
- [x] Add the two builders in crates/nova_assets/src/scenario.rs (or a
      sibling module), each with menu_planetoid + the flags; wire them
      into scenario_generation::build_scenarios and the base bundle
      content list.
- [x] Regenerate RON: cargo run -p nova_assets --bin gen_content; the
      parity test's bundle-set guard forces the bundle list to match.
- [x] Verify: check --all-targets, fmt, content_ron_parity, the
      demo_scenario built-in list (add the new ids), and eyeball at
      least one backdrop in the real app if a display is available
      (render-output-eyeball lesson); otherwise say so honestly.
- [x] CHANGELOG Unreleased line (menu gets varied backdrops).

## Notes

- Backdrops are hidden from the picker, so no thumbnail needed.
- The rotation test (nova_menu) is fixture-based and unaffected by real
  content counts.
- Depends on: 20260716-155849 (landed 4313cf96).

## Close notes (2026-07-16)

What changed: two new builder-backed backdrops in
crates/nova_assets/src/scenario.rs - menu_waystation (two named hauler
orbiters in convoy on opposite phases, three colored dock beacons, a
flat cargo-rock lane) and menu_scrapyard (one tug orbiter, a scatter of
10 on-rails salvage crates, two fixed wreck rocks, one warm YARD
beacon) - plus three shared helpers (backdrop_planetoid /
backdrop_orbiter / backdrop_beacon; menu_ambience deliberately left
untouched so its committed RON stays byte-identical). Wired into
build_scenarios + base.bundle.ron; RON generated via gen_content; the
parity bundle-set guard enforced the wiring. demo_scenario's built-in
list gained both ids. CHANGELOG line added.

Design constraints honored: only proven primitives (orbit-directive
ships - patrol was rejected as unverified near a gravity well; salvage
crates are RigidBody::Static "on rails", zero physics risk); every
static object outside the planetoid's geometric radius (~80-91u) and
below the orbit plane, matching menu_ambience's safety envelope;
distinct scatter seeds; both flagged hidden + menu_backdrop with the
menu_planetoid camera anchor.

Verification: content_ron_parity 2/2, demo_scenario 13/13, check
--all-targets + fmt clean. BEHAVIORAL: 6 real boots of the shipped menu
flow (example 12 under Xvfb :99, autopilot) - the rotation picked all
three backdrops across runs and every cycle completed. VISUAL: example
14 captured tutorial-menu.png over each new backdrop (8 runs);
eyeballed both - waystation reads as a dock with amber/cyan berth
lights, scrapyard reads warm and cozy (crate glows like embers). Full
suite is CI's job per the standing instruction.

Difficulties: a cleanup `pkill -f 'Xvfb :99'` matched the invoking
shell's OWN command line and killed the whole command chain (exit 144);
re-ran without it and left the Xvfb processes alone (one PID may be the
user's real display - never blind-kill by pattern).

Reflection: the screenshot-example infrastructure (13/14) made the
"eyeball it" step cheap enough to actually do - 8 automated captures
beat launching the game by hand. Worth defaulting to for content tasks.
