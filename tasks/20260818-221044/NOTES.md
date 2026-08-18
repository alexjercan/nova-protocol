# Verified facts

Every claim below was read out of the tree at `docs-destruction`. File and line
are the evidence for the doc text that quotes it.

## The two damage readings

- `DamageLevel(f32)`, 0.0 pristine to 1.0 destroyed, derived from the entity's
  OWN `Health`, never from an aggregate. Read-only for consumers.
  `crates/nova_gameplay/src/integrity/erosion.rs:46-60`.
- `DamageMarks(Vec<DamageMark>)`, `DamageMark { at: Vec3, radius: f32 }`, in the
  LOCAL frame of the body carrying them.
  `crates/nova_gameplay/src/integrity/carve.rs:139-187`.
- A hit is recorded on the nearest ancestor carrying `DamageMarks`
  (`mark_owner`), never on the collider it met.
  `crates/nova_gameplay/src/integrity/carve.rs:461-469`.

## Cost model

- `DAMAGE_PER_UNIT_VOLUME = 8.0` hit points per cubic unit. Absolute, never
  relative to what was hit. `carve.rs:63-82`.
- `mark_radius(amount) = (amount / 8.0 * 3 / (2 pi))^(1/3)` - a HEMISPHERE,
  because a hit lands ON a surface. `carve.rs:263-276`.
  - PDC kinetic round, 4.0 damage -> 0.62 world units. Matches the figure
    quoted in `carve.rs:97-98` and `asteroid_carve.rs:64-68`.
- A mark is priced on what the target ABSORBED, not what the round asked for.
  `absorbed_by`: the first `Health` at or above the hit clamps the amount; a
  node already spent pays 0; NO pool anywhere up the chain spends the whole hit
  in material (the asteroid rule). `carve.rs:278-306`.
- `MARK_MIN_RADIUS = 0.15` (`carve.rs:89`). `MERGE_REACH = 4.0`, a multiple of
  the INCOMING bite, not of the grown crater (`carve.rs:91-104`).
  `MERGE_MAX = 1.0` WORLD unit, an absolute cap converted into the body's frame
  (`carve.rs:106-127`). `MARK_BUDGET = 24`, and past it the SMALLEST crater is
  folded into its own nearest neighbour - nothing is dropped and paid volume is
  conserved (`carve.rs:129-137`, `DamageMarks::add` :213-236,
  `fold_smallest_crater` :246-260).
- `DamageMark::accepts` takes the smallest of the crater's own radius,
  `MERGE_REACH * incoming.radius` and the cap. `carve.rs:166-169`.

## Blast

- `apply_blast_damage` queues `record_blast_marks` BEFORE the health triggers,
  so every body prices against one pre-damage snapshot.
  `crates/nova_gameplay/src/damage.rs:308-323`.
- `record_blast_marks` sums per owning body and cuts ONE crater per body,
  capped at the blast's own radius. `carve.rs:383-437`.
- `EXPLOSIVE_SECTION_TRANSMISSION = 0.65`. Pressure follows the centre ray,
  stops at the first ship section it cannot destroy, retains 65% through each
  section it does destroy; cladding and fixtures can be shielded but never
  consume penetration. `damage.rs:449-473`.

## Damage effects (authored)

- `DamageEffect::{Cracks, Sparks, Plume}`. No `Carve`, no `Scorch`.
  `crates/nova_ship/src/sections/damage_effects.rs:52-73`.
- `DamageEffects` default is `[Cracks]`; `DamageEffects::none()` is the empty
  list; `is_default` drives `skip_serializing_if`.
  `damage_effects.rs:83-114`.
- Authored as `base.damage_effects` on `BaseSectionConfig`.
  `crates/nova_ship/src/sections/base_section.rs:275-286`.
- Each variant maps to exactly one component in `fit_damage_effects`
  (`damage_effects.rs:154-172`). NO SECTION LOSES GEOMETRY
  (`damage_effects.rs:32-37`).
- Cracks: fracture pattern on the section's own material clone, glows through
  when critical, burnt when dead. Replaced the whole-body tint.
  `crates/nova_ship/src/sections/damage_cracks.rs:1-33`.
- Sparks: `SPARK_THRESHOLD = 0.35`, geometry untouched at every level.
  `crates/nova_ship/src/sections/damage_sparks.rs:1-38`.
- Plume: `PLUME_THRESHOLD = 0.35`, `PLUME_FLOOR = 0.25` (never reads as shut
  down), `FLICKER_DEPTH = 0.45`. Thrust is UNCHANGED.
  `crates/nova_ship/src/sections/damage_plume.rs:1-45`.
- Shipped authoring: Hull `[Cracks]` (the default, omitted); Controller,
  Turret, Torpedo `[Cracks, Sparks]`; Thruster `[Cracks, Sparks, Plume]`.
  `crates/nova_authoring/src/base_content/sections/standard.rs:242,313,334-337,374,475,561`
  and `crates/nova_authoring/src/base_content/ships/shared.rs:252-270`.
- `crates/nova_ship/src/sections/damage_tint.rs` is DELETED (808 lines, commit
  `0ee9cbb0`).

