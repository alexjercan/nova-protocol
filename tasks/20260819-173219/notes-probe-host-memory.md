# The probe host could OOM a 31 GB box on one small range

Two instrument defects, both found while measuring this task's baselines. Both
made a run report something other than what it measured.

## 1. The host held the whole trace, twice

### Cause

`RunArtifacts::load` read every artifact with `std::fs::read_to_string`, and
`aggregate_system_costs` then built a whole-file `serde_json::Value` over that
string. The text and the parse tree were resident together, and nothing anywhere
checked a size.

The trace is not small. The profiled pass sets `TRACE_CHROME` and
`RUST_LOG=bevy_ecs=info`, which emits a span per system per frame; measured on a
real run dir that is about 28 MB per second of traced gameplay, and the
supervisor gives the traced pass double the normal timeout (360 s by default).
`probe run system_torpedo_launch` wrote a `trace.json` of 7,348,417,430 bytes and
took the host to 27.8 GB RSS on a 31 GB machine. `--correctness-only` avoided it
because it skips the profiled pass entirely.

Measured cost of the old path, on real traces from earlier run dirs:

| trace.json | peak host RSS | ratio |
| --- | --- | --- |
| 702 MB | 3.06 GiB | 4.6x |
| 2.60 GB | 11.06 GiB | 4.5x |

At 4.5x, the observed 7.35 GB file wanted about 33 GB. The box has 31.

### Fix

`aggregate_system_costs` takes a `Read` instead of a `&str` and folds the
top-level array element by element through a `DeserializeSeed`, so exactly one
event is alive at a time. `RunArtifacts` grew `Loader::stream`, which hands the
trace over as an open `File`; every other artifact still goes through `read`,
because every other artifact is kilobytes.

serde is now a direct dependency of `nova_probe_cli` (the derived event struct
and the seed both live in serde proper, not serde_json).

### Before and after

Same binary, same run dirs, `probe report <dir>` under `time -v`, peak RSS:

| trace.json | before | after | wall (after) |
| --- | --- | --- | --- |
| 702 MB | 3,212,972 KB | 74,076 KB | 3.7 s |
| 2.60 GB | 11,594,328 KB | 71,888 KB | 13.5 s |
| 4.95 GB | not attempted (wants ~22 GiB) | 71,144 KB | 25.8 s |
| 8.73 GB | not attempted (wants ~39 GiB) | 73,656 KB | 63 s |

Peak memory is now flat at about 70 MB and independent of the file. Throughput
is 140-190 MB/s, which is not slower than the old path either (13.5 s against
13.79 s on the 2.6 GB file).

The 8.73 GB row is a live reproduction, not an archive: `probe run
system_torpedo_launch --norender` on this branch wrote that trace and then read
it. The box's available memory never went below 19.6 GB during the whole run,
and the largest resident process throughout was an unrelated rust-analyzer at
3.7 GB. The same run before the fix is what took the host to 27.8 GB.

### The report did not get worse

The streaming aggregate is EXACT, not an approximation, and there is a test that
says so: `the_streamed_aggregate_equals_the_whole_file_dom_aggregate` keeps the
old whole-file DOM implementation as a reference and asserts full equality of
the `Vec<SystemCost>` - every row, call count, millisecond and share - on both
the hand-written fixture and a generated trace with the shape a real one has.
If the two ever disagree, that test fails.

Two things in the report changed, both additive:

- The profile section states the trace's size.
- A truncated trace no longer loses its whole table. Rejecting a file that ends
  mid-array was deliberate before, but it threw away a converged ranking for a
  traced child the supervisor had killed on timeout - which is the normal way a
  long traced pass ends. It now aggregates the prefix and the report carries a
  warning banner saying the table covers a prefix and that a system running only
  late in the scene is missing from it. A file that never opened an array, an
  empty file, and a file with trailing junk are all still rejected loudly; the
  difference is read off `serde_json::Error::classify`, so a type mismatch is an
  error and not a silent truncation.

### The guard, and why this one

Streaming makes the file's SIZE structurally irrelevant, which is the guard that
cannot regress: there is no threshold to tune and no data dropped. A refusal
above some byte count would have been the weaker choice - it drops the profile
section for exactly the runs that are most expensive to reproduce, and the
number would have to be guessed.

What streaming does not bound is content-shaped growth, so the two maps that
outlive an event are capped and REFUSE rather than allocate:
`MAX_OPEN_SPANS_PER_THREAD` (4096) for a file whose `E` events are missing, and
`MAX_TRACKED_SPANS` (65,536) for generated span names. Neither can trip on a
trace bevy wrote - real schedule nesting is single digits and a binary has a
fixed system count - which is the point: they exist so that a malformed file
fails with a message instead of growing.

### The writer: deliberately unchanged

A 7.35 GB trace SHOULD exist, and this is a judgement, so here is the reasoning.

