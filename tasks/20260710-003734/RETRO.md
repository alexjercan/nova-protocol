# Retro: Shot-down torpedo dies without its blast

- TASK: 20260710-003734
- BRANCH: fix/torpedo-shootdown (squash-merged, see git log); reopened on
  fix/torpedo-shootdown-defer (merged)
- REVIEW ROUNDS: 2 (round 1 APPROVE, then a live-game panic reopened the
  task; round 2 APPROVE on the deferred-despawn fix)

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

## Round 2: the fix itself crashed the game

The round-1 observer despawned the torpedo root inside the
HealthZeroMarker flush. The integrity pipeline reacts to the SAME marker
and had already queued inserts (IntegrityDisabledMarker) for the dying
section in that flush; after the despawn those commands hit a dead entity
and panicked inside avian's collision-event flush. Shipped fix: two-step
kill - the observer only inserts TorpedoShotDownMarker, a chained Update
system does the try_despawn a pass later, and the detonate system
excludes marked roots so the fuze stays quiet in the gap.

### What went well

- The user's crash trace mapped one-to-one onto a diagnosis (the failing
  command, the flush it ran in, the entity generation) - no reproduction
  hunt needed.
- The new regression test reproduces the production pattern (another
  insert on the dying section in the same flush as the zero-health
  marker) and panics on the round-1 code, so the race is pinned, not just
  patched.

### What went wrong

- **The race escaped both implementation and review.** Root cause: the
  round-1 tests built minimal apps with only the torpedo observer
  registered, so nothing else reacted to HealthZeroMarker and the flush
  was empty except for our own despawn. The test environment under-modeled
  the production flush cascade, and review trusted it ("no findings").
- The round-1 retro said "nothing in-cycle" hours before the crash
  report. A retro on a same-day fix is written before the fix has
  actually been soaked in play.

### What to improve next time

- **Never despawn directly from an observer keyed on a shared marker.**
  Other pipelines react to the same marker in the same flush and may have
  commands queued for the entities being despawned. Defer: mark in the
  observer, despawn from a scheduled system.
- When a fix hooks a marker that other plugins also observe, the
  regression test must include a same-flush neighbor (another insert on
  the same entity) - a lone-observer app proves nothing about command
  ordering.
