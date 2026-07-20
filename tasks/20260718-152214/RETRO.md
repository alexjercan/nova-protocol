# Retro: docs-follow-code audit of the dev wiki

- TASK: 20260718-152214
- BRANCH: docs/dev-wiki-audit
- REVIEW ROUNDS: 1 (APPROVE, no findings)

## What went well

- **Fan-out audit by page-cluster.** Three out-of-context Explore agents each
  reconciled a slice of the 13 pages against code and returned exact page:line +
  code:line drift, which made the fixes mechanical. An out-of-context pass beat
  a same-session read: they found the understated verb lists (FlightVerb 5 vs
  doc 3, ROW_VERBS 7 vs doc 6) I would not have thought to re-count.
- **Verified every agent claim against source before writing.** Caught my own
  wrong path guess (`assets/base/balance_acks.ron` -> the real
  `crates/nova_assets/balance_acks.ron`) and confirmed the New-Game mechanism,
  collider enum, and detonation_sound field at their exact lines. `render-output`
  discipline for docs: build (`npm run ci`) + verify each cited symbol.
- **Verified the one code-path change.** The base-dep drop from the example mod
  is data the merge tests load; ran example_scenario (14 pass) rather than
  assume, after confirming no test asserted the declared dep.

## What went wrong

- The task brief itself was stale (referenced `20_perf_baseline`/`21_render_scale_shot`
  and "01-19" numbered examples that the examples-reorg had already renamed to
  category slugs). Not a failure of this cycle - the exact `verify-stale-brief-
  against-tree` lesson promoted last cycle - but it confirms doc-audit briefs go
  stale fastest, since they name the very things that change.

## What to improve next time

- For a wide doc audit, seed each fan-out agent with the recent structural
  changes (here: examples reorg, probe two-verb surface) so it does not re-flag
  intentional history (the "Consolidated over time" line) as drift - which one
  agent nearly did.

## Action items

- [x] Drift list recorded in TASK.md (each drift traced to its causing code
  change) for tightening the release-flow keeping-docs-in-sync step.
- Sibling task 20260718-231555 (scenario vocabulary + patterns) and 231601
  (modding meta-conventions) remain the ADD-what's-missing half; this cycle was
  fix-what's-wrong only, no overlap.
