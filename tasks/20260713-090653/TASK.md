# Shakedown scenario rework for the radar era: teach the radar, text pass, lock capability beat

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.5.0, scenario, tutorial, polish

## Goal

Once the deliberate-radar family lands (20260713-082324/-082330/-082337), the
Shakedown Run needs a rework/polish pass (user request 2026-07-13): the
tutorial currently teaches the dead passive lock and never teaches combat
locking at all - beat 5 is pure manual gunnery, so focus/inset/component
fine-lock/guided torpedoes are undiscoverable (adversarial round, UX finding).

Scope when picked up (/plan then):

- Full objective-text pass against the radar gestures (the minimal
  "must not lie" correctness fix lands earlier in 20260713-082344; this task
  owns the pedagogy and flow).
- A teach-the-radar beat: designate a beacon via hold-CTRL radar for the GOTO
  leg (travel lock), and a combat-lock moment before/during the scavenger
  fight (raise, radar, watch the inset/fine-lock come alive) - possibly using
  the lock CAPABILITY as a tutorial beat (the computer "comes online" like the
  speed-governor release).
- General playtest polish of beat pacing under the new input model
  (safety-on/off moments, staged tap-clear discoverability, toasts/hints).

## Notes

- Spike: docs/spikes/20260713-082207-deliberate-radar-locking.md (adversarial
  round, tutorial findings).
- Depends on: 20260713-082337 (family landed); coordinate with 20260713-082344
  (docs/text minimal pass) to avoid double-editing the same strings.
- Relevant files: nova_assets/src/scenario/shakedown.rs (+ its pinned
  scenario-flow tests), keybind hints.
