# Retro: Add per-OS release download buttons to the landing page hero

- TASK: 20260724-074940
- BRANCH: feat/web-download-buttons
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- Read the release pipeline (`.github/workflows/release.yaml`) and the real
  `gh release view` asset names BEFORE designing, which surfaced the
  version-in-filename fact early and turned it into the DECISION.md pivot (why
  a runtime API fetch, not static `/latest/download/` links). Designing from
  the system, not a model of it.
- The static site has no browser test runner, so the risky logic (asset
  suffix matching) was factored into a pure exported `pickDownloadUrls()` and
  verified at runtime against the REAL latest-release API JSON; the DOM apply
  loop was then exercised end to end with stubbed `document`/`fetch`. Both
  passed, and the chromium screenshot confirmed the styled layout. This beat
  "the build is green" as proof for client-side DOM logic.
- Round-1 out-of-context review APPROVEd with only optional nits; the one
  worth acting on (a coupling reminder in release.yaml) was cheap and landed.

## What went wrong

- The out-of-context reviewer wrote `VERDICT: APPROVE` as a bare line, which
  `tatr check` rejects (it wants the `- VERDICT:` list-item form). Caught by
  running `tatr check` at the compound gate, but it cost a fix-up. Root cause:
  the review subagent was not told the exact VERDICT line format tatr checks.

## What to improve next time

- When spawning an out-of-context review agent that writes REVIEW.md, tell it
  the exact verdict line format (`- VERDICT: APPROVE|REQUEST_CHANGES`) so the
  artifact passes `tatr check` without a fix-up round.

## Action items

- [x] Added a filename-coupling reminder comment to `.github/workflows/release.yaml`
      pointing at `web/src/downloads.ts` (review nit).
- [x] Recorded the review-agent verdict-format lesson in LESSONS.md.
