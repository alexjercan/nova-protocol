# Audio SFX: thruster hum audible from far away (attenuation bug)

- STATUS: OPEN
- PRIORITY: 90
- TAGS: bug,audio,feedback

Goal: investigate and fix SFX distance attenuation for the thruster hum.

Reported 2026-07-11 (user playtest): the thruster hum of other ships is
audible from far away, which is wrong. The turret shoot sound attenuates
correctly at the same distances, so the distance-attenuation path works for
one-shot SFX; the bug is likely specific to the looping, throttle-tracking
thruster source (crates/nova_gameplay/src/audio.rs, task 20260708-162011
introduced it). Investigate diagnostic-first: trace the actual attenuation
values applied to a distant thruster loop versus a turret shot before
theorizing (see docs/retros/LESSONS.md, diagnostic-first).

Notes:
- Comparison anchor: turret shoot = correct, thruster loop = wrong.
- Suspects to check only after the trace: loop volume never re-evaluated
  after spawn; listener distance computed once; throttle gain overriding
  attenuation; loop attached to the wrong entity.