## Carve leftovers

- `CarveSpew { entity, at, radius }` fires only when a mark changed the body's
  shape; world space. `carve.rs:439-458`.
- Shards: 2 to 7 per carve, sized `0.22 * crater`, kinematic, NO collider,
  `TempEntity(2.5)`. `crates/nova_gameplay/src/integrity/spew.rs:61-93,218,243`.
- Real geometry leaves a body only where a carve SEVERED it; that is the
  asteroid's own path. `spew.rs:22-31`.
- `CHUNK_MIN_VOLUME = 1.0` cubic world unit: under it a severed piece goes out
  as dust instead of a rigid body.
  `crates/nova_gameplay/src/integrity/chunk.rs:50-62`.

## Asteroids

- `AsteroidConfig` fields: `radius`, `texture`, `impact_sound`,
  `destroy_sound`, `mass`, `invulnerable`, `lock_signature`, `seed`. There is
  NO `health` field. `crates/nova_scenario/src/objects/asteroid.rs:42-105`.
- A normal rock gets `DamageMarks` + `CollisionEventsEnabled` and NO `Health`
  at all; `invulnerable: true` gets neither.
  `asteroid.rs:213-218`, test `asteroid.rs:947-979`.
- Field: `FIELD_CELL_WORLD = 0.5` WORLD units (the cell is fixed in the world,
  the count is derived); `FIELD_RESOLUTION_MIN = 16`;
  `FIELD_RESOLUTION_MAX = 64` (binds above about radius 2.9; `65^3` corners is
  1.1 MB, paid only by rocks that are hit); `FIELD_MARGIN = 1.08`.
  `crates/nova_scenario/src/objects/asteroid_carve.rs:60-112`.
- Measured on one desktop core at `64^3`: 12.7 ms to seed, 10.7 ms to remesh
  26,000 triangles, 10.0 ms to rebuild the collider; against 2.3 / 1.6 / 2.2 at
  `32^3`. `asteroid_carve.rs:86-93`.
- The field is the ONE description of a rock's shape: the spawn mesh, the
  collider and the carve field all come off `pristine_field`, and the field is
  dropped after meshing and rebuilt on the first hit.
  `asteroid_carve.rs:10-31,181-226`.
- The remesh is SYNCHRONOUS today, and says so. `asteroid_carve.rs:350-354`.
- A remesh waits until the grid loses a cell (quantized `meshed_volume`), not on
  every mark. `asteroid_carve.rs:135-139,337-348`.
- Destruction: `remaining_world < CHUNK_MIN_VOLUME` or an empty surface ->
  final dust + `IntegrityDestroyMarker` + `OnDestroyed(id, "asteroid")` +
  despawn root. `asteroid_carve.rs:483-508`.
- `BodyRadius` only ever SHRINKS. `asteroid_carve.rs:543-550`.
- Severed islands (`split_off_islands`) become their own rigid bodies carrying
  `v + omega x r`; pieces under `CHUNK_MIN_VOLUME` become dust.
  `asteroid_carve.rs:257-276,445-453`.
- Density rides along unchanged, so a carved rock is a lighter rock.
  `asteroid_carve.rs:532-534`.

## Asteroids never use the slicer

Confirmed against the tree and against the coordinator's note. A rock does not
go through `mesh/explode.rs` at all. It has no `Health`, its volume falls until
the field is exhausted, and then it throws its severed islands and despawns.
`crates/nova_scenario/src/objects/asteroid.rs:214` is the load-bearing comment:
"The field is the rock's only durability." The destruction path even says so at
`asteroid_carve.rs:497-499` - it reuses the destruction CUE seam "without opting
into its health or random-fragment finale".

So any page claiming a rock has hit points, or dies at zero health, is a lie
regardless of what happens to ship sections. Fixed at
`web/src/wiki/scenarios.md:9` and confirmed already correct at
`web/src/create/objects.md`.

## Section death: deliberately NOT documented

Coordinator direction mid-task: the slicer (`crates/nova_gameplay/src/mesh/
explode.rs`) is being deleted and replaced by plain DETACHMENT - a destroyed
section, shell or greeble becomes its own rigid body, keeps the mesh it already
had, tumbles away and despawns on a timer. So this task does NOT write up the
fragmentation stage.

- Player wiki says only the OUTCOME, which survives the change: the part comes
  off, tumbles away, and the wreckage clears after a while.
- `/create/` needs nothing. Verified: no fragment count and no fragment lifetime
  is authorable anywhere. `BODY_FRAGMENT_BUDGET` and `FINALE_BODY_BUDGET` are
  Rust consts, and no section, ship or scenario config carries a fragment field
  (grepped `crates/nova_ship/src/sections/base_section.rs`,
  `crates/nova_scenario/src/objects/`, `assets/base/**`, `webmods/**`,
  `assets/mods/**`).
- Dev book: `docs/sections.md` has a HOLE where section death goes. The false
  claim there (that the finale chooses between real fragments and a generic cube
  burst) is struck, because a lie cannot be left standing, but nothing describes
  the replacement and the page says so in one line.

