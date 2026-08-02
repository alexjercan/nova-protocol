# Tooling: check that every filesystem path named in a comment or doc actually resolves

- PRIORITY: 0
- TAGS: backlog, tooling, chore
- KIND: TASK
- ACTIVITY: -
- GATES: -
- RESOLUTION: WONTDO

## Context

Ledger promotion of `generated-links-need-real-targets` (x5, PROMOTE
2026-08-01; DEFERred at x4 with "delete rotted pointers by hand as each pass
finds them", a disposition that has now failed to keep up twice).

Two shapes of the same rot:

- Pointers to files that no longer exist: manifest-rendered, authored and
  source-comment doc links. A README banner went stale on a dir move; several
  `docs/spikes/*.md` and `DECISION.md` pointers in nova_gameplay HUD comments,
  nova_menu's crate doc and the input layer outlived the files (spike content
  now lives at `tasks/<id>/SPIKE.md`).
- The mirror case: SPLITTING a file rots every `path/to/file.rs` mention
  elsewhere in the tree - 14 of them in 20260731-170340 across five crates and
  the wiki's project-tour table, all missed until review.

## Goal

A check that every filesystem-looking path in a comment or a doc resolves, run
with the other checks.

## Notes

This is the one part of a comment pass that is checkable rather than a
judgment call. See LESSONS.md for the five occurrences.


## Dropped

- REASON: meh
