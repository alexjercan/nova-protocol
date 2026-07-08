# Assert on ScenarioLoaded payload in the smoke harness

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.4.0,test

Follow-up to 20260525-133011, which enriched `ScenarioLoaded` with `scenario_id`,
`handler_count` and `object_count` (landed cc03c40). The event now carries the
data but nothing reads it - close the loop by having the headless smoke harness
observe `ScenarioLoaded` and assert on the payload, so a broken scenario init
fails the smoke test instead of silently loading an empty scenario.

Supports the 0.4.0 testability theme (roadmap spike 20260708-161726).

## Steps

- [ ] Add an observer for `ScenarioLoaded` in the smoke-test harness path used by
  the examples (see `examples/09_editor.rs` and the gameplay examples wired to the
  `bevy_common_systems` autopilot + screenshot harness). Record the payload where
  the autopilot cycle can reach it (e.g. a resource), rather than only logging it.
- [ ] Assert the payload is non-trivial for a real scenario: `scenario_id` matches
  the loaded scenario, and `handler_count` / `object_count` are greater than zero
  (or equal to the scenario's known config counts). Fail the smoke run - non-zero
  exit / panic in the autopilot `cycle complete` check - when the assertion fails.
- [ ] Pick the smallest scenario-loading example that already runs headless so the
  assertion rides an existing smoke test rather than adding a new binary; confirm
  it still passes under `Xvfb :99 & DISPLAY=:99` as documented in the example
  headers.
- [ ] Run the full check suite (`cargo test`, `cargo clippy --all-targets`,
  `cargo fmt --check`).

