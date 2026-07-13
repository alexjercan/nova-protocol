# Spikes

Exploratory research docs from the /spike skill: a fuzzy idea or open question
explored to a direction, before any code. Named `YYYYMMDD-HHMMSS-description.md`
(the tatr task id). Add an index line here when adding a file.

## Index

- [20260708-110317-promotion-eligible-systems.md](20260708-110317-promotion-eligible-systems.md) - Spike: Which nova systems should be promoted into bevy-common-systems?
- [20260708-161726-modding-language-and-scripting.md](20260708-161726-modding-language-and-scripting.md) - Spike: Modding authoring - declarative markup, embedded Lua, or both?
- [20260708-165647-weapons-hud.md](20260708-165647-weapons-hud.md) - Spike: What weapons-HUD improvements to build, and on what substrate?
- [20260708-203517-roadmap-reprioritization-and-juice.md](20260708-203517-roadmap-reprioritization-and-juice.md) - Spike: Roadmap reprioritization - what to drop, what to commit to v0.5.0, and what new "feel" features to add
- [20260709-091536-combat-cue-propagation-dedup.md](20260709-091536-combat-cue-propagation-dedup.md) - Spike: One hit, one cue - dedup HealthApplyDamage propagation in audio + juice
- [20260709-094731-flight-feel-assisted-newtonian.md](20260709-094731-flight-feel-assisted-newtonian.md) - Spike: Flight feel - what makes a capital ship fly well without faking the physics?
- [20260709-103324-diegetic-autopilot.md](20260709-103324-diegetic-autopilot.md) - Spike: Diegetic autopilot - the computer flies the ship through its real actuators
- [20260709-121746-multi-thruster-autopilot.md](20260709-121746-multi-thruster-autopilot.md) - Spike: Multi-thruster autopilot - every engine is an actuator, not just the nose
- [20260709-164502-screen-indicator-architecture.md](20260709-164502-screen-indicator-architecture.md) - Spike: Concrete architecture of the screen-projected-indicator widget
- [20260709-192358-component-lock-vats-lite.md](20260709-192358-component-lock-vats-lite.md) - Spike: Turret auto-lock + component fine-lock (VATS-lite)
- [20260709-193147-gravity-wells-orbital-mechanics.md](20260709-193147-gravity-wells-orbital-mechanics.md) - Spike: Gravity and orbits - how does a ship park in orbit around an asteroid without n-body chaos?
- [20260709-225508-ai-combat-behaviors.md](20260709-225508-ai-combat-behaviors.md) - Spike: Splitting "smarter enemy AI" into combat behavior work items
- [20260710-104011-target-inset-view.md](20260710-104011-target-inset-view.md) - Spike: Target inset view - a zoomed close-up of the locked ship for easier section targeting
- [20260710-134413-v0.5.0-release-scope.md](20260710-134413-v0.5.0-release-scope.md) - Spike: What is in v0.5.0, and in what order?
- [20260710-174523-diegetic-instruments-keybind-hints.md](20260710-174523-diegetic-instruments-keybind-hints.md) - Spike: Diegetic flight instruments and keybind hints - what visual language, what architecture?
- [20260710-204802-gravity-aware-arrival.md](20260710-204802-gravity-aware-arrival.md) - Spike: How should GOTO/STOP arrival planning account for gravity wells?
- [20260710-234019-diegetic-flight-status.md](20260710-234019-diegetic-flight-status.md) - Spike: Diegetic flight status - where does each piece of the bottom-left line go?
- [20260711-103527-twitching-family-two-clocks.md](20260711-103527-twitching-family-two-clocks.md) - Spike: the twitching family - two clocks, one bug
- [20260711-140234-feel-filtering.md](20260711-140234-feel-filtering.md) - Spike: client-side smoothing and deadbands on PD and camera outputs
- [20260711-163800-multi-target-cycle.md](20260711-163800-multi-target-cycle.md) - Spike: Multi-target candidate set + target cycling (HUD + input)
- [20260711-180500-main-menu.md](20260711-180500-main-menu.md) - Spike: How should the main menu be built and wired into the app?
- [20260711-212358-live-ship-systems-outside-editor-scenario.md](20260711-212358-live-ship-systems-outside-editor-scenario.md) - Spike: How do thruster-driven ships come alive outside the editor's Scenario state?
- [20260711-sdlc-skill-suggestions.md](20260711-sdlc-skill-suggestions.md) - SDLC skill suggestions from the twitching-family session (2026-07-11)
- [20260712-092926-starter-scenario.md](20260712-092926-starter-scenario.md) - Spike: What is the starter New Game scenario, beat by beat?
- [20260712-112113-bullets-affected-by-gravity.md](20260712-112113-bullets-affected-by-gravity.md) - Spike: Should turret rounds (bullets) feel gravity wells?
- [20260712-133135-weapon-and-damage-type-variety.md](20260712-133135-weapon-and-damage-type-variety.md) - Spike: How should nova add damage types, resistances, bullet types and alt-fire?
- [20260712-140842-objective-conveyance-visuals.md](20260712-140842-objective-conveyance-visuals.md) - Spike: How should the objective conveyance visuals look and behave?
- [20260712-143113-diegetic-ammo-readout.md](20260712-143113-diegetic-ammo-readout.md) - Spike: How should the per-weapon ammo readout be drawn - world-space diegetic widget or screen-projected HUD node?
- [20260712-143551-controller-provided-verb-flags.md](20260712-143551-controller-provided-verb-flags.md) - Spike: Should flight verbs be gated by the controller section and per-verb flags on it?
- [20260712-160505-damage-and-bullet-type-taxonomy.md](20260712-160505-damage-and-bullet-type-taxonomy.md) - Spike: What concrete damage types and bullet types should nova have, and what is the first resistance table?
- [20260712-203235-lock-stickiness-and-inset-scope.md](20260712-203235-lock-stickiness-and-inset-scope.md) - Spike: Sticky target lock + ship-only inset scope
- [20260712-215256-combat-travel-lock-separation.md](20260712-215256-combat-travel-lock-separation.md) - Spike: Separate combat vs travel locks; widen the cyclable pool to asteroids
- [20260712-215733-unified-target-computer.md](20260712-215733-unified-target-computer.md) - Spike: Unified component-based target computer (cone list + sticky lock)
- [20260712-222610-travel-combat-lock-slots.md](20260712-222610-travel-combat-lock-slots.md) - Spike: Travel/combat lock slots - raise-to-combat, fire-gated-on-lock
- [20260713-082207-deliberate-radar-locking.md](20260713-082207-deliberate-radar-locking.md) - Spike: Deliberate radar locking - travel/combat locks, weapons safety
- [20260713-110039-show-dont-tell-radar-ux.md](20260713-110039-show-dont-tell-radar-ux.md) - Spike: Show-don't-tell radar UX - live lock, inset-as-status, less text
- [20260713-140742-shakedown-beat-sheet-v2.md](20260713-140742-shakedown-beat-sheet-v2.md) - Spike: Shakedown beat sheet v2 - smaller objectives, more beacons, more fun
- [20260713-154023-inset-kill-cam.md](20260713-154023-inset-kill-cam.md) - Spike: Inset kill cam - show the death, don't slam the viewfinder shut
