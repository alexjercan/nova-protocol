# Spike: the seeded procedural open world, and the architecture it needs

- STATUS: CLOSED
- PRIORITY: 75
- TAGS: v0.15.0, spike, gameplay, architecture, open-world

Rewritten in place on 2026-09-21 at owner direction. This was an ideation
note carrying a 2026-09-06 research document whose direction was called
"agreed"; it is now one of the two v0.15.0 spikes that decide what a finished
Nova is. The owner selected continuous streamed sectors on 2026-09-22 after a
new architecture and risk pass. Epic: `20260921-231507`.

This is a SPIKE. It produces decisions, recorded open choices, and child
tasks. It does not implement the mode.

## What the mode is

A seeded procedural free/open-world mode: the player flies a persistent ship
through a procedurally unbounded fictional 3D world, rather than through a
fixed authored sequence. It takes inspiration from Solar-System objects and
plausible near-future technology; it is not a replica of the Solar System.
Nearby sectors stream during flight. The map records explored space and may
show a graph of important places and routes over the spatial sector grid.

Three constraints from the owner shape the architecture:

1. **Bootstrapped through a scenario, but not a scripted scenario.** The mode
   starts by loading something the scenario layer understands, because that is
   how the game gets a ship, a camera, and a place. Once running, it must not
   be a linear RON script with an authored beat list.
2. **The ongoing simulation is programmatic and Bevy-owned.** Faction state,
   travel, encounters, economy, and progression are systems and resources in
   Rust, not scenario event chains. The free-play bootstrap is intentionally
   empty: no authored filters, events, actions, or beat list drive the world.
3. **Routine travel is continuous.** Nearby sectors load and retire in the
   live world. Crossing an ordinary sector boundary does not call
   `LoadScenario` and does not show a loading screen.

## Mods keep clear extension points

The selected architecture must still give mods SUPPORTED, documented ways in.
The spike must name them explicitly and say what each one can and cannot
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
- `crates/nova_gameplay/src/hash.rs` owns stable FNV-1a hashes and
  `SeedStream`. Its contract explicitly rejects ambient RNG and schedule draw
  order for reproducible generated content.
- Avian is pinned to 0.7 and this build uses f32 physics positions. Nova has
  no floating-origin, global-sector-coordinate, or world-streaming runtime.
- `crates/nova_modding/src/lib.rs:77` currently has seven content variants:
  Section, Scenario, Campaign, Style, Ship, Lesson, and UiTheme. The older
  research's content inventory is stale.
- `crates/nova_assets/src/storage.rs:41` has atomic `read`/`write` for small
  key-value state and no `remove`. Its web backend is localStorage. It is not
  yet a save-slot or unbounded world-state store.

## Architecture decision: continuous streamed sectors

Owner decision, 2026-09-22: choose a refined form of option B. One empty
free-play scenario stays live while a world plugin streams nearby sectors.
Routine sector activation and retirement do not use `LoadScenario`.

Evidence for the choice:

- `LoadScenario` is an atomic replacement. It tears down every
  `ScenarioScopedMarker`, clears `NovaEventWorld`, freezes clocks through the
  load gate, and gates input and cameras. Those semantics conflict with
  background streaming during flight.
- Nova can reuse the loader's lower-level patterns - deterministic object
  construction, validation, preloading, and scoped recursive cleanup -
  without reusing the load event or global gate.
- The selected player experience values continuous flight and permits a new
  world-owned lifetime boundary.

The cleanup owners are nested, not competing:

- **Scenario scope** owns the whole free-play session and remains the final
  sweep when the mode ends.
- **Sector scope** owns generated local roots for one sector and retires only
  that subset while the scenario stays live.
- A streamed entity may carry both scopes. Session roots such as the player,
  camera, and UI do not carry sector ownership.

Use `sector`, not `chunk`, for this spatial unit. `CarvedChunkMarker` and
`ChunkGrace` already use chunk for carved asteroid debris.

Option A dies for routine open-world travel. It remains valid for existing
standalone scenarios. Option C remains possible later for exceptional authored
missions, but it is not the sector-streaming mechanism.

### Coordinate direction

Owner decision, 2026-09-22: global space is an integer 3D sector coordinate
plus local f32 physics positions. Crossing an origin-sector boundary performs
a discrete rebase so Avian and rendering stay near local zero. Exact types,
sector dimensions, active radius, and rebase schedule remain open.

