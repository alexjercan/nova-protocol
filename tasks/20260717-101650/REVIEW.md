# Review: Thruster loop as a section sound (per-handle loops)

- TASK: 20260717-101650
- BRANCH: task-20260717-101650-thruster-loop-sound

Reviewed the committed diff (d7da33d7 + header-doc fix) fresh + independent
out-of-context pass. Verified (mine + reviewer's independently):

- Behavior preservation: engine_volume/distance_attenuation/smoothing rate
  unchanged; per-ship loudest-wins now PER HANDLE (identical when all content
  authors one sound, which base does); player exemption intact; the
  menu-ambience backdrop hums (its ship uses basic_thruster_section, which
  authors self://sounds/thruster_loop.wav in regenerated content).
- All hum paths converge on ONE handle: catalog self:// ref, the torpedo's
  direct runtime path, and load-order - base/sounds/thruster_loop.wav; the
  file exists.
- Pause/resume iterate ALL loop entities (With<ThrusterLoopSfx> unaffected by
  the tuple field); WorldSfx::ThrusterLoop fully deleted (bank -> 1 key,
  SalvagePickup); no webmod/example/editor thruster lacks a hum (all use the
  authored base prototype).
- Suites: nova_gameplay lib 547 (28 audio incl. NEW per-handle-independence +
  unauthored-silent), gates 4/4, workspace all-targets clean.

Noted, accepted (not a finding): the loop entity now spawns on the first
burning frame instead of at startup - 1-2 frames of latency, mitigated by the
volume-0 spawn + smoothing that pre-advances while the sink is absent.

## Round 1

- VERDICT: APPROVE

No findings. The stale single-loop module-header prose was caught by the
implementer's own prose-grep (the 101633 retro lesson applied) and fixed
before review.
