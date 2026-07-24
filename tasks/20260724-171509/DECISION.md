# DECISION: a third HudTier::Status for persistent reference chrome

- DATE: 20260724
- STATUS: ACCEPTED (owner gate 2026-07-24)
- TASK: 20260724-171509
- TAGS: decision, ui, hud

## Context

The top-right status bar (bcs `status_bar`: fps + version + the objective count)
is `HudTier::Chrome`, so it hides at the grave/tilde `Minimal` and `None` levels
AND (after task 20260724-134335) on drawer-open. The owner reports it should not
be governed by the gameplay HUD-hide the way flight instruments and learning
cues are: it is reference/overlay chrome ("like the FPS overlay") and should
persist while flying and while the drawer is open, clearing only for a
deliberate cinematic screenshot.

The load-bearing constraint is the cinematic `None` level (grave/tilde's
clean-screen mode): a truly always-on bar would leave fps/version text in the
corner of every capture. So "always on" is wrong; "persists except cinematic"
is the target.

## Options weighed (owner picked the tier)

- **New `HudTier::Status` (CHOSEN)** - visible at `All` + `Minimal`, hidden at
  `None`; drawer-persistent via the existing `HudDrawerExempt`. Semantically
  honest (fps/version is neither a flight Instrument nor a learning-aid Chrome),
  consistent with how the top-center readout strip already behaves, small change.
- **Own "show status bar" on/off setting** - a dedicated toggle fully decoupled
  from the HUD cycle. Closest to a literal FPS overlay, but a bigger change (a
  settings entry + toggle key + persistence) and it re-opens the cinematic
  question. Deferred; can layer on top of the tier later if wanted.
- **Drawer-exempt only** - keep it `Chrome`, just exempt it from the drawer hide.
  Smallest, but it still vanishes at `Minimal`, which does not match the
  always-there feel. Rejected.

## Decision

Add `HudTier::Status`. `HudVisibility::shows` treats `Status` like `Instrument`
for the level cycle (shown at `All` and `Minimal`, hidden at `None`). The status
bar is retagged `Chrome` -> `Status` and given `HudDrawerExempt` +
`GlobalZIndex::default()` so it persists through the drawer and rides the
existing z-lift above the backdrop. The objective count (a child of the bar)
inherits this.

`Status` reuses the existing `HudDrawerExempt` machinery for drawer-persistence
rather than folding drawer-exemption into the tier: a value-filtered lift query
would risk mis-touching the drawer panels' own `GlobalZIndex`, so keeping the
marker as the z-lift trigger is the safe reuse. Status widgets therefore carry
both the tier (Minimal visibility + semantics) and the marker (drawer
persistence + z-lift) - two orthogonal axes.

## Consequences

- The status bar (and the objective count in it) stays visible while flying, at
  `Minimal`, and during the drawer; `None` still clears it for clean captures.
- The objective count shows during the drawer even though the drawer's right
  panel lists objectives in full - harmless (compact reference vs detail); the
  "TAB" affordance reading while the drawer is open is a minor oddity left for a
  possible later polish.
- The top-center readout strip (`Instrument` + `HudDrawerExempt`) already yields
  the same behavior and is NOT migrated here to avoid touching landed code; a
  later move to `Status` for semantic consistency is a reasonable follow-up.
- `Status` is functionally equal to `Instrument` for the current level cycle;
  the separate tier buys semantic clarity and room to diverge later (e.g. if a
  future level should treat status chrome differently from flight instruments).
