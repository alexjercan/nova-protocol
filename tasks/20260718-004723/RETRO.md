# Retro: Web render-scale / resolution lever

- TASK: 20260718-004723
- BRANCH: render-scale-lever (squash-landed as master f9e44f99)
- REVIEW ROUNDS: 1 (APPROVE)

See `render-scale-report.md` for the numbers/decision and
`docs/2026-07-18-render-scale-lever.md` for the design log; this is process only.

## What went well

- **Mapped the camera/UI render architecture before touching the render graph**
  (one Explore pass: single window camera, no `UiTargetCamera`, no `RenderLayers`
  anywhere, the `IsDefaultUiCamera` rule). That map made the whole-frame-into-one-image
  design obviously correct rather than a guess, and the `IsDefaultUiCamera` +
  layer-1-blit isolation followed directly from it.
- **Built an isolation knob for the measurement** (`NOVA_PERF_RENDER_SCALE`) so
  the SAME Low tier could be measured at 1.0 vs a fraction. Without it, Low
  bundled render-scale with the particle/scatter cuts and the lever's own
  contribution was unattributable - the isolation is what turned a plausible
  story into an actual measurement (and it showed ~0%).
- **A correctness screenshot, not just frame times.** Example 21 captured the
  real upscaled frame; a frame-time-only harness cannot tell "faster because
  fewer pixels" from "faster because the screen is black." Cheap, decisive.
- **Honest measure-first reporting.** The gate came back inconvenient (0.7 is
  ~neutral on the only web rig); rather than ship it under the "helps weak HW"
  plausible story the task explicitly warns against, surfaced the fork to the
  user (who chose keep 0.7) and documented the caveat straight in the report,
  CHANGELOG, and code comment.
- **Independent out-of-context review** (three finder angles + manual
  re-derivation) converged on the design being correct - every candidate bug
  (high-DPI sprite size, VRAM leak, projection misalignment) refuted on analysis,
  which is exactly what the review's shared-session blind-spot guard is for.

## What went wrong

- **Enormous wall-clock lost to a contended shared measurement host** (load
  spiked past 50 from parallel agent jobs). The first software-raster sweep was
  contaminated and had to be redone; software raster is pure CPU so contention
  destroyed it, and even the web numbers carried contention tails. Root cause:
  started measuring without checking host load or picking the least
  noise-sensitive rig first.
- **Self-inflicted shell failures.** `cd ... && export ... && Xvfb ... & rest`
  backgrounded the whole cd-chain into a subshell (the `A && B & C` split), so
  the foreground loop ran in the wrong directory with unset vars; and a stray
  `kill/pkill` at the head of a background command disrupted the job wrapper
  (exit 144). Root cause: careless mixing of `&` backgrounding with `&&` chains.
- **Screenshot example crashed first try.** `BCS_SHOT` force-advances to Playing
  before assets load and is mutually exclusive with `BCS_AUTOPILOT`, so
  `nova_screenshot` + `assert_scenario_loaded` do not compose for a
  scenario-loading example. Root cause: wired the harness before reading its
  state-forcing/mutual-exclusion contract. Fixed by dropping the assert and using
  a generous settle.
- **Merge integration with a sibling.** The parallel scatter task
  (20260718-004834) removed `scatter_density` while this branch added
  `render_scale` right next to it - a semantic conflict (drop scatter, keep
  render-scale, fix the tests + docs). Resolved cleanly as merge-integration, but
  it also left a stale player-facing CHANGELOG claim that neither task caught in
  its own diff (filed as a follow-up).

## What to improve next time

- Before a measurement task, check host load / serialize against parallel jobs,
  and reach for the least-CPU-contention-sensitive rig (GPU/web) first - a
  measurement is only as good as a quiet host.
- Read a harness plugin's lifecycle contract (does it force a state? is it
  exclusive with another arm?) before composing it into a new example.
- Never mix `&` backgrounding with `&&` chains on one line; put a backgrounded
  process (Xvfb) on its own statement and keep `kill/pkill` out of commands that
  also launch new background work.

## Action items

- [x] tatr 20260718-130911: fix the stale scatter-thinning CHANGELOG claims
  (pre-existing from the sibling scatter-removal task; found while landing).
- [x] Shipped the `render_scale` perf override + example so the lever can be
  re-measured on a genuinely fill-bound (weak-GPU) rig when one exists.
