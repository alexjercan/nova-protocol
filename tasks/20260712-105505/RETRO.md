# Retro: Bullets affected by gravity wells

- TASK: 20260712-105505
- BRANCH: bullets-gravity (landed as squash 14dbeec)
- REVIEW ROUNDS: 1 (APPROVE, 3 NITs, all addressed)

Process notes only; what/why/evidence live in TASK.md, the direction in the
spike (tasks/20260712-112113/SPIKE.md).

## What went well

- **Spike-first on a decision-reversal paid off.** The ask reversed an explicit
  prior call (gravity-wells spike decision 5: "rounds skip, imperceptible
  curvature"). Spiking before planning forced a measurement (impulse approx)
  that mostly vindicated the old call and reframed the scope - and left room for
  the user's playtest insight (the turret already aims behind a *falling*
  target, so bullet gravity is a free common-mode correction, not a new cost) to
  land before any code was written. Cheap course-correction at the cheapest
  moment.
- **Measure-before-optimize caught the real cost.** I assumed the
  O(wells x affected) scan was the hot path. An A/B bench (N affected + well vs
  N plain bodies, so avian's integration cost cancels and the delta is the
  gravity system) isolated it at ~1.5us/entity and showed the cost was the two
  per-entity `Vec` allocations, not the arithmetic. Reusing `Local<Vec>` scratch
  buffers dropped it ~30x and sped up ships/torpedoes too. The optimization
  would have been misdirected without the isolation bench.
- **The curve regression carries its own A/B.** Its gravity-free control body,
  in the same run, is the "without the marker" case - so it satisfies
  `would-it-fail-without-it` with no sabotage recompile, and the guarantee lives
  in the committed test permanently.
- **Same-session review still did its job.** Per the review skill's re-derive
  rule I independently re-verified two load-bearing claims (buffer reuse is
  behaviour-equivalent because both buffers are cleared before use;
  `DominantWell` churns only on owner-change) rather than trusting my own
  summary. Clean single round.

## What went wrong

- **I wrote guessed numbers into comments before measuring, twice.** The curve
  comment said "~6.6u" (actual 3.25u) and the buffer-reuse comment said
  "~1.5 ms/tick" (actual ~0.1). Both had to be corrected after running.
  Root cause: I wrote the quantity from a mental model at the moment I wrote the
  prose, instead of leaving a placeholder and backfilling from a run. Harmless
  here because I did measure and correct, but a number written from a model that
  never gets re-measured is exactly how folklore bounds ship (see
  `authored-vs-derived-values`).

## What to improve next time

- Never write a specific number (deflection, ms/tick, thresholds) into a
  comment, doc or task until it has been read off an actual run. Leave a
  `TODO(measure)` placeholder and backfill, so a guessed value can never
  survive to commit.
- When judging a hot loop at these scales, suspect per-iteration allocation
  before O(n) arithmetic; an isolation A/B bench answers it in minutes.

## Action items

- [x] Lessons ledger updated: new `measure-before-writing-the-number` (x1);
  positive-pattern `ab-isolation-bench` (x1).
- No follow-up code task filed now. The C2 follow-up (gravity-aware turret
  intercept term modelling both target and bullet acceleration) is deliberately
  deferred until a playtest shows the free common-mode cancellation is not
  enough - recorded in TASK.md and the spike, to be filed then.
