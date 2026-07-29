# RETRO - 20260729-002952 (probe FPS regression investigation)

## What this cycle actually was

A measurement cycle, not a code cycle. The task was opened expecting a bisect
across four suspect commits. It closed with zero commits checked out, because
the artifacts needed to answer it were already on disk - three commit-keyed
`probe-runs/` sets bracketing the regression window, produced as a side effect
of other tasks.

## What went well

- **Reading the raw artifacts first paid for itself.** The task's own DoD
  insisted on `frametime.csv` / `checks.json` / `trace.json` over the HTML
  reports. Parsing all nine CSVs into one table took one script and immediately
  showed HEAD at parity with the pre-regression baseline. The HTML reports would
  have shown per-run deltas against whatever baseline each run picked, which is
  a different and much more confusing question.
- **The confounder got checked instead of assumed away.** `a6d06220` is itself
  the commit that changed the probe measurement environment, and the operator's
  profile is genuinely non-default (`graphics_quality: High`, two enabled mods).
  That is a textbook way to fake a perf win. Two greps settled it: the CSVs
  record `quality=default` on all three runs, and no run log ever mentions the
  mods. Cheap to check, and the whole conclusion depended on it.
- **The owner's instinct was right and the evidence was there to confirm it.**
  "The upfront asset loading fixed it" held up under `git log -S ttc` and a trace
  scan showing the custom loader gone.

## What went wrong

- **A number got stated without being counted.** NOTES claimed "36 further
  commits" between the middle run and HEAD; the real count is 15. Nothing in the
  session produced 36 - it was an eyeball estimate of a `git log` output that
  then got written down as a fact, in a document whose entire value is that its
  numbers are checkable. The review caught it only because the review re-derived
  every number rather than reading the prose. Every other figure in the document
  came from a parser; this one came from a glance, and it is the only one that
  was wrong. That correlation is the lesson.
- **The reviewer was in-context.** Flow wants an out-of-context reviewer for
  round 1; session constraints ruled that out. Re-deriving claims from artifacts
  is a decent substitute for a numeric document, but it cannot catch what a fresh
  reader would catch in the framing. Recorded in REVIEW.md rather than papered
  over.

## What to do differently

- **If a number goes in a document, a command produced it.** Not "roughly", not
  "about". `git rev-list --count A..B`, not a look at the log. This is a
  narrower and more actionable rule than "be careful": the failure mode is
  specifically the mixing of parsed numbers and eyeballed numbers in one
  document, where the parsed ones lend unearned credibility to the eyeballed
  one.
- **When a perf comparison spans a commit that touched the harness, name the
  confounder before reading the numbers.** It worked here because the check
  happened early. Had the sandbox actually changed what was measured, every
  conclusion drawn before that check would have needed throwing out.

## Follow-ups

- Backlog task `20260729-205957`: end-of-sprint probe perf sweep as a standing
  per-sprint check. This regression shipped, sat, and was fixed by an unrelated
  task before anyone measured it - the investigation was archaeology. A routine
  sprint-close sweep converts that into a same-sprint catch with a handful of
  candidate commits instead of dozens.

## Candidate lesson (for LESSONS.md)

- `numbers-in-docs-come-from-commands` - in any document whose value is that its
  figures are checkable, every figure must be the output of a command, not an
  estimate read off a screen. Mixing the two lets the parsed figures vouch for
  the guessed one. This cycle: 9 CSV-derived numbers were all correct; the one
  eyeballed commit count was wrong by 2.4x.
