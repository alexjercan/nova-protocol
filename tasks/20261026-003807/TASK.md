# Bridge New Game to Jobs chart and Station with legible docking resources

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.17.0

## User facts
- First session must explain Jobs/chart/Station and resource/docking legibility without making docking automatic. Fable read-only review found Basic Training has no New Game bridge, conditional RCS pips, dense chart codes/tooltips, and wiki falsely claims roll required. No v0.16 screenshot exists. Specs only.
## Agent findings
- Basic Training is separate gated scenario (`crates/nova_authoring/src/base_content/scenarios/tutorial/mod.rs:303`); New Game starts docked at allied station (`web/src/wiki/getting-started.md:17`). Chart has 7x7 display (`crates/nova_interface/src/map/chart.rs:65-69`) over shipped active 5x5x5 (`crates/nova_world_base/src/lib.rs:190`). Jobs board already holds three offers (`crates/nova_gameplay/src/jobs.rs:89`); conditional pips are driven by HUD state (`crates/nova_hud/src/flight_status.rs:451-477`). Dock sight exists (`crates/nova_hud/src/docking_sight.rs:210-275`), roll not required for capture (`crates/nova_ship/src/sections/docking_section/tests.rs:273`), contrary to `web/src/wiki/sections/docking.md:50`.
## Decisions needed
- Which first-session milestone and trigger owns guidance (scenario/UI/content), opt-out/return behavior, what to show for unavailable RCS and port readiness, chart-label hierarchy and screen-reader/input parity. No new forced tutorial, marker, tooltip default or docking aid approved; coordinate assist and ammo tasks before changing shared HUD.
## Delivery
- End: New Game pilot can find Jobs, read chart objective, use Station and understand resources/docking constraints without unearned capture. Dies: false roll-required wiki copy and superseded dense labels after approved change. May break: new-game flow, input focus, pause timing, UI density/localization and existing tutorial. Bad authored cue/ID fails lint/load; unavailable action has explicit reason.
- Owner/entry/state/order: `nova_authoring` start/tutorial or approved UI owner -> Jobs (`crates/nova_world_base/src/jobs.rs:158-355`) and chart (`crates/nova_interface/src/map/chart.rs:428-766`) -> Station (`crates/nova_interface/src/station/mod.rs:100-147`) -> HUD. Audit UI callers, content builders, wiki/changelog and manual docking rules.
## Verification / Done when
- New Game `nova-bench` flow records navigation from dock to first job/chart/station and resource explanation; inspect actual rendered frames on isolated display before visual claims, never infer from Fable prose; assert no auto-dock. Approve permanent test first.
