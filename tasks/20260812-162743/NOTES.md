# Shared scenario authoring helper catalog

## Plan

- Add public `nova_authoring::scenario_helpers` with a prelude.
- Keep the built-in `scenario` module private.
- Move only generic RON-facing constructors out of Shakedown.
- Keep scenario-specific story, pacing, object, and player-id policy local.
- Replace duplicate lifecycle filter aliases with one `entity(id)` constructor.
- Migrate built-in scenarios and examples that have equivalent local helpers.
- Preserve generated base RON byte-for-byte.

## Initial catalog

Expressions and variables:

- `number`
- `variable`
- `set_variable`
- `increment_variable`
- `number_equals`
- `number_less_than`
- `number_greater_than`
- `scenario_elapsed_watch`

Entity filters:

- `entity`
- `entity_pair`

Actions:

- `spawn_object`
- `despawn_object`
- `post_objective`
- `complete_objective`
- `attach_objective_marker`
- `detach_objective_marker`
- `show_hint_emphasis`
- `clear_hint_emphasis`
- `story_message`

## Scope boundary

Keep local:

- Shakedown's player id and `player_enters` policy.
- Beat gates, opening-line sequencing, and pacing categories.
- Scenario object builders and story constants.

## Implementation

- Added public `nova_authoring::scenario_helpers` and exported its prelude.
- Moved generic expression, variable, entity-filter, watch, objective, marker,
  hint, spawn/despawn, and story constructors out of Shakedown.
- Replaced `defeated`, `destroyed`, and `neutralized` aliases with `entity`.
  Event configs retain the lifecycle meaning.
- Kept Shakedown's `player_enters` local because it owns the player-id policy.
- Migrated Broadside, Lifeline, Final Tally, pacing, and Shakedown.
- Migrated equivalent local constructors in `player_path`, `outcomes`, and
  `scenario_grammar`.

## Verification

- `nix develop --command cargo check`
- `nix develop --command cargo test --lib -p nova_authoring` - 45 passed
- Focused example checks: `player_path`, `outcomes`, `scenario_grammar`
- `nix develop --command cargo run content -- gen`
- Generated scenario SHA-256 hashes unchanged for all 10 base scenario files.
- `nix develop --command cargo run content lint` - 0 errors, 0 warnings
- `nix develop --command cargo fmt --check`
- `git diff --check`
