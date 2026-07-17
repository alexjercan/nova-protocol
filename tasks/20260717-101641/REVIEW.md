# Review: Per-target impact and destroy sounds

- TASK: 20260717-101641
- BRANCH: task-20260717-101641-impact-destroy-sounds

Reviewed the committed diff (76abfbec) fresh + independent out-of-context pass.
Independently verified: torpedo detonation marks the PROJECTILE (the snapshot's
host - the walk finds the detonation voice; a shot-down torpedo's silent child
sections match the designed no-blast-on-shootdown behavior); the walk cannot
over-match (ship roots never carry ImpactDestroySounds); the damage-propagation
original-target guard still holds; base sections (7x2) + all 21 base-scenario
asteroids author both voices; editor-sandbox direct paths are correct (runtime-
built outside the merge); suites 540/89/4 green + workspace all-targets clean.

## Round 1

- VERDICT: REQUEST_CHANGES

- [x] R1.1 (BLOCKER) webmods content gap: gauntlet (10 asteroids) + the-ledger
  chapters (5) author no impact/destroy sounds - their rocks would go silent
  when the bank keys are deleted. The repo-wide sweep covered assets/ +
  examples/ + editor but missed webmods/ content (the reel-miss class,
  LESSONS: sweep-content-repo-wide-not-just-assets).
  - Fix: author `dep://base/sounds/impact.wav` + `.../explosion.wav` on every
    webmod asteroid.
  - Response: Done - 15 asteroids wired across 4 webmod content files;
    content_lint_gate + webmods_validation green (the dep://base refs are
    declared base resources and actually load).

## Round 2

- VERDICT: APPROVE

R1.1 verified: every webmod asteroid now authors both voices; gates green. No
new findings - all other review dimensions passed round 1.
