# Improve autopilot turn timing and show predicted curved trajectory

- STATUS: CLOSED
- PRIORITY: 45
- TAGS: v0.15.0, autopilot, navigation

## User facts

- Autopilot sometimes turns too fast or too slowly for the ship's actual turning time. Investigate and improve how it accounts for turn rate and the time needed to orient before a maneuver.
- Replace the straight-line guidance preview with a curved line that represents the actual predicted trajectory, not just a decorative curve.
- Gravity work in `tasks/20260925-182711/TASK.md` may change both control and trajectory prediction. An initial focused pass before gravity is acceptable, followed by a focused gravity-aware pass after that task; do not assume gravity is already settled.
- At task creation, this task recorded future work only. Implementation was authorized later under the staged design below.

## Agent findings (2026-10-03; baseline evidence)

- Static-well gravity is merged on the Sprout base (`3edd050d8`). GOTO and STOP account for the positive along-track well pull in the braking budget, but the turn lead is estimated separately; ORBIT uses a stable ring and live velocity correction. This does not establish a turn-timing defect.
- The world-space trajectory ribbon reads `ManeuverTelemetry` and draws ship-to-flip-to-park straight segments. A spline through these points would be a decorative curve, not an actual predicted flight path. The Map GOTO route in PR #106 is separate but is now present in this stacked Sprout base.
- Pre-change `system_flight_legs` probe passed the named chain/coast/handover assertions for skiff and carrier. Artifacts: `/tmp/nova-autopilot-flight-legs-prechange/3edd050d8/system_flight_legs/`. This probe covers clean-space GOTO and a gravity-well ORBIT handoff, not a complete under-gravity turn-timing or prediction-error measurement. The carrier's recorded 661 m radius on a planned 530 m ring is a handoff sample, not yet evidence of steady-state drift.
- A short-lived custom target directory created by the first probe attempt was removed.
- The approved off-axis GOTO proof in `crates/nova_ship/src/flight/tests/goto.rs` matches a flight with a strong off-axis well against one without it at one fixed physics tick per update. Both arrived near 50 u standoff at rest. At z=-500, lateral positions were -0.85 u with the well and -3.44 u without it; the neutral leg is not straight. Raw evidence: `/tmp/nova-autopilot-offaxis-before/one-fixed-step-output.log`. The permanent matched-arrival assertions remain uncommitted; temporary diagnostic prints were removed.
- Diagnostic capture at `/tmp/nova-autopilot-offaxis-before/flip-instrument.log` observed residual thruster input while the ship turns through its brake flip. The well flight briefly re-entered the braking *planning flag* without a second 180-degree hull turn. `turn-lead.log` measured hull alignment across high and shipped-scale low turn authority: the brake cone was reached before the approximate lead budget in all four cases; the latest rising aligned throttle differed by about two fixed ticks. No turn-timing or arrival failure was reproduced, so a controller retune is not justified. A gravity-only ballistic curve would miss the neutral drift and may bend in the wrong direction.

## Approved decisions

- Prioritize reliable arrival without oscillation over elapsed time or propulsion use. Do not include collision avoidance or route planning.
- Stack this work on HUD PR #106 so both its UI Map GOTO line and the world-space HUD can consume the same curved prediction. `autopilot-trajectory` starts at HUD head `e802a45e2` after master was merged into `hud-clarity` without rewriting PR history.
- The user approved implementation of the staged shared-step design: capture before/after live-flight traces before adding a bounded dominant-well predictor; then feed the same sampled path to the world HUD and Map. Keep the existing flight policy unchanged unless reproduced failure justifies a separate change.
- Reproduce and measure off-axis GOTO behavior before changing controller or HUD. Use real fixed-step positions and compare well and no-well legs; this diagnostic proof is approved.

## Remaining constraints

