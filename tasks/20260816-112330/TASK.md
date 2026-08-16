# Ship import/export: ships as a first-class content kind

- STATUS: IN_PROGRESS
- PRIORITY: 73
- TAGS: v0.11.0,ship,content,modding,scenario

## Goal

A ship is a TYPE but not a CONTENT KIND. `SpaceshipSectionConfig` is the
per-section authoring record, and every scenario RON inlines the whole section
list for every ship it spawns.

Measured cost on master before the change:

- `lifeline.content.ron` 127 KB, `shakedown_run` 100 KB, `broadside` 59 KB
- 531 KB of scenario RON total, mostly the same three ships copy-pasted
- editing the cargoa means editing it in eleven places

Promote a ship to a content kind resolved by id, the way sections and styles
already are.

## Why it is cheap

`Content` (crates/nova_modding/src/lib.rs:70) says it in its own doc: "Adding a
kind is one variant here plus one router arm downstream." `Style` was added by
`cf61373d` one commit earlier, so a complete worked example of this exact change
is one commit old.

The by-id resolution pattern already exists twice - `SectionSource::Prototype`
against `GameSections`, and `ShipStyle` against `GameStyles`. This is the third
instance, not a new idea. Both log-and-skip on a miss rather than panicking.

## The design call

The ship entry owns what is intrinsic to the hull (section list, style, collapse
threshold). The scenario spawn owns what is per-instance (position, rotation, id,
name, controller, allegiance). An override layer is required because
`cargoa_sections(grade, controller_modifications)` is parameterised today;
`SectionModification` is the precedent to follow one level up.

## Definition of done

- `Ship` resolves from content by id and a mod can author one
- racer, cargoa and cargob are ship entries; no scenario inlines them
- scenario RON size measured before and after
- `content gen` idempotent, `content lint` 0 errors, `content_ron_parity` passes

## Not in scope

Editor save/load and export (later). The skin derivation dump (later).

## Lane

sprout `ship-content`.
