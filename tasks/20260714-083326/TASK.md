# Create nova_modding crate: authoring wrappers + .scenario.ron AssetLoader + lowering

- STATUS: CLOSED
- PRIORITY: 75
- TAGS: v0.6.0, modding, scenario, foundation

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

## Discovery during 133029 impl (20260714) - two tiers

The pure-data config leaves now serialize (committed on branch). Making the FULL
`ScenarioConfig` serialize splits into two tiers by how hard the remaining blockers
are:

- TIER 1 - scenario logic + non-ship objects (near-term, self-contained). Blockers
  are only asset refs: `cubemap` + `AsteroidConfig.texture` (`Handle<Image>`) and
  `BeaconRenderConfig.color` (`Color`; likely serializes under `bevy/serialize` -
  confirm). Plan: an `ImageRef` type (deserializes from an asset-path string,
  resolved to a `Handle` by the loader's `LoadContext`) replaces the raw handle
  fields, so the whole logic/object tree (events, filters, variables, objectives,
  areas, asteroid/beacon/salvage spawns) serializes with NO duplicate authoring
  twins. This is the elegant single-tree path and delivers a real modding format
  for everything except player/AI ships.
- TIER 2 - spaceships (bigger, touches nova_gameplay + input). `SpaceshipConfig`
  embeds `SpaceshipController` -> `Binding` (bevy_enhanced_input, EXTERNAL, no
  serde) and `SpaceshipSectionConfig.config: SectionConfig` (nova_gameplay,
  Reflect-only); `SetControllerVerbActionConfig` embeds `FlightVerb` (nova_gameplay,
  Reflect-only). Needs: serde on nova_gameplay `SectionConfig`/`FlightVerb` (add a
  serde feature there) + an authoring representation for `Binding` (author keys as
  strings, convert). Substantial; the ship-blueprint/editor-save path depends on it.

Recommendation (at discovery time): land TIER 1 first, TIER 2 as a follow-on.

## Outcome (20260714) - BOTH tiers delivered on this branch

Both tiers shipped, not just tier 1. Rather than a separate `ImageRef`, the generic
`nova_gameplay::AssetRef<A>` covers all asset fields (Image cubemap/texture,
WorldAsset meshes, EffectAsset effects). Tier 2 landed too: `nova_gameplay` got a
serde feature covering `SectionConfig`/`FlightVerb`; `Binding` is handled by a
`BindingInput` + `binding_map_serde` `serde(with)` helper (runtime type unchanged).
So the FULL `ScenarioConfig` - ships, sections, and bindings included - serializes,
and all four built-in scenarios (incl. the ship-heavy `shakedown_run`) are ported to
`assets/scenarios/*.ron` (task 133028). The ship-less demo was still added as the
first end-to-end proof. This task (nova_modding crate + loader) is DONE; ready to
close/review on merge.

