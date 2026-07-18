# Nova Protocol docs

This folder holds the project's **transient** records - the working notes the
SDLC workflow generates. The durable reference documentation (architecture,
build/dev, the game systems, modding) now lives in the wiki source and is
published as public pages; see below.

## Reference docs live in the wiki

The onboarding / reference docs moved into the website so they render as real
wiki pages (with diagrams, syntax highlighting and search) at `/wiki/dev/`.
Edit them at their source under `web/src/wiki/dev/` and keep them accurate when
the code they describe changes:

- `web/src/wiki/dev/development.md` - toolchain, everyday commands, features,
  examples, the web build, and the versioning/release checklist.
- `web/src/wiki/dev/architecture.md` - crate map and dependency graph, app
  assembly and plugin order, states, and the Update vs FixedUpdate frame flow.
- `web/src/wiki/dev/scenario-system.md` - the event/filter/action scenario and
  modding engine, variables, scenario objects, and where to add new pieces.
- `web/src/wiki/dev/sections.md` - ship sections, the integrity pipeline, typed
  damage, and ammo slots.
- `web/src/wiki/dev/modding-ron.md` - the RON data format, catalog, bundles and
  enabled set, the local cache and the `mods://` source, and file naming.
- `web/src/wiki/dev/mod-portal.md` - the static mod portal: layout, generator,
  the `catalog.json` wire schema, publishing, and game-side storage.
- `web/src/wiki/dev/keeping-docs-in-sync.md` - the map of which docs
  (CHANGELOG, News, wiki, tutorial) to update when you change code or cut a
  release, so nothing drifts.

## What lives here

The only files that belong under `docs/` are this README, the LESSONS.md
ledger, and the `design/` and `plans/` folders:

- [LESSONS.md](LESSONS.md) - the distilled lessons ledger. Read it before
  starting work; /compound appends to it.
- [design/](design/README.md) - durable design and implementation-reflection
  records for changes not owned by a single tatr task (or that landed outside
  one). Per-task records still live in the task folder.
- [plans/](plans/README.md) - long-form plans spanning multiple tasks (release
  scopes, roadmaps, process proposals).

Nothing else belongs in `docs/` root - no dated or per-task record files. Those
go in their task folder (below) or, if durable and cross-cutting, `design/`.

## Where records go

Everything tied to one task lives in that task's folder, so a `grep` or `ls`
of `tasks/<id>/` shows the whole story:

- `tasks/<id>/TASK.md` - the task itself (tatr).
- `tasks/<id>/SPIKE.md` - research that scoped the task (/spike).
- `tasks/<id>/REVIEW.md` - review rounds and verdicts (/review).
- `tasks/<id>/RETRO.md` - the retrospective (/compound).
- `tasks/<id>/NOTES.md` - design/fix record for the shipped change.

Do not create per-task record files under `docs/` root. A record tied to one
task lives in that task's folder as `NOTES.md`. The records kept under `docs/`
are the [LESSONS.md](LESSONS.md) ledger and the cross-cutting
[design/](design/README.md) records.

## After a meaningful change

Record, per `AGENTS.md`: what changed and why (alternatives, tradeoffs),
difficulties and how they were diagnosed, and what to do differently next time.
Update the relevant reference page under `web/src/wiki/dev/`, or write the
task's `RETRO.md`/`NOTES.md`; new recurring lessons go to the LESSONS.md ledger.
Plain ASCII punctuation only.
