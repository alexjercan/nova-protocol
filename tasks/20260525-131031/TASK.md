# Create tasks for future sprints - Tasks for Nova Protocol

- STATUS: CLOSED
- PRIORITY: 100
- TAGS: docs,planning

Here we have the old tasks (all tasks with "Wishlist" status are not yet
implemented). The current version of the game is `v0.3.0`. Version `v0.3.1`
will be in progress. This new version will be a refactor job, moving some code
into specific crates. And bugfixing things that are broken right now. Also we
will have to think about improving the existing mechanics and making the game
smoother and more fun. I would personally create `tatr` tasks for `v0.3.1` and
add some priorities to them. Then the rest of tasks that seem to work better
for `v0.4.0` can have a priority of `0`. Obviosuly we can also use tags to
indicate which "sprint" we want to have the tasks assigned to. For example
`v0.3.1` or another version. This would allow us to know when to set priorities
for tasks.

## Old Trello Board

[Wishlist] #32 controller: add custom model for controller
[Wishlist] #34 hull: add some textures to the model
[Wishlist] #158 hud: think of some ways to make the hud cooler by adding the health, and stuff directly on the spherical hud of the spaceship
[Wishlist] #155 inventory: implement logic for hull section to have inventory capacity
[Wishlist] #156 inventory: create a basic inventory plugin that uses a `InventoryMarker` component to store a list of `Item`'s
[Wishlist] #157 inventory: create a basic HUD that can be linked to a `InventoryMarker` to display the list of items
[Wishlist] #140 weapons: create some sort of logic for ammo limit
[Wishlist] #154 Cargo Bay
[Wishlist] #85 Collect Item
[Wishlist] #128 polish: improve the mesh explosion algorithm - I want to be able to "chip" at a mesh
[Wishlist] #153 polish: add inertia to the explode fragments to make them move in the same direction as the thing
[Wishlist] #142 torpedo: implement PN controller for torpedo brain to intercept the target
[Wishlist] #122 health: depending on the section that is getting hit, the spaceship can take variable damage (e.g more for thrusters, less for turret)
[Wishlist] #136 section: torpedo bay sounds for shooting
[Wishlist] #131 chore: add some status info the the ScenarioLoaded event
[Wishlist] #133 chore: pull bevy_common_systems out
[Wishlist] #130 modding: allow changing the cubemap for the skybox via action
[Wishlist] #127 chore: add a way to spawn less visual stuff for low end machines
[Wishlist] #74 Main Menu
[Wishlist] #118 find some optimization for modding/events.rs - Optimize by indexing handlers by event name
[Wishlist] #125 maybe better implementation for the `next_scenario` logic
[Wishlist] #129 chore: improve the ondestroy handling to not immediatly despawn entities in case we need them
[Wishlist] #52 controller: add sounds for RCS thrusters
[Wishlist] #50 turret: add sound effects when shooting
[Wishlist] #51 thruster: add sound effects for thruster
[Wishlist] #88 slicer: make sure the mesh slicer will not crash the game. implement better dynamic explosions using the logic from the mesh slicer
[Wishlist] #101 objectives: scenario language config
[Wishlist] #73 objectives: scenario editor
[Wishlist] #33 controller: use acceleration shader to display RCS thrusters
[Wishlist] #57 turret: add counter shooting particle effects (to visualize counter of recoil)
[Wishlist] #77 Editor Menu
[Wishlist] #76 Pause Menu
[Core Mechanics] #6 Nova Protocol
[Core Mechanics] #9 Spaceship Sections
[Core Mechanics] #69 Objectives
[Core Mechanics] #24 YT Content - Devlog
[Core Mechanics] #7 YT Content - Short
[To Do] #160 Fix the TODO's
[To Do] #135 section: torpedo bay particles for shooting
[To Do] #63 mesh slicer showcase
[To Do] #96 chore: create some tests from the examples that I have instead, maybe keep some small examples, but I want to be able to have tests
[To Do] #126 chore: somehow check that the required plugins were added in gameplay plugins
[To Do] #115 improve error handling and logging of the modding logic
[To Do] #161 devlog 0.3
[🎉 Done] #132 chore: maybe add a marker component for the post processing camera
[🎉 Done] #159 Design v0.3.1
[v0.3.0] #137 section: create some sort of default visual for the torpedo bay section
[v0.3.0] #36 thruster: improve thruster shader
[v0.3.0] #138 torpedo: create a few default sections for torpedo (cylinder ball head, thruster)
[v0.3.0] #120 health: game objects with rigidbody should have a structural integrity health to decide when they get destroyed
[v0.3.0] #151 fix the on delete of a section and other entities to remove from graph
[v0.3.0] #152 when a collider reaches zero health, we need to set it first as disabled, and explode it when it is a leaf
[v0.3.0] #148 implement a DFS/BFS algorithm to build a graph of the spaceship sections
[v0.3.0] #150 when a collider (section) reaches zero health explode it + despawn it
[v0.3.0] #149 implement a plugin that deals damage to colliders instead of the rigidbody - and add health to the colliders instead
[v0.3.0] #78 directional: make the sphere around the cone not transparent using a shader
[v0.3.0] #147 torpedo: blast radius visual (explosion shader or something)
[v0.3.0] #146 torpedo: create a HUD for the torpedo target to show when the player shoots.
[v0.3.0] #139 torpedo: create actual damage dealing part for torpedo
[v0.3.0] #141 section: implement the torpedo bay section (similar to turret) to spawn torpedoes
[v0.3.0] #143 torpedo: implement basic torpedo that just follows the assigned target
[v0.3.0] #144 refactor: make the thruster into a `bevy_common_systems` component
[v0.3.0] #145 refactor: make PD controller into a `bevy_common_systems` thingy
[v0.3.0] #71 Torpedo Bay
[v0.3.0] #86 Reach Zone
[v0.3.0] #134 Design v0.3
[V0.2.1] #100 when no spacship camera should be WASD, when spaceship it should be chase
[V0.2.1] #62 turret showcase
[V0.2.1] #113 Documenation
[V0.2.1] #116 cleanup the magic numbers in the turret section
[V0.2.1] #123 devlog: do the thumbnail and title
[V0.2.1] #98 devlog v0.2
[V0.2.1] #114 refactor: improve the input system for the player spaceship components (thruster and turrets should use bevy enhanced input)
[V0.2.1] #117 refactor: add an input mapping to the PlayerControllerConfig for each section some input and the action - pub input_mapping: HashMap<>
[V0.2.1] #124 insert_spaceship_sections: fix editor to not spawn spaceship but only like visual for config
[V0.2.1] #104 bug: more like refactor -> improve the despawning and entity management in the scenarios
[V0.2.1] #109 refactor: probably don’t need the LoadScenarioId, i will just have a menu with all the ids and then call LoadScenario by config
[V0.2.1] #119 Rename this from collision_damage to collision_impact or something like that
[V0.2.1] #106 refactor: all components that are created in bevy common systems should be parented (aka should never spawn entities)
[V0.2.1] #107 refactor: add some sort of base game object that adds physics and health by itself
[V0.2.1] #95 chore: create a changelog.md file with stuff from the history
[V0.2.1] #108 design: how can I get the damage to be more realistic (instead of asteroid explode, maybe just make a chunk fly out, for spaceship we should damage each section) - but i still think we might want to have an overall critical health - too much damage overall means boom - it could work just like ColliderOf - where all children with health contribute to the main game object
[V0.2.1] #103 Design v0.2.1
[V0.2.0] #93 visuals
[V0.2.0] #105 visual: add visual texture for asteroids
[V0.2.0] #87 visual: add visual model for asteroids
[V0.2.0] #92 enemy AI
[V0.2.0] #102 bug: when switching scenes remove all objects
[V0.2.0] #83 enemy: very simple AI movement, random movement
[V0.2.0] #82 enemy: very simple AI where we can just add simple ships on the map that are not controlled by anyone
[V0.2.0] #90 objectives: implement HUD for objectives
[V0.2.0] #72 objectives: implement scenario (hardcoded) with obectives and win/lose conditions
[V0.2.0] #99 add a resource that stores the scenarios
[V0.2.0] #84 Destroy Target
[V0.2.0] #91 objectives
[V0.2.0] #97 modding: implement re-usable module
[V0.2.0] #68 debug: add an even debuggier mode which disables nice graphics
[V0.2.0] #94 refactor: rethink the projectile and spawner plugin, try to make it without use of generics
[V0.2.0] #56 projectiles: implement inertia conservation
[V0.2.0] #66 brain: implement the spaceship brain - refactoring
[V0.2.0] #80 refactor: additional section that will store collider related components and is spawned in each hull_section, etc
[V0.2.0] #23 add fps and other indicators in example scenes
[V0.2.0] #81 Design for V0.2
[V0.1.0] #25 devlog
[V0.1.0] #79 shader for the thruster showcase
[V0.1.0] #60 health: implement collision with other entities
[V0.1.0] #8 thruster showcase
[V0.1.0] #49 turret: add particle effects on shooting
[V0.1.0] #67 Diegetic HUD
[V0.1.0] #59 health: implement destroy damaged entities
[V0.1.0] #58 health: create a simple demo scene to showcase health and damage
[V0.1.0] #55 health: implement damage system (kinematic damage)
[V0.1.0] #54 shooting: implement raycast for bullet
[V0.1.0] #53 turret: add to editor
[V0.1.0] #48 turret: implement shooting system
[V0.1.0] #47 turret: add custom visuals
[V0.1.0] #46 turret: add custom parameters in config
[V0.1.0] #45 turret: add flag for enabling render
[V0.1.0] #44 turret: add custom model with default as is
[V0.1.0] #43 PDC Turret
[V0.1.0] #40 example: create testing example for only controller
[V0.1.0] #39 example: create example for only thruster
[V0.1.0] #38 ci/cd: github workflow to create builds for linux/windows/macos
[V0.1.0] #37 move the game assets plugin into some kind of better library
[V0.1.0] #35 example: implement the 01_demo example to allow all the features to be showcased
[V0.1.0] #31 controller: add custom parameters for PD Controller
[V0.1.0] #30 controller: add custom mass for the section
[V0.1.0] #29 controller: add custom model with default as is
[V0.1.0] #28 controller: add flag for the plugin to not render the graphics
[V0.1.0] #27 Controller
[V0.1.0] #26 example: add to the 01_demo scene a mode where you can actually add sections with click
[V0.1.0] #22 move the Skybox plugin into library
[V0.1.0] #21 Create a template for YT devlogs
[V0.1.0] #20 thruster: add visual for the acceleration using a shader
[V0.1.0] #19 thruster: add flag for the plugin to not render the graphics
[V0.1.0] #18 thruster: add custom model with default as is
[V0.1.0] #17 thruster: add custom mass for the thruster
[V0.1.0] #16 thruster: rename from engine to thruster
[V0.1.0] #15 hull: add flag for the plugin to render if true
[V0.1.0] #14 hull: add default render model as a cube
[V0.1.0] #13 hull: make the hull render model configurable
[V0.1.0] #12 hull: make collider density for hull section configurable
[V0.1.0] #11 Thruster
[V0.1.0] #10 Hull
[V0.1.0] #75 Initial Design for V0.1

