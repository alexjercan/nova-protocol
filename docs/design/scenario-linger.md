# Scenario transitions: when `linger` is false

`NextScenarioActionConfig.linger` (crates/nova_scenario/src/actions.rs) decides
whether a queued scenario switch fires immediately or waits.

- `linger: true` - defer the switch. The current scenario stays on screen (its
  `Outcome` overlay visible) until `release_lingering_next` clears the flag,
  which the Continue/Retry button and the Enter/DPadDown advance input both
  drive (crates/nova_scenario/src/world.rs:250, loader.rs `on_next_input`). The
  switch is player-acknowledged.
- `linger: false` - fire now (or after `delay` seconds of world-keeps-playing,
  the "middle gear"; see the `delay` doc in actions.rs). An immediate cut: the
  next scenario loads and the current one tears down without waiting for input.

## Where `linger: false` makes sense (audit, task 20260717-201534)

The rule is set by the lint (crates/nova_scenario/src/lint.rs:212-238): an
`Outcome` and a non-lingering `NextScenario` in the SAME handler is a trap.
Undelayed, the instant switch tears the world down and swallows the overlay
before it can show; delayed, the overlay's pause freezes the delay clock so the
cut never comes while the player reads. So:

- A transition that shows a Victory/Defeat `Outcome` overlay MUST use
  `linger: true` (optionally with `auto_advance_secs` on the Outcome for a
  timed banner). The overlay is the whole point - the player reads it and
  dismisses it.
- A transition with NO `Outcome` in its handler - a pure bridge/relay cut - is
  where `linger: false` belongs. A lingering switch there has nothing to
  dismiss, so it strands the player until a stray Enter press.

A full sweep of the project (base RON scenarios, webmods, Rust-coded builtins)
found every `NextScenario` paired with an `Outcome` EXCEPT the `asteroid_next`
relay, which exists in two forms:

- `assets/base/scenarios/asteroid_next.content.ron`
- the built-in `asteroid_next` in crates/nova_assets/src/scenario.rs

Both are a bare `OnStart -> NextScenario(asteroid_field)` with no overlay: the
invisible relay that recycles the endless asteroid loop. They were the only
transitions that qualified for `linger: false`, and were flipped. Every
Outcome-paired transition (broadside, shakedown, asteroid_field win/lose, the
ledger chapters, gauntlet) correctly stays `linger: true`.

The built-in relay is pinned by
`scenario::tests::asteroid_next_bridge_is_a_non_lingering_cut`
(crates/nova_assets/src/scenario.rs); the RON asset is covered by the
content-lint gate (crates/nova_assets/tests/content_lint_gate.rs).
