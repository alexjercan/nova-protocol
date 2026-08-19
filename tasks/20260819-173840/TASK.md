# Delete the vestigial section density field

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: archive,done

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

## Outcome

`BaseSectionConfig.mass` is gone. `base_section` passes a literal `1.0` to
`destructible_body`, so a section's mass is the volume of its authored collider
and nothing can author otherwise. The collider is the authored box, never the
render mesh, so mass-equals-volume is exact.

Construction sites removed:

- `crates/nova_ship/src/sections/base_section.rs` - field, use site, 3 tests
- `crates/nova_ship/src/sections/torpedo_section/bay.rs` (2)
- `crates/nova_ship/src/flight/tests/stop.rs`
- `crates/nova_authoring/src/base_content/sections/standard.rs` (7)
- `crates/nova_authoring/src/base_content/ships/shared.rs` - `base_config()`,
  the site every semantic ship part went through
- `crates/nova_authoring/src/lint_walk.rs`, `crates/nova_assets/src/mod_refs.rs`
  - RON string fixtures (4)
- `crates/nova_modding/src/lib.rs`
- `crates/nova_editor/src/gallery/catalog.rs` - the focus card's `density` row

Generated content: `assets/base/sections/base.content.ron` lost exactly 32
`mass: 1.0` lines and nothing else. No other generated file moved.

### Two things the filing did not predict

- The example mod authored a NON-1 density.
  `assets/mods/example/example.content.ron` gave `example_plated_hull_section`
  `mass: 1.2` and sold it as "heavier plating than the base hull". That section
  is now the same weight as any other unit cell, so its name, description and
  the teaching comment beside it were rewritten around HEALTH, the axis it can
  still be distinctive on (500, against the overlay's 400 and base's 200).
- The `/wiki/` catalog tables carried a `Mass` column reading `1.0`, which was
  the density under another name - and already wrong for the semantic parts,
  whose colliders are their own box sizes and therefore never massed 1. The
  column is gone from all six pages; the prose says a section weighs the space
  it fills.

### The one place a section density is still needed

`examples/systems/system_attitude_hold.rs` scaled `config.base.mass` by 10 to
build rig B, whose whole job is a 10x-inertia copy of rig A at identical
geometry. It now overrides the live `ColliderDensity` component after spawn
(`Layout::section_density` + `apply_rig_density`). Content cannot say this and
should not be able to; a probe overriding a runtime component can.

Per step 4: `SKIN_DENSITY` (0.25, `shell_skin.rs:637`) and the per-fixture
greeble densities (`skin_decor.rs:501`, e.g. 0.1) are deliberately NOT 1 and
were left alone. `destructible_body(health, density)` keeps its parameter.

### Verification

`cargo check --workspace --all-targets`, `cargo fmt --all -- --check`,
`content lint` (0 errors), `--lib` for nova_ship / nova_authoring / nova_modding
/ nova_editor / nova_assets, the wasm clippy pass, and `web/ npm run ci`, all
green. `system_attitude_hold` was RUN headless under `NOVA_AUTOPILOT=1`: four
invariants pass, including the 10x-inertia one, exit 0.

### Next time

The field shipped in v0.10.0, so this is a real modder break and earns a
CHANGELOG entry. Worth noting that a knob whose every authored value is the
default still costs a docs sweep across four surfaces - the field was cheap to
delete and the prose around it was not.
