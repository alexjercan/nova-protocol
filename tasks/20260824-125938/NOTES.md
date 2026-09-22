# Working notes: streamed procedural world

Date: 2026-09-22. These are spike notes, not an approved implementation
specification. `TASK.md` owns decisions. `RESEARCH.md` is the stale 2026-09-06
reference record.

## Owner direction recorded in this pass

- The world is procedurally unbounded. It is not a replica of the Solar
  System. Real objects and near-future technology are inspiration for a
  fictional world.
- Space is continuous and three-dimensional from the player's perspective.
- Nearby sectors stream in and out during flight. Routine sector boundaries
  do not invoke `LoadScenario` and do not show a loading screen.
- Free play starts through one intentionally empty scenario. It has no
  authored event/filter/action progression. Bevy-owned world systems run the
  mode after bootstrap.
- The map records explored space. It may show a graph of important places and
  routes over the spatial sector grid; empty grid cells need not become map
  nodes.
- The selected coordinate direction is an integer global sector coordinate
  plus local f32 physics positions, with a discrete origin rebase when the
  active sector changes.

## Selected architecture, in working form

```text
load empty free-play scenario
  -> create or load persistent world state
  -> choose the active global origin sector
  -> request nearby sectors
  -> generate deterministic descriptions
  -> validate every referenced content id
  -> materialize collision-critical objects first, over a frame budget
  -> activate each ready sector

player crosses an origin-sector boundary
  -> atomically advance the global origin coordinate
  -> translate every live local-space participant by the same delta
  -> preserve relative position, velocity, joints, camera, and HUD readings
  -> request sectors ahead
  -> retire sectors outside the retention radius
```

The scenario and sector lifetimes are nested rather than competing:

- Scenario scope is the whole free-play session. It guarantees complete
  cleanup when free play ends.
- Sector scope is a subset. It owns generated local objects and permits one
  sector to retire without touching the player, camera, UI, or other live
  sectors.
- A streamed entity may carry both scopes. Sector cleanup removes the narrow
  scope; scenario teardown remains the final sweep.

Use `sector` for this world unit. `chunk` already names carved asteroid debris
in `nova_gameplay::integrity`.

## Current code facts

- `crates/nova_scenario/src/loader/lifecycle.rs:23` gates ship and camera
  systems on one live scenario. The empty bootstrap satisfies that gate.
- `crates/nova_scenario/src/loader/lifecycle.rs:63` clears scenario mirrors and
  `NovaEventWorld`, then recursively despawns every `ScenarioScopedMarker`.
  It is an atomic scene-replacement lifecycle, not a streaming lifecycle.
- `LoadScenario` also uses the scenario load gate. Reusing it for an incoming
  sector would freeze clocks and input and would tear down all current sector
  content.
- `crates/nova_gameplay/src/hash.rs` owns stable FNV-1a hashes and
  `SeedStream`. Its documentation explicitly rejects global RNG draw order for
  reproducible generated content.
- `ScatterObjectsConfig` and asteroid seed-from-id are useful precedents for
  deterministic placement and stable appearance. They are scenario actions,
  not a world generator.
- `avian3d` is pinned to 0.7 and uses f32 positions in this build. No floating
  origin or global sector coordinate exists in Nova.
- `crates/nova_assets/src/storage.rs:41` is a small key-value interface for
  settings-like data. It has no deletion operation. Native storage is atomic;
  web storage is localStorage and string-oriented. It is not yet a save-game
  store.
- `crates/nova_modding/src/lib.rs:77` currently has seven `Content` variants:
  Section, Scenario, Campaign, Style, Ship, Lesson, and UiTheme. Older
  research listing Grammar, Impact, or Channel is stale.
- The scenario-load lint gate is reached through `LoadScenario`. A streaming
  materializer bypasses it and therefore needs an equivalent fail-before-spawn
  validation boundary.

## Sector state machine

A sector needs explicit state so partially generated space cannot look live:

1. `Absent`
2. `Requested`
3. `Generated`
4. `Validated`
5. `Materializing`
6. `Active`
7. `Retiring`
8. `Absent`

Names and Rust representation remain open. The important invariant is that a
sector is not traversable or targetable until collision-relevant content is
ready. Decorative materialization may finish later.

The first implementation should remain synchronous and frame-budgeted. Nova
has no gameplay-entity async spawning precedent. Background computation may
be investigated later, but ECS entity creation must still return to the main
world with explicit ordering and cancellation.

## Coordinate model

Conceptually:

```text
global_position = integer_sector_coordinate + local_position
```

