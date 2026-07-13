# CHANGELOG entries for the v0.4.0 AI combat-behavior wave

- STATUS: CLOSED
- PRIORITY: 30
- TAGS: v0.4.0, ai, docs

The AI combat wave (spike tasks/20260709-225508/SPIKE.md,
tasks 20260709-225726..225734 as they land) has shipped several features
without CHANGELOG.md entries: behavior state machine, target selection,
fire discipline, point defense, standoff flight, patrol/idle states.
Add consolidated Added/Changed entries under [Unreleased] following the
existing @alexjercan entry style.

## Done (2026-07-10)

Diffed v0.3.1..HEAD (150 commits) against the existing [Unreleased]
section: everything up through the ScenarioLoaded smoke-harness work was
already recorded, so the gap was the run from PR #53 onward. Added:

- Added: audio/SFX system (#53), combat juice (#54), flight-assist
  overhaul (#55), HUD indicator substrate + lead pips + target readout,
  targeting/component-lock arc, faction/relation model, the nine-task AI
  combat wave consolidated into one entry, thrust balancing + off-axis
  counter-torque, torpedo launch burst, CI workflow.
- Changed: 20 km lock range, SfxListenerMarker.
- Fixed: live-structure anchor, section overkill clamp, disabled
  controller torque, shooter-frame bullet lead, AI slewed rotation
  command, torpedo body-death + deferred shot-down despawn, one hit =
  one cue dedup.

Entry text was written from the commit bodies (they carry good detail),
consolidating per feature arc rather than per commit. The AI wave got a
single long entry as the task asked, with each behavior named.

Reflection: the CHANGELOG had been kept current through mid-range
commits and then silently fell behind for ~50 commits. Consider adding
a changelog line to the /work or /compound checklist so entries land
with the feature instead of in a catch-up wave.
