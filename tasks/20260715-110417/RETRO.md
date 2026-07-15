# Retro: Aggregate Rust dependency licenses (cargo-about) (20260715-110417)

Closed 2026-07-15. Verdict APPROVE. Build/CI chore; the interesting part was a
mid-task design pivot forced by a determinism finding.

## What shipped

- `about.toml` + `about.hbs` (Markdown template) driving `cargo-about` to
  produce a per-crate third-party dependency-license manifest.
- Generation wired into all shipping paths (release macOS/linux/windows/web +
  deploy-page) so the manifest ships fresh in every build; a CI `licenses` job
  runs the same generation as a copyleft/unknown-license gate.
- First-party nova_* crates + root binary marked `publish = false` with the
  workspace MIT license, excluded from the manifest via `[private] ignore`.
- `scripts/gen-licenses.sh`; `credits/CREDITS.md` points at it.

## The pivot: commit+diff -> build-time generation

I first built the standard pattern: commit the manifest, CI regenerates and
`git diff --exit-code` to catch drift. During review I tested determinism and
found cargo-about's output is NOT byte-stable - two warm back-to-back runs
differed by ~20 lines (whole `windows-*` blocks appearing/disappearing). A
committed copy would fail the diff every run. Pivoted to generating the manifest
fresh at build/release time (never committed, gitignored); the CI job keeps the
copyleft gate but no longer diffs. This is strictly more correct - the shipped
manifest always matches the actual dependency graph - and matches the task's
"wire generation into the build/release" wording.

## What went well

- Running the real tool early surfaced two genuine findings the config gate is
  meant to catch: `MIT-0` (encase) not in the accepted set, and the first-party
  crates lacking a license field. Both were fixed the right way (widen accepted
  deliberately; mark internal crates publish=false + MIT) rather than papered
  over.
- Testing determinism BEFORE committing to the CI strategy saved shipping a
  check that would fail on every run.

## Difficulties

- TOML ordering bug: I put the `[private]` table before the top-level
  `targets`/`accepted` keys, so TOML folded them into `[private]` and cargo-about
  errored with "unknown field `targets`". Fix: all top-level keys must precede
  any table header. Obvious in hindsight, cost one run.
- No YAML validator in the base env (no pyyaml/ruby/yq); validated the workflow
  edits with `js-yaml` from `web/node_modules` via node.
- `which cargo-about` reported "not installed" after a SUCCESSFUL
  `cargo install` - `~/.cargo/bin` was just not on PATH. Verify a tool via
  `cargo <sub> --version`, not `which`, before concluding it failed.

## Lessons for next time

- **Verify a generator's output is byte-stable before choosing a commit+diff CI
  gate.** If it isn't (cargo-about isn't), generate the artifact at build time
  and have CI assert generation SUCCEEDS, not that it matches a committed copy.
- **TOML: every top-level key must precede the first `[table]` header** or it
  silently belongs to that table.

## Left out / notes

- The release/deploy workflow steps are exercised only on tag/dispatch; the
  identical generate command runs on every PR via the CI gate, so only the
  in-workflow bundling placement is unverified until a real release.
- A dependency-license manifest covers CODE licenses; the asset audit
  (20260714-154958) covers assets. Together the shipped `credits/` is complete.
