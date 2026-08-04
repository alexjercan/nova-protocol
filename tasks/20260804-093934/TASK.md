# Build systems/: code-built fixtures for scenario grammar, the player path, and outcomes

- PRIORITY: 82
- TAGS: v0.10.0, content, examples, testing
- KIND: STORY
- ACTIVITY: WORKING
- GATES: PLAN
- RESOLUTION: -
- PARENT: 20260802-115955
- DEPENDS ON: 20260804-003244, 20260804-093855

## Story

Build the `systems/` category: story-free fixtures, built in code, proving the
cross-cutting gameplay systems that the retired mainline runs used to cover
incidentally.

Per the roster spike (`20260804-003244`), test scenario content is built as a
`ScenarioConfig` value in Rust and loaded with `LoadScenario` - never shipped under
`assets/`, never authored as example-owned RON. The compiler then catches
scenario-grammar changes, and a code-built builder is a reusable `fn` that
`sections/` and `stress/` can share with a count knob.
`examples/gameplay/scenario.rs:89` and `playable.rs:167` already do exactly
this; the rule promotes their precedent. The type is `ScenarioConfig`
(`crates/nova_scenario/src/loader/mod.rs:147`) - there is no `Content` type.

## Steps

- [ ] Move `examples/gameplay/scenario.rs` -> `examples/systems/scenario_grammar.rs`
      with `git mv`, ATOMIC with its catalog and smoke edits, because
      `catalog_matches_disk` is red between them: in `Cargo.toml` the
      `[[example]] name = "scenario"` block becomes `scenario_grammar` under a
      new `systems/` comment block; in `tests/examples_smoke.rs` add
      `const SYSTEMS` and `systems_reach_playing_without_panic`, and drop the
      entry from `GAMEPLAY:43`. Leave the `GAMEPLAY` const and its test alive
      with `broadside`/`lifeline` - `20260804-093910` retires them, not this
      task, and the same goes for the TRANSITIONAL `gameplay` row in
      `crates/nova_probe/src/catalog.rs:191`. The `systems` policy row already
      exists there (`probed: true, frame_time: false`), so no probe edit is
      needed. Commit; `catalog_matches_disk` green.
- [ ] Deepen `scenario_grammar`: extend `showcase()` past the single
      seed-destroy-assert pass into repeated rounds, and gate each round's
      assertions on the scenario's own variables rather than the
      `elapsed < seeded_at + 1.5` wall-clock settle at
      `examples/gameplay/scenario.rs:381`. Reach further into the grammar the
      config already half-covers - `OnNeutralized`, `OnEnter`/`OnExit` via
      `CreateScenarioArea`, `ObjectiveComplete`, `HudReadout`. Keep the
      `nova_invariants().monotonic([...])` list in step with any new tally.
- [ ] Move `examples/gameplay/playable.rs` -> `examples/systems/player_path.rs`
      with `git mv`, atomic with the same two catalog/smoke edits as above.
      Deepen it: more rounds through the GOTO loop point (the script already
      loops via `reload_the_run:366`), asserting the gesture chain - stance,
      combat lock, kill, travel lock, GOTO, arrive - holds on every round, not
      just the first.
- [ ] Build every fixture LOCALLY, inline in its own example file. Do NOT
      design a shared builder or a `systems/fixtures.rs` module - see
      DECISION.md D4. `20260804-094006` is the third caller and does the
      extraction once all three shapes are visible.
