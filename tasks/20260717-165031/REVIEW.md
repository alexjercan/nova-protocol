# Review: Author-configurable orbit-hold and lock-refire durations

- TASK: 20260717-165031
- BRANCH: feature/configurable-event-durations

## Round 1

- VERDICT: APPROVE

Reviewer and implementer share a session, so the two load-bearing claims were
re-derived independently (self-pass here + an out-of-context reviewer agent over
the full diff). Both converged on APPROVE.

### Load-bearing claims - both CONFIRMED

- **Default parity.** Both fields carry
  `#[serde(default, skip_serializing_if = "Option::is_none")]`, so a pre-existing
  RON with no field parses to `None` -> no component inserted ->
  `resolve_window_secs(None, DEFAULT)` -> exactly 5.0. Grep of all 13
  `assets/**/*.ron` shows ZERO occurrences of the new fields; no shipped RON asset
  needs to change. The 5 exhaustive `PlayerControllerConfig` literals across
  nova_assets/nova_editor/nova_scenario correctly got `lock_refire_secs: None`.
- **Threading.** config field -> component insert (correct spawn arm) -> tracker
  query, on every relevant path. `OrbitHoldSecs` only on AI ships, `LockRefireSecs`
  only on player ships, matching each tracker's query. `orbit_hold_secs` with no
  `orbit`: the component is inserted but inert (the ship never gets
  `AutopilotAction::Orbit`, so the tracker early-continues) - matches the lint WARN.
  Both types are reflection-registered; the `tick_lock_slot` signature change has
  exactly one production callsite (all test callsites updated).

### Findings

- [ ] R1.1 (NIT) spaceship.rs (Player/AI spawn arms) - `OrbitHoldSecs` is inserted
  even when `config.orbit` is `None`; the component is inert (never read without an
  orbit directive) and the lint warns on it. Could gate the insert on
  `config.orbit.is_some()`, but leaving it keeps the insert symmetric with
  `lock_refire_secs` and the WARN covers the authoring mistake. Left to discretion.
- [ ] R1.2 (NIT) loader.rs `resolve_window_secs` - silently falls back to the
  default on a bad value that bypassed lint (e.g. a programmatically-built config
  never linted). This is the documented fail-closed intent, not a defect.

### Fail-closed consistency

Runtime accepts iff `secs.is_finite() && secs > 0.0`; lint rejects
`!secs.is_finite() || secs <= 0.0` - exact duals. No value passes lint but
misbehaves at runtime, or vice versa (agree on 0.0, negatives, NaN, infinities).

### Tests

All new tests are discriminating (would fail against the 5s default, proving the
override took effect) and would fail if the fix were reverted:
- `orbit_hold_honors_a_per_ship_override`: one fire by ~1.6s where the 5s default
  needs ~5s (25 frames).
- `a_lock_slot_honors_a_custom_refire_window`: re-fires at 2.5s of hold where the
  5s default would be silent.
- `non_positive_event_window_overrides_are_errors` / `orbit_hold_without_orbit_directive_warns`
  exercise the lint error and warn paths; would fail if `check_controller_durations`
  were removed.
- Updated `a_lock_slot_fires_...` threads `w = LOCK_REFIRE_SECS`, preserving the
  default-path coverage.

Suite: `cargo test -p nova_scenario --features serde --lib` -> 101 passed, 0
failed. `cargo check` + test-compile of nova_assets/nova_editor clean. `cargo fmt`
clean. (`-p nova_scenario` without `--features serde` fails to compile pre-existing
serde tests - unrelated, per ledger `crate-solo-tests-miss-unified-features`.)
