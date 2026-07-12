# Ammo HUD readout for weapon sections

- STATUS: OPEN
- PRIORITY: 44
- TAGS: v0.5.0,weapons,hud


Discovered during task 20260525-133025 (ammo limit logic). Weapons now carry a
finite `SectionAmmo { rounds, capacity }` (crates/nova_gameplay/src/sections/ammo.rs)
that depletes as they fire, but nothing shows it: the player can run a turret or
torpedo bay dry with no on-screen feedback and (until task 20260708-162005 adds
reloading) no way to refill. Add a HUD readout of remaining rounds per weapon,
building on the existing HUD (hud/mod.rs setup_hud_health pattern, HudTier) and
the weapons-HUD spike docs/spikes/20260708-165647-weapons-hud.md.

Depends in spirit on the reload/damage-type pass (20260708-162005): if that
reshapes SectionAmmo into per-bullet-type magazines, the readout should show the
selected type's count, so land the HUD after that direction is settled or design
it to accommodate multiple pools.
