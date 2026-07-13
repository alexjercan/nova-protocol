# Spike: What concrete damage types and bullet types should nova have, and what is the first resistance table?

- DATE: 20260712-160505
- STATUS: RECOMMENDED
- TAGS: spike, weapons, health, combat, balance

## Question

The architecture spike (tasks/20260712-133135/SPIKE.md)
settled *how* typed damage is applied (Option A: nova authors a
`ProjectileDamage { amount, kind }`, pre-scales by a `(SectionKind, DamageType)`
resistance table, and owns the `HealthApplyDamage` trigger so bcs never races).
It deliberately left the *content* open - it lists four types in passing
(`Kinetic | ArmorPiercing | Emp | Explosive`) and files two balance open
questions: "AmmoKind vs DamageType?" and "the actual resistance multipliers".

This spike answers the content half so phase 1 ships behind a real, defensible
first table instead of a placeholder:

1. Which damage types exist in the first cut, and what does each one *mean* (its
   fantasy and its intended prey)?
2. How do bullet types relate to damage types - are they the same set?
3. What are the actual resistance multipliers, keyed by `(SectionKind,
   DamageType)`, and why?

A good answer is a filled-in table with a one-line rationale per type and a
stated design invariant that keeps today's combat feel intact, concrete enough
that the phase-1 `/plan` drops it in without re-litigating balance.

## Context

Grounded in the code as of this session (same combat-depth pass that shipped
ammo and per-type health):

- **Sections** (`sections/base_section.rs:34`): `Hull | Thruster | Controller |
  Turret | Torpedo`. These are the resistance-table rows.
- **Untyped durability baselines** (`nova_assets/sections.rs`): Thruster 70,
  Controller 100, Torpedo 100, Turret 130, Hull 200 (reinforced) / 60 (light).
  This is the orthogonal axis: how much HP a section has. The typed axis this
  spike designs is a *multiplier* on incoming damage, applied before subtraction.
  Effective hits-to-kill = `health / (base_amount x resistance)`.
- **Weapons today.** Turret bullet: emergent kinetic from `projectile_mass`
  (0.1 better / 0.05 light) x relative velocity; high fire rate (100 / 25 rps).
  Torpedo: a `blast_damage: 100.0` area detonation, `blast_radius: 30.0`. Both
  are the only two damage sources in the game.
- Resistance is a pure multiplier on `ProjectileDamage.amount`; `>1.0` means the
  section takes MORE (vulnerable), `<1.0` means it takes LESS (resistant).

## Options considered

### Type set A: the four-type cut - Kinetic / ArmorPiercing / EMP / Explosive (RECOMMENDED)

Maps one-to-one onto the user's "impact, AP, explosion, etc":

- **Kinetic (Impact).** The plain slug - mass driver, autocannon, PDC round.
  The generalist and the *reference type*: it is what the turret fires today and
  what the untyped health baselines were tuned against. Design invariant:
  **Kinetic resistance is 1.0 for every section.** That makes the typed system
  feel-preserving - a default kinetic turret behaves exactly as it does today,
  and every other type is defined as a deviation from that known-good baseline.
- **ArmorPiercing (AP).** A dense penetrator built to defeat plate and hardened
  mounts. Strong vs armored sections (Turret, reinforced Hull), *weak* vs soft
  exposed targets (over-penetrates a thin Thruster, wasting energy), neutral vs
  electronics. Prey: armor.
- **EMP.** Anti-electronics. Devastating vs the Controller (the command core is
  pure electronics) and strong vs the Turret (targeting/servo electronics),
  near-inert vs dumb structural Hull and low vs the mechanical Thruster. Prey:
  the command core. This is the "disable, don't demolish" type.
- **Explosive.** Concussive area damage - the torpedo, and any future frag/rocket
  round. Shreds exposed fragile surfaces (Thruster), neutral vs general
  structure, *bounces off* a hardened armored mount (Turret). Prey: exposed
  sections.

Each non-Kinetic type has one clear best target (AP -> armor, EMP -> Controller,
Explosive -> Thruster), so weapon/bullet choice becomes a legible loadout
decision rather than "bigger number wins". Pros: four types is enough for
rock-paper-scissors depth without a combinatorial table; each maps to an obvious
real weapon; folds the existing torpedo (Explosive) and turret (Kinetic) in with
zero reclassification. Cons: no damage-over-time / status flavor (see below).

### Type set B: add a status/DoT family (Incendiary, Corrosive) now

Also add types that apply an effect over time (burn ticks, armor-strip debuff).

Pros: richer fantasy. Cons: a DoT/status type is NOT a multiplier - it needs a
whole status-effect subsystem (timers, stacking, per-section effect state) that
the `(SectionKind, DamageType) -> f32` model cannot express. Bolting it into the
typed-multiplier pass would conflate two mechanisms. Rejected for phase 1;
recorded as a future *axis* (status effects) that layers on top of typed damage
rather than expanding this enum. When it comes, it is its own spike.

### Type set C: one type per weapon, no shared taxonomy

Turret damage, torpedo damage, each bespoke. Rejected: this is where the game is
today (emergent, per-weapon), and it is exactly what the architecture spike set
out to replace. It gives no shared resistance axis, so sections cannot be "weak
to X, strong to Y" - the entire point of the pass.

