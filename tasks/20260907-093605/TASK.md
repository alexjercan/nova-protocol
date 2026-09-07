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

## Follow-up round, 2026-09-07

Owner review of the packaged media raised three things: the landing hero's two
ships sit too close together and read as static, the destruction particles look
like large circles, and it was not clear whether those particles are the game
or something added for the capture.

### The particles are the game

They are `crates/nova_gameplay/src/integrity/pyre.rs`, an observer on
`IntegrityDestroyMarker` - the same seam `explode` hangs the wreckage off. It
is new this release: before it, a death shed pieces in silence and the game had
impact effects only. It fires for EVERY death, which is why an intercepted
torpedo throws one: a torpedo's controller and thruster sections have 1 HP and
die through the normal integrity pipeline (`on_torpedo_body_destroyed` in
`bay.rs`), so the round that a PDC swats is a small hull coming apart.

Owner decision: keep it on ordnance too, and refine the look rather than
gate it.

### What the refinement changed

The first cut was ONE billboard burst of round soft dots, and its own doc
comments read world units as metres - a factor of ten. A section death was
9 m quads reaching 126 m, drawn at a detonation core's alpha; from fifty
metres that is a heap of flat orange discs, which is the "bubbles" the review
saw.

It is now the house vacuum-burst shape, the one
`torpedo_section/render.rs` already documents: TWO instances per death.

- a CORE, camera-facing, bright, and gone inside a third of a second, because
  nothing out here sustains combustion;
- an EJECTA, oriented along velocity so its quads read as tapered streaks
  contracting into sparks, ballistic and outliving the flash.

Alpha is held low on both, so overlapping quads sum into a translucent volume
with the wreck showing through instead of stacking into opaque discs. Sizes
were re-derived in world units against the build-grid cell (a section) and the
110 m gunship (a hull).

The shipped torpedo blast core was the other half of the same complaint - flat
orange discs at a torpedo hit, at alpha 0.95 - and its gradient tail was
softened to match.

### Framing

- `wfc_arena` stages the pair at 500 m and leaves them closing at 15 m/s each
  under capture, so the landing hero opens on two separated hulls in motion.
  Empirically 620 m and 550 m never landed a warhead; 500 m does. At 25 m/s
  each they ended the loop nose to nose, hence 15.
- `loop_cockpit` stands OUTBOARD (negative `side`): `ring::lit_side` points
  inboard on this set, so every positive-side framing looked at 3 km of empty
  sky. It also re-frames for the climb, because a leg camera's `up` is world Y
  and the beacon is 7.6 km straight up - the resting bearing collapses into a
  view down the hull's own length once the track swings onto Y.
- `loop_goto_arrival`'s ten cuts stood abeam, which at the top of that leg is
  a grey hull on black. They stand down the track now, which both puts the
  planetoid in frame and looks into the braking plume.
- `loop_damage_sequence`'s lens was 76 m off its own subject. At 115 m the
  whole hull is in frame and the freed deck drifts inside the shot.

### Verified

- 7 new unit tests in `pyre.rs` (it had none): the two instances a death
  spawns, the two sizes, the shared graphs and mask, the transform-less node,
  the frame cap with the hull exempt, the refill, and the inherited drift.
  `cargo test -p nova_gameplay --lib integrity::pyre`: 7 passed.
- `cargo check --examples --features debug` and `cargo fmt`: clean.
- All 22 still producers and all 35 loops re-captured with the new effects and
  re-packaged. Every loop under the 3 MB budget; `wfc_arena` needed its own
  `LoopProfile { crf: 37 }` to get the 2v2 there.
- `gen-web-screenshots.py --report`: 0 outstanding (8 scenario thumbnails
  still advisory).
- `cd web && npm run ci`: green.
- Contact sheets read for every re-cut loop.

Not done: `loop-section-turret` still shoots the battery from the `lanes`
measurement pose, so the mount is small in its own section figure. Giving it a
documentation view means a fourth `View` variant, and the three that exist are
measurement knobs.

## Follow-up round two, 2026-09-07

Three more notes on the shipped media, plus one figure that did not earn its
place.

### The hero duel stands 2 km apart

`STRIKE_STAGE_RANGE` is 2 km now, not 500 m. The reason is the warhead: a
torpedo's blast sphere is 300 m, so a pair inside knife range trade a salvo
and both die in the same frame. At 2 km the salvo is six seconds of flight and
lands on the ship it was aimed at.

Raising the band alone did nothing - `fight_within` is a CEILING, so a 2.1 km
ceiling was already true on the first frame of `close the range` and the next
beat cancelled the orders it had just installed. The gate is now
`fight_staged(range, tolerance)`, a BAND, and the pair actually closes to it.