- Integer coordinates identify procedural cells and survive saves.
- Local f32 positions are the only coordinates Avian, rendering, camera, and
  tactical systems should normally consume.
- Neighboring live sectors are placed relative to one active origin sector.
- When the player crosses the origin boundary, all materialized local-space
  participants move by exactly one common sector delta.
- Velocity is not changed by a pure position rebase. Any cached previous
  position, interpolation state, broadphase proxy, or world-space particle
  may still require treatment.

This is a discrete floating origin. It avoids letting literal f32 coordinates
increase without bound, but it is not supported by existing Nova code and is
not documented as a built-in Avian 0.7 operation.

## Risk register

Priority is for spike proof order, not release scheduling.

### R1 - Physics rebase ordering and broadphase correctness

- Priority: critical.
- Trigger: all live bodies translate when the active origin sector changes.
- Evidence: Avian owns `Position`/`Rotation` and synchronizes them with Bevy
  transforms. Its broadphase tracks moved proxies. Avian 0.7 documents no
  origin-rebase operation.
- Failure: missed or phantom collision on the crossing tick; collider and mesh
  disagree for one frame.
- First mitigation: one named ordering choke point before physics consumers;
  mutate every physics body through the Avian-owned position path and never
  rebase only selected visible objects.
- Proof: two bodies collide on the same tick as a scripted rebase. The contact
  must match a no-rebase reference run.

### R2 - Docking, joints, and physics islands split across ownership

- Priority: critical.
- Trigger: one member of a docked or joint-connected group changes sector or
  rebases without the other.
- Evidence: Nova docking uses an Avian fixed joint with body-local anchors.
  Equal translation is safe in principle; partial translation makes world
  anchors disagree by the entire rebase distance.
- Failure: a docked pair snaps, explodes, or separates at a boundary.
- First mitigation: sector assignment and rebasing operate on a connected
  physics group, not independently on each body.
- Proof: a docked pair straddles a boundary and keeps the same relative pose
  through the crossing.

### R3 - Sleeping-body wake storm

- Priority: high.
- Trigger: rebasing many sleeping asteroids, wrecks, or parked ships.
- Evidence: Avian documents transform changes as a wake cause for sleeping
  bodies.
- Failure: a repeatable frame spike on every sector crossing, followed by many
  bodies settling again.
- First mitigation: accept and measure the wake before designing a bypass.
  Do not assume sleeping state can be preserved through a position write.
- Proof: rebase increasing populations of sleeping colliders and inspect body
  state plus matched crossing-frame evidence.

### R4 - CCD and previous-position interpolation cross the rebase

- Priority: high, evidence incomplete.
- Trigger: a projectile or camera interpolator has a pre-rebase previous
  position and a post-rebase current position.
- Failure: a false long sweep, missed impact, camera whip, or one-frame render
  streak.
- First mitigation: rebase only at a defined fixed-step boundary and reset or
  translate every cached previous pose by the same delta.
- Proof: a projectile crosses and impacts on the rebase tick while camera
  relative offset remains unchanged.

### R5 - Mixed coordinate spaces in camera, HUD, targeting, audio, and effects

- Priority: high.
- Trigger: a consumer reads before the rebase while another reads after it, or
  a cached world-space effect is not translated.
- Failure: target indicators jump, sounds come from the old location, turret
  aim diverges, or an existing particle trail detaches from its emitter.
- First mitigation: audit every absolute-position reader and classify it as
  rebase participant, relative-space calculation, transient reset, or known
  acceptable artifact.
- Proof: camera offset and target bearing/range match a no-rebase reference;
  inspect the longest-lived global-space effect visually.

### R6 - Sector ownership and boundary oscillation

- Priority: critical.
- Trigger: an entity moves around a sector boundary or a jointed group spans
  it.
- Failure: ownership flips each frame, entities duplicate or disappear, or
  systems disagree about the current frame.
- First mitigation: one owner computes transitions, with a hysteresis rule;
  stable object identity does not change when sector ownership changes.
- Proof: scripted motion back and forth around a boundary produces no duplicate
  entity, no lost entity, and a bounded number of transitions.

### R7 - Partial materialization or load-ahead failure

- Priority: critical.
- Trigger: the player reaches a requested sector before its required collision
  and gameplay content is validated and active.
- Failure: flight into empty space, late collider pop-in, invisible station, or
  silent use of missing mod content.
- First mitigation: collision-critical readiness is an explicit gate. Prefetch
  ahead and keep a retention ring. Define a visible blocked/failure state
  rather than silently degrading authoring errors.
