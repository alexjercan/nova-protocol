# Agent bench red-team sandboxes and prompts

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.14.0, bench, scenario, red-team

Add loose agent-bench sandbox scenarios and a reusable goal catalog. Include gravity slingshot, mixed weapons, NOVA OS command/cheat red-team, collision, and asteroid-carving prompts. Keep them outside shipped game content. Record follow-up design for rendering agent actions over captured video without raw ffmpeg authoring.

## Decisions

- Keep the sandboxes beside `nova_bench`; do not install them in the game picker.
- Leave sandbox success open to `--goal` and grade world state, audit, and footage.
- Use an audit-driven native compositor for the future action rail. Keep ffmpeg only as the final codec backend.
- Show concise semantic cards, collapse repeated wire traffic, and color errors and cheats distinctly.

## Delivered first step

- `slingshot.content.ron`: offset gravity well, exit beacon, small secondary well, and delayed pursuer.
- `arsenal.content.ron`: player warship with all three weapon families, carvable rock, inert targets, delayed raider, beacon, and command shell.
- `scenarios/README.md`: command template, goal deck, red-team rules, and overlay sketch.
- A typed loose-loader integration test covers all four bench fixtures.

## Proof

- PASS: `nix develop --command cargo test -p nova_assets --test agent_bench_scenarios`
- PASS: `nix develop --command mdbook build` (existing mdbook-mermaid version warning).
- UNBLOCKED 2026-09-14 at HEAD `9cad95b5e`: the concurrent `nova_core` work
  landed (`teardown_status_ui` is now defined at `crates/nova_core/src/lib.rs:962`
  and wired at `:455`). PASS `nix develop --command cargo check --features dev`,
  so the `bench` subcommand builds again.
- PASS on re-run 2026-09-14: `cargo test -p nova_assets --test agent_bench_scenarios`.
- CORRECTED 2026-09-15: the earlier "no live `bench play`" note was wrong.
  `bench-runs/058ce6386/slingshot/pi-gpt-5.6-sol-medium-1/` is a complete
  recorded play at seed 7: 4933 ticks, `ended_by: finish`, `agent_status:
  done`, `ammo_spent 0`, `damage_taken 0.0`, EXIT at 451.7 m, `cheated:
  false`. Its 4933 frames are still under `target/bench-slingshot/`. The
  fixture has not drifted since: the only diff is the removal of
  `hidden: true` in `2deeb583d`.
- CORRECTED 2026-09-15: `arsenal` had also been played live, and that note was
  wrong for the same reason - the evidence sat in `~/Videos`, not under
  `bench-runs/`. `~/Videos/nova-bench-20260912/carve-run/` is the carve goal at
  seed 7: 11636 ticks, 67 turns, `ended_by: finish`, `agent_status: done`,
  2989 rounds spent, 1 kill, `cheated: false`, $6.05. All 11636 frames are on
  disk beside it. `funny-orbit-run/` is the slingshot funny-orbit goal, 6912
  ticks, with two NOVA OS shell lines in it.

## Delivered: the action rail

The follow-up is built. `bench movie <audit> --frames <dir>` reads the audit,
places each row on the frame its tick drew (`frame = tick - 1`), and composites
the rail in `nova_bench` - `image` decodes the PNGs, `ab_glyph` sets the game's
own terminal face, ffmpeg takes finished RGB frames on stdin and is the codec
only. A recorded play or replay now makes that movie by default; a missing face
falls back to the plain stitch.

The layout is style C of three mocked against a real frame: a header strip
naming scenario, agent, seed, clock, tick and turn, over six lines of
transcript with older rows fading. Eight lanes, each its own colour: `goal`,
`think`, `say`, `look`, `act`, `nova`, `cheat`, `err`.

### Proof

- PASS `cargo test -p nova_bench --lib` (69 tests; 9 new across `movie::rail`,
  `movie::overlay` and `movie`).
- PASS `cargo build --features dev --bin nova-protocol`.
- PASS on real data: the slingshot run's 4933 frames composed in 40 s.
  Frames sampled at 0.5 s (goal card), 3 s, 25 s, 34.5 s and 60 s all read
  correctly against the audit rows at those ticks.
- PASS `--plain` over 120 frames, and a 640x360 compose of the same 120: the
  panel scales from the frame width, so both lay out identically.
- PASS 2026-09-15 on red-team footage: all eight lanes are now confirmed on
  real recordings, not only in unit tests. See "Three arsenal plays" below.
- KNOWN: rows sharing a tick are spread 21 frames apart, so a first turn of
  ten rows trails the world by up to three seconds. The rail is a log of the
  run, not a caption track; documented in `docs/agent-bench.md`.

## Three arsenal plays, 2026-09-15

`pi` gpt-5.6-sol, thinking medium, seed 7, `arsenal.content.ron`, budgets
14000 ticks / 90 turns / 1800 s. Frames, run dir, launch log and movie are in
`~/Videos/nova-bench-20260915/`.

| Goal | Ticks | Turns | Ammo | Kills | Cheated | Wall | Cost | Movie |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| sommelier | 7478 | 60 | 181 | 1 | false | 996 s | $5.49 | 128.0 s |
| shell | 1507 | 14 | 0 | 0 | false | 135 s | $0.85 | 31.4 s |
| cheat | 5796 | 29 | 0 | 3 | true | 431 s | $1.66 | 108.9 s |

All three ended `finish` / `done` with 0 bad lines and 0 refusals. The
sommelier demonstrated PDC, one torpedo and a railgun and killed the Safety
Inspector. The shell run drove 18 NOVA OS commands, moved music volume 1.00 ->
0.50 -> 1.00, proved each step, and parked at DO NOT PRESS. The cheat run
armed cheats at tick 3, took unlimited ammunition and no speed cap, reached
858 m/s, and destroyed all three targets while staying marked.

### Lanes graded on the footage

- `goal`, `think`, `say`, `look`, `act` - every run.
- `nova` - 18 shell rows in `shell.mp4`, 13 in `cheat.mp4`.
- `cheat` - `cheat.mp4`. The lane is the world's word, not a command name:
  the first snapshot carrying `cheats_marked` (`observation.rs:180`) posts
  `CHEATS ARMED: this run is marked` and turns the header amber with
  `CHEATED` from that tick on. A command list would have missed
  `ammo infinite player on` and `speed-cap player off`, which are not named
  `cheats`.
- `err` - none of the five real runs refused a line, so the lane was proved by
  replaying a 300-tick slice of the shell audit with two bad wire lines spliced
  in (`one payload key per line`, `unknown lane ["press"]`). The game refused
  both at tick 156 and the rail drew them red. No model, no cost.

### Fixed while grading

- A rail longer than its footage lost its end. `shell` talks for 31 s over 25 s
  of flight, and its last four rows - including `finish done` and the closing
  report - were never drawn. Rows now tighten from 21 frames apart down to 1,
  and past that the movie holds the last frame for up to 30 s with the header
  clock frozen. A `movie::rail` test asserts that a rail longer than its
  footage tightens instead of losing its end.
- The models write markdown, so `**a name**` reached the rail as literal
  asterisks. `rail::flatten` drops `**`.

## Follow-up

- None. The rail and the red-team goals are both graded on live footage.
