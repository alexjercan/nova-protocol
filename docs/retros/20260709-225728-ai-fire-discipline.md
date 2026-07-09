# Retro: AI fire discipline

- TASK: 20260709-225728
- BRANCH: feature/ai-fire-discipline (squash-merged, see git log)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

Fifth task of the AI combat arc.

## What went well

- **The aim-point gate was found by reasoning about the pipeline, not by
  playtest.** Feeding lead velocity makes turrets steer to the intercept;
  a fire gate still aligned to the raw anchor would then hold fire
  forever against crossing targets. Spotting that interaction at plan
  time (it is in the Steps) avoided shipping a "leads but never shoots"
  AI.
- **Caught my own non-discriminating test before review.** The first
  discriminator geometry (30 m off-axis at 100 m) was still inside the
  0.95 alignment cone - the test passed against the OLD code too.
  Recomputing the cosine by hand (0.958 vs threshold 0.95) exposed it;
  moved to 22 degrees. This is the fire-discipline version of the
  thrust-balancing retro's "arithmetic before assertions".
- **Free-running cadence bought volley staggering for free** - per-ship
  phase drift means multiple AI ships do not alpha-strike in sync. Zero
  extra code; documented on the system.

## What went wrong

- The non-discriminating test geometry (above) - root cause: picked
  "looks off-axis" numbers without computing the cosine against the
  threshold.

## What to improve next time

- For any threshold-cone test, compute the boundary value by hand and
  place the fixture explicitly OUTSIDE it; state the cosine in the test
  comment (done here).

## Action items

- None. Friendly-fire hold stays deferred on its recorded trigger
  (formation fights after 225729).
