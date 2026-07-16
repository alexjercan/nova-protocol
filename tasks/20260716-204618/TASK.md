# content_lint --target: lint a single mod (by id or path) for mod developers

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.7.0, modding, tooling

## Goal

`cargo run -p nova_assets --bin content_lint -- --target <mod>` lints
ONE mod instead of the whole tree - the mod-developer loop (user request
2026-07-16). `<mod>` is a directory path (any mod folder on disk, so an
external modder can point at their own work-in-progress) or an in-repo
id resolved against `webmods/<id>`, `assets/mods/<id>` and `base`.

## Steps

- [ ] lint_walk: split the repo walk from the lint loop; add
      `lint_target(dir) -> Vec<(String, LintIssue)>` - reads the target
      bundle, builds known sets from base + the full repo walk + the
      target's own content (so chains into base/dep scenarios resolve),
      lints ONLY the target's scenarios.
- [ ] Resolve helper: existing directory wins; otherwise try
      `webmods/<id>`, `assets/mods/<id>`, and `base` ->` assets/base`;
      unknown target exits with a readable error.
- [ ] Bin: minimal manual arg parsing (`--target <arg>`, no clap dep);
      same output format and exit-code contract.
- [ ] Tests: target the-ledger by id (findings match the full tree's
      ledger subset - the known ch4 warn, zero errors); a tempdir
      external mod referencing base prototypes plus one bad prototype
      (proves path mode + base visibility + error detection).
- [ ] Docs: guide-make-a-mod + authoring guide mention --target;
      CHANGELOG lint bullet gains the flag.
- [ ] Verify: check --all-targets, fmt, gate + new tests, bin run both
      modes.

## Notes

- Spike: tasks/20260716-193858/SPIKE.md (the lint family).
- Extends 20260716-191543 (CLOSED); the runtime gate is untouched.
