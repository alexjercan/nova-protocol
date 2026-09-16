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

## Product contract

- Add a main-menu `Training` entry that opens a large normal-menu modal. Do not
  reuse the NOVA OS CRT casing, terminal prompt, or text-input path.
- Use the existing list-and-details screen language: categorized lesson list on
  the left; lesson title, visual, concise pages, bindings and actions on the
  right; progress summary and Back remain visible.
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

- [ ] Add the Training button and full-screen modal to the shipped main menu.
- [ ] Prototype categories, lesson rows, selected/viewed/completed states,
      progress summary, page navigation, a media frame, a live binding chip,
      related-wiki action, Practice action, and Back.
- [ ] Prototype the initial information architecture: Start Here, Flight,
      Combat, Shipbuilding, NOVA OS, and Advanced. The first shipped content set
      may be smaller, but the layout must not assume only three groups.
- [ ] Prototype a non-blocking bottom-left first-pilot recommendation with
      `Start Training`, `Open Lessons`, and dismissal behavior.
- [ ] Prototype one short menu field-note card and one loading-screen fact slot.
- [ ] Prove mouse and keyboard navigation, modal ownership, scrolling, and
      containment at the supported narrow window. Coordinate gamepad behavior
      with `20260714-001140`; do not create a second navigation policy.
- [ ] Capture the main list, one visual lesson, first-pilot card, and loading
      fact at desktop and narrow widths. Inspect the rendered frames.

STOP after the prototype. Record the captures and get owner approval on the
screen name, layout, information density, progress treatment, visual-media
presentation, and first-pilot card before Phase 2. UI approval is a real gate,
not an implementation checkpoint to pass mechanically.

## Phase 2 - authored lessons and media

Start only after the Phase 1 owner review is recorded on this task.

- [ ] Define stable `LessonId`, category, title, summary, ordered pages,
      related-wiki path, optional practice target, fact references, and explicit
      progress rule. Unknown IDs and missing required fields fail at lint, then
      load.
- [ ] Keep the page vocabulary narrow: concise text, image, chosen looping-media
      form, binding/action reference, and a small diagram form only if a real
      lesson needs it. Do not embed general Markdown or HTML.
- [ ] Decide the content owner and asset pipeline before implementation. Keep
      lesson content data-driven without hand-editing generated base content.
- [ ] Author and review the first release lesson set. At minimum cover Start
      Here, manual flight and momentum, STOP, GOTO, ORBIT, travel versus combat
      locks, turrets, and torpedoes. Add Shipbuilding, NOVA OS, and Advanced
      lessons only where concise reviewed text and useful visuals are ready; do
      not ship filler to populate a category.
- [ ] Give lessons current binding chips and related links into the existing
      player wiki. Update wiki anchors when needed and keep the wiki complete.
- [ ] Add representative visuals: at least one flight loop, one targeting or
      combat loop, and one editor-style visual. Validate all media on native and
      wasm.
- [ ] Wire `Start Training` and applicable Practice actions through the existing
      New Game/scenario handoff. Basic Training is the base bundle's declared
      `tutorial`; do not add a competing launch path.

## Phase 3 - progress and first-player recommendation

- [ ] Persist explicit viewed lesson IDs, completed lesson IDs, and first-pilot
      dismissal/progress. Do not infer first use from settings-file existence,
      and do not make player progress part of `PersistedSettings` without an
      explicit ownership decision.
- [ ] A fresh profile sees the recommendation. `Start Training` launches Basic
      Training; `Open Lessons` opens Start Here. Dismissal survives restart.
- [ ] Completing Basic Training marks only the lessons its asserted flow proves.
      A failed, abandoned, or merely opened scenario does not grant completion.
- [ ] Old, missing, extra, or corrupt progress data fails safely to conservative
      defaults. Scripted runs use isolated or inert storage and never touch the
      developer's profile.

## Phase 4 - menu and loading facts

- [ ] Derive field notes from the same authored learning catalog or stable
      lesson IDs. Do not maintain unrelated copies of the same claim.
- [ ] Select one fact when a menu/loading surface opens. Do not rotate it while
      visible and do not extend loading duration so it can be read.
- [ ] Avoid immediate repeats within a session. Facts name actions, not default
      keys. Use destination context where known, but keep a valid general
      fallback.
- [ ] Make boot facts available before the main asset collection is loaded,
      either as compiled data or in the boot collection. Scenario-load facts may
      use the loaded catalog.
- [ ] Keep each fact to roughly two short lines. A menu field note may open its
      related lesson; a transient loading fact is not interactive.

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
