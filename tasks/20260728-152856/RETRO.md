# RETRO: ship view label table

## What went well

- The plan-gate `AskUserQuestion` with three concrete ASCII table mockups made
  the one real fork (layout + where status goes) a 10-second decision instead of
  a paragraph of prose. The user picked "Full", and that preview WAS the spec -
  the implemented rows match it byte-for-byte.
- The wiring/pure split was pinned by two tests at the right altitudes: a pure
  `terminal_ship_rows` test for the formatting/alignment, and a live
  `ship view` submit test that spawns a real `SectionCode` and asserts it reaches
  the scrollback - the latter fails if the ECS query stops fetching the code
  (`test-the-wiring-system-not-just-its-pure-helpers`). The codeless turret in
  the same test exercises the fallback path for free.
- The alignment assertion (`row.find(code) == header.find("LABEL")`) checks the
  columns line up WITHOUT hardcoding the impl's column widths, so it survives a
  width tweak but still fails on a real misalignment.

## What went wrong / what to improve

- Two small plan deviations, both benign: (1) the Design said to
  `use super::nova_os_ship::SectionCode`, but it already resolved via the prelude
  glob - no import needed. (2) The plan kept the `name` field "for now"; once the
  formatter and sort stopped reading it, `name` was dead code (a `dead_code`
  warning), so I removed the field AND its `Option<&Name>` ECS fetch and switched
  the sort to the `code` label. Lesson: a "carry the old field alongside" plan
  step is worth a second look - if nothing reads it after the change, it is dead
  on arrival and the honest move is to remove it (and its now-unused query fetch),
  not leave a warning.

## Lessons

- `drop-the-field-the-change-orphans` (new): when a formatting/display change
  stops reading a struct field (here `ShipSectionStatus.name` after switching the
  row to the code label + sorting by code), the field AND its upstream ECS query
  fetch (`Option<&Name>`) go dead the same change - remove them in the same pass
  rather than leaving a `dead_code` warning or a wasted query column. A plan that
  says "keep the old field for now" should be checked against "does anything still
  read it after this change?".