- [ ] Add `examples/systems/outcomes.rs` (plus its `Cargo.toml` block and its
      `SYSTEMS` smoke entry) holding two local `fn`s returning `ScenarioConfig`,
      both `hidden: true`:
      `outcome_probe_a` - `OnStart` spawns the player ship, one killable
      hostile and posts an objective; `OnDestroyed` filtered to the player ->
      `Outcome(Defeat)` + `NextScenario { scenario_id: "outcome_probe_a",
      linger: true }` (the requeue is what makes the Defeat overlay show a
      Retry button at all - `outcome.rs:57`, `primary` is `None` unless
      something is queued); `OnDestroyed` filtered to the hostile ->
      `ObjectiveComplete` + a `VariableSet` latch + `Outcome(Victory)` +
      `NextScenario { scenario_id: "outcome_probe_b", linger: true }` (that
      pair IS the CHECKPOINT - there is no checkpoint type in code).
      `outcome_probe_b` - one object and one variable seeded on `OnStart` so
      the chain's arrival is observable.
      Leave `auto_advance_secs: None` on both (DECISION.md D3).
- [ ] In `outcomes.rs`, on `OnEnter(GameAssetsStates::Loaded)`, INSERT both
      configs into the `GameScenarios` resource before triggering
      `LoadScenario(outcome_probe_a(...))`. Required: the queued switch
      resolves its id against that resource and unloads to the menu on a miss
      (`crates/nova_scenario/src/world.rs:201-220`). See DECISION.md D1. Load
      on `Loaded`, not on `OnEnter(Playing)` - `assert_scenario_loaded` checks
      by `OnEnter(Playing)` and loading in the same schedule is a race
      (`examples/gameplay/scenario.rs:83`).
- [ ] Write `outcomes.rs`'s script as `AutopilotPlugin` `.step()` beats
      (DECISION.md D2), each with a `deadline` so a stall names its step:
      wait for the scenario to seed and `player_ship_present()`; kill the
      player with a `HealthApplyDamage { amount: 1e6 }` overkill on the ship
      ROOT (the production entry point - `examples/gameplay/lifeline.rs:165`);
      wait for `CurrentOutcome == Defeat` AND the `Outcome Overlay` entity
      together, never on the outcome alone - the outcome resource lands a frame
      before the overlay spawns and gating on it is a race (broadside review
      R1.2, `examples/gameplay/broadside.rs:216-222`); trigger
      `Activate { entity }` on the `Outcome Primary Button` entity found by
      `Name`; assert the reload is clean (outcome cleared, hostile back, the
      latch variable zeroed, a fresh player ship); kill the hostile; assert the
      objective completed and that `NovaEventWorld::next_scenario` names
      `outcome_probe_b`; trigger `Activate` again; end on `outcome_probe_b`
      being the loaded scenario.
- [ ] Update the category prose that names the fleet: the per-category comments
      in `Cargo.toml` (the `gameplay/` block is marked TRANSITIONAL, not a
      contract) and the dev wiki category table
      (`web/src/wiki/dev/development.md`).
- [ ] RUN each of the three examples under Xvfb, not just `cargo check` - a
      check misses duplicate-component panics and the run is the deliverable.

## Definition of Done

- The `systems/` fleet completes headlessly and asserts through predicates.
  (cmd: `nix develop --command cargo run -p nova_probe -- run systems`)
- One run drives the composed outcome path in a live app: Defeat overlay ->
  Retry -> clean reload -> Victory + CHECKPOINT -> chain into the next
  scenario. (cmd: `nix develop --command cargo run -p nova_probe -- run outcomes`)
- Fixtures are code-built and reach no shipped scenario data.
  (cmd: `test -d examples/systems && ! rg -n 'assets/base/scenarios|include_str' examples/systems`)
- Both renames landed on disk, with nothing left behind under `gameplay/`.
  (cmd: `test -f examples/systems/scenario_grammar.rs && test -f examples/systems/player_path.rs && ! test -e examples/gameplay/scenario.rs && ! test -e examples/gameplay/playable.rs`)
- The catalog, disk and smoke lists agree after the renames.
  (test: `catalog_matches_disk`)

## Notes

The roster:

- `scenario` -> `systems/scenario_grammar`. Rename + deepen. Already the model:
  code-built `ScenarioConfig` exercising `OnStart`/`OnDestroyed`/`OnUpdate`, filters,
  variables, arithmetic.
- `playable` -> `systems/player_path`. Rename + deepen. Already predicate-driven
  with invariants and probe markers: lock, kill, travel-lock, GOTO, arrive.
