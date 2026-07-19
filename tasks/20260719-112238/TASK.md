# nova_probe: structured run-event logging + per-run timeline recorder (correctness capture - the crux)

- STATUS: OPEN
- PRIORITY: 74
- TAGS: v0.8.0, spike, tooling, performance, testing

## Goal

Add structured run-event logging and a per-run timeline recorder: instrument the
game to emit a small structured `ProbeEvent` (timestamp, frame, kind, scenario-
variable snapshot) at the moments that matter, and a recorder plugin that
captures the ordered timeline of a headless autopilot run. This is the CRUX of
the whole tool (correctness capture) and the riskiest piece - de-risk early.

## Notes

- Spike: tasks/20260719-112011/SPIKE.md.
- Emit at: GameStates transitions, scenario variable changes, the scenario
  event-handler signals (kill tally, travel-lock, arrival), autopilot script
  beats. Prefer reusing the scenario's existing event stream over a parallel one.
- Also the "improve in-game logging" the user asked for.
- Open question to resolve HERE empirically: how stable is the timeline run-to-
  run under llvmpipe throttling? Key the recorder on ordered event KINDS + var
  values with generous timing tolerance, not wall-clock, if timing is noisy.
- Depends on the crate skeleton (T1).
