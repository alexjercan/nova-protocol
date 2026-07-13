# Nova Protocol docs

Reference documentation lives here. Task-scoped records (spikes, reviews,
retros, design notes) live next to their task under `tasks/<id>/` - see
"Where records go" below.

## Reference

- [architecture.md](architecture.md) - crate layout, plugin wiring and order,
  app states, frame flow (Update vs FixedUpdate, physics, interpolation).
- [scenario-system.md](scenario-system.md) - the scenario/modding engine:
  event kinds, filters, actions, variables, objects, and where to add new ones.
- [sections.md](sections.md) - ship sections (hull/controller/thruster/turret/
  torpedo), the integrity pipeline, typed damage, and ammo slots.
- [development.md](development.md) - toolchain, everyday commands, testing
  habits, features, examples, web build, and the release checklist.
- [bevy-0.19-migration.md](bevy-0.19-migration.md) - historical: the API
  changes applied moving from Bevy 0.17 to 0.19; useful for the next upgrade.
- [plans/](plans/README.md) - long-form plans spanning multiple tasks
  (release scopes, roadmaps, process proposals).
- [retros/LESSONS.md](retros/LESSONS.md) - the distilled lessons ledger.
  Read it before starting work; /compound appends to it.

## Where records go

Everything tied to one task lives in that task's folder, so a `grep` or `ls`
of `tasks/<id>/` shows the whole story:

- `tasks/<id>/TASK.md` - the task itself (tatr).
- `tasks/<id>/SPIKE.md` - research that scoped the task (/spike).
- `tasks/<id>/REVIEW.md` - review rounds and verdicts (/review).
- `tasks/<id>/RETRO.md` - the retrospective (/compound).
- `tasks/<id>/NOTES.md` - design/fix record for the shipped change.

Do not create per-task record files under `docs/`. The only records kept here
are [retros/](retros/README.md): the LESSONS.md ledger plus a few old records
whose task folder no longer exists.

## After a meaningful change

Record, per `AGENTS.md`: what changed and why (alternatives, tradeoffs),
difficulties and how they were diagnosed, and what to do differently next time.
Update the relevant reference doc, or write the task's `RETRO.md`/`NOTES.md`;
new recurring lessons go to the LESSONS.md ledger. Plain ASCII punctuation
only.
