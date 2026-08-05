# Authorable scenario lighting: let a scene pose its own lights instead of one hardcoded top-down key

- PRIORITY: 66
- TAGS: v0.10.0,render,scenario,modding
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-115955

## Context

Every scenario is lit by exactly one hardcoded light: a `DirectionalLight`
(illuminance 10000) rotated straight down, spawned by the loader at
`crates/nova_scenario/src/loader/lifecycle.rs:203`. Nothing in the scenario RON
can change, add or remove it. A top-down key light with no fill and no rim is
why ships and asteroids read flat: hull faces get one brightness, silhouettes
get no separation from the skybox, and the bloom the camera already applies
(`PostProcessingDefaultPlugin` - `Bloom::NATURAL` + `Tonemapping::TonyMcMapface`)
has almost nothing bright to work with.

Surfaced by the screenshot refresh (`20260805-105154`), which works around it:
an example may spawn its own lights, so the capture examples get a photo rig in
code and the shipped scenes do not. That leaves the menu backdrops
(`menu_ambience`, `menu_scrapyard`, `menu_waystation`) - the first thing a player
sees - and every mod scene on the flat single light.

Owner call 2026-08-05: file it as v0.10.0 work rather than blocking the
screenshot refresh on it.

## Open questions

- What is the authorable surface: a `lights: [...]` list on the scenario, or a
  small named preset ("key+rim", "backlit", "flat") that keeps the RON honest
  and the mod surface narrow? YAGNI says preset until a scene needs more.
- Does the current single light stay as the default when a scenario authors
  nothing, so no shipped scene changes on landing?
- Does this reach the mod format and content lint, and what does a bad light
  authoring mistake (a black scene) look like at lint time?
- Do the capture examples then drop their code-side photo rigs in favour of
  authored lights, or keep them?

## Notes

- Not blocking `20260805-105154`; the capture examples light themselves.
- Shipped scenes that would benefit immediately: the three menu backdrops.
