# Add status info to ScenarioLoaded event

- PRIORITY: 40
- TAGS: v0.4.0, chore
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

Useful for debugging scenario init. Legacy #131.

Pulled into v0.4.0 (roadmap spike 20260708-161726): small chore that supports the
0.4.0 testability theme - the autopilot/screenshot smoke harness can assert on a
richer `ScenarioLoaded` payload (scenario id, object/handler counts).
