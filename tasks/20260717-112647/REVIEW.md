# Review: Scenario clock primitive

- TASK: 20260717-112647
- BRANCH: work/scenario-timer

## Round 1

- VERDICT: APPROVE

- [ ] R1.1 (MINOR) crates/nova_scenario/src/world.rs:87-104 - the clock defeats
  the change-only variable debug log. `state_to_world_system` logs the full
  variable dump only when `variables != last_logged_variables`, a guard whose
  own comment exists "to avoid per-frame spam". `scenario_elapsed` now changes
  every live unpaused frame, so the guard is true every frame: with debug
  logging on, the entire variable table is dumped per frame for the whole
  scenario (burying the transitions the log was built to show), and even with
  logging off the full `HashMap` snapshot clone runs every frame. Suggested
  change: exclude `SCENARIO_ELAPSED_VAR` from the diff (compare and log the
  map minus the reserved key), or diff on a filtered view; the clock's value
  is never interesting per frame.
  - Response: fixed - the change-only log diff now ignores the reserved
    clock key (differs_ignoring_clock in state_to_world_system); no more
    per-frame dump or snapshot clone.

- [ ] R1.2 (MINOR) crates/nova_scenario/src/loader.rs:268-273 (and test rigs at
  1544-1548, 1621-1626) - the load-bearing registration (chain + shared
  run_if) is mirrored in the tests, not exercised. Both new tests re-declare
  `(tick_scenario_clock, fire_on_update).chain().run_if(..)` inline, so a
  future edit to the Plugin's registration (splitting the chain, gating only
  the pulse) keeps every test green while production ordering or the pause
  freeze silently regresses. The repo already has the cure as precedent:
  `configure_scenario_gating` (loader.rs:568) is factored out expressly "so
  the tests below exercise the production wiring". Suggested change: extract
  the pair into a `pub(crate)` configure helper called by the Plugin and both
  test rigs. (The pre-existing pulse test at loader.rs:1430 has the same
  mirror, so this is inherited, but this change makes the chained pairing
  load-bearing and triples the copies.)
  - Response: fixed - register_clock_and_pulse() is the one registration,
    used by the plugin AND both new test rigs.

