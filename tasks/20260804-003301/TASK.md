# Move the design PoC HTML pages out of examples/ui into web/design

- STATUS: CLOSED
- PRIORITY: 86
- TAGS: v0.10.0, web, docs, refactor

## Story

`examples/ui/` holds three HTML files that are not examples at all:
`nova_ui_rework_poc.html`, `hud_rework_poc.html`, `nova_os_terminal_poc.html`.
They are accepted DESIGN SOURCES - `nova_ui_rework_poc.html`'s `:root` block is
the single source of truth for both `crates/nova_ui/src/theme.rs` and
`web/src/style.css`, and `web/tests/theme.test.ts` fails when the two drift.
`web/webpack.config.js` copies all three into the built site.

They sit in an examples category that is about to get a contract ("a `ui/`
example proves the live UI tree"), which they can never satisfy. Move them to
`web/design/` and update every reference.

## Steps

- [x] `git mv` the three `.html` files from `examples/ui/` to `web/design/`.
      Contents stay byte-identical - no reformat, no token edit.
- [x] Repoint the two functional readers:
      - `web/webpack.config.js:355,359,367` - `from:` is resolved against the
        webpack context, which is the cwd `web/` (no `context:` key is set on
        the `CopyPlugin` or the config), so `../examples/ui/X.html` becomes
        `design/X.html`, NOT `../design/X.html`. Also fix the comment above the
        block (line ~346) that says the PoCs "live in `examples/ui/`".
      - `web/tests/theme.test.ts:23` - `POC_PATH` becomes
        `join(REPO, "web", "design", "nova_ui_rework_poc.html")`, and the file's
        header comment (line 3) cites the same new path.
- [x] Repoint the path-bearing comments. Only these cite a PATH:
      `crates/nova_ui/src/theme.rs:5`, `crates/nova_ui/src/hud.rs:7`,
      `crates/nova_gameplay/src/hud/{emphasis.rs:4,objective_stack.rs:4,situation.rs:4}`,
      `examples/screenshots/screenshot_nova_os.rs:6`, and
      `scripts/gen-nova-os-sfx.py:5`. Leave
      `crates/nova_gameplay/src/hud/nova_os/{content.rs,style.rs,tests/structure.rs}`
      alone - they cite the bare filename or "the PoC", which stays true.
- [x] Repoint the docs: `web/src/style.css:16`,
      `web/src/wiki/dev/development.md:147,379`,
      `web/src/wiki/dev/keeping-docs-in-sync.md:60`, and `CHANGELOG.md:98`
      (a stale pointer, not a history rewrite - the file still exists).
- [x] Run the DoD proofs; confirm the repo-wide grep is clean.

## Definition of Done

- No `examples/ui/*.html` remains and no old path survives anywhere.
  (cmd: `! rg -n "examples/ui/.*\.html" --glob '!tasks/**' .`)
- The move is a pure rename - no byte of HTML changed.
  (cmd: `git diff -M --summary HEAD | rg -c "rename .*\(100%\)"` -> 3)
- The theme drift test reads the moved token source and passes.
  (cmd: `cd web && npm test`)
- The site build still emits the three secret routes.
  (cmd: `cd web && npm run build && test -f dist/nova-menu/index.html &&
  test -f dist/nova-hud/index.html && test -f dist/nova-os/index.html`)
- The web CI gate is green (prettier does not reach `web/design/`; its globs
  are `src/**` + `tests/**`).
  (cmd: `cd web && npm run ci`)

## Notes

- Pure move + reference update. No content change to the HTML, the theme, or
  the site styling; a token diff here would be a separate decision.
- `examples/ui/turret_section/` and other non-`.rs` payloads are out of scope.
- `Cargo.toml` needs no change: the three `.html` files have no `[[example]]`
  entry. `examples/ui/nova_os_rtt_poc.rs` does, and is retired separately.
- `web/design/` is new and sits OUTSIDE `web/src/`, so `npm run format` /
  `format:check` (globs `src/**/*.html`) and `tsconfig.json`
  (`include: [src/**/*, tests/**/*]`) never touch it - which is what keeps the
  files byte-identical.
- Kept: the filenames stay `*_poc.html`. Renaming them would touch every
  citation and is a separate call.

## Close-out

**What / why.** `git mv`ed the three design-source HTML files from
`examples/ui/` to `web/design/` and repointed all 14 references. The files were
never examples - they are accepted design sources with live consumers - and
`examples/ui/` is about to get a "proves the live UI tree" contract they can
never satisfy. Everything else is a path-string edit; no HTML byte, token, or
style changed.

**Alternatives.** Considered `design/` at the repo root instead of `web/design/`.
Rejected: both functional readers (webpack's `CopyPlugin`, `theme.test.ts`) run
with cwd `web/`, so keeping the sources under `web/` makes both paths local and
removes the test's repo-root derivation entirely.

**Difficulties / diagnosis.**
- Webpack `from:` resolution. Confirmed empirically rather than assumed: no
  `context:` key exists on the `CopyPlugin` or the config root (`rg -n context
  webpack.config.js` -> only two unrelated `redirect` hits), and the sibling
  entry `from: "src/assets"` is already cwd-relative. So the new value is
  `design/X.html`, NOT `../design/X.html`. The build then emitted all three
  routes byte-identical to the sources, which proves it.
- Plan drift, corrected. The plan specified `POC_PATH = join(REPO, "web",
  "design", ...)`, but that leaves `REPO` (`join(WEB, "..")`) with no other
  reader - dead code plus an `eslint no-unused-vars` warn, and a pointless
  `web/../web` round trip. Used `join(WEB, "design", ...)`, dropped `REPO`, and
  retuned the comment above it that explained the repo-root derivation.

**Known wart, deliberately left.** `web/design/hud_rework_poc.html:337` has an
inline comment reading "as an in-repo file:// review copy (examples/ui/)". Its
BEHAVIOUR is still correct - it rewrites asset srcs to `../../assets/...`, and
`web/design/` is the same depth from the repo root that `examples/ui/` was, so
`file://` review still resolves the key glyphs. Only the parenthetical path is
stale. Editing it would break the "no byte of HTML changed" DoD proof, and the
DoD grep (`examples/ui/.*\.html`) does not match it. Left for whoever next
touches that file's content.

**Evidence.**
| Proof | Result |
| --- | --- |
| `! rg -n "examples/ui/.*\.html" --glob '!tasks/**' .` | clean (exit 1) |
| `git diff -M --cached --summary HEAD \| rg -c "rename .*\(100%\)"` | `3` |
| `cd web && npm test` | `site.test.ts` + `theme.test.ts` all assertions passed |
| `cd web && npm run build` + three route files | `ROUTES_EXIT=0` |
| `cd web && npm run ci` | `CI_EXIT=0` (format:check, lint, test, build) |
| `cargo fmt --all --check` | exit 0 |
| built routes vs sources (`diff -q` x3) | identical (`VERBATIM_EXIT=0`) |

`cargo check`/`clippy`/test-suite skipped per standing instruction; the Rust
diff is seven `//!` doc-comment lines, which cannot affect compilation, and CI
covers it.

**Reflection.** The plan's own uncertainty note ("worth confirming against the
existing copy-plugin context") was the one real risk, and it paid off to resolve
it by reading the config and then proving it with a build rather than reasoning
about webpack defaults. The `REPO` leftover is the general lesson: a move task's
plan enumerates path edits, but a path edit can strand the variable that built
the path - grep the identifier, not just the string.
