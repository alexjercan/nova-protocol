# content audit/lint: emit a human-friendly per-mod report that pinpoints where each finding occurs (file + location), for single-mod audits

- STATUS: OPEN
- PRIORITY: 48
- TAGS: v0.8.0,tooling,modding

## Story

As a modder debugging my bundle, I want `content lint --target <my-mod>` to
hand me a report that says which file, which id and which field each finding is
about, with a suggested fix, so that I can repair my mod without grepping my
own content against a bare stdout message.

The `content` CLI (`crates/nova_assets/src/bin/content.rs`) has `lint` and
`audit`. `audit` prints a combat balance sheet; `lint` surfaces reference
errors (unknown prototypes, dangling NextScenario chains, unseated mounts,
unmatched ObjectiveComplete - `crates/nova_scenario/src/lint.rs`). Both speak
in stdout lines today; a modder debugging a multi-file bundle (The Ledger has
five content files) deserves a document.

## Steps

- [ ] Give `lint --target <mod>` / `audit` a report mode (e.g. `--report
      <path>` or `--format md|html`) that writes a per-mod report grouping
      findings by severity and, for each, the source location: file (relative
      to the mod), the offending id/prototype/field, and a short explanation +
      suggested fix.
- [ ] Ensure `lint.rs` findings carry enough location context to point at the
      file and element; enrich the finding type if it currently only has a
      message.
- [ ] Consider folding lint + balance findings into one "audit this mod"
      report so a modder runs one command and gets both reference errors and
      balance notes; record the decision either way.
- [ ] Markdown is the baseline output (diffable, pasteable); HTML optional if
      it is cheap and matches the perf-report styling (20260718-152230).
- [ ] Prove it on the shipped mods: generate the report for `the-ledger`,
      `gauntlet` and `example` and check every finding's location actually
      points where the report says (seed one deliberate error to verify).
- [ ] Document the report mode in the dev wiki (guide-make-a-mod.md pre-publish
      check, development.md tools list) and the README tools section
      (20260718-152205).

## Definition of Done

- One command produces a per-mod findings report where every finding names
  file + element + explanation + suggested fix; zero findings produces a clean
  report, not an empty file.
- The existing consumers of `nova_scenario::lint` (CLI, `content_lint_gate`
  test, runtime merge sweep) still pass unchanged.
- A deliberately broken test mod's report pinpoints the planted error, and a
  test pins that behavior.

## Notes

- Linter is embedded in `nova_scenario::lint` and reused by the CLI, the
  `content_lint_gate` test, and the runtime merge sweep - keep those working.
- `resolve_target` already maps a mod name/id to a dir (webmods/,
  assets/mods/, base); the report just needs a nicer sink than println!.
- The docs review (2026-07-18) recommends the wiki's pre-publish checklist
  point at this report once it exists (see 20260718-231601's publish-vs-load
  split item) - coordinate wording.
