# Extract torpedo into its own module/plugin; unhardcode blast params

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.4.0,torpedo,refactor


From the TODO sweep (task 20260525-132954). The torpedo logic and its targeting system
live inline in torpedo_section.rs and should be factored into their own module/plugin;
blast parameters (radius, damage) are hardcoded and should be config-driven.

Source TODOs (crates/nova_gameplay/src/sections/torpedo_section.rs):
- Factor out the torpedo logic into a separate module
- Implement a separate plugin for the targeting system
- Unhardcode blast parameters
