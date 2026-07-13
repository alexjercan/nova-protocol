# Spike: Show-don't-tell radar UX - live lock, inset-as-status, less text

- DATE: 20260713-110039
- STATUS: RECOMMENDED (playtest-gated knobs listed)
- TAGS: spike, targeting, hud, ux, audio

## Question

Playtest feedback on the landed deliberate-radar family (spike
20260713-082207, all four tasks landed 2026-07-13):

1. **Too much text.** The status block ("WEAPONS HOT [RAISED]: lock <name>
   <dist>m" + "TORP -> <name>"/"TORP: DUMB") should mostly disappear; the RTT
   inset can carry the state - "obviously if we have RTT we are not in DUMB
   fire mode".
2. **"Lock cleared" toast** is liked as a signal but should become intuitive
   without text.
3. **Gesture annoyance:** having to hold CTRL and wait for release. Proposal:
   after the existing 0.25 s threshold the radar LOCKS the first candidate,
   then retargets instantly while held, and release just makes it stick.

How do we deliver all three with the least text and the most reuse of what
already renders? More playtest is expected; the answer should be a direction
plus explicit knobs, not a final pixel spec.

## Context (verified against the code, 2026-07-13)

- Radar today is lock-on-release: `RadarState.candidate` follows the look ray
  from CTRL press (`update_radar_search`, targeting.rs:577), the slot is
  written only in `on_radar_commit` (targeting.rs:767, fires on
  `Complete<RadarHoldInput>`); `RADAR_TAP_SECS = 0.25` (targeting.rs:167) is
  both the tap-clear window and the Hold threshold, so tap-vs-hold
  discrimination is already exactly the boundary the proposal needs.
- The status text lives in `drive_weapons_status`
  (hud/lock_crosshairs.rs:291); toasts in `spawn_lock_toasts` (:244); the
  provisional radar box carries a `Name` label (`drive_radar_candidate`,
  :206).
- The RTT inset (hud/target_inset.rs, `drive_inset_camera`:285) is gated on
  FOUR conditions: combat lock + **focus dwell complete** (1.5 s) + HUD
  Chrome tier + `InsetZoomable` target. So today "inset visible => guided
  torpedoes" holds, but the converse does not: a fresh combat lock shows no
  inset for 1.5 s, and beacon/non-zoomable locks or reduced HUD tiers never
  show one. Any inset-as-status design must either move the inset to
  lock-time or accept a fallback cue.
- Retargeting the inset is cheap: the camera is moved, not respawned, when
  the anchor changes; teardown happens only on unfocus. Making it live during
  a radar sweep is a gating change, not an RTT rework.
- Sound is NOT a blocker: `NovaSfx` bank (audio.rs:40) has 7 cues and
  `scripts/gen-placeholder-sounds.py` deterministically synthesizes the
  placeholder wavs - lock-on/lock-off/safety cues are a generator extension
  plus new `(key, file)` pairs. The safety OFF->ON blip currently parked in
  task 20260713-090653 belongs to this same batch.
- Consumers are already capture- or derive-based, which is what makes a LIVE
  lock safe: GOTO captures the travel lock at [G] (no live re-read, pinned by
  `goto_keeps_the_captured_target_across_re_designation`), safety derives
  per-frame, focus dwell resets on retarget, turret auto-track reads the
  current lock.

## Strand A - the gesture: live lock while held

- **A1 (proposed by the user): lock at threshold, retarget while held, stick
  on release.** Press starts the radar as today (provisional only inside the
  0.25 s tap window, so tap-clear is untouched); at the Hold threshold the
  latched slot is WRITTEN with the current candidate; every held frame the
  slot retargets with the existing hysteresis; `Complete` just ends the radar
  (the lock already holds). D1 (empty release = no-op) becomes "never saw a
  candidate while held = slot untouched". Implementation is a simplification:
  the commit observer's write moves into `update_radar_search`; the
  provisional/committed visual distinction collapses into "crosshair solid
  from threshold, radar-active cue while held".
- **A2: status quo** (lock-on-release). Deliberate but slow; the user has
  playtested it and called the wait an annoyance. Rejected.
- **A3: lock at press** (no threshold). Kills tap-clear (every tap would lock
  first). Rejected - the 0.25 s window is load-bearing.

Sub-choice for A1, sweeping over EMPTY space after a lock was acquired:
- **keep-last (recommended):** the lock stays on the last valid target;
  sweeping past gaps never drops it; tap remains the only way to clear.
- follow-to-none: sweeping to empty clears the slot. More literal but
  punishes overshoot, and clearing already has a gesture.

Quirks to keep in mind (adversarial pass):
- Safety goes hot at threshold in combat mode (lock exists earlier than
  before). In practice RMB is already held there (the combat slot only
  latches while raised), so weapons were hot anyway - no new exposure.
- RMB released while CTRL still held (combat sweep continues): turret
  auto-track now follows the sweep live. Same class as today's post-commit
  behavior, just sooner; manual-wins while raised is unchanged. Playtest.
- Focus dwell restarts on every retarget, so no accidental inset/fine-lock
  churn mid-sweep under today's gating (moot if B1 moves the inset to
  lock-time, where the inset intentionally follows the sweep).
