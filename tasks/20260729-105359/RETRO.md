# RETRO - Menus + editor spawn the shared nova_ui widget factories

- TASK: 20260729-105359 (follow-up to 20260728-175738)
- DATE: 2026-07-29
- OUTCOME: shipped; review round 1; DoD 5 (owner in-engine eyeball) pending.

## What shipped

The menu + editor screens now spawn the shared `nova_ui::widget` factories
instead of bespoke `Node` trees: settings (segmented + block-meter slider),
mods/scenarios (list_row + checkbox + badge), all 7 modal containers (panel),
editor rail (panel) + "soon" chip (badge). Plus the reusable nova_ui machinery
that made it possible: `segmented_container`/`segmented_option`, an interactive
`ListRow` reconciler, `sync_slider_meters`, `checkbox_colors`, and a
paint-decorator `panel()`.

## What went well

- Factoring the paint out of the widgets (`list_row_colors`, `checkbox_colors`,
  `slider_meter_color`) meant the factory AND the in-place reconciler/sync
  systems paint from ONE source - so a mods checkbox toggled in place can never
  drift from what `checkbox()` would render fresh.
- The `ListRow` reconciler mirrors the button one exactly (observers for
  hover/selection + an `Added` override system), so the mods/scenarios rows got
  live hover + selection highlight for free while looking like the zoo's rows.
- Running the WHOLE nova_menu suite after each screen (not a filtered test)
  caught every behavior break early - the checkbox entity-holding, the scenarios
  tree-reading, the settings thumb assertion - instead of at land.

## What went wrong / difficulties

- The recurring blocker was FACTORY COMPOSITION: a factory that bakes its own
  `Node` (panel, slider_track) cannot be tuple-spawned onto an entity that also
  needs a `Node` (a 85%-sized modal, a flex-grow slider cell) - Bevy panics on
  the duplicate `Node`. Two fixes emerged: (a) make the factory a PAINT
  DECORATOR with no Node (`panel()` now returns only the colour/shadow/gradient
  components; the caller owns the sized Node), and (b) WRAP (the volume slider
  sits in a flex-grow cell; `slider_track` fills it at width:100%). Decorator is
  the cleaner default for anything that must be sized by the caller.
- Rebuild-on-toggle vs in-place update: my first mods-checkbox pass rebuilt the
  whole row on `EnabledMods` change. That respawned the checkbox entity, which
  broke the test that holds the entity across the toggle (and churned the list).
  In-place update (repaint the SAME entity via `checkbox_colors`) is both
  correct and test-friendly. Lesson: prefer in-place restyle over respawn for a
  widget whose identity other code (or a test) holds.
- The scenario-row indent needed a wrapper Node, which broke the tests that read
  `ScenariosList` DIRECT children + a row's first-child Text. Dropped the wrapper
  (lost the 16px member indent; collapsible headers still show hierarchy) and
  made the test `label_of` recurse to the first Text descendant.

## Lessons (for the ledger)

- `factory-that-bakes-a-node-cant-be-sized-by-caller` (domain): a widget factory
  returning a bundle WITH a `Node` cannot be tuple-spawned onto an entity that
  also needs its own `Node` (size/layout) - Bevy panics "duplicate components:
  Node". Make size-flexible factories PAINT DECORATORS (return the visual
  components, no Node; caller supplies a Node with `border`+`border_radius`), or
  wrap them in a sizing parent. `panel()` became a decorator + `panel_node()`
  for the plain case. 20260729-105359.
- `paint-from-one-source-for-factory-plus-reconciler` (positive): when a widget
  is both spawned by a factory AND restyled live (a reconciler, an in-place sync
  system), factor its colours into ONE pure `(state) -> colours` fn both call
  (`list_row_colors`, `checkbox_colors`, `slider_meter_color`), so the two paths
  cannot diverge. 20260729-105359.
- `restyle-in-place-not-respawn-for-held-identity` (x1): to update an
  interactive widget's visual on a state change, repaint the SAME entity rather
  than despawn+respawn its row - a respawn breaks any code/test holding the
  entity and churns siblings (scroll/selection). 20260729-105359.

## Follow-ups

- The explore-tab mod row (`spawn_explore_row`) still builds a bespoke row; it
  could adopt `list_row` like `spawn_mod_row` in a small follow-up.
- Owner: in-engine eyeball of settings/mods/scenarios/pause/editor in both skins
  (DoD 5) - the screens now spawn the same factories the widget_zoo verifies.
- The member-row indent was dropped; if the owner wants it back, add it via a
  wrapper + teach the display test to descend.
