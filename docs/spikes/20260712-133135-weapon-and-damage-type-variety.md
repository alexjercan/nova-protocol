# Spike: How should nova add damage types, resistances, bullet types and alt-fire?

- DATE: 20260712-133135
- STATUS: RECOMMENDED
- TAGS: spike, weapons, health, combat

## Question

Nova has two weapons today (turret bullets, guided torpedoes), both single-mode,
and a single emergent damage model. Task 20260708-162005 wants combat depth:
damage types (armor-piercing, EMP, explosive), the section resistances that make
them matter, alt-fire modes, and multiple bullet types you can reload/switch
between. How should that be built - specifically, where does the typed-damage +
resistance math live, given the two hard constraints below - and in what order?

A good answer names a concrete mechanism for typed damage + resistance that does
not fight the engine, shows how it composes with the ammo foundation
(task 20260525-133025) and the per-section-type health (task 20260525-133004),
and slices the work into direction-level tasks a later `/flow` can plan.

## Context

Everything here was traced in the combat-depth session on 2026-07-12 (the same
session that shipped ammo and per-type health); file:line references are current
as of commit `1c1960e`.

**Damage flow (bcs-owned).** Both damage sources live in
`bevy_common_systems` (bcs) and converge on ONE event:

- `on_impact_collision_deal_damage` (bcs integrity/plugin.rs:83) computes KINETIC
  damage from the two bodies' masses and relative velocity and triggers
  `HealthApplyDamage { entity: <hit collider>, source, amount }`.
- `on_blast_collision_deal_damage` (bcs integrity/plugin.rs:153) computes blast
  falloff from a `BlastDamageConfig` and triggers the same event.
- bcs `on_damage` (bcs health/mod.rs:119) is the SINGLE observer that subtracts:
  it clamps `amount` to the node's remaining health, subtracts, sets
  `HealthZeroMarker` at zero, and re-propagates the clamped amount up `ChildOf`
  (this is what nova's `aggregate_ship_health` and the overkill-clamp semantics
  in `integrity/glue.rs` depend on).

`HealthApplyDamage` (bcs health/mod.rs:70) carries only `{ entity, source,
amount }` - **no damage type**. bcs's own code has a TODO right there asking for
exactly that (health/mod.rs:74).

**The two hard constraints.**

1. **Do not modify bcs** (user directive, 2026-07-12): the typed-damage /
   resistance / health system is nova-side. bcs stays the generic HP + integrity
   core.
