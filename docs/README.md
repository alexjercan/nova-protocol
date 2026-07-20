# Nova Protocol docs/

`docs/` is **ephemeral scratch space**. During a development cycle, write
whatever working notes, investigations, or design sketches you like here - no
structure required. At every release tag the folder is compiled down and wiped,
so the only things that survive are the two permanent files:

- **[LESSONS.md](LESSONS.md)** - the lessons ledger, the durable record of the
  repo's paid-for mistakes. Read it before starting work; `/compound` appends to
  it. This is where a scratch note's lasting *insight* goes.
- **README.md** (this file) - describes the model.

Everything else under `docs/` is transient.

## The two durable homes

Durable knowledge has exactly two homes, and neither is a `docs/` junk drawer:

- **The wiki** (`web/src/wiki/`, published at `/wiki/`) - REFERENCE: how the
  code and systems work, at full detail (architecture, dev workflow, the
  scenario/section/modding guides). A scratch note whose substance is
  reference-grade gets migrated into a wiki dev page, not left in `docs/`.
- **LESSONS.md** - LESSONS: one-or-two-line distilled insights with task ids.

## Release: compile, then wipe

At release time (before tagging):

1. **Distill** everything worth keeping out of `docs/` scratch - lessons into
   `LESSONS.md` (the `/compound` format), reference detail into the wiki. A
   script cannot summarize free-form notes into good lessons, so this step is
   yours.
2. Run **`scripts/wipe-docs.sh`** - clears everything under `docs/` except
   `LESSONS.md` and this `README.md`. Idempotent (a no-op on an already-clean
   `docs/`).
3. The **release-flow guard** (`scripts/check-docs-clean.sh`, run by
   `.github/workflows/release.yaml`) FAILS the tag build if `docs/` still holds
   anything else - so a release can never ship a junk-drawer `docs/`.

## Where records go

- Anything tied to one task lives in that task's folder: `tasks/<id>/TASK.md`,
  `SPIKE.md`, `REVIEW.md`, `RETRO.md`, `NOTES.md`. A `grep`/`ls` of
  `tasks/<id>/` shows the whole story. Do not create per-task record files under
  `docs/`.
- **Plans are tatr tasks**, not `docs/plans` files (that folder is retired). A
  release plan is a task with the strand breakdown in its body (or a parent
  `meta`/`release` task linking the per-strand tasks); `/plan` and `/flow`
  produce tatr tasks directly.
- A durable, cross-cutting design record that used to live in `docs/design` now
  lands in the wiki (if reference) or `LESSONS.md` (if a lesson) in the cycle it
  matters - `docs/` keeps nothing durable of its own.
