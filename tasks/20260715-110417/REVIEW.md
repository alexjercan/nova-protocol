# Review: Aggregate Rust dependency licenses (cargo-about) (20260715-110417)

Branch: `chore/dependency-licenses`

## Verdict: APPROVE

Delivers the task: a complete per-crate third-party license manifest that ships
in every build, a copyleft/unknown-license gate, and generation wired so it
cannot go stale. One significant design decision (below) drove the shape.

## Key decision: generate at build time, do NOT commit

The obvious design - commit `THIRD-PARTY-LICENSES.md` and have CI regenerate +
`git diff --exit-code` to catch staleness - was implemented first, then
abandoned when testing showed **cargo-about's output is not byte-stable**: two
warm, back-to-back runs on the same machine differed (9624 vs 9602 lines; a
run-to-run ~20-line delta, sometimes whole `windows-*` crate blocks). A
committed copy would fail the diff check on every run.

Pivoted to: generate the manifest fresh at build/release time into `credits/`
(the release macOS/linux/windows/web jobs and the deploy-page job each run
`cargo about generate` before bundling), and gitignore it. It is therefore
always in sync with the shipped graph and can never go stale. CI's `licenses`
job runs the same generation to a temp file - and cargo-about exits non-zero on
any license outside `about.toml`'s accepted set - so a copyleft/unknown dep is
caught on the PR that adds it. This matches the task's "wire generation into the
build/release" wording better than a committed snapshot anyway.

## Correctness checks

- **First-party exclusion**: nova_* crates + the root binary marked
  `publish = false` (+ workspace MIT license); `about.toml`'s `[private] ignore`
  drops them. Verified: `grep -c nova_` on the manifest = 0.
- **License set**: current graph resolves to MIT (495), Apache-2.0 (23),
  Unicode-3.0 (22), Zlib (6), BSD-3 (4), MIT-0 (4), ISC (3), BSD-2 (1), CC0 (1),
  MPL-2.0 (1) - all permissive. MPL-2.0 (`option-ext`) is weak file-level
  copyleft, satisfied by shipping its text (which the manifest does) since we
  don't modify the crate. No GPL/LGPL/AGPL.
- **Config gate works**: the first run FAILED on `MIT-0` (encase) until added -
  proof the accepted-list gate actually rejects unlisted licenses.
- **Cargo.toml changes are metadata-only** (`license`, `publish = false`) - do
  not affect the build; `cargo metadata` validates.
- **Template**: switched to `{{{ }}}` (unescaped) so Markdown output has no
  `&quot;` HTML entities; verified 0 in output.
- All three workflows parse (js-yaml); release has 4 generation steps, deploy 1,
  CI the gate job.

## Risks / notes

- The release/deploy workflow steps are UNTESTED end to end (they run only on a
  tag/dispatch). Mitigated: the identical `cargo about generate` command runs on
  every PR via the CI `licenses` job, so the command + config are exercised; only
  the in-workflow placement (bundling picks up the generated file) is unverified.
- Depends on `taiki-e/install-action` supporting `cargo-about` (it does; widely
  used) and cargo-about being installable in CI.
- The local `credits/THIRD-PARTY-LICENSES.md` on disk is gitignored; the
  `CREDITS.md` reference resolves in shipped builds where both files sit together.
