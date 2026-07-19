# nova_probe: structured run-event logging + per-run timeline recorder (correctness capture - the crux)

- STATUS: CLOSED
- PRIORITY: 74
- TAGS: v0.8.0, spike, tooling, performance, testing

## Goal

Add structured run-event logging and a per-run timeline recorder: instrument the
game to emit a small structured `ProbeEvent` (timestamp, frame, kind, scenario-
variable snapshot) at the moments that matter, and a recorder plugin that
captures the ordered timeline of a headless autopilot run. This is the CRUX of
the whole tool (correctness capture) and the riskiest piece - de-risk early.

## Steps

- [x] Upstream unblock (DONE first, user-approved): bevy_common_systems
      v0.19.2 adds public read accessors `GameEvent::name()/info()` (the only
      observable event hook had pub(super) fields; bcs task 20260719-124137,
      commit 6eba8bf, pushed + tagged). Bump all five nova crates' bcs pins
      v0.19.1 -> v0.19.2 and refresh Cargo.lock.
- [x] Add `src/recorder.rs` to nova_probe: `TimelineEvent { t_real, frame,
      scenario_elapsed, kind, name, data }` (serde_json::Value payload),
      `ProbeTimeline` resource owning a JSONL sink (one JSON object per line,
      flushed per entry, so a panicked/backstopped run keeps everything up to
      the panic - panics are exactly when the timeline matters), and
      `nova_timeline()` preset plugin, env-gated on `NOVA_PERF_TIMELINE=<out
      path>` via perf_param (native only; wasm has no fs - plugin is inert
      there; the NOVA_PERF_* prefix is the T6-owned surface). run_start entry
      carries the RunMeta (sha/host reuse from capture.rs); run_end written on
      AppExit.
- [x] Hook state transitions: `MessageReader<StateTransitionEvent<GameStates>>`
      + same for PauseStates (verified: it is a Message with exited/entered,
      bevy_state-0.19.0/src/state/transitions.rs:67-75).
- [x] Hook scenario events: ONE observer on `On<GameEvent>` reading the new
      v0.19.2 `.name()` + `.info().data` (all 8 kinds + payloads, incl. future
      kinds, no per-site sweep; the queue is untouched - bcs pins that with
      observers_read_a_fired_events_name_and_payload).
- [x] Hook variable changes: PostUpdate snapshot-diff of NovaEventWorld
      variables excluding SCENARIO_ELAPSED_VAR (mirrors the write-on-diff in
      world.rs:86-105); emit per-variable old/new entries. Requires a public
      read iterator on NovaEventWorld (add `variables()` to
      crates/nova_scenario/src/world.rs - it has insert/get but no iteration).
- [x] Marker API: the `probe_marker` World helper (no-op when unarmed) so
      autopilot scripts/examples push their beats; wired 10_playable's seven
      beats (raise, combat sweep, fire, lowered, travel sweep, goto, done) as
      the worked example, and 08_scenario gained the bare plugin (its probes
      already log; it is the stability-probe subject) - both inert without
      the env, so no CI smoke impact.
- [x] Deps: nova_probe += serde_json (already in tree via bcs/nova_scenario)
      and nova_scenario; NOT nova_events - GameEvent comes via nova_gameplay's
      bcs re-export, keeping the bcs version unified (plan adapted). Root
      dev-deps += serde_json (marker payloads in examples).
- [x] Tests (would-it-fail-without-it): App-driven recorder rig (production
      plugin, real States + Messages init) proving ordered capture of a state
      transition + a fired GameEvent with payload + a variable change (old/new)
      + a marker into JSONL; a parse-back helper test; flush-per-entry pinned
      by reading the file BEFORE app exit; each hook's entries disappear if
      that hook is deleted.
- [x] Empirical stability probe (the spike's open question, feeds the golden
      task's entry gate): run 08_scenario headless twice with the recorder
      armed; diff the event kind+name sequences (not timestamps); RECORD the
      verdict here and in the spike fix record.
- [x] Docs: wiki development.md Performance section gains a "run timeline"
      paragraph; CHANGELOG Unreleased entry (incl. the bcs 0.19.2 bump).
- [x] Verify: fmt; cargo check --workspace --all-targets --features debug;
      cargo test -p nova_probe (+ the new nova_scenario accessor compiles
      everywhere); wasm check -p nova_probe; the 10_playable smoke run with
      recorder armed produces a sane timeline end to end.

## Notes

- Spike: tasks/20260719-112011/SPIKE.md.
- Emit at: GameStates transitions, scenario variable changes, the scenario
  event-handler signals (kill tally, travel-lock, arrival), autopilot script
  beats. Prefer reusing the scenario's existing event stream over a parallel one.
- Also the "improve in-game logging" the user asked for.
- Open question to resolve HERE empirically: how stable is the timeline run-to-
  run under llvmpipe throttling? Key the recorder on ordered event KINDS + var
  values with generous timing tolerance, not wall-clock, if timing is noisy.
- Depends on the crate skeleton (T1).

## Close-out (2026-07-19, branch feature/probe-run-recorder)

STABILITY VERDICT (the spike's open question; the deferred golden task
20260719-112245 gates on this): two recorded 08_scenario runs on this host
(Xvfb + real GPU) produced IDENTICAL meaningful sequences - same order, names
AND values across states, non-update scenario events (payloads included),
variable old/new changes and the run bracket (12 entries both runs). Only the
by-design per-frame `onupdate` pulse count varied (288 vs 282 - frame-rate
dependent); any comparer must exclude it, exactly as the recorder's own
variable diff excludes `scenario_elapsed`. Caveats for the golden task:
(a) same-host only - the cross-host (llvmpipe CI vs dev GPU) comparison is
still unmeasured; (b) the variable diff is per-FRAME, so same-frame
intermediate values collapse (observed: `beat` 1.0 -> 3.0 in one entry when
two script beats landed inside one diff window) - goldens must compare
final-per-frame values, not assume every write is visible.

End-to-end: a recorded 10_playable run (1221 entries, 25 meaningful) reads as
the intended story - run_start {git_sha, host}, state transitions, the seven
script beat markers interleaved with the scenario's own oncombatlock /
ondestroyed / ontravellock (payloads: which id, which ship), variable changes
(target_down 0->1, leg 0->1), the ~5 s lock-refire pulses, run_end {Success}.
The recorder immediately SURFACED a previously-invisible fact: an `onenter
{id: waypoint, other_id: prey}` fires at spawn (the prey asteroid overlaps
the waypoint's trigger area; harmless - the arrived filter requires
player_ship - but unknown until now). That is the tool doing its job.

Difficulties and how they resolved:

- The only observable event hook (`On<GameEvent>`) had pub(super) fields -
  an external observer could see events but read NOTHING. Resolved upstream
  (user-approved): bcs v0.19.2 adds read accessors, tagged + pushed, pins
  bumped. The alternative (a fire_probed wrapper swept over ~18 call sites)
  would have double-represented every fire site and silently missed future
  ones.
- Bevy's state-transition signal: verified `StateTransitionEvent<S>` is a
  Message BEFORE building (bevy_state source cited in Steps), so the reader
  systems carry `resource_exists::<Messages<...>>` guards
  (messagereader-needs-resource-guard) and the recorder is harmless on
  stateless apps.

Reflection: the fork-and-ask on the pub(super) blocker was the right call -
the user owns bcs, and the 4-line upstream fix beat both in-repo workarounds
on every axis except "no release needed". Verifying every hook in source
before writing Steps (the T1 lesson, applied again) meant zero compile
surprises: the whole recorder + tests went green on the first cargo test run.