The size is inherent, not redundancy. It is one span per system per frame over a
run that lasts minutes. Measured composition of a real 702 MB trace: 47% is the
`system` / `system_commands` spans the table reads, 22% `check_conditions`, 19%
`par_for_each`, 9.5% the multithreaded executor, 1% schedules.

Both write-time options cost something real:

- A byte cap in the supervisor bounds disk by killing the traced child. That
  truncates the raw file, and the raw file's whole job is the Perfetto deep
  dive - it would cut the record at whatever moment the run got interesting,
  which is precisely when someone is profiling. It also needs a new pass outcome
  threaded through the manifest to stay honest about why the child died.
- Filtering `bevy_ecs::query::state` and the single-threaded executor out of
  `RUST_LOG` saves 41% losslessly for the table. But 41% of an unbounded
  quantity is a discount, not a bound - 7.35 GB becomes 4.3 GB - and it removes
  the nesting a Perfetto session reads.

The failure was never "the file is big". It was "the reader was O(file)", and
that is now fixed on the side that had the problem. What the writer did owe was
VISIBILITY, so the report prints the trace's size beside the table and
`docs/performance.md` says to expect gigabytes and to delete the run dir when
you are done profiling.

## 2. wfc_arena measured the aftermath of the fight

### Cause

`wfc_arena` gates its capture on `Scoreboard::fight_happened` - both teams have
fired AND both have dealt damage. But `track_damage` credits a whole-team WIPE
as damage dealt, so the frame that satisfies the gate can be the frame the fight
is decided. The window then opens on a near-empty arena: runs came back at 2-3 ms
mean, worst frame under 8 ms, 1% low 140-175 fps. In one measurement set four
such runs landed in one arm and none in the other, which made a real fix look
slower.

Nothing existing could catch it. `ABORT_SIMULATION_STOPPED` reads
`Time<Virtual>`, and the clock is still running - the result screen pauses it a
second or two later, long after the window opened. Every environment gate passes.
The scene is over and the instrument cannot tell.

### Fix

A liveness half to match the readiness half, following the refusal pattern
rather than warning:

- `PerfLive` / `FrameTimePlugin::live_while` / `NovaProbePlugin::live_frametime`:
  a `Fn(&World) -> bool` re-evaluated every frame by `perf_watch_live`, which
  mirrors `perf_watch_ready` (read-only system, atomic flag) except that it does
  NOT latch - the true-to-false transition is the whole point.
- `ABORT_SCENE_ENDED` (`scene_ended`), checked in `perf_capture` beside
  `simulation_running` in both `Warmup` and `Capture`. Checked from the FIRST
  warm-up frame, because the frame the readiness gate opens on is the one most
  likely to be the frame the scene ended in.
- `wfc_arena` names `Scoreboard::both_teams_standing` - `pool` is `None` for a
  team with no live root, so it reads false for a wipe and for a between-match
  teardown.

`both_teams_standing` deliberately says nothing about how MANY ships are left. A
four-on-one is a fight in progress; refusing it would be a judgement about
workload that nothing here has measured, whereas "a side is gone" is the scene
not existing.

An example that names no predicate is never refused for one, and schedules
exactly what it scheduled before - the watcher system is only added when a
predicate was given.

### Consequences downstream

An aborted capture writes no CSV row and fails `capture_simulated`, so a bad
sample cannot enter a repeat set. There is no retry anywhere in the harness and
this change does not add one: refusing is the honest outcome, because a finished
fight does not restart.

The `capture_simulated` check and the report's Refused-captures note both
hard-coded the stopped-simulation story for every reason, which was already
wrong for `window_size`, `update_throttled` and `refresh_capped`. Both now name
the reason instead of asserting one. The CHECK NAME is left alone on purpose:
renaming it would break every stored `checks.json` baseline, and the module doc
now says to read the `reason` field rather than the name.

## Verification

- `cargo check --features debug --all-targets`, `cargo fmt --check`: clean.
- `cargo test --lib -p nova_probe_cli`: 135 pass. `cargo test --lib -p nova_probe
  --features debug`: 76 pass.
- New tests: the DOM differential above; a truncated trace keeping its prefix and
  flagging it; the unclosed-span cap refusing; a window opening on a finished
  scene refused at its first warm-up frame; a scene ending mid-window refused
  with its collected frames discarded; and an example with no predicate never
  refused for one.
- `probe run system_torpedo_launch --norender` finished in 428 s with verdict OK
  (7/8 measured, the one N/A being "no baseline"), `artifacts_loadable` PASS.
  Both `report.html` and `checks.json` were opened and read: the profile table
  renders populated off the 8.73 GB trace and leads with
  `bevy_time::fixed::run_fixed_main_schedule` at 13.7% over 8188 calls.
- The report was also re-rendered over the three archived traces above and its
  profile table read in each case.

## Next time

The differential test is the piece worth copying. Keeping the OLD implementation
in the test module and asserting exact equality against it turned "trust me, the
rewrite is equivalent" into something a reader can check in ten seconds, and it
costs about sixty lines that will fail loudly if anyone changes the aggregate.
