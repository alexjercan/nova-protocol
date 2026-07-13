# Retro: base game object abstraction (task 20260525-132946)

## What was asked
A base game-object abstraction that adds physics + health automatically.

## What happened
Extracted the shared destructible-body components (Health, ColliderDensity,
ExplodableEntity, Visibility::Inherited) into `destructible_body(health, density)` in a
new nova_gameplay/game_object module, adopted in base_section and asteroid. Provably
equivalent composition, so no behavior change.

## Lessons
- A bundle *function* beat a `#[require(..)]` component here: Health and Collider need
  per-object values, so a required-components marker would force meaningless defaults.
- For an abstraction task verified compile-only, adopt it only where the resulting
  composition is byte-for-byte identical. That gets the boilerplate win without a
  behavior-change gamble you can't runtime-check.
