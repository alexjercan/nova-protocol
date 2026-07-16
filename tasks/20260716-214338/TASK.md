# Bug: objective text missing after restarting a level (UI-only; objective still achievable)

- STATUS: OPEN
- PRIORITY: 52
- TAGS: v0.7.0,bug,hud,ui,objective


## Symptom

Restarting a level (Pause menu Retry, or the Victory/Defeat outcome frame's
Retry - both added recently, task 20260716-125856) drops the on-screen
OBJECTIVE TEXT: the objective panel/marker no longer shows. The objective is
still ACHIEVABLE (completing it works and the scenario progresses), so the
state machine is fine - it is a UI/HUD-only regression on the restart path.

Reported by user 2026-07-16 during playtest.

## Direction (do not implement yet)

- Reproduce: start a scenario with an `Objective` action posted at OnStart,
  restart via Pause > Retry AND via the outcome frame Retry, and confirm the
  objective HUD text is absent on the second run while the objective still
  completes.
- The `Objective` action posts its text once (typically OnStart). On restart,
  the scenario re-runs but the HUD objective widget likely is not re-populated
  or was despawned and not rebuilt - a lifecycle mismatch between the restart
  reset and the objective-display system.
- Likely suspects: the objective HUD state/resource is not reset or re-read on
  restart; or the OnStart objective re-post fires but the display observer/
  system that mirrors it into the HUD only runs on the FIRST scenario load
  (registered-once system, stale change-detection cursor, or an OnEnter that
  restart does not re-trigger). Compare the fresh-load path vs the restart
  path for the objective widget spawn.
- Follow `diagnostic-first` and `registered-system-for-change-detection`:
  trace the exact restart sequence with the objective resource + HUD entity,
  do not theorize the mechanism first.
- Fix so a restarted scenario shows the same objective text a fresh load does;
  pin with a rig that restarts and asserts the objective HUD is populated
  (behavior, not component-presence).

## Notes

- Related surfaces: pause-menu Retry and the scenario outcome frame
  (20260716-125856), the objective HUD/markers (objective_markers).
