# Phase 2 - authored lessons, media, and practice ranges

Built in sprout `training-handbook`, on the Phase 1 screen the owner approved
on 2026-09-16. The placeholder catalog is gone: lessons are CONTENT now, and
the handbook draws whatever the merge produced.

## Lessons are mod content

- `Content::Lesson` joins the shared content enum, so one `Lesson((...))` item
  per lesson lives in an ordinary `*.content.ron`. The base set is
  `assets/base/training/base.content.ron`, listed by `base.bundle.ron`. There
  is no second loader, no per-mod catalog blob, and no new file kind.
- Everything the format already does, lessons get for free: bundle load,
  last-wins overlay by id, `self://` and `dep://` rewriting and validation,
  dependency order, reload, and the offline `content lint` walk.
- 24 lessons in the six fixed categories. A mod adds lessons to a category or
  replaces a base lesson by declaring its id; it does not add a seventh
  category.
- `order` is required and ties break on id, so the list never depends on which
  mods are installed.
- Required fields are explicit and checked in `nova_training/src/validate.rs`:
  an empty id, title, body, wiki path or alt text is an Error; a body over the
  45-word cap is an Error; a loop whose grid cannot cut its sheet is an Error;
  a field note that will not wrap onto the slot's two lines is an Error; a
  `practice` naming an unknown scenario, or one that is not a practice range,
  is an Error. A practice range no lesson sends anyone to is a Warn.

## Media

- `Image` is one still. `Loop` is a sprite sheet plus the grid that cuts it -
  `columns`, `rows`, `frames`, `frames_per_second`, `alt` - which is the
  native+wasm-safe form the task asked us to choose over GIF or video: it is
  Bevy's own image pipeline and needs no second decoder.
- 14 of the 24 lessons loop; the rest are stills. Every path is a `self://`
  reference in the bundle's resource list, so it is gated like any other asset.
- The art is PLACEHOLDER: `scripts/gen-lesson-media.py` renders a deterministic
  phosphor-look PNG per lesson from its own title and id, so every lesson looks
  different and nothing is blank. `--check` re-renders in memory and compares
  byte for byte, and `tests/lesson_media.rs` holds the generator and the
  authored set to the same list. Real footage overwrites the same paths with no
  code change.

## Scenario roles, and the four practice ranges

- `ScenarioConfig::menu_backdrop` became `role: ScenarioRole`, one of
  `Chapter`, `Backdrop` or `Lesson`. That is what makes "a lesson practises
  somewhere FOCUSED" checkable rather than a convention. `hidden` is gone with
  it - the Scenarios list shows every installed chapter.
- Nine lessons carry a practice range - two each on `drill_momentum` (Momentum
  practice), `drill_stop` (STOP practice) and `drill_autopilot` (Autopilot
  practice), three on `drill_gunnery` (Turret practice). None of the four is in
  the Scenarios list.

### The owner's drill note, and what it changed

The first pass flew, but the owner's read was right: the drills had no ending,
the objective text was too long to take in, and nothing told the player which
verb the drill was even about. `base_content/scenarios/drills.rs` now holds
four rules, and `drills/tests.rs` pins each one.

- THE HELM IS THE LESSON'S. Each range authors the player ship's
  `capabilities` and withholds every verb its lesson does not teach, so the
  keybind chip for a verb the player must not press is not drawn at all. The
  momentum range has no flight computer whatsoever - RCS only. STOP adds STOP.
  The autopilot range adds the lock, GOTO and ORBIT but no STOP, because
  stopping is the previous lesson. Gunnery has STOP and the lock but no
  autopilot.
- THE BOARD IS ONE SHORT LINE. The objective chip is a notification with a
  12 s dwell, so it is read once and leaves: "Fly out to mark ALPHA." The
  explanation moved to the comms stack, under the `Training` label.
  Each beat also owns its OWN objective id and completes it before the next is
  posted - the old drills re-posted one id, which `world.push_objective` warns
  about and stacks.
