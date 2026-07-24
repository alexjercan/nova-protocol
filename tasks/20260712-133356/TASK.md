# Alt-fire modes: primary/secondary fire profiles and input

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog, weapons, spike

## Closed (2026-07-24, not pursuing)

Closed during v0.9.0 planning triage. Owner decided (2026-07-12) not to pursue
alt-fire profiles; station-based bullet/weapon switching is the preferred
direction instead. Filed decision preserved here rather than lingering open.

Spike: tasks/20260712-133135/SPIKE.md

DEFERRED to backlog (user, 2026-07-12): not pursuing alt-fire for now. One bullet
type per weapon is enough; the intended way to change a weapon's loaded type is at
a space station / in ship management (built on the LoadedBullet slot from task
20260712-133349), NOT a secondary-fire input. Revisit alt-fire only if a real
need appears; the station-based bullet-type switch is the near-term direction and
should get its own backlog task when scoped.

Give weapons a primary and secondary fire profile (a second bullet/damage type or
firing pattern), with the input to drive them; the turret/torpedo fire systems
read the active profile. Simplest first cut: secondary fire = the secondary
bullet type. Depends on the typed-damage core (20260712-133343) and dovetails with the
multi-type magazines (20260712-133349).
