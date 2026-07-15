# Aggregate Rust dependency licenses into shipped credits (cargo-about)

- STATUS: OPEN
- PRIORITY: 10
- TAGS: backlog,chore,licenses,build

Split out of 20260714-154958 (asset-license audit), which covered shipped
ASSETS only. The shipped native binary statically links Bevy, avian and their
transitive crates (MIT / Apache-2.0); those licenses require the license text to
travel with the binary distribution, but `credits/` currently only carries the
Bevy icon's asset license, not the dependency-code licenses.

Goal: generate a complete dependency-license manifest and ship it in
`credits/` (so it lands in web `dist/credits/` and the native bundle).

Sketch:
- Add `cargo-about` (config `about.toml` + a generated `credits/licenses/` or a
  single `THIRD-PARTY-LICENSES.md`); wire generation into the build/release so it
  cannot go stale.
- Cross-check the accepted license set (MIT, Apache-2.0, BSD, etc.) and flag any
  copyleft/unknown-license dep.
- Point `credits/CREDITS.md`'s "Third-party code" section at the generated file.
