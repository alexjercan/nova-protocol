---
name: probe
description: Verify a change with the nova_probe run-harness - one command runs an autopilot example and produces a reviewable correctness+perf report (report.html + checks.json) with an OK/WARN/FAIL verdict. Use as the post-feature check in /work's verify step for gameplay-touching changes, for before/after evidence on bug and perf tasks, and whenever the user asks to "probe" a run or wants a run report.
---

# Probe - the Run-Harness Check

`nova_probe` runs an autopilot example headless and answers two questions in
one artifact: did the run behave correctly, and what did it cost? It is the
post-feature check of this repo's SDLC: after implementing a change that
touches gameplay, probe the affected example(s) and read the report before
calling the work verified. Built by the 20260719-112011 spike family; design
rationale lives in `tasks/20260719-112011/SPIKE.md`, user-facing docs in the
wiki's Performance section (`web/src/wiki/dev/development.md`).

## Commands

```sh
cargo run -p nova_probe -- run <example>              # clean pass -> report
cargo run -p nova_probe -- run <example> --profile    # + traced pass (top-N systems)
cargo run -p nova_probe -- run <example> --samply     # + named flamegraph
cargo run -p nova_probe -- run <example> --fps        # + frame-time capture (wired examples)
cargo run -p nova_probe -- run <example> --baseline <old-run-dir>   # FPS deltas
cargo run -p nova_probe -- sweep [gpu|sw] [out]       # frame-time sweep (perf-baseline.sh)
cargo run -p nova_probe -- web [scenario]             # web/WebGPU capture (perf-web.sh)
cargo run -p nova_probe -- profile [example] [out]    # deep profile (perf-profile.sh)
```

Also standalone: `--bin run_report -- <run-dir> [--baseline <dir>]` re-renders
a report over any run dir (a sweep out-dir qualifies), `--bin perf_report` for
FPS-only results dirs, `--bin perf_trace` for a chrome-trace top-N table.

`run` writes to `probe-runs/<example>/` (gitignored): `timeline.jsonl`,
`run.log`, `report.html`, `checks.json`, plus `trace.json`/`trace-run.log`
(--profile), `samply-profile.json.gz` (--samply), `frametime.csv` (--fps on a
wired example). Exit code mirrors the verdict (FAIL = 1). It spawns its own
Xvfb (pid-derived display; `--display :0` to reuse one) and times out hung
runs (`--timeout <s>`, default 180).

## Reading the verdict (the honesty rules)

- `checks.json` is the agent-readable mirror - read it instead of parsing
  HTML, and read `verdict` TOGETHER WITH `measured` ("n/total"), never the
  verdict alone. Checks: `process_exit` (the child's real outcome from the
  probe-run.json manifest; a timeout is a FAIL), `run_completed` (a
  TRUNCATED timeline means the run died - entries are flushed as written -
  and the bracket's entry count must match the file), `reached_playing`
  (the smoke contract), `invariants_held` (violations counted per name;
  one stuck entity violates every frame), `fps_within_baseline` (soft
  gate; only REGRESSIONS beyond the threshold WARN - improvements PASS),
  `log_clean` (ANSI-stripped, whole-word ERROR). Each check carries a
  structured `data` object; the top-level `run` object is the manifest.
- SKIPPED means NOT MEASURED, never "held". Zero measured checks =
  verdict NO_DATA and a nonzero exit. An OK on an UNWIRED example is
  OK-with-coverage: it proves the example's own assertions (exit status)
  only - gameplay-verification claims require `run_completed` and
  `invariants_held` MEASURED, and the skip details say when an example
  simply is not wired.
- The profile table RANKS systems; shares overlap (parent and child spans
  both count) so they are never summed, and traced-run numbers never compare
  against the clean pass.
- The tool's verdict is PROVISIONAL. The reviewer (you, in /review) owns the
  final OK/NOT-OK, via the checklist at the bottom of report.html.

## Where it plugs into the SDLC

- **/work, verify step**: for a change touching gameplay/scenario/flight/
  sections, `probe run` the affected example(s) after tests pass. Record the
  invocation and verdict in TASK.md's close-out. A FAIL is a finding, not an
  inconvenience - read the timeline around the failing frame.
- **Bug tasks (reproduce first)**: probe the reporting scenario BEFORE the
  fix - the timeline is the diagnosis evidence (states, events with
  payloads, variable old/new around the failure). Keep the pre-fix run dir;
  probe again after; cite both in TASK.md. Strict invariants
  (`NOVA_PERF_INVARIANTS=strict` on a manual run) panic at the moment of
  corruption when you need the exact frame.
- **Perf tasks (measure first)**: `sweep` before and after the change into
  separate dirs, then `run_report -- <after> --baseline <before>` for the
  delta table. Use `--profile` (or the `profile` subcommand + samply) to
  RANK suspects before optimizing anything; the ledger's perf lessons
  (quiet-host-before-measuring, isolate-the-lever) still apply.
- **/review**: when the implementer cites a probe verdict, open checks.json
  and read `measured` first, then the SKIPPED rows - what was NOT measured
  is the first thing to challenge. For perf claims, confirm same-label baselines and a quiet host.
- **New examples**: wiring is one inert line each -
  `app.add_plugins(nova_probe::nova_timeline())` and
  `nova_probe::nova_invariants().monotonic([...])` (only variables the
  scenario DESIGN promises one-way), plus `probe_marker` calls at script
  beats. FPS capture (`nova_frametime()`) only belongs on measurement
  scenes, not scripted correctness runs.

## Wired today

| Example | timeline + invariants | fps capture |
|---|---|---|
| 08_scenario | yes (monotonic: beat, rocks_destroyed) | no |
| 10_playable | yes (monotonic: target_down, leg) + 7 beat markers | no |
| 20_perf_baseline | no | yes (`--fps`) |
| others | SKIPPED until wired | no |

## Host knobs (flamegraphs)

samply needs `perf_event_paranoid <= 1` and, on many-core hosts, a raised
`perf_event_mlock_kb` (e.g. 16384). Load profiles with the URL `samply load`
prints (drag-dropping the file loses the local symbol server = hex frames);
driver-blob/libc frames stay hex regardless - judge by our modules' frames.
