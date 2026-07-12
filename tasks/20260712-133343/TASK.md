# Nova typed-damage core: DamageType, resistance table, own-the-trigger application

- STATUS: OPEN
- PRIORITY: 46
- TAGS: v0.5.0,weapons,health,spike


Spike: docs/spikes/20260712-133135-weapon-and-damage-type-variety.md

Foundation of the combat-depth typed-damage pass. Add a nova DamageType enum
(Kinetic/ArmorPiercing/Emp/Explosive), a ProjectileDamage { amount, kind }
component on projectiles, and a nova resistance table keyed by (SectionKind,
DamageType). Nova's weapon-hit callsites pre-scale (amount x resistance) and THEN
trigger bcs HealthApplyDamage - owning the trigger sidesteps Bevy 0.19's
arbitrary observer order, so no racing bcs's on_damage subtractor. Neutralize
bcs's emergent kinetic damage for weapons (leading candidate: near-zero
projectile Mass; see the spike's open questions). Do NOT modify bcs.

Composes with per-section-type health (task 20260525-133004, untyped durability)
as the orthogonal typed axis, and with SectionAmmo (the loaded bullet type picks
the ProjectileDamage). Blocks the other two tasks in this family.
