# Editor event surfacing: a scenario's beats without drowning the panel

- STATUS: OPEN
- PRIORITY: 64
- TAGS: v0.12.0,editor,scenario,events

Successor to `20260714-081703` slice 3. **Blocked on `20260820-223059`
(Sequence).** This task exists because of that one: surfacing the handler list
as it stands today means drawing nineteen rows where the author wrote one story
beat, and that is the wrong panel to build.

## Goal

Expose a scenario's events and handlers in the editor without drowning the
panel. An author should see the beats of their scenario, open one, and see what
it does.

## What the shape depends on

- `Sequence` is the primitive that makes a beat one handler instead of many
  (`20260820-223059`). The panel's row is a beat, not a handler, only if that
  lands first.
- Handlers the editor cannot represent re-lift as OPAQUE and must survive a
  round-trip untouched - the convention settled in the save/load task
  (`20260825-223004`). The panel has to show them as opaque without pretending
  it can edit them.
- The scenario node's inspector is the host. Step 12 of the polish plan in
  `20260825-221015` fills that panel with ships, objects and the player's ship;
  events are the next tenant.

## Done when

- The sandbox range's own handlers read as beats in the scenario node's
  inspector, opaque ones marked as opaque.
- Opening and closing the panel round-trips a hand-written mod byte-for-byte.
- A UI-harness walk covers it; probe green.
