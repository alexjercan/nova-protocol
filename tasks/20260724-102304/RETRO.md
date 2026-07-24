# Retro: Drawer shell + interaction model + objectives section

- TASK: 20260724-102304
- BRANCH: feat/tab-drawer-shell
- REVIEW ROUNDS: 2 (round 1 out-of-context REQUEST_CHANGES, round 2 APPROVE)

(What/why and the evidence rig are in TASK.md's close-out; the guard audit and
design decisions in NOTES.md; this file is process only.)

## What went well

- The plan front-loaded the state-route audit as step 1 and sized it (19 `==
  Paused` guards). That turned the risky part - a new frozen state leaking input
  through observers that bypass set-gating - into a mechanical, single-helper
  (`is_frozen()`) widen. Reasoning about behavior-preservation ("`is_frozen()`
  equals `== Paused` for every pre-existing state") also let the shared-observer
  change land without re-running the whole suite, honestly.
- Verifying the dependency before designing around it paid off: reading the bcs
  `Tween` source showed it advances on `Res<Time>` (= `Time<Virtual>`), which the
  drawer pauses - so the plan's "slide via TweenPlugin" note would have produced a
  drawer that freezes mid-slide. Caught before writing it, switched to `Time<Real>`.
- The work-verify step caught a real integration bug by READING the consumer
  rather than assuming: `apply_hud_visibility` force-hides Chrome-tier nodes, which
  would have blanked the drawer when the HUD was minimized. Fixed by giving the
  panel/backdrop no `HudTier`.
- The out-of-context reviewer earned its keep: it found the audio-loop freeze gap
  (R1.1) that the implementing session's own audit missed.

## What went wrong

- R1.1 (thruster/RCS loops keep roaring behind the open drawer). Root cause: the
  audit swept `== PauseStates::Paused` runtime GUARDS but not `OnEnter/OnExit(Paused)`
  system REGISTRATIONS. `audio.rs` freezes its loops by schedule, not by a guard, so
  it was invisible to a guard-only grep - even though it is the same "while-frozen"
  class the audit existed to cover. The `audit-state-gates-on-new-entry-path` lesson
  says to grep `OnEnter/OnExit` too; the implementation read that as "in the crate I'm
  touching" and only swept nova_menu's freeze wiring, not the whole workspace.
- Two test-rig false starts (manual `ButtonInput` needing an explicit `clear()`;
  the flyable-ship rig not carrying `FlightIntent`) cost a build cycle each. Root
  cause: wrote the test rigs from the observer signatures rather than copying a
  known-good sibling rig verbatim first.

## What to improve next time

- When adding a variant to a shared state enum, the audit grep is BOTH
  `== <state>`/`is_frozen`-style guards AND `OnEnter/OnExit(<state>)`/`DespawnOnExit`
  registrations, across the WHOLE workspace, not just the crate under edit. A
  frozen-axis behavior can be wired by schedule.
- Verify a dependency's clock/ordering/observer semantics at PLAN time whenever the
  design leans on it (the bcs Tween clock), not mid-implementation - it was cheap
  here but a virtual-clocked assumption baked deeper could force a redesign.
- Scaffold a new test rig by copying the nearest passing sibling rig verbatim, then
  mutate - do not reconstruct it from the system signature.

## Action items

- [x] R1.1 fixed on this branch (audio loops freeze on the Drawer variant too).
- [x] Lessons ledger updated (see below); no follow-up code task - the drawer's
  remaining sections (comms log 102309, minimap 102320, ship status 102332) are
  already queued.