- `systems/outcomes` - NEW, and the reason the mainline runs can be retired.

`systems/outcomes` reproduces what `broadside`/`lifeline` carried END TO END.
The four systems themselves are NOT uncovered by the retirement - chaining,
Defeat, Retry reload-clean and Victory/CHECKPOINT are already pinned headlessly
in `crates/nova_menu/src/tests/{outcome,pause}.rs`,
`crates/nova_scenario/src/loader/lifecycle.rs` and
`crates/nova_assets/tests/{broadside_assault,lifeline_convoy}.rs`. What is lost
is the COMPOSED, rendered, click-the-real-button path through all four in one
live app, and that is what this run must reproduce. Size it as an end-to-end
composition, not as emergency coverage. All four are generic; none need story:

- Scenario A (`outcome_probe_a`): one objective, one killable hostile, one
  player ship. `OnDestroyed` on the hostile completes the objective and posts
  Victory with a CHECKPOINT; player death posts Defeat.
- Scenario B (`outcome_probe_b`): the chain target - one object, one trivially
  completable objective, so the chain's arrival is observable.
- Beats: die -> assert the Defeat overlay -> Retry -> assert the reload is
  clean (hostile back, tally zeroed) -> kill the hostile -> assert the
  objective posted and the CHECKPOINT landed -> Continue -> assert B loaded.

Roughly 50 lines of `ScenarioConfig` against the 8000 lines of story RON it replaces,
and it never needs touching when the campaign is rebalanced.

- Sequencing: this must land BEFORE `20260804-093910` deletes broadside and
  lifeline, so the end-to-end composition is never absent from the tree. The
  edge is encoded in `093910`'s `DEPENDS ON`, not just here.
- `systems/` carries no fps window, so these runs are free to be short and
  assertion-dense rather than padded.
- Examples must be RUN under Xvfb :99, not only checked.

### Mechanisms confirmed at plan time

Both of NOTES.md's OPEN questions are answered by code; neither needs a spike.

- How `outcomes` kills the player: `HealthApplyDamage { amount: 1e6 }` on the
  ship ROOT from a world-mutating step callback, the production damage entry
  point. Precedent `examples/gameplay/lifeline.rs:165` and
  `broadside.rs:163`, both of which this run inherits before they are deleted.
  The declarative alternative (a scenario action that damages the player) does
  not exist in `EventActionConfig`.
- What a CHECKPOINT IS: not a type. It is `Outcome(Victory)` plus a queued
  `NextScenario { linger: true }` in the same beat, observable as
  `NovaEventWorld::next_scenario` and released by
  `release_lingering_next()` (`world.rs:334`) - which is exactly what the
  overlay's Continue/Retry `Activate` observer calls
  (`crates/nova_menu/src/outcome.rs`, `on_outcome_advance`). The scenario lint
  warns on `Outcome` + a NON-lingering `NextScenario` in one handler, so
  `linger: true` is required, not stylistic.
- The load-bearing surprise: a queued id is resolved against the
  `GameScenarios` registry, so a code-built chain target must be registered
  there or the chain unloads to the menu. See DECISION.md D1. The story
  examples never needed this - the content bundle merge filled the resource for
  them.

### Proof status on the base branch

- `probe run systems` and `probe run outcomes` both error as unknown specs
  (verified: the catalog lists only sections/gameplay/ui/screenshots/perf). Red.
- The absence proof was VACUOUSLY GREEN as originally written: `rg` on the
  missing `examples/systems` exits 2 and `!` turns that into success. The
  `test -d` guard makes it red on base (verified exit 1) and keeps it honest.
- `catalog_matches_disk` is green on base and stays green when the work is
  done - it is a drift GATE, red only in the window between moving a file and
  updating its catalog/smoke entries. It is kept because that window is exactly
  what the atomic-commit Steps must not leave open; the rename `cmd:` proof
  above is the red-on-base delivery guard beside it.
