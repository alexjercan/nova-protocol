# Decision: Build drawer-local styled objective rows

- DATE: 20260725-145157
- STATUS: ACCEPTED
- TASK: 20260724-134350
- TAGS: decision, ui, hud, drawer, objectives

## Context

The drawer right panel already owns `DrawerObjectivesListMarker` and rebuilds it
from `GameObjectives`. The generic `bevy_common_systems` objectives panel also
reads `GameObjectives`, but its contract is a simple objective text panel: one
text line per objective. The playtest request is specifically to make the
drawer's right panel a styled list that matches Nova's drawer chrome.

## Decision

Build the objective list as local Nova drawer UI inside
`crates/nova_gameplay/src/hud/drawer.rs`: a row node per active objective, with
drawer/theme styling, a bullet or status glyph, and message text descendants.
Keep `GameObjectives` as the data source.

## Alternatives considered

- **Reuse the generic bcs objectives panel** - rejected because it renders plain
  objective lines and has no drawer-specific row chrome or glyph language.
- **Move a styled list widget into `nova_ui` now** - rejected because this is the
  first concrete drawer objective list; extracting a shared widget before the
  left panel and later drawer sections exist would make the abstraction guessy.
- **Only change text color and spacing** - rejected because it would leave the
  placeholder artifact shape intact instead of producing a real list.

## Consequences

The task stays tightly scoped to the drawer and can match the existing panel
layout precisely. If later drawer sections need the same row language, a follow-up
can extract the repeated row/chrome helpers from the proven local implementation.

## Amendment 20260725-150000: Derive completed rows in the drawer

### Context

The owner asked to keep completed objectives in the drawer as a log with
line-through treatment. `GameObjectives` only stores active objectives;
completion is represented by an objective disappearing from the active list.
The existing `objective_feedback` system already uses that diff for transient
completion ghosts and treats an empty objective list as scenario teardown, not
success.

### Decision

Keep completion history drawer-local: add a small objective-log resource or
state in `hud/drawer.rs` that updates from `GameObjectives` diffs, marks removed
objectives completed, preserves their messages/order for the current scenario,
and clears on teardown. Render completed entries as muted rows with a done glyph
and a thin line-through overlay node across the text area.

### Alternatives considered

- **Extend `bevy_common_systems::GameObjectives`** - rejected because the flight
  objective hint and generic objectives panel use it as active-objective state;
  completed history is drawer behavior, not the shared active-objective model.
- **Store completion history in `NovaEventWorld`** - rejected for this task
  because the drawer can derive the needed history from the same active-list diff
  already used by HUD feedback, without making scenario state depend on a drawer
  presentation requirement.
- **Use native text decoration** - rejected because no Bevy UI
  `TextDecoration`/line-through component exists in the installed Bevy source.

### Consequences

The drawer can become a compact objective log without changing scenario content
or shared bcs data types. The tradeoff is that completion history is derived UI
state, so the implementation must explicitly clear it on teardown to avoid
showing failed or abandoned objectives as completed.