- THE VERB IS LIT. `HintEmphasisSet` pulses the chip for the verb in hand and
  the beat that consumes it clears the emphasis again.
- IT ENDS. A finished range posts a Victory with nothing queued, so the outcome
  overlay offers Main Menu and the player lands back at the handbook. A wrecked
  or lost trainer posts a Defeat that queues the range itself, so the overlay
  offers Retry. Both defeat handlers are guarded on a `beat` variable set in
  the same frame as the win, so a wreck during the three-second outro does not
  turn a won drill into a loss.

## Links into the manual

Each lesson ends with the page the whole story is on, and nothing in the game
can check that line: the manual is a website, the lesson is content, and
`wiki_path` is a string. The first authored set proved the point - seventeen of
the twenty-four named pages the site has never served (`wiki/flight`,
`wiki/combat`, `wiki/shipbuilding`, `wiki/turrets`). Every path now names a
page `web/src` ships, with the anchor of the heading that carries the claim,
and `tests/lesson_wiki_links.rs` resolves all 24 against the repository, page
and anchor both. The strip draws the PAGE without the anchor: an anchored path
is one word with no spaces to wrap at, so it would run out of the box at the
narrow floor, and the page is what a player would type.

## Field notes

`catalog_field_notes` derives the menu and scenario-load notes from the merged
catalog, so a mod's lessons are in the rotation and every claim still has one
owning lesson. The compiled `boot_field_notes` set is the FLOOR under it, not
the normal path: the boot panel draws before the main asset collection exists.

## The voice: a course, not a radio drama

The owner read the first authored pass and asked for plain tutorial language:
explain how to do it, in simple words, with no roleplay and no quirk. Every
lesson title, every lesson body, every field note and all seventeen drill cues
were rewritten against that. The titles now say what the lesson is - "Turn,
then thrust", "You keep your speed", "The STOP order" - and the handbook
subtitle says what the screen is for.

Reading every string against the code found four that were not just quirky but
wrong, and the rewrite corrects them:

- Torpedoes did not "fly straight and never turn". A torpedo needs a combat
  lock, and it steers itself onto that lock
  (`web/src/wiki/sections/torpedo-bay.md`).
- An off-centre thruster did not make "the hull fight you". The flight computer
  sets each throttle to cancel the spin, so a lopsided ship still flies
  straight with less thrust (`web/src/wiki/sections/thruster.md`).
- Thrust did not "spend fuel". There is no fuel system.
- The HUD had no hull readout and no centre reticle. The instruments sit around
  the ship, and the lesson is about the velocity sphere.

The drills kept the comms card and dropped the character, which was the owner's
call between the two options. `TRAINING` is a LABEL, not a callsign: the cue
carries no portrait and no `icon`, the player's ship is called "Your ship", and
the ranges are named for the course - Momentum practice, STOP practice,
Autopilot practice, Turret practice. Basic Training keeps its cast: the
constants are local to `base_content/scenarios/drills.rs`, and
`base_content/scenarios/tutorial.rs` is untouched.