This is new engine work. Nova has no floating-origin code, and Avian 0.7 has
no documented origin-rebase procedure. `NOTES.md` records the risk register
and the proofs needed before this direction can become an implementation
specification.

### Streaming lifecycle direction

A sector moves through absent, requested, generated, validated,
materializing, active, and retiring states. Exact names remain open. The
invariant is not open: a sector cannot become traversable until its required
collision and gameplay content has validated and materialized. Decorative
content may finish later. Generation and materialization are frame-budgeted;
the player does not silently enter incomplete space.

## Agent findings and risks, 2026-09-22

Three local research passes covered ECS/Avian risks, generation algorithms,
and persistence/mod contracts. A web pass checked public Avian, Bevy,
procedural-generation, and floating-point sources. Detailed notes and source
links are in `NOTES.md`.

Critical risks to retire before implementation:

- A rebase must update every relevant physics body and cached pose in one
  ordered operation. Broadphase proxies, CCD, interpolation, sleeping bodies,
  and transform synchronization need direct proof against Avian 0.7.
- Docked and joint-connected bodies must never split across coordinate frames
  or sector ownership during a crossing.
- Camera, HUD, targeting, audio, projectiles, and world-space effects must
  preserve relative measurements across a rebase.
- Every generated root needs sector ownership. Repeated travel must return
  live entity and collider counts to the same baseline.
- Streaming bypasses the scenario-load lint gate. Generated descriptions need
  an equivalent fail-before-spawn validation boundary.
- A player cannot enter an unready sector. Load-ahead failure needs a visible,
  deterministic stop rather than missing collision content.
- Same seed must not depend on visit order, ECS schedule order, or the ambient
  gameplay RNG. Native/web identity needs proof before it is promised.
- A procedural infinity makes visited-sector summaries and per-object
  tombstones unbounded. Persistence needs compaction and explicit reset
  promises.
- The current settings store is not yet a world-save store, especially on the
  web. Save-write failure must be player-visible.

Generation direction to evaluate:

- Derive independent named seed domains from world seed, generator version,
  integer macro/sector coordinates, purpose, and stable object identity.
- Use hierarchical random-access fields: macro regions for broad character,
  continuous global-coordinate noise for local variation, and stable discrete
  decisions for persistent structures.
- Treat sectors as query windows into one field. Do not reset noise phase or
  independently roll boundary features per sector.
- Generate sparse landmarks from coordinate-addressable candidates whose
  priority is compared with neighboring cells. Sequential Poisson sampling
  alone does not provide visit-order-independent infinite random access.
- Generate planets and other multi-sector anchors once at macro-region scope;
  sectors reference their stable identity rather than rolling duplicates.
- Keep the spatial grid separate from the explored map graph. Graph nodes are
  notable places and useful routes, not every empty cell.

Excluded from the specification: agent estimates for milliseconds, lines of
code, or delivery duration; unverified claims about commercial games'
algorithms; byte-identical floating noise without a native/web proof.

## Decisions the spike must reach or explicitly leave open

- **State ownership**: the world plugin owns streaming and durable state; the
  bootstrap scenario owns session liveness. Name the exact resources and
  components without weakening the nested cleanup rule.
- **Cleanup**: define the sector owner, moving-entity transfer, joint-group
  ownership, and retirement sequence. Reconcile the final session sweep with
  `teardown_scenario_entities`.
- **Coordinates and scale**: sector dimensions, active and retention radii,
  boundary hysteresis, rebase ordering, and how multi-sector anchors and
  gravity work.
- **Persistence**: a full ECS/world dump is rejected. Decide the exact
  seed-plus-sparse-state tiers, valid save points, compaction, platform store,
  compatibility policy, and what a mod change does to an existing save.
- **Deterministic seed**: how the world seed, generator version, coordinate
  bytes, and named domains are stored and threaded, given that the global RNG
  seed is environment-only today. Same seed and content set must mean the
  same structural world independent of visit order.
- **Generation**: choose the first hierarchical algorithm, biome meaning,
  landmark-spacing rule, station/planet ownership, and authored tuning
  surface. Separate persistent structural decisions from visual float noise.
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

