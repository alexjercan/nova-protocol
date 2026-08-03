# Retro: Document nova_autopilot: rustdoc, prelude, and the dev wiki page

- TASK: 20260802-183355
- BRANCH: docs/autopilot-docs
- REVIEW ROUNDS: 2

## What went well

- The prelude DoD was written as a test, not a reading, and the review pushed
  that further: `every_module_is_scanned` closes the hole the first version
  left (a fifth `pub mod` with public items). Enforcement, not prose.
- Round 1's eight findings were all fixed at root cause rather than caveated,
  including the two MAJORs that were about what the page teaches rather than
  what it omits.
- Two nits deferred by `20260802-183349` and one by `20260802-183352` were
  cleared here as planned; the crate now has no outstanding docs debt.

## What went wrong

- The wiki page shipped its first draft describing the post-migration world as
  present fact: a `hold(GameStates::Playing, ...)` opt-in snippet that
  `nova_debug/src/harness.rs` documents at length as the bug to avoid, a
  `cargo run --example scenario` command that is inert until `20260802-183403`
  lands, and a "Nova uses it for" table column for usage that does not exist.
  Root cause: writing docs for a freshly EXTRACTED crate, where the crate's
  contract and the repo's current wiring are two different worlds, and nothing
  in the plan or the DoD forced a choice between them. The plan anticipated the
  env-naming half of this (Notes correctly scoped the rename to
  `20260802-183403`), which made the tense feel already handled.
- The same defect recurred in round 2 (R2.2): R1.7's enforcement-wording fix
  landed in `lib.rs` but not in the wiki page that repeats the same claim. A
  fix applied to one of two artifacts stating one fact.

## What to improve next time

- Docs for an extracted crate need an explicit tense rule at plan time: every
  runnable command and every "X uses it for" claim is either true on the branch
  or labelled with the task that makes it true. Cheap in the plan, two MAJORs
  at review.
- When a review finding is about a claim, grep the claim rather than edit the
  file. R1.7 and R2.2 are one finding found twice.

## Context

- No compaction warning, threshold crossing or handoff during this task. Both
  review rounds were delegated to out-of-context subagents, which kept the
  primary's context to the diff and the findings.

## Knowledge

- Both observations were already covered centrally, so no new slug was created.
  Occurrences added to `docs/document-observed-behavior` (the tense defect) and
  `docs/update-restatements-with-the-source` (the claim restated in two
  artifacts). `knowledge check` clean.
- Process note: `knowledge` in this environment has no `--repo` flag and
  resolves the repository from the cwd. Run from the current checkout it
  silently writes an untracked `lessons/` shadow tree there and exits 0, and
  `knowledge list` then lists only that tree. Run it with the cwd set to
  `/home/alex/personal/agent-knowledge` and confirm with `knowledge list`
  (54 lessons, not 1). The shadow tree written here was removed.

## Action items

- None open. R2.1 and R2.2 were fixed on this branch before close-out (wiki
  page wording), the web build re-verified green (`npm run ci` exit 0), and
  their Responses recorded in REVIEW.md. No follow-up task seeded;
  `20260802-183403` already owns the migration this page points at.
