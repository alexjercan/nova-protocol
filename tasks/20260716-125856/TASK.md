# Scenario outcome frame: Victory/Defeat action + overlay (Enter to continue/retry, Esc to menu)

- STATUS: OPEN
- PRIORITY: 86
- TAGS: v0.7.0,feature,scenario,ui


## Goal

The game has no explicit win/lose presentation: today a scenario "ends" by
silently queuing NextScenario (player death in shakedown_run queues a restart
the player discovers by accident). Add the outcome FRAME as mod-facing
scenario vocabulary: a `ScenarioOutcome` action (Victory | Defeat + optional
message) and an overlay that presents it - banner, message, and prompts
([Enter] continue/retry via the existing linger-advance input, [Esc] menu via
pause). Chaining stays NextScenario's job; the action only owns presentation.
Consumers: the vertical slice (20260708-203659), shakedown_run's death
restart, the story campaign mod (20260716-123535), Gauntlet 2.0.

## Steps

- [ ] Add `Outcome` variant to `EventActionConfig` in
      crates/nova_scenario/src/actions.rs with
      `OutcomeActionConfig { outcome: OutcomeKind (Victory|Defeat), message: Option<String> }`,
      serde like the other actions; EventAction impl queues a command that
      triggers a `ScenarioOutcomeEvent` (or sets a scenario-scoped resource) -
      follow the DebugMessage/Objective queued-command pattern.
- [ ] Overlay UI in crates/nova_menu (pause-overlay style, nova_ui theme):
      on the outcome event spawn a scenario-scoped overlay - VICTORY/DEFEAT
      banner, optional message line, prompt row. Prompt text reflects reality:
      show "[Enter] ..." only when a lingering NextScenario is queued (read
      NovaEventWorld.next_scenario), "[Esc] menu" always. ScenarioScopedMarker
      so scenario teardown despawns it.
- [ ] Confirm the Enter advance path stays live while the overlay is up:
      on_next_input (crates/nova_scenario/src/loader.rs:626) only gates on
      PauseStates::Paused - verify no other gating blocks it, then decide
      whether Defeat should also suppress HUD chrome (probably not in v1).
- [ ] Enter with NO queued scenario: make it return to MainMenu (victory at
      end of content) - smallest correct behavior; wire via GameStates.
- [ ] Dogfood in shakedown_run: the existing OnDestroyed(player) handler
      (assets/base/scenarios/shakedown_run.content.ron:1226) gains
      `Outcome(Defeat, "Your ship broke apart in the belt.")` before its
      NextScenario restart - edit the BUILDER (crates/nova_assets/src/scenario/shakedown.rs),
      regenerate content.ron via the content_ron_parity test.
- [ ] Headless integration test (nova_scenario or nova_menu tests): trigger
      the action -> overlay entities exist with the right banner text; Enter
      -> scenario switch consumed; no queued scenario -> MainMenu.
- [ ] Docs per keeping-docs-in-sync: dev wiki scenario-system action list +
      guide-author-scenario (web/src/wiki/dev/), player wiki scenarios page
      if wording changes, CHANGELOG [Unreleased] line.
- [ ] Verify: cargo check/fmt + the new tests; run the shakedown death path
      under Xvfb and eyeball the overlay (render-output-eyeball).

## Notes

- Verified mechanisms (2026-07-16 survey, file:line in-repo):
  - NextScenario linger + advance input: crates/nova_scenario/src/loader.rs:560-642
    (Enter/DPadDown clears linger; gated only by PauseStates::Paused).
  - NovaEventWorld.next_scenario switch loop: crates/nova_scenario/src/world.rs:68-95.
  - Player death: sections explode, root try_despawn'd, OnDestroyed fires
    (crates/nova_gameplay/src/integrity/explode.rs); no game-over state exists.
  - GameStates: Loading/MainMenu/Playing only (crates/nova_gameplay/src/lib.rs:92).
  - Base scenarios are Rust builders; content.ron is REGENERATED via the
    content_ron_parity test (crates/nova_assets/src/lib.rs:42,73).
- Naming: action name `Outcome` in RON keeps authoring terse; bikeshed at
  implementation if it collides.
- Depends on: nothing open. Blocks: 20260708-203659.
- Spike: tasks/20260716-122954/SPIKE.md; plan docs/plans/20260716-v0.7.0-plan.md.
