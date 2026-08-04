# Build systems/: code-built fixtures for scenario grammar, the player path, and outcomes

- PRIORITY: 82
- TAGS: v0.10.0, content, examples, testing
- KIND: STORY
- ACTIVITY: PLANNING
- GATES: -
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

- [ ] Rename `examples/gameplay/scenario.rs` -> `examples/systems/scenario_grammar.rs`
      and deepen it: more of the event grammar, predicate-gated assertions,
      repeated rounds. Atomic with its `tests/examples_smoke.rs` const edit.
- [ ] Rename `examples/gameplay/playable.rs` -> `examples/systems/player_path.rs`
      and deepen it: more of the gesture chain, more rounds through the loop
      point.
- [ ] Build the fixtures LOCALLY, inline in `systems/`. Do NOT design a shared
      builder: owner call 2026-08-04, `20260804-094006` is the third caller and
      does the extraction once all three shapes are visible. One caller is not
      an abstraction.
- [ ] Add `examples/systems/outcomes.rs` with the `outcome_probe_a` /
      `outcome_probe_b` pair and its beats: die, Defeat overlay, Retry, clean
      reload, kill, objective + CHECKPOINT, Continue, B loaded. Advance the
      overlay by TRIGGERING the button's `Activate` on the target entity, not
      by `click_at` on screen coordinates - owner call 2026-08-04. This run's
      subject is the outcome CHAIN; the menu is `ui/`'s subject, and coupling
      this run to overlay layout would make the sprint's most fragile test
      more fragile still.

## Definition of Done

- The `systems/` fleet completes headlessly and asserts through predicates.
  (cmd: `nix develop --command cargo run -p nova_probe -- run systems`)
- One run drives the composed outcome path in a live app: Defeat overlay ->
  Retry -> clean reload -> Victory + CHECKPOINT -> chain into the next
  scenario. (cmd: `nix develop --command cargo run -p nova_probe -- run outcomes`)
- Fixtures are code-built and reach no shipped scenario data.
  (cmd: `! rg -n 'assets/base/scenarios|include_str' examples/systems`)
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
