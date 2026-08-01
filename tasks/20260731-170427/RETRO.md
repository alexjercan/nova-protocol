# Retro: KISS: nova_scenario

- TASK: 20260731-170427
- BRANCH: refactor/kiss-nova-scenario
- REVIEW ROUNDS: 2

## What went well

- Extracting the three oversized files by `sed` LINE RANGE rather than
  retyping them made the moved code byte-identical, so the only thing left to
  review was module headers and the visibility the compiler demanded. Both the
  out-of-context reviewer and the in-session pass could then PROVE the
  pure-move claim: a line-multiset diff of the old files against the new folder
  modules left only `mod` / `use` / `pub use` lines and visibility keywords,
  and the 90 `#[test]` names came out identical sorted.
- The shared test fixtures were hoisted into a `#[cfg(test)] mod fixtures` in
  both `lint/` and `loader/` rather than copied per child, so nothing can
  drift.
- Round 1 found zero correctness findings. Every finding was a doc surface or
  a rustdoc render.

## What went wrong

- The comment pass was written as a SCRIPTED block-level re-wrap (regex strip
  + `textwrap` over each comment block containing a task id). It collapsed a
  rustdoc numbered list in `render_scale.rs` and a `*` bullet list in the
  dispatch bench into run-on paragraphs (R1.3, R1.4). `check`, `fmt` and the
  tests all stayed green; only reading the rendered doc catches it. The ledger
  already holds this exact failure at x5 AND holds the countermeasure at x2
  (`comment-pass-as-asserted-replacements`); the scripted route was chosen
  anyway because 153 provenance sites looked too many to write out by hand.
  That reasoning was wrong in the way the ledger predicts: the script is only
  cheaper if nothing it touches has structure, and comment blocks do.
- The doc-surface sweep never ran. Four dev-wiki recipes walk the reader to
  `crates/nova_scenario/src/{loader,actions}.rs`, files this branch deleted
  (R1.1). `keep-docs-in-sync-with-code` sits at x10 in the ledger and its entry
  names this precise query ("a file SPLIT is the same sweep with a different
  query: every `path/to/file.rs` mention elsewhere in the tree rots").
- A new module header used an intra-doc link, `[`EventActionConfig`]`, from a
  module where the name is only reachable through a glob re-export, so it did
  not resolve and added the crate's only `cargo doc` warning (R1.2).
  `cargo doc` is not in this task's DoD, so no proof would have caught it.
- NOTES.md recorded the largest remaining file at 1080 lines; `wc -l` says
  1070 (R1.6). The number was written from the pre-fmt inventory and never
  re-measured after the last edit.

## What to improve next time

- Take the ledger's countermeasure at face value when its count is already
  high: write a comment pass as N asserted replacements, each requiring its
  anchor to occur exactly once. 153 sites is a long script, not a hard one,
  and it fails loudly instead of silently reflowing a list.
- For a file split, run `grep -rn '<old-path>' web/ README.md AGENTS.md docs/`
  BEFORE opening the review, not after. The query is mechanical and the ledger
  already names it.
- When a task's DoD is about code shape, `cargo doc --no-deps` is still a
  cheap gate for the comment half of the same task. Consider it a default
  proof for any task whose diff is mostly `//!`/`///`.

## Diagnosis

- Breadth: inherently large. One crate, three files over 1500 lines, ~14k
  lines total, and the epic already sized its children one per crate. Nothing
  here was independently landable as a smaller task; splitting the comment
  pass from the structure pass would have doubled the re-wrap surface.
- Churn: the plan is the subject. Both MAJOR findings are recurring ledger
  entries (x10 and x5 at the time of planning), and neither appeared as a
  Step or a proof. The plan-time question that would have caught it: for each
  ledger lesson above x3 that this diff's SHAPE can trip, is there a Step or a
  `cmd:` proof that would fail if it did? A file split trips
  `keep-docs-in-sync-with-code` by construction, and a 150-site comment pass
  trips `doc-comment-rewrap-changes-the-render` by construction.
- Context: no measured pressure. No compaction warning, no checkpoint, no
  handoff. Implementation ran in one pass; round 1 used one out-of-context
  reviewer, round 2 was in-session with the exception recorded.

## Action items

- Ledger bumps below; no follow-up task. The two promotion candidates this
  task pushes to three occurrences (`comment-pass-as-asserted-replacements`,
  `visibility-sweep-narrows-back`) are for the `lessons` user gate to
  dispose of, not this retro.
