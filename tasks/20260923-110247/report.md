# Report: Simplify generated-world data structures (20260923-110247)

Date: 2026-10-11. Base: `master` at `37e2a9256`. Read-only investigation. No code,
test, or TASK.md change. All sections except "Delivered on master after this
report" are that original snapshot.

## Answer

Most of this task already landed in PR #113 (`86084eb1a`). Commits "Carry whole
spawn configs in sector records" and "Resolve the visited-cell rule once on the
sector worker" in that PR did the main Delivery items. The `Agent findings` in
TASK.md describe the code before #113 and are now stale.

The remaining work is small. It does not justify the standalone refactor that
the task describes. One item is a real defect (stale architecture doc). Three
items are small validation gaps. The rest is naming and one documented rule.

Recommendation: fix the stale doc now. Decide whether to close the validation
gaps under this task or under a smaller task. Then close this task. Do not do a
naming-only refactor unless a feature touches these types.

## Delivered on master after this report (2026-10-11)

Option B without item 4. Status stays OPEN for the naming criterion.

- Item 1: `docs/architecture.md` `nova_world` row now states the record rule,
  the full refusal list, `spawn.inventory` / `spawn.credits`, and that the
  crew policy is the generator's.
- Item 2: `validate_manifest` also refuses a handle in the rock
  `destroy_sound` (field `destroy_sound`) and in each of the ten design
  presentation sounds (field `design`, value names the sound).
  `check_ship_design` destructures `ShipPresentationConfig` exhaustively, so a
  new presentation field fails to compile until it is sorted. Section
  prototype patches carry no asset refs (`nova_ship/src/sections/patch.rs`).
  Correction: the presentation sounds span `ship_design.rs:152-201`, not
  `:152-186`.
- Item 3: `AsteroidConfig::validate_lock_signature` owns the rule. Callers:
  `validate_manifest` (field `lock_signature`) and the lint helper
  `check_asteroid_lock_signature` on direct spawns and scatter templates. The
  load path runs the same lint (`loader/lifecycle.rs:223`).
- Module doc of `generation.rs` states the record and ownership rule.

## What #113 changed (evidence)

Before #113 (`git show 86084eb1a^:crates/nova_world/src/generation.rs`):
`SectorAsteroid { id: String, position, radius, kind, seed: u32,
initial_velocity }` and `SectorShip { ..., design: ShipDesign, crew:
Option<SectorShipCrew>, stock, credits }`. Materialization built the rock config
around its own texture and turned the crew into an AI config. The #113 commit
message states this directly.

Now:

| Record | Shape | Evidence |
| --- | --- | --- |
| `SectorAsteroid` | `id: SectorObjectId`, `position`, `config: AsteroidConfig` | `crates/nova_world/src/generation.rs:56-66` |
| `SectorPlanet` | `id`, `position`, `config: PlanetConfig` | `crates/nova_world/src/generation.rs:87-94` |
| `SectorShip` | `id`, `position`, `rotation`, `clearance`, `condition`, `civilization`, `role`, `spawn: SpaceshipConfig` | `crates/nova_world/src/generation.rs:243-267` |
| `SectorObjectId` | private `String`, built only by `sector_id` | `crates/nova_world/src/generation.rs:33`, `:592-594` |
| `PreparedSector` | `coord`, `planets: Vec<PreparedPlanetoid>`, `bodies: PreparedBodies` | `crates/nova_world/src/generation.rs:499-505` |
| `PreparedBodies` | `Fresh { rocks, ships, arrivals }` or `Thawed(..)` | `crates/nova_world/src/generation.rs:517-529` |
| `PreparedRock` / `PreparedPlanetoid` | `{ body, geometry }` / `{ body, surface }` | `crates/nova_world/src/generation.rs:533-543` |

## Delivery and Done-when against current state

