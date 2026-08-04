# Drawer LEFT panel: comms/chat history + flight-log events journal (slides from left)

- PRIORITY: 50
- TAGS: v0.9.0, spike, feature, ui, hud
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Goal

RESCOPED by the 2026-07-24 playtest (owner): the drawer's LEFT panel. It slides in
from the LEFT and holds the CHAT HISTORY plus a lightweight EVENTS journal - "like
the event log from nova_probe but in game, not as detailed, just important things,
a flight-log journal". Design origin: Spike tasks/20260721-211512/SPIKE.md (the
comms-log view), now expanded to a left-side log+events panel.

## Story

As a player opening the ship-computer drawer, I want the left panel to show one
combined terminal-style event stream, so comms lines and mission events read
like a server log of what happened in order.

As a player scanning the right drawer panel, I want that side to show only what
is currently active, so the drawer has a clear split: past things and journal on
the left, current objectives on the right.

## Flight Log Scope

Recommended by `SPIKE.md` and recorded in `DECISION.md`: Flight Logs v1 are a
curated cockpit record, not a raw probe/debug timeline. The left drawer panel
contains one combined list:

- `COMMS` rows from the full `StoryFeed` transcript, with speaker labels and
  existing/fallback speaker icons.
- `FLIGHT LOG` rows for objective-posted and objective-completed events, derived
  from `GameObjectives` changes and cleared on scenario/drawer teardown.
- Rows render in one chronological stream, server-log style, rather than as
  separate `COMMS` and `FLIGHT LOG` lists.
- Right `OBJECTIVES` panel - current objectives only. Completed objectives move
  to the left `FLIGHT LOG` and no longer remain as struck-through right-panel
  rows.

Defer a scenario-authored `FlightLog` action and raw combat/physics event
capture until a later task has clearer authoring and noise rules.

## Steps

- LEFT panel of the drawer, slides in from the left (the shell 20260724-102304
  slides the right panel; the left-panel slide comes from the drawer-open rework
  20260724-134335).
- [x] Replace the left-panel placeholder in
  `crates/nova_gameplay/src/hud/drawer.rs` with a single styled `FLIGHT LOG`
  stream container and empty state.
- [x] Add a drawer-local combined log resource with row kinds for comms,
  objective posted, and objective completed.
- [x] Append `COMMS <speaker> > <text>` rows from newly observed `StoryFeed`
  entries, preserving transcript order and the same authored/fallback icon
  semantics as the in-flight comms cards.
- [x] Append `OBJ + <message>` when an objective first appears and
  `OBJ x <message>` when it completes; avoid duplicate rows when an active
  objective updates its text.
- [x] Render all left-panel rows in one chronological stream, newest at the
  bottom, with compact terminal/server-log styling that distinguishes comms rows
  from objective rows without splitting them into separate lists.
- [x] Rework the right drawer objectives section back to active-current state:
  remove completed-objective retention/strike-through rows from the right panel
  and show the styled empty state when no objectives are currently active.
- [x] Clear the left-panel comms/log render state on drawer teardown and guard
  empty objective reset so scenario clear does not manufacture completion rows
  (`state-diff-aliases-reset`).
- [x] Add focused drawer tests for combined chronological log rendering, comms
  row append, objective-posted/completed row append, no duplicate entry on
  objective text update, right-panel current-only behavior, and teardown/clear
  behavior.
- [x] Update player-facing HUD docs so the drawer description names the left
  `COMMS` and `FLIGHT LOG` sections accurately.
- [x] Write `tasks/20260724-102309/NOTES.md` with what changed, why this v1
  source was chosen, difficulties, and self-reflection.
- MUST NOT overlap the lower-left keybind hint cluster (hud/keybind_hints.rs,
  bottom:8 left:8) - keep the keys visible (per the 2026-07-24 feedback).

## Definition of Done

- The left drawer panel no longer shows `No messages yet.` and instead has one
  styled `FLIGHT LOG` stream container (test:
  `drawer_left_panel_has_combined_flight_log_stream`).
- Story messages retained in `StoryFeed` append comms rows to the same stream as
  objective events, with speaker attribution and icon/fallback structure (test:
  `drawer_combined_log_renders_story_feed_rows`).
- Objective appearances and completions append terse Flight Log rows in order,
  while an in-place objective text update edits the active row without creating a
  duplicate event (test:
  `drawer_combined_log_records_objective_events_once`).
