# Build systems/: code-built fixtures for scenario grammar, the player path, and outcomes

- PRIORITY: 82
- TAGS: v0.10.0, content, examples, testing
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE
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

- [x] Move `examples/gameplay/scenario.rs` -> `examples/systems/scenario_grammar.rs`
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
- [x] Deepen `scenario_grammar`: extend `showcase()` past the single
      seed-destroy-assert pass into repeated rounds, and gate each round's
      assertions on the scenario's own variables rather than the
      `elapsed < seeded_at + 1.5` wall-clock settle at
      `examples/gameplay/scenario.rs:381`. Reach further into the grammar the
      config already half-covers - `OnNeutralized`, `OnEnter`/`OnExit` via
      `CreateScenarioArea`, `ObjectiveComplete`, `HudReadout`. Keep the
      `nova_invariants().monotonic([...])` list in step with any new tally.
- [x] Move `examples/gameplay/playable.rs` -> `examples/systems/player_path.rs`
      with `git mv`, atomic with the same two catalog/smoke edits as above.
      Deepen it: more rounds through the GOTO loop point (the script already
      loops via `reload_the_run:366`), asserting the gesture chain - stance,
      combat lock, kill, travel lock, GOTO, arrive - holds on every round, not
      just the first.
- [x] Build every fixture LOCALLY, inline in its own example file. Do NOT
      design a shared builder or a `systems/fixtures.rs` module - see
      DECISION.md D4. `20260804-094006` is the third caller and does the
      extraction once all three shapes are visible.
