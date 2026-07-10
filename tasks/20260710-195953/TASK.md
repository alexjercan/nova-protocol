# Sensor-model lock ranges for ships (minimap follow-up)

- STATUS: OPEN
- PRIORITY: 30
- TAGS: sensors, targeting, minimap

## Goal

Second half of the signature-gated lock (20260710-195952), deliberately
parked until a minimap/sensor display exists: decide whether and how
enemy ships are lockable at long range. The user's instinct ("maybe the
enemy ship - not sure yet") wants the sensor fiction thought through
together with the minimap: if ships paint on a minimap at range, the lock
gate and the map should share one signature/sensor model rather than
inventing two.

## Notes

- Blocked on: a minimap/sensor-display task (does not exist yet - file it
  when the arc is picked up, likely with its own /spike).
- Depends on: 20260710-195952 (the signature model this extends).
- Out of the v0.5.0 sprint by user decision (2026-07-10).
