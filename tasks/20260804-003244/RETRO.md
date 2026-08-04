# Retro: Spike: decide the v0.10.0 example fleet roster

- TASK: 20260804-003244
- BRANCH: (none - spike artifacts only, landed on master)
- REVIEW ROUNDS: 2

## What went well

Reading the code before answering the charter's question is what made the
spike worth running. The charter asked how to cope with story scenarios an
autopilot cannot win; ten minutes in the examples showed both already win, with
named assists at specific lines. Surfacing that BEFORE proposing a roster meant
the owner decided against a real premise rather than a assumed one, and the
decision that followed (examples test systems, not stories) was theirs on
corrected facts.

Asking the taxonomy question rather than guessing it. The owner's answer -
"maybe `gameplay` is not the right target, IDK think about it" - was an
explicit delegation, and pairing the recommendation with a cheap-to-reverse
cost check (category strings are only load-bearing in N places) was the right
shape of answer. That the cost check was wrong is the next section; the shape
was right.

## What went wrong

**The "ONLY evidence" claim was manufactured, and it sized a task.** The spike
asserted that `broadside`/`lifeline` were the sole coverage for chaining,
Defeat, Retry-reload and Victory/CHECKPOINT. All four were already pinned in
`nova_menu`, `nova_scenario` and `nova_assets` tests. Root cause: the charter
warned "retiring a run that is the only evidence for a system is a regression;
name what covers it afterwards", and that framing was answered by naming a
REPLACEMENT without first checking whether the premise held. The warning was
read as a task to discharge rather than a claim to verify. The claim then
justified `systems/outcomes`, the largest new task in the sprint - an
unverified premise doing load-bearing sizing work.

**`tests/examples_smoke.rs` was never opened.** The spike declared "renaming
categories is cheap" from a `grep` for category string literals in
`crates/nova_probe/`. A 339-line root-level integration test hardcodes every
category list and gates a bare `cargo test` through `catalog_matches_disk`. The
search was scoped to where the answer was expected to be, and the conclusion
was drawn from its absence elsewhere. This is the same failure as the "only
evidence" one: absence of found evidence read as evidence of absence, twice.

**`Content` is not a type.** The load-bearing rule sentence - "test scenario
content is built as a `Content` value in Rust" - named a type that does not
exist, propagated into three documents and a task others would implement from.
It came from `screenshot_reel`'s doc comment ("the same `Content` type the
modding loader uses"), which is itself stale. A doc comment was trusted as an
API reference without opening the crate.

**Unmeasured premise.** "Mainline story scenarios are the most volatile content
in the repo" was stated as fact and used to reject three options. `git log`
says 11 and 6 commits ever. The measurable version was one command away and
weaker than the claim.

**The status-quo option was never on the ballot.** The alternatives table
opened at "keep the arcs AND add harness documentation" - strictly more work
than doing nothing. Presenting only options that involve work is how a spike
launders a preference into a finding.

## What to improve next time

- Breadth: not applicable in the usual sense - no production code. But the
  spike's OUTPUT breadth (7 tasks, a category rename touching a CI-gated test)
  was under-scoped exactly where it was under-researched. The two dropped
  `20260802-120029` Steps and the missing `examples_smoke.rs` owner all sat in
  the part of the tree the spike never read.
- Churn: both review rounds trace to one plan-time question that was not asked:
  *for every claim of the form "X is the only Y", what search would falsify
  it?* Four of the ten review items were falsifiable by one `rg` over
  `crates/*/tests`. A spike that retires things should carry a standing
  refutation pass over its own exclusivity claims before it recommends.
- When closing a task SUPERSEDED, diff its Steps and DoD against the
  successors' before closing, not after. Closing `20260802-120029` silently
  deleted two epic Done Means and both of its named proof tests; the review
  caught it, but nothing structural would have.
- Never cite a type name from a doc comment. Open the definition.
- The out-of-context review was the single highest-value step in this task and
  found what self-review could not, because it was not anchored to the
  reasoning that produced the errors. Worth spending on any spike whose output
  is a document rather than a diff, where there is no compiler to disagree.

## Action items

- None filed as tasks. All ten review findings were applied to the spike
  artifacts in round 2; the seven successor tasks carry the corrected evidence,
  Steps and proof-bearing DoDs.
- Observation submitted to the central knowledge repository: verify
  exclusivity claims by refutation search before they size downstream work.

## Landing message

```
docs(task): settle the v0.10.0 example fleet roster

Examples test systems, not stories. The spike found the charter's premise
stale - broadside and lifeline already drive full lose/retry/win arcs with
teleport and clock-jump assists - so the question became what an assisted win
over 8000 lines of story RON buys. Owner's call: mainline scenarios get no
example coverage and are tested by players; examples cover systems on
story-free fixtures built as ScenarioConfig in Rust.

Five categories replace the old five: sections, systems, stress, ui,
screenshots. gameplay/ retires, perf/ is absorbed into stress/, and stress/
becomes the only category carrying a frame-time window. Net fleet 26 -> 25.

Supersedes 20260802-120029 and seeds seven successors with Steps and
proof-bearing DoDs. An out-of-context review corrected the spike's central
evidence: broadside/lifeline were NOT the only coverage for chaining, defeat,
retry-reload and victory - all four are already pinned headlessly, and what
the retirement loses is the composed end-to-end path.
```
