# Delete the vestigial section density field

- STATUS: OPEN
- PRIORITY: 35
- TAGS: v0.11.0, cleanup, ship

Epic: `20260818-220812`. Owner: "density should always be `1` for spaceship
sections, they should only depend on volume; we should not expose the density at
all."

## CORRECTED 2026-08-19 - this task was filed on a misread

The original filing claimed authored ships carried densities of 70-350 against a
default of 1.0, giving the cargoa roughly 660x the inertia of a same-size
standard hull, and that fixing it required a full thrust re-pass. **All of that
was wrong.**

Those numbers are HEALTH. `part()` in
`crates/nova_authoring/src/base_content/ships/shared.rs:101` is
`part(id, prototype, mesh, origin, bbox_min, bbox_max, health, role)`, and
`PartSpec` (`:32-41`) has no mass or density field at all. The 350 on a cargoa
pod is 350 hit points.

Verified: **every section in shipped content is `mass: 1.0`** - all 32 occurrences
in the generated `assets/base/**/*.content.ron`. The three blank `mass:` hits are
gravity-well masses on anchors and planetoids, an unrelated field.

So sections ALREADY behave the way the owner wants: density 1, mass derived from
collider volume by avian. Nothing is mis-authored and no ship needs retuning.

## What is actually left

One vestigial field. `SectionConfig.mass`
(`crates/nova_ship/src/sections/base_section.rs:229`) is passed to
`destructible_body(health, density)`
(`crates/nova_gameplay/src/integrity/health.rs:163`) and becomes
`ColliderDensity`. It is named `mass`, it means density, and it is 1.0
everywhere it appears.

The docs around it already say DENSITY in three places
(`base_section.rs:46`, `shell_skin.rs:82`, `:1257`), which is how a reader is
meant to survive the name. Deleting the field removes the trap outright.

## Steps

1. Delete `SectionConfig.mass`. Sections pass `1.0` at the one
   `destructible_body` call in `base_section.rs:375`.
2. Grep every construction site - `nova_authoring`, `nova_ship`, examples,
   tests, `torpedo_section/bay.rs:334,359`.
3. Run `cargo run content -- gen`. The field disappears from the generated RON;
   nothing else may move, since every value was already 1.0.
4. Leave `skin_decor.rs:501` and `shell_skin.rs:637` alone unless they too are
   always 1 - `SKIN_DENSITY` is a deliberate named constant and cladding is
   allowed to differ from structure. Check and record which.

## Not in scope

Asteroids and anchors keep their own mass handling. This is about SECTIONS.

## Done when

- `SectionConfig` has no mass or density field.
- `content -- gen` produces RON identical except for the removed field.
- No caller sets a section density anywhere, and the type makes it impossible.
