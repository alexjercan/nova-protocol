# NOTES

## Decisions

- Replace recurring lock events with four one-shot edges:
  `OnTravelLockStart`, `OnTravelLockEnd`, `OnCombatLockStart`, and
  `OnCombatLockEnd`.
- A direct target switch queues end for the old target, then start for the new
  target.
- Held locks stay quiet. AI locks remain gameplay-internal.
- Remove `lock_refire_secs` and `LockRefireSecs` as part of the clean format
  break.
- Do not change `OnDestroyed` or `OnNeutralized` in this task. Their lifecycle
  relationship needs separate design work.

## Coverage

- Scenario integration test covers all four edges, held-lock silence, target
  switch ordering, id-less targets, and AI exclusion.
- Shakedown and Final Tally tests cover migrated acquisition handlers.
- `player_path` performs the real radar gesture twice across scenario reloads;
  both `OnTravelLockStart` handlers set `leg` from 0 to 1. Probe run
  `35aaef74/player_path` passed 7/7 measured checks.

## Follow-up

Task `20260812-132400` owns the separate destruction, despawn, and
neutralization lifecycle design. This task does not change those events.

## Payload

All four events carry the locked target as `id` and the locking player ship as
`other_id` / `other_type_name`.
