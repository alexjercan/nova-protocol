# KISS pass on nova_probe - design record

## Structure

Two files were over 1500 lines; both held several concerns.

### `src/bin/probe.rs` (2460) -> `src/bin/probe/`

The bin was one file wrapping a single `mod native { ... }` block. A bin file
at `src/bin/probe.rs` resolves its submodules against `src/bin/`, not
`src/bin/probe/`, so the whole target moved to the directory form
(`[[bin]] path = "src/bin/probe/main.rs"` in Cargo.toml). `main.rs` is now the
front door only: the crate doc, the wasm stub `main`, and `native::main()`.

| Module | Concern |
| --- | --- |
| `native.rs` | module wiring + `main()` dispatch |
| `native/cli.rs` | usage text, flags, the parsed `Cmd` (pure) |
| `native/spec.rs` | spec/category resolution against the catalog (pure) |
| `native/paths.rs` | repo/output/baseline path resolution, git SHA lookups |
| `native/env.rs` | fps window + per-pass child env blocks, matrix cells |
| `native/supervise.rs` | Xvfb, example build, timeout-bounded child runs |
| `native/run.rs` | one example through the passes, then the run report |
| `native/web.rs` | the `--platform web` pass (trunk, static server, Chromium) |
| `native/sweep.rs` | the multi-example driver and aggregate index |
| `native/report.rs` | `probe report` re-render |
| `native/fixtures.rs` | `cfg(test)` argv/catalog builders shared by 3 test mods |

### `src/run_report.rs` (1590) -> `src/run_report/`

| Module | Concern |
| --- | --- |
| `mod.rs` | crate-facing doc + `pub use` (public paths unchanged) |
| `manifest.rs` | `RunManifest`, `PassRecord`, `run_identity` |
| `artifacts.rs` | `RunArtifacts` loading of a run dir |
| `checks.rs` | the check rows, verdict, and `checks.json` mirror |
| `html.rs` | `render_run_report` |
| `fixtures.rs` | `cfg(test)` fixture dir, scratch copy, healthy manifest |

Largest file in the crate is now `run_report/checks.rs` at 913 lines, one
concern (the automatic checks and their tests).

Public paths are unchanged: `run_report/mod.rs` re-exports every item the old
file exported, and `lib.rs` is untouched. Inside the bin, items that cross a
module boundary became `pub(crate)`; nothing else changed.

## Comments

Applied the epic's rubric across the crate. Every task-HUID provenance clause
is gone (`grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_probe/` returns
nothing - DoD 3 needs no exception list), as are the "finding N" / "review
R1.x" back-references to closed review rounds. Surviving constraints were
promoted to `NOTE:`; narration that restated the code was deleted. Rustdoc was
kept, edited only where a deleted clause left the sentence ragged.

## Evidence that behavior did not change

The set of `#[test]` function names in `crates/nova_probe` is byte-identical
before and after the split (diff of the sorted name lists is empty), and
`cargo test -p nova_probe --lib --bins` runs 97 tests green in the split
tree. No test was dropped, renamed or weakened.

## Defects found

None. The pass uncovered no defect worth a backlog task.
