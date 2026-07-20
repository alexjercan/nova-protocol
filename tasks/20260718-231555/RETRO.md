# Retro: scenario patterns + promote Gauntlet to the worked example

- TASK: 20260718-231555
- BRANCH: docs/scenario-patterns-gauntlet (landed 1594ec99)
- REVIEW ROUNDS: 1 (out-of-context APPROVE, 1 MINOR fixed)

See TASK.md for what was documented; process only here.

## What went well

- The grooming note's "80% already done" claim was VERIFIED, not trusted: the
  subagent grepped each already-covered sub-item before writing, ticked the real
  ones as covered-by-wiki, and caught that one grooming claim was stale (the
  per-spawn audio fields were documented on a section's base block, not on the
  Asteroid scenario-object surface the Gauntlet rocks use) - and added them in
  the right place, confirmed against `AsteroidConfig`.
- The out-of-context reviewer verified every quoted pattern excerpt VERBATIM
  against the content file with line numbers (the `gate` variable, the
  Equal/LessThan expression syntax, the FINISH/Defeat guards). For a docs task
  whose whole value is quoting real syntax, that is the only review that counts.
- Applied last task's lesson: committed REVIEW.md on the branch BEFORE
  `sprout land`, so it rode the squash (visible in the landed file list) instead
  of being lost with the worktree.

## What went wrong

- The gauntlet mod README still said "published by nova_portal_gen" and
  "v1.1.0" - the first is drift the portal-port task (20260718-152247) left
  behind (its doc sweep updated the wiki + top-level README but not the mod's
  own README), the second a stale version. Both surfaced only because this task
  happened to edit the same README. Root cause: a tool-rename doc sweep that
  covers wiki/README/AGENTS still misses per-mod READMEs and content-file header
  comments.

## What to improve next time

- When renaming a tool/command, extend the doc-surface sweep to
  `webmods/*/README.md` and content-file header comments, not just the wiki +
  top-level README + AGENTS - grep the OLD name across the WHOLE tree
  (`grep -rn <oldname>`), including non-served docs.

## Action items

- [x] LESSONS.md: bumped `keep-docs-in-sync-with-code` with this instance and a
  "sweep per-mod READMEs + content headers, grep the old name tree-wide" clause.
- [x] Fixed the two gauntlet README drifts in this branch (version + generator).