The 2026-09-25 progression audit in
`tasks/20260824-125943/PROGRESSION-RESEARCH.md` adds open shared questions:
what persists when a mined rock/derelict or station sector retires; whether
ship condition, cargo and physical upgrades survive Retry/quit; and whether
P* A+ S* guarantees a placed rock per nonempty cluster, not per cell. Research
child tasks `20260925-190156` (shared persistence) and `20260925-190207`
(composition) record options for both spikes; neither owns a decision alone.
No schema, guarantee, or service is approved by that audit. Current `AppBuilder`
installs `NovaWorldBasePlugin` (`crates/nova_core/src/lib.rs:429`) and New Game
uses one bootstrap scenario; earlier opt-in/scenario-per-sector research and
this task's old opt-in completion language are historical, not current runtime
claims. Do not resolve these boundaries in only one task.

## Output: proposals to validate, not promises

Group the candidate outcomes into three lists. Each list is a PROPOSAL for
the owner to validate. Nothing in it is committed by being written here.

- **Needed for the game to feel complete at 1.0**
- **Nice to have**
- **Later / post-1.0**

Each entry names the child task it would become.

## Spike evidence, 2026-09-22

**Historical snapshot, not current architecture.** Since this spike,
`NovaWorldBasePlugin` has been wired into `AppBuilder`
(`crates/nova_core/src/lib.rs:429`) and installs the generic
`NovaWorldPlugin<NovaLayeredWorld>` through `nova_world_base`. Generation
policy moved from `nova_world` into `nova_world_base`; shipped New Game now
uses one empty bootstrap scenario and live streaming. The example ownership,
feature-sphere policy and opt-in claim below describe the *2026-09-22* state
only. See `PROGRESSION-RESEARCH.md` in sibling task `20260824-125943` for a
current boundary audit.

The streaming lifetime is no longer argued from code reading. It runs, in two
example targets that share one kit:

- `crates/nova_world` - two generators, the feature field and the job lifetime,
  in an opt-in crate nothing wires into `AppBuilder`.
- `examples/shared/world_fixture/mod.rs` - the seed, the cell edge, the active
  radius and the content ids the example targets share.
- `examples/systems/system_world_sectors.rs` - fourteen asserted claims, from
  an empty bootstrap to a swept unload.
- `examples/playable/world_sectors.rs` - the uniform generator, flown by hand.
- `examples/playable/world_features.rs` - the featured generator, flown by
  hand, with the feature spheres drawn.
- `examples/playable/world_field_slices.rs` - the raw field of one layer on one
  plane, painted as a heatmap with its gate drawn on it.
- `examples/playable/world_field_clouds.rs` - the same raw field in three
  dimensions, as sparse marks wherever a layer clears its gate.

Owner decisions taken on 2026-09-22, after the first synchronous version ran:

- **Spike cell edge: 32 km**. A travel-scale cell, so a boundary is crossed
  under way (about 17 s held on the free-fly ramp) rather than drifted over.
- **Active window: 5x5x5, radius 2** - 125 sectors and 500 bodies across a
  160 km cube, leaving at least 64 km of live world on every axis ahead of an
  observer standing anywhere in the centre cell.
- **Sector preparation is asynchronous**, on `AsyncComputeTaskPool`. On wasm
  that pool is the page's own task queue on the one thread it has; that is
  accepted for the spike.
- **In-flight preparation is bounded** to one job per pool thread. A window
  this size wants far more work than a machine can run, so the rest of the
  desired set stays unrequested - not queued - and the nearest missing cell
  takes the next slot that opens.

These correct earlier text in `NOTES.md`, which said 2 km and "no async". What
the runs established, including the two production-interface findings, is
recorded in the `Spike evidence` section of `NOTES.md`. The 32 km edge and the
125-cell window are the selected baseline; production density, activation
horizon, and body sizes remain open.

Owner decisions taken on 2026-09-22, after the uniform generator ran:

- **What a cell holds comes from a world above it**, not from the cell. Three
  independent global `Fbm<Perlin>` fields gate feature spheres on a coarse
  128 km lattice, and a cell contains whatever reaches it. The layers are
  independent on purpose: an asteroid belt, a planet and an anchorage may
  share ground, which is what makes a place read as a place.
- **A feature sphere is pure data** - id, owner cell, centre, radius,
  strength - so the cell that owns one and the five that only see it agree
  without talking to each other.
