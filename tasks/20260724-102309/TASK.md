# Drawer comms-log section: full StoryFeed as scrollable speaker-grouped transcript

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.9.0,spike,feature,ui,hud

## Goal

The drawer's COMMS LOG section: render the full conversation transcript inside
the Tab drawer. Design fixed by Spike: tasks/20260721-211512/SPIKE.md.

Scope (direction-level; /plan breaks into steps at pickup):

- Render the full StoryFeed (crates/nova_gameplay/src/hud/comms_panel.rs:57 -
  the append-only log ALREADY EXISTS) as a scrollable, speaker-grouped
  transcript inside the drawer's Comms Log section (bevy_ui scroll).
- Reuse the per-speaker icons introduced by 20260721-211526 (the comms stack);
  the in-flight stack and this log are two views of the same StoryFeed.

## Notes

- Spike: tasks/20260721-211512/SPIKE.md (RECOMMENDED).
- Depends on the drawer shell (section framework) and rides 20260721-211526
  (speaker icons). Cast speakers: crates/nova_assets/src/scenario/cast.rs.
