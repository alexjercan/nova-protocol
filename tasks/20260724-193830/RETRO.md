# Retro: Campaign content entity - first-class ordered scenario mapping

- TASK: 20260724-193830
- BRANCH: feature/campaign-content-entity
- REVIEW ROUNDS: 2 (R1 out-of-context REQUEST_CHANGES on one doc gap; R2 in-session APPROVE)

Process notes only; the what/why/evidence is in TASK.md close-out, the fork in
DECISION.md, the findings in REVIEW.md.

## What went well

- Reuse-known-good-stack for the wiring: mirroring the existing `Content::Scenario`
  path at every site (merge dup-guard, merge_content_item, MergeOutcome,
  register insert, lint, generation) made a broad change mechanical. `Content`
  has no catch-all `_` arm, so the compiler enumerated every production match site
  for me - a passing build genuinely proves no runtime site was missed.
- The out-of-context reviewer re-ran all five DoD proofs itself and converged on
  exactly the one gap I had already found independently (the modding-ron doc).
  Two independent paths to the same single finding is a good signal the code
  surface was actually clean.
- Referencing the scenario-id CONSTANTS in `build_campaigns()` (not string
  literals) means a future scenario rename can't silently orphan a campaign
  member - and the dangling-member lint backstops it at author time.

## What went wrong

- False-green from `cargo check --all-targets`: the initial verify passed clean,
  but per-crate `cargo test -p` then surfaced broken exhaustive literals/matches
  in FOUR places (nova_scenario loader+world tests, nova_assets balance test, and
  ~9 `tests/*.rs` harness matches). Root cause: those targets only compile under
  each crate's self dev-dep `serde` feature, which a workspace-level `check`
  does NOT enable, so it silently skipped them. Removing a field is as invasive
  as adding one, and the workspace check hid half the blast radius.
- Doc-surface sweep was skipped in the initial /work verify: `modding-ron.md`
  (and two sibling content-model docs) still listed only Section/Scenario kinds.
  Review caught it as the sole finding. Root cause: I treated "new content kind"
  as code-only and didn't grep the content-kind enumerations up front.

## What to improve next time

- When REMOVING or ADDING a field on a widely-constructed serde type, grep the
  FIELD NAME (not just the type name) repo-wide first, and run per-crate
  `cargo test -p <crate> --no-run` on every touched crate before trusting a
  workspace `check --all-targets` - the serde-gated targets are exactly the ones
  the workspace check skips.
- For a new content KIND, run the doc-surface sweep as part of /work: grep the
  content-kind enumerations (`modding-ron`, `guide-make-a-mod`, `scenario-system`)
  in the same pass as the code, not after review.

## Action items

- [x] REVIEW.md R1.1 addressed: Campaign kind documented across the three
  content-model docs.
- [x] ledger: bumped `match-ci-feature-set-in-targeted-tests` (x3, -> Pending
  promotions) with the workspace-check-skips-serde-gated-targets sharpening;
  bumped `keep-docs-in-sync-with-code` with this occurrence.
- No follow-up code tasks: the sibling UI task 20260723-095951 (collapsible
  picker on GameCampaigns) was already queued at plan time and carries the
  hidden-member cold-launch verification.