- **Same-layer spheres are thinned by rank** against a finite halo, never by
  visit order. `system_world_sectors` asserts both the thinning and the
  cross-layer overlap.
- **Density is tuned, not authored.** The fixed thresholds were moved until
  the pinned seed gives empty, single-layer and blended cells near the
  origin. The window at `(-2, -2, 2)` holds all three layers, which is where
  both hand-run examples open.

## Closure, 2026-09-25

The owner accepts the current seeded procedural world and closes this research
spike without requesting more world-generation work. New Game already starts a
single empty bootstrap and streams generated sectors through the
`NovaWorldBasePlugin` installed by `AppBuilder`
(`crates/nova_core/src/lib.rs:429`). The generic engine and base generator now
live in `crates/nova_world` and `crates/nova_world_base`. Routine travel does
not load a new scenario. This is the accepted outcome, not a claim that the
long-term open-world design is complete.

The original Done when list below is historical scope, **not** a claim that
all its proofs, mod contracts, or 1.0 choices were completed. In particular:

- **Coordinates and lifecycle:** generated body validation, stable cell
  ownership, bounded preparation, and sector retirement exist. Global-sector
  floating-origin rebasing, moving-body/jointed-group ownership across distant
  travel, a visible stop at an unready travel boundary, and all rebase/contact
  proofs in `NOTES.md` R1-R7 remain unimplemented or unproved. Do not infer
  infinite safe travel from the current streaming window. Retire those risks
  only with a separately approved travel/coordinate design.
- **Durable state:** there is no world or player-mutation save contract.
  Destroyed streamed content can return pristine after unloading. Save
  ownership, compaction, mod/version mismatch, web quota failures, and
  player-visible write failure remain for `20260925-190156`; `NOTES.md`
  R10-R12 cannot be signed off by this spike.
- **Generation and modding:** the current base generator owns its shipped
  cluster policy; `20260925-190207` owns the proposed asteroid-group guarantee
  and field-readability proof. Mods can supply existing authored content
  kinds, including sections and ships (`crates/nova_modding/src/lib.rs:77`),
  but the base generator does not promise to select arbitrary mod content.
  The generic `SectorGenerator` is a Rust integration interface, not a
  documented data-mod hook. No world-scale mod event/handler contract is
  approved. Content selection, section eligibility, and world-event scope
  need explicit future design; this closure does not claim three shipped mod
  doors or native/web bit-identical generation.
- **Shared boundaries:** persistence, station/world identity, and map/docking
  UI remain open with `20260824-125943`; its research children record options,
  not approved runtime interfaces. It is still open. The candidate lists in
  that task classify `20260925-190156` and `20260925-190219` as potential
  1.0 work, `20260925-190207` and `20260925-190131` as nice-to-have work,
  and broader economy/content as later work. These are proposals, not
  commitments made by closing this spike.

The 2026-09-06 `RESEARCH.md` and 2026-09-22 example measurements remain
historical. Revalidate their citations and assumptions against current master
before reuse. No floating-origin, persistence, new mod API, or additional
world feature is authorized by this closure.

## Done when (original research-spike criteria)

- The three architecture options are compared against verified code seams,
  and one is chosen with its evidence recorded, or the choice is explicitly
  left open with the options and consequences written down.
- Every decision above is either decided with evidence or recorded as an open
  decision with options and consequences.
- Each critical risk in `NOTES.md` has an owner, a chosen mitigation, and a
  named proof, or is explicitly deferred with its consequence.
- The first generation stack is selected with rules for seed domains,
  cross-sector coherence, stable identity, multi-sector anchors, and
  native/web determinism claims.
- The stale `RESEARCH.md` citations are marked in place, and any claim reused
  from that file has been re-verified against master.
- The three mod doors are named with what each can and cannot reach.
- The shared boundaries with `20260824-125943` each carry an agreed decision
  or a recorded open decision on both tasks.
- Child tasks exist, each in one of the three classification lists.
- This original *research spike* did not itself ship a world runtime or save
  schema. Later implementation shipped a generic `nova_world` engine and
  `nova_world_base` generator in New Game through `AppBuilder`; it did not
  implement persistent player/world mutations. The old example-only opt-in
  gate and feature-sphere ownership are **historical**, not current done
  criteria. New progression/persistence choices still require owner review.
