# Extend ScenarioLoaded smoke assertion to 10_gameplay and 07_torpedo_guidance

- STATUS: CLOSED
- PRIORITY: 30
- TAGS: v0.4.0, test

Follow-up to 20260708-194524, which wired the `ScenarioLoaded` payload assertion
into the `03_scenario` headless smoke test (landed 0af7f4e). Extend the same
observe-and-assert pattern to the other scenario-loading smoke examples so every
headless scene guarantees a non-empty scenario init instead of passing on
`autopilot: cycle complete` alone.

Supports the 0.4.0 testability theme (roadmap spike 20260708-161726).

Unlike `03_scenario`, these two examples build bespoke `ScenarioConfig`s inline
(`gameplay_scenario` in `10_gameplay`, `guidance_scenario` in `07_torpedo_guidance`)
rather than looking one up in `GameScenarios`, so the expected `scenario_id`
differs per example (`gameplay_scenario` uses its own id; `07` uses
`"torpedo_guidance"`). The assertion must key off each example's own scenario id
and its own known object/handler counts, not a shared constant.

## Steps

- [ ] Factor the `03_scenario` assertion into a small reusable helper so the
  three examples do not copy-paste it. Candidates: a debug-only helper in
  `nova_debug` (e.g. an observer/plugin `assert_scenario_loaded(expected_id)`
  that panics on a trivial/never-fired payload), or a shared example module.
  Prefer the `nova_debug` harness home since that is where the smoke-test
  machinery already lives; confirm it can depend on `nova_scenario` without a
  cycle before committing to that placement.
- [ ] Wire the helper into `examples/10_gameplay.rs`, passing the id that
  `gameplay_scenario` actually sets. Keep it debug-gated like the existing
  autopilot/screenshot wiring.
- [ ] Wire the helper into `examples/07_torpedo_guidance.rs`, expecting
  `"torpedo_guidance"`. Note this example already fires torpedoes via the
  autopilot input closure; the assertion is independent and should not interfere.
- [ ] If the helper lands in `nova_debug`, retro-fit `03_scenario` to use it too,
  so all three share one implementation and the bespoke `03` version does not
  drift.
- [ ] Verify each example headless under `Xvfb :99 & DISPLAY=:99`
  (`BCS_AUTOPILOT=1 cargo run --example <name> --features debug`): confirm the
  `smoke: ScenarioLoaded ...` line shows non-zero counts and the run still
  reports `cycle complete, no panic`. Build the examples cold first, then time
  only the run (lesson from 20260708-194524).
- [ ] Run the full check suite (`cargo test`, `cargo clippy --all-targets
  --features debug`, `cargo fmt --check`).

