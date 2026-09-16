# Player training handbook, first-run guidance, and loading facts

- STATUS: OPEN
- PRIORITY: 52
- TAGS: v0.14.0, ui, tutorial, menu, content, persistence

## Goal

Ship a Factorio-style in-game training handbook for the v0.14.0 itch.io
release. It is a condensed, visual companion to the player wiki: short lessons,
current bindings, progress, practice routes, and related-wiki links. It is not a
1:1 wiki renderer and not a NOVA OS terminal app.

Owner direction (2026-09-15): recommend Basic Training to a first-time player;
add Wesnoth-style facts to the menu and loading screens; organize the handbook
like Factorio's tutorial menu; show looping demonstrations or editor-style
visuals; use the normal green menu UI in a main-menu modal with no text input;
build and review the UI before authoring the real lesson data and descriptions.
This is an owner-approved feature exception to the v0.14.0 stabilization board
for the itch.io release.

Owner direction (2026-09-16), on the Phase 1 prototype, drawn in
`SCHEMATIC.html` and approved: match Factorio harder. ONE page per tip - a
full-width looping demonstration on top and a text box under it holding the
whole tip. Main menu only for now; no pause-overlay entry.

Owner direction (2026-09-16, later the same day), on the entry points: put
`Lessons` in the main-menu card, and give the bottom-left corner back to the
first-launch `play tutorial` prompt alone. Treat the corner as a notice centre
that today holds one notice. Add an Interface setting that switches the corner
on and off, and have playing the tutorial or dismissing the prompt switch it
off. This REPLACES the earlier "the corner card is the only door" decision.

## Product contract

- Open the handbook from the main-menu card's `Lessons` row, in a large
  normal-menu modal. Do not reuse the NOVA OS CRT casing, terminal prompt, or
  text-input path. The bottom-left corner is a first-launch prompt, not the
  entrance: it can be switched off and the handbook must stay reachable.
- Use the existing list-and-details screen language: categorized lesson list on
  the left; on the right ONE tip - a full-width 16:9 demonstration over a text
  box holding the title, the whole body, bindings and actions. No page cursor:
  a second page is a second lesson. Progress summary and Back remain visible.
- The in-game handbook is concise and visual. The website wiki remains the
  complete searchable manual. Lessons carry stable related-wiki paths.
- Distinguish `Viewed` from `Completed`. Reading or acknowledging a lesson marks
  it Viewed. A real tutorial outcome or practice assertion marks it Completed;
  opening a page alone never claims mastery.
- Show current action bindings from `InputBindings`. Do not hard-code default
  keys in lesson prose, diagrams, or facts.
- Support static images and a portable looping demonstration first. Treat GIF
  as a UX description, not a required file format: spike animated sprite sheets
  against literal GIF/video playback and choose the native+wasm-safe path before
  setting the authored media format. A live 3D/editor viewport is optional only
  if the spike shows it is simpler and robust.

## Phase 1 - UI prototype and owner review

Use placeholder lessons, placeholder text, and representative temporary media.
Do not build the final lesson catalog or write final descriptions in this phase.

- [x] Add the full-screen modal to the shipped main menu, opened from the
      `Lessons` row of the menu card. The row sits between `Scenarios` and
      `Mods`.
- [x] Prototype categories, lesson rows, selected/viewed/completed states,
      progress summary, a media frame, a live binding chip, related-wiki
      action, Practice action, and Back. Page navigation was prototyped, then
      CUT on the 2026-09-16 direction: one tip is one screen, and anything
      that wanted a second page is its own lesson.
- [x] Prototype the initial information architecture: Start Here, Flight,
      Combat, Shipbuilding, NOVA OS, and Advanced. The first shipped content set
      may be smaller, but the layout must not assume only three groups.
- [x] Prototype a non-blocking bottom-left first-pilot recommendation with
      `Start Basic Training`, `Open lessons`, and dismissal behavior. Starting
      or dismissing writes `TrainingPromptSetting::Hidden` and the corner is
      gone for good; `Lessons` in the menu card is unaffected.
- [x] Add `Settings > Interface > Training prompt` (On|Off), persisted in
      `settings.ron`. OWNER DECISION recorded: the dismissal is a switch the
      player threw, so it lives with the settings rather than with the Phase 3
      progress record, and the player can throw it back.
- [x] Prototype one short menu field-note card and one loading-screen fact
      slot. Both ship. The loading slot draws on the boot screen and the
      scenario screen; the menu card is the SECOND notice in the bottom-left
      corner, under the first-launch prompt and outliving it. Closed in
      Phase 4.
- [~] Prove mouse and keyboard navigation, modal ownership, scrolling, and
      containment at the supported narrow window. Coordinate gamepad behavior
      with `20260714-001140`; do not create a second navigation policy.
      Mouse, modal ownership, scrolling and narrow containment are proven.
      KEYBOARD IS NOT: the shipped menu has no focus traversal at all (no
      `TabIndex`, `TabGroup` or `InputFocus` anywhere in `nova_menu`/`nova_ui`),
      so there is nothing to prove and nothing to extend. Giving the handbook
      one of its own is the second navigation policy this line forbids; it
      belongs to `20260714-001140` with the pad.
