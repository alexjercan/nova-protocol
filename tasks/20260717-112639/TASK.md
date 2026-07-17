# Rework broadside pacing: act-split retry and hardened cover ring

- STATUS: OPEN
- PRIORITY: 52
- TAGS: spike,v0.7.0,scenario,content,balance

Goal: broadside is "hard but playable" - keep its good bones (light-turret
corvettes, 550u spawns, the gunship's 1177u approach as breathing room) and
fix the two unfair parts: dying to the act-2 gunship re-earns the whole
corvette fight, and the 24-rock cover ring is paper (health 100 =
0.25s of better-turret fire).

Direction notes:
- Act-split retry: corvette act and gunship act as separate hidden
  scenarios chained via NextScenario, so defeat retries the current act.
- Harden part of the cover ring to invulnerable: true so the gunship
  fight has persistent hard cover per attack bearing; keep some
  destructible rocks as chaff.
- Keep the gunship spawn distance and torpedo setpiece; tune only if the
  balance audit rig (tasks/20260717-112656) says the PDC-screen ask is
  beyond the intended skill bar.

Spike: tasks/20260717-111808/SPIKE.md (findings F4/F7)
