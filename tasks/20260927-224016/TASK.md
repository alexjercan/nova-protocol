# Improve autopilot turn timing and show predicted curved trajectory

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.15.0,autopilot,navigation

## User facts

- Autopilot sometimes turns too fast or too slowly for the ship's actual turning time. Investigate and improve how it accounts for turn rate and the time needed to orient before a maneuver.
- Replace the straight-line guidance preview with a curved line that represents the actual predicted trajectory, not just a decorative curve.
- Gravity work in `tasks/20260925-182711/TASK.md` may change both control and trajectory prediction. An initial focused pass before gravity is acceptable, followed by a focused gravity-aware pass after that task; do not assume gravity is already settled.
- This task records future work only. Do not implement or change autopilot behavior as part of task creation.

## Delivery to plan when work starts

- Reproduce cases where autopilot over- or under-turns; map turn authority, ship inertia, guidance, control ordering, and the existing straight-line display before choosing a controller change.
- Define what the trajectory predicts, its time horizon, update conditions, error bounds, and how it differs from a target/path guide. The visual must follow the same motion assumptions as the pilot, including current velocity, turning, thrust and braking. Decide the gravity dependency explicitly before implementation.
- Split delivery if useful: improve turn timing and prediction under the current physics first; after the static-well gravity model is approved and implemented, revisit prediction and guidance for gravity and orbit cases. Avoid presenting a pre-gravity prediction as gravity-aware.
- Review interfaces, ownership, defaults, failure policy, and a code-backed proof plan with the owner before code changes.

## Verification when implemented

- Compare reproduced before/after turning behavior for different hull turn rates and approach speeds with state/trajectory assertions; inspect the rendered prediction against the actual flight path in matched deterministic runs.
- Include autopilot deceleration and ORBIT, and check prediction error under gravity in the post-gravity pass. Report mismatches instead of claiming an exact path without measured agreement.

## Done when

- The chosen turn-rate behavior and meaning of the trajectory line are approved, implemented, and verified in real flight; any deferred gravity-aware follow-up remains explicit and separately tracked.