- [ ] R1.3 (MINOR) web/src/wiki/dev/scenario-system.md:193-195 - the
  repeating-wave rearm is not literal RON. `VariableSet(next_at,
  Add(Factor(Name("next_at")), Term(Factor(Literal(Number(interval))))))` will
  not parse: `VariableSet` takes `((key: "next_at", expression: ...))` (the
  `Add(term, expr)` inner shape IS correct per variables.rs:123). The task
  step promised "literal RON syntax for one-shot + repeating patterns"; the
  one-shot block is literal and matches shipped content exactly, but the
  repeating pattern - the harder one, and the only one with NO shipped worked
  example - is prose shorthand an author will copy and hit a parse error on.
  Suggested change: a second fenced ```ron block with the full repeating
  handler (filters gating `elapsed > next_at`, action
  `VariableSet((key: "next_at", expression: Add(Factor(Name("next_at")),
  Term(Factor(Literal(Number(20.0)))))))`), plus the OnStart seed of
  `next_at`.
  - Response: fixed - the wiki now carries literal strict-RON for the
    repeating pattern (seeded next_at, Add rearm).

- [ ] R1.4 (NIT) web/src/wiki/dev/scenario-system.md:162-197 - the snapshot
  pattern is unmentioned. Reading the clock inside a VariableSet RHS
  (`my_start = scenario_elapsed`) lints clean (the exemption sits on the
  shared `used_vars` loop, lint.rs:174-189, fed by both filter expressions
  and VariableSet RHSes via `collect_expression_vars`, lint.rs:252) and works
  at runtime (`VariableSetActionConfig::action` evaluates against the event
  world where the engine key lives, actions.rs:406-419). Snapshotting is
  exactly how an author measures "duration since event X" (gate on
  `Subtract(Factor(Name("scenario_elapsed")), Term(Factor(Name("my_start"))))`
  later); one wiki sentence naming the pattern would save authors deriving it.
  - Response: fixed - the snapshot pattern is documented in the same wiki
    section (read always fine, write gated).

- [ ] R1.5 (NIT) crates/nova_scenario/src/loader.rs:359 - use
  `time.delta_secs_f64()` instead of `f64::from(time.delta_secs())`. The
  accumulator is f64 but each delta is quantized through f32 first
  (~1e-9 s/frame bias; ~0.1-0.2 ms/hour at 60 fps - harmless at scenario
  timescales, but the exact call exists on `Time` at zero cost, so the f64
  promise may as well be kept end to end).
  - Response: fixed - delta_secs_f64().

- [ ] R1.6 (NIT) CHANGELOG.md:42 - consider the `**(breaking)**` tag. The
  changelog convention (used one line below for the asset-scheme change) tags
  mod-facing compat breaks. Reserving `scenario_elapsed` breaks any external
  mod that already used that name as its own variable: a write is now a lint
  ERROR, and the runtime content gate (loader.rs on_load_scenario) refuses to
  start scenarios with errors - previously-working content now fails loud.
  No shipped content collides (verified: the only occurrences under assets/
  and webmods/ are this change's reads), and fail-loud is the designed path,
  but the entry could say so.
  - Response: fixed - the CHANGELOG line carries **(breaking)** and names
    the rename a pre-existing colliding mod would need.

### Verification record

Re-derived from source (not from NOTES):

- (a) Chained registration / shared run_if: loader.rs:268-273 registers
  `(tick_scenario_clock, fire_on_update).chain().run_if(scenario_is_live.and_then(in_state(PauseStates::Unpaused)))`.
  Bevy 0.19 semantics confirmed from vendored source
  (bevy_ecs-0.19.0/src/schedule/config.rs): `run_if` on a collection pushes
  into `collective_conditions` - one condition on the anonymous set,
  "evaluated at most once (per schedule run), the first time a system in this
  set prepares to run", gating BOTH members (the per-system variant is
  `distributive_run_if`, a different method). `.chain()` sets ordering
  metadata tick -> pulse. So the claim holds: one gate, both systems, clock
  before pulse.
- (b) Teardown clears the clock, including retry: the variable lives in
  `NovaEventWorld.variables`; `NovaEventWorld::clear()` (world.rs:162-178)
  calls `variables.clear()`. `teardown_scenario_entities` (loader.rs:587)
  calls `world.clear()` and is shared by BOTH the unload observer
  (loader.rs:629) and the load path (on_load_scenario, loader.rs:694, which
  tears down before spawning the next). Retry routes through
  `release_lingering_next` (world.rs:222) -> `state_to_world_system` sees the
  non-lingering request and triggers `LoadScenario` (world.rs:109-126) ->
  on_load_scenario -> teardown -> clear. So the clock resets on retry too.
  Edge: a refused start (content-gate errors) returns BEFORE teardown
  (loader.rs:668-691), leaving the old scenario and its clock live -
  consistent with "whatever was on screen stays".
- (c) Pause and virtual time: `Res<Time>` in Update is virtual time.
  nova_menu (crates/nova_menu/src/lib.rs:210-217, 321-335) runs
  `pause_clocks` on OnEnter(PauseStates::Paused): `virtual_time.pause()` +
  `physics_time.pause()`; `sync_outcome_pause` holds Paused while an outcome
  shows. So in the real game the clock is double-frozen under pause (zero
  delta AND the run_if gate); in headless rigs without nova_menu the run_if
  is the only freeze, which the pause test exercises. No
  `set_relative_speed` call exists anywhere in crates/ - there is no time
  scaling today; if one is added the clock follows virtual time, which
  matches the documented "live, unpaused scenario time". Docs are accurate.
- Namespace collision: `grep -rn scenario_elapsed assets/ webmods/` finds
  only this change's example-mod reads and README/comment mentions - no
  shipped authored variable collides.
- Lint scope: `EventActionConfig` (actions.rs:31-58) has no variant nesting
  other actions - `ScatterObjects` nests an object template, not actions - so
  `VariableSet` exists only at action level and the write ERROR in
  `check_action` (lint.rs:238-252) covers every authored write. The
  unset-variable exemption sits on the single `used_vars` loop
  (lint.rs:174-189), which is fed by filter expressions (check_filter ->
  collect_condition_vars, incl. recursion through Conditional Not/And/Or) AND
  VariableSet RHSes (lint.rs:252) - so a snapshot read in a RHS lints clean,
  and runs correctly (actions.rs:406-419 evaluates against the world;
  undefined -> error log + skip; filters fail closed, filters.rs:161-173).
- Tests vs a deleted tick system:
  `scenario_clock_gates_time_filtered_handlers` - without the tick,
  `scenario_elapsed` is never defined, the expression filter fails closed
  forever, `beat_fired` stays None, the final assert fails: goes red.
  `scenario_clock_freezes_while_paused` - without the tick, elapsed stays 0.0
  and the FIRST assert (`before_pause > 0.0`) fails: goes red.
  `scenario_clock_resets_with_the_event_world` - does not involve the tick
  (it proves clear() semantics only); it would still pass, which is fine for
  its stated purpose. Manual-clock steps: 100 ms per update, under
  `Time<Virtual>` DEFAULT_MAX_DELTA = 250 ms (bevy_time-0.19.0/src/virt.rs:86),
  so no clamping; the threshold asserts (3 updates < 0.5s < 8 updates) hold
  whether or not the first update's delta is zero.
- Content proof: `nudge_sent` and `drifter_sent` are seeded 0.0 in OnStart
  (example.content.ron:216-223). The win check (example.content.ron:286-317)
  gates on `destroyed > 1`, and `destroyed` is bumped only by the two
  id-filtered OnDestroyed handlers for example_target_1/2 - the bonus
  drifter (example_target_3) has no OnDestroyed handler and cannot count
  toward or be required for the win. Both timed beats also gate on
  `arena_done == 0`, so neither fires after the win. Drifter position
  (60, 15, -140): nearest existing object is target_2 at (15, -5, -60),
  ~94 units away; player spawns at origin ~153 units away - clear. 25s/45s
  vs playtime: two 50-HP targets at 45-60 units should clear in well under a
  minute; the 25s nudge only fires for slow play and the 45s drifter is
  gated off once done - sensible.
- Wiki one-shot RON block matches the shipped example mod's strict syntax
  exactly (Expression((GreaterThan(Term(Factor(Name(..))), ..)))); the
  repeating-pattern prose does not (R1.3). AST shapes confirmed against
  variables.rs:29-126 (`Add(Box<VariableTermNode>, Box<VariableExpressionNode>)`).
- CHANGELOG:42 claims (pause freezes, retry restarts, filter-readable, write
  is lint error, example ships both beats) all match verified behavior.

Test runs (verbatim result lines):

`cargo test -p nova_scenario --features serde`:

```
test result: ok. 95 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.46s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.84s
```

The four new tests by name (`cargo test -p nova_scenario --features serde scenario_clock`):

```
test loader::tests::scenario_clock_resets_with_the_event_world ... ok
test lint::tests::scenario_clock_reads_are_clean_and_writes_are_errors ... ok
test loader::tests::scenario_clock_freezes_while_paused ... ok
test loader::tests::scenario_clock_gates_time_filtered_handlers ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 91 filtered out; finished in 0.01s
```

`cargo run -p nova_assets --bin content_lint`:

```
WARN  [the-ledger] scenario 'ledger_ch4_the_buyer': object id 'auditor' is spawned by more than one handler - fine only if the handlers are mutually exclusive
content_lint: clean (1 warning(s))
```

(The ledger warning is pre-existing and unrelated; the example mod's new
clock reads lint clean, proving the exemption on real content.)

Design verdict on the reserved variable vs a first-class AST factor: the
reserved key composes with every existing gate pattern at zero schema cost,
the write path is closed by a lint ERROR that the runtime gate enforces as
refuse-to-start, and no shipped content collides. A `Factor::Elapsed` node
would be unwritable by construction but ripples through AST/serde/docs;
NOTES' "revisit if reserved keys multiply" is the right line to hold.
