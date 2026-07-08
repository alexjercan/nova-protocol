# Add scenario config resource

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.5.0,objectives,modding

Store all scenarios as a resource. Legacy #99.

Spike: docs/spikes/20260708-161726-modding-language-and-scripting.md

Rides on 133029 (RON format + AssetLoader): once scenarios are RON assets, load
them into the `GameScenarios` resource from `assets/scenarios/` instead of
building them in `crates/nova_assets/src/scenario.rs`. A partial `GameScenarios`
resource already exists; this task is really "populate it from data, not code".
