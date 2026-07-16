# Spike: what does a content linter check statically, what must the runtime gate catch, and how does a failed start reach the player?

- DATE: 20260716-193858
- STATUS: RECOMMENDED
- TAGS: spike, modding, testing, v0.7.0

## Question

Task 20260716-191543 wants unknown section prototypes to fail the mod
gates. The user widened it: a LINTER (like the generator - a tool run by
authors and CI) for everything statically checkable, PLUS a runtime
check when mods load, Wesnoth-style - "failed to load X: reason" - so a
scenario that references missing content reports "failed to start"
instead of silently half-working. A good answer places each check at
exactly one tier, names the shared core, and picks the reporting
surface.

## Context (verified in code, 2026-07-16)

- The failure today: `SectionSource::Prototype` resolves at SPAWN
  (crates/nova_scenario/src/objects/spaceship.rs:217) - a miss is
  `error!` + skip the section. Ships half-spawn; nothing is
  player-visible; every load/publish gate passes the content green (the
  Ledger near-miss, task 20260716-123535 close notes).
- Three gate tiers already exist and set the precedent: the portal
  generator (engine-free manifest gate), webmods_validation (engine
  load gate), and nothing at the identifier level - the hole.
- The config types a linter walks (ScenarioConfig, actions, filters)
  live in nova_scenario (serde-featured); the section catalog type
  (SectionConfig, GameSections) in nova_gameplay (a nova_scenario
  dependency). The portal generator CANNOT lint: nova_portal_gen is
  deliberately engine-free.
- At merge time, register_bundles (nova_assets) holds the full ENABLED
  set - merged GameSections + GameScenarios - the only place cross-mod
  references are decidable.
- A player-facing modal already exists: CurrentOutcome (nova_scenario)
  drives the menu's outcome overlay (nova_menu sync_outcome_overlay,
  title + message + buttons).

## Options considered

WHERE THE STATIC LINTER LIVES:

- **Pure lint core in nova_scenario + a CLI bin + a CI test gate**
  (chosen). `nova_scenario::lint`: pure functions over parsed config -
  `lint_scenario(&ScenarioConfig, &known_sections, &known_scenario_ids)
  -> Vec<LintIssue>` (Error|Warn severities). Consumers: (1)
  `cargo run -p nova_assets --bin content_lint` for authors - walks
  assets/base, assets/mods and webmods/, parsing the RON directly, base
  section catalog + each bundle's own Section items as the known set;
  (2) a nova_assets test running the same walk so CI fails on Error
  issues; (3) the runtime gate reuses the same core (below). One
  implementation, three consumers - the gen_content shape.
- **Lint inside nova_portal_gen** - rejected: engine-free by design.
- **Checks only inside webmods_validation** - rejected: authors need a
  CLI with readable output, and base/assets content deserves the gate
  too.

WHAT IS STATICALLY DECIDABLE (v1 check list):

- Error: unknown section prototype (vs base catalog + the bundle's own
  sections); dangling NextScenario target (vs scenario ids visible in
  the bundle + base - a cross-mod target the linter cannot see is a
  Warn naming the assumption); duplicate spawned object ids within a
  scenario; filter/marker/despawn target ids that no
  SpawnScenarioObject or ScatterObjects prefix in the scenario can
  produce.
- Warn: expression variables never VariableSet in the scenario
  (fails-closed at runtime, so almost always an authoring bug);
  ObjectiveComplete without a matching Objective id.
- Out of scope v1: asset paths (async asset semantics, the loaders
  already surface missing files), balance/feel anything.

THE RUNTIME GATE:

- **Merge-time sweep + refuse-to-start** (chosen): after
  register_bundles merges, run the SAME lint core against the MERGED
  registries (this is where cross-mod prototypes/chains become
  decidable) into a `ContentIssues` resource keyed by scenario id.
  `on_load_scenario` consults it: Error-level issues -> do not build
  the scene; log every issue; report the failure to the player. The
  spawn-time `error!` + skip stays as the last-ditch backstop.
- **Spawn-time reporting only** - rejected as primary: by then the
  scene is half-built; the user explicitly wants "failed to start", not
  "started weird". Kept as backstop.

THE REPORTING SURFACE:

- **The existing outcome-overlay path with a failure kind** (chosen):
  a `ScenarioStartFailed` presentation - title "FAILED TO START",
  Wesnoth-style body ("Failed to start 'The Ledger 2': unknown section
  prototype 'basic_hull_section'."), one Main Menu button. Rides the
  proven modal + teardown paths; no new UI system.
- **Mods-menu / picker badges** - good enhancement (mark scenarios with
  issues in the picker details pane), not the core ask; noted as a
  cheap add-on step if the picker wiring is trivial, else deferred.

## Recommendation

Two tasks, one shared core:

1. (20260716-191543, updated) `nova_scenario::lint` core + the
   `content_lint` bin + a CI test gate over base/assets-mods/webmods
   with the v1 check list above. Fails on Error, prints Warns.
2. (seeded) Runtime: merge-time ContentIssues via the same core +
   refuse-to-start with the FAILED TO START overlay; spawn-time error
   stays as backstop.

Ship 1 first (the core defines LintIssue; 2 consumes it).

## Open questions

- ScatterObjects prefix matching: a filter id equal to `<prefix><n>` is
  satisfiable but the linter cannot know n statically - v1 treats "id
  starts with a scatter prefix" as satisfied (documented looseness).
- Whether the picker badge is trivial enough to ride task 2; decided
  at plan time there.

## Next steps

- tatr 20260716-191543: static lint core + bin + CI gate (updated).
- tatr 20260716-193949: runtime ContentIssues + FAILED TO START
  overlay.

## Fix record

(Implementing tasks append here as they land.)

- 20260716-191543 (static half): SHIPPED, landed 00698783 - lint core
  (8 checks), content_lint bin, CI gate; real tree lints clean with one
  accurate warning (the Ledger ch4 choice fork). One design change vs
  the spec: duplicate ids split into same-handler Error /
  cross-handler Warn (the choice-fork pattern is legitimate).
- 20260716-193949 (runtime half): SHIPPED, landed 6a4ff060 - merge
  sweep into ContentIssues, refusal before teardown, FAILED TO START
  modal, backdrop-draw filtering (the menu-camera hazard the spike's
  reporting option missed - a refused MENU load has no camera to show
  a modal on; filtering at the draw was the fix). Both tiers live.
