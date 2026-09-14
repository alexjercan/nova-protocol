# Audit the codebase for duplicated algorithms and logic

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog, refactor, quality

Audit the workspace for duplicated algorithms and logic that must be changed
in more than one place. Identify and record first; change nothing yet.

## Scope

- In scope: `crates/*/src/**`, the real codebase.
- Out of scope: `examples/`, `crates/*/examples/`, `benches/`, fixtures, and
  test scaffolding. Duplication there is acceptable by design.
- Target: the same algorithm or complex decision implemented twice. Not one
  or two repeated lines, not boilerplate a derive already settles.

## Question the audit answers

If I change this behavior in one place, must I change the same behavior
somewhere else for the game to stay consistent?

Priority order:
1. UI interaction logic. One example: does closing a window with `Esc` run
   the same code as closing it with a click, or two separate paths?
2. Two features that are supposed to behave the same, implemented apart.
3. Shared utilities re-written per call site: file and RON reading, id
   lookup, unit conversion, formatting, timers, spatial queries, math.

## Method

Four reviewers in two cross-checking pairs, two waves. Each pair reads the
same chunk by two different methods, structural and behavioral, so that each
finding is either confirmed or contradicted by a second reader.

## Deliverable

`FINDINGS.md` in this folder: each duplicate family with `path:line` for every
copy, a short excerpt, what breaks if only one copy changes, and a proposed
single home. No code change until the owner selects findings.

## Step 1 complete: findings recorded

`REPORT.md` is the summary. `FINDINGS.md` holds every family with `path:line`
evidence, excerpts, proposed homes, refutations and coverage.

Ten reviewer-passes in five pairs over `crates/*/src/**`. Counts: 4 BLOCKER,
24 MAJOR, 24 MINOR, 16 families confirmed NOT duplicated, 11 claims rejected.

BLOCKERs, each already inconsistent in the tree:

1. `rebind-policy-four-surfaces` - four rebind surfaces, four conflict
   policies. A console `bind` is persisted then silently reverted at next
   launch; the in-ship panel drops a section's gamepad binding while the
   editor asserts the opposite.
2. `scroll-driver-fork` - `nova_os_ui` re-implements the `nova_ui` scroll stack
   it imports; wheel step 20.0 against the shared 60.0.
3. `terminal-command-error-vocabulary` - the same typo in the same CRT gets two
   different answers depending on which shell is open.
4. `scenario-name-vocabulary-two-tables` - lint match and editor reflect
   attributes are two tables for one question; a dangling id in
   `SetInfiniteAmmo` passes lint and dies silently at runtime.

Two findings were corrected DOWNWARD after the owner challenged them, and are
recorded as such: the weapon-class fire sequence (shared code is two statements
calling already-shared helpers; the "railgun drift" claim is withdrawn, the
railgun renders as a streak and needs no easing seed) and the menu panel close
(four ~13-line handlers; the missing Escape is a UX gap, not duplication).

Nothing was changed. Next step is the owner's selection of which families to
fix; each has a proposed home and signature in `FINDINGS.md`.

