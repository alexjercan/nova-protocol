# Wiki widgets pass

Widgets and the prose around them. Screenshots and loops are a separate lane.

## Widgets added

Three, registered in `web/src/widgets.ts` and hydrated by `data-widget` key.
Each carries static fallback prose that states its result on its own.

- `ammo-rhythm` - `wiki/combat-weapons.md`, "Ammo & reloading". A magazine as a
  RATE LIMIT. Replays a burst-and-pause trigger pattern against the reload
  rule and plots the level. The point it makes that the prose could not: the
  batch is all-or-nothing on a WHOLE quiet interval, so a pause a tick short
  of the delay returns nothing, and any shot that lands restarts the interval.
- `turret-arc` - `wiki/sections/turret.md`, new "What it can bear on". Traverse
  is unbounded, so the entire blind volume is the cone under the mount's own
  keel. Draws the covered band, the hull sitting in the blind wedge, and the
  slew cost of a swing. This is the geometry `combat-weapons.md` spends three
  paragraphs on under "Point defense" and "Barrel discipline"; both now link
  here instead of re-explaining it.
- `torpedo-run` - `wiki/sections/torpedo-bay.md`, new "The two run-ins". Lance
  and Serpent race one 300 u run-in under one transport clock, with the weave
  tapering out over the terminal band. A defender toggle marks where one stock
  PDC kills each. Arming it is the whole argument: the Lance dies 114 u out,
  the Serpent 40 u out.

No new `:root` tokens and no new mark classes - all three reuse the existing
widget vocabulary, so `tests/theme.test.ts` checks (d)/(e)/(f) are untouched.

## Tables, not widgets

- `wiki/keybinds.md` rebuilt as three columns - action, keyboard & mouse,
  gamepad - with a `<thead>` per table, behind a new `.controls--split`
  modifier in `style.css` (plus a narrow-viewport rule). The old form crammed
  both devices into one cell joined by "or", so a gamepad player had to pick
  their half out of every row. The Editor table stays two columns: the editor
  has no gamepad bindings, and a column of dashes is not information.
- `wiki/settings.md` already leads with a preset table and folds the detail
  behind `show-explanation`. Left alone.

## Pages deliberately given nothing

- `glossary.md` - 25 lines of definitions. Prose is the correct form.
- `getting-started.md` - a walkthrough. Its visuals are the authored capture
  slots (other lane), and a widget would restate the pages it links to.
- `scenarios.md` - a list of authored content. Nothing to compute.
- `sections.md` - overview plus catalog table; the mechanisms live on the two
  child pages that got widgets.
- `sections/hull.md` - a hull is passive. Its only number is health, which the
  catalog table already carries.
- `sections/thruster.md` - REJECTED ON SOURCING. A "more thrusters, more push"
  widget needs acceleration, which needs hull mass; mass is not authored, it
  falls out of section geometry and density. No number to lift, so no widget.
- `nova-os.md` - long (334 lines) but already widget-first: five tables, five
  `show-explanation` folds and a widget. Length here is reference material,
  not a wall of text.

## Numbers lifted, with source

Constants are in `web/src/widgets.ts` with the `file:line` on each comment;
page numbers carry a verification comment in the markdown. Paths below are
under `crates/`.

Turret mount (`nova_authoring/src/base_content/sections/standard.rs`):

- traverse limits `None`/`None` :142-143 (unbounded)
- elevation min `-TURRET_DEPRESSION_LIMIT` :157, that constant `PI / 18` :84
- elevation max `FRAC_PI_2` :158
- hinge speed `PI` rad/s :141,:150
- muzzle speed 100.0 :273, projectile lifetime 2.0 :280
- `nova_ship/src/sections/turret_section/config.rs:124-126` states that product
  IS the reach (200 u)
- fire gate `TURRET_ON_TARGET_RAD = 1.6 / 100` at
  `nova_ship/src/sections/turret_section/aim.rs:19,:24,:47`, the source's own
  comment reading 0.016 rad / 0.92 deg at :26-27
- reachability derived from the elevation hinge alone,
  `nova_ship/src/sections/turret_section/arc.rs:46-102`
- 58.7% sky coverage is DERIVED in the widget from the depression floor,
  `(1 - sin(floor)) / 2`. Not an authored number.

Magazines and reload:

- PDC `ammo_capacity: Some(500)` standard.rs:286, `delay: 3.0` :288,
  `amount: 200` :289, `fire_rate: 100.0` :262
- bay `ammo_capacity: Some(6)` :591, `delay: 10.0` :603, `amount: 1` :604,
  `fire_rate: 1.0` :558
- semantics in `nova_ship/src/sections/ammo.rs`: a successful shot resets the
  clock :136, a whole batch lands at the delay :171-174, clamped at capacity
  :156, empty trigger pulls never reset :134
- sustained rate formula `amount / (delay + amount / rate)` is standard.rs:594's
  own, which works the PDC through it to 40 rounds/s

