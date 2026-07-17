# Shipped art-pack dogfood for cross-mod dep:// refs

- STATUS: CLOSED
- PRIORITY: 30
- TAGS: v0.7.0,modding,content,assets,dogfood,wonto

## Context

Task 20260716-215423 landed the `dep://<id>/<path>` scheme (reference a declared
dependency's shipped resources) and proved it end to end with SYNTHETIC bundles +
unit tests. What it deliberately did NOT do - because it ripples through
installed-count assertions and wants real art - is ship an actual dogfood: a
shared "art pack" mod plus a consumer mod that `dep://`-references it. See
docs/design/mod-binary-resources.md ("Cross-mod references").

## Goal

Ship, in the repo, a real shared-resource mod and a consumer that references its
resources via `dep://`, so the cross-mod path is exercised by the SHIPPED catalog
the way `assets/mods/variety/` dogfoods `self://` today.

## Steps

- [ ] Add a shipped "art pack" mod (e.g. `assets/mods/art_pack/`) that ships a
      texture/skybox as a declared `resources` member and no scenario of its own
      (or a minimal one).
- [ ] Add / adapt a consumer mod that declares `dependencies: ["art_pack"]` and
      references the pack's resource with `dep://art_pack/<path>` in a scenario.
- [ ] Register both in `assets/mods.catalog.ron`.
- [ ] Sweep and fix the installed-count assertions this shifts (demo_scenario.rs,
      mod_cache_install.rs, menu row-index tests - the same spread task
      20260716-123544 hit when adding `variety`). Do a full untruncated sweep
      first (the `truncated-sweep-is-not-a-sweep` lesson).
- [ ] Add an integration assertion that the consumer's merged scenario resolves
      its `dep://` ref against the pack's folder (`mods/art_pack/...`).
- [ ] The static `content_lint` CI gate must stay green over the new tree.

## Notes

- Depends on the landed `dep://` pipeline (task 20260716-215423).
- Consider whether the pack should also be published to the portal (`webmods/`)
  to dogfood the DOWNLOADED cross-mod path, or leave that to a further task.


## Reason

We already ship base as an art pack
