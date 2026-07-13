# Review - 20260713-140929 shakedown beat sheet v2

## Round 1 (2026-07-13)

Walked the player path beat by beat against the config, then attacked the
geometry and the event-ordering traps.

- **One-gesture audit** of all 13 objective texts: B1 pairs [W]/[X]
  (one lesson, the spike's explicit exception), B10 pairs [RMB]+[CTRL]
  (the compound combat-lock gesture, also spike-sanctioned); every other
  text is a single key or zero keys. Longest text is B10 at 18 words -
  over the 15-word target by three, accepted (it names the viewfinder,
  which IS the lesson).
- **Ordering traps swept**: the coast ring spawns with beat 7 (cannot
  fire early); its enter/exit double duty is guard-separated (beat 7 vs
  9) and the orbit provably stays inside it on every seed (widest ring
  181.5 < 210). Beacon 4's trigger inner edge (230u) clears the ring
  (210u) - the already-inside trap is pinned, not eyeballed.
- **Range audit**: beat 4's lesson is taught from the cluster at ~382u <
  600u default range; the waypoint leg ~802u < 900u authored range, both
  pinned with margins. The derelict's short range (~75u) is deliberate -
  the marker walks the player in; text says the hulk "drifts by your old
  salvage field", matching the (300,-40,40) placement 215u from the
  cluster.
- **Soft-lock sweep**: every beat has a reachable completion that cannot
  be consumed early (lock beats tolerate pre-acquired locks via the
  bridge echo - pinned as the stale-echo no-op; spatial beats arm before
  the player can be inside their trigger). The run remains completable
  ignoring every optional gesture except the locks the beats gate on -
  and those are the lessons.
- **every_gameplay_handler_is_beat_gated** still passes over the 12-beat
  script; the death handler stays beat-free.
- R1.1 (NOTE, playtest): the coast drift duration varies ~13-29 s by
  seed and park side; if it reads as dead air, shrink the park-to-ring
  gap (ring radius and beacon-4 distance are the knobs, pins enforce the
  invariants).
- R1.2 (NOTE, playtest): the return leg from the planetoid to the
  derelict (~1200u) is manual flight by design (nothing lockable at that
  range); the marker carries wayfinding. Watch for "are we there yet".

VERDICT: APPROVE (round 1).
