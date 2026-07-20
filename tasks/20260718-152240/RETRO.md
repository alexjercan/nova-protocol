# Retro: content lint - merge balance audit into lint + per-mod located report

- TASK: 20260718-152240
- BRANCH: feature/content-lint-report (landed 279f7bc3)
- REVIEW ROUNDS: 1 (APPROVE; findings were all stale-doc MINOR/NIT)

See TASK.md for what changed and the evidence rig; this is process only.

## What went well

- Round 1 APPROVE from an out-of-context reviewer with only doc-mention
  findings - the design, correctness, tests and exit-code faithfulness held up
  to a fresh skeptic. The out-of-context default earned its keep here by being
  the thing that read every `//!` doc comment I skimmed past.
- Reused the balance code path instead of duplicating it: extracted
  `audit_bundles_to_audits` as the single audit body, fed the whole repo by the
  gate and the walked target set by the report, so a `--target` audit and the
  tree audit provably agree. No second balance computation to drift.
- Checked shipped content's ACTUAL bindings before choosing the input-overlap
  severity - found turrets fire on `RightTrigger2`, not the flight rig's
  `RightTrigger` bumper - so the new check flags zero shipped content and no CI
  gate falsely reddens. Deciding severity from the real data, not the theory.
- The flight-rig drift guard builds the REAL `flight_input_rig()` and set-diffs
  its queried sources against the hand-authored reserved list both directions,
  so the lint's copy of the rig's keys is self-correcting rather than a list
  that silently rots.

## What went wrong

- The doc-surface sweep missed three in-source mentions of the removed `audit`
  subcommand: the `balance.rs` module `//!`, the `nova_assets` crate `//!`, and
  a "three subcommands" count in development.md prose (the code block below it
  WAS trimmed). Root cause: I swept the obvious markdown surfaces (README, wiki
  pages, AGENTS, CHANGELOG) and the bin's own docs, but not the OTHER crates'
  module/crate doc comments that also describe the CLI surface. Markdown-shaped
  thinking, not diff-shaped.
- Two self-caught fixture stumbles (minor): a `content_report` unit test set
  `bundles: ["m"]` while its helper hardcoded bundle "the-ledger" so the finding
  filtered out of the rendered table; and the fixture RON omitted the
  non-defaulted `infinite_ammo` field, a parse-panic round-trip. Both cost a
  minute, neither reached review.

## What to improve next time

- When a change RENAMES or REMOVES a command/symbol, the doc-surface sweep must
  grep the source doc comments too: `grep -rn '<oldname>' --include='*.rs'`
  catches `//!`/`///` mentions that a README/wiki/AGENTS sweep never sees. A
  CLI-surface description lives in module docs as often as in the manual.
- Author fixture RON by copying a known-good shipped block and trimming, rather
  than hand-writing from the struct definition - skips the missing-required-
  field round-trip.

## Action items

- [x] LESSONS.md: added `doc-sweep-covers-source-doc-comments` (x1) and
  `pin-mirrored-list-against-source` (x1, positive).
- No follow-up code work: the task landed complete. Coordinates with the still-
  OPEN 20260719-092952 (removes the `gen` subcommand), which will shrink the
  `content` bin to a single `lint`.
