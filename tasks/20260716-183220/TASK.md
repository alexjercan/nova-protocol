# Comms/story-beat action: speaker-attributed story text + HUD comms panel

- STATUS: CLOSED
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

## Steps

- [x] `StoryFeed(Vec<StoryLine{speaker,text}>)` resource + a minimal
      comms panel widget in a new crates/nova_gameplay/src/hud/comms_panel.rs
      (HudTier::Chrome, bottom-left, shows the LATEST line as
      "SPEAKER > text", 8s dwell timer reset by resource_changed, hides
      when expired or empty; sub-plugin added in hud/mod.rs like
      ObjectivesPlugin).
- [x] `StoryMessageActionConfig{speaker,text}` +
      `EventActionConfig::StoryMessage` variant + dispatch arm in
      crates/nova_scenario/src/actions.rs (serde like siblings).
- [x] NovaEventWorld: `story_messages` Vec + push + CLEAR in clear() +
      write-on-diff sync into StoryFeed in state_to_world_system
      (length compare suffices - the log is append-only within a
      scenario; world.rs).
- [x] Tests: world.rs - dispatch-driven StoryMessage lands in StoryFeed
      and UnloadScenario/clear empties it (the reset class);
      comms_panel.rs - latest line renders on feed change, dwell expiry
      hides it, teardown-empty feed hides it; serde round-trip of the
      new action.
- [x] Docs: guide-author-scenario.md gains the action with strict-RON
      example; CHANGELOG Unreleased (Modding & Mod Portal).
- [x] Verify: check --all-targets, fmt, run the touched test targets
      (nova_scenario paired with nova_menu per the crate-solo lesson;
      nova_gameplay hud tests).

## Notes

- Spike: tasks/20260716-183104/SPIKE.md
- Consumer: tasks/20260716-123535 (The Ledger campaign) - land this
  first so chapters can be authored against it.

## Close notes (2026-07-16)

What changed: StoryMessageActionConfig + EventActionConfig::StoryMessage
(nova_scenario/src/actions.rs, RON documented in the authoring guide);
NovaEventWorld gained a scenario-scoped story_messages log (pushed by
the action, cleared by world.clear(), synced write-on-diff into the new
StoryFeed resource - guarded with get_resource_mut so event-world rigs
without the HUD half keep working); the comms panel widget
(nova_gameplay/src/hud/comms_panel.rs: HudTier::Chrome +
HudSelfDrivenVisibility, bottom-left, renders the latest line as
"SPEAKER > text" with an 8s dwell, hides on expiry or an emptied feed).
CHANGELOG + authoring-guide entries.

Tests: serde round-trip of the authored RON shape; world sync test
(line lands in the feed, unchanged log does not re-flag the resource,
clear empties the feed, a rig WITHOUT StoryFeed does not panic); three
panel tests (speaker-prefixed render + latest-replaces, dwell expiry on
a MEASURED clock, emptied-feed hides immediately).

Difficulties: the dwell test initially trusted TimeUpdateStrategy::
ManualDuration(0.5) at face value; the panel never expired because in
this rig each update advances Time by 0.25s (measured with a throwaway
probe test printing deltas - diagnostic-first paid off). The test now
documents and asserts against the measured rate with wide margins.

Verification: comms_panel 3/3, story world+serde tests 2/2 (paired
nova_scenario run), check --all-targets + fmt clean. Full suite is CI's
job per the standing instruction.
