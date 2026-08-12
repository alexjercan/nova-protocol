# Retro: Define destruction and neutralization event lifecycle

- TASK: 20260812-132400

## What went well

- Small presentation slices were manually validated before the lifecycle event
  model changed.
- Physical destruction now uses `IntegrityDestroyMarker` as proof. Cleanup no
  longer aliases destruction in HUD feedback.
- `OnDefeated` removes duplicated mission logic while preserving detailed
  `OnNeutralized` and `OnDestroyed` edges.

## What to improve

- Scenario authoring helpers are stored under Shakedown and imported elsewhere.
  Generic event, filter, and action constructors need a shared catalog.
- Screenshot examples are not reliable pins for every live combat transition.
  Focused lifecycle tests plus manual playtest were better proof here.

## Follow-ups

- Migrate bundled webmods to `OnDefeated` with bundle version and changelog
  updates.
- Inventory generic authoring helpers and move them into a shared module.
- Decide neutralized-wreck retention and cleanup policy separately.
