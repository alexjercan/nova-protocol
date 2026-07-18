# Tooling inventory + consolidation pass: catalog every bin/script, decide what merges or moves to build-time, document the result

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.8.0,tooling,refactor,docs

## Goal

The user wants the tooling refactored into a better structure: "what can be
merged or moved into some kind of buildtime script we do that." Before moving
pieces individually, take one inventory pass so the moves are coherent rather
than ad hoc. Produces the map that the README tools section (20260718-152205)
and the individual refactor tasks consume.

## Steps

- Catalog every dev entry point with its purpose, invocation, and dependencies:
  - Rust bins: `content` (gen/lint/audit), `nova_perf` (perf_web + new report),
    `nova_meta_gen`, `nova_portal_gen`.
  - scripts/: perf-baseline.sh, perf-web.sh, preview-web.sh, gen-licenses.sh,
    gen-web-screenshots.py, gen-placeholder-sounds.py, cut-obj-into-hulls.py.
- For each, classify: keep-as-Rust, port-to-Python (portal 20260718-152247,
  maybe meta 20260718-152255), fold-into-build-step, or leave. Note what is a
  true build-time hook (meta gen on Trunk build; content gen as a pre-commit /
  CI gate; portal gen on deploy) vs an on-demand dev tool.
- Recommend the consolidated structure (e.g. a single `scripts/` home + a
  documented build-time hook list; whether a task-runner/Justfile is worth it)
  and record it so later tasks execute against one target picture.
- Update the README tools section + wiki development.md with the final map.

## Notes

- This is the umbrella that sequences the concrete tooling tasks; keep it light,
  it is a plan + doc, not a big code change. Content linter stays embedded in
  `nova_scenario::lint` (good as-is), per the survey.

