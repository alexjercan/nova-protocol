# Nicer ship looks, starting with the six start hulls

- STATUS: OPEN
- PRIORITY: 65
- TAGS: v0.19.0, ships, art

Planned 2026-11-16 from the owner review of the game-feel research doc
(claude.ai artifact Hq6GmMBXm2zAfVFwGnQg45).

## User facts
- "ships look a bit meh and could be done nicer".
- The start ships need work on how they look.

## Agent findings
- Start hulls are hand-placed cube cells in
  `crates/nova_authoring/src/base_content/ships/block.rs:136-491` (ids at
  `ships/mod.rs:34-47`). The mining tug is a 3x1x7 block (`block.rs:139`).
- Hulls wear a derived skin from `styles.rs`, with greebles from
  `scripts/greeble-recipes/` through `scripts/gen-greebles.py`
  (`assets/base/base.bundle.ron:97-98`).
- Start hulls may use advancement-0 parts only (`block.rs:9-11`).
- Owner rule on record: content and config languages are solver rules, not
  fiction vocabulary; greeble placement candidates A/B/C were rejected
  2026-10-14.

## Decisions (owner said continue 2026-11-19; agent calls, stated; map `MAP.md`, frames `before/`)
- Cause, from the baseline: the build parts are cubes only, so a start
  hull of 5 to 10 cells reads as a lump. The hulls are short and bluff
  (interceptor 5 x 5 in plan), four of six share a style, and the drives
  sit in one row. `line_warship` and the generated hulls look better
  because they are long, mirrored and taper from a one-cell nose.
- Rules for all six:
  - Same section counts by kind as now, so mass, HP, hold, worth and
    acceleration do not move on paper. At most two light hull cells more or
    fewer where a shape needs it, reported. The census is rerun at the end.
  - Length at least 2.5 times the body width. A stepped nose down to one
    cell. No bluff two-deck bow wall.
  - Mirrored about the centreline, except one stated offset on the raider.
  - Drives in mirrored groups, not one row; no cell hides a bell from the
    stern quarter.
  - Runs of three or more cells along the length, so the skin lays long
    plates.
  - One signature shape per hull, readable at thumbnail size.
  - One style per start hull with its own hue (now: two pairs share).
- Signature per hull:

  | Hull | Shape | Hue |
  |---|---|---|
  | mining_tug | pusher: thin beam boom ahead of a wide low ore body, cab aft, drives in a triangle | ochre |
  | salvage_hauler | forked bow of two prongs, flat work deck, tall bridge block aft over the drives | rust |
  | light_freighter | narrow cab neck, wide box hold, two drive pods outboard aft | off-white |
  | interceptor | dart: one low deck, narrow nose that widens to the stern, short dorsal block | dark blue-grey |
  | raider | hammerhead: wide head forward with the turrets, narrow tail, one outrigger drive pod | dark red-brown |
  | patrol_picket | keeps its widening plan; low bow, bridge tower aft of midships, short strakes | slate |

- Not in this task: new section shapes (slope, wedge, thin part), new
  section art for the bell, collar, turret and intake, paint, stripes,
  lights. The map lists them as not possible with the current vocabulary.
- The after set uses the same `--looks` rig as `before/`.

## Delivery
1. Rendered baseline: one frame per start hull, same rig and light.
2. Redesign silhouettes and section layout per hull; improve skin and
   greeble use. Keep role balance from task 20261116-132143.
3. Rendered after frames for owner review.

## Verification
- Before/after frames of all six start hulls, inspected, not only built.

## Done when
- The owner signs off the six start hulls from rendered frames.

## Delivery record (2026-11-20; task stays OPEN for the owner sign-off)
- Frames: `before/` and `after/` (24 each, same rig), `after/sheet.png`,
  `after/sheet-top.png`, `compare.png` (before and after, bow and top).
- Hand viewing: `start_hull_showroom` (keys 1 to 6, N, free-fly camera):
  `nix develop --command cargo run --example start_hull_showroom --features debug`.
- Landed:
  - `screenshot_base_fleet -- --looks`: fixed light, flat backdrop, four
    views per hull, camera solved from the hull cells.
  - Six new layouts in `ships/block.rs`. Section counts by kind are the
    same on every hull; no cell was added or removed. The start role
    census is identical to `tasks/20261116-132143/CENSUS-AFTER.md`.
  - Six styles for six hulls. New: `scrap` (raider), `patrol` (picket);
    both list existing fixtures, no new model. Palettes pushed on
    `industrial` (ochre), `salvage` (rust), `armoured` (dark blue-grey);
    hulls and stations that share them follow. Industrial weights raised
    for the stack, crane, winch and floodlight.
  - Final plans, cells W x H x L: tug 4 x 2 x 10 (boom, cab aft), hauler
    6 x 4 x 11 (forked bow), freighter 5 x 3 x 9 (nose, neck two decks
    high, box hold, outboard drives), interceptor 7 x 2 x 8 (arrowhead,
    wing-tip mounts), raider 6 x 3 x 8 (hammerhead, short spine, one
    outrigger drive), picket 5 x 5 x 9 (tower aft, controller in the deck
    under it).
  - Mod-visible id changes (changelog, breaking): civilian drive ids,
    raider `pdc_port`, `pdc_starboard`, `drive_port_dorsal`; `plate_N`
    names new cells on all six.
