# probe multi-run Xvfb race: same pid-derived display respawned per example dies on stale lock; share ONE Xvfb across the sweep

- STATUS: CLOSED
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

- [x] run_many: ensure_display once before the loop, hold the guard for
      the sweep, set opts.display for every example; run()'s explicit-
      display path (no spawn) does the rest.
- [x] E2E: a multi-run (category or 4-name list) completes green on ONE
      shared server; record here.
- [x] Verify: fmt; cargo test -p nova_probe (no behavior pins change -
      the fix is orchestration).

## Close-out (2026-07-19, branch fix/probe-shared-xvfb)

Fixed BOTH faces of the display weakness, because the e2e found the
second one live:

1. Self-race (the user's report): run_many now calls ensure_display ONCE
   and pins the display into every per-example RunOptions - one spawn,
   one kill, no kill/respawn race, ~1s/run saved. T1's "per-run spawn
   buys zero risk" deviation is falsified and reverted to the spike's
   shared-server design.
2. Cross-process collision (found when THIS fix's first e2e died on the
   user's live sweep holding :84 - two probes, same pid%10): the fixed
   pid-derived display is replaced by a WALK - `display_candidates()`
   (pure, pinned: the full :80-:89 band exactly once, pid-anchored
   start) tried in order until an Xvfb actually holds; error only when
   all ten die.

Evidence: e2e attempt 1 (shared server, fixed display) died on the
foreign :84 - an honest reproduction of the collision case; with the
walk, `probe run thruster_section,hull_section,turret_section,
torpedo_section` completed 4/4 OK measured 5/6 on one shared server
WHILE the user's --all sweep ran on the same host. 80/80 tests, zero
warnings (dead default_display removed; the walk's band/coverage/
stability pinned by the replacement test). The post-e2e extraction of
display_candidates() is a pure refactor of the exact walk the e2e ran.
