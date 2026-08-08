# ScriptBuilder::on_enter silently replaces instead of appending

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,bug,testing

## Story

`ScriptBuilder`'s `on_enter` REPLACES the step's enter hook rather than
appending, so two chained `.on_enter(...)` calls on one step silently drop the
first. Task `20260804-094006` lost a full measurement cycle to this: the
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

- [ ] `cmd:` a test pins that two `on_enter` calls on one step both run (or
      that the second call panics), failing against today's builder.
- [ ] `cmd:` the three duplicated warning comments in `examples/stress/` are
      deleted, and the sweeps still fill their capture windows
      (`cargo run -p nova_probe -- run stress --fps` -> aggregate OK).