- Rules not met (agent calls): on overall width, length 2.5 times the
  width holds for the tug only (hauler 1.8, freighter 1.8, picket 1.8,
  raider 1.3, interceptor 1.1). The
  freighter cab step does not show in profile. The interceptor is a
  stubby arrowhead with stepped edges.
- Review (independent, read-only): 0 high, 5 medium, 9 low. Fixed:
  - One-cell joins. Worst single kill, sections cut off from the
    controller (before the reshape / first reshape / now): tug 3 / 5 / 5,
    hauler 1 / 5 / 5, freighter 5 / 44 / 1, interceptor 3 / 2 / 2,
    raider 1 / 12 with every drive / 4, picket 3 / 30 / 3.
  - Lesson and loop cameras on the old mounts (`lesson_combat_battery`,
    `lesson_combat_lance`, `loop_spine_cut`), the mount pick in
    `lesson_combat_moves`, the range origin in `lesson_combat_reach`
    (it panicked), the showroom panic with `--ship`, the Transport role
    text, old shape comments and style counts.
- Turn rate moved (the hull is structure-bound; a longer arm turns
  slower). Sustained deg/s, v0.18.0 to now: tug 64 to 63, hauler 71 to
  59, freighter 75 to 67, interceptor 86 to 76, raider 82 to 73, picket
  78 to 76. The interceptor ceiling is 1.776 rad/s^2 against 1.766 for
  the picket. In the changelog.
- Verified: content gen and lint clean; `base_content` unit tests,
  greeble, parity, lint gate and balance gate tests; census; workspace
  check with `debug`; `npm test`; live under Xvfb and lavapipe with a
  sandboxed profile: probes `system_start_roles`, `system_ai_combat`,
  `system_encounters`, `system_station_buy`, `scenario menu_duel`
  (correctness pass); `start_hull_showroom` (also `--ship raider`), the
  four lessons and two loops above, `screenshot_railgun`; frames read.
- Not verified: hand play; any hardware render; a live kill for the join
  numbers (script model of the sever rule); the full `menu_duel` probe
  verdict; `loop_damage_sequence` after its last text edit;
  `screenshot_editor` at 1024x768 (stalls at a thruster step, not tested
  on the commit before this work); stations and wrecks in the new
  colours; the gauntlet course with the slower freighter; workspace tests
  and clippy.
- Open:
  - Looks: tug boom faces are bare (no line piece seats there); hauler
    rust pieces share the hull hue; the raider reads mid red, not dark;
    every hull is still chamfered lumps (cube-only parts).
  - Thin margins: `lesson_combat_reach` passes by 6 m of 120 m;
    `MIN_RING_SPREAD` clears by one section; one barrel-discipline cell
    fires across the hull behind the dorsal block.
  - Controller cover moved: the interceptor and freighter bridges are
    open on four faces.
  - Lesson texts in `lessons.rs` still say "dorsal mounts"; they match
    the shipped media until the re-shoot.
- Pictures to recapture on hardware (none replaced; all show old hulls or
  colours): `greeble-catalog*.png`; `wiki-sections.png`;
  `wiki-ships-damage.png`, `landing-damage-sequence.webm`,
  `spine-cut.webm`; `wiki-section-railgun*.png`,
  `wiki-combat-railgun.png`, `catalog-siege-railgun-lance-section.png`;
  `wiki-section-mining-beam*.png`, `mining-beam-*.png`;
  `wiki-section-cargo-intake.png`, `mine-sell-*.png`; loops
  `loop-section-mining-beam`, `loop-section-cargo-intake`,
  `loop-section-cargo-jettison`, the docked helm loop;
  `tutorial-radar-lock.png`, `wiki-radar.png`; `wiki-combat.png`,
  `wiki-combat-aftermath.png`, `wiki-hud.png`,
  `wiki-combat-torpedo.png`; `tutorial-menu.png`, `wiki-settings.png`,
  `wiki-controls.png`; `wiki-section-docking.png`; `wiki-hud-shell.png`;
  `wiki-sandbox-range.png`, `feature-editor.png`; `wiki-flight.png`;
  the combat lesson media under `assets/base/training`. Frozen news
  pictures stay.
- Later (larger than this task): new part shapes (slope, wedge, thin
  part), new section art, a trim colour, lights.
- Owner calls wanted with the sign-off: the turn rate cost of the longer
  hulls; the rust hauler.
