# Shakedown pacing pass: slower opening conversation, breathing gaps, simpler objective text

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.8.0, content, scenario, playtest

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

- [x] Pace map first: play/derive the current beat timings of shakedown_run
      (per-beat gaps) and write the target rhythm into this task before
      editing (diagnostic-first).
- [x] Opening conversation: player-voiced back-and-forth with the Capt.,
      5-6 clock-paced lines with dwell, gated BEFORE objective 1 posts;
      the speed-capped drift makes the wait diegetic (owner decision
      below; speaker-label proposal to the owner in-task).
- [x] Insert breathing gaps between beat completion and the next objective
      across the tutorial (clock-gated one-shots; objective first, comms
      line after a beat, per the beat sheet).
- [x] Simplify objective texts to the "press X to do Y" register; keep one
      gesture per beat (beat-sheet v2 rules hold).
- [x] `content gen`; `content lint` (beat-sheet arms); parity + shakedown
      geometry tests; update the beat-sheet doc if the convention gains a
      "conversation opening" pattern.
- [x] Probe/example: confirm the scripted walks still pass (added gaps may
      need stage deadline headroom); CHANGELOG + wiki tutorial page sweep.

## Pace map + fix (2026-07-21)

Diagnosis (diagnostic-first): every beat was POSITION-gated (OnEnter arrival /
OnDestroyed / OnOrbit) and posted the next objective + chip the instant the
gesture landed - there were ZERO authored pauses. `OnStart` dropped straight
onto objective 1. So the rush the owner felt is structural: beat N completes
-> beat N+1 objective + marker + spawn, same frame, for all 12 beats. There was
no scenario-clock pacing in shakedown at all (clock gates were a Lifeline/Final
Tally tool only).

Target rhythm (owner decisions): a ~40s opening conversation before objective 1,
then "objective -> fly a beat -> comms line" for each navigation beat, with the
climactic fight kept tight (the fight is the exam).

Implementation (data-only, existing pacing toolbox):
- Opening: `OnStart` now posts a "stand by" holding objective and seeds an
  `open_step` counter; five `scenario_elapsed`-gated `OnUpdate` lines
  (Capt. Halloran x3, You x2) play over ~40s; an `open_step == 5` hand-off
  posts objective 1, spawns + marks beacon 1, and stamps the breather gate.
  Beacon 1 is now LAZY (like 2-4), so the pre-objective drift cannot skip it.
  The 25 u/s speed cap makes the wait diegetic. First player voice = "You"
  (owner decision; single `cast::PLAYER` constant).
- Breathers: each navigation transition stamps `beat_gate = scenario_elapsed`;
  a `breather(beat)` handler fires one comms line `beat_gate + 4s` later
  (beats 2-10). Combat exam (11-12) stays tight; the one fight announces itself
  with a telegraph line (beat-sheet lead-in).
- Objective texts shrank to the goal register ("Burn to Beacon 1.", "Find
  Beacon 2 - hold [Alt] to look around.", "Lock onto Beacon 3 - hold [CTRL].",
  ...); the flavor moved into Halloran's voice.

Tests: 15 shakedown unit tests pass, incl. a new
`the_opening_converses_before_objective_one_and_beats_breathe` pin (opening
deferral + voice + gate stamps) and the walk test now pumps the clock through
the opening (new `set_clock`/`finish_opening` helpers). content gen +
content lint clean (0 errors). Probe: `menu_newgame` (real shakedown boot flow)
run to confirm the opening handlers do not crash the real loader.

Manual (owner replay): confirm the rush is gone and the opening reads well.

## Definition of Done

- The opening holds a conversation before objective 1, and no two beats
  fire back to back without an authored gap
  (test: shakedown suite + a pacing pin on the gaps; manual: owner replay
  says the rush is gone).
- content lint clean (cmd: `cargo run -p nova_assets --bin content -- lint`).
- CHANGELOG + tutorial page synced (cmd: `grep -ni "shakedown" CHANGELOG.md`).

## Notes

- Owner decisions (questionnaire, 2026-07-21): PLAYER VOICED - a real
  back-and-forth with the Capt., 5-6 clock-paced lines (~40s) before
  objective 1. This is the campaign's FIRST player voice; the tone carries
  into later chapters, so write the player terse and professional (the
  belt register) and keep lines reusable.
- OPEN (decide at /work with a proposal to the owner): the player's
  speaker label - a callsign vs a plain "You". The Ledger mod uses
  "Kestrel" for its player; the base campaign has no established callsign
  yet. Single constant in cast.rs either way.
- Spike: tasks/20260721-155249/SPIKE.md (chain design); voice conventions:
  tasks/20260721-160929/NOTES.md.
