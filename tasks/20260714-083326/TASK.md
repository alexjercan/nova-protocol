# Scenario RON authoring layer + AssetLoader: resolve asset paths to Handles, lower to runtime config

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.6.0,modding,scenario,foundation

Spike: tasks/20260714-083224/SPIKE.md

Goal: the design-heavy half of the RON format. Once the config tree has serde
derives (20260525-133029), build a `*.scenario.ron` Bevy `AssetLoader` plus an
authoring layer that resolves asset references written as paths/strings into live
`Handle`s. This is the crux the spike identified: `ScenarioConfig.cubemap`,
`AsteroidConfig.texture` (both `Handle<Image>`) and `BeaconRenderConfig.color`
(bevy `Color`) cannot round-trip a hand-authored file as-is.

Recommended approach (a): separate authoring structs with `String` asset fields,
deserialized from RON, then lowered to the runtime configs inside the loader where
a `LoadContext` exists to turn `load_context.load("textures/rock.png")` into a
`Handle`. Weigh against (b) custom serde on the runtime types during planning; see
the spike for why (a) is favored. Consider whether section blueprints
(`sections.rs` "load from JSON" stub) fold into the same layer.

Gated on 20260525-133029 (serde derives). Feeds 20260525-133028 (load
GameScenarios from assets) and the editor scenario builder (20260714-081703).

