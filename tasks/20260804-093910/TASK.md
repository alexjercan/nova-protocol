# Retire the mainline and POC example runs, reduce screenshots to capture-only

- PRIORITY: 78
- TAGS: v0.10.0, examples, testing
- KIND: STORY
- ACTIVITY: PLANNING
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-115955
- DEPENDS ON: 20260804-003244, 20260804-093855, 20260804-093934

## Story

Delete what the roster spike (`20260804-003244`) retired, and reduce
`screenshots/` to what its contract allows. All mechanical, no new content.

Story scenarios lose their example coverage on purpose. Not because they churn
- the spike's review measured that and it is false (`broadside.rs` 11 commits
ever, `lifeline.rs` 6) - but because an autopilot-assisted win over 8000 lines
of story RON proves little: `broadside` and `lifeline` assert story wave
timings and object ids, which is content, not system behavior. Story is tested
by players; examples test systems.

## Steps

- [ ] Delete `examples/gameplay/broadside.rs` and `lifeline.rs` with their
      catalog and smoke entries; delete the now-empty `gameplay/` directory.
- [ ] Delete `examples/ui/nova_os_rtt_poc.rs` and its entries.
- [ ] Reduce all eight `screenshots/` runs to capture producers: enter, wait on
      a predicate, shoot, exit. No assertions, no fps wiring, no probe
      enrollment.
- [ ] Delete the per-example hacks the driver now owns - beat booleans, panic
      guards, reload-gate polls, ad-hoc runways, and the live `playing_since`
      offsets in `screenshot_orbit.rs:151`, `screenshot_juice.rs:205`,
      `screenshot_combat.rs:231`.

## Definition of Done

- The retired runs are gone from the tree and the catalog.
  (cmd: `! rg -n 'broadside|lifeline|nova_os_rtt_poc' Cargo.toml examples tests`)
- No example carries a hand-rolled completion guard or beat-boolean script.
  (cmd: `! rg -n 'run ended with the scripted run unfinished|playing_since' examples`)
- The reduced screenshot producers still run their full harnessed cycle and
  exit clean, so the capture path is intact even though probe no longer runs
  them. (test: `screenshots_reach_playing_without_panic`)
- The catalog, disk and smoke lists agree after the deletions.
  (test: `catalog_matches_disk`)

## Notes

- RETIRE `examples/gameplay/broadside.rs` and `examples/gameplay/lifeline.rs`.
  Their SYSTEM coverage (scenario chaining, Defeat + Retry reload-clean,
  Victory/CHECKPOINT) is NOT dropped - it moves to `systems/outcomes` in the
  systems task. Do not land this before that coverage exists, or the tree
  briefly has no evidence for those four systems.
- RETIRE `examples/ui/nova_os_rtt_poc.rs`: the RTT pipeline shipped, and a POC
  is not coverage. Its coverage becomes an RTT element test beside the other
  widget tests.
- Delete `fps_exempt = ["broadside"]` (Cargo.toml:35) - the only entry.
- Coverage flag: `--report`, one name, built and owned by `20260724-082856`
  (which now DEPENDS ON this task). Deliberately NOT in this task's DoD - that
  would be circular, since 082856 needs these rebuilt producers and this task
  cannot be gated on a flag 082856 has not written yet. Shot-for-shot coverage
  of the web build is 082856's criterion; this task proves only that the
  reduced producers still run clean.
- Reduce all eight `screenshots/` runs to capture producers on the shared
  driver: enter, wait on a predicate, shoot, exit. No assertions, no fps
  wiring, no probe enrollment. `render_scale_shot` stays out of probe entirely.
- The three `*_poc.html` design sources are NOT this task's: epic child
  `20260804-003301` owns that move. Named here only so the boundary is clear.
- Examples must be RUN under Xvfb :99, not only checked.
