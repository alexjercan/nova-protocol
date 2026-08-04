# Review: Move the design PoC HTML pages out of examples/ui into web/design

- TASK: 20260804-003301
- BRANCH: refactor/design-poc-to-web-design

## Round 1

- REVIEWER: out-of-context
- VERDICT: APPROVE

- [ ] R1.1 (NIT) web/design/hud_rework_poc.html:337 - the inline comment still
  reads "as an in-repo file:// review copy (examples/ui/)", so the DoD prose
  "no old path survives anywhere" is not literally true; the DoD's grep
  (`examples/ui/.*\.html`) cannot match it, and editing the file would drop the
  rename below 100% and break DoD proof 2. Behaviour is unaffected -
  `web/design/` is the same depth from the repo root that `examples/ui/` was,
  so the `../../assets/...` rewrite still resolves. Change: amend the DoD
  bullet to "no old path survives outside the moved HTML", or fix the
  parenthetical whenever that file's content is next touched. The Close-out
  already discloses this, so it is a wording mismatch, not a hidden defect.
  - Response:

Process signal: Step 2's literal sub-bullet specified
`POC_PATH = join(REPO, "web", "design", ...)`, but the implementation used
`join(WEB, "design", ...)` and deleted the now-readerless `REPO`. That is a
strictly better outcome and the Close-out documents it under "Plan drift,
corrected". The lesson for the plan: a path-move plan enumerates strings, but a
path edit can strand the variable that built the path.

Process signal: NOTES.md's webpack sketch shows `from: "../design/X.html"`,
which is wrong; TASK.md Step 2 has the correct `design/X.html`, and the code
follows the Step. Two records disagreeing on a load-bearing value is a trap for
whoever copies from the sketch.

Verification (primary pass, re-derived independently of the round-1 reviewer):

- Webpack context re-derived from the config, not from the task's claim:
  `rg -n context web/webpack.config.js` finds no `context:` on the config root
  or the `CopyPlugin` (only two `devServer` proxy arrays at 415/422), and the
  sibling entries `from: "src/assets"` / `from: "../assets/input-prompts"` are
  already cwd-relative. `design/X.html` is correct; `../design/X.html` would
  have been wrong. The build emitting all three routes confirms it.
- `! rg -n "examples/ui/.*\.html" --glob '!tasks/**' .` -> clean (exit 1).
- `git diff -M --summary master...HEAD | rg -c "rename .*\(100%\)"` -> `3`;
  the three HTML files are byte-identical renames.
- `cd web && npm run ci` (via `nix develop`) -> exit 0: format:check, lint,
  `site.test.ts` + `theme.test.ts`, build.
- `web/dist/nova-{menu,hud,os}/index.html` all present after the build.
- `theme.test.ts` genuinely reads the moved file (`existsSync` guard then token
  parse); not weakened - the assertion set is unchanged, only the path source.
- Close-out claims checked against the diff: every proof in its Evidence table
  reproduces, and the "known wart" it discloses is exactly R1.1.

Not verified: `cargo check`/`clippy`/Rust tests, skipped per the standing
instruction. The Rust diff is seven `//!` doc-comment lines, which cannot
affect compilation, and CI covers it.

No pending `manual:` proofs - all five DoD proofs are `cmd:`.