- Proof: deliberately delay or invalidate the next sector and assert that it
  never becomes traversable as an active sector.

### R8 - Cleanup leaks or broad cleanup deletes session state

- Priority: critical.
- Trigger: sector retirement uses only `ScenarioScopedMarker`, or generated
  entities do not receive a narrower owner.
- Failure: entity/collider count grows with distance travelled, or retiring one
  sector deletes the player or camera.
- First mitigation: sector ownership is mandatory on every sector-generated
  root. Scenario scope remains the superset and final cleanup.
- Proof: cycle between sectors repeatedly; exactly the retiring roots leave,
  bootstrap roots survive, and live counts return to the same baseline.

### R9 - Streaming bypasses content lint

- Priority: critical.
- Trigger: the generator emits an unknown prototype, section, ship, biome, or
  station id without using `LoadScenario`.
- Failure: partial scene, invisible collider, fallback content, or panic after
  the player has reached the sector.
- First mitigation: generated descriptions must validate before
  materialization, using the same known-ID and explicit-field policy as
  authored content.
- Proof: an intentionally unknown generated id refuses the sector before any
  of its entities spawn and reports the exact id and owner.

### R10 - Generation instability

- Priority: high.
- Trigger: visit order, system scheduling, adding a draw, changing a content
  table, platform math, or mod precedence changes existing generated space.
- Failure: the same seed produces different stations or landmarks, or a saved
  identity now denotes another object.
- First mitigation: coordinate-derived named seed domains; structural choices
  use stable integer operations; generation algorithm version and required mod
  versions are recorded.
- Proof: generate sectors in different visit orders and compare canonical
  descriptions. Run the same fixture on native and web before claiming
  cross-platform identity.

### R11 - Save growth and player expectation

- Priority: high.
- Trigger: every visited sector stores a full materialized description or one
  tombstone per ordinary asteroid.
- Failure: an unbounded save, slow web storage, or returning to a place that
  resets something the player believed permanent.
- First mitigation: define persistence by gameplay meaning. Untouched base
  generation costs zero; ordinary depletion uses a compact summary; only
  named or consequential entities are promoted to exact records.
- Proof: visit and modify many generated sectors; measure save growth by each
  persistence tier and verify a compacted save recreates the same named state.

### R12 - Web storage and compute limits

- Priority: high.
- Trigger: a world save is placed in the current localStorage-oriented
  settings store or native generation budgets are assumed to hold on wasm.
- Failure: quota/write failure, blocked main thread, lost progress, or a save
  failure logged without player feedback.
- First mitigation: select a save store separately from settings, surface
  write failure, and verify streaming on the actual web target.
- Proof: web save round-trip and quota/failure behavior, plus the same worst
  materialization fixture on native and web.

## Generation model

### Seed domains

Do not consume one global stream in visit order. Derive independent domains
from canonical bytes:

```text
world seed
  -> generator version
  -> macro-cell coordinate and purpose
  -> sector coordinate and purpose
  -> stable object identity and purpose
```

Purposes include region layout, biome, landmarks, asteroid fields, stations,
traffic, encounters, and visuals. Adding a visual draw must not relocate a
station. Exact separators, byte order, hash, and generator-version policy are
open interface decisions.

### Hierarchical random access

Use several integer lattices rather than generating a whole graph up front:

1. Macro cells establish broad density, resource, settlement, danger, and
   faction tendencies.
2. Region sites generated from each macro cell and its neighbors can define a
   Voronoi/Worley-like influence field without storing an infinite map.
3. Continuous global-coordinate noise can vary density within those regions.
4. Sector queries materialize only local candidates and inspect enough
   neighboring cells to resolve spacing and boundary ownership.
5. Local object details derive from stable object ids.

A sector is a query window into the procedural field, not a separate random
universe. Continuous fields sample global coordinates, so a sector edge does
not reset their phase.

### Biomes

A biome is a generation policy, not necessarily visible terrain. It can
control:

- object density and spatial pattern;
- asteroid, planet, sky, and hazard palettes;
- station and settlement likelihood;
- traffic and encounter tables;
- resource and salvage distribution;
- faction affinity and security;
- parameters for continuous noise fields.

Hard assignment produces clear regions. Blended influence produces gradual
transitions. Persistent structures should use discrete, stable decisions even
if visual density blends continuously.

### Landmark placement

Naive per-sector rejection sampling does not give random access with global
minimum spacing: visit order can decide which side of a boundary wins.
Candidate generation should instead be independent per lattice cell. A
candidate wins only after comparing its stable priority against candidates in
neighbor cells inside the exclusion radius. This gives deterministic local
queries and boundary-safe spacing without generating all prior landmarks.