| Item | State | Evidence |
| --- | --- | --- |
| Coherent worker-safe asteroid, planet, ship payloads | DONE. Each record is id + placement + the whole scenario spawn config. Ship adds generator facts (rotation, clearance, condition, civilization, role). | `generation.rs:56-66`, `:87-94`, `:243-267` |
| Controller, allegiance, loot, stock, credits explicit | DONE. The generator decides them: AI with leash and Enemy grace for intact, `None`/Neutral/lootable for derelict. | `crates/nova_world_base/src/sector_ships.rs:285-317` |
| Materialization adds no spawn policy | MOSTLY DONE. Ship spawn adds only pose, id, derived name, and `DerelictShipMarker`. Rocks and planetoids get `Quat::IDENTITY` and `name = id` from materialization; neither is stated on the record. | `crates/nova_world/src/streaming.rs:309-345`, `:236-261` |
| Invulnerability, mass, lock signature | DONE or N/A. Every asteroid is destructible by type rule (`invulnerable` removed). Planet mass is required in `PlanetConfig`. A `None` lock signature is the spawn factory's documented rule, shared with authored scenarios, not a materialization default. | `crates/nova_scenario/src/objects/asteroid.rs:47-51`, `:92-102`; `crates/nova_scenario/src/objects/planet.rs:144-148` |
| Pair each description with its prepared payload | DONE. Records travel with their payload. No length asserts or positional pairing remain. A held ship resolves by id and panics with coord and id when missing. | `generation.rs:994-1034`; `crates/nova_world/src/frozen.rs:319-336` |
| Normalize prepared names | NOT DONE. See "Remaining" item 4. | `generation.rs:533-543` |
| Update manifests, materialization, preludes, docs, examples, proofs | MOSTLY DONE. One stale doc remains. See item 1. | `crates/nova_world/src/lib.rs:209-228`; `docs/architecture.md:26` |
| Remove flattened fields and reconstruction paths | DONE. `SectorShipCrew` and `check_ship_crew` are gone. `rg SectorShipCrew crates docs web/src` finds nothing. | #113 commit message; search below |
| One documented schema rule | NOT DONE as a single stated rule. The pattern exists in code. Each struct has its own doc. | `generation.rs:54-66`, `:240-266` |
| Main-thread vs worker ownership explicit | MOSTLY DONE. Worker runs `prepare_cell` on `AsyncComputeTaskPool` with only the config, coord and frozen record. Catalog resolve stays on main thread (`require_resolved`). Asset refs are refused as handles only for the rock texture. See item 2. | `streaming.rs:468`, `:219-221`, `:295-307`; `generation.rs:491-497`, `:699-705` |
| Old unshipped schema paths deleted | DONE (save format moved 2 -> 3 for the held-ship change, unshipped). | #113 commit message |
| Focused proofs | EXIST. See "Verification for the chosen option". Not run for this report. | below |

## Remaining work

1. Stale architecture doc (real defect, docs only).
   `docs/architecture.md:26` (the `nova_world` row) says the core enforces "a
   crew present on exactly an intact ship", and that each ship spawns "with its
   manifest `stock`" and "`credits`", and attributes the AI-crew and the
   "unpiloted and Neutral" derelict policy to `nova_world`. Current core rule:
   refuse a player, refuse an AI on a derelict, accept `None` on any ship
   (`generation.rs:804-835`). The policy now lives in the generator
   (`sector_ships.rs:285-299`). The fields are `spawn.inventory` and
   `spawn.credits` (`generation.rs:429-430`). The row also omits the #113
   refusals: `seed: None`, a handle texture, `allegiance: None`, a catalog
   design (`generation.rs:692-705`, `:756-773`).

2. Asset-ref refusal covers one of several fields.
   `validate_manifest` refuses `AssetRef::Handle` only on `config.texture`
   (`generation.rs:699-705`). It accepts a handle in `destroy_sound`
   (`asteroid.rs:91`) and in the ship design's presentation sounds
   (`crates/nova_scenario/src/objects/ship_design.rs:152-186`). `canonical()`
   documents that a handle prints a run-local id and that "generated configs
   carry none" (`generation.rs:375-377`). The base generator sets none today
   (`crates/nova_world_base/src/clusters.rs:975`;
   `crates/nova_world_base/src/ship_layout.rs:1763-1767` uses `..default()`).
   Consequence: a third-party generator can put a handle in those fields and
   pass validation; same-seed canonical text then differs across runs. No
   failure is reproduced. The worker never resolves a handle, so this is a
   determinism and contract gap, not a worker/`Assets` access.

