# Retro: weapon auto-reload/regen mechanic

- TASK: 20260717-085640
- BRANCH: feature/ammo-reload (landed a26a6189)
- REVIEW ROUNDS: 1 (APPROVE, one NIT addressed)

## What went well

- **One-timer/two-behaviors design paid for itself.** Modeling discrete
  reload-on-empty and continuous regen as one `SectionReload`
  (`only_when_empty` + `rounds_per_cycle`) rather than two mechanisms kept the
  surface tiny and, more importantly, made the readout follow-up
  (20260716-123556) purely additive: it reads one `progress()` value and needs
  no new state. Designing the seam the sibling task consumes, up front, is what
  the spike asked for and it held.
- **Factoring `advance()` out of the system made the logic directly testable.**
  The pure method carries every branch (waits-for-empty, clamp, multi-cycle
  catch-up, rest); the system test only had to prove the wiring. Most of the
  coverage never touches an App or a clock, so it is fast and can't be fooled by
  scheduler quirks.
- **A/B integration tests reused the existing fire rigs.** Each new
  recover-after-dry test is the mirror of an existing "caps at magazine, forever"
  test, so deleting the reload wiring makes it fail - the coverage is honest.

## What went wrong

- **The through-the-schedule test failed twice on a clock quirk I already had
  the answer to.** `Time<Virtual>` clamps per-update delta to `max_delta`
  (0.25s), so my 1s `ManualDuration` step advanced only 0.25s and the magazine
  never accumulated a full cycle. Root cause: I theorized about first-frame
  zero-deltas for two iterations instead of reading the actual `left/right`
  assert values (0 vs 6) first. The repo's own fire-rate rigs carry a comment
  about exactly this clamp, and the ledger already had
  `manual-time-rig-measures-its-clock` - I did not check it before writing the
  rig.

## What to improve next time

- When a `ManualDuration` headless rig under-advances, make the `Time<Virtual>`
  `max_delta` clamp the FIRST hypothesis and read the measured values before
  theorizing. Grep the ledger for the relevant slug before writing a
  time-driven test rig, not after it fails.

## Action items

- [x] Bumped `manual-time-rig-measures-its-clock` to x2 in docs/LESSONS.md,
  sharpened with the `max_delta` mechanism and the raise-max_delta fix.
- [ ] Next in this flow: 20260716-123556 (reload-state on the diegetic readout),
  now unblocked - reads `SectionReload::progress()`.