## New tasks

### 🏗️ Crate Structure & Architecture

[refactor] Extract bevy_common_systems crate — Move thruster, PD controller, and shared physics helpers into the standalone crate [new]
[refactor] Create nova_gameplay local crate — Umbrella crate for gameplay-specific plugins (sections, health, weapons, objectives) not yet ready for bevy_common_systems [new]
[refactor] Create nova_core crate — Assembles all plugins into the runnable game; thin wiring layer only [new]
[refactor] Make thruster a bevy_common_systems component — Detach from nova-specific assumptions #144
[refactor] Make PD controller a bevy_common_systems component — Reusable controller with no game-specific deps #145
[refactor] Pull bevy_common_systems out as separate package — Repo/workspace split #133
[refactor] All bevy_common_systems components must never spawn entities directly — Enforce parenting rule across the board #106


### 🧹 Refactor & Cleanup

[refactor] Improve input system for spaceship components — Thruster and turrets use bevy enhanced input #114
[refactor] Add PlayerControllerConfig input mapping — HashMap<Action, Input> per section #117
[refactor] Rework projectile and spawner plugin — Remove generics, simplify API #94
[refactor] Add base game object abstraction — Adds physics + health automatically; reduces boilerplate per entity #107
[refactor] Improve spaceship brain — Clean up refactor pass #66
[refactor] Rename collision_damage to collision_impact — Naming clarity #119
[refactor] Add collider section as separate child entity — Stores collider-related components, spawned per hull/section #80
[refactor] Fix insert_spaceship_sections editor — Should not spawn spaceship, only visual config preview #124
[refactor] Improve next_scenario logic — Cleaner implementation #125
[refactor] Remove LoadScenarioId — Replace with direct LoadScenario by config #109
[refactor] Improve despawning and entity management in scenarios — Consistent teardown #104
[refactor] Fix TODO's across codebase — Sweep and resolve or ticket #160
[refactor] Check required plugins were added in gameplay plugins — Panic or warn early with clear message #126


