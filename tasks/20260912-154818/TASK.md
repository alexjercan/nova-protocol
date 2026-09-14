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
- NOT DONE: no live `bench play` has been driven against `slingshot` or
  `arsenal`. The fixtures are proved loadable, not proved playable.

## Follow-up

- Implement `bench movie` and the audit-driven action rail after the sandbox runs can compile and produce representative audits.
