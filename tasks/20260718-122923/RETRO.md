# Retro: RCS HUD indication (active palette)

- TASK: 20260718-122923
- BRANCH: feat/rcs-hud (landed as master 27568615)
- REVIEW ROUNDS: 1 (APPROVE; 2 nits, one addressed by a rename)

Process only; what/why live in TASK.md + NOTES.md.

## What went well

- Split scope honestly: the task title was "palette + cap ring", but the ring is
  new visual geometry whose "does it read right" can't be judged headless, and
  the sphere is fixed-radius so the ring's semantics are underspecified. Shipped
  the testable palette and seeded the ring as a playtest-gated follow-up
  (20260718-144939) instead of shipping blind geometry. The Goal itself marked
  the ring "Optionally render", so this delivered the required scope. This is
  `render-output-eyeball` applied BEFORE writing the code, not after a bad
  screenshot.
- Reused the proven `sync_engaged_palette` pattern almost verbatim; the diff is a
  color const + a one-arg extension + one query. The pre-existing autopilot
  palette test guarded the change for free.
- Keyed the palette on `RcsActive` (not `RcsIntent`) so the autopilot driving
  `RcsIntent` later reads as ENGAGED - a small decision that pre-solves a
  cross-task interaction, pinned by a test assertion.

## What went wrong

- Nothing significant. The one review nit (function name `sync_engaged_palette`
  now undersells what it does) was a real staleness I could have caught while
  editing - the function's job changed but I left its name. Cheap to fix
  (rename), but the lesson is to re-read a function's NAME when its behavior
  widens.
- Fresh-worktree first build was ~5 min (full crate recompile); unavoidable, but
  worth remembering these HUD/flight iterations are compile-bound.

## What to improve next time

- When a change widens a function's responsibility, re-check its name in the same
  edit (the review caught it; I should have).

## Action items

- [x] Ledger: bumped `render-output-eyeball` (x4) with the positive
  split-the-unverifiable-visual application.
- [ ] Follow-up task 20260718-144939 (cap ring) is seeded and OPEN - pick it up
  only with a playtest available.
- No other follow-ups; the RCS family's last piece (autopilot integration
  20260718-122932) is next in this flow.
