# Nova Protocol docs/

`docs/` is **ephemeral scratch space**. During a development cycle, write
whatever working notes, investigations, or design sketches you like here - no
structure required. At every release tag the folder is compiled down and wiped,
so the only thing that survives here is this README, which describes the model.

Everything under `docs/` except this file is transient.

## The durable home

Durable project reference has one home, not a `docs/` junk drawer:

- **The wiki** (`web/src/wiki/`, published at `/wiki/`) - REFERENCE: how the
  code and systems work, at full detail (architecture, dev workflow, the
  scenario/section/modding guides). A scratch note whose substance is
  reference-grade gets migrated into a wiki dev page, not left in `docs/`.

## Release: compile, then wipe

At release time (before tagging):

1. **Distill** reference detail worth keeping out of `docs/` scratch into the
   wiki.
2. **Wipe** everything under `docs/` except this `README.md`, and commit.

## Where records go

- Anything tied to one task lives in that task's folder: `tasks/<id>/TASK.md`,
  `SPIKE.md`, `REVIEW.md`, `RETRO.md`, `NOTES.md`. A `grep`/`ls` of
  `tasks/<id>/` shows the whole story. Do not create per-task record files under
  `docs/`.
- **Plans are tatr tasks**, not `docs/plans` files (that folder is retired). A
  release plan is a task with the strand breakdown in its body (or a parent
  `meta`/`release` task linking the per-strand tasks); `/plan` and `/flow`
  produce tatr tasks directly.
- A durable, cross-cutting design record that used to live in `docs/design`
  lands in the wiki in the cycle it matters - `docs/` keeps nothing durable
  of its own.
