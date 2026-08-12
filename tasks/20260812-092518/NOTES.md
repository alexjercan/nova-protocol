# NOTES

## Result

Added keyed scenario timers with `TimerStart`, `TimerCancel`, `OnTimerEnd`, and
a timer-key filter. Timers use `scenario_elapsed` deadlines, so the existing
pause and retry clock semantics apply without a second time source.

## Decisions

- Timer state is separate from `VariableLiteral`. Expressions stay values, not
  active engine objects.
- Starting an existing key restarts it. Cancelling a missing key is a no-op.
- Durations must be positive finite numbers. Invalid starts preserve an
  existing timer.
- Ended keys are removed before dispatch and sorted by key. A handler can
  restart its key; simultaneous ends are deterministic.
- Timer-end events queue before the frame's `OnUpdate` pulse.

## Coverage

- Unit coverage: start, restart, cancel, invalid duration, teardown, RON, lint.
- Dispatch coverage: filtered one-shot expiry and pause freeze through the
  production clock/event chain.
- Player-path coverage: `examples/systems/scenario_grammar.rs` starts a timer
  on `OnStart` and waits for its filtered `OnTimerEnd` result.