Torpedo types (`nova_authoring/src/base_content/sections/ordnance.rs`):

- the measured comparison table is the module header, :13-21 - cruise caps,
  weave half-angles, rounds one PDC spends (116 / 390), where it kills each
  (114 u / 40 u), time over a 300 u run-in (9.10 s / 9.78 s), speed along the
  line (31.3 / 29.1 u/s)
- closing rates against a 25 u/s runner (6.3 / 4.1 u/s) at :23-25
- Lance `LANCE_MAX_SPEED = 35.0` :49, weave zeroed :72-73
- Serpent `max_speed: 32.0` / `weave_angle: 0.44` / `weave_rate: 1.4` at
  `nova_ship/src/sections/torpedo_section/mod.rs:347-349`
- the 11.1 u swing off the direct line at that angle and rate, mod.rs:293-303
- weave taper full beyond 3x blast radius to zero at 0.5x,
  `nova_ship/src/sections/torpedo_section/projectile.rs:330-334`
- blast radius 30.0 standard.rs:565
- warhead `projectile_health: 10.0` standard.rs:583, sized above the hardest
  single PDC round with the reasoning at :580-582; siege bay 5000.0 at :509

One modelling decision worth recording: the widget drives each torpedo's
position from the MEASURED arrival time for this run-in, never from the cruise
cap or from the speed along the line. Those are three separate measurements of
three separate questions, and 300 / 9.10 is not 31.3. Dividing one by another
put the Serpent 15 u short of a target the harness says it reaches.

## Numbers already on a page that no longer match the source

Three of these, all the same defect: a world unit read as a metre, when
`nova_events/src/scale.rs:14` puts `METERS_PER_UNIT` at 10.0. All three FIXED.

1. `wiki/combat-weapons.md` - "the fuze fires about a metre off the nearest
   part of that body's skin". `CONTACT_FUZE = 1.0` at
   `nova_ship/src/sections/torpedo_section/projectile.rs:76` is one world unit,
   so it is about TEN metres.
2. `wiki/sections/torpedo-bay.md` - the same claim, same constant, same error.
3. `wiki/combat-weapons.md` - "a round that lands within about a metre of the
   last one joins that crater". `MERGE_MAX = 1.0` at
   `nova_gameplay/src/integrity/carve.rs:128` is one world unit, so ten metres.
   The `[Unreleased]` changelog entry for the same change says "within a unit
   of itself", which is right; the wiki converted it wrongly.

Two documentation-rule defects, also FIXED:

4. `wiki/sections/turret.md` claimed "the ten per-craft `*_turret_*` modules
   ... have been removed". That is history on a reference page (Documentation
   1), and the removal is only in `CHANGELOG.md` `[Unreleased]` (line 28), so
   there is no RELEASED section naming the old prototypes (Documentation 2/3).
   Clause dropped; the page now just says every craft mounts the same gun.
5. `wiki/combat-weapons.md` - "A warhead NOW carries enough hit points that no
   single PDC round can swat it". "Now" frames the reader against the last
   commit rather than the last release. Reworded to state the fact.

One claim left alone but flagged:

6. `wiki/combat-weapons.md` - "the difference between a salvo that costs a
   defender a magazine and one that costs it four". No measurement produces
   "four". The type sweep gives 3.36x (390 / 116, ordnance.rs:13-21); a
   separate 400 u angle sweep gives 4.04x (1245 / 308,
   torpedo_section/mod.rs:280-289); a six-torpedo salvo costs 696 vs 2340
   rounds, which is 1.4 vs 4.7 PDC magazines of 500. Defensible as colour,
   not lifted from a measurement. Retuning prose numbers to fit is exactly
   what Web 2 forbids, so it stays until an owner decides which rig the page
   should quote. The neighbouring "roughly three times the ammunition" IS
   sourced.

## Belongs on another surface

Flagged only - `docs/` is another lane's and neither was changed.

- `wiki/nova-os.md`, "In the WFC arena". Documents
  `cargo run --example wfc_arena`, a developer match bench run from a source
  checkout. A player cannot reach it. This is dev-book material on a player
  page.
- `wiki/scenarios.md`, the "Show the full vocabulary" fold. The events /
  filters / actions list is the authored contract, which is `/create/`'s job
  and has to be exact there. The page already links to `/create/` at the end;
  the fold duplicates it in a place that cannot be held exact.

## Verification

- `npm run ci` in `web/` - format, lint, test, build.
- Rendered and INSPECTED every touched widget in headless Chromium against the
  built `dist/`, including the `torpedo-run` defender state and the
  `ammo-rhythm` dry case. Three collisions were found that way and fixed: a
  depression-floor label over the hull block, torpedo lane labels under the
  weave, and the "ran dry" marker sitting on the trace.
- The site ships ONE skin (phosphor on dark). There is no light theme and no
  `prefers-color-scheme` branch in `style.css`, so there is no second theme to
  check; `tests/theme.test.ts` check (f) is what keeps the retired light-3D
  skin out.
