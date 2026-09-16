# Phase 1 - UI prototype, for owner review

Built in sprout `training-handbook`. Placeholder lessons and placeholder copy
only; no authored content, no real media, no persistence.

The shipped shape is the one drawn in `SCHEMATIC.html` and approved on
2026-09-16: Factorio-style, one tip per screen. The entry points were settled
the same day, after the first pass put the only door in the corner: `Lessons`
in the menu card is the permanent way in, and the corner is a FIRST-LAUNCH
notice that offers Basic Training once.

## What ships in the prototype

- `Lessons` in the menu card, between `Scenarios` and `Mods`. That row is
  always there and never moves; it opens the modal and touches nothing else.
- The bottom-left corner is a NOTICE SLOT holding exactly one notice today: the
  first-launch offer to fly Basic Training. 520px wide, capped at 40% of the
  window, with `Start Basic Training` (primary), `Open lessons` and `Not now`.
  It is built once per menu entry and never reconciled.
- The offer is answered ONCE. `Start Basic Training` and `Not now` both write
  `TrainingPromptSetting::Hidden` and the corner goes for good; `Open lessons`
  does not, because reading is not answering. Nothing else can hide it, and
  losing it costs nothing now that the menu row is the way in.
- The answer is a SETTING, not progress: `Settings > Interface > Training
  prompt` (On|Off), persisted in `settings.ron` beside the skin and the window
  mode. That is the owner decision the task asked for - the dismissal is a
  thing the player switched, not a thing they learned, so it does not wait for
  the Phase 3 progress store and it is reversible in the one place a player
  looks for switches. `#[serde(default)]` means "no file yet" and "no field
  yet" both read as Shown, so a fresh install meets the offer exactly once.
- The handbook modal is 85%/85% in the normal green menu language - the same
  panel, list/details split, scroll bar and footer Back as Mods and Scenarios.
  No CRT casing, no prompt, no text input.
- Left pane: a header per non-empty category, then its lessons as rows with the
  TITLE and a state badge, nothing else. The list is a table of contents. Six
  categories come from `LessonCategory::ORDER` (Start Here, Flight, Combat,
  Shipbuilding, NOVA OS, Advanced); the layout has no idea how many there are.
- Right pane, ONE TIP ONE SCREEN: a 16:9 media frame across the full width of
  the pane, and under it a text box holding the title, the state badge, the
  whole body, the live binding chips, the Practice button and the wiki path.
  There is no page cursor and no Prev/Next. Anything that wanted a second page
  is its own lesson with its own id - that is what lets the list be the table
  of contents and lets `Viewed` mean the player saw a whole thing.
- The frame is drawn for EVERY lesson, so it never appears or disappears as the
  player walks the list. The frame keeps 16:9 on every machine and the text box
  is what gives: about a quarter of the pane on a desk monitor, about half at
  the 640x600 floor. The body scrolls inside the box; the action strip under it
  does not, so the way on stays on screen.
- 24 placeholder tips, every body inside `LESSON_BODY_MAX_WORDS` (45 words),
  which a unit test enforces. The cap is a LAYOUT claim, not a style rule: it
  is what keeps the text box from needing the scroll bar on a desk monitor.
- Progress: `New` has no badge, `Viewed` reads `[READ]`, `Completed` reads
  `[DONE]`; the summary on the title row reads `1 of 24 completed - 3 opened`.
  Opening a lesson marks it Viewed and never Completed. A default or repaired
  selection deliberately marks nothing - only a click does.
- Loading screen: a `FIELD NOTE` slot pinned to the bottom, two short lines,
  `Pickable::IGNORE`, absolutely positioned so a one- or two-line note never
  moves the mark, the LOADING line or the sweep. Boot facts are compiled, so
  the boot panel has one before the asset collection exists. Nothing holds a
  load gate for it.

## Captures (`captures/`, 1920x1080 and the 640x600 floor)

| Frame | Desktop | Narrow |
| --- | --- | --- |
| Front door: prompt + `Lessons` | `training-menu.png` | `training-menu-narrow.png` |
| Lesson list, text tip | `training-lessons.png` | `training-lessons-narrow.png` |
| Loop tip, chip, Practice | `training-lesson-visual.png` | `training-lesson-visual-narrow.png` |
| The switch, in Interface | `training-prompt-setting.png` | `training-prompt-setting-narrow.png` |
| Loading fact | `training-loading-fact.png` | `training-loading-fact-narrow.png` |