Poisson/blue-noise sampling remains an option inside a bounded generated
region, but ordinary Bridson sampling is sequential and should not be assumed
to provide infinite random access by itself.

### Large anchors

Planets, major gravity wells, settlements, and large fields can span many
sectors. Generate them at macro-cell or region scope with one canonical owner
and stable identity. Sectors reference the anchor; they do not independently
roll duplicate planets.

A later design must decide whether planets are physical reachable bodies,
background/strategic anchors, or both at different distances. That choice
controls gravity range, collider scale, travel time, and streaming radius.

### Map graph

The integer sector grid is the spatial substrate. The map graph is a discovered
navigation abstraction over it.

Graph nodes may represent stations, settlements, known resource fields,
landmarks, hazards, or navigation references. Edges represent known useful
routes or relationships; they need not constrain continuous flight. Discovery
state is persistent even when local sector entities are regenerated.

## Persistence tiers to decide

### Always persistent

- world seed and generator version;
- required base/mod identities and versions;
- player global sector coordinate and local position at a valid save point;
- player ship, inventory, credits, and relationships once those systems exist;
- explored map knowledge;
- named stations, settlements, and claimed/player-created structures.

### Summary persistent

- resource-field depletion;
- settlement production or stock summaries;
- sector control, security, and major damage;
- whether a unique opportunity has been consumed.

### Promoted persistent entities

- named or consequential NPCs;
- captured or destroyed major structures;
- player-owned cargo left at a known place;
- wrecks only when gameplay explicitly promises they remain.

### Regenerated

- untouched celestial and asteroid layouts;
- ordinary fields and ambient traffic;
- generic encounters;
- decorative debris.

### Transient

- projectiles;
- visual effects;
- ordinary wreck fragments;
- solver contacts and sleeping state;
- other live ECS implementation state.

The save is not an ECS dump. Exact save points, compaction, deletion, generator
migration, and compatibility policy remain open.

## Fail-loud rules proposed for owner validation

- Missing or unknown generated content id: refuse materialization and name the
  id, sector, generator domain, and owning mod.
- Duplicate stable generated id: refuse the sector. Never choose one by spawn
  order.
- Missing required world seed: create and persist one as an explicit new-world
  operation. Never silently reseed an existing world.
- Non-atomic rebase: defer or stop the transition. Never rebase only part of a
  connected physics group.
- Player reaches an unready sector: block entry visibly. Never mark it active
  with absent collision content.
- Save write fails: surface it to the player. The settings store's current
  log-and-continue policy is not sufficient for progress.
- Missing required mod or incompatible generator: refuse the save with the
  exact dependency. Never discard unknown persistent records.

## External evidence checked

Useful direct sources:

- Avian 0.7 release notes: <https://joonaa.dev/blog/13/avian-0-7>
- Avian API documentation: <https://docs.rs/avian3d>
- Bevy mesh precision issue: <https://github.com/bevyengine/bevy/issues/3176>
- Bevy floating-point semantics context is still subject to Rust and target
  behavior; structural generation should not assume arbitrary floating noise
  is byte-identical without a native/wasm proof:
  <https://rust-lang.github.io/rfcs/3514-float-semantics.html>
- Bridson, Fast Poisson Disk Sampling in Arbitrary Dimensions:
  <https://www.cs.ubc.ca/~rbridson/docs/bridson-siggraph07-poissondisk.pdf>

The web pass found no official Avian 0.7 floating-origin procedure. It also did
not justify the earlier agents' exact millisecond or line-count estimates.
Those estimates are excluded. Claims about named commercial games' internal
algorithms are also excluded unless a primary source is found.

## Research conclusions

- The selected architecture fits the player's desired experience better than
  scenario-per-sector loading, but it transfers cleanup, validation, and
  readiness ownership to a new world-streaming layer.
- The first technical uncertainty is not noise. It is whether a complete local
  physics world can be rebased atomically without breaking contacts, joints,
  interpolation, or player-facing relative measurements.
- Procedural coherence should come from hierarchical, coordinate-addressable
  fields. Independent random rolls per sector will produce seams and random
  soup.
- Persistent structure should use integer and stable-id decisions. Continuous
  float noise is suitable for density and visuals after cross-platform behavior
  is proven.
- Sparse persistence avoids full scenario diffs, but it still requires an
  explicit promise about what resets. That promise is a game-design decision,
  not only a storage optimization.
