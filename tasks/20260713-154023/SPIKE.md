# Spike: Inset kill cam - show the death, don't slam the viewfinder shut

- DATE: 20260713-154023
- STATUS: RECOMMENDED
- TAGS: spike, hud, ux, targeting

## Question

Playtest (user, 2026-07-13): "I would like the inset camera to show the
death of the thing being hit - don't close it instantly when the thing
dies." Today the viewfinder tears down on the exact frame the kill lands -
the most cinematic moment the panel will ever have. What is the cleanest
way to linger on the kill, without warping any gameplay state?

## Context (verified against the code, 2026-07-13)

- The teardown chain: the target dies -> `update_contacts_and_locks`
  clears the `CombatLock` the same frame (validity clear - death/despawn),
  and `drive_inset_camera` (hud/target_inset.rs) sees `lock: None` next
  frame -> panel hidden, RTT camera despawned. The panel closes exactly as
  the explosion begins.
- There is FOOTAGE to film: `handle_entity_explosion` (integrity/
  explode.rs:198) spawns the slicer fragments as WORLD-level entities
  (`MeshFragmentMarker`) with real velocities - they drift through the
  scene after the anchor despawns. A camera that simply stops moving keeps
  rendering them.
- Live tracking of the dead target is impossible anyway: the anchor entity
  despawns (asteroid husk cleanup / ship root despawn), so any linger must
  hold a REMEMBERED pose, not a live one.
- The safety choreography already composes with a linger for free: the
  lock clearing flips WeaponsHot cold (if lowered), which plays the
  SafetyOn click and relaxes the inset frame from hot-red to quiet steel -
  the kill, the stand-down click and the calming border read as one beat
  INSIDE the lingering panel.
- The inset already self-detects its states (lock / zoomable / NO-SIGNAL)
  from polling; a linger is one more presentation state in the same
  reconcile - no targeting-layer changes needed.

## Options considered

- **A. Status quo** (close on death). Rejected by playtest.
- **B (recommended). Freeze-frame kill cam, presentation-only.** The inset
  driver remembers its last framed pose; when the framed target becomes
  UNRESOLVABLE (died) - as opposed to tap-cleared, decayed, or retargeted -
  the panel and camera stay up with the camera FROZEN at that pose for
  `KILL_CAM_SECS` (~2 s), filming the fragments as they fly, then close.
  A NEW combat lock preempts the linger instantly (the live viewfinder
  always wins); the Chrome-tier gate still hides everything immediately.
  Zero gameplay impact: locks, safety, turrets all behave exactly as
  today - this is a camera that takes two seconds to look away.
- **C. Fragment-tracking camera.** Follow the centroid of the spawned
  fragments. Requires attributing fragments to the dead target
  (spawn-time bookkeeping), and fast fragments would swing the camera
  wildly. The static frame gets 90% of the drama for 10% of the code;
  recorded as future polish, not built.
- **D. Delay the lock clear in targeting.** One-line-looking, deeply
  wrong: `CombatLock` is read by turrets, torpedo commit, the safety
  derivation and the AI - a lock lingering on a corpse would keep weapons
  hot and aim guns at debris. Presentation must not warp gameplay state.
  Rejected.

## The design (B, concrete)

- `drive_inset_camera` gains a linger state (a small component or Local on
  the panel/driver): every framed frame records `(target, pose)`; when the
  reconcile would tear down AND the remembered target no longer resolves
  (despawned - the death signature), it enters `Linger { pose, remaining }`
  instead: panel visible, camera kept at `pose`, NO-SIGNAL suppressed,
  countdown on real time. Expiry or a fresh combat lock ends it (the lock
  path re-frames normally).
- Tap-clear does NOT linger: the cleared target still exists, which is the
  discriminator (and the unlatch ghost already narrates that path). The
  30 s decay clear does not linger either (the target is alive; nothing to
  film). Out-of-range/allegiance-flip clears: target alive -> no linger.
- The faction line clears during the linger (its target is gone); the
  frame border keeps tracking the LIVE safety state - which naturally
  relaxes to quiet steel as the kill de-escalates.
- Knobs: `KILL_CAM_SECS` (~2.0); an optional end-fade on the panel border
  is polish, not first-pass.
- Test surface: inset tests (death -> linger with frozen pose + panel up;
  expiry closes; a new lock preempts; tap-clear with a LIVE target does
  not linger - the discriminator pin; Chrome-hide still immediate);
  12_hud_range's end-of-script kills the target ship - verify the final
  asserts tolerate a lingering panel (they assert indicator anchors, not
  the inset camera count, but confirm at plan time).

## Open questions

- Duration feel (1.5-2.5 s) - playtest knob.
- Should the linger end EARLY if the radar opens (search active) even
  before a lock lands, handing the panel to the sweep? (Probably yes -
  cheap to add, decide at plan time.)
- Player death mid-linger: the HUD teardown despawns the panel wholesale -
  confirm nothing dangles (the camera is keyed to the panel's lifecycle).

## Next steps

Direction-level task this spike seeded, for /plan to break into steps:

- tatr 20260713-154217: inset kill cam - freeze-frame linger on target
  death (option B).

## Fix record

- 20260713-154217 LANDED (380c215): option B shipped - explicit
  InsetPanelState (Live/NoSignal/KillCam/Hidden), death discriminator =
  despawned LastFramed target, 2 s frozen final shot, preemption +
  chrome-hide + expiry + discriminator all pinned; 12_hud_range's
  post-kill asserts inverted into the live pin (live run deferred while
  the user's game instance holds the GPU).
