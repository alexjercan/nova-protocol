# Scenario timer primitive - design record

Task 20260717-112647, spike tasks/20260717-111808/SPIKE.md (F5, Option C).

## What shipped

A reserved, engine-maintained variable `scenario_elapsed`
(loader::SCENARIO_ELAPSED_VAR, prelude-exported): seconds of LIVE,
UNPAUSED scenario time. `tick_scenario_clock` accumulates Res<Time> delta,
registered CHAINED AHEAD of the OnUpdate pulse under the identical
`scenario_is_live + Unpaused` run condition (loader.rs) - so time-gated
handlers always see the current frame's clock and pause freezes both
together. Teardown's NovaEventWorld::clear() drops it with every other
variable: the clock is per-scenario AND per-retry (the act's clock, which
is what checkpointed retries want).

Failure paths (mod-facing surface):
- Authored WRITE to the reserved key: content_lint ERROR (the engine
  rewrites it every tick; a write is at best a one-frame glitch).
- READ with no VariableSet: exempted from the unset-variable lint warning
  (the engine sets it); a read before the first tick fails closed via the
  existing undefined-variable rule.

Content proof: the example mod's arena gained two copy-me timed beats (a
25s comms nudge, a 45s bonus drifting target), each a threshold filter +
OnStart-seeded one-shot flag. Docs: dev wiki scenario-system.md gained
"The scenario clock" with literal RON syntax for the one-shot and
repeating patterns; example README bullet; CHANGELOG under Modding.

## Why this shape (alternatives considered)

- **OnTimer event kind**: a new event type spans nova_events +
  nova_scenario + serde + docs, and every real use still needs act/flag
  gates for one-shot semantics. Rejected as strictly more surface for
  less composition.
- **Delayed-action wrapper** (`after_seconds` on actions): cannot express
  repetition, needs new schema on every action, and interacts unclearly
  with pause. Rejected.
- **First-class `Factor::Elapsed` AST node**: cleaner namespace than a
  reserved variable (unwritable by construction) but ripples through the
  expression AST, serde round-trips and the modding docs. The reserved
  variable + lint gate gets the same safety with zero schema change;
  revisit only if reserved keys multiply.

## Test notes (the exact rigs)

- `scenario_clock_gates_time_filtered_handlers`: production tick+pulse
  pair on TimeUpdateStrategy::ManualDuration(100ms) (steps under
  Time<Virtual>'s 0.25s max_delta clamp - the manual-time-rig lesson);
  a real Expression(GreaterThan(elapsed, 0.5)) handler holds at ~0.3s,
  fires past ~0.5s. With the tick system deleted the second half can
  never pass (the gate variable stays undefined).
- `scenario_clock_freezes_while_paused`: same rig + PauseStates
  transitions; frozen under Paused, climbs again after unpause
  (delivery-guarded both ways).
- `scenario_clock_resets_with_the_event_world`: clear() drops the key.
- `scenario_clock_reads_are_clean_and_writes_are_errors` (lint.rs): the
  intended read pattern lints clean; a stomp is exactly one ERROR.
- All run via `cargo test -p nova_scenario --features serde` (the
  crate-solo-tests-miss-unified-features trap, grepped before running).

## Verification

- nova_scenario clock + lint tests: 3 + 9 passed (see close-out).
- content_lint over all shipped content: clean (the example's new clock
  reads produce no warnings - the exemption proven on real content; the
  pre-existing ch4 warning only).
- cargo check --workspace --all-targets green; fmt last. Full suite on CI.
