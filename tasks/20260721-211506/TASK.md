# Shakedown pacing pass: slower opening conversation, breathing gaps, simpler objective text

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.8.0,content,scenario,playtest

## Story

Playtest verdict (owner, 2026-07-21) on the shipped chain: the first
scenario "goes one after the other too fast" - beats fire back to back.
Wanted: drag the storyline at the start - a real opening conversation with
the Capt. BEFORE the first objective; between later beats, breathe (show
the objective, wait a bit, show the message); simpler tutorial objective
texts ("press W/Space to move" -> "go to the beacon" register); comms
lines between objectives.

This OVERRIDES the voice-pass design choice that left the tutorial
text-only (tasks/20260721-160929: "tutorial text untouched") - the owner
wants voice in the tutorial opening. Data-only content work: the v0.7.0
pacing toolbox (clock gates, dwell, arrival grace) covers all of it, so it
is v0.8.0-legal.

## Steps

- [ ] Pace map first: play/derive the current beat timings of shakedown_run
      (per-beat gaps) and write the target rhythm into this task before
      editing (diagnostic-first).
- [ ] Opening conversation per the owner's questionnaire answer (length,
      whether the player character is voiced) - clock-paced lines with
      dwell, gated BEFORE objective 1 posts; the speed-capped drift makes
      the wait diegetic.
- [ ] Insert breathing gaps between beat completion and the next objective
      across the tutorial (clock-gated one-shots; objective first, comms
      line after a beat, per the beat sheet).
- [ ] Simplify objective texts to the "press X to do Y" register; keep one
      gesture per beat (beat-sheet v2 rules hold).
- [ ] `content gen`; `content lint` (beat-sheet arms); parity + shakedown
      geometry tests; update the beat-sheet doc if the convention gains a
      "conversation opening" pattern.
- [ ] Probe/example: confirm the scripted walks still pass (added gaps may
      need stage deadline headroom); CHANGELOG + wiki tutorial page sweep.

## Definition of Done

- The opening holds a conversation before objective 1, and no two beats
  fire back to back without an authored gap
  (test: shakedown suite + a pacing pin on the gaps; manual: owner replay
  says the rush is gone).
- content lint clean (cmd: `cargo run -p nova_assets --bin content -- lint`).
- CHANGELOG + tutorial page synced (cmd: `grep -ni "shakedown" CHANGELOG.md`).

## Notes

- Owner decisions (questionnaire, 2026-07-21): PENDING - conversation
  shape/length, player voiced or silent.
- Spike: tasks/20260721-155249/SPIKE.md (chain design); voice conventions:
  tasks/20260721-160929/NOTES.md.
