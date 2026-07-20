# Review: group --baseline (per-example baseline root, skip missing)

- TASK: 20260720-125322
- BRANCH: fix/probe-group-baseline

## Round 1

- VERDICT: APPROVE

Small, self-contained probe feature; compile clean, unit-tested resolver, and an
e2e proving both the compare and skip paths. Re-derived the load-bearing claims:

- **The single-example path is untouched.** Verified the dispatch: `run_spec`
  returns `run(&base)` for `!resolved.multi` (baseline = the run dir, as before,
  probe.rs:445), and only the multi path reaches `run_many` (probe.rs:459) where
  `--baseline` is re-resolved per example against the root. So the two documented
  meanings of `--baseline` (run dir for one example, root for a group) fall out
  of the existing split - no risk to the single case.
- **Skip is a SKIP, not an error.** `run_many` sets `opts.baseline` only when
  `group_baseline_for` finds a `frametime.csv`, so `run()`'s pre-run baseline
  validation never trips on a group miss; the example just runs with no baseline
  and its `fps_within_baseline` stays SKIPPED. E2e confirmed: scenario (baseline
  removed) -> SKIPPED + the per-skip log, group exit 0.
- **Compare works by label match.** playable found `before/playable/frametime.csv`
  and compared (`fps_within_baseline: PASS`, "worst playable: +1.4%") - the
  per-example label (the example name) matches across runs automatically, so no
  extra label plumbing was needed.
- **Resolver is pinned.** `group_baseline_for` unit test covers present (Some),
  dir-without-csv (None), and missing-dir (None).

- [ ] R1.1 (NIT) The "baseline root matched NONE of the group" warn
  (`baseline_matches == 0`) is code-verified but not exercised by the e2e (the
  e2e hit the partial-match case: one present, one missing). The logic is a
  trivial counter check and the per-example skip path IS e2e'd, so coverage is
  adequate; a wrong-root run would just log the warn and skip everything. Take
  it or leave it.
