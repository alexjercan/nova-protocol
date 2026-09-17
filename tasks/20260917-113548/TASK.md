# Clean base content ownership and nova_authoring structure

- STATUS: CLOSED
- PRIORITY: 100
- TAGS: v0.14.0, refactor, content, examples, docs

## Claim

`nova_authoring` mixes shipped base-game content with example and bench fixtures,
exposes deliberately overpowered set-piece weapons in the player catalog, uses
inconsistent runtime-ID ownership, and carries lore-heavy comments that obscure
mechanical constraints. Make base content contain only supported game content,
move reusable development fixtures to development-owned support, split section
authoring by section family, and make comments explain mechanics only.

## Evidence

- `base_content/sections/standard.rs` registers the deliberately overpowered
  siege railgun and `heavy_torpedo_section`; both are visible in the editor.
- `base_content/sections/ordnance.rs` defines `breaker()`, used only by the
  heavy bay. The menu duel is the only required base scenario consumer.
- `block_warship` and its weapon IDs serve examples and a bench fixture, not a
  shipped base scenario.
- Carrier, skiff, tug, claw, and cleanup leader designs serve examples or bench
  fixtures. The four carrier wreck designs have no consumer outside the base
  catalog and exports.
- The raider catalog entry is not needed by a base scenario; the menu duel
  builds its one-off raider through `inline_raider()`.
- `ships/block.rs` mixes duplicated prototype strings, local constants, and
  aliases such as `const DOCKING_PORT = DOCKING_PORT_SECTION_ID`.
- `sections/standard.rs` holds every section family while `ordnance.rs` alone
  is split out.
- Lore-heavy rustdoc is concentrated in ships, styles, sections, scenarios,
  and scenario helpers. It describes factions, crews, backstory, and story
  meaning instead of code constraints.

## Decisions

- Remove the Breaker bay and siege railgun from the base catalog.
- Define the complete Breaker bay inline in the menu duel.
- Move the stolen warship and siege railgun configuration to shared example
  fixture support.
- Move raider, carrier, skiff, tug, claw, and cleanup leader catalog designs
  to shared example fixture support. Keep only the raider's private base builder
  for the menu duel. Move the placeholder style there too.
- Delete the four unused carrier wreck designs.
- Remove the raider catalog entry, but keep its private builder for the menu
  duel.
- Keep cutter, hauler, workship, frame tender, gunship, and picket in base.
- Keep all balanced player-buildable sections, including cargo and tank hulls,
  Lance and Serpent bays, the standard railgun, and every PDC variant.
- Keep `pdc_twin_pierce_turret_section`; the twin PDC family is game content.
- Use canonical section constants directly. Do not add alias constants.
- Add constants only when an ID is used more than once. Keep one-use authored
  IDs as string literals.
- Split all section families. Replace `ordnance.rs` with `torpedo_bay.rs`.
- Comments and rustdoc explain mechanics only. Lore belongs in
  `web/src/lore/`, not code commentary.

## Work plan

1. Add shared Rust fixture support under `examples/shared/fixtures/`, usable by
   playable, systems, and screenshot examples without adding a production crate
   dependency or packaging fixture content as base assets. Register reusable
   fixture ships, fixture sections, and the placeholder style there.
2. Give loose bench scenarios bench-owned inline fixtures or a bench-owned
   fixture bundle. Bench RON must not depend on example code.
3. Move the raider, carrier, warship, skiff, tug, claw, cleanup leader,
   placeholder style, and siege railgun configuration out of base content.
4. Update every affected example, screenshot, probe, and bench fixture. Remove
   base-content exports that existed only for those consumers.
5. Delete the carrier wreck designs and their IDs. Remove the raider catalog
   ID and entry while retaining the private inline-raider builder.
6. Remove `heavy_torpedo_section`, `breaker()`, the siege railgun prototype,
   and `SIEGE_RAILGUN_LANCE_SECTION_ID` from `nova_ship`.
7. Author the menu duel's Breaker bay as `SectionSource::Inline`, including its
   torpedo type, bay mechanics, sounds, art, collider, links, and animations.
8. Update or replace siege-specific section tests and the editor gallery's
   synthetic hidden-prototype fixture.
