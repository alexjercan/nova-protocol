# Move the design PoC HTML pages out of examples/ui into web/design

- PRIORITY: 55
- TAGS: v0.10.0, web, docs, refactor
- KIND: STORY
- ACTIVITY: PLANNING
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-115955

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

- [ ] Move the three files to `web/design/` (git mv; keep contents byte-identical).
- [ ] Update `web/webpack.config.js` copy entries and `web/tests/theme.test.ts`
      `POC_PATH`.
- [ ] Update the source comments that cite them normatively:
      `crates/nova_ui/src/theme.rs`, `crates/nova_ui/src/hud.rs`,
      `crates/nova_gameplay/src/hud/{emphasis,situation,objective_stack}.rs`,
      `crates/nova_gameplay/src/hud/nova_os/{content,style}.rs`, and
      `crates/nova_gameplay/src/hud/nova_os/tests/structure.rs`.
- [ ] Update `web/src/style.css`'s header comment and the wiki pages
      `web/src/wiki/dev/development.md` + `web/src/wiki/dev/keeping-docs-in-sync.md`.
- [ ] Confirm no reference to the old paths survives anywhere.

## Definition of Done

- No `examples/ui/*.html` remains and nothing references the old paths.
  (cmd: `! rg -n "examples/ui/.*\.html" --glob '!tasks/**' .`)
- The theme drift test still reads the moved token source and passes.
  (cmd: `cd web && npm test`)
- The site build still emits the three pages.
  (manual: build the site and open the three copied pages)

## Notes

- Pure move + reference update. No content change to the HTML, the theme, or
  the site styling; a token diff here would be a separate decision.
- `examples/ui/turret_section/` and other non-`.rs` payloads are out of scope.
