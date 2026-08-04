# Decision: examples test systems, not stories

- DATE: 20260804-094039
- STATUS: ACCEPTED
- TASK: 20260804-003244
- TAGS: v0.10.0, examples, testing

## Context

The v0.10.0 fleet rebuild had to settle what the 26 examples should contain.
The framing question was whether an autopilot could drive the mainline story
scenarios to a real win. It can, and already does: `broadside` (11 beats) and
`lifeline` (19 beats) both play a full lose -> Retry -> win -> chain ->
campaign-complete arc today, using a `teleport_player` assist and a
`jump_clock` fast-forward whose gates then fire for real.

That made the interesting question a different one: what does an assisted win
over 8000 lines of story RON prove, and what does it cost when the story
changes?

## Decision

**Examples test SYSTEMS on purpose-built fixtures. Story scenarios get no
example coverage and are tested by players.**

Three consequences follow, all accepted here:

1. **Test scenario content is built in code** - a `ScenarioConfig` value in Rust
   (`crates/nova_scenario/src/loader/mod.rs:147`), loaded with `LoadScenario`,
   never shipped under `assets/` and never authored as example-owned RON.
   `examples/gameplay/scenario.rs:89` and `playable.rs:167` already do this; the
   rule promotes their precedent. (There is no `Content` type - the name in
   `screenshot_reel`'s doc comment is stale.)
2. **Five categories**: `sections/`, `systems/`, `stress/`, `ui/`,
   `screenshots/`. `gameplay/` is retired (it was never a contract, just a
   leftover) and `perf/` is absorbed into `stress/`, so every frame-time claim
   has one home.
3. **`stress/` is the only category that carries an fps window.** That is what
   frees the others to be short and assertion-dense rather than padded to fill
   a window.

The retired story runs' SYSTEM coverage - scenario chaining, the Defeat
overlay, Retry reload-clean, Victory/CHECKPOINT - is generic, and all four are
ALREADY pinned headlessly in `crates/nova_menu/src/tests/{outcome,pause}.rs`,
`crates/nova_scenario/src/loader/lifecycle.rs` and
`crates/nova_assets/tests/{broadside_assault,lifeline_convoy}.rs`. What the
retirement loses is the composed, rendered, click-the-real-button path through
all four in one live app. That composition moves to one code-built
`systems/outcomes` fixture pair: roughly 50 lines of `ScenarioConfig` against
the 8000 lines of story RON it replaces.

## Alternatives considered

- **Status quo: keep `broadside`/`lifeline` unchanged, zero work.** The
  cheapest live option, and the honest baseline. Rejected because the two runs
  are ~800 lines proving systems that headless tests already pin, they are the
  sole reason `fps_exempt` exists, and they block the category cleanup - not
  because they are a maintenance emergency. Measured churn is low: 11 commits
  ever on `broadside.rs`, 6 on `lifeline.rs`, and only four commits in history
  touching an example and story content together, most churn coming from the
  harness migration this sprint completes.
- **Keep the assisted arcs, legitimize `teleport`/`jump_clock` as a documented
  harness vocabulary.** Rejected: strictly worse than the status quo - the same
  coupling of ~800 lines to story data, plus documentation work.
- **Split each mainline into a cheap smoke run plus a deep assisted arc.**
  Rejected: widens the fleet and still pins the deep half to story data.
- **Downgrade mainline runs to "prove the game plays normally" plus perf**,
  the original task's proposal. Rejected as a halfway house - a run that boots
  a story scenario and drives its opening objectives is still coupled to story
  content, for weaker evidence than a fixture gives.
- **Example-owned `.content.ron` + `include_str!`** (the `screenshot_reel`
  pattern) for fixtures. Rejected: exercises the RON parse path, but a grammar
  change then breaks at runtime mid-run instead of at compile time. The loader
  path stays covered by shipped scenarios on every real boot and by
  `screenshot_reel`'s embedded load.
- **Ship fixtures under `assets/base/scenarios`.** Rejected: they would appear
  in the player-facing Scenarios picker and in the web build.
- **Keep `perf/` beside `stress/`**, separating "our shipped scenes run fast"
  from "the engine scales to N". Rejected in favor of one home for frame-time.
- **Keep the `gameplay/` name with a rewritten contract.** Rejected: the name
  keeps implying the story runs being removed, which is what made the category
  vague to begin with.

## Consequences

- The campaign loses its automated end-to-end regression. A story-breaking
  change to `broadside`/`lifeline`/`final_tally` content will now reach
  players rather than CI. Accepted deliberately: that regression was bought
  with a maintenance cost on every rebalance, and the systems it exercised are
  covered by fixtures either way.
- `fps_exempt = ["broadside"]` disappears - a wart that existed only because
  one story run was too heavy to profile.
- Net fleet 26 - 3 retired - 2 merged + 4 added = 25. Deeper, not wider.
- The category rename is not free: `tests/examples_smoke.rs` hardcodes the
  category lists and gates a bare `cargo test` via `catalog_matches_disk`, so
  every directory move must be atomic with its edit to that file.
- Sequencing constraint: `systems/outcomes` must land before the retire task
  deletes `broadside`/`lifeline`, or the tree briefly has no evidence for
  chaining, defeat, retry or victory.
- Code-built content makes builders reusable `fn`s, so `stress/many_sections`
  shares the `sections/` ship builder with a count knob. That reuse is the
  practical payoff of choosing code over RON.