9. Normalize section prototype references. Existing externally shared IDs stay
   in the lowest shared crate. Authoring-only IDs live beside their builders.
10. Add local constants for repeated scenario IDs, entity IDs, section IDs,
    order IDs, timer keys, variables, model node names, and other typo-sensitive
    references. Do not constantize one-use strings.
11. Split sections into `hull`, `controller`, `thruster`, `turret`,
    `torpedo_bay`, `railgun`, and `docking_port`. Keep catalog assembly and
    genuinely shared helpers in `sections/mod.rs`; colocate family tests.
12. Regenerate base content. Inspect all generated section, ship, style, and
    scenario changes. Never hand-edit generated base RON.

## Comment and rustdoc cleanup

Apply this policy across all of `crates/nova_authoring/src`, including tests:

- Keep invariants, units, geometry, coordinate systems, data ownership,
  ordering, safety constraints, runtime behavior, failure modes, and reasons
  for non-obvious values.
- Keep visual constraints only when they specify measurable rendering,
  framing, contrast, recognition, or placement behavior.
- Remove backstory, faction identity, character interpretation, imagined
  history, narrative motivation, and explanations of what a scene means.
- Rewrite mixed comments to preserve only their mechanical constraint.
- Remove stale task citations and implementation-history narration.
- Do not replace removed lore with shorter lore.
- Keep authored names, descriptions, and dialogue that are actual game data;
  comments must not interpret or expand them.

Primary cleanup areas:

- `base_content/ships/block.rs`: hull geometry, mount placement, and link
  constraints remain; fleet, crew, faction, campaign, and warship lore goes.
- `base_content/ships/mod.rs`: ID docs state identity and runtime consumers.
- `base_content/sections/`: retain dimensions, balance, firing behavior, and
  catalog contracts; remove fictional weapon roles and campaign narration.
- `base_content/styles.rs`: retain placement, density, orientation, contrast,
  and measured silhouette constraints; remove shipyard, crew, commerce,
  faction, and style-"voice" stories.
- `base_content/scenarios/`: retain event sequencing, gates, camera framing,
  and failure prevention; remove plot summaries from implementation comments.
- `scenario_helpers.rs`: document channel/color behavior and caller contracts,
  not fictional speaker taxonomy.
- Tests state the invariant they prove, not a lore justification.

## Magic-string policy

- Runtime IDs shared by a producer and consumer are constants in the lowest
  existing shared module or crate.
- Repeated local runtime IDs use private local constants.
- One-use authored IDs remain literals.
- Generated IDs, display prose, asset paths, and isolated synthetic test IDs
  remain strings.
- Literal expected IDs may remain in tests that intentionally pin the serialized
  public contract.
- RON runtime IDs remain strings because that is the content format.

## Documentation

- Update `web/src/create/base-content.md`, `web/src/create/sections.md`,
  `web/src/create/ships.md`, and `web/src/create/styles.md` for removed base IDs
  and moved development fixtures.
- Check the player section pages for Breaker, siege railgun, placeholder style,
  and removed ship claims; remove invalidated text rather than documenting
  unshipped compatibility.
- Check the dev book for module paths invalidated by the section-family split.
- Keep approved lore in `web/src/lore/`; remove code-comment duplication.
- Add concise `[Unreleased]` changelog entries for the player-visible catalog
  cleanup and the internal authoring cleanup. Mark any shipped content-format
  break as breaking if the release baseline requires it.
- Inspect generated documentation output after building it.

## Blast radius

- `nova_authoring` base builders, exports, generation, parity, catalog tests,
  balance tests, and lint references.
- `nova_ship` catalog IDs.
- Generated base section, ship, style, and scenario RON.
- Editor gallery fixtures and any editor catalog assumptions.
- Playable, systems, and screenshot examples that load moved catalog ships or
  siege equipment.
- Loose bench scenarios that reference moved ship prototypes.
- Player wiki, creator docs, dev book, rustdoc, and changelog.
- Engine comments that call moved fixture hulls shipped reference hulls.

## Verification

- Before edits, record the exact affected consumers and preserve the current
  dirty working-tree changes.
- Run `nix develop --command cargo run content gen` and inspect generated diffs.
- Run `nix develop --command cargo run content lint`.
- Run affected `nova_authoring` catalog, generation, lint, and RON parity tests.
- Build and run each migrated example and each affected loose bench scenario.
- Run the applicable probe assertions; update outcome registrations if an
  example changes ownership or identity.
