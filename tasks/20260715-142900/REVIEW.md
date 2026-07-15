# Review: Static mod portal - webmods/ + generator + deploy step

- TASK: 20260715-142900
- BRANCH: feature/mod-portal

## Round 1

- VERDICT: REQUEST_CHANGES (one MAJOR)

Out-of-context review pass (fresh-context agent over the full 19-file diff).
Verified true: the four moved type definitions are byte-identical to master's
(mechanical diff); the re-export keeps every downstream import compiling; the
generator's dependency tree is bevy-free (`cargo tree | grep -ci bevy` = 0);
write order is files-then-catalog as claimed; determinism is by construction
(sorted dirs/files, Vec-backed, fixed field order) plus the byte-identity
test; the gauntlet mod loads to recursive `Loaded` through the real loaders;
the workflow YAML parses, the step sits between assembly and deploy, and the
toolchain question was investigated to the action's source - the job resolves
the repo's nightly pin via rust-toolchain.toml for trunk AND the new step
alike, so they are consistent. Close-out claims all reproduced (3/9/1/11/1
test counts, generator output 2 files / 4654 bytes).

- [x] R1.1 (MAJOR) crates/nova_portal_gen/src/lib.rs:153-160 - a content path
  that ESCAPES the mod dir (`"../outside.content.ron"`, or an absolute path)
  passed validation via `mod_dir.join(content).is_file()` while `walk_files`
  never lists, hashes, or copies it: a broken mod publishes with exit 0
  (reproduced empirically by the reviewer), and the deep gate can also pass
  since it loads from the SOURCE tree where the escaped file exists.
  - Response: fixed on-branch - validation is now MEMBERSHIP in the walked
    file set (the exact set the portal serves), which subsumes missing,
    escaping, and non-normalized paths. New regression test
    `escaping_content_path_is_rejected` builds the reviewer's exact repro (the
    escaped file EXISTS next to the mod dir) and requires rejection.
- [x] R1.2 (MINOR) lib.rs:18 - doc comment named the deep gate
  "`webmods_load`"; the test is `webmods_validation`.
  - Response: fixed on-branch.
- [x] R1.3 (MINOR) tests/generate.rs - the plan promised an "entries sorted"
  assertion; byte-identity alone catches map-ordering only probabilistically
  at today's counts.
  - Response: fixed on-branch - the real-tree test now asserts entries sorted
    by id and each file list sorted by path.
- [x] R1.4 (MINOR) - an empty `--source` published a valid empty portal with
  exit 0; a workflow path typo would silently deploy an empty catalog.
  - Response: fixed on-branch - zero mods found is now an error ("no mods
    found ... refusing to publish an empty portal"), pinned by
    `empty_source_is_rejected`.
- [x] R1.5 (NIT) - the plan's "duplicate id" failure case is impossible by
  construction (ids are unique subdir names) but the substitution was
  undocumented, and `meta.name` had no failure-case test.
  - Response: fixed on-branch - a comment in `generate` states the
    impossibility, the close-out notes the substitution, and
    `empty_name_is_rejected` covers the name branch.
- [x] R1.6 (NIT) - a `*.bundle.ron` nested in a subdirectory is published as a
  plain file but is validated by neither gate; undocumented.
  - Response: fixed on-branch - docs/mod-portal.md now states only the
    root-level manifest is the entry point and nested bundle files are plain
    data.
- [x] R1.7 (NIT) docs/architecture.md - the backfilled nova_ui row omitted the
  `widget` module.
  - Response: fixed on-branch.

## Round 2

- VERDICT: APPROVE

All seven responses verified against the new diff: the membership check reads
exactly as suggested (walked set built first, content paths checked against
it), the escape repro test fails pre-fix by construction (it recreates R1.1's
empirical setup) and passes post-fix; `cargo test -p nova_portal_gen` is 12
passed (9 + escape + empty-name + empty-source); the real webmods tree still
publishes (gauntlet 1.0.0, 2 files, 4654 bytes); fmt clean. No new findings.
