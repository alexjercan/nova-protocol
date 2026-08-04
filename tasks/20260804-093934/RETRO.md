# Retro: Build systems/: code-built fixtures for scenario grammar, the player path, and outcomes

- TASK: 20260804-093934
- BRANCH: feat/systems-examples
- REVIEW ROUNDS: 2

## What went well

The Step that demanded RUNNING each example under Xvfb rather than
`cargo check` paid for itself three times inside one task: a disabled
`NovaMenuPlugin` that left no overlay to click, an asteroid whose `Health`
lives on a collider child so root damage never landed, and a false
`monotonic_regression` on every reload boundary. None was reachable by a check,
and two would have shipped an example that proves nothing while stalling in a
line nobody reads.

The sharper generalization, worth more than "run it": a predicate that gates on
a RESOURCE alone passes on a build with no UI at all. `outcome_overlay_up`
insisting on the outcome AND a live entity is what converted a vacuous pass
into a nameable stall.

Both DECISION.md deviations (D5, D6) were recorded rather than silently ticked -
the `arrive` clause of Step 3 is literally undelivered, and the argument for
relocating that coverage is on the record where a reviewer could contest it.
The round-1 reviewer accepted it because it was visible.

## What went wrong

The doc sweep. Eight of nine round-1 findings were stale or self-contradictory
prose, and they share one root cause: the Step that owned the sweep enumerated
two files - `Cargo.toml`'s comments and the wiki category table - instead of
naming the grep. Everything missed sat outside that list: three wiki pages
citing the deleted `examples/gameplay/scenario.rs`, a probe SKILL.md left
self-contradictory by its own partial edit (one line updated to
`scenario_grammar`, a table two paragraphs above still saying
`gameplay/scenario`), an invariants paragraph naming two dead examples, and no
CHANGELOG line for a rename that breaks `cargo run --example playable`.

A half-swept doc is worse than an untouched one. The stale half still reads as
current, and the freshly-edited half lends it credibility.

Two smaller misses of the same shape: `player_path.rs` was renamed but did not
rename itself (`#[command(name = "playable")]`, every log line still prefixed
`playable:`), and `outcomes.rs`'s module doc told readers to grep for a line
the file never emitted. The second is the same class as a self-ticked proof -
documentation written from the intended shape rather than the observed one -
and it was caught only because a reviewer grepped the run log for every string
the doc promised.

## What to improve next time

- A rename Step owns a GREP, not a file list. The literal text should be the
  command: `rg -n '<old-name>' --glob '!tasks/**'`, run and shown clean, not
  "update the category prose in X and Y".
- Never enumerate doc surfaces by hand when a mechanical search exists. The
  files this task missed were all findable by the same one-line search that
  eventually found them.
- Doc blocks that promise a grep string are a claim about observed output.
  Verify them against a real run log before commit, the same way a `cmd:` proof
  is run rather than reasoned about.
- When a review finding's stated MECHANISM is wrong but its suggested change is
  right, fix it and correct the reasoning on the record. R1.7 was read as the
  D6 frame-lag race one level down; it is not (overlay and button spawn in one
  command batch). The real gap is that the button is CONDITIONAL on a queued
  scenario. Shipping the fix under the wrong rationale would have seeded a
  false belief about the menu's spawn timing.

## Diagnosis

**Breadth.** The diff is large (~1800 insertions) but not wrongly scoped. Two
renames-plus-deepenings and one new composed end-to-end run were a single
coherent unit: the `systems/` category only becomes a contract when all three
exist, and `20260804-093910` cannot delete `broadside`/`lifeline` until the
composed outcome path exists somewhere else. Splitting `outcomes` out would
have left a window with no end-to-end coverage in the tree. The plan already
deferred the fixture-builder extraction to `20260804-094006` (D4), which is the
split that mattered.

**Churn.** Both review rounds were spent almost entirely on documentation, and
the plan-time question that would have prevented it is not the from-scratch
challenge - the design was right - but the cold-reader test applied to the
SWEEP rather than the code. The plan asked "which prose names the fleet?" and
answered with two files. It should have asked "what does a reader who greps the
old name find tomorrow?", which is a search, not an inventory. Note the plan
did encode one production change correctly: the `nova_probe` monotonic fix was
found by running, recorded in D6, and pinned by a new unit test with a paired
live-regression assertion.

**Context.** No compaction warning, checkpoint, or handoff occurred. Both review
rounds were delegated to out-of-context subagents as the review skill requires,
which kept the reviewing context bounded while the recording pass re-derived
load-bearing claims independently - including one the round-2 reviewer got
wrong (it reported five beat markers per `outcomes` cycle; the timeline shows
six, because `beat: kill` repeats as well as `beat: activate`). Independent
re-derivation earned its cost in both rounds.

## Action items

- Fold "a rename Step names the grep, not the files" into the plan skill's Step
  guidance when a task next renames a public surface. Not filed as a task: it
  is guidance, and it is submitted as a knowledge observation below.
- `cargo test --test examples_smoke` depends on an ambient `DISPLAY` and fails
  on `:0` while passing under `:99`. No DoD proof is wrong (the `test:` proof
  is `catalog_matches_disk`, which needs no display), but the dependency is
  undocumented and the next example task will trip on it. Worth a follow-up if
  it bites again; not filed now, since the fleet is mid-restructure and
  `20260804-093910` and `20260804-094006` will both touch this file.

## Landing message

```
feat(examples): build systems/ on code-built scenario fixtures
```

Replace `examples/gameplay/` with `examples/systems/`: story-free fixtures
built as `ScenarioConfig` values in Rust and loaded with `LoadScenario`, so the
compiler catches scenario-grammar changes and no shipped story content is
reachable from an example.

`scenario` becomes `systems/scenario_grammar` and `playable` becomes
`systems/player_path`, both deepened from a single pass into repeated rounds
gated on the scenario's own variables rather than a wall-clock settle. New
`systems/outcomes` walks the whole outcome arc in one live run - die, Defeat
overlay, Retry, clean reload, kill, objective + CHECKPOINT, Continue, chained
scenario - on two ~50-line fixtures registered into `GameScenarios`, replacing
what 8000 lines of campaign RON carried incidentally.

Also fixes a real `nova_probe` bug the new round loops exposed: a registered
monotonic is one-way within a scenario life, not for the process, so the
memory is now forgotten on `ScenarioLoaded`. A reload overwrites variables in
place and never leaves the vanished-key gap the old reset waited for, so every
replaying example was guaranteed a false `monotonic_regression`.

`cargo run --example scenario` / `playable` and `probe run scenario` /
`playable` are gone - use the new names.
