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
- NOT DONE: `arsenal` has never been played live. It is proved loadable, not
  proved playable.

## Delivered: the action rail

The follow-up is built. `bench movie <audit> --frames <dir>` reads the audit,
places each row on the frame its tick drew (`frame = tick - 1`), and composites
the rail in `nova_bench` - `image` decodes the PNGs, `ab_glyph` sets the game's
own terminal face, ffmpeg takes finished RGB frames on stdin and is the codec
only. A recorded play or replay now makes that movie by default; a missing face
falls back to the plain stitch.

The layout is style C of three mocked against a real frame: a header strip
naming scenario, agent, seed, clock, tick and turn, over six lines of
transcript with older rows fading. Seven lanes, each its own colour: `goal`,
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
- LIMIT: proved on recordings of one run. No red-team audit with `cheat`,
  `nova` or `err` rows has been composed - those lanes are covered by unit
  tests only, and wait on the arsenal play.
- KNOWN: rows sharing a tick are spread 21 frames apart, so a first turn of
  ten rows trails the world by up to three seconds. The rail is a log of the
  run, not a caption track; documented in `docs/agent-bench.md`.

## Follow-up

- Drive one live `bench play` against `arsenal` with `--record`, and grade the
  cheat and shell lanes against the footage.
