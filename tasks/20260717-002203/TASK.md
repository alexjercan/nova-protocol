# Docs + CHANGELOG for base-as-normal-mod / canonical scheme model (Option A)

- STATUS: OPEN
- PRIORITY: 47
- TAGS: v0.7.0,modding,docs

## Goal

Document the canonical namespaced asset-ref model and that base is now a normal,
self-contained mod: base ships its art under `assets/base/`, references it with
`self://`, and mods reference base art with `dep://base/<path>`; a bare
scheme-less asset ref is no longer valid in content.

## Steps

- [ ] web/src/wiki/dev/guide-make-a-mod.md: replace the "bare = base game"
      guidance with the canonical model - every asset ref carries a scheme;
      `dep://base/<path>` reaches base art; `self://` is own folder;
      `dep://<id>/` is a declared dependency. Update the worked example.
- [ ] web/src/wiki/dev/modding-ron.md: same, in the data-model reference.
- [ ] docs/design/mod-binary-resources.md: add the Option A section - base as a
      normal bundle (art under assets/base/), `dep://base` as the implicit
      universal dep, the canonical bare-ref ban and where it is enforced
      (static: content_lint + portal), and WHY A was chosen over B (link the
      spike).
- [ ] CHANGELOG.md: a player/author-facing line (mod authors: reference base art
      with `dep://base/...`, not bare paths).
- [ ] Grep the wiki/docs for stale "bare path == base" statements and fix them
      (keeping-docs-in-sync).

## Notes

- Depends on task 20260717-002105 (migration) and 20260717-002133 (lint) so the
  docs describe the shipped behavior.
- Spike: tasks/20260716-235458/SPIKE.md (Option A).
