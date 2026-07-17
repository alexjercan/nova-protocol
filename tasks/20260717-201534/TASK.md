# Non-lingering cut for the asteroid_next relay bridges

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.7.0,scenario,content,pacing

## Goal

Apply `linger: false` to the scenario transitions where it actually makes
sense: the two `asteroid_next` relay bridges. Every other `NextScenario` in
the project is deliberately paired with a Victory/Defeat `Outcome` overlay and
must stay `linger: true` (the lint enforces this). Making the empty relay a
non-lingering cut removes the current stray Enter press when the asteroid loop
recycles.

## Background (audit result)

`NextScenarioActionConfig.linger` defers a queued scenario switch so an
`Outcome` overlay stays on screen until the player presses Continue/Enter
(`release_lingering_next`, `crates/nova_scenario/src/world.rs:250`). The switch
only fires once the flag is cleared (`world.rs:124`, filter `|r| !r.linger`).

The lint at `crates/nova_scenario/src/lint.rs:212-238` flags an `Outcome` +
non-lingering `NextScenario` in one handler as a trap (the instant switch
swallows the overlay; a delayed one freezes under the overlay's pause). So
`linger: false` is only correct in handlers with NO `Outcome` - pure bridge
cuts.

A full sweep (RON assets, webmods, Rust-coded scenarios) found every
`NextScenario` is Outcome-paired EXCEPT the `asteroid_next` relay, which
exists in two forms and today both use `linger: true`:

1. `assets/base/scenarios/asteroid_next.content.ron` - `OnStart ->
   NextScenario(asteroid_field, linger: true)`, no other content.
2. `crates/nova_assets/src/scenario.rs:883` - the built-in `asteroid_next`
   scenario, same `OnStart` bridge, `linger: true`.

Effect today: clearing `asteroid_field` shows a Victory overlay; Continue
switches to the empty `asteroid_next`; because its `OnStart` bridge lingers
with no overlay to dismiss, the player must press Enter AGAIN to escape the
empty relay back to `asteroid_field`. `linger: false` makes the relay an
immediate cut - one acknowledgement, seamless loop.

## Steps

- [x] In `assets/base/scenarios/asteroid_next.content.ron`, set the `OnStart`
      `NextScenario` to `linger: false`.
- [x] In `crates/nova_assets/src/scenario.rs` (the built-in `asteroid_next`
      `Scenario`), set that `OnStart` `NextScenario`'s `linger` to `false`.
      Left the Defeat and Victory transitions in that file at `linger: true`.
- [x] Add a regression test in `crates/nova_assets/src/scenario.rs`
      (`asteroid_next_bridge_is_a_non_lingering_cut`) asserting the built-in
      `asteroid_next` bridge transition is non-lingering, mirroring the
      filter-events pattern in
      `crates/nova_assets/src/scenario/shakedown.rs:1462`.
- [x] Confirm the content-lint gate stays clean
      (`crates/nova_assets/tests/content_lint_gate.rs`, which walks and lints
      every installed scenario including the edited RON).
- [x] Add a note in `docs/design/scenario-linger.md` recording the audit:
      what `linger` does, why only the `asteroid_next` bridges qualify for
      `linger: false`, and that all Outcome-paired transitions stay
      `linger: true` per the lint.

## Notes

- Relevant files: `crates/nova_scenario/src/actions.rs:522` (the config +
  `delay` doc), `crates/nova_scenario/src/world.rs:120-142,250-263` (switch +
  release), `crates/nova_scenario/src/lint.rs:212-238` (the Outcome+non-linger
  guard), `crates/nova_assets/src/scenario.rs:729-895` (built-in asteroid
  scenarios), `assets/base/scenarios/asteroid_next.content.ron`.
- Scope guard: do NOT touch any transition that shares a handler with an
  `Outcome` (all of broadside, shakedown, the ledger chapters, gauntlet,
  asteroid_field's win/lose). Those are correct as `linger: true`.
- `delay` is not needed here: the relay is an invisible reset, so an instant
  cut is what we want (no world-keeps-playing beat).
- Per AGENTS.md: skip local cargo test/clippy; run check/fmt and the newly
  written test only, report honestly. CI runs the full suite.

## Outcome

Flipped the two `asteroid_next` relay bridges to `linger: false`
(`assets/base/scenarios/asteroid_next.content.ron` and the built-in in
`crates/nova_assets/src/scenario.rs`). These are bare `OnStart ->
NextScenario(asteroid_field)` relays with no `Outcome` overlay; a lingering
switch there stranded the player in the empty relay scenario until a stray
Enter press, so the endless asteroid loop cost two acknowledgements per lap
instead of one. `linger: false` makes the relay an immediate cut.

Every other `NextScenario` in the project is paired with a Victory/Defeat
`Outcome` in the same handler and correctly stays `linger: true` - the lint
(`crates/nova_scenario/src/lint.rs:212-238`) forbids flipping those, and doing
so would swallow the overlay. So the "apply where it makes sense" audit
resolved to exactly these two bridges; nothing else qualified.

### Verification

- New unit test `scenario::tests::asteroid_next_bridge_is_a_non_lingering_cut`
  passes (1 passed). A/B: with the bridge reverted to `linger: true` the test
  FAILS at the linger assertion (scenario.rs:997), proving it pins the fix at
  its own boundary; restored via `git checkout` off the committed fix.
- Content-lint gate `content_lint_gate.rs` passes (2/2) - the whole installed
  tree, including the edited RON, lints clean (the bridges carry no `Outcome`,
  so no new warning).
- `cargo fmt --check -p nova_assets` clean. Full suite left to CI per AGENTS.md.

### Reflection

Most of the work was the audit, not the edit: the change itself is two flags,
but the value was proving that only the overlay-less bridges qualify and that
the lint already encodes the rule. Reading world.rs/loader.rs/lint.rs before
touching anything (per `verify-engine-guarantees-in-source`) is what turned a
vague "apply linger:false somewhere" into a bounded, defensible change. Next
time, reach for the existing content-lint gate immediately as the RON-side
regression rather than wondering whether to hand-roll one.
