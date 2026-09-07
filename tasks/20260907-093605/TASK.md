# The site shows mainline ships: re-capture the web media and drop mod content from the pages

- STATUS: CLOSED
- PRIORITY: 42
- TAGS: v0.13.0, docs, web, art

The site still shows the Kenney fleet. `138dfbfc` (2026-09-04) moved the
modelled Racer/CargoA/CargoB craft into The Ledger and left base block-only,
but every living still and loop under `web/src/assets/` was captured before
that: 2026-08-13, 08-22, 08-31 and 09-01. The wiki's own sections hero is a
CargoA, four combat figures still carry a `v0.10.0` HUD stamp, and the wiki's
flagship widgets compute mainline flight from a mod's geometry.

Owner decision, 2026-09-07: **the web pages are mainline only**. No mod
content illustrates a base page, in a figure, a table or a widget.

News is out of scope and stays frozen. `frozen()` in
`scripts/gen-web-screenshots.py`, `is_frozen()` in
`scripts/capture-web-media.sh` and `web/tests/assets.test.js` already hold
that line in both directions; nothing here relaxes them.

## Evidence

Frames pulled from the shipped assets and read:

| Asset | What it shows |
|---|---|
| `wiki-sections.png` | Kenney CargoA, the sections-chapter hero |
| `wiki-hud.png`, `wiki-combat.png`, `wiki-radar.png`, `wiki-flight.png` | Kenney racer, HUD stamped v0.10.0 |
| `wiki-combat-torpedo.png`, `wiki-combat-aftermath.png` | Kenney CargoB, v0.10.0 |
| `tutorial-radar-lock.png`, `tutorial-combat-lock.png`, `tutorial-orbit.png` | Kenney craft, v0.10.0 |
| `feature-combat.png`, `feature-autopilot.png` | Kenney craft, v0.10.0 |
| `landing-player-flight.webm` (+ alias `loop-section-thruster`) | Kenney racer |
| `landing-damage-sequence.webm`, `landing-cockpit.webm` (+ alias `nova-os-open`) | Kenney craft |
| `torpedo-blast.webm`, `spine-cut.webm` | Kenney craft |
| `wiki-scenarios-picker.png`, `wiki-first-scenario-picker.png` | the retired First Shift chapter list |
| `parts-viewer-racer-exploded.png` | Ledger part geometry, on the news index card |
| `hero-wfc-duel.webm`, `landing-wfc-2v2.webm` | correct block ships, but tiny and near-black |
| `loop-section-controller.webm` | a blown-out white blob, no readable ship |
| `loop-section-torpedo-bay.webm` | two trails on black: no bay, no ship |
| `goto-arrival.webm`, `wiki-gravity.png` | subject a few pixels wide on black |
| `wiki-section-*.png`, `catalog-*-section.png`, `loop-section-hull.webm`, `vfx-range.webm`, `loop-section-turret.webm` | already section style, keep |

The producers are already migrated: `shared/drydock.rs`, `shared/hollow.rs`
and `shared/kit.rs` resolve `block_gunship`, `block_raider`, `block_cutter`
and friends, and `cargo check --examples --features debug` passes at HEAD.
Re-capturing is therefore necessary but not sufficient - the weak framing
above is authored in the examples.

Two gaps are newly closable:

