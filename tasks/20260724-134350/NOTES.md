# Notes: Drawer objective log rows

## What Changed

The drawer right panel now builds objective rows from a drawer-local
`DrawerObjectiveLog` resource instead of spawning direct text children. Active
rows use drawer chrome, an objective accent glyph, borders and row padding.
Completed objectives remain in the list as muted log rows with a done glyph and
a thin overlay line-through across the text area.

The log is derived from `GameObjectives`: active entries upsert as active rows,
objectives removed from the previous non-empty active list become completed
rows, and an empty `GameObjectives` list clears the log as scenario teardown.

## Why

`GameObjectives` is the shared active-objective model used by the flight hint,
feedback and generic objectives panel. Extending it with completed history would
leak a drawer presentation requirement into shared state. A drawer-local derived
log keeps the shared model active-only while still giving the paused drawer a
mission-log feel.

Bevy UI in this tree does not expose a native line-through text decoration, so
the completed row uses a 1px absolute overlay node in the text wrapper instead
of trying to encode decoration on `Text`.

## Alternatives

- Reusing the generic bcs objectives panel was rejected because it renders plain
  text lines and cannot carry Nova drawer row chrome.
- Storing completion history in `NovaEventWorld` was rejected because the drawer
  can derive the history from the same active-list diff already used by
  objective feedback.
- Native text decoration was checked in local Bevy sources and not found.

## Difficulties

The first drawer test compile failed because a helper still had a `#[test]`
attribute and two child-iteration sites passed `&Entity` where Bevy expected
`Entity`. The implementation itself then passed the drawer test filter.

## Reflection

The useful move was checking the existing completion mechanism before coding:
completion is removal from `GameObjectives`, and `objective_feedback` already
defines the teardown exception. Next time, do that source check before drafting
the first plan so owner amendments that affect data shape start from the real
state model.
