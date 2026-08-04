# Decision: code-built chain targets register in GameScenarios, and outcomes runs on step beats

- DATE: 20260804-093934
- STATUS: ACCEPTED
- TASK: 20260804-093934
- TAGS: decision, examples, testing, scenario

## Context

`systems/outcomes` composes the whole outcome arc - die, Defeat overlay, Retry,
clean reload, kill, objective + CHECKPOINT, Continue, next scenario loaded - on
two ~50-line code-built `ScenarioConfig` fixtures instead of the 8000 lines of
campaign RON that `broadside`/`lifeline` carried it on.

NOTES.md left two questions OPEN (how the run kills the player, and what a
CHECKPOINT is in code) and sketched the fixtures as if a code-built scenario
could chain to another code-built scenario the same way a shipped one does.
Reading the switch showed it cannot, which is the decision below. The remaining
choices are the script's shape and the owner calls carried forward.

## Decision

### D1 - Code-built chain targets must be registered in `GameScenarios`

The queued `NextScenario` does NOT carry a config. `NovaEventWorld`'s
`state_to_world_system` resolves the queued id against the `GameScenarios`
resource by key and, on a miss, logs
`next scenario id '...' not found in GameScenarios; unloading` and triggers
`UnloadScenario` (`crates/nova_scenario/src/world.rs:201-220`).

So `outcomes.rs` inserts BOTH fixtures into `GameScenarios` before its first
`LoadScenario`. Without that, the Retry requeue and the Continue chain each
silently unload to the menu rather than chaining, and the run's central claim
would be untestable. The shipped-story examples never had to do this: their
scenarios reached the resource through the content bundle merge.

Both fixtures carry `hidden: true`, so registering them adds no rows to the
Scenarios picker.

### D2 - `.step()` beats, not the legacy staged collector

`outcomes.rs` is written on the `AutopilotPlugin` step builder
(`.step().on_enter().until().deadline().add()`), like `player_path`
(`examples/gameplay/playable.rs:69-134`), not on `broadside`/`lifeline`'s
hand-rolled `SliceAutopilot` stage-number collector.

The arc is strictly linear and every beat has a world predicate to wait on.
`crates/nova_debug/src/harness.rs:173` states the intent directly: "A script
written fresh ends its last step on a world predicate and lets the driver
report done." The stage-number shape exists only because those two examples
predate the builder, and `20260804-093910` deletes both.

### D3 - `auto_advance_secs` stays `None` on both fixtures

An outcome carrying `auto_advance_secs` releases its own lingering chain after
N real seconds (`crates/nova_menu/src/outcome.rs`, `auto_advance_outcome`).
Left set, the overlay would advance itself and the run would pass without the
script's `Activate` ever being the cause - a vacuous proof of the very path
this example exists to walk.

### D4 - Owner calls carried forward (2026-08-04)

- Fixtures are built LOCALLY in each `systems/` example. No shared builder and
  no `systems/fixtures.rs`: `20260804-094006` is the third caller and extracts
  the shared `fn` once all three shapes are visible. One caller is not an
  abstraction, and a signature designed from one caller would get the knobs
  wrong.
- The overlay is advanced by triggering `Activate` on the
  `Outcome Primary Button` entity, never by `click_at` screen coordinates.

## Alternatives considered

- **Carry the next `ScenarioConfig` in the `NextScenario` request.** Rejected:
  a `nova_scenario` interface change to serve one example, when the id-keyed
  registry is the production mechanism every story chain already uses.
- **Skip the chain and assert only that `next_scenario` was queued.** Rejected:
  the queue is already pinned headlessly in
  `crates/nova_menu/src/tests/outcome.rs`. The composed, rendered, actually-
  loads-B path is the only thing this run adds over those unit tests, and
  dropping it would leave the mainline retirement uncovered.
- **A declarative scenario action that damages the player**, keeping the
  fixture pure. Rejected: no such variant exists in `EventActionConfig`, so it
  would mean adding one for a test. The world-mutating overkill on the ship
  ROOT is the production damage entry point and has two precedents
  (`lifeline.rs:165`, `broadside.rs:163`).
- **Keep the `SliceAutopilot` stage-number shape** for symmetry with the
  examples being replaced. Rejected: it is the shape the step builder was
  built to retire, and both of its users are deleted next task.

## Consequences

- This run does NOT prove the outcome buttons are clickable at their rendered
  coordinates - it proves the mechanics behind them. Nothing covers that gap
  unless `ui/` covers the overlay; if it matters, it belongs to
  `20260804-094021`, not here.
- `outcomes.rs` writes to `GameScenarios`, which is otherwise owned by the
  bundle merge. It is a test fixture writing into a production registry, and it
  is the reason the example must set `hidden: true`.
- A CHECKPOINT is documented for the first time as a PATTERN
  (`Outcome(Victory)` + lingering `NextScenario`), not a type. Any later search
  for a checkpoint concept in code will keep coming up empty; the scenario
  lint's warning on `Outcome` + non-lingering `NextScenario` in one handler is
  the closest thing to an enforcement point.
- `20260804-094006` inherits a concrete extraction job: three visible fixture
  shapes, deliberately duplicated, with the count knob still undesigned.
