# Retro: required-plugins check (task 20260525-132955)

## What was asked
Panic or warn early if a required dependency plugin is missing from the gameplay plugins.

## What happened
NovaGameplayPlugin adds almost everything it needs itself (physics, particles, rng, all
bevy_common_systems plugins). The one thing it relies on the host to provide is
`bevy_enhanced_input::EnhancedInputPlugin` (the spaceship input system builds on it).
Added a single early `assert!(app.is_plugin_added::<EnhancedInputPlugin>(), ...)` at the
top of `NovaGameplayPlugin::build()`.

## Lessons
- Put the dependency check on the umbrella plugin, not each sub-plugin: the sub-plugins
  are only ever added together via NovaGameplayPlugin, so one assert covers them all.
- Assert the *non-obvious* requirement. DefaultPlugins/RenderPlugin absence fails loudly
  on its own; EnhancedInputPlugin absence fails obscurely later, so it is the one worth a
  guard. Resisted hard-asserting a render type to avoid brittleness against future
  headless tests.
- Panic vs warn: a missing required plugin makes the game non-functional, so fail fast at
  startup rather than limp along.
