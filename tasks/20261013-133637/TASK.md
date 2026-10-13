# Skin: dead-end sockets link sections but get no skin or greebles

- STATUS: OPEN
- PRIORITY: 96
- TAGS: v0.16.0,skin

## User facts
- Owner feedback 2026-10-13: "some sections (for example the solar panels) to not have skin plating/greebles on them, for example if I create a "wing" of solar panels they will leave out a single link point on the X axis, would be cool to be able to say that that link point doesn't get a skin, like it's a "dead end" but it can have other sections attached to it, just not skin plating".

## Agent findings
- `solar_panel_section` (86e971480) joins side to side on +-X; an open end socket currently gets skin and greebles. Sockets live on the section prototype (`base_section.rs`); skin clads socket faces (`shell_skin.rs`).

## Delivery gate
- Propose the authoring shape (a per-socket flag or a per-section rule), its name, default and lint before code. Other sections can still attach to such a socket; only skin plating and greebles skip it. Regenerate base content; migrate the example mod if the format changes.

## Verification
- Asserted example or unit test: an open dead-end socket gets no skin plate and no greeble; a section attached to it still links. Screenshot of a solar wing end.

## Done when
- Solar panel wing ends render clean and the rule is documented for modders.
