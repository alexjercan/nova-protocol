# Alt-fire modes: primary/secondary fire profiles and input

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.6.0,weapons,spike


Spike: tasks/20260712-133135/SPIKE.md

DEFERRED to v0.6.0 (user, 2026-07-12): not pursuing alt-fire for now. One bullet
type per weapon is enough; the intended way to change a weapon's loaded type is at
a space station / in ship management (built on the LoadedBullet slot from task
20260712-133349), NOT a secondary-fire input. Revisit alt-fire only if a real
need appears; the station-based bullet-type switch is the near-term direction and
should get its own v0.6.0 task when scoped.

Give weapons a primary and secondary fire profile (a second bullet/damage type or
firing pattern), with the input to drive them; the turret/torpedo fire systems
read the active profile. Simplest first cut: secondary fire = the secondary
bullet type. Depends on the typed-damage core (20260712-133343) and dovetails with the
multi-type magazines (20260712-133349).
