# Add scenario config resource

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.6.0,modding,scenario

Store all scenarios as a resource. Legacy #99.

Spike: tasks/20260708-161726/SPIKE.md (direction)
Spike: tasks/20260714-083224/SPIKE.md (detailed design)

Rides on 133029 (serde derives) + 20260714-083326 (AssetLoader/authoring layer):
once scenarios are RON assets, load them into the `GameScenarios` resource from
`assets/scenarios/` instead of building them in `crates/nova_assets/src/scenario.rs`.
A partial `GameScenarios` resource already exists; this task is really "populate it
from data, not code".
