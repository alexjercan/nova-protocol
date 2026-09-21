# Spike: the seeded procedural open world, and the architecture it needs

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.15.0,spike,gameplay,architecture,open-world

Rewritten in place on 2026-09-21 at owner direction. This was an ideation
note carrying a 2026-09-06 research document whose direction was called
"agreed"; it is now one of the two v0.15.0 spikes that decide what a finished
Nova is. Epic: `20260921-231507`.

This is a SPIKE. It produces decisions, recorded open choices, and child
tasks. It does not implement the mode.

## What the mode is

A seeded procedural free/open-world mode: the player flies a persistent ship
through a world generated from a seed, rather than through a fixed authored
sequence.

Two constraints from the owner shape every architecture option below:

1. **Bootstrapped through a scenario, but not a scripted scenario.** The mode
   starts by loading something the scenario layer understands, because that is
   how the game gets a ship, a camera, and a place. Once running, it must not
   be a linear RON script with an authored beat list.
2. **The ongoing simulation is programmatic and Bevy-owned.** Faction state,
   travel, encounters, economy, and progression are systems and resources in
   Rust, not scenario event chains. The scenario layer is the bootstrap and
   possibly the per-place scene; it is not the world.

## Mods keep clear extension points

Whatever architecture wins, a mod must still have SUPPORTED, documented ways
in. The spike must name them explicitly and say what each one can and cannot
reach:

- **Content**: new prototypes, hulls, and object kinds the generator can pick.
- **Sections**: new section kinds usable on a world ship.
- **Events**: the mod-facing event/handler vocabulary, at whatever scope the
  chosen architecture gives a mod.

An architecture that makes the world a private Rust simulation with no mod
surface is a failed answer, not a simple one. If a door has to close, the
spike records which one and what replaces it.

## Revisit the architecture. The old research is not settled.

`RESEARCH.md` (2026-09-06) stays in the task as evidence of the earlier
thinking. It is NOT the accepted design, and its "Decisions (owner,
2026-09-06)" section is a starting position to re-argue, not a constraint.

Its code citations must be revalidated before any of them is reused. Three
were checked on 2026-09-21 against master:

- `SectionModification` at `objects/modification.rs:34` - **STALE**. Neither
  the symbol nor the file exists. `crates/nova_scenario/src/objects/` has no
  `modification.rs`. The research treats it as a ready-made delta format; it
  is not one, because it is not there.
- `ShipSource::Persistent` - **DOES NOT EXIST**. `ShipSource` is not a type
  in the workspace. The research proposes it in sections 3 and 4 while
  reading, in places, as though it exists.
- `LOAD_LIMIT` - **EXISTS BUT UNRELATED**.
  `crates/nova_events/src/scale.rs:17` defines it as an 8g structural
  acceleration limit used by attitude and controller math
  (`crates/nova_ship/src/physics/attitude.rs:83`). `RESEARCH.md:165` invokes
  it as a reason a single persistent scene "fights the loader"; that argument
  needs a real citation or has to be dropped.

Treat every other unchecked citation in that file the same way until it is
re-verified.

## Verified seams, 2026-09-21

- `crates/nova_scenario/src/loader/mod.rs:551` - `LoadScenario(pub
  ScenarioConfig)`. A `ScenarioConfig` built in Rust at runtime can be
  triggered directly; it does not have to come from a RON asset. The editor
  already does this (`loader/lifecycle.rs:1098`). This is the seam a
  runtime-generated place would use.
- `crates/nova_scenario/src/loader/lifecycle.rs:63` -
  `teardown_scenario_entities` despawns every `ScenarioScopedMarker` entity,
  and its first statement is `world.clear()` (`lifecycle.rs:73`).
  `lifecycle.rs:138`
  `unload_scenario` and `lifecycle.rs:334` `on_load_scenario` both route
  through it, so a load tears the previous scenario down before spawning.
  **Scenario teardown currently owns broad cleanup, including clearing
  `NovaEventWorld`.** Any architecture that reloads per place has to answer
  what survives that.
- `crates/nova_scenario/src/world.rs:529` - teardown logs and DISCARDS
  undrained commands. World state parked in the event world is lost at a
  place transition today.