### 💥 Health & Destruction System

[health] Add structural integrity health to rigidbodies — Decides when a game object is destroyed #120
[health] Add health to individual colliders (sections) — Damage per-section, not just per-rigidbody #149
[health] When section health hits zero, disable collider then explode if leaf — Two-step destruction #152
[health] Explode and despawn zero-health section — Triggered after disable step #150
[health] Build graph of spaceship sections (DFS/BFS) — Foundation for structural damage propagation #148
[health] Fix on_delete of sections to remove from graph — Keep section graph consistent #151
[health] Improve on_destroy handling — Delay despawn so systems can react before entity is gone #129
[health] Variable damage by section type — Thrusters take more, turrets take less, etc. #122


### 🧪 Tests & Examples

[test] Create tests from existing examples — Convert smoke-test examples into proper integration tests #96
[test] Add unit tests for bevy_common_systems components — Thruster, PD controller, health [new]
[test] Add unit tests for section graph (DFS/BFS) — Validate graph builds and updates correctly [new]
[test] Add unit tests for health + destruction pipeline — Sequence: damage → disable → explode [new]
[example] Minimal example for nova_gameplay crate — Shows a ship with sections, health, one weapon [new]
[example] Minimal example for bevy_common_systems — Thruster + PD controller standalone [new]
[example] Controller-only example — Already exists, keep and maintain #40
[example] Thruster-only example — Already exists, keep and maintain #39


