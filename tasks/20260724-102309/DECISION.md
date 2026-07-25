# Decision: One combined log stream on the left, current objectives on the right

- DATE: 20260725-155659
- STATUS: ACCEPTED
- TASK: 20260724-102309
- TAGS: decision, ui, hud, drawer, comms

## Context

The 2026-07-24 playtest rescoped the left drawer from a comms transcript into a
chat history plus lightweight events journal, inspired by nova_probe but
player-facing. Existing code already provides `StoryFeed` for the full comms
transcript and `GameObjectives` for active objective state. Adding raw probe
events or a new scenario-authored log action would change the data model before
the UI has proven what is useful.

On 2026-07-25 the owner accepted objective-posted and objective-completed rows in
the Flight Log, then pointed out that keeping completed objectives struck through
on the right panel duplicates the same information. The drawer needs one clear
split: past things and journal on the left, current things on the right.

The owner then clarified that `COMMS` and `FLIGHT LOG` should not be separate
lists. The desired shape is one combined stream, like server logs, where comms
rows and mission rows appear together chronologically.

## Decision

Build Flight Logs v1 from existing player-facing feeds as one combined left-panel
stream. Append comms rows from `StoryFeed`, and derive objective posted/completed
rows from `GameObjectives` transitions. Render those row kinds in one
chronological list, server-log style, rather than as separate `COMMS` and
`FLIGHT LOG` sections. The right drawer `OBJECTIVES` panel shows active
objectives only; completed objectives leave the right panel and live in the left
Flight Log history instead. Clear the retained drawer log on teardown, and guard
objective reset so a scenario clear is not logged as a completion burst.

## Alternatives considered

- **Raw nova_probe timeline in game** - rejected because it would be noisy,
  debug-shaped, and likely expose engine/scenario internals instead of cockpit
  history.
- **New scenario-authored `FlightLog` action now** - rejected for v1 because it
  needs data format, lint, docs, and authoring examples before there is evidence
  the row shape is right.
- **Comms transcript only** - rejected because the active task was explicitly
  rescoped to include a flight-log events journal.
- **Keep completed objectives on the right too** - rejected because it duplicates
  the same completion in two places and blurs the drawer's information model.
  The right side should answer "what now?", while the left side answers "what
  happened?".
- **Separate `COMMS` and `FLIGHT LOG` lists** - rejected because the owner wants
  the left panel to feel like a single terminal/server log where messages and
  mission events are part of one stream.

## Consequences

This ships a useful left drawer with little new scenario surface area and keeps
future explicit log entries additive. It also reverses the recently added
right-panel completed-row retention because completion history now has a better
home. The combined-stream model means row ordering matters and the implementation
needs a real append-only drawer log rather than two independent render passes.
The tradeoff is that v1 only logs events already reflected by objectives and
comms; authored beats, combat events, timestamps, and damage/system severity
wait for later tasks.
