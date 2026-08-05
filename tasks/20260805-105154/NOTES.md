# Notes: Refresh frontend app images

## Problem Statement

The web images are missing because the scenes behind them are not worth
photographing. Confirmed with the owner 2026-08-05:

- Flat lighting. Every scenario gets exactly one hardcoded `DirectionalLight`
  rotated straight down (`crates/nova_scenario/src/loader/lifecycle.rs:203`),
  no fill, no rim, nothing authorable.
- Block-primitive ships. The reel's "showcase ship" is two
  `reinforced_hull_section` + a thruster + a turret on the engine's
  `hull-01.glb` primitive - not a ship anyone would put on a landing page.
- An asteroid field nobody can see. `reel.content.ron` scatters its 28 rocks in
  a `Ring(inner: 90, outer: 180)`, so every frame reads as empty space.
- A two-ship empty combat range. `screenshot_combat` is the player plus one
  target dead ahead, on nothing.

The fix is scene design and example structure. It is NOT capture plumbing:
`--report`, the manifest and the packaging script work and landed already
(`0ff077ff`).

Explicitly not this task: the 7 `manual` post-card thumbnails, the 25
`historical` pre-v0.9.0 news figures, new art assets, and engine rendering work.

Round one ships NO screenshots. It produces runnable, hand-inspectable scenes;
the owner runs each example plainly (free-fly WASD camera, no `NOVA_REEL`) and
says whether it is good enough to shoot. Capture is a later round.

## Context

### Settled with the owner

| Decision | Value |
| --- | --- |
| Roster | 6 examples: `scene`, `flight`, `combat`, `sections`, `ui`, `nova_os` |
| Engine boundary | Example-side photo rigs only. Authorable scenario lighting is filed separately as `20260805-111534` (v0.10.0) and does not block this. |
| New art | Out of scope. Work with the assets in the tree. |
| Round-one gate | Owner runs each example by hand and accepts the look. No captures. |
| Scope | The 27 `capturable` images only. |
| Structure | A bounded shared kit (photo rig, Kenney hull lists, dressing helper); scene RON and beats stay per-example. |
| Cadence | Scene by scene: build one, owner runs it plainly, verdict, next. `screenshot_scene` first. |

### What the tree already gives us

- **Kenney hulls are already built as section lists.** `base.content.ron` holds
  123 section prototypes including `racer_cube_*`, `cargoa_*`, `cargob_*` (the
  cut-up Kenney models under `assets/base/gltf/{racer,cargoa,cargob}/`), and the
  shipped menu backdrops - `menu_scrapyard`, `menu_waystation`,
  `menu_ambience` - already fly whole ships built from them. Those section
  lists are the reference to copy; no modelling needed.
- **AI vs AI is supported.** `Allegiance` is `Player | Enemy | Neutral`;
  `SpaceshipConfig::allegiance` is an explicit `Option` override over the
  controller default (Player ships Player, AI ships Enemy), and AI target
  acquisition runs off the relation model
  (`crates/nova_gameplay/src/input/ai/acquisition.rs`), not off the player
  marker. So a screenshot-only "faction fight" is an AI flight with
  `allegiance: Some(Player)` against a default-Enemy flight, with no player in
  the scene. UNPROVEN in practice - nothing in the tree does it today.
- **Bloom is already on.** Every scenario camera carries `PostProcessingCamera`
  and `PostProcessingDefaultPlugin` gives it `Bloom::NATURAL` +
  `Tonemapping::TonyMcMapface`. Thruster plumes, muzzle flashes and explosions
  will glow the moment the frame has something bright in it.
- **The scenario DSL is rich enough**: `ScatterObjects` (seeded, ring/region,
  radius variance), `SpawnScenarioObject`, `SetCamera`, `SetAllegiance`,
  `CreateScenarioArea`, AI `patrol` / `orbit` / `leash` / `engage_delay`,
  skybox swap.
- **Examples may spawn their own entities**, including lights - the reel scene
  is example-owned data (`examples/screenshots/data/reel.content.ron`,
  `include_str!`, never shipped in `assets/`). That is where the photo rig
  lives.
- **The plain-run mode is the iteration loop.** Every screenshot example already
  boots into the scene under the free-fly WASD camera when neither
  `NOVA_AUTOPILOT` nor `NOVA_REEL` is set. Round one needs no new mode.

### Constraints carried verbatim

- "Use the kenney spaceship where possible to make it look good."
- "More asteroids, more ships, more juice, make the scenes more alive."
- "Factions but gated to fight only for screenshots via autopilot somehow."
- "Do not start straight away - design the maps to make them actually look
  good."
