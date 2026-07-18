# README overhaul: getting-started HOW-TO + a tools reference (how to run every tool/script)

- STATUS: OPEN
- PRIORITY: 65
- TAGS: v0.8.0,docs,readme,tooling

## Story

As a new contributor (human or agent) landing on the repo for the first time, I
want the README to take me from clone to a running game, web app and test suite
in copy-pasteable steps, and to name every dev tool with its exact invocation,
so that I never have to reverse-engineer the toolchain from CI configs and
scripts.

The repo `README.md` has "Build and run" and "The landing site" sections but no
"how to run the tools" reference and no short getting-started HOW-TO. Give a
new contributor the bare minimum to get productive, then point to the wiki for
depth. Headline of the v0.8.0 docs strand.

## Steps

- [ ] Add a getting-started HOW-TO near the top: clone -> nix/toolchain -> run
      the game (`cargo run`) -> run the web app (`web/`, `npm run serve`) ->
      run tests, each as a copy-pasteable command with one line of what it
      does.
- [ ] Add a "Tools" reference table/section documenting how to run every dev
      tool and script, with the exact command and one-line purpose:
  - [ ] content CLI: `cargo run -p nova_assets --bin content -- gen|lint|audit`
        (and `lint --target <mod>`)
  - [ ] perf: `scripts/perf-baseline.sh`, `scripts/perf-web.sh`, and the new
        HTML report bin (20260718-152230)
  - [ ] `nova_meta_gen`, `nova_portal_gen` (or their Python successors once
        20260718-152247 / 20260718-152255 land - keep this section in sync)
  - [ ] `scripts/`: gen-licenses.sh, gen-web-screenshots.py,
        gen-placeholder-sounds.py, cut-obj-into-hulls.py, preview-web.sh
- [ ] Fix the "Project layout" crate table: it lists ~8 of 15 crates. Add
      nova_perf, nova_meta_gen, nova_portal_gen, nova_mod_format, nova_modding,
      nova_ui, nova_info, nova_debug with one-line purposes.
- [ ] Every HOW-TO/tool entry links to the deeper wiki page under
      `web/src/wiki/dev/` (development.md, mod-portal.md, ...) rather than
      duplicating it. README = bare minimum to start; wiki = detail.
- [ ] Verify every command in the final README by running it on a clean
      checkout (or as close as the environment allows); note any that need
      preconditions.

## Definition of Done

- A fresh reader can go clone -> playable game, served web app, green tests
  using only the README, on nix and on a plain toolchain.
- Every bin under `crates/` and every script under `scripts/` appears exactly
  once in the tools section with a working invocation.
- The crate table lists all 15 workspace crates with accurate one-liners.
- No section duplicates wiki content deeper than one paragraph; each links out
  instead.

## Notes

- Sequencing: best done LATE in the v0.8.0 cycle, after the tooling refactors
  land (perf HTML 20260718-152230, portal port 20260718-152247, meta spike
  20260718-152255, inventory 20260718-152304), so the tools section documents
  the end state instead of chasing it.
- Source of truth for detail stays the wiki (see docs/README.md).
- The dev-wiki side of the same sweep is 20260718-152214 (it adds nova_perf /
  nova_meta_gen to project-tour + development.md); keep the two consistent.
