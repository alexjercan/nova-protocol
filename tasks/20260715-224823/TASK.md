# Sync modding wiki guides + mod metadata to the now-playable scenarios

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: docs, web, modding

## Goal

After the gauntlet and demo-arena mods gain real gameplay, bring the modding
docs and player-facing metadata in line so nothing still describes them as
trivial static-beacon demos. The wiki guide embeds the demo-mod RON verbatim,
so it drifts the moment the mod changes.

## Steps

- [x] Update `web/src/wiki/dev/guide-make-a-mod.md`: the embedded demo-mod RON
      block (currently the lone-beacon `demo_mod_arena`) and its surrounding
      prose ("introduces `demo_mod_arena` (add)", the gauntlet template section)
      must match the new content, OR be clearly framed as a trimmed excerpt with
      a pointer to the full file. Do not leave a full-but-stale copy.
- [x] Sweep the rest of the wiki/docs for descriptions of these two mods and
      update any that now read as false: `web/src/wiki/dev/modding-ron.md`
      (catalog snippet mentioning gauntlet), `guide-extend-scenarios`,
      `scenario-system.md`, and any devlog. Consider promoting the new gauntlet
      to the worked "sequential objectives" example in guide-extend-scenarios.
      (Lesson `sweep-then-delete`: grep the whole `web/src/wiki` + `docs` tree
      for `gauntlet`, `demo_mod_arena`, "lone beacon", "arena scenario".)
- [x] Confirm the mod `meta.description` and scenario `description` fields set
      in the two gameplay tasks read well in the Mods menu / details panel
      (these were edited in tasks 20260715-224803 / 20260715-224812; this task
      only verifies consistency, it does not re-edit the RON unless a mismatch
      is found).
- [x] If the wiki has a generated/registered page list (`WIKI_DOC_PAGES` in
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

## Outcome

What changed (`web/src/wiki/dev/guide-make-a-mod.md` only):
- The embedded `demo_mod_arena` snippet description now reads "A shooting gallery
  added by the demo mod: destroy the three derelict rocks." (was "A scenario
  added by the demo mod."), and its `events: [ /* ... */ ]` placeholder now
  carries a comment describing the gameplay shape (player + 3 targets ->
  per-target OnDestroyed counter -> one-shot OnUpdate win).
- The overlay-semantics prose now notes the arena is "a small but playable
  shooting gallery, so the example doubles as a worked scenario".
- The gauntlet publish-template description now reads "A slalom race: fly your
  ship through the beacon gates in order, START to FINISH." (was "A beacon
  slalom course: thread the gates from start to finish.").

Sweep results (nothing else needed changing):
- `guide-extend-scenarios.md` and `guide-author-scenario.md` do not reference
  either mod - no stale text. Promoting the gauntlet into guide-extend-scenarios
  as a worked "sequential objectives" example was left out as scope expansion
  (the task said "consider"): that guide has its own examples and adding one is a
  feature, not a sync.
- `modding-ron.md`'s "gauntlet" is an illustrative `installed.mods.ron`
  cache-record (id/version/bundle), not a gameplay description - accurate.
- `guide-author-section.md`'s demo-mod section overlay text is unchanged and
  still accurate (the overlay was untouched by task 224812).
- `web/dist/**` copies are generated build output, not hand-edited.

Consistency check: the guide's gauntlet template description matches the shipped
`webmods/gauntlet/gauntlet.bundle.ron` meta; the demo bundle meta ("...adds an
arena scenario.") was deliberately left unchanged (demo_scenario.rs asserts it
verbatim and it stays true), so the guide's line 34 still matches reality.

No new wiki page needed: only existing pages were edited, so no `WIKI_DOC_PAGES`
/ `wiki-pages.ts` registration change. The web build was not run locally
(deferred to CI per the standing local-build policy); the edits are prose within
existing code fences with no link, fence, or page-registration changes.