One deliberate asymmetry: a lesson never names a key, because the keybind chip
under it draws the binding, but a drill cue does name a device ("Scroll the
wheel to step the lock onto one section"). The scroll wheel has no chip, and a
cue is read with a hand already on the controls.

## Captures (`captures/`)

| Frame | Desktop | Narrow |
| --- | --- | --- |
| Front door: prompt + `Lessons` | `training-menu.png` | `training-menu-narrow.png` |
| Authored lesson list | `training-lessons.png` | `training-lessons-narrow.png` |
| Authored loop, chip, Practice | `training-lesson-visual.png` | `training-lesson-visual-narrow.png` |
| Momentum practice, first beat | `live_drill_momentum.png` | - |
| STOP practice, first beat | `live_drill_stop.png` | - |
| Autopilot practice, first beat | `live_drill_autopilot.png` | - |
| Turret practice, first beat | `live_drill_gunnery.png` | - |

The four drill frames are live runs on a real GPU (Xvfb :98), each
one captured two seconds after the log recorded its first objective post,
because the chip and the comms card both leave on their own. Read together they
are the drill contract on screen: one short objective line, the `Training`
card with no face on it, the world markers, and exactly the permitted chips -
momentum shows RCS alone, STOP shows STOP emphasised beside RCS, autopilot
shows RADAR emphasised beside RCS with no STOP, gunnery shows RADAR emphasised
beside STOP and RCS with no GOTO or ORBIT.

## Verification

- `cargo test -p nova_authoring --lib` - 106 passed (6 of them the drills').
- `cargo test -p nova_training --lib` - 32 passed.
- `cargo test -p nova_menu --lib` - 156 passed.
- `cargo test -p nova_core --lib` - 17 passed.
- `cargo test -p nova_assets --lib` - 81 passed.
- `cargo test -p nova_modding --lib` - 2 passed.
- `cargo test -p nova_scenario --lib lint` - 81 passed.
- `cargo test -p nova_authoring --test lesson_media --test campaign_membership
  --test ledger_campaign --test content_ron_parity` - 2, 2, 10 and 2 passed.
- `cargo test -p nova_authoring --test lesson_wiki_links` - 1 passed, and it
  was watched to fail on both halves: on `wiki/flight` (no such page) and on
  `wiki/flight-autopilot#rcs` (no such heading).
- `content gen` then `content lint` - 0 errors, 0 warnings, 0 findings, 13
  scenarios balance-audited, 18 creative maps. Generated diffs inspected.
- `cargo fmt --all --check` clean. `mdbook build` and `web && npm run ci` pass.
- Four live drill runs inspected, each reporting zero duplicate-objective
  warnings.
- After the rewrite: all ten handbook frames retaken at 1920x1080 and at the
  640x600 floor and inspected, and all four drills reflown and recaptured. The
  narrow frames are the containment case - the longer titles wrap inside their
  rows and the manual line still fits - and the drill cards carry the
  `TRAINING` label with no portrait.
- `scripts/gen-lesson-media.py --check` re-renders all 24 placeholder frames in
  memory and matches the committed PNGs byte for byte after the retitling.

## Known, deliberate, and left

- THE ART IS A PLACEHOLDER AND THE STILLS SAY SO: each one carries
  `<lesson id> - placeholder` under its title card. A loop carries its title
  and the motion only. It is deterministic and per-lesson, and real footage is
  owner work that drops onto the same 24 paths with no code change.
  SUPERSEDED on 2026-09-16: three of them are real footage now, the
  demonstrations are WebP rather than PNG, and the generator compares pixels
  rather than file bytes. See the visuals line in `TASK.md`.
- THE BROWSER RUN IS DONE. Built with Trunk, served, and driven with a real
  pointer through the main menu into the handbook and onto a Loop lesson:
  `captures/web-training-lessons.png` is the list, and
  `web-training-loop-a.png` / `web-training-loop-b.png` are the same lesson 1.2
  seconds apart, visibly different - which is the sprite sheet both CUT and
  ANIMATING, the thing wasm clippy could not say. No 404s and no asset errors in
  the console; both `bevy_asset_loader` states report done. A headless Chromium
  screenshot of a WebGPU canvas comes back blank, so the frames are X-display
  captures of a real window on Xvfb.
- The win and loss banners were not flown by hand. They are pinned structurally
  by `every_drill_ends_both_ways` and
  `a_won_drill_queues_nothing_and_a_lost_one_queues_itself`, and they use the
  same outcome machinery Basic Training's ending already ships.
- Gunnery ends on Target 1 alone - the marked, named hulk. The other four stay
  as range furniture, so a belt rock taking a stray round is not an ending.
- Practice marked no lesson completed IN PHASE 2. Reading marked it read, and
  nothing in the running game called `mark_completed`. Phase 3 gave completion
  its writer - see `PHASE3.md`.