- [x] Add `examples/systems/outcomes.rs` (plus its `Cargo.toml` block and its
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
- [x] In `outcomes.rs`, on `OnEnter(GameAssetsStates::Loaded)`, INSERT both
      configs into the `GameScenarios` resource before triggering
      `LoadScenario(outcome_probe_a(...))`. Required: the queued switch
      resolves its id against that resource and unloads to the menu on a miss
      (`crates/nova_scenario/src/world.rs:201-220`). See DECISION.md D1. Load
      on `Loaded`, not on `OnEnter(Playing)` - `assert_scenario_loaded` checks
      by `OnEnter(Playing)` and loading in the same schedule is a race
      (`examples/gameplay/scenario.rs:83`).
- [x] Write `outcomes.rs`'s script as `AutopilotPlugin` `.step()` beats
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
- [x] Update the category prose that names the fleet: the per-category comments
      in `Cargo.toml` (the `gameplay/` block is marked TRANSITIONAL, not a
      contract) and the dev wiki category table
      (`web/src/wiki/dev/development.md`).
- [x] RUN each of the three examples under Xvfb, not just `cargo check` - a
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

## Close-out

### What and why

`examples/gameplay/` is now `examples/systems/`: `scenario` and `playable`
moved with `git mv` and were deepened into repeated rounds, and `outcomes` is
new - the composed Defeat -> Retry -> clean reload -> Victory + CHECKPOINT ->
chain arc, on two code-built `ScenarioConfig` fixtures registered into
`GameScenarios` (D1). `broadside`/`lifeline` stay behind under a TRANSITIONAL
`gameplay/` block for `20260804-093910` to delete, so the composed outcome path
is never absent from the tree.

One production change came out of it: `nova_probe`'s monotonic invariant now
forgets its memory on `ScenarioLoaded` (D6). That is a real bug, not test
scaffolding - a replaying example was guaranteed a false `monotonic_regression`
at every round boundary, and dropping the invariant from `player_path` instead
would have traded a correct one-way-latch claim for silence.

### Alternatives

- Keeping `monotonic` off `player_path` entirely. Rejected: the latch IS
  one-way within a scenario life, and the checker's reset rule was simply
  shaped for teardown rather than reload.
- Aiming the overkill at the object root only, and giving the hostile a ship
  body so root damage lands. Rejected: it would have hidden the propagation
  rule (D6) behind a fixture choice, and the asteroid is the cheaper hostile.
- `with_main_menu(true)` to get the overlay. Rejected: it also takes the
  boot-into-MainMenu handoff, which this run must not take.

### Difficulties and diagnosis

Three failures, none of which `cargo check` or `reached_playing` can see. All
three were diagnosed by RUNNING the fleet and reading the step names in the
stall lines, which is precisely the argument the Steps made for running rather
than checking:

- `die to the overkill` stalled with `CurrentOutcome == Defeat` the whole time.
  A temporary `.each()` diagnostic printing every `Outcome*`-named entity showed
  the list empty, which pointed at plugin composition rather than at the
  scenario: `with_game_plugins` had disabled `NovaMenuPlugin`.
- `kill the hostile` stalled with no `on_destroyed_entity` line at all. The
  asteroid's own docs name the cause - the root is a marker carrier, `Health`
  lives on the collider child, and `HealthApplyDamage` only propagates upward.
- `player_path` completed and still failed 5/6 on two `monotonic_regression`
  entries. Reading `check_invariants` showed the reset keyed on a key VANISHING,
  a gap a reload never leaves.

### Evidence

- `probe run systems` - OK: `scenario_grammar` OK, `player_path` OK,
  `outcomes` OK (5/6 each; the sixth is `fps_within_baseline`, SKIPPED, as
  `systems/` carries no fps window by contract).
- `probe run outcomes` - OK on its own: process_exit, run_completed,
  reached_playing, invariants_held (0 violations / 52 frames), log_clean.
- `cargo test --test examples_smoke systems_reach_playing_without_panic` - ok.
- `cargo test --test examples_smoke catalog_matches_disk` - ok.
- `cargo test -p nova_probe --lib invariants` - 12 passed, including the new
  `a_gapless_reload_resets_monotonic_memory`.
- Both `cmd:` shell proofs exit 0; `cargo fmt --check` clean;
  `cargo check --examples --features debug` clean.
- Not run locally: the full workspace suite (CI owns it).

### Reflection

The Step that said "RUN each of the three examples under Xvfb, not just
`cargo check`" earned its place three separate times in one task - every bug
here was invisible to a check and to `reached_playing`, and two of them would
have shipped an example that proves nothing while reporting a stall nobody
reads. The generalizable form is narrower than "run it": a predicate that
gates on a RESOURCE alone (`CurrentOutcome`) passes on a build with no UI at
all, so `outcome_overlay_up`'s insistence on the resource AND the entity is
what turned a vacuous pass into a nameable stall. Worth keeping in mind for
`20260804-094021`, which asserts on rendered UI throughout.

### Round 1 feedback

All nine findings fixed; see REVIEW.md for the per-finding responses. Eight
were the doc sweep, and they share one root cause worth naming: the Step that
owned the sweep enumerated two files (`Cargo.toml`'s comments and the wiki
category table) instead of naming the grep. Everything missed sat outside that
list - three wiki pages citing the deleted `examples/gameplay/scenario.rs`, a
probe SKILL.md left self-contradictory by its own partial edit, an invariants
paragraph naming two dead examples, and no CHANGELOG line for a rename that
breaks `cargo run --example playable`. A half-swept doc is worse than an
untouched one: the stale half still reads as current.

Two findings changed the code rather than the prose:

- `outcome_overlay_up` now waits on `Outcome Primary Button`, not
  `Outcome Overlay`. The reviewer read this as the D6 frame-lag race one level
  down; it is not - both spawn in one `commands.spawn(...).with_children(...)`
  batch, so they land together. The real gap is that the button is CONDITIONAL
  on something being queued, so a fixture that forgot its lingering
  `NextScenario` would have panicked inside the next beat's `activate_named`
  rather than stalling on a named step. Waiting on the button the beat
  presses is right for the second reason, not the first.
- `outcomes` gained a `report the defeat overlay` beat, so the log line its
  module doc told readers to grep for actually exists. The doc block was
  written from the intended shape, not the emitted one - the same class of
  error as a self-ticked proof, caught only because the reviewer grepped the
  run log for every string the doc promised.

Re-verified after the fixes: `probe run systems` OK 3/3 (5/6 each), the full
`cargo test --test examples_smoke` suite 8/8, `cargo test -p nova_probe --lib
invariants` 12 passed, `cargo fmt --check` clean, both shell proofs exit 0, and
the doc sweep re-grepped clean (the only survivors are synthetic manifest
strings inside `catalog.rs`'s own unit tests). Both doc-promised grep strings
confirmed live in `probe-runs/abde17e3/`.
