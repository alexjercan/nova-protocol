# Review 18: Bench, channel, autopilot, and debug

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Specialist: proof and process follow-up
- Verdict: major and minor findings

## Findings

### MAJOR - `crates/nova_bench/src/agent/socket.rs:47-96` - A persistent cmd-agent connection bypasses the post-run grace deadline

`OVER_GRACE` is checked only in the outer accept loop. Once a connection is accepted, the inner read loop never checks it. Its timeout branch calls `check_deadline()` but discards the result. A cmd agent that leaves the connection open can therefore keep `serve()` from returning after the run is over, so score/report writing and bounded child termination never run.

This defeats the documented backstop for a wedged external agent and can hang `bench play` indefinitely. No external wrapper timeout closes the gap. The proof is static; the existing socket fixture can reproduce it without launching the game, but it was not run.

Why not BLOCKER: this affects the development bench's external `cmd:` transport, not shipped gameplay or the common baseline path.

### MAJOR - `crates/nova_bench/src/pages/targeting.md:22-26` - The agent manual gives a stale lock-dwell curve

The page claims 0.645 seconds at 1 km, 0.7125 seconds at 2.5 km, and saturation at 20 km. Current defaults and `lock_dwell_secs` produce 1.05 seconds at 1 km and 1.5 seconds at 2.5 km, saturating at 2 km. The page's worked 54-tick hold at 1 km is shorter than the actual 63-tick requirement.

This directly causes an agent following the official manual to release before lock acquisition. Existing ship tests pin the live formula; the defect is the bench contract.

### MAJOR - `crates/nova_bench/src/gesture.rs:136-148` - Shared-key validation ignores the held state from earlier acts

`check_shared_keys` checks only gestures in the current act. If one logical action remains held from an earlier act and a later act taps its shared-key counterpart, validation passes. The tap then releases the same physical input source while the referee still records the first action as held.

This creates divergence between referee state and game input state. The manual tells agents to release first, but the guard exists to reject invalid agent gestures and fails across acts. A two-act referee fixture can prove the mismatch without a game run; it was not executed.

### MINOR - `crates/nova_bench/src/movie/rail.rs:217-227` - Rail parsing silently drops malformed middle events

The documentation permits ignoring a truncated final line. The implementation uses `filter_map` for every line, silently dropping any unparsable event anywhere in the audit rail while continuing with later events. Current single-writer serialization makes this unlikely, and rail data affects movie overlays rather than scoring.

## Adjudication

No additional finding survived in `nova_channel`, `nova_autopilot`, or `nova_debug`. Process feature gates, recorded-PID cleanup, channel application order, snapshot ownership, audit/score separation, completion collection, and debug plugin composition were checked.

## Coverage

Fully read every source, test, example, manual page, and crate manifest in `nova_bench`, `nova_autopilot`, `nova_channel`, and `nova_debug`. The proof specialist traced socket loops and child cleanup, current targeting math, cross-act held/source semantics through `nova_channel` and `nova_input`, and rail write/read ownership.

Not checked: no Cargo command, bench play, game, probe, GPU process, wasm build, workspace test, or Clippy run. The four proposed focused non-game reproductions were not executed. Bench scenario RON and the TypeScript pi relay were not fully reviewed in this batch.