2. **Bevy 0.19 observer order is arbitrary** (bevy_ecs observer docs,
   distributed_storage.rs:153-155: "make no assumptions about their execution
   order"). So a nova observer CANNOT reliably run before bcs's `on_damage` to
   pre-scale `amount`. Any design that "adds a second observer on
   `HealthApplyDamage` that multiplies the amount" is broken - it will race the
   subtractor and lose half the time.

The corollary that unlocks everything: the way to influence the amount without
racing is to **own the trigger** - compute the final, already-scaled amount and
THEN `trigger(HealthApplyDamage{..})`. bcs's single observer then just subtracts
what nova already decided. Nova already owns the weapon-hit callsites
(`despawn_bullet_on_hit` on the bullet's `CollisionStart`, turret_section.rs:298;
`torpedo_detonate_system`, torpedo_section/projectile.rs:57), so nova is
well-placed to be that trigger.

**What exists to build on.**

- `SectionAmmo { rounds, capacity }` (sections/ammo.rs) on each weapon SECTION
  entity; both fire systems consume-and-gate. Opt-in via `ammo_capacity` on the
  weapon config. Deliberately shaped so reload / multi-magazine layers on top.
- Per-section-type health baselines (nova_assets/sections.rs): thruster 70,
  controller/torpedo 100, turret 130, hull 200/60. This is the UNTYPED durability
  axis; typed resistance is the orthogonal axis this spike adds.
- `SectionKind` enum (Hull/Thruster/Controller/Turret/Torpedo,
  sections/base_section.rs:34) - the natural key for a resistance table.
- Turret bullet: `Sensor` + `CollisionEventsEnabled` + `Mass(projectile_mass)` +
  `ProjectileOwner`; kinetic damage is emergent from mass x velocity. Torpedo:
  guided body, detonates into a `blast_damage(BlastDamageConfig{max_damage})`
  sensor. Weapon params live in `TurretSectionConfig` / `TorpedoSectionConfig`.

## Options considered

### A. Nova typed-damage layer; bcs stays the HP + integrity store (RECOMMENDED)

Nova authors weapon damage as an explicit typed value and owns the application:

- A nova `DamageType` enum (`Kinetic | ArmorPiercing | Emp | Explosive`, growable).
- Projectiles carry a `ProjectileDamage { amount, kind }` component set from the
  weapon (and the loaded bullet type). Weapon damage becomes AUTHORED, not
  emergent from bullet mass - which the game wants anyway, because "AP does X,
  EMP does Y" cannot come out of a single kinetic formula.
- A nova resistance table: a multiplier per `(SectionKind, DamageType)` (e.g. EMP
  x3 vs Controller, x0.2 vs Thruster; AP x1.5 vs Hull). Nova data - a small
  resource or `const fn` - generalizing task 2's untyped baselines to a typed
  axis.
- Nova's existing weapon-hit callsites compute
  `final = damage.amount * resistance(section_kind, damage.kind)` and
  `trigger(HealthApplyDamage { entity: section, amount: final })`. Because nova
  owns the trigger, there is NO observer race; bcs's `on_damage` subtracts the
  pre-scaled amount and the whole integrity/destruction pipeline is reused
  unchanged.

Pros: respects both constraints; reuses the battle-tested integrity pipeline and
its overkill-clamp/aggregate semantics; damage becomes tunable per bullet type;
composes directly with `SectionAmmo` (the loaded bullet type picks the
`ProjectileDamage`) and with task-2 health (durability x resistance are two clean
multipliers). Smallest change that delivers the feature.

Cons / unknowns: bcs's `on_impact_collision_deal_damage` still fires for a
bullet->section contact and would ALSO deal its kinetic amount - double-counting
weapon damage. Nova must neutralize bcs's emergent contribution for weapons. The
neutralization is the main open question (see below); the leading candidate is to
give projectiles a near-zero `Mass` so bcs's kinetic term is ~0 (bullets are
`Sensor`s with no solver response, so mass is otherwise unused), leaving nova's
authored typed amount as the only weapon damage. Non-weapon kinetic collisions
(a ship ramming an asteroid) keep bcs's kinetic model untouched.

### B. Full nova health + damage + integrity (replace bcs Health)

Nova stops using bcs `Health`/`HealthPlugin`/integrity damage entirely and grows
its own `Health`, damage event (typed), resistance, zero/destroy pipeline.

Pros: total control; the typed event is first-class; no neutralization hacks.
Cons: a large, high-risk refactor - it re-implements and re-tunes everything in
`integrity/glue.rs` + `integrity/explode.rs` + `aggregate_ship_health` and the
carefully-tested overkill-clamp behavior (multiple prior retros). Enormous blast
radius for what is, today, "make some hits stronger against some sections." Not
justified by the current need; revisit only if bcs's HP model becomes a real
straitjacket.

### C. Resistance-only overlay on bcs kinetic (the naive approach - REJECTED)

Keep bcs computing the base amount; add a nova observer on `HealthApplyDamage`
that multiplies by resistance.

Rejected outright by constraint 2: Bevy 0.19 gives no ordering between that
observer and bcs's subtractor, so the multiply lands before or after the subtract
unpredictably. Documented here so a future session does not re-propose it -
"just add an observer" is exactly the trap. Option A is C done correctly: scale
BEFORE the trigger, in code nova owns, instead of in a racing observer.

### Reloading + bullet types + alt-fire (spans A or B)

Independent of the damage-sink choice, the ammo/bullet-type model the user asked
for ("reloading different bullet types"):

