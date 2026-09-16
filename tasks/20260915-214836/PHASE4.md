# Phase 4 - menu and loading facts

Built in sprout `training-handbook`, on the Phase 3 progress work. The facts
machinery already drew both loading screens; this phase gave the note a
DESTINATION on a scenario load and a HOME on the menu.

## What was already there

`crates/nova_training/src/facts.rs` and `crates/nova_core/src/loading_screen.rs`
shipped in Phase 1 and answer three of the five checklist lines on their own:

- `catalog_field_notes(catalog)` derives a note from `Lesson::field_notes`, so
  the claim is written once, in the lesson that owns it. 15 of the 24 shipped
  lessons carry one.
- `boot_field_notes()` is COMPILED, which is what lets the boot screen draw a
  fact before any asset collection exists. The scenario screen reads the merged
  catalog instead, so an installed mod's lesson can come up there.
- `wrap_note_lines` refuses anything that needs a third line, and
  `FieldNoteRotation` is the no-immediate-repeats rule.

## Destination context

`notes_for_scenario(catalog, scenario)` is the notes of every lesson that
teaches that scenario - its `practice` range, or one of the scenarios its
`proven_by` names. `FieldNoteRotation::pick_preferred(preferred, all, roll)`
takes one of those first and falls back to the whole set when the destination
has nothing left to say, which is the checklist's "valid general fallback".

`spawn_scenario_load_screen` now reads the event it already had -
`LoadScenario(ScenarioConfig)` carries `.0.id` - so the note a player reads
while a practice range comes up is about what the range will ask them to do. A
campaign chapter no lesson points at draws a note from the whole book rather
than an empty slot.

The rotation's memory bookkeeping moved into one `remember` helper, so a
preferred pick and a general pick spend the set the same way.

## The menu's second notice

The bottom-left corner was built for one notice and held one. It now holds two,
and the two are gated apart:

- `MenuAside` (the corner) is CONTAINMENT only: hidden while any menu modal is
  open, because the modals are 85 percent of the window and a card at the edge
  would otherwise still be clickable.
- `TrainingPromptCard` (the offer) carries the setting: `Inherited` while
  `TrainingPromptSetting::Shown`, `Hidden` once answered. `Inherited`, not
  `Visible`, so the corner still decides whether anything in it is on screen.
- `MenuFieldNoteCard` (the note) has no gate of its own. It was never an offer,
  so answering the offer leaves it standing - which is the point: a player who
  dismissed the prompt on day one still gets a fact on every menu entry.

The note is picked ONCE, in `setup_menu_ui`, off the same session
`FieldNoteRotation` the loading screens use - so a note read on a load does not
come straight back on the menu. The corner is built on menu entry and never
reconciled, which is the "do not rotate it while visible" rule for free.

`Open lesson` on the card opens the handbook AT the lesson the fact came from
and marks it Viewed - a click is a choice, unlike a default selection. A
compiled note with no lesson behind it gets no button.

## Files

| Change | Where |
| --- | --- |
| `notes_for_scenario`, `pick_preferred`, `remember` | `crates/nova_training/src/facts.rs` |
| Prelude export | `crates/nova_training/src/lib.rs` |
| Destination pick on the scenario screen | `crates/nova_core/src/loading_screen.rs` |
| The note card, its observer, the split visibility | `crates/nova_menu/src/training.rs` |
| The once-per-entry pick | `crates/nova_menu/src/menu_ui.rs` |
| `FieldNoteRotation` for a menu-only app | `crates/nova_menu/src/lib.rs` |
| One fixture lesson with a note | `crates/nova_menu/src/tests/support.rs` |
| Live destination assertion | `examples/screenshots/screenshot_loading_fact.rs` |
| Live corner assertion | `examples/screenshots/screenshot_training.rs` |

## Verification

- `cargo test -p nova_training --lib` - 38 pass, including
  `a_ranges_notes_are_the_lessons_that_teach_it` and
  `a_preferred_pick_falls_back_rather_than_repeating`.
- `cargo test -p nova_core --lib` - 20 pass, including
  `the_scenario_screen_prefers_a_note_about_where_it_is_going`,
  `a_scenario_no_lesson_teaches_still_draws_a_note` and
  `a_field_note_does_not_hold_the_load_or_take_a_click`. The last one asserts
  the slot's `Pickable::IGNORE` and the SAME dwell-then-settled teardown the
  note-free screen has - ownership, state and teardown, never a duration.
- `cargo test -p nova_menu --lib` - 176 pass, including
  `the_corner_draws_a_field_note_from_the_catalog`,
  `the_note_opens_the_lesson_it_came_from`,
  `a_catalog_with_no_notes_draws_no_note_card`,
  `not_now_takes_the_offer_down_for_good` and
  `the_setting_is_what_decides_whether_the_offer_draws`.
- `screenshot_training` and `screenshot_loading_fact`, both widths, under Xvfb
  on the shipped app: clean cycles, no panic. Captures in `captures/`:
  `phase4-menu-field-note.png`, `phase4-menu-field-note-narrow.png`,
  `phase4-loading-destination-note.png`.
- `probe run system_training_journey --correctness-only --norender` - OK, 6/8
  measured, `reached_playing` at frame 68, 0 invariant violations, 0 offending
  log lines. The journey drives the corner prompt, so the split visibility is
  walked by a real pointer as well as by the live-tree tests.

No authored content changed, so no `content gen` or `content lint` was needed.

## One repair on the way past

`cargo check --all-targets --features debug` turned up
`examples/systems/bug_menu_fallback.rs` failing to compile: Phase 2 replaced
`ScenarioConfig::menu_backdrop` with `role: ScenarioRole` and left this example
on the old field, so nothing had built it since. Migrated in place - a demotion
now moves a Backdrop to Chapter rather than clearing a flag on every scenario,
which keeps the Lesson ranges' role intact. `probe run bug_menu_fallback
--correctness-only` under Xvfb is OK, 6/8 measured, 0 invariant violations (it
clicks a pause-menu button, so it needs a window; `--norender` stalls it).
It is breakage introduced and fixed inside this cycle, so it gets no changelog
entry.
