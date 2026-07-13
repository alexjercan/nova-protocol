# Shakedown scenario rework for the radar era: teach the radar, text pass, lock capability beat

- STATUS: IN_PROGRESS
- PRIORITY: 45
- TAGS: v0.5.0, scenario, tutorial, polish

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

- [ ] Withhold the Lock capability at spawn: `player_ship`'s
      clone-and-override controller sets `verbs.lock = false` next to the
      existing `goto = false` (shakedown.rs:285-299). Pre-grant, CTRL
      buzzes + flashes (the deny cue) - diegetically "no targeting
      computer yet".
- [ ] Beat 3 -> 4 handler: grant `FlightVerb::Lock` via SetControllerVerb
      (next to the beacon-3 spawn), re-write OBJ_B4 for live-lock +
      the capability moment ("Targeting computer online. Hold [CTRL] and
      put your eyes on BEACON 3 - the white lock sticks when you release.
      Then [G] ..."), and `emphasize("RADAR")` alongside the existing
      GOTO emphasis (the contextual cluster shows the emphasized GOTO row
      early, dim + gold, and lights it when the lock lands - the
      progression is visible in the cluster itself).
- [ ] Beat 4 -> 5 handler: `deemphasize("RADAR")` with GOTO; re-write
      OBJ_B5 to teach the combat lock ("hold [RMB] to raise your weapons,
      keep [CTRL] held to lock it - watch the viewfinder - then [LMB]"),
      so the inset/fine-lock/guided path becomes discoverable in the one
      fight the tutorial has.
- [ ] Epilogue (OBJ_DONE): teach the stand-down - "Tap [CTRL] to clear
      your locks" (the staged clear + the safety click close the loop).
- [ ] Update the pinned tests: emphasis-pairing (RADAR set/cleared pair on
      the same handlers as GOTO), the beat-walk (lock WITHHELD at boot
      with a delivery guard, granted at beat 4 - same shape as the
      governor pin), and any text assertions.
- [ ] Scavenger-fires-under-gate: the chain is compositionally covered
      (AI cadence -> input; mirror -> WeaponsHot; the section gate's
      delivery guard pins hot-fires); pin the scenario-level link the
      tests can reach (the scavenger stays an AI ship with a turret -
      ships_are_minimal already pins the silhouette) and flag the in-game
      beat-5 fight as the user's playtest confirm.
- [ ] Docs: CHANGELOG entry; fix-record line in spike 20260713-082207
      (this task is its last open consumer).
- [ ] fmt + check; nova_assets test suite; 03_scenario autopilot (defer
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