- [x] Capture the main list, one visual lesson, first-pilot card, and loading
      fact at desktop and narrow widths. Inspect the rendered frames. Ten
      frames in `captures/`, reviewed in `PHASE1.md`.

STOP after the prototype. Record the captures and get owner approval on the
screen name, layout, information density, progress treatment, visual-media
presentation, and first-pilot card before Phase 2. UI approval is a real gate,
not an implementation checkpoint to pass mechanically.

Built and captured 2026-09-15: see `PHASE1.md` and `captures/`. The owner
approved the screen on 2026-09-16 - the layout, the one-tip-one-screen details
pane, the progress treatment, the corner prompt and the Interface switch that
owns it - and Phase 2 started against that shape.

## Phase 2 - authored lessons and media

Start only after the Phase 1 owner review is recorded on this task.

- [x] Define stable `LessonId`, category, title, media, body, action names,
      related-wiki path, optional practice target, fact references, and explicit
      progress rule. Unknown IDs and missing required fields fail at lint, then
      load. One tip is one screen: there are no ordered pages.
- [x] Keep the tip vocabulary narrow: one body under the word cap
      (`LESSON_BODY_MAX_WORDS`, 45 in the prototype - confirm the number), one
      media shape, and binding/action references by NAME. Do not embed general
      Markdown or HTML.
- [x] Decide the content owner and asset pipeline before implementation. Keep
      lesson content data-driven without hand-editing generated base content.
- [x] Author and review the first release lesson set. At minimum cover Start
      Here, manual flight and momentum, STOP, GOTO, ORBIT, travel versus combat
      locks, turrets, and torpedoes. Add Shipbuilding, NOVA OS, and Advanced
      lessons only where concise reviewed text and useful visuals are ready; do
      not ship filler to populate a category.
- [x] Give lessons current binding chips and related links into the existing
      player wiki. Update wiki anchors when needed and keep the wiki complete.
- [~] Add representative visuals: at least one flight loop, one targeting or
      combat loop, and one editor-style visual. Validate all media on native and
      wasm. Every lesson ships a demonstration and 14 of the 24 loop, so all
      three FORMS are covered, and all three ASKED-FOR visuals are now real
      footage of the game: the flight loop (`lesson_flight_aim`), the targeting
      loop (`lesson_combat_radar`) and the editor still
      (`lesson_build_sections`). Each is an ordinary capture producer run by
      `scripts/capture-lesson-media.sh`; `examples/screenshots/shared/lesson.rs`
      carries the two devices that make a sheet CLOSE - a POSE loop (frozen
      scene, camera on one sine period) and an ACTION loop (held camera, the act
      itself is the motion, with lead-in cells and a held result). A
      demonstration is WEBP at the size the pane draws it - a 4x5 sheet of
      960x540 cells, 3840x2700 in all, twenty cells at 10 fps - because the pane
      is about 900 logical pixels wide and the first cut of this shipped 240x135
      cells, which the owner rightly called unreadable. Twenty cells is the
      CEILING, not a preference: 4096 is the texture size a WebGL2 target is
      guaranteed, so a longer loop has to shrink the cell again.
      `lesson_combat_radar` is the action loop - the sheet opens unmarked, the
      dwell charges, the bracket lands, and the rest of the cells hold it.
      WASM: the browser run proved the sheet is CUT and ANIMATING with a real
      pointer (`captures/web-training-*.png`), and that mechanism did not
      change; the CODEC is held by
      `every_demonstration_decodes_and_a_loop_divides_by_its_grid`, which
      decodes every committed file with the crate `bevy_image` wraps - the same
      pure-Rust decoder on both targets. A browser re-run since the codec change
      is NOT done. STILL OPEN: the other 21 lessons keep generated placeholder
      art, which `scripts/gen-lesson-media.py` refuses to redraw over a capture,
      so each lands by writing its own producer.
- [x] Wire `Start Training` and applicable Practice actions through the existing
      New Game/scenario handoff. Basic Training is the base bundle's declared
      `tutorial`; do not add a competing launch path. Practice launches four
      `role: Lesson` ranges through the same hand-off.

Built 2026-09-16: see `PHASE2.md` and `captures/`. The owner's drill note the
same day - give them an ending, cut the objective text, withhold the verbs the
topic does not need, light what matters, use comms - is landed and is the
practice-range contract in `web/src/create/scenarios.md`.

Phase 3 built 2026-09-16: see `PHASE3.md`. The progress record, `proven_by` and
its validation, the completion writer, the store's access policy, and the
`system_training_journey` range that walks a fresh profile through a won range
and two restarts. Phase 4 (the facts) is untouched.

## Phase 3 - progress and first-player recommendation