No lens holds both hulls at that separation: framing a 2 km spread means
standing 1.4 km off, where a 110 m hull is a twenty-fifth of the frame width.
So the capture keeps the over-the-shoulder `Cinema` pose and the rival reads as
a marked contact down the threat axis, which is what it looks like in the game.

### The railgun loop and its stills want different moments

The loop closed 0.13 s after the hit, because that is the last frame at which
`wiki-combat-railgun.png` still reads as a hull with a corridor bored through
it. Half a second later it is a cloud of cells - a good LOOP and a useless
still.

They do not have to be one moment. The stills and the loops are two separate
runs of `screenshot_railgun` (`capture-web-shots.sh` stages into
`target/shots`, `capture-web-media.sh` into `target/loop-shots`), and each
packager keeps only what it came for. `NOVA_RAILGUN_AFTERMATH` is the window,
the loops row sets it to 0.5, and the stills pass takes the default. The loop
is 10.7 s of the hull coming apart; the stills are untouched.

### The turret page shows the mount, not the battery

New producer, `loop_turret_stow` -> `loop-section-turret-stow.webm`: one PDC on
a three-cell bench, alone in the dark, 26 m from the lens. The walk presses
`combat_stance` and releases it; everything after that is the production stow
machine. The set is empty on purpose - a contact would combat-lock the ship,
and a locked ship keeps its weapons hot whatever the stance says, so the mount
would never fold.

The four quiet seconds a stow waits out (`STOW_SETTLE_SECONDS`) are a design
cost, not a picture, so the recording runs them at 4x and drops back to real
speed the frame the fold starts. Both animations play at the speed they were
authored at.

This also answers the note left open above: `loop-section-turret` still shoots
the battery from the `lanes` measurement pose, but the mount now has a close
figure of its own on the same page, so the wide shot is no longer the only one.

### `wiki-combat-torpedo.png` was a picture of nothing

The caption promised a salvo in flight; the frame was a corvette filling the
lens with its attacker eclipsed dead centre BEHIND it, under the fps bar.

- The camera stood on the run-in line. The boat, the torpedo and the raider are
  collinear, so a lens anywhere along that line puts the round behind the hull
  it is aimed at. The offset is derived from the set now - broadside to the
  run, and from below, because the rock field is a horizontal annulus and a
  level lens frames rock soup.
- The framing is re-taken on the pair as it actually stands, off
  `hollow::lead_torpedo_position`: the raider drifts and the torpedo crosses
  most of a kilometre between the first framing and the shot.
- `hollow::hud_cinematic` runs before both ordnance frames. The camera has left
  the player's ship by then, so its fps bar and its two contact chevrons were
  chrome over someone else's fight.

### Verified

- `cargo check --examples --features debug` and `cargo fmt --all`: clean.
- Contact sheets read for `landing-wfc-2v2`, `loop-section-railgun` and
  `loop-section-turret-stow`; both torpedo stills read at full resolution.
- 36 loops packaged, all under the 3 MB budget. `gen-web-screenshots.py`: 59
  copied, 4 pending (unchanged).
- `cd web && npm run ci`: green.

## Follow-up round three, 2026-09-07

### The landing duel had no guns in it

The hero and 2v2 loops were two hulls throwing torpedoes at each other in
silence: the whole exchange, and not one tracer.

The staging range was the cause. Every shipped PDC reaches 200 u and the AI
holds its fire outside 0.9 of that - 180 u, `AI_FIRE_RANGE_FACTOR` - so a pair
parked at the 2 km the warhead wanted stood outside its own mounts' gate and
the batteries never opened. `STRIKE_STAGE_RANGE` comes in to 1.5 km, which is
inside the gate and still five blast radii clear of the 300 m warhead sphere
the range was set against in the first place. What the loops show now is the
answering half of the strike: tracer streams both ways, point defence working
the salvo, and blocks coming off the hulls that take one.

### One lance, on the hull the camera is behind

Every arena hull collapses with a bow gun - that is what the tile set is for -
and the AI pulls its own lance trigger whenever the bore comes on, so both
sides fired one. A spinal lance is not a thing a picture can hold twice: the
slug crosses 1.5 km in a tenth of a second, so the near hull's shot is three
frames and the rival's is three frames of a contact a kilometre and a half
away, arriving in the near hull's bow.

`disarm_the_rival_lances` marks every lance but the subject's
`SectionInactiveMarker` at staging, which both fire paths already read, so the
rival's gun cannot fire rather than merely not being asked. A staging cut like
`settle_the_fight`: it lands before the loop opens and nothing on film changes.

### The bay loop launched twice and stood square to the door

Two faults, one frame.

