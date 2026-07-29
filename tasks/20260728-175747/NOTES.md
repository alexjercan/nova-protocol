# Notes: contextual HUD - show-by-relevance, grow-in-use, On/Cinematic

Design record for the shipped change (task 20260728-175747), layered on the
restyle 20260728-175742.

## What was built

Three new pieces in `crates/nova_gameplay/src/hud/`, plus per-widget drivers:

- **`situation.rs`** - `HudSituations`, sensed once per frame from the player
  ship (`Autopilot` -> which `FlightVerb`, `CombatLock`, `WeaponsHot`,
  `TurretSectionInput`, `SectionAmmo`). Every contextual rule keys off this one
  resource, so the ruleset reads in one place instead of a dozen queries that
  can drift. Sensed in its own `HudSituationSensing` set ordered BEFORE
  `NovaHudSystems`, so every widget driver reads this frame's truth.
- **`emphasis.rs`** - `HudEmphasis`, the single grow-in-use mechanism, written
  to `UiTransform::scale` by one PostUpdate system (before UI layout, so a
  driver's `set_held`/`pop` lands the same frame). Two modes: `Settle` (hold
  while the situation lasts, or a one-shot `pop`) and `Pulse` (oscillate while
  it lasts). Demo 2's values are the constants at each site: speed 1.14, lock
  readout 1.12, hot dock chip 1.08, objective pop 1.16/1.2 s, comms 1.12/0.9 s,
  reticle pulse 1.12 over a 0.56 s cycle, all with demo 2's 0.2 s ease.
- **`HudContextGate(bool)`** in `hud/mod.rs` - the contextual SHOW rule, written
  by each widget's own driver and enforced centrally inside
  `apply_hud_visibility` (which already runs after the screen-indicator
  projection, the only place a projected indicator's every-frame
  `Visibility::Visible` can be overruled). Gate resolution walks ancestors like
  the tier does, so the ammo readouts inherit their layer's gate.

Rules wired: ammo gauges gated on weapons-hot OR low-ammo; speed chip grown on
an engaged maneuver; lock readout grown on weapons-hot; reticle pulsed while
the trigger is down; dock chip of the verb the ship is DOING inverted + grown
(the engaged maneuver's own chip, CANCEL while anything is engaged, RADAR while
a combat lock is held); objective hint popped when the reveal card tucks in and
then breathing slowly while work is outstanding; comms card arriving grown.

The `~` cycle collapsed from `All/Minimal/None` to `On/Cinematic`.

## Decisions

- **`HudVisibility::shows()` lost its tier parameter.** With two levels every
  tier answers the same (on at `On`, cleared at `Cinematic`), so a `shows(tier)`
  that ignores its argument would be a lie. `HudTier` itself stays: its live job
  is marking a subtree HUD-MANAGED (the indicator pass skips untagged trees) and
  it is the vocabulary the wiki and the NOVA OS exemption rules speak.
- **`Hot` is checked BEFORE availability in the dock.** `Hot` means "this is
  what the ship is doing", not "press this". The ORBIT offer is retired the
  moment you are parked (`orbit.available` goes false), which is exactly when
  the ORBIT chip should read as the live maneuver. Pinned by
  `an_engaged_orbit_stays_hot_after_its_offer_is_retired`.
- **The comms DWELL was NOT retimed to the demo's 5 s.** The panel already has a
  richer model (authored per-line dwell clamped 3..30 s, default 8 s, plus a
  floor while lines wait) and `nova_assets`'s pacing layer derives its beat gaps
  from those constants - retiming them to a POC number would silently re-pace
  every authored conversation. Only the arrival emphasis was adopted. Recorded
  here so review does not read it as a missed step.
- **The comms card's emphasis is seeded from the line's AGE**
  (`HudEmphasis::popped_at_age`), because `sync_comms_cards` rebuilds the whole
  stack every frame - a freshly spawned component would restart its ease every
  frame and never leave rest. Same shape as the card's existing age-derived
  alpha.
- **The objective pop rides the reveal's TUCK, not the posting.** A new
  `ObjectiveRevealTucked` message fires when the card finishes tucking into the
  hint; the hint pops on it. One motion instead of two animations of the same
  news, and a card cleared early by scenario teardown never sends it.
- **The hint's breath is opacity, not scale**, so it is a small dedicated system
  (`breathe_hint`) rather than part of the shared scale mechanism. bevy_ui has
  no node-level opacity and the parts do not share a base alpha, so each
  breathing part carries its own `base_alpha` and the system writes
  `base * wave` absolutely (never multiplied onto the previous frame).
- **Allegiance triangles left as-is** - the accepted ruleset does not cover
  them either way. Recorded in `allegiance_markers.rs` and still an open
  question for the owner's playtest.
- **The lock readout is a CHILD of the reticle node**, so it inherits the
  reticle's firing pulse: while shooting, the whole lock instrument breathes
  (up to ~1.25 combined at the pulse peak). Deliberate and documented at the
  site; if the playtest finds it jumpy the fix is to drop the readout's own
  hold, not to restructure the node tree.

## Verification

- `cargo check --workspace --all-targets --features dev` clean; `cargo fmt`
  clean.
- `cargo test -p nova_gameplay --lib -- hud::` 278 passed, plus
  `-p nova_menu --lib` 73 passed and the keybind-reference parity tests. New
  App-driven pins: the ammo gate both ways, the speed-chip burn emphasis, the
  dock's hot rules (including the retired-ORBIT case), the hint pop-and-settle
  over virtual time, the hint breath, the comms arrival pop, the reticle vs
  readout split, and the gate's enforcement against the projection's
  every-frame `Visible`.
- `cargo run -p nova_probe -- run playable` -> OK (5/6 measured, fps SKIPPED:
  no baseline), `probe-runs/ff59c72c/playable/report.html`.
- GPU eyeball under Xvfb: `screenshot_combat` (combat lock -> reticle, readout,
  inset, RADAR chip inverted AND grown, dock otherwise dim) and `menu_newgame`
  into `shakedown_run` - a FINITE-ammo ship in idle cruise with NO ammo gauges
  on its turrets, which is the contextual gate's closed state on a ship that
  actually has gauges. `lifeline` ran the full chapter walk clean.
- NOT visually verified: the gauges APPEARING on weapons-hot. No harnessed
  example fires the player's guns (every example ship is `infinite_ammo: true`,
  and the scripted walks kill via `HealthApplyDamage`), so no frame in the
  example set has the safety off on a finite-ammo ship. The open state is
  covered by the App-driven gate tests and by the owner's playtest DoD item.
