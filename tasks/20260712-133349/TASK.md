# Multi-type magazines, reload and bullet-type switching

- STATUS: OPEN
- PRIORITY: 44
- TAGS: v0.5.0,weapons,spike


Spike: docs/spikes/20260712-133135-weapon-and-damage-type-variety.md

Grow SectionAmmo (crates/nova_gameplay/src/sections/ammo.rs) from a single pool
into per-type magazines plus a selected type, with a reload/switch action; firing
spends the selected pool and stamps the spawned projectile's ProjectileDamage.kind.
This is where "reload different bullet types" ships. Depends on the typed-damage
core (20260712-133343) for DamageType/ProjectileDamage.
