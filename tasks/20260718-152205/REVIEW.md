# Review: README overhaul - getting-started HOW-TO + tools reference

- TASK: 20260718-152205
- BRANCH: docs/readme-overhaul

## Round 1

- VERDICT: APPROVE

Clean docs diff (README.md + TASK.md only) that delivers every Definition-of-
Done point. I independently re-derived the load-bearing claims rather than
trusting the summary:

- **Bin coverage (DoD: every bin appears exactly once).** `cargo metadata`
  reports exactly six bin targets: `content` (nova_assets), `probe` +
  `perf_web` (nova_probe), `nova_meta_gen`, `nova_portal_gen`, and the root
  `nova-protocol` game. All five crate bins are in the Tools section; the game
  bin is the `cargo run` in Getting started / Build and run. No bin missing,
  none duplicated.
- **Script coverage.** All five scripts under `scripts/` (gen-licenses.sh,
  gen-web-screenshots.py, gen-placeholder-sounds.py, cut-obj-into-hulls.py,
  preview-web.sh) are documented with args matching each script's argparse /
  header.
- **Crate table.** 16 rows = 15 workspace crates + `web`; one-liners match the
  canonical AGENTS.md table.
- **Command accuracy.** `content -- lint` RUN end-to-end (exit 0). Other CLIs
  source-verified against their clap defs; portal_gen's documented example
  (`--shipped assets/mods.catalog.ron`) references a path that exists;
  `cargo test --workspace --features debug` matches CI's exact test set.
- **Stale-premise handling.** Correctly documents the end state, not the task's
  older wording: `trace` retired (run/report only), meta/portal still Rust (no
  Python successors). Both drifts are called out in TASK.md.
- **Links.** Every link target verified to exist on disk. Good catch fixing the
  pre-existing broken banner (`assets/banner.png` -> `assets/base/banner.png`,
  moved in d055337a and never updated in the README).
- **No wiki duplication.** The probe/screenshot detail stays one line + a link
  into development.md. The landing-site section is detailed but is NOT
  duplicated in the dev wiki (development.md does not cover the proxy/preview
  flow), so it is the canonical home, not a duplication.

- [x] R1.1 (NIT) README.md:60 - the Getting started `cargo test --workspace
  --features debug` line is accurate and CI-faithful, but the windowed
  `examples_smoke` sub-test needs an X display (CI runs it under `xvfb-run`).
  It self-skips loudly without one (ci.yaml:79), so the command never FAILS
  headless - worth a half-clause noting "(the windowed examples-smoke test
  self-skips without a display; CI runs it under xvfb-run)" to fully satisfy
  the DoD's "note any that need preconditions." Optional; take it or leave it.
  - Response: Folded in - the test line now ends "(windowed smoke test needs a
    display; else it self-skips)". Resolved.
