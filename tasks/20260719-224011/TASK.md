# probe multi-run Xvfb race: same pid-derived display respawned per example dies on stale lock; share ONE Xvfb across the sweep

- STATUS: OPEN
- PRIORITY: 64
- TAGS: v0.8.0,bug,tooling


## Goal

Field bug (user's first full-fat `probe run --all`, 2026-07-19):
`perf_baseline: Xvfb on :84 exited immediately (exit status: 1)`. In a
multi spec every `run()` derives the SAME display (`:80 + pid % 10` - one
process, one pid), spawns its own Xvfb and SIGKILLs it at run end; the
next run respawns the same display milliseconds later, and when the old
server's lock/socket has not cleared, the new one dies instantly. The
continue-on-failure driver reported it honestly (an ERROR row), but the
row is a probe artifact, not game evidence.

This falsifies T1's recorded deviation ("per-run Xvfb spawn buys zero new
lifecycle risk") - the spike's shared-Xvfb sketch was right. Fix: run_many
spawns ONE Xvfb (via ensure_display, honoring an explicit --display) and
pins it into every per-example RunOptions; one spawn, one kill, no
respawn race, and ~1s/run saved as a bonus.

## Steps

- [ ] run_many: ensure_display once before the loop, hold the guard for
      the sweep, set opts.display for every example; run()'s explicit-
      display path (no spawn) does the rest.
- [ ] E2E: a multi-run (category or 4-name list) completes green on ONE
      shared server; record here.
- [ ] Verify: fmt; cargo test -p nova_probe (no behavior pins change -
      the fix is orchestration).
