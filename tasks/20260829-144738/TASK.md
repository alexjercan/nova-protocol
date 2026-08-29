# Events editor: one page per expression, names, pickers, tooltips

- STATUS: CLOSED
- PRIORITY: 62
- TAGS: v0.12.0, editor, ui, events

Successor to `20260829-133038`, from the owner's read of the shipped Events
mode. The mode itself is right; what it draws is not finished.

## The expression AST leaves the hierarchy

The AST stays - the owner reads it as the cleaner shape - but it is not a
hierarchy concern. The rail shows ONE row for the expression filter; selecting
it opens the expression editor as a PAGE: the whole tree, indented, each
operator and each leaf editable in place. `InspectorField` is already
node-addressed, so a page of rows can write to several document nodes.

A variable is an IDENTIFIER, not a value. A leaf naming one picks from the
variables in scope the way the VariableSet row does, rather than being typed.

## Handlers get a label

`ScenarioEventConfig.name` is the trigger, so a title is a new optional field.
The tree row reads BOTH: `OnEnter - picket warden wakes`.

## The rest of the surface

- The Add menu splits by view, like Ship does, with names that say what they
  make. `Sequence` is the hard one to figure out and should read like what it
  is.
- A choice with many options is a DROPDOWN, not a wrapping wall of chips - the
  handler trigger has sixteen.
- Every field holding a path gets a pick, and says whether the path resolves.
  Assets belong on the Events screen; drag-and-drop is the shape to aim at.
- TOOLTIPS on most things. The Events screen has to explain what a field means
  without the docs open beside it.
- Every textbox aligned with its unit; every label earning its place. A row
  reading `turret` and nothing else says nothing.

## Done when

- An expression is one row in the tree and one page in the editor, with its
  operators emphasised and its identifiers picked rather than typed.
- A handler can be named, and the tree reads the trigger and the name.
- Add offers what the current view can make, under names that need no glossary.
- A long choice is a dropdown; a path field picks and validates.
- Resting on a field in Events says what it is for.
- Units, boxes and labels line up, and no label is a bare word with no subject.

## What shipped

- The expression AST is ONE PAGE. The rail shows a single `Expression` row for
  the filter; selecting it draws the whole tree in the panel, a row per node,
  each writing to its own entity through `InspectorRow::owner`. A leaf naming a
  variable picks from the variables in scope instead of being typed.
- `ScenarioEventConfig` grew an optional `label`. The tree row reads both -
  `On Enter - picket warden wakes` - and the panel's first row is the label.
- Add splits by view (`AddPalette::World` / `AddPalette::Script`): Scene offers
  ships, rocks, beacons, salvage and lights; Events offers Handler, Filter,
  Action, Sequence, Step and Gate, each under the glyph the tree draws it with.
- A choice of seven options or more is a picker WINDOW, not a wall of chips,
  and each option carries the doc sentence of the variant it stands on.
- Every row says what it is for on hover, out of the config author's own doc
  comment (`bevy/reflect_documentation`). No second list of hints exists.
- Units stand in a column of their own (`UNIT_W`), so boxes, chips and
  checkboxes down the panel end on one line.
- A row holding an `AssetRef<A>` picks its file from what the installed
  bundles DECLARE, written as the `dep://` ref that resolves (with `#Scene0`
  where a mesh needs it), and reads `unknown` beside a path no bundle ships.

## Proof

- `cargo test -p nova_editor --lib`: 405 passed, 0 failed.
- New behaviour tests: `a_row_explains_itself_from_the_config_that_declares_it`,
  `a_choice_explains_the_option_it_is_on`, `a_long_choice_lists_what_each_option_does`,
  `hovering_a_row_reveals_its_whole_name_and_what_it_is_for`,
  `the_hint_takes_the_side_of_the_row_that_has_room`,
  `the_inspector_hint_goes_away_with_the_pointer`,
  `add_shows_only_the_palette_the_mode_can_use`,
  `a_row_that_names_a_file_says_what_kind_of_file`,
  `a_file_row_offers_the_bundles_files_and_marks_one_they_do_not_ship`,
  `every_span_the_panel_spawns_takes_the_editor_typeface`, and the five
  `asset_index` tests.
- `cargo run --example screenshot_editor --features debug` under Xvfb :99:
  cycle complete, no panic, every shot written. The captures show the handler
  panel (Label / Trigger / Once), the Trigger picker listing all sixteen events
  each with its own sentence, and the hint panel beside the row it is on.

## Found on the way

The whole editor rendered in bevy's BUILT-IN font: `nova_ui`'s font router only
touches spans marked `UiText`, and `nova_editor` marked a third of them. That is
what drew the picker chips as empty boxes. Every span in the editor is marked
now, and `every_span_the_panel_spawns_takes_the_editor_typeface` holds the
panel to it. `nova_hud` has the same gap and is NOT fixed here.

## Not done

Drag-and-drop of a file onto a row. The picker is the shape that shipped; a
drop target needs a window file-drop path the editor has none of yet.
