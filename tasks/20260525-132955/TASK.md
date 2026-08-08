# Check required plugins were added in gameplay plugins

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.3.1, refactor

Panic or warn early with a clear message if a required dependency plugin is missing. Legacy #126.

## Steps

- [x] Determine which plugins NovaGameplayPlugin depends on but does not add itself.
- [x] Assert their presence early with a clear message.
- [x] Verify build --all-targets, clippy, fmt.

## Resolution (CLOSED)

Audited NovaGameplayPlugin: it adds its own physics (PhysicsPlugins), particles
(hanabi), rng (EntropyPlugin) and every bevy_common_systems plugin. The one external
dependency it does NOT add is bevy_enhanced_input::EnhancedInputPlugin - the spaceship
input system (SpaceshipPlayerInputPlugin / SpaceshipAIInputPlugin, via add_input_context)
is built on it, and it is supplied by the host app (AppBuilder adds it before
NovaGameplayPlugin). If a consumer adds NovaGameplayPlugin without it, input-context
registration fails obscurely later.

Added an early `assert!(app.is_plugin_added::<EnhancedInputPlugin>(), ...)` at the top of
NovaGameplayPlugin::build() with a message that names the missing plugin and how to fix
it. In normal use (AppBuilder, examples) the assert passes because EnhancedInputPlugin is
added first.

Scope decisions:
- Chose a panic (assert) over a warning: a missing required plugin makes the game
  non-functional, so failing at startup is better than a degraded run.
- Considered also asserting Bevy's DefaultPlugins (RenderPlugin). Dropped it: its absence
  is caught immediately and obviously by Bevy itself, and hard-asserting a specific
  render type risks brittleness against future headless test setups. EnhancedInputPlugin
  is the genuinely non-obvious, easy-to-forget requirement, so that is the high-value
  check.

Verified: build --all-targets, clippy, fmt green.

Self-reflection: the umbrella plugin was the right place for the check (one assert
covers the whole gameplay stack); pushing a check into each sub-plugin would have been
redundant since they are only ever added together via NovaGameplayPlugin.
