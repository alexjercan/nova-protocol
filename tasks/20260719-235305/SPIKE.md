# SPIKE: harness exit coordination - who gets to end the app, and when

- TASK: 20260719-235305
- DATE: 2026-07-19
- QUESTION: the fleet's exit logic races two uncoordinated clocks
  (wall-seconds vs frame-counts) and resolves ownership by per-example
  folklore. What is the right architecture so evidence collectors always
  get their data - including the user's instinct: "repeat the scene if it
  finishes too early"?

## Review: how exiting works today (and why it reads hacky)

Four actors can end (or refuse to end) an app, each on its own clock:

| Actor | Clock | Exit behavior |
|---|---|---|
| bcs `AutopilotPlugin` | wall seconds | writes `AppExit::Success` after the last `(state, seconds)` hold (autopilot.rs:192); `nova_autopilot()` = one 6.0s hold |
| self-ending scripts (broadside) | script stages | long runway hold (50s), the script exits itself at its final stage; a guard PANICS if the app exits with the script unfinished |
| frame capture (`nova_frametime`) | frame counts | writes `AppExit::Success` after warmup+window - but only OWNS the exit on perf_baseline (a `!perf_armed()` conditional adds/removes the autopilot) |
| probe supervisor | outer wall clock | SIGKILL at `--timeout` (180s) - the backstop |

What is GOOD and must survive any redesign:
- env-gated inertness (every collector costs nothing un-armed);
- assertions co-located with scripts, firing before the exit;
- the self-ending guard pattern (a stalled script cannot pass vacuously);
- probe's outer timeout as the last word.

What is HACKY (the user is right):
- Exit ownership is FOLKLORE: three patterns plus one conditional,
  chosen per example, with nothing enforcing exclusivity - the
  perf_baseline fix (task 20260719-210443) and the all-or-nothing capture
  loss (task 20260719-233732: scenario missed its window by ELEVEN
  frames and discarded 229 samples) are both symptoms of the same
  missing protocol.
- Two units race: holds are SECONDS, capture windows are FRAMES, and the
  conversion rate (fps) is unknowable per host/build. No constant fixes
  it.
- Assertions hang off a magic constant (`elapsed > NOVA_AUTOPILOT_SECS -
  0.5`): change the lifetime, silently move every assertion.
- A finished scene under a still-open capture measures IDLE frames -
  even "hold the app open longer" quietly changes WHAT is measured.

## Design space

### D1. Completion protocol: collectors negotiate the exit (RECOMMENDED)

Upstream (bcs harness, the established path - v0.19.2 shipped this way):
a tiny coordination resource. Every armed collector REGISTERS at build
("autopilot", "capture", "screenshot"); each reports DONE when its own
clock completes; the harness writes `AppExit::Success` when the pending
set empties. A generous in-app deadline (default well under probe's
timeout) force-exits with the laggards NAMED - which probe surfaces as a
FAIL detail instead of a silent SIGKILL.

- The autopilot stops writing AppExit; its timeline completion becomes
  `done("autopilot")`. Self-ending scripts call `done` at their final
  stage - the completion guard becomes protocol-level and uniform.
- The capture stops writing AppExit; window completion becomes
  `done("capture")`.
- perf_baseline's `!perf_armed()` conditional DELETES - both collectors
  register, the exit happens when both finish. Exclusive-exit-ownership
  (the 210443 lesson) is superseded by no-unilateral-exit.
- Assertions move off the magic constant: fire on the autopilot's OWN
  completion event instead of `elapsed > SECS - 0.5`.

### D2. Scene looping for measurement windows (the user's instinct)

D1 alone leaves the idle-tail problem: a 6s script under a 10s capture
measures 4s of nothing happening. The benchmarking-honest fix is to
REPEAT THE WORKLOAD: when the script completes and the capture is still
pending, restart the scene (re-trigger `LoadScenario`, reset script
state) and keep sampling until the window closes.

- Opt-in per example (`nova_autopilot().loop_while_pending()` or
  similar): only scenes that reload cleanly enroll; the reload hitch
  frames are REAL cost and stay in the tail (or get marked - decide at
  implementation with data).
- CRITICAL interplay: looping resets design-promised monotonics (`beat`
  restarts) and re-brackets the timeline - so a LOOPED capture must run
  capture-only, recorder/invariants disarmed. The architecture already
  has this exact precedent: sweep cells disarm the recorder surfaces
  because "cells measure frames". Generalize it: when probe's `--fps`
  needs looping, the fps capture becomes its OWN pass (like `--profile`
  already is), and the clean pass stays correctness-only. Two-pass rule,
  applied to measurement instead of tracing overhead.

### D3. Partial-window emit (task 20260719-233732) - demoted to safety net

With D1+D2 the common case never loses samples. Partial-emit survives as
the LAST-RESORT honesty net (deadline hit mid-window emits what exists,
marked) plus the diagnostic skip messages ("exited at 43/240 captured").
The 233732 task re-scopes after this spike: part 1 shrinks to the net +
diagnostics; part 2 (category window defaults) stays as pure ergonomics.

### Rejected

- Generalizing capture-owns-exit (per-example conditionals fleet-wide):
  more folklore, breaks self-ending guards, measures idle tails.