- `block_warship` carries two `siege_railgun_lance_section`s
  (`assets/base/ships/base.content.ron:38700`), so the PENDING reason in
  `scripts/capture-web-media.sh` ("no capture example flies a hull carrying
  a lance") is stale. The four railgun figures, `loop-section-railgun.webm`
  and the two lance catalog cards are capturable now.
- `greeble-catalog.png` has a producer (`greeble_catalog`) that has never
  been run for the site, though greebles are half of the new look.

## Plan

### 1. Producers

- Reframe the weak captures where they are authored: `loop_goto_arrival`,
  `screenshot_flip_burn` (exposure), `system_torpedo_launch` (bay and hull
  in frame), `screenshot_orbit`, `screenshot_gravity`,
  `screenshot_torpedo_run`, and the two `wfc_arena` compositions.
- Add the railgun captures off `block_warship`:
  `wiki-section-railgun.png`, `wiki-section-railgun-sight.png`,
  `wiki-section-railgun-corridor.png`, `wiki-combat-railgun.png`,
  `catalog-railgun-lance-section.png`,
  `catalog-siege-railgun-lance-section.png`, `loop-section-railgun.webm`.
- Add `wiki-ships-damage.png`.
- Show what mainline gained: WFC generate in `screenshot_editor`, the
  greeble atlas from `greeble_catalog`, the current scenario picker and the
  tutorial.
- Update the PENDING rows and the `example=None` slots in both packagers.

### 2. Capture

All still producers and loop compositions, Xvfb plus lavapipe, one at a
time. News frozen (no `NOVA_UNFREEZE`). Package with
`scripts/gen-web-screenshots.py` and `scripts/capture-web-media.sh`.

Precondition: a quiet host. A live agent-bench or game session on the box
contaminates the run.

### 3. Pages, mainline only

- Drop the eleven `catalog-racer-*`, `catalog-cargoa-*` and
  `catalog-cargob-*` rows from the hull, thruster, controller and
  torpedo-bay catalogs, and the "The craft rows are The Ledger's" captions
  under them.
- Re-base the widgets on base hulls: `goto-verb`, `controller-arm`,
  `controller-margin`, `thruster-mass`, `hull-armour` in
  `web/src/widgets.ts` currently run on `RACER_PARTS`, `CARGOA_PARTS` and
  `CARGOB_PARTS`. Move them to block hulls, and carry the static fallback
  prose and the `file:line` verification comments with them.
- Prose sweep for mod craft on base pages: `wiki/sections.md:41`,
  `wiki/sections/hull.md:44`, the corvette references on
  `wiki/flight-autopilot.md`, `wiki/sections/controller.md`,
  `wiki/sections/thruster.md`, `wiki/sections/railgun.md` and
  `wiki/sections/torpedo-bay.md`.
- Refresh the tutorial page and the scenario pages against what ships now
  (First Shift retired, `tutorial` "Basic Training" in base).
- Refresh placeholder notes and alt text that describe a modelled craft.

### 4. Verify

- `python3 scripts/gen-web-screenshots.py --report`
- `cd web && npm run ci`
- Read the rendered pages, not only the diff.
- One `CHANGELOG.md` entry under Web & Platform.

## Out of scope

`web/src/news/*`, every `news-*` and `thumb-news-*` asset, and the
comic/story art.

## What landed

### Producers

- `shared/kit.rs` grew `catalog_ship` (the shipped hull WHOLE - sections plus
  the derived skin, its style, its collapse threshold and sound) and `clad`
  (a hand-built cell list wearing the same derived skin). `catalog_hull` went
  private: structure alone is not a ship a scene should spawn. Every scene
  producer now spawns clad catalog hulls.
- Catalog cards, section closeups and section loops stay BARE on purpose:
  cladding fills the EMPTY cells around a hull, so a clad documentation rig
  photographs the plate instead of the part. Cladding the railgun rigs hid the
  lance behind a smooth plate; both reverted, with the reason on the call.
- `shared/ring.rs` gained `ring::leg` - install a leg camera on any bearing
  and CUT to it. `chase` and `lead` are now two named cases of it.
- `loop_goto_arrival`'s montage was ten PINNED poses. A ship braking from a
  transfer crosses 200 m in the 0.65 s a cut holds, so it left every frame it
  was placed in: eleven seconds of empty starfield. The ten bearings are leg
  cameras now and the hull is in frame throughout.
- `loop_damage_sequence`'s still framed the drive bell (a world-space bearing
  against a tumbling hull). It is measured off the live hull rotation now and
  looks at the port flank, which is the only flank any beat touches.
- `screenshot_menu` pins the backdrop to `menu_gauntlet` and anchors the
  camera to its gunship (`ScriptedCameraAnchor`, hull frame) 170 m off the
  bow. The unpinned menu drew one of four backdrops at random, and the
  authored backdrop pose is a 2.6 km establishing shot.
- `screenshot_railgun` is new: the bore, the sight, the corridor, the combat
  shot and the siege lance's catalog card, off `block_warship`.
- Two latent breaks, both the same shape: `stress_point_defense` and
  `screenshot_damage_levels` wrote `news-` loop names directly while the
  capture script expected the living names, so `loop-section-turret` and
  `loop-section-hull` could never be re-cut. Both now write the living name
  and the script aliases the posts off it. `system_torpedo_launch` had the
  third instance and now records the bay's door cycle as its own loop.

### Capture

Both packagers and the scratch still-runner now pin `NOVA_MODDING_CACHE_ROOT`
and `NOVA_CONFIG_ROOT` at empty directories. A developer box with The Ledger
or Gauntlet installed merges those mods at load and reskins the fleet the site
is a picture of; the first capture round was shot that way and re-shot clean.

58 stills, 6 icons and 34 loops packaged. News frozen throughout (no
`NOVA_UNFREEZE`): every `news-` row in both manifests reads `frozen`.

### Pages

- `wiki/scenarios.md` now describes a base install (Basic Training alone,
  backdrops hidden, no campaign shipped) and frames campaign grouping as what
  an installed mod adds.
- Figure notes rewritten against the frames that exist: the menu backdrop, the
  ships chapter's damage still, the GOTO burn, the torpedo bay's iris, the
  firefight, the scenario picker and the three railgun figures.
- `create/styles.md` gained the greeble catalog figure - the producer existed
  and shipped a still nothing referenced.

### Verified

- `python3 scripts/gen-web-screenshots.py --report`: every referenced image is
  shipped and correctly shaped, 0 outstanding.
- `cd web && npm run ci`: green, news namespace held across 74 source files.
- Rendered pages read for the figures and prose above.
- `CHANGELOG.md` [Unreleased] > Web & Platform: two entries.

Not done: the scenario picker thumbnails are still generated placeholder art
(advisory, 8 of them), and the landing page's four `feature-*.png` stills are
shipped but no longer referenced by any page.
