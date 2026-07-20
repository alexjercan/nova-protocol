# Review

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

What I tried to break: the key risk here is a lesson marked PROMOTED whose
claimed home does not actually carry the guidance, so I ran the honesty check
adversarially. I ran the DoD command against the worktree's ledger with the
tatr build the task pinned and confirmed `promotion-stalled` returns 0 (the
"Pending promotions" section is now the empty sentinel). For each of the five
folded lessons I diffed the AGENTS.md rule text against the ledger annotation
it claims to satisfy, hunting for a vague gesture that fails to capture the
lesson. prose-from-diff-not-intent, verify-stale-brief-against-tree,
render-output-eyeball (keeps the header-only-table counterexample),
authored-vs-derived-values, and advertised-but-unwired are all present in the
"Promoted ledger lessons" block and each captures its ledger entry faithfully,
not a hollow restatement. I checked the block's placement: it is the last
`###` subsection of `## Conventions` (line 136, before `## Shared-checkout
discipline` at 156), i.e. the end-of-Conventions home the task asked for. For
out-of-context-review-pass (no fold), I did not take "already /flow round-1
practice" on faith - I read the review skill and found it codifies "Round 1
comes from out of context by default" with a fresh reviewer that has not seen
the implementing session, so the annotation is factually accurate for an x31
positive practice. The Promoted section in the ledger keeps all six entries
annotated as the paid record, and emptying Pending promotions is correct since
none were retired.

- No findings.