- Time-based capture windows: redefines the unit all baselines use.
- Bigger constants / per-example knob tables: no constant fits every
  host; a knob table is configuration where a protocol is needed.

## Recommendation and cuts

1. **S1 (bcs upstream + nova adoption)**: the completion protocol -
   register/done/deadline in bcs harness; autopilot + capture +
   screenshot converted; nova examples recompile unchanged except
   perf_baseline (conditional deleted) and broadside (guard becomes
   protocol-level). Release bcs, bump the pin.
2. **S2 (probe + examples)**: looped measurement pass - `--fps` runs
   capture-only as its own pass when looping is enrolled; gameplay/
   scenes opt in to `loop_while_pending`; e2e proves `probe run gameplay
   --fps` yields three FULL windows with zero env knobs.
3. **233732 re-scope**: partial-emit as the deadline net + skip
   diagnostics.

Order: S1 -> S2 -> 233732. S1 is the load-bearing change and is
upstream-first (the bcs release rhythm is proven).

## Adjudications (user input 2026-07-20) and answers

1. **Loop enrollment: PER EXAMPLE (opt-in).** Settled - reload
   cleanliness is a per-scene fact.

2. **Reload frames: EXCLUDE from the scene stats, REPORT separately.**
   The user's tension is real ("not part of the scene, but scene-loading
   is still part of the game") and the resolution is that these are TWO
   measurements, not one:
   - The fps row answers "what does this SCENE cost per frame" - and its
     KEY property is comparability (baseline deltas). How many reloads
     land inside a window depends on host speed, so including them makes
     two runs' distributions non-comparable garbage: the delta would
     measure reload COUNT, not scene cost.
   - Scene-loading cost deserves its own visible number, not a smear in
     someone else's tail: the looped capture marks reload intervals and
     the report emits a reload line per looped row ("3 reloads: mean
     210ms, max 320ms"). Excluding-and-reporting serves "loading needs
     checking" BETTER than inclusion - the cost becomes readable instead
     of buried. If load hitches ever need gating, that line is the
     natural seed for a dedicated check.

3. **fps pass: ALWAYS SPLIT.** The user leaned this way unsure; the
   review makes it clear-cut, for a reason stronger than uniformity:
   the clean pass arms the run RECORDER, which flushes JSONL PER ENTRY
   on the frame path (state transitions, events, variable diffs - the
   scenario's onupdate pulse fires every frame). Today's fps-on-clean-
   pass numbers are contaminated by recorder I/O - a T2-era honesty gap
   this spike's review caught (the dev badge kept them non-baselines,
   but the two-pass principle was already violated in spirit). Always
   splitting: measurement passes (fps, trace, samply) are ALWAYS
   separate from the correctness pass; the fps pass is capture-only, so
   looping never touches monotonics/timelines AT ALL (D2's interplay
   paragraph dissolves); and one command means one thing on every
   example instead of a hidden mode switch when a script happens to
   outlast a window.

## Review round (honest adversarial pass on this spike)

- **R1 (design gap, now fixed in S1's scope): assertion-before-done
  ordering.** "Assertions move off the magic constant" was understated:
  in-example asserts currently fire at SECS-0.5 so a panic propagates
  BEFORE AppExit. Firing them "on completion" must be ordered - the
  autopilot runs its final assert hook, THEN reports done - or a failing
  assert races the negotiated exit. S1 must make this an explicit,
  tested part of the protocol, and the conversion touches every
  example's assert timing (mechanical but fleet-wide).
- **R2 (upstream blast radius)**: S1 changes bcs AutopilotPlugin's exit
  semantics - bcs's OWN examples/tests and any non-nova consumers must
  be checked in the upstream cycle, not assumed.
- **R3 (deadline arithmetic)**: the in-app deadline must resolve BELOW
  probe's --timeout so the named-laggards report always beats the
  SIGKILL; the default needs to be derived (e.g. min(own default,
  supervisor budget)), not two independent constants.
- **R4 (self-correction)**: the fps-on-clean-pass contamination (see
  adjudication 3) was shipped by T2 with this session's own review
  calling it honest. The always-split decision retroactively fixes it;
  the lesson is that "inert without env" reasoning checked the UNARMED
  cost but never the ARMED cross-talk between collectors.
- **R5 (what partial-emit still covers)**: with looping opt-in and
  splitting universal, the net catches exactly: non-enrolled examples
  whose window does not fit, and deadline hits mid-window. Still worth
  having; still demoted (233732).

## Revised cuts

1. **S1 (bcs upstream + adoption)**: completion protocol
   (register/done/deadline-naming-laggards), assert-then-done ordering
   explicit, autopilot/capture/screenshot converted, bcs's own consumers
   checked; release + pin bump; perf_baseline conditional deleted,
   broadside guard becomes protocol-level.
2. **S2 (probe + examples)**: `--fps` ALWAYS a dedicated capture-only
   pass (manifest records it; fps check reads the same artifacts);
   loop_while_pending opt-in on gameplay/ scenes; reload intervals
   marked, excluded from scene stats, reported as their own line. E2E:
   `probe run gameplay --fps` -> three FULL windows, zero env knobs,
   reload lines on looped rows.
3. **233732 re-scope**: partial-emit as the deadline net + skip
   diagnostics (unchanged from the earlier note).
