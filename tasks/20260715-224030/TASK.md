# Creator docs: complete starter scenario, author-a-section RON reference, honest launch note

- STATUS: CLOSED
- PRIORITY: 46
- TAGS: docs,web,feature,modding

From the docs review spike 20260715-223147 (creator persona). A determined RON
author can ship a mod, but the journey has gaps: only scenario fragments (no
complete clone-me file), no creator-facing section-authoring vocabulary (the
demo mod uses a `Section` the guides never document), and the "launch your own
scenario" reality is buried.

## Goal

Make the creator journey complete and honest: a full starter scenario, a
section-authoring RON reference, and a front-and-centre launch note.

## Steps

- [x] End `web/src/wiki/dev/guide-author-scenario.md` section 6 with a COMPLETE,
      copy-pasteable scenario file: the `[Scenario((...))]` wrapper, an
      `OnStart` with `CreateScenarioArea` + spawns, and the worked objective-loop
      handlers assembled into one runnable file. Also link
      `assets/base/scenarios/asteroid_field.content.ron` explicitly as the
      "clone this" full example.
- [x] Add a creator-facing "Author a section (RON)" reference documenting the
      `Section((base: BaseSectionConfig, kind: SectionKind))` content item and
      each `SectionKind` variant's RON fields (Hull / Thruster / Controller /
      Turret / Torpedo). Ground every field in
      `crates/nova_gameplay/src/sections/{hull,thruster,controller,turret,torpedo}_section.rs`
      (the `*SectionConfig` structs) + `BaseSectionConfig`, and show the real
      `Section` block from `assets/mods/demo/mod.content.ron`. Place it as a
      section in `web/src/wiki/dev/guide-make-a-mod.md`, or a new
      `web/src/wiki/dev/guide-author-section.md` in the "Scenarios & mods" band.
- [x] Rework `guide-author-scenario.md` section 7 so the launch reality leads:
      today there is NO pure-RON way to boot your own scenario - you either edit
      `NEW_GAME_SCENARIO_ID` in `crates/nova_menu/src/lib.rs` (one line of Rust)
      or reach it via a `NextScenario` action chained from a running scenario.
      State the chaining route FIRST (closest to no-Rust), and note the scenario
      picker (task 20260715-200828, OPEN) as the tracked resolution.
- [x] Fix the journey mismatch in `guide-make-a-mod.md`: "copy the demo mod"
      points at a mod whose headline is a section overlay the scenario guide
      never taught - either cross-link the new section reference before the demo,
      or add a scenario-only starter to copy.
- [x] (optional) Add a short `webmods/gauntlet/README.md` mirroring
      `assets/mods/demo/README.md`, so the cited publish example is
      self-orienting.
- [x] Verify: `npm run ci` green; sanity-check the assembled starter scenario's
      RON shape against a shipped `*.content.ron` (field names, newtype parens,
      `Some(...)`); serve + eyeball the new reference renders.

## Notes

- 2026-07-15 (impl): step 3's premise was stale. The Scenarios picker
  (task 20260715-200828) shipped and is now CLOSED, so there IS a pure-RON way
  to launch your scenario. Section 7 was reworked to lead with the picker route
  (author -> enable mod -> Scenarios picker -> Play), with NextScenario chaining
  and the `NEW_GAME_SCENARIO_ID` one-liner as the other two routes, rather than
  the "no picker yet" framing the step described.
- Placement: the section reference became a new page
  `web/src/wiki/dev/guide-author-section.md` (not inlined into guide-make-a-mod),
  registered in the "Scenarios & mods" band next to the sibling guides.
- Depends on: 20260715-223551 (fix the stale `*.scenario.ron` format in
  modding-ron.md first, so the reference this task cross-links is correct).
- Grounding files: `crates/nova_gameplay/src/sections/*_section.rs`,
  `assets/mods/demo/mod.content.ron`,
  `assets/base/scenarios/asteroid_field.content.ron`,
  `crates/nova_scenario/src/actions.rs` (ScenarioObjectKind / SpawnScenarioObject).
- Do not invent RON field names - copy shapes from the section config structs and
  the shipped files.