- **No world or save runtime exists.** There is no save-game, world-state, or
  persistent-ship runtime in `crates/`. The only persistence is
  `crates/nova_assets/src/storage.rs:41`, a platform key-value store used for
  settings and training progress. A persistent world means building this,
  not extending it.
- `crates/nova_core/src/lib.rs` `AppBuilder` fixes plugin order: Bevy ->
  input -> assets -> gameplay -> scenario -> UI -> debug. A world plugin has
  to declare where it sits in that order and what it may observe.

## The architecture question

Compare at least these three, on evidence, and pick one or record why the
choice is still open:

**A. Scenario-per-place, reload on travel.** Each station, belt, or
encounter is a `ScenarioConfig` generated at runtime and loaded through
`LoadScenario`. Travel is an unload plus a load.
- Buys: the existing loader, teardown, and scoping do the work unchanged.
- Costs: every piece of world state must live ABOVE the scenario and survive
  `teardown_scenario_entities` and `NovaEventWorld::clear`. A load screen or
  a hitch sits on every transition.

**B. One long-lived bootstrap scenario.** Load once; the world plugin
spawns, streams, and despawns inside that single live scenario for the whole
session.
- Buys: no transition teardown, no state-rescue problem, continuous
  simulation.
- Costs: the world plugin now owns lifetime and cleanup that the scenario
  layer owns everywhere else, and two cleanup owners is exactly the kind of
  split that leaks entities. Needs an explicit answer for what
  `ScenarioScopedMarker` means in this mode.

**C. Hybrid areas.** A persistent world shell plus scenario-scoped areas for
places that want authored or generated scenes.
- Buys: cheap local transitions, authored missions stay authored.
- Costs: the boundary between shell-owned and area-owned state is a new
  contract that must be written down, or it becomes ambiguous per-system.

Do not pick on aesthetics. Pick on what each option does to state ownership
and cleanup, and say what evidence decided it.

## Decisions the spike must reach or explicitly leave open

- **State ownership**: what the world plugin owns, what a scenario owns, and
  where the line is. Name the resources and components.
- **Cleanup**: who despawns what, and what survives a place transition.
  Reconcile with `teardown_scenario_entities` rather than working around it.
- **Persistence**: what a save IS (world dump vs. seed plus deltas), when it
  is written, where it lives on each platform, and what a mod change does to
  an existing save.
- **Deterministic seed**: how the world seed is stored and threaded, given
  that the global RNG seed is environment-only today and re-seeded from the
  OS on the web. Same seed must mean the same world.
- **Mod contract**: the three doors above, concretely - what a mod may add,
  at what scope, and what breaks a save.

Record each as decided-with-evidence or as an open decision with options and
one consequence each. An unsettled choice stays a choice; do not invent
closure.

## Coordinate with the station and UI spike

`20260824-125943` runs beside this one. They SHARE three boundaries:

- **Persistence**: ship inventory and credits are world state. One save
  format, decided once.
- **Content ownership**: stations are world objects and scenario objects.
- **UI surface**: the world map, travel, and docking all land in the same UI
  direction that spike is choosing.

Neither task may silently settle the other's question. Where they disagree,
record the disagreement as an open decision on BOTH tasks and take it to the
owner.

## Output: proposals to validate, not promises

Group the candidate outcomes into three lists. Each list is a PROPOSAL for
the owner to validate. Nothing in it is committed by being written here.

- **Needed for the game to feel complete at 1.0**
- **Nice to have**
- **Later / post-1.0**

Each entry names the child task it would become.

## Done when

- The three architecture options are compared against verified code seams,
  and one is chosen with its evidence recorded, or the choice is explicitly
  left open with the options and consequences written down.
- Every decision above is either decided with evidence or recorded as an open
  decision with options and consequences.
- The stale `RESEARCH.md` citations are marked in place, and any claim reused
  from that file has been re-verified against master.
- The three mod doors are named with what each can and cannot reach.
- The shared boundaries with `20260824-125943` each carry an agreed decision
  or a recorded open decision on both tasks.
- Child tasks exist, each in one of the three classification lists.
- No open-world code, content, or schema has been written by this task.