### 🐛 Bug Fixes & Stability

[bug] When switching scenes, remove all objects — Full cleanup on scene transition #102
[bug] Ensure mesh slicer does not crash the game — Guard against edge cases #88
[bug] No spaceship → camera should be WASD; with spaceship → chase cam — Camera mode switching #100
[bug] Improve error handling and logging in modding logic — Fail loudly, not silently #115


### ⚙️ Systems & Infrastructure

[chore] Add status info to ScenarioLoaded event — Useful for debugging scenario init #131
[chore] Add marker component for post-processing camera — Already done, verify it's wired #132
[chore] Add spawn-less visual mode for low-end machines — Skip particles/shaders flag #127
[chore] Optimize modding event handler lookup — Index handlers by event name #118
[chore] Create changelog.md — From git history #95
[chore] Add FPS and diagnostics overlay in example scenes — Already partially done #23
[modding] Allow changing skybox cubemap via action — Modding hook #130


### 🚀 Torpedo & Weapons (existing open, stabilization only)

[torpedo] Basic torpedo that follows assigned target — No PN yet, just lock-on #143
[torpedo] Implement torpedo bay section — Similar to turret section #141
[torpedo] Implement damage dealing for torpedo — On impact #139
[torpedo] Implement PN guidance for torpedo — Proportional navigation intercept #142
[torpedo] Add HUD indicator when torpedo is fired — Show target lock #146
[torpedo] Blast radius visual — Shader or particle effect on detonation #147
[torpedo] Torpedo bay shooting particles — Visual feedback #135
[weapons] Implement ammo limit logic — Generic across turret and torpedo #140


### 📋 Objectives & Scenarios

[objectives] Implement scenario with hardcoded objectives and win/lose — Foundation #72
[objectives] Implement HUD for objectives — Display current objective state #90
[objectives] Add scenario config resource — Store all scenarios #99
[objectives] Scenario language/config format — Data-driven scenario definition #101


### 📝 Documentation

[docs] Write documentation for nova_gameplay public API [new]
[docs] Write documentation for bevy_common_systems public API [new]
[docs] Add inline doc comments to all public plugin structs/components [new]
[docs] General documentation pass — Already tracked #113


Suggested implementation order for v0.3.1:
Crate structure → Refactor sweep → Health/destruction stabilization → Tests → Bug fixes → Docs. Torpedo/objectives can come after the foundation is solid.