- The trigger was a HELD input and nothing took it off until the door beat.
  The bay reloaded inside the 0.8 s run-out, launched again, and the door beat
  then cancelled it - an iris seen starting to reopen and thinking better of
  it. `release_the_trigger` now runs at the top of the run-out, so the loop is
  one launch and the iris shuts behind the round that is already away.
- The lens stood 33 degrees off the bore, which is almost down it: the bays'
  20 m of length collapsed into their door face and the housing read as a squat
  box in the top-left corner. It swings out to a three-quarter at ~60 degrees
  and 42 m, and the aim point comes off the muzzle - everything the shot is of
  lies aft of and below the point the ordnance leaves from.

Swinging out has a limit the first cut found: past about 55 degrees, with the
lens still IN FRONT of the door, the ordnance leaves along the bore straight at
the camera and fills the frame from the near plane. The pose that works sits
aft of the door face looking forward along the hull, so the round departs
across the frame and away.

### Verified

- `cargo check --example wfc_arena --example system_torpedo_launch --features
  debug` and `cargo fmt --check`: clean.
- Contact sheets read for `hero-wfc-duel`, `landing-wfc-2v2` and
  `loop-section-torpedo-bay`, plus full-resolution frames of the hero mid-loop
  and its close.
- 36 loops re-packaged, all under the 3 MB budget (`landing-wfc-2v2` is the
  heaviest at 2.9 MB).
- `cd web && npm run ci`: green.

## Re-shoot for shed cladding, 2026-09-07

`3f8163fa8` made a spent skin plate fly instead of vanish, so every living
still and loop was re-recorded against it. The shed plates read plainly: flat
angular panels coming away from the hull and tumbling clear, distinct from the
cubic section blocks a dying section throws.

One thing broke on the way through. `landing-wfc-2v2` encoded 3.37 MB against
the packager's 3 MB budget - the loop already carried four hulls, two salvos
and the point defence answering them, and a hull's worth of flying cladding on
top of that is bitrate VP9 spends. The arena's capture profile splits its two
rosters rather than paying for it across the set: the duel is the landing
page's lead and stays at CRF 37 with room to spare, the 2v2 is a background
band behind text and goes to 41, which lands it at 2.37 MB with no visible
blocking.

`web/src/wiki/ships.md` said a plate "comes off". It now says it comes away
where it stood, tumbling clear on the ship's own motion, greebles and all.

### Verified

- 36 loops packaged, all under budget; `gen-web-screenshots.py`: 59 copied, 4
  pending (unchanged).
- Full-resolution frames read for the duel's close and the 2v2 at CRF 41.
- `cd web && npm run ci`: green.

## Debris pivot, and a real-speed railgun cut, 2026-09-07

Two requests, one capture pass.

### The pivot

Shed cladding and greebles swung rather than tumbled. Avian turns a body about
its centre of mass and reads that off the body's colliders, and BOTH finales
hand out a body with no collider on it: `shed_dead_fixtures` strips the shape
because debris is not material, and `detach_destroyed_body` waits on
`ChunkGrace` for its own. With nothing to derive from, the centre falls back to
the entity ORIGIN - which is nowhere near the piece. A fixture is authored
around the face it mounts on: `plate_collider` hangs its box at
`-REACH + volume * 0.5`, half a cell clear of a thin plate, and `decor_collider`
stands a greeble's box on a foot at `y = 0`. So a plate orbited a point up to
5 m off itself at 2-6 rad/s, which is 20 m/s of swing on top of a 15-40 m/s
kick.

Both now read the centre off the collider on the way out and state it as an
explicit `CenterOfMass`. The section half also fixes a pop that shipped in
0.12.0: a wreck piece pivoted about its origin for the whole grace and snapped
straight the moment `ChunkGrace` handed it a collider.

### The real-speed cut

`loop-section-railgun` is a twentieth of real time. That is what makes the
muzzle flash, the slug and the corridor separable at all, and it is also a lie
about the weapon - it makes something that guts a gunship between two blinks
look watchable. `NOVA_RAILGUN_LIVE=1` records the same walk with the clock
handed back at the moment the recorder opens, under
`loop-section-railgun-live`, and the railgun page carries both.

An armed run is frame-clocked at the profile's fps, so relative speed 1.0 is
one second of world per second of footage - the live cut plays back at exactly
the speed the gun goes off at. The clock changes at the LOOP-OPEN beat and not
at the framing beat before it: one frame of real time is a twentieth of the
charge that is left, so setting it any earlier fires the gun before the
recorder is open. The live cut also opens at `SIGHT_CHARGE`, which the beat
before already ran past, so the recording starts the moment that still is off
and carries about a quarter second of intact hull before the flash.
