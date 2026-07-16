# Asset variety pack: themed skyboxes, asteroid/planet texture variants, alternate hull model

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.7.0,art,assets,spike


## Goal

Scenarios stop all looking the same. Today the shipped art is 1 asteroid
texture, 3 same-style skyboxes (cubemap, cubemap_alt, cubemap_alt), 1 hull
model and the turret/torpedo part models. Add: at least one THEMED skybox
(so SetSkybox has somewhere interesting to go), asteroid and planet(oid)
texture variants, and at least one alternate hull or section model variant,
wired into the section/scenario content so the v0.7.0 scenarios can use them.

First step when picked up is a sourcing decision (this task keeps its spike
tag for that): procedural generation, curated CC0 packs, or hand-made in
Blender (art/ holds sources today) - including how art licensing/attribution
is recorded (cargo-about covers code only; credits/ needs an art answer).

## Notes

- Spike: tasks/20260716-122954/SPIKE.md (v0.7.0 release scope)
- Plan: docs/plans/20260716-v0.7.0-plan.md, strand 1
- Consumers: vertical slice (20260708-203659), scenario pack
  (20260716-123535), picker thumbnails (20260715-220011).

## Mods must be able to ship resources (added 20260716, user direction)

This task also owns the PIPELINE gap, not just the art: mods currently cannot
carry their own binary assets. A bundle manifest lists only `content` RON
files, and every asset reference in a mod resolves against the BASE game's
assets/ (gauntlet's skybox is "textures/cubemap.png" - a base file). Extend
the mod pipeline so a bundle can include resource files (PNG textures,
skyboxes, GLB models, audio) as members and its content can reference them
with mod-relative AssetRef paths, linked to the owning mod:

- bundle manifest/member model: declare (or auto-include) resource files next
  to the content list; nova_portal_gen already walks and copies EVERY file
  verbatim with size+sha256, so distribution mostly works - validation should
  reject references to files that are not members;
- game side: resolve mod-relative AssetRef paths against the mod's own folder
  (shipped assets/mods/<id>/, and the mods:// source for downloaded bundles),
  native AND web;
- dogfood: the new skybox/texture variants from this task load from a mod,
  not just from base assets/ - consumers are the story campaign
  (20260716-123535) and Gauntlet 2.0 (20260716-124722) wanting their own look.

## PIPELINE half LANDED 20260716 (feature/mod-binary-assets)

The mod-binary-resources pipeline is implemented and merged:

- `BundleManifest.resources: Vec<String>` declares the binary files a bundle
  ships; content references them with the reserved `self://<path>` scheme.
- `self://` refs rewrite to the owning mod's folder at merge time
  (`mods/<id>/` shipped, `mods://<id>/` downloaded), native + web, via a generic
  serde-value rewrite that covers every AssetRef field.
- Membership validation in all three domains (portal generator, static
  `content_lint`, runtime content gate): a `self://` ref must name a declared
  resource.
- Dogfood: the shipped `variety` mod (`assets/mods/variety/`) renders from its
  own skybox + asteroid texture (PLACEHOLDER art).
- Design + rationale: docs/design/mod-binary-resources.md.

REMAINING (the ART half): real skybox/asteroid/planet/hull art to replace the
placeholders is split into task 20260716-205214 (user-owned sourcing decision).
This umbrella keeps its spike tag for that fork.
