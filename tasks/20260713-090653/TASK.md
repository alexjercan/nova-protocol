# Shakedown scenario rework for the radar era: teach the radar, text pass, lock capability beat

- STATUS: CLOSED
- PRIORITY: 45
- TAGS: v0.5.0, scenario, tutorial, polish

## Outcome (CLOSED 2026-07-13)

Shipped per the swept plan:

- LOCK is withheld on the player controller at spawn (clone-and-override,
  next to GOTO) and granted by the beat 3 -> 4 handler - the "targeting
  computer online" capability moment; before it, CTRL answers with the
  deny buzz + flash. Pinned in the beat-walk against the REAL controller
  section (withheld at boot, granted at beat 4 - delivery-guarded, the
  governor pin shape), which also proves the SetControllerVerb(Lock)
  action executes through the real event pipeline.
- Beat 4 teaches the live-lock gesture in its text and pulses BOTH new
  keys (RADAR + GOTO); the contextual cluster makes the progression
  visible (GOTO shows early via emphasis, lights when the lock lands).
  Beat 5 teaches raise + combat lock + viewfinder before firing; the
  epilogue teaches the stand-down tap. Emphasis pairing re-pinned
  (RADAR/GOTO set on beat 4, cleared on the orbit handler).
- Scavenger-fires-under-gate: closed as compositional - AI cadence ->
  input, mirror -> WeaponsHot, and the section gate's hot-fires delivery
  guard are each pinned in nova_gameplay; ships_are_minimal pins the
  scavenger's AI + turret silhouette. The in-game beat-5 fight is the
  user's playtest confirm (they are actively playtesting this build).
- Deviation: no new "gesture rows" were built - the swept plan records
  why (the contextual cluster + [CTRL] RADAR row + emphasis + objective
  text supersede the old sub-item).

Verified: 16 nova_assets tests (walk incl. capability lifecycle + both
emphases), fmt + check clean. The 03_scenario autopilot run is DEFERRED:
the user's game instance is running (contention flake documented in
20260713-124000); the beat-walk exercises the same event pipeline
headlessly. Run it once the session closes.

## Goal

Once the deliberate-radar family lands, the Shakedown Run needs a rework
pass (user request 2026-07-13): the tutorial teaches the GOTO leg's radar
lock only via text and never teaches combat locking at all - beat 5 is pure
manual gunnery, so the viewfinder inset / fine-lock / guided torpedoes are
undiscoverable. Teach the radar where it is the natural tool, using the
lock CAPABILITY as a tutorial beat (the targeting computer "comes online"
like the speed-governor release).

STALE-DOC SWEEP (2026-07-13, before implementation - the task predated
five landed changes):

- The radar is now LIVE-LOCK (spike 20260713-110039 A1): it grabs at the
  0.25 s threshold and retargets under the sweep; release just sticks.
  Beat text phrased for lock-on-release is re-phrased for this.
- The lock-cleared TOAST is gone (unlatch ghosts + LockOff sfx); "toasts"
  in the old scope text is dead vocabulary.
- The inherited 082337 audio blip SHIPPED in 20260713-110311 (SafetyOn on
  the hot->cold edge) - nothing to do here.
- The keybind cluster gained a [CTRL] RADAR row (20260713-130305) and went
  CONTEXTUAL (20260713-131820): rows show only while actionable, and an
  EMPHASIZED verb shows early - which is exactly the tutorial spotlight
  this task needs; the old "context-sensitive hold/tap gesture rows"
  sub-item is superseded by row + emphasis + objective text.
- The RADAR DENY cue exists (buzz + centered flash, F7/Q8a): a lock-less
  computer pressing CTRL gets feedback, which makes the capability beat
  self-explaining before the grant.
- `SetControllerVerb` and `emphasize("RADAR")` both exist (the GOTO unlock
  already uses the former; RADAR is a ROW_VERBS member since 130305).

## Steps

- [x] Withhold the Lock capability at spawn: `player_ship`'s
      clone-and-override controller sets `verbs.lock = false` next to the
      existing `goto = false` (shakedown.rs:285-299). Pre-grant, CTRL
      buzzes + flashes (the deny cue) - diegetically "no targeting
      computer yet".
- [x] Beat 3 -> 4 handler: grant `FlightVerb::Lock` via SetControllerVerb
      (next to the beacon-3 spawn), re-write OBJ_B4 for live-lock +
      the capability moment ("Targeting computer online. Hold [CTRL] and
      put your eyes on BEACON 3 - the white lock sticks when you release.
      Then [G] ..."), and `emphasize("RADAR")` alongside the existing
      GOTO emphasis (the contextual cluster shows the emphasized GOTO row
      early, dim + gold, and lights it when the lock lands - the
      progression is visible in the cluster itself).
- [x] Beat 4 -> 5 handler: `deemphasize("RADAR")` with GOTO; re-write
      OBJ_B5 to teach the combat lock ("hold [RMB] to raise your weapons,
      keep [CTRL] held to lock it - watch the viewfinder - then [LMB]"),
      so the inset/fine-lock/guided path becomes discoverable in the one
      fight the tutorial has.
- [x] Epilogue (OBJ_DONE): teach the stand-down - "Tap [CTRL] to clear
      your locks" (the staged clear + the safety click close the loop).
- [x] Update the pinned tests: emphasis-pairing (RADAR set/cleared pair on
      the same handlers as GOTO), the beat-walk (lock WITHHELD at boot
      with a delivery guard, granted at beat 4 - same shape as the
      governor pin), and any text assertions.
- [x] Scavenger-fires-under-gate: the chain is compositionally covered
      (AI cadence -> input; mirror -> WeaponsHot; the section gate's
      delivery guard pins hot-fires); pin the scenario-level link the
      tests can reach (the scavenger stays an AI ship with a turret -
      ships_are_minimal already pins the silhouette) and flag the in-game
      beat-5 fight as the user's playtest confirm.
- [x] Docs: CHANGELOG entry; fix-record line in spike 20260713-082207
      (this task is its last open consumer).
- [x] fmt + check; nova_assets test suite; 03_scenario autopilot (defer
      the live run if the user's game instance is still up - contention
      flake documented in 20260713-124000).

## Notes

- Spike: docs/spikes/20260713-082207-deliberate-radar-locking.md
  (adversarial round, tutorial findings) + 20260713-110039 (live lock).
- The minimal "must not lie" text fix landed in 20260713-082344; this
  task owns pedagogy and flow.
- Relevant files: nova_assets/src/scenario/shakedown.rs (+ its pinned
  scenario-flow tests).
- Beat structure unchanged (five beats + epilogue); this pass changes
  WHAT beats 4/5 teach, not the flow graph - beats 1-3 stay as-is.
