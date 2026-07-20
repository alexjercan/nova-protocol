# Retro: Auditor bay mount + section-overlap lint

- TASK: 20260717-151208
- BRANCH: fix/auditor-bay-mount (landed 32c3e60)
- REVIEW ROUNDS: 1 (APPROVE; 1 MINOR + 3 NIT)

## What went well

- Guard-first sequencing: writing the generic overlap lint BEFORE the
  content fix turned the shipped bug into its own fail-first (4 errors on
  the real tree, zero elsewhere) and closed the whole authoring class,
  not one instance. The lint's repo-wide run doubled as the sweep.
- The reviewer parsed the GLB's vertex data to verify the mount-base and
  hatch axes, catching that the fix ALSO cured torpedoes spawning inside
  the dorsal turret's cube - evidence-grade review paying beyond its
  brief (probe-surfaces-adjacent-issues, review-side).

## What went wrong

- R1.1: a python heredoc's continuation line baked ~26 literal spaces
  into a Rust string; fmt cannot see string contents, so it shipped to
  the committed lint output and review had to catch it. Scripted edits
  that build STRING LITERALS across continuation lines need their
  produced text grepped (verify-scripted-edits-applied's output-side
  variant - I verified the edit applied, not what it said).

## What to improve next time

- After any scripted edit that constructs a user-visible string, echo the
  produced line and READ it, not just the replace count.

## Action items

- [x] LESSONS.md: bumped verify-scripted-edits-applied (x4 ->
  pending stays) with the string-literal-contents variant.
