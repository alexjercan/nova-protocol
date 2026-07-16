# Consolidate demo + variety into ONE self-contained 'example' tutorial mod (a little of everything)

- STATUS: CLOSED
- PRIORITY: 47
- TAGS: v0.7.0,modding,example,content,docs

## Outcome (2026-07-16)

Shipped `assets/mods/example/` as the single copy-me tutorial mod (bundle +
content + README + moved skybox/.meta + rock texture). It demonstrates: a
Section overlay of `reinforced_hull_section` (health 400, renamed) AND a new
`example_plated_hull_section`; a playable `example_arena` (base prototypes +
both mod sections, a range beacon, two self://-textured targets, an objective,
a `destroyed > 1` win gate, two StoryMessage beats, Victory + Defeat Outcomes);
`self://` skybox + `.meta` + rock texture declared in `resources`; and a
`menu_backdrop`-flagged `example_menu` scene. Removed `assets/mods/{demo,
variety}/`, repointed `mods.catalog.ron` (installed count 3 -> 2).

Swept every live reference: renamed `tests/demo_scenario.rs` ->
`example_scenario.rs` (counts 3 -> 2, ids/meta repointed), updated
`mod_cache_install.rs` (4 -> 3, downloaded-row index shift) and
`mod_binary_resources.rs` (example_arena self:// paths), `mod_refs.rs`
fixtures, a `nova_assets` doc comment; and the four wiki docs
(guide-make-a-mod, guide-author-section, modding-ron, modding), the
binary-resources design doc, the gauntlet content comment, and CHANGELOG.

Review APPROVEd round 1 (independent out-of-context pass). Two self-review
catches: menu orbiter pointed at a stale `menu_planetoid` id (fixed); a stale
"Variety Demo" fixture label (fixed). Left generic `nova_mod_format`/`nova_menu`
decoder test doubles (invented authors, a fabricated `reel` mod) untouched -
they load nothing removed. Verified: content_lint clean, workspace check
--all-targets, fmt, and nova_assets example_scenario 14 / mod_binary_resources 2
/ mod_cache_install 7 / mod_refs 7 all green.


## Goal (user direction 2026-07-16)

Replace the scattered example mods (`assets/mods/demo/` - a section overlay +
arena scenario; `assets/mods/variety/` - the self:// binary-resources dogfood)
with a SINGLE `example` (tutorial) mod that demonstrates a little of EVERYTHING
a modder can do, works by itself, and is THE copy-me starting point. A newcomer
should be able to copy one folder and see every capability in one place.

## What the one mod should demonstrate (a little of each)

- a `Section` overlay (override a base section by id) AND a new section;
- a new playable `Scenario` (spawn a ship from base prototypes, a couple of
  targets, an objective, a win condition);
- mod-relative binary resources: ship its own skybox (+ `.meta`) and a texture,
  referenced via `self://` (folds in what `variety` proves today);
- a menu_backdrop-flagged scene (mods can ship menu ambience);
- ideally one StoryMessage/comms beat and one Outcome (Victory/Defeat) so the
  newer actions are shown too;
- concise inline comments that teach, since this is the reference example.

## Migration / cleanup

- Fold the `variety` dogfood (self:// skybox + asteroid texture) into this mod;
  remove `assets/mods/variety/` and (if replaced) `assets/mods/demo/`, updating
  `assets/mods.catalog.ron`.
- Sweep every reference to `demo`/`variety` (the mod-authoring guide, modding-ron
  reference, README files, tests that name `demo_mod_arena` / `variety_pack_showcase`,
  the "copy-me example" pointers) and repoint them at the new `example` mod
  (sweep-then-delete, keep-docs-in-sync-with-code). Note: several tests assert
  the installed-mod COUNT and specific ids (mod_cache_install.rs,
  demo_scenario.rs) - update them together.
- Keep the CHANGELOG honest about the consolidation.

## Notes

- Supersedes the placeholder-art dogfood shape from 20260716-123544; the real
  variety-pack ART sourcing (20260716-205214) still applies - its assets can
  live in this example mod.
- Decide whether `demo` stays as a second, minimal example or is fully removed
  (does-the-old-element-survive): the user wants ONE, so default to removing
  both and shipping only `example` unless a reason to keep a second surfaces.
