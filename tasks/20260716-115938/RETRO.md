# Retro: Document the release + code->docs sync workflow

- TASK: 20260716-115938
- BRANCH: changelog-revamp
- REVIEW ROUNDS: 0 (self-verified: build green, all referenced pages/crates
  exist, ASCII clean - the "review" for a reference doc is checking its
  references resolve, done inline)

## What went well

- The dependency map was verified mechanically, not by eye: grepped every wiki
  `.md` name and `nova_*` crate the page references and checked each exists (22
  pages + 11 crates, zero dead references). For a doc whose whole value is
  pointing at the right files, that check is the equivalent of a test.
- Placing it right: durable dev reference -> a published wiki page (per the
  repo's own docs/README convention), enforcement rule -> AGENTS.md, ledger
  reminder -> LESSONS. Each fact in one home, cross-linked, no duplication of
  the detailed release steps that already live in development.md.

## What went wrong

- Nothing broke, but the task confirmed the problem it documents: the /news/
  merge two tasks earlier had left "devblog" wording stale in AGENTS.md and
  development.md's release step 8. The docs-sync obligation was real and had
  already been missed once - which is the argument for writing the map down.

## What to improve next time

- When a task retires or renames a user-facing surface (blog -> news), grep the
  whole repo for the old name (`blog`, `devblog`, `changelog`) including
  AGENTS.md and the dev wiki, not just the code and the section being changed.
  The merge task's stale-reference sweep covered web/src but not AGENTS.md.

## Action items

- [x] `keep-docs-in-sync-with-code` recorded in docs/LESSONS.md.
- [x] Fixed the stale "devblog" references found in AGENTS.md + development.md.