- The staged, shared controller-step and bounded prediction design is approved. Reject decorative splines and do not claim predicted accuracy without a matched observed flight path. The measured brake-flip lateral drift does not justify a controller retune in this change.
- Stage 1 captured six before traces across 4,553 fixed ticks, repeated with identical output. The extracted live controller and snapshot helpers reproduced all six trace hashes exactly, including the zero-mass hold path; the later shared-helper extraction reproduced them again. Artifacts: `/tmp/nova-autopilot-extraction-baseline/`, `/tmp/nova-autopilot-predict-proof-report.md`.
- The bounded `FlightPrediction` simulates the same controller, RCS, drive spool, and dominant-well acceleration as flight. It samples at eight fixed ticks over at most 30 seconds; it predicts GOTO and STOP, does not extend the path to the goal beyond its horizon, and does not predict the subsequent ORBIT leg. Moving targets/wells, docked ships, and released autopilot have no prediction.
- Five GOTO/STOP cases with and without an off-axis well and at two turn authorities compared every interpolated forecast tick with live COM. The largest error was 0.043 engine units (one-unit asserted limit); each sampled point matched exactly. A GOTO at a static well additionally predicted park within one unit of the live ORBIT handoff; it removed the prediction on handoff. Evidence: `crates/nova_ship/src/flight/tests/prediction.rs`, `/tmp/nova-autopilot-predict-final.md`.
- In one quiet-host matched five-repeat dev-profile set, the predictor added 325 microseconds mean per fixed update to an off-axis-well GOTO (same 2,257 flown ticks with/without); with-predictor whole-update p95 was 2.07 ms versus 0.35 ms without. Two later quiet-host matched five-repeat sets timed each predictor slice directly on the optimized test profile, with temporary instrumentation since removed. At 240 ticks per slice, p95 was 305.5/262.2 microseconds and max 1,600.7/405.2 microseconds; at 120, p95 was 147.5/134.8 and max 680.6/223.4. The set-1 240-tick max exceeded the approved 1 ms per-slice bound, so the slice is now 120 ticks. A 30 s horizon run therefore publishes 16 fixed updates or paused frames after its seed instead of eight. This is one host and not a frame-budget guarantee: a catch-up frame can stack several slices. Evidence: `/tmp/nova-autopilot-slice-cost/REPORT.md`, `/tmp/nova-autopilot-predict-proof-report.md`, `crates/nova_ship/src/flight/prediction.rs:35-42,489`.
- World HUD and Map rendering consume sampled predictions; the Map destination marker remains independent and the FLIP label uses the same point/time as the predicted gate. The final off-grid sample carries its actual tick time, with focused tests in the predictor and both UI consumers.
- TAB pauses virtual time, starving the fixed-step predictor after a Map GOTO. `PostUpdate` now runs the same bounded predictor only while virtual time is paused. A named ECS test verifies publication and continuous visibility without advancing live pose, velocity or fixed elapsed time; an unchanged same-order forecast remains displayed while a new run computes. The test first reproduced a one-frame-in-eight flicker, then passed after the path-lifetime fix.
- Final affected checks after the 120-tick slice passed 91 flight, 18 gravity (one expected ignored performance test), 24 Map and 11 HUD tests; `cargo check -p nova_core`, crate formatting and diff-check also passed. The paused-Map test now publishes after 16 paused frames, checks the live ship against the paused forecast at exactly eight unpaused ticks, and checks republishing after the same number of unpaused ticks. `mdbook build` and web `npm run ci` passed. A temporary NVIDIA/Xvfb fixture captured a real paused Map GOTO under a dominant well after 18 frames: `/tmp/nova-ap-visual-goto/tmp-map-route.png` shows a gently curved path, ending short of its live target; `/tmp/nova-ap-visual-goto/feature-autopilot.png` shows the world ribbon at burn. The ring's beacon needed a temporary Map contact marker and its example needed the normal menu plugin to pause; neither workaround was shipped. The temporary fixture was removed, leaving only a corrected obsolete example comment. A static independent review found no production correctness blockers. It noted duplicate private path-decimation code and repeated work on persistent non-finite forecasts. No controller retune was justified by reproduced flight behavior. Workspace-wide tests, Clippy and a release-profile performance claim remain outside this task's proof.

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
