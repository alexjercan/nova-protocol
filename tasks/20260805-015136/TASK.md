# StepBuilder::on_enter appends actions in call order

- STATUS: CLOSED
- PRIORITY: 98
- TAGS: v0.11.0, bug, testing, autopilot

## Story

The type is now named `StepBuilder`, but its `on_enter` still REPLACED the
step's enter hook rather than appending. Two chained `.on_enter(...)` calls on
one step silently dropped the first. Task `20260804-094006` lost a full measurement cycle to this: the
dropped call was `capture_reload_end`, so the fps reload gate latched open and
every frame after the first loop was excluded from the capture. The run still
passed - it just measured nothing.

The mitigation that shipped is three copies of a warning comment across the
`stress/` sweeps, which does not help the next caller anywhere else.

Fix it at the builder: either append hooks (run both, in call order) or reject
a second `on_enter` on the same step with a panic naming the step. Appending is
the better default if nothing depends on replacement; grep the existing callers
before choosing.

## Done when

- [x] `cmd:` a test pins that two `on_enter` calls on one step run in call
      order and failed against the old builder.
- [x] `cmd:` the three duplicated warning comments in `examples/stress/` are
      deleted, and the sweeps fill their capture windows
      (`cargo run --features debug -- probe run stress` -> aggregate OK).
