# Show-don't-tell lock HUD: inset-on-lock viewfinder, state styling, lock sfx, status text retired

- STATUS: OPEN
- PRIORITY: 48
- TAGS: v0.5.0,hud,ux,audio,spike

## Goal

Playtest feedback (2026-07-13): too much text; the RTT inset should carry
the state ("if we have RTT we are not in DUMB fire mode"), and the lock
cleared cue should work without words. Strands B1 + C1 from the spike,
inset-on-lock confirmed by the user 2026-07-13.

## Scope (adversarial round folded in)

- **Inset-on-lock viewfinder** (user-confirmed): the inset appears the
  moment a COMBAT lock exists - including live during a radar sweep. Drop
  the focus-dwell gate for the panel only (drive_inset_camera,
  target_inset.rs:285); dwell keeps gating the component fine-lock and its
  section highlight. No new steady-state RTT cost (F6, verified: 256 px
  target already renders continuously while focused today).
- **Frame carries the state:** color/shape per Q5 recommended (armed
  corner-ticks + red tint when hot - redundant encoding); presence = guided
  torpedoes; gesture-time text per Q6 recommended (name + distance numeral
  only while the radar is held).
- **Non-zoomable target mid-sweep** per Q4 recommended: NO-SIGNAL
  placeholder (panel stays up with text-free static, camera torn down) so
  the viewfinder does not flicker across beacons (F5).
- **Universal fallback for weapons-hot** where no inset/lock crosshair
  exists: armed ticks on the red combat reticle AND on the turret-view aim
  surface (hud/turret_lead.rs) - raised-manual with no lock is hot with
  neither the inset nor a lock crosshair on screen (F4, necessity).
- **Status text retired:** the drive_weapons_status block
  (hud/lock_crosshairs.rs:291) is removed; "TORP: DUMB" dies without
  replacement text - the red combat reticle's presence anywhere IS the
  guided cue; ammo-gauge pip stays a reserved fallback knob.
- **Lock cleared without words** per Q7 recommended: crosshair unlatch
  animation (detached ghost node, scale-up + fade at the last screen
  position, per slot color - reuse the toast spawn/fade shape) + LockOff
  sfx; the text toast is removed; LockClearedToast stays as the internal
  message the animation subscribes to. GOTO-disengage keeps the unlatch pop
  + mode chip change as its only cues (F10, playtest note).
- **Sfx batch** (extend scripts/gen-placeholder-sounds.py + NovaSfx):
  LockOn per Q3 recommended (acquire-only at the threshold, silence during
  retargets - F2 spam guard), LockOff on clear, the safety OFF->ON blip
  absorbed from 20260713-090653, and the capability deny cue per Q8
  recommended (deny buzz + brief red flash of the radar adornment) - F7:
  the deny cue promised to 082337 never landed; also fix the stale comment
  in on_radar_start (targeting.rs:~750).
- **Provisional visuals collapse** (F11 default): nothing inside the 250 ms
  tap window; past threshold the hollow radar box becomes the radar-active
  adornment around the solid crosshair, gone on release.

## Notes

- Spike: docs/spikes/20260713-110039-show-dont-tell-radar-ux.md (strands
  B1 + C1, adversarial round F2/F4/F5/F6/F7/F10/F11, questionnaire
  Q3-Q8).
- Depends on: 20260713-110330 (live lock changes what the HUD must show).
- Blocked on the questionnaire answers (Q3-Q8); recommended defaults above
  make the task plannable on "all recommended".
- Main surfaces: hud/lock_crosshairs.rs, hud/target_inset.rs,
  hud/torpedo_target.rs, hud/turret_lead.rs, audio.rs, the
  placeholder-sound generator.
- Coordinate with 20260713-090653 (blip already re-pointed here).
- /plan before implementation.
