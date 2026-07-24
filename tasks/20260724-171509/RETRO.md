# RETRO: status bar gets its own HudTier::Status (task 20260724-171509)

- OUTCOME: CLOSED, review APPROVE round 1, probe playable OK.
- BRANCH: feat/status-tier (one squash commit).

## What changed and why

The top-right status bar (fps + version + the objective count) was
`HudTier::Chrome`, so it vanished at `HudVisibility::Minimal` and on drawer-open.
The owner wants it treated like an FPS overlay - persistent reference chrome that
rides the whole session and only clears for a cinematic screenshot. Added a third
`HudTier::Status`: shown at `All` + `Minimal`, hidden only at `None`; the status
bar is retagged `Chrome -> Status` and given `HudDrawerExempt` +
`GlobalZIndex::default()` so it persists through the drawer and rides the existing
z-lift. The objective count (its child) inherits all of this.

## Decision (this task practiced the mandatory-DECISION.md rule)

The owner picked "new tier" over "own on/off setting" and "drawer-exempt only" at
a gate; DECISION.md records the fork, the cinematic-None constraint, and why
`Status` reuses the `HudDrawerExempt` marker for drawer-persistence (a
value-filtered lift query would risk mis-touching the drawer panels' z) rather
than folding exemption into the tier. This is the first task in the session to
write the DECISION.md BEFORE building, per the flow/plan skill edits made earlier
today - the process lesson applied to itself.

## Difficulties / notes

- The interesting design question was WHAT clears the status bar, not whether to
  hide it. Surfacing the cinematic-`None` constraint (a truly always-on bar
  litters screenshots) turned a vague "don't hide it" into a precise "persist
  except None", which the tier expresses cleanly.
- `Status` is functionally equal to `Instrument` for the current level cycle; the
  separate tier is justified by semantics (fps/version is neither a flight
  instrument nor a learning aid) and headroom to diverge later, not by present
  behavior. Flagged honestly in the DECISION.

## Self-reflection / for next time

- This is the third playtest note in a row that turned out to be a HUD-visibility
  taxonomy question (drawer hide, hint placement, now status persistence). The
  `HudTier` x `HudVisibility` x `HudDrawerExempt` matrix is becoming the load-
  bearing model for "what shows when"; a short table of (tier -> which levels +
  drawer) in the module docs would save re-deriving it each time. Worth a follow-
  up doc task.
- Applied the confirm-the-artifact + DECISION.md lesson directly: asked the tier-
  vs-setting fork with the constraint named, then recorded it before code. No
  rework this time - the confirmation and the build matched.

## Ledger candidate

- `hud-visibility-is-a-three-axis-matrix`: "what shows when" in this HUD is
  governed by THREE orthogonal axes - `HudTier` (Instrument/Chrome/Status: which
  grave/tilde levels), `HudVisibility` (All/Minimal/None), and `HudDrawerExempt`
  (survives the Tab drawer + z-lift). A new persistent element picks a point in
  that matrix; do not reach for a new bespoke hide path. 20260724-171509.