- [x] Persist explicit viewed lesson IDs and completed lesson IDs. Do not infer
      first use from settings-file existence. The first-pilot dismissal is
      ALREADY DECIDED and shipped: it is `PersistedSettings::training_prompt`,
      by the 2026-09-16 owner direction, and player LEARNING stays out of the
      settings file. `PersistedTraining` is a second file beside the settings,
      under the same root, with two explicit id lists and nothing else.
- [x] A fresh profile sees the recommendation. `Start Basic Training` launches
      Basic Training; `Open lessons` opens Start Here. Dismissal survives
      restart (it is a setting) and the Interface row brings it back.
      `system_training_journey` walks the offer, `Open lessons`, `Not now` and
      its survival across three app lifetimes. `Start Basic Training` and the
      Interface row stay on the menu's live-tree tests, which drive the button
      and the switch directly.
- [x] Completing Basic Training marks only the lessons its asserted flow proves.
      A failed, abandoned, or merely opened scenario does not grant completion.
      A new lesson field, `proven_by`, says which scenarios may claim it;
      `complete_what_a_won_scenario_proves` is the only writer, and it reads
      Victory only.
- [x] Old, missing, extra, or corrupt progress data fails safely to conservative
      defaults. Scripted runs use isolated or inert storage and never touch the
      developer's profile. `TrainingStoreAccess` mirrors the settings store's
      three directions and goes inert under `harness_env_active`.

## Phase 4 - menu and loading facts

Built 2026-09-16: see `PHASE4.md`. The destination-aware scenario note, the
menu corner's second notice, and the corner's split visibility.

- [x] Derive field notes from the same authored learning catalog or stable
      lesson IDs. Do not maintain unrelated copies of the same claim.
      `catalog_field_notes` reads `Lesson::field_notes`; a derived note carries
      the lesson id it came from, which is what the menu card opens.
- [x] Select one fact when a menu/loading surface opens. Do not rotate it while
      visible and do not extend loading duration so it can be read. All three
      surfaces are built once and never reconciled;
      `a_field_note_does_not_hold_the_load_or_take_a_click` asserts the gate and
      the slot's `Pickable::IGNORE`, never a duration.
- [x] Avoid immediate repeats within a session. Facts name actions, not default
      keys. Use destination context where known, but keep a valid general
      fallback. `FieldNoteRotation` is one session resource shared by the menu
      and both screens; `pick_preferred` takes `notes_for_scenario` first and
      falls back to the whole set.
- [x] Make boot facts available before the main asset collection is loaded,
      either as compiled data or in the boot collection. Scenario-load facts may
      use the loaded catalog. `boot_field_notes` is compiled; the scenario
      screen reads the MERGED catalog, so a mod's lesson can come up there.
- [x] Keep each fact to roughly two short lines. A transient loading fact is
      not interactive. A menu field note needs a place to live first: the
      bottom-left corner is a notice slot, so a note is a second notice there.
      `wrap_note_lines` refuses a third line; the corner now holds the offer and
      the note, and answering the offer leaves the note standing. Each notice
      carries its OWN switch: `Don't show again` on the note writes
      `FieldNoteSetting`, the way `Not now` writes `TrainingPromptSetting`, and
      Settings > Interface holds both. Neither touches the loading screens.

## Verification

- Focused unit tests cover catalog validation, stable ordering, progress
  migration/defaults, completion rules, and non-repeating fact selection.
- Menu live-tree tests cover modal open/back, selection, page navigation,
  Viewed versus Completed rendering, current binding resolution, first-pilot
  actions, and narrow containment.
- Add or extend an asserted systems example for the real pointer journey:
  fresh profile -> recommendation -> handbook -> Basic Training -> proven
  completion -> restart -> persisted state. Register its `outcome:` markers in
  `crates/nova_probe_cli/tests/catalog_drift.rs`.
- Run the affected menu/loading ranges. Inspect desktop and narrow captures;
  headless tests do not prove appearance. Validate representative looping media
  on native and wasm.
- Prove the loading fact does not alter either load gate. Never assert timing;
  assert ownership, state, settlement, and teardown.
- Run affected checks only, plus content generation/lint if authored content is
  added. Inspect generated output.

## Documentation

- Update `CHANGELOG.md` under `[Unreleased]` with one joined player-facing entry
  for the handbook, first-pilot guidance, progress, and loading facts.
- Update `web/src/wiki/getting-started.md` for the Training entry and first-run
  route. Update related wiki pages and stable anchors with the shipped lessons.
- Update `docs/architecture.md`, `docs/keeping-docs-in-sync.md`, and the concept
  index for handbook ownership, persistence, authored content, and the relation
  between lessons and the web manual.

## Done when

- The approved Training modal ships from the main menu with reviewed condensed
  lessons, native+wasm visuals, current bindings, progress, and wiki links.
- A fresh player gets a non-blocking route to Basic Training and the handbook;
  the decision and proven progress survive restart without abusing settings.
- Menu and both loading contexts show short non-repeating facts without changing
  load duration or gates.
- Basic Training completion grants only supported lesson completion.
- Affected tests, content lint, UI range, persistence journey, native build, and
  wasm build pass, and rendered output has been inspected.
