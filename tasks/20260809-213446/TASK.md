# Sync the wiki to the post-refactor crate layout and close the process gap

- STATUS: OPEN
- PRIORITY: 38
- TAGS: v0.10.0,docs,web

PROBLEM: The 20260806-121625 refactor moved ~50k lines into new crates
(`nova_ship`, `nova_hud`, `nova_os_ui`, `nova_authoring`, `nova_probe_cli`,
`nova_perf_web`, the `nova_ui` screen module) and `web/src/wiki/` was never
updated. The prose now describes a codebase that does not exist - the exact
live risk `notes/18-benchmark-baseline.md` named.

Benchmark evidence (after-run): the `docs` persona fell 0.75 -> 0.58
(corrected), gave up on 4 questions, and failed the t1-005 CONTROL question by
inferring from prose that `nova_hud` never touches `nova_ui` - no wiki page
describes the new crates or their dependencies. The prose channel was the
weakest at baseline and is the only channel that genuinely regressed.

Two halves:

1. Update the wiki. Sweep `web/src/wiki/` for every page that names a crate,
   module path, or `cargo run -p` target and re-derive it against the current
   tree. Architecture pages first (crate map, dependency directions), then the
   dev guides.
2. Close the process gap. The epic ran 12 lanes with per-lane proofs and a
   final review, and no step anywhere said "update the wiki".
   `web/src/wiki/dev/keeping-docs-in-sync.md` is the declared routing map and
   it did not catch a four-crate split. Work out why (routing map not in any
   lane checklist? no CI signal? review scope excluded web/?) and fix the
   workflow so a structural refactor cannot land without a wiki pass again.
   Record the finding, not just the fix.
