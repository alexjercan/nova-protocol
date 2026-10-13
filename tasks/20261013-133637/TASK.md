# Skin: dead-end sockets link sections but get no skin or greebles

- STATUS: CLOSED
- PRIORITY: 96
- TAGS: v0.16.0, skin

## User facts
- Owner feedback 2026-10-13: "some sections (for example the solar panels) to not have skin plating/greebles on them, for example if I create a "wing" of solar panels they will leave out a single link point on the X axis, would be cool to be able to say that that link point doesn't get a skin, like it's a "dead end" but it can have other sections attached to it, just not skin plating".

## Agent findings
- `solar_panel_section` (86e971480) joins side to side on +-X; an open end socket currently gets skin and greebles. Sockets live on the section prototype (`base_section.rs`); skin clads socket faces (`shell_skin.rs`).

## Agent decision
- Owner asked to work the list without stopping, so the agent chose the shape below; review it here.
- Options considered:
  - A. Per-socket `skin: LinkPointSkinType` (`Clad`, `Bare`), required in RON. Consequence: every socket in base RON, the example mod, the-ledger webmod, the web docs and any saved inline section must add `skin: Clad`; a **(breaking)** change for no new behaviour.
  - B. Per-socket `skin: LinkPointSkinType`, omitted means `Clad`, serialised only when `Bare` (the `hide_in_editor` / `damage_effects` precedent in `base_section.rs`). Consequence: no migration; an author writes only the dead end. CHOSEN.
  - C. Per-section `bare_sockets: [id]`. Consequence: a second list names sockets by id, needs an unknown-id lint, and drifts from the sockets it names.
  - D. Per-section `skin: false`. Consequence: cannot mark one link point; the owner asked per link point.
- Shape: `LinkPoint.skin: LinkPointSkinType` in `crates/nova_ship/src/sections/link_points.rs`. Default `Clad`. Absence has defined behaviour (the socket clads, as every socket did before), so the default hides no invalid authoring.
- Rule: `read_cells` (`shell_skin.rs`) drops `Bare` sockets from the `SkinStructure`. An open `Bare` face gets no plate, so no greeble; the skin beside it does not climb it as a wall; clearance does not count it as a cladding blocker. The link graph, placement and integrity graph read every socket unchanged.
- Lint: no semantic lint. Every combination of `Clad`/`Bare` is valid; a misspelt variant fails to parse at `content lint` and at load.
- Content: `truss_hull_section` and `solar_panel_section` are `Bare` on both sockets, so spoke ends and wing tips stop clean.
- Not changed: `nova_wfc`'s lattice reads tile faces from every socket, so its clearance check still counts a `Bare` face as clad. WFC draws no truss or solar part today.

- Orchestrator close 2026-10-14: landed in 53c5a186a (`LinkPoint.skin: LinkPointSkinType { Clad, Bare }`, truss and solar sockets `Bare`), dcc5aaaee (`screenshot_solar_wing`) and ef8e44fbe (docs). The orchestrator reviewed the wide and wing-tip before/after stacks: the end caps and their greebles are gone, and the panels and the open girder end show clean. `content lint` re-run: 0 errors. For the owner: the field defaults to `Clad` and may be left out of RON. This differs from the explicit-fields rule; the agent's reason is that absence has a defined meaning and a required field would break every mod for no new behaviour. Debt: the ship generator clearance check still treats a `Bare` face as clad (the generator does not use truss or solar parts today).

## Delivery gate
- Propose the authoring shape (a per-socket flag or a per-section rule), its name, default and lint before code. Other sections can still attach to such a socket; only skin plating and greebles skip it. Regenerate base content; migrate the example mod if the format changes.

## Verification
- Asserted example or unit test: an open dead-end socket gets no skin plate and no greeble; a section attached to it still links. Screenshot of a solar wing end.

## Done when
- Solar panel wing ends render clean and the rule is documented for modders.