- Render the menu duel and verify the inline Breaker launches, defeats the
  victor, completes the backdrop cycle, and is absent from the player catalog.
- Verify the editor still lists every retained balanced section and does not
  list the removed siege weapons or placeholder style.
- Run `nix develop --command mdbook build` and inspect its output.
- Run `cd web && npm run ci` and inspect the generated documentation pages.
- Do not use full workspace tests or Clippy unless separately requested.

## Known consumers to migrate

The worker must repeat the reference search before editing. At review time the
known direct consumers were:

- Bench: `arsenal.content.ron`, `hunt.content.ron`, and
  `slingshot.content.ron` under `crates/nova_bench/scenarios/`.
- Playable: `first_shift_map.rs` and `first_shift_ships.rs`, plus any consumer
  found by the fresh reference search.
- Screenshots: `lesson_combat_battery.rs`, `lesson_combat_field.rs`,
  `lesson_combat_lance.rs`, `loop_goto_standoff.rs`,
  `loop_torpedo_blast.rs`, `screenshot_hud_shell.rs`,
  `screenshot_railgun.rs`, `shared/hollow.rs`, and `shared/kit.rs`.
- Systems: `stress_hull_collapse.rs`, `system_ai_combat.rs`,
  `system_ai_evade.rs`, `system_ai_patrol.rs`,
  `system_collision_damage.rs`, `system_flight_legs.rs`,
  `system_gravity_wells.rs`, `system_helm_orders.rs`,
  `system_hud_scales.rs`, `system_hud_shell.rs`,
  `system_hull_scaling.rs`, `system_railgun_hulls.rs`,
  `system_torpedo_capital.rs`, and `system_wreck_lock.rs`.
- Non-authoring comments and tests that describe these as shipped reference
  hulls must be corrected when their claim becomes false.

## Acceptance criteria

- Base section IDs do not include `heavy_torpedo_section` or
  `siege_railgun_lance_section`; all retained balanced section IDs still exist
  and remain editor-visible as before.
- Base ship IDs include only current base consumers. They do not include the
  raider, carrier, warship, skiff, tug, claw, cleanup leader, or carrier wrecks.
- Base style IDs do not include `placeholder`; retained styles are unchanged.
- `SIEGE_RAILGUN_LANCE_SECTION_ID`, `breaker()`, `stolen_warship()`, and the
  removed public ship and weapon IDs have no production definition or export.
- The menu duel owns a fully inline Breaker bay and has no prototype reference
  to either removed siege section ID.
- Every known example still receives each fixture it uses from shared example
  support. No example relies on a moved base catalog ID being present.
- Every affected bench scenario resolves using bench-owned content. It does not
  depend on `examples/` or on a removed base catalog ID.
- `sections/standard.rs` and `sections/ordnance.rs` are gone. Each retained
  section family and its tests live in the agreed family module.
- Repeated runtime IDs use one canonical constant; no constant aliases another
  constant solely to rename it. One-use authored IDs remain literals.
- A manual review of all `nova_authoring` comments finds only mechanical,
  rendering, or authoring constraints under the stated comment policy.
- Generated base RON contains no removed ship, section, or placeholder-style
  entry and matches the Rust builders byte for byte.
- Creator docs contain no catalog row for a removed base ID. Current player and
  developer docs make no false shipped-content or module-path claim.
- The changelog describes the final user-visible delta once, against the last
  release baseline, without preserving removed unreleased behavior.
- All listed verification checks pass, or the task records the exact blocked
  proof and leaves no unverified success claim.

## Worker handoff

- Start from the current working tree. Run `git status --short` and inspect
  existing diffs before editing. Do not discard or overwrite unrelated work.
- Follow the repository Pair, Verify, Content, Probe, and Docs skills as each
  phase applies. Do not use `nova-review` for this bounded cleanup.
- Keep this as one task. Store decisions, proof, generated-diff review, and any
  blocked verification in this task before requesting validation.
- Do not close the task. The reviewing agent will validate the implementation,
  evidence, generated output, documentation, and comment-policy sweep, then
  close it if complete.
