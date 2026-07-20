# Retro: portal client - fetch + staged install/uninstall

- TASK: 20260715-163508
- BRANCH: feature/portal-fetch (landed on master as 11e2ef89)
- REVIEW ROUNDS: 2 (R1 APPROVE with findings addressed, R2 APPROVE)

## What went well

- The 142906 review artifacts paid forward exactly as designed: R1.4's
  request-vs-commit caveat (written as a comment addressed to this task)
  shaped the wasm commit implementation, and R1.7's open note became this
  task's uninstall semantics - review findings as inter-task contracts.
- The real-wire e2e (tiny_http serving a nova_portal_gen tree of the real
  webmods, real ehttp GETs) means the full portal pipeline - generator ->
  HTTP -> staged verify -> cache -> load -> merge - is exercised end to end
  by one test file, on every CI run.
- Seven sabotage runs across the two commits; the reviewer independently
  re-ran one and it bit on the exact assertion. The sabotage-matrix habit from
  142906 is now the delegated-implementation norm.
- ehttp 0.7.1 compiled clean on both targets first try (rustls native, fetch
  wasm) - the fallback plan was never needed.

## What went wrong

- R1.1 recurred a known theme at a NEW layer: paths were validated as local
  Path components but not as URL segments - `%2e%2e` is Normal locally and a
  dot-dot on the wire. Third occurrence of boundary-semantics mismatches this
  family (existence-vs-membership, write-vs-read-back, now local-vs-wire
  meaning). The general rule: a validation gate must check the meaning the
  value has in EACH domain it crosses into, not the domain it was written in.
- The wasm uninstall/reinstall race (R1.2) came from mixing a detached async
  removal with synchronous admission state - the same shape as the mark-system
  subtlety in 142906. Async-half/sync-half state machines need their in-flight
  sets modeled explicitly.

## What to improve next time

- When a value crosses representation domains (fs path -> URL -> IDB key),
  enumerate the domains in the plan and pin a validation test per domain.

## Action items

- [x] LESSONS.md: `validate-membership-not-existence` sharpened into the
  cross-domain rule (third occurrence -> flagged for promotion).
