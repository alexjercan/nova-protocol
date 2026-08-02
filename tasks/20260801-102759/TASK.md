# Tooling: diff comment text base-vs-branch so a comment pass cannot silently damage rustdoc

- PRIORITY: 0
- TAGS: backlog, tooling, chore
- KIND: TASK
- ACTIVITY: -
- GATES: -
- RESOLUTION: WONTDO

## Context

Ledger promotion of `doc-comment-rewrap-changes-the-render` (x4, PROMOTE
2026-08-01). Four KISS passes in a row have damaged rustdoc through scripted
comment substitution, and every instance was invisible to `cargo check`,
`clippy`, `cargo fmt` and the tests:

- a rewrapped `- ` list collapses into one paragraph;
- a following line starting `-`/`#`/`>`/`1.` becomes a block construct;
- a hyphenated code span split across lines gains a space;
- a code span gets spliced (`a `.chain()`` -> `a.chain`);
- a doc line is duplicated;
- a live task ID is deleted out of a deferred-work note.

The damage is mechanical and only visible as a DIFFERENCE from the base text,
which is why a lint over the result keeps missing it.

## Goal

A check that compares comment text (or the rendered doc output) before and
after a change, so a comment pass has to look at what it altered.

## Notes

The epic 20260731-170222 has further comment passes queued; landing this
before them is the point. See LESSONS.md for the four occurrences.


## Dropped

- REASON: meh
