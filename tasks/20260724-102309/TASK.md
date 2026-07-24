# Drawer LEFT panel: comms/chat history + flight-log events journal (slides from left)

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.9.0,spike,feature,ui,hud

## Goal

RESCOPED by the 2026-07-24 playtest (owner): the drawer's LEFT panel. It slides in
from the LEFT and holds the CHAT HISTORY plus a lightweight EVENTS journal - "like
the event log from nova_probe but in game, not as detailed, just important things,
a flight-log journal". Design origin: Spike tasks/20260721-211512/SPIKE.md (the
comms-log view), now expanded to a left-side log+events panel.

Scope (direction-level; /plan breaks into steps at pickup):

- LEFT panel of the drawer, slides in from the left (the shell 20260724-102304
  slides the right panel; the left-panel slide comes from the drawer-open rework
  20260724-134335).
- Chat history: render the full StoryFeed (crates/nova_gameplay/src/hud/
  comms_panel.rs:57 - append-only log ALREADY EXISTS) as a scrollable,
  speaker-grouped transcript; reuse 20260721-211526's per-speaker icons.
- Flight-log EVENTS: a curated, in-game journal of IMPORTANT scenario events
  (objectives posted/completed, key beats, outcomes) - the nova_probe timeline is
  the inspiration, but only the player-meaningful events, terse. Decide the event
  source (scenario events / existing markers) at plan time.
- MUST NOT overlap the lower-left keybind hint cluster (hud/keybind_hints.rs,
  bottom:8 left:8) - keep the keys visible (per the 2026-07-24 feedback).

## Notes

- Spike: tasks/20260721-211512/SPIKE.md (RECOMMENDED). RESCOPED from "comms-log
  section" to the left panel (log + events) by the 2026-07-24 playtest.
- Depends on the drawer shell (20260724-102304, section framework) and the
  drawer-open rework (20260724-134335, left-panel slide + keeping keys clear);
  rides 20260721-211526 (speaker icons). Cast: crates/nova_assets/src/scenario/cast.rs.
