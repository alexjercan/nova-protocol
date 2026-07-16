# Comms/story-beat action: speaker-attributed story text + HUD comms panel

- STATUS: OPEN
- PRIORITY: 61
- TAGS: v0.7.0, feature, scenario, modding, story

## Goal

A scenario action that presents SPEAKER-ATTRIBUTED story text (e.g.
`StoryMessage((speaker: "Foreman Okono", text: "..."))`) rendered in a
small HUD comms panel with a short queue - the missing story-text
surface the Ledger campaign consumes (objectives are the only text
vocabulary today, and one-liners cannot carry cast or tone). Modding
surface: any scenario/mod can use it.

Direction-level (from spike tasks/20260716-183104/SPIKE.md); /plan
breaks it into steps when picked up. Design constraints from the spike:
scenario-scoped state cleared on teardown (the
state-diff-aliases-reset reset class - a leaked comms line must not
survive into the next scenario or the menu), degradable (the campaign
ships objectives-only text if this slips), and documented in the
scenario authoring guide with strict-RON syntax examples.

## Notes

- Spike: tasks/20260716-183104/SPIKE.md
- Consumer: tasks/20260716-123535 (The Ledger campaign) - land this
  first so chapters can be authored against it.
