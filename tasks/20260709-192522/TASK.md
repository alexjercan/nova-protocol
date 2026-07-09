# Focus dwell + component fine-lock state and selection

- STATUS: OPEN
- PRIORITY: 56
- TAGS: v0.4.0,targeting,gameplay,spike


Spike: docs/spikes/20260709-192358-component-lock-vats-lite.md

The core mechanic: a focus timer that accumulates while the ship lock stays on
the same entity (focused when >= FOCUS_TIME, reset on lock change/break), and
a component fine-lock (Option<section Entity> of the locked ship) that is only
available while focused. Selection is aim-snap by default (nearest live
section to the crosshair ray, with hysteresis) plus cycle next/prev keys that
pin the selection and suppress snap for a short window (snap resumes ~2 s
after the last cycle press or when the pinned section dies). Validity: the
section must stay an attached child with Health; decide at plan time whether
`SectionInactiveMarker` (disabled-in-place) sections stay lockable. Depends
on: 20260709-192503 (acquisition) for the lock semantics it rides on.
