# content_lint --target: lint a single mod (by id or path) for mod developers

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.7.0, modding, tooling

## Goal

`cargo run -p nova_assets --bin content_lint -- --target <mod>` lints
ONE mod instead of the whole tree - the mod-developer loop (user request
2026-07-16). `<mod>` is a directory path (any mod folder on disk, so an
external modder can point at their own work-in-progress) or an in-repo
id resolved against `webmods/<id>`, `assets/mods/<id>` and `base`.

## Steps

- [x] lint_walk: split the repo walk from the lint loop; add
      `lint_target(dir) -> Vec<(String, LintIssue)>` - reads the target
      bundle, builds known sets from base + the full repo walk + the
      target's own content (so chains into base/dep scenarios resolve),
      lints ONLY the target's scenarios.
- [x] Resolve helper: existing directory wins; otherwise try
      `webmods/<id>`, `assets/mods/<id>`, and `base` ->` assets/base`;
      unknown target exits with a readable error.
- [x] Bin: minimal manual arg parsing (`--target <arg>`, no clap dep);
      same output format and exit-code contract.
- [x] Tests: target the-ledger by id (findings match the full tree's
      ledger subset - the known ch4 warn, zero errors); a tempdir
      external mod referencing base prototypes plus one bad prototype
      (proves path mode + base visibility + error detection).
- [x] Docs: guide-make-a-mod + authoring guide mention --target;
      CHANGELOG lint bullet gains the flag.
- [x] Verify: check --all-targets, fmt, gate + new tests, bin run both
      modes.

## Notes

- Spike: tasks/20260716-193858/SPIKE.md (the lint family).
- Extends 20260716-191543 (CLOSED); the runtime gate is untouched.

## Close notes (2026-07-16)

What changed: lint_walk split into walk_repo_bundles + lint_bundle
(shared by both modes - the known-set rules stay byte-identical);
lint_target(dir) lints only the target bundle with the full repo walk
as known set (in-repo targets deduped by dir-name id so their own
content is not double-counted); resolve_target(arg) prefers an existing
directory (external modder path), then webmods/<id>, assets/mods/<id>,
base. The bin parses `--target <arg>` manually (no new dependency),
prints the resolved dir, errors readably on unknown targets, and keeps
the exit-code contract.

Verified live: --target the-ledger (id mode, exactly the known ch4
warn), --target base (clean), --target nope (readable error, non-zero),
no-arg full tree unchanged. Tests: in-repo target findings are the
ledger subset with zero errors; an external tempdir mod using one real
base prototype and one bogus one flags exactly the bogus (proves path
mode, base-catalog visibility, and dir-name attribution). Gate 2/2,
check --all-targets + fmt clean. Full suite is CI's job per the
standing instruction.
