# Ledger beat-sheet pacing pass: ch1/ch2/ch2b (opening conversations, breathers, no objective dump)

- STATUS: OPEN
- PRIORITY: 56
- TAGS: v0.8.0, content, scenario

## Story

Apply the Shakedown beat-sheet pacing discipline to chapters 1, 2, and 2b:
clock-paced opening conversations, split the frame-0 objective dumps, breathers
between beats, one StoryMessage per handler, objectives posting a beat after
their intro line. Data-only; uses the v0.7.0 pacing toolbox (scenario_elapsed,
dwell, engage_delay, delay) - no new engine features.

Umbrella: 20260722-212808. Implements the ch1/ch2/ch2b findings from the
diagnostic pace-map (dep). Idiom reference: shakedown_run.content.ron
(beat_gate cascade, lines ~679-800) and the beat sheet in the dev wiki.

## Steps

- [ ] ch1: replace the OnStart dump (1 message + 3 objectives + all spawns)
      with a clock-paced opening (seed open_step/beat_gate; Okono briefs over
      ~scenario_elapsed gates; first objective lazy-posts + spawns its target on
      hand-off so a blind burn cannot skip it). One objective at a time,
      breathers between the quota pickups, one StoryMessage per handler.
- [ ] ch2 + ch2b: split the opening frame-0 objective dumps; add a short
      opening conversation beat; add a breather between the kills==2 victory and
      the next-scenario handoff; keep the engage_delay telegraphs. Do NOT move
      spawn geometry (the ch2 fairness rig pins ranges/bearings/cover).
- [ ] Add a clock-pumping test path where a walk test would stall on a deferred
      objective (lesson from 20260721-211506: time-gated content needs a clock
      pump or walk tests silently hang). Prefer sequencer counters over
      per-line one-shot flags.
- [ ] `content lint --target the-ledger` clean (beat-sheet arms: no >1
      StoryMessage/handler, no StoryMessage+Outcome co-fire); ack intended
      drama with reasons only.
- [ ] Probe the changed scenarios (lesson probe-content-not-just-code /
      review-rig-can-false-green: verify OnStart clock reads against the REAL
      loader/probe, not a synthetic rig - the loader fires OnStart before the
      first tick).

## Definition of Done

- ch1/ch2/ch2b open with a clock-paced conversation, no objective dump, and
  breathers between beats. (cmd: `content lint --target the-ledger` clean.)
- No objective posts at OnStart while a conversation plays; objectives post a
  beat after their intro line. (test: a walk/probe of each chapter reaches its
  first objective via the clock hand-off; probe shows 0 panics / 0 undefined
  scenario_elapsed reads.)
- ch2 fairness rig still green with any pins updated DELIBERATELY, not reactively.
  (cmd: `cargo test -p nova_assets --test ledger_ch2_encounter`.)

## Notes

Do not disturb ch2/ch2b spawn geometry - the pacing layer is additive (clock
gates + comms), not a geometry change. If a pin moves, justify it in the diff.
