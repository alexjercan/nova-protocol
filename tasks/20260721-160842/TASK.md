# Resolve asteroid_field hidden-vs-wiki contradiction (picker sandbox or hidden?)

- STATUS: CLOSED
- PRIORITY: 56
- TAGS: v0.8.0,scenario,docs

## Story

The spike (tasks/20260721-155249/SPIKE.md) found the shipped
asteroid_field.content.ron carries `hidden: true` while the player wiki
scenarios.md still describes it as a picker-visible sandbox. One of them is
wrong: either the hiding was an unintended regression, or the wiki is stale.
Small verify-first task so the docs edits in the ch3 tasks build on a
correct scenarios.md.

## Steps

- [x] Verify-first: `git log -p --follow -S "hidden" -- crates/nova_assets/src/scenario.rs`
      (and the generated RON) to find when and why asteroid_field got
      `hidden: true`; record the evidence (commit + reason) in this file.
- [x] Decide from the evidence: intentional -> fix web/src/wiki/scenarios.md
      (and any other surface listing the picker) to match; regression ->
      unhide in the asteroid_field builder and `content gen`.
- [x] Sweep the doc surfaces for the picker list (web/src/wiki/scenarios.md,
      tutorial page, README) so every surface agrees with the shipped flag.
- [x] Run `cargo run -p nova_assets --bin content -- lint`; parity tests if
      the RON changed.
- [x] CHANGELOG entry only if player-visible behavior changed (unhide).

## Definition of Done

- The shipped flag and every doc surface agree
  (cmd: `grep -n "hidden" assets/base/scenarios/asteroid_field.content.ron`;
  cmd: `grep -n "Asteroid Field" web/src/wiki/scenarios.md`).
- The decision and its git-history evidence are recorded in this file
  (cmd: `grep -n "Decision:" tasks/20260721-160842/TASK.md`).
- content lint green (cmd: `cargo run -p nova_assets --bin content -- lint`).

## Notes

- Spike: tasks/20260721-155249/SPIKE.md (Open questions).
- Flow umbrella: 20260721-160425.

## Record (2026-07-21)

Decision: UNHIDE - the hiding rationale was never true; the flag was wrong
at introduction (corrected in review round 1 - the first-pass record here
misread the history; see REVIEW.md R1.1).

Evidence: `git blame` puts `hidden: true` on asteroid_field at baf56811e
(2026-07-15, "feat(menu): Scenarios picker to play any registered
scenario"), comment "a mid-story stage reached by chaining from the
shakedown run". That premise was FALSE in every committed state: shakedown's
NextScenario targets across its whole history are only shakedown_run (its
own retry) and broadside (pickaxe on shakedown.rs; every committed revision
of shakedown_run.content.ron has zero asteroid_field occurrences). The real
history: asteroid_field was the ORIGINAL New Game scenario
(NEW_GAME_SCENARIO_ID = "asteroid_field") and 24491209 swapped New Game to
shakedown_run - my first-pass pickaxe hit was that constant swap, over-read
as "shakedown chained into it". Since then NOTHING referenced asteroid_field
except its own asteroid_next relay loop (verified: grep of NextScenario
targets across assets/base/scenarios, webmods/, assets/mods). Result:
finished sandbox content hidden from the picker AND unreachable from any
chain - dead content - while the player wiki (scenarios.md line 29) kept
advertising it as the combat/gravity sandbox.

What changed:

- crates/nova_assets/src/scenario.rs: asteroid_field drops `hidden: true`,
  gains the sibling placeholder thumbnail (`self://banner.png`) and a
  comment recording this history. asteroid_next stays hidden (it is a
  relay, correctly so).
- `content gen`: asteroid_field.content.ron regenerated (the only RON that
  changed).
- CHANGELOG Unreleased, Scenarios & Objectives: the sandbox is back in the
  picker.
- Doc sweep: scenarios.md needs no edit (its description is true again);
  news/dev-wiki mentions checked, none claim visibility state.

Also in this diff: `cargo fmt` (the documented pre-commit ritual) healed
pre-existing formatting drift on master in four files untouched by this
task (hud/readout.rs, probe/bin/probe.rs, probe/catalog.rs,
scenario/actions.rs - drift landed with recent HudReadout/probe commits).
Formatting-only; noted so the reviewer does not attribute those hunks to
this change.

Verification: content lint 0 errors (1 pre-existing Ledger WARN + 2
pre-existing acks); `cargo test -p nova_assets --test content_ron_parity
--test broadside_assault --test example_scenario` all green (27 tests, 28
after the review-round pin); `cargo check` green; full clippy/test suite
left to CI per repo policy. The visibility pin
(`the_sandbox_is_listed_and_its_relay_is_not`, review R1.2) is
sabotage-proven: with master's RON (hidden: true) checked out it fails at
broadside_assault.rs:571 ("the sandbox is a Scenarios-picker entry"); with
the branch RON it passes.

Reflection: the in-code comment cited a chain that NEVER existed, and my
first-pass record repeated the error by over-reading a pickaxe hit instead
of opening the commit - the out-of-context reviewer caught it (R1.1).
Lesson applied: a `-S` hit names a commit that TOUCHED the string, not what
it did with it; open the diff before writing history. The verify-first step
still turned what looked like a docs fix into reviving orphaned content,
and the corrected history (original New Game scenario, never mid-story)
makes the unhide strictly better grounded.
