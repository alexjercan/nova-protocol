# Retro: Wire BCS autopilot + screenshot harness into nova examples

- TASK: 20260707-100002
- BRANCH: feature/v0.4.0-harness-wiring
- PR: #26 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE, two NITs addressed)

See `tasks/20260707-100002/TASK.md` for what changed and
`docs/2026-07-07-example-harness-wiring.md` for the design writeup; this retro is
only about how the working went.

## What went well

- Verified the cross-repo dependency before designing. The pinned
  `bevy_common_systems` rev (`47548cd`) happened to be the local repo's HEAD, so I
  read the actual `AutopilotPlugin` / `ScreenshotPlugin` source instead of guessing
  the API. That is what surfaced the asset-gating conflict early.
- Caught the real design hazard at design time, not via a crash. Reading
  `AppBuilder` and `autopilot.rs` together made it obvious that force-setting
  `Playing` on a timeline fights nova's asset-gated `Loading -> Playing` (fires
  before `GameAssets` exists, or double-runs `OnEnter(Playing)`). The single-step
  "hold Loading, let the loader arrive" shape fell out of that, avoiding a whole
  class of flaky behavior before writing it.
- Proactively closed the false-pass gap: a no-panic run stuck in `Loading` would
  otherwise still log "cycle complete". The `reached Playing` assertion makes that
  fail loudly.
- Verified end to end, not just compile: ran under Xvfb against the real GPU render
  node and confirmed the two log lines, clean exit, and an actual PNG - plus the
  no-`debug` build to prove the harness cfg's out.

## What went wrong

- `tatr new` ID collision during planning. Five rapid `tatr new` calls in the same
  wall-clock second all got ID `20260707-095020` (IDs are `YYYYMMDD-HHMMSS`), so
  four tasks silently overwrote one directory. Root cause: second-resolution IDs +
  no uniqueness guard in the tool. Recovered by writing the `TASK.md` files by hand
  with distinct minute offsets.
- Lost build visibility to a pipe buffer. The first full build was run as
  `cargo build ... | tail -40`; `tail` emits nothing until stdin closes, so the
  output file stayed empty for ~4 minutes and I briefly suspected a stall. Root
  cause: `tail -N` buffers to the end; it is the wrong tool for watching a
  long-running command.
- One small self-inflicted detour: an initial `pub use ... as _AutopilotPlugin`
  re-export that was pointless and immediately reverted.

## What to improve next time

- When creating several tatr tasks programmatically, write the `TASK.md` files
  directly with distinct IDs (or space the `new` calls past a second); do not fire
  `tatr new` in a tight loop.
- For a long background build/command, redirect to a file and `Read` it (or `tee`),
  never `| tail -N`, when interim progress matters.

## Action items

- [x] Both review NITs addressed on-branch (prelude narrowed, doc import added).
- [ ] Candidate AGENTS.md note (propose to user, do not self-edit their global
      file): "tatr IDs are second-resolution - batch-create tasks by writing files,
      not rapid `tatr new`," and "don't pipe long-running commands through `tail`."
- [ ] Follow-up already tracked: `20260525-133005` will wrap this env-gated run in a
      `#[test]`; the turret/torpedo ranges (`20260707-095008` / `20260707-100001`)
      consume `nova_autopilot().input(...)`.
