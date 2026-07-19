# Review: run-timeline recorder (correctness capture)

- TASK: 20260719-112238
- BRANCH: feature/probe-run-recorder

## Round 1

- VERDICT: APPROVE (two NITs recorded, neither blocking)

Shared-session caveat: implementer and reviewer are one session, so the
load-bearing claims were independently re-derived, not read off the diff:

- **"The dispatch queue is untouched"**: the recorder never names
  `GameEventQueue` anywhere (grep-verified); its observer reads only the
  event value. The non-starvation property is pinned UPSTREAM by bcs
  0.19.2's `observers_read_a_fired_events_name_and_payload` test (an extra
  observer present, queue still holds both events for handlers).
- **The upstream pin is real and public**: bcs v0.19.2 tag exists on
  GitHub (ls-remote verified at push time, 6eba8bf), all five nova pins
  bumped, Cargo.lock resolves to that exact rev - CI can fetch it.
- **Hook claims cited and checked in source**: StateTransitionEvent is a
  Message (bevy_state transitions.rs:67-75), RenderAdapterInfo pattern
  reused from T1, `Messages<...>` resource guards on both state readers
  (messagereader-needs-resource-guard), NovaEventWorld gained a read-only
  iterator (no mutation surface added).
- **Empirical evidence is from REAL runs, not the rig alone**: three
  headless app runs (Xvfb + GPU) - two 08_scenario runs whose meaningful
  sequences are byte-identical (order+names+values; only the by-design
  per-frame onupdate count differs, 288 vs 282), and one 10_playable run
  whose 25 meaningful entries read as the intended story with correct
  payloads (oncombatlock/ondestroyed/ontravellock ids, target_down 0->1,
  leg 0->1, seven beat markers in script order). The run_end self-count
  (1220) is consistent with the file (1221 lines incl. run_end itself).
- **Would-it-fail test audit**: the main rig asserts per-hook artifacts
  (state entry with entered=Playing, event entry with payload id=prey,
  variable entry with old/new, marker entry, run_start metadata) - deleting
  any single hook fails its assertion; the steady-state test pins the
  variable diff does NOT spam unchanged frames (3 idle updates, still
  exactly one appear entry); the flush pin reads the file BEFORE exit; the
  unarmed test proves the plugin is a no-op without env/override and that
  probe_marker is safe then. No existing test was weakened; 27 pass.
- **CI safety**: the smoke suite sets no NOVA_PERF env (grep-verified), so
  both newly-wired examples run the recorder as a no-op there; the wasm
  target compiles via same-signature stubs (checked).

Findings:

- R1.1 (NIT) crates/nova_probe/src/recorder.rs - `run_end.data.entries`
  counts entries written BEFORE the bracket entry itself (1220 of 1221).
  Self-consistent and documented by observation here; rename to
  `entries_before_end` only if T5's checks ever read it programmatically.
  Left as-is.
- R1.2 (NIT) - the armed recorder writes+flushes one JSONL line per
  onupdate pulse (per frame); harmless for a dev tool (armed runs only,
  ~1.2 KB/entry) but T5/T6 may want an option to fold the pulse count into
  run_end instead of streaming it. Noted for the report task; left as-is.

Deliberately NOT findings: the per-frame variable-diff granularity
(same-frame intermediate values collapse) and the same-host-only stability
scope are recorded honestly in the Close-out as constraints on the deferred
golden task - they are properties of the design the spike accepted, not
defects of this implementation.
