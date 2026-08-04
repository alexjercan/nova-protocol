# Review: Spike: decide the v0.10.0 example fleet roster

- TASK: 20260804-003244
- BRANCH: (none - spike artifacts only, landed on master)

## Round 1

- REVIEWER: out-of-context
- VERDICT: REQUEST_CHANGES

Reviewed by a fresh agent with no exposure to the spike's
reasoning, against the artifacts and the tree. Ten changes required. Every
factual claim in the review was independently re-verified before acting on it;
all ten held.

The three that were real defects rather than polish:

1. **"broadside/lifeline are the ONLY evidence" was false.** All four systems
   (chaining, Defeat overlay, Retry reload-clean, Victory/CHECKPOINT) already
   have headless coverage in `crates/nova_menu/src/tests/{outcome,pause}.rs`,
   `crates/nova_scenario/src/loader/lifecycle.rs` and
   `crates/nova_assets/tests/{broadside_assault,lifeline_convoy}.rs`. The spike
   manufactured a coverage cliff that then justified its largest new task.
   Corrected to the accurate and narrower claim: what the retirement loses is
   the COMPOSED, rendered, click-the-real-button path, not the systems.
2. **`tests/examples_smoke.rs` was missed entirely.** 339 lines hardcoding the
   category layout, four per-category const lists, `NOT_SMOKED`, per-category
   `#[test]` fns and `catalog_matches_disk` as a drift gate under a bare
   `cargo test`. This falsified "renaming categories is cheap" and makes each
   directory rename atomic with its edit to that file. Now owned by
   `20260804-093855`.
3. **Three of `20260802-120029`'s Steps fell on the floor** when it was closed
   SUPERSEDED, two of them epic Done Means: the per-example hack deletion
   (`playing_since` is still live in three `screenshots/` runs) and the
   full-fleet evidence report. The latter had no owner at all; it is now
   `20260804-095507`.

Also corrected: the fixture type is `ScenarioConfig`, not `Content` (no such
type exists - the name came from a stale doc comment in `screenshot_reel`, and
it was the load-bearing noun in the rule sentence); `LoadScenario` line cites
pointed at the `ScenarioConfig` literal rather than the trigger; lifeline
handler count 24 -> 32; the roster arithmetic (DECISION.md computed 24, not
25); the missing status-quo option; and the unmeasured "most volatile content
in the repo" premise, which git contradicts - 11 and 6 commits ever, four
commits in history touching an example and story content together.

Sequencing was prose-only and is now encoded in `DEPENDS ON`, with three edges
the spike had missed (`093950 -> 094006` shared ship builder; `093855 ->
093910`; `003301 -> 094021` for the `*_poc.html` end-state). The `*_poc.html`
move was double-owned; it belongs to `20260804-003301`.

## Round 2

- REVIEWER: out-of-context
- VERDICT: APPROVE

All ten changes applied and re-verified against the tree. The spike's
conclusion is unchanged - the taxonomy, the code-built fixture rule and the
retirement all survive the corrections - but its evidence now matches the
repository, and the seven successor tasks carry Steps and proof-bearing DoDs
rather than prose intent.

Approved on the corrected document.
