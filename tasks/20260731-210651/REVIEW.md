# Review: Make the local test suite runnable: cap link jobs and drop test-binary DWARF

- TASK: 20260731-210651
- BRANCH: master (already landed in fa0e227a; corrections in ac70dba8)

## Round 1

- REVIEWER: in-session (post-landing closure audit; no implementation context in this session)
- VERDICT: APPROVE

- No findings.
- Verified the jobs cap, test-profile DWARF removal, and AGENTS.md `--lib`
  guidance against the landed diff and current tree.
- Re-derived the formerly blocking external fact: the isolated shakedown test
  now passes, so the historical unrelated failure no longer prevents closure.
- The original full-suite run measured an 8.19 GiB peak and completed all 64
  binaries without exhausting RAM. Its exit 101 was solely the now-green
  shakedown test.
