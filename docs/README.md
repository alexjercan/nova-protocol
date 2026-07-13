# Nova Protocol docs

All project documentation lives here. Look here before touching a subsystem;
write new decisions down here.

## Reference

- [architecture.md](architecture.md) - crate layout, plugin wiring, states, frame flow.
- [scenario-system.md](scenario-system.md) - the scenario/modding engine: events, filters, actions, objects.
- [sections.md](sections.md) - ship sections and the integrity/damage system.
- [development.md](development.md) - toolchain, build/run/test, web build, release steps.
- [bevy-0.19-migration.md](bevy-0.19-migration.md) - historical: the 0.17 -> 0.19 migration notes.

## Records

Each folder has a README indexing its files. Files are named
`YYYYMMDD-description.md` (a `-HHMMSS` in the middle is a tatr task id).

- [retros/](retros/README.md) - per-task records: what changed, why, difficulties, lessons.
  [retros/LESSONS.md](retros/LESSONS.md) is the distilled lessons ledger - read it before starting work.
- [spikes/](spikes/README.md) - exploratory research that landed on a direction.
- [plans/](plans/README.md) - long-form plans spanning multiple tasks.
- [reviews/](reviews/README.md) - standalone review notes (task reviews live in `tasks/<id>/REVIEW.md`).

## After a meaningful change

Record, per `AGENTS.md`: what changed and why (alternatives, tradeoffs),
difficulties and how they were diagnosed, and what to do differently next time.
Update the relevant reference doc or add a retro; new recurring lessons go to
the LESSONS.md ledger. Plain ASCII punctuation only.
