# Sync modding wiki guides + mod metadata to the now-playable scenarios

- STATUS: OPEN
- PRIORITY: 40
- TAGS: docs, web, modding

## Goal

After the gauntlet and demo-arena mods gain real gameplay, bring the modding
docs and player-facing metadata in line so nothing still describes them as
trivial static-beacon demos. The wiki guide embeds the demo-mod RON verbatim,
so it drifts the moment the mod changes.

## Steps

- [ ] Update `web/src/wiki/dev/guide-make-a-mod.md`: the embedded demo-mod RON
      block (currently the lone-beacon `demo_mod_arena`) and its surrounding
      prose ("introduces `demo_mod_arena` (add)", the gauntlet template section)
      must match the new content, OR be clearly framed as a trimmed excerpt with
      a pointer to the full file. Do not leave a full-but-stale copy.
- [ ] Sweep the rest of the wiki/docs for descriptions of these two mods and
      update any that now read as false: `web/src/wiki/dev/modding-ron.md`
      (catalog snippet mentioning gauntlet), `guide-extend-scenarios`,
      `scenario-system.md`, and any devlog. Consider promoting the new gauntlet
      to the worked "sequential objectives" example in guide-extend-scenarios.
      (Lesson `sweep-then-delete`: grep the whole `web/src/wiki` + `docs` tree
      for `gauntlet`, `demo_mod_arena`, "lone beacon", "arena scenario".)
- [ ] Confirm the mod `meta.description` and scenario `description` fields set
      in the two gameplay tasks read well in the Mods menu / details panel
      (these were edited in tasks 20260715-224803 / 20260715-224812; this task
      only verifies consistency, it does not re-edit the RON unless a mismatch
      is found).
- [ ] If the wiki has a generated/registered page list (`WIKI_DOC_PAGES` in
      `web/webpack.config.js`, `web/src/wiki-pages.ts`), confirm no NEW page is
      needed - this task edits existing pages only. If a new "worked example"
      page is genuinely warranted, register it there.

## Notes

- Relevant files: `web/src/wiki/dev/guide-make-a-mod.md` (embeds demo RON at
  ~lines 25-94 + gauntlet template ~130), `web/src/wiki/dev/modding-ron.md`
  (gauntlet catalog snippet ~line 180), `web/src/wiki/dev/guide-extend-
  scenarios.md`, `web/src/wiki/dev/scenario-system.md`.
- Docs-only task; no Rust/RON behavior changes. The web build/lint (if any)
  should stay green.
- Depends on: 20260715-224803 (gauntlet) and 20260715-224812 (arena) - the docs
  describe their FINAL content, so land after both.
