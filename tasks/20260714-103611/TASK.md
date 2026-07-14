# New events/filters/actions surfaced while porting built-ins to RON

- STATUS: CLOSED
- PRIORITY: 20
- TAGS: v0.6.0, modding, scenario

Catch-all for format gaps found while porting the built-ins to RON (133028).

Finding (20260714): the ONLY new format feature the port needed was procedural
scatter, split out and implemented as `ScatterObjects` (task 20260714-103622,
committed). Everything else the built-ins express - events (OnStart/OnDestroyed/
OnEnter/OnUpdate/OnOrbit/OnTravelLock/OnCombatLock), filters (Entity/Expression/
Conditional), and the full action set - was already in the config model and now
serializes, so no other new events/filters/actions were required. The scenarios
are pure `ScenarioConfig`, nothing lived in Rust outside the config. Close as "no
further additions needed" once the port lands, unless the editor scenario-builder
surfaces new needs.