3. Asteroid lock signature is not checked anywhere.
   `PlanetConfig::validate` refuses a non-finite or non-positive lock
   signature (`crates/nova_scenario/src/objects/planet_type.rs:449-452`).
   `validate_manifest` does not check the asteroid one (`generation.rs:672-708`),
   and the scenario lint has no asteroid case (`rg lock_signature
   crates/nova_scenario/src/lint` hits only the planet message at
   `crates/nova_scenario/src/lint/scenario.rs:1460` and fixtures). The spawn uses
   it directly (`asteroid.rs:364-367`). This gap is shared with authored
   scenarios, so its owner is `AsteroidConfig`, not `nova_world`. Unverified:
   runtime effect of a NaN `LockSignature`.

4. Naming is not parallel (cosmetic).
   Records: `SectorAsteroid` / `SectorPlanet`. Pairs: `PreparedRock` /
   `PreparedPlanetoid` with fields `geometry` / `surface`. Scenario payloads:
   `PreparedAsteroid` / `PreparedPlanet` (`asteroid.rs:173`,
   `planet.rs:90`). So one object is "asteroid" and "rock", the other "planet"
   and "planetoid". No doc states a naming rule.

5. Planet config travels twice (minor).
   `prepare_cell` clones `body.config` into `prepare_planet`
   (`generation.rs:997-999`). `PreparedPlanet` keeps its own config
   (`planet.rs:90-95`). Materialization reads only `body.id` and
   `body.position` from the record (`streaming.rs:250-260`). The asteroid path
   is the opposite: geometry holds only seed and radius and the spawn asserts
   the match (`asteroid.rs:315-322`). Materialization passes `seed` beside
   `config` although `config.seed` is already `Some` (`streaming.rs:246-247`).
   Both differences are documented on purpose (`asteroid.rs:156-159`,
   `planet.rs:84-88`). The task's user fact forbids hiding this difference, so
   this item is about clarity only.

6. Trusted accessors panic instead of the type proving the rule.
   `SectorAsteroid::seed()` panics on `None` (`generation.rs:75-82`) and
   `SectorShip::design()` panics on a catalog design (`:276-285`). Validation
   makes both unreachable (`:692-698`, `:756-765`). A dedicated payload type
   could make them unrepresentable, but it would duplicate `AsteroidConfig` and
   `SpaceshipConfig`, which TASK.md `Decisions` says to avoid. #113 chose the
   reuse-and-validate path. I recommend keeping it.

## Adjacent observation (not this task)

The base generator uses `GravitySettings::default()` for planetoid SOI checks
and planned velocities (`clusters.rs:386`, `:722`), not the live resource.
Search for a production override (`rg "insert_resource\(GravitySettings|GravitySettings \{"
crates examples`, tests excluded) found none, so no divergence today. This is
generator policy, outside the materialization scope.

## Risk and scope

- Item 1: docs only. No behavior risk.
- Items 2-3: add refusals. Risk is a refusal of a config a generator or mod
  already writes. The base generator writes none of these (evidence above).
  Item 3 also touches authored scenario lint, so it can refuse mod content;
  that needs a lint message and a changelog entry if any shipped content uses
  the field. Unverified: whether shipped or mod content authors an asteroid
  `lock_signature`.
- Items 4-5: renames and one removed clone inside `nova_world` and
  `nova_scenario`. Callers: `streaming.rs`, `frozen.rs`, tests, the
  `world_sector_inspector` example. Public preludes export `PreparedSector`
  only from `nova_world` (`lib.rs:211-216`); `PreparedRock` and
  `PreparedPlanetoid` are `pub(crate)`. `PreparedAsteroid` and
  `PreparedPlanet` are public in `nova_scenario` (`asteroid.rs:32-39`,
  `planet.rs:33-38`). No content or save format change. The pinned canonical
  digest does not move for renames.
- Item 2 or a schema rule change that edits `canonical()` text moves the
  pinned digest in `crates/nova_world_base/src/tests.rs:336-341`.

