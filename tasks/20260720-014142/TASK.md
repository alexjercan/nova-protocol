# Loop reload gate closes too early: ScenarioLoaded fires before OnStart seeds variables - playable panics on first real loop; close the gate on script-observed readiness

- STATUS: CLOSED
- PRIORITY: 63
- TAGS: v0.8.0,bug,tooling


## Close-out (2026-07-20, branch fix/loop-gate-readiness; bcs v0.19.5)

Field crash (user's first real-GPU playable loop) unpacked into THREE
layered defects, each fixed at its honest site:

1. **Gate closed too early** (the crash): ScenarioLoaded fires before
   OnStart seeds variables; playable's variable-reading stages hit the
   None window. FIX: the reload gate closes on the SCRIPT'S OWN readiness
   signal (its seeded variable exists) - ScenarioLoaded observers
   removed; the seed wait honestly counts as reload cost.
2. **Looped cycles cannot meet first-cycle deadlines**: cycle N starts
   late by the reload duration, so completion backstops fired on healthy
   scripts. FIX: `looped` flag - completion enforcement binds on cycle 1
   and every clean-pass run; looped cycles feed the capture activity.
3. **Dead-cycle tail** (bcs v0.19.5): the looping autopilot only checked
   others_pending at cycle END, wasting up to a full cycle after the
   capture completed and straddling the deadline into a false laggard.
   FIX: in the looping regime it reports done mid-cycle the moment
   collectors finish.

E2E (forced loop, NOVA_PERF_FRAMES=2000): playable exit 0, 2 loops, full
2000-frame capture, 2 reload lines, log clean, and the mid-cycle finish
on the record ("collectors done after 2 loop(s)... t=1.0s"). scenario
default-window: 3 loops, 900 frames, exit 0. The review's R1.1 (playable's
loop path unexercised on fast hosts) was the exact gap the field found -
forced-loop e2es now exist for it.

Also learned in-cycle: an oversized test window (5000 frames at Xvfb
rates) legitimately exceeds the 120s completion deadline - deadline
verdicts in fps e2es need window-vs-rate arithmetic first.
