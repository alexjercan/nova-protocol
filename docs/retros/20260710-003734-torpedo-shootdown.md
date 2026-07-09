# Retro: Shot-down torpedo dies without its blast

- TASK: 20260710-003734
- BRANCH: fix/torpedo-shootdown (squash-merged, see git log)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

Second live user report folded into the AI arc mid-flow (after the PDC
priority decision) - and the report was load-bearing: point defense is
pointless if a hit torpedo keeps coming.

## What went well

- **Reframed the ask before fixing.** "Make torpedoes easier to destroy"
  read like a health/damage tuning request; five minutes of reading showed
  the body sections already die to one bullet - the ROOT just never
  noticed. The right fix was wiring, not numbers; tuning HP would have
  changed nothing.
- **The no-blast rule was made explicit and tested.** The quiet-death test
  asserts zero BlastDamageMarker entities through the real damage
  pipeline, so the design intent (defeating the warhead is the point of
  shooting it down) is pinned, not implied.
- **The guard test earned its place**: an observer keyed on a generic
  marker (HealthZeroMarker) is one wrong filter away from despawning
  ships; the non-torpedo-parent test makes that regression loud.

## What went wrong

- Nothing in-cycle. The suppressed section-debris on shoot-down kills
  (despawn happens before the integrity destroy stage) is a known,
  recorded polish gap, deliberately not padded into this fix.

## What to improve next time

- Keep translating symptom reports into pipeline questions ("who is
  supposed to tell X that Y happened?") before reaching for tuning knobs.

## Action items

- Playtest the shoot-down feel; if kills read as too silent, file a juice
  task for a small kill flash (noted in the task Resolution).
