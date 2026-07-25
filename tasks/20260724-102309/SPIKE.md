# Spike: what belongs in the Flight Logs?

- DATE: 20260725-155659
- STATUS: RECOMMENDED
- TAGS: spike, ui, hud, drawer, comms

## Question

The owner likes the left drawer panel and wants to decide what "Flight Logs"
should contain before implementation. A good answer chooses a first shipping
scope that feels useful in game, avoids a noisy debug dump, and can be built
from current scenario/HUD state without inventing a half-designed authoring
surface.

## Context

The drawer shell already exists in `crates/nova_gameplay/src/hud/drawer.rs`:
the right panel shows objectives, while the left panel has a `COMMS / LOG`
placeholder with `FLIGHT LOG` and `No messages yet.`. The full comms transcript
already exists as `StoryFeed` in `hud/comms_panel.rs`; `nova_scenario` mirrors
scenario `StoryMessage` actions into it and clears it at teardown.
`GameObjectives` is also mirrored write-on-diff from scenario state. The
objective drawer already keeps completed objectives as a retained log, and the
lessons ledger warns that state diffs can mistake reset/teardown for events.
Owner adjustment on 2026-07-25: if the Flight Log records objective received and
completed events, the right drawer panel should stop retaining completed
objectives. Left panel becomes past things plus journal; right panel becomes
current objectives only.
Owner adjustment on 2026-07-25: the left panel should be one combined list, like
server logs, not separate `COMMS` and `FLIGHT LOG` sections.

## Options considered

- **Raw nova_probe-style event stream** - Mirror low-level run events, variables,
  contacts, physics and outcome details into the drawer. This matches the source
  inspiration but is too noisy for a paused cockpit screen and would expose debug
  concepts to players.
- **Scenario-authored FlightLog action** - Add a new scenario action so content
  authors explicitly post journal beats. This gives authors maximum control, but
  it adds data format/docs/lint work before the UI proves the shape. It also
  duplicates many beats already expressed as comms and objectives.
- **Curated derived cockpit log** - Render `StoryFeed` as the comms transcript
  and derive mission rows from objective appearances/completions. This uses
  existing player-facing state, gives the left panel immediate value, and keeps
  future explicit log actions additive.
- **Comms transcript only** - Ship the original drawer comms-log task and leave
  Flight Log empty. This is easy but no longer matches the 2026-07-24 playtest
  rescope.

## Recommendation

Ship Flight Logs v1 as a curated cockpit record:

- `COMMS` shows the full `StoryFeed` transcript, in delivery order, with speaker
  labels and the same icon/fallback semantics as the in-flight comms cards.
- `FLIGHT LOG` shows terse mission/system rows for objective posted and
  objective completed. Suggested row text: `OBJ + <message>` for a new objective
  and `OBJ x <message>` for completion.
- The left panel renders those comms and objective rows in one chronological
  terminal/server-log stream, not as two separate lists.
- The right `OBJECTIVES` panel shows only active objectives. Completed
  objectives leave that panel and are retained only in the left Flight Log.

Do not add a new scenario action or raw event bus in this task. The current game
already has enough player-facing signals for a useful log, and the drawer can
later accept explicit `FlightLog` entries if content needs beats that are not
comms or objectives.

The main implementation risk is reset aliasing: objective diffs must not treat a
scenario clear or drawer teardown as a real completion burst. The plan should
pin that behavior with tests.

## Open questions

- Should Flight Log rows eventually include a mission timestamp? Defer for v1.
  There is no stable always-visible scenario-clock contract for every scenario,
  and timestamps are less important than getting the event content right.
- Should authored key beats/outcomes get explicit log rows? Defer until a
  scenario-authoring task can add the action, lint, docs, and examples together.
- Should combat events such as target destroyed or damage critical enter the
  log? Defer until the ship-status/damage drawer work defines the severity
  vocabulary.

## Next steps

- tatr 20260724-102309: plan and build the left drawer `COMMS` transcript plus
  curated objective-derived `FLIGHT LOG` rows as one combined stream, and make
  the right drawer `OBJECTIVES` panel current-only.
