# Nova Protocol docs

Project documentation lives here. It is the place future agent sessions (and humans)
should look before touching a subsystem, and where new decisions get written down.

## Contents

- [architecture.md](architecture.md) - workspace/crate layout, plugin wiring, app
  states, and the frame flow.
- [scenario-system.md](scenario-system.md) - the scenario/modding engine: events,
  filters, actions, variables, objectives, and the event-world queue.
- [sections.md](sections.md) - spaceship sections (hull/thruster/controller/turret/
  torpedo) and the integrity/health/destruction system.
- [development.md](development.md) - toolchain, build/run/test, features, web build,
  and release.
- [bevy-0.19-migration.md](bevy-0.19-migration.md) - the Bevy 0.17 -> 0.19 API
  changes applied to the codebase.
- `retros/` - retrospectives from completed tasks (see the `/compound` skill).
- `spikes/` - exploratory research docs (see the `/spike` skill).

## When you make a meaningful change

Per the repo conventions (see `AGENTS.md` and `~/AGENTS.md`), after a meaningful
change record:

1. **What changed and why** - the decision, alternatives considered, tradeoffs.
2. **Difficulties** - bugs hit along the way, how they were diagnosed and fixed.
3. **Self-reflection** - what could have gone better, what to do differently next time.

Update the relevant doc above, or drop a retro under `retros/`. Keep the writing in
plain ASCII punctuation (no em dashes, smart quotes, or arrows).