- Generalize `SectionAmmo` from one pool to per-type magazines plus a selected
  type: `{ magazines: Map<AmmoKind, u32>, selected: AmmoKind, ... }` (AmmoKind
  can just be `DamageType`, or a superset if two bullet types share a damage
  type). Firing spends the selected pool and stamps the spawned projectile's
  `ProjectileDamage.kind`. Reload refills / switches the selected type (optionally
  behind a reload timer). This is the shape the single-pool `SectionAmmo` was
  deliberately built to grow into.
- Alt-fire: a weapon carries a primary and secondary fire profile (a second
  damage/bullet type, or a firing pattern like a charged shot). Needs a second
  fire input binding and a `selected profile` on the weapon; the fire systems
  read the active profile. Simplest first cut: secondary fire = the secondary
  bullet type.

## Recommendation

**Option A**, phased. It is the only option that satisfies both constraints
without a heavy refactor, and it turns the observer-order wall into a clean
design (own the trigger, pre-scale, let bcs subtract). Concretely, in dependency
order:

1. **Typed-damage core.** `DamageType` enum; `ProjectileDamage { amount, kind }`
   on projectiles; a nova resistance table keyed by `(SectionKind, DamageType)`;
   nova's weapon-hit callsites pre-scale and trigger `HealthApplyDamage`;
   neutralize bcs's emergent kinetic for weapons. This is the foundation - types
   and resistances exist and matter, even before any new bullet or fire mode.
2. **Bullet types + reload.** Grow `SectionAmmo` into per-type magazines with a
   selected type and a reload/switch action; firing stamps the projectile kind.
   This is where "reload different bullet types" ships.
3. **Alt-fire modes.** Primary/secondary fire profiles + the input to drive them.

Task 2's per-type HEALTH stays as the untyped durability axis; this spike adds the
orthogonal typed-resistance axis. A section's effective damage taken becomes
`incoming x type_resistance`, subtracted from its type-tuned health - two legible
knobs instead of one conflated number.

## Open questions

- **Neutralizing bcs's emergent weapon damage (blocks phase 1).** Leading
  candidate: near-zero projectile `Mass` so bcs's kinetic term vanishes and nova's
  authored amount is the only weapon damage. Verify in a headless rig that a
  ~0-mass sensor bullet yields ~0 from `on_impact_collision_deal_damage` and that
  physics/knockback (already zero for sensors) is unaffected. Fallback if that is
  too fiddly: nova stops adding bcs's two damage observers and provides its own
  impact/blast sources that route through the typed path (a bigger slice of
  Option A, leaning toward B). Resolve during phase-1 `/plan`.
- **Torpedo blast typing.** The blast is a bcs `blast_damage` sensor whose damage
  bcs computes; typing it the Option-A way means nova computes the blast amount
  (it already knows radius/falloff intent) and triggers the typed
  `HealthApplyDamage`, rather than delegating to bcs's blast observer. Confirm
  nova can drive falloff itself cleanly.
- **AmmoKind vs DamageType.** Are they the same set, or can two bullet types
  (e.g. two kinetic loads) share a damage type? Pick when designing phase 2.
- **Resistance values.** The actual multiplier table (which type is strong/weak
  vs which section) is a balance question for playtest; phase 1 ships a defensible
  first table behind the mechanism.
- **HUD.** Damage types, the selected bullet type, and per-type ammo want HUD
  surfacing; folds into the already-filed ammo HUD task 20260712-131348.

## Next steps

Direction-level tasks seeded from the recommendation (for `/plan` to break into
steps). The umbrella task 20260708-162005 is superseded by these three and should
be closed pointing here.

- tatr 20260712-133343: nova typed-damage core (DamageType + resistance table +
  own-the-trigger application, neutralize bcs emergent weapon damage)
- tatr 20260712-133349: multi-type magazines, reload and bullet-type switching
  (extends SectionAmmo)
- tatr 20260712-133356: alt-fire modes (primary/secondary fire profiles + input)

## Fix record

(Appended by each implementing task as it lands.)
