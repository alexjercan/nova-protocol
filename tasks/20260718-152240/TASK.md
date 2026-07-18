# content audit/lint: emit a human-friendly per-mod report that pinpoints where each finding occurs (file + location), for single-mod audits

- STATUS: OPEN
- PRIORITY: 48
- TAGS: v0.8.0,tooling,modding

## Goal

The `content` CLI (`crates/nova_assets/src/bin/content.rs`) has `lint` and
`audit`. `audit` prints a combat balance sheet; `lint` surfaces reference errors
(unknown prototypes, dangling NextScenario chains, unseated mounts, unmatched
ObjectiveComplete - `crates/nova_scenario/src/lint.rs`). The user wants the mod
tool to generate a proper report when you audit a single mod: a human-friendly
document that shows WHERE each finding happens (which file, which id/field), not
just a stdout line. This makes the tool useful for a modder debugging their
bundle.

## Steps

- Give `lint --target <mod>` / `audit` a report mode (e.g. `--report <path>` or
  `--format md|html`) that writes a per-mod report grouping findings by
  severity and, for each, the source location: file (relative to the mod),
  the offending id/prototype/field, and a short explanation + suggested fix.
- Ensure `lint.rs` findings carry enough location context to point at the file
  and element; enrich the finding type if it currently only has a message.
- Consider folding lint + balance findings into one "audit this mod" report so
  a modder runs one command and gets both reference errors and balance notes.
- Markdown is the baseline output (diffable, pasteable); HTML optional if it is
  cheap and matches the perf-report styling (20260718-152230).

## Notes

- Linter is embedded in `nova_scenario::lint` and reused by the CLI, the
  `content_lint_gate` test, and the runtime merge sweep - keep those working.
- `resolve_target` already maps a mod name/id to a dir (webmods/, assets/mods/,
  base); the report just needs a nicer sink than println!.