- "A round of creating these examples but WITHOUT doing screenshots, such that
  it's easier to iterate first. I will test them manually."
- Framing fixes land in the example's scene/pose code, never on a PNG.
- Producers stay capture-only (`20260804-093910`); `nova_probe` never enters
  this path (`20260802-120045` WONTDO).

### Ceilings we accept this round

| Ceiling | Consequence |
| --- | --- |
| One hardcoded top-down scenario light | Examples spawn their own rig; shipped menu backdrops stay flat until `20260805-111534` |
| Two skyboxes (`cubemap.png`, `cubemap_alt.png`) | Both get used; no new art |
| `feature-juice` wants a shatter AND a fixed close pose | A live brawl gives the first and ruins the second, so the blow stays scripted |

## Ideas

Ranked, best first.

### 1. Six scene-owned examples over one shared photo kit (recommended)

Each example owns its scene RON and its beats. A small shared module under
`examples/screenshots/` holds what all six need: the photo rig (key + rim +
fill lights), the Kenney hull section lists lifted from the menu backdrops, and
a near-field dressing helper.

- Cost: one shared module plus six scene files; the rig is written once.
- Named requirement for the sharing: 4+ examples need the same lights and 3+
  need a Kenney hull. Not speculative.
- Risk: the shared module grows into a framework. Bound it - lights, hulls,
  dressing, nothing else.

### 2. Six fully self-contained examples, no shared module

Every example carries its own lights and its own copy of a Kenney section list.

- Cost: a racer hull is ~100 lines of RON section entries; four copies of it,
  and a lighting tweak has to be applied six times.
- Wins only if the six scenes turn out to want genuinely different rigs. Worth
  reconsidering after the first two scenes exist - not before.

### 3. Build the scenes as shipped scenario content

Author the showcase scenes under `assets/base/scenarios/` so players can fly
them too.

- Rejected: shipped content is the game's promise to players and gets balance,
  lint and mod-format obligations; a scene built for one camera angle fails all
  of them. Breaks the established example-owned-data convention
  (`reel.content.ron` exists precisely so capture scenes stay out of `assets/`).

### 4. One big scene reused by every example

A single "Nova showcase system" scene, with each example just posing a camera
into it.

- Rejected by the roster: the six examples need contradictory states - frozen
  vs alive, HUD vs no HUD, player ship vs no player ship, clean backdrop vs
  dense field. One scene serving all of them becomes a mode switch per shot,
  which is idea 1 with worse ergonomics.

## Map designs

First-pass intent, to be tuned by eye. Every scene is example-owned data.

| Example | Scene | Alive how | Shots |
| --- | --- | --- | --- |
| `screenshot_scene` | "Drydock drift": planetoid off-frame-left at readable distance, near-field rocks scattered 15-60 units with radius variance, a hero Kenney racer posed foreground, two more Kenney hulls drifting mid-field | AI `orbit` on the drifting hulls, hero ship posed | `feature-gravity`, `wiki-gravity`, `wiki-sections` |
| `screenshot_flight` | "The ring": gravity planetoid, player Kenney racer on the ORBIT autopilot at ring radius, rocks along the ring for parallax, HUD on | Live orbit maneuver, HUD ring + radius spoke | `feature-autopilot`, `wiki-flight`, `tutorial-orbit` |
| `screenshot_combat` | "Rock hollow": two Kenney flights split by allegiance inside a dense near asteroid field, plus a player ship off to one side for the lock/HUD beats | AI vs AI with `engage_delay` so they ARRIVE and then fight; one scripted section blow for the juice beat | 10, incl. `feature-juice` and 2x `news-090` |
| `screenshot_sections` | Frozen ship carrying all five ENGINE section types on a clean backdrop, three-point rig | Frozen on purpose | the five `wiki-section-*` |
| `screenshot_ui` | The shipped app (`editor_app`) | Real UI states | `feature-editor`, `tutorial-menu`, `wiki-settings`, `news-090-scenario-campaigns` |
| `screenshot_nova_os` | Existing one-ship range, Tab computer | Real command script | `news-090-nova-os-terminal`, `news-090-nova-os-apps` |

The four ALIASES (`wiki-combat`, `wiki-hud`, `wiki-flight`, `wiki-radar`) each
become their own framed beat, so `ALIASES` empties out of the manifest.

## Open questions

- AI-vs-AI via an `allegiance: Some(Player)` flight is inferred from the
  relation model, never run. It is the one thing that could invalidate the
  combat scene design, so prove it with a throwaway run before the scene is
  built out.
- `wiki-settings.png` and the three `news-090-*` shots have no manifest slot;
  they get one when their producer is settled, not before.