### Bullet types vs damage types: same set, or superset?

The architecture spike's open question. Two sub-options:

- **AmmoKind == DamageType (RECOMMENDED for now).** A loaded bullet type *is* a
  damage type; the magazine's selected kind stamps `ProjectileDamage.kind`
  directly. Simplest, and phase 1 needs no `AmmoKind` at all - just `DamageType`.
- **AmmoKind as a superset.** Two bullet loads could share a damage type (a cheap
  low-velocity kinetic slug and an expensive high-velocity kinetic sabot, both
  Kinetic but different `amount`/speed). Only justified once a concrete
  second-same-type round exists.

Recommendation: keep them 1:1 through phase 1; shape `ProjectileDamage { amount,
kind }` so `amount` already carries the per-round magnitude. If phase 2's
magazines ever need two rounds of one type, introduce `AmmoKind` then as a thin
superset - it does not change the damage-application code, only the magazine
key. Decide at phase 2, not now.

## Recommendation

Ship **type set A** (Kinetic / ArmorPiercing / EMP / Explosive) with
**AmmoKind == DamageType** for now, and this first resistance table. Multiplier
on incoming `amount`; `>1.0` = takes more, `<1.0` = takes less.

|            | Kinetic | ArmorPiercing | EMP  | Explosive |
|------------|:-------:|:-------------:|:----:|:---------:|
| Hull       |  1.0    |     1.5       | 0.1  |   1.0     |
| Thruster   |  1.0    |     0.75      | 0.25 |   1.5     |
| Controller |  1.0    |     1.0       | 3.0  |   1.0     |
| Turret     |  1.0    |     1.75      | 1.5  |   0.5     |
| Torpedo    |  1.0    |     1.0       | 1.25 |   1.25    |

Reading it by type (the design intent, which is what to preserve if the exact
numbers get re-tuned in playtest):

- **Kinetic - flat 1.0 everywhere (the invariant).** Preserves today's feel; the
  generalist that is never wrong but never exploits a weakness.
- **AP - beats armor, wasted on soft targets.** Peaks vs Turret (1.75) and Hull
  (1.5); penalized vs the thin Thruster (0.75, over-penetration); neutral vs the
  electronics sections.
- **EMP - crushes the command core, inert vs structure.** Controller 3.0 and
  Turret 1.5 (electronics-heavy); Hull 0.1 and Thruster 0.25 (structure and raw
  mechanism barely notice); Torpedo bay 1.25 (has launcher electronics).
- **Explosive - shreds the exposed, bounces off armor.** Thruster 1.5 (exposed,
  fragile); Turret 0.5 (hardened mount); Hull/Controller neutral; Torpedo 1.25.

Column extremes are intentional and readable: EMP's 3.0-vs-0.1 spread is the most
dramatic (a dedicated disable weapon), AP and Explosive are gentler swings around
1.0 so a wrong load is suboptimal, not useless.

Weapon mapping for phase 1: **turret bullet -> Kinetic**, **torpedo blast ->
Explosive**. Because Kinetic is 1.0 everywhere, author the turret's
`ProjectileDamage.amount` to match its current emergent per-hit kinetic value and
DPS is unchanged - the typed system is invisible until a non-Kinetic round is
introduced (phase 2). The torpedo's Explosive typing changes torpedo-vs-section
balance (0.5 vs Turret, 1.5 vs Thruster); flag that as an intended playtest
delta, not a regression.

## Open questions

- **Turret base amount.** The kinetic `amount` that reproduces today's emergent
  mass x velocity per-hit damage - measure in the phase-1 headless rig and author
  it so Kinetic-at-1.0 is exactly feel-preserving. (Resolve in phase-1 `/plan`.)
- **Torpedo falloff typing.** The architecture spike notes nova should compute
  blast falloff itself and trigger typed `HealthApplyDamage` rather than delegate
  to bcs's blast observer; the Explosive multiplier then applies to the
  falloff-scaled amount. Confirm during phase 1.
- **Exact multipliers.** The table is a defensible *first* cut; the per-cell
  numbers are a playtest knob. The type *intent* (which type beats which section)
  is the durable decision - preserve it even if the numbers move.
- **Status/DoT axis.** Incendiary/corrosive want a status-effect subsystem, not a
  multiplier. Deferred to its own future spike; do not grow `DamageType` for them.

## Next steps

This spike seeds **no new tasks** - the three-task family from the architecture
spike already exists. It *refines* phase 1 by fixing the taxonomy and table:

- tatr 20260712-133343 (typed-damage core): implement `DamageType` = { Kinetic,
  ArmorPiercing, Emp, Explosive } and ship the resistance table above; turret ->
  Kinetic, torpedo -> Explosive. THIS is the task being flowed now.
- tatr 20260712-133349 (magazines/reload): decide AmmoKind == DamageType vs
  superset here, per the "bullet types vs damage types" section.
- tatr 20260712-133356 (alt-fire): unaffected by taxonomy.

Cross-referenced from the architecture spike's family; both docs are the
combat-depth pass's shared source of truth.
