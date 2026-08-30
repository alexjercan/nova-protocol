# Seven bounded polish corrections from round 6

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.12.0

## Goal

Items 1-7 of round 6 of research record `20260815-231945`
(`POLISH-REVIEW-2.md`), taken as one batch. The review's own recommended first
cutoff: bounded, independent, and mostly a reuse of a surface that already
exists. None of them needs the visual direction pass that item 8 owns.

Owner decision 2026-08-30: this batch before vacuum VFX (`20260822-204201`),
because item 8's lighting baseline changes every material judgement and VFX
tuning done before it would be redone.

## The seven

1. **Gate Backquote while NovaOS owns text input.** `hud_cinematic` is bound
   `Context::Always` on Backquote while terminal input accepts a printable
   backquote, so typing one cycles the HUD. Keep the binding; gate the action.
   The clearest remaining correctness defect of the seven.
2. **Finish terminal editing basics.** The shared UI text field already has
   Home / End caret semantics; NovaOS terminal input does not. Reuse them, add
   the swallowed Ctrl-A/E/U/K chords, and let the first deliberate key skip the
   boot delay. Click-to-position stays OUT - it needs glyph-to-caret mapping and
   is a pointer change, not a keyboard one.
3. **Render the target inset at its display size.** It renders 512 px into a
   256 px panel and runs the full marked-camera post chain. Start at 256.
   Keep bloom only if a screenshot comparison proves it buys readable target
   damage at that size.
4. **Make the placeholder hot nozzle emissive.** The fallback art calls the
   nozzle hot red and gives it albedo only. Low reach - authored parts replace
   it - but the intended value is unambiguous.
5. **Consolidate the combat colour tokens.** Red literals drift across the edge
   indicators, lead pips, lock crosshairs, target focus, component lock and the
   inset. Centralise the shared RGB families. PRESERVE per-widget alpha and the
   separate warning meanings: this is consistency, not one opacity for every
   warning.
6. **Say why a combat lock dropped.** `CombatLockDropped` already carries
   TargetGone, OutOfRange, AllegianceFlip and IdleDecay. Route a short line
   through the log/comms path that exists. No new HUD instrument.
7. **Show rated speed where speed already is.** `FlightSpeedCap` is a live
   component and the speed chip already owns that register. A compact
   `current / rated` readout, not another floating instrument.

## Proof

- A test per item where the behaviour is testable headlessly: the Backquote
  gate, the terminal chords and boot skip, the drop reason reaching the log,
  the rated readout.
- Items 3 and 4 need one RENDERED inspection each before landing. A screenshot
  comparison decides whether the inset keeps bloom at 256 px; the review does
  not pre-judge it.
- Item 5 is a refactor with no behaviour change: the proof is that the rendered
  HUD is unchanged apart from the literals that were wrong.

## Progress

- Item 1 landed in `ca55e306`, item 2 in `4fce30ac`, item 6 in `6ec927c2`,
  item 7 in `3eede6d4`, item 4 in `ee6f3469`, item 3 in `0c4608c7`.
- Item 4's rendered inspection: `placeholder-nozzle-emissive.png`, the
  `basic_thruster (today)` subject of `screenshot_thruster_gallery` (which
  spawns `ThrusterSectionConfig::default()`, so it wears the placeholder).
  Left is the shipped albedo-only cone, middle the landed 1.2 emissive, right
  a 2.5 that was shot and rejected for washing the red out.
- Item 3's rendered inspection: `target-inset-512-256-bloom.png` - 512+bloom,
  256+bloom, 256 without. Bloom kept; the reason it earns its place turned out
  to be matching the main view, not target readability.
- Item 5's rendered inspection: `combat-tokens-before-after.png`, the same
  `screenshot_combat_lock` beat either side of the change. The frames differ
  only in where the debris is; every HUD tint in them is identical, because the
  literals those widgets use did not move. The five that DID move are the
  component-lock marker pair, the two edge-indicator reds, the hot lead pip and
  the torpedo focus meter - none of which this capture shows. The marker pair
  is verified against its own docstring, which already said "the same hue at
  full presence" while the literal said otherwise.

## Deliberately out

- Click-to-position in the terminal. Item 2 says so: glyph-to-caret mapping is
  a pointer follow-up.
- Item 8, the lighting baseline. It is the next batch and it is its own
  reviewable unit.
- Making every warning one opacity. Item 5 is explicit that the meanings and
  the alphas stay distinct.