Produced by `examples/screenshots/screenshot_training.rs` (3 frames) and
`screenshot_loading_fact.rs` (2 frames), each run twice, `--narrow` pinning the
window to `MIN_WINDOW_WIDTH` x `MIN_WINDOW_HEIGHT`.

Every run is a FRESH install and none of them touches the developer's profile:
`SettingsStorePlugin::from_env` is Inert under the autopilot, so the prompt
setting sits at its default and the walk that presses `Start Basic Training`
writes no file.

Both walks assert on the screen's own record, not on the beat's intent: the
clicked row must carry `Selected`, the media frame must be laid out and
visible, the `Lessons` row and the prompt must both resolve as laid-out visible
nodes, the Interface tab must actually hold the On|Off row, and the corner must
be gone from the layout while a modal is up.

## Found and fixed while inspecting the frames

- The handbook opened EMPTY in the real app while every headless test passed.
  The catalog and the progress record are inserted when the plugin is built,
  which is before `Update` first initializes during `Loading`, so every refresh
  condition saw stale change ticks forever. The fix is a just-spawned trigger
  (`Added<TrainingList>`, `Added<TrainingDetailsPanel>`, `Added<TrainingCard>`),
  and the regression is pinned by
  `the_list_draws_after_a_boot_that_ran_frames_before_the_menu`, which runs
  frames before entering the menu the way the shipped boot does.
- At 640x600 the corner card overlapped the menu card by 8px. The cap is 40%
  instead of 45%, which leaves a gap at the floor and changes nothing on a desk
  monitor.
- The header used the small-caps `panel_head` treatment; its sibling modals use
  a 24px title over a muted subtitle. It matches them now, with the progress
  summary riding the title row rather than taking a third line.
- `Full manual: ...` slid to the LEFT edge of the text box on a lesson with no
  Practice button, so the line moved across the box as the player walked the
  list. The strip is right-anchored when it holds one child.
- `refresh_settings_tab` was one parameter under bevy's system-param ceiling,
  so the prompt row would not compile. The settings it draws are one
  `SettingsValues` param now, which is also where the next one goes.

## Known, deliberate, and left for the review

- The main menu card bleeds past the right edge of the modal. That is how every
  menu modal in the shipped game looks (see
  `web/src/assets/wiki-scenarios-picker.png`); the handbook was not given a
  private fix for it.
- `Full manual: wiki/flight` is static text, not a button. Phase 1 has no
  browser-opening path, and a button that does nothing is worse than a line of
  text.
- The binding chips sit INSIDE the scrolling body, under the text. At the
  640x600 floor a long tip puts them below the fold.
- THE MENU FIELD NOTE IS GONE. The corner used to carry a Wesnoth-style note in
  its quiet state; with the corner reduced to the one-off offer there is
  nowhere on the menu for it. Loading screens keep their facts (boot and
  scenario, `nova_core::loading_screen`, which picks its own). Phase 4's menu
  note needs a new home - a second notice in this corner is the obvious one.
- `screenshot_menu.rs` writes the wiki asset `tutorial-menu.png`, which now has
  a `Lessons` row in it. That asset has not been regenerated here.
- The 45-word body cap is my number, not the owner's. It is the one place the
  layout constrains the writing, so it is the thing to argue with first.
- MAIN MENU ONLY. There is no pause-overlay entry to the handbook in this
  phase, by owner direction.
- Gamepad navigation is untouched: no second navigation policy here. It stays
  with `20260714-001140`.
- KEYBOARD NAVIGATION IS NOT PROVEN, because the shipped menu does not have
  any: there is no `TabIndex`, `TabGroup` or `InputFocus` in `nova_menu` or
  `nova_ui`, and Escape plus the pointer is the whole policy. The handbook
  follows that policy exactly rather than inventing a second one. Focus
  traversal for the whole menu is `20260714-001140`'s call, next to the pad.

## Verification

- `cargo test -p nova_training --lib` - 24 passed.
- `cargo test -p nova_menu --lib` - 153 passed (16 the handbook's, one the new
  store rule that a file with no prompt field still offers the prompt).
- `cargo test -p nova_core --lib` - 16 passed.
- `cargo test -p nova_probe_cli --test catalog_drift` - 2 passed.
- `cargo fmt --all --check` clean; both capture examples build under
  `--features debug`.
- Ten frames rendered on a real GPU (Xvfb :98, 1920x1080) and inspected.

## The gate

Phase 2 does not start until the owner approves: the screen name, the layout,
the information density, the progress treatment, the visual-media presentation,
the corner prompt and the Interface switch that owns it.