- Comms rows and objective rows appear in the same list in observation order
  instead of in separate sections (test:
  `drawer_combined_log_interleaves_comms_and_objective_rows`).
- The right drawer `OBJECTIVES` section shows only active objectives; completing
  an objective removes it from the right panel instead of retaining a
  struck-through duplicate of the left-panel history row (test:
  `drawer_right_panel_shows_only_active_objectives`).
- When the final objective completes, the right panel returns to its styled
  no-active-objectives empty state while the left Flight Log keeps the
  completion row (test: `drawer_final_objective_moves_to_flight_log_only`).
- Scenario/drawer teardown clears left-panel retained state instead of turning a
  reset into completion events (test:
  `drawer_flight_log_clears_on_drawer_teardown`).
- The player HUD docs describe the left drawer comms/log behavior and no longer
  promise that the comms log will land later (cmd:
  `grep -ni "flight log" web/src/wiki/hud.md`).
- `tasks/20260724-102309/SPIKE.md`, `DECISION.md`, and `NOTES.md` exist with the
  design and fix record (cmd: `test -f tasks/20260724-102309/SPIKE.md && test -f tasks/20260724-102309/DECISION.md && test -f tasks/20260724-102309/NOTES.md`).
- Overall verification is clean for the touched surface (cmd:
  `nix develop --command cargo test -p nova_gameplay drawer`; cmd:
  `nix develop --command cargo fmt --check`; cmd:
  `nix develop --command cargo check`; cmd: `tatr check --ledger LESSONS.md`).
- manual: in a real scenario, opening Tab shows the left drawer above the
  lower-left keybind hints; recent comms and objective events read as one
  compact terminal/server-style log rather than two separate lists; the right
  drawer panel reads as current work only.

## Notes

- Spike: tasks/20260721-211512/SPIKE.md (RECOMMENDED). RESCOPED from "comms-log
  section" to the left panel (log + events) by the 2026-07-24 playtest.
- Local spike: tasks/20260724-102309/SPIKE.md. Decision:
  tasks/20260724-102309/DECISION.md.
- Depends on the drawer shell (20260724-102304, section framework) and the
  drawer-open rework (20260724-134335, left-panel slide + keeping keys clear);
  rides 20260721-211526 (speaker icons). Cast: crates/nova_assets/src/scenario/cast.rs.
- Grounded facts verified 2026-07-25: left drawer placeholder is in
  `crates/nova_gameplay/src/hud/drawer.rs`; `StoryFeed` is the scenario-scoped,
  append-only comms transcript in `crates/nova_gameplay/src/hud/comms_panel.rs`;
  `NovaEventWorld::state_to_world_system` mirrors story messages and objectives
  into HUD resources in `crates/nova_scenario/src/world.rs`; objective diff
  state must guard teardown/reset per `state-diff-aliases-reset`.
- Owner adjustment 2026-07-25: because Flight Log now records received and
  completed objectives, the right panel must not duplicate completed objectives.
  Left panel = past things + journal; right panel = current things only.
- Owner adjustment 2026-07-25: the left panel should use one combined list, like
  server logs, with comms and objective journal rows interleaved rather than
  separate `COMMS` and `FLIGHT LOG` sections.

## Close-out (2026-07-25)

Shipped the left drawer as one combined `FLIGHT LOG` stream: comms transcript
rows from `StoryFeed`, objective received rows, and objective completed rows
all append into the same compact log. The right drawer panel now renders only
the current active objectives; completion history moved to the left stream.

Why: the owner wanted the left panel to feel like server logs rather than two
separate panes, and once completion events live there, struck-through completed
objectives on the right duplicated the same information. A new scenario-authored
`FlightLog` action was deferred because the current player-facing feeds already
cover the useful v1 events without adding format/lint/docs surface area.

Difficulties: the recent right-panel completed-objective retention had to be
backed out without losing the final-completion behavior. The fix splits the
state model: `GameObjectives` drives the right current-only list directly, while
`DrawerFlightLog` owns the historical stream and clears on drawer teardown.
Tests cover objective text updates, final completion, current-only right rows,
and comms/objective interleaving.

Self-reflection: the initial plan treated `COMMS` and `FLIGHT LOG` as separate
sections because that was the easiest mapping from existing resources. The owner
clarified the desired shape before work started, which saved a rework cycle.
Next time, for a "log" UI, ask whether the user means grouped categories or one
chronological stream before writing the first implementation plan.
