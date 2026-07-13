# Retro: Lock reticle on beacons sizes to the trigger sensor

- TASK: 20260712-154318
- BRANCH: beacon-reticle-size (landed as a9097e0)
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- Diagnosis was instant because last cycle's R1.1 had already mapped the
  mechanism (ApparentSize unions sensor AABBs); the playtest report
  ("really big target thingy") matched the reticle-on-locked-beacon path
  in one code read. The fix, tests and doc updates reused R1.1's own
  analysis.
- The strip-the-Sensor delivery guard makes the exclusion test unable to
  pass vacuously.

## What went wrong

- This bug was VISIBLE at R1.1 time and deferred on the grounds that "no
  other ApparentSize consumer anchors a sensor-only entity" - but that
  checked the CONSUMER list, not the ANCHORABLE-entity list; beacons are
  deliberately lockable (beat 4 depends on it), so reticle-on-beacon was
  reachable the whole time. One playtest round later it cost a second
  cycle.

## What to improve next time

- When deferring a generic fix because "current consumers are safe",
  enumerate the entities the surviving code path can MEET (what can be
  locked/anchored/targeted), not the call sites that exist today.

## Action items

- [x] Ledger: folded into `advertised-but-unwired`'s
      generic-mode-vs-this-anchor variant as the deferred-fix corollary.
