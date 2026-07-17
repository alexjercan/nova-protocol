# Scenario timer primitive: elapsed-time events / delayed actions for authored pacing

- STATUS: CLOSED
- PRIORITY: 51
- TAGS: spike,v0.7.0,scenario,modding,feature

Goal: the scenario engine has no notion of elapsed time - no delay action,
no timer event (events.rs:13-30, actions.rs:28-55); pacing can only come
from proximity gates or player actions, so authors cannot write "breather,
then reinforcements". Add a small timer primitive so scenarios and mods can
author time-based pacing.

Direction notes:
- Candidate shapes (decide in /plan): an OnTimer event kind
  (id + seconds, one-shot or repeating), or an elapsed-seconds term in the
  filter expression language, or a delayed-action wrapper. Prefer whichever
  composes with the existing variable-gate patterns and fails closed.
- Respect pause semantics (OnUpdate freezes under PauseStates::Paused -
  timers must too).
- Ships with a content proof: use it for a wave breather in the reworked
  second scenarios (tasks/20260717-112630, tasks/20260717-112639 land
  first with proximity gates; this upgrades their pacing).
- Mod-facing surface: plan failure paths up front (docs/LESSONS.md
  mod-facing-surface-plans-failure-paths) and document the RON syntax in
  the same change (author-facing-schema-needs-syntax-doc).

Spike: tasks/20260717-111808/SPIKE.md (finding F5; Options C)

Verified at plan time: the OnUpdate pulse gates on
`scenario_is_live.and_then(in_state(PauseStates::Unpaused))`
(loader.rs:266) - the clock ticks in the same gate so pause-freeze comes
free; `scenario_is_live` is just `CurrentScenario.is_some()` (loader.rs:214)
so rigs can arm it directly; NovaEventWorld::reset() clears variables at
teardown (world.rs:176) so the clock restarts per scenario AND per retry
(correct: it is the act's clock). content_lint warns on filter variables no
action sets - the engine-maintained name must be exempted there or every
consumer scenario lints dirty.

DESIGN (chosen over an OnTimer event kind and a delayed-action wrapper):
a reserved, engine-maintained variable `scenario_elapsed` (seconds,
f64, ticked only while the scenario is live and unpaused) exposed to the
EXISTING expression filters. Zero new RON schema: one-shots compose as
`Expression(GreaterThan(Name("scenario_elapsed"), Literal(Number(N)))) `
plus the standard act/flag gate; repeating waves compose as
`elapsed > next_at` + `VariableSet(next_at, Add(next_at, interval))`.
An OnTimer event kind would span three crates and still need act gates;
a delay wrapper cannot express repetition. Failure paths: an authored
VariableSet on the reserved key is a content_lint ERROR (the engine
overwrites it every tick); an unset clock in a filter fails closed by the
existing undefined-variable rule.

## Steps

- [x] Engine: `SCENARIO_ELAPSED_VAR` const (prelude-exported) +
  `tick_scenario_clock` system in nova_scenario chained BEFORE
  fire_on_update under the identical run_if (loader.rs:266); reads
  Res<Time> delta, accumulates into the variable.
- [x] content_lint: exempt SCENARIO_ELAPSED_VAR from the unset-variable
  warning; ERROR on any authored VariableSet writing it (mod-facing
  failure path).
- [x] Content proof in the example mod's arena
  (assets/mods/example/example.content.ron): a timed story beat and a
  timed late-wave reinforcement, both act/flag-gated one-shots -
  the copy-me demonstration of the pattern.
- [x] Tests (nova_scenario; run with --features serde - the
  crate-solo-tests-miss-unified-features trap): clock accumulates across
  ticks (TimeUpdateStrategy::ManualDuration steps <= 0.25s or raised
  max_delta - the manual-time-rig lesson); clock frozen under
  PauseStates::Paused (delivery guard: same rig unpaused advances); an
  Expression(GreaterThan(elapsed, N)) handler holds before N and fires
  after (fail-first: with the tick system removed the test must go red);
  reset() clears the clock (fails closed after teardown).
- [x] Docs: dev wiki scenario-system.md (the clock, the reserved-key
  rule, literal RON syntax for one-shot + repeating patterns -
  author-facing-schema-needs-syntax-doc); CHANGELOG (Modding & Mod
  Portal); example mod README if it enumerates its demonstrations.
- [x] Verify: cargo test -p nova_scenario --features serde <new tests>;
  content_lint (clean on the new example content, ERROR proven on a
  scratch stomp file then removed); cargo check --workspace
  --all-targets; fmt last. Full suite on CI.

## Close-out record

All six steps landed; design, alternatives and the exact rigs are in
NOTES.md. Verification: cargo test -p nova_scenario --features serde =
95 + 1 passed / 0 failed (includes 3 clock tests + the new lint test);
content_lint clean over all shipped content including the example's new
clock reads; cargo check --workspace --all-targets green; fmt last. The
lint ERROR path is unit-proven (a stomp file against the real bin was
skipped as redundant - the bin drives lint_scenario directly). Full suite
on CI per standing instruction.

Reflection: choosing the smallest surface (a reserved variable) over the
obvious feature shape (a new event kind) meant zero schema churn and full
composition with every existing pattern - the spike's open-question about
timer semantics resolved itself once the design goal was restated as
"compose with the gate vocabulary" rather than "add a timer".
