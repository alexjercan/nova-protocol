# Notes: Drawer combined Flight Log

- TASK: 20260724-102309
- DATE: 2026-07-25

## What Changed

The drawer left panel now owns one combined `FLIGHT LOG` stream. It appends
`COMMS <speaker> > <text>` rows from `StoryFeed`, `OBJ + <message>` rows when
objectives appear, and `OBJ x <message>` rows when objectives complete. These
rows render in one chronological list with compact terminal-style chrome.

The right drawer `OBJECTIVES` panel now renders only active objectives. Completed
objectives no longer remain as struck-through rows there; the left Flight Log is
the historical record.

`web/src/wiki/hud.md` was updated to describe the two-panel split: left is past
events and comms, right is current objectives.

## Why This Shape

The owner clarified that the left panel should read like server logs, not like
two separate `COMMS` and `FLIGHT LOG` sections. Using one drawer-local log
resource gives that chronological stream and avoids duplicating completed
objectives in the right panel.

Rejected alternatives:

- Raw nova_probe timeline: too debug-shaped and too noisy for players.
- New scenario-authored `FlightLog` action now: useful later, but v1 can be
  built from `StoryFeed` and `GameObjectives` without adding format/lint/docs
  surface area.
- Separate comms and flight-log lists: does not match the desired terminal-log
  feel.

## Difficulties

The main trap was reset aliasing. Objective diffs are useful for posted and
completed rows, but a scenario/drawer clear must not become fake completion
history. The implementation clears `DrawerFlightLog` in `remove_drawer`, and
the tests pin that retained state disappears on teardown.

The other adjustment was unwinding the right-panel completed-objective retention
from the previous drawer task. The new model keeps `GameObjectives` as active
state for the right panel and stores history only in `DrawerFlightLog`.

## Verification

- `nix develop --command cargo test -p nova_gameplay drawer`
- `nix develop --command cargo fmt --check`
- `nix develop --command cargo check`
- `npm run ci` in `web/` after `npm ci`
- `tatr check --ledger LESSONS.md`

## Self-Reflection

The plan improved materially because the owner clarified "single list" before
implementation. The first plan mapped resources to sections too directly. For
future log-style UI tasks, establish whether the desired reader experience is
grouped categories or one chronological stream before locking the plan.
