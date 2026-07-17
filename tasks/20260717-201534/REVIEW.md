# Review: Non-lingering cut for the asteroid_next relay bridges

- TASK: 20260717-201534
- BRANCH: linger-tuning

## Round 1

- VERDICT: APPROVE

Scope: flip `linger: true -> false` on the two `asteroid_next` `OnStart` relay
bridges (RON asset + Rust builtin), a regression test, a docs note, task close.

### What I verified

- **Diff is minimal and on-target.** Only the two bridge flags change; both
  the RON (`assets/base/scenarios/asteroid_next.content.ron`) and the Rust
  builtin (`crates/nova_assets/src/scenario.rs:890`) carry a comment naming the
  task and the rationale. No Outcome-paired transition was touched.

- **Completeness of the audit (the load-bearing claim), independently
  re-derived.** Rather than trust the summary, I re-enumerated every
  `NextScenario` in the repo (base RON, webmods, Rust scenarios, and the
  `assets/mods/example` mod) via a fresh out-of-context pass. Result: 28
  transitions total - 26 have an `Outcome` sibling in the same handler and
  correctly stay `linger: true`; exactly the 2 `asteroid_next` bridges are
  overlay-less and are the ones flipped. No overlay-less transition was missed,
  and no `linger: false` + `Outcome` pairing exists (which would violate
  lint.rs:212-238). The `assets/mods/example` mod and the ledger
  bundle/header `NextScenario` mentions are comments, not actions.

- **Test is meaningful and pins the fix at its boundary.**
  `asteroid_next_bridge_is_a_non_lingering_cut` asserts `!linger` on the
  relay's sole `OnStart` cut. A/B confirmed in the work log and re-checked:
  reverting the bridge to `linger: true` makes the test FAIL at the linger
  assertion (scenario.rs:997), so it would fail with the fix deleted.

- **Lint gate genuinely green.** `content_lint_gate.rs` walks and lints every
  installed scenario including the edited RON; it passes 2/2, so the RON side
  has real regression coverage even though it has no unit test of its own.

- **Docs and TASK.md honest.** `docs/design/scenario-linger.md` states the rule
  (Outcome -> linger:true; overlay-less bridge -> linger:false) and the audit
  result; TASK.md's Outcome/Verification match what the code does.

### Findings

- [ ] R1.1 (NIT) `crates/nova_assets/src/scenario.rs` - the new test covers the
  Rust builtin relay; the RON relay is covered only transitively by the
  content-lint gate (which checks lint-cleanliness, not the `linger` value
  specifically). Optional: a tiny loader-parse assert on the RON's `linger`
  would pin the asset value directly. Not blocking - the gate + the shared
  authoring intent make this low-risk, and the RON is data the lint already
  walks.

No BLOCKER/MAJOR/MINOR findings. The change is correct, minimal, well-tested,
and the "apply where it makes sense" audit resolved to exactly the two
qualifying bridges. Approved.
