# Prototype references + Modification model for ship sections; re-port built-in ships; serde default omission

- STATUS: OPEN
- PRIORITY: 56
- TAGS: v0.6.0,modding,scenario

Spike: tasks/20260714-110502/SPIKE.md

Goal (step 2, the big dedup): let a ship section reference a catalog prototype by
id and apply deltas instead of inlining the full config. Authoring form:
`(id, position, rotation, prototype: "<catalog id>", modifications: [..])`. Add a
closed `Modification` enum (Rename/SetMass/SetHealth/DisableVerb/SetRenderMesh/
SetBindings, room to grow - pure data, no scripting). The nova_modding authoring
layer resolves prototype -> clone -> apply mods -> runtime `SpaceshipSectionConfig`.
Stack `#[serde(default)]` field omission underneath to trim per-field noise. Re-port
the built-in ships to references and regenerate; the parity test proves the lowered
result is byte-identical to today's configs (this is where shakedown collapses from
~1480 lines). Gated on the catalog (20260714-113408).

## Plan (20260714)

Design decisions (user):
- **Prototype resolution: single tree, resolved at spawn.** `SpaceshipSectionConfig`
  becomes `{ id, position, rotation, source: SectionSource, modifications:
  Vec<SectionModification> }` where `SectionSource = Inline(SectionConfig) |
  Prototype(SectionId)`. `insert_spaceship_sections` resolves the source against
  `Res<GameSections>` (the catalog, task 113408) - mirrors AssetRef's resolve-at-spawn.
  `GameScenarios` holds the compact authored form. No parallel authoring tree.
- **Modifications are COMPONENTS, applied by observers (user's model).** A closed
  `SectionModification` enum authored in RON; at spawn, each is inserted as a
  component on the resolved section entity, and a small observer applies it WHERE
  RELEVANT (queries the target component) and is INERT elsewhere. E.g.
  `DisableVerb(Orbit)` on a controller -> observer with `ControllerVerbs` clears the
  verb; the same component on a hull -> no observer matches -> nothing. Extensible:
  new modification = new component + observer, no central match. (Open "any registered
  component" via reflection is a later step; a closed enum is the first cut.)
- **Include serde(default) field omission this cycle** (user's call) - the broadest,
  riskiest part; done as its own step.

Steps:
- [ ] 1. nova_scenario: `SectionSource` enum + reshape `SpaceshipSectionConfig`
  (source + modifications); serde. Resolve the source in `insert_spaceship_sections`
  via `Res<GameSections>` (missing prototype -> error+skip, no panic). Fix the
  spaceship tests + construction sites. (No modifications applied yet.)
- [ ] 2. `SectionModification` closed enum + a modification component per variant +
  an `On<Add, _>` observer per variant that applies it where relevant, inert
  elsewhere. Start small: DisableVerb(FlightVerb), SetHealth(f32), SetMass(f32),
  Rename(String). Insert the components at spawn. Tests: applied-on-controller vs
  inert-on-hull, health/mass override.
- [ ] 3. Re-port: scenario generators emit `source: Prototype("<id>"),
  modifications: []` instead of inlining `get_section(id).clone()`; drop the
  now-unused `&GameSections` builder param; regenerate the scenario RON; the parity
  test guards it (shakedown collapses from ~1480 lines).
- [ ] 4. serde(default) + `skip_serializing_if` across the config tree (asteroid,
  beacon, section, spaceship, ...) so authored files omit default fields; ensure
  round-trip; regenerate RON (smaller); parity guard.
- [ ] 5. Verify: `cargo test --workspace --no-run`; nova_scenario/nova_assets tests;
  `12_menu_newgame` (ships spawn from prototype refs resolved against the catalog) +
  `09_editor` under `DISPLAY=:0 BCS_AUTOPILOT=1 --features debug`; parity tests green.

Follow-on: whole-ship prototypes (113414), multi-file bundles (113418).
