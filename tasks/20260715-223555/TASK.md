# Fix dead examples/03_scenario.rs reference in scenario-system wiki page

- STATUS: OPEN
- PRIORITY: 50
- TAGS: bug,docs,web

From the docs review spike 20260715-223147.

## The bug (verified)

`web/src/wiki/dev/scenario-system.md:26` cites `examples/03_scenario.rs`, which
does not exist. The scenario examples consolidated into `examples/08_scenario.rs`
(the sibling `development.md` documents the merge); `03` is now
`03_hull_section.rs`. `ls examples/` confirms: no `03_scenario.rs`,
`08_scenario.rs` present.

## Steps

- [ ] Change the `examples/03_scenario.rs` reference in scenario-system.md:26 to
      `examples/08_scenario.rs`.
- [ ] Grep the wiki (and docs) for any other `0N_scenario` / stale example
      references and fix them.
- [ ] Verify: `npm run ci` green.

## Notes

Trivial docs fix. The spike also flagged hardcoded example/line refs as a general
drift risk (prefer symbol-based anchors) - that is a separate larger task.
