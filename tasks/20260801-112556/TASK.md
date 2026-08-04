# Tooling: re-runnable measured numbers in task records

- PRIORITY: 0
- TAGS: backlog
- ACTIVITY: -
- GATES: -
- RESOLUTION: WONTDO

## Context

Ledger promotion of `re-measure-records-after-the-last-edit` (x4, PROMOTE
2026-08-01). A record holding measured numbers - line counts, `file:line`
inventories, diff totals, grep counts - goes stale the moment ANY later edit
touches the measured files. Four occurrences so far, all caught in review, all
the same shape: the number was correct when first measured and nobody
re-measured after the round's last edit.

Occurrences: 20260731-170335, 20260731-170359, 20260731-170340,
20260731-170345.

The insight the ledger already carries: every one of these numbers comes from
ONE command (`wc -l`, `grep -c`, `git show <base>:<file> | grep -c`). So the
record should carry the command rather than the transcribed result, and a
check should re-run it.

## Story

As an agent writing NOTES.md and REVIEW.md I want measured numbers to be
re-derived rather than transcribed, so a later edit in the same round cannot
silently falsify a record I have already verified.

## Open questions for planning

- Where does the command live - an inline fenced form in the record, a
  sidecar file, or a `tatr`-owned block? The record must stay readable as
  prose after the numbers are filled in.
- Is the check a `tatr check` arm (fails conformance on a stale number) or a
  render step run before commit? The first is a gate, the second is a fixer.
- Does the already-promoted comment-diff check (20260801-102759) absorb part
  of this? Both re-derive something about the diff; decide before building.
- Scope: is it worth covering `file:line` markers (which shift on any rewrap)
  or only aggregate counts?


## Dropped

- REASON: meh
