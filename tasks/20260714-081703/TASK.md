# In-editor scenario builder: place objects/objectives and save/load scenarios to RON

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.11.0,editor,scenario,modding,spike

Spike: tasks/20260714-081636/SPIKE.md
Spike: tasks/20260714-204059/SPIKE.md (editor UI rework - splits this into baseline + "the rest")

SCOPE NARROWED (20260714): the editor rework was split by the 204059 spike into a
BASELINE slice (wiki-style category rail + component drawer + tooltips +
player-only asteroid+planetoid scenario), now tracked as task 20260714-204219 and
built first, and "THE REST" - which is what THIS task now owns:
export/load `*.scenario.ron`, placing non-ship objects (asteroids, planetoids,
beacons, salvage), events/objectives wiring, factions (player vs enemy),
modifications beyond keybinds, and real component icons. Plan this task only AFTER
the baseline (204219) lands, extending its rail/drawer with the deferred
categories.

Promoted into v0.11.0 after the baseline editor work closed. Before planning
implementation, re-spike the current delta against the shipped gallery,
link-point placement, and flat scenario representation; the old v0.6.0 split is
historical context, not a current architecture assumption.

Goal: let the sandbox editor build a scenario, not just a ship - place scenario
objects (spaceships, asteroids, beacons, salvage), wire simple objectives/win-lose,
and save/load the result to/from a `*.scenario.ron` file on disk. Because a ship is
just a scenario with a single spaceship, "export my ship" falls out for free.
Deliberately expandable: start with place + save/load round-trip, grow toward
richer authoring (events, filters, objective wiring) over time.

The RON scenario format (`20260525-133029`), config-as-asset resource
(`20260525-133028`), and baseline editor (`20260714-204219`) are closed. The
editor must author the same serialized `ScenarioConfig` the runtime loads, so
hand-written mods and editor-built scenarios retain one representation.

Still a `spike`: "we also need to explore more here" (user). Before committing to a
plan, spike the authoring UX - how objects are placed/edited, how objectives/events
are surfaced without overwhelming the panel, and how the editor's in-memory ship
(today ad-hoc ECS entities) is lowered to a serializable `ScenarioConfig`. The
deeper design lives in the modding/authoring spike (tasks/20260714-081636 and its
follow-up); expand this task from there.