## Options

A. Close now with the doc fix only. Fix `docs/architecture.md:26`, update
   TASK.md findings, close. Cost: smallest. Leaves items 2-6.

B. Doc fix plus the validation gaps (recommended). Item 1, plus item 2 in
   `validate_manifest`, plus item 3 in the owning `AsteroidConfig` path so the
   authored lint and the world check share one rule (the `PlanetConfig::validate`
   pattern). Add one paragraph to the `generation.rs` module doc that states
   the record rule (id + placement + whole spawn config; generator facts beside
   it; materialization adds pose identity, name and markers only) and the
   rock/planet ownership difference. Cost: small. Closes the Verification bullet
   about malformed descriptions. Needs approval of where the asteroid check
   lives, its error form, and any new test.

C. B plus naming and duplication cleanup (items 4-5). Defer per the task's own
   "Defer until a feature touches these data structures". Cost: medium churn for
   names only.

D. Introduce dedicated worker payload types (item 6). Not recommended. It
   duplicates scenario configs against TASK.md `Decisions` and reverses #113.

Sequence for B: doc fix -> world refusals for asset handles -> asteroid config
check at its owner -> module doc rule -> TASK.md update and close.

## Decisions needed before B or C

- Where the asteroid lock-signature check lives: an `AsteroidConfig` method in
  `nova_scenario` (planet pattern) or only in `validate_manifest`.
- Whether `validate_manifest` refuses every `AssetRef::Handle` in a record, or
  only the texture as now.
- Whether rock and planetoid records state a rotation, or the doc states that
  materialization spawns them at identity.
- For C: the naming rule (asteroid/planet vs rock/planetoid) and the field
  names.

No type or function name above is approved. Names in this report are current
code names only.

## Verification for the chosen option

- Malformed descriptions: extend the existing case table in
  `a_malformed_generator_answer_is_refused_before_preparation`
  (`crates/nova_world/src/tests.rs:256`, cases at about `:265-420`), which already
  covers wrong cell, duplicate id, face crossing, overlap, unknown kind,
  non-finite velocity, missing seed, handle texture, NaN planet mass, no
  sections. New cases need approval.
- Determinism: `the_open_world_describes_the_same_sectors_in_any_visit_order`
  (`crates/nova_world_base/src/tests.rs:101`) and
  `a_pinned_window_generates_the_recorded_bodies` (`:324-342`).
- Canonical identity: `crates/nova_world/src/tests.rs:673`, `:730`, `:834`,
  `:862`.
- Spawn and lifecycle: `crates/nova_world/src/tests.rs:971` (held ship until
  the observer is clear), `:1056`, `:1146`;
  `crates/nova_world/src/tests/frozen.rs:447`.
- Cluster replay determinism:
  `crates/nova_world_base/src/clusters.rs:1963`.
- Commands (affected only): `nix develop --command cargo check -p nova_world -p
  nova_world_base -p nova_scenario --all-targets`; `nix develop --command cargo
  test -p nova_world --lib`; `nix develop --command cargo test -p
  nova_world_base --lib`; `nix develop --command cargo check --examples
  --keep-going`. Item 3 also needs the scenario lint over base content.

## Proof commands used for this report

All read-only:

- `git show --stat 86084eb1a`, `git show 86084eb1a -- docs/architecture.md`
- `git show 86084eb1a^:crates/nova_world/src/generation.rs` (old shapes)
- `git log -S "pub config: AsteroidConfig"` and similar (all land in `86084eb1a`)
- Full read of `crates/nova_world/src/generation.rs`; `streaming.rs:1-60`,
  `:150-360`; `frozen.rs:319-356`
- Reads of `asteroid.rs:1-240`, `:279-367`; `planet.rs:1-200`;
  `planet_type.rs:326-460`; `ship_design.rs:115-186`;
  `sector_ships.rs:250-318`; `clusters.rs:955-1010`; `ship_layout.rs:1755-1775`
- `rg` for record constructors, `SectorShipCrew`, `lock_signature` in lint,
  `AssetRef` in world crates, doc mentions, and overlapping tasks (none open)

No tests or builds were run.
