# README overhaul: getting-started HOW-TO + a tools reference (how to run every tool/script)

- STATUS: OPEN
- PRIORITY: 65
- TAGS: v0.8.0,docs,readme,tooling

## Goal

The repo `README.md` has "Build and run" and "The landing site" sections but no
"how to run the tools" reference and no short getting-started HOW-TO. Give a new
contributor the bare minimum to get productive, then point to the wiki for
depth. Headline of the v0.8.0 docs strand.

## Steps

- Add a getting-started HOW-TO near the top: clone -> nix/toolchain -> run the
  game (`cargo run`) -> run the web app (`web/`, `npm run serve`) -> run tests,
  each as a copy-pasteable command with one line of what it does.
- Add a "Tools" reference table/section documenting how to run every dev tool
  and script, with the exact command and one-line purpose:
  - content CLI: `cargo run -p nova_assets --bin content -- gen|lint|audit`
    (and `lint --target <mod>`)
  - perf: `scripts/perf-baseline.sh`, `scripts/perf-web.sh`, and the new HTML
    report bin (20260718-152230)
  - `nova_meta_gen`, `nova_portal_gen` (or their Python successors once
    20260718-152247 / 20260718-152255 land - keep this section in sync)
  - `scripts/`: gen-licenses.sh, gen-web-screenshots.py,
    gen-placeholder-sounds.py, cut-obj-into-hulls.py, preview-web.sh
- Fix the "Project layout" crate table: it lists ~8 of 15 crates. Add
  nova_perf, nova_meta_gen, nova_portal_gen, nova_mod_format, nova_modding,
  nova_ui, nova_info, nova_debug with one-line purposes.
- Every HOW-TO/tool entry links to the deeper wiki page under
  `web/src/wiki/dev/` (development.md, mod-portal.md, ...) rather than
  duplicating it. README = bare minimum to start; wiki = detail.

## Notes

- Keep in sync with the tooling refactors landing this release (perf HTML,
  portal/meta Python ports, tooling inventory 20260718-152304).
- Source of truth for detail stays the wiki (see docs/README.md).