- Tests: the gesture e2e family (exact-boundary frame, D1 no-op, commit
  asserts) and the 12_hud_range script re-pin threshold-commit instead of
  release-commit.

## Strand B - status surface: the inset carries it

Text to retire: the whole `drive_weapons_status` block. State it encodes:
weapons hot/safe, why (raised vs lock), lock target name + distance, torpedo
guided vs dumb.

- **B1 (recommended): inset-on-lock, viewfinder while sweeping.** Drop the
  focus-dwell gate for the inset PANEL (keep dwell for the component
  fine-lock/highlight it was designed for): the inset appears the moment a
  combat lock exists - including live during an A1 sweep, which turns the
  radar into a viewfinder and is the single biggest "use the RTT more" win.
  The inset frame carries the state without words: frame color = safety
  (hot red / safe neutral), presence = guided torpedoes, a small distance
  numeral on the frame edge (a number, not prose), target name only while
  the radar is held (it doubles as the sweep's confirmation readout, then
  fades). Beacon/non-zoomable combat locks and sub-Chrome HUD tiers get no
  inset, so a minimal non-text fallback must exist -> the reticle itself:
  the red combat crosshair gains "armed" tick marks / a filled center while
  weapons are hot. That reticle cue is the universal truth; the inset is the
  rich version.
- **B2: keep the dwell gate,** move the text onto reticle styling only.
  Smaller change, but the inset then still says nothing for 1.5 s per lock
  and the user's core ask (make the RTT carry it) is only half-served.
- **B3: do nothing** (keep text). Rejected by playtest.

Torpedo guided/dumb specifically: guided iff a combat lock exists, which B1
makes visible as inset-presence + the existing combat reticle on the target.
The "TORP: DUMB" line dies without replacement text; if playtest misses it, a
small hollow-vs-filled pip on the ammo gauge is the non-text fallback (knob,
not part of the first pass).

## Strand C - lock cleared without a toast

- **C1 (recommended): unlatch animation + sound.** On clear, the crosshair
  visibly unlatches: scale-up + fade at the target's last screen position
  (white for travel, red for combat - the staged tap naturally reads as two
  distinct pops), the inset collapses, and a `LockOff` cue plays from the
  extended placeholder generator (`LockOn` at threshold-lock is the natural
  sibling; the 090653 safety blip joins the same batch). The text toast is
  removed once the animation+sfx land; the existing `LockClearedToast`
  message stays as the internal event the animation subscribes to.
- **C2: keep the toast, add sound.** Cheapest, but keeps the text the user
  wants gone.

## Recommendation

A1 (live lock, keep-last on empty) + B1 (inset-on-lock viewfinder, frame
styling, reticle armed-ticks as universal fallback, status text block
removed) + C1 (unlatch animation + LockOn/LockOff/safety sfx, toast retired).
Two tasks, gesture first (it changes what the HUD must show), HUD second.
Everything visual/audio lands behind the existing playtest loop; the knobs
below are expected to move.

## Adversarial round (2026-07-13, code-checked)

User confirmed inset-on-lock before this round. Attacks run against the
recommendation; defused ones are recorded so nobody re-derives them.

- **F1 - slot latch timing (opportunity).** Live-lock moves the first slot
  write from release to threshold, which makes PRESS-time latching
  (`on_radar_start`, targeting.rs:725, reads `WeaponsRaised` at Start) worse
  than it needs to be: the recorded same-frame RMB+CTRL sharp edge exists
  precisely because raised derives in Update while the radar Start observer
  runs PreUpdate. Latching the slot AT THE THRESHOLD instead (0.25 s later,
  raised state long settled) kills that edge entirely and matches intent
  ("raise and radar together" = combat lock). -> Q1.
- **F2 - LockOn sfx spam.** Under live-lock the slot is written per retarget;
  a LockOn cue naively tied to "lock written" machine-guns during a sweep.
  The cue needs an explicit granularity decision. -> Q3.
- **F3 - retarget churn: DEFUSED, verified.** No system reads
  `Changed<TravelLock>`/`Changed<CombatLock>` or observes lock inserts
  (grep 2026-07-13); consumers poll. The focus dwell resets by comparison
  (`tick_lock_focus`, targeting.rs:925-931), toasts fire only from the
  tap-clear observer. Per-frame slot writes are safe.
- **F4 - hot cue with NO lock.** Raised-manual (RMB held, no combat lock) is
  weapons-hot with no lock crosshair and no inset on screen. The armed cue
  must also live on the turret-view aim surface (hud/turret_lead.rs), or the
  hot state is invisible exactly when the player is free-aiming. Folded into
  110311 scope as a necessity, not a question.
- **F5 - non-zoomable flicker.** Sweeping across a beacon (lockable, not
  `InsetZoomable`) blanks the viewfinder camera and panel, then it reappears
  on the next ship - a flicker in the middle of the marquee feature. -> Q4.
