# Show-don't-tell lock HUD: inset-on-lock viewfinder, state styling, lock sfx, status text retired

- STATUS: CLOSED
- PRIORITY: 48
- TAGS: v0.5.0,hud,ux,audio,spike

## Outcome (CLOSED 2026-07-13)

Shipped per plan (strands B1 + C1, Q3a-Q8a). Notes against the plan:

- Q6a applied LITERALLY: name AND distance are gesture-time only (one
  caption on the inset frame while a COMBAT sweep is engaged; the radar
  box label carries name+distance for TRAVEL sweeps - no doubled text).
  The plan step's "distance numeral while the panel is up" was a drafting
  slip; the questionnaire answer wins.
- The turret lead pips take a COLOR-ONLY hot shift (PIP_HOT_COLOR): ticks
  at 8 px would be noise - the deliberate exception to Q5a's shape+color,
  documented in code. The inset frame carries 8 ticks (an L per corner),
  the combat reticle 4 corner pips.
- The unlatch ghost anchors the cleared target via the screen-indicator
  widget; the "center-screen fallback" was dropped - the tap only fires on
  a Some slot, so the toast always carries a target.
- Folded in mid-task (user, live): the inset panel moved below the bcs
  FPS/latency status bar (top 12 -> 44 px, INSET_TOP_PX; it overlapped).
- The 090653 safety blip is absorbed here (SafetyOn on the player's
  hot->cold edge); 090653's task file was already re-pointed.
- run_system_once vs MessageReader cursor bit once more in the deny-flash
  test; fixed with the ledger's registered-system pattern.

Verified: 471 nova_gameplay lib tests (inset family re-pinned to
lock-time, NO-SIGNAL, frame/reticle/ghost/deny tests; audio bank pairing
at 11 cues); 12_hud_range live-asserts the viewfinder up MID-SWEEP with
armed ticks on; 10_gameplay + 03_scenario autopilots exit 0; fmt clean.

## Goal

Playtest feedback (2026-07-13): too much text; the RTT inset should carry
the state ("if we have RTT we are not in DUMB fire mode"), and the lock
cleared cue should work without words. Strands B1 + C1; inset-on-lock
confirmed by the user.

Questionnaire ANSWERED 2026-07-13: all recommended (Q3a acquire-only
LockOn, Q4a NO-SIGNAL placeholder, Q5a shape+color hot cue, Q6a
name+distance while radar held, Q7a toast removed now, Q8a deny buzz +
flash).

## Steps

- [x] Messages (targeting.rs): add `RadarDenied` emitted in
      `on_radar_start`'s capability-deny branch and fix its stale comment
      ("lands with 082337" - it never did, F7); extend `LockClearedToast`
      with `target: Option<Entity>` (the ghost needs an anchor) at every
      writer (tap observer + any decay/validity clear that toasts).
      `RadarLockAcquired` lands in 110330.
- [x] Placeholder sfx (scripts/gen-placeholder-sounds.py SOUNDS dict): add
      `lock_on` (short rising blip, distinct from objective_new),
      `lock_off` (falling), `safety_on` (dull double click), `radar_deny`
      (low buzz); run the generator, commit the wavs. audio.rs: NovaSfx
      variants + NOVA_SFX_FILES entries (the every_nova_sfx_key_has_a_file
      test pins the pairing).
- [x] Audio cue systems (audio.rs, follow the objective UI-cue pattern
      ~:440): LockOn on `RadarLockAcquired` (acquire-only, Q3a/F2 spam
      guard); LockOff on `LockClearedToast`; SafetyOn on the player's
      `WeaponsHot` hot->cold edge (Changed-gated, player ship only);
      RadarDeny on `RadarDenied`.
- [x] Inset-on-lock (target_inset.rs:285 `drive_inset_camera`): the panel
      shows whenever the player's `CombatLock` is `Some` + Chrome tier -
      the focus-dwell gate moves off the panel/camera entirely (dwell keeps
      gating only `ComponentLock`/section highlight, which are already
      focus-scoped upstream). Camera spawns only for a zoomable resolvable
      target; a `Some` lock on a NON-zoomable target (beacon) shows the
      NO-SIGNAL placeholder instead (Q4a/F5): a dark overlay child with a
      time-driven alpha flicker, text-free, camera torn down - the panel
      never blinks mid-sweep.
- [x] Inset frame state styling (Q5a, target_inset.rs:164
      `target_inset_hud`): red-tinted border + armed corner-tick nodes
      while the player is `WeaponsHot`, neutral border + no ticks while
      safe; a small distance numeral on the frame edge while the panel is
      up; the target `Name` label ONLY while a radar gesture is engaged
      (Q6a - it is the sweep's confirmation readout).
- [x] Universal hot fallback (F4): armed-tick styling on the combat
      reticle (hud/torpedo_target.rs) and hot-shifted turret lead pips
      (hud/turret_lead.rs PIP_COLOR) while `WeaponsHot` - raised-manual
      with no lock has neither inset nor lock crosshair on screen.
- [x] Retire the status text (lock_crosshairs.rs): delete the
      WeaponsStatus node, `drive_weapons_status` (:291) and its tests -
      the red reticle's presence IS the guided cue; "TORP: DUMB" dies
      without replacement (ammo-gauge pip stays a reserved knob).
- [x] Unlatch ghosts replace toast text (Q7a, lock_crosshairs.rs): on
      `LockClearedToast` spawn a ghost crosshair (screen-indicator node
      anchored to the cleared target, slot-colored, scale 1 -> ~1.6 +
      alpha -> 0 over ~0.7 s, then despawn; center-screen fallback when the
      target is gone); staged taps read as two pops. Delete the text-toast
      spawn; keep the stack/fade shape as the ghost driver. GOTO-disengage
      cues = the pop + mode chip (F10, playtest note).
- [x] Radar-active adornment (F11 default): while a gesture is engaged the
      hollow box rides the ENGAGED SLOT's target (not the raw candidate -
      keep-last means candidate can be None while the lock holds); nothing
      renders inside the tap window; box gone on release. The Q8a deny
      flash reuses this box (brief red flash on `RadarDenied`).
- [x] Tests: inset family rewrite (panel at lock without dwell; NO-SIGNAL
      overlay for a non-zoomable lock - panel visible, camera absent;
      Chrome-tier gate intact; highlight still dwell-gated); frame + reticle
      + pip hot/safe styling; name-only-while-engaged; ghost
      spawn/fade/despawn; audio: bank pairing test updated, LockOn once per
      gesture, SafetyOn only on the player's hot->cold edge, deny message
      emitted when the capability is missing.
- [x] 12_hud_range script: inset asserts move to lock-time (component
      fine-lock stages keep their dwell waits); replace status-text asserts
      with frame/reticle state asserts.
- [x] cargo fmt + cargo check; new/rewritten test filters; the three
      autopilots.

## Notes

- Spike: tasks/20260713-110039/SPIKE.md (strands
  B1 + C1, adversarial F2/F4/F5/F6/F7/F10/F11, questionnaire Q3-Q8 -
  answered).
- Depends on: 20260713-110330 (RadarState.engaged, RadarLockAcquired).
- No new steady-state RTT cost (F6, verified: the 256 px target already
  renders continuously while focused today).
- Main surfaces: hud/lock_crosshairs.rs, hud/target_inset.rs,
  hud/torpedo_target.rs, hud/turret_lead.rs, audio.rs, the sound
  generator. Blip already re-pointed here from 20260713-090653.
