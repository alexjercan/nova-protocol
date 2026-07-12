# Objective feedback: delay the new-objective cue after a completion

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.5.0,feel,audio

## Goal

Playtest round 4 (2026-07-12): "we can have a short timeout between
finishing and getting a new objective (like 1.0 sec) can be configured
maybe - helps with sounds". The completion chime and the new-objective
blip currently fire in the same frame (beat handlers complete + post in
one action list), so they mask each other.

## Steps

- [ ] Presentation-side delay in hud/objective_feedback.rs: when one
      GameObjectives change contains BOTH completions and additions, play
      the complete cue immediately and hold the new-objective cue in a
      pending timer; a tick system plays it when the timer finishes.
      Pure additions (no completion in the same change) stay immediate.
      A further change while a cue is pending refreshes the pending state
      (latest change wins; no stacking).
- [ ] Configurable: `ObjectiveFeedbackSettings { new_cue_delay_secs }`
      resource (Reflect, default 1.0), consumed by the timer.
- [ ] Tests via PlaySfx capture (observer counting bcs PlaySfx triggers,
      SoundBank loaded from the real NOVA_SFX_FILES): complete+add in
      one change -> exactly one cue immediately and the second only
      after the delay elapses (delivery guard: assert it has NOT played
      at delay/2); pure add -> immediate; teardown-empty -> still
      silent.
- [ ] CHANGELOG entry; check --tests --examples + fmt.

## Notes

- Scenario timing is untouched: the objective DATA still changes in the
  same frame (beat gating unaffected); only the audio presentation is
  deferred. The panel text swaps instantly - acceptable because the
  green ghost of the finished objective covers the reading gap.
- Follows: 20260712-125342 (round 3, CLOSED, landed 8bf4a99).