## Torpedo fuze

- `CONTACT_FUZE = 1.0` unit to the target's SKIN.
  `crates/nova_ship/src/sections/torpedo_section/projectile.rs:65-76`.
- `contact_reach(speed, dt) = CONTACT_FUZE.max(speed * dt)`, so a fast closer
  cannot step over the window. `projectile.rs:78-88`.
- Distance is `SpatialQuery::project_point_predicate`, solid, filtered to the
  colliders avian links to THAT body. `projectile.rs:90-111`.
- A locked BODY fuzes on its skin; a bare aim POINT (no `TorpedoTargetEntity`,
  or a target that died in flight) still fuzes at `blast.radius * 0.5`.
  `projectile.rs:113-130,181-186`.
- `weave_fade` band: terminal `blast_radius * 0.5`, full `blast_radius * 3.0`.
  Measured off the BLAST RADIUS and explicitly NOT off the fuze.
  `projectile.rs:322-333`.

## Turret catalog

- Exactly two prototypes: `pdc_kinetic_turret_section` (Kinetic 4.0/hit) and
  `pdc_pierce_turret_section` (Pierce 2.0/hit = kinetic * 0.5), 130 health,
  mount box `PDC_TURRET_SIZE = 0.5`.
  `crates/nova_authoring/src/base_content/sections/standard.rs:56-78,429-448`.
- `PDC_KINETIC_SECTION_ID` is "the one turret every shipped craft mounts".
  `standard.rs:73-78`.
- One projectile lifetime in the catalog: `projectile_lifetime: 2.0`
  (`standard.rs:288`). There is no second turret with its own reach.
- The scavenger grade is now `ENEMY_TURRET_HEALTH = 60.0` applied as a
  `SetHealth` modification - "the one thing left of the old `_light` turret
  prototype ... There is one turret in the catalog now".
  `crates/nova_authoring/src/base_content/ships/shared.rs:434-442`.
- No `better_turret`, `light_turret` or per-craft `*_turret_*` id survives
  anywhere in `assets/base/**` or `crates/**`. Verified by grep.

## Related constants the docs quote

- `DEFAULT_STRUCTURAL_COLLAPSE_THRESHOLD = 0.05` (five percent), and no shipped
  ship overrides it. `crates/nova_ship/src/sections/integrity.rs:22-27`.
- The combat readout hides the health bar for a target with no `Health`, which
  is now every asteroid. `crates/nova_hud/src/torpedo_target.rs:462-465`.

# Task record claims that turned out WRONG

1. "A body now wears its damage in its own GEOMETRY." Half true, and the half
   that is false is the important one. ONLY asteroids change shape. Ships
   explicitly do NOT: `damage_effects.rs:32-37` states "NO SECTION LOSES
   GEOMETRY", and the `Carve` effect was built and then removed. On a ship the
   marks drive WHERE shards are thrown, not what shape the hull is.
2. "The Better turret, the Light turret and ten per-craft turret prototypes
   were REMOVED ... Any page naming a removed prototype id is now actively
   wrong." The prune landed in `b0f13908`, an ANCESTOR of `0ee9cbb0`, and it
   carried most of its own doc work. `web/src/` and `docs/` were already free of
   `better_turret` and `light_turret` before this task. Two survivors, both
   prose rather than ids: `web/src/create/base-content.md:85-89` still said the
   ten per-craft prototypes "stay in the catalog", contradicting the same page
   27 lines later; `web/src/create/sections.md:590` still advised swapping in a
   `_light` turret variant. `web/src/wiki/combat-weapons.md:26` still gave a
   "scavenger turret" its own reach. So the blast radius was three sentences,
   not a catalog.
3. "the carve cost model in `asteroid_carve.rs`'s `FIELD_RESOLUTION_MAX` doc is
   currently the best description of carve cost anywhere in the tree". It is the
   best description of the FIELD's cost. The material cost model is
   `DAMAGE_PER_UNIT_VOLUME` in `carve.rs`, which is a different thing and the
   one a reader actually needs first.

# Defects found outside the doc surfaces (not fixed here)

- `crates/nova_gameplay/src/integrity/chunk.rs:59` says "One cubic unit is 80
  hit points at the cladding's toughness". `DAMAGE_PER_UNIT_VOLUME` is 8.0. The
  comment kept the pre-rework figure.
- `crates/nova_ship/src/sections/torpedo_section/projectile.rs:199` says the
  blast "obeys the resistance table". The per-section resistance table was
  removed this cycle.
- `examples/screenshots/damage_levels.rs:483` spawns
  `"light_turret_section"`, a prototype that no longer exists in the catalog.
- `CHANGELOG.md` `[Unreleased]` carries superseded entries that state the OLD
  contract inside the same release block: `Scorch` / default `[Scorch]`, "Hull
  sections CARVE ... Authorable as `Carve`", "The `racer_turret_*` prototypes
  stay in the catalog", "The per-craft `*_turret_*` prototypes are catalog-only
  now". Each is contradicted by a later entry in the same block.
