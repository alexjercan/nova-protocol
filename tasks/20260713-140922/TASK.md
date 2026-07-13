# OnLock scenario event: bridge the lock components into the event vocabulary

- STATUS: CLOSED
- PRIORITY: 44
- TAGS: v0.5.0,scenario,events,spike

## Outcome (CLOSED 2026-07-13)

Shipped per plan with ONE deliberate deviation: the bridge is NOT
once-per-acquisition - acquisition fires immediately AND a held lock
echoes every LOCK_REFIRE_SECS (5 s), following the orbit tracker's R1.1
rationale (a one-shot consumed under a rejecting beat guard soft-locks
any scenario whose beat advances while the lock is already held;
beat-gated handlers make repeats no-ops). Implemented as a pure
`tick_lock_slot` (unit-tested: acquire/echo/retarget/clear-re-arm) driven
by `track_player_locks` with a `LockEcho` component - poll-and-compare
instead of Changed filters, so the dedup is explicit state. Player-scoped
(the AI mirror writes CombatLock on AI ships; pinned by the e2e test:
an AI lock on the same target never fires). Id-less targets fire nothing
(the echo retries, orbit R1.2 parity). E2e through the real pipeline:
travel and combat handlers tick their variables, filters compose like
OnEnter. 37 nova_scenario tests green; fmt + workspace check clean.

## Goal

The beat-sheet-v2 spike needs a "player locked X" completion signal so the
split radar lessons TICK the instant a lock lands (three consumers: the
first-lock lesson, the waypoint re-designation leg, the combat-lock
rehearsal). Two event variants - `OnTravelLock` and `OnCombatLock` - so the
existing `EntityFilterConfig` (string-field matching, filters.rs:30) works
unchanged; one loader bridge fires both.

## Steps

- [x] nova_events: add `OnTravelLockEvent`/`OnTravelLockEventInfo` and
      `OnCombatLockEvent`/`OnCombatLockEventInfo`, both `{ id, other_id,
      other_type_name }` - the exact `OnOrbitEventInfo` shape
      (nova_events/src/lib.rs:94); export via the prelude.
- [x] nova_scenario events.rs: `EventConfig::OnTravelLock` / `OnCombatLock`
      variants mapping to `EventHandler::new::<...>()` (events.rs:27 match).
- [x] loader.rs: `track_player_locks` bridge - query the PLAYER ship's
      `TravelLock` + `CombatLock` with `Changed<...>` filters, scoped
      `With<PlayerSpaceshipMarker>` (VERIFY-FIRST constraint: the AI combat
      mirror writes CombatLock on AI ships every engagement change,
      input/ai.rs - an unscoped bridge would fire for AI). On a slot's
      value changing to `Some(target)`: resolve the target's `EntityId`
      (the `q_ids` pattern, loader.rs:222; no id = no fire), fire the
      slot's event with id = target id, other_id/other_type_name = the
      player ship's. Once-per-acquisition falls out of change detection:
      the slot writers are equality-skipped (targeting.rs radar search),
      so a held live-lock on the same target does not re-fire.
- [x] Register beside `track_orbit_holds` with `run_if(scenario_is_live)`
      (loader.rs:146 production, :631 test wiring).
- [x] Tests (loader.rs, the orbit-hold test shapes): acquisition fires the
      right variant once with the right ids; a live-radar retarget onto a
      SECOND id fires for it; re-designating the SAME target is quiet
      (delivery-guarded by the first fire); an AI ship's CombatLock write
      never fires; a lock on a body without an EntityId is quiet.
- [x] fmt + check; nova_scenario suite.

## Notes

- Spike: docs/spikes/20260713-140742-shakedown-beat-sheet-v2.md (option C;
  option D's wider vocabulary deliberately rejected).
- Two variants (not one event + bool field) because EntityFilterConfig
  matches only the string fields in the info data map - a bool field would
  need a filter extension for zero gain.
- nova_editor untouched (verified: it does not enumerate event variants).
- 20260713-140929 depends on this task.