- **F6 - RTT cost: DEFUSED, verified.** The inset is a 256 px render target
  (INSET_PANEL_PX, target_inset.rs:55) and today's steady state already
  renders it continuously while a lock is focused; inset-on-lock adds only
  the 1.5 s dwell head start and sweep time. No new steady-state cost.
- **F7 - capability deny cue never landed (gap).** `on_radar_start`'s deny
  branch comments that the cue "lands with the HUD task 20260713-082337" -
  it did not (grep: no deny cue in lock_crosshairs.rs or player.rs). A
  lock-less computer holding CTRL is a silent no-op today. Fold a non-text
  deny cue (buzz sfx + brief radar-adornment flash) into 110311 and fix the
  stale comment. -> Q8 for the style.
- **F8 - sweep text is still text.** The name label + distance numeral run
  against the less-text goal; they are defensible as transient gesture-time
  info, but the user should own the call. -> Q6.
- **F9 - boundary re-pin (test cost).** With the lock written mid-gesture,
  the exact-boundary e2e must re-pin: sub-threshold release (tap) fires
  clear having never written; threshold-frame release must not both lock and
  clear. Known cost, owned by 110330.
- **F10 - GOTO disengage loses its words.** Travel tap-clear disengages an
  engaged GOTO; with the toast retired, the cues are the unlatch pop + the
  mode chip change. Probably enough; flagged for playtest, owned by 110311.
- **F11 - provisional visuals collapse.** Pre-threshold, the candidate box
  is visible for at most 250 ms - sub-perceptual. Default chosen in-task: no
  candidate visual inside the tap window; past threshold the hollow radar
  box becomes the radar-active adornment around the SOLID crosshair, gone on
  release. Feel knob, not a questionnaire item.
- **F12 - decay interplay.** A retarget write should reset the 30 s
  `CombatDecay` (any lock write = combat activity), or a long sweep could
  cross the decay boundary mid-gesture. Plan detail, owned by 110330.

## Questionnaire (technical decisions - ANSWERED 2026-07-13: all recommended)

User decision 2026-07-13: **all recommended** - Q1a threshold latch, Q2a
keep-last, Q3a acquire-only LockOn, Q4a NO-SIGNAL placeholder, Q5a
shape+color hot cue, Q6a name+distance while held, Q7a toast removed now,
Q8a deny buzz + flash. Recommended option first and bolded.

- **Q1 - combat/travel slot latch timing.** **(a) latch at the Hold
  threshold** (kills the F1 same-frame edge; raised state settled; matches
  intent) / (b) keep latching at CTRL press (status quo, keeps the recorded
  sharp edge).
- **Q2 - sweeping over empty space after acquiring.** **(a) keep-last** (the
  lock stays on the last valid target; tap is the only clear) / (b)
  follow-to-none (sweeping to empty clears the slot).
- **Q3 - LockOn sfx granularity.** **(a) acquire-only** (one cue at the
  threshold acquisition, silence during retargets) / (b) acquire + a subtle
  per-retarget tick (radar-like, risks noise) / (c) acquire + a stick-confirm
  on release (three-phase, most audio).
- **Q4 - non-zoomable target in the viewfinder (beacon mid-sweep).**
  **(a) NO-SIGNAL placeholder** (panel stays up showing text-free static /
  noise, camera torn down) / (b) hide the panel (status quo behavior,
  accepts the F5 flicker) / (c) freeze the last frame (stale info, listed
  for completeness).
- **Q5 - safety-hot cue style (inset frame + reticles).** **(a) shape +
  color** (armed corner-ticks appear AND the tint shifts red - redundant
  encoding, colorblind-safe) / (b) color-only / (c) shape-only.
- **Q6 - gesture-time text.** **(a) name + distance numeral while the radar
  is held only** (transient, info-dense moment) / (b) name only / (c) fully
  text-free.
- **Q7 - lock-cleared toast retirement.** **(a) remove now** (unlatch
  animation + LockOff sfx replace it in the same task) / (b) keep the text
  toast behind a playtest knob during a transition period.
- **Q8 - capability deny cue (F7).** **(a) deny buzz sfx + brief red flash
  of the radar adornment** / (b) leave silent (status quo no-op).

## Open questions (playtest knobs, non-blocking)

- Does the inset-as-viewfinder feel right during fast sweeps, or does it
  need a small retarget debounce?
- Does the travel lock (no inset by design) also want a transient name
  readout beyond the sweep-time label?
- Does dropping "TORP: DUMB" lose anything in practice (ammo-gauge pip is
  the reserved fallback)?
- Placeholder sfx tone/mix once heard in-game.

## Next steps

Direction-level tasks this spike seeded, for /plan to break into steps:

- tatr 20260713-110330: live radar lock - lock at threshold, retarget while
  held, stick on release (strand A1).
- tatr 20260713-110311: show-don't-tell lock HUD - inset-on-lock viewfinder,
  frame/reticle state styling, unlatch animation, lock/safety sfx, status
  text retired (strands B1+C1; depends on 110330; absorbs the safety blip
  from 20260713-090653).

## Fix record

(appended by the implementing tasks as they land)
